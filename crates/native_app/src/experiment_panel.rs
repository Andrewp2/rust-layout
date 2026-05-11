use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::experiment::{
    ExperimentAnalysisSummary, ExperimentFactor, ExperimentPlan, ExperimentRun, ExperimentRunId,
    ExperimentRunStatus, FactorEffect, FactorId, ResponseCaptureError, ResponseSpec,
    ResponseSpecId, ResponseStats, ResponseValue, ResponseValueStatus, format_compact_number,
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
const OPERAD_ROW_HEIGHT: f32 = 62.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_RUN_ROW_LIMIT: usize = 32;
const OPERAD_EFFECT_ROW_LIMIT: usize = 24;
const OPERAD_ACTION_SELECT_RESPONSE: &str = "experiment.action.select_response.";
const OPERAD_ACTION_SELECT_RUN: &str = "experiment.action.select_run.";
const OPERAD_ACTION_SET_FILTER: &str = "experiment.action.set_filter.";
const OPERAD_ACTION_SELECT_LOT: &str = "experiment.action.select_lot.";
const OPERAD_ACTION_CLEAR_FILTERS: &str = "experiment.action.clear_filters";
const OPERAD_ACTION_TOGGLE_MISSING: &str = "experiment.action.toggle_missing_only";
const OPERAD_ACTION_QUEUE_PENDING: &str = "experiment.action.queue_pending";
const OPERAD_ACTION_CAPTURE_DEMO: &str = "experiment.action.capture_demo";

pub(crate) struct ExperimentPlannerPanel {
    plan: ExperimentPlan,
    selected_run: Option<ExperimentRunId>,
    selected_response: ResponseSpecId,
    capture_value: f64,
    show_missing_only: bool,
    run_filter: RunMatrixFilter,
    lot_filter: Option<String>,
}

#[derive(Debug)]
struct ExperimentOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct ExperimentMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct ExperimentOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

impl ExperimentPlannerPanel {
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
        if let Err(error) = self.operad_dashboard_ui(ui, status) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, status);
        }
    }

    fn operad_dashboard_ui(
        &mut self,
        ui: &mut egui::Ui,
        status: &mut String,
    ) -> Result<(), String> {
        let mut action_status = None;
        let mut result_state = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("experiment_planner_dashboard_operad")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width);
                if let Err(error) = view
                    .document
                    .compute_layout(view.size, &mut ApproxTextMeasurer)
                    .map_err(|error| error.to_string())
                {
                    result_state = Err(error);
                    return;
                }

                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(node_name) =
                        operad_egui::hit_test_name(&view.document, rect, pointer)
                    && let Some(message) = self.handle_operad_action(&node_name)
                {
                    if !message.is_empty() {
                        action_status = Some(message);
                    }
                    view = self.build_operad_view(width);
                    if let Err(error) = view
                        .document
                        .compute_layout(view.size, &mut ApproxTextMeasurer)
                        .map_err(|error| error.to_string())
                    {
                        result_state = Err(error);
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
        if let Some(message) = action_status {
            *status = message;
        }
        result_state
    }

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
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

    fn build_operad_view(&self, width: f32) -> ExperimentOperadView {
        let primary_response_id = self.selected_response_if_valid();
        let analysis = self.plan.analysis_summary(primary_response_id.as_ref());
        let size;
        let mut document;

        if self.is_blank_plan() {
            let height = experiment_operad_view_height(width, 0, &[0]);
            size = UiSize::new(width, height);
            document = UiDocument::new(root_style(width, height));
            let root = document.root;
            document.set_node_visual(
                root,
                UiVisual::panel(
                    ColorRgba::new(15, 18, 21, 255),
                    Some(StrokeStyle::new(ColorRgba::new(39, 46, 52, 255), 1.0)),
                    0.0,
                ),
            );
            add_experiment_operad_header(
                &mut document,
                root,
                "PROCESS ENGINEERING",
                "DOE Planner",
                "Plan factorial splits, capture responses, and track readiness for analysis",
                "No experiment plan loaded",
            );
            add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
            add_experiment_operad_section(
                &mut document,
                root,
                width,
                "experiment.empty",
                "Experiment Plan",
                "No experiment plan loaded",
                &[],
            );
            return ExperimentOperadView { document, size };
        }

        let metrics = self.operad_metrics(&analysis);
        let control_rows = self.operad_control_rows();
        let factor_rows = self.operad_factor_rows();
        let response_rows = self.operad_response_rows(&analysis);
        let split_rows = self.operad_split_lot_rows();
        let run_rows = self.operad_run_rows();
        let selected_run_rows = self.operad_selected_run_rows();
        let readiness_rows = self.operad_readiness_rows(&analysis);
        let response_summary_rows = self.operad_response_summary_rows(&analysis);
        let effect_rows = self.operad_effect_rows(&analysis);
        let height = experiment_operad_view_height(
            width,
            metrics.len(),
            &[
                control_rows.len(),
                factor_rows.len(),
                response_rows.len(),
                split_rows.len(),
                run_rows.len(),
                selected_run_rows.len(),
                readiness_rows.len(),
                response_summary_rows.len(),
                effect_rows.len(),
            ],
        );
        size = UiSize::new(width, height);
        document = UiDocument::new(root_style(width, height));
        let root = document.root;
        document.set_node_visual(
            root,
            UiVisual::panel(
                ColorRgba::new(15, 18, 21, 255),
                Some(StrokeStyle::new(ColorRgba::new(39, 46, 52, 255), 1.0)),
                0.0,
            ),
        );

        add_experiment_operad_header(
            &mut document,
            root,
            "PROCESS ENGINEERING",
            "DOE Planner",
            &self.plan.objective,
            &format!(
                "{} · {} · owner {} · {} run(s)",
                self.plan.id,
                self.plan.status.label(),
                self.plan.owner,
                self.plan.runs.len()
            ),
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_metric_grid(&mut document, root, width, &metrics);
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.controls",
            "Experiment Controls",
            "No experiment controls available",
            &control_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.factors",
            "Factor Split",
            "No DOE factors defined",
            &factor_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.responses",
            "Response Plan",
            "No DOE responses defined",
            &response_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.splits",
            "Split Lots",
            "No split-lot assignments in this plan",
            &split_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.runs",
            "Run Matrix",
            "No runs match the active DOE filters",
            &run_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.selected_run",
            "Selected Run",
            "No run selected",
            &selected_run_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.readiness",
            "Analysis Readiness",
            "No analysis readiness available",
            &readiness_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.response_summary",
            "Response Summary",
            "No response values captured",
            &response_summary_rows,
        );
        add_experiment_operad_spacer(&mut document, root, OPERAD_GAP);
        add_experiment_operad_section(
            &mut document,
            root,
            width,
            "experiment.effects",
            &format!("Main Effects: {}", self.selected_response_label()),
            "No main-effect data captured for the selected response",
            &effect_rows,
        );

        ExperimentOperadView { document, size }
    }

    fn operad_metrics(&self, analysis: &ExperimentAnalysisSummary) -> Vec<ExperimentMetricTile> {
        let total_values = analysis.run_count.saturating_mul(self.plan.responses.len());
        let captured_values = total_values.saturating_sub(analysis.missing_response_count);
        let progress = analysis_progress(analysis, self.plan.responses.len());
        let split_count = self.split_lot_summaries().len();
        vec![
            ExperimentMetricTile {
                label: "Runs".to_string(),
                value: format!("{} / {}", analysis.completed_runs, analysis.run_count),
                detail: format!(
                    "{} pending response values",
                    analysis.missing_response_count
                ),
                tone: if analysis.completed_runs == analysis.run_count && analysis.run_count > 0 {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            },
            ExperimentMetricTile {
                label: "Responses".to_string(),
                value: format!("{captured_values} / {total_values}"),
                detail: format!("{} response specs", self.plan.responses.len()),
                tone: readiness_tone(analysis),
            },
            ExperimentMetricTile {
                label: "Split lots".to_string(),
                value: split_count.to_string(),
                detail: format!("{} factor(s) in matrix", self.plan.factors.len()),
                tone: Tone::Info,
            },
            ExperimentMetricTile {
                label: "Readiness".to_string(),
                value: format!("{:.0}%", progress * 100.0),
                detail: readiness_label(analysis, self.plan.responses.len()),
                tone: readiness_tone(analysis),
            },
            ExperimentMetricTile {
                label: "Primary response".to_string(),
                value: self.selected_response_label(),
                detail: analysis
                    .best_run_id
                    .as_ref()
                    .map(|run_id| format!("best observed run {run_id}"))
                    .unwrap_or_else(|| "best run pending".to_string()),
                tone: Tone::Info,
            },
            ExperimentMetricTile {
                label: "Matrix".to_string(),
                value: format!("{} factors", self.plan.factors.len()),
                detail: format!(
                    "{} run(s) after active filters",
                    self.filtered_run_matrix_rows().len()
                ),
                tone: Tone::Neutral,
            },
        ]
    }

    fn operad_control_rows(&self) -> Vec<ExperimentOperadRow> {
        let mut rows = vec![
            experiment_operad_row(
                "Queue next pending response",
                format!(
                    "Selects the next missing response after {}",
                    self.selected_run_label()
                ),
                Tone::Info,
                Some(OPERAD_ACTION_QUEUE_PENDING.to_string()),
                false,
            ),
            experiment_operad_row(
                "Capture next demo response",
                "Fills one pending demo value and advances the selected run/response",
                Tone::Warning,
                Some(OPERAD_ACTION_CAPTURE_DEMO.to_string()),
                false,
            ),
            experiment_operad_row(
                if self.show_missing_only {
                    "Detail pending only: on"
                } else {
                    "Detail pending only: off"
                },
                "Controls whether selected-run detail hides completed response values",
                Tone::Neutral,
                Some(OPERAD_ACTION_TOGGLE_MISSING.to_string()),
                self.show_missing_only,
            ),
            experiment_operad_row(
                "Clear filters",
                format!(
                    "Run filter {}, lot {}",
                    self.run_filter.label(),
                    self.lot_filter.as_deref().unwrap_or("all lots")
                ),
                Tone::Neutral,
                Some(OPERAD_ACTION_CLEAR_FILTERS.to_string()),
                false,
            ),
        ];
        rows.extend(RunMatrixFilter::ALL.into_iter().map(|filter| {
            experiment_operad_row(
                format!("Run filter: {}", filter.label()),
                format!("{} matching run(s)", self.filtered_run_count_for(filter)),
                if self.run_filter == filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_FILTER}{}|filter",
                    filter.slug()
                )),
                self.run_filter == filter,
            )
        }));
        rows
    }

    fn operad_factor_rows(&self) -> Vec<ExperimentOperadRow> {
        self.plan
            .factors
            .iter()
            .map(|factor| {
                let levels = factor
                    .levels
                    .iter()
                    .map(|level| {
                        format!(
                            "{}: {}",
                            level.label,
                            level.value_label(factor.unit.as_deref())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                experiment_operad_row(
                    format!("{} · {} level(s)", factor.name, factor.levels.len()),
                    format!("{} · {levels}", factor.source.label()),
                    Tone::Neutral,
                    None,
                    false,
                )
            })
            .collect()
    }

    fn operad_response_rows(
        &self,
        analysis: &ExperimentAnalysisSummary,
    ) -> Vec<ExperimentOperadRow> {
        self.plan
            .responses
            .iter()
            .map(|response| {
                let stats = analysis
                    .response_stats
                    .iter()
                    .find(|stat| stat.response_id == response.id);
                let stat_detail = stats
                    .map(|stat| {
                        format!(
                            "N {} / missing {} / mean {}",
                            stat.sample_count,
                            stat.missing_count,
                            format_optional_number(stat.mean)
                        )
                    })
                    .unwrap_or_else(|| "no captured values".to_string());
                let selected = self.selected_response == response.id;
                experiment_operad_row(
                    format!("{} · {}", response.name, response.goal.label()),
                    format!(
                        "{} · {} · {}",
                        response_spec_label(response),
                        response.source.label(),
                        stat_detail
                    ),
                    if selected {
                        Tone::Info
                    } else if stats.is_some_and(|stat| stat.missing_count == 0) {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_RESPONSE}{}|response",
                        response.id.as_str()
                    )),
                    selected,
                )
            })
            .collect()
    }

    fn operad_split_lot_rows(&self) -> Vec<ExperimentOperadRow> {
        self.split_lot_summaries()
            .into_iter()
            .map(|split| {
                let selected = self.lot_filter.as_deref() == Some(split.lot_id.as_str());
                experiment_operad_row(
                    format!("{} · {}", split.lot_id, split.block),
                    format!(
                        "{} wafer(s), {} run(s), order {}-{}, {} pending, {} out of spec",
                        split.wafer_ids.len(),
                        split.run_count,
                        split.min_order,
                        split.max_order,
                        split.pending_values,
                        split.out_of_spec_count
                    ),
                    if split.out_of_spec_count > 0 {
                        Tone::Danger
                    } else if split.pending_values > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                    Some(format!("{OPERAD_ACTION_SELECT_LOT}{}|lot", split.lot_id)),
                    selected,
                )
            })
            .collect()
    }

    fn operad_run_rows(&self) -> Vec<ExperimentOperadRow> {
        let rows = self.filtered_run_matrix_rows();
        let mut operad_rows = rows
            .iter()
            .take(OPERAD_RUN_ROW_LIMIT)
            .map(|row| {
                let factor_detail = self
                    .plan
                    .factors
                    .iter()
                    .map(|factor| factor.name.as_str())
                    .zip(row.factor_values.iter().map(String::as_str))
                    .map(|(factor, value)| format!("{factor}: {value}"))
                    .collect::<Vec<_>>()
                    .join(" · ");
                experiment_operad_row(
                    format!("{} · {}", row.run_id, row.status),
                    format!(
                        "order {} · {} W{:02} · split {} · captured {} · primary {} · {}",
                        row.run_order,
                        row.wafer_id,
                        row.slot,
                        row.block,
                        row.capture_summary,
                        row.primary_value,
                        factor_detail
                    ),
                    run_status_tone(row.status_kind),
                    Some(format!("{OPERAD_ACTION_SELECT_RUN}{}|run", row.run_id)),
                    row.selected,
                )
            })
            .collect::<Vec<_>>();
        if rows.len() > OPERAD_RUN_ROW_LIMIT {
            operad_rows.push(experiment_operad_row(
                format!(
                    "Showing first {} of {} matching runs",
                    OPERAD_RUN_ROW_LIMIT,
                    rows.len()
                ),
                "Use the run and lot filters to narrow the matrix before inspecting individual runs",
                Tone::Info,
                None,
                false,
            ));
        }
        operad_rows
    }

    fn operad_selected_run_rows(&self) -> Vec<ExperimentOperadRow> {
        let Some(run) = self
            .selected_run
            .as_ref()
            .and_then(|run_id| self.plan.run(run_id))
        else {
            return Vec::new();
        };
        let mut rows = vec![
            experiment_operad_row(
                format!("Run {} · {}", run.id, run.status.label()),
                format!(
                    "{} / {} slot {} / order {} / split {}",
                    run.assignment.lot_id,
                    run.assignment.wafer_id,
                    run.assignment.slot,
                    run.assignment.run_order,
                    run.assignment.block.as_deref().unwrap_or("unblocked")
                ),
                run_status_tone(run.status),
                None,
                false,
            ),
            experiment_operad_row(
                "Execution context",
                format!(
                    "Recipe {} · tool {}",
                    run.recipe
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "unassigned".to_string()),
                    run.tool_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "unassigned".to_string())
                ),
                Tone::Neutral,
                None,
                false,
            ),
        ];
        rows.extend(self.plan.factors.iter().map(|factor| {
            experiment_operad_row(
                format!("Factor: {}", factor.name),
                self.plan.run_factor_label(run, factor),
                Tone::Neutral,
                None,
                false,
            )
        }));
        for response in &self.plan.responses {
            let captured = run.responses.get(&response.id);
            if self.show_missing_only && captured.is_some() {
                continue;
            }
            let selected = self.selected_response == response.id;
            match captured {
                Some(value) => {
                    let value_status = response.value_status(value.value);
                    rows.push(experiment_operad_row(
                        format!(
                            "{} · {}",
                            format_response_value(value.value, response),
                            response.name
                        ),
                        format!(
                            "{} · captured {} · {}",
                            value_status.label(),
                            value.captured_at,
                            value
                                .measurement_id
                                .as_deref()
                                .unwrap_or("no measurement id")
                        ),
                        response_value_tone(value_status),
                        Some(format!(
                            "{OPERAD_ACTION_SELECT_RESPONSE}{}|selected-run-response",
                            response.id.as_str()
                        )),
                        selected,
                    ));
                }
                None => rows.push(experiment_operad_row(
                    format!("pending · {}", response.name),
                    response_spec_label(response),
                    Tone::Warning,
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_RESPONSE}{}|selected-run-response",
                        response.id.as_str()
                    )),
                    selected,
                )),
            }
        }
        rows
    }

    fn operad_readiness_rows(
        &self,
        analysis: &ExperimentAnalysisSummary,
    ) -> Vec<ExperimentOperadRow> {
        let progress = analysis_progress(analysis, self.plan.responses.len());
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
        let mut rows = vec![
            experiment_operad_row(
                readiness_label(analysis, self.plan.responses.len()),
                format!(
                    "{:.0}% response matrix captured · {} complete runs · {} pending values",
                    progress * 100.0,
                    analysis.completed_runs,
                    analysis.missing_response_count
                ),
                readiness_tone(analysis),
                None,
                false,
            ),
            experiment_operad_row(
                "Primary response coverage",
                format!(
                    "{primary_missing} run(s) still need {} · {} level effect(s) observed",
                    self.selected_response_label(),
                    analysis.factor_effects.len()
                ),
                if primary_missing == 0 {
                    Tone::Success
                } else {
                    Tone::Warning
                },
                None,
                false,
            ),
        ];
        if let Some(best_run_id) = &analysis.best_run_id {
            rows.push(experiment_operad_row(
                "Best observed run",
                best_run_id.to_string(),
                Tone::Info,
                Some(format!(
                    "{OPERAD_ACTION_SELECT_RUN}{}|best-run",
                    best_run_id.as_str()
                )),
                self.selected_run.as_ref() == Some(best_run_id),
            ));
        }
        rows.extend(
            analysis.notes.iter().map(|note| {
                experiment_operad_row("Analysis note", note, Tone::Neutral, None, false)
            }),
        );
        rows
    }

    fn operad_response_summary_rows(
        &self,
        analysis: &ExperimentAnalysisSummary,
    ) -> Vec<ExperimentOperadRow> {
        analysis
            .response_stats
            .iter()
            .map(|stat| {
                let response = self.plan.response(&stat.response_id);
                let title = response
                    .map(|response| response.name.clone())
                    .unwrap_or_else(|| stat.response_id.to_string());
                let range = match (stat.min, stat.max) {
                    (Some(min), Some(max)) => {
                        format!(
                            "{}..{}",
                            format_compact_number(min),
                            format_compact_number(max)
                        )
                    }
                    _ => "-".to_string(),
                };
                experiment_operad_row(
                    title,
                    format!(
                        "mean {} · std dev {} · range {} · N {} / missing {} · {}",
                        format_optional_number(stat.mean),
                        format_optional_number(stat.std_dev),
                        range,
                        stat.sample_count,
                        stat.missing_count,
                        response
                            .map(response_spec_label)
                            .unwrap_or_else(|| "no spec".to_string())
                    ),
                    if stat.missing_count == 0 {
                        Tone::Success
                    } else if stat.sample_count > 0 {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                    response.map(|response| {
                        format!(
                            "{OPERAD_ACTION_SELECT_RESPONSE}{}|response-summary",
                            response.id.as_str()
                        )
                    }),
                    response.is_some_and(|response| self.selected_response == response.id),
                )
            })
            .collect()
    }

    fn operad_effect_rows(&self, analysis: &ExperimentAnalysisSummary) -> Vec<ExperimentOperadRow> {
        let mut rows = analysis
            .factor_effects
            .iter()
            .take(OPERAD_EFFECT_ROW_LIMIT)
            .map(|effect| {
                experiment_operad_row(
                    format!("{} · {}", effect.factor_name, effect.level_label),
                    format!(
                        "mean {} · delta {} · N {}",
                        format_compact_number(effect.mean_response),
                        format_signed_number(effect.delta_from_overall),
                        effect.sample_count
                    ),
                    if effect.delta_from_overall.abs() < 0.01 {
                        Tone::Neutral
                    } else {
                        Tone::Info
                    },
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if analysis.factor_effects.len() > OPERAD_EFFECT_ROW_LIMIT {
            rows.push(experiment_operad_row(
                format!(
                    "Showing first {} of {} level effects",
                    OPERAD_EFFECT_ROW_LIMIT,
                    analysis.factor_effects.len()
                ),
                "Narrow the primary response or inspect the detailed table for the full effect set",
                Tone::Info,
                None,
                false,
            ));
        }
        rows
    }

    fn filtered_run_count_for(&self, filter: RunMatrixFilter) -> usize {
        self.run_matrix_rows()
            .into_iter()
            .filter(|row| {
                self.lot_filter
                    .as_deref()
                    .is_none_or(|lot_id| row.lot_id == lot_id)
            })
            .filter(|row| filter.matches(row))
            .count()
    }

    fn handle_operad_action(&mut self, node_name: &str) -> Option<String> {
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SELECT_RESPONSE) {
            let response_id = ResponseSpecId::new(rest.split('|').next().unwrap_or_default());
            if self.plan.response(&response_id).is_some() {
                self.selected_response = response_id.clone();
                return Some(format!("DOE primary response set to {response_id}"));
            }
            return None;
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SELECT_RUN) {
            let run_id = ExperimentRunId::new(rest.split('|').next().unwrap_or_default());
            if self.plan.run(&run_id).is_some() {
                self.selected_run = Some(run_id.clone());
                return Some(format!("DOE run selected: {run_id}"));
            }
            return None;
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SET_FILTER) {
            let slug = rest.split('|').next().unwrap_or_default();
            let filter = RunMatrixFilter::from_slug(slug)?;
            self.run_filter = filter;
            return Some(format!("DOE run filter set to {}", filter.label()));
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SELECT_LOT) {
            let lot_id = rest.split('|').next().unwrap_or_default();
            if self.lot_filter.as_deref() == Some(lot_id) {
                self.lot_filter = None;
                return Some("DOE lot filter cleared".to_string());
            }
            if self.lot_options().iter().any(|option| option == lot_id) {
                self.lot_filter = Some(lot_id.to_string());
                return Some(format!("DOE lot filter set to {lot_id}"));
            }
            return None;
        }
        match node_name {
            OPERAD_ACTION_CLEAR_FILTERS => {
                self.run_filter = RunMatrixFilter::All;
                self.lot_filter = None;
                self.show_missing_only = false;
                Some("DOE filters cleared".to_string())
            }
            OPERAD_ACTION_TOGGLE_MISSING => {
                self.show_missing_only = !self.show_missing_only;
                Some(format!(
                    "DOE pending-only detail {}",
                    if self.show_missing_only {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ))
            }
            OPERAD_ACTION_QUEUE_PENDING => {
                let mut status = String::new();
                self.queue_next_pending_response(&mut status);
                Some(status)
            }
            OPERAD_ACTION_CAPTURE_DEMO => {
                let mut status = String::new();
                self.capture_next_demo_response(&mut status);
                Some(status)
            }
            _ => None,
        }
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui, status);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        if self.is_blank_plan() {
            return render_sidecar(
                ui,
                "experiment.context",
                &[SidecarSection::new("DOE Planner").empty("No experiment plan loaded")],
            );
        }
        let primary_response_id = self.selected_response_if_valid();
        let analysis = self.plan.analysis_summary(primary_response_id.as_ref());
        let progress = analysis_progress(&analysis, self.plan.responses.len());
        let pending_values = analysis.missing_response_count;
        let mut sections = vec![
            SidecarSection::new("DOE Planner")
                .row(
                    SidecarRow::new(
                        &self.plan.title,
                        format!("{} | {}", self.plan.id, self.plan.status.label()),
                        Tone::Info,
                    )
                    .selected(true),
                )
                .row(SidecarRow::new(
                    "FabOS Links",
                    format!(
                        "{} | {} | {}",
                        self.plan.route_id, self.plan.step_id, self.plan.baseline_recipe
                    ),
                    Tone::Neutral,
                )),
            SidecarSection::new("Readiness")
                .row(SidecarRow::new(
                    format!("{:.0}% captured", progress * 100.0),
                    readiness_label(&analysis, self.plan.responses.len()),
                    if progress >= 1.0 {
                        Tone::Success
                    } else if pending_values > 0 {
                        Tone::Warning
                    } else {
                        Tone::Info
                    },
                ))
                .row(SidecarRow::new(
                    "Runs / pending",
                    format!(
                        "{} runs | {pending_values} pending values",
                        self.plan.runs.len()
                    ),
                    if pending_values > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                )),
        ];
        let mut splits = SidecarSection::new("Split Lots").empty("No split lots");
        for split in self.split_lot_summaries().into_iter().take(4) {
            splits = splits.row(SidecarRow::new(
                split.lot_id,
                format!(
                    "{} wafers | {} runs | {} pending values",
                    split.wafer_ids.len(),
                    split.run_count,
                    split.pending_values
                ),
                if split.pending_values > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            ));
        }
        sections.push(splits);
        let mut responses = SidecarSection::new("Responses").empty("No responses");
        for response in self.plan.responses.iter().take(4) {
            responses = responses.row(SidecarRow::new(
                &response.name,
                format!(
                    "{} | {} | {}",
                    response.goal.label(),
                    response.unit,
                    response.source.label()
                ),
                Tone::Info,
            ));
        }
        sections.push(responses);
        render_sidecar(ui, "experiment.context", &sections)
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
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

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsAnyResponse => "needs-any-response",
            Self::NeedsSelectedResponse => "needs-selected-response",
            Self::InProgress => "in-progress",
            Self::Complete => "complete",
            Self::OutOfSpec => "out-of-spec",
            Self::Blocked => "blocked",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "needs-any-response" => Some(Self::NeedsAnyResponse),
            "needs-selected-response" => Some(Self::NeedsSelectedResponse),
            "in-progress" => Some(Self::InProgress),
            "complete" => Some(Self::Complete),
            "out-of-spec" => Some(Self::OutOfSpec),
            "blocked" => Some(Self::Blocked),
            _ => None,
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

fn experiment_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    if metric_count > 0 {
        height += experiment_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    }
    for row_count in row_counts {
        height += experiment_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn experiment_operad_metric_columns(width: f32) -> usize {
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

fn experiment_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    if metric_count == 0 {
        return 0.0;
    }
    let columns = experiment_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn experiment_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_experiment_operad_header(
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
            "experiment.header",
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
    add_experiment_operad_text(
        document,
        header,
        "experiment.header.eyebrow",
        eyebrow,
        experiment_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_experiment_operad_text(
        document,
        header,
        "experiment.header.title",
        title,
        experiment_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_experiment_operad_text(
        document,
        header,
        "experiment.header.detail",
        truncate_middle(detail, 120),
        experiment_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_experiment_operad_text(
        document,
        header,
        "experiment.header.meta",
        meta,
        experiment_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_experiment_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[ExperimentMetricTile],
) {
    if metrics.is_empty() {
        return;
    }
    let columns = experiment_operad_metric_columns(width);
    let grid_height = experiment_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "experiment.metrics",
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
                format!("experiment.metrics.row.{row_index}"),
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
            add_experiment_operad_metric_tile(
                document,
                row,
                &format!("experiment.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_experiment_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &ExperimentMetricTile,
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
                experiment_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_experiment_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        experiment_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_experiment_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        truncate_middle(&metric.value, 42),
        experiment_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_experiment_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 56),
        experiment_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            experiment_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_experiment_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[ExperimentOperadRow],
) {
    let height = experiment_operad_section_height(rows.len());
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
    add_experiment_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        experiment_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_experiment_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_experiment_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_experiment_operad_empty_row(
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
    add_experiment_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        experiment_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_experiment_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &ExperimentOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        experiment_operad_tone_color(Tone::Info)
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
            experiment_operad_tone_color(row.tone),
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
    add_experiment_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 78),
        experiment_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        22.0,
    );
    add_experiment_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 116),
        experiment_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        20.0,
    );
}

fn add_experiment_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("experiment.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_experiment_operad_text(
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

fn experiment_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn experiment_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn experiment_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> ExperimentOperadRow {
    ExperimentOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
    }
}

fn run_status_tone(status: ExperimentRunStatus) -> Tone {
    match status {
        ExperimentRunStatus::Ready => Tone::Info,
        ExperimentRunStatus::InProgress => Tone::Warning,
        ExperimentRunStatus::Complete => Tone::Success,
        ExperimentRunStatus::Blocked => Tone::Danger,
    }
}

fn response_value_tone(status: ResponseValueStatus) -> Tone {
    match status {
        ResponseValueStatus::InSpec => Tone::Success,
        ResponseValueStatus::BelowSpec | ResponseValueStatus::AboveSpec => Tone::Danger,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_operad_view_audits_common_widths() {
        let mut panel = ExperimentPlannerPanel::from_plan(ExperimentPlan::sample());
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
    fn experiment_operad_actions_update_panel_state() {
        let mut panel = ExperimentPlannerPanel::from_plan(ExperimentPlan::sample());
        let target_response = panel.plan.responses.last().unwrap().id.clone();
        let target_run = panel.plan.runs.last().unwrap().id.clone();
        let target_lot = panel.lot_options().last().unwrap().clone();

        assert_eq!(
            panel.handle_operad_action(&format!(
                "{OPERAD_ACTION_SELECT_RESPONSE}{}|test",
                target_response.as_str()
            )),
            Some(format!("DOE primary response set to {target_response}"))
        );
        assert_eq!(panel.selected_response, target_response);

        assert_eq!(
            panel.handle_operad_action(&format!(
                "{OPERAD_ACTION_SELECT_RUN}{}|test",
                target_run.as_str()
            )),
            Some(format!("DOE run selected: {target_run}"))
        );
        assert_eq!(panel.selected_run.as_ref(), Some(&target_run));

        assert_eq!(
            panel.handle_operad_action(&format!(
                "{OPERAD_ACTION_SET_FILTER}{}|test",
                RunMatrixFilter::NeedsSelectedResponse.slug()
            )),
            Some("DOE run filter set to Needs selected response".to_string())
        );
        assert_eq!(panel.run_filter, RunMatrixFilter::NeedsSelectedResponse);

        assert_eq!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_SELECT_LOT}{target_lot}|test")),
            Some(format!("DOE lot filter set to {target_lot}"))
        );
        assert_eq!(panel.lot_filter.as_deref(), Some(target_lot.as_str()));

        assert!(!panel.show_missing_only);
        assert_eq!(
            panel.handle_operad_action(OPERAD_ACTION_TOGGLE_MISSING),
            Some("DOE pending-only detail enabled".to_string())
        );
        assert!(panel.show_missing_only);

        assert_eq!(
            panel.handle_operad_action(OPERAD_ACTION_CLEAR_FILTERS),
            Some("DOE filters cleared".to_string())
        );
        assert_eq!(panel.run_filter, RunMatrixFilter::All);
        assert!(panel.lot_filter.is_none());
        assert!(!panel.show_missing_only);

        let before_missing = panel
            .plan
            .analysis_summary(panel.selected_response_if_valid().as_ref())
            .missing_response_count;
        let message = panel
            .handle_operad_action(OPERAD_ACTION_CAPTURE_DEMO)
            .unwrap();
        let after_missing = panel
            .plan
            .analysis_summary(panel.selected_response_if_valid().as_ref())
            .missing_response_count;
        assert!(message.starts_with("DOE: captured demo"));
        assert!(after_missing < before_missing);
    }
}
