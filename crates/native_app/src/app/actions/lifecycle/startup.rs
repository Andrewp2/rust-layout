#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub fn new_with_options(options: StartupOptions) -> Self {
        let mut app_options = options.app_options.clone().unwrap_or_default().normalized();
        if options.show_options {
            app_options.shell.show_options_panel = true;
        }
        let mut workspace = if let Some(count) = options.stress_count {
            let mut workspace = WorkspaceDataset::demo();
            workspace.document = Document::stress(count);
            workspace
        } else if options.hierarchy_demo || options.hierarchy_workflow_demo {
            let mut workspace = WorkspaceDataset::demo();
            workspace.document = Document::hierarchy_demo();
            workspace
        } else {
            WorkspaceDataset::demo()
        };
        let (layout_active_technology_index, layout_active_technology) =
            layout_builtin_technology_from_slug(&app_options.layout.technology);
        apply_layout_technology_preserving_extra_layers(
            &mut workspace.document,
            &layout_active_technology,
        )
        .expect("built-in layout technology applies to the startup document");
        let layout_disabled_connectivity_links = app_options
            .layout
            .disabled_connectivity_links
            .iter()
            .copied()
            .filter(|index| *index < layout_active_technology.connectivity.len())
            .collect::<BTreeSet<_>>();
        let moved_shape = options
            .move_first_vertex
            .then(|| move_first_layout_shape_vertex(&mut workspace.document))
            .flatten();
        let configured_layer = LayerId(app_options.layout.active_layer);
        let active_layer = workspace
            .document
            .layers
            .contains_key(&configured_layer)
            .then_some(configured_layer)
            .unwrap_or_else(|| default_active_layer(&workspace));
        let selected_layout_shape = moved_shape.or_else(|| {
            options
                .select_first_shape
                .then(|| default_layout_shape(&workspace))
                .flatten()
        });
        let selected_yield_lot = default_yield_lot(&workspace);
        let selected_yield_wafer = default_yield_wafer(&workspace, selected_yield_lot.as_deref());
        let workflow_focus_lot = app_options
            .domains
            .workflow
            .focus_lot
            .as_ref()
            .filter(|lot_id| workflow_lot_ids(&workspace).iter().any(|id| id == *lot_id))
            .cloned()
            .or_else(|| default_workflow_focus_lot(&workspace));
        let selected_equipment_tool = app_options
            .domains
            .fab_control
            .auto_select_first_tool
            .then(|| default_equipment_tool(&workspace))
            .flatten();
        let selected_maintenance_tool = default_maintenance_tool(&workspace);
        let selected_environment_sensor = default_environment_sensor_with_options(
            &workspace,
            app_options.domains.environment.show_alarm_sensors_first,
        );
        let inventory_filter =
            InventoryQuickFilter::from_slug(&app_options.domains.inventory.quick_filter)
                .unwrap_or(InventoryQuickFilter::All);
        let selected_inventory_lot = default_inventory_lot(&workspace, inventory_filter);
        let mask_source_lot = default_mask_lot(&workspace);
        let selected_scheduler_tool = default_scheduler_tool(&workspace);
        let selected_safety_tool = default_safety_tool(&workspace);
        let process_flow_filter =
            ProcessFlowNodeFilter::from_slug(&app_options.domains.process_flow.node_filter)
                .unwrap_or(ProcessFlowNodeFilter::All);
        let selected_process_node =
            default_process_flow_node_for_filter(&workspace, process_flow_filter);
        let process_control_loop_filter =
            ControlLoopFilter::from_slug(&app_options.domains.run_to_run.loop_filter)
                .unwrap_or(ControlLoopFilter::All);
        let selected_control_loop =
            default_control_loop_for_filter(&workspace, process_control_loop_filter);
        let selected_control_action =
            default_control_action(&workspace, selected_control_loop.as_ref());
        let selected_spc_chart = default_spc_chart(&workspace);
        let selected_fdc_trace = default_fdc_trace(&workspace);
        let selected_cross_section_material = default_cross_section_material(&workspace);
        let selected_trace_lot = default_trace_lot(&workspace);
        let selected_trace_wafer = default_trace_wafer(&workspace, selected_trace_lot.as_ref());
        let selected_trace_detail = default_trace_selection(
            &workspace,
            selected_trace_lot.as_ref(),
            selected_trace_wafer.as_ref(),
        );
        let selected_notebook_entry = default_notebook_entry(&workspace);
        let selected_experiment_run = default_experiment_run(&workspace);
        let selected_experiment_response = default_experiment_response(&workspace);

        let active_view = options
            .view_mode
            .or_else(|| options.view_3d.then_some(StartupView::Layout3d))
            .or_else(|| StartupView::from_slug(&app_options.shell.startup_view))
            .unwrap_or(StartupView::Workflow);
        let active_tool =
            ToolMode::from_slug(&app_options.layout.default_tool).unwrap_or(ToolMode::Select);
        let layout_hierarchy_depth =
            LayoutHierarchyDepth::from_slug(&app_options.layout.hierarchy_depth)
                .unwrap_or(LayoutHierarchyDepth::Full);
        let layout_hierarchy_min_depth = clamp_layout_hierarchy_min_depth_for_max(
            app_options.layout.hierarchy_min_depth,
            layout_hierarchy_depth,
        );
        let unit_display = UnitDisplay::from_slug(&app_options.appearance.unit_display)
            .unwrap_or(UnitDisplay::Auto);
        let theme_preference = app_options.appearance.theme;
        let nav_rail_views = nav_rail_views_from_option_slugs(&app_options.shell.nav_rail_views);
        let mut camera_3d = Camera3d::default();
        camera_3d.speed = app_options.viewport3d.movement_speed;
        camera_3d.fov_y = app_options.viewport3d.fov_degrees.to_radians();
        let cross_section_step = app_options
            .domains
            .cross_section
            .step
            .min(workspace.cross_section.steps.len());
        let layout_view_top_cell = workspace.document.top_cell;
        let layout_view_bookmarks =
            layout_view_bookmarks_from_options(&app_options.layout.view_bookmarks);
        let layout_view_bookmark_names =
            layout_view_bookmark_names_from_options(&app_options.layout.view_bookmarks);
        let layout_layer_sets =
            layout_layer_sets_from_options(&workspace.document, &app_options.layout.layer_sets);
        let layout_layer_set_names =
            layout_layer_set_names_from_options(&app_options.layout.layer_sets);
        let layout_layer_group_filter =
            LayoutLayerGroupFilter::from_slug(&app_options.layout.layer_group_filter)
                .unwrap_or(LayoutLayerGroupFilter::All);
        let layout_layer_usage_filter =
            LayoutLayerUsageFilter::from_slug(&app_options.layout.layer_usage_filter)
                .unwrap_or(LayoutLayerUsageFilter::All);
        let layout_layer_depth_overrides = layout_layer_depth_overrides_from_options(
            &workspace.document,
            &app_options.layout.layer_depths,
        );
        let layout_measurement_mode =
            MeasurementMode::from_slug(&app_options.layout.measurement_mode)
                .unwrap_or(MeasurementMode::Direct);
        let layout_library_via_array_presets =
            layout_via_array_library_presets_from_options(&app_options.layout.library_presets);

        let mut app = Self {
            workspace,
            active_view,
            active_menu: None,
            active_view_group: None,
            active_tool,
            layout_zoom: options
                .zoom
                .unwrap_or(app_options.layout.zoom)
                .clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM),
            layout_pan: options.pan.unwrap_or(app_options.layout.pan),
            layout_pointer: None,
            layout_canvas_size: None,
            layout_drag: None,
            layout_3d_drag: None,
            camera_3d,
            flycam_captured: false,
            viewport_fullscreen: app_options.viewport3d.fullscreen_on_start,
            drawing_start: None,
            drawing_points: Vec::new(),
            measure_start: None,
            route_points: Vec::new(),
            layout_frame_ms: StdCell::new(None),
            layout_frame_ema_ms: StdCell::new(None),
            layout_frame_last_at: StdCell::new(None),
            layout_revision: 0,
            layout_view_revision: 0,
            layout_view_top_cell,
            layout_hierarchy_depth,
            layout_hierarchy_min_depth,
            layout_previous_view: None,
            layout_view_bookmarks,
            layout_view_bookmark_names,
            layout_layer_sets,
            layout_layer_set_names,
            layout_layer_group_filter,
            layout_layer_usage_filter,
            layout_layer_depth_overrides,
            layout_measurement_mode,
            layout_measurement_filter: LayoutMeasurementFilter::All,
            layout_measurement_browser_sort: LayoutMeasurementBrowserSort::Id,
            layout_active_technology_index,
            layout_active_technology,
            layout_disabled_connectivity_links,
            layout_library_via_array_presets,
            layout_library_via_array_columns: LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_COLUMNS,
            layout_library_via_array_rows: LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_ROWS,
            layout_library_via_array_size_grids: LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_SIZE_GRIDS,
            layout_library_via_array_pitch_grids: LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_PITCH_GRIDS,
            layout_import_offset: Vector::ZERO,
            layout_import_layer_policy: LayoutImportLayerPolicy::Id,
            layout_import_layer_id_offset: 0,
            layout_tree_collapsed_cells: BTreeSet::new(),
            layout_hidden_cells: BTreeSet::new(),
            layout_index_cache: RefCell::new(None),
            connectivity_report_cache: RefCell::new(None),
            layout_spice_comparison: None,
            drc_report_cache: RefCell::new(None),
            layout_drc_report_history: Vec::new(),
            layout_selected_drc_report_id: None,
            layout_next_drc_report_id: 1,
            layout_custom_drc_deck: None,
            layout_custom_drc_deck_label: None,
            nav_rail_views,
            active_layer,
            selected_layout_shape,
            selected_layout_occurrence: selected_layout_shape.map(ShapeOccurrenceId::top_level),
            layout_shape_browser_filter: LayoutShapeBrowserFilter::All,
            layout_shape_browser_sort: LayoutShapeBrowserSort::Id,
            layout_cell_browser_filter: LayoutCellBrowserFilter::All,
            layout_cell_browser_sort: LayoutCellBrowserSort::Name,
            layout_net_browser_filter: LayoutNetBrowserFilter::All,
            layout_net_browser_sort: LayoutNetBrowserSort::Name,
            layout_trace_highlight_mode: LayoutTraceHighlightMode::Selected,
            layout_instance_browser_scope: LayoutInstanceBrowserScope::CurrentCell,
            layout_instance_browser_filter: LayoutInstanceBrowserFilter::All,
            layout_instance_browser_sort: LayoutInstanceBrowserSort::Id,
            layout_browser_search: String::new(),
            layout_browser_search_active: false,
            layout_browser_replace: String::new(),
            layout_browser_replace_active: false,
            layout_browser_replace_scope: LayoutBrowserReplaceScope::Document,
            layout_browser_columns: LayoutBrowserColumnSet::All,
            layout_drc_marker_filter: LayoutDrcMarkerFilter::Active,
            layout_drc_marker_sort: LayoutDrcMarkerSort::Id,
            layout_drc_marker_category_filter: None,
            layout_drc_marker_directory_filter: None,
            layout_selected_drc_marker_key: None,
            layout_trace_history: Vec::new(),
            layout_clipboard_shapes: Vec::new(),
            layout_undo: Vec::new(),
            layout_redo: Vec::new(),
            workflow_focus_lot,
            selected_equipment_tool,
            equipment_recipe_drafts: BTreeMap::new(),
            selected_maintenance_tool,
            maintenance_work_filter: MaintenanceWorkFilter::from_slug(
                &app_options.domains.maintenance.work_filter,
            )
            .unwrap_or(MaintenanceWorkFilter::Actionable),
            maintenance_history_filter: MaintenanceHistoryFilter::from_slug(
                &app_options.domains.maintenance.history_filter,
            )
            .unwrap_or(MaintenanceHistoryFilter::SelectedTool),
            selected_environment_sensor,
            selected_inventory_lot,
            inventory_filter,
            mask_source_lot,
            mask_issue_severity_filter: MaskIssueSeverityFilter::from_slug(
                &app_options.domains.mask_prep.severity_filter,
            )
            .unwrap_or(MaskIssueSeverityFilter::All),
            mask_issue_grouping: MaskIssueGrouping::from_slug(
                &app_options.domains.mask_prep.grouping,
            )
            .unwrap_or(MaskIssueGrouping::Code),
            mask_issue_page: 0,
            layout_diff_baseline: LayoutDiffSource::Demo,
            layout_diff_candidate: LayoutDiffSource::Current,
            layout_diff_changed_only: app_options.domains.layout_diff.changed_only,
            layout_diff_change_filter: LayoutChangeFilter::All,
            layout_diff_change_page: 0,
            layout_diff_page_size: app_options.domains.layout_diff.page_size,
            layout_diff_review_state: LayoutReviewDisposition::NeedsReview,
            selected_trace_lot,
            selected_trace_wafer,
            selected_trace_detail,
            trace_impact_mode: TraceImpactMode::from_slug(
                &app_options.domains.traceability.impact_mode,
            )
            .unwrap_or(TraceImpactMode::ToolRun),
            trace_related_only: app_options.domains.traceability.related_only,
            selected_notebook_entry,
            notebook_tag_filter: app_options.domains.notebook.tag_filter.clone(),
            notebook_link_kind_filter: None,
            notebook_preview_mode: app_options.domains.notebook.preview_mode,
            notebook_followups_only: app_options.domains.notebook.followups_only,
            scheduler_policy: dispatch_policy_from_slug(
                &app_options.domains.scheduler.dispatch_policy,
            )
            .unwrap_or(DispatchPolicy::PriorityThenFifo),
            selected_scheduler_tool,
            scheduler_min_priority: app_options.domains.scheduler.minimum_priority,
            scheduler_conflicts_only: app_options.domains.scheduler.conflicts_only,
            scheduler_focus_selected_tool: app_options.domains.scheduler.focus_selected_tool,
            selected_safety_tool,
            acknowledged_conditions: BTreeSet::new(),
            acknowledged_lockouts: BTreeSet::new(),
            acknowledged_incidents: BTreeSet::new(),
            selected_process_node,
            process_flow_filter,
            process_flow_errors_only: app_options.domains.process_flow.errors_only,
            process_control_loop_filter,
            selected_control_loop,
            selected_control_action,
            selected_spc_chart,
            selected_fdc_trace,
            spc_severity_filter: SpcSeverityFilter::from_slug(
                &app_options.domains.spc_fdc.severity_filter,
            )
            .unwrap_or(SpcSeverityFilter::All),
            spc_source_filter: SpcSourceFilter::from_slug(
                &app_options.domains.spc_fdc.source_filter,
            )
            .unwrap_or(SpcSourceFilter::All),
            spc_context_filter: app_options.domains.spc_fdc.context_filter.clone(),
            cross_section_step,
            selected_cross_section_material,
            cross_section_show_mask: app_options.domains.cross_section.show_mask,
            cross_section_show_dimensions: app_options.domains.cross_section.show_dimensions,
            cross_section_show_risks: app_options.domains.cross_section.show_risks,
            metrology_map_mode: MetrologyMapMode::from_slug(
                &app_options.domains.metrology.map_mode,
            )
            .unwrap_or(MetrologyMapMode::ValueMap),
            metrology_kind: measurement_kind_from_slug(
                &app_options.domains.metrology.measurement_kind,
            )
            .unwrap_or(MeasurementKind::CriticalDimensionNm),
            metrology_failed_only: app_options.domains.metrology.failed_only,
            selected_die: None,
            yield_map_filter: YieldMapFilter::from_slug(
                &app_options.domains.yield_dashboard.map_filter,
            )
            .unwrap_or(YieldMapFilter::All),
            show_only_attention_wafers: app_options.domains.yield_dashboard.attention_only,
            show_only_excursions: app_options.domains.yield_dashboard.excursions_only,
            selected_yield_lot,
            selected_yield_wafer,
            selected_experiment_run,
            selected_experiment_response,
            experiment_capture_value: app_options.domains.experiment.capture_value,
            experiment_run_filter: ExperimentRunFilter::from_slug(
                &app_options.domains.experiment.run_filter,
            )
            .unwrap_or(ExperimentRunFilter::All),
            experiment_lot_filter: None,
            experiment_show_missing_only: app_options.domains.experiment.show_missing_only,
            status_message: "Ready".to_string(),
            dark_theme: theme_preference != options::ThemePreference::Light,
            theme_preference,
            show_command_palette: app_options.shell.show_command_palette_on_start,
            show_sidebar_modules: app_options.shell.show_sidebar_modules,
            show_options_panel: app_options.shell.show_options_panel,
            show_diagnostics_panel: app_options.shell.show_diagnostics_panel,
            collapsed_detail_sections: BTreeSet::new(),
            unit_display,
            snap_enabled: app_options.layout.snap_enabled,
            show_grid: app_options.layout.show_2d_grid,
            show_3d_grid: app_options.viewport3d.show_grid,
            show_origin_marker: app_options.layout.show_origin_marker,
            show_drc_overlay: app_options.layout.show_drc_overlay,
            show_reference_images: app_options.layout.show_reference_images,
            show_inspector: app_options.shell.show_details_panel,
            show_layers: app_options.shell.show_secondary_panel,
            app_options,
            options_file_path: options.options_file_path,
            layout_modifiers: operad::KeyModifiers::NONE,
        };
        for action in options.startup_actions {
            app.apply_clicked_node_name(&action);
        }
        app.sync_app_options_from_state();
        app
    }

    pub fn workspace(&self) -> &WorkspaceDataset {
        &self.workspace
    }

    pub fn active_view(&self) -> StartupView {
        self.active_view
    }

    pub fn active_menu(&self) -> Option<AppMenu> {
        self.active_menu
    }

    pub fn flycam_captured(&self) -> bool {
        self.flycam_captured
    }

    pub fn layout_browser_search_active(&self) -> bool {
        self.layout_browser_search_active
    }

    pub fn layout_browser_replace_active(&self) -> bool {
        self.layout_browser_replace_active
    }

    pub fn layout_browser_search(&self) -> &str {
        &self.layout_browser_search
    }

    pub fn layout_browser_replace(&self) -> &str {
        &self.layout_browser_replace
    }

    pub fn begin_layout_browser_search(&mut self) -> bool {
        self.layout_browser_search_active = true;
        self.layout_browser_replace_active = false;
        self.status_message = if self.layout_browser_search.trim().is_empty() {
            "Browser search ready".to_string()
        } else {
            format!(
                "Browser search {}",
                compact_button_label(self.layout_browser_search.trim(), 32)
            )
        };
        true
    }

    pub fn begin_layout_browser_replace(&mut self) -> bool {
        self.layout_browser_replace_active = true;
        self.layout_browser_search_active = false;
        self.status_message = if self.layout_browser_replace.trim().is_empty() {
            "Browser replace ready".to_string()
        } else {
            format!(
                "Browser replace {}",
                compact_button_label(self.layout_browser_replace.trim(), 32)
            )
        };
        true
    }

    pub fn handle_layout_browser_search_key(
        &mut self,
        key: operad::KeyCode,
        modifiers: operad::KeyModifiers,
    ) -> bool {
        if !self.layout_browser_search_active {
            return false;
        }
        match key {
            operad::KeyCode::Escape => {
                self.layout_browser_search_active = false;
                self.status_message = "Browser search closed".to_string();
                true
            }
            operad::KeyCode::Enter => {
                self.layout_browser_search_active = false;
                self.status_message = if self.layout_browser_search.trim().is_empty() {
                    "Browser search empty".to_string()
                } else {
                    format!(
                        "Browser search {}",
                        compact_button_label(self.layout_browser_search.trim(), 32)
                    )
                };
                true
            }
            operad::KeyCode::Backspace
                if !layout_browser_search_has_command_modifier(modifiers) =>
            {
                self.layout_browser_search.pop();
                self.status_message = layout_browser_search_status(&self.layout_browser_search);
                true
            }
            operad::KeyCode::Delete if !layout_browser_search_has_command_modifier(modifiers) => {
                true
            }
            operad::KeyCode::Character(character)
                if !layout_browser_search_has_command_modifier(modifiers)
                    && layout_browser_search_accepts_character(character) =>
            {
                push_layout_browser_search_character(&mut self.layout_browser_search, character);
                self.status_message = layout_browser_search_status(&self.layout_browser_search);
                true
            }
            _ => false,
        }
    }

    pub fn handle_layout_browser_replace_key(
        &mut self,
        key: operad::KeyCode,
        modifiers: operad::KeyModifiers,
    ) -> bool {
        if !self.layout_browser_replace_active {
            return false;
        }
        match key {
            operad::KeyCode::Escape => {
                self.layout_browser_replace_active = false;
                self.status_message = "Browser replace closed".to_string();
                true
            }
            operad::KeyCode::Enter => {
                self.layout_browser_replace_active = false;
                self.status_message = layout_browser_replace_status(&self.layout_browser_replace);
                true
            }
            operad::KeyCode::Backspace
                if !layout_browser_search_has_command_modifier(modifiers) =>
            {
                self.layout_browser_replace.pop();
                self.status_message = layout_browser_replace_status(&self.layout_browser_replace);
                true
            }
            operad::KeyCode::Delete if !layout_browser_search_has_command_modifier(modifiers) => {
                true
            }
            operad::KeyCode::Character(character)
                if !layout_browser_search_has_command_modifier(modifiers)
                    && layout_browser_search_accepts_character(character) =>
            {
                push_layout_browser_search_character(&mut self.layout_browser_replace, character);
                self.status_message = layout_browser_replace_status(&self.layout_browser_replace);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn set_layout_browser_search(&mut self, query: impl Into<String>) {
        self.layout_browser_search = sanitize_layout_browser_search(query.into());
        self.status_message = layout_browser_search_status(&self.layout_browser_search);
    }

    pub(crate) fn set_layout_browser_replace(&mut self, replacement: impl Into<String>) {
        self.layout_browser_replace = sanitize_layout_browser_search(replacement.into());
        self.status_message = layout_browser_replace_status(&self.layout_browser_replace);
    }

    pub(crate) fn clear_layout_browser_search(&mut self) -> bool {
        if self.layout_browser_search.is_empty() && !self.layout_browser_search_active {
            return false;
        }
        self.layout_browser_search.clear();
        self.layout_browser_search_active = false;
        self.layout_browser_replace_active = false;
        self.status_message = "Browser search cleared".to_string();
        true
    }

    pub fn dismiss_transient_ui(&mut self) -> bool {
        if self.layout_browser_search_active {
            self.layout_browser_search_active = false;
            return true;
        }
        if self.layout_browser_replace_active {
            self.layout_browser_replace_active = false;
            return true;
        }
        if self.active_menu.is_some() {
            self.active_menu = None;
            self.active_view_group = None;
            return true;
        }
        if self.show_command_palette {
            self.show_command_palette = false;
            return true;
        }
        if self.show_sidebar_modules {
            self.show_sidebar_modules = false;
            return true;
        }
        if self.show_options_panel {
            self.show_options_panel = false;
            return true;
        }
        if self.show_diagnostics_panel {
            self.show_diagnostics_panel = false;
            return true;
        }
        false
    }

    pub fn active_tool(&self) -> ToolMode {
        self.active_tool
    }

    pub fn layout_zoom(&self) -> f32 {
        self.layout_zoom
    }

    pub fn layout_pan(&self) -> [f32; 2] {
        self.layout_pan
    }

    pub fn active_layer(&self) -> LayerId {
        self.active_layer
    }

    pub fn selected_layout_shape(&self) -> Option<ShapeId> {
        self.selected_layout_shape
    }

    pub(crate) fn current_layout_view_state(&self) -> LayoutViewState {
        LayoutViewState {
            zoom: self.layout_zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM),
            pan: self.layout_pan,
            top_cell: self.layout_view_top_cell,
            hierarchy_depth: self.layout_hierarchy_depth,
            hierarchy_min_depth: self.layout_hierarchy_min_depth,
        }
    }

    pub(crate) fn record_layout_previous_view(&mut self) {
        let current = self.current_layout_view_state();
        if self.layout_previous_view != Some(current) {
            self.layout_previous_view = Some(current);
        }
    }

    pub(crate) fn layout_view_state_is_valid(&self, state: LayoutViewState) -> bool {
        state.zoom.is_finite() && state.pan[0].is_finite() && state.pan[1].is_finite()
    }

    pub(crate) fn apply_layout_view_state(&mut self, state: LayoutViewState) {
        let top_cell = if self.workspace.document.cell(state.top_cell).is_some() {
            state.top_cell
        } else {
            self.workspace.document.top_cell
        };
        let root_or_depth_changed = self.layout_view_top_cell != top_cell
            || self.layout_hierarchy_depth != state.hierarchy_depth
            || self.layout_hierarchy_min_depth != state.hierarchy_min_depth;
        self.layout_zoom = state.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
        self.layout_pan = if state.pan[0].is_finite() && state.pan[1].is_finite() {
            state.pan
        } else {
            [0.0, 0.0]
        };
        self.layout_view_top_cell = top_cell;
        self.layout_hierarchy_depth = state.hierarchy_depth;
        self.layout_hierarchy_min_depth = clamp_layout_hierarchy_min_depth_for_max(
            state.hierarchy_min_depth,
            state.hierarchy_depth,
        );
        self.layout_hidden_cells.remove(&top_cell);
        if root_or_depth_changed {
            self.invalidate_layout_view_caches();
            self.repair_layout_selection_after_operation();
        } else {
            self.reset_layout_frame_timing();
        }
    }

    pub(crate) fn restore_layout_view_state(
        &mut self,
        state: LayoutViewState,
        label: &str,
    ) -> bool {
        if !self.layout_view_state_is_valid(state) {
            self.status_message = "Saved layout view is no longer valid".to_string();
            return true;
        }
        let current = self.current_layout_view_state();
        if current != state {
            self.layout_previous_view = Some(current);
            self.apply_layout_view_state(state);
        }
        self.status_message = label.to_string();
        true
    }

    pub(crate) fn save_layout_view_bookmark(&mut self, slot: u8) -> bool {
        if !LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&slot) {
            return false;
        }
        let state = self.current_layout_view_state();
        let name = layout_view_bookmark_generated_name(&self.workspace.document, slot, state);
        self.layout_view_bookmarks.insert(slot, state);
        self.layout_view_bookmark_names.insert(slot, name.clone());
        self.status_message = format!("Saved layout view bookmark {slot}: {name}");
        true
    }

    pub(crate) fn restore_layout_view_bookmark(&mut self, slot: u8) -> bool {
        if !LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&slot) {
            return false;
        }
        let Some(bookmark) = self.layout_view_bookmarks.get(&slot).copied() else {
            self.status_message = format!("No layout view bookmark {slot} saved");
            return true;
        };
        let name = self.layout_view_bookmark_name(slot);
        self.restore_layout_view_state(bookmark, &format!("Restored layout view bookmark {name}"))
    }

    pub(crate) fn clear_layout_view_bookmark(&mut self, slot: u8) -> bool {
        if !LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&slot) {
            return false;
        }
        let removed = self.layout_view_bookmarks.remove(&slot).is_some();
        self.layout_view_bookmark_names.remove(&slot);
        self.status_message = if removed {
            format!("Cleared layout view bookmark {slot}")
        } else {
            format!("No layout view bookmark {slot} saved")
        };
        true
    }

    pub(crate) fn clear_all_layout_view_bookmarks(&mut self) -> bool {
        let count = self.layout_view_bookmarks.len();
        if count == 0 {
            self.layout_view_bookmark_names.clear();
            self.status_message = "No layout view bookmarks saved".to_string();
            return true;
        }
        self.layout_view_bookmarks.clear();
        self.layout_view_bookmark_names.clear();
        self.status_message = format!(
            "Cleared {count} layout view bookmark{}",
            if count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn export_layout_view_bookmarks(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_view_bookmark_exchange_path();
            return self.export_layout_view_bookmarks_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "View bookmark export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_view_bookmarks_to_path(&mut self, path: &Path) -> bool {
        let exchange = LayoutViewBookmarkExchange::from_app(self);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("View bookmark export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("View bookmark export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported view bookmarks {} ({} bookmark(s))",
            path.display(),
            exchange.bookmarks.len()
        );
        true
    }

    pub(crate) fn import_layout_view_bookmarks(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_view_bookmark_exchange_path();
            return self.import_layout_view_bookmarks_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "View bookmark import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_view_bookmarks_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("View bookmark import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutViewBookmarkExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("View bookmark import failed: {error}");
                return true;
            }
        };
        let document_name = exchange.document_name.clone();
        let (bookmarks, bookmark_names) = match exchange.into_bookmarks(&self.workspace.document) {
            Ok(bookmarks) => bookmarks,
            Err(error) => {
                self.status_message = format!("View bookmark import failed: {error}");
                return true;
            }
        };
        let imported_count = bookmarks.len();
        self.layout_view_bookmarks = bookmarks;
        self.layout_view_bookmark_names = bookmark_names;
        self.sync_app_options_from_state();
        self.status_message = format!(
            "Imported view bookmarks {} from {} ({} bookmark(s))",
            path.display(),
            document_name,
            imported_count
        );
        true
    }

    pub(crate) fn layout_view_bookmark_name(&self, slot: u8) -> String {
        self.layout_view_bookmark_names
            .get(&slot)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| layout_view_bookmark_default_name(slot))
    }

    pub(crate) fn current_visible_layout_layers(&self) -> BTreeSet<LayerId> {
        self.workspace
            .document
            .layers
            .values()
            .filter(|layer| layer.visible)
            .map(|layer| layer.id)
            .collect()
    }

    pub(crate) fn current_layout_layer_set_state(&self) -> LayoutLayerSetState {
        LayoutLayerSetState {
            visible_layers: self.current_visible_layout_layers(),
            layer_group_filter: self.layout_layer_group_filter,
            layer_usage_filter: self.layout_layer_usage_filter,
            layer_depth_overrides: self.layout_layer_depth_overrides.clone(),
        }
    }

    pub(crate) fn layout_layer_set_name(&self, slot: u8) -> String {
        self.layout_layer_set_names
            .get(&slot)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| layout_layer_set_default_name(slot))
    }

    pub(crate) fn save_layout_layer_set(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_layer_set_slot(value) else {
            return false;
        };
        let state = self.current_layout_layer_set_state();
        let name = self.layout_layer_set_name(slot);
        let visible_count = state.visible_layers.len();
        let layer_group = state.layer_group_filter.path_label();
        let usage_filter = state.layer_usage_filter.label();
        let depth_count = state.layer_depth_overrides.len();
        self.layout_layer_sets.insert(slot, state);
        self.layout_layer_set_names.insert(slot, name.clone());
        self.status_message = format!(
            "Saved {name} with {visible_count} visible layer(s), {layer_group} group, {usage_filter} rows, {depth_count} layer depth override(s)"
        );
        true
    }

    pub(crate) fn clear_layout_layer_set(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_layer_set_slot(value) else {
            return false;
        };
        let name = self.layout_layer_set_name(slot);
        let removed_set = self.layout_layer_sets.remove(&slot).is_some();
        let removed_name = self.layout_layer_set_names.remove(&slot).is_some();
        if removed_set || removed_name {
            self.status_message = format!("Cleared {name}");
        } else {
            self.status_message = format!("No {name} saved");
        }
        true
    }

    pub(crate) fn clear_all_layout_layer_sets(&mut self) -> bool {
        let count = self.layout_layer_sets.len();
        if count == 0 {
            self.layout_layer_set_names.clear();
            self.status_message = "No layer sets saved".to_string();
            return true;
        }
        self.layout_layer_sets.clear();
        self.layout_layer_set_names.clear();
        self.status_message = format!(
            "Cleared {count} layer set{}",
            if count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn restore_layout_layer_set(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_layer_set_slot(value) else {
            return false;
        };
        let Some(state) = self.layout_layer_sets.get(&slot).cloned() else {
            self.status_message = format!("No {} saved", self.layout_layer_set_name(slot));
            return true;
        };
        let name = self.layout_layer_set_name(slot);

        let mut operations = Vec::new();
        let mut undo_operations = Vec::new();
        let mut layers = self.workspace.document.layers.values().collect::<Vec<_>>();
        layers.sort_by_key(|layer| (layer.display_order, layer.id));
        for layer in layers {
            let visible = state.visible_layers.contains(&layer.id);
            if layer.visible == visible {
                continue;
            }
            operations.push(Operation::SetLayerVisibility {
                layer: layer.id,
                visible,
            });
            undo_operations.push(Operation::SetLayerVisibility {
                layer: layer.id,
                visible: layer.visible,
            });
        }

        let next_layer_depth_overrides = state
            .layer_depth_overrides
            .iter()
            .filter_map(|(layer, depth)| {
                self.workspace
                    .document
                    .layers
                    .contains_key(layer)
                    .then_some((*layer, *depth))
            })
            .collect::<BTreeMap<_, _>>();
        let group_changed = self.layout_layer_group_filter != state.layer_group_filter;
        let usage_changed = self.layout_layer_usage_filter != state.layer_usage_filter;
        let depth_changed = self.layout_layer_depth_overrides != next_layer_depth_overrides;
        if operations.is_empty() && !group_changed && !usage_changed && !depth_changed {
            self.status_message = format!("{name} already active");
            return true;
        }
        let changed_count = operations.len();
        if !operations.is_empty() {
            self.apply_layout_operation_with_history(
                Operation::Batch { operations },
                Operation::Batch {
                    operations: undo_operations.into_iter().rev().collect(),
                },
            );
        }
        if group_changed {
            self.layout_layer_group_filter = state.layer_group_filter;
        }
        if usage_changed {
            self.layout_layer_usage_filter = state.layer_usage_filter;
        }
        if depth_changed {
            self.layout_layer_depth_overrides = next_layer_depth_overrides;
            self.invalidate_layout_view_caches();
            self.repair_layout_selection_after_operation();
        }
        let mut restored_parts = Vec::new();
        if changed_count > 0 {
            restored_parts.push(format!("{changed_count} layer(s)"));
        }
        if group_changed {
            restored_parts.push("layer group".to_string());
        }
        if usage_changed {
            restored_parts.push("layer row filter".to_string());
        }
        if depth_changed {
            restored_parts.push("layer depths".to_string());
        }
        self.status_message = format!("Restored {name}; changed {}", restored_parts.join(", "));
        true
    }

    pub(crate) fn export_layout_layer_sets(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layer_set_exchange_path();
            return self.export_layout_layer_sets_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Layer set export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_layer_sets_to_path(&mut self, path: &Path) -> bool {
        let exchange = LayoutLayerSetExchange::from_app(self);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Layer set export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Layer set export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported layer sets {} ({} set(s))",
            path.display(),
            exchange.layer_sets.len()
        );
        true
    }

    pub(crate) fn import_layout_layer_sets(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_layer_set_exchange_path();
            return self.import_layout_layer_sets_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Layer set import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_layer_sets_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Layer set import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutLayerSetExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Layer set import failed: {error}");
                return true;
            }
        };
        let document_name = exchange.document_name.clone();
        let (layer_sets, layer_set_names) = match exchange.into_layer_sets(&self.workspace.document)
        {
            Ok(layer_sets) => layer_sets,
            Err(error) => {
                self.status_message = format!("Layer set import failed: {error}");
                return true;
            }
        };
        let imported_count = layer_sets.len();
        self.layout_layer_sets = layer_sets;
        self.layout_layer_set_names = layer_set_names;
        self.sync_app_options_from_state();
        self.status_message = format!(
            "Imported layer sets {} from {} ({} set(s))",
            path.display(),
            document_name,
            imported_count
        );
        true
    }

    pub(crate) fn restore_previous_layout_view(&mut self) -> bool {
        let Some(previous) = self.layout_previous_view else {
            self.status_message = "No previous layout view".to_string();
            return true;
        };
        if !self.layout_view_state_is_valid(previous) {
            self.layout_previous_view = None;
            self.status_message = "Previous layout view is no longer valid".to_string();
            return true;
        }
        let current = self.current_layout_view_state();
        if current != previous {
            self.apply_layout_view_state(previous);
            self.layout_previous_view = Some(current);
        }
        self.status_message = "Restored previous layout view".to_string();
        true
    }

    pub(crate) fn focus_layout_origin(&mut self) -> bool {
        let mut state = self.current_layout_view_state();
        state.pan = [0.0, 0.0];
        self.restore_layout_view_state(state, "Focused layout origin")
    }

    pub(crate) fn focus_layout_bounds(&mut self) -> bool {
        let bounds = self.with_layout_display_index(|index| index.bounds());
        let size = self
            .layout_canvas_size
            .unwrap_or_else(|| UiSize::new(1_000.0, 700.0));
        let bounds =
            bounds.unwrap_or_else(|| Rect::from_min_size(Point::new(-500, -500), 1_000, 1_000));
        let width = bounds.width().max(1) as f32;
        let height = bounds.height().max(1) as f32;
        let zoom = (size.width.max(1.0) / (width * 1.12))
            .min(size.height.max(1.0) / (height * 1.12))
            .clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
        let center = bounds.center();
        let mut state = self.current_layout_view_state();
        state.zoom = zoom;
        state.pan = [-(center.x as f32) * zoom, center.y as f32 * zoom];
        self.restore_layout_view_state(state, "Focused layout bounds")
    }

    pub(crate) fn focus_layout_selection(&mut self) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.clone() else {
            self.status_message = "No layout selection to focus".to_string();
            return true;
        };
        let Some((bounds, layer)) = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
            .map(|view| (view.bounds, view.shape.layer))
        else {
            self.status_message = "Selected layout occurrence is no longer visible".to_string();
            return true;
        };
        self.ensure_layout_occurrence_depth_visible(&occurrence);
        self.selected_layout_shape = Some(occurrence.source_shape_id());
        self.selected_layout_occurrence = Some(occurrence.clone());
        self.active_layer = layer;
        self.focus_layout_rect(
            bounds,
            &format!(
                "Focused selected {}",
                compact_button_label(&layout_occurrence_label(&occurrence), 48)
            ),
        )
    }

    pub(crate) fn focus_layout_rect(&mut self, bounds: Rect, label: &str) -> bool {
        let size = self
            .layout_canvas_size
            .unwrap_or_else(|| UiSize::new(1_000.0, 700.0));
        let width = bounds.width().max(1) as f32;
        let height = bounds.height().max(1) as f32;
        let zoom = (size.width.max(1.0) / (width * 1.12))
            .min(size.height.max(1.0) / (height * 1.12))
            .clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
        let center = bounds.center();
        let mut state = self.current_layout_view_state();
        state.zoom = zoom;
        state.pan = [-(center.x as f32) * zoom, center.y as f32 * zoom];
        self.restore_layout_view_state(state, label)
    }

    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    pub(crate) fn record_layout_frame_sample(
        &self,
        frame_started: std::time::Instant,
        fallback_ms: Option<f64>,
    ) {
        let frame_ms = self
            .layout_frame_last_at
            .get()
            .map(|previous| frame_started.duration_since(previous).as_secs_f64() * 1000.0)
            .filter(|value| value.is_finite() && *value > 0.0)
            .or_else(|| fallback_ms.filter(|value| value.is_finite() && *value > 0.0));
        self.layout_frame_last_at.set(Some(frame_started));
        let Some(frame_ms) = frame_ms else {
            return;
        };
        let alpha = if self.app_options.performance.fps_ema_alpha.is_finite() {
            self.app_options.performance.fps_ema_alpha.clamp(0.01, 1.0)
        } else {
            LAYOUT_FPS_EMA_ALPHA
        };
        let smoothed = self
            .layout_frame_ema_ms
            .get()
            .map(|previous| previous + (frame_ms - previous) * alpha)
            .unwrap_or(frame_ms);
        self.layout_frame_ms.set(Some(frame_ms));
        self.layout_frame_ema_ms.set(Some(smoothed));
    }

    #[cfg(test)]
    pub(crate) fn record_layout_frame_timing(
        &self,
        frame_started: std::time::Instant,
        render_ms: f64,
    ) {
        self.record_layout_frame_sample(frame_started, Some(render_ms));
    }

    pub fn record_layout_frame_tick(&self, frame_started: std::time::Instant) {
        self.record_layout_frame_sample(frame_started, None);
    }

    pub(crate) fn reset_layout_frame_timing(&self) {
        self.layout_frame_ms.set(None);
        self.layout_frame_ema_ms.set(None);
        self.layout_frame_last_at.set(None);
    }

    pub fn layout_fps_frame_ms(&self) -> Option<f64> {
        self.layout_frame_ema_ms
            .get()
            .or_else(|| self.layout_frame_ms.get())
    }

    pub fn app_options(&self) -> &AppOptions {
        &self.app_options
    }

    pub fn options_file_path(&self) -> Option<&str> {
        self.options_file_path.as_deref()
    }

    pub fn live_layout_fps_meter(&self) -> bool {
        self.app_options.performance.live_fps_meter
            && self.app_options.performance.idle_redraw_layout_viewports
    }

    pub(crate) fn options_file_path_for_display(&self) -> String {
        self.options_file_path
            .clone()
            .unwrap_or_else(|| "default config path".to_string())
    }

    pub(crate) fn sync_app_options_from_state(&mut self) {
        self.app_options = self.app_options_from_state();
    }

    pub(crate) fn active_layout_technology(&self) -> &TechnologyFile {
        &self.layout_active_technology
    }

    pub(crate) fn active_connectivity_technology(&self) -> TechnologyFile {
        let mut technology = self.layout_active_technology.clone();
        if !self.layout_disabled_connectivity_links.is_empty() {
            technology.connectivity = technology
                .connectivity
                .into_iter()
                .enumerate()
                .filter_map(|(index, connection)| {
                    (!self.layout_disabled_connectivity_links.contains(&index))
                        .then_some(connection)
                })
                .collect();
        }
        technology
    }

    pub(crate) fn normalized_disabled_connectivity_links<I>(&self, links: I) -> BTreeSet<usize>
    where
        I: IntoIterator<Item = usize>,
    {
        let link_count = self.layout_active_technology.connectivity.len();
        links
            .into_iter()
            .filter(|index| *index < link_count)
            .collect()
    }

    pub(crate) fn set_disabled_connectivity_links<I>(&mut self, links: I)
    where
        I: IntoIterator<Item = usize>,
    {
        let next = self.normalized_disabled_connectivity_links(links);
        if self.layout_disabled_connectivity_links != next {
            self.layout_disabled_connectivity_links = next;
            self.mark_layout_dirty();
        }
    }

    pub(crate) fn apply_layout_technology_selection(
        &mut self,
        index: usize,
        update_status: bool,
    ) -> bool {
        let Some((resolved_index, technology)) = layout_builtin_technology_by_index(index) else {
            if update_status {
                self.status_message = format!("Unknown technology index {index}");
            }
            return true;
        };
        if self.layout_active_technology_index == resolved_index
            && self.layout_active_technology.name == technology.name
        {
            if update_status {
                self.status_message =
                    format!("Technology: {}", display_technology_name(&technology.name));
            }
            return true;
        }
        let preserved_extra_layers = match apply_layout_technology_preserving_extra_layers(
            &mut self.workspace.document,
            &technology,
        ) {
            Ok(count) => count,
            Err(error) => {
                if update_status {
                    self.status_message = format!("Technology switch failed: {error}");
                }
                return true;
            }
        };
        self.layout_active_technology_index = resolved_index;
        self.layout_active_technology = technology;
        self.layout_disabled_connectivity_links = self.normalized_disabled_connectivity_links(
            self.layout_disabled_connectivity_links
                .iter()
                .copied()
                .collect::<Vec<_>>(),
        );
        self.layout_layer_depth_overrides
            .retain(|layer, _| self.workspace.document.layers.contains_key(layer));
        if !self
            .workspace
            .document
            .layers
            .contains_key(&self.active_layer)
        {
            self.active_layer = default_active_layer(&self.workspace);
        }
        self.mark_layout_dirty();
        if update_status {
            let extra_label = if preserved_extra_layers == 0 {
                String::new()
            } else {
                format!(", preserved {preserved_extra_layers} extra layer(s)")
            };
            self.status_message = format!(
                "Technology: {}{}",
                display_technology_name(&self.layout_active_technology.name),
                extra_label
            );
        }
        true
    }

    pub(crate) fn toggle_layout_connectivity_link(&mut self, value: &str) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        if index >= self.layout_active_technology.connectivity.len() {
            self.status_message = format!("Connectivity link {} is unavailable", index + 1);
            return true;
        }
        if !self.layout_disabled_connectivity_links.insert(index) {
            self.layout_disabled_connectivity_links.remove(&index);
        }
        self.mark_layout_dirty();
        let connection = &self.layout_active_technology.connectivity[index];
        let state = if self.layout_disabled_connectivity_links.contains(&index) {
            "disabled"
        } else {
            "enabled"
        };
        self.status_message = format!(
            "Connectivity link {} {}: {} / {} / {}",
            index + 1,
            state,
            connection.from,
            connection.through,
            connection.to
        );
        true
    }

    pub(crate) fn clear_disabled_connectivity_links(&mut self) -> bool {
        if self.layout_disabled_connectivity_links.is_empty() {
            self.status_message = "All connectivity links already enabled".to_string();
            return true;
        }
        self.layout_disabled_connectivity_links.clear();
        self.mark_layout_dirty();
        self.status_message = "Enabled all connectivity links".to_string();
        true
    }

    pub(crate) fn app_options_from_state(&self) -> AppOptions {
        let mut options = self.app_options.clone();
        options.schema_version = APP_OPTIONS_SCHEMA_VERSION;
        options.appearance.theme = self.theme_preference;
        options.appearance.unit_display = self.unit_display.slug().to_string();
        options.shell.startup_view = self.active_view.slug().to_string();
        options.shell.show_command_palette_on_start = self.show_command_palette;
        options.shell.show_sidebar_modules = self.show_sidebar_modules;
        options.shell.show_options_panel = self.show_options_panel;
        options.shell.show_diagnostics_panel = self.show_diagnostics_panel;
        options.shell.show_details_panel = self.show_inspector;
        options.shell.show_secondary_panel = self.show_layers;
        options.shell.nav_rail_views = self
            .nav_rail_views
            .iter()
            .map(|view| view.slug().to_string())
            .collect();
        options.layout.default_tool = self.active_tool.slug().to_string();
        options.layout.technology =
            layout_builtin_technology_slug(self.layout_active_technology_index).to_string();
        options.layout.disabled_connectivity_links = self
            .layout_disabled_connectivity_links
            .iter()
            .copied()
            .collect();
        options.layout.active_layer = self.active_layer.0;
        options.layout.snap_enabled = self.snap_enabled;
        options.layout.show_2d_grid = self.show_grid;
        options.layout.show_origin_marker = self.show_origin_marker;
        options.layout.show_drc_overlay = self.show_drc_overlay;
        options.layout.show_reference_images = self.show_reference_images;
        options.layout.hierarchy_depth = self.layout_hierarchy_depth.slug();
        options.layout.hierarchy_min_depth = self.layout_hierarchy_min_depth;
        options.layout.layer_group_filter = self.layout_layer_group_filter.slug().to_string();
        options.layout.layer_usage_filter = self.layout_layer_usage_filter.slug().to_string();
        options.layout.measurement_mode = self.layout_measurement_mode.slug().to_string();
        options.layout.zoom = self.layout_zoom;
        options.layout.pan = self.layout_pan;
        options.layout.view_bookmarks = self
            .layout_view_bookmarks
            .iter()
            .map(|(slot, state)| {
                let mut bookmark = state.to_bookmark_options(*slot);
                bookmark.name = self.layout_view_bookmark_name(*slot);
                bookmark
            })
            .collect();
        options.layout.layer_sets = self
            .layout_layer_sets
            .iter()
            .map(|(slot, state)| options::LayoutLayerSetOptions {
                slot: *slot,
                name: self.layout_layer_set_name(*slot),
                visible_layers: state.visible_layers.iter().map(|layer| layer.0).collect(),
                layer_group_filter: state.layer_group_filter.slug().to_string(),
                layer_usage_filter: state.layer_usage_filter.slug().to_string(),
                layer_depths: state
                    .layer_depth_overrides
                    .iter()
                    .map(|(layer, depth)| options::LayoutLayerDepthOptions {
                        layer: layer.0,
                        max_depth: depth.slug(),
                    })
                    .collect(),
            })
            .collect();
        options.layout.layer_depths = self
            .layout_layer_depth_overrides
            .iter()
            .map(|(layer, depth)| options::LayoutLayerDepthOptions {
                layer: layer.0,
                max_depth: depth.slug(),
            })
            .collect();
        options.layout.library_presets = layout_library_preset_options_from_via_array_presets(
            &self.layout_library_via_array_presets,
        );
        options.viewport3d.show_grid = self.show_3d_grid;
        options.viewport3d.movement_speed = self.camera_3d.speed;
        options.viewport3d.fov_degrees = self.camera_3d.fov_y.to_degrees();
        options.viewport3d.fullscreen_on_start = self.viewport_fullscreen;
        options.domains.mask_prep.severity_filter = self.mask_issue_severity_filter.slug().into();
        options.domains.mask_prep.grouping = self.mask_issue_grouping.slug().into();
        options.domains.layout_diff.changed_only = self.layout_diff_changed_only;
        options.domains.layout_diff.page_size = self.layout_diff_page_size;
        options.domains.inventory.quick_filter = self.inventory_filter.slug().into();
        options.domains.maintenance.work_filter = self.maintenance_work_filter.slug().into();
        options.domains.maintenance.history_filter = self.maintenance_history_filter.slug().into();
        options.domains.scheduler.dispatch_policy =
            dispatch_policy_slug(self.scheduler_policy).into();
        options.domains.scheduler.minimum_priority = self.scheduler_min_priority;
        options.domains.scheduler.conflicts_only = self.scheduler_conflicts_only;
        options.domains.scheduler.focus_selected_tool = self.scheduler_focus_selected_tool;
        options.domains.traceability.impact_mode = self.trace_impact_mode.slug().into();
        options.domains.traceability.related_only = self.trace_related_only;
        options.domains.metrology.map_mode = self.metrology_map_mode.slug().into();
        options.domains.metrology.measurement_kind =
            measurement_kind_slug(self.metrology_kind).into();
        options.domains.metrology.failed_only = self.metrology_failed_only;
        options.domains.yield_dashboard.map_filter = self.yield_map_filter.slug().into();
        options.domains.yield_dashboard.attention_only = self.show_only_attention_wafers;
        options.domains.yield_dashboard.excursions_only = self.show_only_excursions;
        options.domains.spc_fdc.severity_filter = self.spc_severity_filter.slug().into();
        options.domains.spc_fdc.source_filter = self.spc_source_filter.slug().into();
        options.domains.spc_fdc.context_filter = self.spc_context_filter.clone();
        options.domains.process_flow.node_filter = self.process_flow_filter.slug().into();
        options.domains.process_flow.errors_only = self.process_flow_errors_only;
        options.domains.run_to_run.loop_filter = self.process_control_loop_filter.slug().into();
        options.domains.cross_section.step = self.cross_section_step;
        options.domains.cross_section.show_mask = self.cross_section_show_mask;
        options.domains.cross_section.show_dimensions = self.cross_section_show_dimensions;
        options.domains.cross_section.show_risks = self.cross_section_show_risks;
        options.domains.notebook.preview_mode = self.notebook_preview_mode;
        options.domains.notebook.followups_only = self.notebook_followups_only;
        options.domains.notebook.tag_filter = self.notebook_tag_filter.clone();
        options.domains.experiment.run_filter = self.experiment_run_filter.slug().into();
        options.domains.experiment.show_missing_only = self.experiment_show_missing_only;
        options.domains.experiment.capture_value = self.experiment_capture_value;
        options.normalized()
    }
}
