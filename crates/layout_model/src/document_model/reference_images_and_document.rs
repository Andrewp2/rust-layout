#![allow(unused_imports)]
use super::*;

impl ReferenceImageOverlay {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        uri: impl Into<String>,
        bounds: Rect,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            uri: uri.into(),
            bounds,
            ..Self::default()
        }
    }

    pub fn display_name(&self) -> &str {
        let name = self.name.trim();
        if !name.is_empty() {
            return name;
        }
        let id = self.id.trim();
        if !id.is_empty() {
            return id;
        }
        self.uri.trim()
    }

    pub fn image_key(&self) -> String {
        let uri = self.uri.trim();
        if !uri.is_empty() {
            return format!("layout.reference_image.{uri}");
        }
        format!("layout.reference_image.{}", self.id.trim())
    }

    pub fn aligned_bounds_from_landmarks(
        &self,
    ) -> Result<Option<Rect>, ReferenceImageAlignmentError> {
        if self.landmarks.len() < 2 {
            return Ok(None);
        }
        let size = self
            .pixel_size
            .ok_or(ReferenceImageAlignmentError::MissingPixelSize)?;
        if size.width <= 0 || size.height <= 0 {
            return Err(ReferenceImageAlignmentError::InvalidPixelSize);
        }

        let mut scale_x = None;
        let mut scale_y = None;
        for (index, a) in self.landmarks.iter().enumerate() {
            for b in self.landmarks.iter().skip(index + 1) {
                let image_dx = b.image.x - a.image.x;
                let layout_dx = b.layout.x - a.layout.x;
                if image_dx != 0 && layout_dx != 0 {
                    let candidate = layout_dx as f64 / image_dx as f64;
                    if candidate.is_finite() && candidate > 0.0 {
                        scale_x = Some(candidate);
                    }
                }

                let image_dy = b.image.y - a.image.y;
                let layout_dy = b.layout.y - a.layout.y;
                if image_dy != 0 && layout_dy != 0 {
                    let candidate = -(layout_dy as f64 / image_dy as f64);
                    if candidate.is_finite() && candidate > 0.0 {
                        scale_y = Some(candidate);
                    }
                }
            }
        }

        let scale_x = scale_x
            .or(scale_y)
            .ok_or(ReferenceImageAlignmentError::InsufficientLandmarks)?;
        let scale_y = scale_y.unwrap_or(scale_x);
        let anchor = self
            .landmarks
            .first()
            .ok_or(ReferenceImageAlignmentError::InsufficientLandmarks)?;
        let left = anchor.layout.x as f64 - anchor.image.x as f64 * scale_x;
        let top = anchor.layout.y as f64 + anchor.image.y as f64 * scale_y;
        let right = left + size.width as f64 * scale_x;
        let bottom = top - size.height as f64 * scale_y;
        Ok(Some(Rect::new(
            Point::new(round_to_coord(left)?, round_to_coord(bottom)?),
            Point::new(round_to_coord(right)?, round_to_coord(top)?),
        )))
    }

    pub fn apply_landmark_alignment(&mut self) -> Result<bool, ReferenceImageAlignmentError> {
        let Some(bounds) = self.aligned_bounds_from_landmarks()? else {
            return Ok(false);
        };
        let changed = self.bounds != bounds;
        self.bounds = bounds;
        Ok(changed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceImageAlignmentError {
    MissingPixelSize,
    InvalidPixelSize,
    InsufficientLandmarks,
    CoordinateOverflow,
}

impl fmt::Display for ReferenceImageAlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPixelSize => write!(f, "missing pixel size"),
            Self::InvalidPixelSize => write!(f, "invalid pixel size"),
            Self::InsufficientLandmarks => write!(f, "need two distinct matching landmarks"),
            Self::CoordinateOverflow => write!(f, "aligned bounds exceed layout coordinate range"),
        }
    }
}

impl std::error::Error for ReferenceImageAlignmentError {}

pub(crate) fn round_to_coord(value: f64) -> Result<Coord, ReferenceImageAlignmentError> {
    if !value.is_finite() || value < Coord::MIN as f64 || value > Coord::MAX as f64 {
        return Err(ReferenceImageAlignmentError::CoordinateOverflow);
    }
    Ok(value.round() as Coord)
}

#[derive(Clone, Debug, Default)]
pub struct InstanceStore {
    pub(crate) alive: Vec<bool>,
    pub(crate) ids: Vec<InstanceId>,
    pub(crate) names: Vec<Option<String>>,
    pub(crate) cells: Vec<CellId>,
    pub(crate) transforms: Vec<Transform>,
    pub(crate) arrays: Vec<InstanceArray>,
    pub(crate) properties: Vec<BTreeMap<String, String>>,
    pub(crate) id_to_row: Vec<Option<usize>>,
    pub(crate) overflow_id_to_row: BTreeMap<InstanceId, usize>,
    pub(crate) len: usize,
}

pub struct InstanceMut<'a> {
    pub(crate) store: &'a mut InstanceStore,
    pub(crate) row: usize,
    pub(crate) old_id: InstanceId,
    pub(crate) instance: CellInstance,
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
            properties: Vec::with_capacity(capacity),
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
        self.properties.clear();
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

    pub(crate) fn live_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| row)
    }

    pub(crate) fn row_id(&self, row: usize) -> InstanceId {
        self.ids[row]
    }

    pub(crate) fn row_cell(&self, row: usize) -> CellId {
        self.cells[row]
    }

    pub(crate) fn row_transform(&self, row: usize) -> Transform {
        self.transforms[row]
    }

    pub(crate) fn row_array(&self, row: usize) -> InstanceArray {
        self.arrays[row]
    }

    pub(crate) fn row_for_id(&self, id: InstanceId) -> Option<usize> {
        dense_instance_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    pub(crate) fn set_row_for_id(&mut self, id: InstanceId, row: usize) {
        if let Some(index) = dense_instance_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            warn!(
                instance_id = id.0,
                "instance id exceeded usize range; using overflow index"
            );
            self.overflow_id_to_row.insert(id, row);
        }
    }

    pub(crate) fn clear_row_for_id(&mut self, id: InstanceId) {
        if let Some(index) = dense_instance_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }

    pub(crate) fn push_row(&mut self, instance: CellInstance) {
        self.alive.push(true);
        self.ids.push(instance.id);
        self.names.push(instance.name);
        self.cells.push(instance.cell);
        self.transforms.push(instance.transform);
        self.arrays.push(instance.array);
        self.properties.push(instance.properties);
    }

    pub(crate) fn materialize_row(&self, row: usize) -> CellInstance {
        CellInstance {
            id: self.ids[row],
            name: self.names[row].clone(),
            cell: self.cells[row],
            transform: self.transforms[row],
            array: self.arrays[row],
            properties: self.properties[row].clone(),
        }
    }

    pub(crate) fn write_row(&mut self, row: usize, instance: CellInstance) {
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
        self.properties[row] = instance.properties;
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
        struct InstanceStoreVisitor;

        impl<'de> de::Visitor<'de> for InstanceStoreVisitor {
            type Value = InstanceStore;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an instance map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut store = InstanceStore::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((id, instance)) = map.next_entry::<InstanceId, CellInstance>()? {
                    store.insert(id, instance);
                }
                Ok(store)
            }
        }

        deserializer.deserialize_map(InstanceStoreVisitor)
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

pub(crate) fn dense_instance_index(id: InstanceId) -> Option<usize> {
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
        if self.columns == 0 || self.rows == 0 {
            warn!(
                columns = self.columns,
                rows = self.rows,
                "instance array dimensions were below supported range; using minimum dimension 1"
            );
        }
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

pub(crate) fn default_array_count() -> u32 {
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

#[allow(clippy::large_enum_variant)]
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
    AddShapeToCell {
        cell: CellId,
        shape: Shape,
    },
    DeleteShapeFromCell {
        cell: CellId,
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
    SetCellProperties {
        id: CellId,
        properties: BTreeMap<String, String>,
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
    DeleteLayer {
        id: LayerId,
    },
    RenameLayer {
        id: LayerId,
        name: String,
    },
    SetLayerVisibility {
        layer: LayerId,
        visible: bool,
    },
    SetLayerDisplayStyle {
        layer: LayerId,
        fill_style: LayerFillStyle,
        line_style: LayerLineStyle,
    },
    SetMarkerState {
        key: String,
        state: Option<MarkerState>,
    },
    SetConnectivityIssueState {
        key: String,
        state: Option<MarkerState>,
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
    pub visited: bool,
    #[serde(default)]
    pub important: bool,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub signoff: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

pub(crate) fn apply_marker_state_operation(
    states: &mut BTreeMap<String, MarkerState>,
    key: &str,
    state: Option<&MarkerState>,
) {
    if key.trim().is_empty() {
        warn!("marker state operation skipped empty key");
        return;
    }
    match state {
        Some(state) if *state != MarkerState::default() => {
            states.insert(key.to_string(), state.clone());
        }
        _ => {
            states.remove(key);
        }
    }
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

#[allow(clippy::large_enum_variant)]
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
    pub connectivity_issue_states: BTreeMap<String, MarkerState>,
    #[serde(default)]
    pub reference_images: Vec<ReferenceImageOverlay>,
    #[serde(default)]
    pub crdt_seen: BTreeSet<CrdtOpId>,
    #[serde(default)]
    pub crdt_actor_clocks: BTreeMap<Uuid, u64>,
    #[serde(default)]
    pub crdt_operation_log: Vec<CrdtOperation>,
    pub operation_log: Vec<LoggedOperation>,
}
