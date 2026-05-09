use std::collections::BTreeSet;

use eframe::egui::{self, RichText};
use layout_model::{
    Document,
    environment::CleanroomEnvironment,
    equipment::EquipmentSimulator,
    inventory::{FabObjectLink, Inventory},
    maintenance::{FabDate, MaintenanceModel},
    mes::{FabMesData, LotId, TravelerStatus},
    metrology::WaferMap,
    notebook::LabNotebook,
    process_flow::ProcessFlowModel,
    recipe::{RecipeCatalog, RecipeId},
    safety::SafetySystem,
    scheduler::{DispatchPolicy, DispatchSchedule, format_shift_time},
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

                if !self.focus_lot.is_empty() {
                    ui_chrome::muted(ui, format!("Focus lot {}", self.focus_lot));
                    ui.add_space(4.0);
                }

                self.metric_row(ui, &data);
                ui.separator();
                self.spine_ui(ui, &data, &mut destination);
                ui.separator();
                self.cross_link_ui(ui, &data, &mut destination);
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

        ui.label(format!(
            "Route nodes: {}",
            data.process_flow.route.nodes.len()
        ));
        ui.label(format!("Route recipes: {}", recipe_links.len()));
        ui.label(format!("Missing recipes: {}", missing_recipes.len()));
        ui.label(format!("Linked materials: {material_count}"));
        ui.label(format!("Notebook entries: {notebook_count}"));
    }

    fn metric_row(&self, ui: &mut egui::Ui, data: &WorkflowData<'_>) {
        let recipe_links = linked_recipe_ids(data.process_flow);
        let missing_recipes = missing_recipe_ids(&recipe_links, data.recipes);
        let material_count = linked_material_count(data.inventory, &self.focus_lot);
        let safety_summary = data.safety.summary();
        let yield_label = data
            .yield_analysis
            .lot_summary(&self.focus_lot)
            .map(|summary| format!("{:.1}%", summary.yield_fraction * 100.0))
            .unwrap_or_else(|| "n/a".to_string());

        let metrics = [
            (
                "Layout",
                data.document.shapes.len().to_string(),
                "shapes in workspace",
                Tone::Neutral,
            ),
            (
                "Route",
                data.process_flow.route.nodes.len().to_string(),
                "process steps",
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
                "linked and found",
                if recipe_links.is_empty() {
                    Tone::Neutral
                } else if missing_recipes.is_empty() {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            ),
            (
                "Materials",
                material_count.to_string(),
                "linked lots",
                Tone::Neutral,
            ),
            (
                "Safety",
                safety_summary.locked_out_tool_count.to_string(),
                "tool lockouts",
                if data.safety.sensors.is_empty() && safety_summary.locked_out_tool_count == 0 {
                    Tone::Neutral
                } else if safety_summary.locked_out_tool_count == 0 {
                    Tone::Success
                } else {
                    Tone::Warning
                },
            ),
            ("Yield", yield_label, "focus lot", Tone::Neutral),
        ];
        ui_chrome::metric_tiles(ui, &metrics);
    }

    fn spine_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "End-to-End Flow");
        egui::Grid::new("workflow_spine_grid")
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                self.workflow_step(
                    ui,
                    "1",
                    "Mask layout",
                    layout_detail(data),
                    if data.document.shapes.is_empty() {
                        Tone::Neutral
                    } else {
                        Tone::Success
                    },
                    WorkflowDestination::Layout,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "2",
                    "Process route",
                    route_detail(data.process_flow),
                    match (
                        data.process_flow.route.nodes.is_empty(),
                        data.process_flow.findings().is_empty(),
                    ) {
                        (true, _) => Tone::Neutral,
                        (false, true) => Tone::Success,
                        (false, false) => Tone::Warning,
                    },
                    WorkflowDestination::ProcessFlow,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "3",
                    "Recipe control",
                    recipe_detail(data.process_flow, data.recipes),
                    {
                        let recipe_ids = linked_recipe_ids(data.process_flow);
                        if recipe_ids.is_empty() {
                            Tone::Neutral
                        } else if missing_recipe_ids(&recipe_ids, data.recipes).is_empty() {
                            Tone::Success
                        } else {
                            Tone::Warning
                        }
                    },
                    WorkflowDestination::ProcessFlow,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "4",
                    "Traveler",
                    traveler_detail(data.mes, &self.focus_lot),
                    if data
                        .mes
                        .travelers
                        .contains_key(&LotId::new(self.focus_lot.as_str()))
                    {
                        Tone::Success
                    } else {
                        Tone::Neutral
                    },
                    WorkflowDestination::FabControl,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "5",
                    "Materials",
                    format!(
                        "{} material lot links",
                        linked_material_count(data.inventory, &self.focus_lot)
                    ),
                    if linked_material_count(data.inventory, &self.focus_lot) == 0 {
                        Tone::Neutral
                    } else {
                        Tone::Success
                    },
                    WorkflowDestination::Inventory,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "6",
                    "Dispatch and tools",
                    dispatch_detail(data.scheduler, &self.focus_lot),
                    if data
                        .scheduler
                        .lots
                        .iter()
                        .any(|lot| lot.id.as_str() == self.focus_lot)
                    {
                        Tone::Success
                    } else {
                        Tone::Neutral
                    },
                    WorkflowDestination::Scheduler,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "7",
                    "Facility guardrails",
                    guardrail_detail(data),
                    if data.safety.sensors.is_empty()
                        && data.environment.sensors.is_empty()
                        && data.maintenance.tools.is_empty()
                    {
                        Tone::Neutral
                    } else if data.safety.summary().locked_out_tool_count == 0 {
                        Tone::Success
                    } else {
                        Tone::Warning
                    },
                    WorkflowDestination::Safety,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "8",
                    "Measure and learn",
                    measurement_detail(data, &self.focus_lot),
                    if data.yield_analysis.lot_summary(&self.focus_lot).is_some() {
                        Tone::Success
                    } else {
                        Tone::Neutral
                    },
                    WorkflowDestination::Yield,
                    destination,
                );
                self.workflow_step(
                    ui,
                    "9",
                    "Engineering notes",
                    format!(
                        "{} notebook entries linked",
                        linked_notebook_count(data.notebook, &self.focus_lot)
                    ),
                    if linked_notebook_count(data.notebook, &self.focus_lot) == 0 {
                        Tone::Neutral
                    } else {
                        Tone::Success
                    },
                    WorkflowDestination::Notebook,
                    destination,
                );
            });
    }

    fn workflow_step(
        &self,
        ui: &mut egui::Ui,
        number: &str,
        title: &str,
        detail: String,
        tone: Tone,
        target: WorkflowDestination,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui.label(RichText::new(number).strong());
        ui.label(RichText::new(title).strong());
        ui_chrome::status_pill(ui, tone_label(tone), tone);
        ui.label(detail);
        if ui.button("Open").clicked() {
            *destination = Some(target);
        }
        ui.end_row();
    }

    fn cross_link_ui(
        &self,
        ui: &mut egui::Ui,
        data: &WorkflowData<'_>,
        destination: &mut Option<WorkflowDestination>,
    ) {
        ui_chrome::section_label(ui, "Shared Objects");
        let mut links = |ui: &mut egui::Ui| {
            if ui.button("Trace genealogy").clicked() {
                *destination = Some(WorkflowDestination::Traceability);
            }
            if ui.button("Open metrology").clicked() {
                *destination = Some(WorkflowDestination::Metrology);
            }
            if ui.button("Open maintenance").clicked() {
                *destination = Some(WorkflowDestination::Maintenance);
            }
            if ui.button("Open environment").clicked() {
                *destination = Some(WorkflowDestination::Environment);
            }
        };
        if ui.ctx().content_rect().width() < 760.0 || ui.available_width() < 760.0 {
            ui.vertical(|ui| links(ui));
        } else {
            ui.horizontal_wrapped(|ui| links(ui));
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
