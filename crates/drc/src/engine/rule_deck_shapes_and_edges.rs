#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDeck {
    pub grid: Coord,
    #[serde(default)]
    pub derived_layers: Vec<DerivedLayerRule>,
    #[serde(default)]
    pub derived_min_width: BTreeMap<String, Coord>,
    #[serde(default)]
    pub derived_max_width: BTreeMap<String, Coord>,
    #[serde(default)]
    pub derived_min_area: BTreeMap<String, Coord>,
    #[serde(default)]
    pub derived_max_area: BTreeMap<String, Coord>,
    #[serde(default)]
    pub derived_min_spacing: BTreeMap<String, Coord>,
    #[serde(default)]
    pub derived_min_edge_spacing: BTreeMap<String, Coord>,
    pub min_width: BTreeMap<LayerId, Coord>,
    #[serde(default)]
    pub max_width: BTreeMap<LayerId, Coord>,
    #[serde(default)]
    pub min_area: BTreeMap<LayerId, Coord>,
    #[serde(default)]
    pub max_area: BTreeMap<LayerId, Coord>,
    pub min_spacing: BTreeMap<LayerId, Coord>,
    #[serde(default)]
    pub min_edge_spacing: BTreeMap<LayerId, Coord>,
    pub via_enclosure: Vec<EnclosureRule>,
    pub forbidden_overlaps: Vec<ForbiddenOverlapRule>,
    #[serde(default)]
    pub derived_forbidden_overlaps: Vec<DerivedForbiddenOverlapRule>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedLayerOperation {
    And,
    Not,
    Or,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DerivedLayerRule {
    pub name: String,
    pub operation: DerivedLayerOperation,
    pub a: LayerId,
    pub b: LayerId,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DerivedForbiddenOverlapRule {
    pub derived: String,
    pub layer: LayerId,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrcViolation {
    pub id: usize,
    pub rule: String,
    pub message: String,
    pub shape_ids: Vec<ShapeId>,
    #[serde(default)]
    pub occurrence_ids: Vec<ShapeOccurrenceId>,
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
    pub(crate) key_index: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CalibreRveImportReport {
    pub marker_count: usize,
    pub skipped_line_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibreRveImportResult {
    pub violations: Vec<DrcViolation>,
    pub report: CalibreRveImportReport,
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
    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self {
            severity: DrcValidationSeverity::Error,
            message: message.into(),
        }
    }

    pub(crate) fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: DrcValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl DrcViolation {
    pub fn stable_key(&self) -> String {
        let mut shapes = if self.occurrence_ids.is_empty() {
            self.shape_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>()
        } else {
            self.occurrence_ids
                .iter()
                .map(occurrence_stable_key)
                .collect::<Vec<_>>()
        };
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

pub(crate) fn occurrence_stable_key(occurrence: &ShapeOccurrenceId) -> String {
    let mut key = occurrence.shape.0.to_string();
    if !occurrence.instance_path.is_empty() {
        key.push('@');
        key.push_str(
            &occurrence
                .instance_path
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>()
                .join("."),
        );
    }
    if !occurrence.array_path.is_empty() {
        key.push('#');
        key.push_str(
            &occurrence
                .array_path
                .iter()
                .map(|index| format!("{}:{}", index.column, index.row))
                .collect::<Vec<_>>()
                .join("."),
        );
    }
    key
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
                            "DRC issue key {:?} points at stale record index {indexed}",
                            record.key
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
                    "DRC lookup index contains stale key {key:?} at index {index}"
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

        let mut max_width = BTreeMap::new();
        for rule in &technology.drc.max_width {
            if rule.value > 0 {
                max_width.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut min_area = BTreeMap::new();
        for rule in &technology.drc.min_area {
            if rule.value > 0 {
                min_area.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut max_area = BTreeMap::new();
        for rule in &technology.drc.max_area {
            if rule.value > 0 {
                max_area.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut min_spacing = BTreeMap::new();
        for rule in &technology.drc.min_spacing {
            if rule.value > 0 {
                min_spacing.insert(document_layer_id(document, &rule.layer)?, rule.value);
            }
        }

        let mut min_edge_spacing = BTreeMap::new();
        for rule in &technology.drc.min_edge_spacing {
            if rule.value > 0 {
                min_edge_spacing.insert(document_layer_id(document, &rule.layer)?, rule.value);
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
            derived_layers: Vec::new(),
            derived_min_width: BTreeMap::new(),
            derived_max_width: BTreeMap::new(),
            derived_min_area: BTreeMap::new(),
            derived_max_area: BTreeMap::new(),
            derived_min_spacing: BTreeMap::new(),
            derived_min_edge_spacing: BTreeMap::new(),
            min_width,
            max_width,
            min_area,
            max_area,
            min_spacing,
            min_edge_spacing,
            via_enclosure,
            forbidden_overlaps,
            derived_forbidden_overlaps: Vec::new(),
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

        let mut derived_layer_names = BTreeSet::new();
        for rule in &self.derived_layers {
            let name = normalize_derived_layer_name(&rule.name);
            if name.is_empty() {
                findings.push(DrcValidationFinding::error(
                    "DRC derived-layer rule name cannot be empty",
                ));
            } else if !derived_layer_names.insert(name) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC derived-layer rule {:?} is duplicated",
                    rule.name
                )));
            }
            validate_rule_layer(document, rule.a, "derived-layer source", &mut findings);
            validate_rule_layer(document, rule.b, "derived-layer source", &mut findings);
        }
        for (name, required) in &self.derived_min_area {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-min-area",
                &derived_layer_names,
                &mut findings,
            );
        }
        for (name, required) in &self.derived_max_area {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-max-area",
                &derived_layer_names,
                &mut findings,
            );
        }
        for (name, required) in &self.derived_min_width {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-min-width",
                &derived_layer_names,
                &mut findings,
            );
        }
        for (name, required) in &self.derived_max_width {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-max-width",
                &derived_layer_names,
                &mut findings,
            );
        }
        for (name, required) in &self.derived_min_spacing {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-min-spacing",
                &derived_layer_names,
                &mut findings,
            );
        }
        for (name, required) in &self.derived_min_edge_spacing {
            validate_derived_numeric_rule(
                name,
                *required,
                "derived-min-edge-spacing",
                &derived_layer_names,
                &mut findings,
            );
        }

        for (layer_id, required) in &self.min_width {
            validate_rule_layer(document, *layer_id, "min-width", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-width rule for layer {layer_id:?} must be positive, got {required}"
                )));
            }
        }
        for (layer_id, required) in &self.max_width {
            validate_rule_layer(document, *layer_id, "max-width", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC max-width rule for layer {layer_id:?} must be positive, got {required}"
                )));
            }
        }
        for (layer_id, required) in &self.min_area {
            validate_rule_layer(document, *layer_id, "min-area", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-area rule for layer {layer_id:?} must be positive, got {required}"
                )));
            }
        }
        for (layer_id, required) in &self.max_area {
            validate_rule_layer(document, *layer_id, "max-area", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC max-area rule for layer {layer_id:?} must be positive, got {required}"
                )));
            }
        }
        for (layer_id, required) in &self.min_spacing {
            validate_rule_layer(document, *layer_id, "min-spacing", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-spacing rule for layer {layer_id:?} must be positive, got {required}"
                )));
            }
        }
        for (layer_id, required) in &self.min_edge_spacing {
            validate_rule_layer(document, *layer_id, "min-edge-spacing", &mut findings);
            if *required <= 0 {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC min-edge-spacing rule for layer {layer_id:?} must be positive, got {required}"
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

        let mut derived_overlap_keys = BTreeSet::new();
        for rule in &self.derived_forbidden_overlaps {
            let derived = normalize_derived_layer_name(&rule.derived);
            if derived.is_empty() {
                findings.push(DrcValidationFinding::error(
                    "DRC derived-overlap rule references an empty derived layer name",
                ));
            } else if !derived_layer_names.contains(&derived) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC derived-overlap rule references missing derived layer {:?}",
                    rule.derived
                )));
            }
            validate_rule_layer(
                document,
                rule.layer,
                "derived-overlap physical layer",
                &mut findings,
            );
            let key = (derived, rule.layer, rule.name.trim().to_string());
            if !derived_overlap_keys.insert(key) {
                findings.push(DrcValidationFinding::error(format!(
                    "DRC derived-overlap rule {:?}->{:?} named {:?} is duplicated",
                    rule.derived, rule.layer, rule.name
                )));
            }
            if rule.name.trim().is_empty() {
                findings.push(DrcValidationFinding::error(
                    "DRC derived-overlap rule name cannot be empty",
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
    check_max_width(document, rules, &mut violations);
    check_min_area(document, rules, &mut violations);
    check_max_area(document, rules, &mut violations);
    check_derived_min_width(document, rules, &mut violations);
    check_derived_max_width(document, rules, &mut violations);
    check_derived_min_area(document, rules, &mut violations);
    check_derived_max_area(document, rules, &mut violations);
    check_derived_spacing(document, rules, &mut violations);
    check_derived_edge_spacing(document, rules, &mut violations);
    check_edge_spacing(document, rules, &mut violations);
    check_spacing(document, rules, &mut violations);
    check_derived_forbidden_overlaps(document, rules, &mut violations);
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

pub fn import_calibre_rve_markers(text: &str) -> CalibreRveImportResult {
    let mut violations = Vec::new();
    let mut skipped_line_count = 0;
    let mut warnings = Vec::new();
    let mut current_rule = String::from("marker");
    let mut current_message = String::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("//")
            || line.starts_with('*')
            || line == "{"
            || line == "}"
        {
            continue;
        }

        let Some(first) = line.split_whitespace().next() else {
            continue;
        };
        let first_lower = first.to_ascii_lowercase();
        let marker_bounds = if matches!(first_lower.as_str(), "p" | "polygon") {
            calibre_polygon_bounds(line)
        } else if matches!(
            first_lower.as_str(),
            "r" | "rect" | "rectangle" | "b" | "box"
        ) {
            calibre_rect_bounds(line)
        } else if matches!(first_lower.as_str(), "e" | "edge" | "path" | "line") {
            calibre_edge_bounds(line)
        } else {
            None
        };

        if let Some(bounds) = marker_bounds {
            let rule = format!(
                "calibre.{}",
                sanitize_calibre_rule_identifier(&current_rule)
            );
            let message = if current_message.trim().is_empty() {
                format!("Imported Calibre/RVE marker for {current_rule}")
            } else {
                current_message.clone()
            };
            violations.push(DrcViolation {
                id: violations.len() + 1,
                rule,
                message,
                shape_ids: Vec::new(),
                occurrence_ids: Vec::new(),
                bounds,
                required: 0,
                actual: 0.0,
            });
            continue;
        }

        if first_lower.starts_with('@') {
            current_message = line.trim_start_matches('@').trim().to_string();
            if current_rule == "marker"
                && let Some(rule) = current_message.split_whitespace().next()
            {
                current_rule = rule.to_string();
            }
            continue;
        }

        if let Some(rule) = calibre_rule_name_from_line(line) {
            current_rule = rule;
            current_message.clear();
        } else {
            skipped_line_count += 1;
            if warnings.len() < 8 {
                warnings.push(format!(
                    "line {line_number}: skipped unsupported marker syntax"
                ));
            }
        }
    }

    CalibreRveImportResult {
        report: CalibreRveImportReport {
            marker_count: violations.len(),
            skipped_line_count,
            warnings,
        },
        violations,
    }
}

pub(crate) fn max_rule_distance(rules: &RuleDeck) -> Coord {
    rules
        .min_width
        .values()
        .chain(rules.max_width.values())
        .chain(rules.derived_min_width.values())
        .chain(rules.derived_max_width.values())
        .chain(rules.derived_min_spacing.values())
        .chain(rules.derived_min_edge_spacing.values())
        .chain(rules.min_spacing.values())
        .chain(rules.min_edge_spacing.values())
        .chain(rules.via_enclosure.iter().map(|rule| &rule.required))
        .copied()
        .max()
        .unwrap_or(rules.grid)
        .max(rules.grid)
}

pub(crate) fn document_layer_id(
    document: &Document,
    reference: &str,
) -> Result<LayerId, TechnologyError> {
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

pub(crate) fn normalize_layer_ref(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .filter(|character| *character != '_' && *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn calibre_polygon_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    if coords.len() < 4 || coords.len() % 2 != 0 {
        return None;
    }
    let points = coords
        .chunks_exact(2)
        .map(|chunk| Point::new(chunk[0], chunk[1]))
        .collect::<Vec<_>>();
    Rect::from_points(&points).map(nonzero_marker_bounds)
}

pub(crate) fn calibre_rect_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    if coords.len() < 4 {
        return None;
    }
    Some(nonzero_marker_bounds(Rect::new(
        Point::new(coords[0], coords[1]),
        Point::new(coords[2], coords[3]),
    )))
}

pub(crate) fn calibre_edge_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    if coords.len() < 4 {
        return None;
    }
    Some(nonzero_marker_bounds(
        Rect::new(
            Point::new(coords[0], coords[1]),
            Point::new(coords[2], coords[3]),
        )
        .expanded(1),
    ))
}

pub(crate) fn calibre_geometry_coords(line: &str) -> Vec<Coord> {
    let mut values = line
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '(' | ')' | '[' | ']' | ',' | ';' | ':')
        })
        .skip(1)
        .filter_map(parse_calibre_coord)
        .collect::<Vec<_>>();
    if values.len() >= 3 {
        let count = values[0];
        if count > 0 && values.len() == count as usize * 2 + 1 {
            values.remove(0);
        }
    }
    values
}

pub(crate) fn parse_calibre_coord(value: &str) -> Option<Coord> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    value.parse::<Coord>().ok().or_else(|| {
        value
            .parse::<f64>()
            .ok()
            .map(|coord| coord.round() as Coord)
    })
}

pub(crate) fn nonzero_marker_bounds(bounds: Rect) -> Rect {
    if bounds.width() == 0 || bounds.height() == 0 {
        bounds.expanded(1)
    } else {
        bounds
    }
}

pub(crate) fn calibre_rule_name_from_line(line: &str) -> Option<String> {
    let line = line.trim().trim_matches('{').trim();
    if line.is_empty() || line.contains(char::is_whitespace) && line.split_whitespace().count() > 8
    {
        return None;
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    let rule = match words.as_slice() {
        ["RULE", rest @ ..] | ["Rule", rest @ ..] | ["rule", rest @ ..] => rest.join("_"),
        ["RULECHECK", rest @ ..] | ["RuleCheck", rest @ ..] | ["rulecheck", rest @ ..] => {
            rest.join("_")
        }
        [single] => (*single).to_string(),
        _ => words.join("_"),
    };
    let rule = rule.trim_matches(|character: char| matches!(character, ':' | '{' | '}'));
    (!rule.is_empty()).then(|| rule.to_string())
}

pub(crate) fn sanitize_calibre_rule_identifier(value: &str) -> String {
    let mut output = String::new();
    let mut previous_separator = false;
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if !previous_separator {
            output.push('_');
            previous_separator = true;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        "marker".to_string()
    } else {
        output
    }
}

pub(crate) fn validate_rule_layer(
    document: &Document,
    layer_id: LayerId,
    context: &str,
    findings: &mut Vec<DrcValidationFinding>,
) {
    if !document.layers.contains_key(&layer_id) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC {context} rule references missing document layer {layer_id:?}"
        )));
    }
}

pub(crate) fn validate_derived_numeric_rule(
    name: &str,
    required: Coord,
    context: &str,
    derived_layer_names: &BTreeSet<String>,
    findings: &mut Vec<DrcValidationFinding>,
) {
    let normalized = normalize_derived_layer_name(name);
    if normalized.is_empty() {
        findings.push(DrcValidationFinding::error(format!(
            "DRC {context} rule references an empty derived layer name"
        )));
    } else if !derived_layer_names.contains(&normalized) {
        findings.push(DrcValidationFinding::error(format!(
            "DRC {context} rule references missing derived layer {name:?}"
        )));
    }
    if required <= 0 {
        findings.push(DrcValidationFinding::error(format!(
            "DRC {context} rule for derived layer {name:?} must be positive, got {required}"
        )));
    }
}

pub(crate) fn normalize_derived_layer_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

#[derive(Clone, Debug)]
pub(crate) struct DrcShape {
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) shape: Shape,
}

impl DrcShape {
    pub(crate) fn source_shape_id(&self) -> ShapeId {
        self.occurrence.source_shape_id()
    }

    pub(crate) fn layer(&self) -> LayerId {
        self.shape.layer
    }

    pub(crate) fn bounds(&self) -> Rect {
        self.shape.kind.bounds()
    }
}

pub(crate) fn drc_shapes(document: &Document) -> Vec<DrcShape> {
    document
        .flattened_shapes()
        .into_iter()
        .map(|flattened| DrcShape {
            occurrence: flattened.id.clone(),
            shape: flattened.transformed_shape(),
        })
        .collect()
}

#[derive(Clone, Debug)]
pub(crate) struct DerivedDrcShape {
    pub(crate) shape_ids: Vec<ShapeId>,
    pub(crate) occurrence_ids: Vec<ShapeOccurrenceId>,
    pub(crate) bounds: Rect,
}

pub(crate) fn derived_drc_shapes(
    document: &Document,
    rules: &RuleDeck,
    derived_layer_name: &str,
) -> Vec<DerivedDrcShape> {
    let target_name = normalize_derived_layer_name(derived_layer_name);
    if target_name.is_empty() {
        return Vec::new();
    }
    let source_shapes = drc_shapes(document);
    let mut derived = Vec::new();
    for rule in rules
        .derived_layers
        .iter()
        .filter(|rule| normalize_derived_layer_name(&rule.name) == target_name)
    {
        match rule.operation {
            DerivedLayerOperation::And => {
                for left_index in 0..source_shapes.len() {
                    let left = &source_shapes[left_index];
                    if left.layer() != rule.a {
                        continue;
                    }
                    for right_index in 0..source_shapes.len() {
                        if rule.a == rule.b && right_index <= left_index {
                            continue;
                        }
                        let right = &source_shapes[right_index];
                        if right.layer() != rule.b {
                            continue;
                        }
                        let Some(bounds) = left.bounds().intersection(right.bounds()) else {
                            continue;
                        };
                        if bounds.area() <= 0 {
                            continue;
                        }
                        derived.push(DerivedDrcShape {
                            shape_ids: vec![left.source_shape_id(), right.source_shape_id()],
                            occurrence_ids: vec![left.occurrence.clone(), right.occurrence.clone()],
                            bounds,
                        });
                    }
                }
            }
            DerivedLayerOperation::Or => {
                let mut seen = BTreeSet::new();
                for shape in source_shapes
                    .iter()
                    .filter(|shape| shape.layer() == rule.a || shape.layer() == rule.b)
                {
                    if seen.insert(occurrence_stable_key(&shape.occurrence)) {
                        derived.push(DerivedDrcShape {
                            shape_ids: vec![shape.source_shape_id()],
                            occurrence_ids: vec![shape.occurrence.clone()],
                            bounds: shape.bounds(),
                        });
                    }
                }
            }
            DerivedLayerOperation::Not => {
                for left in source_shapes.iter().filter(|shape| shape.layer() == rule.a) {
                    let cutters = source_shapes
                        .iter()
                        .filter(|shape| {
                            shape.layer() == rule.b && shape.occurrence != left.occurrence
                        })
                        .filter(|shape| {
                            left.bounds()
                                .intersection(shape.bounds())
                                .is_some_and(|overlap| overlap.area() > 0)
                        })
                        .collect::<Vec<_>>();
                    let cut_bounds = cutters
                        .iter()
                        .map(|shape| shape.bounds())
                        .collect::<Vec<_>>();
                    for bounds in subtract_rects(left.bounds(), &cut_bounds) {
                        let (shape_ids, occurrence_ids) = derived_not_shape_refs(left, &cutters);
                        derived.push(DerivedDrcShape {
                            shape_ids,
                            occurrence_ids,
                            bounds,
                        });
                    }
                }
            }
        }
    }
    derived
}

pub(crate) fn derived_not_shape_refs(
    source: &DrcShape,
    cutters: &[&DrcShape],
) -> (Vec<ShapeId>, Vec<ShapeOccurrenceId>) {
    let mut seen = BTreeSet::new();
    let mut shape_ids = Vec::new();
    let mut occurrence_ids = Vec::new();
    for shape in std::iter::once(source).chain(cutters.iter().copied()) {
        if seen.insert(occurrence_stable_key(&shape.occurrence)) {
            shape_ids.push(shape.source_shape_id());
            occurrence_ids.push(shape.occurrence.clone());
        }
    }
    (shape_ids, occurrence_ids)
}

pub(crate) fn positive_area_rect(min: Point, max: Point) -> Option<Rect> {
    let rect = Rect::new(min, max);
    (rect.width() > 0 && rect.height() > 0).then_some(rect)
}

pub(crate) fn rect_difference(rect: Rect, cut: Rect) -> Vec<Rect> {
    let Some(overlap) = rect.intersection(cut) else {
        return vec![rect];
    };
    if overlap.width() <= 0 || overlap.height() <= 0 {
        return vec![rect];
    }

    let mut fragments = Vec::new();
    if let Some(fragment) = positive_area_rect(rect.min, Point::new(rect.max.x, overlap.min.y)) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(Point::new(rect.min.x, overlap.max.y), rect.max) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(
        Point::new(rect.min.x, overlap.min.y),
        Point::new(overlap.min.x, overlap.max.y),
    ) {
        fragments.push(fragment);
    }
    if let Some(fragment) = positive_area_rect(
        Point::new(overlap.max.x, overlap.min.y),
        Point::new(rect.max.x, overlap.max.y),
    ) {
        fragments.push(fragment);
    }
    fragments
}

pub(crate) fn subtract_rects(rect: Rect, cuts: &[Rect]) -> Vec<Rect> {
    let mut fragments = vec![rect];
    for cut in cuts {
        let mut next = Vec::new();
        for fragment in fragments {
            next.extend(rect_difference(fragment, *cut));
        }
        fragments = next;
        if fragments.is_empty() {
            break;
        }
    }
    fragments
}

pub(crate) fn derived_layer_rule_suffix(name: &str) -> String {
    let mut suffix = String::new();
    let mut previous_separator = false;
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            suffix.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if !previous_separator {
            suffix.push('_');
            previous_separator = true;
        }
    }
    let suffix = suffix.trim_matches('_').to_string();
    if suffix.is_empty() {
        "derived".to_string()
    } else {
        suffix
    }
}

pub(crate) fn check_grid(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    if rules.grid <= 1 {
        return;
    }
    for shape in drc_shapes(document) {
        let off_grid = shape
            .shape
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
                    shape.source_shape_id(),
                    rules.grid
                ),
                shape_ids: vec![shape.source_shape_id()],
                occurrence_ids: vec![shape.occurrence.clone()],
                bounds: shape.bounds().expanded(rules.grid),
                required: rules.grid,
                actual: 0.0,
            });
        }
    }
}

pub(crate) fn check_min_width(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for shape in drc_shapes(document) {
        let Some(required) = rules.min_width.get(&shape.layer()).copied() else {
            continue;
        };
        let actual = approximate_width(&shape.shape);
        if actual > 0.0 && actual < required as f64 {
            violations.push(DrcViolation {
                id: 0,
                rule: "min_width".to_string(),
                message: format!("minimum width is {required} dbu, found {actual:.1} dbu"),
                shape_ids: vec![shape.source_shape_id()],
                occurrence_ids: vec![shape.occurrence.clone()],
                bounds: shape.bounds().expanded(required),
                required,
                actual,
            });
        }
    }
}

pub(crate) fn check_max_width(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for shape in drc_shapes(document) {
        let Some(required) = rules.max_width.get(&shape.layer()).copied() else {
            continue;
        };
        let actual = approximate_max_width(&shape.shape);
        if actual > required as f64 {
            violations.push(DrcViolation {
                id: 0,
                rule: "max_width".to_string(),
                message: format!("maximum width is {required} dbu, found {actual:.1} dbu"),
                shape_ids: vec![shape.source_shape_id()],
                occurrence_ids: vec![shape.occurrence.clone()],
                bounds: shape.bounds().expanded(rules.grid.max(1)),
                required,
                actual,
            });
        }
    }
}

pub(crate) fn check_min_area(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for shape in drc_shapes(document) {
        let Some(required) = rules.min_area.get(&shape.layer()).copied() else {
            continue;
        };
        let actual = approximate_area(&shape.shape);
        if actual > 0.0 && actual < required as f64 {
            violations.push(DrcViolation {
                id: 0,
                rule: "min_area".to_string(),
                message: format!("minimum area is {required} dbu^2, found {actual:.1} dbu^2"),
                shape_ids: vec![shape.source_shape_id()],
                occurrence_ids: vec![shape.occurrence.clone()],
                bounds: shape.bounds().expanded(rules.grid.max(1)),
                required,
                actual,
            });
        }
    }
}

pub(crate) fn check_max_area(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for shape in drc_shapes(document) {
        let Some(required) = rules.max_area.get(&shape.layer()).copied() else {
            continue;
        };
        let actual = approximate_area(&shape.shape);
        if actual > required as f64 {
            violations.push(DrcViolation {
                id: 0,
                rule: "max_area".to_string(),
                message: format!("maximum area is {required} dbu^2, found {actual:.1} dbu^2"),
                shape_ids: vec![shape.source_shape_id()],
                occurrence_ids: vec![shape.occurrence.clone()],
                bounds: shape.bounds().expanded(rules.grid.max(1)),
                required,
                actual,
            });
        }
    }
}

pub(crate) fn check_derived_min_area(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_min_area {
        for shape in derived_drc_shapes(document, rules, name) {
            let actual = shape.bounds.area() as f64;
            if actual > 0.0 && actual < *required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: format!("derived_min_area.{}", derived_layer_rule_suffix(name)),
                    message: format!(
                        "derived layer {name:?} minimum area is {required} dbu^2, found {actual:.1} dbu^2"
                    ),
                    shape_ids: shape.shape_ids,
                    occurrence_ids: shape.occurrence_ids,
                    bounds: shape.bounds.expanded(rules.grid.max(1)),
                    required: *required,
                    actual,
                });
            }
        }
    }
}

pub(crate) fn check_derived_max_area(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_max_area {
        for shape in derived_drc_shapes(document, rules, name) {
            let actual = shape.bounds.area() as f64;
            if actual > *required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: format!("derived_max_area.{}", derived_layer_rule_suffix(name)),
                    message: format!(
                        "derived layer {name:?} maximum area is {required} dbu^2, found {actual:.1} dbu^2"
                    ),
                    shape_ids: shape.shape_ids,
                    occurrence_ids: shape.occurrence_ids,
                    bounds: shape.bounds.expanded(rules.grid.max(1)),
                    required: *required,
                    actual,
                });
            }
        }
    }
}

pub(crate) fn check_derived_min_width(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_min_width {
        for shape in derived_drc_shapes(document, rules, name) {
            let actual = shape.bounds.width().min(shape.bounds.height()) as f64;
            if actual > 0.0 && actual < *required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: format!("derived_min_width.{}", derived_layer_rule_suffix(name)),
                    message: format!(
                        "derived layer {name:?} minimum width is {required} dbu, found {actual:.1} dbu"
                    ),
                    shape_ids: shape.shape_ids,
                    occurrence_ids: shape.occurrence_ids,
                    bounds: shape.bounds.expanded(*required),
                    required: *required,
                    actual,
                });
            }
        }
    }
}

pub(crate) fn check_derived_max_width(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_max_width {
        for shape in derived_drc_shapes(document, rules, name) {
            let actual = shape.bounds.width().max(shape.bounds.height()) as f64;
            if actual > *required as f64 {
                violations.push(DrcViolation {
                    id: 0,
                    rule: format!("derived_max_width.{}", derived_layer_rule_suffix(name)),
                    message: format!(
                        "derived layer {name:?} maximum width is {required} dbu, found {actual:.1} dbu"
                    ),
                    shape_ids: shape.shape_ids,
                    occurrence_ids: shape.occurrence_ids,
                    bounds: shape.bounds.expanded(rules.grid.max(1)),
                    required: *required,
                    actual,
                });
            }
        }
    }
}

pub(crate) fn check_derived_spacing(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_min_spacing {
        if *required <= 0 {
            continue;
        }
        let shapes = derived_drc_shapes(document, rules, name);
        for left_index in 0..shapes.len() {
            let left = &shapes[left_index];
            for right in &shapes[left_index + 1..] {
                if left.bounds.intersects(right.bounds) {
                    continue;
                }
                let actual = left.bounds.distance_to_rect(right.bounds);
                if actual < *required as f64 {
                    let (shape_ids, occurrence_ids) = merged_derived_shape_refs(left, right);
                    violations.push(DrcViolation {
                        id: 0,
                        rule: format!("derived_min_spacing.{}", derived_layer_rule_suffix(name)),
                        message: format!(
                            "derived layer {name:?} spacing is {actual:.1} dbu, below required {required} dbu"
                        ),
                        shape_ids,
                        occurrence_ids,
                        bounds: left.bounds.union(right.bounds).expanded(*required),
                        required: *required,
                        actual,
                    });
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DerivedDrcBoundaryEdge {
    pub(crate) shape_index: usize,
    pub(crate) shape_ids: Vec<ShapeId>,
    pub(crate) occurrence_ids: Vec<ShapeOccurrenceId>,
    pub(crate) axis: EdgeAxis,
    pub(crate) fixed: Coord,
    pub(crate) start: Coord,
    pub(crate) end: Coord,
}

pub(crate) fn check_derived_edge_spacing(
    document: &Document,
    rules: &RuleDeck,
    violations: &mut Vec<DrcViolation>,
) {
    for (name, required) in &rules.derived_min_edge_spacing {
        if *required <= 0 {
            continue;
        }
        let edges = derived_drc_shapes(document, rules, name)
            .iter()
            .enumerate()
            .flat_map(|(index, shape)| derived_shape_boundary_edges(index, shape))
            .collect::<Vec<_>>();
        for left_index in 0..edges.len() {
            let left = &edges[left_index];
            for right in &edges[left_index + 1..] {
                if left.shape_index == right.shape_index || left.axis != right.axis {
                    continue;
                }
                let distance = (left.fixed - right.fixed).abs();
                if distance == 0 || distance >= *required {
                    continue;
                }
                let overlap_start = left.start.max(right.start);
                let overlap_end = left.end.min(right.end);
                if overlap_end <= overlap_start {
                    continue;
                }
                let (shape_ids, occurrence_ids) = merged_derived_edge_refs(left, right);
                violations.push(DrcViolation {
                    id: 0,
                    rule: format!(
                        "derived_min_edge_spacing.{}",
                        derived_layer_rule_suffix(name)
                    ),
                    message: format!(
                        "derived layer {name:?} parallel edge spacing is {distance} dbu, below required {required} dbu"
                    ),
                    shape_ids,
                    occurrence_ids,
                    bounds: derived_edge_pair_marker_bounds(
                        left,
                        right,
                        overlap_start,
                        overlap_end,
                        *required,
                    ),
                    required: *required,
                    actual: distance as f64,
                });
            }
        }
    }
}

pub(crate) fn derived_shape_boundary_edges(
    shape_index: usize,
    shape: &DerivedDrcShape,
) -> Vec<DerivedDrcBoundaryEdge> {
    let corners = shape.bounds.corners();
    [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ]
    .into_iter()
    .filter_map(|(a, b)| derived_boundary_edge_from_points(shape_index, shape, a, b))
    .collect()
}

pub(crate) fn derived_boundary_edge_from_points(
    shape_index: usize,
    shape: &DerivedDrcShape,
    a: Point,
    b: Point,
) -> Option<DerivedDrcBoundaryEdge> {
    if a == b {
        return None;
    }
    let (axis, fixed, start, end) = if a.y == b.y {
        (EdgeAxis::Horizontal, a.y, a.x.min(b.x), a.x.max(b.x))
    } else if a.x == b.x {
        (EdgeAxis::Vertical, a.x, a.y.min(b.y), a.y.max(b.y))
    } else {
        return None;
    };
    (end > start).then(|| DerivedDrcBoundaryEdge {
        shape_index,
        shape_ids: shape.shape_ids.clone(),
        occurrence_ids: shape.occurrence_ids.clone(),
        axis,
        fixed,
        start,
        end,
    })
}

pub(crate) fn merged_derived_edge_refs(
    left: &DerivedDrcBoundaryEdge,
    right: &DerivedDrcBoundaryEdge,
) -> (Vec<ShapeId>, Vec<ShapeOccurrenceId>) {
    let mut seen = BTreeSet::new();
    let mut shape_ids = Vec::new();
    let mut occurrence_ids = Vec::new();
    for (shape_id, occurrence) in left
        .shape_ids
        .iter()
        .chain(right.shape_ids.iter())
        .copied()
        .zip(
            left.occurrence_ids
                .iter()
                .chain(right.occurrence_ids.iter())
                .cloned(),
        )
    {
        if seen.insert(occurrence_stable_key(&occurrence)) {
            shape_ids.push(shape_id);
            occurrence_ids.push(occurrence);
        }
    }
    (shape_ids, occurrence_ids)
}

pub(crate) fn derived_edge_pair_marker_bounds(
    left: &DerivedDrcBoundaryEdge,
    right: &DerivedDrcBoundaryEdge,
    overlap_start: Coord,
    overlap_end: Coord,
    required: Coord,
) -> Rect {
    let raw = match left.axis {
        EdgeAxis::Horizontal => Rect::new(
            Point::new(overlap_start, left.fixed),
            Point::new(overlap_end, right.fixed),
        ),
        EdgeAxis::Vertical => Rect::new(
            Point::new(left.fixed, overlap_start),
            Point::new(right.fixed, overlap_end),
        ),
    };
    raw.expanded(required.max(1))
}

pub(crate) fn merged_derived_shape_refs(
    left: &DerivedDrcShape,
    right: &DerivedDrcShape,
) -> (Vec<ShapeId>, Vec<ShapeOccurrenceId>) {
    let mut seen = BTreeSet::new();
    let mut shape_ids = Vec::new();
    let mut occurrence_ids = Vec::new();
    for (shape_id, occurrence) in left
        .shape_ids
        .iter()
        .chain(right.shape_ids.iter())
        .copied()
        .zip(
            left.occurrence_ids
                .iter()
                .chain(right.occurrence_ids.iter())
                .cloned(),
        )
    {
        if seen.insert(occurrence_stable_key(&occurrence)) {
            shape_ids.push(shape_id);
            occurrence_ids.push(occurrence);
        }
    }
    (shape_ids, occurrence_ids)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EdgeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub(crate) struct DrcBoundaryEdge {
    pub(crate) layer: LayerId,
    pub(crate) shape_id: ShapeId,
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) axis: EdgeAxis,
    pub(crate) fixed: Coord,
    pub(crate) start: Coord,
    pub(crate) end: Coord,
}
