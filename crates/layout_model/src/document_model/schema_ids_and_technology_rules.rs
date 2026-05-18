#![allow(unused_imports)]
use super::*;

pub const CURRENT_SCHEMA_VERSION: u32 = 5;
pub const DEFAULT_TOP_CELL_ID: CellId = CellId(1);
pub const DEFAULT_TECHNOLOGY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/technology/glassworks_demo.json"
));
pub const HIGH_DENSITY_TECHNOLOGY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/technology/glassworks_high_density.json"
));

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

    pub fn hierarchy_depth(&self) -> usize {
        self.instance_path.len()
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
    pub(crate) doc: LoroDoc,
    pub(crate) emitted: BTreeSet<CrdtOpId>,
}

impl fmt::Debug for LoroCrdtLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoroCrdtLog")
            .field("emitted", &self.emitted)
            .finish_non_exhaustive()
    }
}

impl LoroCrdtLog {
    pub(crate) const OPERATION_LOG: &'static str = "glassworks_ops";
    pub(crate) const SHAPES: &'static str = "glassworks_shapes";
    pub(crate) const CELLS: &'static str = "glassworks_cells";
    pub(crate) const INSTANCES: &'static str = "glassworks_instances";

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

    pub(crate) fn mark_all_emitted(&mut self) -> Result<(), LoroCrdtError> {
        for operation in self.operations()? {
            self.emitted.insert(operation.id);
        }
        Ok(())
    }

    pub(crate) fn operations(&self) -> Result<Vec<CrdtOperation>, LoroCrdtError> {
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

    pub(crate) fn mirror_operation_to_objects(
        &self,
        operation: &Operation,
    ) -> Result<(), LoroCrdtError> {
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
            Operation::AddShapeToCell { cell, shape } => {
                self.upsert_shape(Some(*cell), shape)?;
            }
            Operation::DeleteShapeFromCell { id, .. } => {
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
            Operation::SetCellProperties { id, properties } => {
                let cell = self.object_map(Self::CELLS, &cell_key(*id))?;
                cell.insert("id", id.0 as i64)?;
                set_optional_string(
                    &cell,
                    "properties_json",
                    (!properties.is_empty())
                        .then(|| serde_json::to_string(properties))
                        .transpose()
                        .map_err(LoroCrdtError::Encode)?
                        .as_deref(),
                )?;
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
            | Operation::DeleteLayer { .. }
            | Operation::RenameLayer { .. }
            | Operation::SetLayerVisibility { .. }
            | Operation::SetLayerDisplayStyle { .. }
            | Operation::SetMarkerState { .. }
            | Operation::SetConnectivityIssueState { .. }
            | Operation::Cursor { .. } => {}
        }
        Ok(())
    }

    pub(crate) fn object_map(
        &self,
        collection: &'static str,
        key: &str,
    ) -> Result<LoroMap, LoroCrdtError> {
        self.doc
            .get_map(collection)
            .get_or_create_container(key, LoroMap::new())
            .map_err(LoroCrdtError::from)
    }

    pub(crate) fn existing_object_map(
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

    pub(crate) fn upsert_shape(
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
        set_optional_string(
            &map,
            "properties_json",
            (!shape.properties.is_empty())
                .then(|| serde_json::to_string(&shape.properties))
                .transpose()
                .map_err(LoroCrdtError::Encode)?
                .as_deref(),
        )?;
        map.insert(
            "kind_json",
            serde_json::to_string(&shape.kind).map_err(LoroCrdtError::Encode)?,
        )?;
        map.insert("deleted", false)?;
        Ok(())
    }

    pub(crate) fn tombstone_shape(&self, id: ShapeId) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::SHAPES, &shape_key(id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    pub(crate) fn shape_record(&self, id: ShapeId) -> Result<Option<Shape>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::SHAPES, &shape_key(id))? else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or_else(|| {
            warn!(
                shape_id = id.0,
                "Loro shape record missing deleted flag; treating as active"
            );
            false
        }) {
            return Ok(None);
        }
        let Some(layer) = map_i64(&map, "layer") else {
            warn!(
                shape_id = id.0,
                "Loro shape record missing layer; shape record skipped"
            );
            return Ok(None);
        };
        let Some(kind_json) = map_string(&map, "kind_json") else {
            warn!(
                shape_id = id.0,
                "Loro shape record missing kind_json; shape record skipped"
            );
            return Ok(None);
        };
        Ok(Some(Shape {
            id,
            layer: LayerId(layer as u32),
            net: map_i64(&map, "net").map(|net| NetId(net as u32)),
            kind: serde_json::from_str(&kind_json).map_err(LoroCrdtError::Decode)?,
            name: map_optional_string(&map, "name"),
            properties: map_string(&map, "properties_json")
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(LoroCrdtError::Decode)?
                .unwrap_or_default(),
        }))
    }

    pub(crate) fn shape_parent_cell(&self, id: ShapeId) -> Result<Option<CellId>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::SHAPES, &shape_key(id))? else {
            return Ok(None);
        };
        Ok(map_i64(&map, "parent_cell").map(|cell| CellId(cell as u64)))
    }

    pub(crate) fn upsert_cell(&self, cell: &Cell) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::CELLS, &cell_key(cell.id))?;
        map.insert("id", cell.id.0 as i64)?;
        map.insert("name", cell.name.clone())?;
        set_optional_string(
            &map,
            "properties_json",
            (!cell.properties.is_empty())
                .then(|| serde_json::to_string(&cell.properties))
                .transpose()
                .map_err(LoroCrdtError::Encode)?
                .as_deref(),
        )?;
        map.insert("deleted", false)?;
        Ok(())
    }

    pub(crate) fn tombstone_cell(&self, id: CellId) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::CELLS, &cell_key(id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    pub fn cell_is_deleted(&self, id: CellId) -> Result<bool, LoroCrdtError> {
        Ok(
            match self.existing_object_map(Self::CELLS, &cell_key(id))? {
                Some(map) => map_bool(&map, "deleted").unwrap_or_else(|| {
                    warn!(
                        cell_id = id.0,
                        "Loro cell record missing deleted flag; treating as active"
                    );
                    false
                }),
                None => false,
            },
        )
    }

    pub(crate) fn upsert_instance(
        &self,
        parent: CellId,
        instance: &CellInstance,
    ) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::INSTANCES, &instance_key(parent, instance.id))?;
        map.insert("id", instance.id.0 as i64)?;
        map.insert("parent_cell", parent.0 as i64)?;
        map.insert("cell", instance.cell.0 as i64)?;
        set_optional_string(&map, "name", instance.name.as_deref())?;
        set_optional_string(
            &map,
            "properties_json",
            (!instance.properties.is_empty())
                .then(|| serde_json::to_string(&instance.properties))
                .transpose()
                .map_err(LoroCrdtError::Encode)?
                .as_deref(),
        )?;
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

    pub(crate) fn tombstone_instance(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<(), LoroCrdtError> {
        let map = self.object_map(Self::INSTANCES, &instance_key(parent, id))?;
        map.insert("id", id.0 as i64)?;
        map.insert("parent_cell", parent.0 as i64)?;
        map.insert("deleted", true)?;
        Ok(())
    }

    pub(crate) fn instance_record(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<Option<CellInstance>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::INSTANCES, &instance_key(parent, id))?
        else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or_else(|| {
            warn!(
                parent_cell = parent.0,
                instance_id = id.0,
                "Loro instance record missing deleted flag; treating as active"
            );
            false
        }) {
            return Ok(None);
        }
        let Some(cell) = map_i64(&map, "cell") else {
            warn!(
                parent_cell = parent.0,
                instance_id = id.0,
                "Loro instance record missing target cell; instance skipped"
            );
            return Ok(None);
        };
        let transform = match map_string(&map, "transform_json") {
            Some(json) => serde_json::from_str(&json).map_err(LoroCrdtError::Decode)?,
            None => {
                warn!(
                    parent_cell = parent.0,
                    instance_id = id.0,
                    "Loro instance record missing transform_json; using identity transform"
                );
                Transform::default()
            }
        };
        let array = match map_string(&map, "array_json") {
            Some(json) => serde_json::from_str(&json).map_err(LoroCrdtError::Decode)?,
            None => {
                warn!(
                    parent_cell = parent.0,
                    instance_id = id.0,
                    "Loro instance record missing array_json; using single instance array"
                );
                InstanceArray::default()
            }
        };
        Ok(Some(CellInstance {
            id,
            name: map_optional_string(&map, "name"),
            cell: CellId(cell as u64),
            transform,
            array,
            properties: map_string(&map, "properties_json")
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(LoroCrdtError::Decode)?
                .unwrap_or_default(),
        }))
    }

    pub fn shape_is_deleted(&self, id: ShapeId) -> Result<bool, LoroCrdtError> {
        Ok(
            match self.existing_object_map(Self::SHAPES, &shape_key(id))? {
                Some(map) => map_bool(&map, "deleted").unwrap_or_else(|| {
                    warn!(
                        shape_id = id.0,
                        "Loro shape record missing deleted flag; treating as active"
                    );
                    false
                }),
                None => false,
            },
        )
    }

    pub fn instance_is_deleted(
        &self,
        parent: CellId,
        id: InstanceId,
    ) -> Result<bool, LoroCrdtError> {
        Ok(
            match self.existing_object_map(Self::INSTANCES, &instance_key(parent, id))? {
                Some(map) => map_bool(&map, "deleted").unwrap_or_else(|| {
                    warn!(
                        parent_cell = parent.0,
                        instance_id = id.0,
                        "Loro instance record missing deleted flag; treating as active"
                    );
                    false
                }),
                None => false,
            },
        )
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
                    } else {
                        warn!(
                            shape_id = shape.id.0,
                            parent_cell = parent.0,
                            "Loro shape parent cell missing; shape skipped"
                        );
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
                    warn!(
                        parent_cell = parent.0,
                        "Loro instance parent cell missing; creating fallback cell"
                    );
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

    pub(crate) fn cell_record(&self, id: CellId) -> Result<Option<Cell>, LoroCrdtError> {
        let Some(map) = self.existing_object_map(Self::CELLS, &cell_key(id))? else {
            return Ok(None);
        };
        if map_bool(&map, "deleted").unwrap_or_else(|| {
            warn!(
                cell_id = id.0,
                "Loro cell record missing deleted flag; treating as active"
            );
            false
        }) {
            return Ok(None);
        }
        let name = map_string(&map, "name").unwrap_or_else(|| {
            warn!(
                cell_id = id.0,
                "Loro cell record missing name; using generated fallback cell name"
            );
            format!("cell {}", id.0)
        });
        let mut cell = Cell::new(id, name);
        if let Some(json) = map_string(&map, "properties_json") {
            cell.properties = serde_json::from_str(&json).map_err(LoroCrdtError::Decode)?;
        }
        Ok(Some(cell))
    }

    pub(crate) fn shape_ids(&self) -> Vec<ShapeId> {
        self.doc
            .get_map(Self::SHAPES)
            .keys()
            .filter_map(|key| match key.parse::<u64>() {
                Ok(id) => Some(ShapeId(id)),
                Err(err) => {
                    warn!(
                        key = key.as_ref(),
                        error = %err,
                        "Loro shape object key is not a valid ShapeId; object skipped"
                    );
                    None
                }
            })
            .collect()
    }

    pub(crate) fn cell_ids(&self) -> Vec<CellId> {
        self.doc
            .get_map(Self::CELLS)
            .keys()
            .filter_map(|key| match key.parse::<u64>() {
                Ok(id) => Some(CellId(id)),
                Err(err) => {
                    warn!(
                        key = key.as_ref(),
                        error = %err,
                        "Loro cell object key is not a valid CellId; object skipped"
                    );
                    None
                }
            })
            .collect()
    }

    pub(crate) fn instance_ids(&self) -> Vec<(CellId, InstanceId)> {
        self.doc
            .get_map(Self::INSTANCES)
            .keys()
            .filter_map(|key| {
                let Some((parent, id)) = key.split_once(':') else {
                    warn!(
                        key = key.as_ref(),
                        "Loro instance object key is missing ':'; object skipped"
                    );
                    return None;
                };
                let parent = match parent.parse::<u64>() {
                    Ok(parent) => CellId(parent),
                    Err(err) => {
                        warn!(
                            key = key.as_ref(),
                            error = %err,
                            "Loro instance parent key is not a valid CellId; object skipped"
                        );
                        return None;
                    }
                };
                let id = match id.parse::<u64>() {
                    Ok(id) => InstanceId(id),
                    Err(err) => {
                        warn!(
                            key = key.as_ref(),
                            error = %err,
                            "Loro instance key is not a valid InstanceId; object skipped"
                        );
                        return None;
                    }
                };
                Some((parent, id))
            })
            .collect()
    }
}

pub fn loro_peer_id(actor: Uuid) -> PeerID {
    let raw = actor.as_u128();
    ((raw >> 64) as u64) ^ raw as u64
}

pub(crate) fn shape_key(id: ShapeId) -> String {
    id.0.to_string()
}

pub(crate) fn cell_key(id: CellId) -> String {
    id.0.to_string()
}

pub(crate) fn instance_key(parent: CellId, id: InstanceId) -> String {
    format!("{}:{}", parent.0, id.0)
}

pub(crate) fn set_optional_string(
    map: &LoroMap,
    key: &str,
    value: Option<&str>,
) -> Result<(), LoroCrdtError> {
    match value {
        Some(value) => map.insert(key, value)?,
        None => map.insert(key, LoroValue::Null)?,
    }
    Ok(())
}

pub(crate) fn set_optional_cell_id(
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

pub(crate) fn set_optional_net_id(
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

pub(crate) fn map_value(map: &LoroMap, key: &str) -> Option<LoroValue> {
    match map.get(key)? {
        ValueOrContainer::Value(value) => Some(value),
        ValueOrContainer::Container(_) => None,
    }
}

pub(crate) fn map_bool(map: &LoroMap, key: &str) -> Option<bool> {
    match map_value(map, key)? {
        LoroValue::Bool(value) => Some(value),
        _ => None,
    }
}

pub(crate) fn map_i64(map: &LoroMap, key: &str) -> Option<i64> {
    match map_value(map, key)? {
        LoroValue::I64(value) => Some(value),
        _ => None,
    }
}

pub(crate) fn map_string(map: &LoroMap, key: &str) -> Option<String> {
    match map_value(map, key)? {
        LoroValue::String(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(crate) fn map_optional_string(map: &LoroMap, key: &str) -> Option<String> {
    match map_value(map, key)? {
        LoroValue::String(value) => Some(value.to_string()),
        LoroValue::Null => None,
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerFillStyle {
    #[default]
    Solid,
    Outline,
    Hatched,
    CrossHatched,
    Stippled,
    DenseStippled,
    SparseStippled,
}

impl LayerFillStyle {
    pub const ALL: [Self; 7] = [
        Self::Solid,
        Self::Outline,
        Self::Hatched,
        Self::CrossHatched,
        Self::Stippled,
        Self::DenseStippled,
        Self::SparseStippled,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Outline => "Outline",
            Self::Hatched => "Hatched",
            Self::CrossHatched => "Cross Hatch",
            Self::Stippled => "Stipple",
            Self::DenseStippled => "Dense Stipple",
            Self::SparseStippled => "Sparse Stipple",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|style| *style == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerLineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

impl LayerLineStyle {
    pub const ALL: [Self; 4] = [Self::Solid, Self::Dashed, Self::Dotted, Self::DashDot];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid",
            Self::Dashed => "Dashed",
            Self::Dotted => "Dotted",
            Self::DashDot => "Dash Dot",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|style| *style == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
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
    pub fill_style: LayerFillStyle,
    #[serde(default)]
    pub line_style: LayerLineStyle,
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

pub(crate) const MAX_DENSE_LAYER_ID_INDEX: usize = 1_000_000;

#[derive(Clone, Debug, Default)]
pub struct LayerStore {
    pub(crate) alive: Vec<bool>,
    pub(crate) ids: Vec<LayerId>,
    pub(crate) layers: Vec<Layer>,
    pub(crate) id_to_row: Vec<Option<usize>>,
    pub(crate) overflow_id_to_row: BTreeMap<LayerId, usize>,
    pub(crate) len: usize,
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

    pub(crate) fn row_for_id(&self, id: LayerId) -> Option<usize> {
        dense_layer_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    pub(crate) fn set_row_for_id(&mut self, id: LayerId, row: usize) {
        if let Some(index) = dense_layer_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            warn!(
                layer_id = id.0,
                max_dense_layer_id_index = MAX_DENSE_LAYER_ID_INDEX,
                "layer id exceeded dense store range; using overflow index"
            );
            self.overflow_id_to_row.insert(id, row);
        }
    }

    pub(crate) fn clear_row_for_id(&mut self, id: LayerId) {
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
        struct LayerStoreVisitor;

        impl<'de> de::Visitor<'de> for LayerStoreVisitor {
            type Value = LayerStore;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a layer map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut store = LayerStore::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((id, layer)) = map.next_entry::<LayerId, Layer>()? {
                    store.insert(id, layer);
                }
                Ok(store)
            }
        }

        deserializer.deserialize_map(LayerStoreVisitor)
    }
}

pub(crate) fn dense_layer_index(id: LayerId) -> Option<usize> {
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
    pub z_base: Option<f32>,
    #[serde(default)]
    pub z_thickness: Option<f32>,
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
    pub max_width: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub min_area: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub max_area: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub min_spacing: Vec<TechnologyLayerRule>,
    #[serde(default)]
    pub min_edge_spacing: Vec<TechnologyLayerRule>,
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
