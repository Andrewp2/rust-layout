#![allow(unused_imports)]
use super::*;

pub(crate) fn add_scheduler_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let result = app.workspace.scheduler.dispatch(app.scheduler_policy);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.scheduler.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(352.0)),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 29, 36, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(55, 69, 83, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.scheduler.primary.title",
        format!(
            "{} dispatch - {} waiting lots / {} tools",
            display_dispatch_policy_label(app.scheduler_policy),
            app.workspace.scheduler.lots.len(),
            app.workspace.scheduler.tools.len()
        ),
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    document.add_child(
        panel,
        UiNode::scene(
            "glassworks.scheduler.timeline",
            scheduler_timeline_primitives(app, &result, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(226.0)),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 24, 30, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(50, 64, 78, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );

    let queue = result
        .queue_summaries
        .iter()
        .max_by_key(|summary| summary.total_process_minutes);
    let summary = document.add_child(
        panel,
        UiNode::container(
            "glassworks.scheduler.summary",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(54.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        summary,
        "glassworks.scheduler.summary.assignments",
        format!("{} assignments", result.assignments.len()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "glassworks.scheduler.summary.unscheduled",
        format!("{} unscheduled", result.unscheduled_lots.len()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "glassworks.scheduler.summary.queue",
        queue
            .map(|summary| {
                format!(
                    "{} bottleneck: {} lots / {} min",
                    summary.tool_class.label(),
                    summary.waiting_lots,
                    summary.total_process_minutes
                )
            })
            .unwrap_or_else(|| "No queued bottleneck".to_string()),
        1.6,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "glassworks.scheduler.summary.selected",
        app.selected_scheduler_tool
            .as_ref()
            .map(|tool| format!("Selected {tool}"))
            .unwrap_or_else(|| "Selected none".to_string()),
        1.0,
        false,
        ui_scale,
    );
}
