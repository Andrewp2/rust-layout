use std::{cmp::Ordering, collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::mes::{LotId, ProcessStepId, RecipeId, ToolClass, ToolId};

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
                || self
                    .compatible_recipes
                    .iter()
                    .any(|recipe| *recipe == lot.recipe_id))
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
                .filter_map(|tool| {
                    let ready = *tool_available.get(&tool.id).unwrap_or(&self.now_minute);
                    let start =
                        tool.next_start_after(ready.max(lot.ready_at_minute), lot.process_minutes);
                    Some((tool, start, start.saturating_add(lot.process_minutes)))
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
            compatible_recipes: vec![RecipeId::new("LITHO-A"), RecipeId::new("LITHO-B")],
            maintenance_windows: vec![MaintenanceWindow {
                start_minute: 11 * 60,
                end_minute: 11 * 60 + 45,
                reason: "coat cup clean".to_string(),
            }],
        },
        DispatchTool {
            id: ToolId::new("ALIGN-02"),
            name: "Mask aligner 02".to_string(),
            class: ToolClass::MaskAligner,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![RecipeId::new("ALIGN-248")],
            maintenance_windows: Vec::new(),
        },
        DispatchTool {
            id: ToolId::new("ETCH-03"),
            name: "Etch chamber 03".to_string(),
            class: ToolClass::PlasmaEtcher,
            state: ToolDispatchState::Available,
            compatible_recipes: vec![RecipeId::new("ETCH-OX"), RecipeId::new("ETCH-SI")],
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
            compatible_recipes: vec![RecipeId::new("CD-POLY"), RecipeId::new("CD-VIA")],
            maintenance_windows: Vec::new(),
        },
    ]
}

fn sample_lots() -> Vec<DispatchLot> {
    vec![
        sample_lot(
            "LOT-2201",
            "demo logic",
            5,
            1,
            8 * 60,
            13 * 60,
            ToolClass::LithographyTrack,
            "LITHO-A",
            82,
        ),
        sample_lot(
            "LOT-2202",
            "sensor",
            3,
            2,
            8 * 60 + 10,
            12 * 60 + 30,
            ToolClass::PlasmaEtcher,
            "ETCH-OX",
            74,
        ),
        sample_lot(
            "LOT-2203",
            "demo logic",
            4,
            3,
            8 * 60 + 25,
            14 * 60,
            ToolClass::MaskAligner,
            "ALIGN-248",
            58,
        ),
        sample_lot(
            "LOT-2204",
            "memory shuttle",
            2,
            4,
            9 * 60,
            15 * 60,
            ToolClass::LithographyTrack,
            "LITHO-B",
            96,
        ),
        sample_lot(
            "LOT-2205",
            "sensor",
            5,
            5,
            9 * 60 + 15,
            12 * 60,
            ToolClass::CdMetrology,
            "CD-POLY",
            34,
        ),
        sample_lot(
            "LOT-2206",
            "demo logic",
            1,
            6,
            9 * 60 + 30,
            16 * 60,
            ToolClass::PlasmaEtcher,
            "ETCH-SI",
            88,
        ),
    ]
}

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
        step_id: ProcessStepId::new(format!("STEP-{fifo_sequence:02}")),
        step_name: required_tool_class.label().to_string(),
        required_tool_class,
        recipe_id: RecipeId::new(recipe),
        process_minutes,
        wafer_count: 25,
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

        assert_eq!(recommendation.lot_id, Some(LotId::new("LOT-2201")));
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
