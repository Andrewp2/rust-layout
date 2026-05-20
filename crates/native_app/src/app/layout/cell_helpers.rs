#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_parent_cells(document: &Document, child_cell: CellId) -> Vec<&Cell> {
    let parent_ids = layout_cell_instance_refs(document, child_cell)
        .into_iter()
        .map(|(parent, _)| parent)
        .collect::<BTreeSet<_>>();
    ordered_layout_cells(document)
        .into_iter()
        .filter(|cell| parent_ids.contains(&cell.id))
        .collect()
}

pub(crate) fn layout_cell_instance_refs(
    document: &Document,
    child_cell: CellId,
) -> Vec<(CellId, InstanceId)> {
    let mut refs = Vec::new();
    for cell in document.cells.values() {
        for instance in cell.instances.values() {
            if instance.cell == child_cell {
                refs.push((cell.id, instance.id));
            }
        }
    }
    refs.sort_unstable();
    refs
}

pub(crate) fn layout_descendant_cell_ids(
    document: &Document,
    root_cell: CellId,
) -> BTreeSet<CellId> {
    let mut descendants = BTreeSet::new();
    let mut stack = vec![root_cell];
    while let Some(cell_id) = stack.pop() {
        if !descendants.insert(cell_id) {
            continue;
        }
        if let Some(cell) = document.cell(cell_id) {
            stack.extend(cell.instances.values().map(|instance| instance.cell));
        }
    }
    descendants
}

pub(crate) fn layout_ancestor_cell_ids(document: &Document, root_cell: CellId) -> BTreeSet<CellId> {
    let mut ancestors = BTreeSet::new();
    let mut stack = layout_cell_instance_refs(document, root_cell)
        .into_iter()
        .map(|(parent, _)| parent)
        .collect::<Vec<_>>();
    while let Some(cell_id) = stack.pop() {
        if !ancestors.insert(cell_id) {
            continue;
        }
        stack.extend(
            layout_cell_instance_refs(document, cell_id)
                .into_iter()
                .map(|(parent, _)| parent),
        );
    }
    ancestors.remove(&root_cell);
    ancestors
}

pub(crate) fn layout_unused_cell_ids(document: &Document) -> BTreeSet<CellId> {
    let used_cells = layout_descendant_cell_ids(document, document.top_cell);
    document
        .cells
        .keys()
        .copied()
        .filter(|cell_id| *cell_id != document.top_cell && !used_cells.contains(cell_id))
        .collect()
}

pub(crate) fn layout_deep_delete_cell_ids(
    document: &Document,
    root_cell: CellId,
) -> BTreeSet<CellId> {
    let candidates = layout_descendant_cell_ids(document, root_cell);
    let mut delete_cells = candidates.clone();
    loop {
        let preserved = delete_cells
            .iter()
            .copied()
            .filter(|cell_id| *cell_id != root_cell)
            .filter(|cell_id| {
                layout_cell_instance_refs(document, *cell_id)
                    .iter()
                    .any(|(parent, _)| !delete_cells.contains(parent))
            })
            .collect::<Vec<_>>();
        if preserved.is_empty() {
            break;
        }
        for cell_id in preserved {
            delete_cells.remove(&cell_id);
        }
    }
    delete_cells
}

pub(crate) fn layout_cell_with_local_origin_delta(mut cell: Cell, delta: Vector) -> Cell {
    let shifted_shapes = cell
        .shapes
        .values()
        .map(|mut shape| {
            shape.kind.translate(delta);
            shape
        })
        .collect::<Vec<_>>();
    for shape in shifted_shapes {
        cell.shapes.insert(shape.id, shape);
    }

    let shift = Transform::from_translation(delta);
    let shifted_instances = cell
        .instances
        .values()
        .map(|mut instance| {
            instance.transform = shift.compose(instance.transform);
            instance
        })
        .collect::<Vec<_>>();
    for instance in shifted_instances {
        cell.instances.insert(instance.id, instance);
    }
    cell
}

pub(crate) fn layout_child_cells(document: &Document, parent_cell: CellId) -> Vec<&Cell> {
    let child_ids = document
        .cell(parent_cell)
        .map(|cell| {
            cell.instances
                .values()
                .map(|instance| instance.cell)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    ordered_layout_cells(document)
        .into_iter()
        .filter(|cell| child_ids.contains(&cell.id))
        .collect()
}

pub(crate) fn layout_sibling_cell_ids(document: &Document, cell_id: CellId) -> BTreeSet<CellId> {
    let parent_ids = layout_cell_instance_refs(document, cell_id)
        .into_iter()
        .map(|(parent, _)| parent)
        .collect::<BTreeSet<_>>();
    let mut siblings = BTreeSet::new();
    for parent_id in parent_ids {
        if let Some(parent) = document.cell(parent_id) {
            siblings.extend(
                parent
                    .instances
                    .values()
                    .map(|instance| instance.cell)
                    .filter(|sibling| *sibling != cell_id && *sibling != document.top_cell),
            );
        }
    }
    siblings
}

pub(crate) fn layout_cell_browser_label(cell: &Cell, document_top_cell: CellId) -> String {
    let prefix = if cell.id == document_top_cell {
        "Doc top".to_string()
    } else {
        format!("C{}", cell.id.0)
    };
    format!(
        "{} {} {}s/{}i",
        prefix,
        compact_button_label(&cell.name, 12),
        cell.shapes.len(),
        cell.instances.len()
    )
}
