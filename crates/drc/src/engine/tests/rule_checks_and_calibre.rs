#![allow(unused_imports)]
use super::*;
use crate::*;
use geometry_core::Vector;
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
pub(crate) fn calibre_rve_import_reads_rule_text_polygons_rects_edges_points_and_circles() {
    let imported = import_calibre_rve_markers(
        "
M1_WIDTH
@ Minimum width failed
@ foundry deck context
@ required=90nm actual=75dbu
@ owner = \"layout-team\"
@ note : 'reviewed externally'
@ approval_status = \"accepted\"
@ approved_by : \"layout-review\"
@ approval_note = \"accepted from imported approval deck\"
@ approval_role : 'layout'
@ approval_at = \"2026-05-19T12:15:00Z\"
@ tag \"category=Litho/Hotspots\"
@ tags severity=\"critical\", source='foundry, external'; review_bucket=\"A;B\"
@ tags defect_class: \"edge, jog\", review_stage: 'layout signoff'
@ tag review_lane layout
@ tags inspection_phase final; disposition accepted
@ classification: litho hotspot
@ tool_version Calibre 2026.1
p 4 0 0 100 0 100 80 0 80
RULE M2_SPACE
r 0.2um 300nm 0.26um 340nm
EDGE_CHECK
e 400 500 450 500
POINT_CHECK
pt 700dbu 0.8um
CIRCLE_CHECK
circle 1um 1100dbu 25nm
RuleCheck: VIA1 SPACE
Cell = TOP_CELL
Cell Name: TOP_CELL_ALIAS
Source Cell = VIA_SOURCE
Layer : VIA1
Layer Name: VIA1_DRAWING
Source Layer = VIA1_PIN
Datatype = 5
Message: Via spacing from RVE
Rule Description: foundry context line
Required = 0.15 um Actual : 120 nm
r 10 20 30 40
MULTILINE_POLYGON
p 4
0 0
100 0
100 80
0 80
KEYED_RECT
rect y2=2200nm x2=1100nm y1=2um x1=1um
KEYED_SIZE_RECT
rectangle height: 50 nm x 3 um width: 250 nm y = 4 um
KEYED_CIRCLE
circle radius=25nm cy=1100dbu cx=1um
SEPARATED_UNIT_RECT
r 1 um 2 um 1100 nm 2200 nm
Rule Check Name: RVE DELIMITER
Result 1 of 1
r 50 60 70 80
INDEXED_MULTILINE_POLYGON
p 4
1: 10 10
2: 110 10
3: 110 90
4: 10 90
COLON_GEOMETRY_LABELS
bbox: 5 6 25 36
POLY_ALIAS
poly 3 10 10 20 10 20 30
PLURAL_POINT_ALIAS
points: 123 456
CIRCLE_SHORT_ALIAS
circ: 1um 2um 50nm
INDEXED_RECT_ROWS
rect
1: 200 210
2: 260 270
INDEXED_POINT_ROW
point
1: 300 310
INDEXED_CIRCLE_ROW
circle
1: 1000 2000 25
WHITESPACE_INDEXED_POLYGON
p 4
1 20 20
2 120 20
3 120 100
4 20 100
WHITESPACE_INDEXED_RECT_ROWS
rect
1 700 710
2 760 770
WHITESPACE_INDEXED_CIRCLE_ROW
circle
1 1500 2500 25
RuleCheck: \"QUOTED LONG RULE NAME WITH MANY EXTRA TOKENS\"
r 400 410 430 450
INDEXED_INLINE_RECT
1: rect 500 510 560 570
INDEXED_INLINE_EDGE
2. edge 600 610 640 610
EQUALS_ATTACHED_GEOMETRY_LABEL
bbox=800 810 860 870
COLON_ATTACHED_POINT_LABEL
points:900 910
KEYED_CORNER_PAIR_RECT
rect ll=(2um, 3um) ur=(2.1um, 3.2um)
CENTER_PAIR_CIRCLE
circle center=(1um, 1100dbu) r=25nm
KEYED_VERTEX_PAIR_POLYGON
poly vertex0=(10,10) vertex1=(20,10) vertex2=(20,30)
DIAMETER_CIRCLE
circle center=(2um, 3um) diameter=100nm
UNCOUNTED_MULTILINE_POLYGON
polygon
30 30
130 30
130 110
30 110
CENTER_SIZE_RECT
rect center=(10um,20um) width=100nm height=200nm
BROKEN_POLYGON
p 4
0 0
100 0
unsupported long line with many words that should be skipped by the importer
",
    );

    assert_eq!(imported.report.marker_count, 34);
    assert_eq!(imported.report.marker_state_count, 2);
    assert_eq!(imported.report.skipped_line_count, 2);
    assert!(
        imported
            .report
            .warnings
            .iter()
            .any(|warning| warning.contains("incomplete polygon marker geometry"))
    );
    assert_eq!(imported.violations.len(), 34);
    assert_eq!(imported.violations[0].rule, "calibre.m1_width");
    assert_eq!(
        imported.violations[0].message,
        "Minimum width failed\nfoundry deck context"
    );
    let first_state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("Calibre/RVE review metadata should attach to the marker");
    assert_eq!(first_state.owner.as_deref(), Some("layout-team"));
    assert_eq!(first_state.note.as_deref(), Some("reviewed externally"));
    assert_eq!(first_state.signoff.as_deref(), Some("accepted"));
    assert_eq!(first_state.signoff_by.as_deref(), Some("layout-review"));
    assert_eq!(
        first_state.signoff_note.as_deref(),
        Some("accepted from imported approval deck")
    );
    let first_signoff = first_state.signoff_records.get("layout-review").unwrap();
    assert_eq!(first_signoff.status, "accepted");
    assert_eq!(first_signoff.role.as_deref(), Some("layout"));
    assert_eq!(first_signoff.by.as_deref(), Some("layout-review"));
    assert_eq!(
        first_signoff.note.as_deref(),
        Some("accepted from imported approval deck")
    );
    assert_eq!(
        first_signoff.recorded_at.as_deref(),
        Some("2026-05-19T12:15:00Z")
    );
    assert_eq!(
        first_state.tags.get("category").map(String::as_str),
        Some("Litho/Hotspots")
    );
    assert_eq!(
        first_state.tags.get("severity").map(String::as_str),
        Some("critical")
    );
    assert_eq!(
        first_state.tags.get("source").map(String::as_str),
        Some("foundry, external")
    );
    assert_eq!(
        first_state.tags.get("review_bucket").map(String::as_str),
        Some("A;B")
    );
    assert_eq!(
        first_state.tags.get("defect_class").map(String::as_str),
        Some("edge, jog")
    );
    assert_eq!(
        first_state.tags.get("review_stage").map(String::as_str),
        Some("layout signoff")
    );
    assert_eq!(
        first_state.tags.get("review_lane").map(String::as_str),
        Some("layout")
    );
    assert_eq!(
        first_state.tags.get("inspection_phase").map(String::as_str),
        Some("final")
    );
    assert_eq!(
        first_state.tags.get("disposition").map(String::as_str),
        Some("accepted")
    );
    assert_eq!(
        first_state.tags.get("classification").map(String::as_str),
        Some("litho hotspot")
    );
    assert_eq!(
        first_state.tags.get("tool_version").map(String::as_str),
        Some("Calibre 2026.1")
    );
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(0, 0), Point::new(100, 80))
    );
    assert_eq!(imported.violations[0].required, 90);
    assert_eq!(imported.violations[0].actual, 75.0);
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
    assert_eq!(imported.violations[3].rule, "calibre.point_check");
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(699, 799), Point::new(701, 801))
    );
    assert_eq!(imported.violations[4].rule, "calibre.circle_check");
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(975, 1075), Point::new(1025, 1125))
    );
    assert_eq!(imported.violations[5].rule, "calibre.via1_space");
    assert_eq!(
        imported.violations[5].message,
        "Via spacing from RVE\nfoundry context line"
    );
    assert_eq!(
        imported.violations[5].bounds,
        Rect::new(Point::new(10, 20), Point::new(30, 40))
    );
    assert_eq!(imported.violations[5].required, 150);
    assert_eq!(imported.violations[5].actual, 120.0);
    let rve_context_state = imported
        .marker_states
        .get(&imported.violations[5].stable_key())
        .expect("Calibre/RVE labeled context should attach searchable marker tags");
    assert_eq!(
        rve_context_state.tags.get("cell").map(String::as_str),
        Some("TOP_CELL")
    );
    assert_eq!(
        rve_context_state.tags.get("cell_name").map(String::as_str),
        Some("TOP_CELL_ALIAS")
    );
    assert_eq!(
        rve_context_state
            .tags
            .get("source_cell")
            .map(String::as_str),
        Some("VIA_SOURCE")
    );
    assert_eq!(
        rve_context_state.tags.get("layer").map(String::as_str),
        Some("VIA1")
    );
    assert_eq!(
        rve_context_state.tags.get("layer_name").map(String::as_str),
        Some("VIA1_DRAWING")
    );
    assert_eq!(
        rve_context_state
            .tags
            .get("source_layer")
            .map(String::as_str),
        Some("VIA1_PIN")
    );
    assert_eq!(
        rve_context_state.tags.get("datatype").map(String::as_str),
        Some("5")
    );
    assert_eq!(imported.violations[6].rule, "calibre.multiline_polygon");
    assert_eq!(
        imported.violations[6].bounds,
        Rect::new(Point::new(0, 0), Point::new(100, 80))
    );
    assert_eq!(imported.violations[7].rule, "calibre.keyed_rect");
    assert_eq!(
        imported.violations[7].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2200))
    );
    assert_eq!(imported.violations[8].rule, "calibre.keyed_size_rect");
    assert_eq!(
        imported.violations[8].bounds,
        Rect::new(Point::new(3000, 4000), Point::new(3250, 4050))
    );
    assert_eq!(imported.violations[9].rule, "calibre.keyed_circle");
    assert_eq!(
        imported.violations[9].bounds,
        Rect::new(Point::new(975, 1075), Point::new(1025, 1125))
    );
    assert_eq!(imported.violations[10].rule, "calibre.separated_unit_rect");
    assert_eq!(
        imported.violations[10].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2200))
    );
    assert_eq!(imported.violations[11].rule, "calibre.rve_delimiter");
    assert_eq!(
        imported.violations[11].bounds,
        Rect::new(Point::new(50, 60), Point::new(70, 80))
    );
    assert_eq!(
        imported.violations[12].rule,
        "calibre.indexed_multiline_polygon"
    );
    assert_eq!(
        imported.violations[12].bounds,
        Rect::new(Point::new(10, 10), Point::new(110, 90))
    );
    assert_eq!(
        imported.violations[13].rule,
        "calibre.colon_geometry_labels"
    );
    assert_eq!(
        imported.violations[13].bounds,
        Rect::new(Point::new(5, 6), Point::new(25, 36))
    );
    assert_eq!(imported.violations[14].rule, "calibre.poly_alias");
    assert_eq!(
        imported.violations[14].bounds,
        Rect::new(Point::new(10, 10), Point::new(20, 30))
    );
    assert_eq!(imported.violations[15].rule, "calibre.plural_point_alias");
    assert_eq!(
        imported.violations[15].bounds,
        Rect::new(Point::new(122, 455), Point::new(124, 457))
    );
    assert_eq!(imported.violations[16].rule, "calibre.circle_short_alias");
    assert_eq!(
        imported.violations[16].bounds,
        Rect::new(Point::new(950, 1950), Point::new(1050, 2050))
    );
    assert_eq!(imported.violations[17].rule, "calibre.indexed_rect_rows");
    assert_eq!(
        imported.violations[17].bounds,
        Rect::new(Point::new(200, 210), Point::new(260, 270))
    );
    assert_eq!(imported.violations[18].rule, "calibre.indexed_point_row");
    assert_eq!(
        imported.violations[18].bounds,
        Rect::new(Point::new(299, 309), Point::new(301, 311))
    );
    assert_eq!(imported.violations[19].rule, "calibre.indexed_circle_row");
    assert_eq!(
        imported.violations[19].bounds,
        Rect::new(Point::new(975, 1975), Point::new(1025, 2025))
    );
    assert_eq!(
        imported.violations[20].rule,
        "calibre.whitespace_indexed_polygon"
    );
    assert_eq!(
        imported.violations[20].bounds,
        Rect::new(Point::new(20, 20), Point::new(120, 100))
    );
    assert_eq!(
        imported.violations[21].rule,
        "calibre.whitespace_indexed_rect_rows"
    );
    assert_eq!(
        imported.violations[21].bounds,
        Rect::new(Point::new(700, 710), Point::new(760, 770))
    );
    assert_eq!(
        imported.violations[22].rule,
        "calibre.whitespace_indexed_circle_row"
    );
    assert_eq!(
        imported.violations[22].bounds,
        Rect::new(Point::new(1475, 2475), Point::new(1525, 2525))
    );
    assert_eq!(
        imported.violations[23].rule,
        "calibre.quoted_long_rule_name_with_many_extra_tokens"
    );
    assert_eq!(
        imported.violations[23].message,
        "Imported Calibre/RVE marker for QUOTED LONG RULE NAME WITH MANY EXTRA TOKENS"
    );
    assert_eq!(
        imported.violations[23].bounds,
        Rect::new(Point::new(400, 410), Point::new(430, 450))
    );
    assert_eq!(imported.violations[24].rule, "calibre.indexed_inline_rect");
    assert_eq!(
        imported.violations[24].bounds,
        Rect::new(Point::new(500, 510), Point::new(560, 570))
    );
    assert_eq!(imported.violations[25].rule, "calibre.indexed_inline_edge");
    assert_eq!(
        imported.violations[25].bounds,
        Rect::new(Point::new(599, 609), Point::new(641, 611))
    );
    assert_eq!(
        imported.violations[26].rule,
        "calibre.equals_attached_geometry_label"
    );
    assert_eq!(
        imported.violations[26].bounds,
        Rect::new(Point::new(800, 810), Point::new(860, 870))
    );
    assert_eq!(
        imported.violations[27].rule,
        "calibre.colon_attached_point_label"
    );
    assert_eq!(
        imported.violations[27].bounds,
        Rect::new(Point::new(899, 909), Point::new(901, 911))
    );
    assert_eq!(
        imported.violations[28].rule,
        "calibre.keyed_corner_pair_rect"
    );
    assert_eq!(
        imported.violations[28].bounds,
        Rect::new(Point::new(2000, 3000), Point::new(2100, 3200))
    );
    assert_eq!(imported.violations[29].rule, "calibre.center_pair_circle");
    assert_eq!(
        imported.violations[29].bounds,
        Rect::new(Point::new(975, 1075), Point::new(1025, 1125))
    );
    assert_eq!(
        imported.violations[30].rule,
        "calibre.keyed_vertex_pair_polygon"
    );
    assert_eq!(
        imported.violations[30].bounds,
        Rect::new(Point::new(10, 10), Point::new(20, 30))
    );
    assert_eq!(imported.violations[31].rule, "calibre.diameter_circle");
    assert_eq!(
        imported.violations[31].bounds,
        Rect::new(Point::new(1950, 2950), Point::new(2050, 3050))
    );
    assert_eq!(
        imported.violations[32].rule,
        "calibre.uncounted_multiline_polygon"
    );
    assert_eq!(
        imported.violations[32].bounds,
        Rect::new(Point::new(30, 30), Point::new(130, 110))
    );
    assert_eq!(imported.violations[33].rule, "calibre.center_size_rect");
    assert_eq!(
        imported.violations[33].bounds,
        Rect::new(Point::new(9950, 19900), Point::new(10050, 20100))
    );
    assert_eq!(
        DrcIssueStore::from_violations(imported.violations).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_alternate_keyed_geometry_tuples() {
    let imported = import_calibre_rve_markers(
        "
ALT_CORNER_PAIR_RECT
rect ul=(2um, 3.2um) lr=(2.1um, 3um)
EDGE_ENDPOINT_PAIR
edge start=(50, 60) end=(80, 60)
POINT_LOCATION_PAIR
point loc=(4um, 5um)
CIRCLE_POSITION_PAIR
circle position=(6um, 7um) rad=25nm
CENTRE_SIZE_RECT
rect xc=10um yc=20um size_x=100nm size_y=200nm
",
    );

    assert_eq!(imported.report.marker_count, 5);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 5);
    assert_eq!(imported.violations[0].rule, "calibre.alt_corner_pair_rect");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(2000, 3000), Point::new(2100, 3200))
    );
    assert_eq!(imported.violations[1].rule, "calibre.edge_endpoint_pair");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(49, 59), Point::new(81, 61))
    );
    assert_eq!(imported.violations[2].rule, "calibre.point_location_pair");
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(3999, 4999), Point::new(4001, 5001))
    );
    assert_eq!(imported.violations[3].rule, "calibre.circle_position_pair");
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(5975, 6975), Point::new(6025, 7025))
    );
    assert_eq!(imported.violations[4].rule, "calibre.centre_size_rect");
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(9950, 19900), Point::new(10050, 20100))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_tuple_origin_size_and_min_max_rects() {
    let imported = import_calibre_rve_markers(
        "
ORIGIN_SIZE_TUPLE_RECT
rect origin=(1um,2um) size=(100nm,200nm)
CENTER_SIZE_TUPLE_RECT
rect center=(10um,20um) size=(100nm,200nm)
MIN_MAX_TUPLE_RECT
rect min=(3000,4000) max=(3250,4050)
LOWER_UPPER_TUPLE_RECT
rect lower=(5um,6um) upper=(5.2um,6.3um)
ORIGIN_SCALAR_SIZE_RECT
rect origin=(7um,8um) size_x=100nm size_y=200nm
",
    );

    assert_eq!(imported.report.marker_count, 5);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 5);
    assert_eq!(
        imported.violations[0].rule,
        "calibre.origin_size_tuple_rect"
    );
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2200))
    );
    assert_eq!(
        imported.violations[1].rule,
        "calibre.center_size_tuple_rect"
    );
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(9950, 19900), Point::new(10050, 20100))
    );
    assert_eq!(imported.violations[2].rule, "calibre.min_max_tuple_rect");
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(3000, 4000), Point::new(3250, 4050))
    );
    assert_eq!(
        imported.violations[3].rule,
        "calibre.lower_upper_tuple_rect"
    );
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(5000, 6000), Point::new(5200, 6300))
    );
    assert_eq!(
        imported.violations[4].rule,
        "calibre.origin_scalar_size_rect"
    );
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(7000, 8000), Point::new(7100, 8200))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_bracketed_indexed_keyed_coordinates() {
    let imported = import_calibre_rve_markers(
        "
BRACKETED_CORNER_RECT
rect ll=[2um,3um] ur=[2.1um,3.2um]
BRACKETED_AXIS_POLYGON
poly x[0]=10 y[0]=10 x[1]=40 y[1]=10 x[2]=40 y[2]=30 x[3]=10 y[3]=30
UNDERSCORE_AXIS_POLYGON
poly x_1=50 y_1=50 x_2=80 y_2=50 x_3=80 y_3=90
BRACKETED_VERTEX_POLYGON
poly vertex[1]=(100,100) vertex[2]=(130,100) vertex[3]=(130,140)
BRACKETED_CENTER_CIRCLE
circle center=[1um,1100dbu] radius=25nm
",
    );

    assert_eq!(imported.report.marker_count, 5);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 5);
    assert_eq!(imported.violations[0].rule, "calibre.bracketed_corner_rect");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(2000, 3000), Point::new(2100, 3200))
    );
    assert_eq!(
        imported.violations[1].rule,
        "calibre.bracketed_axis_polygon"
    );
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(10, 10), Point::new(40, 30))
    );
    assert_eq!(
        imported.violations[2].rule,
        "calibre.underscore_axis_polygon"
    );
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(50, 50), Point::new(80, 90))
    );
    assert_eq!(
        imported.violations[3].rule,
        "calibre.bracketed_vertex_polygon"
    );
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(100, 100), Point::new(130, 140))
    );
    assert_eq!(
        imported.violations[4].rule,
        "calibre.bracketed_center_circle"
    );
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(975, 1075), Point::new(1025, 1125))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_keyed_geometry_kind_rows() {
    let imported = import_calibre_rve_markers(
        "
KEYED_KIND_RECT
kind=rect x1=10 y1=20 x2=30 y2=40
KEYED_TYPE_POLYGON
geometry_type: polygon x[0]=10 y[0]=10 x[1]=40 y[1]=10 x[2]=40 y[2]=30
KEYED_SHAPE_POINT
shape = point loc=(1um,2um)
INDEXED_KEYED_KIND_EDGE
1: type=edge start=(50,60) end=(80,60)
PENDING_KEYED_KIND_RECT
marker_type: rectangle
1: 100 110
2: 130 150
",
    );

    assert_eq!(imported.report.marker_count, 5);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 5);
    assert_eq!(imported.violations[0].rule, "calibre.keyed_kind_rect");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(10, 20), Point::new(30, 40))
    );
    assert_eq!(imported.violations[1].rule, "calibre.keyed_type_polygon");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(10, 10), Point::new(40, 30))
    );
    assert_eq!(imported.violations[2].rule, "calibre.keyed_shape_point");
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(999, 1999), Point::new(1001, 2001))
    );
    assert_eq!(
        imported.violations[3].rule,
        "calibre.indexed_keyed_kind_edge"
    );
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(49, 59), Point::new(81, 61))
    );
    assert_eq!(
        imported.violations[4].rule,
        "calibre.pending_keyed_kind_rect"
    );
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(100, 110), Point::new(130, 150))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_labeled_multiline_coordinate_rows() {
    let imported = import_calibre_rve_markers(
        "
LABELED_RECT_ROWS
rect
coordinate: 200 210
coordinate: 260 270
LABELED_POLYGON_ROWS
polygon
vertex 1: 10 10
vertex 2: 40 10
point[3]: 40 30
coordinates: 10 30
LABELED_EDGE_ROWS
edge
start: 50 60
end: 80 60
LABELED_POINT_ROW
point
location: 1um 2um
LABELED_CIRCLE_ROW
circle
center: 3um 4um 25nm
",
    );

    assert_eq!(imported.report.marker_count, 5);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 5);
    assert_eq!(imported.violations[0].rule, "calibre.labeled_rect_rows");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(200, 210), Point::new(260, 270))
    );
    assert_eq!(imported.violations[1].rule, "calibre.labeled_polygon_rows");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(10, 10), Point::new(40, 30))
    );
    assert_eq!(imported.violations[2].rule, "calibre.labeled_edge_rows");
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(49, 59), Point::new(81, 61))
    );
    assert_eq!(imported.violations[3].rule, "calibre.labeled_point_row");
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(999, 1999), Point::new(1001, 2001))
    );
    assert_eq!(imported.violations[4].rule, "calibre.labeled_circle_row");
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(2975, 3975), Point::new(3025, 4025))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_pending_keyed_coordinate_rows() {
    let imported = import_calibre_rve_markers(
        "
KEYED_PENDING_POLYGON_ROWS
polygon
x 10 y 10
x: 40 y: 10
x=40 y=30
x 10 y 30
KEYED_PENDING_RECT_ROWS
rect
x: 200 y: 210
x=260 y=270
KEYED_PENDING_CIRCLE_ROW
circle
cx 3000 cy 4000 radius 25nm
",
    );

    assert_eq!(imported.report.marker_count, 3);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 3);
    assert_eq!(
        imported.violations[0].rule,
        "calibre.keyed_pending_polygon_rows"
    );
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(10, 10), Point::new(40, 30))
    );
    assert_eq!(
        imported.violations[1].rule,
        "calibre.keyed_pending_rect_rows"
    );
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(200, 210), Point::new(260, 270))
    );
    assert_eq!(
        imported.violations[2].rule,
        "calibre.keyed_pending_circle_row"
    );
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(2975, 3975), Point::new(3025, 4025))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_long_unit_aliases_and_scientific_notation() {
    let imported = import_calibre_rve_markers(
        "
LONG_UNIT_RECT
Required = 1.5e-1 micrometer Actual : 1.2e2 nanometres
rect 1e0micrometer 2e0micrometres 1.1e-3millimeter 2200nanometres
LONG_UNIT_CIRCLE
circle center=(2micrometers, 3micrometre) diameter=1e2nanometers
SEPARATED_LONG_UNIT_RECT
rectangle x 0.001 millimeter y 0.002 millimetres width 100 nanometre height 2e-1 micrometer
",
    );

    assert_eq!(imported.report.marker_count, 3);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 3);
    assert_eq!(imported.violations[0].rule, "calibre.long_unit_rect");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2200))
    );
    assert_eq!(imported.violations[0].required, 150);
    assert_eq!(imported.violations[0].actual, 120.0);
    assert_eq!(imported.violations[1].rule, "calibre.long_unit_circle");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(1950, 2950), Point::new(2050, 3050))
    );
    assert_eq!(
        imported.violations[2].rule,
        "calibre.separated_long_unit_rect"
    );
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2200))
    );
}

#[test]
pub(crate) fn calibre_rve_import_reads_ascii_database_signatures_and_cn_properties() {
    let imported = import_calibre_rve_markers(
        "
TOP 1000
ASCII_POLYGON_RULE
2 2 1 May 12 08:32:00 2003
@ Polygon check text
p 1 4
CN CHILD c 1 0 0 1 1000 2000
DV 520
DA M1M 0
0 0
100 0
100 80
0 80
ASCII_EDGE_CLUSTER
1 1 0 May 12 08:33:00 2003
e 1 2
0 0 50 0
100 100 100 150
",
    );

    assert_eq!(imported.report.marker_count, 2);
    assert_eq!(imported.report.marker_state_count, 1);
    assert_eq!(imported.report.skipped_line_count, 0);
    assert!(imported.report.warnings.is_empty());
    assert_eq!(imported.violations.len(), 2);
    assert_eq!(imported.violations[0].rule, "calibre.ascii_polygon_rule");
    assert_eq!(imported.violations[0].message, "Polygon check text");
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1100, 2080))
    );
    let polygon_state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("CN and ASCII result properties should attach to the polygon marker");
    assert_eq!(
        polygon_state.tags.get("source_cell").map(String::as_str),
        Some("CHILD")
    );
    assert_eq!(
        polygon_state
            .tags
            .get("calibre_cn_cell_space")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        polygon_state
            .tags
            .get("calibre_cn_transform")
            .map(String::as_str),
        Some("1 0 0 1 1000 2000")
    );
    assert_eq!(
        polygon_state
            .tags
            .get("calibre_property_dv")
            .map(String::as_str),
        Some("520")
    );
    assert_eq!(
        polygon_state
            .tags
            .get("calibre_property_da")
            .map(String::as_str),
        Some("M1M 0")
    );
    assert_eq!(imported.violations[1].rule, "calibre.ascii_edge_cluster");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(-1, -1), Point::new(101, 151))
    );
}

#[test]
pub(crate) fn klayout_rdb_import_reads_xml_report_database_geometry_and_states() {
    let imported = import_klayout_rdb_markers(
        r#"
<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description>External signoff markers</description>
 <original-file>/tmp/source-run.lyrdb</original-file>
 <generator>KLayout DRC runset 2026.05</generator>
 <top-cell>TOP</top-cell>
 <tags>
  <tag>
   <name>important</name>
   <description>KLayout important review marker</description>
  </tag>
  <tag>
   <name>custom-tag</name>
   <description>Custom KLayout review tag</description>
  </tag>
  <tag>
   <name>review&apos;s item tag</name>
   <description>Quoted KLayout item tag</description>
  </tag>
  <tag>
   <name>measured-width</name>
   <description>Measured width scalar</description>
  </tag>
  <tag>
   <name>review-note</name>
   <description>Tagged text scalar</description>
  </tag>
  <tag>
   <name>geometry-review</name>
   <description>Tagged box geometry</description>
  </tag>
  <tag>
   <name>raw-review</name>
   <description>Tagged raw value</description>
  </tag>
  <tag>
   <name>unused-tag</name>
   <description>Unused declared KLayout tag</description>
  </tag>
 </tags>
 <categories>
  <category>
   <name>M1-Metal</name>
   <description>Metal 1 checks</description>
   <categories>
    <category>
     <name>Spacing</name>
     <description>Spacing from KLayout deck</description>
    </category>
   </categories>
  </category>
  <category>
   <name>Unused</name>
   <description>Unused KLayout category declaration</description>
  </category>
 </categories>
 <cells>
  <cell>
   <name>TOP</name>
   <variant>1</variant>
   <layout-name>TOP$1</layout-name>
   <references>
    <ref>
     <parent>ROOT</parent>
     <trans>r90 *1 17.5,-25</trans>
    </ref>
   </references>
  </cell>
  <cell>
   <name>CHILD</name>
   <layout-name>CHILD_LAYOUT</layout-name>
   <references>
    <ref>
     <parent>TOP</parent>
     <trans>r90 *1 17.5,-25</trans>
    </ref>
    <reference>
     <parent>TOP</parent>
     <trans>*2 30,40</trans>
    </reference>
    <reference>
     <parent>TOP</parent>
     <trans>r45</trans>
    </reference>
   </references>
  </cell>
  <cell>
   <name>VARIANT_CHILD</name>
   <variant>2</variant>
   <layout-name>VARIANT_CHILD_LAYOUT</layout-name>
   <references>
    <reference>
     <parent>TOP:1</parent>
     <trans>r0 4,5</trans>
    </reference>
   </references>
  </cell>
  <cell>
   <name>UNUSED</name>
   <variant>7</variant>
   <layout-name>UNUSED_LAYOUT</layout-name>
   <references>
    <reference>
     <parent>TOP</parent>
     <trans>r0 1,2</trans>
    </reference>
   </references>
  </cell>
 </cells>
 <items>
  <item>
   <tags>important, waived, 'custom-tag', 'review\'s item tag'</tags>
   <category>'M1-Metal'.Spacing</category>
   <cell>CHILD</cell>
   <visited>true</visited>
   <multiplicity>7</multiplicity>
   <comment>reviewed externally</comment>
   <image>iVBORw0KGgo=</image>
   <values>
    <value>text: "spacing too small"</value>
    <value>[#'geometry-review'] box: (1.0,2.0;1.2,2.3)</value>
    <value>[#'review-reference'] reference: TOP/r0/7.0/8.0</value>
    <value>string: "scalar review lane"</value>
    <value>float: 1.25</value>
    <value>[#'measured-width'] float: 0.42</value>
    <value>[#'review-note'] text: 'tagged scalar note'</value>
    <value>text: ' leading scalar '</value>
    <value>text: ''</value>
    <value>text: '   '</value>
    <value>[#'raw-review'] unknown: preserved raw payload</value>
   </values>
  </item>
  <item>
   <category>Via.Point</category>
   <values>
    <value>point: (7.0,8.0)</value>
   </values>
  </item>
  <item>
   <category>Space.EdgePair</category>
   <values>
    <value>edge-pair: (0.0,0.0;1.0,0.0)/(0.0,0.2;1.0,0.2)</value>
   </values>
  </item>
  <item>
   <category>Poly.Edge</category>
   <values>
    <value>edge: (-1.0,0.0;1.0,0.0)</value>
   </values>
  </item>
  <item>
   <category>Diff.Polygon</category>
   <values>
    <value>polygon: (0,0;2,0;2,2;0,2/0.5,0.5;1.0,0.5;1.0,1.0)</value>
   </values>
  </item>
  <item>
   <category>Routing.Path</category>
   <values>
    <value>path: (0,0;1,0;1,1) w=0.2 bx=0.4 ex=0.3 r=false</value>
   </values>
  </item>
  <item>
   <category>Labels.Pin</category>
   <values>
    <value>label: ('pin text',r0 3.5,4.5)</value>
   </values>
  </item>
  <item>
   <category>Texts.DText</category>
   <values>
    <value>text: ('text object',r90 4.0,5.0)</value>
   </values>
  </item>
  <item>
   <category>Variant.Child</category>
   <cell>VARIANT_CHILD</cell>
   <values>
    <value>box: (0,0;0.2,0.3)</value>
   </values>
  </item>
  <item>
   <category>Warnings.TextOnly</category>
   <values>
    <value>text: "database-level warning"</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB XML should import");

    assert_eq!(imported.report.marker_count, 11);
    assert_eq!(imported.report.marker_state_count, 11);
    assert_eq!(imported.report.skipped_item_count, 0);
    assert_eq!(imported.findings.len(), 1);
    assert_eq!(
        imported.findings[0].severity,
        DrcValidationSeverity::Warning
    );
    assert!(
        imported.findings[0]
            .message
            .contains("database-level warning"),
        "{:?}",
        imported.findings
    );
    assert_eq!(
        imported.report.description.as_deref(),
        Some("External signoff markers")
    );
    assert_eq!(
        imported.report.original_file.as_deref(),
        Some("/tmp/source-run.lyrdb")
    );
    assert_eq!(
        imported.report.generator.as_deref(),
        Some("KLayout DRC runset 2026.05")
    );
    assert_eq!(imported.report.top_cell.as_deref(), Some("TOP"));
    assert_eq!(imported.report.warnings.len(), 1);
    assert!(
        imported.report.warnings[0].contains("unknown: preserved raw payload"),
        "{:?}",
        imported.report.warnings
    );
    assert!(
        imported
            .report
            .warnings
            .iter()
            .all(|warning| !warning.contains("reference: TOP/r0/7.0/8.0")),
        "{:?}",
        imported.report.warnings
    );
    assert_eq!(imported.violations.len(), 11);
    assert_eq!(imported.violations[0].rule, "klayout.m1_metal_spacing");
    assert_eq!(
        imported.violations[0].message,
        "spacing too small\nleading scalar\nreviewed externally"
    );
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(15200, -24000), Point::new(15500, -23800))
    );
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(32000, 44000), Point::new(32400, 44600))
    );
    assert_eq!(
        imported.violations[2].bounds,
        Rect::new(Point::new(-919, 2121), Point::new(-566, 2475))
    );
    assert_eq!(imported.violations[3].rule, "klayout.via_point");
    assert_eq!(
        imported.violations[3].bounds,
        Rect::new(Point::new(6999, 7999), Point::new(7001, 8001))
    );
    assert_eq!(imported.violations[4].rule, "klayout.space_edgepair");
    assert_eq!(
        imported.violations[4].bounds,
        Rect::new(Point::new(-1, -1), Point::new(1001, 201))
    );
    assert_eq!(imported.violations[5].rule, "klayout.poly_edge");
    assert_eq!(
        imported.violations[5].bounds,
        Rect::new(Point::new(-1001, -1), Point::new(1001, 1))
    );
    assert_eq!(imported.violations[6].rule, "klayout.diff_polygon");
    assert_eq!(
        imported.violations[6].bounds,
        Rect::new(Point::new(0, 0), Point::new(2000, 2000))
    );
    assert_eq!(imported.violations[7].rule, "klayout.routing_path");
    assert_eq!(
        imported.violations[7].bounds,
        Rect::new(Point::new(-500, -100), Point::new(1100, 1400))
    );
    assert_eq!(imported.violations[8].rule, "klayout.labels_pin");
    assert_eq!(
        imported.violations[8].bounds,
        Rect::new(Point::new(3499, 4499), Point::new(3501, 4501))
    );
    assert_eq!(imported.violations[9].rule, "klayout.texts_dtext");
    assert_eq!(
        imported.violations[9].bounds,
        Rect::new(Point::new(3999, 4999), Point::new(4001, 5001))
    );
    assert_eq!(imported.violations[10].rule, "klayout.variant_child");
    assert_eq!(
        imported.violations[10].bounds,
        Rect::new(Point::new(4000, 5000), Point::new(4200, 5300))
    );

    let first_key = imported.violations[0].stable_key();
    let state = imported
        .marker_states
        .get(&first_key)
        .expect("KLayout item state should be keyed to the imported marker");
    assert!(state.visited);
    assert!(state.important);
    assert!(state.waived);
    assert_eq!(state.note.as_deref(), Some("reviewed externally"));
    assert_eq!(
        state.tags.get("rdb_multiplicity").map(String::as_str),
        Some("7")
    );
    assert_eq!(
        state.tags.get("rdb_category").map(String::as_str),
        Some("M1-Metal/Spacing")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_1")
            .map(String::as_str),
        Some("Metal 1 checks")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_2")
            .map(String::as_str),
        Some("Spacing from KLayout deck")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_category_3_part_1")
            .map(String::as_str),
        Some("Unused")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_category_3_description")
            .map(String::as_str),
        Some("Unused KLayout category declaration")
    );
    assert_eq!(
        state.tags.get("source_cell").map(String::as_str),
        Some("CHILD")
    );
    assert_eq!(
        state.tags.get("rdb_image").map(String::as_str),
        Some("embedded")
    );
    assert_eq!(
        state.tags.get("rdb_image_base64").map(String::as_str),
        Some("iVBORw0KGgo=")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_original_file")
            .map(String::as_str),
        Some("/tmp/source-run.lyrdb")
    );
    assert_eq!(
        state.tags.get("rdb_report_description").map(String::as_str),
        Some("External signoff markers")
    );
    assert_eq!(
        state.tags.get("rdb_report_top_cell").map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state.tags.get("rdb_report_generator").map(String::as_str),
        Some("KLayout DRC runset 2026.05")
    );
    assert_eq!(
        state.tags.get("rdb_geometry_value_1").map(String::as_str),
        Some("box: (1.0,2.0;1.2,2.3)")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_geometry_value_1_tag")
            .map(String::as_str),
        Some("geometry-review")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_1")
            .map(String::as_str),
        Some("text: \"spacing too small\"")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_2")
            .map(String::as_str),
        Some("[#'geometry-review'] box: (1.0,2.0;1.2,2.3)")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_3")
            .map(String::as_str),
        Some("[#'review-reference'] reference: TOP/r0/7.0/8.0")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_4")
            .map(String::as_str),
        Some("string: \"scalar review lane\"")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_5")
            .map(String::as_str),
        Some("float: 1.25")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_6")
            .map(String::as_str),
        Some("[#'measured-width'] float: 0.42")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_7")
            .map(String::as_str),
        Some("[#'review-note'] text: 'tagged scalar note'")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_8")
            .map(String::as_str),
        Some("text: ' leading scalar '")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_9")
            .map(String::as_str),
        Some("text: ''")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_10")
            .map(String::as_str),
        Some("text: '   '")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_11")
            .map(String::as_str),
        Some("[#'raw-review'] unknown: preserved raw payload")
    );
    assert_eq!(
        state.tags.get("custom_tag").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state.tags.get("review_s_item_tag").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_custom_tag")
            .map(String::as_str),
        Some("custom-tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_review_s_item_tag")
            .map(String::as_str),
        Some("review's item tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_important")
            .map(String::as_str),
        Some("KLayout important review marker")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_custom_tag")
            .map(String::as_str),
        Some("Custom KLayout review tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_review_s_item_tag")
            .map(String::as_str),
        Some("Quoted KLayout item tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_tag_unused_tag")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_unused_tag")
            .map(String::as_str),
        Some("unused-tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_unused_tag")
            .map(String::as_str),
        Some("Unused declared KLayout tag")
    );
    assert_eq!(
        state.tags.get("rdb_item_reference_1").map(String::as_str),
        Some("TOP/r0/7.0/8.0")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_reference_1_tag")
            .map(String::as_str),
        Some("review-reference")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_1_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_1_value").map(String::as_str),
        Some("spacing too small")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_2_type").map(String::as_str),
        Some("string")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_2_value").map(String::as_str),
        Some("scalar review lane")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_3_type").map(String::as_str),
        Some("float")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_3_value").map(String::as_str),
        Some("1.25")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_type").map(String::as_str),
        Some("float")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_value").map(String::as_str),
        Some("0.42")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_tag").map(String::as_str),
        Some("measured-width")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_value").map(String::as_str),
        Some("tagged scalar note")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_tag").map(String::as_str),
        Some("review-note")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_6_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_6_value").map(String::as_str),
        Some(" leading scalar ")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_7_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_7_empty").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_8_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_value_8_value_hex")
            .map(String::as_str),
        Some("202020")
    );
    assert_eq!(
        state.tags.get("rdb_raw_value_1").map(String::as_str),
        Some("unknown: preserved raw payload")
    );
    assert_eq!(
        state.tags.get("rdb_raw_value_1_tag").map(String::as_str),
        Some("raw-review")
    );
    assert_eq!(
        state.tags.get("rdb_cell_layout_name").map(String::as_str),
        Some("CHILD_LAYOUT")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r90 *1 17.5,-25")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_2_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_2_trans")
            .map(String::as_str),
        Some("*2 30,40")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_3_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_3_trans")
            .map(String::as_str),
        Some("r45")
    );
    let unused_declared_cell_index = (1..=16)
        .find(|index| {
            state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "UNUSED")
        })
        .expect("unused declared KLayout cell should be retained");
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_variant"
            ))
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_layout_name"
            ))
            .map(String::as_str),
        Some("UNUSED_LAYOUT")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_parent"
            ))
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_trans"
            ))
            .map(String::as_str),
        Some("r0 1,2")
    );
    let polygon_state = imported
        .marker_states
        .get(&imported.violations[6].stable_key())
        .expect("KLayout polygon item state should be keyed to the imported marker");
    assert_eq!(
        polygon_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("polygon: (0,0;2,0;2,2;0,2/0.5,0.5;1.0,0.5;1.0,1.0)")
    );
    let text_state = imported
        .marker_states
        .get(&imported.violations[9].stable_key())
        .expect("KLayout text geometry item state should be keyed to the imported marker");
    assert_eq!(
        text_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("text: ('text object',r90 4.0,5.0)")
    );
    assert_eq!(
        text_state
            .tags
            .get("rdb_item_ordered_value_1")
            .map(String::as_str),
        Some("text: ('text object',r90 4.0,5.0)")
    );
    let variant_state = imported
        .marker_states
        .get(&imported.violations[10].stable_key())
        .expect("KLayout unique cell variant should seed marker state");
    assert_eq!(
        variant_state.tags.get("source_cell").map(String::as_str),
        Some("VARIANT_CHILD")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_layout_name")
            .map(String::as_str),
        Some("VARIANT_CHILD_LAYOUT")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("TOP:1")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r0 4,5")
    );
    assert_eq!(
        DrcIssueStore::from_violations(imported.violations).validate(),
        Vec::new()
    );
}

#[test]
pub(crate) fn klayout_rdb_import_preserves_empty_root_metadata_markers() {
    let imported = import_klayout_rdb_markers(
        r#"
<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description/>
 <original-file/>
 <generator/>
 <top-cell/>
 <tags>
 </tags>
 <categories>
  <category>
   <name>EmptyMetadata</name>
   <description/>
   <categories>
   </categories>
  </category>
 </categories>
 <cells>
  <cell>
   <name>TOP</name>
   <variant/>
   <layout-name/>
   <references>
   </references>
  </cell>
 </cells>
 <items>
  <item>
   <tags/>
   <category>EmptyMetadata</category>
   <cell>TOP</cell>
   <visited>false</visited>
   <multiplicity>1</multiplicity>
   <comment/>
   <image/>
   <values>
    <value>box: (0,0;1,1)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB with native empty root metadata should import");

    assert_eq!(imported.report.description, None);
    assert_eq!(imported.report.original_file, None);
    assert_eq!(imported.report.generator, None);
    assert_eq!(imported.report.top_cell, None);
    assert_eq!(imported.violations.len(), 1);
    let state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("empty root metadata marker should preserve marker state");
    assert_eq!(
        state
            .tags
            .get("rdb_report_description_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_original_file_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_generator_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_top_cell_empty")
            .map(String::as_str),
        Some("true")
    );
}

#[test]
pub(crate) fn klayout_rdb_import_normalizes_native_hash_quoted_item_tags() {
    let imported = import_klayout_rdb_markers(
        r#"
<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <top-cell>TOP</top-cell>
 <tags>
  <tag>
   <name>review lane</name>
   <description>KLayout native hash-quoted item tag</description>
  </tag>
  <tag>
   <name>geometry_review</name>
   <description>KLayout native hash-word value tag</description>
  </tag>
 </tags>
  <items>
  <item>
   <tags>#hidden,#'review lane'</tags>
   <category>Metal.Width</category>
   <cell>TOP</cell>
   <values>
    <value>[#geometry_review] box: (1.0,2.0;1.5,2.5)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB native item tags should import");

    assert_eq!(imported.report.marker_count, 1);
    assert_eq!(imported.report.marker_state_count, 1);
    let state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("native KLayout item tag should attach marker state");
    assert!(state.hidden);
    assert_eq!(
        state.tags.get("review_lane").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_review_lane")
            .map(String::as_str),
        Some("review lane")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_geometry_value_1_tag")
            .map(String::as_str),
        Some("geometry_review")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_geometry_review")
            .map(String::as_str),
        Some("KLayout native hash-word value tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_review_lane")
            .map(String::as_str),
        Some("KLayout native hash-quoted item tag")
    );
}

#[test]
pub(crate) fn klayout_rdb_import_preserves_quoted_category_path_parts() {
    let imported = import_klayout_rdb_markers(
        r#"
<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <top-cell>TOP</top-cell>
 <categories>
  <category>
   <name>Layer's.Group</name>
   <description>Layer apostrophe category</description>
   <categories>
    <category>
     <name>Rule.Name</name>
     <description>Quoted subcategory with dot</description>
    </category>
   </categories>
  </category>
 </categories>
 <items>
  <item>
   <category>'Layer\'s.Group'.'Rule.Name'</category>
   <cell>TOP</cell>
   <values>
    <value>box: (1.0,2.0;1.5,2.5)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("quoted KLayout RDB category paths should import");

    assert_eq!(imported.report.marker_count, 1);
    assert_eq!(
        imported.violations[0].rule,
        "klayout.layer_s_group_rule_name"
    );
    let state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("quoted category metadata should seed marker state");
    assert_eq!(
        state.tags.get("rdb_category").map(String::as_str),
        Some("Layer's.Group/Rule.Name")
    );
    assert_eq!(
        state.tags.get("rdb_category_part_1").map(String::as_str),
        Some("Layer's.Group")
    );
    assert_eq!(
        state.tags.get("rdb_category_part_2").map(String::as_str),
        Some("Rule.Name")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_1")
            .map(String::as_str),
        Some("Layer apostrophe category")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_2")
            .map(String::as_str),
        Some("Quoted subcategory with dot")
    );
}

#[test]
pub(crate) fn klayout_rdb_import_keeps_literal_colon_cells_distinct_from_variants() {
    let imported = import_klayout_rdb_markers(
        r#"
<report-database>
 <top-cell>TOP</top-cell>
 <cells>
  <cell>
   <name>TOP:ANNOTATION</name>
   <variant/>
   <layout-name>LITERAL_LAYOUT</layout-name>
   <references>
    <reference>
     <parent>TOP</parent>
     <trans>r0 10,20</trans>
    </reference>
   </references>
  </cell>
  <cell>
   <name>TOP</name>
   <variant>ANNOTATION</variant>
   <layout-name>VARIANT_LAYOUT</layout-name>
   <references/>
  </cell>
 </cells>
 <items>
  <item>
   <category>Cell.Collision</category>
   <cell>TOP:ANNOTATION</cell>
   <values>
    <value>box: (1.0,2.0;1.5,2.5)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB cell names and variants should import");

    assert_eq!(imported.report.marker_count, 1);
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(11000, 22000), Point::new(11500, 22500))
    );
    let state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("cell metadata should seed marker state");
    assert_eq!(
        state.tags.get("source_cell").map(String::as_str),
        Some("TOP:ANNOTATION")
    );
    assert_eq!(
        state.tags.get("rdb_cell_layout_name").map(String::as_str),
        Some("LITERAL_LAYOUT")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r0 10,20")
    );
    let literal_index = (1..=4)
        .find(|index| {
            state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "TOP:ANNOTATION")
                && !state
                    .tags
                    .contains_key(&format!("rdb_declared_cell_{index}_variant"))
        })
        .expect("literal colon cell declaration should be retained");
    assert_eq!(
        state
            .tags
            .get(&format!("rdb_declared_cell_{literal_index}_layout_name"))
            .map(String::as_str),
        Some("LITERAL_LAYOUT")
    );
    let variant_index = (1..=4)
        .find(|index| {
            state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "TOP")
                && state
                    .tags
                    .get(&format!("rdb_declared_cell_{index}_variant"))
                    .is_some_and(|variant| variant == "ANNOTATION")
        })
        .expect("cell variant declaration should be retained");
    assert_eq!(
        state
            .tags
            .get(&format!("rdb_declared_cell_{variant_index}_layout_name"))
            .map(String::as_str),
        Some("VARIANT_LAYOUT")
    );
}

#[test]
pub(crate) fn klayout_rdb_import_uses_document_context_for_incomplete_cell_graph() {
    let mut document = Document::new("KLayout RDB fallback context");
    let top_cell = document.top_cell;
    document.cell_mut(top_cell).unwrap().name = "TOP".to_string();
    let child = document.create_cell("CHILD");
    let child_transform = Transform {
        matrix: [0, -1, 1, 0],
        translation: Vector::new(17_500, -25_000),
    };
    document
        .insert_instance_in_top(child, child_transform)
        .expect("fallback context fixture should insert child instance");
    let context = KlayoutRdbImportContext::from_document(&document);
    let contents = r#"
<report-database>
 <top-cell>TOP</top-cell>
 <cells>
  <cell>
   <name>CHILD</name>
   <layout-name>CHILD_LAYOUT</layout-name>
   <references/>
  </cell>
 </cells>
 <items>
  <item>
   <category>Metal.Width</category>
   <cell>CHILD</cell>
   <values>
    <value>box: (1.0,2.0;1.5,2.5)</value>
   </values>
  </item>
 </items>
</report-database>
"#;

    let imported_without_context =
        import_klayout_rdb_markers(contents).expect("RDB import without context should parse");
    assert_eq!(
        imported_without_context.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1500, 2500))
    );

    let imported_with_context = import_klayout_rdb_markers_with_context(contents, &context)
        .expect("RDB import with document context should parse");
    assert_eq!(imported_with_context.report.marker_count, 1);
    assert_eq!(
        imported_with_context.violations[0].bounds,
        Rect::new(Point::new(15000, -24000), Point::new(15500, -23500))
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
