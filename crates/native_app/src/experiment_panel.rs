use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Color32, RichText};
use layout_model::experiment::{
    ExperimentAnalysisSummary, ExperimentFactor, ExperimentPlan, ExperimentRun, ExperimentRunId,
    ExperimentRunStatus, FactorEffect, FactorId, ResponseCaptureError, ResponseSpec,
    ResponseSpecId, ResponseStats, ResponseValue, ResponseValueStatus, format_compact_number,
};
use layout_model::{
    mes::{ProcessRouteId, ProcessStepId},
    recipe::RecipeBinding,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct ExperimentPlannerPanel {
    plan: ExperimentPlan,
    selected_run: Option<ExperimentRunId>,
    selected_response: ResponseSpecId,
    capture_value: f64,
    show_missing_only: bool,
    run_filter: RunMatrixFilter,
    lot_filter: Option<String>,
}

impl ExperimentPlannerPanel {
    pub(crate) fn empty() -> Self {
        Self::from_plan(blank_experiment_plan())
    }

    pub(crate) fn from_plan(plan: ExperimentPlan) -> Self {
        let selected_run = plan.runs.first().map(|run| run.id.clone());
        let selected_response = plan
            .responses
            .first()
            .map(|response| response.id.clone())
            .unwrap_or_default();
        Self {
            plan,
            selected_run,
            selected_response,
            capture_value: 72.0,
            show_missing_only: false,
            run_filter: RunMatrixFilter::All,
            lot_filter: None,
        }
    }

    pub(crate) fn plan(&self) -> &ExperimentPlan {
        &self.plan
    }

    pub(crate) fn dashboard_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let primary_response_id = self.selected_response_if_valid();
        let analysis = self.plan.analysis_summary(primary_response_id.as_ref());

        egui::ScrollArea::vertical()
            .id_salt("experiment_planner_dashboard")
            .show(ui, |ui| {
                let detail = format!("Owner: {}", self.plan.owner);
                ui_chrome::module_header(ui, "Process engineering", "DOE Planner", &detail, |ui| {
                    ui.colored_label(
                        experiment_status_color(self.plan.status),
                        self.plan.status.label(),
                    );
                });
                if self.is_blank_plan() {
                    ui_chrome::empty_state(ui, "No experiment plan loaded");
                    let _ = status;
                    return;
                }
                ui_chrome::muted(ui, &self.plan.objective);

                self.summary_metrics_ui(ui, &analysis);

                ui.separator();
                if ui.available_width() < 820.0 {
                    self.factor_overview_ui(ui);
                    ui.separator();
                    self.response_overview_ui(ui, &analysis);
                } else {
                    ui.columns(2, |columns| {
                        self.factor_overview_ui(&mut columns[0]);
                        self.response_overview_ui(&mut columns[1], &analysis);
                    });
                }

                ui.separator();
                self.split_lot_ui(ui);

                ui.separator();
                self.run_workbench_ui(ui);

                ui.separator();
                self.analysis_readiness_ui(ui, &analysis);

                ui.separator();
                self.analysis_ui(ui, &analysis);

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Queue next pending response").clicked() {
                        self.queue_next_pending_response(status);
                    }
                    if ui.button("Capture next demo response").clicked() {
                        self.capture_next_demo_response(status);
                    }
                    ui.checkbox(&mut self.show_missing_only, "Detail pending only");
                });
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "DOE Planner");
        if self.is_blank_plan() {
            ui_chrome::empty_state(ui, "No experiment plan loaded");
            let _ = status;
            return;
        }
        ui.label(&self.plan.title);
        ui.small(format!("{}  {}", self.plan.id, self.plan.status.label()));
        ui.separator();
        ui_chrome::section_label(ui, "FabOS Links");
        ui.label(format!("Route: {}", self.plan.route_id));
        ui.label(format!("Step: {}", self.plan.step_id));
        ui.label(format!("Baseline: {}", self.plan.baseline_recipe));

        ui.separator();
        ui_chrome::section_label(ui, "Readiness");
        let primary_response_id = self.selected_response_if_valid();
        let analysis = self.plan.analysis_summary(primary_response_id.as_ref());
        let progress = analysis_progress(&analysis, self.plan.responses.len());
        ui.add(
            egui::ProgressBar::new(progress)
                .text(format!("{:.0}% captured", progress * 100.0))
                .desired_width(ui.available_width()),
        );
        ui.small(readiness_label(&analysis, self.plan.responses.len()));

        ui.separator();
        ui_chrome::section_label(ui, "Split Lots");
        for split in self.split_lot_summaries() {
            ui.group(|ui| {
                ui.strong(&split.lot_id);
                ui.small(&split.block);
                ui.label(format!(
                    "{} wafers, {} runs, {} pending values",
                    split.wafer_ids.len(),
                    split.run_count,
                    split.pending_values
                ));
            });
        }

        ui.separator();
        ui_chrome::section_label(ui, "Factors");
        for factor in &self.plan.factors {
            ui.group(|ui| {
                ui.strong(&factor.name);
                ui.small(factor.source.label());
                ui.horizontal_wrapped(|ui| {
                    for level in &factor.levels {
                        ui.label(format!(
                            "{}: {}",
                            level.label,
                            level.value_label(factor.unit.as_deref())
                        ));
                    }
                });
            });
        }

        ui.separator();
        ui_chrome::section_label(ui, "Responses");
        for response in &self.plan.responses {
            ui.group(|ui| {
                ui.strong(&response.name);
                ui.small(format!(
                    "{} / {} / {}",
                    response.goal.label(),
                    response.unit,
                    response.source.label()
                ));
                ui.small(response_spec_label(response));
            });
        }

        ui.separator();
        for note in &self.plan.notes {
            ui.small(note);
        }
    }

    pub(crate) fn response_capture_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Response Capture");
        if self.is_blank_plan() {
            ui_chrome::empty_state(ui, "No experiment plan loaded");
            let _ = status;
            return;
        }

        if ui.available_width() < 680.0 {
            self.capture_form_ui(ui, status);
            ui.separator();
            self.selected_run_detail_ui(ui);
        } else {
            ui.columns(2, |columns| {
                self.capture_form_ui(&mut columns[0], status);
                self.selected_run_detail_ui(&mut columns[1]);
            });
        }
    }

    fn capture_form_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.run_filter_bar_ui(ui, "doe_capture_filter_bar");

        let run_label = self.selected_run_label();
        egui::ComboBox::from_id_salt("doe_capture_run")
            .selected_text(run_label)
            .show_ui(ui, |ui| {
                for run in &self.plan.runs {
                    ui.selectable_value(
                        &mut self.selected_run,
                        Some(run.id.clone()),
                        run.id.to_string(),
                    );
                }
            });

        self.response_selector_ui(ui, "doe_capture_response", "Response");

        if let Some(response) = self.plan.response(&self.selected_response) {
            response_capture_spec_ui(ui, response);
        }

        ui.horizontal(|ui| {
            ui.label("Value");
            let unit = self
                .plan
                .response(&self.selected_response)
                .map(|response| response.unit.as_str())
                .unwrap_or("");
            let mut value_editor = egui::DragValue::new(&mut self.capture_value)
                .speed(0.1)
                .range(-1_000.0..=10_000.0);
            if !unit.is_empty() {
                value_editor = value_editor.suffix(format!(" {unit}"));
            }
            ui.add(value_editor);
            if let Some(response) = self.plan.response(&self.selected_response) {
                let value_status = response.value_status(self.capture_value);
                ui.colored_label(response_status_color(value_status), value_status.label());
            }
        });

        ui.horizontal_wrapped(|ui| {
            if ui.button("Capture").clicked() {
                self.capture_selected_response(status);
            }
            if ui.button("Capture and advance").clicked() {
                self.capture_and_advance(status);
            }
            if ui.button("Next pending").clicked() {
                self.queue_next_pending_response(status);
            }
            if ui.button("Use demo value").clicked() {
                self.use_demo_capture_value(status);
            }
            if ui.button("Use target").clicked() {
                self.use_response_target(status);
            }
            if ui.button("Capture next demo").clicked() {
                self.capture_next_demo_response(status);
            }
        });
        ui.checkbox(&mut self.show_missing_only, "Show pending only");
    }

    fn summary_metrics_ui(&self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        let total_values = analysis.run_count.saturating_mul(self.plan.responses.len());
        let captured_values = total_values.saturating_sub(analysis.missing_response_count);
        let progress = analysis_progress(analysis, self.plan.responses.len());
        let split_count = self.split_lot_summaries().len();
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Runs",
                    format!(
                        "{} / {} complete",
                        analysis.completed_runs, analysis.run_count
                    ),
                    &format!("{} pending values", analysis.missing_response_count),
                    Tone::Info,
                ),
                (
                    "Responses",
                    format!("{captured_values} / {total_values} captured"),
                    &format!("{} response specs", self.plan.responses.len()),
                    readiness_tone(analysis),
                ),
                (
                    "Split lots",
                    format!("{split_count} split groups"),
                    &format!("{} factors in matrix", self.plan.factors.len()),
                    Tone::Info,
                ),
                (
                    "Readiness",
                    format!("{:.0}%", progress * 100.0),
                    &readiness_label(analysis, self.plan.responses.len()),
                    readiness_tone(analysis),
                ),
                (
                    "Best run",
                    analysis
                        .best_run_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "pending".to_string()),
                    &self.selected_response_label(),
                    Tone::Info,
                ),
                (
                    "Matrix",
                    format!("{} factors", self.plan.factors.len()),
                    &format!("{} responses", self.plan.responses.len()),
                    Tone::Info,
                ),
            ],
        );
    }

    fn factor_overview_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Factor Split");
        if ui.available_width() < 520.0 {
            for factor in &self.plan.factors {
                factor_card_ui(ui, factor, ui.available_width());
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                for factor in &self.plan.factors {
                    factor_card_ui(ui, factor, 260.0);
                }
            });
        }
    }

    fn response_overview_ui(&mut self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        ui_chrome::section_label(ui, "Response Plan");
        let mut clicked_response = None;
        if ui.available_width() < 520.0 {
            for response in &self.plan.responses {
                let stats = analysis
                    .response_stats
                    .iter()
                    .find(|stat| stat.response_id == response.id);
                if response_card_ui(
                    ui,
                    response,
                    stats,
                    ui.available_width(),
                    self.selected_response == response.id,
                ) {
                    clicked_response = Some(response.id.clone());
                }
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                for response in &self.plan.responses {
                    let stats = analysis
                        .response_stats
                        .iter()
                        .find(|stat| stat.response_id == response.id);
                    if response_card_ui(
                        ui,
                        response,
                        stats,
                        260.0,
                        self.selected_response == response.id,
                    ) {
                        clicked_response = Some(response.id.clone());
                    }
                }
            });
        }
        if let Some(response_id) = clicked_response {
            self.selected_response = response_id;
        }
    }

    fn split_lot_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Split Lots");
        let summaries = self.split_lot_summaries();
        if summaries.is_empty() {
            ui_chrome::empty_state(ui, "No split-lot assignments in this plan");
            return;
        }

        let mut selected_lot = None;
        if ui.available_width() < 520.0 {
            for split in &summaries {
                if split_lot_card_ui(
                    ui,
                    split,
                    ui.available_width(),
                    self.lot_filter.as_deref() == Some(split.lot_id.as_str()),
                ) {
                    selected_lot = Some(split.lot_id.clone());
                }
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                for split in &summaries {
                    if split_lot_card_ui(
                        ui,
                        split,
                        240.0,
                        self.lot_filter.as_deref() == Some(split.lot_id.as_str()),
                    ) {
                        selected_lot = Some(split.lot_id.clone());
                    }
                }
            });
        }
        if let Some(lot_id) = selected_lot {
            if self.lot_filter.as_deref() == Some(lot_id.as_str()) {
                self.lot_filter = None;
            } else {
                self.lot_filter = Some(lot_id);
            }
        }
    }

    fn run_workbench_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 900.0 {
            self.run_matrix_ui(ui);
            ui.separator();
            self.selected_run_detail_ui(ui);
        } else {
            ui.columns(2, |columns| {
                self.run_matrix_ui(&mut columns[0]);
                ui_chrome::section_label(&mut columns[1], "Selected Run");
                self.selected_run_detail_ui(&mut columns[1]);
            });
        }
    }

    fn run_matrix_ui(&mut self, ui: &mut egui::Ui) {
        self.run_filter_bar_ui(ui, "doe_matrix_filter_bar");
        let rows = self.filtered_run_matrix_rows();
        let factor_names = self
            .plan
            .factors
            .iter()
            .map(|factor| factor.name.clone())
            .collect::<Vec<_>>();

        ui_chrome::section_label(
            ui,
            &format!("Run Matrix ({} / {})", rows.len(), self.plan.runs.len()),
        );
        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No runs match the active DOE filters");
            return;
        }
        if ui.available_width() < 520.0 {
            for row in rows {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(row.selected, &row.run_id)
                            .on_hover_text("Select run for response capture")
                            .clicked()
                        {
                            self.selected_run = Some(ExperimentRunId::new(row.run_id.clone()));
                        }
                        ui.colored_label(row.status_color, &row.status);
                    });
                    ui.small(format!(
                        "Order {}  {}  slot {}",
                        row.run_order, row.wafer_id, row.slot
                    ));
                    ui.small(format!("Split {}", row.block));
                    for (factor_name, factor_value) in factor_names.iter().zip(&row.factor_values) {
                        ui.add(egui::Label::new(format!("{factor_name}: {factor_value}")).wrap());
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Captured {}", row.capture_summary));
                        ui.colored_label(row.primary_color, &row.primary_value);
                    });
                });
            }
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("doe_run_matrix_x")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("doe_run_matrix")
                    .striped(true)
                    .min_col_width(68.0)
                    .show(ui, |ui| {
                        ui.strong("Run");
                        ui.strong("Order");
                        ui.strong("Lot");
                        ui.strong("Wafer");
                        ui.strong("Split");
                        for factor_name in &factor_names {
                            ui.strong(factor_name);
                        }
                        ui.strong("Status");
                        ui.strong("Captured");
                        ui.strong("Primary");
                        ui.end_row();

                        for row in rows {
                            if ui
                                .selectable_label(row.selected, &row.run_id)
                                .on_hover_text("Select run for response capture")
                                .clicked()
                            {
                                self.selected_run = Some(ExperimentRunId::new(row.run_id.clone()));
                            }
                            ui.label(row.run_order.to_string());
                            ui.label(row.lot_id);
                            ui.label(format!("{} W{:02}", row.wafer_id, row.slot));
                            ui.label(row.block);
                            for factor_value in row.factor_values {
                                ui.label(factor_value);
                            }
                            ui.colored_label(row.status_color, row.status);
                            ui.label(row.capture_summary);
                            ui.colored_label(row.primary_color, row.primary_value);
                            ui.end_row();
                        }
                    });
            });
    }

    fn run_filter_bar_ui(&mut self, ui: &mut egui::Ui, id_salt: &str) {
        ui.push_id(id_salt, |ui| {
            ui.horizontal_wrapped(|ui| {
                self.response_selector_ui(ui, "primary_response_filter", "Primary");

                ui.label("Run filter");
                egui::ComboBox::from_id_salt("run_filter")
                    .selected_text(self.run_filter.label())
                    .show_ui(ui, |ui| {
                        for filter in RunMatrixFilter::ALL {
                            ui.selectable_value(&mut self.run_filter, filter, filter.label());
                        }
                    });

                ui.label("Lot");
                let lot_label = self.lot_filter.as_deref().unwrap_or("All lots");
                egui::ComboBox::from_id_salt("lot_filter")
                    .selected_text(lot_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.lot_filter, None, "All lots");
                        for lot_id in self.lot_options() {
                            ui.selectable_value(&mut self.lot_filter, Some(lot_id.clone()), lot_id);
                        }
                    });

                if ui.button("Clear filters").clicked() {
                    self.run_filter = RunMatrixFilter::All;
                    self.lot_filter = None;
                }
            });
        });
    }

    fn response_selector_ui(&mut self, ui: &mut egui::Ui, id_salt: &str, label: &str) {
        ui.label(label);
        let response_label = self
            .plan
            .response(&self.selected_response)
            .map(|response| response.name.clone())
            .unwrap_or_else(|| "none".to_string());
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(response_label)
            .show_ui(ui, |ui| {
                for response in &self.plan.responses {
                    ui.selectable_value(
                        &mut self.selected_response,
                        response.id.clone(),
                        response.name.clone(),
                    );
                }
            });
    }

    fn analysis_readiness_ui(&self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        ui_chrome::section_label(ui, "Analysis Readiness");
        let progress = analysis_progress(analysis, self.plan.responses.len());
        ui.add(
            egui::ProgressBar::new(progress)
                .text(format!("{:.0}% response matrix captured", progress * 100.0))
                .desired_width(ui.available_width()),
        );
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &readiness_label(analysis, self.plan.responses.len()),
                readiness_tone(analysis),
            );
            ui.label(format!(
                "{} complete runs, {} pending values",
                analysis.completed_runs, analysis.missing_response_count
            ));
            if let Some(primary_response_id) = &analysis.primary_response_id {
                ui.label(format!("Primary response: {primary_response_id}"));
            }
        });

        let primary_missing = self
            .selected_response_if_valid()
            .as_ref()
            .map(|response_id| {
                self.plan
                    .runs
                    .iter()
                    .filter(|run| !run.responses.contains_key(response_id))
                    .count()
            })
            .unwrap_or(0);
        let effect_count = analysis.factor_effects.len();
        ui.small(format!(
            "{primary_missing} runs still need the selected primary response; {effect_count} level effects have observed data."
        ));
    }

    fn analysis_ui(&self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        if ui.available_width() < 760.0 {
            ui_chrome::section_label(ui, "Response Summary");
            egui::ScrollArea::horizontal()
                .id_salt("doe_response_stats_x")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    response_stats_ui(ui, &self.plan, &analysis.response_stats)
                });
            ui.separator();
            ui_chrome::section_label(
                ui,
                &format!("Main Effects: {}", self.selected_response_label()),
            );
            egui::ScrollArea::horizontal()
                .id_salt("doe_factor_effects_x")
                .auto_shrink([false, false])
                .show(ui, |ui| factor_effects_ui(ui, &analysis.factor_effects));
        } else {
            ui.columns(2, |columns| {
                ui_chrome::section_label(&mut columns[0], "Response Summary");
                response_stats_ui(&mut columns[0], &self.plan, &analysis.response_stats);

                ui_chrome::section_label(
                    &mut columns[1],
                    &format!("Main Effects: {}", self.selected_response_label()),
                );
                factor_effects_ui(&mut columns[1], &analysis.factor_effects);
            });
        }

        if !analysis.notes.is_empty() {
            ui.separator();
            for note in &analysis.notes {
                ui.small(note);
            }
        }
    }

    fn selected_run_detail_ui(&self, ui: &mut egui::Ui) {
        let Some(run) = self
            .selected_run
            .as_ref()
            .and_then(|run_id| self.plan.run(run_id))
        else {
            ui_chrome::empty_state(ui, "No run selected");
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("Run {}", run.id)).strong());
            ui.colored_label(status_color(run.status), run.status.label());
        });
        ui.label(format!(
            "{} / {} slot {} / order {}",
            run.assignment.lot_id,
            run.assignment.wafer_id,
            run.assignment.slot,
            run.assignment.run_order
        ));
        ui.small(format!(
            "Split: {}",
            run.assignment.block.as_deref().unwrap_or("unblocked")
        ));
        if let Some(recipe) = &run.recipe {
            ui.small(format!("Recipe: {recipe}"));
        }
        if let Some(tool_id) = &run.tool_id {
            ui.small(format!("Tool: {tool_id}"));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Factor Levels");
        for factor in &self.plan.factors {
            ui.label(format!(
                "{}: {}",
                factor.name,
                self.plan.run_factor_label(run, factor)
            ));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Responses");
        for response in &self.plan.responses {
            let captured = run.responses.get(&response.id);
            if self.show_missing_only && captured.is_some() {
                continue;
            }
            match captured {
                Some(value) => {
                    let value_status = response.value_status(value.value);
                    let color = response_status_color(value_status);
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(color, format_response_value(value.value, response));
                            ui.label(&response.name);
                            ui.small(value_status.label());
                        });
                        ui.small(format!("Captured {}", value.captured_at));
                        if let Some(measurement_id) = &value.measurement_id {
                            ui.small(format!("Measurement {measurement_id}"));
                        }
                    });
                }
                None => {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(Color32::YELLOW, "pending");
                        ui.label(&response.name);
                    });
                }
            }
        }
    }

    fn capture_selected_response(&mut self, status: &mut String) -> bool {
        let Some(run_id) = self.selected_run.clone() else {
            *status = "DOE: select a run before capturing a response".to_string();
            return false;
        };
        let response_id = self.selected_response.clone();
        let value = ResponseValue {
            value: self.capture_value,
            measurement_id: Some(format!("DOE-{run_id}-{response_id}")),
            captured_at: "2026-05-06T16:00:00Z".to_string(),
        };
        match self.plan.capture_response(&run_id, &response_id, value) {
            Ok(()) => {
                *status = format!(
                    "DOE: captured {} for {}",
                    self.selected_response_label(),
                    run_id
                );
                true
            }
            Err(err) => {
                *status = format!("DOE capture blocked: {}", response_error_label(&err));
                false
            }
        }
    }

    fn capture_and_advance(&mut self, status: &mut String) {
        if !self.capture_selected_response(status) {
            return;
        }
        if self.select_next_pending_response() {
            let captured_status = status.clone();
            *status = format!("{captured_status}; queued next pending response");
        }
    }

    fn capture_next_demo_response(&mut self, status: &mut String) {
        let runs = self.plan.runs.clone();
        let responses = self.plan.responses.clone();
        for run in runs {
            for response in &responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                let Some(value) = demo_response_value(&run, &response.id) else {
                    continue;
                };
                let run_id = run.id.clone();
                let response_id = response.id.clone();
                let capture = ResponseValue {
                    value,
                    measurement_id: Some(format!("SIM-{run_id}-{response_id}")),
                    captured_at: "2026-05-06T16:30:00Z".to_string(),
                };
                match self.plan.capture_response(&run_id, &response_id, capture) {
                    Ok(()) => {
                        self.selected_run = Some(run_id.clone());
                        self.selected_response = response_id.clone();
                        self.capture_value = value;
                        *status = format!("DOE: captured demo {response_id} for {run_id}");
                    }
                    Err(err) => {
                        *status = format!("DOE capture blocked: {}", response_error_label(&err));
                    }
                }
                return;
            }
        }
        *status = "DOE: no pending demo responses".to_string();
    }

    fn queue_next_pending_response(&mut self, status: &mut String) {
        if self.select_next_pending_response() {
            let run = self
                .selected_run
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string());
            *status = format!("DOE: queued {} for {run}", self.selected_response_label());
        } else {
            *status = "DOE: no pending responses remain".to_string();
        }
    }

    fn use_demo_capture_value(&mut self, status: &mut String) {
        let Some(run) = self
            .selected_run
            .as_ref()
            .and_then(|run_id| self.plan.run(run_id))
        else {
            *status = "DOE: select a run before loading a demo value".to_string();
            return;
        };
        let Some(value) = demo_response_value(run, &self.selected_response) else {
            *status = "DOE: no demo value available for this response".to_string();
            return;
        };
        self.capture_value = value;
        *status = format!("DOE: loaded demo value {}", format_compact_number(value));
    }

    fn use_response_target(&mut self, status: &mut String) {
        let Some(response) = self.plan.response(&self.selected_response) else {
            *status = "DOE: select a response before using a target".to_string();
            return;
        };
        let Some(target) = response.target else {
            *status = format!("DOE: {} has no target value", response.name);
            return;
        };
        self.capture_value = target;
        *status = format!("DOE: loaded target for {}", response.name);
    }

    fn select_next_pending_response(&mut self) -> bool {
        if self.plan.runs.is_empty() {
            return false;
        }
        let run_count = self.plan.runs.len();
        let selected_run_index = self
            .selected_run
            .as_ref()
            .and_then(|run_id| self.plan.runs.iter().position(|run| &run.id == run_id))
            .unwrap_or(0);

        for offset in 0..run_count {
            let index = (selected_run_index + offset) % run_count;
            let run = &self.plan.runs[index];
            for response in &self.plan.responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                self.selected_run = Some(run.id.clone());
                self.selected_response = response.id.clone();
                self.capture_value = response
                    .target
                    .or_else(|| demo_response_value(run, &response.id))
                    .unwrap_or_default();
                return true;
            }
        }
        false
    }

    fn filtered_run_matrix_rows(&self) -> Vec<RunMatrixRow> {
        self.run_matrix_rows()
            .into_iter()
            .filter(|row| {
                self.lot_filter
                    .as_deref()
                    .is_none_or(|lot_id| row.lot_id == lot_id)
            })
            .filter(|row| self.run_filter.matches(row))
            .collect()
    }

    fn run_matrix_rows(&self) -> Vec<RunMatrixRow> {
        let primary_response_id = self.selected_response_if_valid();
        self.plan
            .runs
            .iter()
            .map(|run| {
                let (primary_value, primary_color) = primary_response_id
                    .as_ref()
                    .and_then(|response_id| {
                        let response = self.plan.response(response_id)?;
                        let value = run.responses.get(response_id)?;
                        Some((
                            format_response_value(value.value, response),
                            response_status_color(response.value_status(value.value)),
                        ))
                    })
                    .unwrap_or_else(|| ("pending".to_string(), Color32::YELLOW));
                let missing_count = self
                    .plan
                    .responses
                    .len()
                    .saturating_sub(run.responses.len());
                let has_out_of_spec = run.responses.iter().any(|(response_id, value)| {
                    self.plan.response(response_id).is_some_and(|response| {
                        response.value_status(value.value) != ResponseValueStatus::InSpec
                    })
                });
                let needs_selected_response = primary_response_id
                    .as_ref()
                    .is_some_and(|response_id| !run.responses.contains_key(response_id));
                RunMatrixRow {
                    run_id: run.id.to_string(),
                    selected: self.selected_run.as_ref() == Some(&run.id),
                    run_order: run.assignment.run_order,
                    lot_id: run.assignment.lot_id.to_string(),
                    wafer_id: run.assignment.wafer_id.to_string(),
                    slot: run.assignment.slot,
                    block: run
                        .assignment
                        .block
                        .clone()
                        .unwrap_or_else(|| "unblocked".to_string()),
                    factor_values: self
                        .plan
                        .factors
                        .iter()
                        .map(|factor| self.plan.run_factor_label(run, factor))
                        .collect(),
                    status: run.status.label().to_string(),
                    status_kind: run.status,
                    status_color: status_color(run.status),
                    capture_summary: format!(
                        "{} / {}",
                        run.responses.len(),
                        self.plan.responses.len()
                    ),
                    missing_count,
                    needs_selected_response,
                    has_out_of_spec,
                    primary_value,
                    primary_color,
                }
            })
            .collect()
    }

    fn ensure_selection(&mut self) {
        if self
            .selected_run
            .as_ref()
            .is_none_or(|run_id| self.plan.run(run_id).is_none())
        {
            self.selected_run = self.plan.runs.first().map(|run| run.id.clone());
        }
        if self.plan.response(&self.selected_response).is_none() {
            self.selected_response = self
                .plan
                .responses
                .first()
                .map(|response| response.id.clone())
                .unwrap_or_default();
        }
        if self.lot_filter.as_deref().is_some_and(|lot_id| {
            !self
                .plan
                .runs
                .iter()
                .any(|run| run.assignment.lot_id.to_string() == lot_id)
        }) {
            self.lot_filter = None;
        }
    }

    fn selected_response_if_valid(&self) -> Option<ResponseSpecId> {
        self.plan
            .response(&self.selected_response)
            .map(|response| response.id.clone())
    }

    fn selected_response_label(&self) -> String {
        self.plan
            .response(&self.selected_response)
            .map(|response| response.name.clone())
            .unwrap_or_else(|| "primary response".to_string())
    }

    fn is_blank_plan(&self) -> bool {
        self.plan.id.as_str().is_empty()
            && self.plan.factors.is_empty()
            && self.plan.responses.is_empty()
            && self.plan.runs.is_empty()
    }

    fn split_lot_summaries(&self) -> Vec<SplitLotSummary> {
        let mut summaries = BTreeMap::<String, SplitLotSummary>::new();
        for run in &self.plan.runs {
            let lot_id = run.assignment.lot_id.to_string();
            let block = run
                .assignment
                .block
                .clone()
                .unwrap_or_else(|| "unblocked".to_string());
            let key = format!("{lot_id}|{block}");
            let summary = summaries
                .entry(key)
                .or_insert_with(|| SplitLotSummary::new(lot_id, block));
            summary.run_count += 1;
            if run.status == ExperimentRunStatus::Complete {
                summary.completed_count += 1;
            }
            summary.pending_values += self
                .plan
                .responses
                .len()
                .saturating_sub(run.responses.len());
            summary
                .wafer_ids
                .insert(run.assignment.wafer_id.to_string());
            summary.min_order = summary.min_order.min(run.assignment.run_order);
            summary.max_order = summary.max_order.max(run.assignment.run_order);
            if run.responses.iter().any(|(response_id, value)| {
                self.plan.response(response_id).is_some_and(|response| {
                    response.value_status(value.value) != ResponseValueStatus::InSpec
                })
            }) {
                summary.out_of_spec_count += 1;
            }
        }
        summaries.into_values().collect()
    }

    fn lot_options(&self) -> Vec<String> {
        self.plan
            .runs
            .iter()
            .map(|run| run.assignment.lot_id.to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn selected_run_label(&self) -> String {
        let Some(run) = self
            .selected_run
            .as_ref()
            .and_then(|run_id| self.plan.run(run_id))
        else {
            return "none".to_string();
        };
        format!(
            "{}  {} W{:02}",
            run.id, run.assignment.lot_id, run.assignment.slot
        )
    }
}

fn blank_experiment_plan() -> ExperimentPlan {
    ExperimentPlan {
        id: Default::default(),
        title: "No experiment plan loaded".to_string(),
        objective: String::new(),
        owner: String::new(),
        status: layout_model::experiment::ExperimentStatus::Draft,
        route_id: ProcessRouteId::default(),
        step_id: ProcessStepId::default(),
        baseline_recipe: RecipeBinding::new("", 0),
        factors: Vec::new(),
        responses: Vec::new(),
        runs: Vec::new(),
        notes: Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunMatrixFilter {
    All,
    NeedsAnyResponse,
    NeedsSelectedResponse,
    InProgress,
    Complete,
    OutOfSpec,
    Blocked,
}

impl RunMatrixFilter {
    const ALL: [Self; 7] = [
        Self::All,
        Self::NeedsAnyResponse,
        Self::NeedsSelectedResponse,
        Self::InProgress,
        Self::Complete,
        Self::OutOfSpec,
        Self::Blocked,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All runs",
            Self::NeedsAnyResponse => "Needs any response",
            Self::NeedsSelectedResponse => "Needs selected response",
            Self::InProgress => "In progress",
            Self::Complete => "Complete",
            Self::OutOfSpec => "Out of spec",
            Self::Blocked => "Blocked",
        }
    }

    fn matches(self, row: &RunMatrixRow) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAnyResponse => row.missing_count > 0,
            Self::NeedsSelectedResponse => row.needs_selected_response,
            Self::InProgress => row.status_kind == ExperimentRunStatus::InProgress,
            Self::Complete => row.status_kind == ExperimentRunStatus::Complete,
            Self::OutOfSpec => row.has_out_of_spec,
            Self::Blocked => row.status_kind == ExperimentRunStatus::Blocked,
        }
    }
}

struct RunMatrixRow {
    run_id: String,
    selected: bool,
    run_order: u32,
    lot_id: String,
    wafer_id: String,
    slot: u8,
    block: String,
    factor_values: Vec<String>,
    status: String,
    status_kind: ExperimentRunStatus,
    status_color: Color32,
    capture_summary: String,
    missing_count: usize,
    needs_selected_response: bool,
    has_out_of_spec: bool,
    primary_value: String,
    primary_color: Color32,
}

#[derive(Clone, Debug)]
struct SplitLotSummary {
    lot_id: String,
    block: String,
    run_count: usize,
    completed_count: usize,
    pending_values: usize,
    out_of_spec_count: usize,
    wafer_ids: BTreeSet<String>,
    min_order: u32,
    max_order: u32,
}

impl SplitLotSummary {
    fn new(lot_id: String, block: String) -> Self {
        Self {
            lot_id,
            block,
            run_count: 0,
            completed_count: 0,
            pending_values: 0,
            out_of_spec_count: 0,
            wafer_ids: BTreeSet::new(),
            min_order: u32::MAX,
            max_order: 0,
        }
    }
}

fn factor_card_ui(ui: &mut egui::Ui, factor: &ExperimentFactor, width: f32) {
    ui.group(|ui| {
        ui.set_width(width.clamp(160.0, 300.0));
        ui.add(egui::Label::new(RichText::new(&factor.name).strong()).wrap());
        ui.add(
            egui::Label::new(
                RichText::new(factor.source.label())
                    .small()
                    .color(ui.visuals().weak_text_color()),
            )
            .wrap(),
        );
        ui.small(format!("{} levels", factor.levels.len()));
        for level in &factor.levels {
            ui.add(
                egui::Label::new(format!(
                    "{}: {}",
                    level.label,
                    level.value_label(factor.unit.as_deref())
                ))
                .wrap(),
            );
        }
    });
}

fn response_card_ui(
    ui: &mut egui::Ui,
    response: &ResponseSpec,
    stats: Option<&ResponseStats>,
    width: f32,
    selected: bool,
) -> bool {
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    let mut clicked = false;
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(width.clamp(180.0, 320.0));
            ui.add(egui::Label::new(RichText::new(&response.name).strong()).wrap());
            ui.small(format!(
                "{} / {}",
                response.goal.label(),
                response.source.label()
            ));
            ui.label(response_spec_label(response));
            if let Some(stats) = stats {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("N {}", stats.sample_count));
                    ui.label(format!("Missing {}", stats.missing_count));
                    ui.label(format!("Mean {}", format_optional_number(stats.mean)));
                });
            }
            clicked = ui.button("Use as primary").clicked();
        });
    clicked
}

fn split_lot_card_ui(
    ui: &mut egui::Ui,
    split: &SplitLotSummary,
    width: f32,
    selected: bool,
) -> bool {
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    let mut clicked = false;
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(width.clamp(180.0, 300.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&split.lot_id).strong());
                if split.out_of_spec_count > 0 {
                    ui.colored_label(
                        Color32::LIGHT_RED,
                        format!("{} OOS", split.out_of_spec_count),
                    );
                }
            });
            ui.small(&split.block);
            ui.label(format!(
                "{} wafers, {} runs",
                split.wafer_ids.len(),
                split.run_count
            ));
            ui.small(format!(
                "Order {}-{}  complete {} / {}",
                split.min_order, split.max_order, split.completed_count, split.run_count
            ));
            ui.small(format!("{} pending response values", split.pending_values));
            clicked = ui
                .button(if selected {
                    "Clear lot filter"
                } else {
                    "Filter lot"
                })
                .clicked();
        });
    clicked
}

fn response_capture_spec_ui(ui: &mut egui::Ui, response: &ResponseSpec) {
    ui.group(|ui| {
        ui.label(RichText::new(&response.name).strong());
        ui.small(format!("Goal: {}", response.goal.label()));
        ui.small(response_spec_label(response));
        ui.small(format!("Source: {}", response.source.label()));
    });
}

fn response_stats_ui(ui: &mut egui::Ui, plan: &ExperimentPlan, stats: &[ResponseStats]) {
    egui::Grid::new("doe_response_stats")
        .striped(true)
        .min_col_width(72.0)
        .show(ui, |ui| {
            ui.strong("Response");
            ui.strong("Mean");
            ui.strong("Std dev");
            ui.strong("Range");
            ui.strong("N");
            ui.strong("Spec");
            ui.end_row();
            for stat in stats {
                let response = plan.response(&stat.response_id);
                ui.label(
                    response
                        .map(|response| response.name.as_str())
                        .unwrap_or(stat.response_id.as_str()),
                );
                ui.label(format_optional_number(stat.mean));
                ui.label(format_optional_number(stat.std_dev));
                ui.label(match (stat.min, stat.max) {
                    (Some(min), Some(max)) => {
                        format!(
                            "{}..{}",
                            format_compact_number(min),
                            format_compact_number(max)
                        )
                    }
                    _ => "-".to_string(),
                });
                ui.label(format!("{} / {}", stat.sample_count, stat.missing_count));
                ui.label(
                    response
                        .map(response_spec_label)
                        .unwrap_or_else(|| "-".to_string()),
                );
                ui.end_row();
            }
        });
}

fn factor_effects_ui(ui: &mut egui::Ui, effects: &[FactorEffect]) {
    egui::Grid::new("doe_factor_effects")
        .striped(true)
        .min_col_width(66.0)
        .show(ui, |ui| {
            ui.strong("Factor");
            ui.strong("Level");
            ui.strong("Mean");
            ui.strong("Delta");
            ui.strong("N");
            ui.end_row();
            for effect in effects {
                ui.label(&effect.factor_name);
                ui.label(&effect.level_label);
                ui.label(format_compact_number(effect.mean_response));
                ui.colored_label(
                    delta_color(effect.delta_from_overall),
                    format_signed_number(effect.delta_from_overall),
                );
                ui.label(effect.sample_count.to_string());
                ui.end_row();
            }
        });
}

fn demo_response_value(run: &ExperimentRun, response_id: &ResponseSpecId) -> Option<f64> {
    let dose_level = run
        .factor_levels
        .get(&FactorId::new("dose"))
        .map(|level| level.as_str())
        .unwrap_or("dose_nominal");
    let focus_level = run
        .factor_levels
        .get(&FactorId::new("focus"))
        .map(|level| level.as_str())
        .unwrap_or("focus_minus");

    let dose_cd: f64 = match dose_level {
        "dose_low" => -2.2,
        "dose_high" => 2.4,
        _ => 0.0,
    };
    let focus_cd = match focus_level {
        "focus_plus" => -0.7,
        _ => 0.4,
    };
    let process_bonus = match (dose_level, focus_level) {
        ("dose_nominal", "focus_plus") => 0.03,
        ("dose_high", "focus_plus") => 0.01,
        ("dose_low", "focus_plus") => -0.02,
        _ => 0.0,
    };

    match response_id.as_str() {
        "poly_cd_nm" => Some(72.0 + dose_cd + focus_cd),
        "defect_count" => Some(match dose_level {
            "dose_low" => 8.0,
            "dose_high" if focus_level == "focus_minus" => 6.0,
            "dose_high" => 4.0,
            _ => 3.0,
        }),
        "yield_fraction" => Some((0.88 + process_bonus - dose_cd.abs() * 0.01).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn format_response_value(value: f64, response: &ResponseSpec) -> String {
    if response.unit == "%" && value.abs() <= 1.0 {
        format!("{:.1}%", value * 100.0)
    } else if response.unit.is_empty() {
        format_compact_number(value)
    } else {
        format!("{} {}", format_compact_number(value), response.unit)
    }
}

fn response_spec_label(response: &ResponseSpec) -> String {
    let target = response
        .target
        .map(|value| format_response_value(value, response))
        .unwrap_or_else(|| "no target".to_string());
    let spec = match (response.lower_spec, response.upper_spec) {
        (Some(lower), Some(upper)) => format!(
            "{}..{}",
            format_response_value(lower, response),
            format_response_value(upper, response)
        ),
        (Some(lower), None) => format!(">= {}", format_response_value(lower, response)),
        (None, Some(upper)) => format!("<= {}", format_response_value(upper, response)),
        (None, None) => "no spec".to_string(),
    };
    format!("Target {target}; spec {spec}")
}

fn analysis_progress(analysis: &ExperimentAnalysisSummary, response_count: usize) -> f32 {
    let total = analysis.run_count.saturating_mul(response_count);
    if total == 0 {
        return 0.0;
    }
    let captured = total.saturating_sub(analysis.missing_response_count);
    captured as f32 / total as f32
}

fn readiness_tone(analysis: &ExperimentAnalysisSummary) -> Tone {
    if analysis.run_count > 0 && analysis.missing_response_count == 0 {
        Tone::Success
    } else if analysis.completed_runs > 0 {
        Tone::Warning
    } else {
        Tone::Info
    }
}

fn readiness_label(analysis: &ExperimentAnalysisSummary, response_count: usize) -> String {
    if analysis.run_count == 0 {
        "Matrix not generated".to_string()
    } else if response_count == 0 {
        "No responses defined".to_string()
    } else if analysis.missing_response_count == 0 {
        "Ready for analysis".to_string()
    } else if analysis.completed_runs > 0 {
        "Partial analysis".to_string()
    } else {
        "Capture pending".to_string()
    }
}

fn format_optional_number(value: Option<f64>) -> String {
    value
        .map(format_compact_number)
        .unwrap_or_else(|| "-".to_string())
}

fn format_signed_number(value: f64) -> String {
    if value >= 0.0 {
        format!("+{}", format_compact_number(value))
    } else {
        format_compact_number(value)
    }
}

fn response_error_label(err: &ResponseCaptureError) -> String {
    err.to_string()
}

fn experiment_status_color(status: layout_model::experiment::ExperimentStatus) -> Color32 {
    match status {
        layout_model::experiment::ExperimentStatus::Draft => Color32::GRAY,
        layout_model::experiment::ExperimentStatus::MatrixGenerated => Color32::LIGHT_BLUE,
        layout_model::experiment::ExperimentStatus::Running => Color32::YELLOW,
        layout_model::experiment::ExperimentStatus::AnalysisReady => Color32::LIGHT_GREEN,
    }
}

fn status_color(status: ExperimentRunStatus) -> Color32 {
    match status {
        ExperimentRunStatus::Ready => Color32::LIGHT_BLUE,
        ExperimentRunStatus::InProgress => Color32::YELLOW,
        ExperimentRunStatus::Complete => Color32::LIGHT_GREEN,
        ExperimentRunStatus::Blocked => Color32::LIGHT_RED,
    }
}

fn response_status_color(status: ResponseValueStatus) -> Color32 {
    match status {
        ResponseValueStatus::InSpec => Color32::LIGHT_GREEN,
        ResponseValueStatus::BelowSpec | ResponseValueStatus::AboveSpec => Color32::LIGHT_RED,
    }
}

fn delta_color(value: f64) -> Color32 {
    if value.abs() < 0.01 {
        Color32::GRAY
    } else if value > 0.0 {
        Color32::LIGHT_BLUE
    } else {
        Color32::LIGHT_RED
    }
}
