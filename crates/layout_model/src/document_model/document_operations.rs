#![allow(unused_imports)]
use super::*;

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
            connectivity_issue_states: BTreeMap::new(),
            reference_images: Vec::new(),
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
        let explicit_layer_ids = technology
            .layers
            .iter()
            .filter_map(|layer| layer.id)
            .collect::<BTreeSet<_>>();
        for technology_layer in &technology.layers {
            let process = ProcessLayer::from_technology_name(&technology_layer.process)
                .ok_or_else(|| {
                    TechnologyError::Invalid(format!(
                        "layer {:?} has unknown process {:?}",
                        technology_layer.name, technology_layer.process
                    ))
                })?;
            let id = if let Some(id) = technology_layer.id {
                id
            } else {
                let fallback = self.next_available_implicit_layer_id(&explicit_layer_ids)?;
                warn!(
                    layer_name = %technology_layer.name,
                    fallback_layer_id = fallback.0,
                    "technology layer missing id; assigning next layer id"
                );
                fallback
            };
            let next_after_id = id.0.checked_add(1).ok_or_else(|| {
                TechnologyError::Invalid(
                    "unable to assign next layer id; layer id space is exhausted".to_string(),
                )
            })?;
            self.next_layer_id = self.next_layer_id.max(next_after_id);
            self.layers.insert(
                id,
                Layer {
                    id,
                    name: technology_layer.name.clone(),
                    process,
                    purpose: technology_layer.purpose.clone(),
                    color: technology_layer.color,
                    fill_style: LayerFillStyle::Solid,
                    line_style: LayerLineStyle::Solid,
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

    pub(crate) fn next_available_implicit_layer_id(
        &mut self,
        explicit_layer_ids: &BTreeSet<LayerId>,
    ) -> Result<LayerId, TechnologyError> {
        loop {
            let candidate = LayerId(self.next_layer_id);
            if !explicit_layer_ids.contains(&candidate) && !self.layers.contains_key(&candidate) {
                return Ok(candidate);
            }
            self.next_layer_id = self.next_layer_id.checked_add(1).ok_or_else(|| {
                TechnologyError::Invalid(
                    "unable to assign implicit layer id; layer id space is exhausted".to_string(),
                )
            })?;
        }
    }

    pub fn demo() -> Self {
        let mut doc = Self::new("Glassworks demo inverter");
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
        let mut doc = Self::new("Glassworks hierarchy demo");
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
        let mut doc = Self::new(format!("Glassworks realistic stress {count}"));
        let fallback_layer = doc.layers.keys().next().copied().unwrap_or(LayerId(1));
        let diffusion = doc
            .layer_by_process(ProcessLayer::Diffusion)
            .unwrap_or(fallback_layer);
        let poly = doc
            .layer_by_process(ProcessLayer::Poly)
            .unwrap_or(fallback_layer);
        let contact = doc
            .layer_by_process(ProcessLayer::Contact)
            .unwrap_or(fallback_layer);
        let metal1 = doc
            .layer_by_process(ProcessLayer::Metal1)
            .unwrap_or(fallback_layer);
        let via1 = doc
            .layer_by_process(ProcessLayer::Via1)
            .or_else(|| doc.layer_by_process(ProcessLayer::Contact))
            .unwrap_or(fallback_layer);
        let metal2 = doc
            .layer_by_process(ProcessLayer::Metal2)
            .unwrap_or(fallback_layer);
        let oxide = doc
            .layer_by_process(ProcessLayer::Oxide)
            .unwrap_or(fallback_layer);
        let cell_count = count.div_ceil(64).max(1) as Coord;
        let columns = ((cell_count as f64).sqrt() * 1.65).ceil().max(1.0) as Coord;
        let rows = ((cell_count + columns - 1) / columns).max(1);
        let site_pitch_x = 420;
        let row_pitch_y = 960;
        let first_shape_id = doc.next_shape_id;
        doc.shapes =
            ShapeStore::from_dense_id_range(ShapeId(first_shape_id), count, |index, id| {
                let cell = (index / 64) as Coord;
                let column = cell % columns;
                let row = cell / columns;
                let row_stagger = if row % 2 == 0 { 0 } else { site_pitch_x / 2 };
                let base_x = column * site_pitch_x + row_stagger - columns * site_pitch_x / 2;
                let base_y = row * row_pitch_y - rows * row_pitch_y / 2;
                let motif = index % 64;
                let lane = ((index / 64) % 4) as Coord;
                let hash = (index as u64)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .rotate_left(17);
                let jitter_x = ((hash & 0x1f) as Coord - 16) * 2;
                let jitter_y = (((hash >> 5) & 0x0f) as Coord - 8) * 2;
                let (layer, origin, width, height, net) = match motif {
                    0..=13 => (
                        diffusion,
                        Point::new(base_x - 160 + jitter_x, base_y - 145 + jitter_y),
                        280 + ((motif % 4) as Coord) * 34,
                        150 + (lane % 2) * 34,
                        None,
                    ),
                    14..=23 => (
                        poly,
                        Point::new(
                            base_x - 28 + ((motif as Coord - 18) * 18),
                            base_y - 310 + jitter_y,
                        ),
                        48 + (lane % 2) * 12,
                        620,
                        None,
                    ),
                    24..=35 => (
                        metal1,
                        Point::new(base_x - 190 + jitter_x, base_y - 34 + lane * 72),
                        330 + ((motif % 3) as Coord) * 120,
                        66,
                        Some(NetId((index as u32 % 4093) + 1)),
                    ),
                    36..=43 => (
                        contact,
                        Point::new(
                            base_x - 132 + ((motif as Coord - 36) % 4) * 86,
                            base_y - 94 + ((motif as Coord - 36) / 4) * 176,
                        ),
                        82,
                        82,
                        Some(NetId((index as u32 % 4093) + 1)),
                    ),
                    44..=51 => (
                        metal2,
                        Point::new(
                            base_x - 44 + ((motif as Coord - 44) % 2) * 168,
                            base_y - 360,
                        ),
                        88,
                        720 + lane * 84,
                        Some(NetId((index as u32 % 4093) + 1)),
                    ),
                    52..=55 => (
                        via1,
                        Point::new(base_x - 36 + ((motif as Coord - 52) % 2) * 130, base_y - 36),
                        72,
                        72,
                        Some(NetId((index as u32 % 4093) + 1)),
                    ),
                    56..=61 => (
                        metal1,
                        Point::new(base_x - 210, base_y - 420 + ((motif as Coord - 56) * 132)),
                        520,
                        54,
                        Some(NetId((index as u32 % 4093) + 1)),
                    ),
                    _ => (
                        oxide,
                        Point::new(base_x - 185 + jitter_x, base_y + 265 + jitter_y),
                        130 + (lane * 18),
                        84,
                        None,
                    ),
                };
                Shape {
                    id,
                    layer,
                    net,
                    kind: ShapeKind::Rectangle(Rect::from_min_size(origin, width, height)),
                    name: None,
                    properties: BTreeMap::new(),
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
                fill_style: LayerFillStyle::Solid,
                line_style: LayerLineStyle::Solid,
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
            properties: BTreeMap::new(),
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
            properties: BTreeMap::new(),
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
        self.instance_parent_for_path_from_cell(self.top_cell, path)
    }

    pub fn instance_parent_for_path_from_cell(
        &self,
        root_cell: CellId,
        path: &[InstanceId],
    ) -> Option<(CellId, InstanceId)> {
        let mut parent = root_cell;
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
            properties: BTreeMap::new(),
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
                if self.shapes.remove(id).is_none() {
                    warn!(
                        shape_id = id.0,
                        "delete shape operation skipped missing shape"
                    );
                }
            }
            Operation::AddShapeToCell { cell, shape } => {
                self.ensure_hierarchy();
                self.next_shape_id = self.next_shape_id.max(shape.id.0 + 1);
                if let Some(cell) = self.cells.get_mut(cell) {
                    cell.shapes.insert(shape.id, shape.clone());
                } else {
                    warn!(
                        cell_id = cell.0,
                        shape_id = shape.id.0,
                        "add shape to cell operation skipped missing cell"
                    );
                }
            }
            Operation::DeleteShapeFromCell { cell, id } => {
                if let Some(cell) = self.cells.get_mut(cell) {
                    if cell.shapes.remove(id).is_none() {
                        warn!(
                            cell_id = cell.id.0,
                            shape_id = id.0,
                            "delete shape from cell operation skipped missing shape"
                        );
                    }
                } else {
                    warn!(
                        cell_id = cell.0,
                        shape_id = id.0,
                        "delete shape from cell operation skipped missing cell"
                    );
                }
            }
            Operation::ReplaceShape { id, shape } => {
                if self.shapes.contains_key(id) {
                    self.next_shape_id = self.next_shape_id.max(shape.id.0 + 1);
                    self.shapes.insert(*id, shape.clone());
                } else {
                    warn!(
                        shape_id = id.0,
                        replacement_shape_id = shape.id.0,
                        "replace shape operation skipped missing shape"
                    );
                }
            }
            Operation::MoveShape { id, delta } => {
                if let Some(mut shape) = self.shapes.get_mut(id) {
                    shape.kind.translate(*delta);
                } else {
                    warn!(
                        shape_id = id.0,
                        delta_x = delta.dx,
                        delta_y = delta.dy,
                        "move shape operation skipped missing shape"
                    );
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
                    if self.cells.remove(id).is_none() {
                        warn!(cell_id = id.0, "delete cell operation skipped missing cell");
                    }
                } else {
                    warn!(cell_id = id.0, "delete cell operation skipped top cell");
                }
            }
            Operation::RenameCell { id, name } => {
                if let Some(cell) = self.cells.get_mut(id) {
                    cell.name = name.clone();
                } else {
                    warn!(cell_id = id.0, "rename cell operation skipped missing cell");
                }
            }
            Operation::SetCellProperties { id, properties } => {
                self.ensure_hierarchy();
                if let Some(cell) = self.cells.get_mut(id) {
                    cell.properties = properties.clone();
                } else {
                    warn!(
                        cell_id = id.0,
                        "set cell properties operation skipped missing cell"
                    );
                }
            }
            Operation::AddInstance { parent, instance } => {
                self.ensure_hierarchy();
                if self.cells.contains_key(&instance.cell) {
                    self.next_instance_id = self.next_instance_id.max(instance.id.0 + 1);
                    if let Some(parent) = self.cells.get_mut(parent) {
                        parent.instances.insert(instance.id, instance.clone());
                    } else {
                        warn!(
                            parent_cell_id = parent.0,
                            instance_id = instance.id.0,
                            "add instance operation skipped missing parent cell"
                        );
                    }
                } else {
                    warn!(
                        target_cell_id = instance.cell.0,
                        instance_id = instance.id.0,
                        "add instance operation skipped missing target cell"
                    );
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
                } else {
                    warn!(
                        parent_cell_id = parent.0,
                        instance_id = id.0,
                        target_cell_id = instance.cell.0,
                        replacement_instance_id = instance.id.0,
                        "replace instance operation skipped invalid reference"
                    );
                }
            }
            Operation::DeleteInstance { parent, id } => {
                if let Some(parent) = self.cells.get_mut(parent) {
                    if parent.instances.remove(id).is_none() {
                        warn!(
                            parent_cell_id = parent.id.0,
                            instance_id = id.0,
                            "delete instance operation skipped missing instance"
                        );
                    }
                } else {
                    warn!(
                        parent_cell_id = parent.0,
                        instance_id = id.0,
                        "delete instance operation skipped missing parent cell"
                    );
                }
            }
            Operation::RenameInstance { parent, id, name } => {
                if let Some(mut instance) = self.instance_mut(*parent, *id) {
                    instance.name = name.clone();
                } else {
                    warn!(
                        parent_cell_id = parent.0,
                        instance_id = id.0,
                        "rename instance operation skipped missing instance"
                    );
                }
            }
            Operation::MoveInstance { parent, id, delta } => {
                if let Some(mut instance) = self.instance_mut(*parent, *id) {
                    instance.transform = instance
                        .transform
                        .compose(Transform::from_translation(*delta));
                } else {
                    warn!(
                        parent_cell_id = parent.0,
                        instance_id = id.0,
                        delta_x = delta.dx,
                        delta_y = delta.dy,
                        "move instance operation skipped missing instance"
                    );
                }
            }
            Operation::AddLayer { layer } => {
                self.next_layer_id = self.next_layer_id.max(layer.id.0 + 1);
                self.layers.insert(layer.id, layer.clone());
            }
            Operation::DeleteLayer { id } => {
                if self.layers.remove(id).is_some() {
                    let shape_ids = self
                        .shapes
                        .values()
                        .filter_map(|shape| (shape.layer == *id).then_some(shape.id))
                        .collect::<Vec<_>>();
                    for shape_id in shape_ids {
                        self.shapes.remove(&shape_id);
                    }
                } else {
                    warn!(
                        layer_id = id.0,
                        "delete layer operation skipped missing layer"
                    );
                }
            }
            Operation::RenameLayer { id, name } => {
                if let Some(layer) = self.layers.get_mut(id) {
                    layer.name = name.clone();
                } else {
                    warn!(
                        layer_id = id.0,
                        "rename layer operation skipped missing layer"
                    );
                }
            }
            Operation::SetLayerVisibility { layer, visible } => {
                if let Some(layer) = self.layers.get_mut(layer) {
                    layer.visible = *visible;
                } else {
                    warn!(
                        layer_id = layer.0,
                        visible, "set layer visibility operation skipped missing layer"
                    );
                }
            }
            Operation::SetLayerDisplayStyle {
                layer,
                fill_style,
                line_style,
            } => {
                if let Some(layer) = self.layers.get_mut(layer) {
                    layer.fill_style = *fill_style;
                    layer.line_style = *line_style;
                } else {
                    warn!(
                        layer_id = layer.0,
                        fill_style = ?fill_style,
                        line_style = ?line_style,
                        "set layer display style operation skipped missing layer"
                    );
                }
            }
            Operation::SetMarkerState { key, state } => {
                apply_marker_state_operation(&mut self.marker_states, key, state.as_ref());
            }
            Operation::SetConnectivityIssueState { key, state } => {
                apply_marker_state_operation(
                    &mut self.connectivity_issue_states,
                    key,
                    state.as_ref(),
                );
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
        self.layer(id).map(|layer| layer.color).unwrap_or_else(|| {
            warn!(
                layer_id = id.0,
                "layer missing color; using fallback display color"
            );
            [0.8, 0.8, 0.8, 0.35]
        })
    }

    pub fn layer_display_color(&self, id: LayerId) -> [f32; 4] {
        let Some(layer) = self.layer(id) else {
            warn!(
                layer_id = id.0,
                "layer missing display color; using fallback display color"
            );
            return [0.8, 0.8, 0.8, 0.35];
        };
        let mut color = layer.color;
        match layer.fill_style {
            LayerFillStyle::Solid => {}
            LayerFillStyle::Outline => {
                color[3] = (color[3] * 0.18).min(0.18);
            }
            LayerFillStyle::Hatched | LayerFillStyle::CrossHatched => {
                color[3] = (color[3] * 0.52).min(0.52);
            }
            LayerFillStyle::Stippled
            | LayerFillStyle::DenseStippled
            | LayerFillStyle::SparseStippled => {
                color[3] = (color[3] * 0.4).min(0.4);
            }
        }
        color
    }

    pub fn shape_view_for_occurrence(
        &self,
        occurrence: &ShapeOccurrenceId,
    ) -> Option<FlattenedShapeView<'_>> {
        self.shape_view_for_occurrence_from_cell(self.top_cell, occurrence)
    }

    pub fn shape_view_for_occurrence_from_cell(
        &self,
        root_cell: CellId,
        occurrence: &ShapeOccurrenceId,
    ) -> Option<FlattenedShapeView<'_>> {
        if occurrence.instance_path.is_empty() {
            if !occurrence.array_path.is_empty() {
                return None;
            }
            if root_cell == self.top_cell
                && let Some(shape) = self.shapes.view(&occurrence.shape)
            {
                return Some(FlattenedShapeView {
                    shape,
                    source_cell: self.top_cell,
                    transform: Transform::IDENTITY,
                    bounds: shape.bounds(),
                });
            }
            let root = self.cells.get(&root_cell)?;
            let shape = root.shapes.view(&occurrence.shape)?;
            return Some(FlattenedShapeView {
                shape,
                source_cell: root.id,
                transform: Transform::IDENTITY,
                bounds: shape.bounds(),
            });
        }

        let mut cell = self.cells.get(&root_cell)?;
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
        self.shapes
            .live_rows()
            .filter(|&row| self.layer_is_visible(self.shapes.row_layer(row)))
            .map(|row| self.shapes.materialize_row(row))
    }

    pub fn has_hierarchy_instances(&self) -> bool {
        self.cells
            .values()
            .any(|cell| !cell.shapes.is_empty() || !cell.instances.is_empty())
    }

    pub fn flattened_shapes(&self) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(self.top_cell, false, 0, None)
    }

    pub fn visible_flattened_shapes(&self) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(self.top_cell, true, 0, None)
    }

    pub fn visible_flattened_shapes_with_max_depth(
        &self,
        max_depth: Option<usize>,
    ) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(self.top_cell, true, 0, max_depth)
    }

    pub fn visible_flattened_shapes_with_depth_range(
        &self,
        min_depth: usize,
        max_depth: Option<usize>,
    ) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(self.top_cell, true, min_depth, max_depth)
    }

    pub fn visible_flattened_shapes_for_cell(
        &self,
        root_cell: CellId,
        max_depth: Option<usize>,
    ) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(root_cell, true, 0, max_depth)
    }

    pub fn visible_flattened_shapes_for_cell_with_depth_range(
        &self,
        root_cell: CellId,
        min_depth: usize,
        max_depth: Option<usize>,
    ) -> Vec<FlattenedShape> {
        self.collect_flattened_shapes(root_cell, true, min_depth, max_depth)
    }

    pub(crate) fn collect_flattened_shapes(
        &self,
        root_cell: CellId,
        visible_only: bool,
        min_depth: usize,
        max_depth: Option<usize>,
    ) -> Vec<FlattenedShape> {
        let mut flattened = Vec::with_capacity(self.flattened_shape_count_estimate());
        self.for_each_flattened_shape_view_matching(
            root_cell,
            visible_only,
            min_depth,
            max_depth,
            |id, view| {
                let instance_path = id.instance_path.clone();
                flattened.push(FlattenedShape {
                    id,
                    shape: view.shape.to_shape(),
                    source_cell: view.source_cell,
                    instance_path,
                    transform: view.transform,
                    bounds: view.bounds,
                });
            },
        );
        flattened
    }

    pub fn for_each_flattened_shape_view<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_flattened_shape_view_matching(self.top_cell, false, 0, None, |id, shape| {
            visit(id, shape);
        });
    }

    pub fn for_each_flattened_shape_view_for_cell<'a, V>(
        &'a self,
        root_cell: CellId,
        max_depth: Option<usize>,
        mut visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_flattened_shape_view_matching(root_cell, false, 0, max_depth, |id, shape| {
            visit(id, shape);
        });
    }

    pub fn for_each_visible_flattened_shape_view<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_flattened_shape_view_matching(self.top_cell, true, 0, None, |id, shape| {
            visit(id, shape);
        });
    }

    pub fn for_each_visible_flattened_shape_view_with_max_depth<'a, V>(
        &'a self,
        max_depth: Option<usize>,
        mut visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_flattened_shape_view_matching(
            self.top_cell,
            true,
            0,
            max_depth,
            |id, shape| {
                visit(id, shape);
            },
        );
    }

    pub fn for_each_visible_flattened_shape_view_for_cell<'a, V>(
        &'a self,
        root_cell: CellId,
        max_depth: Option<usize>,
        visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_visible_flattened_shape_view_for_cell_with_depth_range(
            root_cell, 0, max_depth, visit,
        );
    }

    pub fn for_each_visible_flattened_shape_view_for_cell_with_depth_range<'a, V>(
        &'a self,
        root_cell: CellId,
        min_depth: usize,
        max_depth: Option<usize>,
        mut visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.for_each_flattened_shape_view_matching(
            root_cell,
            true,
            min_depth,
            max_depth,
            |id, shape| {
                visit(id, shape);
            },
        );
    }

    pub(crate) fn for_each_flattened_shape_view_matching<'a, V>(
        &'a self,
        root_cell: CellId,
        visible_only: bool,
        min_depth: usize,
        max_depth: Option<usize>,
        mut visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>),
    {
        self.visit_flattened_shape_views_matching(
            root_cell,
            visible_only,
            min_depth,
            max_depth,
            |id, shape| {
                visit(id, shape);
                true
            },
        );
    }

    pub fn visit_flattened_shape_views<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        self.visit_flattened_shape_views_matching(self.top_cell, false, 0, None, |id, shape| {
            visit(id, shape)
        });
    }

    pub fn visit_visible_flattened_shape_views<'a, V>(&'a self, mut visit: V)
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        self.visit_flattened_shape_views_matching(self.top_cell, true, 0, None, |id, shape| {
            visit(id, shape)
        });
    }

    pub(crate) fn visit_flattened_shape_views_matching<'a, V>(
        &'a self,
        root_cell: CellId,
        visible_only: bool,
        min_depth: usize,
        max_depth: Option<usize>,
        mut visit: V,
    ) where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        let identity = Transform::IDENTITY;
        if root_cell == self.top_cell {
            for row in self.shapes.live_rows() {
                if visible_only && !self.layer_is_visible(self.shapes.row_layer(row)) {
                    continue;
                }
                if min_depth > 0 {
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
        }

        let mut stack = Vec::new();
        let mut array_stack = Vec::new();
        if let Some(root) = self.cells.get(&root_cell) {
            for row in root.shapes.live_rows() {
                if visible_only && !self.layer_is_visible(root.shapes.row_layer(row)) {
                    continue;
                }
                if min_depth > 0 {
                    continue;
                }
                let shape = root.shapes.row_view(row);
                if !visit(
                    ShapeOccurrenceId::top_level(shape.id),
                    FlattenedShapeView {
                        shape,
                        source_cell: root.id,
                        transform: identity,
                        bounds: shape.bounds(),
                    },
                ) {
                    return;
                }
            }
            let _ = self.visit_instance_shape_views(
                root,
                identity,
                visible_only,
                min_depth,
                max_depth,
                1,
                &mut stack,
                &mut array_stack,
                &mut visit,
            );
        }
    }

    pub(crate) fn visit_instance_shape_views<'a, V>(
        &'a self,
        parent: &'a Cell,
        parent_transform: Transform,
        visible_only: bool,
        min_depth: usize,
        max_depth: Option<usize>,
        depth: usize,
        instance_path: &mut Vec<InstanceId>,
        array_path: &mut Vec<ArrayIndex>,
        visit: &mut V,
    ) -> bool
    where
        V: FnMut(ShapeOccurrenceId, FlattenedShapeView<'a>) -> bool,
    {
        if max_depth.is_some_and(|limit| depth > limit) {
            return true;
        }

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
                        if visible_only && !self.layer_is_visible(cell.shapes.row_layer(shape_row))
                        {
                            continue;
                        }
                        if depth < min_depth {
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
                        visible_only,
                        min_depth,
                        max_depth,
                        depth + 1,
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

    pub(crate) fn layer_is_visible(&self, layer: LayerId) -> bool {
        self.layers.get(&layer).is_some_and(|layer| layer.visible)
    }
}

pub(crate) fn default_top_cell() -> CellId {
    DEFAULT_TOP_CELL_ID
}

pub(crate) fn default_next_cell_id() -> u64 {
    DEFAULT_TOP_CELL_ID.0 + 1
}

pub(crate) fn default_next_instance_id() -> u64 {
    1
}

pub(crate) fn default_dbu_per_micron() -> Coord {
    geometry_core::DBU_PER_MICRON
}

pub(crate) fn default_grid() -> Coord {
    DEFAULT_GRID
}

pub(crate) fn default_visible() -> bool {
    true
}

pub(crate) fn validate_non_negative(
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

pub(crate) fn validate_color_components(
    layer: &str,
    color: [f32; 4],
) -> Result<(), TechnologyError> {
    if color
        .iter()
        .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
    {
        return Err(TechnologyError::Invalid(format!(
            "layer {layer:?} color components must be finite values from 0.0 to 1.0"
        )));
    }
    Ok(())
}

pub(crate) fn validate_stack_ranges(
    ranges: &mut Vec<(&str, f32, f32)>,
) -> Result<(), TechnologyError> {
    ranges.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.2.total_cmp(&b.2)));
    let mut previous: Option<(&str, f32)> = None;
    for &(name, base, top) in ranges.iter() {
        if let Some((previous_name, previous_top)) = previous
            && base < previous_top
        {
            return Err(TechnologyError::Invalid(format!(
                "3D stack range for layer {name:?} overlaps layer {previous_name:?}"
            )));
        }
        previous = Some((name, top));
    }
    Ok(())
}

pub(crate) fn normalize_layer_ref(reference: &str) -> String {
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

pub(crate) const LAYOUT_INDEX_TILE_SIZE: Coord = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LayoutIndexTileKey {
    pub(crate) x: Coord,
    pub(crate) y: Coord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutIndexRef {
    pub(crate) key: LayoutIndexTileKey,
    pub(crate) entry: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutIndexBucket {
    pub(crate) key: LayoutIndexTileKey,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutIndex {
    pub(crate) entries: Vec<IndexedShape>,
    pub(crate) refs: Vec<LayoutIndexRef>,
    pub(crate) buckets: Vec<LayoutIndexBucket>,
}
