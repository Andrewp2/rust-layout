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
    collect_layout_hierarchy_tree_rows(
        &app.workspace.document,
        app.workspace.document.top_cell,
        0,
        app,
        &mut lineage,
        &mut rows,
    );
    rows
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
