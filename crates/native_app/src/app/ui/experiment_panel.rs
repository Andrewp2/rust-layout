#![allow(unused_imports)]
use super::*;

pub(crate) fn add_experiment_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let plan = &app.workspace.experiment_plan;
    if plan.runs.is_empty() && plan.factors.is_empty() && plan.responses.is_empty() {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    }
    let primary_response_id = app.selected_experiment_response_if_valid();
    let analysis = plan.analysis_summary(primary_response_id.as_ref());
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.experiment.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(if compact_rows { 500.0 } else { 414.0 })),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 29, 36, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(55, 69, 83, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.experiment.primary.title",
        experiment_plan_title_label(plan, compact_rows),
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(
            layout::percent(1.0),
            layout::px(ui_scale.value(if compact_rows { 28.0 } else { 24.0 })),
        ),
    );
    document.add_child(
        panel,
        UiNode::scene(
            "glassworks.experiment.matrix",
            experiment_view_primitives(
                plan,
                &analysis,
                app.experiment_show_missing_only,
                ui_scale,
                compact_rows,
                ui_scale.value(if compact_rows { 560.0 } else { 920.0 }),
            ),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(if compact_rows { 220.0 } else { 238.0 })),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 24, 30, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(50, 64, 78, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );

    let metric_items = [
        format!(
            "{} / {} complete",
            analysis.completed_runs, analysis.run_count
        ),
        format!("{} pending values", analysis.missing_response_count),
        format!(
            "{} factors / {} responses",
            plan.factors.len(),
            plan.responses.len()
        ),
        analysis
            .best_run_id
            .as_ref()
            .and_then(|run_id| plan.run(run_id))
            .map(|run| format!("best observed {}", display_experiment_run_label(run)))
            .unwrap_or_else(|| "best run pending".to_string()),
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.experiment.metrics",
            &metric_items,
            ui_scale,
        );
    } else {
        let metrics = document.add_child(
            panel,
            UiNode::container(
                "glassworks.experiment.metrics",
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
        add_primary_cell(
            document,
            metrics,
            "glassworks.experiment.metrics.runs",
            metric_items[0].clone(),
            0.9,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.experiment.metrics.pending",
            metric_items[1].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.experiment.metrics.scope",
            metric_items[2].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.experiment.metrics.best",
            metric_items[3].clone(),
            1.2,
            false,
            ui_scale,
        );
    }

    let note_items = [
        compact_button_label(
            &display_experiment_objective(&plan.objective),
            if compact_rows { 46 } else { 96 },
        ),
        experiment_readiness_label(&analysis, plan.responses.len()),
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.experiment.note",
            &note_items,
            ui_scale,
        );
    } else {
        let note_row = document.add_child(
            panel,
            UiNode::container(
                "glassworks.experiment.note",
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
        add_primary_cell(
            document,
            note_row,
            "glassworks.experiment.note.objective",
            note_items[0].clone(),
            2.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            note_row,
            "glassworks.experiment.note.readiness",
            note_items[1].clone(),
            1.0,
            false,
            ui_scale,
        );
    }

    if compact_rows {
        return;
    }

    let actions = document.add_child(
        panel,
        UiNode::container(
            "glassworks.experiment.actions",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, (action, label, selected)) in [
        (
            "experiment.pending_only",
            "Pending",
            app.experiment_show_missing_only,
        ),
        ("experiment.next_pending", "Next", false),
        ("experiment.capture_next_demo", "Next sample", false),
        ("experiment.use_target", "Target", false),
    ]
    .into_iter()
    .enumerate()
    {
        add_button(
            document,
            actions,
            format!("glassworks.primary.action.experiment.{index}.{action}"),
            label,
            selected,
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}
