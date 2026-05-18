#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn clear_selected_layout_drc_marker_state(&mut self) -> bool {
        let Some(key) = self.layout_selected_drc_marker_key.clone() else {
            self.status_message = "Select a DRC marker first".to_string();
            return true;
        };
        self.workspace
            .document
            .apply_operation_without_log(&Operation::SetMarkerState { key, state: None });
        self.layout_view_revision = self.layout_view_revision.wrapping_add(1);
        self.reset_layout_frame_timing();
        self.status_message = "DRC marker state cleared".to_string();
        true
    }

    pub(crate) fn record_layout_trace_history(&mut self, component_id: usize) {
        self.layout_trace_history
            .retain(|history_id| *history_id != component_id);
        self.layout_trace_history.insert(0, component_id);
        self.layout_trace_history.truncate(MAX_LAYOUT_TRACE_HISTORY);
    }

    pub(crate) fn clear_layout_trace_history(&mut self) -> bool {
        let cleared_count = self.layout_trace_history.len();
        if cleared_count == 0 {
            self.status_message = "Trace history is already empty".to_string();
            return true;
        }
        self.layout_trace_history.clear();
        self.status_message = format!(
            "Cleared {cleared_count} traced net{}",
            if cleared_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn trace_all_layout_nets(&mut self) -> bool {
        if self.workspace.document.flattened_shape_count_estimate()
            > MAX_CONNECTIVITY_OVERLAY_SHAPES
        {
            self.status_message = format!(
                "Trace all skipped over {} shapes",
                self.workspace.document.flattened_shape_count_estimate()
            );
            return true;
        }

        let (component_count, history, selected_component) = {
            let report = match self.connectivity_report() {
                Ok(report) => report,
                Err(error) => {
                    self.status_message = format!("Connectivity failed: {error}");
                    return true;
                }
            };
            let mut components = report.components.iter().collect::<Vec<_>>();
            components.sort_by(|left, right| {
                right
                    .shapes
                    .len()
                    .cmp(&left.shapes.len())
                    .then_with(|| left.id.cmp(&right.id))
            });
            (
                components.len(),
                components
                    .iter()
                    .take(MAX_LAYOUT_TRACE_HISTORY)
                    .map(|component| component.id)
                    .collect::<Vec<_>>(),
                components.first().map(|component| component.id),
            )
        };

        self.layout_net_browser_filter = LayoutNetBrowserFilter::All;
        self.layout_net_browser_sort = LayoutNetBrowserSort::Size;
        self.layout_browser_search.clear();
        self.layout_browser_search_active = false;
        self.layout_trace_history = history.clone();

        let Some(component_id) = selected_component else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.status_message = "Trace all found no net components".to_string();
            return true;
        };

        let selected = self.select_layout_net_component(&component_id.to_string());
        self.layout_trace_history = history;
        if selected {
            self.status_message =
                format!("Traced all {component_count} net component(s); selected #{component_id}");
        }
        true
    }

    pub(crate) fn top_layout_occurrence_at_trace_point(
        &self,
        point: Point,
    ) -> Result<ShapeOccurrenceId, String> {
        let Some(view_occurrence) = self.with_layout_display_index(|index| {
            index.hit_test_occurrence(point, self.layout_hit_tolerance())
        }) else {
            return Err("No traceable shape at point".to_string());
        };
        layout_top_occurrence_for_view_occurrence(
            &self.workspace.document,
            self.layout_view_top_cell,
            &view_occurrence,
        )
        .ok_or_else(|| "No document-top context for traced cell".to_string())
    }

    pub(crate) fn trace_layout_between_route_points(&mut self) -> bool {
        if self.route_points.len() < 2 {
            self.status_message = "Set two route points before tracing between them".to_string();
            return true;
        }
        if self.workspace.document.flattened_shape_count_estimate()
            > MAX_CONNECTIVITY_OVERLAY_SHAPES
        {
            self.status_message = format!(
                "Trace path skipped over {} shapes",
                self.workspace.document.flattened_shape_count_estimate()
            );
            return true;
        }

        let start = self.route_points[0];
        let goal = *self.route_points.last().unwrap_or(&start);
        let start_occurrence = match self.top_layout_occurrence_at_trace_point(start) {
            Ok(occurrence) => occurrence,
            Err(error) => {
                self.status_message = format!("Trace path start failed: {error}");
                return true;
            }
        };
        let goal_occurrence = match self.top_layout_occurrence_at_trace_point(goal) {
            Ok(occurrence) => occurrence,
            Err(error) => {
                self.status_message = format!("Trace path end failed: {error}");
                return true;
            }
        };
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Connectivity failed: {error}");
                return true;
            }
        };
        let Some(start_component) = report.component_for_occurrence(&start_occurrence) else {
            self.status_message = format!(
                "Trace path start has no extracted net: {}",
                layout_occurrence_label(&start_occurrence)
            );
            return true;
        };
        let Some(goal_component) = report.component_for_occurrence(&goal_occurrence) else {
            self.status_message = format!(
                "Trace path end has no extracted net: {}",
                layout_occurrence_label(&goal_occurrence)
            );
            return true;
        };
        if start_component != goal_component {
            self.status_message = format!(
                "Trace path endpoints are disconnected: #{start_component} to #{goal_component}"
            );
            return true;
        }

        let component_name = report
            .component(start_component)
            .map(connectivity_component_display_name)
            .unwrap_or_else(|| format!("component {start_component}"));
        let selected = self.select_layout_net_component(&start_component.to_string());
        if selected {
            self.status_message = format!(
                "Trace path connected route points on net component {start_component}: {component_name}"
            );
        }
        true
    }

    pub(crate) fn trace_layout_net_at_point(&mut self, point: Point) -> bool {
        if self.workspace.document.flattened_shape_count_estimate()
            > MAX_CONNECTIVITY_OVERLAY_SHAPES
        {
            self.status_message = format!(
                "Trace skipped over {} shapes",
                self.workspace.document.flattened_shape_count_estimate()
            );
            return true;
        }

        let occurrence = match self.top_layout_occurrence_at_trace_point(point) {
            Ok(occurrence) => occurrence,
            Err(error) => {
                self.status_message = error;
                return true;
            }
        };

        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Connectivity failed: {error}");
                return true;
            }
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
        self.selected_layout_occurrence = Some(occurrence.clone());
        if let Some(layer) = active_layer {
            self.active_layer = layer;
        }

        if let Some(component_id) = report.component_for_occurrence(&occurrence)
            && let Some(component) = report.component(component_id)
        {
            let component_name = connectivity_component_display_name(component);
            self.record_layout_trace_history(component_id);
            self.status_message = format!("Traced net component {component_id}: {component_name}");
        } else {
            self.status_message = format!(
                "No extracted net for {}",
                layout_occurrence_label(&occurrence)
            );
        }
        true
    }

    pub(crate) fn select_next_layout_shape(&mut self) -> bool {
        let shape_ids = layout_shape_ids(&self.workspace.document);
        let Some(next) = next_layout_shape_id(self.selected_layout_shape, &shape_ids) else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.status_message = "No layout shapes to select".to_string();
            return true;
        };
        self.select_layout_shape(&next.0.to_string())
    }

    pub(crate) fn add_layout_layer(&mut self) -> bool {
        let index = self.workspace.document.layers.len() + 1;
        let id = LayerId(self.workspace.document.next_layer_id);
        let layer = Layer {
            id,
            name: format!("Layer {index}"),
            process: ProcessLayer::Annotation,
            purpose: ProcessLayer::Annotation.as_technology_name().to_string(),
            color: [0.55, 0.76, 0.92, 0.35],
            fill_style: LayerFillStyle::Solid,
            line_style: LayerLineStyle::Solid,
            display_order: id.0 as i32 * 10,
            gds_layer: u16::try_from(id.0).ok(),
            gds_datatype: 0,
            gds_texttype: 0,
            visible: true,
            locked: false,
        };
        self.apply_layout_operation_with_history(
            Operation::AddLayer {
                layer: layer.clone(),
            },
            Operation::DeleteLayer { id },
        );
        self.active_layer = id;
        self.status_message = format!("Added {}", layer.name);
        true
    }

    pub(crate) fn remove_layout_layer(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u32>() else {
            return false;
        };
        let layer_id = LayerId(id);
        let Some(layer) = self.workspace.document.layers.get(&layer_id).cloned() else {
            return false;
        };
        let removed_shapes = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| shape.layer == layer_id)
            .collect::<Vec<_>>();
        let undo_operations = std::iter::once(Operation::AddLayer {
            layer: layer.clone(),
        })
        .chain(
            removed_shapes
                .iter()
                .cloned()
                .map(|shape| Operation::AddShape { shape }),
        )
        .collect();
        self.apply_layout_operation_with_history(
            Operation::DeleteLayer { id: layer_id },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        if self.active_layer == layer_id {
            self.active_layer = default_active_layer(&self.workspace);
        }
        self.layout_layer_depth_overrides.remove(&layer_id);
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Removed {} with {} shape{}",
            layer.name,
            removed_shapes.len(),
            if removed_shapes.len() == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn update_layout_drag_to(
        &mut self,
        world: Point,
        modifiers: operad::KeyModifiers,
    ) -> bool {
        let Some(drag) = self.layout_drag.clone() else {
            return false;
        };
        match drag {
            LayoutCanvasDrag::MoveShape {
                shape_id,
                start_world,
                last_world,
                copy_on_drag,
                added_shape,
            } => {
                let drag_world = layout_drag_world(start_world, world, modifiers);
                let mut shape_id = shape_id;
                let mut copy_on_drag = copy_on_drag;
                let mut added_shape = added_shape;
                if copy_on_drag {
                    if let Some((new_id, shape)) = self.duplicate_layout_shape_for_drag(shape_id) {
                        shape_id = new_id;
                        added_shape = Some(shape);
                    }
                    copy_on_drag = false;
                }
                let delta = Vector::new(drag_world.x - last_world.x, drag_world.y - last_world.y);
                if delta != Vector::ZERO {
                    self.apply_layout_operation_live(Operation::MoveShape {
                        id: shape_id,
                        delta,
                    });
                }
                self.layout_drag = Some(LayoutCanvasDrag::MoveShape {
                    shape_id,
                    start_world,
                    last_world: drag_world,
                    copy_on_drag,
                    added_shape,
                });
                delta != Vector::ZERO
            }
            LayoutCanvasDrag::MoveInstance {
                parent,
                instance_id,
                start_world,
                last_world,
                copy_on_drag,
                added_instance,
            } => {
                let drag_world = layout_drag_world(start_world, world, modifiers);
                let mut instance_id = instance_id;
                let mut copy_on_drag = copy_on_drag;
                let mut added_instance = added_instance;
                if copy_on_drag {
                    if let Some((new_id, instance)) =
                        self.duplicate_layout_instance_for_drag(parent, instance_id)
                    {
                        instance_id = new_id;
                        added_instance = Some(instance);
                    }
                    copy_on_drag = false;
                }
                let delta = Vector::new(drag_world.x - last_world.x, drag_world.y - last_world.y);
                if delta != Vector::ZERO {
                    self.apply_layout_operation_live(Operation::MoveInstance {
                        parent,
                        id: instance_id,
                        delta,
                    });
                }
                self.layout_drag = Some(LayoutCanvasDrag::MoveInstance {
                    parent,
                    instance_id,
                    start_world,
                    last_world: drag_world,
                    copy_on_drag,
                    added_instance,
                });
                delta != Vector::ZERO
            }
            LayoutCanvasDrag::MoveVertex {
                shape_id,
                vertex,
                start_world,
                original_position,
                original_shape,
            } => {
                let position =
                    layout_vertex_drag_position(original_position, start_world, world, modifiers);
                let Some(old_shape) = self.workspace.document.shapes.get(&shape_id) else {
                    return false;
                };
                let Some(new_shape) = shape_with_moved_vertex(&old_shape, vertex, position) else {
                    return false;
                };
                let changed =
                    self.replace_layout_shape_live(shape_id, old_shape, new_shape, "Edited vertex");
                self.layout_drag = Some(LayoutCanvasDrag::MoveVertex {
                    shape_id,
                    vertex,
                    start_world,
                    original_position,
                    original_shape,
                });
                changed
            }
            LayoutCanvasDrag::MoveEdge {
                shape_id,
                edge,
                last_world,
                original_shape,
            } => {
                let drag_world = layout_drag_world(last_world, world, modifiers);
                let delta = Vector::new(drag_world.x - last_world.x, drag_world.y - last_world.y);
                if delta == Vector::ZERO {
                    return false;
                }
                let Some(old_shape) = self.workspace.document.shapes.get(&shape_id) else {
                    return false;
                };
                let Some(new_shape) = shape_with_moved_edge(&old_shape, edge, delta) else {
                    return false;
                };
                let changed =
                    self.replace_layout_shape_live(shape_id, old_shape, new_shape, "Moved edge");
                self.layout_drag = Some(LayoutCanvasDrag::MoveEdge {
                    shape_id,
                    edge,
                    last_world: drag_world,
                    original_shape,
                });
                changed
            }
            LayoutCanvasDrag::Rect { .. } => false,
        }
    }

    pub(crate) fn commit_layout_drag_history(&mut self, drag: LayoutCanvasDrag) -> bool {
        match drag {
            LayoutCanvasDrag::MoveShape {
                shape_id,
                start_world,
                last_world,
                added_shape,
                ..
            } => {
                let total_delta =
                    Vector::new(last_world.x - start_world.x, last_world.y - start_world.y);
                if let Some(shape) = added_shape {
                    let mut redo = vec![Operation::AddShape {
                        shape: shape.clone(),
                    }];
                    if total_delta != Vector::ZERO {
                        redo.push(Operation::MoveShape {
                            id: shape_id,
                            delta: total_delta,
                        });
                    }
                    self.push_layout_history(
                        Operation::Batch { operations: redo },
                        Operation::DeleteShape { id: shape_id },
                    );
                    return true;
                }
                if total_delta == Vector::ZERO {
                    return false;
                }
                self.push_layout_history(
                    Operation::MoveShape {
                        id: shape_id,
                        delta: total_delta,
                    },
                    Operation::MoveShape {
                        id: shape_id,
                        delta: Vector::new(-total_delta.dx, -total_delta.dy),
                    },
                );
                true
            }
            LayoutCanvasDrag::MoveInstance {
                parent,
                instance_id,
                start_world,
                last_world,
                added_instance,
                ..
            } => {
                let total_delta =
                    Vector::new(last_world.x - start_world.x, last_world.y - start_world.y);
                if let Some(instance) = added_instance {
                    let mut redo = vec![Operation::AddInstance {
                        parent,
                        instance: instance.clone(),
                    }];
                    if total_delta != Vector::ZERO {
                        redo.push(Operation::MoveInstance {
                            parent,
                            id: instance_id,
                            delta: total_delta,
                        });
                    }
                    self.push_layout_history(
                        Operation::Batch { operations: redo },
                        Operation::DeleteInstance {
                            parent,
                            id: instance_id,
                        },
                    );
                    return true;
                }
                if total_delta == Vector::ZERO {
                    return false;
                }
                self.push_layout_history(
                    Operation::MoveInstance {
                        parent,
                        id: instance_id,
                        delta: total_delta,
                    },
                    Operation::MoveInstance {
                        parent,
                        id: instance_id,
                        delta: Vector::new(-total_delta.dx, -total_delta.dy),
                    },
                );
                true
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
            } => {
                let Some(final_shape) = self.workspace.document.shapes.get(&shape_id) else {
                    return false;
                };
                if final_shape == original_shape {
                    return false;
                }
                self.push_layout_history(
                    Operation::ReplaceShape {
                        id: shape_id,
                        shape: final_shape,
                    },
                    Operation::ReplaceShape {
                        id: shape_id,
                        shape: original_shape,
                    },
                );
                true
            }
            LayoutCanvasDrag::Rect { .. } => false,
        }
    }

    pub(crate) fn apply_layout_operation_live(&mut self, operation: Operation) -> bool {
        self.workspace
            .document
            .apply_operation_without_log(&operation);
        self.mark_layout_dirty();
        true
    }

    pub(crate) fn push_layout_history(&mut self, redo: Operation, undo: Operation) {
        self.layout_undo.push(UndoEntry { undo, redo });
        self.layout_redo.clear();
    }

    pub(crate) fn mark_layout_dirty(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
        self.layout_view_revision = self.layout_view_revision.wrapping_add(1);
        self.repair_layout_view_top_cell();
        self.layout_index_cache.get_mut().take();
        self.connectivity_report_cache.get_mut().take();
        self.layout_spice_comparison = None;
        self.drc_report_cache.get_mut().take();
        self.layout_trace_history.clear();
        self.layout_selected_drc_report_id = None;
        self.layout_selected_drc_marker_key = None;
    }

    pub(crate) fn invalidate_layout_view_caches(&mut self) {
        self.layout_view_revision = self.layout_view_revision.wrapping_add(1);
        self.layout_index_cache.get_mut().take();
        self.reset_layout_frame_timing();
    }

    pub(crate) fn repair_layout_view_top_cell(&mut self) {
        if !self
            .workspace
            .document
            .cells
            .contains_key(&self.layout_view_top_cell)
        {
            self.layout_view_top_cell = self.workspace.document.top_cell;
        }
    }

    pub(crate) fn apply_layout_operation_with_history(
        &mut self,
        redo: Operation,
        undo: Operation,
    ) -> bool {
        self.workspace.document.apply_operation_without_log(&redo);
        self.mark_layout_dirty();
        self.push_layout_history(redo, undo);
        true
    }

    pub(crate) fn undo_layout_operation(&mut self) -> bool {
        let Some(entry) = self.layout_undo.pop() else {
            self.status_message = "Nothing to undo".to_string();
            return true;
        };
        self.workspace
            .document
            .apply_operation_without_log(&entry.undo);
        self.mark_layout_dirty();
        self.layout_redo.push(entry);
        self.layout_drag = None;
        self.repair_layout_selection_after_operation();
        self.status_message = "Undo".to_string();
        true
    }

    pub(crate) fn redo_layout_operation(&mut self) -> bool {
        let Some(entry) = self.layout_redo.pop() else {
            self.status_message = "Nothing to redo".to_string();
            return true;
        };
        self.workspace
            .document
            .apply_operation_without_log(&entry.redo);
        self.mark_layout_dirty();
        self.layout_undo.push(entry);
        self.layout_drag = None;
        self.repair_layout_selection_after_operation();
        self.status_message = "Redo".to_string();
        true
    }

    pub(crate) fn repair_layout_selection_after_operation(&mut self) {
        if let Some(occurrence) = &self.selected_layout_occurrence {
            if !self.layout_occurrence_within_display_depth(occurrence) {
                self.selected_layout_shape = None;
                self.selected_layout_occurrence = None;
                self.layout_drag = None;
                return;
            }
            if self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)
                .is_some()
            {
                self.selected_layout_shape = Some(occurrence.source_shape_id());
                return;
            }
        }
        if let Some(shape_id) = self.selected_layout_shape {
            if self.workspace.document.shapes.contains_key(&shape_id) {
                self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
                return;
            }
        }
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
    }

    pub(crate) fn layout_occurrence_within_display_depth(
        &self,
        occurrence: &ShapeOccurrenceId,
    ) -> bool {
        layout_depth_includes_occurrence(
            self.layout_hierarchy_depth,
            self.layout_hierarchy_min_depth,
            occurrence,
        ) && !layout_occurrence_hidden_by_cells(
            &self.workspace.document,
            self.layout_view_top_cell,
            occurrence,
            &self.layout_hidden_cells,
        )
    }

    pub(crate) fn selected_layout_shape_ref(&self) -> Option<Shape> {
        if let Some(occurrence) = &self.selected_layout_occurrence
            && let Some(view) = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)
        {
            return Some(view.transformed_shape());
        }
        self.selected_layout_shape.and_then(|id| {
            self.workspace.document.shapes.get(&id).inspect(|_| {
                debug_assert!(
                    self.selected_layout_occurrence
                        .as_ref()
                        .is_none_or(ShapeOccurrenceId::is_top_level)
                );
            })
        })
    }

    pub(crate) fn selected_top_level_layout_shape(&self) -> Option<Shape> {
        let id = self.selected_layout_shape?;
        if self
            .selected_layout_occurrence
            .as_ref()
            .is_none_or(|occurrence| {
                occurrence.is_top_level() && occurrence.source_shape_id() == id
            })
        {
            self.workspace.document.shapes.get(&id)
        } else {
            None
        }
    }

    pub(crate) fn copy_layout_selection(&mut self) -> bool {
        let Some(shape) = self.selected_top_level_layout_shape() else {
            self.status_message = "Select a layout shape to copy".to_string();
            return true;
        };
        self.layout_clipboard_shapes.clear();
        self.layout_clipboard_shapes.push(shape);
        self.status_message = "Copied 1 shape".to_string();
        true
    }

    pub(crate) fn paste_layout_clipboard(&mut self) -> bool {
        if self.layout_clipboard_shapes.is_empty() {
            self.status_message = "Layout clipboard is empty".to_string();
            return true;
        }
        let shapes = self.layout_clipboard_shapes.clone();
        self.add_copied_layout_shapes(shapes, "Pasted")
    }

    pub(crate) fn duplicate_layout_selection(&mut self) -> bool {
        let Some(shape) = self.selected_top_level_layout_shape() else {
            self.status_message = "Select a layout shape to duplicate".to_string();
            return true;
        };
        self.layout_clipboard_shapes.clear();
        self.layout_clipboard_shapes.push(shape.clone());
        self.add_copied_layout_shapes(vec![shape], "Duplicated")
    }

    pub(crate) fn duplicate_layout_shape_for_drag(
        &mut self,
        shape_id: ShapeId,
    ) -> Option<(ShapeId, Shape)> {
        let mut shape = self.workspace.document.shapes.get(&shape_id)?.clone();
        shape.id = self.workspace.document.allocate_shape_id();
        let new_id = shape.id;
        self.apply_layout_operation_live(Operation::AddShape {
            shape: shape.clone(),
        });
        self.selected_layout_shape = Some(new_id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(new_id));
        self.status_message = format!("Duplicated shape #{}", shape_id.0);
        Some((new_id, shape))
    }

    pub(crate) fn duplicate_layout_instance_for_drag(
        &mut self,
        parent: CellId,
        instance_id: InstanceId,
    ) -> Option<(InstanceId, CellInstance)> {
        let mut instance = self.workspace.document.instance(parent, instance_id)?;
        instance.id = self.workspace.document.allocate_instance_id();
        let new_id = instance.id;
        self.apply_layout_operation_live(Operation::AddInstance {
            parent,
            instance: instance.clone(),
        });
        if let Some(mut occurrence) = self.selected_layout_occurrence.clone() {
            if let Some(last) = occurrence.instance_path.last_mut() {
                *last = new_id;
            }
            self.selected_layout_occurrence = Some(occurrence);
        }
        self.status_message = format!("Duplicated instance {}", instance_id.0);
        Some((new_id, instance))
    }

    pub(crate) fn add_copied_layout_shapes(&mut self, shapes: Vec<Shape>, label: &str) -> bool {
        let offset = (self.workspace.document.grid.max(10) * 8).max(80);
        let mut added = Vec::with_capacity(shapes.len());
        for mut shape in shapes {
            shape.id = self.workspace.document.allocate_shape_id();
            shape.kind.translate(Vector::new(offset, -offset));
            added.push(shape);
        }
        if added.is_empty() {
            return true;
        }
        let selected = added.first().map(|shape| shape.id);
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: added
                    .iter()
                    .cloned()
                    .map(|shape| Operation::AddShape { shape })
                    .collect(),
            },
            Operation::Batch {
                operations: added
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        if let Some(shape) = self.selected_layout_shape_ref() {
            self.active_layer = shape.layer;
        }
        self.status_message = format!(
            "{label} {} shape{}",
            added.len(),
            if added.len() == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn delete_layout_selection(&mut self) -> bool {
        if let Some(occurrence) = &self.selected_layout_occurrence
            && !occurrence.is_top_level()
        {
            if let Some((parent, id)) = self.workspace.document.instance_parent_for_path_from_cell(
                self.layout_view_top_cell,
                &occurrence.instance_path,
            ) {
                let Some(instance) = self.workspace.document.instance(parent, id) else {
                    return true;
                };
                self.apply_layout_operation_with_history(
                    Operation::DeleteInstance { parent, id },
                    Operation::AddInstance { parent, instance },
                );
                self.selected_layout_shape = None;
                self.selected_layout_occurrence = None;
                self.status_message = "Deleted instance".to_string();
                return true;
            }
        }
        let Some(shape_id) = self.selected_layout_shape else {
            self.status_message = "No layout selection to delete".to_string();
            return true;
        };
        if !self.workspace.document.shapes.contains_key(&shape_id) {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.status_message = "Layout selection was missing".to_string();
            return true;
        }
        let Some(shape) = self.workspace.document.shapes.get(&shape_id) else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
            self.status_message = "Layout selection was missing".to_string();
            return true;
        };
        self.apply_layout_operation_with_history(
            Operation::DeleteShape { id: shape_id },
            Operation::AddShape { shape },
        );
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        self.status_message = format!("Deleted shape #{}", shape_id.0);
        true
    }

    pub(crate) fn make_layout_cell_from_selection(&mut self) -> bool {
        let Some(mut shape) = self.selected_top_level_layout_shape() else {
            self.status_message = "Select a top-level shape before creating a cell".to_string();
            return true;
        };
        let original_shape = shape.clone();
        let origin = self.snap_layout_point(shape.kind.bounds().min);
        shape.kind.translate(Vector::new(-origin.x, -origin.y));

        let cell_id = self.workspace.document.allocate_cell_id();
        let instance_id = self.workspace.document.allocate_instance_id();
        let mut cell = Cell::new(cell_id, format!("cell {}", cell_id.0));
        cell.shapes.insert(shape.id, shape.clone());
        let instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", cell.name)),
            cell: cell_id,
            transform: Transform::translate(origin.x, origin.y),
            array: InstanceArray::single(),
            properties: BTreeMap::new(),
        };

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: vec![
                    Operation::DeleteShape { id: shape.id },
                    Operation::AddCell { cell: cell.clone() },
                    Operation::AddInstance {
                        parent: self.workspace.document.top_cell,
                        instance: instance.clone(),
                    },
                ],
            },
            Operation::Batch {
                operations: vec![
                    Operation::DeleteInstance {
                        parent: self.workspace.document.top_cell,
                        id: instance.id,
                    },
                    Operation::DeleteCell { id: cell.id },
                    Operation::AddShape {
                        shape: original_shape,
                    },
                ],
            },
        );
        self.selected_layout_shape = Some(shape.id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::from_instance_path(
            shape.id,
            &[instance_id],
        ));
        self.status_message = format!("Created {} from 1 shape", cell.name);
        true
    }

    pub(crate) fn export_layout_library_catalog(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_library_catalog_exchange_path();
            return self.export_layout_library_catalog_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Library catalog export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_library_catalog_to_path(&mut self, path: &Path) -> bool {
        let exchange = LayoutLibraryCatalogExchange::from_app(self);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Library catalog export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Library catalog export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported library catalog {} ({} preset(s))",
            path.display(),
            exchange.presets.len()
        );
        true
    }

    pub(crate) fn import_layout_library_catalog(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_library_catalog_exchange_path();
            return self.import_layout_library_catalog_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "Library catalog import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_library_catalog_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Library catalog import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutLibraryCatalogExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Library catalog import failed: {error}");
                return true;
            }
        };
        let document_name = exchange.document_name.clone();
        let presets = match exchange.into_presets() {
            Ok(presets) => presets,
            Err(error) => {
                self.status_message = format!("Library catalog import failed: {error}");
                return true;
            }
        };
        let imported_count = presets.len();
        self.layout_library_via_array_presets = presets;
        self.sync_app_options_from_state();
        self.status_message = format!(
            "Imported library catalog {} from {} ({} preset(s))",
            path.display(),
            document_name,
            imported_count
        );
        true
    }

    pub(crate) fn create_layout_library_macro(&mut self, slug: &str) -> bool {
        if let Some(value) = slug.strip_prefix("preset.save.") {
            return self.save_layout_via_array_library_preset(value);
        }
        if let Some(value) = slug.strip_prefix("preset.load.") {
            return self.load_layout_via_array_library_preset(value);
        }
        if let Some(value) = slug.strip_prefix("preset.create.") {
            return self.create_layout_via_array_library_cell_from_preset(value);
        }
        if let Some(value) = slug.strip_prefix("preset.clear.") {
            return self.clear_layout_via_array_library_preset(value);
        }

        match slug {
            "catalog.export" => self.export_layout_library_catalog(),
            "catalog.import" => self.import_layout_library_catalog(),
            "via_array_3x3" => self.create_layout_via_array_library_cell(
                LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_COLUMNS,
                LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_ROWS,
                LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_SIZE_GRIDS,
                LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_PITCH_GRIDS,
            ),
            "via_array.create" => self.create_layout_via_array_library_cell(
                self.layout_library_via_array_columns,
                self.layout_library_via_array_rows,
                self.layout_library_via_array_size_grids,
                self.layout_library_via_array_pitch_grids,
            ),
            "via_array.from_selection" => {
                self.create_layout_via_array_library_cell_from_selection()
            }
            "make_static" => self.make_selected_layout_library_cell_static(),
            "via_array.reset" => self.reset_layout_via_array_library_parameters(),
            "via_array.load_selected" => self.load_selected_layout_via_array_library_cell(),
            "via_array.update_selected" => self.update_selected_layout_via_array_library_cell(),
            "via_array.columns.dec" => self.adjust_layout_via_array_columns(-1),
            "via_array.columns.inc" => self.adjust_layout_via_array_columns(1),
            "via_array.rows.dec" => self.adjust_layout_via_array_rows(-1),
            "via_array.rows.inc" => self.adjust_layout_via_array_rows(1),
            "via_array.size.dec" => {
                self.adjust_layout_via_array_size_grids(-LAYOUT_LIBRARY_VIA_ARRAY_SIZE_STEP_GRIDS)
            }
            "via_array.size.inc" => {
                self.adjust_layout_via_array_size_grids(LAYOUT_LIBRARY_VIA_ARRAY_SIZE_STEP_GRIDS)
            }
            "via_array.pitch.dec" => {
                self.adjust_layout_via_array_pitch_grids(-LAYOUT_LIBRARY_VIA_ARRAY_PITCH_STEP_GRIDS)
            }
            "via_array.pitch.inc" => {
                self.adjust_layout_via_array_pitch_grids(LAYOUT_LIBRARY_VIA_ARRAY_PITCH_STEP_GRIDS)
            }
            _ => false,
        }
    }

    pub(crate) fn reset_layout_via_array_library_parameters(&mut self) -> bool {
        self.layout_library_via_array_columns = LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_COLUMNS;
        self.layout_library_via_array_rows = LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_ROWS;
        self.layout_library_via_array_size_grids = LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_SIZE_GRIDS;
        self.layout_library_via_array_pitch_grids = LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_PITCH_GRIDS;
        self.status_message = self.layout_via_array_parameter_status();
        true
    }

    pub(crate) fn save_layout_via_array_library_preset(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_library_preset_slot(value) else {
            return false;
        };
        let (size_grids, pitch_grids, via_size, pitch) = self.layout_via_array_dimensions(
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids,
        );
        let name = compact_button_label(
            &format!(
                "Via {}x{} s{} p{}",
                self.layout_library_via_array_columns,
                self.layout_library_via_array_rows,
                via_size,
                pitch
            ),
            32,
        );
        self.layout_library_via_array_presets.insert(
            slot,
            LayoutViaArrayLibraryPreset {
                name: name.clone(),
                columns: self.layout_library_via_array_columns,
                rows: self.layout_library_via_array_rows,
                size_grids,
                pitch_grids,
            },
        );
        self.status_message = format!("Saved library preset {slot}: {name}");
        true
    }

    pub(crate) fn load_layout_via_array_library_preset(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_library_preset_slot(value) else {
            return false;
        };
        let Some(preset) = self.layout_library_via_array_presets.get(&slot).cloned() else {
            self.status_message = format!("No library preset {slot} saved");
            return true;
        };
        self.apply_layout_via_array_library_preset(&preset);
        self.status_message = format!(
            "Loaded library preset {slot}: {}",
            self.layout_via_array_parameter_status()
        );
        true
    }

    pub(crate) fn create_layout_via_array_library_cell_from_preset(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_library_preset_slot(value) else {
            return false;
        };
        let Some(preset) = self.layout_library_via_array_presets.get(&slot).cloned() else {
            self.status_message = format!("No library preset {slot} saved");
            return true;
        };
        self.apply_layout_via_array_library_preset(&preset);
        self.create_layout_via_array_library_cell(
            preset.columns,
            preset.rows,
            preset.size_grids,
            preset.pitch_grids,
        )
    }

    pub(crate) fn clear_layout_via_array_library_preset(&mut self, value: &str) -> bool {
        let Some(slot) = parse_layout_library_preset_slot(value) else {
            return false;
        };
        let removed = self
            .layout_library_via_array_presets
            .remove(&slot)
            .is_some();
        self.status_message = if removed {
            format!("Cleared library preset {slot}")
        } else {
            format!("No library preset {slot} saved")
        };
        true
    }

    pub(crate) fn apply_layout_via_array_library_preset(
        &mut self,
        preset: &LayoutViaArrayLibraryPreset,
    ) {
        self.layout_library_via_array_columns = preset.columns.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.layout_library_via_array_rows = preset.rows.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.layout_library_via_array_size_grids = preset.size_grids.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
        );
        self.layout_library_via_array_pitch_grids = clamp_layout_library_via_array_pitch_grids(
            self.layout_library_via_array_size_grids,
            preset.pitch_grids,
        );
    }

    pub(crate) fn layout_via_array_library_preset_label(&self, slot: u8) -> String {
        self.layout_library_via_array_presets
            .get(&slot)
            .map(|preset| {
                format!(
                    "P{}: {} {}x{} s{} p{}",
                    slot,
                    compact_button_label(&preset.name, 12),
                    preset.columns,
                    preset.rows,
                    preset.size_grids,
                    preset.pitch_grids
                )
            })
            .unwrap_or_else(|| format!("P{slot}: empty"))
    }

    pub(crate) fn adjust_layout_via_array_columns(&mut self, delta: i8) -> bool {
        self.layout_library_via_array_columns = if delta.is_negative() {
            self.layout_library_via_array_columns
                .saturating_sub(delta.unsigned_abs())
        } else {
            self.layout_library_via_array_columns
                .saturating_add(delta as u8)
        }
        .clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.status_message = self.layout_via_array_parameter_status();
        true
    }

    pub(crate) fn adjust_layout_via_array_rows(&mut self, delta: i8) -> bool {
        self.layout_library_via_array_rows = if delta.is_negative() {
            self.layout_library_via_array_rows
                .saturating_sub(delta.unsigned_abs())
        } else {
            self.layout_library_via_array_rows
                .saturating_add(delta as u8)
        }
        .clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.status_message = self.layout_via_array_parameter_status();
        true
    }

    pub(crate) fn adjust_layout_via_array_size_grids(&mut self, delta: Coord) -> bool {
        self.layout_library_via_array_size_grids =
            (self.layout_library_via_array_size_grids + delta).clamp(
                LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
                LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
            );
        self.layout_library_via_array_pitch_grids = clamp_layout_library_via_array_pitch_grids(
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids,
        );
        self.status_message = self.layout_via_array_parameter_status();
        true
    }

    pub(crate) fn adjust_layout_via_array_pitch_grids(&mut self, delta: Coord) -> bool {
        self.layout_library_via_array_pitch_grids = clamp_layout_library_via_array_pitch_grids(
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids + delta,
        );
        self.status_message = self.layout_via_array_parameter_status();
        true
    }

    pub(crate) fn layout_via_array_parameter_status(&self) -> String {
        format!(
            "Via array {}x{} size {} pitch {}",
            self.layout_library_via_array_columns,
            self.layout_library_via_array_rows,
            self.format_layout_length(self.layout_via_array_size_dbu() as f64),
            self.format_layout_length(self.layout_via_array_pitch_dbu() as f64)
        )
    }

    pub(crate) fn layout_via_array_size_dbu(&self) -> Coord {
        self.workspace.document.grid.max(1) * self.layout_library_via_array_size_grids
    }

    pub(crate) fn layout_via_array_pitch_dbu(&self) -> Coord {
        self.workspace.document.grid.max(1) * self.layout_library_via_array_pitch_grids
    }

    pub(crate) fn load_selected_layout_via_array_library_cell(&mut self) -> bool {
        let Some(cell_id) = self.selected_layout_library_cell_id() else {
            self.status_message = "Select a generated via-array library cell first".to_string();
            return true;
        };
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            self.status_message = "Selected library cell is missing".to_string();
            return true;
        };
        let Some((columns, rows, size_grids, pitch_grids)) =
            layout_via_array_parameters_from_cell(cell)
        else {
            self.status_message = "Selected library cell is not a via-array macro".to_string();
            return true;
        };
        self.layout_library_via_array_columns = columns;
        self.layout_library_via_array_rows = rows;
        self.layout_library_via_array_size_grids = size_grids;
        self.layout_library_via_array_pitch_grids =
            clamp_layout_library_via_array_pitch_grids(size_grids, pitch_grids);
        self.status_message = format!("Loaded {}", self.layout_via_array_parameter_status());
        true
    }

    pub(crate) fn update_selected_layout_via_array_library_cell(&mut self) -> bool {
        let Some(cell_id) = self.selected_layout_library_cell_id() else {
            self.status_message = "Select a generated via-array library cell first".to_string();
            return true;
        };
        let Some(old_cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Selected library cell is missing".to_string();
            return true;
        };
        if old_cell.properties.get("library.macro").map(String::as_str) != Some("via_array") {
            self.status_message = "Selected library cell is not a via-array macro".to_string();
            return true;
        }
        let Some((via_layer, lower, upper)) = self.layout_via_array_layers() else {
            return true;
        };
        let (size_grids, pitch_grids, via_size, pitch) = self.layout_via_array_dimensions(
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids,
        );
        let new_properties = Self::layout_via_array_library_properties(
            self.layout_library_via_array_columns,
            self.layout_library_via_array_rows,
            size_grids,
            pitch_grids,
            via_size,
            pitch,
        );
        let new_shapes = self.layout_via_array_shapes(
            self.layout_library_via_array_columns,
            self.layout_library_via_array_rows,
            via_layer,
            lower,
            upper,
            via_size,
            pitch,
        );
        if new_shapes.is_empty() {
            self.status_message = "Via-array macro generated no shapes".to_string();
            return true;
        }
        let old_shapes = old_cell.shapes.values().collect::<Vec<_>>();
        let selection_path = self
            .layout_selection_instance_path_for_cell(cell_id)
            .or_else(|| self.first_layout_instance_path_for_cell(cell_id))
            .unwrap_or_default();

        let mut redo = Vec::new();
        for shape in &old_shapes {
            redo.push(Operation::DeleteShapeFromCell {
                cell: cell_id,
                id: shape.id,
            });
        }
        for shape in &new_shapes {
            redo.push(Operation::AddShapeToCell {
                cell: cell_id,
                shape: shape.clone(),
            });
        }
        redo.push(Operation::SetCellProperties {
            id: cell_id,
            properties: new_properties.clone(),
        });

        let mut undo = Vec::new();
        for shape in &new_shapes {
            undo.push(Operation::DeleteShapeFromCell {
                cell: cell_id,
                id: shape.id,
            });
        }
        for shape in &old_shapes {
            undo.push(Operation::AddShapeToCell {
                cell: cell_id,
                shape: shape.clone(),
            });
        }
        undo.push(Operation::SetCellProperties {
            id: cell_id,
            properties: old_cell.properties,
        });

        let first_shape = new_shapes.first().map(|shape| shape.id);
        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.active_layer = via_layer;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence = first_shape.map(|id| {
            if selection_path.is_empty() {
                ShapeOccurrenceId::top_level(id)
            } else {
                ShapeOccurrenceId::from_instance_path(id, &selection_path)
            }
        });
        self.status_message = format!(
            "Updated {} to reusable {}-via library cell",
            old_cell.name,
            usize::from(self.layout_library_via_array_columns)
                * usize::from(self.layout_library_via_array_rows)
        );
        true
    }

    pub(crate) fn make_selected_layout_library_cell_static(&mut self) -> bool {
        let Some(cell_id) = self.selected_layout_library_cell_id() else {
            self.status_message = "Select a generated library cell first".to_string();
            return true;
        };
        let Some(old_cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Selected library cell is missing".to_string();
            return true;
        };
        let mut properties = old_cell.properties.clone();
        let library_keys = properties
            .keys()
            .filter(|key| key.starts_with("library."))
            .cloned()
            .collect::<Vec<_>>();
        if library_keys.is_empty() {
            self.status_message = "Selected cell is already static".to_string();
            return true;
        }
        for key in library_keys {
            properties.remove(&key);
        }
        self.apply_layout_operation_with_history(
            Operation::SetCellProperties {
                id: cell_id,
                properties,
            },
            Operation::SetCellProperties {
                id: cell_id,
                properties: old_cell.properties,
            },
        );
        self.status_message = format!("Converted {} to a normal static cell", old_cell.name);
        true
    }

    pub(crate) fn selected_layout_library_cell_id(&self) -> Option<CellId> {
        if let Some(occurrence) = self.selected_layout_occurrence.as_ref()
            && let Some(view) = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)
            && self
                .workspace
                .document
                .cell(view.source_cell)
                .is_some_and(layout_cell_is_library_generated)
        {
            return Some(view.source_cell);
        }
        self.workspace
            .document
            .cell(self.layout_view_top_cell)
            .filter(|cell| layout_cell_is_library_generated(cell))
            .map(|cell| cell.id)
    }

    pub(crate) fn layout_selection_instance_path_for_cell(
        &self,
        cell_id: CellId,
    ) -> Option<Vec<InstanceId>> {
        let occurrence = self.selected_layout_occurrence.as_ref()?;
        let view = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)?;
        (view.source_cell == cell_id).then(|| occurrence.instance_path.clone())
    }

    pub(crate) fn first_layout_instance_path_for_cell(
        &self,
        cell_id: CellId,
    ) -> Option<Vec<InstanceId>> {
        let root = self.workspace.document.cell(self.layout_view_top_cell)?;
        root.instances
            .values()
            .find(|instance| instance.cell == cell_id)
            .map(|instance| vec![instance.id])
    }

    pub(crate) fn layout_via_array_layers(&mut self) -> Option<(LayerId, LayerId, LayerId)> {
        let Some(via_layer) = self
            .workspace
            .document
            .layer_by_process(ProcessLayer::Via1)
            .or_else(|| {
                self.workspace
                    .document
                    .layer_by_process(ProcessLayer::Contact)
            })
        else {
            self.status_message =
                "Library via array needs a via1 or contact technology layer".to_string();
            return None;
        };
        let Some(lower) = self
            .workspace
            .document
            .layer_by_process(ProcessLayer::Metal1)
        else {
            self.status_message = "Library via array needs a metal1 layer".to_string();
            return None;
        };
        let Some(upper) = self
            .workspace
            .document
            .layer_by_process(ProcessLayer::Metal2)
        else {
            self.status_message = "Library via array needs a metal2 layer".to_string();
            return None;
        };
        Some((via_layer, lower, upper))
    }

    pub(crate) fn layout_via_array_dimensions(
        &self,
        size_grids: Coord,
        pitch_grids: Coord,
    ) -> (Coord, Coord, Coord, Coord) {
        let grid = self.workspace.document.grid.max(1);
        let size_grids = size_grids.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
        );
        let pitch_grids = clamp_layout_library_via_array_pitch_grids(size_grids, pitch_grids);
        let via_size = (grid * size_grids).max(grid);
        let pitch = (grid * pitch_grids).max(via_size + grid);
        (size_grids, pitch_grids, via_size, pitch)
    }

    pub(crate) fn layout_via_array_library_properties(
        columns: u8,
        rows: u8,
        size_grids: Coord,
        pitch_grids: Coord,
        via_size: Coord,
        pitch: Coord,
    ) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("library.kind".to_string(), "macro".to_string()),
            ("library.macro".to_string(), "via_array".to_string()),
            ("library.columns".to_string(), columns.to_string()),
            ("library.rows".to_string(), rows.to_string()),
            ("library.size_grids".to_string(), size_grids.to_string()),
            ("library.pitch_grids".to_string(), pitch_grids.to_string()),
            ("library.via_size_dbu".to_string(), via_size.to_string()),
            ("library.pitch_dbu".to_string(), pitch.to_string()),
        ])
    }

    pub(crate) fn layout_via_array_shapes(
        &mut self,
        columns: u8,
        rows: u8,
        via_layer: LayerId,
        lower: LayerId,
        upper: LayerId,
        via_size: Coord,
        pitch: Coord,
    ) -> Vec<Shape> {
        let column_offset = (Coord::from(columns) - 1) * pitch / 2;
        let row_offset = (Coord::from(rows) - 1) * pitch / 2;
        let mut shapes = Vec::with_capacity(usize::from(columns) * usize::from(rows));
        for row in 0..rows {
            for column in 0..columns {
                let id = self.workspace.document.allocate_shape_id();
                let center = Point::new(
                    Coord::from(column) * pitch - column_offset,
                    Coord::from(row) * pitch - row_offset,
                );
                shapes.push(Shape {
                    id,
                    layer: via_layer,
                    net: None,
                    kind: ShapeKind::Via {
                        center,
                        size: via_size,
                        lower,
                        upper,
                    },
                    name: Some(format!(
                        "via array {}x{} r{} c{}",
                        columns,
                        rows,
                        row + 1,
                        column + 1
                    )),
                    properties: BTreeMap::new(),
                });
            }
        }
        shapes
    }
}
