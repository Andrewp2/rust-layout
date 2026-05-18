#![allow(unused_imports)]
use super::*;

pub(crate) fn process_control_view_primitives(
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) -> Vec<ScenePrimitive> {
    let Some(loop_definition) = selected_control_loop(app) else {
        return Vec::new();
    };
    let trend = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id);
    let actions = app
        .workspace
        .process_control
        .actions_for_loop(&loop_definition.id);
    let selected_action = selected_control_action(app)
        .filter(|action| action.loop_id == loop_definition.id)
        .or_else(|| actions.first().copied());
    let mut primitives = Vec::with_capacity(trend.len() * 4 + actions.len() * 4 + 80);
    let frame = if compact_rows {
        UiRect::new(
            ui_scale.value(30.0),
            ui_scale.value(18.0),
            ui_scale.value(250.0),
            ui_scale.value(168.0),
        )
    } else {
        UiRect::new(
            ui_scale.value(30.0),
            ui_scale.value(18.0),
            ui_scale.value(900.0),
            ui_scale.value(202.0),
        )
    };
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let trend_bounds = if compact_rows {
        UiRect::new(
            frame.x + ui_scale.value(16.0),
            frame.y + ui_scale.value(20.0),
            frame.width - ui_scale.value(32.0),
            frame.height - ui_scale.value(36.0),
        )
    } else {
        UiRect::new(
            frame.x + ui_scale.value(22.0),
            frame.y + ui_scale.value(24.0),
            ui_scale.value(430.0),
            ui_scale.value(154.0),
        )
    };
    add_process_control_trend_primitives(
        &mut primitives,
        trend_bounds,
        loop_definition,
        trend,
        ui_scale,
    );
    if compact_rows {
        return primitives;
    }

    let action_bounds = UiRect::new(
        frame.x + ui_scale.value(492.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(172.0),
        ui_scale.value(154.0),
    );
    let adjustment_bounds = UiRect::new(
        frame.x + ui_scale.value(704.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(174.0),
        ui_scale.value(154.0),
    );
    add_process_control_action_primitives(
        &mut primitives,
        action_bounds,
        &actions,
        selected_action,
        ui_scale,
    );
    add_process_control_adjustment_primitives(
        &mut primitives,
        adjustment_bounds,
        selected_action,
        ui_scale,
    );
    primitives
}

pub(crate) fn add_process_control_trend_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, bounds, ui_scale);
    if trend.len() < 2 {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }

    let (min_x, max_x) = trend
        .first()
        .zip(trend.last())
        .map(|(first, last)| (first.source.run_index as f32, last.source.run_index as f32))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for point in trend {
        add_spc_plot_value(&mut min_value, &mut max_value, point.value);
        add_spc_plot_value(&mut min_value, &mut max_value, point.target);
        add_spc_plot_value(
            &mut min_value,
            &mut max_value,
            point.target + point.ewma_error,
        );
    }
    let deadband = loop_definition.deadband.abs();
    add_spc_plot_value(
        &mut min_value,
        &mut max_value,
        loop_definition.output.target - deadband,
    );
    add_spc_plot_value(
        &mut min_value,
        &mut max_value,
        loop_definition.output.target + deadband,
    );
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    let band_top = spc_plot_y(
        bounds,
        min_value,
        max_value,
        loop_definition.output.target + deadband,
    );
    let band_bottom = spc_plot_y(
        bounds,
        min_value,
        max_value,
        loop_definition.output.target - deadband,
    );
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            bounds.x,
            band_top.min(band_bottom),
            bounds.width,
            (band_bottom - band_top).abs().max(ui_scale.value(1.0)),
        ),
        ColorRgba::new(80, 166, 114, 48),
    )));
    add_spc_plot_guides(
        primitives,
        bounds,
        min_value,
        max_value,
        &[SpcPlotGuide {
            value: loop_definition.output.target,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.1),
        }],
    );

    for pair in trend.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].source.run_index as f32),
                spc_plot_y(bounds, min_value, max_value, pair[0].value),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].source.run_index as f32),
                spc_plot_y(bounds, min_value, max_value, pair[1].value),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(93, 168, 232, 235), ui_scale.value(2.0)),
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].source.run_index as f32),
                spc_plot_y(
                    bounds,
                    min_value,
                    max_value,
                    pair[0].target + pair[0].ewma_error,
                ),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].source.run_index as f32),
                spc_plot_y(
                    bounds,
                    min_value,
                    max_value,
                    pair[1].target + pair[1].ewma_error,
                ),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(238, 181, 82, 210), ui_scale.value(1.3)),
        });
    }

    for point in trend {
        let actionable = point.actionable(loop_definition);
        let center = UiPoint::new(
            spc_plot_x(bounds, min_x, max_x, point.source.run_index as f32),
            spc_plot_y(bounds, min_value, max_value, point.value),
        );
        if actionable {
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(center.x, bounds.y),
                to: UiPoint::new(center.x, bounds.bottom()),
                stroke: StrokeStyle::new(ColorRgba::new(236, 91, 88, 74), ui_scale.value(1.0)),
            });
        }
        primitives.push(ScenePrimitive::Circle {
            center,
            radius: ui_scale.value(if actionable { 4.4 } else { 3.0 }),
            fill: if actionable {
                ColorRgba::new(244, 112, 104, 245)
            } else if point.in_spec {
                ColorRgba::new(105, 201, 135, 245)
            } else {
                ColorRgba::new(238, 181, 82, 245)
            },
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(1.0),
            )),
        });
    }
}

pub(crate) fn add_process_control_action_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    actions: &[&ControlAction],
    selected_action: Option<&ControlAction>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if actions.is_empty() {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }
    let visible = actions.len().min(7);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, action) in actions.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        let selected = selected_action.is_some_and(|selected| selected.id == action.id);
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                row,
                if selected {
                    ColorRgba::new(72, 128, 188, 72)
                } else {
                    ColorRgba::new(25, 32, 39, 255)
                },
            )
            .stroke(StrokeStyle::new(
                if selected {
                    ColorRgba::new(252, 253, 255, 210)
                } else {
                    ColorRgba::new(41, 52, 63, 180)
                },
                ui_scale.value(if selected { 1.2 } else { 0.7 }),
            )),
        ));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                row.x,
                row.y,
                row.width * action.confidence.clamp(0.0, 1.0) as f32,
                row.height,
            ),
            process_control_action_state_color(action.state, 170),
        )));
        let center_y = row.y + row.height * 0.5;
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(row.x + ui_scale.value(8.0), center_y),
            radius: ui_scale.value(3.6),
            fill: process_control_action_state_color(action.state, 245),
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(0.8),
            )),
        });
    }
}

pub(crate) fn add_process_control_adjustment_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    action: Option<&ControlAction>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let Some(action) = action else {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    };
    if action.adjustments.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 145),
        )));
        return;
    }

    let visible = action.adjustments.len().min(6);
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    let max_delta = action
        .adjustments
        .iter()
        .map(|adjustment| adjustment.delta.abs())
        .fold(0.0_f64, f64::max)
        .max(0.001);
    let center_x = bounds.x + bounds.width * 0.5;
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(center_x, bounds.y),
        to: UiPoint::new(center_x, bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(130, 145, 160, 145), ui_scale.value(1.0)),
    });
    for (index, adjustment) in action.adjustments.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = (adjustment.delta.abs() / max_delta) as f32
            * (bounds.width * 0.46 - ui_scale.value(2.0));
        let bar_height = row_height * 0.68;
        let x = if adjustment.delta >= 0.0 {
            center_x
        } else {
            center_x - width
        };
        let color = if adjustment.delta.abs() < f64::EPSILON {
            ColorRgba::new(91, 190, 130, 180)
        } else if adjustment.delta > 0.0 {
            ColorRgba::new(238, 181, 82, 220)
        } else {
            ColorRgba::new(102, 190, 236, 220)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                x,
                y + (row_height - bar_height) * 0.5,
                width.max(ui_scale.value(2.0)),
                bar_height,
            ),
            color,
        )));

        let range_span = (adjustment.upper_bound - adjustment.lower_bound)
            .abs()
            .max(f64::EPSILON);
        let range_fraction =
            ((adjustment.delta - adjustment.lower_bound) / range_span).clamp(0.0, 1.0) as f32;
        let marker_x = bounds.x + bounds.width * range_fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(marker_x, y),
            to: UiPoint::new(marker_x, y + row_height),
            stroke: StrokeStyle::new(ColorRgba::new(252, 253, 255, 120), ui_scale.value(0.8)),
        });
    }
}

pub(crate) fn process_control_action_state_color(
    state: ControlActionState,
    alpha: u8,
) -> ColorRgba {
    match state {
        ControlActionState::Proposed => ColorRgba::new(93, 168, 232, alpha),
        ControlActionState::Approved => ColorRgba::new(91, 190, 130, alpha),
        ControlActionState::Rejected => ColorRgba::new(236, 91, 88, alpha),
        ControlActionState::Applied => ColorRgba::new(136, 207, 190, alpha),
        ControlActionState::Held => ColorRgba::new(238, 181, 82, alpha),
    }
}

pub(crate) fn process_control_action_label(action: &ControlAction, compact_rows: bool) -> String {
    if compact_rows {
        return format!(
            "{} {}",
            process_control_action_short_label(action),
            action.state.label()
        );
    }
    let adjustment_label = action
        .adjustments
        .first()
        .map(|adjustment| {
            let unit = adjustment.unit.map(|unit| unit.symbol()).unwrap_or("");
            format!("{} {:+.3}{unit}", adjustment.label, adjustment.delta)
        })
        .unwrap_or_else(|| "no recipe adjustment".to_string());
    compact_button_label(
        &format!(
            "{} {} error {:+.3}, confidence {:.0}%, {}",
            process_control_action_short_label(action),
            action.state.label(),
            action.error,
            action.confidence.clamp(0.0, 1.0) * 100.0,
            adjustment_label
        ),
        92,
    )
}

pub(crate) fn process_control_yield_link_label(
    app: &GlassworksApp,
    loop_definition: &ControlLoop,
    compact_rows: bool,
) -> String {
    let Some(point) = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id)
        .last()
    else {
        return "No linked yield sample".to_string();
    };
    let yield_label = app
        .workspace
        .yield_analysis
        .wafer_summary(&point.source.lot_id, &point.source.wafer_id)
        .map(|summary| percent_label(summary.yield_fraction))
        .or_else(|| point.yield_fraction.map(percent_label))
        .unwrap_or_else(|| "yield n/a".to_string());
    if compact_rows {
        return format!("yield {yield_label}");
    }
    compact_button_label(
        &format!(
            "{yield_label} yield for {} / {} / {}",
            workflow_lot_label(&app.workspace, &point.source.lot_id, 32),
            yield_wafer_label(&point.source.wafer_id),
            control_run_label(point.source.run_index)
        ),
        70,
    )
}
