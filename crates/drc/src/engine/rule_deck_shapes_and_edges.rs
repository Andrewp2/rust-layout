#![allow(unused_imports)]
use super::*;
use layout_model::{CellId, MarkerSignoffRecord, Transform};

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
    pub marker_state_count: usize,
    pub skipped_line_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibreRveImportResult {
    pub violations: Vec<DrcViolation>,
    pub marker_states: BTreeMap<String, MarkerState>,
    pub report: CalibreRveImportReport,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KlayoutRdbImportReport {
    pub marker_count: usize,
    pub marker_state_count: usize,
    pub skipped_item_count: usize,
    pub warnings: Vec<String>,
    pub description: Option<String>,
    pub original_file: Option<String>,
    pub generator: Option<String>,
    pub top_cell: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KlayoutRdbImportResult {
    pub violations: Vec<DrcViolation>,
    pub marker_states: BTreeMap<String, MarkerState>,
    pub findings: Vec<DrcValidationFinding>,
    pub report: KlayoutRdbImportReport,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KlayoutRdbImportContext {
    pub top_cell_names: BTreeSet<String>,
    pub cell_transforms_to_top: BTreeMap<String, Vec<KlayoutRdbTransformSpec>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KlayoutRdbTransformSpec {
    pub matrix: [f64; 4],
    pub translation: [f64; 2],
}

impl KlayoutRdbImportContext {
    pub fn from_document(document: &Document) -> Self {
        let mut context = Self::default();
        let Some(top_cell) = document.cells.get(&document.top_cell) else {
            return context;
        };
        context.top_cell_names.insert(top_cell.name.clone());
        collect_klayout_rdb_document_cell_transforms(
            document,
            top_cell.id,
            KlayoutRdbTransform::IDENTITY,
            &mut BTreeSet::new(),
            &mut context,
        );
        context
    }
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
    let mut marker_states = BTreeMap::new();
    let mut skipped_line_count = 0;
    let mut warnings = Vec::new();
    let mut current_rule = String::from("marker");
    let mut current_message = String::new();
    let mut current_state = MarkerState::default();
    let mut current_required = 0;
    let mut current_actual = 0.0;
    let mut pending_geometry: Option<PendingCalibreGeometry> = None;

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
        if let Some(mut pending) = pending_geometry.take() {
            let coords = pending.line_coords(line);
            if !coords.is_empty() {
                pending.extend_coords(coords);
                if let Some(bounds) = pending.bounds() {
                    let marker_state =
                        combined_calibre_marker_state(&current_state, &pending.marker_state);
                    push_calibre_violation(
                        &mut violations,
                        &mut marker_states,
                        &current_rule,
                        &current_message,
                        &marker_state,
                        current_required,
                        current_actual,
                        bounds,
                    );
                } else {
                    pending_geometry = Some(pending);
                }
                continue;
            }
            if pending.consume_result_property_line(line) {
                pending_geometry = Some(pending);
                continue;
            }
            push_or_warn_finished_calibre_geometry(
                &mut violations,
                &mut marker_states,
                &mut skipped_line_count,
                &mut warnings,
                &current_rule,
                &current_message,
                &current_state,
                current_required,
                current_actual,
                &pending,
            );
        }

        let geometry_line = calibre_line_without_geometry_record_index(line).unwrap_or(line);
        let geometry_kind = calibre_geometry_kind_from_line(geometry_line);
        let marker_bounds =
            geometry_kind.and_then(|kind| calibre_bounds_for_kind(kind, geometry_line));

        if let Some(bounds) = marker_bounds {
            push_calibre_violation(
                &mut violations,
                &mut marker_states,
                &current_rule,
                &current_message,
                &current_state,
                current_required,
                current_actual,
                bounds,
            );
            continue;
        }
        if let Some(kind) = geometry_kind
            && let Some(pending) =
                PendingCalibreGeometry::from_geometry_line(kind, geometry_line, line_number)
        {
            pending_geometry = Some(pending);
            continue;
        }

        if first_lower.starts_with('@') {
            let content = line.trim_start_matches('@').trim();
            if parse_calibre_rve_measure_line(content, &mut current_required, &mut current_actual) {
                continue;
            }
            if parse_calibre_rve_metadata_line(content, &mut current_state) {
                continue;
            }
            append_calibre_rve_message(&mut current_message, content);
            if current_rule == "marker"
                && let Some(rule) = current_message.split_whitespace().next()
            {
                current_rule = rule.to_string();
            }
            continue;
        }

        if append_calibre_rve_labeled_message(&mut current_message, line) {
            continue;
        }
        if parse_calibre_rve_measure_line(line, &mut current_required, &mut current_actual) {
            continue;
        }
        if parse_calibre_rve_metadata_line(line, &mut current_state) {
            continue;
        }
        if current_rule == "marker"
            && current_message.is_empty()
            && current_state == MarkerState::default()
            && is_calibre_rve_database_header_line(line)
        {
            continue;
        }
        if is_calibre_rve_rule_summary_line(line) {
            continue;
        }
        if is_calibre_rve_record_delimiter_line(line) {
            continue;
        }
        if let Some(rule) = calibre_rule_name_from_line(line) {
            current_rule = rule;
            current_message.clear();
            current_state = MarkerState::default();
            current_required = 0;
            current_actual = 0.0;
        } else {
            skipped_line_count += 1;
            if warnings.len() < 8 {
                warnings.push(format!(
                    "line {line_number}: skipped unsupported marker syntax"
                ));
            }
        }
    }
    if let Some(pending) = pending_geometry.take() {
        push_or_warn_finished_calibre_geometry(
            &mut violations,
            &mut marker_states,
            &mut skipped_line_count,
            &mut warnings,
            &current_rule,
            &current_message,
            &current_state,
            current_required,
            current_actual,
            &pending,
        );
    }

    CalibreRveImportResult {
        report: CalibreRveImportReport {
            marker_count: violations.len(),
            marker_state_count: marker_states.len(),
            skipped_line_count,
            warnings,
        },
        violations,
        marker_states,
    }
}

pub fn import_klayout_rdb_markers(text: &str) -> Result<KlayoutRdbImportResult, String> {
    import_klayout_rdb_markers_with_context(text, &KlayoutRdbImportContext::default())
}

pub fn import_klayout_rdb_markers_with_context(
    text: &str,
    context: &KlayoutRdbImportContext,
) -> Result<KlayoutRdbImportResult, String> {
    let document = roxmltree::Document::parse(text.trim_start())
        .map_err(|error| format!("invalid XML: {error}"))?;
    let root = document.root_element();
    if !root.has_tag_name("report-database") {
        return Err(format!(
            "expected report-database root, found {}",
            root.tag_name().name()
        ));
    }

    let root_metadata = klayout_rdb_root_metadata(root);
    let mut violations = Vec::new();
    let mut marker_states = BTreeMap::new();
    let mut findings = Vec::new();
    let mut skipped_item_count = 0usize;
    let mut warnings = Vec::new();
    let cell_metadata = klayout_rdb_cells(root);
    let category_descriptions = klayout_rdb_category_descriptions(root);
    let tag_descriptions = klayout_rdb_tag_descriptions(root);

    let Some(items_node) = klayout_rdb_child(root, "items") else {
        return Ok(KlayoutRdbImportResult {
            violations,
            marker_states,
            findings,
            report: KlayoutRdbImportReport {
                marker_count: 0,
                marker_state_count: 0,
                skipped_item_count: 0,
                warnings,
                description: root_metadata.description,
                original_file: root_metadata.original_file,
                generator: root_metadata.generator,
                top_cell: root_metadata.top_cell,
            },
        });
    };

    for (item_index, item_node) in items_node
        .children()
        .filter(|node| node.has_tag_name("item"))
        .enumerate()
    {
        let item_number = item_index + 1;
        let category =
            klayout_rdb_child_text(item_node, "category").unwrap_or_else(|| "marker".to_string());
        let cell = klayout_rdb_child_text(item_node, "cell");
        let tags = klayout_rdb_child_text(item_node, "tags").unwrap_or_default();
        let visited = klayout_rdb_child_text(item_node, "visited")
            .and_then(|value| parse_calibre_rve_bool(&value))
            .unwrap_or(false);
        let multiplicity = klayout_rdb_child_text(item_node, "multiplicity");
        let comment = klayout_rdb_child_text(item_node, "comment");
        let image = klayout_rdb_child_text(item_node, "image");
        let mut bounds = None::<Rect>;
        let mut geometry_values = Vec::new();
        let mut item_references = Vec::new();
        let mut scalar_values = Vec::new();
        let mut text_values = Vec::new();
        let mut ordered_values = Vec::new();
        let mut raw_values = Vec::new();
        let item_cell_metadata = cell.as_deref().and_then(|cell| {
            klayout_rdb_cell_metadata(cell, &cell_metadata).map(|(_, metadata)| metadata)
        });

        if let Some(values_node) = klayout_rdb_child(item_node, "values") {
            for value_node in values_node
                .children()
                .filter(|node| node.has_tag_name("value"))
            {
                let value = value_node.text().unwrap_or_default().trim();
                if value.is_empty() {
                    continue;
                }
                ordered_values.push(klayout_rdb_import_ordered_value(value));
                match klayout_rdb_value(value) {
                    Ok(Some(KlayoutRdbValue::Geometry {
                        bounds: value_bounds,
                        value: geometry_value,
                        tag,
                    })) => {
                        bounds = Some(
                            bounds
                                .map(|bounds| bounds.union(value_bounds))
                                .unwrap_or(value_bounds),
                        );
                        geometry_values.push(KlayoutRdbGeometryValue {
                            value: geometry_value,
                            tag,
                        });
                    }
                    Ok(Some(KlayoutRdbValue::Reference(reference))) => {
                        if !reference.value.trim().is_empty() {
                            item_references.push(reference);
                        }
                    }
                    Ok(Some(KlayoutRdbValue::Raw(raw))) => {
                        if !raw.value.trim().is_empty() {
                            raw_values.push(raw);
                        }
                    }
                    Ok(Some(KlayoutRdbValue::Scalar(scalar))) => {
                        if klayout_rdb_scalar_value_should_preserve(&scalar) {
                            if scalar.kind == "text"
                                && scalar.tag.is_none()
                                && !scalar.value.trim().is_empty()
                            {
                                text_values.push(scalar.value.trim().to_string());
                            }
                            scalar_values.push(scalar);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let (tag, typed_value) = klayout_rdb_tagged_value(value);
                        raw_values.push(KlayoutRdbRawValue {
                            value: typed_value.to_string(),
                            tag,
                        });
                        push_klayout_rdb_warning(
                            &mut warnings,
                            format!("item {item_number}: preserved raw value `{value}` ({error})"),
                        );
                    }
                }
            }
        }

        let rule = format!(
            "klayout.{}",
            sanitize_calibre_rule_identifier(&klayout_rdb_rule_label(&category))
        );
        let message = klayout_rdb_marker_message(&category, comment.as_deref(), &text_values);
        let Some(bounds) = bounds else {
            if klayout_rdb_item_is_text_diagnostic(comment.as_deref(), &text_values) {
                findings.push(DrcValidationFinding::warning(
                    klayout_rdb_text_diagnostic_message(&category, &message),
                ));
            } else {
                skipped_item_count += 1;
                let context = text_values
                    .first()
                    .or(comment.as_ref())
                    .map(|value| format!(": {}", klayout_rdb_message_preview(value)))
                    .unwrap_or_default();
                push_klayout_rdb_warning(
                    &mut warnings,
                    format!("item {item_number}: skipped item without supported geometry{context}"),
                );
            }
            continue;
        };

        let marker_bounds = klayout_rdb_marker_bounds_in_top_cell(
            bounds,
            cell.as_deref(),
            root_metadata.top_cell.as_deref(),
            &cell_metadata,
            context,
        );
        let state = klayout_rdb_marker_state(
            &category,
            cell.as_deref(),
            &tags,
            visited,
            multiplicity.as_deref(),
            comment.as_deref(),
            image.as_deref(),
            &root_metadata,
            &geometry_values,
            &ordered_values,
            &item_references,
            &scalar_values,
            &raw_values,
            item_cell_metadata,
            &cell_metadata,
            &category_descriptions,
            &tag_descriptions,
        );
        for bounds in marker_bounds {
            let violation = DrcViolation {
                id: violations.len() + 1,
                rule: rule.clone(),
                message: message.clone(),
                shape_ids: Vec::new(),
                occurrence_ids: Vec::new(),
                bounds,
                required: 0,
                actual: 0.0,
            };
            if state != MarkerState::default() {
                marker_states.insert(violation.stable_key(), state.clone());
            }
            violations.push(violation);
        }
    }

    Ok(KlayoutRdbImportResult {
        report: KlayoutRdbImportReport {
            marker_count: violations.len(),
            marker_state_count: marker_states.len(),
            skipped_item_count,
            warnings,
            description: root_metadata.description,
            original_file: root_metadata.original_file,
            generator: root_metadata.generator,
            top_cell: root_metadata.top_cell,
        },
        violations,
        marker_states,
        findings,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum KlayoutRdbValue {
    Geometry {
        bounds: Rect,
        value: String,
        tag: Option<String>,
    },
    Reference(KlayoutRdbReferenceValue),
    Raw(KlayoutRdbRawValue),
    Scalar(KlayoutRdbScalarValue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbGeometryValue {
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbReferenceValue {
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbScalarValue {
    kind: String,
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbRawValue {
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbCategoryMetadata {
    parts: Vec<String>,
    description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbTagMetadata {
    name: String,
    description: String,
}

const KLAYOUT_RDB_RAW_VALUE_TAG_PREFIX: &str = "rdb_raw_value_";
const KLAYOUT_RDB_CELL_LAYOUT_NAME_TAG: &str = "rdb_cell_layout_name";
const KLAYOUT_RDB_CELL_REFERENCE_TAG_PREFIX: &str = "rdb_cell_reference_";
const KLAYOUT_RDB_CATEGORY_DESCRIPTION_TAG_PREFIX: &str = "rdb_category_description_";
const KLAYOUT_RDB_CATEGORY_PART_TAG_PREFIX: &str = "rdb_category_part_";
const KLAYOUT_RDB_DECLARED_CELL_TAG_PREFIX: &str = "rdb_declared_cell_";
const KLAYOUT_RDB_DECLARED_CATEGORY_TAG_PREFIX: &str = "rdb_declared_category_";
const KLAYOUT_RDB_DECLARED_TAG_PREFIX: &str = "rdb_declared_tag_";
const KLAYOUT_RDB_GEOMETRY_VALUE_TAG_PREFIX: &str = "rdb_geometry_value_";
const KLAYOUT_RDB_ITEM_ORDERED_VALUE_TAG_PREFIX: &str = "rdb_item_ordered_value_";
const KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX: &str = "rdb_item_value_";
const KLAYOUT_RDB_ITEM_REFERENCE_TAG_PREFIX: &str = "rdb_item_reference_";
const KLAYOUT_RDB_MULTIPLICITY_TAG: &str = "rdb_multiplicity";
const KLAYOUT_RDB_REPORT_DESCRIPTION_TAG: &str = "rdb_report_description";
const KLAYOUT_RDB_REPORT_DESCRIPTION_EMPTY_TAG: &str = "rdb_report_description_empty";
const KLAYOUT_RDB_REPORT_GENERATOR_TAG: &str = "rdb_report_generator";
const KLAYOUT_RDB_REPORT_GENERATOR_EMPTY_TAG: &str = "rdb_report_generator_empty";
const KLAYOUT_RDB_REPORT_ORIGINAL_FILE_TAG: &str = "rdb_report_original_file";
const KLAYOUT_RDB_REPORT_ORIGINAL_FILE_EMPTY_TAG: &str = "rdb_report_original_file_empty";
const KLAYOUT_RDB_REPORT_TOP_CELL_TAG: &str = "rdb_report_top_cell";
const KLAYOUT_RDB_REPORT_TOP_CELL_EMPTY_TAG: &str = "rdb_report_top_cell_empty";
const KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX: &str = "rdb_tag_description_";
const KLAYOUT_RDB_TAG_NAME_TAG_PREFIX: &str = "rdb_tag_name_";
const KLAYOUT_RDB_TEXT_RAW_VALUE_PREFIX: &str = "glassworks-raw-value: ";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct KlayoutRdbRootMetadata {
    description: Option<String>,
    description_empty: bool,
    original_file: Option<String>,
    original_file_empty: bool,
    generator: Option<String>,
    generator_empty: bool,
    top_cell: Option<String>,
    top_cell_empty: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct KlayoutRdbCellMetadata {
    name: String,
    variant: Option<String>,
    layout_name: Option<String>,
    references: Vec<KlayoutRdbCellReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KlayoutRdbCellKey {
    name: String,
    variant: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbCellReference {
    parent: String,
    trans: String,
}

impl KlayoutRdbCellKey {
    fn new(name: impl Into<String>, variant: Option<String>) -> Self {
        let variant = variant
            .map(|variant| variant.trim().to_string())
            .filter(|variant| !variant.is_empty());
        Self {
            name: name.into().trim().to_string(),
            variant,
        }
    }

    fn qualified_name(&self) -> String {
        klayout_rdb_qualified_cell_name(&self.name, self.variant.as_deref())
    }

    fn is_literal_match(&self, cell: &str) -> bool {
        self.variant.is_none() && self.name.trim() == cell.trim()
    }

    fn is_qualified_match(&self, cell: &str) -> bool {
        self.qualified_name().trim() == cell.trim()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct KlayoutRdbTransform {
    matrix: [f64; 4],
    translation: [f64; 2],
}

impl KlayoutRdbTransform {
    const IDENTITY: Self = Self {
        matrix: [1.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0],
    };

    fn from_spec(spec: KlayoutRdbTransformSpec) -> Self {
        Self {
            matrix: spec.matrix,
            translation: spec.translation,
        }
    }

    fn into_spec(self) -> KlayoutRdbTransformSpec {
        KlayoutRdbTransformSpec {
            matrix: self.matrix,
            translation: self.translation,
        }
    }

    fn from_layout_transform(transform: Transform) -> Self {
        Self {
            matrix: [
                f64::from(transform.matrix[0]),
                f64::from(transform.matrix[1]),
                f64::from(transform.matrix[2]),
                f64::from(transform.matrix[3]),
            ],
            translation: [
                transform.translation.dx as f64,
                transform.translation.dy as f64,
            ],
        }
    }

    fn from_translation(dx: Coord, dy: Coord) -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 1.0],
            translation: [dx as f64, dy as f64],
        }
    }

    fn compose(self, child: Self) -> Self {
        let [a, b, c, d] = self.matrix;
        let [e, f, g, h] = child.matrix;
        let child_translation = self.apply_vector(child.translation);
        Self {
            matrix: [a * e + b * g, a * f + b * h, c * e + d * g, c * f + d * h],
            translation: [
                child_translation[0] + self.translation[0],
                child_translation[1] + self.translation[1],
            ],
        }
    }

    fn apply_rect(self, rect: Rect) -> Rect {
        let corners = rect.corners().map(|point| self.apply_point(point));
        Rect::from_points(&corners)
            .map(nonzero_marker_bounds)
            .unwrap_or_else(|| nonzero_marker_bounds(rect))
    }

    fn apply_point(self, point: Point) -> Point {
        let [a, b, c, d] = self.matrix;
        Point::new(
            round_klayout_rdb_transform_coord(
                a * point.x as f64 + b * point.y as f64 + self.translation[0],
            ),
            round_klayout_rdb_transform_coord(
                c * point.x as f64 + d * point.y as f64 + self.translation[1],
            ),
        )
    }

    fn apply_vector(self, vector: [f64; 2]) -> [f64; 2] {
        let [a, b, c, d] = self.matrix;
        [a * vector[0] + b * vector[1], c * vector[0] + d * vector[1]]
    }
}

fn round_klayout_rdb_transform_coord(value: f64) -> Coord {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(Coord::MIN as f64, Coord::MAX as f64) as Coord
}

fn collect_klayout_rdb_document_cell_transforms(
    document: &Document,
    cell_id: CellId,
    transform_to_top: KlayoutRdbTransform,
    stack: &mut BTreeSet<CellId>,
    context: &mut KlayoutRdbImportContext,
) {
    if !stack.insert(cell_id) {
        return;
    }
    let Some(cell) = document.cells.get(&cell_id) else {
        stack.remove(&cell_id);
        return;
    };
    context
        .cell_transforms_to_top
        .entry(cell.name.clone())
        .or_default()
        .push(transform_to_top.into_spec());
    for instance in cell.instances.values() {
        let array = instance.array.normalized();
        for row in 0..array.rows {
            for column in 0..array.columns {
                let offset = array.element_offset(column, row);
                let child_transform = transform_to_top.compose(
                    KlayoutRdbTransform::from_translation(offset.dx, offset.dy).compose(
                        KlayoutRdbTransform::from_layout_transform(instance.transform),
                    ),
                );
                collect_klayout_rdb_document_cell_transforms(
                    document,
                    instance.cell,
                    child_transform,
                    stack,
                    context,
                );
            }
        }
    }
    stack.remove(&cell_id);
}

fn klayout_rdb_child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    tag_name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children().find(|child| child.has_tag_name(tag_name))
}

fn klayout_rdb_child_text(node: roxmltree::Node<'_, '_>, tag_name: &str) -> Option<String> {
    klayout_rdb_child_raw_text(node, tag_name).filter(|value| !value.is_empty())
}

fn klayout_rdb_child_raw_text(node: roxmltree::Node<'_, '_>, tag_name: &str) -> Option<String> {
    klayout_rdb_child(node, tag_name)
        .and_then(|child| child.text())
        .map(str::trim)
        .map(str::to_string)
}

fn klayout_rdb_child_is_explicit_empty(
    node: roxmltree::Node<'_, '_>,
    tag_name: &str,
) -> bool {
    klayout_rdb_child(node, tag_name)
        .map(|child| child.text().unwrap_or_default().trim().is_empty())
        .unwrap_or(false)
}

fn klayout_rdb_root_metadata(root: roxmltree::Node<'_, '_>) -> KlayoutRdbRootMetadata {
    KlayoutRdbRootMetadata {
        description: klayout_rdb_child_text(root, "description"),
        description_empty: klayout_rdb_child_is_explicit_empty(root, "description"),
        original_file: klayout_rdb_child_text(root, "original-file"),
        original_file_empty: klayout_rdb_child_is_explicit_empty(root, "original-file"),
        generator: klayout_rdb_child_text(root, "generator"),
        generator_empty: klayout_rdb_child_is_explicit_empty(root, "generator"),
        top_cell: klayout_rdb_child_text(root, "top-cell"),
        top_cell_empty: klayout_rdb_child_is_explicit_empty(root, "top-cell"),
    }
}

fn klayout_rdb_cells(
    root: roxmltree::Node<'_, '_>,
) -> BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata> {
    let mut cells = BTreeMap::new();
    let Some(cells_node) = klayout_rdb_child(root, "cells") else {
        return cells;
    };
    for cell_node in cells_node
        .children()
        .filter(|node| node.has_tag_name("cell"))
    {
        let Some(name) = klayout_rdb_child_raw_text(cell_node, "name") else {
            continue;
        };
        let variant = klayout_rdb_child_text(cell_node, "variant");
        let cell_key = KlayoutRdbCellKey::new(&name, variant.clone());
        let mut metadata = KlayoutRdbCellMetadata {
            name,
            variant,
            layout_name: klayout_rdb_child_text(cell_node, "layout-name"),
            ..KlayoutRdbCellMetadata::default()
        };
        if let Some(references_node) = klayout_rdb_child(cell_node, "references") {
            for reference_node in references_node
                .children()
                .filter(|node| node.has_tag_name("ref") || node.has_tag_name("reference"))
            {
                let parent = klayout_rdb_child_text(reference_node, "parent").unwrap_or_default();
                let trans = klayout_rdb_child_text(reference_node, "trans").unwrap_or_default();
                if !parent.is_empty() || !trans.is_empty() {
                    metadata
                        .references
                        .push(KlayoutRdbCellReference { parent, trans });
                }
            }
        }
        if !cell_key.name.trim().is_empty() || metadata != KlayoutRdbCellMetadata::default() {
            cells.insert(cell_key, metadata);
        }
    }
    cells
}

fn klayout_rdb_qualified_cell_name(name: &str, variant: Option<&str>) -> String {
    let name = name.trim();
    match variant.map(str::trim).filter(|variant| !variant.is_empty()) {
        Some(variant) => format!("{name}:{variant}"),
        None => name.to_string(),
    }
}

fn klayout_rdb_cell_metadata<'a>(
    cell: &str,
    cell_metadata: &'a BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
) -> Option<(&'a KlayoutRdbCellKey, &'a KlayoutRdbCellMetadata)> {
    let cell = cell.trim();
    if cell.is_empty() {
        return None;
    }
    if let Some((key, metadata)) = cell_metadata
        .iter()
        .find(|(key, _)| key.is_literal_match(cell))
    {
        return Some((key, metadata));
    }
    let mut qualified_matches = cell_metadata
        .iter()
        .filter(|(key, _)| key.is_qualified_match(cell));
    let first_qualified = qualified_matches.next();
    if let Some(first) = first_qualified
        && qualified_matches.next().is_none()
    {
        return Some(first);
    }
    let (name, variant) = cell
        .split_once(':')
        .map(|(name, variant)| (name.trim(), Some(variant.trim())))
        .unwrap_or((cell, None));
    let mut matches = cell_metadata.iter().filter(|(_, metadata)| {
        metadata.name.trim() == name
            && variant.is_none_or(|variant| {
                metadata
                    .variant
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|declared_variant| declared_variant == variant)
            })
    });
    let first = matches.next()?;
    (matches.next().is_none()).then_some(first)
}

fn klayout_rdb_marker_bounds_in_top_cell(
    bounds: Rect,
    cell: Option<&str>,
    top_cell: Option<&str>,
    cell_metadata: &BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
    context: &KlayoutRdbImportContext,
) -> Vec<Rect> {
    let Some(cell) = cell else {
        return vec![bounds];
    };
    let mut transforms = top_cell
        .map(|top_cell| klayout_rdb_cell_transforms_to_top(cell, top_cell, cell_metadata))
        .unwrap_or_default();
    if transforms.is_empty() {
        transforms = klayout_rdb_context_cell_transforms_to_top(cell, top_cell, context);
    }
    if transforms.is_empty() {
        return vec![bounds];
    }
    transforms
        .into_iter()
        .map(|transform| transform.apply_rect(bounds))
        .collect()
}

fn klayout_rdb_context_cell_transforms_to_top(
    cell: &str,
    top_cell: Option<&str>,
    context: &KlayoutRdbImportContext,
) -> Vec<KlayoutRdbTransform> {
    if !klayout_rdb_context_matches_top_cell(top_cell, context) {
        return Vec::new();
    }
    klayout_rdb_context_transform_specs(cell, context)
        .into_iter()
        .map(KlayoutRdbTransform::from_spec)
        .collect()
}

fn klayout_rdb_context_matches_top_cell(
    top_cell: Option<&str>,
    context: &KlayoutRdbImportContext,
) -> bool {
    let Some(top_cell) = top_cell
        .map(str::trim)
        .filter(|top_cell| !top_cell.is_empty())
    else {
        return !context.top_cell_names.is_empty();
    };
    context
        .top_cell_names
        .iter()
        .any(|name| klayout_rdb_cell_name_matches(top_cell, name))
}

fn klayout_rdb_context_transform_specs(
    cell: &str,
    context: &KlayoutRdbImportContext,
) -> Vec<KlayoutRdbTransformSpec> {
    let cell = cell.trim();
    context
        .cell_transforms_to_top
        .get(cell)
        .cloned()
        .or_else(|| {
            let (name, _) = cell.split_once(':')?;
            context.cell_transforms_to_top.get(name.trim()).cloned()
        })
        .unwrap_or_default()
}

fn klayout_rdb_cell_transforms_to_top(
    cell: &str,
    top_cell: &str,
    cell_metadata: &BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
) -> Vec<KlayoutRdbTransform> {
    let cell = cell.trim();
    let top_cell = top_cell.trim();
    if cell.is_empty()
        || top_cell.is_empty()
        || klayout_rdb_cell_is_top(cell, top_cell, cell_metadata)
    {
        return Vec::new();
    }
    klayout_rdb_cell_transforms_to_top_inner(
        cell,
        top_cell,
        cell_metadata,
        &mut std::collections::BTreeSet::new(),
    )
    .into_iter()
    .filter(|transform| *transform != KlayoutRdbTransform::IDENTITY)
    .collect()
}

fn klayout_rdb_cell_transforms_to_top_inner(
    cell: &str,
    top_cell: &str,
    cell_metadata: &BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
    visited: &mut std::collections::BTreeSet<KlayoutRdbCellKey>,
) -> Vec<KlayoutRdbTransform> {
    if klayout_rdb_cell_is_top(cell, top_cell, cell_metadata) {
        return vec![KlayoutRdbTransform::IDENTITY];
    }
    let Some((metadata_key, metadata)) = klayout_rdb_cell_metadata(cell, cell_metadata) else {
        return Vec::new();
    };
    if !visited.insert(metadata_key.clone()) {
        return Vec::new();
    }
    let mut transforms = Vec::new();
    for reference in &metadata.references {
        let parent = reference.parent.trim();
        if parent.is_empty() {
            continue;
        }
        let Some(child_to_parent) = klayout_rdb_reference_transform(&reference.trans) else {
            continue;
        };
        if klayout_rdb_cell_is_top(parent, top_cell, cell_metadata) {
            transforms.push(child_to_parent);
            continue;
        }
        let mut branch_visited = visited.clone();
        for parent_to_top in klayout_rdb_cell_transforms_to_top_inner(
            parent,
            top_cell,
            cell_metadata,
            &mut branch_visited,
        ) {
            transforms.push(parent_to_top.compose(child_to_parent));
        }
    }
    transforms
}

fn klayout_rdb_cell_is_top(
    cell: &str,
    top_cell: &str,
    cell_metadata: &BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
) -> bool {
    let cell = cell.trim();
    let top_cell = top_cell.trim();
    if cell == top_cell {
        return true;
    }
    if let Some((key, _)) = klayout_rdb_cell_metadata(cell, cell_metadata) {
        return key.name.trim() == top_cell
            && (key.variant.is_some() || key.qualified_name().trim() == top_cell);
    }
    klayout_rdb_cell_name_matches(cell, top_cell)
}

fn klayout_rdb_cell_name_matches(candidate: &str, expected: &str) -> bool {
    let candidate = candidate.trim();
    let expected = expected.trim();
    candidate == expected
        || candidate
            .split_once(':')
            .is_some_and(|(name, _)| name.trim() == expected)
        || expected
            .split_once(':')
            .is_some_and(|(name, _)| name.trim() == candidate)
}

fn klayout_rdb_reference_transform(value: &str) -> Option<KlayoutRdbTransform> {
    let mut matrix = KlayoutRdbTransform::IDENTITY.matrix;
    let mut translation = [0.0, 0.0];
    let mut magnification = 1.0;
    for token in value
        .trim()
        .trim_matches(|character| matches!(character, '(' | ')'))
        .split_whitespace()
    {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let normalized = token.to_ascii_lowercase();
        if let Some(scale) = normalized.strip_prefix('*') {
            let scale = scale.parse::<f64>().ok()?;
            if !scale.is_finite() {
                return None;
            }
            magnification = scale;
            continue;
        }
        if let Some(orientation) = klayout_rdb_reference_orientation_matrix(&normalized) {
            matrix = orientation;
            continue;
        }
        if let Some(point) = klayout_rdb_point(token) {
            translation = [point.x as f64, point.y as f64];
            continue;
        }
        return None;
    }
    Some(KlayoutRdbTransform {
        matrix: [
            matrix[0] * magnification,
            matrix[1] * magnification,
            matrix[2] * magnification,
            matrix[3] * magnification,
        ],
        translation,
    })
}

fn klayout_rdb_reference_orientation_matrix(value: &str) -> Option<[f64; 4]> {
    if let Some(angle) = value.strip_prefix('r') {
        let radians = angle.parse::<f64>().ok()?.to_radians();
        if !radians.is_finite() {
            return None;
        }
        let cos = radians.cos();
        let sin = radians.sin();
        return Some([cos, -sin, sin, cos]);
    }
    if let Some(axis) = value.strip_prefix('m') {
        let radians = axis.parse::<f64>().ok()?.to_radians();
        if !radians.is_finite() {
            return None;
        }
        let cos = (2.0 * radians).cos();
        let sin = (2.0 * radians).sin();
        return Some([cos, sin, sin, -cos]);
    }
    None
}

fn klayout_rdb_category_descriptions(
    root: roxmltree::Node<'_, '_>,
) -> BTreeMap<String, KlayoutRdbCategoryMetadata> {
    let mut descriptions = BTreeMap::new();
    if let Some(categories_node) = klayout_rdb_child(root, "categories") {
        collect_klayout_rdb_category_descriptions(
            categories_node,
            &mut Vec::new(),
            &mut descriptions,
        );
    }
    descriptions
}

fn klayout_rdb_tag_descriptions(
    root: roxmltree::Node<'_, '_>,
) -> BTreeMap<String, KlayoutRdbTagMetadata> {
    let mut descriptions = BTreeMap::new();
    let Some(tags_node) = klayout_rdb_child(root, "tags") else {
        return descriptions;
    };
    for tag_node in tags_node.children().filter(|node| node.has_tag_name("tag")) {
        let Some(name) = klayout_rdb_child_text(tag_node, "name") else {
            continue;
        };
        let key = sanitize_calibre_metadata_key(&name);
        let Some(tag) = klayout_rdb_export_tag_name_for_key(&key) else {
            continue;
        };
        let description = klayout_rdb_child_text(tag_node, "description").unwrap_or_default();
        descriptions.insert(tag.to_string(), KlayoutRdbTagMetadata { name, description });
    }
    descriptions
}

fn collect_klayout_rdb_category_descriptions(
    node: roxmltree::Node<'_, '_>,
    prefix: &mut Vec<String>,
    descriptions: &mut BTreeMap<String, KlayoutRdbCategoryMetadata>,
) {
    for category_node in node.children().filter(|node| node.has_tag_name("category")) {
        let Some(name) = klayout_rdb_child_text(category_node, "name")
            .map(|name| klayout_rdb_unquote_value(&name))
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        prefix.push(name);
        if let Some(description) = klayout_rdb_child_text(category_node, "description") {
            descriptions.insert(
                prefix.join("/"),
                KlayoutRdbCategoryMetadata {
                    parts: prefix.clone(),
                    description,
                },
            );
        }
        if let Some(children) = klayout_rdb_child(category_node, "categories") {
            collect_klayout_rdb_category_descriptions(children, prefix, descriptions);
        }
        prefix.pop();
    }
}

fn klayout_rdb_value(value: &str) -> Result<Option<KlayoutRdbValue>, String> {
    let (tag, typed_value) = klayout_rdb_tagged_value(value);
    let Some((raw_kind, raw_body)) = typed_value.split_once(':') else {
        return Err("missing value type".to_string());
    };
    let kind = sanitize_calibre_metadata_key(raw_kind);
    let body = raw_body.trim();
    match kind.as_str() {
        "text" => {
            if let Some(value) = klayout_rdb_text_geometry_value(body, typed_value, tag.clone()) {
                Ok(Some(value))
            } else {
                let value = klayout_rdb_unquote_text_value(body);
                if let Some(reference) = klayout_rdb_text_reference_value(&value) {
                    return Ok(Some(KlayoutRdbValue::Reference(KlayoutRdbReferenceValue {
                        value: reference,
                        tag,
                    })));
                }
                if let Some(raw) = klayout_rdb_text_raw_value(&value) {
                    return Ok(Some(KlayoutRdbValue::Raw(KlayoutRdbRawValue {
                        value: raw,
                        tag,
                    })));
                }
                Ok(Some(KlayoutRdbValue::Scalar(KlayoutRdbScalarValue {
                    kind: "text".to_string(),
                    value,
                    tag,
                })))
            }
        }
        "box" | "rectangle" | "rect" => {
            let points = klayout_rdb_parenthesized_points(body)?;
            if points.len() < 2 {
                return Err("box value needs two points".to_string());
            }
            Ok(Some(KlayoutRdbValue::Geometry {
                bounds: nonzero_marker_bounds(Rect::new(points[0], points[1])),
                value: typed_value.to_string(),
                tag,
            }))
        }
        "edge" => {
            let points = klayout_rdb_parenthesized_points(body)?;
            if points.len() < 2 {
                return Err("edge value needs two points".to_string());
            }
            Ok(Some(KlayoutRdbValue::Geometry {
                bounds: nonzero_marker_bounds(Rect::new(points[0], points[1]).expanded(1)),
                value: typed_value.to_string(),
                tag,
            }))
        }
        "edge_pair" | "edgepair" => {
            let points = klayout_rdb_loose_points(body);
            if points.len() < 4 {
                return Err("edge-pair value needs four points".to_string());
            }
            Rect::from_points(&points)
                .map(|bounds| nonzero_marker_bounds(bounds.expanded(1)))
                .map(|bounds| KlayoutRdbValue::Geometry {
                    bounds,
                    value: typed_value.to_string(),
                    tag,
                })
                .map(Some)
                .ok_or_else(|| "edge-pair value has no points".to_string())
        }
        "point" => {
            let points = klayout_rdb_parenthesized_points(body)?;
            let point = points
                .first()
                .copied()
                .ok_or_else(|| "point value needs one point".to_string())?;
            Ok(Some(KlayoutRdbValue::Geometry {
                bounds: nonzero_marker_bounds(Rect::new(point, point)),
                value: typed_value.to_string(),
                tag,
            }))
        }
        "reference" => Ok(Some(KlayoutRdbValue::Reference(KlayoutRdbReferenceValue {
            value: body.to_string(),
            tag,
        }))),
        "float" => {
            if body.parse::<f64>().is_err() {
                return Err("float value needs a numeric payload".to_string());
            }
            Ok(Some(KlayoutRdbValue::Scalar(KlayoutRdbScalarValue {
                kind: "float".to_string(),
                value: body.to_string(),
                tag,
            })))
        }
        "string" => Ok(Some(KlayoutRdbValue::Scalar(KlayoutRdbScalarValue {
            kind: "string".to_string(),
            value: klayout_rdb_unquote_text_value(body),
            tag,
        }))),
        "polygon" | "poly" => {
            let points = klayout_rdb_parenthesized_points(body)?;
            if points.len() < 2 {
                return Err("polygon value needs at least two points".to_string());
            }
            Rect::from_points(&points)
                .map(nonzero_marker_bounds)
                .map(|bounds| KlayoutRdbValue::Geometry {
                    bounds,
                    value: typed_value.to_string(),
                    tag,
                })
                .map(Some)
                .ok_or_else(|| "polygon value has no points".to_string())
        }
        "path" => {
            let points = klayout_rdb_parenthesized_points(body)?;
            if points.len() < 2 {
                return Err("path value needs at least two points".to_string());
            }
            let width = klayout_rdb_micron_field(body, "w").unwrap_or(0);
            let begin_extension = klayout_rdb_micron_field(body, "bx").unwrap_or(0);
            let end_extension = klayout_rdb_micron_field(body, "ex").unwrap_or(0);
            klayout_rdb_path_bounds(&points, width, begin_extension, end_extension)
                .map(|bounds| KlayoutRdbValue::Geometry {
                    bounds,
                    value: typed_value.to_string(),
                    tag,
                })
                .map(Some)
                .ok_or_else(|| "path value has no points".to_string())
        }
        "label" => {
            let geometry = klayout_rdb_label_geometry_value(body, typed_value, tag)?;
            Ok(Some(geometry))
        }
        _ => Err(format!("unsupported value type `{kind}`")),
    }
}

fn klayout_rdb_tagged_value(value: &str) -> (Option<String>, &str) {
    let value = value.trim();
    let Some(rest) = value.strip_prefix('[') else {
        return (None, value);
    };
    let mut quote = None::<char>;
    let mut escaped = false;
    for (offset, character) in rest.char_indices() {
        if escaped {
            escaped = false;
        } else if quote.is_some() && character == '\\' {
            escaped = true;
        } else if matches!(character, '"' | '\'') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if quote.is_none() && character == ']' {
            let content = &rest[..offset];
            let tag = klayout_rdb_first_quoted_string(content).or_else(|| {
                let tag = content.trim();
                let tag = tag.strip_prefix('#').unwrap_or(tag).trim();
                (!tag.is_empty()).then(|| tag.to_string())
            });
            let typed_value = rest[offset + character.len_utf8()..].trim_start();
            return (tag, typed_value);
        }
    }
    (None, value)
}

fn klayout_rdb_label_geometry_value(
    body: &str,
    value: &str,
    tag: Option<String>,
) -> Result<KlayoutRdbValue, String> {
    let content = klayout_rdb_parenthesized_content(body)
        .ok_or_else(|| "label value needs parenthesized content".to_string())?;
    let _label = klayout_rdb_first_quoted_string(content).unwrap_or_default();
    let point = klayout_rdb_last_coord_pair(content)
        .ok_or_else(|| "label value needs a transform coordinate".to_string())?;
    Ok(KlayoutRdbValue::Geometry {
        bounds: nonzero_marker_bounds(Rect::new(point, point)),
        value: value.to_string(),
        tag,
    })
}

fn klayout_rdb_text_geometry_value(
    body: &str,
    value: &str,
    tag: Option<String>,
) -> Option<KlayoutRdbValue> {
    let content = klayout_rdb_parenthesized_content(body)?;
    klayout_rdb_first_quoted_string(content)?;
    let point = klayout_rdb_last_coord_pair(content)?;
    Some(KlayoutRdbValue::Geometry {
        bounds: nonzero_marker_bounds(Rect::new(point, point)),
        value: value.to_string(),
        tag,
    })
}

fn klayout_rdb_text_reference_value(value: &str) -> Option<String> {
    let reference = value.trim().strip_prefix("reference:")?.trim();
    (!reference.is_empty()).then(|| reference.to_string())
}

fn klayout_rdb_text_raw_value(value: &str) -> Option<String> {
    let raw = value
        .trim()
        .strip_prefix(KLAYOUT_RDB_TEXT_RAW_VALUE_PREFIX)?
        .trim();
    (!raw.is_empty()).then(|| raw.to_string())
}

fn klayout_rdb_import_ordered_value(value: &str) -> String {
    let (tag, typed_value) = klayout_rdb_tagged_value(value);
    let Some((raw_kind, raw_body)) = typed_value.split_once(':') else {
        return value.to_string();
    };
    if sanitize_calibre_metadata_key(raw_kind) != "text" {
        return value.to_string();
    }
    let decoded = klayout_rdb_unquote_value(raw_body.trim());
    let Some(raw) = klayout_rdb_text_raw_value(&decoded) else {
        return value.to_string();
    };
    let tag_prefix = tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(klayout_rdb_tagged_value_prefix)
        .unwrap_or_default();
    format!("{tag_prefix}{raw}")
}

fn klayout_rdb_tagged_value_prefix(tag: &str) -> String {
    let tag = tag.trim();
    let tag = tag.strip_prefix('#').unwrap_or(tag).trim();
    if tag.is_empty() {
        String::new()
    } else if klayout_rdb_value_tag_word(tag) {
        format!("[#{tag}] ")
    } else {
        format!("[#{}] ", klayout_rdb_quoted_text(tag))
    }
}

fn klayout_rdb_value_tag_word(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn klayout_rdb_quoted_text(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn klayout_rdb_parenthesized_points(value: &str) -> Result<Vec<Point>, String> {
    let content = klayout_rdb_parenthesized_content(value)
        .ok_or_else(|| "missing parenthesized coordinate list".to_string())?;
    let points = klayout_rdb_loose_points(content);
    if points.is_empty() {
        Err("no coordinate pairs found".to_string())
    } else {
        Ok(points)
    }
}

fn klayout_rdb_loose_points(value: &str) -> Vec<Point> {
    value
        .split([';', '/'])
        .filter_map(klayout_rdb_point)
        .collect::<Vec<_>>()
}

fn klayout_rdb_path_bounds(
    points: &[Point],
    width: Coord,
    begin_extension: Coord,
    end_extension: Coord,
) -> Option<Rect> {
    let mut bounds_points = points.to_vec();
    if let Some(extended_start) = klayout_rdb_extended_path_endpoint(
        points.first().copied()?,
        points.get(1).copied()?,
        begin_extension.max(0),
    ) {
        bounds_points.push(extended_start);
    }
    if let Some(extended_end) = klayout_rdb_extended_path_endpoint(
        points[points.len().saturating_sub(1)],
        points[points.len().saturating_sub(2)],
        end_extension.max(0),
    ) {
        bounds_points.push(extended_end);
    }
    let expansion = ((width as f64) / 2.0).ceil() as Coord;
    Rect::from_points(&bounds_points)
        .map(|bounds| nonzero_marker_bounds(bounds.expanded(expansion.max(1))))
}

fn klayout_rdb_extended_path_endpoint(
    endpoint: Point,
    neighbor: Point,
    extension: Coord,
) -> Option<Point> {
    if extension == 0 {
        return None;
    }
    let dx = endpoint.x - neighbor.x;
    let dy = endpoint.y - neighbor.y;
    let length = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
    if !length.is_finite() || length == 0.0 {
        return None;
    }
    Some(Point::new(
        endpoint.x + (dx as f64 / length * extension as f64).round() as Coord,
        endpoint.y + (dy as f64 / length * extension as f64).round() as Coord,
    ))
}

fn klayout_rdb_parenthesized_content(value: &str) -> Option<&str> {
    let start = value.find('(')?;
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in value[start + 1..].char_indices() {
        if escaped {
            escaped = false;
        } else if quote.is_some() && character == '\\' {
            escaped = true;
        } else if matches!(character, '"' | '\'') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if quote.is_none() && character == ')' {
            return Some(&value[start + 1..start + 1 + offset]);
        }
    }
    None
}

fn klayout_rdb_point(value: &str) -> Option<Point> {
    let (x, y) = value.trim().split_once(',')?;
    Some(Point::new(
        parse_klayout_rdb_micron_coord(x)?,
        parse_klayout_rdb_micron_coord(y)?,
    ))
}

fn klayout_rdb_last_coord_pair(value: &str) -> Option<Point> {
    value.match_indices(',').rev().find_map(|(comma_index, _)| {
        let x = klayout_rdb_number_before(value, comma_index)?;
        let y = klayout_rdb_number_after(value, comma_index + 1)?;
        Some(Point::new(
            parse_klayout_rdb_micron_coord(x)?,
            parse_klayout_rdb_micron_coord(y)?,
        ))
    })
}

fn klayout_rdb_number_before(value: &str, end: usize) -> Option<&str> {
    let mut end = end;
    while end > 0 && value[..end].chars().next_back()?.is_ascii_whitespace() {
        end -= value[..end].chars().next_back()?.len_utf8();
    }
    let mut start = end;
    while start > 0 {
        let character = value[..start].chars().next_back()?;
        if !is_klayout_rdb_number_char(character) {
            break;
        }
        start -= character.len_utf8();
    }
    (start < end).then_some(value[start..end].trim())
}

fn klayout_rdb_number_after(value: &str, start: usize) -> Option<&str> {
    let mut start = start;
    while start < value.len() && value[start..].chars().next()?.is_ascii_whitespace() {
        start += value[start..].chars().next()?.len_utf8();
    }
    let mut end = start;
    while end < value.len() {
        let character = value[end..].chars().next()?;
        if !is_klayout_rdb_number_char(character) {
            break;
        }
        end += character.len_utf8();
    }
    (start < end).then_some(value[start..end].trim())
}

fn is_klayout_rdb_number_char(character: char) -> bool {
    character.is_ascii_digit() || matches!(character, '+' | '-' | '.' | 'e' | 'E')
}

fn parse_klayout_rdb_micron_coord(value: &str) -> Option<Coord> {
    let value = value
        .trim()
        .trim_matches(|character| matches!(character, '(' | ')'));
    if value.is_empty() {
        return None;
    }
    let value = value
        .strip_suffix("um")
        .or_else(|| value.strip_suffix("micron"))
        .or_else(|| value.strip_suffix("microns"))
        .unwrap_or(value)
        .trim();
    let coord = value.parse::<f64>().ok()? * geometry_core::DBU_PER_MICRON as f64;
    round_calibre_coord(coord)
}

fn klayout_rdb_micron_field(value: &str, key: &str) -> Option<Coord> {
    value.split_whitespace().find_map(|token| {
        let (raw_key, raw_value) = token.split_once('=')?;
        (sanitize_calibre_metadata_key(raw_key) == key)
            .then(|| parse_klayout_rdb_micron_coord(raw_value))
            .flatten()
    })
}

fn klayout_rdb_first_quoted_string(value: &str) -> Option<String> {
    let mut chars = value.char_indices();
    while let Some((_, character)) = chars.next() {
        if !matches!(character, '"' | '\'') {
            continue;
        }
        let quote = character;
        let mut output = String::new();
        let mut escaped = false;
        for (_, character) in chars.by_ref() {
            if escaped {
                output.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                return Some(output.trim().to_string());
            } else {
                output.push(character);
            }
        }
        return None;
    }
    None
}

fn klayout_rdb_marker_state(
    category: &str,
    cell: Option<&str>,
    tags: &str,
    visited: bool,
    multiplicity: Option<&str>,
    comment: Option<&str>,
    image: Option<&str>,
    root_metadata: &KlayoutRdbRootMetadata,
    geometry_values: &[KlayoutRdbGeometryValue],
    ordered_values: &[String],
    item_references: &[KlayoutRdbReferenceValue],
    scalar_values: &[KlayoutRdbScalarValue],
    raw_values: &[KlayoutRdbRawValue],
    item_cell_metadata: Option<&KlayoutRdbCellMetadata>,
    cell_metadata: &BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
    category_descriptions: &BTreeMap<String, KlayoutRdbCategoryMetadata>,
    tag_descriptions: &BTreeMap<String, KlayoutRdbTagMetadata>,
) -> MarkerState {
    let mut state = MarkerState {
        visited,
        ..MarkerState::default()
    };
    if let Some(comment) = comment.map(str::trim).filter(|comment| !comment.is_empty()) {
        state.note = Some(comment.to_string());
    }
    if let Some(multiplicity) = multiplicity
        .map(str::trim)
        .filter(|multiplicity| !multiplicity.is_empty() && *multiplicity != "1")
    {
        state.tags.insert(
            KLAYOUT_RDB_MULTIPLICITY_TAG.to_string(),
            multiplicity.to_string(),
        );
    }
    if let Some(category) =
        klayout_rdb_normalized_category(category).filter(|category| !category.is_empty())
    {
        state.tags.insert("rdb_category".to_string(), category);
    }
    for (index, part) in klayout_rdb_category_parts(category)
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .enumerate()
    {
        state.tags.insert(
            format!("{KLAYOUT_RDB_CATEGORY_PART_TAG_PREFIX}{}", index + 1),
            part,
        );
    }
    for (index, description) in
        klayout_rdb_category_description_tags(category, category_descriptions)
    {
        state.tags.insert(
            format!("{KLAYOUT_RDB_CATEGORY_DESCRIPTION_TAG_PREFIX}{index}"),
            description,
        );
    }
    for (index, metadata) in category_descriptions.values().enumerate() {
        let index = index + 1;
        for (part_index, part) in metadata
            .parts
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            state.tags.insert(
                format!(
                    "{KLAYOUT_RDB_DECLARED_CATEGORY_TAG_PREFIX}{index}_part_{}",
                    part_index + 1
                ),
                part.to_string(),
            );
        }
        let description = metadata.description.trim();
        if !description.is_empty() {
            state.tags.insert(
                format!("{KLAYOUT_RDB_DECLARED_CATEGORY_TAG_PREFIX}{index}_description"),
                description.to_string(),
            );
        }
    }
    if let Some(cell) = cell.map(str::trim).filter(|cell| !cell.is_empty()) {
        state
            .tags
            .insert("source_cell".to_string(), cell.to_string());
    }
    if let Some(image) = image.map(str::trim).filter(|image| !image.is_empty()) {
        state
            .tags
            .insert("rdb_image".to_string(), "embedded".to_string());
        state
            .tags
            .insert("rdb_image_base64".to_string(), image.to_string());
    }
    klayout_rdb_insert_root_metadata_tag(
        &mut state,
        KLAYOUT_RDB_REPORT_DESCRIPTION_TAG,
        KLAYOUT_RDB_REPORT_DESCRIPTION_EMPTY_TAG,
        root_metadata.description.as_deref(),
        root_metadata.description_empty,
    );
    klayout_rdb_insert_root_metadata_tag(
        &mut state,
        KLAYOUT_RDB_REPORT_TOP_CELL_TAG,
        KLAYOUT_RDB_REPORT_TOP_CELL_EMPTY_TAG,
        root_metadata.top_cell.as_deref(),
        root_metadata.top_cell_empty,
    );
    klayout_rdb_insert_root_metadata_tag(
        &mut state,
        KLAYOUT_RDB_REPORT_ORIGINAL_FILE_TAG,
        KLAYOUT_RDB_REPORT_ORIGINAL_FILE_EMPTY_TAG,
        root_metadata.original_file.as_deref(),
        root_metadata.original_file_empty,
    );
    klayout_rdb_insert_root_metadata_tag(
        &mut state,
        KLAYOUT_RDB_REPORT_GENERATOR_TAG,
        KLAYOUT_RDB_REPORT_GENERATOR_EMPTY_TAG,
        root_metadata.generator.as_deref(),
        root_metadata.generator_empty,
    );
    for (index, geometry) in geometry_values
        .iter()
        .filter(|geometry| !geometry.value.trim().is_empty())
        .enumerate()
    {
        let index = index + 1;
        state.tags.insert(
            format!("{KLAYOUT_RDB_GEOMETRY_VALUE_TAG_PREFIX}{index}"),
            geometry.value.trim().to_string(),
        );
        if let Some(tag) = geometry
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_GEOMETRY_VALUE_TAG_PREFIX}{index}_tag"),
                tag.to_string(),
            );
        }
    }
    for (index, value) in ordered_values
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .enumerate()
    {
        state.tags.insert(
            format!("{KLAYOUT_RDB_ITEM_ORDERED_VALUE_TAG_PREFIX}{}", index + 1),
            value.to_string(),
        );
    }
    for (index, reference) in item_references
        .iter()
        .filter(|reference| !reference.value.trim().is_empty())
        .enumerate()
    {
        let index = index + 1;
        state.tags.insert(
            format!("{KLAYOUT_RDB_ITEM_REFERENCE_TAG_PREFIX}{index}"),
            reference.value.trim().to_string(),
        );
        if let Some(tag) = reference
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_ITEM_REFERENCE_TAG_PREFIX}{index}_tag"),
                tag.to_string(),
            );
        }
    }
    for (index, scalar) in scalar_values
        .iter()
        .filter(|scalar| klayout_rdb_scalar_value_should_preserve(scalar))
        .enumerate()
    {
        let index = index + 1;
        state.tags.insert(
            format!("{KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX}{index}_type"),
            scalar.kind.trim().to_string(),
        );
        if scalar.value.is_empty() {
            state.tags.insert(
                format!("{KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX}{index}_empty"),
                "true".to_string(),
            );
        } else if scalar.value.trim().is_empty() {
            state.tags.insert(
                format!("{KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX}{index}_value_hex"),
                klayout_rdb_text_value_hex(&scalar.value),
            );
        } else {
            state.tags.insert(
                format!("{KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX}{index}_value"),
                scalar.value.clone(),
            );
        }
        if let Some(tag) = scalar
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX}{index}_tag"),
                tag.to_string(),
            );
        }
    }
    for (index, raw) in raw_values
        .iter()
        .filter(|raw| !raw.value.trim().is_empty())
        .enumerate()
    {
        let index = index + 1;
        state.tags.insert(
            format!("{KLAYOUT_RDB_RAW_VALUE_TAG_PREFIX}{index}"),
            raw.value.trim().to_string(),
        );
        if let Some(tag) = raw
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_RAW_VALUE_TAG_PREFIX}{index}_tag"),
                tag.to_string(),
            );
        }
    }
    if let Some(item_cell_metadata) = item_cell_metadata {
        if let Some(layout_name) = item_cell_metadata
            .layout_name
            .as_deref()
            .map(str::trim)
            .filter(|layout_name| !layout_name.is_empty())
        {
            state.tags.insert(
                KLAYOUT_RDB_CELL_LAYOUT_NAME_TAG.to_string(),
                layout_name.to_string(),
            );
        }
        for (index, reference) in item_cell_metadata.references.iter().enumerate() {
            let index = index + 1;
            if !reference.parent.trim().is_empty() {
                state.tags.insert(
                    format!("{KLAYOUT_RDB_CELL_REFERENCE_TAG_PREFIX}{index}_parent"),
                    reference.parent.trim().to_string(),
                );
            }
            if !reference.trans.trim().is_empty() {
                state.tags.insert(
                    format!("{KLAYOUT_RDB_CELL_REFERENCE_TAG_PREFIX}{index}_trans"),
                    reference.trans.trim().to_string(),
                );
            }
        }
    }
    for (index, metadata) in cell_metadata.values().enumerate() {
        klayout_rdb_record_declared_cell_metadata(&mut state, index + 1, metadata);
    }
    for (tag_name, metadata) in tag_descriptions {
        state.tags.insert(
            format!("{KLAYOUT_RDB_DECLARED_TAG_PREFIX}{tag_name}"),
            "true".to_string(),
        );
        let description = metadata.description.trim();
        if !description.is_empty() {
            state.tags.insert(
                format!("{KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX}{tag_name}"),
                description.to_string(),
            );
        }
        let original_name = metadata.name.trim();
        if !original_name.is_empty() && original_name != tag_name {
            state.tags.insert(
                format!("{KLAYOUT_RDB_TAG_NAME_TAG_PREFIX}{tag_name}"),
                original_name.to_string(),
            );
        }
    }
    for tag in klayout_rdb_tag_items(tags) {
        let key = sanitize_calibre_metadata_key(&tag);
        let original_tag_name = tag.trim();
        if let Some(tag_name) = klayout_rdb_export_tag_name_for_key(&key)
            && let Some(description) = tag_descriptions
                .get(tag_name)
                .map(|metadata| metadata.description.as_str())
                .map(str::trim)
                .filter(|description| !description.is_empty())
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX}{tag_name}"),
                description.to_string(),
            );
        }
        if !key.is_empty()
            && let Some(tag_name) = klayout_rdb_export_tag_name_for_key(&key)
            && !original_tag_name.is_empty()
            && original_tag_name != tag_name
        {
            state.tags.insert(
                format!("{KLAYOUT_RDB_TAG_NAME_TAG_PREFIX}{tag_name}"),
                original_tag_name.to_string(),
            );
        }
        match key.as_str() {
            "hidden" | "hide" => state.hidden = true,
            "important" => state.important = true,
            "waived" | "waive" => state.waived = true,
            "visited" => state.visited = true,
            _ if !key.is_empty() => {
                state.tags.insert(key, "true".to_string());
            }
            _ => {}
        }
    }
    state
}

fn klayout_rdb_insert_root_metadata_tag(
    state: &mut MarkerState,
    value_tag: &str,
    empty_tag: &str,
    value: Option<&str>,
    explicit_empty: bool,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        state.tags.insert(value_tag.to_string(), value.to_string());
    } else if explicit_empty {
        state
            .tags
            .insert(empty_tag.to_string(), "true".to_string());
    }
}

fn klayout_rdb_scalar_value_should_preserve(scalar: &KlayoutRdbScalarValue) -> bool {
    !scalar.kind.trim().is_empty()
        && (!scalar.value.trim().is_empty() || klayout_rdb_scalar_value_allows_empty(&scalar.kind))
}

fn klayout_rdb_scalar_value_allows_empty(kind: &str) -> bool {
    matches!(kind.trim(), "text" | "string")
}

fn klayout_rdb_text_value_hex(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn klayout_rdb_record_declared_cell_metadata(
    state: &mut MarkerState,
    index: usize,
    metadata: &KlayoutRdbCellMetadata,
) {
    let prefix = format!("{KLAYOUT_RDB_DECLARED_CELL_TAG_PREFIX}{index}_");
    state
        .tags
        .insert(format!("{prefix}name"), metadata.name.trim().to_string());
    if let Some(variant) = metadata
        .variant
        .as_deref()
        .map(str::trim)
        .filter(|variant| !variant.is_empty())
    {
        state
            .tags
            .insert(format!("{prefix}variant"), variant.to_string());
    }
    if let Some(layout_name) = metadata
        .layout_name
        .as_deref()
        .map(str::trim)
        .filter(|layout_name| !layout_name.is_empty())
    {
        state
            .tags
            .insert(format!("{prefix}layout_name"), layout_name.to_string());
    }
    for (reference_index, reference) in metadata.references.iter().enumerate() {
        let reference_index = reference_index + 1;
        if !reference.parent.trim().is_empty() {
            state.tags.insert(
                format!("{prefix}reference_{reference_index}_parent"),
                reference.parent.trim().to_string(),
            );
        }
        if !reference.trans.trim().is_empty() {
            state.tags.insert(
                format!("{prefix}reference_{reference_index}_trans"),
                reference.trans.trim().to_string(),
            );
        }
    }
}

fn klayout_rdb_tag_items(tags: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    let mut escaped = false;

    for character in tags.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                current.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            } else {
                current.push(character);
            }
        } else if matches!(character, ',' | ';') {
            push_klayout_rdb_tag_item(&mut items, &current);
            current.clear();
        } else if character == '#' && current.trim().is_empty() {
            current.push(character);
        } else if matches!(character, '"' | '\'')
            && (current.trim().is_empty() || current.trim() == "#")
        {
            quote = Some(character);
            current.clear();
        } else {
            current.push(character);
        }
    }

    if escaped {
        current.push('\\');
    }
    push_klayout_rdb_tag_item(&mut items, &current);
    items
}

fn push_klayout_rdb_tag_item(items: &mut Vec<String>, item: &str) {
    let item = klayout_rdb_normalized_tag_item(item);
    if !item.is_empty() {
        items.push(item);
    }
}

fn klayout_rdb_normalized_tag_item(item: &str) -> String {
    let item = item.trim();
    let item = item.strip_prefix('#').unwrap_or(item).trim();
    if let Some(quoted) = klayout_rdb_first_quoted_string(item)
        && item
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '"' | '\''))
    {
        return quoted;
    }
    item.to_string()
}

fn klayout_rdb_export_tag_name_for_key(key: &str) -> Option<&str> {
    match key {
        "important" => Some("important"),
        "waived" | "waive" => Some("waived"),
        "visited" => None,
        _ if !key.is_empty() => Some(key),
        _ => None,
    }
}

fn klayout_rdb_marker_message(
    category: &str,
    comment: Option<&str>,
    text_values: &[String],
) -> String {
    let mut parts = text_values
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(comment) = comment.map(str::trim).filter(|comment| !comment.is_empty())
        && !parts.iter().any(|part| part == comment)
    {
        parts.push(comment.to_string());
    }
    if parts.is_empty() {
        format!(
            "Imported KLayout RDB marker for {}",
            klayout_rdb_rule_label(category)
        )
    } else {
        parts.join("\n")
    }
}

fn klayout_rdb_item_is_text_diagnostic(comment: Option<&str>, text_values: &[String]) -> bool {
    comment.is_some_and(|comment| !comment.trim().is_empty())
        || text_values.iter().any(|value| !value.trim().is_empty())
}

fn klayout_rdb_text_diagnostic_message(category: &str, message: &str) -> String {
    let category = klayout_rdb_rule_label(category);
    if category.is_empty() {
        message.to_string()
    } else {
        format!("{category}: {message}")
    }
}

fn klayout_rdb_rule_label(category: &str) -> String {
    klayout_rdb_category_parts(category)
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn klayout_rdb_normalized_category(category: &str) -> Option<String> {
    let parts = klayout_rdb_category_parts(category);
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn klayout_rdb_category_description_tags(
    category: &str,
    descriptions: &BTreeMap<String, KlayoutRdbCategoryMetadata>,
) -> Vec<(usize, String)> {
    let parts = klayout_rdb_category_parts(category);
    (1..=parts.len())
        .filter_map(|index| {
            let description = descriptions
                .get(&parts[..index].join("/"))?
                .description
                .trim();
            (!description.is_empty()).then(|| (index, description.to_string()))
        })
        .collect()
}

fn klayout_rdb_category_parts(category: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in category.char_indices() {
        if escaped {
            escaped = false;
        } else if quote.is_some() && character == '\\' {
            escaped = true;
        } else if matches!(character, '"' | '\'') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        } else if quote.is_none() && character == '.' {
            let part = klayout_rdb_unquote_value(category[start..index].trim());
            if !part.is_empty() {
                parts.push(part);
            }
            start = index + character.len_utf8();
        }
    }
    let part = klayout_rdb_unquote_value(category[start..].trim());
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn klayout_rdb_unquote_value(value: &str) -> String {
    klayout_rdb_unquote_value_with_mode(value, true)
}

fn klayout_rdb_unquote_text_value(value: &str) -> String {
    klayout_rdb_unquote_value_with_mode(value, false)
}

fn klayout_rdb_unquote_value_with_mode(value: &str, trim_decoded: bool) -> String {
    let value = value.trim();
    if value.len() < 2 {
        return value.to_string();
    }
    let Some(first) = value.chars().next() else {
        return String::new();
    };
    let Some(last) = value.chars().next_back() else {
        return String::new();
    };
    if !matches!((first, last), ('"', '"') | ('\'', '\'')) {
        return value.to_string();
    }
    let mut output = String::new();
    let mut escaped = false;
    for character in value[first.len_utf8()..value.len() - last.len_utf8()].chars() {
        if escaped {
            output.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    if trim_decoded {
        output.trim().to_string()
    } else {
        output
    }
}

fn klayout_rdb_message_preview(value: &str) -> String {
    let mut preview = value.trim().chars().take(64).collect::<String>();
    if value.trim().chars().count() > 64 {
        preview.push_str("...");
    }
    preview
}

fn push_klayout_rdb_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < 8 {
        warnings.push(warning);
    }
}

fn push_or_warn_finished_calibre_geometry(
    violations: &mut Vec<DrcViolation>,
    marker_states: &mut BTreeMap<String, MarkerState>,
    skipped_line_count: &mut usize,
    warnings: &mut Vec<String>,
    current_rule: &str,
    current_message: &str,
    current_state: &MarkerState,
    required: Coord,
    actual: f64,
    pending: &PendingCalibreGeometry,
) {
    if let Some(bounds) = pending.finished_bounds() {
        let marker_state = combined_calibre_marker_state(current_state, &pending.marker_state);
        push_calibre_violation(
            violations,
            marker_states,
            current_rule,
            current_message,
            &marker_state,
            required,
            actual,
            bounds,
        );
    } else {
        warn_incomplete_calibre_geometry(skipped_line_count, warnings, pending);
    }
}

fn warn_incomplete_calibre_geometry(
    skipped_line_count: &mut usize,
    warnings: &mut Vec<String>,
    pending: &PendingCalibreGeometry,
) {
    *skipped_line_count += 1;
    if warnings.len() < 8 {
        warnings.push(format!(
            "line {}: skipped incomplete {} marker geometry (expected {}, found {})",
            pending.start_line,
            pending.kind.label(),
            pending.expected_coord_description(),
            pending.coords.len()
        ));
    }
}

fn append_calibre_rve_message(current_message: &mut String, content: &str) {
    let content = content.trim();
    if content.is_empty() {
        return;
    }
    if !current_message.is_empty() {
        current_message.push('\n');
    }
    current_message.push_str(content);
}

fn append_calibre_rve_labeled_message(current_message: &mut String, content: &str) -> bool {
    let Some((raw_key, raw_value)) = split_calibre_metadata_key_value(content) else {
        return false;
    };
    let key = sanitize_calibre_metadata_key(raw_key);
    if !matches!(
        key.as_str(),
        "message" | "description" | "comment" | "rve_message" | "rule_text" | "rule_description"
    ) {
        return false;
    }
    let value = unquote_calibre_metadata_value(raw_value);
    if value.is_empty() {
        return true;
    }
    append_calibre_rve_message(current_message, &value);
    true
}

fn push_calibre_violation(
    violations: &mut Vec<DrcViolation>,
    marker_states: &mut BTreeMap<String, MarkerState>,
    current_rule: &str,
    current_message: &str,
    current_state: &MarkerState,
    required: Coord,
    actual: f64,
    bounds: Rect,
) {
    let rule = format!("calibre.{}", sanitize_calibre_rule_identifier(current_rule));
    let message = if current_message.trim().is_empty() {
        format!("Imported Calibre/RVE marker for {current_rule}")
    } else {
        current_message.to_string()
    };
    let violation = DrcViolation {
        id: violations.len() + 1,
        rule,
        message,
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds,
        required,
        actual,
    };
    let marker_state = calibre_marker_state_for_violation(current_state);
    if marker_state != MarkerState::default() {
        marker_states.insert(violation.stable_key(), marker_state);
    }
    violations.push(violation);
}

fn calibre_marker_state_for_violation(state: &MarkerState) -> MarkerState {
    let mut state = state.clone();
    if state
        .signoff
        .as_deref()
        .is_some_and(|signoff| !signoff.trim().is_empty())
    {
        if state.signoff_by.is_none() {
            state.signoff_by = state
                .owner
                .as_deref()
                .filter(|owner| !owner.trim().is_empty())
                .map(str::to_string);
        }
        if state.signoff_note.is_none() {
            state.signoff_note = state
                .note
                .as_deref()
                .filter(|note| !note.trim().is_empty())
                .map(str::to_string);
        }
        record_marker_signoff(&mut state);
    }
    state
}

fn combined_calibre_marker_state(base: &MarkerState, local: &MarkerState) -> MarkerState {
    if local == &MarkerState::default() {
        return base.clone();
    }
    let mut state = base.clone();
    state.hidden |= local.hidden;
    state.waived |= local.waived;
    state.visited |= local.visited;
    state.important |= local.important;
    if local.note.is_some() {
        state.note = local.note.clone();
    }
    if local.owner.is_some() {
        state.owner = local.owner.clone();
    }
    if local.signoff.is_some() {
        state.signoff = local.signoff.clone();
    }
    if local.signoff_by.is_some() {
        state.signoff_by = local.signoff_by.clone();
    }
    if local.signoff_note.is_some() {
        state.signoff_note = local.signoff_note.clone();
    }
    state.signoff_records.extend(local.signoff_records.clone());
    state.tags.extend(local.tags.clone());
    state
}

fn record_marker_signoff(state: &mut MarkerState) {
    let Some(status) = state
        .signoff
        .as_deref()
        .filter(|signoff| !signoff.trim().is_empty())
        .map(str::to_string)
    else {
        return;
    };
    let Some(party) = state
        .signoff_by
        .as_deref()
        .or(state.owner.as_deref())
        .filter(|party| !party.trim().is_empty())
        .map(str::to_string)
    else {
        return;
    };
    state.signoff_records.insert(
        party,
        MarkerSignoffRecord {
            status,
            role: state
                .tags
                .get("signoff_role")
                .filter(|role| !role.trim().is_empty())
                .cloned(),
            by: state
                .signoff_by
                .as_deref()
                .filter(|signer| !signer.trim().is_empty())
                .map(str::to_string),
            note: state
                .signoff_note
                .as_deref()
                .filter(|note| !note.trim().is_empty())
                .map(str::to_string),
            recorded_at: state
                .tags
                .get("signoff_at")
                .or_else(|| state.tags.get("reviewed_at"))
                .filter(|recorded_at| !recorded_at.trim().is_empty())
                .cloned(),
        },
    );
}

fn parse_calibre_rve_metadata_line(content: &str, state: &mut MarkerState) -> bool {
    let Some((raw_key, raw_value)) = split_calibre_metadata_key_value(content) else {
        return false;
    };
    let key = sanitize_calibre_metadata_key(raw_key);
    let value = unquote_calibre_metadata_value(raw_value);
    if matches!(key.as_str(), "tag" | "tags") {
        return parse_calibre_rve_tag_metadata(&value, state);
    }
    if value.is_empty() {
        return true;
    }
    match key.as_str() {
        "hidden" => {
            state.hidden = parse_calibre_rve_bool(&value).unwrap_or(true);
            true
        }
        "waived" | "waive" => {
            state.waived = parse_calibre_rve_bool(&value).unwrap_or(true);
            true
        }
        "visited" => {
            state.visited = parse_calibre_rve_bool(&value).unwrap_or(true);
            true
        }
        "important" | "priority" => {
            state.important = parse_calibre_rve_bool(&value).unwrap_or(true);
            true
        }
        "note" | "review_note" => {
            state.note = Some(value);
            true
        }
        "owner" | "assignee" => {
            state.owner = Some(value);
            true
        }
        "signoff" | "signoff_status" | "review_status" | "approval" | "approval_status" => {
            state.signoff = Some(value);
            true
        }
        "signoff_by" | "signer" | "reviewer" | "reviewed_by" | "approver" | "approved_by" => {
            state.signoff_by = Some(value);
            true
        }
        "signoff_note" | "signoff_rationale" | "review_comment" | "approval_note" | "rationale" => {
            state.signoff_note = Some(value);
            true
        }
        "signoff_role" | "review_role" | "approval_role" => {
            state.tags.insert("signoff_role".to_string(), value);
            true
        }
        "signoff_at" | "reviewed_at" | "review_timestamp" | "approval_at" | "approved_at" => {
            state.tags.insert("signoff_at".to_string(), value);
            true
        }
        "category" | "marker_category" | "rdb_category" => {
            state.tags.insert(key, value);
            true
        }
        key if is_calibre_rve_review_tag_key(key) => {
            state.tags.insert(key.to_string(), value);
            true
        }
        _ => false,
    }
}

fn is_calibre_rve_database_header_line(line: &str) -> bool {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    tokens.len() == 2
        && !tokens[0]
            .chars()
            .all(|character| character.is_ascii_digit())
        && tokens[1]
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn is_calibre_rve_rule_summary_line(line: &str) -> bool {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    tokens.len() >= 3
        && tokens[..3]
            .iter()
            .all(|token| token.chars().all(|character| character.is_ascii_digit()))
        && tokens[3..]
            .iter()
            .any(|token| token.contains(':') || is_calibre_month_token(token))
}

fn is_calibre_month_token(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "jan"
            | "feb"
            | "mar"
            | "apr"
            | "may"
            | "jun"
            | "jul"
            | "aug"
            | "sep"
            | "oct"
            | "nov"
            | "dec"
    )
}

fn calibre_rve_cn_property_line(line: &str) -> Option<(String, bool, Option<KlayoutRdbTransform>)> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 || !tokens[0].eq_ignore_ascii_case("CN") {
        return None;
    }
    let cell = unquote_calibre_metadata_value(tokens[1]);
    if cell.is_empty() {
        return None;
    }
    let mut index = 2;
    let mut cell_space = false;
    if tokens
        .get(index)
        .is_some_and(|token| token.eq_ignore_ascii_case("c"))
    {
        cell_space = true;
        index += 1;
    }
    let transform = calibre_rve_cn_transform(&tokens[index..]);
    Some((cell, cell_space, transform))
}

fn calibre_rve_cn_transform(tokens: &[&str]) -> Option<KlayoutRdbTransform> {
    if tokens.len() < 6 {
        return None;
    }
    let a = tokens[0].parse::<f64>().ok()?;
    let b = tokens[1].parse::<f64>().ok()?;
    let c = tokens[2].parse::<f64>().ok()?;
    let d = tokens[3].parse::<f64>().ok()?;
    let tx = parse_calibre_coord(tokens[4])? as f64;
    let ty = parse_calibre_coord(tokens[5])? as f64;
    Some(KlayoutRdbTransform {
        matrix: [a, b, c, d],
        translation: [tx, ty],
    })
}

fn calibre_transform_tag_value(transform: KlayoutRdbTransform) -> String {
    format!(
        "{} {} {} {} {} {}",
        format_calibre_transform_value(transform.matrix[0]),
        format_calibre_transform_value(transform.matrix[1]),
        format_calibre_transform_value(transform.matrix[2]),
        format_calibre_transform_value(transform.matrix[3]),
        format_calibre_transform_value(transform.translation[0]),
        format_calibre_transform_value(transform.translation[1])
    )
}

fn format_calibre_transform_value(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn calibre_rve_result_property_line(line: &str) -> Option<(String, String)> {
    let (raw_key, raw_value) = line
        .split_once(char::is_whitespace)
        .or_else(|| line.split_once(':'))
        .or_else(|| line.split_once('='))?;
    if !is_calibre_rve_result_property_key(raw_key) {
        return None;
    }
    let key = sanitize_calibre_metadata_key(raw_key);
    let value = unquote_calibre_metadata_value(trim_calibre_metadata_value_separator(raw_value));
    (!key.is_empty() && !value.is_empty()).then_some((key, value))
}

fn is_calibre_rve_result_property_key(key: &str) -> bool {
    let key = key
        .trim()
        .trim_matches(|character| matches!(character, ':' | '='));
    !key.is_empty()
        && !key.chars().all(|character| character.is_ascii_digit())
        && key.len() <= 16
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        && key.chars().any(|character| character.is_ascii_uppercase())
}

fn append_calibre_result_property_tag(state: &mut MarkerState, key: &str, value: &str) {
    let tag_key = format!("calibre_property_{key}");
    state
        .tags
        .entry(tag_key)
        .and_modify(|existing| {
            if !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(value);
        })
        .or_insert_with(|| value.to_string());
}

fn parse_calibre_rve_measure_line(
    content: &str,
    current_required: &mut Coord,
    current_actual: &mut f64,
) -> bool {
    let tokens = calibre_measure_tokens(content);
    let mut index = 0;
    let mut saw_explicit_measure_key = false;
    let mut parsed_measure_value = false;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some((raw_key, raw_value)) = token.split_once('=').or_else(|| token.split_once(':'))
        {
            let key = sanitize_calibre_metadata_key(raw_key);
            if is_calibre_required_measure_key(&key) || is_calibre_actual_measure_key(&key) {
                saw_explicit_measure_key = true;
                let (value, next_index) = if raw_value.trim().is_empty() {
                    calibre_measure_value_from_tokens(&tokens, index + 1)
                } else {
                    calibre_inline_measure_value(raw_value, tokens.get(index + 1)).map_index(index)
                };
                parsed_measure_value |= apply_calibre_measure_value(
                    &key,
                    value.as_deref(),
                    current_required,
                    current_actual,
                );
                index = next_index;
                continue;
            }
        }
        let key = sanitize_calibre_metadata_key(token);
        if is_calibre_required_measure_key(&key) || is_calibre_actual_measure_key(&key) {
            let (value, next_index) = calibre_measure_value_from_tokens(&tokens, index + 1);
            parsed_measure_value |= apply_calibre_measure_value(
                &key,
                value.as_deref(),
                current_required,
                current_actual,
            );
            index = next_index;
            continue;
        }
        index += 1;
    }
    saw_explicit_measure_key || parsed_measure_value
}

fn apply_calibre_measure_value(
    key: &str,
    value: Option<&str>,
    current_required: &mut Coord,
    current_actual: &mut f64,
) -> bool {
    let Some(value) = value else {
        return false;
    };
    if is_calibre_required_measure_key(key) {
        if let Some(required) = parse_calibre_measure_coord(value) {
            *current_required = required.max(0);
            return true;
        }
    } else if is_calibre_actual_measure_key(key)
        && let Some(actual) = parse_calibre_measure_value(value)
    {
        *current_actual = actual.max(0.0);
        return true;
    }
    false
}

fn is_calibre_required_measure_key(key: &str) -> bool {
    matches!(
        key,
        "required" | "required_value" | "requirement" | "constraint" | "limit" | "minimum" | "min"
    )
}

fn is_calibre_actual_measure_key(key: &str) -> bool {
    matches!(
        key,
        "actual"
            | "actual_value"
            | "measured"
            | "measured_value"
            | "measurement"
            | "value"
            | "found"
    )
}

fn calibre_measure_value_from_tokens(tokens: &[&str], index: usize) -> (Option<String>, usize) {
    let index = calibre_measure_value_index(tokens, index);
    let Some(value) = tokens.get(index).copied() else {
        return (None, index);
    };
    calibre_inline_measure_value(value, tokens.get(index + 1)).map_index(index)
}

fn calibre_measure_value_index(tokens: &[&str], index: usize) -> usize {
    if tokens
        .get(index)
        .is_some_and(|token| matches!(*token, "=" | ":"))
    {
        index + 1
    } else {
        index
    }
}

trait CalibreMeasureValueIndex {
    fn map_index(self, value_index: usize) -> (Option<String>, usize);
}

impl CalibreMeasureValueIndex for (Option<String>, usize) {
    fn map_index(self, value_index: usize) -> (Option<String>, usize) {
        let (value, consumed) = self;
        (value, value_index + consumed)
    }
}

fn calibre_inline_measure_value(value: &str, next_token: Option<&&str>) -> (Option<String>, usize) {
    let value = unquote_calibre_metadata_value(value);
    if value.is_empty() {
        return (None, 1);
    }
    if calibre_coord_can_take_separated_unit(&value)
        && let Some(unit) = next_token.copied()
        && is_calibre_measure_unit(unit)
    {
        return (Some(format!("{value}{unit}")), 2);
    }
    (Some(value), 1)
}

fn is_calibre_measure_unit(value: &str) -> bool {
    calibre_unit_scale(value).is_some()
}

fn calibre_measure_tokens(content: &str) -> Vec<&str> {
    content
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '(' | ')' | '[' | ']' | ',' | ';')
        })
        .filter(|token| !token.trim().is_empty())
        .collect()
}

fn parse_calibre_measure_coord(value: &str) -> Option<Coord> {
    parse_calibre_measure_value(value).and_then(round_calibre_coord)
}

fn parse_calibre_measure_value(value: &str) -> Option<f64> {
    let token = value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '(' | ')' | '[' | ']' | ',' | ';' | ':')
        })
        .find(|token| !token.trim().is_empty())?
        .trim();
    if let Ok(value) = token.parse::<f64>() {
        return value.is_finite().then_some(value);
    }
    let (number, unit) = split_calibre_unit_coord(token)?;
    let scale = calibre_unit_scale(unit)?;
    let value = number.parse::<f64>().ok()? * scale;
    value.is_finite().then_some(value)
}

fn split_calibre_metadata_key_value(content: &str) -> Option<(&str, &str)> {
    if let Some((raw_key, raw_value)) = split_calibre_explicit_metadata_key_value(content) {
        return Some((raw_key, raw_value));
    }
    if let Some((first, rest)) = content.split_once(char::is_whitespace)
        && is_calibre_metadata_key(&sanitize_calibre_metadata_key(first))
    {
        return Some((first, trim_calibre_metadata_value_separator(rest)));
    }
    content.split_once(':').or_else(|| content.split_once('='))
}

fn split_calibre_explicit_metadata_key_value(content: &str) -> Option<(&str, &str)> {
    [':', '=']
        .into_iter()
        .filter_map(|separator| content.find(separator).map(|index| (index, separator)))
        .filter_map(|(index, separator)| {
            let raw_key = content[..index].trim();
            let raw_value = content[index + separator.len_utf8()..].trim_start();
            is_calibre_metadata_key(&sanitize_calibre_metadata_key(raw_key))
                .then_some((index, raw_key, raw_value))
        })
        .min_by_key(|(index, _, _)| *index)
        .map(|(_, raw_key, raw_value)| (raw_key, raw_value))
}

fn trim_calibre_metadata_value_separator(value: &str) -> &str {
    let value = value.trim_start();
    value
        .strip_prefix('=')
        .or_else(|| value.strip_prefix(':'))
        .map(str::trim_start)
        .unwrap_or(value)
}

fn sanitize_calibre_metadata_key(key: &str) -> String {
    key.trim()
        .trim_start_matches('@')
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn unquote_calibre_metadata_value(value: &str) -> String {
    let value = value.trim();
    if value.len() < 2 {
        return value.to_string();
    }
    let Some(first) = value.chars().next() else {
        return String::new();
    };
    let Some(last) = value.chars().next_back() else {
        return String::new();
    };
    if matches!((first, last), ('"', '"') | ('\'', '\'')) {
        value[first.len_utf8()..value.len() - last.len_utf8()]
            .trim()
            .to_string()
    } else {
        value.to_string()
    }
}

fn is_calibre_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "hidden"
            | "waived"
            | "waive"
            | "visited"
            | "important"
            | "priority"
            | "note"
            | "review_note"
            | "owner"
            | "assignee"
            | "signoff"
            | "signoff_status"
            | "review_status"
            | "approval"
            | "approval_status"
            | "signoff_by"
            | "signer"
            | "reviewer"
            | "reviewed_by"
            | "approver"
            | "approved_by"
            | "signoff_note"
            | "signoff_rationale"
            | "review_comment"
            | "approval_note"
            | "rationale"
            | "signoff_role"
            | "review_role"
            | "approval_role"
            | "signoff_at"
            | "reviewed_at"
            | "review_timestamp"
            | "approval_at"
            | "approved_at"
            | "category"
            | "marker_category"
            | "rdb_category"
            | "cell"
            | "cell_name"
            | "source_cell"
            | "top_cell"
            | "layer"
            | "layer_name"
            | "source_layer"
            | "datatype"
            | "rdb"
            | "rdb_file"
            | "database"
            | "file"
            | "filename"
            | "required"
            | "required_value"
            | "requirement"
            | "constraint"
            | "limit"
            | "minimum"
            | "min"
            | "actual"
            | "actual_value"
            | "measured"
            | "measured_value"
            | "measurement"
            | "value"
            | "found"
            | "tag"
            | "tags"
            | "severity"
            | "classification"
            | "class"
            | "source"
            | "origin"
            | "run"
            | "run_id"
            | "tool"
            | "tool_version"
            | "deck"
            | "deck_name"
    )
}

fn is_calibre_rve_review_tag_key(key: &str) -> bool {
    matches!(
        key,
        "severity"
            | "classification"
            | "class"
            | "source"
            | "origin"
            | "run"
            | "run_id"
            | "tool"
            | "tool_version"
            | "deck"
            | "deck_name"
            | "cell"
            | "cell_name"
            | "source_cell"
            | "top_cell"
            | "layer"
            | "layer_name"
            | "source_layer"
            | "datatype"
            | "rdb"
            | "rdb_file"
            | "database"
            | "file"
            | "filename"
    )
}

fn parse_calibre_rve_tag_metadata(value: &str, state: &mut MarkerState) -> bool {
    let mut parsed_any = false;
    for item in split_calibre_rve_tag_metadata_items(value) {
        let item = unquote_calibre_metadata_value(item);
        let Some((tag_key, tag_value)) = calibre_rve_tag_metadata_item_key_value(&item) else {
            continue;
        };
        let tag_key = tag_key.trim();
        let tag_value = unquote_calibre_metadata_value(tag_value);
        parsed_any = true;
        if tag_key.is_empty() || tag_value.is_empty() {
            continue;
        }
        state.tags.insert(tag_key.to_string(), tag_value);
    }
    parsed_any
}

fn calibre_rve_tag_metadata_item_key_value(item: &str) -> Option<(&str, &str)> {
    item.split_once('=')
        .or_else(|| item.split_once(':'))
        .or_else(|| item.split_once(char::is_whitespace))
}

fn split_calibre_rve_tag_metadata_items(value: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut quote = None;
    for (index, ch) in value.char_indices() {
        if matches!(ch, '"' | '\'') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
        } else if quote.is_none() && matches!(ch, ';' | ',') {
            let item = value[start..index].trim();
            if !item.is_empty() {
                items.push(item);
            }
            start = index + ch.len_utf8();
        }
    }
    let item = value[start..].trim();
    if !item.is_empty() {
        items.push(item);
    }
    items
}

fn parse_calibre_rve_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" | "set" => Some(true),
        "0" | "false" | "no" | "n" | "off" | "clear" => Some(false),
        _ => None,
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

#[derive(Clone, Copy, Debug)]
enum CalibreGeometryKind {
    Polygon,
    Rect,
    Edge,
    Point,
    Circle,
}

impl CalibreGeometryKind {
    fn label(self) -> &'static str {
        match self {
            CalibreGeometryKind::Polygon => "polygon",
            CalibreGeometryKind::Rect => "rectangle",
            CalibreGeometryKind::Edge => "edge",
            CalibreGeometryKind::Point => "point",
            CalibreGeometryKind::Circle => "circle",
        }
    }
}

struct PendingCalibreGeometry {
    kind: CalibreGeometryKind,
    start_line: usize,
    expected_coords: Option<usize>,
    coords: Vec<Coord>,
    marker_state: MarkerState,
    transform_to_top: Option<KlayoutRdbTransform>,
    allow_result_properties: bool,
}

impl PendingCalibreGeometry {
    fn from_geometry_line(
        kind: CalibreGeometryKind,
        line: &str,
        start_line: usize,
    ) -> Option<Self> {
        let raw_coords = calibre_geometry_raw_coords(line);
        let prefix = calibre_geometry_coord_prefix(kind, &raw_coords);
        let coords = raw_coords[prefix.coord_start..].to_vec();
        let expected_coords = prefix.expected_coords.or_else(|| match kind {
            CalibreGeometryKind::Polygon => None,
            CalibreGeometryKind::Rect | CalibreGeometryKind::Edge => Some(4),
            CalibreGeometryKind::Point => Some(2),
            CalibreGeometryKind::Circle => Some(3),
        });
        expected_coords
            .is_none_or(|expected_coords| coords.len() < expected_coords)
            .then_some(Self {
                kind,
                start_line,
                expected_coords,
                allow_result_properties: coords.is_empty(),
                coords,
                marker_state: MarkerState::default(),
                transform_to_top: None,
            })
    }

    fn bounds(&self) -> Option<Rect> {
        let expected_coords = self.expected_coords?;
        if self.coords.len() < expected_coords {
            return None;
        }
        let coords = &self.coords[..expected_coords];
        self.bounds_for_coords(coords)
    }

    fn finished_bounds(&self) -> Option<Rect> {
        if let Some(bounds) = self.bounds() {
            return Some(bounds);
        }
        if !matches!(self.kind, CalibreGeometryKind::Polygon) || self.expected_coords.is_some() {
            return None;
        }
        if self.coords.len() < 6 || self.coords.len() % 2 != 0 {
            return None;
        }
        self.bounds_for_coords(&self.coords)
    }

    fn bounds_for_coords(&self, coords: &[Coord]) -> Option<Rect> {
        if let Some(transform) = self.transform_to_top {
            calibre_bounds_for_transformed_coords(self.kind, coords, transform)
        } else {
            calibre_bounds_for_coords(self.kind, coords)
        }
    }

    fn extend_coords(&mut self, coords: Vec<Coord>) {
        if !coords.is_empty() {
            self.allow_result_properties = false;
            self.coords.extend(coords);
        }
    }

    fn consume_result_property_line(&mut self, line: &str) -> bool {
        if !self.allow_result_properties {
            return false;
        }
        if let Some((cell, cell_space, transform)) = calibre_rve_cn_property_line(line) {
            self.marker_state
                .tags
                .insert("source_cell".to_string(), cell);
            self.marker_state
                .tags
                .insert("calibre_cn_cell_space".to_string(), cell_space.to_string());
            if let Some(transform) = transform {
                self.transform_to_top = Some(transform);
                self.marker_state.tags.insert(
                    "calibre_cn_transform".to_string(),
                    calibre_transform_tag_value(transform),
                );
            }
            return true;
        }
        if parse_calibre_rve_metadata_line(line, &mut self.marker_state) {
            return true;
        }
        if let Some((key, value)) = calibre_rve_result_property_line(line) {
            append_calibre_result_property_tag(&mut self.marker_state, &key, &value);
            return true;
        }
        false
    }

    fn expected_coord_description(&self) -> String {
        self.expected_coords.map_or_else(
            || "at least 6 coordinate value(s)".to_string(),
            |expected_coords| format!("{expected_coords} coordinate value(s)"),
        )
    }

    fn line_coords(&self, line: &str) -> Vec<Coord> {
        for candidate in calibre_pending_coord_row_candidates(line) {
            let mut coords = calibre_line_coords(candidate);
            if coords.is_empty() {
                coords = calibre_pending_keyed_line_coords(self.kind, candidate);
            }
            if coords.is_empty() {
                continue;
            }
            if calibre_line_has_coord_row_index(candidate) && coords.len() >= 3 {
                coords.remove(0);
            } else if self.has_whitespace_coord_row_index(candidate, &coords) {
                coords.remove(0);
            }
            return coords;
        }
        Vec::new()
    }

    fn has_whitespace_coord_row_index(&self, line: &str, coords: &[Coord]) -> bool {
        let row_width = self.row_coord_width();
        coords.len() > row_width
            && calibre_line_has_whitespace_coord_row_index(line)
            && coords.first().copied() == Some(self.next_coord_row_index())
    }

    fn row_coord_width(&self) -> usize {
        match self.kind {
            CalibreGeometryKind::Circle => 3,
            CalibreGeometryKind::Polygon
            | CalibreGeometryKind::Rect
            | CalibreGeometryKind::Edge
            | CalibreGeometryKind::Point => 2,
        }
    }

    fn next_coord_row_index(&self) -> Coord {
        (self.coords.len() / self.row_coord_width() + 1) as Coord
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CalibreGeometryCoordPrefix {
    coord_start: usize,
    expected_coords: Option<usize>,
}

fn calibre_pending_keyed_line_coords(kind: CalibreGeometryKind, line: &str) -> Vec<Coord> {
    let values = calibre_keyed_coord_values_from_index(line, 0);
    if values.is_empty() {
        return Vec::new();
    }
    match kind {
        CalibreGeometryKind::Polygon => calibre_keyed_polygon_coords(&values)
            .or_else(|| calibre_keyed_point_coords(&values))
            .unwrap_or_default(),
        CalibreGeometryKind::Rect | CalibreGeometryKind::Edge => calibre_keyed_bbox_coords(&values)
            .or_else(|| calibre_keyed_point_coords(&values))
            .unwrap_or_default(),
        CalibreGeometryKind::Point => calibre_keyed_point_coords(&values).unwrap_or_default(),
        CalibreGeometryKind::Circle => calibre_keyed_circle_coords(&values)
            .or_else(|| calibre_keyed_point_coords(&values))
            .unwrap_or_default(),
    }
}

fn calibre_pending_coord_row_candidates(line: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    if let Some(rest) = calibre_line_without_coord_row_label(line) {
        candidates.push(rest);
        if let Some(indexed_rest) = calibre_line_without_leading_coord_row_index(rest) {
            candidates.push(indexed_rest);
        }
    }
    if let Some(rest) = calibre_line_without_leading_coord_row_index(line) {
        candidates.push(rest);
        if let Some(labeled_rest) = calibre_line_without_coord_row_label(rest) {
            candidates.push(labeled_rest);
        }
    }
    candidates.push(line);
    candidates
}

fn calibre_line_without_coord_row_label(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let mut label_end = 0;
    for (index, character) in line.char_indices() {
        if !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '[' | ']')) {
            break;
        }
        label_end = index + character.len_utf8();
    }
    if label_end == 0 {
        return None;
    }
    let key = sanitize_calibre_metadata_key(&line[..label_end]);
    if !is_calibre_coord_row_label_key(&key) {
        return None;
    }
    let rest = line[label_end..].trim_start();
    let rest = rest
        .strip_prefix(':')
        .or_else(|| rest.strip_prefix('='))
        .map(str::trim_start)
        .unwrap_or(rest);
    (!rest.is_empty()).then_some(rest)
}

fn is_calibre_coord_row_label_key(key: &str) -> bool {
    matches!(
        key,
        "coord"
            | "coords"
            | "coordinate"
            | "coordinates"
            | "xy"
            | "point"
            | "pt"
            | "vertex"
            | "v"
            | "location"
            | "loc"
            | "position"
            | "pos"
            | "center"
            | "centre"
            | "start"
            | "begin"
            | "from"
            | "end"
            | "finish"
            | "to"
            | "target"
    ) || calibre_indexed_geometry_key_suffix(
        &canonical_calibre_geometry_coord_key(key),
        &[
            "coord",
            "coords",
            "coordinate",
            "coordinates",
            "vertex",
            "point",
            "pt",
            "xy",
            "v",
            "p",
        ],
    )
    .is_some()
}

fn calibre_line_has_coord_row_index(line: &str) -> bool {
    let line = line.trim_start();
    let mut digit_end = 0;
    for (index, character) in line.char_indices() {
        if !character.is_ascii_digit() {
            break;
        }
        digit_end = index + character.len_utf8();
    }
    if digit_end == 0 {
        return false;
    }
    let suffix = line[digit_end..].trim_start();
    suffix.starts_with(':') || suffix.starts_with(')') || suffix.starts_with('.')
}

fn calibre_line_has_whitespace_coord_row_index(line: &str) -> bool {
    let line = line.trim_start();
    let mut digit_end = 0;
    for (index, character) in line.char_indices() {
        if !character.is_ascii_digit() {
            break;
        }
        digit_end = index + character.len_utf8();
    }
    digit_end > 0
        && line[digit_end..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
}

fn calibre_line_without_leading_coord_row_index(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let mut digit_end = 0;
    for (index, character) in line.char_indices() {
        if !character.is_ascii_digit() {
            break;
        }
        digit_end = index + character.len_utf8();
    }
    if digit_end == 0 {
        return None;
    }
    let suffix = line[digit_end..].trim_start();
    let rest = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix(')'))
        .or_else(|| suffix.strip_prefix('.'))?
        .trim_start();
    (!rest.is_empty()).then_some(rest)
}

fn calibre_line_without_geometry_record_index(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let mut digit_end = 0;
    for (index, character) in line.char_indices() {
        if !character.is_ascii_digit() {
            break;
        }
        digit_end = index + character.len_utf8();
    }
    if digit_end == 0 {
        return None;
    }
    let suffix = line[digit_end..].trim_start();
    let rest = suffix
        .strip_prefix(':')
        .or_else(|| suffix.strip_prefix(')'))
        .or_else(|| suffix.strip_prefix('.'))
        .map(str::trim_start)
        .unwrap_or(suffix);
    calibre_geometry_kind_from_line(rest)
        .is_some()
        .then_some(rest)
}

fn calibre_geometry_kind_from_line(line: &str) -> Option<CalibreGeometryKind> {
    let first_lower = line.split_whitespace().next()?.to_ascii_lowercase();
    calibre_geometry_kind(&first_lower).or_else(|| calibre_keyed_geometry_kind(line))
}

fn calibre_geometry_kind(first_lower: &str) -> Option<CalibreGeometryKind> {
    let first_lower = first_lower
        .split_once(':')
        .or_else(|| first_lower.split_once('='))
        .map(|(keyword, _)| keyword)
        .unwrap_or(first_lower)
        .trim_end_matches(':');
    if matches!(
        first_lower,
        "p" | "poly" | "polys" | "polygon" | "polygons" | "boundary" | "boundaries"
    ) {
        Some(CalibreGeometryKind::Polygon)
    } else if matches!(
        first_lower,
        "r" | "rect"
            | "rects"
            | "rectangle"
            | "rectangles"
            | "b"
            | "box"
            | "boxes"
            | "bbox"
            | "bounds"
    ) {
        Some(CalibreGeometryKind::Rect)
    } else if matches!(
        first_lower,
        "e" | "edge"
            | "edges"
            | "path"
            | "paths"
            | "line"
            | "lines"
            | "segment"
            | "segments"
            | "seg"
            | "segs"
    ) {
        Some(CalibreGeometryKind::Edge)
    } else if matches!(first_lower, "pt" | "pts" | "point" | "points") {
        Some(CalibreGeometryKind::Point)
    } else if matches!(first_lower, "c" | "circ" | "circle" | "circles") {
        Some(CalibreGeometryKind::Circle)
    } else {
        None
    }
}

fn calibre_keyed_geometry_kind(line: &str) -> Option<CalibreGeometryKind> {
    let tokens = calibre_keyed_coord_tokens(line);
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some((raw_key, raw_value)) = token.split_once('=').or_else(|| token.split_once(':'))
        {
            let key = sanitize_calibre_metadata_key(raw_key);
            if is_calibre_geometry_kind_key(&key) {
                let (value, consumed) = if raw_value.trim().is_empty() {
                    (tokens.get(index + 1).copied(), 2)
                } else {
                    (Some(raw_value), 1)
                };
                if let Some(kind) = value.and_then(calibre_geometry_kind_value) {
                    return Some(kind);
                }
                index += consumed;
                continue;
            }
        }
        let key = sanitize_calibre_metadata_key(token);
        if is_calibre_geometry_kind_key(&key) {
            let value_index = if tokens
                .get(index + 1)
                .is_some_and(|token| matches!(*token, "=" | ":"))
            {
                index + 2
            } else {
                index + 1
            };
            if let Some(kind) = tokens
                .get(value_index)
                .copied()
                .and_then(calibre_geometry_kind_value)
            {
                return Some(kind);
            }
            index = value_index.saturating_add(1);
            continue;
        }
        index += 1;
    }
    None
}

fn is_calibre_geometry_kind_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "kind"
            | "geometry"
            | "geometry_type"
            | "shape"
            | "shape_type"
            | "marker_type"
            | "object_type"
    )
}

fn calibre_geometry_kind_value(value: &str) -> Option<CalibreGeometryKind> {
    let value = unquote_calibre_metadata_value(value);
    let value = sanitize_calibre_metadata_key(&value);
    calibre_geometry_kind(&value)
}

fn calibre_bounds_for_kind(kind: CalibreGeometryKind, line: &str) -> Option<Rect> {
    calibre_keyed_bounds_for_kind(kind, line).or_else(|| match kind {
        CalibreGeometryKind::Polygon => calibre_polygon_bounds(line),
        CalibreGeometryKind::Rect => calibre_rect_bounds(line),
        CalibreGeometryKind::Edge => calibre_edge_bounds(line),
        CalibreGeometryKind::Point => calibre_point_bounds(line),
        CalibreGeometryKind::Circle => calibre_circle_bounds(line),
    })
}

fn calibre_bounds_for_coords(kind: CalibreGeometryKind, coords: &[Coord]) -> Option<Rect> {
    match kind {
        CalibreGeometryKind::Polygon => calibre_polygon_bounds_from_coords(coords),
        CalibreGeometryKind::Rect => calibre_rect_bounds_from_coords(coords),
        CalibreGeometryKind::Edge => calibre_edge_bounds_from_coords(coords),
        CalibreGeometryKind::Point => calibre_point_bounds_from_coords(coords),
        CalibreGeometryKind::Circle => calibre_circle_bounds_from_coords(coords),
    }
}

fn calibre_bounds_for_transformed_coords(
    kind: CalibreGeometryKind,
    coords: &[Coord],
    transform: KlayoutRdbTransform,
) -> Option<Rect> {
    match kind {
        CalibreGeometryKind::Circle => {
            if coords.len() < 3 {
                return None;
            }
            let center = transform.apply_point(Point::new(coords[0], coords[1]));
            calibre_circle_bounds_from_coords(&[center.x, center.y, coords[2]])
        }
        CalibreGeometryKind::Polygon
        | CalibreGeometryKind::Rect
        | CalibreGeometryKind::Edge
        | CalibreGeometryKind::Point => {
            let transformed = coords
                .chunks_exact(2)
                .flat_map(|chunk| {
                    let point = transform.apply_point(Point::new(chunk[0], chunk[1]));
                    [point.x, point.y]
                })
                .collect::<Vec<_>>();
            calibre_bounds_for_coords(kind, &transformed)
        }
    }
}

fn calibre_keyed_bounds_for_kind(kind: CalibreGeometryKind, line: &str) -> Option<Rect> {
    let values = calibre_keyed_coord_values(line);
    if values.is_empty() {
        return None;
    }
    let coords = match kind {
        CalibreGeometryKind::Polygon => calibre_keyed_polygon_coords(&values)?,
        CalibreGeometryKind::Rect => calibre_keyed_bbox_coords(&values)?,
        CalibreGeometryKind::Edge => calibre_keyed_bbox_coords(&values)?,
        CalibreGeometryKind::Point => calibre_keyed_point_coords(&values)?,
        CalibreGeometryKind::Circle => calibre_keyed_circle_coords(&values)?,
    };
    calibre_bounds_for_coords(kind, &coords)
}

fn calibre_keyed_bbox_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    calibre_keyed_corner_bbox_coords(values)
        .or_else(|| calibre_keyed_center_size_bbox_coords(values))
}

fn calibre_keyed_corner_bbox_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    let x1 = calibre_keyed_coord(
        values,
        &[
            "x1", "x0", "x", "origin_x", "originx", "x_min", "xmin", "xlo", "left", "llx",
        ],
    )?;
    let y1 = calibre_keyed_coord(
        values,
        &[
            "y1", "y0", "y", "origin_y", "originy", "y_min", "ymin", "ylo", "bottom", "lly",
        ],
    )?;
    if let (Some(x2), Some(y2)) = (
        calibre_keyed_coord(values, &["x2", "x_max", "xmax", "xhi", "right", "urx"]),
        calibre_keyed_coord(values, &["y2", "y_max", "ymax", "yhi", "top", "ury"]),
    ) {
        return Some(vec![x1, y1, x2, y2]);
    }
    let width = calibre_keyed_coord(
        values,
        &["width", "w", "dx", "x_size", "xsize", "size_x", "sizex"],
    )?;
    let height = calibre_keyed_coord(
        values,
        &["height", "h", "dy", "y_size", "ysize", "size_y", "sizey"],
    )?;
    Some(vec![x1, y1, x1 + width, y1 + height])
}

fn calibre_keyed_center_size_bbox_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    let cx = calibre_keyed_coord(
        values,
        &["cx", "xc", "center_x", "centre_x", "x_center", "xcentre"],
    )?;
    let cy = calibre_keyed_coord(
        values,
        &["cy", "yc", "center_y", "centre_y", "y_center", "ycentre"],
    )?;
    let width = calibre_keyed_coord(
        values,
        &["width", "w", "dx", "x_size", "xsize", "size_x", "sizex"],
    )?;
    let height = calibre_keyed_coord(
        values,
        &["height", "h", "dy", "y_size", "ysize", "size_y", "sizey"],
    )?;
    Some(vec![
        round_calibre_coord(cx as f64 - width as f64 / 2.0)?,
        round_calibre_coord(cy as f64 - height as f64 / 2.0)?,
        round_calibre_coord(cx as f64 + width as f64 / 2.0)?,
        round_calibre_coord(cy as f64 + height as f64 / 2.0)?,
    ])
}

fn calibre_keyed_point_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    Some(vec![
        calibre_keyed_coord(
            values,
            &[
                "x", "x1", "x0", "cx", "xc", "center_x", "centre_x", "x_center", "xcentre",
            ],
        )?,
        calibre_keyed_coord(
            values,
            &[
                "y", "y1", "y0", "cy", "yc", "center_y", "centre_y", "y_center", "ycentre",
            ],
        )?,
    ])
}

fn calibre_keyed_circle_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    let radius = calibre_keyed_coord(values, &["radius", "rad", "r"])
        .or_else(|| calibre_keyed_circle_diameter_radius(values))?;
    Some(vec![
        calibre_keyed_coord(
            values,
            &[
                "cx", "xc", "center_x", "centre_x", "x_center", "xcentre", "x", "x1", "x0",
            ],
        )?,
        calibre_keyed_coord(
            values,
            &[
                "cy", "yc", "center_y", "centre_y", "y_center", "ycentre", "y", "y1", "y0",
            ],
        )?,
        radius,
    ])
}

fn calibre_keyed_circle_diameter_radius(values: &BTreeMap<String, Coord>) -> Option<Coord> {
    let diameter = calibre_keyed_coord(values, &["diameter", "diam", "dia", "d"])?;
    round_calibre_coord(diameter as f64 / 2.0)
}

fn calibre_keyed_polygon_coords(values: &BTreeMap<String, Coord>) -> Option<Vec<Coord>> {
    let mut coords = Vec::new();
    for index in 0..=64 {
        let x_key = format!("x{index}");
        let y_key = format!("y{index}");
        if let (Some(x), Some(y)) = (values.get(&x_key), values.get(&y_key)) {
            coords.push(*x);
            coords.push(*y);
        }
    }
    (coords.len() >= 4).then_some(coords)
}

fn calibre_keyed_coord(values: &BTreeMap<String, Coord>, aliases: &[&str]) -> Option<Coord> {
    aliases.iter().find_map(|key| values.get(*key).copied())
}

fn calibre_keyed_coord_values(line: &str) -> BTreeMap<String, Coord> {
    calibre_keyed_coord_values_from_index(line, 1)
}

fn calibre_keyed_coord_values_from_index(
    line: &str,
    start_index: usize,
) -> BTreeMap<String, Coord> {
    let mut values = BTreeMap::new();
    let tokens = calibre_keyed_coord_tokens(line);
    let mut index = start_index;
    while index < tokens.len() {
        if let Some(((x_key, x_coord), (y_key, y_coord), consumed)) =
            calibre_keyed_coord_pair_from_tokens(&tokens, index)
        {
            values.insert(x_key, x_coord);
            values.insert(y_key, y_coord);
            index += consumed;
        } else if let Some((key, coord, consumed)) = calibre_keyed_coord_from_tokens(&tokens, index)
        {
            values.insert(key, coord);
            index += consumed;
        } else {
            index += 1;
        }
    }
    values
}

fn calibre_keyed_coord_pair_from_tokens(
    tokens: &[&str],
    index: usize,
) -> Option<((String, Coord), (String, Coord), usize)> {
    let token = tokens.get(index).copied()?;
    if let Some((raw_key, raw_value)) = token.split_once('=').or_else(|| token.split_once(':')) {
        let key = canonical_calibre_geometry_coord_key(&sanitize_calibre_metadata_key(raw_key));
        let (x_key, y_key) = calibre_geometry_coord_pair_keys(&key)?;
        let (x_coord, second_index) = if raw_value.trim().is_empty() {
            let (coord, consumed) =
                parse_calibre_coord_token(tokens.get(index + 1).copied()?, tokens.get(index + 2));
            (coord?, index + 1 + consumed)
        } else {
            let (coord, consumed) = parse_calibre_coord_token(raw_value, tokens.get(index + 1));
            (coord?, index + consumed)
        };
        let (y_coord, y_consumed) = parse_calibre_coord_token(
            tokens.get(second_index).copied()?,
            tokens.get(second_index + 1),
        );
        return Some((
            (x_key.to_string(), x_coord),
            (y_key.to_string(), y_coord?),
            second_index - index + y_consumed,
        ));
    }

    let key = canonical_calibre_geometry_coord_key(&sanitize_calibre_metadata_key(token));
    let (x_key, y_key) = calibre_geometry_coord_pair_keys(&key)?;
    let value_index = if tokens
        .get(index + 1)
        .is_some_and(|token| matches!(*token, "=" | ":"))
    {
        index + 2
    } else {
        index + 1
    };
    let (x_coord, x_consumed) = parse_calibre_coord_token(
        tokens.get(value_index).copied()?,
        tokens.get(value_index + 1),
    );
    let second_index = value_index + x_consumed;
    let (y_coord, y_consumed) = parse_calibre_coord_token(
        tokens.get(second_index).copied()?,
        tokens.get(second_index + 1),
    );
    Some((
        (x_key.to_string(), x_coord?),
        (y_key.to_string(), y_coord?),
        second_index - index + y_consumed,
    ))
}

fn calibre_geometry_coord_pair_keys(key: &str) -> Option<(String, String)> {
    if let Some(index) = calibre_geometry_vertex_pair_index(key) {
        return Some((format!("x{index}"), format!("y{index}")));
    }
    match key {
        "ll" | "lower_left" | "lowerleft" => Some(("llx".to_string(), "lly".to_string())),
        "ur" | "upper_right" | "upperright" => Some(("urx".to_string(), "ury".to_string())),
        "ul" | "upper_left" | "upperleft" => Some(("x_min".to_string(), "y_max".to_string())),
        "lr" | "lower_right" | "lowerright" => Some(("x_max".to_string(), "y_min".to_string())),
        "origin" | "orig" => Some(("origin_x".to_string(), "origin_y".to_string())),
        "size" | "dimension" | "dimensions" | "dim" | "dims" => {
            Some(("width".to_string(), "height".to_string()))
        }
        "min" | "minimum" | "min_corner" | "minimum_corner" | "lower" | "lo" | "lower_bound"
        | "lower_bounds" => Some(("x_min".to_string(), "y_min".to_string())),
        "max" | "maximum" | "max_corner" | "maximum_corner" | "upper" | "hi" | "upper_bound"
        | "upper_bounds" => Some(("x_max".to_string(), "y_max".to_string())),
        "start" | "begin" | "from" => Some(("x1".to_string(), "y1".to_string())),
        "end" | "finish" | "to" | "target" => Some(("x2".to_string(), "y2".to_string())),
        "center" | "centre" | "centroid" | "xy" => {
            Some(("center_x".to_string(), "center_y".to_string()))
        }
        "loc" | "location" | "pos" | "position" => {
            Some(("center_x".to_string(), "center_y".to_string()))
        }
        _ => None,
    }
}

fn calibre_geometry_vertex_pair_index(key: &str) -> Option<usize> {
    calibre_indexed_geometry_key_suffix(key, &["vertex", "point", "pt", "xy", "v", "p"])
}

fn calibre_keyed_coord_from_tokens(
    tokens: &[&str],
    index: usize,
) -> Option<(String, Coord, usize)> {
    let token = tokens.get(index).copied()?;
    if let Some((raw_key, raw_value)) = token.split_once('=').or_else(|| token.split_once(':')) {
        let key = canonical_calibre_geometry_coord_key(&sanitize_calibre_metadata_key(raw_key));
        if !is_calibre_geometry_coord_key(&key) {
            return None;
        }
        if raw_value.trim().is_empty() {
            let (coord, value_consumed) =
                parse_calibre_coord_token(tokens.get(index + 1).copied()?, tokens.get(index + 2));
            return coord.map(|coord| (key, coord, 1 + value_consumed));
        }
        let (coord, value_consumed) = parse_calibre_coord_token(raw_value, tokens.get(index + 1));
        return coord.map(|coord| (key, coord, value_consumed));
    }

    let key = canonical_calibre_geometry_coord_key(&sanitize_calibre_metadata_key(token));
    if !is_calibre_geometry_coord_key(&key) {
        return None;
    }
    let value_index = if tokens
        .get(index + 1)
        .is_some_and(|token| matches!(*token, "=" | ":"))
    {
        index + 2
    } else {
        index + 1
    };
    let prefix_consumed = value_index.saturating_sub(index);
    let (coord, value_consumed) = parse_calibre_coord_token(
        tokens.get(value_index).copied()?,
        tokens.get(value_index + 1),
    );
    coord.map(|coord| (key, coord, prefix_consumed + value_consumed))
}

fn calibre_keyed_coord_tokens(line: &str) -> Vec<&str> {
    line.split(|character: char| {
        character.is_whitespace() || matches!(character, '(' | ')' | ',' | ';')
    })
    .filter(|token| !token.trim().is_empty())
    .collect()
}

pub(crate) fn calibre_polygon_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    calibre_polygon_bounds_from_coords(&coords)
}

fn calibre_polygon_bounds_from_coords(coords: &[Coord]) -> Option<Rect> {
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
    calibre_rect_bounds_from_coords(&coords)
}

fn calibre_rect_bounds_from_coords(coords: &[Coord]) -> Option<Rect> {
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
    calibre_edge_bounds_from_coords(&coords)
}

fn calibre_edge_bounds_from_coords(coords: &[Coord]) -> Option<Rect> {
    if coords.len() < 4 {
        return None;
    }
    let points = coords
        .chunks_exact(4)
        .flat_map(|chunk| {
            [
                Point::new(chunk[0], chunk[1]),
                Point::new(chunk[2], chunk[3]),
            ]
        })
        .collect::<Vec<_>>();
    if points.len() >= 2 {
        return Rect::from_points(&points).map(|bounds| nonzero_marker_bounds(bounds.expanded(1)));
    }
    Some(nonzero_marker_bounds(
        Rect::new(
            Point::new(coords[0], coords[1]),
            Point::new(coords[2], coords[3]),
        )
        .expanded(1),
    ))
}

pub(crate) fn calibre_point_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    calibre_point_bounds_from_coords(&coords)
}

fn calibre_point_bounds_from_coords(coords: &[Coord]) -> Option<Rect> {
    if coords.len() < 2 {
        return None;
    }
    Some(nonzero_marker_bounds(Rect::new(
        Point::new(coords[0], coords[1]),
        Point::new(coords[0], coords[1]),
    )))
}

pub(crate) fn calibre_circle_bounds(line: &str) -> Option<Rect> {
    let coords = calibre_geometry_coords(line);
    calibre_circle_bounds_from_coords(&coords)
}

fn calibre_circle_bounds_from_coords(coords: &[Coord]) -> Option<Rect> {
    if coords.len() < 3 {
        return None;
    }
    let radius = coords[2].saturating_abs();
    Some(nonzero_marker_bounds(Rect::new(
        Point::new(coords[0] - radius, coords[1] - radius),
        Point::new(coords[0] + radius, coords[1] + radius),
    )))
}

pub(crate) fn calibre_geometry_coords(line: &str) -> Vec<Coord> {
    let values = calibre_geometry_raw_coords(line);
    calibre_geometry_kind_from_line(line)
        .map(|kind| calibre_geometry_coords_from_raw(kind, &values))
        .unwrap_or(values)
}

fn calibre_geometry_coords_from_raw(kind: CalibreGeometryKind, values: &[Coord]) -> Vec<Coord> {
    let prefix = calibre_geometry_coord_prefix(kind, values);
    values[prefix.coord_start..].to_vec()
}

fn calibre_geometry_coord_prefix(
    kind: CalibreGeometryKind,
    coords: &[Coord],
) -> CalibreGeometryCoordPrefix {
    match kind {
        CalibreGeometryKind::Polygon => calibre_polygon_coord_prefix(coords),
        CalibreGeometryKind::Edge => calibre_edge_coord_prefix(coords),
        CalibreGeometryKind::Rect | CalibreGeometryKind::Point | CalibreGeometryKind::Circle => {
            CalibreGeometryCoordPrefix::default()
        }
    }
}

fn calibre_polygon_coord_prefix(coords: &[Coord]) -> CalibreGeometryCoordPrefix {
    if let Some(count) = coords
        .first()
        .copied()
        .filter(|count| *count >= 2)
        .map(|count| count as usize * 2)
        && (coords.len() == 1 || coords.len() == count + 1)
    {
        return CalibreGeometryCoordPrefix {
            coord_start: 1,
            expected_coords: Some(count),
        };
    }
    if let Some(count) = coords
        .get(1)
        .copied()
        .filter(|count| *count >= 2)
        .map(|count| count as usize * 2)
        && (coords.len() == 2 || coords.len() == count + 2)
    {
        return CalibreGeometryCoordPrefix {
            coord_start: 2,
            expected_coords: Some(count),
        };
    }
    CalibreGeometryCoordPrefix::default()
}

fn calibre_edge_coord_prefix(coords: &[Coord]) -> CalibreGeometryCoordPrefix {
    if let Some(count) = coords
        .get(1)
        .copied()
        .filter(|count| *count > 0)
        .map(|count| count as usize * 4)
        && (coords.len() == 2 || coords.len() == count + 2)
    {
        return CalibreGeometryCoordPrefix {
            coord_start: 2,
            expected_coords: Some(count),
        };
    }
    CalibreGeometryCoordPrefix::default()
}

fn calibre_geometry_raw_coords(line: &str) -> Vec<Coord> {
    let tokens = calibre_coord_tokens(line);
    let mut coords = Vec::new();
    if let Some(first) = tokens.first().copied()
        && let Some(coord) = calibre_attached_geometry_label_coord(first)
    {
        coords.push(coord);
    }
    let mut index = 1;
    while index < tokens.len() {
        let (coord, consumed) = parse_calibre_coord_token(tokens[index], tokens.get(index + 1));
        if let Some(coord) = coord {
            coords.push(coord);
        }
        index += consumed;
    }
    coords
}

fn calibre_attached_geometry_label_coord(token: &str) -> Option<Coord> {
    let (keyword, value) = token.split_once('=')?;
    let keyword = keyword.to_ascii_lowercase();
    calibre_geometry_kind(&keyword)?;
    parse_calibre_coord(&unquote_calibre_metadata_value(value))
}

fn calibre_line_coords(line: &str) -> Vec<Coord> {
    let tokens = calibre_coord_tokens(line);
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut coords = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let (coord, consumed) = parse_calibre_coord_token(tokens[index], tokens.get(index + 1));
        let Some(coord) = coord else {
            return Vec::new();
        };
        coords.push(coord);
        index += consumed;
    }
    coords
}

fn calibre_coord_tokens(line: &str) -> Vec<&str> {
    line.split(|character: char| {
        character.is_whitespace() || matches!(character, '(' | ')' | '[' | ']' | ',' | ';' | ':')
    })
    .filter(|token| !token.trim().is_empty())
    .collect()
}

pub(crate) fn parse_calibre_coord(value: &str) -> Option<Coord> {
    let value = value
        .trim()
        .trim_matches(|character| matches!(character, '[' | ']'));
    if value.is_empty() {
        return None;
    }
    value
        .parse::<Coord>()
        .ok()
        .or_else(|| value.parse::<f64>().ok().and_then(round_calibre_coord))
        .or_else(|| parse_calibre_keyed_coord(value))
        .or_else(|| parse_calibre_unit_coord(value))
}

fn parse_calibre_keyed_coord(value: &str) -> Option<Coord> {
    let (key, value) = value.split_once('=')?;
    let key = canonical_calibre_geometry_coord_key(&sanitize_calibre_metadata_key(key));
    if !is_calibre_geometry_coord_key(&key) {
        return None;
    }
    parse_calibre_coord(&unquote_calibre_metadata_value(value))
}

fn parse_calibre_coord_token(value: &str, next_token: Option<&&str>) -> (Option<Coord>, usize) {
    let value = unquote_calibre_metadata_value(value);
    if calibre_coord_can_take_separated_unit(&value)
        && let Some(unit) = next_token.copied()
        && is_calibre_measure_unit(unit)
    {
        return (parse_calibre_coord(&format!("{value}{unit}")), 2);
    }
    (parse_calibre_coord(&value), 1)
}

fn calibre_coord_can_take_separated_unit(value: &str) -> bool {
    value
        .trim()
        .trim_matches(|character| matches!(character, '[' | ']'))
        .parse::<f64>()
        .is_ok_and(|value| value.is_finite())
}

fn canonical_calibre_geometry_coord_key(key: &str) -> String {
    if let Some(index) = calibre_indexed_geometry_key_suffix(key, &["x", "y"]) {
        return format!("{}{}", &key[..1], index);
    }
    for prefix in ["vertex", "point", "pt", "xy", "v", "p"] {
        if let Some(index) = calibre_indexed_geometry_key_suffix(key, &[prefix]) {
            return format!("{prefix}{index}");
        }
    }
    key.to_string()
}

fn calibre_indexed_geometry_key_suffix(key: &str, prefixes: &[&str]) -> Option<usize> {
    prefixes
        .iter()
        .find_map(|prefix| key.strip_prefix(prefix))
        .map(|suffix| suffix.trim_start_matches('_'))
        .filter(|suffix| {
            !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
        })
        .and_then(|suffix| suffix.parse::<usize>().ok())
}

fn is_calibre_geometry_coord_key(key: &str) -> bool {
    matches!(
        key,
        "x" | "y"
            | "x0"
            | "y0"
            | "x1"
            | "y1"
            | "x2"
            | "y2"
            | "x_min"
            | "y_min"
            | "x_max"
            | "y_max"
            | "xmin"
            | "ymin"
            | "xmax"
            | "ymax"
            | "xlo"
            | "ylo"
            | "xhi"
            | "yhi"
            | "left"
            | "bottom"
            | "right"
            | "top"
            | "llx"
            | "lly"
            | "urx"
            | "ury"
            | "cx"
            | "cy"
            | "xc"
            | "yc"
            | "center_x"
            | "center_y"
            | "centre_x"
            | "centre_y"
            | "x_center"
            | "y_center"
            | "xcentre"
            | "ycentre"
            | "origin_x"
            | "origin_y"
            | "originx"
            | "originy"
            | "width"
            | "height"
            | "w"
            | "h"
            | "dx"
            | "dy"
            | "x_size"
            | "y_size"
            | "xsize"
            | "ysize"
            | "size_x"
            | "size_y"
            | "sizex"
            | "sizey"
            | "radius"
            | "rad"
            | "diameter"
            | "diam"
            | "dia"
            | "d"
            | "r"
    ) || calibre_indexed_geometry_key_suffix(key, &["x", "y"]).is_some()
}

fn parse_calibre_unit_coord(value: &str) -> Option<Coord> {
    let (number, unit) = split_calibre_unit_coord(value)?;
    let scale = calibre_unit_scale(unit)?;
    number
        .parse::<f64>()
        .ok()
        .and_then(|coord| round_calibre_coord(coord * scale))
}

fn calibre_unit_scale(unit: &str) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "dbu" | "databaseunit" | "databaseunits" => Some(1.0),
        "um" | "u" | "micron" | "microns" | "micrometer" | "micrometers" | "micrometre"
        | "micrometres" => Some(geometry_core::DBU_PER_MICRON as f64),
        "nm" | "nanometer" | "nanometers" | "nanometre" | "nanometres" => {
            Some(geometry_core::DBU_PER_MICRON as f64 / 1000.0)
        }
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
            Some(geometry_core::DBU_PER_MICRON as f64 * 1000.0)
        }
        _ => None,
    }
}

fn split_calibre_unit_coord(value: &str) -> Option<(&str, &str)> {
    let value = value.trim();
    let mut number_end = 0;
    let mut saw_digit = false;
    let mut previous_was_exponent = false;
    for (index, character) in value.char_indices() {
        let allowed_sign = matches!(character, '+' | '-') && (index == 0 || previous_was_exponent);
        if character.is_ascii_digit() {
            saw_digit = true;
            previous_was_exponent = false;
            number_end = index + character.len_utf8();
        } else if character == '.' || allowed_sign {
            previous_was_exponent = false;
            number_end = index + character.len_utf8();
        } else if matches!(character, 'e' | 'E') && saw_digit {
            previous_was_exponent = true;
            number_end = index + character.len_utf8();
        } else {
            break;
        }
    }
    if !saw_digit || number_end == 0 {
        return None;
    }
    let number = value[..number_end].trim();
    let unit = value[number_end..].trim();
    if number.is_empty() || unit.is_empty() {
        return None;
    }
    Some((number, unit))
}

fn round_calibre_coord(coord: f64) -> Option<Coord> {
    if coord.is_finite() {
        Some(coord.round() as Coord)
    } else {
        None
    }
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
    if line.is_empty() {
        return None;
    }
    let normalized_prefix = line
        .split_once(':')
        .or_else(|| line.split_once('='))
        .map(|(prefix, value)| (sanitize_calibre_metadata_key(prefix), value.trim()));
    if let Some((prefix, value)) = normalized_prefix
        && matches!(
            prefix.as_str(),
            "rule"
                | "rule_name"
                | "rulename"
                | "rule_check"
                | "rule_check_name"
                | "rulecheck"
                | "rulecheck_name"
                | "rulecheckname"
                | "check"
                | "check_name"
                | "checkname"
        )
    {
        let rule = unquote_calibre_metadata_value(value);
        return (!rule.is_empty()).then_some(rule);
    }
    if line.contains(char::is_whitespace) && line.split_whitespace().count() > 8 {
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
    let rule = unquote_calibre_metadata_value(rule);
    (!rule.is_empty()).then_some(rule)
}

fn is_calibre_rve_record_delimiter_line(line: &str) -> bool {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let Some(first) = tokens.first() else {
        return false;
    };
    let first_key = sanitize_calibre_metadata_key(first);
    if !matches!(
        first_key.as_str(),
        "result" | "results" | "marker" | "markers" | "violation" | "violations"
    ) {
        return false;
    }
    tokens.len() == 1
        || tokens[1..]
            .iter()
            .all(|token| is_calibre_rve_record_delimiter_token(token))
}

fn is_calibre_rve_record_delimiter_token(token: &str) -> bool {
    let token = token.trim_matches(|character: char| {
        matches!(character, '#' | ':' | '(' | ')' | '[' | ']' | ',' | ';')
    });
    token == "/"
        || token.eq_ignore_ascii_case("of")
        || !token.is_empty() && token.chars().all(|character| character.is_ascii_digit())
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
