use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    equipment::{EquipmentSimulator, ToolState as EquipmentToolState},
    mes::{LotId, ProcessStepId, RecipeId, ToolClass, ToolId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPolicy {
    Fifo,
    PriorityThenFifo,
    DueDateThenPriority,
}

impl DispatchPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fifo => "FIFO",
            Self::PriorityThenFifo => "priority + FIFO",
            Self::DueDateThenPriority => "due date + priority",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDispatchState {
    Available,
    Maintenance,
    Down,
}

impl ToolDispatchState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Maintenance => "maintenance",
            Self::Down => "down",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub start_minute: u32,
    pub end_minute: u32,
    pub reason: String,
}

impl MaintenanceWindow {
    pub fn overlaps(&self, start_minute: u32, end_minute: u32) -> bool {
        start_minute < self.end_minute && end_minute > self.start_minute
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchTool {
    pub id: ToolId,
    pub name: String,
    pub class: ToolClass,
    pub state: ToolDispatchState,
    #[serde(default)]
    pub compatible_recipes: Vec<RecipeId>,
    #[serde(default)]
    pub maintenance_windows: Vec<MaintenanceWindow>,
}

impl DispatchTool {
    pub fn can_process(&self, lot: &DispatchLot) -> bool {
        self.state == ToolDispatchState::Available
            && self.class == lot.required_tool_class
            && (self.compatible_recipes.is_empty()
                || self.compatible_recipes.contains(&lot.recipe_id))
    }

    pub fn next_start_after(&self, requested_start: u32, duration_minutes: u32) -> u32 {
        let mut start = requested_start;
        loop {
            let end = start.saturating_add(duration_minutes);
            if let Some(window) = self
                .maintenance_windows
                .iter()
                .filter(|window| window.overlaps(start, end))
                .min_by_key(|window| window.start_minute)
            {
                start = window.end_minute;
            } else {
                return start;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchLot {
    pub id: LotId,
    pub product: String,
    pub priority: u8,
    pub fifo_sequence: u32,
    pub ready_at_minute: u32,
    pub due_at_minute: u32,
    pub step_id: ProcessStepId,
    pub step_name: String,
    pub required_tool_class: ToolClass,
    pub recipe_id: RecipeId,
    pub process_minutes: u32,
    pub wafer_count: u16,
}

impl DispatchLot {
    pub fn slack_minutes_at(&self, minute: u32) -> i32 {
        self.due_at_minute as i32 - minute as i32 - self.process_minutes as i32
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchAssignment {
    pub lot_id: LotId,
    pub tool_id: ToolId,
    pub start_minute: u32,
    pub finish_minute: u32,
    pub due_at_minute: u32,
    pub priority: u8,
    pub wait_minutes: u32,
    pub tardy_minutes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRecommendation {
    pub tool_id: ToolId,
    pub lot_id: Option<LotId>,
    pub reason: String,
    pub start_minute: Option<u32>,
    pub finish_minute: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueSummary {
    pub tool_class: ToolClass,
    pub waiting_lots: usize,
    pub total_process_minutes: u32,
    pub earliest_due_minute: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchResult {
    pub assignments: Vec<DispatchAssignment>,
    pub recommendations: Vec<ToolRecommendation>,
    pub queue_summaries: Vec<QueueSummary>,
    pub unscheduled_lots: Vec<LotId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchSchedule {
    pub now_minute: u32,
    #[serde(default)]
    pub tools: Vec<DispatchTool>,
    #[serde(default)]
    pub lots: Vec<DispatchLot>,
    #[serde(default)]
    pub assignments: Vec<DispatchAssignment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerValidationFinding {
    pub severity: SchedulerValidationSeverity,
    pub message: String,
}

impl SchedulerValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: SchedulerValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: SchedulerValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchedulerValidationContext {
    pub recipe_ids: BTreeSet<String>,
    pub mes_lot_ids: BTreeSet<String>,
    pub mes_lot_routes: BTreeMap<String, String>,
    pub route_steps: BTreeMap<String, BTreeSet<String>>,
    pub route_step_recipes: BTreeMap<(String, String), String>,
    pub route_step_tool_classes: BTreeMap<(String, String), ToolClass>,
    pub mes_eligible_tool_ids: BTreeSet<String>,
    pub equipment_tool_states: BTreeMap<String, EquipmentToolState>,
    pub equipment_tool_recipe_ids: BTreeMap<String, BTreeSet<String>>,
    pub equipment_active_lot_ids: BTreeMap<String, String>,
}

impl SchedulerValidationContext {
    pub fn from_recipe_ids_and_mes<I>(recipe_ids: I, mes: &crate::mes::FabMesData) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut route_steps = BTreeMap::<String, BTreeSet<String>>::new();
        let mut route_step_recipes = BTreeMap::new();
        let mut route_step_tool_classes = BTreeMap::new();
        let mut mes_eligible_tool_ids = BTreeSet::new();
        for route in mes.routes.values() {
            let route_id = route.id.as_str().to_string();
            let steps = route_steps.entry(route_id.clone()).or_default();
            for step in &route.steps {
                let step_id = step.id.as_str().to_string();
                steps.insert(step_id.clone());
                route_step_recipes.insert(
                    (route_id.clone(), step_id.clone()),
                    step.required_recipe.as_str().to_string(),
                );
                route_step_tool_classes
                    .insert((route_id.clone(), step_id), step.required_tool_class);
                for tool_id in &step.eligible_tools {
                    mes_eligible_tool_ids.insert(tool_id.as_str().to_string());
                }
            }
        }

        Self {
            recipe_ids: recipe_ids.into_iter().collect(),
            mes_lot_ids: mes
                .lots
                .keys()
                .map(|lot_id| lot_id.as_str().to_string())
                .collect(),
            mes_lot_routes: mes
                .lots
                .values()
                .map(|lot| {
                    (
                        lot.id.as_str().to_string(),
                        lot.route_id.as_str().to_string(),
                    )
                })
                .collect(),
            route_steps,
            route_step_recipes,
            route_step_tool_classes,
            mes_eligible_tool_ids,
            equipment_tool_states: BTreeMap::new(),
            equipment_tool_recipe_ids: BTreeMap::new(),
            equipment_active_lot_ids: BTreeMap::new(),
        }
    }

    pub fn with_equipment(mut self, equipment: &EquipmentSimulator) -> Self {
        for tool in equipment.tools() {
            let tool_id = tool.id.as_str().to_string();
            self.equipment_tool_states
                .insert(tool_id.clone(), tool.state);
            if tool.state == EquipmentToolState::Running
                && let Some(lot_id) = tool
                    .active_run
                    .as_ref()
                    .and_then(|run| run.recipe.lot_id.as_deref())
                    .map(str::trim)
                    .filter(|lot_id| !lot_id.is_empty())
            {
                self.equipment_active_lot_ids
                    .insert(tool_id.clone(), lot_id.to_string());
            }
            self.equipment_tool_recipe_ids.insert(
                tool_id,
                tool.available_recipes
                    .keys()
                    .map(|recipe_id| recipe_id.as_str().to_string())
                    .collect(),
            );
        }
        self
    }

    fn contains_recipe(&self, recipe_id: &RecipeId) -> bool {
        self.recipe_ids.contains(recipe_id.as_str())
    }

    fn contains_mes_lot(&self, lot_id: &LotId) -> bool {
        self.mes_lot_ids.contains(lot_id.as_str())
    }

    fn mes_route_for_lot(&self, lot_id: &LotId) -> Option<&str> {
        self.mes_lot_routes.get(lot_id.as_str()).map(String::as_str)
    }

    fn contains_step_on_route(&self, route_id: &str, step_id: &ProcessStepId) -> bool {
        self.route_steps
            .get(route_id)
            .is_some_and(|steps| steps.contains(step_id.as_str()))
    }

    fn step_recipe(&self, route_id: &str, step_id: &ProcessStepId) -> Option<&str> {
        self.route_step_recipes
            .get(&(route_id.to_string(), step_id.as_str().to_string()))
            .map(String::as_str)
    }

    fn step_tool_class(&self, route_id: &str, step_id: &ProcessStepId) -> Option<ToolClass> {
        self.route_step_tool_classes
            .get(&(route_id.to_string(), step_id.as_str().to_string()))
            .copied()
    }

    fn contains_eligible_tool(&self, tool_id: &ToolId) -> bool {
        self.mes_eligible_tool_ids.contains(tool_id.as_str())
    }

    fn equipment_tool_state(&self, tool_id: &ToolId) -> Option<EquipmentToolState> {
        self.equipment_tool_states.get(tool_id.as_str()).copied()
    }

    fn equipment_tool_has_recipe(&self, tool_id: &ToolId, recipe_id: &RecipeId) -> Option<bool> {
        self.equipment_tool_recipe_ids
            .get(tool_id.as_str())
            .map(|recipes| recipes.contains(recipe_id.as_str()))
    }

    fn equipment_active_lot(&self, tool_id: &ToolId) -> Option<&str> {
        self.equipment_active_lot_ids
            .get(tool_id.as_str())
            .map(String::as_str)
    }
}

impl DispatchSchedule {
    pub fn sample() -> Self {
        let mut schedule = Self {
            now_minute: 8 * 60,
            tools: sample_tools(),
            lots: sample_lots(),
            assignments: Vec::new(),
        };
        schedule.assignments = schedule
            .dispatch(DispatchPolicy::PriorityThenFifo)
            .assignments;
        schedule
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.lots.is_empty() && self.assignments.is_empty()
    }

    pub fn validate(&self) -> Vec<SchedulerValidationFinding> {
        let mut findings = Vec::new();
        self.validate_internal(&mut findings);
        findings
    }

    pub fn validate_with_context(
        &self,
        context: &SchedulerValidationContext,
    ) -> Vec<SchedulerValidationFinding> {
        let mut findings = self.validate();
        self.validate_context_links(context, &mut findings);
        findings
    }

    pub fn is_valid(&self) -> bool {
        self.validate()
            .iter()
            .all(|finding| finding.severity != SchedulerValidationSeverity::Error)
    }

    fn validate_internal(&self, findings: &mut Vec<SchedulerValidationFinding>) {
        let mut tool_ids = BTreeSet::new();
        let mut tools_by_id = BTreeMap::new();
        for tool in &self.tools {
            if tool_ids.insert(tool.id.clone()) {
                tools_by_id.insert(tool.id.clone(), tool);
            } else {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler contains duplicate tool {}",
                    tool.id
                )));
            }
            if tool.id.as_str().trim().is_empty() {
                findings.push(SchedulerValidationFinding::error(
                    "scheduler tool id cannot be empty",
                ));
            }
            if tool.name.trim().is_empty() {
                findings.push(SchedulerValidationFinding::warning(format!(
                    "scheduler tool {} has no display name",
                    tool.id
                )));
            }
            validate_maintenance_windows(tool, findings);
        }

        let mut lot_ids = BTreeSet::new();
        let mut lots_by_id = BTreeMap::new();
        for lot in &self.lots {
            if lot_ids.insert(lot.id.clone()) {
                lots_by_id.insert(lot.id.clone(), lot);
            } else {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler contains duplicate lot {}",
                    lot.id
                )));
            }
            validate_dispatch_lot(lot, findings);
        }

        let mut assigned_lots = BTreeSet::new();
        let mut assignments_by_tool = BTreeMap::<ToolId, Vec<&DispatchAssignment>>::new();
        for assignment in &self.assignments {
            if !assigned_lots.insert(assignment.lot_id.clone()) {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler lot {} has multiple persisted assignments",
                    assignment.lot_id
                )));
            }
            assignments_by_tool
                .entry(assignment.tool_id.clone())
                .or_default()
                .push(assignment);
            validate_assignment(assignment, &lots_by_id, &tools_by_id, findings);
        }
        validate_tool_assignment_overlaps(assignments_by_tool, findings);
    }

    fn validate_context_links(
        &self,
        context: &SchedulerValidationContext,
        findings: &mut Vec<SchedulerValidationFinding>,
    ) {
        for lot in &self.lots {
            if !context.contains_recipe(&lot.recipe_id) {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler lot {} references missing recipe {}",
                    lot.id, lot.recipe_id
                )));
            }
            if !context.contains_mes_lot(&lot.id) {
                findings.push(SchedulerValidationFinding::warning(format!(
                    "scheduler lot {} is not present in MES lots",
                    lot.id
                )));
            } else if let Some(route_id) = context.mes_route_for_lot(&lot.id) {
                if !context.contains_step_on_route(route_id, &lot.step_id) {
                    findings.push(SchedulerValidationFinding::error(format!(
                        "scheduler lot {} references missing MES step {} on route {route_id}",
                        lot.id, lot.step_id
                    )));
                } else {
                    if let Some(required_recipe) = context.step_recipe(route_id, &lot.step_id)
                        && required_recipe != lot.recipe_id.as_str()
                    {
                        findings.push(SchedulerValidationFinding::error(format!(
                            "scheduler lot {} recipe {} does not match MES step {} required recipe {required_recipe}",
                            lot.id, lot.recipe_id, lot.step_id
                        )));
                    }
                    if let Some(required_class) = context.step_tool_class(route_id, &lot.step_id)
                        && required_class != lot.required_tool_class
                    {
                        findings.push(SchedulerValidationFinding::error(format!(
                            "scheduler lot {} tool class {} does not match MES step {} class {}",
                            lot.id, lot.required_tool_class, lot.step_id, required_class
                        )));
                    }
                }
            }
        }
        for tool in &self.tools {
            if !context.contains_eligible_tool(&tool.id) {
                findings.push(SchedulerValidationFinding::warning(format!(
                    "scheduler tool {} is not listed as eligible on any MES route step",
                    tool.id
                )));
            }
            for recipe_id in &tool.compatible_recipes {
                if !context.contains_recipe(recipe_id) {
                    findings.push(SchedulerValidationFinding::error(format!(
                        "scheduler tool {} references missing compatible recipe {recipe_id}",
                        tool.id
                    )));
                }
                if context
                    .equipment_tool_has_recipe(&tool.id, recipe_id)
                    .is_some_and(|has_recipe| !has_recipe)
                {
                    findings.push(SchedulerValidationFinding::warning(format!(
                        "scheduler tool {} compatible recipe {recipe_id} is not available on equipment tool",
                        tool.id
                    )));
                }
            }
            if let Some(state) = context.equipment_tool_state(&tool.id) {
                let equipment_available = equipment_state_accepts_dispatch(state);
                if tool.state == ToolDispatchState::Available && !equipment_available {
                    findings.push(SchedulerValidationFinding::warning(format!(
                        "scheduler tool {} is marked available but equipment state is {}",
                        tool.id,
                        state.label()
                    )));
                }
                if tool.state != ToolDispatchState::Available && equipment_available {
                    findings.push(SchedulerValidationFinding::warning(format!(
                        "scheduler tool {} is marked {} but equipment state is {}",
                        tool.id,
                        tool.state.label(),
                        state.label()
                    )));
                }
            }
        }
        for assignment in &self.assignments {
            if let Some(state) = context.equipment_tool_state(&assignment.tool_id)
                && !equipment_state_accepts_dispatch(state)
            {
                findings.push(SchedulerValidationFinding::warning(format!(
                    "scheduler assignment for lot {} uses equipment tool {} while runtime state is {}",
                    assignment.lot_id,
                    assignment.tool_id,
                    state.label()
                )));
            }
            if let Some(active_lot_id) = context.equipment_active_lot(&assignment.tool_id)
                && active_lot_id != assignment.lot_id.as_str()
            {
                findings.push(SchedulerValidationFinding::warning(format!(
                    "scheduler assignment for lot {} uses equipment tool {} while runtime active lot is {active_lot_id}",
                    assignment.lot_id,
                    assignment.tool_id
                )));
            }
        }
    }

    pub fn dispatch(&self, policy: DispatchPolicy) -> DispatchResult {
        let mut tool_available = self
            .tools
            .iter()
            .map(|tool| (tool.id.clone(), self.now_minute))
            .collect::<BTreeMap<_, _>>();
        let mut lots = self.lots.clone();
        sort_lots(&mut lots, policy);

        let mut assignments = Vec::new();
        let mut unscheduled_lots = Vec::new();

        for lot in lots {
            let candidate = self
                .tools
                .iter()
                .filter(|tool| tool.can_process(&lot))
                .map(|tool| {
                    let ready = *tool_available.get(&tool.id).unwrap_or(&self.now_minute);
                    let start =
                        tool.next_start_after(ready.max(lot.ready_at_minute), lot.process_minutes);
                    (tool, start, start.saturating_add(lot.process_minutes))
                })
                .min_by(|left, right| {
                    left.1
                        .cmp(&right.1)
                        .then_with(|| left.2.cmp(&right.2))
                        .then_with(|| left.0.id.cmp(&right.0.id))
                });

            if let Some((tool, start, finish)) = candidate {
                tool_available.insert(tool.id.clone(), finish);
                assignments.push(DispatchAssignment {
                    lot_id: lot.id,
                    tool_id: tool.id.clone(),
                    start_minute: start,
                    finish_minute: finish,
                    due_at_minute: lot.due_at_minute,
                    priority: lot.priority,
                    wait_minutes: start.saturating_sub(lot.ready_at_minute),
                    tardy_minutes: finish.saturating_sub(lot.due_at_minute),
                });
            } else {
                unscheduled_lots.push(lot.id);
            }
        }

        let recommendations =
            self.recommendations_from_assignments(&assignments, &unscheduled_lots);
        let queue_summaries = self.queue_summaries();

        DispatchResult {
            assignments,
            recommendations,
            queue_summaries,
            unscheduled_lots,
        }
    }

    pub fn bottleneck_queue(&self) -> Option<QueueSummary> {
        self.queue_summaries().into_iter().max_by(|left, right| {
            left.total_process_minutes
                .cmp(&right.total_process_minutes)
                .then_with(|| left.waiting_lots.cmp(&right.waiting_lots))
        })
    }

    pub fn queue_summaries(&self) -> Vec<QueueSummary> {
        let mut summaries = BTreeMap::<ToolClass, QueueSummary>::new();
        for lot in &self.lots {
            let entry = summaries
                .entry(lot.required_tool_class)
                .or_insert(QueueSummary {
                    tool_class: lot.required_tool_class,
                    waiting_lots: 0,
                    total_process_minutes: 0,
                    earliest_due_minute: None,
                });
            entry.waiting_lots += 1;
            entry.total_process_minutes = entry
                .total_process_minutes
                .saturating_add(lot.process_minutes);
            entry.earliest_due_minute = Some(
                entry
                    .earliest_due_minute
                    .map_or(lot.due_at_minute, |due| due.min(lot.due_at_minute)),
            );
        }
        summaries.into_values().collect()
    }

    pub fn completion_for_lot(&self, lot_id: &LotId) -> Option<u32> {
        self.assignments
            .iter()
            .find(|assignment| assignment.lot_id == *lot_id)
            .map(|assignment| assignment.finish_minute)
    }

    pub fn utilization_percent(&self, tool_id: &ToolId) -> u32 {
        let total = self
            .assignments
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

    fn recommendations_from_assignments(
        &self,
        assignments: &[DispatchAssignment],
        unscheduled_lots: &[LotId],
    ) -> Vec<ToolRecommendation> {
        self.tools
            .iter()
            .map(|tool| {
                if tool.state != ToolDispatchState::Available {
                    return ToolRecommendation {
                        tool_id: tool.id.clone(),
                        lot_id: None,
                        reason: tool.state.label().to_string(),
                        start_minute: None,
                        finish_minute: None,
                    };
                }
                if let Some(assignment) = assignments
                    .iter()
                    .filter(|assignment| assignment.tool_id == tool.id)
                    .min_by_key(|assignment| assignment.start_minute)
                {
                    ToolRecommendation {
                        tool_id: tool.id.clone(),
                        lot_id: Some(assignment.lot_id.clone()),
                        reason: format!(
                            "priority {} lot, wait {} min",
                            assignment.priority, assignment.wait_minutes
                        ),
                        start_minute: Some(assignment.start_minute),
                        finish_minute: Some(assignment.finish_minute),
                    }
                } else if unscheduled_lots.is_empty() {
                    ToolRecommendation {
                        tool_id: tool.id.clone(),
                        lot_id: None,
                        reason: "no compatible waiting lots".to_string(),
                        start_minute: None,
                        finish_minute: None,
                    }
                } else {
                    ToolRecommendation {
                        tool_id: tool.id.clone(),
                        lot_id: None,
                        reason: "compatible queue empty".to_string(),
                        start_minute: None,
                        finish_minute: None,
                    }
                }
            })
            .collect()
    }
}

fn validate_maintenance_windows(
    tool: &DispatchTool,
    findings: &mut Vec<SchedulerValidationFinding>,
) {
    for window in &tool.maintenance_windows {
        if window.end_minute <= window.start_minute {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler tool {} has invalid maintenance window {}..{}",
                tool.id, window.start_minute, window.end_minute
            )));
        }
        if window.reason.trim().is_empty() {
            findings.push(SchedulerValidationFinding::warning(format!(
                "scheduler tool {} has maintenance window without a reason",
                tool.id
            )));
        }
    }
}

fn validate_dispatch_lot(lot: &DispatchLot, findings: &mut Vec<SchedulerValidationFinding>) {
    if lot.id.as_str().trim().is_empty() {
        findings.push(SchedulerValidationFinding::error(
            "scheduler lot id cannot be empty",
        ));
    }
    if lot.product.trim().is_empty() {
        findings.push(SchedulerValidationFinding::warning(format!(
            "scheduler lot {} has no product name",
            lot.id
        )));
    }
    if lot.step_id.as_str().trim().is_empty() {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler lot {} has no process step id",
            lot.id
        )));
    }
    if lot.step_name.trim().is_empty() {
        findings.push(SchedulerValidationFinding::warning(format!(
            "scheduler lot {} has no process step name",
            lot.id
        )));
    }
    if lot.recipe_id.as_str().trim().is_empty() {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler lot {} has no recipe id",
            lot.id
        )));
    }
    if lot.process_minutes == 0 {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler lot {} has zero process time",
            lot.id
        )));
    }
    if lot.wafer_count == 0 {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler lot {} has no wafers",
            lot.id
        )));
    }
    if lot.due_at_minute < lot.ready_at_minute {
        findings.push(SchedulerValidationFinding::warning(format!(
            "scheduler lot {} due time is before ready time",
            lot.id
        )));
    }
}

fn validate_assignment(
    assignment: &DispatchAssignment,
    lots_by_id: &BTreeMap<LotId, &DispatchLot>,
    tools_by_id: &BTreeMap<ToolId, &DispatchTool>,
    findings: &mut Vec<SchedulerValidationFinding>,
) {
    let lot = lots_by_id.get(&assignment.lot_id).copied();
    let tool = tools_by_id.get(&assignment.tool_id).copied();

    if lot.is_none() {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler assignment references missing lot {}",
            assignment.lot_id
        )));
    }
    if tool.is_none() {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler assignment references missing tool {}",
            assignment.tool_id
        )));
    }
    if assignment.finish_minute <= assignment.start_minute {
        findings.push(SchedulerValidationFinding::error(format!(
            "scheduler assignment for lot {} has invalid time window {}..{}",
            assignment.lot_id, assignment.start_minute, assignment.finish_minute
        )));
    }

    if let Some(lot) = lot {
        if assignment.start_minute < lot.ready_at_minute {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler assignment for lot {} starts before ready time {} < {}",
                assignment.lot_id, assignment.start_minute, lot.ready_at_minute
            )));
        }
        if assignment.due_at_minute != lot.due_at_minute {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler assignment for lot {} due time {} does not match lot due time {}",
                assignment.lot_id, assignment.due_at_minute, lot.due_at_minute
            )));
        }
        if assignment.priority != lot.priority {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler assignment for lot {} priority {} does not match lot priority {}",
                assignment.lot_id, assignment.priority, lot.priority
            )));
        }
        let expected_wait = assignment.start_minute.saturating_sub(lot.ready_at_minute);
        if assignment.wait_minutes != expected_wait {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler assignment for lot {} wait {} does not match derived wait {}",
                assignment.lot_id, assignment.wait_minutes, expected_wait
            )));
        }
        let expected_tardy = assignment.finish_minute.saturating_sub(lot.due_at_minute);
        if assignment.tardy_minutes != expected_tardy {
            findings.push(SchedulerValidationFinding::error(format!(
                "scheduler assignment for lot {} tardy {} does not match derived tardy {}",
                assignment.lot_id, assignment.tardy_minutes, expected_tardy
            )));
        }
        if assignment.finish_minute > assignment.start_minute {
            let duration = assignment
                .finish_minute
                .saturating_sub(assignment.start_minute);
            if duration != lot.process_minutes {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler assignment for lot {} duration {} does not match lot process time {}",
                    assignment.lot_id, duration, lot.process_minutes
                )));
            }
        }
        if let Some(tool) = tool {
            if !tool.can_process(lot) {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler assignment for lot {} uses incompatible tool {}",
                    assignment.lot_id, assignment.tool_id
                )));
            }
            for window in &tool.maintenance_windows {
                if window.overlaps(assignment.start_minute, assignment.finish_minute) {
                    findings.push(SchedulerValidationFinding::error(format!(
                        "scheduler assignment for lot {} overlaps tool {} maintenance window {}..{}",
                        assignment.lot_id,
                        assignment.tool_id,
                        window.start_minute,
                        window.end_minute
                    )));
                }
            }
        }
    }
}

fn validate_tool_assignment_overlaps(
    assignments_by_tool: BTreeMap<ToolId, Vec<&DispatchAssignment>>,
    findings: &mut Vec<SchedulerValidationFinding>,
) {
    for (tool_id, mut assignments) in assignments_by_tool {
        assignments.sort_by(|left, right| {
            left.start_minute
                .cmp(&right.start_minute)
                .then_with(|| left.finish_minute.cmp(&right.finish_minute))
                .then_with(|| left.lot_id.cmp(&right.lot_id))
        });
        for pair in assignments.windows(2) {
            let previous = pair[0];
            let next = pair[1];
            if next.start_minute < previous.finish_minute {
                findings.push(SchedulerValidationFinding::error(format!(
                    "scheduler tool {tool_id} has overlapping assignments for lots {} and {} ({}..{} overlaps {}..{})",
                    previous.lot_id,
                    next.lot_id,
                    previous.start_minute,
                    previous.finish_minute,
                    next.start_minute,
                    next.finish_minute
                )));
            }
        }
    }
}

fn equipment_state_accepts_dispatch(state: EquipmentToolState) -> bool {
    matches!(
        state,
        EquipmentToolState::OnlineIdle
            | EquipmentToolState::RecipeLoaded
            | EquipmentToolState::Completed
    )
}

pub fn format_shift_time(minute: u32) -> String {
    let hour = (minute / 60) % 24;
    let minute = minute % 60;
    format!("{hour:02}:{minute:02}")
}

fn sort_lots(lots: &mut [DispatchLot], policy: DispatchPolicy) {
    lots.sort_by(|left, right| {
        let ordering = match policy {
            DispatchPolicy::Fifo => left.fifo_sequence.cmp(&right.fifo_sequence),
            DispatchPolicy::PriorityThenFifo => right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.fifo_sequence.cmp(&right.fifo_sequence))
                .then_with(|| left.due_at_minute.cmp(&right.due_at_minute)),
            DispatchPolicy::DueDateThenPriority => left
                .due_at_minute
                .cmp(&right.due_at_minute)
                .then_with(|| right.priority.cmp(&left.priority))
                .then_with(|| left.fifo_sequence.cmp(&right.fifo_sequence)),
        };
        ordering.then_with(|| stable_lot_order(left, right))
    });
}

fn stable_lot_order(left: &DispatchLot, right: &DispatchLot) -> Ordering {
    left.id.cmp(&right.id)
}

fn sample_tools() -> Vec<DispatchTool> {
    vec![
        DispatchTool {
            id: ToolId::new("TRACK-01"),
            name: "Litho track 01".to_string(),
            class: ToolClass::LithographyTrack,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![
                RecipeId::new("SPIN_PR_3000"),
                RecipeId::new("LITHO_DEVELOP_001"),
            ],
            maintenance_windows: vec![MaintenanceWindow {
                start_minute: 11 * 60,
                end_minute: 11 * 60 + 45,
                reason: "coat cup clean".to_string(),
            }],
        },
        DispatchTool {
            id: ToolId::new("ALIGNER-01"),
            name: "Mask aligner 01".to_string(),
            class: ToolClass::MaskAligner,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![RecipeId::new("LITHO_POLY_EXPOSE_001")],
            maintenance_windows: Vec::new(),
        },
        DispatchTool {
            id: ToolId::new("ETCH-01"),
            name: "Etch chamber 01".to_string(),
            class: ToolClass::PlasmaEtcher,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![RecipeId::new("ETCH_CF4_POLY_001")],
            maintenance_windows: vec![MaintenanceWindow {
                start_minute: 10 * 60 + 20,
                end_minute: 10 * 60 + 55,
                reason: "seasoning run".to_string(),
            }],
        },
        DispatchTool {
            id: ToolId::new("CDSEM-01"),
            name: "CD-SEM 01".to_string(),
            class: ToolClass::CdMetrology,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![RecipeId::new("METRO_POLY_CD_001")],
            maintenance_windows: Vec::new(),
        },
    ]
}

fn sample_lots() -> Vec<DispatchLot> {
    vec![
        sample_lot(
            "L-00042",
            "demo inverter poly loop",
            5,
            1,
            8 * 60,
            13 * 60,
            ToolClass::MaskAligner,
            "LITHO_POLY_EXPOSE_001",
            58,
        ),
        sample_lot(
            "L-00042-ETCH",
            "demo inverter poly loop",
            3,
            2,
            8 * 60 + 10,
            12 * 60 + 30,
            ToolClass::PlasmaEtcher,
            "ETCH_CF4_POLY_001",
            74,
        ),
        sample_lot(
            "L-00043",
            "poly etch split",
            4,
            3,
            8 * 60 + 25,
            14 * 60,
            ToolClass::LithographyTrack,
            "SPIN_PR_3000",
            82,
        ),
        sample_lot(
            "L-00043-ETCH",
            "poly etch split",
            2,
            4,
            9 * 60,
            15 * 60,
            ToolClass::PlasmaEtcher,
            "ETCH_CF4_POLY_001",
            88,
        ),
        sample_lot(
            "L-00042-MET",
            "demo inverter poly loop",
            5,
            5,
            9 * 60 + 15,
            12 * 60,
            ToolClass::CdMetrology,
            "METRO_POLY_CD_001",
            34,
        ),
        sample_lot(
            "L-00044",
            "photoresist monitor",
            1,
            6,
            9 * 60 + 30,
            16 * 60,
            ToolClass::LithographyTrack,
            "LITHO_DEVELOP_001",
            96,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn sample_lot(
    id: &str,
    product: &str,
    priority: u8,
    fifo_sequence: u32,
    ready_at_minute: u32,
    due_at_minute: u32,
    required_tool_class: ToolClass,
    recipe: &str,
    process_minutes: u32,
) -> DispatchLot {
    DispatchLot {
        id: LotId::new(id),
        product: product.to_string(),
        priority,
        fifo_sequence,
        ready_at_minute,
        due_at_minute,
        step_id: ProcessStepId::new(sample_step_id_for_recipe(recipe, fifo_sequence)),
        step_name: required_tool_class.label().to_string(),
        required_tool_class,
        recipe_id: RecipeId::new(recipe),
        process_minutes,
        wafer_count: 25,
    }
}

fn sample_step_id_for_recipe(recipe: &str, fifo_sequence: u32) -> String {
    match recipe {
        "SPIN_PR_3000" => "S010-COAT".to_string(),
        "LITHO_POLY_EXPOSE_001" => "S020-EXPOSE".to_string(),
        "LITHO_DEVELOP_001" => "S030-DEVELOP".to_string(),
        "ETCH_CF4_POLY_001" => "S040-ETCH".to_string(),
        "METRO_POLY_CD_001" => "S050-CD-METRO".to_string(),
        _ => format!("STEP-{fifo_sequence:02}"),
    }
}

impl fmt::Display for DispatchPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation_context() -> SchedulerValidationContext {
        SchedulerValidationContext::from_recipe_ids_and_mes(
            crate::recipe::RecipeCatalog::sample()
                .recipes
                .keys()
                .map(|recipe_id| recipe_id.as_str().to_string()),
            &crate::mes::FabMesData::sample(),
        )
    }

    fn validation_context_with_equipment() -> SchedulerValidationContext {
        validation_context().with_equipment(&crate::equipment::EquipmentSimulator::demo_fab())
    }

    fn validation_context_with_running_equipment_lot(
        tool_id: &str,
        runtime_lot_id: &str,
    ) -> SchedulerValidationContext {
        let mut simulator = crate::equipment::EquipmentSimulator::demo_fab();
        let equipment_tool_id = crate::equipment::ToolId::new(tool_id);
        let mut selection = simulator.selection_for(&equipment_tool_id, "ETCH_OXIDE_DESCUM");
        selection.lot_id = Some(runtime_lot_id.to_string());
        let started_at_s = simulator.now_s.saturating_sub(60);
        if let Some(tool) = simulator.tool_mut(&equipment_tool_id) {
            tool.state = crate::equipment::ToolState::Running;
            tool.active_run = Some(crate::equipment::ToolRun {
                id: crate::equipment::RunId::new("RUN-SCHEDULER-CONTEXT"),
                tool_id: equipment_tool_id,
                recipe: selection,
                started_at_s,
                completed_at_s: None,
                status: crate::equipment::RunStatus::Running,
                sensor_count: 0,
            });
        }
        validation_context().with_equipment(&simulator)
    }

    fn has_validation_error(findings: &[SchedulerValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == SchedulerValidationSeverity::Error
                && finding.message.contains(needle)
        })
    }

    fn has_validation_warning(findings: &[SchedulerValidationFinding], needle: &str) -> bool {
        findings.iter().any(|finding| {
            finding.severity == SchedulerValidationSeverity::Warning
                && finding.message.contains(needle)
        })
    }

    #[test]
    fn sample_schedule_validates_internally_and_links_recipes() {
        let schedule = DispatchSchedule::sample();
        let findings = schedule.validate();

        assert!(findings.is_empty(), "{findings:?}");
        assert!(schedule.is_valid());

        let linked_findings = schedule.validate_with_context(&validation_context());
        assert!(
            !linked_findings
                .iter()
                .any(|finding| finding.severity == SchedulerValidationSeverity::Error),
            "{linked_findings:?}"
        );
        assert!(has_validation_warning(
            &linked_findings,
            "scheduler lot L-00043 is not present in MES lots"
        ));
    }

    #[test]
    fn scheduler_validation_rejects_invalid_tools_lots_and_assignments() {
        let mut schedule = DispatchSchedule::sample();
        schedule.tools.push(schedule.tools[0].clone());
        schedule.lots[0].process_minutes = 0;
        schedule.assignments.push(DispatchAssignment {
            lot_id: LotId::new("LOT-MISSING"),
            tool_id: ToolId::new("TOOL-MISSING"),
            start_minute: 30,
            finish_minute: 30,
            due_at_minute: 40,
            priority: 1,
            wait_minutes: 0,
            tardy_minutes: 0,
        });

        let findings = schedule.validate();

        assert!(has_validation_error(
            &findings,
            "scheduler contains duplicate tool TRACK-01"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00042 has zero process time"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment references missing lot LOT-MISSING"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment references missing tool TOOL-MISSING"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot LOT-MISSING has invalid time window 30..30"
        ));
    }

    #[test]
    fn scheduler_validation_rejects_stale_assignment_snapshots() {
        let mut schedule = DispatchSchedule::sample();
        schedule.assignments = vec![DispatchAssignment {
            lot_id: LotId::new("L-00042"),
            tool_id: ToolId::new("TRACK-01"),
            start_minute: 7 * 60,
            finish_minute: 7 * 60 + 10,
            due_at_minute: 999,
            priority: 0,
            wait_minutes: 123,
            tardy_minutes: 321,
        }];

        let findings = schedule.validate();

        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 starts before ready time 420 < 480"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 due time 999 does not match lot due time 780"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 priority 0 does not match lot priority 5"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 wait 123 does not match derived wait 0"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 tardy 321 does not match derived tardy 0"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 duration 10 does not match lot process time 58"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00042 uses incompatible tool TRACK-01"
        ));
    }

    #[test]
    fn scheduler_validation_rejects_assignment_inside_maintenance_window() {
        let mut schedule = DispatchSchedule::sample();
        schedule.assignments = vec![DispatchAssignment {
            lot_id: LotId::new("L-00043"),
            tool_id: ToolId::new("TRACK-01"),
            start_minute: 10 * 60 + 45,
            finish_minute: 10 * 60 + 45 + 82,
            due_at_minute: 14 * 60,
            priority: 4,
            wait_minutes: (10 * 60 + 45) - (8 * 60 + 25),
            tardy_minutes: 0,
        }];

        let findings = schedule.validate();

        assert!(has_validation_error(
            &findings,
            "scheduler assignment for lot L-00043 overlaps tool TRACK-01 maintenance window 660..705"
        ));
    }

    #[test]
    fn scheduler_validation_rejects_duplicate_lot_assignments_and_tool_overlaps() {
        let mut schedule = DispatchSchedule::sample();
        schedule.tools[0].maintenance_windows.clear();
        schedule.assignments = vec![
            DispatchAssignment {
                lot_id: LotId::new("L-00043"),
                tool_id: ToolId::new("TRACK-01"),
                start_minute: 8 * 60 + 40,
                finish_minute: 8 * 60 + 40 + 82,
                due_at_minute: 14 * 60,
                priority: 4,
                wait_minutes: 15,
                tardy_minutes: 0,
            },
            DispatchAssignment {
                lot_id: LotId::new("L-00043"),
                tool_id: ToolId::new("TRACK-01"),
                start_minute: 10 * 60 + 2,
                finish_minute: 10 * 60 + 2 + 82,
                due_at_minute: 14 * 60,
                priority: 4,
                wait_minutes: 97,
                tardy_minutes: 0,
            },
            DispatchAssignment {
                lot_id: LotId::new("L-00044"),
                tool_id: ToolId::new("TRACK-01"),
                start_minute: 9 * 60 + 30,
                finish_minute: 9 * 60 + 30 + 96,
                due_at_minute: 16 * 60,
                priority: 1,
                wait_minutes: 0,
                tardy_minutes: 0,
            },
        ];

        let findings = schedule.validate();

        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00043 has multiple persisted assignments"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler tool TRACK-01 has overlapping assignments for lots L-00043 and L-00044"
        ));
    }

    #[test]
    fn scheduler_validation_rejects_missing_recipe_references() {
        let mut schedule = DispatchSchedule::sample();
        schedule.lots[0].recipe_id = RecipeId::new("RECIPE-MISSING");
        schedule.tools[0]
            .compatible_recipes
            .push(RecipeId::new("TOOL-RECIPE-MISSING"));

        let findings = schedule.validate_with_context(&validation_context());

        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00042 references missing recipe RECIPE-MISSING"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler tool TRACK-01 references missing compatible recipe TOOL-RECIPE-MISSING"
        ));
    }

    #[test]
    fn scheduler_validation_checks_mes_route_step_context() {
        let mut schedule = DispatchSchedule::sample();
        schedule.lots[0].step_id = ProcessStepId::new("S999-MISSING");

        let findings = schedule.validate_with_context(&validation_context());

        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00042 references missing MES step S999-MISSING"
        ));

        schedule.lots[0].step_id = ProcessStepId::new("S020-EXPOSE");
        schedule.lots[0].recipe_id = RecipeId::new("SPIN_PR_3000");
        schedule.lots[0].required_tool_class = ToolClass::LithographyTrack;
        let findings = schedule.validate_with_context(&validation_context());

        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00042 recipe SPIN_PR_3000 does not match MES step S020-EXPOSE"
        ));
        assert!(has_validation_error(
            &findings,
            "scheduler lot L-00042 tool class lithography track does not match MES step S020-EXPOSE class mask aligner"
        ));
    }

    #[test]
    fn scheduler_validation_surfaces_equipment_runtime_mismatches() {
        let schedule = DispatchSchedule::sample();

        let findings = schedule.validate_with_context(&validation_context_with_equipment());

        assert!(has_validation_warning(
            &findings,
            "scheduler tool ETCH-01 compatible recipe ETCH_CF4_POLY_001 is not available on equipment tool"
        ));
        assert!(has_validation_warning(
            &findings,
            "scheduler tool ETCH-01 is marked available but equipment state is Alarm"
        ));
        assert!(has_validation_warning(
            &findings,
            "uses equipment tool ETCH-01 while runtime state is Alarm"
        ));
    }

    #[test]
    fn scheduler_validation_warns_when_runtime_active_lot_conflicts_with_assignment() {
        let schedule = DispatchSchedule::sample();

        let findings = schedule.validate_with_context(
            &validation_context_with_running_equipment_lot("ETCH-01", "L-RUNTIME"),
        );

        assert!(has_validation_warning(
            &findings,
            "scheduler assignment for lot L-00042-ETCH uses equipment tool ETCH-01 while runtime active lot is L-RUNTIME"
        ));
    }

    #[test]
    fn priority_dispatch_recommends_high_priority_lot_first() {
        let schedule = DispatchSchedule::sample();
        let result = schedule.dispatch(DispatchPolicy::PriorityThenFifo);
        let track = ToolId::new("TRACK-01");
        let recommendation = result
            .recommendations
            .iter()
            .find(|recommendation| recommendation.tool_id == track)
            .expect("track recommendation");

        assert_eq!(recommendation.lot_id, Some(LotId::new("L-00043")));
        assert!(result.unscheduled_lots.is_empty());
    }

    #[test]
    fn dispatch_respects_maintenance_windows() {
        let schedule = DispatchSchedule::sample();
        let result = schedule.dispatch(DispatchPolicy::PriorityThenFifo);
        let track_assignments = result
            .assignments
            .iter()
            .filter(|assignment| assignment.tool_id == ToolId::new("TRACK-01"))
            .collect::<Vec<_>>();

        assert!(
            track_assignments
                .iter()
                .all(|assignment| !((11 * 60)..(11 * 60 + 45)).contains(&assignment.start_minute))
        );
        assert!(
            track_assignments
                .iter()
                .all(|assignment| assignment.finish_minute <= 11 * 60
                    || assignment.start_minute >= 11 * 60 + 45)
        );
    }

    #[test]
    fn incompatible_lot_is_unscheduled() {
        let mut schedule = DispatchSchedule::sample();
        schedule.lots.push(sample_lot(
            "LOT-NOPE",
            "unknown",
            9,
            99,
            8 * 60,
            12 * 60,
            ToolClass::BakeOven,
            "BAKE-X",
            30,
        ));

        let result = schedule.dispatch(DispatchPolicy::PriorityThenFifo);

        assert!(result.unscheduled_lots.contains(&LotId::new("LOT-NOPE")));
    }
}
