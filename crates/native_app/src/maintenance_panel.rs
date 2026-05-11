use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::{
    equipment::ToolId,
    maintenance::{
        CalibrationOutcome, CalibrationRecord, DueState, FabDate, MaintenanceKind,
        MaintenanceModel, QualificationOutcome, ToolReleaseState,
    },
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
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_WORK_FILTER: &str = "maintenance.action.work_filter.";
const OPERAD_ACTION_HISTORY_FILTER: &str = "maintenance.action.history_filter.";
const OPERAD_ACTION_SELECT_TOOL: &str = "maintenance.action.select_tool.";
const OPERAD_ACTION_STATUS: &str = "maintenance.action.status.";

pub(crate) struct MaintenancePanel {
    model: MaintenanceModel,
    selected_tool: Option<ToolId>,
    work_filter: WorkFilter,
    history_filter: HistoryFilter,
}

#[derive(Debug)]
struct MaintenanceOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct MaintenanceMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct MaintenanceOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
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
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        let sections = self.context_sections();
        render_sidecar(ui, "maintenance.context", &sections)
    }

    fn context_sections(&self) -> Vec<SidecarSection> {
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
        let mut sections = vec![
            SidecarSection::new("Maintenance")
                .row(SidecarRow::new(
                    format!("Audit date {today}"),
                    format!("{at_risk} action queue | {overdue} overdue"),
                    if overdue > 0 {
                        Tone::Danger
                    } else if at_risk > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                ))
                .row(SidecarRow::new(
                    "Locked tools",
                    locked.to_string(),
                    if locked > 0 {
                        Tone::Danger
                    } else {
                        Tone::Neutral
                    },
                )),
        ];

        let Some(tool_id) = self.selected_tool.clone() else {
            sections
                .push(SidecarSection::new("Selected Tool").empty("No maintenance tools loaded"));
            return sections;
        };
        let Some(tool) = self.model.tool(&tool_id) else {
            sections.push(SidecarSection::new("Selected Tool").empty("Selected tool is missing"));
            return sections;
        };
        let release = self.model.release_for_tool(&tool_id, today);
        let reason_detail = if release.reasons.is_empty() {
            "No release holds".to_string()
        } else {
            release
                .reasons
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        };
        sections.push(
            SidecarSection::new("Selected Tool")
                .row(
                    SidecarRow::new(
                        &tool.tool_name,
                        format!("{} runs | {}", tool.run_count, release.state.label()),
                        release_tone(release.state),
                    )
                    .selected(true),
                )
                .row(SidecarRow::new(
                    "Release reasons",
                    reason_detail,
                    release_tone(release.state),
                )),
        );

        let next_section = if let Some(next) = self.next_schedule_for_selected(today) {
            let mut section = SidecarSection::new("Next Action")
                .row(SidecarRow::new(
                    &next.task,
                    format!(
                        "Due {} ({})",
                        next.due_date,
                        due_window_label(next.days_until)
                    ),
                    due_state_tone(next.due_state),
                ))
                .row(SidecarRow::new(
                    "Run interval",
                    next.interval_runs
                        .map(|interval| {
                            format!(
                                "{} of {} runs since completion",
                                next.run_count_delta.unwrap_or_default(),
                                interval
                            )
                        })
                        .unwrap_or_else(|| "Calendar based".to_string()),
                    Tone::Neutral,
                ));
            for item in next.checklist.iter().take(3) {
                section = section.row(SidecarRow::new("Checklist", item, Tone::Info));
            }
            section
        } else {
            SidecarSection::new("Next Action").empty("No scheduled work for selected tool")
        };
        sections.push(next_section);
        sections
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
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
        if let Err(error) = self.operad_ui(ui, status) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, status);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui, status: &mut String) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("maintenance_dashboard_operad_scroll")
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
                    && self.handle_operad_action(&node_name, status)
                {
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

    fn build_operad_view(&self, width: f32) -> MaintenanceOperadView {
        let today = self.today();
        let metrics = self.operad_metrics(today);
        let filter_rows = self.operad_filter_rows(today);
        let work_rows = self.operad_work_rows(today);
        let detail_rows = self.operad_selected_tool_rows(today);
        let release_rows = self.operad_release_rows(today);
        let history_filter_rows = self.operad_history_filter_rows();
        let history_rows = self.operad_history_rows();
        let height = maintenance_operad_view_height(
            width,
            metrics.len(),
            &[
                filter_rows.len(),
                work_rows.len(),
                detail_rows.len(),
                release_rows.len(),
                history_filter_rows.len(),
                history_rows.len(),
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

        add_maintenance_operad_header(
            &mut document,
            root,
            "FAB OPERATIONS",
            "Maintenance and Calibration",
            "Release gating, maintenance queue, calibration status, qualification, and audit records",
            &format!(
                "Audit date {today} · {} tools tracked · {} locked",
                self.model.tools.len(),
                self.locked_tool_count(today)
            ),
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_metric_grid(&mut document, root, width, &metrics);
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.filters",
            "Queue Filters",
            "No maintenance filters available",
            &filter_rows,
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.queue",
            "Maintenance Queue",
            "No maintenance tasks match the current filter",
            &work_rows,
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.detail",
            "Tool Detail",
            "No tool selected",
            &detail_rows,
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.release",
            "Release Board",
            "No maintenance tools loaded",
            &release_rows,
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.history_filters",
            "History Scope",
            "No history filters available",
            &history_filter_rows,
        );
        add_maintenance_operad_spacer(&mut document, root, OPERAD_GAP);
        add_maintenance_operad_section(
            &mut document,
            root,
            width,
            "maintenance.history",
            "Maintenance History",
            "No history entries match the scope",
            &history_rows,
        );

        MaintenanceOperadView { document, size }
    }

    fn operad_metrics(&self, today: FabDate) -> Vec<MaintenanceMetricTile> {
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
        vec![
            MaintenanceMetricTile {
                label: "Overdue".to_string(),
                value: overdue.to_string(),
                detail: "release risk".to_string(),
                tone: if overdue > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            },
            MaintenanceMetricTile {
                label: "Due now".to_string(),
                value: due_now.to_string(),
                detail: "needs action".to_string(),
                tone: if due_now > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            },
            MaintenanceMetricTile {
                label: "Due soon".to_string(),
                value: due_soon.to_string(),
                detail: "next 7 days".to_string(),
                tone: Tone::Info,
            },
            MaintenanceMetricTile {
                label: "Calibration watch".to_string(),
                value: calibration_watch.to_string(),
                detail: "due or out of spec".to_string(),
                tone: if calibration_watch > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            },
            MaintenanceMetricTile {
                label: "Locked".to_string(),
                value: locked.to_string(),
                detail: "not production released".to_string(),
                tone: if locked > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            },
        ]
    }

    fn operad_filter_rows(&self, today: FabDate) -> Vec<MaintenanceOperadRow> {
        WorkFilter::ALL
            .into_iter()
            .map(|filter| {
                let count = self
                    .work_items(today)
                    .into_iter()
                    .filter(|work| filter.matches(work))
                    .count();
                MaintenanceOperadRow {
                    title: format!("{} ({count})", filter.label()),
                    detail: if filter == self.work_filter {
                        "Current queue filter".to_string()
                    } else {
                        "Click to filter maintenance work".to_string()
                    },
                    tone: if filter == self.work_filter {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                    action_name: Some(format!("{OPERAD_ACTION_WORK_FILTER}{}", filter.slug())),
                    selected: filter == self.work_filter,
                }
            })
            .collect()
    }

    fn operad_work_rows(&self, today: FabDate) -> Vec<MaintenanceOperadRow> {
        self.filtered_work_items(today)
            .into_iter()
            .enumerate()
            .map(|(index, work)| MaintenanceOperadRow {
                title: format!(
                    "{} · {} · {}",
                    work.tool_id,
                    work.due_state.label(),
                    work.kind.label()
                ),
                detail: format!(
                    "{} · due {} ({}) · release {} · {} checklist item(s)",
                    truncate_middle(&work.task, 36),
                    work.due_date,
                    due_window_label(work.days_until),
                    work.release_state.label().to_lowercase(),
                    work.checklist_len
                ),
                tone: if work.blocked {
                    Tone::Danger
                } else {
                    due_state_tone(work.due_state)
                },
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_TOOL}{}|queue.{index}",
                    work.tool_id
                )),
                selected: self.selected_tool.as_ref() == Some(&work.tool_id),
            })
            .collect()
    }

    fn operad_selected_tool_rows(&self, today: FabDate) -> Vec<MaintenanceOperadRow> {
        let Some(tool_id) = self.selected_tool.as_ref() else {
            return Vec::new();
        };
        let Some(tool) = self.model.tool(tool_id) else {
            return Vec::new();
        };
        let release = self.model.release_for_tool(tool_id, today);
        let mut rows = vec![MaintenanceOperadRow {
            title: tool.tool_name.clone(),
            detail: format!(
                "{} · {} runs · {}",
                tool_id,
                tool.run_count,
                release.state.label()
            ),
            tone: release_tone(release.state),
            action_name: Some(format!("{OPERAD_ACTION_SELECT_TOOL}{tool_id}|detail")),
            selected: true,
        }];
        if release.reasons.is_empty() {
            rows.push(MaintenanceOperadRow {
                title: "Release context".to_string(),
                detail: "No active holds".to_string(),
                tone: Tone::Success,
                action_name: None,
                selected: false,
            });
        } else {
            for (index, reason) in release.reasons.iter().enumerate() {
                rows.push(MaintenanceOperadRow {
                    title: format!("Hold {}", index + 1),
                    detail: reason.clone(),
                    tone: Tone::Danger,
                    action_name: None,
                    selected: false,
                });
            }
        }
        rows.push(MaintenanceOperadRow {
            title: "Schedule maintenance".to_string(),
            detail: format!("Open scheduling workflow for {tool_id}"),
            tone: Tone::Info,
            action_name: Some(format!("{OPERAD_ACTION_STATUS}schedule|{tool_id}")),
            selected: false,
        });
        let calibration_enabled = tool
            .schedules
            .iter()
            .any(|schedule| schedule.kind == MaintenanceKind::Calibration)
            || release.state == ToolReleaseState::CalibrationLockout;
        if calibration_enabled {
            rows.push(MaintenanceOperadRow {
                title: "Capture calibration".to_string(),
                detail: format!("Open calibration capture for {tool_id}"),
                tone: Tone::Warning,
                action_name: Some(format!("{OPERAD_ACTION_STATUS}calibration|{tool_id}")),
                selected: false,
            });
        }
        if !release.state.released_to_production() {
            rows.push(MaintenanceOperadRow {
                title: "Resolve release hold".to_string(),
                detail: format!("Open release review for {tool_id}"),
                tone: Tone::Danger,
                action_name: Some(format!("{OPERAD_ACTION_STATUS}release|{tool_id}")),
                selected: false,
            });
        }
        if let Some(next) = self.next_schedule_for_tool(tool_id, today) {
            rows.push(MaintenanceOperadRow {
                title: format!("Next work · {}", next.due_state.label()),
                detail: format!(
                    "{} · due {} ({})",
                    next.task,
                    next.due_date,
                    due_window_label(next.days_until)
                ),
                tone: due_state_tone(next.due_state),
                action_name: None,
                selected: false,
            });
            if !next.checklist.is_empty() {
                rows.push(MaintenanceOperadRow {
                    title: "Checklist".to_string(),
                    detail: next
                        .checklist
                        .iter()
                        .take(4)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" · "),
                    tone: Tone::Neutral,
                    action_name: None,
                    selected: false,
                });
            }
        }
        if let Some(record) = self.latest_calibration(tool_id) {
            rows.push(MaintenanceOperadRow {
                title: "Latest calibration".to_string(),
                detail: format!(
                    "{} · {} · {:.2} against {} · {}",
                    record.performed_at,
                    record.parameter,
                    record.measured_value,
                    record.tolerance,
                    record.technician
                ),
                tone: calibration_tone(record.outcome),
                action_name: None,
                selected: false,
            });
        }
        if let Some(result) = self
            .model
            .qualification_results_for(tool_id)
            .into_iter()
            .max_by_key(|result| result.performed_at)
        {
            rows.push(MaintenanceOperadRow {
                title: "Latest qualification".to_string(),
                detail: format!(
                    "{} · {} · {} {:.2} ({})",
                    result.performed_at, result.wafer_id, result.metric, result.value, result.spec
                ),
                tone: qualification_tone(result.outcome),
                action_name: None,
                selected: false,
            });
        }
        rows
    }

    fn operad_release_rows(&self, today: FabDate) -> Vec<MaintenanceOperadRow> {
        self.model
            .tools
            .iter()
            .enumerate()
            .map(|(index, tool)| {
                let release = self.model.release_for_tool(&tool.tool_id, today);
                MaintenanceOperadRow {
                    title: format!("{} · {}", tool.tool_name, release.state.label()),
                    detail: if release.reasons.is_empty() {
                        format!(
                            "{} · {} runs · no release holds",
                            tool.tool_id, tool.run_count
                        )
                    } else {
                        format!(
                            "{} · {} runs · {}",
                            tool.tool_id,
                            tool.run_count,
                            release
                                .reasons
                                .iter()
                                .take(2)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(" · ")
                        )
                    },
                    tone: release_tone(release.state),
                    action_name: Some(format!(
                        "{OPERAD_ACTION_SELECT_TOOL}{}|release.{index}",
                        tool.tool_id
                    )),
                    selected: self.selected_tool.as_ref() == Some(&tool.tool_id),
                }
            })
            .collect()
    }

    fn operad_history_filter_rows(&self) -> Vec<MaintenanceOperadRow> {
        HistoryFilter::ALL
            .into_iter()
            .map(|filter| MaintenanceOperadRow {
                title: filter.label().to_string(),
                detail: if filter == self.history_filter {
                    "Current history scope".to_string()
                } else {
                    "Click to change history scope".to_string()
                },
                tone: if filter == self.history_filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                action_name: Some(format!("{OPERAD_ACTION_HISTORY_FILTER}{}", filter.slug())),
                selected: filter == self.history_filter,
            })
            .collect()
    }

    fn operad_history_rows(&self) -> Vec<MaintenanceOperadRow> {
        self.history_entries()
            .into_iter()
            .take(14)
            .enumerate()
            .map(|(index, entry)| MaintenanceOperadRow {
                title: format!("{} · {} · {}", entry.date, entry.kind, entry.tool_id),
                detail: format!("{} · {}", entry.label, entry.detail),
                tone: entry.tone,
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_TOOL}{}|history.{index}",
                    entry.tool_id
                )),
                selected: self.selected_tool.as_ref() == Some(&entry.tool_id),
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str, status: &mut String) -> bool {
        if let Some(slug) = node_name.strip_prefix(OPERAD_ACTION_WORK_FILTER)
            && let Some(filter) = WorkFilter::from_slug(slug)
        {
            self.work_filter = filter;
            *status = format!("maintenance queue filter: {}", filter.label());
            return true;
        }
        if let Some(slug) = node_name.strip_prefix(OPERAD_ACTION_HISTORY_FILTER)
            && let Some(filter) = HistoryFilter::from_slug(slug)
        {
            self.history_filter = filter;
            *status = format!("maintenance history scope: {}", filter.label());
            return true;
        }
        if let Some(tool_id) = node_name.strip_prefix(OPERAD_ACTION_SELECT_TOOL) {
            let tool_id = tool_id
                .split_once('|')
                .map(|(tool_id, _)| tool_id)
                .unwrap_or(tool_id);
            let tool_id = ToolId::new(tool_id.to_string());
            if self.model.tool(&tool_id).is_some() {
                self.selected_tool = Some(tool_id.clone());
                *status = format!("selected maintenance tool {tool_id}");
                return true;
            }
        }
        if let Some(action) = node_name.strip_prefix(OPERAD_ACTION_STATUS) {
            let Some((kind, tool_id)) = action.split_once('|') else {
                return false;
            };
            let tool_id = ToolId::new(tool_id.to_string());
            if self.model.tool(&tool_id).is_none() {
                return false;
            }
            *status = match kind {
                "schedule" => format!("maintenance scheduling opened for {tool_id}"),
                "calibration" => format!("calibration capture opened for {tool_id}"),
                "release" => format!("release hold review opened for {tool_id}"),
                _ => return false,
            };
            return true;
        }
        false
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

fn maintenance_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += maintenance_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += maintenance_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn maintenance_operad_metric_columns(width: f32) -> usize {
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

fn maintenance_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = maintenance_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn maintenance_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_maintenance_operad_header(
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
            "maintenance.header",
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
    add_maintenance_operad_text(
        document,
        header,
        "maintenance.header.eyebrow",
        eyebrow,
        maintenance_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_maintenance_operad_text(
        document,
        header,
        "maintenance.header.title",
        title,
        maintenance_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_maintenance_operad_text(
        document,
        header,
        "maintenance.header.detail",
        detail,
        maintenance_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_maintenance_operad_text(
        document,
        header,
        "maintenance.header.meta",
        meta,
        maintenance_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_maintenance_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[MaintenanceMetricTile],
) {
    let columns = maintenance_operad_metric_columns(width);
    let grid_height = maintenance_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "maintenance.metrics",
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
                format!("maintenance.metrics.row.{row_index}"),
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
            add_maintenance_operad_metric_tile(
                document,
                row,
                &format!("maintenance.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_maintenance_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &MaintenanceMetricTile,
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
                maintenance_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_maintenance_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        maintenance_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_maintenance_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        maintenance_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_maintenance_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        maintenance_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            maintenance_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_maintenance_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[MaintenanceOperadRow],
) {
    let height = maintenance_operad_section_height(rows.len());
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
    add_maintenance_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        maintenance_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_maintenance_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_maintenance_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_maintenance_operad_empty_row(
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
    add_maintenance_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        maintenance_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_maintenance_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &MaintenanceOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        maintenance_operad_tone_color(Tone::Info)
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
            maintenance_operad_tone_color(row.tone),
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
    add_maintenance_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        maintenance_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_maintenance_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        maintenance_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_maintenance_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("maintenance.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_maintenance_operad_text(
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

fn maintenance_operad_text_style(
    font_size: f32,
    weight: FontWeight,
    color: ColorRgba,
) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn maintenance_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
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

    fn slug(self) -> &'static str {
        match self {
            Self::Actionable => "actionable",
            Self::Upcoming => "upcoming",
            Self::Calibration => "calibration",
            Self::Locked => "locked",
            Self::All => "all",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "actionable" => Some(Self::Actionable),
            "upcoming" => Some(Self::Upcoming),
            "calibration" => Some(Self::Calibration),
            "locked" => Some(Self::Locked),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    fn matches(self, work: &WorkItem) -> bool {
        match self {
            Self::Actionable => work.blocked || work.due_state.blocks_release(),
            Self::Upcoming => work.due_state != DueState::Overdue && work.days_until <= 14,
            Self::Calibration => {
                work.kind == MaintenanceKind::Calibration
                    || work.release_state == ToolReleaseState::CalibrationLockout
            }
            Self::Locked => work.blocked,
            Self::All => true,
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

    fn slug(self) -> &'static str {
        match self {
            Self::SelectedTool => "selected",
            Self::AllTools => "all",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "selected" => Some(Self::SelectedTool),
            "all" => Some(Self::AllTools),
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_operad_view_audits_common_widths() {
        let panel = MaintenancePanel::from_model(MaintenanceModel::sample());
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
    fn maintenance_operad_actions_update_panel_state() {
        let mut panel = MaintenancePanel::from_model(MaintenanceModel::sample());
        let mut status = String::new();

        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_WORK_FILTER}calibration"),
            &mut status
        ));
        assert_eq!(panel.work_filter, WorkFilter::Calibration);
        assert!(status.contains("Calibration"));

        assert!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_HISTORY_FILTER}all"), &mut status)
        );
        assert_eq!(panel.history_filter, HistoryFilter::AllTools);
        assert!(status.contains("All tools"));

        let target_tool = panel.model.tools.last().unwrap().tool_id.clone();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_TOOL}{target_tool}|test"),
            &mut status
        ));
        assert_eq!(panel.selected_tool.as_ref(), Some(&target_tool));
        assert!(status.contains(&target_tool.to_string()));

        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_STATUS}schedule|{target_tool}"),
            &mut status
        ));
        assert!(status.contains("maintenance scheduling opened"));
    }
}
