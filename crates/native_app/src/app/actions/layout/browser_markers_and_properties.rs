#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn delete_inactive_empty_layout_layers(&mut self) -> bool {
        let usage = layout_layer_usage_counts(&self.workspace.document);
        let mut layers = self
            .workspace
            .document
            .layers
            .values()
            .filter(|layer| {
                layer.id != self.active_layer && usage.get(&layer.id).copied().unwrap_or(0) == 0
            })
            .cloned()
            .collect::<Vec<_>>();
        layers.sort_by_key(|layer| (layer.display_order, layer.id));
        if layers.is_empty() {
            self.status_message = "No inactive empty layers to remove".to_string();
            return true;
        }

        let operations = layers
            .iter()
            .map(|layer| Operation::DeleteLayer { id: layer.id })
            .collect::<Vec<_>>();
        let undo_operations = layers
            .iter()
            .rev()
            .cloned()
            .map(|layer| Operation::AddLayer { layer })
            .collect::<Vec<_>>();
        let removed_count = operations.len();
        for layer in &layers {
            self.layout_layer_depth_overrides.remove(&layer.id);
        }
        self.apply_layout_operation_with_history(
            Operation::Batch { operations },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Removed {removed_count} inactive empty layer{}",
            if removed_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn set_layout_layer_group_filter(&mut self, value: &str) -> bool {
        let Some(group) = LayoutLayerGroupFilter::from_slug(value) else {
            return false;
        };
        if self.layout_layer_group_filter == group {
            self.status_message = format!("Layer group already {}", group.path_label());
            return true;
        }
        self.layout_layer_group_filter = group;
        self.status_message = format!("Layer group {}", group.path_label());
        true
    }

    pub(crate) fn set_layout_layer_usage_filter(&mut self, value: &str) -> bool {
        let Some(filter) = LayoutLayerUsageFilter::from_slug(value) else {
            return false;
        };
        if self.layout_layer_usage_filter == filter {
            self.status_message = format!("Layer rows already {}", filter.label());
            return true;
        }
        self.layout_layer_usage_filter = filter;
        self.status_message = format!("Layer rows {}", filter.label());
        true
    }

    pub(crate) fn set_layout_layer_group_hierarchy_depth(&mut self, value: &str) -> bool {
        let next_depth = if value == "inherit" {
            None
        } else {
            let Some(depth) = LayoutHierarchyDepth::from_slug(value) else {
                return false;
            };
            if depth.shows_instance_boxes() {
                return false;
            }
            Some(depth)
        };
        let group = self.layout_layer_group_filter;
        let mut target_layers = self
            .workspace
            .document
            .layers
            .values()
            .filter(|layer| group.matches(layer.process))
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        target_layers.sort_unstable();
        if target_layers.is_empty() {
            self.status_message = format!("Layer group {} has no layers", group.path_label());
            return true;
        }

        let mut changed_count = 0;
        for layer in target_layers {
            let previous_depth = self.layout_layer_depth_overrides.get(&layer).copied();
            if previous_depth == next_depth {
                continue;
            }
            match next_depth {
                Some(depth) => {
                    self.layout_layer_depth_overrides.insert(layer, depth);
                }
                None => {
                    self.layout_layer_depth_overrides.remove(&layer);
                }
            }
            changed_count += 1;
        }

        let depth_label = layout_layer_depth_override_label(next_depth);
        if changed_count == 0 {
            self.status_message = format!(
                "Layer group {} hierarchy depth already {depth_label}",
                group.path_label()
            );
            return true;
        }
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Layer group {} hierarchy depth {depth_label} on {changed_count} layer(s)",
            group.path_label()
        );
        true
    }

    pub(crate) fn set_layout_measurement_mode(&mut self, value: &str) -> bool {
        let Some(mode) = MeasurementMode::from_slug(value) else {
            return false;
        };
        if self.layout_measurement_mode == mode {
            self.status_message = format!("Ruler mode already {}", mode.label());
            return true;
        }
        self.layout_measurement_mode = mode;
        self.status_message = format!("Ruler mode {}", mode.label());
        true
    }

    pub(crate) fn set_layout_measurement_filter(&mut self, value: &str) -> bool {
        let Some(filter) = LayoutMeasurementFilter::from_slug(value) else {
            return false;
        };
        if self.layout_measurement_filter == filter {
            self.status_message = format!("Measurement filter already {}", filter.label());
            return true;
        }
        self.layout_measurement_filter = filter;
        self.status_message = format!("Measurement filter {}", filter.label());
        true
    }

    pub(crate) fn select_first_layout_measurement_browser_match(&mut self) -> bool {
        let Some((occurrence, _, shape)) = layout_measurement_entries(self).into_iter().next()
        else {
            self.status_message =
                "Measurement browser has no matching rulers to select".to_string();
            return true;
        };
        let label = layout_measurement_browser_label(self, &occurrence, &shape);
        let key = layout_occurrence_action_key(&occurrence);
        if !self.select_layout_occurrence(&key) {
            self.status_message = "Measurement browser match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!(
            "Selected browser measurement {}",
            compact_button_label(&label, 32)
        );
        true
    }

    pub(crate) fn clear_visible_layout_measurements(&mut self) -> bool {
        let entries = layout_measurement_entries_with_options(self, false, false, usize::MAX);
        let mut removed = BTreeMap::new();
        for (_, source_cell, shape) in entries {
            removed.entry((source_cell, shape.id)).or_insert(shape);
        }
        if removed.is_empty() {
            self.status_message = "No visible measurements to clear".to_string();
            return true;
        }

        let mut redo = Vec::new();
        let mut undo = Vec::new();
        for ((source_cell, shape_id), shape) in &removed {
            if *source_cell == self.workspace.document.top_cell
                && self.workspace.document.shapes.contains_key(shape_id)
            {
                redo.push(Operation::DeleteShape { id: *shape_id });
                undo.push(Operation::AddShape {
                    shape: shape.clone(),
                });
            } else {
                redo.push(Operation::DeleteShapeFromCell {
                    cell: *source_cell,
                    id: *shape_id,
                });
                undo.push(Operation::AddShapeToCell {
                    cell: *source_cell,
                    shape: shape.clone(),
                });
            }
        }

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch {
                operations: undo.into_iter().rev().collect(),
            },
        );
        if self
            .selected_layout_shape
            .is_some_and(|selected| removed.keys().any(|(_, shape_id)| *shape_id == selected))
        {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
        self.status_message = format!(
            "Cleared {} visible measurement{}",
            removed.len(),
            if removed.len() == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn delete_selected_layout_measurement(&mut self) -> bool {
        let Some((occurrence, source_cell, shape)) = selected_visible_layout_measurement(self)
        else {
            self.status_message = "No selected visible measurement to delete".to_string();
            return true;
        };
        let (redo, undo) = if source_cell == self.workspace.document.top_cell
            && self.workspace.document.shapes.contains_key(&shape.id)
        {
            (
                Operation::DeleteShape { id: shape.id },
                Operation::AddShape {
                    shape: shape.clone(),
                },
            )
        } else {
            (
                Operation::DeleteShapeFromCell {
                    cell: source_cell,
                    id: shape.id,
                },
                Operation::AddShapeToCell {
                    cell: source_cell,
                    shape: shape.clone(),
                },
            )
        };
        self.apply_layout_operation_with_history(redo, undo);
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        self.status_message = format!(
            "Deleted selected measurement {}",
            layout_occurrence_label(&occurrence)
        );
        true
    }

    pub(crate) fn focus_selected_layout_measurement(&mut self) -> bool {
        let Some((occurrence, _, shape)) = selected_visible_layout_measurement(self) else {
            self.status_message = "No selected visible measurement to focus".to_string();
            return true;
        };
        let label = layout_measurement_label_for_button(&shape.kind);
        self.ensure_layout_occurrence_depth_visible(&occurrence);
        self.selected_layout_shape = Some(occurrence.source_shape_id());
        self.selected_layout_occurrence = Some(occurrence);
        self.active_layer = shape.layer;
        self.focus_layout_rect(
            shape.kind.bounds(),
            &format!("Focused measurement {}", compact_button_label(&label, 48)),
        )
    }

    pub(crate) fn cycle_active_layout_layer_fill_style(&mut self) -> bool {
        let Some(layer) = self.workspace.document.layers.get(&self.active_layer) else {
            self.status_message = "Select a layer before changing fill style".to_string();
            return true;
        };
        let next_fill = layer.fill_style.next();
        let line_style = layer.line_style;
        let old_fill = layer.fill_style;
        let name = layer.name.clone();
        self.apply_layout_operation_with_history(
            Operation::SetLayerDisplayStyle {
                layer: self.active_layer,
                fill_style: next_fill,
                line_style,
            },
            Operation::SetLayerDisplayStyle {
                layer: self.active_layer,
                fill_style: old_fill,
                line_style,
            },
        );
        self.status_message = format!("Layer {name} fill {}", next_fill.label());
        true
    }

    pub(crate) fn cycle_active_layout_layer_line_style(&mut self) -> bool {
        let Some(layer) = self.workspace.document.layers.get(&self.active_layer) else {
            self.status_message = "Select a layer before changing line style".to_string();
            return true;
        };
        let fill_style = layer.fill_style;
        let next_line = layer.line_style.next();
        let old_line = layer.line_style;
        let name = layer.name.clone();
        self.apply_layout_operation_with_history(
            Operation::SetLayerDisplayStyle {
                layer: self.active_layer,
                fill_style,
                line_style: next_line,
            },
            Operation::SetLayerDisplayStyle {
                layer: self.active_layer,
                fill_style,
                line_style: old_line,
            },
        );
        self.status_message = format!("Layer {name} line {}", next_line.label());
        true
    }

    pub(crate) fn set_active_layout_layer_hierarchy_depth(&mut self, value: &str) -> bool {
        let Some(layer) = self.workspace.document.layers.get(&self.active_layer) else {
            self.status_message = "Select a layer before changing layer depth".to_string();
            return true;
        };
        let layer_name = layer.name.clone();
        let next_depth = if value == "inherit" {
            None
        } else {
            let Some(depth) = LayoutHierarchyDepth::from_slug(value) else {
                return false;
            };
            if depth.shows_instance_boxes() {
                return false;
            }
            Some(depth)
        };
        let previous_depth = self
            .layout_layer_depth_overrides
            .get(&self.active_layer)
            .copied();
        if previous_depth == next_depth {
            self.status_message = format!(
                "Layer {layer_name} hierarchy depth already {}",
                layout_layer_depth_override_label(next_depth)
            );
            return true;
        }
        match next_depth {
            Some(depth) => {
                self.layout_layer_depth_overrides
                    .insert(self.active_layer, depth);
            }
            None => {
                self.layout_layer_depth_overrides.remove(&self.active_layer);
            }
        }
        self.invalidate_layout_view_caches();
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Layer {layer_name} hierarchy depth {}",
            layout_layer_depth_override_label(next_depth)
        );
        true
    }

    pub(crate) fn select_layout_shape(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u64>() else {
            return false;
        };
        let shape_id = ShapeId(id);
        let Some(shape) = self.workspace.document.shapes.get(&shape_id) else {
            return false;
        };
        self.selected_layout_shape = Some(shape_id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
        if self.layout_hierarchy_min_depth > 0 {
            self.layout_hierarchy_min_depth = 0;
            self.invalidate_layout_view_caches();
        }
        self.active_layer = shape.layer;
        self.status_message = format!("Selected shape #{}", shape.id.0);
        true
    }

    pub(crate) fn select_layout_occurrence(&mut self, value: &str) -> bool {
        let Some(occurrence) = parse_layout_occurrence_action_key(value) else {
            return false;
        };
        let Some(active_layer) = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
            .map(|view| view.shape.layer)
        else {
            return false;
        };

        self.ensure_layout_occurrence_depth_visible(&occurrence);
        self.selected_layout_shape = Some(occurrence.source_shape_id());
        self.selected_layout_occurrence = Some(occurrence.clone());
        self.active_layer = active_layer;
        self.status_message = format!("Selected {}", layout_occurrence_label(&occurrence));
        true
    }

    pub(crate) fn select_first_layout_shape_browser_match(&mut self) -> bool {
        let Some((occurrence, label)) = layout_shape_browser_entries(self).into_iter().next()
        else {
            self.status_message = "Shape browser has no matching shapes to select".to_string();
            return true;
        };
        let key = layout_occurrence_action_key(&occurrence);
        if !self.select_layout_occurrence(&key) {
            self.status_message = "Shape browser match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!(
            "Selected browser shape {}",
            compact_button_label(&label, 32)
        );
        true
    }

    pub(crate) fn select_first_layout_cell_browser_match(&mut self) -> bool {
        let Some((cell_id, label)) =
            layout_cell_browser_cells(self)
                .into_iter()
                .next()
                .map(|cell| {
                    (
                        cell.id,
                        layout_cell_browser_label(cell, self.workspace.document.top_cell),
                    )
                })
        else {
            self.status_message = "Cell browser has no matching cells to view".to_string();
            return true;
        };
        if !self.set_layout_view_top_cell(&cell_id.0.to_string()) {
            self.status_message = "Cell browser match is no longer viewable".to_string();
            return true;
        }
        self.status_message = format!("Viewing browser cell {}", compact_button_label(&label, 32));
        true
    }

    pub(crate) fn select_first_layout_instance_browser_match(&mut self) -> bool {
        let Some((parent, id, label)) = layout_instance_browser_entries(self).into_iter().next()
        else {
            self.status_message =
                "Instance browser has no matching instances to select".to_string();
            return true;
        };
        let key = layout_instance_action_key(parent, id);
        if !self.select_layout_instance(&key) {
            self.status_message = "Instance browser match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!(
            "Selected browser instance {}",
            compact_button_label(&label, 32)
        );
        true
    }

    pub(crate) fn select_layout_instance(&mut self, value: &str) -> bool {
        let Some((parent, id)) = parse_layout_instance_action_key(value) else {
            return false;
        };
        let Some((child_cell, child_name)) =
            self.workspace
                .document
                .instance(parent, id)
                .and_then(|instance| {
                    let child_name = self
                        .workspace
                        .document
                        .cell(instance.cell)
                        .map(|cell| cell.name.clone())?;
                    Some((instance.cell, child_name))
                })
        else {
            return false;
        };

        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.invalidate_layout_view_caches();
        }
        if self.layout_hidden_cells.remove(&child_cell) {
            self.invalidate_layout_view_caches();
        }

        let Some(occurrence) =
            first_layout_occurrence_for_instance(&self.workspace.document, parent, id)
        else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.status_message = format!(
                "Selected instance #{} -> cell #{} {}",
                id.0, child_cell.0, child_name
            );
            return true;
        };

        self.ensure_layout_occurrence_depth_visible(&occurrence);
        let active_layer = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
            .map(|view| view.shape.layer);
        self.selected_layout_shape = Some(occurrence.source_shape_id());
        self.selected_layout_occurrence = Some(occurrence.clone());
        if let Some(layer) = active_layer {
            self.active_layer = layer;
        }
        self.status_message = format!("Selected instance #{} -> {}", id.0, child_name);
        true
    }

    pub(crate) fn select_layout_net_component(&mut self, value: &str) -> bool {
        let Ok(component_id) = value.parse::<usize>() else {
            return false;
        };
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Connectivity failed: {error}");
                return true;
            }
        };
        let Some(component) = report.component(component_id) else {
            return false;
        };
        let Some(occurrence) = component.shapes.first().cloned() else {
            self.status_message = format!("Net component {component_id} has no shapes");
            return true;
        };
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.ensure_layout_occurrence_depth_visible(&occurrence);
        if layout_occurrence_hidden_by_cells(
            &self.workspace.document,
            self.layout_view_top_cell,
            &occurrence,
            &self.layout_hidden_cells,
        ) {
            self.layout_hidden_cells.clear();
        }
        self.invalidate_layout_view_caches();

        let active_layer = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
            .map(|view| view.shape.layer);
        self.selected_layout_shape = Some(occurrence.source_shape_id());
        self.selected_layout_occurrence = Some(occurrence);
        if let Some(layer) = active_layer {
            self.active_layer = layer;
        }
        let component_name = connectivity_component_display_name(component);
        self.record_layout_trace_history(component_id);
        self.status_message = format!("Net component {component_id}: {component_name}");
        true
    }

    pub(crate) fn select_first_layout_net_browser_match(&mut self) -> bool {
        let Some((component_id, label)) = layout_net_browser_entries(self).into_iter().next()
        else {
            self.status_message = "Net browser has no matching nets to select".to_string();
            return true;
        };
        if !self.select_layout_net_component(&component_id.to_string()) {
            self.status_message = "Net browser match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!("Selected browser net {}", compact_button_label(&label, 32));
        true
    }

    pub(crate) fn select_first_layout_trace_history_match(&mut self) -> bool {
        let Some((component_id, label)) = layout_trace_history_entries(self).into_iter().next()
        else {
            self.status_message = "Trace history has no matching nets to select".to_string();
            return true;
        };
        if !self.select_layout_net_component(&component_id.to_string()) {
            self.status_message = "Trace history match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!(
            "Selected trace history net {}",
            compact_button_label(&label, 32)
        );
        true
    }

    pub(crate) fn select_layout_drc_report(&mut self, value: &str) -> bool {
        let Ok(report_id) = value.parse::<u64>() else {
            return false;
        };
        let Some(entry) = self
            .layout_drc_report_history
            .iter()
            .find(|entry| entry.id == report_id)
            .cloned()
        else {
            return false;
        };
        self.layout_selected_drc_report_id = Some(entry.id);
        self.layout_selected_drc_marker_key = None;
        self.layout_drc_marker_category_filter = None;
        *self.drc_report_cache.get_mut() = Some(DrcReportCacheEntry {
            revision: entry.revision,
            value: entry.value,
        });
        self.show_drc_overlay = true;
        self.status_message = format!("Selected DRC report {}", entry.label);
        true
    }

    pub(crate) fn delete_layout_drc_report(&mut self, value: &str) -> bool {
        let Ok(report_id) = value.parse::<u64>() else {
            return false;
        };
        let Some(index) = self
            .layout_drc_report_history
            .iter()
            .position(|entry| entry.id == report_id)
        else {
            return false;
        };
        let removed = self.layout_drc_report_history.remove(index);
        if self.layout_selected_drc_report_id == Some(report_id) {
            self.layout_selected_drc_report_id =
                self.layout_drc_report_history.first().map(|entry| entry.id);
            self.layout_selected_drc_marker_key = None;
            self.sync_selected_layout_drc_report_cache();
        }
        self.status_message = format!("Deleted DRC report {}", removed.label);
        true
    }

    pub(crate) fn clear_layout_drc_reports(&mut self) -> bool {
        let count = self.layout_drc_report_history.len();
        self.layout_drc_report_history.clear();
        self.layout_selected_drc_report_id = None;
        self.layout_selected_drc_marker_key = None;
        self.drc_report_cache.get_mut().take();
        self.status_message = if count == 0 {
            "No DRC reports to clear".to_string()
        } else {
            format!("Cleared {count} DRC report(s)")
        };
        true
    }

    pub(crate) fn select_layout_drc_marker(&mut self, value: &str) -> bool {
        let Ok(marker_id) = value.parse::<usize>() else {
            return false;
        };
        let Some(report) = self.drc_report() else {
            self.status_message = "Run DRC before browsing markers".to_string();
            return true;
        };
        let Some(violation) = report
            .violations
            .iter()
            .find(|violation| violation.id == marker_id)
            .cloned()
        else {
            return false;
        };
        let key = violation.stable_key();
        self.layout_selected_drc_marker_key = Some(key.clone());
        self.mark_layout_drc_marker_visited(&key);
        self.show_drc_overlay = true;

        if let Some(occurrence) = violation.occurrence_ids.first().cloned() {
            self.layout_view_top_cell = self.workspace.document.top_cell;
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            if layout_occurrence_hidden_by_cells(
                &self.workspace.document,
                self.layout_view_top_cell,
                &occurrence,
                &self.layout_hidden_cells,
            ) {
                self.layout_hidden_cells.clear();
            }
            self.invalidate_layout_view_caches();

            let active_layer = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
                .map(|view| view.shape.layer);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(layer) = active_layer {
                self.active_layer = layer;
            }
        } else if let Some(shape_id) = violation
            .shape_ids
            .first()
            .copied()
            .filter(|shape_id| self.workspace.document.shapes.contains_key(shape_id))
        {
            self.selected_layout_shape = Some(shape_id);
            self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
            if self.layout_hierarchy_min_depth > 0 {
                self.layout_hierarchy_min_depth = 0;
                self.invalidate_layout_view_caches();
            }
        }

        self.center_layout_view_on_rect(violation.bounds);
        self.status_message = format!(
            "DRC marker {}: {}",
            violation.id,
            compact_button_label(&violation.message, 48)
        );
        true
    }

    pub(crate) fn select_first_layout_drc_marker_browser_match(&mut self) -> bool {
        let Some(entry) = layout_drc_marker_entries(self).into_iter().next() else {
            self.status_message =
                "DRC marker browser has no matching markers to select".to_string();
            return true;
        };
        if !self.select_layout_drc_marker(&entry.id.to_string()) {
            self.status_message = "DRC marker browser match is no longer selectable".to_string();
            return true;
        }
        self.status_message = format!(
            "Selected browser DRC marker {}",
            compact_button_label(&entry.label, 32)
        );
        true
    }

    pub(crate) fn center_layout_view_on_rect(&mut self, bounds: Rect) {
        let center = bounds.center();
        let mut state = self.current_layout_view_state();
        state.pan = [
            -(center.x as f32) * self.layout_zoom,
            center.y as f32 * self.layout_zoom,
        ];
        let current = self.current_layout_view_state();
        if current != state {
            self.layout_previous_view = Some(current);
            self.apply_layout_view_state(state);
        }
    }

    pub(crate) fn toggle_layout_reference_images(&mut self) -> bool {
        self.show_reference_images = !self.show_reference_images;
        self.reset_layout_frame_timing();
        self.status_message = format!(
            "Reference images {}",
            if self.show_reference_images {
                "shown"
            } else {
                "hidden"
            }
        );
        true
    }

    pub(crate) fn toggle_layout_reference_image(&mut self, value: &str) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        let Some(image) = self.workspace.document.reference_images.get_mut(index) else {
            self.status_message = format!("No reference image {index}");
            return true;
        };
        image.visible = !image.visible;
        if image.visible {
            self.show_reference_images = true;
        }
        let display_name = image.display_name().to_string();
        let visible = image.visible;
        self.mark_layout_dirty();
        self.status_message = format!(
            "Reference image {display_name} {}",
            if visible { "shown" } else { "hidden" }
        );
        true
    }

    pub(crate) fn adjust_layout_reference_image_opacity(
        &mut self,
        value: &str,
        delta: i16,
    ) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        let Some(image) = self.workspace.document.reference_images.get_mut(index) else {
            self.status_message = format!("No reference image {index}");
            return true;
        };
        let current = image.opacity as i16;
        let next = (current + delta).clamp(0, u8::MAX as i16) as u8;
        let changed = image.opacity != next;
        if changed {
            image.opacity = next;
        }
        if image.visible && next > 0 {
            self.show_reference_images = true;
        }
        let display_name = image.display_name().to_string();
        if changed {
            self.mark_layout_dirty();
        } else {
            self.reset_layout_frame_timing();
        }
        self.status_message = format!(
            "Reference image {display_name} opacity {}",
            reference_image_opacity_label(next)
        );
        true
    }

    pub(crate) fn move_layout_reference_image(&mut self, value: &str, direction: isize) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        let len = self.workspace.document.reference_images.len();
        if index >= len {
            self.status_message = format!("No reference image {index}");
            return true;
        }
        let next = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < len).then_some(index + 1)
        };
        let Some(next) = next else {
            let display_name = self.workspace.document.reference_images[index]
                .display_name()
                .to_string();
            self.status_message = format!("Reference image {display_name} already at edge");
            return true;
        };
        self.workspace.document.reference_images.swap(index, next);
        let display_name = self.workspace.document.reference_images[next]
            .display_name()
            .to_string();
        self.mark_layout_dirty();
        self.status_message = format!("Moved reference image {display_name} to slot {}", next + 1);
        true
    }

    pub(crate) fn focus_layout_reference_image(&mut self, value: &str) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        let Some(image) = self.workspace.document.reference_images.get_mut(index) else {
            self.status_message = format!("No reference image {index}");
            return true;
        };
        let display_name = image.display_name().to_string();
        let bounds = image.bounds;
        if !image.visible {
            image.visible = true;
            self.mark_layout_dirty();
        }
        self.show_reference_images = true;
        self.focus_layout_rect(
            bounds,
            &format!(
                "Focused reference image {}",
                compact_button_label(&display_name, 48)
            ),
        )
    }

    pub(crate) fn remove_layout_reference_image(&mut self, value: &str) -> bool {
        let Ok(index) = value.parse::<usize>() else {
            return false;
        };
        if index >= self.workspace.document.reference_images.len() {
            self.status_message = format!("No reference image {index}");
            return true;
        }
        let image = self.workspace.document.reference_images.remove(index);
        self.mark_layout_dirty();
        self.status_message = format!("Removed reference image {}", image.display_name());
        true
    }

    pub(crate) fn align_layout_reference_images(&mut self) -> bool {
        if self.workspace.document.reference_images.is_empty() {
            self.status_message = "No reference images to align".to_string();
            return true;
        }
        let mut aligned = 0usize;
        let mut warnings = 0usize;
        for image in &mut self.workspace.document.reference_images {
            match image.apply_landmark_alignment() {
                Ok(true) => aligned += 1,
                Ok(false) => {}
                Err(_) => warnings += 1,
            }
        }
        if aligned > 0 {
            self.show_reference_images = true;
            self.mark_layout_dirty();
        } else {
            self.reset_layout_frame_timing();
        }
        self.status_message =
            format!("Reference image alignment: {aligned} aligned, {warnings} warning(s)");
        true
    }

    pub(crate) fn seed_layout_reference_image_landmarks(&mut self) -> bool {
        if self.workspace.document.reference_images.is_empty() {
            self.status_message = "No reference images to seed".to_string();
            return true;
        }
        let mut seeded = 0usize;
        let mut skipped = 0usize;
        for image in &mut self.workspace.document.reference_images {
            if let Some(landmarks) = reference_image_corner_landmarks(image, image.bounds) {
                image.landmarks = landmarks;
                seeded += 1;
            } else {
                skipped += 1;
            }
        }
        if seeded > 0 {
            self.show_reference_images = true;
            self.mark_layout_dirty();
        } else {
            self.reset_layout_frame_timing();
        }
        self.status_message =
            format!("Seeded landmarks for {seeded} reference image(s), skipped {skipped}");
        true
    }

    pub(crate) fn fit_layout_reference_image_landmarks_to_selection(&mut self) -> bool {
        let Some(selection) = self.selected_layout_shape_ref() else {
            self.status_message =
                "Select a layout shape before fitting reference landmarks".to_string();
            return true;
        };
        let bounds = selection.kind.bounds();
        if bounds.width() <= 0 || bounds.height() <= 0 {
            self.status_message = "Selected shape has empty bounds".to_string();
            return true;
        }
        let Some(image) = self.workspace.document.reference_images.first_mut() else {
            self.status_message = "No reference images to fit".to_string();
            return true;
        };
        let display_name = image.display_name().to_string();
        let Some(landmarks) = reference_image_corner_landmarks(image, bounds) else {
            self.status_message =
                format!("Reference image {display_name} needs a valid pixel size");
            return true;
        };
        image.landmarks = landmarks;
        image.visible = true;
        let alignment = match image.apply_landmark_alignment() {
            Ok(true) => "aligned",
            Ok(false) => "already aligned",
            Err(error) => {
                self.status_message = format!("Reference image fit failed: {error}");
                return true;
            }
        };
        self.show_reference_images = true;
        self.mark_layout_dirty();
        self.status_message =
            format!("Fit reference image {display_name} to selection ({alignment})");
        true
    }

    pub(crate) fn clear_layout_reference_image_landmarks(&mut self) -> bool {
        if self.workspace.document.reference_images.is_empty() {
            self.status_message = "No reference image landmarks to clear".to_string();
            return true;
        }
        let mut cleared = 0usize;
        for image in &mut self.workspace.document.reference_images {
            if !image.landmarks.is_empty() {
                image.landmarks.clear();
                cleared += 1;
            }
        }
        if cleared > 0 {
            self.mark_layout_dirty();
        } else {
            self.reset_layout_frame_timing();
        }
        self.status_message = format!("Cleared landmarks on {cleared} reference image(s)");
        true
    }

    pub(crate) fn toggle_selected_layout_drc_marker_state(&mut self, value: &str) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let label = match value {
            "hidden" | "hide" => {
                state.hidden = !state.hidden;
                if state.hidden { "hidden" } else { "visible" }
            }
            "waived" | "waive" => {
                state.waived = !state.waived;
                if state.waived { "waived" } else { "unwaived" }
            }
            "visited" | "visit" => {
                state.visited = !state.visited;
                if state.visited {
                    "visited"
                } else {
                    "unvisited"
                }
            }
            "important" | "star" => {
                state.important = !state.important;
                if state.important {
                    "important"
                } else {
                    "normal priority"
                }
            }
            _ => return false,
        };
        self.write_layout_drc_marker_state(key, state);
        self.status_message = format!("DRC marker {label}");
        true
    }

    pub(crate) fn set_selected_layout_drc_marker_note(&mut self, value: &str) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let status = if value == "clear" {
            state.note = None;
            "DRC marker note cleared".to_string()
        } else {
            let Some((label, note)) = layout_drc_marker_note_preset(value) else {
                return false;
            };
            state.note = Some(note.to_string());
            format!("DRC marker {label}")
        };
        self.write_layout_drc_marker_state(key, state);
        self.status_message = status;
        true
    }

    pub(crate) fn set_selected_layout_drc_marker_owner(&mut self, value: &str) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let status = if value == "clear" {
            state.owner = None;
            "DRC marker owner cleared".to_string()
        } else {
            let Some((label, owner)) = layout_drc_marker_owner_preset(value) else {
                return false;
            };
            state.owner = Some(owner.to_string());
            format!("DRC marker {label}")
        };
        self.write_layout_drc_marker_state(key, state);
        self.status_message = status;
        true
    }

    pub(crate) fn set_selected_layout_drc_marker_signoff(&mut self, value: &str) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let status = if value == "clear" {
            state.signoff = None;
            "DRC marker signoff cleared".to_string()
        } else {
            let Some((label, signoff)) = layout_drc_marker_signoff_preset(value) else {
                return false;
            };
            state.signoff = Some(signoff.to_string());
            format!("DRC marker {label}")
        };
        self.write_layout_drc_marker_state(key, state);
        self.status_message = status;
        true
    }

    pub(crate) fn set_selected_layout_drc_marker_tag(&mut self, value: &str) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let status = if value == "clear" {
            state.tags.clear();
            "DRC marker tags cleared".to_string()
        } else {
            let Some((label, tag_key, tag_value)) = layout_drc_marker_tag_preset(value) else {
                return false;
            };
            state
                .tags
                .insert(tag_key.to_string(), tag_value.to_string());
            format!("DRC marker {label}")
        };
        self.write_layout_drc_marker_state(key, state);
        self.status_message = status;
        true
    }

    pub(crate) fn apply_selected_layout_drc_marker_custom_tag(&mut self) -> bool {
        let Some(marker_key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let tag_key = self.layout_browser_search.trim().to_string();
        if tag_key.is_empty() {
            self.status_message = "Enter a marker tag key in browser search".to_string();
            return true;
        }
        let tag_value = self.layout_browser_replace.trim().to_string();
        if tag_value.is_empty() {
            self.status_message = "Enter a marker tag value in browser replace".to_string();
            return true;
        }
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&marker_key)
            .cloned()
            .unwrap_or_default();
        state.tags.insert(tag_key.clone(), tag_value.clone());
        self.write_layout_drc_marker_state(marker_key, state);
        self.status_message = format!(
            "DRC marker tag {}",
            compact_button_label(&format!("{tag_key}={tag_value}"), 48)
        );
        true
    }

    pub(crate) fn remove_selected_layout_drc_marker_custom_tag(&mut self) -> bool {
        let Some(marker_key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let tag_key = self.layout_browser_search.trim().to_string();
        if tag_key.is_empty() {
            self.status_message = "Enter a marker tag key in browser search".to_string();
            return true;
        }
        let Some(mut state) = self
            .workspace
            .document
            .marker_states
            .get(&marker_key)
            .cloned()
        else {
            self.status_message = format!("DRC marker tag {tag_key} not found");
            return true;
        };
        if state.tags.remove(&tag_key).is_none() {
            self.status_message = format!("DRC marker tag {tag_key} not found");
            return true;
        }
        self.write_layout_drc_marker_state(marker_key, state);
        self.status_message = format!("DRC marker tag {tag_key} removed");
        true
    }

    pub(crate) fn selected_layout_shape_source_for_property_edit(
        &self,
    ) -> Option<(CellId, ShapeOccurrenceId, Shape)> {
        if let Some(occurrence) = &self.selected_layout_occurrence
            && let Some(view) = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)
        {
            return Some((view.source_cell, occurrence.clone(), view.shape.to_shape()));
        }
        let shape = self.selected_top_level_layout_shape()?;
        Some((
            self.workspace.document.top_cell,
            ShapeOccurrenceId::top_level(shape.id),
            shape,
        ))
    }

    pub(crate) fn apply_selected_layout_shape_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter a shape property key in browser search".to_string();
            return true;
        }
        let value = self.layout_browser_replace.trim().to_string();
        if value.is_empty() {
            self.status_message = "Enter a shape property value in browser replace".to_string();
            return true;
        }
        let Some((source_cell, occurrence, old_shape)) =
            self.selected_layout_shape_source_for_property_edit()
        else {
            self.status_message = "Select a shape before setting a property".to_string();
            return true;
        };
        if old_shape.properties.get(&key) == Some(&value) {
            self.status_message = format!(
                "Shape property {} already set",
                compact_button_label(&key, 32)
            );
            return true;
        }
        let mut new_shape = old_shape.clone();
        new_shape.properties.insert(key.clone(), value.clone());
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            format!(
                "Shape property {}",
                compact_button_label(&format!("{key}={value}"), 48)
            ),
        );
        true
    }

    pub(crate) fn remove_selected_layout_shape_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter a shape property key in browser search".to_string();
            return true;
        }
        let Some((source_cell, occurrence, old_shape)) =
            self.selected_layout_shape_source_for_property_edit()
        else {
            self.status_message = "Select a shape before removing a property".to_string();
            return true;
        };
        if !old_shape.properties.contains_key(&key) {
            self.status_message = format!("Shape property {key} not found");
            return true;
        }
        let mut new_shape = old_shape.clone();
        new_shape.properties.remove(&key);
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            format!("Shape property {key} removed"),
        );
        true
    }

    pub(crate) fn apply_selected_layout_instance_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter an instance property key in browser search".to_string();
            return true;
        }
        let value = self.layout_browser_replace.trim().to_string();
        if value.is_empty() {
            self.status_message = "Enter an instance property value in browser replace".to_string();
            return true;
        }
        let Some((parent, id, old_instance)) = self.selected_layout_instance_context() else {
            self.status_message = "Select an instance before setting a property".to_string();
            return true;
        };
        if old_instance.properties.get(&key) == Some(&value) {
            self.status_message = format!(
                "Instance property {} already set",
                compact_button_label(&key, 32)
            );
            return true;
        }
        let mut new_instance = old_instance.clone();
        new_instance.properties.insert(key.clone(), value.clone());
        self.apply_layout_operation_with_history(
            Operation::ReplaceInstance {
                parent,
                id,
                instance: new_instance,
            },
            Operation::ReplaceInstance {
                parent,
                id,
                instance: old_instance,
            },
        );
        self.status_message = format!(
            "Instance property {}",
            compact_button_label(&format!("{key}={value}"), 48)
        );
        true
    }

    pub(crate) fn remove_selected_layout_instance_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter an instance property key in browser search".to_string();
            return true;
        }
        let Some((parent, id, old_instance)) = self.selected_layout_instance_context() else {
            self.status_message = "Select an instance before removing a property".to_string();
            return true;
        };
        if !old_instance.properties.contains_key(&key) {
            self.status_message = format!("Instance property {key} not found");
            return true;
        }
        let mut new_instance = old_instance.clone();
        new_instance.properties.remove(&key);
        self.apply_layout_operation_with_history(
            Operation::ReplaceInstance {
                parent,
                id,
                instance: new_instance,
            },
            Operation::ReplaceInstance {
                parent,
                id,
                instance: old_instance,
            },
        );
        self.status_message = format!("Instance property {key} removed");
        true
    }

    pub(crate) fn apply_layout_view_cell_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter a cell property key in browser search".to_string();
            return true;
        }
        let value = self.layout_browser_replace.trim().to_string();
        if value.is_empty() {
            self.status_message = "Enter a cell property value in browser replace".to_string();
            return true;
        }
        let cell_id = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        if cell.properties.get(&key) == Some(&value) {
            self.status_message = format!(
                "Cell property {} already set",
                compact_button_label(&key, 32)
            );
            return true;
        }
        let old_properties = cell.properties.clone();
        let mut new_properties = old_properties.clone();
        new_properties.insert(key.clone(), value.clone());
        self.apply_layout_operation_with_history(
            Operation::SetCellProperties {
                id: cell_id,
                properties: new_properties,
            },
            Operation::SetCellProperties {
                id: cell_id,
                properties: old_properties,
            },
        );
        self.status_message = format!(
            "Cell property {}",
            compact_button_label(&format!("{key}={value}"), 48)
        );
        true
    }

    pub(crate) fn remove_layout_view_cell_custom_property(&mut self) -> bool {
        let key = self.layout_browser_search.trim().to_string();
        if key.is_empty() {
            self.status_message = "Enter a cell property key in browser search".to_string();
            return true;
        }
        let cell_id = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        if !cell.properties.contains_key(&key) {
            self.status_message = format!("Cell property {key} not found");
            return true;
        }
        let old_properties = cell.properties.clone();
        let mut new_properties = old_properties.clone();
        new_properties.remove(&key);
        self.apply_layout_operation_with_history(
            Operation::SetCellProperties {
                id: cell_id,
                properties: new_properties,
            },
            Operation::SetCellProperties {
                id: cell_id,
                properties: old_properties,
            },
        );
        self.status_message = format!("Cell property {key} removed");
        true
    }

    pub(crate) fn apply_layout_shape_source_replacement(
        &mut self,
        source_cell: CellId,
        occurrence: ShapeOccurrenceId,
        old_shape: Shape,
        new_shape: Shape,
        status: String,
    ) {
        let shape_id = old_shape.id;
        if source_cell == self.workspace.document.top_cell
            && self.workspace.document.shapes.contains_key(&shape_id)
        {
            self.apply_layout_operation_with_history(
                Operation::ReplaceShape {
                    id: shape_id,
                    shape: new_shape.clone(),
                },
                Operation::ReplaceShape {
                    id: shape_id,
                    shape: old_shape,
                },
            );
        } else {
            self.apply_layout_operation_with_history(
                Operation::Batch {
                    operations: vec![
                        Operation::DeleteShapeFromCell {
                            cell: source_cell,
                            id: old_shape.id,
                        },
                        Operation::AddShapeToCell {
                            cell: source_cell,
                            shape: new_shape.clone(),
                        },
                    ],
                },
                Operation::Batch {
                    operations: vec![
                        Operation::DeleteShapeFromCell {
                            cell: source_cell,
                            id: new_shape.id,
                        },
                        Operation::AddShapeToCell {
                            cell: source_cell,
                            shape: old_shape,
                        },
                    ],
                },
            );
        }
        self.selected_layout_shape = Some(shape_id);
        self.selected_layout_occurrence = Some(occurrence);
        self.active_layer = new_shape.layer;
        self.status_message = status;
    }

    pub(crate) fn export_selected_layout_drc_marker_snapshot(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_drc_marker_snapshot_path();
            return self.export_selected_layout_drc_marker_snapshot_to_path(&path, 640, 480);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "DRC marker snapshot export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_selected_layout_drc_marker_snapshot_to_path(
        &mut self,
        path: &Path,
        width: u32,
        height: u32,
    ) -> bool {
        let Some(marker_key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        let Some(violation) = selected_layout_drc_marker_violation(self) else {
            self.status_message = "Selected DRC marker is not in the active report".to_string();
            return true;
        };

        self.active_view = StartupView::Layout2d;
        self.show_drc_overlay = true;
        self.center_layout_view_on_rect(violation.bounds);

        let ui_scale = UiScale::new(self.app_options.appearance.ui_scale);
        let report = match self.render_operad_snapshot_scaled(width, height, ui_scale) {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("DRC marker snapshot export failed: {error}");
                return true;
            }
        };
        if let Err(error) = write_operad_snapshot_rgba(path, &report) {
            self.status_message = format!("DRC marker snapshot export failed: {error}");
            return true;
        }
        let snapshot = report
            .render
            .snapshot
            .as_ref()
            .map(|image| format!("{}x{}", image.size.width, image.size.height))
            .unwrap_or_else(|| format!("{}x{}", width.max(1), height.max(1)));
        let bounds = rect_summary(violation.bounds);
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(&marker_key)
            .cloned()
            .unwrap_or_default();
        state
            .tags
            .insert("screenshot".to_string(), path.display().to_string());
        state.tags.insert("screenshot_bounds".to_string(), bounds);
        state
            .tags
            .insert("screenshot_size".to_string(), snapshot.clone());
        self.write_layout_drc_marker_state(marker_key, state);
        self.status_message = format!(
            "Exported DRC marker snapshot {} ({snapshot} RGBA)",
            path.display()
        );
        true
    }

    pub(crate) fn mark_layout_drc_marker_visited(&mut self, key: &str) {
        let mut state = self
            .workspace
            .document
            .marker_states
            .get(key)
            .cloned()
            .unwrap_or_default();
        if state.visited {
            return;
        }
        state.visited = true;
        self.write_layout_drc_marker_state(key.to_string(), state);
    }

    pub(crate) fn write_layout_drc_marker_state(&mut self, key: String, state: MarkerState) {
        let next_state = (state != MarkerState::default()).then_some(state);
        self.workspace
            .document
            .apply_operation_without_log(&Operation::SetMarkerState {
                key,
                state: next_state,
            });
        self.layout_view_revision = self.layout_view_revision.wrapping_add(1);
        self.reset_layout_frame_timing();
    }
}
