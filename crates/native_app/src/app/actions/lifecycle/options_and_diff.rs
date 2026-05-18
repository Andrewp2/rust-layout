#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub fn apply_app_options(&mut self, options: AppOptions) {
        let options = options.normalized();
        self.theme_preference = options.appearance.theme;
        self.dark_theme = self.theme_preference != options::ThemePreference::Light;
        self.unit_display =
            UnitDisplay::from_slug(&options.appearance.unit_display).unwrap_or(UnitDisplay::Auto);
        if let Some(view) = StartupView::from_slug(&options.shell.startup_view) {
            self.set_active_view(view);
        }
        self.show_command_palette = options.shell.show_command_palette_on_start;
        self.show_sidebar_modules = options.shell.show_sidebar_modules;
        self.show_options_panel = options.shell.show_options_panel;
        self.show_diagnostics_panel = options.shell.show_diagnostics_panel;
        self.show_inspector = options.shell.show_details_panel;
        self.show_layers = options.shell.show_secondary_panel;
        self.nav_rail_views = nav_rail_views_from_option_slugs(&options.shell.nav_rail_views);
        self.active_tool =
            ToolMode::from_slug(&options.layout.default_tool).unwrap_or(ToolMode::Select);
        let (technology_index, _) = layout_builtin_technology_from_slug(&options.layout.technology);
        self.apply_layout_technology_selection(technology_index, false);
        self.set_disabled_connectivity_links(
            options.layout.disabled_connectivity_links.iter().copied(),
        );
        let active_layer = LayerId(options.layout.active_layer);
        if self.workspace.document.layers.contains_key(&active_layer) {
            self.active_layer = active_layer;
        }
        self.snap_enabled = options.layout.snap_enabled;
        self.show_grid = options.layout.show_2d_grid;
        self.show_origin_marker = options.layout.show_origin_marker;
        self.show_drc_overlay = options.layout.show_drc_overlay;
        self.show_reference_images = options.layout.show_reference_images;
        let next_hierarchy_depth = LayoutHierarchyDepth::from_slug(&options.layout.hierarchy_depth)
            .unwrap_or(LayoutHierarchyDepth::Full);
        let next_hierarchy_min_depth = clamp_layout_hierarchy_min_depth_for_max(
            options.layout.hierarchy_min_depth,
            next_hierarchy_depth,
        );
        if self.layout_hierarchy_depth != next_hierarchy_depth
            || self.layout_hierarchy_min_depth != next_hierarchy_min_depth
        {
            self.layout_hierarchy_depth = next_hierarchy_depth;
            self.layout_hierarchy_min_depth = next_hierarchy_min_depth;
            self.invalidate_layout_view_caches();
            self.repair_layout_selection_after_operation();
        }
        self.layout_zoom = options.layout.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
        self.layout_pan = options.layout.pan;
        self.layout_view_bookmarks =
            layout_view_bookmarks_from_options(&options.layout.view_bookmarks);
        self.layout_view_bookmark_names =
            layout_view_bookmark_names_from_options(&options.layout.view_bookmarks);
        self.layout_layer_sets =
            layout_layer_sets_from_options(&self.workspace.document, &options.layout.layer_sets);
        self.layout_layer_set_names =
            layout_layer_set_names_from_options(&options.layout.layer_sets);
        self.layout_layer_group_filter =
            LayoutLayerGroupFilter::from_slug(&options.layout.layer_group_filter)
                .unwrap_or(LayoutLayerGroupFilter::All);
        self.layout_layer_usage_filter =
            LayoutLayerUsageFilter::from_slug(&options.layout.layer_usage_filter)
                .unwrap_or(LayoutLayerUsageFilter::All);
        self.layout_measurement_mode = MeasurementMode::from_slug(&options.layout.measurement_mode)
            .unwrap_or(MeasurementMode::Direct);
        let next_layer_depth_overrides = layout_layer_depth_overrides_from_options(
            &self.workspace.document,
            &options.layout.layer_depths,
        );
        if self.layout_layer_depth_overrides != next_layer_depth_overrides {
            self.layout_layer_depth_overrides = next_layer_depth_overrides;
            self.invalidate_layout_view_caches();
            self.repair_layout_selection_after_operation();
        }
        self.layout_library_via_array_presets =
            layout_via_array_library_presets_from_options(&options.layout.library_presets);
        self.show_3d_grid = options.viewport3d.show_grid;
        self.camera_3d.speed = options.viewport3d.movement_speed;
        self.camera_3d.fov_y = options.viewport3d.fov_degrees.to_radians();
        self.viewport_fullscreen = options.viewport3d.fullscreen_on_start;
        if options.domains.environment.show_alarm_sensors_first {
            self.selected_environment_sensor =
                default_environment_sensor_with_options(&self.workspace, true);
        }
        if options.domains.fab_control.auto_select_first_tool
            && self.selected_equipment_tool.is_none()
        {
            self.selected_equipment_tool = default_equipment_tool(&self.workspace);
        } else if !options.domains.fab_control.auto_select_first_tool {
            self.selected_equipment_tool = None;
        }
        self.mask_issue_severity_filter =
            MaskIssueSeverityFilter::from_slug(&options.domains.mask_prep.severity_filter)
                .unwrap_or(MaskIssueSeverityFilter::All);
        self.mask_issue_grouping =
            MaskIssueGrouping::from_slug(&options.domains.mask_prep.grouping)
                .unwrap_or(MaskIssueGrouping::Code);
        self.layout_diff_changed_only = options.domains.layout_diff.changed_only;
        self.layout_diff_page_size = options.domains.layout_diff.page_size;
        self.inventory_filter =
            InventoryQuickFilter::from_slug(&options.domains.inventory.quick_filter)
                .unwrap_or(InventoryQuickFilter::All);
        self.maintenance_work_filter =
            MaintenanceWorkFilter::from_slug(&options.domains.maintenance.work_filter)
                .unwrap_or(MaintenanceWorkFilter::Actionable);
        self.maintenance_history_filter =
            MaintenanceHistoryFilter::from_slug(&options.domains.maintenance.history_filter)
                .unwrap_or(MaintenanceHistoryFilter::SelectedTool);
        self.scheduler_policy =
            dispatch_policy_from_slug(&options.domains.scheduler.dispatch_policy)
                .unwrap_or(DispatchPolicy::PriorityThenFifo);
        self.scheduler_min_priority = options.domains.scheduler.minimum_priority;
        self.scheduler_conflicts_only = options.domains.scheduler.conflicts_only;
        self.scheduler_focus_selected_tool = options.domains.scheduler.focus_selected_tool;
        self.trace_impact_mode =
            TraceImpactMode::from_slug(&options.domains.traceability.impact_mode)
                .unwrap_or(TraceImpactMode::ToolRun);
        self.trace_related_only = options.domains.traceability.related_only;
        self.metrology_map_mode = MetrologyMapMode::from_slug(&options.domains.metrology.map_mode)
            .unwrap_or(MetrologyMapMode::ValueMap);
        self.metrology_kind =
            measurement_kind_from_slug(&options.domains.metrology.measurement_kind)
                .unwrap_or(MeasurementKind::CriticalDimensionNm);
        self.metrology_failed_only = options.domains.metrology.failed_only;
        self.yield_map_filter =
            YieldMapFilter::from_slug(&options.domains.yield_dashboard.map_filter)
                .unwrap_or(YieldMapFilter::All);
        self.show_only_attention_wafers = options.domains.yield_dashboard.attention_only;
        self.show_only_excursions = options.domains.yield_dashboard.excursions_only;
        self.spc_severity_filter =
            SpcSeverityFilter::from_slug(&options.domains.spc_fdc.severity_filter)
                .unwrap_or(SpcSeverityFilter::All);
        self.spc_source_filter = SpcSourceFilter::from_slug(&options.domains.spc_fdc.source_filter)
            .unwrap_or(SpcSourceFilter::All);
        self.spc_context_filter = options.domains.spc_fdc.context_filter.clone();
        self.process_flow_filter =
            ProcessFlowNodeFilter::from_slug(&options.domains.process_flow.node_filter)
                .unwrap_or(ProcessFlowNodeFilter::All);
        self.process_flow_errors_only = options.domains.process_flow.errors_only;
        self.process_control_loop_filter =
            ControlLoopFilter::from_slug(&options.domains.run_to_run.loop_filter)
                .unwrap_or(ControlLoopFilter::All);
        if !self
            .selected_control_loop
            .as_ref()
            .and_then(|id| self.workspace.process_control.loop_by_id(id))
            .is_some_and(|loop_definition| {
                control_loop_matches(
                    &self.workspace,
                    self.process_control_loop_filter,
                    loop_definition,
                )
            })
        {
            self.selected_control_loop =
                default_control_loop_for_filter(&self.workspace, self.process_control_loop_filter);
            self.selected_control_action =
                default_control_action(&self.workspace, self.selected_control_loop.as_ref());
        }
        self.cross_section_step = options
            .domains
            .cross_section
            .step
            .min(self.workspace.cross_section.steps.len());
        self.cross_section_show_mask = options.domains.cross_section.show_mask;
        self.cross_section_show_dimensions = options.domains.cross_section.show_dimensions;
        self.cross_section_show_risks = options.domains.cross_section.show_risks;
        self.notebook_preview_mode = options.domains.notebook.preview_mode;
        self.notebook_followups_only = options.domains.notebook.followups_only;
        self.notebook_tag_filter = options.domains.notebook.tag_filter.clone();
        self.experiment_run_filter =
            ExperimentRunFilter::from_slug(&options.domains.experiment.run_filter)
                .unwrap_or(ExperimentRunFilter::All);
        self.experiment_show_missing_only = options.domains.experiment.show_missing_only;
        self.experiment_capture_value = options.domains.experiment.capture_value;
        self.app_options = options;
        self.reset_layout_frame_timing();
    }

    pub fn reset_app_options_to_defaults(&mut self) {
        self.apply_app_options(AppOptions::default());
        self.status_message = "Options reset to defaults".to_string();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_app_options_to_file(&mut self) -> Result<(), String> {
        self.sync_app_options_from_state();
        let path = self
            .options_file_path
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(default_options_path);
        save_options_file(&path, &self.app_options)?;
        self.options_file_path = Some(path.display().to_string());
        self.status_message = format!("Saved options to {}", path.display());
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_app_options_from_file(&mut self) -> Result<(), String> {
        let path = self
            .options_file_path
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(default_options_path);
        let options = load_options_file(&path)?;
        self.apply_app_options(options);
        self.options_file_path = Some(path.display().to_string());
        self.status_message = format!("Reloaded options from {}", path.display());
        Ok(())
    }

    pub(crate) fn with_layout_index<T>(&self, f: impl FnOnce(&LayoutIndex) -> T) -> T {
        self.with_layout_index_at_depth(self.workspace.document.top_cell, 0, None, 0, f)
    }

    pub(crate) fn with_layout_display_index<T>(&self, f: impl FnOnce(&LayoutIndex) -> T) -> T {
        self.with_layout_index_at_depth(
            self.layout_view_top_cell,
            self.layout_hierarchy_min_depth as usize,
            self.layout_hierarchy_depth.max_depth(),
            self.layout_view_revision,
            f,
        )
    }

    pub(crate) fn with_layout_index_at_depth<T>(
        &self,
        root_cell: CellId,
        min_depth: usize,
        max_depth: Option<usize>,
        view_revision: u64,
        f: impl FnOnce(&LayoutIndex) -> T,
    ) -> T {
        if !self.app_options.performance.cache_layout_index {
            let index = build_layout_display_index(
                &self.workspace.document,
                root_cell,
                min_depth,
                max_depth,
                &self.layout_hidden_cells,
                &self.layout_layer_depth_overrides,
            );
            return f(&index);
        }
        let mut cache = self.layout_index_cache.borrow_mut();
        let needs_rebuild = cache.as_ref().is_none_or(|cache| {
            cache.layout_revision != self.layout_revision
                || cache.view_revision != view_revision
                || cache.root_cell != root_cell
                || cache.min_depth != min_depth
                || cache.max_depth != max_depth
        });
        if needs_rebuild {
            *cache = Some(LayoutIndexCacheValue {
                layout_revision: self.layout_revision,
                view_revision,
                root_cell,
                min_depth,
                max_depth,
                index: build_layout_display_index(
                    &self.workspace.document,
                    root_cell,
                    min_depth,
                    max_depth,
                    &self.layout_hidden_cells,
                    &self.layout_layer_depth_overrides,
                ),
            });
        }
        f(&cache
            .as_ref()
            .expect("layout index cache should be populated")
            .index)
    }

    pub(crate) fn connectivity_report(&self) -> Result<ConnectivityReport, String> {
        if !self.app_options.performance.cache_connectivity_reports {
            let technology = self.active_connectivity_technology();
            return extract_connectivity(&self.workspace.document, &technology)
                .map_err(|error| error.to_string());
        }
        let mut cache = self.connectivity_report_cache.borrow_mut();
        let needs_rebuild = cache
            .as_ref()
            .is_none_or(|cache| cache.revision != self.layout_revision);
        if needs_rebuild {
            let technology = self.active_connectivity_technology();
            *cache = Some(ConnectivityReportCacheValue {
                revision: self.layout_revision,
                report: extract_connectivity(&self.workspace.document, &technology)
                    .map_err(|error| error.to_string()),
            });
        }
        cache
            .as_ref()
            .expect("connectivity cache should be populated")
            .report
            .clone()
    }

    pub(crate) fn selected_drc_report_history_entry(&self) -> Option<&DrcReportHistoryEntry> {
        let selected_id = self.layout_selected_drc_report_id?;
        self.layout_drc_report_history
            .iter()
            .find(|entry| entry.id == selected_id)
    }

    pub(crate) fn drc_report(&self) -> Option<DrcReportCacheValue> {
        if let Some(entry) = self.selected_drc_report_history_entry() {
            return Some(entry.value.clone());
        }
        self.drc_report_cache
            .borrow()
            .as_ref()
            .filter(|cache| cache.revision == self.layout_revision)
            .map(|cache| cache.value.clone())
    }

    pub(crate) fn push_layout_drc_report(
        &mut self,
        label: impl Into<String>,
        value: DrcReportCacheValue,
    ) {
        let id = self.layout_next_drc_report_id;
        self.layout_next_drc_report_id = self.layout_next_drc_report_id.wrapping_add(1).max(1);
        let entry = DrcReportHistoryEntry {
            id,
            revision: self.layout_revision,
            label: label.into(),
            value: value.clone(),
        };
        *self.drc_report_cache.get_mut() = Some(DrcReportCacheEntry {
            revision: self.layout_revision,
            value,
        });
        self.layout_drc_report_history.insert(0, entry);
        self.layout_drc_report_history
            .truncate(MAX_LAYOUT_DRC_REPORT_HISTORY);
        self.layout_selected_drc_report_id = Some(id);
        self.layout_selected_drc_marker_key = None;
    }

    pub(crate) fn sync_selected_layout_drc_report_cache(&mut self) {
        let Some(entry) = self.selected_drc_report_history_entry().cloned() else {
            self.drc_report_cache.get_mut().take();
            return;
        };
        *self.drc_report_cache.get_mut() = Some(DrcReportCacheEntry {
            revision: entry.revision,
            value: entry.value,
        });
    }

    pub(crate) fn restore_layout_drc_report_history_from_session(
        &mut self,
        reports: Vec<LayoutDrcReportDatabaseEntryExchange>,
        active_report_index: Option<usize>,
    ) {
        let report_limit = reports.len().min(MAX_LAYOUT_DRC_REPORT_HISTORY);
        let mut next_id = 1u64;
        let mut history = Vec::new();
        for (index, report) in reports.into_iter().take(report_limit).enumerate() {
            let label = if report.label.trim().is_empty() {
                format!("Report {}", index + 1)
            } else {
                report.label.trim().chars().take(48).collect::<String>()
            };
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
        let active_index = active_report_index
            .filter(|index| *index < history.len())
            .unwrap_or(0);
        self.layout_next_drc_report_id = next_id;
        self.layout_selected_drc_report_id = history.get(active_index).map(|entry| entry.id);
        self.layout_drc_report_history = history;
        self.layout_selected_drc_marker_key = None;
        self.layout_drc_marker_category_filter = None;
        self.sync_selected_layout_drc_report_cache();
        if !self.layout_drc_report_history.is_empty() {
            self.show_drc_overlay = true;
        }
    }

    pub fn handle_layout_canvas_input(
        &mut self,
        event: &operad::UiInputEvent,
        canvas_rect: UiRect,
    ) -> bool {
        self.handle_layout_canvas_input_with_modifiers(
            event,
            canvas_rect,
            operad::KeyModifiers::NONE,
        )
    }

    pub fn handle_layout_canvas_input_with_modifiers(
        &mut self,
        event: &operad::UiInputEvent,
        canvas_rect: UiRect,
        modifiers: operad::KeyModifiers,
    ) -> bool {
        if self.active_view != StartupView::Layout2d {
            return false;
        }
        let modifiers = self.effective_layout_modifiers(modifiers);
        self.layout_modifiers = modifiers;
        self.layout_canvas_size = Some(rect_size(canvas_rect));
        match event {
            operad::UiInputEvent::PointerMove(point) => {
                let point = *point;
                if !canvas_rect.contains_point(point) && self.layout_drag.is_none() {
                    return false;
                }
                let local = local_canvas_point(point, canvas_rect);
                self.layout_pointer = Some(local);
                let world =
                    self.layout_world_point_for_modifiers(local, rect_size(canvas_rect), modifiers);
                self.update_layout_drag_to(world, modifiers);
                true
            }
            operad::UiInputEvent::PointerDown(point) => {
                let point = *point;
                if !canvas_rect.contains_point(point) {
                    return false;
                }
                let local = local_canvas_point(point, canvas_rect);
                self.layout_pointer = Some(local);
                let world =
                    self.layout_world_point_for_modifiers(local, rect_size(canvas_rect), modifiers);
                self.handle_layout_pointer_down(world, modifiers);
                true
            }
            operad::UiInputEvent::PointerUp(point) => {
                let point = *point;
                if !canvas_rect.contains_point(point) && self.layout_drag.is_none() {
                    return false;
                }
                let local = local_canvas_point(point, canvas_rect);
                self.layout_pointer = Some(local);
                let world =
                    self.layout_world_point_for_modifiers(local, rect_size(canvas_rect), modifiers);
                self.handle_layout_pointer_up(world, modifiers);
                true
            }
            operad::UiInputEvent::Wheel(wheel) => {
                if !canvas_rect.contains_point(wheel.position) || !wheel.delta.y.is_finite() {
                    return false;
                }
                let scroll = -wheel.delta.y;
                if scroll.abs() <= f32::EPSILON {
                    return true;
                }
                let local = local_canvas_point(wheel.position, canvas_rect);
                self.zoom_layout_canvas_around(local, rect_size(canvas_rect), scroll);
                self.layout_pointer = Some(local);
                true
            }
            _ => false,
        }
    }

    pub fn handle_layout_canvas_double_click(
        &mut self,
        point: UiPoint,
        canvas_rect: UiRect,
    ) -> bool {
        if self.active_view != StartupView::Layout2d
            || self.active_tool != ToolMode::Select
            || !canvas_rect.contains_point(point)
        {
            return false;
        }
        let size = rect_size(canvas_rect);
        self.layout_canvas_size = Some(size);
        let local = local_canvas_point(point, canvas_rect);
        self.layout_pointer = Some(local);
        let world = self.snap_layout_point(self.layout_canvas_to_world_local(local, size));
        let Some(hit) = self.hit_selected_layout_edge(world, self.layout_hit_tolerance()) else {
            return false;
        };
        self.insert_layout_vertex(hit.shape_id, hit.edge, world);
        true
    }

    pub fn handle_layout_canvas_secondary_click(
        &mut self,
        point: UiPoint,
        canvas_rect: UiRect,
    ) -> bool {
        if self.active_view != StartupView::Layout2d
            || !matches!(self.active_tool, ToolMode::Polygon | ToolMode::Path)
            || !canvas_rect.contains_point(point)
        {
            return false;
        }
        let size = rect_size(canvas_rect);
        self.layout_canvas_size = Some(size);
        self.layout_pointer = Some(local_canvas_point(point, canvas_rect));
        self.finish_layout_polyline()
    }

    pub fn handle_layout_editor_key(
        &mut self,
        key: operad::KeyCode,
        modifiers: operad::KeyModifiers,
    ) -> bool {
        if self.active_view != StartupView::Layout2d
            || modifiers.ctrl
            || modifiers.alt
            || modifiers.meta
        {
            return false;
        }
        match key {
            operad::KeyCode::Escape => self.clear_layout_edit_drafts(),
            operad::KeyCode::Enter => self.finish_layout_polyline(),
            operad::KeyCode::Backspace | operad::KeyCode::Delete => {
                self.delete_layout_vertex_at_pointer() || self.delete_layout_selection()
            }
            _ => false,
        }
    }

    pub fn pan_layout_canvas_by(&mut self, delta: UiPoint) -> bool {
        if self.active_view != StartupView::Layout2d || !delta.x.is_finite() || !delta.y.is_finite()
        {
            return false;
        }
        if delta.x.abs() > f32::EPSILON || delta.y.abs() > f32::EPSILON {
            self.record_layout_previous_view();
        }
        self.layout_pan[0] += delta.x;
        self.layout_pan[1] += delta.y;
        self.reset_layout_frame_timing();
        true
    }

    pub fn handle_layout_3d_canvas_input(
        &mut self,
        event: &operad::UiInputEvent,
        canvas_rect: UiRect,
    ) -> bool {
        if self.active_view != StartupView::Layout3d {
            return false;
        }
        match event {
            operad::UiInputEvent::PointerDown(point) => {
                if !canvas_rect.contains_point(*point) {
                    return false;
                }
                self.layout_3d_drag = Some(*point);
                if self.app_options.viewport3d.capture_flycam_on_click {
                    self.flycam_captured = true;
                }
                true
            }
            operad::UiInputEvent::PointerMove(point) => {
                let Some(previous) = self.layout_3d_drag else {
                    return false;
                };
                let delta = UiPoint::new(point.x - previous.x, point.y - previous.y);
                self.layout_3d_drag = Some(*point);
                let sensitivity = if self.flycam_captured {
                    self.app_options.viewport3d.mouse_sensitivity
                } else {
                    self.app_options.viewport3d.mouse_sensitivity * 2.0
                };
                self.camera_3d.look_delta(delta, sensitivity);
                true
            }
            operad::UiInputEvent::PointerUp(point) => {
                if self.layout_3d_drag.is_none() && !canvas_rect.contains_point(*point) {
                    return false;
                }
                self.layout_3d_drag = None;
                true
            }
            operad::UiInputEvent::Wheel(wheel) => {
                if !canvas_rect.contains_point(wheel.position) {
                    return false;
                }
                self.camera_3d.scroll_forward(-wheel.delta.y);
                true
            }
            _ => false,
        }
    }

    pub fn handle_layout_3d_key(
        &mut self,
        key: operad::KeyCode,
        _modifiers: operad::KeyModifiers,
    ) -> bool {
        if self.active_view != StartupView::Layout3d {
            return false;
        }
        match key {
            operad::KeyCode::Escape => {
                self.viewport_fullscreen = false;
                self.flycam_captured = false;
                self.layout_3d_drag = None;
                true
            }
            operad::KeyCode::Character('f') | operad::KeyCode::Character('F') => {
                self.viewport_fullscreen = !self.viewport_fullscreen;
                true
            }
            operad::KeyCode::ArrowUp | operad::KeyCode::Character('w' | 'W') => self
                .advance_layout_3d_flycam(
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    _modifiers.shift,
                    1.0 / 60.0,
                ),
            operad::KeyCode::ArrowDown | operad::KeyCode::Character('s' | 'S') => self
                .advance_layout_3d_flycam(
                    false,
                    true,
                    false,
                    false,
                    false,
                    false,
                    _modifiers.shift,
                    1.0 / 60.0,
                ),
            operad::KeyCode::ArrowLeft | operad::KeyCode::Character('a' | 'A') => self
                .advance_layout_3d_flycam(
                    false,
                    false,
                    true,
                    false,
                    false,
                    false,
                    _modifiers.shift,
                    1.0 / 60.0,
                ),
            operad::KeyCode::ArrowRight | operad::KeyCode::Character('d' | 'D') => self
                .advance_layout_3d_flycam(
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    _modifiers.shift,
                    1.0 / 60.0,
                ),
            operad::KeyCode::Character('e' | 'E' | ' ') => self.advance_layout_3d_flycam(
                false,
                false,
                false,
                false,
                true,
                false,
                _modifiers.shift,
                1.0 / 60.0,
            ),
            operad::KeyCode::Character('q' | 'Q') => self.advance_layout_3d_flycam(
                false,
                false,
                false,
                false,
                false,
                true,
                _modifiers.shift,
                1.0 / 60.0,
            ),
            _ => false,
        }
    }

    pub fn set_layout_3d_flycam_capture(&mut self, captured: bool) -> bool {
        if captured && self.active_view != StartupView::Layout3d {
            return false;
        }
        let changed = self.flycam_captured != captured;
        self.flycam_captured = captured;
        if !captured {
            self.layout_3d_drag = None;
        }
        changed
    }

    pub fn apply_layout_3d_mouse_delta(&mut self, delta: UiPoint, _raw_motion: bool) -> bool {
        if self.active_view != StartupView::Layout3d {
            return false;
        }
        if !delta.x.is_finite() || !delta.y.is_finite() || (delta.x == 0.0 && delta.y == 0.0) {
            return false;
        }
        let sensitivity = if _raw_motion {
            self.app_options.viewport3d.mouse_sensitivity
        } else {
            self.app_options.viewport3d.mouse_sensitivity * 2.0
        };
        self.camera_3d.look_delta(delta, sensitivity);
        true
    }

    pub fn advance_layout_3d_flycam(
        &mut self,
        forward: bool,
        backward: bool,
        left: bool,
        right: bool,
        up: bool,
        down: bool,
        fast: bool,
        dt: f32,
    ) -> bool {
        if self.active_view != StartupView::Layout3d || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let basis = self.camera_3d.basis();
        let mut direction = Vec3::ZERO;
        if forward {
            direction += basis.forward;
        }
        if backward {
            direction = direction - basis.forward;
        }
        if right {
            direction += basis.right;
        }
        if left {
            direction = direction - basis.right;
        }
        if up {
            direction += Vec3::new(0.0, 0.0, 1.0);
        }
        if down {
            direction = direction - Vec3::new(0.0, 0.0, 1.0);
        }
        if direction.length() <= f32::EPSILON {
            return false;
        }
        let speed = self.camera_3d.speed
            * if fast {
                self.app_options.viewport3d.fast_multiplier
            } else {
                1.0
            };
        let delta = direction.normalized() * (speed * dt.clamp(1.0 / 240.0, 1.0 / 20.0));
        self.camera_3d.position += delta;
        true
    }

    pub(crate) fn reset_3d_camera_to_document(&mut self) {
        let bounds = self
            .with_layout_index(|index| index.bounds())
            .unwrap_or_else(|| Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000)));
        let center = bounds.center();
        let span = bounds.width().abs().max(bounds.height().abs()).max(1_000) as f32;
        let target = Vec3::new(center.x as f32, center.y as f32, 360.0);
        let position = Vec3::new(
            center.x as f32 - span * 0.85,
            center.y as f32 - span * 0.95,
            span * 0.62 + 1_800.0,
        );
        self.camera_3d = Camera3d::look_at(position, target, span.max(2_000.0) * 0.85);
    }

    pub(crate) fn camera_3d_far_plane(&self) -> f32 {
        let bounds = self
            .with_layout_index(|index| index.bounds())
            .unwrap_or_else(|| Rect::from_min_size(Point::new(-500, -500), 1_000, 1_000));
        let max_layer_z = self
            .workspace
            .document
            .layers
            .values()
            .map(|layer| layer_3d_stack_range(layer.process).1)
            .fold(1_000.0_f32, f32::max);
        let span = bounds.width().max(bounds.height()).max(1_000) as f32;
        let corners = [
            Vec3::new(bounds.min.x as f32, bounds.min.y as f32, 0.0),
            Vec3::new(bounds.max.x as f32, bounds.min.y as f32, 0.0),
            Vec3::new(bounds.max.x as f32, bounds.max.y as f32, 0.0),
            Vec3::new(bounds.min.x as f32, bounds.max.y as f32, 0.0),
            Vec3::new(bounds.min.x as f32, bounds.min.y as f32, max_layer_z),
            Vec3::new(bounds.max.x as f32, bounds.min.y as f32, max_layer_z),
            Vec3::new(bounds.max.x as f32, bounds.max.y as f32, max_layer_z),
            Vec3::new(bounds.min.x as f32, bounds.max.y as f32, max_layer_z),
        ];
        let forward = self.camera_3d.forward();
        let max_forward_depth = corners
            .into_iter()
            .map(|corner| (corner - self.camera_3d.position).dot(forward))
            .fold(CAMERA_NEAR_PLANE * 4.0, f32::max);
        let margin = span * CAMERA_FAR_PLANE_MARGIN_MULTIPLIER + max_layer_z * 2.0;
        let minimum_far =
            CAMERA_FAR_PLANE_MIN.max(span * CAMERA_FAR_PLANE_SPAN_MULTIPLIER + max_layer_z);
        (max_forward_depth + margin).max(minimum_far)
    }

    pub(crate) fn handle_layout_pointer_down(
        &mut self,
        world: Point,
        modifiers: operad::KeyModifiers,
    ) {
        match self.active_tool {
            ToolMode::Select => self.begin_layout_selection(world, modifiers),
            ToolMode::Rect => {
                self.drawing_start = Some(world);
                self.layout_drag = Some(LayoutCanvasDrag::Rect { start: world });
                self.status_message = "Drawing rectangle".to_string();
            }
            ToolMode::Polygon | ToolMode::Path => {
                let point = self
                    .drawing_points
                    .last()
                    .copied()
                    .map(|start| layout_drag_world(start, world, modifiers))
                    .unwrap_or(world);
                if self.drawing_points.last().copied() != Some(point) {
                    self.drawing_points.push(point);
                }
            }
            ToolMode::Via => {
                let via_layer = self
                    .workspace
                    .document
                    .layer_by_process(ProcessLayer::Via1)
                    .unwrap_or(self.active_layer);
                let lower = self
                    .workspace
                    .document
                    .layer_by_process(ProcessLayer::Metal1)
                    .unwrap_or(self.active_layer);
                let upper = self
                    .workspace
                    .document
                    .layer_by_process(ProcessLayer::Metal2)
                    .unwrap_or(self.active_layer);
                self.add_layout_shape(
                    via_layer,
                    ShapeKind::Via {
                        center: world,
                        size: 180,
                        lower,
                        upper,
                    },
                );
            }
            ToolMode::Label => {
                self.place_layout_label(world);
            }
            ToolMode::Measure => {
                let world = self
                    .measure_start
                    .map(|start| layout_drag_world(start, world, modifiers))
                    .unwrap_or(world);
                if let Some(start) = self.measure_start.take() {
                    let end = layout_measurement_mode_endpoint(
                        start,
                        world,
                        self.layout_measurement_mode,
                    );
                    let distance =
                        layout_measurement_mode_length(start, end, self.layout_measurement_mode);
                    let layer = self
                        .workspace
                        .document
                        .layer_by_process(ProcessLayer::Annotation)
                        .unwrap_or(self.active_layer);
                    self.add_layout_shape(
                        layer,
                        ShapeKind::Measurement {
                            a: start,
                            b: end,
                            label: self.format_layout_length(distance),
                            mode: self.layout_measurement_mode,
                        },
                    );
                } else {
                    self.measure_start = Some(world);
                    self.status_message = "Measurement start set".to_string();
                }
            }
            ToolMode::Route => {
                let point = self
                    .route_points
                    .last()
                    .copied()
                    .map(|start| layout_drag_world(start, world, modifiers))
                    .unwrap_or(world);
                self.route_points.push(point);
                self.status_message = if self.route_points.len() >= 2 {
                    "Route endpoints set; run route to place wire".to_string()
                } else {
                    "Route start set".to_string()
                };
            }
            ToolMode::Trace => {
                self.trace_layout_net_at_point(world);
            }
        }
    }

    pub(crate) fn handle_layout_pointer_up(
        &mut self,
        world: Point,
        modifiers: operad::KeyModifiers,
    ) {
        let Some(drag) = self.layout_drag.clone() else {
            return;
        };
        if !matches!(drag, LayoutCanvasDrag::Rect { .. }) {
            self.update_layout_drag_to(world, modifiers);
        }
        let Some(drag) = self.layout_drag.take() else {
            return;
        };
        if let LayoutCanvasDrag::Rect { start } = drag {
            self.drawing_start = None;
            let rect = Rect::new(start, layout_rect_end(start, world, modifiers));
            let minimum_size = self.minimum_layout_draw_size();
            if rect.width().abs() >= minimum_size && rect.height().abs() >= minimum_size {
                self.add_layout_shape(self.active_layer, ShapeKind::Rectangle(rect));
            }
            return;
        }
        self.commit_layout_drag_history(drag);
    }

    pub(crate) fn begin_layout_selection(&mut self, world: Point, modifiers: operad::KeyModifiers) {
        let tolerance = self.layout_hit_tolerance();
        if let Some(hit) = self.hit_selected_layout_vertex(world, tolerance) {
            let Some(original_shape) = self.workspace.document.shapes.get(&hit.shape_id) else {
                return;
            };
            let original_position = editable_vertex_points(&original_shape.kind)
                .get(hit.vertex)
                .copied()
                .unwrap_or(world);
            self.layout_drag = Some(LayoutCanvasDrag::MoveVertex {
                shape_id: hit.shape_id,
                vertex: hit.vertex,
                start_world: world,
                original_position,
                original_shape,
            });
            self.status_message = format!(
                "Editing vertex {} on shape #{}",
                hit.vertex + 1,
                hit.shape_id.0
            );
            return;
        }
        if let Some(hit) = self.hit_selected_layout_draggable_edge(world, tolerance) {
            let Some(original_shape) = self.workspace.document.shapes.get(&hit.shape_id) else {
                return;
            };
            self.layout_drag = Some(LayoutCanvasDrag::MoveEdge {
                shape_id: hit.shape_id,
                edge: hit.edge,
                last_world: world,
                original_shape,
            });
            self.status_message =
                format!("Editing edge {} on shape #{}", hit.edge + 1, hit.shape_id.0);
            return;
        }

        let occurrence_hit =
            self.with_layout_display_index(|index| index.hit_test_occurrence(world, tolerance));
        if let Some(occurrence) = occurrence_hit {
            let shape_id = occurrence.source_shape_id();
            self.selected_layout_shape = Some(shape_id);
            self.selected_layout_occurrence = Some(occurrence.clone());
            if let Some(shape) = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
                .map(|view| view.shape.to_shape())
            {
                self.active_layer = shape.layer;
            }
            if occurrence.is_top_level() && self.workspace.document.shapes.contains_key(&shape_id) {
                self.layout_drag = Some(LayoutCanvasDrag::MoveShape {
                    shape_id,
                    start_world: world,
                    last_world: world,
                    copy_on_drag: modifiers.alt,
                    added_shape: None,
                });
                self.status_message = format!("Selected shape #{}", shape_id.0);
            } else if let Some((parent, instance_id)) =
                self.workspace.document.instance_parent_for_path_from_cell(
                    self.layout_view_top_cell,
                    &occurrence.instance_path,
                )
            {
                self.layout_drag = Some(LayoutCanvasDrag::MoveInstance {
                    parent,
                    instance_id,
                    start_world: world,
                    last_world: world,
                    copy_on_drag: modifiers.alt,
                    added_instance: None,
                });
                self.status_message =
                    format!("Picked instance {}", layout_occurrence_label(&occurrence));
            } else {
                self.layout_drag = None;
                self.status_message =
                    format!("Picked instance {}", layout_occurrence_label(&occurrence));
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.layout_drag = None;
            self.status_message = "Layout selection cleared".to_string();
        }
    }

    pub(crate) fn add_layout_shape(&mut self, layer: LayerId, kind: ShapeKind) -> Option<ShapeId> {
        self.add_layout_shape_with_metadata(layer, kind, None, None)
    }

    pub(crate) fn add_layout_shape_with_metadata(
        &mut self,
        layer: LayerId,
        kind: ShapeKind,
        net: Option<NetId>,
        name: Option<String>,
    ) -> Option<ShapeId> {
        if !self.workspace.document.layers.contains_key(&layer) {
            self.status_message = "Add a layer before drawing".to_string();
            return None;
        }
        let id = self.workspace.document.allocate_shape_id();
        let shape = Shape {
            id,
            layer,
            net,
            kind,
            name,
            properties: BTreeMap::new(),
        };
        self.apply_layout_operation_with_history(
            Operation::AddShape {
                shape: shape.clone(),
            },
            Operation::DeleteShape { id },
        );
        self.selected_layout_shape = Some(id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(id));
        self.active_layer = layer;
        self.status_message = format!("Added shape #{}", id.0);
        Some(id)
    }

    pub(crate) fn place_layout_label(&mut self, position: Point) -> bool {
        let text = self.layout_label_text_for_point(position);
        let layer = self.active_layer;
        let Some(shape_id) = self.add_layout_shape(
            layer,
            ShapeKind::Label {
                position,
                text: text.clone(),
            },
        ) else {
            return true;
        };
        self.status_message = format!("Added label {text} as shape #{}", shape_id.0);
        true
    }

    pub(crate) fn layout_label_text_for_point(&self, point: Point) -> String {
        if let Ok(report) = self.connectivity_report() {
            if let Some(component) = self
                .selected_layout_occurrence
                .as_ref()
                .and_then(|occurrence| report.component_for_occurrence(occurrence))
                .and_then(|component_id| report.component(component_id))
            {
                return layout_label_text_for_component(component);
            }
            if let Ok(occurrence) = self.top_layout_occurrence_at_trace_point(point)
                && let Some(component) = report
                    .component_for_occurrence(&occurrence)
                    .and_then(|component_id| report.component(component_id))
            {
                return layout_label_text_for_component(component);
            }
        }
        self.layout_net_at_point(point)
            .map(|net| format!("NET{}", net.0))
            .unwrap_or_else(|| "NET".to_string())
    }

    pub(crate) fn route_layout_between_points(&mut self) -> bool {
        if self.route_points.len() < 2 {
            self.status_message = "Set route endpoints first".to_string();
            return true;
        }
        let start = self.route_points[0];
        let goal = *self.route_points.last().unwrap_or(&start);
        let net = self
            .layout_net_at_point(start)
            .or_else(|| self.layout_net_at_point(goal));
        let ignored_shape_ids = self.layout_route_ignored_shape_ids(start, goal, net);
        let ignored_shape_count = ignored_shape_ids.len();
        let route = router::route(
            &self.workspace.document,
            router::RouteRequest {
                start,
                goal,
                layer: self.active_layer,
                wire_width: 180,
                bounds: Some(Rect::new(start, goal).expanded(8_000)),
                allowed_net: net,
                ignored_shape_ids,
            },
            router::RouterConfig::default(),
        )
        .map(|route| route.points)
        .unwrap_or_else(|_| self.route_points.clone());
        let name = net.map(|net| format!("NET{}", net.0));
        self.route_points.clear();
        let Some(shape_id) = self.add_layout_shape_with_metadata(
            self.active_layer,
            ShapeKind::Path {
                points: route,
                width: 180,
            },
            net,
            name.clone(),
        ) else {
            return true;
        };
        self.status_message = if let Some(name) = name {
            if ignored_shape_count > 0 {
                format!(
                    "Route placed for {name} using {ignored_shape_count} existing net shape(s) as shape #{}",
                    shape_id.0
                )
            } else {
                format!("Route placed for {name} as shape #{}", shape_id.0)
            }
        } else {
            format!("Route placed as shape #{}", shape_id.0)
        };
        true
    }

    pub(crate) fn layout_route_ignored_shape_ids(
        &self,
        start: Point,
        goal: Point,
        net: Option<NetId>,
    ) -> Vec<ShapeId> {
        let mut ignored = BTreeSet::new();
        if let Some(net) = net {
            for flattened in self.workspace.document.flattened_shapes() {
                if flattened.shape.layer == self.active_layer && flattened.shape.net == Some(net) {
                    ignored.insert(flattened.source_shape_id());
                }
            }
        }

        if self.workspace.document.flattened_shape_count_estimate()
            <= MAX_CONNECTIVITY_OVERLAY_SHAPES
            && let Ok(report) = self.connectivity_report()
        {
            let start_component =
                layout_connectivity_component_at_point(&self.workspace.document, &report, start);
            let goal_component =
                layout_connectivity_component_at_point(&self.workspace.document, &report, goal);
            if let (Some(start_component), Some(goal_component)) = (start_component, goal_component)
                && start_component == goal_component
                && let Some(component) = report.component(start_component)
            {
                for occurrence in &component.shapes {
                    if self
                        .workspace
                        .document
                        .shape_view_for_occurrence(occurrence)
                        .is_some_and(|view| view.shape.layer == self.active_layer)
                    {
                        ignored.insert(occurrence.source_shape_id());
                    }
                }
            }
        }

        ignored.into_iter().collect()
    }

    pub(crate) fn layout_net_at_point(&self, point: Point) -> Option<NetId> {
        self.workspace
            .document
            .flattened_shapes()
            .into_iter()
            .map(|shape| shape.transformed_shape())
            .find(|shape| shape.kind.bounds().expanded(1).contains_point(point))
            .and_then(|shape| shape.net)
    }

    pub(crate) fn insert_layout_vertex(
        &mut self,
        shape_id: ShapeId,
        edge: usize,
        position: Point,
    ) -> bool {
        let Some(old_shape) = self.workspace.document.shapes.get(&shape_id) else {
            return false;
        };
        let Some(new_shape) = shape_with_inserted_vertex(&old_shape, edge, position) else {
            self.status_message = "Vertex insertion needs a polygon or path edge".to_string();
            return false;
        };
        self.replace_layout_shape(shape_id, old_shape, new_shape, "Inserted vertex")
    }

    pub(crate) fn delete_layout_vertex(&mut self, shape_id: ShapeId, vertex: usize) -> bool {
        let Some(old_shape) = self.workspace.document.shapes.get(&shape_id) else {
            return false;
        };
        let Some(new_shape) = shape_with_deleted_vertex(&old_shape, vertex) else {
            self.status_message = "Cannot delete that vertex".to_string();
            return true;
        };
        self.replace_layout_shape(shape_id, old_shape, new_shape, "Deleted vertex")
    }

    pub(crate) fn delete_layout_vertex_at_pointer(&mut self) -> bool {
        if self.active_tool != ToolMode::Select {
            return false;
        }
        let (Some(pointer), Some(size)) = (self.layout_pointer, self.layout_canvas_size) else {
            return false;
        };
        let world = self.snap_layout_point(self.layout_canvas_to_world_local(pointer, size));
        let Some(hit) = self.hit_selected_layout_vertex(world, self.layout_hit_tolerance()) else {
            return false;
        };
        self.delete_layout_vertex(hit.shape_id, hit.vertex)
    }

    pub(crate) fn replace_layout_shape(
        &mut self,
        shape_id: ShapeId,
        old_shape: Shape,
        new_shape: Shape,
        label: &str,
    ) -> bool {
        if old_shape == new_shape {
            return false;
        }
        self.apply_layout_operation_with_history(
            Operation::ReplaceShape {
                id: shape_id,
                shape: new_shape,
            },
            Operation::ReplaceShape {
                id: shape_id,
                shape: old_shape,
            },
        );
        self.selected_layout_shape = Some(shape_id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
        self.status_message = format!("{label} on shape #{}", shape_id.0);
        true
    }

    pub(crate) fn replace_layout_shape_live(
        &mut self,
        shape_id: ShapeId,
        old_shape: Shape,
        new_shape: Shape,
        label: &str,
    ) -> bool {
        if old_shape == new_shape {
            return false;
        }
        self.apply_layout_operation_live(Operation::ReplaceShape {
            id: shape_id,
            shape: new_shape,
        });
        self.selected_layout_shape = Some(shape_id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
        self.status_message = format!("{label} on shape #{}", shape_id.0);
        true
    }

    pub(crate) fn hit_selected_layout_vertex(
        &self,
        world: Point,
        tolerance: Coord,
    ) -> Option<LayoutVertexHit> {
        let shape_id = self.selected_layout_shape?;
        let shape = self.selected_top_level_layout_shape()?;
        editable_vertex_points(&shape.kind)
            .into_iter()
            .enumerate()
            .filter_map(|(vertex, point)| {
                let distance = point.distance_to(world);
                (distance <= tolerance as f64).then_some((vertex, distance))
            })
            .min_by(|(_, left), (_, right)| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(vertex, _)| LayoutVertexHit { shape_id, vertex })
    }

    pub(crate) fn hit_selected_layout_edge(
        &self,
        world: Point,
        tolerance: Coord,
    ) -> Option<LayoutEdgeHit> {
        let shape_id = self.selected_layout_shape?;
        let shape = self.selected_top_level_layout_shape()?;
        editable_edges(&shape.kind)
            .into_iter()
            .enumerate()
            .filter_map(|(edge, (a, b))| {
                let distance = distance_point_to_segment(world, a, b);
                (distance <= tolerance as f64).then_some((edge, distance))
            })
            .min_by(|(_, left), (_, right)| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(edge, _)| LayoutEdgeHit { shape_id, edge })
    }

    pub(crate) fn hit_selected_layout_draggable_edge(
        &self,
        world: Point,
        tolerance: Coord,
    ) -> Option<LayoutEdgeHit> {
        let hit = self.hit_selected_layout_edge(world, tolerance)?;
        let shape = self.workspace.document.shapes.get(&hit.shape_id)?;
        shape_edge_is_draggable(&shape.kind, hit.edge).then_some(hit)
    }

    pub(crate) fn finish_layout_polyline(&mut self) -> bool {
        match self.active_tool {
            ToolMode::Polygon if self.drawing_points.len() >= 3 => {
                let points = std::mem::take(&mut self.drawing_points);
                self.add_layout_shape(
                    self.active_layer,
                    ShapeKind::Polygon(geometry_core::Polygon::new(points)),
                );
                true
            }
            ToolMode::Path if self.drawing_points.len() >= 2 => {
                let points = std::mem::take(&mut self.drawing_points);
                self.add_layout_shape(self.active_layer, ShapeKind::Path { points, width: 180 });
                true
            }
            _ => false,
        }
    }

    pub(crate) fn clear_layout_edit_drafts(&mut self) -> bool {
        let drag = self.layout_drag.take();
        let had_draft = drag.is_some()
            || self.drawing_start.is_some()
            || !self.drawing_points.is_empty()
            || self.measure_start.is_some()
            || !self.route_points.is_empty();
        if let Some(drag) = drag {
            self.rollback_layout_drag(drag);
        }
        self.drawing_start = None;
        self.drawing_points.clear();
        self.measure_start = None;
        self.route_points.clear();
        if had_draft {
            self.status_message = "Layout edit canceled".to_string();
        }
        had_draft
    }

    pub(crate) fn rollback_layout_drag(&mut self, drag: LayoutCanvasDrag) -> bool {
        match drag {
            LayoutCanvasDrag::MoveShape {
                shape_id,
                start_world,
                last_world,
                added_shape,
                ..
            } => {
                if added_shape.is_some() {
                    return self
                        .apply_layout_operation_live(Operation::DeleteShape { id: shape_id });
                }
                let total_delta =
                    Vector::new(last_world.x - start_world.x, last_world.y - start_world.y);
                if total_delta == Vector::ZERO {
                    return false;
                }
                self.apply_layout_operation_live(Operation::MoveShape {
                    id: shape_id,
                    delta: Vector::new(-total_delta.dx, -total_delta.dy),
                })
            }
            LayoutCanvasDrag::MoveInstance {
                parent,
                instance_id,
                start_world,
                last_world,
                added_instance,
                ..
            } => {
                if added_instance.is_some() {
                    return self.apply_layout_operation_live(Operation::DeleteInstance {
                        parent,
                        id: instance_id,
                    });
                }
                let total_delta =
                    Vector::new(last_world.x - start_world.x, last_world.y - start_world.y);
                if total_delta == Vector::ZERO {
                    return false;
                }
                self.apply_layout_operation_live(Operation::MoveInstance {
                    parent,
                    id: instance_id,
                    delta: Vector::new(-total_delta.dx, -total_delta.dy),
                })
            }
            LayoutCanvasDrag::MoveVertex {
                shape_id,
                original_shape,
                ..
            }
            | LayoutCanvasDrag::MoveEdge {
                shape_id,
                original_shape,
                ..
            } => self.apply_layout_operation_live(Operation::ReplaceShape {
                id: shape_id,
                shape: original_shape,
            }),
            LayoutCanvasDrag::Rect { .. } => false,
        }
    }

    pub(crate) fn zoom_layout_canvas_around(&mut self, local: UiPoint, size: UiSize, scroll: f32) {
        let previous_zoom = self.layout_zoom;
        let previous_state = self.current_layout_view_state();
        self.layout_zoom =
            (self.layout_zoom * (scroll * 0.0015).exp()).clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
        if (self.layout_zoom - previous_zoom).abs() <= f32::EPSILON {
            return;
        }
        self.layout_previous_view = Some(previous_state);
        let before_x = (local.x - size.width * 0.5 - self.layout_pan[0]) / previous_zoom;
        let before_y = (local.y - size.height * 0.5 - self.layout_pan[1]) / previous_zoom;
        self.layout_pan[0] = local.x - size.width * 0.5 - before_x * self.layout_zoom;
        self.layout_pan[1] = local.y - size.height * 0.5 - before_y * self.layout_zoom;
        self.reset_layout_frame_timing();
    }

    pub(crate) fn layout_viewport_for_size(&self, size: UiSize) -> Rect {
        Rect::new(
            self.layout_canvas_to_world_local(UiPoint::new(0.0, 0.0), size),
            self.layout_canvas_to_world_local(UiPoint::new(size.width, size.height), size),
        )
    }

    pub(crate) fn layout_world_to_canvas(&self, point: Point, size: UiSize) -> UiPoint {
        UiPoint::new(
            size.width * 0.5 + self.layout_pan[0] + point.x as f32 * self.layout_zoom,
            size.height * 0.5 + self.layout_pan[1] - point.y as f32 * self.layout_zoom,
        )
    }

    pub(crate) fn layout_canvas_to_world_local(&self, point: UiPoint, size: UiSize) -> Point {
        let local_x = (point.x - size.width * 0.5 - self.layout_pan[0]) / self.layout_zoom;
        let local_y = (point.y - size.height * 0.5 - self.layout_pan[1]) / self.layout_zoom;
        Point::new(local_x.round() as Coord, (-local_y).round() as Coord)
    }

    pub(crate) fn layout_world_point_for_modifiers(
        &self,
        local: UiPoint,
        size: UiSize,
        modifiers: operad::KeyModifiers,
    ) -> Point {
        self.snap_layout_point_for_modifiers(
            self.layout_canvas_to_world_local(local, size),
            modifiers,
        )
    }

    pub(crate) fn snap_layout_point(&self, point: Point) -> Point {
        if self.snap_enabled {
            point.snap(self.workspace.document.grid.max(1))
        } else {
            point
        }
    }

    pub(crate) fn snap_layout_point_for_modifiers(
        &self,
        point: Point,
        modifiers: operad::KeyModifiers,
    ) -> Point {
        if modifiers.ctrl {
            point
        } else {
            self.snap_layout_point(point)
        }
    }

    pub(crate) fn layout_hit_tolerance(&self) -> Coord {
        ((10.0 / self.layout_zoom.max(LAYOUT_MIN_ZOOM)) as Coord)
            .max(self.workspace.document.grid.max(1))
    }

    pub(crate) fn minimum_layout_draw_size(&self) -> Coord {
        self.workspace.document.grid.max(1)
    }

    pub(crate) fn layout_grid_step_for_zoom(&self) -> Coord {
        let base = self.workspace.document.grid.max(1);
        let mut step = base;
        while (step as f32) * self.layout_zoom < 24.0 && step < Coord::MAX / 2 {
            step *= 2;
        }
        step
    }

    pub(crate) fn format_layout_length(&self, length_dbu: f64) -> String {
        format_physical_length_with_options(
            length_dbu,
            self.active_layout_technology().dbu_per_micron,
            self.unit_display,
            2,
        )
    }

    pub(crate) fn layout_import_offset_step(&self) -> Coord {
        self.workspace.document.grid.max(1) * LAYOUT_IMPORT_OFFSET_STEP_GRIDS
    }

    pub(crate) fn layout_import_offset_label(&self) -> String {
        format!(
            "{}, {}",
            self.format_layout_length(self.layout_import_offset.dx as f64),
            self.format_layout_length(self.layout_import_offset.dy as f64)
        )
    }

    pub(crate) fn layout_import_layer_id_offset_label(&self) -> String {
        match self.layout_import_layer_id_offset.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", self.layout_import_layer_id_offset),
            std::cmp::Ordering::Equal => "0".to_string(),
            std::cmp::Ordering::Less => self.layout_import_layer_id_offset.to_string(),
        }
    }

    pub fn workflow_focus_lot(&self) -> Option<&str> {
        self.workflow_focus_lot.as_deref()
    }

    pub fn selected_equipment_tool(&self) -> Option<&EquipmentToolId> {
        self.selected_equipment_tool.as_ref()
    }

    pub fn selected_maintenance_tool(&self) -> Option<&EquipmentToolId> {
        self.selected_maintenance_tool.as_ref()
    }

    pub fn maintenance_work_filter(&self) -> MaintenanceWorkFilter {
        self.maintenance_work_filter
    }

    pub fn maintenance_history_filter(&self) -> MaintenanceHistoryFilter {
        self.maintenance_history_filter
    }

    pub fn selected_environment_sensor(&self) -> Option<&str> {
        self.selected_environment_sensor.as_deref()
    }

    pub fn selected_inventory_lot(&self) -> Option<&MaterialLotId> {
        self.selected_inventory_lot.as_ref()
    }

    pub fn inventory_filter(&self) -> InventoryQuickFilter {
        self.inventory_filter
    }

    pub fn mask_source_lot(&self) -> Option<&LotId> {
        self.mask_source_lot.as_ref()
    }

    pub fn mask_issue_severity_filter(&self) -> MaskIssueSeverityFilter {
        self.mask_issue_severity_filter
    }

    pub fn mask_issue_grouping(&self) -> MaskIssueGrouping {
        self.mask_issue_grouping
    }

    pub fn layout_diff_baseline(&self) -> LayoutDiffSource {
        self.layout_diff_baseline
    }

    pub fn layout_diff_candidate(&self) -> LayoutDiffSource {
        self.layout_diff_candidate
    }
}
