#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_instance_browser_entries(
    app: &GlassworksApp,
) -> Vec<(CellId, InstanceId, String)> {
    let document = &app.workspace.document;
    let search = layout_browser_search_query_lower(app);
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
            layout_instance_matches_filter(&instance, app.layout_instance_browser_filter)
                && search.as_deref().is_none_or(|query| {
                    if app.layout_instance_browser_filter == LayoutInstanceBrowserFilter::Properties
                    {
                        layout_properties_selector_matches_search(&instance.properties, query)
                    } else {
                        layout_instance_matches_search(document, *parent, &instance, label, query)
                    }
                })
        })
    });
    sort_layout_instance_browser_entries(document, app.layout_instance_browser_sort, &mut entries);
    entries
}

pub(crate) fn layout_instance_matches_filter(
    instance: &CellInstance,
    filter: LayoutInstanceBrowserFilter,
) -> bool {
    match filter {
        LayoutInstanceBrowserFilter::All => true,
        LayoutInstanceBrowserFilter::Named => {
            instance.name.as_ref().is_some_and(|name| !name.is_empty())
        }
        LayoutInstanceBrowserFilter::Properties => !instance.properties.is_empty(),
        LayoutInstanceBrowserFilter::Arrays => !instance.array.is_single(),
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
    let mut rows = match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            (
                "Child cell".to_string(),
                layout_cell_display_name(document, instance.cell),
            ),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Instance id".to_string(), format!("#{}", instance.id.0)),
            ("Translation".to_string(), translation),
            ("Matrix".to_string(), matrix),
            ("Array".to_string(), array_label),
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
        ],
    };
    rows.extend(layout_stored_property_rows(&instance.properties));
    rows
}
