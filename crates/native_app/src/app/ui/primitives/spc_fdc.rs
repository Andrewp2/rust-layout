#![allow(unused_imports)]
use super::*;

#[derive(Clone, Copy)]
pub(crate) struct SpcPlotGuide {
    pub(crate) value: f64,
    pub(crate) color: ColorRgba,
    pub(crate) width: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct SpcPlotHighlight {
    pub(crate) start_x: f32,
    pub(crate) end_x: f32,
    pub(crate) severity: MonitorSeverity,
}

pub(crate) fn spc_fdc_monitor_primitives(
    chart: Option<&ControlChart>,
    trace: Option<&SensorTrace>,
    monitor: &SpcFdcMonitor,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(120);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(190.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));

    let chart_plot = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(22.0),
        ui_scale.value(402.0),
        ui_scale.value(132.0),
    );
    let trace_plot = UiRect::new(
        frame.x + ui_scale.value(476.0),
        frame.y + ui_scale.value(22.0),
        ui_scale.value(402.0),
        ui_scale.value(132.0),
    );
    add_spc_chart_plot_primitives(&mut primitives, chart_plot, chart, ui_scale);
    add_fdc_trace_plot_primitives(&mut primitives, trace_plot, trace, ui_scale);
    add_spc_alarm_strip_primitives(
        &mut primitives,
        UiRect::new(
            frame.x + ui_scale.value(22.0),
            frame.bottom() - ui_scale.value(24.0),
            frame.width - ui_scale.value(44.0),
            ui_scale.value(10.0),
        ),
        monitor,
        ui_scale,
    );
    primitives
}

pub(crate) fn add_spc_chart_plot_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    chart: Option<&ControlChart>,
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, plot, ui_scale);
    let Some(chart) = chart else {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    };
    if chart.points.is_empty() {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    }

    let values = chart
        .points
        .iter()
        .map(|point| {
            (
                point.sequence as f32,
                point.value,
                !chart.limits.contains(point.value),
            )
        })
        .collect::<Vec<_>>();
    let (min_x, max_x) = values
        .first()
        .zip(values.last())
        .map(|(first, last)| (first.0, last.0))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value, _) in &values {
        add_spc_plot_value(&mut min_value, &mut max_value, *value);
    }
    for guide in [
        chart.limits.lower_control,
        chart.limits.upper_control,
        chart.limits.center,
        chart.limits.two_sigma_low(),
        chart.limits.two_sigma_high(),
    ] {
        add_spc_plot_value(&mut min_value, &mut max_value, guide);
    }
    if let Some(lower_spec) = chart.limits.lower_spec {
        add_spc_plot_value(&mut min_value, &mut max_value, lower_spec);
    }
    if let Some(upper_spec) = chart.limits.upper_spec {
        add_spc_plot_value(&mut min_value, &mut max_value, upper_spec);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    for violation in &chart.violations {
        if let Some(highlight) = spc_chart_highlight(chart, violation) {
            add_spc_plot_highlight(primitives, plot, min_x, max_x, highlight, ui_scale);
        }
    }

    let mut guides = vec![
        SpcPlotGuide {
            value: chart.limits.upper_control,
            color: ColorRgba::new(130, 145, 160, 200),
            width: ui_scale.value(1.2),
        },
        SpcPlotGuide {
            value: chart.limits.lower_control,
            color: ColorRgba::new(130, 145, 160, 200),
            width: ui_scale.value(1.2),
        },
        SpcPlotGuide {
            value: chart.limits.center,
            color: ColorRgba::new(93, 168, 232, 230),
            width: ui_scale.value(1.1),
        },
        SpcPlotGuide {
            value: chart.limits.two_sigma_high(),
            color: ColorRgba::new(220, 176, 72, 165),
            width: ui_scale.value(0.9),
        },
        SpcPlotGuide {
            value: chart.limits.two_sigma_low(),
            color: ColorRgba::new(220, 176, 72, 165),
            width: ui_scale.value(0.9),
        },
    ];
    if let Some(upper_spec) = chart.limits.upper_spec {
        guides.push(SpcPlotGuide {
            value: upper_spec,
            color: ColorRgba::new(226, 96, 96, 180),
            width: ui_scale.value(1.0),
        });
    }
    if let Some(lower_spec) = chart.limits.lower_spec {
        guides.push(SpcPlotGuide {
            value: lower_spec,
            color: ColorRgba::new(226, 96, 96, 180),
            width: ui_scale.value(1.0),
        });
    }
    add_spc_plot_guides(primitives, plot, min_value, max_value, &guides);
    add_spc_plot_values(
        primitives, plot, min_x, max_x, min_value, max_value, &values, ui_scale,
    );
}

pub(crate) fn add_fdc_trace_plot_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    trace: Option<&SensorTrace>,
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, plot, ui_scale);
    let Some(trace) = trace else {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    };
    if trace.samples.is_empty() {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    }

    let values = trace
        .samples
        .iter()
        .map(|point| {
            (
                point.at_s as f32,
                point.value,
                !trace.limit.contains(point.value),
            )
        })
        .collect::<Vec<_>>();
    let (min_x, max_x) = values
        .first()
        .zip(values.last())
        .map(|(first, last)| (first.0, last.0))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value, _) in &values {
        add_spc_plot_value(&mut min_value, &mut max_value, *value);
    }
    if let Some(lower) = trace.limit.lower {
        add_spc_plot_value(&mut min_value, &mut max_value, lower);
    }
    if let Some(upper) = trace.limit.upper {
        add_spc_plot_value(&mut min_value, &mut max_value, upper);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    let span = trace_sample_span(trace);
    for violation in &trace.violations {
        add_spc_plot_highlight(
            primitives,
            plot,
            min_x,
            max_x,
            SpcPlotHighlight {
                start_x: violation.at_s as f32 - span * 0.35,
                end_x: violation.at_s as f32 + span * 0.35,
                severity: violation.severity,
            },
            ui_scale,
        );
    }

    let mut guides = Vec::new();
    if let Some(lower) = trace.limit.lower {
        guides.push(SpcPlotGuide {
            value: lower,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.2),
        });
    }
    if let Some(upper) = trace.limit.upper {
        guides.push(SpcPlotGuide {
            value: upper,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.2),
        });
    }
    add_spc_plot_guides(primitives, plot, min_value, max_value, &guides);
    add_spc_plot_values(
        primitives, plot, min_x, max_x, min_value, max_value, &values, ui_scale,
    );
}

pub(crate) fn add_spc_plot_background(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(plot, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    for fraction in [0.25_f32, 0.5, 0.75] {
        let x = plot.x + plot.width * fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(122, 138, 154, 46), ui_scale.value(1.0)),
        });
        let y = plot.y + plot.height * fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, y),
            to: UiPoint::new(plot.right(), y),
            stroke: StrokeStyle::new(ColorRgba::new(122, 138, 154, 46), ui_scale.value(1.0)),
        });
    }
}

pub(crate) fn add_spc_empty_plot(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(plot.x + ui_scale.value(24.0), plot.y + plot.height * 0.5),
        to: UiPoint::new(
            plot.right() - ui_scale.value(24.0),
            plot.y + plot.height * 0.5,
        ),
        stroke: StrokeStyle::new(ColorRgba::new(92, 106, 120, 190), ui_scale.value(1.0)),
    });
}

pub(crate) fn add_spc_plot_guides(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_value: f64,
    max_value: f64,
    guides: &[SpcPlotGuide],
) {
    for guide in guides {
        let y = spc_plot_y(plot, min_value, max_value, guide.value);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, y),
            to: UiPoint::new(plot.right(), y),
            stroke: StrokeStyle::new(guide.color, guide.width),
        });
    }
}

pub(crate) fn add_spc_plot_highlight(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_x: f32,
    max_x: f32,
    highlight: SpcPlotHighlight,
    ui_scale: UiScale,
) {
    let left = spc_plot_x(plot, min_x, max_x, highlight.start_x);
    let right = spc_plot_x(plot, min_x, max_x, highlight.end_x);
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            left.min(right),
            plot.y,
            (right - left).abs().max(ui_scale.value(2.0)),
            plot.height,
        ),
        spc_severity_color(highlight.severity, 42),
    )));
}

pub(crate) fn add_spc_plot_values(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_x: f32,
    max_x: f32,
    min_value: f64,
    max_value: f64,
    values: &[(f32, f64, bool)],
    ui_scale: UiScale,
) {
    for pair in values.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, pair[0].0),
                spc_plot_y(plot, min_value, max_value, pair[0].1),
            ),
            to: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, pair[1].0),
                spc_plot_y(plot, min_value, max_value, pair[1].1),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(82, 156, 219, 235), ui_scale.value(2.0)),
        });
    }
    for (index, (x, value, outside)) in values.iter().enumerate() {
        let latest = index + 1 == values.len();
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, *x),
                spc_plot_y(plot, min_value, max_value, *value),
            ),
            radius: ui_scale.value(if latest { 4.7 } else { 3.4 }),
            fill: if *outside {
                ColorRgba::new(244, 112, 104, 255)
            } else {
                ColorRgba::new(105, 201, 135, 245)
            },
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(1.0),
            )),
        });
    }
}

pub(crate) fn add_spc_alarm_strip_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    strip: UiRect,
    monitor: &SpcFdcMonitor,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(strip, ColorRgba::new(27, 35, 42, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(54, 66, 78, 255),
            ui_scale.value(1.0),
        )),
    ));
    if monitor.alarm_summary.active_count == 0 {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(strip.x, strip.y, strip.width, strip.height),
            ColorRgba::new(80, 166, 114, 135),
        )));
        return;
    }
    let visible = monitor.alarm_summary.by_tool.len().max(1).min(10);
    let gap = ui_scale.value(3.0);
    let item_width = (strip.width - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, count) in monitor
        .alarm_summary
        .by_tool
        .values()
        .take(visible)
        .enumerate()
    {
        let color = if *count > 1 {
            ColorRgba::new(236, 91, 88, 185)
        } else {
            ColorRgba::new(238, 181, 82, 175)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                strip.x + index as f32 * (item_width + gap),
                strip.y,
                item_width,
                strip.height,
            ),
            color,
        )));
    }
}

pub(crate) fn spc_plot_x(plot: UiRect, min_x: f32, max_x: f32, x: f32) -> f32 {
    let span = (max_x - min_x).abs().max(1.0);
    plot.x + ((x - min_x) / span).clamp(0.0, 1.0) * plot.width
}

pub(crate) fn spc_plot_y(plot: UiRect, min_value: f64, max_value: f64, value: f64) -> f32 {
    let span = (max_value - min_value).abs().max(f64::EPSILON);
    let fraction = ((value - min_value) / span) as f32;
    plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height
}

pub(crate) fn add_spc_plot_value(min_value: &mut f64, max_value: &mut f64, value: f64) {
    if value.is_finite() {
        *min_value = min_value.min(value);
        *max_value = max_value.max(value);
    }
}

pub(crate) fn padded_plot_range(min_value: f64, max_value: f64) -> (f64, f64) {
    if !min_value.is_finite() || !max_value.is_finite() {
        return (0.0, 1.0);
    }
    let padding = ((max_value - min_value).abs() * 0.12).max(0.5);
    (min_value - padding, max_value + padding)
}

pub(crate) fn spc_chart_highlight(
    chart: &ControlChart,
    violation: &layout_model::spc_fdc::RuleViolation,
) -> Option<SpcPlotHighlight> {
    let mut points = violation
        .point_indices
        .iter()
        .filter_map(|index| chart.points.get(*index));
    let first = points.next()?;
    let mut start_x = first.sequence as f32;
    let mut end_x = start_x;
    for point in points {
        let x = point.sequence as f32;
        start_x = start_x.min(x);
        end_x = end_x.max(x);
    }
    let padding = if (end_x - start_x).abs() < f32::EPSILON {
        0.35
    } else {
        0.2
    };
    Some(SpcPlotHighlight {
        start_x: start_x - padding,
        end_x: end_x + padding,
        severity: violation.severity,
    })
}

pub(crate) fn trace_sample_span(trace: &SensorTrace) -> f32 {
    let span = trace
        .samples
        .windows(2)
        .filter_map(|pair| {
            let span = pair[1].at_s.saturating_sub(pair[0].at_s);
            (span > 0).then_some(span as f32)
        })
        .fold(f32::INFINITY, f32::min);
    if span.is_finite() { span.max(1.0) } else { 1.0 }
}

pub(crate) fn spc_severity_color(severity: MonitorSeverity, alpha: u8) -> ColorRgba {
    match severity {
        MonitorSeverity::Advisory => ColorRgba::new(88, 178, 232, alpha),
        MonitorSeverity::Warning => ColorRgba::new(238, 181, 82, alpha),
        MonitorSeverity::Critical => ColorRgba::new(236, 91, 88, alpha),
    }
}

pub(crate) fn spc_chart_summary_label(chart: &ControlChart, max_chars: usize) -> String {
    let latest = chart
        .latest_point()
        .map(|point| {
            format!(
                "{} {} on {}",
                spc_compact_number(point.value),
                chart.unit,
                point.wafer_id
            )
        })
        .unwrap_or_else(|| "no samples".to_string());
    compact_button_label(
        &format!(
            "{}: {latest}; {}; {} rules",
            display_measurement_identifier(&chart.metric),
            spc_chart_trend_label(chart),
            chart.violations.len()
        ),
        max_chars,
    )
}

pub(crate) fn spc_trace_summary_label(
    monitor: &SpcFdcMonitor,
    trace: &SensorTrace,
    max_chars: usize,
) -> String {
    let latest = trace
        .latest_point()
        .map(|point| {
            format!(
                "{} {} at t+{}s",
                spc_compact_number(point.value),
                trace.unit,
                point.at_s
            )
        })
        .unwrap_or_else(|| "no samples".to_string());
    compact_button_label(
        &format!(
            "{}: {latest}; {}; {} alarms",
            display_spc_trace_name(trace),
            spc_trace_range_label(trace),
            same_tool_alarm_count(monitor, &trace.tool_id)
        ),
        max_chars,
    )
}

pub(crate) fn spc_chart_trend_label(chart: &ControlChart) -> String {
    let Some(latest) = chart.points.last() else {
        return "n/a".to_string();
    };
    let Some(previous) = chart.points.iter().rev().nth(1) else {
        return "single point".to_string();
    };
    let delta = latest.value - previous.value;
    let direction = if delta.abs() <= chart.limits.one_sigma * 0.05 {
        "flat"
    } else if delta > 0.0 {
        "rising"
    } else {
        "falling"
    };
    let signed_delta = if delta >= 0.0 {
        format!("+{}", spc_compact_number(delta))
    } else {
        spc_compact_number(delta)
    };
    format!("{direction} ({signed_delta} {})", chart.unit)
}

pub(crate) fn spc_trace_range_label(trace: &SensorTrace) -> String {
    let Some(first) = trace.samples.first() else {
        return "n/a".to_string();
    };
    let (min_value, max_value) = trace.samples.iter().fold(
        (first.value, first.value),
        |(min_value, max_value), sample| (min_value.min(sample.value), max_value.max(sample.value)),
    );
    format!(
        "{}..{} {}",
        spc_compact_number(min_value),
        spc_compact_number(max_value),
        trace.unit
    )
}

pub(crate) fn same_tool_alarm_count(monitor: &SpcFdcMonitor, tool_id: &str) -> usize {
    monitor
        .findings
        .iter()
        .filter(|finding| {
            finding.source == FindingSource::EquipmentAlarm
                && finding.tool_id.as_deref() == Some(tool_id)
        })
        .count()
}

pub(crate) fn spc_compact_number(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}
