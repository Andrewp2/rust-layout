#![allow(unused_imports)]
use super::*;
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayoutDrcReportDatabaseImportMode {
    Replace,
    Append,
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

impl GlassworksApp {
    pub(crate) fn first_attention_die(&self) -> Option<DieCoord> {
        self.workspace
            .wafer_map
            .measurements
            .iter()
            .find(|measurement| {
                measurement.kind == self.metrology_kind
                    && measurement.status != MeasurementStatus::Pass
            })
            .or_else(|| {
                self.workspace
                    .wafer_map
                    .measurements
                    .iter()
                    .find(|measurement| measurement.status != MeasurementStatus::Pass)
            })
            .map(|measurement| measurement.die)
            .or_else(|| {
                self.workspace
                    .wafer_map
                    .defects
                    .first()
                    .map(|defect| defect.die)
            })
    }

    pub(crate) fn experiment_pending_count(&self) -> usize {
        let response_count = self.workspace.experiment_plan.responses.len();
        self.workspace
            .experiment_plan
            .runs
            .iter()
            .filter(|run| response_count == 0 || run.responses.len() < response_count)
            .count()
    }

    pub(crate) fn active_layout_drc_deck_for(&self, document: &Document) -> RuleDeck {
        self.layout_custom_drc_deck.clone().unwrap_or_else(|| {
            RuleDeck::from_technology(document, self.active_layout_technology())
                .expect("active layout technology applies to the document")
        })
    }

    pub(crate) fn active_layout_drc_deck_label(&self) -> String {
        self.layout_custom_drc_deck_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .map(|label| format!("custom {label}"))
            .unwrap_or_else(|| {
                format!(
                    "built-in {}",
                    display_technology_name(&self.active_layout_technology().name)
                )
            })
    }

    pub(crate) fn run_drc_summary(&mut self) -> String {
        let rules = self.active_layout_drc_deck_for(&self.workspace.document);
        let deck_label = self.active_layout_drc_deck_label();
        let rule_findings = rules.validate_for_document(&self.workspace.document);
        let value = if rule_findings.is_empty() {
            DrcReportCacheValue {
                findings: Vec::new(),
                violations: run_drc(&self.workspace.document, &rules),
            }
        } else {
            DrcReportCacheValue {
                findings: rule_findings,
                violations: Vec::new(),
            }
        };
        let finding_count = value.findings.len();
        let violation_count = value.violations.len();
        self.push_layout_drc_report("Full DRC", value);
        if finding_count > 0 {
            return format!("DRC rule deck {deck_label} has {finding_count} issue(s)");
        }
        format!("DRC found {violation_count} violation(s) using {deck_label} deck")
    }

    pub(crate) fn run_drc_for_selected_region_summary(&mut self) -> String {
        let Some((region_cell, _, shape)) = self.selected_current_cell_layout_shape() else {
            return "Select a current-cell rectangle or polygon before running region DRC"
                .to_string();
        };
        let Some(region_parts) = convex_region_parts_for_shape_kind(&shape.kind) else {
            return "Region DRC requires a usable polygon region".to_string();
        };

        let mut scoped_document = self.workspace.document.clone();
        if region_cell != self.workspace.document.top_cell {
            scoped_document.shapes.clear();
        }
        scoped_document.top_cell = region_cell;
        let rules = self.active_layout_drc_deck_for(&scoped_document);
        let deck_label = self.active_layout_drc_deck_label();
        let rule_findings = rules.validate_for_document(&scoped_document);
        let mut full_violation_count = 0;
        let value = if rule_findings.is_empty() {
            let violations = run_drc(&scoped_document, &rules);
            full_violation_count = violations.len();
            DrcReportCacheValue {
                findings: Vec::new(),
                violations: violations
                    .into_iter()
                    .filter(|violation| {
                        drc_violation_intersects_any_region(violation, &region_parts)
                    })
                    .collect(),
            }
        } else {
            DrcReportCacheValue {
                findings: rule_findings,
                violations: Vec::new(),
            }
        };
        let finding_count = value.findings.len();
        let violation_count = value.violations.len();
        self.push_layout_drc_report("Selected Region DRC", value);
        if finding_count > 0 {
            return format!("DRC rule deck {deck_label} has {finding_count} issue(s)");
        }
        format!(
            "DRC region found {violation_count} of {full_violation_count} violation(s) using {deck_label} deck"
        )
    }

    pub(crate) fn run_drc_for_current_cell_summary(&mut self) -> String {
        let Some(cell) = self.workspace.document.cell(self.layout_view_top_cell) else {
            return format!(
                "Cannot run cell DRC because cell #{} is missing",
                self.layout_view_top_cell.0
            );
        };
        let cell_name = cell.name.clone();
        let mut scoped_document = self.workspace.document.clone();
        if self.layout_view_top_cell != self.workspace.document.top_cell {
            scoped_document.shapes.clear();
        }
        scoped_document.top_cell = self.layout_view_top_cell;
        let rules = self.active_layout_drc_deck_for(&scoped_document);
        let deck_label = self.active_layout_drc_deck_label();
        let rule_findings = rules.validate_for_document(&scoped_document);
        let value = if rule_findings.is_empty() {
            DrcReportCacheValue {
                findings: Vec::new(),
                violations: run_drc(&scoped_document, &rules),
            }
        } else {
            DrcReportCacheValue {
                findings: rule_findings,
                violations: Vec::new(),
            }
        };
        let finding_count = value.findings.len();
        let violation_count = value.violations.len();
        self.push_layout_drc_report(format!("Cell DRC {cell_name}"), value);
        if finding_count > 0 {
            return format!("DRC rule deck {deck_label} has {finding_count} issue(s)");
        }
        format!("DRC cell {cell_name} found {violation_count} violation(s) using {deck_label} deck")
    }

    pub(crate) fn export_layout_drc_deck(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_deck_exchange_path();
            return self.export_layout_drc_deck_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DRC deck export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_drc_deck_to_path(&mut self, path: &Path) -> bool {
        let deck = self.active_layout_drc_deck_for(&self.workspace.document);
        let findings = deck.validate_for_document(&self.workspace.document);
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .map(|finding| finding.message.clone())
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            self.status_message = format!(
                "DRC deck export failed: {}",
                errors.into_iter().take(3).collect::<Vec<_>>().join("; ")
            );
            return true;
        }
        let exchange = LayoutDrcDeckExchange::from_app(self, &deck);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("DRC deck export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("DRC deck export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("drc_deck", path);
        self.status_message = format!(
            "Exported DRC deck {} ({} derived layer, {} derived min-width, {} derived max-width, {} derived min-area, {} derived max-area, {} derived spacing, {} derived edge-spacing, {} derived overlap, {} min-width, {} max-width, {} min-area, {} max-area, {} spacing, {} edge-spacing, {} enclosure, {} overlap rule(s))",
            path.display(),
            exchange.derived_layers.len(),
            exchange.derived_min_width.len(),
            exchange.derived_max_width.len(),
            exchange.derived_min_area.len(),
            exchange.derived_max_area.len(),
            exchange.derived_min_spacing.len(),
            exchange.derived_min_edge_spacing.len(),
            exchange.derived_forbidden_overlaps.len(),
            exchange.min_width.len(),
            exchange.max_width.len(),
            exchange.min_area.len(),
            exchange.max_area.len(),
            exchange.min_spacing.len(),
            exchange.min_edge_spacing.len(),
            exchange.via_enclosure.len(),
            exchange.forbidden_overlaps.len()
        );
        true
    }

    pub(crate) fn import_layout_drc_deck(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_deck_exchange_path();
            return self.import_layout_drc_deck_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DRC deck import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_drc_deck_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DRC deck import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutDrcDeckExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("DRC deck import failed: {error}");
                return true;
            }
        };
        let rule_count = exchange.min_width.len()
            + exchange.max_width.len()
            + exchange.min_area.len()
            + exchange.max_area.len()
            + exchange.derived_layers.len()
            + exchange.derived_min_width.len()
            + exchange.derived_max_width.len()
            + exchange.derived_min_area.len()
            + exchange.derived_max_area.len()
            + exchange.derived_min_spacing.len()
            + exchange.derived_min_edge_spacing.len()
            + exchange.derived_forbidden_overlaps.len()
            + exchange.min_spacing.len()
            + exchange.min_edge_spacing.len()
            + exchange.via_enclosure.len()
            + exchange.forbidden_overlaps.len();
        let deck = match exchange.into_rule_deck(&self.workspace.document) {
            Ok(deck) => deck,
            Err(error) => {
                self.status_message = format!("DRC deck import failed: {error}");
                return true;
            }
        };
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("custom DRC deck")
            .to_string();
        self.layout_custom_drc_deck = Some(deck);
        self.layout_custom_drc_deck_label = Some(label.clone());
        self.drc_report_cache.get_mut().take();
        self.record_recent_layout_file("drc_deck", path);
        self.status_message = format!(
            "Imported DRC deck {} ({} rule(s)); future DRC runs use custom deck",
            path.display(),
            rule_count
        );
        true
    }

    pub(crate) fn reset_layout_drc_deck(&mut self) -> bool {
        if self.layout_custom_drc_deck.is_none() {
            self.status_message = "DRC already uses the built-in deck".to_string();
            return true;
        }
        self.layout_custom_drc_deck = None;
        self.layout_custom_drc_deck_label = None;
        self.drc_report_cache.get_mut().take();
        self.status_message = "DRC uses the built-in deck".to_string();
        true
    }

    pub(crate) fn export_layout_drc_report(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_report_exchange_path();
            return self.export_layout_drc_report_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DRC report export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_drc_report_to_path(&mut self, path: &Path) -> bool {
        let Some(report) = self.drc_report() else {
            self.status_message = "Run DRC before exporting a report".to_string();
            return true;
        };
        let report_keys = report
            .violations
            .iter()
            .map(DrcViolation::stable_key)
            .collect::<BTreeSet<_>>();
        let marker_states = self
            .workspace
            .document
            .marker_states
            .iter()
            .filter(|(key, _)| report_keys.contains(*key))
            .map(|(key, state)| (key.clone(), state.clone()))
            .collect::<BTreeMap<_, _>>();
        let exchange = LayoutDrcReportExchange {
            schema_version: LAYOUT_DRC_REPORT_EXCHANGE_SCHEMA_VERSION,
            document_id: self.workspace.document.id.to_string(),
            document_name: self.workspace.document.name.clone(),
            layout_revision: self.layout_revision,
            findings: report.findings,
            violations: report.violations,
            marker_states,
        };
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("DRC report export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("DRC report export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("drc_report", path);
        self.status_message = format!(
            "Exported DRC report {} ({} marker(s), {} state(s), {} diagnostic(s))",
            path.display(),
            exchange.violations.len(),
            exchange.marker_states.len(),
            exchange.findings.len()
        );
        true
    }

    pub(crate) fn import_layout_drc_report(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_report_exchange_path();
            return self.import_layout_drc_report_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "DRC report import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_drc_report_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DRC report import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutDrcReportExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("DRC report import failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_DRC_REPORT_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "DRC report import failed: report schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_DRC_REPORT_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        let issue_store = DrcIssueStore::from_violations(exchange.violations.clone());
        let errors = issue_store
            .validate()
            .into_iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .map(|finding| finding.message)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            self.status_message = format!(
                "DRC report import failed: {}",
                errors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            return true;
        }
        if let Err(error) = Self::validate_drc_marker_states(&exchange.marker_states) {
            self.status_message = format!("DRC report import failed: {error}");
            return true;
        }
        let report_keys = exchange
            .violations
            .iter()
            .map(DrcViolation::stable_key)
            .collect::<BTreeSet<_>>();
        for key in &report_keys {
            if !exchange.marker_states.contains_key(key) {
                self.workspace.document.marker_states.remove(key);
            }
        }
        for (key, state) in exchange.marker_states {
            if state == MarkerState::default() {
                self.workspace.document.marker_states.remove(&key);
            } else {
                self.workspace.document.marker_states.insert(key, state);
            }
        }
        let marker_count = exchange.violations.len();
        let finding_count = exchange.findings.len();
        let label = format!(
            "Imported {}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("DRC report")
        );
        self.push_layout_drc_report(
            label,
            DrcReportCacheValue {
                findings: exchange.findings,
                violations: exchange.violations,
            },
        );
        self.show_drc_overlay = true;
        self.record_recent_layout_file("drc_report", path);
        self.status_message = format!(
            "Imported DRC report {} ({} marker(s), {} diagnostic(s))",
            path.display(),
            marker_count,
            finding_count
        );
        true
    }

    pub(crate) fn export_layout_drc_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_report_database_exchange_path();
            return self.export_layout_drc_report_database_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "DRC report database export is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn export_layout_klayout_rdb_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_klayout_rdb_exchange_path();
            return self.export_layout_drc_report_database_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "KLayout RDB report database export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_drc_report_database_to_path(&mut self, path: &Path) -> bool {
        let reports = if self.layout_drc_report_history.is_empty() {
            let Some(report) = self.drc_report() else {
                self.status_message = "Run or import DRC before exporting reports".to_string();
                return true;
            };
            vec![LayoutDrcReportDatabaseEntryExchange {
                label: "Active DRC Report".to_string(),
                source: None,
                layout_revision: self.layout_revision,
                findings: report.findings,
                violations: report.violations,
            }]
        } else {
            self.layout_drc_report_history
                .iter()
                .map(|entry| LayoutDrcReportDatabaseEntryExchange {
                    label: entry.label.clone(),
                    source: entry.source.clone(),
                    layout_revision: entry.revision,
                    findings: entry.value.findings.clone(),
                    violations: entry.value.violations.clone(),
                })
                .collect::<Vec<_>>()
        };
        let active_report_index = self
            .layout_selected_drc_report_id
            .and_then(|selected_id| {
                self.layout_drc_report_history
                    .iter()
                    .position(|entry| entry.id == selected_id)
            })
            .or_else(|| (reports.len() == 1).then_some(0));
        let report_keys = reports
            .iter()
            .flat_map(|report| report.violations.iter().map(DrcViolation::stable_key))
            .collect::<BTreeSet<_>>();
        let marker_states = self
            .workspace
            .document
            .marker_states
            .iter()
            .filter(|(key, _)| report_keys.contains(*key))
            .map(|(key, state)| (key.clone(), state.clone()))
            .collect::<BTreeMap<_, _>>();
        let marker_count = reports
            .iter()
            .map(|report| report.violations.len())
            .sum::<usize>();
        let finding_count = reports
            .iter()
            .map(|report| report.findings.len())
            .sum::<usize>();
        if layout_drc_report_database_path_is_klayout_rdb(path) {
            let bytes = layout_klayout_rdb_report_database_xml(
                &self.workspace.document,
                &reports,
                &marker_states,
            );
            if let Err(error) = atomic_write_klayout_rdb_report_database_bytes(path, &bytes) {
                self.status_message = format!("KLayout RDB report database export failed: {error}");
                return true;
            }
            self.record_recent_layout_file("drc_report_database", path);
            self.status_message = format!(
                "Exported KLayout RDB report database {} ({} report(s), {} marker(s), {} state(s), {} diagnostic item(s))",
                path.display(),
                reports.len(),
                marker_count,
                marker_states.len(),
                finding_count
            );
            return true;
        }
        let exchange = LayoutDrcReportDatabaseExchange {
            schema_version: LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION,
            document_id: self.workspace.document.id.to_string(),
            document_name: self.workspace.document.name.clone(),
            layout_revision: self.layout_revision,
            active_report_index,
            reports,
            marker_states,
        };
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("DRC report database export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("DRC report database export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("drc_report_database", path);
        self.status_message = format!(
            "Exported DRC report database {} ({} report(s), {} marker(s), {} state(s), {} diagnostic(s))",
            path.display(),
            exchange.reports.len(),
            marker_count,
            exchange.marker_states.len(),
            finding_count
        );
        true
    }

    pub(crate) fn import_layout_drc_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_report_database_exchange_path();
            return self.import_layout_drc_report_database_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "DRC report database import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_klayout_rdb_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_klayout_rdb_exchange_path();
            return self.import_layout_drc_report_database_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "KLayout RDB report database import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn append_layout_drc_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_report_database_exchange_path();
            return self.append_layout_drc_report_database_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "DRC report database append is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn append_layout_klayout_rdb_report_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_klayout_rdb_exchange_path();
            return self.append_layout_drc_report_database_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "KLayout RDB report database append is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_drc_report_database_from_path(&mut self, path: &Path) -> bool {
        self.load_layout_drc_report_database_from_path(
            path,
            LayoutDrcReportDatabaseImportMode::Replace,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn append_layout_drc_report_database_from_path(&mut self, path: &Path) -> bool {
        self.load_layout_drc_report_database_from_path(
            path,
            LayoutDrcReportDatabaseImportMode::Append,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_layout_drc_report_database_from_path(
        &mut self,
        path: &Path,
        mode: LayoutDrcReportDatabaseImportMode,
    ) -> bool {
        let action = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => "import",
            LayoutDrcReportDatabaseImportMode::Append => "append",
        };
        let action_past = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => "Imported",
            LayoutDrcReportDatabaseImportMode::Append => "Appended",
        };
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DRC report database {action} failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutDrcReportDatabaseExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                if layout_drc_report_database_contents_looks_like_klayout_rdb(&contents) {
                    return self.load_layout_klayout_rdb_report_database_from_contents(
                        path, &contents, mode,
                    );
                }
                self.status_message = format!("DRC report database {action} failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "DRC report database {action} failed: database schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        if exchange.reports.is_empty() {
            self.status_message =
                format!("DRC report database {action} failed: database has no reports");
            return true;
        }
        let available_slots = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => MAX_LAYOUT_DRC_REPORT_HISTORY,
            LayoutDrcReportDatabaseImportMode::Append => {
                MAX_LAYOUT_DRC_REPORT_HISTORY.saturating_sub(self.layout_drc_report_history.len())
            }
        };
        if available_slots == 0 {
            self.status_message = format!(
                "DRC report database append skipped: report history already has {} report(s)",
                self.layout_drc_report_history.len()
            );
            return true;
        }
        for report in &exchange.reports {
            let issue_store = DrcIssueStore::from_violations(report.violations.clone());
            let errors = issue_store
                .validate()
                .into_iter()
                .filter(|finding| finding.severity == DrcValidationSeverity::Error)
                .map(|finding| finding.message)
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                let label = report.label.trim();
                let label = if label.is_empty() {
                    "unnamed report"
                } else {
                    label
                };
                self.status_message = format!(
                    "DRC report database {action} failed in {label}: {}",
                    errors
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                );
                return true;
            }
        }
        if let Err(error) = Self::validate_drc_marker_states(&exchange.marker_states) {
            self.status_message = format!("DRC report database {action} failed: {error}");
            return true;
        }

        let original_report_count = exchange.reports.len();
        let report_limit = original_report_count.min(available_slots);
        let exchange_active_report_index = exchange.active_report_index;
        let report_keys = exchange
            .reports
            .iter()
            .take(report_limit)
            .flat_map(|report| report.violations.iter().map(DrcViolation::stable_key))
            .collect::<BTreeSet<_>>();
        for key in &report_keys {
            if !exchange.marker_states.contains_key(key) {
                self.workspace.document.marker_states.remove(key);
            }
        }
        let mut imported_state_count = 0usize;
        for (key, state) in exchange.marker_states {
            if !report_keys.contains(&key) {
                continue;
            }
            if state == MarkerState::default() {
                self.workspace.document.marker_states.remove(&key);
            } else {
                self.workspace.document.marker_states.insert(key, state);
                imported_state_count += 1;
            }
        }

        let mut next_id = self.layout_next_drc_report_id;
        let mut history = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => Vec::new(),
            LayoutDrcReportDatabaseImportMode::Append => self.layout_drc_report_history.clone(),
        };
        let imported_history_start = history.len();
        let mut marker_count = 0usize;
        let mut finding_count = 0usize;
        for (index, report) in exchange.reports.into_iter().take(report_limit).enumerate() {
            let label = if report.label.trim().is_empty() {
                format!("Report {}", index + 1)
            } else {
                report.label.trim().chars().take(48).collect::<String>()
            };
            marker_count += report.violations.len();
            finding_count += report.findings.len();
            let id = next_id;
            next_id = next_id.wrapping_add(1).max(1);
            let source = report
                .source
                .filter(|source| !source.trim().is_empty())
                .or_else(|| Some(path.display().to_string()));
            history.push(DrcReportHistoryEntry {
                id,
                revision: report.layout_revision,
                label,
                source,
                value: DrcReportCacheValue {
                    findings: report.findings,
                    violations: report.violations,
                },
            });
        }
        let imported_active_index = exchange_active_report_index
            .filter(|index| *index < report_limit)
            .unwrap_or(0);
        let active_index = imported_history_start + imported_active_index;
        self.layout_next_drc_report_id = next_id;
        self.layout_selected_drc_report_id = history.get(active_index).map(|entry| entry.id);
        self.layout_drc_report_history = history;
        self.layout_selected_drc_marker_key = None;
        self.layout_drc_marker_category_filter = None;
        self.layout_drc_marker_directory_filter = None;
        self.sync_selected_layout_drc_report_cache();
        self.show_drc_overlay = true;
        self.record_recent_layout_file("drc_report_database", path);
        let truncated_suffix = if original_report_count > report_limit {
            format!("; truncated to {report_limit}")
        } else {
            String::new()
        };
        let total_suffix = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => String::new(),
            LayoutDrcReportDatabaseImportMode::Append => {
                format!("; {} total report(s)", self.layout_drc_report_history.len())
            }
        };
        self.status_message = format!(
            "{action_past} DRC report database {} ({} report(s), {} marker(s), {} state(s), {} diagnostic(s)){truncated_suffix}{total_suffix}",
            path.display(),
            report_limit,
            marker_count,
            imported_state_count,
            finding_count
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_layout_klayout_rdb_report_database_from_contents(
        &mut self,
        path: &Path,
        contents: &str,
        mode: LayoutDrcReportDatabaseImportMode,
    ) -> bool {
        let action = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => "import",
            LayoutDrcReportDatabaseImportMode::Append => "append",
        };
        let action_past = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => "Imported",
            LayoutDrcReportDatabaseImportMode::Append => "Appended",
        };
        let available_slots = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => MAX_LAYOUT_DRC_REPORT_HISTORY,
            LayoutDrcReportDatabaseImportMode::Append => {
                MAX_LAYOUT_DRC_REPORT_HISTORY.saturating_sub(self.layout_drc_report_history.len())
            }
        };
        if available_slots == 0 {
            self.status_message = format!(
                "KLayout RDB report database append skipped: report history already has {} report(s)",
                self.layout_drc_report_history.len()
            );
            return true;
        }

        let import_context = KlayoutRdbImportContext::from_document(&self.workspace.document);
        let imported = match import_klayout_rdb_markers_with_context(contents, &import_context) {
            Ok(imported) => imported,
            Err(error) => {
                self.status_message =
                    format!("KLayout RDB report database {action} failed: {error}");
                return true;
            }
        };
        let diagnostic_count = imported.findings.len() + imported.report.warnings.len();
        if imported.violations.is_empty() && diagnostic_count == 0 {
            self.status_message = format!(
                "KLayout RDB report database {action} found no markers in {}",
                path.display()
            );
            return true;
        }
        let issue_store = DrcIssueStore::from_violations(imported.violations.clone());
        let errors = issue_store
            .validate()
            .into_iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .map(|finding| finding.message)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            self.status_message = format!(
                "KLayout RDB report database {action} failed: {}",
                errors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            return true;
        }
        if let Err(error) = Self::validate_drc_marker_states(&imported.marker_states) {
            self.status_message = format!("KLayout RDB report database {action} failed: {error}");
            return true;
        }

        let report_keys = imported
            .violations
            .iter()
            .map(DrcViolation::stable_key)
            .collect::<BTreeSet<_>>();
        for key in &report_keys {
            if !imported.marker_states.contains_key(key) {
                self.workspace.document.marker_states.remove(key);
            }
        }
        let mut imported_state_count = 0usize;
        for (key, state) in imported.marker_states {
            if !report_keys.contains(&key) {
                continue;
            }
            if state == MarkerState::default() {
                self.workspace.document.marker_states.remove(&key);
            } else {
                self.workspace.document.marker_states.insert(key, state);
                imported_state_count += 1;
            }
        }

        let mut history = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => Vec::new(),
            LayoutDrcReportDatabaseImportMode::Append => self.layout_drc_report_history.clone(),
        };
        let id = self.layout_next_drc_report_id;
        self.layout_next_drc_report_id = self.layout_next_drc_report_id.wrapping_add(1).max(1);
        let label = layout_klayout_rdb_report_label(path, imported.report.description.as_deref());
        let marker_count = imported.violations.len();
        let skipped_item_count = imported.report.skipped_item_count;
        let mut findings = imported.findings;
        findings.extend(
            imported
                .report
                .warnings
                .into_iter()
                .map(|warning| DrcValidationFinding {
                    severity: DrcValidationSeverity::Warning,
                    message: format!("KLayout RDB import warning: {warning}"),
                }),
        );
        history.push(DrcReportHistoryEntry {
            id,
            revision: self.layout_revision,
            label,
            source: Some(path.display().to_string()),
            value: DrcReportCacheValue {
                findings,
                violations: imported.violations,
            },
        });
        self.layout_selected_drc_report_id = Some(id);
        self.layout_drc_report_history = history;
        self.layout_selected_drc_marker_key = None;
        self.layout_drc_marker_filter = LayoutDrcMarkerFilter::Active;
        self.layout_drc_marker_sort = LayoutDrcMarkerSort::Id;
        self.layout_drc_marker_category_filter = None;
        self.layout_drc_marker_directory_filter = None;
        self.layout_browser_search.clear();
        self.sync_selected_layout_drc_report_cache();
        self.show_drc_overlay = true;
        self.record_recent_layout_file("drc_report_database", path);
        let total_suffix = match mode {
            LayoutDrcReportDatabaseImportMode::Replace => String::new(),
            LayoutDrcReportDatabaseImportMode::Append => {
                format!("; {} total report(s)", self.layout_drc_report_history.len())
            }
        };
        self.status_message = format!(
            "{action_past} KLayout RDB report database {} (1 report(s), {} marker(s), {} state(s), {} diagnostic(s), {} skipped item(s)){total_suffix}",
            path.display(),
            marker_count,
            imported_state_count,
            diagnostic_count,
            skipped_item_count
        );
        true
    }

    pub(crate) fn import_layout_calibre_rve_markers(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_calibre_rve_exchange_path();
            return self.import_layout_calibre_rve_markers_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Calibre/RVE marker import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_calibre_rve_markers_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Calibre/RVE marker import failed: {error}");
                return true;
            }
        };
        let imported = import_calibre_rve_markers(&contents);
        let warning_suffix = imported
            .report
            .warnings
            .first()
            .map(|warning| format!("; warning: {warning}"))
            .unwrap_or_default();
        let import_findings = imported
            .report
            .warnings
            .iter()
            .map(|warning| DrcValidationFinding {
                severity: DrcValidationSeverity::Warning,
                message: format!("Calibre/RVE import warning: {warning}"),
            })
            .collect::<Vec<_>>();
        let label = format!(
            "Calibre/RVE {}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("markers")
        );
        if imported.violations.is_empty() {
            let diagnostic_count = import_findings.len();
            if diagnostic_count > 0 {
                self.push_layout_drc_report(
                    label,
                    DrcReportCacheValue {
                        findings: import_findings,
                        violations: Vec::new(),
                    },
                );
                self.layout_drc_marker_filter = LayoutDrcMarkerFilter::Active;
                self.layout_drc_marker_sort = LayoutDrcMarkerSort::Id;
                self.layout_drc_marker_category_filter = None;
                self.layout_drc_marker_directory_filter = None;
                self.layout_selected_drc_marker_key = None;
                self.layout_browser_search.clear();
                self.show_drc_overlay = true;
                self.record_recent_layout_file("calibre_rve", path);
            }
            self.status_message = format!(
                "Calibre/RVE marker import found no markers in {} ({} diagnostic(s), {} skipped line(s)){}",
                path.display(),
                diagnostic_count,
                imported.report.skipped_line_count,
                warning_suffix
            );
            return true;
        }
        let issue_store = DrcIssueStore::from_violations(imported.violations.clone());
        let errors = issue_store
            .validate()
            .into_iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .map(|finding| finding.message)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            self.status_message = format!(
                "Calibre/RVE marker import failed: {}",
                errors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            return true;
        }
        if let Err(error) = Self::validate_drc_marker_states(&imported.marker_states) {
            self.status_message = format!("Calibre/RVE marker import failed: {error}");
            return true;
        }
        let marker_count = imported.violations.len();
        let marker_state_count = imported.marker_states.len();
        let skipped_line_count = imported.report.skipped_line_count;
        self.push_layout_drc_report(
            label,
            DrcReportCacheValue {
                findings: import_findings,
                violations: imported.violations,
            },
        );
        for (key, state) in imported.marker_states {
            if state == MarkerState::default() {
                self.workspace.document.marker_states.remove(&key);
            } else {
                self.workspace.document.marker_states.insert(key, state);
            }
        }
        self.layout_drc_marker_filter = LayoutDrcMarkerFilter::Active;
        self.layout_drc_marker_sort = LayoutDrcMarkerSort::Id;
        self.layout_drc_marker_category_filter = None;
        self.layout_drc_marker_directory_filter = None;
        self.layout_selected_drc_marker_key = None;
        self.layout_browser_search.clear();
        self.show_drc_overlay = true;
        self.record_recent_layout_file("calibre_rve", path);
        self.status_message = format!(
            "Imported Calibre/RVE markers {} ({} marker(s), {} review state(s), {} skipped line(s)){}",
            path.display(),
            marker_count,
            marker_state_count,
            skipped_line_count,
            warning_suffix
        );
        true
    }

    pub(crate) fn write_drc_markers_to_layout_layer(&mut self) -> bool {
        let Some(report) = self.drc_report() else {
            self.status_message = "Run DRC before writing markers to a layer".to_string();
            return true;
        };
        let deck_error_count = report
            .findings
            .iter()
            .filter(|finding| finding.severity == DrcValidationSeverity::Error)
            .count();
        if deck_error_count > 0 {
            self.status_message =
                format!("Fix {deck_error_count} DRC rule-deck issue(s) before writing markers");
            return true;
        }
        let Some(output_layer) = self
            .workspace
            .document
            .layer_by_process(ProcessLayer::Annotation)
            .or_else(|| {
                self.workspace
                    .document
                    .layers
                    .contains_key(&self.active_layer)
                    .then_some(self.active_layer)
            })
            .or_else(|| self.workspace.document.layers.keys().next().copied())
        else {
            self.status_message = "Add a layer before writing DRC markers".to_string();
            return true;
        };
        let layer_name = layout_layer_display_name(&self.workspace.document, output_layer);
        let mut active_violations = report
            .violations
            .iter()
            .filter(|violation| drc_violation_is_active(&self.workspace.document, violation))
            .cloned()
            .collect::<Vec<_>>();
        active_violations.sort_by_key(|violation| violation.id);
        if active_violations.is_empty() {
            self.status_message = if report.violations.is_empty() {
                "DRC report has no markers to write".to_string()
            } else {
                "No active DRC markers to write".to_string()
            };
            return true;
        }

        let grid = self.workspace.document.grid.max(1);
        let mut shapes = Vec::with_capacity(active_violations.len() * 2);
        for violation in &active_violations {
            let bounds = layout_drc_marker_output_bounds(violation.bounds, grid);
            shapes.push(Shape {
                id: self.workspace.document.allocate_shape_id(),
                layer: output_layer,
                net: None,
                kind: ShapeKind::Rectangle(bounds),
                name: Some(format!("drc marker #{} {}", violation.id, violation.rule)),
                properties: BTreeMap::new(),
            });
            shapes.push(Shape {
                id: self.workspace.document.allocate_shape_id(),
                layer: output_layer,
                net: None,
                kind: ShapeKind::Label {
                    position: bounds.center().snap(grid),
                    text: format!("DRC #{} {}", violation.id, violation.rule),
                },
                name: Some(format!("drc marker #{} label", violation.id)),
                properties: BTreeMap::new(),
            });
        }
        let selected = shapes.first().map(|shape| shape.id);
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: shapes
                    .iter()
                    .cloned()
                    .map(|shape| Operation::AddShape { shape })
                    .collect(),
            },
            Operation::Batch {
                operations: shapes
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .collect(),
            },
        );
        self.active_layer = output_layer;
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        let skipped_count = report.violations.len() - active_violations.len();
        let skipped_suffix = if skipped_count == 0 {
            String::new()
        } else {
            format!(", skipped {skipped_count} hidden/waived")
        };
        self.status_message = format!(
            "Wrote {} active DRC marker(s) to {layer_name}{skipped_suffix}",
            active_violations.len()
        );
        true
    }

    pub(crate) fn validate_drc_marker_states(
        marker_states: &BTreeMap<String, MarkerState>,
    ) -> Result<(), String> {
        let mut workspace = WorkspaceDataset::blank();
        workspace.metadata = WorkspaceSnapshotMetadata::current(workspace.document.schema_version);
        workspace.document.marker_states = marker_states.clone();
        let validation = workspace.validate();
        if validation.is_valid() {
            Ok(())
        } else {
            Err(validation.error_summary())
        }
    }

    pub(crate) fn validate_reference_image_overlays(
        reference_images: &[ReferenceImageOverlay],
    ) -> Result<(), String> {
        let mut workspace = WorkspaceDataset::blank();
        workspace.metadata = WorkspaceSnapshotMetadata::current(workspace.document.schema_version);
        workspace.document.reference_images = reference_images.to_vec();
        let validation = workspace.validate();
        if validation.is_valid() {
            Ok(())
        } else {
            Err(validation.error_summary())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn record_recent_layout_file(&mut self, kind: &str, path: &Path) {
        let limit = self.app_options.files.recent_workspace_limit;
        if limit == 0 {
            self.app_options.files.recent_files.clear();
            return;
        }
        let path = path.display().to_string();
        self.app_options
            .files
            .recent_files
            .retain(|file| !(file.kind == kind && file.path == path));
        self.app_options.files.recent_files.insert(
            0,
            options::RecentFileOptions {
                kind: kind.to_string(),
                path,
            },
        );
        self.app_options.files.recent_files.truncate(limit);
    }

    pub(crate) fn open_recent_layout_file(&mut self, value: &str) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        let Some(entry) = self.app_options.files.recent_files.get(index).cloned() else {
            self.status_message = format!("No recent file {index}");
            return true;
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = PathBuf::from(&entry.path);
            match entry.kind.as_str() {
                "session" => self.load_app_session_from_path(&path),
                "workspace" => self.load_workspace_session_from_path(&path),
                "layout_json" => self.load_layout_json_from_path(&path),
                "gds" => self.import_layout_gds_from_path(&path),
                "cif" => self.import_layout_cif_from_path(&path),
                "dxf" => self.import_layout_dxf_from_path(&path),
                "def" => self.import_layout_def_from_path(&path),
                "lef" => self.import_layout_lef_from_path(&path),
                "reference_images" => self.import_layout_reference_images_from_path(&path),
                "drc_report" => self.import_layout_drc_report_from_path(&path),
                "drc_report_database" => self.import_layout_drc_report_database_from_path(&path),
                "drc_deck" => self.import_layout_drc_deck_from_path(&path),
                "calibre_rve" => self.import_layout_calibre_rve_markers_from_path(&path),
                "netlist" => self.import_layout_netlist_from_path(&path),
                "spice_schematic" => self.compare_layout_spice_schematic_from_path(&path),
                "trace_state" => self.import_layout_trace_state_from_path(&path),
                "l2n_database" => self.import_layout_l2n_database_from_path(&path),
                _ => false,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Recent file loading is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn reload_recent_layout_file(&mut self) -> bool {
        if self.app_options.files.recent_files.is_empty() {
            self.status_message = "No recent file to reload".to_string();
            return true;
        }
        self.open_recent_layout_file("0")
    }

    pub(crate) fn export_layout_reference_images(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_reference_image_exchange_path();
            return self.export_layout_reference_images_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Reference image export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_reference_images_to_path(&mut self, path: &Path) -> bool {
        let exchange = LayoutReferenceImageExchange::from_app(self);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Reference image export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Reference image export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("reference_images", path);
        self.status_message = format!(
            "Exported {} reference image(s) to {}",
            exchange.reference_images.len(),
            path.display()
        );
        true
    }

    pub(crate) fn import_layout_reference_images(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_reference_image_exchange_path();
            return self.import_layout_reference_images_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Reference image import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_reference_images_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Reference image import failed: {error}");
                return true;
            }
        };
        let exchange = match parse_layout_reference_image_exchange(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Reference image import failed: {error}");
                return true;
            }
        };
        let mut reference_images = match exchange.into_reference_images() {
            Ok(reference_images) => reference_images,
            Err(error) => {
                self.status_message = format!("Reference image import failed: {error}");
                return true;
            }
        };

        let mut aligned = 0usize;
        let mut alignment_warnings = 0usize;
        for image in &mut reference_images {
            match image.apply_landmark_alignment() {
                Ok(true) => aligned += 1,
                Ok(false) => {}
                Err(_) => alignment_warnings += 1,
            }
        }
        self.workspace.document.reference_images = reference_images;
        self.show_reference_images = true;
        self.mark_layout_dirty();
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("reference_images", path);
        self.status_message = format!(
            "Imported {} reference image(s) from {} ({} aligned, {} warning(s))",
            self.workspace.document.reference_images.len(),
            path.display(),
            aligned,
            alignment_warnings
        );
        true
    }

    pub(crate) fn save_app_session(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_app_session_path();
            return self.save_app_session_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Session save is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn save_app_session_to_path(&mut self, path: &Path) -> bool {
        self.sync_app_options_from_state();
        let exchange = AppSessionExchange::from_app(self);
        let validation = exchange.workspace.validate();
        if !validation.is_valid() {
            self.status_message = format!("Session save failed: {}", validation.error_summary());
            return true;
        }
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Session save failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Session save failed: {error}");
            return true;
        }
        self.record_recent_layout_file("session", path);
        self.status_message = format!(
            "Saved session {} ({} shape(s), {} cell(s), {} DRC report(s), view {})",
            path.display(),
            exchange.workspace.document.flattened_shape_count_estimate(),
            exchange.workspace.document.cells.len(),
            exchange.drc_reports.len(),
            exchange.app_options.shell.startup_view
        );
        true
    }

    pub(crate) fn load_app_session(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_app_session_path();
            return self.load_app_session_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Session load is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load_app_session_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Session load failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<AppSessionExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Session load failed: {error}");
                return true;
            }
        };
        let parts = match exchange.into_parts() {
            Ok(parts) => parts,
            Err(error) => {
                self.status_message = format!("Session load failed: {error}");
                return true;
            }
        };
        let shape_count = parts.workspace.document.flattened_shape_count_estimate();
        let cell_count = parts.workspace.document.cells.len();
        let report_count = parts.drc_reports.len().min(MAX_LAYOUT_DRC_REPORT_HISTORY);
        self.replace_workspace(parts.workspace, "Session workspace loaded");
        self.apply_app_options(parts.app_options);
        self.apply_layout_view_state(parts.current_layout_view);
        self.layout_custom_drc_deck = parts.drc_custom_deck;
        self.layout_custom_drc_deck_label = parts.drc_custom_deck_label;
        self.restore_layout_drc_report_history_from_session(
            parts.drc_reports,
            parts.drc_active_report_index,
        );
        self.record_recent_layout_file("session", path);
        self.status_message = format!(
            "Loaded session {} ({} shape(s), {} cell(s), {} DRC report(s), view {})",
            path.display(),
            shape_count,
            cell_count,
            report_count,
            self.active_view.slug()
        );
        true
    }

    pub(crate) fn save_workspace_session(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_workspace_session_path();
            return self.save_workspace_session_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Workspace save is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn save_workspace_session_to_path(&mut self, path: &Path) -> bool {
        let mut workspace = self.workspace.clone();
        workspace.schema_version = WORKSPACE_DATASET_SCHEMA_VERSION;
        workspace.metadata = WorkspaceSnapshotMetadata::current(workspace.document.schema_version);
        let validation = workspace.validate();
        if !validation.is_valid() {
            self.status_message = format!("Workspace save failed: {}", validation.error_summary());
            return true;
        }
        let bytes = match serde_json::to_vec_pretty(&workspace) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Workspace save failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Workspace save failed: {error}");
            return true;
        }
        self.record_recent_layout_file("workspace", path);
        self.status_message = format!(
            "Saved workspace {} ({} shape(s), {} cell(s))",
            path.display(),
            workspace.document.flattened_shape_count_estimate(),
            workspace.document.cells.len()
        );
        true
    }

    pub(crate) fn load_workspace_session(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_workspace_session_path();
            return self.load_workspace_session_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Workspace load is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load_workspace_session_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Workspace load failed: {error}");
                return true;
            }
        };
        let workspace = match WorkspaceDataset::from_json_str(&contents) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.status_message = format!("Workspace load failed: {error}");
                return true;
            }
        };
        let shape_count = workspace.document.flattened_shape_count_estimate();
        let cell_count = workspace.document.cells.len();
        self.replace_workspace(
            workspace,
            format!(
                "Loaded workspace {} ({} shape(s), {} cell(s))",
                path.display(),
                shape_count,
                cell_count
            ),
        );
        self.record_recent_layout_file("workspace", path);
        true
    }

    pub(crate) fn validate_layout_document_snapshot(document: &Document) -> Result<(), String> {
        let mut workspace = WorkspaceDataset::blank();
        workspace.schema_version = WORKSPACE_DATASET_SCHEMA_VERSION;
        workspace.metadata = WorkspaceSnapshotMetadata::current(document.schema_version);
        workspace.document = document.clone();
        let validation = workspace.validate();
        if validation.is_valid() {
            Ok(())
        } else {
            Err(validation.error_summary())
        }
    }

    pub(crate) fn save_layout_json(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.save_layout_json_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Layout JSON save is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn save_layout_json_to_path(&mut self, path: &Path) -> bool {
        let document = self.workspace.document.clone();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON save failed: {error}");
            return true;
        }
        let bytes = match serde_json::to_vec_pretty(&document) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Layout JSON save failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Layout JSON save failed: {error}");
            return true;
        }
        self.record_recent_layout_file("layout_json", path);
        self.status_message = format!(
            "Saved layout JSON {} ({} shape(s), {} cell(s))",
            path.display(),
            document.flattened_shape_count_estimate(),
            document.cells.len()
        );
        true
    }

    pub(crate) fn load_layout_json(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.load_layout_json_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Layout JSON load is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load_layout_json_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layout JSON load failed: {error}");
                return true;
            }
        };
        let mut document = match serde_json::from_str::<Document>(&contents) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Layout JSON load failed: {error}");
                return true;
            }
        };
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON load failed: {error}");
            return true;
        }
        let shape_count = document.flattened_shape_count_estimate();
        let cell_count = document.cells.len();
        self.workspace.document = document;
        self.reset_layout_document_state();
        self.active_view = StartupView::Layout2d;
        self.status_message = format!(
            "Loaded layout JSON {} ({} shape(s), {} cell(s))",
            path.display(),
            shape_count,
            cell_count
        );
        self.record_recent_layout_file("layout_json", path);
        true
    }

    pub(crate) fn adjust_layout_import_offset(&mut self, dx: Coord, dy: Coord) {
        self.layout_import_offset = Vector::new(
            self.layout_import_offset.dx.saturating_add(dx),
            self.layout_import_offset.dy.saturating_add(dy),
        );
        self.status_message = format!("Import offset {}", self.layout_import_offset_label());
    }

    pub(crate) fn adjust_layout_import_layer_id_offset(&mut self, delta: i32) {
        self.layout_import_layer_id_offset = self
            .layout_import_layer_id_offset
            .saturating_add(delta)
            .clamp(
                LAYOUT_IMPORT_LAYER_ID_OFFSET_MIN,
                LAYOUT_IMPORT_LAYER_ID_OFFSET_MAX,
            );
        self.status_message = format!(
            "Import layer ID offset {}",
            self.layout_import_layer_id_offset_label()
        );
    }

    pub(crate) fn merge_layout_json(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.merge_layout_json_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Layout JSON merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_json_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layout JSON merge failed: {error}");
                return true;
            }
        };
        let mut document = match serde_json::from_str::<Document>(&contents) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Layout JSON merge failed: {error}");
                return true;
            }
        };
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        let import = match self.prepare_layout_json_merge_import(&document) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("Layout JSON merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message = format!("Layout JSON merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("Layout JSON merge failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import.first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file("layout_json", path);
        self.status_message = format!(
            "Merged layout JSON {} into top cell ({} shape(s), {} layer(s))",
            path.display(),
            import.imported_shapes,
            import.added_layers
        );
        true
    }

    pub(crate) fn merge_layout_json_hierarchy(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.merge_layout_json_hierarchy_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Layout JSON hierarchy merge is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn merge_layout_json_hierarchy_from_path(&mut self, path: &Path) -> bool {
        let contents = match read_native_text_file_maybe_gzip(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layout JSON hierarchy merge failed: {error}");
                return true;
            }
        };
        let mut document = match serde_json::from_str::<Document>(&contents) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Layout JSON hierarchy merge failed: {error}");
                return true;
            }
        };
        document.ensure_hierarchy();
        if let Err(error) = Self::validate_layout_document_snapshot(&document) {
            self.status_message = format!("Layout JSON hierarchy merge failed: {error}");
            return true;
        }
        let import_name = document.name.trim().to_string().if_empty_then(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("layout")
                .to_string()
        });
        self.apply_layout_hierarchy_merge_document(
            &document,
            &import_name,
            path,
            "layout JSON",
            "layout_json",
            String::new(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn apply_layout_hierarchy_merge_document(
        &mut self,
        document: &Document,
        import_name: &str,
        path: &Path,
        format_label: &str,
        recent_kind: &str,
        status_extra: String,
    ) -> bool {
        let import = match self.prepare_layout_json_hierarchy_merge_import(document, import_name) {
            Ok(import) => import,
            Err(error) => {
                self.status_message = format!("{format_label} hierarchy merge failed: {error}");
                return true;
            }
        };
        if import.imported_shapes == 0 {
            self.status_message =
                format!("{format_label} hierarchy merge found no shapes in {import_name}");
            return true;
        }
        let mut validation_document = self.workspace.document.clone();
        validation_document.apply_operation_without_log(&Operation::Batch {
            operations: import.redo_operations.clone(),
        });
        if let Err(error) = Self::validate_layout_document_snapshot(&validation_document) {
            self.status_message = format!("{format_label} hierarchy merge failed: {error}");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: import.redo_operations,
            },
            Operation::Batch {
                operations: import.undo_operations,
            },
        );
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.selected_layout_shape = import.first_shape;
        self.selected_layout_occurrence = import.first_shape.map(ShapeOccurrenceId::top_level);
        self.active_view = StartupView::Layout2d;
        self.record_recent_layout_file(recent_kind, path);
        self.status_message = format!(
            "Merged {format_label} {} hierarchy into top cell ({} shape(s), {} cell(s), {} layer(s){})",
            path.display(),
            import.imported_shapes,
            import.added_cells,
            import.added_layers,
            status_extra
        );
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_layout_json_merge_import(
        &mut self,
        document: &Document,
    ) -> Result<LayoutJsonMergeImportPlan, String> {
        let (layer_map, added_layers) = self.prepare_layout_json_import_layers(document)?;
        let mut imported_shapes = Vec::new();
        document.for_each_flattened_shape_view(|_, view| {
            imported_shapes.push(view.transformed_shape());
        });
        let mut mapped_shapes = Vec::with_capacity(imported_shapes.len());
        for shape in imported_shapes {
            let mut mapped_shape = self.remap_layout_json_import_shape(shape, &layer_map)?;
            if self.layout_import_offset != Vector::ZERO {
                mapped_shape.kind.translate(self.layout_import_offset);
            }
            mapped_shapes.push(mapped_shape);
        }
        mapped_shapes.sort_by_key(|shape| shape.id);
        let first_shape = mapped_shapes.first().map(|shape| shape.id);

        let mut redo_operations = Vec::new();
        for layer in &added_layers {
            redo_operations.push(Operation::AddLayer {
                layer: layer.clone(),
            });
        }
        for shape in &mapped_shapes {
            redo_operations.push(Operation::AddShape {
                shape: shape.clone(),
            });
        }
        let mut undo_operations = Vec::new();
        for shape in mapped_shapes.iter().rev() {
            undo_operations.push(Operation::DeleteShape { id: shape.id });
        }
        for layer in added_layers.iter().rev() {
            undo_operations.push(Operation::DeleteLayer { id: layer.id });
        }

        Ok(LayoutJsonMergeImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            imported_shapes: mapped_shapes.len(),
            added_layers: added_layers.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_layout_json_hierarchy_merge_import(
        &mut self,
        document: &Document,
        import_name: &str,
    ) -> Result<LayoutJsonHierarchyMergeImportPlan, String> {
        let (layer_map, added_layers) = self.prepare_layout_json_import_layers(document)?;
        let source_cells = document.cells.values().cloned().collect::<Vec<_>>();
        if source_cells.is_empty() {
            return Err("imported layout has no cells".to_string());
        }
        let source_top = document.top_cell;
        let target_top = self.workspace.document.top_cell;
        let mut cell_map = BTreeMap::new();
        for cell in &source_cells {
            if cell.id != source_top {
                cell_map.insert(cell.id, self.workspace.document.allocate_cell_id());
            }
        }

        let mut imported_cells = Vec::new();
        for cell in &source_cells {
            if cell.id == source_top {
                continue;
            }
            let mapped_cell = *cell_map
                .get(&cell.id)
                .ok_or_else(|| format!("missing cell mapping for C{}", cell.id.0))?;
            let mut imported_cell = Cell::new(
                mapped_cell,
                format!(
                    "merge {} {} {}",
                    compact_button_label(import_name, 12),
                    compact_button_label(&cell.name, 18),
                    mapped_cell.0
                ),
            );
            imported_cell.properties = cell.properties.clone();
            imported_cell
                .properties
                .insert("import.source_document".to_string(), document.name.clone());
            imported_cell
                .properties
                .insert("import.source_cell".to_string(), cell.name.clone());

            let mut source_shapes = cell.shapes.values().collect::<Vec<_>>();
            source_shapes.sort_by_key(|shape| shape.id);
            for shape in source_shapes {
                let imported_shape = self.remap_layout_json_import_shape(shape, &layer_map)?;
                imported_cell
                    .shapes
                    .insert(imported_shape.id, imported_shape);
            }

            let mut instances = cell.instances.values().collect::<Vec<_>>();
            instances.sort_by_key(|instance| instance.id);
            for instance in instances {
                let target = *cell_map.get(&instance.cell).ok_or_else(|| {
                    if instance.cell == source_top {
                        format!(
                            "imported cell C{} references the source top cell C{}, which cannot be hierarchy-merged",
                            cell.id.0, source_top.0
                        )
                    } else {
                        format!(
                            "imported instance I{} references missing cell C{}",
                            instance.id.0, instance.cell.0
                        )
                    }
                })?;
                let mut imported_instance = instance;
                imported_instance.id = self.workspace.document.allocate_instance_id();
                imported_instance.cell = target;
                imported_cell
                    .instances
                    .insert(imported_instance.id, imported_instance);
            }
            imported_cells.push(imported_cell);
        }

        let source_top_cell = document
            .cell(source_top)
            .ok_or_else(|| "imported layout has no top cell".to_string())?;
        let mut root_shapes = source_top_cell.shapes.values().collect::<Vec<_>>();
        root_shapes.extend(document.shapes.values());
        root_shapes.sort_by_key(|shape| shape.id);
        let mut mapped_root_shapes = Vec::with_capacity(root_shapes.len());
        for shape in root_shapes {
            let mut mapped_shape = self.remap_layout_json_import_shape(shape, &layer_map)?;
            if self.layout_import_offset != Vector::ZERO {
                mapped_shape.kind.translate(self.layout_import_offset);
            }
            mapped_root_shapes.push(mapped_shape);
        }
        mapped_root_shapes.sort_by_key(|shape| shape.id);
        let first_shape = mapped_root_shapes.first().map(|shape| shape.id);

        let root_shift = Transform::from_translation(self.layout_import_offset);
        let mut root_instances = source_top_cell.instances.values().collect::<Vec<_>>();
        root_instances.sort_by_key(|instance| instance.id);
        let mut mapped_root_instances = Vec::with_capacity(root_instances.len());
        for instance in root_instances {
            let target = *cell_map.get(&instance.cell).ok_or_else(|| {
                if instance.cell == source_top {
                    format!(
                        "imported top-cell instance I{} references the source top cell C{}, which cannot be hierarchy-merged",
                        instance.id.0, source_top.0
                    )
                } else {
                    format!(
                        "imported top-cell instance I{} references missing cell C{}",
                        instance.id.0, instance.cell.0
                    )
                }
            })?;
            let mut imported_instance = instance;
            imported_instance.id = self.workspace.document.allocate_instance_id();
            imported_instance.cell = target;
            imported_instance.transform = root_shift.compose(imported_instance.transform);
            mapped_root_instances.push(imported_instance);
        }
        mapped_root_instances.sort_by_key(|instance| instance.id);

        let mut redo_operations = Vec::new();
        for layer in &added_layers {
            redo_operations.push(Operation::AddLayer {
                layer: layer.clone(),
            });
        }
        for cell in &imported_cells {
            redo_operations.push(Operation::AddCell { cell: cell.clone() });
        }
        for shape in &mapped_root_shapes {
            redo_operations.push(Operation::AddShape {
                shape: shape.clone(),
            });
        }
        for instance in &mapped_root_instances {
            redo_operations.push(Operation::AddInstance {
                parent: target_top,
                instance: instance.clone(),
            });
        }

        let mut undo_operations = Vec::new();
        for instance in mapped_root_instances.iter().rev() {
            undo_operations.push(Operation::DeleteInstance {
                parent: target_top,
                id: instance.id,
            });
        }
        for shape in mapped_root_shapes.iter().rev() {
            undo_operations.push(Operation::DeleteShape { id: shape.id });
        }
        for cell in imported_cells.iter().rev() {
            undo_operations.push(Operation::DeleteCell { id: cell.id });
        }
        for layer in added_layers.iter().rev() {
            undo_operations.push(Operation::DeleteLayer { id: layer.id });
        }

        Ok(LayoutJsonHierarchyMergeImportPlan {
            redo_operations,
            undo_operations,
            first_shape,
            imported_shapes: document.flattened_shape_count_estimate(),
            added_cells: imported_cells.len(),
            added_layers: added_layers.len(),
        })
    }

    pub(crate) fn import_layout_json_as_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.import_layout_json_as_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Layout JSON cell import is available in the native app".to_string();
            true
        }
    }

    pub(crate) fn import_layout_json_as_top_cell(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layout_json_exchange_path();
            return self.import_layout_json_as_top_cell_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Layout JSON top-cell import is available in the native app".to_string();
            true
        }
    }
}

fn layout_drc_report_database_contents_looks_like_klayout_rdb(contents: &str) -> bool {
    let contents = contents.trim_start();
    contents.starts_with("<report-database")
        || contents
            .strip_prefix("<?xml")
            .is_some_and(|rest| rest.contains("<report-database"))
}

#[cfg(not(target_arch = "wasm32"))]
fn layout_drc_report_database_path_is_klayout_rdb(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|name| name.ends_with(".lyrdb") || name.ends_with(".lyrdb.gz"))
}

#[cfg(not(target_arch = "wasm32"))]
fn atomic_write_klayout_rdb_report_database_bytes(
    path: &Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
    {
        use std::io::Write as _;

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes)?;
        let bytes = encoder.finish()?;
        atomic_write_bytes(path, &bytes)
    } else {
        atomic_write_bytes(path, bytes)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn layout_klayout_rdb_report_label(path: &Path, description: Option<&str>) -> String {
    let label = description
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "KLayout RDB".to_string());
    label.chars().take(48).collect()
}

#[derive(Default)]
struct KlayoutRdbCategoryTree {
    description: Option<String>,
    children: BTreeMap<String, KlayoutRdbCategoryTree>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct KlayoutRdbCellMetadata {
    name: Option<String>,
    variant: Option<String>,
    layout_name: Option<String>,
    references: Vec<KlayoutRdbCellReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KlayoutRdbCellKey {
    name: String,
    variant: Option<String>,
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

    fn literal(name: &str) -> Self {
        Self::new(name, None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbCellReference {
    parent: String,
    trans: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbItemValue {
    kind: String,
    value: String,
    empty: bool,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbGeometryValue {
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbItemReferenceValue {
    value: String,
    tag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KlayoutRdbRawValue {
    value: String,
    tag: Option<String>,
}

impl KlayoutRdbCategoryTree {
    fn insert(&mut self, parts: &[String], descriptions: &[Option<String>]) {
        if let Some((first, rest)) = parts.split_first() {
            let child = self.children.entry(first.clone()).or_default();
            if child.description.is_none()
                && let Some(description) = descriptions
                    .first()
                    .and_then(Option::as_deref)
                    .map(str::trim)
                    .filter(|description| !description.is_empty())
            {
                child.description = Some(description.to_string());
            }
            child.insert(rest, descriptions.get(1..).unwrap_or(&[]));
        }
    }
}

fn layout_klayout_rdb_report_database_xml(
    document: &Document,
    reports: &[LayoutDrcReportDatabaseEntryExchange],
    marker_states: &BTreeMap<String, MarkerState>,
) -> Vec<u8> {
    let mut top_cell = layout_klayout_rdb_top_cell_name(document);
    let mut category_tree = KlayoutRdbCategoryTree::default();
    let mut cells = BTreeMap::<KlayoutRdbCellKey, KlayoutRdbCellMetadata>::new();
    let mut declared_tags = BTreeMap::<String, String>::new();
    let mut report_description = None::<String>;
    let mut report_original_file = None::<String>;
    let mut report_generator = None::<String>;
    let mut report_top_cell = None::<String>;
    let mut marker_items = Vec::new();
    let mut needs_all_cells_declaration = false;
    let include_report_prefix = reports.len() > 1;

    for report in reports {
        for violation in &report.violations {
            let key = violation.stable_key();
            let state = marker_states.get(&key);
            let category = layout_klayout_rdb_violation_category(
                report,
                violation,
                state,
                include_report_prefix,
            );
            layout_klayout_rdb_record_declared_categories(&mut category_tree, state);
            let category_descriptions =
                layout_klayout_rdb_category_descriptions(state, category.len());
            category_tree.insert(&category, &category_descriptions);
            layout_klayout_rdb_record_report_metadata(
                &mut report_description,
                &mut report_top_cell,
                &mut report_original_file,
                &mut report_generator,
                state,
            );
            if let Some(report_top_cell) = report_top_cell.as_deref() {
                top_cell = report_top_cell.to_string();
            }
            layout_klayout_rdb_record_declared_cells(&mut cells, state);
            let cell = layout_klayout_rdb_violation_cell(state, &top_cell);
            if !cell.is_empty() {
                layout_klayout_rdb_record_cell_metadata(&mut cells, &cell, state);
            }
            let tags = layout_klayout_rdb_item_tags(state);
            layout_klayout_rdb_record_declared_tags(&mut declared_tags, state, &tags);
            marker_items.push(layout_klayout_rdb_marker_item(
                violation, state, &category, &cell, &tags, report,
            ));
        }
        for finding in &report.findings {
            needs_all_cells_declaration = true;
            let category = vec![
                if include_report_prefix {
                    layout_klayout_rdb_category_part(&report.label)
                } else {
                    "Diagnostics".to_string()
                },
                layout_klayout_rdb_category_part(match finding.severity {
                    DrcValidationSeverity::Error => "Error",
                    DrcValidationSeverity::Warning => "Warning",
                }),
            ];
            category_tree.insert(&category, &[]);
            marker_items.push(layout_klayout_rdb_diagnostic_item(finding, &category));
        }
    }
    if let Some(report_top_cell) = report_top_cell {
        top_cell = report_top_cell;
    }
    if !top_cell.is_empty() {
        cells
            .entry(KlayoutRdbCellKey::literal(&top_cell))
            .or_insert_with(|| KlayoutRdbCellMetadata {
                name: Some(top_cell.clone()),
                ..KlayoutRdbCellMetadata::default()
            });
    }
    if needs_all_cells_declaration {
        cells
            .entry(KlayoutRdbCellKey::literal(""))
            .or_insert_with(|| KlayoutRdbCellMetadata {
                name: Some(String::new()),
                ..KlayoutRdbCellMetadata::default()
            });
    }

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<report-database>\n");
    let fallback_description = format!(
        "Glassworks DRC report database ({} report(s))",
        reports.len()
    );
    layout_klayout_rdb_write_root_text_element(
        &mut xml,
        "description",
        report_description
            .as_deref()
            .unwrap_or(&fallback_description),
    );
    layout_klayout_rdb_write_root_text_element(
        &mut xml,
        "original-file",
        report_original_file.as_deref().unwrap_or_default(),
    );
    layout_klayout_rdb_write_root_text_element(
        &mut xml,
        "generator",
        report_generator
            .as_deref()
            .unwrap_or("Glassworks KLayout RDB subset exporter"),
    );
    layout_klayout_rdb_write_root_text_element(&mut xml, "top-cell", &top_cell);
    xml.push_str(" <tags>\n");
    for (tag, description) in declared_tags {
        xml.push_str("  <tag>\n");
        xml.push_str("   <name>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&tag));
        xml.push_str("</name>\n");
        xml.push_str("   <description>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&description));
        xml.push_str("</description>\n");
        xml.push_str("  </tag>\n");
    }
    xml.push_str(" </tags>\n");
    xml.push_str(" <categories>\n");
    layout_klayout_rdb_write_category_tree(&mut xml, &category_tree, 2);
    xml.push_str(" </categories>\n");
    xml.push_str(" <cells>\n");
    for (cell, metadata) in cells {
        let name = metadata.name.as_deref().unwrap_or(cell.name.as_str());
        let variant = metadata.variant.as_deref().or(cell.variant.as_deref());
        xml.push_str("  <cell>\n");
        xml.push_str("   <name>");
        xml.push_str(&layout_klayout_rdb_xml_escape(name));
        xml.push_str("</name>\n");
        if let Some(variant) = variant {
            xml.push_str("   <variant>");
            xml.push_str(&layout_klayout_rdb_xml_escape(variant));
            xml.push_str("</variant>\n");
        } else {
            xml.push_str("   <variant/>\n");
        }
        if let Some(layout_name) = metadata
            .layout_name
            .as_deref()
            .map(str::trim)
            .filter(|layout_name| !layout_name.is_empty())
        {
            xml.push_str("   <layout-name>");
            xml.push_str(&layout_klayout_rdb_xml_escape(layout_name));
            xml.push_str("</layout-name>\n");
        } else {
            xml.push_str("   <layout-name/>\n");
        }
        layout_klayout_rdb_write_cell_references(&mut xml, &metadata.references);
        xml.push_str("  </cell>\n");
    }
    xml.push_str(" </cells>\n");
    xml.push_str(" <items>\n");
    for item in marker_items {
        xml.push_str(&item);
    }
    xml.push_str(" </items>\n");
    xml.push_str("</report-database>\n");
    xml.into_bytes()
}

fn layout_klayout_rdb_write_root_text_element(xml: &mut String, tag: &str, value: &str) {
    xml.push(' ');
    if value.is_empty() {
        xml.push('<');
        xml.push_str(tag);
        xml.push_str("/>\n");
    } else {
        xml.push('<');
        xml.push_str(tag);
        xml.push('>');
        xml.push_str(&layout_klayout_rdb_xml_escape(value));
        xml.push_str("</");
        xml.push_str(tag);
        xml.push_str(">\n");
    }
}

fn layout_klayout_rdb_marker_item(
    violation: &DrcViolation,
    state: Option<&MarkerState>,
    category: &[String],
    cell: &str,
    tags: &[String],
    report: &LayoutDrcReportDatabaseEntryExchange,
) -> String {
    let mut xml = String::new();
    xml.push_str("  <item>\n");
    let tag_list = layout_klayout_rdb_tag_list(tags);
    if tag_list.is_empty() {
        xml.push_str("   <tags/>\n");
    } else {
        xml.push_str("   <tags>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&tag_list));
        xml.push_str("</tags>\n");
    }
    xml.push_str("   <category>");
    xml.push_str(&layout_klayout_rdb_xml_escape(
        &layout_klayout_rdb_category_path(category),
    ));
    xml.push_str("</category>\n");
    xml.push_str("   <cell>");
    xml.push_str(&layout_klayout_rdb_xml_escape(cell));
    xml.push_str("</cell>\n");
    xml.push_str("   <visited>");
    xml.push_str(if state.is_some_and(|state| state.visited) {
        "true"
    } else {
        "false"
    });
    xml.push_str("</visited>\n");
    xml.push_str("   <multiplicity>");
    xml.push_str(&layout_klayout_rdb_xml_escape(
        &layout_klayout_rdb_item_multiplicity(state),
    ));
    xml.push_str("</multiplicity>\n");
    if let Some(comment) = state.and_then(|state| state.note.as_deref()) {
        xml.push_str("   <comment>");
        xml.push_str(&layout_klayout_rdb_xml_escape(comment));
        xml.push_str("</comment>\n");
    } else {
        xml.push_str("   <comment/>\n");
    }
    if let Some(image) = layout_klayout_rdb_item_image_base64(state) {
        xml.push_str("   <image>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&image));
        xml.push_str("</image>\n");
    } else {
        xml.push_str("   <image/>\n");
    }
    xml.push_str("   <values>\n");
    let (wrote_ordered_values, wrote_ordered_geometry) =
        layout_klayout_rdb_write_item_ordered_values(&mut xml, state);
    if !wrote_ordered_values {
        layout_klayout_rdb_write_item_references(&mut xml, state);
        layout_klayout_rdb_write_item_scalar_values(&mut xml, state);
        layout_klayout_rdb_write_raw_values(&mut xml, state);
        layout_klayout_rdb_write_text_value(&mut xml, &violation.message);
        if let Some(source) = report
            .source
            .as_deref()
            .filter(|source| !source.trim().is_empty())
        {
            layout_klayout_rdb_write_text_value(&mut xml, &format!("source: {source}"));
        }
        if let Some(owner) = state
            .and_then(|state| state.owner.as_deref())
            .filter(|owner| !owner.trim().is_empty())
        {
            layout_klayout_rdb_write_text_value(&mut xml, &format!("owner: {owner}"));
        }
        if let Some(signoff) = state
            .and_then(|state| state.signoff.as_deref())
            .filter(|signoff| !signoff.trim().is_empty())
        {
            layout_klayout_rdb_write_text_value(&mut xml, &format!("signoff: {signoff}"));
        }
    }
    if !wrote_ordered_geometry && !layout_klayout_rdb_write_item_geometry_values(&mut xml, state) {
        xml.push_str("    <value>");
        xml.push_str(&layout_klayout_rdb_xml_escape(
            &layout_klayout_rdb_box_value(violation.bounds),
        ));
        xml.push_str("</value>\n");
    }
    xml.push_str("   </values>\n");
    xml.push_str("  </item>\n");
    xml
}

fn layout_klayout_rdb_diagnostic_item(
    finding: &DrcValidationFinding,
    category: &[String],
) -> String {
    let mut xml = String::new();
    xml.push_str("  <item>\n");
    xml.push_str("   <tags/>\n");
    xml.push_str("   <category>");
    xml.push_str(&layout_klayout_rdb_xml_escape(
        &layout_klayout_rdb_category_path(category),
    ));
    xml.push_str("</category>\n");
    xml.push_str("   <cell/>\n");
    xml.push_str("   <visited>false</visited>\n");
    xml.push_str("   <multiplicity>1</multiplicity>\n");
    xml.push_str("   <comment/>\n");
    xml.push_str("   <image/>\n");
    xml.push_str("   <values>\n");
    layout_klayout_rdb_write_text_value(&mut xml, &finding.message);
    xml.push_str("   </values>\n");
    xml.push_str("  </item>\n");
    xml
}

fn layout_klayout_rdb_write_text_value(xml: &mut String, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    xml.push_str("    <value>");
    xml.push_str(&layout_klayout_rdb_xml_escape(&format!(
        "text: {}",
        layout_klayout_rdb_text_payload(text)
    )));
    xml.push_str("</value>\n");
}

fn layout_klayout_rdb_write_item_references(xml: &mut String, state: Option<&MarkerState>) {
    for reference in layout_klayout_rdb_item_references(state) {
        xml.push_str("    <value>");
        let tag_prefix = reference
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(layout_klayout_rdb_tagged_value_prefix)
            .unwrap_or_default();
        let text = layout_klayout_rdb_reference_text_value(&reference.value);
        xml.push_str(&layout_klayout_rdb_xml_escape(&format!(
            "{tag_prefix}text: {}",
            layout_klayout_rdb_text_payload(&text)
        )));
        xml.push_str("</value>\n");
    }
}

fn layout_klayout_rdb_write_item_scalar_values(xml: &mut String, state: Option<&MarkerState>) {
    for value in layout_klayout_rdb_item_scalar_values(state) {
        xml.push_str("    <value>");
        let tag_prefix = value
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(layout_klayout_rdb_tagged_value_prefix)
            .unwrap_or_default();
        let export_kind = layout_klayout_rdb_scalar_value_export_kind(&value.kind);
        let payload = match export_kind {
            "text" => format!(
                "{tag_prefix}{}: {}",
                export_kind,
                layout_klayout_rdb_text_payload(&value.value)
            ),
            "float" if value.value.trim().parse::<f64>().is_ok() => {
                format!("{tag_prefix}{}: {}", export_kind, value.value)
            }
            _ => format!(
                "{tag_prefix}text: {}",
                layout_klayout_rdb_text_payload(&layout_klayout_rdb_raw_text_value(
                    &layout_klayout_rdb_item_scalar_raw_value(&value),
                ))
            ),
        };
        xml.push_str(&layout_klayout_rdb_xml_escape(&payload));
        xml.push_str("</value>\n");
    }
}

fn layout_klayout_rdb_scalar_value_export_kind(kind: &str) -> &str {
    match kind.trim() {
        "string" => "text",
        other => other,
    }
}

fn layout_klayout_rdb_item_scalar_raw_value(value: &KlayoutRdbItemValue) -> String {
    let kind = value.kind.trim();
    if kind.is_empty() {
        value.value.trim().to_string()
    } else {
        format!("{kind}: {}", value.value.trim())
    }
}

fn layout_klayout_rdb_tagged_value_prefix(tag: &str) -> String {
    let tag = tag.trim();
    let tag = tag.strip_prefix('#').unwrap_or(tag).trim();
    if tag.is_empty() {
        String::new()
    } else if layout_klayout_rdb_tag_list_word(tag) {
        format!("[#{tag}] ")
    } else {
        format!("[#{}] ", layout_klayout_rdb_quoted_text(tag))
    }
}

fn layout_klayout_rdb_write_raw_values(xml: &mut String, state: Option<&MarkerState>) {
    for value in layout_klayout_rdb_raw_values(state) {
        xml.push_str("    <value>");
        let tag_prefix = value
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(layout_klayout_rdb_tagged_value_prefix)
            .unwrap_or_default();
        let raw_text = layout_klayout_rdb_raw_text_value(&value.value);
        xml.push_str(&layout_klayout_rdb_xml_escape(&format!(
            "{tag_prefix}text: {}",
            layout_klayout_rdb_text_payload(&raw_text)
        )));
        xml.push_str("</value>\n");
    }
}

fn layout_klayout_rdb_write_item_ordered_values(
    xml: &mut String,
    state: Option<&MarkerState>,
) -> (bool, bool) {
    let mut wrote_value = false;
    let mut wrote_geometry = false;
    for value in layout_klayout_rdb_item_ordered_values(state) {
        let value = layout_klayout_rdb_export_ordered_value(&value);
        xml.push_str("    <value>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&value));
        xml.push_str("</value>\n");
        wrote_value = true;
        wrote_geometry |= layout_klayout_rdb_geometry_value_is_supported(&value);
    }
    (wrote_value, wrote_geometry)
}

fn layout_klayout_rdb_export_ordered_value(value: &str) -> String {
    let typed_value = layout_klayout_rdb_value_without_tag(value);
    let Some((kind, body)) = typed_value.split_once(':') else {
        return value.to_string();
    };
    let kind_key = layout_klayout_rdb_value_kind_key(kind);
    let tag_prefix = || {
        layout_klayout_rdb_value_tag(value)
            .map(|tag| layout_klayout_rdb_tagged_value_prefix(&tag))
            .unwrap_or_default()
    };
    if kind_key == "string" {
        return format!("{}text:{body}", tag_prefix());
    }
    if kind_key == "text" && layout_klayout_rdb_text_geometry_value_is_supported(body) {
        return format!("{}label:{body}", tag_prefix());
    }
    if matches!(kind_key.as_str(), "rectangle" | "rect") {
        return format!("{}box:{body}", tag_prefix());
    }
    if matches!(kind_key.as_str(), "edge_pair" | "edgepair") {
        return format!("{}edge-pair:{body}", tag_prefix());
    }
    if kind_key == "poly" {
        return format!("{}polygon:{body}", tag_prefix());
    }
    if kind_key == "point"
        && let Some(body) = layout_klayout_rdb_point_export_box_body(body)
    {
        return format!("{}box: {body}", tag_prefix());
    }
    if kind_key == "reference" {
        let reference = body.trim();
        if !reference.is_empty() {
            return format!(
                "{}text: {}",
                tag_prefix(),
                layout_klayout_rdb_text_payload(&layout_klayout_rdb_reference_text_value(
                    reference
                ))
            );
        }
    }
    if !layout_klayout_rdb_ordered_value_kind_is_klayout_supported(&kind_key) {
        return format!(
            "{}text: {}",
            tag_prefix(),
            layout_klayout_rdb_text_payload(&layout_klayout_rdb_raw_text_value(typed_value))
        );
    }
    value.to_string()
}

fn layout_klayout_rdb_export_geometry_value(value: &str) -> String {
    let value = layout_klayout_rdb_value_without_tag(value).trim();
    let Some((kind, body)) = value.split_once(':') else {
        return value.to_string();
    };
    if layout_klayout_rdb_value_kind_key(kind) == "text"
        && layout_klayout_rdb_text_geometry_value_is_supported(body)
    {
        format!("label:{body}")
    } else if matches!(
        layout_klayout_rdb_value_kind_key(kind).as_str(),
        "rectangle" | "rect"
    ) {
        format!("box:{body}")
    } else if matches!(
        layout_klayout_rdb_value_kind_key(kind).as_str(),
        "edge_pair" | "edgepair"
    ) {
        format!("edge-pair:{body}")
    } else if layout_klayout_rdb_value_kind_key(kind) == "poly" {
        format!("polygon:{body}")
    } else if layout_klayout_rdb_value_kind_key(kind) == "point" {
        layout_klayout_rdb_point_export_box_body(body)
            .map(|body| format!("box: {body}"))
            .unwrap_or_else(|| value.to_string())
    } else {
        value.to_string()
    }
}

fn layout_klayout_rdb_write_item_geometry_values(
    xml: &mut String,
    state: Option<&MarkerState>,
) -> bool {
    let mut wrote_geometry = false;
    for value in layout_klayout_rdb_item_geometry_values(state) {
        xml.push_str("    <value>");
        let tag_prefix = value
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(layout_klayout_rdb_tagged_value_prefix)
            .unwrap_or_default();
        let value = layout_klayout_rdb_export_geometry_value(&value.value);
        xml.push_str(&layout_klayout_rdb_xml_escape(&format!(
            "{tag_prefix}{}",
            value
        )));
        xml.push_str("</value>\n");
        wrote_geometry = true;
    }
    wrote_geometry
}

fn layout_klayout_rdb_item_multiplicity(state: Option<&MarkerState>) -> String {
    state
        .and_then(|state| state.tags.get(KLAYOUT_RDB_MULTIPLICITY_TAG))
        .map(String::as_str)
        .map(str::trim)
        .filter(|multiplicity| multiplicity.parse::<u64>().is_ok())
        .unwrap_or("1")
        .to_string()
}

fn layout_klayout_rdb_record_declared_tags(
    declared_tags: &mut BTreeMap<String, String>,
    state: Option<&MarkerState>,
    tags: &[String],
) {
    if let Some(state) = state {
        for (key, _) in &state.tags {
            let Some(tag_key) = key.strip_prefix(KLAYOUT_RDB_DECLARED_TAG_PREFIX) else {
                continue;
            };
            let tag_key = tag_key.trim();
            if tag_key.is_empty() {
                continue;
            }
            let tag = layout_klayout_rdb_original_tag_name(state, tag_key)
                .unwrap_or_else(|| tag_key.to_string());
            let description =
                layout_klayout_rdb_tag_description(Some(state), &tag).unwrap_or_default();
            declared_tags.entry(tag).or_insert(description);
        }
        for (key, description) in &state.tags {
            let Some(tag_key) = key.strip_prefix(KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX) else {
                continue;
            };
            let tag_key = tag_key.trim();
            if tag_key.is_empty() {
                continue;
            }
            let description = description.trim();
            if description.is_empty() {
                continue;
            }
            let tag = layout_klayout_rdb_original_tag_name(state, tag_key)
                .unwrap_or_else(|| tag_key.to_string());
            declared_tags.entry(tag).or_insert(description.to_string());
        }
        for (key, tag) in &state.tags {
            let Some(tag_key) = key.strip_prefix(KLAYOUT_RDB_TAG_NAME_TAG_PREFIX) else {
                continue;
            };
            if tag_key.trim().is_empty() {
                continue;
            }
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }
            let description = layout_klayout_rdb_tag_description(Some(state), tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag.to_string()).or_insert(description);
        }
        for value in layout_klayout_rdb_item_scalar_values(Some(state)) {
            let Some(tag) = value
                .tag
                .as_deref()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
            else {
                continue;
            };
            let description = layout_klayout_rdb_tag_description(Some(state), tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag.to_string()).or_insert(description);
        }
        for reference in layout_klayout_rdb_item_references(Some(state)) {
            let Some(tag) = reference
                .tag
                .as_deref()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
            else {
                continue;
            };
            let description = layout_klayout_rdb_tag_description(Some(state), tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag.to_string()).or_insert(description);
        }
        for geometry in layout_klayout_rdb_item_geometry_values(Some(state)) {
            let Some(tag) = geometry
                .tag
                .as_deref()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
            else {
                continue;
            };
            let description = layout_klayout_rdb_tag_description(Some(state), tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag.to_string()).or_insert(description);
        }
        for raw in layout_klayout_rdb_raw_values(Some(state)) {
            let Some(tag) = raw
                .tag
                .as_deref()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
            else {
                continue;
            };
            let description = layout_klayout_rdb_tag_description(Some(state), tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag.to_string()).or_insert(description);
        }
        for value in layout_klayout_rdb_item_ordered_values(Some(state)) {
            let Some(tag) = layout_klayout_rdb_value_tag(&value) else {
                continue;
            };
            let description = layout_klayout_rdb_tag_description(Some(state), &tag)
                .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
            declared_tags.entry(tag).or_insert(description);
        }
    }
    for tag in tags {
        let description = layout_klayout_rdb_tag_description(state, tag)
            .unwrap_or_else(|| format!("Glassworks marker tag {tag}"));
        declared_tags.entry(tag.clone()).or_insert(description);
    }
}

fn layout_klayout_rdb_tag_description(state: Option<&MarkerState>, tag: &str) -> Option<String> {
    let state = state?;
    state
        .tags
        .get(&format!("{KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX}{tag}"))
        .or_else(|| {
            state.tags.get(&format!(
                "{KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX}{}",
                layout_klayout_rdb_token(tag)
            ))
        })
        .map(String::as_str)
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(str::to_string)
}

fn layout_klayout_rdb_record_report_metadata(
    description: &mut Option<String>,
    top_cell: &mut Option<String>,
    original_file: &mut Option<String>,
    generator: &mut Option<String>,
    state: Option<&MarkerState>,
) {
    if description.is_none() {
        *description = layout_klayout_rdb_report_metadata_value(
            state,
            KLAYOUT_RDB_REPORT_DESCRIPTION_TAG,
            KLAYOUT_RDB_REPORT_DESCRIPTION_EMPTY_TAG,
        );
    }
    if top_cell.is_none() {
        *top_cell = layout_klayout_rdb_report_metadata_value(
            state,
            KLAYOUT_RDB_REPORT_TOP_CELL_TAG,
            KLAYOUT_RDB_REPORT_TOP_CELL_EMPTY_TAG,
        );
    }
    if original_file.is_none() {
        *original_file = layout_klayout_rdb_report_metadata_value(
            state,
            KLAYOUT_RDB_REPORT_ORIGINAL_FILE_TAG,
            KLAYOUT_RDB_REPORT_ORIGINAL_FILE_EMPTY_TAG,
        );
    }
    if generator.is_none() {
        *generator = layout_klayout_rdb_report_metadata_value(
            state,
            KLAYOUT_RDB_REPORT_GENERATOR_TAG,
            KLAYOUT_RDB_REPORT_GENERATOR_EMPTY_TAG,
        );
    }
}

fn layout_klayout_rdb_report_metadata_value(
    state: Option<&MarkerState>,
    value_tag: &str,
    empty_tag: &str,
) -> Option<String> {
    if layout_klayout_rdb_state_tag_is_truthy(state, empty_tag) {
        Some(String::new())
    } else {
        layout_klayout_rdb_state_tag_value(state, value_tag)
    }
}

fn layout_klayout_rdb_state_tag_is_truthy(state: Option<&MarkerState>, key: &str) -> bool {
    state
        .and_then(|state| state.tags.get(key))
        .is_some_and(|value| layout_klayout_rdb_truthy_value(value))
}

fn layout_klayout_rdb_state_tag_value(state: Option<&MarkerState>, key: &str) -> Option<String> {
    state
        .and_then(|state| state.tags.get(key))
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn layout_klayout_rdb_item_references(
    state: Option<&MarkerState>,
) -> Vec<KlayoutRdbItemReferenceValue> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut references = BTreeMap::<usize, KlayoutRdbItemReferenceValue>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_item_reference_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let reference = references
            .entry(index)
            .or_insert_with(|| KlayoutRdbItemReferenceValue {
                value: String::new(),
                tag: None,
            });
        match field {
            None | Some("value") => reference.value = value.to_string(),
            Some("tag") => reference.tag = Some(value.to_string()),
            _ => {}
        }
    }
    references
        .into_iter()
        .map(|(_, reference)| reference)
        .filter(|reference| !reference.value.is_empty())
        .collect()
}

fn layout_klayout_rdb_item_reference_field(key: &str) -> Option<(usize, Option<&str>)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_ITEM_REFERENCE_TAG_PREFIX)?;
    if let Ok(index) = rest.parse::<usize>() {
        return (index > 0).then_some((index, None));
    }
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, Some(field)))
}

fn layout_klayout_rdb_item_scalar_values(state: Option<&MarkerState>) -> Vec<KlayoutRdbItemValue> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut values = BTreeMap::<usize, KlayoutRdbItemValue>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_item_value_field(key) else {
            continue;
        };
        let raw_value = value.as_str();
        let value = value.trim();
        if value.is_empty() && field != "value" {
            continue;
        }
        let item = values.entry(index).or_insert_with(|| KlayoutRdbItemValue {
            kind: String::new(),
            value: String::new(),
            empty: false,
            tag: None,
        });
        match field {
            "type" => item.kind = value.to_ascii_lowercase(),
            "value" => {
                if !raw_value.is_empty() {
                    item.value = raw_value.to_string();
                    item.empty = false;
                }
            }
            "value_hex" => {
                if let Some(decoded) = layout_klayout_rdb_text_value_from_hex(value) {
                    item.value = decoded;
                    item.empty = false;
                }
            }
            "empty" => {
                if layout_klayout_rdb_truthy_value(value) {
                    item.value.clear();
                    item.empty = true;
                }
            }
            "tag" => item.tag = Some(value.to_string()),
            _ => {}
        }
    }
    values
        .into_values()
        .filter(|value| {
            !value.kind.is_empty()
                && (!value.value.is_empty()
                    || (value.empty && layout_klayout_rdb_scalar_value_allows_empty(&value.kind)))
        })
        .collect()
}

fn layout_klayout_rdb_item_value_field(key: &str) -> Option<(usize, &str)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX)?;
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, field))
}

fn layout_klayout_rdb_scalar_value_allows_empty(kind: &str) -> bool {
    matches!(kind.trim(), "text" | "string")
}

fn layout_klayout_rdb_truthy_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y"
    )
}

fn layout_klayout_rdb_text_value_from_hex(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for chunk in value.as_bytes().chunks_exact(2) {
        let high = layout_klayout_rdb_hex_digit(chunk[0])?;
        let low = layout_klayout_rdb_hex_digit(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    String::from_utf8(bytes).ok()
}

fn layout_klayout_rdb_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn layout_klayout_rdb_raw_values(state: Option<&MarkerState>) -> Vec<KlayoutRdbRawValue> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut values = BTreeMap::<usize, KlayoutRdbRawValue>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_raw_value_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let raw = values.entry(index).or_insert_with(|| KlayoutRdbRawValue {
            value: String::new(),
            tag: None,
        });
        match field {
            None | Some("value") => {
                if raw.tag.is_none() {
                    raw.tag = layout_klayout_rdb_value_tag(value);
                }
                raw.value = layout_klayout_rdb_value_without_tag(value)
                    .trim()
                    .to_string();
            }
            Some("tag") => raw.tag = Some(value.to_string()),
            _ => {}
        }
    }
    values
        .into_values()
        .filter(|value| !value.value.is_empty())
        .collect()
}

fn layout_klayout_rdb_raw_value_field(key: &str) -> Option<(usize, Option<&str>)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_RAW_VALUE_TAG_PREFIX)?;
    if let Ok(index) = rest.parse::<usize>() {
        return (index > 0).then_some((index, None));
    }
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, Some(field)))
}

fn layout_klayout_rdb_item_ordered_values(state: Option<&MarkerState>) -> Vec<String> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut values = state
        .tags
        .iter()
        .filter_map(|(key, value)| {
            let index = layout_klayout_rdb_index(key, KLAYOUT_RDB_ITEM_ORDERED_VALUE_TAG_PREFIX)?;
            let value = value.trim();
            (!value.is_empty()).then(|| (index, value.to_string()))
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|(index, _)| *index);
    values.into_iter().map(|(_, value)| value).collect()
}

fn layout_klayout_rdb_item_geometry_values(
    state: Option<&MarkerState>,
) -> Vec<KlayoutRdbGeometryValue> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut values = BTreeMap::<usize, KlayoutRdbGeometryValue>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_geometry_value_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let geometry = values
            .entry(index)
            .or_insert_with(|| KlayoutRdbGeometryValue {
                value: String::new(),
                tag: None,
            });
        match field {
            None | Some("value") => {
                if geometry.tag.is_none() {
                    geometry.tag = layout_klayout_rdb_value_tag(value);
                }
                geometry.value = layout_klayout_rdb_value_without_tag(value)
                    .trim()
                    .to_string();
            }
            Some("tag") => geometry.tag = Some(value.to_string()),
            _ => {}
        }
    }
    values
        .into_values()
        .filter(|value| layout_klayout_rdb_geometry_value_is_supported(&value.value))
        .collect()
}

fn layout_klayout_rdb_geometry_value_field(key: &str) -> Option<(usize, Option<&str>)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_GEOMETRY_VALUE_TAG_PREFIX)?;
    if let Ok(index) = rest.parse::<usize>() {
        return (index > 0).then_some((index, None));
    }
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, Some(field)))
}

fn layout_klayout_rdb_geometry_value_is_supported(value: &str) -> bool {
    let value = layout_klayout_rdb_value_without_tag(value);
    let Some((kind, body)) = value.split_once(':') else {
        return false;
    };
    if body.trim().is_empty() {
        return false;
    }
    matches!(
        layout_klayout_rdb_value_kind_key(kind).as_str(),
        "box"
            | "rectangle"
            | "rect"
            | "edge"
            | "edge_pair"
            | "edgepair"
            | "point"
            | "polygon"
            | "poly"
            | "path"
            | "label"
    ) || (layout_klayout_rdb_value_kind_key(kind) == "text"
        && layout_klayout_rdb_text_geometry_value_is_supported(body))
}

fn layout_klayout_rdb_ordered_value_kind_is_klayout_supported(kind_key: &str) -> bool {
    matches!(
        kind_key,
        "box" | "edge" | "edge_pair" | "polygon" | "path" | "label" | "text" | "float"
    )
}

fn layout_klayout_rdb_value_without_tag(value: &str) -> &str {
    layout_klayout_rdb_tagged_value(value).1
}

fn layout_klayout_rdb_value_tag(value: &str) -> Option<String> {
    layout_klayout_rdb_tagged_value(value).0
}

fn layout_klayout_rdb_tagged_value(value: &str) -> (Option<String>, &str) {
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
            let tag = layout_klayout_rdb_first_quoted_string(content).or_else(|| {
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

fn layout_klayout_rdb_value_kind_key(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn layout_klayout_rdb_text_geometry_value_is_supported(value: &str) -> bool {
    let Some(content) = layout_klayout_rdb_parenthesized_content(value) else {
        return false;
    };
    layout_klayout_rdb_first_quoted_string(content).is_some()
        && layout_klayout_rdb_last_coord_pair(content).is_some()
}

fn layout_klayout_rdb_parenthesized_content(value: &str) -> Option<&str> {
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

fn layout_klayout_rdb_first_quoted_string(value: &str) -> Option<String> {
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

fn layout_klayout_rdb_last_coord_pair(value: &str) -> Option<()> {
    let _ = layout_klayout_rdb_last_coord_pair_text(value)?;
    Some(())
}

fn layout_klayout_rdb_last_coord_pair_text(value: &str) -> Option<(&str, &str)> {
    value.match_indices(',').rev().find_map(|(comma_index, _)| {
        let x = layout_klayout_rdb_number_before(value, comma_index)?;
        let y = layout_klayout_rdb_number_after(value, comma_index + 1)?;
        (x.parse::<f64>().ok()?.is_finite() && y.parse::<f64>().ok()?.is_finite()).then_some((x, y))
    })
}

fn layout_klayout_rdb_point_export_box_body(value: &str) -> Option<String> {
    let content = layout_klayout_rdb_parenthesized_content(value).unwrap_or(value);
    let (x, y) = layout_klayout_rdb_last_coord_pair_text(content)?;
    Some(format!("({x},{y};{x},{y})"))
}

fn layout_klayout_rdb_reference_text_value(reference: &str) -> String {
    format!("reference: {}", reference.trim())
}

fn layout_klayout_rdb_raw_text_value(raw: &str) -> String {
    format!("{KLAYOUT_RDB_TEXT_RAW_VALUE_PREFIX}{}", raw.trim())
}

fn layout_klayout_rdb_number_before(value: &str, end: usize) -> Option<&str> {
    let mut end = end;
    while end > 0 && value[..end].chars().next_back()?.is_ascii_whitespace() {
        end -= value[..end].chars().next_back()?.len_utf8();
    }
    let mut start = end;
    while start > 0 {
        let character = value[..start].chars().next_back()?;
        if !layout_klayout_rdb_number_char(character) {
            break;
        }
        start -= character.len_utf8();
    }
    (start < end).then_some(value[start..end].trim())
}

fn layout_klayout_rdb_number_after(value: &str, start: usize) -> Option<&str> {
    let mut start = start;
    while start < value.len() && value[start..].chars().next()?.is_ascii_whitespace() {
        start += value[start..].chars().next()?.len_utf8();
    }
    let mut end = start;
    while end < value.len() {
        let character = value[end..].chars().next()?;
        if !layout_klayout_rdb_number_char(character) {
            break;
        }
        end += character.len_utf8();
    }
    (start < end).then_some(value[start..end].trim())
}

fn layout_klayout_rdb_number_char(character: char) -> bool {
    character.is_ascii_digit() || matches!(character, '+' | '-' | '.' | 'e' | 'E')
}

fn layout_klayout_rdb_index(key: &str, prefix: &str) -> Option<usize> {
    let index = key.strip_prefix(prefix)?.parse::<usize>().ok()?;
    (index > 0).then_some(index)
}

fn layout_klayout_rdb_category_descriptions(
    state: Option<&MarkerState>,
    part_count: usize,
) -> Vec<Option<String>> {
    let Some(state) = state else {
        return Vec::new();
    };
    (1..=part_count)
        .map(|index| {
            state
                .tags
                .get(&format!(
                    "{KLAYOUT_RDB_CATEGORY_DESCRIPTION_TAG_PREFIX}{index}"
                ))
                .map(String::as_str)
                .map(str::trim)
                .filter(|description| !description.is_empty())
                .map(str::to_string)
        })
        .collect()
}

fn layout_klayout_rdb_record_declared_categories(
    category_tree: &mut KlayoutRdbCategoryTree,
    state: Option<&MarkerState>,
) {
    let Some(state) = state else {
        return;
    };
    let mut categories = BTreeMap::<usize, (BTreeMap<usize, String>, Option<String>)>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_declared_category_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let entry = categories.entry(index).or_default();
        if field == "description" {
            entry.1 = Some(value.to_string());
        } else if let Some(part_index) = field
            .strip_prefix("part_")
            .and_then(|part| part.parse::<usize>().ok())
            .filter(|part| *part > 0)
        {
            entry.0.insert(part_index, value.to_string());
        }
    }
    for (_, (parts, description)) in categories {
        let parts = parts.into_values().collect::<Vec<_>>();
        if parts.is_empty() {
            continue;
        }
        let mut descriptions = vec![None; parts.len()];
        if let Some(description) = description
            && let Some(last) = descriptions.last_mut()
        {
            *last = Some(description);
        }
        category_tree.insert(&parts, &descriptions);
    }
}

fn layout_klayout_rdb_declared_category_field(key: &str) -> Option<(usize, &str)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_DECLARED_CATEGORY_TAG_PREFIX)?;
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, field))
}

#[derive(Default)]
struct KlayoutRdbDeclaredCellMetadata {
    name: Option<String>,
    variant: Option<String>,
    metadata: KlayoutRdbCellMetadata,
}

fn layout_klayout_rdb_record_declared_cells(
    cells: &mut BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
    state: Option<&MarkerState>,
) {
    let Some(state) = state else {
        return;
    };
    let mut declared_cells = BTreeMap::<usize, KlayoutRdbDeclaredCellMetadata>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_declared_cell_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() && field != "name" {
            continue;
        }
        let entry = declared_cells.entry(index).or_default();
        match field {
            "name" => entry.name = Some(value.to_string()),
            "variant" => entry.variant = Some(value.to_string()),
            "layout_name" => entry.metadata.layout_name = Some(value.to_string()),
            _ if field.starts_with("reference_") => {
                layout_klayout_rdb_record_declared_cell_reference(
                    &mut entry.metadata.references,
                    field,
                    value,
                );
            }
            _ => {}
        }
    }
    for (_, declared_cell) in declared_cells {
        let Some(name) = declared_cell.name else {
            continue;
        };
        let key = KlayoutRdbCellKey::new(&name, declared_cell.variant.clone());
        let metadata = cells.entry(key).or_default();
        if metadata.name.is_none() {
            metadata.name = Some(name);
        }
        if metadata.variant.is_none() {
            metadata.variant = declared_cell.variant;
        }
        if metadata.layout_name.is_none() {
            metadata.layout_name = declared_cell.metadata.layout_name;
        }
        for reference in declared_cell.metadata.references {
            if !metadata.references.contains(&reference) {
                metadata.references.push(reference);
            }
        }
    }
}

fn layout_klayout_rdb_record_declared_cell_reference(
    references: &mut Vec<KlayoutRdbCellReference>,
    field: &str,
    value: &str,
) {
    let rest = field.strip_prefix("reference_").unwrap_or_default();
    let Some((index, field)) = rest.split_once('_') else {
        return;
    };
    let Some(index) = index
        .parse::<usize>()
        .ok()
        .and_then(|index| index.checked_sub(1))
    else {
        return;
    };
    if references.len() <= index {
        references.resize_with(index + 1, || KlayoutRdbCellReference {
            parent: String::new(),
            trans: String::new(),
        });
    }
    match field {
        "parent" => references[index].parent = value.to_string(),
        "trans" => references[index].trans = value.to_string(),
        _ => {}
    }
}

fn layout_klayout_rdb_declared_cell_field(key: &str) -> Option<(usize, &str)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_DECLARED_CELL_TAG_PREFIX)?;
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, field))
}

fn layout_klayout_rdb_record_cell_metadata(
    cells: &mut BTreeMap<KlayoutRdbCellKey, KlayoutRdbCellMetadata>,
    cell: &str,
    state: Option<&MarkerState>,
) {
    let metadata = cells.entry(KlayoutRdbCellKey::literal(cell)).or_default();
    if metadata.name.is_none() {
        metadata.name = Some(cell.to_string());
    }
    let Some(state) = state else {
        return;
    };
    if metadata.layout_name.is_none()
        && let Some(layout_name) = state
            .tags
            .get(KLAYOUT_RDB_CELL_LAYOUT_NAME_TAG)
            .map(String::as_str)
            .map(str::trim)
            .filter(|layout_name| !layout_name.is_empty())
    {
        metadata.layout_name = Some(layout_name.to_string());
    }
    for reference in layout_klayout_rdb_cell_references(state) {
        if !metadata.references.contains(&reference) {
            metadata.references.push(reference);
        }
    }
}

fn layout_klayout_rdb_cell_references(state: &MarkerState) -> Vec<KlayoutRdbCellReference> {
    let mut references = BTreeMap::<usize, KlayoutRdbCellReference>::new();
    for (key, value) in &state.tags {
        let Some((index, field)) = layout_klayout_rdb_cell_reference_field(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let reference = references
            .entry(index)
            .or_insert_with(|| KlayoutRdbCellReference {
                parent: String::new(),
                trans: String::new(),
            });
        match field {
            "parent" => reference.parent = value.to_string(),
            "trans" => reference.trans = value.to_string(),
            _ => {}
        }
    }
    references
        .into_values()
        .filter(|reference| !reference.parent.is_empty() || !reference.trans.is_empty())
        .collect()
}

fn layout_klayout_rdb_cell_reference_field(key: &str) -> Option<(usize, &str)> {
    let rest = key.strip_prefix(KLAYOUT_RDB_CELL_REFERENCE_TAG_PREFIX)?;
    let (index, field) = rest.split_once('_')?;
    let index = index.parse::<usize>().ok()?;
    (index > 0).then_some((index, field))
}

fn layout_klayout_rdb_write_cell_references(
    xml: &mut String,
    references: &[KlayoutRdbCellReference],
) {
    xml.push_str("   <references>\n");
    for reference in references {
        xml.push_str("    <ref>\n");
        xml.push_str("     <parent>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&reference.parent));
        xml.push_str("</parent>\n");
        xml.push_str("     <trans>");
        xml.push_str(&layout_klayout_rdb_xml_escape(&reference.trans));
        xml.push_str("</trans>\n");
        xml.push_str("    </ref>\n");
    }
    xml.push_str("   </references>\n");
}

fn layout_klayout_rdb_write_category_tree(
    xml: &mut String,
    tree: &KlayoutRdbCategoryTree,
    depth: usize,
) {
    let indent = " ".repeat(depth);
    for (name, subtree) in &tree.children {
        xml.push_str(&indent);
        xml.push_str("<category>\n");
        xml.push_str(&indent);
        xml.push_str(" <name>");
        xml.push_str(&layout_klayout_rdb_xml_escape(name));
        xml.push_str("</name>\n");
        if let Some(description) = subtree
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
        {
            xml.push_str(&indent);
            xml.push_str(" <description>");
            xml.push_str(&layout_klayout_rdb_xml_escape(description));
            xml.push_str("</description>\n");
        } else {
            xml.push_str(&indent);
            xml.push_str(" <description/>\n");
        }
        xml.push_str(&indent);
        xml.push_str(" <categories>\n");
        layout_klayout_rdb_write_category_tree(xml, subtree, depth + 2);
        xml.push_str(&indent);
        xml.push_str(" </categories>\n");
        xml.push_str(&indent);
        xml.push_str("</category>\n");
    }
}

fn layout_klayout_rdb_violation_category(
    report: &LayoutDrcReportDatabaseEntryExchange,
    violation: &DrcViolation,
    state: Option<&MarkerState>,
    include_report_prefix: bool,
) -> Vec<String> {
    let mut parts = Vec::new();
    if include_report_prefix {
        parts.push(layout_klayout_rdb_category_part(&report.label));
    }
    if let Some(category) = state.and_then(layout_klayout_rdb_state_category) {
        parts.extend(category);
    } else {
        parts.push(layout_klayout_rdb_category_part(
            violation
                .rule
                .strip_prefix("calibre.")
                .or_else(|| violation.rule.strip_prefix("klayout."))
                .or_else(|| violation.rule.strip_prefix("external."))
                .unwrap_or(&violation.rule),
        ));
    }
    if parts.is_empty() {
        parts.push("Markers".to_string());
    }
    parts
}

fn layout_klayout_rdb_state_category(state: &MarkerState) -> Option<Vec<String>> {
    let category_parts = layout_klayout_rdb_category_part_values(state);
    if !category_parts.is_empty() {
        return Some(category_parts);
    }
    for key in ["rdb_category", "category", "marker_category"] {
        let Some(value) = state.tags.get(key).map(|value| value.trim()) else {
            continue;
        };
        let parts = value
            .split(['/', ':', '.'])
            .map(layout_klayout_rdb_category_part)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            return Some(parts);
        }
    }
    None
}

fn layout_klayout_rdb_category_part_values(state: &MarkerState) -> Vec<String> {
    let mut parts = state
        .tags
        .iter()
        .filter_map(|(key, value)| {
            let index = layout_klayout_rdb_index(key, KLAYOUT_RDB_CATEGORY_PART_TAG_PREFIX)?;
            let part = layout_klayout_rdb_category_part(value);
            (!part.is_empty()).then_some((index, part))
        })
        .collect::<Vec<_>>();
    parts.sort_by_key(|(index, _)| *index);
    parts.into_iter().map(|(_, part)| part).collect()
}

fn layout_klayout_rdb_violation_cell(state: Option<&MarkerState>, top_cell: &str) -> String {
    state
        .and_then(|state| state.tags.get("source_cell"))
        .map(String::as_str)
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .unwrap_or(top_cell)
        .to_string()
}

fn layout_klayout_rdb_item_tags(state: Option<&MarkerState>) -> Vec<String> {
    let Some(state) = state else {
        return Vec::new();
    };
    let mut tags = BTreeSet::new();
    if state.important {
        tags.insert("important".to_string());
    }
    if state.waived {
        tags.insert("waived".to_string());
    }
    if state.hidden {
        tags.insert("hidden".to_string());
    }
    for (key, value) in &state.tags {
        if key == KLAYOUT_RDB_CELL_LAYOUT_NAME_TAG
            || key == KLAYOUT_RDB_MULTIPLICITY_TAG
            || key.starts_with(KLAYOUT_RDB_RAW_VALUE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_CELL_REFERENCE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_CATEGORY_DESCRIPTION_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_CATEGORY_PART_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_DECLARED_CELL_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_DECLARED_CATEGORY_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_DECLARED_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_GEOMETRY_VALUE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_ITEM_ORDERED_VALUE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_ITEM_VALUE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_ITEM_REFERENCE_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_TAG_DESCRIPTION_TAG_PREFIX)
            || key.starts_with(KLAYOUT_RDB_TAG_NAME_TAG_PREFIX)
            || key == KLAYOUT_RDB_REPORT_DESCRIPTION_TAG
            || key == KLAYOUT_RDB_REPORT_DESCRIPTION_EMPTY_TAG
            || key == KLAYOUT_RDB_REPORT_GENERATOR_TAG
            || key == KLAYOUT_RDB_REPORT_GENERATOR_EMPTY_TAG
            || key == KLAYOUT_RDB_REPORT_ORIGINAL_FILE_TAG
            || key == KLAYOUT_RDB_REPORT_ORIGINAL_FILE_EMPTY_TAG
            || key == KLAYOUT_RDB_REPORT_TOP_CELL_TAG
            || key == KLAYOUT_RDB_REPORT_TOP_CELL_EMPTY_TAG
        {
            continue;
        }
        if matches!(
            key.as_str(),
            "rdb_category"
                | "category"
                | "marker_category"
                | "source_cell"
                | "rdb_image"
                | "rdb_image_base64"
                | "screenshot"
                | "screenshot_bounds"
                | "screenshot_format"
                | "screenshot_size"
        ) {
            continue;
        }
        let tag = if value.trim().is_empty() || value.eq_ignore_ascii_case("true") {
            layout_klayout_rdb_original_tag_name(state, key)
                .unwrap_or_else(|| layout_klayout_rdb_token(key))
        } else {
            layout_klayout_rdb_token(&format!("{key}_{}", layout_klayout_rdb_token(value)))
        };
        let tag = tag.trim();
        if !tag.is_empty() {
            tags.insert(tag.to_string());
        }
    }
    tags.into_iter().collect()
}

fn layout_klayout_rdb_tag_list(tags: &[String]) -> String {
    tags.iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| {
            if layout_klayout_rdb_tag_list_word(tag) {
                format!("#{tag}")
            } else {
                format!("#{}", layout_klayout_rdb_quoted_text(tag))
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn layout_klayout_rdb_tag_list_word(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn layout_klayout_rdb_original_tag_name(state: &MarkerState, key: &str) -> Option<String> {
    state
        .tags
        .get(&format!("{KLAYOUT_RDB_TAG_NAME_TAG_PREFIX}{key}"))
        .map(String::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
}

fn layout_klayout_rdb_item_image_base64(state: Option<&MarkerState>) -> Option<String> {
    let state = state?;
    state
        .tags
        .get("rdb_image_base64")
        .map(String::as_str)
        .map(str::trim)
        .filter(|image| !image.is_empty())
        .map(str::to_string)
        .or_else(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = state
                    .tags
                    .get("screenshot")
                    .map(String::as_str)
                    .map(str::trim)
                    .filter(|path| !path.is_empty())?;
                fs::read(path)
                    .ok()
                    .filter(|bytes| !bytes.is_empty())
                    .map(|bytes| general_purpose::STANDARD.encode(bytes))
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        })
}

fn layout_klayout_rdb_top_cell_name(document: &Document) -> String {
    document
        .cell(document.top_cell)
        .map(|cell| cell.name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| document.name.trim())
        .to_string()
}

fn layout_klayout_rdb_category_part(value: &str) -> String {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    let mut part = String::new();
    let mut previous_space = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric()
            || matches!(character, '_' | '-' | '/' | '.' | '\'' | '"' | '\\' | ' ')
        {
            if character.is_ascii_whitespace() {
                if !previous_space {
                    part.push(' ');
                    previous_space = true;
                }
            } else {
                part.push(character);
                previous_space = false;
            }
        } else if character == ':' {
            part.push(' ');
            previous_space = true;
        }
    }
    let part = part.trim().to_string();
    if part.is_empty() {
        "Markers".to_string()
    } else {
        part
    }
}

fn layout_klayout_rdb_token(value: &str) -> String {
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
    output.trim_matches('_').to_string()
}

fn layout_klayout_rdb_category_path(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| layout_klayout_rdb_quoted_category_part(part))
        .collect::<Vec<_>>()
        .join(".")
}

fn layout_klayout_rdb_quoted_category_part(part: &str) -> String {
    if part
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        part.to_string()
    } else {
        layout_klayout_rdb_quoted_text(part)
    }
}

fn layout_klayout_rdb_box_value(bounds: Rect) -> String {
    format!(
        "box: ({},{};{},{})",
        layout_klayout_rdb_coord(bounds.min.x),
        layout_klayout_rdb_coord(bounds.min.y),
        layout_klayout_rdb_coord(bounds.max.x),
        layout_klayout_rdb_coord(bounds.max.y)
    )
}

fn layout_klayout_rdb_coord(coord: Coord) -> String {
    let mut value = format!("{:.6}", coord as f64 / geometry_core::DBU_PER_MICRON as f64);
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    if value == "-0" {
        "0".to_string()
    } else {
        value
    }
}

fn layout_klayout_rdb_quoted_text(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn layout_klayout_rdb_text_payload(value: &str) -> String {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return layout_klayout_rdb_quoted_text(value);
    };
    let native_unquoted = !first.is_ascii_digit()
        && (first.is_ascii_alphanumeric() || matches!(first, '_' | '.'))
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'));
    if native_unquoted {
        value.to_string()
    } else {
        layout_klayout_rdb_quoted_text(value)
    }
}

fn layout_klayout_rdb_xml_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&apos;".chars().collect::<Vec<_>>(),
            _ => vec![character],
        })
        .collect()
}
