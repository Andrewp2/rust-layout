use std::collections::BTreeSet;

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::safety::{
    AlarmRouteTarget, IncidentStatus, SafetyAuditEvent, SafetyAuditKind, SafetyIncident,
    SafetySensor, SafetySeverity, SafetySystem, ToolLockout,
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
const OPERAD_ROW_HEIGHT: f32 = 52.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SELECT_TOOL: &str = "safety.action.select_tool.";
const OPERAD_ACTION_ACK_CONDITION: &str = "safety.action.ack_condition.";
const OPERAD_ACTION_ACK_LOCKOUT: &str = "safety.action.ack_lockout.";
const OPERAD_ACTION_ACK_INCIDENT: &str = "safety.action.ack_incident.";

pub(crate) struct SafetyPanel {
    model: SafetySystem,
    selected_tool: Option<String>,
    acknowledged_conditions: BTreeSet<String>,
    acknowledged_lockouts: BTreeSet<String>,
    acknowledged_incidents: BTreeSet<String>,
}

#[derive(Debug)]
struct SafetyOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct SafetyMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct SafetyOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    button_name: Option<String>,
    button_label: Option<&'static str>,
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
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        let summary = self.model.summary();
        let mut conditions =
            SidecarSection::new("Active Conditions").empty("No active safety interlocks");
        for sensor in self.model.active_conditions().into_iter().take(4) {
            conditions = conditions.row(
                SidecarRow::new(
                    &sensor.name,
                    active_condition_summary(sensor, &self.model),
                    severity_tone(sensor.severity),
                )
                .selected(
                    self.acknowledged_conditions
                        .contains(&sensor.id.to_string()),
                ),
            );
        }
        let sections = vec![
            SidecarSection::new("Safety Interlocks")
                .row(SidecarRow::new(
                    summary
                        .highest_severity
                        .map(SafetySeverity::label)
                        .unwrap_or("normal"),
                    "Simulated monitoring only",
                    severity_tone(summary.highest_severity.unwrap_or(SafetySeverity::Normal)),
                ))
                .row(SidecarRow::new(
                    "Sensors / interlocks",
                    format!(
                        "{} sensors | {} active interlocks",
                        summary.sensor_count, summary.active_condition_count
                    ),
                    if summary.active_condition_count > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                ))
                .row(SidecarRow::new(
                    "Lockouts / incidents",
                    format!(
                        "{} tool lockouts | {} open incidents",
                        summary.locked_out_tool_count, summary.open_incident_count
                    ),
                    if summary.locked_out_tool_count > 0 || summary.open_incident_count > 0 {
                        Tone::Danger
                    } else {
                        Tone::Neutral
                    },
                ))
                .row(SidecarRow::new(
                    "Acknowledged",
                    format!("{} this session", self.acknowledged_count()),
                    Tone::Info,
                )),
            conditions,
        ];
        render_sidecar(ui, "safety.context", &sections)
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
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
        if let Err(error) = self.operad_ui(ui, status) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, status);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui, status: &mut String) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("safety_interlock_dashboard_operad_scroll")
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
                {
                    if self.handle_operad_action(&node_name, status) {
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

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
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

    fn build_operad_view(&self, width: f32) -> SafetyOperadView {
        let lockouts = self.model.evaluate_lockouts();
        let selected_permissives = self
            .selected_tool
            .as_deref()
            .map(|tool_id| self.permissive_rows_for_tool(tool_id))
            .unwrap_or_default();
        let metrics = self.operad_metrics(&selected_permissives);
        let active_rows = self.operad_active_condition_rows();
        let lockout_rows = self.operad_lockout_rows(&lockouts);
        let selected_rows = self.operad_selected_tool_rows(&lockouts, &selected_permissives);
        let sensor_rows = self.operad_sensor_rows();
        let route_rows = self.operad_route_rows();
        let incident_rows = self.operad_incident_rows();
        let audit_rows = self.operad_audit_rows();

        let height = self.operad_view_height(
            width,
            metrics.len(),
            &[
                active_rows.len(),
                lockout_rows.len(),
                selected_rows.len(),
                sensor_rows.len(),
                route_rows.len(),
                incident_rows.len(),
                audit_rows.len(),
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

        add_operad_header(
            &mut document,
            root,
            "SIMULATED OPERATIONS",
            "Safety and Interlock Dashboard",
            "Demo-only safety monitoring, alarm routing, tool lockout, and audit trail",
            &format!(
                "Selected tool: {}",
                self.selected_tool.as_deref().unwrap_or("none")
            ),
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_metric_grid(&mut document, root, width, &metrics);
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.active_interlocks",
            "Active Interlocks",
            "All simulated interlocks are clear",
            &active_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.tool_lockouts",
            "Tool Lockouts",
            "No tool interlocks loaded",
            &lockout_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.selected_tool",
            "Selected Tool Readiness",
            "Select a tool to inspect permissives",
            &selected_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.sensors",
            "Simulated Sensors",
            "No safety sensors loaded",
            &sensor_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.routes",
            "Alarm Routing",
            "No alarm routes loaded",
            &route_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.incidents",
            "Incident Trail",
            "No safety incidents loaded",
            &incident_rows,
        );
        add_operad_spacer(&mut document, root, OPERAD_GAP);
        add_operad_section(
            &mut document,
            root,
            width,
            "safety.audit",
            "Audit Trail",
            "No safety audit events loaded",
            &audit_rows,
        );

        SafetyOperadView { document, size }
    }

    fn operad_view_height(&self, width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
        let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
        height += operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
        for row_count in row_counts {
            height += operad_section_height(*row_count) + OPERAD_GAP;
        }
        height + OPERAD_PAD
    }

    fn operad_metrics(&self, selected_permissives: &[PermissiveRow]) -> Vec<SafetyMetricTile> {
        let summary = self.model.summary();
        let ready_permissives = selected_permissives.iter().filter(|row| row.passes).count();
        let acknowledged_count = self.acknowledged_count();
        vec![
            SafetyMetricTile {
                label: "Overall state".to_string(),
                value: summary
                    .highest_severity
                    .map(SafetySeverity::label)
                    .unwrap_or("blank")
                    .to_string(),
                detail: severity_breakdown(&self.model.sensors),
                tone: severity_tone(summary.highest_severity.unwrap_or(SafetySeverity::Normal)),
            },
            SafetyMetricTile {
                label: "Active interlocks".to_string(),
                value: summary.active_condition_count.to_string(),
                detail: "conditions blocking permissives".to_string(),
                tone: if summary.active_condition_count == 0 {
                    Tone::Success
                } else if summary.highest_severity == Some(SafetySeverity::Critical) {
                    Tone::Danger
                } else {
                    Tone::Warning
                },
            },
            SafetyMetricTile {
                label: "Permissives".to_string(),
                value: format!("{ready_permissives}/{}", selected_permissives.len()),
                detail: "selected tool checks".to_string(),
                tone: if selected_permissives.is_empty()
                    || ready_permissives == selected_permissives.len()
                {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            },
            SafetyMetricTile {
                label: "Tool lockouts".to_string(),
                value: summary.locked_out_tool_count.to_string(),
                detail: "computed from interlocks".to_string(),
                tone: if summary.locked_out_tool_count == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            },
            SafetyMetricTile {
                label: "Open incidents".to_string(),
                value: summary.open_incident_count.to_string(),
                detail: "requiring operator review".to_string(),
                tone: if summary.open_incident_count == 0 {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            },
            SafetyMetricTile {
                label: "Acknowledged".to_string(),
                value: acknowledged_count.to_string(),
                detail: "current UI session".to_string(),
                tone: if acknowledged_count == 0 {
                    Tone::Neutral
                } else {
                    Tone::Info
                },
            },
        ]
    }

    fn operad_active_condition_rows(&self) -> Vec<SafetyOperadRow> {
        self.model
            .active_conditions()
            .into_iter()
            .map(|sensor| {
                let sensor_id = sensor.id.to_string();
                SafetyOperadRow {
                    title: format!("{} - {}", sensor.severity.label(), sensor.name),
                    detail: truncate_middle(
                        format!(
                            "{} {} / limit {} / routed {}",
                            compact_number(sensor.value),
                            sensor.unit,
                            limit_label(&sensor.limit),
                            route_summary(sensor, &self.model)
                        ),
                        96,
                    ),
                    tone: severity_tone(sensor.severity),
                    action_name: None,
                    button_name: (!self.acknowledged_conditions.contains(&sensor_id))
                        .then(|| format!("{OPERAD_ACTION_ACK_CONDITION}{sensor_id}")),
                    button_label: (!self.acknowledged_conditions.contains(&sensor_id))
                        .then_some("Acknowledge"),
                }
            })
            .collect()
    }

    fn operad_lockout_rows(&self, lockouts: &[ToolLockout]) -> Vec<SafetyOperadRow> {
        lockouts
            .iter()
            .map(|lockout| {
                let selected = self.selected_tool.as_deref() == Some(lockout.tool_id.as_str());
                SafetyOperadRow {
                    title: format!("{}{}", if selected { "* " } else { "" }, lockout.tool_name),
                    detail: truncate_middle(
                        if lockout.reasons.is_empty() {
                            "No failed simulated safety condition".to_string()
                        } else {
                            lockout.reasons.join(" / ")
                        },
                        96,
                    ),
                    tone: if lockout.locked_out {
                        Tone::Danger
                    } else {
                        Tone::Success
                    },
                    action_name: Some(format!("{OPERAD_ACTION_SELECT_TOOL}{}", lockout.tool_id)),
                    button_name: None,
                    button_label: None,
                }
            })
            .collect()
    }

    fn operad_selected_tool_rows(
        &self,
        lockouts: &[ToolLockout],
        permissives: &[PermissiveRow],
    ) -> Vec<SafetyOperadRow> {
        let Some(lockout) = self.selected_lockout(lockouts) else {
            return Vec::new();
        };
        let mut rows = vec![SafetyOperadRow {
            title: format!("{} ({})", lockout.tool_name, lockout.tool_id),
            detail: if lockout.reasons.is_empty() {
                "All simulated interlocks clear".to_string()
            } else {
                truncate_middle(lockout.reasons.join(" / "), 96)
            },
            tone: if lockout.locked_out {
                Tone::Danger
            } else {
                Tone::Success
            },
            action_name: None,
            button_name: (lockout.locked_out
                && !self.acknowledged_lockouts.contains(&lockout.tool_id))
            .then(|| format!("{OPERAD_ACTION_ACK_LOCKOUT}{}", lockout.tool_id)),
            button_label: (lockout.locked_out
                && !self.acknowledged_lockouts.contains(&lockout.tool_id))
            .then_some("Acknowledge"),
        }];
        rows.extend(permissives.iter().map(|row| SafetyOperadRow {
            title: format!(
                "{} - {}",
                if row.passes { "pass" } else { "blocked" },
                row.name
            ),
            detail: truncate_middle(
                format!(
                    "{} / {} / {} / limit {} / {}s",
                    row.domain, row.state, row.value, row.limit, row.last_seen_s
                ),
                96,
            ),
            tone: if row.passes {
                Tone::Success
            } else {
                Tone::Danger
            },
            action_name: None,
            button_name: None,
            button_label: None,
        }));
        rows
    }

    fn operad_sensor_rows(&self) -> Vec<SafetyOperadRow> {
        self.model
            .sensors
            .iter()
            .map(|sensor| SafetyOperadRow {
                title: sensor.name.clone(),
                detail: truncate_middle(
                    format!(
                        "{} / {} / {} {} / last {}s",
                        sensor.domain.label(),
                        sensor.state.label(),
                        compact_number(sensor.value),
                        sensor.unit,
                        sensor.last_seen_s
                    ),
                    96,
                ),
                tone: severity_tone(sensor.severity),
                action_name: None,
                button_name: None,
                button_label: None,
            })
            .collect()
    }

    fn operad_route_rows(&self) -> Vec<SafetyOperadRow> {
        let mut rows = self
            .model
            .alarm_routes
            .iter()
            .map(|route| SafetyOperadRow {
                title: format!(
                    "{} - {}",
                    route.domain.label(),
                    route.minimum_severity.label()
                ),
                detail: truncate_middle(
                    format!("{} via {}", route.target.label(), route.channel),
                    96,
                ),
                tone: severity_tone(route.minimum_severity),
                action_name: None,
                button_name: None,
                button_label: None,
            })
            .collect::<Vec<_>>();
        for sensor in self.model.active_conditions() {
            rows.push(SafetyOperadRow {
                title: format!("Active: {}", sensor.name),
                detail: truncate_middle(route_summary(sensor, &self.model), 96),
                tone: severity_tone(sensor.severity),
                action_name: None,
                button_name: None,
                button_label: None,
            });
        }
        rows
    }

    fn operad_incident_rows(&self) -> Vec<SafetyOperadRow> {
        self.model
            .incidents
            .iter()
            .map(|incident| {
                let incident_id = incident.id.to_string();
                SafetyOperadRow {
                    title: format!("{} - {}", incident.id, incident.status.label()),
                    detail: truncate_middle(
                        format!(
                            "{} / opened {} / routed {} / {}",
                            incident.domain.label(),
                            incident.opened_at,
                            route_targets_label(&incident.routed_to),
                            incident.summary
                        ),
                        108,
                    ),
                    tone: severity_tone(incident.severity),
                    action_name: None,
                    button_name: (incident.status != IncidentStatus::Closed
                        && !self.acknowledged_incidents.contains(&incident_id))
                    .then(|| format!("{OPERAD_ACTION_ACK_INCIDENT}{incident_id}")),
                    button_label: (incident.status != IncidentStatus::Closed
                        && !self.acknowledged_incidents.contains(&incident_id))
                    .then_some("Acknowledge"),
                }
            })
            .collect()
    }

    fn operad_audit_rows(&self) -> Vec<SafetyOperadRow> {
        self.model
            .audit_events
            .iter()
            .take(8)
            .map(|event| SafetyOperadRow {
                title: format!("#{} - {}", event.sequence, event.kind.label()),
                detail: truncate_middle(
                    format!("{} / {} / {}", event.timestamp, event.actor, event.message),
                    108,
                ),
                tone: audit_tone(event.kind),
                action_name: None,
                button_name: None,
                button_label: None,
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str, status: &mut String) -> bool {
        if let Some(tool_id) = node_name.strip_prefix(OPERAD_ACTION_SELECT_TOOL) {
            if self.selected_tool.as_deref() != Some(tool_id) {
                self.selected_tool = Some(tool_id.to_string());
                *status = "safety interlock context selected".to_string();
            }
            return true;
        }

        if let Some(sensor_id) = node_name.strip_prefix(OPERAD_ACTION_ACK_CONDITION) {
            if let Some(sensor) = self
                .model
                .sensors
                .iter()
                .find(|sensor| sensor.id.to_string() == sensor_id)
            {
                self.acknowledge_condition(sensor_id.to_string(), sensor.name.clone(), status);
                return true;
            }
        }

        if let Some(tool_id) = node_name.strip_prefix(OPERAD_ACTION_ACK_LOCKOUT) {
            let lockout = self
                .model
                .evaluate_lockouts()
                .into_iter()
                .find(|lockout| lockout.tool_id == tool_id);
            if let Some(lockout) = lockout {
                self.acknowledge_lockout(&lockout, status);
                return true;
            }
        }

        if let Some(incident_id) = node_name.strip_prefix(OPERAD_ACTION_ACK_INCIDENT) {
            let incident = self
                .model
                .incidents
                .iter()
                .find(|incident| incident.id.to_string() == incident_id)
                .cloned();
            if let Some(incident) = incident {
                self.acknowledge_incident(&incident, status);
                return true;
            }
        }

        false
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

fn operad_metric_columns(width: f32) -> usize {
    if width >= 900.0 {
        3
    } else if width >= 560.0 {
        2
    } else {
        1
    }
}

fn operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = operad_metric_columns(width);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_operad_header(
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
            "safety.header",
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
    add_operad_text(
        document,
        header,
        "safety.header.eyebrow",
        eyebrow,
        operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_operad_text(
        document,
        header,
        "safety.header.title",
        title,
        operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_operad_text(
        document,
        header,
        "safety.header.detail",
        detail,
        operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_operad_text(
        document,
        header,
        "safety.header.meta",
        meta,
        operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[SafetyMetricTile],
) {
    let columns = operad_metric_columns(width);
    let grid_height = operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "safety.metrics",
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
                format!("safety.metrics.row.{row_index}"),
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
            add_operad_metric_tile(
                document,
                row,
                &format!("safety.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &SafetyMetricTile,
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
            Some(StrokeStyle::new(operad_tone_color(metric.tone), 1.0)),
            6.0,
        )),
    );
    add_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        operad_text_style(12.0, FontWeight::NORMAL, operad_tone_color(metric.tone)),
        18.0,
    );
}

fn add_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[SafetyOperadRow],
) {
    let height = operad_section_height(rows.len());
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
    add_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_operad_empty_row(document: &mut UiDocument, parent: UiNodeId, name: &str, label: &str) {
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
    add_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &SafetyOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
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
        ColorRgba::new(26, 31, 36, 255),
        Some(StrokeStyle::new(ColorRgba::new(42, 50, 58, 255), 1.0)),
        4.0,
    ));
    if row.action_name.is_some() {
        node = node.with_input(InputBehavior::BUTTON);
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
        .with_visual(UiVisual::panel(operad_tone_color(row.tone), None, 2.0)),
    );
    let button_width = if row.button_name.is_some() {
        112.0
    } else {
        0.0
    };
    let text_width = (row_width - button_width - 28.0).max(120.0);
    let text_column = document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.text"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::px(text_width),
                    layout::px(OPERAD_ROW_HEIGHT - 12.0),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    add_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 64),
        operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        20.0,
    );
    add_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 96),
        operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        18.0,
    );
    if let (Some(button_name), Some(button_label)) = (&row.button_name, row.button_label) {
        let mut options = widgets::ButtonOptions::new(layout::fixed(104.0, 28.0));
        options.text_style =
            operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(12, 16, 20, 255));
        options.visual = UiVisual::panel(
            ColorRgba::new(220, 176, 72, 255),
            Some(StrokeStyle::new(ColorRgba::new(238, 202, 119, 255), 1.0)),
            5.0,
        );
        widgets::button(document, row_node, button_name, button_label, options);
    }
}

fn add_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("safety.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_operad_text(
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

fn operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn audit_tone(kind: SafetyAuditKind) -> Tone {
    match kind {
        SafetyAuditKind::SensorSample => Tone::Info,
        SafetyAuditKind::AlarmRouted => Tone::Warning,
        SafetyAuditKind::ToolLockedOut | SafetyAuditKind::IncidentOpened => Tone::Danger,
        SafetyAuditKind::IncidentUpdated => Tone::Neutral,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_operad_view_audits_common_widths() {
        let panel = SafetyPanel::from_model(SafetySystem::simulated_demo());
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width);
            view.document
                .compute_layout(view.size, &mut ApproxTextMeasurer)
                .expect("safety operad view should lay out");
            let warnings = view.document.audit_layout();
            assert!(warnings.is_empty(), "{warnings:?}");
            assert!(view.document.node_count() > 40);
            assert!(!view.document.paint_list().is_empty());
        }
    }

    #[test]
    fn safety_operad_actions_update_panel_state() {
        let mut panel = SafetyPanel::from_model(SafetySystem::simulated_demo());
        let mut status = String::new();
        let target_tool = panel
            .model
            .evaluate_lockouts()
            .last()
            .expect("demo safety model has lockouts")
            .tool_id
            .clone();

        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_TOOL}{target_tool}"),
            &mut status
        ));
        assert_eq!(panel.selected_tool.as_deref(), Some(target_tool.as_str()));

        let sensor_id = panel
            .model
            .active_conditions()
            .first()
            .expect("demo safety model has active conditions")
            .id
            .to_string();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_ACK_CONDITION}{sensor_id}"),
            &mut status
        ));
        assert!(panel.acknowledged_conditions.contains(&sensor_id));
    }
}
