#![allow(unused_imports)]
use super::*;

pub(crate) fn view_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    match app.active_view {
        StartupView::Workflow => workflow_primary_rows(app),
        StartupView::Layout2d | StartupView::Layout3d => Vec::new(),
        StartupView::MaskPrep => mask_primary_rows(app),
        StartupView::LayoutDiff => layout_diff_primary_rows(app),
        StartupView::FabControl => fab_primary_rows(app),
        StartupView::Maintenance => maintenance_primary_rows(app),
        StartupView::Environment => environment_primary_rows(app),
        StartupView::Inventory => inventory_primary_rows(app),
        StartupView::Scheduler => scheduler_primary_rows(app),
        StartupView::Safety => safety_primary_rows(app),
        StartupView::Traceability => traceability_primary_rows(app),
        StartupView::ProcessFlow => process_flow_primary_rows(app),
        StartupView::ProcessControl => process_control_primary_rows(app),
        StartupView::SpcFdc => spc_fdc_primary_rows(app),
        StartupView::CrossSection => cross_section_primary_rows(app),
        StartupView::Metrology => metrology_primary_rows(app),
        StartupView::Yield => yield_primary_rows(app),
        StartupView::Experiment => experiment_primary_rows(app),
        StartupView::Notebook => notebook_primary_rows(app),
    }
}

pub(crate) fn workflow_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let mut rows = Vec::new();
    if let Some(lot_id) = app
        .workflow_focus_lot
        .clone()
        .or_else(|| workflow_lot_ids(&app.workspace).into_iter().next())
    {
        rows.push(
            PrimaryRow::new(
                format!("workflow-focus-{lot_id}"),
                format!("Focus lot {lot_id}"),
                workflow_focus_traveler_label(app),
                workflow_focus_yield_label(app),
            )
            .action(
                format!("glassworks.viewctl.workflow.focus_lot.{lot_id}"),
                app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
            ),
        );
    }
    rows.extend(
        workflow_lot_ids(&app.workspace)
            .into_iter()
            .take(3)
            .map(|lot_id| {
                let lot = app
                    .workspace
                    .mes
                    .lots
                    .values()
                    .find(|lot| lot.id.as_str() == lot_id);
                let traveler = app
                    .workspace
                    .mes
                    .travelers
                    .values()
                    .find(|traveler| traveler.lot_id.as_str() == lot_id);
                let value = traveler
                    .map(|traveler| {
                        let step = traveler
                            .current_step_id
                            .as_ref()
                            .map(|step| step.as_str())
                            .unwrap_or("complete");
                        format!("{} {step}", traveler.status.label())
                    })
                    .unwrap_or_else(|| "No traveler".to_string());
                let yield_label = app
                    .workspace
                    .yield_analysis
                    .lot_summary(&lot_id)
                    .map(|summary| percent_label(summary.yield_fraction))
                    .unwrap_or_else(|| "yield n/a".to_string());
                let detail = lot
                    .map(|lot| {
                        format!(
                            "{} P{} {} wafers; {yield_label}",
                            lot.product,
                            lot.priority,
                            lot.wafers.len()
                        )
                    })
                    .unwrap_or(yield_label);
                PrimaryRow::new(format!("lot-{lot_id}"), lot_id.clone(), value, detail).action(
                    format!("glassworks.viewctl.workflow.focus_lot.{lot_id}"),
                    app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
                )
            }),
    );
    rows.extend(
        app.workspace
            .process_flow
            .route
            .nodes
            .iter()
            .take(2)
            .map(|node| {
                PrimaryRow::new(
                    format!("workflow-node-{}", node.id),
                    format!("{} {}", node.id, node.name),
                    format!("{} {}", node.kind.label(), node.area),
                    format!(
                        "{} tools; {}",
                        node.eligible_tools.len(),
                        node.recipe
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "no recipe".to_string())
                    ),
                )
                .action(
                    "glassworks.viewctl.workflow.open.process-flow",
                    app.active_view == StartupView::ProcessFlow,
                )
            }),
    );
    let dispatch = app.workspace.scheduler.dispatch(app.scheduler_policy);
    rows.extend(dispatch.assignments.iter().take(2).map(|assignment| {
        PrimaryRow::new(
            format!(
                "workflow-dispatch-{}-{}",
                assignment.lot_id, assignment.tool_id
            ),
            format!("Dispatch {}", assignment.lot_id),
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
            "glassworks.viewctl.workflow.open.scheduler",
            app.active_view == StartupView::Scheduler,
        )
    }));
    rows.extend(
        app.workspace
            .inventory
            .alerts(INVENTORY_DEMO_TODAY)
            .into_iter()
            .take(1)
            .map(|alert| {
                PrimaryRow::new(
                    format!("workflow-inventory-alert-{}", alert.lot_id),
                    format!("Inventory {}", alert.kind.label()),
                    alert.lot_id.to_string(),
                    format!("{}; {}", alert.material_name, alert.message),
                )
                .action(
                    "glassworks.viewctl.workflow.open.inventory",
                    app.active_view == StartupView::Inventory,
                )
            }),
    );
    rows.extend(
        app.workspace
            .safety
            .active_conditions()
            .into_iter()
            .take(1)
            .map(|sensor| {
                PrimaryRow::new(
                    format!("workflow-safety-{}", sensor.id),
                    format!("Safety {}", sensor.severity.label()),
                    sensor.name.clone(),
                    sensor.message.clone(),
                )
                .action(
                    "glassworks.viewctl.workflow.open.safety",
                    app.active_view == StartupView::Safety,
                )
            }),
    );
    rows.extend(workflow_cross_link_rows(app));
    rows.truncate(16);
    rows
}

pub(crate) fn workflow_cross_link_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let workspace = &app.workspace;
    let focus_lot = app.workflow_focus_lot.as_deref().unwrap_or("no focus lot");
    let today = maintenance_today(workspace);
    let actionable = maintenance_filtered_work_count(workspace, MaintenanceWorkFilter::Actionable);
    let locked = maintenance_locked_count(workspace);
    let active_alarms = workspace.environment.active_alarms().len();
    let safety = workspace.safety.summary();
    let metrology_summary = workspace.wafer_map.summary(app.metrology_kind);
    let trace_summary = workspace.genealogy.summary();
    vec![
        PrimaryRow::new(
            "workflow-maintenance",
            "Maintenance",
            format!("{actionable} due"),
            format!("{locked} locked tools; today {today}"),
        )
        .action(
            "glassworks.viewctl.workflow.open.maintenance",
            app.active_view == StartupView::Maintenance,
        ),
        PrimaryRow::new(
            "workflow-environment",
            "Cleanroom",
            format!("{active_alarms} alarms"),
            format!(
                "{} sensors; {} readings",
                workspace.environment.sensors.len(),
                workspace.environment.readings.len()
            ),
        )
        .action(
            "glassworks.viewctl.workflow.open.environment",
            app.active_view == StartupView::Environment,
        ),
        PrimaryRow::new(
            "workflow-safety-guardrails",
            "Safety",
            format!("{} active", safety.active_condition_count),
            format!(
                "{} locked; {} open incidents; highest {}",
                safety.locked_out_tool_count,
                safety.open_incident_count,
                safety
                    .highest_severity
                    .map(SafetySeverity::label)
                    .unwrap_or("normal")
            ),
        )
        .action(
            "glassworks.viewctl.workflow.open.safety",
            app.active_view == StartupView::Safety,
        ),
        PrimaryRow::new(
            "workflow-metrology",
            "Metrology",
            format!("{} sites", metrology_summary.sample_count),
            format!(
                "{} fail / {} outlier on {}",
                metrology_summary.fail_count,
                metrology_summary.outlier_count,
                app.metrology_kind.label()
            ),
        )
        .action(
            "glassworks.viewctl.workflow.open.metrology",
            app.active_view == StartupView::Metrology,
        ),
        PrimaryRow::new(
            "workflow-traceability",
            "Traceability",
            format!("{} lots", trace_summary.lot_count),
            format!(
                "{} wafers; {} process records for {focus_lot}",
                trace_summary.wafer_count, trace_summary.process_record_count
            ),
        )
        .action(
            "glassworks.viewctl.workflow.open.traceability",
            app.active_view == StartupView::Traceability,
        ),
        PrimaryRow::new(
            "workflow-notebook",
            "Notebook",
            format!("{} entries", workspace.lab_notebook.entries.len()),
            format!(
                "{} tags; {} links",
                workspace.lab_notebook.tags().len(),
                workspace
                    .lab_notebook
                    .entries
                    .iter()
                    .map(|entry| entry.link_count())
                    .sum::<usize>()
            ),
        )
        .action(
            "glassworks.viewctl.workflow.open.notebook",
            app.active_view == StartupView::Notebook,
        ),
    ]
}
