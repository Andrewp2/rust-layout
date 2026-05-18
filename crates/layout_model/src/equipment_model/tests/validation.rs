#![allow(unused_imports)]
use super::*;
use crate::*;

pub(crate) fn first_selection(tool: &SyntheticTool) -> RecipeSelection {
    let recipe = tool.tool.available_recipes.values().next().unwrap();
    RecipeSelection::new(recipe.id.clone(), recipe.version)
}

#[test]
pub(crate) fn offline_to_completed_flow() {
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
pub(crate) fn running_tool_can_enter_alarm_and_be_cleared() {
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
pub(crate) fn maintenance_path_rejects_running_tools() {
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
pub(crate) fn invalid_transitions_are_rejected() {
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
pub(crate) fn demo_equipment_simulator_validates() {
    let simulator = EquipmentSimulator::demo_fab();

    assert_eq!(simulator.validate(), Vec::new());
}

pub(crate) fn workspace_context() -> EquipmentValidationContext {
    EquipmentValidationContext::from_mes_and_recipe_catalog(
        &crate::mes::FabMesData::sample(),
        &crate::recipe::RecipeCatalog::sample(),
    )
}

#[test]
pub(crate) fn demo_equipment_simulator_validates_against_workspace_context() {
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
pub(crate) fn validation_context_rejects_missing_mes_and_recipe_catalog_references() {
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
pub(crate) fn validation_rejects_broken_tool_recipe_run_alarm_and_sensor_state() {
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
