#![allow(unused_imports)]
use super::*;

pub(crate) fn yield_view_primitives(
    app: &GlassworksApp,
    lot_id: &str,
    wafer_id: &str,
    outcomes: &[DieOutcome],
    ui_scale: UiScale,
    compact_rows: bool,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(outcomes.len() * 2 + 90);
    let frame_width = if compact_rows { 250.0 } else { 900.0 };
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(frame_width),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    if compact_rows {
        let wafer_size = ui_scale.value(178.0);
        let wafer_bounds = UiRect::new(
            frame.x + (frame.width - wafer_size) * 0.5,
            frame.y + ui_scale.value(12.0),
            wafer_size,
            wafer_size,
        );
        add_yield_die_map_primitives(&mut primitives, wafer_bounds, outcomes, app, ui_scale);
        return primitives;
    }
    let wafer_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(12.0),
        ui_scale.value(178.0),
        ui_scale.value(178.0),
    );
    let wafer_strip = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(286.0),
        ui_scale.value(52.0),
    );
    let failure_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(112.0),
        ui_scale.value(286.0),
        ui_scale.value(70.0),
    );
    let measurement_bounds = UiRect::new(
        frame.x + ui_scale.value(576.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(292.0),
        ui_scale.value(154.0),
    );
    add_yield_die_map_primitives(&mut primitives, wafer_bounds, outcomes, app, ui_scale);
    add_yield_wafer_strip_primitives(
        &mut primitives,
        wafer_strip,
        &app.workspace.yield_analysis,
        lot_id,
        wafer_id,
        ui_scale,
    );
    add_yield_failure_primitives(
        &mut primitives,
        failure_bounds,
        app.workspace
            .yield_analysis
            .wafer_summary(lot_id, wafer_id)
            .or_else(|| app.workspace.yield_analysis.lot_summary(lot_id)),
        ui_scale,
    );
    add_yield_measurement_primitives(
        &mut primitives,
        measurement_bounds,
        &app.workspace
            .yield_analysis
            .measurements_for_wafer(lot_id, wafer_id),
        ui_scale,
    );
    primitives
}

pub(crate) fn add_yield_die_map_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    outcomes: &[DieOutcome],
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let center = UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    let radius = bounds.width.min(bounds.height) * 0.5;
    primitives.push(ScenePrimitive::Circle {
        center,
        radius,
        fill: ColorRgba::new(20, 27, 32, 255),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(96, 111, 126, 255),
            ui_scale.value(1.2),
        )),
    });
    if outcomes.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let min_col = outcomes
        .iter()
        .map(|outcome| outcome.die.column)
        .min()
        .unwrap_or(0);
    let max_col = outcomes
        .iter()
        .map(|outcome| outcome.die.column)
        .max()
        .unwrap_or(0);
    let min_row = outcomes
        .iter()
        .map(|outcome| outcome.die.row)
        .min()
        .unwrap_or(0);
    let max_row = outcomes
        .iter()
        .map(|outcome| outcome.die.row)
        .max()
        .unwrap_or(0);
    let columns = (max_col - min_col + 1).max(1) as f32;
    let rows = (max_row - min_row + 1).max(1) as f32;
    let cell =
        (bounds.width.min(bounds.height) * 0.82 / columns.max(rows)).max(ui_scale.value(3.0));
    for outcome in outcomes {
        if !yield_outcome_matches_filter(outcome, app.yield_map_filter) {
            continue;
        }
        let x = center.x + (outcome.die.column as f32 - (min_col + max_col) as f32 * 0.5) * cell;
        let y = center.y - (outcome.die.row as f32 - (min_row + max_row) as f32 * 0.5) * cell;
        if ((x - center.x).powi(2) + (y - center.y).powi(2)).sqrt() > radius * 0.96 {
            continue;
        }
        let rect = UiRect::new(
            x - cell * 0.42,
            y - cell * 0.42,
            (cell * 0.84).max(ui_scale.value(2.0)),
            (cell * 0.84).max(ui_scale.value(2.0)),
        );
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(rect, yield_outcome_color(outcome, 225)).stroke(
                StrokeStyle::new(ColorRgba::new(16, 20, 24, 150), ui_scale.value(0.7)),
            ),
        ));
        if !outcome.passed {
            primitives.push(ScenePrimitive::Circle {
                center: UiPoint::new(x, y),
                radius: (cell * 0.18).max(ui_scale.value(1.4)),
                fill: ColorRgba::new(252, 253, 255, 210),
                stroke: None,
            });
        }
    }
}

pub(crate) fn add_yield_wafer_strip_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    analysis: &YieldAnalysis,
    lot_id: &str,
    selected_wafer: &str,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let wafers = analysis.wafer_summaries_for_lot(lot_id);
    if wafers.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(2.0);
    let width = (bounds.width - gap * wafers.len().saturating_sub(1) as f32) / wafers.len() as f32;
    for (index, summary) in wafers.iter().enumerate() {
        let wafer_id = summary.wafer_id.as_deref().unwrap_or("");
        let rect = UiRect::new(
            bounds.x + index as f32 * (width + gap),
            bounds.y + ui_scale.value(5.0),
            width.max(ui_scale.value(3.0)),
            bounds.height - ui_scale.value(10.0),
        );
        let stroke = if wafer_id == selected_wafer {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(1.4))
        } else {
            StrokeStyle::new(ColorRgba::new(16, 20, 24, 150), ui_scale.value(0.7))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(rect, yield_fraction_color(summary.yield_fraction, 225))
                .stroke(stroke),
        ));
    }
}

pub(crate) fn add_yield_failure_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    summary: Option<&YieldSummary>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let Some(summary) = summary else {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    };
    if summary.failure_counts.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 155),
        )));
        return;
    }
    let total = summary.failure_counts.values().sum::<u32>().max(1) as f32;
    let gap = ui_scale.value(4.0);
    let row_height = (bounds.height - gap * summary.failure_counts.len().saturating_sub(1) as f32)
        / summary.failure_counts.len() as f32;
    for (index, (mode, count)) in summary.failure_counts.iter().enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = bounds.width * (*count as f32 / total);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, width.max(ui_scale.value(2.0)), row_height),
            yield_failure_mode_color(*mode, 215),
        )));
    }
}

pub(crate) fn add_yield_measurement_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    measurements: &[&ProcessMeasurement],
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if measurements.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = measurements.len().min(8);
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, measurement) in measurements.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let ratio = yield_measurement_ratio(measurement);
        let marker_x = row.x + row.width * ratio.clamp(0.0, 1.0);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(marker_x, row.y),
            to: UiPoint::new(marker_x, row.bottom()),
            stroke: StrokeStyle::new(
                if yield_measurement_excursion(measurement) {
                    ColorRgba::new(236, 91, 88, 230)
                } else {
                    ColorRgba::new(136, 207, 190, 220)
                },
                ui_scale.value(1.4),
            ),
        });
    }
}

pub(crate) fn yield_outcome_matches_filter(outcome: &DieOutcome, filter: YieldMapFilter) -> bool {
    match filter {
        YieldMapFilter::All => true,
        YieldMapFilter::Failing => !outcome.passed,
        YieldMapFilter::Passing => outcome.passed,
    }
}

pub(crate) fn yield_outcome_color(outcome: &DieOutcome, alpha: u8) -> ColorRgba {
    if outcome.passed {
        return ColorRgba::new(91, 190, 130, alpha);
    }
    outcome
        .failure_modes
        .first()
        .map(|mode| yield_failure_mode_color(*mode, alpha))
        .unwrap_or(ColorRgba::new(236, 91, 88, alpha))
}

pub(crate) fn yield_failure_mode_color(mode: FailureMode, alpha: u8) -> ColorRgba {
    match mode {
        FailureMode::OpenCircuit => ColorRgba::new(236, 91, 88, alpha),
        FailureMode::ShortCircuit => ColorRgba::new(226, 126, 74, alpha),
        FailureMode::HighLeakage => ColorRgba::new(238, 181, 82, alpha),
        FailureMode::LowFrequency => ColorRgba::new(102, 190, 236, alpha),
        FailureMode::ParametricDrift => ColorRgba::new(157, 126, 226, alpha),
        FailureMode::ContactResistance => ColorRgba::new(210, 146, 92, alpha),
        FailureMode::EdgeDefect => ColorRgba::new(244, 112, 104, alpha),
    }
}

pub(crate) fn yield_fraction_color(value: f64, alpha: u8) -> ColorRgba {
    if value >= 0.98 {
        ColorRgba::new(91, 190, 130, alpha)
    } else if value >= 0.94 {
        ColorRgba::new(102, 190, 236, alpha)
    } else if value >= 0.90 {
        ColorRgba::new(238, 181, 82, alpha)
    } else {
        ColorRgba::new(236, 91, 88, alpha)
    }
}

pub(crate) fn yield_measurement_ratio(measurement: &ProcessMeasurement) -> f32 {
    match (measurement.lower_spec, measurement.upper_spec) {
        (Some(lower), Some(upper)) if (upper - lower).abs() > f64::EPSILON => {
            ((measurement.value - lower) / (upper - lower)).clamp(0.0, 1.0) as f32
        }
        (Some(lower), Some(_)) => {
            ((measurement.value - lower) / lower.abs().max(1.0) * 0.5 + 0.5).clamp(0.0, 1.0) as f32
        }
        (Some(lower), None) => {
            ((measurement.value - lower) / lower.abs().max(1.0) * 0.5 + 0.5).clamp(0.0, 1.0) as f32
        }
        (None, Some(upper)) => (measurement.value / upper.abs().max(1.0)).clamp(0.0, 1.0) as f32,
        (None, None) => 0.5,
    }
}

pub(crate) fn yield_measurement_excursion(measurement: &ProcessMeasurement) -> bool {
    measurement
        .lower_spec
        .is_some_and(|lower| measurement.value < lower)
        || measurement
            .upper_spec
            .is_some_and(|upper| measurement.value > upper)
}

pub(crate) fn yield_excursion_count(measurements: &[&ProcessMeasurement]) -> usize {
    measurements
        .iter()
        .filter(|measurement| yield_measurement_excursion(measurement))
        .count()
}

pub(crate) fn yield_failure_label(summary: &YieldSummary) -> String {
    compact_button_label(
        &format!(
            "{}; {}; {}",
            summary
                .dominant_failure
                .map(|failure| failure.label())
                .unwrap_or("no dominant failure"),
            summary.spatial_pattern.label(),
            summary
                .root_cause_hints
                .first()
                .map(String::as_str)
                .unwrap_or("no root-cause hint")
        ),
        68,
    )
}

pub(crate) fn yield_failure_mode_short_label(failure: Option<FailureMode>) -> &'static str {
    match failure {
        Some(FailureMode::OpenCircuit) => "open",
        Some(FailureMode::ShortCircuit) => "short",
        Some(FailureMode::HighLeakage) => "leakage",
        Some(FailureMode::LowFrequency) => "low-freq",
        Some(FailureMode::ParametricDrift) => "drift",
        Some(FailureMode::ContactResistance) => "contact R",
        Some(FailureMode::EdgeDefect) => "edge defect",
        None => "no fails",
    }
}

pub(crate) fn yield_spatial_short_label(summary: &YieldSummary) -> &'static str {
    match summary.spatial_pattern.label() {
        "edge-heavy" => "edge",
        "center-heavy" => "center",
        "no failures" => "none",
        other => other,
    }
}

pub(crate) fn yield_failure_compact_label(summary: &YieldSummary) -> String {
    format!(
        "{}; {}",
        yield_failure_mode_short_label(summary.dominant_failure),
        yield_spatial_short_label(summary)
    )
}

pub(crate) fn yield_root_cause_compact_label(summary: &YieldSummary) -> String {
    match (
        summary.spatial_pattern.label(),
        summary.dominant_failure.map(|failure| failure.label()),
    ) {
        ("edge-heavy", Some("Low frequency")) => "Edge low-frequency issue".to_string(),
        (_, Some(failure)) => format!("{failure} issue"),
        _ => "No root-cause hint".to_string(),
    }
}
