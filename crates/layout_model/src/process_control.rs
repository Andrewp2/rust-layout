use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    ProcessLayer,
    recipe::{RecipeBinding, RecipeId, RecipeParameterValue, RecipeUnit, ToolClass},
    yield_analysis::{ProcessMeasurement, YieldAnalysis},
};

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
    fn from_measurement(run_index: u32, measurement: &ProcessMeasurement) -> Self {
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

    fn transition_action(
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

    fn next_audit_sequence(&self) -> u64 {
        self.audit_events
            .iter()
            .map(|event| event.sequence)
            .max()
            .unwrap_or(0)
            + 1
    }
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

fn trend_points_for_loop(
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
            let run_index = u32::try_from(index + 1).unwrap_or(u32::MAX);
            let target = measurement.target.unwrap_or(loop_definition.output.target);
            let error = target - measurement.value;
            let lambda = loop_definition.ewma_lambda.clamp(0.0, 1.0);
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

fn recipe_binding_for_measurement(
    analysis: &YieldAnalysis,
    measurement: &ProcessMeasurement,
) -> RecipeBinding {
    let version = analysis
        .recipes
        .iter()
        .find(|recipe| recipe.id == measurement.recipe_id)
        .map(|recipe| recipe.version)
        .unwrap_or(1);
    RecipeBinding::new(RecipeId::new(measurement.recipe_id.clone()), version)
}

fn adjustment_for_parameter(
    parameter: &ManipulatedParameter,
    effective_error: f64,
) -> Option<RecipeParameterAdjustment> {
    if parameter.output_sensitivity.abs() <= f64::EPSILON {
        return None;
    }
    let max_delta = parameter.max_delta.abs();
    let raw_delta = effective_error / parameter.output_sensitivity * parameter.damping;
    let clamped_delta = raw_delta.clamp(-max_delta, max_delta);
    let previous_numeric = parameter
        .current_value
        .clamp(parameter.lower_bound, parameter.upper_bound);
    let proposed_numeric = numeric_recipe_value(
        parameter.value_kind,
        previous_numeric + clamped_delta,
        parameter.lower_bound,
        parameter.upper_bound,
    );
    let previous_numeric = numeric_recipe_value(
        parameter.value_kind,
        previous_numeric,
        parameter.lower_bound,
        parameter.upper_bound,
    );
    let delta = proposed_numeric - previous_numeric;
    if delta.abs() <= f64::EPSILON {
        return None;
    }

    Some(RecipeParameterAdjustment {
        parameter_key: parameter.key.clone(),
        label: parameter.label.clone(),
        unit: parameter.unit,
        previous_value: recipe_value(parameter.value_kind, previous_numeric),
        proposed_value: recipe_value(parameter.value_kind, proposed_numeric),
        delta,
        lower_bound: parameter.lower_bound,
        upper_bound: parameter.upper_bound,
    })
}

fn numeric_recipe_value(
    value_kind: NumericRecipeValueKind,
    value: f64,
    lower_bound: f64,
    upper_bound: f64,
) -> f64 {
    let value = value.clamp(lower_bound, upper_bound);
    match value_kind {
        NumericRecipeValueKind::Decimal => round_to(value, 3),
        NumericRecipeValueKind::Integer => value.round(),
    }
}

fn recipe_value(value_kind: NumericRecipeValueKind, value: f64) -> RecipeParameterValue {
    match value_kind {
        NumericRecipeValueKind::Decimal => RecipeParameterValue::Decimal(value),
        NumericRecipeValueKind::Integer => RecipeParameterValue::Integer(value as i64),
    }
}

fn recommendation_confidence(
    source: &ControlTrendPoint,
    trend_len: usize,
    loop_definition: &ControlLoop,
) -> f64 {
    let tolerance = match (
        loop_definition.output.lower_spec,
        loop_definition.output.upper_spec,
    ) {
        (Some(lower), Some(upper)) => ((upper - lower) * 0.5).abs().max(loop_definition.deadband),
        _ => loop_definition
            .output
            .target
            .abs()
            .mul_add(0.05, 0.0)
            .max(1.0),
    };
    let error_penalty = (source.ewma_error.abs() / tolerance).min(4.0) * 0.08;
    let sample_bonus = (trend_len.min(12) as f64) * 0.012;
    (0.74 + sample_bonus - error_penalty).clamp(0.05, 0.98)
}

fn valid_transition(from: ControlActionState, to: ControlActionState) -> bool {
    matches!(
        (from, to),
        (ControlActionState::Proposed, ControlActionState::Approved)
            | (ControlActionState::Proposed, ControlActionState::Rejected)
            | (ControlActionState::Proposed, ControlActionState::Held)
            | (ControlActionState::Held, ControlActionState::Proposed)
            | (ControlActionState::Held, ControlActionState::Rejected)
            | (ControlActionState::Approved, ControlActionState::Applied)
            | (ControlActionState::Approved, ControlActionState::Rejected)
    )
}

fn in_spec(value: f64, lower_spec: Option<f64>, upper_spec: Option<f64>) -> bool {
    !lower_spec.is_some_and(|lower| value < lower) && !upper_spec.is_some_and(|upper| value > upper)
}

fn stable_action_suffix(loop_id: &ControlLoopId, source: &ControlTrendPoint) -> String {
    format!(
        "{}-{}-{}-{:02}",
        loop_id.as_str().replace('_', "-"),
        source.source.lot_id,
        source.source.wafer_id,
        source.source.run_index
    )
}

fn format_signed(value: f64) -> String {
    if value.abs() >= 10.0 {
        format!("{value:+.1}")
    } else {
        format!("{value:+.2}")
    }
}

fn round_to(value: f64, precision: u32) -> f64 {
    let scale = 10_f64.powi(precision as i32);
    (value * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_model_links_yield_measurements_to_control_actions() {
        let analysis = YieldAnalysis::synthetic();
        let model = ProcessControlModel::from_yield_analysis(&analysis);
        let edge_loop = ControlLoopId::from("R2R-POLY-EDGE-OX");
        let contact_loop = ControlLoopId::from("R2R-CONTACT-R");

        assert_eq!(model.loops.len(), 2);
        assert_eq!(model.trend_for_loop(&edge_loop).len(), 8);
        assert_eq!(model.trend_for_loop(&contact_loop).len(), 8);
        assert!(
            model
                .actions_for_loop(&edge_loop)
                .iter()
                .any(|action| action.target_recipe.recipe_id.as_str() == "POLY_ETCH_004")
        );
        assert!(model.audit_for_loop(&edge_loop).len() >= 1);
    }

    #[test]
    fn recommendation_moves_recipe_parameter_against_process_error() {
        let model = ProcessControlModel::from_yield_analysis(&YieldAnalysis::synthetic());
        let edge_loop = ControlLoopId::from("R2R-POLY-EDGE-OX");
        let action = model.recommend_action(&edge_loop).unwrap();
        let adjustment = action
            .adjustments
            .iter()
            .find(|adjustment| adjustment.parameter_key == "etch_time_s")
            .unwrap();

        assert!(action.ewma_error > 0.0);
        assert!(adjustment.delta < 0.0);
        assert!(adjustment.delta.abs() <= 5.0);
        assert_eq!(
            adjustment.previous_value,
            RecipeParameterValue::Decimal(58.0)
        );
    }

    #[test]
    fn parameter_adjustment_respects_delta_and_recipe_bounds() {
        let parameter = ManipulatedParameter {
            key: "time_s".to_string(),
            label: "Time".to_string(),
            value_kind: NumericRecipeValueKind::Decimal,
            unit: Some(RecipeUnit::Second),
            current_value: 49.0,
            lower_bound: 45.0,
            upper_bound: 55.0,
            max_delta: 3.0,
            output_sensitivity: 0.5,
            damping: 1.0,
        };

        let adjustment = adjustment_for_parameter(&parameter, 8.0).unwrap();

        assert_eq!(
            adjustment.proposed_value,
            RecipeParameterValue::Decimal(52.0)
        );
        assert_eq!(adjustment.delta, 3.0);
    }

    #[test]
    fn action_state_changes_are_audited_and_guarded() {
        let mut model = ProcessControlModel::from_yield_analysis(&YieldAnalysis::synthetic());
        let action_id = model.actions.first().unwrap().id.clone();

        model
            .approve_action(
                &action_id,
                "process.eng",
                "2026-05-06T15:00:00Z",
                "accept trim",
            )
            .unwrap();
        model
            .apply_action(
                &action_id,
                "recipe.bot",
                "2026-05-06T15:05:00Z",
                "staged override",
            )
            .unwrap();

        let action = model
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .unwrap();
        assert_eq!(action.state, ControlActionState::Applied);
        assert!(model.audit_events.len() >= 3);
        let invalid = model.reject_action(
            &action_id,
            "process.eng",
            "2026-05-06T15:06:00Z",
            "too late",
        );
        assert!(matches!(
            invalid,
            Err(ProcessControlError::InvalidTransition { .. })
        ));
    }
}
