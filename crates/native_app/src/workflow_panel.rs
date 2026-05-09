use std::collections::BTreeSet;

use eframe::egui::{self, RichText};
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

use crate::ui_chrome::{self, Tone};

const DEFAULT_FOCUS_LOT: &str = "L-00042";

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
        let has_workspace_data = workflow_has_data(&data);
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
                            self.lot_picker(ui, &data);
                        } else if ui.button("Load demo workspace").clicked() {
                            load_demo = true;
                        }
                    },
                );

                if !has_workspace_data {
                    ui_chrome::empty_state(ui, "No workspace data loaded");
                    return;
                }

                self.metric_row(ui, &data);
                ui.separator();
                self.production_focus_ui(ui, &data, &mut destination);
                ui.separator();

                if ui.available_width() >= 940.0 {
                    ui.columns(2, |columns| {
                        self.spine_ui(&mut columns[0], &data, &mut destination);
                        self.route_operations_ui(&mut columns[1], &data, &mut destination);
                    });
                    ui.separator();
                    self.cross_link_ui(ui, &data, &mut destination);
                } else {
                    self.spine_ui(ui, &data, &mut destination);
                    ui.separator();
                    self.route_operations_ui(ui, &data, &mut destination);
                    ui.separator();
                    self.cross_link_ui(ui, &data, &mut destination);
                }
            });

        if load_demo {
            Some(WorkflowAction::LoadDemoWorkspace)
        } else {
            destination.map(WorkflowAction::Open)
        }
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, data: WorkflowData<'_>) {
        self.ensure_focus_lot(&data);
        ui_chrome::section_label(ui, "Workflow");
        if workflow_lot_ids(&data).is_empty() {
            ui_chrome::muted(ui, "No lot loaded");
        } else {
            self.lot_picker(ui, &data);
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
                focus_product_label(&data, &self.focus_lot)
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
        ui.label(format!("Guardrails: {}", concise_guardrail_label(&data)));
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
                "{} focus material links, {} notebook entries",
                material_count, notebook_count
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
