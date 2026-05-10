use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::process_flow::{
    ProcessFlowEdge, ProcessFlowEdgeKind, ProcessFlowFinding, ProcessFlowFindingSeverity,
    ProcessFlowModel, ProcessFlowNode, ProcessFlowNodeId, ProcessFlowNodeKind,
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
const OPERAD_ACTION_FILTER: &str = "process_flow.action.filter.";
const OPERAD_ACTION_SELECT_NODE: &str = "process_flow.action.select_node.";
const OPERAD_ACTION_TOGGLE_ERRORS: &str = "process_flow.action.toggle_errors";
const OPERAD_ACTION_EXPORT: &str = "process_flow.action.export";
const OPERAD_ACTION_VALIDATE: &str = "process_flow.action.validate";

pub(crate) struct ProcessFlowPanel {
    model: ProcessFlowModel,
    selected_node: Option<ProcessFlowNodeId>,
    show_errors_only: bool,
    node_filter: ProcessFlowNodeFilter,
}

#[derive(Debug)]
struct ProcessFlowOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct ProcessFlowMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct ProcessFlowOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessFlowNodeFilter {
    All,
    RecipeSteps,
    Metrology,
    Holds,
    Rework,
}

impl ProcessFlowNodeFilter {
    const ALL: [Self; 5] = [
        Self::All,
        Self::RecipeSteps,
        Self::Metrology,
        Self::Holds,
        Self::Rework,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::RecipeSteps => "Recipes",
            Self::Metrology => "Metrology",
            Self::Holds => "Holds",
            Self::Rework => "Rework",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::All => "Full route traveler",
            Self::RecipeSteps => "Recipe-bound process work",
            Self::Metrology => "Measurement and sample-plan checkpoints",
            Self::Holds => "Engineer signoff and explicit hold points",
            Self::Rework => "Nodes that send or receive rework loops",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::RecipeSteps => "recipes",
            Self::Metrology => "metrology",
            Self::Holds => "holds",
            Self::Rework => "rework",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "all" => Some(Self::All),
            "recipes" => Some(Self::RecipeSteps),
            "metrology" => Some(Self::Metrology),
            "holds" => Some(Self::Holds),
            "rework" => Some(Self::Rework),
            _ => None,
        }
    }
}

impl ProcessFlowPanel {
    pub(crate) fn from_model(model: ProcessFlowModel) -> Self {
        let selected_node = model.route.nodes.first().map(|node| node.id.clone());
        Self {
            model,
            selected_node,
            show_errors_only: false,
            node_filter: ProcessFlowNodeFilter::All,
        }
    }

    pub(crate) fn model(&self) -> &ProcessFlowModel {
        &self.model
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        if self.operad_context_ui(ui, status).is_err() {
            self.egui_context_ui(ui, status);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) -> Result<(), String> {
        self.ensure_selection();
        let findings = self.model.findings();
        let error_count = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let warning_count = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Warning)
            .count();

        let mut sections = Vec::new();
        sections.push(
            SidecarSection::new("Process Flow")
                .row(SidecarRow::new(
                    self.model.route.name.clone(),
                    format!("{} v{}", self.model.route.id, self.model.route.version),
                    Tone::Info,
                ))
                .row(SidecarRow::new(
                    "Topology",
                    format!(
                        "{} nodes / {} edges",
                        self.model.route.nodes.len(),
                        self.model.route.edges.len()
                    ),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    if error_count == 0 {
                        "Exportable"
                    } else {
                        "Blocked"
                    },
                    format!("{error_count} errors / {warning_count} warnings"),
                    if error_count == 0 {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                )),
        );

        sections.push(
            SidecarSection::new("MES Export")
                .row(SidecarRow::new(
                    "Route",
                    self.model.route.mes_route_id.clone(),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Mask",
                    self.model.route.mask_design_id.clone(),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Layout",
                    self.model.route.layout_revision.clone(),
                    Tone::Neutral,
                ))
                .row(
                    SidecarRow::new(
                        "Prepare MES-lite route export",
                        "Build traveler steps for the current route",
                        Tone::Info,
                    )
                    .action(OPERAD_ACTION_EXPORT),
                ),
        );

        let mut validation = SidecarSection::new("Validation")
            .row(
                SidecarRow::new(
                    if self.show_errors_only {
                        "Showing errors only"
                    } else {
                        "Showing all findings"
                    },
                    "Toggle validation filter",
                    Tone::Neutral,
                )
                .action(OPERAD_ACTION_TOGGLE_ERRORS),
            )
            .empty("No process-flow findings");
        for (index, finding) in findings.iter().take(5).enumerate() {
            let action = finding.node_id.as_ref().map(|node_id| {
                format!("{OPERAD_ACTION_SELECT_NODE}{node_id}|context.finding.{index}")
            });
            let mut row = SidecarRow::new(
                format!("{} · {}", finding.severity.label(), finding.code),
                finding.message.clone(),
                finding_tone(finding.severity),
            );
            if let Some(action) = action {
                row = row.action(action);
            }
            validation = validation.row(row);
        }
        sections.push(validation);

        let mut history = SidecarSection::new("Version History").empty("No route versions");
        for version in self.model.versions.iter().rev().take(4) {
            history = history.row(SidecarRow::new(
                format!("v{} {}", version.version, version.timestamp),
                format!(
                    "{} nodes / {} edges by {}",
                    version.node_count, version.edge_count, version.author
                ),
                Tone::Neutral,
            ));
        }
        sections.push(history);

        if let Some(action) = render_sidecar_interactive(ui, "process_flow.context", &sections)? {
            self.handle_operad_action(&action, status);
        }
        Ok(())
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        let findings = self.model.findings();
        let error_count = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        ui_chrome::section_label(ui, "Process Flow");
        ui.label(&self.model.route.name);
        ui.small(format!(
            "{}  v{}",
            self.model.route.id, self.model.route.version
        ));
        self.compact_route_status_ui(ui, &findings);
        ui.separator();

        ui_chrome::section_label(ui, "MES Export");
        ui.label(format!("Route: {}", self.model.route.mes_route_id));
        ui.label(format!("Mask: {}", self.model.route.mask_design_id));
        ui.label(format!("Layout: {}", self.model.route.layout_revision));
        if ui.button("Prepare MES-lite route export").clicked() {
            let mes_route = self.model.export_to_mes_route();
            *status = format!(
                "prepared {} MES steps for {}",
                mes_route.steps.len(),
                mes_route.id
            );
        }

        ui.separator();
        ui_chrome::section_label(ui, "Validation");
        ui_chrome::status_pill(
            ui,
            if error_count == 0 {
                "exportable"
            } else {
                "blocked"
            },
            if error_count == 0 {
                Tone::Success
            } else {
                Tone::Danger
            },
        );
        ui.checkbox(&mut self.show_errors_only, "Errors only");
        self.finding_list_ui(ui, &findings, 180.0);

        ui.separator();
        ui_chrome::section_label(ui, "Version History");
        for version in self.model.versions.iter().rev().take(4) {
            ui.group(|ui| {
                ui.strong(format!("v{} {}", version.version, version.timestamp));
                ui.small(format!(
                    "{} nodes / {} edges by {}",
                    version.node_count, version.edge_count, version.author
                ));
                ui.label(&version.change_note);
            });
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
            .id_salt("process_flow_dashboard_operad_scroll")
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
        let findings = self.model.findings();
        egui::ScrollArea::vertical()
            .id_salt("process_flow_dashboard")
            .show(ui, |ui| {
                let detail = format!(
                    "Owner: {} | {} v{}",
                    self.model.route.owner, self.model.route.id, self.model.route.version
                );
                ui_chrome::module_header(
                    ui,
                    "Process engineering",
                    "Process-flow Designer",
                    &detail,
                    |ui| {
                        if ui.button("Prepare MES-lite route export").clicked() {
                            let mes_route = self.model.export_to_mes_route();
                            *status = format!(
                                "prepared {} MES steps for {}",
                                mes_route.steps.len(),
                                mes_route.id
                            );
                        }
                        if ui.button("Validate").clicked() {
                            let error_count = findings
                                .iter()
                                .filter(|finding| {
                                    finding.severity == ProcessFlowFindingSeverity::Error
                                })
                                .count();
                            *status =
                                format!("process flow validation found {} issue(s)", error_count);
                        }
                    },
                );

                self.summary_ui(ui, &findings);
                self.filter_bar_ui(ui);
                ui.separator();

                let available_width = ui.available_width();
                if available_width < 760.0 {
                    ui_chrome::section_label(ui, "Route Timeline");
                    self.route_timeline_ui(ui, &findings, 360.0);
                    ui.separator();
                    ui_chrome::section_label(ui, "Selected Step");
                    self.selected_node_ui(ui, &findings);
                    ui.separator();
                    ui_chrome::section_label(ui, "Dependencies");
                    self.dependency_ui(ui);
                    ui.separator();
                    ui_chrome::section_label(ui, "Controls");
                    self.controls_overview_ui(ui);
                    ui.separator();
                    ui_chrome::section_label(ui, "Findings");
                    self.finding_list_ui(ui, &findings, 240.0);
                } else if available_width < 1120.0 {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(320.0);
                        columns[1].set_min_width(360.0);

                        ui_chrome::section_label(&mut columns[0], "Route Timeline");
                        self.route_timeline_ui(&mut columns[0], &findings, 560.0);
                        columns[0].separator();
                        ui_chrome::section_label(&mut columns[0], "Controls");
                        self.controls_overview_ui(&mut columns[0]);

                        ui_chrome::section_label(&mut columns[1], "Selected Step");
                        self.selected_node_ui(&mut columns[1], &findings);
                        columns[1].separator();
                        ui_chrome::section_label(&mut columns[1], "Dependencies");
                        self.dependency_ui(&mut columns[1]);
                        columns[1].separator();
                        ui_chrome::section_label(&mut columns[1], "Findings");
                        self.finding_list_ui(&mut columns[1], &findings, 260.0);
                    });
                } else {
                    ui.columns(3, |columns| {
                        columns[0].set_min_width(320.0);
                        columns[1].set_min_width(360.0);
                        columns[2].set_min_width(320.0);

                        ui_chrome::section_label(&mut columns[0], "Route Timeline");
                        self.route_timeline_ui(&mut columns[0], &findings, 600.0);

                        ui_chrome::section_label(&mut columns[1], "Selected Step");
                        self.selected_node_ui(&mut columns[1], &findings);

                        ui_chrome::section_label(&mut columns[2], "Dependencies");
                        self.dependency_ui(&mut columns[2]);
                        columns[2].separator();
                        ui_chrome::section_label(&mut columns[2], "Controls");
                        self.controls_overview_ui(&mut columns[2]);
                        columns[2].separator();
                        ui_chrome::section_label(&mut columns[2], "Findings");
                        self.finding_list_ui(&mut columns[2], &findings, 240.0);
                    });
                }

                ui.separator();
                self.mes_export_preview_ui(ui);
            });
    }

    fn build_operad_view(&self, width: f32) -> ProcessFlowOperadView {
        let findings = self.model.findings();
        let metrics = self.operad_metrics(&findings);
        let action_rows = self.operad_action_rows(&findings);
        let filter_rows = self.operad_filter_rows();
        let route_rows = self.operad_route_rows(&findings);
        let selected_rows = self.operad_selected_rows(&findings);
        let dependency_rows = self.operad_dependency_rows();
        let control_rows = self.operad_control_rows();
        let finding_rows = self.operad_finding_rows(&findings);
        let export_rows = self.operad_export_rows();
        let height = process_flow_operad_view_height(
            width,
            metrics.len(),
            &[
                action_rows.len(),
                filter_rows.len(),
                route_rows.len(),
                selected_rows.len(),
                dependency_rows.len(),
                control_rows.len(),
                finding_rows.len(),
                export_rows.len(),
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

        add_process_flow_operad_header(
            &mut document,
            root,
            "PROCESS ENGINEERING",
            "Process-flow Designer",
            "Route graph, MES export readiness, controls, dependencies, and validation findings",
            &format!(
                "Owner: {} · {} v{}",
                self.model.route.owner, self.model.route.id, self.model.route.version
            ),
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_metric_grid(&mut document, root, width, &metrics);
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.actions",
            "Actions",
            "No process-flow actions available",
            &action_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.filters",
            "Route Filters",
            "No filters available",
            &filter_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.route",
            "Route Timeline",
            "No route steps match this filter",
            &route_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.selected",
            "Selected Step",
            "No node selected",
            &selected_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.dependencies",
            "Dependencies",
            "Select a node to inspect dependencies",
            &dependency_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.controls",
            "Controls",
            "No holds, rework paths, or checkpoints",
            &control_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.findings",
            "Findings",
            if self.show_errors_only {
                "No error findings"
            } else {
                "No validation findings"
            },
            &finding_rows,
        );
        add_process_flow_operad_spacer(&mut document, root, OPERAD_GAP);
        add_process_flow_operad_section(
            &mut document,
            root,
            width,
            "process_flow.export",
            "MES-lite Export Preview",
            "No MES export steps",
            &export_rows,
        );

        ProcessFlowOperadView { document, size }
    }

    fn operad_metrics(&self, findings: &[ProcessFlowFinding]) -> Vec<ProcessFlowMetricTile> {
        let recipe_required = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .count();
        let recipe_ready = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .filter(|node| node.recipe.is_some() && node.has_tool_coverage())
            .count();
        let checkpoint_count = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count();
        let hold_points = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();
        let rework_edges = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .count();
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let warnings = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Warning)
            .count();
        vec![
            ProcessFlowMetricTile {
                label: "Route graph".to_string(),
                value: format!(
                    "{} nodes / {} edges",
                    self.model.route.nodes.len(),
                    self.model.route.edges.len()
                ),
                detail: "topology".to_string(),
                tone: Tone::Neutral,
            },
            ProcessFlowMetricTile {
                label: "Recipes".to_string(),
                value: format!("{recipe_ready}/{recipe_required} ready"),
                detail: "bound and tool-covered".to_string(),
                tone: if recipe_ready == recipe_required {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            },
            ProcessFlowMetricTile {
                label: "Metrology".to_string(),
                value: format!("{checkpoint_count} checkpoints"),
                detail: "sample plans".to_string(),
                tone: if checkpoint_count > 0 {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            },
            ProcessFlowMetricTile {
                label: "Holds / rework".to_string(),
                value: format!("{hold_points} holds / {rework_edges} paths"),
                detail: "release controls".to_string(),
                tone: if hold_points > 0 || rework_edges > 0 {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            },
            ProcessFlowMetricTile {
                label: "Validation".to_string(),
                value: if errors == 0 {
                    format!("{warnings} warnings")
                } else {
                    format!("{errors} errors")
                },
                detail: "recipe and eligible-tool coverage".to_string(),
                tone: if errors == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            },
        ]
    }

    fn operad_action_rows(&self, findings: &[ProcessFlowFinding]) -> Vec<ProcessFlowOperadRow> {
        let error_count = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let mes_route = self.model.export_to_mes_route();
        vec![
            ProcessFlowOperadRow {
                title: "Prepare MES-lite route export".to_string(),
                detail: format!(
                    "Prepare {} MES steps for {}",
                    mes_route.steps.len(),
                    mes_route.id
                ),
                tone: Tone::Info,
                action_name: Some(OPERAD_ACTION_EXPORT.to_string()),
                selected: false,
            },
            ProcessFlowOperadRow {
                title: "Validate".to_string(),
                detail: format!("Report {error_count} blocking issue(s)"),
                tone: if error_count == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
                action_name: Some(OPERAD_ACTION_VALIDATE.to_string()),
                selected: false,
            },
            ProcessFlowOperadRow {
                title: "Errors only".to_string(),
                detail: if self.show_errors_only {
                    "Showing error findings only".to_string()
                } else {
                    "Showing all validation findings".to_string()
                },
                tone: if self.show_errors_only {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
                action_name: Some(OPERAD_ACTION_TOGGLE_ERRORS.to_string()),
                selected: self.show_errors_only,
            },
        ]
    }

    fn operad_filter_rows(&self) -> Vec<ProcessFlowOperadRow> {
        ProcessFlowNodeFilter::ALL
            .into_iter()
            .map(|filter| {
                let count = self.count_nodes_for_filter(filter);
                ProcessFlowOperadRow {
                    title: format!("{} ({count})", filter.label()),
                    detail: filter.detail().to_string(),
                    tone: if filter == self.node_filter {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                    action_name: Some(format!("{OPERAD_ACTION_FILTER}{}", filter.slug())),
                    selected: filter == self.node_filter,
                }
            })
            .collect()
    }

    fn operad_route_rows(&self, findings: &[ProcessFlowFinding]) -> Vec<ProcessFlowOperadRow> {
        self.visible_nodes()
            .into_iter()
            .enumerate()
            .map(|(visible_index, (index, node))| {
                let issue_count = self.node_findings(&node.id, findings).len();
                let incoming_count = self.incoming_edges(&node.id).len();
                let outgoing_count = self.outgoing_edges(&node.id).len();
                let rework_count = self.rework_edge_count_for_node(&node.id);
                let mut details = vec![format!(
                    "{} · {} in / {} out",
                    node.area, incoming_count, outgoing_count
                )];
                if node.kind.requires_recipe() {
                    details.push(
                        node.recipe
                            .as_ref()
                            .map(|recipe| format!("recipe {recipe}"))
                            .unwrap_or_else(|| "unassigned recipe".to_string()),
                    );
                }
                if node.hold_point {
                    details.push("hold".to_string());
                }
                if node.measurement_checkpoint.is_some() {
                    details.push("checkpoint".to_string());
                }
                if rework_count > 0 {
                    details.push(format!("{rework_count} rework path(s)"));
                }
                if issue_count > 0 {
                    details.push(format!("{issue_count} finding(s)"));
                }
                ProcessFlowOperadRow {
                    title: format!("{:02}. {} · {}", index + 1, node.name, node.kind.label()),
                    detail: details.join(" · "),
                    tone: if issue_count > 0 {
                        Tone::Danger
                    } else {
                        tone_for_kind(node.kind)
                    },
                    action_name: Some(format!(
                        "{OPERAD_ACTION_SELECT_NODE}{}|route.{visible_index}",
                        node.id
                    )),
                    selected: self.selected_node.as_ref() == Some(&node.id),
                }
            })
            .collect()
    }

    fn operad_selected_rows(&self, findings: &[ProcessFlowFinding]) -> Vec<ProcessFlowOperadRow> {
        let Some(node) = self.selected_node() else {
            return Vec::new();
        };
        let mut rows = vec![ProcessFlowOperadRow {
            title: format!("{} · {}", node.name, node.kind.label()),
            detail: format!("{} · {}", node.id, node.area),
            tone: tone_for_kind(node.kind),
            action_name: Some(format!("{OPERAD_ACTION_SELECT_NODE}{}|selected", node.id)),
            selected: true,
        }];
        if node.kind.requires_recipe() {
            rows.push(ProcessFlowOperadRow {
                title: "Recipe and equipment".to_string(),
                detail: format!(
                    "{} · {}",
                    node.recipe
                        .as_ref()
                        .map(|recipe| format!("recipe {recipe}"))
                        .unwrap_or_else(|| "recipe unassigned".to_string()),
                    if node.has_tool_coverage() {
                        "tool coverage"
                    } else {
                        "tools missing"
                    }
                ),
                tone: if node.recipe.is_some() && node.has_tool_coverage() {
                    Tone::Success
                } else {
                    Tone::Danger
                },
                action_name: None,
                selected: false,
            });
        }
        if !node.allowed_tool_classes.is_empty() {
            rows.push(ProcessFlowOperadRow {
                title: "Allowed classes".to_string(),
                detail: node
                    .allowed_tool_classes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        if !node.eligible_tools.is_empty() {
            rows.push(ProcessFlowOperadRow {
                title: "Eligible tools".to_string(),
                detail: node.eligible_tools.join(", "),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        if !node.expected_inputs.is_empty() {
            rows.push(ProcessFlowOperadRow {
                title: "Inputs".to_string(),
                detail: node.expected_inputs.join(" · "),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        if !node.expected_outputs.is_empty() {
            rows.push(ProcessFlowOperadRow {
                title: "Outputs".to_string(),
                detail: node.expected_outputs.join(" · "),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        if let Some(checkpoint) = &node.measurement_checkpoint {
            rows.push(ProcessFlowOperadRow {
                title: "Measurement checkpoint".to_string(),
                detail: format!(
                    "{} · target {} · {}",
                    checkpoint.measurement_name, checkpoint.target, checkpoint.sample_plan
                ),
                tone: Tone::Success,
                action_name: None,
                selected: false,
            });
        }
        for finding in self.node_findings(&node.id, findings) {
            rows.push(ProcessFlowOperadRow {
                title: format!("{} · {}", finding.severity.label(), finding.code),
                detail: finding.message.clone(),
                tone: finding_tone(finding.severity),
                action_name: None,
                selected: false,
            });
        }
        if !node.notes.is_empty() {
            rows.push(ProcessFlowOperadRow {
                title: "Notes".to_string(),
                detail: node.notes.clone(),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        rows
    }

    fn operad_dependency_rows(&self) -> Vec<ProcessFlowOperadRow> {
        let Some(selected_id) = self.selected_node.clone() else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (index, edge) in self.incoming_edges(&selected_id).into_iter().enumerate() {
            rows.push(ProcessFlowOperadRow {
                title: format!("Prerequisite · {}", edge.kind.label()),
                detail: format!(
                    "{} → {} · {}",
                    self.node_label(&edge.from),
                    self.node_label(&edge.to),
                    if edge.condition.is_empty() {
                        "always"
                    } else {
                        &edge.condition
                    }
                ),
                tone: edge_tone(edge.kind),
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_NODE}{}|incoming.{index}",
                    edge.from
                )),
                selected: false,
            });
        }
        for (index, edge) in self.outgoing_edges(&selected_id).into_iter().enumerate() {
            rows.push(ProcessFlowOperadRow {
                title: format!("Outcome · {}", edge.kind.label()),
                detail: format!(
                    "{} → {} · {}",
                    self.node_label(&edge.from),
                    self.node_label(&edge.to),
                    if edge.condition.is_empty() {
                        "always"
                    } else {
                        &edge.condition
                    }
                ),
                tone: edge_tone(edge.kind),
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_NODE}{}|outgoing.{index}",
                    edge.to
                )),
                selected: false,
            });
        }
        rows
    }

    fn operad_control_rows(&self) -> Vec<ProcessFlowOperadRow> {
        let mut rows = Vec::new();
        for (index, node) in self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .enumerate()
        {
            if let Some(checkpoint) = &node.measurement_checkpoint {
                rows.push(ProcessFlowOperadRow {
                    title: format!("Metrology · {}", checkpoint.measurement_name),
                    detail: format!(
                        "{} · target {} · {}",
                        node.name, checkpoint.target, checkpoint.sample_plan
                    ),
                    tone: Tone::Success,
                    action_name: Some(format!(
                        "{OPERAD_ACTION_SELECT_NODE}{}|checkpoint.{index}",
                        node.id
                    )),
                    selected: self.selected_node.as_ref() == Some(&node.id),
                });
            }
        }
        for (index, node) in self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
            .enumerate()
        {
            rows.push(ProcessFlowOperadRow {
                title: format!("Hold · {}", node.name),
                detail: if node.notes.is_empty() {
                    format!("{} · {}", node.id, node.area)
                } else {
                    format!("{} · {} · {}", node.id, node.area, node.notes)
                },
                tone: Tone::Warning,
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_NODE}{}|hold.{index}",
                    node.id
                )),
                selected: self.selected_node.as_ref() == Some(&node.id),
            });
        }
        for (index, edge) in self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .enumerate()
        {
            rows.push(ProcessFlowOperadRow {
                title: "Rework path".to_string(),
                detail: format!(
                    "{} → {}{}",
                    self.node_label(&edge.from),
                    self.node_label(&edge.to),
                    if edge.condition.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", edge.condition)
                    }
                ),
                tone: Tone::Danger,
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_NODE}{}|rework.{index}",
                    edge.from
                )),
                selected: false,
            });
        }
        rows
    }

    fn operad_finding_rows(&self, findings: &[ProcessFlowFinding]) -> Vec<ProcessFlowOperadRow> {
        findings
            .iter()
            .filter(|finding| {
                !self.show_errors_only || finding.severity == ProcessFlowFindingSeverity::Error
            })
            .enumerate()
            .map(|(index, finding)| ProcessFlowOperadRow {
                title: format!("{} · {}", finding.severity.label(), finding.code),
                detail: format!(
                    "{}{}{}",
                    finding.message,
                    finding
                        .node_id
                        .as_ref()
                        .map(|node_id| format!(" · node {node_id}"))
                        .unwrap_or_default(),
                    finding
                        .edge_id
                        .as_ref()
                        .map(|edge_id| format!(" · edge {edge_id}"))
                        .unwrap_or_default()
                ),
                tone: finding_tone(finding.severity),
                action_name: finding
                    .node_id
                    .as_ref()
                    .map(|node_id| format!("{OPERAD_ACTION_SELECT_NODE}{node_id}|finding.{index}")),
                selected: finding.node_id.as_ref() == self.selected_node.as_ref(),
            })
            .collect()
    }

    fn operad_export_rows(&self) -> Vec<ProcessFlowOperadRow> {
        self.model
            .export_to_mes_route()
            .steps
            .into_iter()
            .map(|step| {
                let mut controls = Vec::new();
                if step.signoff_required {
                    controls.push("signoff");
                }
                if step.rework_allowed {
                    controls.push("rework");
                }
                ProcessFlowOperadRow {
                    title: format!("{} · {}", step.sequence, step.name),
                    detail: format!(
                        "{} · {} · recipe {} · tools {} · {}",
                        step.area,
                        step.required_tool_class.label(),
                        step.required_recipe,
                        step.eligible_tools
                            .iter()
                            .map(|tool| tool.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        if controls.is_empty() {
                            "standard".to_string()
                        } else {
                            controls.join(", ")
                        }
                    ),
                    tone: if step.required_recipe.as_str().is_empty() {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                    action_name: None,
                    selected: false,
                }
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str, status: &mut String) -> bool {
        if let Some(slug) = node_name.strip_prefix(OPERAD_ACTION_FILTER)
            && let Some(filter) = ProcessFlowNodeFilter::from_slug(slug)
        {
            self.node_filter = filter;
            *status = format!("process flow filter: {}", filter.label());
            return true;
        }
        if let Some(node_id) = node_name.strip_prefix(OPERAD_ACTION_SELECT_NODE) {
            let node_id = node_id
                .split_once('|')
                .map(|(node_id, _)| node_id)
                .unwrap_or(node_id);
            let node_id = ProcessFlowNodeId::new(node_id.to_string());
            if self.model.route.node(&node_id).is_some() {
                self.selected_node = Some(node_id.clone());
                *status = format!("selected process-flow node {node_id}");
                return true;
            }
        }
        if node_name == OPERAD_ACTION_TOGGLE_ERRORS {
            self.show_errors_only = !self.show_errors_only;
            *status = if self.show_errors_only {
                "process flow showing error findings only".to_string()
            } else {
                "process flow showing all findings".to_string()
            };
            return true;
        }
        if node_name == OPERAD_ACTION_EXPORT {
            let mes_route = self.model.export_to_mes_route();
            *status = format!(
                "prepared {} MES steps for {}",
                mes_route.steps.len(),
                mes_route.id
            );
            return true;
        }
        if node_name == OPERAD_ACTION_VALIDATE {
            let error_count = self
                .model
                .findings()
                .iter()
                .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
                .count();
            *status = format!("process flow validation found {} issue(s)", error_count);
            return true;
        }
        false
    }

    fn summary_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let recipe_required = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .count();
        let recipe_ready = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .filter(|node| node.recipe.is_some() && node.has_tool_coverage())
            .count();
        let checkpoint_count = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count();
        let hold_points = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();
        let rework_edges = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .count();
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let warnings = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Warning)
            .count();
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Route graph",
                    format!(
                        "{} nodes / {} edges",
                        self.model.route.nodes.len(),
                        self.model.route.edges.len()
                    ),
                    "topology",
                    Tone::Neutral,
                ),
                (
                    "Recipes",
                    format!("{recipe_ready}/{recipe_required} ready"),
                    "bound and tool-covered",
                    if recipe_ready == recipe_required {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                ),
                (
                    "Metrology",
                    format!("{checkpoint_count} checkpoints"),
                    "sample plans",
                    if checkpoint_count > 0 {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                ),
                (
                    "Holds / rework",
                    format!("{hold_points} holds / {rework_edges} paths"),
                    "release controls",
                    if hold_points > 0 || rework_edges > 0 {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                ),
                (
                    "Validation",
                    if errors == 0 {
                        format!("{warnings} warnings")
                    } else {
                        format!("{errors} errors")
                    },
                    "recipe and eligible-tool coverage",
                    if errors == 0 {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                ),
            ],
        );
    }

    fn compact_route_status_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let errors = findings
            .iter()
            .filter(|finding| finding.severity == ProcessFlowFindingSeverity::Error)
            .count();
        let recipe_required = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.kind.requires_recipe())
            .count();
        let checkpoints = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .count();
        let holds = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point)
            .count();

        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                if errors == 0 { "exportable" } else { "blocked" },
                if errors == 0 {
                    Tone::Success
                } else {
                    Tone::Danger
                },
            );
            ui_chrome::status_pill(ui, &format!("{recipe_required} recipes"), Tone::Info);
            ui_chrome::status_pill(ui, &format!("{checkpoints} checkpoints"), Tone::Success);
            if holds > 0 {
                ui_chrome::status_pill(ui, &format!("{holds} holds"), Tone::Warning);
            }
        });
    }

    fn filter_bar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Filter").strong());
            for filter in ProcessFlowNodeFilter::ALL {
                let count = self.count_nodes_for_filter(filter);
                let selected = self.node_filter == filter;
                if ui
                    .selectable_label(selected, format!("{} ({count})", filter.label()))
                    .clicked()
                {
                    self.node_filter = filter;
                }
            }
        });
        ui.small(RichText::new(self.node_filter.detail()).color(ui.visuals().weak_text_color()));
    }

    fn route_timeline_ui(
        &mut self,
        ui: &mut egui::Ui,
        findings: &[ProcessFlowFinding],
        max_height: f32,
    ) {
        let visible_nodes = self.visible_nodes();
        if visible_nodes.is_empty() {
            ui_chrome::empty_state(ui, "No route steps match this filter");
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("process_flow_route_timeline")
            .max_height(max_height)
            .show(ui, |ui| {
                for (index, node) in visible_nodes {
                    let selected = self.selected_node.as_ref() == Some(&node.id);
                    let issue_count = self.node_findings(&node.id, findings).len();
                    let incoming_count = self.incoming_edges(&node.id).len();
                    let outgoing = self.outgoing_edges(&node.id);
                    let outgoing_count = outgoing.len();
                    let rework_count = self.rework_edge_count_for_node(&node.id);
                    let fill = if selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().faint_bg_color
                    };
                    let stroke_color = if selected {
                        Tone::Info.color()
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    };
                    let mut open_node = false;

                    egui::Frame::new()
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.0, stroke_color))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!("{:02}. {}", index + 1, node.name),
                                    )
                                    .clicked()
                                {
                                    open_node = true;
                                }
                                ui.colored_label(kind_color(node.kind), node.kind.label());
                            });
                            ui.horizontal_wrapped(|ui| {
                                if node.hold_point {
                                    ui_chrome::status_pill(ui, "hold", Tone::Warning);
                                }
                                if node.measurement_checkpoint.is_some() {
                                    ui_chrome::status_pill(ui, "checkpoint", Tone::Success);
                                }
                                if rework_count > 0 {
                                    ui_chrome::status_pill(ui, "rework", Tone::Danger);
                                }
                                if issue_count > 0 {
                                    ui_chrome::status_pill(
                                        ui,
                                        &format!("{issue_count} findings"),
                                        Tone::Danger,
                                    );
                                }
                            });
                            ui.small(format!(
                                "{} | {} in / {} out",
                                node.area, incoming_count, outgoing_count
                            ));
                            if node.kind.requires_recipe() {
                                let recipe = node
                                    .recipe
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_else(|| "unassigned recipe".to_string());
                                ui.small(format!("Recipe: {recipe}"));
                            }
                            if let Some(next_sequence) = outgoing
                                .iter()
                                .find(|edge| edge.kind == ProcessFlowEdgeKind::Sequence)
                            {
                                ui.small(format!("Next: {}", self.node_label(&next_sequence.to)));
                            }
                        });

                    if open_node {
                        self.selected_node = Some(node.id.clone());
                    }
                    ui.add_space(4.0);
                }
            });
    }

    fn selected_node_ui(&self, ui: &mut egui::Ui, findings: &[ProcessFlowFinding]) {
        let Some(node) = self.selected_node() else {
            ui_chrome::empty_state(ui, "No node selected");
            return;
        };

        ui.strong(&node.name);
        ui.small(format!("{} | {}", node.id, node.area));
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, node.kind.label(), tone_for_kind(node.kind));
            if node.kind.requires_recipe() {
                ui_chrome::status_pill(ui, "MES step", Tone::Info);
            }
            if node.hold_point {
                ui_chrome::status_pill(ui, "hold point", Tone::Warning);
            }
            if self
                .outgoing_edges(&node.id)
                .iter()
                .any(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            {
                ui_chrome::status_pill(ui, "rework allowed", Tone::Danger);
            }
        });

        ui.separator();
        ui_chrome::section_label(ui, "Recipe and Equipment");
        if node.kind.requires_recipe() {
            if let Some(recipe) = &node.recipe {
                ui.label(format!("Recipe: {recipe}"));
            } else {
                ui.colored_label(Color32::from_rgb(226, 96, 96), "Recipe: unassigned");
            }
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(
                    ui,
                    if node.recipe.is_some() {
                        "recipe bound"
                    } else {
                        "recipe missing"
                    },
                    if node.recipe.is_some() {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                );
                ui_chrome::status_pill(
                    ui,
                    if node.has_tool_coverage() {
                        "tool coverage"
                    } else {
                        "tools missing"
                    },
                    if node.has_tool_coverage() {
                        Tone::Success
                    } else {
                        Tone::Danger
                    },
                );
            });
        } else {
            ui_chrome::muted(ui, "No recipe required for this route node");
        }
        if !node.allowed_tool_classes.is_empty() {
            ui.label(format!(
                "Allowed classes: {}",
                node.allowed_tool_classes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !node.eligible_tools.is_empty() {
            ui.label(format!(
                "Eligible tools: {}",
                node.eligible_tools.join(", ")
            ));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Material Contract");
        io_list(ui, "Inputs", &node.expected_inputs);
        io_list(ui, "Outputs", &node.expected_outputs);

        if let Some(checkpoint) = &node.measurement_checkpoint {
            ui.separator();
            ui_chrome::section_label(ui, "Measurement Checkpoint");
            ui.label(&checkpoint.measurement_name);
            ui.small(format!("Target: {}", checkpoint.target));
            ui.small(format!("Sample plan: {}", checkpoint.sample_plan));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Dependency State");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} prerequisites", self.incoming_edges(&node.id).len()),
                Tone::Neutral,
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} outcomes", self.outgoing_edges(&node.id).len()),
                Tone::Neutral,
            );
        });

        let step_findings = self.node_findings(&node.id, findings);
        ui.separator();
        ui_chrome::section_label(ui, "Selected-step Findings");
        if step_findings.is_empty() {
            ui_chrome::status_pill(ui, "no selected-step findings", Tone::Success);
        } else {
            for finding in step_findings {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(finding_color(finding.severity), finding.severity.label());
                        ui.strong(&finding.code);
                    });
                    ui.label(&finding.message);
                });
            }
        }

        if !node.notes.is_empty() {
            ui.separator();
            ui.label(RichText::new(&node.notes).color(ui.visuals().weak_text_color()));
        }
    }

    fn dependency_ui(&mut self, ui: &mut egui::Ui) {
        let selected_id = self.selected_node.clone();
        let Some(selected_id) = selected_id else {
            ui_chrome::empty_state(ui, "Select a node to inspect dependencies");
            return;
        };

        ui.small(format!("For {}", self.node_label(&selected_id)));

        let outgoing = self
            .outgoing_edges(&selected_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let incoming = self
            .incoming_edges(&selected_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        ui.label(RichText::new("Prerequisites").strong());
        if incoming.is_empty() {
            ui_chrome::muted(ui, "No upstream dependency");
        }
        for edge in &incoming {
            self.edge_dependency_row_ui(ui, edge, false);
        }

        ui.separator();
        ui.label(RichText::new("Outcomes").strong());
        if outgoing.is_empty() {
            ui_chrome::muted(ui, "No downstream outcome");
        }
        for edge in &outgoing {
            self.edge_dependency_row_ui(ui, edge, true);
        }
    }

    fn edge_dependency_row_ui(
        &mut self,
        ui: &mut egui::Ui,
        edge: &ProcessFlowEdge,
        outgoing: bool,
    ) {
        let target_id = if outgoing { &edge.to } else { &edge.from };
        let target_name = self
            .model
            .route
            .node(target_id)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| "missing node".to_string());
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(if outgoing { "Open next" } else { "Open prior" })
                    .clicked()
                {
                    self.selected_node = Some(target_id.clone());
                }
                ui.colored_label(edge_color(edge.kind), edge.kind.label());
                ui.label(format!("{} {}", target_id, target_name));
            });
            if edge.condition.is_empty() {
                ui.small("Condition: always");
            } else {
                ui.small(format!("Condition: {}", edge.condition));
            }
        });
    }

    fn controls_overview_ui(&mut self, ui: &mut egui::Ui) {
        let checkpoints = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.measurement_checkpoint.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let holds = self
            .model
            .route
            .nodes
            .iter()
            .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
            .cloned()
            .collect::<Vec<_>>();
        let rework_edges = self
            .model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .cloned()
            .collect::<Vec<_>>();

        ui.label(RichText::new("Metrology checkpoints").strong());
        if checkpoints.is_empty() {
            ui_chrome::empty_state(ui, "No measurement checkpoints");
        }
        for node in checkpoints {
            if let Some(checkpoint) = &node.measurement_checkpoint {
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(node.id.to_string()).clicked() {
                            self.selected_node = Some(node.id.clone());
                        }
                        ui.strong(&checkpoint.measurement_name);
                    });
                    ui.small(format!("Step: {}", node.name));
                    ui.small(format!("Target: {}", checkpoint.target));
                    ui.small(format!("Sample plan: {}", checkpoint.sample_plan));
                });
            }
        }

        ui.separator();
        ui.label(RichText::new("Holds and rework").strong());
        if holds.is_empty() && rework_edges.is_empty() {
            ui_chrome::empty_state(ui, "No holds or rework paths");
        }
        for node in holds {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(node.id.to_string()).clicked() {
                        self.selected_node = Some(node.id.clone());
                    }
                    ui_chrome::status_pill(ui, "hold", Tone::Warning);
                    ui.strong(&node.name);
                });
                ui.small(format!("Area: {}", node.area));
                if !node.notes.is_empty() {
                    ui.small(&node.notes);
                }
            });
        }
        for edge in rework_edges {
            let from_label = self.node_label(&edge.from);
            let to_label = self.node_label(&edge.to);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(edge.from.to_string()).clicked() {
                        self.selected_node = Some(edge.from.clone());
                    }
                    ui_chrome::status_pill(ui, "rework", Tone::Danger);
                    ui.label(format!("{from_label} -> {to_label}"));
                });
                if !edge.condition.is_empty() {
                    ui.small(format!("Condition: {}", edge.condition));
                }
            });
        }
    }

    fn finding_list_ui(
        &mut self,
        ui: &mut egui::Ui,
        findings: &[ProcessFlowFinding],
        max_height: f32,
    ) {
        let visible = findings
            .iter()
            .filter(|finding| {
                !self.show_errors_only || finding.severity == ProcessFlowFindingSeverity::Error
            })
            .collect::<Vec<_>>();
        if visible.is_empty() {
            ui_chrome::empty_state(
                ui,
                if self.show_errors_only {
                    "No error findings"
                } else {
                    "No validation findings"
                },
            );
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("process_flow_findings")
            .max_height(max_height)
            .show(ui, |ui| {
                for finding in visible {
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                finding_color(finding.severity),
                                finding.severity.label(),
                            );
                            ui.strong(&finding.code);
                        });
                        ui.label(&finding.message);
                        if let Some(node_id) = &finding.node_id {
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Open node").clicked() {
                                    self.selected_node = Some(node_id.clone());
                                }
                                ui.small(format!("Node: {node_id}"));
                            });
                        }
                        if let Some(edge_id) = &finding.edge_id {
                            ui.small(format!("Edge: {edge_id}"));
                        }
                    });
                }
            });
    }

    fn mes_export_preview_ui(&self, ui: &mut egui::Ui) {
        let mes_route = self.model.export_to_mes_route();
        ui_chrome::section_label(ui, "MES-lite Export Preview");
        egui::ScrollArea::horizontal()
            .id_salt("process_flow_mes_export_scroll")
            .show(ui, |ui| {
                egui::Grid::new("process_flow_mes_export_grid")
                    .striped(true)
                    .min_col_width(80.0)
                    .show(ui, |ui| {
                        ui.strong("Seq");
                        ui.strong("Step");
                        ui.strong("Area");
                        ui.strong("Class");
                        ui.strong("Recipe");
                        ui.strong("Tools");
                        ui.strong("Controls");
                        ui.end_row();
                        for step in &mes_route.steps {
                            let mut controls = Vec::new();
                            if step.signoff_required {
                                controls.push("signoff");
                            }
                            if step.rework_allowed {
                                controls.push("rework");
                            }
                            ui.label(step.sequence.to_string());
                            ui.label(&step.name);
                            ui.label(&step.area);
                            ui.label(step.required_tool_class.label());
                            ui.label(step.required_recipe.as_str());
                            ui.label(
                                step.eligible_tools
                                    .iter()
                                    .map(|tool| tool.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            );
                            ui.label(if controls.is_empty() {
                                "standard".to_string()
                            } else {
                                controls.join(", ")
                            });
                            ui.end_row();
                        }
                    });
            });
    }

    fn visible_nodes(&self) -> Vec<(usize, ProcessFlowNode)> {
        self.model
            .route
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| self.node_matches_filter(self.node_filter, node))
            .map(|(index, node)| (index, node.clone()))
            .collect()
    }

    fn count_nodes_for_filter(&self, filter: ProcessFlowNodeFilter) -> usize {
        self.model
            .route
            .nodes
            .iter()
            .filter(|node| self.node_matches_filter(filter, node))
            .count()
    }

    fn node_matches_filter(&self, filter: ProcessFlowNodeFilter, node: &ProcessFlowNode) -> bool {
        match filter {
            ProcessFlowNodeFilter::All => true,
            ProcessFlowNodeFilter::RecipeSteps => {
                node.kind.requires_recipe() || node.recipe.is_some()
            }
            ProcessFlowNodeFilter::Metrology => {
                node.kind == ProcessFlowNodeKind::Measurement
                    || node.measurement_checkpoint.is_some()
            }
            ProcessFlowNodeFilter::Holds => {
                node.hold_point || node.kind == ProcessFlowNodeKind::Hold
            }
            ProcessFlowNodeFilter::Rework => self.model.route.edges.iter().any(|edge| {
                edge.kind == ProcessFlowEdgeKind::Rework
                    && (edge.from == node.id || edge.to == node.id)
            }),
        }
    }

    fn incoming_edges(&self, node_id: &ProcessFlowNodeId) -> Vec<&ProcessFlowEdge> {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| &edge.to == node_id)
            .collect()
    }

    fn outgoing_edges(&self, node_id: &ProcessFlowNodeId) -> Vec<&ProcessFlowEdge> {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| &edge.from == node_id)
            .collect()
    }

    fn rework_edge_count_for_node(&self, node_id: &ProcessFlowNodeId) -> usize {
        self.model
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == ProcessFlowEdgeKind::Rework)
            .filter(|edge| &edge.from == node_id || &edge.to == node_id)
            .count()
    }

    fn node_findings<'a>(
        &self,
        node_id: &ProcessFlowNodeId,
        findings: &'a [ProcessFlowFinding],
    ) -> Vec<&'a ProcessFlowFinding> {
        findings
            .iter()
            .filter(|finding| finding.node_id.as_ref() == Some(node_id))
            .collect()
    }

    fn node_label(&self, node_id: &ProcessFlowNodeId) -> String {
        self.model
            .route
            .node(node_id)
            .map(|node| format!("{} {}", node.id, node.name))
            .unwrap_or_else(|| format!("{node_id} missing node"))
    }

    fn selected_node(&self) -> Option<&ProcessFlowNode> {
        self.selected_node
            .as_ref()
            .and_then(|id| self.model.route.node(id))
            .or_else(|| self.model.route.nodes.first())
    }

    fn ensure_selection(&mut self) {
        if self
            .selected_node
            .as_ref()
            .and_then(|id| self.model.route.node(id))
            .is_none()
        {
            self.selected_node = self.model.route.nodes.first().map(|node| node.id.clone());
        }
    }
}

fn process_flow_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += process_flow_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += process_flow_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn process_flow_operad_metric_columns(width: f32) -> usize {
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

fn process_flow_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = process_flow_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn process_flow_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_process_flow_operad_header(
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
            "process_flow.header",
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
    add_process_flow_operad_text(
        document,
        header,
        "process_flow.header.eyebrow",
        eyebrow,
        process_flow_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_process_flow_operad_text(
        document,
        header,
        "process_flow.header.title",
        title,
        process_flow_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_process_flow_operad_text(
        document,
        header,
        "process_flow.header.detail",
        detail,
        process_flow_operad_text_style(
            14.0,
            FontWeight::NORMAL,
            ColorRgba::new(178, 185, 194, 255),
        ),
        20.0,
    );
    add_process_flow_operad_text(
        document,
        header,
        "process_flow.header.meta",
        meta,
        process_flow_operad_text_style(
            13.0,
            FontWeight::NORMAL,
            ColorRgba::new(112, 183, 239, 255),
        ),
        18.0,
    );
}

fn add_process_flow_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[ProcessFlowMetricTile],
) {
    let columns = process_flow_operad_metric_columns(width);
    let grid_height = process_flow_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "process_flow.metrics",
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
                format!("process_flow.metrics.row.{row_index}"),
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
            add_process_flow_operad_metric_tile(
                document,
                row,
                &format!("process_flow.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_process_flow_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &ProcessFlowMetricTile,
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
                process_flow_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_process_flow_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        process_flow_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_process_flow_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        process_flow_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_process_flow_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        process_flow_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            process_flow_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_process_flow_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[ProcessFlowOperadRow],
) {
    let height = process_flow_operad_section_height(rows.len());
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
    add_process_flow_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        process_flow_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_process_flow_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_process_flow_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_process_flow_operad_empty_row(
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
    add_process_flow_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        process_flow_operad_text_style(
            13.0,
            FontWeight::NORMAL,
            ColorRgba::new(154, 163, 172, 255),
        ),
        24.0,
    );
}

fn add_process_flow_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &ProcessFlowOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        process_flow_operad_tone_color(Tone::Info)
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
            process_flow_operad_tone_color(row.tone),
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
    add_process_flow_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        process_flow_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_process_flow_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        process_flow_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            ColorRgba::new(162, 171, 180, 255),
        ),
        19.0,
    );
}

fn add_process_flow_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("process_flow.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_process_flow_operad_text(
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

fn process_flow_operad_text_style(
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

fn process_flow_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn edge_tone(kind: ProcessFlowEdgeKind) -> Tone {
    match kind {
        ProcessFlowEdgeKind::Sequence => Tone::Info,
        ProcessFlowEdgeKind::Branch => Tone::Warning,
        ProcessFlowEdgeKind::Rework => Tone::Danger,
    }
}

fn finding_tone(severity: ProcessFlowFindingSeverity) -> Tone {
    match severity {
        ProcessFlowFindingSeverity::Info => Tone::Info,
        ProcessFlowFindingSeverity::Warning => Tone::Warning,
        ProcessFlowFindingSeverity::Error => Tone::Danger,
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

fn io_list(ui: &mut egui::Ui, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    ui.separator();
    ui.label(RichText::new(label).strong());
    for value in values {
        ui.label(format!("- {value}"));
    }
}

fn tone_for_kind(kind: ProcessFlowNodeKind) -> Tone {
    match kind {
        ProcessFlowNodeKind::Start | ProcessFlowNodeKind::End => Tone::Neutral,
        ProcessFlowNodeKind::Operation => Tone::Info,
        ProcessFlowNodeKind::Measurement => Tone::Success,
        ProcessFlowNodeKind::Hold => Tone::Warning,
    }
}

fn kind_color(kind: ProcessFlowNodeKind) -> Color32 {
    tone_for_kind(kind).color()
}

fn edge_color(kind: ProcessFlowEdgeKind) -> Color32 {
    match kind {
        ProcessFlowEdgeKind::Sequence => Tone::Info.color(),
        ProcessFlowEdgeKind::Branch => Tone::Warning.color(),
        ProcessFlowEdgeKind::Rework => Tone::Danger.color(),
    }
}

fn finding_color(severity: ProcessFlowFindingSeverity) -> Color32 {
    match severity {
        ProcessFlowFindingSeverity::Info => Tone::Info.color(),
        ProcessFlowFindingSeverity::Warning => Tone::Warning.color(),
        ProcessFlowFindingSeverity::Error => Tone::Danger.color(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_flow_operad_view_audits_common_widths() {
        let panel = ProcessFlowPanel::from_model(ProcessFlowModel::sample());
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
    fn process_flow_operad_actions_update_panel_state() {
        let mut panel = ProcessFlowPanel::from_model(ProcessFlowModel::sample());
        let mut status = String::new();

        assert!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_FILTER}metrology"), &mut status)
        );
        assert_eq!(panel.node_filter, ProcessFlowNodeFilter::Metrology);
        assert!(status.contains("Metrology"));

        let target_node = panel.model.route.nodes.last().unwrap().id.clone();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_NODE}{target_node}|test"),
            &mut status
        ));
        assert_eq!(panel.selected_node.as_ref(), Some(&target_node));
        assert!(status.contains(&target_node.to_string()));

        assert!(panel.handle_operad_action(OPERAD_ACTION_TOGGLE_ERRORS, &mut status));
        assert!(panel.show_errors_only);

        assert!(panel.handle_operad_action(OPERAD_ACTION_EXPORT, &mut status));
        assert!(status.contains("prepared"));

        assert!(panel.handle_operad_action(OPERAD_ACTION_VALIDATE, &mut status));
        assert!(status.contains("process flow validation"));
    }
}
