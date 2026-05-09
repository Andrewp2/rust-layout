use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, vec2};
use layout_model::{
    process_control::{
        ControlAction, ControlActionId, ControlActionState, ControlAuditEvent, ControlAuditKind,
        ControlLoop, ControlLoopId, ControlTrendPoint, ManipulatedParameter, ProcessControlModel,
        RecipeParameterAdjustment,
    },
    recipe::{RecipeUnit, format_parameter_value},
    yield_analysis::YieldAnalysis,
};
use web_time::Instant;

use crate::ui_chrome;

pub(crate) struct ProcessControlPanel {
    model: ProcessControlModel,
    selected_loop: Option<ControlLoopId>,
    selected_action: Option<ControlActionId>,
    actor: String,
    session_started: Instant,
}

impl ProcessControlPanel {
    pub(crate) fn from_model(model: ProcessControlModel) -> Self {
        let selected_loop = model
            .loops
            .first()
            .map(|loop_definition| loop_definition.id.clone());
        let selected_action = selected_loop.as_ref().and_then(|loop_id| {
            model
                .actions_for_loop(loop_id)
                .first()
                .map(|action| action.id.clone())
        });
        Self {
            model,
            selected_loop,
            selected_action,
            actor: "process.engineer".to_string(),
            session_started: Instant::now(),
        }
    }

    pub(crate) fn model(&self) -> &ProcessControlModel {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, analysis: &YieldAnalysis) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Process Control");
        let Some(loop_id) = self.selected_loop.clone() else {
            ui_chrome::empty_state(ui, "No control loops loaded");
            return;
        };
        let Some(loop_definition) = self.model.loop_by_id(&loop_id).cloned() else {
            ui_chrome::empty_state(ui, "Selected control loop is missing");
            return;
        };
        let trend = self.model.trend_for_loop(&loop_id).to_vec();
        let actions = self
            .model
            .actions_for_loop(&loop_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let (state_label, state_tone, state_detail) =
            loop_control_state(&loop_definition, &trend, &actions);

        ui.separator();
        ui_chrome::section_label(ui, "Control Health");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, &state_label, state_tone);
            ui.label(RichText::new(state_detail).color(ui.visuals().weak_text_color()));
        });
        if let Some(latest) = trend.last() {
            ui.label(format!(
                "Latest {} from {} {}",
                format_measurement(latest.value, &latest.unit),
                latest.source.lot_id,
                latest.source.wafer_id
            ));
            ui.label(format!(
                "EWMA error {} {}, deadband +/-{} {}",
                format_signed(latest.ewma_error),
                latest.unit,
                format_number(loop_definition.deadband.abs()),
                latest.unit
            ));
        }

        ui.separator();
        selected_loop_identity_ui(ui, &loop_definition);

        ui.separator();
        output_limits_ui(ui, &loop_definition, trend.last());

        ui.separator();
        ui_chrome::section_label(ui, "Recipe Guardrails");
        manipulated_parameter_table(
            ui,
            &loop_definition.manipulated_parameters,
            &loop_definition.output.unit,
        );

        ui.separator();
        ui_chrome::section_label(ui, "Yield Link");
        if let Some(correlation) = analysis
            .correlations
            .iter()
            .find(|record| record.measurement_name == loop_definition.output.measurement_name)
        {
            ui.label(format!(
                "Correlation to failure rate: {:+.2}",
                correlation.correlation_to_failure_rate
            ));
            ui.label(&correlation.root_cause_hint);
        } else {
            ui_chrome::empty_state(ui, "No yield correlation loaded for this output");
        }

        ui.separator();
        ui_chrome::section_label(ui, "Audit Trail");
        egui::ScrollArea::vertical()
            .id_salt("process_control_audit_scroll")
            .max_height(160.0)
            .show(ui, |ui| {
                for event in self
                    .model
                    .audit_for_loop(&loop_id)
                    .into_iter()
                    .rev()
                    .take(10)
                {
                    audit_event_row(ui, event);
                }
            });
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, analysis: &YieldAnalysis, status: &mut String) {
        self.ensure_selection();
        let mut requested_transition = None;

        egui::ScrollArea::vertical()
            .id_salt("process_control_dashboard_scroll")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Process engineering",
                    "Run-to-Run Control",
                    "",
                    |ui| {
                        ui.label("Loop");
                        let mut selected = self.selected_loop.clone();
                        let selected_text = selected
                            .as_ref()
                            .and_then(|id| self.model.loop_by_id(id))
                            .map(|loop_definition| loop_definition.name.clone())
                            .unwrap_or_else(|| "none".to_string());
                        egui::ComboBox::from_id_salt("process_control_loop_picker")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for loop_definition in &self.model.loops {
                                    ui.selectable_value(
                                        &mut selected,
                                        Some(loop_definition.id.clone()),
                                        &loop_definition.name,
                                    );
                                }
                            });
                        if selected != self.selected_loop {
                            self.selected_loop = selected;
                            self.selected_action = None;
                            self.ensure_selection();
                            *status = "process control loop selected".to_string();
                        }
                    },
                );

                self.loop_portfolio_ui(ui, status);
                ui.separator();

                let Some(loop_id) = self.selected_loop.clone() else {
                    ui_chrome::empty_state(ui, "No process-control loops loaded");
                    return;
                };
                let Some(loop_definition) = self.model.loop_by_id(&loop_id).cloned() else {
                    ui_chrome::empty_state(ui, "Selected process-control loop is missing");
                    return;
                };
                let trend = self.model.trend_for_loop(&loop_id).to_vec();
                let actions = self
                    .model
                    .actions_for_loop(&loop_id)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();

                control_overview_metrics_ui(ui, analysis, &loop_definition, &trend, &actions);

                ui.separator();
                let available_width = ui.available_width();
                if available_width < 760.0 {
                    ui_chrome::section_label(ui, "Target vs measured");
                    draw_control_chart(ui, &loop_definition, &trend);
                    ui.separator();
                    metrology_feedback_ui(ui, analysis, &loop_definition, &trend);
                    ui.separator();
                    requested_transition = self.control_actions_ui(ui, &loop_definition, &actions);
                    ui.separator();
                    selected_loop_details_ui(ui, analysis, &loop_definition, &trend, &actions);
                } else if available_width < 1120.0 {
                    ui.columns(2, |columns| {
                        ui_chrome::section_label(&mut columns[0], "Target vs measured");
                        draw_control_chart(&mut columns[0], &loop_definition, &trend);
                        columns[0].separator();
                        metrology_feedback_ui(&mut columns[0], analysis, &loop_definition, &trend);

                        requested_transition =
                            self.control_actions_ui(&mut columns[1], &loop_definition, &actions);
                        columns[1].separator();
                        selected_loop_details_ui(
                            &mut columns[1],
                            analysis,
                            &loop_definition,
                            &trend,
                            &actions,
                        );
                    });
                } else {
                    ui.columns(3, |columns| {
                        ui_chrome::section_label(&mut columns[0], "Target vs measured");
                        draw_control_chart(&mut columns[0], &loop_definition, &trend);
                        columns[0].separator();
                        metrology_feedback_ui(&mut columns[0], analysis, &loop_definition, &trend);

                        requested_transition =
                            self.control_actions_ui(&mut columns[1], &loop_definition, &actions);

                        selected_loop_details_ui(
                            &mut columns[2],
                            analysis,
                            &loop_definition,
                            &trend,
                            &actions,
                        );
                    });
                }
            });

        if let Some((action_id, transition)) = requested_transition {
            self.apply_requested_transition(action_id, transition, status);
        }
    }

    fn loop_portfolio_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        if self.model.loops.is_empty() {
            return;
        }

        ui_chrome::section_label(ui, "Control loops");
        let mut next_loop = None;
        ui.horizontal_wrapped(|ui| {
            for loop_definition in &self.model.loops {
                let actions = self
                    .model
                    .actions_for_loop(&loop_definition.id)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                let trend = self.model.trend_for_loop(&loop_definition.id);
                let (state_label, state_tone, state_detail) =
                    loop_control_state(loop_definition, trend, &actions);
                let selected = self.selected_loop.as_ref() == Some(&loop_definition.id);
                let label = format!("{}  {}", loop_definition.name, state_label);
                let response = ui
                    .selectable_label(selected, label)
                    .on_hover_text(state_detail);
                if response.clicked() {
                    next_loop = Some(loop_definition.id.clone());
                }
                if selected {
                    ui.colored_label(state_tone.color(), loop_definition.id.as_str());
                }
            }
        });

        if let Some(loop_id) = next_loop {
            if self.selected_loop.as_ref() != Some(&loop_id) {
                self.selected_loop = Some(loop_id);
                self.selected_action = None;
                self.ensure_selection();
                *status = "process control loop selected".to_string();
            }
        }
    }

    fn control_actions_ui(
        &mut self,
        ui: &mut egui::Ui,
        loop_definition: &ControlLoop,
        actions: &[ControlAction],
    ) -> Option<(ControlActionId, RequestedTransition)> {
        ui_chrome::section_label(ui, "Recommendations");
        if actions.is_empty() {
            ui_chrome::empty_state(ui, "No recommended adjustments for the selected loop");
            return None;
        }

        let mut requested_transition = None;
        let counts = action_counts(actions);
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} active", counts.active()),
                action_queue_tone(&counts),
            );
            if counts.held > 0 {
                ui_chrome::status_pill(
                    ui,
                    &format!("{} held", counts.held),
                    ui_chrome::Tone::Warning,
                );
            }
            if counts.proposed > 0 {
                ui_chrome::status_pill(
                    ui,
                    &format!("{} proposed", counts.proposed),
                    ui_chrome::Tone::Info,
                );
            }
            if counts.approved > 0 {
                ui_chrome::status_pill(
                    ui,
                    &format!("{} approved", counts.approved),
                    ui_chrome::Tone::Success,
                );
            }
            if counts.applied > 0 {
                ui_chrome::status_pill(
                    ui,
                    &format!("{} applied", counts.applied),
                    ui_chrome::Tone::Success,
                );
            }
            if counts.rejected > 0 {
                ui_chrome::status_pill(
                    ui,
                    &format!("{} rejected", counts.rejected),
                    ui_chrome::Tone::Danger,
                );
            }
        });

        ui.add_space(2.0);
        let compact = ui.ctx().content_rect().width() < 760.0 || ui.available_width() < 520.0;
        for action in actions {
            let selected = self.selected_action.as_ref() == Some(&action.id);
            if compact {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(selected, short_action_id(action.id.as_str()))
                        .clicked()
                    {
                        self.selected_action = Some(action.id.clone());
                    }
                    ui_chrome::status_pill(
                        ui,
                        action.state.label(),
                        action_state_tone(action.state),
                    );
                    ui.label(format!(
                        "{} {}",
                        action.source.lot_id, action.source.wafer_id
                    ));
                    ui.colored_label(
                        confidence_tone(action.confidence, loop_definition.minimum_confidence)
                            .color(),
                        format_percent(action.confidence),
                    );
                });
                ui.add(
                    egui::Label::new(
                        RichText::new(recommendation_reason(action, loop_definition))
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    )
                    .wrap(),
                );
            } else {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(selected, short_action_id(action.id.as_str()))
                        .clicked()
                    {
                        self.selected_action = Some(action.id.clone());
                    }
                    ui_chrome::status_pill(
                        ui,
                        action.state.label(),
                        action_state_tone(action.state),
                    );
                    ui.label(format!(
                        "{} {}",
                        action.source.lot_id, action.source.wafer_id
                    ));
                    ui.label(format!(
                        "EWMA {} {}",
                        format_signed(action.ewma_error),
                        loop_definition.output.unit
                    ));
                    ui.colored_label(
                        confidence_tone(action.confidence, loop_definition.minimum_confidence)
                            .color(),
                        format!("confidence {}", format_percent(action.confidence)),
                    );
                });
            }
        }

        let selected_action = self
            .selected_action
            .as_ref()
            .and_then(|id| actions.iter().find(|action| &action.id == id))
            .or_else(|| actions.first());
        let Some(action) = selected_action else {
            return None;
        };

        ui.separator();
        selected_action_detail_ui(ui, loop_definition, action, &mut requested_transition);

        requested_transition
    }

    fn apply_requested_transition(
        &mut self,
        action_id: ControlActionId,
        transition: RequestedTransition,
        status: &mut String,
    ) {
        let timestamp = self.session_timestamp();
        let actor = self.actor.clone();
        let result = match transition {
            RequestedTransition::Approve => self.model.approve_action(
                &action_id,
                actor,
                timestamp,
                "manual approval from process-control panel",
            ),
            RequestedTransition::Reject => self.model.reject_action(
                &action_id,
                actor,
                timestamp,
                "manual rejection from process-control panel",
            ),
            RequestedTransition::Apply => self.model.apply_action(
                &action_id,
                actor,
                timestamp,
                "recipe override staged for next run",
            ),
        };

        match result {
            Ok(()) => {
                *status = format!("process control: {} {}", transition.label(), action_id);
            }
            Err(error) => {
                *status = format!("process control blocked: {error}");
            }
        }
    }

    fn ensure_selection(&mut self) {
        let loop_exists = self
            .selected_loop
            .as_ref()
            .is_some_and(|id| self.model.loop_by_id(id).is_some());
        if !loop_exists {
            self.selected_loop = self
                .model
                .loops
                .first()
                .map(|loop_definition| loop_definition.id.clone());
        }

        let Some(loop_id) = self.selected_loop.clone() else {
            self.selected_action = None;
            return;
        };
        let action_exists = self.selected_action.as_ref().is_some_and(|id| {
            self.model
                .actions_for_loop(&loop_id)
                .iter()
                .any(|action| &action.id == id)
        });
        if !action_exists {
            self.selected_action = self
                .model
                .actions_for_loop(&loop_id)
                .first()
                .map(|action| action.id.clone());
        }
    }

    fn session_timestamp(&self) -> String {
        format!("session+{}s", self.session_started.elapsed().as_secs())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestedTransition {
    Approve,
    Reject,
    Apply,
}

impl RequestedTransition {
    fn label(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
            Self::Apply => "applied",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ActionStateCounts {
    proposed: usize,
    approved: usize,
    held: usize,
    rejected: usize,
    applied: usize,
}

impl ActionStateCounts {
    fn active(self) -> usize {
        self.proposed + self.approved + self.held
    }
}

#[derive(Debug)]
struct GuardrailCheck {
    label: &'static str,
    status: &'static str,
    tone: ui_chrome::Tone,
    detail: String,
}

fn control_overview_metrics_ui(
    ui: &mut egui::Ui,
    analysis: &YieldAnalysis,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
    actions: &[ControlAction],
) {
    let (state_label, state_tone, state_detail) =
        loop_control_state(loop_definition, trend, actions);
    let latest = trend.last();
    let latest_value = latest
        .map(|point| format_measurement(point.value, &point.unit))
        .unwrap_or_else(|| "-".to_string());
    let latest_detail = latest
        .map(|point| {
            format!(
                "target {}, {}",
                format_measurement(point.target, &point.unit),
                point.source.tool_run_id
            )
        })
        .unwrap_or_else(|| "waiting for metrology".to_string());
    let ewma_value = latest
        .map(|point| format!("{} {}", format_signed(point.ewma_error), point.unit))
        .unwrap_or_else(|| "-".to_string());
    let ewma_tone = latest
        .map(|point| {
            if point.ewma_error.abs() > loop_definition.deadband.abs() {
                ui_chrome::Tone::Warning
            } else {
                ui_chrome::Tone::Success
            }
        })
        .unwrap_or(ui_chrome::Tone::Neutral);
    let yield_value = latest
        .and_then(|point| point.yield_fraction)
        .map(format_percent)
        .unwrap_or_else(|| "-".to_string());
    let yield_detail = latest
        .map(|point| yield_context_detail(analysis, point))
        .unwrap_or_else(|| "no wafer summary".to_string());
    let counts = action_counts(actions);
    let recommendation_value = if counts.active() > 0 {
        format!("{} active", counts.active())
    } else {
        "none".to_string()
    };
    let recommendation_detail = format!(
        "{} proposed / {} approved / {} held / {} applied / {} rejected",
        counts.proposed, counts.approved, counts.held, counts.applied, counts.rejected
    );
    let ewma_detail = format!(
        "deadband +/-{} {}",
        format_number(loop_definition.deadband.abs()),
        loop_definition.output.unit
    );

    ui_chrome::metric_tiles(
        ui,
        &[
            ("Loop state", state_label, state_detail.as_str(), state_tone),
            (
                "Latest output",
                latest_value,
                latest_detail.as_str(),
                latest
                    .map(|point| point_tone(point, loop_definition))
                    .unwrap_or(ui_chrome::Tone::Neutral),
            ),
            ("EWMA error", ewma_value, ewma_detail.as_str(), ewma_tone),
            (
                "Yield context",
                yield_value,
                yield_detail.as_str(),
                ui_chrome::Tone::Neutral,
            ),
            (
                "Recommendations",
                recommendation_value,
                recommendation_detail.as_str(),
                action_queue_tone(&counts),
            ),
        ],
    );
}

fn selected_loop_details_ui(
    ui: &mut egui::Ui,
    analysis: &YieldAnalysis,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
    actions: &[ControlAction],
) {
    ui_chrome::section_label(ui, "Selected loop details");
    let (state_label, state_tone, state_detail) =
        loop_control_state(loop_definition, trend, actions);
    ui.horizontal_wrapped(|ui| {
        ui_chrome::status_pill(ui, &state_label, state_tone);
        ui.label(RichText::new(state_detail).color(ui.visuals().weak_text_color()));
    });

    selected_loop_identity_ui(ui, loop_definition);
    ui.separator();
    output_limits_ui(ui, loop_definition, trend.last());

    ui.separator();
    ui_chrome::section_label(ui, "Recipe correction limits");
    manipulated_parameter_table(
        ui,
        &loop_definition.manipulated_parameters,
        &loop_definition.output.unit,
    );

    ui.separator();
    ui_chrome::section_label(ui, "Yield link");
    if let Some(correlation) = analysis
        .correlations
        .iter()
        .find(|record| record.measurement_name == loop_definition.output.measurement_name)
    {
        ui.label(format!(
            "Failure-rate correlation {:+.2}",
            correlation.correlation_to_failure_rate
        ));
        ui.add(egui::Label::new(&correlation.root_cause_hint).wrap());
    } else {
        ui_chrome::empty_state(ui, "No yield correlation loaded for this output");
    }
}

fn selected_loop_identity_ui(ui: &mut egui::Ui, loop_definition: &ControlLoop) {
    ui_chrome::section_label(ui, &loop_definition.name);
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            detail_row(ui, "Route", &loop_definition.route_id);
            detail_row(ui, "Step", &loop_definition.process_step_id);
            detail_row(ui, "Tool", &loop_definition.tool_id);
            detail_row(ui, "Class", &loop_definition.tool_class.to_string());
            detail_row(ui, "Recipe", &loop_definition.recipe.to_string());
            detail_row(
                ui,
                "EWMA",
                &format!("lambda {}", format_number(loop_definition.ewma_lambda)),
            );
            detail_row(
                ui,
                "Hold gate",
                &format!(
                    "minimum confidence {}",
                    format_percent(loop_definition.minimum_confidence)
                ),
            );
            if let Some(layer) = loop_definition.output.process_layer {
                detail_row(ui, "Layer", layer.as_technology_name());
            }
        });
}

fn output_limits_ui(
    ui: &mut egui::Ui,
    loop_definition: &ControlLoop,
    latest: Option<&ControlTrendPoint>,
) {
    ui_chrome::section_label(ui, "Metrology and limits");
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            detail_row(ui, "Output", &loop_definition.output.label);
            detail_row(
                ui,
                "Target",
                &format_measurement(loop_definition.output.target, &loop_definition.output.unit),
            );
            detail_row(ui, "Spec", &spec_label(loop_definition));
            detail_row(
                ui,
                "Deadband",
                &format!(
                    "+/-{} {}",
                    format_number(loop_definition.deadband.abs()),
                    loop_definition.output.unit
                ),
            );
            detail_row(ui, "Metrology", &loop_definition.output.measurement_step_id);
            detail_row(ui, "Measurement", &loop_definition.output.measurement_name);
            if let Some(point) = latest {
                detail_row(ui, "Latest ID", &point.measurement_id);
                detail_row(
                    ui,
                    "Latest status",
                    if point.in_spec {
                        "inside spec"
                    } else {
                        "outside spec"
                    },
                );
            }
        });
}

fn metrology_feedback_ui(
    ui: &mut egui::Ui,
    analysis: &YieldAnalysis,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
) {
    ui_chrome::section_label(ui, "Metrology feedback");
    if trend.is_empty() {
        ui_chrome::empty_state(ui, "No metrology feedback loaded for this loop");
        return;
    }

    if let Some(latest) = trend.last() {
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                if latest.in_spec {
                    "inside spec"
                } else {
                    "outside spec"
                },
                point_tone(latest, loop_definition),
            );
            if latest.actionable(loop_definition) {
                ui_chrome::status_pill(ui, "correction eligible", ui_chrome::Tone::Warning);
            }
            ui.label(format!(
                "{} / {} / {}",
                latest.source.lot_id, latest.source.wafer_id, latest.source.tool_run_id
            ));
        });
        ui.add(
            egui::Label::new(
                RichText::new(yield_context_detail(analysis, latest))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .wrap(),
        );
    }

    control_history_table(ui, loop_definition, trend);
}

fn selected_action_detail_ui(
    ui: &mut egui::Ui,
    loop_definition: &ControlLoop,
    action: &ControlAction,
    requested_transition: &mut Option<(ControlActionId, RequestedTransition)>,
) {
    ui_chrome::section_label(ui, "Selected recommendation");
    ui.horizontal_wrapped(|ui| {
        ui_chrome::status_pill(ui, action.state.label(), action_state_tone(action.state));
        ui.add(egui::Label::new(RichText::new(action.id.as_str()).strong()).wrap());
    });
    ui.add(
        egui::Label::new(
            RichText::new(recommendation_reason(action, loop_definition))
                .color(recommendation_tone(action, loop_definition).color()),
        )
        .wrap(),
    );
    ui.add(
        egui::Label::new(format!(
            "{} from {} {} at {}",
            loop_definition.output.label,
            action.source.lot_id,
            action.source.wafer_id,
            action.proposed_at
        ))
        .wrap(),
    );

    let error_detail = format!(
        "measured {}, target {}",
        format_measurement(action.measured_value, &loop_definition.output.unit),
        format_measurement(action.target_value, &loop_definition.output.unit)
    );
    let confidence_detail = format!(
        "minimum {}",
        format_percent(loop_definition.minimum_confidence)
    );
    let ewma_detail = format!(
        "deadband +/-{} {}",
        format_number(loop_definition.deadband.abs()),
        loop_definition.output.unit
    );
    ui_chrome::metric_tiles(
        ui,
        &[
            (
                "Instant error",
                format!(
                    "{} {}",
                    format_signed(action.error),
                    loop_definition.output.unit
                ),
                error_detail.as_str(),
                if action_in_spec(action, loop_definition) {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Danger
                },
            ),
            (
                "EWMA error",
                format!(
                    "{} {}",
                    format_signed(action.ewma_error),
                    loop_definition.output.unit
                ),
                ewma_detail.as_str(),
                if action.ewma_error.abs() > loop_definition.deadband.abs() {
                    ui_chrome::Tone::Warning
                } else {
                    ui_chrome::Tone::Neutral
                },
            ),
            (
                "Confidence",
                format_percent(action.confidence),
                confidence_detail.as_str(),
                confidence_tone(action.confidence, loop_definition.minimum_confidence),
            ),
        ],
    );

    ui.separator();
    ui_chrome::section_label(ui, "Pre-apply checks");
    guardrail_check_table(ui, &action_guardrail_checks(action, loop_definition));

    ui.separator();
    ui_chrome::section_label(ui, "Recipe correction");
    adjustment_table(ui, &action.adjustments);
    ui.add(
        egui::Label::new(
            RichText::new(&action.rationale)
                .small()
                .color(ui.visuals().weak_text_color()),
        )
        .wrap(),
    );

    ui.separator();
    ui.horizontal_wrapped(|ui| match action.state {
        ControlActionState::Proposed => {
            if ui.button("Approve").clicked() {
                *requested_transition = Some((action.id.clone(), RequestedTransition::Approve));
            }
            if ui.button("Reject").clicked() {
                *requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
            }
        }
        ControlActionState::Approved => {
            if ui.button("Apply").clicked() {
                *requested_transition = Some((action.id.clone(), RequestedTransition::Apply));
            }
            if ui.button("Reject").clicked() {
                *requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
            }
        }
        ControlActionState::Held => {
            ui.colored_label(action_state_color(action.state), "Held for engineer review");
            if ui.button("Reject").clicked() {
                *requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
            }
        }
        ControlActionState::Rejected | ControlActionState::Applied => {
            ui.label("Final state");
        }
    });
}

fn guardrail_check_table(ui: &mut egui::Ui, checks: &[GuardrailCheck]) {
    egui::Grid::new(ui.next_auto_id())
        .striped(true)
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.strong("Check");
            ui.strong("Status");
            ui.strong("Detail");
            ui.end_row();
            for check in checks {
                ui.label(check.label);
                ui.colored_label(check.tone.color(), check.status);
                ui.add(egui::Label::new(&check.detail).wrap());
                ui.end_row();
            }
        });
}

fn manipulated_parameter_table(
    ui: &mut egui::Ui,
    parameters: &[ManipulatedParameter],
    output_unit: &str,
) {
    if parameters.is_empty() {
        ui_chrome::empty_state(ui, "No recipe parameters are bound to this loop");
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt(ui.next_auto_id())
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(72.0)
                .show(ui, |ui| {
                    ui.strong("Parameter");
                    ui.strong("Current");
                    ui.strong("Bounds");
                    ui.strong("Max step");
                    ui.strong("Sensitivity");
                    ui.end_row();
                    for parameter in parameters {
                        let unit = parameter_unit(parameter.unit);
                        ui.label(&parameter.label);
                        ui.label(format!(
                            "{} {}",
                            format_number(parameter.current_value),
                            unit
                        ));
                        ui.label(format!(
                            "{}..{} {}",
                            format_number(parameter.lower_bound),
                            format_number(parameter.upper_bound),
                            unit
                        ));
                        ui.label(format!(
                            "+/-{} {}",
                            format_number(parameter.max_delta.abs()),
                            unit
                        ));
                        ui.label(format!(
                            "{} {} / {}",
                            format_signed(parameter.output_sensitivity),
                            output_unit,
                            unit
                        ));
                        ui.end_row();
                    }
                });
        });
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).color(ui.visuals().weak_text_color()));
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}

fn loop_control_state(
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
    actions: &[ControlAction],
) -> (String, ui_chrome::Tone, String) {
    let counts = action_counts(actions);
    if counts.held > 0 {
        return (
            "Held".to_string(),
            ui_chrome::Tone::Warning,
            format!("{} recommendation requires review", counts.held),
        );
    }
    if counts.approved > 0 {
        return (
            "Approved".to_string(),
            ui_chrome::Tone::Success,
            format!("{} correction staged for apply", counts.approved),
        );
    }
    if counts.proposed > 0 {
        return (
            "Review".to_string(),
            ui_chrome::Tone::Info,
            format!("{} proposed recipe correction", counts.proposed),
        );
    }

    let Some(latest) = trend.last() else {
        return (
            "No data".to_string(),
            ui_chrome::Tone::Neutral,
            "waiting for metrology feedback".to_string(),
        );
    };

    if !latest.in_spec {
        return (
            "Out of spec".to_string(),
            ui_chrome::Tone::Danger,
            format!(
                "{} vs spec {}",
                format_measurement(latest.value, &latest.unit),
                spec_label(loop_definition)
            ),
        );
    }
    if latest.ewma_error.abs() > loop_definition.deadband.abs() {
        return (
            "EWMA drift".to_string(),
            ui_chrome::Tone::Warning,
            format!(
                "{} {} exceeds +/-{} {} deadband",
                format_signed(latest.ewma_error),
                latest.unit,
                format_number(loop_definition.deadband.abs()),
                latest.unit
            ),
        );
    }

    (
        "In control".to_string(),
        ui_chrome::Tone::Success,
        format!(
            "latest {} on target {}",
            latest.source.wafer_id,
            format_measurement(latest.target, &latest.unit)
        ),
    )
}

fn action_counts(actions: &[ControlAction]) -> ActionStateCounts {
    let mut counts = ActionStateCounts::default();
    for action in actions {
        match action.state {
            ControlActionState::Proposed => counts.proposed += 1,
            ControlActionState::Approved => counts.approved += 1,
            ControlActionState::Rejected => counts.rejected += 1,
            ControlActionState::Applied => counts.applied += 1,
            ControlActionState::Held => counts.held += 1,
        }
    }
    counts
}

fn action_queue_tone(counts: &ActionStateCounts) -> ui_chrome::Tone {
    if counts.held > 0 {
        ui_chrome::Tone::Warning
    } else if counts.approved > 0 {
        ui_chrome::Tone::Success
    } else if counts.proposed > 0 {
        ui_chrome::Tone::Info
    } else {
        ui_chrome::Tone::Neutral
    }
}

fn point_tone(point: &ControlTrendPoint, loop_definition: &ControlLoop) -> ui_chrome::Tone {
    if !point.in_spec {
        ui_chrome::Tone::Danger
    } else if point.ewma_error.abs() > loop_definition.deadband.abs() {
        ui_chrome::Tone::Warning
    } else {
        ui_chrome::Tone::Success
    }
}

fn action_state_tone(state: ControlActionState) -> ui_chrome::Tone {
    match state {
        ControlActionState::Proposed => ui_chrome::Tone::Info,
        ControlActionState::Approved | ControlActionState::Applied => ui_chrome::Tone::Success,
        ControlActionState::Rejected => ui_chrome::Tone::Danger,
        ControlActionState::Held => ui_chrome::Tone::Warning,
    }
}

fn confidence_tone(confidence: f64, minimum_confidence: f64) -> ui_chrome::Tone {
    if confidence >= minimum_confidence {
        ui_chrome::Tone::Success
    } else if confidence >= minimum_confidence * 0.85 {
        ui_chrome::Tone::Warning
    } else {
        ui_chrome::Tone::Danger
    }
}

fn recommendation_tone(action: &ControlAction, loop_definition: &ControlLoop) -> ui_chrome::Tone {
    match action.state {
        ControlActionState::Held => ui_chrome::Tone::Warning,
        ControlActionState::Rejected => ui_chrome::Tone::Danger,
        ControlActionState::Applied | ControlActionState::Approved => ui_chrome::Tone::Success,
        ControlActionState::Proposed => {
            confidence_tone(action.confidence, loop_definition.minimum_confidence)
        }
    }
}

fn recommendation_reason(action: &ControlAction, loop_definition: &ControlLoop) -> String {
    match action.state {
        ControlActionState::Held => format!(
            "Hold: confidence {} is below the {} release gate.",
            format_percent(action.confidence),
            format_percent(loop_definition.minimum_confidence)
        ),
        ControlActionState::Proposed => {
            if action.ewma_error.abs() > loop_definition.deadband.abs() {
                format!(
                    "Approve to stage a damped recipe correction for EWMA error {} {}.",
                    format_signed(action.ewma_error),
                    loop_definition.output.unit
                )
            } else {
                "Review before approval; the latest EWMA is inside deadband.".to_string()
            }
        }
        ControlActionState::Approved => {
            "Apply to make this the next-run recipe override.".to_string()
        }
        ControlActionState::Applied => "Applied to the recipe queue.".to_string(),
        ControlActionState::Rejected => "Rejected; retained for audit traceability.".to_string(),
    }
}

fn short_action_id(id: &str) -> String {
    id.strip_prefix("PCA-").unwrap_or(id).to_string()
}

fn action_guardrail_checks(
    action: &ControlAction,
    loop_definition: &ControlLoop,
) -> Vec<GuardrailCheck> {
    let confidence_ok = action.confidence >= loop_definition.minimum_confidence;
    let deadband_active = action.ewma_error.abs() > loop_definition.deadband.abs();
    let (bounds_ok, bounds_detail) = adjustment_bounds_status(action);
    let (step_status, step_tone, step_detail) = step_limit_status(action, loop_definition);

    vec![
        GuardrailCheck {
            label: "Confidence",
            status: if confidence_ok { "OK" } else { "Hold" },
            tone: confidence_tone(action.confidence, loop_definition.minimum_confidence),
            detail: format!(
                "{} observed, {} required",
                format_percent(action.confidence),
                format_percent(loop_definition.minimum_confidence)
            ),
        },
        GuardrailCheck {
            label: "Deadband",
            status: if deadband_active { "Active" } else { "Observe" },
            tone: if deadband_active {
                ui_chrome::Tone::Warning
            } else {
                ui_chrome::Tone::Neutral
            },
            detail: format!(
                "EWMA {} {}, deadband +/-{} {}",
                format_signed(action.ewma_error),
                loop_definition.output.unit,
                format_number(loop_definition.deadband.abs()),
                loop_definition.output.unit
            ),
        },
        GuardrailCheck {
            label: "Spec limits",
            status: if action_in_spec(action, loop_definition) {
                "Inside"
            } else {
                "Outside"
            },
            tone: if action_in_spec(action, loop_definition) {
                ui_chrome::Tone::Success
            } else {
                ui_chrome::Tone::Danger
            },
            detail: format!(
                "{} measured against {}",
                format_measurement(action.measured_value, &loop_definition.output.unit),
                spec_label(loop_definition)
            ),
        },
        GuardrailCheck {
            label: "Recipe bounds",
            status: if bounds_ok { "OK" } else { "Clamp" },
            tone: if bounds_ok {
                ui_chrome::Tone::Success
            } else {
                ui_chrome::Tone::Warning
            },
            detail: bounds_detail,
        },
        GuardrailCheck {
            label: "Step limit",
            status: step_status,
            tone: step_tone,
            detail: step_detail,
        },
    ]
}

fn adjustment_bounds_status(action: &ControlAction) -> (bool, String) {
    if action.adjustments.is_empty() {
        return (true, "No parameter deltas proposed".to_string());
    }

    let outside = action.adjustments.iter().find(|adjustment| {
        adjustment
            .proposed_value
            .as_f64()
            .is_some_and(|value| value < adjustment.lower_bound || value > adjustment.upper_bound)
    });
    if let Some(adjustment) = outside {
        (
            false,
            format!(
                "{} would exceed {}..{} {}",
                adjustment.label,
                format_number(adjustment.lower_bound),
                format_number(adjustment.upper_bound),
                parameter_unit(adjustment.unit)
            ),
        )
    } else {
        (
            true,
            format!(
                "{} parameter delta(s) remain inside bounds",
                action.adjustments.len()
            ),
        )
    }
}

fn step_limit_status(
    action: &ControlAction,
    loop_definition: &ControlLoop,
) -> (&'static str, ui_chrome::Tone, String) {
    let mut limiting_parameter = None;
    for adjustment in &action.adjustments {
        let Some(parameter) = loop_definition
            .manipulated_parameters
            .iter()
            .find(|parameter| parameter.key == adjustment.parameter_key)
        else {
            continue;
        };
        let max_delta = parameter.max_delta.abs();
        if max_delta <= f64::EPSILON {
            continue;
        }
        let ratio = (adjustment.delta.abs() / max_delta).clamp(0.0, 1.0);
        let replace_limit = match limiting_parameter {
            Some((current_ratio, _, _)) => ratio > current_ratio,
            None => true,
        };
        if replace_limit {
            limiting_parameter = Some((ratio, adjustment.label.as_str(), parameter.unit));
        }
    }

    let Some((ratio, label, unit)) = limiting_parameter else {
        return (
            "OK",
            ui_chrome::Tone::Neutral,
            "No per-run step limit configured for this action".to_string(),
        );
    };
    let tone = if ratio >= 0.95 {
        ui_chrome::Tone::Warning
    } else {
        ui_chrome::Tone::Success
    };
    let status = if ratio >= 0.95 { "At limit" } else { "OK" };
    (
        status,
        tone,
        format!(
            "{} uses {} of allowed step ({})",
            label,
            format_percent(ratio),
            parameter_unit(unit)
        ),
    )
}

fn action_in_spec(action: &ControlAction, loop_definition: &ControlLoop) -> bool {
    !loop_definition
        .output
        .lower_spec
        .is_some_and(|lower| action.measured_value < lower)
        && !loop_definition
            .output
            .upper_spec
            .is_some_and(|upper| action.measured_value > upper)
}

fn spec_label(loop_definition: &ControlLoop) -> String {
    match (
        loop_definition.output.lower_spec,
        loop_definition.output.upper_spec,
    ) {
        (Some(lower), Some(upper)) => format!(
            "{}..{} {}",
            format_number(lower),
            format_number(upper),
            loop_definition.output.unit
        ),
        (Some(lower), None) => {
            format!(
                ">= {} {}",
                format_number(lower),
                loop_definition.output.unit
            )
        }
        (None, Some(upper)) => {
            format!(
                "<= {} {}",
                format_number(upper),
                loop_definition.output.unit
            )
        }
        (None, None) => "no spec limits".to_string(),
    }
}

fn parameter_unit(unit: Option<RecipeUnit>) -> &'static str {
    unit.map(RecipeUnit::symbol).unwrap_or("recipe units")
}

fn audit_event_row(ui: &mut egui::Ui, event: &ControlAuditEvent) {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
        ui.label(event.kind.label());
        if let Some(action_id) = &event.action_id {
            ui.small(short_action_id(action_id.as_str()));
        }
        if let Some(to_state) = event.to_state {
            ui.small(format!("-> {}", to_state.label()));
        }
        ui.small(&event.actor);
        ui.small(&event.timestamp);
    });
    if !event.note.is_empty() {
        ui.add(
            egui::Label::new(
                RichText::new(&event.note)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .wrap(),
        );
    }
}

fn draw_control_chart(
    ui: &mut egui::Ui,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
) {
    let (rect, response) =
        ui.allocate_exact_size(ui_chrome::stable_plot_size(ui, 220.0), Sense::hover());
    let painter = ui.painter_at(rect);
    ui_chrome::plot_background(ui, rect);

    if trend.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no control history",
            FontId::proportional(14.0),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    let mut min_value = trend
        .iter()
        .map(|point| {
            point
                .value
                .min(point.target - point.ewma_error)
                .min(point.target)
        })
        .fold(f64::INFINITY, f64::min);
    let mut max_value = trend
        .iter()
        .map(|point| {
            point
                .value
                .max(point.target - point.ewma_error)
                .max(point.target)
        })
        .fold(f64::NEG_INFINITY, f64::max);
    let deadband = loop_definition.deadband.abs();
    min_value = min_value.min(loop_definition.output.target - deadband);
    max_value = max_value.max(loop_definition.output.target + deadband);
    if let Some(lower) = loop_definition.output.lower_spec {
        min_value = min_value.min(lower);
    }
    if let Some(upper) = loop_definition.output.upper_spec {
        max_value = max_value.max(upper);
    }
    let padding = ((max_value - min_value) * 0.12).max(0.5);
    min_value -= padding;
    max_value += padding;

    let plot = rect.shrink2(vec2(34.0, 20.0));
    let x_at = |index: usize| -> f32 {
        if trend.len() <= 1 {
            plot.center().x
        } else {
            plot.left() + (index as f32 / (trend.len() - 1) as f32) * plot.width()
        }
    };
    let y_at = |value: f64| -> f32 {
        let fraction = ((value - min_value) / (max_value - min_value).max(f64::EPSILON)) as f32;
        plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height()
    };

    if let (Some(lower), Some(upper)) = (
        loop_definition.output.lower_spec,
        loop_definition.output.upper_spec,
    ) {
        let spec_rect = Rect::from_min_max(
            Pos2::new(plot.left(), y_at(upper)),
            Pos2::new(plot.right(), y_at(lower)),
        );
        painter.rect_filled(
            spec_rect,
            0.0,
            Color32::from_rgba_premultiplied(69, 148, 103, 28),
        );
    }

    if deadband > f64::EPSILON {
        let deadband_rect = Rect::from_min_max(
            Pos2::new(plot.left(), y_at(loop_definition.output.target + deadband)),
            Pos2::new(plot.right(), y_at(loop_definition.output.target - deadband)),
        );
        painter.rect_filled(
            deadband_rect,
            0.0,
            Color32::from_rgba_premultiplied(220, 176, 72, 24),
        );
    }

    draw_horizontal_line(
        &painter,
        plot,
        y_at(loop_definition.output.target),
        Color32::from_rgb(190, 196, 205),
    );
    if let Some(lower) = loop_definition.output.lower_spec {
        draw_horizontal_line(
            &painter,
            plot,
            y_at(lower),
            Color32::from_rgb(124, 132, 144),
        );
        painter.text(
            Pos2::new(plot.right(), y_at(lower) - 2.0),
            Align2::RIGHT_BOTTOM,
            "LSL",
            FontId::proportional(10.0),
            ui.visuals().weak_text_color(),
        );
    }
    if let Some(upper) = loop_definition.output.upper_spec {
        draw_horizontal_line(
            &painter,
            plot,
            y_at(upper),
            Color32::from_rgb(124, 132, 144),
        );
        painter.text(
            Pos2::new(plot.right(), y_at(upper) - 2.0),
            Align2::RIGHT_BOTTOM,
            "USL",
            FontId::proportional(10.0),
            ui.visuals().weak_text_color(),
        );
    }

    for pair in trend.windows(2).enumerate() {
        let (index, points) = pair;
        let left = Pos2::new(x_at(index), y_at(points[0].value));
        let right = Pos2::new(x_at(index + 1), y_at(points[1].value));
        painter.line_segment(
            [left, right],
            Stroke::new(2.0, Color32::from_rgb(82, 156, 219)),
        );
    }

    for pair in trend.windows(2).enumerate() {
        let (index, points) = pair;
        let left = Pos2::new(x_at(index), y_at(points[0].target - points[0].ewma_error));
        let right = Pos2::new(
            x_at(index + 1),
            y_at(points[1].target - points[1].ewma_error),
        );
        painter.line_segment(
            [left, right],
            Stroke::new(1.5, Color32::from_rgb(220, 176, 72)),
        );
    }

    let mut hovered = None;
    for (index, point) in trend.iter().enumerate() {
        let pos = Pos2::new(x_at(index), y_at(point.value));
        let color = if point.in_spec {
            Color32::from_rgb(92, 176, 123)
        } else {
            Color32::from_rgb(216, 93, 88)
        };
        painter.circle_filled(pos, 4.0, color);
        if point.actionable(loop_definition) {
            painter.circle_stroke(pos, 7.0, Stroke::new(1.0, Color32::from_rgb(220, 176, 72)));
        }
        if response
            .hover_pos()
            .is_some_and(|pointer| pointer.distance(pos) <= 8.0)
        {
            hovered = Some(point);
        }
    }

    painter.text(
        Pos2::new(plot.left(), y_at(loop_definition.output.target) - 4.0),
        Align2::LEFT_BOTTOM,
        "target",
        FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
    painter.text(
        Pos2::new(plot.right(), plot.top()),
        Align2::RIGHT_TOP,
        "blue measured / amber EWMA",
        FontId::proportional(10.0),
        ui.visuals().weak_text_color(),
    );

    if let Some(point) = hovered {
        response.on_hover_text(format!(
            "{} {}\nvalue {}\ntarget {}\nerror {} {}\nEWMA error {} {}\nyield {}",
            point.source.lot_id,
            point.source.wafer_id,
            format_measurement(point.value, &point.unit),
            format_measurement(point.target, &point.unit),
            format_signed(point.error),
            point.unit,
            format_signed(point.ewma_error),
            point.unit,
            point
                .yield_fraction
                .map(format_percent)
                .unwrap_or_else(|| "-".to_string())
        ));
    }
}

fn control_history_table(
    ui: &mut egui::Ui,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
) {
    egui::ScrollArea::horizontal()
        .id_salt(ui.next_auto_id())
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(64.0)
                .show(ui, |ui| {
                    ui.strong("Run");
                    ui.strong("Source");
                    ui.strong("Value");
                    ui.strong("Error");
                    ui.strong("EWMA");
                    ui.strong("Yield");
                    ui.strong("Disposition");
                    ui.end_row();
                    for point in trend.iter().rev().take(8) {
                        ui.label(point.source.run_index.to_string());
                        ui.label(format!("{} {}", point.source.lot_id, point.source.wafer_id));
                        ui.colored_label(
                            if point.in_spec {
                                Color32::from_rgb(112, 190, 135)
                            } else {
                                Color32::from_rgb(220, 104, 96)
                            },
                            format_measurement(point.value, &point.unit),
                        );
                        ui.label(format!("{} {}", format_signed(point.error), point.unit));
                        ui.label(format!(
                            "{} {}",
                            format_signed(point.ewma_error),
                            point.unit
                        ));
                        ui.label(
                            point
                                .yield_fraction
                                .map(format_percent)
                                .unwrap_or_else(|| "-".to_string()),
                        );
                        ui.colored_label(
                            point_tone(point, loop_definition).color(),
                            if point.actionable(loop_definition) {
                                "recommend"
                            } else if point.in_spec {
                                "monitor"
                            } else {
                                "limit"
                            },
                        );
                        ui.end_row();
                    }
                });
        });
}

fn adjustment_table(ui: &mut egui::Ui, adjustments: &[RecipeParameterAdjustment]) {
    if adjustments.is_empty() {
        ui.label("No recipe parameter changes");
        return;
    }
    egui::ScrollArea::horizontal()
        .id_salt(ui.next_auto_id())
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(70.0)
                .show(ui, |ui| {
                    ui.strong("Parameter");
                    ui.strong("Current");
                    ui.strong("Proposed");
                    ui.strong("Delta");
                    ui.strong("Bounds");
                    ui.strong("Guardrail");
                    ui.end_row();
                    for adjustment in adjustments {
                        ui.label(&adjustment.label);
                        ui.label(format_parameter_value(
                            &adjustment.previous_value,
                            adjustment.unit,
                        ));
                        ui.label(
                            RichText::new(format_parameter_value(
                                &adjustment.proposed_value,
                                adjustment.unit,
                            ))
                            .strong(),
                        );
                        ui.label(format!(
                            "{} {}",
                            format_signed(adjustment.delta),
                            adjustment.unit.map(RecipeUnit::symbol).unwrap_or("")
                        ));
                        ui.label(format!(
                            "{}..{} {}",
                            format_number(adjustment.lower_bound),
                            format_number(adjustment.upper_bound),
                            parameter_unit(adjustment.unit)
                        ));
                        let inside_bounds = match adjustment.proposed_value.as_f64() {
                            Some(value) => {
                                value >= adjustment.lower_bound && value <= adjustment.upper_bound
                            }
                            None => true,
                        };
                        ui.colored_label(
                            if inside_bounds {
                                ui_chrome::Tone::Success.color()
                            } else {
                                ui_chrome::Tone::Warning.color()
                            },
                            if inside_bounds { "inside" } else { "clamped" },
                        );
                        ui.end_row();
                    }
                });
        });
}

fn draw_horizontal_line(painter: &egui::Painter, plot: Rect, y: f32, color: Color32) {
    painter.line_segment(
        [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
        Stroke::new(1.0, color),
    );
}

fn yield_context_detail(analysis: &YieldAnalysis, point: &ControlTrendPoint) -> String {
    analysis
        .wafer_summary(&point.source.lot_id, &point.source.wafer_id)
        .map(|summary| {
            format!(
                "{} failing dies, {}",
                summary.failing_dies,
                summary.spatial_pattern.label()
            )
        })
        .unwrap_or_else(|| "no wafer summary".to_string())
}

fn action_state_color(state: ControlActionState) -> Color32 {
    match state {
        ControlActionState::Proposed => Color32::from_rgb(98, 168, 222),
        ControlActionState::Approved => Color32::from_rgb(96, 190, 126),
        ControlActionState::Rejected => Color32::from_rgb(216, 96, 92),
        ControlActionState::Applied => Color32::from_rgb(132, 204, 156),
        ControlActionState::Held => Color32::from_rgb(214, 160, 72),
    }
}

fn audit_color(kind: ControlAuditKind) -> Color32 {
    match kind {
        ControlAuditKind::Proposed => Color32::from_rgb(98, 168, 222),
        ControlAuditKind::Approved => Color32::from_rgb(96, 190, 126),
        ControlAuditKind::Rejected => Color32::from_rgb(216, 96, 92),
        ControlAuditKind::Applied => Color32::from_rgb(132, 204, 156),
        ControlAuditKind::Held => Color32::from_rgb(214, 160, 72),
    }
}

fn format_measurement(value: f64, unit: &str) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0} {unit}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.2} {unit}")
    }
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 0.005 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn format_signed(value: f64) -> String {
    if value.abs() >= 10.0 {
        format!("{value:+.1}")
    } else {
        format!("{value:+.2}")
    }
}
