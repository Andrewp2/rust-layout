#![allow(unused_imports)]
use super::*;

pub(crate) fn recipe_binding_for_measurement(
    analysis: &YieldAnalysis,
    measurement: &ProcessMeasurement,
) -> RecipeBinding {
    let version = analysis
        .recipes
        .iter()
        .find(|recipe| recipe.id == measurement.recipe_id)
        .map(|recipe| recipe.version)
        .unwrap_or_else(|| {
            warn!(
                measurement_id = %measurement.measurement_id,
                recipe_id = %measurement.recipe_id,
                "process measurement recipe missing from analysis; using recipe version 1"
            );
            1
        });
    RecipeBinding::new(RecipeId::new(measurement.recipe_id.clone()), version)
}

pub(crate) fn adjustment_for_parameter(
    parameter: &ManipulatedParameter,
    effective_error: f64,
) -> Option<RecipeParameterAdjustment> {
    if parameter.output_sensitivity.abs() <= f64::EPSILON {
        return None;
    }
    if parameter.max_delta < 0.0 {
        warn!(
            parameter_key = %parameter.key,
            max_delta = parameter.max_delta,
            "manipulated parameter max delta was negative; using absolute value"
        );
    }
    let max_delta = parameter.max_delta.abs();
    let raw_delta = effective_error / parameter.output_sensitivity * parameter.damping;
    let clamped_delta = raw_delta.clamp(-max_delta, max_delta);
    if clamped_delta != raw_delta {
        warn!(
            parameter_key = %parameter.key,
            raw_delta,
            clamped_delta,
            max_delta,
            "recipe parameter adjustment exceeded max delta; clamping"
        );
    }
    let previous_numeric = parameter
        .current_value
        .clamp(parameter.lower_bound, parameter.upper_bound);
    if previous_numeric != parameter.current_value {
        warn!(
            parameter_key = %parameter.key,
            current_value = parameter.current_value,
            clamped_value = previous_numeric,
            lower_bound = parameter.lower_bound,
            upper_bound = parameter.upper_bound,
            "current recipe parameter value outside bounds; clamping"
        );
    }
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

pub(crate) fn numeric_recipe_value(
    value_kind: NumericRecipeValueKind,
    value: f64,
    lower_bound: f64,
    upper_bound: f64,
) -> f64 {
    let raw_value = value;
    let value = raw_value.clamp(lower_bound, upper_bound);
    if value != raw_value {
        warn!(
            raw_value,
            clamped_value = value,
            lower_bound,
            upper_bound,
            "numeric recipe value outside bounds; clamping"
        );
    }
    match value_kind {
        NumericRecipeValueKind::Decimal => round_to(value, 3),
        NumericRecipeValueKind::Integer => value.round(),
    }
}

pub(crate) fn recipe_value(value_kind: NumericRecipeValueKind, value: f64) -> RecipeParameterValue {
    match value_kind {
        NumericRecipeValueKind::Decimal => RecipeParameterValue::Decimal(value),
        NumericRecipeValueKind::Integer => RecipeParameterValue::Integer(value as i64),
    }
}

pub(crate) fn recommendation_confidence(
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
    if loop_definition.output.lower_spec.is_none() || loop_definition.output.upper_spec.is_none() {
        warn!(
            loop_id = %loop_definition.id,
            fallback_tolerance = tolerance,
            "control loop missing one or both spec limits; using derived recommendation tolerance"
        );
    }
    let raw_error_penalty = source.ewma_error.abs() / tolerance;
    let error_penalty_scale = raw_error_penalty.min(4.0);
    if error_penalty_scale != raw_error_penalty {
        warn!(
            loop_id = %loop_definition.id,
            raw_error_penalty,
            clamped_error_penalty = error_penalty_scale,
            "recommendation error penalty exceeded cap; clamping"
        );
    }
    let error_penalty = error_penalty_scale * 0.08;
    let capped_trend_len = trend_len.min(12);
    if capped_trend_len != trend_len {
        warn!(
            loop_id = %loop_definition.id,
            trend_len,
            capped_trend_len,
            "recommendation sample bonus exceeded cap; clamping"
        );
    }
    let sample_bonus = (capped_trend_len as f64) * 0.012;
    let confidence = 0.74 + sample_bonus - error_penalty;
    let clamped_confidence = confidence.clamp(0.05, 0.98);
    if clamped_confidence != confidence {
        warn!(
            loop_id = %loop_definition.id,
            confidence,
            clamped_confidence,
            "recommendation confidence outside range; clamping"
        );
    }
    clamped_confidence
}

pub(crate) fn valid_transition(from: ControlActionState, to: ControlActionState) -> bool {
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

pub(crate) fn in_spec(value: f64, lower_spec: Option<f64>, upper_spec: Option<f64>) -> bool {
    !lower_spec.is_some_and(|lower| value < lower) && !upper_spec.is_some_and(|upper| value > upper)
}

pub(crate) fn stable_action_suffix(loop_id: &ControlLoopId, source: &ControlTrendPoint) -> String {
    format!(
        "{}-{}-{}-{:02}",
        loop_id.as_str().replace('_', "-"),
        source.source.lot_id,
        source.source.wafer_id,
        source.source.run_index
    )
}

pub(crate) fn format_signed(value: f64) -> String {
    if value.abs() >= 10.0 {
        format!("{value:+.1}")
    } else {
        format!("{value:+.2}")
    }
}

pub(crate) fn round_to(value: f64, precision: u32) -> f64 {
    let scale = 10_f64.powi(precision as i32);
    (value * scale).round() / scale
}
