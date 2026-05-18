#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControlLoopId(pub String);

impl ControlLoopId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ControlLoopId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ControlLoopId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControlActionId(pub String);

impl ControlActionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ControlActionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for ControlActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericRecipeValueKind {
    Decimal,
    Integer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlledOutput {
    pub measurement_name: String,
    pub label: String,
    pub unit: String,
    pub target: f64,
    pub lower_spec: Option<f64>,
    pub upper_spec: Option<f64>,
    #[serde(default)]
    pub process_layer: Option<ProcessLayer>,
    pub measurement_step_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManipulatedParameter {
    pub key: String,
    pub label: String,
    pub value_kind: NumericRecipeValueKind,
    #[serde(default)]
    pub unit: Option<RecipeUnit>,
    pub current_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub max_delta: f64,
    pub output_sensitivity: f64,
    pub damping: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlLoop {
    pub id: ControlLoopId,
    pub name: String,
    pub route_id: String,
    pub process_step_id: String,
    pub tool_id: String,
    pub tool_class: ToolClass,
    pub recipe: RecipeBinding,
    pub output: ControlledOutput,
    #[serde(default)]
    pub manipulated_parameters: Vec<ManipulatedParameter>,
    pub ewma_lambda: f64,
    pub deadband: f64,
    pub minimum_confidence: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunReference {
    pub run_index: u32,
    pub lot_id: String,
    pub wafer_id: String,
    pub tool_run_id: String,
}

impl RunReference {
    pub(crate) fn from_measurement(run_index: u32, measurement: &ProcessMeasurement) -> Self {
        Self {
            run_index,
            lot_id: measurement.lot_id.clone(),
            wafer_id: measurement.wafer_id.clone(),
            tool_run_id: measurement.tool_run_id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlTrendPoint {
    pub source: RunReference,
    pub recipe: RecipeBinding,
    pub measurement_id: String,
    pub value: f64,
    pub target: f64,
    pub error: f64,
    pub ewma_error: f64,
    pub unit: String,
    pub in_spec: bool,
    #[serde(default)]
    pub yield_fraction: Option<f64>,
}

impl ControlTrendPoint {
    pub fn actionable(&self, loop_definition: &ControlLoop) -> bool {
        !self.in_spec || self.ewma_error.abs() > loop_definition.deadband.abs()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeParameterAdjustment {
    pub parameter_key: String,
    pub label: String,
    #[serde(default)]
    pub unit: Option<RecipeUnit>,
    pub previous_value: RecipeParameterValue,
    pub proposed_value: RecipeParameterValue,
    pub delta: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlActionState {
    Proposed,
    Approved,
    Rejected,
    Applied,
    Held,
}

impl ControlActionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Applied => "applied",
            Self::Held => "held",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlAction {
    pub id: ControlActionId,
    pub loop_id: ControlLoopId,
    pub source: RunReference,
    pub target_recipe: RecipeBinding,
    pub state: ControlActionState,
    pub measured_value: f64,
    pub target_value: f64,
    pub error: f64,
    pub ewma_error: f64,
    pub confidence: f64,
    pub proposed_at: String,
    #[serde(default)]
    pub adjustments: Vec<RecipeParameterAdjustment>,
    pub rationale: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAuditKind {
    Proposed,
    Approved,
    Rejected,
    Applied,
    Held,
}

impl ControlAuditKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Applied => "applied",
            Self::Held => "held",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAuditEvent {
    pub sequence: u64,
    #[serde(default)]
    pub action_id: Option<ControlActionId>,
    pub loop_id: ControlLoopId,
    pub actor: String,
    pub timestamp: String,
    pub kind: ControlAuditKind,
    #[serde(default)]
    pub from_state: Option<ControlActionState>,
    #[serde(default)]
    pub to_state: Option<ControlActionState>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessControlModel {
    pub loops: Vec<ControlLoop>,
    #[serde(default)]
    pub trends: BTreeMap<ControlLoopId, Vec<ControlTrendPoint>>,
    #[serde(default)]
    pub actions: Vec<ControlAction>,
    #[serde(default)]
    pub audit_events: Vec<ControlAuditEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessControlValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessControlValidationFinding {
    pub severity: ProcessControlValidationSeverity,
    pub message: String,
}

impl ProcessControlValidationFinding {
    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ProcessControlValidationSeverity::Error,
            message: message.into(),
        }
    }

    pub(crate) fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ProcessControlValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessControlMeasurementRef {
    pub(crate) name: String,
    pub(crate) step_id: String,
    pub(crate) lot_id: String,
    pub(crate) wafer_id: String,
    pub(crate) tool_run_id: String,
    pub(crate) recipe_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessControlValidationContext {
    pub recipe_versions: BTreeMap<String, BTreeSet<u32>>,
    pub(crate) measurements: BTreeMap<String, ProcessControlMeasurementRef>,
}

impl ProcessControlValidationContext {
    pub fn from_yield_analysis(analysis: &YieldAnalysis) -> Self {
        Self {
            recipe_versions: analysis.recipes.iter().fold(
                BTreeMap::new(),
                |mut versions, recipe| {
                    versions
                        .entry(recipe.id.clone())
                        .or_default()
                        .insert(recipe.version);
                    versions
                },
            ),
            measurements: analysis
                .process_measurements
                .iter()
                .map(|measurement| {
                    (
                        measurement.measurement_id.clone(),
                        ProcessControlMeasurementRef {
                            name: measurement.name.clone(),
                            step_id: measurement.step_id.clone(),
                            lot_id: measurement.lot_id.clone(),
                            wafer_id: measurement.wafer_id.clone(),
                            tool_run_id: measurement.tool_run_id.clone(),
                            recipe_id: measurement.recipe_id.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn contains_recipe_binding(&self, binding: &RecipeBinding) -> bool {
        self.recipe_versions
            .get(binding.recipe_id.as_str())
            .is_some_and(|versions| versions.contains(&binding.version.0))
    }

    pub(crate) fn contains_measurement_name(&self, measurement_name: &str) -> bool {
        self.measurements
            .values()
            .any(|measurement| measurement.name == measurement_name)
    }

    pub(crate) fn contains_measurement_name_at_step(
        &self,
        measurement_name: &str,
        step_id: &str,
    ) -> bool {
        self.measurements.values().any(|measurement| {
            measurement.name == measurement_name && measurement.step_id == step_id
        })
    }

    pub(crate) fn measurement(
        &self,
        measurement_id: &str,
    ) -> Option<&ProcessControlMeasurementRef> {
        self.measurements.get(measurement_id)
    }

    pub(crate) fn contains_source(&self, source: &RunReference) -> bool {
        self.measurements.values().any(|measurement| {
            measurement.lot_id == source.lot_id
                && measurement.wafer_id == source.wafer_id
                && measurement.tool_run_id == source.tool_run_id
        })
    }
}

impl ProcessControlModel {
    pub fn synthetic() -> Self {
        let analysis = YieldAnalysis::synthetic();
        Self::from_yield_analysis(&analysis)
    }

    pub fn from_yield_analysis(analysis: &YieldAnalysis) -> Self {
        let loops = sample_control_loops();
        let mut trends = BTreeMap::new();
        for loop_definition in &loops {
            trends.insert(
                loop_definition.id.clone(),
                trend_points_for_loop(analysis, loop_definition),
            );
        }

        let mut model = Self {
            loops,
            trends,
            actions: Vec::new(),
            audit_events: Vec::new(),
        };
        let loop_ids = model
            .loops
            .iter()
            .map(|loop_definition| loop_definition.id.clone())
            .collect::<Vec<_>>();
        for loop_id in loop_ids {
            if let Some(action) = model.recommend_action(&loop_id) {
                let event_kind = if action.state == ControlActionState::Held {
                    ControlAuditKind::Held
                } else {
                    ControlAuditKind::Proposed
                };
                let action_id = action.id.clone();
                model.actions.push(action);
                model.audit_events.push(ControlAuditEvent {
                    sequence: model.next_audit_sequence(),
                    action_id: Some(action_id),
                    loop_id,
                    actor: "r2r.controller".to_string(),
                    timestamp: "2026-05-06T14:00:00Z".to_string(),
                    kind: event_kind,
                    from_state: None,
                    to_state: Some(match event_kind {
                        ControlAuditKind::Held => ControlActionState::Held,
                        _ => ControlActionState::Proposed,
                    }),
                    note: "initial synthetic recommendation".to_string(),
                });
            }
        }
        model
    }

    pub fn loop_by_id(&self, id: &ControlLoopId) -> Option<&ControlLoop> {
        self.loops
            .iter()
            .find(|loop_definition| &loop_definition.id == id)
    }

    pub fn trend_for_loop(&self, id: &ControlLoopId) -> &[ControlTrendPoint] {
        self.trends.get(id).map(Vec::as_slice).unwrap_or_default()
    }

    pub fn actions_for_loop(&self, id: &ControlLoopId) -> Vec<&ControlAction> {
        let mut actions = self
            .actions
            .iter()
            .filter(|action| &action.loop_id == id)
            .collect::<Vec<_>>();
        actions.sort_by_key(|action| action.source.run_index);
        actions
    }

    pub fn audit_for_loop(&self, id: &ControlLoopId) -> Vec<&ControlAuditEvent> {
        self.audit_events
            .iter()
            .filter(|event| &event.loop_id == id)
            .collect()
    }

    pub fn recommend_action(&self, id: &ControlLoopId) -> Option<ControlAction> {
        let loop_definition = self.loop_by_id(id)?;
        let trend = self.trend_for_loop(id);
        let source = trend
            .iter()
            .rev()
            .find(|point| point.actionable(loop_definition))?;
        let adjustments = loop_definition
            .manipulated_parameters
            .iter()
            .filter_map(|parameter| adjustment_for_parameter(parameter, source.ewma_error))
            .collect::<Vec<_>>();
        if adjustments.is_empty() {
            return None;
        }

        let confidence = recommendation_confidence(source, trend.len(), loop_definition);
        let state = if confidence >= loop_definition.minimum_confidence {
            ControlActionState::Proposed
        } else {
            ControlActionState::Held
        };
        let id_suffix = stable_action_suffix(&loop_definition.id, source);
        Some(ControlAction {
            id: ControlActionId::new(format!("PCA-{id_suffix}")),
            loop_id: loop_definition.id.clone(),
            source: source.source.clone(),
            target_recipe: source.recipe.clone(),
            state,
            measured_value: source.value,
            target_value: source.target,
            error: source.error,
            ewma_error: source.ewma_error,
            confidence,
            proposed_at: "2026-05-06T14:00:00Z".to_string(),
            adjustments,
            rationale: format!(
                "{} EWMA error is {} {}; update {} before the next run.",
                loop_definition.output.label,
                format_signed(source.ewma_error),
                source.unit,
                source.recipe
            ),
        })
    }

    pub fn approve_action(
        &mut self,
        action_id: &ControlActionId,
        actor: impl Into<String>,
        timestamp: impl Into<String>,
        note: impl Into<String>,
    ) -> Result<(), ProcessControlError> {
        self.transition_action(
            action_id,
            ControlActionState::Approved,
            ControlAuditKind::Approved,
            actor,
            timestamp,
            note,
        )
    }

    pub fn reject_action(
        &mut self,
        action_id: &ControlActionId,
        actor: impl Into<String>,
        timestamp: impl Into<String>,
        note: impl Into<String>,
    ) -> Result<(), ProcessControlError> {
        self.transition_action(
            action_id,
            ControlActionState::Rejected,
            ControlAuditKind::Rejected,
            actor,
            timestamp,
            note,
        )
    }

    pub fn apply_action(
        &mut self,
        action_id: &ControlActionId,
        actor: impl Into<String>,
        timestamp: impl Into<String>,
        note: impl Into<String>,
    ) -> Result<(), ProcessControlError> {
        self.transition_action(
            action_id,
            ControlActionState::Applied,
            ControlAuditKind::Applied,
            actor,
            timestamp,
            note,
        )
    }

    pub(crate) fn transition_action(
        &mut self,
        action_id: &ControlActionId,
        to_state: ControlActionState,
        kind: ControlAuditKind,
        actor: impl Into<String>,
        timestamp: impl Into<String>,
        note: impl Into<String>,
    ) -> Result<(), ProcessControlError> {
        let Some(index) = self
            .actions
            .iter()
            .position(|action| &action.id == action_id)
        else {
            return Err(ProcessControlError::UnknownAction(action_id.clone()));
        };
        let from_state = self.actions[index].state;
        if !valid_transition(from_state, to_state) {
            return Err(ProcessControlError::InvalidTransition {
                action_id: action_id.clone(),
                from: from_state,
                to: to_state,
            });
        }

        self.actions[index].state = to_state;
        let loop_id = self.actions[index].loop_id.clone();
        self.audit_events.push(ControlAuditEvent {
            sequence: self.next_audit_sequence(),
            action_id: Some(action_id.clone()),
            loop_id,
            actor: actor.into(),
            timestamp: timestamp.into(),
            kind,
            from_state: Some(from_state),
            to_state: Some(to_state),
            note: note.into(),
        });
        Ok(())
    }

    pub(crate) fn next_audit_sequence(&self) -> u64 {
        self.audit_events
            .iter()
            .map(|event| event.sequence)
            .max()
            .unwrap_or(0)
            + 1
    }

    pub fn validate(&self) -> Vec<ProcessControlValidationFinding> {
        let mut findings = Vec::new();
        let mut loop_ids = BTreeSet::new();
        let mut parameter_keys_by_loop = BTreeMap::new();
        for loop_definition in &self.loops {
            validate_loop(
                loop_definition,
                &mut loop_ids,
                &mut parameter_keys_by_loop,
                &mut findings,
            );
        }

        for (loop_id, trend) in &self.trends {
            if !loop_ids.contains(loop_id) {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control trend references missing loop {loop_id}"
                )));
            }
            for point in trend {
                validate_trend_point(loop_id, point, &mut findings);
            }
        }

        let mut action_ids = BTreeSet::new();
        let mut action_sources = BTreeSet::new();
        let mut actions_by_id = BTreeMap::new();
        for action in &self.actions {
            validate_action(
                action,
                &loop_ids,
                &parameter_keys_by_loop,
                &mut action_ids,
                &mut action_sources,
                &mut findings,
            );
            if !action.id.as_str().trim().is_empty() {
                actions_by_id.entry(action.id.clone()).or_insert(action);
            }
            validate_action_trend_link(action, &self.trends, &mut findings);
        }

        let mut audit_sequences = BTreeSet::new();
        let mut audited_actions = BTreeSet::new();
        let mut latest_audit_state_by_action = BTreeMap::new();
        for event in &self.audit_events {
            if event.sequence == 0 {
                findings.push(ProcessControlValidationFinding::error(
                    "process-control audit sequence 0 is invalid",
                ));
            } else if !audit_sequences.insert(event.sequence) {
                findings.push(ProcessControlValidationFinding::warning(format!(
                    "process-control audit sequence {} is duplicated",
                    event.sequence
                )));
            }
            if !loop_ids.contains(&event.loop_id) {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control audit event {} references missing loop {}",
                    event.sequence, event.loop_id
                )));
            }
            if let Some(action_id) = event.action_id.as_ref()
                && !action_ids.contains(action_id)
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control audit event {} references missing action {}",
                    event.sequence, action_id
                )));
            }
            if let Some(action_id) = event.action_id.as_ref() {
                audited_actions.insert(action_id.clone());
                if let Some(action) = actions_by_id.get(action_id) {
                    if event.loop_id != action.loop_id {
                        findings.push(ProcessControlValidationFinding::error(format!(
                            "process-control audit event {} references action {} from loop {}, but event loop is {}",
                            event.sequence, action_id, action.loop_id, event.loop_id
                        )));
                    }
                    if valid_process_control_timestamp(&event.timestamp)
                        && valid_process_control_timestamp(&action.proposed_at)
                        && event.timestamp.as_str() < action.proposed_at.as_str()
                    {
                        findings.push(ProcessControlValidationFinding::error(format!(
                            "process-control audit event {} predates action {} proposal",
                            event.sequence, action_id
                        )));
                    }
                }
                if let Some(to_state) = event.to_state {
                    latest_audit_state_by_action
                        .entry(action_id.clone())
                        .and_modify(|(sequence, state)| {
                            if event.sequence >= *sequence {
                                *sequence = event.sequence;
                                *state = to_state;
                            }
                        })
                        .or_insert((event.sequence, to_state));
                }
            }
            if event.actor.trim().is_empty() || event.timestamp.trim().is_empty() {
                findings.push(ProcessControlValidationFinding::warning(format!(
                    "process-control audit event {} has incomplete audit metadata",
                    event.sequence
                )));
            }
            if !event.timestamp.trim().is_empty()
                && !valid_process_control_timestamp(&event.timestamp)
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control audit event {} has invalid timestamp {}",
                    event.sequence, event.timestamp
                )));
            }
            validate_audit_event_state(event, &mut findings);
            if let (Some(from), Some(to)) = (event.from_state, event.to_state)
                && !valid_transition(from, to)
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control audit event {} records invalid transition {} to {}",
                    event.sequence,
                    from.label(),
                    to.label()
                )));
            }
        }
        for action in &self.actions {
            if !audited_actions.contains(&action.id) {
                findings.push(ProcessControlValidationFinding::warning(format!(
                    "process-control action {} has no audit trail",
                    action.id
                )));
            }
            if let Some((_, audited_state)) = latest_audit_state_by_action.get(&action.id)
                && *audited_state != action.state
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control action {} state is {}, but latest audit records {}",
                    action.id,
                    action.state.label(),
                    audited_state.label()
                )));
            }
        }

        findings
    }

    pub fn validate_with_context(
        &self,
        context: &ProcessControlValidationContext,
    ) -> Vec<ProcessControlValidationFinding> {
        let mut findings = self.validate();
        self.validate_context_links(context, &mut findings);
        findings
    }

    pub(crate) fn validate_context_links(
        &self,
        context: &ProcessControlValidationContext,
        findings: &mut Vec<ProcessControlValidationFinding>,
    ) {
        let loops_by_id = self
            .loops
            .iter()
            .map(|loop_definition| (&loop_definition.id, loop_definition))
            .collect::<BTreeMap<_, _>>();

        for loop_definition in &self.loops {
            if !loop_definition.recipe.recipe_id.as_str().trim().is_empty()
                && !context.contains_recipe_binding(&loop_definition.recipe)
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control loop {} references missing recipe binding {}",
                    loop_definition.id, loop_definition.recipe
                )));
            }
            if !loop_definition.output.measurement_name.trim().is_empty()
                && !context.contains_measurement_name(&loop_definition.output.measurement_name)
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control loop {} references missing measurement name {}",
                    loop_definition.id, loop_definition.output.measurement_name
                )));
            }
            if !loop_definition.output.measurement_name.trim().is_empty()
                && !loop_definition.output.measurement_step_id.trim().is_empty()
                && !context.contains_measurement_name_at_step(
                    &loop_definition.output.measurement_name,
                    &loop_definition.output.measurement_step_id,
                )
            {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control loop {} references missing measurement {} at step {}",
                    loop_definition.id,
                    loop_definition.output.measurement_name,
                    loop_definition.output.measurement_step_id
                )));
            }
        }

        for (loop_id, trend) in &self.trends {
            let loop_definition = loops_by_id.get(loop_id);
            for point in trend {
                validate_trend_point_context(loop_definition.copied(), point, context, findings);
            }
        }

        for action in &self.actions {
            if !context.contains_recipe_binding(&action.target_recipe) {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control action {} references missing target recipe binding {}",
                    action.id, action.target_recipe
                )));
            }
            if !context.contains_source(&action.source) {
                findings.push(ProcessControlValidationFinding::error(format!(
                    "process-control action {} references missing source run {} / {} / {}",
                    action.id,
                    action.source.lot_id,
                    action.source.wafer_id,
                    action.source.tool_run_id
                )));
            }
        }
    }
}

pub(crate) fn validate_loop(
    loop_definition: &ControlLoop,
    loop_ids: &mut BTreeSet<ControlLoopId>,
    parameter_keys_by_loop: &mut BTreeMap<ControlLoopId, BTreeSet<String>>,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if loop_definition.id.as_str().trim().is_empty() {
        findings.push(ProcessControlValidationFinding::error(
            "process-control loop id is empty",
        ));
    } else if !loop_ids.insert(loop_definition.id.clone()) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {} is duplicated",
            loop_definition.id
        )));
    }
    for (field, value) in [
        ("name", loop_definition.name.as_str()),
        ("route id", loop_definition.route_id.as_str()),
        ("process step id", loop_definition.process_step_id.as_str()),
        ("tool id", loop_definition.tool_id.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(ProcessControlValidationFinding::warning(format!(
                "process-control loop {} has empty {field}",
                loop_definition.id
            )));
        }
    }
    if loop_definition.recipe.recipe_id.as_str().trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {} has an empty recipe id",
            loop_definition.id
        )));
    }
    validate_controlled_output(loop_definition, findings);

    if !loop_definition.ewma_lambda.is_finite() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {} has non-finite EWMA lambda",
            loop_definition.id
        )));
    } else if !(0.0..=1.0).contains(&loop_definition.ewma_lambda) {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {} EWMA lambda is outside 0..1",
            loop_definition.id
        )));
    }
    if !loop_definition.deadband.is_finite() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {} has non-finite deadband",
            loop_definition.id
        )));
    }
    if !loop_definition.minimum_confidence.is_finite() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {} has non-finite minimum confidence",
            loop_definition.id
        )));
    } else if !(0.0..=1.0).contains(&loop_definition.minimum_confidence) {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {} minimum confidence is outside 0..1",
            loop_definition.id
        )));
    }

    let mut parameter_keys = BTreeSet::new();
    for parameter in &loop_definition.manipulated_parameters {
        validate_manipulated_parameter(
            &loop_definition.id,
            parameter,
            &mut parameter_keys,
            findings,
        );
    }
    parameter_keys_by_loop.insert(loop_definition.id.clone(), parameter_keys);
}

pub(crate) fn validate_controlled_output(
    loop_definition: &ControlLoop,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    let output = &loop_definition.output;
    if output.measurement_name.trim().is_empty() || output.unit.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {} has incomplete output metadata",
            loop_definition.id
        )));
    }
    for (label, value) in [
        ("target", Some(output.target)),
        ("lower spec", output.lower_spec),
        ("upper spec", output.upper_spec),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control loop {} output has non-finite {label}",
                loop_definition.id
            )));
        }
    }
    if let (Some(lower), Some(upper)) = (output.lower_spec, output.upper_spec)
        && lower > upper
    {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {} output lower spec is above upper spec",
            loop_definition.id
        )));
    }
}

pub(crate) fn validate_manipulated_parameter(
    loop_id: &ControlLoopId,
    parameter: &ManipulatedParameter,
    parameter_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if parameter.key.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {loop_id} has an empty parameter key"
        )));
    } else if !parameter_keys.insert(parameter.key.clone()) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {loop_id} parameter {} is duplicated",
            parameter.key
        )));
    }
    for (label, value) in [
        ("current value", parameter.current_value),
        ("lower bound", parameter.lower_bound),
        ("upper bound", parameter.upper_bound),
        ("max delta", parameter.max_delta),
        ("output sensitivity", parameter.output_sensitivity),
        ("damping", parameter.damping),
    ] {
        if !value.is_finite() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control loop {loop_id} parameter {} has non-finite {label}",
                parameter.key
            )));
        }
    }
    if parameter.lower_bound > parameter.upper_bound {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control loop {loop_id} parameter {} lower bound is above upper bound",
            parameter.key
        )));
    }
    if parameter.max_delta < 0.0 {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {loop_id} parameter {} has negative max delta",
            parameter.key
        )));
    }
    if parameter.output_sensitivity.abs() <= f64::EPSILON {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control loop {loop_id} parameter {} has zero output sensitivity",
            parameter.key
        )));
    }
}

pub(crate) fn validate_trend_point(
    loop_id: &ControlLoopId,
    point: &ControlTrendPoint,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    validate_run_reference(
        &format!(
            "process-control trend {} for loop {loop_id}",
            point.measurement_id
        ),
        &point.source,
        findings,
    );
    if point.measurement_id.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control trend for loop {loop_id} has empty measurement id"
        )));
    }
    if point.recipe.recipe_id.as_str().trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control trend {} for loop {loop_id} has empty recipe id",
            point.measurement_id
        )));
    }
    if point.unit.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control trend {} for loop {loop_id} has empty unit",
            point.measurement_id
        )));
    }
    for (label, value) in [
        ("value", point.value),
        ("target", point.target),
        ("error", point.error),
        ("EWMA error", point.ewma_error),
    ] {
        if !value.is_finite() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control trend {} for loop {loop_id} has non-finite {label}",
                point.measurement_id
            )));
        }
    }
    if (point.target - point.value - point.error).abs() > 0.001 {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control trend {} for loop {loop_id} has inconsistent target/value/error",
            point.measurement_id
        )));
    }
    if let Some(yield_fraction) = point.yield_fraction
        && (!yield_fraction.is_finite() || !(0.0..=1.0).contains(&yield_fraction))
    {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control trend {} for loop {loop_id} has invalid yield fraction",
            point.measurement_id
        )));
    }
}

pub(crate) fn validate_trend_point_context(
    loop_definition: Option<&ControlLoop>,
    point: &ControlTrendPoint,
    context: &ProcessControlValidationContext,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if !context.contains_recipe_binding(&point.recipe) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control trend {} references missing recipe binding {}",
            point.measurement_id, point.recipe
        )));
    }
    if !context.contains_source(&point.source) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control trend {} references missing source run {} / {} / {}",
            point.measurement_id,
            point.source.lot_id,
            point.source.wafer_id,
            point.source.tool_run_id
        )));
    }

    let Some(measurement) = context.measurement(&point.measurement_id) else {
        if !point.measurement_id.trim().is_empty() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control trend references missing measurement {}",
                point.measurement_id
            )));
        }
        return;
    };

    if measurement.recipe_id != point.recipe.recipe_id.as_str() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control trend {} uses recipe {}, but source measurement uses {}",
            point.measurement_id, point.recipe.recipe_id, measurement.recipe_id
        )));
    }

    if let Some(loop_definition) = loop_definition {
        if measurement.name != loop_definition.output.measurement_name {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control trend {} belongs to measurement {}, but loop {} controls {}",
                point.measurement_id,
                measurement.name,
                loop_definition.id,
                loop_definition.output.measurement_name
            )));
        }
        if !loop_definition.output.measurement_step_id.trim().is_empty()
            && measurement.step_id != loop_definition.output.measurement_step_id
        {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control trend {} belongs to step {}, but loop {} expects {}",
                point.measurement_id,
                measurement.step_id,
                loop_definition.id,
                loop_definition.output.measurement_step_id
            )));
        }
    }
}

pub(crate) fn validate_action(
    action: &ControlAction,
    loop_ids: &BTreeSet<ControlLoopId>,
    parameter_keys_by_loop: &BTreeMap<ControlLoopId, BTreeSet<String>>,
    action_ids: &mut BTreeSet<ControlActionId>,
    action_sources: &mut BTreeSet<(ControlLoopId, u32, String, String, String)>,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if action.id.as_str().trim().is_empty() {
        findings.push(ProcessControlValidationFinding::error(
            "process-control action id is empty",
        ));
    } else if !action_ids.insert(action.id.clone()) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} is duplicated",
            action.id
        )));
    }
    if !loop_ids.contains(&action.loop_id) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} references missing loop {}",
            action.id, action.loop_id
        )));
    }
    validate_run_reference(
        &format!("process-control action {}", action.id),
        &action.source,
        findings,
    );
    let action_source = (
        action.loop_id.clone(),
        action.source.run_index,
        action.source.lot_id.clone(),
        action.source.wafer_id.clone(),
        action.source.tool_run_id.clone(),
    );
    if !action_sources.insert(action_source) {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control action {} duplicates an existing action for source run {} / {} / {}",
            action.id, action.source.lot_id, action.source.wafer_id, action.source.tool_run_id
        )));
    }
    if action.proposed_at.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control action {} has empty proposal timestamp",
            action.id
        )));
    } else if !valid_process_control_timestamp(&action.proposed_at) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} has invalid proposal timestamp {}",
            action.id, action.proposed_at
        )));
    }
    if action.rationale.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control action {} has empty rationale",
            action.id
        )));
    }
    for (label, value) in [
        ("measured value", action.measured_value),
        ("target value", action.target_value),
        ("error", action.error),
        ("EWMA error", action.ewma_error),
        ("confidence", action.confidence),
    ] {
        if !value.is_finite() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control action {} has non-finite {label}",
                action.id
            )));
        }
    }
    if (action.target_value - action.measured_value - action.error).abs() > 0.001 {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control action {} has inconsistent target/measured/error",
            action.id
        )));
    }
    if !(0.0..=1.0).contains(&action.confidence) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} has confidence outside 0..1",
            action.id
        )));
    }
    if action.adjustments.is_empty() {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control action {} has no recipe adjustments",
            action.id
        )));
    }
    let parameter_keys = parameter_keys_by_loop.get(&action.loop_id);
    let mut adjustment_keys = BTreeSet::new();
    for adjustment in &action.adjustments {
        if !adjustment.parameter_key.trim().is_empty()
            && !adjustment_keys.insert(adjustment.parameter_key.clone())
        {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control action {} has duplicate adjustment parameter {}",
                action.id, adjustment.parameter_key
            )));
        }
        validate_adjustment(&action.id, adjustment, parameter_keys, findings);
    }
}

pub(crate) fn validate_action_trend_link(
    action: &ControlAction,
    trends: &BTreeMap<ControlLoopId, Vec<ControlTrendPoint>>,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    let Some(point) = trends.get(&action.loop_id).and_then(|trend| {
        trend
            .iter()
            .find(|point| same_source(&point.source, &action.source))
    }) else {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} references source run {} / {} / {} that is not present in loop {} trend",
            action.id,
            action.source.lot_id,
            action.source.wafer_id,
            action.source.tool_run_id,
            action.loop_id
        )));
        return;
    };

    if action.target_recipe != point.recipe {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {} targets recipe {}, but source trend {} uses {}",
            action.id, action.target_recipe, point.measurement_id, point.recipe
        )));
    }
    for (label, action_value, trend_value) in [
        ("measured value", action.measured_value, point.value),
        ("target value", action.target_value, point.target),
        ("error", action.error, point.error),
        ("EWMA error", action.ewma_error, point.ewma_error),
    ] {
        if action_value.is_finite()
            && trend_value.is_finite()
            && (action_value - trend_value).abs() > 0.001
        {
            findings.push(ProcessControlValidationFinding::warning(format!(
                "process-control action {} {label} does not match source trend {}",
                action.id, point.measurement_id
            )));
        }
    }
}

pub(crate) fn validate_adjustment(
    action_id: &ControlActionId,
    adjustment: &RecipeParameterAdjustment,
    parameter_keys: Option<&BTreeSet<String>>,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if adjustment.parameter_key.trim().is_empty() {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {action_id} has an empty adjustment parameter key"
        )));
    } else if parameter_keys.is_some_and(|keys| !keys.contains(&adjustment.parameter_key)) {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {action_id} adjustment references missing parameter {}",
            adjustment.parameter_key
        )));
    }
    for (label, value) in [
        ("delta", adjustment.delta),
        ("lower bound", adjustment.lower_bound),
        ("upper bound", adjustment.upper_bound),
    ] {
        if !value.is_finite() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control action {action_id} adjustment {} has non-finite {label}",
                adjustment.parameter_key
            )));
        }
    }
    if adjustment.lower_bound > adjustment.upper_bound {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control action {action_id} adjustment {} lower bound is above upper bound",
            adjustment.parameter_key
        )));
    }
    for (label, value) in [
        ("previous value", adjustment.previous_value.as_f64()),
        ("proposed value", adjustment.proposed_value.as_f64()),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control action {action_id} adjustment {} has non-finite {label}",
                adjustment.parameter_key
            )));
        }
    }
}

pub(crate) fn validate_audit_event_state(
    event: &ControlAuditEvent,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    let expected_to_state = audit_kind_state(event.kind);
    match event.to_state {
        Some(to_state) if to_state != expected_to_state => {
            findings.push(ProcessControlValidationFinding::error(format!(
                "process-control audit event {} kind {} records to_state {}",
                event.sequence,
                event.kind.label(),
                to_state.label()
            )));
        }
        Some(_) => {}
        None => findings.push(ProcessControlValidationFinding::error(format!(
            "process-control audit event {} has no to_state",
            event.sequence
        ))),
    }
    if matches!(
        event.kind,
        ControlAuditKind::Approved | ControlAuditKind::Rejected | ControlAuditKind::Applied
    ) && event.from_state.is_none()
    {
        findings.push(ProcessControlValidationFinding::error(format!(
            "process-control audit event {} has no from_state",
            event.sequence
        )));
    }
    if event.note.trim().is_empty()
        && matches!(
            event.kind,
            ControlAuditKind::Approved
                | ControlAuditKind::Rejected
                | ControlAuditKind::Applied
                | ControlAuditKind::Held
        )
    {
        findings.push(ProcessControlValidationFinding::warning(format!(
            "process-control audit event {} has empty note",
            event.sequence
        )));
    }
}

pub(crate) fn audit_kind_state(kind: ControlAuditKind) -> ControlActionState {
    match kind {
        ControlAuditKind::Proposed => ControlActionState::Proposed,
        ControlAuditKind::Approved => ControlActionState::Approved,
        ControlAuditKind::Rejected => ControlActionState::Rejected,
        ControlAuditKind::Applied => ControlActionState::Applied,
        ControlAuditKind::Held => ControlActionState::Held,
    }
}

pub(crate) fn validate_run_reference(
    context: &str,
    source: &RunReference,
    findings: &mut Vec<ProcessControlValidationFinding>,
) {
    if source.run_index == 0 {
        findings.push(ProcessControlValidationFinding::error(format!(
            "{context} has source run index 0"
        )));
    }
    for (field, value) in [
        ("lot id", source.lot_id.as_str()),
        ("wafer id", source.wafer_id.as_str()),
        ("tool run id", source.tool_run_id.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(ProcessControlValidationFinding::error(format!(
                "{context} has empty source {field}"
            )));
        }
    }
}

pub(crate) fn same_source(left: &RunReference, right: &RunReference) -> bool {
    left.run_index == right.run_index
        && left.lot_id == right.lot_id
        && left.wafer_id == right.wafer_id
        && left.tool_run_id == right.tool_run_id
}

pub(crate) fn valid_process_control_timestamp(timestamp: &str) -> bool {
    if timestamp.len() != 20 || !timestamp.ends_with('Z') {
        return false;
    }
    let bytes = timestamp.as_bytes();
    for (index, expected) in [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')] {
        if bytes[index] != expected {
            return false;
        }
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit())
    {
        return false;
    }
    let year = parse_timestamp_field(timestamp, 0, 4);
    let month = parse_timestamp_field(timestamp, 5, 7);
    let day = parse_timestamp_field(timestamp, 8, 10);
    let hour = parse_timestamp_field(timestamp, 11, 13);
    let minute = parse_timestamp_field(timestamp, 14, 16);
    let second = parse_timestamp_field(timestamp, 17, 19);

    (1900..=9999).contains(&year)
        && valid_calendar_day(year, month, day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

pub(crate) fn parse_timestamp_field(timestamp: &str, start: usize, end: usize) -> u32 {
    timestamp[start..end].parse().unwrap_or_default()
}

pub(crate) fn valid_calendar_day(year: u32, month: u32, day: u32) -> bool {
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

pub(crate) fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessControlError {
    UnknownAction(ControlActionId),
    InvalidTransition {
        action_id: ControlActionId,
        from: ControlActionState,
        to: ControlActionState,
    },
}

impl fmt::Display for ProcessControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAction(action_id) => write!(f, "unknown control action {action_id}"),
            Self::InvalidTransition {
                action_id,
                from,
                to,
            } => write!(
                f,
                "cannot move control action {action_id} from {} to {}",
                from.label(),
                to.label()
            ),
        }
    }
}

impl Error for ProcessControlError {}

pub fn sample_control_loops() -> Vec<ControlLoop> {
    vec![
        ControlLoop {
            id: ControlLoopId::from("R2R-POLY-EDGE-OX"),
            name: "Poly etch edge oxide trim".to_string(),
            route_id: "INV_ROUTE_A".to_string(),
            process_step_id: "ETCH_POLY".to_string(),
            tool_id: "ETCHER-02".to_string(),
            tool_class: ToolClass::PlasmaEtcher,
            recipe: RecipeBinding::new("POLY_ETCH_004", 4),
            output: ControlledOutput {
                measurement_name: "edge_oxide_thickness".to_string(),
                label: "Edge oxide thickness".to_string(),
                unit: "nm".to_string(),
                target: 100.0,
                lower_spec: Some(96.0),
                upper_spec: Some(104.0),
                process_layer: Some(ProcessLayer::Oxide),
                measurement_step_id: "MET_POLY_POST".to_string(),
            },
            manipulated_parameters: vec![ManipulatedParameter {
                key: "etch_time_s".to_string(),
                label: "Etch time".to_string(),
                value_kind: NumericRecipeValueKind::Decimal,
                unit: Some(RecipeUnit::Second),
                current_value: 58.0,
                lower_bound: 45.0,
                upper_bound: 75.0,
                max_delta: 5.0,
                output_sensitivity: -0.72,
                damping: 0.45,
            }],
            ewma_lambda: 0.55,
            deadband: 0.65,
            minimum_confidence: 0.55,
        },
        ControlLoop {
            id: ControlLoopId::from("R2R-CONTACT-R"),
            name: "Contact resistance cleanup".to_string(),
            route_id: "INV_ROUTE_A".to_string(),
            process_step_id: "ETCH_POLY".to_string(),
            tool_id: "ETCHER-02".to_string(),
            tool_class: ToolClass::PlasmaEtcher,
            recipe: RecipeBinding::new("POLY_ETCH_004", 4),
            output: ControlledOutput {
                measurement_name: "contact_resistance".to_string(),
                label: "Contact resistance".to_string(),
                unit: "ohm".to_string(),
                target: 9.0,
                lower_spec: Some(5.0),
                upper_spec: Some(12.0),
                process_layer: Some(ProcessLayer::Contact),
                measurement_step_id: "MET_POLY_POST".to_string(),
            },
            manipulated_parameters: vec![
                ManipulatedParameter {
                    key: "rf_power_w".to_string(),
                    label: "RF power".to_string(),
                    value_kind: NumericRecipeValueKind::Integer,
                    unit: Some(RecipeUnit::Watt),
                    current_value: 150.0,
                    lower_bound: 80.0,
                    upper_bound: 220.0,
                    max_delta: 15.0,
                    output_sensitivity: -0.10,
                    damping: 0.30,
                },
                ManipulatedParameter {
                    key: "o2_flow_sccm".to_string(),
                    label: "O2 flow".to_string(),
                    value_kind: NumericRecipeValueKind::Integer,
                    unit: Some(RecipeUnit::Sccm),
                    current_value: 8.0,
                    lower_bound: 0.0,
                    upper_bound: 40.0,
                    max_delta: 3.0,
                    output_sensitivity: -0.35,
                    damping: 0.35,
                },
            ],
            ewma_lambda: 0.6,
            deadband: 0.45,
            minimum_confidence: 0.50,
        },
    ]
}

pub(crate) fn trend_points_for_loop(
    analysis: &YieldAnalysis,
    loop_definition: &ControlLoop,
) -> Vec<ControlTrendPoint> {
    let mut measurements = analysis
        .process_measurements
        .iter()
        .filter(|measurement| measurement.name == loop_definition.output.measurement_name)
        .collect::<Vec<_>>();
    measurements.sort_by(|left, right| {
        left.lot_id
            .cmp(&right.lot_id)
            .then_with(|| left.wafer_id.cmp(&right.wafer_id))
            .then_with(|| left.measurement_id.cmp(&right.measurement_id))
    });

    let mut ewma_error = None;
    measurements
        .into_iter()
        .enumerate()
        .map(|(index, measurement)| {
            let run_index = u32::try_from(index + 1).unwrap_or_else(|_| {
                warn!(
                    trend_index = index,
                    max_run_index = u32::MAX,
                    "control trend run index exceeded u32 range; clamping run index"
                );
                u32::MAX
            });
            let target = measurement.target.unwrap_or_else(|| {
                warn!(
                    measurement_id = %measurement.measurement_id,
                    loop_id = %loop_definition.id,
                    fallback_target = loop_definition.output.target,
                    "process measurement missing target; using control loop target"
                );
                loop_definition.output.target
            });
            let error = target - measurement.value;
            let lambda = loop_definition.ewma_lambda.clamp(0.0, 1.0);
            if lambda != loop_definition.ewma_lambda {
                warn!(
                    loop_id = %loop_definition.id,
                    requested_lambda = loop_definition.ewma_lambda,
                    clamped_lambda = lambda,
                    "control loop EWMA lambda outside range; clamping to [0, 1]"
                );
            }
            let smoothed = match ewma_error {
                Some(previous) => lambda * error + (1.0 - lambda) * previous,
                None => error,
            };
            ewma_error = Some(smoothed);
            let lower_spec = measurement.lower_spec.or(loop_definition.output.lower_spec);
            let upper_spec = measurement.upper_spec.or(loop_definition.output.upper_spec);
            ControlTrendPoint {
                source: RunReference::from_measurement(run_index, measurement),
                recipe: recipe_binding_for_measurement(analysis, measurement),
                measurement_id: measurement.measurement_id.clone(),
                value: measurement.value,
                target,
                error,
                ewma_error: smoothed,
                unit: if measurement.unit.is_empty() {
                    warn!(
                        measurement_id = %measurement.measurement_id,
                        loop_id = %loop_definition.id,
                        fallback_unit = %loop_definition.output.unit,
                        "process measurement missing unit; using control loop unit"
                    );
                    loop_definition.output.unit.clone()
                } else {
                    measurement.unit.clone()
                },
                in_spec: in_spec(measurement.value, lower_spec, upper_spec),
                yield_fraction: analysis
                    .wafer_summary(&measurement.lot_id, &measurement.wafer_id)
                    .map(|summary| summary.yield_fraction),
            }
        })
        .collect()
}
