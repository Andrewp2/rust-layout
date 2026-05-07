use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, vec2};
use layout_model::{
    process_control::{
        ControlAction, ControlActionId, ControlActionState, ControlAuditKind, ControlLoop,
        ControlLoopId, ControlTrendPoint, ProcessControlModel, RecipeParameterAdjustment,
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
    pub(crate) fn new(analysis: &YieldAnalysis) -> Self {
        let model = ProcessControlModel::from_yield_analysis(analysis);
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

        ui.separator();
        ui_chrome::section_label(ui, &loop_definition.name);
        ui.label(format!("Route: {}", loop_definition.route_id));
        ui.label(format!("Step: {}", loop_definition.process_step_id));
        ui.label(format!("Tool: {}", loop_definition.tool_id));
        ui.label(format!("Recipe: {}", loop_definition.recipe));

        ui.separator();
        ui_chrome::section_label(ui, "Measured Output");
        ui.label(format!(
            "{} target {}",
            loop_definition.output.label,
            format_measurement(loop_definition.output.target, &loop_definition.output.unit)
        ));
        ui.label(format!(
            "Metrology: {} / {}",
            loop_definition.output.measurement_step_id, loop_definition.output.measurement_name
        ));

        ui.separator();
        ui_chrome::section_label(ui, "Recipe Adjusters");
        for parameter in &loop_definition.manipulated_parameters {
            ui.label(format!(
                "{} ({})",
                parameter.label,
                parameter
                    .unit
                    .map(RecipeUnit::symbol)
                    .unwrap_or("recipe units")
            ));
        }

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
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(audit_color(event.kind), format!("#{}", event.sequence));
                        ui.label(event.kind.label());
                        ui.small(&event.actor);
                        if !event.note.is_empty() {
                            ui.small(&event.note);
                        }
                    });
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
                let latest = trend.last();

                if let Some(latest) = latest {
                    ui.horizontal_wrapped(|ui| {
                        control_metric_ui(
                            ui,
                            "Latest output",
                            format_measurement(latest.value, &latest.unit),
                            format!(
                                "target {}, EWMA {} {}",
                                format_measurement(latest.target, &latest.unit),
                                format_signed(latest.ewma_error),
                                latest.unit
                            ),
                        );
                        control_metric_ui(
                            ui,
                            "Recipe path",
                            latest.recipe.to_string(),
                            format!(
                                "{} / {} / {}",
                                latest.source.lot_id,
                                latest.source.wafer_id,
                                latest.source.tool_run_id
                            ),
                        );
                        control_metric_ui(
                            ui,
                            "Yield context",
                            latest
                                .yield_fraction
                                .map(format_percent)
                                .unwrap_or_else(|| "-".to_string()),
                            yield_context_detail(analysis, latest),
                        );
                    });
                }

                ui.separator();
                ui.columns(2, |columns| {
                    ui_chrome::section_label(&mut columns[0], "Target vs measured");
                    draw_control_chart(&mut columns[0], &loop_definition, &trend);
                    columns[0].separator();
                    control_history_table(&mut columns[0], &trend);

                    ui_chrome::section_label(&mut columns[1], "Control actions");
                    requested_transition =
                        self.control_actions_ui(&mut columns[1], &loop_definition, &actions);
                });
            });

        if let Some((action_id, transition)) = requested_transition {
            self.apply_requested_transition(action_id, transition, status);
        }
    }

    fn control_actions_ui(
        &mut self,
        ui: &mut egui::Ui,
        loop_definition: &ControlLoop,
        actions: &[ControlAction],
    ) -> Option<(ControlActionId, RequestedTransition)> {
        if actions.is_empty() {
            ui_chrome::empty_state(ui, "No recommended adjustments for the selected loop");
            return None;
        }

        let mut requested_transition = None;
        for action in actions {
            let selected = self.selected_action.as_ref() == Some(&action.id);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        selected,
                        format!(
                            "{} {}",
                            action.source.wafer_id,
                            action.state.label().to_uppercase()
                        ),
                    )
                    .clicked()
                {
                    self.selected_action = Some(action.id.clone());
                }
                ui.colored_label(action_state_color(action.state), action.id.as_str());
            });
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
        ui.label(RichText::new(&action.id.0).strong());
        ui.label(format!(
            "{} from {} {}",
            loop_definition.output.label, action.source.lot_id, action.source.wafer_id
        ));
        ui.label(format!(
            "Measured {}, target {}, error {} {}",
            format_measurement(action.measured_value, &loop_definition.output.unit),
            format_measurement(action.target_value, &loop_definition.output.unit),
            format_signed(action.error),
            loop_definition.output.unit
        ));
        ui.label(format!("Confidence {}", format_percent(action.confidence)));
        ui.label(&action.rationale);

        ui.separator();
        adjustment_table(ui, &action.adjustments);

        ui.separator();
        ui.horizontal_wrapped(|ui| match action.state {
            ControlActionState::Proposed => {
                if ui.button("Approve").clicked() {
                    requested_transition = Some((action.id.clone(), RequestedTransition::Approve));
                }
                if ui.button("Reject").clicked() {
                    requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
                }
            }
            ControlActionState::Approved => {
                if ui.button("Apply").clicked() {
                    requested_transition = Some((action.id.clone(), RequestedTransition::Apply));
                }
                if ui.button("Reject").clicked() {
                    requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
                }
            }
            ControlActionState::Held => {
                ui.colored_label(Color32::from_rgb(212, 156, 64), "Held for review");
                if ui.button("Reject").clicked() {
                    requested_transition = Some((action.id.clone(), RequestedTransition::Reject));
                }
            }
            ControlActionState::Rejected | ControlActionState::Applied => {
                ui.label("Final state");
            }
        });

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

fn control_metric_ui(ui: &mut egui::Ui, label: &str, value: String, detail: String) {
    ui_chrome::metric_tile(ui, label, value, &detail);
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
        .map(|point| point.value.min(point.target))
        .fold(f64::INFINITY, f64::min);
    let mut max_value = trend
        .iter()
        .map(|point| point.value.max(point.target))
        .fold(f64::NEG_INFINITY, f64::max);
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
    }
    if let Some(upper) = loop_definition.output.upper_spec {
        draw_horizontal_line(
            &painter,
            plot,
            y_at(upper),
            Color32::from_rgb(124, 132, 144),
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

    let mut hovered = None;
    for (index, point) in trend.iter().enumerate() {
        let pos = Pos2::new(x_at(index), y_at(point.value));
        let color = if point.in_spec {
            Color32::from_rgb(92, 176, 123)
        } else {
            Color32::from_rgb(216, 93, 88)
        };
        painter.circle_filled(pos, 4.0, color);
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

    if let Some(point) = hovered {
        response.on_hover_text(format!(
            "{} {}\nvalue {}\nEWMA error {} {}",
            point.source.lot_id,
            point.source.wafer_id,
            format_measurement(point.value, &point.unit),
            format_signed(point.ewma_error),
            point.unit
        ));
    }
}

fn control_history_table(ui: &mut egui::Ui, trend: &[ControlTrendPoint]) {
    egui::Grid::new("process_control_history_grid")
        .striped(true)
        .min_col_width(64.0)
        .show(ui, |ui| {
            ui.strong("Run");
            ui.strong("Wafer");
            ui.strong("Value");
            ui.strong("EWMA");
            ui.strong("Yield");
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
                ui.end_row();
            }
        });
}

fn adjustment_table(ui: &mut egui::Ui, adjustments: &[RecipeParameterAdjustment]) {
    if adjustments.is_empty() {
        ui.label("No recipe parameter changes");
        return;
    }
    egui::Grid::new("process_control_adjustments_grid")
        .striped(true)
        .min_col_width(70.0)
        .show(ui, |ui| {
            ui.strong("Parameter");
            ui.strong("Current");
            ui.strong("Proposed");
            ui.strong("Delta");
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
                ui.end_row();
            }
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
