use eframe::egui::{self, Color32, RichText};
use layout_model::{
    equipment::ToolId,
    maintenance::{
        CalibrationOutcome, DueState, FabDate, MaintenanceModel, QualificationOutcome,
        ToolReleaseState,
    },
};

use crate::ui_chrome;

pub(crate) struct MaintenancePanel {
    model: MaintenanceModel,
    selected_tool: Option<ToolId>,
}

impl MaintenancePanel {
    pub(crate) fn from_model(model: MaintenanceModel) -> Self {
        let selected_tool = model.tools.first().map(|tool| tool.tool_id.clone());
        Self {
            model,
            selected_tool,
        }
    }

    pub(crate) fn model(&self) -> &MaintenanceModel {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Maintenance");
        let today = self.today();
        let due = self.model.due_work(today);
        ui.label(format!("Audit date: {today}"));
        ui.label(format!("Due work: {}", due.len()));
        ui.label(format!(
            "Locked tools: {}",
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
        ));

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
        ui.colored_label(release_state_color(release.state), release.state.label());
        for reason in &release.reasons {
            ui.small(reason);
        }

        ui.separator();
        ui_chrome::section_label(ui, "Checklist");
        if let Some(next) = tool
            .schedules
            .iter()
            .min_by_key(|schedule| schedule.next_due)
        {
            ui.label(format!("{} due {}", next.task, next.next_due));
            for item in &next.checklist {
                ui.label(format!("- {item}"));
            }
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let today = self.today();
        egui::ScrollArea::vertical()
            .id_salt("maintenance_dashboard_scroll")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Fab operations",
                    "Maintenance and Calibration",
                    "",
                    |ui| {
                        ui.label("Tool");
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
                                    ui.selectable_value(
                                        &mut selected,
                                        Some(tool.tool_id.clone()),
                                        &tool.tool_name,
                                    );
                                }
                            });
                        if selected != self.selected_tool {
                            self.selected_tool = selected;
                            *status = "maintenance tool selected".to_string();
                        }
                    },
                );

                if self.model.tools.is_empty() {
                    ui_chrome::empty_state(ui, "No maintenance model loaded");
                    return;
                }

                self.metric_row(ui, today);
                ui.separator();
                ui.columns(2, |columns| {
                    self.due_work_ui(&mut columns[0], today);
                    self.selected_tool_ui(&mut columns[1], today);
                });

                ui.separator();
                self.audit_ui(ui);
            });
    }

    fn metric_row(&self, ui: &mut egui::Ui, today: FabDate) {
        let due = self.model.due_work(today);
        let overdue = due
            .iter()
            .filter(|work| work.due_state == DueState::Overdue)
            .count();
        let due_today = due
            .iter()
            .filter(|work| work.due_state == DueState::Due)
            .count();
        let locked = self
            .model
            .tools
            .iter()
            .filter(|tool| {
                !self
                    .model
                    .release_for_tool(&tool.tool_id, today)
                    .state
                    .released_to_production()
            })
            .count();
        let open_downtime = self
            .model
            .downtime
            .iter()
            .filter(|record| record.ended_at.is_none())
            .count();

        ui.horizontal_wrapped(|ui| {
            ui_chrome::metric_tile_tone(
                ui,
                "Overdue",
                overdue,
                "blocks release",
                if overdue > 0 {
                    ui_chrome::Tone::Danger
                } else {
                    ui_chrome::Tone::Neutral
                },
            );
            ui_chrome::metric_tile_tone(
                ui,
                "Due now",
                due_today,
                "calendar or run count",
                ui_chrome::Tone::Warning,
            );
            ui_chrome::metric_tile_tone(
                ui,
                "Locked",
                locked,
                "not production released",
                ui_chrome::Tone::Danger,
            );
            ui_chrome::metric_tile(ui, "Open downtime", open_downtime, "active events");
        });
    }

    fn due_work_ui(&mut self, ui: &mut egui::Ui, today: FabDate) {
        ui_chrome::section_label(ui, "Due Work");
        let due = self.model.due_work(today);
        if due.is_empty() {
            ui_chrome::empty_state(ui, "No due maintenance or calibration work");
            return;
        }
        for work in due {
            let selected = self.selected_tool.as_ref() == Some(&work.tool_id);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(selected, work.tool_id.to_string())
                        .clicked()
                    {
                        self.selected_tool = Some(work.tool_id.clone());
                    }
                    ui.colored_label(due_state_color(work.due_state), work.due_state.label());
                    ui.small(work.kind.label());
                });
                ui.label(RichText::new(&work.task).strong());
                ui.label(format!("Due {}", work.due_date));
                if let Some(delta) = work.run_count_delta {
                    ui.small(format!("{delta} runs since last completion"));
                }
            });
        }
    }

    fn selected_tool_ui(&self, ui: &mut egui::Ui, today: FabDate) {
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
            ui.colored_label(release_state_color(release.state), release.state.label());
            ui.label(format!("{} runs", tool.run_count));
        });
        for reason in &release.reasons {
            ui.small(reason);
        }

        ui.separator();
        ui_chrome::section_label(ui, "Schedules");
        for schedule in &tool.schedules {
            let due_state = schedule.due_state(today, tool.run_count);
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(due_state_color(due_state), due_state.label());
                ui.label(format!("{} due {}", schedule.task, schedule.next_due));
            });
        }

        ui.separator();
        ui_chrome::section_label(ui, "Calibration Status");
        for record in self
            .model
            .calibration_records_for(tool_id)
            .into_iter()
            .rev()
            .take(4)
        {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(calibration_color(record.outcome), record.outcome.label());
                ui.label(format!(
                    "{} {} {} ({})",
                    record.performed_at, record.parameter, record.measured_value, record.tolerance
                ));
            });
            ui.small(format!("{} / {}", record.instrument, record.technician));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Qualification");
        for result in self
            .model
            .qualification_results_for(tool_id)
            .into_iter()
            .rev()
            .take(4)
        {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(qualification_color(result.outcome), result.outcome.label());
                ui.label(format!(
                    "{} {} {} ({})",
                    result.wafer_id, result.metric, result.value, result.spec
                ));
            });
            ui.small(format!("{} / {}", result.performed_at, result.recipe_id));
        }
    }

    fn audit_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Records / Audit");
        ui.columns(3, |columns| {
            ui_chrome::section_label(&mut columns[0], "Downtime");
            for record in &self.model.downtime {
                columns[0].label(format!("{} {}", record.tool_id, record.reason));
                columns[0].small(format!(
                    "{} to {}",
                    record.started_at,
                    record
                        .ended_at
                        .map(|date| date.to_string())
                        .unwrap_or_else(|| "open".to_string())
                ));
            }

            ui_chrome::section_label(&mut columns[1], "Spare Parts");
            for part in &self.model.spare_parts {
                columns[1].label(format!(
                    "{} x{} {}",
                    part.tool_id, part.quantity, part.part_number
                ));
                columns[1].small(format!("{} ${:.0}", part.description, part.unit_cost));
            }

            ui_chrome::section_label(&mut columns[2], "Release");
            let today = self.today();
            for tool in &self.model.tools {
                let release = self.model.release_for_tool(&tool.tool_id, today);
                columns[2].horizontal_wrapped(|ui| {
                    ui.colored_label(release_state_color(release.state), release.state.label());
                    ui.label(&tool.tool_id.0);
                });
            }
        });
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

fn due_state_color(state: DueState) -> Color32 {
    match state {
        DueState::NotDue => Color32::from_rgb(92, 170, 118),
        DueState::Due => Color32::from_rgb(212, 156, 64),
        DueState::Overdue => Color32::from_rgb(220, 76, 72),
    }
}

fn release_state_color(state: ToolReleaseState) -> Color32 {
    match state {
        ToolReleaseState::Released => Color32::from_rgb(92, 170, 118),
        ToolReleaseState::DueSoon => Color32::from_rgb(212, 156, 64),
        ToolReleaseState::MaintenanceRequired => Color32::from_rgb(220, 76, 72),
        ToolReleaseState::CalibrationLockout => Color32::from_rgb(220, 76, 72),
        ToolReleaseState::QualificationLockout => Color32::from_rgb(220, 76, 72),
        ToolReleaseState::Down => Color32::from_rgb(180, 92, 205),
    }
}

fn calibration_color(outcome: CalibrationOutcome) -> Color32 {
    match outcome {
        CalibrationOutcome::Passed => Color32::from_rgb(92, 170, 118),
        CalibrationOutcome::Failed => Color32::from_rgb(220, 76, 72),
        CalibrationOutcome::Conditional => Color32::from_rgb(212, 156, 64),
    }
}

fn qualification_color(outcome: QualificationOutcome) -> Color32 {
    match outcome {
        QualificationOutcome::Passed => Color32::from_rgb(92, 170, 118),
        QualificationOutcome::Failed => Color32::from_rgb(220, 76, 72),
        QualificationOutcome::PendingReview => Color32::from_rgb(212, 156, 64),
    }
}
