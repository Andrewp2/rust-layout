use std::collections::{BTreeMap, BTreeSet};

use geometry_core::Rect;

use crate::{Document, FlattenedShape, LayerId, LayoutIndex, Shape, ShapeId, ShapeOccurrenceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDiffReport {
    pub baseline_name: String,
    pub candidate_name: String,
    pub summary: LayoutDiffSummary,
    pub baseline_bounds: Option<Rect>,
    pub candidate_bounds: Option<Rect>,
    pub layers: Vec<LayerDiffSummary>,
    pub changes: Vec<ShapeChange>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutDiffSummary {
    pub baseline_shapes: usize,
    pub candidate_shapes: usize,
    pub added_shapes: usize,
    pub removed_shapes: usize,
    pub modified_shapes: usize,
    pub baseline_layers: usize,
    pub candidate_layers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerDiffSummary {
    pub layer: LayerId,
    pub name: String,
    pub baseline_shapes: usize,
    pub candidate_shapes: usize,
    pub added_shapes: usize,
    pub removed_shapes: usize,
    pub modified_shapes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeChange {
    pub kind: ShapeChangeKind,
    pub occurrence: ShapeOccurrenceId,
    pub id: ShapeId,
    pub layer: LayerId,
    pub layer_name: String,
    pub bounds: Rect,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeChangeKind {
    Added,
    Removed,
    Modified,
}

impl ShapeChangeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
        }
    }
}

pub fn diff_documents(
    baseline_name: impl Into<String>,
    baseline: &Document,
    candidate_name: impl Into<String>,
    candidate: &Document,
) -> LayoutDiffReport {
    let baseline_shapes = shape_map(baseline);
    let candidate_shapes = shape_map(candidate);
    let mut changes = Vec::new();
    let mut layer_ids = baseline
        .layers
        .keys()
        .chain(candidate.layers.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    for (occurrence, shape) in &candidate_shapes {
        if !baseline_shapes.contains_key(occurrence) {
            layer_ids.insert(shape.shape.layer);
            changes.push(shape_change(
                ShapeChangeKind::Added,
                candidate,
                shape,
                "shape exists only in candidate",
            ));
        }
    }

    for (occurrence, shape) in &baseline_shapes {
        if !candidate_shapes.contains_key(occurrence) {
            layer_ids.insert(shape.shape.layer);
            changes.push(shape_change(
                ShapeChangeKind::Removed,
                baseline,
                shape,
                "shape exists only in baseline",
            ));
        }
    }

    for (occurrence, baseline_shape) in &baseline_shapes {
        let Some(candidate_shape) = candidate_shapes.get(occurrence) else {
            continue;
        };
        let baseline_shape_for_diff = flattened_shape_for_diff(baseline_shape);
        let candidate_shape_for_diff = flattened_shape_for_diff(candidate_shape);
        if baseline_shape_for_diff != candidate_shape_for_diff {
            layer_ids.insert(candidate_shape_for_diff.layer);
            changes.push(shape_change(
                ShapeChangeKind::Modified,
                candidate,
                candidate_shape,
                modified_detail(&baseline_shape_for_diff, &candidate_shape_for_diff),
            ));
        }
    }

    changes.sort_by_key(|change| (change.layer, change.occurrence.clone(), change.kind.label()));
    let layers = layer_ids
        .into_iter()
        .map(|layer| layer_summary(layer, baseline, candidate, &changes))
        .collect::<Vec<_>>();

    LayoutDiffReport {
        baseline_name: baseline_name.into(),
        candidate_name: candidate_name.into(),
        summary: LayoutDiffSummary {
            baseline_shapes: baseline_shapes.len(),
            candidate_shapes: candidate_shapes.len(),
            added_shapes: changes
                .iter()
                .filter(|change| change.kind == ShapeChangeKind::Added)
                .count(),
            removed_shapes: changes
                .iter()
                .filter(|change| change.kind == ShapeChangeKind::Removed)
                .count(),
            modified_shapes: changes
                .iter()
                .filter(|change| change.kind == ShapeChangeKind::Modified)
                .count(),
            baseline_layers: baseline.layers.len(),
            candidate_layers: candidate.layers.len(),
        },
        baseline_bounds: LayoutIndex::rebuild_hierarchical(baseline).bounds(),
        candidate_bounds: LayoutIndex::rebuild_hierarchical(candidate).bounds(),
        layers,
        changes,
    }
}

fn shape_map(document: &Document) -> BTreeMap<ShapeOccurrenceId, FlattenedShape> {
    document
        .visible_flattened_shapes()
        .into_iter()
        .map(|shape| (shape.id.clone(), shape))
        .collect()
}

fn flattened_shape_for_diff(flattened: &FlattenedShape) -> Shape {
    flattened.transformed_shape()
}

fn shape_change(
    kind: ShapeChangeKind,
    document: &Document,
    shape: &FlattenedShape,
    detail: impl Into<String>,
) -> ShapeChange {
    let transformed = flattened_shape_for_diff(shape);
    ShapeChange {
        kind,
        occurrence: shape.id.clone(),
        id: shape.source_shape_id(),
        layer: transformed.layer,
        layer_name: layer_name(document, transformed.layer),
        bounds: shape.bounds,
        detail: detail.into(),
    }
}

fn modified_detail(baseline: &Shape, candidate: &Shape) -> String {
    let mut details = Vec::new();
    if baseline.layer != candidate.layer {
        details.push(format!(
            "layer {} -> {}",
            baseline.layer.0, candidate.layer.0
        ));
    }
    if baseline.kind != candidate.kind {
        details.push("geometry changed".to_string());
    }
    if baseline.net != candidate.net {
        details.push("net changed".to_string());
    }
    if baseline.name != candidate.name {
        details.push("name changed".to_string());
    }
    if details.is_empty() {
        "metadata changed".to_string()
    } else {
        details.join(", ")
    }
}

fn layer_summary(
    layer: LayerId,
    baseline: &Document,
    candidate: &Document,
    changes: &[ShapeChange],
) -> LayerDiffSummary {
    LayerDiffSummary {
        layer,
        name: layer_name(candidate, layer).or_else_unknown(layer_name(baseline, layer)),
        baseline_shapes: baseline
            .visible_flattened_shapes()
            .iter()
            .filter(|shape| shape.shape.layer == layer)
            .count(),
        candidate_shapes: candidate
            .visible_flattened_shapes()
            .iter()
            .filter(|shape| shape.shape.layer == layer)
            .count(),
        added_shapes: changes
            .iter()
            .filter(|change| change.layer == layer && change.kind == ShapeChangeKind::Added)
            .count(),
        removed_shapes: changes
            .iter()
            .filter(|change| change.layer == layer && change.kind == ShapeChangeKind::Removed)
            .count(),
        modified_shapes: changes
            .iter()
            .filter(|change| change.layer == layer && change.kind == ShapeChangeKind::Modified)
            .count(),
    }
}

trait UnknownNameFallback {
    fn or_else_unknown(self, fallback: String) -> String;
}

impl UnknownNameFallback for String {
    fn or_else_unknown(self, fallback: String) -> String {
        if self.starts_with("layer ") {
            fallback
        } else {
            self
        }
    }
}

fn layer_name(document: &Document, layer: LayerId) -> String {
    document
        .layers
        .get(&layer)
        .map(|layer| layer.name.clone())
        .unwrap_or_else(|| format!("layer {}", layer.0))
}

#[cfg(test)]
mod tests {
    use geometry_core::{Point, Rect, Vector};

    use crate::{Document, Operation, ProcessLayer, ShapeKind, Transform};

    use super::*;

    #[test]
    fn diff_reports_added_removed_and_modified_shapes() {
        let mut baseline = Document::new("baseline");
        let metal1 = baseline.layer_by_process(ProcessLayer::Metal1).unwrap();
        let kept = baseline.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
        );
        let removed = baseline.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 100, 100)),
        );
        let mut candidate = baseline.clone();
        candidate.apply_operation_without_log(&Operation::DeleteShape { id: removed });
        candidate.apply_operation_without_log(&Operation::MoveShape {
            id: kept,
            delta: Vector::new(10, 0),
        });
        candidate.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(600, 0), 100, 100)),
        );

        let report = diff_documents("baseline", &baseline, "candidate", &candidate);

        assert_eq!(report.summary.added_shapes, 1);
        assert_eq!(report.summary.removed_shapes, 1);
        assert_eq!(report.summary.modified_shapes, 1);
        assert_eq!(report.changes.len(), 3);
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.kind == ShapeChangeKind::Modified && change.id == kept)
        );
    }

    #[test]
    fn diff_reports_changes_in_hierarchical_visible_geometry() {
        let mut baseline = Document::new("hierarchical baseline");
        let metal1 = baseline.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = baseline.create_cell("unit");
        let child_shape = baseline
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
            )
            .unwrap();
        let instance = baseline
            .insert_instance_in_top(child, Transform::translate(0, 0))
            .unwrap();
        let mut candidate = baseline.clone();
        candidate.apply_operation_without_log(&Operation::MoveInstance {
            parent: candidate.top_cell,
            id: instance,
            delta: Vector::new(500, 0),
        });

        let report = diff_documents("baseline", &baseline, "candidate", &candidate);

        assert_eq!(report.summary.baseline_shapes, 1);
        assert_eq!(report.summary.candidate_shapes, 1);
        assert_eq!(report.summary.added_shapes, 0);
        assert_eq!(report.summary.removed_shapes, 0);
        assert_eq!(report.summary.modified_shapes, 1);
        assert_eq!(
            report.baseline_bounds,
            Some(Rect::from_min_size(Point::new(0, 0), 100, 100))
        );
        assert_eq!(
            report.candidate_bounds,
            Some(Rect::from_min_size(Point::new(500, 0), 100, 100))
        );

        let change = &report.changes[0];
        assert_eq!(change.kind, ShapeChangeKind::Modified);
        assert_eq!(change.id, child_shape);
        assert!(!change.occurrence.is_top_level());
        assert_eq!(
            change.bounds,
            Rect::from_min_size(Point::new(500, 0), 100, 100)
        );
        assert_eq!(change.detail, "geometry changed");
    }
}
