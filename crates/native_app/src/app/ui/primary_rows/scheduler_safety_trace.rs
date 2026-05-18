#![allow(unused_imports)]
use super::*;

pub(crate) fn scheduler_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let dispatch = app.workspace.scheduler.dispatch(app.scheduler_policy);
    let mut rows = dispatch
        .assignments
        .iter()
        .filter(|assignment| {
            !app.scheduler_focus_selected_tool
                || app.selected_scheduler_tool.as_ref() == Some(&assignment.tool_id)
        })
        .take(10)
        .map(|assignment| {
            PrimaryRow::new(
                format!("dispatch-{}-{}", assignment.lot_id, assignment.tool_id),
                assignment.lot_id.to_string(),
                assignment.tool_id.to_string(),
                format!(
                    "P{} start {} finish {}; tardy {} min",
                    assignment.priority,
                    assignment.start_minute,
                    assignment.finish_minute,
                    assignment.tardy_minutes
                ),
            )
            .action(
                format!("glassworks.viewctl.scheduler.tool.{}", assignment.tool_id),
                app.selected_scheduler_tool.as_ref() == Some(&assignment.tool_id),
            )
        })
        .collect::<Vec<_>>();
    if rows.len() < 10 {
        rows.extend(
            dispatch
                .recommendations
                .iter()
                .take(10 - rows.len())
                .map(|recommendation| {
                    let lot = recommendation
                        .lot_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "idle".to_string());
                    PrimaryRow::new(
                        format!("recommend-{}", recommendation.tool_id),
                        format!("Recommend {}", recommendation.tool_id),
                        lot,
                        recommendation.reason.clone(),
                    )
                    .action(
                        format!("glassworks.viewctl.scheduler.tool.{}", recommendation.tool_id),
                        app.selected_scheduler_tool.as_ref() == Some(&recommendation.tool_id),
                    )
                }),
        );
    }
    rows
}

pub(crate) fn safety_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let safety = &app.workspace.safety;
    let lockouts = safety.evaluate_lockouts();
    let mut rows = safety
        .active_conditions()
        .into_iter()
        .take(3)
        .map(|sensor| {
            let sensor_id = sensor.id.to_string();
            PrimaryRow::new(
                format!("condition-{sensor_id}"),
                format!("{} {}", sensor.severity.label(), sensor.name),
                format!("{} {}", sensor.state.label(), sensor.domain.label()),
                format!(
                    "{} {}; limit {}; routed {}",
                    spc_compact_number(sensor.value),
                    sensor.unit,
                    safety_limit_label(&sensor.limit),
                    safety
                        .route_targets_for(sensor)
                        .into_iter()
                        .map(|target| target.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .action(
                format!("glassworks.viewctl.safety.ack.condition.{sensor_id}"),
                app.acknowledged_conditions.contains(&sensor_id),
            )
        })
        .collect::<Vec<_>>();

    rows.extend(
        lockouts
            .iter()
            .take(3)
            .map(|lockout| {
                PrimaryRow::new(
                    format!("lockout-{}", lockout.tool_id),
                    safety_tool_label(&lockout.tool_id, &lockout.tool_name),
                    if lockout.locked_out {
                        "Locked out"
                    } else {
                        "Clear"
                    },
                    if lockout.reasons.is_empty() {
                        "All interlocks clear".to_string()
                    } else {
                        lockout.reasons.join("; ")
                    },
                )
                .action(
                    format!("glassworks.viewctl.safety.tool.{}", lockout.tool_id),
                    app.selected_safety_tool.as_deref() == Some(lockout.tool_id.as_str()),
                )
            })
            .take(10usize.saturating_sub(rows.len())),
    );

    if let Some(lockout) = selected_safety_lockout(app) {
        rows.extend(
            safety_selected_tool_rows(app, &lockout)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }

    rows.extend(
        safety
            .alarm_routes
            .iter()
            .take(10usize.saturating_sub(rows.len()).min(2))
            .map(|route| {
                PrimaryRow::new(
                    format!(
                        "route-{}-{}-{}",
                        route.domain.label(),
                        route.minimum_severity.label(),
                        route.target.label()
                    ),
                    format!(
                        "{} {}",
                        route.domain.label(),
                        route.minimum_severity.label()
                    ),
                    route.target.label(),
                    format!("channel {}", route.channel),
                )
            }),
    );

    if rows.len() < 10 {
        rows.extend(
            safety
                .incidents
                .iter()
                .take(10 - rows.len())
                .map(|incident| {
                    let incident_id = incident.id.to_string();
                    PrimaryRow::new(
                        format!("incident-{incident_id}"),
                        display_safety_incident_identifier(&incident.id),
                        format!("{} {}", incident.severity.label(), incident.status.label()),
                        format!("{}; {}", incident.domain.label(), incident.summary),
                    )
                    .action(
                        format!("glassworks.viewctl.safety.ack.incident.{incident_id}"),
                        app.acknowledged_incidents.contains(&incident_id),
                    )
                }),
        );
    }
    if rows.len() < 10 {
        rows.extend(
            safety
                .audit_events
                .iter()
                .take(10 - rows.len())
                .map(|event| {
                    PrimaryRow::new(
                        format!("audit-{}", event.sequence),
                        format!("#{} {}", event.sequence, event.kind.label()),
                        event.actor.clone(),
                        format!("{}; {}", event.timestamp, event.message),
                    )
                }),
        );
    }
    rows
}

pub(crate) fn safety_selected_tool_rows(
    app: &GlassworksApp,
    lockout: &layout_model::safety::ToolLockout,
) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            format!("selected-lockout-{}", lockout.tool_id),
            format!("Selected {}", lockout.tool_name),
            if lockout.locked_out {
                "blocked"
            } else {
                "clear"
            },
            if lockout.reasons.is_empty() {
                "All interlocks clear".to_string()
            } else {
                lockout.reasons.join("; ")
            },
        )
        .action(
            format!("glassworks.viewctl.safety.ack.lockout.{}", lockout.tool_id),
            app.acknowledged_lockouts.contains(&lockout.tool_id),
        ),
    ];
    let Some(interlock) = app
        .workspace
        .safety
        .tool_interlocks
        .iter()
        .find(|interlock| interlock.tool_id == lockout.tool_id)
    else {
        return rows;
    };
    for sensor_id in &interlock.required_sensors {
        let Some(sensor) = app
            .workspace
            .safety
            .sensors
            .iter()
            .find(|sensor| &sensor.id == sensor_id)
        else {
            continue;
        };
        rows.push(PrimaryRow::new(
            format!("permissive-{}-{}", lockout.tool_id, sensor.id),
            format!(
                "{} {}",
                if sensor.fails_interlock() {
                    "blocked"
                } else {
                    "pass"
                },
                sensor.name
            ),
            sensor.domain.label(),
            format!(
                "{} {} / state {} / limit {} / last {}s",
                spc_compact_number(sensor.value),
                sensor.unit,
                sensor.state.label(),
                safety_limit_label(&sensor.limit),
                sensor.last_seen_s
            ),
        ));
    }
    rows
}

pub(crate) fn safety_limit_label(limit: &layout_model::safety::SafetyLimit) -> String {
    match (limit.lower, limit.upper) {
        (Some(lower), Some(upper)) => {
            format!(
                "{}..{}",
                spc_compact_number(lower),
                spc_compact_number(upper)
            )
        }
        (Some(lower), None) => format!(">= {}", spc_compact_number(lower)),
        (None, Some(upper)) => format!("<= {}", spc_compact_number(upper)),
        (None, None) => "none".to_string(),
    }
}

pub(crate) fn traceability_lot_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let genealogy = &app.workspace.genealogy;
    genealogy
        .lot_ids()
        .into_iter()
        .take(10)
        .map(|lot_id| {
            let wafer_count = genealogy.wafer_refs_for_lot(&lot_id).len();
            PrimaryRow::new(
                format!("trace-lot-{lot_id}"),
                trace_lot_label(genealogy, &lot_id, 32),
                format!("{wafer_count} wafers"),
                if app.selected_trace_lot.as_ref() == Some(&lot_id) {
                    "selected lot".to_string()
                } else {
                    "available lot".to_string()
                },
            )
            .action(
                format!("glassworks.viewctl.trace.lot.{lot_id}"),
                app.selected_trace_lot.as_ref() == Some(&lot_id),
            )
        })
        .collect()
}

pub(crate) fn traceability_wafer_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let Some(lot_id) = app
        .selected_trace_lot
        .clone()
        .or_else(|| app.workspace.genealogy.lot_ids().into_iter().next())
    else {
        return Vec::new();
    };
    let related_wafers = selected_trace_wafer_ref(app)
        .map(|wafer| {
            app.workspace
                .genealogy
                .wafer_lineage(&wafer)
                .into_iter()
                .chain(app.workspace.genealogy.wafer_descendants(&wafer))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    app.workspace
        .genealogy
        .wafer_refs_for_lot(&lot_id)
        .into_iter()
        .filter(|wafer| {
            !app.trace_related_only || related_wafers.is_empty() || related_wafers.contains(wafer)
        })
        .take(10)
        .map(|wafer| {
            let selected = app.selected_trace_lot.as_ref() == Some(&wafer.lot_id)
                && app.selected_trace_wafer.as_ref() == Some(&wafer.wafer_id);
            PrimaryRow::new(
                format!("trace-wafer-{}-{}", wafer.lot_id, wafer.wafer_id),
                trace_wafer_label(&wafer),
                trace_wafer_context_label(&app.workspace.genealogy, &wafer),
                trace_latest_step_label(app, &wafer),
            )
            .action(
                format!(
                    "glassworks.viewctl.trace.wafer.{}|{}",
                    wafer.lot_id, wafer.wafer_id
                ),
                selected,
            )
        })
        .collect()
}

pub(crate) fn traceability_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let Some(lot_id) = app
        .selected_trace_lot
        .clone()
        .or_else(|| app.workspace.genealogy.lot_ids().into_iter().next())
    else {
        return Vec::new();
    };
    let related_wafers = selected_trace_wafer_ref(app)
        .map(|wafer| {
            app.workspace
                .genealogy
                .wafer_lineage(&wafer)
                .into_iter()
                .chain(app.workspace.genealogy.wafer_descendants(&wafer))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    app.workspace
        .genealogy
        .wafer_refs_for_lot(&lot_id)
        .into_iter()
        .filter(|wafer| {
            !app.trace_related_only || related_wafers.is_empty() || related_wafers.contains(wafer)
        })
        .take(4)
        .map(|wafer| trace_wafer_primary_row(app, wafer))
        .chain(trace_selected_wafer_rows(app).into_iter().take(6))
        .take(10)
        .collect()
}

pub(crate) fn trace_wafer_primary_row(app: &GlassworksApp, wafer: WaferRef) -> PrimaryRow {
    let lineage = app.workspace.genealogy.wafer_lineage(&wafer).len();
    let descendants = app.workspace.genealogy.wafer_descendants(&wafer).len();
    let slot = app
        .workspace
        .genealogy
        .wafer(&wafer)
        .map(|wafer| format!("slot {}", wafer.slot))
        .unwrap_or_else(|| "missing wafer".to_string());
    PrimaryRow::new(
        format!("trace-{}-{}", wafer.lot_id, wafer.wafer_id),
        trace_wafer_label(&wafer),
        format!(
            "{} {slot}",
            trace_lot_label(&app.workspace.genealogy, &wafer.lot_id, 24)
        ),
        format!("lineage {lineage}; descendants {descendants}"),
    )
    .action(
        trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
        app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
    )
}

pub(crate) fn trace_selected_wafer_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let Some(selected_wafer) = selected_trace_wafer_ref(app) else {
        return Vec::new();
    };
    let genealogy = &app.workspace.genealogy;
    let mut rows = Vec::new();
    rows.extend(
        genealogy
            .wafer_lineage(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|wafer| {
                PrimaryRow::new(
                    format!("trace-lineage-{}-{}", wafer.lot_id, wafer.wafer_id),
                    if wafer == selected_wafer {
                        "Selected wafer".to_string()
                    } else {
                        "Parent wafer".to_string()
                    },
                    trace_wafer_context_label(genealogy, &wafer),
                    trace_latest_step_label(app, &wafer),
                )
                .action(
                    trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
                    app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
                )
            }),
    );
    rows.extend(
        genealogy
            .wafer_descendants(&selected_wafer)
            .into_iter()
            .take(2)
            .map(|wafer| {
                PrimaryRow::new(
                    format!("trace-child-{}-{}", wafer.lot_id, wafer.wafer_id),
                    "Child wafer",
                    trace_wafer_context_label(genealogy, &wafer),
                    trace_latest_step_label(app, &wafer),
                )
                .action(
                    trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
                    app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
                )
            }),
    );
    if let Some(query) = trace_impact_query(app) {
        let impact = genealogy.impact_for(query.clone());
        rows.push(PrimaryRow::new(
            "trace-impact-summary",
            app.trace_impact_mode.label(),
            trace_impact_query_label(genealogy, &query),
            format!(
                "{} direct, {} impacted, {} evidence rows",
                impact.direct_wafers.len(),
                impact.impacted_wafers.len(),
                impact.matching_process_records.len() + impact.matching_material_uses.len()
            ),
        ));
        rows.extend(impact.impacted_wafers.into_iter().take(2).map(|impact| {
            PrimaryRow::new(
                format!(
                    "trace-impact-{}-{}",
                    impact.wafer.lot_id, impact.wafer.wafer_id
                ),
                "Impacted wafer",
                trace_wafer_context_label(genealogy, &impact.wafer),
                format!(
                    "{}; latest {}",
                    impact.relationship.label(),
                    impact
                        .latest_step_id
                        .as_ref()
                        .map(|step_id| display_step_identifier(step_id.as_str()))
                        .unwrap_or_else(|| "n/a".to_string())
                ),
            )
            .action(
                trace_detail_action(&TraceSelection::Wafer(impact.wafer.clone())),
                app.selected_trace_detail == Some(TraceSelection::Wafer(impact.wafer)),
            )
        }));
    }
    rows.extend(
        genealogy
            .inherited_process_history_for_wafer(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|record| {
                PrimaryRow::new(
                    format!("trace-process-{}", record.sequence),
                    record.step_name.clone(),
                    equipment_tool_label_for_raw_id(&app.workspace, record.tool_id.as_str()),
                    format!(
                        "{}; {}; {}",
                        display_recipe_identifier(record.recipe_id.as_str()),
                        display_tool_run_identifier(record.tool_run_id.as_str()),
                        record.completed_at
                    ),
                )
                .action(
                    trace_detail_action(&TraceSelection::Process(record.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::Process(record.sequence)),
                )
            }),
    );
    rows.extend(
        genealogy
            .material_ancestry_for_wafer(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|record| {
                let material_name = trace_material_lot_label(genealogy, &record.material_lot_id);
                PrimaryRow::new(
                    format!("trace-material-{}", record.sequence),
                    material_name,
                    format!("{} {}", spc_compact_number(record.quantity), record.unit),
                    format!(
                        "{}; {}",
                        display_step_identifier(record.step_id.as_str()),
                        display_tool_run_identifier(record.tool_run_id.as_str())
                    ),
                )
                .action(
                    trace_detail_action(&TraceSelection::MaterialUse(record.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::MaterialUse(record.sequence)),
                )
            }),
    );
    rows.extend(
        app.workspace
            .genealogy
            .events
            .iter()
            .rev()
            .take(2)
            .map(|event| {
                let (title, detail) = trace_event_labels(genealogy, &event.kind);
                PrimaryRow::new(
                    format!("trace-event-{}", event.sequence),
                    capitalize_ascii_first(title.to_string()),
                    "event",
                    detail,
                )
                .action(
                    trace_detail_action(&TraceSelection::Event(event.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::Event(event.sequence)),
                )
            }),
    );
    rows
}

pub(crate) fn trace_latest_step_label(app: &GlassworksApp, wafer: &WaferRef) -> String {
    app.workspace
        .genealogy
        .inherited_process_history_for_wafer(wafer)
        .last()
        .map(|record| {
            format!(
                "latest {} on {}",
                record.step_name,
                equipment_tool_label_for_raw_id(&app.workspace, record.tool_id.as_str())
            )
        })
        .unwrap_or_else(|| "no process history".to_string())
}

pub(crate) fn trace_wafer_label(wafer: &WaferRef) -> String {
    let lot_prefix = format!("{}-", wafer.lot_id);
    wafer
        .wafer_id
        .as_str()
        .strip_prefix(&lot_prefix)
        .unwrap_or_else(|| wafer.wafer_id.as_str())
        .to_string()
}

pub(crate) fn trace_wafer_context_label(
    genealogy: &layout_model::genealogy::LotGenealogy,
    wafer: &WaferRef,
) -> String {
    format!(
        "{} / {}",
        trace_lot_label(genealogy, &wafer.lot_id, 32),
        trace_wafer_label(wafer)
    )
}

pub(crate) fn trace_material_lot_label(
    genealogy: &layout_model::genealogy::LotGenealogy,
    material_lot_id: &layout_model::genealogy::MaterialLotId,
) -> String {
    genealogy
        .material_lots
        .get(material_lot_id)
        .map(|material| material.name.clone())
        .unwrap_or_else(|| material_lot_id.to_string())
}

pub(crate) fn trace_process_button_label(
    record: &layout_model::genealogy::WaferProcessRecord,
) -> String {
    compact_button_label(&record.step_name, 16)
}

pub(crate) fn trace_material_use_button_label(
    genealogy: &layout_model::genealogy::LotGenealogy,
    record: &layout_model::genealogy::MaterialUse,
) -> String {
    compact_button_label(
        &trace_material_lot_label(genealogy, &record.material_lot_id),
        16,
    )
}

pub(crate) fn trace_event_button_label(
    event: &layout_model::genealogy::GenealogyEvent,
) -> &'static str {
    match &event.kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { .. } => "Lot started",
        layout_model::genealogy::GenealogyEventKind::LotSplit { .. } => "Lot split",
        layout_model::genealogy::GenealogyEventKind::LotMerge { .. } => "Lot merge",
    }
}

pub(crate) fn trace_event_labels(
    genealogy: &layout_model::genealogy::LotGenealogy,
    kind: &layout_model::genealogy::GenealogyEventKind,
) -> (&'static str, String) {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { lot_id } => {
            ("lot started", trace_lot_label(genealogy, lot_id, 42))
        }
        layout_model::genealogy::GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            wafer_count,
            reason,
        } => (
            "lot split",
            format!(
                "{} -> {}; {wafer_count} wafers; {reason}",
                trace_lot_label(genealogy, source_lot_id, 24),
                trace_lot_label(genealogy, target_lot_id, 24)
            ),
        ),
        layout_model::genealogy::GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            wafer_count,
            reason,
        } => (
            "lot merge",
            format!(
                "{} -> {}; {wafer_count} wafers; {reason}",
                source_lot_ids
                    .iter()
                    .map(|lot_id| trace_lot_label(genealogy, lot_id, 24))
                    .collect::<Vec<_>>()
                    .join(", "),
                trace_lot_label(genealogy, target_lot_id, 24)
            ),
        ),
    }
}

pub(crate) fn trace_selected_detail_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let selection = app.selected_trace_detail.clone().or_else(|| {
        default_trace_selection(
            &app.workspace,
            app.selected_trace_lot.as_ref(),
            app.selected_trace_wafer.as_ref(),
        )
    });
    let Some(selection) = selection else {
        return vec![("Selected node".to_string(), "None".to_string())];
    };
    let genealogy = &app.workspace.genealogy;
    match selection {
        TraceSelection::Lot(lot_id) => {
            let Some(lot) = genealogy.lots.get(&lot_id) else {
                return vec![("Selected node".to_string(), format!("Missing lot {lot_id}"))];
            };
            let child_count = genealogy
                .lots
                .values()
                .filter(|candidate| {
                    candidate
                        .created_from
                        .iter()
                        .any(|parent| parent == &lot_id)
                })
                .count();
            let active_wafers = lot
                .wafers
                .values()
                .filter(|wafer| {
                    matches!(
                        wafer.state,
                        layout_model::genealogy::WaferGenealogyState::Active
                    )
                })
                .count();
            vec![
                (
                    "Selection".to_string(),
                    format!("Lot {}", trace_lot_label(genealogy, &lot_id, 42)),
                ),
                ("Product".to_string(), display_product_label(&lot.product)),
                ("Route".to_string(), lot.route_id.to_string()),
                ("Disposition".to_string(), trace_lot_disposition_label(lot)),
                (
                    "Parent / child lots".to_string(),
                    format!("{} / {child_count}", lot.created_from.len()),
                ),
                (
                    "Active / moved".to_string(),
                    format!("{} / {}", active_wafers, lot.wafers.len() - active_wafers),
                ),
            ]
        }
        TraceSelection::Wafer(wafer_ref) => {
            let Some(wafer) = genealogy.wafer(&wafer_ref) else {
                return vec![(
                    "Selection".to_string(),
                    format!("Missing wafer {wafer_ref}"),
                )];
            };
            let process_records = genealogy.inherited_process_history_for_wafer(&wafer_ref);
            let material_records = genealogy.material_ancestry_for_wafer(&wafer_ref);
            vec![
                (
                    "Selection".to_string(),
                    format!("Wafer {}", trace_wafer_context_label(genealogy, &wafer_ref)),
                ),
                (
                    "Slot / substrate".to_string(),
                    format!("{} / {}", wafer.slot, wafer.substrate),
                ),
                (
                    "State".to_string(),
                    trace_wafer_state_label(genealogy, wafer),
                ),
                (
                    "Parent".to_string(),
                    wafer
                        .parent
                        .as_ref()
                        .map(|parent| trace_wafer_context_label(genealogy, parent))
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Lineage / descendants".to_string(),
                    format!(
                        "{} / {}",
                        genealogy.wafer_lineage(&wafer_ref).len(),
                        genealogy.wafer_descendants(&wafer_ref).len()
                    ),
                ),
                (
                    "Process / material".to_string(),
                    format!("{} / {}", process_records.len(), material_records.len()),
                ),
            ]
        }
        TraceSelection::Process(sequence) => {
            let Some(record) = trace_process_record(app, sequence) else {
                return vec![(
                    "Selection".to_string(),
                    format!("Missing process #{sequence}"),
                )];
            };
            let material_uses = genealogy
                .material_uses
                .iter()
                .filter(|material| {
                    material.tool_run_id == record.tool_run_id && material.wafer == record.wafer
                })
                .count();
            vec![
                ("Selection".to_string(), "Process step".to_string()),
                (
                    "Wafer".to_string(),
                    trace_wafer_context_label(genealogy, &record.wafer),
                ),
                ("Step".to_string(), record.step_name.clone()),
                ("Layer".to_string(), trace_process_layer_label(record)),
                (
                    "Recipe / tool".to_string(),
                    format!(
                        "{} / {}",
                        display_recipe_identifier(record.recipe_id.as_str()),
                        equipment_tool_label_for_raw_id(&app.workspace, record.tool_id.as_str())
                    ),
                ),
                (
                    "Run / materials".to_string(),
                    format!(
                        "{} / {}",
                        display_tool_run_identifier(record.tool_run_id.as_str()),
                        material_uses
                    ),
                ),
            ]
        }
        TraceSelection::MaterialUse(sequence) => {
            let Some(record) = trace_material_use(app, sequence) else {
                return vec![(
                    "Selection".to_string(),
                    format!("Missing material #{sequence}"),
                )];
            };
            let material = genealogy.material_lots.get(&record.material_lot_id);
            vec![
                ("Selection".to_string(), "Material use".to_string()),
                (
                    "Wafer".to_string(),
                    trace_wafer_context_label(genealogy, &record.wafer),
                ),
                (
                    "Material lot".to_string(),
                    trace_material_lot_label(genealogy, &record.material_lot_id),
                ),
                (
                    "Material".to_string(),
                    material
                        .map(|material| material.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                (
                    "Supplier / cert".to_string(),
                    material
                        .map(|material| {
                            format!("{} / {}", material.supplier, material.certificate_id)
                        })
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                (
                    "Step / qty".to_string(),
                    format!(
                        "{} / {} {}",
                        display_step_identifier(record.step_id.as_str()),
                        spc_compact_number(record.quantity),
                        record.unit
                    ),
                ),
            ]
        }
        TraceSelection::Event(sequence) => {
            let Some(event) = trace_event(app, sequence) else {
                return vec![(
                    "Selection".to_string(),
                    format!("Missing event #{sequence}"),
                )];
            };
            let (title, detail) = trace_event_labels(genealogy, &event.kind);
            vec![
                (
                    "Selection".to_string(),
                    trace_event_button_label(event).to_string(),
                ),
                ("Operation".to_string(), title.to_string()),
                (
                    "Scope".to_string(),
                    trace_event_scope(genealogy, &event.kind),
                ),
                ("Reason".to_string(), trace_event_reason(&event.kind)),
                ("Audit text".to_string(), detail),
            ]
        }
    }
}

pub(crate) fn trace_process_record(
    app: &GlassworksApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::WaferProcessRecord> {
    app.workspace
        .genealogy
        .process_history
        .iter()
        .find(|record| record.sequence == sequence)
}

pub(crate) fn trace_material_use(
    app: &GlassworksApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::MaterialUse> {
    app.workspace
        .genealogy
        .material_uses
        .iter()
        .find(|record| record.sequence == sequence)
}

pub(crate) fn trace_event(
    app: &GlassworksApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::GenealogyEvent> {
    app.workspace
        .genealogy
        .events
        .iter()
        .find(|event| event.sequence == sequence)
}

pub(crate) fn trace_lot_disposition_label(lot: &layout_model::genealogy::GenealogyLot) -> String {
    match &lot.disposition {
        layout_model::genealogy::LotDisposition::Active => "active".to_string(),
        layout_model::genealogy::LotDisposition::Closed { reason } => {
            format!("closed: {reason}")
        }
    }
}

pub(crate) fn trace_wafer_state_label(
    genealogy: &layout_model::genealogy::LotGenealogy,
    wafer: &layout_model::genealogy::GenealogyWafer,
) -> String {
    match &wafer.state {
        layout_model::genealogy::WaferGenealogyState::Active => "active".to_string(),
        layout_model::genealogy::WaferGenealogyState::SplitTo { lot_id } => {
            format!("split to {}", trace_lot_label(genealogy, lot_id, 32))
        }
        layout_model::genealogy::WaferGenealogyState::MergedTo { lot_id } => {
            format!("merged to {}", trace_lot_label(genealogy, lot_id, 32))
        }
        layout_model::genealogy::WaferGenealogyState::Scrapped { reason } => {
            format!("scrapped: {reason}")
        }
    }
}

pub(crate) fn trace_process_layer_label(
    record: &layout_model::genealogy::WaferProcessRecord,
) -> String {
    record
        .process_layer
        .map(ProcessLayer::as_technology_name)
        .unwrap_or("n/a")
        .to_string()
}

pub(crate) fn trace_event_scope(
    genealogy: &layout_model::genealogy::LotGenealogy,
    kind: &layout_model::genealogy::GenealogyEventKind,
) -> String {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { lot_id } => {
            trace_lot_label(genealogy, lot_id, 42)
        }
        layout_model::genealogy::GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            ..
        } => format!(
            "{} -> {}",
            trace_lot_label(genealogy, source_lot_id, 24),
            trace_lot_label(genealogy, target_lot_id, 24)
        ),
        layout_model::genealogy::GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            ..
        } => format!(
            "{} -> {}",
            source_lot_ids
                .iter()
                .map(|lot_id| trace_lot_label(genealogy, lot_id, 24))
                .collect::<Vec<_>>()
                .join(", "),
            trace_lot_label(genealogy, target_lot_id, 24)
        ),
    }
}

pub(crate) fn trace_event_reason(kind: &layout_model::genealogy::GenealogyEventKind) -> String {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { .. } => "initial lot".to_string(),
        layout_model::genealogy::GenealogyEventKind::LotSplit { reason, .. }
        | layout_model::genealogy::GenealogyEventKind::LotMerge { reason, .. } => reason.clone(),
    }
}
