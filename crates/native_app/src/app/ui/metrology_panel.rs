#![allow(unused_imports)]
use super::*;

pub(crate) fn add_metrology_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let map = &app.workspace.wafer_map;
    if map.dies.is_empty() {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    }
    let summary = map.summary(app.metrology_kind);
    let attention_sites = metrology_attention_sites(map, app.metrology_kind);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.metrology.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(if compact_rows { 486.0 } else { 414.0 })),
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
        "glassworks.metrology.primary.title",
        if compact_rows {
            format!(
                "{} - {}",
                app.metrology_kind.label(),
                app.metrology_map_mode.label()
            )
        } else {
            format!(
                "{} - {} / {}",
                metrology_map_display_name(&app.workspace, map),
                app.metrology_map_mode.label(),
                app.metrology_kind.label()
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
            "glassworks.metrology.wafer_map",
            metrology_view_primitives(app, ui_scale),
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

    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.metrology.metrics",
            &[
                format!("{} samples", summary.sample_count),
                format!(
                    "{} pass / {} fail / {} outlier",
                    summary.pass_count, summary.fail_count, summary.outlier_count
                ),
                metrology_summary_label(summary),
                format!(
                    "{} defects / {} notes",
                    map.defects.len(),
                    map.annotations.len()
                ),
            ],
            ui_scale,
        );
    } else {
        let metrics = document.add_child(
            panel,
            UiNode::container(
                "glassworks.metrology.metrics",
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
            "glassworks.metrology.metrics.samples",
            format!("{} samples", summary.sample_count),
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.metrology.metrics.status",
            format!(
                "{} pass / {} fail / {} outlier",
                summary.pass_count, summary.fail_count, summary.outlier_count
            ),
            1.5,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.metrology.metrics.center",
            metrology_summary_label(summary),
            1.2,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.metrology.metrics.defects",
            format!(
                "{} defects / {} notes",
                map.defects.len(),
                map.annotations.len()
            ),
            1.0,
            false,
            ui_scale,
        );
    }

    let selected = document.add_child(
        panel,
        UiNode::container(
            "glassworks.metrology.selected",
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
        "glassworks.metrology.selected.die",
        compact_button_label(
            &metrology_selected_die_label(map, app.selected_die),
            if compact_rows { 54 } else { 96 },
        ),
        2.2,
        false,
        ui_scale,
    );
    if !compact_rows {
        add_primary_cell(
            document,
            selected,
            "glassworks.metrology.selected.filters",
            if app.metrology_failed_only {
                "Showing failed/outlier sites"
            } else {
                "Showing all measured dies"
            },
            1.0,
            false,
            ui_scale,
        );
    }

    let triage = document.add_child(
        panel,
        UiNode::container(
            "glassworks.metrology.triage",
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
    if attention_sites.is_empty() {
        add_primary_cell(
            document,
            triage,
            "glassworks.metrology.triage.clear",
            "No defects or measurement excursions",
            1.0,
            false,
            ui_scale,
        );
    } else {
        if compact_rows {
            for (row_index, chunk) in attention_sites
                .iter()
                .take(4)
                .collect::<Vec<_>>()
                .chunks(2)
                .enumerate()
            {
                let row = document.add_child(
                    triage,
                    UiNode::container(
                        format!("glassworks.metrology.triage.row.{row_index}"),
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
                for (die, score) in chunk {
                    add_button(
                        document,
                        row,
                        format!("glassworks.viewctl.metrology.die.{}|{}", die.column, die.row),
                        compact_button_label(
                            &metrology_attention_site_label(*die, *score, true),
                            18,
                        ),
                        app.selected_die == Some(*die),
                        primary_cell_layout(1.0),
                        ui_scale,
                    );
                }
            }
        } else {
            for (index, (die, score)) in attention_sites.iter().take(5).enumerate() {
                add_button(
                    document,
                    triage,
                    format!("glassworks.viewctl.metrology.die.{}|{}", die.column, die.row),
                    metrology_attention_site_label(*die, *score, false),
                    app.selected_die == Some(*die),
                    primary_cell_layout(1.0),
                    ui_scale,
                );
                if index == 4 {
                    break;
                }
            }
        }
    }
}

pub(crate) fn add_compact_metric_rows(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    items: &[String],
    ui_scale: UiScale,
) {
    let metrics = document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(70.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for (row_index, chunk) in items.chunks(2).enumerate() {
        let row = document.add_child(
            metrics,
            UiNode::container(
                format!("{name}.row.{row_index}"),
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
        for (column_index, text) in chunk.iter().enumerate() {
            add_primary_cell(
                document,
                row,
                format!("{name}.{row_index}.{column_index}"),
                compact_button_label(text, 20),
                1.0,
                false,
                ui_scale,
            );
        }
    }
}
