use std::collections::BTreeSet;

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::{
    Document,
    environment::CleanroomEnvironment,
    equipment::EquipmentSimulator,
    inventory::{FabObjectLink, Inventory},
    maintenance::{DueState, FabDate, MaintenanceModel},
    mes::{FabMesData, Lot, LotId, ProcessRoute, TravelerState, TravelerStatus},
    metrology::WaferMap,
    notebook::LabNotebook,
    process_flow::{ProcessFlowModel, ProcessFlowNode, ProcessFlowNodeKind},
    recipe::{RecipeCatalog, RecipeId},
    safety::SafetySystem,
    scheduler::{
        DispatchAssignment, DispatchLot, DispatchPolicy, DispatchSchedule, format_shift_time,
    },
    yield_analysis::YieldAnalysis,
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

const DEFAULT_FOCUS_LOT: &str = "L-00042";
const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_OPEN: &str = "workflow.action.open.";
const OPERAD_ACTION_FOCUS_LOT: &str = "workflow.action.focus_lot.";
const OPERAD_ACTION_LOAD_DEMO: &str = "workflow.action.load_demo";

pub(crate) struct WorkflowPanel {
    focus_lot: String,
}

pub(crate) struct WorkflowData<'a> {
    pub(crate) document: &'a Document,
    pub(crate) process_flow: &'a ProcessFlowModel,
    pub(crate) recipes: &'a RecipeCatalog,
    pub(crate) mes: &'a FabMesData,
    pub(crate) inventory: &'a Inventory,
    pub(crate) maintenance: &'a MaintenanceModel,
    pub(crate) environment: &'a CleanroomEnvironment,
    pub(crate) scheduler: &'a DispatchSchedule,
    pub(crate) safety: &'a SafetySystem,
    pub(crate) equipment: &'a EquipmentSimulator,
    pub(crate) wafer_map: &'a WaferMap,
    pub(crate) yield_analysis: &'a YieldAnalysis,
    pub(crate) notebook: &'a LabNotebook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkflowDestination {
    Layout,
    ProcessFlow,
    Inventory,
    Scheduler,
    FabControl,
    Maintenance,
    Environment,
    Safety,
    Metrology,
    Yield,
    Notebook,
    Traceability,
}

impl WorkflowDestination {
    fn slug(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::ProcessFlow => "process-flow",
            Self::Inventory => "inventory",
            Self::Scheduler => "scheduler",
            Self::FabControl => "fab-control",
            Self::Maintenance => "maintenance",
            Self::Environment => "environment",
            Self::Safety => "safety",
            Self::Metrology => "metrology",
            Self::Yield => "yield",
            Self::Notebook => "notebook",
            Self::Traceability => "traceability",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "layout" => Some(Self::Layout),
            "process-flow" => Some(Self::ProcessFlow),
            "inventory" => Some(Self::Inventory),
            "scheduler" => Some(Self::Scheduler),
            "fab-control" => Some(Self::FabControl),
            "maintenance" => Some(Self::Maintenance),
            "environment" => Some(Self::Environment),
            "safety" => Some(Self::Safety),
            "metrology" => Some(Self::Metrology),
            "yield" => Some(Self::Yield),
            "notebook" => Some(Self::Notebook),
            "traceability" => Some(Self::Traceability),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkflowAction {
    Open(WorkflowDestination),
    LoadDemoWorkspace,
}

struct WorkflowStepSummary {
    area: &'static str,
    title: &'static str,
    detail: String,
    tone: Tone,
    target: WorkflowDestination,
    action: &'static str,
}

struct CrossLinkSummary {
    title: &'static str,
    status: &'static str,
    detail: String,
    tone: Tone,
    target: WorkflowDestination,
    action: &'static str,
}

#[derive(Debug)]
struct WorkflowOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct WorkflowMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct WorkflowOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

impl Default for WorkflowPanel {
    fn default() -> Self {
        Self {
            focus_lot: DEFAULT_FOCUS_LOT.to_string(),
        }
    }
}

impl WorkflowPanel {
    pub(crate) fn focus_lot(&self) -> &str {
        &self.focus_lot
    }

    pub(crate) fn set_focus_lot(&mut self, lot_id: impl Into<String>) {
        self.focus_lot = lot_id.into();
    }

    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        data: WorkflowData<'_>,
    ) -> Option<WorkflowAction> {
        self.ensure_focus_lot(&data);
        match self.operad_ui(ui, &data) {
            Ok(action) => action,
            Err(error) => {
                ui.colored_label(Color32::from_rgb(226, 96, 96), error);
                self.egui_dashboard_ui(ui, &data)
            }
        }
    }

    fn operad_ui(
        &mut self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
    ) -> Result<Option<WorkflowAction>, String> {
        let mut result = Ok(None);
        egui::ScrollArea::vertical()
            .id_salt("fab_workflow_dashboard_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width, data);
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
                {
                    if let Some(action) = self.handle_operad_action(&node_name) {
                        result = Ok(Some(action));
                    } else if self.handle_operad_focus_action(&node_name, data) {
                        view = self.build_operad_view(width, data);
                        if let Err(error) = view
                            .document
                            .compute_layout(view.size, &mut ApproxTextMeasurer)
                            .map_err(|error| error.to_string())
                        {
                            result = Err(error);
                            return;
                        }
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

    fn egui_dashboard_ui(
        &mut self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
    ) -> Option<WorkflowAction> {
        let has_workspace_data = workflow_has_data(data);
        let mut destination = None;
        let mut load_demo = false;

        egui::ScrollArea::vertical()
            .id_salt("fab_workflow_dashboard")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    if has_workspace_data {
                        "Workspace workflow"
                    } else {
                        "Blank workspace"
                    },
                    "Fab Workflow",
                    if self.focus_lot.is_empty() {
                        "No lot selected"
                    } else {
                        ""
                    },
                    |ui| {
                        if has_workspace_data {
                            self.lot_picker(ui, data);
                        } else if ui.button("Load demo workspace").clicked() {
                            load_demo = true;
                        }
                    },
                );

                if !has_workspace_data {
                    ui_chrome::empty_state(ui, "No workspace data loaded");
                    return;
                }

                self.metric_row(ui, data);
                ui.separator();
                self.production_focus_ui(ui, data, &mut destination);
                ui.separator();

                if ui.available_width() >= 940.0 {
                    ui.columns(2, |columns| {
                        self.spine_ui(&mut columns[0], data, &mut destination);
                        self.route_operations_ui(&mut columns[1], data, &mut destination);
                    });
                    ui.separator();
                    self.cross_link_ui(ui, data, &mut destination);
                } else {
                    self.spine_ui(ui, data, &mut destination);
                    ui.separator();
                    self.route_operations_ui(ui, data, &mut destination);
                    ui.separator();
                    self.cross_link_ui(ui, data, &mut destination);
                }
            });

        if load_demo {
            Some(WorkflowAction::LoadDemoWorkspace)
        } else {
            destination.map(WorkflowAction::Open)
        }
    }

    fn build_operad_view(&self, width: f32, data: &WorkflowData<'_>) -> WorkflowOperadView {
        let has_workspace_data = workflow_has_data(data);
        let metrics = if has_workspace_data {
            self.operad_metrics(data)
        } else {
            Vec::new()
        };
        let action_rows = self.operad_action_rows(data, has_workspace_data);
        let lot_rows = if has_workspace_data {
            self.operad_lot_rows(data)
        } else {
            Vec::new()
        };
        let focus_rows = if has_workspace_data {
            self.operad_focus_rows(data)
        } else {
            vec![WorkflowOperadRow {
                title: "No workspace data loaded".to_string(),
                detail: "Load the demo workspace to populate layout, MES, dispatch, metrology, yield, and notes.".to_string(),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            }]
        };
        let workflow_rows = if has_workspace_data {
            self.operad_workflow_rows(data)
        } else {
            Vec::new()
        };
        let route_rows = if has_workspace_data {
            self.operad_route_rows(data)
        } else {
            Vec::new()
        };
        let cross_link_rows = if has_workspace_data {
            self.operad_cross_link_rows(data)
        } else {
            Vec::new()
        };
        let height = workflow_operad_view_height(
            width,
            metrics.len(),
            &[
                action_rows.len(),
                lot_rows.len(),
                focus_rows.len(),
                workflow_rows.len(),
                route_rows.len(),
                cross_link_rows.len(),
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

        add_workflow_operad_header(
            &mut document,
            root,
            if has_workspace_data {
                "WORKSPACE WORKFLOW"
            } else {
                "BLANK WORKSPACE"
            },
            "Fab Workflow",
            if self.focus_lot.is_empty() {
                "No lot selected"
            } else {
                "Cross-linked production context across design, operations, analysis, and engineering notes"
            },
            if self.focus_lot.is_empty() {
                "Focus lot: none".to_string()
            } else {
                format!("Focus lot: {}", self.focus_lot)
            },
        );
        add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
        if !metrics.is_empty() {
            add_workflow_operad_metric_grid(&mut document, root, width, &metrics);
            add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
        }
        add_workflow_operad_section(
            &mut document,
            root,
            width,
            "workflow.actions",
            "Actions",
            "No workflow actions available",
            &action_rows,
        );
        add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
        if has_workspace_data {
            add_workflow_operad_section(
                &mut document,
                root,
                width,
                "workflow.lots",
                "Focus Lot",
                "No lot loaded",
                &lot_rows,
            );
            add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
        }
        add_workflow_operad_section(
            &mut document,
            root,
            width,
            "workflow.focus",
            "Production Focus",
            "No production lot selected",
            &focus_rows,
        );
        add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
        if has_workspace_data {
            add_workflow_operad_section(
                &mut document,
                root,
                width,
                "workflow.spine",
                "Operational Workflow",
                "No workflow steps loaded",
                &workflow_rows,
            );
            add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
            add_workflow_operad_section(
                &mut document,
                root,
                width,
                "workflow.route",
                "Route Operations",
                "No operation nodes in route",
                &route_rows,
            );
            add_workflow_operad_spacer(&mut document, root, OPERAD_GAP);
            add_workflow_operad_section(
                &mut document,
                root,
                width,
                "workflow.cross_links",
                "Cross-Links",
                "No cross-links loaded",
                &cross_link_rows,
            );
        }

        WorkflowOperadView { document, size }
    }

    fn operad_metrics(&self, data: &WorkflowData<'_>) -> Vec<WorkflowMetricTile> {
        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let traveler = focus_traveler(data.mes, &self.focus_lot);
        let (dispatch_value, dispatch_detail, dispatch_tone) =
            focus_dispatch_metric(data.scheduler, &self.focus_lot);
        let (guardrail_value, guardrail_detail, guardrail_tone) = guardrail_metric(data);
        let yield_label = focus_yield_label(data.yield_analysis, &self.focus_lot);
        let yield_tone = data
            .yield_analysis
            .lot_summary(&self.focus_lot)
            .map_or(Tone::Neutral, |summary| yield_tone(summary.yield_fraction));
        let route_detail = format!(
            "{} ops / {} holds",
            operation_node_count(data.process_flow),
            hold_point_count(data.process_flow)
        );
        let recipe_detail = if recipe_links.is_empty() {
            "no route bindings".to_string()
        } else if missing_recipes.is_empty() {
            "all bindings resolve".to_string()
        } else {
            format!("{} missing", missing_recipes.len())
        };
        let traveler_detail = traveler
            .map(|traveler| {
                format!(
                    "{} completed",
                    pluralize_count(traveler.completed_steps.len(), "step")
                )
            })
            .unwrap_or_else(|| "no MES traveler".to_string());
        let traveler_value = traveler
            .map(|traveler| traveler.status.label().to_string())
            .unwrap_or_else(|| "missing".to_string());
        vec![
            WorkflowMetricTile {
                label: "Layout scope".to_string(),
                value: data.document.shapes.len().to_string(),
                detail: "workspace shapes".to_string(),
                tone: Tone::Neutral,
            },
            WorkflowMetricTile {
                label: "Route ops".to_string(),
                value: data.process_flow.route.nodes.len().to_string(),
                detail: route_detail,
                tone: if data.process_flow.route.nodes.is_empty() {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
            },
            WorkflowMetricTile {
                label: "Recipes".to_string(),
                value: recipe_links
                    .len()
                    .saturating_sub(missing_recipes.len())
                    .to_string(),
                detail: recipe_detail,
                tone: if recipe_links.is_empty() {
                    Tone::Neutral
                } else if missing_recipes.is_empty() {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            },
            WorkflowMetricTile {
                label: "Traveler".to_string(),
                value: traveler_value,
                detail: traveler_detail,
                tone: traveler.map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
            },
            WorkflowMetricTile {
                label: "Dispatch".to_string(),
                value: dispatch_value,
                detail: dispatch_detail,
                tone: dispatch_tone,
            },
            WorkflowMetricTile {
                label: "Materials".to_string(),
                value: material_count.to_string(),
                detail: "linked to focus lot".to_string(),
                tone: Tone::Neutral,
            },
            WorkflowMetricTile {
                label: "Guardrails".to_string(),
                value: guardrail_value,
                detail: guardrail_detail,
                tone: guardrail_tone,
            },
            WorkflowMetricTile {
                label: "Yield".to_string(),
                value: yield_label,
                detail: "focus lot".to_string(),
                tone: yield_tone,
            },
        ]
    }

    fn operad_action_rows(
        &self,
        data: &WorkflowData<'_>,
        has_workspace_data: bool,
    ) -> Vec<WorkflowOperadRow> {
        if !has_workspace_data {
            return vec![WorkflowOperadRow {
                title: "Load demo workspace".to_string(),
                detail: "Populate the workflow with linked layout, MES, recipes, inventory, dispatch, metrology, yield, and notebook data.".to_string(),
                tone: Tone::Info,
                action_name: Some(OPERAD_ACTION_LOAD_DEMO.to_string()),
                selected: false,
            }];
        }
        vec![
            WorkflowOperadRow {
                title: "Traveler".to_string(),
                detail: traveler_detail(data.mes, &self.focus_lot),
                tone: focus_traveler(data.mes, &self.focus_lot)
                    .map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::FabControl,
                    "action.0",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Dispatch".to_string(),
                detail: dispatch_detail(data.scheduler, &self.focus_lot),
                tone: focus_dispatch_metric(data.scheduler, &self.focus_lot).2,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Scheduler,
                    "action.1",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Genealogy".to_string(),
                detail: format!(
                    "{} linked material lots",
                    linked_material_count(data.inventory, &self.focus_lot)
                ),
                tone: Tone::Info,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Traceability,
                    "action.2",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Yield".to_string(),
                detail: focus_yield_label(data.yield_analysis, &self.focus_lot),
                tone: data
                    .yield_analysis
                    .lot_summary(&self.focus_lot)
                    .map_or(Tone::Neutral, |summary| yield_tone(summary.yield_fraction)),
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Yield,
                    "action.3",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Notebook".to_string(),
                detail: format!(
                    "{} linked entries",
                    linked_notebook_count(data.notebook, &self.focus_lot)
                ),
                tone: Tone::Info,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Notebook,
                    "action.4",
                )),
                selected: false,
            },
        ]
    }

    fn operad_lot_rows(&self, data: &WorkflowData<'_>) -> Vec<WorkflowOperadRow> {
        workflow_lot_ids(data)
            .into_iter()
            .enumerate()
            .map(|(index, lot_id)| WorkflowOperadRow {
                title: lot_id.clone(),
                detail: format!(
                    "{} · {} · {}",
                    focus_product_label(data, &lot_id),
                    traveler_detail(data.mes, &lot_id),
                    focus_yield_label(data.yield_analysis, &lot_id)
                ),
                tone: if lot_id == self.focus_lot {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                action_name: Some(format!("{OPERAD_ACTION_FOCUS_LOT}{lot_id}|lot.{index}")),
                selected: lot_id == self.focus_lot,
            })
            .collect()
    }

    fn operad_focus_rows(&self, data: &WorkflowData<'_>) -> Vec<WorkflowOperadRow> {
        if self.focus_lot.is_empty() {
            return Vec::new();
        }
        let lot_id = self.focus_lot.as_str();
        let lot_key = LotId::new(lot_id);
        let mes_lot = data.mes.lots.get(&lot_key);
        let traveler = data.mes.travelers.get(&lot_key);
        let route = traveler.and_then(|traveler| data.mes.routes.get(&traveler.route_id));
        let scheduler_lot = data
            .scheduler
            .lots
            .iter()
            .find(|lot| lot.id.as_str() == lot_id);
        let dispatch_result = data.scheduler.dispatch(DispatchPolicy::PriorityThenFifo);
        let assignment = dispatch_result
            .assignments
            .iter()
            .find(|assignment| assignment.lot_id.as_str() == lot_id);
        vec![
            WorkflowOperadRow {
                title: format!("Lot {lot_id}"),
                detail: focus_product_label(data, lot_id),
                tone: traveler.map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
                action_name: None,
                selected: true,
            },
            WorkflowOperadRow {
                title: "Route".to_string(),
                detail: focus_route_label(route, data.process_flow),
                tone: if data.process_flow.route.nodes.is_empty() {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::ProcessFlow,
                    "focus.route",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Current step".to_string(),
                detail: focus_step_label(data.mes, lot_id),
                tone: traveler.map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::FabControl,
                    "focus.step",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Dispatch".to_string(),
                detail: focus_assignment_label(assignment, scheduler_lot),
                tone: if assignment.is_some() {
                    Tone::Success
                } else {
                    Tone::Neutral
                },
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Scheduler,
                    "focus.dispatch",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Wafers".to_string(),
                detail: focus_wafer_label(mes_lot, scheduler_lot),
                tone: Tone::Neutral,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Traceability,
                    "focus.wafers",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Materials".to_string(),
                detail: format!(
                    "{} linked material lots",
                    linked_material_count(data.inventory, lot_id)
                ),
                tone: Tone::Info,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Inventory,
                    "focus.materials",
                )),
                selected: false,
            },
            WorkflowOperadRow {
                title: "Yield".to_string(),
                detail: focus_yield_label(data.yield_analysis, lot_id),
                tone: data
                    .yield_analysis
                    .lot_summary(lot_id)
                    .map_or(Tone::Neutral, |summary| yield_tone(summary.yield_fraction)),
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::Yield,
                    "focus.yield",
                )),
                selected: false,
            },
        ]
    }

    fn operad_workflow_rows(&self, data: &WorkflowData<'_>) -> Vec<WorkflowOperadRow> {
        self.workflow_steps(data)
            .into_iter()
            .enumerate()
            .map(|(index, step)| WorkflowOperadRow {
                title: format!("{} · {}", step.area, step.title),
                detail: step.detail,
                tone: step.tone,
                action_name: Some(workflow_open_action_name(
                    step.target,
                    &format!("spine.{index}"),
                )),
                selected: false,
            })
            .collect()
    }

    fn operad_route_rows(&self, data: &WorkflowData<'_>) -> Vec<WorkflowOperadRow> {
        data.process_flow
            .route
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    ProcessFlowNodeKind::Operation
                        | ProcessFlowNodeKind::Measurement
                        | ProcessFlowNodeKind::Hold
                )
            })
            .enumerate()
            .map(|(index, node)| WorkflowOperadRow {
                title: format!("{} · {}", node.name, node.kind.label()),
                detail: format!(
                    "{} · {} · {}",
                    node.area,
                    route_node_recipe_label(node),
                    route_node_control_label(node)
                ),
                tone: recipe_link_tone(data.recipes, node),
                action_name: Some(workflow_open_action_name(
                    if node.eligible_tools.is_empty() {
                        WorkflowDestination::ProcessFlow
                    } else {
                        WorkflowDestination::Scheduler
                    },
                    &format!("route.{index}"),
                )),
                selected: false,
            })
            .collect()
    }

    fn operad_cross_link_rows(&self, data: &WorkflowData<'_>) -> Vec<WorkflowOperadRow> {
        let mut rows = cross_link_summaries(data, &self.focus_lot)
            .into_iter()
            .enumerate()
            .map(|(index, link)| WorkflowOperadRow {
                title: format!("{} · {}", link.title, link.status),
                detail: link.detail,
                tone: link.tone,
                action_name: Some(workflow_open_action_name(
                    link.target,
                    &format!("cross.{index}"),
                )),
                selected: false,
            })
            .collect::<Vec<_>>();
        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        if recipe_links.is_empty() {
            rows.push(WorkflowOperadRow {
                title: "Recipe links".to_string(),
                detail: "No process-flow recipe links loaded.".to_string(),
                tone: Tone::Neutral,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::ProcessFlow,
                    "cross.recipe",
                )),
                selected: false,
            });
        } else if missing_recipes.is_empty() {
            rows.push(WorkflowOperadRow {
                title: "Recipe links".to_string(),
                detail: "All process-flow recipe links resolve.".to_string(),
                tone: Tone::Success,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::ProcessFlow,
                    "cross.recipe",
                )),
                selected: false,
            });
        } else {
            rows.push(WorkflowOperadRow {
                title: "Missing recipe definitions".to_string(),
                detail: missing_recipes.join(", "),
                tone: Tone::Warning,
                action_name: Some(workflow_open_action_name(
                    WorkflowDestination::ProcessFlow,
                    "cross.recipe",
                )),
                selected: false,
            });
        }
        rows
    }

    fn handle_operad_action(&self, node_name: &str) -> Option<WorkflowAction> {
        if node_name == OPERAD_ACTION_LOAD_DEMO {
            return Some(WorkflowAction::LoadDemoWorkspace);
        }
        let slug = node_name.strip_prefix(OPERAD_ACTION_OPEN)?;
        let slug = slug.split_once('|').map(|(slug, _)| slug).unwrap_or(slug);
        WorkflowDestination::from_slug(slug).map(WorkflowAction::Open)
    }

    fn handle_operad_focus_action(&mut self, node_name: &str, data: &WorkflowData<'_>) -> bool {
        let Some(lot_id) = node_name.strip_prefix(OPERAD_ACTION_FOCUS_LOT) else {
            return false;
        };
        let lot_id = lot_id
            .split_once('|')
            .map(|(lot_id, _)| lot_id)
            .unwrap_or(lot_id);
        if workflow_lot_ids(data).iter().any(|id| id == lot_id) {
            self.focus_lot = lot_id.to_string();
            return true;
        }
        false
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, data: WorkflowData<'_>) {
        self.ensure_focus_lot(&data);
        if self.operad_context_ui(ui, &data).is_err() {
            self.egui_context_ui(ui, &data);
        }
    }

    fn operad_context_ui(
        &mut self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
    ) -> Result<(), String> {
        let lot_ids = workflow_lot_ids(data);
        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let notebook_count = linked_notebook_count(data.notebook, &self.focus_lot);

        let mut sections = Vec::new();
        let mut workflow = SidecarSection::new("Workflow");
        if lot_ids.is_empty() {
            workflow = workflow.empty("No lot loaded");
        } else {
            workflow = workflow.row(SidecarRow::new(
                "Focus lot",
                self.focus_lot.clone(),
                Tone::Info,
            ));
        }
        sections.push(workflow);

        if !self.focus_lot.is_empty() {
            sections.push(
                SidecarSection::new("Focus Lot")
                    .row(SidecarRow::new(
                        "Product",
                        focus_product_label(data, &self.focus_lot),
                        Tone::Neutral,
                    ))
                    .row(SidecarRow::new(
                        "Traveler",
                        traveler_detail(data.mes, &self.focus_lot),
                        Tone::Neutral,
                    ))
                    .row(SidecarRow::new(
                        "Dispatch",
                        dispatch_detail(data.scheduler, &self.focus_lot),
                        Tone::Neutral,
                    ))
                    .row(SidecarRow::new(
                        "Yield",
                        focus_yield_label(data.yield_analysis, &self.focus_lot),
                        Tone::Neutral,
                    )),
            );
        }

        let mut lot_focus = SidecarSection::new("Lot Focus").empty("No lot focus targets");
        for lot_id in lot_ids.iter().take(6) {
            lot_focus = lot_focus.row(
                SidecarRow::new(
                    lot_id.clone(),
                    if lot_id == &self.focus_lot {
                        "current focus".to_string()
                    } else {
                        "switch focus lot".to_string()
                    },
                    if lot_id == &self.focus_lot {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                )
                .selected(lot_id == &self.focus_lot)
                .action(format!("{OPERAD_ACTION_FOCUS_LOT}{lot_id}|context")),
            );
        }
        sections.push(lot_focus);

        sections.push(
            SidecarSection::new("Production Links")
                .row(SidecarRow::new(
                    "Route ops",
                    format!(
                        "{} operations / {} measurements",
                        operation_node_count(data.process_flow),
                        measurement_node_count(data.process_flow)
                    ),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Route recipes",
                    format!(
                        "{} linked / {} missing",
                        recipe_links.len(),
                        missing_recipes.len()
                    ),
                    if missing_recipes.is_empty() {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                ))
                .row(SidecarRow::new(
                    "Cross-links",
                    format!("{material_count} materials / {notebook_count} notebook entries"),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Guardrails",
                    concise_guardrail_label(data),
                    Tone::Info,
                )),
        );

        if let Some(action) = render_sidecar_interactive(ui, "workflow.context", &sections)? {
            self.handle_operad_focus_action(&action, data);
        }
        Ok(())
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui, data: &WorkflowData<'_>) {
        ui_chrome::section_label(ui, "Workflow");
        if workflow_lot_ids(data).is_empty() {
            ui_chrome::muted(ui, "No lot loaded");
        } else {
            self.lot_picker(ui, data);
        }
        ui.separator();

        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let notebook_count = linked_notebook_count(data.notebook, &self.focus_lot);

        if self.focus_lot.is_empty() {
            ui_chrome::muted(ui, "No lot selected");
        } else {
            ui.label(RichText::new(format!("Focus lot {}", self.focus_lot)).strong());
            ui.label(format!(
                "Product: {}",
                focus_product_label(data, &self.focus_lot)
            ));
            ui.label(format!(
                "Traveler: {}",
                traveler_detail(data.mes, &self.focus_lot)
            ));
            ui.label(format!(
                "Dispatch: {}",
                dispatch_detail(data.scheduler, &self.focus_lot)
            ));
            ui.label(format!(
                "Yield: {}",
                focus_yield_label(data.yield_analysis, &self.focus_lot)
            ));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Production Links");
        ui.label(format!(
            "Route ops: {} operations / {} measurements",
            operation_node_count(data.process_flow),
            measurement_node_count(data.process_flow)
        ));
        ui.label(format!("Route recipes: {}", recipe_links.len()));
        ui.label(format!("Missing recipes: {}", missing_recipes.len()));
        ui.label(format!("Linked materials: {material_count}"));
        ui.label(format!("Notebook entries: {notebook_count}"));
        ui.label(format!("Guardrails: {}", concise_guardrail_label(data)));
    }

    fn metric_row(&self, ui: &mut egui::Ui, data: &WorkflowData<'_>) {
        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let traveler = focus_traveler(data.mes, &self.focus_lot);
        let (dispatch_value, dispatch_detail, dispatch_tone) =
            focus_dispatch_metric(data.scheduler, &self.focus_lot);
        let (guardrail_value, guardrail_detail, guardrail_tone) = guardrail_metric(data);
        let yield_label = focus_yield_label(data.yield_analysis, &self.focus_lot);
        let yield_tone = data
            .yield_analysis
            .lot_summary(&self.focus_lot)
            .map_or(Tone::Neutral, |summary| yield_tone(summary.yield_fraction));
        let route_detail = format!(
            "{} ops / {} holds",
            operation_node_count(data.process_flow),
            hold_point_count(data.process_flow)
        );
        let recipe_detail = if recipe_links.is_empty() {
            "no route bindings".to_string()
        } else if missing_recipes.is_empty() {
            "all bindings resolve".to_string()
        } else {
            format!("{} missing", missing_recipes.len())
        };
        let traveler_detail = traveler
            .map(|traveler| {
                format!(
                    "{} completed",
                    pluralize_count(traveler.completed_steps.len(), "step")
                )
            })
            .unwrap_or_else(|| "no MES traveler".to_string());
        let traveler_value = traveler
            .map(|traveler| traveler.status.label().to_string())
            .unwrap_or_else(|| "missing".to_string());

        let metrics = [
            (
                "Layout scope",
                data.document.shapes.len().to_string(),
                "workspace shapes",
                Tone::Neutral,
            ),
            (
                "Route ops",
                data.process_flow.route.nodes.len().to_string(),
                route_detail.as_str(),
                if data.process_flow.route.nodes.is_empty() {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
            ),
            (
                "Recipes",
                recipe_links
                    .len()
                    .saturating_sub(missing_recipes.len())
                    .to_string(),
                recipe_detail.as_str(),
                if recipe_links.is_empty() {
                    Tone::Neutral
                } else if missing_recipes.is_empty() {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            ),
            (
                "Traveler",
                traveler_value,
                traveler_detail.as_str(),
                traveler.map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
            ),
            (
                "Dispatch",
                dispatch_value,
                dispatch_detail.as_str(),
                dispatch_tone,
            ),
            (
                "Materials",
                material_count.to_string(),
                "linked to focus lot",
                Tone::Neutral,
            ),
            (
                "Guardrails",
                guardrail_value,
                guardrail_detail.as_str(),
                guardrail_tone,
            ),
            ("Yield", yield_label, "focus lot", yield_tone),
        ];
        ui_chrome::metric_tiles(ui, &metrics);
    }

    fn production_focus_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "Production Focus");
        if self.focus_lot.is_empty() {
            ui_chrome::empty_state(ui, "No production lot selected");
            return;
        }

        let lot_id = self.focus_lot.as_str();
        let lot_key = LotId::new(lot_id);
        let mes_lot = data.mes.lots.get(&lot_key);
        let traveler = data.mes.travelers.get(&lot_key);
        let route = traveler.and_then(|traveler| data.mes.routes.get(&traveler.route_id));
        let scheduler_lot = data
            .scheduler
            .lots
            .iter()
            .find(|lot| lot.id.as_str() == lot_id);
        let dispatch_result = data.scheduler.dispatch(DispatchPolicy::PriorityThenFifo);
        let assignment = dispatch_result
            .assignments
            .iter()
            .find(|assignment| assignment.lot_id.as_str() == lot_id);
        let guardrail_tone = production_gate_tone(data);

        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                traveler
                    .map(|traveler| traveler.status.label())
                    .unwrap_or("no traveler"),
                traveler.map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status)),
            );
            ui_chrome::status_pill(
                ui,
                if assignment.is_some() {
                    "scheduled"
                } else if dispatch_result
                    .unscheduled_lots
                    .iter()
                    .any(|id| id.as_str() == lot_id)
                {
                    "unscheduled"
                } else {
                    "queue"
                },
                if assignment.is_some() {
                    Tone::Success
                } else if dispatch_result
                    .unscheduled_lots
                    .iter()
                    .any(|id| id.as_str() == lot_id)
                {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            );
            ui_chrome::status_pill(ui, production_gate_label(guardrail_tone), guardrail_tone);
        });

        ui.add_space(4.0);
        if ui.available_width() < 620.0 {
            self.focus_fact(ui, "Product", focus_product_label(data, lot_id));
            self.focus_fact(ui, "Route", focus_route_label(route, data.process_flow));
            self.focus_fact(ui, "Current step", focus_step_label(data.mes, lot_id));
            self.focus_fact(
                ui,
                "Dispatch",
                focus_assignment_label(assignment, scheduler_lot),
            );
            self.focus_fact(ui, "Wafers", focus_wafer_label(mes_lot, scheduler_lot));
            self.focus_fact(
                ui,
                "Materials",
                format!(
                    "{linked} linked material lots",
                    linked = linked_material_count(data.inventory, lot_id)
                ),
            );
            self.focus_fact(ui, "Yield", focus_yield_label(data.yield_analysis, lot_id));
        } else {
            egui::Grid::new("workflow_focus_lot_grid")
                .num_columns(2)
                .striped(true)
                .min_col_width(110.0)
                .show(ui, |ui| {
                    self.focus_fact_row(ui, "Lot", lot_id);
                    self.focus_fact_row(ui, "Product", focus_product_label(data, lot_id));
                    self.focus_fact_row(ui, "Route", focus_route_label(route, data.process_flow));
                    self.focus_fact_row(ui, "Current step", focus_step_label(data.mes, lot_id));
                    self.focus_fact_row(
                        ui,
                        "Dispatch",
                        focus_assignment_label(assignment, scheduler_lot),
                    );
                    self.focus_fact_row(ui, "Wafers", focus_wafer_label(mes_lot, scheduler_lot));
                    self.focus_fact_row(
                        ui,
                        "Materials",
                        format!(
                            "{} linked material lots",
                            linked_material_count(data.inventory, lot_id)
                        ),
                    );
                    self.focus_fact_row(
                        ui,
                        "Yield",
                        focus_yield_label(data.yield_analysis, lot_id),
                    );
                });
        }

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Traveler").clicked() {
                *destination = Some(WorkflowDestination::FabControl);
            }
            if ui.button("Dispatch").clicked() {
                *destination = Some(WorkflowDestination::Scheduler);
            }
            if ui.button("Genealogy").clicked() {
                *destination = Some(WorkflowDestination::Traceability);
            }
            if ui.button("Yield").clicked() {
                *destination = Some(WorkflowDestination::Yield);
            }
            if ui.button("Notebook").clicked() {
                *destination = Some(WorkflowDestination::Notebook);
            }
        });
    }

    fn focus_fact(&self, ui: &mut egui::Ui, label: &str, value: impl ToString) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(label).strong());
            ui.add(egui::Label::new(value.to_string()).wrap());
        });
    }

    fn focus_fact_row(&self, ui: &mut egui::Ui, label: &str, value: impl ToString) {
        ui.label(RichText::new(label).strong());
        ui.add(egui::Label::new(value.to_string()).wrap());
        ui.end_row();
    }

    fn spine_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "Operational Workflow");
        let steps = self.workflow_steps(data);
        if ui.available_width() < 640.0 {
            for step in &steps {
                self.workflow_step_card(ui, step, destination);
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("workflow_spine_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("workflow_spine_grid")
                    .striped(true)
                    .min_col_width(92.0)
                    .show(ui, |ui| {
                        ui.strong("Area");
                        ui.strong("Workstream");
                        ui.strong("State");
                        ui.strong("Production context");
                        ui.strong("Action");
                        ui.end_row();

                        for step in &steps {
                            self.workflow_step_row(ui, step, destination);
                        }
                    });
            });
    }

    fn workflow_steps(&self, data: &WorkflowData<'_>) -> Vec<WorkflowStepSummary> {
        let recipe_ids = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_ids, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let notebook_count = linked_notebook_count(data.notebook, &self.focus_lot);
        let route_tone = match (
            data.process_flow.route.nodes.is_empty(),
            data.process_flow.findings().is_empty(),
        ) {
            (true, _) => Tone::Neutral,
            (false, true) => Tone::Success,
            (false, false) => Tone::Warning,
        };
        let recipe_tone = if recipe_ids.is_empty() {
            Tone::Neutral
        } else if missing_recipes.is_empty() {
            Tone::Success
        } else {
            Tone::Warning
        };
        let traveler_tone = focus_traveler(data.mes, &self.focus_lot)
            .map_or(Tone::Neutral, |traveler| traveler_tone(&traveler.status));
        let dispatch_tone = focus_dispatch_metric(data.scheduler, &self.focus_lot).2;
        let guardrail_tone = production_gate_tone(data);

        vec![
            WorkflowStepSummary {
                area: "Definition",
                title: "Mask layout",
                detail: layout_detail(data),
                tone: if data.document.shapes.is_empty() {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
                target: WorkflowDestination::Layout,
                action: "Layout",
            },
            WorkflowStepSummary {
                area: "Definition",
                title: "Process route",
                detail: route_detail(data.process_flow),
                tone: route_tone,
                target: WorkflowDestination::ProcessFlow,
                action: "Route",
            },
            WorkflowStepSummary {
                area: "Definition",
                title: "Recipe control",
                detail: recipe_detail(data.process_flow, data.recipes),
                tone: recipe_tone,
                target: WorkflowDestination::ProcessFlow,
                action: "Recipes",
            },
            WorkflowStepSummary {
                area: "Execution",
                title: "Traveler",
                detail: traveler_detail(data.mes, &self.focus_lot),
                tone: traveler_tone,
                target: WorkflowDestination::FabControl,
                action: "MES",
            },
            WorkflowStepSummary {
                area: "Execution",
                title: "Materials",
                detail: format!(
                    "{} material lot {}",
                    material_count,
                    if material_count == 1 { "link" } else { "links" }
                ),
                tone: if material_count == 0 {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
                target: WorkflowDestination::Inventory,
                action: "Inventory",
            },
            WorkflowStepSummary {
                area: "Execution",
                title: "Dispatch and tools",
                detail: dispatch_detail(data.scheduler, &self.focus_lot),
                tone: dispatch_tone,
                target: WorkflowDestination::Scheduler,
                action: "Dispatch",
            },
            WorkflowStepSummary {
                area: "Control",
                title: "Guardrails",
                detail: guardrail_detail(data),
                tone: guardrail_tone,
                target: WorkflowDestination::Safety,
                action: "Safety",
            },
            WorkflowStepSummary {
                area: "Learning",
                title: "Measure and yield",
                detail: measurement_detail(data, &self.focus_lot),
                tone: if data.yield_analysis.lot_summary(&self.focus_lot).is_some() {
                    Tone::Success
                } else {
                    Tone::Neutral
                },
                target: WorkflowDestination::Yield,
                action: "Yield",
            },
            WorkflowStepSummary {
                area: "Learning",
                title: "Engineering notes",
                detail: format!(
                    "{} notebook {} linked",
                    notebook_count,
                    if notebook_count == 1 {
                        "entry"
                    } else {
                        "entries"
                    }
                ),
                tone: if notebook_count == 0 {
                    Tone::Neutral
                } else {
                    Tone::Success
                },
                target: WorkflowDestination::Notebook,
                action: "Notebook",
            },
        ]
    }

    fn workflow_step_row(
        &self,
        ui: &mut egui::Ui,
        step: &WorkflowStepSummary,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui.label(step.area);
        ui.label(RichText::new(step.title).strong());
        ui_chrome::status_pill(ui, tone_label(step.tone), step.tone);
        ui.add(egui::Label::new(&step.detail).wrap());
        if ui.button(step.action).clicked() {
            *destination = Some(step.target);
        }
        ui.end_row();
    }

    fn workflow_step_card(
        &self,
        ui: &mut egui::Ui,
        step: &WorkflowStepSummary,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width().clamp(240.0, 520.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(step.area)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                ui_chrome::status_pill(ui, tone_label(step.tone), step.tone);
            });
            ui.label(RichText::new(step.title).strong());
            ui.add(egui::Label::new(&step.detail).wrap());
            if ui.button(step.action).clicked() {
                *destination = Some(step.target);
            }
        });
    }

    fn route_operations_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "Route Operations");
        let nodes = data
            .process_flow
            .route
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    ProcessFlowNodeKind::Operation
                        | ProcessFlowNodeKind::Measurement
                        | ProcessFlowNodeKind::Hold
                )
            })
            .collect::<Vec<_>>();

        if nodes.is_empty() {
            ui_chrome::empty_state(ui, "No operation nodes in route");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            ui_chrome::muted(
                ui,
                format!(
                    "{} / rev {} / MES {}",
                    data.process_flow.route.name,
                    data.process_flow.route.version,
                    data.process_flow.route.mes_route_id
                ),
            );
        });

        if ui.available_width() < 700.0 {
            for node in nodes {
                self.route_operation_card(ui, data, node, destination);
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("workflow_route_operations_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("workflow_route_operations")
                    .striped(true)
                    .min_col_width(88.0)
                    .show(ui, |ui| {
                        ui.strong("Step");
                        ui.strong("Area");
                        ui.strong("Recipe");
                        ui.strong("Tools");
                        ui.strong("Controls");
                        ui.strong("Action");
                        ui.end_row();

                        for node in nodes {
                            ui.label(RichText::new(&node.name).strong());
                            ui.label(&node.area);
                            ui.colored_label(
                                recipe_link_tone(data.recipes, node).color(),
                                route_node_recipe_label(node),
                            );
                            ui.label(route_node_tool_label(node));
                            ui.add(egui::Label::new(route_node_control_label(node)).wrap());
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("Route").clicked() {
                                    *destination = Some(WorkflowDestination::ProcessFlow);
                                }
                                if !node.eligible_tools.is_empty()
                                    && ui.button("Dispatch").clicked()
                                {
                                    *destination = Some(WorkflowDestination::Scheduler);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    }

    fn route_operation_card(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        node: &ProcessFlowNode,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width().clamp(240.0, 520.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(node.kind.label())
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
                ui_chrome::status_pill(
                    ui,
                    tone_label(recipe_link_tone(data.recipes, node)),
                    recipe_link_tone(data.recipes, node),
                );
            });
            ui.label(RichText::new(&node.name).strong());
            ui.small(format!("{} / {}", node.id, node.area));
            ui.label(route_node_recipe_label(node));
            ui.add(egui::Label::new(route_node_tool_label(node)).wrap());
            ui.add(egui::Label::new(route_node_control_label(node)).wrap());
            ui.horizontal_wrapped(|ui| {
                if ui.button("Route").clicked() {
                    *destination = Some(WorkflowDestination::ProcessFlow);
                }
                if !node.eligible_tools.is_empty() && ui.button("Dispatch").clicked() {
                    *destination = Some(WorkflowDestination::Scheduler);
                }
            });
        });
    }

    fn cross_link_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "Cross-Links");
        let links = cross_link_summaries(data, &self.focus_lot);
        if ui.available_width() < 640.0 {
            for link in &links {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(240.0, 520.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(link.title).strong());
                        ui_chrome::status_pill(ui, link.status, link.tone);
                    });
                    ui.add(egui::Label::new(&link.detail).wrap());
                    if ui.button(link.action).clicked() {
                        *destination = Some(link.target);
                    }
                });
            }
        } else {
            egui::ScrollArea::horizontal()
                .id_salt("workflow_cross_link_horizontal")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("workflow_cross_link_grid")
                        .striped(true)
                        .min_col_width(110.0)
                        .show(ui, |ui| {
                            ui.strong("System");
                            ui.strong("State");
                            ui.strong("Linked context");
                            ui.strong("Action");
                            ui.end_row();

                            for link in &links {
                                ui.label(RichText::new(link.title).strong());
                                ui_chrome::status_pill(ui, link.status, link.tone);
                                ui.add(egui::Label::new(&link.detail).wrap());
                                if ui.button(link.action).clicked() {
                                    *destination = Some(link.target);
                                }
                                ui.end_row();
                            }
                        });
                });
        }

        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        if recipe_links.is_empty() {
            ui_chrome::muted(ui, "No process-flow recipe links loaded.");
        } else if missing_recipes.is_empty() {
            ui_chrome::muted(ui, "All process-flow recipe links resolve.");
        } else {
            ui.colored_label(
                Tone::Warning.color(),
                format!("Missing recipe definitions: {}", missing_recipes.join(", ")),
            );
        }
    }

    fn lot_picker(&mut self, ui: &mut egui::Ui, data: &WorkflowData<'_>) {
        let lot_ids = workflow_lot_ids(data);
        egui::ComboBox::from_id_salt("workflow_focus_lot")
            .selected_text(&self.focus_lot)
            .show_ui(ui, |ui| {
                for lot_id in lot_ids {
                    ui.selectable_value(&mut self.focus_lot, lot_id.clone(), lot_id);
                }
            });
    }

    fn ensure_focus_lot(&mut self, data: &WorkflowData<'_>) {
        let lot_ids = workflow_lot_ids(data);
        if lot_ids.is_empty() {
            self.focus_lot.clear();
            return;
        }
        if !lot_ids.contains(&self.focus_lot) {
            self.focus_lot = lot_ids
                .iter()
                .find(|lot_id| lot_id.as_str() == DEFAULT_FOCUS_LOT)
                .cloned()
                .unwrap_or_else(|| lot_ids[0].clone());
        }
    }
}

fn workflow_open_action_name(destination: WorkflowDestination, suffix: &str) -> String {
    format!("{OPERAD_ACTION_OPEN}{}|{suffix}", destination.slug())
}

fn workflow_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    if metric_count > 0 {
        height += workflow_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    }
    for row_count in row_counts {
        height += workflow_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn workflow_operad_metric_columns(width: f32) -> usize {
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

fn workflow_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = workflow_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn workflow_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_workflow_operad_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    eyebrow: &str,
    title: &str,
    detail: &str,
    meta: String,
) {
    let header = document.add_child(
        parent,
        UiNode::container(
            "workflow.header",
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
    add_workflow_operad_text(
        document,
        header,
        "workflow.header.eyebrow",
        eyebrow,
        workflow_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_workflow_operad_text(
        document,
        header,
        "workflow.header.title",
        title,
        workflow_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_workflow_operad_text(
        document,
        header,
        "workflow.header.detail",
        detail,
        workflow_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_workflow_operad_text(
        document,
        header,
        "workflow.header.meta",
        meta,
        workflow_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_workflow_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[WorkflowMetricTile],
) {
    let columns = workflow_operad_metric_columns(width);
    let grid_height = workflow_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "workflow.metrics",
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
                format!("workflow.metrics.row.{row_index}"),
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
            add_workflow_operad_metric_tile(
                document,
                row,
                &format!("workflow.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_workflow_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &WorkflowMetricTile,
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
                workflow_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_workflow_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        workflow_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_workflow_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        workflow_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_workflow_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        workflow_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            workflow_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_workflow_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[WorkflowOperadRow],
) {
    let height = workflow_operad_section_height(rows.len());
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
    add_workflow_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        workflow_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_workflow_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_workflow_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_workflow_operad_empty_row(
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
    add_workflow_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        workflow_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_workflow_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &WorkflowOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        workflow_operad_tone_color(Tone::Info)
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
            workflow_operad_tone_color(row.tone),
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
    add_workflow_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        workflow_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_workflow_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        workflow_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_workflow_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("workflow.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_workflow_operad_text(
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

fn workflow_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn workflow_operad_tone_color(tone: Tone) -> ColorRgba {
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

fn workflow_has_data(data: &WorkflowData<'_>) -> bool {
    !data.document.shapes.is_empty()
        || !data.process_flow.route.nodes.is_empty()
        || !data.recipes.recipes.is_empty()
        || !data.mes.lots.is_empty()
        || !data.inventory.lots.is_empty()
        || !data.maintenance.tools.is_empty()
        || !data.environment.sensors.is_empty()
        || !data.scheduler.tools.is_empty()
        || !data.scheduler.lots.is_empty()
        || !data.safety.sensors.is_empty()
        || !data.wafer_map.dies.is_empty()
        || !data.yield_analysis.lots.is_empty()
        || !data.notebook.entries.is_empty()
        || data.equipment.tools().next().is_some()
}

fn workflow_lot_ids(data: &WorkflowData<'_>) -> Vec<String> {
    let mut ids = BTreeSet::new();
    ids.extend(data.mes.lots.keys().map(|id| id.as_str().to_string()));
    ids.extend(
        data.scheduler
            .lots
            .iter()
            .map(|lot| lot.id.as_str().to_string()),
    );
    ids.extend(data.yield_analysis.lots.iter().map(|lot| lot.id.clone()));
    ids.extend(
        data.notebook
            .entries
            .iter()
            .flat_map(|entry| entry.links.lots.iter().map(|lot| lot.as_str().to_string())),
    );
    ids.into_iter().collect()
}

fn linked_recipe_ids(process_flow: &ProcessFlowModel) -> BTreeSet<String> {
    process_flow
        .route
        .nodes
        .iter()
        .filter_map(|node| node.recipe.as_ref())
        .map(|binding| binding.recipe_id.as_str().to_string())
        .collect()
}

fn missing_recipe_ids(recipe_ids: &BTreeSet<String>, recipes: &RecipeCatalog) -> Vec<String> {
    recipe_ids
        .iter()
        .filter(|id| recipes.recipe(&RecipeId::from((*id).clone())).is_none())
        .cloned()
        .collect()
}

fn linked_material_count(inventory: &Inventory, lot_id: &str) -> usize {
    inventory
        .lots
        .values()
        .filter(|lot| {
            lot.usage.iter().any(|usage| {
                usage.links.iter().any(|link| match link {
                    FabObjectLink::Lot { lot_id: linked_lot } => linked_lot == lot_id,
                    FabObjectLink::Wafer {
                        lot_id: linked_lot, ..
                    } => linked_lot == lot_id,
                    FabObjectLink::ToolRun { .. } | FabObjectLink::LayoutShape { .. } => false,
                })
            })
        })
        .count()
}

fn linked_notebook_count(notebook: &LabNotebook, lot_id: &str) -> usize {
    notebook
        .entries
        .iter()
        .filter(|entry| entry.links.lots.iter().any(|lot| lot.as_str() == lot_id))
        .count()
}

fn focus_traveler<'a>(mes: &'a FabMesData, lot_id: &str) -> Option<&'a TravelerState> {
    mes.travelers.get(&LotId::new(lot_id))
}

fn focus_product_label(data: &WorkflowData<'_>, lot_id: &str) -> String {
    let lot_key = LotId::new(lot_id);
    data.mes
        .lots
        .get(&lot_key)
        .map(|lot| lot.product.clone())
        .or_else(|| {
            data.scheduler
                .lots
                .iter()
                .find(|lot| lot.id.as_str() == lot_id)
                .map(|lot| lot.product.clone())
        })
        .or_else(|| {
            data.yield_analysis
                .lots
                .iter()
                .find(|lot| lot.id == lot_id)
                .map(|lot| lot.product.clone())
        })
        .unwrap_or_else(|| "not loaded".to_string())
}

fn focus_route_label(route: Option<&ProcessRoute>, process_flow: &ProcessFlowModel) -> String {
    route
        .map(|route| format!("{} rev {}", route.name, route.revision))
        .unwrap_or_else(|| {
            if process_flow.route.nodes.is_empty() {
                "no route loaded".to_string()
            } else {
                format!(
                    "{} v{}",
                    process_flow.route.name, process_flow.route.version
                )
            }
        })
}

fn focus_step_label(mes: &FabMesData, lot_id: &str) -> String {
    let lot_id = LotId::new(lot_id);
    let Some(traveler) = mes.travelers.get(&lot_id) else {
        return "no MES traveler".to_string();
    };
    let Some(route) = mes.routes.get(&traveler.route_id) else {
        return format!("route {} missing", traveler.route_id);
    };
    traveler
        .current_step(route)
        .map(|step| format!("{} / {}", step.name, traveler.status.label()))
        .unwrap_or_else(|| traveler.status.label().to_string())
}

fn focus_assignment_label(
    assignment: Option<&DispatchAssignment>,
    scheduler_lot: Option<&DispatchLot>,
) -> String {
    if let Some(assignment) = assignment {
        return format!(
            "{} on {} to {}",
            assignment.lot_id,
            assignment.tool_id,
            format_shift_time(assignment.finish_minute)
        );
    }
    if let Some(lot) = scheduler_lot {
        return format!(
            "{} min {} step due {}",
            lot.process_minutes,
            lot.required_tool_class.label(),
            format_shift_time(lot.due_at_minute)
        );
    }
    "not in dispatch queue".to_string()
}

fn focus_wafer_label(mes_lot: Option<&Lot>, scheduler_lot: Option<&DispatchLot>) -> String {
    if let Some(lot) = mes_lot {
        return format!(
            "{} active / {} rework / {} scrapped",
            lot.processable_wafer_count(),
            lot.rework_wafer_count(),
            lot.scrapped_wafer_count()
        );
    }
    if let Some(lot) = scheduler_lot {
        return format!("{} queued wafers", lot.wafer_count);
    }
    "no wafer context".to_string()
}

fn focus_yield_label(analysis: &YieldAnalysis, lot_id: &str) -> String {
    analysis
        .lot_summary(lot_id)
        .map(|summary| {
            format!(
                "{:.1}% ({} failing dies)",
                summary.yield_fraction * 100.0,
                summary.failing_dies
            )
        })
        .unwrap_or_else(|| "not measured".to_string())
}

fn operation_node_count(process_flow: &ProcessFlowModel) -> usize {
    process_flow
        .route
        .nodes
        .iter()
        .filter(|node| node.kind == ProcessFlowNodeKind::Operation)
        .count()
}

fn measurement_node_count(process_flow: &ProcessFlowModel) -> usize {
    process_flow
        .route
        .nodes
        .iter()
        .filter(|node| node.kind == ProcessFlowNodeKind::Measurement)
        .count()
}

fn hold_point_count(process_flow: &ProcessFlowModel) -> usize {
    process_flow
        .route
        .nodes
        .iter()
        .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
        .count()
}

fn pluralize_count(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn traveler_tone(status: &TravelerStatus) -> Tone {
    match status {
        TravelerStatus::Running | TravelerStatus::Complete => Tone::Success,
        TravelerStatus::WaitingForStep | TravelerStatus::WaitingForSignoff => Tone::Info,
        TravelerStatus::OnHold => Tone::Warning,
        TravelerStatus::Scrapped => Tone::Danger,
    }
}

fn yield_tone(yield_fraction: f64) -> Tone {
    if yield_fraction >= 0.9 {
        Tone::Success
    } else if yield_fraction >= 0.75 {
        Tone::Warning
    } else {
        Tone::Danger
    }
}

fn focus_dispatch_metric(scheduler: &DispatchSchedule, lot_id: &str) -> (String, String, Tone) {
    let result = scheduler.dispatch(DispatchPolicy::PriorityThenFifo);
    if let Some(assignment) = result
        .assignments
        .iter()
        .find(|assignment| assignment.lot_id.as_str() == lot_id)
    {
        let value = format_shift_time(assignment.finish_minute);
        let detail = format!("{} on {}", assignment.lot_id, assignment.tool_id);
        let tone = if assignment.tardy_minutes > 0 {
            Tone::Warning
        } else {
            Tone::Success
        };
        return (value, detail, tone);
    }
    if result
        .unscheduled_lots
        .iter()
        .any(|id| id.as_str() == lot_id)
    {
        return (
            "unscheduled".to_string(),
            "no compatible available tool".to_string(),
            Tone::Warning,
        );
    }
    if scheduler.lots.iter().any(|lot| lot.id.as_str() == lot_id) {
        return (
            "queued".to_string(),
            format!("{} candidate tools", scheduler.tools.len()),
            Tone::Info,
        );
    }
    (
        "not queued".to_string(),
        format!("{} queued lots", scheduler.lots.len()),
        Tone::Neutral,
    )
}

fn maintenance_due_counts(data: &WorkflowData<'_>) -> (usize, usize) {
    let today = data.maintenance.today.unwrap_or(FabDate::new(2026, 5, 8));
    let due = data.maintenance.due_work(today);
    let overdue = due
        .iter()
        .filter(|work| work.due_state == DueState::Overdue)
        .count();
    (due.len(), overdue)
}

fn guardrail_metric(data: &WorkflowData<'_>) -> (String, String, Tone) {
    let safety = data.safety.summary();
    let environment_alarms = data.environment.active_alarms().len();
    let (due_work, overdue_work) = maintenance_due_counts(data);
    let value = safety
        .locked_out_tool_count
        .saturating_add(environment_alarms)
        .saturating_add(overdue_work)
        .to_string();
    let detail = format!(
        "{} lockouts / {} env alarms / {} due PM",
        safety.locked_out_tool_count, environment_alarms, due_work
    );
    let tone = if safety.locked_out_tool_count > 0 || overdue_work > 0 {
        Tone::Danger
    } else if environment_alarms > 0 || due_work > 0 {
        Tone::Warning
    } else if data.safety.sensors.is_empty()
        && data.environment.sensors.is_empty()
        && data.maintenance.tools.is_empty()
    {
        Tone::Neutral
    } else {
        Tone::Success
    };
    (value, detail, tone)
}

fn production_gate_tone(data: &WorkflowData<'_>) -> Tone {
    guardrail_metric(data).2
}

fn production_gate_label(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => "no gates",
        Tone::Info | Tone::Success => "released",
        Tone::Warning => "review gates",
        Tone::Danger => "blocked gates",
    }
}

fn concise_guardrail_label(data: &WorkflowData<'_>) -> String {
    let safety = data.safety.summary();
    let environment_alarms = data.environment.active_alarms().len();
    let (due_work, overdue_work) = maintenance_due_counts(data);
    format!(
        "{} lockouts, {} env alarms, {} due PM ({} overdue)",
        safety.locked_out_tool_count, environment_alarms, due_work, overdue_work
    )
}

fn route_node_recipe_label(node: &ProcessFlowNode) -> String {
    node.recipe
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            if node.kind.requires_recipe() {
                "missing recipe".to_string()
            } else {
                "not required".to_string()
            }
        })
}

fn route_node_tool_label(node: &ProcessFlowNode) -> String {
    if node.allowed_tool_classes.is_empty() && node.eligible_tools.is_empty() {
        return "no tool coverage".to_string();
    }
    let classes = node
        .allowed_tool_classes
        .iter()
        .map(|class| class.label())
        .collect::<Vec<_>>()
        .join(", ");
    let tools = if node.eligible_tools.is_empty() {
        "no eligible tools".to_string()
    } else {
        node.eligible_tools.join(", ")
    };
    if classes.is_empty() {
        tools
    } else {
        format!("{classes}: {tools}")
    }
}

fn route_node_control_label(node: &ProcessFlowNode) -> String {
    let mut controls = Vec::new();
    if node.hold_point || node.kind == ProcessFlowNodeKind::Hold {
        controls.push("hold point".to_string());
    }
    if let Some(checkpoint) = &node.measurement_checkpoint {
        controls.push(format!(
            "{} / {}",
            checkpoint.measurement_name, checkpoint.sample_plan
        ));
    }
    if !node.expected_inputs.is_empty() {
        controls.push(format!("inputs {}", node.expected_inputs.join(", ")));
    }
    if !node.expected_outputs.is_empty() {
        controls.push(format!("outputs {}", node.expected_outputs.join(", ")));
    }
    if controls.is_empty() {
        "standard route control".to_string()
    } else {
        controls.join("; ")
    }
}

fn recipe_link_tone(recipes: &RecipeCatalog, node: &ProcessFlowNode) -> Tone {
    match &node.recipe {
        Some(binding) if recipes.recipe(&binding.recipe_id).is_some() => Tone::Success,
        Some(_) => Tone::Warning,
        None if node.kind.requires_recipe() => Tone::Danger,
        None => Tone::Neutral,
    }
}

fn cross_link_summaries(data: &WorkflowData<'_>, lot_id: &str) -> Vec<CrossLinkSummary> {
    let material_count = linked_material_count(data.inventory, lot_id);
    let notebook_count = linked_notebook_count(data.notebook, lot_id);
    let safety = data.safety.summary();
    let environment_alarms = data.environment.active_alarms().len();
    let (due_work, overdue_work) = maintenance_due_counts(data);
    let yield_summary = data.yield_analysis.lot_summary(lot_id);

    vec![
        CrossLinkSummary {
            title: "Traceability",
            status: if material_count > 0 {
                "linked"
            } else {
                "empty"
            },
            detail: format!(
                "{material_count} focus material links, {notebook_count} notebook entries"
            ),
            tone: if material_count > 0 {
                Tone::Success
            } else {
                Tone::Neutral
            },
            target: WorkflowDestination::Traceability,
            action: "Trace",
        },
        CrossLinkSummary {
            title: "Metrology",
            status: if data.wafer_map.dies.is_empty() {
                "empty"
            } else {
                "linked"
            },
            detail: format!(
                "{} wafer-map dies, {} process measurements",
                data.wafer_map.dies.len(),
                data.yield_analysis.process_measurements.len()
            ),
            tone: if data.wafer_map.dies.is_empty() {
                Tone::Neutral
            } else {
                Tone::Success
            },
            target: WorkflowDestination::Metrology,
            action: "Metrology",
        },
        CrossLinkSummary {
            title: "Maintenance",
            status: if overdue_work > 0 {
                "blocked"
            } else if due_work > 0 {
                "review"
            } else {
                "ready"
            },
            detail: format!(
                "{} due work items, {} open downtime events",
                due_work,
                data.maintenance
                    .downtime
                    .iter()
                    .filter(|record| record.ended_at.is_none())
                    .count()
            ),
            tone: if overdue_work > 0 {
                Tone::Danger
            } else if due_work > 0 {
                Tone::Warning
            } else if data.maintenance.tools.is_empty() {
                Tone::Neutral
            } else {
                Tone::Success
            },
            target: WorkflowDestination::Maintenance,
            action: "Maintenance",
        },
        CrossLinkSummary {
            title: "Environment",
            status: if environment_alarms > 0 {
                "review"
            } else if data.environment.sensors.is_empty() {
                "empty"
            } else {
                "ready"
            },
            detail: format!(
                "{} active alarms, {} sensors, {} correlations",
                environment_alarms,
                data.environment.sensors.len(),
                data.environment.correlations.len()
            ),
            tone: if environment_alarms > 0 {
                Tone::Warning
            } else if data.environment.sensors.is_empty() {
                Tone::Neutral
            } else {
                Tone::Success
            },
            target: WorkflowDestination::Environment,
            action: "Environment",
        },
        CrossLinkSummary {
            title: "Safety",
            status: if safety.locked_out_tool_count > 0 {
                "blocked"
            } else if safety.active_condition_count > 0 || safety.open_incident_count > 0 {
                "review"
            } else if safety.sensor_count == 0 {
                "empty"
            } else {
                "ready"
            },
            detail: format!(
                "{} active conditions, {} tool lockouts, {} open incidents",
                safety.active_condition_count,
                safety.locked_out_tool_count,
                safety.open_incident_count
            ),
            tone: if safety.locked_out_tool_count > 0 {
                Tone::Danger
            } else if safety.active_condition_count > 0 || safety.open_incident_count > 0 {
                Tone::Warning
            } else if safety.sensor_count == 0 {
                Tone::Neutral
            } else {
                Tone::Success
            },
            target: WorkflowDestination::Safety,
            action: "Safety",
        },
        CrossLinkSummary {
            title: "Yield",
            status: if yield_summary.is_some() {
                "linked"
            } else {
                "empty"
            },
            detail: yield_summary
                .map(|summary| {
                    format!(
                        "{:.1}% focus yield, {} root-cause hints",
                        summary.yield_fraction * 100.0,
                        summary.root_cause_hints.len()
                    )
                })
                .unwrap_or_else(|| "no focus lot yield summary".to_string()),
            tone: yield_summary.map_or(Tone::Neutral, |summary| yield_tone(summary.yield_fraction)),
            target: WorkflowDestination::Yield,
            action: "Yield",
        },
    ]
}

fn layout_detail(data: &WorkflowData<'_>) -> String {
    if data.document.shapes.is_empty() {
        "blank workspace; load demo data or import a layout".to_string()
    } else {
        format!(
            "{} shapes in {}",
            data.document.shapes.len(),
            data.process_flow.route.mask_design_id
        )
    }
}

fn route_detail(process_flow: &ProcessFlowModel) -> String {
    let findings = process_flow.findings();
    if findings.is_empty() {
        format!(
            "{} nodes, no validation findings",
            process_flow.route.nodes.len()
        )
    } else {
        format!(
            "{} nodes, {} validation findings",
            process_flow.route.nodes.len(),
            findings.len()
        )
    }
}

fn recipe_detail(process_flow: &ProcessFlowModel, recipes: &RecipeCatalog) -> String {
    let recipe_ids = linked_recipe_ids(process_flow);
    let missing = missing_recipe_ids(&recipe_ids, recipes);
    if recipe_ids.is_empty() {
        "no recipes linked from process flow".to_string()
    } else if missing.is_empty() {
        format!("{} linked recipes resolve", recipe_ids.len())
    } else {
        format!("{} linked, {} missing", recipe_ids.len(), missing.len())
    }
}

fn traveler_detail(mes: &FabMesData, lot_id: &str) -> String {
    let lot_id = LotId::new(lot_id);
    let Some(traveler) = mes.travelers.get(&lot_id) else {
        return "no MES traveler for focus lot".to_string();
    };
    let Some(route) = mes.routes.get(&traveler.route_id) else {
        return format!("traveler route {} is missing", traveler.route_id);
    };
    let step = traveler
        .current_step(route)
        .map(|step| step.name.as_str())
        .unwrap_or("complete");
    let status = match traveler.status {
        TravelerStatus::WaitingForStep => "waiting",
        TravelerStatus::Running => "running",
        TravelerStatus::WaitingForSignoff => "awaiting signoff",
        TravelerStatus::OnHold => "on hold",
        TravelerStatus::Complete => "complete",
        TravelerStatus::Scrapped => "scrapped",
    };
    format!(
        "{status} at {step}; {} completed steps",
        traveler.completed_steps.len()
    )
}

fn dispatch_detail(scheduler: &DispatchSchedule, lot_id: &str) -> String {
    let result = scheduler.dispatch(DispatchPolicy::PriorityThenFifo);
    let lot_id = LotId::new(lot_id);
    if let Some(assignment) = result
        .assignments
        .iter()
        .find(|assignment| assignment.lot_id == lot_id)
    {
        format!(
            "{} on {} finishes {}",
            assignment.lot_id,
            assignment.tool_id,
            format_shift_time(assignment.finish_minute)
        )
    } else if result.unscheduled_lots.iter().any(|id| id == &lot_id) {
        "focus lot is unscheduled".to_string()
    } else {
        format!(
            "{} queued lots across {} tools",
            scheduler.lots.len(),
            scheduler.tools.len()
        )
    }
}

fn guardrail_detail(data: &WorkflowData<'_>) -> String {
    let safety = data.safety.summary();
    let environment_alarms = data.environment.active_alarms().len();
    let due_work = data
        .maintenance
        .due_work(data.maintenance.today.unwrap_or(FabDate::new(2026, 5, 8)))
        .len();
    format!(
        "{} safety lockouts, {} environment alarms, {} due maintenance items",
        safety.locked_out_tool_count, environment_alarms, due_work
    )
}

fn measurement_detail(data: &WorkflowData<'_>, lot_id: &str) -> String {
    let yield_label = data
        .yield_analysis
        .lot_summary(lot_id)
        .map(|summary| format!("{:.1}% lot yield", summary.yield_fraction * 100.0))
        .unwrap_or_else(|| "no yield summary loaded".to_string());
    format!(
        "{}; {} wafer-map dies; {} equipment tools",
        yield_label,
        data.wafer_map.dies.len(),
        data.equipment.tools().count()
    )
}

fn tone_label(tone: Tone) -> &'static str {
    match tone {
        Tone::Neutral => "empty",
        Tone::Info => "linked",
        Tone::Success => "ready",
        Tone::Warning => "review",
        Tone::Danger => "blocked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WorkflowTestData {
        document: Document,
        process_flow: ProcessFlowModel,
        recipes: RecipeCatalog,
        mes: FabMesData,
        inventory: Inventory,
        maintenance: MaintenanceModel,
        environment: CleanroomEnvironment,
        scheduler: DispatchSchedule,
        safety: SafetySystem,
        equipment: EquipmentSimulator,
        wafer_map: WaferMap,
        yield_analysis: YieldAnalysis,
        notebook: LabNotebook,
    }

    impl WorkflowTestData {
        fn sample() -> Self {
            Self {
                document: Document::demo(),
                process_flow: ProcessFlowModel::sample(),
                recipes: RecipeCatalog::sample(),
                mes: FabMesData::sample(),
                inventory: Inventory::sample(),
                maintenance: MaintenanceModel::sample(),
                environment: CleanroomEnvironment::sample(),
                scheduler: DispatchSchedule::sample(),
                safety: SafetySystem::simulated_demo(),
                equipment: EquipmentSimulator::demo_fab(),
                wafer_map: WaferMap::synthetic_demo(),
                yield_analysis: YieldAnalysis::synthetic(),
                notebook: LabNotebook::sample(),
            }
        }

        fn data(&self) -> WorkflowData<'_> {
            WorkflowData {
                document: &self.document,
                process_flow: &self.process_flow,
                recipes: &self.recipes,
                mes: &self.mes,
                inventory: &self.inventory,
                maintenance: &self.maintenance,
                environment: &self.environment,
                scheduler: &self.scheduler,
                safety: &self.safety,
                equipment: &self.equipment,
                wafer_map: &self.wafer_map,
                yield_analysis: &self.yield_analysis,
                notebook: &self.notebook,
            }
        }
    }

    #[test]
    fn workflow_operad_view_audits_common_widths() {
        let bundle = WorkflowTestData::sample();
        let mut panel = WorkflowPanel::default();
        let data = bundle.data();
        panel.ensure_focus_lot(&data);
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width, &data);
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
    fn workflow_operad_actions_update_focus_and_return_actions() {
        let bundle = WorkflowTestData::sample();
        let mut panel = WorkflowPanel::default();
        let data = bundle.data();
        panel.ensure_focus_lot(&data);

        assert_eq!(
            panel.handle_operad_action(&workflow_open_action_name(
                WorkflowDestination::Scheduler,
                "test"
            )),
            Some(WorkflowAction::Open(WorkflowDestination::Scheduler))
        );
        assert_eq!(
            panel.handle_operad_action(OPERAD_ACTION_LOAD_DEMO),
            Some(WorkflowAction::LoadDemoWorkspace)
        );

        let target_lot = workflow_lot_ids(&data)
            .into_iter()
            .find(|lot_id| lot_id != panel.focus_lot())
            .expect("sample workflow has multiple lot ids");
        assert!(panel.handle_operad_focus_action(
            &format!("{OPERAD_ACTION_FOCUS_LOT}{target_lot}|test"),
            &data
        ));
        assert_eq!(panel.focus_lot(), target_lot);
    }
}
