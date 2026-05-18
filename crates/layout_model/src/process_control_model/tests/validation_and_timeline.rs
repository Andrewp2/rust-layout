#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn synthetic_model_links_yield_measurements_to_control_actions() {
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
    assert!(!model.audit_for_loop(&edge_loop).is_empty());
}

#[test]
pub(crate) fn synthetic_process_control_model_validates() {
    let model = ProcessControlModel::from_yield_analysis(&YieldAnalysis::synthetic());

    assert_eq!(model.validate(), Vec::new());
}

#[test]
pub(crate) fn synthetic_process_control_model_validates_against_yield_context() {
    let analysis = YieldAnalysis::synthetic();
    let model = ProcessControlModel::from_yield_analysis(&analysis);
    let context = ProcessControlValidationContext::from_yield_analysis(&analysis);

    assert_eq!(model.validate_with_context(&context), Vec::new());
}

#[test]
pub(crate) fn validation_context_rejects_missing_measurements_recipes_and_sources() {
    let analysis = YieldAnalysis::synthetic();
    let context = ProcessControlValidationContext::from_yield_analysis(&analysis);
    let mut model = ProcessControlModel::from_yield_analysis(&analysis);
    model.loops[0].recipe = RecipeBinding::new("MISSING_RECIPE", 1);
    let loop_id = model.loops[0].id.clone();
    model.trends.get_mut(&loop_id).unwrap()[0].measurement_id = "MISSING_MEASUREMENT".to_string();
    model.trends.get_mut(&loop_id).unwrap()[1].recipe = RecipeBinding::new("POLY_ETCH_004", 99);
    model.actions[0].target_recipe = RecipeBinding::new("POLY_ETCH_004", 99);
    model.actions[0].source.tool_run_id = "MISSING_RUN".to_string();

    let findings = model.validate_with_context(&context);

    assert!(
        findings.iter().any(|finding| {
            finding
                .message
                .contains("loop R2R-POLY-EDGE-OX references missing recipe binding")
        }),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| {
            finding
                .message
                .contains("trend references missing measurement MISSING_MEASUREMENT")
        }),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| {
            finding
                .message
                .contains("trend M-L-00042-02-EDGE-OX references missing recipe binding")
        }),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| {
            finding.message.contains("action PCA-R2R-POLY-EDGE-OX")
                && finding
                    .message
                    .contains("missing target recipe binding POLY_ETCH_004 v99")
        }),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| {
            finding.message.contains("references missing source run")
                && finding.message.contains("MISSING_RUN")
        }),
        "{findings:?}"
    );
}

#[test]
pub(crate) fn recommendation_moves_recipe_parameter_against_process_error() {
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
pub(crate) fn parameter_adjustment_respects_delta_and_recipe_bounds() {
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
pub(crate) fn action_state_changes_are_audited_and_guarded() {
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

#[test]
pub(crate) fn validation_rejects_broken_loops_trends_actions_and_audits() {
    let mut model = ProcessControlModel::from_yield_analysis(&YieldAnalysis::synthetic());
    model.loops[0].output.lower_spec = Some(10.0);
    model.loops[0].output.upper_spec = Some(1.0);
    model.loops[0].manipulated_parameters[0].current_value = f64::NAN;
    let loop_id = model.loops[0].id.clone();
    model.trends.get_mut(&loop_id).unwrap()[0].yield_fraction = Some(1.5);
    model.actions[0].confidence = 1.5;
    model.actions[0].adjustments[0].parameter_key = "missing-parameter".to_string();
    let duplicate_adjustment = model.actions[0].adjustments[0].clone();
    model.actions[0].adjustments.push(duplicate_adjustment);
    model.audit_events.push(ControlAuditEvent {
        sequence: 999,
        action_id: Some(ControlActionId::new("MISSING_ACTION")),
        loop_id: ControlLoopId::from("MISSING_LOOP"),
        actor: String::new(),
        timestamp: String::new(),
        kind: ControlAuditKind::Applied,
        from_state: Some(ControlActionState::Rejected),
        to_state: Some(ControlActionState::Applied),
        note: String::new(),
    });

    let findings = model.validate();

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("lower spec is above upper spec")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("non-finite current value")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("invalid yield fraction")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("confidence outside 0..1")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("missing parameter")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("duplicate adjustment parameter")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("missing action")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("invalid transition")),
        "{findings:?}"
    );
}

#[test]
pub(crate) fn validation_rejects_stale_action_and_audit_timeline() {
    let mut model = ProcessControlModel::from_yield_analysis(&YieldAnalysis::synthetic());
    let action_id = model.actions[0].id.clone();
    let loop_id = model.actions[0].loop_id.clone();
    let mut duplicate_source_action = model.actions[0].clone();
    duplicate_source_action.id = ControlActionId::new("PCA-DUPLICATE-SOURCE");

    model.actions[0].state = ControlActionState::Approved;
    model.actions[0].proposed_at = "2026-02-30T14:00:00Z".to_string();
    model.actions[0].measured_value += 1.0;
    model.actions.push(duplicate_source_action);
    model.audit_events.push(ControlAuditEvent {
        sequence: 0,
        action_id: Some(action_id),
        loop_id,
        actor: "process.eng".to_string(),
        timestamp: "2026-05-06T25:00:00Z".to_string(),
        kind: ControlAuditKind::Applied,
        from_state: None,
        to_state: Some(ControlActionState::Approved),
        note: String::new(),
    });

    let findings = model.validate();

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("invalid proposal timestamp")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("measured value does not match")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("duplicates an existing action")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("audit sequence 0 is invalid")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("invalid timestamp")),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| finding
            .message
            .contains("kind applied records to_state approved")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("has no from_state")),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|finding| {
            finding
                .message
                .contains("state is approved, but latest audit records proposed")
        }),
        "{findings:?}"
    );
}
