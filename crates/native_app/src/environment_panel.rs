use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, vec2};
use layout_model::environment::{
    CleanroomEnvironment, EnvironmentAlarm, EnvironmentAlarmSeverity, EnvironmentReading,
    EnvironmentSensor, FacilityEventKind, TrendDirection,
};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{
    operad_egui,
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar},
    ui_chrome::{self, Tone},
};

const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SELECT_SENSOR: &str = "environment.action.select_sensor.";

pub(crate) struct EnvironmentPanel {
    model: CleanroomEnvironment,
    selected_sensor: Option<String>,
}

#[derive(Debug)]
struct EnvironmentOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct EnvironmentMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct EnvironmentOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
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
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        let active = self.model.active_alarms();
        let mut sections = vec![
            SidecarSection::new("Cleanroom Environment")
                .row(SidecarRow::new(
                    self.facility_state_label(),
                    format!(
                        "{} sensors | {} zones | {} active alarms",
                        self.model.sensors.len(),
                        self.zone_count(),
                        active.len()
                    ),
                    self.facility_state_tone(),
                ))
                .row(SidecarRow::new(
                    "Readings / events",
                    format!(
                        "{} readings | {} events",
                        self.model.readings.len(),
                        self.model.events.len()
                    ),
                    Tone::Neutral,
                )),
        ];
        if let Some(sensor) = self.selected_sensor_record() {
            let latest = self.model.latest_reading(&sensor.id);
            let latest_detail = latest
                .map(|reading| {
                    format!(
                        "{} at {}",
                        format_reading(reading.value, &sensor.unit),
                        format_time(reading.timestamp_min)
                    )
                })
                .unwrap_or_else(|| "No samples".to_string());
            let severity = latest.map(|reading| sensor.thresholds.evaluate(reading.value));
            sections.push(
                SidecarSection::new("Selected Sensor")
                    .row(
                        SidecarRow::new(
                            &sensor.name,
                            format!("{} | {}", sensor.zone, sensor.kind.label()),
                            severity.map_or(Tone::Neutral, severity_tone),
                        )
                        .selected(true),
                    )
                    .row(SidecarRow::new(
                        severity.map(sensor_state_label).unwrap_or("No samples"),
                        latest_detail,
                        severity.map_or(Tone::Neutral, severity_tone),
                    ))
                    .row(SidecarRow::new(
                        "Nominal",
                        sensor.thresholds.nominal_label(&sensor.unit),
                        Tone::Neutral,
                    )),
            );
        }
        render_sidecar(ui, "environment.context", &sections)
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
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
        if let Err(error) = self.operad_ui(ui) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("environment_dashboard_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width);
                if let Err(error) = view
                    .document
                    .compute_layout(view.size, &mut ApproxTextMeasurer)
                    .map_err(|error| error.to_string())
                {
                    result = Err(error);
                    return;
                }

                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(node_name) =
                        operad_egui::hit_test_name(&view.document, rect, pointer)
                    && self.handle_operad_action(&node_name)
                {
                    view = self.build_operad_view(width);
                    if let Err(error) = view
                        .document
                        .compute_layout(view.size, &mut ApproxTextMeasurer)
                        .map_err(|error| error.to_string())
                    {
                        result = Err(error);
                        return;
                    }
                }

                if response.hovered()
                    && let Some(pointer) = ui.ctx().pointer_hover_pos()
                    && operad_egui::hit_test_name(&view.document, rect, pointer).is_some()
                {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
                }
                operad_egui::paint_document_at(ui, &view.document, rect);
            });
        result
    }

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui) {
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

    fn build_operad_view(&self, width: f32) -> EnvironmentOperadView {
        let metrics = self.operad_metrics();
        let overview_rows = self.operad_overview_rows();
        let selected_rows = self.operad_selected_sensor_rows();
        let alarm_rows = self.operad_alarm_rows();
        let sensor_rows = self.operad_sensor_rows();
        let zone_rows = self.operad_zone_rows();
        let event_rows = self.operad_event_rows();
        let correlation_rows = self.operad_correlation_rows();
        let height = environment_operad_view_height(
            width,
            metrics.len(),
            &[
                overview_rows.len(),
                selected_rows.len(),
                alarm_rows.len(),
                sensor_rows.len(),
                zone_rows.len(),
                event_rows.len(),
                correlation_rows.len(),
            ],
        );
        let size = UiSize::new(width, height);
        let mut document = UiDocument::new(root_style(width, height));
        let root = document.root;
        document.set_node_visual(
            root,
            UiVisual::panel(
                ColorRgba::new(15, 18, 21, 255),
                Some(StrokeStyle::new(ColorRgba::new(39, 46, 52, 255), 1.0)),
                0.0,
            ),
        );

        add_environment_operad_header(
            &mut document,
            root,
            "FACILITY OPERATIONS",
            "Cleanroom Environment",
            "Environmental sensors, excursions, zone posture, and process correlations",
            &format!(
                "{} active alarm(s), {} historical excursion(s), {} facility event(s)",
                self.model.active_alarms().len(),
                self.model.alarms.len(),
                self.model.events.len()
            ),
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_metric_grid(&mut document, root, width, &metrics);
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.overview",
            "Status Overview",
            "No cleanroom status loaded",
            &overview_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.selected_sensor",
            "Selected Sensor Trend",
            "No environmental sensors loaded",
            &selected_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.alarms",
            "Alarm Console",
            "No active or historical environmental alarms",
            &alarm_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.sensors",
            "Sensor Trend Matrix",
            "No environmental sensors loaded",
            &sensor_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.zones",
            "Zone Status",
            "No cleanroom zones loaded",
            &zone_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.events",
            "Facility Event Timeline",
            "No facility events loaded",
            &event_rows,
        );
        add_environment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_environment_operad_section(
            &mut document,
            root,
            width,
            "environment.correlations",
            "Process Correlations",
            "No process correlation labels loaded",
            &correlation_rows,
        );

        EnvironmentOperadView { document, size }
    }

    fn operad_metrics(&self) -> Vec<EnvironmentMetricTile> {
        let active_critical = self
            .model
            .active_alarms()
            .into_iter()
            .filter(|alarm| alarm.severity == EnvironmentAlarmSeverity::Critical)
            .count();
        vec![
            EnvironmentMetricTile {
                label: "Sensors online".to_string(),
                value: self.model.sensors.len().to_string(),
                detail: "continuous monitor".to_string(),
                tone: Tone::Neutral,
            },
            EnvironmentMetricTile {
                label: "Active excursions".to_string(),
                value: self.model.active_alarms().len().to_string(),
                detail: "latest sample state".to_string(),
                tone: self.facility_state_tone(),
            },
            EnvironmentMetricTile {
                label: "Critical holds".to_string(),
                value: active_critical.to_string(),
                detail: "needs disposition".to_string(),
                tone: if active_critical == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            },
            EnvironmentMetricTile {
                label: "Correlation tags".to_string(),
                value: self.model.correlations.len().to_string(),
                detail: "process risk labels".to_string(),
                tone: Tone::Neutral,
            },
        ]
    }

    fn operad_overview_rows(&self) -> Vec<EnvironmentOperadRow> {
        let mut rows = vec![EnvironmentOperadRow {
            title: self.facility_state_label().to_string(),
            detail: format!(
                "{} monitored zone(s), {} sensor reading(s)",
                self.zone_count(),
                self.model.readings.len()
            ),
            tone: self.facility_state_tone(),
            action_name: None,
            selected: false,
        }];
        if let Some(last_event) = self
            .model
            .events
            .iter()
            .max_by_key(|event| event.timestamp_min)
        {
            rows.push(EnvironmentOperadRow {
                title: format!("Latest event · {}", last_event.kind.label()),
                detail: format!(
                    "{} · {} · {}",
                    format_time(last_event.timestamp_min),
                    last_event.zone,
                    last_event.title
                ),
                tone: event_tone(last_event.kind),
                action_name: None,
                selected: false,
            });
        }
        rows
    }

    fn operad_selected_sensor_rows(&self) -> Vec<EnvironmentOperadRow> {
        let Some(sensor) = self.selected_sensor_record() else {
            return Vec::new();
        };
        let readings = self.model.readings_for_sensor(&sensor.id);
        let latest = self.model.latest_reading(&sensor.id);
        let severity = latest.and_then(|reading| sensor.thresholds.evaluate(reading.value));
        let mut rows = vec![EnvironmentOperadRow {
            title: sensor.name.clone(),
            detail: format!(
                "{} · {} · nominal {}",
                sensor.zone,
                sensor.kind.label(),
                sensor.thresholds.nominal_label(&sensor.unit)
            ),
            tone: severity_tone(severity),
            action_name: Some(format!(
                "{OPERAD_ACTION_SELECT_SENSOR}{}|selected",
                sensor.id
            )),
            selected: true,
        }];
        if let Some(reading) = latest {
            rows.push(EnvironmentOperadRow {
                title: "Latest sample".to_string(),
                detail: format!(
                    "{} at {}",
                    format_reading(reading.value, &sensor.unit),
                    format_time(reading.timestamp_min)
                ),
                tone: severity_tone(severity),
                action_name: None,
                selected: false,
            });
        }
        if let Some(summary) = self.model.trend_summary(&sensor.id) {
            rows.push(EnvironmentOperadRow {
                title: format!("Trend {}", summary.direction.label()),
                detail: format!(
                    "latest {}, avg {}, range {}..{} {}, {} excursion(s)",
                    format_reading(summary.latest_value, &sensor.unit),
                    format_reading(summary.average_value, &sensor.unit),
                    compact_number(summary.min_value),
                    compact_number(summary.max_value),
                    sensor.unit,
                    summary.excursion_count
                ),
                tone: trend_tone(summary.direction),
                action_name: None,
                selected: false,
            });
        }
        rows.push(EnvironmentOperadRow {
            title: "Spec band".to_string(),
            detail: sensor.thresholds.nominal_label(&sensor.unit),
            tone: Tone::Neutral,
            action_name: None,
            selected: false,
        });
        if !sensor.process_tags.is_empty() {
            rows.push(EnvironmentOperadRow {
                title: "Process tags".to_string(),
                detail: sensor.process_tags.join(", "),
                tone: Tone::Info,
                action_name: None,
                selected: false,
            });
        }
        if readings.is_empty() {
            rows.push(EnvironmentOperadRow {
                title: "Trend samples".to_string(),
                detail: "No samples loaded".to_string(),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        rows
    }

    fn operad_alarm_rows(&self) -> Vec<EnvironmentOperadRow> {
        let mut rows = Vec::new();
        let (critical, warning, advisory) = self.alarm_counts(self.model.alarms.iter());
        rows.push(EnvironmentOperadRow {
            title: "Alarm counts".to_string(),
            detail: format!("{critical} critical, {warning} warning, {advisory} advisory"),
            tone: if critical > 0 {
                Tone::Danger
            } else if warning > 0 {
                Tone::Warning
            } else if advisory > 0 {
                Tone::Info
            } else {
                Tone::Success
            },
            action_name: None,
            selected: false,
        });
        for (index, alarm) in self.model.active_alarms().into_iter().enumerate() {
            rows.push(self.operad_alarm_row(alarm, format!("active.{index}")));
        }
        for (index, alarm) in self.model.alarms.iter().take(8).enumerate() {
            rows.push(self.operad_alarm_row(alarm, format!("recent.{index}")));
        }
        rows
    }

    fn operad_alarm_row(&self, alarm: &EnvironmentAlarm, suffix: String) -> EnvironmentOperadRow {
        let sensor = self.sensor(&alarm.sensor_id);
        EnvironmentOperadRow {
            title: format!(
                "{} · {}{}",
                alarm.severity.label(),
                format_time(alarm.timestamp_min),
                if alarm.active { " · active" } else { "" }
            ),
            detail: format!(
                "{}{} · {}",
                sensor
                    .map(|sensor| format!(
                        "{} / {}",
                        sensor.zone,
                        format_reading(alarm.value, &sensor.unit)
                    ))
                    .unwrap_or_else(|| alarm.sensor_id.clone()),
                if alarm.correlation_labels.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", alarm.correlation_labels.join(", "))
                },
                alarm.message
            ),
            tone: severity_tone(Some(alarm.severity)),
            action_name: Some(format!(
                "{OPERAD_ACTION_SELECT_SENSOR}{}|alarm.{suffix}",
                alarm.sensor_id
            )),
            selected: self.selected_sensor.as_deref() == Some(alarm.sensor_id.as_str()),
        }
    }

    fn operad_sensor_rows(&self) -> Vec<EnvironmentOperadRow> {
        self.model
            .sensors
            .iter()
            .enumerate()
            .map(|(index, sensor)| {
                let latest = self.model.latest_reading(&sensor.id);
                let severity = latest.and_then(|reading| sensor.thresholds.evaluate(reading.value));
                let summary = self.model.trend_summary(&sensor.id);
                EnvironmentOperadRow {
                    title: format!("{} · {}", sensor.zone, sensor.name),
                    detail: format!(
                        "{} · {} · spec {}{}",
                        sensor.kind.label(),
                        latest
                            .map(|reading| format!(
                                "latest {} at {}",
                                format_reading(reading.value, &sensor.unit),
                                format_time(reading.timestamp_min)
                            ))
                            .unwrap_or_else(|| "no samples".to_string()),
                        sensor.thresholds.nominal_label(&sensor.unit),
                        summary
                            .map(|summary| format!(
                                " · trend {}, {} excursion(s)",
                                summary.direction.label(),
                                summary.excursion_count
                            ))
                            .unwrap_or_default()
                    ),
                    tone: severity_tone(severity),
                    action_name: Some(format!(
                        "{OPERAD_ACTION_SELECT_SENSOR}{}|sensor.{index}",
                        sensor.id
                    )),
                    selected: self.selected_sensor.as_deref() == Some(sensor.id.as_str()),
                }
            })
            .collect()
    }

    fn operad_zone_rows(&self) -> Vec<EnvironmentOperadRow> {
        self.zone_summaries()
            .into_iter()
            .map(|summary| EnvironmentOperadRow {
                title: format!("{} · {}", summary.name, zone_state_label(&summary)),
                detail: format!(
                    "{} sensor(s), {} historical excursion(s), latest {}",
                    summary.sensor_count,
                    summary.excursion_count,
                    summary
                        .latest_timestamp_min
                        .map(format_time)
                        .unwrap_or_else(|| "--:--".to_string())
                ),
                tone: severity_tone(summary.worst_severity()),
                action_name: None,
                selected: false,
            })
            .collect()
    }

    fn operad_event_rows(&self) -> Vec<EnvironmentOperadRow> {
        self.model
            .events
            .iter()
            .map(|event| EnvironmentOperadRow {
                title: format!(
                    "{} · {}",
                    format_time(event.timestamp_min),
                    event.kind.label()
                ),
                detail: format!(
                    "{} · {}{}",
                    event.zone,
                    event.title,
                    if event.linked_alarm_id.is_some() {
                        " · linked alarm"
                    } else {
                        ""
                    }
                ),
                tone: event_tone(event.kind),
                action_name: None,
                selected: false,
            })
            .collect()
    }

    fn operad_correlation_rows(&self) -> Vec<EnvironmentOperadRow> {
        let related = self.selected_sensor.as_deref().map_or_else(Vec::new, |id| {
            self.model
                .correlations
                .iter()
                .filter(|label| label.sensor_ids.iter().any(|sensor_id| sensor_id == id))
                .collect::<Vec<_>>()
        });
        let correlations = if related.is_empty() {
            self.model.correlations.iter().collect::<Vec<_>>()
        } else {
            related
        };
        correlations
            .into_iter()
            .map(|label| EnvironmentOperadRow {
                title: format!("{} · {}", label.process_area, label.label),
                detail: format!(
                    "{} · sensors {}",
                    label.description,
                    label.sensor_ids.join(", ")
                ),
                tone: Tone::Info,
                action_name: label.sensor_ids.first().map(|sensor_id| {
                    format!(
                        "{OPERAD_ACTION_SELECT_SENSOR}{sensor_id}|correlation.{}",
                        label.id
                    )
                }),
                selected: label
                    .sensor_ids
                    .iter()
                    .any(|sensor_id| self.selected_sensor.as_deref() == Some(sensor_id.as_str())),
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str) -> bool {
        let Some(sensor_id) = node_name.strip_prefix(OPERAD_ACTION_SELECT_SENSOR) else {
            return false;
        };
        let sensor_id = sensor_id
            .split_once('|')
            .map(|(sensor_id, _)| sensor_id)
            .unwrap_or(sensor_id);
        if self.sensor(sensor_id).is_some() {
            self.selected_sensor = Some(sensor_id.to_string());
            return true;
        }
        false
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

fn environment_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += environment_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += environment_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn environment_operad_metric_columns(width: f32) -> usize {
    if width >= 1020.0 {
        4
    } else if width >= 680.0 {
        3
    } else if width >= 440.0 {
        2
    } else {
        1
    }
}

fn environment_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = environment_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn environment_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_environment_operad_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    eyebrow: &str,
    title: &str,
    detail: &str,
    meta: &str,
) {
    let header = document.add_child(
        parent,
        UiNode::container(
            "environment.header",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(OPERAD_HEADER_HEIGHT),
                    ),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 27, 32, 255),
            Some(StrokeStyle::new(ColorRgba::new(46, 55, 64, 255), 1.0)),
            6.0,
        )),
    );
    add_environment_operad_text(
        document,
        header,
        "environment.header.eyebrow",
        eyebrow,
        environment_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_environment_operad_text(
        document,
        header,
        "environment.header.title",
        title,
        environment_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_environment_operad_text(
        document,
        header,
        "environment.header.detail",
        detail,
        environment_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_environment_operad_text(
        document,
        header,
        "environment.header.meta",
        meta,
        environment_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_environment_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[EnvironmentMetricTile],
) {
    let columns = environment_operad_metric_columns(width);
    let grid_height = environment_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "environment.metrics",
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(grid_height),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    let tile_width =
        ((width - OPERAD_GAP * (columns.saturating_sub(1) as f32)) / columns as f32).max(120.0);
    for (row_index, chunk) in metrics.chunks(columns).enumerate() {
        let row = document.add_child(
            grid,
            UiNode::container(
                format!("environment.metrics.row.{row_index}"),
                UiNodeStyle {
                    layout: layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_METRIC_HEIGHT),
                    ),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        for (column, metric) in chunk.iter().enumerate() {
            add_environment_operad_metric_tile(
                document,
                row,
                &format!("environment.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_environment_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &EnvironmentMetricTile,
) {
    let tile = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_margin_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width.max(116.0)),
                            layout::px(OPERAD_METRIC_HEIGHT - 8.0),
                        ),
                        3.0,
                    ),
                    9.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(29, 35, 40, 255),
            Some(StrokeStyle::new(
                environment_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_environment_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        environment_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_environment_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        environment_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_environment_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        environment_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            environment_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_environment_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[EnvironmentOperadRow],
) {
    let height = environment_operad_section_height(rows.len());
    let section = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(21, 26, 31, 255),
            Some(StrokeStyle::new(ColorRgba::new(45, 53, 61, 255), 1.0)),
            6.0,
        )),
    );
    add_environment_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        environment_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_environment_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_environment_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_environment_operad_empty_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    label: &str,
) {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.empty"),
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_EMPTY_ROW_HEIGHT),
                    ),
                    8.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(27, 32, 37, 255),
            Some(StrokeStyle::new(ColorRgba::new(43, 50, 58, 255), 1.0)),
            5.0,
        )),
    );
    add_environment_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        environment_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_environment_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &EnvironmentOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        environment_operad_tone_color(Tone::Info)
    } else {
        ColorRgba::new(42, 50, 58, 255)
    };
    let fill = if row.selected {
        ColorRgba::new(26, 42, 56, 255)
    } else {
        ColorRgba::new(26, 31, 36, 255)
    };
    let mut node = UiNode::container(
        row_name,
        UiNodeStyle {
            layout: layout::with_padding_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(OPERAD_ROW_HEIGHT),
                ),
                6.0,
            ),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(
        fill,
        Some(StrokeStyle::new(stroke_color, 1.0)),
        4.0,
    ));
    if row.action_name.is_some() {
        node = node.with_input(InputBehavior::BUTTON).with_accessibility(
            crate::ui_chrome::operad_button_accessibility(&row.title, &row.detail),
        );
    }
    let row_node = document.add_child(parent, node);
    document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.tone"),
            UiNodeStyle {
                layout: layout::fixed(5.0, OPERAD_ROW_HEIGHT - 12.0),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            environment_operad_tone_color(row.tone),
            None,
            2.0,
        )),
    );
    let text_column = document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.text"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::px((row_width - 28.0).max(120.0)),
                    layout::px(OPERAD_ROW_HEIGHT - 12.0),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    add_environment_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        environment_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_environment_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        environment_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_environment_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("environment.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_environment_operad_text(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    text: impl Into<String>,
    style: TextStyle,
    height: f32,
) {
    widgets::label(
        document,
        parent,
        name,
        text,
        style,
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
    );
}

fn environment_operad_text_style(
    font_size: f32,
    weight: FontWeight,
    color: ColorRgba,
) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn environment_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn event_tone(kind: FacilityEventKind) -> Tone {
    match kind {
        FacilityEventKind::Maintenance => Tone::Info,
        FacilityEventKind::Excursion | FacilityEventKind::ProcessHold => Tone::Warning,
        FacilityEventKind::Recovery => Tone::Success,
    }
}

fn trend_tone(direction: TrendDirection) -> Tone {
    match direction {
        TrendDirection::Falling | TrendDirection::Rising => Tone::Warning,
        TrendDirection::Stable => Tone::Success,
    }
}

fn truncate_middle(text: impl AsRef<str>, max_chars: usize) -> String {
    let text = text.as_ref();
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let head = keep / 2;
    let tail = keep - head;
    let start = text.chars().take(head).collect::<String>();
    let end = text
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}...{end}")
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
    let width = ui.available_width().clamp(120.0, 460.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_operad_view_audits_common_widths() {
        let mut panel = EnvironmentPanel::from_model(CleanroomEnvironment::sample());
        panel.ensure_selection();
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width);
            view.document
                .compute_layout(view.size, &mut ApproxTextMeasurer)
                .unwrap();
            let warnings = view.document.audit_layout();
            assert!(warnings.is_empty(), "{warnings:#?}");
            assert!(view.document.node_count() > 20);
            assert!(!view.document.paint_list().items.is_empty());
        }
    }

    #[test]
    fn environment_operad_action_selects_sensor() {
        let mut panel = EnvironmentPanel::from_model(CleanroomEnvironment::sample());
        let target = panel.model.sensors.last().unwrap().id.clone();
        assert!(panel.handle_operad_action(&format!("{OPERAD_ACTION_SELECT_SENSOR}{target}|test")));
        assert_eq!(panel.selected_sensor.as_deref(), Some(target.as_str()));
    }
}
