#![allow(unused_imports)]
use super::*;

pub(crate) fn inventory_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let mut rows = Vec::new();
    for (index, alert) in app
        .workspace
        .inventory
        .alerts(INVENTORY_DEMO_TODAY)
        .into_iter()
        .take(3)
        .enumerate()
    {
        rows.push(
            PrimaryRow::new(
                format!("inventory-alert-{}-{index}", alert.lot_id),
                format!("{} {}", alert.kind.label(), alert.lot_id),
                alert.material_name,
                alert.message,
            )
            .action(
                format!("glassworks.viewctl.inventory.lot.{}", alert.lot_id),
                app.selected_inventory_lot.as_ref() == Some(&alert.lot_id),
            ),
        );
    }

    rows.extend(
        app.workspace
            .inventory
            .sorted_lots()
            .into_iter()
            .filter(|lot| inventory_filter_matches(app.inventory_filter, lot))
            .take(6usize.saturating_sub(rows.len()).max(2))
            .map(inventory_lot_primary_row(app)),
    );

    if let Some(lot) = selected_inventory_lot(app) {
        rows.extend(
            inventory_selected_lot_rows(app, lot)
                .into_iter()
                .take(10 - rows.len().min(10)),
        );
    }

    rows.truncate(10);
    rows
}

pub(crate) fn inventory_lot_primary_row<'a>(
    app: &'a GlassworksApp,
) -> impl Fn(&'a MaterialLot) -> PrimaryRow {
    |lot| {
        let expiry = lot
            .expires_on
            .map(layout_model::inventory::format_date)
            .map(|date| format!("expires {date}"))
            .unwrap_or_else(|| "no expiry".to_string());
        PrimaryRow::new(
            format!("inventory-{}", lot.id),
            format!("{} {}", lot.id, inventory_material_status_label(lot)),
            lot.material_name.clone(),
            format!(
                "{} {}; {}; {}",
                lot.stock.format(),
                lot.category.label(),
                lot.location.label(),
                expiry
            ),
        )
        .action(
            format!("glassworks.viewctl.inventory.lot.{}", lot.id),
            app.selected_inventory_lot.as_ref() == Some(&lot.id),
        )
    }
}

pub(crate) fn inventory_selected_lot_rows(
    _app: &GlassworksApp,
    lot: &MaterialLot,
) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            format!("inventory-detail-stock-{}", lot.id),
            "Stock",
            lot.stock.format(),
            format!("reorder at {}", lot.reorder_threshold.format()),
        ),
        PrimaryRow::new(
            format!("inventory-detail-expiration-{}", lot.id),
            "Expiration",
            inventory_expiration_label(lot),
            inventory_release_cue(lot),
        ),
        PrimaryRow::new(
            format!("inventory-detail-location-{}", lot.id),
            "Location",
            lot.location.label(),
            format!(
                "cabinet {} bin {} {}",
                lot.location.cabinet, lot.location.bin, lot.location.temperature
            ),
        ),
        PrimaryRow::new(
            format!("inventory-detail-supplier-{}", lot.id),
            "Supplier",
            lot.supplier.supplier.clone(),
            format!(
                "lot {} received {} by {}",
                lot.supplier.supplier_lot,
                layout_model::inventory::format_date(lot.supplier.received_date),
                display_inventory_actor_identifier(&lot.supplier.received_by)
            ),
        ),
    ];
    rows.extend(lot.usage.iter().take(3).enumerate().map(|(index, usage)| {
        PrimaryRow::new(
            format!("inventory-usage-{}-{index}", lot.id),
            format!(
                "{} {}",
                layout_model::inventory::format_date(usage.timestamp),
                usage.quantity.format()
            ),
            display_inventory_actor_identifier(&usage.actor),
            format!(
                "{}; {}",
                usage
                    .links
                    .iter()
                    .map(|link| link.label())
                    .collect::<Vec<_>>()
                    .join(", "),
                usage.note
            ),
        )
    }));
    if let Some(certificate_id) = &lot.supplier.certificate_id {
        rows.push(PrimaryRow::new(
            format!("inventory-certificate-{}", lot.id),
            "Certificate",
            certificate_id.clone(),
            lot.supplier
                .certificate_url
                .clone()
                .unwrap_or_else(|| "no record URL".to_string()),
        ));
    }
    rows
}

pub(crate) fn inventory_material_status_label(lot: &MaterialLot) -> &'static str {
    if lot.is_expired(INVENTORY_DEMO_TODAY) {
        "expired"
    } else if lot.is_low_stock() {
        "low"
    } else if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
        "expiring"
    } else if !lot.usage.is_empty() {
        "in use"
    } else {
        "released"
    }
}

pub(crate) fn inventory_lot_button_label(lot: &MaterialLot, max_chars: usize) -> String {
    compact_button_label(
        &format!(
            "{} {}",
            lot.material_name,
            inventory_material_status_label(lot)
        ),
        max_chars,
    )
}

pub(crate) fn inventory_expiration_label(lot: &MaterialLot) -> String {
    lot.expires_on
        .map(layout_model::inventory::format_date)
        .map(|date| {
            if lot.is_expired(INVENTORY_DEMO_TODAY) {
                format!("expired {date}")
            } else if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
                format!("expires soon {date}")
            } else {
                format!("expires {date}")
            }
        })
        .unwrap_or_else(|| "no expiry".to_string())
}

pub(crate) fn inventory_release_cue(lot: &MaterialLot) -> String {
    let mut cues = Vec::new();
    if lot.is_expired(INVENTORY_DEMO_TODAY) {
        cues.push(InventoryAlertKind::Expired.label());
    }
    if lot.is_low_stock() {
        cues.push(InventoryAlertKind::LowStock.label());
    }
    if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
        cues.push(InventoryAlertKind::ExpiringSoon.label());
    }
    if cues.is_empty() {
        "released for use".to_string()
    } else {
        cues.join(", ")
    }
}

pub(crate) fn maintenance_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let today = maintenance_today(&app.workspace);
    let mut rows = Vec::new();
    for tool in &app.workspace.maintenance.tools {
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
            rows.push(
                PrimaryRow::new(
                    format!("maint-{}-{}", tool.tool_id, schedule.id),
                    format!("{} {}", tool.tool_id, schedule.task),
                    due_state.label(),
                    format!(
                        "{} due {}; runs {}",
                        schedule.kind.label(),
                        schedule.next_due,
                        tool.run_count
                    ),
                )
                .action(
                    format!("glassworks.viewctl.maintenance.tool.{}", tool.tool_id),
                    app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
                ),
            );
            if rows.len() >= 4 {
                break;
            }
        }
        if rows.len() >= 4 {
            break;
        }
    }

    if let Some(tool) = selected_maintenance_tool(app) {
        rows.extend(
            maintenance_selected_tool_rows(app, tool, today)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }

    rows.extend(
        maintenance_release_rows(app, today)
            .into_iter()
            .take(10usize.saturating_sub(rows.len())),
    );

    rows.extend(
        maintenance_history_rows(app)
            .into_iter()
            .take(10usize.saturating_sub(rows.len())),
    );

    if rows.is_empty() {
        rows.extend(app.workspace.maintenance.tools.iter().take(10).map(|tool| {
            PrimaryRow::new(
                format!("maint-{}", tool.tool_id),
                format!("{} {}", tool.tool_id, tool.tool_name),
                format!("{} schedules", tool.schedules.len()),
                format!("{} tool runs", tool.run_count),
            )
            .action(
                format!("glassworks.viewctl.maintenance.tool.{}", tool.tool_id),
                app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
            )
        }));
    }
    rows.truncate(10);
    rows
}

pub(crate) fn maintenance_selected_tool_rows(
    app: &GlassworksApp,
    tool: &layout_model::maintenance::ToolMaintenanceState,
    today: FabDate,
) -> Vec<PrimaryRow> {
    let release = app
        .workspace
        .maintenance
        .release_for_tool(&tool.tool_id, today);
    let mut rows = vec![
        PrimaryRow::new(
            format!("maint-selected-{}", tool.tool_id),
            tool.tool_name.clone(),
            format!("{} runs", tool.run_count),
            release.state.label().to_string(),
        )
        .action(
            format!("glassworks.viewctl.maintenance.tool.{}", tool.tool_id),
            app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
        ),
    ];
    if release.reasons.is_empty() {
        rows.push(PrimaryRow::new(
            format!("maint-release-context-{}", tool.tool_id),
            "Release context",
            "No active holds",
            "released workflow is clear",
        ));
    } else {
        rows.extend(
            release
                .reasons
                .iter()
                .take(2)
                .enumerate()
                .map(|(index, reason)| {
                    PrimaryRow::new(
                        format!("maint-release-hold-{}-{index}", tool.tool_id),
                        format!("Hold {}", index + 1),
                        release.state.label(),
                        reason.clone(),
                    )
                }),
        );
    }
    if let Some(next) = tool
        .schedules
        .iter()
        .min_by_key(|schedule| (schedule.next_due, schedule.id.clone()))
    {
        let due_state = next.due_state(today, tool.run_count);
        rows.push(PrimaryRow::new(
            format!("maint-next-{}-{}", tool.tool_id, next.id),
            format!("Next {}", next.task),
            due_state.label(),
            format!(
                "due {}; {} checklist item(s)",
                next.next_due,
                next.checklist.len()
            ),
        ));
        if !next.checklist.is_empty() {
            rows.push(PrimaryRow::new(
                format!("maint-checklist-{}-{}", tool.tool_id, next.id),
                "Checklist",
                format!("{} items", next.checklist.len()),
                next.checklist
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
    }
    if let Some(record) = app
        .workspace
        .maintenance
        .calibration_records_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|record| record.performed_at)
    {
        rows.push(PrimaryRow::new(
            format!("maint-calibration-{}", record.id),
            "Latest calibration",
            record.outcome.label(),
            format!(
                "{} {} {:.2} vs {} by {}",
                record.performed_at,
                record.parameter,
                record.measured_value,
                record.tolerance,
                record.technician
            ),
        ));
    }
    if let Some(result) = app
        .workspace
        .maintenance
        .qualification_results_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|result| result.performed_at)
    {
        rows.push(PrimaryRow::new(
            format!("maint-qualification-{}", result.id),
            "Latest qualification",
            result.outcome.label(),
            format!(
                "{} {} {} {:.2} ({})",
                result.performed_at, result.wafer_id, result.metric, result.value, result.spec
            ),
        ));
    }
    rows
}

pub(crate) fn maintenance_release_rows(app: &GlassworksApp, today: FabDate) -> Vec<PrimaryRow> {
    app.workspace
        .maintenance
        .tools
        .iter()
        .map(|tool| {
            let release = app
                .workspace
                .maintenance
                .release_for_tool(&tool.tool_id, today);
            PrimaryRow::new(
                format!("maint-release-{}", tool.tool_id),
                format!("{} {}", tool.tool_id, release.state.label()),
                tool.tool_name.clone(),
                if release.reasons.is_empty() {
                    format!("{} runs; no release holds", tool.run_count)
                } else {
                    format!(
                        "{} runs; {}",
                        tool.run_count,
                        release
                            .reasons
                            .iter()
                            .take(2)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                },
            )
            .action(
                format!("glassworks.viewctl.maintenance.tool.{}", tool.tool_id),
                app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
            )
        })
        .collect()
}

pub(crate) fn maintenance_history_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    let selected = app.selected_maintenance_tool.as_ref();
    let mut rows = Vec::new();
    rows.extend(
        app.workspace
            .maintenance
            .calibration_records
            .iter()
            .filter(|record| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&record.tool_id)
            })
            .take(3)
            .map(|record| {
                PrimaryRow::new(
                    format!("maint-history-cal-{}", record.id),
                    format!("{} calibration {}", record.performed_at, record.tool_id),
                    record.outcome.label(),
                    format!(
                        "{} {} by {}",
                        record.instrument, record.parameter, record.technician
                    ),
                )
                .action(
                    format!("glassworks.viewctl.maintenance.tool.{}", record.tool_id),
                    selected == Some(&record.tool_id),
                )
            }),
    );
    rows.extend(
        app.workspace
            .maintenance
            .qualification_results
            .iter()
            .filter(|result| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&result.tool_id)
            })
            .take(3)
            .map(|result| {
                PrimaryRow::new(
                    format!("maint-history-qual-{}", result.id),
                    format!("{} qual {}", result.performed_at, result.tool_id),
                    result.outcome.label(),
                    format!("{} {} {:.2}", result.wafer_id, result.metric, result.value),
                )
                .action(
                    format!("glassworks.viewctl.maintenance.tool.{}", result.tool_id),
                    selected == Some(&result.tool_id),
                )
            }),
    );
    rows.extend(
        app.workspace
            .maintenance
            .downtime
            .iter()
            .filter(|record| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&record.tool_id)
            })
            .take(3)
            .map(|record| {
                PrimaryRow::new(
                    format!("maint-history-down-{}", record.id),
                    format!("{} downtime {}", record.started_at, record.tool_id),
                    record
                        .ended_at
                        .map(|date| format!("ended {date}"))
                        .unwrap_or_else(|| "open".to_string()),
                    format!("{}; owner {}", record.reason, record.owner),
                )
                .action(
                    format!("glassworks.viewctl.maintenance.tool.{}", record.tool_id),
                    selected == Some(&record.tool_id),
                )
            }),
    );
    rows
}

pub(crate) fn environment_primary_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    app.workspace
        .environment
        .sensors
        .iter()
        .take(10)
        .map(|sensor| {
            let tags = if sensor.process_tags.is_empty() {
                "untagged".to_string()
            } else {
                sensor.process_tags.join(", ")
            };
            PrimaryRow::new(
                format!("sensor-{}", sensor.id),
                display_environment_sensor_label(sensor),
                environment_sensor_status_for(&app.workspace, sensor),
                format!("{}; {}; {}", sensor.zone, sensor.kind.label(), tags),
            )
            .action(
                format!("glassworks.viewctl.environment.sensor.{}", sensor.id),
                app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
            )
        })
        .collect()
}
