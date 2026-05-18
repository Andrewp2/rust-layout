#![allow(unused_imports)]
use super::*;
use crate::*;
use layout_model::{
    InstanceId, Operation, ProcessLayer, ShapeKind, TechnologyLayerRule, Transform,
    default_technology,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct DrcGoldenFixture {
    pub(crate) name: String,
    pub(crate) shapes: Vec<DrcFixtureShape>,
    pub(crate) expect: DrcGoldenExpectation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DrcGoldenExpectation {
    pub(crate) total_count: usize,
    pub(crate) omitted_count: usize,
    pub(crate) rule_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DrcFixtureShape {
    Rect {
        layer: String,
        x: Coord,
        y: Coord,
        w: Coord,
        h: Coord,
    },
}

pub(crate) fn document_from_drc_fixture(fixture: &DrcGoldenFixture) -> Document {
    let mut document = Document::new(&fixture.name);
    for shape in &fixture.shapes {
        match shape {
            DrcFixtureShape::Rect { layer, x, y, w, h } => {
                let layer_id = fixture_layer(&document, layer);
                document.insert_shape(
                    layer_id,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
                );
            }
        }
    }
    document
}

pub(crate) fn fixture_layer(document: &Document, layer: &str) -> LayerId {
    let process = ProcessLayer::from_technology_name(layer)
        .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
    document
        .layer_by_process(process)
        .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
}

#[test]
pub(crate) fn finds_min_width_violation() {
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
pub(crate) fn finds_max_width_violation() {
    let mut doc = Document::new("max width");
    let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape = doc.insert_shape(
        layer,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 200)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.max_width.insert(layer, 500);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "max_width")
        .expect("oversized shape width should be reported");

    assert_eq!(violation.shape_ids, vec![shape]);
    assert_eq!(violation.required, 500);
    assert_eq!(violation.actual, 800.0);
    assert!(violation.message.contains("maximum width"));
}

#[test]
pub(crate) fn finds_min_area_violation() {
    let mut doc = Document::new("min area");
    let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape = doc.insert_shape(
        layer,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 200, 200)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.min_area.insert(layer, 50_000);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "min_area")
        .expect("undersized shape area should be reported");

    assert_eq!(violation.shape_ids, vec![shape]);
    assert_eq!(violation.required, 50_000);
    assert_eq!(violation.actual, 40_000.0);
    assert!(violation.message.contains("minimum area"));
}

#[test]
pub(crate) fn finds_max_area_violation() {
    let mut doc = Document::new("max area");
    let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape = doc.insert_shape(
        layer,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 300, 300)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.max_area.insert(layer, 50_000);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "max_area")
        .expect("oversized shape area should be reported");

    assert_eq!(violation.shape_ids, vec![shape]);
    assert_eq!(violation.required, 50_000);
    assert_eq!(violation.actual, 90_000.0);
    assert!(violation.message.contains("maximum area"));
}

#[test]
pub(crate) fn derived_and_layer_min_area_reports_small_intersection() {
    let mut doc = Document::new("derived min area");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let metal_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    let poly_shape = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(80, 80), 100, 100)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "m1_poly_overlap".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    rules
        .derived_min_area
        .insert("m1_poly_overlap".to_string(), 500);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_area.m1_poly_overlap")
        .expect("small derived overlap should be reported");

    assert_eq!(violation.shape_ids, vec![metal_shape, poly_shape]);
    assert_eq!(violation.required, 500);
    assert_eq!(violation.actual, 400.0);
    assert!(violation.message.contains("derived layer"));
}

#[test]
pub(crate) fn derived_and_layer_max_area_reports_large_intersection() {
    let mut doc = Document::new("derived max area");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(25, 25), 100, 100)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "m1_poly_overlap".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    rules
        .derived_max_area
        .insert("m1_poly_overlap".to_string(), 5_000);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_max_area.m1_poly_overlap")
        .expect("large derived overlap should be reported");

    assert_eq!(violation.required, 5_000);
    assert_eq!(violation.actual, 5_625.0);
    assert!(violation.message.contains("maximum area"));
}

#[test]
pub(crate) fn derived_or_layer_min_area_reports_source_layer_shapes() {
    let mut doc = Document::new("derived or");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let small_metal = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
    );
    let large_poly = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 0), 100, 100)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "conductors".to_string(),
        operation: DerivedLayerOperation::Or,
        a: metal1,
        b: poly,
    });
    rules.derived_min_area.insert("conductors".to_string(), 500);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_area.conductors")
        .expect("small OR-derived source shape should be reported");

    assert_eq!(violation.shape_ids, vec![small_metal]);
    assert_eq!(violation.required, 500);
    assert_eq!(violation.actual, 400.0);
    assert!(
        !violation.shape_ids.contains(&large_poly),
        "large source shape should not be reported by the min-area rule"
    );
    assert_eq!(
        DrcIssueStore::from_violations(violations.clone()).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn derived_or_layer_deduplicates_same_source_layer() {
    let mut doc = Document::new("derived or same layer");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let first = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
    );
    let second = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 0), 20, 20)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "m1_copy".to_string(),
        operation: DerivedLayerOperation::Or,
        a: metal1,
        b: metal1,
    });
    rules.derived_min_area.insert("m1_copy".to_string(), 500);

    let violations = run_drc(&doc, &rules)
        .into_iter()
        .filter(|violation| violation.rule == "derived_min_area.m1_copy")
        .collect::<Vec<_>>();

    assert_eq!(violations.len(), 2);
    assert_eq!(violations[0].shape_ids, vec![first]);
    assert_eq!(violations[1].shape_ids, vec![second]);
}

#[test]
pub(crate) fn derived_not_layer_min_area_reports_remaining_fragment() {
    let mut doc = Document::new("derived not");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let metal_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    let poly_shape = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(20, 0), 100, 100)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "m1_not_poly".to_string(),
        operation: DerivedLayerOperation::Not,
        a: metal1,
        b: poly,
    });
    rules
        .derived_min_area
        .insert("m1_not_poly".to_string(), 2_500);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_area.m1_not_poly")
        .expect("small NOT-derived source fragment should be reported");

    assert_eq!(violation.shape_ids, vec![metal_shape, poly_shape]);
    assert_eq!(violation.required, 2_500);
    assert_eq!(violation.actual, 2_000.0);
    assert_eq!(
        violation.bounds,
        Rect::from_min_size(Point::new(-10, -10), 40, 120)
    );
    assert_eq!(
        DrcIssueStore::from_violations(violations.clone()).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn derived_layer_width_rules_measure_derived_intersection_bounds() {
    let mut doc = Document::new("derived width");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let metal_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 200, 200)),
    );
    let poly_shape = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(80, -20), 40, 240)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "gate".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    rules.derived_min_width.insert("gate".to_string(), 50);
    rules.derived_max_width.insert("gate".to_string(), 150);

    let violations = run_drc(&doc, &rules);
    let min_violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_width.gate")
        .expect("narrow derived intersection should be reported");
    let max_violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_max_width.gate")
        .expect("long derived intersection should be reported");

    assert_eq!(min_violation.shape_ids, vec![metal_shape, poly_shape]);
    assert_eq!(min_violation.required, 50);
    assert_eq!(min_violation.actual, 40.0);
    assert!(min_violation.message.contains("minimum width"));
    assert_eq!(max_violation.shape_ids, vec![metal_shape, poly_shape]);
    assert_eq!(max_violation.required, 150);
    assert_eq!(max_violation.actual, 200.0);
    assert!(max_violation.message.contains("maximum width"));
}

#[test]
pub(crate) fn derived_forbidden_overlap_reports_derived_layer_against_physical_layer() {
    let mut doc = Document::new("derived overlap");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let metal1_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    let poly_shape = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(20, -20), 40, 140)),
    );
    let metal2_shape = doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(50, 30), 80, 40)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "gate".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    rules
        .derived_forbidden_overlaps
        .push(DerivedForbiddenOverlapRule {
            derived: "gate".to_string(),
            layer: metal2,
            name: "derived_gate_overlap".to_string(),
        });

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_gate_overlap")
        .expect("derived gate overlap should be reported");

    assert_eq!(
        violation.shape_ids,
        vec![metal1_shape, poly_shape, metal2_shape]
    );
    assert_eq!(violation.required, 0);
    assert_eq!(violation.actual, 400.0);
    assert!(violation.message.contains("derived layer"));
    assert_eq!(
        DrcIssueStore::from_violations(violations.clone()).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn min_edge_spacing_finds_close_polygon_edges_with_overlapping_bounds() {
    let mut doc = Document::new("edge spacing");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let left = doc.insert_shape(
        metal1,
        ShapeKind::Polygon(geometry_core::Polygon::new(vec![
            Point::new(0, 0),
            Point::new(100, 0),
            Point::new(100, 40),
            Point::new(40, 40),
            Point::new(40, 100),
            Point::new(0, 100),
        ])),
    );
    let right = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(60, 60), 100, 100)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.min_edge_spacing.insert(metal1, 30);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "min_edge_spacing")
        .expect("close polygon/rectangle edge pair should be reported");

    assert_eq!(violation.shape_ids, vec![left, right]);
    assert_eq!(violation.required, 30);
    assert_eq!(violation.actual, 20.0);
    assert!(violation.message.contains("parallel edge spacing"));
    assert!(
        !violations
            .iter()
            .any(|violation| violation.rule == "min_spacing"),
        "bounds-based spacing should skip this overlapping-bounds fixture"
    );
}

#[test]
pub(crate) fn derived_min_spacing_reports_nearby_derived_intersections() {
    let mut doc = Document::new("derived spacing");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let first_metal = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    let first_poly = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(20, -20), 40, 140)),
    );
    let second_metal = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(90, 0), 100, 100)),
    );
    let second_poly = doc.insert_shape(
        poly,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(130, -20), 40, 140)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "gate".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    rules.derived_min_spacing.insert("gate".to_string(), 80);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_spacing.gate")
        .expect("nearby derived intersections should be reported");

    assert_eq!(
        violation.shape_ids,
        vec![first_metal, first_poly, second_metal, second_poly]
    );
    assert_eq!(violation.required, 80);
    assert_eq!(violation.actual, 70.0);
    assert!(violation.message.contains("derived layer"));
    assert_eq!(
        DrcIssueStore::from_violations(violations.clone()).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn derived_min_edge_spacing_reports_nearby_parallel_edges() {
    let mut doc = Document::new("derived edge spacing");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let left = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
    );
    let right = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(120, 20), 100, 60)),
    );
    let mut rules = RuleDeck::demo(&doc);
    rules.derived_layers.push(DerivedLayerRule {
        name: "conductors".to_string(),
        operation: DerivedLayerOperation::Or,
        a: metal1,
        b: poly,
    });
    rules
        .derived_min_edge_spacing
        .insert("conductors".to_string(), 30);

    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "derived_min_edge_spacing.conductors")
        .expect("nearby derived parallel edges should be reported");

    assert_eq!(violation.shape_ids, vec![left, right]);
    assert_eq!(violation.required, 30);
    assert_eq!(violation.actual, 20.0);
    assert!(violation.message.contains("parallel edge spacing"));
    assert_eq!(
        DrcIssueStore::from_violations(violations.clone()).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn drc_golden_fixture_reports_all_rule_families_once() {
    let fixture: DrcGoldenFixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/quality/drc_golden.json"
    )))
    .unwrap();
    let doc = document_from_drc_fixture(&fixture);

    let rules = RuleDeck::demo(&doc);
    assert_eq!(rules.validate_for_document(&doc), Vec::new());
    let violations = run_drc(&doc, &rules);
    let store = DrcIssueStore::from_violations(violations.clone());
    let summary = store.summary(16);
    let mut rule_counts = BTreeMap::new();
    for violation in violations {
        *rule_counts.entry(violation.rule).or_insert(0usize) += 1;
    }

    assert_eq!(store.validate(), Vec::new());
    assert_eq!(summary.total_count, fixture.expect.total_count);
    assert_eq!(summary.omitted_count, fixture.expect.omitted_count);
    assert_eq!(rule_counts, fixture.expect.rule_counts);
}

#[test]
pub(crate) fn calibre_rve_import_reads_rule_text_polygons_rects_and_edges() {
    let imported = import_calibre_rve_markers(
        "
M1_WIDTH
@ Minimum width failed
p 4 0 0 100 0 100 80 0 80
RULE M2_SPACE
r 200 300 260 340
EDGE_CHECK
e 400 500 450 500
unsupported long line with many words that should be skipped by the importer
",
    );

    assert_eq!(imported.report.marker_count, 3);
    assert_eq!(imported.report.skipped_line_count, 1);
    assert_eq!(imported.violations.len(), 3);
    assert_eq!(imported.violations[0].rule, "calibre.m1_width");
    assert_eq!(imported.violations[0].message, "Minimum width failed");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(0, 0), Point::new(100, 80))
    );
    assert_eq!(imported.violations[1].rule, "calibre.m2_space");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(200, 300), Point::new(260, 340))
    );
    assert_eq!(imported.violations[2].rule, "calibre.edge_check");
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(399, 499), Point::new(451, 501))
    );
    assert_eq!(
        DrcIssueStore::from_violations(imported.violations).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn technology_spacing_rules_change_drc_results() {
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
pub(crate) fn technology_edge_spacing_rules_change_drc_results() {
    let mut doc = Document::new("edge spacing tech");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Polygon(geometry_core::Polygon::new(vec![
            Point::new(0, 0),
            Point::new(100, 0),
            Point::new(100, 40),
            Point::new(40, 40),
            Point::new(40, 100),
            Point::new(0, 100),
        ])),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(60, 60), 100, 100)),
    );

    let default_rules = RuleDeck::from_technology(&doc, &default_technology()).unwrap();
    assert!(
        !run_drc(&doc, &default_rules)
            .iter()
            .any(|violation| violation.rule == "min_edge_spacing")
    );

    let mut technology = default_technology();
    technology.drc.min_edge_spacing.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 30,
    });
    let strict_rules = RuleDeck::from_technology(&doc, &technology).unwrap();
    assert_eq!(strict_rules.min_edge_spacing.get(&metal1), Some(&30));

    assert!(
        run_drc(&doc, &strict_rules)
            .iter()
            .any(|violation| violation.rule == "min_edge_spacing")
    );
}

#[test]
pub(crate) fn technology_max_width_rules_change_drc_results() {
    let mut doc = Document::new("max width tech");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 200)),
    );

    let default_rules = RuleDeck::from_technology(&doc, &default_technology()).unwrap();
    assert!(
        !run_drc(&doc, &default_rules)
            .iter()
            .any(|violation| violation.rule == "max_width")
    );

    let mut technology = default_technology();
    technology.drc.max_width.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 120,
    });
    let strict_rules = RuleDeck::from_technology(&doc, &technology).unwrap();

    assert_eq!(strict_rules.max_width.get(&metal1), Some(&120));
    assert!(
        run_drc(&doc, &strict_rules)
            .iter()
            .any(|violation| violation.rule == "max_width")
    );
}

#[test]
pub(crate) fn technology_area_rules_change_drc_results() {
    let mut doc = Document::new("area tech");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 200, 200)),
    );

    let default_rules = RuleDeck::from_technology(&doc, &default_technology()).unwrap();
    assert!(
        !run_drc(&doc, &default_rules)
            .iter()
            .any(|violation| violation.rule == "min_area")
    );

    let mut technology = default_technology();
    technology.drc.min_area.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 50_000,
    });
    let strict_rules = RuleDeck::from_technology(&doc, &technology).unwrap();

    assert_eq!(strict_rules.min_area.get(&metal1), Some(&50_000));
    assert!(
        run_drc(&doc, &strict_rules)
            .iter()
            .any(|violation| violation.rule == "min_area")
    );
}

#[test]
pub(crate) fn technology_max_area_rules_change_drc_results() {
    let mut doc = Document::new("max area tech");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 300, 300)),
    );

    let default_rules = RuleDeck::from_technology(&doc, &default_technology()).unwrap();
    assert!(
        !run_drc(&doc, &default_rules)
            .iter()
            .any(|violation| violation.rule == "max_area")
    );

    let mut technology = default_technology();
    technology.drc.max_area.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 50_000,
    });
    let strict_rules = RuleDeck::from_technology(&doc, &technology).unwrap();

    assert_eq!(strict_rules.max_area.get(&metal1), Some(&50_000));
    assert!(
        run_drc(&doc, &strict_rules)
            .iter()
            .any(|violation| violation.rule == "max_area")
    );
}

#[test]
pub(crate) fn drc_checks_hidden_layers_and_flattened_instances() {
    let mut doc = Document::new("hierarchical drc");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("unit");
    let child_shape = doc
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        )
        .unwrap();
    let instance = doc
        .insert_instance_in_top(child, Transform::translate(2_000, 3_000))
        .unwrap();
    doc.apply_operation_without_log(&Operation::SetLayerVisibility {
        layer: metal1,
        visible: false,
    });

    let rules = RuleDeck::demo(&doc);
    let violations = run_drc(&doc, &rules);
    let violation = violations
        .iter()
        .find(|violation| violation.rule == "min_width")
        .expect("flattened child-cell shape should be checked even when its layer is hidden");

    assert_eq!(violation.shape_ids, vec![child_shape]);
    assert_eq!(
        violation.occurrence_ids,
        vec![ShapeOccurrenceId::from_instance_path(
            child_shape,
            &[instance]
        )]
    );
    assert_eq!(
        violation.bounds,
        Rect::from_min_size(Point::new(2_000, 3_000), 100, 50).expanded(220)
    );
}

#[test]
pub(crate) fn incremental_drc_preserves_markers_outside_dirty_region() {
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
pub(crate) fn violation_stable_key_ignores_row_id_and_shape_order() {
    let mut first = DrcViolation {
        id: 1,
        rule: "min_spacing".to_string(),
        message: "spacing".to_string(),
        shape_ids: vec![ShapeId(7), ShapeId(3)],
        occurrence_ids: vec![],
        bounds: Rect::from_min_size(Point::new(10, 20), 30, 40),
        required: 200,
        actual: 120.0,
    };
    let mut second = first.clone();
    second.id = 99;
    second.shape_ids.reverse();

    assert_eq!(first.stable_key(), second.stable_key());

    first.actual = 121.0;
    assert_ne!(first.stable_key(), second.stable_key());
}

#[test]
pub(crate) fn violation_stable_key_distinguishes_repeated_instance_occurrences() {
    let first = DrcViolation {
        id: 1,
        rule: "min_width".to_string(),
        message: "too narrow".to_string(),
        shape_ids: vec![ShapeId(7)],
        occurrence_ids: vec![ShapeOccurrenceId::from_instance_path(
            ShapeId(7),
            &[InstanceId(1)],
        )],
        bounds: Rect::from_min_size(Point::new(0, 0), 100, 20),
        required: 200,
        actual: 20.0,
    };
    let mut second = first.clone();
    second.occurrence_ids = vec![ShapeOccurrenceId::from_instance_path(
        ShapeId(7),
        &[InstanceId(2)],
    )];

    assert_ne!(first.stable_key(), second.stable_key());
}

#[test]
pub(crate) fn issue_store_queries_by_stable_key_and_summarizes_capped_rows() {
    let first = DrcViolation {
        id: 1,
        rule: "min_width".to_string(),
        message: "too narrow".to_string(),
        shape_ids: vec![ShapeId(1)],
        occurrence_ids: vec![],
        bounds: Rect::from_min_size(Point::new(0, 0), 100, 20),
        required: 100,
        actual: 20.0,
    };
    let second = DrcViolation {
        id: 2,
        rule: "min_spacing".to_string(),
        message: "too close".to_string(),
        shape_ids: vec![ShapeId(2), ShapeId(3)],
        occurrence_ids: vec![],
        bounds: Rect::from_min_size(Point::new(200, 0), 300, 100),
        required: 200,
        actual: 90.0,
    };
    let third = DrcViolation {
        id: 3,
        rule: "min_spacing".to_string(),
        message: "too close".to_string(),
        shape_ids: vec![ShapeId(4), ShapeId(5)],
        occurrence_ids: vec![],
        bounds: Rect::from_min_size(Point::new(700, 0), 300, 100),
        required: 200,
        actual: 80.0,
    };
    let second_key = second.stable_key();
    let store = DrcIssueStore::from_violations(vec![first, second, third]);
    let summary = store.summary(2);

    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.displayed_count, 2);
    assert_eq!(summary.omitted_count, 1);
    assert_eq!(summary.error_count, 3);
    assert_eq!(summary.rule_counts.get("min_width"), Some(&1));
    assert_eq!(summary.rule_counts.get("min_spacing"), Some(&2));
    assert_eq!(store.displayed_records(2).len(), 2);
    assert_eq!(
        store.get(&second_key).map(|record| record.violation.id),
        Some(2)
    );
    assert_eq!(
        store.get(&second_key).map(|record| record.severity.label()),
        Some("error")
    );
}

#[test]
pub(crate) fn issue_store_validation_rejects_stale_duplicate_and_malformed_records() {
    let first = DrcViolation {
        id: 1,
        rule: "min_spacing".to_string(),
        message: "too close".to_string(),
        shape_ids: vec![ShapeId(7), ShapeId(3)],
        occurrence_ids: vec![],
        bounds: Rect::from_min_size(Point::new(10, 20), 30, 40),
        required: 200,
        actual: 120.0,
    };
    let mut duplicate = first.clone();
    duplicate.id = 2;
    duplicate.shape_ids.reverse();
    let malformed = DrcViolation {
        id: 0,
        rule: " ".to_string(),
        message: " ".to_string(),
        shape_ids: vec![ShapeId(0), ShapeId(0)],
        occurrence_ids: vec![],
        bounds: Rect {
            min: Point::new(50, 50),
            max: Point::new(10, 10),
        },
        required: -1,
        actual: f64::NAN,
    };
    let mut store = DrcIssueStore::from_violations(vec![first, duplicate, malformed]);
    store.records[2].key = "stale-drc-key".to_string();
    store.key_index.insert("orphan-drc-key".to_string(), 99);

    let findings = store.validate();
    let messages = findings
        .iter()
        .map(|finding| finding.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        findings
            .iter()
            .any(|finding| finding.severity == DrcValidationSeverity::Error)
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.severity == DrcValidationSeverity::Warning)
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("stable issue key") && message.contains("duplicated"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("does not match violation stable key"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing from the lookup index"))
    );
    assert!(messages.iter().any(|message| message.contains("stale key")));
    assert!(messages.iter().any(|message| message.contains("has id 0")));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("empty rule"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("invalid shape id 0"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("repeats shape id"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("inverted bounds"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("negative required"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("non-finite actual"))
    );
}

#[test]
pub(crate) fn rule_deck_validation_rejects_missing_layers_and_invalid_rules() {
    let doc = Document::new("bad drc rules");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let missing = LayerId(999);
    let rules = RuleDeck {
        grid: 0,
        derived_layers: vec![
            DerivedLayerRule {
                name: " ".to_string(),
                operation: DerivedLayerOperation::And,
                a: metal1,
                b: missing,
            },
            DerivedLayerRule {
                name: "gate".to_string(),
                operation: DerivedLayerOperation::And,
                a: metal1,
                b: missing,
            },
            DerivedLayerRule {
                name: "GATE".to_string(),
                operation: DerivedLayerOperation::And,
                a: metal1,
                b: missing,
            },
        ],
        derived_min_width: BTreeMap::from([("gate".to_string(), 0)]),
        derived_max_width: BTreeMap::from([("missing_derived".to_string(), 0)]),
        derived_min_area: BTreeMap::from([("gate".to_string(), 0)]),
        derived_max_area: BTreeMap::from([("missing_derived".to_string(), 0)]),
        derived_min_spacing: BTreeMap::from([("gate".to_string(), 0)]),
        derived_min_edge_spacing: BTreeMap::from([("missing_derived".to_string(), 0)]),
        min_width: BTreeMap::from([(missing, 0)]),
        max_width: BTreeMap::from([(missing, 0)]),
        min_area: BTreeMap::from([(missing, 0)]),
        max_area: BTreeMap::from([(missing, 0)]),
        min_spacing: BTreeMap::from([(metal1, -10)]),
        min_edge_spacing: BTreeMap::from([(missing, 0)]),
        via_enclosure: vec![
            EnclosureRule {
                via_layer: missing,
                enclosure_layer: metal1,
                required: 0,
            },
            EnclosureRule {
                via_layer: missing,
                enclosure_layer: metal1,
                required: 20,
            },
        ],
        forbidden_overlaps: vec![
            ForbiddenOverlapRule {
                a: metal1,
                b: missing,
                name: " ".to_string(),
            },
            ForbiddenOverlapRule {
                a: missing,
                b: metal1,
                name: " ".to_string(),
            },
        ],
        derived_forbidden_overlaps: vec![
            DerivedForbiddenOverlapRule {
                derived: "gate".to_string(),
                layer: missing,
                name: " ".to_string(),
            },
            DerivedForbiddenOverlapRule {
                derived: "GATE".to_string(),
                layer: missing,
                name: " ".to_string(),
            },
        ],
    };

    let findings = rules.validate_for_document(&doc);
    let messages = findings
        .iter()
        .map(|finding| finding.message.as_str())
        .collect::<Vec<_>>();

    assert!(messages.iter().any(|message| message.contains("grid")));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing document layer"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("min-width") && message.contains("must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("max-width") && message.contains("must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("min-area") && message.contains("must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("max-area") && message.contains("must be positive"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("min-spacing") && message.contains("must be positive"))
    );
    assert!(messages.iter().any(
        |message| message.contains("min-edge-spacing") && message.contains("must be positive")
    ));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("via-enclosure") && message.contains("duplicated"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("forbidden-overlap") && message.contains("duplicated"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("name cannot be empty"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("derived-overlap") && message.contains("duplicated"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("derived-layer") && message.contains("duplicated"))
    );
    assert!(messages.iter().any(
        |message| message.contains("derived-min-width") && message.contains("must be positive")
    ));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("derived-max-width")
                && message.contains("missing derived layer"))
    );
    assert!(messages.iter().any(
        |message| message.contains("derived-min-area") && message.contains("must be positive")
    ));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("derived-max-area")
                && message.contains("missing derived layer"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("derived-min-spacing")
                && message.contains("must be positive"))
    );
}

#[test]
pub(crate) fn invalid_technology_rule_reports_missing_layer() {
    let doc = Document::new("invalid tech");
    let mut technology = default_technology();
    technology.drc.min_width[0].layer = "missing_layer".to_string();

    let err = RuleDeck::from_technology(&doc, &technology).unwrap_err();

    assert!(err.to_string().contains("missing_layer"));
}
