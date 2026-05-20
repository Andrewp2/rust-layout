#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutHierarchyTreeRow {
    pub(crate) cell: CellId,
    pub(crate) depth: usize,
    pub(crate) label: String,
    pub(crate) has_children: bool,
    pub(crate) collapsed: bool,
    pub(crate) hidden: bool,
    pub(crate) current: bool,
}

pub(crate) fn layout_hierarchy_tree_rows(app: &GlassworksApp) -> Vec<LayoutHierarchyTreeRow> {
    let mut rows = Vec::new();
    let mut lineage = BTreeSet::new();
    if let Some(query) = layout_browser_search_query_lower(app) {
        collect_layout_hierarchy_tree_search_rows(
            &app.workspace.document,
            app.workspace.document.top_cell,
            0,
            app,
            &query,
            &mut lineage,
            &mut rows,
        );
    } else {
        collect_layout_hierarchy_tree_rows(
            &app.workspace.document,
            app.workspace.document.top_cell,
            0,
            app,
            &mut lineage,
            &mut rows,
        );
    }
    rows
}

pub(crate) fn layout_hierarchy_tree_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if search.is_empty() {
        return format!("Hierarchy Tree ({listed_count} {row_label})");
    }
    format!(
        "Hierarchy Tree - search {} ({listed_count} {row_label})",
        compact_button_label(search, 24)
    )
}

pub(crate) fn layout_hierarchy_branch_cells(document: &Document) -> BTreeSet<CellId> {
    let mut branch_cells = BTreeSet::new();
    let mut lineage = BTreeSet::new();
    collect_layout_hierarchy_branch_cells(
        document,
        document.top_cell,
        &mut lineage,
        &mut branch_cells,
    );
    branch_cells
}

pub(crate) fn collect_layout_hierarchy_branch_cells(
    document: &Document,
    cell_id: CellId,
    lineage: &mut BTreeSet<CellId>,
    branch_cells: &mut BTreeSet<CellId>,
) {
    let Some(cell) = document.cell(cell_id) else {
        return;
    };
    if !lineage.insert(cell_id) {
        return;
    }
    let child_cells = ordered_child_cells_for_tree(document, cell);
    if !child_cells.is_empty() {
        branch_cells.insert(cell_id);
    }
    for child_id in child_cells {
        collect_layout_hierarchy_branch_cells(document, child_id, lineage, branch_cells);
    }
    lineage.remove(&cell_id);
}

pub(crate) fn collect_layout_hierarchy_tree_rows(
    document: &Document,
    cell_id: CellId,
    depth: usize,
    app: &GlassworksApp,
    lineage: &mut BTreeSet<CellId>,
    rows: &mut Vec<LayoutHierarchyTreeRow>,
) {
    let Some(cell) = document.cell(cell_id) else {
        return;
    };
    let child_cells = ordered_child_cells_for_tree(document, cell);
    let has_children = !child_cells.is_empty();
    let collapsed = app.layout_tree_collapsed_cells.contains(&cell_id);
    let hidden = app.layout_hidden_cells.contains(&cell_id);
    let label = if depth == 0 {
        layout_cell_browser_label(cell, document.top_cell)
    } else {
        format!(
            "C{} {} {}s/{}i{}",
            cell.id.0,
            compact_button_label(&cell.name, 12),
            cell.shapes.len(),
            cell.instances.len(),
            if hidden { " hidden" } else { "" }
        )
    };
    rows.push(LayoutHierarchyTreeRow {
        cell: cell_id,
        depth,
        label,
        has_children,
        collapsed,
        hidden,
        current: app.layout_view_top_cell == cell_id,
    });
    if collapsed || !lineage.insert(cell_id) {
        return;
    }
    for child_id in child_cells {
        collect_layout_hierarchy_tree_rows(document, child_id, depth + 1, app, lineage, rows);
    }
    lineage.remove(&cell_id);
}

pub(crate) fn collect_layout_hierarchy_tree_search_rows(
    document: &Document,
    cell_id: CellId,
    depth: usize,
    app: &GlassworksApp,
    query_lower: &str,
    lineage: &mut BTreeSet<CellId>,
    rows: &mut Vec<LayoutHierarchyTreeRow>,
) -> bool {
    let Some(cell) = document.cell(cell_id) else {
        return false;
    };
    if !lineage.insert(cell_id) {
        return false;
    }
    let child_cells = ordered_child_cells_for_tree(document, cell);
    let mut child_rows = Vec::new();
    for child_id in &child_cells {
        collect_layout_hierarchy_tree_search_rows(
            document,
            *child_id,
            depth + 1,
            app,
            query_lower,
            lineage,
            &mut child_rows,
        );
    }
    let matches = layout_hierarchy_tree_cell_matches_search(document, cell, query_lower);
    let include = matches || !child_rows.is_empty();
    if include {
        rows.push(layout_hierarchy_tree_row(
            document,
            cell,
            depth,
            app,
            !child_cells.is_empty(),
            false,
        ));
        rows.extend(child_rows);
    }
    lineage.remove(&cell_id);
    include
}

pub(crate) fn layout_hierarchy_tree_cell_matches_search(
    document: &Document,
    cell: &Cell,
    query_lower: &str,
) -> bool {
    if layout_hierarchy_tree_cell_selector_matches_search(document, cell, query_lower) {
        return true;
    }
    if layout_properties_selector_matches_search(&cell.properties, query_lower) {
        return true;
    }
    let depth_label = layout_cell_hierarchy_depth_label(document, cell.id);
    let mut fields = vec![
        cell.name.clone(),
        format!("C{}", cell.id.0),
        format!("cell {}", cell.id.0),
        format!("hierarchy depth {depth_label}"),
        format!("depth {depth_label}"),
        format!("{} shapes", cell.shapes.len()),
        format!("{} instances", cell.instances.len()),
    ];
    if cell.id == document.top_cell {
        push_layout_search_field(&mut fields, "top");
    }
    if cell.instances.is_empty() {
        push_layout_search_field(&mut fields, "leaf");
    } else {
        push_layout_search_field(&mut fields, "branch");
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_hierarchy_tree_cell_selector_matches_search(
    document: &Document,
    cell: &Cell,
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
    let depth_label = layout_cell_hierarchy_depth_label(document, cell.id);
    let role = if cell.id == document.top_cell {
        "top"
    } else if layout_parent_cells(document, cell.id).is_empty() {
        "unused"
    } else if cell.instances.is_empty() {
        "leaf"
    } else {
        "branch"
    };
    let fields = vec![
        ("cell", cell.name.clone()),
        ("cell", format!("C{}", cell.id.0)),
        ("cell_id", cell.id.0.to_string()),
        ("id", cell.id.0.to_string()),
        ("name", cell.name.clone()),
        ("role", role.to_string()),
        ("state", role.to_string()),
        ("hierarchy_depth", depth_label.clone()),
        ("depth", depth_label),
        ("shapes", cell.shapes.len().to_string()),
        ("shape_count", cell.shapes.len().to_string()),
        ("instances", cell.instances.len().to_string()),
        ("child_instances", cell.instances.len().to_string()),
        ("child_instance_count", cell.instances.len().to_string()),
    ];
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn layout_hierarchy_tree_row(
    document: &Document,
    cell: &Cell,
    depth: usize,
    app: &GlassworksApp,
    has_children: bool,
    collapsed: bool,
) -> LayoutHierarchyTreeRow {
    let hidden = app.layout_hidden_cells.contains(&cell.id);
    let label = if depth == 0 {
        layout_cell_browser_label(cell, document.top_cell)
    } else {
        format!(
            "C{} {} {}s/{}i{}",
            cell.id.0,
            compact_button_label(&cell.name, 12),
            cell.shapes.len(),
            cell.instances.len(),
            if hidden { " hidden" } else { "" }
        )
    };
    LayoutHierarchyTreeRow {
        cell: cell.id,
        depth,
        label,
        has_children,
        collapsed,
        hidden,
        current: app.layout_view_top_cell == cell.id,
    }
}

pub(crate) fn ordered_child_cells_for_tree(document: &Document, cell: &Cell) -> Vec<CellId> {
    let mut child_ids = cell
        .instances
        .values()
        .map(|instance| instance.cell)
        .collect::<Vec<_>>();
    child_ids.sort_by(|left, right| {
        let left_cell = document.cell(*left);
        let right_cell = document.cell(*right);
        left_cell
            .map(|cell| cell.name.to_lowercase())
            .cmp(&right_cell.map(|cell| cell.name.to_lowercase()))
            .then_with(|| left.cmp(right))
    });
    child_ids.dedup();
    child_ids
}
