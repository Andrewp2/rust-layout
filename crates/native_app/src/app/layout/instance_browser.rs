#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_instance_browser_entries(
    app: &GlassworksApp,
) -> Vec<(CellId, InstanceId, String)> {
    layout_instance_browser_entries_with_filter(
        app,
        app.layout_instance_browser_filter,
        true,
        MAX_LAYOUT_BROWSER_INSTANCES,
    )
}

pub(crate) fn layout_instance_browser_entries_with_filter(
    app: &GlassworksApp,
    filter: LayoutInstanceBrowserFilter,
    apply_search: bool,
    limit: usize,
) -> Vec<(CellId, InstanceId, String)> {
    let document = &app.workspace.document;
    let search = apply_search
        .then(|| layout_browser_search_query_lower(app))
        .flatten();
    let mut entries = match app.layout_instance_browser_scope {
        LayoutInstanceBrowserScope::CurrentCell => layout_instance_browser_entries_for_cell(
            document,
            app.layout_view_top_cell,
            false,
            MAX_LAYOUT_BROWSER_INSTANCES,
        ),
        LayoutInstanceBrowserScope::Hierarchy => {
            let mut entries = Vec::new();
            let mut visited_cells = BTreeSet::new();
            collect_layout_instance_browser_entries(
                document,
                app.layout_view_top_cell,
                &mut visited_cells,
                &mut entries,
            );
            entries
        }
    };
    entries.retain(|(parent, id, label)| {
        document.instance(*parent, *id).is_some_and(|instance| {
            layout_instance_matches_filter(document, &instance, filter, &app.layout_hidden_cells)
                && search.as_deref().is_none_or(|query| {
                    if filter == LayoutInstanceBrowserFilter::Properties {
                        layout_instance_selector_matches_search(document, *parent, &instance, query)
                            || layout_properties_selector_matches_search(
                                &instance.properties,
                                query,
                            )
                    } else {
                        layout_instance_matches_search(document, *parent, &instance, label, query)
                    }
                })
        })
    });
    sort_layout_instance_browser_entries(document, app.layout_instance_browser_sort, &mut entries);
    entries.truncate(limit);
    entries
}

pub(crate) fn layout_instance_browser_filter_count(
    app: &GlassworksApp,
    filter: LayoutInstanceBrowserFilter,
) -> usize {
    layout_instance_browser_entries_with_filter(app, filter, true, usize::MAX).len()
}

pub(crate) fn layout_instance_browser_filter_button_label(
    app: &GlassworksApp,
    filter: LayoutInstanceBrowserFilter,
) -> String {
    format!(
        "{} ({})",
        filter.label(),
        layout_instance_browser_filter_count(app, filter)
    )
}

pub(crate) fn layout_instance_browser_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if app.layout_instance_browser_scope == LayoutInstanceBrowserScope::CurrentCell
        && app.layout_instance_browser_filter == LayoutInstanceBrowserFilter::All
        && search.is_empty()
    {
        return format!("Instance Browser ({listed_count} {row_label})");
    }
    let mut context = Vec::new();
    if app.layout_instance_browser_scope != LayoutInstanceBrowserScope::CurrentCell {
        context.push(app.layout_instance_browser_scope.label().to_string());
    }
    if app.layout_instance_browser_filter != LayoutInstanceBrowserFilter::All {
        context.push(app.layout_instance_browser_filter.label().to_string());
    }
    if !search.is_empty() {
        context.push(format!("search {}", compact_button_label(search, 24)));
    }
    format!(
        "Instance Browser - {} ({listed_count} {row_label})",
        context.join(" / ")
    )
}

pub(crate) fn layout_instance_browser_filter_status_label(
    filter: LayoutInstanceBrowserFilter,
) -> String {
    match filter {
        LayoutInstanceBrowserFilter::All => "instances".to_string(),
        LayoutInstanceBrowserFilter::Named => "named instances".to_string(),
        LayoutInstanceBrowserFilter::Properties => "property-bearing instances".to_string(),
        LayoutInstanceBrowserFilter::Arrays => "array instances".to_string(),
        LayoutInstanceBrowserFilter::Hidden => "hidden-target instances".to_string(),
        LayoutInstanceBrowserFilter::Visible => "visible-target instances".to_string(),
        LayoutInstanceBrowserFilter::LeafTargets => "leaf-target instances".to_string(),
        LayoutInstanceBrowserFilter::BranchTargets => "branch-target instances".to_string(),
        LayoutInstanceBrowserFilter::Identity => "identity instances".to_string(),
        LayoutInstanceBrowserFilter::Transformed => "transformed instances".to_string(),
    }
}

pub(crate) fn layout_instance_matches_filter(
    document: &Document,
    instance: &CellInstance,
    filter: LayoutInstanceBrowserFilter,
    hidden_cells: &BTreeSet<CellId>,
) -> bool {
    match filter {
        LayoutInstanceBrowserFilter::All => true,
        LayoutInstanceBrowserFilter::Named => {
            instance.name.as_ref().is_some_and(|name| !name.is_empty())
        }
        LayoutInstanceBrowserFilter::Properties => !instance.properties.is_empty(),
        LayoutInstanceBrowserFilter::Arrays => !instance.array.is_single(),
        LayoutInstanceBrowserFilter::Hidden => hidden_cells.contains(&instance.cell),
        LayoutInstanceBrowserFilter::Visible => !hidden_cells.contains(&instance.cell),
        LayoutInstanceBrowserFilter::LeafTargets => document
            .cell(instance.cell)
            .is_some_and(|cell| cell.instances.is_empty()),
        LayoutInstanceBrowserFilter::BranchTargets => document
            .cell(instance.cell)
            .is_some_and(|cell| !cell.instances.is_empty()),
        LayoutInstanceBrowserFilter::Identity => instance.transform == Transform::IDENTITY,
        LayoutInstanceBrowserFilter::Transformed => instance.transform != Transform::IDENTITY,
    }
}

pub(crate) fn layout_instance_matches_search(
    document: &Document,
    parent: CellId,
    instance: &CellInstance,
    label: &str,
    query_lower: &str,
) -> bool {
    if layout_instance_selector_matches_search(document, parent, instance, query_lower) {
        return true;
    }
    let array = instance.array.normalized();
    let mut fields = vec![
        label.to_string(),
        format!("#{}", instance.id.0),
        format!("instance {}", instance.id.0),
        format!("C{}", parent.0),
        format!("parent {}", parent.0),
        format!("C{}", instance.cell.0),
        format!("child {}", instance.cell.0),
        format!(
            "{},{}",
            instance.transform.translation.dx, instance.transform.translation.dy
        ),
    ];
    if let Some(name) = &instance.name {
        push_layout_search_field(&mut fields, name.clone());
    }
    for (key, value) in &instance.properties {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
        push_layout_search_field(&mut fields, format!("{key}={value}"));
    }
    if !array.is_single() {
        push_layout_search_field(&mut fields, "array");
        push_layout_search_field(&mut fields, format!("{}x{}", array.columns, array.rows));
    }
    if let Some(parent_cell) = document.cell(parent) {
        push_layout_search_field(&mut fields, parent_cell.name.clone());
    }
    if let Some(child_cell) = document.cell(instance.cell) {
        push_layout_search_field(&mut fields, child_cell.name.clone());
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_instance_selector_matches_search(
    document: &Document,
    parent: CellId,
    instance: &CellInstance,
    query_lower: &str,
) -> bool {
    let Some((key_query, value_query)) = query_lower.split_once('=') else {
        return false;
    };
    let key_query = key_query.trim();
    let value_query = value_query.trim();
    if key_query.is_empty() && value_query.is_empty() {
        return true;
    }
    let parent_cell_label = document
        .cell(parent)
        .map(|cell| format!("C{} {}", cell.id.0, cell.name))
        .unwrap_or_else(|| format!("C{}", parent.0));
    let child_cell_label = document
        .cell(instance.cell)
        .map(|cell| format!("C{} {}", cell.id.0, cell.name))
        .unwrap_or_else(|| format!("C{}", instance.cell.0));
    let array = instance.array.normalized();
    let array_label = if array.is_single() {
        "single".to_string()
    } else {
        format!("{}x{}", array.columns, array.rows)
    };
    let translation = format!(
        "{},{}",
        instance.transform.translation.dx, instance.transform.translation.dy
    );
    let matrix = format!(
        "[{} {} ; {} {}]",
        instance.transform.matrix[0],
        instance.transform.matrix[1],
        instance.transform.matrix[2],
        instance.transform.matrix[3]
    );
    let target_rows = layout_instance_target_property_rows(document, instance.cell);
    let mut fields = vec![
        ("id", instance.id.0.to_string()),
        ("instance", instance.id.0.to_string()),
        ("instance_id", instance.id.0.to_string()),
        ("parent", parent_cell_label.clone()),
        ("parent_cell", parent_cell_label),
        ("parent_cell_id", parent.0.to_string()),
        ("child", child_cell_label.clone()),
        ("child_cell", child_cell_label.clone()),
        ("target", child_cell_label.clone()),
        ("target_cell", child_cell_label),
        ("child_cell_id", instance.cell.0.to_string()),
        ("target_cell_id", instance.cell.0.to_string()),
        ("translation", translation.clone()),
        ("transform", translation),
        ("dx", instance.transform.translation.dx.to_string()),
        ("dy", instance.transform.translation.dy.to_string()),
        ("matrix", matrix),
        ("array", array_label),
        ("array_columns", array.columns.to_string()),
        ("columns", array.columns.to_string()),
        ("array_rows", array.rows.to_string()),
        ("rows", array.rows.to_string()),
        ("target_role", target_rows.role.clone()),
        ("role", target_rows.role),
        ("target_shapes", target_rows.shape_count.to_string()),
        (
            "target_child_instances",
            target_rows.child_instance_count.to_string(),
        ),
        ("target_bounds", target_rows.bounds.clone()),
        ("bounds", target_rows.bounds),
    ];
    if let Some(name) = &instance.name {
        fields.push(("name", name.clone()));
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
        || layout_properties_selector_matches_search(&instance.properties, query_lower)
}

pub(crate) fn layout_instance_browser_entries_for_cell(
    document: &Document,
    cell_id: CellId,
    include_parent: bool,
    limit: usize,
) -> Vec<(CellId, InstanceId, String)> {
    let Some(cell) = document.cell(cell_id) else {
        return Vec::new();
    };
    let mut instances = cell.instances.values().collect::<Vec<_>>();
    instances.sort_by_key(|instance| instance.id);
    instances
        .into_iter()
        .take(limit)
        .map(|instance| {
            (
                cell_id,
                instance.id,
                layout_instance_browser_label(document, cell_id, &instance, include_parent),
            )
        })
        .collect()
}

pub(crate) fn collect_layout_instance_browser_entries(
    document: &Document,
    cell_id: CellId,
    visited_cells: &mut BTreeSet<CellId>,
    entries: &mut Vec<(CellId, InstanceId, String)>,
) {
    if entries.len() >= MAX_LAYOUT_BROWSER_INSTANCES || !visited_cells.insert(cell_id) {
        return;
    }
    let cell_entries = layout_instance_browser_entries_for_cell(
        document,
        cell_id,
        true,
        MAX_LAYOUT_BROWSER_INSTANCES.saturating_sub(entries.len()),
    );
    let child_cells = cell_entries
        .iter()
        .filter_map(|(parent, instance_id, _)| {
            document
                .instance(*parent, *instance_id)
                .map(|instance| instance.cell)
        })
        .collect::<Vec<_>>();
    entries.extend(cell_entries);
    for child_cell in child_cells {
        if entries.len() >= MAX_LAYOUT_BROWSER_INSTANCES {
            break;
        }
        collect_layout_instance_browser_entries(document, child_cell, visited_cells, entries);
    }
}

pub(crate) fn sort_layout_instance_browser_entries(
    document: &Document,
    sort: LayoutInstanceBrowserSort,
    entries: &mut [(CellId, InstanceId, String)],
) {
    entries.sort_by(|left, right| {
        let left_instance = document.instance(left.0, left.1);
        let right_instance = document.instance(right.0, right.1);
        let left_parent = document.cell(left.0).map(|cell| cell.name.as_str());
        let right_parent = document.cell(right.0).map(|cell| cell.name.as_str());
        let left_child = left_instance
            .as_ref()
            .and_then(|instance| document.cell(instance.cell).map(|cell| cell.name.as_str()));
        let right_child = right_instance
            .as_ref()
            .and_then(|instance| document.cell(instance.cell).map(|cell| cell.name.as_str()));
        let left_array_size = left_instance
            .as_ref()
            .map(|instance| {
                let array = instance.array.normalized();
                array.columns.saturating_mul(array.rows)
            })
            .unwrap_or(0);
        let right_array_size = right_instance
            .as_ref()
            .map(|instance| {
                let array = instance.array.normalized();
                array.columns.saturating_mul(array.rows)
            })
            .unwrap_or(0);
        match sort {
            LayoutInstanceBrowserSort::Id => left.1.cmp(&right.1),
            LayoutInstanceBrowserSort::Parent => left_parent
                .cmp(&right_parent)
                .then_with(|| left.0.cmp(&right.0))
                .then_with(|| left.1.cmp(&right.1)),
            LayoutInstanceBrowserSort::Child => left_child
                .cmp(&right_child)
                .then_with(|| {
                    left_instance
                        .as_ref()
                        .map(|instance| instance.cell)
                        .cmp(&right_instance.as_ref().map(|instance| instance.cell))
                })
                .then_with(|| left.1.cmp(&right.1)),
            LayoutInstanceBrowserSort::Array => right_array_size
                .cmp(&left_array_size)
                .then_with(|| left.1.cmp(&right.1)),
        }
    });
}

pub(crate) fn layout_instance_browser_label(
    document: &Document,
    parent: CellId,
    instance: &CellInstance,
    include_parent: bool,
) -> String {
    let parent_label = include_parent
        .then(|| {
            document
                .cell(parent)
                .map(|cell| compact_button_label(&cell.name, 8))
                .unwrap_or_else(|| format!("C{}", parent.0))
        })
        .map(|label| format!("{label} "))
        .unwrap_or_default();
    let child = document
        .cell(instance.cell)
        .map(|cell| compact_button_label(&cell.name, 12))
        .unwrap_or_else(|| format!("cell {}", instance.cell.0));
    let array = instance.array.normalized();
    let array_label = if array.is_single() {
        String::new()
    } else {
        format!(" {}x{}", array.columns, array.rows)
    };
    compact_button_label(
        &format!(
            "{}#{} -> {}{} @{},{}",
            parent_label,
            instance.id.0,
            child,
            array_label,
            instance.transform.translation.dx,
            instance.transform.translation.dy
        ),
        30,
    )
}

pub(crate) fn selected_layout_instance_key(app: &GlassworksApp) -> Option<(CellId, InstanceId)> {
    let occurrence = app.selected_layout_occurrence.as_ref()?;
    app.workspace
        .document
        .instance_parent_for_path_from_cell(app.layout_view_top_cell, &occurrence.instance_path)
}

pub(crate) fn layout_instance_browser_property_rows(
    app: &GlassworksApp,
    entries: &[(CellId, InstanceId, String)],
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let key = selected_layout_instance_key(app)
        .filter(|selected| {
            entries
                .iter()
                .any(|(parent, id, _)| (*parent, *id) == *selected)
        })
        .or_else(|| entries.first().map(|(parent, id, _)| (*parent, *id)));
    let Some((parent, id)) = key else {
        return Vec::new();
    };
    let Some(instance) = document.instance(parent, id) else {
        return Vec::new();
    };
    let array = instance.array.normalized();
    let array_label = if array.is_single() {
        "single".to_string()
    } else {
        format!(
            "{}x{} col {},{} row {},{}",
            array.columns,
            array.rows,
            array.column_pitch.dx,
            array.column_pitch.dy,
            array.row_pitch.dx,
            array.row_pitch.dy
        )
    };
    let translation = format!(
        "{},{}",
        instance.transform.translation.dx, instance.transform.translation.dy
    );
    let matrix = format!(
        "[{} {} ; {} {}]",
        instance.transform.matrix[0],
        instance.transform.matrix[1],
        instance.transform.matrix[2],
        instance.transform.matrix[3]
    );
    let name = instance.name.clone().unwrap_or_else(|| "None".to_string());
    let target_rows = layout_instance_target_property_rows(document, instance.cell);
    let mut rows = match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            (
                "Child cell".to_string(),
                layout_cell_display_name(document, instance.cell),
            ),
            target_rows.role_row(),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            ("Translation".to_string(), translation),
            ("Matrix".to_string(), matrix),
            ("Array".to_string(), array_label),
            target_rows.bounds_row(),
        ],
        LayoutBrowserColumnSet::Relations => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            (
                "Parent cell".to_string(),
                layout_cell_display_name(document, parent),
            ),
            (
                "Child cell".to_string(),
                layout_cell_display_name(document, instance.cell),
            ),
            ("Name".to_string(), name),
            ("Array".to_string(), array_label),
            target_rows.role_row(),
            target_rows.shape_count_row(),
            target_rows.child_instance_count_row(),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            (
                "Parent cell".to_string(),
                layout_cell_display_name(document, parent),
            ),
            (
                "Child cell".to_string(),
                layout_cell_display_name(document, instance.cell),
            ),
            ("Name".to_string(), name),
            ("Translation".to_string(), translation),
            ("Matrix".to_string(), matrix),
            ("Array".to_string(), array_label),
            target_rows.role_row(),
            target_rows.shape_count_row(),
            target_rows.child_instance_count_row(),
            target_rows.bounds_row(),
        ],
    };
    rows.extend(layout_stored_property_rows(&instance.properties));
    rows
}

pub(crate) struct LayoutInstanceTargetRows {
    role: String,
    shape_count: usize,
    child_instance_count: usize,
    bounds: String,
}

impl LayoutInstanceTargetRows {
    fn role_row(&self) -> (String, String) {
        ("Target role".to_string(), self.role.clone())
    }

    fn shape_count_row(&self) -> (String, String) {
        ("Target shapes".to_string(), self.shape_count.to_string())
    }

    fn child_instance_count_row(&self) -> (String, String) {
        (
            "Target child instances".to_string(),
            self.child_instance_count.to_string(),
        )
    }

    fn bounds_row(&self) -> (String, String) {
        ("Target local bounds".to_string(), self.bounds.clone())
    }
}

pub(crate) fn layout_instance_target_property_rows(
    document: &Document,
    cell_id: CellId,
) -> LayoutInstanceTargetRows {
    let Some(cell) = document.cell(cell_id) else {
        return LayoutInstanceTargetRows {
            role: "missing".to_string(),
            shape_count: 0,
            child_instance_count: 0,
            bounds: "None".to_string(),
        };
    };
    let shape_count = layout_instance_target_local_shape_count(document, cell);
    let child_instance_count = cell.instances.len();
    let role = if child_instance_count == 0 {
        "leaf".to_string()
    } else {
        "branch".to_string()
    };
    let bounds = layout_instance_target_local_bounds(document, cell)
        .map(rect_summary)
        .unwrap_or_else(|| "None".to_string());
    LayoutInstanceTargetRows {
        role,
        shape_count,
        child_instance_count,
        bounds,
    }
}

pub(crate) fn layout_instance_target_local_shape_count(document: &Document, cell: &Cell) -> usize {
    cell.shapes.len()
        + if cell.id == document.top_cell {
            document.shapes.len()
        } else {
            0
        }
}

pub(crate) fn layout_instance_target_local_bounds(
    document: &Document,
    cell: &Cell,
) -> Option<Rect> {
    let cell_bounds = cell
        .shapes
        .values()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right));
    if cell.id == document.top_cell {
        document
            .shapes
            .values()
            .map(|shape| shape.kind.bounds())
            .chain(cell_bounds)
            .reduce(|left, right| left.union(right))
    } else {
        cell_bounds
    }
}
