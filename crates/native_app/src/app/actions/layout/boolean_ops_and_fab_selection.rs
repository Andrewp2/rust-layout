#![allow(unused_imports)]
use super::*;

impl GlassworksApp {
    pub(crate) fn subtract_selection_from_active_layer_rectangles(&mut self) -> bool {
        let Some(cut_shape) = self.selected_top_level_layout_shape() else {
            self.status_message =
                "Select a top-level rectangle or polygon before subtracting from a layer"
                    .to_string();
            return true;
        };
        let Some(cut_regions) = convex_region_parts_for_shape_kind(&cut_shape.kind) else {
            self.status_message = "Layer subtraction needs a usable polygon region".to_string();
            return true;
        };

        let mut old_shapes = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && convex_region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        old_shapes.sort_by_key(|shape| shape.id);
        if old_shapes.is_empty() {
            self.status_message =
                "Active layer has no top-level rectangles or convex polygons to subtract from"
                    .to_string();
            return true;
        }

        let mut fragments = Vec::new();
        for shape in &old_shapes {
            for fragment_kind in subtract_convex_regions_from_shape_kind(
                &shape.kind,
                cut_regions.iter().map(Vec::as_slice),
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
        fragments.sort_by_key(|shape| shape.id);

        let unchanged = old_shapes.len() == fragments.len()
            && old_shapes
                .iter()
                .zip(&fragments)
                .all(|(old, fragment)| old.net == fragment.net && old.kind == fragment.kind);
        if unchanged {
            self.status_message =
                "Layer subtraction left active-layer shapes unchanged".to_string();
            return true;
        }

        let selected = fragments.first().map(|shape| shape.id);
        let old_count = old_shapes.len();
        let fragment_count = fragments.len();
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        fragments
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: fragments
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Subtracted selection from {} active-layer shape{}; kept {} fragment{}",
            old_count,
            if old_count == 1 { "" } else { "s" },
            fragment_count,
            if fragment_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn subtract_active_layer_rectangles_from_selection(&mut self) -> bool {
        let Some(source_shape) = self.selected_top_level_layout_shape() else {
            self.status_message =
                "Select a top-level rectangle or polygon before subtracting a layer".to_string();
            return true;
        };
        if convex_region_parts_for_shape_kind(&source_shape.kind).is_none() {
            self.status_message = "Selection minus layer needs a usable polygon region".to_string();
            return true;
        }

        let mut old_shapes = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && convex_region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        old_shapes.sort_by_key(|shape| shape.id);
        let active_regions = old_shapes
            .iter()
            .filter_map(|shape| convex_region_points_for_shape_kind(&shape.kind))
            .collect::<Vec<_>>();

        let mut fragments = subtract_convex_regions_from_region_parts(
            &source_shape.kind,
            active_regions.iter().map(Vec::as_slice),
        )
        .into_iter()
        .map(|fragment_kind| Shape {
            id: self.workspace.document.allocate_shape_id(),
            layer: self.active_layer,
            net: source_shape.net,
            kind: fragment_kind,
            name: None,
            properties: source_shape.properties.clone(),
        })
        .collect::<Vec<_>>();
        fragments.sort_by_key(|shape| shape.id);

        if old_shapes.is_empty() && fragments.is_empty() {
            self.status_message = "Selection minus layer found no shapes to write".to_string();
            return true;
        }

        let selected = fragments.first().map(|shape| shape.id);
        let old_count = old_shapes.len();
        let fragment_count = fragments.len();
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        fragments
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: fragments
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Subtracted {} active-layer shape{} from selection; wrote {} fragment{}",
            old_count,
            if old_count == 1 { "" } else { "s" },
            fragment_count,
            if fragment_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn xor_active_layer_rectangles_with_selection(&mut self) -> bool {
        let Some(cut_shape) = self.selected_top_level_layout_shape() else {
            self.status_message =
                "Select a top-level rectangle or polygon before XORing a layer".to_string();
            return true;
        };
        let Some(cut_regions) = convex_region_parts_for_shape_kind(&cut_shape.kind) else {
            self.status_message = "Layer XOR needs a usable polygon region".to_string();
            return true;
        };

        let mut old_shapes = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && convex_region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        old_shapes.sort_by_key(|shape| shape.id);

        let active_regions = old_shapes
            .iter()
            .filter_map(|shape| convex_region_points_for_shape_kind(&shape.kind))
            .collect::<Vec<_>>();

        let mut fragments = Vec::new();
        for shape in &old_shapes {
            for fragment_kind in subtract_convex_regions_from_shape_kind(
                &shape.kind,
                cut_regions.iter().map(Vec::as_slice),
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
        for fragment_kind in subtract_convex_regions_from_region_parts(
            &cut_shape.kind,
            active_regions.iter().map(Vec::as_slice),
        ) {
            fragments.push(Shape {
                id: self.workspace.document.allocate_shape_id(),
                layer: self.active_layer,
                net: cut_shape.net,
                kind: fragment_kind,
                name: None,
                properties: cut_shape.properties.clone(),
            });
        }
        fragments.sort_by_key(|shape| shape.id);

        if old_shapes.is_empty() && fragments.is_empty() {
            self.status_message = "Layer XOR found no shapes to write".to_string();
            return true;
        }

        let selected = fragments.first().map(|shape| shape.id);
        let old_count = old_shapes.len();
        let fragment_count = fragments.len();
        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: old_shapes
                    .iter()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        fragments
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
            Operation::Batch {
                operations: fragments
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(
                        old_shapes
                            .iter()
                            .cloned()
                            .map(|shape| Operation::AddShape { shape }),
                    )
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "XORed {} active-layer shape{} with selection; wrote {} fragment{}",
            old_count,
            if old_count == 1 { "" } else { "s" },
            fragment_count,
            if fragment_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn boolean_selected_shape_with_clipboard(
        &mut self,
        op: LayoutShapeClipboardBoolean,
    ) -> bool {
        let Some(source_shape) = self.selected_top_level_layout_shape() else {
            self.status_message =
                "Select a top-level rectangle or polygon before running shape boolean".to_string();
            return true;
        };
        let Some(source_regions) = convex_region_parts_for_shape_kind(&source_shape.kind) else {
            self.status_message =
                "Shape boolean currently supports selected rectangles and polygons".to_string();
            return true;
        };
        let Some(clipboard_shape) = self.layout_clipboard_shapes.first().cloned() else {
            self.status_message =
                "Copy a rectangle or polygon before running shape boolean".to_string();
            return true;
        };
        let Some(clipboard_regions) = convex_region_parts_for_shape_kind(&clipboard_shape.kind)
        else {
            self.status_message =
                "Shape boolean clipboard must be a rectangle or polygon".to_string();
            return true;
        };

        let fragment_kinds = match op {
            LayoutShapeClipboardBoolean::And => source_regions
                .iter()
                .filter_map(|points| shape_kind_from_region_points(points.clone()))
                .flat_map(|source_kind| {
                    clipboard_regions.iter().filter_map(move |clip_points| {
                        clip_shape_kind_to_region(&source_kind, clip_points)
                    })
                })
                .collect::<Vec<_>>(),
            LayoutShapeClipboardBoolean::Or => {
                let mut fragments = subtract_convex_regions_from_region_parts(
                    &source_shape.kind,
                    clipboard_regions.iter().map(Vec::as_slice),
                );
                fragments.push(clipboard_shape.kind.clone());
                fragments
            }
            LayoutShapeClipboardBoolean::Not => subtract_convex_regions_from_region_parts(
                &source_shape.kind,
                clipboard_regions.iter().map(Vec::as_slice),
            ),
            LayoutShapeClipboardBoolean::Xor => {
                let mut fragments = subtract_convex_regions_from_region_parts(
                    &source_shape.kind,
                    clipboard_regions.iter().map(Vec::as_slice),
                );
                fragments.extend(subtract_convex_regions_from_region_parts(
                    &clipboard_shape.kind,
                    source_regions.iter().map(Vec::as_slice),
                ));
                fragments
            }
        };
        let output_net = match op {
            LayoutShapeClipboardBoolean::And
                if clipboard_shape
                    .net
                    .is_none_or(|net| source_shape.net == Some(net)) =>
            {
                source_shape.net
            }
            LayoutShapeClipboardBoolean::Or if clipboard_shape.net == source_shape.net => {
                source_shape.net
            }
            LayoutShapeClipboardBoolean::Not => source_shape.net,
            LayoutShapeClipboardBoolean::Xor if clipboard_shape.net == source_shape.net => {
                source_shape.net
            }
            _ => None,
        };
        if fragment_kinds.len() == 1
            && fragment_kinds[0] == source_shape.kind
            && output_net == source_shape.net
        {
            self.status_message = format!(
                "Shape {} clipboard left shape #{} unchanged",
                op.label(),
                source_shape.id.0
            );
            return true;
        }

        let mut fragments = fragment_kinds
            .into_iter()
            .map(|kind| Shape {
                id: self.workspace.document.allocate_shape_id(),
                layer: source_shape.layer,
                net: output_net,
                kind,
                name: None,
                properties: BTreeMap::new(),
            })
            .collect::<Vec<_>>();
        fragments.sort_by_key(|shape| shape.id);
        let selected = fragments.first().map(|shape| shape.id);
        let fragment_count = fragments.len();

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: std::iter::once(Operation::DeleteShape {
                    id: source_shape.id,
                })
                .chain(
                    fragments
                        .iter()
                        .cloned()
                        .map(|shape| Operation::AddShape { shape }),
                )
                .collect(),
            },
            Operation::Batch {
                operations: fragments
                    .iter()
                    .rev()
                    .map(|shape| Operation::DeleteShape { id: shape.id })
                    .chain(std::iter::once(Operation::AddShape {
                        shape: source_shape.clone(),
                    }))
                    .collect(),
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence = selected.map(ShapeOccurrenceId::top_level);
        self.status_message = format!(
            "Shape {} clipboard replaced shape #{} with {} fragment{}",
            op.label(),
            source_shape.id.0,
            fragment_count,
            if fragment_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn create_clip_cell_from_selected_region(&mut self) -> bool {
        let Some(clip_shape) = self.selected_top_level_layout_shape() else {
            self.status_message =
                "Select a top-level rectangle or polygon before creating a clip cell".to_string();
            return true;
        };
        let Some(clip_regions) = convex_region_parts_for_shape_kind(&clip_shape.kind) else {
            self.status_message = "Clip cell creation needs a usable polygon region".to_string();
            return true;
        };
        let Some(clip_bounds) = region_points_for_shape_kind(&clip_shape.kind)
            .and_then(|points| Rect::from_points(&points))
        else {
            self.status_message = "Selected clip region has no usable bounds".to_string();
            return true;
        };

        let mut source_shapes = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| {
                shape.layer == self.active_layer
                    && shape.id != clip_shape.id
                    && region_points_for_shape_kind(&shape.kind).is_some()
            })
            .collect::<Vec<_>>();
        source_shapes.sort_by_key(|shape| shape.id);

        let origin = clip_bounds.min;
        let mut clipped_shapes = Vec::new();
        for shape in source_shapes {
            for clip_points in &clip_regions {
                let Some(mut clipped_kind) = clip_shape_kind_to_region(&shape.kind, clip_points)
                else {
                    continue;
                };
                clipped_kind.translate(Vector::new(-origin.x, -origin.y));
                clipped_shapes.push(Shape {
                    id: self.workspace.document.allocate_shape_id(),
                    layer: self.active_layer,
                    net: shape.net,
                    kind: clipped_kind,
                    name: shape.name.clone(),
                    properties: shape.properties.clone(),
                });
            }
        }

        if clipped_shapes.is_empty() {
            self.status_message =
                "Active layer has no top-level rectangles or polygons inside the selected clip region"
                    .to_string();
            return true;
        }

        let cell_id = self.workspace.document.allocate_cell_id();
        let instance_id = self.workspace.document.allocate_instance_id();
        let mut cell = Cell::new(cell_id, format!("clip {}", cell_id.0));
        for shape in &clipped_shapes {
            cell.shapes.insert(shape.id, shape.clone());
        }
        let instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", cell.name)),
            cell: cell_id,
            transform: Transform::translate(origin.x, origin.y),
            array: InstanceArray::single(),
            properties: BTreeMap::new(),
        };
        let selected = clipped_shapes.first().map(|shape| shape.id);
        let clipped_count = clipped_shapes.len();

        self.apply_layout_operation_with_history(
            Operation::Batch {
                operations: vec![
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
                ],
            },
        );
        self.selected_layout_shape = selected;
        self.selected_layout_occurrence =
            selected.map(|id| ShapeOccurrenceId::from_instance_path(id, &[instance_id]));
        self.status_message = format!(
            "Created {} from {} clipped shape{}",
            cell.name,
            clipped_count,
            if clipped_count == 1 { "" } else { "s" }
        );
        true
    }

    pub(crate) fn transform_selected_layout_instance(
        &mut self,
        transform: Transform,
        label: &str,
    ) -> bool {
        let Some(occurrence) = &self.selected_layout_occurrence else {
            return false;
        };
        let Some((parent, id)) = self.workspace.document.instance_parent_for_path_from_cell(
            self.layout_view_top_cell,
            &occurrence.instance_path,
        ) else {
            return false;
        };
        let Some(old_instance) = self.workspace.document.instance(parent, id) else {
            return false;
        };
        let mut instance = old_instance.clone();
        instance.transform = instance.transform.compose(transform);
        self.apply_layout_operation_with_history(
            Operation::ReplaceInstance {
                parent,
                id,
                instance,
            },
            Operation::ReplaceInstance {
                parent,
                id,
                instance: old_instance,
            },
        );
        self.status_message = label.to_string();
        true
    }

    pub(crate) fn transform_selected_layout_shapes(
        &mut self,
        transform: ShapeTransform,
        label: &str,
    ) -> bool {
        let Some(old_shape) = self.selected_top_level_layout_shape() else {
            self.status_message = "Select a top-level shape to transform".to_string();
            return true;
        };
        let center = old_shape.kind.bounds().center();
        let mut new_shape = old_shape.clone();
        new_shape.kind = transform_shape_kind(&old_shape.kind, center, transform);
        if old_shape == new_shape {
            self.status_message = format!("{label} shape #{}", old_shape.id.0);
            return true;
        }
        self.apply_layout_operation_with_history(
            Operation::ReplaceShape {
                id: old_shape.id,
                shape: new_shape,
            },
            Operation::ReplaceShape {
                id: old_shape.id,
                shape: old_shape.clone(),
            },
        );
        self.selected_layout_shape = Some(old_shape.id);
        self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(old_shape.id));
        self.status_message = format!("{label} shape #{}", old_shape.id.0);
        true
    }

    pub(crate) fn connectivity_summary(&self) -> String {
        let technology = self.active_connectivity_technology();
        match extract_connectivity(&self.workspace.document, &technology) {
            Ok(report) => format!(
                "Connectivity: {} component(s), {} short(s), {} open(s)",
                report.components.len(),
                report.shorts.len(),
                report.opens.len()
            ),
            Err(error) => format!("Connectivity failed: {error}"),
        }
    }

    pub(crate) fn layout_browser_replace_targets(&self) -> LayoutBrowserReplaceTargets {
        match self.layout_browser_replace_scope {
            LayoutBrowserReplaceScope::Document => self.document_layout_browser_replace_targets(),
            LayoutBrowserReplaceScope::CurrentCell => {
                self.current_cell_layout_browser_replace_targets()
            }
            LayoutBrowserReplaceScope::VisibleHierarchy => {
                self.visible_hierarchy_layout_browser_replace_targets()
            }
        }
    }

    pub(crate) fn document_layout_browser_replace_targets(&self) -> LayoutBrowserReplaceTargets {
        let document = &self.workspace.document;
        let mut targets = LayoutBrowserReplaceTargets {
            include_layers: true,
            ..Default::default()
        };
        targets.top_shapes.extend(document.shapes.keys().copied());
        for cell in document.cells.values() {
            targets.cells.insert(cell.id);
            targets
                .cell_shapes
                .insert(cell.id, cell.shapes.keys().copied().collect());
            targets
                .instances
                .insert(cell.id, cell.instances.keys().copied().collect());
        }
        targets
    }

    pub(crate) fn current_cell_layout_browser_replace_targets(
        &self,
    ) -> LayoutBrowserReplaceTargets {
        let document = &self.workspace.document;
        let mut targets = LayoutBrowserReplaceTargets::default();
        if self.layout_view_top_cell == document.top_cell {
            targets.top_shapes.extend(document.shapes.keys().copied());
        }
        let Some(cell) = document.cell(self.layout_view_top_cell) else {
            return targets;
        };
        targets.cells.insert(cell.id);
        targets
            .cell_shapes
            .insert(cell.id, cell.shapes.keys().copied().collect());
        targets
            .instances
            .insert(cell.id, cell.instances.keys().copied().collect());
        targets
    }

    pub(crate) fn visible_hierarchy_layout_browser_replace_targets(
        &self,
    ) -> LayoutBrowserReplaceTargets {
        let document = &self.workspace.document;
        let mut targets = LayoutBrowserReplaceTargets::default();
        document.for_each_visible_flattened_shape_view_for_cell_with_depth_range(
            self.layout_view_top_cell,
            self.layout_hierarchy_min_depth as usize,
            layout_traversal_max_depth(
                self.layout_hierarchy_depth.max_depth(),
                &self.layout_layer_depth_overrides,
            ),
            |occurrence, view| {
                if layout_occurrence_hidden_by_cells(
                    document,
                    self.layout_view_top_cell,
                    &occurrence,
                    &self.layout_hidden_cells,
                ) || !layout_occurrence_in_layer_depth(
                    view.shape.layer,
                    occurrence.hierarchy_depth(),
                    self.layout_hierarchy_depth.max_depth(),
                    &self.layout_layer_depth_overrides,
                ) {
                    return;
                }
                let shape_id = occurrence.source_shape_id();
                if view.source_cell == document.top_cell && document.shapes.contains_key(&shape_id)
                {
                    targets.top_shapes.insert(shape_id);
                } else {
                    targets
                        .cell_shapes
                        .entry(view.source_cell)
                        .or_default()
                        .insert(shape_id);
                }
            },
        );
        let mut stack = BTreeSet::new();
        self.collect_visible_hierarchy_replace_cells(
            self.layout_view_top_cell,
            0,
            self.layout_hierarchy_depth.max_depth(),
            &mut stack,
            &mut targets,
        );
        targets
    }

    pub(crate) fn collect_visible_hierarchy_replace_cells(
        &self,
        cell_id: CellId,
        depth: usize,
        max_depth: Option<usize>,
        stack: &mut BTreeSet<CellId>,
        targets: &mut LayoutBrowserReplaceTargets,
    ) {
        if !stack.insert(cell_id) {
            return;
        }
        let Some(cell) = self.workspace.document.cell(cell_id) else {
            stack.remove(&cell_id);
            return;
        };
        targets.cells.insert(cell_id);
        let mut instances = cell.instances.values().collect::<Vec<_>>();
        instances.sort_by_key(|instance| instance.id);
        for instance in instances {
            if self.layout_hidden_cells.contains(&instance.cell) {
                continue;
            }
            let child_depth = depth + 1;
            if max_depth.is_some_and(|max_depth| child_depth > max_depth) {
                continue;
            }
            targets
                .instances
                .entry(cell_id)
                .or_default()
                .insert(instance.id);
            self.collect_visible_hierarchy_replace_cells(
                instance.cell,
                child_depth,
                max_depth,
                stack,
                targets,
            );
        }
        stack.remove(&cell_id);
    }

    pub(crate) fn replace_layout_browser_search_matches(&mut self) -> bool {
        let query = self.layout_browser_search.trim().to_string();
        if query.is_empty() {
            self.status_message = "Enter browser search text before replacing".to_string();
            return true;
        }
        let replacement = self.layout_browser_replace.clone();
        let targets = self.layout_browser_replace_targets();
        let top_replacements = self
            .workspace
            .document
            .shapes
            .values()
            .filter(|shape| targets.includes_top_shape(shape.id))
            .filter_map(|shape| {
                shape_with_layout_search_replacement(&shape, &query, &replacement)
                    .map(|new_shape| (shape.id, shape, new_shape))
            })
            .collect::<Vec<_>>();
        let cell_shape_replacements = self
            .workspace
            .document
            .cells
            .values()
            .flat_map(|cell| {
                cell.shapes
                    .values()
                    .filter(|shape| targets.includes_cell_shape(cell.id, shape.id))
                    .filter_map(|shape| {
                        shape_with_layout_search_replacement(&shape, &query, &replacement)
                            .map(|new_shape| (cell.id, shape, new_shape))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let cell_name_replacements = self
            .workspace
            .document
            .cells
            .values()
            .filter(|cell| targets.includes_cell(cell.id))
            .filter_map(|cell| {
                replace_ascii_case_insensitive(&cell.name, &query, &replacement)
                    .map(|new_name| (cell.id, cell.name.clone(), new_name))
            })
            .collect::<Vec<_>>();
        let cell_property_replacements = self
            .workspace
            .document
            .cells
            .values()
            .filter(|cell| targets.includes_cell(cell.id))
            .filter_map(|cell| {
                layout_properties_with_search_replacement(&cell.properties, &query, &replacement)
                    .map(|properties| (cell.id, cell.properties.clone(), properties))
            })
            .collect::<Vec<_>>();
        let layer_name_replacements = self
            .workspace
            .document
            .layers
            .values()
            .filter(|_| targets.include_layers)
            .filter_map(|layer| {
                let new_name = replace_ascii_case_insensitive(&layer.name, &query, &replacement)?;
                (!new_name.trim().is_empty()).then(|| (layer.id, layer.name.clone(), new_name))
            })
            .collect::<Vec<_>>();
        let instance_replacements = self
            .workspace
            .document
            .cells
            .values()
            .flat_map(|cell| {
                cell.instances
                    .values()
                    .filter(|instance| targets.includes_instance(cell.id, instance.id))
                    .filter_map(|instance| {
                        instance_with_layout_search_replacement(&instance, &query, &replacement)
                            .map(|new_instance| (cell.id, instance.id, instance, new_instance))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let replacement_count = top_replacements.len()
            + cell_shape_replacements.len()
            + cell_name_replacements.len()
            + cell_property_replacements.len()
            + layer_name_replacements.len()
            + instance_replacements.len();
        if replacement_count == 0 {
            self.status_message = format!(
                "No layout objects matched {}",
                compact_button_label(&query, 32)
            );
            return true;
        }

        let mut redo = Vec::with_capacity(
            top_replacements.len()
                + cell_shape_replacements.len() * 2
                + cell_name_replacements.len()
                + cell_property_replacements.len()
                + layer_name_replacements.len()
                + instance_replacements.len(),
        );
        let mut undo = Vec::with_capacity(redo.capacity());
        for (shape_id, old_shape, new_shape) in &top_replacements {
            redo.push(Operation::ReplaceShape {
                id: *shape_id,
                shape: new_shape.clone(),
            });
            undo.push(Operation::ReplaceShape {
                id: *shape_id,
                shape: old_shape.clone(),
            });
        }
        for (cell, old_name, new_name) in &cell_name_replacements {
            redo.push(Operation::RenameCell {
                id: *cell,
                name: new_name.clone(),
            });
            undo.push(Operation::RenameCell {
                id: *cell,
                name: old_name.clone(),
            });
        }
        for (cell, old_properties, new_properties) in &cell_property_replacements {
            redo.push(Operation::SetCellProperties {
                id: *cell,
                properties: new_properties.clone(),
            });
            undo.push(Operation::SetCellProperties {
                id: *cell,
                properties: old_properties.clone(),
            });
        }
        for (layer, old_name, new_name) in &layer_name_replacements {
            redo.push(Operation::RenameLayer {
                id: *layer,
                name: new_name.clone(),
            });
            undo.push(Operation::RenameLayer {
                id: *layer,
                name: old_name.clone(),
            });
        }
        for (parent, instance_id, old_instance, new_instance) in &instance_replacements {
            redo.push(Operation::ReplaceInstance {
                parent: *parent,
                id: *instance_id,
                instance: new_instance.clone(),
            });
            undo.push(Operation::ReplaceInstance {
                parent: *parent,
                id: *instance_id,
                instance: old_instance.clone(),
            });
        }
        for (cell, old_shape, new_shape) in &cell_shape_replacements {
            redo.push(Operation::DeleteShapeFromCell {
                cell: *cell,
                id: old_shape.id,
            });
            redo.push(Operation::AddShapeToCell {
                cell: *cell,
                shape: new_shape.clone(),
            });
            undo.push(Operation::DeleteShapeFromCell {
                cell: *cell,
                id: new_shape.id,
            });
            undo.push(Operation::AddShapeToCell {
                cell: *cell,
                shape: old_shape.clone(),
            });
        }
        let selected = top_replacements.first().map(|(shape_id, _, _)| *shape_id);
        self.apply_layout_operation_with_history(
            Operation::Batch { operations: redo },
            Operation::Batch { operations: undo },
        );
        if let Some(shape_id) = selected {
            self.selected_layout_shape = Some(shape_id);
            self.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
        }
        self.layout_browser_search_active = false;
        self.layout_browser_replace_active = false;
        self.status_message = format!(
            "Replaced {} layout object{} matching {} in {}",
            replacement_count,
            if replacement_count == 1 { "" } else { "s" },
            compact_button_label(&query, 32),
            self.layout_browser_replace_scope.status_label()
        );
        true
    }

    pub(crate) fn export_layout_netlist(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_extracted_netlist_exchange_path();
            return self.export_layout_netlist_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Netlist export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_netlist_to_path(&mut self, path: &Path) -> bool {
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("Netlist export failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("Netlist export failed: {error}");
            return true;
        }
        let exchange = LayoutExtractedNetlistExchange::from_report(
            &self.workspace.document,
            self.layout_revision,
            self.active_layout_technology().name.clone(),
            report,
        );
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Netlist export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Netlist export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported netlist {} ({} component(s), {} device(s), {} short(s), {} open(s))",
            path.display(),
            exchange.components.len(),
            exchange.devices.len(),
            exchange.shorts.len(),
            exchange.opens.len()
        );
        true
    }

    pub(crate) fn export_layout_spice_netlist(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_spice_netlist_exchange_path();
            return self.export_layout_spice_netlist_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "SPICE netlist export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_spice_netlist_to_path(&mut self, path: &Path) -> bool {
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("SPICE netlist export failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("SPICE netlist export failed: {error}");
            return true;
        }
        let result = export_connectivity_spice(&self.workspace.document.name, &report);
        if let Err(error) = atomic_write_bytes(path, result.text.as_bytes()) {
            self.status_message = format!("SPICE netlist export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported SPICE netlist {} ({} component(s), {} device(s), {} named net(s), {} generated net(s), {} warning(s))",
            path.display(),
            result.report.component_count,
            result.report.device_count,
            result.report.named_net_count,
            result.report.generated_net_count,
            result.report.warnings.len()
        );
        true
    }

    pub(crate) fn compare_layout_spice_schematic(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_schematic_spice_exchange_path();
            return self.compare_layout_spice_schematic_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message =
                "SPICE schematic compare is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn compare_layout_spice_schematic_from_path(&mut self, path: &Path) -> bool {
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("SPICE schematic compare failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("SPICE schematic compare failed: {error}");
            return true;
        }
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("SPICE schematic compare failed: {error}");
                return true;
            }
        };
        let schematic = match parse_spice_schematic_netlist(&contents) {
            Ok(schematic) => schematic,
            Err(error) => {
                self.status_message = format!("SPICE schematic compare failed: {error}");
                return true;
            }
        };
        let comparison = compare_connectivity_to_spice_schematic(&report, &schematic);
        let status = comparison.status.label();
        let missing = comparison.missing_layout_nets.len();
        let extra = comparison.extra_layout_nets.len();
        let device_mismatches =
            comparison.missing_layout_devices.len() + comparison.extra_layout_devices.len();
        let layout_issues = comparison.layout_issue_count();
        let devices = comparison.schematic_device_count;
        let named_nets = comparison.layout_named_nets.len();
        self.layout_spice_comparison = Some(comparison);
        self.layout_net_browser_filter = LayoutNetBrowserFilter::All;
        self.layout_net_browser_sort = LayoutNetBrowserSort::Name;
        self.layout_browser_search.clear();
        self.status_message = if missing == 0 && extra == 0 && layout_issues == 0 {
            format!(
                "SPICE schematic {} matches layout connectivity ({} named net(s), {} device(s))",
                path.display(),
                named_nets,
                devices
            )
        } else {
            format!(
                "SPICE schematic compare {status}: {} missing, {} extra, {} problem(s) in {}",
                missing,
                extra,
                layout_issues + device_mismatches,
                path.display()
            )
        };
        true
    }

    pub(crate) fn import_layout_netlist(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_extracted_netlist_exchange_path();
            return self.import_layout_netlist_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Netlist import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_netlist_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Netlist import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutExtractedNetlistExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Netlist import failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "Netlist import failed: netlist schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        let component_count = exchange.components.len();
        let device_count = exchange.devices.len();
        let short_count = exchange.shorts.len();
        let open_count = exchange.opens.len();
        let report = exchange.into_report();
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("Netlist import failed: {error}");
            return true;
        }
        *self.connectivity_report_cache.borrow_mut() = Some(ConnectivityReportCacheValue {
            revision: self.layout_revision,
            report: Ok(report),
        });
        self.layout_spice_comparison = None;
        self.layout_net_browser_filter = LayoutNetBrowserFilter::All;
        self.layout_net_browser_sort = LayoutNetBrowserSort::Name;
        self.layout_browser_search.clear();
        self.status_message = format!(
            "Imported netlist {} ({} component(s), {} device(s), {} short(s), {} open(s))",
            path.display(),
            component_count,
            device_count,
            short_count,
            open_count
        );
        true
    }

    pub(crate) fn export_layout_trace_state(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_trace_state_exchange_path();
            return self.export_layout_trace_state_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Trace state export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_trace_state_to_path(&mut self, path: &Path) -> bool {
        let exchange = LayoutTraceStateExchange::from_app(self);
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("Trace state export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("Trace state export failed: {error}");
            return true;
        }
        self.status_message = format!(
            "Exported trace state {} ({} trace(s), {} route point(s))",
            path.display(),
            exchange.history.len(),
            exchange.route_points.len()
        );
        true
    }

    pub(crate) fn import_layout_trace_state(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_trace_state_exchange_path();
            return self.import_layout_trace_state_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "Trace state import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_trace_state_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("Trace state import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutTraceStateExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("Trace state import failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "Trace state import failed: trace schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }

        let (trace_count, route_point_count, skipped_components) =
            match self.apply_layout_trace_state_exchange(exchange) {
                Ok(summary) => summary,
                Err(error) => {
                    self.status_message = format!("Trace state import failed: {error}");
                    return true;
                }
            };
        self.status_message = format!(
            "Imported trace state {} ({} trace(s), {} route point(s), {} skipped)",
            path.display(),
            trace_count,
            route_point_count,
            skipped_components
        );
        true
    }

    pub(crate) fn apply_layout_trace_state_exchange(
        &mut self,
        exchange: LayoutTraceStateExchange,
    ) -> Result<(usize, usize, usize), String> {
        let report = self.connectivity_report()?;
        let known_components = report
            .components
            .iter()
            .map(|component| component.id)
            .collect::<BTreeSet<_>>();
        let mut skipped_components = 0;
        let mut history = Vec::new();
        for component_id in exchange.history {
            if !known_components.contains(&component_id) {
                skipped_components += 1;
                continue;
            }
            if !history.contains(&component_id) {
                history.push(component_id);
            }
            if history.len() >= MAX_LAYOUT_TRACE_HISTORY {
                break;
            }
        }
        let selected_component = exchange.selected_component.and_then(|component_id| {
            if known_components.contains(&component_id) {
                Some(component_id)
            } else {
                skipped_components += 1;
                None
            }
        });
        drop(report);

        let route_point_count = exchange.route_points.len();
        self.route_points = exchange.route_points;
        self.layout_trace_history = history.clone();
        self.layout_net_browser_filter = LayoutNetBrowserFilter::All;
        self.layout_browser_search.clear();
        self.layout_browser_search_active = false;

        let selected_component = selected_component.or_else(|| history.first().copied());
        if let Some(component_id) = selected_component {
            let selected = self.select_layout_net_component(&component_id.to_string());
            self.layout_trace_history = history;
            if !selected {
                self.selected_layout_shape = None;
                self.selected_layout_occurrence = None;
            }
        } else {
            self.selected_layout_shape = None;
            self.selected_layout_occurrence = None;
        }
        Ok((
            self.layout_trace_history.len(),
            route_point_count,
            skipped_components,
        ))
    }

    pub(crate) fn export_layout_l2n_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_l2n_database_exchange_path();
            return self.export_layout_l2n_database_to_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "L2N database export is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn export_layout_l2n_database_to_path(&mut self, path: &Path) -> bool {
        let report = match self.connectivity_report() {
            Ok(report) => report,
            Err(error) => {
                self.status_message = format!("L2N database export failed: {error}");
                return true;
            }
        };
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("L2N database export failed: {error}");
            return true;
        }
        let netlist = LayoutExtractedNetlistExchange::from_report(
            &self.workspace.document,
            self.layout_revision,
            self.active_layout_technology().name.clone(),
            report,
        );
        let trace_state = LayoutTraceStateExchange::from_app(self);
        let exchange = LayoutL2nDatabaseExchange {
            schema_version: LAYOUT_L2N_DATABASE_EXCHANGE_SCHEMA_VERSION,
            document_id: self.workspace.document.id.to_string(),
            document_name: self.workspace.document.name.clone(),
            layout_revision: self.layout_revision,
            netlist,
            trace_state,
        };
        let bytes = match serde_json::to_vec_pretty(&exchange) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status_message = format!("L2N database export failed: {error}");
                return true;
            }
        };
        if let Err(error) = atomic_write_bytes(path, &bytes) {
            self.status_message = format!("L2N database export failed: {error}");
            return true;
        }
        self.record_recent_layout_file("l2n_database", path);
        self.status_message = format!(
            "Exported L2N database {} ({} component(s), {} device(s), {} trace(s), {} route point(s))",
            path.display(),
            exchange.netlist.components.len(),
            exchange.netlist.devices.len(),
            exchange.trace_state.history.len(),
            exchange.trace_state.route_points.len()
        );
        true
    }

    pub(crate) fn import_layout_l2n_database(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = default_ui_l2n_database_exchange_path();
            return self.import_layout_l2n_database_from_path(&path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status_message = "L2N database import is available in the native app".to_string();
            true
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn import_layout_l2n_database_from_path(&mut self, path: &Path) -> bool {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                self.status_message = format!("L2N database import failed: {error}");
                return true;
            }
        };
        let exchange = match serde_json::from_str::<LayoutL2nDatabaseExchange>(&contents) {
            Ok(exchange) => exchange,
            Err(error) => {
                self.status_message = format!("L2N database import failed: {error}");
                return true;
            }
        };
        if exchange.schema_version != LAYOUT_L2N_DATABASE_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "L2N database import failed: database schema {} is unsupported; expected {}",
                exchange.schema_version, LAYOUT_L2N_DATABASE_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        if exchange.netlist.schema_version != LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "L2N database import failed: netlist schema {} is unsupported; expected {}",
                exchange.netlist.schema_version, LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        if exchange.trace_state.schema_version != LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION {
            self.status_message = format!(
                "L2N database import failed: trace schema {} is unsupported; expected {}",
                exchange.trace_state.schema_version, LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION
            );
            return true;
        }
        let component_count = exchange.netlist.components.len();
        let device_count = exchange.netlist.devices.len();
        let short_count = exchange.netlist.shorts.len();
        let open_count = exchange.netlist.opens.len();
        let report = exchange.netlist.into_report();
        if let Err(error) = Self::validate_connectivity_report(&report) {
            self.status_message = format!("L2N database import failed: {error}");
            return true;
        }
        *self.connectivity_report_cache.borrow_mut() = Some(ConnectivityReportCacheValue {
            revision: self.layout_revision,
            report: Ok(report),
        });
        self.layout_spice_comparison = None;
        let (trace_count, route_point_count, skipped_components) =
            match self.apply_layout_trace_state_exchange(exchange.trace_state) {
                Ok(summary) => summary,
                Err(error) => {
                    self.status_message = format!("L2N database import failed: {error}");
                    return true;
                }
            };
        self.status_message = format!(
            "Imported L2N database {} ({} component(s), {} device(s), {} short(s), {} open(s), {} trace(s), {} route point(s), {} skipped)",
            path.display(),
            component_count,
            device_count,
            short_count,
            open_count,
            trace_count,
            route_point_count,
            skipped_components
        );
        self.record_recent_layout_file("l2n_database", path);
        true
    }

    pub(crate) fn validate_connectivity_report(report: &ConnectivityReport) -> Result<(), String> {
        let errors = report
            .validate()
            .into_iter()
            .filter(|finding| finding.severity == ConnectivityValidationSeverity::Error)
            .map(|finding| finding.message)
            .collect::<Vec<_>>();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("; "))
        }
    }

    pub(crate) fn reset_layout_diff_review(&mut self) {
        self.layout_diff_change_page = 0;
        self.layout_diff_review_state = LayoutReviewDisposition::NeedsReview;
    }

    pub(crate) fn reset_layout_document_state(&mut self) {
        self.active_layer = default_active_layer(&self.workspace);
        self.layout_view_top_cell = self.workspace.document.top_cell;
        self.layout_previous_view = None;
        self.layout_tree_collapsed_cells.clear();
        self.layout_hidden_cells.clear();
        self.layout_layer_depth_overrides
            .retain(|layer, _| self.workspace.document.layers.contains_key(layer));
        self.selected_layout_shape = None;
        self.selected_layout_occurrence = None;
        self.layout_drc_marker_category_filter = None;
        self.layout_drc_report_history.clear();
        self.layout_selected_drc_report_id = None;
        self.layout_next_drc_report_id = 1;
        self.layout_custom_drc_deck = None;
        self.layout_custom_drc_deck_label = None;
        self.drc_report_cache.get_mut().take();
        self.layout_selected_drc_marker_key = None;
        self.layout_spice_comparison = None;
        self.layout_browser_search.clear();
        self.layout_browser_search_active = false;
        self.layout_browser_replace.clear();
        self.layout_browser_replace_active = false;
        self.layout_browser_columns = LayoutBrowserColumnSet::All;
        self.layout_clipboard_shapes.clear();
        self.clear_layout_edit_drafts();
        self.mark_layout_dirty();
    }

    pub(crate) fn ensure_notebook_selection(&mut self) {
        let selected_valid = self
            .selected_notebook_entry
            .as_ref()
            .is_some_and(|entry_id| {
                notebook_filtered_entries(self)
                    .iter()
                    .any(|entry| &entry.id == entry_id)
            });
        if !selected_valid {
            self.selected_notebook_entry = notebook_filtered_entries(self)
                .first()
                .map(|entry| entry.id.clone());
        }
    }

    pub(crate) fn apply_notebook_entry_action(&mut self, action: NotebookEntryAction) -> bool {
        let Some(entry_id) = self.selected_notebook_entry.clone() else {
            self.status_message = "Notebook: select an entry before applying an action".to_string();
            return false;
        };
        let Some(entry) = self.workspace.lab_notebook.entry_mut(&entry_id) else {
            return false;
        };
        self.status_message = match action {
            NotebookEntryAction::AddFollowUpPlan => {
                add_notebook_tag_once(entry, "follow-up");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Follow-up",
                    "- [ ] Review the open disposition before the next run.\n- [ ] Attach updated metrology or yield evidence.",
                );
                format!("added follow-up plan to notebook entry {}", entry.id)
            }
            NotebookEntryAction::InsertMetrologyReview => {
                add_notebook_tag_once(entry, "metrology-review");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Metrology Review",
                    "- Confirm linked metrology covers the selected lot and wafer.\n- Record any spec excursions and owner.",
                );
                format!("inserted metrology review for notebook entry {}", entry.id)
            }
            NotebookEntryAction::RequestImageEvidence => {
                add_notebook_tag_once(entry, "image-needed");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Image Evidence",
                    "- [ ] Add inspection image or SEM reference.\n- [ ] Link image evidence to the impacted lot.",
                );
                format!("requested image evidence for notebook entry {}", entry.id)
            }
            NotebookEntryAction::TagHandoff => {
                add_notebook_tag_once(entry, "handoff");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Shift Handoff",
                    "- [ ] Summarize open disposition, owner, and next lot to inspect.",
                );
                format!("tagged notebook entry {} for handoff", entry.id)
            }
        };
        entry.updated_at = "2026-05-06T17:00:00Z".to_string();
        true
    }

    pub(crate) fn focus_notebook_link(&mut self, kind: NotebookLinkKind, query: &str) -> bool {
        let Some(entry) = self
            .workspace
            .lab_notebook
            .entries
            .iter()
            .find(|entry| notebook_entry_has_link(entry, kind, query))
        else {
            return false;
        };
        self.selected_notebook_entry = Some(entry.id.clone());
        self.notebook_link_kind_filter = Some(kind);
        self.notebook_tag_filter = None;
        self.notebook_followups_only = false;
        self.status_message = format!("Notebook focused {} link {query}", kind.label());
        true
    }

    pub(crate) fn selected_recipe_for_tool(
        &self,
        tool: &EquipmentTool,
    ) -> Option<EquipmentRecipeId> {
        self.equipment_recipe_drafts
            .get(&tool.id)
            .cloned()
            .or_else(|| {
                tool.selected_recipe
                    .as_ref()
                    .map(|selection| selection.recipe_id.clone())
            })
            .or_else(|| tool.available_recipes.keys().next().cloned())
    }

    pub(crate) fn equipment_selection_for_tool(
        &self,
        tool: &EquipmentTool,
        recipe_id: EquipmentRecipeId,
    ) -> RecipeSelection {
        let mut selection = self.workspace.equipment.selection_for(&tool.id, recipe_id);
        if let Some(lot_id) = self.workflow_focus_lot.as_deref() {
            selection.lot_id = Some(lot_id.to_string());
            selection.wafer_id = self
                .workspace
                .mes
                .lots
                .values()
                .find(|lot| lot.id.as_str() == lot_id)
                .and_then(|lot| lot.wafers.first())
                .map(|wafer| wafer.id.as_str().to_string());
        }
        selection
    }
}
