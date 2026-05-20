#![allow(unused_imports)]
use super::*;

pub(crate) fn add_control_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    rows: Vec<Vec<ViewControlButton>>,
    ui_scale: UiScale,
    body_width: f32,
) {
    add_control_panel_with_button_width(
        document, parent, name, title, rows, 132.0, ui_scale, body_width,
    );
}

pub(crate) fn add_control_panel_with_button_width(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    rows: Vec<Vec<ViewControlButton>>,
    button_width: f32,
    ui_scale: UiScale,
    body_width: f32,
) {
    let title_height = ui_scale.value(22.0);
    let label_height = ui_scale.value(14.0);
    let button_height = ui_scale.value(26.0);
    let button_gap = ui_scale.value(4.0);
    let group_gap = ui_scale.value(12.0);
    let vertical_gap = ui_scale.value(4.0);
    let padding = ui_scale.value(8.0);
    let base_button_width = ui_scale.value(button_width.min(124.0));
    let group_height = label_height + ui_scale.value(2.0) + button_height;
    let max_group_row_width = (body_width - ui_scale.value(220.0)).max(ui_scale.value(180.0));
    let max_group_row_width = if body_width < ui_scale.value(700.0) {
        max_group_row_width.min(ui_scale.value(286.0))
    } else {
        max_group_row_width
    };
    let groups = control_panel_group_rows(
        &rows,
        base_button_width,
        button_gap,
        group_gap,
        max_group_row_width,
    );
    let group_row_count = groups.len().max(1);
    let height = padding * 2.0
        + title_height
        + group_row_count as f32 * group_height
        + group_row_count as f32 * vertical_gap;
    let panel = document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    vertical_gap,
                ),
                padding,
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_text(
        document,
        panel,
        format!("{name}.title"),
        title,
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(title_height)),
    );

    for (group_row_index, group_row) in groups.iter().enumerate() {
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("{name}.group_row.{group_row_index}"),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(group_height),
                    ),
                    group_gap,
                ),
            ),
        );

        for &row_index in group_row {
            let row_buttons = &rows[row_index];
            let row_label = control_row_label(name, row_index, row_buttons);
            let group_button_width = control_panel_button_width(
                row_buttons.len(),
                base_button_width,
                button_gap,
                max_group_row_width,
            );
            let group_width = control_panel_group_width(
                row_buttons.len(),
                group_button_width,
                button_gap,
                max_group_row_width,
            );
            let group = document.add_child(
                row,
                UiNode::container(
                    format!("{name}.row.{row_index}"),
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(group_width),
                            layout::px(group_height),
                        ),
                        ui_scale.value(2.0),
                    ),
                ),
            );
            add_text(
                document,
                group,
                format!("{name}.row.{row_index}.label"),
                row_label,
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::px(group_width), layout::px(label_height)),
            );
            let buttons = document.add_child(
                group,
                UiNode::container(
                    format!("{name}.row.{row_index}.buttons"),
                    layout::with_gap_all(
                        layout::with_size(
                            layout::row(),
                            layout::px(group_width),
                            layout::px(button_height),
                        ),
                        button_gap,
                    ),
                ),
            );
            for button in row_buttons {
                add_button(
                    document,
                    buttons,
                    button.name.clone(),
                    button.label.clone(),
                    button.selected,
                    layout::size(layout::px(group_button_width), layout::px(button_height)),
                    ui_scale,
                );
            }
        }
    }
}

pub(crate) fn control_panel_group_rows(
    rows: &[Vec<ViewControlButton>],
    base_button_width: f32,
    button_gap: f32,
    group_gap: f32,
    max_width: f32,
) -> Vec<Vec<usize>> {
    let mut grouped_rows = Vec::new();
    let mut current_row = Vec::new();
    let mut current_width = 0.0;
    for (row_index, row) in rows.iter().enumerate() {
        let group_button_width =
            control_panel_button_width(row.len(), base_button_width, button_gap, max_width);
        let group_width =
            control_panel_group_width(row.len(), group_button_width, button_gap, max_width);
        let next_width = if current_row.is_empty() {
            group_width
        } else {
            current_width + group_gap + group_width
        };
        if !current_row.is_empty() && next_width > max_width {
            grouped_rows.push(current_row);
            current_row = Vec::new();
            current_width = 0.0;
        }
        current_width = if current_row.is_empty() {
            group_width
        } else {
            current_width + group_gap + group_width
        };
        current_row.push(row_index);
    }
    if !current_row.is_empty() {
        grouped_rows.push(current_row);
    }
    grouped_rows
}

pub(crate) fn control_panel_button_width(
    button_count: usize,
    base_button_width: f32,
    button_gap: f32,
    max_group_width: f32,
) -> f32 {
    let button_count = button_count.max(1);
    let fit_width = (max_group_width - button_count.saturating_sub(1) as f32 * button_gap)
        / button_count as f32;
    base_button_width.min(fit_width).max(52.0)
}

pub(crate) fn control_panel_group_width(
    button_count: usize,
    button_width: f32,
    button_gap: f32,
    max_group_width: f32,
) -> f32 {
    let button_count = button_count.max(1);
    (button_count as f32 * button_width + button_count.saturating_sub(1) as f32 * button_gap)
        .min(max_group_width)
}

pub(crate) fn control_row_label(
    panel_name: &str,
    row_index: usize,
    row_buttons: &[ViewControlButton],
) -> &'static str {
    let Some(first) = row_buttons.first().map(|button| button.name.as_str()) else {
        return "Controls";
    };
    if panel_name == "glassworks.viewctl.workflow" {
        return match row_index {
            0 => "Design",
            1 => "Operations",
            2 => "Materials",
            3 => "Facilities",
            4 => "Quality",
            5 => "Trace",
            6 => "Data",
            _ => "Lot focus",
        };
    }
    if panel_name == "glassworks.viewctl.fab" {
        if first.contains(".fab.select.") {
            return if row_index == 0 {
                "Tools"
            } else {
                "More tools"
            };
        }
        if first.contains(".fab.recipe.") {
            return "Recipe";
        }
        if first.contains(".fab.command.online") {
            return "Run";
        }
        if first.contains(".fab.command.alarm") {
            return "Service";
        }
    }
    if first.contains(".layout_diff.demo_current") || first.contains(".layout_diff.empty_current") {
        return "Load";
    }
    if first.contains(".layout_diff.reset_filters") {
        return "Reset";
    }
    if first.contains(".workflow.open.") {
        return "Open";
    }
    if first.contains(".workflow.load_demo") {
        return "Data";
    }
    if first.contains(".workflow.focus_lot.") {
        return "Lot focus";
    }
    if first.contains(".mask.lot.") {
        return "Source lot";
    }
    if first.contains(".mask.severity.") {
        return "Severity";
    }
    if first.contains(".mask.group.") {
        return "Group by";
    }
    if first.contains(".mask.prev_page") || first.contains(".mask.next_page") {
        return "Issues";
    }
    if first.contains(".mask.rebuild") || first.contains(".mask.clear_filters") {
        return "Actions";
    }
    if first.contains(".layout_diff.baseline.") {
        return "Baseline";
    }
    if first.contains(".layout_diff.candidate.") {
        return "Candidate";
    }
    if first.contains(".layout_diff.swap") || first.contains(".layout_diff.toggle_changed_only") {
        return "Compare";
    }
    if first.contains(".layout_diff.filter.") {
        return "Filter";
    }
    if first.contains(".layout_diff.page_size.") {
        return "Page size";
    }
    if first.contains(".layout_diff.prev_page") || first.contains(".layout_diff.next_page") {
        return "Changes";
    }
    if first.contains(".layout_diff.review.") {
        return "Review";
    }
    if first.contains(".layout_diff.demo_current")
        || first.contains(".layout_diff.empty_current")
        || first.contains(".layout_diff.reset_filters")
    {
        return "Actions";
    }
    if first.contains(".maintenance.history.") {
        return "History";
    }
    if first.contains(".maintenance.tool.") {
        return "Tool";
    }
    if first.contains(".maintenance.status.") {
        return "Actions";
    }
    if first.contains(".environment.sensor.") {
        return "Sensor";
    }
    if first.contains(".environment.zone.") {
        return "Zone";
    }
    if first.contains(".inventory.lot.") {
        return "Lot";
    }
    if first.contains(".scheduler.policy.") {
        return "Policy";
    }
    if first.contains(".scheduler.priority.") {
        return "Priority";
    }
    if first.contains(".scheduler.toggle_") {
        return "Scope";
    }
    if first.contains(".scheduler.tool.") {
        return "Tool";
    }
    if first.contains(".safety.tool.") {
        return "Tool";
    }
    if first.contains(".safety.ack.condition.") {
        return "Interlock";
    }
    if first.contains(".safety.ack.lockout.") {
        return "Lockout";
    }
    if first.contains(".safety.ack.incident.") {
        return "Incident";
    }
    if first.contains(".trace.lot.") {
        return "Lot";
    }
    if first.contains(".trace.wafer.") {
        return "Wafer";
    }
    if first.contains(".trace.toggle_related") {
        return "Scope";
    }
    if first.contains(".trace.impact.") {
        return "Impact";
    }
    if first.contains(".trace.detail.process.") {
        return "Process";
    }
    if first.contains(".trace.detail.material.") {
        return "Material";
    }
    if first.contains(".trace.detail.event.") {
        return "Event";
    }
    if first.contains(".process_flow.toggle_errors") {
        return "Review";
    }
    if first.contains(".process_flow.export") {
        return "Export";
    }
    if first.contains(".process_flow.node.") {
        return "Node";
    }
    if first.contains(".process_control.filter.") {
        return "Filter";
    }
    if first.contains(".process_control.loop.") {
        return "Loop";
    }
    if first.contains(".process_control.action.") {
        return "Action";
    }
    if first.contains(".process_control.transition.") {
        return "State";
    }
    if first.contains(".spc.severity.") {
        return "Severity";
    }
    if first.contains(".spc.source.") {
        return "Source";
    }
    if first.contains(".spc.chart.") {
        return "Chart";
    }
    if first.contains(".spc.trace.") {
        return "Trace";
    }
    if first.contains(".spc.clear_context") {
        return "Actions";
    }
    if first.contains(".metrology.mode.") {
        return "Mode";
    }
    if first.contains(".metrology.kind.") {
        return "Measure";
    }
    if first.contains(".metrology.failed_only")
        || first.contains(".metrology.next_attention")
        || first.contains(".metrology.clear_die")
    {
        return "Review";
    }
    if first.contains(".yield.attention") {
        return "Focus";
    }
    if first.contains(".yield.lot.") {
        return "Lot";
    }
    if first.contains(".yield.wafer.") {
        return "Wafer";
    }
    if first.contains(".cross_section.step.") {
        return "Step";
    }
    if first.contains(".cross_section.toggle_") {
        return "Display";
    }
    if first.contains(".cross_section.material.") {
        return "Material";
    }
    if first.contains(".notebook.preview") {
        return "Mode";
    }
    if first.contains(".notebook.entry_action.") {
        return "Add";
    }
    if first.contains(".notebook.tag.") {
        return "Tag";
    }
    if first.contains(".notebook.link.") {
        return "Link";
    }
    if first.contains(".notebook.focus_link.") {
        return "Focus";
    }
    if first.contains(".notebook.entry.") {
        return "Entry";
    }
    if first.contains(".experiment.pending_only") {
        return "Queue";
    }
    if first.contains(".experiment.use_demo") {
        return "Capture";
    }
    if first.contains(".experiment.response.") {
        return "Response";
    }
    if first.contains(".experiment.filter.") {
        return "Filter";
    }
    if first.contains(".experiment.lot.") {
        return "Lot";
    }
    if first.contains(".experiment.run.") {
        return "Run";
    }
    if first.contains(".layout.view.") {
        return "View";
    }
    if first.contains(".layout.toggle_grid") || first.contains(".layout.toggle_snap") {
        return "Viewport";
    }
    if first.contains(".layout.hierarchy.") {
        return "Hierarchy";
    }
    if first.contains(".layout.hierarchy_step.") {
        return "Depth";
    }
    if first.contains(".layout.top_cell.") {
        return "Top cell";
    }
    if first.contains(".layout.tree_cell.") {
        return "Tree";
    }
    if first.contains(".layout.tree_toggle.") {
        return "Tree";
    }
    if first.contains(".layout.cell_visibility.") {
        return "Cell";
    }
    if first.contains(".layout.net_component.") {
        return "Net";
    }
    if first.contains(".layout.toggle_drc") || first.contains(".layout.run_drc") {
        return "Review";
    }
    if first.contains(".layout.add_layer") || first.contains(".layout.layer.") {
        return "Layer";
    }
    if first.contains(".layout.toggle_layer.") {
        return "Visibility";
    }
    if first.contains(".layout.shape.") {
        return "Shape";
    }
    if first.contains(".layout.occurrence.") {
        return "Shape";
    }
    if first.contains(".layout.measurement.") {
        return "Measurement";
    }
    if first.contains(".layout.instance.") {
        return "Instance";
    }
    if first.contains(".layout.copy")
        || first.contains(".layout.paste")
        || first.contains(".layout.duplicate")
        || first.contains(".layout.delete")
    {
        return "Edit";
    }
    if first.contains(".layout.clear_selection") || first.contains(".layout.connectivity") {
        return "Selection";
    }
    if first.contains(".select.") {
        return "Select";
    }
    if first.contains(".recipe.") {
        return "Recipe";
    }
    if first.contains(".command.") {
        return "Command";
    }
    if first.contains(".filter.") {
        return "Filter";
    }
    if first.contains(".mode.") || first.contains(".kind.") {
        return "Mode";
    }
    if first.contains(".page") {
        return "Page";
    }
    if first.contains(".ack") || first.contains(".clear") || first.contains(".reset") {
        return "Actions";
    }
    match (panel_name, row_index) {
        ("glassworks.viewctl.workflow", _) => "Open",
        ("glassworks.viewctl.fab", 0) => "Tool",
        ("glassworks.viewctl.maintenance", 0) => "Tool",
        ("glassworks.viewctl.environment", 0) => "Zone",
        ("glassworks.viewctl.inventory", 0) => "Lot",
        ("glassworks.viewctl.scheduler", 0) => "Tool",
        ("glassworks.viewctl.safety", 0) => "Tool",
        ("glassworks.viewctl.trace", 0) => "Lot",
        ("glassworks.viewctl.process_flow", 0) => "Node",
        ("glassworks.viewctl.process_control", 0) => "Loop",
        ("glassworks.viewctl.spc", 0) => "Chart",
        ("glassworks.viewctl.cross_section", 0) => "Step",
        ("glassworks.viewctl.metrology", 0) => "Map",
        ("glassworks.viewctl.yield", 0) => "Lot",
        ("glassworks.viewctl.experiment", 0) => "Response",
        ("glassworks.viewctl.notebook", 0) => "Entry",
        _ => "Controls",
    }
}

pub(crate) fn add_tool_strip(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport_width: f32,
    ui_scale: UiScale,
) {
    let drawer_mode = viewport_width < ui_scale.value(980.0);
    let compact_editing_tools = viewport_width < ui_scale.value(LAYOUT_COMPACT_TOOL_STRIP_WIDTH)
        && app.active_view == StartupView::Layout2d;
    let strip_height = tool_strip_height(app.active_view, viewport_width, ui_scale);
    let strip = document.add_child(
        parent,
        UiNode::container(
            "glassworks.tool_strip",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(strip_height),
                    ),
                    ui_scale.value(4.0),
                ),
                ui_scale.value(4.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );

    let first_row = document.add_child(
        strip,
        UiNode::container(
            "glassworks.tool_strip.row.0",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale.value(4.0),
            ),
        ),
    );
    let second_row = compact_editing_tools.then(|| {
        document.add_child(
            strip,
            UiNode::container(
                "glassworks.tool_strip.row.1",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(26.0)),
                    ),
                    ui_scale.value(4.0),
                ),
            ),
        )
    });

    if matches!(
        app.active_view,
        StartupView::Layout2d | StartupView::Layout3d
    ) {
        add_button(
            document,
            first_row,
            "glassworks.toolbar.view.layout2d",
            "2D",
            app.active_view == StartupView::Layout2d,
            layout::size(
                layout::px(ui_scale.value(52.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            first_row,
            "glassworks.toolbar.view.layout3d",
            "3D",
            app.active_view == StartupView::Layout3d,
            layout::size(
                layout::px(ui_scale.value(52.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }

    if app.active_view == StartupView::Layout2d {
        if compact_editing_tools {
            for tool in [ToolMode::Select, ToolMode::Rect, ToolMode::Polygon] {
                add_button(
                    document,
                    first_row,
                    format!("glassworks.tool.{}", tool.slug()),
                    tool.label(),
                    app.active_tool == tool,
                    layout::size(
                        layout::px(ui_scale.value(72.0)),
                        layout::px(ui_scale.value(26.0)),
                    ),
                    ui_scale,
                );
            }
            let second_row = second_row.expect("compact layout editor should create a second row");
            for tool in [
                ToolMode::Path,
                ToolMode::Via,
                ToolMode::Label,
                ToolMode::Measure,
                ToolMode::Route,
                ToolMode::Trace,
            ] {
                let width = if tool == ToolMode::Measure {
                    82.0
                } else {
                    72.0
                };
                add_button(
                    document,
                    second_row,
                    format!("glassworks.tool.{}", tool.slug()),
                    tool.label(),
                    app.active_tool == tool,
                    layout::size(
                        layout::px(ui_scale.value(width)),
                        layout::px(ui_scale.value(26.0)),
                    ),
                    ui_scale,
                );
            }
            add_button(
                document,
                second_row,
                "glassworks.toolbar.edit.delete",
                "Delete",
                false,
                layout::size(
                    layout::px(ui_scale.value(82.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
            if app.active_menu != Some(AppMenu::Edit) {
                add_node_marker(document, second_row, "glassworks.menu.item.edit.delete");
            }
        } else {
            for tool in ToolMode::ALL {
                add_button(
                    document,
                    first_row,
                    format!("glassworks.tool.{}", tool.slug()),
                    tool.label(),
                    app.active_tool == tool,
                    layout::size(
                        layout::px(ui_scale.value(82.0)),
                        layout::px(ui_scale.value(26.0)),
                    ),
                    ui_scale,
                );
            }
            add_button(
                document,
                first_row,
                "glassworks.toolbar.edit.delete",
                "Delete",
                false,
                layout::size(
                    layout::px(ui_scale.value(82.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
            if app.active_menu != Some(AppMenu::Edit) {
                add_node_marker(document, first_row, "glassworks.menu.item.edit.delete");
            }
        }
    } else if app.active_view == StartupView::Layout3d {
        add_button(
            document,
            first_row,
            "glassworks.toolbar.tools.reset3d",
            "Reset 3D",
            false,
            layout::size(
                layout::px(ui_scale.value(96.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            first_row,
            "glassworks.toolbar.tools.fullscreen",
            "Fullscreen",
            false,
            layout::size(
                layout::px(ui_scale.value(108.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        if app.active_menu != Some(AppMenu::Tools) {
            add_node_marker(document, first_row, "glassworks.menu.item.tools.fullscreen");
        }
    }

    if drawer_mode && app.has_inspector_panel() {
        add_button(
            document,
            first_row,
            "glassworks.drawer.inspector",
            "Details",
            app.show_inspector,
            layout::size(
                layout::px(ui_scale.value(92.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    if drawer_mode && app.has_secondary_panel() {
        add_button(
            document,
            first_row,
            "glassworks.drawer.layers",
            app.secondary_panel_label(),
            app.show_layers,
            layout::size(
                layout::px(ui_scale.value(96.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
}

pub(crate) fn add_tool_strip_spacer(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    viewport_width: f32,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let height = if viewport_width < ui_scale.value(LAYOUT_COMPACT_TOOL_STRIP_WIDTH) {
        ui_scale.value(64.0)
    } else {
        ui_scale.value(34.0)
    };
    document.add_child(
        parent,
        UiNode::container(
            "glassworks.tool_strip_spacer",
            layout::size(layout::percent(1.0), layout::px(height)),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    )
}

pub(crate) fn tool_strip_height(
    active_view: StartupView,
    viewport_width: f32,
    ui_scale: UiScale,
) -> f32 {
    if viewport_width < ui_scale.value(LAYOUT_COMPACT_TOOL_STRIP_WIDTH)
        && active_view == StartupView::Layout2d
    {
        ui_scale.value(64.0)
    } else {
        ui_scale.value(34.0)
    }
}
