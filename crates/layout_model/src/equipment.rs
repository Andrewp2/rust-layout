use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{mes::FabMesData, recipe::RecipeCatalog};

const MAX_SENSOR_HISTORY: usize = 192;
const MAX_RUN_LOG: usize = 24;
const MAX_EVENT_LOG: usize = 96;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolId(pub String);

impl ToolId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ToolId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ToolId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub String);

impl RecipeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RecipeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for RecipeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RecipeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunId(pub String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    SpinCoater,
    HotPlate,
    MaskAligner,
    Etcher,
    Microscope,
    ProbeStation,
}

impl ToolKind {
    pub fn class(self) -> ToolClass {
        match self {
            Self::SpinCoater => ToolClass::Coat,
            Self::HotPlate => ToolClass::Bake,
            Self::MaskAligner => ToolClass::Lithography,
            Self::Etcher => ToolClass::Etch,
            Self::Microscope => ToolClass::Inspection,
            Self::ProbeStation => ToolClass::Probe,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SpinCoater => "Spin coater",
            Self::HotPlate => "Hot plate",
            Self::MaskAligner => "Mask aligner",
            Self::Etcher => "Etcher",
            Self::Microscope => "Microscope",
            Self::ProbeStation => "Probe station",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolClass {
    Coat,
    Bake,
    Lithography,
    Etch,
    Inspection,
    Probe,
}

impl ToolClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Coat => "Coat",
            Self::Bake => "Bake",
            Self::Lithography => "Lithography",
            Self::Etch => "Etch",
            Self::Inspection => "Inspection",
            Self::Probe => "Probe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolState {
    Offline,
    OnlineIdle,
    RecipeLoaded,
    Running,
    Completed,
    Alarm,
    Maintenance,
}

impl ToolState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Offline => "Offline",
            Self::OnlineIdle => "Online idle",
            Self::RecipeLoaded => "Recipe loaded",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Alarm => "Alarm",
            Self::Maintenance => "Maintenance",
        }
    }

    pub fn accepts_recipe_load(self) -> bool {
        matches!(
            self,
            Self::OnlineIdle | Self::RecipeLoaded | Self::Completed
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipeParameter {
    Number { value: f64, unit: String },
    Text { value: String },
    Bool { value: bool },
}

impl RecipeParameter {
    pub fn number(value: f64, unit: impl Into<String>) -> Self {
        Self::Number {
            value,
            unit: unit.into(),
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text {
            value: value.into(),
        }
    }

    pub fn bool(value: bool) -> Self {
        Self::Bool { value }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number { value, .. } => Some(*value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub tool_kind: ToolKind,
    pub version: u32,
    pub duration_s: u64,
    pub parameters: BTreeMap<String, RecipeParameter>,
}

impl Recipe {
    pub fn new(
        id: impl Into<RecipeId>,
        name: impl Into<String>,
        tool_kind: ToolKind,
        version: u32,
        duration_s: u64,
        parameters: BTreeMap<String, RecipeParameter>,
    ) -> Self {
        let version = if version == 0 {
            warn!("equipment recipe version was zero; using version 1");
            1
        } else {
            version
        };
        let duration_s = if duration_s == 0 {
            warn!("equipment recipe duration was zero; using 1 second");
            1
        } else {
            duration_s
        };
        Self {
            id: id.into(),
            name: name.into(),
            tool_kind,
            version,
            duration_s,
            parameters,
        }
    }

    pub fn number(&self, key: &str, default: f64) -> f64 {
        match self.parameters.get(key) {
            Some(parameter) => parameter.as_number().unwrap_or_else(|| {
                warn!(
                    recipe_id = %self.id,
                    parameter_key = key,
                    default,
                    "equipment recipe parameter is not numeric; using default"
                );
                default
            }),
            None => {
                warn!(
                    recipe_id = %self.id,
                    parameter_key = key,
                    default,
                    "equipment recipe parameter missing; using default"
                );
                default
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeSelection {
    pub recipe_id: RecipeId,
    pub recipe_version: u32,
    #[serde(default)]
    pub lot_id: Option<String>,
    #[serde(default)]
    pub wafer_id: Option<String>,
    #[serde(default)]
    pub process_step_id: Option<String>,
    #[serde(default)]
    pub operator: Option<String>,
}

impl RecipeSelection {
    pub fn new(recipe_id: impl Into<RecipeId>, recipe_version: u32) -> Self {
        let recipe_version = if recipe_version == 0 {
            warn!("equipment recipe selection version was zero; using version 1");
            1
        } else {
            recipe_version
        };
        Self {
            recipe_id: recipe_id.into(),
            recipe_version,
            lot_id: None,
            wafer_id: None,
            process_step_id: None,
            operator: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmSeverity {
    Advisory,
    Warning,
    Critical,
}

impl AlarmSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Advisory => "Advisory",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alarm {
    pub id: String,
    pub tool_id: ToolId,
    pub code: String,
    pub message: String,
    pub severity: AlarmSeverity,
    pub active: bool,
    pub occurred_at_s: u64,
    #[serde(default)]
    pub cleared_at_s: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorSample {
    pub tool_id: ToolId,
    pub at_s: u64,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Completed,
    Aborted,
    Alarmed,
}

impl RunStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Aborted => "Aborted",
            Self::Alarmed => "Alarmed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRun {
    pub id: RunId,
    pub tool_id: ToolId,
    pub recipe: RecipeSelection,
    pub started_at_s: u64,
    #[serde(default)]
    pub completed_at_s: Option<u64>,
    pub status: RunStatus,
    #[serde(default)]
    pub sensor_count: usize,
}

impl ToolRun {
    pub fn elapsed_s(&self, now_s: u64) -> u64 {
        let completed_at_s = self.completed_at_s.unwrap_or_else(|| {
            warn!(
                run_id = %self.id,
                now_s,
                "tool run has no completed timestamp; using current simulator time for elapsed duration"
            );
            now_s
        });
        completed_at_s.saturating_sub(self.started_at_s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolLogEntry {
    pub at_s: u64,
    pub tool_id: ToolId,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub id: ToolId,
    pub name: String,
    pub kind: ToolKind,
    pub class: ToolClass,
    pub state: ToolState,
    pub available_recipes: BTreeMap<RecipeId, Recipe>,
    #[serde(default)]
    pub selected_recipe: Option<RecipeSelection>,
    #[serde(default)]
    pub active_run: Option<ToolRun>,
    #[serde(default)]
    pub recent_runs: Vec<ToolRun>,
    #[serde(default)]
    pub active_alarms: Vec<Alarm>,
    #[serde(default)]
    pub recent_sensors: VecDeque<SensorSample>,
    #[serde(default)]
    pub event_log: Vec<ToolLogEntry>,
    pub last_updated_at_s: u64,
}

impl Tool {
    pub fn new(
        id: impl Into<ToolId>,
        name: impl Into<String>,
        kind: ToolKind,
        recipes: Vec<Recipe>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            class: kind.class(),
            state: ToolState::Offline,
            available_recipes: recipes
                .into_iter()
                .map(|recipe| (recipe.id.clone(), recipe))
                .collect(),
            selected_recipe: None,
            active_run: None,
            recent_runs: Vec::new(),
            active_alarms: Vec::new(),
            recent_sensors: VecDeque::new(),
            event_log: Vec::new(),
            last_updated_at_s: 0,
        }
    }

    pub fn recipe(&self, id: &RecipeId) -> Option<&Recipe> {
        self.available_recipes.get(id)
    }

    pub fn selected_recipe_details(&self) -> Option<&Recipe> {
        self.selected_recipe
            .as_ref()
            .and_then(|selection| self.recipe(&selection.recipe_id))
    }

    pub fn latest_sensor(&self, name: &str) -> Option<&SensorSample> {
        self.recent_sensors
            .iter()
            .rev()
            .find(|sample| sample.name == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostCommand {
    BringOnline,
    LoadRecipe {
        selection: RecipeSelection,
    },
    Start,
    Stop,
    ClearAlarm,
    EnterMaintenance,
    ExitMaintenance,
    TriggerAlarm {
        code: String,
        message: String,
        severity: AlarmSeverity,
    },
    Reset,
}

impl HostCommand {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BringOnline => "bring_online",
            Self::LoadRecipe { .. } => "load_recipe",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::ClearAlarm => "clear_alarm",
            Self::EnterMaintenance => "enter_maintenance",
            Self::ExitMaintenance => "exit_maintenance",
            Self::TriggerAlarm { .. } => "trigger_alarm",
            Self::Reset => "reset",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EquipmentEvent {
    StateChanged {
        tool_id: ToolId,
        from: ToolState,
        to: ToolState,
        at_s: u64,
    },
    RecipeLoaded {
        tool_id: ToolId,
        selection: RecipeSelection,
        at_s: u64,
    },
    RunStarted {
        run: ToolRun,
    },
    RunEnded {
        run: ToolRun,
    },
    AlarmRaised {
        alarm: Alarm,
    },
    AlarmCleared {
        alarm: Alarm,
    },
    SensorSample {
        sample: SensorSample,
    },
    Log {
        entry: ToolLogEntry,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolTransitionError {
    pub tool_id: ToolId,
    pub state: ToolState,
    pub command: String,
    pub message: String,
}

impl fmt::Display for ToolTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} rejected {} while {}: {}",
            self.tool_id,
            self.command,
            self.state.label(),
            self.message
        )
    }
}

impl Error for ToolTransitionError {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyntheticTool {
    pub tool: Tool,
    run_elapsed_s: u64,
    run_counter: u64,
    alarm_counter: u64,
    planned_alarm_at_s: Option<u64>,
}

impl SyntheticTool {
    pub fn new(
        id: impl Into<ToolId>,
        name: impl Into<String>,
        kind: ToolKind,
        recipes: Vec<Recipe>,
    ) -> Self {
        Self {
            tool: Tool::new(id, name, kind, recipes),
            run_elapsed_s: 0,
            run_counter: 0,
            alarm_counter: 0,
            planned_alarm_at_s: None,
        }
    }

    pub fn spin_coater(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::SpinCoater, spin_coater_recipes())
    }

    pub fn hot_plate(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::HotPlate, hot_plate_recipes())
    }

    pub fn mask_aligner(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::MaskAligner, mask_aligner_recipes())
    }

    pub fn etcher(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::Etcher, etcher_recipes()).with_planned_alarm(14)
    }

    pub fn microscope(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::Microscope, microscope_recipes())
    }

    pub fn probe_station(id: impl Into<ToolId>, name: impl Into<String>) -> Self {
        Self::new(id, name, ToolKind::ProbeStation, probe_station_recipes())
    }

    pub fn with_planned_alarm(mut self, elapsed_s: u64) -> Self {
        let elapsed_s = if elapsed_s == 0 {
            warn!("planned alarm elapsed time was zero; using 1 second");
            1
        } else {
            elapsed_s
        };
        self.planned_alarm_at_s = Some(elapsed_s);
        self
    }

    pub fn command(
        &mut self,
        command: HostCommand,
        at_s: u64,
    ) -> Result<Vec<EquipmentEvent>, ToolTransitionError> {
        let mut events = Vec::new();
        match command {
            HostCommand::BringOnline => match self.tool.state {
                ToolState::Offline => {
                    self.transition_to(ToolState::OnlineIdle, at_s, &mut events);
                    self.push_log(at_s, "host established online control", &mut events);
                }
                ToolState::OnlineIdle => {
                    self.push_log(at_s, "online handshake refreshed", &mut events);
                }
                _ => {
                    return Err(self.invalid(
                        "bring_online",
                        "tool must be offline or already online idle",
                    ));
                }
            },
            HostCommand::LoadRecipe { selection } => {
                if !self.tool.state.accepts_recipe_load() {
                    return Err(self.invalid(
                        "load_recipe",
                        "recipe load requires online idle, recipe loaded, or completed",
                    ));
                }
                let Some(recipe) = self.tool.recipe(&selection.recipe_id) else {
                    return Err(self.invalid(
                        "load_recipe",
                        format!("unknown recipe {}", selection.recipe_id),
                    ));
                };
                if recipe.tool_kind != self.tool.kind {
                    return Err(self.invalid("load_recipe", "recipe is for a different tool kind"));
                }
                self.tool.selected_recipe = Some(selection.clone());
                self.tool.active_run = None;
                self.run_elapsed_s = 0;
                self.transition_to(ToolState::RecipeLoaded, at_s, &mut events);
                events.push(EquipmentEvent::RecipeLoaded {
                    tool_id: self.tool.id.clone(),
                    selection: selection.clone(),
                    at_s,
                });
                self.push_log(
                    at_s,
                    &format!(
                        "loaded recipe {} v{}",
                        selection.recipe_id, selection.recipe_version
                    ),
                    &mut events,
                );
            }
            HostCommand::Start => {
                if self.tool.state != ToolState::RecipeLoaded {
                    return Err(self.invalid("start", "start requires a loaded recipe"));
                }
                let Some(selection) = self.tool.selected_recipe.clone() else {
                    return Err(self.invalid("start", "no selected recipe"));
                };
                let Some(recipe) = self.tool.recipe(&selection.recipe_id) else {
                    return Err(self.invalid("start", "selected recipe is unavailable"));
                };
                let recipe_name = recipe.name.clone();
                self.run_counter += 1;
                self.run_elapsed_s = 0;
                let run = ToolRun {
                    id: RunId::new(format!(
                        "{}-R{:04}",
                        self.tool.id.as_str().replace('-', ""),
                        self.run_counter
                    )),
                    tool_id: self.tool.id.clone(),
                    recipe: selection,
                    started_at_s: at_s,
                    completed_at_s: None,
                    status: RunStatus::Running,
                    sensor_count: 0,
                };
                self.tool.active_run = Some(run.clone());
                self.transition_to(ToolState::Running, at_s, &mut events);
                events.push(EquipmentEvent::RunStarted { run });
                self.push_log(
                    at_s,
                    &format!("remote start accepted for {recipe_name}"),
                    &mut events,
                );
            }
            HostCommand::Stop => {
                if self.tool.state != ToolState::Running {
                    return Err(self.invalid("stop", "stop requires a running tool"));
                }
                self.finish_active_run(RunStatus::Aborted, at_s, &mut events);
                self.transition_to(ToolState::OnlineIdle, at_s, &mut events);
                self.push_log(at_s, "remote stop aborted active run", &mut events);
            }
            HostCommand::ClearAlarm => {
                if self.tool.state != ToolState::Alarm {
                    return Err(self.invalid("clear_alarm", "tool is not alarmed"));
                }
                let cleared = self
                    .tool
                    .active_alarms
                    .iter_mut()
                    .filter_map(|alarm| {
                        if alarm.active {
                            alarm.active = false;
                            alarm.cleared_at_s = Some(at_s);
                            Some(alarm.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                for alarm in cleared {
                    events.push(EquipmentEvent::AlarmCleared { alarm });
                }
                self.transition_to(ToolState::OnlineIdle, at_s, &mut events);
                self.push_log(at_s, "active alarms cleared by host", &mut events);
            }
            HostCommand::EnterMaintenance => {
                if self.tool.state == ToolState::Running {
                    return Err(self.invalid(
                        "enter_maintenance",
                        "maintenance requires stopping the active run first",
                    ));
                }
                self.transition_to(ToolState::Maintenance, at_s, &mut events);
                self.push_log(at_s, "tool placed in maintenance", &mut events);
            }
            HostCommand::ExitMaintenance => {
                if self.tool.state != ToolState::Maintenance {
                    return Err(self.invalid("exit_maintenance", "tool is not in maintenance"));
                }
                self.tool.selected_recipe = None;
                self.tool.active_run = None;
                self.transition_to(ToolState::Offline, at_s, &mut events);
                self.push_log(at_s, "maintenance released tool to offline", &mut events);
            }
            HostCommand::TriggerAlarm {
                code,
                message,
                severity,
            } => {
                if matches!(self.tool.state, ToolState::Offline | ToolState::Maintenance) {
                    return Err(
                        self.invalid("trigger_alarm", "alarm injection requires an online tool")
                    );
                }
                self.raise_alarm(code, message, severity, at_s, &mut events);
            }
            HostCommand::Reset => {
                if self.tool.state == ToolState::Running {
                    return Err(self.invalid("reset", "cannot reset during a running process"));
                }
                self.tool.selected_recipe = None;
                self.tool.active_run = None;
                self.run_elapsed_s = 0;
                if self.tool.state != ToolState::Offline {
                    self.transition_to(ToolState::OnlineIdle, at_s, &mut events);
                }
                self.push_log(
                    at_s,
                    "host reset cleared recipe and run context",
                    &mut events,
                );
            }
        }
        self.tool.last_updated_at_s = at_s;
        Ok(events)
    }

    pub fn tick_one(&mut self, at_s: u64) -> Vec<EquipmentEvent> {
        self.tool.last_updated_at_s = at_s;
        let mut events = Vec::new();
        if matches!(self.tool.state, ToolState::Offline | ToolState::Maintenance) {
            return events;
        }

        if self.tool.state == ToolState::Running {
            self.run_elapsed_s += 1;
        }

        let samples = self.sensor_samples(at_s);
        if let Some(run) = self.tool.active_run.as_mut() {
            run.sensor_count += samples.len();
        }
        for sample in samples {
            self.record_sample(sample, &mut events);
        }

        if self.tool.state == ToolState::Running {
            if self.planned_alarm_at_s == Some(self.run_elapsed_s) {
                self.raise_alarm(
                    "AUTO-INTERLOCK".to_string(),
                    "deterministic interlock trip during simulated run".to_string(),
                    AlarmSeverity::Critical,
                    at_s,
                    &mut events,
                );
                return events;
            }
            let duration_s = self
                .tool
                .selected_recipe_details()
                .map(|recipe| recipe.duration_s)
                .unwrap_or_else(|| {
                    warn!(
                        tool_id = %self.tool.id,
                        "running tool has no selected recipe details; using 1 second duration"
                    );
                    1
                });
            if self.run_elapsed_s >= duration_s {
                self.finish_active_run(RunStatus::Completed, at_s, &mut events);
                self.transition_to(ToolState::Completed, at_s, &mut events);
                self.push_log(at_s, "run completed", &mut events);
            }
        }

        events
    }

    fn invalid(
        &self,
        command: impl Into<String>,
        message: impl Into<String>,
    ) -> ToolTransitionError {
        ToolTransitionError {
            tool_id: self.tool.id.clone(),
            state: self.tool.state,
            command: command.into(),
            message: message.into(),
        }
    }

    fn transition_to(&mut self, state: ToolState, at_s: u64, events: &mut Vec<EquipmentEvent>) {
        let from = self.tool.state;
        if from == state {
            return;
        }
        self.tool.state = state;
        self.tool.last_updated_at_s = at_s;
        events.push(EquipmentEvent::StateChanged {
            tool_id: self.tool.id.clone(),
            from,
            to: state,
            at_s,
        });
    }

    fn push_log(&mut self, at_s: u64, message: &str, events: &mut Vec<EquipmentEvent>) {
        let entry = ToolLogEntry {
            at_s,
            tool_id: self.tool.id.clone(),
            message: message.to_string(),
        };
        self.tool.event_log.push(entry.clone());
        if self.tool.event_log.len() > MAX_EVENT_LOG {
            let overflow = self.tool.event_log.len() - MAX_EVENT_LOG;
            warn!(
                tool_id = %self.tool.id,
                dropped_events = overflow,
                max_event_log = MAX_EVENT_LOG,
                "tool event log exceeded retention limit; dropping oldest events"
            );
            self.tool.event_log.drain(0..overflow);
        }
        events.push(EquipmentEvent::Log { entry });
    }

    fn record_sample(&mut self, sample: SensorSample, events: &mut Vec<EquipmentEvent>) {
        self.tool.recent_sensors.push_back(sample.clone());
        while self.tool.recent_sensors.len() > MAX_SENSOR_HISTORY {
            warn!(
                tool_id = %self.tool.id,
                max_sensor_history = MAX_SENSOR_HISTORY,
                "tool sensor history exceeded retention limit; dropping oldest sample"
            );
            self.tool.recent_sensors.pop_front();
        }
        events.push(EquipmentEvent::SensorSample { sample });
    }

    fn finish_active_run(
        &mut self,
        status: RunStatus,
        at_s: u64,
        events: &mut Vec<EquipmentEvent>,
    ) {
        let Some(mut run) = self.tool.active_run.take() else {
            return;
        };
        run.completed_at_s = Some(at_s);
        run.status = status;
        self.tool.recent_runs.insert(0, run.clone());
        if self.tool.recent_runs.len() > MAX_RUN_LOG {
            warn!(
                tool_id = %self.tool.id,
                run_count = self.tool.recent_runs.len(),
                max_run_log = MAX_RUN_LOG,
                "tool run log exceeded retention limit; dropping oldest runs"
            );
        }
        self.tool.recent_runs.truncate(MAX_RUN_LOG);
        events.push(EquipmentEvent::RunEnded { run });
    }

    fn raise_alarm(
        &mut self,
        code: String,
        message: String,
        severity: AlarmSeverity,
        at_s: u64,
        events: &mut Vec<EquipmentEvent>,
    ) {
        if self.tool.state == ToolState::Running {
            self.finish_active_run(RunStatus::Alarmed, at_s, events);
        }
        self.alarm_counter += 1;
        let alarm = Alarm {
            id: format!(
                "{}-A{:03}",
                self.tool.id.as_str().replace('-', ""),
                self.alarm_counter
            ),
            tool_id: self.tool.id.clone(),
            code,
            message,
            severity,
            active: true,
            occurred_at_s: at_s,
            cleared_at_s: None,
        };
        self.tool.active_alarms.push(alarm.clone());
        self.transition_to(ToolState::Alarm, at_s, events);
        events.push(EquipmentEvent::AlarmRaised {
            alarm: alarm.clone(),
        });
        self.push_log(
            at_s,
            &format!("alarm {} raised: {}", alarm.code, alarm.message),
            events,
        );
    }

    fn sensor_samples(&self, at_s: u64) -> Vec<SensorSample> {
        let recipe = self.tool.selected_recipe_details();
        let running = self.tool.state == ToolState::Running;
        let elapsed = self.run_elapsed_s as f64;
        let duration = recipe
            .map(|recipe| recipe.duration_s)
            .unwrap_or_else(|| {
                warn!(
                    tool_id = %self.tool.id,
                    "sensor model missing selected recipe details; using 1 second duration"
                );
                1
            })
            .max(1) as f64;
        let progress = if running {
            let raw_progress = elapsed / duration;
            let progress = raw_progress.clamp(0.0, 1.0);
            if progress != raw_progress {
                warn!(
                    tool_id = %self.tool.id,
                    raw_progress,
                    progress,
                    "tool run progress outside [0, 1]; clamping"
                );
            }
            progress
        } else {
            0.0
        };
        match self.tool.kind {
            ToolKind::SpinCoater => {
                let target_rpm = recipe
                    .map(|recipe| recipe.number("rpm", 4000.0))
                    .unwrap_or_else(|| {
                        warn!(
                            tool_id = %self.tool.id,
                            "spin coater has no recipe while sampling sensors; using target rpm 0"
                        );
                        0.0
                    });
                let rpm = if running {
                    let raw_spinup = progress * 1.4;
                    let spinup = raw_spinup.min(1.0);
                    if spinup != raw_spinup {
                        warn!(
                            tool_id = %self.tool.id,
                            raw_spinup,
                            spinup,
                            "spin coater spinup factor exceeded range; clamping"
                        );
                    }
                    target_rpm * spinup + wave(at_s, 1) * 18.0
                } else {
                    0.0
                };
                let clamped_rpm = rpm.max(0.0);
                if clamped_rpm != rpm {
                    warn!(
                        tool_id = %self.tool.id,
                        rpm,
                        clamped_rpm,
                        "spin coater rpm below zero; clamping"
                    );
                }
                vec![
                    self.sample(at_s, "chuck_rpm", clamped_rpm, "rpm"),
                    self.sample(at_s, "exhaust_pressure", -115.0 + wave(at_s, 2) * 2.5, "Pa"),
                ]
            }
            ToolKind::HotPlate => {
                let target_c = recipe
                    .map(|recipe| recipe.number("temperature_c", 95.0))
                    .unwrap_or_else(|| {
                        warn!(
                            tool_id = %self.tool.id,
                            "hot plate has no recipe while sampling sensors; using ambient target temperature"
                        );
                        24.0
                    });
                let temp = if running {
                    24.0 + (target_c - 24.0) * (0.25 + progress * 0.75) + wave(at_s, 3) * 0.4
                } else if self.tool.state == ToolState::RecipeLoaded {
                    38.0 + wave(at_s, 4) * 0.3
                } else {
                    24.0 + wave(at_s, 5) * 0.2
                };
                vec![
                    self.sample(at_s, "surface_temp", temp, "C"),
                    self.sample(at_s, "zone_delta", 0.7 + wave(at_s, 6) * 0.2, "C"),
                ]
            }
            ToolKind::MaskAligner => {
                let error = if running {
                    let raw_decay = 1.0 - progress;
                    let decay = raw_decay.max(0.08);
                    if decay != raw_decay {
                        warn!(
                            tool_id = %self.tool.id,
                            raw_decay,
                            decay,
                            "mask aligner alignment decay below floor; clamping"
                        );
                    }
                    2.6 * decay + wave(at_s, 7) * 0.03
                } else {
                    2.8 + wave(at_s, 8) * 0.08
                };
                let clamped_error = error.max(0.0);
                if clamped_error != error {
                    warn!(
                        tool_id = %self.tool.id,
                        alignment_error = error,
                        clamped_error,
                        "mask aligner alignment error below zero; clamping"
                    );
                }
                vec![
                    self.sample(at_s, "alignment_error", clamped_error, "um"),
                    self.sample(
                        at_s,
                        "lamp_power",
                        if running { 13.5 } else { 0.0 },
                        "mW/cm2",
                    ),
                ]
            }
            ToolKind::Etcher => {
                let pressure = recipe
                    .map(|recipe| recipe.number("pressure_mtorr", 80.0))
                    .unwrap_or_else(|| {
                        warn!(
                            tool_id = %self.tool.id,
                            "etcher has no recipe while sampling sensors; using idle pressure"
                        );
                        4.0
                    });
                let rf = recipe
                    .map(|recipe| recipe.number("rf_power_w", 150.0))
                    .unwrap_or_else(|| {
                        warn!(
                            tool_id = %self.tool.id,
                            "etcher has no recipe while sampling sensors; using RF power 0"
                        );
                        0.0
                    });
                vec![
                    self.sample(
                        at_s,
                        "chamber_pressure",
                        if running {
                            pressure + wave(at_s, 9) * 1.5
                        } else {
                            4.0 + wave(at_s, 10) * 0.4
                        },
                        "mTorr",
                    ),
                    self.sample(
                        at_s,
                        "rf_power",
                        if running {
                            rf + wave(at_s, 11) * 2.5
                        } else {
                            0.0
                        },
                        "W",
                    ),
                    self.sample(at_s, "endpoint_signal", 18.0 + progress * 64.0, "%"),
                ]
            }
            ToolKind::Microscope => vec![
                self.sample(at_s, "focus_z", 52.0 + wave(at_s, 12) * 0.15, "um"),
                self.sample(at_s, "illumination", if running { 72.0 } else { 44.0 }, "%"),
            ],
            ToolKind::ProbeStation => vec![
                self.sample(
                    at_s,
                    "contact_resistance",
                    if running {
                        0.42 + wave(at_s, 13) * 0.03
                    } else {
                        1.6 + wave(at_s, 14) * 0.1
                    },
                    "ohm",
                ),
                self.sample(at_s, "chuck_temp", 25.0 + wave(at_s, 15) * 0.2, "C"),
            ],
        }
    }

    fn sample(&self, at_s: u64, name: &str, value: f64, unit: &str) -> SensorSample {
        SensorSample {
            tool_id: self.tool.id.clone(),
            at_s,
            name: name.to_string(),
            value,
            unit: unit.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquipmentSimulator {
    tools: BTreeMap<ToolId, SyntheticTool>,
    pub now_s: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquipmentValidationFinding {
    pub severity: EquipmentValidationSeverity,
    pub message: String,
}

impl EquipmentValidationFinding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: EquipmentValidationSeverity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: EquipmentValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipmentValidationContext {
    lot_routes: BTreeMap<String, String>,
    lot_wafers: BTreeMap<String, BTreeSet<String>>,
    route_steps: BTreeMap<String, BTreeSet<String>>,
    route_step_recipes: BTreeMap<(String, String), String>,
    recipe_versions: BTreeMap<String, BTreeSet<u32>>,
}

impl EquipmentValidationContext {
    pub fn from_mes_and_recipe_catalog(mes: &FabMesData, recipes: &RecipeCatalog) -> Self {
        let mut route_steps = BTreeMap::<String, BTreeSet<String>>::new();
        let mut route_step_recipes = BTreeMap::new();
        for route in mes.routes.values() {
            let route_id = route.id.as_str().to_string();
            let steps = route_steps.entry(route_id.clone()).or_default();
            for step in &route.steps {
                let step_id = step.id.as_str().to_string();
                steps.insert(step_id.clone());
                route_step_recipes.insert(
                    (route_id.clone(), step_id),
                    step.required_recipe.as_str().to_string(),
                );
            }
        }

        Self {
            lot_routes: mes
                .lots
                .values()
                .map(|lot| {
                    (
                        lot.id.as_str().to_string(),
                        lot.route_id.as_str().to_string(),
                    )
                })
                .collect(),
            lot_wafers: mes
                .lots
                .values()
                .map(|lot| {
                    (
                        lot.id.as_str().to_string(),
                        lot.wafers
                            .iter()
                            .map(|wafer| wafer.id.as_str().to_string())
                            .collect(),
                    )
                })
                .collect(),
            route_steps,
            route_step_recipes,
            recipe_versions: recipes
                .recipes
                .values()
                .map(|recipe| {
                    (
                        recipe.id.as_str().to_string(),
                        recipe
                            .versions
                            .iter()
                            .map(|version| version.version.0)
                            .collect(),
                    )
                })
                .collect(),
        }
    }

    fn contains_lot(&self, lot_id: &str) -> bool {
        self.lot_routes.contains_key(lot_id)
    }

    fn contains_lot_wafer(&self, lot_id: &str, wafer_id: &str) -> bool {
        self.lot_wafers
            .get(lot_id)
            .is_some_and(|wafers| wafers.contains(wafer_id))
    }

    fn lot_route(&self, lot_id: &str) -> Option<&str> {
        self.lot_routes.get(lot_id).map(String::as_str)
    }

    fn contains_step_on_route(&self, route_id: &str, step_id: &str) -> bool {
        self.route_steps
            .get(route_id)
            .is_some_and(|steps| steps.contains(step_id))
    }

    fn contains_step(&self, step_id: &str) -> bool {
        self.route_steps
            .values()
            .any(|steps| steps.contains(step_id))
    }

    fn step_required_recipe(&self, route_id: &str, step_id: &str) -> Option<&str> {
        self.route_step_recipes
            .get(&(route_id.to_string(), step_id.to_string()))
            .map(String::as_str)
    }

    fn contains_recipe(&self, recipe_id: &str) -> bool {
        self.recipe_versions.contains_key(recipe_id)
    }

    fn contains_recipe_version(&self, recipe_id: &str, version: u32) -> bool {
        self.recipe_versions
            .get(recipe_id)
            .is_some_and(|versions| versions.contains(&version))
    }
}

impl EquipmentSimulator {
    pub fn new(tools: Vec<SyntheticTool>) -> Self {
        Self {
            tools: tools
                .into_iter()
                .map(|tool| (tool.tool.id.clone(), tool))
                .collect(),
            now_s: 0,
        }
    }

    pub fn demo_fab() -> Self {
        let mut simulator = Self::new(vec![
            SyntheticTool::spin_coater("COAT-01", "Spin Coater 01"),
            SyntheticTool::hot_plate("BAKE-01", "Hot Plate 01"),
            SyntheticTool::mask_aligner("ALIGN-01", "Mask Aligner 01"),
            SyntheticTool::etcher("ETCH-01", "Reactive Ion Etcher 01"),
            SyntheticTool::microscope("MET-01", "Inspection Microscope 01"),
            SyntheticTool::probe_station("PROBE-01", "Probe Station 01"),
        ]);

        let coat = ToolId::new("COAT-01");
        let bake = ToolId::new("BAKE-01");
        let align = ToolId::new("ALIGN-01");
        let etch = ToolId::new("ETCH-01");
        let microscope = ToolId::new("MET-01");

        log_demo_command(
            simulator.command(&coat, HostCommand::BringOnline),
            "bring coat tool online",
        );
        log_demo_command(
            simulator.command(
                &coat,
                HostCommand::LoadRecipe {
                    selection: simulator.selection_for(&coat, "SPIN_PR_3000"),
                },
            ),
            "load coat recipe",
        );
        log_demo_command(
            simulator.command(&coat, HostCommand::Start),
            "start coat tool",
        );
        log_demo_command(
            simulator.command(&bake, HostCommand::BringOnline),
            "bring bake tool online",
        );
        log_demo_command(
            simulator.command(
                &bake,
                HostCommand::LoadRecipe {
                    selection: simulator.selection_for(&bake, "BAKE_SOFT_095C"),
                },
            ),
            "load bake recipe",
        );
        log_demo_command(
            simulator.command(&align, HostCommand::BringOnline),
            "bring aligner online",
        );
        log_demo_command(
            simulator.command(&etch, HostCommand::BringOnline),
            "bring etch tool online",
        );
        log_demo_command(
            simulator.command(
                &etch,
                HostCommand::TriggerAlarm {
                    code: "VAC-LOW".to_string(),
                    message: "foreline pressure below simulated threshold".to_string(),
                    severity: AlarmSeverity::Warning,
                },
            ),
            "trigger demo etch alarm",
        );
        log_demo_command(
            simulator.command(&microscope, HostCommand::BringOnline),
            "bring microscope online",
        );
        simulator.tick(5);
        simulator
    }

    pub fn tools(&self) -> impl Iterator<Item = &Tool> {
        self.tools.values().map(|tool| &tool.tool)
    }

    pub fn tool(&self, id: &ToolId) -> Option<&Tool> {
        self.tools.get(id).map(|tool| &tool.tool)
    }

    pub fn tool_mut(&mut self, id: &ToolId) -> Option<&mut Tool> {
        self.tools.get_mut(id).map(|tool| &mut tool.tool)
    }

    pub fn command(
        &mut self,
        id: &ToolId,
        command: HostCommand,
    ) -> Result<Vec<EquipmentEvent>, ToolTransitionError> {
        let Some(tool) = self.tools.get_mut(id) else {
            return Err(ToolTransitionError {
                tool_id: id.clone(),
                state: ToolState::Offline,
                command: command.label().to_string(),
                message: "unknown tool id".to_string(),
            });
        };
        tool.command(command, self.now_s)
    }

    pub fn tick(&mut self, seconds: u64) -> Vec<EquipmentEvent> {
        let mut events = Vec::new();
        for _ in 0..seconds {
            self.now_s += 1;
            for tool in self.tools.values_mut() {
                events.extend(tool.tick_one(self.now_s));
            }
        }
        events
    }

    pub fn active_alarms(&self) -> Vec<&Alarm> {
        self.tools()
            .flat_map(|tool| tool.active_alarms.iter())
            .filter(|alarm| alarm.active)
            .collect()
    }

    pub fn recent_runs(&self) -> Vec<&ToolRun> {
        let mut runs = self
            .tools()
            .flat_map(|tool| tool.recent_runs.iter())
            .collect::<Vec<_>>();
        runs.sort_by(|left, right| right.started_at_s.cmp(&left.started_at_s));
        runs
    }

    pub fn selection_for(&self, id: &ToolId, recipe: impl Into<RecipeId>) -> RecipeSelection {
        let recipe_id = recipe.into();
        let version = self
            .tool(id)
            .and_then(|tool| tool.recipe(&recipe_id))
            .map(|recipe| recipe.version)
            .unwrap_or_else(|| {
                warn!(
                    tool_id = %id,
                    recipe_id = %recipe_id,
                    "equipment recipe missing for selection; using version 1"
                );
                1
            });
        let step = self
            .tool(id)
            .and_then(|tool| process_step_for_kind(tool.kind).map(str::to_string));
        RecipeSelection {
            recipe_id,
            recipe_version: version,
            lot_id: Some("L-00042".to_string()),
            wafer_id: Some(format!("L-00042-W{:02}", (self.now_s % 25) + 1)),
            process_step_id: step,
            operator: Some("sim-host".to_string()),
        }
    }

    pub fn validate(&self) -> Vec<EquipmentValidationFinding> {
        let mut findings = Vec::new();
        for (tool_id, synthetic) in &self.tools {
            let tool = &synthetic.tool;
            if tool_id != &tool.id {
                findings.push(EquipmentValidationFinding::error(format!(
                    "equipment tool map key {tool_id} does not match tool id {}",
                    tool.id
                )));
            }
            validate_tool(tool, self.now_s, &mut findings);
        }
        findings
    }

    pub fn validate_with_context(
        &self,
        context: &EquipmentValidationContext,
    ) -> Vec<EquipmentValidationFinding> {
        let mut findings = self.validate();
        for tool in self.tools() {
            validate_tool_context(tool, context, &mut findings);
        }
        findings
    }
}

fn validate_tool(tool: &Tool, now_s: u64, findings: &mut Vec<EquipmentValidationFinding>) {
    if tool.id.as_str().trim().is_empty() {
        findings.push(EquipmentValidationFinding::error(
            "equipment tool id is empty",
        ));
    }
    if tool.name.trim().is_empty() {
        findings.push(EquipmentValidationFinding::warning(format!(
            "equipment tool {} has an empty name",
            tool.id
        )));
    }
    if tool.class != tool.kind.class() {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} class {} does not match kind {}",
            tool.id,
            tool.class.label(),
            tool.kind.label()
        )));
    }

    for (recipe_id, recipe) in &tool.available_recipes {
        validate_recipe_key(tool, recipe_id, recipe, findings);
    }
    if let Some(selection) = tool.selected_recipe.as_ref() {
        validate_recipe_selection(tool, selection, findings);
    }

    match tool.state {
        ToolState::Running => match tool.active_run.as_ref() {
            Some(run) if run.status == RunStatus::Running => {}
            Some(run) => findings.push(EquipmentValidationFinding::error(format!(
                "equipment tool {} is running but active run {} has status {}",
                tool.id,
                run.id,
                run.status.label()
            ))),
            None => findings.push(EquipmentValidationFinding::error(format!(
                "equipment tool {} is running without an active run",
                tool.id
            ))),
        },
        ToolState::Alarm => {
            if !tool.active_alarms.iter().any(|alarm| alarm.active) {
                findings.push(EquipmentValidationFinding::warning(format!(
                    "equipment tool {} is alarmed without an active alarm",
                    tool.id
                )));
            }
        }
        ToolState::Offline | ToolState::Maintenance => {
            if tool.active_run.is_some() {
                findings.push(EquipmentValidationFinding::error(format!(
                    "equipment tool {} is {} but still has an active run",
                    tool.id,
                    tool.state.label()
                )));
            }
        }
        ToolState::OnlineIdle | ToolState::RecipeLoaded | ToolState::Completed => {}
    }

    let mut run_ids = BTreeSet::new();
    if let Some(run) = tool.active_run.as_ref() {
        validate_run(tool, run, true, now_s, findings);
        if !run.id.as_str().trim().is_empty() {
            run_ids.insert(run.id.clone());
        }
    }
    for run in &tool.recent_runs {
        if !run.id.as_str().trim().is_empty() && !run_ids.insert(run.id.clone()) {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment tool {} has duplicate run {}",
                tool.id, run.id
            )));
        }
        validate_run(tool, run, false, now_s, findings);
    }

    let mut alarm_ids = BTreeSet::new();
    for alarm in &tool.active_alarms {
        if alarm.id.trim().is_empty() {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment tool {} has an alarm with empty id",
                tool.id
            )));
        } else if !alarm_ids.insert(alarm.id.clone()) {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment tool {} has duplicate alarm {}",
                tool.id, alarm.id
            )));
        }
        if alarm.tool_id != tool.id {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment alarm {} belongs to {} but is stored on {}",
                alarm.id, alarm.tool_id, tool.id
            )));
        }
        if !alarm.active && alarm.cleared_at_s.is_none() {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment alarm {} is inactive without a clear timestamp",
                alarm.id
            )));
        }
        if alarm
            .cleared_at_s
            .is_some_and(|cleared| cleared < alarm.occurred_at_s)
        {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment alarm {} clears before it occurs",
                alarm.id
            )));
        }
        if alarm.occurred_at_s > now_s || alarm.cleared_at_s.is_some_and(|cleared| cleared > now_s)
        {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment alarm {} is timestamped after simulator time {}",
                alarm.id, now_s
            )));
        }
    }

    for sample in &tool.recent_sensors {
        if sample.tool_id != tool.id {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment sensor sample {} belongs to {} but is stored on {}",
                sample.name, sample.tool_id, tool.id
            )));
        }
        if sample.name.trim().is_empty() || sample.unit.trim().is_empty() {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment sensor sample on {} has incomplete metadata",
                tool.id
            )));
        }
        if !sample.value.is_finite() {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment sensor sample {} on {} has non-finite value",
                sample.name, tool.id
            )));
        }
        if sample.at_s > now_s {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment sensor sample {} on {} is timestamped after simulator time {}",
                sample.name, tool.id, now_s
            )));
        }
    }

    for entry in &tool.event_log {
        if entry.tool_id != tool.id {
            findings.push(EquipmentValidationFinding::error(format!(
                "equipment log entry on {} belongs to {}",
                tool.id, entry.tool_id
            )));
        }
        if entry.message.trim().is_empty() {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment log entry on {} has an empty message",
                tool.id
            )));
        }
        if entry.at_s > now_s {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment log entry on {} is timestamped after simulator time {}",
                tool.id, now_s
            )));
        }
    }
}

fn validate_recipe_key(
    tool: &Tool,
    recipe_id: &RecipeId,
    recipe: &Recipe,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    if recipe_id != &recipe.id {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} recipe map key {recipe_id} does not match recipe id {}",
            tool.id, recipe.id
        )));
    }
    if recipe.id.as_str().trim().is_empty() {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} has recipe with empty id",
            tool.id
        )));
    }
    if recipe.tool_kind != tool.kind {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment recipe {} is for {} but is available on {}",
            recipe.id,
            recipe.tool_kind.label(),
            tool.kind.label()
        )));
    }
    if recipe.name.trim().is_empty() {
        findings.push(EquipmentValidationFinding::warning(format!(
            "equipment recipe {} has an empty name",
            recipe.id
        )));
    }
    if recipe.version == 0 || recipe.duration_s == 0 {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment recipe {} has zero version or duration",
            recipe.id
        )));
    }
    for (key, parameter) in &recipe.parameters {
        if key.trim().is_empty() {
            findings.push(EquipmentValidationFinding::warning(format!(
                "equipment recipe {} has an empty parameter key",
                recipe.id
            )));
        }
        if let RecipeParameter::Number { value, unit } = parameter {
            if !value.is_finite() {
                findings.push(EquipmentValidationFinding::error(format!(
                    "equipment recipe {} parameter {key} has non-finite value",
                    recipe.id
                )));
            }
            if unit.trim().is_empty() {
                findings.push(EquipmentValidationFinding::warning(format!(
                    "equipment recipe {} parameter {key} has an empty unit",
                    recipe.id
                )));
            }
        }
    }
}

fn validate_recipe_selection(
    tool: &Tool,
    selection: &RecipeSelection,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    match tool.available_recipes.get(&selection.recipe_id) {
        Some(recipe) if recipe.version == selection.recipe_version => {}
        Some(recipe) => findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} selected recipe {} version {} but available version is {}",
            tool.id, selection.recipe_id, selection.recipe_version, recipe.version
        ))),
        None => findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} selected missing recipe {}",
            tool.id, selection.recipe_id
        ))),
    }
}

fn validate_run(
    tool: &Tool,
    run: &ToolRun,
    active: bool,
    now_s: u64,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    if run.id.as_str().trim().is_empty() {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment tool {} has run with empty id",
            tool.id
        )));
    }
    if run.tool_id != tool.id {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment run {} belongs to {} but is stored on {}",
            run.id, run.tool_id, tool.id
        )));
    }
    validate_recipe_selection(tool, &run.recipe, findings);
    match run.status {
        RunStatus::Running => {
            if !active {
                findings.push(EquipmentValidationFinding::warning(format!(
                    "equipment completed run log contains running run {}",
                    run.id
                )));
            }
            if run.completed_at_s.is_some() {
                findings.push(EquipmentValidationFinding::error(format!(
                    "equipment running run {} has a completion timestamp",
                    run.id
                )));
            }
        }
        RunStatus::Completed | RunStatus::Aborted | RunStatus::Alarmed => {
            if run.completed_at_s.is_none() {
                findings.push(EquipmentValidationFinding::error(format!(
                    "equipment finished run {} has no completion timestamp",
                    run.id
                )));
            }
        }
    }
    if run
        .completed_at_s
        .is_some_and(|completed| completed < run.started_at_s)
    {
        findings.push(EquipmentValidationFinding::error(format!(
            "equipment run {} completes before it starts",
            run.id
        )));
    }
    if run.started_at_s > now_s
        || run
            .completed_at_s
            .is_some_and(|completed| completed > now_s)
    {
        findings.push(EquipmentValidationFinding::warning(format!(
            "equipment run {} is timestamped after simulator time {}",
            run.id, now_s
        )));
    }
}

fn validate_tool_context(
    tool: &Tool,
    context: &EquipmentValidationContext,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    if let Some(selection) = tool.selected_recipe.as_ref() {
        validate_recipe_selection_context(
            &format!("equipment tool {} selected recipe", tool.id),
            selection,
            context,
            findings,
        );
    }
    if let Some(run) = tool.active_run.as_ref() {
        validate_recipe_selection_context(
            &format!("equipment active run {}", run.id),
            &run.recipe,
            context,
            findings,
        );
    }
    for run in &tool.recent_runs {
        validate_recipe_selection_context(
            &format!("equipment recent run {}", run.id),
            &run.recipe,
            context,
            findings,
        );
    }
}

fn validate_recipe_selection_context(
    context_label: &str,
    selection: &RecipeSelection,
    context: &EquipmentValidationContext,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    let lot_id = selection
        .lot_id
        .as_deref()
        .map(str::trim)
        .filter(|lot_id| !lot_id.is_empty());
    let wafer_id = selection
        .wafer_id
        .as_deref()
        .map(str::trim)
        .filter(|wafer_id| !wafer_id.is_empty());
    let step_id = selection
        .process_step_id
        .as_deref()
        .map(str::trim)
        .filter(|step_id| !step_id.is_empty());
    let recipe_id = selection.recipe_id.as_str();

    if let Some(lot_id) = lot_id {
        if !context.contains_lot(lot_id) {
            findings.push(EquipmentValidationFinding::error(format!(
                "{context_label} references missing MES lot {lot_id}"
            )));
        }
    } else if wafer_id.is_some() || step_id.is_some() {
        findings.push(EquipmentValidationFinding::warning(format!(
            "{context_label} has MES wafer or step metadata without a lot id"
        )));
    }

    if let Some(wafer_id) = wafer_id {
        match lot_id {
            Some(lot_id) if context.contains_lot(lot_id) => {
                if !context.contains_lot_wafer(lot_id, wafer_id) {
                    findings.push(EquipmentValidationFinding::error(format!(
                        "{context_label} references missing MES wafer {wafer_id} on lot {lot_id}"
                    )));
                }
            }
            Some(_) | None => {}
        }
    }

    if let Some(step_id) = step_id {
        match lot_id.and_then(|lot_id| context.lot_route(lot_id)) {
            Some(route_id) if !context.contains_step_on_route(route_id, step_id) => {
                findings.push(EquipmentValidationFinding::error(format!(
                    "{context_label} references missing MES step {step_id} on route {route_id}"
                )));
                validate_selection_recipe_context(
                    context_label,
                    recipe_id,
                    selection.recipe_version,
                    None,
                    context,
                    findings,
                );
            }
            Some(route_id) => {
                validate_selection_recipe_context(
                    context_label,
                    recipe_id,
                    selection.recipe_version,
                    Some((route_id, step_id)),
                    context,
                    findings,
                );
            }
            None if !context.contains_step(step_id) => {
                findings.push(EquipmentValidationFinding::error(format!(
                    "{context_label} references missing MES step {step_id}"
                )));
                validate_selection_recipe_context(
                    context_label,
                    recipe_id,
                    selection.recipe_version,
                    None,
                    context,
                    findings,
                );
            }
            None => validate_selection_recipe_context(
                context_label,
                recipe_id,
                selection.recipe_version,
                None,
                context,
                findings,
            ),
        }
    } else {
        validate_selection_recipe_context(
            context_label,
            recipe_id,
            selection.recipe_version,
            None,
            context,
            findings,
        );
    }
}

fn validate_selection_recipe_context(
    context_label: &str,
    recipe_id: &str,
    version: u32,
    route_step: Option<(&str, &str)>,
    context: &EquipmentValidationContext,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
    if recipe_id.trim().is_empty() {
        return;
    }

    if context.contains_recipe(recipe_id) {
        if !context.contains_recipe_version(recipe_id, version) {
            findings.push(EquipmentValidationFinding::error(format!(
                "{context_label} references missing recipe catalog binding {recipe_id} v{version}"
            )));
        }
        if let Some((route_id, step_id)) = route_step
            && let Some(required_recipe) = context.step_required_recipe(route_id, step_id)
            && required_recipe != recipe_id
        {
            findings.push(EquipmentValidationFinding::error(format!(
                "{context_label} recipe {recipe_id} does not match MES step {step_id} required recipe {required_recipe}"
            )));
        }
    } else {
        findings.push(EquipmentValidationFinding::warning(format!(
            "{context_label} uses equipment-local recipe program {recipe_id}; no recipe catalog binding was found"
        )));
    }
}

fn log_demo_command(
    result: Result<Vec<EquipmentEvent>, ToolTransitionError>,
    description: &'static str,
) {
    if let Err(err) = result {
        warn!(error = %err, "demo equipment command failed: {description}");
    }
}

fn process_step_for_kind(kind: ToolKind) -> Option<&'static str> {
    match kind {
        ToolKind::SpinCoater => Some("S010-COAT"),
        ToolKind::HotPlate => None,
        ToolKind::MaskAligner => Some("S020-EXPOSE"),
        ToolKind::Etcher => Some("S040-ETCH"),
        ToolKind::Microscope => Some("S050-CD-METRO"),
        ToolKind::ProbeStation => None,
    }
}

fn wave(at_s: u64, seed: u64) -> f64 {
    let phase = ((at_s + seed * 11) % 31) as f64 / 31.0;
    (phase * std::f64::consts::TAU).sin()
}

fn spin_coater_recipes() -> Vec<Recipe> {
    vec![
        recipe(
            "COAT_PR_4000",
            "Positive PR 4000 rpm",
            ToolKind::SpinCoater,
            2,
            22,
            [
                ("rpm", RecipeParameter::number(4000.0, "rpm")),
                ("duration_s", RecipeParameter::number(45.0, "s")),
                ("acceleration", RecipeParameter::number(1000.0, "rpm/s")),
            ],
        ),
        recipe(
            "SPIN_PR_3000",
            "Positive PR 3000 rpm",
            ToolKind::SpinCoater,
            1,
            18,
            [
                ("rpm", RecipeParameter::number(3000.0, "rpm")),
                ("duration_s", RecipeParameter::number(35.0, "s")),
                ("acceleration", RecipeParameter::number(850.0, "rpm/s")),
            ],
        ),
    ]
}

fn hot_plate_recipes() -> Vec<Recipe> {
    vec![
        recipe(
            "BAKE_SOFT_095C",
            "Soft bake 95 C",
            ToolKind::HotPlate,
            3,
            26,
            [
                ("temperature_c", RecipeParameter::number(95.0, "C")),
                ("duration_s", RecipeParameter::number(60.0, "s")),
                ("ramp_c_per_s", RecipeParameter::number(2.5, "C/s")),
            ],
        ),
        recipe(
            "BAKE_POST_115C",
            "Post exposure bake 115 C",
            ToolKind::HotPlate,
            1,
            30,
            [
                ("temperature_c", RecipeParameter::number(115.0, "C")),
                ("duration_s", RecipeParameter::number(90.0, "s")),
                ("ramp_c_per_s", RecipeParameter::number(2.0, "C/s")),
            ],
        ),
    ]
}

fn mask_aligner_recipes() -> Vec<Recipe> {
    vec![recipe(
        "ALIGN_POLY_EXPOSE",
        "Poly gate expose",
        ToolKind::MaskAligner,
        1,
        24,
        [
            ("dose_mj_cm2", RecipeParameter::number(125.0, "mJ/cm2")),
            ("contact_gap_um", RecipeParameter::number(8.0, "um")),
            ("mask_id", RecipeParameter::text("RETICLE-POLY-A")),
        ],
    )]
}

fn etcher_recipes() -> Vec<Recipe> {
    vec![recipe(
        "ETCH_OXIDE_DESCUM",
        "Oxide descum",
        ToolKind::Etcher,
        2,
        34,
        [
            ("pressure_mtorr", RecipeParameter::number(80.0, "mTorr")),
            ("rf_power_w", RecipeParameter::number(150.0, "W")),
            ("gas", RecipeParameter::text("CF4/O2")),
        ],
    )]
}

fn microscope_recipes() -> Vec<Recipe> {
    vec![recipe(
        "MICRO_CRITICAL_DIM",
        "Critical dimension inspection",
        ToolKind::Microscope,
        1,
        16,
        [
            ("objective", RecipeParameter::text("50x")),
            ("illumination_pct", RecipeParameter::number(72.0, "%")),
            ("capture_images", RecipeParameter::bool(true)),
        ],
    )]
}

fn probe_station_recipes() -> Vec<Recipe> {
    vec![recipe(
        "PROBE_IV_SWEEP",
        "Transistor IV sweep",
        ToolKind::ProbeStation,
        1,
        28,
        [
            ("vds_max", RecipeParameter::number(1.8, "V")),
            ("vgs_step", RecipeParameter::number(0.1, "V")),
            ("compliance_ma", RecipeParameter::number(2.0, "mA")),
        ],
    )]
}

fn recipe<const N: usize>(
    id: &str,
    name: &str,
    kind: ToolKind,
    version: u32,
    duration_s: u64,
    parameters: [(&str, RecipeParameter); N],
) -> Recipe {
    Recipe::new(
        id,
        name,
        kind,
        version,
        duration_s,
        parameters
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_selection(tool: &SyntheticTool) -> RecipeSelection {
        let recipe = tool.tool.available_recipes.values().next().unwrap();
        RecipeSelection::new(recipe.id.clone(), recipe.version)
    }

    #[test]
    fn offline_to_completed_flow() {
        let mut tool = SyntheticTool::spin_coater("COAT-T", "Test coater");
        assert_eq!(tool.tool.state, ToolState::Offline);

        tool.command(HostCommand::BringOnline, 1).unwrap();
        assert_eq!(tool.tool.state, ToolState::OnlineIdle);

        let selection = first_selection(&tool);
        tool.command(HostCommand::LoadRecipe { selection }, 2)
            .unwrap();
        assert_eq!(tool.tool.state, ToolState::RecipeLoaded);

        tool.command(HostCommand::Start, 3).unwrap();
        assert_eq!(tool.tool.state, ToolState::Running);

        for second in 4..40 {
            tool.tick_one(second);
            if tool.tool.state == ToolState::Completed {
                break;
            }
        }

        assert_eq!(tool.tool.state, ToolState::Completed);
        assert_eq!(tool.tool.recent_runs.len(), 1);
        assert_eq!(tool.tool.recent_runs[0].status, RunStatus::Completed);
        assert!(tool.tool.recent_runs[0].sensor_count > 0);
    }

    #[test]
    fn running_tool_can_enter_alarm_and_be_cleared() {
        let mut tool = SyntheticTool::hot_plate("BAKE-T", "Test bake");
        tool.command(HostCommand::BringOnline, 1).unwrap();
        let selection = first_selection(&tool);
        tool.command(HostCommand::LoadRecipe { selection }, 2)
            .unwrap();
        tool.command(HostCommand::Start, 3).unwrap();

        tool.command(
            HostCommand::TriggerAlarm {
                code: "TEMP-HIGH".to_string(),
                message: "plate exceeded simulated limit".to_string(),
                severity: AlarmSeverity::Critical,
            },
            4,
        )
        .unwrap();
        assert_eq!(tool.tool.state, ToolState::Alarm);
        assert_eq!(tool.tool.recent_runs[0].status, RunStatus::Alarmed);
        assert!(tool.tool.active_alarms.iter().any(|alarm| alarm.active));

        tool.command(HostCommand::ClearAlarm, 5).unwrap();
        assert_eq!(tool.tool.state, ToolState::OnlineIdle);
        assert!(tool.tool.active_alarms.iter().all(|alarm| !alarm.active));
    }

    #[test]
    fn maintenance_path_rejects_running_tools() {
        let mut tool = SyntheticTool::mask_aligner("ALIGN-T", "Test aligner");
        tool.command(HostCommand::BringOnline, 1).unwrap();
        tool.command(HostCommand::EnterMaintenance, 2).unwrap();
        assert_eq!(tool.tool.state, ToolState::Maintenance);

        tool.command(HostCommand::ExitMaintenance, 3).unwrap();
        assert_eq!(tool.tool.state, ToolState::Offline);

        tool.command(HostCommand::BringOnline, 4).unwrap();
        let selection = first_selection(&tool);
        tool.command(HostCommand::LoadRecipe { selection }, 5)
            .unwrap();
        tool.command(HostCommand::Start, 6).unwrap();
        let err = tool.command(HostCommand::EnterMaintenance, 7).unwrap_err();
        assert_eq!(err.state, ToolState::Running);
        assert_eq!(tool.tool.state, ToolState::Running);
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        let mut tool = SyntheticTool::probe_station("PROBE-T", "Test probe");
        assert!(tool.command(HostCommand::Start, 1).is_err());
        assert!(
            tool.command(
                HostCommand::LoadRecipe {
                    selection: RecipeSelection::new("PROBE_IV_SWEEP", 1),
                },
                2,
            )
            .is_err()
        );

        tool.command(HostCommand::BringOnline, 3).unwrap();
        assert!(tool.command(HostCommand::Start, 4).is_err());
        assert!(
            tool.command(
                HostCommand::LoadRecipe {
                    selection: RecipeSelection::new("NO_SUCH_RECIPE", 1),
                },
                5,
            )
            .is_err()
        );
    }

    #[test]
    fn demo_equipment_simulator_validates() {
        let simulator = EquipmentSimulator::demo_fab();

        assert_eq!(simulator.validate(), Vec::new());
    }

    fn workspace_context() -> EquipmentValidationContext {
        EquipmentValidationContext::from_mes_and_recipe_catalog(
            &crate::mes::FabMesData::sample(),
            &crate::recipe::RecipeCatalog::sample(),
        )
    }

    #[test]
    fn demo_equipment_simulator_validates_against_workspace_context() {
        let simulator = EquipmentSimulator::demo_fab();
        let findings = simulator.validate_with_context(&workspace_context());

        assert!(
            findings
                .iter()
                .all(|finding| finding.severity != EquipmentValidationSeverity::Error),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("equipment-local recipe program BAKE_SOFT_095C")
            }),
            "{findings:?}"
        );
    }

    #[test]
    fn validation_context_rejects_missing_mes_and_recipe_catalog_references() {
        let mut simulator = EquipmentSimulator::demo_fab();
        let coat = simulator.tool_mut(&ToolId::new("COAT-01")).unwrap();
        let run = coat.active_run.as_mut().unwrap();
        run.recipe.lot_id = Some("L-00042".to_string());
        run.recipe.wafer_id = Some("L-00042-W99".to_string());
        run.recipe.process_step_id = Some("S999-MISSING".to_string());
        run.recipe.recipe_id = RecipeId::new("SPIN_PR_3000");
        run.recipe.recipe_version = 99;

        let findings = simulator.validate_with_context(&workspace_context());

        assert!(
            findings
                .iter()
                .any(|finding| { finding.message.contains("missing MES wafer L-00042-W99") }),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("missing MES step S999-MISSING")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("missing recipe catalog binding SPIN_PR_3000 v99")
            }),
            "{findings:?}"
        );
    }

    #[test]
    fn validation_rejects_broken_tool_recipe_run_alarm_and_sensor_state() {
        let mut simulator = EquipmentSimulator::demo_fab();
        let future_s = simulator.now_s + 100;
        let coat = simulator.tools.get_mut(&ToolId::new("COAT-01")).unwrap();
        coat.tool.class = ToolClass::Etch;
        coat.tool.selected_recipe = Some(RecipeSelection::new("MISSING_RECIPE", 1));
        let duplicate_active_run = {
            let run = coat.tool.active_run.as_mut().unwrap();
            run.completed_at_s = Some(0);
            run.recipe.recipe_id = RecipeId::new("MISSING_RECIPE");
            run.clone()
        };
        coat.tool.recent_runs.push(duplicate_active_run);
        coat.tool.active_alarms.push(Alarm {
            id: "bad-alarm".to_string(),
            tool_id: ToolId::new("OTHER"),
            code: "BAD".to_string(),
            message: "bad alarm".to_string(),
            severity: AlarmSeverity::Warning,
            active: false,
            occurred_at_s: future_s,
            cleared_at_s: Some(future_s - 1),
        });
        coat.tool.recent_sensors.push_back(SensorSample {
            tool_id: ToolId::new("COAT-01"),
            at_s: future_s,
            name: "bad".to_string(),
            value: f64::NAN,
            unit: "rpm".to_string(),
        });
        coat.tool.event_log.push(ToolLogEntry {
            at_s: future_s,
            tool_id: ToolId::new("COAT-01"),
            message: "future log".to_string(),
        });

        let findings = simulator.validate();

        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("class Etch does not match kind")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("selected missing recipe")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("running run")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("duplicate run")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("clears before it occurs")),
            "{findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("non-finite value")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("alarm bad-alarm is timestamped after")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| {
                finding
                    .message
                    .contains("sensor sample bad on COAT-01 is timestamped after")
            }),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding
                .message
                .contains("log entry on COAT-01 is timestamped after")),
            "{findings:?}"
        );
    }
}
