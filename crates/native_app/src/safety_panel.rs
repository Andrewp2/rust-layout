use eframe::egui::{self, Color32, RichText};
use layout_model::safety::{
    AlarmRouteTarget, SafetyAuditKind, SafetyIncident, SafetySeverity, SafetySystem, ToolLockout,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct SafetyPanel {
    model: SafetySystem,
    selected_tool: Option<String>,
}

impl SafetyPanel {
    pub(crate) fn from_model(model: SafetySystem) -> Self {
        let selected_tool = model
            .evaluate_lockouts()
            .first()
            .map(|lockout| lockout.tool_id.clone());
        Self {
            model,
            selected_tool,
        }
    }

    pub(crate) fn model(&self) -> &SafetySystem {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        let summary = self.model.summary();

        ui_chrome::section_label(ui, "Safety Interlocks");
        ui.label("Simulated monitoring only");
        ui.label(format!("Sensors: {}", summary.sensor_count));
        ui.label(format!(
            "Active conditions: {}",
            summary.active_condition_count
        ));
        ui.label(format!("Tool lockouts: {}", summary.locked_out_tool_count));
        ui.label(format!("Open incidents: {}", summary.open_incident_count));

        ui.separator();
        ui_chrome::section_label(ui, "Selected Tool");
        let lockouts = self.model.evaluate_lockouts();
        if let Some(lockout) = self.selected_lockout(&lockouts) {
            ui.label(format!("{} ({})", lockout.tool_name, lockout.tool_id));
            status_pill(ui, lockout);
            for reason in &lockout.reasons {
                ui.small(reason);
            }
            if lockout.reasons.is_empty() {
                ui_chrome::muted(ui, "All simulated interlocks clear");
            }
        } else {
            ui_chrome::empty_state(ui, "No tool interlocks loaded");
        }

        ui.separator();
        ui_chrome::section_label(ui, "Recent Audit");
        for event in self.model.audit_events.iter().take(6) {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
                ui.label(event.kind.label());
                ui.small(&event.timestamp);
            });
            ui.small(&event.message);
        }
        if self.model.audit_events.is_empty() {
            ui_chrome::empty_state(ui, "No safety audit events loaded");
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let summary = self.model.summary();
        let lockouts = self.model.evaluate_lockouts();

        egui::ScrollArea::vertical()
            .id_salt("safety_interlock_dashboard_scroll")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Simulated operations",
                    "Safety and Interlock Dashboard",
                    "Demo-only safety monitoring, alarm routing, tool lockout, and audit trail",
                    |ui| {
                        ui.label("Tool");
                        let mut selected = self.selected_tool.clone();
                        let selected_text = selected
                            .as_deref()
                            .and_then(|tool_id| {
                                lockouts
                                    .iter()
                                    .find(|lockout| lockout.tool_id == tool_id)
                                    .map(|lockout| lockout.tool_name.clone())
                            })
                            .unwrap_or_else(|| "none".to_string());
                        egui::ComboBox::from_id_salt("safety_tool_picker")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for lockout in &lockouts {
                                    ui.selectable_value(
                                        &mut selected,
                                        Some(lockout.tool_id.clone()),
                                        &lockout.tool_name,
                                    );
                                }
                            });
                        if selected != self.selected_tool {
                            self.selected_tool = selected;
                            *status = "safety interlock context selected".to_string();
                        }
                    },
                );

                ui_chrome::metric_tiles(
                    ui,
                    &[
                        (
                            "Overall state",
                            summary
                                .highest_severity
                                .map(SafetySeverity::label)
                                .unwrap_or("blank")
                                .to_string(),
                            "simulated monitor",
                            severity_tone(
                                summary.highest_severity.unwrap_or(SafetySeverity::Normal),
                            ),
                        ),
                        (
                            "Active conditions",
                            summary.active_condition_count.to_string(),
                            "warning or critical sensors",
                            if summary.active_condition_count == 0 {
                                Tone::Success
                            } else {
                                Tone::Warning
                            },
                        ),
                        (
                            "Locked tools",
                            summary.locked_out_tool_count.to_string(),
                            "computed from interlocks",
                            if summary.locked_out_tool_count == 0 {
                                Tone::Success
                            } else {
                                Tone::Danger
                            },
                        ),
                        (
                            "Open incidents",
                            summary.open_incident_count.to_string(),
                            "simulated audit trail",
                            if summary.open_incident_count == 0 {
                                Tone::Success
                            } else {
                                Tone::Warning
                            },
                        ),
                    ],
                );

                ui.separator();
                if ui.available_width() < 720.0 {
                    self.sensors_ui(ui);
                    ui.separator();
                    self.lockouts_ui(ui, &lockouts);
                } else {
                    ui.columns(2, |columns| {
                        self.sensors_ui(&mut columns[0]);
                        self.lockouts_ui(&mut columns[1], &lockouts);
                    });
                }

                ui.separator();
                if ui.available_width() < 720.0 {
                    self.routing_ui(ui);
                    ui.separator();
                    incidents_ui(ui, &self.model.incidents);
                } else {
                    ui.columns(2, |columns| {
                        self.routing_ui(&mut columns[0]);
                        incidents_ui(&mut columns[1], &self.model.incidents);
                    });
                }

                ui.separator();
                audit_ui(ui, &self.model);
            });
    }

    fn sensors_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Simulated Sensors");
        if self.model.sensors.is_empty() {
            ui_chrome::empty_state(ui, "No safety sensors loaded");
            return;
        }

        if ui.available_width() < 520.0 {
            for sensor in &self.model.sensors {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.strong(&sensor.name);
                    ui.small(sensor.domain.label());
                    ui.colored_label(
                        severity_color(sensor.severity),
                        format!("{} / {}", sensor.state.label(), sensor.severity.label()),
                    );
                    ui.label(format!("{} {}", compact_number(sensor.value), sensor.unit));
                });
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("safety_sensor_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("safety_sensor_grid")
                    .striped(true)
                    .min_col_width(76.0)
                    .show(ui, |ui| {
                        ui.strong("Sensor");
                        ui.strong("Domain");
                        ui.strong("State");
                        ui.strong("Value");
                        ui.end_row();
                        for sensor in &self.model.sensors {
                            ui.label(&sensor.name);
                            ui.label(sensor.domain.label());
                            ui.colored_label(
                                severity_color(sensor.severity),
                                format!("{} / {}", sensor.state.label(), sensor.severity.label()),
                            );
                            ui.label(format!("{} {}", compact_number(sensor.value), sensor.unit));
                            ui.end_row();
                        }
                    });
            });
    }

    fn lockouts_ui(&mut self, ui: &mut egui::Ui, lockouts: &[ToolLockout]) {
        ui_chrome::section_label(ui, "Tool Lockout");
        if lockouts.is_empty() {
            ui_chrome::empty_state(ui, "No tool interlocks loaded");
            return;
        }

        for lockout in lockouts {
            let selected = self.selected_tool.as_deref() == Some(lockout.tool_id.as_str());
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(selected, RichText::new(lockout.tool_name.as_str()).strong())
                    .clicked()
                {
                    self.selected_tool = Some(lockout.tool_id.clone());
                }
                status_pill(ui, lockout);
            });
            if selected {
                if lockout.reasons.is_empty() {
                    ui_chrome::muted(ui, "No failed simulated safety condition");
                } else {
                    for reason in &lockout.reasons {
                        ui.small(reason);
                    }
                }
            }
        }
    }

    fn routing_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Alarm Routing");
        if self.model.alarm_routes.is_empty() {
            ui_chrome::empty_state(ui, "No alarm routes loaded");
            return;
        }

        if ui.available_width() < 520.0 {
            for route in &self.model.alarm_routes {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.strong(route.domain.label());
                    ui.small(route.minimum_severity.label());
                    ui.add(
                        egui::Label::new(format!("{} via {}", route.target.label(), route.channel))
                            .wrap(),
                    );
                });
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("safety_alarm_routes_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("safety_alarm_routes")
                    .striped(true)
                    .min_col_width(90.0)
                    .show(ui, |ui| {
                        ui.strong("Domain");
                        ui.strong("Severity");
                        ui.strong("Target");
                        ui.end_row();
                        for route in &self.model.alarm_routes {
                            ui.label(route.domain.label());
                            ui.label(route.minimum_severity.label());
                            ui.label(format!("{} via {}", route.target.label(), route.channel));
                            ui.end_row();
                        }
                    });
            });

        ui.add_space(6.0);
        ui_chrome::section_label(ui, "Active Routes");
        for sensor in self.model.active_conditions() {
            let targets = self
                .model
                .route_targets_for(sensor)
                .into_iter()
                .map(AlarmRouteTarget::label)
                .collect::<Vec<_>>();
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(severity_color(sensor.severity), sensor.name.as_str());
                ui.label(if targets.is_empty() {
                    "no matching route".to_string()
                } else {
                    targets.join(", ")
                });
            });
        }
    }

    fn ensure_selection(&mut self) {
        let lockouts = self.model.evaluate_lockouts();
        let selection_valid = self
            .selected_tool
            .as_ref()
            .is_some_and(|tool_id| lockouts.iter().any(|lockout| &lockout.tool_id == tool_id));
        if !selection_valid {
            self.selected_tool = lockouts.first().map(|lockout| lockout.tool_id.clone());
        }
    }

    fn selected_lockout<'a>(&self, lockouts: &'a [ToolLockout]) -> Option<&'a ToolLockout> {
        self.selected_tool
            .as_deref()
            .and_then(|tool_id| lockouts.iter().find(|lockout| lockout.tool_id == tool_id))
    }
}

fn incidents_ui(ui: &mut egui::Ui, incidents: &[SafetyIncident]) {
    ui_chrome::section_label(ui, "Incident Trail");
    if incidents.is_empty() {
        ui_chrome::empty_state(ui, "No safety incidents loaded");
        return;
    }

    for incident in incidents {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(severity_color(incident.severity), incident.id.to_string());
            ui.label(incident.status.label());
            ui.label(incident.domain.label());
        });
        ui.label(&incident.summary);
        ui.small(format!("opened {}", incident.opened_at));
        ui.add_space(4.0);
    }
}

fn audit_ui(ui: &mut egui::Ui, model: &SafetySystem) {
    ui_chrome::section_label(ui, "Audit Trail");
    if model.audit_events.is_empty() {
        ui_chrome::empty_state(ui, "No safety audit events loaded");
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("safety_audit_horizontal")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("safety_audit_grid")
                .striped(true)
                .min_col_width(90.0)
                .show(ui, |ui| {
                    ui.strong("Seq");
                    ui.strong("Time");
                    ui.strong("Kind");
                    ui.strong("Message");
                    ui.end_row();
                    for event in &model.audit_events {
                        ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
                        ui.label(&event.timestamp);
                        ui.label(event.kind.label());
                        ui.label(&event.message);
                        ui.end_row();
                    }
                });
        });
}

fn status_pill(ui: &mut egui::Ui, lockout: &ToolLockout) {
    if lockout.locked_out {
        ui_chrome::status_pill(ui, "Locked out", Tone::Danger);
    } else {
        ui_chrome::status_pill(ui, "Clear", Tone::Success);
    }
}

fn severity_tone(severity: SafetySeverity) -> Tone {
    match severity {
        SafetySeverity::Normal => Tone::Success,
        SafetySeverity::Advisory => Tone::Info,
        SafetySeverity::Warning => Tone::Warning,
        SafetySeverity::Critical => Tone::Danger,
    }
}

fn severity_color(severity: SafetySeverity) -> Color32 {
    severity_tone(severity).color()
}

fn audit_color(kind: SafetyAuditKind) -> Color32 {
    match kind {
        SafetyAuditKind::SensorSample => Color32::from_rgb(126, 166, 214),
        SafetyAuditKind::AlarmRouted => Color32::from_rgb(220, 176, 72),
        SafetyAuditKind::ToolLockedOut => Color32::from_rgb(226, 96, 96),
        SafetyAuditKind::IncidentOpened => Color32::from_rgb(232, 130, 96),
        SafetyAuditKind::IncidentUpdated => Color32::from_rgb(145, 152, 160),
    }
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
