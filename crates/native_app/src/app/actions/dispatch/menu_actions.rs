#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn apply_menu_action(&mut self, action: &str) {
        if let Some(slug) = action.strip_prefix("tool.") {
            if let Some(tool) = ToolMode::from_slug(slug) {
                self.active_tool = tool;
                self.status_message = format!("{} tool selected", tool.label());
                self.active_menu = None;
                return;
            }
        }
        if let Some(slug) = action.strip_prefix("display.units.") {
            if let Some(unit) = UnitDisplay::from_slug(slug) {
                self.unit_display = unit;
                self.status_message = format!("Units: {}", unit.label());
                self.active_menu = None;
                self.active_view_group = None;
                return;
            }
        }
        if let Some(index) = action
            .strip_prefix("tools.technology.")
            .and_then(|value| value.parse::<usize>().ok())
        {
            self.apply_layout_technology_selection(index, true);
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(lot_id) = action.strip_prefix("tools.focus_note.") {
            self.status_message = format!("Added focus note for {lot_id}");
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(slot) = action
            .strip_prefix("bookmarks.save_layout_view.")
            .and_then(parse_layout_view_bookmark_slot)
        {
            self.save_layout_view_bookmark(slot);
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(slot) = action
            .strip_prefix("bookmarks.restore_layout_view.")
            .and_then(parse_layout_view_bookmark_slot)
        {
            self.restore_layout_view_bookmark(slot);
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(slot) = action
            .strip_prefix("bookmarks.clear_layout_view.")
            .and_then(parse_layout_view_bookmark_slot)
        {
            self.clear_layout_view_bookmark(slot);
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if action == "bookmarks.export_layout_views" {
            self.export_layout_view_bookmarks();
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if action == "bookmarks.import_layout_views" {
            self.import_layout_view_bookmarks();
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(index) = action.strip_prefix("file.recent.") {
            self.open_recent_layout_file(index);
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        if let Some(slug) = action.strip_prefix("file.import_layer_policy.")
            && let Some(policy) = LayoutImportLayerPolicy::from_slug(slug)
        {
            self.layout_import_layer_policy = policy;
            self.status_message = format!("Import layer mapping {}", policy.status_label());
            self.active_menu = None;
            self.active_view_group = None;
            return;
        }
        match action {
            "file.new_blank" => {
                self.replace_workspace(WorkspaceDataset::blank(), "New blank workspace loaded");
            }
            "file.load_demo" | "file.new_demo" => {
                self.replace_workspace(WorkspaceDataset::demo(), "Sample workspace loaded");
            }
            "file.save" => {
                self.save_workspace_session();
            }
            "file.load" => {
                self.load_workspace_session();
            }
            "file.save_session" => {
                self.save_app_session();
            }
            "file.load_session" => {
                self.load_app_session();
            }
            "file.save_layout" => {
                self.save_layout_json();
            }
            "file.load_layout" => {
                self.load_layout_json();
            }
            "file.import_layout_cell" => {
                self.import_layout_json_as_cell();
            }
            "file.import_layout_top_cell" => {
                self.import_layout_json_as_top_cell();
            }
            "file.merge_layout" => {
                self.merge_layout_json();
            }
            "file.import_layer_offset.reset" => {
                self.layout_import_layer_id_offset = 0;
                self.status_message = format!(
                    "Import layer ID offset {}",
                    self.layout_import_layer_id_offset_label()
                );
            }
            "file.import_layer_offset.neg" => {
                self.adjust_layout_import_layer_id_offset(-LAYOUT_IMPORT_LAYER_ID_OFFSET_STEP);
            }
            "file.import_layer_offset.pos" => {
                self.adjust_layout_import_layer_id_offset(LAYOUT_IMPORT_LAYER_ID_OFFSET_STEP);
            }
            "file.import_offset.reset" => {
                self.layout_import_offset = Vector::ZERO;
                self.status_message =
                    format!("Import offset {}", self.layout_import_offset_label());
            }
            "file.import_offset.x_neg" => {
                self.adjust_layout_import_offset(-self.layout_import_offset_step(), 0);
            }
            "file.import_offset.x_pos" => {
                self.adjust_layout_import_offset(self.layout_import_offset_step(), 0);
            }
            "file.import_offset.y_neg" => {
                self.adjust_layout_import_offset(0, -self.layout_import_offset_step());
            }
            "file.import_offset.y_pos" => {
                self.adjust_layout_import_offset(0, self.layout_import_offset_step());
            }
            "file.reload_recent" => {
                self.reload_recent_layout_file();
            }
            "file.export_gds" => {
                self.export_layout_gds();
            }
            "file.import_gds" => {
                self.import_layout_gds();
            }
            "file.import_gds_cell" => {
                self.import_layout_gds_as_cell();
            }
            "file.import_gds_top_cell" => {
                self.import_layout_gds_as_top_cell();
            }
            "file.merge_gds" => {
                self.merge_layout_gds();
            }
            "file.export_cif" => {
                self.export_layout_cif();
            }
            "file.import_cif" => {
                self.import_layout_cif();
            }
            "file.import_cif_cell" => {
                self.import_layout_cif_as_cell();
            }
            "file.import_cif_top_cell" => {
                self.import_layout_cif_as_top_cell();
            }
            "file.merge_cif" => {
                self.merge_layout_cif();
            }
            "file.export_dxf" => {
                self.export_layout_dxf();
            }
            "file.import_dxf" => {
                self.import_layout_dxf();
            }
            "file.import_dxf_cell" => {
                self.import_layout_dxf_as_cell();
            }
            "file.import_dxf_top_cell" => {
                self.import_layout_dxf_as_top_cell();
            }
            "file.merge_dxf" => {
                self.merge_layout_dxf();
            }
            "file.export_def" => {
                self.export_layout_def();
            }
            "file.import_def" => {
                self.import_layout_def();
            }
            "file.import_def_cell" => {
                self.import_layout_def_as_cell();
            }
            "file.import_def_top_cell" => {
                self.import_layout_def_as_top_cell();
            }
            "file.merge_def" => {
                self.merge_layout_def();
            }
            "file.export_lef" => {
                self.export_layout_lef();
            }
            "file.import_lef" => {
                self.import_layout_lef();
            }
            "file.export_reference_images" => {
                self.export_layout_reference_images();
            }
            "file.import_reference_images" => {
                self.import_layout_reference_images();
            }
            "file.export_screenshot" => {
                self.export_ui_screenshot();
            }
            "file.export_screenshot_ppm" => {
                self.export_ui_screenshot_ppm();
            }
            "file.collaboration" => {
                self.status_message = "Collaboration connects through the sync server".to_string()
            }
            "edit.undo" => {
                self.undo_layout_operation();
            }
            "edit.redo" => {
                self.redo_layout_operation();
            }
            "edit.copy" => {
                self.copy_layout_selection();
            }
            "edit.paste" => {
                self.paste_layout_clipboard();
            }
            "edit.duplicate" => {
                self.duplicate_layout_selection();
            }
            "edit.delete" => {
                self.delete_layout_selection();
            }
            "edit.make_cell" => {
                self.make_layout_cell_from_selection();
            }
            "edit.duplicate_cell" => {
                self.duplicate_layout_view_cell();
            }
            "edit.delete_cell_shallow" => {
                self.delete_layout_view_cell_shallow();
            }
            "edit.make_variant" => {
                self.make_selected_layout_instance_variant();
            }
            "edit.flatten_instance" => {
                self.flatten_selected_layout_instance(LayoutFlattenDepth::Deep);
            }
            "edit.flatten_instance_one" => {
                self.flatten_selected_layout_instance(LayoutFlattenDepth::OneLevel);
            }
            "edit.flatten_cell" => {
                self.flatten_current_layout_cell(LayoutFlattenDepth::Deep);
            }
            "edit.flatten_cell_one" => {
                self.flatten_current_layout_cell(LayoutFlattenDepth::OneLevel);
            }
            "edit.cell_origin_selection" => {
                self.adjust_current_layout_cell_origin_to_selection();
            }
            "edit.move_shape_up" => {
                self.move_selected_layout_shape_up_hierarchy();
            }
            "edit.move_instance_up" => {
                self.move_selected_layout_instance_up_hierarchy();
            }
            "edit.resolve_array" => {
                self.resolve_selected_layout_instance_array();
            }
            "edit.merge_layer_rects" => {
                self.merge_active_layer_rectangles();
            }
            "edit.layer_and_selection" => {
                self.intersect_active_layer_shapes_with_selection();
            }
            "edit.layer_or_selection" => {
                self.or_active_layer_shapes_with_selection();
            }
            "edit.layer_not_selection" => {
                self.subtract_selection_from_active_layer_rectangles();
            }
            "edit.selection_not_layer" => {
                self.subtract_active_layer_rectangles_from_selection();
            }
            "edit.layer_xor_selection" => {
                self.xor_active_layer_rectangles_with_selection();
            }
            "edit.shape_and_clipboard" => {
                self.boolean_selected_shape_with_clipboard(LayoutShapeClipboardBoolean::And);
            }
            "edit.shape_or_clipboard" => {
                self.boolean_selected_shape_with_clipboard(LayoutShapeClipboardBoolean::Or);
            }
            "edit.shape_not_clipboard" => {
                self.boolean_selected_shape_with_clipboard(LayoutShapeClipboardBoolean::Not);
            }
            "edit.shape_xor_clipboard" => {
                self.boolean_selected_shape_with_clipboard(LayoutShapeClipboardBoolean::Xor);
            }
            "edit.create_clip_cell" => {
                self.create_clip_cell_from_selected_region();
            }
            "edit.rotate90" => {
                self.rotate_layout_selection_90();
            }
            "edit.mirror_x" => {
                self.mirror_layout_selection_x();
            }
            "edit.mirror_y" => {
                self.mirror_layout_selection_y();
            }
            "edit.grow" => {
                self.size_layout_selection(self.layout_size_step());
            }
            "edit.shrink" => {
                self.size_layout_selection(-self.layout_size_step());
            }
            "edit.chamfer_corners" => {
                self.chamfer_selected_layout_shape();
            }
            "edit.round_corners" => {
                self.round_selected_layout_shape();
            }
            "edit.grow_layer" => {
                self.size_active_layer_shapes(self.layout_size_step());
            }
            "edit.shrink_layer" => {
                self.size_active_layer_shapes(-self.layout_size_step());
            }
            "edit.align_left" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::Left);
            }
            "edit.align_right" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::Right);
            }
            "edit.align_top" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::Top);
            }
            "edit.align_bottom" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::Bottom);
            }
            "edit.align_center_x" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::CenterX);
            }
            "edit.align_center_y" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::CenterY);
            }
            "edit.align_origin_x" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::OriginX);
            }
            "edit.align_origin_y" => {
                self.align_selected_layout_shape_to_active_layer_edge(LayoutAlignEdge::OriginY);
            }
            "bookmarks.save_layout_view" => {
                self.save_layout_view_bookmark(1);
            }
            "bookmarks.restore_layout_view" => {
                self.restore_layout_view_bookmark(1);
            }
            "bookmarks.previous_layout_view" => {
                self.restore_previous_layout_view();
            }
            "bookmarks.export_layout_views" => {
                self.export_layout_view_bookmarks();
            }
            "bookmarks.import_layout_views" => {
                self.import_layout_view_bookmarks();
            }
            "bookmarks.origin" => {
                self.focus_layout_origin();
            }
            "bookmarks.bounds" => {
                self.focus_layout_bounds();
            }
            "bookmarks.selection" => {
                self.focus_layout_selection();
            }
            "display.grid2d" | "options.grid" => {
                self.show_grid = !self.show_grid;
                self.status_message = format!(
                    "2D grid {}",
                    if self.show_grid { "shown" } else { "hidden" }
                );
            }
            "display.grid3d" => {
                self.show_3d_grid = !self.show_3d_grid;
                self.status_message = format!(
                    "3D grid {}",
                    if self.show_3d_grid { "shown" } else { "hidden" }
                );
            }
            "display.origin" => {
                self.show_origin_marker = !self.show_origin_marker;
                self.status_message = format!(
                    "Origin crosshair {}",
                    if self.show_origin_marker {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            "display.drc" => {
                self.show_drc_overlay = !self.show_drc_overlay;
                self.status_message = format!(
                    "DRC overlay {}",
                    if self.show_drc_overlay {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            "display.reference_images" => {
                self.toggle_layout_reference_images();
            }
            "display.inspector" => {
                self.toggle_inspector_panel();
                self.active_menu = None;
                self.active_view_group = None;
            }
            "display.secondary_panel" => {
                self.toggle_secondary_panel();
                self.active_menu = None;
                self.active_view_group = None;
            }
            "display.theme.dark" => {
                self.theme_preference = options::ThemePreference::Dark;
                self.dark_theme = true;
                self.status_message = "Theme: dark".to_string();
            }
            "display.theme.light" => {
                self.theme_preference = options::ThemePreference::Light;
                self.dark_theme = false;
                self.status_message = "Theme: light".to_string();
            }
            "display.theme.system" => {
                self.theme_preference = options::ThemePreference::System;
                self.dark_theme = true;
                self.status_message = "Theme: system".to_string();
            }
            "display.options" => {
                self.show_options_panel = true;
                self.status_message = "Options".to_string();
            }
            "display.units" => self.status_message = "Unit display cycled".to_string(),
            "options.snap" => {
                self.snap_enabled = !self.snap_enabled;
                self.status_message = format!(
                    "Snap {}",
                    if self.snap_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            "view.command_palette" | "tools.palette" => {
                self.show_command_palette = true;
                self.status_message = "Command Palette".to_string();
            }
            "view.sidebar_modules" => {
                self.show_sidebar_modules = true;
                self.status_message = "Sidebar Modules".to_string();
            }
            "tools.diagnostics" => {
                self.show_diagnostics_panel = true;
                self.status_message = "Diagnostics".to_string();
            }
            "tools.rundrc" => self.status_message = self.run_drc_summary(),
            "tools.rundrc_region" => {
                self.status_message = self.run_drc_for_selected_region_summary();
            }
            "tools.rundrc_cell" => {
                self.status_message = self.run_drc_for_current_cell_summary();
            }
            "tools.tracebetween" => {
                self.trace_layout_between_route_points();
            }
            "layout.drc_report_export" => {
                self.export_layout_drc_report();
            }
            "layout.drc_report_import" => {
                self.import_layout_drc_report();
            }
            "layout.drc_report_database_export" => {
                self.export_layout_drc_report_database();
            }
            "layout.drc_report_database_import" => {
                self.import_layout_drc_report_database();
            }
            "layout.drc_deck_export" => {
                self.export_layout_drc_deck();
            }
            "layout.drc_deck_import" => {
                self.import_layout_drc_deck();
            }
            "layout.drc_deck_reset" => {
                self.reset_layout_drc_deck();
            }
            "layout.calibre_rve_import" => {
                self.import_layout_calibre_rve_markers();
            }
            "layout.drc_markers.write_layer" => {
                self.write_drc_markers_to_layout_layer();
            }
            "layout.drc_marker_snapshot" => {
                self.export_selected_layout_drc_marker_snapshot();
            }
            "layout.netlist_export" => {
                self.export_layout_netlist();
            }
            "layout.spice_netlist_export" => {
                self.export_layout_spice_netlist();
            }
            "layout.spice_schematic_compare" => {
                self.compare_layout_spice_schematic();
            }
            "layout.netlist_import" => {
                self.import_layout_netlist();
            }
            "layout.trace_state_export" => {
                self.export_layout_trace_state();
            }
            "layout.trace_state_import" => {
                self.import_layout_trace_state();
            }
            "layout.l2n_database_export" => {
                self.export_layout_l2n_database();
            }
            "layout.l2n_database_import" => {
                self.import_layout_l2n_database();
            }
            "layout.reference_images.align" => {
                self.align_layout_reference_images();
            }
            "layout.reference_images.landmarks.seed" => {
                self.seed_layout_reference_image_landmarks();
            }
            "layout.reference_images.landmarks.fit_selection" => {
                self.fit_layout_reference_image_landmarks_to_selection();
            }
            "layout.reference_images.landmarks.clear" => {
                self.clear_layout_reference_image_landmarks();
            }
            "layout.reference_images.export" => {
                self.export_layout_reference_images();
            }
            "layout.reference_images.import" => {
                self.import_layout_reference_images();
            }
            "tools.runroute" => {
                self.route_layout_between_points();
            }
            "tools.reset3d" => {
                self.reset_3d_camera_to_document();
                self.status_message = "3D camera reset".to_string();
            }
            "tools.fullscreen" => {
                self.viewport_fullscreen = !self.viewport_fullscreen;
                self.status_message = if self.viewport_fullscreen {
                    "Viewport fullscreen".to_string()
                } else {
                    "Viewport restored".to_string()
                };
            }
            "macros.validate" => self.status_message = "Workspace validation clean".to_string(),
            "macros.demo" => {
                self.replace_workspace(WorkspaceDataset::demo(), "Full sample workspace loaded");
            }
            "macros.stress10k" => {
                self.workspace.document = Document::stress(10_000);
                self.reset_layout_document_state();
                self.status_message = "10k stress scene loaded".to_string();
            }
            "macros.stress100k" => {
                self.workspace.document = Document::stress(100_000);
                self.reset_layout_document_state();
                self.status_message = "100k stress scene loaded".to_string();
            }
            "macros.stress1m" => {
                self.workspace.document = Document::stress(1_000_000);
                self.reset_layout_document_state();
                self.status_message = "1M stress scene loaded".to_string();
            }
            "macros.hierarchy" => {
                self.workspace.document = Document::hierarchy_demo();
                self.reset_layout_document_state();
                self.status_message = "Hierarchy scene loaded".to_string();
            }
            "macros.drc" => {
                self.status_message = self.run_drc_summary();
            }
            "macros.snapshot" => {
                self.export_ui_screenshot();
            }
            "help.shortcuts" => {
                self.status_message =
                    "Shortcuts: click menus or left nav to switch views".to_string()
            }
            "help.about" => self.status_message = "Mask-layout editor".to_string(),
            _ => self.status_message = action.replace('.', " "),
        }
        self.active_menu = None;
        self.active_view_group = None;
    }

    pub(crate) fn replace_workspace(
        &mut self,
        workspace: WorkspaceDataset,
        status_message: impl Into<String>,
    ) {
        self.workspace = workspace;
        self.reset_layout_document_state();
        self.workflow_focus_lot = default_workflow_focus_lot(&self.workspace);
        self.selected_equipment_tool = default_equipment_tool(&self.workspace);
        self.equipment_recipe_drafts.clear();
        self.selected_maintenance_tool = default_maintenance_tool(&self.workspace);
        self.maintenance_work_filter = MaintenanceWorkFilter::Actionable;
        self.maintenance_history_filter = MaintenanceHistoryFilter::SelectedTool;
        self.selected_environment_sensor = default_environment_sensor(&self.workspace);
        self.selected_inventory_lot =
            default_inventory_lot(&self.workspace, InventoryQuickFilter::All);
        self.inventory_filter = InventoryQuickFilter::All;
        self.mask_source_lot = default_mask_lot(&self.workspace);
        self.mask_issue_severity_filter = MaskIssueSeverityFilter::All;
        self.mask_issue_grouping = MaskIssueGrouping::Code;
        self.mask_issue_page = 0;
        self.layout_diff_baseline = LayoutDiffSource::Demo;
        self.layout_diff_candidate = LayoutDiffSource::Current;
        self.layout_diff_changed_only = true;
        self.layout_diff_change_filter = LayoutChangeFilter::All;
        self.layout_diff_change_page = 0;
        self.layout_diff_page_size = LAYOUT_DIFF_DEFAULT_PAGE_SIZE;
        self.layout_diff_review_state = LayoutReviewDisposition::NeedsReview;
        self.selected_trace_lot = default_trace_lot(&self.workspace);
        self.selected_trace_wafer =
            default_trace_wafer(&self.workspace, self.selected_trace_lot.as_ref());
        self.selected_trace_detail = default_trace_selection(
            &self.workspace,
            self.selected_trace_lot.as_ref(),
            self.selected_trace_wafer.as_ref(),
        );
        self.trace_impact_mode = TraceImpactMode::ToolRun;
        self.trace_related_only = false;
        self.selected_notebook_entry = default_notebook_entry(&self.workspace);
        self.notebook_tag_filter = None;
        self.notebook_link_kind_filter = None;
        self.notebook_preview_mode = true;
        self.notebook_followups_only = false;
        self.scheduler_policy = DispatchPolicy::PriorityThenFifo;
        self.selected_scheduler_tool = default_scheduler_tool(&self.workspace);
        self.scheduler_min_priority = 0;
        self.scheduler_conflicts_only = false;
        self.scheduler_focus_selected_tool = false;
        self.selected_safety_tool = default_safety_tool(&self.workspace);
        self.acknowledged_conditions.clear();
        self.acknowledged_lockouts.clear();
        self.acknowledged_incidents.clear();
        self.selected_process_node = default_process_flow_node(&self.workspace);
        self.process_flow_filter = ProcessFlowNodeFilter::All;
        self.process_flow_errors_only = false;
        self.process_control_loop_filter = ControlLoopFilter::All;
        self.selected_control_loop = default_control_loop(&self.workspace);
        self.selected_control_action =
            default_control_action(&self.workspace, self.selected_control_loop.as_ref());
        self.selected_spc_chart = default_spc_chart(&self.workspace);
        self.selected_fdc_trace = default_fdc_trace(&self.workspace);
        self.spc_severity_filter = SpcSeverityFilter::All;
        self.spc_source_filter = SpcSourceFilter::All;
        self.spc_context_filter.clear();
        self.cross_section_step = 0;
        self.selected_cross_section_material = default_cross_section_material(&self.workspace);
        self.cross_section_show_mask = true;
        self.cross_section_show_dimensions = true;
        self.cross_section_show_risks = true;
        self.selected_die = None;
        self.selected_yield_lot = default_yield_lot(&self.workspace);
        self.selected_yield_wafer =
            default_yield_wafer(&self.workspace, self.selected_yield_lot.as_deref());
        self.status_message = status_message.into();
    }

    pub(crate) fn select_equipment_tool(&mut self, tool_id: &str) -> bool {
        if self
            .workspace
            .equipment
            .tools()
            .any(|tool| tool.id.as_str() == tool_id)
        {
            self.selected_equipment_tool = Some(EquipmentToolId::new(tool_id));
            self.status_message = format!("Selected tool {tool_id}");
            return true;
        }
        false
    }

    pub(crate) fn load_equipment_recipe(&mut self, tool_id: &str, recipe_id: &str) -> bool {
        let tool_id = EquipmentToolId::new(tool_id);
        let recipe_id = EquipmentRecipeId::new(recipe_id);
        let Some(tool) = self.workspace.equipment.tool(&tool_id) else {
            return false;
        };
        if !tool.available_recipes.contains_key(&recipe_id) {
            return false;
        }
        self.selected_equipment_tool = Some(tool_id.clone());
        self.equipment_recipe_drafts
            .insert(tool_id.clone(), recipe_id.clone());
        if !tool.state.accepts_recipe_load() {
            self.status_message = format!("Selected recipe {recipe_id}; tool is not loadable");
            return true;
        }
        let selection = self.equipment_selection_for_tool(tool, recipe_id);
        self.send_equipment_command(&tool_id, HostCommand::LoadRecipe { selection })
    }

    pub(crate) fn apply_equipment_command(&mut self, raw: &str) -> bool {
        let mut parts = raw.split('|');
        let Some(command) = parts.next() else {
            return false;
        };
        let Some(tool_id) = parts.next() else {
            return false;
        };
        let tool_id = EquipmentToolId::new(tool_id);
        let Some(tool) = self.workspace.equipment.tool(&tool_id) else {
            return false;
        };
        self.selected_equipment_tool = Some(tool_id.clone());
        let host_command = match command {
            "online" => HostCommand::BringOnline,
            "load" => {
                let Some(recipe_id) = self.selected_recipe_for_tool(tool) else {
                    self.status_message = format!("{tool_id} has no recipe to load");
                    return true;
                };
                HostCommand::LoadRecipe {
                    selection: self.equipment_selection_for_tool(tool, recipe_id),
                }
            }
            "start" => HostCommand::Start,
            "stop" => HostCommand::Stop,
            "alarm" => HostCommand::TriggerAlarm {
                code: "HOST-SIM".to_string(),
                message: "operator injected simulator alarm".to_string(),
                severity: AlarmSeverity::Warning,
            },
            "clear" => HostCommand::ClearAlarm,
            "reset" => HostCommand::Reset,
            "maintenance" => {
                if tool.state == EquipmentToolState::Maintenance {
                    HostCommand::ExitMaintenance
                } else {
                    HostCommand::EnterMaintenance
                }
            }
            _ => return false,
        };
        self.send_equipment_command(&tool_id, host_command)
    }

    pub(crate) fn send_equipment_command(
        &mut self,
        tool_id: &EquipmentToolId,
        command: HostCommand,
    ) -> bool {
        let label = command.label();
        match self.workspace.equipment.command(tool_id, command) {
            Ok(events) => {
                let state = self
                    .workspace
                    .equipment
                    .tool(tool_id)
                    .map(|tool| tool.state.label())
                    .unwrap_or("unknown");
                let major_events = events
                    .iter()
                    .filter(|event| {
                        !matches!(
                            event,
                            layout_model::equipment::EquipmentEvent::SensorSample { .. }
                        )
                    })
                    .count();
                self.status_message =
                    format!("{tool_id} {label}: {state}, {major_events} event(s)");
                true
            }
            Err(err) => {
                self.status_message = err.to_string();
                true
            }
        }
    }

    pub(crate) fn select_control_loop(&mut self, loop_id: &str) -> bool {
        let loop_id = ControlLoopId::new(loop_id);
        if self
            .workspace
            .process_control
            .loop_by_id(&loop_id)
            .is_some()
        {
            self.selected_control_loop = Some(loop_id.clone());
            self.selected_control_action = default_control_action(&self.workspace, Some(&loop_id));
            self.status_message = format!("Process-control loop {loop_id}");
            return true;
        }
        false
    }

    pub(crate) fn select_control_action(&mut self, action_id: &str) -> bool {
        let action_id = ControlActionId::new(action_id);
        let Some(loop_id) = self
            .workspace
            .process_control
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .map(|action| action.loop_id.clone())
        else {
            return false;
        };
        self.selected_control_loop = Some(loop_id);
        self.selected_control_action = Some(action_id.clone());
        self.status_message = format!("Process-control action {action_id}");
        true
    }

    pub(crate) fn apply_process_control_transition(
        &mut self,
        action_id: &str,
        transition: &str,
    ) -> bool {
        let action_id = ControlActionId::new(action_id);
        let actor = "process.engineer";
        let timestamp = "2026-05-12T12:00:00Z";
        let result = match transition {
            "approve" => self.workspace.process_control.approve_action(
                &action_id,
                actor,
                timestamp,
                "approved from controls",
            ),
            "reject" => self.workspace.process_control.reject_action(
                &action_id,
                actor,
                timestamp,
                "rejected from controls",
            ),
            "apply" => self.workspace.process_control.apply_action(
                &action_id,
                actor,
                timestamp,
                "applied from controls",
            ),
            _ => return false,
        };
        match result {
            Ok(()) => {
                self.selected_control_action = Some(action_id.clone());
                if let Some(loop_id) = self
                    .workspace
                    .process_control
                    .actions
                    .iter()
                    .find(|action| action.id == action_id)
                    .map(|action| action.loop_id.clone())
                {
                    self.selected_control_loop = Some(loop_id);
                }
                self.status_message = format!("Process-control {transition} {action_id}");
            }
            Err(error) => {
                self.status_message = format!("Process-control blocked: {error}");
            }
        }
        true
    }

    pub(crate) fn select_layout_layer(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u32>() else {
            return false;
        };
        let layer_id = LayerId(id);
        let Some(layer) = self.workspace.document.layers.get(&layer_id) else {
            return false;
        };
        self.active_layer = layer_id;
        self.status_message = format!("Active layer L{} {}", layer.id.0, layer.name);
        true
    }

    pub(crate) fn set_layout_hierarchy_depth(&mut self, depth: LayoutHierarchyDepth) -> bool {
        if self.layout_hierarchy_depth == depth {
            self.status_message = format!(
                "Hierarchy display already {}",
                layout_hierarchy_display_label(depth, self.layout_hierarchy_min_depth)
            );
            return true;
        }
        self.record_layout_previous_view();
        self.layout_hierarchy_depth = depth;
        self.layout_hierarchy_min_depth = clamp_layout_hierarchy_min_depth_for_max(
            self.layout_hierarchy_min_depth,
            self.layout_hierarchy_depth,
        );
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Hierarchy display {}",
            layout_hierarchy_display_label(
                self.layout_hierarchy_depth,
                self.layout_hierarchy_min_depth
            )
        );
        true
    }

    pub(crate) fn set_layout_hierarchy_min_depth(&mut self, min_depth: u8) -> bool {
        let min_depth = min_depth.min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH);
        let mut next_depth = self.layout_hierarchy_depth;
        if next_depth
            .max_depth()
            .is_some_and(|max_depth| max_depth < min_depth as usize)
        {
            next_depth = LayoutHierarchyDepth::for_minimum_depth(min_depth as usize);
        }
        if self.layout_hierarchy_min_depth == min_depth && self.layout_hierarchy_depth == next_depth
        {
            self.status_message = format!(
                "Hierarchy minimum already {}",
                layout_hierarchy_min_depth_label(min_depth)
            );
            return true;
        }
        self.record_layout_previous_view();
        self.layout_hierarchy_min_depth = min_depth;
        self.layout_hierarchy_depth = next_depth;
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Hierarchy display {}",
            layout_hierarchy_display_label(
                self.layout_hierarchy_depth,
                self.layout_hierarchy_min_depth
            )
        );
        true
    }

    pub(crate) fn ensure_layout_occurrence_depth_visible(
        &mut self,
        occurrence: &ShapeOccurrenceId,
    ) -> bool {
        let occurrence_depth = occurrence
            .hierarchy_depth()
            .min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH as usize);
        let mut changed = false;
        if self.layout_hierarchy_min_depth as usize > occurrence_depth {
            self.layout_hierarchy_min_depth = occurrence_depth as u8;
            changed = true;
        }
        if self
            .layout_hierarchy_depth
            .max_depth()
            .is_some_and(|limit| occurrence_depth > limit)
        {
            self.layout_hierarchy_depth = LayoutHierarchyDepth::for_minimum_depth(occurrence_depth);
            changed = true;
        }
        if changed {
            self.invalidate_layout_view_caches();
        }
        changed
    }

    pub(crate) fn set_layout_view_top_cell(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u64>() else {
            return false;
        };
        let cell_id = CellId(id);
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            return false;
        };
        if self.layout_view_top_cell == cell_id {
            self.status_message = format!("View top cell already {}", cell.name);
            return true;
        }
        let name = cell.name.clone();
        self.record_layout_previous_view();
        self.layout_view_top_cell = cell_id;
        self.layout_hidden_cells.remove(&cell_id);
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!("View top cell {name}");
        true
    }

    pub(crate) fn selected_layout_instance_context(
        &self,
    ) -> Option<(CellId, InstanceId, CellInstance)> {
        let occurrence = self.selected_layout_occurrence.as_ref()?;
        let (parent, id) = self.workspace.document.instance_parent_for_path_from_cell(
            self.layout_view_top_cell,
            &occurrence.instance_path,
        )?;
        let instance = self.workspace.document.instance(parent, id)?;
        Some((parent, id, instance))
    }

    pub(crate) fn descend_selected_layout_instance(&mut self) -> bool {
        let Some((parent, id, instance)) = self.selected_layout_instance_context() else {
            self.status_message = "Select an instance occurrence to descend".to_string();
            return true;
        };
        let Some(child_cell) = self.workspace.document.cell(instance.cell) else {
            return false;
        };
        let child_name = child_cell.name.clone();
        if self.layout_view_top_cell == instance.cell {
            self.status_message = format!("Already viewing cell {child_name}");
            return true;
        }

        let selected_shape = self.selected_layout_occurrence.as_ref().map(|occurrence| {
            (
                occurrence.source_shape_id(),
                child_cell
                    .shapes
                    .contains_key(&occurrence.source_shape_id()),
            )
        });
        self.record_layout_previous_view();
        self.layout_view_top_cell = instance.cell;
        self.layout_hierarchy_min_depth = 0;
        self.layout_hidden_cells.remove(&instance.cell);
        if let Some((shape_id, shape_is_local)) = selected_shape {
            if shape_is_local {
                self.selected_layout_shape = Some(shape_id);
                self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
            } else {
                self.selected_layout_shape = None;
                self.selected_layout_occurrence = None;
            }
        }
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Descended into instance #{} from C{}: {}",
            id.0, parent.0, child_name
        );
        true
    }

    pub(crate) fn delete_unused_layout_view_cell(&mut self) -> bool {
        let cell_id = self.layout_view_top_cell;
        if cell_id == self.workspace.document.top_cell {
            self.status_message = "Document top cell cannot be deleted".to_string();
            return true;
        }
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let references = layout_cell_instance_refs(&self.workspace.document, cell_id);
        if !references.is_empty() {
            self.status_message = format!(
                "Cell {} is still instanced {} time{}",
                cell.name,
                references.len(),
                if references.len() == 1 { "" } else { "s" }
            );
            return true;
        }
        self.apply_layout_operation_with_history(
            Operation::DeleteCell { id: cell_id },
            Operation::AddCell { cell: cell.clone() },
        );
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        self.layout_hidden_cells.remove(&cell_id);
        self.layout_tree_collapsed_cells.remove(&cell_id);
        self.status_message = format!("Deleted unused cell {}", cell.name);
        true
    }

    pub(crate) fn delete_layout_view_cell_shallow(&mut self) -> bool {
        let cell_id = self.layout_view_top_cell;
        if cell_id == self.workspace.document.top_cell {
            self.status_message = "Document top cell cannot be deleted".to_string();
            return true;
        }
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };

        let references = layout_cell_instance_refs(&self.workspace.document, cell_id)
            .into_iter()
            .filter_map(|(parent, id)| {
                self.workspace
                    .document
                    .instance(parent, id)
                    .map(|instance| (parent, instance))
            })
            .collect::<Vec<_>>();
        let reference_count = references.len();
        let mut redo = references
            .iter()
            .map(|(parent, instance)| Operation::DeleteInstance {
                parent: *parent,
                id: instance.id,
            })
            .collect::<Vec<_>>();
        redo.push(Operation::DeleteCell { id: cell_id });
        let mut undo = Vec::with_capacity(reference_count + 1);
        undo.push(Operation::AddCell { cell: cell.clone() });
        undo.extend(
            references
                .iter()
                .map(|(parent, instance)| Operation::AddInstance {
                    parent: *parent,
                    instance: instance.clone(),
                }),
        );

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        self.layout_hidden_cells.remove(&cell_id);
        self.layout_tree_collapsed_cells.remove(&cell_id);
        self.status_message = format!(
            "Deleted cell {} and {} reference{}",
            cell.name,
            reference_count,
            if reference_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn delete_layout_view_cell_deep(&mut self) -> bool {
        let cell_id = self.layout_view_top_cell;
        if cell_id == self.workspace.document.top_cell {
            self.status_message = "Document top cell cannot be deleted".to_string();
            return true;
        }
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };

        let delete_cells = layout_deep_delete_cell_ids(&self.workspace.document, cell_id);
        let deleted_cells = delete_cells
            .iter()
            .filter_map(|cell_id| self.workspace.document.cell(*cell_id).cloned())
            .collect::<Vec<_>>();
        if deleted_cells.is_empty() {
            self.status_message = "No current-cell subtree was available to delete".to_string();
            return true;
        }

        let mut removed_references = Vec::new();
        for target_cell in &delete_cells {
            for (parent, id) in layout_cell_instance_refs(&self.workspace.document, *target_cell) {
                if !delete_cells.contains(&parent)
                    && let Some(instance) = self.workspace.document.instance(parent, id)
                {
                    removed_references.push((parent, instance));
                }
            }
        }
        removed_references.sort_by_key(|(parent, instance)| (*parent, instance.id));

        let mut redo = removed_references
            .iter()
            .map(|(parent, instance)| Operation::DeleteInstance {
                parent: *parent,
                id: instance.id,
            })
            .collect::<Vec<_>>();
        redo.extend(
            delete_cells
                .iter()
                .map(|cell_id| Operation::DeleteCell { id: *cell_id }),
        );
        let mut undo = deleted_cells
            .iter()
            .cloned()
            .map(|cell| Operation::AddCell { cell })
            .collect::<Vec<_>>();
        undo.extend(
            removed_references
                .iter()
                .map(|(parent, instance)| Operation::AddInstance {
                    parent: *parent,
                    instance: instance.clone(),
                }),
        );

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        for deleted_cell in &delete_cells {
            self.layout_hidden_cells.remove(deleted_cell);
            self.layout_tree_collapsed_cells.remove(deleted_cell);
        }
        self.status_message = format!(
            "Deep deleted cell {}: {} cell{}, {} reference{}",
            cell.name,
            deleted_cells.len(),
            if deleted_cells.len() == 1 { "" } else { "s" },
            removed_references.len(),
            if removed_references.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        true
    }

    pub(crate) fn delete_layout_view_cell_complete(&mut self) -> bool {
        let cell_id = self.layout_view_top_cell;
        if cell_id == self.workspace.document.top_cell {
            self.status_message = "Document top cell cannot be deleted".to_string();
            return true;
        }
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };

        let mut delete_cells = layout_descendant_cell_ids(&self.workspace.document, cell_id);
        delete_cells.remove(&self.workspace.document.top_cell);
        let deleted_cells = delete_cells
            .iter()
            .filter_map(|cell_id| self.workspace.document.cell(*cell_id).cloned())
            .collect::<Vec<_>>();
        if deleted_cells.is_empty() {
            self.status_message = "No current-cell subtree was available to delete".to_string();
            return true;
        }

        let mut removed_references = Vec::new();
        for target_cell in &delete_cells {
            for (parent, id) in layout_cell_instance_refs(&self.workspace.document, *target_cell) {
                if !delete_cells.contains(&parent)
                    && let Some(instance) = self.workspace.document.instance(parent, id)
                {
                    removed_references.push((parent, instance));
                }
            }
        }
        removed_references.sort_by_key(|(parent, instance)| (*parent, instance.id));

        let mut redo = removed_references
            .iter()
            .map(|(parent, instance)| Operation::DeleteInstance {
                parent: *parent,
                id: instance.id,
            })
            .collect::<Vec<_>>();
        redo.extend(
            delete_cells
                .iter()
                .map(|cell_id| Operation::DeleteCell { id: *cell_id }),
        );
        let mut undo = deleted_cells
            .iter()
            .cloned()
            .map(|cell| Operation::AddCell { cell })
            .collect::<Vec<_>>();
        undo.extend(
            removed_references
                .iter()
                .map(|(parent, instance)| Operation::AddInstance {
                    parent: *parent,
                    instance: instance.clone(),
                }),
        );

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        for deleted_cell in &delete_cells {
            self.layout_hidden_cells.remove(deleted_cell);
            self.layout_tree_collapsed_cells.remove(deleted_cell);
        }
        self.status_message = format!(
            "Complete deleted cell {}: {} cell{}, {} reference{}",
            cell.name,
            deleted_cells.len(),
            if deleted_cells.len() == 1 { "" } else { "s" },
            removed_references.len(),
            if removed_references.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        true
    }

    pub(crate) fn duplicate_layout_view_cell(&mut self) -> bool {
        let source_id = self.layout_view_top_cell;
        let Some(source_cell) = self.workspace.document.cell(source_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };

        let duplicate_id = self.workspace.document.allocate_cell_id();
        let mut duplicate = Cell::new(
            duplicate_id,
            format!("{} copy {}", source_cell.name, duplicate_id.0),
        );
        duplicate.properties = source_cell.properties.clone();
        let mut shape_map = BTreeMap::new();
        let mut source_shapes = source_cell.shapes.values().collect::<Vec<_>>();
        if source_id == self.workspace.document.top_cell {
            source_shapes.extend(self.workspace.document.shapes.values());
        }
        source_shapes.sort_by_key(|shape| shape.id);
        for mut shape in source_shapes {
            let source_shape_id = shape.id;
            shape.id = self.workspace.document.allocate_shape_id();
            shape_map.insert(source_shape_id, shape.id);
            duplicate.shapes.insert(shape.id, shape);
        }

        let mut source_instances = source_cell.instances.values().collect::<Vec<_>>();
        source_instances.sort_by_key(|instance| instance.id);
        for mut instance in source_instances {
            instance.id = self.workspace.document.allocate_instance_id();
            if instance.cell == source_id {
                instance.cell = duplicate_id;
            }
            duplicate.instances.insert(instance.id, instance);
        }
        let copied_shape = self
            .selected_layout_occurrence
            .as_ref()
            .filter(|occurrence| occurrence.instance_path.is_empty())
            .and_then(|occurrence| shape_map.get(&occurrence.source_shape_id()).copied());

        self.record_layout_previous_view();
        self.apply_layout_operation_with_history(
            Operation::AddCell {
                cell: duplicate.clone(),
            },
            Operation::DeleteCell { id: duplicate_id },
        );
        self.layout_view_top_cell = duplicate_id;
        self.layout_hierarchy_min_depth = 0;
        self.layout_hidden_cells.remove(&duplicate_id);
        self.invalidate_layout_view_caches();
        self.selected_layout_shape = copied_shape;
        self.selected_layout_occurrence = copied_shape.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Duplicated cell {} as {} ({} shape(s), {} instance(s))",
            source_cell.name,
            duplicate.name,
            duplicate.shapes.len(),
            duplicate.instances.len()
        );
        true
    }

    pub(crate) fn toggle_layout_tree_cell(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u64>() else {
            return false;
        };
        let cell_id = CellId(id);
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            return false;
        };
        let collapsed = if !self.layout_tree_collapsed_cells.remove(&cell_id) {
            self.layout_tree_collapsed_cells.insert(cell_id);
            true
        } else {
            false
        };
        self.status_message = format!(
            "{} {}",
            if collapsed { "Collapsed" } else { "Expanded" },
            cell.name
        );
        true
    }

    pub(crate) fn expand_all_layout_tree_cells(&mut self) -> bool {
        let collapsed = self.layout_tree_collapsed_cells.len();
        self.layout_tree_collapsed_cells.clear();
        self.status_message = if collapsed == 0 {
            "Hierarchy tree already expanded".to_string()
        } else {
            format!(
                "Expanded {} hierarchy tree cell{}",
                collapsed,
                if collapsed == 1 { "" } else { "s" }
            )
        };
        true
    }

    pub(crate) fn collapse_all_layout_tree_cells(&mut self) -> bool {
        let branch_cells = layout_hierarchy_branch_cells(&self.workspace.document);
        let collapsed = branch_cells.len();
        self.layout_tree_collapsed_cells = branch_cells;
        self.status_message = if collapsed == 0 {
            "No hierarchy tree cells to collapse".to_string()
        } else {
            format!(
                "Collapsed {} hierarchy tree cell{}",
                collapsed,
                if collapsed == 1 { "" } else { "s" }
            )
        };
        true
    }

    pub(crate) fn toggle_layout_cell_visibility(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u64>() else {
            return false;
        };
        let cell_id = CellId(id);
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            return false;
        };
        let cell_name = cell.name.clone();
        if cell_id == self.workspace.document.top_cell || cell_id == self.layout_view_top_cell {
            self.status_message = format!("{cell_name} is the active view root");
            return true;
        }
        let hidden = if !self.layout_hidden_cells.remove(&cell_id) {
            self.layout_hidden_cells.insert(cell_id);
            true
        } else {
            false
        };
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Cell {} {}",
            cell_name,
            if hidden { "hidden" } else { "shown" }
        );
        true
    }

    pub(crate) fn show_all_layout_cells(&mut self) -> bool {
        let hidden_count = self.layout_hidden_cells.len();
        if hidden_count == 0 {
            self.status_message = "All cells are already shown".to_string();
            return true;
        }
        self.layout_hidden_cells.clear();
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Shown {} hidden cell{}",
            hidden_count,
            if hidden_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn set_layout_child_cells_visibility(&mut self, hidden: bool) -> bool {
        let Some(current_cell) = self.workspace.document.cell(self.layout_view_top_cell) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let current_name = current_cell.name.clone();
        let child_cells = current_cell
            .instances
            .values()
            .map(|instance| instance.cell)
            .filter(|cell| {
                *cell != self.workspace.document.top_cell && *cell != self.layout_view_top_cell
            })
            .collect::<BTreeSet<_>>();
        if child_cells.is_empty() {
            self.status_message = format!("{current_name} has no child cells to show or hide");
            return true;
        }

        let mut changed = 0;
        for cell in child_cells {
            if hidden {
                if self.layout_hidden_cells.insert(cell) {
                    changed += 1;
                }
            } else if self.layout_hidden_cells.remove(&cell) {
                changed += 1;
            }
        }
        if changed == 0 {
            self.status_message = format!(
                "Child cells of {} are already {}",
                current_name,
                if hidden { "hidden" } else { "shown" }
            );
            return true;
        }

        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "{} {} child cell{} of {}",
            if hidden { "Hid" } else { "Showed" },
            changed,
            if changed == 1 { "" } else { "s" },
            current_name
        );
        true
    }

    pub(crate) fn set_layout_descendant_cells_visibility(&mut self, hidden: bool) -> bool {
        let Some(current_cell) = self.workspace.document.cell(self.layout_view_top_cell) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let current_name = current_cell.name.clone();
        let descendant_cells =
            layout_descendant_cell_ids(&self.workspace.document, self.layout_view_top_cell)
                .into_iter()
                .filter(|cell| {
                    *cell != self.workspace.document.top_cell && *cell != self.layout_view_top_cell
                })
                .collect::<BTreeSet<_>>();
        if descendant_cells.is_empty() {
            self.status_message = format!("{current_name} has no descendant cells to show or hide");
            return true;
        }

        let mut changed = 0;
        for cell in descendant_cells {
            if hidden {
                if self.layout_hidden_cells.insert(cell) {
                    changed += 1;
                }
            } else if self.layout_hidden_cells.remove(&cell) {
                changed += 1;
            }
        }
        if changed == 0 {
            self.status_message = format!(
                "Descendant cells of {} are already {}",
                current_name,
                if hidden { "hidden" } else { "shown" }
            );
            return true;
        }

        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "{} {} descendant cell{} of {}",
            if hidden { "Hid" } else { "Showed" },
            changed,
            if changed == 1 { "" } else { "s" },
            current_name
        );
        true
    }

    pub(crate) fn toggle_layout_layer_visibility(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u32>() else {
            return false;
        };
        let layer_id = LayerId(id);
        let Some(layer) = self.workspace.document.layers.get(&layer_id) else {
            return false;
        };
        let visible = !layer.visible;
        let name = layer.name.clone();
        self.apply_layout_operation_with_history(
            Operation::SetLayerVisibility {
                layer: layer_id,
                visible,
            },
            Operation::SetLayerVisibility {
                layer: layer_id,
                visible: layer.visible,
            },
        );
        self.status_message = format!("Layer {name} {}", if visible { "shown" } else { "hidden" });
        true
    }

    pub(crate) fn apply_layout_layer_visibility_preset(&mut self, mode: &str) -> bool {
        let usage = layout_layer_usage_counts(&self.workspace.document);
        let mut operations = Vec::new();
        let mut undo_operations = Vec::new();
        let mut layers = self.workspace.document.layers.values().collect::<Vec<_>>();
        layers.sort_by_key(|layer| (layer.display_order, layer.id));
        for layer in layers {
            let used = usage.get(&layer.id).copied().unwrap_or(0) > 0;
            let visible = match mode {
                "show_all" => true,
                "show_used" => used,
                "isolate_active" => layer.id == self.active_layer,
                "invert" => !layer.visible,
                "hide_empty" => {
                    if used {
                        layer.visible
                    } else {
                        false
                    }
                }
                _ => return false,
            };
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

        let label = match mode {
            "show_all" => "Show all layers",
            "show_used" => "Show used layers",
            "isolate_active" => "Isolate active layer",
            "invert" => "Invert layer visibility",
            "hide_empty" => "Hide empty layers",
            _ => return false,
        };
        if operations.is_empty() {
            self.status_message = format!("{label}: no layer visibility changes");
            return true;
        }
        let changed_count = operations.len();
        self.apply_layout_operation_with_history(
            Operation::Batch { operations },
            Operation::Batch {
                operations: undo_operations.into_iter().rev().collect(),
            },
        );
        self.status_message = format!("{label}: changed {changed_count} layer(s)");
        true
    }
}
