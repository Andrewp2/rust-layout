#![allow(unused_imports)]
use super::*;

pub(crate) fn add_inventory_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let visible_lots = inventory_visible_lots(app);
    let alerts = app.workspace.inventory.alerts(INVENTORY_DEMO_TODAY);
    let selected_lot = selected_inventory_lot(app);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.inventory.overview",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(if compact_rows { 860.0 } else { 632.0 })),
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
        "glassworks.inventory.title",
        "Materials control - Inventory Tracker",
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    add_text(
        document,
        panel,
        "glassworks.inventory.subtitle",
        format!(
            "{} visible of {} lots / {} alert(s)",
            visible_lots.len(),
            app.workspace.inventory.lots.len(),
            alerts.len()
        ),
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );

    add_inventory_summary_row(document, panel, app, ui_scale, compact_rows);
    add_inventory_alert_section(document, panel, &alerts, ui_scale, compact_rows);

    let workbench = document.add_child(
        panel,
        UiNode::container(
            "glassworks.inventory.workbench",
            layout::with_gap_all(
                layout::with_size(
                    if compact_rows {
                        layout::column()
                    } else {
                        layout::row()
                    },
                    layout::percent(1.0),
                    layout::px(ui_scale.value(if compact_rows { 520.0 } else { 330.0 })),
                ),
                ui_scale.value(10.0),
            ),
        ),
    );
    add_inventory_lot_table(
        document,
        workbench,
        app,
        &visible_lots,
        ui_scale,
        compact_rows,
    );
    add_inventory_detail_panel(document, workbench, app, selected_lot, ui_scale);
}

pub(crate) fn inventory_visible_lots(app: &GlassworksApp) -> Vec<&MaterialLot> {
    app.workspace
        .inventory
        .sorted_lots()
        .into_iter()
        .filter(|lot| inventory_filter_matches(app.inventory_filter, lot))
        .collect()
}

pub(crate) fn inventory_usage_count(app: &GlassworksApp) -> usize {
    app.workspace
        .inventory
        .lots
        .values()
        .map(|lot| lot.usage.len())
        .sum()
}

pub(crate) fn inventory_location_count(app: &GlassworksApp) -> usize {
    app.workspace
        .inventory
        .lots
        .values()
        .map(|lot| lot.location.area.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

pub(crate) fn add_inventory_summary_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    if compact_rows {
        add_compact_metric_rows(
            document,
            parent,
            "glassworks.inventory.summary",
            &[
                format!("Lots: {}", app.workspace.inventory.lots.len()),
                format!(
                    "Action: {} / Hold: {}",
                    inventory_filter_count(&app.workspace, InventoryQuickFilter::NeedsAction),
                    inventory_filter_count(&app.workspace, InventoryQuickFilter::ProductHold)
                ),
                format!("Locations: {}", inventory_location_count(app)),
                format!("Usage links: {}", inventory_usage_count(app)),
            ],
            ui_scale,
        );
        return;
    }
    let summary_items = [
        format!("Material lots: {}", app.workspace.inventory.lots.len()),
        format!(
            "Action required: {}",
            inventory_filter_count(&app.workspace, InventoryQuickFilter::NeedsAction)
        ),
        format!(
            "Product hold: {}",
            inventory_filter_count(&app.workspace, InventoryQuickFilter::ProductHold)
        ),
        format!("Locations: {}", inventory_location_count(app)),
        format!("Usage links: {}", inventory_usage_count(app)),
    ];
    let row = document.add_child(
        parent,
        UiNode::container(
            "glassworks.inventory.summary",
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
            format!("glassworks.inventory.summary.{index}"),
            text,
            1.0,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_inventory_alert_section(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    alerts: &[layout_model::inventory::InventoryAlert],
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let visible_alerts = alerts.len().max(1).min(2);
    let section_height = if compact_rows {
        ui_scale.value(30.0) + ui_scale.value(78.0) * visible_alerts as f32
    } else {
        ui_scale.value(104.0)
    };
    let section = document.add_child(
        parent,
        UiNode::container(
            "glassworks.inventory.alerts",
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(section_height),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_text(
        document,
        section,
        "glassworks.inventory.alerts.title",
        "Alerts",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    if alerts.is_empty() {
        add_primary_cell(
            document,
            section,
            "glassworks.inventory.alerts.empty",
            "No low-stock or expiration alerts",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, alert) in alerts.iter().take(2).enumerate() {
            add_inventory_alert_card(document, section, alert, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        section,
        "glassworks.inventory.alerts.header",
        &[
            ("Alert", 0.8),
            ("Material", 1.3),
            ("Lot", 0.8),
            ("Message", 2.2),
        ],
        ui_scale,
    );
    for (index, alert) in alerts.iter().take(2).enumerate() {
        let row = add_inventory_table_row(
            document,
            section,
            format!("glassworks.inventory.alerts.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.alerts.row.{index}.kind"),
            alert.kind.label(),
            0.8,
            false,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.invalert{index}.inventory.lot.{}",
                alert.lot_id
            ),
            alert.lot_id.to_string(),
            false,
            primary_cell_layout(0.8),
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.alerts.row.{index}.material"),
            alert.lot_id.to_string(),
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.alerts.row.{index}.message"),
            compact_button_label(&alert.message, 72),
            2.2,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_inventory_alert_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    alert: &layout_model::inventory::InventoryAlert,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.inventory.alerts.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(72.0)),
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
    let top = document.add_child(
        row,
        UiNode::container(
            format!("{row_name}.top"),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_primary_text_cell(
        document,
        top,
        format!("{row_name}.kind"),
        alert.kind.label(),
        false,
        ui_scale,
        layout::with_padding_all(primary_cell_layout(0.8), ui_scale.value(7.0)),
    );
    add_button(
        document,
        top,
        format!(
            "glassworks.primary.action.invalert{index}.inventory.lot.{}",
            alert.lot_id
        ),
        compact_button_label(&alert.material_name, 28),
        false,
        primary_cell_layout(1.2),
        ui_scale,
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.detail"),
        compact_button_label(&format!("{} - {}", alert.material_name, alert.message), 54),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(30.0))),
            ui_scale.value(7.0),
        ),
    );
}

pub(crate) fn add_inventory_lot_table(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    visible_lots: &[&MaterialLot],
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let table_layout = if compact_rows {
        layout::with_size(
            layout::with_flex(layout::column(), 1.65, 1.0, layout::px(0.0)),
            layout::percent(1.0),
            layout::px(ui_scale.value(330.0)),
        )
    } else {
        layout::with_size(
            layout::with_flex(layout::column(), 1.65, 1.0, layout::px(0.0)),
            layout::auto(),
            layout::percent(1.0),
        )
    };
    let table = document.add_child(
        parent,
        UiNode::container(
            "glassworks.inventory.lots",
            layout::with_padding_all(
                layout::with_gap_all(table_layout, ui_scale.value(6.0)),
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
        table,
        "glassworks.inventory.lots.title",
        "Material Lots",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    if visible_lots.is_empty() {
        add_primary_cell(
            document,
            table,
            "glassworks.inventory.lots.empty",
            "No material lots match this filter",
            1.0,
            false,
            ui_scale,
        );
        return;
    }
    if compact_rows {
        for (index, lot) in visible_lots.iter().take(7).enumerate() {
            add_inventory_lot_card(document, table, app, lot, index, ui_scale);
        }
        return;
    }
    add_inventory_table_header(
        document,
        table,
        "glassworks.inventory.lots.header",
        &[
            ("Status", 0.85),
            ("Lot", 0.9),
            ("Material", 1.6),
            ("Stock", 1.0),
            ("Expires", 1.2),
            ("Location", 1.6),
        ],
        ui_scale,
    );
    for (index, lot) in visible_lots.iter().take(7).enumerate() {
        let row = add_inventory_table_row(
            document,
            table,
            format!("glassworks.inventory.lots.row.{index}"),
            index,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.lots.row.{index}.status"),
            inventory_material_status_label(lot),
            0.85,
            false,
            ui_scale,
        );
        add_button(
            document,
            row,
            format!(
                "glassworks.primary.action.invlot{index}.inventory.lot.{}",
                lot.id
            ),
            lot.id.to_string(),
            app.selected_inventory_lot.as_ref() == Some(&lot.id),
            primary_cell_layout(0.9),
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.lots.row.{index}.material"),
            compact_button_label(&lot.material_name, 28),
            1.6,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.lots.row.{index}.stock"),
            inventory_stock_label(lot),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.lots.row.{index}.expires"),
            inventory_expiration_label(lot),
            1.2,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.inventory.lots.row.{index}.location"),
            compact_button_label(&lot.location.label(), 32),
            1.6,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn add_inventory_lot_card(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    lot: &MaterialLot,
    index: usize,
    ui_scale: UiScale,
) {
    let row_name = format!("glassworks.inventory.lots.row.{index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(82.0)),
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
            "glassworks.primary.action.invlot{index}.inventory.lot.{}",
            lot.id
        ),
        compact_button_label(
            &format!("{} {}", lot.id, inventory_material_status_label(lot)),
            44,
        ),
        app.selected_inventory_lot.as_ref() == Some(&lot.id),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(32.0))),
        ui_scale,
    );
    add_primary_text_cell(
        document,
        row,
        format!("{row_name}.material"),
        compact_button_label(&lot.material_name, 54),
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
        format!("{row_name}.stock"),
        compact_button_label(
            &format!(
                "{} / {} / {}",
                inventory_stock_label(lot),
                inventory_expiration_label(lot),
                lot.location.label()
            ),
            46,
        ),
        false,
        ui_scale,
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale.value(7.0),
        ),
    );
}

pub(crate) fn add_inventory_detail_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    _app: &GlassworksApp,
    lot: Option<&MaterialLot>,
    ui_scale: UiScale,
) {
    let detail = document.add_child(
        parent,
        UiNode::container(
            "glassworks.inventory.detail",
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
        detail,
        "glassworks.inventory.detail.title",
        "Selected Material",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    let Some(lot) = lot else {
        add_primary_cell(
            document,
            detail,
            "glassworks.inventory.detail.empty",
            "No material lot selected",
            1.0,
            false,
            ui_scale,
        );
        return;
    };

    for (index, (label, value)) in [
        ("Lot", lot.id.to_string()),
        ("Material", lot.material_name.clone()),
        ("Status", inventory_release_cue(lot)),
        ("Stock", inventory_stock_label(lot)),
        ("Location", lot.location.label()),
        ("Supplier", lot.supplier.supplier.clone()),
        ("Supplier lot", lot.supplier.supplier_lot.clone()),
    ]
    .into_iter()
    .enumerate()
    {
        add_primary_cell(
            document,
            detail,
            format!("glassworks.inventory.detail.row.{index}"),
            format!("{label}: {value}"),
            1.0,
            false,
            ui_scale,
        );
    }

    add_text(
        document,
        detail,
        "glassworks.inventory.usage.title",
        "Usage History",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    if lot.usage.is_empty() {
        add_primary_cell(
            document,
            detail,
            "glassworks.inventory.usage.empty",
            "No usage linked",
            1.0,
            false,
            ui_scale,
        );
    } else {
        for (index, usage) in lot.usage.iter().take(4).enumerate() {
            let links = usage
                .links
                .iter()
                .map(|link| inventory_usage_link_label(link))
                .collect::<Vec<_>>()
                .join(", ");
            add_primary_cell(
                document,
                detail,
                format!("glassworks.inventory.usage.row.{index}"),
                compact_button_label(
                    &format!(
                        "{} {} by {}; {}",
                        layout_model::inventory::format_date(usage.timestamp),
                        usage.quantity.format(),
                        display_inventory_actor_identifier(&usage.actor),
                        links
                    ),
                    58,
                ),
                1.0,
                false,
                ui_scale,
            );
        }
    }
}

pub(crate) fn inventory_stock_label(lot: &MaterialLot) -> String {
    if lot.is_low_stock() {
        format!(
            "{} <= {}",
            lot.stock.format(),
            lot.reorder_threshold.format()
        )
    } else {
        lot.stock.format()
    }
}

pub(crate) fn add_inventory_table_header(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    columns: &[(&str, f32)],
    ui_scale: UiScale,
) {
    let name = name.into();
    let header = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    for (index, (label, weight)) in columns.iter().enumerate() {
        add_primary_cell(
            document,
            header,
            format!("{name}.col.{index}"),
            *label,
            *weight,
            true,
            ui_scale,
        );
    }
}

pub(crate) fn add_inventory_table_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    index: usize,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(6.0),
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
    )
}
