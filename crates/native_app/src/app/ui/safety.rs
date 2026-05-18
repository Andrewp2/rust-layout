#![allow(unused_imports)]
use super::*;

pub(crate) fn add_safety_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let summary = app.workspace.safety.summary();
    let show_acknowledged = app.app_options.domains.safety.show_acknowledged;
    let mut lockouts = app.workspace.safety.evaluate_lockouts();
    if !show_acknowledged {
        lockouts.retain(|lockout| !app.acknowledged_lockouts.contains(&lockout.tool_id));
    }
    let active_conditions = app
        .workspace
        .safety
        .active_conditions()
        .into_iter()
        .filter(|sensor| {
            show_acknowledged || !app.acknowledged_conditions.contains(sensor.id.as_str())
        })
        .collect::<Vec<_>>();
    let stacked_workbench = compact_rows || ui_scale.factor() > 1.25;
    let panel_height = if compact_rows {
        860.0
    } else if stacked_workbench {
        820.0
    } else {
        646.0
    };
    let workbench_height = if compact_rows {
        430.0
    } else if stacked_workbench {
        430.0
    } else {
        266.0
    };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.overview",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(panel_height)),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.safety.title",
        "Safety Operations",
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    add_text(
        document,
        panel,
        "glassworks.safety.subtitle",
        "Safety monitoring, alarm routing, tool lockout, and audit trail",
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );

    add_safety_summary_row(document, panel, app, &summary, ui_scale, compact_rows);
    add_safety_active_interlocks(
        document,
        panel,
        app,
        &active_conditions,
        ui_scale,
        compact_rows,
    );

    let workbench = document.add_child(
        panel,
        UiNode::container(
            "glassworks.safety.workbench",
            layout::with_gap_all(
                layout::with_size(
                    if stacked_workbench {
                        layout::column()
                    } else {
                        layout::row()
                    },
                    layout::percent(1.0),
                    layout::px(ui_scale.value(workbench_height)),
                ),
                ui_scale.value(10.0),
            ),
        ),
    );
    add_safety_sensor_table(document, workbench, app, ui_scale, compact_rows);
    add_safety_tool_panel(document, workbench, app, &lockouts, ui_scale);
    add_safety_incident_section(document, panel, app, ui_scale, compact_rows);
}

pub(crate) fn add_safety_summary_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    summary: &layout_model::safety::SafetySummary,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    if compact_rows {
        add_compact_metric_rows(
            document,
            parent,
            "glassworks.safety.summary",
            &[
                format!(
                    "State: {}",
                    summary
                        .highest_severity
                        .map(SafetySeverity::label)
                        .unwrap_or("blank")
                ),
                format!("Interlocks: {}", summary.active_condition_count),
                format!("Locked: {}", summary.locked_out_tool_count),
                format!(
                    "Incidents {} / Ack {}",
                    summary.open_incident_count,
                    app.acknowledged_count()
                ),
            ],
            ui_scale,
        );
        return;
    }
    let summary_items = [
        format!(
            "Overall state: {}",
            summary
                .highest_severity
                .map(SafetySeverity::label)
                .unwrap_or("blank")
        ),
        format!("Active interlocks: {}", summary.active_condition_count),
        format!("Locked tools: {}", summary.locked_out_tool_count),
        format!("Open incidents: {}", summary.open_incident_count),
        format!("Acknowledged: {}", app.acknowledged_count()),
    ];
    let row = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.summary",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, text) in summary_items.into_iter().enumerate() {
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.summary.{index}"),
            text,
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_safety_active_interlocks(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    active_conditions: &[&layout_model::safety::SafetySensor],
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let visible_conditions = active_conditions.len().max(1).min(3);
    let section_height = if compact_rows {
        ui_scale.value(30.0) + ui_scale.value(102.0) * visible_conditions as f32
    } else {
        ui_scale.value(132.0)
    };
    let section = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.interlocks",
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(section_height),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_text(
        document,
        section,
        "glassworks.safety.interlocks.title",
        "Active Interlocks",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    if active_conditions.is_empty() {
        add_primary_cell(
            document,
            section,
            "glassworks.safety.interlocks.empty",
            "All interlocks are clear",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, sensor) in active_conditions.iter().take(3).enumerate() {
            add_safety_interlock_card(document, section, app, sensor, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        section,
        "glassworks.safety.interlocks.header",
        &[
            ("Severity", 0.7),
            ("Condition", 2.4),
            ("Value", 1.2),
            ("Route", 1.0),
        ],
        ui_scale,
    );
    for (index, sensor) in active_conditions.iter().take(3).enumerate() {
        let row = add_inventory_table_row(
            document,
            section,
            format!("glassworks.safety.interlocks.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.interlocks.row.{index}.severity"),
            sensor.severity.label(),
            0.7,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.interlocks.row.{index}.condition"),
            compact_button_label(
                &format!(
                    "{} / {} / {}",
                    sensor.name,
                    sensor.domain.label(),
                    safety_sensor_tool_label(&app.workspace, sensor)
                ),
                72,
            ),
            2.4,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.interlocks.row.{index}.value"),
            safety_sensor_value(sensor),
            1.2,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.interlocks.row.{index}.route"),
            safety_route_summary(&app.workspace.safety, sensor),
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_safety_interlock_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    sensor: &layout_model::safety::SafetySensor,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.safety.interlocks.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(96.0)),
                ),
                ui_scale.value(4.0),
            ),
        )
        .with_visual(UiVisual::panel(
            if index % 2 == 0 {
                COLOR_PANEL_ALT
            } else {
                COLOR_PANEL_BG
            },
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    let top = document.add_child(
        row,
        UiNode::container(
            format!("{row_name}.top"),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_primary_text_cell(
        document,
        top,
        format!("{row_name}.severity"),
        sensor.severity.label(),
        false,
        ui_scale,
        layout::with_padding_all(primary_cell_layout(0.8), ui_scale.value(7.0)),
    );
    add_button(
        document,
        top,
        format!(
            "glassworks.primary.action.safetycondition{index}.safety.ack.condition.{}",
            sensor.id
        ),
        if app.acknowledged_conditions.contains(sensor.id.as_str()) {
            "acked".to_string()
        } else {
            safety_ack_condition_label(sensor)
        },
        app.acknowledged_conditions.contains(sensor.id.as_str()),
        primary_cell_layout(1.0),
        ui_scale,
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.condition"),
        compact_button_label(
            &format!(
                "{} / {} / {}",
                sensor.name,
                sensor.domain.label(),
                safety_sensor_tool_label(&app.workspace, sensor)
            ),
            72,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(36.0))),
            ui_scale.value(7.0),
        ),
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.route"),
        compact_button_label(
            &format!(
                "{} / {}",
                safety_sensor_value(sensor),
                safety_route_summary(&app.workspace.safety, sensor)
            ),
            54,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale.value(7.0),
        ),
    );
}

pub(crate) fn add_safety_sensor_table(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let table = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.sensors",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::with_flex(layout::column(), 1.2, 1.0, layout::px(0.0)),
                        layout::auto(),
                        layout::percent(1.0),
                    ),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_APP_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    add_text(
        document,
        table,
        "glassworks.safety.sensors.title",
        "Safety Sensors",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    if app.workspace.safety.sensors.is_empty() {
        add_primary_cell(
            document,
            table,
            "glassworks.safety.sensors.empty",
            "No safety sensors loaded",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, sensor) in app.workspace.safety.sensors.iter().take(5).enumerate() {
            add_safety_sensor_card(document, table, sensor, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        table,
        "glassworks.safety.sensors.header",
        &[
            ("Sensor", 1.5),
            ("Domain", 1.0),
            ("State", 1.0),
            ("Value", 1.0),
        ],
        ui_scale,
    );
    for (index, sensor) in app.workspace.safety.sensors.iter().take(5).enumerate() {
        let row = add_inventory_table_row(
            document,
            table,
            format!("glassworks.safety.sensors.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.sensors.row.{index}.name"),
            compact_button_label(&sensor.name, 28),
            1.5,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.sensors.row.{index}.domain"),
            sensor.domain.label(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.sensors.row.{index}.state"),
            format!("{} / {}", sensor.state.label(), sensor.severity.label()),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.sensors.row.{index}.value"),
            safety_sensor_value(sensor),
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_safety_sensor_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    sensor: &layout_model::safety::SafetySensor,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.safety.sensors.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(60.0)),
                ),
                ui_scale.value(4.0),
            ),
        )
        .with_visual(UiVisual::panel(
            if index % 2 == 0 {
                COLOR_PANEL_ALT
            } else {
                COLOR_PANEL_BG
            },
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.name"),
        compact_button_label(&format!("{} / {}", sensor.name, sensor.domain.label()), 54),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale.value(7.0),
        ),
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.state"),
        compact_button_label(
            &format!(
                "{} / {} / {}",
                sensor.state.label(),
                sensor.severity.label(),
                safety_sensor_value(sensor)
            ),
            54,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale.value(7.0),
        ),
    );
}

pub(crate) fn add_safety_tool_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    lockouts: &[layout_model::safety::ToolLockout],
    ui_scale: UiScale,
) {
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.tools",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::with_flex(layout::column(), 1.0, 1.0, layout::px(0.0)),
                        layout::auto(),
                        layout::percent(1.0),
                    ),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_APP_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.safety.tools.title",
        "Tool Lockouts",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    if lockouts.is_empty() {
        add_primary_cell(
            document,
            panel,
            "glassworks.safety.tools.empty",
            "No tool interlocks loaded",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    for (index, lockout) in lockouts.iter().take(3).enumerate() {
        let row = add_inventory_table_row(
            document,
            panel,
            format!("glassworks.safety.tools.row.{index}"),
            index,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.safetytool{index}.safety.tool.{}",
                lockout.tool_id
            ),
            compact_button_label(&lockout.tool_name, 24),
            app.selected_safety_tool.as_deref() == Some(lockout.tool_id.as_str()),
            primary_cell_layout(1.4),
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.tools.row.{index}.state"),
            if lockout.locked_out {
                "locked out"
            } else {
                "ready"
            },
            0.8,
            false,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.safetylockout{index}.safety.ack.lockout.{}",
                lockout.tool_id
            ),
            if app.acknowledged_lockouts.contains(&lockout.tool_id) {
                "acked"
            } else {
                "Acknowledge"
            },
            app.acknowledged_lockouts.contains(&lockout.tool_id),
            primary_cell_layout(0.9),
            ui_scale,
        );
    }
    add_text(
        document,
        panel,
        "glassworks.safety.selected.title",
        "Selected Tool Readiness",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    if let Some(lockout) = selected_safety_lockout(app) {
        add_primary_cell(
            document,
            panel,
            "glassworks.safety.selected.detail",
            compact_button_label(
                &format!(
                    "{}: {}",
                    lockout.tool_name,
                    if lockout.reasons.is_empty() {
                        "all permissives pass".to_string()
                    } else {
                        lockout.reasons.join("; ")
                    }
                ),
                72,
            ),
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_safety_incident_section(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let incidents = app
        .workspace
        .safety
        .incidents
        .iter()
        .filter(|incident| {
            app.app_options.domains.safety.show_acknowledged
                || !app.acknowledged_incidents.contains(incident.id.0.as_str())
        })
        .collect::<Vec<_>>();
    let visible_incidents = incidents.len().max(1).min(2);
    let section_height = if compact_rows {
        ui_scale.value(30.0) + ui_scale.value(76.0) * visible_incidents as f32
    } else {
        ui_scale.value(118.0)
    };
    let section = document.add_child(
        parent,
        UiNode::container(
            "glassworks.safety.incidents",
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(section_height),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_text(
        document,
        section,
        "glassworks.safety.incidents.title",
        "Incident Trail",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    if incidents.is_empty() {
        add_primary_cell(
            document,
            section,
            "glassworks.safety.incidents.empty",
            "No safety incidents loaded",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, incident) in incidents.iter().take(2).enumerate() {
            add_safety_incident_card(document, section, app, incident, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        section,
        "glassworks.safety.incidents.header",
        &[
            ("Incident", 1.0),
            ("Status", 0.8),
            ("Domain", 0.9),
            ("Route", 1.7),
            ("Action", 1.0),
        ],
        ui_scale,
    );
    for (index, incident) in incidents.iter().take(2).enumerate() {
        let row = add_inventory_table_row(
            document,
            section,
            format!("glassworks.safety.incidents.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.incidents.row.{index}.id"),
            format!(
                "{} {}",
                display_safety_incident_identifier(&incident.id),
                incident.severity.label()
            ),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.incidents.row.{index}.status"),
            incident.status.label(),
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.incidents.row.{index}.domain"),
            incident.domain.label(),
            0.9,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.safety.incidents.row.{index}.route"),
            compact_button_label(
                &incident
                    .routed_to
                    .iter()
                    .map(|target| target.label())
                    .collect::<Vec<_>>()
                    .join(", "),
                48,
            ),
            1.7,
            false,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.safetyincident{index}.safety.ack.incident.{}",
                incident.id
            ),
            if app.acknowledged_incidents.contains(incident.id.0.as_str()) {
                "acked"
            } else {
                "Acknowledge"
            },
            app.acknowledged_incidents.contains(incident.id.0.as_str()),
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}

pub(crate) fn add_safety_incident_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    incident: &layout_model::safety::SafetyIncident,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.safety.incidents.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(70.0)),
                ),
                ui_scale.value(4.0),
            ),
        )
        .with_visual(UiVisual::panel(
            if index % 2 == 0 {
                COLOR_PANEL_ALT
            } else {
                COLOR_PANEL_BG
            },
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    let top = document.add_child(
        row,
        UiNode::container(
            format!("{row_name}.top"),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_primary_text_cell(
        document,
        top,
        format!("{row_name}.id"),
        compact_button_label(
            &format!(
                "{} {}",
                display_safety_incident_identifier(&incident.id),
                incident.severity.label()
            ),
            28,
        ),
        false,
        ui_scale,
        layout::with_padding_all(primary_cell_layout(1.2), ui_scale.value(7.0)),
    );
    add_button(
        document,
        top,
        format!(
            "glassworks.primary.action.safetyincident{index}.safety.ack.incident.{}",
            incident.id
        ),
        if app.acknowledged_incidents.contains(incident.id.0.as_str()) {
            "acked"
        } else {
            "Acknowledge"
        },
        app.acknowledged_incidents.contains(incident.id.0.as_str()),
        primary_cell_layout(1.0),
        ui_scale,
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.route"),
        compact_button_label(
            &format!(
                "{} / {} / {}",
                incident.status.label(),
                incident.domain.label(),
                incident
                    .routed_to
                    .iter()
                    .map(|target| target.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            54,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale.value(7.0),
        ),
    );
}

pub(crate) fn safety_sensor_value(sensor: &layout_model::safety::SafetySensor) -> String {
    format!(
        "{} {} ({})",
        compact_f64(sensor.value),
        sensor.unit,
        safety_limit_label(&sensor.limit)
    )
}

pub(crate) fn compact_f64(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

pub(crate) fn safety_route_summary(
    safety: &layout_model::safety::SafetySystem,
    sensor: &layout_model::safety::SafetySensor,
) -> String {
    let targets = safety
        .route_targets_for(sensor)
        .into_iter()
        .map(|target| target.label())
        .collect::<Vec<_>>();
    if targets.is_empty() {
        "no matching route".to_string()
    } else {
        targets.join(", ")
    }
}
