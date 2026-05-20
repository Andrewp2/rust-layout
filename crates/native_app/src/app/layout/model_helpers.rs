#![allow(unused_imports)]
use super::*;

pub(crate) fn default_active_layer(workspace: &WorkspaceDataset) -> LayerId {
    workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .or_else(|| workspace.document.layers.keys().next().copied())
        .unwrap_or(LayerId(1))
}

pub(crate) fn default_layout_shape(workspace: &WorkspaceDataset) -> Option<ShapeId> {
    layout_shape_ids(&workspace.document).first().copied()
}

pub(crate) fn layout_shape_ids(document: &Document) -> Vec<ShapeId> {
    document.shapes.keys().copied().collect()
}

pub(crate) fn build_layout_display_index(
    document: &Document,
    root_cell: CellId,
    min_depth: usize,
    max_depth: Option<usize>,
    hidden_cells: &BTreeSet<CellId>,
    layer_depth_overrides: &BTreeMap<LayerId, LayoutHierarchyDepth>,
) -> LayoutIndex {
    if hidden_cells.is_empty() && layer_depth_overrides.is_empty() {
        return LayoutIndex::rebuild_hierarchical_for_cell_with_depth_range(
            document, root_cell, min_depth, max_depth,
        );
    }
    let traversal_max_depth = layout_traversal_max_depth(max_depth, layer_depth_overrides);
    let mut entries = Vec::with_capacity(document.flattened_shape_count_estimate());
    document.for_each_visible_flattened_shape_view_for_cell_with_depth_range(
        root_cell,
        min_depth,
        traversal_max_depth,
        |id, view| {
            if !layout_occurrence_hidden_by_cells(document, root_cell, &id, hidden_cells)
                && layout_occurrence_in_layer_depth(
                    view.shape.layer,
                    id.hierarchy_depth(),
                    max_depth,
                    layer_depth_overrides,
                )
            {
                entries.push(IndexedShape {
                    id,
                    bounds: view.bounds,
                });
            }
        },
    );
    LayoutIndex::bulk_load(entries)
}

pub(crate) fn layout_traversal_max_depth(
    global_max_depth: Option<usize>,
    layer_depth_overrides: &BTreeMap<LayerId, LayoutHierarchyDepth>,
) -> Option<usize> {
    let mut traversal_max = global_max_depth;
    for depth in layer_depth_overrides.values() {
        match depth.max_depth() {
            Some(layer_max) => {
                if let Some(global_max) = traversal_max.as_mut() {
                    *global_max = (*global_max).max(layer_max);
                }
            }
            None => return None,
        }
    }
    traversal_max
}

pub(crate) fn layout_occurrence_in_layer_depth(
    layer: LayerId,
    occurrence_depth: usize,
    global_max_depth: Option<usize>,
    layer_depth_overrides: &BTreeMap<LayerId, LayoutHierarchyDepth>,
) -> bool {
    let effective_max_depth = layer_depth_overrides
        .get(&layer)
        .map(|depth| depth.max_depth())
        .unwrap_or(global_max_depth);
    effective_max_depth.is_none_or(|limit| occurrence_depth <= limit)
}

pub(crate) fn layout_occurrence_hidden_by_cells(
    document: &Document,
    root_cell: CellId,
    occurrence: &ShapeOccurrenceId,
    hidden_cells: &BTreeSet<CellId>,
) -> bool {
    if hidden_cells.is_empty() || occurrence.instance_path.is_empty() {
        return false;
    }
    let mut cell_id = root_cell;
    for instance_id in &occurrence.instance_path {
        let Some(instance) = document.instance(cell_id, *instance_id) else {
            return false;
        };
        if hidden_cells.contains(&instance.cell) {
            return true;
        }
        cell_id = instance.cell;
    }
    false
}

pub(crate) fn layout_top_occurrence_for_view_occurrence(
    document: &Document,
    root_cell: CellId,
    occurrence: &ShapeOccurrenceId,
) -> Option<ShapeOccurrenceId> {
    if root_cell == document.top_cell {
        return Some(occurrence.clone());
    }
    let (mut instance_path, mut array_path) = first_layout_context_to_cell(document, root_cell)?;
    instance_path.extend(occurrence.instance_path.iter().copied());
    array_path.extend(occurrence.array_path.iter().copied());
    Some(ShapeOccurrenceId {
        shape: occurrence.shape,
        instance_path,
        array_path,
    })
}

pub(crate) fn first_layout_context_to_cell(
    document: &Document,
    target: CellId,
) -> Option<(Vec<InstanceId>, Vec<ArrayIndex>)> {
    let mut instance_path = Vec::new();
    let mut array_path = Vec::new();
    let mut stack = BTreeSet::new();
    first_layout_context_to_cell_inner(
        document,
        document.top_cell,
        target,
        &mut instance_path,
        &mut array_path,
        &mut stack,
    )
    .then_some((instance_path, array_path))
}

pub(crate) fn first_layout_context_to_cell_inner(
    document: &Document,
    cell_id: CellId,
    target: CellId,
    instance_path: &mut Vec<InstanceId>,
    array_path: &mut Vec<ArrayIndex>,
    stack: &mut BTreeSet<CellId>,
) -> bool {
    if cell_id == target {
        return true;
    }
    if !stack.insert(cell_id) {
        return false;
    }
    let Some(cell) = document.cell(cell_id) else {
        stack.remove(&cell_id);
        return false;
    };
    let mut instances = cell.instances.values().collect::<Vec<_>>();
    instances.sort_by_key(|instance| instance.id);
    for instance in instances {
        instance_path.push(instance.id);
        let has_array_index = !instance.array.normalized().is_single();
        if has_array_index {
            array_path.push(ArrayIndex { column: 0, row: 0 });
        }
        if first_layout_context_to_cell_inner(
            document,
            instance.cell,
            target,
            instance_path,
            array_path,
            stack,
        ) {
            return true;
        }
        if has_array_index {
            array_path.pop();
        }
        instance_path.pop();
    }
    stack.remove(&cell_id);
    false
}

pub(crate) fn layout_layer_usage_counts(document: &Document) -> BTreeMap<LayerId, usize> {
    let mut counts = BTreeMap::new();
    for shape in document.shapes.values() {
        *counts.entry(shape.layer).or_insert(0) += 1;
    }
    for cell in document.cells.values() {
        for shape in cell.shapes.values() {
            *counts.entry(shape.layer).or_insert(0) += 1;
        }
    }
    counts
}

pub(crate) fn layout_layer_row_filter_count(
    document: &Document,
    group_filter: LayoutLayerGroupFilter,
    usage_filter: LayoutLayerUsageFilter,
) -> usize {
    let usage = layout_layer_usage_counts(document);
    document
        .layers
        .values()
        .filter(|layer| group_filter.matches(layer.process))
        .filter(|layer| {
            usage_filter.matches(usage.get(&layer.id).copied().unwrap_or(0), layer.visible)
        })
        .count()
}

pub(crate) fn layout_layer_usage_filter_button_label(
    document: &Document,
    group_filter: LayoutLayerGroupFilter,
    usage_filter: LayoutLayerUsageFilter,
) -> String {
    format!(
        "{} ({})",
        usage_filter.label(),
        layout_layer_row_filter_count(document, group_filter, usage_filter)
    )
}

pub(crate) fn layout_layer_group_filter_button_label(
    document: &Document,
    group_filter: LayoutLayerGroupFilter,
    usage_filter: LayoutLayerUsageFilter,
    label: &str,
) -> String {
    format!(
        "{label} ({})",
        layout_layer_row_filter_count(document, group_filter, usage_filter)
    )
}

pub(crate) fn layout_layer_visibility_preset_target_visible(
    mode: &str,
    layer: &Layer,
    usage_count: usize,
    active_layer: LayerId,
) -> Option<bool> {
    let used = usage_count > 0;
    match mode {
        "show_all" => Some(true),
        "show_used" => Some(used),
        "isolate_active" => Some(layer.id == active_layer),
        "invert" => Some(!layer.visible),
        "hide_empty" => Some(if used { layer.visible } else { false }),
        _ => None,
    }
}

pub(crate) fn layout_layer_visibility_preset_change_count(
    document: &Document,
    active_layer: LayerId,
    mode: &str,
) -> Option<usize> {
    let usage = layout_layer_usage_counts(document);
    let mut count = 0usize;
    for layer in document.layers.values() {
        let visible = layout_layer_visibility_preset_target_visible(
            mode,
            layer,
            usage.get(&layer.id).copied().unwrap_or(0),
            active_layer,
        )?;
        if layer.visible != visible {
            count += 1;
        }
    }
    Some(count)
}

pub(crate) fn layout_layer_visibility_preset_button_label(
    document: &Document,
    active_layer: LayerId,
    mode: &str,
    label: &str,
) -> String {
    format!(
        "{label} ({})",
        layout_layer_visibility_preset_change_count(document, active_layer, mode).unwrap_or(0)
    )
}

pub(crate) fn layout_inactive_empty_layer_count(
    document: &Document,
    active_layer: LayerId,
) -> usize {
    let usage = layout_layer_usage_counts(document);
    document
        .layers
        .values()
        .filter(|layer| layer.id != active_layer && usage.get(&layer.id).copied().unwrap_or(0) == 0)
        .count()
}

pub(crate) fn layout_inactive_empty_layer_cleanup_button_label(
    document: &Document,
    active_layer: LayerId,
) -> String {
    format!(
        "Prune Empty ({})",
        layout_inactive_empty_layer_count(document, active_layer)
    )
}

pub(crate) fn layout_occurrence_action_key(occurrence: &ShapeOccurrenceId) -> String {
    let instance_path = if occurrence.instance_path.is_empty() {
        "-".to_string()
    } else {
        occurrence
            .instance_path
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join("-")
    };
    let array_path = if occurrence.array_path.is_empty() {
        "-".to_string()
    } else {
        occurrence
            .array_path
            .iter()
            .map(|index| format!("{}x{}", index.column, index.row))
            .collect::<Vec<_>>()
            .join("-")
    };
    format!("{}~{}~{}", occurrence.shape.0, instance_path, array_path)
}

pub(crate) fn parse_layout_occurrence_action_key(value: &str) -> Option<ShapeOccurrenceId> {
    let mut parts = value.split('~');
    let shape = ShapeId(parts.next()?.parse::<u64>().ok()?);
    let instance_path = match parts.next() {
        Some("") | Some("-") | None => Vec::new(),
        Some(path) => path
            .split('-')
            .map(|id| id.parse::<u64>().ok().map(InstanceId))
            .collect::<Option<Vec<_>>>()?,
    };
    let array_path = match parts.next() {
        Some("") | Some("-") | None => Vec::new(),
        Some(path) => path
            .split('-')
            .map(|entry| {
                let (column, row) = entry.split_once('x')?;
                Some(ArrayIndex {
                    column: column.parse::<u32>().ok()?,
                    row: row.parse::<u32>().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?,
    };
    if parts.next().is_some() {
        return None;
    }
    Some(ShapeOccurrenceId {
        shape,
        instance_path,
        array_path,
    })
}

pub(crate) fn layout_instance_action_key(parent: CellId, id: InstanceId) -> String {
    format!("{}:{}", parent.0, id.0)
}

pub(crate) fn parse_layout_instance_action_key(value: &str) -> Option<(CellId, InstanceId)> {
    let (parent, id) = value.split_once(':')?;
    Some((CellId(parent.parse().ok()?), InstanceId(id.parse().ok()?)))
}

pub(crate) fn layout_depth_includes_occurrence(
    depth: LayoutHierarchyDepth,
    min_depth: u8,
    occurrence: &ShapeOccurrenceId,
) -> bool {
    occurrence.hierarchy_depth() >= min_depth as usize
        && depth
            .max_depth()
            .is_none_or(|limit| occurrence.hierarchy_depth() <= limit)
}

pub(crate) fn first_layout_occurrence_for_instance(
    document: &Document,
    parent: CellId,
    instance_id: InstanceId,
) -> Option<ShapeOccurrenceId> {
    let mut found = None;
    document.for_each_visible_flattened_shape_view_for_cell(parent, None, |occurrence, _| {
        if found.is_none() && occurrence.instance_path.first() == Some(&instance_id) {
            found = Some(occurrence);
        }
    });
    found
}

pub(crate) fn next_layout_shape_id(
    current: Option<ShapeId>,
    shape_ids: &[ShapeId],
) -> Option<ShapeId> {
    if shape_ids.is_empty() {
        return None;
    }
    let Some(current) = current else {
        return shape_ids.first().copied();
    };
    let index = shape_ids.iter().position(|id| *id == current);
    let next = index.map_or(0, |index| (index + 1) % shape_ids.len());
    shape_ids.get(next).copied()
}

pub(crate) fn move_first_layout_shape_vertex(document: &mut Document) -> Option<ShapeId> {
    let shape_id = layout_shape_ids(document).first().copied()?;
    let mut shape = document.shapes.get(&shape_id)?.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let moved = Point::new(rect.min.x - 500, rect.min.y + 420);
            *rect = Rect::new(moved, rect.max);
        }
        ShapeKind::Polygon(polygon) => {
            if let Some(point) = polygon.points.first_mut() {
                *point = Point::new(point.x - 500, point.y + 420);
            } else {
                return None;
            }
        }
        ShapeKind::Path { points, .. } => {
            if let Some(point) = points.first_mut() {
                *point = Point::new(point.x - 500, point.y + 420);
            } else {
                return None;
            }
        }
        ShapeKind::Via { center, .. } => {
            *center = Point::new(center.x - 500, center.y + 420);
        }
        ShapeKind::Label { position, .. } => {
            *position = Point::new(position.x - 500, position.y + 420);
        }
        ShapeKind::Measurement { a, .. } => {
            *a = Point::new(a.x - 500, a.y + 420);
        }
    }
    document.apply_operation_without_log(&Operation::ReplaceShape {
        id: shape_id,
        shape,
    });
    Some(shape_id)
}
