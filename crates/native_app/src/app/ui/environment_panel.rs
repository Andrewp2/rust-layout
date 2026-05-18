#![allow(unused_imports)]
use super::*;

pub(crate) fn add_environment_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let environment = &app.workspace.environment;
    let Some(sensor) = selected_environment_sensor(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let active_alarm_count = environment.active_alarms().len();
    let zones = environment_zone_summaries(environment);
    let panel_height = if compact_rows { 490.0 } else { 414.0 };
    let trend_height = if compact_rows { 190.0 } else { 238.0 };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.environment.primary",
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
        "glassworks.environment.primary.title",
        if compact_rows {
            format!(
                "Cleanroom - {} sensors / {} zones",
                environment.sensors.len(),
                zones.len()
            )
        } else {
            format!(
                "Cleanroom environment - {} sensors / {} zones / {} active alarms",
                environment.sensors.len(),
                zones.len(),
                active_alarm_count
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
            "glassworks.environment.trend",
            environment_view_primitives(app, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(trend_height)),
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

    let selected_metric = if compact_rows {
        environment
            .latest_reading(&sensor.id)
            .map(|reading| {
                format!(
                    "{} / {}",
                    environment_sensor_button_label(sensor),
                    environment_alarm_short_label(sensor.thresholds.evaluate(reading.value))
                )
            })
            .unwrap_or_else(|| format!("{} / no data", environment_sensor_button_label(sensor)))
    } else {
        environment_selected_sensor_label(environment, sensor)
    };
    let trend_label = environment
        .trend_summary(&sensor.id)
        .map(|summary| {
            if compact_rows {
                format!(
                    "{} {:.2} {}",
                    summary.direction.label(),
                    summary.average_value,
                    sensor.unit
                )
            } else {
                format!(
                    "{} avg {:.2} {}",
                    summary.direction.label(),
                    summary.average_value,
                    sensor.unit
                )
            }
        })
        .unwrap_or_else(|| "No trend".to_string());
    let metric_items = [
        selected_metric,
        trend_label,
        if compact_rows {
            format!(
                "{} readings / {} ev",
                environment.readings.len(),
                environment.events.len()
            )
        } else {
            format!(
                "{} readings / {} events",
                environment.readings.len(),
                environment.events.len()
            )
        },
        if compact_rows {
            format!(
                "{active_alarm_count} active / {} alarms",
                environment.alarms.len()
            )
        } else {
            format!(
                "{} active / {} total alarms",
                active_alarm_count,
                environment.alarms.len()
            )
        },
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.environment.metrics",
            &metric_items,
            ui_scale,
        );
    } else {
        let metrics = document.add_child(
            panel,
            UiNode::container(
                "glassworks.environment.metrics",
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
            "glassworks.environment.metrics.selected",
            metric_items[0].clone(),
            1.4,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.environment.metrics.trend",
            metric_items[1].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.environment.metrics.readings",
            metric_items[2].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            metrics,
            "glassworks.environment.metrics.alarms",
            metric_items[3].clone(),
            1.0,
            false,
            ui_scale,
        );
    }

    let detail_items = if compact_rows {
        [
            zones
                .iter()
                .find(|zone| zone.name == sensor.zone)
                .map(|zone| format!("{} / {} sens", zone.name, zone.sensor_count))
                .unwrap_or_else(|| format!("{} / no data", sensor.zone)),
            "Nominal band".to_string(),
            environment_correlation_label(environment, &sensor.id)
                .replace(" candidate", "")
                .replace(" labels", ""),
        ]
    } else {
        [
            environment_selected_zone_label(&zones, &sensor.zone),
            format!("Nominal {}", sensor.thresholds.nominal_label(&sensor.unit)),
            environment_correlation_label(environment, &sensor.id),
        ]
    };
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.environment.detail",
            &detail_items,
            ui_scale,
        );
    } else {
        let detail = document.add_child(
            panel,
            UiNode::container(
                "glassworks.environment.detail",
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
            detail,
            "glassworks.environment.detail.zone",
            detail_items[0].clone(),
            1.3,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            detail,
            "glassworks.environment.detail.nominal",
            detail_items[1].clone(),
            1.1,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            detail,
            "glassworks.environment.detail.correlation",
            detail_items[2].clone(),
            1.2,
            false,
            ui_scale,
        );
    }

    let sensors = document.add_child(
        panel,
        UiNode::container(
            "glassworks.environment.sensors",
            layout::with_gap_all(
                layout::with_size(
                    if compact_rows {
                        layout::column()
                    } else {
                        layout::row()
                    },
                    layout::percent(1.0),
                    layout::px(ui_scale.value(if compact_rows { 70.0 } else { 38.0 })),
                ),
                ui_scale.value(if compact_rows { 6.0 } else { 8.0 }),
            ),
        ),
    );
    if compact_rows {
        for (row_index, chunk) in environment
            .sensors
            .iter()
            .take(5)
            .enumerate()
            .collect::<Vec<_>>()
            .chunks(1)
            .enumerate()
        {
            let row = document.add_child(
                sensors,
                UiNode::container(
                    format!("glassworks.environment.sensors.row.{row_index}"),
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
            for (index, sensor) in chunk {
                add_button(
                    document,
                    row,
                    format!(
                        "glassworks.primary.action.environment.{index}.environment.sensor.{}",
                        sensor.id
                    ),
                    compact_button_label(&display_environment_sensor_label(sensor), 18),
                    app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
                    primary_cell_layout(1.0),
                    ui_scale,
                );
            }
        }
    } else {
        for (index, sensor) in environment.sensors.iter().take(5).enumerate() {
            add_button(
                document,
                sensors,
                format!(
                    "glassworks.primary.action.environment.{index}.environment.sensor.{}",
                    sensor.id
                ),
                compact_button_label(&display_environment_sensor_label(sensor), 28),
                app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
                primary_cell_layout(1.0),
                ui_scale,
            );
        }
    }
}
