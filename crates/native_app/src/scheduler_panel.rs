use std::collections::BTreeMap;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, vec2};
use layout_model::mes::{LotId, ToolClass, ToolId};
use layout_model::scheduler::{
    DispatchAssignment, DispatchLot, DispatchPolicy, DispatchResult, DispatchSchedule,
    DispatchTool, QueueSummary, ToolDispatchState, ToolRecommendation, format_shift_time,
};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{
    operad_egui,
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar_interactive},
    ui_chrome::{self, Tone},
};

const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SET_POLICY: &str = "scheduler.action.set_policy.";
const OPERAD_ACTION_SELECT_TOOL: &str = "scheduler.action.select_tool.";
const OPERAD_ACTION_SET_MIN_PRIORITY: &str = "scheduler.action.set_min_priority.";
const OPERAD_ACTION_TOGGLE_CONFLICTS: &str = "scheduler.action.toggle_conflicts";
const OPERAD_ACTION_TOGGLE_FOCUS_TOOL: &str = "scheduler.action.toggle_focus_tool";
const OPERAD_ACTION_RESET_FILTERS: &str = "scheduler.action.reset_filters";

pub(crate) struct SchedulerPanel {
    schedule: DispatchSchedule,
    policy: DispatchPolicy,
    selected_tool: Option<ToolId>,
    filter_text: String,
    min_priority: u8,
    show_conflicts_only: bool,
    focus_selected_tool: bool,
}

#[derive(Debug)]
struct SchedulerOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct SchedulerMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct SchedulerOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

impl SchedulerPanel {
    pub(crate) fn from_schedule(schedule: DispatchSchedule) -> Self {
        let selected_tool = schedule.tools.first().map(|tool| tool.id.clone());
        Self {
            schedule,
            policy: DispatchPolicy::PriorityThenFifo,
            selected_tool,
            filter_text: String::new(),
            min_priority: 0,
            show_conflicts_only: false,
            focus_selected_tool: false,
        }
    }

    pub(crate) fn schedule(&self) -> &DispatchSchedule {
        &self.schedule
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let result = self.schedule.dispatch(self.policy);
        self.schedule.assignments = result.assignments.clone();

        if let Err(error) = self.operad_ui(ui, &result, status) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, &result, status);
        }
    }

    fn operad_ui(
        &mut self,
        ui: &mut egui::Ui,
        result: &DispatchResult,
        status: &mut String,
    ) -> Result<(), String> {
        let mut action_status = None;
        let mut result_state = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("scheduler_dispatch_dashboard_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width, result);
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
                    let updated_result = self.schedule.dispatch(self.policy);
                    self.schedule.assignments = updated_result.assignments.clone();
                    view = self.build_operad_view(width, &updated_result);
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

    fn egui_dashboard_ui(
        &mut self,
        ui: &mut egui::Ui,
        result: &DispatchResult,
        status: &mut String,
    ) {
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

                self.summary_ui(ui, result);
                ui.separator();
                self.filters_ui(ui);
                ui.separator();
                self.timeline_ui(ui, &result.assignments);
                ui.separator();
                self.next_actions_ui(ui, result, status);
                ui.separator();
                self.queue_ui(ui, result);
                ui.separator();
                self.tool_match_ui(ui, result, status);
                ui.separator();
                self.queue_priority_ui(ui, result, status);
                ui.separator();
                self.conflicts_ui(ui, result);
                ui.separator();
                self.assignment_table_ui(ui, &result.assignments, status);
            });
    }

    fn build_operad_view(&self, width: f32, result: &DispatchResult) -> SchedulerOperadView {
        let metrics = self.operad_metrics(result);
        let control_rows = self.operad_control_rows();
        let next_action_rows = self.operad_next_action_rows(result);
        let queue_rows = self.operad_queue_rows(result);
        let tool_rows = self.operad_tool_rows(result);
        let priority_rows = self.operad_priority_rows(result);
        let conflict_rows = self.operad_conflict_rows(result);
        let assignment_rows = self.operad_assignment_rows(result);
        let height = scheduler_operad_view_height(
            width,
            metrics.len(),
            &[
                control_rows.len(),
                next_action_rows.len(),
                queue_rows.len(),
                tool_rows.len(),
                priority_rows.len(),
                conflict_rows.len(),
                assignment_rows.len(),
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

        add_scheduler_operad_header(
            &mut document,
            root,
            "FAB OPERATIONS",
            "Scheduler / Dispatch",
            "Dispatch policy, bottlenecks, tool matching, queue risk, and estimated completion",
            &format!(
                "{} · shift {} · {} waiting lot(s), {} tool(s)",
                self.policy.label(),
                format_shift_time(self.schedule.now_minute),
                self.schedule.lots.len(),
                self.schedule.tools.len()
            ),
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_metric_grid(&mut document, root, width, &metrics);
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.controls",
            "Dispatch Controls",
            "No scheduler controls available",
            &control_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.actions",
            "Next Actions",
            "Dispatch plan is clear",
            &next_action_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.queue",
            "Bottlenecks",
            "No queued lots",
            &queue_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.tools",
            "Lot / Tool Matching",
            "No dispatch tools loaded",
            &tool_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.priority",
            "Queue Priority",
            "No lots match the dispatch filters",
            &priority_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.conflicts",
            "Schedule Conflicts",
            "No schedule conflicts",
            &conflict_rows,
        );
        add_scheduler_operad_spacer(&mut document, root, OPERAD_GAP);
        add_scheduler_operad_section(
            &mut document,
            root,
            width,
            "scheduler.assignments",
            "Estimated Completion",
            "No scheduled assignments match the filters",
            &assignment_rows,
        );

        SchedulerOperadView { document, size }
    }

    fn operad_metrics(&self, result: &DispatchResult) -> Vec<SchedulerMetricTile> {
        let scheduled = result.assignments.len();
        let tardy = result
            .assignments
            .iter()
            .filter(|assignment| assignment.tardy_minutes > 0)
            .count();
        let on_time = scheduled.saturating_sub(tardy);
        let unscheduled = result.unscheduled_lots.len();
        let available_tools = self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool.state == ToolDispatchState::Available)
            .count();
        let ready_lots = self
            .schedule
            .lots
            .iter()
            .filter(|lot| lot.ready_at_minute <= self.schedule.now_minute)
            .count();
        let bottleneck = result
            .queue_summaries
            .iter()
            .max_by_key(|summary| summary.total_process_minutes);
        vec![
            SchedulerMetricTile {
                label: "Scheduled".to_string(),
                value: scheduled.to_string(),
                detail: format!("{on_time} on time"),
                tone: if tardy > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            },
            SchedulerMetricTile {
                label: "Dispatch risk".to_string(),
                value: (tardy + unscheduled).to_string(),
                detail: format!("{tardy} tardy, {unscheduled} unscheduled"),
                tone: if unscheduled > 0 || tardy > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            },
            SchedulerMetricTile {
                label: "Bottleneck".to_string(),
                value: bottleneck
                    .map(|summary| summary.tool_class.label().to_string())
                    .unwrap_or_else(|| "none".to_string()),
                detail: bottleneck
                    .map(|summary| format!("{} lots", summary.waiting_lots))
                    .unwrap_or_default(),
                tone: Tone::Warning,
            },
            SchedulerMetricTile {
                label: "Queue load".to_string(),
                value: format!("{} min", total_queue_minutes(&result.queue_summaries)),
                detail: format!("{ready_lots} ready now"),
                tone: Tone::Neutral,
            },
            SchedulerMetricTile {
                label: "Tool capacity".to_string(),
                value: self.schedule.tools.len().to_string(),
                detail: format!("{available_tools}/{} available", self.schedule.tools.len()),
                tone: if available_tools == self.schedule.tools.len() {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            },
        ]
    }

    fn operad_control_rows(&self) -> Vec<SchedulerOperadRow> {
        let mut rows = Vec::new();
        if self.schedule.is_empty() {
            rows.push(scheduler_operad_row(
                "No scheduler data loaded".to_string(),
                "Load a workspace with dispatch data to populate queue and tool matching"
                    .to_string(),
                Tone::Neutral,
                None,
                false,
            ));
            return rows;
        }
        rows.extend(DISPATCH_POLICIES.into_iter().map(|policy| {
            scheduler_operad_row(
                format!("Policy: {}", policy.label()),
                "Select dispatch sorting policy".to_string(),
                if self.policy == policy {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_POLICY}{}|controls",
                    dispatch_policy_slug(policy)
                )),
                self.policy == policy,
            )
        }));
        rows.push(scheduler_operad_row(
            self.selected_tool
                .as_ref()
                .map(|tool| format!("Selected tool: {tool}"))
                .unwrap_or_else(|| "Selected tool: none".to_string()),
            "Rows that reference a tool can update this focus".to_string(),
            Tone::Info,
            None,
            false,
        ));
        for tool in self.schedule.tools.iter().take(12) {
            rows.push(scheduler_operad_row(
                format!("Tool focus: {}", tool.id),
                format!(
                    "{} · {} · {}",
                    tool.name,
                    tool.class.label(),
                    tool.state.label()
                ),
                tool_state_tone(tool.state),
                Some(format!(
                    "{OPERAD_ACTION_SELECT_TOOL}{}|controls",
                    tool.id.as_str()
                )),
                self.selected_tool.as_ref() == Some(&tool.id),
            ));
        }
        rows.push(scheduler_operad_row(
            if self.filter_text.trim().is_empty() {
                "Search inactive".to_string()
            } else {
                format!("Search: {}", self.filter_text.trim())
            },
            "Text filtering stays on the fallback egui path until Operad has full edit routing"
                .to_string(),
            Tone::Neutral,
            None,
            false,
        ));
        rows.extend((0..=5).map(|priority| {
            scheduler_operad_row(
                format!("Minimum priority: P{priority}"),
                "Filter queue and assignment rows".to_string(),
                if self.min_priority == priority {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_MIN_PRIORITY}{priority}|controls"
                )),
                self.min_priority == priority,
            )
        }));
        rows.extend([
            scheduler_operad_row(
                if self.show_conflicts_only {
                    "Conflicts only: on"
                } else {
                    "Conflicts only: off"
                },
                "Limit queue views to risk items".to_string(),
                if self.show_conflicts_only {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
                Some(OPERAD_ACTION_TOGGLE_CONFLICTS.to_string()),
                self.show_conflicts_only,
            ),
            scheduler_operad_row(
                if self.focus_selected_tool {
                    "Focus selected tool: on"
                } else {
                    "Focus selected tool: off"
                },
                "Limit queue views to the selected tool".to_string(),
                if self.focus_selected_tool {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(OPERAD_ACTION_TOGGLE_FOCUS_TOOL.to_string()),
                self.focus_selected_tool,
            ),
            scheduler_operad_row(
                "Reset filters".to_string(),
                "Clear text search, priority, conflict-only, and tool-focus filters".to_string(),
                Tone::Neutral,
                Some(OPERAD_ACTION_RESET_FILTERS.to_string()),
                false,
            ),
        ]);
        rows
    }

    fn operad_next_action_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        self.next_action_rows(result)
            .into_iter()
            .take(12)
            .map(|action| {
                scheduler_operad_row(
                    action.title,
                    format!("{} · {}", action.timing, action.detail),
                    action.tone,
                    action.tool_id.map(|tool_id| {
                        format!("{OPERAD_ACTION_SELECT_TOOL}{}|next", tool_id.as_str())
                    }),
                    false,
                )
            })
            .collect()
    }

    fn operad_queue_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        self.queue_load_rows(result)
            .into_iter()
            .map(|row| {
                scheduler_operad_row(
                    format!("{} · {}", row.class_label, row.status_label),
                    format!(
                        "{} lots / {} min / {} per tool · {} · {}",
                        row.waiting_lots,
                        row.total_process_minutes,
                        row.load_per_tool,
                        row.driver,
                        row.action
                    ),
                    row.tone,
                    None,
                    false,
                )
            })
            .collect()
    }

    fn operad_tool_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        self.schedule
            .tools
            .iter()
            .map(|tool| {
                let match_count = self.matching_lots_for_tool(tool).count();
                let visible_count = self
                    .matching_lots_for_tool(tool)
                    .filter(|lot| {
                        let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                        self.lot_passes_filters(lot, assignment)
                    })
                    .count();
                let next_lot = self.top_lot_for_tool(tool, result);
                let recommendation = self.recommendation_for_tool(result, &tool.id);
                scheduler_operad_row(
                    format!("{} · {}", tool.id, tool.state.label()),
                    format!(
                        "{} · {} · {} · {}{}",
                        tool.name,
                        tool.class.label(),
                        match_count_label(visible_count, match_count, self.filters_active()),
                        next_lot
                            .map(|lot| format!(
                                "next P{} {} due {}",
                                lot.priority,
                                lot.id,
                                format_shift_time(lot.due_at_minute)
                            ))
                            .unwrap_or_else(|| "no matching waiting lots".to_string()),
                        recommendation
                            .map(|recommendation| format!(" · {}", recommendation.reason))
                            .unwrap_or_default()
                    ),
                    tool_state_tone(tool.state),
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_TOOL}{}|tool",
                        tool.id.as_str()
                    )),
                    self.selected_tool.as_ref() == Some(&tool.id),
                )
            })
            .collect()
    }

    fn operad_priority_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        self.priority_lots(result)
            .into_iter()
            .take(28)
            .enumerate()
            .map(|(index, lot)| {
                let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                let (eligible_tools, available_tools) = self.compatible_tool_counts(lot);
                let (risk, tone) = self.lot_risk(lot, assignment, available_tools);
                let candidate_tool = assignment
                    .map(|assignment| assignment.tool_id.clone())
                    .or_else(|| self.first_matching_tool(lot).map(|tool| tool.id.clone()));
                scheduler_operad_row(
                    format!("P{} {} · {risk}", lot.priority, lot.id),
                    format!(
                        "{} · {} · {} min · due {} · {available_tools}/{eligible_tools} tools · {}",
                        lot.product,
                        lot.required_tool_class.label(),
                        lot.process_minutes,
                        format_shift_time(lot.due_at_minute),
                        schedule_label(assignment)
                    ),
                    tone,
                    candidate_tool.map(|tool_id| {
                        format!(
                            "{OPERAD_ACTION_SELECT_TOOL}{}|priority.{index}",
                            tool_id.as_str()
                        )
                    }),
                    false,
                )
            })
            .collect()
    }

    fn operad_conflict_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        self.conflict_items(result)
            .into_iter()
            .take(16)
            .map(|conflict| {
                scheduler_operad_row(
                    format!("{} · {}", conflict.target, conflict.issue),
                    format!("{} · {}", conflict.impact, conflict.action),
                    conflict.tone,
                    None,
                    false,
                )
            })
            .collect()
    }

    fn operad_assignment_rows(&self, result: &DispatchResult) -> Vec<SchedulerOperadRow> {
        result
            .assignments
            .iter()
            .filter(|assignment| self.assignment_passes_filters(assignment))
            .take(36)
            .enumerate()
            .map(|(index, assignment)| {
                let (state, tone) = assignment_state(assignment);
                scheduler_operad_row(
                    format!("P{} {} · {state}", assignment.priority, assignment.lot_id),
                    format!(
                        "{} · {}-{} · wait {} min · due {}",
                        assignment.tool_id,
                        format_shift_time(assignment.start_minute),
                        format_shift_time(assignment.finish_minute),
                        assignment.wait_minutes,
                        format_shift_time(assignment.due_at_minute)
                    ),
                    tone,
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_TOOL}{}|assignment.{index}",
                        assignment.tool_id.as_str()
                    )),
                    self.selected_tool.as_ref() == Some(&assignment.tool_id),
                )
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str) -> Option<String> {
        if node_name == OPERAD_ACTION_TOGGLE_CONFLICTS {
            self.show_conflicts_only = !self.show_conflicts_only;
            return Some("dispatch conflict filter toggled".to_string());
        }
        if node_name == OPERAD_ACTION_TOGGLE_FOCUS_TOOL {
            self.focus_selected_tool = !self.focus_selected_tool;
            return Some("dispatch selected-tool filter toggled".to_string());
        }
        if node_name == OPERAD_ACTION_RESET_FILTERS {
            self.filter_text.clear();
            self.min_priority = 0;
            self.show_conflicts_only = false;
            self.focus_selected_tool = false;
            return Some("dispatch filters reset".to_string());
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_POLICY) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(policy) = dispatch_policy_from_slug(slug) {
                self.policy = policy;
                return Some(format!("dispatch policy set to {}", policy.label()));
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_MIN_PRIORITY) {
            let value = value
                .split_once('|')
                .map(|(value, _)| value)
                .unwrap_or(value);
            if let Ok(priority) = value.parse::<u8>()
                && priority <= 5
            {
                self.min_priority = priority;
                return Some(format!("minimum dispatch priority set to P{priority}"));
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_TOOL) {
            let tool_id = value.split_once('|').map(|(tool, _)| tool).unwrap_or(value);
            if self
                .schedule
                .tools
                .iter()
                .any(|tool| tool.id.as_str() == tool_id)
            {
                self.selected_tool = Some(ToolId::new(tool_id));
                return Some(format!("selected dispatch tool {tool_id}"));
            }
        }
        None
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        if self.schedule.is_empty() {
            return render_sidecar_interactive(
                ui,
                "scheduler.context",
                &[SidecarSection::new("Dispatch Control").empty("No scheduler data loaded")],
            )
            .map(|_| ());
        }
        let result = self.schedule.dispatch(self.policy);
        let conflicts = self.conflict_items(&result);
        let ready_lots = self
            .schedule
            .lots
            .iter()
            .filter(|lot| lot.ready_at_minute <= self.schedule.now_minute)
            .count();
        let available_tools = self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool.state == ToolDispatchState::Available)
            .count();

        let mut sections = Vec::new();
        sections.push(
            SidecarSection::new("Dispatch Control")
                .row(SidecarRow::new(
                    "Shift clock",
                    format_shift_time(self.schedule.now_minute),
                    Tone::Info,
                ))
                .row(SidecarRow::new(
                    "Queue",
                    format!(
                        "{} waiting lots / {} ready now",
                        self.schedule.lots.len(),
                        ready_lots
                    ),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Tool capacity",
                    format!(
                        "{} tools / {} available",
                        self.schedule.tools.len(),
                        available_tools
                    ),
                    if available_tools == self.schedule.tools.len() {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                ))
                .row(SidecarRow::new(
                    if conflicts.is_empty() {
                        "No schedule conflicts"
                    } else {
                        "Schedule conflicts"
                    },
                    format!("{} conflict(s)", conflicts.len()),
                    if conflicts.is_empty() {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                )),
        );

        if let Some(bottleneck) = result
            .queue_summaries
            .iter()
            .max_by_key(|summary| summary.total_process_minutes)
        {
            sections.push(
                SidecarSection::new("Bottleneck")
                    .row(SidecarRow::new(
                        bottleneck.tool_class.label(),
                        format!(
                            "{} waiting lots / {} process min",
                            bottleneck.waiting_lots, bottleneck.total_process_minutes
                        ),
                        Tone::Warning,
                    ))
                    .row(SidecarRow::new(
                        "Earliest due",
                        bottleneck
                            .earliest_due_minute
                            .map(format_shift_time)
                            .unwrap_or_else(|| "none".to_string()),
                        Tone::Neutral,
                    )),
            );
        }

        let mut tool_focus = SidecarSection::new("Tool Focus").empty("No tools available");
        if let Some(tool) = self.selected_tool_detail() {
            let matching_lots = self.matching_lots_for_tool(tool).count();
            tool_focus = tool_focus.row(SidecarRow::new(
                tool.name.clone(),
                format!(
                    "{} / {} / {}% utilized / {} matching lots",
                    tool.id,
                    tool.state.label(),
                    utilization_for_tool(&result.assignments, &tool.id),
                    matching_lots
                ),
                tool_state_tone(tool.state),
            ));
            if let Some(recommendation) = self.recommendation_for_tool(&result, &tool.id) {
                tool_focus = tool_focus.row(SidecarRow::new(
                    "Next recommendation",
                    format!(
                        "{} {}-{}",
                        recommendation
                            .lot_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "--".to_string()),
                        format_optional_time(recommendation.start_minute),
                        format_optional_time(recommendation.finish_minute)
                    ),
                    Tone::Info,
                ));
            }
        }
        for tool in self.schedule.tools.iter().take(6) {
            tool_focus = tool_focus.row(
                SidecarRow::new(
                    tool.id.to_string(),
                    format!("{} / {}", tool.class.label(), tool.state.label()),
                    tool_state_tone(tool.state),
                )
                .selected(self.selected_tool.as_ref() == Some(&tool.id))
                .action(format!("{OPERAD_ACTION_SELECT_TOOL}{}|context", tool.id)),
            );
        }
        sections.push(tool_focus);

        let mut controls = SidecarSection::new("Controls");
        for policy in DISPATCH_POLICIES {
            controls = controls.row(
                SidecarRow::new(
                    policy.label(),
                    "dispatch policy",
                    if self.policy == policy {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                )
                .selected(self.policy == policy)
                .action(format!(
                    "{OPERAD_ACTION_SET_POLICY}{}|context",
                    dispatch_policy_slug(policy)
                )),
            );
        }
        controls = controls
            .row(
                SidecarRow::new(
                    if self.show_conflicts_only {
                        "Showing conflicts only"
                    } else {
                        "Showing all work"
                    },
                    "toggle conflict filter",
                    Tone::Neutral,
                )
                .action(OPERAD_ACTION_TOGGLE_CONFLICTS),
            )
            .row(
                SidecarRow::new(
                    if self.focus_selected_tool {
                        "Focused on selected tool"
                    } else {
                        "All tools in scope"
                    },
                    "toggle selected-tool filter",
                    Tone::Neutral,
                )
                .action(OPERAD_ACTION_TOGGLE_FOCUS_TOOL),
            )
            .row(
                SidecarRow::new(
                    "Reset filters",
                    "clear scheduler context filters",
                    Tone::Info,
                )
                .action(OPERAD_ACTION_RESET_FILTERS),
            );
        sections.push(controls);

        if let Some(action) = render_sidecar_interactive(ui, "scheduler.context", &sections)? {
            let _ = self.handle_operad_action(&action);
        }
        Ok(())
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        ui_chrome::section_label(ui, "Dispatch Control");
        if self.schedule.is_empty() {
            ui_chrome::empty_state(ui, "No scheduler data loaded");
            return;
        }
        let result = self.schedule.dispatch(self.policy);
        let conflicts = self.conflict_items(&result);

        ui.label(format!(
            "Shift clock: {}",
            format_shift_time(self.schedule.now_minute)
        ));
        ui.label(format!("Waiting lots: {}", self.schedule.lots.len()));
        ui.label(format!("Tools: {}", self.schedule.tools.len()));
        if conflicts.is_empty() {
            ui_chrome::status_pill(ui, "no schedule conflicts", Tone::Success);
        } else {
            ui_chrome::status_pill(ui, &format!("{} conflicts", conflicts.len()), Tone::Danger);
        }

        if let Some(bottleneck) = result
            .queue_summaries
            .iter()
            .max_by_key(|summary| summary.total_process_minutes)
        {
            ui.separator();
            ui_chrome::section_label(ui, "Bottleneck");
            queue_summary_ui(ui, bottleneck);
        }

        ui.separator();
        ui_chrome::section_label(ui, "Tool Focus");
        self.tool_picker_ui(ui);
        self.selected_tool_summary_ui(ui, &result);

        ui.separator();
        self.filters_ui(ui);
    }

    fn summary_ui(&self, ui: &mut egui::Ui, result: &DispatchResult) {
        let scheduled = result.assignments.len();
        let tardy = result
            .assignments
            .iter()
            .filter(|assignment| assignment.tardy_minutes > 0)
            .count();
        let on_time = scheduled.saturating_sub(tardy);
        let unscheduled = result.unscheduled_lots.len();
        let available_tools = self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool.state == ToolDispatchState::Available)
            .count();
        let ready_lots = self
            .schedule
            .lots
            .iter()
            .filter(|lot| lot.ready_at_minute <= self.schedule.now_minute)
            .count();
        let bottleneck = result
            .queue_summaries
            .iter()
            .max_by_key(|summary| summary.total_process_minutes);
        let bottleneck_detail = bottleneck
            .map(|summary| format!("{} lots", summary.waiting_lots))
            .unwrap_or_default();
        let scheduled_detail = format!("{on_time} on time");
        let risk_detail = format!("{tardy} tardy, {unscheduled} unscheduled");
        let queue_detail = format!("{ready_lots} ready now");
        let tool_detail = format!("{available_tools}/{} available", self.schedule.tools.len());
        let risk_tone = if unscheduled > 0 || tardy > 0 {
            Tone::Danger
        } else {
            Tone::Success
        };
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Scheduled",
                    scheduled.to_string(),
                    scheduled_detail.as_str(),
                    if tardy > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                ),
                (
                    "Dispatch risk",
                    (tardy + unscheduled).to_string(),
                    risk_detail.as_str(),
                    risk_tone,
                ),
                (
                    "Bottleneck",
                    bottleneck
                        .map(|summary| summary.tool_class.label().to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    bottleneck_detail.as_str(),
                    Tone::Warning,
                ),
                (
                    "Queue load",
                    format!("{} min", total_queue_minutes(&result.queue_summaries)),
                    queue_detail.as_str(),
                    Tone::Neutral,
                ),
                (
                    "Tool capacity",
                    self.schedule.tools.len().to_string(),
                    tool_detail.as_str(),
                    if available_tools == self.schedule.tools.len() {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                ),
            ],
        );
    }

    fn filters_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Dispatch Filters");
        let compact = ui.available_width() < 620.0;
        let filter_controls = |ui: &mut egui::Ui, panel: &mut SchedulerPanel| {
            ui.add(
                egui::TextEdit::singleline(&mut panel.filter_text)
                    .hint_text("lot, product, recipe, class, tool")
                    .desired_width(if compact { f32::INFINITY } else { 230.0 }),
            );
            ui.add(egui::Slider::new(&mut panel.min_priority, 0..=5).text("min P"));
            ui.checkbox(&mut panel.show_conflicts_only, "conflicts only");
            ui.checkbox(&mut panel.focus_selected_tool, "focus tool");
            if ui.button("Reset").clicked() {
                panel.filter_text.clear();
                panel.min_priority = 0;
                panel.show_conflicts_only = false;
                panel.focus_selected_tool = false;
            }
        };

        if compact {
            ui.vertical(|ui| filter_controls(ui, self));
        } else {
            ui.horizontal_wrapped(|ui| filter_controls(ui, self));
        }
    }

    fn next_actions_ui(&mut self, ui: &mut egui::Ui, result: &DispatchResult, status: &mut String) {
        ui_chrome::section_label(ui, "Next Actions");
        let actions = self.next_action_rows(result);
        if actions.is_empty() {
            ui_chrome::status_pill(ui, "dispatch plan is clear", Tone::Success);
            return;
        }

        let mut pending_selection = None;
        if ui.available_width() < 700.0 {
            for action in actions.iter().take(8) {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        ui_chrome::status_pill(ui, tone_label(action.tone), action.tone);
                        if let Some(tool_id) = &action.tool_id {
                            if ui
                                .selectable_label(
                                    self.selected_tool.as_ref() == Some(tool_id),
                                    &action.title,
                                )
                                .clicked()
                            {
                                pending_selection = Some(tool_id.clone());
                            }
                        } else {
                            ui.strong(&action.title);
                        }
                    });
                    ui.small(&action.timing);
                    ui.add(egui::Label::new(&action.detail).wrap());
                });
            }
        } else {
            egui::Grid::new("scheduler_next_actions")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Action");
                    ui.strong("Window");
                    ui.strong("Driver");
                    ui.strong("State");
                    ui.end_row();
                    for action in actions.iter().take(8) {
                        if let Some(tool_id) = &action.tool_id {
                            if ui
                                .selectable_label(
                                    self.selected_tool.as_ref() == Some(tool_id),
                                    &action.title,
                                )
                                .clicked()
                            {
                                pending_selection = Some(tool_id.clone());
                            }
                        } else {
                            ui.label(&action.title);
                        }
                        ui.label(&action.timing);
                        ui.add(egui::Label::new(&action.detail).wrap());
                        ui_chrome::status_pill(ui, tone_label(action.tone), action.tone);
                        ui.end_row();
                    }
                });
        }

        if let Some(tool_id) = pending_selection {
            self.selected_tool = Some(tool_id.clone());
            *status = format!("selected dispatch tool {tool_id}");
        }
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
        if self.schedule.now_minute >= start && self.schedule.now_minute <= end {
            let x = time_x(plot, start, end, self.schedule.now_minute);
            painter.line_segment(
                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                Stroke::new(1.5, Tone::Info.color()),
            );
            painter.text(
                Pos2::new(x + 3.0, plot.top() + 6.0),
                egui::Align2::LEFT_CENTER,
                "now",
                egui::FontId::proportional(11.0),
                Tone::Info.color(),
            );
        }

        let row_h = (plot.height() / self.schedule.tools.len().max(1) as f32).max(24.0);
        for (index, tool) in self.schedule.tools.iter().enumerate() {
            let y = plot.top() + index as f32 * row_h;
            let row_rect = Rect::from_min_size(
                Pos2::new(plot.left(), y + 3.0),
                Vec2::new(plot.width(), row_h - 6.0),
            );
            if self.selected_tool.as_ref() == Some(&tool.id) {
                painter.rect_filled(
                    row_rect,
                    3.0,
                    Color32::from_rgba_unmultiplied(93, 168, 232, 32),
                );
            }
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
            if assignment.due_at_minute >= start && assignment.due_at_minute <= end {
                let due_x = time_x(plot, start, end, assignment.due_at_minute);
                painter.line_segment(
                    [
                        Pos2::new(due_x, bar.top() - 3.0),
                        Pos2::new(due_x, bar.bottom() + 3.0),
                    ],
                    Stroke::new(
                        1.4,
                        if assignment.tardy_minutes > 0 {
                            Tone::Danger.color()
                        } else {
                            ui.visuals().weak_text_color()
                        },
                    ),
                );
            }
            painter.rect_filled(bar, 4.0, assignment_color(assignment.priority));
            painter.rect_stroke(
                bar,
                4.0,
                Stroke::new(
                    if assignment.tardy_minutes > 0 {
                        2.0
                    } else {
                        1.0
                    },
                    if assignment.tardy_minutes > 0 {
                        Tone::Danger.color()
                    } else {
                        Color32::from_black_alpha(80)
                    },
                ),
                StrokeKind::Inside,
            );
            painter.text(
                bar.center(),
                egui::Align2::CENTER_CENTER,
                if bar.width() < 46.0 {
                    format!("P{}", assignment.priority)
                } else {
                    assignment.lot_id.to_string()
                },
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }

    fn queue_ui(&self, ui: &mut egui::Ui, result: &DispatchResult) {
        ui_chrome::section_label(ui, "Bottlenecks");
        let rows = self.queue_load_rows(result);
        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No queued lots");
            return;
        }

        if ui.available_width() < 680.0 {
            for row in &rows {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&row.class_label);
                        ui_chrome::status_pill(ui, &row.status_label, row.tone);
                    });
                    ui.small(format!(
                        "{} lots / {} min / {} min per available tool",
                        row.waiting_lots, row.total_process_minutes, row.load_per_tool
                    ));
                    ui.small(format!(
                        "{} tools, next idle {}",
                        row.tool_capacity, row.next_idle
                    ));
                    ui.add(egui::Label::new(&row.driver).wrap());
                    ui.add(egui::Label::new(&row.action).wrap());
                });
            }
            return;
        }

        egui::Grid::new("scheduler_queue_summary")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Class");
                ui.strong("Lots");
                ui.strong("Load");
                ui.strong("Tools");
                ui.strong("Next idle");
                ui.strong("Driver");
                ui.strong("Action");
                ui.end_row();
                for row in &rows {
                    ui.label(&row.class_label);
                    ui.label(row.waiting_lots.to_string());
                    ui.label(format!(
                        "{} min / {} per tool",
                        row.total_process_minutes, row.load_per_tool
                    ));
                    ui.label(&row.tool_capacity);
                    ui.label(&row.next_idle);
                    ui.add(egui::Label::new(&row.driver).wrap());
                    ui.horizontal(|ui| {
                        ui_chrome::status_pill(ui, &row.status_label, row.tone);
                        ui.add(egui::Label::new(&row.action).wrap());
                    });
                    ui.end_row();
                }
            });
    }

    fn tool_match_ui(&mut self, ui: &mut egui::Ui, result: &DispatchResult, status: &mut String) {
        ui_chrome::section_label(ui, "Lot / Tool Matching");
        if self.schedule.tools.is_empty() {
            ui_chrome::empty_state(ui, "No dispatch tools loaded");
            return;
        }

        let mut pending_selection = None;
        if ui.available_width() < 720.0 {
            for tool in &self.schedule.tools {
                let selected = self.selected_tool.as_ref() == Some(&tool.id);
                let match_count = self.matching_lots_for_tool(tool).count();
                let visible_count = self
                    .matching_lots_for_tool(tool)
                    .filter(|lot| {
                        let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                        self.lot_passes_filters(lot, assignment)
                    })
                    .count();
                let next_lot = self.top_lot_for_tool(tool, result);
                let recommendation = self.recommendation_for_tool(result, &tool.id);
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(selected, tool.id.to_string()).clicked() {
                            pending_selection = Some(tool.id.clone());
                        }
                        ui_chrome::status_pill(ui, tool.state.label(), tool_state_tone(tool.state));
                    });
                    ui.small(format!(
                        "{} / {} / {}",
                        tool.name,
                        tool.class.label(),
                        recipe_scope(tool)
                    ));
                    ui.label(match_count_label(
                        visible_count,
                        match_count,
                        self.filters_active(),
                    ));
                    if let Some(lot) = next_lot {
                        ui.small(format!(
                            "next match P{} {} due {}",
                            lot.priority,
                            lot.id,
                            format_shift_time(lot.due_at_minute)
                        ));
                    } else {
                        ui.small("no matching waiting lots");
                    }
                    if let Some(recommendation) = recommendation {
                        ui.add(egui::Label::new(&recommendation.reason).wrap());
                    }
                });
            }
        } else {
            egui::Grid::new("scheduler_tool_match")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Tool");
                    ui.strong("State");
                    ui.strong("Class");
                    ui.strong("Recipe scope");
                    ui.strong("Queue match");
                    ui.strong("Next lot");
                    ui.strong("Action");
                    ui.end_row();
                    for tool in &self.schedule.tools {
                        let selected = self.selected_tool.as_ref() == Some(&tool.id);
                        let match_count = self.matching_lots_for_tool(tool).count();
                        let visible_count = self
                            .matching_lots_for_tool(tool)
                            .filter(|lot| {
                                let assignment =
                                    self.assignment_for_lot(&result.assignments, &lot.id);
                                self.lot_passes_filters(lot, assignment)
                            })
                            .count();
                        let next_lot = self.top_lot_for_tool(tool, result);
                        let recommendation = self.recommendation_for_tool(result, &tool.id);
                        if ui.selectable_label(selected, tool.id.to_string()).clicked() {
                            pending_selection = Some(tool.id.clone());
                        }
                        ui_chrome::status_pill(ui, tool.state.label(), tool_state_tone(tool.state));
                        ui.label(tool.class.label());
                        ui.label(recipe_scope(tool));
                        ui.label(match_count_label(
                            visible_count,
                            match_count,
                            self.filters_active(),
                        ));
                        if let Some(lot) = next_lot {
                            ui.label(format!("P{} {} ({})", lot.priority, lot.id, lot.recipe_id));
                        } else {
                            ui.label("--");
                        }
                        if let Some(recommendation) = recommendation {
                            ui.add(egui::Label::new(&recommendation.reason).wrap());
                        } else {
                            ui.label("--");
                        }
                        ui.end_row();
                    }
                });
        }

        if let Some(tool_id) = pending_selection {
            self.selected_tool = Some(tool_id.clone());
            *status = format!("selected dispatch tool {tool_id}");
        }
    }

    fn queue_priority_ui(
        &mut self,
        ui: &mut egui::Ui,
        result: &DispatchResult,
        status: &mut String,
    ) {
        ui_chrome::section_label(ui, "Queue Priority");
        let lots = self.priority_lots(result);
        if lots.is_empty() {
            ui_chrome::empty_state(ui, "No lots match the dispatch filters");
            return;
        }

        let mut pending_selection = None;
        if ui.available_width() < 760.0 {
            for lot in lots.iter().take(16) {
                let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                let (eligible_tools, available_tools) = self.compatible_tool_counts(lot);
                let (risk, tone) = self.lot_risk(lot, assignment, available_tools);
                let candidate_tool = assignment
                    .map(|assignment| assignment.tool_id.clone())
                    .or_else(|| self.first_matching_tool(lot).map(|tool| tool.id.clone()));
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 560.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("P{} {}", lot.priority, lot.id));
                        ui_chrome::status_pill(ui, &risk, tone);
                    });
                    ui.small(format!(
                        "{} / {} / {} min / due {}",
                        lot.product,
                        lot.required_tool_class.label(),
                        lot.process_minutes,
                        format_shift_time(lot.due_at_minute)
                    ));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{available_tools}/{eligible_tools} tools available"
                        ));
                        if let Some(tool_id) = &candidate_tool {
                            if ui
                                .selectable_label(
                                    self.selected_tool.as_ref() == Some(tool_id),
                                    tool_id.to_string(),
                                )
                                .clicked()
                            {
                                pending_selection = Some(tool_id.clone());
                            }
                        }
                    });
                    ui.small(schedule_label(assignment));
                });
            }
        } else {
            egui::Grid::new("scheduler_priority_queue")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Lot");
                    ui.strong("P");
                    ui.strong("Queue");
                    ui.strong("Match");
                    ui.strong("Schedule");
                    ui.strong("Due");
                    ui.strong("Risk");
                    ui.end_row();
                    for lot in lots.iter().take(24) {
                        let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                        let (eligible_tools, available_tools) = self.compatible_tool_counts(lot);
                        let (risk, tone) = self.lot_risk(lot, assignment, available_tools);
                        let candidate_tool = assignment
                            .map(|assignment| assignment.tool_id.clone())
                            .or_else(|| self.first_matching_tool(lot).map(|tool| tool.id.clone()));
                        ui.vertical(|ui| {
                            ui.label(lot.id.to_string());
                            ui.small(&lot.product);
                        });
                        ui.label(lot.priority.to_string());
                        ui.vertical(|ui| {
                            ui.label(lot.required_tool_class.label());
                            ui.small(format!("{} / {} min", lot.recipe_id, lot.process_minutes));
                        });
                        ui.vertical(|ui| {
                            ui.label(format!("{available_tools}/{eligible_tools} available"));
                            if let Some(tool_id) = &candidate_tool {
                                if ui
                                    .selectable_label(
                                        self.selected_tool.as_ref() == Some(tool_id),
                                        tool_id.to_string(),
                                    )
                                    .clicked()
                                {
                                    pending_selection = Some(tool_id.clone());
                                }
                            }
                        });
                        ui.label(schedule_label(assignment));
                        ui.label(format_shift_time(lot.due_at_minute));
                        ui_chrome::status_pill(ui, &risk, tone);
                        ui.end_row();
                    }
                });
        }

        if let Some(tool_id) = pending_selection {
            self.selected_tool = Some(tool_id.clone());
            *status = format!("selected dispatch tool {tool_id}");
        }
    }

    fn conflicts_ui(&self, ui: &mut egui::Ui, result: &DispatchResult) {
        ui_chrome::section_label(ui, "Schedule Conflicts");
        let conflicts = self.conflict_items(result);
        if conflicts.is_empty() {
            ui_chrome::status_pill(ui, "no schedule conflicts", Tone::Success);
            return;
        }

        if ui.available_width() < 720.0 {
            for conflict in conflicts.iter().take(12) {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&conflict.target);
                        ui_chrome::status_pill(ui, &conflict.issue, conflict.tone);
                    });
                    ui.add(egui::Label::new(&conflict.impact).wrap());
                    ui.small(&conflict.action);
                });
            }
            return;
        }

        egui::Grid::new("scheduler_conflicts")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Target");
                ui.strong("Issue");
                ui.strong("Impact");
                ui.strong("Next action");
                ui.end_row();
                for conflict in conflicts.iter().take(12) {
                    ui.label(&conflict.target);
                    ui_chrome::status_pill(ui, &conflict.issue, conflict.tone);
                    ui.add(egui::Label::new(&conflict.impact).wrap());
                    ui.add(egui::Label::new(&conflict.action).wrap());
                    ui.end_row();
                }
            });
    }

    fn assignment_table_ui(
        &mut self,
        ui: &mut egui::Ui,
        assignments: &[DispatchAssignment],
        status: &mut String,
    ) {
        ui_chrome::section_label(ui, "Estimated Completion");
        let visible = assignments
            .iter()
            .filter(|assignment| self.assignment_passes_filters(assignment))
            .collect::<Vec<_>>();
        if visible.is_empty() {
            ui_chrome::empty_state(ui, "No scheduled assignments match the filters");
            return;
        }

        let mut pending_selection = None;
        if ui.available_width() < 720.0 {
            for assignment in visible.iter().take(18) {
                let (state, tone) = assignment_state(assignment);
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("P{} {}", assignment.priority, assignment.lot_id));
                        ui_chrome::status_pill(ui, &state, tone);
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(
                                self.selected_tool.as_ref() == Some(&assignment.tool_id),
                                assignment.tool_id.to_string(),
                            )
                            .clicked()
                        {
                            pending_selection = Some(assignment.tool_id.clone());
                        }
                        ui.label(format!(
                            "{}-{}",
                            format_shift_time(assignment.start_minute),
                            format_shift_time(assignment.finish_minute)
                        ));
                    });
                    ui.small(format!(
                        "wait {} min / due {}",
                        assignment.wait_minutes,
                        format_shift_time(assignment.due_at_minute)
                    ));
                });
            }
        } else {
            egui::Grid::new("scheduler_assignment_table")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Lot");
                    ui.strong("P");
                    ui.strong("Tool");
                    ui.strong("Start");
                    ui.strong("Finish");
                    ui.strong("Wait");
                    ui.strong("Due");
                    ui.strong("Status");
                    ui.end_row();
                    for assignment in visible.iter().take(36) {
                        let (state, tone) = assignment_state(assignment);
                        ui.label(assignment.lot_id.to_string());
                        ui.label(assignment.priority.to_string());
                        if ui
                            .selectable_label(
                                self.selected_tool.as_ref() == Some(&assignment.tool_id),
                                assignment.tool_id.to_string(),
                            )
                            .clicked()
                        {
                            pending_selection = Some(assignment.tool_id.clone());
                        }
                        ui.label(format_shift_time(assignment.start_minute));
                        let finish = RichText::new(format_shift_time(assignment.finish_minute))
                            .color(if assignment.tardy_minutes > 0 {
                                Tone::Danger.color()
                            } else {
                                ui.visuals().text_color()
                            });
                        ui.label(finish);
                        ui.label(format!("{} min", assignment.wait_minutes));
                        ui.label(format_shift_time(assignment.due_at_minute));
                        ui_chrome::status_pill(ui, &state, tone);
                        ui.end_row();
                    }
                });
        }

        if let Some(tool_id) = pending_selection {
            self.selected_tool = Some(tool_id.clone());
            *status = format!("selected dispatch tool {tool_id}");
        }
    }

    fn tool_picker_ui(&mut self, ui: &mut egui::Ui) {
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
    }

    fn selected_tool_summary_ui(&self, ui: &mut egui::Ui, result: &DispatchResult) {
        let Some(tool) = self.selected_tool_detail() else {
            ui_chrome::empty_state(ui, "Selected tool is missing");
            return;
        };
        let mut matching_lots = self.matching_lots_for_tool(tool).collect::<Vec<_>>();
        matching_lots.sort_by_key(|lot| self.lot_priority_key(lot, result));
        let recommendation = self.recommendation_for_tool(result, &tool.id);

        ui.group(|ui| {
            ui.strong(&tool.name);
            ui.horizontal_wrapped(|ui| {
                ui.small(format!("{} / {}", tool.id, tool.class.label()));
                ui_chrome::status_pill(ui, tool.state.label(), tool_state_tone(tool.state));
            });
            ui.label(format!(
                "Utilization: {}%",
                utilization_for_tool(&result.assignments, &tool.id)
            ));
            ui.label(format!("Compatible queue: {} lots", matching_lots.len()));
            if let Some(recommendation) = recommendation {
                ui.add(
                    egui::Label::new(format!(
                        "Next: {} {}-{} ({})",
                        recommendation
                            .lot_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "--".to_string()),
                        format_optional_time(recommendation.start_minute),
                        format_optional_time(recommendation.finish_minute),
                        recommendation.reason
                    ))
                    .wrap(),
                );
            }
            if matching_lots.is_empty() {
                ui.small("No matching waiting lots");
            } else {
                for lot in matching_lots.iter().take(4) {
                    ui.small(format!(
                        "P{} {} due {} / {}",
                        lot.priority,
                        lot.id,
                        format_shift_time(lot.due_at_minute),
                        lot.recipe_id
                    ));
                }
            }
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

    fn next_action_rows(&self, result: &DispatchResult) -> Vec<NextAction> {
        let mut actions = Vec::new();

        for conflict in self.conflict_items(result).into_iter().take(5) {
            actions.push(NextAction {
                tool_id: None,
                title: format!("Resolve {}", conflict.target),
                timing: conflict.issue.clone(),
                detail: format!("{} {}", conflict.impact, conflict.action),
                tone: conflict.tone,
            });
        }

        for recommendation in &result.recommendations {
            if let Some(lot_id) = &recommendation.lot_id {
                let assignment = self.assignment_for_lot(&result.assignments, lot_id);
                let tone = assignment.map_or(Tone::Info, |assignment| {
                    if assignment.tardy_minutes > 0 {
                        Tone::Danger
                    } else if assignment.priority >= 5 || assignment.wait_minutes > 60 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    }
                });
                actions.push(NextAction {
                    tool_id: Some(recommendation.tool_id.clone()),
                    title: format!("Dispatch {lot_id} on {}", recommendation.tool_id),
                    timing: format!(
                        "{}-{}",
                        format_optional_time(recommendation.start_minute),
                        format_optional_time(recommendation.finish_minute)
                    ),
                    detail: recommendation.reason.clone(),
                    tone,
                });
            } else if recommendation.reason != "no compatible waiting lots"
                && recommendation.reason != "compatible queue empty"
            {
                actions.push(NextAction {
                    tool_id: Some(recommendation.tool_id.clone()),
                    title: format!("Clear {}", recommendation.tool_id),
                    timing: "no dispatch window".to_string(),
                    detail: recommendation.reason.clone(),
                    tone: Tone::Warning,
                });
            }
        }

        actions.sort_by(|left, right| {
            tone_rank(right.tone)
                .cmp(&tone_rank(left.tone))
                .then_with(|| left.title.cmp(&right.title))
        });
        actions
    }

    fn queue_load_rows(&self, result: &DispatchResult) -> Vec<QueueLoadRow> {
        let mut latest_by_tool = self
            .schedule
            .tools
            .iter()
            .map(|tool| (tool.id.clone(), self.schedule.now_minute))
            .collect::<BTreeMap<_, _>>();
        for assignment in &result.assignments {
            latest_by_tool
                .entry(assignment.tool_id.clone())
                .and_modify(|minute| *minute = (*minute).max(assignment.finish_minute))
                .or_insert(assignment.finish_minute);
        }

        let mut rows = result
            .queue_summaries
            .iter()
            .map(|summary| {
                let tools = self
                    .schedule
                    .tools
                    .iter()
                    .filter(|tool| tool.class == summary.tool_class)
                    .collect::<Vec<_>>();
                let total_tools = tools.len();
                let available_tools = tools
                    .iter()
                    .filter(|tool| tool.state == ToolDispatchState::Available)
                    .count();
                let load_per_tool = if available_tools == 0 {
                    summary.total_process_minutes
                } else {
                    ceil_div(summary.total_process_minutes, available_tools as u32)
                };
                let next_idle = tools
                    .iter()
                    .filter(|tool| tool.state == ToolDispatchState::Available)
                    .filter_map(|tool| latest_by_tool.get(&tool.id).copied())
                    .min()
                    .map(format_shift_time)
                    .unwrap_or_else(|| "--".to_string());
                let top_lot = self.top_lot_for_class(summary.tool_class, result);
                let driver = top_lot
                    .map(|lot| {
                        format!(
                            "P{} {} due {}",
                            lot.priority,
                            lot.id,
                            format_shift_time(lot.due_at_minute)
                        )
                    })
                    .or_else(|| {
                        summary
                            .earliest_due_minute
                            .map(|due| format!("earliest due {}", format_shift_time(due)))
                    })
                    .unwrap_or_else(|| "no waiting lots".to_string());
                let (status_label, tone, action) = queue_status(
                    summary,
                    available_tools,
                    load_per_tool,
                    top_lot.map(|lot| lot.slack_minutes_at(self.schedule.now_minute)),
                );
                QueueLoadRow {
                    class_label: summary.tool_class.label().to_string(),
                    waiting_lots: summary.waiting_lots,
                    total_process_minutes: summary.total_process_minutes,
                    load_per_tool,
                    tool_capacity: format!("{available_tools}/{total_tools} available"),
                    next_idle,
                    driver,
                    status_label,
                    action,
                    tone,
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| {
            tone_rank(right.tone)
                .cmp(&tone_rank(left.tone))
                .then_with(|| right.load_per_tool.cmp(&left.load_per_tool))
                .then_with(|| right.waiting_lots.cmp(&left.waiting_lots))
        });
        rows
    }

    fn priority_lots(&self, result: &DispatchResult) -> Vec<&DispatchLot> {
        let mut lots = self
            .schedule
            .lots
            .iter()
            .filter(|lot| {
                let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                self.lot_passes_filters(lot, assignment)
            })
            .collect::<Vec<_>>();
        lots.sort_by_key(|lot| self.lot_priority_key(lot, result));
        lots
    }

    fn top_lot_for_tool<'a>(
        &'a self,
        tool: &'a DispatchTool,
        result: &DispatchResult,
    ) -> Option<&'a DispatchLot> {
        let mut lots = self
            .matching_lots_for_tool(tool)
            .filter(|lot| {
                let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
                self.lot_passes_filters(lot, assignment)
            })
            .collect::<Vec<_>>();
        lots.sort_by_key(|lot| self.lot_priority_key(lot, result));
        lots.into_iter().next()
    }

    fn top_lot_for_class<'a>(
        &'a self,
        tool_class: ToolClass,
        result: &DispatchResult,
    ) -> Option<&'a DispatchLot> {
        let mut lots = self
            .schedule
            .lots
            .iter()
            .filter(|lot| lot.required_tool_class == tool_class)
            .collect::<Vec<_>>();
        lots.sort_by_key(|lot| self.lot_priority_key(lot, result));
        lots.into_iter().next()
    }

    fn matching_lots_for_tool<'a>(
        &'a self,
        tool: &'a DispatchTool,
    ) -> impl Iterator<Item = &'a DispatchLot> + 'a {
        self.schedule
            .lots
            .iter()
            .filter(move |lot| tool_matches_lot(tool, lot))
    }

    fn first_matching_tool(&self, lot: &DispatchLot) -> Option<&DispatchTool> {
        self.schedule
            .tools
            .iter()
            .filter(|tool| tool_matches_lot(tool, lot))
            .min_by(|left, right| {
                tool_state_rank(left.state)
                    .cmp(&tool_state_rank(right.state))
                    .then_with(|| left.id.cmp(&right.id))
            })
    }

    fn compatible_tool_counts(&self, lot: &DispatchLot) -> (usize, usize) {
        let eligible = self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool_matches_lot(tool, lot))
            .count();
        let available = self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool.can_process(lot))
            .count();
        (eligible, available)
    }

    fn lot_risk(
        &self,
        lot: &DispatchLot,
        assignment: Option<&DispatchAssignment>,
        available_tools: usize,
    ) -> (String, Tone) {
        if let Some(assignment) = assignment {
            if assignment.tardy_minutes > 0 {
                return (
                    format!("late {} min", assignment.tardy_minutes),
                    Tone::Danger,
                );
            }
            if self.maintenance_delay_label(assignment).is_some() {
                return ("maintenance delay".to_string(), Tone::Warning);
            }
        } else if available_tools == 0 {
            return ("unscheduled".to_string(), Tone::Danger);
        } else {
            return ("not placed".to_string(), Tone::Warning);
        }

        let slack = lot.slack_minutes_at(self.schedule.now_minute);
        if slack <= 0 {
            ("due now".to_string(), Tone::Danger)
        } else if slack <= 60 || lot.priority >= 5 {
            (format!("{} min slack", slack.max(0)), Tone::Warning)
        } else {
            ("ready".to_string(), Tone::Success)
        }
    }

    fn lot_priority_key(
        &self,
        lot: &DispatchLot,
        result: &DispatchResult,
    ) -> (u8, u8, i32, u32, u32, String) {
        let assignment = self.assignment_for_lot(&result.assignments, &lot.id);
        let conflict_rank = if assignment.map_or(true, |assignment| assignment.tardy_minutes > 0) {
            0
        } else if lot.slack_minutes_at(self.schedule.now_minute) <= 60 {
            1
        } else {
            2
        };
        (
            conflict_rank,
            u8::MAX - lot.priority,
            lot.slack_minutes_at(self.schedule.now_minute),
            lot.due_at_minute,
            lot.fifo_sequence,
            lot.id.to_string(),
        )
    }

    fn lot_passes_filters(
        &self,
        lot: &DispatchLot,
        assignment: Option<&DispatchAssignment>,
    ) -> bool {
        if lot.priority < self.min_priority {
            return false;
        }

        let query = self.filter_text.trim().to_ascii_lowercase();
        if !query.is_empty() && !self.lot_matches_query(lot, assignment, &query) {
            return false;
        }

        if self.focus_selected_tool {
            let Some(selected_tool) = self.selected_tool_detail() else {
                return false;
            };
            let assigned_to_selected = assignment
                .map(|assignment| assignment.tool_id == selected_tool.id)
                .unwrap_or(false);
            if !assigned_to_selected && !tool_matches_lot(selected_tool, lot) {
                return false;
            }
        }

        if self.show_conflicts_only {
            let (_, available_tools) = self.compatible_tool_counts(lot);
            let conflicted = assignment
                .map(|assignment| {
                    assignment.tardy_minutes > 0
                        || self.maintenance_delay_label(assignment).is_some()
                })
                .unwrap_or(true)
                || available_tools == 0
                || lot.slack_minutes_at(self.schedule.now_minute) <= 0;
            if !conflicted {
                return false;
            }
        }

        true
    }

    fn lot_matches_query(
        &self,
        lot: &DispatchLot,
        assignment: Option<&DispatchAssignment>,
        query: &str,
    ) -> bool {
        let mut haystack = format!(
            "{} {} {} {} {} {}",
            lot.id,
            lot.product,
            lot.step_id,
            lot.step_name,
            lot.required_tool_class.label(),
            lot.recipe_id
        )
        .to_ascii_lowercase();
        if let Some(assignment) = assignment {
            haystack.push(' ');
            haystack.push_str(&assignment.tool_id.to_string().to_ascii_lowercase());
        }
        haystack.contains(query)
    }

    fn assignment_passes_filters(&self, assignment: &DispatchAssignment) -> bool {
        if let Some(lot) = self.lot_by_id(&assignment.lot_id) {
            return self.lot_passes_filters(lot, Some(assignment));
        }

        if assignment.priority < self.min_priority {
            return false;
        }
        let query = self.filter_text.trim().to_ascii_lowercase();
        if !query.is_empty()
            && !format!("{} {}", assignment.lot_id, assignment.tool_id)
                .to_ascii_lowercase()
                .contains(&query)
        {
            return false;
        }
        if self.focus_selected_tool && self.selected_tool.as_ref() != Some(&assignment.tool_id) {
            return false;
        }
        if self.show_conflicts_only && assignment.tardy_minutes == 0 {
            return false;
        }
        true
    }

    fn conflict_items(&self, result: &DispatchResult) -> Vec<ConflictItem> {
        let mut items = Vec::new();

        for lot_id in &result.unscheduled_lots {
            if let Some(lot) = self.lot_by_id(lot_id) {
                let (eligible_tools, available_tools) = self.compatible_tool_counts(lot);
                let (issue, impact, action) = if eligible_tools == 0 {
                    (
                        "no match".to_string(),
                        format!(
                            "P{} {} requires {} / {} with no qualified tool",
                            lot.priority,
                            lot.id,
                            lot.required_tool_class.label(),
                            lot.recipe_id
                        ),
                        "Qualify a recipe/tool pair or reroute the lot".to_string(),
                    )
                } else if available_tools == 0 {
                    (
                        "tool unavailable".to_string(),
                        format!(
                            "{} matching tools exist but none are available",
                            eligible_tools
                        ),
                        "Release maintenance/down tools or move the recipe to an alternate tool"
                            .to_string(),
                    )
                } else {
                    (
                        "unscheduled".to_string(),
                        format!(
                            "{} compatible tools available; policy did not place the lot",
                            available_tools
                        ),
                        "Review policy order and queue capacity for this class".to_string(),
                    )
                };
                items.push(ConflictItem {
                    target: lot.id.to_string(),
                    issue,
                    impact,
                    action,
                    tone: Tone::Danger,
                });
            }
        }

        for assignment in result
            .assignments
            .iter()
            .filter(|assignment| assignment.tardy_minutes > 0)
        {
            items.push(ConflictItem {
                target: assignment.lot_id.to_string(),
                issue: "late".to_string(),
                impact: format!(
                    "Finishes {} min after due time {}",
                    assignment.tardy_minutes,
                    format_shift_time(assignment.due_at_minute)
                ),
                action: "Move ahead in the queue or choose an alternate matching tool".to_string(),
                tone: Tone::Danger,
            });
        }

        for assignment in &result.assignments {
            if let Some(delay) = self.maintenance_delay_label(assignment) {
                items.push(ConflictItem {
                    target: assignment.tool_id.to_string(),
                    issue: "maintenance".to_string(),
                    impact: format!(
                        "{} waits {} min before {}",
                        assignment.lot_id, assignment.wait_minutes, delay
                    ),
                    action:
                        "Confirm the window or dispatch the lot to an alternate compatible tool"
                            .to_string(),
                    tone: Tone::Warning,
                });
            }
        }

        for tool in self
            .schedule
            .tools
            .iter()
            .filter(|tool| tool.state != ToolDispatchState::Available)
        {
            let matching_lots = self.matching_lots_for_tool(tool).count();
            if matching_lots > 0 {
                items.push(ConflictItem {
                    target: tool.id.to_string(),
                    issue: tool.state.label().to_string(),
                    impact: format!("{matching_lots} compatible waiting lots cannot use this tool"),
                    action: "Recover the tool or move matching recipes to available capacity"
                        .to_string(),
                    tone: Tone::Warning,
                });
            }
        }

        items.sort_by(|left, right| {
            tone_rank(right.tone)
                .cmp(&tone_rank(left.tone))
                .then_with(|| left.target.cmp(&right.target))
                .then_with(|| left.issue.cmp(&right.issue))
        });
        items
    }

    fn maintenance_delay_label(&self, assignment: &DispatchAssignment) -> Option<String> {
        let lot = self.lot_by_id(&assignment.lot_id)?;
        let tool = self.tool_by_id(&assignment.tool_id)?;
        let requested_start = lot.ready_at_minute.max(self.schedule.now_minute);
        let requested_finish = requested_start.saturating_add(lot.process_minutes);
        tool.maintenance_windows
            .iter()
            .find(|window| {
                window.overlaps(requested_start, requested_finish)
                    && assignment.start_minute >= window.end_minute
            })
            .map(|window| {
                format!(
                    "{} {}-{}",
                    window.reason,
                    format_shift_time(window.start_minute),
                    format_shift_time(window.end_minute)
                )
            })
    }

    fn recommendation_for_tool<'a>(
        &self,
        result: &'a DispatchResult,
        tool_id: &ToolId,
    ) -> Option<&'a ToolRecommendation> {
        result
            .recommendations
            .iter()
            .find(|recommendation| recommendation.tool_id == *tool_id)
    }

    fn assignment_for_lot<'a>(
        &self,
        assignments: &'a [DispatchAssignment],
        lot_id: &LotId,
    ) -> Option<&'a DispatchAssignment> {
        assignments
            .iter()
            .find(|assignment| assignment.lot_id == *lot_id)
    }

    fn lot_by_id(&self, lot_id: &LotId) -> Option<&DispatchLot> {
        self.schedule.lots.iter().find(|lot| lot.id == *lot_id)
    }

    fn tool_by_id(&self, tool_id: &ToolId) -> Option<&DispatchTool> {
        self.schedule.tools.iter().find(|tool| tool.id == *tool_id)
    }

    fn filters_active(&self) -> bool {
        !self.filter_text.trim().is_empty()
            || self.min_priority > 0
            || self.show_conflicts_only
            || self.focus_selected_tool
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

struct NextAction {
    tool_id: Option<ToolId>,
    title: String,
    timing: String,
    detail: String,
    tone: Tone,
}

struct QueueLoadRow {
    class_label: String,
    waiting_lots: usize,
    total_process_minutes: u32,
    load_per_tool: u32,
    tool_capacity: String,
    next_idle: String,
    driver: String,
    status_label: String,
    action: String,
    tone: Tone,
}

struct ConflictItem {
    target: String,
    issue: String,
    impact: String,
    action: String,
    tone: Tone,
}

const DISPATCH_POLICIES: [DispatchPolicy; 3] = [
    DispatchPolicy::PriorityThenFifo,
    DispatchPolicy::DueDateThenPriority,
    DispatchPolicy::Fifo,
];

fn dispatch_policy_slug(policy: DispatchPolicy) -> &'static str {
    match policy {
        DispatchPolicy::Fifo => "fifo",
        DispatchPolicy::PriorityThenFifo => "priority-fifo",
        DispatchPolicy::DueDateThenPriority => "due-date-priority",
    }
}

fn dispatch_policy_from_slug(slug: &str) -> Option<DispatchPolicy> {
    match slug {
        "fifo" => Some(DispatchPolicy::Fifo),
        "priority-fifo" => Some(DispatchPolicy::PriorityThenFifo),
        "due-date-priority" => Some(DispatchPolicy::DueDateThenPriority),
        _ => None,
    }
}

fn scheduler_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += scheduler_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += scheduler_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn scheduler_operad_metric_columns(width: f32) -> usize {
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

fn scheduler_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = scheduler_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn scheduler_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_scheduler_operad_header(
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
            "scheduler.header",
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
    add_scheduler_operad_text(
        document,
        header,
        "scheduler.header.eyebrow",
        eyebrow,
        scheduler_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_scheduler_operad_text(
        document,
        header,
        "scheduler.header.title",
        title,
        scheduler_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_scheduler_operad_text(
        document,
        header,
        "scheduler.header.detail",
        detail,
        scheduler_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_scheduler_operad_text(
        document,
        header,
        "scheduler.header.meta",
        meta,
        scheduler_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_scheduler_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[SchedulerMetricTile],
) {
    let columns = scheduler_operad_metric_columns(width);
    let grid_height = scheduler_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "scheduler.metrics",
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
                format!("scheduler.metrics.row.{row_index}"),
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
            add_scheduler_operad_metric_tile(
                document,
                row,
                &format!("scheduler.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_scheduler_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &SchedulerMetricTile,
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
                scheduler_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_scheduler_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        scheduler_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_scheduler_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        scheduler_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_scheduler_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        scheduler_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            scheduler_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_scheduler_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[SchedulerOperadRow],
) {
    let height = scheduler_operad_section_height(rows.len());
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
    add_scheduler_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        scheduler_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_scheduler_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_scheduler_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_scheduler_operad_empty_row(
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
    add_scheduler_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        scheduler_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_scheduler_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &SchedulerOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        scheduler_operad_tone_color(Tone::Info)
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
        .with_visual(UiVisual::panel(
            scheduler_operad_tone_color(row.tone),
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
    add_scheduler_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        scheduler_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_scheduler_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        scheduler_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_scheduler_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("scheduler.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_scheduler_operad_text(
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

fn scheduler_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn scheduler_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn scheduler_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> SchedulerOperadRow {
    SchedulerOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
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

fn queue_summary_ui(ui: &mut egui::Ui, summary: &QueueSummary) {
    ui.label(summary.tool_class.label());
    ui.label(format!("{} waiting lots", summary.waiting_lots));
    ui.label(format!("{} min queued", summary.total_process_minutes));
    if let Some(due) = summary.earliest_due_minute {
        ui.label(format!("Earliest due {}", format_shift_time(due)));
    }
}

fn queue_status(
    summary: &QueueSummary,
    available_tools: usize,
    load_per_tool: u32,
    top_slack: Option<i32>,
) -> (String, Tone, String) {
    if summary.waiting_lots == 0 {
        return (
            "clear".to_string(),
            Tone::Success,
            "No waiting lots for this class".to_string(),
        );
    }
    if available_tools == 0 {
        return (
            "blocked".to_string(),
            Tone::Danger,
            format!("Recover or qualify {} capacity", summary.tool_class.label()),
        );
    }
    if matches!(top_slack, Some(slack) if slack <= 0) {
        return (
            "due now".to_string(),
            Tone::Danger,
            "Dispatch the earliest due lot before lower priority work".to_string(),
        );
    }
    if load_per_tool >= 180 {
        return (
            "bottleneck".to_string(),
            Tone::Warning,
            "Pull qualified alternate capacity or split the queue".to_string(),
        );
    }
    if load_per_tool >= 90 {
        return (
            "loaded".to_string(),
            Tone::Info,
            "Keep policy order and monitor the next idle tool".to_string(),
        );
    }
    (
        "balanced".to_string(),
        Tone::Success,
        "Dispatch in policy order".to_string(),
    )
}

fn utilization_for_tool(assignments: &[DispatchAssignment], tool_id: &ToolId) -> u32 {
    let total = assignments
        .iter()
        .filter(|assignment| assignment.tool_id == *tool_id)
        .map(|assignment| {
            assignment
                .finish_minute
                .saturating_sub(assignment.start_minute)
        })
        .sum::<u32>();
    ((total as f32 / (8.0 * 60.0)) * 100.0).round() as u32
}

fn tool_matches_lot(tool: &DispatchTool, lot: &DispatchLot) -> bool {
    tool.class == lot.required_tool_class
        && (tool.compatible_recipes.is_empty()
            || tool
                .compatible_recipes
                .iter()
                .any(|recipe| *recipe == lot.recipe_id))
}

fn recipe_scope(tool: &DispatchTool) -> String {
    if tool.compatible_recipes.is_empty() {
        "all recipes".to_string()
    } else if tool.compatible_recipes.len() == 1 {
        tool.compatible_recipes[0].to_string()
    } else {
        format!("{} recipes", tool.compatible_recipes.len())
    }
}

fn match_count_label(visible: usize, total: usize, filtered: bool) -> String {
    if filtered {
        format!("{visible}/{total} lots")
    } else {
        format!("{total} lots")
    }
}

fn schedule_label(assignment: Option<&DispatchAssignment>) -> String {
    assignment.map_or_else(
        || "unscheduled".to_string(),
        |assignment| {
            format!(
                "{}-{} on {}",
                format_shift_time(assignment.start_minute),
                format_shift_time(assignment.finish_minute),
                assignment.tool_id
            )
        },
    )
}

fn assignment_state(assignment: &DispatchAssignment) -> (String, Tone) {
    if assignment.tardy_minutes > 0 {
        (
            format!("late {} min", assignment.tardy_minutes),
            Tone::Danger,
        )
    } else if assignment.wait_minutes > 60 {
        (
            format!("wait {} min", assignment.wait_minutes),
            Tone::Warning,
        )
    } else {
        ("on time".to_string(), Tone::Success)
    }
}

fn tool_state_tone(state: ToolDispatchState) -> Tone {
    match state {
        ToolDispatchState::Available => Tone::Success,
        ToolDispatchState::Maintenance => Tone::Warning,
        ToolDispatchState::Down => Tone::Danger,
    }
}

fn tool_state_rank(state: ToolDispatchState) -> u8 {
    match state {
        ToolDispatchState::Available => 0,
        ToolDispatchState::Maintenance => 1,
        ToolDispatchState::Down => 2,
    }
}

fn tone_rank(tone: Tone) -> u8 {
    match tone {
        Tone::Danger => 4,
        Tone::Warning => 3,
        Tone::Info => 2,
        Tone::Success => 1,
        Tone::Neutral => 0,
    }
}

fn tone_label(tone: Tone) -> &'static str {
    match tone {
        Tone::Danger => "blocked",
        Tone::Warning => "watch",
        Tone::Info => "next",
        Tone::Success => "ready",
        Tone::Neutral => "queue",
    }
}

fn ceil_div(value: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        value
    } else {
        value.div_ceil(divisor)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_operad_view_audits_common_widths() {
        let mut panel = SchedulerPanel::from_schedule(DispatchSchedule::sample());
        panel.ensure_selection();
        let result = panel.schedule.dispatch(panel.policy);
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width, &result);
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
    fn scheduler_operad_actions_update_panel_state() {
        let mut panel = SchedulerPanel::from_schedule(DispatchSchedule::sample());
        let target_tool = panel.schedule.tools.last().unwrap().id.clone();

        assert_eq!(
            panel.handle_operad_action(&format!(
                "{OPERAD_ACTION_SET_POLICY}{}|test",
                dispatch_policy_slug(DispatchPolicy::DueDateThenPriority)
            )),
            Some("dispatch policy set to due date + priority".to_string())
        );
        assert_eq!(panel.policy, DispatchPolicy::DueDateThenPriority);

        assert_eq!(
            panel.handle_operad_action(&format!(
                "{OPERAD_ACTION_SELECT_TOOL}{}|test",
                target_tool.as_str()
            )),
            Some(format!("selected dispatch tool {target_tool}"))
        );
        assert_eq!(panel.selected_tool.as_ref(), Some(&target_tool));

        assert_eq!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_SET_MIN_PRIORITY}4|test")),
            Some("minimum dispatch priority set to P4".to_string())
        );
        assert_eq!(panel.min_priority, 4);

        assert!(!panel.show_conflicts_only);
        assert_eq!(
            panel.handle_operad_action(OPERAD_ACTION_TOGGLE_CONFLICTS),
            Some("dispatch conflict filter toggled".to_string())
        );
        assert!(panel.show_conflicts_only);

        assert_eq!(
            panel.handle_operad_action(OPERAD_ACTION_RESET_FILTERS),
            Some("dispatch filters reset".to_string())
        );
        assert_eq!(panel.min_priority, 0);
        assert!(!panel.show_conflicts_only);
        assert!(!panel.focus_selected_tool);
    }
}
