#![allow(unused_imports)]
use super::*;

pub(crate) fn add_process_flow_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
    body_width: f32,
) {
    let route = &app.workspace.process_flow.route;
    let findings = app.workspace.process_flow.findings();
    let selected = selected_process_flow_node(app);
    let timeline_height = if compact_rows { 216.0 } else { 232.0 };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.process_flow.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(350.0)),
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
        "glassworks.process_flow.primary.title",
        if compact_rows {
            format!(
                "{} v{}",
                display_process_flow_route_name(&route.name),
                route.version
            )
        } else {
            format!(
                "{} v{} - {}",
                display_process_flow_route_name(&route.name),
                route.version,
                display_owner_identifier(&route.owner)
            )
        },
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
            "glassworks.process_flow.timeline",
            process_flow_timeline_primitives(app, &findings, ui_scale, compact_rows, body_width),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(timeline_height)),
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

    let selected_label = selected
        .map(display_process_flow_node_label)
        .unwrap_or_else(|| "none".to_string());
    let dependency_label = selected
        .map(|node| {
            let incoming = route.edges.iter().filter(|edge| edge.to == node.id).count();
            let outgoing = route
                .edges
                .iter()
                .filter(|edge| edge.from == node.id)
                .count();
            format!("{incoming} in / {outgoing} out")
        })
        .unwrap_or_else(|| "0 in / 0 out".to_string());
    let control_label = format!(
        "{} checkpoints / {} holds",
        route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count(),
        route
            .nodes
            .iter()
            .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
            .count()
    );
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.process_flow.summary",
            &[
                selected_label,
                dependency_label,
                control_label.replace("checkpoints", "checks"),
                format!("{} findings", findings.len()),
            ],
            ui_scale,
        );
    } else {
        let summary = document.add_child(
            panel,
            UiNode::container(
                "glassworks.process_flow.summary",
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
            "glassworks.process_flow.summary.selected",
            format!("Selected {selected_label}"),
            1.5,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.process_flow.summary.dependencies",
            dependency_label,
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.process_flow.summary.controls",
            control_label,
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.process_flow.summary.findings",
            format!("{} findings", findings.len()),
            0.7,
            false,
            ui_scale,
        );
    }
}
