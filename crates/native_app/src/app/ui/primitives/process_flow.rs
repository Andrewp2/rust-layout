#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ProcessFlowNodePlacement {
    pub(crate) id: ProcessFlowNodeId,
    pub(crate) center: UiPoint,
    pub(crate) rect: UiRect,
}

pub(crate) fn process_flow_timeline_primitives(
    app: &GlassworksApp,
    findings: &[layout_model::process_flow::ProcessFlowFinding],
    ui_scale: UiScale,
    compact_rows: bool,
    body_width: f32,
) -> Vec<ScenePrimitive> {
    let nodes = process_flow_visible_nodes(app, findings);
    let mut primitives =
        Vec::with_capacity(nodes.len() * 4 + app.workspace.process_flow.route.edges.len());
    let track_x = ui_scale.value(if compact_rows { 18.0 } else { 42.0 });
    let track_width = if compact_rows {
        (body_width - ui_scale.value(76.0)).clamp(ui_scale.value(180.0), ui_scale.value(860.0))
    } else {
        ui_scale.value(860.0)
    };
    let track = UiRect::new(
        track_x,
        ui_scale.value(26.0),
        track_width,
        ui_scale.value(178.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(track, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    if nodes.is_empty() {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(track.x + ui_scale.value(24.0), track.y + track.height * 0.5),
            to: UiPoint::new(
                track.right() - ui_scale.value(24.0),
                track.y + track.height * 0.5,
            ),
            stroke: StrokeStyle::new(ColorRgba::new(80, 94, 108, 255), ui_scale.value(1.0)),
        });
        return primitives;
    }

    let placements = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let fraction = if nodes.len() == 1 {
                0.5
            } else {
                index as f32 / (nodes.len() - 1) as f32
            };
            let center_x =
                track.x + ui_scale.value(48.0) + (track.width - ui_scale.value(96.0)) * fraction;
            let center_y = match node.kind {
                ProcessFlowNodeKind::Measurement => track.y + ui_scale.value(58.0),
                ProcessFlowNodeKind::Hold => track.y + ui_scale.value(132.0),
                ProcessFlowNodeKind::Start
                | ProcessFlowNodeKind::End
                | ProcessFlowNodeKind::Operation => track.y + ui_scale.value(96.0),
            };
            let rect_width = (track.width / nodes.len() as f32 * 0.62)
                .clamp(ui_scale.value(42.0), ui_scale.value(88.0));
            let rect_height = ui_scale.value(42.0);
            ProcessFlowNodePlacement {
                id: node.id.clone(),
                center: UiPoint::new(center_x, center_y),
                rect: UiRect::new(
                    center_x - rect_width * 0.5,
                    center_y - rect_height * 0.5,
                    rect_width,
                    rect_height,
                ),
            }
        })
        .collect::<Vec<_>>();
    let placement_by_id = placements
        .iter()
        .map(|placement| (placement.id.clone(), placement))
        .collect::<BTreeMap<_, _>>();

    for edge in &app.workspace.process_flow.route.edges {
        let (Some(from), Some(to)) = (
            placement_by_id.get(&edge.from),
            placement_by_id.get(&edge.to),
        ) else {
            continue;
        };
        add_process_flow_edge_primitives(
            edge.kind,
            from.center,
            to.center,
            ui_scale,
            &mut primitives,
        );
    }

    for (node, placement) in nodes.iter().zip(placements.iter()) {
        let selected = app.selected_process_node.as_ref() == Some(&node.id);
        let issue_count = process_flow_node_findings(&node.id, findings).len();
        let rework_count = app
            .workspace
            .process_flow
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == layout_model::process_flow::ProcessFlowEdgeKind::Rework)
            .filter(|edge| edge.from == node.id || edge.to == node.id)
            .count();
        let stroke_color = if selected {
            ColorRgba::new(250, 219, 112, 255)
        } else if issue_count > 0 {
            ColorRgba::new(236, 91, 88, 255)
        } else {
            ColorRgba::new(24, 29, 34, 180)
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(placement.rect, process_flow_node_color(node.kind, 220))
                .stroke(StrokeStyle::new(
                    stroke_color,
                    ui_scale.value(if selected { 2.2 } else { 1.0 }),
                )),
        ));
        let mut badge_x = placement.rect.right() - ui_scale.value(8.0);
        if node.measurement_checkpoint.is_some() {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                ColorRgba::new(78, 188, 135, 245),
                ui_scale,
            ));
            badge_x -= ui_scale.value(12.0);
        }
        if node.hold_point || node.kind == ProcessFlowNodeKind::Hold {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                ColorRgba::new(238, 181, 82, 245),
                ui_scale,
            ));
            badge_x -= ui_scale.value(12.0);
        }
        if rework_count > 0 || issue_count > 0 {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                if issue_count > 0 {
                    ColorRgba::new(236, 91, 88, 245)
                } else {
                    ColorRgba::new(206, 111, 220, 245)
                },
                ui_scale,
            ));
        }
    }

    primitives
}

pub(crate) fn process_flow_visible_nodes<'a>(
    app: &'a GlassworksApp,
    findings: &[layout_model::process_flow::ProcessFlowFinding],
) -> Vec<&'a layout_model::process_flow::ProcessFlowNode> {
    let error_nodes = findings
        .iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .filter_map(|finding| finding.node_id.clone())
        .collect::<BTreeSet<_>>();
    app.workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .filter(|node| !app.process_flow_errors_only || error_nodes.contains(&node.id))
        .take(14)
        .collect()
}

pub(crate) fn add_process_flow_edge_primitives(
    kind: layout_model::process_flow::ProcessFlowEdgeKind,
    from: UiPoint,
    to: UiPoint,
    ui_scale: UiScale,
    primitives: &mut Vec<ScenePrimitive>,
) {
    let color = process_flow_edge_color(kind);
    let stroke = StrokeStyle::new(color, ui_scale.value(1.5));
    match kind {
        layout_model::process_flow::ProcessFlowEdgeKind::Rework => {
            let y = from.y.min(to.y) - ui_scale.value(34.0);
            primitives.push(ScenePrimitive::Line {
                from,
                to: UiPoint::new(from.x, y),
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(from.x, y),
                to: UiPoint::new(to.x, y),
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(to.x, y),
                to,
                stroke,
            });
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Branch => {
            let mid = UiPoint::new(
                (from.x + to.x) * 0.5,
                (from.y + to.y) * 0.5 + ui_scale.value(20.0),
            );
            primitives.push(ScenePrimitive::Line {
                from,
                to: mid,
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: mid,
                to,
                stroke,
            });
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Sequence => {
            primitives.push(ScenePrimitive::Line { from, to, stroke });
        }
    }
}

pub(crate) fn process_flow_badge(
    x: f32,
    y: f32,
    fill: ColorRgba,
    ui_scale: UiScale,
) -> ScenePrimitive {
    ScenePrimitive::Circle {
        center: UiPoint::new(x, y),
        radius: ui_scale.value(4.4),
        fill,
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(16, 20, 24, 185),
            ui_scale.value(1.0),
        )),
    }
}

pub(crate) fn process_flow_node_findings<'a>(
    node_id: &ProcessFlowNodeId,
    findings: &'a [layout_model::process_flow::ProcessFlowFinding],
) -> Vec<&'a layout_model::process_flow::ProcessFlowFinding> {
    findings
        .iter()
        .filter(|finding| finding.node_id.as_ref() == Some(node_id))
        .collect()
}

pub(crate) fn process_flow_node_color(kind: ProcessFlowNodeKind, alpha: u8) -> ColorRgba {
    match kind {
        ProcessFlowNodeKind::Start | ProcessFlowNodeKind::End => {
            ColorRgba::new(95, 108, 120, alpha)
        }
        ProcessFlowNodeKind::Operation => ColorRgba::new(62, 132, 196, alpha),
        ProcessFlowNodeKind::Measurement => ColorRgba::new(54, 162, 122, alpha),
        ProcessFlowNodeKind::Hold => ColorRgba::new(208, 153, 69, alpha),
    }
}

pub(crate) fn process_flow_edge_color(
    kind: layout_model::process_flow::ProcessFlowEdgeKind,
) -> ColorRgba {
    match kind {
        layout_model::process_flow::ProcessFlowEdgeKind::Sequence => {
            ColorRgba::new(96, 160, 220, 210)
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Branch => {
            ColorRgba::new(232, 176, 78, 220)
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Rework => {
            ColorRgba::new(220, 105, 198, 220)
        }
    }
}
