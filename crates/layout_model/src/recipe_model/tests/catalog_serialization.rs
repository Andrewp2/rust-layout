#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn validates_parameter_ranges_types_and_unknowns() {
    let catalog = sample_recipe_catalog();
    let recipe = catalog
        .recipe(&RecipeId::from("ETCH_CF4_POLY_001"))
        .expect("etch recipe");
    let mut version = recipe.version(RecipeVersionNumber(1)).unwrap().clone();
    assert!(recipe.validate_version(&version).is_empty());

    version
        .parameters
        .insert("rf_power_w".to_string(), RecipeParameterValue::Integer(900));
    version.parameters.insert(
        "gas_recipe".to_string(),
        RecipeParameterValue::Choice("argon".to_string()),
    );
    version.parameters.insert(
        "operator_note".to_string(),
        RecipeParameterValue::Text("split".to_string()),
    );

    let issues = recipe.validate_version(&version);
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Error)
            .count(),
        2
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue.parameter_key.as_deref() == Some("rf_power_w"))
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue.parameter_key.as_deref() == Some("gas_recipe"))
    );
    assert!(issues.iter().any(|issue| {
        issue.severity == ValidationSeverity::Warning
            && issue.parameter_key.as_deref() == Some("operator_note")
    }));
}

#[test]
pub(crate) fn sample_recipe_catalog_validates() {
    let catalog = sample_recipe_catalog();
    let issues = catalog.validate();
    assert!(
        !issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error),
        "{issues:?}"
    );
}

#[test]
pub(crate) fn recipe_catalog_validation_rejects_bad_catalog_references() {
    let mut catalog = sample_recipe_catalog();
    let spin_id = RecipeId::from("SPIN_PR_3000");
    let first_spin_version = catalog.recipes[&spin_id].versions[0].clone();
    catalog
        .recipes
        .get_mut(&spin_id)
        .unwrap()
        .versions
        .push(first_spin_version);
    catalog
        .process_routes
        .push(catalog.process_routes[0].clone());
    catalog.process_routes[0].steps[0].recipe = Some(RecipeBinding::new("MISSING_RECIPE", 1));
    catalog.process_routes[0].steps[1].sequence = 0;
    let mut duplicate_run = catalog.tool_runs[0].clone();
    duplicate_run.recipe = RecipeBinding::new("SPIN_PR_3000", 99);
    duplicate_run.process_step.as_mut().unwrap().step_id = ProcessStepId::from("missing_step");
    catalog.tool_runs.push(duplicate_run);
    let mut stale_run = catalog.tool_runs[1].clone();
    stale_run.id = ToolRunId::from("RUN-STALE-META");
    stale_run.tool_class = ToolClass::SpinCoater;
    stale_run.started_at = "2026-04-22T11:05:00Z".to_string();
    stale_run.completed_at = Some("2026-04-22T11:00:00Z".to_string());
    catalog.tool_runs.push(stale_run);
    let mut queued_run = catalog.tool_runs[1].clone();
    queued_run.id = ToolRunId::from("RUN-QUEUED-META");
    queued_run.state = ToolRunState::Queued;
    queued_run.completed_at = Some("2026-04-22T11:03:00Z".to_string());
    catalog.tool_runs.push(queued_run);
    let mut zero_version_run = catalog.tool_runs[1].clone();
    zero_version_run.id = ToolRunId::from("RUN-ZERO-STEP-VERSION");
    zero_version_run
        .process_step
        .as_mut()
        .unwrap()
        .route_version = 0;
    catalog.tool_runs.push(zero_version_run);

    let issues = catalog.validate();
    let messages = issues
        .iter()
        .map(|issue| issue.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("recipe SPIN_PR_3000 has duplicate version v1"))
    );
    assert!(messages.iter().any(|message| {
        message.contains("process route ROUTE_POLY_GATE_DEMO v1 is duplicated")
    }));
    assert!(messages.iter().any(|message| {
        message.contains("process route ROUTE_POLY_GATE_DEMO v1 step soft_bake has sequence 0")
    }));
    assert!(
            messages.iter().any(|message| message.contains(
                "process route ROUTE_POLY_GATE_DEMO v1 step coat_photoresist references missing recipe MISSING_RECIPE"
            ))
        );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("tool run RUN-SPIN-0007 is duplicated"))
    );
    assert!(
        messages.iter().any(|message| message
            .contains("tool run RUN-SPIN-0007 references missing recipe binding SPIN_PR_3000 v99"))
    );
    assert!(
            messages.iter().any(|message| message.contains(
                "tool run RUN-SPIN-0007 references missing process step ROUTE_POLY_GATE_DEMO v1 missing_step"
            ))
        );
    assert!(messages.iter().any(|message| message.contains(
            "tool run RUN-STALE-META class Spin coater does not match process step ROUTE_POLY_GATE_DEMO v1 etch_poly class Plasma etcher"
        )));
    assert!(messages.iter().any(|message| message.contains(
        "tool run RUN-STALE-META completed_at timestamp is before started_at timestamp"
    )));
    assert!(messages.iter().any(|message| {
        message.contains("queued tool run RUN-QUEUED-META already has started_at timestamp")
    }));
    assert!(
        messages.iter().any(|message| message
            .contains("non-completed tool run RUN-QUEUED-META already has completed_at timestamp"))
    );
    assert!(messages.iter().any(|message| {
        message.contains("tool run RUN-ZERO-STEP-VERSION references process route version 0")
    }));
}

#[test]
pub(crate) fn diffs_versions_as_readable_parameter_changes() {
    let catalog = sample_recipe_catalog();
    let recipe = catalog
        .recipe(&RecipeId::from("SPIN_PR_3000"))
        .expect("spin recipe");
    let from = recipe.version(RecipeVersionNumber(1)).unwrap();
    let to = recipe.version(RecipeVersionNumber(2)).unwrap();

    let diff = diff_recipe_versions(recipe, from, to);
    assert!(diff.entries.iter().any(|entry| {
        entry.parameter_key.as_deref() == Some("spin_rpm")
            && entry.before.as_deref() == Some("4000 rpm")
            && entry.after.as_deref() == Some("4200 rpm")
    }));
    assert!(diff.entries.iter().any(|entry| {
        entry.kind == RecipeDiffKind::ApprovalChanged
            && entry.before.as_deref() == Some("Approved")
            && entry.after.as_deref() == Some("In review")
    }));
}

#[test]
pub(crate) fn approval_transitions_require_review_and_clean_validation() {
    let catalog = sample_recipe_catalog();
    let recipe = catalog
        .recipe(&RecipeId::from("LITHO_POLY_EXPOSE_001"))
        .expect("lithography recipe");
    let mut version = recipe.version(RecipeVersionNumber(2)).unwrap().clone();

    let valid_issues = recipe.validate_version(&version);
    let direct_approve = version.apply_approval_action(
        ApprovalAction::Approve,
        "qa.eng",
        "2026-04-22T12:00:00Z",
        "",
        &valid_issues,
    );
    assert!(matches!(
        direct_approve,
        Err(ApprovalTransitionError::InvalidTransition {
            from: ApprovalState::Draft,
            action: ApprovalAction::Approve
        })
    ));

    version
        .apply_approval_action(
            ApprovalAction::SubmitForReview,
            "litho.dev",
            "2026-04-22T12:05:00Z",
            "",
            &valid_issues,
        )
        .unwrap();
    assert_eq!(version.approval_state, ApprovalState::InReview);

    version.parameters.insert(
        "exposure_dose_mj_cm2".to_string(),
        RecipeParameterValue::Decimal(1000.0),
    );
    let invalid_issues = recipe.validate_version(&version);
    let blocked = version.apply_approval_action(
        ApprovalAction::Approve,
        "qa.eng",
        "2026-04-22T12:10:00Z",
        "",
        &invalid_issues,
    );
    assert!(matches!(
        blocked,
        Err(ApprovalTransitionError::ValidationFailed { error_count: 1 })
    ));

    version.parameters.insert(
        "exposure_dose_mj_cm2".to_string(),
        RecipeParameterValue::Decimal(90.0),
    );
    let fixed_issues = recipe.validate_version(&version);
    version
        .apply_approval_action(
            ApprovalAction::Approve,
            "qa.eng",
            "2026-04-22T12:20:00Z",
            "Split approved.",
            &fixed_issues,
        )
        .unwrap();
    assert_eq!(version.approval_state, ApprovalState::Approved);
    assert_eq!(version.approved_by.as_deref(), Some("qa.eng"));
}

#[test]
pub(crate) fn serializes_schema_friendly_catalog_and_route_recipe_refs() {
    let catalog = sample_recipe_catalog();
    let json = serde_json::to_value(&catalog).expect("serialize catalog");

    let spin_params = &json["recipes"]["SPIN_PR_3000"]["versions"][0]["parameters"];
    assert_eq!(spin_params["spin_rpm"]["type"], "integer");
    assert_eq!(spin_params["spin_rpm"]["value"], 4000);
    assert_eq!(
        json["process_routes"][0]["steps"][0]["recipe"]["recipe_id"],
        "SPIN_PR_3000"
    );
    assert_eq!(
        json["process_routes"][0]["steps"][0]["recipe"]["version"],
        1
    );

    let round_tripped: RecipeCatalog = serde_json::from_value(json).expect("round-trip catalog");
    let route = &round_tripped.process_routes[0];
    let step = route
        .step(&ProcessStepId::from("etch_poly"))
        .expect("etch step");
    assert_eq!(
        step.recipe.as_ref(),
        Some(&RecipeBinding::new("ETCH_CF4_POLY_001", 1))
    );
}
