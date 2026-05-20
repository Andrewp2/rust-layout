#![allow(unused_imports)]
use super::*;

pub(crate) fn environment_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let ordered_sensors = ordered_environment_sensors(app);
    for chunk in ordered_sensors.iter().take(8).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|sensor| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.environment.sensor.{}", sensor.id),
                        environment_sensor_button_label(sensor),
                        app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
                    )
                })
                .collect(),
        );
    }
    let mut zones = app
        .workspace
        .environment
        .sensors
        .iter()
        .map(|sensor| sensor.zone.clone())
        .collect::<Vec<_>>();
    zones.sort();
    zones.dedup();
    for chunk in zones.iter().take(4).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|zone| {
                    let selected = selected_environment_sensor(app)
                        .is_some_and(|sensor| sensor.zone == **zone);
                    ViewControlButton::new(
                        format!("glassworks.viewctl.environment.zone.{zone}"),
                        zone.to_string(),
                        selected,
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn inventory_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in InventoryQuickFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.inventory.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            inventory_filter_count(&app.workspace, *filter)
                        ),
                        app.inventory_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    for chunk in app
        .workspace
        .inventory
        .sorted_lots()
        .into_iter()
        .filter(|lot| inventory_filter_matches(app.inventory_filter, lot))
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.inventory.lot.{}", lot.id),
                        inventory_lot_button_label(lot, 18),
                        app.selected_inventory_lot.as_ref() == Some(&lot.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn scheduler_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    rows.push(
        [
            DispatchPolicy::PriorityThenFifo,
            DispatchPolicy::DueDateThenPriority,
        ]
        .iter()
        .map(|policy| {
            ViewControlButton::new(
                format!(
                    "glassworks.viewctl.scheduler.policy.{}",
                    dispatch_policy_slug(*policy)
                ),
                display_dispatch_policy_label(*policy),
                app.scheduler_policy == *policy,
            )
        })
        .collect(),
    );
    rows.push(vec![ViewControlButton::new(
        format!(
            "glassworks.viewctl.scheduler.policy.{}",
            dispatch_policy_slug(DispatchPolicy::Fifo)
        ),
        display_dispatch_policy_label(DispatchPolicy::Fifo),
        app.scheduler_policy == DispatchPolicy::Fifo,
    )]);
    for chunk in [0_u8, 1, 2, 3, 4, 5].chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|priority| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.scheduler.priority.{priority}"),
                        format!("P{priority}+"),
                        app.scheduler_min_priority == *priority,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.scheduler.toggle_conflicts",
            "Conflicts",
            app.scheduler_conflicts_only,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.scheduler.toggle_focus_tool",
            "Tool",
            app.scheduler_focus_selected_tool,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.scheduler.reset",
        "Reset",
        false,
    )]);
    for chunk in app
        .workspace
        .scheduler
        .tools
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.scheduler.tool.{}", tool.id),
                        tool.id.to_string(),
                        app.selected_scheduler_tool.as_ref() == Some(&tool.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn safety_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 4 };
    let lockouts = app.workspace.safety.evaluate_lockouts();
    for chunk in lockouts
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|lockout| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.safety.tool.{}", lockout.tool_id),
                        compact_button_label(
                            &safety_tool_label(&lockout.tool_id, &lockout.tool_name),
                            if compact_rows { 16 } else { 18 },
                        ),
                        app.selected_safety_tool.as_deref() == Some(lockout.tool_id.as_str()),
                    )
                })
                .collect(),
        );
    }
    for chunk in app
        .workspace
        .safety
        .active_conditions()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|sensor| {
                    let sensor_id = sensor.id.to_string();
                    ViewControlButton::new(
                        format!("glassworks.viewctl.safety.ack.condition.{sensor_id}"),
                        safety_ack_condition_label(sensor),
                        app.acknowledged_conditions.contains(&sensor_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(lockout) = selected_safety_lockout(app) {
        rows.push(vec![ViewControlButton::new(
            format!("glassworks.viewctl.safety.ack.lockout.{}", lockout.tool_id),
            format!(
                "Ack {}",
                compact_button_label(&safety_tool_label(&lockout.tool_id, &lockout.tool_name), 14)
            ),
            app.acknowledged_lockouts.contains(&lockout.tool_id),
        )]);
    }
    for chunk in app
        .workspace
        .safety
        .incidents
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|incident| {
                    let id = incident.id.to_string();
                    ViewControlButton::new(
                        format!("glassworks.viewctl.safety.ack.incident.{id}"),
                        safety_incident_action_label(incident),
                        app.acknowledged_incidents.contains(&id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn traceability_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 4 };
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
    for chunk in app
        .workspace
        .genealogy
        .lot_ids()
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.trace.lot.{lot_id}"),
                        trace_lot_button_label(&app.workspace.genealogy, lot_id, compact_rows),
                        app.selected_trace_lot.as_ref() == Some(lot_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(lot_id) = app.selected_trace_lot.as_ref() {
        for chunk in app
            .workspace
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .into_iter()
            .filter(|wafer| {
                !app.trace_related_only
                    || related_wafers.is_empty()
                    || related_wafers.contains(wafer)
            })
            .take(6)
            .collect::<Vec<_>>()
            .chunks(chunk_size)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|wafer| {
                        ViewControlButton::new(
                            format!(
                                "glassworks.viewctl.trace.wafer.{}|{}",
                                wafer.lot_id, wafer.wafer_id
                            ),
                            trace_wafer_label(wafer),
                            app.selected_trace_lot.as_ref() == Some(&wafer.lot_id)
                                && app.selected_trace_wafer.as_ref() == Some(&wafer.wafer_id),
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.trace.toggle_related",
        "Related",
        app.trace_related_only,
    )]);
    let impact_buttons = TraceImpactMode::ALL
        .iter()
        .map(|mode| {
            ViewControlButton::new(
                format!("glassworks.viewctl.trace.impact.{}", mode.slug()),
                mode.label(),
                app.trace_impact_mode == *mode,
            )
        })
        .collect::<Vec<_>>();
    for chunk in impact_buttons.chunks(chunk_size) {
        rows.push(chunk.to_vec());
    }
    if let Some(wafer) = selected_trace_wafer_ref(app) {
        let process_buttons = app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .into_iter()
            .rev()
            .take(4)
            .map(|record| {
                ViewControlButton::new(
                    trace_detail_action(&TraceSelection::Process(record.sequence)),
                    trace_process_button_label(record),
                    app.selected_trace_detail == Some(TraceSelection::Process(record.sequence)),
                )
            })
            .collect::<Vec<_>>();
        for chunk in process_buttons.chunks(chunk_size) {
            rows.push(chunk.to_vec());
        }

        let material_buttons = app
            .workspace
            .genealogy
            .material_ancestry_for_wafer(&wafer)
            .into_iter()
            .rev()
            .take(4)
            .map(|record| {
                ViewControlButton::new(
                    trace_detail_action(&TraceSelection::MaterialUse(record.sequence)),
                    trace_material_use_button_label(&app.workspace.genealogy, record),
                    app.selected_trace_detail == Some(TraceSelection::MaterialUse(record.sequence)),
                )
            })
            .collect::<Vec<_>>();
        for chunk in material_buttons.chunks(chunk_size) {
            rows.push(chunk.to_vec());
        }
    }
    let event_buttons = app
        .workspace
        .genealogy
        .events
        .iter()
        .rev()
        .take(4)
        .map(|event| {
            ViewControlButton::new(
                trace_detail_action(&TraceSelection::Event(event.sequence)),
                trace_event_button_label(event),
                app.selected_trace_detail == Some(TraceSelection::Event(event.sequence)),
            )
        })
        .collect::<Vec<_>>();
    for chunk in event_buttons.chunks(chunk_size) {
        rows.push(chunk.to_vec());
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn process_flow_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in ProcessFlowNodeFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.process_flow.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            process_flow_filter_count(&app.workspace, *filter)
                        ),
                        app.process_flow_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.process_flow.toggle_errors",
            "Errors",
            app.process_flow_errors_only,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.process_flow.validate",
            "Validate",
            false,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.process_flow.export",
        "Export",
        false,
    )]);
    for chunk in app
        .workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|node| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.process_flow.node.{}", node.id),
                        process_flow_node_button_label(node),
                        app.selected_process_node.as_ref() == Some(&node.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}
