#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct EnvironmentZoneSummary {
    pub(crate) name: String,
    pub(crate) sensor_count: usize,
    pub(crate) advisory_count: usize,
    pub(crate) warning_count: usize,
    pub(crate) critical_count: usize,
    pub(crate) excursion_count: usize,
    pub(crate) latest_timestamp_min: Option<u32>,
}

pub(crate) fn environment_view_primitives(
    app: &GlassworksApp,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let environment = &app.workspace.environment;
    let mut primitives =
        Vec::with_capacity(environment.sensors.len() * 8 + environment.alarms.len() + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let trend_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(24.0),
        ui_scale.value(404.0),
        ui_scale.value(158.0),
    );
    let zone_bounds = UiRect::new(
        frame.x + ui_scale.value(468.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(186.0),
        ui_scale.value(154.0),
    );
    let alarm_bounds = UiRect::new(
        frame.x + ui_scale.value(696.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(184.0),
        ui_scale.value(154.0),
    );
    if let Some(sensor) = selected_environment_sensor(app) {
        add_environment_trend_primitives(
            &mut primitives,
            trend_bounds,
            environment,
            sensor,
            ui_scale,
        );
    } else {
        add_metrology_empty_line(&mut primitives, trend_bounds, ui_scale);
    }
    add_environment_zone_primitives(
        &mut primitives,
        zone_bounds,
        &environment_zone_summaries(environment),
        selected_environment_sensor(app).map(|sensor| sensor.zone.as_str()),
        ui_scale,
    );
    add_environment_alarm_primitives(&mut primitives, alarm_bounds, environment, ui_scale);
    primitives
}

pub(crate) fn add_environment_trend_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    environment: &CleanroomEnvironment,
    sensor: &EnvironmentSensor,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let readings = environment.readings_for_sensor(&sensor.id);
    if readings.len() < 2 {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let min_x = readings
        .first()
        .map(|reading| reading.timestamp_min)
        .unwrap_or_default() as f32;
    let max_x = readings
        .last()
        .map(|reading| reading.timestamp_min)
        .unwrap_or_default() as f32;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for reading in &readings {
        add_spc_plot_value(&mut min_value, &mut max_value, reading.value);
    }
    for threshold in [
        sensor.thresholds.critical_low,
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
        sensor.thresholds.critical_high,
    ]
    .into_iter()
    .flatten()
    {
        add_spc_plot_value(&mut min_value, &mut max_value, threshold);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);
    add_environment_threshold_band(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
        ColorRgba::new(80, 166, 114, 54),
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_low,
        ColorRgba::new(238, 181, 82, 165),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_high,
        ColorRgba::new(238, 181, 82, 165),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.critical_low,
        ColorRgba::new(236, 91, 88, 175),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.critical_high,
        ColorRgba::new(236, 91, 88, 175),
        ui_scale,
    );
    let trend_color = environment
        .trend_summary(&sensor.id)
        .map(|summary| environment_trend_color(summary.direction, 230))
        .unwrap_or(ColorRgba::new(93, 168, 232, 230));
    for pair in readings.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].timestamp_min as f32),
                spc_plot_y(bounds, min_value, max_value, pair[0].value),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].timestamp_min as f32),
                spc_plot_y(bounds, min_value, max_value, pair[1].value),
            ),
            stroke: StrokeStyle::new(trend_color, ui_scale.value(1.8)),
        });
    }
    for reading in &readings {
        let severity = sensor.thresholds.evaluate(reading.value);
        let point = UiPoint::new(
            spc_plot_x(bounds, min_x, max_x, reading.timestamp_min as f32),
            spc_plot_y(bounds, min_value, max_value, reading.value),
        );
        if let Some(severity) = severity {
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(point.x, bounds.y),
                to: UiPoint::new(point.x, bounds.bottom()),
                stroke: StrokeStyle::new(
                    environment_alarm_severity_color(severity, 92),
                    ui_scale.value(1.0),
                ),
            });
        }
        primitives.push(ScenePrimitive::Circle {
            center: point,
            radius: ui_scale.value(if severity.is_some() { 3.7 } else { 2.6 }),
            fill: environment_sensor_state_color(severity, 235),
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(0.8),
            )),
        });
    }
}

pub(crate) fn add_environment_threshold_band(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    min_value: f64,
    max_value: f64,
    low: Option<f64>,
    high: Option<f64>,
    color: ColorRgba,
) {
    let Some(low) = low else {
        return;
    };
    let Some(high) = high else {
        return;
    };
    let y_high = spc_plot_y(bounds, min_value, max_value, high);
    let y_low = spc_plot_y(bounds, min_value, max_value, low);
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            bounds.x,
            y_high.min(y_low),
            bounds.width,
            (y_low - y_high).abs(),
        ),
        color,
    )));
}

pub(crate) fn add_environment_threshold_line(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    min_value: f64,
    max_value: f64,
    value: Option<f64>,
    color: ColorRgba,
    ui_scale: UiScale,
) {
    let Some(value) = value else {
        return;
    };
    let y = spc_plot_y(bounds, min_value, max_value, value);
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, y),
        to: UiPoint::new(bounds.right(), y),
        stroke: StrokeStyle::new(color, ui_scale.value(1.0)),
    });
}

pub(crate) fn add_environment_zone_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    zones: &[EnvironmentZoneSummary],
    selected_zone: Option<&str>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if zones.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = zones.len().min(7);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, zone) in zones.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        let fill = if selected_zone == Some(zone.name.as_str()) {
            ColorRgba::new(72, 128, 188, 70)
        } else {
            ColorRgba::new(25, 32, 39, 255)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(row, fill)));
        let total = zone.sensor_count.max(1) as f32;
        let critical_width = row.width * zone.critical_count as f32 / total;
        let warning_width = row.width * zone.warning_count as f32 / total;
        let advisory_width = row.width * zone.advisory_count as f32 / total;
        let mut x = row.x;
        for (width, color) in [
            (
                critical_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Critical, 215),
            ),
            (
                warning_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Warning, 215),
            ),
            (
                advisory_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Advisory, 190),
            ),
        ] {
            if width <= 0.0 {
                continue;
            }
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, row.y, width.max(ui_scale.value(1.0)), row.height),
                color,
            )));
            x += width;
        }
    }
}

pub(crate) fn add_environment_alarm_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    environment: &CleanroomEnvironment,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if environment.alarms.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 150),
        )));
        return;
    }
    let visible = environment.alarms.len().min(10);
    let gap = ui_scale.value(4.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, alarm) in environment.alarms.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(bounds.x, y, bounds.width, row_height),
                environment_alarm_severity_color(
                    alarm.severity,
                    if alarm.active { 225 } else { 115 },
                ),
            )
            .stroke(StrokeStyle::new(
                if alarm.active {
                    ColorRgba::new(252, 253, 255, 210)
                } else {
                    ColorRgba::new(16, 20, 24, 130)
                },
                ui_scale.value(if alarm.active { 1.2 } else { 0.6 }),
            )),
        ));
    }
}

pub(crate) fn environment_zone_summaries(
    environment: &CleanroomEnvironment,
) -> Vec<EnvironmentZoneSummary> {
    let mut zones: BTreeMap<String, EnvironmentZoneSummary> = BTreeMap::new();
    for sensor in &environment.sensors {
        let entry = zones
            .entry(sensor.zone.clone())
            .or_insert_with(|| EnvironmentZoneSummary {
                name: sensor.zone.clone(),
                ..Default::default()
            });
        entry.sensor_count += 1;
        if let Some(reading) = environment.latest_reading(&sensor.id) {
            entry.latest_timestamp_min = Some(
                entry
                    .latest_timestamp_min
                    .map_or(reading.timestamp_min, |current| {
                        current.max(reading.timestamp_min)
                    }),
            );
            match sensor.thresholds.evaluate(reading.value) {
                Some(EnvironmentAlarmSeverity::Critical) => entry.critical_count += 1,
                Some(EnvironmentAlarmSeverity::Warning) => entry.warning_count += 1,
                Some(EnvironmentAlarmSeverity::Advisory) => entry.advisory_count += 1,
                None => {}
            }
        }
        entry.excursion_count += environment
            .alarms
            .iter()
            .filter(|alarm| alarm.sensor_id == sensor.id)
            .count();
    }
    zones.into_values().collect()
}

pub(crate) fn environment_selected_sensor_label(
    environment: &CleanroomEnvironment,
    sensor: &EnvironmentSensor,
) -> String {
    environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            format!(
                "{} / {} / {:.2} {} / {}",
                display_environment_sensor_label(sensor),
                sensor.zone,
                reading.value,
                sensor.unit,
                severity.map_or("Nominal", |severity| severity.label())
            )
        })
        .unwrap_or_else(|| {
            format!(
                "{} / {} / no samples",
                display_environment_sensor_label(sensor),
                sensor.zone
            )
        })
}

pub(crate) fn environment_selected_zone_label(
    zones: &[EnvironmentZoneSummary],
    selected_zone: &str,
) -> String {
    zones
        .iter()
        .find(|zone| zone.name == selected_zone)
        .map(|zone| {
            format!(
                "{}: {} sensors, {} excursions",
                zone.name, zone.sensor_count, zone.excursion_count
            )
        })
        .unwrap_or_else(|| format!("{selected_zone}: no zone summary"))
}

pub(crate) fn environment_correlation_label(
    environment: &CleanroomEnvironment,
    sensor_id: &str,
) -> String {
    let labels = environment
        .correlations
        .iter()
        .filter(|label| label.sensor_ids.iter().any(|id| id == sensor_id))
        .map(|label| label.label.as_str())
        .take(3)
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "No process correlation labels".to_string()
    } else {
        compact_button_label(&labels.join(", "), 54)
    }
}

pub(crate) fn environment_sensor_state_color(
    severity: Option<EnvironmentAlarmSeverity>,
    alpha: u8,
) -> ColorRgba {
    severity.map_or(ColorRgba::new(91, 190, 130, alpha), |severity| {
        environment_alarm_severity_color(severity, alpha)
    })
}

pub(crate) fn environment_alarm_severity_color(
    severity: EnvironmentAlarmSeverity,
    alpha: u8,
) -> ColorRgba {
    match severity {
        EnvironmentAlarmSeverity::Advisory => ColorRgba::new(102, 190, 236, alpha),
        EnvironmentAlarmSeverity::Warning => ColorRgba::new(238, 181, 82, alpha),
        EnvironmentAlarmSeverity::Critical => ColorRgba::new(236, 91, 88, alpha),
    }
}

pub(crate) fn environment_trend_color(direction: TrendDirection, alpha: u8) -> ColorRgba {
    match direction {
        TrendDirection::Falling => ColorRgba::new(102, 190, 236, alpha),
        TrendDirection::Stable => ColorRgba::new(91, 190, 130, alpha),
        TrendDirection::Rising => ColorRgba::new(238, 181, 82, alpha),
    }
}
