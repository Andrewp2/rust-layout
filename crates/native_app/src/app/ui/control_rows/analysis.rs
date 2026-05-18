#![allow(unused_imports)]
use super::*;

pub(crate) fn process_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    rows.push(
        ControlLoopFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.process_control.filter.{}", filter.slug()),
                    filter.label(),
                    app.process_control_loop_filter == *filter,
                )
            })
            .collect(),
    );
    let loops = filtered_control_loops(app);
    for chunk in loops.iter().take(6).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|loop_definition| {
                    ViewControlButton::new(
                        format!(
                            "glassworks.viewctl.process_control.loop.{}",
                            loop_definition.id
                        ),
                        process_control_loop_button_label(&loop_definition.name),
                        app.selected_control_loop.as_ref() == Some(&loop_definition.id),
                    )
                })
                .collect(),
        );
    }

    if let Some(loop_definition) = selected_control_loop(app) {
        let actions = app
            .workspace
            .process_control
            .actions_for_loop(&loop_definition.id);
        for chunk in actions.iter().take(6).collect::<Vec<_>>().chunks(2) {
            rows.push(
                chunk
                    .iter()
                    .map(|action| {
                        ViewControlButton::new(
                            format!("glassworks.viewctl.process_control.action.{}", action.id),
                            process_control_action_button_label(action),
                            app.selected_control_action.as_ref() == Some(&action.id),
                        )
                    })
                    .collect(),
            );
        }
    }

    if let Some(action) = selected_control_action(app) {
        let transitions = process_control_transition_options(action.state);
        if !transitions.is_empty() {
            rows.push(
                transitions
                    .iter()
                    .map(|transition| {
                        ViewControlButton::new(
                            format!(
                                "glassworks.viewctl.process_control.transition.{}|{}",
                                action.id, transition
                            ),
                            process_control_transition_label(transition),
                            false,
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn spc_fdc_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let monitor = spc_fdc_monitor(&app.workspace);
    let selected_chart = selected_spc_chart(app, &monitor)
        .map(|chart| chart.id.as_str().to_string())
        .or_else(|| app.selected_spc_chart.clone());
    let selected_trace = selected_fdc_trace(app, &monitor)
        .map(|trace| trace.id.clone())
        .or_else(|| app.selected_fdc_trace.clone());
    let chunk_size = if compact_rows { 2 } else { 4 };
    let mut rows = Vec::new();

    rows.push(
        SpcSeverityFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.spc.severity.{}", filter.slug()),
                    filter.label(),
                    app.spc_severity_filter == *filter,
                )
            })
            .collect(),
    );
    rows.push(
        SpcSourceFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.spc.source.{}", filter.slug()),
                    filter.label(),
                    app.spc_source_filter == *filter,
                )
            })
            .collect(),
    );

    for chunk in monitor
        .charts
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|chart| {
                    let id = chart.id.as_str();
                    ViewControlButton::new(
                        format!("glassworks.viewctl.spc.chart.{id}"),
                        format!(
                            "{} ({})",
                            compact_button_label(
                                &display_measurement_identifier(&chart.metric),
                                18
                            ),
                            chart.violations.len()
                        ),
                        selected_chart.as_deref() == Some(id),
                    )
                })
                .collect(),
        );
    }

    for chunk in monitor
        .traces
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|trace| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.spc.trace.{}", trace.id),
                        format!(
                            "{} ({})",
                            compact_button_label(
                                &display_measurement_identifier(&trace.sensor_name),
                                18
                            ),
                            trace.violations.len()
                        ),
                        selected_trace.as_deref() == Some(trace.id.as_str()),
                    )
                })
                .collect(),
        );
    }

    if !app.spc_context_filter.is_empty() {
        rows.push(vec![ViewControlButton::new(
            "glassworks.viewctl.spc.clear_context",
            "Clear context",
            true,
        )]);
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn cross_section_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in (0..cross_section_snapshot_count(&app.workspace))
        .take(8)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|step| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.cross_section.step.{step}"),
                        format!("Step {step}"),
                        app.cross_section_step == *step,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.cross_section.toggle_mask",
            "Mask",
            app.cross_section_show_mask,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.cross_section.toggle_dimensions",
            "Dims",
            app.cross_section_show_dimensions,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.cross_section.toggle_risks",
        "Risks",
        app.cross_section_show_risks,
    )]);
    for chunk in app
        .workspace
        .cross_section
        .materials
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|material| {
                    ViewControlButton::new(
                        format!(
                            "glassworks.viewctl.cross_section.material.{}",
                            material.id.as_str()
                        ),
                        material.name.clone(),
                        app.selected_cross_section_material.as_ref() == Some(&material.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn metrology_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 3 };
    for chunk in MetrologyMapMode::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|mode| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.metrology.mode.{}", mode.slug()),
                        mode.short_label(),
                        app.metrology_map_mode == *mode,
                    )
                })
                .collect(),
        );
    }
    for chunk in MeasurementKind::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|kind| {
                    ViewControlButton::new(
                        format!(
                            "glassworks.viewctl.metrology.kind.{}",
                            measurement_kind_slug(*kind)
                        ),
                        kind.label(),
                        app.metrology_kind == *kind,
                    )
                })
                .collect(),
        );
    }
    let utility_buttons = vec![
        ViewControlButton::new(
            "glassworks.viewctl.metrology.failed_only",
            "Failed",
            app.metrology_failed_only,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.metrology.next_attention",
            "Next",
            app.selected_die.is_some(),
        ),
        ViewControlButton::new("glassworks.viewctl.metrology.clear_die", "Clear", false),
    ];
    rows.extend(
        utility_buttons
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec()),
    );
    rows
}

pub(crate) fn yield_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    let chunk_size = if compact_rows { 2 } else { 4 };
    for chunk in YieldMapFilter::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.yield.filter.{}", filter.slug()),
                        filter.label(),
                        app.yield_map_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    let focus_buttons = vec![
        ViewControlButton::new(
            "glassworks.viewctl.yield.attention",
            "Attention",
            app.show_only_attention_wafers,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.yield.excursions",
            "Excursions",
            app.show_only_excursions,
        ),
    ];
    rows.extend(focus_buttons.chunks(chunk_size).map(|chunk| chunk.to_vec()));
    for chunk in app
        .workspace
        .yield_analysis
        .lots
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.yield.lot.{}", lot.id),
                        lot.id.clone(),
                        app.selected_yield_lot.as_deref() == Some(lot.id.as_str()),
                    )
                })
                .collect(),
        );
    }
    if let Some(lot_id) = app.selected_yield_lot.as_deref() {
        let wafers = app.workspace.yield_analysis.wafer_ids_for_lot(lot_id);
        for chunk in wafers.iter().take(4).collect::<Vec<_>>().chunks(chunk_size) {
            rows.push(
                chunk
                    .iter()
                    .map(|wafer_id| {
                        ViewControlButton::new(
                            format!("glassworks.viewctl.yield.wafer.{wafer_id}"),
                            wafer_id.to_string(),
                            app.selected_yield_wafer.as_deref() == Some(wafer_id.as_str()),
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn notebook_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let filtered_entries = notebook_filtered_entries(app);
    let chunk_size = if compact_rows { 2 } else { 4 };
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let primary_actions = vec![
        ViewControlButton::new(
            "glassworks.viewctl.notebook.preview",
            "Preview",
            app.notebook_preview_mode,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.notebook.edit",
            "Edit",
            !app.notebook_preview_mode,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.notebook.toggle_followups",
            if compact_rows {
                "Follow-ups".to_string()
            } else {
                format!("Follow-ups ({})", filtered_entries.len())
            },
            app.notebook_followups_only,
        ),
        ViewControlButton::new("glassworks.viewctl.notebook.clear_filters", "Clear", false),
    ];
    rows.extend(
        primary_actions
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec()),
    );
    for chunk in NotebookEntryAction::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|action| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.notebook.entry_action.{}", action.slug()),
                        if compact_rows {
                            compact_button_label(action.label(), 14)
                        } else {
                            action.label().to_string()
                        },
                        false,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.notebook.tag.all",
        "All tags",
        app.notebook_tag_filter.is_none(),
    )]);
    for chunk in app
        .workspace
        .lab_notebook
        .tags()
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|tag| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.notebook.tag.{tag}"),
                        format!("#{tag}"),
                        app.notebook_tag_filter.as_deref() == Some(tag.as_str()),
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.notebook.link.any",
        "Any link",
        app.notebook_link_kind_filter.is_none(),
    )]);
    for chunk in NotebookLinkKind::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|kind| {
                    let label = format!(
                        "{} ({})",
                        kind.label(),
                        notebook_link_count_for_kind(app, *kind)
                    );
                    ViewControlButton::new(
                        format!(
                            "glassworks.viewctl.notebook.link.{}",
                            notebook_link_kind_slug(*kind)
                        ),
                        if compact_rows {
                            compact_button_label(&label, 18)
                        } else {
                            label
                        },
                        app.notebook_link_kind_filter == Some(*kind),
                    )
                })
                .collect(),
        );
    }
    if let Some(entry) = selected_notebook_entry(app) {
        for chunk in notebook_entry_link_focus_tokens(entry)
            .iter()
            .take(4)
            .collect::<Vec<_>>()
            .chunks(chunk_size)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|(kind, value)| {
                        ViewControlButton::new(
                            format!(
                                "glassworks.viewctl.notebook.focus_link.{}:{}",
                                notebook_link_kind_slug(*kind),
                                value
                            ),
                            compact_button_label(&notebook_link_value_label(*kind, value), 14),
                            app.notebook_link_kind_filter == Some(*kind),
                        )
                    })
                    .collect(),
            );
        }
    }
    for chunk in filtered_entries
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|entry| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.notebook.entry.{}", entry.id),
                        compact_button_label(&entry.title, if compact_rows { 11 } else { 14 }),
                        app.selected_notebook_entry.as_ref() == Some(&entry.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

pub(crate) fn experiment_control_rows(
    app: &GlassworksApp,
    compact_rows: bool,
) -> Vec<Vec<ViewControlButton>> {
    let chunk_size = if compact_rows { 2 } else { 4 };
    let mut rows = Vec::new();
    let primary_actions = vec![
        ViewControlButton::new(
            "glassworks.viewctl.experiment.pending_only",
            "Pending",
            app.experiment_show_missing_only,
        ),
        ViewControlButton::new("glassworks.viewctl.experiment.next_pending", "Next", false),
        ViewControlButton::new("glassworks.viewctl.experiment.capture", "Capture", false),
        ViewControlButton::new(
            "glassworks.viewctl.experiment.capture_advance",
            "Advance",
            false,
        ),
        ViewControlButton::new("glassworks.viewctl.experiment.use_demo", "Sample", false),
        ViewControlButton::new("glassworks.viewctl.experiment.use_target", "Target", false),
        ViewControlButton::new(
            "glassworks.viewctl.experiment.capture_next_demo",
            "Next sample",
            false,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.experiment.clear_filters",
            if compact_rows {
                "Clear"
            } else {
                "Clear filters"
            },
            false,
        ),
    ];
    rows.extend(
        primary_actions
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec()),
    );

    for chunk in app
        .workspace
        .experiment_plan
        .responses
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|response| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.experiment.response.{}", response.id),
                        compact_button_label(&response.name, 14),
                        app.selected_experiment_response.as_ref() == Some(&response.id),
                    )
                })
                .collect(),
        );
    }

    for chunk in ExperimentRunFilter::ALL.chunks(chunk_size) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    let label = if compact_rows {
                        compact_button_label(filter.short_label(), 11)
                    } else {
                        filter.short_label().to_string()
                    };
                    ViewControlButton::new(
                        format!("glassworks.viewctl.experiment.filter.{}", filter.slug()),
                        label,
                        app.experiment_run_filter == *filter,
                    )
                })
                .collect(),
        );
    }

    for chunk in app
        .experiment_lot_options()
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.experiment.lot.{lot_id}"),
                        workflow_lot_label(
                            &app.workspace,
                            lot_id,
                            if compact_rows { 11 } else { 14 },
                        ),
                        app.experiment_lot_filter.as_deref() == Some(lot_id.as_str()),
                    )
                })
                .collect(),
        );
    }

    for chunk in app
        .filtered_experiment_run_rows()
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
    {
        rows.push(
            chunk
                .iter()
                .map(|row| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.experiment.run.{}", row.run_id),
                        experiment_run_order_label(row.run_order),
                        row.selected,
                    )
                })
                .collect(),
        );
    }

    rows.retain(|row| !row.is_empty());
    rows
}
