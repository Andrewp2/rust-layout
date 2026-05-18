#![allow(unused_imports)]
use super::*;

pub(crate) fn validate_tool(
    tool: &Tool,
    now_s: u64,
    findings: &mut Vec<EquipmentValidationFinding>,
) {
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

pub(crate) fn validate_recipe_key(
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

pub(crate) fn validate_recipe_selection(
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

pub(crate) fn validate_run(
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

pub(crate) fn validate_tool_context(
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

pub(crate) fn validate_recipe_selection_context(
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

pub(crate) fn validate_selection_recipe_context(
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

pub(crate) fn log_demo_command(
    result: Result<Vec<EquipmentEvent>, ToolTransitionError>,
    description: &'static str,
) {
    if let Err(err) = result {
        warn!(error = %err, "demo equipment command failed: {description}");
    }
}

pub(crate) fn process_step_for_kind(kind: ToolKind) -> Option<&'static str> {
    match kind {
        ToolKind::SpinCoater => Some("S010-COAT"),
        ToolKind::HotPlate => None,
        ToolKind::MaskAligner => Some("S020-EXPOSE"),
        ToolKind::Etcher => Some("S040-ETCH"),
        ToolKind::Microscope => Some("S050-CD-METRO"),
        ToolKind::ProbeStation => None,
    }
}

pub(crate) fn wave(at_s: u64, seed: u64) -> f64 {
    let phase = ((at_s + seed * 11) % 31) as f64 / 31.0;
    (phase * std::f64::consts::TAU).sin()
}

pub(crate) fn spin_coater_recipes() -> Vec<Recipe> {
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

pub(crate) fn hot_plate_recipes() -> Vec<Recipe> {
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

pub(crate) fn mask_aligner_recipes() -> Vec<Recipe> {
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

pub(crate) fn etcher_recipes() -> Vec<Recipe> {
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

pub(crate) fn microscope_recipes() -> Vec<Recipe> {
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

pub(crate) fn probe_station_recipes() -> Vec<Recipe> {
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

pub(crate) fn recipe<const N: usize>(
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
