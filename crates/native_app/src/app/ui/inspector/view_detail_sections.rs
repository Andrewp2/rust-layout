#![allow(unused_imports)]
use super::*;

pub(crate) fn view_detail_sections(app: &GlassworksApp) -> Vec<DetailSection> {
    if matches!(
        app.active_view,
        StartupView::Layout2d | StartupView::Layout3d
    ) {
        return layout_editor_inspector_sections(app);
    }

    let workspace = &app.workspace;
    let document = &workspace.document;
    let overview = DetailSection::new(
        "Workspace Overview",
        vec![
            (
                "Document".to_string(),
                display_document_name(&document.name),
            ),
            (
                "Shapes".to_string(),
                document.flattened_shape_count_estimate().to_string(),
            ),
            ("Layers".to_string(), document.layers.len().to_string()),
            ("Cells".to_string(), document.cells.len().to_string()),
            ("Tool".to_string(), app.active_tool.label().to_string()),
            (
                "Display".to_string(),
                format!(
                    "grid={}, snap={}, drc={}",
                    app.show_grid, app.snap_enabled, app.show_drc_overlay
                ),
            ),
        ],
    );

    let specific = match app.active_view {
        StartupView::Workflow => DetailSection::new(
            "Fab Workflow",
            vec![
                (
                    "Focus lot".to_string(),
                    app.workflow_focus_lot
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                ("Lots".to_string(), workspace.mes.lots.len().to_string()),
                ("Traveler".to_string(), workflow_focus_traveler_label(app)),
                (
                    "Dispatch lots".to_string(),
                    workspace.scheduler.lots.len().to_string(),
                ),
                (
                    "Tools online model".to_string(),
                    workspace.equipment.tools().count().to_string(),
                ),
                ("Yield".to_string(), workflow_focus_yield_label(app)),
                (
                    "Safety sensors".to_string(),
                    workspace.safety.sensors.len().to_string(),
                ),
                (
                    "Notebook entries".to_string(),
                    workspace.lab_notebook.entries.len().to_string(),
                ),
            ],
        ),
        StartupView::Layout2d | StartupView::Layout3d => {
            DetailSection::new("Layout Editor", layout_editor_detail_rows(app))
        }
        StartupView::MaskPrep => {
            let prep = reticle_prep_for_app(app);
            let report = prep.validate_document(document);
            DetailSection::new(
                "Reticle Prep",
                vec![
                    (
                        "Lot".to_string(),
                        app.mask_source_lot
                            .as_ref()
                            .map(|lot_id| workflow_lot_label(workspace, lot_id.as_str(), 32))
                            .unwrap_or_else(|| "Unlinked".to_string()),
                    ),
                    (
                        "Reticle".to_string(),
                        display_reticle_identifier(prep.reticle.id.as_str()),
                    ),
                    (
                        "Severity".to_string(),
                        app.mask_issue_severity_filter.detail_label().to_string(),
                    ),
                    (
                        "Grouping".to_string(),
                        app.mask_issue_grouping.label().to_string(),
                    ),
                    (
                        "Filtered issues".to_string(),
                        mask_filtered_issue_count(app, &report).to_string(),
                    ),
                    (
                        "Groups".to_string(),
                        mask_issue_group_count(app, &report).to_string(),
                    ),
                    ("Layers".to_string(), report.layer_count.to_string()),
                    ("Fields".to_string(), report.field_count.to_string()),
                ],
            )
        }
        StartupView::LayoutDiff => {
            let report = layout_diff_report(app);
            DetailSection::new(
                "Layout Diff",
                vec![
                    (
                        "Baseline".to_string(),
                        app.layout_diff_baseline.detail_label().to_string(),
                    ),
                    (
                        "Candidate".to_string(),
                        app.layout_diff_candidate.detail_label().to_string(),
                    ),
                    (
                        "Review".to_string(),
                        app.layout_diff_review_state.label().to_string(),
                    ),
                    (
                        "Filter".to_string(),
                        app.layout_diff_change_filter.detail_label().to_string(),
                    ),
                    (
                        "Changed only".to_string(),
                        app.layout_diff_changed_only.to_string(),
                    ),
                    (
                        "Filtered changes".to_string(),
                        layout_diff_filtered_change_count(app, &report).to_string(),
                    ),
                    (
                        "Total changes".to_string(),
                        layout_diff_total_changes(&report).to_string(),
                    ),
                    ("Layers".to_string(), report.layers.len().to_string()),
                ],
            )
        }
        StartupView::FabControl => DetailSection::new("Fab Control", {
            let mut rows = vec![
                (
                    "Selected".to_string(),
                    selected_equipment_tool(app)
                        .map(|tool| tool.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Tools".to_string(),
                    workspace.equipment.tools().count().to_string(),
                ),
                (
                    "Running".to_string(),
                    equipment_running_count(workspace).to_string(),
                ),
                (
                    "Active alarms".to_string(),
                    equipment_alarm_count(workspace).to_string(),
                ),
                (
                    "Samples".to_string(),
                    equipment_sample_count(workspace).to_string(),
                ),
            ];
            if let Some(tool) = selected_equipment_tool(app) {
                rows.push((tool.name.clone(), tool.state.label().to_string()));
                rows.push(("Recipe".to_string(), equipment_recipe_summary(tool)));
            }
            rows
        }),
        StartupView::Inventory => DetailSection::new(
            "Inventory",
            vec![
                (
                    "Selected".to_string(),
                    selected_inventory_lot(app)
                        .map(|lot| lot.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Filter".to_string(),
                    app.inventory_filter.detail_label().to_string(),
                ),
                (
                    "Material lots".to_string(),
                    workspace.inventory.lots.len().to_string(),
                ),
                (
                    "Visible".to_string(),
                    inventory_filter_count(workspace, app.inventory_filter).to_string(),
                ),
                (
                    "Alerts".to_string(),
                    workspace
                        .inventory
                        .alerts(INVENTORY_DEMO_TODAY)
                        .len()
                        .to_string(),
                ),
                (
                    "Linked MES lots".to_string(),
                    workspace.mes.lots.len().to_string(),
                ),
            ],
        ),
        StartupView::Maintenance => DetailSection::new(
            "Maintenance",
            vec![
                (
                    "Selected".to_string(),
                    selected_maintenance_tool(app)
                        .map(|tool| tool.tool_id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Work filter".to_string(),
                    app.maintenance_work_filter.detail_label().to_string(),
                ),
                (
                    "History".to_string(),
                    app.maintenance_history_filter.label().to_string(),
                ),
                (
                    "Tools".to_string(),
                    workspace.maintenance.tools.len().to_string(),
                ),
                (
                    "Action queue".to_string(),
                    maintenance_filtered_work_count(workspace, MaintenanceWorkFilter::Actionable)
                        .to_string(),
                ),
                (
                    "Locked".to_string(),
                    maintenance_locked_count(workspace).to_string(),
                ),
                (
                    "Calibrations".to_string(),
                    workspace.maintenance.calibration_records.len().to_string(),
                ),
                (
                    "Qualifications".to_string(),
                    workspace
                        .maintenance
                        .qualification_results
                        .len()
                        .to_string(),
                ),
                (
                    "Downtime".to_string(),
                    workspace.maintenance.downtime.len().to_string(),
                ),
            ],
        ),
        StartupView::Environment => DetailSection::new(
            "Cleanroom Environment",
            vec![
                (
                    "Selected".to_string(),
                    selected_environment_sensor(app)
                        .map(|sensor| sensor.id.clone())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                ("Latest".to_string(), environment_sensor_status(app)),
                (
                    "Sensors".to_string(),
                    workspace.environment.sensors.len().to_string(),
                ),
                (
                    "Active alarms".to_string(),
                    workspace.environment.active_alarms().len().to_string(),
                ),
                (
                    "Readings".to_string(),
                    workspace.environment.readings.len().to_string(),
                ),
                (
                    "Alarms".to_string(),
                    workspace.environment.alarms.len().to_string(),
                ),
                (
                    "Events".to_string(),
                    workspace.environment.events.len().to_string(),
                ),
            ],
        ),
        StartupView::Scheduler => DetailSection::new(
            "Dispatch",
            vec![
                (
                    "Policy".to_string(),
                    display_dispatch_policy_label(app.scheduler_policy).to_string(),
                ),
                (
                    "Shift minute".to_string(),
                    workspace.scheduler.now_minute.to_string(),
                ),
                (
                    "Selected tool".to_string(),
                    app.selected_scheduler_tool
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Min priority".to_string(),
                    format!("P{}+", app.scheduler_min_priority),
                ),
                (
                    "Tools".to_string(),
                    workspace.scheduler.tools.len().to_string(),
                ),
                (
                    "Lots".to_string(),
                    workspace.scheduler.lots.len().to_string(),
                ),
                (
                    "Visible lots".to_string(),
                    scheduler_filtered_lot_count(app).to_string(),
                ),
                (
                    "Assignments".to_string(),
                    scheduler_assignment_count(app).to_string(),
                ),
                (
                    "Unscheduled".to_string(),
                    scheduler_unscheduled_count(app).to_string(),
                ),
            ],
        ),
        StartupView::Safety => DetailSection::new(
            "Safety",
            vec![
                (
                    "Sensors".to_string(),
                    workspace.safety.sensors.len().to_string(),
                ),
                ("State".to_string(), safety_highest_label(app).to_string()),
                ("Selected tool".to_string(), safety_selected_tool_label(app)),
                (
                    "Acknowledged".to_string(),
                    app.acknowledged_count().to_string(),
                ),
                (
                    "Active conditions".to_string(),
                    workspace.safety.active_conditions().len().to_string(),
                ),
                (
                    "Alarm routes".to_string(),
                    workspace.safety.alarm_routes.len().to_string(),
                ),
                (
                    "Interlocks".to_string(),
                    workspace.safety.tool_interlocks.len().to_string(),
                ),
                (
                    "Incidents".to_string(),
                    workspace.safety.incidents.len().to_string(),
                ),
            ],
        ),
        StartupView::Traceability => {
            let selected_wafer = selected_trace_wafer_ref(app);
            let mut rows = trace_selected_detail_rows(app);
            rows.push((
                "Selected lot".to_string(),
                app.selected_trace_lot
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "None".to_string()),
            ));
            rows.push((
                "Selected wafer".to_string(),
                selected_wafer
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "None".to_string()),
            ));
            rows.push((
                "Impact mode".to_string(),
                app.trace_impact_mode.label().to_string(),
            ));
            rows.push((
                "Related / impacted".to_string(),
                format!(
                    "{} / {}",
                    trace_related_wafer_count(app),
                    trace_impact_count(app)
                ),
            ));
            DetailSection::new("Lot Traceability", rows)
        }
        StartupView::Metrology => DetailSection::new(
            "Metrology",
            vec![
                ("Wafer".to_string(), workspace.wafer_map.name.clone()),
                (
                    "Map".to_string(),
                    app.metrology_map_mode.label().to_string(),
                ),
                (
                    "Measurement".to_string(),
                    app.metrology_kind.label().to_string(),
                ),
                (
                    "Failed only".to_string(),
                    enabled_state_label(app.metrology_failed_only).to_string(),
                ),
                (
                    "Selected die".to_string(),
                    die_coord_label(app.selected_die),
                ),
                (
                    "Dies".to_string(),
                    workspace.wafer_map.dies.len().to_string(),
                ),
                (
                    "Measurements".to_string(),
                    workspace.wafer_map.measurements.len().to_string(),
                ),
                (
                    "Defects".to_string(),
                    workspace.wafer_map.defects.len().to_string(),
                ),
                (
                    "Annotations".to_string(),
                    workspace.wafer_map.annotations.len().to_string(),
                ),
            ],
        ),
        StartupView::Yield => DetailSection::new(
            "Yield Dashboard",
            vec![
                (
                    "Lot".to_string(),
                    app.selected_yield_lot
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Wafer".to_string(),
                    app.selected_yield_wafer
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                ("Filter".to_string(), yield_filter_summary(app)),
                (
                    "Attention only".to_string(),
                    enabled_state_label(app.show_only_attention_wafers).to_string(),
                ),
                (
                    "Excursions only".to_string(),
                    enabled_state_label(app.show_only_excursions).to_string(),
                ),
                (
                    "Lots".to_string(),
                    workspace.yield_analysis.lots.len().to_string(),
                ),
                (
                    "Wafer summaries".to_string(),
                    workspace.yield_analysis.wafer_summaries.len().to_string(),
                ),
                (
                    "Lot summaries".to_string(),
                    workspace.yield_analysis.lot_summaries.len().to_string(),
                ),
                (
                    "Correlations".to_string(),
                    workspace.yield_analysis.correlations.len().to_string(),
                ),
            ],
        ),
        StartupView::SpcFdc => {
            let monitor = spc_fdc_monitor(workspace);
            let selected_chart = selected_spc_chart(app, &monitor);
            let selected_trace = selected_fdc_trace(app, &monitor);
            DetailSection::new(
                "SPC / FDC",
                vec![
                    (
                        "Selected chart".to_string(),
                        selected_chart
                            .map(display_spc_chart_label)
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Selected trace".to_string(),
                        selected_trace
                            .map(display_spc_trace_name)
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Severity".to_string(),
                        app.spc_severity_filter.detail_label().to_string(),
                    ),
                    (
                        "Source".to_string(),
                        app.spc_source_filter.detail_label().to_string(),
                    ),
                    (
                        "Filtered findings".to_string(),
                        spc_filtered_finding_count(app, &monitor).to_string(),
                    ),
                    (
                        "Critical".to_string(),
                        monitor
                            .finding_count_by_severity(MonitorSeverity::Critical)
                            .to_string(),
                    ),
                    ("Charts".to_string(), monitor.charts.len().to_string()),
                    ("Traces".to_string(), monitor.traces.len().to_string()),
                ],
            )
        }
        StartupView::ProcessFlow => DetailSection::new(
            "Process Flow",
            vec![
                (
                    "Route".to_string(),
                    display_process_flow_route_name(&workspace.process_flow.route.name),
                ),
                (
                    "Selected node".to_string(),
                    selected_process_flow_node(app)
                        .map(|node| node.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Filter".to_string(),
                    app.process_flow_filter.label().to_string(),
                ),
                (
                    "Errors only".to_string(),
                    app.process_flow_errors_only.to_string(),
                ),
                (
                    "Nodes".to_string(),
                    workspace.process_flow.route.nodes.len().to_string(),
                ),
                (
                    "Edges".to_string(),
                    workspace.process_flow.route.edges.len().to_string(),
                ),
                (
                    "Errors".to_string(),
                    process_flow_error_count(workspace).to_string(),
                ),
                (
                    "Versions".to_string(),
                    workspace.process_flow.versions.len().to_string(),
                ),
            ],
        ),
        StartupView::ProcessControl => {
            let selected_loop = selected_control_loop(app);
            let selected_action = selected_control_action(app);
            DetailSection::new(
                "Run-to-Run Control",
                vec![
                    (
                        "Selected loop".to_string(),
                        selected_loop
                            .map(|loop_definition| loop_definition.name.clone())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Selected action".to_string(),
                        selected_action
                            .map(process_control_action_button_label)
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Action state".to_string(),
                        selected_action
                            .map(|action| capitalize_ascii_first(action.state.label().to_string()))
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Proposed".to_string(),
                        process_control_action_count(app, ControlActionState::Proposed).to_string(),
                    ),
                    (
                        "Approved".to_string(),
                        process_control_action_count(app, ControlActionState::Approved).to_string(),
                    ),
                    (
                        "Loops".to_string(),
                        workspace.process_control.loops.len().to_string(),
                    ),
                    (
                        "Actions".to_string(),
                        workspace.process_control.actions.len().to_string(),
                    ),
                    (
                        "Audit events".to_string(),
                        workspace.process_control.audit_events.len().to_string(),
                    ),
                ],
            )
        }
        StartupView::CrossSection => DetailSection::new(
            "Process Cross-Section",
            vec![
                (
                    "Step".to_string(),
                    format!(
                        "{} / {}",
                        app.cross_section_step,
                        cross_section_snapshot_count(workspace).saturating_sub(1)
                    ),
                ),
                ("Selected".to_string(), cross_section_selected_title(app)),
                (
                    "Material".to_string(),
                    app.selected_cross_section_material
                        .as_ref()
                        .map(|material| material.as_str().to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Overlays".to_string(),
                    format!(
                        "mask={}, dims={}, risks={}",
                        app.cross_section_show_mask,
                        app.cross_section_show_dimensions,
                        app.cross_section_show_risks
                    ),
                ),
                (
                    "Width".to_string(),
                    format!("{:.1} um", workspace.cross_section.width_um),
                ),
                (
                    "Columns".to_string(),
                    workspace.cross_section.columns.to_string(),
                ),
                (
                    "Materials".to_string(),
                    workspace.cross_section.materials.len().to_string(),
                ),
                (
                    "Steps".to_string(),
                    workspace.cross_section.steps.len().to_string(),
                ),
            ],
        ),
        StartupView::Experiment => DetailSection::new(
            "DOE Planner",
            vec![
                ("Plan".to_string(), workspace.experiment_plan.title.clone()),
                (
                    "Status".to_string(),
                    format!("{:?}", workspace.experiment_plan.status),
                ),
                (
                    "Selected run".to_string(),
                    app.selected_experiment_run_label(),
                ),
                (
                    "Primary response".to_string(),
                    app.selected_experiment_response_label(),
                ),
                (
                    "Response spec".to_string(),
                    app.selected_experiment_response
                        .as_ref()
                        .and_then(|response_id| workspace.experiment_plan.response(response_id))
                        .map(experiment_response_spec_label)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Run filter".to_string(),
                    app.experiment_run_filter.label().to_string(),
                ),
                (
                    "Lot filter".to_string(),
                    app.experiment_lot_filter
                        .as_deref()
                        .map(|lot_id| workflow_lot_label(workspace, lot_id, 32))
                        .unwrap_or_else(|| "All lots".to_string()),
                ),
                (
                    "Capture value".to_string(),
                    format_compact_number(app.experiment_capture_value),
                ),
                (
                    "Factors".to_string(),
                    workspace.experiment_plan.factors.len().to_string(),
                ),
                (
                    "Runs".to_string(),
                    workspace.experiment_plan.runs.len().to_string(),
                ),
                (
                    "Pending runs".to_string(),
                    app.experiment_pending_count().to_string(),
                ),
                (
                    "Pending only".to_string(),
                    app.experiment_show_missing_only.to_string(),
                ),
            ],
        ),
        StartupView::Notebook => {
            let filtered_entries = notebook_filtered_entries(app);
            let selected_entry = selected_notebook_entry(app);
            DetailSection::new(
                "Lab Notebook",
                vec![
                    (
                        "Selected".to_string(),
                        selected_entry
                            .map(|entry| entry.id.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Mode".to_string(),
                        if app.notebook_preview_mode {
                            "Preview"
                        } else {
                            "Edit"
                        }
                        .to_string(),
                    ),
                    (
                        "Tag".to_string(),
                        app.notebook_tag_filter
                            .clone()
                            .unwrap_or_else(|| "All tags".to_string()),
                    ),
                    (
                        "Link".to_string(),
                        app.notebook_link_kind_filter
                            .map(|kind| kind.label().to_string())
                            .unwrap_or_else(|| "Any link".to_string()),
                    ),
                    (
                        "Follow-ups only".to_string(),
                        app.notebook_followups_only.to_string(),
                    ),
                    ("Matches".to_string(), filtered_entries.len().to_string()),
                    (
                        "Entries".to_string(),
                        workspace.lab_notebook.entries.len().to_string(),
                    ),
                ],
            )
        }
    };

    let selection = DetailSection::new(
        "Current Controls",
        vec![
            (
                "Menu".to_string(),
                app.active_menu.map_or("Closed", AppMenu::label).to_string(),
            ),
            ("Details".to_string(), app.show_inspector.to_string()),
            (
                app.secondary_panel_label().to_string(),
                app.show_layers.to_string(),
            ),
        ],
    );

    vec![overview, specific, selection]
}
