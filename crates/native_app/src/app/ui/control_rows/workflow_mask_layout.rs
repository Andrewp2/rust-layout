#![allow(unused_imports)]
use super::*;

pub(crate) fn workflow_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = vec![
        vec![
            ViewControlButton::new("glassworks.viewctl.workflow.open.layout2d", "Layout", false),
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.process-flow",
                "Process",
                false,
            ),
        ],
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.fab-control",
                "Tools",
                false,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.scheduler",
                "Dispatch",
                false,
            ),
        ],
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.inventory",
                "Inventory",
                false,
            ),
            ViewControlButton::new("glassworks.viewctl.workflow.open.yield", "Yield", false),
        ],
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.maintenance",
                "Maintenance",
                false,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.environment",
                "Cleanroom",
                false,
            ),
        ],
        vec![
            ViewControlButton::new("glassworks.viewctl.workflow.open.safety", "Safety", false),
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.metrology",
                "Metrology",
                false,
            ),
        ],
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.traceability",
                "Trace",
                false,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.workflow.open.notebook",
                "Notebook",
                false,
            ),
        ],
        vec![ViewControlButton::new(
            "glassworks.viewctl.workflow.load_demo",
            "Load demo",
            false,
        )],
    ];
    let lot_ids = workflow_lot_ids(&app.workspace);
    for chunk in lot_ids.iter().take(4).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.workflow.focus_lot.{lot_id}"),
                        lot_id.to_string(),
                        app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
                    )
                })
                .collect(),
        );
    }
    rows
}

pub(crate) fn mask_prep_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let report = mask_check_report(app);
    let page_count = mask_issue_page_count(app, &report);
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in app
        .workspace
        .mes
        .lots
        .keys()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.mask.lot.{lot_id}"),
                        workflow_lot_label(&app.workspace, lot_id.as_str(), 18),
                        app.mask_source_lot.as_ref() == Some(lot_id),
                    )
                })
                .collect(),
        );
    }
    rows.push(
        MaskIssueSeverityFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.mask.severity.{}", filter.slug()),
                    filter.label(),
                    app.mask_issue_severity_filter == *filter,
                )
            })
            .collect(),
    );
    rows.push(
        MaskIssueGrouping::ALL
            .iter()
            .map(|grouping| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.mask.group.{}", grouping.slug()),
                    grouping.label(),
                    app.mask_issue_grouping == *grouping,
                )
            })
            .collect(),
    );
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.mask.prev_page",
            "Prev issues",
            app.mask_issue_page > 0,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.mask.next_page",
            format!(
                "Next {}/{}",
                app.mask_issue_page.min(page_count - 1) + 1,
                page_count
            ),
            app.mask_issue_page + 1 < page_count,
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new("glassworks.viewctl.mask.rebuild", "Rebuild", false),
        ViewControlButton::new("glassworks.viewctl.mask.clear_filters", "Clear", false),
    ]);
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn layout_diff_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let report = layout_diff_report(app);
    let page_count = layout_diff_page_count(app, &report);
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 4 };
    for chunk in LayoutDiffSource::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|source| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout_diff.baseline.{}", source.slug()),
                        format!("Base {}", source.label()),
                        app.layout_diff_baseline == *source,
                    )
                })
                .collect(),
        );
    }
    for chunk in LayoutDiffSource::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|source| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout_diff.candidate.{}", source.slug()),
                        format!("Cand {}", source.label()),
                        app.layout_diff_candidate == *source,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new("glassworks.viewctl.layout_diff.swap", "Swap", false),
        ViewControlButton::new(
            "glassworks.viewctl.layout_diff.toggle_changed_only",
            "Changed",
            app.layout_diff_changed_only,
        ),
    ]);
    for chunk in LayoutChangeFilter::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout_diff.filter.{}", filter.slug()),
                        filter.label(),
                        app.layout_diff_change_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    for chunk in LAYOUT_DIFF_PAGE_SIZE_OPTIONS.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|page_size| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout_diff.page_size.{page_size}"),
                        format!("{page_size} rows"),
                        normalized_layout_diff_page_size(app) == *page_size,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout_diff.prev_page",
            "Prev",
            app.layout_diff_change_page > 0,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.layout_diff.next_page",
            format!(
                "Next {}/{}",
                app.layout_diff_change_page.min(page_count - 1) + 1,
                page_count
            ),
            app.layout_diff_change_page + 1 < page_count,
        ),
    ]);
    for chunk in LayoutReviewDisposition::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|state| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout_diff.review.{}", state.slug()),
                        state.label(),
                        app.layout_diff_review_state == *state,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout_diff.demo_current",
            "Demo -> Cur",
            false,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.layout_diff.empty_current",
            "Empty -> Cur",
            false,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.layout_diff.reset_filters",
        "Reset filters",
        false,
    )]);
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn fab_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 4 };
    for chunk in app
        .workspace
        .equipment
        .tools()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.fab.select.{}", tool.id),
                        tool.id.to_string(),
                        app.selected_equipment_tool.as_ref() == Some(&tool.id),
                    )
                })
                .collect(),
        );
    }
    if let Some(tool) = selected_equipment_tool(app) {
        for chunk in tool
            .available_recipes
            .values()
            .take(4)
            .collect::<Vec<_>>()
            .chunks(chunk_size)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|recipe| {
                        let selected = app.equipment_recipe_drafts.get(&tool.id).or_else(|| {
                            tool.selected_recipe
                                .as_ref()
                                .map(|selection| &selection.recipe_id)
                        }) == Some(&recipe.id);
                        ViewControlButton::new(
                            format!("glassworks.viewctl.fab.recipe.{}|{}", tool.id, recipe.id),
                            display_recipe_identifier(recipe.id.as_str()),
                            selected,
                        )
                    })
                    .collect(),
            );
        }
        let command_buttons = vec![
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.online|{}", tool.id),
                "Online",
                tool.state == EquipmentToolState::OnlineIdle,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.load|{}", tool.id),
                "Load",
                tool.state == EquipmentToolState::RecipeLoaded,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.start|{}", tool.id),
                "Start",
                tool.state == EquipmentToolState::Running,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.stop|{}", tool.id),
                "Stop",
                false,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.alarm|{}", tool.id),
                "Alarm",
                equipment_active_alarm_count(tool) > 0,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.clear|{}", tool.id),
                "Clear",
                false,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.reset|{}", tool.id),
                "Reset",
                false,
            ),
            ViewControlButton::new(
                format!("glassworks.viewctl.fab.command.maintenance|{}", tool.id),
                "Maint",
                tool.state == EquipmentToolState::Maintenance,
            ),
        ];
        rows.extend(
            command_buttons
                .chunks(chunk_size)
                .map(|chunk| chunk.to_vec()),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn maintenance_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 3 };
    for chunk in MaintenanceWorkFilter::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.maintenance.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            maintenance_filtered_work_count(&app.workspace, *filter)
                        ),
                        app.maintenance_work_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    rows.push(
        MaintenanceHistoryFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.maintenance.history.{}", filter.slug()),
                    filter.label(),
                    app.maintenance_history_filter == *filter,
                )
            })
            .collect(),
    );
    for chunk in app
        .workspace
        .maintenance
        .tools
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.maintenance.tool.{}", tool.tool_id),
                        tool.tool_id.to_string(),
                        app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(tool) = selected_maintenance_tool(app) {
        let mut tool_actions = vec![
            ViewControlButton::new(
                format!(
                    "glassworks.viewctl.maintenance.status.schedule|{}",
                    tool.tool_id
                ),
                "Schedule",
                false,
            ),
            ViewControlButton::new(
                format!(
                    "glassworks.viewctl.maintenance.status.calibration|{}",
                    tool.tool_id
                ),
                "Calibrate",
                false,
            ),
            ViewControlButton::new(
                format!(
                    "glassworks.viewctl.maintenance.status.release|{}",
                    tool.tool_id
                ),
                "Release",
                !app.workspace
                    .maintenance
                    .release_for_tool(&tool.tool_id, maintenance_today(&app.workspace))
                    .state
                    .released_to_production(),
            ),
        ];
        if compact_rows {
            if let Some(release) = tool_actions.pop() {
                rows.push(tool_actions);
                rows.push(vec![release]);
            }
        } else {
            rows.push(tool_actions);
        }
    }
    rows.retain(|row| !row.is_empty());
    rows
}
