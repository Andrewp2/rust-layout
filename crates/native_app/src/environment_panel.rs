use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, vec2};
use layout_model::environment::{
    CleanroomEnvironment, EnvironmentAlarm, EnvironmentAlarmSeverity, EnvironmentReading,
    EnvironmentSensor, FacilityEventKind, TrendDirection,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct EnvironmentPanel {
    model: CleanroomEnvironment,
    selected_sensor: Option<String>,
}

#[derive(Default)]
struct ZoneSummary {
    name: String,
    sensor_count: usize,
    active_advisory: usize,
    active_warning: usize,
    active_critical: usize,
    excursion_count: usize,
    latest_timestamp_min: Option<u32>,
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
        let active = self.model.active_alarms();
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, self.facility_state_label(), self.facility_state_tone());
            ui.label(format!("{} sensor(s)", self.model.sensors.len()));
            ui.label(format!("{} zone(s)", self.zone_count()));
        });
        ui_chrome::muted(
            ui,
            format!(
                "{} reading(s), {} event(s), {} active alarm(s)",
                self.model.readings.len(),
                self.model.events.len(),
                active.len()
            ),
        );

        ui.separator();
        ui_chrome::section_label(ui, "Selected Sensor");
        if let Some(sensor) = self.selected_sensor_record() {
            let latest = self.model.latest_reading(&sensor.id);
            ui.label(RichText::new(&sensor.name).strong());
            ui_chrome::muted(ui, format!("{} / {}", sensor.zone, sensor.kind.label()));
            ui.horizontal_wrapped(|ui| {
                if let Some(reading) = latest {
                    let severity = sensor.thresholds.evaluate(reading.value);
                    ui.label(format!(
                        "Latest {} at {}",
                        format_reading(reading.value, &sensor.unit),
                        format_time(reading.timestamp_min)
                    ));
                    ui_chrome::status_pill(
                        ui,
                        sensor_state_label(severity),
                        severity_tone(severity),
                    );
                } else {
                    ui_chrome::status_pill(ui, "No samples", Tone::Neutral);
                }
            });
            ui_chrome::muted(
                ui,
                format!("Nominal {}", sensor.thresholds.nominal_label(&sensor.unit)),
            );
            if !sensor.process_tags.is_empty() {
                ui_chrome::muted(ui, format!("Tags: {}", sensor.process_tags.join(", ")));
            }
        } else {
            ui_chrome::empty_state(ui, "No environmental sensors loaded");
        }

        ui.separator();
        ui_chrome::section_label(ui, "Zone Posture");
        for summary in self.zone_summaries().into_iter().take(4) {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&summary.name).strong());
                ui_chrome::status_pill(
                    ui,
                    zone_state_label(&summary),
                    severity_tone(summary.worst_severity()),
                );
            });
            ui_chrome::muted(
                ui,
                format!(
                    "{} sensor(s), {} excursion(s), latest {}",
                    summary.sensor_count,
                    summary.excursion_count,
                    summary
                        .latest_timestamp_min
                        .map(format_time)
                        .unwrap_or_else(|| "--:--".to_string())
                ),
            );
        }

        ui.separator();
        ui_chrome::section_label(ui, "Process Correlations");
        let related = self.selected_sensor.as_deref().map_or_else(Vec::new, |id| {
            self.model
                .correlations
                .iter()
                .filter(|label| label.sensor_ids.iter().any(|sensor_id| sensor_id == id))
                .collect::<Vec<_>>()
        });
        let correlations = if related.is_empty() {
            self.model.correlations.iter().take(3).collect::<Vec<_>>()
        } else {
            related
        };
        for label in correlations {
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
                    "{} active alarm(s), {} historical excursion(s), {} facility event(s)",
                    self.model.active_alarms().len(),
                    self.model.alarms.len(),
                    self.model.events.len()
                );
                ui_chrome::module_header(
                    ui,
                    "Facility operations",
                    "Cleanroom Environment",
                    &detail,
                    |_| {},
                );

                self.status_overview(ui);
                self.sensor_picker(ui);
                ui.add_space(4.0);
                self.metric_tiles(ui);

                ui.separator();
                if ui.available_width() < 880.0 {
                    self.selected_sensor_trend(ui);
                    ui.separator();
                    self.alarm_panel(ui);
                } else {
                    ui.columns(2, |columns| {
                        self.selected_sensor_trend(&mut columns[0]);
                        self.alarm_panel(&mut columns[1]);
                    });
                }

                ui.separator();
                self.sensor_trend_matrix(ui);

                ui.separator();
                if ui.available_width() < 920.0 {
                    self.zone_dashboard(ui);
                    ui.separator();
                    self.facility_timeline(ui);
                } else {
                    ui.columns(2, |columns| {
                        self.zone_dashboard(&mut columns[0]);
                        self.facility_timeline(&mut columns[1]);
                    });
                }
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

    fn status_overview(&self, ui: &mut egui::Ui) {
        let summaries = self.zone_summaries();
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, self.facility_state_label(), self.facility_state_tone());
            ui.label(format!("{} monitored zone(s)", summaries.len()));
            ui.label(format!("{} sensor reading(s)", self.model.readings.len()));
            if let Some(last_event) = self
                .model
                .events
                .iter()
                .max_by_key(|event| event.timestamp_min)
            {
                ui.colored_label(
                    event_color(last_event.kind),
                    format!(
                        "Latest event {} {}",
                        format_time(last_event.timestamp_min),
                        last_event.zone
                    ),
                );
            }
        });
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
        let active_critical = self
            .model
            .active_alarms()
            .into_iter()
            .filter(|alarm| alarm.severity == EnvironmentAlarmSeverity::Critical)
            .count();
        let metrics = [
            (
                "Sensors online",
                self.model.sensors.len().to_string(),
                "continuous monitor",
                Tone::Neutral,
            ),
            (
                "Active excursions",
                self.model.active_alarms().len().to_string(),
                "latest sample state",
                self.facility_state_tone(),
            ),
            (
                "Critical holds",
                active_critical.to_string(),
                "needs disposition",
                if active_critical == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            ),
            (
                "Correlation tags",
                self.model.correlations.len().to_string(),
                "process risk labels",
                Tone::Neutral,
            ),
        ];
        ui_chrome::metric_tiles(ui, &metrics);
    }

    fn selected_sensor_trend(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Selected Sensor Trend");
        let Some(sensor_id) = self.selected_sensor.as_deref() else {
            ui_chrome::empty_state(ui, "No environmental sensors loaded");
            return;
        };
        let Some(sensor) = self.sensor(sensor_id) else {
            ui_chrome::empty_state(ui, "Selected sensor is missing");
            return;
        };
        let readings = self.model.readings_for_sensor(sensor_id);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&sensor.name).strong());
            if let Some(reading) = self.model.latest_reading(sensor_id) {
                ui_chrome::status_pill(
                    ui,
                    sensor_state_label(sensor.thresholds.evaluate(reading.value)),
                    severity_tone(sensor.thresholds.evaluate(reading.value)),
                );
            }
        });
        ui_chrome::muted(
            ui,
            format!(
                "{} / {} / nominal {}",
                sensor.zone,
                sensor.kind.label(),
                sensor.thresholds.nominal_label(&sensor.unit)
            ),
        );
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
        self.spec_band_legend(ui, sensor);
        if !sensor.process_tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(Tone::Info.color(), "Process tags");
                for tag in &sensor.process_tags {
                    wrapped_tag(ui, tag);
                }
            });
        }
    }

    fn alarm_panel(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Alarm Console");
        let (critical, warning, advisory) = self.alarm_counts(self.model.alarms.iter());
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                severity_color(EnvironmentAlarmSeverity::Critical),
                format!("{critical} critical"),
            );
            ui.colored_label(
                severity_color(EnvironmentAlarmSeverity::Warning),
                format!("{warning} warning"),
            );
            ui.colored_label(
                severity_color(EnvironmentAlarmSeverity::Advisory),
                format!("{advisory} advisory"),
            );
        });

        let active = self.model.active_alarms();
        ui_chrome::section_label(ui, "Active Now");
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
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(1.0, severity_color(alarm.severity)))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(9, 7))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui_chrome::status_pill(
                        ui,
                        alarm.severity.label(),
                        severity_tone(Some(alarm.severity)),
                    );
                    ui.label(format_time(alarm.timestamp_min));
                    if let Some(sensor) = sensor {
                        ui.label(format!(
                            "{} / {}",
                            sensor.zone,
                            format_reading(alarm.value, &sensor.unit)
                        ));
                    }
                    if alarm.active {
                        ui_chrome::status_pill(ui, "active", Tone::Danger);
                    }
                });
                ui.add(
                    egui::Label::new(
                        RichText::new(&alarm.message).color(ui.visuals().text_color()),
                    )
                    .wrap(),
                );
                ui_chrome::muted(ui, recommended_alarm_action(alarm.severity));
                if !alarm.correlation_labels.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(Tone::Info.color(), "Tags");
                        for tag in &alarm.correlation_labels {
                            wrapped_tag(ui, tag);
                        }
                    });
                }
            });
        ui.add_space(5.0);
    }

    fn zone_dashboard(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Zone Status");
        let summaries = self.zone_summaries();
        if summaries.is_empty() {
            ui_chrome::empty_state(ui, "No cleanroom zones loaded");
            return;
        }

        let width = ui.available_width();
        let columns = if width >= 1060.0 {
            3
        } else if width >= 700.0 {
            2
        } else {
            1
        };
        ui.columns(columns, |columns_ui| {
            for (index, summary) in summaries.iter().enumerate() {
                let column = &mut columns_ui[index % columns];
                self.zone_card(column, summary);
            }
        });
    }

    fn sensor_trend_matrix(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Sensor Trend Matrix");
        if self.model.sensors.is_empty() {
            ui_chrome::empty_state(ui, "No environmental sensors loaded");
            return;
        }

        let mut pending_selection = None;
        for sensor in &self.model.sensors {
            let selected = self
                .selected_sensor
                .as_deref()
                .is_some_and(|id| id == sensor.id);
            let inner = egui::Frame::new()
                .fill(if selected {
                    ui.visuals().selection.bg_fill.linear_multiply(0.35)
                } else {
                    ui.visuals().faint_bg_color
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        Tone::Info.color()
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    },
                ))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(9, 7))
                .show(ui, |ui| {
                    self.sensor_trend_row(ui, sensor);
                });
            let response = ui.interact(
                inner.response.rect,
                ui.make_persistent_id(("environment_sensor_trend_row", sensor.id.as_str())),
                Sense::click(),
            );
            if response.clicked() {
                pending_selection = Some(sensor.id.clone());
            }
            ui.add_space(5.0);
        }
        if let Some(sensor_id) = pending_selection {
            self.selected_sensor = Some(sensor_id);
        }
    }

    fn sensor_trend_row(&self, ui: &mut egui::Ui, sensor: &EnvironmentSensor) {
        let latest = self.model.latest_reading(&sensor.id);
        let severity = latest.and_then(|reading| sensor.thresholds.evaluate(reading.value));
        let summary = self.model.trend_summary(&sensor.id);
        let readings = self.model.readings_for_sensor(&sensor.id);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&sensor.zone).strong());
            ui.label(&sensor.name);
            ui_chrome::status_pill(ui, sensor_state_label(severity), severity_tone(severity));
            if let Some(summary) = &summary {
                ui.colored_label(
                    trend_color(summary.direction),
                    format!("Trend {}", summary.direction.label()),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            if let Some(reading) = latest {
                ui.label(format!(
                    "Latest {} at {}",
                    format_reading(reading.value, &sensor.unit),
                    format_time(reading.timestamp_min)
                ));
            } else {
                ui.label("No samples");
            }
            if let Some(summary) = &summary {
                ui.label(format!(
                    "Range {}..{} {}",
                    compact_number(summary.min_value),
                    compact_number(summary.max_value),
                    sensor.unit
                ));
                ui.label(format!("{} excursion(s)", summary.excursion_count));
            }
        });
        if ui.available_width() >= 360.0 {
            draw_sensor_sparkline(ui, &readings, sensor);
        }
        ui_chrome::muted(
            ui,
            format!("Spec {}", sensor.thresholds.nominal_label(&sensor.unit)),
        );
    }

    fn zone_card(&self, ui: &mut egui::Ui, summary: &ZoneSummary) {
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(
                1.0,
                summary
                    .worst_severity()
                    .map(severity_color)
                    .unwrap_or(ui.visuals().widgets.noninteractive.bg_stroke.color),
            ))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&summary.name).strong());
                    ui_chrome::status_pill(
                        ui,
                        zone_state_label(summary),
                        severity_tone(summary.worst_severity()),
                    );
                });
                ui_chrome::muted(
                    ui,
                    format!(
                        "{} sensor(s), {} historical excursion(s), latest {}",
                        summary.sensor_count,
                        summary.excursion_count,
                        summary
                            .latest_timestamp_min
                            .map(format_time)
                            .unwrap_or_else(|| "--:--".to_string())
                    ),
                );
                ui.add_space(3.0);
                for sensor in self
                    .model
                    .sensors
                    .iter()
                    .filter(|sensor| sensor.zone == summary.name)
                {
                    self.zone_sensor_line(ui, sensor);
                }
            });
        ui.add_space(7.0);
    }

    fn zone_sensor_line(&self, ui: &mut egui::Ui, sensor: &EnvironmentSensor) {
        let latest = self.model.latest_reading(&sensor.id);
        let severity = latest.and_then(|reading| sensor.thresholds.evaluate(reading.value));
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                severity_tone(severity).color(),
                sensor_state_label(severity),
            );
            ui.label(&sensor.name);
            if let Some(reading) = latest {
                ui.label(format_reading(reading.value, &sensor.unit));
            } else {
                ui.label("-");
            }
        });
    }

    fn facility_timeline(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Facility Event Timeline");
        for event in &self.model.events {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .stroke(Stroke::new(1.0, event_color(event.kind)))
                .corner_radius(6)
                .inner_margin(egui::Margin::symmetric(9, 7))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format_time(event.timestamp_min)).strong());
                        ui.colored_label(event_color(event.kind), event.kind.label());
                        ui.label(&event.zone);
                        if event.linked_alarm_id.is_some() {
                            ui_chrome::status_pill(ui, "linked alarm", Tone::Neutral);
                        }
                    });
                    ui.add(egui::Label::new(RichText::new(&event.title).strong()).wrap());
                    ui_chrome::muted(ui, &event.detail);
                });
            ui.add_space(5.0);
        }
        if self.model.events.is_empty() {
            ui_chrome::empty_state(ui, "No facility events loaded");
        }
    }

    fn spec_band_legend(&self, ui: &mut egui::Ui, sensor: &EnvironmentSensor) {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(Tone::Success.color(), "Nominal");
            ui.label(sensor.thresholds.nominal_label(&sensor.unit));

            if sensor.thresholds.warning_low.is_some() || sensor.thresholds.warning_high.is_some() {
                ui.colored_label(Tone::Warning.color(), "Warning");
                ui.label(threshold_edge_label(
                    sensor.thresholds.warning_low,
                    sensor.thresholds.warning_high,
                    &sensor.unit,
                ));
            }

            if sensor.thresholds.critical_low.is_some() || sensor.thresholds.critical_high.is_some()
            {
                ui.colored_label(Tone::Danger.color(), "Critical");
                ui.label(threshold_edge_label(
                    sensor.thresholds.critical_low,
                    sensor.thresholds.critical_high,
                    &sensor.unit,
                ));
            }
        });
    }

    fn selected_sensor_record(&self) -> Option<&EnvironmentSensor> {
        self.selected_sensor
            .as_deref()
            .and_then(|sensor_id| self.sensor(sensor_id))
    }

    fn alarm_counts<'a>(
        &self,
        alarms: impl Iterator<Item = &'a EnvironmentAlarm>,
    ) -> (usize, usize, usize) {
        let mut critical = 0;
        let mut warning = 0;
        let mut advisory = 0;
        for alarm in alarms {
            match alarm.severity {
                EnvironmentAlarmSeverity::Critical => critical += 1,
                EnvironmentAlarmSeverity::Warning => warning += 1,
                EnvironmentAlarmSeverity::Advisory => advisory += 1,
            }
        }
        (critical, warning, advisory)
    }

    fn facility_state_label(&self) -> &'static str {
        match self.active_worst_severity() {
            Some(EnvironmentAlarmSeverity::Critical) => "Critical excursion",
            Some(EnvironmentAlarmSeverity::Warning) => "Warning excursion",
            Some(EnvironmentAlarmSeverity::Advisory) => "Advisory",
            None => "In spec",
        }
    }

    fn facility_state_tone(&self) -> Tone {
        severity_tone(self.active_worst_severity())
    }

    fn active_worst_severity(&self) -> Option<EnvironmentAlarmSeverity> {
        self.model
            .active_alarms()
            .into_iter()
            .map(|alarm| alarm.severity)
            .max()
    }

    fn zone_summaries(&self) -> Vec<ZoneSummary> {
        let mut summaries = BTreeMap::<String, ZoneSummary>::new();
        for sensor in &self.model.sensors {
            let entry = summaries
                .entry(sensor.zone.clone())
                .or_insert_with(|| ZoneSummary {
                    name: sensor.zone.clone(),
                    ..Default::default()
                });
            entry.sensor_count += 1;
            entry.excursion_count += self
                .model
                .alarms
                .iter()
                .filter(|alarm| alarm.sensor_id == sensor.id)
                .count();
            if let Some(reading) = self.model.latest_reading(&sensor.id) {
                entry.latest_timestamp_min = Some(
                    entry
                        .latest_timestamp_min
                        .map_or(reading.timestamp_min, |current| {
                            current.max(reading.timestamp_min)
                        }),
                );
                match sensor.thresholds.evaluate(reading.value) {
                    Some(EnvironmentAlarmSeverity::Critical) => entry.active_critical += 1,
                    Some(EnvironmentAlarmSeverity::Warning) => entry.active_warning += 1,
                    Some(EnvironmentAlarmSeverity::Advisory) => entry.active_advisory += 1,
                    None => {}
                }
            }
        }
        let mut summaries = summaries.into_values().collect::<Vec<_>>();
        summaries.sort_by(|left, right| {
            right
                .worst_severity()
                .cmp(&left.worst_severity())
                .then_with(|| right.excursion_count.cmp(&left.excursion_count))
                .then_with(|| left.name.cmp(&right.name))
        });
        summaries
    }

    fn sensor(&self, sensor_id: &str) -> Option<&EnvironmentSensor> {
        self.model
            .sensors
            .iter()
            .find(|sensor| sensor.id == sensor_id)
    }

    fn zone_count(&self) -> usize {
        self.model
            .sensors
            .iter()
            .map(|sensor| sensor.zone.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

impl ZoneSummary {
    fn worst_severity(&self) -> Option<EnvironmentAlarmSeverity> {
        if self.active_critical > 0 {
            Some(EnvironmentAlarmSeverity::Critical)
        } else if self.active_warning > 0 {
            Some(EnvironmentAlarmSeverity::Warning)
        } else if self.active_advisory > 0 {
            Some(EnvironmentAlarmSeverity::Advisory)
        } else {
            None
        }
    }
}

fn draw_environment_plot(
    ui: &mut egui::Ui,
    readings: &[&EnvironmentReading],
    sensor: &EnvironmentSensor,
) {
    let (rect, _) = ui.allocate_exact_size(environment_plot_size(ui, 210.0), Sense::hover());
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
    let horizontal_margin = if rect.width() < 260.0 { 22.0 } else { 38.0 };
    let plot = rect.shrink2(vec2(horizontal_margin, 22.0));
    let x_at = |x: u32| {
        let span = (max_x - min_x).max(1.0);
        plot.left() + ((x as f32 - min_x) / span).clamp(0.0, 1.0) * plot.width()
    };
    let y_at = |value: f64| {
        let span = (max_value - min_value).max(f64::EPSILON);
        let fraction = ((value - min_value) / span) as f32;
        plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height()
    };

    draw_threshold_bands(&painter, plot, sensor, &y_at);
    draw_plot_grid(
        &painter,
        plot,
        ui.visuals().widgets.noninteractive.bg_stroke.color,
    );

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

    for reading in readings
        .iter()
        .filter(|reading| sensor.thresholds.evaluate(reading.value).is_some())
    {
        let x = x_at(reading.timestamp_min);
        painter.line_segment(
            [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
            Stroke::new(
                0.8,
                severity_color(sensor.thresholds.evaluate(reading.value).unwrap())
                    .linear_multiply(0.65),
            ),
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

    if let Some((latest, point)) = readings.last().zip(points.last()) {
        painter.circle_stroke(*point, 5.5, Stroke::new(1.4, Color32::WHITE));
        painter.text(
            Pos2::new(plot.right(), plot.top()),
            Align2::RIGHT_TOP,
            format_reading(latest.value, &sensor.unit),
            FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
    }

    painter.text(
        Pos2::new(plot.left(), plot.bottom() + 5.0),
        Align2::LEFT_TOP,
        readings
            .first()
            .map(|reading| format_time(reading.timestamp_min))
            .unwrap_or_else(|| "--:--".to_string()),
        FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
    painter.text(
        Pos2::new(plot.right(), plot.bottom() + 5.0),
        Align2::RIGHT_TOP,
        readings
            .last()
            .map(|reading| format_time(reading.timestamp_min))
            .unwrap_or_else(|| "--:--".to_string()),
        FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
}

fn draw_sensor_sparkline(
    ui: &mut egui::Ui,
    readings: &[&EnvironmentReading],
    sensor: &EnvironmentSensor,
) {
    let width = ui.available_width().min(460.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

    if readings.len() < 2 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no trend",
            FontId::proportional(11.0),
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
    let padding = ((max_value - min_value) * 0.12).max(0.1);
    min_value -= padding;
    max_value += padding;

    let min_x = readings.first().unwrap().timestamp_min as f32;
    let max_x = readings.last().unwrap().timestamp_min as f32;
    let plot = rect.shrink2(vec2(8.0, 6.0));
    let points = readings
        .iter()
        .map(|reading| {
            let x_fraction =
                ((reading.timestamp_min as f32 - min_x) / (max_x - min_x).max(1.0)).clamp(0.0, 1.0);
            let y_fraction =
                ((reading.value - min_value) / (max_value - min_value).max(f64::EPSILON)) as f32;
            Pos2::new(
                plot.left() + x_fraction * plot.width(),
                plot.bottom() - y_fraction.clamp(0.0, 1.0) * plot.height(),
            )
        })
        .collect::<Vec<_>>();
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(1.4, Color32::from_rgb(96, 178, 255)),
        );
    }
    for (reading, point) in readings.iter().zip(points.iter()) {
        let color = sensor
            .thresholds
            .evaluate(reading.value)
            .map(severity_color)
            .unwrap_or_else(|| Tone::Success.color());
        painter.circle_filled(*point, 2.3, color);
    }
}

fn environment_plot_size(ui: &egui::Ui, height: f32) -> egui::Vec2 {
    let available = ui
        .available_width()
        .min(ui.ctx().content_rect().width())
        .max(64.0);
    vec2(available.min(680.0), height)
}

fn draw_plot_grid(painter: &egui::Painter, plot: Rect, color: Color32) {
    let grid_color = color.linear_multiply(0.45);
    for index in 1..4 {
        let fraction = index as f32 / 4.0;
        let y = egui::lerp(plot.top()..=plot.bottom(), fraction);
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(0.7, grid_color),
        );
    }
}

fn draw_threshold_bands(
    painter: &egui::Painter,
    plot: Rect,
    sensor: &EnvironmentSensor,
    y_at: &impl Fn(f64) -> f32,
) {
    if let (Some(low), Some(high)) = (
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
    ) {
        fill_value_band(
            painter,
            plot,
            y_at,
            low,
            high,
            Color32::from_rgba_unmultiplied(93, 184, 132, 32),
        );
    } else if let Some(high) = sensor.thresholds.warning_high {
        fill_open_band(
            painter,
            plot,
            y_at,
            None,
            Some(high),
            Color32::from_rgba_unmultiplied(93, 184, 132, 24),
        );
    } else if let Some(low) = sensor.thresholds.warning_low {
        fill_open_band(
            painter,
            plot,
            y_at,
            Some(low),
            None,
            Color32::from_rgba_unmultiplied(93, 184, 132, 24),
        );
    }

    draw_limit_zone(
        painter,
        plot,
        y_at,
        sensor.thresholds.warning_low,
        sensor.thresholds.critical_low,
        false,
        Color32::from_rgba_unmultiplied(220, 176, 72, 24),
    );
    draw_limit_zone(
        painter,
        plot,
        y_at,
        sensor.thresholds.warning_high,
        sensor.thresholds.critical_high,
        true,
        Color32::from_rgba_unmultiplied(220, 176, 72, 24),
    );
    draw_limit_zone(
        painter,
        plot,
        y_at,
        sensor.thresholds.critical_low,
        None,
        false,
        Color32::from_rgba_unmultiplied(226, 96, 96, 26),
    );
    draw_limit_zone(
        painter,
        plot,
        y_at,
        sensor.thresholds.critical_high,
        None,
        true,
        Color32::from_rgba_unmultiplied(226, 96, 96, 26),
    );
}

fn draw_limit_zone(
    painter: &egui::Painter,
    plot: Rect,
    y_at: &impl Fn(f64) -> f32,
    boundary: Option<f64>,
    outer: Option<f64>,
    high_side: bool,
    color: Color32,
) {
    let Some(boundary) = boundary else {
        return;
    };
    let boundary_y = y_at(boundary).clamp(plot.top(), plot.bottom());
    let outer_y = outer
        .map(|value| y_at(value).clamp(plot.top(), plot.bottom()))
        .unwrap_or(if high_side { plot.top() } else { plot.bottom() });
    let (top, bottom) = if high_side {
        (outer_y.min(boundary_y), boundary_y.max(outer_y))
    } else {
        (boundary_y.min(outer_y), outer_y.max(boundary_y))
    };
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(plot.left(), top), Pos2::new(plot.right(), bottom)),
        0.0,
        color,
    );
}

fn fill_open_band(
    painter: &egui::Painter,
    plot: Rect,
    y_at: &impl Fn(f64) -> f32,
    low: Option<f64>,
    high: Option<f64>,
    color: Color32,
) {
    let top = high
        .map(|value| y_at(value).clamp(plot.top(), plot.bottom()))
        .unwrap_or(plot.top());
    let bottom = low
        .map(|value| y_at(value).clamp(plot.top(), plot.bottom()))
        .unwrap_or(plot.bottom());
    if top <= bottom {
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(plot.left(), top), Pos2::new(plot.right(), bottom)),
            0.0,
            color,
        );
    }
}

fn fill_value_band(
    painter: &egui::Painter,
    plot: Rect,
    y_at: &impl Fn(f64) -> f32,
    low: f64,
    high: f64,
    color: Color32,
) {
    let top = y_at(high).clamp(plot.top(), plot.bottom());
    let bottom = y_at(low).clamp(plot.top(), plot.bottom());
    if top <= bottom {
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(plot.left(), top), Pos2::new(plot.right(), bottom)),
            0.0,
            color,
        );
    }
}

fn severity_color(severity: EnvironmentAlarmSeverity) -> Color32 {
    match severity {
        EnvironmentAlarmSeverity::Advisory => Tone::Info.color(),
        EnvironmentAlarmSeverity::Warning => Tone::Warning.color(),
        EnvironmentAlarmSeverity::Critical => Tone::Danger.color(),
    }
}

fn severity_tone(severity: Option<EnvironmentAlarmSeverity>) -> Tone {
    match severity {
        Some(EnvironmentAlarmSeverity::Advisory) => Tone::Info,
        Some(EnvironmentAlarmSeverity::Warning) => Tone::Warning,
        Some(EnvironmentAlarmSeverity::Critical) => Tone::Danger,
        None => Tone::Success,
    }
}

fn sensor_state_label(severity: Option<EnvironmentAlarmSeverity>) -> &'static str {
    match severity {
        Some(EnvironmentAlarmSeverity::Advisory) => "Advisory",
        Some(EnvironmentAlarmSeverity::Warning) => "Warning",
        Some(EnvironmentAlarmSeverity::Critical) => "Critical",
        None => "Nominal",
    }
}

fn zone_state_label(summary: &ZoneSummary) -> &'static str {
    match summary.worst_severity() {
        Some(EnvironmentAlarmSeverity::Critical) => "Critical",
        Some(EnvironmentAlarmSeverity::Warning) => "Warning",
        Some(EnvironmentAlarmSeverity::Advisory) => "Advisory",
        None => "Nominal",
    }
}

fn recommended_alarm_action(severity: EnvironmentAlarmSeverity) -> &'static str {
    match severity {
        EnvironmentAlarmSeverity::Critical => {
            "Action: hold affected lots, page facilities, and verify recovery before release."
        }
        EnvironmentAlarmSeverity::Warning => {
            "Action: watch next samples and check correlated process monitors."
        }
        EnvironmentAlarmSeverity::Advisory => {
            "Action: annotate the run log and keep the sensor under review."
        }
    }
}

fn threshold_edge_label(low: Option<f64>, high: Option<f64>, unit: &str) -> String {
    match (low, high) {
        (Some(low), Some(high)) => format!(
            "< {} or > {} {}",
            compact_number(low),
            compact_number(high),
            unit
        ),
        (Some(low), None) => format!("< {} {}", compact_number(low), unit),
        (None, Some(high)) => format!("> {} {}", compact_number(high), unit),
        (None, None) => "unbounded".to_string(),
    }
}

fn wrapped_tag(ui: &mut egui::Ui, tag: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(tag)
                .small()
                .color(ui.visuals().weak_text_color()),
        )
        .wrap(),
    );
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
