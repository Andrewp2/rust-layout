use std::{collections::BTreeMap, fmt};

use geometry_core::{Coord, Point, Rect, Vector};
use serde::{Deserialize, Serialize};

use crate::{
    CellId, DEFAULT_TOP_CELL_ID, Document, LayerId, ProcessLayer, ShapeId, ShapeKind,
    mes::{ProcessRoute, ProcessRouteId, ProcessStepId, ToolClass},
    recipe::{RecipeBinding, RecipeId},
};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(
            Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(ReticleId);
string_id!(ReticleFieldId);
string_id!(ExposureBlockId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskTone {
    ClearField,
    DarkField,
}

impl MaskTone {
    pub fn label(self) -> &'static str {
        match self {
            Self::ClearField => "clear field",
            Self::DarkField => "dark field",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReticleSpec {
    pub id: ReticleId,
    pub name: String,
    pub size: ReticleSize,
    pub edge_clearance: Coord,
    pub alignment_clearance: Coord,
}

impl ReticleSpec {
    pub fn printable_bounds(&self) -> Rect {
        let half_width = self.size.width / 2;
        let half_height = self.size.height / 2;
        let clearance = self.edge_clearance.max(0);
        Rect::new(
            Point::new(-half_width + clearance, -half_height + clearance),
            Point::new(half_width - clearance, half_height - clearance),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReticleSize {
    pub width: Coord,
    pub height: Coord,
    pub max_field_width: Coord,
    pub max_field_height: Coord,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaskLayer {
    pub layer: LayerId,
    pub process: ProcessLayer,
    pub name: String,
    pub tone: MaskTone,
    pub display_order: i32,
    pub critical: bool,
    pub min_feature: Coord,
    pub min_spacing: Coord,
    #[serde(default)]
    pub process_step_id: Option<ProcessStepId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldStepping {
    pub columns: u32,
    pub rows: u32,
    pub pitch: Vector,
}

impl FieldStepping {
    pub const fn single() -> Self {
        Self {
            columns: 1,
            rows: 1,
            pitch: Vector::ZERO,
        }
    }
}

impl Default for FieldStepping {
    fn default() -> Self {
        Self::single()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReticleField {
    pub id: ReticleFieldId,
    pub name: String,
    pub source_cell: CellId,
    pub layout_bounds: Rect,
    pub reticle_origin: Vector,
    #[serde(default)]
    pub stepping: FieldStepping,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExposureBlock {
    pub id: ExposureBlockId,
    pub name: String,
    pub field_id: ReticleFieldId,
    #[serde(default)]
    pub process_step_id: Option<ProcessStepId>,
    #[serde(default)]
    pub recipe: Option<RecipeBinding>,
    #[serde(default)]
    pub layer_ids: Vec<LayerId>,
    pub dose_mj_cm2: f64,
    pub focus_offset_um: f64,
    pub passes: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReticlePrep {
    pub mask_design_id: String,
    pub layout_revision: String,
    #[serde(default)]
    pub route_id: Option<ProcessRouteId>,
    pub reticle: ReticleSpec,
    pub layer_stack: Vec<MaskLayer>,
    pub fields: Vec<ReticleField>,
    pub exposure_blocks: Vec<ExposureBlock>,
}

impl ReticlePrep {
    pub fn from_document(document: &Document) -> Self {
        let bounds = layout_bounds_for_document(document).unwrap_or_else(default_field_bounds);
        let reticle = reticle_for_bounds("RETICLE-UNLINKED", "Unlinked reticle prep", bounds);
        let layer_stack = layer_stack_from_document(document, None);
        let field = ReticleField {
            id: ReticleFieldId::new("F-DEMO"),
            name: "Top layout field".to_string(),
            source_cell: document.top_cell,
            layout_bounds: bounds,
            reticle_origin: Vector::ZERO,
            stepping: FieldStepping::single(),
        };
        let exposure_layers = preferred_exposure_layers(&layer_stack);
        let block = ExposureBlock {
            id: ExposureBlockId::new("B-DEMO"),
            name: "Default exposure".to_string(),
            field_id: field.id.clone(),
            process_step_id: None,
            recipe: None,
            layer_ids: exposure_layers,
            dose_mj_cm2: 95.0,
            focus_offset_um: 0.0,
            passes: 1,
            bounds,
        };

        Self {
            mask_design_id: "UNLINKED-MASK".to_string(),
            layout_revision: format!(
                "layout_model:{}@schema-{}",
                document.name, document.schema_version
            ),
            route_id: None,
            reticle,
            layer_stack,
            fields: vec![field],
            exposure_blocks: vec![block],
        }
    }

    pub fn from_document_and_route(document: &Document, route: &ProcessRoute) -> Self {
        let exposure_step = route
            .steps
            .iter()
            .find(|step| step.required_tool_class == ToolClass::MaskAligner);
        let bounds = layout_bounds_for_document(document).unwrap_or_else(default_field_bounds);
        let reticle = reticle_for_bounds(
            &format!("{}-RETICLE-A", route.mask_design_id),
            &format!("{} reticle A", route.mask_design_id),
            bounds,
        );
        let exposure_step_id = exposure_step.map(|step| step.id.clone());
        let layer_stack = layer_stack_from_document(document, exposure_step_id.clone());
        let field = ReticleField {
            id: ReticleFieldId::new("F-POLY-CORE"),
            name: "Poly module field".to_string(),
            source_cell: document.top_cell,
            layout_bounds: bounds,
            reticle_origin: Vector::ZERO,
            stepping: FieldStepping::single(),
        };
        let recipe = exposure_step.map(|step| {
            RecipeBinding::new(RecipeId::new(step.required_recipe.as_str().to_string()), 1)
        });
        let block = ExposureBlock {
            id: ExposureBlockId::new(
                exposure_step
                    .map(|step| format!("B-{}", step.id.as_str()))
                    .unwrap_or_else(|| "B-EXPOSE".to_string()),
            ),
            name: exposure_step
                .map(|step| step.name.clone())
                .unwrap_or_else(|| "Mask exposure".to_string()),
            field_id: field.id.clone(),
            process_step_id: exposure_step_id,
            recipe,
            layer_ids: preferred_exposure_layers(&layer_stack),
            dose_mj_cm2: 95.0,
            focus_offset_um: 0.0,
            passes: 1,
            bounds,
        };

        Self {
            mask_design_id: route.mask_design_id.clone(),
            layout_revision: route.layout_revision.clone(),
            route_id: Some(route.id.clone()),
            reticle,
            layer_stack,
            fields: vec![field],
            exposure_blocks: vec![block],
        }
    }

    pub fn validate_document(&self, document: &Document) -> MaskCheckReport {
        let mut report = MaskCheckReport {
            layer_count: self.layer_stack.len(),
            field_count: self.fields.len(),
            exposure_block_count: self.exposure_blocks.len(),
            printable_shape_count: 0,
            issues: Vec::new(),
        };

        validate_reticle(self, &mut report);

        let mut layer_lookup = BTreeMap::new();
        for layer in &self.layer_stack {
            if layer_lookup.insert(layer.layer, layer).is_some() {
                report.issues.push(MaskPrepIssue::error(
                    "duplicate_layer",
                    format!(
                        "layer {} appears more than once in the mask stack",
                        layer.layer.0
                    ),
                ));
            }
            if document.layer(layer.layer).is_none() {
                report.issues.push(MaskPrepIssue::error(
                    "unknown_layer",
                    format!("mask stack references missing layer {}", layer.layer.0),
                ));
            }
            if layer.min_feature < 0 || layer.min_spacing < 0 {
                report.issues.push(MaskPrepIssue::error(
                    "negative_rule",
                    format!("{} has a negative reticle rule", layer.name),
                ));
            }
        }
        if self.layer_stack.is_empty() {
            report.issues.push(MaskPrepIssue::error(
                "empty_stack",
                "mask layer stack is empty",
            ));
        }

        let mut field_lookup = BTreeMap::new();
        for field in &self.fields {
            validate_field(document, &self.reticle, field, &mut report);
            if field_lookup.insert(field.id.clone(), field).is_some() {
                report.issues.push(
                    MaskPrepIssue::error(
                        "duplicate_field",
                        format!("reticle field {} appears more than once", field.id),
                    )
                    .with_field(field.id.clone()),
                );
            }
        }
        if self.fields.is_empty() {
            report.issues.push(MaskPrepIssue::error(
                "empty_fields",
                "reticle has no fields",
            ));
        }

        for block in &self.exposure_blocks {
            validate_exposure_block(block, &field_lookup, &layer_lookup, document, &mut report);
        }
        if self.exposure_blocks.is_empty() {
            report.issues.push(MaskPrepIssue::error(
                "empty_exposure_blocks",
                "reticle prep has no exposure blocks",
            ));
        }

        let shapes = mask_shapes_for_document(document, &layer_lookup);
        report.printable_shape_count = shapes.len();
        for layer in &self.layer_stack {
            if layer.critical && !shapes.iter().any(|shape| shape.layer == layer.layer) {
                report.issues.push(
                    MaskPrepIssue::warning(
                        "empty_critical_layer",
                        format!(
                            "critical mask layer {} has no printable geometry",
                            layer.name
                        ),
                    )
                    .with_layer(layer.layer),
                );
            }
        }

        validate_mask_geometry(&shapes, &layer_lookup, &mut report);
        report
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskIssueSeverity {
    Error,
    Warning,
}

impl MaskIssueSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaskPrepIssue {
    pub severity: MaskIssueSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub layer: Option<LayerId>,
    #[serde(default)]
    pub field_id: Option<ReticleFieldId>,
    #[serde(default)]
    pub block_id: Option<ExposureBlockId>,
    #[serde(default)]
    pub shape_id: Option<ShapeId>,
    #[serde(default)]
    pub bounds: Option<Rect>,
}

impl MaskPrepIssue {
    fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(MaskIssueSeverity::Error, code, message)
    }

    fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(MaskIssueSeverity::Warning, code, message)
    }

    fn new(
        severity: MaskIssueSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            layer: None,
            field_id: None,
            block_id: None,
            shape_id: None,
            bounds: None,
        }
    }

    fn with_layer(mut self, layer: LayerId) -> Self {
        self.layer = Some(layer);
        self
    }

    fn with_field(mut self, field_id: ReticleFieldId) -> Self {
        self.field_id = Some(field_id);
        self
    }

    fn with_block(mut self, block_id: ExposureBlockId) -> Self {
        self.block_id = Some(block_id);
        self
    }

    fn with_shape(mut self, shape_id: ShapeId, bounds: Rect) -> Self {
        self.shape_id = Some(shape_id);
        self.bounds = Some(bounds);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaskCheckReport {
    pub layer_count: usize,
    pub field_count: usize,
    pub exposure_block_count: usize,
    pub printable_shape_count: usize,
    pub issues: Vec<MaskPrepIssue>,
}

impl MaskCheckReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == MaskIssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == MaskIssueSeverity::Warning)
            .count()
    }

    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
struct MaskShapeRef {
    shape_id: ShapeId,
    layer: LayerId,
    bounds: Rect,
    min_feature: Option<f64>,
}

fn validate_reticle(prep: &ReticlePrep, report: &mut MaskCheckReport) {
    if prep.mask_design_id.trim().is_empty() {
        report.issues.push(MaskPrepIssue::error(
            "empty_mask_id",
            "mask design id is empty",
        ));
    }
    if prep.reticle.size.width <= 0 || prep.reticle.size.height <= 0 {
        report.issues.push(MaskPrepIssue::error(
            "bad_reticle_size",
            "reticle size must be positive",
        ));
    }
    if prep.reticle.size.max_field_width <= 0 || prep.reticle.size.max_field_height <= 0 {
        report.issues.push(MaskPrepIssue::error(
            "bad_field_limit",
            "reticle field limits must be positive",
        ));
    }
}

fn validate_field(
    document: &Document,
    reticle: &ReticleSpec,
    field: &ReticleField,
    report: &mut MaskCheckReport,
) {
    if field.name.trim().is_empty() {
        report.issues.push(
            MaskPrepIssue::error("empty_field_name", "reticle field name is empty")
                .with_field(field.id.clone()),
        );
    }
    if document.cell(field.source_cell).is_none() && field.source_cell != DEFAULT_TOP_CELL_ID {
        report.issues.push(
            MaskPrepIssue::error(
                "unknown_source_cell",
                format!(
                    "field {} references missing cell {}",
                    field.id, field.source_cell.0
                ),
            )
            .with_field(field.id.clone()),
        );
    }
    if field.layout_bounds.width() <= 0 || field.layout_bounds.height() <= 0 {
        report.issues.push(
            MaskPrepIssue::error(
                "empty_field_bounds",
                format!("field {} has empty layout bounds", field.id),
            )
            .with_field(field.id.clone()),
        );
    }
    if field.layout_bounds.width() > reticle.size.max_field_width
        || field.layout_bounds.height() > reticle.size.max_field_height
    {
        report.issues.push(
            MaskPrepIssue::error(
                "field_too_large",
                format!("field {} exceeds the reticle field limit", field.id),
            )
            .with_field(field.id.clone()),
        );
    }
    if field.stepping.columns == 0 || field.stepping.rows == 0 {
        report.issues.push(
            MaskPrepIssue::error(
                "bad_field_stepping",
                format!("field {} has zero stepping count", field.id),
            )
            .with_field(field.id.clone()),
        );
    }
}

fn validate_exposure_block(
    block: &ExposureBlock,
    fields: &BTreeMap<ReticleFieldId, &ReticleField>,
    layers: &BTreeMap<LayerId, &MaskLayer>,
    document: &Document,
    report: &mut MaskCheckReport,
) {
    let Some(field) = fields.get(&block.field_id) else {
        report.issues.push(
            MaskPrepIssue::error(
                "unknown_exposure_field",
                format!(
                    "block {} references missing field {}",
                    block.id, block.field_id
                ),
            )
            .with_block(block.id.clone()),
        );
        return;
    };
    if block.dose_mj_cm2 <= 0.0 || !block.dose_mj_cm2.is_finite() {
        report.issues.push(
            MaskPrepIssue::error(
                "bad_exposure_dose",
                format!("block {} has invalid exposure dose", block.id),
            )
            .with_block(block.id.clone()),
        );
    }
    if block.passes == 0 {
        report.issues.push(
            MaskPrepIssue::error(
                "bad_exposure_passes",
                format!("block {} has zero exposure passes", block.id),
            )
            .with_block(block.id.clone()),
        );
    }
    if block.bounds.width() <= 0 || block.bounds.height() <= 0 {
        report.issues.push(
            MaskPrepIssue::error(
                "empty_exposure_bounds",
                format!("block {} has empty exposure bounds", block.id),
            )
            .with_block(block.id.clone()),
        );
    } else if !field.layout_bounds.contains_rect(block.bounds) {
        report.issues.push(
            MaskPrepIssue::warning(
                "block_outside_field",
                format!("block {} extends outside field {}", block.id, field.id),
            )
            .with_block(block.id.clone()),
        );
    }
    if block.layer_ids.is_empty() {
        report.issues.push(
            MaskPrepIssue::warning(
                "empty_block_layers",
                format!("block {} does not expose any mask layers", block.id),
            )
            .with_block(block.id.clone()),
        );
    }
    for layer_id in &block.layer_ids {
        if !layers.contains_key(layer_id) {
            report.issues.push(
                MaskPrepIssue::error(
                    "block_layer_not_in_stack",
                    format!(
                        "block {} uses layer {} outside the mask stack",
                        block.id, layer_id.0
                    ),
                )
                .with_block(block.id.clone())
                .with_layer(*layer_id),
            );
        }
        if document.layer(*layer_id).is_none() {
            report.issues.push(
                MaskPrepIssue::error(
                    "block_unknown_layer",
                    format!("block {} references missing layer {}", block.id, layer_id.0),
                )
                .with_block(block.id.clone())
                .with_layer(*layer_id),
            );
        }
    }
}

fn validate_mask_geometry(
    shapes: &[MaskShapeRef],
    layers: &BTreeMap<LayerId, &MaskLayer>,
    report: &mut MaskCheckReport,
) {
    for shape in shapes {
        let Some(layer) = layers.get(&shape.layer) else {
            continue;
        };
        if let Some(min_feature) = shape.min_feature
            && min_feature < layer.min_feature as f64
        {
            report.issues.push(
                MaskPrepIssue::error(
                    "min_feature",
                    format!(
                        "{} shape {} is {:.0} dbu wide, below reticle minimum {}",
                        layer.name, shape.shape_id.0, min_feature, layer.min_feature
                    ),
                )
                .with_layer(shape.layer)
                .with_shape(shape.shape_id, shape.bounds),
            );
        }
    }

    for left_index in 0..shapes.len() {
        for right in &shapes[left_index + 1..] {
            let left = shapes[left_index];
            if left.layer != right.layer {
                continue;
            }
            let Some(layer) = layers.get(&left.layer) else {
                continue;
            };
            if layer.min_spacing <= 0 {
                continue;
            }
            let distance = left.bounds.distance_to_rect(right.bounds);
            if distance < layer.min_spacing as f64 {
                report.issues.push(
                    MaskPrepIssue::warning(
                        "min_spacing",
                        format!(
                            "{} shapes {} and {} are {:.0} dbu apart, below reticle spacing {}",
                            layer.name,
                            left.shape_id.0,
                            right.shape_id.0,
                            distance,
                            layer.min_spacing
                        ),
                    )
                    .with_layer(left.layer)
                    .with_shape(left.shape_id, left.bounds.union(right.bounds)),
                );
            }
        }
    }
}

fn layer_stack_from_document(
    document: &Document,
    exposure_step_id: Option<ProcessStepId>,
) -> Vec<MaskLayer> {
    let mut layers: Vec<_> = document.layers.values().collect();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    layers
        .into_iter()
        .filter(|layer| layer.process != ProcessLayer::Annotation)
        .map(|layer| MaskLayer {
            layer: layer.id,
            process: layer.process,
            name: layer.name.clone(),
            tone: default_tone_for_process(layer.process),
            display_order: layer.display_order,
            critical: layer.process != ProcessLayer::Oxide,
            min_feature: default_min_feature_for_process(layer.process),
            min_spacing: default_min_spacing_for_process(layer.process),
            process_step_id: exposure_step_id.clone(),
        })
        .collect()
}

fn preferred_exposure_layers(layer_stack: &[MaskLayer]) -> Vec<LayerId> {
    layer_stack
        .iter()
        .find(|layer| layer.process == ProcessLayer::Poly)
        .or_else(|| layer_stack.iter().find(|layer| layer.critical))
        .map(|layer| vec![layer.layer])
        .unwrap_or_default()
}

fn reticle_for_bounds(id: &str, name: &str, field_bounds: Rect) -> ReticleSpec {
    let field_width = field_bounds.width().max(1);
    let field_height = field_bounds.height().max(1);
    let width = (field_width + 6_000).max(12_000);
    let height = (field_height + 6_000).max(10_000);
    ReticleSpec {
        id: ReticleId::new(id),
        name: name.to_string(),
        size: ReticleSize {
            width,
            height,
            max_field_width: (width - 2_000).max(1),
            max_field_height: (height - 2_000).max(1),
        },
        edge_clearance: 800,
        alignment_clearance: 500,
    }
}

fn layout_bounds_for_document(document: &Document) -> Option<Rect> {
    document
        .visible_flattened_shapes()
        .into_iter()
        .filter(|shape| is_printable_kind(&shape.shape.kind))
        .map(|shape| shape.bounds)
        .reduce(Rect::union)
}

fn default_field_bounds() -> Rect {
    Rect::from_min_size(Point::new(-2_500, -2_000), 5_000, 4_000)
}

fn mask_shapes_for_document(
    document: &Document,
    layers: &BTreeMap<LayerId, &MaskLayer>,
) -> Vec<MaskShapeRef> {
    document
        .visible_flattened_shapes()
        .into_iter()
        .filter(|shape| layers.contains_key(&shape.shape.layer))
        .filter(|shape| is_printable_kind(&shape.shape.kind))
        .map(|shape| MaskShapeRef {
            shape_id: shape.source_shape_id(),
            layer: shape.shape.layer,
            bounds: shape.bounds,
            min_feature: shape_min_feature(&shape.shape.kind),
        })
        .collect()
}

fn is_printable_kind(kind: &ShapeKind) -> bool {
    matches!(
        kind,
        ShapeKind::Rectangle(_)
            | ShapeKind::Polygon(_)
            | ShapeKind::Path { .. }
            | ShapeKind::Via { .. }
    )
}

fn shape_min_feature(kind: &ShapeKind) -> Option<f64> {
    match kind {
        ShapeKind::Rectangle(rect) => Some(rect.width().min(rect.height()) as f64),
        ShapeKind::Polygon(polygon) => polygon.min_edge_length(),
        ShapeKind::Path { width, .. } => Some(*width as f64),
        ShapeKind::Via { size, .. } => Some(*size as f64),
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => None,
    }
}

fn default_tone_for_process(process: ProcessLayer) -> MaskTone {
    match process {
        ProcessLayer::Contact | ProcessLayer::Via1 => MaskTone::DarkField,
        ProcessLayer::Diffusion
        | ProcessLayer::Poly
        | ProcessLayer::Metal1
        | ProcessLayer::Metal2
        | ProcessLayer::Oxide
        | ProcessLayer::Annotation => MaskTone::ClearField,
    }
}

fn default_min_feature_for_process(process: ProcessLayer) -> Coord {
    match process {
        ProcessLayer::Diffusion => 180,
        ProcessLayer::Poly => 140,
        ProcessLayer::Contact => 120,
        ProcessLayer::Metal1 => 220,
        ProcessLayer::Via1 => 140,
        ProcessLayer::Metal2 => 260,
        ProcessLayer::Oxide => 300,
        ProcessLayer::Annotation => 0,
    }
}

fn default_min_spacing_for_process(process: ProcessLayer) -> Coord {
    match process {
        ProcessLayer::Diffusion => 160,
        ProcessLayer::Poly => 140,
        ProcessLayer::Contact => 120,
        ProcessLayer::Metal1 => 220,
        ProcessLayer::Via1 => 140,
        ProcessLayer::Metal2 => 260,
        ProcessLayer::Oxide => 180,
        ProcessLayer::Annotation => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, mes::demo_process_route};

    #[test]
    fn route_builder_links_mask_id_exposure_step_and_recipe() {
        let document = Document::demo();
        let route = demo_process_route();
        let prep = ReticlePrep::from_document_and_route(&document, &route);

        assert_eq!(prep.mask_design_id, "FABRICAD-DEMO-INVERTER");
        assert_eq!(prep.layout_revision, "layout_model:demo-inverter@rev-5");
        assert_eq!(prep.route_id.as_ref(), Some(&route.id));
        assert!(
            prep.layer_stack
                .iter()
                .any(|layer| layer.process == ProcessLayer::Poly)
        );

        let block = prep.exposure_blocks.first().expect("exposure block");
        assert_eq!(
            block.process_step_id.as_ref().map(ProcessStepId::as_str),
            Some("S020-EXPOSE")
        );
        assert_eq!(
            block
                .recipe
                .as_ref()
                .map(|binding| binding.recipe_id.as_str()),
            Some("LITHO_POLY_EXPOSE_001")
        );
    }

    #[test]
    fn validation_flags_min_feature_errors() {
        let document = Document::demo();
        let route = demo_process_route();
        let mut prep = ReticlePrep::from_document_and_route(&document, &route);
        let poly = prep
            .layer_stack
            .iter_mut()
            .find(|layer| layer.process == ProcessLayer::Poly)
            .expect("poly mask layer");
        poly.min_feature = 400;
        let poly_layer = poly.layer;

        let report = prep.validate_document(&document);

        assert!(report.error_count() >= 1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "min_feature" && issue.layer == Some(poly_layer))
        );
    }

    #[test]
    fn validation_flags_bad_exposure_blocks() {
        let document = Document::demo();
        let route = demo_process_route();
        let mut prep = ReticlePrep::from_document_and_route(&document, &route);
        let block = prep.exposure_blocks.first_mut().expect("exposure block");
        block.dose_mj_cm2 = 0.0;
        block.passes = 0;
        block.layer_ids.push(LayerId(99));

        let report = prep.validate_document(&document);

        assert!(report.error_count() >= 3);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "bad_exposure_dose")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "bad_exposure_passes")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "block_unknown_layer")
        );
    }
}
