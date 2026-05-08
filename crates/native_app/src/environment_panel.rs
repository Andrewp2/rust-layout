use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, vec2};
use layout_model::environment::{
    CleanroomEnvironment, EnvironmentAlarm, EnvironmentAlarmSeverity, EnvironmentReading,
    EnvironmentSensor, FacilityEventKind, TrendDirection,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct EnvironmentPanel {
    model: CleanroomEnvironment,
    selected_sensor: Option<String>,
}

impl EnvironmentPanel {
    pub(crate) fn from_model(model: CleanroomEnvironment) -> Self {
        let selected_sensor = model.sensors.first().map(|sensor| sensor.id.clone());
        Self {
            model,
            selected_sensor,
        }
    }

    pub(crate) fn model(&self) -> &CleanroomEnvironment {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Cleanroom Environment");
        ui.label(format!("Sensors: {}", self.model.sensors.len()));
        ui.label(format!("Readings: {}", self.model.readings.len()));
        ui.label(format!(
            "Active alarms: {}",
            self.model.active_alarms().len()
        ));

        ui.separator();
        ui_chrome::section_label(ui, "Alarm Summary");
        let critical = self
            .model
            .alarms
            .iter()
            .filter(|alarm| alarm.severity == EnvironmentAlarmSeverity::Critical)
            .count();
        let warning = self
            .model
            .alarms
            .iter()
            .filter(|alarm| alarm.severity == EnvironmentAlarmSeverity::Warning)
            .count();
        ui.colored_label(
            severity_color(EnvironmentAlarmSeverity::Critical),
            format!("{critical} critical"),
        );
        ui.colored_label(
            severity_color(EnvironmentAlarmSeverity::Warning),
            format!("{warning} warning"),
        );

        ui.separator();
        ui_chrome::section_label(ui, "Process Correlations");
        for label in &self.model.correlations {
            ui.label(RichText::new(&label.label).strong());
            ui_chrome::muted(ui, format!("{}: {}", label.process_area, label.description));
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        egui::ScrollArea::vertical()
            .id_salt("environment_dashboard_scroll")
            .show(ui, |ui| {
                let detail = format!(
                    "{} active alarm(s), {} facility event(s)",
                    self.model.active_alarms().len(),
                    self.model.events.len()
                );
                ui_chrome::module_header(
                    ui,
                    "Facility operations",
                    "Cleanroom Environment",
                    &detail,
                    |_| {},
                );

                self.sensor_picker(ui);
                ui.add_space(4.0);
                self.metric_tiles(ui);

                ui.separator();
                ui.columns(2, |columns| {
                    self.selected_sensor_trend(&mut columns[0]);
                    self.alarm_panel(&mut columns[1]);
                });

                ui.separator();
                ui.columns(2, |columns| {
                    self.zone_dashboard(&mut columns[0]);
                    self.facility_timeline(&mut columns[1]);
                });
            });
    }

    fn ensure_selection(&mut self) {
        let valid = self
            .selected_sensor
            .as_ref()
            .is_some_and(|id| self.model.sensors.iter().any(|sensor| &sensor.id == id));
        if !valid {
            self.selected_sensor = self.model.sensors.first().map(|sensor| sensor.id.clone());
        }
    }

    fn sensor_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Sensor");
            let selected_text = self
                .selected_sensor
                .as_ref()
                .and_then(|id| self.sensor(id))
                .map(|sensor| sensor.name.clone())
                .unwrap_or_else(|| "No sensor".to_string());
            egui::ComboBox::from_id_salt("environment_sensor_picker")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    let sensor_choices = self
                        .model
                        .sensors
                        .iter()
                        .map(|sensor| {
                            (
                                sensor.id.clone(),
                                format!("{} / {}", sensor.zone, sensor.name),
                            )
                        })
                        .collect::<Vec<_>>();
                    for (sensor_id, label) in sensor_choices {
                        ui.selectable_value(&mut self.selected_sensor, Some(sensor_id), label);
                    }
                });
        });
    }

    fn metric_tiles(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui_chrome::metric_tile(
                ui,
                "Sensors online",
                self.model.sensors.len(),
                "facility monitor",
            );
            ui_chrome::metric_tile_tone(
                ui,
                "Active alarms",
                self.model.active_alarms().len(),
                "latest samples",
                if self.model.active_alarms().is_empty() {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            );
            ui_chrome::metric_tile(
                ui,
                "Zones covered",
                self.zone_count(),
                "litho, CMP, wet bench",
            );
            ui_chrome::metric_tile(
                ui,
                "Correlation labels",
                self.model.correlations.len(),
                "process excursion tags",
            );
        });
    }

    fn selected_sensor_trend(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Historical Trend");
        let Some(sensor_id) = self.selected_sensor.as_deref() else {
            ui_chrome::empty_state(ui, "No environmental sensors loaded");
            return;
        };
        let Some(sensor) = self.sensor(sensor_id) else {
            ui_chrome::empty_state(ui, "Selected sensor is missing");
            return;
        };
        let readings = self.model.readings_for_sensor(sensor_id);

        ui.label(RichText::new(&sensor.name).strong());
        ui_chrome::muted(ui, format!("{} / {}", sensor.zone, sensor.kind.label()));
        if let Some(summary) = self.model.trend_summary(sensor_id) {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Latest {}",
                    format_reading(summary.latest_value, &sensor.unit)
                ));
                ui.label(format!(
                    "Avg {}",
                    format_reading(summary.average_value, &sensor.unit)
                ));
                ui.colored_label(
                    trend_color(summary.direction),
                    format!("Trend {}", summary.direction.label()),
                );
            });
            ui_chrome::muted(
                ui,
                format!(
                    "Range {}..{} {}, {} excursion(s)",
                    compact_number(summary.min_value),
                    compact_number(summary.max_value),
                    sensor.unit,
                    summary.excursion_count
                ),
            );
        }
        draw_environment_plot(ui, &readings, sensor);
        ui_chrome::muted(
            ui,
            format!("Nominal {}", sensor.thresholds.nominal_label(&sensor.unit)),
        );
    }

    fn alarm_panel(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Alarms");
        let active = self.model.active_alarms();
        if active.is_empty() {
            ui_chrome::status_pill(ui, "No active environmental alarms", Tone::Success);
        } else {
            for alarm in active.into_iter().take(5) {
                self.alarm_row(ui, alarm);
            }
        }

        ui.separator();
        ui_chrome::section_label(ui, "Recent Excursions");
        for alarm in self.model.alarms.iter().take(8) {
            self.alarm_row(ui, alarm);
        }
        if self.model.alarms.is_empty() {
            ui_chrome::empty_state(ui, "No historical threshold excursions");
        }
    }

    fn alarm_row(&self, ui: &mut egui::Ui, alarm: &EnvironmentAlarm) {
        let sensor = self.sensor(&alarm.sensor_id);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(severity_color(alarm.severity), alarm.severity.label());
            ui.label(format_time(alarm.timestamp_min));
            if let Some(sensor) = sensor {
                ui.label(format!(
                    "{} {}",
                    sensor.zone,
                    format_reading(alarm.value, &sensor.unit)
                ));
            }
            if alarm.active {
                ui_chrome::status_pill(ui, "active", Tone::Danger);
            }
        });
        ui_chrome::muted(ui, &alarm.message);
        if !alarm.correlation_labels.is_empty() {
            ui_chrome::muted(ui, format!("Tags: {}", alarm.correlation_labels.join(", ")));
        }
        ui.add_space(3.0);
    }

    fn zone_dashboard(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Live Sensor Dashboard");
        egui::Grid::new("environment_live_grid")
            .striped(true)
            .min_col_width(86.0)
            .show(ui, |ui| {
                ui.strong("Zone");
                ui.strong("Sensor");
                ui.strong("Latest");
                ui.strong("State");
                ui.end_row();
                for sensor in &self.model.sensors {
                    let latest = self.model.latest_reading(&sensor.id);
                    ui.label(&sensor.zone);
                    ui.label(&sensor.name);
                    ui.label(
                        latest
                            .map(|reading| format_reading(reading.value, &sensor.unit))
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    let severity =
                        latest.and_then(|reading| sensor.thresholds.evaluate(reading.value));
                    match severity {
                        Some(severity) => {
                            ui.colored_label(severity_color(severity), severity.label());
                        }
                        None => {
                            ui.colored_label(Tone::Success.color(), "Nominal");
                        }
                    }
                    ui.end_row();
                }
            });
    }

    fn facility_timeline(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Facility Event Timeline");
        for event in &self.model.events {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format_time(event.timestamp_min)).strong());
                ui.colored_label(event_color(event.kind), event.kind.label());
                ui.label(&event.zone);
            });
            ui.label(&event.title);
            ui_chrome::muted(ui, &event.detail);
            ui.add_space(4.0);
        }
        if self.model.events.is_empty() {
            ui_chrome::empty_state(ui, "No facility events loaded");
        }
    }

    fn sensor(&self, sensor_id: &str) -> Option<&EnvironmentSensor> {
        self.model
            .sensors
            .iter()
            .find(|sensor| sensor.id == sensor_id)
    }

    fn zone_count(&self) -> usize {
        let mut zones = self
            .model
            .sensors
            .iter()
            .map(|sensor| sensor.zone.as_str())
            .collect::<Vec<_>>();
        zones.sort_unstable();
        zones.dedup();
        zones.len()
    }
}

fn draw_environment_plot(
    ui: &mut egui::Ui,
    readings: &[&EnvironmentReading],
    sensor: &EnvironmentSensor,
) {
    let (rect, _) = ui.allocate_exact_size(ui_chrome::stable_plot_size(ui, 190.0), Sense::hover());
    let painter = ui.painter_at(rect);
    ui_chrome::plot_background(ui, rect);

    if readings.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no samples",
            FontId::proportional(14.0),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    let mut min_value = readings
        .iter()
        .map(|reading| reading.value)
        .fold(f64::INFINITY, f64::min);
    let mut max_value = readings
        .iter()
        .map(|reading| reading.value)
        .fold(f64::NEG_INFINITY, f64::max);
    for limit in [
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
        sensor.thresholds.critical_low,
        sensor.thresholds.critical_high,
    ]
    .into_iter()
    .flatten()
    {
        min_value = min_value.min(limit);
        max_value = max_value.max(limit);
    }
    let padding = ((max_value - min_value) * 0.12).max(0.5);
    min_value -= padding;
    max_value += padding;

    let min_x = readings
        .first()
        .map(|reading| reading.timestamp_min)
        .unwrap_or(0) as f32;
    let max_x = readings
        .last()
        .map(|reading| reading.timestamp_min)
        .unwrap_or(1) as f32;
    let plot = rect.shrink2(vec2(32.0, 18.0));
    let x_at = |x: u32| {
        let span = (max_x - min_x).max(1.0);
        plot.left() + ((x as f32 - min_x) / span).clamp(0.0, 1.0) * plot.width()
    };
    let y_at = |value: f64| {
        let span = (max_value - min_value).max(f64::EPSILON);
        let fraction = ((value - min_value) / span) as f32;
        plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height()
    };

    for limit in [
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
    ]
    .into_iter()
    .flatten()
    {
        let y = y_at(limit);
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.0, Tone::Warning.color()),
        );
    }
    for limit in [
        sensor.thresholds.critical_low,
        sensor.thresholds.critical_high,
    ]
    .into_iter()
    .flatten()
    {
        let y = y_at(limit);
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.2, Tone::Danger.color()),
        );
    }

    let points = readings
        .iter()
        .map(|reading| Pos2::new(x_at(reading.timestamp_min), y_at(reading.value)))
        .collect::<Vec<_>>();
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(1.8, Color32::from_rgb(96, 178, 255)),
        );
    }
    for (reading, point) in readings.iter().zip(points.iter()) {
        let color = sensor
            .thresholds
            .evaluate(reading.value)
            .map(severity_color)
            .unwrap_or_else(|| Color32::from_rgb(125, 210, 155));
        painter.circle_filled(*point, 3.0, color);
    }
}

fn severity_color(severity: EnvironmentAlarmSeverity) -> Color32 {
    match severity {
        EnvironmentAlarmSeverity::Advisory => Tone::Info.color(),
        EnvironmentAlarmSeverity::Warning => Tone::Warning.color(),
        EnvironmentAlarmSeverity::Critical => Tone::Danger.color(),
    }
}

fn trend_color(direction: TrendDirection) -> Color32 {
    match direction {
        TrendDirection::Falling => Color32::from_rgb(116, 186, 238),
        TrendDirection::Stable => Tone::Success.color(),
        TrendDirection::Rising => Tone::Warning.color(),
    }
}

fn event_color(kind: FacilityEventKind) -> Color32 {
    match kind {
        FacilityEventKind::Maintenance => Tone::Info.color(),
        FacilityEventKind::Excursion => Tone::Warning.color(),
        FacilityEventKind::Recovery => Tone::Success.color(),
        FacilityEventKind::ProcessHold => Tone::Danger.color(),
    }
}

fn format_reading(value: f64, unit: &str) -> String {
    format!("{} {}", compact_number(value), unit)
}

fn compact_number(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn format_time(timestamp_min: u32) -> String {
    format!("{:02}:{:02}", timestamp_min / 60, timestamp_min % 60)
}
