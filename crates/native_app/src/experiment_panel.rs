use eframe::egui::{self, Color32};
use layout_model::experiment::{
    ExperimentAnalysisSummary, ExperimentPlan, ExperimentRun, ExperimentRunId, ExperimentRunStatus,
    FactorEffect, FactorId, ResponseCaptureError, ResponseSpec, ResponseSpecId, ResponseStats,
    ResponseValue, ResponseValueStatus, format_compact_number,
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct ExperimentPlannerPanel {
    plan: ExperimentPlan,
    selected_run: Option<ExperimentRunId>,
    selected_response: ResponseSpecId,
    capture_value: f64,
    show_missing_only: bool,
}

impl ExperimentPlannerPanel {
    pub(crate) fn new() -> Self {
        let plan = ExperimentPlan::sample();
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
        }
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
                ui_chrome::muted(ui, &self.plan.objective);

                self.summary_metrics_ui(ui, &analysis);

                ui.separator();
                self.factor_overview_ui(ui);

                ui.separator();
                self.run_matrix_ui(ui);

                ui.separator();
                self.analysis_ui(ui, &analysis);

                if ui.button("Capture next pending demo response").clicked() {
                    self.capture_next_demo_response(status);
                }
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "DOE Planner");
        ui.label(&self.plan.title);
        ui.small(format!("{}  {}", self.plan.id, self.plan.status.label()));
        ui.separator();
        ui_chrome::section_label(ui, "FabOS Links");
        ui.label(format!("Route: {}", self.plan.route_id));
        ui.label(format!("Step: {}", self.plan.step_id));
        ui.label(format!("Baseline: {}", self.plan.baseline_recipe));

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
            });
        }

        ui.separator();
        if ui.button("Reset DOE sample").clicked() {
            *self = Self::new();
            *status = "DOE: reset sample experiment plan".to_string();
        }

        ui.separator();
        for note in &self.plan.notes {
            ui.small(note);
        }
    }

    pub(crate) fn response_capture_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Response Capture");

        let run_label = self
            .selected_run
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string());
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

        let response_label = self
            .plan
            .response(&self.selected_response)
            .map(|response| response.name.clone())
            .unwrap_or_else(|| "none".to_string());
        egui::ComboBox::from_id_salt("doe_capture_response")
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

        ui.horizontal(|ui| {
            ui.label("Value");
            ui.add(
                egui::DragValue::new(&mut self.capture_value)
                    .speed(0.1)
                    .range(-1_000.0..=10_000.0),
            );
        });

        ui.horizontal_wrapped(|ui| {
            if ui.button("Capture").clicked() {
                self.capture_selected_response(status);
            }
            if ui.button("Capture next demo").clicked() {
                self.capture_next_demo_response(status);
            }
        });
        ui.checkbox(&mut self.show_missing_only, "Show pending only");

        ui.separator();
        self.selected_run_detail_ui(ui);
    }

    fn summary_metrics_ui(&self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        ui.horizontal_wrapped(|ui| {
            experiment_metric_ui(
                ui,
                "Runs",
                format!(
                    "{} / {} complete",
                    analysis.completed_runs, analysis.run_count
                ),
                format!("{} pending values", analysis.missing_response_count),
            );
            experiment_metric_ui(
                ui,
                "Matrix",
                format!("{} factors", self.plan.factors.len()),
                format!("{} responses", self.plan.responses.len()),
            );
            experiment_metric_ui(
                ui,
                "Best run",
                analysis
                    .best_run_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "pending".to_string()),
                self.selected_response_label(),
            );
        });
    }

    fn factor_overview_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Factor Split");
        ui.horizontal_wrapped(|ui| {
            for factor in &self.plan.factors {
                ui.group(|ui| {
                    ui.strong(&factor.name);
                    ui.small(factor.source.label());
                    for level in &factor.levels {
                        ui.label(format!(
                            "{} {}",
                            level.label,
                            factor.unit.as_deref().unwrap_or("")
                        ));
                    }
                });
            }
        });
    }

    fn run_matrix_ui(&mut self, ui: &mut egui::Ui) {
        let rows = self.run_matrix_rows();
        let factor_names = self
            .plan
            .factors
            .iter()
            .map(|factor| factor.name.clone())
            .collect::<Vec<_>>();

        ui_chrome::section_label(ui, "Run Matrix");
        egui::Grid::new("doe_run_matrix")
            .striped(true)
            .min_col_width(68.0)
            .show(ui, |ui| {
                ui.strong("Run");
                ui.strong("Wafer");
                for factor_name in &factor_names {
                    ui.strong(factor_name);
                }
                ui.strong("Status");
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
                    ui.label(row.assignment);
                    for factor_value in row.factor_values {
                        ui.label(factor_value);
                    }
                    ui.colored_label(row.status_color, row.status);
                    ui.colored_label(row.primary_color, row.primary_value);
                    ui.end_row();
                }
            });
    }

    fn analysis_ui(&self, ui: &mut egui::Ui, analysis: &ExperimentAnalysisSummary) {
        ui.columns(2, |columns| {
            ui_chrome::section_label(&mut columns[0], "Response Summary");
            response_stats_ui(&mut columns[0], &self.plan, &analysis.response_stats);

            ui_chrome::section_label(
                &mut columns[1],
                &format!("Main Effects: {}", self.selected_response_label()),
            );
            factor_effects_ui(&mut columns[1], &analysis.factor_effects);
        });

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

        ui.label(format!("Run {}", run.id));
        ui.label(format!(
            "{} slot {}",
            run.assignment.wafer_id, run.assignment.slot
        ));
        if let Some(block) = &run.assignment.block {
            ui.small(block);
        }
        ui.colored_label(status_color(run.status), run.status.label());

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
                    let color = response_status_color(response.value_status(value.value));
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(color, format_response_value(value.value, response));
                        ui.label(&response.name);
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

    fn capture_selected_response(&mut self, status: &mut String) {
        let Some(run_id) = self.selected_run.clone() else {
            *status = "DOE: select a run before capturing a response".to_string();
            return;
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
            }
            Err(err) => {
                *status = format!("DOE capture blocked: {}", response_error_label(&err));
            }
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
                RunMatrixRow {
                    run_id: run.id.to_string(),
                    selected: self.selected_run.as_ref() == Some(&run.id),
                    assignment: format!("{} W{:02}", run.assignment.lot_id, run.assignment.slot),
                    factor_values: self
                        .plan
                        .factors
                        .iter()
                        .map(|factor| self.plan.run_factor_label(run, factor))
                        .collect(),
                    status: run.status.label().to_string(),
                    status_color: status_color(run.status),
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
}

struct RunMatrixRow {
    run_id: String,
    selected: bool,
    assignment: String,
    factor_values: Vec<String>,
    status: String,
    status_color: Color32,
    primary_value: String,
    primary_color: Color32,
}

fn experiment_metric_ui(ui: &mut egui::Ui, label: &str, value: String, detail: String) {
    ui_chrome::metric_tile_tone(ui, label, value, &detail, Tone::Info);
}

fn response_stats_ui(ui: &mut egui::Ui, plan: &ExperimentPlan, stats: &[ResponseStats]) {
    egui::Grid::new("doe_response_stats")
        .striped(true)
        .min_col_width(72.0)
        .show(ui, |ui| {
            ui.strong("Response");
            ui.strong("Mean");
            ui.strong("Range");
            ui.strong("N");
            ui.end_row();
            for stat in stats {
                let response = plan.response(&stat.response_id);
                ui.label(
                    response
                        .map(|response| response.name.as_str())
                        .unwrap_or(stat.response_id.as_str()),
                );
                ui.label(format_optional_number(stat.mean));
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
            ui.end_row();
            for effect in effects {
                ui.label(&effect.factor_name);
                ui.label(&effect.level_label);
                ui.label(format_compact_number(effect.mean_response));
                ui.colored_label(
                    delta_color(effect.delta_from_overall),
                    format_signed_number(effect.delta_from_overall),
                );
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

    let dose_cd = match dose_level {
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
