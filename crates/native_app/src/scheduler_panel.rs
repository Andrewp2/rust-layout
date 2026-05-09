use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, vec2};
use layout_model::scheduler::{
    DispatchAssignment, DispatchPolicy, DispatchSchedule, QueueSummary, format_shift_time,
};
use layout_model::{mes::ToolId, scheduler::ToolRecommendation};

use crate::ui_chrome::{self, Tone};

pub(crate) struct SchedulerPanel {
    schedule: DispatchSchedule,
    policy: DispatchPolicy,
    selected_tool: Option<ToolId>,
}

impl SchedulerPanel {
    pub(crate) fn empty() -> Self {
        Self::from_schedule(DispatchSchedule::default())
    }

    pub(crate) fn from_schedule(schedule: DispatchSchedule) -> Self {
        let selected_tool = schedule.tools.first().map(|tool| tool.id.clone());
        Self {
            schedule,
            policy: DispatchPolicy::PriorityThenFifo,
            selected_tool,
        }
    }

    pub(crate) fn schedule(&self) -> &DispatchSchedule {
        &self.schedule
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let result = self.schedule.dispatch(self.policy);
        self.schedule.assignments = result.assignments.clone();

        egui::ScrollArea::vertical()
            .id_salt("scheduler_dispatch_dashboard")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Fab operations",
                    "Scheduler / Dispatch",
                    &format!("Policy: {}", self.policy.label()),
                    |ui| {
                        egui::ComboBox::from_id_salt("dispatch_policy")
                            .selected_text(self.policy.label())
                            .show_ui(ui, |ui| {
                                for policy in [
                                    DispatchPolicy::PriorityThenFifo,
                                    DispatchPolicy::DueDateThenPriority,
                                    DispatchPolicy::Fifo,
                                ] {
                                    ui.selectable_value(&mut self.policy, policy, policy.label());
                                }
                            });
                    },
                );

                if self.schedule.is_empty() {
                    ui_chrome::empty_state(ui, "No scheduler data loaded");
                    let _ = status;
                    return;
                }

                self.summary_ui(ui, &result.queue_summaries, result.unscheduled_lots.len());
                ui.separator();
                self.recommendations_ui(ui, &result.recommendations, status);
                ui.separator();
                self.timeline_ui(ui, &result.assignments);
                ui.separator();
                self.queue_ui(ui, &result.queue_summaries);
                ui.separator();
                self.assignment_table_ui(ui, &result.assignments);
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Dispatch Control");
        if self.schedule.is_empty() {
            ui_chrome::empty_state(ui, "No scheduler data loaded");
            return;
        }

        ui.label(format!(
            "Shift clock: {}",
            format_shift_time(self.schedule.now_minute)
        ));
        ui.label(format!("Waiting lots: {}", self.schedule.lots.len()));
        ui.label(format!("Tools: {}", self.schedule.tools.len()));

        if let Some(bottleneck) = self.schedule.bottleneck_queue() {
            ui.separator();
            ui_chrome::section_label(ui, "Bottleneck");
            queue_summary_ui(ui, &bottleneck);
        }

        ui.separator();
        ui_chrome::section_label(ui, "Tool Focus");
        let selected_label = self
            .selected_tool
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string());
        egui::ComboBox::from_id_salt("scheduler_selected_tool")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                for tool in &self.schedule.tools {
                    ui.selectable_value(
                        &mut self.selected_tool,
                        Some(tool.id.clone()),
                        tool.id.to_string(),
                    );
                }
            });

        if let Some(tool) = self.selected_tool_detail() {
            ui.group(|ui| {
                ui.strong(&tool.name);
                ui.small(format!("{} / {}", tool.id, tool.class.label()));
                ui.label(format!("State: {}", tool.state.label()));
                ui.label(format!(
                    "Utilization: {}%",
                    self.schedule.utilization_percent(&tool.id)
                ));
                if tool.maintenance_windows.is_empty() {
                    ui.small("No maintenance windows");
                } else {
                    for window in &tool.maintenance_windows {
                        ui.small(format!(
                            "{}-{} {}",
                            format_shift_time(window.start_minute),
                            format_shift_time(window.end_minute),
                            window.reason
                        ));
                    }
                }
            });
        }
    }

    fn summary_ui(
        &self,
        ui: &mut egui::Ui,
        queue_summaries: &[QueueSummary],
        unscheduled_lots: usize,
    ) {
        let scheduled = self.schedule.dispatch(self.policy).assignments.len();
        let bottleneck = queue_summaries
            .iter()
            .max_by_key(|summary| summary.total_process_minutes);
        let bottleneck_detail = bottleneck
            .map(|summary| format!("{} lots", summary.waiting_lots))
            .unwrap_or_default();
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Recommended",
                    scheduled.to_string(),
                    &format!("{} unscheduled", unscheduled_lots),
                    Tone::Neutral,
                ),
                (
                    "Queue load",
                    format!("{} min", total_queue_minutes(queue_summaries)),
                    "waiting process time",
                    Tone::Neutral,
                ),
                (
                    "Bottleneck",
                    bottleneck
                        .map(|summary| summary.tool_class.label().to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    bottleneck_detail.as_str(),
                    Tone::Warning,
                ),
            ],
        );
    }

    fn recommendations_ui(
        &mut self,
        ui: &mut egui::Ui,
        recommendations: &[ToolRecommendation],
        status: &mut String,
    ) {
        ui_chrome::section_label(ui, "Recommended Next Lot");
        if ui.available_width() < 520.0 {
            for recommendation in recommendations {
                let selected = self.selected_tool.as_ref() == Some(&recommendation.tool_id);
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(selected, recommendation.tool_id.to_string())
                            .clicked()
                        {
                            self.selected_tool = Some(recommendation.tool_id.clone());
                            *status = format!("selected dispatch tool {}", recommendation.tool_id);
                        }
                        ui.label(
                            recommendation
                                .lot_id
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "--".to_string()),
                        );
                    });
                    ui.small(format!(
                        "{} to {}",
                        format_optional_time(recommendation.start_minute),
                        format_optional_time(recommendation.finish_minute)
                    ));
                    ui.add(egui::Label::new(&recommendation.reason).wrap());
                });
            }
            return;
        }
        egui::Grid::new("scheduler_recommendations")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Tool");
                ui.strong("Lot");
                ui.strong("Start");
                ui.strong("Finish");
                ui.strong("Reason");
                ui.end_row();
                for recommendation in recommendations {
                    let selected = self.selected_tool.as_ref() == Some(&recommendation.tool_id);
                    if ui
                        .selectable_label(selected, recommendation.tool_id.to_string())
                        .clicked()
                    {
                        self.selected_tool = Some(recommendation.tool_id.clone());
                        *status = format!("selected dispatch tool {}", recommendation.tool_id);
                    }
                    ui.label(
                        recommendation
                            .lot_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "--".to_string()),
                    );
                    ui.label(format_optional_time(recommendation.start_minute));
                    ui.label(format_optional_time(recommendation.finish_minute));
                    ui.label(&recommendation.reason);
                    ui.end_row();
                }
            });
    }

    fn timeline_ui(&self, ui: &mut egui::Ui, assignments: &[DispatchAssignment]) {
        ui_chrome::section_label(ui, "Dispatch Timeline");
        let height = (self.schedule.tools.len().max(1) as f32 * 34.0 + 34.0).clamp(120.0, 320.0);
        let size = vec2(ui.available_width().max(320.0), height);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        ui_chrome::plot_background(ui, rect);

        if assignments.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No assignments",
                egui::FontId::proportional(13.0),
                ui.visuals().weak_text_color(),
            );
            return;
        }

        let start = assignments
            .iter()
            .map(|assignment| assignment.start_minute)
            .min()
            .unwrap_or(self.schedule.now_minute)
            .min(self.schedule.now_minute);
        let end = assignments
            .iter()
            .map(|assignment| assignment.finish_minute)
            .max()
            .unwrap_or(start + 60)
            .max(start + 60);
        let left_pad = 82.0;
        let top_pad = 24.0;
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + left_pad, rect.top() + top_pad),
            Pos2::new(rect.right() - 8.0, rect.bottom() - 10.0),
        );

        for tick in hourly_ticks(start, end) {
            let x = time_x(plot, start, end, tick);
            painter.line_segment(
                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
            );
            painter.text(
                Pos2::new(x + 2.0, rect.top() + 6.0),
                egui::Align2::LEFT_CENTER,
                format_shift_time(tick),
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
            );
        }

        let row_h = (plot.height() / self.schedule.tools.len().max(1) as f32).max(24.0);
        for (index, tool) in self.schedule.tools.iter().enumerate() {
            let y = plot.top() + index as f32 * row_h;
            let row_rect = Rect::from_min_size(
                Pos2::new(plot.left(), y + 3.0),
                Vec2::new(plot.width(), row_h - 6.0),
            );
            painter.text(
                Pos2::new(rect.left() + 8.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                tool.id.to_string(),
                egui::FontId::proportional(12.0),
                ui.visuals().text_color(),
            );
            painter.line_segment(
                [
                    Pos2::new(plot.left(), row_rect.bottom()),
                    Pos2::new(plot.right(), row_rect.bottom()),
                ],
                Stroke::new(1.0, ui.visuals().faint_bg_color),
            );

            for window in &tool.maintenance_windows {
                if window.end_minute < start || window.start_minute > end {
                    continue;
                }
                let x0 = time_x(plot, start, end, window.start_minute.max(start));
                let x1 = time_x(plot, start, end, window.end_minute.min(end));
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x0, row_rect.top()),
                        Pos2::new(x1.max(x0 + 2.0), row_rect.bottom()),
                    ),
                    3.0,
                    Color32::from_rgba_unmultiplied(220, 176, 72, 55),
                );
            }
        }

        for assignment in assignments {
            let Some(row) = self
                .schedule
                .tools
                .iter()
                .position(|tool| tool.id == assignment.tool_id)
            else {
                continue;
            };
            let y = plot.top() + row as f32 * row_h;
            let x0 = time_x(plot, start, end, assignment.start_minute);
            let x1 = time_x(plot, start, end, assignment.finish_minute);
            let bar = Rect::from_min_max(
                Pos2::new(x0, y + 7.0),
                Pos2::new(x1.max(x0 + 8.0), y + row_h - 7.0),
            );
            painter.rect_filled(bar, 4.0, assignment_color(assignment.priority));
            painter.rect_stroke(
                bar,
                4.0,
                Stroke::new(1.0, Color32::from_black_alpha(80)),
                StrokeKind::Inside,
            );
            painter.text(
                bar.center(),
                egui::Align2::CENTER_CENTER,
                assignment.lot_id.to_string(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }

    fn queue_ui(&self, ui: &mut egui::Ui, queue_summaries: &[QueueSummary]) {
        ui_chrome::section_label(ui, "Bottleneck / Queue View");
        egui::Grid::new("scheduler_queue_summary")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Class");
                ui.strong("Lots");
                ui.strong("Load");
                ui.strong("Earliest due");
                ui.end_row();
                for summary in queue_summaries {
                    ui.label(summary.tool_class.label());
                    ui.label(summary.waiting_lots.to_string());
                    ui.label(format!("{} min", summary.total_process_minutes));
                    ui.label(
                        summary
                            .earliest_due_minute
                            .map(format_shift_time)
                            .unwrap_or_else(|| "--".to_string()),
                    );
                    ui.end_row();
                }
            });
    }

    fn assignment_table_ui(&self, ui: &mut egui::Ui, assignments: &[DispatchAssignment]) {
        ui_chrome::section_label(ui, "Estimated Completion");
        egui::Grid::new("scheduler_assignment_table")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Lot");
                ui.strong("Tool");
                ui.strong("Start");
                ui.strong("Finish");
                ui.strong("Wait");
                ui.strong("Due");
                ui.end_row();
                for assignment in assignments {
                    ui.label(assignment.lot_id.to_string());
                    ui.label(assignment.tool_id.to_string());
                    ui.label(format_shift_time(assignment.start_minute));
                    let finish = RichText::new(format_shift_time(assignment.finish_minute)).color(
                        if assignment.tardy_minutes > 0 {
                            Tone::Danger.color()
                        } else {
                            ui.visuals().text_color()
                        },
                    );
                    ui.label(finish);
                    ui.label(format!("{} min", assignment.wait_minutes));
                    ui.label(format_shift_time(assignment.due_at_minute));
                    ui.end_row();
                }
            });
    }

    fn ensure_selection(&mut self) {
        if self.selected_tool.is_none() {
            self.selected_tool = self.schedule.tools.first().map(|tool| tool.id.clone());
        }
    }

    fn selected_tool_detail(&self) -> Option<&layout_model::scheduler::DispatchTool> {
        let selected = self.selected_tool.as_ref()?;
        self.schedule.tools.iter().find(|tool| tool.id == *selected)
    }
}

fn queue_summary_ui(ui: &mut egui::Ui, summary: &QueueSummary) {
    ui.label(summary.tool_class.label());
    ui.label(format!("{} waiting lots", summary.waiting_lots));
    ui.label(format!("{} min queued", summary.total_process_minutes));
    if let Some(due) = summary.earliest_due_minute {
        ui.label(format!("Earliest due {}", format_shift_time(due)));
    }
}

fn total_queue_minutes(queue_summaries: &[QueueSummary]) -> u32 {
    queue_summaries
        .iter()
        .map(|summary| summary.total_process_minutes)
        .sum()
}

fn format_optional_time(minute: Option<u32>) -> String {
    minute
        .map(format_shift_time)
        .unwrap_or_else(|| "--".to_string())
}

fn hourly_ticks(start: u32, end: u32) -> Vec<u32> {
    let first = ((start + 59) / 60) * 60;
    let mut ticks = Vec::new();
    let mut tick = first;
    while tick <= end {
        ticks.push(tick);
        tick += 60;
    }
    ticks
}

fn time_x(plot: Rect, start: u32, end: u32, minute: u32) -> f32 {
    let span = end.saturating_sub(start).max(1) as f32;
    let offset = minute.saturating_sub(start) as f32;
    plot.left() + plot.width() * (offset / span).clamp(0.0, 1.0)
}

fn assignment_color(priority: u8) -> Color32 {
    match priority {
        5..=u8::MAX => Color32::from_rgb(209, 88, 88),
        4 => Color32::from_rgb(218, 150, 66),
        3 => Color32::from_rgb(82, 145, 214),
        _ => Color32::from_rgb(92, 168, 132),
    }
}
