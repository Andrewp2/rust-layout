use std::collections::{BTreeMap, BTreeSet};

use geometry_core::Rect;

use crate::{Document, LayerId, LayoutIndex, Shape, ShapeId};

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

    for (id, shape) in &candidate_shapes {
        if !baseline_shapes.contains_key(id) {
            layer_ids.insert(shape.layer);
            changes.push(shape_change(
                ShapeChangeKind::Added,
                candidate,
                shape,
                "shape exists only in candidate",
            ));
        }
    }

    for (id, shape) in &baseline_shapes {
        if !candidate_shapes.contains_key(id) {
            layer_ids.insert(shape.layer);
            changes.push(shape_change(
                ShapeChangeKind::Removed,
                baseline,
                shape,
                "shape exists only in baseline",
            ));
        }
    }

    for (id, baseline_shape) in &baseline_shapes {
        let Some(candidate_shape) = candidate_shapes.get(id) else {
            continue;
        };
        if baseline_shape != candidate_shape {
            layer_ids.insert(candidate_shape.layer);
            changes.push(shape_change(
                ShapeChangeKind::Modified,
                candidate,
                candidate_shape,
                modified_detail(baseline_shape, candidate_shape),
            ));
        }
    }

    changes.sort_by_key(|change| (change.layer, change.id, change.kind.label()));
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
        baseline_bounds: LayoutIndex::rebuild(baseline).bounds(),
        candidate_bounds: LayoutIndex::rebuild(candidate).bounds(),
        layers,
        changes,
    }
}

fn shape_map(document: &Document) -> BTreeMap<ShapeId, Shape> {
    document
        .shapes
        .values()
        .map(|shape| (shape.id, shape))
        .collect()
}

fn shape_change(
    kind: ShapeChangeKind,
    document: &Document,
    shape: &Shape,
    detail: impl Into<String>,
) -> ShapeChange {
    ShapeChange {
        kind,
        id: shape.id,
        layer: shape.layer,
        layer_name: layer_name(document, shape.layer),
        bounds: shape.kind.bounds(),
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
            .shapes
            .values()
            .filter(|shape| shape.layer == layer)
            .count(),
        candidate_shapes: candidate
            .shapes
            .values()
            .filter(|shape| shape.layer == layer)
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

    use crate::{Document, Operation, ProcessLayer, ShapeKind};

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
}
