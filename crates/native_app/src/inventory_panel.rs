use std::collections::BTreeMap;

use eframe::egui::{self, Color32, RichText};
use layout_model::inventory::{
    Inventory, InventoryAlert, InventoryAlertKind, MaterialLot, MaterialLotId, format_date,
};

use crate::ui_chrome::{self, Tone};

const DEMO_TODAY: u32 = 20260508;

pub(crate) struct InventoryPanel {
    inventory: Inventory,
    selected_lot: Option<MaterialLotId>,
    quick_filter: InventoryQuickFilter,
}

impl InventoryPanel {
    pub(crate) fn from_inventory(inventory: Inventory) -> Self {
        let selected_lot = inventory.sorted_lots().first().map(|lot| lot.id.clone());
        Self {
            inventory,
            selected_lot,
            quick_filter: InventoryQuickFilter::All,
        }
    }

    pub(crate) fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.ensure_selection();
        egui::ScrollArea::vertical()
            .id_salt("inventory_panel")
            .show(ui, |ui| {
                ui_chrome::module_header(ui, "Materials control", "Inventory Tracker", "", |ui| {
                    self.lot_picker(ui, status);
                });

                self.summary_ui(ui, status);
                ui.separator();
                self.quick_filter_ui(ui, status);
                ui.separator();
                if ui.available_width() < 960.0 {
                    self.lot_table_ui(ui, status);
                    ui.separator();
                    if let Some(lot) = self.selected_lot().cloned() {
                        self.detail_ui(ui, &lot);
                    } else {
                        ui_chrome::empty_state(ui, "No material lot selected");
                    }
                } else {
                    ui.columns(2, |columns| {
                        columns[0].set_min_width(520.0);
                        columns[1].set_min_width(360.0);
                        self.lot_table_ui(&mut columns[0], status);
                        if let Some(lot) = self.selected_lot().cloned() {
                            self.detail_ui(&mut columns[1], &lot);
                        } else {
                            ui_chrome::empty_state(&mut columns[1], "No material lot selected");
                        }
                    });
                }
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        let alerts = self.inventory.alerts(DEMO_TODAY);
        ui_chrome::section_label(ui, "Inventory");
        ui.label(format!("Material lots: {}", self.inventory.lots.len()));
        ui.label(format!(
            "Action required: {}",
            self.inventory
                .lots
                .values()
                .filter(|lot| InventoryQuickFilter::NeedsAction.matches(lot))
                .count()
        ));
        ui.label(format!("Alerts: {}", alerts.len()));
        ui.label(format!(
            "Low stock: {}",
            alerts
                .iter()
                .filter(|alert| alert.kind == InventoryAlertKind::LowStock)
                .count()
        ));
        ui.label(format!(
            "Expired: {}",
            alerts
                .iter()
                .filter(|alert| alert.kind == InventoryAlertKind::Expired)
                .count()
        ));

        ui.separator();
        if let Some(lot) = self.selected_lot() {
            ui.label(RichText::new(&lot.material_name).strong());
            ui.label(format!("Lot: {}", lot.id));
            ui.colored_label(material_status_color(lot), material_status_label(lot));
            ui.label(format!("Stock: {}", lot.stock.format()));
            ui.label(format!("Location: {}", lot.location.label()));
            ui.colored_label(expiration_color(lot), expiration_label(lot));
        }
    }

    fn ensure_selection(&mut self) {
        let selected_visible = self
            .selected_lot
            .as_ref()
            .and_then(|id| self.inventory.lot(id))
            .is_some_and(|lot| self.quick_filter.matches(lot));
        if !selected_visible {
            self.selected_lot = self
                .inventory
                .sorted_lots()
                .into_iter()
                .find(|lot| self.quick_filter.matches(lot))
                .map(|lot| lot.id.clone());
        }
    }

    fn selected_lot(&self) -> Option<&MaterialLot> {
        self.selected_lot
            .as_ref()
            .and_then(|id| self.inventory.lot(id))
    }

    fn visible_lots(&self) -> Vec<&MaterialLot> {
        self.inventory
            .sorted_lots()
            .into_iter()
            .filter(|lot| self.quick_filter.matches(lot))
            .collect()
    }

    fn select_alert_lot(&mut self, lot_id: MaterialLotId, status: &mut String) {
        if self
            .inventory
            .lot(&lot_id)
            .is_some_and(|lot| !self.quick_filter.matches(lot))
        {
            self.quick_filter = InventoryQuickFilter::NeedsAction;
        }
        self.selected_lot = Some(lot_id.clone());
        *status = format!("inventory selected {lot_id}");
    }

    fn lot_picker(&mut self, ui: &mut egui::Ui, status: &mut String) {
        let mut selected = self.selected_lot.clone();
        let selected_text = selected
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "No matching lot".to_string());
        egui::ComboBox::from_id_salt("inventory_lot_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for lot in self.visible_lots() {
                    ui.selectable_value(
                        &mut selected,
                        Some(lot.id.clone()),
                        format!("{} - {}", lot.id, lot.material_name),
                    );
                }
            });
        if selected != self.selected_lot {
            self.selected_lot = selected;
            *status = "inventory material lot selected".to_string();
        }
    }

    fn summary_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        let alerts = self.inventory.alerts(DEMO_TODAY);
        let lots = self.inventory.lots.values().collect::<Vec<_>>();
        let action_lots = lots
            .iter()
            .filter(|lot| InventoryQuickFilter::NeedsAction.matches(lot))
            .count();
        let product_hold = lots.iter().filter(|lot| lot.is_expired(DEMO_TODAY)).count();
        let shortages = lots.iter().filter(|lot| lot.is_low_stock()).count();
        let expiring = lots.iter().filter(|lot| is_expiring_soon(lot)).count();
        let usage_count = self
            .inventory
            .lots
            .values()
            .map(|lot| lot.usage.len())
            .sum::<usize>();
        let storage_areas = self.location_counts().len();

        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Material lots",
                    self.inventory.lots.len().to_string(),
                    "tracked",
                    Tone::Neutral,
                ),
                (
                    "Action required",
                    action_lots.to_string(),
                    "hold, reorder, or use-first",
                    if action_lots > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                ),
                (
                    "Product hold",
                    product_hold.to_string(),
                    "expired lots",
                    if product_hold > 0 {
                        Tone::Danger
                    } else {
                        Tone::Success
                    },
                ),
                (
                    "Shortages",
                    shortages.to_string(),
                    "below reorder point",
                    if shortages > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                ),
                (
                    "Use first",
                    expiring.to_string(),
                    "within 30 days",
                    Tone::Info,
                ),
                (
                    "Locations",
                    storage_areas.to_string(),
                    "storage areas",
                    Tone::Neutral,
                ),
                (
                    "Usage links",
                    usage_count.to_string(),
                    "fab objects",
                    Tone::Neutral,
                ),
            ],
        );

        ui.add_space(4.0);
        self.alert_summary_ui(ui, &alerts, status);
        self.location_summary_ui(ui);
    }

    fn alert_summary_ui(
        &mut self,
        ui: &mut egui::Ui,
        alerts: &[InventoryAlert],
        status: &mut String,
    ) {
        ui_chrome::section_label(ui, "Alerts");
        if alerts.is_empty() {
            ui_chrome::empty_state(ui, "No low-stock or expiration alerts");
            return;
        }

        if ui.available_width() < 520.0 {
            for alert in alerts {
                ui.group(|ui| {
                    fill_card_width(ui);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(alert_color(alert.kind), alert.kind.label());
                        if ui.button(alert.lot_id.to_string()).clicked() {
                            self.select_alert_lot(alert.lot_id.clone(), status);
                        }
                    });
                    ui.small(&alert.material_name);
                    ui.add(egui::Label::new(&alert.message).wrap());
                });
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("inventory_alert_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("inventory_alert_grid")
                    .striped(true)
                    .min_col_width(92.0)
                    .show(ui, |ui| {
                        ui.strong("Alert");
                        ui.strong("Lot");
                        ui.strong("Material");
                        ui.strong("Message");
                        ui.end_row();
                        for alert in alerts {
                            ui.colored_label(alert_color(alert.kind), alert.kind.label());
                            if ui.button(alert.lot_id.to_string()).clicked() {
                                self.select_alert_lot(alert.lot_id.clone(), status);
                            }
                            ui.add(egui::Label::new(&alert.material_name).wrap());
                            ui.add(egui::Label::new(&alert.message).wrap());
                            ui.end_row();
                        }
                    });
            });
    }

    fn location_summary_ui(&self, ui: &mut egui::Ui) {
        let location_counts = self.location_counts();
        if location_counts.is_empty() {
            return;
        }

        ui_chrome::section_label(ui, "Locations");
        if ui.available_width() < 520.0 {
            for (area, summary) in location_counts {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(area).strong());
                    ui_chrome::status_pill(ui, &format!("{} lots", summary.total), Tone::Neutral);
                    if summary.action_required > 0 {
                        ui_chrome::status_pill(
                            ui,
                            &format!("{} action", summary.action_required),
                            Tone::Warning,
                        );
                    }
                });
            }
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for (area, summary) in location_counts {
                ui.group(|ui| {
                    ui.set_min_width(150.0);
                    ui.label(RichText::new(area).strong());
                    ui.small(format!("{} lot(s)", summary.total));
                    if summary.action_required > 0 {
                        ui.colored_label(
                            alert_color(InventoryAlertKind::LowStock),
                            format!("{} need action", summary.action_required),
                        );
                    } else {
                        ui_chrome::muted(ui, "released stock");
                    }
                });
            }
        });
    }

    fn quick_filter_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        ui_chrome::section_label(ui, "Quick Filters");
        let previous = self.quick_filter;
        ui.horizontal_wrapped(|ui| {
            for filter in InventoryQuickFilter::ALL {
                let count = self
                    .inventory
                    .lots
                    .values()
                    .filter(|lot| filter.matches(lot))
                    .count();
                let label = format!("{} ({count})", filter.label());
                if ui
                    .selectable_label(self.quick_filter == filter, label)
                    .clicked()
                {
                    self.quick_filter = filter;
                }
            }
        });

        if self.quick_filter != previous {
            self.ensure_selection();
            *status = format!("inventory filter: {}", self.quick_filter.label());
        }

        let visible_count = self.visible_lots().len();
        ui.small(format!(
            "Showing {visible_count} of {} material lots",
            self.inventory.lots.len()
        ));
    }

    fn lot_table_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        ui_chrome::section_label(ui, "Material Lots");
        if self.inventory.lots.is_empty() {
            ui_chrome::empty_state(ui, "No inventory loaded");
            return;
        }
        if self.visible_lots().is_empty() {
            ui_chrome::empty_state(ui, "No material lots match this filter");
            return;
        }

        let mut clicked_lot = None;
        if ui.available_width() < 700.0 {
            for lot in self.visible_lots() {
                let selected = self.selected_lot.as_ref() == Some(&lot.id);
                ui.group(|ui| {
                    fill_card_width(ui);
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(selected, lot.id.to_string()).clicked() {
                            clicked_lot = Some(lot.id.clone());
                        }
                        ui.colored_label(material_status_color(lot), material_status_label(lot));
                    });
                    ui.add(egui::Label::new(&lot.material_name).wrap());
                    status_tags_ui(ui, lot);
                    ui.small(format!("Location: {}", lot.location.label()));
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(stock_color(lot), lot.stock.format());
                        if let Some(shortage) = shortage_label(lot) {
                            ui.colored_label(alert_color(InventoryAlertKind::LowStock), shortage);
                        }
                        ui.colored_label(expiration_color(lot), expiration_label(lot));
                    });
                });
            }
            if let Some(lot_id) = clicked_lot {
                self.selected_lot = Some(lot_id.clone());
                *status = format!("inventory selected {lot_id}");
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("inventory_lot_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("inventory_lot_table")
                    .striped(true)
                    .min_col_width(78.0)
                    .show(ui, |ui| {
                        ui.strong("Status");
                        ui.strong("Lot");
                        ui.strong("Material");
                        ui.strong("Stock");
                        ui.strong("Shortage");
                        ui.strong("Expires");
                        ui.strong("Location");
                        ui.strong("Usage");
                        ui.end_row();

                        for lot in self.visible_lots() {
                            let selected = self.selected_lot.as_ref() == Some(&lot.id);
                            ui.colored_label(
                                material_status_color(lot),
                                material_status_label(lot),
                            );
                            if ui.selectable_label(selected, lot.id.to_string()).clicked() {
                                clicked_lot = Some(lot.id.clone());
                            }
                            ui.add(egui::Label::new(&lot.material_name).wrap());
                            ui.colored_label(stock_color(lot), lot.stock.format());
                            if let Some(shortage) = shortage_label(lot) {
                                ui.colored_label(
                                    alert_color(InventoryAlertKind::LowStock),
                                    shortage,
                                );
                            } else {
                                ui_chrome::muted(ui, "ok");
                            }
                            ui.colored_label(expiration_color(lot), expiration_label(lot));
                            ui.add(egui::Label::new(lot.location.label()).wrap());
                            ui.label(format!("{} use(s)", lot.usage.len()));
                            ui.end_row();
                        }
                    });
            });
        if let Some(lot_id) = clicked_lot {
            self.selected_lot = Some(lot_id.clone());
            *status = format!("inventory selected {lot_id}");
        }
    }

    fn location_counts(&self) -> BTreeMap<String, LocationSummary> {
        let mut counts = BTreeMap::new();
        for lot in self.inventory.lots.values() {
            let summary = counts
                .entry(lot.location.area.clone())
                .or_insert_with(LocationSummary::default);
            summary.total += 1;
            if InventoryQuickFilter::NeedsAction.matches(lot) {
                summary.action_required += 1;
            }
        }
        counts
    }

    fn detail_ui(&self, ui: &mut egui::Ui, lot: &MaterialLot) {
        ui_chrome::section_label(ui, "Selected Material");
        ui.add(egui::Label::new(RichText::new(&lot.material_name).strong()).wrap());
        ui.horizontal_wrapped(|ui| {
            status_tags_ui(ui, lot);
        });

        ui.separator();
        ui_chrome::section_label(ui, "Material Status");
        ui.label(format!("Lot: {}", lot.id));
        ui.colored_label(material_status_color(lot), material_status_label(lot));
        if let Some(progress) = stock_progress(lot) {
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(ui.available_width().clamp(180.0, 420.0))
                    .text(format!(
                        "{} on hand / reorder {}",
                        lot.stock.format(),
                        lot.reorder_threshold.format()
                    )),
            );
        } else {
            ui.label(format!("Stock: {}", lot.stock.format()));
        }
        ui.label(format!("Reorder point: {}", lot.reorder_threshold.format()));
        if let Some(shortage) = shortage_label(lot) {
            ui.colored_label(alert_color(InventoryAlertKind::LowStock), shortage);
        } else {
            ui_chrome::muted(ui, "Stock is above reorder point");
        }
        ui.colored_label(expiration_color(lot), expiration_label(lot));
        ui.add(egui::Label::new(release_cue(lot)).wrap());

        ui.separator();
        ui_chrome::section_label(ui, "Location / Handling");
        egui::Grid::new(("inventory_location_detail", lot.id.as_str()))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                detail_row(ui, "Area", &lot.location.area);
                detail_row(ui, "Cabinet", &lot.location.cabinet);
                detail_row(ui, "Bin", &lot.location.bin);
                detail_row(ui, "Storage", &lot.location.temperature);
            });
        if !lot.notes.is_empty() {
            ui.add(
                egui::Label::new(RichText::new(&lot.notes).color(ui.visuals().weak_text_color()))
                    .wrap(),
            );
        }

        ui.separator();
        ui_chrome::section_label(ui, "Supplier / Certificate");
        egui::Grid::new(("inventory_supplier_detail", lot.id.as_str()))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                detail_row(ui, "Supplier", &lot.supplier.supplier);
                detail_row(ui, "Supplier lot", &lot.supplier.supplier_lot);
                detail_row(ui, "Received", &format_date(lot.supplier.received_date));
                detail_row(ui, "Received by", &lot.supplier.received_by);
                if let Some(certificate_id) = &lot.supplier.certificate_id {
                    detail_row(ui, "Certificate", certificate_id);
                }
                if let Some(certificate_url) = &lot.supplier.certificate_url {
                    detail_row(ui, "Record", certificate_url);
                }
            });

        ui.separator();
        self.usage_history_ui(ui, lot);
    }

    fn usage_history_ui(&self, ui: &mut egui::Ui, lot: &MaterialLot) {
        ui_chrome::section_label(ui, "Usage History");
        if lot.usage.is_empty() {
            ui_chrome::empty_state(ui, "No usage recorded");
            return;
        }

        if ui.available_width() < 520.0 {
            for usage in &lot.usage {
                ui.group(|ui| {
                    fill_card_width(ui);
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format_date(usage.timestamp));
                        ui.colored_label(stock_color(lot), usage.quantity.format());
                    });
                    ui.small(format!("Actor: {}", usage.actor));
                    ui.add(egui::Label::new(&usage.note).wrap());
                    ui.small(format!(
                        "Links: {}",
                        usage
                            .links
                            .iter()
                            .map(|link| link.label())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                });
            }
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt(("inventory_usage_horizontal", lot.id.as_str()))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new(("inventory_usage", lot.id.as_str()))
                    .striped(true)
                    .min_col_width(78.0)
                    .show(ui, |ui| {
                        ui.strong("Date");
                        ui.strong("Qty");
                        ui.strong("Actor");
                        ui.strong("Fab links");
                        ui.strong("Note");
                        ui.end_row();
                        for usage in &lot.usage {
                            ui.label(format_date(usage.timestamp));
                            ui.label(usage.quantity.format());
                            ui.label(&usage.actor);
                            ui.label(
                                usage
                                    .links
                                    .iter()
                                    .map(|link| link.label())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            )
                            .on_hover_text(&usage.note);
                            ui.add(egui::Label::new(&usage.note).wrap());
                            ui.end_row();
                        }
                    });
            });
    }
}

fn stock_color(lot: &MaterialLot) -> Color32 {
    if lot.is_low_stock() {
        Color32::from_rgb(220, 176, 72)
    } else {
        Color32::from_rgb(93, 184, 132)
    }
}

fn alert_color(kind: InventoryAlertKind) -> Color32 {
    match kind {
        InventoryAlertKind::LowStock => Color32::from_rgb(220, 176, 72),
        InventoryAlertKind::Expired => Color32::from_rgb(226, 96, 96),
        InventoryAlertKind::ExpiringSoon => Color32::from_rgb(93, 168, 232),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InventoryQuickFilter {
    All,
    NeedsAction,
    ProductHold,
    LowStock,
    ExpiringSoon,
    InUse,
}

impl InventoryQuickFilter {
    const ALL: [Self; 6] = [
        Self::All,
        Self::NeedsAction,
        Self::ProductHold,
        Self::LowStock,
        Self::ExpiringSoon,
        Self::InUse,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NeedsAction => "Action",
            Self::ProductHold => "Hold",
            Self::LowStock => "Low stock",
            Self::ExpiringSoon => "Expiring",
            Self::InUse => "In use",
        }
    }

    fn matches(self, lot: &MaterialLot) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAction => {
                lot.is_expired(DEMO_TODAY) || lot.is_low_stock() || is_expiring_soon(lot)
            }
            Self::ProductHold => lot.is_expired(DEMO_TODAY),
            Self::LowStock => lot.is_low_stock(),
            Self::ExpiringSoon => is_expiring_soon(lot),
            Self::InUse => !lot.usage.is_empty(),
        }
    }
}

#[derive(Default)]
struct LocationSummary {
    total: usize,
    action_required: usize,
}

fn status_tags_ui(ui: &mut egui::Ui, lot: &MaterialLot) {
    ui_chrome::status_pill(ui, lot.category.label(), Tone::Neutral);
    ui_chrome::status_pill(ui, material_status_label(lot), material_status_tone(lot));
}

fn material_status_label(lot: &MaterialLot) -> &'static str {
    if lot.is_expired(DEMO_TODAY) {
        "Product hold"
    } else if lot.is_low_stock() && is_expiring_soon(lot) {
        "Short + use first"
    } else if lot.is_low_stock() {
        "Shortage"
    } else if is_expiring_soon(lot) {
        "Use first"
    } else {
        "Released"
    }
}

fn material_status_tone(lot: &MaterialLot) -> Tone {
    if lot.is_expired(DEMO_TODAY) {
        Tone::Danger
    } else if lot.is_low_stock() {
        Tone::Warning
    } else if is_expiring_soon(lot) {
        Tone::Info
    } else {
        Tone::Success
    }
}

fn material_status_color(lot: &MaterialLot) -> Color32 {
    material_status_tone(lot).color()
}

fn is_expiring_soon(lot: &MaterialLot) -> bool {
    !lot.is_expired(DEMO_TODAY) && lot.expires_within_days(DEMO_TODAY, 30)
}

fn shortage_label(lot: &MaterialLot) -> Option<String> {
    if !lot.is_low_stock() || lot.stock.unit != lot.reorder_threshold.unit {
        return None;
    }

    let gap = lot.reorder_threshold.amount - lot.stock.amount;
    if gap <= 0.0 {
        Some(format!("At reorder ({})", lot.reorder_threshold.format()))
    } else {
        Some(format!(
            "Short {}",
            format_quantity(gap, lot.reorder_threshold.unit.symbol())
        ))
    }
}

fn stock_progress(lot: &MaterialLot) -> Option<f32> {
    if lot.stock.unit != lot.reorder_threshold.unit || lot.reorder_threshold.amount <= 0.0 {
        return None;
    }
    Some((lot.stock.amount / lot.reorder_threshold.amount).clamp(0.0, 1.0) as f32)
}

fn expiration_label(lot: &MaterialLot) -> String {
    let Some(expires_on) = lot.expires_on else {
        return "No expiry".to_string();
    };
    let date = format_date(expires_on);
    if lot.is_expired(DEMO_TODAY) {
        if let Some(days) = days_between(expires_on, DEMO_TODAY) {
            return format!("Expired {days}d ago ({date})");
        }
        return format!("Expired {date}");
    }
    if let Some(days) = days_between(DEMO_TODAY, expires_on) {
        if days <= 30 {
            return format!("Expires in {days}d ({date})");
        }
    }
    format!("Expires {date}")
}

fn expiration_color(lot: &MaterialLot) -> Color32 {
    if lot.is_expired(DEMO_TODAY) {
        alert_color(InventoryAlertKind::Expired)
    } else if is_expiring_soon(lot) {
        alert_color(InventoryAlertKind::ExpiringSoon)
    } else {
        ui_chrome::Tone::Neutral.color()
    }
}

fn release_cue(lot: &MaterialLot) -> String {
    if lot.is_expired(DEMO_TODAY) {
        return "Release: product hold until replacement stock or QA disposition is recorded."
            .to_string();
    }
    if lot.is_low_stock() && is_expiring_soon(lot) {
        return "Release: available, but use first and replenish before the next product run."
            .to_string();
    }
    if lot.is_low_stock() {
        return "Release: available with open reorder action before additional starts.".to_string();
    }
    if is_expiring_soon(lot) {
        return "Release: available; consume before newer stock to avoid expiry loss.".to_string();
    }
    "Release: available for product use.".to_string()
}

fn format_quantity(amount: f64, unit: &str) -> String {
    let amount = if amount.fract().abs() < f64::EPSILON {
        format!("{amount:.0}")
    } else {
        format!("{amount:.1}")
    };
    format!("{amount} {unit}")
}

fn fill_card_width(ui: &mut egui::Ui) {
    let width = ui.available_width().max(220.0);
    ui.set_min_width(width);
    ui.set_max_width(width);
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(
        RichText::new(label)
            .small()
            .color(ui.visuals().weak_text_color()),
    );
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}

fn days_between(start: u32, end: u32) -> Option<i64> {
    let (start_year, start_month, start_day) = parse_date(start)?;
    let (end_year, end_month, end_day) = parse_date(end)?;
    Some(
        days_from_civil(end_year, end_month, end_day)
            - days_from_civil(start_year, start_month, start_day),
    )
}

fn parse_date(date: u32) -> Option<(i32, u32, u32)> {
    let year = i32::try_from(date / 10_000).ok()?;
    let month = (date / 100) % 100;
    let day = date % 100;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i32::try_from(month).unwrap_or_default();
    let day = i32::try_from(day).unwrap_or_default();
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146_097 + doe - 719_468)
}
