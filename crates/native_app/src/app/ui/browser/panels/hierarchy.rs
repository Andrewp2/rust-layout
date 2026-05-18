#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_hierarchy_tree(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let rows = layout_hierarchy_tree_rows(app);
    if rows.is_empty() {
        return;
    }

    add_text(
        document,
        parent,
        "glassworks.layout.hierarchy_tree.title",
        "Hierarchy Tree",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    let branch_cell_count = layout_hierarchy_branch_cells(&app.workspace.document).len();
    if branch_cell_count > 0 {
        let controls = document.add_child(
            parent,
            UiNode::container(
                "glassworks.layout.hierarchy_tree.controls",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        add_button(
            document,
            controls,
            "glassworks.viewctl.layout.tree_expand_all",
            "Expand Tree",
            app.layout_tree_collapsed_cells.is_empty(),
            layout::with_flex(layout::row(), 1.0, 1.0, layout::px(ui_scale.value(0.0))),
            ui_scale,
        );
        add_button(
            document,
            controls,
            "glassworks.viewctl.layout.tree_collapse_all",
            "Collapse Tree",
            false,
            layout::with_flex(layout::row(), 1.0, 1.0, layout::px(ui_scale.value(0.0))),
            ui_scale,
        );
    }

    for (row_index, row) in rows.into_iter().take(32).enumerate() {
        let row_node = document.add_child(
            parent,
            UiNode::container(
                format!(
                    "glassworks.layout.hierarchy_tree.row.{}.{}",
                    row_index, row.cell.0
                ),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(4.0),
                ),
            ),
        );
        let toggle_label = if row.has_children {
            if row.collapsed { "+" } else { "-" }
        } else {
            ""
        };
        if row.has_children {
            add_button(
                document,
                row_node,
                format!("glassworks.viewctl.layout.tree_toggle.{}", row.cell.0),
                toggle_label,
                row.collapsed,
                layout::size(
                    layout::px(ui_scale.value(30.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        } else {
            add_text(
                document,
                row_node,
                format!(
                    "glassworks.layout.hierarchy_tree.leaf.{}.{}",
                    row_index, row.cell.0
                ),
                "",
                text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                layout::size(
                    layout::px(ui_scale.value(30.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
            );
        }
        let indent = "  ".repeat(row.depth.min(6));
        add_button(
            document,
            row_node,
            format!("glassworks.viewctl.layout.tree_cell.{}", row.cell.0),
            format!("{indent}{}", row.label),
            row.current,
            layout::with_flex(layout::row(), 1.0, 1.0, layout::px(ui_scale.value(0.0))),
            ui_scale,
        );
        if row.depth > 0 && row.cell != app.layout_view_top_cell {
            add_button(
                document,
                row_node,
                format!("glassworks.viewctl.layout.cell_visibility.{}", row.cell.0),
                if row.hidden { "Show" } else { "Hide" },
                row.hidden,
                layout::size(
                    layout::px(ui_scale.value(54.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
    }
}

pub(crate) fn add_layout_hierarchy_context(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let layout = &app.workspace.document;
    let Some(current_cell) = layout.cell(app.layout_view_top_cell) else {
        return;
    };
    let parent_cells = layout_parent_cells(layout, app.layout_view_top_cell);
    let child_cells = layout_child_cells(layout, app.layout_view_top_cell);
    let descendant_count = layout_descendant_cell_ids(layout, app.layout_view_top_cell)
        .into_iter()
        .filter(|cell| *cell != layout.top_cell && *cell != app.layout_view_top_cell)
        .count();
    let has_document_top_parent = parent_cells.iter().any(|cell| cell.id == layout.top_cell);
    let selected_instance = app.selected_layout_instance_context();
    let selected_local_instance = selected_instance
        .as_ref()
        .is_some_and(|(parent, _, _)| *parent == app.layout_view_top_cell);

    add_text(
        document,
        parent,
        "glassworks.layout.hierarchy_context.title",
        "Hierarchy Context",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    add_text(
        document,
        parent,
        "glassworks.layout.hierarchy_context.current",
        format!(
            "Current: {}",
            layout_cell_browser_label(current_cell, layout.top_cell)
        ),
        text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    if !child_cells.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_visibility.hide_children",
            "Hide child cells",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_visibility.show_children",
            "Show child cells",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if descendant_count > child_cells.len() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_visibility.hide_descendants",
            "Hide descendants",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_visibility.show_descendants",
            "Show descendants",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if !app.layout_hidden_cells.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_visibility.show_all",
            format!("Show hidden cells ({})", app.layout_hidden_cells.len()),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.duplicate_current_cell",
        "Duplicate cell",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    if let Some((_, id, instance)) = selected_instance
        && let Some(child) = layout.cell(instance.cell)
    {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.descend_selected_instance",
            format!(
                "Descend #{} {}",
                id.0,
                compact_button_label(&child.name, 12)
            ),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.make_cell_variant",
            format!("Variant #{}", id.0),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.flatten_selected_instance_one",
            format!("Flatten #{} 1 level", id.0),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if !current_cell.instances.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.flatten_current_cell_one",
            "Flatten cell 1 level",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.flatten_current_cell",
            "Flatten cell",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if app.selected_layout_occurrence.is_some() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_origin.selection",
            "Origin to selection",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.cell_origin.exact",
        "Origin exact",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    for (action, label) in [
        ("x_neg", "Origin -X"),
        ("x_pos", "Origin +X"),
        ("y_neg", "Origin -Y"),
        ("y_pos", "Origin +Y"),
    ] {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.cell_origin.{action}"),
            label,
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if app.layout_view_top_cell != layout.top_cell
        && app
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(ShapeOccurrenceId::is_top_level)
        && !parent_cells.is_empty()
    {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.move_shape_up",
            "Move shape up",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if app.layout_view_top_cell != layout.top_cell
        && selected_local_instance
        && !parent_cells.is_empty()
    {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.move_instance_up",
            "Move instance up",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if app.layout_view_top_cell != layout.top_cell && !has_document_top_parent {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.context_top_cell.{}",
                layout.top_cell.0
            ),
            "Document top",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
    if app.layout_view_top_cell != layout.top_cell {
        let references = layout_cell_instance_refs(layout, app.layout_view_top_cell).len();
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.delete_unused_cell",
            if references == 0 {
                "Delete unused cell".to_string()
            } else {
                format!("Delete unused ({} refs)", references)
            },
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.delete_cell_shallow",
            if references == 0 {
                "Shallow delete cell".to_string()
            } else {
                format!("Shallow delete ({} refs)", references)
            },
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        let deep_delete_cells = layout_deep_delete_cell_ids(layout, app.layout_view_top_cell).len();
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.delete_cell_deep",
            format!("Deep delete ({} cells)", deep_delete_cells),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        let complete_delete_cell_ids = layout_descendant_cell_ids(layout, app.layout_view_top_cell);
        let mut complete_delete_cells = complete_delete_cell_ids.len();
        if complete_delete_cell_ids.contains(&layout.top_cell) {
            complete_delete_cells = complete_delete_cells.saturating_sub(1);
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.delete_cell_complete",
            format!("Complete delete ({} cells)", complete_delete_cells),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }

    add_text(
        document,
        parent,
        "glassworks.layout.hierarchy_context.parents",
        if parent_cells.is_empty() {
            "Parents: none".to_string()
        } else {
            "Parents".to_string()
        },
        text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for cell in parent_cells {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.context_parent_cell.{}", cell.id.0),
            format!("Up {}", compact_button_label(&cell.name, 14)),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }

    add_text(
        document,
        parent,
        "glassworks.layout.hierarchy_context.children",
        if child_cells.is_empty() {
            "Children: none".to_string()
        } else {
            "Children".to_string()
        },
        text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for cell in child_cells {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.context_child_cell.{}", cell.id.0),
            format!("Down {}", compact_button_label(&cell.name, 12)),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
