use eframe::egui::{self, Color32, RichText};
use layout_model::{
    equipment::ToolId,
    maintenance::{
        CalibrationOutcome, CalibrationRecord, DueState, FabDate, MaintenanceKind,
        MaintenanceModel, QualificationOutcome, ToolReleaseState,
    },
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct MaintenancePanel {
    model: MaintenanceModel,
    selected_tool: Option<ToolId>,
    work_filter: WorkFilter,
    history_filter: HistoryFilter,
}

impl MaintenancePanel {
    pub(crate) fn from_model(model: MaintenanceModel) -> Self {
        let selected_tool = model.tools.first().map(|tool| tool.tool_id.clone());
        Self {
            model,
            selected_tool,
            work_filter: WorkFilter::Actionable,
            history_filter: HistoryFilter::SelectedTool,
        }
    }

    pub(crate) fn model(&self) -> &MaintenanceModel {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Maintenance");
        let today = self.today();
        let work_items = self.work_items(today);
        let overdue = work_items
            .iter()
            .filter(|work| work.due_state == DueState::Overdue)
            .count();
        let at_risk = work_items
            .iter()
            .filter(|work| work.due_state == DueState::Due || work.days_until <= 7)
            .count();
        let locked = self.locked_tool_count(today);
        ui.label(format!("Audit date: {today}"));
        ui.label(format!("Action queue: {at_risk}"));
        ui.label(format!("Overdue: {overdue}"));
        ui.label(format!("Locked tools: {locked}"));

        ui.separator();
        ui_chrome::section_label(ui, "Selected Tool");
        let Some(tool_id) = self.selected_tool.clone() else {
            ui_chrome::empty_state(ui, "No maintenance tools loaded");
            return;
        };
        let Some(tool) = self.model.tool(&tool_id) else {
            ui_chrome::empty_state(ui, "Selected tool is missing");
            return;
        };
        let release = self.model.release_for_tool(&tool_id, today);
        ui.label(RichText::new(&tool.tool_name).strong());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, release.state.label(), release_tone(release.state));
            ui_chrome::status_pill(ui, &format!("{} runs", tool.run_count), Tone::Neutral);
        });
        self.release_reasons_ui(ui, &release.reasons);

        ui.separator();
        ui_chrome::section_label(ui, "Next Action");
        if let Some(next) = self.next_schedule_for_selected(today) {
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(ui, next.due_state.label(), due_state_tone(next.due_state));
                ui.label(RichText::new(&next.task).strong());
            });
            ui.small(format!(
                "Due {} ({})",
                next.due_date,
                due_window_label(next.days_until)
            ));
            if let Some(interval_runs) = next.interval_runs {
                ui.small(format!(
                    "{} of {} runs since completion",
                    next.run_count_delta.unwrap_or_default(),
                    interval_runs
                ));
            }
            for item in next.checklist.iter().take(4) {
                ui.label(format!("- {item}"));
            }
        } else {
            ui_chrome::empty_state(ui, "No scheduled work for selected tool");
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let today = self.today();
        egui::ScrollArea::vertical()
            .id_salt("maintenance_dashboard_scroll")
            .show(ui, |ui| {
                let detail = format!(
                    "Audit date {today} - {} tools tracked - {} locked",
                    self.model.tools.len(),
                    self.locked_tool_count(today)
                );
                ui_chrome::module_header(
                    ui,
                    "Fab operations",
                    "Maintenance and Calibration",
                    &detail,
                    |ui| {
                        ui.label("Tool");
                        self.tool_picker(ui, status);
                    },
                );

                if self.model.tools.is_empty() {
                    ui_chrome::empty_state(ui, "No maintenance model loaded");
                    return;
                }

                self.metric_row(ui, today);
                ui.separator();
                self.filter_bar_ui(ui, today);
                ui.separator();

                let available_width = ui.available_width();
                if available_width < 720.0 {
                    self.work_queue_ui(ui, today, status);
                    ui.separator();
                    self.selected_tool_ui(ui, today, status);
                    ui.separator();
                    self.audit_ui(ui);
                } else if available_width < 1080.0 {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(280.0);
                        self.work_queue_ui(&mut columns[0], today, status);
                        self.selected_tool_ui(&mut columns[1], today, status);
                    });
                    ui.separator();
                    self.audit_ui(ui);
                } else {
                    ui.columns(3, |columns| {
                        columns[0].set_min_width(280.0);
                        self.work_queue_ui(&mut columns[0], today, status);
                        self.selected_tool_ui(&mut columns[1], today, status);
                        self.release_board_ui(&mut columns[2], today);
                        columns[2].separator();
                        self.history_ui(&mut columns[2]);
                    });
                }
            });
    }

    fn tool_picker(&mut self, ui: &mut egui::Ui, status: &mut String) {
        let mut selected = self.selected_tool.clone();
        let selected_text = selected
            .as_ref()
            .and_then(|id| self.model.tool(id))
            .map(|tool| tool.tool_name.clone())
            .unwrap_or_else(|| "none".to_string());
        egui::ComboBox::from_id_salt("maintenance_tool_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for tool in &self.model.tools {
                    ui.selectable_value(&mut selected, Some(tool.tool_id.clone()), &tool.tool_name);
                }
            });
        if selected != self.selected_tool {
            self.selected_tool = selected;
            *status = "maintenance tool selected".to_string();
        }
    }

    fn metric_row(&self, ui: &mut egui::Ui, today: FabDate) {
        let work_items = self.work_items(today);
        let overdue = work_items
            .iter()
            .filter(|work| work.due_state == DueState::Overdue)
            .count();
        let due_now = work_items
            .iter()
            .filter(|work| work.due_state == DueState::Due)
            .count();
        let due_soon = work_items
            .iter()
            .filter(|work| work.due_state == DueState::NotDue && work.days_until <= 7)
            .count();
        let calibration_watch = work_items
            .iter()
            .filter(|work| {
                work.kind == MaintenanceKind::Calibration
                    && (work.due_state.blocks_release() || work.days_until <= 14)
            })
            .count()
            + self
                .model
                .tools
                .iter()
                .filter(|tool| {
                    self.latest_calibration(&tool.tool_id)
                        .is_some_and(|record| record.outcome != CalibrationOutcome::Passed)
                })
                .count();
        let locked = self.locked_tool_count(today);

        let metrics = [
            (
                "Overdue",
                overdue.to_string(),
                "release risk",
                if overdue > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            ),
            (
                "Due now",
                due_now.to_string(),
                "needs action",
                if due_now > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            ),
            ("Due soon", due_soon.to_string(), "next 7 days", Tone::Info),
            (
                "Calibration watch",
                calibration_watch.to_string(),
                "due or out of spec",
                if calibration_watch > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            ),
            (
                "Locked",
                locked.to_string(),
                "not production released",
                if locked > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            ),
        ];
        ui_chrome::metric_tiles(ui, &metrics);
    }

    fn filter_bar_ui(&mut self, ui: &mut egui::Ui, today: FabDate) {
        let matching = self.filtered_work_items(today).len();
        if ui.available_width() < 560.0 {
            ui.label("Queue filter");
            egui::ComboBox::from_id_salt("maintenance_work_filter")
                .selected_text(self.work_filter.label())
                .show_ui(ui, |ui| {
                    for filter in WorkFilter::ALL {
                        ui.selectable_value(&mut self.work_filter, filter, filter.label());
                    }
                });
            ui.small(format!("{matching} matching tasks"));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Queue filter");
                for filter in WorkFilter::ALL {
                    ui.selectable_value(&mut self.work_filter, filter, filter.label());
                }
                ui.separator();
                ui.small(format!("{matching} matching tasks"));
            });
        }
    }

    fn work_queue_ui(&mut self, ui: &mut egui::Ui, today: FabDate, status: &mut String) {
        ui_chrome::section_label(ui, "Maintenance Queue");
        let work_items = self.filtered_work_items(today);
        if work_items.is_empty() {
            ui_chrome::empty_state(ui, "No maintenance tasks match the current filter");
            return;
        }

        for work in work_items {
            let selected = self.selected_tool.as_ref() == Some(&work.tool_id);
            ui.group(|ui| {
                ui.set_width(ui.available_width().clamp(240.0, 560.0));
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(selected, work.tool_id.to_string())
                        .clicked()
                    {
                        self.selected_tool = Some(work.tool_id.clone());
                        *status = format!("selected maintenance tool {}", work.tool_id);
                    }
                    ui_chrome::status_pill(
                        ui,
                        work.due_state.label(),
                        due_state_tone(work.due_state),
                    );
                    ui_chrome::status_pill(ui, work.kind.label(), kind_tone(work.kind));
                });
                ui.add(egui::Label::new(RichText::new(&work.task).strong()).wrap());
                ui.small(format!(
                    "{} - due {} ({})",
                    work.tool_name,
                    work.due_date,
                    due_window_label(work.days_until)
                ));
                ui.horizontal_wrapped(|ui| {
                    ui.small(format!("Schedule {}", work.schedule_id));
                    ui.separator();
                    ui.small(format!(
                        "Release {}",
                        work.release_state.label().to_lowercase()
                    ));
                });
                if let Some(interval_runs) = work.interval_runs {
                    ui.add(
                        egui::ProgressBar::new(run_progress(
                            work.run_count_delta.unwrap_or_default(),
                            interval_runs,
                        ))
                        .text(format!(
                            "{} / {} runs",
                            work.run_count_delta.unwrap_or_default(),
                            interval_runs
                        )),
                    );
                }
                ui.small(format!("{} checklist items", work.checklist_len));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Stage work packet").clicked() {
                        *status = format!("work packet staged for {} {}", work.tool_id, work.task);
                    }
                    let log_label = if work.kind == MaintenanceKind::Calibration {
                        "Log calibration"
                    } else if work.kind == MaintenanceKind::Qualification {
                        "Log qualification"
                    } else {
                        "Log completion"
                    };
                    if ui.button(log_label).clicked() {
                        *status =
                            format!("{} ready for {}", log_label.to_lowercase(), work.tool_id);
                    }
                    if ui
                        .add_enabled(work.blocked, egui::Button::new("Release review"))
                        .clicked()
                    {
                        *status = format!("release review opened for {}", work.tool_id);
                    }
                });
            });
        }
    }

    fn selected_tool_ui(&self, ui: &mut egui::Ui, today: FabDate, status: &mut String) {
        ui_chrome::section_label(ui, "Tool Detail");
        let Some(tool_id) = self.selected_tool.as_ref() else {
            ui_chrome::empty_state(ui, "No tool selected");
            return;
        };
        let Some(tool) = self.model.tool(tool_id) else {
            ui_chrome::empty_state(ui, "Selected tool is missing");
            return;
        };
        let release = self.model.release_for_tool(tool_id, today);
        ui.label(RichText::new(&tool.tool_name).strong());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, release.state.label(), release_tone(release.state));
            ui_chrome::status_pill(ui, &format!("{} runs", tool.run_count), Tone::Neutral);
            let open_downtime = self.open_downtime_count(tool_id);
            if open_downtime > 0 {
                ui_chrome::status_pill(ui, &format!("{open_downtime} downtime"), Tone::Danger);
            }
        });
        ui.small(tool_id.to_string());
        self.release_reasons_ui(ui, &release.reasons);
        self.selected_tool_actions_ui(ui, tool_id, release.state, status);

        ui.separator();
        self.next_work_ui(ui, today, tool_id);
        ui.separator();
        self.schedule_ui(ui, today, tool_id);
        ui.separator();
        self.calibration_ui(ui, tool_id);
        ui.separator();
        self.qualification_ui(ui, tool_id);
        ui.separator();
        self.tool_history_ui(ui, tool_id);
    }

    fn selected_tool_actions_ui(
        &self,
        ui: &mut egui::Ui,
        tool_id: &ToolId,
        release_state: ToolReleaseState,
        status: &mut String,
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Schedule maintenance").clicked() {
                *status = format!("maintenance scheduling opened for {tool_id}");
            }
            let calibration_enabled = self.model.tool(tool_id).is_some_and(|tool| {
                tool.schedules
                    .iter()
                    .any(|schedule| schedule.kind == MaintenanceKind::Calibration)
            }) || release_state == ToolReleaseState::CalibrationLockout;
            if ui
                .add_enabled(
                    calibration_enabled,
                    egui::Button::new("Capture calibration"),
                )
                .clicked()
            {
                *status = format!("calibration capture opened for {tool_id}");
            }
            let review_enabled = !release_state.released_to_production();
            if ui
                .add_enabled(review_enabled, egui::Button::new("Resolve release hold"))
                .clicked()
            {
                *status = format!("release hold review opened for {tool_id}");
            }
        });
    }

    fn next_work_ui(&self, ui: &mut egui::Ui, today: FabDate, tool_id: &ToolId) {
        ui_chrome::section_label(ui, "Next Work");
        let Some(next) = self.next_schedule_for_tool(tool_id, today) else {
            ui_chrome::empty_state(ui, "No scheduled work");
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, next.due_state.label(), due_state_tone(next.due_state));
            ui_chrome::status_pill(ui, next.kind.label(), kind_tone(next.kind));
        });
        ui.add(egui::Label::new(RichText::new(&next.task).strong()).wrap());
        ui.small(format!(
            "Due {} ({})",
            next.due_date,
            due_window_label(next.days_until)
        ));
        if next.checklist.is_empty() {
            ui.small("No checklist recorded");
        } else {
            for item in next.checklist.iter().take(5) {
                ui.label(format!("- {item}"));
            }
            if next.checklist.len() > 5 {
                ui.small(format!("{} more checklist items", next.checklist.len() - 5));
            }
        }
    }

    fn schedule_ui(&self, ui: &mut egui::Ui, today: FabDate, tool_id: &ToolId) {
        ui_chrome::section_label(ui, "Schedules");
        let Some(tool) = self.model.tool(tool_id) else {
            ui_chrome::empty_state(ui, "Selected tool is missing");
            return;
        };
        if tool.schedules.is_empty() {
            ui_chrome::empty_state(ui, "No schedules loaded for this tool");
            return;
        }

        let mut schedules = tool.schedules.iter().collect::<Vec<_>>();
        schedules.sort_by_key(|schedule| {
            (
                due_state_priority(schedule.due_state(today, tool.run_count)),
                schedule.next_due,
                schedule.task.clone(),
            )
        });

        for schedule in schedules {
            let due_state = schedule.due_state(today, tool.run_count);
            ui.group(|ui| {
                ui.set_width(ui.available_width().clamp(240.0, 620.0));
                ui.horizontal_wrapped(|ui| {
                    ui_chrome::status_pill(ui, due_state.label(), due_state_tone(due_state));
                    ui_chrome::status_pill(ui, schedule.kind.label(), kind_tone(schedule.kind));
                    ui.label(RichText::new(&schedule.task).strong());
                });
                ui.small(format!(
                    "Last completed {} - next due {} ({})",
                    schedule.last_completed,
                    schedule.next_due,
                    due_window_label(today.days_until(schedule.next_due))
                ));
                if let Some(interval_days) = schedule.interval_days {
                    let elapsed = schedule.last_completed.days_until(today).max(0) as u32;
                    ui.add(
                        egui::ProgressBar::new(run_progress(elapsed, interval_days))
                            .text(format!("{elapsed} / {interval_days} days")),
                    );
                }
                if let Some(interval_runs) = schedule.interval_runs {
                    let run_delta = tool
                        .run_count
                        .saturating_sub(schedule.last_completed_run_count);
                    ui.add(
                        egui::ProgressBar::new(run_progress(run_delta, interval_runs))
                            .text(format!("{run_delta} / {interval_runs} runs")),
                    );
                }
            });
        }
    }

    fn calibration_ui(&self, ui: &mut egui::Ui, tool_id: &ToolId) {
        ui_chrome::section_label(ui, "Calibration Status");
        let records = self.model.calibration_records_for(tool_id);
        if records.is_empty() {
            ui_chrome::empty_state(ui, "No calibration records for this tool");
            return;
        }
        if let Some(record) = records.iter().max_by_key(|record| record.performed_at) {
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(
                    ui,
                    record.outcome.label(),
                    calibration_tone(record.outcome),
                );
                ui.label(format!("{} {}", record.performed_at, record.parameter));
            });
            ui.small(format!(
                "{} measured {:.2} against {}",
                record.instrument, record.measured_value, record.tolerance
            ));
            ui.small(format!("Technician {}", record.technician));
        }

        for record in records.into_iter().rev().take(3) {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(calibration_color(record.outcome), record.outcome.label());
                ui.small(format!(
                    "{} {} {:.2} ({})",
                    record.performed_at, record.parameter, record.measured_value, record.tolerance
                ));
            });
        }
    }

    fn qualification_ui(&self, ui: &mut egui::Ui, tool_id: &ToolId) {
        ui_chrome::section_label(ui, "Qualification");
        let results = self.model.qualification_results_for(tool_id);
        if results.is_empty() {
            ui_chrome::empty_state(ui, "No qualification wafers for this tool");
            return;
        }
        if let Some(result) = results.iter().max_by_key(|result| result.performed_at) {
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(
                    ui,
                    result.outcome.label(),
                    qualification_tone(result.outcome),
                );
                ui.label(format!("{} {}", result.performed_at, result.wafer_id));
            });
            ui.small(format!(
                "{} / {} {:.2} ({})",
                result.recipe_id, result.metric, result.value, result.spec
            ));
        }

        for result in results.into_iter().rev().take(3) {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(qualification_color(result.outcome), result.outcome.label());
                ui.small(format!(
                    "{} {} {:.2} ({})",
                    result.wafer_id, result.metric, result.value, result.spec
                ));
            });
        }
    }

    fn tool_history_ui(&self, ui: &mut egui::Ui, tool_id: &ToolId) {
        ui_chrome::section_label(ui, "Tool History");
        let mut rows = Vec::new();
        for record in self.model.downtime_for(tool_id) {
            rows.push(format!(
                "{} downtime: {} ({})",
                record.started_at, record.reason, record.owner
            ));
        }
        for part in self.model.spare_parts_for(tool_id) {
            rows.push(format!(
                "{} part: x{} {} ${:.0}",
                part.used_at, part.quantity, part.part_number, part.unit_cost
            ));
        }
        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No downtime or part history for this tool");
            return;
        }
        for row in rows.into_iter().take(5) {
            ui.small(row);
        }
    }

    fn audit_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Records / Audit");
        let today = self.today();
        if ui.available_width() < 780.0 {
            self.release_board_ui(ui, today);
            ui.separator();
            self.history_ui(ui);
        } else {
            ui.columns(2, |columns| {
                self.release_board_ui(&mut columns[0], today);
                self.history_ui(&mut columns[1]);
            });
        }
    }

    fn release_board_ui(&self, ui: &mut egui::Ui, today: FabDate) {
        ui_chrome::section_label(ui, "Release Board");
        if self.model.tools.is_empty() {
            ui_chrome::empty_state(ui, "No maintenance tools loaded");
            return;
        }

        for tool in &self.model.tools {
            let selected = self.selected_tool.as_ref() == Some(&tool.tool_id);
            let release = self.model.release_for_tool(&tool.tool_id, today);
            ui.group(|ui| {
                ui.set_width(ui.available_width().clamp(240.0, 480.0));
                ui.horizontal_wrapped(|ui| {
                    ui.label(if selected { ">" } else { " " });
                    ui.label(RichText::new(&tool.tool_name).strong());
                });
                ui.horizontal_wrapped(|ui| {
                    ui_chrome::status_pill(ui, release.state.label(), release_tone(release.state));
                    ui.small(format!("{} runs", tool.run_count));
                });
                if release.reasons.is_empty() {
                    ui.small("No release holds");
                } else {
                    for reason in release.reasons.iter().take(2) {
                        ui.small(reason);
                    }
                }
            });
        }
    }

    fn history_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Maintenance History");
        ui.horizontal_wrapped(|ui| {
            ui.label("Scope");
            for filter in HistoryFilter::ALL {
                ui.selectable_value(&mut self.history_filter, filter, filter.label());
            }
        });

        let entries = self.history_entries();
        if entries.is_empty() {
            ui_chrome::empty_state(ui, "No history entries match the scope");
            return;
        }

        if ui.available_width() < 560.0 {
            for entry in entries.into_iter().take(10) {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(240.0, 480.0));
                    ui.horizontal_wrapped(|ui| {
                        ui_chrome::status_pill(ui, entry.kind, entry.tone);
                        ui.label(entry.date.to_string());
                    });
                    ui.label(RichText::new(entry.label).strong());
                    ui.small(format!("{} - {}", entry.tool_id, entry.detail));
                });
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("maintenance_history_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("maintenance_history_grid")
                    .striped(true)
                    .min_col_width(88.0)
                    .show(ui, |ui| {
                        ui.strong("Date");
                        ui.strong("Type");
                        ui.strong("Tool");
                        ui.strong("Record");
                        ui.end_row();
                        for entry in entries.into_iter().take(14) {
                            ui.label(entry.date.to_string());
                            ui.colored_label(entry.tone.color(), entry.kind);
                            ui.label(entry.tool_id.to_string());
                            ui.label(format!("{} - {}", entry.label, entry.detail));
                            ui.end_row();
                        }
                    });
            });
    }

    fn work_items(&self, today: FabDate) -> Vec<WorkItem> {
        let mut items = Vec::new();
        for tool in &self.model.tools {
            let release = self.model.release_for_tool(&tool.tool_id, today);
            let blocked = !release.state.released_to_production();
            for schedule in &tool.schedules {
                let due_state = schedule.due_state(today, tool.run_count);
                let run_count_delta = schedule.interval_runs.map(|_| {
                    tool.run_count
                        .saturating_sub(schedule.last_completed_run_count)
                });
                items.push(WorkItem {
                    tool_id: tool.tool_id.clone(),
                    tool_name: tool.tool_name.clone(),
                    schedule_id: schedule.id.clone(),
                    task: schedule.task.clone(),
                    kind: schedule.kind,
                    due_state,
                    due_date: schedule.next_due,
                    days_until: today.days_until(schedule.next_due),
                    run_count_delta,
                    interval_runs: schedule.interval_runs,
                    release_state: release.state,
                    blocked,
                    checklist_len: schedule.checklist.len(),
                    checklist: schedule.checklist.clone(),
                });
            }
        }
        items.sort_by_key(|work| {
            (
                !work.blocked,
                due_state_priority(work.due_state),
                work.days_until,
                work.tool_id.clone(),
                work.task.clone(),
            )
        });
        items
    }

    fn filtered_work_items(&self, today: FabDate) -> Vec<WorkItem> {
        self.work_items(today)
            .into_iter()
            .filter(|work| match self.work_filter {
                WorkFilter::Actionable => work.blocked || work.due_state.blocks_release(),
                WorkFilter::Upcoming => {
                    work.due_state != DueState::Overdue && work.days_until <= 14
                }
                WorkFilter::Calibration => {
                    work.kind == MaintenanceKind::Calibration
                        || work.release_state == ToolReleaseState::CalibrationLockout
                }
                WorkFilter::Locked => work.blocked,
                WorkFilter::All => true,
            })
            .collect()
    }

    fn next_schedule_for_selected(&self, today: FabDate) -> Option<WorkItem> {
        let tool_id = self.selected_tool.as_ref()?;
        self.next_schedule_for_tool(tool_id, today)
    }

    fn next_schedule_for_tool(&self, tool_id: &ToolId, today: FabDate) -> Option<WorkItem> {
        self.work_items(today)
            .into_iter()
            .filter(|work| &work.tool_id == tool_id)
            .min_by_key(|work| {
                (
                    due_state_priority(work.due_state),
                    work.days_until,
                    work.task.clone(),
                )
            })
    }

    fn history_entries(&self) -> Vec<HistoryEntry> {
        let selected_tool = self.selected_tool.as_ref();
        let include_tool = |tool_id: &ToolId| match self.history_filter {
            HistoryFilter::SelectedTool => selected_tool == Some(tool_id),
            HistoryFilter::AllTools => true,
        };

        let mut entries = Vec::new();
        for record in &self.model.calibration_records {
            if include_tool(&record.tool_id) {
                entries.push(HistoryEntry {
                    date: record.performed_at,
                    kind: "Calibration",
                    tool_id: record.tool_id.clone(),
                    label: format!("{} {}", record.parameter, record.outcome.label()),
                    detail: format!(
                        "{} {:.2} vs {}",
                        record.instrument, record.measured_value, record.tolerance
                    ),
                    tone: calibration_tone(record.outcome),
                });
            }
        }
        for result in &self.model.qualification_results {
            if include_tool(&result.tool_id) {
                entries.push(HistoryEntry {
                    date: result.performed_at,
                    kind: "Qualification",
                    tool_id: result.tool_id.clone(),
                    label: format!("{} {}", result.wafer_id, result.outcome.label()),
                    detail: format!("{} {} {:.2}", result.recipe_id, result.metric, result.value),
                    tone: qualification_tone(result.outcome),
                });
            }
        }
        for record in &self.model.downtime {
            if include_tool(&record.tool_id) {
                entries.push(HistoryEntry {
                    date: record.started_at,
                    kind: "Downtime",
                    tool_id: record.tool_id.clone(),
                    label: record.reason.clone(),
                    detail: format!(
                        "{} to {} / {}",
                        record.started_at,
                        record
                            .ended_at
                            .map(|date| date.to_string())
                            .unwrap_or_else(|| "open".to_string()),
                        record.owner
                    ),
                    tone: if record.ended_at.is_none() {
                        Tone::Danger
                    } else {
                        Tone::Neutral
                    },
                });
            }
        }
        for part in &self.model.spare_parts {
            if include_tool(&part.tool_id) {
                entries.push(HistoryEntry {
                    date: part.used_at,
                    kind: "Part",
                    tool_id: part.tool_id.clone(),
                    label: format!("x{} {}", part.quantity, part.part_number),
                    detail: format!("{} ${:.0}", part.description, part.unit_cost),
                    tone: Tone::Info,
                });
            }
        }
        entries.sort_by_key(|entry| (entry.date, entry.tool_id.clone(), entry.label.clone()));
        entries.reverse();
        entries
    }

    fn release_reasons_ui(&self, ui: &mut egui::Ui, reasons: &[String]) {
        if reasons.is_empty() {
            ui.small("Release context: no active holds");
            return;
        }
        for reason in reasons {
            ui.small(format!("Hold: {reason}"));
        }
    }

    fn latest_calibration(&self, tool_id: &ToolId) -> Option<&CalibrationRecord> {
        self.model
            .calibration_records_for(tool_id)
            .into_iter()
            .max_by_key(|record| record.performed_at)
    }

    fn open_downtime_count(&self, tool_id: &ToolId) -> usize {
        self.model
            .downtime
            .iter()
            .filter(|record| &record.tool_id == tool_id && record.ended_at.is_none())
            .count()
    }

    fn locked_tool_count(&self, today: FabDate) -> usize {
        self.model
            .tools
            .iter()
            .filter(|tool| {
                !self
                    .model
                    .release_for_tool(&tool.tool_id, today)
                    .state
                    .released_to_production()
            })
            .count()
    }

    fn ensure_selection(&mut self) {
        let selection_exists = self
            .selected_tool
            .as_ref()
            .is_some_and(|id| self.model.tool(id).is_some());
        if !selection_exists {
            self.selected_tool = self.model.tools.first().map(|tool| tool.tool_id.clone());
        }
    }

    fn today(&self) -> FabDate {
        self.model.today.unwrap_or(FabDate::new(2026, 5, 8))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkFilter {
    Actionable,
    Upcoming,
    Calibration,
    Locked,
    All,
}

impl WorkFilter {
    const ALL: [Self; 5] = [
        Self::Actionable,
        Self::Upcoming,
        Self::Calibration,
        Self::Locked,
        Self::All,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Actionable => "Action required",
            Self::Upcoming => "Upcoming",
            Self::Calibration => "Calibration",
            Self::Locked => "Locked tools",
            Self::All => "All tasks",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryFilter {
    SelectedTool,
    AllTools,
}

impl HistoryFilter {
    const ALL: [Self; 2] = [Self::SelectedTool, Self::AllTools];

    fn label(self) -> &'static str {
        match self {
            Self::SelectedTool => "Selected tool",
            Self::AllTools => "All tools",
        }
    }
}

struct WorkItem {
    tool_id: ToolId,
    tool_name: String,
    schedule_id: String,
    task: String,
    kind: MaintenanceKind,
    due_state: DueState,
    due_date: FabDate,
    days_until: i32,
    run_count_delta: Option<u32>,
    interval_runs: Option<u32>,
    release_state: ToolReleaseState,
    blocked: bool,
    checklist_len: usize,
    checklist: Vec<String>,
}

struct HistoryEntry {
    date: FabDate,
    kind: &'static str,
    tool_id: ToolId,
    label: String,
    detail: String,
    tone: Tone,
}

fn due_state_priority(state: DueState) -> u8 {
    match state {
        DueState::Overdue => 0,
        DueState::Due => 1,
        DueState::NotDue => 2,
    }
}

fn run_progress(current: u32, target: u32) -> f32 {
    if target == 0 {
        1.0
    } else {
        (current as f32 / target as f32).clamp(0.0, 1.0)
    }
}

fn due_window_label(days_until: i32) -> String {
    if days_until < 0 {
        format!("{} days overdue", days_until.abs())
    } else if days_until == 0 {
        "today".to_string()
    } else if days_until == 1 {
        "tomorrow".to_string()
    } else {
        format!("in {days_until} days")
    }
}

fn due_state_tone(state: DueState) -> Tone {
    match state {
        DueState::NotDue => Tone::Success,
        DueState::Due => Tone::Warning,
        DueState::Overdue => Tone::Danger,
    }
}

fn kind_tone(kind: MaintenanceKind) -> Tone {
    match kind {
        MaintenanceKind::PreventiveMaintenance => Tone::Info,
        MaintenanceKind::ChamberClean => Tone::Warning,
        MaintenanceKind::Calibration => Tone::Warning,
        MaintenanceKind::Qualification => Tone::Info,
        MaintenanceKind::Downtime => Tone::Danger,
        MaintenanceKind::SparePart => Tone::Neutral,
    }
}

fn release_tone(state: ToolReleaseState) -> Tone {
    match state {
        ToolReleaseState::Released => Tone::Success,
        ToolReleaseState::DueSoon => Tone::Warning,
        ToolReleaseState::MaintenanceRequired => Tone::Danger,
        ToolReleaseState::CalibrationLockout => Tone::Danger,
        ToolReleaseState::QualificationLockout => Tone::Danger,
        ToolReleaseState::Down => Tone::Danger,
    }
}

fn calibration_tone(outcome: CalibrationOutcome) -> Tone {
    match outcome {
        CalibrationOutcome::Passed => Tone::Success,
        CalibrationOutcome::Failed => Tone::Danger,
        CalibrationOutcome::Conditional => Tone::Warning,
    }
}

fn qualification_tone(outcome: QualificationOutcome) -> Tone {
    match outcome {
        QualificationOutcome::Passed => Tone::Success,
        QualificationOutcome::Failed => Tone::Danger,
        QualificationOutcome::PendingReview => Tone::Warning,
    }
}

fn calibration_color(outcome: CalibrationOutcome) -> Color32 {
    calibration_tone(outcome).color()
}

fn qualification_color(outcome: QualificationOutcome) -> Color32 {
    qualification_tone(outcome).color()
}
