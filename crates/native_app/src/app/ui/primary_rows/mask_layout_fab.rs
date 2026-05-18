#![allow(unused_imports)]
use super::*;

pub(crate) fn mask_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let prep = reticle_prep_for_app(app);
    let report = mask_check_report(app);
    let mut rows = mask_reticle_rows(&prep);
    rows.extend(mask_exposure_rows(&prep, &report).into_iter().take(2));
    rows.extend(mask_layer_rows(&prep).into_iter().take(2));
    rows.extend(mask_issue_group_rows(app, &report).into_iter().take(2));
    rows.extend(
        report
            .issues
            .iter()
            .filter(|issue| mask_issue_matches_filter(app, issue))
            .skip(app.mask_issue_page * MASK_ISSUE_PAGE_SIZE)
            .take(10usize.saturating_sub(rows.len()))
            .map(|issue| mask_issue_primary_row(app, issue)),
    );
    if rows.is_empty() {
        rows.push(PrimaryRow::new(
            "mask-clean",
            "Checks clean",
            "No mask prep issues",
            format!(
                "{} printable shapes across {} layers",
                report.printable_shape_count, report.layer_count
            ),
        ));
    }
    rows.truncate(10);
    rows
}

pub(crate) fn mask_reticle_rows(prep: &ReticlePrep) -> Vec<PrimaryRow> {
    let printable = prep.reticle.printable_bounds();
    let mut rows = vec![
        PrimaryRow::new(
            "mask-design",
            "Mask design",
            display_mask_identifier(&prep.mask_design_id),
            format!(
                "{}; route {}",
                display_layout_revision_identifier(&prep.layout_revision),
                prep.route_id
                    .as_ref()
                    .map(|route_id| display_route_identifier(route_id.as_str()))
                    .unwrap_or_else(|| "unlinked".to_string())
            ),
        ),
        PrimaryRow::new(
            "mask-reticle",
            format!(
                "Reticle {}",
                display_reticle_identifier(prep.reticle.id.as_str())
            ),
            display_mask_identifier(&prep.reticle.name),
            format!(
                "size {} x {}; printable {} x {}",
                prep.reticle.size.width,
                prep.reticle.size.height,
                printable.width(),
                printable.height()
            ),
        ),
    ];
    rows.extend(prep.fields.iter().take(1).map(|field| {
        PrimaryRow::new(
            format!("mask-field-{}", field.id),
            field.name.clone(),
            display_reticle_field_identifier(field.id.as_str()),
            format!(
                "cell {}; bounds {} x {}; stepping {} x {}",
                field.source_cell.0,
                field.layout_bounds.width(),
                field.layout_bounds.height(),
                field.stepping.columns,
                field.stepping.rows
            ),
        )
    }));
    rows
}

pub(crate) fn mask_exposure_rows(prep: &ReticlePrep, report: &MaskCheckReport) -> Vec<PrimaryRow> {
    prep.exposure_blocks
        .iter()
        .map(|block| {
            let issue_count = report
                .issues
                .iter()
                .filter(|issue| issue.block_id.as_ref() == Some(&block.id))
                .count();
            PrimaryRow::new(
                format!("mask-exposure-{}", block.id),
                format!("Exposure {}", block.id),
                block.name.clone(),
                format!(
                    "field {}; dose {:.1}; focus {:+.2}; passes {}; layers {}; {} issue(s)",
                    block.field_id,
                    block.dose_mj_cm2,
                    block.focus_offset_um,
                    block.passes,
                    block.layer_ids.len(),
                    issue_count
                ),
            )
        })
        .collect()
}

pub(crate) fn mask_layer_rows(prep: &ReticlePrep) -> Vec<PrimaryRow> {
    prep.layer_stack
        .iter()
        .map(|layer| {
            PrimaryRow::new(
                format!("mask-layer-{}", layer.layer.0),
                format!("Layer {} {}", layer.layer.0, layer.name),
                layer.tone.label(),
                format!(
                    "{:?}; feature {}; spacing {}; order {}{}",
                    layer.process,
                    layer.min_feature,
                    layer.min_spacing,
                    layer.display_order,
                    if layer.critical { "; critical" } else { "" }
                ),
            )
        })
        .collect()
}

pub(crate) fn mask_issue_group_rows(
    app: &GlassworksApp,
    report: &MaskCheckReport,
) -> Vec<PrimaryRow> {
    let mut groups = BTreeMap::<String, (usize, usize)>::new();
    for issue in report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
    {
        let entry = groups
            .entry(mask_issue_group_key(issue, app.mask_issue_grouping))
            .or_default();
        match issue.severity {
            MaskIssueSeverity::Error => entry.0 += 1,
            MaskIssueSeverity::Warning => entry.1 += 1,
        }
    }
    let mut rows = groups.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_total = left.1.0 + left.1.1;
        let right_total = right.1.0 + right.1.1;
        right_total
            .cmp(&left_total)
            .then_with(|| left.0.cmp(&right.0))
    });
    rows.into_iter()
        .map(|(group, (errors, warnings))| {
            PrimaryRow::new(
                format!("mask-group-{group}"),
                format!("{group} group"),
                format!("{} total", errors + warnings),
                format!("{errors} error(s), {warnings} warning(s)"),
            )
        })
        .collect()
}

pub(crate) fn mask_issue_primary_row(
    app: &GlassworksApp,
    issue: &layout_model::mask::MaskPrepIssue,
) -> PrimaryRow {
    let location = [
        issue.layer.map(|layer| format!("L{}", layer.0)),
        issue
            .field_id
            .as_ref()
            .map(|field| format!("field {field}")),
        issue
            .block_id
            .as_ref()
            .map(|block| format!("block {block}")),
        issue.shape_id.map(|shape| format!("shape {}", shape.0)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let row = PrimaryRow::new(
        format!("mask-{}", issue.code),
        format!("{} {}", issue.severity.label(), issue.code),
        if location.is_empty() {
            "Document".to_string()
        } else {
            location
        },
        issue.message.clone(),
    );
    if let Some(shape) = issue.shape_id {
        row.action(
            format!("glassworks.viewctl.layout.shape.{}", shape.0),
            app.selected_layout_shape == Some(shape),
        )
    } else {
        row
    }
}

pub(crate) fn layout_diff_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let report = layout_diff_report(app);
    let page_size = normalized_layout_diff_page_size(app);
    let mut rows = vec![
        PrimaryRow::new(
            "diff-setup",
            "Comparison",
            format!(
                "{} -> {}",
                app.layout_diff_baseline.label(),
                app.layout_diff_candidate.label()
            ),
            format!(
                "changed only {}; filter {}; page size {}",
                app.layout_diff_changed_only,
                app.layout_diff_change_filter.detail_label(),
                page_size
            ),
        ),
        PrimaryRow::new(
            "diff-summary",
            "Shape summary",
            format!("{} changes", layout_diff_total_changes(&report)),
            format!(
                "{} added; {} removed; {} modified",
                report.summary.added_shapes,
                report.summary.removed_shapes,
                report.summary.modified_shapes
            ),
        ),
        PrimaryRow::new(
            "diff-review",
            "Review",
            app.layout_diff_review_state.label(),
            format!(
                "{} baseline shapes; {} candidate shapes",
                report.summary.baseline_shapes, report.summary.candidate_shapes
            ),
        ),
    ];
    rows.extend(report.layers.iter().take(3).map(|layer| {
        PrimaryRow::new(
            format!("diff-layer-{}", layer.layer.0),
            format!("Layer {} {}", layer.layer.0, layer.name),
            format!("{} -> {}", layer.baseline_shapes, layer.candidate_shapes),
            format!(
                "{} added; {} removed; {} modified",
                layer.added_shapes, layer.removed_shapes, layer.modified_shapes
            ),
        )
    }));
    rows.extend(
        report
            .changes
            .iter()
            .filter(|change| app.layout_diff_change_filter.matches(change.kind))
            .skip(app.layout_diff_change_page * page_size)
            .take(10usize.saturating_sub(rows.len()))
            .map(|change| {
                PrimaryRow::new(
                    format!("diff-{}-{}", change.kind.label(), change.id.0),
                    format!("{} shape {}", change.kind.label(), change.id.0),
                    format!("L{} {}", change.layer.0, change.layer_name),
                    change.detail.clone(),
                )
                .action(
                    format!("glassworks.viewctl.layout.shape.{}", change.id.0),
                    app.selected_layout_shape == Some(change.id),
                )
            }),
    );
    rows.truncate(10);
    rows
}

pub(crate) fn fab_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let mut rows = app
        .workspace
        .equipment
        .tools()
        .take(5)
        .map(|tool| {
            let alarms = equipment_active_alarm_count(tool);
            let alarm_label = if alarms == 1 { "alarm" } else { "alarms" };
            PrimaryRow::new(
                format!("tool-{}", tool.id),
                format!("{} {}", tool.id, tool.name),
                tool.state.label(),
                format!(
                    "{}; {alarms} active {alarm_label}",
                    equipment_recipe_summary(tool)
                ),
            )
            .action(
                format!("glassworks.viewctl.fab.select.{}", tool.id),
                app.selected_equipment_tool.as_ref() == Some(&tool.id),
            )
        })
        .collect::<Vec<_>>();
    if let Some(tool) = selected_equipment_tool(app) {
        rows.extend(
            fab_selected_tool_rows(tool)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }
    rows.truncate(10);
    rows
}

pub(crate) fn fab_selected_tool_rows(tool: &EquipmentTool) -> Vec<PrimaryRow> {
    let mut rows = vec![PrimaryRow::new(
        format!("fab-selected-{}", tool.id),
        format!("Selected {}", tool.name),
        tool.state.label(),
        format!(
            "{}; {} recipes; last update t+{}s",
            tool.kind.label(),
            tool.available_recipes.len(),
            tool.last_updated_at_s
        ),
    )];
    if let Some(selection) = tool.selected_recipe.as_ref() {
        rows.push(PrimaryRow::new(
            format!("fab-recipe-{}", tool.id),
            "Selected recipe",
            display_recipe_identifier(selection.recipe_id.as_str()),
            format!("version {}", selection.recipe_version),
        ));
    } else {
        rows.push(PrimaryRow::new(
            format!("fab-recipe-{}", tool.id),
            "Selected recipe",
            "none",
            "load a recipe before running",
        ));
    }
    rows.extend(tool.active_alarms.iter().take(2).map(|alarm| {
        PrimaryRow::new(
            format!("fab-alarm-{}-{}", tool.id, alarm.id),
            format!("{} {}", alarm.severity.label(), alarm.code),
            if alarm.active { "active" } else { "cleared" },
            format!("{} at t+{}s", alarm.message, alarm.occurred_at_s),
        )
    }));
    if let Some(run) = tool.active_run.as_ref() {
        rows.push(PrimaryRow::new(
            format!("fab-run-{}", run.id),
            format!("Run {}", run.id),
            run.status.label(),
            format!(
                "{} since t+{}s; {} samples",
                display_recipe_identifier(run.recipe.recipe_id.as_str()),
                run.started_at_s,
                run.sensor_count
            ),
        ));
    }
    rows.extend(tool.recent_sensors.iter().rev().take(3).map(|sample| {
        PrimaryRow::new(
            format!("fab-sensor-{}-{}-{}", tool.id, sample.name, sample.at_s),
            sample.name.clone(),
            format!("{} {}", spc_compact_number(sample.value), sample.unit),
            format!("sample t+{}s", sample.at_s),
        )
    }));
    rows.extend(tool.event_log.iter().rev().take(2).map(|event| {
        PrimaryRow::new(
            format!("fab-event-{}-{}", tool.id, event.at_s),
            "Event",
            format!("t+{}s", event.at_s),
            event.message.clone(),
        )
    }));
    rows
}
