use std::collections::BTreeSet;

use eframe::egui::{self, Color32, RichText};
use layout_model::safety::{
    AlarmRouteTarget, IncidentStatus, SafetyAuditEvent, SafetyAuditKind, SafetyIncident,
    SafetySensor, SafetySeverity, SafetySystem, ToolLockout,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct SafetyPanel {
    model: SafetySystem,
    selected_tool: Option<String>,
    acknowledged_conditions: BTreeSet<String>,
    acknowledged_lockouts: BTreeSet<String>,
    acknowledged_incidents: BTreeSet<String>,
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
            acknowledged_conditions: BTreeSet::new(),
            acknowledged_lockouts: BTreeSet::new(),
            acknowledged_incidents: BTreeSet::new(),
        }
    }

    pub(crate) fn model(&self) -> &SafetySystem {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        let summary = self.model.summary();

        ui_chrome::section_label(ui, "Safety Interlocks");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                summary
                    .highest_severity
                    .map(SafetySeverity::label)
                    .unwrap_or("normal"),
                severity_tone(summary.highest_severity.unwrap_or(SafetySeverity::Normal)),
            );
            ui.label("Simulated monitoring only");
        });
        ui.label(format!("Sensors: {}", summary.sensor_count));
        ui.label(format!(
            "Active interlocks: {}",
            summary.active_condition_count
        ));
        ui.label(format!("Tool lockouts: {}", summary.locked_out_tool_count));
        ui.label(format!("Open incidents: {}", summary.open_incident_count));
        ui.label(format!(
            "Acknowledged this session: {}",
            self.acknowledged_count()
        ));

        ui.separator();
        ui_chrome::section_label(ui, "Active Conditions");
        let active_conditions = self
            .model
            .active_conditions()
            .into_iter()
            .take(4)
            .collect::<Vec<_>>();
        if active_conditions.is_empty() {
            ui_chrome::empty_state(ui, "No active safety interlocks");
        } else {
            for sensor in active_conditions {
                ui.horizontal_wrapped(|ui| {
                    severity_pill(ui, sensor.severity);
                    ui.label(&sensor.name);
                    if self
                        .acknowledged_conditions
                        .contains(&sensor.id.to_string())
                    {
                        ui_chrome::status_pill(ui, "acked", Tone::Info);
                    }
                });
                ui.small(active_condition_summary(sensor, &self.model));
            }
        }

        ui.separator();
        ui_chrome::section_label(ui, "Selected Tool");
        let lockouts = self.model.evaluate_lockouts();
        if let Some(lockout) = self.selected_lockout(&lockouts) {
            ui.label(format!("{} ({})", lockout.tool_name, lockout.tool_id));
            status_pill(ui, lockout);
            if self.acknowledged_lockouts.contains(&lockout.tool_id) {
                ui_chrome::status_pill(ui, "acknowledged", Tone::Info);
            }
            for reason in &lockout.reasons {
                ui.small(reason);
            }
            if lockout.reasons.is_empty() {
                ui_chrome::muted(ui, "All simulated interlocks clear");
            }
            ui.separator();
            let permissives = self.permissive_rows_for_tool(&lockout.tool_id);
            for row in permissives.iter().take(5) {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        if row.passes {
                            Tone::Success.color()
                        } else {
                            Tone::Danger.color()
                        },
                        if row.passes { "pass" } else { "blocked" },
                    );
                    ui.label(&row.name);
                });
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
        let selected_permissives = self
            .selected_tool
            .as_deref()
            .map(|tool_id| self.permissive_rows_for_tool(tool_id))
            .unwrap_or_default();
        let ready_permissives = selected_permissives.iter().filter(|row| row.passes).count();
        let severity_detail = severity_breakdown(&self.model.sensors);
        let acknowledged_count = self.acknowledged_count();

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
                            severity_detail.as_str(),
                            severity_tone(
                                summary.highest_severity.unwrap_or(SafetySeverity::Normal),
                            ),
                        ),
                        (
                            "Active interlocks",
                            summary.active_condition_count.to_string(),
                            "conditions blocking permissives",
                            if summary.active_condition_count == 0 {
                                Tone::Success
                            } else if summary.highest_severity == Some(SafetySeverity::Critical) {
                                Tone::Danger
                            } else {
                                Tone::Warning
                            },
                        ),
                        (
                            "Permissives",
                            format!("{ready_permissives}/{}", selected_permissives.len()),
                            "selected tool checks",
                            if selected_permissives.is_empty()
                                || ready_permissives == selected_permissives.len()
                            {
                                Tone::Success
                            } else {
                                Tone::Danger
                            },
                        ),
                        (
                            "Tool lockouts",
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
                            "requiring operator review",
                            if summary.open_incident_count == 0 {
                                Tone::Success
                            } else {
                                Tone::Warning
                            },
                        ),
                        (
                            "Acknowledged",
                            acknowledged_count.to_string(),
                            "current UI session",
                            if acknowledged_count == 0 {
                                Tone::Neutral
                            } else {
                                Tone::Info
                            },
                        ),
                    ],
                );

                ui.separator();
                self.active_interlocks_ui(ui, status);

                ui.separator();
                if ui.available_width() < 760.0 {
                    self.lockouts_ui(ui, &lockouts, status);
                    ui.separator();
                    self.selected_tool_ui(ui, &lockouts, status);
                } else {
                    ui.columns(2, |columns| {
                        self.lockouts_ui(&mut columns[0], &lockouts, status);
                        self.selected_tool_ui(&mut columns[1], &lockouts, status);
                    });
                }

                ui.separator();
                if ui.available_width() < 760.0 {
                    self.sensors_ui(ui);
                    ui.separator();
                    self.routing_ui(ui);
                } else {
                    ui.columns(2, |columns| {
                        self.sensors_ui(&mut columns[0]);
                        self.routing_ui(&mut columns[1]);
                    });
                }

                ui.separator();
                if ui.available_width() < 760.0 {
                    self.incidents_ui(ui, status);
                    ui.separator();
                    audit_ui(ui, &self.model);
                } else {
                    ui.columns(2, |columns| {
                        self.incidents_ui(&mut columns[0], status);
                        audit_ui(&mut columns[1], &self.model);
                    });
                }
            });
    }

    fn active_interlocks_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        ui_chrome::section_label(ui, "Active Interlocks");
        let active_conditions = self
            .model
            .active_conditions()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        if active_conditions.is_empty() {
            ui_chrome::empty_state(ui, "All simulated interlocks are clear");
            return;
        }

        if ui.available_width() < 680.0 {
            for sensor in &active_conditions {
                active_condition_card_ui(ui, sensor, &self.model);
                self.condition_ack_ui(ui, sensor, status);
                ui.add_space(4.0);
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("safety_active_interlocks_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("safety_active_interlocks_grid")
                    .striped(true)
                    .min_col_width(82.0)
                    .show(ui, |ui| {
                        ui.strong("Severity");
                        ui.strong("Condition");
                        ui.strong("Value");
                        ui.strong("Route");
                        ui.strong("Action");
                        ui.end_row();
                        for sensor in &active_conditions {
                            severity_pill(ui, sensor.severity);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&sensor.name).strong());
                                ui.small(format!(
                                    "{} / {} / {}",
                                    sensor.domain.label(),
                                    sensor.state.label(),
                                    sensor.tool_id.as_deref().unwrap_or("global interlock")
                                ));
                            });
                            ui.label(format!(
                                "{} {} ({})",
                                compact_number(sensor.value),
                                sensor.unit,
                                limit_label(&sensor.limit)
                            ));
                            ui.label(route_summary(sensor, &self.model));
                            self.condition_ack_ui(ui, sensor, status);
                            ui.end_row();
                        }
                    });
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

    fn lockouts_ui(&mut self, ui: &mut egui::Ui, lockouts: &[ToolLockout], status: &mut String) {
        ui_chrome::section_label(ui, "Tool Lockouts");
        if lockouts.is_empty() {
            ui_chrome::empty_state(ui, "No tool interlocks loaded");
            return;
        }

        for lockout in lockouts {
            let selected = self.selected_tool.as_deref() == Some(lockout.tool_id.as_str());
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(selected, RichText::new(lockout.tool_name.as_str()).strong())
                    .clicked()
                {
                    self.selected_tool = Some(lockout.tool_id.clone());
                }
                status_pill(ui, lockout);
                if self.acknowledged_lockouts.contains(&lockout.tool_id) {
                    ui_chrome::status_pill(ui, "acked", Tone::Info);
                } else if lockout.locked_out
                    && ui
                        .button("Acknowledge")
                        .on_hover_text(
                            "Record operator acknowledgement without clearing the interlock",
                        )
                        .clicked()
                {
                    self.acknowledge_lockout(lockout, status);
                }
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

    fn selected_tool_ui(
        &mut self,
        ui: &mut egui::Ui,
        lockouts: &[ToolLockout],
        status: &mut String,
    ) {
        ui_chrome::section_label(ui, "Selected Tool Readiness");
        let Some(lockout) = self.selected_lockout(lockouts).cloned() else {
            ui_chrome::empty_state(ui, "Select a tool to inspect permissives");
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(&lockout.tool_name).strong());
            ui.label(&lockout.tool_id);
            status_pill(ui, &lockout);
        });
        if lockout.locked_out {
            ui_chrome::muted(
                ui,
                "Acknowledgement records review only; it does not clear an active interlock.",
            );
            if self.acknowledged_lockouts.contains(&lockout.tool_id) {
                ui_chrome::status_pill(ui, "lockout acknowledged", Tone::Info);
            } else if ui.button("Acknowledge lockout").clicked() {
                self.acknowledge_lockout(&lockout, status);
            }
        }

        ui.add_space(4.0);
        ui_chrome::section_label(ui, "Permissive Checklist");
        let permissives = self.permissive_rows_for_tool(&lockout.tool_id);
        if permissives.is_empty() {
            ui_chrome::empty_state(ui, "No permissives are configured for this tool");
            return;
        }

        if ui.available_width() < 560.0 {
            for row in &permissives {
                permissive_card_ui(ui, row);
            }
            return;
        }

        egui::Grid::new("safety_permissive_grid")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Ready");
                ui.strong("Permissive");
                ui.strong("State");
                ui.strong("Value");
                ui.strong("Limit");
                ui.strong("Last");
                ui.end_row();
                for row in &permissives {
                    ui.colored_label(
                        if row.passes {
                            Tone::Success.color()
                        } else {
                            Tone::Danger.color()
                        },
                        if row.passes { "pass" } else { "blocked" },
                    );
                    ui.vertical(|ui| {
                        ui.label(&row.name);
                        ui.small(&row.domain);
                    });
                    ui.colored_label(severity_color(row.severity), &row.state);
                    ui.label(&row.value);
                    ui.label(&row.limit);
                    ui.label(format!("{}s", row.last_seen_s));
                    ui.end_row();
                }
            });
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
        let active_conditions = self.model.active_conditions();
        if active_conditions.is_empty() {
            ui_chrome::empty_state(ui, "No active alarm routes");
            return;
        }
        for sensor in active_conditions {
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

    fn incidents_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        ui_chrome::section_label(ui, "Incident Trail");
        if self.model.incidents.is_empty() {
            ui_chrome::empty_state(ui, "No safety incidents loaded");
            return;
        }

        let incidents = self.model.incidents.clone();
        if ui.available_width() < 620.0 {
            for incident in &incidents {
                incident_card_ui(ui, incident);
                self.incident_ack_ui(ui, incident, status);
                ui.add_space(4.0);
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("safety_incident_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("safety_incident_grid")
                    .striped(true)
                    .min_col_width(82.0)
                    .show(ui, |ui| {
                        ui.strong("Incident");
                        ui.strong("Status");
                        ui.strong("Domain");
                        ui.strong("Route");
                        ui.strong("Action");
                        ui.end_row();
                        for incident in &incidents {
                            ui.vertical(|ui| {
                                ui.colored_label(
                                    severity_color(incident.severity),
                                    incident.id.to_string(),
                                );
                                ui.small(&incident.opened_at);
                            });
                            ui.label(incident.status.label());
                            ui.label(incident.domain.label());
                            ui.label(route_targets_label(&incident.routed_to));
                            self.incident_ack_ui(ui, incident, status);
                            ui.end_row();

                            ui.label("");
                            ui.label("");
                            ui.add(egui::Label::new(&incident.summary).wrap().selectable(false));
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        }
                    });
            });
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

    fn condition_ack_ui(&mut self, ui: &mut egui::Ui, sensor: &SafetySensor, status: &mut String) {
        let sensor_id = sensor.id.to_string();
        if self.acknowledged_conditions.contains(&sensor_id) {
            ui_chrome::status_pill(ui, "acknowledged", Tone::Info);
        } else if ui.button("Acknowledge").clicked() {
            self.acknowledge_condition(sensor_id, sensor.name.clone(), status);
        }
    }

    fn incident_ack_ui(
        &mut self,
        ui: &mut egui::Ui,
        incident: &SafetyIncident,
        status: &mut String,
    ) {
        let incident_id = incident.id.to_string();
        if self.acknowledged_incidents.contains(&incident_id) {
            ui_chrome::status_pill(ui, "acknowledged", Tone::Info);
        } else if incident.status == IncidentStatus::Closed {
            ui_chrome::status_pill(ui, "closed", Tone::Neutral);
        } else if ui.button("Acknowledge").clicked() {
            self.acknowledge_incident(incident, status);
        }
    }

    fn acknowledge_condition(
        &mut self,
        sensor_id: String,
        sensor_name: String,
        status: &mut String,
    ) {
        if self.acknowledged_conditions.insert(sensor_id.clone()) {
            self.push_audit_event(
                SafetyAuditKind::IncidentUpdated,
                format!("Acknowledged active condition {sensor_id}: {sensor_name}"),
            );
            *status = format!("acknowledged safety condition {sensor_id}");
        } else {
            *status = format!("safety condition {sensor_id} was already acknowledged");
        }
    }

    fn acknowledge_lockout(&mut self, lockout: &ToolLockout, status: &mut String) {
        if self.acknowledged_lockouts.insert(lockout.tool_id.clone()) {
            self.push_audit_event(
                SafetyAuditKind::ToolLockedOut,
                format!(
                    "Acknowledged lockout for {} ({})",
                    lockout.tool_name, lockout.tool_id
                ),
            );
            *status = format!("acknowledged {} safety lockout", lockout.tool_id);
        } else {
            *status = format!(
                "{} safety lockout was already acknowledged",
                lockout.tool_id
            );
        }
    }

    fn acknowledge_incident(&mut self, incident: &SafetyIncident, status: &mut String) {
        let incident_id = incident.id.to_string();
        if self.acknowledged_incidents.insert(incident_id.clone()) {
            self.push_audit_event(
                SafetyAuditKind::IncidentUpdated,
                format!("Acknowledged incident {incident_id}: {}", incident.summary),
            );
            *status = format!("acknowledged safety incident {incident_id}");
        } else {
            *status = format!("safety incident {incident_id} was already acknowledged");
        }
    }

    fn push_audit_event(&mut self, kind: SafetyAuditKind, message: String) {
        let sequence = self
            .model
            .audit_events
            .iter()
            .map(|event| event.sequence)
            .max()
            .unwrap_or(0)
            + 1;
        self.model.audit_events.insert(
            0,
            SafetyAuditEvent {
                sequence,
                timestamp: "current session".to_string(),
                actor: "fabricad.operator".to_string(),
                kind,
                message,
            },
        );
    }

    fn acknowledged_count(&self) -> usize {
        self.acknowledged_conditions.len()
            + self.acknowledged_lockouts.len()
            + self.acknowledged_incidents.len()
    }

    fn permissive_rows_for_tool(&self, tool_id: &str) -> Vec<PermissiveRow> {
        let Some(interlock) = self
            .model
            .tool_interlocks
            .iter()
            .find(|interlock| interlock.tool_id == tool_id)
        else {
            return Vec::new();
        };

        let mut seen = BTreeSet::new();
        let mut rows = Vec::new();
        for sensor_id in &interlock.required_sensors {
            seen.insert(sensor_id.to_string());
            rows.push(
                self.model
                    .sensors
                    .iter()
                    .find(|sensor| sensor.id == *sensor_id)
                    .map(PermissiveRow::from_sensor)
                    .unwrap_or_else(|| PermissiveRow::missing(sensor_id.to_string())),
            );
        }

        for sensor in self
            .model
            .sensors
            .iter()
            .filter(|sensor| sensor.tool_id.is_none() && sensor.required_for_interlock)
        {
            if seen.insert(sensor.id.to_string()) {
                rows.push(PermissiveRow::from_sensor(sensor));
            }
        }
        rows
    }
}

fn audit_ui(ui: &mut egui::Ui, model: &SafetySystem) {
    ui_chrome::section_label(ui, "Audit Trail");
    if model.audit_events.is_empty() {
        ui_chrome::empty_state(ui, "No safety audit events loaded");
        return;
    }

    if ui.available_width() < 620.0 {
        for event in &model.audit_events {
            ui.group(|ui| {
                ui.set_width(ui.available_width().clamp(220.0, 480.0));
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
                    ui.label(event.kind.label());
                    ui.small(&event.timestamp);
                });
                ui.add(egui::Label::new(&event.message).wrap());
                ui.small(format!("actor {}", event.actor));
            });
        }
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
                    ui.strong("Actor");
                    ui.strong("Message");
                    ui.end_row();
                    for event in &model.audit_events {
                        ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
                        ui.label(&event.timestamp);
                        ui.label(event.kind.label());
                        ui.label(&event.actor);
                        ui.label(&event.message);
                        ui.end_row();
                    }
                });
        });
}

#[derive(Clone, Debug)]
struct PermissiveRow {
    name: String,
    domain: String,
    state: String,
    severity: SafetySeverity,
    passes: bool,
    value: String,
    limit: String,
    last_seen_s: u64,
    message: String,
}

impl PermissiveRow {
    fn from_sensor(sensor: &SafetySensor) -> Self {
        Self {
            name: sensor.name.clone(),
            domain: sensor.domain.label().to_string(),
            state: sensor.state.label().to_string(),
            severity: sensor.severity,
            passes: !sensor.fails_interlock(),
            value: format!("{} {}", compact_number(sensor.value), sensor.unit),
            limit: limit_label(&sensor.limit),
            last_seen_s: sensor.last_seen_s,
            message: sensor.message.clone(),
        }
    }

    fn missing(sensor_id: String) -> Self {
        Self {
            name: format!("Missing sensor {sensor_id}"),
            domain: "Unresolved".to_string(),
            state: "missing".to_string(),
            severity: SafetySeverity::Critical,
            passes: false,
            value: "n/a".to_string(),
            limit: "required".to_string(),
            last_seen_s: 0,
            message: "Required permissive is not present in the safety model".to_string(),
        }
    }
}

fn active_condition_card_ui(ui: &mut egui::Ui, sensor: &SafetySensor, model: &SafetySystem) {
    ui.group(|ui| {
        ui.set_width(ui.available_width().clamp(220.0, 520.0));
        ui.horizontal_wrapped(|ui| {
            severity_pill(ui, sensor.severity);
            ui.label(RichText::new(&sensor.name).strong());
        });
        ui.label(active_condition_summary(sensor, model));
        ui.small(format!(
            "{} {} / limit {} / last sample {}s",
            compact_number(sensor.value),
            sensor.unit,
            limit_label(&sensor.limit),
            sensor.last_seen_s
        ));
        if !sensor.message.is_empty() {
            ui.small(&sensor.message);
        }
    });
}

fn permissive_card_ui(ui: &mut egui::Ui, row: &PermissiveRow) {
    ui.group(|ui| {
        ui.set_width(ui.available_width().clamp(220.0, 480.0));
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                if row.passes { "pass" } else { "blocked" },
                if row.passes {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            );
            ui.label(RichText::new(&row.name).strong());
        });
        ui.label(format!(
            "{} / {} / {} / limit {}",
            row.domain, row.state, row.value, row.limit
        ));
        ui.small(format!("last sample {}s", row.last_seen_s));
        if !row.message.is_empty() {
            ui.small(&row.message);
        }
    });
}

fn incident_card_ui(ui: &mut egui::Ui, incident: &SafetyIncident) {
    ui.group(|ui| {
        ui.set_width(ui.available_width().clamp(220.0, 520.0));
        ui.horizontal_wrapped(|ui| {
            severity_pill(ui, incident.severity);
            ui.label(RichText::new(incident.id.to_string()).strong());
            ui_chrome::status_pill(ui, incident.status.label(), incident_tone(incident.status));
        });
        ui.label(&incident.summary);
        ui.small(format!(
            "{} / opened {} / routed {}",
            incident.domain.label(),
            incident.opened_at,
            route_targets_label(&incident.routed_to)
        ));
    });
}

fn active_condition_summary(sensor: &SafetySensor, model: &SafetySystem) -> String {
    format!(
        "{} {} / {} / routed {}",
        sensor.tool_id.as_deref().unwrap_or("global interlock"),
        sensor.domain.label(),
        sensor.state.label(),
        route_summary(sensor, model)
    )
}

fn route_summary(sensor: &SafetySensor, model: &SafetySystem) -> String {
    route_targets_label(&model.route_targets_for(sensor))
}

fn route_targets_label(targets: &[AlarmRouteTarget]) -> String {
    if targets.is_empty() {
        "no matching route".to_string()
    } else {
        targets
            .iter()
            .map(|target| target.label())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn severity_breakdown(sensors: &[SafetySensor]) -> String {
    let critical = sensors
        .iter()
        .filter(|sensor| sensor.severity == SafetySeverity::Critical)
        .count();
    let warning = sensors
        .iter()
        .filter(|sensor| sensor.severity == SafetySeverity::Warning)
        .count();
    let advisory = sensors
        .iter()
        .filter(|sensor| sensor.severity == SafetySeverity::Advisory)
        .count();

    if critical + warning + advisory == 0 {
        "all sensors nominal".to_string()
    } else {
        format!("{critical} critical / {warning} warning / {advisory} advisory")
    }
}

fn limit_label(limit: &layout_model::safety::SafetyLimit) -> String {
    match (limit.lower, limit.upper) {
        (Some(lower), Some(upper)) => {
            format!("{}..{}", compact_number(lower), compact_number(upper))
        }
        (Some(lower), None) => format!(">= {}", compact_number(lower)),
        (None, Some(upper)) => format!("<= {}", compact_number(upper)),
        (None, None) => "none".to_string(),
    }
}

fn status_pill(ui: &mut egui::Ui, lockout: &ToolLockout) {
    if lockout.locked_out {
        ui_chrome::status_pill(ui, "Locked out", Tone::Danger);
    } else {
        ui_chrome::status_pill(ui, "Clear", Tone::Success);
    }
}

fn severity_pill(ui: &mut egui::Ui, severity: SafetySeverity) {
    ui_chrome::status_pill(ui, severity.label(), severity_tone(severity));
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

fn incident_tone(status: IncidentStatus) -> Tone {
    match status {
        IncidentStatus::Open => Tone::Warning,
        IncidentStatus::Contained => Tone::Info,
        IncidentStatus::Closed => Tone::Neutral,
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
