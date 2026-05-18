#![allow(unused_imports)]
use super::*;

pub(crate) fn add_process_control_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let Some(loop_definition) = selected_control_loop(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let trend = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id);
    let actions = app
        .workspace
        .process_control
        .actions_for_loop(&loop_definition.id);
    let selected_action =
        selected_control_action(app).filter(|action| action.loop_id == loop_definition.id);
    let latest = trend.last();
    let audit_count = app
        .workspace
        .process_control
        .audit_for_loop(&loop_definition.id)
        .len();
    let latest_label = process_control_latest_label(latest, compact_rows);
    let target_label = process_control_target_label(loop_definition, compact_rows);
    let actions_label = format!("{} actions / {audit_count} audits", actions.len());
    let detail_items = [
        selected_action
            .map(|action| process_control_action_label(action, compact_rows))
            .unwrap_or_else(|| "No selected recommendation".to_string()),
        process_control_yield_link_label(app, loop_definition, compact_rows),
    ];

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.process_control.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(if compact_rows { 500.0 } else { 414.0 })),
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
        "glassworks.process_control.primary.title",
        if compact_rows {
            format!(
                "{} - {}",
                process_control_loop_button_label(&loop_definition.name),
                equipment_tool_label_for_raw_id(&app.workspace, &loop_definition.tool_id)
            )
        } else {
            format!(
                "{} - {} on {}",
                loop_definition.name,
                display_measurement_identifier(&loop_definition.output.measurement_name),
                equipment_tool_label_for_raw_id(&app.workspace, &loop_definition.tool_id)
            )
        },
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(
            layout::percent(1.0),
            layout::px(ui_scale.value(if compact_rows { 28.0 } else { 24.0 })),
        ),
    );
    document.add_child(
        panel,
        UiNode::scene(
            "glassworks.process_control.trend",
            process_control_view_primitives(app, ui_scale, compact_rows),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(if compact_rows { 204.0 } else { 238.0 })),
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

    let metric_items = [latest_label, target_label, actions_label];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.process_control.metrics",
            &metric_items,
            ui_scale,
        );
    } else {
        let metrics = document.add_child(
            panel,
            UiNode::container(
                "glassworks.process_control.metrics",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(42.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.process_control.metrics.latest",
            metric_items[0].clone(),
            1.5,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.process_control.metrics.target",
            metric_items[1].clone(),
            1.2,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.process_control.metrics.actions",
            metric_items[2].clone(),
            1.0,
            false,
            ui_scale,
        );
    }

    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.process_control.detail",
            &detail_items,
            ui_scale,
        );
    } else {
        let detail = document.add_child(
            panel,
            UiNode::container(
                "glassworks.process_control.detail",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(42.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            detail,
            "glassworks.process_control.detail.action",
            detail_items[0].clone(),
            1.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            detail,
            "glassworks.process_control.detail.yield",
            detail_items[1].clone(),
            1.4,
            false,
            ui_scale,
        );
    }

    let loops = document.add_child(
        panel,
        UiNode::container(
            "glassworks.process_control.loops",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    let filtered_loops = filtered_control_loops(app);
    for (index, loop_definition) in filtered_loops.iter().take(5).enumerate() {
        add_button(
            document,
            loops,
            format!(
                "glassworks.primary.action.process_control.{index}.process_control.loop.{}",
                loop_definition.id
            ),
            process_control_loop_button_label(&loop_definition.name),
            app.selected_control_loop.as_ref() == Some(&loop_definition.id),
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}
