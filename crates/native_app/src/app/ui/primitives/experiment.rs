#![allow(unused_imports)]
use super::*;

pub(crate) fn experiment_view_primitives(
    plan: &ExperimentPlan,
    analysis: &ExperimentAnalysisSummary,
    missing_only: bool,
    ui_scale: UiScale,
    compact_rows: bool,
    body_width: f32,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(plan.runs.len() * plan.factors.len() + 100);
    let frame_x = ui_scale.value(if compact_rows { 10.0 } else { 30.0 });
    let frame_y = ui_scale.value(if compact_rows { 10.0 } else { 18.0 });
    let frame_width = (body_width - frame_x - ui_scale.value(30.0))
        .max(ui_scale.value(220.0))
        .min(ui_scale.value(900.0));
    let frame_height = ui_scale.value(if compact_rows { 184.0 } else { 202.0 });
    let frame = UiRect::new(frame_x, frame_y, frame_width, frame_height);
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let compact_scene = compact_rows || frame.width < ui_scale.value(620.0);
    let matrix_bounds = if compact_scene {
        UiRect::new(
            frame.x + ui_scale.value(18.0),
            frame.y + ui_scale.value(20.0),
            frame.width - ui_scale.value(36.0),
            frame.height - ui_scale.value(40.0),
        )
    } else {
        let inset = ui_scale.value(22.0);
        let gap = ui_scale.value(40.0);
        let side_width = (frame.width * 0.2)
            .max(ui_scale.value(150.0))
            .min(ui_scale.value(184.0));
        let matrix_width =
            (frame.width - inset * 2.0 - gap * 2.0 - side_width * 2.0).max(ui_scale.value(220.0));
        UiRect::new(
            frame.x + inset,
            frame.y + ui_scale.value(20.0),
            matrix_width,
            ui_scale.value(162.0),
        )
    };
    add_experiment_matrix_primitives(&mut primitives, matrix_bounds, plan, missing_only, ui_scale);
    if compact_scene {
        return primitives;
    }

    let gap = ui_scale.value(40.0);
    let side_width = (frame.width * 0.2)
        .max(ui_scale.value(150.0))
        .min(ui_scale.value(184.0));
    let response_bounds = UiRect::new(
        matrix_bounds.right() + gap,
        frame.y + ui_scale.value(28.0),
        side_width,
        ui_scale.value(70.0),
    );
    let effect_bounds = UiRect::new(
        response_bounds.x,
        frame.y + ui_scale.value(126.0),
        side_width,
        ui_scale.value(56.0),
    );
    let status_bounds = UiRect::new(
        response_bounds.right() + gap,
        frame.y + ui_scale.value(28.0),
        side_width,
        ui_scale.value(154.0),
    );
    add_experiment_response_primitives(&mut primitives, response_bounds, plan, analysis, ui_scale);
    add_experiment_effect_primitives(&mut primitives, effect_bounds, analysis, ui_scale);
    add_experiment_status_primitives(&mut primitives, status_bounds, plan, ui_scale);
    primitives
}

pub(crate) fn add_experiment_matrix_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    missing_only: bool,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let response_count = plan.responses.len();
    let runs = plan
        .runs
        .iter()
        .filter(|run| !missing_only || response_count == 0 || run.responses.len() < response_count)
        .take(14)
        .collect::<Vec<_>>();
    if runs.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(3.0);
    let row_height =
        (bounds.height - gap * runs.len().saturating_sub(1) as f32) / runs.len().max(1) as f32;
    let columns = plan.factors.len().max(1) + 1;
    let cell_width = (bounds.width - gap * columns.saturating_sub(1) as f32) / columns as f32;
    for (row_index, run) in runs.iter().enumerate() {
        let y = bounds.y + row_index as f32 * (row_height + gap);
        for (column, factor) in plan.factors.iter().enumerate() {
            let x = bounds.x + column as f32 * (cell_width + gap);
            let fill = run
                .factor_levels
                .get(&factor.id)
                .and_then(|level_id| factor.levels.iter().position(|level| level.id == *level_id))
                .map(|index| experiment_level_color(index, 210))
                .unwrap_or(ColorRgba::new(45, 54, 62, 160));
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, y, cell_width, row_height),
                fill,
            )));
        }
        let x = bounds.x + (columns - 1) as f32 * (cell_width + gap);
        let progress = if response_count == 0 {
            0.0
        } else {
            run.responses.len() as f32 / response_count as f32
        };
        let cell = UiRect::new(x, y, cell_width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            cell,
            ColorRgba::new(28, 36, 44, 255),
        )));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(cell.x, cell.y, cell.width * progress, cell.height),
            experiment_run_status_color(run.status, 215),
        )));
    }
}

pub(crate) fn add_experiment_response_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    analysis: &ExperimentAnalysisSummary,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if analysis.response_stats.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = analysis.response_stats.len().min(5);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, stat) in analysis.response_stats.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let progress = if analysis.run_count == 0 {
            0.0
        } else {
            stat.sample_count as f32 / analysis.run_count as f32
        };
        let color = plan
            .response(&stat.response_id)
            .zip(stat.mean)
            .map(|(response, mean)| {
                experiment_response_status_color(response.value_status(mean), 215)
            })
            .unwrap_or(ColorRgba::new(93, 168, 232, 190));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(row.x, row.y, row.width * progress, row.height),
            color,
        )));
    }
}

pub(crate) fn add_experiment_effect_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    analysis: &ExperimentAnalysisSummary,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if analysis.factor_effects.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = analysis.factor_effects.len().min(5);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    let max_abs = analysis
        .factor_effects
        .iter()
        .map(|effect| effect.delta_from_overall.abs())
        .fold(0.0_f64, f64::max)
        .max(0.001) as f32;
    let center_x = bounds.x + bounds.width * 0.5;
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(center_x, bounds.y),
        to: UiPoint::new(center_x, bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(130, 145, 160, 145), ui_scale.value(1.0)),
    });
    for (index, effect) in analysis.factor_effects.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = (effect.delta_from_overall.abs() as f32 / max_abs) * bounds.width * 0.48;
        let x = if effect.delta_from_overall >= 0.0 {
            center_x
        } else {
            center_x - width
        };
        let color = if effect.delta_from_overall.abs() < 0.01 {
            ColorRgba::new(91, 190, 130, 190)
        } else if effect.delta_from_overall > 0.0 {
            ColorRgba::new(238, 181, 82, 215)
        } else {
            ColorRgba::new(102, 190, 236, 215)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(x, y, width.max(ui_scale.value(2.0)), row_height),
            color,
        )));
    }
}

pub(crate) fn add_experiment_status_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if plan.runs.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let statuses = [
        ExperimentRunStatus::Ready,
        ExperimentRunStatus::InProgress,
        ExperimentRunStatus::Complete,
        ExperimentRunStatus::Blocked,
    ];
    let gap = ui_scale.value(6.0);
    let row_height =
        (bounds.height - gap * statuses.len().saturating_sub(1) as f32) / statuses.len() as f32;
    let total = plan.runs.len().max(1) as f32;
    for (index, status) in statuses.iter().enumerate() {
        let count = plan.runs.iter().filter(|run| run.status == *status).count();
        let y = bounds.y + index as f32 * (row_height + gap);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, bounds.width, row_height),
            ColorRgba::new(25, 32, 39, 255),
        )));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, bounds.width * count as f32 / total, row_height),
            experiment_run_status_color(*status, 215),
        )));
    }
}

pub(crate) fn experiment_run_status_color(status: ExperimentRunStatus, alpha: u8) -> ColorRgba {
    match status {
        ExperimentRunStatus::Ready => ColorRgba::new(102, 190, 236, alpha),
        ExperimentRunStatus::InProgress => ColorRgba::new(238, 181, 82, alpha),
        ExperimentRunStatus::Complete => ColorRgba::new(91, 190, 130, alpha),
        ExperimentRunStatus::Blocked => ColorRgba::new(236, 91, 88, alpha),
    }
}

pub(crate) fn experiment_response_status_color(
    status: ResponseValueStatus,
    alpha: u8,
) -> ColorRgba {
    match status {
        ResponseValueStatus::InSpec => ColorRgba::new(91, 190, 130, alpha),
        ResponseValueStatus::BelowSpec => ColorRgba::new(102, 190, 236, alpha),
        ResponseValueStatus::AboveSpec => ColorRgba::new(236, 91, 88, alpha),
    }
}

pub(crate) fn demo_experiment_response_value(
    run: &ExperimentRun,
    response_id: &ResponseSpecId,
) -> Option<f64> {
    let dose_level = run
        .factor_levels
        .get(&FactorId::new("dose"))
        .map(|level| level.as_str())
        .unwrap_or("dose_nominal");
    let focus_level = run
        .factor_levels
        .get(&FactorId::new("focus"))
        .map(|level| level.as_str())
        .unwrap_or("focus_minus");

    let dose_cd: f64 = match dose_level {
        "dose_low" => -2.2,
        "dose_high" => 2.4,
        _ => 0.0,
    };
    let focus_cd = match focus_level {
        "focus_plus" => -0.7,
        _ => 0.4,
    };
    let process_bonus = match (dose_level, focus_level) {
        ("dose_nominal", "focus_plus") => 0.03,
        ("dose_high", "focus_plus") => 0.01,
        ("dose_low", "focus_plus") => -0.02,
        _ => 0.0,
    };

    match response_id.as_str() {
        "poly_cd_nm" => Some(72.0 + dose_cd + focus_cd),
        "defect_count" => Some(match dose_level {
            "dose_low" => 8.0,
            "dose_high" if focus_level == "focus_minus" => 6.0,
            "dose_high" => 4.0,
            _ => 3.0,
        }),
        "yield_fraction" => Some((0.88 + process_bonus - dose_cd.abs() * 0.01).clamp(0.0, 1.0)),
        _ => None,
    }
}

pub(crate) fn format_experiment_response_value(value: f64, response: &ResponseSpec) -> String {
    if response.unit == "%" && value.abs() <= 1.0 {
        format!("{:.1}%", value * 100.0)
    } else if response.unit.is_empty() {
        format_compact_number(value)
    } else {
        format!("{} {}", format_compact_number(value), response.unit)
    }
}

pub(crate) fn experiment_response_spec_label(response: &ResponseSpec) -> String {
    let target = response
        .target
        .map(|value| format_experiment_response_value(value, response))
        .unwrap_or_else(|| "no target".to_string());
    let spec = match (response.lower_spec, response.upper_spec) {
        (Some(lower), Some(upper)) => format!(
            "{}..{}",
            format_experiment_response_value(lower, response),
            format_experiment_response_value(upper, response)
        ),
        (Some(lower), None) => format!(">= {}", format_experiment_response_value(lower, response)),
        (None, Some(upper)) => format!("<= {}", format_experiment_response_value(upper, response)),
        (None, None) => "no spec".to_string(),
    };
    format!("Target {target}; spec {spec}")
}

pub(crate) fn experiment_level_color(index: usize, alpha: u8) -> ColorRgba {
    match index % 6 {
        0 => ColorRgba::new(93, 168, 232, alpha),
        1 => ColorRgba::new(136, 207, 190, alpha),
        2 => ColorRgba::new(238, 181, 82, alpha),
        3 => ColorRgba::new(157, 126, 226, alpha),
        4 => ColorRgba::new(226, 126, 74, alpha),
        _ => ColorRgba::new(105, 201, 135, alpha),
    }
}

pub(crate) fn experiment_readiness_label(
    analysis: &ExperimentAnalysisSummary,
    response_count: usize,
) -> String {
    if analysis.run_count == 0 {
        "No run matrix".to_string()
    } else if response_count == 0 {
        "No responses defined".to_string()
    } else if analysis.missing_response_count == 0 {
        "Ready for analysis".to_string()
    } else if analysis.completed_runs > 0 {
        "Partial analysis ready".to_string()
    } else {
        "Capture responses".to_string()
    }
}
