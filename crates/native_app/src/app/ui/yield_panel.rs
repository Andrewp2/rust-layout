#![allow(unused_imports)]
use super::*;

pub(crate) fn add_yield_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let analysis = &app.workspace.yield_analysis;
    let Some(lot_id) = app
        .selected_yield_lot
        .as_deref()
        .or_else(|| analysis.lots.first().map(|lot| lot.id.as_str()))
    else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let selected_wafer = app
        .selected_yield_wafer
        .as_deref()
        .map(str::to_string)
        .or_else(|| analysis.wafer_ids_for_lot(lot_id).first().cloned());
    let Some(wafer_id) = selected_wafer else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            view_primary_rows(app),
            ui_scale,
        );
        return;
    };
    let wafer_id = wafer_id.as_str();
    let lot_summary = analysis.lot_summary(lot_id);
    let wafer_summary = analysis.wafer_summary(lot_id, wafer_id);
    let outcomes = analysis.die_outcomes_for_wafer(lot_id, wafer_id);
    let measurements = analysis.measurements_for_wafer(lot_id, wafer_id);
    let wafer_rows = analysis.wafer_summaries_for_lot(lot_id);

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.yield.primary",
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
        "glassworks.yield.primary.title",
        if compact_rows {
            format!("{lot_id} / {wafer_id} / {}", app.yield_map_filter.label())
        } else {
            format!(
                "Yield dashboard - {lot_id} / {wafer_id} / {}",
                app.yield_map_filter.label()
            )
        },
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    document.add_child(
        panel,
        UiNode::scene(
            "glassworks.yield.wafer_map",
            yield_view_primitives(app, lot_id, wafer_id, &outcomes, ui_scale, compact_rows),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let yield_metric_items = [
        lot_summary
            .map(|summary| format!("lot {}", percent_label(summary.yield_fraction)))
            .unwrap_or_else(|| "lot n/a".to_string()),
        wafer_summary
            .map(|summary| {
                if compact_rows {
                    format!(
                        "wafer {} / {}F",
                        percent_label(summary.yield_fraction),
                        summary.failing_dies
                    )
                } else {
                    format!(
                        "wafer {} / {} fail",
                        percent_label(summary.yield_fraction),
                        summary.failing_dies
                    )
                }
            })
            .unwrap_or_else(|| "wafer n/a".to_string()),
        wafer_summary
            .or(lot_summary)
            .map(|summary| {
                if compact_rows {
                    yield_failure_compact_label(summary)
                } else {
                    yield_failure_label(summary)
                }
            })
            .unwrap_or_else(|| "No failure mode".to_string()),
        if compact_rows {
            format!(
                "{} meas / {} exc",
                measurements.len(),
                yield_excursion_count(&measurements)
            )
        } else {
            format!(
                "{} measurements / {} excursions",
                measurements.len(),
                yield_excursion_count(&measurements)
            )
        },
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.yield.metrics",
            &yield_metric_items,
            ui_scale,
        );
    } else {
        let metrics = document.add_child(
            panel,
            UiNode::container(
                "glassworks.yield.metrics",
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
            "glassworks.yield.metrics.lot",
            yield_metric_items[0].clone(),
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.yield.metrics.wafer",
            yield_metric_items[1].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.yield.metrics.failure",
            yield_metric_items[2].clone(),
            1.4,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.yield.metrics.measurements",
            yield_metric_items[3].clone(),
            1.0,
            false,
            ui_scale,
        );
    }

    let selected = document.add_child(
        panel,
        UiNode::container(
            "glassworks.yield.selected",
            layout::with_gap_all(
                layout::with_size(
                    if compact_rows {
                        layout::column()
                    } else {
                        layout::row()
                    },
                    layout::percent(1.0),
                    layout::px(ui_scale.value(if compact_rows { 64.0 } else { 42.0 })),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        selected,
        "glassworks.yield.selected.root_cause",
        wafer_summary
            .or(lot_summary)
            .map(|summary| {
                if compact_rows {
                    yield_root_cause_compact_label(summary)
                } else {
                    summary
                        .root_cause_hints
                        .first()
                        .map(|hint| compact_button_label(hint, 82))
                        .unwrap_or_else(|| "No root-cause hint".to_string())
                }
            })
            .unwrap_or_else(|| "No root-cause hint".to_string()),
        2.0,
        false,
        ui_scale,
    );
    if !compact_rows {
        add_primary_cell(
            document,
            selected,
            "glassworks.yield.selected.filters",
            yield_filter_summary(app),
            1.2,
            false,
            ui_scale,
        );
    }

    let wafers = document.add_child(
        panel,
        UiNode::container(
            "glassworks.yield.wafers",
            layout::with_gap_all(
                layout::with_size(
                    if compact_rows {
                        layout::column()
                    } else {
                        layout::row()
                    },
                    layout::percent(1.0),
                    layout::px(ui_scale.value(if compact_rows { 74.0 } else { 38.0 })),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    let visible_wafer_rows = wafer_rows
        .into_iter()
        .filter(|summary| yield_summary_matches(app, summary))
        .take(if compact_rows { 4 } else { 5 })
        .collect::<Vec<_>>();
    if compact_rows {
        for (row_index, chunk) in visible_wafer_rows.chunks(2).enumerate() {
            let row = document.add_child(
                wafers,
                UiNode::container(
                    format!("glassworks.yield.wafers.row.{row_index}"),
                    layout::with_gap_all(
                        layout::with_size(
                            layout::row(),
                            layout::percent(1.0),
                            layout::px(ui_scale.value(32.0)),
                        ),
                        ui_scale.value(8.0),
                    ),
                ),
            );
            for (column_index, summary) in chunk.iter().enumerate() {
                let Some(row_wafer_id) = summary.wafer_id.as_deref() else {
                    continue;
                };
                add_button(
                    document,
                    row,
                    format!(
                        "glassworks.primary.action.yield.{}_{}.yield.wafer.{row_wafer_id}",
                        row_index, column_index
                    ),
                    compact_button_label(
                        &format!("{} {}", row_wafer_id, percent_label(summary.yield_fraction)),
                        18,
                    ),
                    app.selected_yield_wafer.as_deref() == Some(row_wafer_id),
                    primary_cell_layout(1.0),
                    ui_scale,
                );
            }
        }
    } else {
        for (index, summary) in visible_wafer_rows.into_iter().enumerate() {
            let Some(row_wafer_id) = summary.wafer_id.as_deref() else {
                continue;
            };
            add_button(
                document,
                wafers,
                format!("glassworks.primary.action.yield.{index}.yield.wafer.{row_wafer_id}"),
                format!("{} {}", row_wafer_id, percent_label(summary.yield_fraction)),
                app.selected_yield_wafer.as_deref() == Some(row_wafer_id),
                primary_cell_layout(1.0),
                ui_scale,
            );
        }
    }
}
