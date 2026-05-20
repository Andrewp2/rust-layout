#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn create_layout_via_array_library_cell(
        &mut self,
        columns: u8,
        rows: u8,
        size_grids: Coord,
        pitch_grids: Coord,
    ) -> bool {
        let origin = self
            .layout_canvas_size
            .map(|size| {
                self.snap_layout_point(self.layout_canvas_to_world_local(
                    UiPoint::new(size.width * 0.5, size.height * 0.5),
                    size,
                ))
            })
            .unwrap_or_else(|| self.snap_layout_point(Point::new(0, 0)));
        self.create_layout_via_array_library_cell_at(
            columns,
            rows,
            size_grids,
            pitch_grids,
            origin,
            self.layout_view_top_cell,
            None,
        )
    }

    pub(crate) fn create_layout_via_array_library_cell_from_selection(&mut self) -> bool {
        let Some((guide_cell, _, guide_shape)) = self.selected_current_cell_layout_shape() else {
            self.status_message =
                "Select a current-cell guide shape before creating a via-array macro".to_string();
            return true;
        };
        let bounds = guide_shape.kind.bounds();
        if bounds.width() <= 0 || bounds.height() <= 0 {
            self.status_message =
                "Via-array macro conversion needs a guide shape with nonzero bounds".to_string();
            return true;
        }
        let (_, pitch_grids, _, pitch) = self.layout_via_array_dimensions(
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids,
        );
        let columns = ((bounds.width().max(pitch) / pitch).max(1) as u8).clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        let rows = ((bounds.height().max(pitch) / pitch).max(1) as u8).clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.layout_library_via_array_columns = columns;
        self.layout_library_via_array_rows = rows;
        self.layout_library_via_array_pitch_grids = pitch_grids;
        self.create_layout_via_array_library_cell_at(
            columns,
            rows,
            self.layout_library_via_array_size_grids,
            self.layout_library_via_array_pitch_grids,
            self.snap_layout_point(bounds.center()),
            guide_cell,
            Some(guide_shape),
        )
    }

    pub(crate) fn create_layout_via_array_library_cell_at(
        &mut self,
        columns: u8,
        rows: u8,
        size_grids: Coord,
        pitch_grids: Coord,
        origin: Point,
        parent_cell: CellId,
        guide_shape: Option<Shape>,
    ) -> bool {
        if columns == 0 || rows == 0 {
            self.status_message = "Library macro has invalid dimensions".to_string();
            return true;
        }
        let Some((via_layer, lower, upper)) = self.layout_via_array_layers() else {
            return true;
        };

        let (size_grids, pitch_grids, via_size, pitch) =
            self.layout_via_array_dimensions(size_grids, pitch_grids);
        let cell_id = self.workspace.document.allocate_cell_id();
        let instance_id = self.workspace.document.allocate_instance_id();
        let mut cell = Cell::new(
            cell_id,
            format!(
                "lib via array {}x{} s{} p{} {}",
                columns, rows, via_size, pitch, cell_id.0
            ),
        );
        cell.properties = Self::layout_via_array_library_properties(
            columns,
            rows,
            size_grids,
            pitch_grids,
            via_size,
            pitch,
        );
        for shape in
            self.layout_via_array_shapes(columns, rows, via_layer, lower, upper, via_size, pitch)
        {
            cell.shapes.insert(shape.id, shape);
        }
        let first_shape = cell.shapes.keys().copied().next();

        let instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", cell.name)),
            cell: cell_id,
            transform: Transform::translate(origin.x, origin.y),
            array: InstanceArray::single(),
            properties: BTreeMap::new(),
        };
        let converted_guide = guide_shape.is_some();
        let mut redo_operations = Vec::new();
        if let Some(shape) = guide_shape.as_ref() {
            redo_operations.push(self.shape_delete_operation(parent_cell, shape.id));
        }
        redo_operations.push(Operation::AddCell { cell: cell.clone() });
        redo_operations.push(Operation::AddInstance {
            parent: parent_cell,
            instance: instance.clone(),
        });
        let mut undo_operations = vec![
            Operation::DeleteInstance {
                parent: parent_cell,
                id: instance.id,
            },
            Operation::DeleteCell { id: cell.id },
        ];
        if let Some(shape) = guide_shape {
            undo_operations.push(self.shape_add_operation(parent_cell, shape));
        }
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.layout_view_top_cell = parent_cell;
        self.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
        self.layout_hierarchy_min_depth = 0;
        self.layout_hidden_cells.remove(&parent_cell);
        self.active_layer = via_layer;
        self.selected_layout_shape = first_shape;
        self.selected_layout_occurrence =
            first_shape.map(|id| ShapeOccurrenceId::from_instance_path(id, &[instance_id]));
        self.status_message = format!(
            "{} {} as a reusable {}-via library cell",
            if converted_guide {
                "Converted"
            } else {
                "Created"
            },
            cell.name,
            usize::from(columns) * usize::from(rows)
        );
        true
    }

    pub(crate) fn make_selected_layout_instance_variant(&mut self) -> bool {
        let Some((parent, id, old_instance)) = self.selected_layout_instance_context() else {
            self.status_message = "Select an instance before making a variant".to_string();
            return true;
        };
        if old_instance.cell == self.workspace.document.top_cell {
            self.status_message = "Document top cell cannot be variant-copied".to_string();
            return true;
        }
        let Some(source_cell) = self.workspace.document.cell(old_instance.cell).cloned() else {
            self.status_message = "Selected instance target cell is missing".to_string();
            return true;
        };

        let variant_id = self.workspace.document.allocate_cell_id();
        let mut variant = Cell::new(
            variant_id,
            format!("{} variant {}", source_cell.name, variant_id.0),
        );
        let mut source_shapes = source_cell.shapes.values().collect::<Vec<_>>();
        source_shapes.sort_by_key(|shape| shape.id);
        for mut shape in source_shapes {
            shape.id = self.workspace.document.allocate_shape_id();
            variant.shapes.insert(shape.id, shape);
        }
        let mut source_instances = source_cell.instances.values().collect::<Vec<_>>();
        source_instances.sort_by_key(|instance| instance.id);
        for mut instance in source_instances {
            instance.id = self.workspace.document.allocate_instance_id();
            variant.instances.insert(instance.id, instance);
        }

        let mut new_instance = old_instance.clone();
        new_instance.cell = variant_id;

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: vec![
                    Operation::AddCell {
                        cell: variant.clone(),
                    },
                    Operation::ReplaceInstance {
                        parent,
                        id,
                        instance: new_instance,
                    },
                ],
            },
            Operation::Batch {
                operations: vec![
                    Operation::ReplaceInstance {
                        parent,
                        id,
                        instance: old_instance.clone(),
                    },
                    Operation::DeleteCell { id: variant_id },
                ],
            },
        );

        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        if let Some(occurrence) =
            first_layout_occurrence_for_instance(&self.workspace.document, parent, id)
        {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
        self.status_message = format!("Made cell variant {} for instance #{}", variant.name, id.0);
        true
    }

    pub(crate) fn make_current_layout_cell_child_variants(&mut self) -> bool {
        let parent = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(parent).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        self.make_layout_cell_child_variants_batch(
            vec![parent],
            "child cell",
            format!("Cell {} has no child instances", cell.name),
            format!("Cell {} has no variant-copyable child instances", cell.name),
            Some(format!("in {}", cell.name)),
        )
    }

    pub(crate) fn make_descendant_layout_cell_child_variants(&mut self) -> bool {
        let root = self.layout_view_top_cell;
        let Some(root_cell) = self.workspace.document.cell(root) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let root_name = root_cell.name.clone();
        let cell_ids = layout_descendant_cell_ids(&self.workspace.document, root)
            .into_iter()
            .filter(|cell| *cell != root)
            .collect::<Vec<_>>();
        self.make_layout_cell_child_variants_batch(
            cell_ids,
            "descendant child",
            format!("{root_name} has no descendant cells with child instances"),
            format!("{root_name} descendant cells have no variant-copyable child instances"),
            None,
        )
    }

    pub(crate) fn make_document_layout_cell_child_variants(&mut self) -> bool {
        let cell_ids = self
            .workspace
            .document
            .cells
            .keys()
            .copied()
            .collect::<Vec<_>>();
        self.make_layout_cell_child_variants_batch(
            cell_ids,
            "document child",
            "Document has no cells with child instances".to_string(),
            "Document cells have no variant-copyable child instances".to_string(),
            None,
        )
    }

    fn make_layout_cell_child_variants_batch(
        &mut self,
        cell_ids: Vec<CellId>,
        scope_label: &str,
        empty_instances_message: String,
        empty_variants_message: String,
        status_suffix: Option<String>,
    ) -> bool {
        let mut variants = Vec::new();
        let mut replacements = Vec::new();
        let mut child_instance_cells = 0usize;
        let mut seen = BTreeSet::new();
        for parent in cell_ids {
            if !seen.insert(parent) {
                continue;
            }
            let Some(cell) = self.workspace.document.cell(parent).cloned() else {
                continue;
            };
            let mut instances = cell.instances.values().collect::<Vec<_>>();
            instances.sort_by_key(|instance| instance.id);
            if instances.is_empty() {
                continue;
            }
            child_instance_cells += 1;
            for old_instance in instances {
                if old_instance.cell == self.workspace.document.top_cell {
                    continue;
                }
                let Some(source_cell) = self.workspace.document.cell(old_instance.cell).cloned()
                else {
                    continue;
                };
                let variant = self.layout_cell_variant_copy(&source_cell);
                let mut new_instance = old_instance.clone();
                new_instance.cell = variant.id;
                variants.push(variant);
                replacements.push((parent, old_instance, new_instance));
            }
        }

        if replacements.is_empty() {
            self.status_message = if child_instance_cells == 0 {
                empty_instances_message
            } else {
                empty_variants_message
            };
            return true;
        };
        let Some((first_parent, first_instance_id)) = replacements
            .first()
            .map(|(parent, instance, _)| (*parent, instance.id))
        else {
            self.status_message = empty_variants_message;
            return true;
        };
        let variant_count = variants.len();

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: variants
                    .iter()
                    .cloned()
                    .map(|cell| Operation::AddCell { cell })
                    .chain(
                        replacements
                            .iter()
                            .map(|(parent, old_instance, new_instance)| {
                                Operation::ReplaceInstance {
                                    parent: *parent,
                                    id: old_instance.id,
                                    instance: new_instance.clone(),
                                }
                            }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: replacements
                    .iter()
                    .rev()
                    .map(|(parent, old_instance, _)| Operation::ReplaceInstance {
                        parent: *parent,
                        id: old_instance.id,
                        instance: old_instance.clone(),
                    })
                    .chain(
                        variants
                            .iter()
                            .rev()
                            .map(|cell| Operation::DeleteCell { id: cell.id }),
                    )
                    .collect(),
            },
        );

        if self.layout_view_top_cell != first_parent {
            self.layout_view_top_cell = first_parent;
            self.layout_hidden_cells.remove(&first_parent);
            self.invalidate_layout_view_caches();
        }
        if let Some(occurrence) = first_layout_occurrence_for_instance(
            &self.workspace.document,
            first_parent,
            first_instance_id,
        ) {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
        self.status_message = format!(
            "Made {} {} variant{}{}",
            variant_count,
            scope_label,
            if variant_count == 1 { "" } else { "s" },
            status_suffix
                .map(|suffix| format!(" {suffix}"))
                .unwrap_or_default()
        );
        true
    }

    fn layout_cell_variant_copy(&mut self, source_cell: &Cell) -> Cell {
        let variant_id = self.workspace.document.allocate_cell_id();
        let mut variant = Cell::new(
            variant_id,
            format!("{} variant {}", source_cell.name, variant_id.0),
        );
        let mut source_shapes = source_cell.shapes.values().collect::<Vec<_>>();
        source_shapes.sort_by_key(|shape| shape.id);
        for mut shape in source_shapes {
            shape.id = self.workspace.document.allocate_shape_id();
            variant.shapes.insert(shape.id, shape);
        }
        let mut source_instances = source_cell.instances.values().collect::<Vec<_>>();
        source_instances.sort_by_key(|instance| instance.id);
        for mut instance in source_instances {
            instance.id = self.workspace.document.allocate_instance_id();
            variant.instances.insert(instance.id, instance);
        }
        variant
    }

    pub(crate) fn flatten_selected_layout_instance(&mut self, depth: LayoutFlattenDepth) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.clone() else {
            self.status_message = "Select an instance to flatten".to_string();
            return true;
        };
        if occurrence.instance_path.is_empty() {
            self.status_message = "Select an instance to flatten".to_string();
            return true;
        }
        let Some((parent, id)) = self.workspace.document.instance_parent_for_path_from_cell(
            self.layout_view_top_cell,
            &occurrence.instance_path,
        ) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };
        let Some(instance) = self.workspace.document.instance(parent, id) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };

        let instances = [instance.clone()];
        let replacements =
            self.replacements_for_flattened_cell_instances(&instances, depth, parent);
        if replacements.shapes.is_empty() && replacements.instances.is_empty() {
            self.status_message = "Selected instance has no geometry to flatten".to_string();
            return true;
        }

        let selected_shape = replacements.shapes.first().map(|shape| shape.id);
        let selected_instance = replacements.instances.first().map(|instance| instance.id);
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: std::iter::once(Operation::DeleteInstance { parent, id })
                    .chain(replacements.shapes.iter().cloned().map(|shape| {
                        if parent == self.workspace.document.top_cell {
                            Operation::AddShape { shape }
                        } else {
                            Operation::AddShapeToCell {
                                cell: parent,
                                shape,
                            }
                        }
                    }))
                    .chain(
                        replacements
                            .instances
                            .iter()
                            .cloned()
                            .map(|instance| Operation::AddInstance { parent, instance }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: replacements
                    .instances
                    .iter()
                    .rev()
                    .map(|instance| Operation::DeleteInstance {
                        parent,
                        id: instance.id,
                    })
                    .chain(replacements.shapes.iter().rev().map(|shape| {
                        if parent == self.workspace.document.top_cell {
                            Operation::DeleteShape { id: shape.id }
                        } else {
                            Operation::DeleteShapeFromCell {
                                cell: parent,
                                id: shape.id,
                            }
                        }
                    }))
                    .chain(std::iter::once(Operation::AddInstance {
                        parent,
                        instance: instance.clone(),
                    }))
                    .collect(),
            },
        );
        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        self.select_layout_flatten_replacement(parent, selected_shape, selected_instance);
        self.status_message = format!(
            "Flattened instance #{} {} into {} shape{} and {} instance{}",
            id.0,
            depth.label(),
            replacements.shapes.len(),
            if replacements.shapes.len() == 1 {
                ""
            } else {
                "s"
            },
            replacements.instances.len(),
            if replacements.instances.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        true
    }

    pub(crate) fn flatten_current_layout_cell(&mut self, depth: LayoutFlattenDepth) -> bool {
        let cell_id = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let mut instances = cell.instances.values().collect::<Vec<_>>();
        instances.sort_by_key(|instance| instance.id);
        if instances.is_empty() {
            self.status_message = format!("Cell {} has no child instances", cell.name);
            return true;
        }

        let replacements =
            self.replacements_for_flattened_cell_instances(&instances, depth, cell_id);
        let selected_shape = replacements.shapes.first().map(|shape| shape.id);
        let selected_instance = replacements.instances.first().map(|instance| instance.id);
        let shape_count = replacements.shapes.len();
        let promoted_instance_count = replacements.instances.len();
        let instance_count = instances.len();
        if shape_count == 0 && promoted_instance_count == 0 {
            self.status_message = format!("Cell {} has no child geometry to flatten", cell.name);
            return true;
        }

        let redo = Operation::Batch {
            operations: instances
                .iter()
                .map(|instance| Operation::DeleteInstance {
                    parent: cell_id,
                    id: instance.id,
                })
                .chain(replacements.shapes.iter().cloned().map(|shape| {
                    if cell_id == self.workspace.document.top_cell {
                        Operation::AddShape { shape }
                    } else {
                        Operation::AddShapeToCell {
                            cell: cell_id,
                            shape,
                        }
                    }
                }))
                .chain(replacements.instances.iter().cloned().map(|instance| {
                    Operation::AddInstance {
                        parent: cell_id,
                        instance,
                    }
                }))
                .collect(),
        };

        let undo = Operation::Batch {
            operations: replacements
                .instances
                .iter()
                .rev()
                .map(|instance| Operation::DeleteInstance {
                    parent: cell_id,
                    id: instance.id,
                })
                .chain(replacements.shapes.iter().rev().map(|shape| {
                    if cell_id == self.workspace.document.top_cell {
                        Operation::DeleteShape { id: shape.id }
                    } else {
                        Operation::DeleteShapeFromCell {
                            cell: cell_id,
                            id: shape.id,
                        }
                    }
                }))
                .chain(
                    instances
                        .iter()
                        .cloned()
                        .map(|instance| Operation::AddInstance {
                            parent: cell_id,
                            instance,
                        }),
                )
                .collect(),
        };

        self.apply_layout_operation_with_history(redo, undo);
        self.select_layout_flatten_replacement(cell_id, selected_shape, selected_instance);
        self.status_message = format!(
            "Flattened cell {} {}: removed {} instance{}, added {} shape{} and {} instance{}",
            cell.name,
            depth.label(),
            instance_count,
            if instance_count == 1 { "" } else { "s" },
            shape_count,
            if shape_count == 1 { "" } else { "s" },
            promoted_instance_count,
            if promoted_instance_count == 1 {
                ""
            } else {
                "s"
            }
        );
        true
    }

    pub(crate) fn flatten_descendant_layout_cells(&mut self, depth: LayoutFlattenDepth) -> bool {
        let root = self.layout_view_top_cell;
        let Some(root_cell) = self.workspace.document.cell(root) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let root_name = root_cell.name.clone();
        let cell_ids = layout_descendant_cell_ids(&self.workspace.document, root)
            .into_iter()
            .filter(|cell| *cell != root)
            .collect::<Vec<_>>();

        self.flatten_layout_cell_batch(
            cell_ids,
            depth,
            "descendant",
            format!("{root_name} has no descendant cells with child instances"),
            format!("{root_name} descendant cells have no child geometry to flatten"),
        )
    }

    pub(crate) fn flatten_document_layout_cells(&mut self, depth: LayoutFlattenDepth) -> bool {
        let cell_ids = self
            .workspace
            .document
            .cells
            .keys()
            .copied()
            .collect::<Vec<_>>();
        self.flatten_layout_cell_batch(
            cell_ids,
            depth,
            "document",
            "Document has no cells with child instances".to_string(),
            "Document cells have no child geometry to flatten".to_string(),
        )
    }

    fn flatten_layout_cell_batch(
        &mut self,
        cell_ids: Vec<CellId>,
        depth: LayoutFlattenDepth,
        scope_label: &str,
        empty_instances_message: String,
        empty_geometry_message: String,
    ) -> bool {
        let mut flatten_batches = Vec::new();
        let mut child_instance_cells = 0usize;
        for cell_id in cell_ids {
            let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
                continue;
            };
            let mut instances = cell.instances.values().collect::<Vec<_>>();
            instances.sort_by_key(|instance| instance.id);
            if instances.is_empty() {
                continue;
            }
            child_instance_cells += 1;
            let replacements =
                self.replacements_for_flattened_cell_instances(&instances, depth, cell_id);
            if replacements.shapes.is_empty() && replacements.instances.is_empty() {
                continue;
            }
            flatten_batches.push((cell_id, instances, replacements));
        }

        if flatten_batches.is_empty() {
            self.status_message = if child_instance_cells == 0 {
                empty_instances_message
            } else {
                empty_geometry_message
            };
            return true;
        }

        let first_cell = flatten_batches[0].0;
        let selected_shape = flatten_batches[0].2.shapes.first().map(|shape| shape.id);
        let selected_instance = flatten_batches[0]
            .2
            .instances
            .first()
            .map(|instance| instance.id);
        let cell_count = flatten_batches.len();
        let instance_count = flatten_batches
            .iter()
            .map(|(_, instances, _)| instances.len())
            .sum::<usize>();
        let shape_count = flatten_batches
            .iter()
            .map(|(_, _, replacements)| replacements.shapes.len())
            .sum::<usize>();
        let promoted_instance_count = flatten_batches
            .iter()
            .map(|(_, _, replacements)| replacements.instances.len())
            .sum::<usize>();

        let mut redo_operations = Vec::new();
        for (cell_id, instances, replacements) in &flatten_batches {
            for instance in instances {
                redo_operations.push(Operation::DeleteInstance {
                    parent: *cell_id,
                    id: instance.id,
                });
            }
            for shape in &replacements.shapes {
                if *cell_id == self.workspace.document.top_cell {
                    redo_operations.push(Operation::AddShape {
                        shape: shape.clone(),
                    });
                } else {
                    redo_operations.push(Operation::AddShapeToCell {
                        cell: *cell_id,
                        shape: shape.clone(),
                    });
                }
            }
            for instance in &replacements.instances {
                redo_operations.push(Operation::AddInstance {
                    parent: *cell_id,
                    instance: instance.clone(),
                });
            }
        }

        let mut undo_operations = Vec::new();
        for (cell_id, instances, replacements) in flatten_batches.iter().rev() {
            for instance in replacements.instances.iter().rev() {
                undo_operations.push(Operation::DeleteInstance {
                    parent: *cell_id,
                    id: instance.id,
                });
            }
            for shape in replacements.shapes.iter().rev() {
                if *cell_id == self.workspace.document.top_cell {
                    undo_operations.push(Operation::DeleteShape { id: shape.id });
                } else {
                    undo_operations.push(Operation::DeleteShapeFromCell {
                        cell: *cell_id,
                        id: shape.id,
                    });
                }
            }
            for instance in instances {
                undo_operations.push(Operation::AddInstance {
                    parent: *cell_id,
                    instance: instance.clone(),
                });
            }
        }

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: redo_operations,
            },
            Operation::Batch {
                operations: undo_operations,
            },
        );
        self.select_layout_flatten_replacement(first_cell, selected_shape, selected_instance);
        self.status_message = format!(
            "Flattened {} {} cell{} {}: removed {} instance{}, added {} shape{} and {} instance{}",
            cell_count,
            scope_label,
            if cell_count == 1 { "" } else { "s" },
            depth.label(),
            instance_count,
            if instance_count == 1 { "" } else { "s" },
            shape_count,
            if shape_count == 1 { "" } else { "s" },
            promoted_instance_count,
            if promoted_instance_count == 1 {
                ""
            } else {
                "s"
            }
        );
        true
    }

    pub(crate) fn replacements_for_flattened_cell_instances(
        &mut self,
        instances: &[CellInstance],
        depth: LayoutFlattenDepth,
        destination_cell: CellId,
    ) -> LayoutFlattenReplacements {
        let mut replacements = LayoutFlattenReplacements::default();
        for instance in instances {
            let mut child_shapes = Vec::new();
            let mut child_instances = Vec::new();
            match depth {
                LayoutFlattenDepth::Deep => {
                    self.workspace
                        .document
                        .for_each_flattened_shape_view_for_cell(instance.cell, None, |_, view| {
                            child_shapes.push(view.transformed_shape())
                        });
                }
                LayoutFlattenDepth::OneLevel => {
                    if let Some(cell) = self.workspace.document.cell(instance.cell) {
                        child_shapes.extend(cell.shapes.values());
                        child_instances.extend(cell.instances.values());
                    }
                }
            }
            let array = instance.array.normalized();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let transform = Transform::from_translation(array.element_offset(column, row))
                        .compose(instance.transform);
                    for template in &child_shapes {
                        let mut shape = template.clone();
                        shape.id = self.workspace.document.allocate_shape_id();
                        shape.kind = transform.apply_shape_kind(&shape.kind);
                        replacements.shapes.push(shape);
                    }
                    for child_instance in &child_instances {
                        if child_instance.cell == destination_cell {
                            continue;
                        }
                        let mut promoted = child_instance.clone();
                        promoted.id = self.workspace.document.allocate_instance_id();
                        promoted.transform = transform.compose(child_instance.transform);
                        if !array.is_single()
                            && let Some(name) = &child_instance.name
                        {
                            promoted.name = Some(format!("{name} {column}x{row}"));
                        }
                        replacements.instances.push(promoted);
                    }
                }
            }
        }
        replacements
    }

    pub(crate) fn select_layout_flatten_replacement(
        &mut self,
        parent: CellId,
        selected_shape: Option<ShapeId>,
        selected_instance: Option<InstanceId>,
    ) {
        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        if let Some(shape_id) = selected_shape {
            self.selected_layout_shape = Some(shape_id);
            self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
            return;
        }
        if let Some(instance_id) = selected_instance
            && let Some(occurrence) =
                first_layout_occurrence_for_instance(&self.workspace.document, parent, instance_id)
        {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
            return;
        }
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
    }

    pub(crate) fn adjust_current_layout_cell_origin_to_selection(&mut self) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.as_ref() else {
            self.status_message =
                "Select layout geometry before setting the cell origin".to_string();
            return true;
        };
        let Some(view) = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)
        else {
            self.status_message = "Selected layout geometry is no longer visible".to_string();
            return true;
        };
        self.adjust_current_layout_cell_origin(view.bounds.min, "selection")
    }

    pub(crate) fn adjust_current_layout_cell_origin_by(
        &mut self,
        x_steps: Coord,
        y_steps: Coord,
    ) -> bool {
        let step = self.layout_size_step();
        let origin = Point::new(step.saturating_mul(x_steps), step.saturating_mul(y_steps));
        let label = format!(
            "offset {},{}",
            self.format_layout_length(origin.x as f64),
            self.format_layout_length(origin.y as f64)
        );
        self.adjust_current_layout_cell_origin(origin, &label)
    }

    pub(crate) fn adjust_current_layout_cell_origin_exact_from_search(&mut self) -> bool {
        let input = self.layout_browser_search.trim();
        let Some(origin) = parse_layout_origin_point(input) else {
            self.status_message =
                "Enter exact origin as two DBU coordinates in browser search".to_string();
            return true;
        };
        self.adjust_current_layout_cell_origin_raw(
            origin,
            &format!("exact {},{}", origin.x, origin.y),
        )
    }

    pub(crate) fn adjust_current_layout_cell_origin(&mut self, origin: Point, label: &str) -> bool {
        let origin = self.snap_layout_point(origin);
        self.adjust_current_layout_cell_origin_raw(origin, label)
    }

    pub(crate) fn adjust_current_layout_cell_origin_raw(
        &mut self,
        origin: Point,
        label: &str,
    ) -> bool {
        if origin == Point::ZERO {
            self.status_message = format!("Current cell origin is already at {label}");
            return true;
        }
        let cell_id = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let parent_refs = layout_cell_instance_refs(&self.workspace.document, cell_id);
        if parent_refs.iter().any(|(parent, _)| *parent == cell_id) {
            self.status_message = format!(
                "Cannot adjust origin for directly self-instanced cell {}",
                cell.name
            );
            return true;
        }

        let local_delta = Vector::new(-origin.x, -origin.y);
        let parent_compensation = Transform::translate(origin.x, origin.y);
        let shifted_cell = layout_cell_with_local_origin_delta(cell.clone(), local_delta);
        let parent_replacements = parent_refs
            .into_iter()
            .filter_map(|(parent, id)| {
                let old_instance = self.workspace.document.instance(parent, id)?;
                let mut new_instance = old_instance.clone();
                new_instance.transform = old_instance.transform.compose(parent_compensation);
                Some((parent, id, old_instance, new_instance))
            })
            .collect::<Vec<_>>();

        let mut redo = Vec::new();
        let mut undo = Vec::new();
        if cell_id == self.workspace.document.top_cell {
            for shape in self.workspace.document.shapes.values() {
                let mut shifted = shape.clone();
                shifted.kind.translate(local_delta);
                redo.push(Operation::ReplaceShape {
                    id: shape.id,
                    shape: shifted,
                });
                undo.push(Operation::ReplaceShape {
                    id: shape.id,
                    shape,
                });
            }
        }
        redo.push(Operation::AddCell { cell: shifted_cell });
        undo.push(Operation::AddCell { cell });
        for (parent, id, old_instance, new_instance) in parent_replacements {
            redo.push(Operation::ReplaceInstance {
                parent,
                id,
                instance: new_instance,
            });
            undo.insert(
                0,
                Operation::ReplaceInstance {
                    parent,
                    id,
                    instance: old_instance,
                },
            );
        }

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Set {} origin to {},{} from {label}",
            self.workspace
                .document
                .cell(cell_id)
                .map(|cell| cell.name.as_str())
                .unwrap_or("cell"),
            origin.x,
            origin.y
        );
        true
    }

    pub(crate) fn adjust_descendant_leaf_layout_cell_origins_to_bounds(&mut self) -> bool {
        let root = self.layout_view_top_cell;
        let Some(root_cell) = self.workspace.document.cell(root) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let root_name = root_cell.name.clone();
        let mut candidates = Vec::new();
        for cell_id in layout_descendant_cell_ids(&self.workspace.document, root) {
            if cell_id == root {
                continue;
            }
            let Some(cell) = self.workspace.document.cell(cell_id) else {
                continue;
            };
            if !cell.instances.is_empty() {
                continue;
            }
            let Some(origin) = self
                .layout_cell_local_shape_bounds(cell_id)
                .map(|bounds| self.snap_layout_point(bounds.min))
            else {
                continue;
            };
            if origin != Point::ZERO {
                candidates.push((cell_id, origin));
            }
        }

        if candidates.is_empty() {
            self.status_message =
                format!("{root_name} has no descendant leaf-cell origins to adjust");
            return true;
        }

        let mut redo = Vec::new();
        let mut undo = Vec::new();
        let mut adjusted = 0usize;
        for (cell_id, origin) in candidates {
            let Some(cell) = self.workspace.document.cell(cell_id).cloned() else {
                continue;
            };
            let parent_refs = layout_cell_instance_refs(&self.workspace.document, cell_id);
            if parent_refs.iter().any(|(parent, _)| *parent == cell_id) {
                continue;
            }

            let local_delta = Vector::new(-origin.x, -origin.y);
            let parent_compensation = Transform::translate(origin.x, origin.y);
            redo.push(Operation::AddCell {
                cell: layout_cell_with_local_origin_delta(cell.clone(), local_delta),
            });
            undo.push(Operation::AddCell { cell });
            for (parent, id) in parent_refs {
                let Some(old_instance) = self.workspace.document.instance(parent, id) else {
                    continue;
                };
                let mut new_instance = old_instance.clone();
                new_instance.transform = old_instance.transform.compose(parent_compensation);
                redo.push(Operation::ReplaceInstance {
                    parent,
                    id,
                    instance: new_instance,
                });
                undo.insert(
                    0,
                    Operation::ReplaceInstance {
                        parent,
                        id,
                        instance: old_instance,
                    },
                );
            }
            adjusted += 1;
        }

        if adjusted == 0 {
            self.status_message =
                format!("{root_name} has no descendant leaf-cell origins to adjust");
            return true;
        }

        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.repair_layout_selection_after_operation();
        self.status_message = format!(
            "Set {} descendant leaf-cell origin{} from local bounds",
            adjusted,
            if adjusted == 1 { "" } else { "s" }
        );
        true
    }

    fn layout_cell_local_shape_bounds(&self, cell_id: CellId) -> Option<Rect> {
        let cell = self.workspace.document.cell(cell_id)?;
        let mut bounds = None;
        for shape in cell.shapes.values() {
            let shape_bounds = shape.kind.bounds();
            bounds = Some(bounds.map_or(shape_bounds, |current: Rect| current.union(shape_bounds)));
        }
        if cell_id == self.workspace.document.top_cell {
            for shape in self.workspace.document.shapes.values() {
                let shape_bounds = shape.kind.bounds();
                bounds =
                    Some(bounds.map_or(shape_bounds, |current: Rect| current.union(shape_bounds)));
            }
        }
        bounds
    }

    pub(crate) fn move_selected_layout_shape_up_hierarchy(&mut self) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.clone() else {
            self.status_message = "Select a shape before moving it up".to_string();
            return true;
        };
        let shape_id = occurrence.source_shape_id();
        let Some(source_cell) = self
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, &occurrence)
            .map(|view| view.source_cell)
        else {
            self.status_message = "Selected shape is no longer visible".to_string();
            return true;
        };
        if source_cell == self.workspace.document.top_cell {
            self.status_message = "Document-top shapes cannot move up".to_string();
            return true;
        }
        let Some(cell) = self.workspace.document.cell(source_cell) else {
            self.status_message = "Selected shape source cell is missing".to_string();
            return true;
        };
        let Some(shape) = cell.shapes.get(&shape_id) else {
            self.status_message = "Selected shape is not local to its source cell".to_string();
            return true;
        };
        let parent_refs = layout_cell_instance_refs(&self.workspace.document, source_cell);
        if parent_refs.is_empty() {
            self.status_message = "Selected shape source cell has no parent instances".to_string();
            return true;
        }
        if parent_refs.iter().any(|(parent, _)| *parent == source_cell) {
            self.status_message =
                "Move shape up does not support directly self-instanced cells".to_string();
            return true;
        }

        let mut moved_shapes = Vec::new();
        for (parent, instance_id) in parent_refs {
            let Some(instance) = self.workspace.document.instance(parent, instance_id) else {
                continue;
            };
            let array = instance.array.normalized();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let transform = Transform::from_translation(array.element_offset(column, row))
                        .compose(instance.transform);
                    let mut moved = shape.clone();
                    moved.id = self.workspace.document.allocate_shape_id();
                    moved.kind = transform.apply_shape_kind(&moved.kind);
                    moved_shapes.push((parent, moved));
                }
            }
        }
        if moved_shapes.is_empty() {
            self.status_message = "No parent placement available for selected shape".to_string();
            return true;
        }

        let selected_parent = moved_shapes.first().map(|(parent, _)| *parent);
        let selected_shape = moved_shapes.first().map(|(_, shape)| shape.id);
        let redo = Operation::Batch {
            operations: std::iter::once(Operation::DeleteShapeFromCell {
                cell: source_cell,
                id: shape_id,
            })
            .chain(moved_shapes.iter().cloned().map(|(parent, shape)| {
                if parent == self.workspace.document.top_cell {
                    Operation::AddShape { shape }
                } else {
                    Operation::AddShapeToCell {
                        cell: parent,
                        shape,
                    }
                }
            }))
            .collect(),
        };
        let undo = Operation::Batch {
            operations: moved_shapes
                .iter()
                .rev()
                .map(|(parent, shape)| {
                    if *parent == self.workspace.document.top_cell {
                        Operation::DeleteShape { id: shape.id }
                    } else {
                        Operation::DeleteShapeFromCell {
                            cell: *parent,
                            id: shape.id,
                        }
                    }
                })
                .chain(std::iter::once(Operation::AddShapeToCell {
                    cell: source_cell,
                    shape: shape.clone(),
                }))
                .collect(),
        };

        let moved_count = moved_shapes.len();
        self.apply_layout_operation_with_history(redo, undo);
        if let Some(parent) = selected_parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        self.selected_layout_shape = selected_shape;
        self.selected_layout_occurrence = selected_shape.map(ShapeOccurrenceId::top_level);
        if let Some(shape) = self.selected_layout_shape_ref() {
            self.active_layer = shape.layer;
        }
        self.status_message = format!(
            "Moved shape #{} up into {} parent placement{}",
            shape_id.0,
            moved_count,
            if moved_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn move_selected_layout_instance_up_hierarchy(&mut self) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.clone() else {
            self.status_message = "Select an instance before moving it up".to_string();
            return true;
        };
        if occurrence.instance_path.is_empty() {
            self.status_message = "Select an instance before moving it up".to_string();
            return true;
        }
        let Some((cell_id, id)) = self.workspace.document.instance_parent_for_path_from_cell(
            self.layout_view_top_cell,
            &occurrence.instance_path,
        ) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };
        if cell_id == self.workspace.document.top_cell {
            self.status_message = "Document-top instances cannot move up".to_string();
            return true;
        }
        let Some(instance) = self.workspace.document.instance(cell_id, id) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        if !cell.instances.contains_key(&id) {
            self.status_message = "Selected instance is not local to the current cell".to_string();
            return true;
        }
        let parent_refs = layout_cell_instance_refs(&self.workspace.document, cell_id);
        if parent_refs.is_empty() {
            self.status_message = "Current cell has no parent instances".to_string();
            return true;
        }
        if parent_refs.iter().any(|(parent, _)| *parent == cell_id) {
            self.status_message =
                "Move instance up does not support directly self-instanced cells".to_string();
            return true;
        }

        let source_array = instance.array.normalized();
        let mut moved_instances = Vec::new();
        for (parent, parent_instance_id) in parent_refs {
            if parent == instance.cell {
                continue;
            }
            let Some(parent_instance) =
                self.workspace.document.instance(parent, parent_instance_id)
            else {
                continue;
            };
            let parent_array = parent_instance.array.normalized();
            for parent_row in 0..parent_array.rows {
                for parent_column in 0..parent_array.columns {
                    let parent_transform = Transform::from_translation(
                        parent_array.element_offset(parent_column, parent_row),
                    )
                    .compose(parent_instance.transform);
                    for source_row in 0..source_array.rows {
                        for source_column in 0..source_array.columns {
                            let source_transform = Transform::from_translation(
                                source_array.element_offset(source_column, source_row),
                            )
                            .compose(instance.transform);
                            let mut moved = instance.clone();
                            moved.id = self.workspace.document.allocate_instance_id();
                            moved.array = InstanceArray::single();
                            moved.transform = parent_transform.compose(source_transform);
                            moved_instances.push((parent, moved));
                        }
                    }
                }
            }
        }
        if moved_instances.is_empty() {
            self.status_message = "No parent placement available for selected instance".to_string();
            return true;
        }

        let selected_parent = moved_instances.first().map(|(parent, _)| *parent);
        let selected_instance = moved_instances.first().map(|(_, instance)| instance.id);
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: std::iter::once(Operation::DeleteInstance {
                    parent: cell_id,
                    id,
                })
                .chain(
                    moved_instances
                        .iter()
                        .cloned()
                        .map(|(parent, instance)| Operation::AddInstance { parent, instance }),
                )
                .collect(),
            },
            Operation::Batch {
                operations: moved_instances
                    .iter()
                    .rev()
                    .map(|(parent, instance)| Operation::DeleteInstance {
                        parent: *parent,
                        id: instance.id,
                    })
                    .chain(std::iter::once(Operation::AddInstance {
                        parent: cell_id,
                        instance: instance.clone(),
                    }))
                    .collect(),
            },
        );

        if let Some(parent) = selected_parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        if let Some((parent, instance_id)) = selected_parent.zip(selected_instance)
            && let Some(occurrence) =
                first_layout_occurrence_for_instance(&self.workspace.document, parent, instance_id)
        {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }

        self.status_message = format!(
            "Moved instance #{} up into {} parent placement{}",
            id.0,
            moved_instances.len(),
            if moved_instances.len() == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn resolve_selected_layout_instance_array(&mut self) -> bool {
        let Some(occurrence) = self.selected_layout_occurrence.clone() else {
            self.status_message = "Select an instance array to resolve".to_string();
            return true;
        };
        if occurrence.instance_path.is_empty() {
            self.status_message = "Select an instance array to resolve".to_string();
            return true;
        }
        let Some((parent, id)) = self.workspace.document.instance_parent_for_path_from_cell(
            self.layout_view_top_cell,
            &occurrence.instance_path,
        ) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };
        let Some(instance) = self.workspace.document.instance(parent, id) else {
            self.status_message = "Selected instance no longer exists".to_string();
            return true;
        };
        let array = instance.array.normalized();
        if array.is_single() {
            self.status_message = "Selected instance is not an array".to_string();
            return true;
        }

        let mut replacements = Vec::with_capacity((array.columns * array.rows) as usize);
        for row in 0..array.rows {
            for column in 0..array.columns {
                let mut replacement = instance.clone();
                replacement.id = self.workspace.document.allocate_instance_id();
                replacement.array = InstanceArray::single();
                replacement.transform =
                    Transform::from_translation(array.element_offset(column, row))
                        .compose(instance.transform);
                if let Some(name) = &instance.name {
                    replacement.name = Some(format!("{name} {column}x{row}"));
                }
                replacements.push(replacement);
            }
        }
        let Some(first_replacement_id) = replacements.first().map(|instance| instance.id) else {
            self.status_message = "Selected instance array had no elements".to_string();
            return true;
        };

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: std::iter::once(Operation::DeleteInstance { parent, id })
                    .chain(
                        replacements
                            .iter()
                            .cloned()
                            .map(|instance| Operation::AddInstance { parent, instance }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: replacements
                    .iter()
                    .rev()
                    .map(|instance| Operation::DeleteInstance {
                        parent,
                        id: instance.id,
                    })
                    .chain(std::iter::once(Operation::AddInstance {
                        parent,
                        instance: instance.clone(),
                    }))
                    .collect(),
            },
        );

        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        if let Some(occurrence) = first_layout_occurrence_for_instance(
            &self.workspace.document,
            parent,
            first_replacement_id,
        ) {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
        self.status_message = format!(
            "Resolved instance array #{} into {} instances",
            id.0,
            replacements.len()
        );
        true
    }

    pub(crate) fn resolve_current_layout_cell_arrays(&mut self) -> bool {
        let parent = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(parent) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let cell_name = cell.name.clone();
        let arrays = self.layout_cell_array_instances([parent]);
        if arrays.is_empty() {
            self.status_message = format!("{cell_name} has no instance arrays to resolve");
            return true;
        }
        let resolved_arrays = arrays.len();

        let Some((focus_parent, first_replacement_id, replacement_count)) =
            self.resolve_layout_cell_array_batch(arrays)
        else {
            self.status_message = "Current cell arrays had no elements".to_string();
            return true;
        };

        self.select_resolved_layout_instance(focus_parent, first_replacement_id);
        self.status_message = format!(
            "Resolved {} current-cell array{} into {} instances",
            resolved_arrays,
            if resolved_arrays == 1 { "" } else { "s" },
            replacement_count
        );
        true
    }

    pub(crate) fn resolve_descendant_layout_cell_arrays(&mut self) -> bool {
        let root = self.layout_view_top_cell;
        let Some(cell) = self.workspace.document.cell(root) else {
            self.status_message = "Current view cell is missing".to_string();
            return true;
        };
        let cell_name = cell.name.clone();
        let descendant_cells = layout_descendant_cell_ids(&self.workspace.document, root)
            .into_iter()
            .filter(|cell| *cell != root)
            .collect::<Vec<_>>();
        let arrays = self.layout_cell_array_instances(descendant_cells);
        if arrays.is_empty() {
            self.status_message =
                format!("{cell_name} has no descendant instance arrays to resolve");
            return true;
        }
        let resolved_arrays = arrays.len();

        let Some((focus_parent, first_replacement_id, replacement_count)) =
            self.resolve_layout_cell_array_batch(arrays)
        else {
            self.status_message = "Descendant cell arrays had no elements".to_string();
            return true;
        };

        self.select_resolved_layout_instance(focus_parent, first_replacement_id);
        self.status_message = format!(
            "Resolved {} descendant array{} into {} instances",
            resolved_arrays,
            if resolved_arrays == 1 { "" } else { "s" },
            replacement_count
        );
        true
    }

    pub(crate) fn resolve_document_layout_cell_arrays(&mut self) -> bool {
        let cells = self
            .workspace
            .document
            .cells
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let arrays = self.layout_cell_array_instances(cells);
        if arrays.is_empty() {
            self.status_message = "Document has no instance arrays to resolve".to_string();
            return true;
        }
        let resolved_arrays = arrays.len();

        let Some((focus_parent, first_replacement_id, replacement_count)) =
            self.resolve_layout_cell_array_batch(arrays)
        else {
            self.status_message = "Document arrays had no elements".to_string();
            return true;
        };

        self.select_resolved_layout_instance(focus_parent, first_replacement_id);
        self.status_message = format!(
            "Resolved {} document array{} into {} instances",
            resolved_arrays,
            if resolved_arrays == 1 { "" } else { "s" },
            replacement_count
        );
        true
    }

    fn layout_cell_array_instances(
        &self,
        parents: impl IntoIterator<Item = CellId>,
    ) -> Vec<(CellId, CellInstance)> {
        let mut arrays = Vec::new();
        let mut seen = BTreeSet::new();
        for parent in parents {
            if !seen.insert(parent) {
                continue;
            }
            let Some(cell) = self.workspace.document.cell(parent) else {
                continue;
            };
            arrays.extend(
                cell.instances
                    .values()
                    .filter(|instance| !instance.array.normalized().is_single())
                    .map(|instance| (parent, instance)),
            );
        }
        arrays
    }

    fn resolve_layout_cell_array_batch(
        &mut self,
        arrays: Vec<(CellId, CellInstance)>,
    ) -> Option<(CellId, InstanceId, usize)> {
        let mut replacements = Vec::new();
        for (parent, instance) in &arrays {
            let array = instance.array.normalized();
            for row in 0..array.rows {
                for column in 0..array.columns {
                    let mut replacement = instance.clone();
                    replacement.id = self.workspace.document.allocate_instance_id();
                    replacement.array = InstanceArray::single();
                    replacement.transform =
                        Transform::from_translation(array.element_offset(column, row))
                            .compose(instance.transform);
                    if let Some(name) = &instance.name {
                        replacement.name = Some(format!("{name} {column}x{row}"));
                    }
                    replacements.push((*parent, replacement));
                }
            }
        }
        let (first_parent, first_replacement_id) = replacements
            .first()
            .map(|(parent, instance)| (*parent, instance.id))?;
        let replacement_count = replacements.len();

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: arrays
                    .iter()
                    .map(|(parent, instance)| Operation::DeleteInstance {
                        parent: *parent,
                        id: instance.id,
                    })
                    .chain(
                        replacements
                            .iter()
                            .cloned()
                            .map(|(parent, instance)| Operation::AddInstance { parent, instance }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: replacements
                    .iter()
                    .rev()
                    .map(|(parent, instance)| Operation::DeleteInstance {
                        parent: *parent,
                        id: instance.id,
                    })
                    .chain(
                        arrays
                            .iter()
                            .cloned()
                            .map(|(parent, instance)| Operation::AddInstance { parent, instance }),
                    )
                    .collect(),
            },
        );

        Some((first_parent, first_replacement_id, replacement_count))
    }

    fn select_resolved_layout_instance(&mut self, parent: CellId, instance_id: InstanceId) {
        if self.layout_view_top_cell != parent {
            self.layout_view_top_cell = parent;
            self.layout_hidden_cells.remove(&parent);
            self.invalidate_layout_view_caches();
        }
        if let Some(occurrence) =
            first_layout_occurrence_for_instance(&self.workspace.document, parent, instance_id)
        {
            self.ensure_layout_occurrence_depth_visible(&occurrence);
            self.selected_layout_shape = Some(occurrence.source_shape_id());
            self.selected_layout_occurrence = Some(occurrence);
            if let Some(shape) = self.selected_layout_shape_ref() {
                self.active_layer = shape.layer;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
    }

    pub(crate) fn rotate_layout_selection_90(&mut self) -> bool {
        if self.transform_selected_layout_instance(Transform::rotate_cw90(), "Rotated instance") {
            return true;
        }
        self.transform_selected_layout_shapes(ShapeTransform::RotateCw90, "Rotated")
    }

    pub(crate) fn mirror_layout_selection_x(&mut self) -> bool {
        if self.transform_selected_layout_instance(Transform::mirror_x(), "Mirrored instance X") {
            return true;
        }
        self.transform_selected_layout_shapes(ShapeTransform::MirrorX, "Mirrored X")
    }

    pub(crate) fn mirror_layout_selection_y(&mut self) -> bool {
        if self.transform_selected_layout_instance(Transform::mirror_y(), "Mirrored instance Y") {
            return true;
        }
        self.transform_selected_layout_shapes(ShapeTransform::MirrorY, "Mirrored Y")
    }

    pub(crate) fn layout_size_step(&self) -> Coord {
        self.workspace
            .document
            .grid
            .max(1)
            .saturating_mul(LAYOUT_SIZE_STEP_GRIDS)
    }

    pub(crate) fn selected_current_cell_layout_shape(
        &self,
    ) -> Option<(CellId, ShapeOccurrenceId, Shape)> {
        if let Some(occurrence) = &self.selected_layout_occurrence
            && occurrence.is_top_level()
            && self
                .selected_layout_shape
                .is_none_or(|id| id == occurrence.source_shape_id())
        {
            let view = self
                .workspace
                .document
                .shape_view_for_occurrence_from_cell(self.layout_view_top_cell, occurrence)?;
            if view.source_cell == self.layout_view_top_cell {
                return Some((view.source_cell, occurrence.clone(), view.shape.to_shape()));
            }
        }

        let shape = self.selected_top_level_layout_shape()?;
        Some((
            self.workspace.document.top_cell,
            ShapeOccurrenceId::top_level(shape.id),
            shape,
        ))
    }

    pub(crate) fn layout_cell_shapes(&self, cell_id: CellId) -> Vec<Shape> {
        if cell_id == self.workspace.document.top_cell {
            return self.workspace.document.shapes.values().collect();
        }
        self.workspace
            .document
            .cell(cell_id)
            .map(|cell| cell.shapes.values().collect())
            .unwrap_or_default()
    }

    pub(crate) fn layout_cell_shape(&self, cell_id: CellId, shape_id: ShapeId) -> Option<Shape> {
        if cell_id == self.workspace.document.top_cell {
            return self.workspace.document.shapes.get(&shape_id);
        }
        self.workspace
            .document
            .cell(cell_id)
            .and_then(|cell| cell.shapes.get(&shape_id))
    }

    fn push_shape_replacement_operations(
        &self,
        cell_id: CellId,
        old_shape: &Shape,
        new_shape: &Shape,
        redo: &mut Vec<Operation>,
        undo: &mut Vec<Operation>,
    ) {
        if cell_id == self.workspace.document.top_cell
            && self.workspace.document.shapes.contains_key(&old_shape.id)
        {
            redo.push(Operation::ReplaceShape {
                id: old_shape.id,
                shape: new_shape.clone(),
            });
            undo.push(Operation::ReplaceShape {
                id: old_shape.id,
                shape: old_shape.clone(),
            });
        } else {
            redo.push(Operation::DeleteShapeFromCell {
                cell: cell_id,
                id: old_shape.id,
            });
            redo.push(Operation::AddShapeToCell {
                cell: cell_id,
                shape: new_shape.clone(),
            });
            undo.push(Operation::DeleteShapeFromCell {
                cell: cell_id,
                id: new_shape.id,
            });
            undo.push(Operation::AddShapeToCell {
                cell: cell_id,
                shape: old_shape.clone(),
            });
        }
    }

    pub(crate) fn shape_add_operation(&self, cell_id: CellId, shape: Shape) -> Operation {
        if cell_id == self.workspace.document.top_cell {
            Operation::AddShape { shape }
        } else {
            Operation::AddShapeToCell {
                cell: cell_id,
                shape,
            }
        }
    }

    pub(crate) fn shape_delete_operation(&self, cell_id: CellId, id: ShapeId) -> Operation {
        if cell_id == self.workspace.document.top_cell {
            Operation::DeleteShape { id }
        } else {
            Operation::DeleteShapeFromCell { cell: cell_id, id }
        }
    }

    pub(crate) fn size_layout_selection(&mut self, amount: Coord) -> bool {
        self.size_layout_selection_with_mode(amount, LayoutSizeMode::Uniform)
    }

    pub(crate) fn size_layout_selection_with_mode(
        &mut self,
        amount: Coord,
        mode: LayoutSizeMode,
    ) -> bool {
        let Some((source_cell, occurrence, old_shape)) = self.selected_current_cell_layout_shape()
        else {
            self.status_message = "Select a current-cell shape to size".to_string();
            return true;
        };
        let Some(new_kind) = sized_shape_kind_with_mode(&old_shape.kind, amount, mode) else {
            self.status_message = format!(
                "Sizing{} supports selected {} for now",
                mode.status_axis_label(),
                mode.supported_shape_label()
            );
            return true;
        };
        let mut new_shape = old_shape.clone();
        new_shape.kind = new_kind;
        if old_shape == new_shape {
            self.status_message = format!("Sizing left shape #{} unchanged", old_shape.id.0);
            return true;
        }
        let status = format!(
            "{} shape #{}{} by {}",
            if amount >= 0 { "Grew" } else { "Shrank" },
            old_shape.id.0,
            mode.status_axis_label(),
            self.format_layout_length(amount.abs() as f64)
        );
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            status,
        );
        true
    }

    pub(crate) fn chamfer_selected_layout_shape(&mut self) -> bool {
        let Some((source_cell, occurrence, old_shape)) = self.selected_current_cell_layout_shape()
        else {
            self.status_message =
                "Select a current-cell rectangle or polygon to chamfer".to_string();
            return true;
        };
        if !matches!(
            old_shape.kind,
            ShapeKind::Rectangle(_) | ShapeKind::Polygon(_)
        ) {
            self.status_message =
                "Corner chamfer currently supports selected rectangles and polygons".to_string();
            return true;
        }
        let Some((polygon, chamfer)) =
            chamfered_shape_polygon(&old_shape.kind, self.layout_size_step())
        else {
            self.status_message = format!(
                "Shape #{} is too small or invalid for corner chamfering",
                old_shape.id.0
            );
            return true;
        };

        let mut new_shape = old_shape.clone();
        new_shape.kind = ShapeKind::Polygon(polygon);
        let status = format!(
            "Chamfered shape #{} corners by {}",
            old_shape.id.0,
            self.format_layout_length(chamfer as f64)
        );
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            status,
        );
        true
    }

    pub(crate) fn round_selected_layout_shape(&mut self) -> bool {
        let Some((source_cell, occurrence, old_shape)) = self.selected_current_cell_layout_shape()
        else {
            self.status_message = "Select a current-cell rectangle or polygon to round".to_string();
            return true;
        };
        if !matches!(
            old_shape.kind,
            ShapeKind::Rectangle(_) | ShapeKind::Polygon(_)
        ) {
            self.status_message =
                "Corner rounding currently supports selected rectangles and polygons".to_string();
            return true;
        }
        let Some((polygon, radius)) =
            rounded_shape_polygon(&old_shape.kind, self.layout_size_step())
        else {
            self.status_message = format!(
                "Shape #{} is too small or invalid for corner rounding",
                old_shape.id.0
            );
            return true;
        };

        let mut new_shape = old_shape.clone();
        new_shape.kind = ShapeKind::Polygon(polygon);
        let status = format!(
            "Rounded shape #{} corners by {}",
            old_shape.id.0,
            self.format_layout_length(radius as f64)
        );
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            status,
        );
        true
    }

    pub(crate) fn chamfer_active_layer_layout_shapes(&mut self) -> bool {
        self.corner_active_layer_layout_shapes(false)
    }

    pub(crate) fn round_active_layer_layout_shapes(&mut self) -> bool {
        self.corner_active_layer_layout_shapes(true)
    }

    fn corner_active_layer_layout_shapes(&mut self, rounded: bool) -> bool {
        let cell_id = self.layout_view_top_cell;
        let amount = self.layout_size_step();
        let mut replacements = self
            .layout_cell_shapes(cell_id)
            .into_iter()
            .filter(|shape| shape.layer == self.active_layer)
            .filter_map(|old_shape| {
                let (polygon, _) = if rounded {
                    rounded_shape_polygon(&old_shape.kind, amount)?
                } else {
                    chamfered_shape_polygon(&old_shape.kind, amount)?
                };
                let mut new_shape = old_shape.clone();
                new_shape.kind = ShapeKind::Polygon(polygon);
                (new_shape != old_shape).then_some((old_shape, new_shape))
            })
            .collect::<Vec<_>>();
        replacements.sort_by_key(|(old_shape, _)| old_shape.id);
        if replacements.is_empty() {
            self.status_message = format!(
                "No active-layer current-cell rectangles or polygons could be {}",
                if rounded { "rounded" } else { "chamfered" }
            );
            return true;
        }

        let selected = replacements.first().map(|(shape, _)| shape.id);
        let mut redo = Vec::new();
        let mut undo = Vec::new();
        for (old_shape, new_shape) in &replacements {
            self.push_shape_replacement_operations(
                cell_id, old_shape, new_shape, &mut redo, &mut undo,
            );
        }
        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "{} {} active-layer current-cell shape{} corners by up to {}",
            if rounded { "Rounded" } else { "Chamfered" },
            replacements.len(),
            if replacements.len() == 1 { "" } else { "s" },
            self.format_layout_length(amount as f64)
        );
        true
    }

    pub(crate) fn size_active_layer_shapes(&mut self, amount: Coord) -> bool {
        self.size_active_layer_shapes_with_mode(amount, LayoutSizeMode::Uniform)
    }

    pub(crate) fn size_active_layer_shapes_with_mode(
        &mut self,
        amount: Coord,
        mode: LayoutSizeMode,
    ) -> bool {
        let cell_id = self.layout_view_top_cell;
        let mut replacements = self
            .layout_cell_shapes(cell_id)
            .into_iter()
            .filter(|shape| shape.layer == self.active_layer)
            .filter_map(|old_shape| {
                let new_kind = sized_shape_kind_with_mode(&old_shape.kind, amount, mode)?;
                let mut new_shape = old_shape.clone();
                new_shape.kind = new_kind;
                (new_shape != old_shape).then_some((old_shape, new_shape))
            })
            .collect::<Vec<_>>();
        replacements.sort_by_key(|(old_shape, _)| old_shape.id);
        if replacements.is_empty() {
            self.status_message = format!(
                "No active-layer current-cell {} could be sized{}",
                mode.supported_shape_label(),
                mode.status_axis_label()
            );
            return true;
        }

        let selected = replacements.first().map(|(shape, _)| shape.id);
        let mut redo = Vec::new();
        let mut undo = Vec::new();
        for (old_shape, new_shape) in &replacements {
            self.push_shape_replacement_operations(
                cell_id, old_shape, new_shape, &mut redo, &mut undo,
            );
        }
        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "{} {} active-layer current-cell shape{}{} by {}",
            if amount >= 0 { "Grew" } else { "Shrank" },
            replacements.len(),
            if replacements.len() == 1 { "" } else { "s" },
            mode.status_axis_label(),
            self.format_layout_length(amount.abs() as f64)
        );
        true
    }

    pub(crate) fn align_selected_layout_shape_to_active_layer_edge(
        &mut self,
        edge: LayoutAlignEdge,
    ) -> bool {
        if let Some((source_cell, occurrence, old_shape)) =
            self.selected_current_cell_layout_shape()
        {
            return self.align_current_cell_layout_shape_to_active_layer_edge(
                source_cell,
                occurrence,
                old_shape,
                edge,
            );
        }
        let Some((parent, id, old_instance)) = self.selected_layout_instance_context() else {
            self.status_message =
                format!("Select a current-cell shape or instance before aligning {edge}");
            return true;
        };

        self.align_layout_instance_to_active_layer_edge(parent, id, old_instance, edge)
    }

    pub(crate) fn align_active_layer_shapes_to_selected_shape(
        &mut self,
        edge: LayoutAlignEdge,
    ) -> bool {
        let Some((source_cell, _, reference_shape)) = self.selected_current_cell_layout_shape()
        else {
            self.status_message =
                format!("Select a current-cell reference shape before aligning layer {edge}");
            return true;
        };
        if source_cell != self.layout_view_top_cell {
            self.status_message =
                "Layer alignment reference must be local to the current view cell".to_string();
            return true;
        }
        if matches!(edge, LayoutAlignEdge::OriginX | LayoutAlignEdge::OriginY) {
            self.status_message =
                "Layer alignment to a reference shape supports edge and center targets".to_string();
            return true;
        }

        let reference_bounds = reference_shape.kind.bounds();
        let mut replacements = self
            .layout_cell_shapes(source_cell)
            .into_iter()
            .filter(|shape| shape.layer == self.active_layer && shape.id != reference_shape.id)
            .filter_map(|old_shape| {
                let shape_bounds = old_shape.kind.bounds();
                let delta = layout_align_delta_to_bounds(shape_bounds, reference_bounds, edge);
                if delta == Vector::ZERO {
                    return None;
                }
                let mut new_shape = old_shape.clone();
                new_shape.kind.translate(delta);
                Some((old_shape, new_shape))
            })
            .collect::<Vec<_>>();
        replacements.sort_by_key(|(old_shape, _)| old_shape.id);
        if replacements.is_empty() {
            self.status_message =
                format!("No active-layer current-cell shapes needed layer alignment {edge}");
            return true;
        }

        let selected = replacements.first().map(|(shape, _)| shape.id);
        let mut redo = Vec::new();
        let mut undo = Vec::new();
        for (old_shape, new_shape) in &replacements {
            self.push_shape_replacement_operations(
                source_cell,
                old_shape,
                new_shape,
                &mut redo,
                &mut undo,
            );
        }
        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Aligned {} active-layer current-cell shape{} {edge} to shape #{}",
            replacements.len(),
            if replacements.len() == 1 { "" } else { "s" },
            reference_shape.id.0
        );
        true
    }

    fn align_current_cell_layout_shape_to_active_layer_edge(
        &mut self,
        source_cell: CellId,
        occurrence: ShapeOccurrenceId,
        old_shape: Shape,
        edge: LayoutAlignEdge,
    ) -> bool {
        let selection_bounds = old_shape.kind.bounds();
        let delta = match edge {
            LayoutAlignEdge::OriginX => Vector::new(-rect_center_x(selection_bounds), 0),
            LayoutAlignEdge::OriginY => Vector::new(0, -rect_center_y(selection_bounds)),
            _ => {
                let Some(layer_bounds) =
                    self.active_layer_alignment_bounds(source_cell, Some(old_shape.id))
                else {
                    self.status_message =
                        "Active layer has no other current-cell shapes to align against"
                            .to_string();
                    return true;
                };
                layout_align_delta_to_bounds(selection_bounds, layer_bounds, edge)
            }
        };
        if delta == Vector::ZERO {
            self.status_message = format!("Shape #{} is already aligned {edge}", old_shape.id.0);
            return true;
        }

        let mut new_shape = old_shape.clone();
        new_shape.kind.translate(delta);
        let status = format!("Aligned shape #{} {edge}", old_shape.id.0);
        self.apply_layout_shape_source_replacement(
            source_cell,
            occurrence,
            old_shape,
            new_shape,
            status,
        );
        true
    }

    fn align_layout_instance_to_active_layer_edge(
        &mut self,
        parent: CellId,
        id: InstanceId,
        old_instance: CellInstance,
        edge: LayoutAlignEdge,
    ) -> bool {
        let Some(selection_bounds) = self.layout_instance_bounds(&old_instance) else {
            self.status_message = format!("Selected instance #{} has no geometry to align", id.0);
            return true;
        };
        let delta = match edge {
            LayoutAlignEdge::OriginX => Vector::new(-rect_center_x(selection_bounds), 0),
            LayoutAlignEdge::OriginY => Vector::new(0, -rect_center_y(selection_bounds)),
            _ => {
                let Some(layer_bounds) = self.active_layer_alignment_bounds(parent, None) else {
                    self.status_message =
                        "Active layer has no parent-cell shapes to align against".to_string();
                    return true;
                };
                layout_align_delta_to_bounds(selection_bounds, layer_bounds, edge)
            }
        };
        if delta == Vector::ZERO {
            self.status_message = format!("Instance #{} is already aligned {edge}", id.0);
            return true;
        }

        let mut new_instance = old_instance.clone();
        new_instance.transform.translation = Vector::new(
            new_instance.transform.translation.dx + delta.dx,
            new_instance.transform.translation.dy + delta.dy,
        );
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
        self.status_message = format!("Aligned instance #{} {edge}", id.0);
        true
    }

    fn layout_instance_bounds(&self, instance: &CellInstance) -> Option<Rect> {
        let mut bounds = None;
        let array = instance.array.normalized();
        for row in 0..array.rows {
            for column in 0..array.columns {
                let transform = Transform::from_translation(array.element_offset(column, row))
                    .compose(instance.transform);
                self.workspace
                    .document
                    .for_each_flattened_shape_view_for_cell(instance.cell, None, |_, view| {
                        let placed = transform.apply_rect(view.bounds);
                        bounds = Some(bounds.map_or(placed, |current: Rect| current.union(placed)));
                    });
            }
        }
        bounds
    }

    pub(crate) fn active_layer_alignment_bounds(
        &self,
        parent: CellId,
        selected_shape: Option<ShapeId>,
    ) -> Option<Rect> {
        let mut bounds: Option<Rect> = None;
        let mut extend = |shape: Shape| {
            if shape.layer == self.active_layer && Some(shape.id) != selected_shape {
                let shape_bounds = shape.kind.bounds();
                bounds = Some(bounds.map_or(shape_bounds, |current| current.union(shape_bounds)));
            }
        };
        if parent == self.workspace.document.top_cell {
            for shape in self.workspace.document.shapes.values() {
                extend(shape);
            }
        } else if let Some(cell) = self.workspace.document.cell(parent) {
            for shape in cell.shapes.values() {
                extend(shape);
            }
        }
        bounds
    }

    pub(crate) fn merge_active_layer_rectangles(&mut self) -> bool {
        let cell_id = self.layout_view_top_cell;
        let mut remaining = self
            .layout_cell_shapes(cell_id)
            .into_iter()
            .filter(|shape| {
                shape.layer == self.active_layer && matches!(shape.kind, ShapeKind::Rectangle(_))
            })
            .collect::<Vec<_>>();
        remaining.sort_by_key(|shape| std::cmp::Reverse(shape.id));
        if remaining.len() < 2 {
            self.status_message = "Active layer has fewer than two rectangles to merge".to_string();
            return true;
        }

        let mut groups = Vec::new();
        while let Some(shape) = remaining.pop() {
            let ShapeKind::Rectangle(bounds) = shape.kind else {
                continue;
            };
            if bounds.area() <= 0 {
                continue;
            }
            let mut group = vec![shape];
            let mut group_rects = vec![bounds];
            let mut changed = true;
            while changed {
                changed = false;
                let mut index = 0;
                while index < remaining.len() {
                    let ShapeKind::Rectangle(candidate_bounds) = remaining[index].kind else {
                        index += 1;
                        continue;
                    };
                    if candidate_bounds.area() > 0
                        && group_rects
                            .iter()
                            .any(|bounds| bounds.intersects(candidate_bounds))
                    {
                        let candidate = remaining.remove(index);
                        group_rects.push(candidate_bounds);
                        group.push(candidate);
                        changed = true;
                    } else {
                        index += 1;
                    }
                }
            }
            if group.len() > 1 {
                groups.push((rect_union_rectangles(&group_rects), group));
            }
        }

        if groups.is_empty() {
            self.status_message = "No touching active-layer rectangles to merge".to_string();
            return true;
        }

        let mut old_shapes = Vec::new();
        let mut merged_shapes = Vec::new();
        for (union_rects, group) in groups {
            let net = same_optional_net(&group);
            old_shapes.extend(group);
            for bounds in union_rects {
                merged_shapes.push(Shape {
                    id: self.workspace.document.allocate_shape_id(),
                    layer: self.active_layer,
                    net,
                    kind: ShapeKind::Rectangle(bounds),
                    name: None,
                    properties: BTreeMap::new(),
                });
            }
        }
        old_shapes.sort_by_key(|shape| shape.id);
        merged_shapes.sort_by_key(|shape| shape.id);
        let selected = merged_shapes.first().map(|shape| shape.id);

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        merged_shapes
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: merged_shapes
                    .iter()
                    .rev()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Merged {} rectangles into {} rectangle{}",
            old_shapes.len(),
            merged_shapes.len(),
            if merged_shapes.len() == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn intersect_active_layer_shapes_with_selection(&mut self) -> bool {
        let Some((cell_id, _, clip_shape)) = self.selected_current_cell_layout_shape() else {
            self.status_message =
                "Select a current-cell rectangle or polygon before intersecting a layer"
                    .to_string();
            return true;
        };
        let Some(clip_regions) = convex_region_parts_for_shape_kind(&clip_shape.kind) else {
            self.status_message = "Layer intersection needs a usable polygon region".to_string();
            return true;
        };

        let mut old_shapes = self
            .layout_cell_shapes(cell_id)
            .into_iter()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        old_shapes.sort_by_key(|shape| shape.id);
        if old_shapes.is_empty() {
            self.status_message =
                "Active layer has no current-cell rectangles or polygons to intersect".to_string();
            return true;
        }

        let mut clipped_shapes = Vec::new();
        for shape in &old_shapes {
            for clip_points in &clip_regions {
                let Some(clipped_kind) = clip_shape_kind_to_region(&shape.kind, clip_points) else {
                    continue;
                };
                clipped_shapes.push(Shape {
                    id: self.workspace.document.allocate_shape_id(),
                    layer: self.active_layer,
                    net: if clip_shape.net.is_none_or(|net| shape.net == Some(net)) {
                        shape.net
                    } else {
                        None
                    },
                    kind: clipped_kind,
                    name: None,
                    properties: shape.properties.clone(),
                });
            }
        }
        clipped_shapes.sort_by_key(|shape| shape.id);

        let unchanged = old_shapes.len() == clipped_shapes.len()
            && old_shapes
                .iter()
                .zip(&clipped_shapes)
                .all(|(old, clipped)| old.net == clipped.net && old.kind == clipped.kind);
        if unchanged {
            self.status_message =
                "Layer intersection left active-layer geometry unchanged".to_string();
            return true;
        }

        let selected = clipped_shapes.first().map(|shape| shape.id);
        let old_count = old_shapes.len();
        let clipped_count = clipped_shapes.len();
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        clipped_shapes
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: clipped_shapes
                    .iter()
                    .rev()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Intersected {} active-layer shape{} with selection; kept {} shape{}",
            old_count,
            if old_count == 1 { "" } else { "s" },
            clipped_count,
            if clipped_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn or_active_layer_shapes_with_selection(&mut self) -> bool {
        let Some((cell_id, _, region_shape)) = self.selected_current_cell_layout_shape() else {
            self.status_message =
                "Select a current-cell rectangle or polygon before unioning a layer".to_string();
            return true;
        };
        let Some(region_parts) = convex_region_parts_for_shape_kind(&region_shape.kind) else {
            self.status_message = "Layer OR needs a usable polygon region".to_string();
            return true;
        };

        let mut old_shapes = self
            .layout_cell_shapes(cell_id)
            .into_iter()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        old_shapes.sort_by_key(|shape| shape.id);

        let mut fragments = Vec::new();
        for shape in &old_shapes {
            for fragment_kind in subtract_convex_regions_from_region_parts(
                &shape.kind,
                region_parts.iter().map(Vec::as_slice),
            ) {
                fragments.push(Shape {
                    id: self.workspace.document.allocate_shape_id(),
                    layer: self.active_layer,
                    net: shape.net,
                    kind: fragment_kind,
                    name: None,
                    properties: shape.properties.clone(),
                });
            }
        }
        fragments.push(Shape {
            id: self.workspace.document.allocate_shape_id(),
            layer: self.active_layer,
            net: region_shape.net,
            kind: region_shape.kind.clone(),
            name: None,
            properties: region_shape.properties.clone(),
        });
        fragments.sort_by_key(|shape| shape.id);

        if old_shapes.is_empty() && fragments.is_empty() {
            self.status_message = "Layer OR found no shapes to write".to_string();
            return true;
        }

        let selected = fragments.first().map(|shape| shape.id);
        let old_count = old_shapes.len();
        let fragment_count = fragments.len();
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        fragments
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: fragments
                    .iter()
                    .rev()
                    .map(|shape| self.shape_delete_operation(cell_id, shape.id))
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| self.shape_add_operation(cell_id, shape)),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "ORed {} active-layer shape{} with selection; wrote {} fragment{}",
            old_count,
            if old_count == 1 { "" } else { "s" },
            fragment_count,
            if fragment_count == 1 { "" } else { "s" }
        );
        true
    }
}
