use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrcSeverity {
    Error,
}

impl DrcSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrcIssueRecord {
    pub key: String,
    pub severity: DrcSeverity,
    pub violation: DrcViolation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrcIssueSummary {
    pub total_count: usize,
    pub displayed_count: usize,
    pub omitted_count: usize,
    pub error_count: usize,
    pub rule_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DrcIssueStore {
    pub records: Vec<DrcIssueRecord>,
    key_index: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrcValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrcValidationFinding {
    pub severity: DrcValidationSeverity,
    pub message: String,
}

impl DrcValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: DrcValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DrcValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl DrcViolation {
    pub fn stable_key(&self) -> String {
        let mut shapes = self
            .shape_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>();
        shapes.sort();
        format!(
            "{}|{}|{},{},{},{}|{}|{:.3}",
            self.rule,
            shapes.join(","),
            self.bounds.min.x,
            self.bounds.min.y,
            self.bounds.max.x,
            self.bounds.max.y,
            self.required,
            self.actual
        )
    }
}

impl DrcIssueStore {
    pub fn from_violations(violations: Vec<DrcViolation>) -> Self {
        let records = violations
            .into_iter()
            .map(|violation| DrcIssueRecord {
                key: violation.stable_key(),
                severity: DrcSeverity::Error,
                violation,
            })
            .collect::<Vec<_>>();
        let mut key_index = BTreeMap::new();
        for (index, record) in records.iter().enumerate() {
            key_index.entry(record.key.clone()).or_insert(index);
        }
        Self { records, key_index }
    }

    pub fn get(&self, key: &str) -> Option<&DrcIssueRecord> {
        self.key_index
            .get(key)
            .and_then(|index| self.records.get(*index))
    }

    pub fn displayed_records(&self, max_issue_rows: usize) -> &[DrcIssueRecord] {
        let displayed_count = self.records.len().min(max_issue_rows);
        &self.records[..displayed_count]
    }

    pub fn summary(&self, max_issue_rows: usize) -> DrcIssueSummary {
        let displayed_count = self.records.len().min(max_issue_rows);
        let mut rule_counts = BTreeMap::new();
        for record in &self.records {
            *rule_counts
                .entry(record.violation.rule.clone())
                .or_insert(0) += 1;
        }
        DrcIssueSummary {
            total_count: self.records.len(),
            displayed_count,
            omitted_count: self.records.len().saturating_sub(displayed_count),
            error_count: self
                .records
                .iter()
                .filter(|record| record.severity == DrcSeverity::Error)
                .count(),
            rule_counts,
        }
    }

    pub fn validate(&self) -> Vec<DrcValidationFinding> {
        let mut findings = Vec::new();
        let mut seen_keys = BTreeSet::new();
        for (index, record) in self.records.iter().enumerate() {
            validate_drc_record(record, index, &mut seen_keys, &mut findings);
            match self.key_index.get(&record.key) {
                Some(indexed) => {
                    if self
                        .records
                        .get(*indexed)
                        .is_none_or(|indexed_record| indexed_record.key != record.key)
                    {
                        findings.push(DrcValidationFinding::error(format!(
                            "DRC issue key {:?} points at stale record index {}",
                            record.key, indexed
                        )));
                    }
                }
                None => findings.push(DrcValidationFinding::error(format!(
                    "DRC issue key {:?} is missing from the lookup index",
                    record.key
                ))),
            }
        }
        for (key, index) in &self.key_index {
            if self
                .records
                .get(*index)
                .is_none_or(|record| record.key != *key)
            {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC lookup index contains stale key {:?} at index {}",
                    key, index
                )));
            }
        }
        findings
    }
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

    pub fn validate_for_document(&self, document: &Document) -> Vec<DrcValidationFinding> {
        let mut findings = Vec::new();
        if self.grid <= 0 {
            findings.push(DrcValidationFinding::error(format!(
                "DRC rule deck grid must be positive, got {}",
                self.grid
            )));
        }

        for (layer_id, required) in &self.min_width {
            validate_rule_layer(document, *layer_id, "min-width", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-width rule for layer {:?} must be positive, got {}",
                    layer_id, required
                )));
            }
        }
        for (layer_id, required) in &self.min_spacing {
            validate_rule_layer(document, *layer_id, "min-spacing", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-spacing rule for layer {:?} must be positive, got {}",
                    layer_id, required
                )));
            }
        }

        let mut enclosure_keys = BTreeSet::new();
        for rule in &self.via_enclosure {
            validate_rule_layer(document, rule.via_layer, "via-enclosure via", &mut findings);
            validate_rule_layer(
                document,
                rule.enclosure_layer,
                "via-enclosure enclosing",
                &mut findings,
            );
            if !enclosure_keys.insert((rule.via_layer, rule.enclosure_layer)) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC via-enclosure rule {:?}->{:?} is duplicated",
                    rule.via_layer, rule.enclosure_layer
                )));
            }
            if rule.required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC via-enclosure rule {:?}->{:?} must be positive, got {}",
                    rule.via_layer, rule.enclosure_layer, rule.required
                )));
            }
        }

        let mut overlap_keys = BTreeSet::new();
        for rule in &self.forbidden_overlaps {
            validate_rule_layer(document, rule.a, "forbidden-overlap", &mut findings);
            validate_rule_layer(document, rule.b, "forbidden-overlap", &mut findings);
            let key = if rule.a <= rule.b {
                (rule.a, rule.b, rule.name.trim().to_string())
            } else {
                (rule.b, rule.a, rule.name.trim().to_string())
            };
            if !overlap_keys.insert(key) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC forbidden-overlap rule {:?}<->{:?} named {:?} is duplicated",
                    rule.a, rule.b, rule.name
                )));
            }
            if rule.name.trim().is_empty() {
                findings.push(DrcValidationFinding::error(
                    "DRC forbidden-overlap rule name cannot be empty",
                ));
            }
        }

        findings
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

fn validate_rule_layer(
    document: &Document,
    layer_id: LayerId,
    context: &str,
    findings: &mut Vec<DrcValidationFinding>,
) {
    if !document.layers.contains_key(&layer_id) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC {context} rule references missing document layer {:?}",
            layer_id
        )));
    }
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

fn validate_drc_record(
    record: &DrcIssueRecord,
    index: usize,
    seen_keys: &mut BTreeSet<String>,
    findings: &mut Vec<DrcValidationFinding>,
) {
    let expected_key = record.violation.stable_key();
    if record.key.trim().is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC record at index {index} has an empty stable issue key"
        )));
    } else if !seen_keys.insert(record.key.clone()) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC stable issue key {:?} is duplicated",
            record.key
        )));
    }
    if record.key != expected_key {
        findings.push(DrcValidationFinding::error(format!(
            "DRC record key {:?} does not match violation stable key {:?}",
            record.key, expected_key
        )));
    }

    validate_violation(&record.violation, index, findings);
}

fn validate_violation(
    violation: &DrcViolation,
    index: usize,
    findings: &mut Vec<DrcValidationFinding>,
) {
    let expected_id = index + 1;
    if violation.id == 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation at index {index} has id 0"
        )));
    } else if violation.id != expected_id {
        findings.push(DrcValidationFinding::warning(format!(
            "DRC violation at index {index} has id {}, expected {}",
            violation.id, expected_id
        )));
    }
    if violation.rule.trim().is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has an empty rule",
            violation.id
        )));
    }
    if violation.message.trim().is_empty() {
        findings.push(DrcValidationFinding::warning(format!(
            "DRC violation {} has an empty message",
            violation.id
        )));
    }
    if violation.shape_ids.is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has no shape references",
            violation.id
        )));
    }
    let mut seen_shapes = BTreeSet::new();
    for shape_id in &violation.shape_ids {
        if shape_id.0 == 0 {
            findings.push(DrcValidationFinding::error(format!(
                "DRC violation {} references invalid shape id 0",
                violation.id
            )));
        }
        if !seen_shapes.insert(*shape_id) {
            findings.push(DrcValidationFinding::error(format!(
                "DRC violation {} repeats shape id {:?}",
                violation.id, shape_id
            )));
        }
    }
    if violation.bounds.width() < 0 || violation.bounds.height() < 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has inverted bounds {:?}",
            violation.id, violation.bounds
        )));
    }
    if violation.required < 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a negative required value {}",
            violation.id, violation.required
        )));
    }
    if !violation.actual.is_finite() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a non-finite actual value {}",
            violation.id, violation.actual
        )));
    } else if violation.actual < 0.0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC violation {} has a negative actual value {}",
            violation.id, violation.actual
        )));
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
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct DrcGoldenFixture {
        name: String,
        shapes: Vec<DrcFixtureShape>,
        expect: DrcGoldenExpectation,
    }

    #[derive(Debug, Deserialize)]
    struct DrcGoldenExpectation {
        total_count: usize,
        omitted_count: usize,
        rule_counts: BTreeMap<String, usize>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum DrcFixtureShape {
        Rect {
            layer: String,
            x: Coord,
            y: Coord,
            w: Coord,
            h: Coord,
        },
    }

    fn document_from_drc_fixture(fixture: &DrcGoldenFixture) -> Document {
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

    fn fixture_layer(document: &Document, layer: &str) -> LayerId {
        let process = ProcessLayer::from_technology_name(layer)
            .unwrap_or_else(|| panic!("unknown fixture layer {layer}"));
        document
            .layer_by_process(process)
            .unwrap_or_else(|| panic!("fixture layer {layer} missing from document"))
    }

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
    fn drc_golden_fixture_reports_all_rule_families_once() {
        let fixture: DrcGoldenFixture =
            serde_json::from_str(include_str!("../../../fixtures/quality/drc_golden.json"))
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
    fn violation_stable_key_ignores_row_id_and_shape_order() {
        let mut first = DrcViolation {
            id: 1,
            rule: "min_spacing".to_string(),
            message: "spacing".to_string(),
            shape_ids: vec![ShapeId(7), ShapeId(3)],
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
    fn issue_store_queries_by_stable_key_and_summarizes_capped_rows() {
        let first = DrcViolation {
            id: 1,
            rule: "min_width".to_string(),
            message: "too narrow".to_string(),
            shape_ids: vec![ShapeId(1)],
            bounds: Rect::from_min_size(Point::new(0, 0), 100, 20),
            required: 100,
            actual: 20.0,
        };
        let second = DrcViolation {
            id: 2,
            rule: "min_spacing".to_string(),
            message: "too close".to_string(),
            shape_ids: vec![ShapeId(2), ShapeId(3)],
            bounds: Rect::from_min_size(Point::new(200, 0), 300, 100),
            required: 200,
            actual: 90.0,
        };
        let third = DrcViolation {
            id: 3,
            rule: "min_spacing".to_string(),
            message: "too close".to_string(),
            shape_ids: vec![ShapeId(4), ShapeId(5)],
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
    fn issue_store_validation_rejects_stale_duplicate_and_malformed_records() {
        let first = DrcViolation {
            id: 1,
            rule: "min_spacing".to_string(),
            message: "too close".to_string(),
            shape_ids: vec![ShapeId(7), ShapeId(3)],
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
                .any(|message| message.contains("stable issue key")
                    && message.contains("duplicated"))
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
    fn rule_deck_validation_rejects_missing_layers_and_invalid_rules() {
        let doc = Document::new("bad drc rules");
        let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
        let missing = LayerId(999);
        let rules = RuleDeck {
            grid: 0,
            min_width: BTreeMap::from([(missing, 0)]),
            min_spacing: BTreeMap::from([(metal1, -10)]),
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
            messages.iter().any(
                |message| message.contains("min-width") && message.contains("must be positive")
            )
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("min-spacing")
                    && message.contains("must be positive"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("via-enclosure") && message.contains("duplicated"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("forbidden-overlap")
                    && message.contains("duplicated"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("name cannot be empty"))
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
