#![allow(unused_imports)]
use super::*;

pub(crate) fn scheduler_timeline_primitives(
    app: &GlassworksApp,
    result: &layout_model::scheduler::DispatchResult,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let tools = app
        .workspace
        .scheduler
        .tools
        .iter()
        .take(8)
        .collect::<Vec<_>>();
    let mut primitives = Vec::with_capacity(tools.len() * 8 + result.assignments.len() * 4 + 20);
    let frame = UiRect::new(
        ui_scale.value(42.0),
        ui_scale.value(22.0),
        ui_scale.value(864.0),
        ui_scale.value(174.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    if tools.is_empty() {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(frame.x + ui_scale.value(24.0), frame.y + frame.height * 0.5),
            to: UiPoint::new(
                frame.right() - ui_scale.value(24.0),
                frame.y + frame.height * 0.5,
            ),
            stroke: StrokeStyle::new(ColorRgba::new(80, 94, 108, 255), ui_scale.value(1.0)),
        });
        return primitives;
    }

    let start = result
        .assignments
        .iter()
        .map(|assignment| assignment.start_minute)
        .min()
        .unwrap_or(app.workspace.scheduler.now_minute)
        .min(app.workspace.scheduler.now_minute);
    let end = result
        .assignments
        .iter()
        .map(|assignment| assignment.finish_minute)
        .max()
        .unwrap_or(start + 60)
        .max(start + 60);
    let label_width = ui_scale.value(92.0);
    let plot = UiRect::new(
        frame.x + label_width,
        frame.y + ui_scale.value(22.0),
        frame.width - label_width - ui_scale.value(10.0),
        frame.height - ui_scale.value(32.0),
    );

    for tick in scheduler_hourly_ticks(start, end) {
        let x = scheduler_time_x(plot, start, end, tick);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(116, 132, 148, 70), ui_scale.value(1.0)),
        });
    }
    if app.workspace.scheduler.now_minute >= start && app.workspace.scheduler.now_minute <= end {
        let x = scheduler_time_x(plot, start, end, app.workspace.scheduler.now_minute);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y - ui_scale.value(6.0)),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(88, 178, 232, 240), ui_scale.value(1.7)),
        });
    }

    let row_h = (plot.height / tools.len().max(1) as f32).max(ui_scale.value(20.0));
    for (index, tool) in tools.iter().enumerate() {
        let row_y = plot.y + index as f32 * row_h;
        let row = UiRect::new(
            plot.x,
            row_y + ui_scale.value(3.0),
            plot.width,
            row_h - ui_scale.value(6.0),
        );
        if app.selected_scheduler_tool.as_ref() == Some(&tool.id) {
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                row,
                ColorRgba::new(72, 128, 188, 58),
            )));
        }
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(
                    frame.x + ui_scale.value(12.0),
                    row.y + row.height * 0.5 - ui_scale.value(5.0),
                    ui_scale.value(64.0),
                    ui_scale.value(10.0),
                ),
                scheduler_tool_state_color(tool.state),
            )
            .stroke(StrokeStyle::new(
                ColorRgba::new(14, 18, 22, 170),
                ui_scale.value(1.0),
            )),
        ));
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, row.bottom()),
            to: UiPoint::new(plot.right(), row.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(42, 52, 62, 190), ui_scale.value(1.0)),
        });
        for window in &tool.maintenance_windows {
            if window.end_minute < start || window.start_minute > end {
                continue;
            }
            let x0 = scheduler_time_x(plot, start, end, window.start_minute.max(start));
            let x1 = scheduler_time_x(plot, start, end, window.end_minute.min(end));
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x0, row.y, (x1 - x0).max(ui_scale.value(2.0)), row.height),
                ColorRgba::new(224, 178, 72, 72),
            )));
        }
    }

    for assignment in &result.assignments {
        let Some(row_index) = tools.iter().position(|tool| tool.id == assignment.tool_id) else {
            continue;
        };
        let row_y = plot.y + row_index as f32 * row_h;
        let x0 = scheduler_time_x(plot, start, end, assignment.start_minute);
        let x1 = scheduler_time_x(plot, start, end, assignment.finish_minute);
        let bar = UiRect::new(
            x0,
            row_y + ui_scale.value(6.0),
            (x1 - x0).max(ui_scale.value(8.0)),
            (row_h - ui_scale.value(12.0)).max(ui_scale.value(8.0)),
        );
        if assignment.due_at_minute >= start && assignment.due_at_minute <= end {
            let due_x = scheduler_time_x(plot, start, end, assignment.due_at_minute);
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(due_x, bar.y - ui_scale.value(3.0)),
                to: UiPoint::new(due_x, bar.bottom() + ui_scale.value(3.0)),
                stroke: StrokeStyle::new(
                    if assignment.tardy_minutes > 0 {
                        ColorRgba::new(236, 91, 88, 245)
                    } else {
                        ColorRgba::new(178, 188, 198, 190)
                    },
                    ui_scale.value(1.4),
                ),
            });
        }
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(bar, scheduler_assignment_color(assignment.priority)).stroke(
                StrokeStyle::new(
                    if assignment.tardy_minutes > 0 {
                        ColorRgba::new(236, 91, 88, 255)
                    } else {
                        ColorRgba::new(16, 20, 24, 170)
                    },
                    ui_scale.value(if assignment.tardy_minutes > 0 {
                        2.0
                    } else {
                        1.0
                    }),
                ),
            ),
        ));
    }
    primitives
}

pub(crate) fn scheduler_time_x(plot: UiRect, start: u32, end: u32, minute: u32) -> f32 {
    let span = end.saturating_sub(start).max(1) as f32;
    plot.x + plot.width * minute.saturating_sub(start) as f32 / span
}

pub(crate) fn scheduler_hourly_ticks(start: u32, end: u32) -> Vec<u32> {
    let mut tick = start - start % 60;
    if tick < start {
        tick += 60;
    }
    let mut ticks = Vec::new();
    while tick <= end {
        ticks.push(tick);
        tick += 60;
    }
    ticks
}

pub(crate) fn scheduler_tool_state_color(
    state: layout_model::scheduler::ToolDispatchState,
) -> ColorRgba {
    match state {
        layout_model::scheduler::ToolDispatchState::Available => ColorRgba::new(66, 166, 118, 230),
        layout_model::scheduler::ToolDispatchState::Maintenance => {
            ColorRgba::new(224, 176, 72, 230)
        }
        layout_model::scheduler::ToolDispatchState::Down => ColorRgba::new(224, 84, 88, 230),
    }
}

pub(crate) fn scheduler_assignment_color(priority: u8) -> ColorRgba {
    match priority {
        0 | 1 => ColorRgba::new(86, 158, 216, 230),
        2 | 3 => ColorRgba::new(72, 174, 132, 230),
        _ => ColorRgba::new(230, 170, 76, 235),
    }
}
