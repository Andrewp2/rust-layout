#![allow(unused_imports)]
use super::*;

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
        let Some(shape) = self.selected_top_level_layout_shape() else {
            return "Select a top-level rectangle or polygon before running region DRC".to_string();
        };
        let Some(region_parts) = convex_region_parts_for_shape_kind(&shape.kind) else {
            return "Region DRC requires a usable polygon region".to_string();
        };

        let rules = self.active_layout_drc_deck_for(&self.workspace.document);
        let deck_label = self.active_layout_drc_deck_label();
        let rule_findings = rules.validate_for_document(&self.workspace.document);
        let mut full_violation_count = 0;
        let value = if rule_findings.is_empty() {
            let violations = run_drc(&self.workspace.document, &rules);
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
        let contents = match fs::read_to_string(path) {
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
            "Exported DRC report {} ({} marker(s), {} state(s), {} deck issue(s))",
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
        let contents = match fs::read_to_string(path) {
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
            "Imported DRC report {} ({} marker(s), {} deck issue(s))",
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_drc_report_database_to_path(&mut self, path: &Path) -> bool {
        let reports = if self.layout_drc_report_history.is_empty() {
            let Some(report) = self.drc_report() else {
                self.status_message = "Run or import DRC before exporting reports".to_string();
                return true;
            };
            vec![LayoutDrcReportDatabaseEntryExchange {
                label: "Active DRC Report".to_string(),
                layout_revision: self.layout_revision,
                findings: report.findings,
                violations: report.violations,
            }]
        } else {
            self.layout_drc_report_history
                .iter()
                .map(|entry| LayoutDrcReportDatabaseEntryExchange {
                    label: entry.label.clone(),
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
            "Exported DRC report database {} ({} report(s), {} marker(s), {} state(s), {} deck issue(s))",
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_drc_report_database_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("DRC report database import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutDrcReportDatabaseExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("DRC report database import failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "DRC report database import failed: database schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        if exchange.reports.is_empty() {
            self.status_message =
                "DRC report database import failed: database has no reports".to_string();
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
                    "DRC report database import failed in {label}: {}",
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
            self.status_message = format!("DRC report database import failed: {error}");
            return true;
        }

        let original_report_count = exchange.reports.len();
        let report_limit = original_report_count.min(MAX_LAYOUT_DRC_REPORT_HISTORY);
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
        let mut history = Vec::new();
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
            history.push(DrcReportHistoryEntry {
                id,
                revision: report.layout_revision,
                label,
                value: DrcReportCacheValue {
                    findings: report.findings,
                    violations: report.violations,
                },
            });
        }
        let active_index = exchange
            .active_report_index
            .filter(|index| *index < history.len())
            .unwrap_or(0);
        self.layout_next_drc_report_id = next_id;
        self.layout_selected_drc_report_id = history.get(active_index).map(|entry| entry.id);
        self.layout_drc_report_history = history;
        self.layout_selected_drc_marker_key = None;
        self.layout_drc_marker_category_filter = None;
        self.sync_selected_layout_drc_report_cache();
        self.show_drc_overlay = true;
        self.record_recent_layout_file("drc_report_database", path);
        let truncated_suffix = if original_report_count > report_limit {
            format!("; truncated to {report_limit}")
        } else {
            String::new()
        };
        self.status_message = format!(
            "Imported DRC report database {} ({} report(s), {} marker(s), {} state(s), {} deck issue(s)){truncated_suffix}",
            path.display(),
            self.layout_drc_report_history.len(),
            marker_count,
            imported_state_count,
            finding_count
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
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Calibre/RVE marker import failed: {error}");
                return true;
            }
        };
        let imported = import_calibre_rve_markers(&contents);
        if imported.violations.is_empty() {
            self.status_message = format!(
                "Calibre/RVE marker import found no markers in {} ({} skipped line(s))",
                path.display(),
                imported.report.skipped_line_count
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
        let marker_count = imported.violations.len();
        let skipped_line_count = imported.report.skipped_line_count;
        let label = format!(
            "Calibre/RVE {}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("markers")
        );
        self.push_layout_drc_report(
            label,
            DrcReportCacheValue {
                findings: Vec::new(),
                violations: imported.violations,
            },
        );
        self.layout_drc_marker_filter = LayoutDrcMarkerFilter::Active;
        self.layout_drc_marker_sort = LayoutDrcMarkerSort::Id;
        self.layout_drc_marker_category_filter = None;
        self.layout_selected_drc_marker_key = None;
        self.layout_browser_search.clear();
        self.show_drc_overlay = true;
        self.record_recent_layout_file("calibre_rve", path);
        self.status_message = format!(
            "Imported Calibre/RVE markers {} ({} marker(s), {} skipped line(s))",
            path.display(),
            marker_count,
            skipped_line_count
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
