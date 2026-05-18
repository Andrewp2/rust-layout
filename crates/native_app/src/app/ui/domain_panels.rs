#![allow(unused_imports)]
use super::*;

pub(crate) fn add_workflow_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    add_domain_panel(
        document,
        parent,
        app,
        "Fab Workflow",
        "Workspace workflow, production focus, route operations, and cross-links",
        "Fab Workflow",
        vec![
            DomainSection::new(
                "glassworks.workflow.dashboard",
                "Workflow Summary",
                "No workflow summary available",
                dashboard_metric_rows("workflow", workflow_dashboard_metrics(app)),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.workflow.lots",
                "Lot Focus",
                "No lot focus targets",
                workflow_lot_rows(app),
            )
            .max_rows(6),
            DomainSection::new(
                "glassworks.workflow.focus",
                "Production Focus",
                "No workspace data loaded",
                workflow_focus_rows(app),
            )
            .max_rows(6),
            DomainSection::new(
                "glassworks.workflow.spine",
                "Operational Workflow",
                "No workflow steps loaded",
                workflow_spine_rows(app),
            )
            .max_rows(6),
            DomainSection::new(
                "glassworks.workflow.cross_links",
                "Cross-links",
                "No workflow cross-links available",
                workflow_cross_link_rows(app),
            )
            .max_rows(6),
        ],
        ui_scale,
        compact_rows,
    );
}

pub(crate) fn add_mask_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let prep = reticle_prep_for_app(app);
    add_domain_panel(
        document,
        parent,
        app,
        "Mask / Reticle Prep",
        "Reticle prep, layer stack, exposure blocks, fields, and retained mask checks",
        "Mask / Reticle Prep",
        vec![
            DomainSection::new(
                "glassworks.mask.overview",
                "Reticle Prep Summary",
                "No reticle prep summary available",
                dashboard_metric_rows("mask", mask_dashboard_metrics(app, compact_rows)),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.mask.reticle",
                "Reticle",
                "No reticle data available",
                mask_reticle_rows(&prep),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.mask.fields",
                "Reticle Fields",
                "No reticle fields available",
                mask_primary_rows(app),
            )
            .max_rows(6),
            DomainSection::new(
                "glassworks.mask.layers",
                "Mask Layers",
                "No mask layers available",
                mask_layer_rows(&prep),
            )
            .max_rows(8),
            DomainSection::new(
                "glassworks.mask.checks",
                "Mask Checks",
                "No retained mask checks",
                mask_primary_rows(app),
            )
            .max_rows(8),
        ],
        ui_scale,
        compact_rows,
    );
}

pub(crate) fn add_layout_diff_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    add_domain_panel(
        document,
        parent,
        app,
        "Layout Diff Review",
        "Baseline/candidate comparison, layer deltas, change paging, and review state",
        "Layout Diff Review",
        vec![
            DomainSection::new(
                "glassworks.layout_diff.overview",
                "Diff Summary",
                "No diff summary available",
                dashboard_metric_rows("diff", layout_diff_dashboard_metrics(app)),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.layout_diff.layers",
                "Changed Layers",
                "No layer changes available",
                layout_diff_primary_rows(app),
            )
            .max_rows(8),
            DomainSection::new(
                "glassworks.layout_diff.changes",
                "Shape Changes",
                "No shape changes match the active filters",
                layout_diff_primary_rows(app),
            )
            .max_rows(10),
        ],
        ui_scale,
        compact_rows,
    );
}

pub(crate) fn add_fab_control_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    add_domain_panel(
        document,
        parent,
        app,
        "Fab Control Room",
        "Equipment state, recipe loading, host commands, alarms, and telemetry",
        "Fab Control Room",
        vec![
            DomainSection::new(
                "glassworks.fab_control.overview",
                "Tool Summary",
                "No equipment summary available",
                dashboard_metric_rows("fab", fab_control_dashboard_metrics(app)),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.fab_control.tools",
                "Tool Fleet",
                "No equipment tools loaded",
                fab_tool_rows(app),
            )
            .max_rows(8),
            DomainSection::new(
                "glassworks.fab_control.detail",
                "Selected Tool",
                "No tool selected",
                selected_equipment_tool(app)
                    .map(fab_selected_tool_rows)
                    .unwrap_or_default(),
            )
            .max_rows(10),
        ],
        ui_scale,
        compact_rows,
    );
}

pub(crate) fn add_maintenance_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
    wide_rows: bool,
    body_width: f32,
) {
    let today = maintenance_today(&app.workspace);
    let stacked_workbench = compact_rows || !wide_rows || body_width < ui_scale.value(1600.0);
    let panel_height = if compact_rows {
        840.0
    } else if stacked_workbench {
        820.0
    } else {
        646.0
    };
    let workbench_height = if compact_rows {
        520.0
    } else if stacked_workbench {
        540.0
    } else {
        372.0
    };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.maintenance.overview",
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
        "glassworks.maintenance.title",
        if compact_rows {
            "Maintenance and Calibration"
        } else {
            "Fab operations - Maintenance and Calibration"
        },
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    add_text(
        document,
        panel,
        "glassworks.maintenance.subtitle",
        if compact_rows {
            format!(
                "{today} - {} tools / {} locked",
                app.workspace.maintenance.tools.len(),
                maintenance_locked_count(&app.workspace)
            )
        } else {
            format!(
                "Audit date {today} - {} tools tracked - {} locked",
                app.workspace.maintenance.tools.len(),
                maintenance_locked_count(&app.workspace)
            )
        },
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );

    if app.workspace.maintenance.tools.is_empty() {
        add_primary_cell(
            document,
            panel,
            "glassworks.maintenance.empty",
            "No maintenance model loaded",
            1.0,
            false,
            ui_scale,
        );
        return;
    }

    add_maintenance_summary_row(document, panel, app, today, ui_scale, compact_rows);

    let workbench = document.add_child(
        panel,
        UiNode::container(
            "glassworks.maintenance.workbench",
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
    add_maintenance_queue_panel(document, workbench, app, ui_scale, compact_rows);
    if compact_rows {
        return;
    }
    add_maintenance_detail_panel(document, workbench, app, today, ui_scale);
    add_maintenance_audit_panel(document, workbench, app, today, ui_scale);
}

pub(crate) struct MaintenanceWorkItem<'a> {
    pub(crate) tool: &'a layout_model::maintenance::ToolMaintenanceState,
    pub(crate) schedule: &'a layout_model::maintenance::MaintenanceSchedule,
    pub(crate) due_state: DueState,
    pub(crate) days_until: i32,
    pub(crate) run_count_delta: Option<u32>,
    pub(crate) release_state: ToolReleaseState,
    pub(crate) blocked: bool,
}

pub(crate) fn maintenance_work_items(app: &GlassworksApp) -> Vec<MaintenanceWorkItem<'_>> {
    let today = maintenance_today(&app.workspace);
    let mut items = Vec::new();
    for tool in &app.workspace.maintenance.tools {
        let release = app
            .workspace
            .maintenance
            .release_for_tool(&tool.tool_id, today);
        for schedule in &tool.schedules {
            let due_state = schedule.due_state(today, tool.run_count);
            if !maintenance_work_matches(
                &app.workspace,
                app.maintenance_work_filter,
                &tool.tool_id,
                schedule.kind,
                due_state,
                schedule.next_due,
            ) {
                continue;
            }
            items.push(MaintenanceWorkItem {
                tool,
                schedule,
                due_state,
                days_until: today.days_until(schedule.next_due),
                run_count_delta: schedule.interval_runs.map(|_| {
                    tool.run_count
                        .saturating_sub(schedule.last_completed_run_count)
                }),
                release_state: release.state,
                blocked: !release.state.released_to_production(),
            });
        }
    }
    items.sort_by_key(|item| {
        (
            maintenance_due_priority(item.due_state),
            item.schedule.next_due,
            item.tool.tool_id.to_string(),
            item.schedule.id.clone(),
        )
    });
    items
}

pub(crate) fn maintenance_due_priority(state: DueState) -> u8 {
    match state {
        DueState::Overdue => 0,
        DueState::Due => 1,
        DueState::NotDue => 2,
    }
}

pub(crate) fn maintenance_due_window_label(days_until: i32) -> String {
    if days_until < 0 {
        format!("{} days overdue", days_until.abs())
    } else if days_until == 0 {
        "today".to_string()
    } else if days_until == 1 {
        "tomorrow".to_string()
    } else {
        format!("in {days_until} days")
    }
}

pub(crate) fn add_maintenance_summary_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    today: FabDate,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let mut overdue = 0;
    let mut due_now = 0;
    let mut due_soon = 0;
    let mut calibration_watch = 0;
    for tool in &app.workspace.maintenance.tools {
        for schedule in &tool.schedules {
            let due_state = schedule.due_state(today, tool.run_count);
            let days_until = today.days_until(schedule.next_due);
            match due_state {
                DueState::Overdue => overdue += 1,
                DueState::Due => due_now += 1,
                DueState::NotDue if days_until <= 7 => due_soon += 1,
                DueState::NotDue => {}
            }
            if schedule.kind == MaintenanceKind::Calibration
                && (due_state.blocks_release() || days_until <= 14)
            {
                calibration_watch += 1;
            }
        }
        if app
            .workspace
            .maintenance
            .calibration_records_for(&tool.tool_id)
            .into_iter()
            .max_by_key(|record| record.performed_at)
            .is_some_and(|record| {
                record.outcome != layout_model::maintenance::CalibrationOutcome::Passed
            })
        {
            calibration_watch += 1;
        }
    }

    if compact_rows {
        add_compact_metric_rows(
            document,
            parent,
            "glassworks.maintenance.summary",
            &[
                format!("Overdue: {overdue}"),
                format!("Due now: {due_now}"),
                format!("Due soon: {due_soon}"),
                format!(
                    "Cal: {calibration_watch} / Locked: {}",
                    maintenance_locked_count(&app.workspace)
                ),
            ],
            ui_scale,
        );
        return;
    }
    let summary_items = [
        format!("Overdue: {overdue} release risk"),
        format!("Due now: {due_now} needs action"),
        format!("Due soon: {due_soon} next 7 days"),
        format!("Calibration watch: {calibration_watch}"),
        format!("Locked: {}", maintenance_locked_count(&app.workspace)),
    ];

    let row = document.add_child(
        parent,
        UiNode::container(
            "glassworks.maintenance.summary",
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
            format!("glassworks.maintenance.summary.{index}"),
            text,
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_maintenance_queue_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let queue = document.add_child(
        parent,
        UiNode::container(
            "glassworks.maintenance.queue",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::with_flex(layout::column(), 1.35, 1.0, layout::px(0.0)),
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
        queue,
        "glassworks.maintenance.queue.title",
        "Maintenance Queue",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    let work_items = maintenance_work_items(app);
    if work_items.is_empty() {
        add_primary_cell(
            document,
            queue,
            "glassworks.maintenance.queue.empty",
            "No maintenance tasks match the current filter",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, item) in work_items.iter().take(8).enumerate() {
            add_maintenance_work_card(document, queue, app, item, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        queue,
        "glassworks.maintenance.queue.header",
        &[
            ("State", 0.85),
            ("Tool", 0.9),
            ("Kind", 1.0),
            ("Task", 1.8),
            ("Due", 1.15),
            ("Release", 1.25),
        ],
        ui_scale,
    );
    for (index, item) in work_items.iter().take(8).enumerate() {
        let row = add_inventory_table_row(
            document,
            queue,
            format!("glassworks.maintenance.queue.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.queue.row.{index}.state"),
            item.due_state.label(),
            0.85,
            false,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.maintqueue{index}.maintenance.tool.{}",
                item.tool.tool_id
            ),
            item.tool.tool_id.to_string(),
            app.selected_maintenance_tool.as_ref() == Some(&item.tool.tool_id),
            primary_cell_layout(0.9),
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.queue.row.{index}.kind"),
            item.schedule.kind.label(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.queue.row.{index}.task"),
            compact_button_label(
                &item
                    .run_count_delta
                    .map(|runs| format!("{} - {} runs", item.schedule.task, runs))
                    .unwrap_or_else(|| item.schedule.task.clone()),
                32,
            ),
            1.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.queue.row.{index}.due"),
            format!(
                "{} ({})",
                item.schedule.next_due,
                maintenance_due_window_label(item.days_until)
            ),
            1.15,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.queue.row.{index}.release"),
            if item.blocked {
                format!("{} hold", item.release_state.label())
            } else {
                item.release_state.label().to_string()
            },
            1.25,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_maintenance_work_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    item: &MaintenanceWorkItem<'_>,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.maintenance.queue.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(84.0)),
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
    add_button(
        document,
        row,
        format!(
            "glassworks.primary.action.maintqueue{index}.maintenance.tool.{}",
            item.tool.tool_id
        ),
        compact_button_label(
            &format!(
                "{} {} {}",
                item.due_state.label(),
                item.tool.tool_id,
                item.schedule.kind.label()
            ),
            42,
        ),
        app.selected_maintenance_tool.as_ref() == Some(&item.tool.tool_id),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(30.0))),
        ui_scale,
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.task"),
        compact_button_label(
            &item
                .run_count_delta
                .map(|runs| format!("{} - {} runs", item.schedule.task, runs))
                .unwrap_or_else(|| item.schedule.task.clone()),
            54,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale.value(7.0),
        ),
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.due"),
        compact_button_label(
            &format!(
                "{} ({}) / {}",
                item.schedule.next_due,
                maintenance_due_window_label(item.days_until),
                if item.blocked {
                    format!("{} hold", item.release_state.label())
                } else {
                    item.release_state.label().to_string()
                }
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

pub(crate) fn add_maintenance_detail_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    today: FabDate,
    ui_scale: UiScale,
) {
    let detail = document.add_child(
        parent,
        UiNode::container(
            "glassworks.maintenance.detail",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::with_flex(layout::column(), 1.05, 1.0, layout::px(0.0)),
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
        detail,
        "glassworks.maintenance.detail.title",
        "Tool Detail",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    let Some(tool) = selected_maintenance_tool(app) else {
        add_primary_cell(
            document,
            detail,
            "glassworks.maintenance.detail.empty",
            "No tool selected",
            1.0,
            false,
            ui_scale,
        );
        return;
    };
    let release = app
        .workspace
        .maintenance
        .release_for_tool(&tool.tool_id, today);
    add_button(
        document,
        detail,
        format!(
            "glassworks.primary.action.maintdetailtool.maintenance.tool.{}",
            tool.tool_id
        ),
        tool.tool_name.clone(),
        true,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(32.0))),
        ui_scale,
    );
    for (index, text) in [
        format!(
            "{} - {} runs - {}",
            tool.tool_id,
            tool.run_count,
            release.state.label()
        ),
        if release.reasons.is_empty() {
            "Release context: no active holds".to_string()
        } else {
            format!(
                "Release hold: {}",
                release
                    .reasons
                    .iter()
                    .take(2)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        },
    ]
    .into_iter()
    .enumerate()
    {
        add_primary_cell(
            document,
            detail,
            format!("glassworks.maintenance.detail.context.{index}"),
            compact_button_label(&text, 58),
            1.0,
            false,
            ui_scale,
        );
    }

    let actions = document.add_child(
        detail,
        UiNode::container(
            "glassworks.maintenance.detail.actions",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(32.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_button(
        document,
        actions,
        format!(
            "glassworks.primary.action.maintstatusschedule.maintenance.status.schedule|{}",
            tool.tool_id
        ),
        "Schedule maintenance",
        false,
        primary_cell_layout(1.1),
        ui_scale,
    );
    if tool
        .schedules
        .iter()
        .any(|schedule| schedule.kind == MaintenanceKind::Calibration)
        || release.state == ToolReleaseState::CalibrationLockout
    {
        add_button(
            document,
            actions,
            format!(
                "glassworks.primary.action.maintstatuscalibration.maintenance.status.calibration|{}",
                tool.tool_id
            ),
            "Capture calibration",
            false,
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
    if !release.state.released_to_production() {
        add_button(
            document,
            actions,
            format!(
                "glassworks.primary.action.maintstatusrelease.maintenance.status.release|{}",
                tool.tool_id
            ),
            "Resolve release hold",
            false,
            primary_cell_layout(1.0),
            ui_scale,
        );
    }

    add_text(
        document,
        detail,
        "glassworks.maintenance.detail.next.title",
        "Next Work",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    if let Some(schedule) = tool.schedules.iter().min_by_key(|schedule| {
        (
            maintenance_due_priority(schedule.due_state(today, tool.run_count)),
            schedule.next_due,
            schedule.id.clone(),
        )
    }) {
        let due_state = schedule.due_state(today, tool.run_count);
        add_primary_cell(
            document,
            detail,
            "glassworks.maintenance.detail.next.row",
            compact_button_label(
                &format!(
                    "{} - {} due {} ({})",
                    schedule.task,
                    due_state.label(),
                    schedule.next_due,
                    maintenance_due_window_label(today.days_until(schedule.next_due))
                ),
                64,
            ),
            1.0,
            false,
            ui_scale,
        );
        if !schedule.checklist.is_empty() {
            add_primary_cell(
                document,
                detail,
                "glassworks.maintenance.detail.checklist",
                compact_button_label(
                    &format!(
                        "Checklist: {}",
                        schedule
                            .checklist
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                    64,
                ),
                1.0,
                false,
                ui_scale,
            );
        }
    }

    if let Some(record) = app
        .workspace
        .maintenance
        .calibration_records_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|record| record.performed_at)
    {
        add_primary_cell(
            document,
            detail,
            "glassworks.maintenance.detail.calibration",
            compact_button_label(
                &format!(
                    "Latest calibration: {} {} {:.2} vs {} by {}",
                    record.performed_at,
                    record.parameter,
                    record.measured_value,
                    record.tolerance,
                    record.technician
                ),
                64,
            ),
            1.0,
            false,
            ui_scale,
        );
    }
    if let Some(result) = app
        .workspace
        .maintenance
        .qualification_results_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|result| result.performed_at)
    {
        add_primary_cell(
            document,
            detail,
            "glassworks.maintenance.detail.qualification",
            compact_button_label(
                &format!(
                    "Latest qualification: {} {} {} {:.2} ({})",
                    result.performed_at, result.wafer_id, result.metric, result.value, result.spec
                ),
                64,
            ),
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_maintenance_audit_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    today: FabDate,
    ui_scale: UiScale,
) {
    let audit = document.add_child(
        parent,
        UiNode::container(
            "glassworks.maintenance.audit",
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
        audit,
        "glassworks.maintenance.audit.title",
        "Release Board",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    for (index, tool) in app.workspace.maintenance.tools.iter().take(3).enumerate() {
        let release = app
            .workspace
            .maintenance
            .release_for_tool(&tool.tool_id, today);
        let row = add_inventory_table_row(
            document,
            audit,
            format!("glassworks.maintenance.release.row.{index}"),
            index,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.maintrelease{index}.maintenance.tool.{}",
                tool.tool_id
            ),
            compact_button_label(&tool.tool_name, 28),
            app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
            primary_cell_layout(1.3),
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.release.row.{index}.state"),
            release.state.label(),
            1.1,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.maintenance.release.row.{index}.detail"),
            if release.reasons.is_empty() {
                format!("{} runs; no release holds", tool.run_count)
            } else {
                compact_button_label(
                    &format!(
                        "{} runs; {}",
                        tool.run_count,
                        release
                            .reasons
                            .iter()
                            .take(2)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                    42,
                )
            },
            1.8,
            false,
            ui_scale,
        );
    }

    add_text(
        document,
        audit,
        "glassworks.maintenance.history_filters.title",
        "History Scope",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    let filters = document.add_child(
        audit,
        UiNode::container(
            "glassworks.maintenance.history_filters",
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
    for filter in MaintenanceHistoryFilter::ALL {
        add_button(
            document,
            filters,
            format!(
                "glassworks.primary.action.mainthistory{}.maintenance.history.{}",
                filter.slug(),
                filter.slug()
            ),
            filter.label(),
            app.maintenance_history_filter == filter,
            primary_cell_layout(1.0),
            ui_scale,
        );
    }

    add_text(
        document,
        audit,
        "glassworks.maintenance.history.title",
        "Maintenance History",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    let rows = maintenance_history_rows(app);
    if rows.is_empty() {
        add_primary_cell(
            document,
            audit,
            "glassworks.maintenance.history.empty",
            "No history entries match the scope",
            1.0,
            false,
            ui_scale,
        );
    } else {
        for (index, row_data) in rows.iter().take(3).enumerate() {
            let row = add_inventory_table_row(
                document,
                audit,
                format!("glassworks.maintenance.history.row.{index}"),
                index,
                ui_scale,
            );
            if let Some(action) = row_data.action.as_ref() {
                let button_name = action
                    .strip_prefix("glassworks.viewctl.")
                    .map(|action| format!("glassworks.primary.action.mainthist{index}.{action}"))
                    .unwrap_or_else(|| format!("glassworks.maintenance.history.row.{index}.action"));
                add_button(
                    document,
                    row,
                    button_name,
                    compact_button_label(&row_data.title, 32),
                    row_data.selected,
                    primary_cell_layout(1.4),
                    ui_scale,
                );
            } else {
                add_primary_cell(
                    document,
                    row,
                    format!("glassworks.maintenance.history.row.{index}.title"),
                    compact_button_label(&row_data.title, 32),
                    1.4,
                    false,
                    ui_scale,
                );
            }
            add_primary_cell(
                document,
                row,
                format!("glassworks.maintenance.history.row.{index}.value"),
                compact_button_label(&row_data.value, 20),
                0.8,
                false,
                ui_scale,
            );
            add_primary_cell(
                document,
                row,
                format!("glassworks.maintenance.history.row.{index}.detail"),
                compact_button_label(&row_data.detail, 42),
                1.8,
                false,
                ui_scale,
            );
        }
    }
}
