#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub fn layout_diff_changed_only(&self) -> bool {
        self.layout_diff_changed_only
    }

    pub fn layout_diff_change_filter(&self) -> LayoutChangeFilter {
        self.layout_diff_change_filter
    }

    pub fn layout_diff_review_state(&self) -> LayoutReviewDisposition {
        self.layout_diff_review_state
    }

    pub fn selected_trace_lot(&self) -> Option<&LotId> {
        self.selected_trace_lot.as_ref()
    }

    pub fn selected_trace_wafer(&self) -> Option<&WaferId> {
        self.selected_trace_wafer.as_ref()
    }

    pub fn trace_impact_mode(&self) -> TraceImpactMode {
        self.trace_impact_mode
    }

    pub fn trace_related_only(&self) -> bool {
        self.trace_related_only
    }

    pub fn selected_notebook_entry(&self) -> Option<&NotebookEntryId> {
        self.selected_notebook_entry.as_ref()
    }

    pub fn notebook_tag_filter(&self) -> Option<&str> {
        self.notebook_tag_filter.as_deref()
    }

    pub fn notebook_link_kind_filter(&self) -> Option<NotebookLinkKind> {
        self.notebook_link_kind_filter
    }

    pub fn notebook_preview_mode(&self) -> bool {
        self.notebook_preview_mode
    }

    pub fn notebook_followups_only(&self) -> bool {
        self.notebook_followups_only
    }

    pub fn scheduler_policy(&self) -> DispatchPolicy {
        self.scheduler_policy
    }

    pub fn selected_scheduler_tool(&self) -> Option<&SchedulerToolId> {
        self.selected_scheduler_tool.as_ref()
    }

    pub fn scheduler_min_priority(&self) -> u8 {
        self.scheduler_min_priority
    }

    pub fn scheduler_conflicts_only(&self) -> bool {
        self.scheduler_conflicts_only
    }

    pub fn scheduler_focus_selected_tool(&self) -> bool {
        self.scheduler_focus_selected_tool
    }

    pub fn selected_safety_tool(&self) -> Option<&str> {
        self.selected_safety_tool.as_deref()
    }

    pub fn acknowledged_count(&self) -> usize {
        self.acknowledged_conditions.len()
            + self.acknowledged_lockouts.len()
            + self.acknowledged_incidents.len()
    }

    pub fn selected_process_node(&self) -> Option<&ProcessFlowNodeId> {
        self.selected_process_node.as_ref()
    }

    pub fn process_flow_filter(&self) -> ProcessFlowNodeFilter {
        self.process_flow_filter
    }

    pub fn process_flow_errors_only(&self) -> bool {
        self.process_flow_errors_only
    }

    pub fn selected_control_loop(&self) -> Option<&ControlLoopId> {
        self.selected_control_loop.as_ref()
    }

    pub fn selected_control_action(&self) -> Option<&ControlActionId> {
        self.selected_control_action.as_ref()
    }

    pub fn selected_spc_chart(&self) -> Option<&str> {
        self.selected_spc_chart.as_deref()
    }

    pub fn selected_fdc_trace(&self) -> Option<&str> {
        self.selected_fdc_trace.as_deref()
    }

    pub fn spc_severity_filter(&self) -> SpcSeverityFilter {
        self.spc_severity_filter
    }

    pub fn spc_source_filter(&self) -> SpcSourceFilter {
        self.spc_source_filter
    }

    pub fn cross_section_step(&self) -> usize {
        self.cross_section_step
    }

    pub fn cross_section_show_mask(&self) -> bool {
        self.cross_section_show_mask
    }

    pub fn metrology_map_mode(&self) -> MetrologyMapMode {
        self.metrology_map_mode
    }

    pub fn metrology_kind(&self) -> MeasurementKind {
        self.metrology_kind
    }

    pub fn metrology_failed_only(&self) -> bool {
        self.metrology_failed_only
    }

    pub fn selected_die(&self) -> Option<DieCoord> {
        self.selected_die
    }

    pub fn yield_map_filter(&self) -> YieldMapFilter {
        self.yield_map_filter
    }

    pub fn show_only_attention_wafers(&self) -> bool {
        self.show_only_attention_wafers
    }

    pub fn show_only_excursions(&self) -> bool {
        self.show_only_excursions
    }

    pub fn selected_yield_lot(&self) -> Option<&str> {
        self.selected_yield_lot.as_deref()
    }

    pub fn selected_yield_wafer(&self) -> Option<&str> {
        self.selected_yield_wafer.as_deref()
    }

    pub fn experiment_show_missing_only(&self) -> bool {
        self.experiment_show_missing_only
    }

    pub(crate) fn ensure_experiment_selection(&mut self) {
        if self
            .selected_experiment_run
            .as_ref()
            .is_none_or(|run_id| self.workspace.experiment_plan.run(run_id).is_none())
        {
            self.selected_experiment_run = default_experiment_run(&self.workspace);
        }
        if self
            .selected_experiment_response
            .as_ref()
            .is_none_or(|response_id| {
                self.workspace
                    .experiment_plan
                    .response(response_id)
                    .is_none()
            })
        {
            self.selected_experiment_response = default_experiment_response(&self.workspace);
        }
        if self.experiment_lot_filter.as_deref().is_some_and(|lot_id| {
            !self
                .workspace
                .experiment_plan
                .runs
                .iter()
                .any(|run| run.assignment.lot_id.as_str() == lot_id)
        }) {
            self.experiment_lot_filter = None;
        }
    }

    pub(crate) fn selected_experiment_response_if_valid(&self) -> Option<ResponseSpecId> {
        self.selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
            .map(|response| response.id.clone())
    }

    pub(crate) fn selected_experiment_response_label(&self) -> String {
        self.selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
            .map(|response| response.name.clone())
            .unwrap_or_else(|| "primary response".to_string())
    }

    pub(crate) fn selected_experiment_run_label(&self) -> String {
        let Some(run) = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| self.workspace.experiment_plan.run(run_id))
        else {
            return "none".to_string();
        };
        format!(
            "{} {} W{:02}",
            display_experiment_run_label(run),
            run.assignment.lot_id,
            run.assignment.slot
        )
    }

    pub(crate) fn experiment_lot_options(&self) -> Vec<String> {
        self.workspace
            .experiment_plan
            .runs
            .iter()
            .map(|run| run.assignment.lot_id.as_str().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(crate) fn experiment_run_matrix_rows(&self) -> Vec<ExperimentRunMatrixRow> {
        let plan = &self.workspace.experiment_plan;
        let primary_response_id = self.selected_experiment_response_if_valid();
        plan.runs
            .iter()
            .map(|run| {
                let primary_value = primary_response_id
                    .as_ref()
                    .and_then(|response_id| {
                        let response = plan.response(response_id)?;
                        let value = run.responses.get(response_id)?;
                        Some(format_experiment_response_value(value.value, response))
                    })
                    .unwrap_or_else(|| "pending".to_string());
                let missing_count = plan.responses.len().saturating_sub(run.responses.len());
                let has_out_of_spec = run.responses.iter().any(|(response_id, value)| {
                    plan.response(response_id).is_some_and(|response| {
                        response.value_status(value.value) != ResponseValueStatus::InSpec
                    })
                });
                let needs_selected_response = primary_response_id
                    .as_ref()
                    .is_some_and(|response_id| !run.responses.contains_key(response_id));
                ExperimentRunMatrixRow {
                    run_id: run.id.to_string(),
                    selected: self.selected_experiment_run.as_ref() == Some(&run.id),
                    run_order: run.assignment.run_order,
                    lot_id: run.assignment.lot_id.as_str().to_string(),
                    wafer_id: run.assignment.wafer_id.as_str().to_string(),
                    block: run
                        .assignment
                        .block
                        .clone()
                        .unwrap_or_else(|| "unblocked".to_string()),
                    factor_values: plan
                        .factors
                        .iter()
                        .map(|factor| plan.run_factor_label(run, factor))
                        .collect(),
                    status: run.status.label().to_string(),
                    status_kind: run.status,
                    capture_summary: format!("{}/{}", run.responses.len(), plan.responses.len()),
                    missing_count,
                    needs_selected_response,
                    has_out_of_spec,
                    primary_value,
                }
            })
            .collect()
    }

    pub(crate) fn filtered_experiment_run_rows(&self) -> Vec<ExperimentRunMatrixRow> {
        self.experiment_run_matrix_rows()
            .into_iter()
            .filter(|row| {
                self.experiment_lot_filter
                    .as_deref()
                    .is_none_or(|lot_id| row.lot_id == lot_id)
            })
            .filter(|row| self.experiment_run_filter.matches(row))
            .filter(|row| !self.experiment_show_missing_only || row.missing_count > 0)
            .collect()
    }

    pub(crate) fn select_next_pending_experiment_response(&mut self) -> bool {
        let plan = &self.workspace.experiment_plan;
        if plan.runs.is_empty() || plan.responses.is_empty() {
            return false;
        }
        let run_count = plan.runs.len();
        let selected_run_index = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| plan.runs.iter().position(|run| &run.id == run_id))
            .unwrap_or(0);

        for offset in 0..run_count {
            let index = (selected_run_index + offset) % run_count;
            let run = &plan.runs[index];
            for response in &plan.responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                self.selected_experiment_run = Some(run.id.clone());
                self.selected_experiment_response = Some(response.id.clone());
                self.experiment_capture_value = response
                    .target
                    .or_else(|| demo_experiment_response_value(run, &response.id))
                    .unwrap_or_default();
                return true;
            }
        }
        false
    }

    pub(crate) fn capture_selected_experiment_response(&mut self) -> bool {
        let Some(run_id) = self.selected_experiment_run.clone() else {
            self.status_message = "DOE: select a run before capturing a response".to_string();
            return false;
        };
        let Some(response_id) = self.selected_experiment_response.clone() else {
            self.status_message = "DOE: select a response before capturing a value".to_string();
            return false;
        };
        let response_label = self.selected_experiment_response_label();
        let value = ResponseValue {
            value: self.experiment_capture_value,
            measurement_id: Some(format!("DOE-{run_id}-{response_id}")),
            captured_at: "2026-05-06T16:00:00Z".to_string(),
        };
        match self
            .workspace
            .experiment_plan
            .capture_response(&run_id, &response_id, value)
        {
            Ok(()) => {
                self.status_message = format!("DOE: captured {response_label} for {run_id}");
                true
            }
            Err(error) => {
                self.status_message = format!("DOE capture blocked: {error}");
                false
            }
        }
    }

    pub(crate) fn capture_next_demo_experiment_response(&mut self) {
        let runs = self.workspace.experiment_plan.runs.clone();
        let responses = self.workspace.experiment_plan.responses.clone();
        for run in runs {
            for response in &responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                let Some(value) = demo_experiment_response_value(&run, &response.id) else {
                    continue;
                };
                let run_id = run.id.clone();
                let response_id = response.id.clone();
                let capture = ResponseValue {
                    value,
                    measurement_id: Some(format!("SIM-{run_id}-{response_id}")),
                    captured_at: "2026-05-06T16:30:00Z".to_string(),
                };
                match self.workspace.experiment_plan.capture_response(
                    &run_id,
                    &response_id,
                    capture,
                ) {
                    Ok(()) => {
                        self.selected_experiment_run = Some(run_id.clone());
                        self.selected_experiment_response = Some(response_id.clone());
                        self.experiment_capture_value = value;
                        self.status_message =
                            format!("DOE: captured demo {response_id} for {run_id}");
                    }
                    Err(error) => {
                        self.status_message = format!("DOE capture blocked: {error}");
                    }
                }
                return;
            }
        }
        self.status_message = "DOE: no pending sample responses".to_string();
    }

    pub(crate) fn use_demo_experiment_capture_value(&mut self) {
        let Some(run) = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| self.workspace.experiment_plan.run(run_id))
        else {
            self.status_message = "DOE: select a run before loading a sample value".to_string();
            return;
        };
        let Some(response_id) = self.selected_experiment_response.as_ref() else {
            self.status_message =
                "DOE: select a response before loading a sample value".to_string();
            return;
        };
        let Some(value) = demo_experiment_response_value(run, response_id) else {
            self.status_message = "DOE: no sample value available for this response".to_string();
            return;
        };
        self.experiment_capture_value = value;
        self.status_message = format!("DOE: loaded sample value {}", format_compact_number(value));
    }

    pub(crate) fn use_experiment_response_target(&mut self) {
        let Some(response) = self
            .selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
        else {
            self.status_message = "DOE: select a response before using a target".to_string();
            return;
        };
        let Some(target) = response.target else {
            self.status_message = format!("DOE: {} has no target value", response.name);
            return;
        };
        let response_name = response.name.clone();
        self.experiment_capture_value = target;
        self.status_message = format!("DOE: loaded target for {response_name}");
    }

    pub fn show_inspector(&self) -> bool {
        self.show_inspector
    }

    pub fn show_layers(&self) -> bool {
        self.show_layers
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub(crate) fn toggle_inspector_panel(&mut self) {
        self.show_inspector = !self.show_inspector;
        self.status_message = format!(
            "Details {}",
            if self.show_inspector {
                "shown"
            } else {
                "hidden"
            }
        );
    }

    pub(crate) fn toggle_secondary_panel(&mut self) {
        self.show_layers = !self.show_layers;
        self.status_message = format!(
            "{} {}",
            self.secondary_panel_label(),
            if self.show_layers { "shown" } else { "hidden" }
        );
    }

    pub(crate) fn toggle_detail_section(&mut self, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }
        if !self.collapsed_detail_sections.remove(key) {
            self.collapsed_detail_sections.insert(key.to_string());
        }
        true
    }

    pub(crate) fn detail_section_key_for_header_node(&self, name: &str) -> Option<String> {
        if let Some(raw_index) = name.strip_prefix("glassworks.domain.section.") {
            let index = raw_index.strip_suffix(".header")?.parse::<usize>().ok()?;
            return Some(domain_section_key(self.active_view, index));
        }
        let (raw_index, sections) =
            if let Some(raw_index) = name.strip_prefix("glassworks.layout.inspector.section.") {
                (raw_index, layout_editor_inspector_sections(self))
            } else if let Some(raw_index) = name.strip_prefix("glassworks.details.section.") {
                (raw_index, view_detail_sections(self))
            } else if let Some(raw_index) = name.strip_prefix("glassworks.inspector.section.") {
                (
                    raw_index,
                    view_detail_sections(self)
                        .into_iter()
                        .filter(|section| section.title != "Current Controls")
                        .collect(),
                )
            } else {
                return None;
            };
        let index = raw_index.strip_suffix(".header")?.parse::<usize>().ok()?;
        sections
            .get(index)
            .map(|section| detail_section_key(self.active_view, index, &section.title))
    }

    pub fn set_active_view(&mut self, view: StartupView) {
        let previous = self.active_view;
        self.active_view = view;
        self.active_menu = None;
        self.active_view_group = None;
        if previous != view {
            self.reset_layout_frame_timing();
        }
        if view != StartupView::Layout3d {
            self.flycam_captured = false;
            self.layout_3d_drag = None;
        }
        if !matches!(view, StartupView::Layout2d | StartupView::Layout3d) {
            self.viewport_fullscreen = false;
        }
    }

    pub fn apply_clicked_node_name(&mut self, name: &str) -> bool {
        if let Some(target) = name.strip_prefix("glassworks.options.proxy.") {
            let handled = self.apply_clicked_node_name(target);
            if handled {
                self.sync_app_options_from_state();
            }
            return handled;
        }

        if let Some(slug) = name.strip_prefix("glassworks.menu.") {
            if let Some(menu) = AppMenu::from_slug(slug) {
                let next_menu = (self.active_menu != Some(menu)).then_some(menu);
                self.active_menu = next_menu;
                if next_menu != Some(AppMenu::View) {
                    self.active_view_group = None;
                }
                self.status_message = format!("{} menu", menu.label());
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.nav.action.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(key) = name.strip_prefix("glassworks.inspector.section.toggle.") {
            return self.toggle_detail_section(key);
        }

        if let Some(key) = self.detail_section_key_for_header_node(name) {
            return self.toggle_detail_section(&key);
        }

        if let Some(slug) = name.strip_prefix("glassworks.tool.") {
            if let Some(tool) = ToolMode::from_slug(slug) {
                self.active_tool = tool;
                self.status_message = format!("{} tool selected", tool.label());
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.drawer.") {
            match slug {
                "inspector" => {
                    self.toggle_inspector_panel();
                    return true;
                }
                "layers" => {
                    self.toggle_secondary_panel();
                    return true;
                }
                _ => {}
            }
        }

        if let Some(raw) = name.strip_prefix("glassworks.primary.action.") {
            let mut candidate = raw;
            loop {
                if self.apply_view_control_action(candidate) {
                    self.sync_app_options_from_state();
                    return true;
                }
                let Some((_, next)) = candidate.split_once('.') else {
                    return false;
                };
                candidate = next;
            }
        }

        if let Some(action) = name.strip_prefix("glassworks.viewctl.") {
            let handled = self.apply_view_control_action(action);
            if handled {
                self.sync_app_options_from_state();
            }
            return handled;
        }

        if let Some(action) = name.strip_prefix("glassworks.toolbar.") {
            if let Some(slug) = action.strip_prefix("view.") {
                if let Some(view) = StartupView::from_slug(slug) {
                    self.set_active_view(view);
                    return true;
                }
            }
            self.apply_menu_action(action);
            return true;
        }

        if let Some(slug) = name.strip_prefix("glassworks.menu.item.view.group.") {
            if let Some(group) = ModuleGroup::from_slug(slug) {
                self.active_view_group = Some(group);
                self.active_menu = Some(AppMenu::View);
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.menu.item.more.") {
            if let Some(menu) = AppMenu::from_slug(slug) {
                self.active_menu = Some(menu);
                self.active_view_group = None;
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.menu.item.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.command.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                self.show_command_palette = false;
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("glassworks.sidebar.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                if self.nav_rail_views.contains(&view) {
                    self.nav_rail_views.remove(&view);
                } else {
                    self.nav_rail_views.insert(view);
                }
                self.sync_app_options_from_state();
                return true;
            }
        }

        if let Some(layer_id) = name.strip_prefix("glassworks.layers.delete.") {
            return self.remove_layout_layer(layer_id);
        }

        if let Some(action) = name.strip_prefix("glassworks.sidebar.") {
            match action {
                "default" | "all" => {
                    self.nav_rail_views = default_nav_rail_views();
                    self.sync_app_options_from_state();
                    return true;
                }
                "none" => {
                    self.nav_rail_views.clear();
                    self.sync_app_options_from_state();
                    return true;
                }
                "close" => {
                    self.show_sidebar_modules = false;
                    self.sync_app_options_from_state();
                    return true;
                }
                _ => {}
            }
        }

        if name == "glassworks.command.close" {
            self.show_command_palette = false;
            return true;
        }

        if name == "glassworks.options.close" {
            self.show_options_panel = false;
            self.sync_app_options_from_state();
            return true;
        }

        if name == "glassworks.diagnostics.close" {
            self.show_diagnostics_panel = false;
            self.sync_app_options_from_state();
            return true;
        }

        if let Some(action) = name.strip_prefix("glassworks.options.action.") {
            return self.apply_options_action(action);
        }

        if let Some(action) = name.strip_prefix("glassworks.menu.item.") {
            self.apply_menu_action(action);
            self.sync_app_options_from_state();
            return true;
        }

        false
    }

    pub(crate) fn trace_selection_exists(&self, selection: &TraceSelection) -> bool {
        match selection {
            TraceSelection::Lot(lot_id) => self.workspace.genealogy.lots.contains_key(lot_id),
            TraceSelection::Wafer(wafer) => self.workspace.genealogy.wafer(wafer).is_some(),
            TraceSelection::Process(sequence) => self
                .workspace
                .genealogy
                .process_history
                .iter()
                .any(|record| record.sequence == *sequence),
            TraceSelection::MaterialUse(sequence) => self
                .workspace
                .genealogy
                .material_uses
                .iter()
                .any(|record| record.sequence == *sequence),
            TraceSelection::Event(sequence) => self
                .workspace
                .genealogy
                .events
                .iter()
                .any(|event| event.sequence == *sequence),
        }
    }

    pub(crate) fn apply_options_action(&mut self, action: &str) -> bool {
        if let Some(theme) = action.strip_prefix("appearance.theme.") {
            self.theme_preference = match theme {
                "dark" => options::ThemePreference::Dark,
                "light" => options::ThemePreference::Light,
                "system" => options::ThemePreference::System,
                _ => return false,
            };
            self.dark_theme = self.theme_preference != options::ThemePreference::Light;
            self.app_options.appearance.theme = self.theme_preference;
            self.status_message = format!("Theme preference set to {theme}");
            return true;
        }
        if let Some(scale) = action.strip_prefix("appearance.ui_scale.") {
            let Some(scale) = option_scale_from_slug(scale) else {
                return false;
            };
            self.app_options.appearance.ui_scale = scale;
            self.status_message = format!("UI scale preference set to {:.0}%", scale * 100.0);
            return true;
        }
        if action == "appearance.dense_mode" {
            self.app_options.appearance.dense_mode = !self.app_options.appearance.dense_mode;
            self.status_message = format!(
                "Dense UI mode {}",
                if self.app_options.appearance.dense_mode {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            return true;
        }
        if action == "shell.command_palette_on_start" {
            self.app_options.shell.show_command_palette_on_start =
                !self.app_options.shell.show_command_palette_on_start;
            self.show_command_palette = self.app_options.shell.show_command_palette_on_start;
            self.status_message = format!(
                "Startup command palette {}",
                option_enabled(self.app_options.shell.show_command_palette_on_start)
            );
            return true;
        }
        if let Some(layer) = action.strip_prefix("layout.active_layer.") {
            let Some(layer) = layer.parse::<u32>().ok().map(LayerId) else {
                return false;
            };
            if !self.workspace.document.layers.contains_key(&layer) {
                return false;
            }
            self.active_layer = layer;
            self.app_options.layout.active_layer = layer.0;
            self.status_message = format!("Default layer set to L{}", layer.0);
            return true;
        }
        if let Some(zoom) = action.strip_prefix("layout.zoom.") {
            let Some(zoom) = option_zoom_from_slug(zoom) else {
                return false;
            };
            if (self.layout_zoom - zoom).abs() > f32::EPSILON {
                self.record_layout_previous_view();
            }
            self.layout_zoom = zoom;
            self.app_options.layout.zoom = zoom;
            self.status_message = format!("Default layout zoom set to {zoom:.4}");
            return true;
        }
        if action == "layout.pan.reset" {
            if self.layout_pan != [0.0, 0.0] {
                self.record_layout_previous_view();
            }
            self.layout_pan = [0.0, 0.0];
            self.app_options.layout.pan = self.layout_pan;
            self.status_message = "Default layout pan reset".to_string();
            return true;
        }
        if let Some(modifier) = action.strip_prefix("layout.modifier.") {
            let target = match modifier {
                "shift" => &mut self.app_options.layout.constrain_with_shift,
                "ctrl" => &mut self.app_options.layout.bypass_snap_with_ctrl,
                "alt" => &mut self.app_options.layout.duplicate_drag_with_alt,
                _ => return false,
            };
            *target = !*target;
            self.status_message = format!("Layout modifier {modifier} {}", option_enabled(*target));
            return true;
        }
        if let Some(speed) = action.strip_prefix("viewport3d.speed.") {
            let Some(speed) = option_speed_from_slug(speed) else {
                return false;
            };
            self.camera_3d.speed = speed;
            self.app_options.viewport3d.movement_speed = speed;
            self.status_message = format!("3D flycam speed set to {speed:.0}");
            return true;
        }
        if let Some(fov) = action.strip_prefix("viewport3d.fov.") {
            let Some(fov) = option_fov_from_slug(fov) else {
                return false;
            };
            self.camera_3d.fov_y = fov.to_radians();
            self.app_options.viewport3d.fov_degrees = fov;
            self.status_message = format!("3D FOV set to {fov:.0} degrees");
            return true;
        }
        if let Some(sensitivity) = action.strip_prefix("viewport3d.sensitivity.") {
            let Some(sensitivity) = option_sensitivity_from_slug(sensitivity) else {
                return false;
            };
            self.app_options.viewport3d.mouse_sensitivity = sensitivity;
            self.status_message = format!("3D mouse sensitivity set to {sensitivity:.4}");
            return true;
        }
        if let Some(multiplier) = action.strip_prefix("viewport3d.fast_multiplier.") {
            let Some(multiplier) = option_fast_multiplier_from_slug(multiplier) else {
                return false;
            };
            self.app_options.viewport3d.fast_multiplier = multiplier;
            self.status_message = format!("3D fast multiplier set to {multiplier:.0}x");
            return true;
        }
        if action == "viewport3d.capture_flycam" {
            self.app_options.viewport3d.capture_flycam_on_click =
                !self.app_options.viewport3d.capture_flycam_on_click;
            self.status_message = format!(
                "Click-to-capture flycam {}",
                option_enabled(self.app_options.viewport3d.capture_flycam_on_click)
            );
            return true;
        }
        if action == "domains.workflow.load_demo_on_start" {
            self.app_options.domains.workflow.load_demo_on_start =
                !self.app_options.domains.workflow.load_demo_on_start;
            self.status_message = format!(
                "Startup sample workspace {}",
                option_enabled(self.app_options.domains.workflow.load_demo_on_start)
            );
            return true;
        }
        if action == "domains.fab_control.auto_select_first_tool" {
            self.app_options.domains.fab_control.auto_select_first_tool =
                !self.app_options.domains.fab_control.auto_select_first_tool;
            if self.app_options.domains.fab_control.auto_select_first_tool
                && self.selected_equipment_tool.is_none()
            {
                self.selected_equipment_tool = default_equipment_tool(&self.workspace);
            } else if !self.app_options.domains.fab_control.auto_select_first_tool {
                self.selected_equipment_tool = None;
            }
            self.status_message = format!(
                "Fab Control startup tool selection {}",
                option_enabled(self.app_options.domains.fab_control.auto_select_first_tool)
            );
            return true;
        }
        if action == "domains.environment.alarm_sensors_first" {
            self.app_options
                .domains
                .environment
                .show_alarm_sensors_first = !self
                .app_options
                .domains
                .environment
                .show_alarm_sensors_first;
            if self
                .app_options
                .domains
                .environment
                .show_alarm_sensors_first
            {
                self.selected_environment_sensor =
                    default_environment_sensor_with_options(&self.workspace, true);
            }
            self.status_message = format!(
                "Environment alarm sensors first {}",
                option_enabled(
                    self.app_options
                        .domains
                        .environment
                        .show_alarm_sensors_first
                )
            );
            return true;
        }
        if action == "domains.safety.show_acknowledged" {
            self.app_options.domains.safety.show_acknowledged =
                !self.app_options.domains.safety.show_acknowledged;
            self.status_message = format!(
                "Acknowledged safety items {}",
                if self.app_options.domains.safety.show_acknowledged {
                    "shown"
                } else {
                    "hidden"
                }
            );
            return true;
        }
        if let Some(alpha) = action.strip_prefix("performance.fps_alpha.") {
            let Some(alpha) = option_fps_alpha_from_slug(alpha) else {
                return false;
            };
            self.app_options.performance.fps_ema_alpha = alpha;
            self.reset_layout_frame_timing();
            self.status_message = format!("FPS smoothing set to {alpha:.2}");
            return true;
        }
        if let Some(interval) = action.strip_prefix("files.autosave_interval.") {
            let Some(interval) = option_interval_from_slug(interval) else {
                return false;
            };
            self.app_options.files.autosave_interval_seconds = interval;
            self.status_message = format!("Autosave interval set to {interval}s");
            return true;
        }
        if let Some(limit) = action.strip_prefix("files.recent_limit.") {
            let Some(limit) = option_recent_limit_from_slug(limit) else {
                return false;
            };
            self.app_options.files.recent_workspace_limit = limit;
            self.app_options.files.recent_files.truncate(limit);
            self.status_message = format!("Recent workspace limit set to {limit}");
            return true;
        }
        match action {
            "file.save" => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Err(error) = self.save_app_options_to_file() {
                        self.status_message = error;
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.status_message = "Options file save is not available on web".to_string();
                }
                true
            }
            "file.reload" => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Err(error) = self.reload_app_options_from_file() {
                        self.status_message = error;
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.status_message = "Options file reload is not available on web".to_string();
                }
                true
            }
            "file.defaults" => {
                self.reset_app_options_to_defaults();
                true
            }
            "performance.live_fps" => {
                self.app_options.performance.live_fps_meter =
                    !self.app_options.performance.live_fps_meter;
                self.reset_layout_frame_timing();
                self.status_message = format!(
                    "Live FPS meter {}",
                    if self.app_options.performance.live_fps_meter {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.idle_redraw" => {
                self.app_options.performance.idle_redraw_layout_viewports =
                    !self.app_options.performance.idle_redraw_layout_viewports;
                self.reset_layout_frame_timing();
                self.status_message = format!(
                    "Layout idle redraw {}",
                    if self.app_options.performance.idle_redraw_layout_viewports {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.cache_layout_index" => {
                self.app_options.performance.cache_layout_index =
                    !self.app_options.performance.cache_layout_index;
                self.layout_index_cache.borrow_mut().take();
                self.status_message = format!(
                    "Layout index cache {}",
                    if self.app_options.performance.cache_layout_index {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.cache_drc" => {
                self.app_options.performance.cache_drc_reports =
                    !self.app_options.performance.cache_drc_reports;
                self.drc_report_cache.borrow_mut().take();
                self.status_message = format!(
                    "DRC cache {}",
                    if self.app_options.performance.cache_drc_reports {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.cache_connectivity" => {
                self.app_options.performance.cache_connectivity_reports =
                    !self.app_options.performance.cache_connectivity_reports;
                self.connectivity_report_cache.borrow_mut().take();
                self.status_message = format!(
                    "Connectivity cache {}",
                    if self.app_options.performance.cache_connectivity_reports {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.dense_2d_lod" => {
                self.app_options.performance.dense_2d_lod =
                    !self.app_options.performance.dense_2d_lod;
                self.status_message = format!(
                    "Dense 2D LOD {}",
                    if self.app_options.performance.dense_2d_lod {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "performance.dense_3d_instancing" => {
                self.app_options.performance.dense_3d_instancing =
                    !self.app_options.performance.dense_3d_instancing;
                self.status_message = format!(
                    "Dense 3D instancing {}",
                    if self.app_options.performance.dense_3d_instancing {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "files.autosave" => {
                self.app_options.files.autosave_enabled = !self.app_options.files.autosave_enabled;
                self.status_message = format!(
                    "Autosave {}",
                    if self.app_options.files.autosave_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "files.remember_workspace" => {
                self.app_options.files.remember_last_workspace =
                    !self.app_options.files.remember_last_workspace;
                self.status_message = format!(
                    "Remember workspace {}",
                    if self.app_options.files.remember_last_workspace {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "files.prompt_destructive" => {
                self.app_options.files.prompt_before_destructive_actions =
                    !self.app_options.files.prompt_before_destructive_actions;
                self.status_message = format!(
                    "Destructive prompts {}",
                    if self.app_options.files.prompt_before_destructive_actions {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            _ => false,
        }
    }

    pub(crate) fn effective_layout_modifiers(
        &self,
        mut modifiers: operad::KeyModifiers,
    ) -> operad::KeyModifiers {
        if !self.app_options.layout.constrain_with_shift {
            modifiers.shift = false;
        }
        if !self.app_options.layout.bypass_snap_with_ctrl {
            modifiers.ctrl = false;
        }
        if !self.app_options.layout.duplicate_drag_with_alt {
            modifiers.alt = false;
        }
        modifiers
    }

    pub(crate) fn select_trace_detail(&mut self, selection: TraceSelection) -> bool {
        if !self.trace_selection_exists(&selection) {
            return false;
        }
        match &selection {
            TraceSelection::Lot(lot_id) => {
                self.selected_trace_lot = Some(lot_id.clone());
                let wafer_still_in_lot =
                    self.selected_trace_wafer.as_ref().is_some_and(|wafer_id| {
                        self.workspace
                            .genealogy
                            .wafer_refs_for_lot(lot_id)
                            .iter()
                            .any(|wafer| &wafer.wafer_id == wafer_id)
                    });
                if !wafer_still_in_lot {
                    self.selected_trace_wafer = default_trace_wafer(&self.workspace, Some(lot_id));
                }
                self.status_message = format!("Traceability lot {lot_id}");
            }
            TraceSelection::Wafer(wafer) => {
                self.selected_trace_lot = Some(wafer.lot_id.clone());
                self.selected_trace_wafer = Some(wafer.wafer_id.clone());
                self.status_message = format!("Traceability wafer {}", wafer.wafer_id);
            }
            TraceSelection::Process(sequence) => {
                if let Some(wafer) = self
                    .workspace
                    .genealogy
                    .process_history
                    .iter()
                    .find(|record| record.sequence == *sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_trace_lot = Some(wafer.lot_id);
                    self.selected_trace_wafer = Some(wafer.wafer_id);
                }
                self.status_message = format!("Traceability process record #{sequence}");
            }
            TraceSelection::MaterialUse(sequence) => {
                if let Some(wafer) = self
                    .workspace
                    .genealogy
                    .material_uses
                    .iter()
                    .find(|record| record.sequence == *sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_trace_lot = Some(wafer.lot_id);
                    self.selected_trace_wafer = Some(wafer.wafer_id);
                }
                self.status_message = format!("Traceability material use #{sequence}");
            }
            TraceSelection::Event(sequence) => {
                self.status_message = format!("Traceability audit event #{sequence}");
            }
        }
        self.selected_trace_detail = Some(selection);
        true
    }
}
