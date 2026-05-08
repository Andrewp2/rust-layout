use serde::{Deserialize, Serialize};

use crate::equipment::ToolId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FabDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl FabDate {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    pub fn days_until(self, other: Self) -> i32 {
        other.serial_day() - self.serial_day()
    }

    fn serial_day(self) -> i32 {
        self.year * 372 + self.month as i32 * 31 + self.day as i32
    }
}

impl std::fmt::Display for FabDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaintenanceKind {
    PreventiveMaintenance,
    ChamberClean,
    Calibration,
    Qualification,
    Downtime,
    SparePart,
}

impl MaintenanceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::PreventiveMaintenance => "PM",
            Self::ChamberClean => "Chamber clean",
            Self::Calibration => "Calibration",
            Self::Qualification => "Qualification",
            Self::Downtime => "Downtime",
            Self::SparePart => "Spare part",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceSchedule {
    pub id: String,
    pub tool_id: ToolId,
    pub task: String,
    pub kind: MaintenanceKind,
    pub interval_days: Option<u32>,
    pub interval_runs: Option<u32>,
    pub last_completed: FabDate,
    pub last_completed_run_count: u32,
    pub next_due: FabDate,
    pub checklist: Vec<String>,
}

impl MaintenanceSchedule {
    pub fn due_state(&self, today: FabDate, current_run_count: u32) -> DueState {
        if self.next_due < today {
            return DueState::Overdue;
        }
        if self.next_due == today {
            return DueState::Due;
        }
        if let Some(interval_runs) = self.interval_runs {
            let run_delta = current_run_count.saturating_sub(self.last_completed_run_count);
            if run_delta >= interval_runs {
                return DueState::Due;
            }
        }
        DueState::NotDue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DueState {
    NotDue,
    Due,
    Overdue,
}

impl DueState {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotDue => "Not due",
            Self::Due => "Due",
            Self::Overdue => "Overdue",
        }
    }

    pub fn blocks_release(self) -> bool {
        matches!(self, Self::Due | Self::Overdue)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalibrationOutcome {
    Passed,
    Failed,
    Conditional,
}

impl CalibrationOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::Failed => "Failed",
            Self::Conditional => "Conditional",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationRecord {
    pub id: String,
    pub tool_id: ToolId,
    pub performed_at: FabDate,
    pub instrument: String,
    pub parameter: String,
    pub measured_value: f64,
    pub tolerance: String,
    pub outcome: CalibrationOutcome,
    pub technician: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualificationOutcome {
    Passed,
    Failed,
    PendingReview,
}

impl QualificationOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::Failed => "Failed",
            Self::PendingReview => "Pending review",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QualificationResult {
    pub id: String,
    pub tool_id: ToolId,
    pub wafer_id: String,
    pub performed_at: FabDate,
    pub recipe_id: String,
    pub metric: String,
    pub value: f64,
    pub spec: String,
    pub outcome: QualificationOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DowntimeRecord {
    pub id: String,
    pub tool_id: ToolId,
    pub started_at: FabDate,
    pub ended_at: Option<FabDate>,
    pub reason: String,
    pub owner: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SparePartUse {
    pub id: String,
    pub tool_id: ToolId,
    pub used_at: FabDate,
    pub part_number: String,
    pub description: String,
    pub quantity: u32,
    pub unit_cost: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolReleaseState {
    Released,
    DueSoon,
    MaintenanceRequired,
    CalibrationLockout,
    QualificationLockout,
    Down,
}

impl ToolReleaseState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Released => "Released",
            Self::DueSoon => "Due soon",
            Self::MaintenanceRequired => "Maintenance required",
            Self::CalibrationLockout => "Calibration lockout",
            Self::QualificationLockout => "Qualification lockout",
            Self::Down => "Down",
        }
    }

    pub fn released_to_production(self) -> bool {
        matches!(self, Self::Released | Self::DueSoon)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRelease {
    pub tool_id: ToolId,
    pub state: ToolReleaseState,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DueWork {
    pub tool_id: ToolId,
    pub schedule_id: String,
    pub task: String,
    pub kind: MaintenanceKind,
    pub due_state: DueState,
    pub due_date: FabDate,
    pub run_count_delta: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolMaintenanceState {
    pub tool_id: ToolId,
    pub tool_name: String,
    pub run_count: u32,
    pub schedules: Vec<MaintenanceSchedule>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceModel {
    pub today: Option<FabDate>,
    pub tools: Vec<ToolMaintenanceState>,
    pub calibration_records: Vec<CalibrationRecord>,
    pub qualification_results: Vec<QualificationResult>,
    pub downtime: Vec<DowntimeRecord>,
    pub spare_parts: Vec<SparePartUse>,
}

impl MaintenanceModel {
    pub fn sample() -> Self {
        sample_maintenance_model()
    }

    pub fn tool(&self, tool_id: &ToolId) -> Option<&ToolMaintenanceState> {
        self.tools.iter().find(|tool| &tool.tool_id == tool_id)
    }

    pub fn due_work(&self, today: FabDate) -> Vec<DueWork> {
        let mut work = Vec::new();
        for tool in &self.tools {
            for schedule in &tool.schedules {
                let due_state = schedule.due_state(today, tool.run_count);
                if due_state == DueState::NotDue {
                    continue;
                }
                work.push(DueWork {
                    tool_id: tool.tool_id.clone(),
                    schedule_id: schedule.id.clone(),
                    task: schedule.task.clone(),
                    kind: schedule.kind,
                    due_state,
                    due_date: schedule.next_due,
                    run_count_delta: schedule.interval_runs.map(|_| {
                        tool.run_count
                            .saturating_sub(schedule.last_completed_run_count)
                    }),
                });
            }
        }
        work.sort_by_key(|work| {
            (
                work.due_state != DueState::Overdue,
                work.due_date,
                work.tool_id.clone(),
            )
        });
        work
    }

    pub fn calibration_records_for(&self, tool_id: &ToolId) -> Vec<&CalibrationRecord> {
        self.calibration_records
            .iter()
            .filter(|record| &record.tool_id == tool_id)
            .collect()
    }

    pub fn qualification_results_for(&self, tool_id: &ToolId) -> Vec<&QualificationResult> {
        self.qualification_results
            .iter()
            .filter(|record| &record.tool_id == tool_id)
            .collect()
    }

    pub fn downtime_for(&self, tool_id: &ToolId) -> Vec<&DowntimeRecord> {
        self.downtime
            .iter()
            .filter(|record| &record.tool_id == tool_id)
            .collect()
    }

    pub fn spare_parts_for(&self, tool_id: &ToolId) -> Vec<&SparePartUse> {
        self.spare_parts
            .iter()
            .filter(|record| &record.tool_id == tool_id)
            .collect()
    }

    pub fn release_for_tool(&self, tool_id: &ToolId, today: FabDate) -> ToolRelease {
        let mut reasons = Vec::new();
        if self
            .downtime
            .iter()
            .any(|record| &record.tool_id == tool_id && record.ended_at.is_none())
        {
            reasons.push("open downtime event".to_string());
            return ToolRelease {
                tool_id: tool_id.clone(),
                state: ToolReleaseState::Down,
                reasons,
            };
        }

        let failed_calibration = self
            .calibration_records_for(tool_id)
            .into_iter()
            .max_by_key(|record| record.performed_at)
            .is_some_and(|record| record.outcome == CalibrationOutcome::Failed);
        if failed_calibration {
            reasons.push("latest calibration failed".to_string());
            return ToolRelease {
                tool_id: tool_id.clone(),
                state: ToolReleaseState::CalibrationLockout,
                reasons,
            };
        }

        let failed_qualification = self
            .qualification_results_for(tool_id)
            .into_iter()
            .max_by_key(|record| record.performed_at)
            .is_some_and(|record| record.outcome != QualificationOutcome::Passed);
        if failed_qualification {
            reasons.push("latest qualification wafer is not passing".to_string());
            return ToolRelease {
                tool_id: tool_id.clone(),
                state: ToolReleaseState::QualificationLockout,
                reasons,
            };
        }

        let Some(tool) = self.tool(tool_id) else {
            reasons.push("tool is not in maintenance model".to_string());
            return ToolRelease {
                tool_id: tool_id.clone(),
                state: ToolReleaseState::MaintenanceRequired,
                reasons,
            };
        };

        let mut due_soon = false;
        for schedule in &tool.schedules {
            match schedule.due_state(today, tool.run_count) {
                DueState::Overdue => {
                    reasons.push(format!("{} is overdue", schedule.task));
                    return ToolRelease {
                        tool_id: tool_id.clone(),
                        state: ToolReleaseState::MaintenanceRequired,
                        reasons,
                    };
                }
                DueState::Due => {
                    reasons.push(format!("{} is due", schedule.task));
                    return ToolRelease {
                        tool_id: tool_id.clone(),
                        state: ToolReleaseState::MaintenanceRequired,
                        reasons,
                    };
                }
                DueState::NotDue => {
                    if today.days_until(schedule.next_due) <= 7 {
                        due_soon = true;
                        reasons.push(format!("{} due {}", schedule.task, schedule.next_due));
                    }
                }
            }
        }

        ToolRelease {
            tool_id: tool_id.clone(),
            state: if due_soon {
                ToolReleaseState::DueSoon
            } else {
                ToolReleaseState::Released
            },
            reasons,
        }
    }
}

pub fn sample_maintenance_model() -> MaintenanceModel {
    let today = FabDate::new(2026, 5, 8);
    MaintenanceModel {
        today: Some(today),
        tools: vec![
            tool_state(
                "ETCH-01",
                "Reactive Ion Etcher 01",
                138,
                vec![
                    schedule(
                        "pm-etch-01",
                        "ETCH-01",
                        "Quarterly PM",
                        MaintenanceKind::PreventiveMaintenance,
                        "2026-02-20",
                        90,
                        None,
                        "2026-05-20",
                        96,
                        &[
                            "Inspect RF match",
                            "Leak check chamber",
                            "Verify endpoint optics",
                        ],
                    ),
                    schedule(
                        "clean-etch-01",
                        "ETCH-01",
                        "Chamber clean",
                        MaintenanceKind::ChamberClean,
                        "2026-05-03",
                        14,
                        Some(50),
                        "2026-05-17",
                        122,
                        &["Vent chamber", "Replace shield kit", "Particle check"],
                    ),
                    schedule(
                        "cal-etch-01",
                        "ETCH-01",
                        "RF power calibration",
                        MaintenanceKind::Calibration,
                        "2026-04-25",
                        30,
                        None,
                        "2026-05-25",
                        132,
                        &["Connect power meter", "Verify 50 W", "Verify 150 W"],
                    ),
                ],
            ),
            tool_state(
                "ALIGN-01",
                "Mask Aligner 01",
                91,
                vec![schedule(
                    "cal-align-01",
                    "ALIGN-01",
                    "Alignment calibration",
                    MaintenanceKind::Calibration,
                    "2026-04-18",
                    21,
                    None,
                    "2026-05-09",
                    88,
                    &["Clean stage", "Verify theta", "Expose qualification wafer"],
                )],
            ),
            tool_state(
                "MET-01",
                "Inspection Microscope 01",
                47,
                vec![schedule(
                    "pm-met-01",
                    "MET-01",
                    "Objective and stage PM",
                    MaintenanceKind::PreventiveMaintenance,
                    "2026-03-28",
                    30,
                    None,
                    "2026-04-27",
                    44,
                    &["Clean optics", "Verify stage repeatability"],
                )],
            ),
        ],
        calibration_records: vec![
            calibration(
                "cal-rec-1",
                "ETCH-01",
                "2026-04-25",
                "RF power meter PM-118",
                "RF power",
                149.4,
                "+/- 2.0 W",
                CalibrationOutcome::Passed,
            ),
            calibration(
                "cal-rec-2",
                "ALIGN-01",
                "2026-04-18",
                "Overlay grid OG-12",
                "Alignment error",
                0.18,
                "<= 0.25 um",
                CalibrationOutcome::Passed,
            ),
            calibration(
                "cal-rec-3",
                "MET-01",
                "2026-04-27",
                "Stage artifact ST-03",
                "Stage repeatability",
                0.42,
                "<= 0.30 um",
                CalibrationOutcome::Failed,
            ),
        ],
        qualification_results: vec![
            qualification(
                "qual-1",
                "ETCH-01",
                "QW-ETCH-0507",
                "2026-05-07",
                "ETCH_OXIDE_DESCUM",
                "etch rate sigma",
                2.1,
                "<= 3.0%",
                QualificationOutcome::Passed,
            ),
            qualification(
                "qual-2",
                "ALIGN-01",
                "QW-ALIGN-0506",
                "2026-05-06",
                "ALIGN_POLY_EXPOSE",
                "overlay P95",
                0.21,
                "<= 0.30 um",
                QualificationOutcome::Passed,
            ),
            qualification(
                "qual-3",
                "MET-01",
                "QW-MET-0427",
                "2026-04-27",
                "MICRO_CRITICAL_DIM",
                "CD bias",
                0.09,
                "<= 0.05 um",
                QualificationOutcome::Failed,
            ),
        ],
        downtime: vec![
            DowntimeRecord {
                id: "down-1".to_string(),
                tool_id: ToolId::from("MET-01"),
                started_at: FabDate::new(2026, 5, 7),
                ended_at: None,
                reason: "stage calibration failure".to_string(),
                owner: "maintenance.tech".to_string(),
            },
            DowntimeRecord {
                id: "down-2".to_string(),
                tool_id: ToolId::from("ETCH-01"),
                started_at: FabDate::new(2026, 5, 3),
                ended_at: Some(FabDate::new(2026, 5, 3)),
                reason: "scheduled chamber clean".to_string(),
                owner: "etch.owner".to_string(),
            },
        ],
        spare_parts: vec![
            SparePartUse {
                id: "part-1".to_string(),
                tool_id: ToolId::from("ETCH-01"),
                used_at: FabDate::new(2026, 5, 3),
                part_number: "SHIELD-ETCH-200".to_string(),
                description: "chamber shield kit".to_string(),
                quantity: 1,
                unit_cost: 820.0,
            },
            SparePartUse {
                id: "part-2".to_string(),
                tool_id: ToolId::from("MET-01"),
                used_at: FabDate::new(2026, 5, 7),
                part_number: "STAGE-BELT-04".to_string(),
                description: "stage drive belt".to_string(),
                quantity: 2,
                unit_cost: 64.0,
            },
        ],
    }
}

fn tool_state(
    id: &str,
    name: &str,
    run_count: u32,
    schedules: Vec<MaintenanceSchedule>,
) -> ToolMaintenanceState {
    ToolMaintenanceState {
        tool_id: ToolId::from(id),
        tool_name: name.to_string(),
        run_count,
        schedules,
    }
}

#[allow(clippy::too_many_arguments)]
fn schedule(
    id: &str,
    tool_id: &str,
    task: &str,
    kind: MaintenanceKind,
    last_completed: &str,
    interval_days: u32,
    interval_runs: Option<u32>,
    next_due: &str,
    last_completed_run_count: u32,
    checklist: &[&str],
) -> MaintenanceSchedule {
    MaintenanceSchedule {
        id: id.to_string(),
        tool_id: ToolId::from(tool_id),
        task: task.to_string(),
        kind,
        interval_days: Some(interval_days),
        interval_runs,
        last_completed: parse_sample_date(last_completed),
        last_completed_run_count,
        next_due: parse_sample_date(next_due),
        checklist: checklist.iter().map(|item| item.to_string()).collect(),
    }
}

#[allow(clippy::too_many_arguments)]
fn calibration(
    id: &str,
    tool_id: &str,
    performed_at: &str,
    instrument: &str,
    parameter: &str,
    measured_value: f64,
    tolerance: &str,
    outcome: CalibrationOutcome,
) -> CalibrationRecord {
    CalibrationRecord {
        id: id.to_string(),
        tool_id: ToolId::from(tool_id),
        performed_at: parse_sample_date(performed_at),
        instrument: instrument.to_string(),
        parameter: parameter.to_string(),
        measured_value,
        tolerance: tolerance.to_string(),
        outcome,
        technician: "maintenance.tech".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn qualification(
    id: &str,
    tool_id: &str,
    wafer_id: &str,
    performed_at: &str,
    recipe_id: &str,
    metric: &str,
    value: f64,
    spec: &str,
    outcome: QualificationOutcome,
) -> QualificationResult {
    QualificationResult {
        id: id.to_string(),
        tool_id: ToolId::from(tool_id),
        wafer_id: wafer_id.to_string(),
        performed_at: parse_sample_date(performed_at),
        recipe_id: recipe_id.to_string(),
        metric: metric.to_string(),
        value,
        spec: spec.to_string(),
        outcome,
    }
}

fn parse_sample_date(value: &str) -> FabDate {
    let mut parts = value.split('-');
    FabDate::new(
        parts
            .next()
            .and_then(|part| part.parse().ok())
            .unwrap_or(2026),
        parts.next().and_then(|part| part.parse().ok()).unwrap_or(1),
        parts.next().and_then(|part| part.parse().ok()).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_work_includes_calendar_and_run_count_triggers() {
        let mut model = MaintenanceModel::sample();
        let today = FabDate::new(2026, 5, 8);
        let etcher = model
            .tools
            .iter_mut()
            .find(|tool| tool.tool_id == ToolId::from("ETCH-01"))
            .unwrap();
        etcher.run_count = 180;

        let due = model.due_work(today);

        assert!(due.iter().any(|work| {
            work.tool_id == ToolId::from("ETCH-01")
                && work.task == "Chamber clean"
                && work.due_state == DueState::Due
        }));
        assert!(due.iter().any(|work| {
            work.tool_id == ToolId::from("MET-01") && work.due_state == DueState::Overdue
        }));
    }

    #[test]
    fn failed_calibration_locks_out_tool_release() {
        let model = MaintenanceModel::sample();

        let release = model.release_for_tool(&ToolId::from("MET-01"), FabDate::new(2026, 5, 8));

        assert_eq!(release.state, ToolReleaseState::Down);
        assert!(!release.state.released_to_production());
    }

    #[test]
    fn passing_qualification_and_not_due_releases_tool() {
        let model = MaintenanceModel::sample();

        let release = model.release_for_tool(&ToolId::from("ETCH-01"), FabDate::new(2026, 5, 8));

        assert_eq!(release.state, ToolReleaseState::Released);
        assert!(release.state.released_to_production());
    }

    #[test]
    fn failed_latest_qualification_blocks_release() {
        let mut model = MaintenanceModel::sample();
        model
            .downtime
            .retain(|record| record.tool_id != ToolId::from("MET-01"));
        model
            .calibration_records
            .retain(|record| record.tool_id != ToolId::from("MET-01"));

        let release = model.release_for_tool(&ToolId::from("MET-01"), FabDate::new(2026, 5, 8));

        assert_eq!(release.state, ToolReleaseState::QualificationLockout);
    }
}
