#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct DashboardMetric {
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) detail: String,
}

impl DashboardMetric {
    pub(crate) fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            detail: detail.into(),
        }
    }
}

pub(crate) fn workflow_dashboard_metrics(app: &GlassworksApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let dispatch = workspace.scheduler.dispatch(app.scheduler_policy);
    vec![
        DashboardMetric::new(
            "Layout",
            workspace
                .document
                .flattened_shape_count_estimate()
                .to_string(),
            format!("{} layers", workspace.document.layers.len()),
        ),
        DashboardMetric::new(
            "Flow",
            workspace.process_flow.route.nodes.len().to_string(),
            format!("{} findings", workspace.process_flow.findings().len()),
        ),
        DashboardMetric::new(
            "WIP",
            workspace.mes.lots.len().to_string(),
            format!("{} dispatches", dispatch.assignments.len()),
        ),
        DashboardMetric::new(
            "Factory",
            workspace.equipment.tools().count().to_string(),
            format!(
                "{} running / {} alarms",
                equipment_running_count(workspace),
                equipment_alarm_count(workspace)
            ),
        ),
        DashboardMetric::new(
            "Quality",
            workflow_focus_yield_label(app),
            format!("{} notebook entries", workspace.lab_notebook.entries.len()),
        ),
    ]
}

pub(crate) fn mask_dashboard_metrics(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<DashboardMetric> {
    let prep = reticle_prep_for_app(app);
    let report = mask_check_report(app);
    let filtered = mask_filtered_issue_count(app, &report);
    vec![
        DashboardMetric::new(
            "Reticle",
            display_reticle_identifier(prep.reticle.id.as_str()),
            format!("{} fields", report.field_count),
        ),
        DashboardMetric::new(
            "Issues",
            filtered.to_string(),
            format!(
                "{} errors / {} warnings",
                report.error_count(),
                report.warning_count()
            ),
        ),
        DashboardMetric::new(
            "Layers",
            report.layer_count.to_string(),
            format!("{} printable shapes", report.printable_shape_count),
        ),
        DashboardMetric::new(
            "Grouping",
            app.mask_issue_grouping.label(),
            format!("{} groups", mask_issue_group_count(app, &report)),
        ),
        DashboardMetric::new(
            "Exposures",
            report.exposure_block_count.to_string(),
            app.mask_source_lot
                .as_ref()
                .map(|lot| {
                    if compact_rows {
                        "linked lot".to_string()
                    } else {
                        format!(
                            "linked {}",
                            workflow_lot_label(&app.workspace, lot.as_str(), 32)
                        )
                    }
                })
                .unwrap_or_else(|| "unlinked lot".to_string()),
        ),
    ]
}

pub(crate) fn layout_diff_dashboard_metrics(app: &GlassworksApp) -> Vec<DashboardMetric> {
    let report = layout_diff_report(app);
    vec![
        DashboardMetric::new(
            "Changes",
            layout_diff_filtered_change_count(app, &report).to_string(),
            format!("{} total", layout_diff_total_changes(&report)),
        ),
        DashboardMetric::new(
            "Added",
            report.summary.added_shapes.to_string(),
            format!("{} removed", report.summary.removed_shapes),
        ),
        DashboardMetric::new(
            "Modified",
            report.summary.modified_shapes.to_string(),
            format!("{} layers", report.layers.len()),
        ),
        DashboardMetric::new(
            "Sources",
            app.layout_diff_baseline.label(),
            format!("candidate {}", app.layout_diff_candidate.label()),
        ),
        DashboardMetric::new(
            "Review",
            app.layout_diff_review_state.label(),
            app.layout_diff_change_filter.detail_label(),
        ),
    ]
}

pub(crate) fn fab_control_dashboard_metrics(app: &GlassworksApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let tools = workspace.equipment.tools().count();
    let selected = selected_equipment_tool(app);
    vec![
        DashboardMetric::new(
            "Tools",
            tools.to_string(),
            format!("{} running", equipment_running_count(workspace)),
        ),
        DashboardMetric::new(
            "Alarms",
            equipment_alarm_count(workspace).to_string(),
            "active equipment alarms",
        ),
        DashboardMetric::new(
            "Telemetry",
            equipment_sample_count(workspace).to_string(),
            "recent sensor samples",
        ),
        DashboardMetric::new(
            "Selected",
            selected
                .map(|tool| tool.id.to_string())
                .unwrap_or_else(|| "None".to_string()),
            selected
                .map(|tool| tool.state.label().to_string())
                .unwrap_or_else(|| "no tool selected".to_string()),
        ),
        DashboardMetric::new(
            "Recipe",
            selected
                .map(equipment_recipe_summary)
                .unwrap_or_else(|| "n/a".to_string()),
            "loaded/draft recipe",
        ),
    ]
}

pub(crate) fn traceability_dashboard_metrics(app: &GlassworksApp) -> Vec<DashboardMetric> {
    let summary = app.workspace.genealogy.summary();
    vec![
        DashboardMetric::new(
            "Lots",
            summary.lot_count.to_string(),
            format!("{} wafers", summary.wafer_count),
        ),
        DashboardMetric::new(
            "Lineage",
            trace_related_wafer_count(app).to_string(),
            "related wafers",
        ),
        DashboardMetric::new(
            "Impact",
            trace_impact_count(app).to_string(),
            app.trace_impact_mode.label(),
        ),
        DashboardMetric::new(
            "Records",
            summary.process_record_count.to_string(),
            format!("{} material lots", summary.material_lot_count),
        ),
        DashboardMetric::new(
            "Events",
            app.workspace.genealogy.events.len().to_string(),
            format!(
                "{} splits / {} merges",
                summary.split_count, summary.merge_count
            ),
        ),
    ]
}
