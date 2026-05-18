#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn apply_view_control_action(&mut self, action: &str) -> bool {
        if action.starts_with("experiment.") {
            self.ensure_experiment_selection();
        }

        if let Some(slug) = action.strip_prefix("layout.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(layer_id) = action.strip_prefix("layout.layer.") {
            return self.select_layout_layer(layer_id);
        }

        if let Some(layer_id) = action.strip_prefix("layout.toggle_layer.") {
            return self.toggle_layout_layer_visibility(layer_id);
        }

        if let Some(mode) = action.strip_prefix("layout.layer_visibility.") {
            return self.apply_layout_layer_visibility_preset(mode);
        }

        if action == "layout.layer_cleanup.delete_empty" {
            return self.delete_inactive_empty_layout_layers();
        }

        if let Some(group) = action.strip_prefix("layout.layer_group.") {
            return self.set_layout_layer_group_filter(group);
        }

        if let Some(filter) = action.strip_prefix("layout.layer_usage_filter.") {
            return self.set_layout_layer_usage_filter(filter);
        }

        if let Some(depth) = action.strip_prefix("layout.layer_group_depth.") {
            return self.set_layout_layer_group_hierarchy_depth(depth);
        }

        if action == "layout.layer_fill_style.cycle" {
            return self.cycle_active_layout_layer_fill_style();
        }

        if action == "layout.layer_line_style.cycle" {
            return self.cycle_active_layout_layer_line_style();
        }

        if let Some(depth) = action.strip_prefix("layout.layer_depth.") {
            return self.set_active_layout_layer_hierarchy_depth(depth);
        }

        if let Some(slot) = action.strip_prefix("layout.layer_set.save.") {
            return self.save_layout_layer_set(slot);
        }

        if let Some(slot) = action.strip_prefix("layout.layer_set.restore.") {
            return self.restore_layout_layer_set(slot);
        }

        if let Some(slot) = action.strip_prefix("layout.layer_set.tab.") {
            return self.restore_layout_layer_set(slot);
        }

        if action == "layout.layer_set.export" {
            return self.export_layout_layer_sets();
        }

        if action == "layout.layer_set.import" {
            return self.import_layout_layer_sets();
        }

        if let Some(slug) = action.strip_prefix("layout.library_macro.") {
            return self.create_layout_library_macro(slug);
        }

        if let Some(shape_id) = action.strip_prefix("layout.shape.") {
            return self.select_layout_shape(shape_id);
        }

        if let Some(value) = action.strip_prefix("layout.occurrence.") {
            return self.select_layout_occurrence(value);
        }

        if let Some(value) = action.strip_prefix("layout.measurement.") {
            return self.select_layout_occurrence(value);
        }

        if action == "layout.measurement_browser.select_first" {
            return self.select_first_layout_measurement_browser_match();
        }

        if action == "layout.measurements.clear" {
            return self.clear_visible_layout_measurements();
        }

        if action == "layout.measurements.delete_selected" {
            return self.delete_selected_layout_measurement();
        }

        if action == "layout.measurements.focus_selected" {
            return self.focus_selected_layout_measurement();
        }

        if let Some(mode) = action.strip_prefix("layout.measurement_mode.") {
            return self.set_layout_measurement_mode(mode);
        }

        if let Some(filter) = action.strip_prefix("layout.measurement_filter.") {
            return self.set_layout_measurement_filter(filter);
        }

        if let Some(value) = action.strip_prefix("layout.instance.") {
            return self.select_layout_instance(value);
        }

        if let Some(slug) = action.strip_prefix("layout.instance_browser_scope.") {
            if let Some(scope) = LayoutInstanceBrowserScope::from_slug(slug) {
                self.layout_instance_browser_scope = scope;
                self.status_message = format!("Instance browser scope {}", scope.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.instance_browser_filter.") {
            if let Some(filter) = LayoutInstanceBrowserFilter::from_slug(slug) {
                self.layout_instance_browser_filter = filter;
                self.status_message = format!("Instance browser filter {}", filter.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.instance_browser_sort.") {
            if let Some(sort) = LayoutInstanceBrowserSort::from_slug(slug) {
                self.layout_instance_browser_sort = sort;
                self.status_message = format!("Instance browser sort {}", sort.label());
                return true;
            }
        }

        if action == "layout.instance_browser.select_first" {
            return self.select_first_layout_instance_browser_match();
        }

        if let Some(direction) = action.strip_prefix("layout.hierarchy_step.") {
            let depth = match direction {
                "less" | "up" | "shallower" => self.layout_hierarchy_depth.shallower(),
                "more" | "down" | "deeper" => self.layout_hierarchy_depth.deeper(),
                _ => return false,
            };
            return self.set_layout_hierarchy_depth(depth);
        }

        if let Some(slug) = action.strip_prefix("layout.hierarchy.") {
            if let Some(depth) = LayoutHierarchyDepth::from_slug(slug) {
                return self.set_layout_hierarchy_depth(depth);
            }
        }

        if let Some(direction) = action.strip_prefix("layout.hierarchy_min_step.") {
            let depth = match direction {
                "less" | "up" | "shallower" => self.layout_hierarchy_min_depth.saturating_sub(1),
                "more" | "down" | "deeper" => self
                    .layout_hierarchy_min_depth
                    .saturating_add(1)
                    .min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH),
                _ => return false,
            };
            return self.set_layout_hierarchy_min_depth(depth);
        }

        if let Some(slug) = action.strip_prefix("layout.hierarchy_min.") {
            if let Some(depth) = parse_layout_hierarchy_min_depth(slug) {
                return self.set_layout_hierarchy_min_depth(depth);
            }
        }

        if let Some(value) = action.strip_prefix("layout.top_cell.") {
            return self.set_layout_view_top_cell(value);
        }

        if let Some(value) = action.strip_prefix("layout.tree_cell.") {
            return self.set_layout_view_top_cell(value);
        }

        if let Some(value) = action.strip_prefix("layout.tree_toggle.") {
            return self.toggle_layout_tree_cell(value);
        }

        match action {
            "layout.tree_expand_all" => return self.expand_all_layout_tree_cells(),
            "layout.tree_collapse_all" => return self.collapse_all_layout_tree_cells(),
            "layout.cell_visibility.show_all" => return self.show_all_layout_cells(),
            "layout.cell_visibility.hide_children" => {
                return self.set_layout_child_cells_visibility(true);
            }
            "layout.cell_visibility.show_children" => {
                return self.set_layout_child_cells_visibility(false);
            }
            "layout.cell_visibility.hide_descendants" => {
                return self.set_layout_descendant_cells_visibility(true);
            }
            "layout.cell_visibility.show_descendants" => {
                return self.set_layout_descendant_cells_visibility(false);
            }
            _ => {}
        }

        if let Some(value) = action.strip_prefix("layout.cell_visibility.") {
            return self.toggle_layout_cell_visibility(value);
        }

        if let Some(value) = action.strip_prefix("layout.net_component.") {
            return self.select_layout_net_component(value);
        }

        if action == "layout.connectivity_links.enable_all" {
            return self.clear_disabled_connectivity_links();
        }

        if let Some(value) = action.strip_prefix("layout.connectivity_link.toggle.") {
            return self.toggle_layout_connectivity_link(value);
        }

        if action == "layout.trace_history.clear" {
            return self.clear_layout_trace_history();
        }

        if action == "layout.trace_history.select_first" {
            return self.select_first_layout_trace_history_match();
        }

        if let Some(value) = action.strip_prefix("layout.trace_history.") {
            return self.select_layout_net_component(value);
        }

        if action == "layout.browser_search.start" {
            return self.begin_layout_browser_search();
        }

        if action == "layout.browser_search.clear" {
            return self.clear_layout_browser_search();
        }

        if action == "layout.browser_replace.start" {
            return self.begin_layout_browser_replace();
        }

        if let Some(query) = action.strip_prefix("layout.browser_search.set.") {
            self.set_layout_browser_search(query.replace('_', " "));
            self.layout_browser_search_active = false;
            return true;
        }

        if let Some(replacement) = action.strip_prefix("layout.browser_replace.set.") {
            self.set_layout_browser_replace(replacement.replace('_', " "));
            self.layout_browser_replace_active = false;
            return true;
        }

        if action == "layout.browser_replace.apply" {
            return self.replace_layout_browser_search_matches();
        }

        if action == "layout.shape_property.apply_selected" {
            return self.apply_selected_layout_shape_custom_property();
        }

        if action == "layout.shape_property.remove_selected" {
            return self.remove_selected_layout_shape_custom_property();
        }

        if action == "layout.instance_property.apply_selected" {
            return self.apply_selected_layout_instance_custom_property();
        }

        if action == "layout.instance_property.remove_selected" {
            return self.remove_selected_layout_instance_custom_property();
        }

        if action == "layout.cell_property.apply_current" {
            return self.apply_layout_view_cell_custom_property();
        }

        if action == "layout.cell_property.remove_current" {
            return self.remove_layout_view_cell_custom_property();
        }

        if let Some(slug) = action.strip_prefix("layout.browser_replace_scope.") {
            if let Some(scope) = LayoutBrowserReplaceScope::from_slug(slug) {
                self.layout_browser_replace_scope = scope;
                self.status_message = format!("Browser replace scope {}", scope.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.browser_columns.") {
            if let Some(columns) = LayoutBrowserColumnSet::from_slug(slug) {
                self.layout_browser_columns = columns;
                self.status_message = format!("Browser columns {}", columns.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.shape_browser_filter.") {
            if let Some(filter) = LayoutShapeBrowserFilter::from_slug(slug) {
                self.layout_shape_browser_filter = filter;
                self.status_message = format!("Shape browser filter {}", filter.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.shape_browser_sort.") {
            if let Some(sort) = LayoutShapeBrowserSort::from_slug(slug) {
                self.layout_shape_browser_sort = sort;
                self.status_message = format!("Shape browser sort {}", sort.label());
                return true;
            }
        }

        if action == "layout.shape_browser.select_first" {
            return self.select_first_layout_shape_browser_match();
        }

        if let Some(slug) = action.strip_prefix("layout.cell_browser_filter.") {
            if let Some(filter) = LayoutCellBrowserFilter::from_slug(slug) {
                self.layout_cell_browser_filter = filter;
                self.status_message = format!("Cell browser filter {}", filter.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.cell_browser_sort.") {
            if let Some(sort) = LayoutCellBrowserSort::from_slug(slug) {
                self.layout_cell_browser_sort = sort;
                self.status_message = format!("Cell browser sort {}", sort.label());
                return true;
            }
        }

        if action == "layout.cell_browser.select_first" {
            return self.select_first_layout_cell_browser_match();
        }

        if let Some(slug) = action.strip_prefix("layout.net_browser_filter.") {
            if let Some(filter) = LayoutNetBrowserFilter::from_slug(slug) {
                self.layout_net_browser_filter = filter;
                self.status_message = format!("Net browser filter {}", filter.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.net_browser_sort.") {
            if let Some(sort) = LayoutNetBrowserSort::from_slug(slug) {
                self.layout_net_browser_sort = sort;
                self.status_message = format!("Net browser sort {}", sort.label());
                return true;
            }
        }

        if action == "layout.net_browser.select_first" {
            return self.select_first_layout_net_browser_match();
        }

        if let Some(slug) = action.strip_prefix("layout.trace_highlight.") {
            if let Some(mode) = LayoutTraceHighlightMode::from_slug(slug) {
                self.layout_trace_highlight_mode = mode;
                self.invalidate_layout_view_caches();
                self.status_message = format!("Trace highlight {}", mode.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.drc_marker_filter.") {
            if let Some(filter) = LayoutDrcMarkerFilter::from_slug(slug) {
                self.layout_drc_marker_filter = filter;
                self.status_message = format!("DRC marker filter {}", filter.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.drc_marker_sort.") {
            if let Some(sort) = LayoutDrcMarkerSort::from_slug(slug) {
                self.layout_drc_marker_sort = sort;
                self.status_message = format!("DRC marker sort {}", sort.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout.drc_marker_category.") {
            if slug == "all" {
                self.layout_drc_marker_category_filter = None;
                self.status_message = "DRC marker category All".to_string();
                return true;
            }
            if let Some(category) = LayoutDrcMarkerCategory::from_slug(slug) {
                self.layout_drc_marker_category_filter = Some(category);
                self.status_message = format!("DRC marker category {}", category.label());
                return true;
            }
        }

        if let Some(value) = action.strip_prefix("layout.drc_report.select.") {
            return self.select_layout_drc_report(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_report.delete.") {
            return self.delete_layout_drc_report(value);
        }

        if action == "layout.drc_reports.clear" {
            return self.clear_layout_drc_reports();
        }

        if action == "layout.drc_marker_browser.select_first" {
            return self.select_first_layout_drc_marker_browser_match();
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker.") {
            return self.select_layout_drc_marker(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker_toggle.") {
            return self.toggle_selected_layout_drc_marker_state(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker_note.") {
            return self.set_selected_layout_drc_marker_note(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker_owner.") {
            return self.set_selected_layout_drc_marker_owner(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker_signoff.") {
            return self.set_selected_layout_drc_marker_signoff(value);
        }

        if let Some(value) = action.strip_prefix("layout.drc_marker_tag.") {
            return self.set_selected_layout_drc_marker_tag(value);
        }

        if action == "layout.drc_marker_custom_tag.apply" {
            return self.apply_selected_layout_drc_marker_custom_tag();
        }

        if action == "layout.drc_marker_custom_tag.remove" {
            return self.remove_selected_layout_drc_marker_custom_tag();
        }

        if action == "layout.drc_marker_snapshot" {
            return self.export_selected_layout_drc_marker_snapshot();
        }

        if action == "layout.drc_marker_clear_state" {
            return self.clear_selected_layout_drc_marker_state();
        }

        if action == "layout.drc_markers.write_layer" {
            return self.write_drc_markers_to_layout_layer();
        }

        if action == "layout.drc_report_export" {
            return self.export_layout_drc_report();
        }

        if action == "layout.drc_report_import" {
            return self.import_layout_drc_report();
        }

        if action == "layout.drc_report_database_export" {
            return self.export_layout_drc_report_database();
        }

        if action == "layout.drc_report_database_import" {
            return self.import_layout_drc_report_database();
        }

        if action == "layout.drc_deck_export" {
            return self.export_layout_drc_deck();
        }

        if action == "layout.drc_deck_import" {
            return self.import_layout_drc_deck();
        }

        if action == "layout.drc_deck_reset" {
            return self.reset_layout_drc_deck();
        }

        if action == "layout.calibre_rve_import" {
            return self.import_layout_calibre_rve_markers();
        }

        if action == "layout.netlist_export" {
            return self.export_layout_netlist();
        }

        if action == "layout.spice_netlist_export" {
            return self.export_layout_spice_netlist();
        }

        if action == "layout.spice_schematic_compare" {
            return self.compare_layout_spice_schematic();
        }

        if action == "layout.netlist_import" {
            return self.import_layout_netlist();
        }

        if action == "layout.trace_state_export" {
            return self.export_layout_trace_state();
        }

        if action == "layout.trace_state_import" {
            return self.import_layout_trace_state();
        }

        if action == "layout.l2n_database_export" {
            return self.export_layout_l2n_database();
        }

        if action == "layout.l2n_database_import" {
            return self.import_layout_l2n_database();
        }

        if action == "layout.reference_images.toggle" {
            return self.toggle_layout_reference_images();
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.toggle.") {
            return self.toggle_layout_reference_image(index);
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.opacity_less.") {
            return self.adjust_layout_reference_image_opacity(
                index,
                -(REFERENCE_IMAGE_OPACITY_STEP as i16),
            );
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.opacity_more.") {
            return self
                .adjust_layout_reference_image_opacity(index, REFERENCE_IMAGE_OPACITY_STEP as i16);
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.move_up.") {
            return self.move_layout_reference_image(index, -1);
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.move_down.") {
            return self.move_layout_reference_image(index, 1);
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.focus.") {
            return self.focus_layout_reference_image(index);
        }

        if let Some(index) = action.strip_prefix("layout.reference_image.remove.") {
            return self.remove_layout_reference_image(index);
        }

        if action == "layout.reference_images.align" {
            return self.align_layout_reference_images();
        }

        if action == "layout.reference_images.landmarks.seed" {
            return self.seed_layout_reference_image_landmarks();
        }

        if action == "layout.reference_images.landmarks.fit_selection" {
            return self.fit_layout_reference_image_landmarks_to_selection();
        }

        if action == "layout.reference_images.landmarks.clear" {
            return self.clear_layout_reference_image_landmarks();
        }

        if action == "layout.reference_images.export" {
            return self.export_layout_reference_images();
        }

        if action == "layout.reference_images.import" {
            return self.import_layout_reference_images();
        }

        if let Some(value) = action.strip_prefix("layout.context_top_cell.") {
            return self.set_layout_view_top_cell(value);
        }

        if let Some(value) = action.strip_prefix("layout.context_parent_cell.") {
            return self.set_layout_view_top_cell(value);
        }

        if let Some(value) = action.strip_prefix("layout.context_child_cell.") {
            return self.set_layout_view_top_cell(value);
        }

        if action == "layout.descend_selected_instance" {
            return self.descend_selected_layout_instance();
        }

        if action == "layout.make_cell_variant" {
            return self.make_selected_layout_instance_variant();
        }

        if action == "layout.duplicate_current_cell" {
            return self.duplicate_layout_view_cell();
        }

        if action == "layout.flatten_selected_instance_one" {
            return self.flatten_selected_layout_instance(LayoutFlattenDepth::OneLevel);
        }

        if action == "layout.flatten_current_cell_one" {
            return self.flatten_current_layout_cell(LayoutFlattenDepth::OneLevel);
        }

        if action == "layout.flatten_current_cell" {
            return self.flatten_current_layout_cell(LayoutFlattenDepth::Deep);
        }

        if action == "layout.cell_origin.selection" {
            return self.adjust_current_layout_cell_origin_to_selection();
        }

        if action == "layout.cell_origin.exact" {
            return self.adjust_current_layout_cell_origin_exact_from_search();
        }

        if let Some(axis) = action.strip_prefix("layout.cell_origin.") {
            return match axis {
                "x_neg" => self.adjust_current_layout_cell_origin_by(-1, 0),
                "x_pos" => self.adjust_current_layout_cell_origin_by(1, 0),
                "y_neg" => self.adjust_current_layout_cell_origin_by(0, -1),
                "y_pos" => self.adjust_current_layout_cell_origin_by(0, 1),
                _ => false,
            };
        }

        if action == "layout.move_shape_up" {
            return self.move_selected_layout_shape_up_hierarchy();
        }

        if action == "layout.move_instance_up" {
            return self.move_selected_layout_instance_up_hierarchy();
        }

        if action == "layout.delete_unused_cell" {
            return self.delete_unused_layout_view_cell();
        }

        if action == "layout.delete_cell_shallow" {
            return self.delete_layout_view_cell_shallow();
        }

        if action == "layout.delete_cell_deep" {
            return self.delete_layout_view_cell_deep();
        }

        if action == "layout.delete_cell_complete" {
            return self.delete_layout_view_cell_complete();
        }

        if let Some(slug) = action.strip_prefix("workflow.open.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("workflow.focus_lot.") {
            if workflow_lot_ids(&self.workspace)
                .iter()
                .any(|id| id == lot_id)
            {
                self.workflow_focus_lot = Some(lot_id.to_string());
                self.status_message = format!("Workflow focus lot {lot_id}");
                return true;
            }
        }

        if action == "workflow.load_demo" {
            self.replace_workspace(WorkspaceDataset::demo(), "demo workspace loaded");
            return true;
        }

        if let Some(tool_id) = action.strip_prefix("fab.select.") {
            return self.select_equipment_tool(tool_id);
        }

        if let Some(raw) = action.strip_prefix("fab.recipe.") {
            let mut parts = raw.split('|');
            let Some(tool_id) = parts.next() else {
                return false;
            };
            let Some(recipe_id) = parts.next() else {
                return false;
            };
            return self.load_equipment_recipe(tool_id, recipe_id);
        }

        if let Some(raw) = action.strip_prefix("fab.command.") {
            return self.apply_equipment_command(raw);
        }

        if let Some(slug) = action.strip_prefix("maintenance.filter.") {
            if let Some(filter) = MaintenanceWorkFilter::from_slug(slug) {
                self.maintenance_work_filter = filter;
                self.status_message = format!("Maintenance filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("maintenance.history.") {
            if let Some(filter) = MaintenanceHistoryFilter::from_slug(slug) {
                self.maintenance_history_filter = filter;
                self.status_message = format!("Maintenance history {}", filter.label());
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("maintenance.tool.") {
            let tool_id = EquipmentToolId::new(tool_id);
            if self.workspace.maintenance.tool(&tool_id).is_some() {
                self.selected_maintenance_tool = Some(tool_id.clone());
                self.status_message = format!("Selected maintenance tool {tool_id}");
                return true;
            }
        }

        if let Some(raw) = action.strip_prefix("maintenance.status.") {
            let Some((kind, tool_id)) = raw.split_once('|') else {
                return false;
            };
            let tool_id = EquipmentToolId::new(tool_id);
            if self.workspace.maintenance.tool(&tool_id).is_none() {
                return false;
            }
            self.status_message = match kind {
                "schedule" => format!("Maintenance scheduling opened for {tool_id}"),
                "calibration" => format!("Calibration capture opened for {tool_id}"),
                "release" => format!("Release hold review opened for {tool_id}"),
                _ => return false,
            };
            return true;
        }

        if let Some(sensor_id) = action.strip_prefix("environment.sensor.") {
            if self
                .workspace
                .environment
                .sensors
                .iter()
                .any(|sensor| sensor.id == sensor_id)
            {
                self.selected_environment_sensor = Some(sensor_id.to_string());
                self.status_message = format!("Selected environment sensor {sensor_id}");
                return true;
            }
        }

        if let Some(zone) = action.strip_prefix("environment.zone.") {
            if let Some(sensor) = self
                .workspace
                .environment
                .sensors
                .iter()
                .find(|sensor| sensor.zone == zone)
            {
                self.selected_environment_sensor = Some(sensor.id.clone());
                self.status_message = format!("Environment zone {zone}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("inventory.filter.") {
            if let Some(filter) = InventoryQuickFilter::from_slug(slug) {
                self.inventory_filter = filter;
                self.selected_inventory_lot = default_inventory_lot(&self.workspace, filter);
                self.status_message = format!("Inventory filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("inventory.lot.") {
            let lot_id = MaterialLotId::new(lot_id);
            if self.workspace.inventory.lot(&lot_id).is_some() {
                self.selected_inventory_lot = Some(lot_id.clone());
                self.status_message = format!("Selected inventory lot {lot_id}");
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("mask.lot.") {
            let lot_id = LotId::new(lot_id);
            if self.workspace.mes.lots.contains_key(&lot_id) {
                self.mask_source_lot = Some(lot_id.clone());
                self.mask_issue_page = 0;
                self.status_message = format!("Reticle prep lot {lot_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("mask.severity.") {
            if let Some(filter) = MaskIssueSeverityFilter::from_slug(slug) {
                self.mask_issue_severity_filter = filter;
                self.mask_issue_page = 0;
                self.status_message = format!("Reticle issues {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("mask.group.") {
            if let Some(grouping) = MaskIssueGrouping::from_slug(slug) {
                self.mask_issue_grouping = grouping;
                self.status_message = format!("Reticle issues grouped by {}", grouping.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.baseline.") {
            if let Some(source) = LayoutDiffSource::from_slug(slug) {
                self.layout_diff_baseline = source;
                self.reset_layout_diff_review();
                self.status_message = format!("Diff baseline {}", source.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.candidate.") {
            if let Some(source) = LayoutDiffSource::from_slug(slug) {
                self.layout_diff_candidate = source;
                self.reset_layout_diff_review();
                self.status_message = format!("Diff candidate {}", source.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.filter.") {
            if let Some(filter) = LayoutChangeFilter::from_slug(slug) {
                self.layout_diff_change_filter = filter;
                self.layout_diff_change_page = 0;
                self.status_message = format!("Diff filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(value) = action.strip_prefix("layout_diff.page_size.") {
            if let Ok(page_size) = value.parse::<usize>()
                && LAYOUT_DIFF_PAGE_SIZE_OPTIONS.contains(&page_size)
            {
                self.layout_diff_page_size = page_size;
                self.layout_diff_change_page = 0;
                self.status_message = format!("Diff rows per page {page_size}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.review.") {
            if let Some(state) = LayoutReviewDisposition::from_slug(slug) {
                self.layout_diff_review_state = state;
                self.status_message = format!("Diff review {}", state.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("trace.lot.") {
            let lot_id = LotId::new(lot_id);
            return self.select_trace_detail(TraceSelection::Lot(lot_id));
        }

        if let Some(raw) = action.strip_prefix("trace.wafer.") {
            let Some((lot_id, wafer_id)) = raw.split_once('|') else {
                return false;
            };
            let lot_id = LotId::new(lot_id);
            let wafer_id = WaferId::new(wafer_id);
            let wafer = WaferRef {
                lot_id: lot_id.clone(),
                wafer_id: wafer_id.clone(),
            };
            return self.select_trace_detail(TraceSelection::Wafer(wafer));
        }

        if let Some(raw) = action.strip_prefix("trace.detail.") {
            if let Some(selection) = trace_selection_from_view_action(raw) {
                return self.select_trace_detail(selection);
            }
        }

        if let Some(slug) = action.strip_prefix("trace.impact.") {
            if let Some(mode) = TraceImpactMode::from_slug(slug) {
                self.trace_impact_mode = mode;
                self.status_message = format!("Traceability impact {}", mode.label());
                return true;
            }
        }

        if let Some(entry_id) = action.strip_prefix("notebook.entry.") {
            let entry_id = NotebookEntryId::new(entry_id);
            if self.workspace.lab_notebook.entry(&entry_id).is_some() {
                self.selected_notebook_entry = Some(entry_id.clone());
                self.status_message = format!("Notebook entry {entry_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("notebook.entry_action.") {
            if let Some(entry_action) = NotebookEntryAction::from_slug(slug) {
                return self.apply_notebook_entry_action(entry_action);
            }
        }

        if let Some(token) = action.strip_prefix("notebook.focus_link.") {
            let Some((kind_slug, query)) = token.split_once(':') else {
                return false;
            };
            if let Some(kind) = notebook_link_kind_from_slug(kind_slug) {
                return self.focus_notebook_link(kind, query);
            }
        }

        if let Some(tag) = action.strip_prefix("notebook.tag.") {
            if tag == "all" {
                self.notebook_tag_filter = None;
                self.ensure_notebook_selection();
                self.status_message = "Notebook tag filter all".to_string();
                return true;
            }
            if self
                .workspace
                .lab_notebook
                .tags()
                .iter()
                .any(|candidate| candidate == tag)
            {
                self.notebook_tag_filter = Some(tag.to_string());
                self.ensure_notebook_selection();
                self.status_message = format!("Notebook tag {tag}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("notebook.link.") {
            self.notebook_link_kind_filter = if slug == "any" {
                None
            } else {
                let Some(kind) = notebook_link_kind_from_slug(slug) else {
                    return false;
                };
                Some(kind)
            };
            self.ensure_notebook_selection();
            self.status_message = format!(
                "Notebook link {}",
                self.notebook_link_kind_filter
                    .map(|kind| kind.label())
                    .unwrap_or("any")
            );
            return true;
        }

        if let Some(response_id) = action.strip_prefix("experiment.response.") {
            let response_id = ResponseSpecId::new(response_id);
            if self
                .workspace
                .experiment_plan
                .response(&response_id)
                .is_some()
            {
                self.selected_experiment_response = Some(response_id.clone());
                self.status_message = format!("DOE primary response set to {response_id}");
                return true;
            }
        }

        if let Some(run_id) = action.strip_prefix("experiment.run.") {
            let run_id = ExperimentRunId::new(run_id);
            if self.workspace.experiment_plan.run(&run_id).is_some() {
                self.selected_experiment_run = Some(run_id.clone());
                self.status_message = format!("DOE run selected: {run_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("experiment.filter.") {
            if let Some(filter) = ExperimentRunFilter::from_slug(slug) {
                self.experiment_run_filter = filter;
                self.status_message = format!("DOE run filter set to {}", filter.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("experiment.lot.") {
            if self
                .experiment_lot_options()
                .iter()
                .any(|candidate| candidate == lot_id)
            {
                if self.experiment_lot_filter.as_deref() == Some(lot_id) {
                    self.experiment_lot_filter = None;
                    self.status_message = "DOE lot filter cleared".to_string();
                } else {
                    self.experiment_lot_filter = Some(lot_id.to_string());
                    self.status_message = format!("DOE lot filter set to {lot_id}");
                }
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("scheduler.policy.") {
            if let Some(policy) = dispatch_policy_from_slug(slug) {
                self.scheduler_policy = policy;
                self.status_message =
                    format!("Dispatch policy {}", display_dispatch_policy_label(policy));
                return true;
            }
        }

        if let Some(value) = action.strip_prefix("scheduler.priority.") {
            if let Ok(priority) = value.parse::<u8>()
                && priority <= 5
            {
                self.scheduler_min_priority = priority;
                self.status_message = format!("Minimum dispatch priority P{priority}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("scheduler.tool.") {
            if self
                .workspace
                .scheduler
                .tools
                .iter()
                .any(|tool| tool.id.as_str() == tool_id)
            {
                self.selected_scheduler_tool = Some(SchedulerToolId::new(tool_id));
                self.status_message = format!("Selected dispatch tool {tool_id}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("safety.tool.") {
            if self
                .workspace
                .safety
                .evaluate_lockouts()
                .iter()
                .any(|lockout| lockout.tool_id == tool_id)
            {
                self.selected_safety_tool = Some(tool_id.to_string());
                self.status_message = format!("Selected safety tool {tool_id}");
                return true;
            }
        }

        if let Some(sensor_id) = action.strip_prefix("safety.ack.condition.") {
            if self
                .workspace
                .safety
                .sensors
                .iter()
                .any(|sensor| sensor.id.to_string() == sensor_id)
            {
                self.acknowledged_conditions.insert(sensor_id.to_string());
                self.status_message = format!("Acknowledged safety condition {sensor_id}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("safety.ack.lockout.") {
            if self
                .workspace
                .safety
                .evaluate_lockouts()
                .iter()
                .any(|lockout| lockout.tool_id == tool_id)
            {
                self.acknowledged_lockouts.insert(tool_id.to_string());
                self.status_message = format!("Acknowledged safety lockout {tool_id}");
                return true;
            }
        }

        if let Some(incident_id) = action.strip_prefix("safety.ack.incident.") {
            if self
                .workspace
                .safety
                .incidents
                .iter()
                .any(|incident| incident.id.to_string() == incident_id)
            {
                self.acknowledged_incidents.insert(incident_id.to_string());
                self.status_message = format!("Acknowledged safety incident {incident_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("process_flow.filter.") {
            if let Some(filter) = ProcessFlowNodeFilter::from_slug(slug) {
                self.process_flow_filter = filter;
                self.selected_process_node =
                    default_process_flow_node_for_filter(&self.workspace, filter);
                self.status_message = format!("Process flow filter {}", filter.label());
                return true;
            }
        }

        if let Some(node_id) = action.strip_prefix("process_flow.node.") {
            let node_id = ProcessFlowNodeId::new(node_id);
            if self
                .workspace
                .process_flow
                .route
                .nodes
                .iter()
                .any(|node| node.id == node_id)
            {
                self.selected_process_node = Some(node_id.clone());
                self.status_message = format!("Selected process node {node_id}");
                return true;
            }
        }

        if let Some(loop_id) = action.strip_prefix("process_control.loop.") {
            return self.select_control_loop(loop_id);
        }

        if let Some(slug) = action.strip_prefix("process_control.filter.") {
            if let Some(filter) = ControlLoopFilter::from_slug(slug) {
                self.process_control_loop_filter = filter;
                if !self
                    .selected_control_loop
                    .as_ref()
                    .and_then(|id| self.workspace.process_control.loop_by_id(id))
                    .is_some_and(|loop_definition| {
                        control_loop_matches(&self.workspace, filter, loop_definition)
                    })
                {
                    self.selected_control_loop =
                        default_control_loop_for_filter(&self.workspace, filter);
                    self.selected_control_action = default_control_action(
                        &self.workspace,
                        self.selected_control_loop.as_ref(),
                    );
                }
                self.status_message = format!("Run-to-run loop filter {}", filter.label());
                return true;
            }
        }

        if let Some(action_id) = action.strip_prefix("process_control.action.") {
            return self.select_control_action(action_id);
        }

        if let Some(raw) = action.strip_prefix("process_control.transition.") {
            let Some((action_id, transition)) = raw.split_once('|') else {
                return false;
            };
            return self.apply_process_control_transition(action_id, transition);
        }

        if let Some(slug) = action.strip_prefix("spc.severity.") {
            if let Some(filter) = SpcSeverityFilter::from_slug(slug) {
                self.spc_severity_filter = filter;
                self.status_message = format!("SPC/FDC severity {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("spc.source.") {
            if let Some(filter) = SpcSourceFilter::from_slug(slug) {
                self.spc_source_filter = filter;
                self.status_message = format!("SPC/FDC source {}", filter.detail_label());
                return true;
            }
        }

        if let Some(chart_id) = action.strip_prefix("spc.chart.") {
            let monitor = spc_fdc_monitor(&self.workspace);
            if monitor
                .charts
                .iter()
                .any(|chart| chart.id.as_str() == chart_id)
            {
                self.selected_spc_chart = Some(chart_id.to_string());
                self.spc_context_filter = chart_id.to_string();
                self.status_message = format!("Selected SPC chart {chart_id}");
                return true;
            }
        }

        if let Some(trace_id) = action.strip_prefix("spc.trace.") {
            let monitor = spc_fdc_monitor(&self.workspace);
            if monitor.traces.iter().any(|trace| trace.id == trace_id) {
                self.selected_fdc_trace = Some(trace_id.to_string());
                self.spc_context_filter = trace_id.to_string();
                self.status_message = format!("Selected FDC trace {trace_id}");
                return true;
            }
        }

        if let Some(step) = action.strip_prefix("cross_section.step.") {
            if let Ok(step) = step.parse::<usize>() {
                let max_step = self.workspace.cross_section.steps.len();
                if step <= max_step {
                    self.cross_section_step = step;
                    self.status_message = format!("Cross-section step {step}");
                    return true;
                }
            }
        }

        if let Some(material_id) = action.strip_prefix("cross_section.material.") {
            let material_id = MaterialId::new(material_id);
            if self
                .workspace
                .cross_section
                .materials
                .iter()
                .any(|material| material.id == material_id)
            {
                self.selected_cross_section_material = Some(material_id.clone());
                self.status_message =
                    format!("Selected cross-section material {}", material_id.as_str());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("metrology.mode.") {
            if let Some(mode) = MetrologyMapMode::from_slug(slug) {
                self.metrology_map_mode = mode;
                self.status_message = format!("Metrology {}", mode.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("metrology.kind.") {
            if let Some(kind) = measurement_kind_from_slug(slug) {
                self.metrology_kind = kind;
                self.status_message = format!("Metrology measurement {}", kind.label());
                return true;
            }
        }

        if let Some(raw) = action.strip_prefix("metrology.die.") {
            let Some((column, row)) = raw.split_once('|') else {
                return false;
            };
            if let (Ok(column), Ok(row)) = (column.parse::<i32>(), row.parse::<i32>()) {
                let die = DieCoord::new(column, row);
                if self
                    .workspace
                    .wafer_map
                    .dies
                    .iter()
                    .any(|entry| *entry == die)
                {
                    self.selected_die = Some(die);
                    self.status_message = format!("Metrology die {}", die_coord_label(Some(die)));
                    return true;
                }
            }
        }

        if let Some(slug) = action.strip_prefix("yield.filter.") {
            if let Some(filter) = YieldMapFilter::from_slug(slug) {
                self.yield_map_filter = filter;
                self.status_message = format!("Yield map filter {}", filter.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("yield.lot.") {
            if self
                .workspace
                .yield_analysis
                .lots
                .iter()
                .any(|lot| lot.id == lot_id)
            {
                self.selected_yield_lot = Some(lot_id.to_string());
                self.selected_yield_wafer =
                    default_yield_wafer(&self.workspace, self.selected_yield_lot.as_deref());
                self.status_message = format!("Yield lot {lot_id}");
                return true;
            }
        }

        if let Some(wafer_id) = action.strip_prefix("yield.wafer.") {
            if let Some(lot_id) = self.selected_yield_lot.as_deref() {
                let wafer_ids = self.workspace.yield_analysis.wafer_ids_for_lot(lot_id);
                if wafer_ids.iter().any(|id| id == wafer_id) {
                    self.selected_yield_wafer = Some(wafer_id.to_string());
                    self.status_message = format!("Yield wafer {wafer_id}");
                    return true;
                }
            }
        }

        match action {
            "layout.add_layer" => self.add_layout_layer(),
            "layout.next_shape" => self.select_next_layout_shape(),
            "layout.clear_selection" => {
                self.selected_layout_shape = None;
                self.selected_layout_occurrence = None;
                self.status_message = "Layout selection cleared".to_string();
                true
            }
            "layout.copy" => self.copy_layout_selection(),
            "layout.paste" => self.paste_layout_clipboard(),
            "layout.duplicate" => self.duplicate_layout_selection(),
            "layout.delete" => self.delete_layout_selection(),
            "layout.toggle_grid" => {
                self.show_grid = !self.show_grid;
                self.status_message = format!(
                    "2D grid {}",
                    if self.show_grid { "shown" } else { "hidden" }
                );
                true
            }
            "layout.toggle_snap" => {
                self.snap_enabled = !self.snap_enabled;
                self.status_message = format!(
                    "Snap {}",
                    if self.snap_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "layout.toggle_drc" => {
                self.show_drc_overlay = !self.show_drc_overlay;
                self.status_message = format!(
                    "DRC overlay {}",
                    if self.show_drc_overlay {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "layout.toggle_reference_images" => self.toggle_layout_reference_images(),
            "layout.run_drc" => {
                self.status_message = self.run_drc_summary();
                true
            }
            "layout.run_drc_region" => {
                self.status_message = self.run_drc_for_selected_region_summary();
                true
            }
            "layout.run_drc_cell" => {
                self.status_message = self.run_drc_for_current_cell_summary();
                true
            }
            "layout.run_route" => self.route_layout_between_points(),
            "layout.trace_between" => self.trace_layout_between_route_points(),
            "layout.trace_all" => self.trace_all_layout_nets(),
            "layout.connectivity" => {
                self.status_message = self.connectivity_summary();
                true
            }
            "metrology.failed_only" => {
                self.metrology_failed_only = !self.metrology_failed_only;
                self.status_message = if self.metrology_failed_only {
                    "Metrology showing failed/outlier sites".to_string()
                } else {
                    "Metrology showing all sites".to_string()
                };
                true
            }
            "metrology.next_attention" => {
                self.selected_die = self.first_attention_die();
                self.status_message = self
                    .selected_die
                    .map(|die| format!("Selected attention die {}", die_coord_label(Some(die))))
                    .unwrap_or_else(|| "No attention die found".to_string());
                true
            }
            "metrology.clear_die" => {
                self.selected_die = None;
                self.status_message = "Metrology selection cleared".to_string();
                true
            }
            "yield.attention" => {
                self.show_only_attention_wafers = !self.show_only_attention_wafers;
                self.status_message = format!(
                    "{} attention wafers",
                    if self.show_only_attention_wafers {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "yield.excursions" => {
                self.show_only_excursions = !self.show_only_excursions;
                self.status_message = format!(
                    "{} yield excursions",
                    if self.show_only_excursions {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "experiment.pending_only" => {
                self.experiment_show_missing_only = !self.experiment_show_missing_only;
                self.status_message = format!(
                    "{} pending DOE runs",
                    if self.experiment_show_missing_only {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "experiment.next_pending" => {
                if self.select_next_pending_experiment_response() {
                    let run = self.selected_experiment_run_label();
                    self.status_message = format!(
                        "DOE: queued {} for {run}",
                        self.selected_experiment_response_label()
                    );
                } else {
                    self.status_message = "DOE: no pending responses remain".to_string();
                }
                true
            }
            "experiment.capture" => self.capture_selected_experiment_response(),
            "experiment.capture_advance" => {
                if self.capture_selected_experiment_response()
                    && self.select_next_pending_experiment_response()
                {
                    self.status_message
                        .push_str("; queued next pending response");
                }
                true
            }
            "experiment.use_demo" => {
                self.use_demo_experiment_capture_value();
                true
            }
            "experiment.use_target" => {
                self.use_experiment_response_target();
                true
            }
            "experiment.capture_next_demo" => {
                self.capture_next_demo_experiment_response();
                true
            }
            "experiment.clear_filters" => {
                self.experiment_run_filter = ExperimentRunFilter::All;
                self.experiment_lot_filter = None;
                self.experiment_show_missing_only = false;
                self.status_message = "DOE filters cleared".to_string();
                true
            }
            "scheduler.toggle_conflicts" => {
                self.scheduler_conflicts_only = !self.scheduler_conflicts_only;
                self.status_message = "Dispatch conflict filter toggled".to_string();
                true
            }
            "scheduler.toggle_focus_tool" => {
                self.scheduler_focus_selected_tool = !self.scheduler_focus_selected_tool;
                self.status_message = "Dispatch selected-tool filter toggled".to_string();
                true
            }
            "scheduler.reset" => {
                self.scheduler_min_priority = 0;
                self.scheduler_conflicts_only = false;
                self.scheduler_focus_selected_tool = false;
                self.status_message = "Dispatch filters reset".to_string();
                true
            }
            "mask.rebuild" => {
                self.mask_issue_page = 0;
                let report = mask_check_report(self);
                self.status_message = format!(
                    "Reticle prep rebuilt: {} issue(s)",
                    report.total_issue_count()
                );
                true
            }
            "mask.prev_page" => {
                self.mask_issue_page = self.mask_issue_page.saturating_sub(1);
                self.status_message = format!("Reticle issue page {}", self.mask_issue_page + 1);
                true
            }
            "mask.next_page" => {
                let report = mask_check_report(self);
                let page_count = mask_issue_page_count(self, &report);
                if self.mask_issue_page + 1 < page_count {
                    self.mask_issue_page += 1;
                }
                self.status_message = format!("Reticle issue page {}", self.mask_issue_page + 1);
                true
            }
            "mask.clear_filters" => {
                self.mask_issue_severity_filter = MaskIssueSeverityFilter::All;
                self.mask_issue_grouping = MaskIssueGrouping::Code;
                self.mask_issue_page = 0;
                self.status_message = "Reticle issue filters cleared".to_string();
                true
            }
            "layout_diff.swap" => {
                std::mem::swap(
                    &mut self.layout_diff_baseline,
                    &mut self.layout_diff_candidate,
                );
                self.reset_layout_diff_review();
                self.status_message = "Diff sources swapped".to_string();
                true
            }
            "layout_diff.demo_current" => {
                self.layout_diff_baseline = LayoutDiffSource::Demo;
                self.layout_diff_candidate = LayoutDiffSource::Current;
                self.reset_layout_diff_review();
                self.status_message = "Diff set to sample -> current".to_string();
                true
            }
            "layout_diff.empty_current" => {
                self.layout_diff_baseline = LayoutDiffSource::Empty;
                self.layout_diff_candidate = LayoutDiffSource::Current;
                self.reset_layout_diff_review();
                self.status_message = "Diff set to empty -> current".to_string();
                true
            }
            "layout_diff.toggle_changed_only" => {
                self.layout_diff_changed_only = !self.layout_diff_changed_only;
                self.status_message = format!(
                    "Diff changed layers {}",
                    if self.layout_diff_changed_only {
                        "only"
                    } else {
                        "and unchanged"
                    }
                );
                true
            }
            "layout_diff.prev_page" => {
                self.layout_diff_change_page = self.layout_diff_change_page.saturating_sub(1);
                self.status_message =
                    format!("Diff change page {}", self.layout_diff_change_page + 1);
                true
            }
            "layout_diff.next_page" => {
                let report = layout_diff_report(self);
                let page_count = layout_diff_page_count(self, &report);
                if self.layout_diff_change_page + 1 < page_count {
                    self.layout_diff_change_page += 1;
                }
                self.status_message =
                    format!("Diff change page {}", self.layout_diff_change_page + 1);
                true
            }
            "layout_diff.reset_filters" => {
                self.layout_diff_change_filter = LayoutChangeFilter::All;
                self.layout_diff_page_size = LAYOUT_DIFF_DEFAULT_PAGE_SIZE;
                self.layout_diff_change_page = 0;
                self.status_message = "Diff filters reset".to_string();
                true
            }
            "trace.toggle_related" => {
                self.trace_related_only = !self.trace_related_only;
                self.status_message = "Traceability related-only filter toggled".to_string();
                true
            }
            "notebook.toggle_followups" => {
                self.notebook_followups_only = !self.notebook_followups_only;
                self.ensure_notebook_selection();
                self.status_message = "Notebook follow-up filter toggled".to_string();
                true
            }
            "notebook.preview" => {
                self.notebook_preview_mode = true;
                self.status_message = "Notebook preview mode".to_string();
                true
            }
            "notebook.edit" => {
                self.notebook_preview_mode = false;
                self.status_message = "Notebook edit mode".to_string();
                true
            }
            "notebook.clear_filters" => {
                self.notebook_tag_filter = None;
                self.notebook_link_kind_filter = None;
                self.notebook_followups_only = false;
                self.ensure_notebook_selection();
                self.status_message = "Notebook filters cleared".to_string();
                true
            }
            "process_flow.toggle_errors" => {
                self.process_flow_errors_only = !self.process_flow_errors_only;
                self.status_message = if self.process_flow_errors_only {
                    "Process flow showing errors only".to_string()
                } else {
                    "Process flow showing all findings".to_string()
                };
                true
            }
            "process_flow.validate" => {
                let findings = self.workspace.process_flow.findings();
                self.status_message = format!("Process flow has {} finding(s)", findings.len());
                true
            }
            "process_flow.export" => {
                let route = self.workspace.process_flow.export_to_mes_route();
                self.status_message = format!(
                    "Prepared {} MES step(s) for {}",
                    route.steps.len(),
                    route.id
                );
                true
            }
            "spc.clear_context" => {
                self.spc_context_filter.clear();
                self.status_message = "SPC/FDC context filter cleared".to_string();
                true
            }
            "cross_section.toggle_mask" => {
                self.cross_section_show_mask = !self.cross_section_show_mask;
                self.status_message = format!(
                    "Cross-section mask {}",
                    if self.cross_section_show_mask {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "cross_section.toggle_dimensions" => {
                self.cross_section_show_dimensions = !self.cross_section_show_dimensions;
                self.status_message = format!(
                    "Cross-section dimensions {}",
                    if self.cross_section_show_dimensions {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "cross_section.toggle_risks" => {
                self.cross_section_show_risks = !self.cross_section_show_risks;
                self.status_message = format!(
                    "Cross-section risks {}",
                    if self.cross_section_show_risks {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            _ => false,
        }
    }
}
