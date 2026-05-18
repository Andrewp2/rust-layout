#![allow(unused_imports)]
use super::*;

pub(crate) fn add_spc_fdc_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let monitor = spc_fdc_monitor(&app.workspace);
    let selected_chart = selected_spc_chart(app, &monitor);
    let selected_trace = selected_fdc_trace(app, &monitor);
    let chart_violation_count = monitor
        .charts
        .iter()
        .map(|chart| chart.violations.len())
        .sum::<usize>();
    let trace_excursion_count = monitor
        .traces
        .iter()
        .map(|trace| trace.violations.len())
        .sum::<usize>();

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.spc.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(386.0)),
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
        "glassworks.spc.primary.title",
        if compact_rows {
            format!("SPC / FDC - {} findings", monitor.findings.len())
        } else {
            format!(
                "SPC / FDC monitor - {} findings across {} charts / {} traces",
                monitor.findings.len(),
                monitor.charts.len(),
                monitor.traces.len()
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
            "glassworks.spc.chart",
            spc_fdc_monitor_primitives(selected_chart, selected_trace, &monitor, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(226.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "glassworks.spc.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(46.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "glassworks.spc.metrics.critical",
        format!(
            "{} critical",
            monitor.finding_count_by_severity(MonitorSeverity::Critical)
        ),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "glassworks.spc.metrics.warning",
        format!(
            "{} warning",
            monitor.finding_count_by_severity(MonitorSeverity::Warning)
        ),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "glassworks.spc.metrics.spc",
        format!("{chart_violation_count} SPC rule hits"),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "glassworks.spc.metrics.fdc",
        format!("{trace_excursion_count} FDC excursions"),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "glassworks.spc.metrics.alarms",
        format!("{} active alarms", monitor.alarm_summary.active_count),
        1.0,
        false,
        ui_scale,
    );

    let selected = document.add_child(
        panel,
        UiNode::container(
            "glassworks.spc.selected",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(46.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        selected,
        "glassworks.spc.selected.chart",
        selected_chart
            .map(|chart| spc_chart_summary_label(chart, if compact_rows { 40 } else { 78 }))
            .unwrap_or_else(|| "No selected SPC chart".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "glassworks.spc.selected.trace",
        selected_trace
            .map(|trace| {
                spc_trace_summary_label(&monitor, trace, if compact_rows { 40 } else { 78 })
            })
            .unwrap_or_else(|| "No selected FDC trace".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "glassworks.spc.selected.filters",
        format!(
            "{} / {} / {} filtered",
            app.spc_severity_filter.label(),
            app.spc_source_filter.label(),
            spc_filtered_finding_count(app, &monitor)
        ),
        1.0,
        false,
        ui_scale,
    );
}
