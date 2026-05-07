use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use geometry_core::{Coord, DEFAULT_GRID, Point, Polygon, Rect, Vector};
use loro::{
    Container, ExportMode, LoroDoc, LoroEncodeError, LoroError, LoroMap, LoroValue, PeerID,
    ValueOrContainer,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeMap};
use uuid::Uuid;

pub mod connectivity;
pub mod equipment;
pub mod experiment;
pub mod gdsii;
pub mod genealogy;
pub mod mask;
pub mod mes;
pub mod metrology;
pub mod process_control;
pub mod recipe;
pub mod spc_fdc;
pub mod yield_analysis;

pub const CURRENT_SCHEMA_VERSION: u32 = 5;
pub const DEFAULT_TOP_CELL_ID: CellId = CellId(1);
pub const DEFAULT_TECHNOLOGY_JSON: &str =
    include_str!("../../../assets/technology/fabricad_demo.json");
pub const HIGH_DENSITY_TECHNOLOGY_JSON: &str =
    include_str!("../../../assets/technology/fabricad_high_density.json");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShapeId(pub u64);

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShapeOccurrenceId {
    pub shape: ShapeId,
    pub instance_path: Vec<InstanceId>,
    #[serde(default)]
    pub array_path: Vec<ArrayIndex>,
}

impl ShapeOccurrenceId {
    pub fn top_level(shape: ShapeId) -> Self {
        Self {
            shape,
            instance_path: Vec::new(),
            array_path: Vec::new(),
        }
    }

    pub fn from_instance_path(shape: ShapeId, instance_path: &[InstanceId]) -> Self {
        Self {
            shape,
            instance_path: instance_path.to_vec(),
            array_path: Vec::new(),
        }
    }

    pub fn from_instance_array_path(
        shape: ShapeId,
        instance_path: &[InstanceId],
        array_path: &[ArrayIndex],
    ) -> Self {
        Self {
            shape,
            instance_path: instance_path.to_vec(),
            array_path: array_path.to_vec(),
        }
    }

    pub fn source_shape_id(&self) -> ShapeId {
        self.shape
    }

    pub fn is_top_level(&self) -> bool {
        self.instance_path.is_empty()
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ArrayIndex {
    pub column: u32,
    pub row: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetId(pub u32);

macro_rules! impl_numeric_id_serde {
    ($type:ident, $inner:ty, $expecting:literal) => {
        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct IdVisitor;

                impl<'de> de::Visitor<'de> for IdVisitor {
                    type Value = $type;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str($expecting)
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        <$inner>::try_from(value)
                            .map($type)
                            .map_err(|_| E::custom(format!("{value} is out of range")))
                    }

                    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        if value < 0 {
                            return Err(E::custom(format!("{value} is negative")));
                        }
                        self.visit_u64(value as u64)
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        let parsed = value
                            .parse::<u64>()
                            .map_err(|_| E::custom(format!("{value:?} is not numeric")))?;
                        self.visit_u64(parsed)
                    }
                }

                deserializer.deserialize_any(IdVisitor)
            }
        }
    };
}

impl_numeric_id_serde!(ShapeId, u64, "a shape id as a number or numeric string");
impl_numeric_id_serde!(CellId, u64, "a cell id as a number or numeric string");
impl_numeric_id_serde!(
    InstanceId,
    u64,
    "an instance id as a number or numeric string"
);
impl_numeric_id_serde!(LayerId, u32, "a layer id as a number or numeric string");
impl_numeric_id_serde!(NetId, u32, "a net id as a number or numeric string");

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct CrdtOpId {
    pub actor: Uuid,
    pub counter: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub id: CrdtOpId,
    #[serde(default)]
    pub deps: Vec<CrdtOpId>,
    pub operation: Operation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoroUpdate {
    pub sender: Uuid,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrdtApplyResult {
    Applied,
    Duplicate,
}

#[derive(Debug)]
pub enum LoroCrdtError {
    Loro(LoroError),
    Export(LoroEncodeError),
    Encode(serde_json::Error),
    Decode(serde_json::Error),
    NonStringOperation {
        index: usize,
    },
    NonMapObject {
        collection: &'static str,
        key: String,
    },
}

impl fmt::Display for LoroCrdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loro(err) => write!(f, "loro operation failed: {err}"),
            Self::Export(err) => write!(f, "loro export failed: {err}"),
            Self::Encode(err) => write!(f, "failed to encode operation for Loro: {err}"),
            Self::Decode(err) => write!(f, "failed to decode operation from Loro: {err}"),
            Self::NonStringOperation { index } => {
                write!(f, "Loro operation log entry {index} is not a string")
            }
            Self::NonMapObject { collection, key } => {
                write!(f, "Loro object {collection}/{key} is not a map")
            }
        }
    }
}

impl Error for LoroCrdtError {}

impl From<LoroError> for LoroCrdtError {
    fn from(value: LoroError) -> Self {
        Self::Loro(value)
    }
}

impl From<LoroEncodeError> for LoroCrdtError {
    fn from(value: LoroEncodeError) -> Self {
        Self::Export(value)
    }
}

pub struct LoroCrdtLog {
    doc: LoroDoc,
    emitted: BTreeSet<CrdtOpId>,
}

impl fmt::Debug for LoroCrdtLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoroCrdtLog")
            .field("emitted", &self.emitted)
            .finish_non_exhaustive()
    }
}

impl LoroCrdtLog {
    const OPERATION_LOG: &'static str = "fabricad_ops";
    const SHAPES: &'static str = "fabricad_shapes";
    const CELLS: &'static str = "fabricad_cells";
    const INSTANCES: &'static str = "fabricad_instances";

    pub fn new(actor: Uuid) -> Result<Self, LoroCrdtError> {
        let doc = LoroDoc::new();
        doc.set_peer_id(loro_peer_id(actor))?;
        Ok(Self {
            doc,
            emitted: BTreeSet::new(),
        })
    }

    pub fn from_snapshot(actor: Uuid, snapshot: &[u8]) -> Result<Self, LoroCrdtError> {
        let mut log = Self::new(actor)?;
        if !snapshot.is_empty() {
            log.doc.import(snapshot)?;
        }
        log.mark_all_emitted()?;
        Ok(log)
    }

    pub fn seed_document_objects(&mut self, document: &Document) -> Result<(), LoroCrdtError> {
        for shape in document.shapes.values() {
            self.upsert_shape(None, &shape)?;
        }
        for cell in document.cells.values() {
            self.upsert_cell(cell)?;
            for shape in cell.shapes.values() {
                self.upsert_shape(Some(cell.id), &shape)?;
            }
            for instance in cell.instances.values() {
                self.upsert_instance(cell.id, &instance)?;
            }
        }
        self.doc.commit();
        Ok(())
    }

    pub fn append_operation(
        &mut self,
        sender: Uuid,
        operation: CrdtOperation,
    ) -> Result<LoroUpdate, LoroCrdtError> {
        let before = self.doc.oplog_vv();
        let encoded = serde_json::to_string(&operation).map_err(LoroCrdtError::Encode)?;
        self.doc.get_list(Self::OPERATION_LOG).push(encoded)?;
        self.mirror_operation_to_objects(&operation.operation)?;
        self.doc.commit();
        self.emitted.insert(operation.id);
        Ok(LoroUpdate {
            sender,
            bytes: self.doc.export(ExportMode::updates(&before))?,
        })
    }

    pub fn import_update(
        &mut self,
        update: &LoroUpdate,
    ) -> Result<Vec<CrdtOperation>, LoroCrdtError> {
        self.doc.import(&update.bytes)?;
        self.drain_unemitted_operations()
    }

    pub fn snapshot(&self, sender: Uuid) -> Result<LoroUpdate, LoroCrdtError> {
        Ok(LoroUpdate {
            sender,
            bytes: self.doc.export(ExportMode::Snapshot)?,
        })
    }

    pub fn export_snapshot(&self) -> Result<Vec<u8>, LoroCrdtError> {
        Ok(self.doc.export(ExportMode::Snapshot)?)
    }

    pub fn drain_unemitted_operations(&mut self) -> Result<Vec<CrdtOperation>, LoroCrdtError> {
        let mut operations = Vec::new();
        for operation in self.operations()? {
            if self.emitted.insert(operation.id) {
                operations.push(operation);
            }
        }
        Ok(operations)
    }

    fn mark_all_emitted(&mut self) -> Result<(), LoroCrdtError> {
        for operation in self.operations()? {
            self.emitted.insert(operation.id);
        }
        Ok(())
    }

    fn operations(&self) -> Result<Vec<CrdtOperation>, LoroCrdtError> {
        let log = self.doc.get_list(Self::OPERATION_LOG);
        let mut operations = Vec::with_capacity(log.len());
        for index in 0..log.len() {
            let Some(entry) = log.get(index) else {
                continue;
            };
            let ValueOrContainer::Value(value) = entry else {
                return Err(LoroCrdtError::NonStringOperation { index });
            };
            let LoroValue::String(json) = value else {
                return Err(LoroCrdtError::NonStringOperation { index });
            };
            operations.push(serde_json::from_str(&json).map_err(LoroCrdtError::Decode)?);
        }
        Ok(operations)
    }

    fn mirror_operation_to_objects(&self, operation: &Operation) -> Result<(), LoroCrdtError> {
        match operation {
            Operation::Batch { operations } => {
                for operation in operations {
                    self.mirror_operation_to_objects(operation)?;
                }
            }
            Operation::AddShape { shape } => {
                self.upsert_shape(None, shape)?;
            }
            Operation::DeleteShape { id } => {
                self.tombstone_shape(*id)?;
            }
            Operation::ReplaceShape { id, shape } => {
                if self.shape_record(*id)?.is_some() {
                    let parent = self.shape_parent_cell(*id)?;
                    self.upsert_shape(parent, shape)?;
                }
            }
            Operation::MoveShape { id, delta } => {
                if let Some(mut shape) = self.shape_record(*id)? {
                    shape.kind.translate(*delta);
                    let parent = self.shape_parent_cell(*id)?;
                    self.upsert_shape(parent, &shape)?;
                }
            }
            Operation::AddCell { cell } => {
                self.upsert_cell(cell)?;
                for shape in cell.shapes.values() {
                    self.upsert_shape(Some(cell.id), &shape)?;
                }
                for instance in cell.instances.values() {
                    self.upsert_instance(cell.id, &instance)?;
                }
            }
            Operation::DeleteCell { id } => {
                self.tombstone_cell(*id)?;
            }
            Operation::RenameCell { id, name } => {
                let cell = self.object_map(Self::CELLS, &cell_key(*id))?;
                cell.insert("id", id.0 as i64)?;
                cell.insert("name", name.clone())?;
            }
            Operation::AddInstance { parent, instance } => {
                self.upsert_instance(*parent, instance)?;
            }
            Operation::ReplaceInstance {
                parent, instance, ..
            } => {
                if self.instance_record(*parent, instance.id)?.is_some() {
                    self.upsert_instance(*parent, instance)?;
                }
            }
            Operation::DeleteInstance { parent, id } => {
                self.tombstone_instance(*parent, *id)?;
            }
            Operation::RenameInstance { parent, id, name } => {
                let instance = self.object_map(Self::INSTANCES, &instance_key(*parent, *id))?;
                set_optional_string(&instance, "name", name.as_deref())?;
            }
            Operation::MoveInstance { parent, id, delta } => {
                if let Some(mut instance) = self.instance_record(*parent, *id)? {
                    instance.transform = instance
                        .transform
                        .compose(Transform::from_translation(*delta));
                    self.upsert_instance(*parent, &instance)?;
                }
            }
            Operation::AddLayer { .. }
            | Operation::SetLayerVisibility { .. }
            | Operation::Cursor { .. } => {}
        }
        Ok(())
    }

    fn object_map(&self, collection: &'static str, key: &str) -> Result<LoroMap, LoroCrdtError> {
        self.doc
            .get_map(collection)
            .get_or_create_container(key, LoroMap::new())
            .map_err(LoroCrdtError::from)
    }

    fn existing_object_map(
        &self,
        collection: &'static str,
        key: &str,
    ) -> Result<Option<LoroMap>, LoroCrdtError> {
        let Some(value) = self.doc.get_map(collection).get(key) else {
            return Ok(None);
        };
        match value {
            ValueOrContainer::Container(Container::Map(map)) => Ok(Some(map)),
            _ => Err(LoroCrdtError::NonMapObject {
                collection,
                key: key.to_string(),
            }),
        }
    }

    fn upsert_shape(
        &self,
        parent_cell: Option<CellId>,
        shape: &Shape,
    ) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::SHAPES, &shape_key(shape.id))?;
        map.insert("id", shape.id.0 as i64)?;
        set_optional_cell_id(&map, "parent_cell", parent_cell)?;
        map.insert("layer", shape.layer.0 as i64)?;
        set_optional_net_id(&map, "net", shape.net)?;
        set_optional_string(&map, "name", shape.name.as_deref())?;
        map.insert(
            "kind_json",
            serde_json::to_string(&shape.kind).map_err(LoroCrdtError::Encode)?,
        )?;
        map.insert("deleted", false)?;
        Ok(())
    }

    fn tombstone_shape(&self, id: ShapeId) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::SHAPES, &shape_key(id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    fn shape_record(&self, id: ShapeId) -> Result<Option<Shape>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::SHAPES, &shape_key(id))? else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or(false) {
            return Ok(None);
        }
        let Some(layer) = map_i64(&map, "layer") else {
            return Ok(None);
        };
        let Some(kind_json) = map_string(&map, "kind_json") else {
            return Ok(None);
        };
        Ok(Some(Shape {
            id,
            layer: LayerId(layer as u32),
            net: map_i64(&map, "net").map(|net| NetId(net as u32)),
            kind: serde_json::from_str(&kind_json).map_err(LoroCrdtError::Decode)?,
            name: map_optional_string(&map, "name"),
        }))
    }

    fn shape_parent_cell(&self, id: ShapeId) -> Result<Option<CellId>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::SHAPES, &shape_key(id))? else {
            return Ok(None);
        };
        Ok(map_i64(&map, "parent_cell").map(|cell| CellId(cell as u64)))
    }

    fn upsert_cell(&self, cell: &Cell) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::CELLS, &cell_key(cell.id))?;
        map.insert("id", cell.id.0 as i64)?;
        map.insert("name", cell.name.clone())?;
        map.insert("deleted", false)?;
        Ok(())
    }

    fn tombstone_cell(&self, id: CellId) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::CELLS, &cell_key(id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    pub fn cell_is_deleted(&self, id: CellId) -> Result<bool, LoroCrdtError> {
        Ok(self
            .existing_object_map(Self::CELLS, &cell_key(id))?
            .and_then(|map| map_bool(&map, "deleted"))
            .unwrap_or(false))
    }

    fn upsert_instance(
        &self,
        parent: CellId,
        instance: &CellInstance,
    ) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::INSTANCES, &instance_key(parent, instance.id))?;
        map.insert("id", instance.id.0 as i64)?;
        map.insert("parent_cell", parent.0 as i64)?;
        map.insert("cell", instance.cell.0 as i64)?;
        set_optional_string(&map, "name", instance.name.as_deref())?;
        map.insert(
            "transform_json",
            serde_json::to_string(&instance.transform).map_err(LoroCrdtError::Encode)?,
        )?;
        map.insert(
            "array_json",
            serde_json::to_string(&instance.array).map_err(LoroCrdtError::Encode)?,
        )?;
        map.insert("deleted", false)?;
        Ok(())
    }

    fn tombstone_instance(&self, parent: CellId, id: InstanceId) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::INSTANCES, &instance_key(parent, id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("parent_cell", parent.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    fn instance_record(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<Option<CellInstance>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::INSTANCES, &instance_key(parent, id))?
        else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or(false) {
            return Ok(None);
        }
        let Some(cell) = map_i64(&map, "cell") else {
            return Ok(None);
        };
        let transform = map_string(&map, "transform_json")
            .map(|json| serde_json::from_str(&json).map_err(LoroCrdtError::Decode))
            .transpose()?
            .unwrap_or_default();
        let array = map_string(&map, "array_json")
            .map(|json| serde_json::from_str(&json).map_err(LoroCrdtError::Decode))
            .transpose()?
            .unwrap_or_default();
        Ok(Some(CellInstance {
            id,
            name: map_optional_string(&map, "name"),
            cell: CellId(cell as u64),
            transform,
            array,
        }))
    }

    pub fn shape_is_deleted(&self, id: ShapeId) -> Result<bool, LoroCrdtError> {
        Ok(self
            .existing_object_map(Self::SHAPES, &shape_key(id))?
            .and_then(|map| map_bool(&map, "deleted"))
            .unwrap_or(false))
    }

    pub fn instance_is_deleted(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<bool, LoroCrdtError> {
        Ok(self
            .existing_object_map(Self::INSTANCES, &instance_key(parent, id))?
            .and_then(|map| map_bool(&map, "deleted"))
            .unwrap_or(false))
    }

    pub fn shape_from_store(&self, id: ShapeId) -> Result<Option<Shape>, LoroCrdtError> {
        self.shape_record(id)
    }

    pub fn instance_from_store(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<Option<CellInstance>, LoroCrdtError> {
        self.instance_record(parent, id)
    }

    pub fn materialize_objects_into_document(
        &self,
        document: &mut Document,
    ) -> Result<(), LoroCrdtError> {
        document.shapes.clear();
        document.cells.clear();
        document.ensure_hierarchy();

        for cell_id in self.cell_ids() {
            if let Some(cell) = self.cell_record(cell_id)? {
                document.cells.insert(cell.id, cell);
            }
        }
        document.ensure_hierarchy();

        for shape_id in self.shape_ids() {
            let Some(shape) = self.shape_record(shape_id)? else {
                continue;
            };
            document.next_shape_id = document.next_shape_id.max(shape.id.0 + 1);
            match self.shape_parent_cell(shape.id)? {
                Some(parent) => {
                    if let Some(cell) = document.cells.get_mut(&parent) {
                        cell.shapes.insert(shape.id, shape);
                    }
                }
                None => {
                    document.shapes.insert(shape.id, shape);
                }
            }
        }

        for (parent, instance_id) in self.instance_ids() {
            let Some(instance) = self.instance_record(parent, instance_id)? else {
                continue;
            };
            if document.cells.contains_key(&instance.cell) {
                document.next_instance_id = document.next_instance_id.max(instance.id.0 + 1);
                if !document.cells.contains_key(&parent) {
                    document
                        .cells
                        .insert(parent, Cell::new(parent, format!("cell {}", parent.0)));
                }
                if let Some(parent) = document.cells.get_mut(&parent) {
                    parent.instances.insert(instance.id, instance);
                }
            }
        }
        document.ensure_hierarchy();
        Ok(())
    }

    fn cell_record(&self, id: CellId) -> Result<Option<Cell>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::CELLS, &cell_key(id))? else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or(false) {
            return Ok(None);
        }
        let name = map_string(&map, "name").unwrap_or_else(|| format!("cell {}", id.0));
        Ok(Some(Cell::new(id, name)))
    }

    fn shape_ids(&self) -> Vec<ShapeId> {
        self.doc
            .get_map(Self::SHAPES)
            .keys()
            .filter_map(|key| key.parse::<u64>().ok().map(ShapeId))
            .collect()
    }

    fn cell_ids(&self) -> Vec<CellId> {
        self.doc
            .get_map(Self::CELLS)
            .keys()
            .filter_map(|key| key.parse::<u64>().ok().map(CellId))
            .collect()
    }

    fn instance_ids(&self) -> Vec<(CellId, InstanceId)> {
        self.doc
            .get_map(Self::INSTANCES)
            .keys()
            .filter_map(|key| {
                let (parent, id) = key.split_once(':')?;
                Some((CellId(parent.parse().ok()?), InstanceId(id.parse().ok()?)))
            })
            .collect()
    }
}

pub fn loro_peer_id(actor: Uuid) -> PeerID {
    let raw = actor.as_u128();
    ((raw >> 64) as u64) ^ raw as u64
}

fn shape_key(id: ShapeId) -> String {
    id.0.to_string()
}

fn cell_key(id: CellId) -> String {
    id.0.to_string()
}

fn instance_key(parent: CellId, id: InstanceId) -> String {
    format!("{}:{}", parent.0, id.0)
}

fn set_optional_string(map: &LoroMap, key: &str, value: Option<&str>) -> Result<(), LoroCrdtError> {
    match value {
        Some(value) => map.insert(key, value)?,
        None => map.insert(key, LoroValue::Null)?,
    }
    Ok(())
}

fn set_optional_cell_id(
    map: &LoroMap,
    key: &str,
    value: Option<CellId>,
) -> Result<(), LoroCrdtError> {
    match value {
        Some(value) => map.insert(key, value.0 as i64)?,
        None => map.insert(key, LoroValue::Null)?,
    }
    Ok(())
}

fn set_optional_net_id(
    map: &LoroMap,
    key: &str,
    value: Option<NetId>,
) -> Result<(), LoroCrdtError> {
    match value {
        Some(value) => map.insert(key, value.0 as i64)?,
        None => map.insert(key, LoroValue::Null)?,
    }
    Ok(())
}

fn map_value(map: &LoroMap, key: &str) -> Option<LoroValue> {
    match map.get(key)? {
        ValueOrContainer::Value(value) => Some(value),
        ValueOrContainer::Container(_) => None,
    }
}

fn map_bool(map: &LoroMap, key: &str) -> Option<bool> {
    match map_value(map, key)? {
        LoroValue::Bool(value) => Some(value),
        _ => None,
    }
}

fn map_i64(map: &LoroMap, key: &str) -> Option<i64> {
    match map_value(map, key)? {
        LoroValue::I64(value) => Some(value),
        _ => None,
    }
}

fn map_string(map: &LoroMap, key: &str) -> Option<String> {
    match map_value(map, key)? {
        LoroValue::String(value) => Some(value.to_string()),
        _ => None,
    }
}

fn map_optional_string(map: &LoroMap, key: &str) -> Option<String> {
    match map_value(map, key)? {
        LoroValue::String(value) => Some(value.to_string()),
        LoroValue::Null => None,
        _ => None,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub process: ProcessLayer,
    #[serde(default)]
    pub purpose: String,
    pub color: [f32; 4],
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub gds_layer: Option<u16>,
    #[serde(default)]
    pub gds_datatype: u16,
    #[serde(default)]
    pub gds_texttype: u16,
    pub visible: bool,
    pub locked: bool,
}

const MAX_DENSE_LAYER_ID_INDEX: usize = 1_000_000;

#[derive(Clone, Debug, Default)]
pub struct LayerStore {
    alive: Vec<bool>,
    ids: Vec<LayerId>,
    layers: Vec<Layer>,
    id_to_row: Vec<Option<usize>>,
    overflow_id_to_row: BTreeMap<LayerId, usize>,
    len: usize,
}

impl LayerStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            alive: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            layers: Vec::with_capacity(capacity),
            id_to_row: Vec::new(),
            overflow_id_to_row: BTreeMap::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.alive.clear();
        self.ids.clear();
        self.layers.clear();
        self.id_to_row.clear();
        self.overflow_id_to_row.clear();
        self.len = 0;
    }

    pub fn get(&self, id: &LayerId) -> Option<&Layer> {
        self.row_for_id(*id).map(|row| &self.layers[row])
    }

    pub fn get_mut(&mut self, id: &LayerId) -> Option<&mut Layer> {
        let row = self.row_for_id(*id)?;
        Some(&mut self.layers[row])
    }

    pub fn contains_key(&self, id: &LayerId) -> bool {
        self.row_for_id(*id).is_some()
    }

    pub fn insert(&mut self, id: LayerId, mut layer: Layer) -> Option<Layer> {
        layer.id = id;
        if let Some(row) = self.row_for_id(id)
            && self.alive.get(row).copied().unwrap_or(false)
        {
            return Some(std::mem::replace(&mut self.layers[row], layer));
        }

        let row = self.ids.len();
        self.set_row_for_id(id, row);
        self.alive.push(true);
        self.ids.push(id);
        self.layers.push(layer);
        self.len += 1;
        None
    }

    pub fn remove(&mut self, id: &LayerId) -> Option<Layer> {
        let row = self.row_for_id(*id)?;
        self.clear_row_for_id(*id);
        if !self.alive.get(row).copied().unwrap_or(false) {
            return None;
        }
        self.alive[row] = false;
        self.len -= 1;
        Some(self.layers[row].clone())
    }

    pub fn values(&self) -> impl Iterator<Item = &Layer> {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| &self.layers[row])
    }

    pub fn keys(&self) -> impl Iterator<Item = &LayerId> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(_, id)| id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&LayerId, &Layer)> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(row, id)| (id, &self.layers[row]))
    }

    fn row_for_id(&self, id: LayerId) -> Option<usize> {
        dense_layer_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    fn set_row_for_id(&mut self, id: LayerId, row: usize) {
        if let Some(index) = dense_layer_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            self.overflow_id_to_row.insert(id, row);
        }
    }

    fn clear_row_for_id(&mut self, id: LayerId) {
        if let Some(index) = dense_layer_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }
}

impl Extend<(LayerId, Layer)> for LayerStore {
    fn extend<T: IntoIterator<Item = (LayerId, Layer)>>(&mut self, iter: T) {
        for (id, layer) in iter {
            self.insert(id, layer);
        }
    }
}

impl FromIterator<(LayerId, Layer)> for LayerStore {
    fn from_iter<T: IntoIterator<Item = (LayerId, Layer)>>(iter: T) -> Self {
        let mut store = Self::new();
        for (id, layer) in iter {
            store.insert(id, layer);
        }
        store
    }
}

impl Serialize for LayerStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len))?;
        for (id, layer) in self.iter() {
            map.serialize_entry(id, layer)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for LayerStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let layers = BTreeMap::<LayerId, Layer>::deserialize(deserializer)?;
        Ok(layers.into_iter().collect())
    }
}

fn dense_layer_index(id: LayerId) -> Option<usize> {
    let index = usize::try_from(id.0).ok()?;
    (index <= MAX_DENSE_LAYER_ID_INDEX).then_some(index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessLayer {
    Diffusion,
    Poly,
    Contact,
    Metal1,
    Via1,
    Metal2,
    Oxide,
    Annotation,
}

impl ProcessLayer {
    pub fn as_technology_name(self) -> &'static str {
        match self {
            Self::Diffusion => "diffusion",
            Self::Poly => "poly",
            Self::Contact => "contact",
            Self::Metal1 => "metal1",
            Self::Via1 => "via1",
            Self::Metal2 => "metal2",
            Self::Oxide => "oxide",
            Self::Annotation => "annotation",
        }
    }

    pub fn from_technology_name(value: &str) -> Option<Self> {
        let normalized = value
            .chars()
            .filter(|character| {
                *character != '_' && *character != '-' && !character.is_whitespace()
            })
            .flat_map(char::to_lowercase)
            .collect::<String>();
        match normalized.as_str() {
            "diffusion" | "active" => Some(Self::Diffusion),
            "poly" | "polysilicon" => Some(Self::Poly),
            "contact" | "cont" => Some(Self::Contact),
            "metal1" | "m1" => Some(Self::Metal1),
            "via1" | "v1" => Some(Self::Via1),
            "metal2" | "m2" => Some(Self::Metal2),
            "oxide" => Some(Self::Oxide),
            "annotation" | "label" | "text" => Some(Self::Annotation),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TechnologyError {
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for TechnologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(err) => write!(f, "failed to parse technology JSON: {err}"),
            Self::Invalid(message) => write!(f, "invalid technology file: {message}"),
        }
    }
}

impl Error for TechnologyError {}

impl From<serde_json::Error> for TechnologyError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TechnologyFile {
    pub name: String,
    #[serde(default = "default_dbu_per_micron")]
    pub dbu_per_micron: Coord,
    #[serde(default = "default_grid")]
    pub grid: Coord,
    pub layers: Vec<TechnologyLayer>,
    #[serde(default)]
    pub connectivity: Vec<TechnologyConnection>,
    #[serde(default)]
    pub drc: TechnologyDrc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TechnologyLayer {
    #[serde(default)]
    pub id: Option<LayerId>,
    pub name: String,
    pub process: String,
    #[serde(default)]
    pub purpose: String,
    pub color: [f32; 4],
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub gds_layer: Option<u16>,
    #[serde(default)]
    pub gds_datatype: u16,
    #[serde(default)]
    pub gds_texttype: u16,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologyConnection {
    pub from: String,
    pub through: String,
    pub to: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologyDrc {
    #[serde(default)]
    pub min_width: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub min_spacing: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub via_enclosure: Vec<TechnologyEnclosureRule>,
    #[serde(default)]
    pub forbidden_overlaps: Vec<TechnologyForbiddenOverlapRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologyLayerRule {
    pub layer: String,
    pub value: Coord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologyEnclosureRule {
    pub via: String,
    pub enclosure: String,
    pub required: Coord,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnologyForbiddenOverlapRule {
    pub a: String,
    pub b: String,
    pub name: String,
}

impl TechnologyFile {
    pub fn from_json_str(contents: &str) -> Result<Self, TechnologyError> {
        let technology: Self = serde_json::from_str(contents)?;
        technology.validate()?;
        Ok(technology)
    }

    pub fn validate(&self) -> Result<(), TechnologyError> {
        if self.name.trim().is_empty() {
            return Err(TechnologyError::Invalid(
                "technology name must not be empty".to_string(),
            ));
        }
        if self.dbu_per_micron <= 0 {
            return Err(TechnologyError::Invalid(
                "dbu_per_micron must be positive".to_string(),
            ));
        }
        if self.grid <= 0 {
            return Err(TechnologyError::Invalid(
                "grid must be positive".to_string(),
            ));
        }
        if self.layers.is_empty() {
            return Err(TechnologyError::Invalid(
                "at least one layer is required".to_string(),
            ));
        }

        let mut names = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut gds_geometry_pairs = BTreeSet::new();
        let mut gds_text_pairs = BTreeSet::new();
        for layer in &self.layers {
            if layer.name.trim().is_empty() {
                return Err(TechnologyError::Invalid(
                    "layer names must not be empty".to_string(),
                ));
            }
            let key = normalize_layer_ref(&layer.name);
            if !names.insert(key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate layer name {:?}",
                    layer.name
                )));
            }
            if let Some(id) = layer.id
                && !ids.insert(id)
            {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate layer id {}",
                    id.0
                )));
            }
            if ProcessLayer::from_technology_name(&layer.process).is_none() {
                return Err(TechnologyError::Invalid(format!(
                    "layer {:?} has unknown process {:?}",
                    layer.name, layer.process
                )));
            }
            let Some(gds_layer) = layer.effective_gds_layer() else {
                return Err(TechnologyError::Invalid(format!(
                    "layer {:?} needs an id or gds_layer for GDSII mapping",
                    layer.name
                )));
            };
            if !gds_geometry_pairs.insert((gds_layer, layer.gds_datatype)) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate GDSII geometry mapping ({gds_layer}, {})",
                    layer.gds_datatype
                )));
            }
            if !gds_text_pairs.insert((gds_layer, layer.gds_texttype)) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate GDSII text mapping ({gds_layer}, {})",
                    layer.gds_texttype
                )));
            }
        }

        for connection in &self.connectivity {
            self.layer_id(&connection.from)?;
            self.layer_id(&connection.through)?;
            self.layer_id(&connection.to)?;
        }
        for rule in &self.drc.min_width {
            self.layer_id(&rule.layer)?;
            validate_non_negative(rule.value, "min_width", &rule.layer)?;
        }
        for rule in &self.drc.min_spacing {
            self.layer_id(&rule.layer)?;
            validate_non_negative(rule.value, "min_spacing", &rule.layer)?;
        }
        for rule in &self.drc.via_enclosure {
            self.layer_id(&rule.via)?;
            self.layer_id(&rule.enclosure)?;
            validate_non_negative(rule.required, "via_enclosure", &rule.via)?;
        }
        for rule in &self.drc.forbidden_overlaps {
            self.layer_id(&rule.a)?;
            self.layer_id(&rule.b)?;
            if rule.name.trim().is_empty() {
                return Err(TechnologyError::Invalid(
                    "forbidden overlap rules need a non-empty name".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn layer_id(&self, reference: &str) -> Result<LayerId, TechnologyError> {
        let normalized = normalize_layer_ref(reference);
        self.layers
            .iter()
            .find(|layer| normalize_layer_ref(&layer.name) == normalized)
            .and_then(|layer| layer.id)
            .ok_or_else(|| {
                TechnologyError::Invalid(format!("unknown layer reference {reference:?}"))
            })
    }

    pub fn layer_for_gds_geometry(
        &self,
        gds_layer: u16,
        datatype: u16,
    ) -> Option<&TechnologyLayer> {
        self.layers.iter().find(|layer| {
            layer.effective_gds_layer() == Some(gds_layer) && layer.gds_datatype == datatype
        })
    }

    pub fn layer_for_gds_text(&self, gds_layer: u16, texttype: u16) -> Option<&TechnologyLayer> {
        self.layers.iter().find(|layer| {
            layer.effective_gds_layer() == Some(gds_layer) && layer.gds_texttype == texttype
        })
    }

    pub fn gds_mapping_for_layer(&self, id: LayerId) -> Option<(u16, u16, u16)> {
        self.layers
            .iter()
            .find(|layer| layer.id == Some(id))
            .and_then(|layer| {
                Some((
                    layer.effective_gds_layer()?,
                    layer.gds_datatype,
                    layer.gds_texttype,
                ))
            })
    }
}

impl TechnologyLayer {
    pub fn effective_gds_layer(&self) -> Option<u16> {
        self.gds_layer
            .or_else(|| self.id.and_then(|id| u16::try_from(id.0).ok()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub layer: LayerId,
    pub net: Option<NetId>,
    pub kind: ShapeKind,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    Rectangle(Rect),
    Polygon(Polygon),
    Path {
        points: Vec<Point>,
        width: Coord,
    },
    Via {
        center: Point,
        size: Coord,
        lower: LayerId,
        upper: LayerId,
    },
    Label {
        position: Point,
        text: String,
    },
    Measurement {
        a: Point,
        b: Point,
        label: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ShapeView<'a> {
    pub id: ShapeId,
    pub layer: LayerId,
    pub net: Option<NetId>,
    pub kind: ShapeKindView<'a>,
    pub name: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub enum ShapeKindView<'a> {
    Rectangle(Rect),
    Polygon(&'a Polygon),
    Path {
        points: &'a [Point],
        width: Coord,
    },
    Via {
        center: Point,
        size: Coord,
        lower: LayerId,
        upper: LayerId,
    },
    Label {
        position: Point,
        text: &'a str,
    },
    Measurement {
        a: Point,
        b: Point,
        label: &'a str,
    },
}

impl ShapeView<'_> {
    pub fn bounds(self) -> Rect {
        self.kind.bounds()
    }

    pub fn to_shape(self) -> Shape {
        Shape {
            id: self.id,
            layer: self.layer,
            net: self.net,
            kind: self.kind.to_shape_kind(),
            name: self.name.map(str::to_owned),
        }
    }
}

impl ShapeKindView<'_> {
    pub fn bounds(self) -> Rect {
        match self {
            Self::Rectangle(rect) => rect,
            Self::Polygon(poly) => poly.bounds().unwrap_or_default(),
            Self::Path { points, width } => Rect::from_points(points)
                .unwrap_or_default()
                .expanded(width / 2),
            Self::Via { center, size, .. } => {
                let half = size / 2;
                Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                )
            }
            Self::Label { position, .. } => Rect::new(position, position).expanded(80),
            Self::Measurement { a, b, .. } => Rect::new(a, b).expanded(40),
        }
    }

    pub fn to_shape_kind(self) -> ShapeKind {
        match self {
            Self::Rectangle(rect) => ShapeKind::Rectangle(rect),
            Self::Polygon(poly) => ShapeKind::Polygon(poly.clone()),
            Self::Path { points, width } => ShapeKind::Path {
                points: points.to_vec(),
                width,
            },
            Self::Via {
                center,
                size,
                lower,
                upper,
            } => ShapeKind::Via {
                center,
                size,
                lower,
                upper,
            },
            Self::Label { position, text } => ShapeKind::Label {
                position,
                text: text.to_string(),
            },
            Self::Measurement { a, b, label } => ShapeKind::Measurement {
                a,
                b,
                label: label.to_string(),
            },
        }
    }
}

const MAX_DENSE_SHAPE_ID_INDEX: usize = 10_000_000;

#[derive(Clone, Debug, Default)]
pub struct ShapeStore {
    alive: Vec<bool>,
    ids: Vec<ShapeId>,
    layers: Vec<LayerId>,
    nets: Vec<Option<NetId>>,
    geometry: Vec<ShapeGeometryRef>,
    names: Vec<Option<String>>,
    rectangles: Vec<Rect>,
    polygons: Vec<Polygon>,
    paths: Vec<PathGeometry>,
    vias: Vec<ViaGeometry>,
    labels: Vec<LabelGeometry>,
    measurements: Vec<MeasurementGeometry>,
    id_to_row: Vec<Option<usize>>,
    overflow_id_to_row: BTreeMap<ShapeId, usize>,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShapeGeometryRef {
    Rectangle(usize),
    Polygon(usize),
    Path(usize),
    Via(usize),
    Label(usize),
    Measurement(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathGeometry {
    points: Vec<Point>,
    width: Coord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ViaGeometry {
    center: Point,
    size: Coord,
    lower: LayerId,
    upper: LayerId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LabelGeometry {
    position: Point,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MeasurementGeometry {
    a: Point,
    b: Point,
    label: String,
}

pub struct ShapeMut<'a> {
    store: &'a mut ShapeStore,
    row: usize,
    old_id: ShapeId,
    shape: Shape,
}

impl ShapeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            alive: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            layers: Vec::with_capacity(capacity),
            nets: Vec::with_capacity(capacity),
            geometry: Vec::with_capacity(capacity),
            names: Vec::with_capacity(capacity),
            rectangles: Vec::new(),
            polygons: Vec::new(),
            paths: Vec::new(),
            vias: Vec::new(),
            labels: Vec::new(),
            measurements: Vec::new(),
            id_to_row: Vec::new(),
            overflow_id_to_row: BTreeMap::new(),
            len: 0,
        }
    }

    pub fn from_shapes(shapes: impl IntoIterator<Item = Shape>) -> Self {
        let shapes = shapes.into_iter().collect::<Vec<_>>();
        let mut store = Self::with_capacity(shapes.len());
        let dense_len = shapes
            .iter()
            .filter_map(|shape| dense_shape_index(shape.id))
            .max()
            .map_or(0, |index| index + 1);
        store.id_to_row.resize(dense_len, None);

        for shape in shapes {
            let row = store.ids.len();
            store.set_row_for_id(shape.id, row);
            store.push_row(shape);
            store.len += 1;
        }
        store
    }

    pub fn from_dense_id_range(
        first_id: ShapeId,
        count: usize,
        mut make_shape: impl FnMut(usize, ShapeId) -> Shape,
    ) -> Self {
        let mut store = Self::with_capacity(count);
        if count == 0 {
            return store;
        }

        if let Some(last_id) = first_id.0.checked_add(count as u64 - 1)
            && let Some(last_index) = dense_shape_index(ShapeId(last_id))
        {
            store.id_to_row.resize(last_index + 1, None);
        }

        for index in 0..count {
            let id = ShapeId(first_id.0 + index as u64);
            let mut shape = make_shape(index, id);
            shape.id = id;
            let row = store.ids.len();
            store.set_row_for_id(id, row);
            store.push_row(shape);
            store.len += 1;
        }
        store
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.alive.clear();
        self.ids.clear();
        self.layers.clear();
        self.nets.clear();
        self.geometry.clear();
        self.names.clear();
        self.rectangles.clear();
        self.polygons.clear();
        self.paths.clear();
        self.vias.clear();
        self.labels.clear();
        self.measurements.clear();
        self.id_to_row.clear();
        self.overflow_id_to_row.clear();
        self.len = 0;
    }

    pub fn get(&self, id: &ShapeId) -> Option<Shape> {
        self.row_for_id(*id).map(|row| self.materialize_row(row))
    }

    pub fn view(&self, id: &ShapeId) -> Option<ShapeView<'_>> {
        self.row_for_id(*id).map(|row| self.row_view(row))
    }

    pub fn get_mut(&mut self, id: &ShapeId) -> Option<ShapeMut<'_>> {
        let row = self.row_for_id(*id)?;
        Some(ShapeMut {
            old_id: *id,
            shape: self.materialize_row(row),
            store: self,
            row,
        })
    }

    pub fn contains_key(&self, id: &ShapeId) -> bool {
        self.get(id).is_some()
    }

    pub fn insert(&mut self, id: ShapeId, mut shape: Shape) -> Option<Shape> {
        shape.id = id;
        if let Some(row) = self.row_for_id(id)
            && self.alive.get(row).copied().unwrap_or(false)
        {
            let old_shape = self.materialize_row(row);
            self.write_row(row, shape);
            return Some(old_shape);
        }

        let row = self.ids.len();
        self.set_row_for_id(id, row);
        self.push_row(shape);
        self.len += 1;
        None
    }

    pub fn remove(&mut self, id: &ShapeId) -> Option<Shape> {
        let row = self.row_for_id(*id)?;
        self.clear_row_for_id(*id);
        if !self.alive.get(row).copied().unwrap_or(false) {
            return None;
        }
        let removed = self.materialize_row(row);
        self.alive[row] = false;
        self.len -= 1;
        Some(removed)
    }

    pub fn values(&self) -> impl Iterator<Item = Shape> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| self.materialize_row(row))
    }

    pub fn keys(&self) -> impl Iterator<Item = &ShapeId> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(_, id)| id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ShapeId, Shape)> + '_ {
        self.values().map(|shape| (shape.id, shape))
    }

    fn live_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| row)
    }

    fn row_id(&self, row: usize) -> ShapeId {
        self.ids[row]
    }

    fn row_layer(&self, row: usize) -> LayerId {
        self.layers[row]
    }

    fn row_bounds(&self, row: usize) -> Rect {
        self.geometry_bounds(self.geometry[row])
    }

    fn row_view(&self, row: usize) -> ShapeView<'_> {
        ShapeView {
            id: self.ids[row],
            layer: self.layers[row],
            net: self.nets[row],
            kind: self.geometry_view(self.geometry[row]),
            name: self.names[row].as_deref(),
        }
    }

    fn row_for_id(&self, id: ShapeId) -> Option<usize> {
        dense_shape_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    fn set_row_for_id(&mut self, id: ShapeId, row: usize) {
        if let Some(index) = dense_shape_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            self.overflow_id_to_row.insert(id, row);
        }
    }

    fn clear_row_for_id(&mut self, id: ShapeId) {
        if let Some(index) = dense_shape_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }

    fn push_row(&mut self, shape: Shape) {
        self.alive.push(true);
        self.ids.push(shape.id);
        self.layers.push(shape.layer);
        self.nets.push(shape.net);
        let geometry = self.push_geometry(shape.kind);
        self.geometry.push(geometry);
        self.names.push(shape.name);
    }

    fn materialize_row(&self, row: usize) -> Shape {
        Shape {
            id: self.ids[row],
            layer: self.layers[row],
            net: self.nets[row],
            kind: self.materialize_geometry(self.geometry[row]),
            name: self.names[row].clone(),
        }
    }

    fn write_row(&mut self, row: usize, shape: Shape) {
        let old_id = self.ids[row];
        if old_id != shape.id {
            self.clear_row_for_id(old_id);
            self.set_row_for_id(shape.id, row);
        }
        self.ids[row] = shape.id;
        self.layers[row] = shape.layer;
        self.nets[row] = shape.net;
        self.geometry[row] = self.push_geometry(shape.kind);
        self.names[row] = shape.name;
        self.alive[row] = true;
    }

    fn push_geometry(&mut self, kind: ShapeKind) -> ShapeGeometryRef {
        match kind {
            ShapeKind::Rectangle(rect) => {
                let index = self.rectangles.len();
                self.rectangles.push(rect);
                ShapeGeometryRef::Rectangle(index)
            }
            ShapeKind::Polygon(poly) => {
                let index = self.polygons.len();
                self.polygons.push(poly);
                ShapeGeometryRef::Polygon(index)
            }
            ShapeKind::Path { points, width } => {
                let index = self.paths.len();
                self.paths.push(PathGeometry { points, width });
                ShapeGeometryRef::Path(index)
            }
            ShapeKind::Via {
                center,
                size,
                lower,
                upper,
            } => {
                let index = self.vias.len();
                self.vias.push(ViaGeometry {
                    center,
                    size,
                    lower,
                    upper,
                });
                ShapeGeometryRef::Via(index)
            }
            ShapeKind::Label { position, text } => {
                let index = self.labels.len();
                self.labels.push(LabelGeometry { position, text });
                ShapeGeometryRef::Label(index)
            }
            ShapeKind::Measurement { a, b, label } => {
                let index = self.measurements.len();
                self.measurements.push(MeasurementGeometry { a, b, label });
                ShapeGeometryRef::Measurement(index)
            }
        }
    }

    fn materialize_geometry(&self, geometry: ShapeGeometryRef) -> ShapeKind {
        match geometry {
            ShapeGeometryRef::Rectangle(index) => ShapeKind::Rectangle(self.rectangles[index]),
            ShapeGeometryRef::Polygon(index) => ShapeKind::Polygon(self.polygons[index].clone()),
            ShapeGeometryRef::Path(index) => {
                let path = &self.paths[index];
                ShapeKind::Path {
                    points: path.points.clone(),
                    width: path.width,
                }
            }
            ShapeGeometryRef::Via(index) => {
                let via = &self.vias[index];
                ShapeKind::Via {
                    center: via.center,
                    size: via.size,
                    lower: via.lower,
                    upper: via.upper,
                }
            }
            ShapeGeometryRef::Label(index) => {
                let label = &self.labels[index];
                ShapeKind::Label {
                    position: label.position,
                    text: label.text.clone(),
                }
            }
            ShapeGeometryRef::Measurement(index) => {
                let measurement = &self.measurements[index];
                ShapeKind::Measurement {
                    a: measurement.a,
                    b: measurement.b,
                    label: measurement.label.clone(),
                }
            }
        }
    }

    fn geometry_view(&self, geometry: ShapeGeometryRef) -> ShapeKindView<'_> {
        match geometry {
            ShapeGeometryRef::Rectangle(index) => ShapeKindView::Rectangle(self.rectangles[index]),
            ShapeGeometryRef::Polygon(index) => ShapeKindView::Polygon(&self.polygons[index]),
            ShapeGeometryRef::Path(index) => {
                let path = &self.paths[index];
                ShapeKindView::Path {
                    points: &path.points,
                    width: path.width,
                }
            }
            ShapeGeometryRef::Via(index) => {
                let via = &self.vias[index];
                ShapeKindView::Via {
                    center: via.center,
                    size: via.size,
                    lower: via.lower,
                    upper: via.upper,
                }
            }
            ShapeGeometryRef::Label(index) => {
                let label = &self.labels[index];
                ShapeKindView::Label {
                    position: label.position,
                    text: &label.text,
                }
            }
            ShapeGeometryRef::Measurement(index) => {
                let measurement = &self.measurements[index];
                ShapeKindView::Measurement {
                    a: measurement.a,
                    b: measurement.b,
                    label: &measurement.label,
                }
            }
        }
    }

    fn geometry_bounds(&self, geometry: ShapeGeometryRef) -> Rect {
        match geometry {
            ShapeGeometryRef::Rectangle(index) => self.rectangles[index],
            ShapeGeometryRef::Polygon(index) => self.polygons[index].bounds().unwrap_or_default(),
            ShapeGeometryRef::Path(index) => Rect::from_points(&self.paths[index].points)
                .unwrap_or_default()
                .expanded(self.paths[index].width / 2),
            ShapeGeometryRef::Via(index) => {
                let via = &self.vias[index];
                let half = via.size / 2;
                Rect::new(
                    Point::new(via.center.x - half, via.center.y - half),
                    Point::new(via.center.x + half, via.center.y + half),
                )
            }
            ShapeGeometryRef::Label(index) => {
                Rect::new(self.labels[index].position, self.labels[index].position).expanded(80)
            }
            ShapeGeometryRef::Measurement(index) => {
                let measurement = &self.measurements[index];
                Rect::new(measurement.a, measurement.b).expanded(40)
            }
        }
    }
}

impl std::ops::Deref for ShapeMut<'_> {
    type Target = Shape;

    fn deref(&self) -> &Self::Target {
        &self.shape
    }
}

impl std::ops::DerefMut for ShapeMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shape
    }
}

impl Drop for ShapeMut<'_> {
    fn drop(&mut self) {
        if self.shape.id != self.old_id {
            self.store.clear_row_for_id(self.old_id);
        }
        self.store.write_row(self.row, self.shape.clone());
    }
}

impl Extend<(ShapeId, Shape)> for ShapeStore {
    fn extend<T: IntoIterator<Item = (ShapeId, Shape)>>(&mut self, iter: T) {
        for (id, shape) in iter {
            self.insert(id, shape);
        }
    }
}

impl FromIterator<(ShapeId, Shape)> for ShapeStore {
    fn from_iter<T: IntoIterator<Item = (ShapeId, Shape)>>(iter: T) -> Self {
        Self::from_shapes(iter.into_iter().map(|(id, mut shape)| {
            shape.id = id;
            shape
        }))
    }
}

impl Serialize for ShapeStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len))?;
        for shape in self.values() {
            map.serialize_entry(&shape.id, &shape)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ShapeStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let shapes = BTreeMap::<ShapeId, Shape>::deserialize(deserializer)?;
        Ok(shapes.into_iter().collect())
    }
}

fn dense_shape_index(id: ShapeId) -> Option<usize> {
    let index = usize::try_from(id.0).ok()?;
    (index <= MAX_DENSE_SHAPE_ID_INDEX).then_some(index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transform {
    #[serde(default = "identity_transform_matrix")]
    pub matrix: [i8; 4],
    pub translation: Vector,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        matrix: [1, 0, 0, 1],
        translation: Vector::ZERO,
    };

    pub const fn from_translation(translation: Vector) -> Self {
        Self {
            matrix: [1, 0, 0, 1],
            translation,
        }
    }

    pub const fn translate(dx: Coord, dy: Coord) -> Self {
        Self {
            matrix: [1, 0, 0, 1],
            translation: Vector::new(dx, dy),
        }
    }

    pub const fn rotate_cw90() -> Self {
        Self {
            matrix: [0, 1, -1, 0],
            translation: Vector::ZERO,
        }
    }

    pub const fn mirror_x() -> Self {
        Self {
            matrix: [-1, 0, 0, 1],
            translation: Vector::ZERO,
        }
    }

    pub const fn mirror_y() -> Self {
        Self {
            matrix: [1, 0, 0, -1],
            translation: Vector::ZERO,
        }
    }

    pub fn with_translation(mut self, translation: Vector) -> Self {
        self.translation = translation;
        self
    }

    pub fn compose(self, child: Self) -> Self {
        let [a, b, c, d] = self.matrix;
        let [e, f, g, h] = child.matrix;
        let child_translation = self.apply_vector(child.translation);
        Self {
            matrix: [a * e + b * g, a * f + b * h, c * e + d * g, c * f + d * h],
            translation: Vector::new(
                child_translation.dx + self.translation.dx,
                child_translation.dy + self.translation.dy,
            ),
        }
    }

    pub fn apply_point(self, point: Point) -> Point {
        let [a, b, c, d] = self.matrix;
        Point::new(
            Coord::from(a) * point.x + Coord::from(b) * point.y + self.translation.dx,
            Coord::from(c) * point.x + Coord::from(d) * point.y + self.translation.dy,
        )
    }

    pub fn apply_rect(self, rect: Rect) -> Rect {
        let corners = rect.corners().map(|point| self.apply_point(point));
        Rect::from_points(&corners)
            .unwrap_or_else(|| Rect::new(self.apply_point(rect.min), self.apply_point(rect.max)))
    }

    pub fn apply_shape_kind(self, kind: &ShapeKind) -> ShapeKind {
        match kind {
            ShapeKind::Rectangle(rect) => ShapeKind::Rectangle(self.apply_rect(*rect)),
            ShapeKind::Polygon(poly) => ShapeKind::Polygon(Polygon::new(
                poly.points
                    .iter()
                    .copied()
                    .map(|point| self.apply_point(point))
                    .collect(),
            )),
            ShapeKind::Path { points, width } => ShapeKind::Path {
                points: points
                    .iter()
                    .copied()
                    .map(|point| self.apply_point(point))
                    .collect(),
                width: *width,
            },
            ShapeKind::Via {
                center,
                size,
                lower,
                upper,
            } => ShapeKind::Via {
                center: self.apply_point(*center),
                size: *size,
                lower: *lower,
                upper: *upper,
            },
            ShapeKind::Label { position, text } => ShapeKind::Label {
                position: self.apply_point(*position),
                text: text.clone(),
            },
            ShapeKind::Measurement { a, b, label } => ShapeKind::Measurement {
                a: self.apply_point(*a),
                b: self.apply_point(*b),
                label: label.clone(),
            },
        }
    }

    fn apply_vector(self, vector: Vector) -> Vector {
        let [a, b, c, d] = self.matrix;
        Vector::new(
            Coord::from(a) * vector.dx + Coord::from(b) * vector.dy,
            Coord::from(c) * vector.dx + Coord::from(d) * vector.dy,
        )
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

fn identity_transform_matrix() -> [i8; 4] {
    [1, 0, 0, 1]
}

impl ShapeKind {
    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rectangle(rect) => *rect,
            Self::Polygon(poly) => poly.bounds().unwrap_or_default(),
            Self::Path { points, width } => Rect::from_points(points)
                .unwrap_or_default()
                .expanded(*width / 2),
            Self::Via { center, size, .. } => {
                let half = *size / 2;
                Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                )
            }
            Self::Label { position, .. } => Rect::new(*position, *position).expanded(80),
            Self::Measurement { a, b, .. } => Rect::new(*a, *b).expanded(40),
        }
    }

    pub fn translate(&mut self, delta: Vector) {
        match self {
            Self::Rectangle(rect) => *rect = rect.translated(delta),
            Self::Polygon(poly) => poly.translate(delta),
            Self::Path { points, .. } => {
                for point in points {
                    *point = point.translated(delta);
                }
            }
            Self::Via { center, .. }
            | Self::Label {
                position: center, ..
            } => *center = center.translated(delta),
            Self::Measurement { a, b, .. } => {
                *a = a.translated(delta);
                *b = b.translated(delta);
            }
        }
    }

    pub fn key_points(&self) -> Vec<Point> {
        match self {
            Self::Rectangle(rect) => rect.corners().to_vec(),
            Self::Polygon(poly) => poly.points.clone(),
            Self::Path { points, .. } => points.clone(),
            Self::Via { center, .. }
            | Self::Label {
                position: center, ..
            } => vec![*center],
            Self::Measurement { a, b, .. } => vec![*a, *b],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub id: CellId,
    pub name: String,
    pub shapes: ShapeStore,
    pub instances: InstanceStore,
}

impl Cell {
    pub fn new(id: CellId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            shapes: ShapeStore::new(),
            instances: InstanceStore::new(),
        }
    }
}

const MAX_DENSE_CELL_ID_INDEX: usize = 10_000_000;

#[derive(Clone, Debug, Default)]
pub struct CellStore {
    alive: Vec<bool>,
    ids: Vec<CellId>,
    cells: Vec<Cell>,
    id_to_row: Vec<Option<usize>>,
    overflow_id_to_row: BTreeMap<CellId, usize>,
    len: usize,
}

impl CellStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            alive: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            cells: Vec::with_capacity(capacity),
            id_to_row: Vec::new(),
            overflow_id_to_row: BTreeMap::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.alive.clear();
        self.ids.clear();
        self.cells.clear();
        self.id_to_row.clear();
        self.overflow_id_to_row.clear();
        self.len = 0;
    }

    pub fn get(&self, id: &CellId) -> Option<&Cell> {
        self.row_for_id(*id).map(|row| &self.cells[row])
    }

    pub fn get_mut(&mut self, id: &CellId) -> Option<&mut Cell> {
        let row = self.row_for_id(*id)?;
        Some(&mut self.cells[row])
    }

    pub fn contains_key(&self, id: &CellId) -> bool {
        self.row_for_id(*id).is_some()
    }

    pub fn insert(&mut self, id: CellId, mut cell: Cell) -> Option<Cell> {
        cell.id = id;
        if let Some(row) = self.row_for_id(id)
            && self.alive.get(row).copied().unwrap_or(false)
        {
            return Some(std::mem::replace(&mut self.cells[row], cell));
        }

        let row = self.ids.len();
        self.set_row_for_id(id, row);
        self.alive.push(true);
        self.ids.push(id);
        self.cells.push(cell);
        self.len += 1;
        None
    }

    pub fn remove(&mut self, id: &CellId) -> Option<Cell> {
        let row = self.row_for_id(*id)?;
        self.clear_row_for_id(*id);
        if !self.alive.get(row).copied().unwrap_or(false) {
            return None;
        }
        self.alive[row] = false;
        self.len -= 1;
        Some(self.cells[row].clone())
    }

    pub fn values(&self) -> impl Iterator<Item = &Cell> {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| &self.cells[row])
    }

    pub fn keys(&self) -> impl Iterator<Item = &CellId> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(_, id)| id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&CellId, &Cell)> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(row, id)| (id, &self.cells[row]))
    }

    fn row_for_id(&self, id: CellId) -> Option<usize> {
        dense_cell_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    fn set_row_for_id(&mut self, id: CellId, row: usize) {
        if let Some(index) = dense_cell_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            self.overflow_id_to_row.insert(id, row);
        }
    }

    fn clear_row_for_id(&mut self, id: CellId) {
        if let Some(index) = dense_cell_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }
}

impl Extend<(CellId, Cell)> for CellStore {
    fn extend<T: IntoIterator<Item = (CellId, Cell)>>(&mut self, iter: T) {
        for (id, cell) in iter {
            self.insert(id, cell);
        }
    }
}

impl FromIterator<(CellId, Cell)> for CellStore {
    fn from_iter<T: IntoIterator<Item = (CellId, Cell)>>(iter: T) -> Self {
        let mut store = Self::new();
        for (id, cell) in iter {
            store.insert(id, cell);
        }
        store
    }
}

impl Serialize for CellStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len))?;
        for (id, cell) in self.iter() {
            map.serialize_entry(id, cell)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for CellStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cells = BTreeMap::<CellId, Cell>::deserialize(deserializer)?;
        Ok(cells.into_iter().collect())
    }
}

fn dense_cell_index(id: CellId) -> Option<usize> {
    let index = usize::try_from(id.0).ok()?;
    (index <= MAX_DENSE_CELL_ID_INDEX).then_some(index)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellInstance {
    pub id: InstanceId,
    pub name: Option<String>,
    pub cell: CellId,
    pub transform: Transform,
    #[serde(default)]
    pub array: InstanceArray,
}

#[derive(Clone, Debug, Default)]
pub struct InstanceStore {
    alive: Vec<bool>,
    ids: Vec<InstanceId>,
    names: Vec<Option<String>>,
    cells: Vec<CellId>,
    transforms: Vec<Transform>,
    arrays: Vec<InstanceArray>,
    id_to_row: Vec<Option<usize>>,
    overflow_id_to_row: BTreeMap<InstanceId, usize>,
    len: usize,
}

pub struct InstanceMut<'a> {
    store: &'a mut InstanceStore,
    row: usize,
    old_id: InstanceId,
    instance: CellInstance,
}

impl InstanceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            alive: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            names: Vec::with_capacity(capacity),
            cells: Vec::with_capacity(capacity),
            transforms: Vec::with_capacity(capacity),
            arrays: Vec::with_capacity(capacity),
            id_to_row: Vec::new(),
            overflow_id_to_row: BTreeMap::new(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.alive.clear();
        self.ids.clear();
        self.names.clear();
        self.cells.clear();
        self.transforms.clear();
        self.arrays.clear();
        self.id_to_row.clear();
        self.overflow_id_to_row.clear();
        self.len = 0;
    }

    pub fn get(&self, id: &InstanceId) -> Option<CellInstance> {
        self.row_for_id(*id).map(|row| self.materialize_row(row))
    }

    pub fn get_mut(&mut self, id: &InstanceId) -> Option<InstanceMut<'_>> {
        let row = self.row_for_id(*id)?;
        Some(InstanceMut {
            old_id: *id,
            instance: self.materialize_row(row),
            store: self,
            row,
        })
    }

    pub fn contains_key(&self, id: &InstanceId) -> bool {
        self.get(id).is_some()
    }

    pub fn insert(&mut self, id: InstanceId, mut instance: CellInstance) -> Option<CellInstance> {
        instance.id = id;
        if let Some(row) = self.row_for_id(id)
            && self.alive.get(row).copied().unwrap_or(false)
        {
            let old = self.materialize_row(row);
            self.write_row(row, instance);
            return Some(old);
        }

        let row = self.ids.len();
        self.set_row_for_id(id, row);
        self.push_row(instance);
        self.len += 1;
        None
    }

    pub fn remove(&mut self, id: &InstanceId) -> Option<CellInstance> {
        let row = self.row_for_id(*id)?;
        self.clear_row_for_id(*id);
        if !self.alive.get(row).copied().unwrap_or(false) {
            return None;
        }
        let removed = self.materialize_row(row);
        self.alive[row] = false;
        self.len -= 1;
        Some(removed)
    }

    pub fn values(&self) -> impl Iterator<Item = CellInstance> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| self.materialize_row(row))
    }

    pub fn keys(&self) -> impl Iterator<Item = &InstanceId> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(row, _)| self.alive.get(*row).copied().unwrap_or(false))
            .map(|(_, id)| id)
    }

    fn live_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| row)
    }

    fn row_id(&self, row: usize) -> InstanceId {
        self.ids[row]
    }

    fn row_cell(&self, row: usize) -> CellId {
        self.cells[row]
    }

    fn row_transform(&self, row: usize) -> Transform {
        self.transforms[row]
    }

    fn row_array(&self, row: usize) -> InstanceArray {
        self.arrays[row]
    }

    fn row_for_id(&self, id: InstanceId) -> Option<usize> {
        dense_instance_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    fn set_row_for_id(&mut self, id: InstanceId, row: usize) {
        if let Some(index) = dense_instance_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            self.overflow_id_to_row.insert(id, row);
        }
    }

    fn clear_row_for_id(&mut self, id: InstanceId) {
        if let Some(index) = dense_instance_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }

    fn push_row(&mut self, instance: CellInstance) {
        self.alive.push(true);
        self.ids.push(instance.id);
        self.names.push(instance.name);
        self.cells.push(instance.cell);
        self.transforms.push(instance.transform);
        self.arrays.push(instance.array);
    }

    fn materialize_row(&self, row: usize) -> CellInstance {
        CellInstance {
            id: self.ids[row],
            name: self.names[row].clone(),
            cell: self.cells[row],
            transform: self.transforms[row],
            array: self.arrays[row],
        }
    }

    fn write_row(&mut self, row: usize, instance: CellInstance) {
        let old_id = self.ids[row];
        if old_id != instance.id {
            self.clear_row_for_id(old_id);
            self.set_row_for_id(instance.id, row);
        }
        self.ids[row] = instance.id;
        self.names[row] = instance.name;
        self.cells[row] = instance.cell;
        self.transforms[row] = instance.transform;
        self.arrays[row] = instance.array;
        self.alive[row] = true;
    }
}

impl Extend<(InstanceId, CellInstance)> for InstanceStore {
    fn extend<T: IntoIterator<Item = (InstanceId, CellInstance)>>(&mut self, iter: T) {
        for (id, instance) in iter {
            self.insert(id, instance);
        }
    }
}

impl FromIterator<(InstanceId, CellInstance)> for InstanceStore {
    fn from_iter<T: IntoIterator<Item = (InstanceId, CellInstance)>>(iter: T) -> Self {
        let mut store = Self::new();
        for (id, instance) in iter {
            store.insert(id, instance);
        }
        store
    }
}

impl Serialize for InstanceStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len))?;
        for instance in self.values() {
            map.serialize_entry(&instance.id, &instance)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for InstanceStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let instances = BTreeMap::<InstanceId, CellInstance>::deserialize(deserializer)?;
        Ok(instances.into_iter().collect())
    }
}

impl std::ops::Deref for InstanceMut<'_> {
    type Target = CellInstance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl std::ops::DerefMut for InstanceMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.instance
    }
}

impl Drop for InstanceMut<'_> {
    fn drop(&mut self) {
        if self.instance.id != self.old_id {
            self.store.clear_row_for_id(self.old_id);
        }
        self.store.write_row(self.row, self.instance.clone());
    }
}

fn dense_instance_index(id: InstanceId) -> Option<usize> {
    usize::try_from(id.0).ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceArray {
    #[serde(default = "default_array_count")]
    pub columns: u32,
    #[serde(default = "default_array_count")]
    pub rows: u32,
    #[serde(default)]
    pub column_pitch: Vector,
    #[serde(default)]
    pub row_pitch: Vector,
}

impl InstanceArray {
    pub const fn single() -> Self {
        Self {
            columns: 1,
            rows: 1,
            column_pitch: Vector::ZERO,
            row_pitch: Vector::ZERO,
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            columns: self.columns.max(1),
            rows: self.rows.max(1),
            column_pitch: self.column_pitch,
            row_pitch: self.row_pitch,
        }
    }

    pub fn is_single(self) -> bool {
        let normalized = self.normalized();
        normalized.columns == 1 && normalized.rows == 1
    }

    pub fn element_offset(self, column: u32, row: u32) -> Vector {
        let normalized = self.normalized();
        Vector::new(
            normalized.column_pitch.dx * Coord::from(column)
                + normalized.row_pitch.dx * Coord::from(row),
            normalized.column_pitch.dy * Coord::from(column)
                + normalized.row_pitch.dy * Coord::from(row),
        )
    }
}

impl Default for InstanceArray {
    fn default() -> Self {
        Self::single()
    }
}

fn default_array_count() -> u32 {
    1
}

#[derive(Clone, Debug)]
pub struct FlattenedShape {
    pub id: ShapeOccurrenceId,
    pub shape: Shape,
    pub source_cell: CellId,
    pub instance_path: Vec<InstanceId>,
    pub transform: Transform,
    pub bounds: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct FlattenedShapeView<'a> {
    pub shape: ShapeView<'a>,
    pub source_cell: CellId,
    pub transform: Transform,
    pub bounds: Rect,
}

impl FlattenedShapeView<'_> {
    pub fn transformed_shape(self) -> Shape {
        let mut shape = self.shape.to_shape();
        shape.kind = self.transform.apply_shape_kind(&shape.kind);
        shape
    }
}

impl FlattenedShape {
    pub fn source_shape_id(&self) -> ShapeId {
        self.id.source_shape_id()
    }

    pub fn transformed_shape(&self) -> Shape {
        let mut shape = self.shape.clone();
        shape.kind = self.transform.apply_shape_kind(&shape.kind);
        shape
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    Batch {
        operations: Vec<Operation>,
    },
    AddShape {
        shape: Shape,
    },
    DeleteShape {
        id: ShapeId,
    },
    ReplaceShape {
        id: ShapeId,
        shape: Shape,
    },
    MoveShape {
        id: ShapeId,
        delta: Vector,
    },
    AddCell {
        cell: Cell,
    },
    DeleteCell {
        id: CellId,
    },
    RenameCell {
        id: CellId,
        name: String,
    },
    AddInstance {
        parent: CellId,
        instance: CellInstance,
    },
    ReplaceInstance {
        parent: CellId,
        id: InstanceId,
        instance: CellInstance,
    },
    DeleteInstance {
        parent: CellId,
        id: InstanceId,
    },
    RenameInstance {
        parent: CellId,
        id: InstanceId,
        name: Option<String>,
    },
    MoveInstance {
        parent: CellId,
        id: InstanceId,
        delta: Vector,
    },
    AddLayer {
        layer: Layer,
    },
    SetLayerVisibility {
        layer: LayerId,
        visible: bool,
    },
    Cursor {
        user: Uuid,
        position: Point,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggedOperation {
    pub sequence: u64,
    pub user: Uuid,
    pub operation: Operation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerState {
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub waived: bool,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        user: Uuid,
    },
    Operation {
        user: Uuid,
        operation: Operation,
    },
    CrdtOperation {
        operation: CrdtOperation,
    },
    LoroUpdate {
        update: LoroUpdate,
    },
    Cursor {
        user: Uuid,
        position: Point,
    },
    Selection {
        user: Uuid,
        selection: Vec<ShapeOccurrenceId>,
    },
    RequestSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Snapshot {
        document: Document,
        cursors: BTreeMap<Uuid, Point>,
        #[serde(default)]
        selections: BTreeMap<Uuid, Vec<ShapeOccurrenceId>>,
        #[serde(default)]
        loro_snapshot: Vec<u8>,
    },
    Operation {
        operation: LoggedOperation,
    },
    CrdtOperation {
        operation: CrdtOperation,
    },
    LoroUpdate {
        update: LoroUpdate,
    },
    Cursor {
        user: Uuid,
        position: Point,
    },
    Selection {
        user: Uuid,
        selection: Vec<ShapeOccurrenceId>,
    },
    UserJoined {
        user: Uuid,
    },
    UserLeft {
        user: Uuid,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub grid: Coord,
    pub next_shape_id: u64,
    pub next_layer_id: u32,
    #[serde(default = "default_next_cell_id")]
    pub next_cell_id: u64,
    #[serde(default = "default_next_instance_id")]
    pub next_instance_id: u64,
    #[serde(default = "default_top_cell")]
    pub top_cell: CellId,
    pub layers: LayerStore,
    pub shapes: ShapeStore,
    #[serde(default)]
    pub cells: CellStore,
    #[serde(default)]
    pub marker_states: BTreeMap<String, MarkerState>,
    #[serde(default)]
    pub crdt_seen: BTreeSet<CrdtOpId>,
    #[serde(default)]
    pub crdt_actor_clocks: BTreeMap<Uuid, u64>,
    #[serde(default)]
    pub crdt_operation_log: Vec<CrdtOperation>,
    pub operation_log: Vec<LoggedOperation>,
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        let technology = default_technology();
        Self::from_technology(name, &technology).expect("built-in default technology is valid")
    }

    pub fn from_technology(
        name: impl Into<String>,
        technology: &TechnologyFile,
    ) -> Result<Self, TechnologyError> {
        technology.validate()?;
        let mut document = Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            name: name.into(),
            grid: technology.grid,
            next_shape_id: 1,
            next_layer_id: 1,
            next_cell_id: default_next_cell_id(),
            next_instance_id: default_next_instance_id(),
            top_cell: default_top_cell(),
            layers: LayerStore::new(),
            shapes: ShapeStore::new(),
            cells: CellStore::new(),
            marker_states: BTreeMap::new(),
            crdt_seen: BTreeSet::new(),
            crdt_actor_clocks: BTreeMap::new(),
            crdt_operation_log: Vec::new(),
            operation_log: Vec::new(),
        };
        document.ensure_hierarchy();
        document.apply_technology(technology)?;
        Ok(document)
    }

    pub fn apply_technology(&mut self, technology: &TechnologyFile) -> Result<(), TechnologyError> {
        technology.validate()?;
        self.grid = technology.grid;
        self.layers.clear();
        self.next_layer_id = 1;
        for technology_layer in &technology.layers {
            let process = ProcessLayer::from_technology_name(&technology_layer.process)
                .ok_or_else(|| {
                    TechnologyError::Invalid(format!(
                        "layer {:?} has unknown process {:?}",
                        technology_layer.name, technology_layer.process
                    ))
                })?;
            let id = technology_layer.id.unwrap_or(LayerId(self.next_layer_id));
            self.next_layer_id = self.next_layer_id.max(id.0 + 1);
            self.layers.insert(
                id,
                Layer {
                    id,
                    name: technology_layer.name.clone(),
                    process,
                    purpose: technology_layer.purpose.clone(),
                    color: technology_layer.color,
                    display_order: technology_layer.display_order,
                    gds_layer: technology_layer.effective_gds_layer(),
                    gds_datatype: technology_layer.gds_datatype,
                    gds_texttype: technology_layer.gds_texttype,
                    visible: technology_layer.visible,
                    locked: technology_layer.locked,
                },
            );
        }
        Ok(())
    }

    pub fn demo() -> Self {
        let mut doc = Self::new("Fabricad demo inverter");
        let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
        let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let contact = doc.layer_by_process(ProcessLayer::Contact).unwrap();
        let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

        doc.insert_shape(
            diffusion,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2600, -700), 5200, 1400)),
        );
        doc.insert_shape(
            poly,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-180, -1200), 360, 2400)),
        );
        doc.insert_shape(
            metal1,
            ShapeKind::Path {
                points: vec![
                    Point::new(-2400, 0),
                    Point::new(-700, 0),
                    Point::new(700, 0),
                    Point::new(2400, 0),
                ],
                width: 360,
            },
        );
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2500, 1200), 5000, 320)),
        );
        doc.insert_shape(
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2500, -1540), 5000, 320)),
        );
        for x in [-1900, 1900] {
            doc.insert_shape(
                contact,
                ShapeKind::Via {
                    center: Point::new(x, 0),
                    size: 280,
                    lower: diffusion,
                    upper: metal1,
                },
            );
        }
        doc.insert_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(-2500, 1800),
                text: "tiny inverter mask sketch".to_string(),
            },
        );
        doc
    }

    pub fn hierarchy_demo() -> Self {
        let mut doc = Self::new("Fabricad hierarchy demo");
        let diffusion = doc.layer_by_process(ProcessLayer::Diffusion).unwrap();
        let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let contact = doc.layer_by_process(ProcessLayer::Contact).unwrap();
        let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
        let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

        let unit = doc.create_cell("standard cell slice");
        doc.insert_shape_in_cell(
            unit,
            diffusion,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-420, -180), 840, 360)),
        );
        doc.insert_shape_in_cell(
            unit,
            poly,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-70, -460), 140, 920)),
        );
        doc.insert_shape_in_cell(
            unit,
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(-520, 0), Point::new(520, 0)],
                width: 160,
            },
        );
        for x in [-280, 280] {
            doc.insert_shape_in_cell(
                unit,
                contact,
                ShapeKind::Via {
                    center: Point::new(x, 0),
                    size: 140,
                    lower: diffusion,
                    upper: metal1,
                },
            );
        }

        let strap = doc.create_cell("power strap");
        doc.insert_shape_in_cell(
            strap,
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-8_300, -120), 16_600, 240)),
        );
        doc.insert_shape_in_cell(
            strap,
            annotation,
            ShapeKind::Label {
                position: Point::new(-8_200, 360),
                text: "hierarchical cell instances".to_string(),
            },
        );

        let pitch_x = 1_250;
        let pitch_y = 1_000;
        for row in 0..4 {
            for col in 0..12 {
                let x = (col - 6) * pitch_x + if row % 2 == 0 { 0 } else { pitch_x / 2 };
                let y = (row - 2) * pitch_y;
                doc.insert_instance_in_top(unit, Transform::translate(x, y))
                    .unwrap();
            }
        }
        for y in [-2_000, 2_000] {
            doc.insert_instance_in_top(strap, Transform::translate(0, y))
                .unwrap();
        }

        doc
    }

    pub fn stress(count: usize) -> Self {
        let mut doc = Self::new(format!("Fabricad stress {count}"));
        let layers: Vec<LayerId> = [
            ProcessLayer::Diffusion,
            ProcessLayer::Poly,
            ProcessLayer::Metal1,
            ProcessLayer::Metal2,
            ProcessLayer::Oxide,
        ]
        .into_iter()
        .filter_map(|layer| doc.layer_by_process(layer))
        .collect();
        let columns = (count as f64).sqrt().ceil() as Coord;
        let pitch = 240;
        let first_shape_id = doc.next_shape_id;
        doc.shapes =
            ShapeStore::from_dense_id_range(ShapeId(first_shape_id), count, |index, id| {
                let i = index as Coord;
                let x = (i % columns) * pitch - columns * pitch / 2;
                let y = (i / columns) * pitch - columns * pitch / 2;
                let width = 50 + ((index % 7) as Coord) * 10;
                let height = 40 + ((index % 5) as Coord) * 12;
                let layer = layers[index % layers.len()];
                Shape {
                    id,
                    layer,
                    net: None,
                    kind: ShapeKind::Rectangle(Rect::from_min_size(
                        Point::new(x, y),
                        width,
                        height,
                    )),
                    name: None,
                }
            });
        doc.next_shape_id = first_shape_id + count as u64;
        doc
    }

    pub fn create_layer(
        &mut self,
        name: impl Into<String>,
        process: ProcessLayer,
        color: [f32; 4],
    ) -> LayerId {
        let id = LayerId(self.next_layer_id);
        self.next_layer_id += 1;
        self.layers.insert(
            id,
            Layer {
                id,
                name: name.into(),
                process,
                purpose: process.as_technology_name().to_string(),
                color,
                display_order: id.0 as i32 * 10,
                gds_layer: u16::try_from(id.0).ok(),
                gds_datatype: 0,
                gds_texttype: 0,
                visible: true,
                locked: false,
            },
        );
        id
    }

    pub fn allocate_shape_id(&mut self) -> ShapeId {
        let id = ShapeId(self.next_shape_id);
        self.next_shape_id += 1;
        id
    }

    pub fn allocate_cell_id(&mut self) -> CellId {
        self.ensure_hierarchy();
        let id = CellId(self.next_cell_id);
        self.next_cell_id += 1;
        id
    }

    pub fn allocate_instance_id(&mut self) -> InstanceId {
        self.ensure_hierarchy();
        let id = InstanceId(self.next_instance_id);
        self.next_instance_id += 1;
        id
    }

    pub fn ensure_hierarchy(&mut self) {
        if self.top_cell.0 == 0 {
            self.top_cell = default_top_cell();
        }
        if !self.cells.contains_key(&self.top_cell) {
            self.cells
                .insert(self.top_cell, Cell::new(self.top_cell, "top"));
        }

        self.next_cell_id = self
            .cells
            .keys()
            .map(|id| id.0 + 1)
            .chain([self.top_cell.0 + 1, default_next_cell_id()])
            .max()
            .unwrap_or_else(default_next_cell_id)
            .max(self.next_cell_id);

        self.next_instance_id = self
            .cells
            .values()
            .flat_map(|cell| cell.instances.keys())
            .map(|id| id.0 + 1)
            .chain([default_next_instance_id()])
            .max()
            .unwrap_or_else(default_next_instance_id)
            .max(self.next_instance_id);
    }

    pub fn insert_shape(&mut self, layer: LayerId, kind: ShapeKind) -> ShapeId {
        let id = self.allocate_shape_id();
        let shape = Shape {
            id,
            layer,
            net: None,
            kind,
            name: None,
        };
        self.shapes.insert(id, shape);
        id
    }

    pub fn insert_shape_in_cell(
        &mut self,
        cell: CellId,
        layer: LayerId,
        kind: ShapeKind,
    ) -> Option<ShapeId> {
        if cell == self.top_cell {
            return Some(self.insert_shape(layer, kind));
        }
        self.ensure_hierarchy();
        if !self.cells.contains_key(&cell) {
            return None;
        }
        let id = self.allocate_shape_id();
        let shape = Shape {
            id,
            layer,
            net: None,
            kind,
            name: None,
        };
        self.cells.get_mut(&cell)?.shapes.insert(id, shape);
        Some(id)
    }

    pub fn create_cell(&mut self, name: impl Into<String>) -> CellId {
        let id = self.allocate_cell_id();
        self.cells.insert(id, Cell::new(id, name));
        id
    }

    pub fn cell(&self, id: CellId) -> Option<&Cell> {
        self.cells.get(&id)
    }

    pub fn cell_mut(&mut self, id: CellId) -> Option<&mut Cell> {
        self.ensure_hierarchy();
        self.cells.get_mut(&id)
    }

    pub fn instance_parent_for_path(&self, path: &[InstanceId]) -> Option<(CellId, InstanceId)> {
        let mut parent = self.top_cell;
        for (index, instance_id) in path.iter().copied().enumerate() {
            let instance = self.cells.get(&parent)?.instances.get(&instance_id)?;
            if index == path.len().saturating_sub(1) {
                return Some((parent, instance_id));
            }
            parent = instance.cell;
        }
        None
    }

    pub fn instance(&self, parent: CellId, id: InstanceId) -> Option<CellInstance> {
        self.cells.get(&parent)?.instances.get(&id)
    }

    pub fn instance_mut(&mut self, parent: CellId, id: InstanceId) -> Option<InstanceMut<'_>> {
        self.ensure_hierarchy();
        self.cells.get_mut(&parent)?.instances.get_mut(&id)
    }

    pub fn insert_instance(
        &mut self,
        parent: CellId,
        cell: CellId,
        transform: Transform,
    ) -> Option<InstanceId> {
        self.ensure_hierarchy();
        if !self.cells.contains_key(&parent) || !self.cells.contains_key(&cell) {
            return None;
        }
        let id = self.allocate_instance_id();
        let instance = CellInstance {
            id,
            name: None,
            cell,
            transform,
            array: InstanceArray::single(),
        };
        self.cells.get_mut(&parent)?.instances.insert(id, instance);
        Some(id)
    }

    pub fn insert_instance_in_top(
        &mut self,
        cell: CellId,
        transform: Transform,
    ) -> Option<InstanceId> {
        self.insert_instance(self.top_cell, cell, transform)
    }

    pub fn apply_operation(&mut self, logged: LoggedOperation) {
        self.apply_operation_without_log(&logged.operation);
        self.operation_log.push(logged);
    }

    pub fn crdt_has_seen(&self, id: CrdtOpId) -> bool {
        self.crdt_seen.contains(&id)
    }

    pub fn crdt_actor_clock(&self, actor: Uuid) -> u64 {
        self.crdt_actor_clocks.get(&actor).copied().unwrap_or(0)
    }

    pub fn next_crdt_operation_id(&self, actor: Uuid) -> CrdtOpId {
        CrdtOpId {
            actor,
            counter: self.crdt_actor_clock(actor) + 1,
        }
    }

    pub fn crdt_dependency_frontier(&self) -> Vec<CrdtOpId> {
        self.crdt_actor_clocks
            .iter()
            .filter_map(|(actor, counter)| {
                (*counter > 0).then_some(CrdtOpId {
                    actor: *actor,
                    counter: *counter,
                })
            })
            .collect()
    }

    pub fn apply_crdt_operation(&mut self, operation: CrdtOperation) -> CrdtApplyResult {
        if !self.crdt_seen.insert(operation.id) {
            return CrdtApplyResult::Duplicate;
        }
        self.crdt_actor_clocks
            .entry(operation.id.actor)
            .and_modify(|counter| *counter = (*counter).max(operation.id.counter))
            .or_insert(operation.id.counter);
        self.apply_operation_without_log(&operation.operation);
        self.crdt_operation_log.push(operation);
        CrdtApplyResult::Applied
    }

    pub fn apply_operation_without_log(&mut self, operation: &Operation) {
        match operation {
            Operation::Batch { operations } => {
                for operation in operations {
                    self.apply_operation_without_log(operation);
                }
            }
            Operation::AddShape { shape } => {
                self.next_shape_id = self.next_shape_id.max(shape.id.0 + 1);
                self.shapes.insert(shape.id, shape.clone());
            }
            Operation::DeleteShape { id } => {
                self.shapes.remove(id);
            }
            Operation::ReplaceShape { id, shape } => {
                if self.shapes.contains_key(id) {
                    self.next_shape_id = self.next_shape_id.max(shape.id.0 + 1);
                    self.shapes.insert(*id, shape.clone());
                }
            }
            Operation::MoveShape { id, delta } => {
                if let Some(mut shape) = self.shapes.get_mut(id) {
                    shape.kind.translate(*delta);
                }
            }
            Operation::AddCell { cell } => {
                self.ensure_hierarchy();
                self.next_cell_id = self.next_cell_id.max(cell.id.0 + 1);
                for shape in cell.shapes.values() {
                    self.next_shape_id = self.next_shape_id.max(shape.id.0 + 1);
                }
                for instance in cell.instances.values() {
                    self.next_instance_id = self.next_instance_id.max(instance.id.0 + 1);
                }
                self.cells.insert(cell.id, cell.clone());
            }
            Operation::DeleteCell { id } => {
                if *id != self.top_cell {
                    self.cells.remove(id);
                }
            }
            Operation::RenameCell { id, name } => {
                if let Some(cell) = self.cells.get_mut(id) {
                    cell.name = name.clone();
                }
            }
            Operation::AddInstance { parent, instance } => {
                self.ensure_hierarchy();
                if self.cells.contains_key(&instance.cell) {
                    self.next_instance_id = self.next_instance_id.max(instance.id.0 + 1);
                    if let Some(parent) = self.cells.get_mut(parent) {
                        parent.instances.insert(instance.id, instance.clone());
                    }
                }
            }
            Operation::ReplaceInstance {
                parent,
                id,
                instance,
            } => {
                self.ensure_hierarchy();
                if self.cells.contains_key(&instance.cell)
                    && instance.id == *id
                    && let Some(parent) = self.cells.get_mut(parent)
                    && parent.instances.contains_key(id)
                {
                    parent.instances.insert(*id, instance.clone());
                }
            }
            Operation::DeleteInstance { parent, id } => {
                if let Some(parent) = self.cells.get_mut(parent) {
                    parent.instances.remove(id);
                }
            }
            Operation::RenameInstance { parent, id, name } => {
                if let Some(mut instance) = self.instance_mut(*parent, *id) {
                    instance.name = name.clone();
                }
            }
            Operation::MoveInstance { parent, id, delta } => {
                if let Some(mut instance) = self.instance_mut(*parent, *id) {
                    instance.transform = instance
                        .transform
                        .compose(Transform::from_translation(*delta));
                }
            }
            Operation::AddLayer { layer } => {
                self.next_layer_id = self.next_layer_id.max(layer.id.0 + 1);
                self.layers.insert(layer.id, layer.clone());
            }
            Operation::SetLayerVisibility { layer, visible } => {
                if let Some(layer) = self.layers.get_mut(layer) {
                    layer.visible = *visible;
                }
            }
            Operation::Cursor { .. } => {}
        }
    }

    pub fn layer_by_process(&self, process: ProcessLayer) -> Option<LayerId> {
        self.layers
            .iter()
            .find_map(|(id, layer)| (layer.process == process).then_some(*id))
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.get(&id)
    }

    pub fn layer_color(&self, id: LayerId) -> [f32; 4] {
        self.layer(id)
            .map(|layer| layer.color)
            .unwrap_or([0.8, 0.8, 0.8, 0.35])
    }

    pub fn shape_view_for_occurrence(
        &self,
        occurrence: &ShapeOccurrenceId,
    ) -> Option<FlattenedShapeView<'_>> {
        if occurrence.instance_path.is_empty() {
            if !occurrence.array_path.is_empty() {
                return None;
            }
            if let Some(shape) = self.shapes.view(&occurrence.shape) {
                return Some(FlattenedShapeView {
                    shape,
                    source_cell: self.top_cell,
                    transform: Transform::IDENTITY,
                    bounds: shape.bounds(),
                });
            }
            let top = self.cells.get(&self.top_cell)?;
            let shape = top.shapes.view(&occurrence.shape)?;
            return Some(FlattenedShapeView {
                shape,
                source_cell: top.id,
                transform: Transform::IDENTITY,
                bounds: shape.bounds(),
            });
        }

        let mut cell = self.cells.get(&self.top_cell)?;
        let mut transform = Transform::IDENTITY;
        let mut array_path_index = 0;
        for instance_id in &occurrence.instance_path {
            let instance_row = cell.instances.row_for_id(*instance_id)?;
            let child_cell_id = cell.instances.row_cell(instance_row);
            let instance_transform = cell.instances.row_transform(instance_row);
            let array = cell.instances.row_array(instance_row).normalized();
            let offset = if array.is_single() {
                Vector::ZERO
            } else {
                let array_index = occurrence.array_path.get(array_path_index)?;
                if array_index.column >= array.columns || array_index.row >= array.rows {
                    return None;
                }
                array_path_index += 1;
                array.element_offset(array_index.column, array_index.row)
            };
            transform =
                transform.compose(Transform::from_translation(offset).compose(instance_transform));
            cell = self.cells.get(&child_cell_id)?;
        }
        if array_path_index != occurrence.array_path.len() {
            return None;
        }

        let shape = cell.shapes.view(&occurrence.shape)?;
        Some(FlattenedShapeView {
            shape,
            source_cell: cell.id,
            transform,
            bounds: transform.apply_rect(shape.bounds()),
        })
    }

    pub fn visible_shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.shapes.live_rows().filter_map(|row| {
            self.layer_is_visible(self.shapes.row_layer(row))
                .then(|| self.shapes.materialize_row(row))
        })
    }

    pub fn has_hierarchy_instances(&self) -> bool {
        self.cells
            .values()
            .any(|cell| !cell.shapes.is_empty() || !cell.instances.is_empty())
    }

    pub fn visible_flattened_shapes(&self) -> Vec<FlattenedShape> {
        let mut flattened = Vec::with_capacity(self.flattened_shape_count_estimate());
        self.for_each_visible_flattened_shape_view(|id, view| {
            let instance_path = id.instance_path.clone();
            flattened.push(FlattenedShape {
                id,
                shape: view.shape.to_shape(),
                source_cell: view.source_cell,
                instance_path,
                transform: view.transform,
                bounds: view.bounds,
            });
        });
        flattened
    }

    pub fn for_each_visible_flattened_shape_view<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.visit_visible_flattened_shape_views(|id, shape| {
            visit(id, shape);
            true
        });
    }

    pub fn visit_visible_flattened_shape_views<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        let identity = Transform::IDENTITY;
        for row in self.shapes.live_rows() {
            if !self.layer_is_visible(self.shapes.row_layer(row)) {
                continue;
            }
            let shape = self.shapes.row_view(row);
            if !visit(
                ShapeOccurrenceId::top_level(shape.id),
                FlattenedShapeView {
                    shape,
                    source_cell: self.top_cell,
                    transform: identity,
                    bounds: shape.bounds(),
                },
            ) {
                return;
            }
        }

        let mut stack = Vec::new();
        let mut array_stack = Vec::new();
        if let Some(top) = self.cells.get(&self.top_cell) {
            for row in top.shapes.live_rows() {
                if !self.layer_is_visible(top.shapes.row_layer(row)) {
                    continue;
                }
                let shape = top.shapes.row_view(row);
                if !visit(
                    ShapeOccurrenceId::top_level(shape.id),
                    FlattenedShapeView {
                        shape,
                        source_cell: self.top_cell,
                        transform: identity,
                        bounds: shape.bounds(),
                    },
                ) {
                    return;
                }
            }
            let _ = self.visit_instance_shape_views(
                top,
                identity,
                &mut stack,
                &mut array_stack,
                &mut visit,
            );
        }
    }

    fn visit_instance_shape_views<'a, V>(
        &'a self,
        parent: &'a Cell,
        parent_transform: Transform,
        instance_path: &mut Vec<InstanceId>,
        array_path: &mut Vec<ArrayIndex>,
        visit: &mut V,
    ) -> bool
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        for instance_row in parent.instances.live_rows() {
            let instance_id = parent.instances.row_id(instance_row);
            if instance_path.contains(&instance_id) {
                continue;
            }
            let Some(cell) = self.cells.get(&parent.instances.row_cell(instance_row)) else {
                continue;
            };
            let instance_transform = parent.instances.row_transform(instance_row);
            let array = parent.instances.row_array(instance_row).normalized();
            let include_array_index = !array.is_single();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let offset = array.element_offset(column, row);
                    let transform = parent_transform
                        .compose(Transform::from_translation(offset).compose(instance_transform));
                    instance_path.push(instance_id);
                    if include_array_index {
                        array_path.push(ArrayIndex { column, row });
                    }
                    for shape_row in cell.shapes.live_rows() {
                        if !self.layer_is_visible(cell.shapes.row_layer(shape_row)) {
                            continue;
                        }
                        let shape = cell.shapes.row_view(shape_row);
                        if !visit(
                            ShapeOccurrenceId::from_instance_array_path(
                                shape.id,
                                instance_path,
                                array_path,
                            ),
                            FlattenedShapeView {
                                shape,
                                source_cell: cell.id,
                                transform,
                                bounds: transform.apply_rect(shape.bounds()),
                            },
                        ) {
                            if include_array_index {
                                array_path.pop();
                            }
                            instance_path.pop();
                            return false;
                        }
                    }
                    if !self.visit_instance_shape_views(
                        cell,
                        transform,
                        instance_path,
                        array_path,
                        visit,
                    ) {
                        if include_array_index {
                            array_path.pop();
                        }
                        instance_path.pop();
                        return false;
                    }
                    if include_array_index {
                        array_path.pop();
                    }
                    instance_path.pop();
                }
            }
        }
        true
    }

    fn layer_is_visible(&self, layer: LayerId) -> bool {
        self.layers.get(&layer).is_some_and(|layer| layer.visible)
    }
}

fn default_top_cell() -> CellId {
    DEFAULT_TOP_CELL_ID
}

fn default_next_cell_id() -> u64 {
    DEFAULT_TOP_CELL_ID.0 + 1
}

fn default_next_instance_id() -> u64 {
    1
}

fn default_dbu_per_micron() -> Coord {
    geometry_core::DBU_PER_MICRON
}

fn default_grid() -> Coord {
    DEFAULT_GRID
}

fn default_visible() -> bool {
    true
}

fn validate_non_negative(
    value: Coord,
    rule: &'static str,
    layer: &str,
) -> Result<(), TechnologyError> {
    if value < 0 {
        return Err(TechnologyError::Invalid(format!(
            "{rule} for layer {layer:?} must be non-negative"
        )));
    }
    Ok(())
}

fn normalize_layer_ref(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .filter(|character| *character != '_' && *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn default_technology() -> TechnologyFile {
    TechnologyFile::from_json_str(DEFAULT_TECHNOLOGY_JSON)
        .expect("built-in default technology JSON is valid")
}

pub fn builtin_technologies() -> Vec<TechnologyFile> {
    vec![
        default_technology(),
        TechnologyFile::from_json_str(HIGH_DENSITY_TECHNOLOGY_JSON)
            .expect("built-in high-density technology JSON is valid"),
    ]
}

pub fn default_layers() -> Vec<(String, ProcessLayer, [f32; 4])> {
    default_technology()
        .layers
        .into_iter()
        .filter_map(|layer| {
            let process = ProcessLayer::from_technology_name(&layer.process)?;
            Some((layer.name, process, layer.color))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedShape {
    pub id: ShapeOccurrenceId,
    pub bounds: Rect,
}

const LAYOUT_INDEX_TILE_SIZE: Coord = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LayoutIndexTileKey {
    x: Coord,
    y: Coord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayoutIndexRef {
    key: LayoutIndexTileKey,
    entry: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayoutIndexBucket {
    key: LayoutIndexTileKey,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutIndex {
    entries: Vec<IndexedShape>,
    refs: Vec<LayoutIndexRef>,
    buckets: Vec<LayoutIndexBucket>,
}

impl LayoutIndex {
    pub fn rebuild(document: &Document) -> Self {
        let mut entries = Vec::with_capacity(document.shapes.len());
        entries.extend(document.shapes.live_rows().filter_map(|row| {
            document
                .layers
                .get(&document.shapes.row_layer(row))
                .is_some_and(|layer| layer.visible)
                .then(|| IndexedShape {
                    id: ShapeOccurrenceId::top_level(document.shapes.row_id(row)),
                    bounds: document.shapes.row_bounds(row),
                })
        }));
        Self::bulk_load(entries)
    }

    pub fn rebuild_hierarchical(document: &Document) -> Self {
        let mut entries = Vec::with_capacity(document.flattened_shape_count_estimate());
        document.append_visible_index_entries(&mut entries);
        Self::bulk_load(entries)
    }

    pub fn bulk_load(entries: Vec<IndexedShape>) -> Self {
        let mut refs = Vec::with_capacity(entries.len());
        for (entry, shape) in entries.iter().enumerate() {
            let min_x = shape.bounds.min.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let max_x = shape.bounds.max.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let min_y = shape.bounds.min.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            let max_y = shape.bounds.max.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    refs.push(LayoutIndexRef {
                        key: LayoutIndexTileKey { x, y },
                        entry,
                    });
                }
            }
        }
        refs.sort_unstable_by_key(|reference| reference.key);

        let mut buckets = Vec::new();
        let mut start = 0;
        while start < refs.len() {
            let key = refs[start].key;
            let mut end = start + 1;
            while end < refs.len() && refs[end].key == key {
                end += 1;
            }
            buckets.push(LayoutIndexBucket { key, start, end });
            start = end;
        }

        Self {
            entries,
            refs,
            buckets,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn query_occurrences_into(&self, rect: Rect, out: &mut Vec<ShapeOccurrenceId>) {
        let start_len = out.len();
        let min_x = rect.min.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_x = rect.max.x.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let min_y = rect.min.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        let max_y = rect.max.y.div_euclid(LAYOUT_INDEX_TILE_SIZE);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let key = LayoutIndexTileKey { x, y };
                let Ok(bucket_index) = self.buckets.binary_search_by_key(&key, |bucket| bucket.key)
                else {
                    continue;
                };
                let bucket = self.buckets[bucket_index];
                for reference in &self.refs[bucket.start..bucket.end] {
                    let entry = &self.entries[reference.entry];
                    if entry.bounds.intersects(rect) {
                        out.push(entry.id.clone());
                    }
                }
            }
        }

        out[start_len..].sort_unstable();
        let mut write = start_len;
        for read in start_len..out.len() {
            if read == start_len || out[read] != out[write - 1] {
                if write != read {
                    out[write] = out[read].clone();
                }
                write += 1;
            }
        }
        out.truncate(write);
    }

    pub fn query_rect(&self, rect: Rect) -> Vec<ShapeId> {
        self.query_occurrences(rect)
            .into_iter()
            .map(|id| id.source_shape_id())
            .collect()
    }

    pub fn query_occurrences(&self, rect: Rect) -> Vec<ShapeOccurrenceId> {
        let mut out = Vec::new();
        self.query_occurrences_into(rect, &mut out);
        out
    }

    pub fn hit_test(&self, point: Point, tolerance: Coord) -> Option<ShapeId> {
        self.hit_test_occurrence(point, tolerance)
            .map(|id| id.source_shape_id())
    }

    pub fn hit_test_occurrence(&self, point: Point, tolerance: Coord) -> Option<ShapeOccurrenceId> {
        let query = Rect::new(point, point).expanded(tolerance);
        self.query_occurrences(query).into_iter().next()
    }
}

impl Document {
    fn append_visible_index_entries(&self, entries: &mut Vec<IndexedShape>) {
        for row in self.shapes.live_rows() {
            if !self.layer_is_visible(self.shapes.row_layer(row)) {
                continue;
            }
            entries.push(IndexedShape {
                id: ShapeOccurrenceId::top_level(self.shapes.row_id(row)),
                bounds: self.shapes.row_bounds(row),
            });
        }

        let identity = Transform::IDENTITY;
        let mut instance_path = Vec::new();
        let mut array_path = Vec::new();
        if let Some(top) = self.cells.get(&self.top_cell) {
            for row in top.shapes.live_rows() {
                if !self.layer_is_visible(top.shapes.row_layer(row)) {
                    continue;
                }
                entries.push(IndexedShape {
                    id: ShapeOccurrenceId::top_level(top.shapes.row_id(row)),
                    bounds: top.shapes.row_bounds(row),
                });
            }
            self.append_instance_index_entries(
                top,
                identity,
                &mut instance_path,
                &mut array_path,
                entries,
            );
        }
    }

    fn append_instance_index_entries(
        &self,
        parent: &Cell,
        parent_transform: Transform,
        instance_path: &mut Vec<InstanceId>,
        array_path: &mut Vec<ArrayIndex>,
        entries: &mut Vec<IndexedShape>,
    ) {
        for instance_row in parent.instances.live_rows() {
            let instance_id = parent.instances.row_id(instance_row);
            if instance_path.contains(&instance_id) {
                continue;
            }
            let Some(cell) = self.cells.get(&parent.instances.row_cell(instance_row)) else {
                continue;
            };
            let instance_transform = parent.instances.row_transform(instance_row);
            let array = parent.instances.row_array(instance_row).normalized();
            let include_array_index = !array.is_single();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let offset = array.element_offset(column, row);
                    let transform = parent_transform
                        .compose(Transform::from_translation(offset).compose(instance_transform));
                    instance_path.push(instance_id);
                    if include_array_index {
                        array_path.push(ArrayIndex { column, row });
                    }
                    for shape_row in cell.shapes.live_rows() {
                        if !self.layer_is_visible(cell.shapes.row_layer(shape_row)) {
                            continue;
                        }
                        entries.push(IndexedShape {
                            id: ShapeOccurrenceId::from_instance_array_path(
                                cell.shapes.row_id(shape_row),
                                instance_path,
                                array_path,
                            ),
                            bounds: transform.apply_rect(cell.shapes.row_bounds(shape_row)),
                        });
                    }
                    self.append_instance_index_entries(
                        cell,
                        transform,
                        instance_path,
                        array_path,
                        entries,
                    );
                    if include_array_index {
                        array_path.pop();
                    }
                    instance_path.pop();
                }
            }
        }
    }

    pub fn flattened_shape_count_estimate(&self) -> usize {
        let mut count = self.shapes.len();
        if let Some(top) = self.cells.get(&self.top_cell) {
            count += top.shapes.len();
            count += self.flattened_instance_shape_count_estimate(top, &mut Vec::new());
        }
        count
    }

    fn flattened_instance_shape_count_estimate(
        &self,
        parent: &Cell,
        instance_path: &mut Vec<InstanceId>,
    ) -> usize {
        let mut count = 0;
        for instance_row in parent.instances.live_rows() {
            let instance_id = parent.instances.row_id(instance_row);
            if instance_path.contains(&instance_id) {
                continue;
            }
            let Some(cell) = self.cells.get(&parent.instances.row_cell(instance_row)) else {
                continue;
            };
            let array = parent.instances.row_array(instance_row).normalized();
            let array_count = array.columns as usize * array.rows as usize;
            count += cell.shapes.len() * array_count;
            instance_path.push(instance_id);
            count +=
                self.flattened_instance_shape_count_estimate(cell, instance_path) * array_count;
            instance_path.pop();
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_allocate_past_remote_shape_ids() {
        let mut doc = Document::new("test");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let shape = Shape {
            id: ShapeId(99),
            layer,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
            name: None,
        };
        doc.apply_operation_without_log(&Operation::AddShape { shape });
        assert_eq!(doc.allocate_shape_id(), ShapeId(100));
    }

    #[test]
    fn new_documents_have_a_top_cell_for_hierarchy() {
        let doc = Document::new("hierarchy");
        assert_eq!(doc.top_cell, DEFAULT_TOP_CELL_ID);
        assert!(doc.cell(doc.top_cell).is_some());
        assert_eq!(doc.next_cell_id, DEFAULT_TOP_CELL_ID.0 + 1);
    }

    #[test]
    fn default_technology_builds_document_layers() {
        let technology = default_technology();
        let doc = Document::from_technology("tech", &technology).unwrap();

        assert_eq!(doc.grid, 10);
        assert_eq!(
            doc.layer_by_process(ProcessLayer::Metal1),
            technology.layer_id("metal1").ok()
        );
        let metal1 = doc
            .layer(doc.layer_by_process(ProcessLayer::Metal1).unwrap())
            .unwrap();
        assert_eq!(metal1.purpose, "first routing metal");
        assert_eq!(metal1.display_order, 40);
        assert_eq!(metal1.gds_layer, Some(4));
        assert_eq!(metal1.gds_datatype, 0);
        assert_eq!(
            technology.layer_for_gds_geometry(4, 0).unwrap().name,
            "metal1"
        );
    }

    #[test]
    fn invalid_technology_reports_duplicate_layer_names() {
        let mut technology = default_technology();
        technology.layers[1].name = technology.layers[0].name.clone();

        let err = technology.validate().unwrap_err();

        assert!(err.to_string().contains("duplicate layer name"));
    }

    #[test]
    fn invalid_technology_reports_duplicate_gds_mapping() {
        let mut technology = default_technology();
        technology.layers[1].gds_layer = technology.layers[0].gds_layer;
        technology.layers[1].gds_datatype = technology.layers[0].gds_datatype;

        let err = technology.validate().unwrap_err();

        assert!(err.to_string().contains("duplicate GDSII geometry mapping"));
    }

    #[test]
    fn flattened_shapes_include_translated_cell_instances() {
        let mut doc = Document::new("hierarchy");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = doc.create_cell("unit");
        let child_shape = doc
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
            )
            .unwrap();
        let first = doc
            .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
            .unwrap();
        let second = doc
            .insert_instance_in_top(child, Transform::translate(-500, 300))
            .unwrap();

        let flattened = doc.visible_flattened_shapes();
        let instance_shapes: Vec<_> = flattened
            .iter()
            .filter(|shape| shape.source_shape_id() == child_shape)
            .collect();

        assert_eq!(instance_shapes.len(), 2);
        assert!(instance_shapes.iter().any(|shape| {
            shape.instance_path == vec![first]
                && shape.bounds == Rect::from_min_size(Point::new(1_000, 2_000), 100, 60)
        }));
        assert!(instance_shapes.iter().any(|shape| {
            shape.instance_path == vec![second]
                && shape.bounds == Rect::from_min_size(Point::new(-500, 300), 100, 60)
        }));
    }

    #[test]
    fn hierarchical_index_queries_transformed_instance_bounds() {
        let mut doc = Document::new("hierarchy index");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = doc.create_cell("unit");
        let child_shape = doc
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
            )
            .unwrap();
        doc.insert_instance_in_top(child, Transform::translate(10_000, 0))
            .unwrap();

        let flat_index = LayoutIndex::rebuild(&doc);
        let hierarchy_index = LayoutIndex::rebuild_hierarchical(&doc);
        let query = Rect::from_min_size(Point::new(9_950, -50), 200, 200);

        assert!(!flat_index.query_rect(query).contains(&child_shape));
        assert!(hierarchy_index.query_rect(query).contains(&child_shape));
    }

    #[test]
    fn tiled_layout_index_deduplicates_shapes_spanning_tiles() {
        let mut doc = Document::new("wide shape");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-20_000, -100), 40_000, 200)),
        );
        let index = LayoutIndex::rebuild(&doc);

        let hits = index.query_rect(Rect::from_min_size(
            Point::new(-30_000, -1_000),
            60_000,
            2_000,
        ));

        assert_eq!(hits, vec![id]);
    }

    #[test]
    fn shape_store_serializes_as_legacy_shape_map() {
        let mut doc = Document::new("shape store serde");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
        );

        let encoded = serde_json::to_value(&doc).unwrap();
        let shapes = encoded
            .get("shapes")
            .and_then(|value| value.as_object())
            .unwrap();
        assert!(shapes.contains_key(&id.0.to_string()));

        let restored: Document = serde_json::from_value(encoded).unwrap();
        assert_eq!(
            restored.shapes.get(&id).unwrap().kind.bounds(),
            doc.shapes.get(&id).unwrap().kind.bounds()
        );
    }

    #[test]
    fn hierarchy_serializes_and_round_trips() {
        let mut doc = Document::new("round trip");
        let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
        let child = doc.create_cell("nand2");
        let shape_id = doc
            .insert_shape_in_cell(
                child,
                poly,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(25, 40), 120, 600)),
            )
            .unwrap();
        let instance_id = doc
            .insert_instance_in_top(child, Transform::translate(800, -300))
            .unwrap();

        let json = serde_json::to_string(&doc).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        let restored_cell = restored.cell(child).unwrap();

        assert!(restored_cell.shapes.contains_key(&shape_id));
        assert!(
            restored
                .cell(restored.top_cell)
                .unwrap()
                .instances
                .contains_key(&instance_id)
        );
        assert_eq!(restored.visible_flattened_shapes().len(), 1);
    }

    #[test]
    fn old_flat_documents_deserialize_with_default_hierarchy_fields() {
        let mut doc = Document::new("old");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
        );
        let mut json = serde_json::to_value(&doc).unwrap();
        let object = json.as_object_mut().unwrap();
        object.remove("next_cell_id");
        object.remove("next_instance_id");
        object.remove("top_cell");
        object.remove("cells");
        object.remove("marker_states");

        let restored: Document = serde_json::from_value(json).unwrap();
        assert_eq!(restored.top_cell, DEFAULT_TOP_CELL_ID);
        assert_eq!(restored.visible_shapes().count(), 1);
        assert_eq!(restored.visible_flattened_shapes().len(), 1);
        assert!(restored.marker_states.is_empty());
    }

    #[test]
    fn old_documents_deserialize_with_default_crdt_metadata() {
        let doc = Document::new("old crdt");
        let mut json = serde_json::to_value(&doc).unwrap();
        let object = json.as_object_mut().unwrap();
        object.remove("crdt_seen");
        object.remove("crdt_actor_clocks");
        object.remove("crdt_operation_log");

        let restored: Document = serde_json::from_value(json).unwrap();

        assert!(restored.crdt_seen.is_empty());
        assert!(restored.crdt_actor_clocks.is_empty());
        assert!(restored.crdt_operation_log.is_empty());
    }

    #[test]
    fn old_instance_documents_deserialize_with_identity_matrix_and_single_array() {
        let mut doc = Document::new("old instance");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell = doc.create_cell("unit");
        doc.insert_shape_in_cell(
            cell,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 20)),
        )
        .unwrap();
        let instance = doc
            .insert_instance_in_top(cell, Transform::translate(30, 40))
            .unwrap();
        let mut json = serde_json::to_value(&doc).unwrap();
        let instance_json = json
            .pointer_mut(&format!("/cells/1/instances/{}", instance.0))
            .unwrap()
            .as_object_mut()
            .unwrap();
        instance_json.remove("array");
        instance_json
            .get_mut("transform")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("matrix");

        let restored: Document = serde_json::from_value(json).unwrap();
        let restored_instance = restored.instance(restored.top_cell, instance).unwrap();

        assert_eq!(
            restored_instance.transform.matrix,
            Transform::IDENTITY.matrix
        );
        assert_eq!(restored_instance.transform.translation, Vector::new(30, 40));
        assert_eq!(restored_instance.array, InstanceArray::single());
        assert_eq!(restored.visible_flattened_shapes().len(), 1);
    }

    #[test]
    fn marker_states_round_trip_with_document() {
        let mut doc = Document::new("markers");
        doc.marker_states.insert(
            "min_width|1|0,0,10,10|200|80.000".to_string(),
            MarkerState {
                hidden: true,
                waived: true,
                note: Some("known demo marker".to_string()),
            },
        );

        let restored: Document =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        let state = restored
            .marker_states
            .get("min_width|1|0,0,10,10|200|80.000")
            .unwrap();

        assert!(state.hidden);
        assert!(state.waived);
        assert_eq!(state.note.as_deref(), Some("known demo marker"));
    }

    #[test]
    fn server_snapshot_json_round_trips_numeric_id_map_keys() {
        let doc = Document::new("snapshot json");
        let message = ServerMessage::Snapshot {
            document: doc.clone(),
            cursors: BTreeMap::new(),
            selections: BTreeMap::new(),
            loro_snapshot: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"layers\":{\"1\""));

        let restored: ServerMessage = serde_json::from_str(&json).unwrap();
        let ServerMessage::Snapshot {
            document,
            loro_snapshot,
            ..
        } = restored
        else {
            panic!("expected snapshot message");
        };
        assert_eq!(document.layers.len(), doc.layers.len());
        assert_eq!(document.cell(DEFAULT_TOP_CELL_ID).unwrap().name, "top");
        assert_eq!(loro_snapshot, vec![1, 2, 3]);
    }

    #[test]
    fn crdt_operations_are_idempotent_by_actor_counter() {
        let mut doc = Document::new("crdt idempotency");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let shape_id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        );
        let actor = Uuid::from_u128(7);
        let operation = CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::MoveShape {
                id: shape_id,
                delta: Vector::new(20, -10),
            },
        };

        assert_eq!(
            doc.apply_crdt_operation(operation.clone()),
            CrdtApplyResult::Applied
        );
        assert_eq!(
            doc.apply_crdt_operation(operation),
            CrdtApplyResult::Duplicate
        );

        let shape = doc.shapes.get(&shape_id).unwrap();
        assert_eq!(
            shape.kind.bounds(),
            Rect::from_min_size(Point::new(20, -10), 100, 50)
        );
        assert_eq!(doc.crdt_operation_log.len(), 1);
        assert_eq!(doc.crdt_actor_clock(actor), 1);
    }

    #[test]
    fn crdt_operation_ids_and_dependency_frontier_use_actor_clocks() {
        let mut doc = Document::new("crdt frontier");
        let actor_a = Uuid::from_u128(11);
        let actor_b = Uuid::from_u128(12);
        let layer = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

        let add_layer = CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 3,
            },
            deps: Vec::new(),
            operation: Operation::SetLayerVisibility {
                layer,
                visible: false,
            },
        };
        assert_eq!(
            doc.apply_crdt_operation(add_layer),
            CrdtApplyResult::Applied
        );

        let next = doc.next_crdt_operation_id(actor_a);
        let mut deps = doc.crdt_dependency_frontier();
        deps.sort();

        assert_eq!(
            next,
            CrdtOpId {
                actor: actor_a,
                counter: 4
            }
        );
        assert_eq!(
            deps,
            vec![CrdtOpId {
                actor: actor_a,
                counter: 3
            }]
        );
        assert_eq!(doc.next_crdt_operation_id(actor_b).counter, 1);
    }

    #[test]
    fn loro_log_replication_emits_each_operation_once() {
        let actor_a = Uuid::from_u128(21);
        let actor_b = Uuid::from_u128(22);
        let mut source = LoroCrdtLog::new(actor_a).unwrap();
        let mut target = LoroCrdtLog::new(actor_b).unwrap();
        let operation = CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::Cursor {
                user: actor_a,
                position: Point::new(10, 20),
            },
        };

        let update = source.append_operation(actor_a, operation.clone()).unwrap();

        assert!(source.import_update(&update).unwrap().is_empty());
        let imported = target.import_update(&update).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].id, operation.id);
        assert!(target.import_update(&update).unwrap().is_empty());
    }

    #[test]
    fn loro_snapshots_seed_log_without_reemitting_history() {
        let actor_a = Uuid::from_u128(23);
        let actor_b = Uuid::from_u128(24);
        let mut source = LoroCrdtLog::new(actor_a).unwrap();
        let first = CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::Cursor {
                user: actor_a,
                position: Point::new(1, 2),
            },
        };
        let second = CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 2,
            },
            deps: vec![first.id],
            operation: Operation::Cursor {
                user: actor_a,
                position: Point::new(3, 4),
            },
        };
        source.append_operation(actor_a, first).unwrap();
        let snapshot = source.export_snapshot().unwrap();
        let mut restored = LoroCrdtLog::from_snapshot(actor_b, &snapshot).unwrap();

        assert!(restored.drain_unemitted_operations().unwrap().is_empty());

        let update = source.append_operation(actor_a, second.clone()).unwrap();
        let imported = restored.import_update(&update).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].id, second.id);
    }

    #[test]
    fn loro_shape_store_uses_registers_and_tombstones() {
        let actor = Uuid::from_u128(25);
        let mut log = LoroCrdtLog::new(actor).unwrap();
        let shape = Shape {
            id: ShapeId(7),
            layer: LayerId(3),
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 40)),
            name: Some("m1 strap".to_string()),
        };

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 1 },
                deps: Vec::new(),
                operation: Operation::AddShape {
                    shape: shape.clone(),
                },
            },
        )
        .unwrap();

        let stored = log.shape_from_store(shape.id).unwrap().unwrap();
        assert_eq!(stored.layer, shape.layer);
        assert_eq!(stored.name.as_deref(), Some("m1 strap"));
        assert_eq!(stored.kind.bounds(), shape.kind.bounds());

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 2 },
                deps: Vec::new(),
                operation: Operation::MoveShape {
                    id: shape.id,
                    delta: Vector::new(50, -10),
                },
            },
        )
        .unwrap();
        let moved = log.shape_from_store(shape.id).unwrap().unwrap();
        assert_eq!(
            moved.kind.bounds(),
            Rect::from_min_size(Point::new(50, -10), 100, 40)
        );

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 3 },
                deps: Vec::new(),
                operation: Operation::DeleteShape { id: shape.id },
            },
        )
        .unwrap();
        assert!(log.shape_is_deleted(shape.id).unwrap());
        assert!(log.shape_from_store(shape.id).unwrap().is_none());
    }

    #[test]
    fn loro_hierarchy_store_tracks_cells_instances_and_child_shapes() {
        let actor = Uuid::from_u128(26);
        let mut log = LoroCrdtLog::new(actor).unwrap();
        let cell_id = CellId(5);
        let shape_id = ShapeId(9);
        let instance_id = InstanceId(11);
        let mut cell = Cell::new(cell_id, "unit");
        cell.shapes.insert(
            shape_id,
            Shape {
                id: shape_id,
                layer: LayerId(4),
                net: None,
                kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 30)),
                name: None,
            },
        );
        let instance = CellInstance {
            id: instance_id,
            name: Some("u0".to_string()),
            cell: cell_id,
            transform: Transform::translate(100, 200),
            array: InstanceArray::single(),
        };

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 1 },
                deps: Vec::new(),
                operation: Operation::AddCell { cell },
            },
        )
        .unwrap();
        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 2 },
                deps: Vec::new(),
                operation: Operation::AddInstance {
                    parent: DEFAULT_TOP_CELL_ID,
                    instance,
                },
            },
        )
        .unwrap();

        assert_eq!(log.shape_parent_cell(shape_id).unwrap(), Some(cell_id));
        let stored_instance = log
            .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored_instance.name.as_deref(), Some("u0"));
        assert_eq!(stored_instance.transform, Transform::translate(100, 200));

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 3 },
                deps: Vec::new(),
                operation: Operation::MoveInstance {
                    parent: DEFAULT_TOP_CELL_ID,
                    id: instance_id,
                    delta: Vector::new(-50, 25),
                },
            },
        )
        .unwrap();
        let moved_instance = log
            .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
            .unwrap()
            .unwrap();
        assert_eq!(moved_instance.transform, Transform::translate(50, 225));

        let mut arrayed_instance = moved_instance.clone();
        arrayed_instance.array = InstanceArray {
            columns: 4,
            rows: 2,
            column_pitch: Vector::new(120, 0),
            row_pitch: Vector::new(0, 90),
        };
        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 4 },
                deps: Vec::new(),
                operation: Operation::ReplaceInstance {
                    parent: DEFAULT_TOP_CELL_ID,
                    id: instance_id,
                    instance: arrayed_instance,
                },
            },
        )
        .unwrap();
        let stored_array = log
            .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
            .unwrap()
            .unwrap()
            .array;
        assert_eq!(stored_array.columns, 4);
        assert_eq!(stored_array.rows, 2);

        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 5 },
                deps: Vec::new(),
                operation: Operation::DeleteInstance {
                    parent: DEFAULT_TOP_CELL_ID,
                    id: instance_id,
                },
            },
        )
        .unwrap();
        assert!(
            log.instance_is_deleted(DEFAULT_TOP_CELL_ID, instance_id)
                .unwrap()
        );
        assert!(
            log.instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn loro_object_store_replicates_through_update_bytes() {
        let actor_a = Uuid::from_u128(27);
        let actor_b = Uuid::from_u128(28);
        let mut source = LoroCrdtLog::new(actor_a).unwrap();
        let mut target = LoroCrdtLog::new(actor_b).unwrap();
        let shape = Shape {
            id: ShapeId(31),
            layer: LayerId(2),
            net: Some(NetId(4)),
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(5, 6), 70, 80)),
            name: None,
        };
        let update = source
            .append_operation(
                actor_a,
                CrdtOperation {
                    id: CrdtOpId {
                        actor: actor_a,
                        counter: 1,
                    },
                    deps: Vec::new(),
                    operation: Operation::AddShape {
                        shape: shape.clone(),
                    },
                },
            )
            .unwrap();

        let operations = target.import_update(&update).unwrap();
        let stored = target.shape_from_store(shape.id).unwrap().unwrap();

        assert_eq!(operations.len(), 1);
        assert_eq!(stored.layer, shape.layer);
        assert_eq!(stored.net, shape.net);
        assert_eq!(stored.kind.bounds(), shape.kind.bounds());
    }

    #[test]
    fn loro_object_store_can_materialize_document_objects() {
        let actor = Uuid::from_u128(29);
        let mut log = LoroCrdtLog::new(actor).unwrap();
        let cell_id = CellId(6);
        let shape_id = ShapeId(41);
        let instance_id = InstanceId(3);
        let mut cell = Cell::new(cell_id, "slice");
        cell.shapes.insert(
            shape_id,
            Shape {
                id: shape_id,
                layer: LayerId(4),
                net: None,
                kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 10)),
                name: None,
            },
        );
        let instance = CellInstance {
            id: instance_id,
            name: Some("x0".to_string()),
            cell: cell_id,
            transform: Transform::translate(500, -100),
            array: InstanceArray::single(),
        };
        log.append_operation(
            actor,
            CrdtOperation {
                id: CrdtOpId { actor, counter: 1 },
                deps: Vec::new(),
                operation: Operation::Batch {
                    operations: vec![
                        Operation::AddCell { cell },
                        Operation::AddInstance {
                            parent: DEFAULT_TOP_CELL_ID,
                            instance,
                        },
                    ],
                },
            },
        )
        .unwrap();

        let mut doc = Document::new("materialized");
        log.materialize_objects_into_document(&mut doc).unwrap();

        assert_eq!(doc.cell(cell_id).unwrap().name, "slice");
        assert!(doc.cell(cell_id).unwrap().shapes.contains_key(&shape_id));
        assert!(
            doc.cell(DEFAULT_TOP_CELL_ID)
                .unwrap()
                .instances
                .contains_key(&instance_id)
        );
        assert_eq!(doc.visible_flattened_shapes().len(), 1);
        assert_eq!(
            doc.visible_flattened_shapes()[0].bounds,
            Rect::from_min_size(Point::new(500, -100), 20, 10)
        );
    }

    #[test]
    fn operations_add_and_move_cell_instances() {
        let mut doc = Document::new("instance ops");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell_id = doc.allocate_cell_id();
        let shape_id = doc.allocate_shape_id();
        let instance_id = doc.allocate_instance_id();
        let mut cell = Cell::new(cell_id, "unit");
        cell.shapes.insert(
            shape_id,
            Shape {
                id: shape_id,
                layer: metal1,
                net: None,
                kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
                name: None,
            },
        );
        let instance = CellInstance {
            id: instance_id,
            name: None,
            cell: cell_id,
            transform: Transform::translate(1_000, 2_000),
            array: InstanceArray::single(),
        };

        doc.apply_operation_without_log(&Operation::Batch {
            operations: vec![
                Operation::AddCell { cell },
                Operation::AddInstance {
                    parent: doc.top_cell,
                    instance,
                },
            ],
        });

        let flattened = doc.visible_flattened_shapes();
        assert_eq!(flattened.len(), 1);
        assert_eq!(
            flattened[0].bounds,
            Rect::from_min_size(Point::new(1_000, 2_000), 100, 80)
        );

        doc.apply_operation_without_log(&Operation::MoveInstance {
            parent: doc.top_cell,
            id: instance_id,
            delta: Vector::new(300, -500),
        });

        let flattened = doc.visible_flattened_shapes();
        assert_eq!(
            flattened[0].bounds,
            Rect::from_min_size(Point::new(1_300, 1_500), 100, 80)
        );
    }

    #[test]
    fn instance_arrays_flatten_to_distinct_occurrences() {
        let mut doc = Document::new("array instance ops");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell_id = doc.create_cell("unit");
        let shape_id = doc
            .insert_shape_in_cell(
                cell_id,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
            )
            .unwrap();
        let instance_id = doc
            .insert_instance_in_top(cell_id, Transform::translate(10, 20))
            .unwrap();
        let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
        instance.array = InstanceArray {
            columns: 3,
            rows: 2,
            column_pitch: Vector::new(200, 0),
            row_pitch: Vector::new(0, 300),
        };
        doc.apply_operation_without_log(&Operation::ReplaceInstance {
            parent: doc.top_cell,
            id: instance_id,
            instance,
        });

        let flattened = doc.visible_flattened_shapes();
        let occurrences = flattened
            .iter()
            .map(|shape| shape.id.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(flattened.len(), 6);
        assert_eq!(occurrences.len(), 6);
        assert!(
            occurrences.contains(&ShapeOccurrenceId::from_instance_array_path(
                shape_id,
                &[instance_id],
                &[ArrayIndex { column: 2, row: 1 }],
            ))
        );
        assert!(
            flattened
                .iter()
                .any(|shape| shape.bounds == Rect::from_min_size(Point::new(410, 320), 100, 50))
        );
    }

    #[test]
    fn instance_transform_rotation_and_mirror_affect_flattened_bounds() {
        let mut doc = Document::new("oriented instance ops");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell_id = doc.create_cell("unit");
        doc.insert_shape_in_cell(
            cell_id,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 40)),
        )
        .unwrap();
        let instance_id = doc
            .insert_instance_in_top(cell_id, Transform::translate(1_000, 2_000))
            .unwrap();
        let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
        instance.transform = instance.transform.compose(Transform::rotate_cw90());
        doc.apply_operation_without_log(&Operation::ReplaceInstance {
            parent: doc.top_cell,
            id: instance_id,
            instance,
        });
        assert_eq!(
            doc.visible_flattened_shapes()[0].bounds,
            Rect::from_min_size(Point::new(1_000, 1_900), 40, 100)
        );

        let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
        instance.transform = Transform::translate(1_000, 2_000).compose(Transform::mirror_x());
        doc.apply_operation_without_log(&Operation::ReplaceInstance {
            parent: doc.top_cell,
            id: instance_id,
            instance,
        });
        assert_eq!(
            doc.visible_flattened_shapes()[0].bounds,
            Rect::from_min_size(Point::new(900, 2_000), 100, 40)
        );
    }

    #[test]
    fn operations_delete_instances_and_cells() {
        let mut doc = Document::new("delete instance ops");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell_id = doc.create_cell("unit");
        doc.insert_shape_in_cell(
            cell_id,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        )
        .unwrap();
        let instance_id = doc
            .insert_instance_in_top(cell_id, Transform::translate(100, 200))
            .unwrap();
        assert_eq!(doc.visible_flattened_shapes().len(), 1);

        doc.apply_operation_without_log(&Operation::Batch {
            operations: vec![
                Operation::DeleteInstance {
                    parent: doc.top_cell,
                    id: instance_id,
                },
                Operation::DeleteCell { id: cell_id },
            ],
        });

        assert!(doc.cell(cell_id).is_none());
        assert_eq!(doc.visible_flattened_shapes().len(), 0);
    }

    #[test]
    fn replace_shape_does_not_resurrect_deleted_shape() {
        let mut doc = Document::new("delete wins over replace");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        );
        let replacement = Shape {
            id,
            layer: metal1,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 10), 100, 80)),
            name: None,
        };

        doc.apply_operation_without_log(&Operation::DeleteShape { id });
        doc.apply_operation_without_log(&Operation::ReplaceShape {
            id,
            shape: replacement,
        });

        assert!(!doc.shapes.contains_key(&id));
    }

    #[test]
    fn crdt_delete_wins_over_concurrent_move() {
        let mut doc = Document::new("move delete conflict");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        );
        let actor_a = Uuid::from_u128(101);
        let actor_b = Uuid::from_u128(102);

        assert_eq!(
            doc.apply_crdt_operation(CrdtOperation {
                id: CrdtOpId {
                    actor: actor_a,
                    counter: 1,
                },
                deps: Vec::new(),
                operation: Operation::DeleteShape { id },
            }),
            CrdtApplyResult::Applied
        );
        assert_eq!(
            doc.apply_crdt_operation(CrdtOperation {
                id: CrdtOpId {
                    actor: actor_b,
                    counter: 1,
                },
                deps: Vec::new(),
                operation: Operation::MoveShape {
                    id,
                    delta: Vector::new(50, 25),
                },
            }),
            CrdtApplyResult::Applied
        );

        assert!(!doc.shapes.contains_key(&id));
    }

    #[test]
    fn crdt_delete_wins_over_concurrent_vertex_replace() {
        let mut doc = Document::new("vertex delete conflict");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        );
        let actor_a = Uuid::from_u128(103);
        let actor_b = Uuid::from_u128(104);
        let replacement = Shape {
            id,
            layer: metal1,
            net: None,
            kind: ShapeKind::Polygon(Polygon {
                points: vec![
                    Point::new(0, 0),
                    Point::new(140, 0),
                    Point::new(100, 80),
                    Point::new(0, 80),
                ],
            }),
            name: None,
        };

        doc.apply_crdt_operation(CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::DeleteShape { id },
        });
        doc.apply_crdt_operation(CrdtOperation {
            id: CrdtOpId {
                actor: actor_b,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::ReplaceShape {
                id,
                shape: replacement,
            },
        });

        assert!(!doc.shapes.contains_key(&id));
    }

    #[test]
    fn crdt_duplicate_insert_is_idempotent() {
        let mut doc = Document::new("duplicate insert conflict");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let actor = Uuid::from_u128(105);
        let shape = Shape {
            id: ShapeId(90),
            layer: metal1,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            name: None,
        };
        let operation = CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::AddShape {
                shape: shape.clone(),
            },
        };

        assert_eq!(
            doc.apply_crdt_operation(operation.clone()),
            CrdtApplyResult::Applied
        );
        assert_eq!(
            doc.apply_crdt_operation(operation),
            CrdtApplyResult::Duplicate
        );

        assert_eq!(doc.shapes.len(), 1);
        assert_eq!(
            doc.shapes.get(&shape.id).unwrap().kind.bounds(),
            shape.kind.bounds()
        );
    }

    #[test]
    fn crdt_ordered_concurrent_layer_changes_converge() {
        let mut doc = Document::new("layer change conflict");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let actor_a = Uuid::from_u128(106);
        let actor_b = Uuid::from_u128(107);
        let hide = CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::SetLayerVisibility {
                layer,
                visible: false,
            },
        };
        let show = CrdtOperation {
            id: CrdtOpId {
                actor: actor_b,
                counter: 1,
            },
            deps: vec![hide.id],
            operation: Operation::SetLayerVisibility {
                layer,
                visible: true,
            },
        };

        doc.apply_crdt_operation(hide.clone());
        doc.apply_crdt_operation(show);
        doc.apply_crdt_operation(hide);

        assert!(doc.layer(layer).unwrap().visible);
    }

    #[test]
    fn operations_rename_cells_and_instances() {
        let mut doc = Document::new("rename ops");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let cell_id = doc.create_cell("unit");
        doc.insert_shape_in_cell(
            cell_id,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        )
        .unwrap();
        let instance_id = doc
            .insert_instance_in_top(cell_id, Transform::translate(100, 200))
            .unwrap();

        doc.apply_operation_without_log(&Operation::RenameCell {
            id: cell_id,
            name: "renamed unit".to_string(),
        });
        doc.apply_operation_without_log(&Operation::RenameInstance {
            parent: doc.top_cell,
            id: instance_id,
            name: Some("u0".to_string()),
        });

        assert_eq!(doc.cell(cell_id).unwrap().name, "renamed unit");
        assert_eq!(
            doc.instance(doc.top_cell, instance_id)
                .unwrap()
                .name
                .as_deref(),
            Some("u0")
        );
    }
}
