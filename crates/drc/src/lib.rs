use std::collections::BTreeMap;

use geometry_core::{Coord, Point, Rect};
use layout_model::{
    Document, LayerId, Shape, ShapeId, ShapeKind, TechnologyError, TechnologyFile,
    default_technology,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDeck {
    pub grid: Coord,
    pub min_width: BTreeMap<LayerId, Coord>,
    pub min_spacing: BTreeMap<LayerId, Coord>,
    pub via_enclosure: Vec<EnclosureRule>,
    pub forbidden_overlaps: Vec<ForbiddenOverlapRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnclosureRule {
    pub via_layer: LayerId,
    pub enclosure_layer: LayerId,
    pub required: Coord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForbiddenOverlapRule {
    pub a: LayerId,
    pub b: LayerId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrcViolation {
    pub id: usize,
    pub rule: String,
    pub message: String,
    pub shape_ids: Vec<ShapeId>,
    pub bounds: Rect,
    pub required: Coord,
    pub actual: f64,
}

impl RuleDeck {
    pub fn from_technology(
        document: &Document,
        technology: &TechnologyFile,
    ) -> Result<Self, TechnologyError> {
        technology.validate()?;
        let mut min_width = BTreeMap::new();
        for rule in &technology.drc.min_width {
            if rule.value > 0 {
                min_width.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut min_spacing = BTreeMap::new();
        for rule in &technology.drc.min_spacing {
            if rule.value > 0 {
                min_spacing.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut via_enclosure = Vec::new();
        for rule in &technology.drc.via_enclosure {
            if rule.required > 0 {
                via_enclosure.push(EnclosureRule {
                    via_layer: document_layer_id(document, &rule.via)?,
                    enclosure_layer: document_layer_id(document, &rule.enclosure)?,
                    required: rule.required,
                });
            }
        }

        let mut forbidden_overlaps = Vec::new();
        for rule in &technology.drc.forbidden_overlaps {
            forbidden_overlaps.push(ForbiddenOverlapRule {
                a: document_layer_id(document, &rule.a)?,
                b: document_layer_id(document, &rule.b)?,
                name: rule.name.clone(),
            });
        }

        Ok(Self {
            grid: technology.grid,
            min_width,
            min_spacing,
            via_enclosure,
            forbidden_overlaps,
        })
    }

    pub fn demo(document: &Document) -> Self {
        Self::from_technology(document, &default_technology())
            .expect("default DRC technology applies to default document layers")
    }
}

pub fn run_drc(document: &Document, rules: &RuleDeck) -> Vec<DrcViolation> {
    let mut violations = Vec::new();
    check_grid(document, rules, &mut violations);
    check_min_width(document, rules, &mut violations);
    check_spacing(document, rules, &mut violations);
    check_forbidden_overlaps(document, rules, &mut violations);
    check_via_enclosure(document, rules, &mut violations);
    for (id, violation) in violations.iter_mut().enumerate() {
        violation.id = id + 1;
    }
    violations
}

pub fn run_drc_incremental(
    document: &Document,
    rules: &RuleDeck,
    previous: &[DrcViolation],
    dirty_region: Rect,
) -> Vec<DrcViolation> {
    let affected_region = dirty_region.expanded(max_rule_distance(rules));
    let mut merged = previous
        .iter()
        .filter(|violation| !violation.bounds.intersects(affected_region))
        .cloned()
        .collect::<Vec<_>>();
    merged.extend(
        run_drc(document, rules)
            .into_iter()
            .filter(|violation| violation.bounds.intersects(affected_region)),
    );
    for (id, violation) in merged.iter_mut().enumerate() {
        violation.id = id + 1;
    }
    merged
}

fn max_rule_distance(rules: &RuleDeck) -> Coord {
    rules
        .min_width
        .values()
        .chain(rules.min_spacing.values())
        .chain(rules.via_enclosure.iter().map(|rule| &rule.required))
        .copied()
        .max()
        .unwrap_or(rules.grid)
        .max(rules.grid)
}

fn document_layer_id(document: &Document, reference: &str) -> Result<LayerId, TechnologyError> {
    let normalized = normalize_layer_ref(reference);
    document
        .layers
        .values()
        .find(|layer| normalize_layer_ref(&layer.name) == normalized)
        .map(|layer| layer.id)
        .ok_or_else(|| {
            TechnologyError::Invalid(format!(
                "technology rule references missing document layer {reference:?}"
            ))
        })
}

fn normalize_layer_ref(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .filter(|character| *character != '_' && *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn check_grid(document: &Document, rules: &RuleDeck, violations: &mut Vec<DrcViolation>) {
    if rules.grid <= 1 {
        return;
    }
    for shape in document.visible_shapes() {
        let off_grid = shape
            .kind
            .key_points()
            .into_iter()
            .any(|point| point.x % rules.grid != 0 || point.y % rules.grid != 0);
        if off_grid {
            violations.push(DrcViolation {
                id: 0,
                rule: "off_grid".to_string(),
                message: format!(
                    "shape {:?} has vertices off the {} dbu manufacturing grid",
                    shape.id, rules.grid
                ),
                shape_ids: vec![shape.id],
                bounds: shape.kind.bounds().expanded(rules.grid),
                required: rules.grid,
                actual: 0.0,
            });
        }
    }
}

fn check_min_width(document: &Document, rules: &RuleDeck, violations: &mut Vec<DrcViolation>) {
    for shape in document.visible_shapes() {
        let Some(required) = rules.min_width.get(&shape.layer).copied() else {
            continue;
        };
        let actual = approximate_width(&shape);
        if actual > 0.0 && actual < required as f64 {
            violations.push(DrcViolation {
                id: 0,
                rule: "min_width".to_string(),
                message: format!("minimum width is {} dbu, found {:.1} dbu", required, actual),
                shape_ids: vec![shape.id],
                bounds: shape.kind.bounds().expanded(required),
                required,
                actual,
            });
        }
    }
}

fn check_spacing(document: &Document, rules: &RuleDeck, violations: &mut Vec<DrcViolation>) {
    let shapes: Vec<Shape> = document.visible_shapes().collect();
    for left_index in 0..shapes.len() {
        let left = &shapes[left_index];
        let Some(required) = rules.min_spacing.get(&left.layer).copied() else {
            continue;
        };
        if required <= 0 {
            continue;
        }
        let left_bounds = left.kind.bounds();
        for right in &shapes[left_index + 1..] {
            if left.layer != right.layer {
                continue;
            }
            let right_bounds = right.kind.bounds();
            if left_bounds.intersects(right_bounds) {
                continue;
            }
            let actual = left_bounds.distance_to_rect(right_bounds);
            if actual < required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: "min_spacing".to_string(),
                    message: format!(
                        "same-layer spacing is {:.1} dbu, below required {} dbu",
                        actual, required
                    ),
                    shape_ids: vec![left.id, right.id],
                    bounds: left_bounds.union(right_bounds).expanded(required),
                    required,
                    actual,
                });
            }
        }
    }
}

fn check_forbidden_overlaps(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    let shapes: Vec<Shape> = document.visible_shapes().collect();
    for rule in &rules.forbidden_overlaps {
        for left_index in 0..shapes.len() {
            let left = &shapes[left_index];
            if left.layer != rule.a && left.layer != rule.b {
                continue;
            }
            for right in &shapes[left_index + 1..] {
                let matching_layers = (left.layer == rule.a && right.layer == rule.b)
                    || (left.layer == rule.b && right.layer == rule.a);
                if !matching_layers {
                    continue;
                }
                let left_bounds = left.kind.bounds();
                let right_bounds = right.kind.bounds();
                if let Some(overlap) = left_bounds.intersection(right_bounds) {
                    violations.push(DrcViolation {
                        id: 0,
                        rule: rule.name.clone(),
                        message: "forbidden process layers overlap".to_string(),
                        shape_ids: vec![left.id, right.id],
                        bounds: overlap.expanded(80),
                        required: 0,
                        actual: overlap.area() as f64,
                    });
                }
            }
        }
    }
}

fn check_via_enclosure(document: &Document, rules: &RuleDeck, violations: &mut Vec<DrcViolation>) {
    let shapes: Vec<Shape> = document.visible_shapes().collect();
    for rule in &rules.via_enclosure {
        for via in shapes.iter().filter(|shape| shape.layer == rule.via_layer) {
            let via_bounds = via.kind.bounds();
            let required_rect = via_bounds.expanded(rule.required);
            let enclosed = shapes
                .iter()
                .filter(|shape| shape.layer == rule.enclosure_layer)
                .any(|shape| shape.kind.bounds().contains_rect(required_rect));
            if !enclosed {
                violations.push(DrcViolation {
                    id: 0,
                    rule: "via_enclosure".to_string(),
                    message: format!(
                        "via requires {} dbu enclosure on layer {:?}",
                        rule.required, rule.enclosure_layer
                    ),
                    shape_ids: vec![via.id],
                    bounds: required_rect.expanded(rule.required),
                    required: rule.required,
                    actual: 0.0,
                });
            }
        }
    }
}

fn approximate_width(shape: &Shape) -> f64 {
    match &shape.kind {
        ShapeKind::Rectangle(rect) => rect.width().min(rect.height()) as f64,
        ShapeKind::Polygon(poly) => {
            let bbox_width = poly
                .bounds()
                .map(|rect| rect.width().min(rect.height()) as f64)
                .unwrap_or(0.0);
            poly.min_edge_length().unwrap_or(bbox_width).min(bbox_width)
        }
        ShapeKind::Path { width, .. } => *width as f64,
        ShapeKind::Via { size, .. } => *size as f64,
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => 0.0,
    }
}

pub fn violation_marker(point: Point, size: Coord) -> Rect {
    Rect::new(
        Point::new(point.x - size, point.y - size),
        Point::new(point.x + size, point.y + size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_model::{ProcessLayer, ShapeKind, default_technology};

    #[test]
    fn finds_min_width_violation() {
        let mut doc = Document::new("test");
        let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        doc.insert_shape(
            layer,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        );
        let rules = RuleDeck::demo(&doc);
        let violations = run_drc(&doc, &rules);
        assert!(
            violations
                .iter()
                .any(|violation| violation.rule == "min_width")
        );
    }

    #[test]
    fn technology_spacing_rules_change_drc_results() {
        let mut doc = Document::new("spacing tech");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 500, 500)),
        );
        doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(900, 0), 500, 500)),
        );

        let default_rules = RuleDeck::from_technology(&doc, &default_technology()).unwrap();
        assert!(
            !run_drc(&doc, &default_rules)
                .iter()
                .any(|violation| violation.rule == "min_spacing")
        );

        let mut technology = default_technology();
        let metal_spacing = technology
            .drc
            .min_spacing
            .iter_mut()
            .find(|rule| rule.layer == "metal1")
            .unwrap();
        metal_spacing.value = 700;
        let strict_rules = RuleDeck::from_technology(&doc, &technology).unwrap();

        assert!(
            run_drc(&doc, &strict_rules)
                .iter()
                .any(|violation| violation.rule == "min_spacing")
        );
    }

    #[test]
    fn incremental_drc_preserves_markers_outside_dirty_region() {
        let mut doc = Document::new("incremental drc");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let first = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        );
        let second = doc.insert_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(10_000, 0), 100, 50)),
        );
        let rules = RuleDeck::demo(&doc);
        let previous = run_drc(&doc, &rules);
        assert!(
            previous
                .iter()
                .any(|violation| violation.shape_ids == vec![first])
        );
        assert!(
            previous
                .iter()
                .any(|violation| violation.shape_ids == vec![second])
        );

        doc.apply_operation_without_log(&layout_model::Operation::DeleteShape { id: first });
        let incremental = run_drc_incremental(
            &doc,
            &rules,
            &previous,
            Rect::from_min_size(Point::new(0, 0), 200, 200),
        );

        assert!(
            !incremental
                .iter()
                .any(|violation| violation.shape_ids == vec![first])
        );
        assert!(
            incremental
                .iter()
                .any(|violation| violation.shape_ids == vec![second])
        );
    }

    #[test]
    fn invalid_technology_rule_reports_missing_layer() {
        let doc = Document::new("invalid tech");
        let mut technology = default_technology();
        technology.drc.min_width[0].layer = "missing_layer".to_string();

        let err = RuleDeck::from_technology(&doc, &technology).unwrap_err();

        assert!(err.to_string().contains("missing_layer"));
    }
}
