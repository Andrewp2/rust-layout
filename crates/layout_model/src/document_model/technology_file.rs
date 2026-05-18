#![allow(unused_imports)]
use super::*;

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
        let mut stack_ranges = Vec::new();
        for layer in &self.layers {
            if layer.name.trim().is_empty() {
                return Err(TechnologyError::Invalid(
                    "layer names must not be empty".to_string(),
                ));
            }
            validate_color_components(&layer.name, layer.color)?;
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
            match (layer.z_base, layer.z_thickness) {
                (Some(base), Some(thickness)) => {
                    if !base.is_finite() || !thickness.is_finite() {
                        return Err(TechnologyError::Invalid(format!(
                            "layer {:?} has non-finite 3D stack metadata",
                            layer.name
                        )));
                    }
                    if thickness <= 0.0 {
                        return Err(TechnologyError::Invalid(format!(
                            "layer {:?} z_thickness must be positive",
                            layer.name
                        )));
                    }
                    stack_ranges.push((layer.name.as_str(), base, base + thickness));
                }
                (Some(_), None) | (None, Some(_)) => {
                    return Err(TechnologyError::Invalid(format!(
                        "layer {:?} must define both z_base and z_thickness",
                        layer.name
                    )));
                }
                (None, None) => {}
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
        validate_stack_ranges(&mut stack_ranges)?;

        let mut connectivity_keys = BTreeSet::new();
        for connection in &self.connectivity {
            self.layer_id(&connection.from)?;
            self.layer_id(&connection.through)?;
            self.layer_id(&connection.to)?;
            let from = normalize_layer_ref(&connection.from);
            let through = normalize_layer_ref(&connection.through);
            let to = normalize_layer_ref(&connection.to);
            let key = if from <= to {
                (from, through, to)
            } else {
                (to, through, from)
            };
            if !connectivity_keys.insert(key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate connectivity rule {:?} through {:?} to {:?}",
                    connection.from, connection.through, connection.to
                )));
            }
        }
        let mut min_width_layers = BTreeSet::new();
        for rule in &self.drc.min_width {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !min_width_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate min_width rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "min_width", &rule.layer)?;
        }
        let mut max_width_layers = BTreeSet::new();
        for rule in &self.drc.max_width {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !max_width_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate max_width rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "max_width", &rule.layer)?;
        }
        let mut min_area_layers = BTreeSet::new();
        for rule in &self.drc.min_area {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !min_area_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate min_area rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "min_area", &rule.layer)?;
        }
        let mut max_area_layers = BTreeSet::new();
        for rule in &self.drc.max_area {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !max_area_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate max_area rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "max_area", &rule.layer)?;
        }
        let mut min_spacing_layers = BTreeSet::new();
        for rule in &self.drc.min_spacing {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !min_spacing_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate min_spacing rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "min_spacing", &rule.layer)?;
        }
        let mut min_edge_spacing_layers = BTreeSet::new();
        for rule in &self.drc.min_edge_spacing {
            self.layer_id(&rule.layer)?;
            let layer_key = normalize_layer_ref(&rule.layer);
            if !min_edge_spacing_layers.insert(layer_key) {
                return Err(TechnologyError::Invalid(format!(
                    "duplicate min_edge_spacing rule for layer {:?}",
                    rule.layer
                )));
            }
            validate_non_negative(rule.value, "min_edge_spacing", &rule.layer)?;
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

    pub fn layer_stack_range_for_process(&self, process: ProcessLayer) -> Option<(f32, f32)> {
        self.layers.iter().find_map(|layer| {
            (ProcessLayer::from_technology_name(&layer.process) == Some(process))
                .then(|| layer.stack_range())
                .flatten()
        })
    }
}

impl TechnologyLayer {
    pub fn effective_gds_layer(&self) -> Option<u16> {
        self.gds_layer
            .or_else(|| self.id.and_then(|id| u16::try_from(id.0).ok()))
    }

    pub fn stack_range(&self) -> Option<(f32, f32)> {
        let base = self.z_base?;
        let thickness = self.z_thickness?;
        Some((base, base + thickness))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub layer: LayerId,
    pub net: Option<NetId>,
    pub kind: ShapeKind,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
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
        #[serde(default)]
        mode: MeasurementMode,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementMode {
    #[default]
    Direct,
    Horizontal,
    Vertical,
    Manhattan,
}

impl MeasurementMode {
    pub const ALL: [Self; 4] = [
        Self::Direct,
        Self::Horizontal,
        Self::Vertical,
        Self::Manhattan,
    ];

    pub fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "direct" | "distance" | "euclidean" => Some(Self::Direct),
            "horizontal" | "x" => Some(Self::Horizontal),
            "vertical" | "y" => Some(Self::Vertical),
            "manhattan" | "orthogonal" | "xy" => Some(Self::Manhattan),
            _ => None,
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Manhattan => "manhattan",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Direct => "Direct",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
            Self::Manhattan => "Manhattan",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShapeView<'a> {
    pub id: ShapeId,
    pub layer: LayerId,
    pub net: Option<NetId>,
    pub kind: ShapeKindView<'a>,
    pub name: Option<&'a str>,
    pub properties: &'a BTreeMap<String, String>,
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
        mode: MeasurementMode,
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
            properties: self.properties.clone(),
        }
    }
}

impl ShapeKindView<'_> {
    pub fn bounds(self) -> Rect {
        match self {
            Self::Rectangle(rect) => rect,
            Self::Polygon(poly) => poly.bounds().unwrap_or_else(|| {
                warn!("polygon view has no points; using default bounds");
                Rect::default()
            }),
            Self::Path { points, width } => Rect::from_points(points)
                .unwrap_or_else(|| {
                    warn!("path view has no points; using default bounds");
                    Rect::default()
                })
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
            Self::Measurement { a, b, label, mode } => ShapeKind::Measurement {
                a,
                b,
                label: label.to_string(),
                mode,
            },
        }
    }
}

pub(crate) const MAX_DENSE_SHAPE_ID_INDEX: usize = 10_000_000;

#[derive(Clone, Debug, Default)]
pub struct ShapeStore {
    pub(crate) alive: Vec<bool>,
    pub(crate) ids: Vec<ShapeId>,
    pub(crate) layers: Vec<LayerId>,
    pub(crate) nets: Vec<Option<NetId>>,
    pub(crate) geometry: Vec<ShapeGeometryRef>,
    pub(crate) names: Vec<Option<String>>,
    pub(crate) properties: Vec<BTreeMap<String, String>>,
    pub(crate) rectangles: Vec<Rect>,
    pub(crate) polygons: Vec<Polygon>,
    pub(crate) paths: Vec<PathGeometry>,
    pub(crate) vias: Vec<ViaGeometry>,
    pub(crate) labels: Vec<LabelGeometry>,
    pub(crate) measurements: Vec<MeasurementGeometry>,
    pub(crate) id_to_row: Vec<Option<usize>>,
    pub(crate) overflow_id_to_row: BTreeMap<ShapeId, usize>,
    pub(crate) len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeGeometryRef {
    Rectangle(usize),
    Polygon(usize),
    Path(usize),
    Via(usize),
    Label(usize),
    Measurement(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathGeometry {
    pub(crate) points: Vec<Point>,
    pub(crate) width: Coord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViaGeometry {
    pub(crate) center: Point,
    pub(crate) size: Coord,
    pub(crate) lower: LayerId,
    pub(crate) upper: LayerId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LabelGeometry {
    pub(crate) position: Point,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeasurementGeometry {
    pub(crate) a: Point,
    pub(crate) b: Point,
    pub(crate) label: String,
    pub(crate) mode: MeasurementMode,
}

pub struct ShapeMut<'a> {
    pub(crate) store: &'a mut ShapeStore,
    pub(crate) row: usize,
    pub(crate) old_id: ShapeId,
    pub(crate) shape: Shape,
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
            properties: Vec::with_capacity(capacity),
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
        self.properties.clear();
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

    pub(crate) fn live_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(row, _)| row)
    }

    pub(crate) fn row_id(&self, row: usize) -> ShapeId {
        self.ids[row]
    }

    pub(crate) fn row_layer(&self, row: usize) -> LayerId {
        self.layers[row]
    }

    pub(crate) fn row_bounds(&self, row: usize) -> Rect {
        self.geometry_bounds(self.geometry[row])
    }

    pub(crate) fn row_view(&self, row: usize) -> ShapeView<'_> {
        ShapeView {
            id: self.ids[row],
            layer: self.layers[row],
            net: self.nets[row],
            kind: self.geometry_view(self.geometry[row]),
            name: self.names[row].as_deref(),
            properties: &self.properties[row],
        }
    }

    pub(crate) fn row_for_id(&self, id: ShapeId) -> Option<usize> {
        dense_shape_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    pub(crate) fn set_row_for_id(&mut self, id: ShapeId, row: usize) {
        if let Some(index) = dense_shape_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            warn!(
                shape_id = id.0,
                max_dense_shape_id_index = MAX_DENSE_SHAPE_ID_INDEX,
                "shape id exceeded dense store range; using overflow index"
            );
            self.overflow_id_to_row.insert(id, row);
        }
    }

    pub(crate) fn clear_row_for_id(&mut self, id: ShapeId) {
        if let Some(index) = dense_shape_index(id)
            && let Some(slot) = self.id_to_row.get_mut(index)
        {
            *slot = None;
            return;
        }
        self.overflow_id_to_row.remove(&id);
    }

    pub(crate) fn push_row(&mut self, shape: Shape) {
        self.alive.push(true);
        self.ids.push(shape.id);
        self.layers.push(shape.layer);
        self.nets.push(shape.net);
        let geometry = self.push_geometry(shape.kind);
        self.geometry.push(geometry);
        self.names.push(shape.name);
        self.properties.push(shape.properties);
    }

    pub(crate) fn materialize_row(&self, row: usize) -> Shape {
        Shape {
            id: self.ids[row],
            layer: self.layers[row],
            net: self.nets[row],
            kind: self.materialize_geometry(self.geometry[row]),
            name: self.names[row].clone(),
            properties: self.properties[row].clone(),
        }
    }

    pub(crate) fn write_row(&mut self, row: usize, shape: Shape) {
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
        self.properties[row] = shape.properties;
        self.alive[row] = true;
    }

    pub(crate) fn push_geometry(&mut self, kind: ShapeKind) -> ShapeGeometryRef {
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
            ShapeKind::Measurement { a, b, label, mode } => {
                let index = self.measurements.len();
                self.measurements
                    .push(MeasurementGeometry { a, b, label, mode });
                ShapeGeometryRef::Measurement(index)
            }
        }
    }

    pub(crate) fn materialize_geometry(&self, geometry: ShapeGeometryRef) -> ShapeKind {
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
                    mode: measurement.mode,
                }
            }
        }
    }

    pub(crate) fn geometry_view(&self, geometry: ShapeGeometryRef) -> ShapeKindView<'_> {
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
                    mode: measurement.mode,
                }
            }
        }
    }

    pub(crate) fn geometry_bounds(&self, geometry: ShapeGeometryRef) -> Rect {
        match geometry {
            ShapeGeometryRef::Rectangle(index) => self.rectangles[index],
            ShapeGeometryRef::Polygon(index) => {
                self.polygons[index].bounds().unwrap_or_else(|| {
                    warn!(
                        polygon_index = index,
                        "stored polygon has no points; using default bounds"
                    );
                    Rect::default()
                })
            }
            ShapeGeometryRef::Path(index) => Rect::from_points(&self.paths[index].points)
                .unwrap_or_else(|| {
                    warn!(
                        path_index = index,
                        "stored path has no points; using default bounds"
                    );
                    Rect::default()
                })
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
        struct ShapeStoreVisitor;

        impl<'de> de::Visitor<'de> for ShapeStoreVisitor {
            type Value = ShapeStore;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a shape map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut store = ShapeStore::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((id, mut shape)) = map.next_entry::<ShapeId, Shape>()? {
                    shape.id = id;
                    let row = store.ids.len();
                    store.set_row_for_id(id, row);
                    store.push_row(shape);
                    store.len += 1;
                }
                Ok(store)
            }
        }

        deserializer.deserialize_map(ShapeStoreVisitor)
    }
}

pub(crate) fn dense_shape_index(id: ShapeId) -> Option<usize> {
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
        Rect::from_points(&corners).unwrap_or_else(|| {
            warn!("transformed rectangle corners were empty; using transformed min/max bounds");
            Rect::new(self.apply_point(rect.min), self.apply_point(rect.max))
        })
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
            ShapeKind::Measurement { a, b, label, mode } => ShapeKind::Measurement {
                a: self.apply_point(*a),
                b: self.apply_point(*b),
                label: label.clone(),
                mode: *mode,
            },
        }
    }

    pub(crate) fn apply_vector(self, vector: Vector) -> Vector {
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

pub(crate) fn identity_transform_matrix() -> [i8; 4] {
    [1, 0, 0, 1]
}

impl ShapeKind {
    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rectangle(rect) => *rect,
            Self::Polygon(poly) => poly.bounds().unwrap_or_else(|| {
                warn!("polygon has no points; using default bounds");
                Rect::default()
            }),
            Self::Path { points, width } => Rect::from_points(points)
                .unwrap_or_else(|| {
                    warn!("path has no points; using default bounds");
                    Rect::default()
                })
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
    pub shapes: ShapeStore,
    pub instances: InstanceStore,
}

impl Cell {
    pub fn new(id: CellId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            properties: BTreeMap::new(),
            shapes: ShapeStore::new(),
            instances: InstanceStore::new(),
        }
    }
}

pub(crate) const MAX_DENSE_CELL_ID_INDEX: usize = 10_000_000;

#[derive(Clone, Debug, Default)]
pub struct CellStore {
    pub(crate) alive: Vec<bool>,
    pub(crate) ids: Vec<CellId>,
    pub(crate) cells: Vec<Cell>,
    pub(crate) id_to_row: Vec<Option<usize>>,
    pub(crate) overflow_id_to_row: BTreeMap<CellId, usize>,
    pub(crate) len: usize,
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

    pub(crate) fn row_for_id(&self, id: CellId) -> Option<usize> {
        dense_cell_index(id)
            .and_then(|index| self.id_to_row.get(index).copied().flatten())
            .or_else(|| self.overflow_id_to_row.get(&id).copied())
            .filter(|row| self.alive.get(*row).copied().unwrap_or(false))
    }

    pub(crate) fn set_row_for_id(&mut self, id: CellId, row: usize) {
        if let Some(index) = dense_cell_index(id) {
            if index >= self.id_to_row.len() {
                self.id_to_row.resize(index + 1, None);
            }
            self.id_to_row[index] = Some(row);
        } else {
            warn!(
                cell_id = id.0,
                max_dense_cell_id_index = MAX_DENSE_CELL_ID_INDEX,
                "cell id exceeded dense store range; using overflow index"
            );
            self.overflow_id_to_row.insert(id, row);
        }
    }

    pub(crate) fn clear_row_for_id(&mut self, id: CellId) {
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
        struct CellStoreVisitor;

        impl<'de> de::Visitor<'de> for CellStoreVisitor {
            type Value = CellStore;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a cell map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut store = CellStore::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((id, cell)) = map.next_entry::<CellId, Cell>()? {
                    store.insert(id, cell);
                }
                Ok(store)
            }
        }

        deserializer.deserialize_map(CellStoreVisitor)
    }
}

pub(crate) fn dense_cell_index(id: CellId) -> Option<usize> {
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceImageSize {
    pub width: Coord,
    pub height: Coord,
}

impl ReferenceImageSize {
    pub const fn new(width: Coord, height: Coord) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceImageLandmark {
    pub name: String,
    pub image: Point,
    pub layout: Point,
}

impl ReferenceImageLandmark {
    pub fn new(name: impl Into<String>, image: Point, layout: Point) -> Self {
        Self {
            name: name.into(),
            image,
            layout,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReferenceImageOverlay {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub bounds: Rect,
    pub pixel_size: Option<ReferenceImageSize>,
    pub visible: bool,
    pub opacity: u8,
    pub landmarks: Vec<ReferenceImageLandmark>,
}

impl Default for ReferenceImageOverlay {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            uri: String::new(),
            bounds: Rect::default(),
            pixel_size: None,
            visible: true,
            opacity: 96,
            landmarks: Vec::new(),
        }
    }
}
