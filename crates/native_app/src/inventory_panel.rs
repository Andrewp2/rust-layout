use eframe::egui::{self, Color32, RichText};
use layout_model::inventory::{
    Inventory, InventoryAlert, InventoryAlertKind, MaterialLot, MaterialLotId, format_date,
};

use crate::ui_chrome::{self, Tone};

const DEMO_TODAY: u32 = 20260508;

pub(crate) struct InventoryPanel {
    inventory: Inventory,
    selected_lot: Option<MaterialLotId>,
}

impl InventoryPanel {
    pub(crate) fn from_inventory(inventory: Inventory) -> Self {
        let selected_lot = inventory.sorted_lots().first().map(|lot| lot.id.clone());
        Self {
            inventory,
            selected_lot,
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

                self.summary_ui(ui);
                ui.separator();
                ui.columns(2, |columns| {
                    self.lot_table_ui(&mut columns[0], status);
                    if let Some(lot) = self.selected_lot().cloned() {
                        self.detail_ui(&mut columns[1], &lot);
                    } else {
                        ui_chrome::empty_state(&mut columns[1], "No material lot selected");
                    }
                });
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        let alerts = self.inventory.alerts(DEMO_TODAY);
        ui_chrome::section_label(ui, "Inventory");
        ui.label(format!("Material lots: {}", self.inventory.lots.len()));
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
            ui.label(format!("Stock: {}", lot.stock.format()));
            ui.label(format!("Location: {}", lot.location.label()));
            if let Some(expires_on) = lot.expires_on {
                ui.label(format!("Expires: {}", format_date(expires_on)));
            }
        }
    }

    fn ensure_selection(&mut self) {
        if self
            .selected_lot
            .as_ref()
            .is_none_or(|id| !self.inventory.lots.contains_key(id))
        {
            self.selected_lot = self
                .inventory
                .sorted_lots()
                .first()
                .map(|lot| lot.id.clone());
        }
    }

    fn selected_lot(&self) -> Option<&MaterialLot> {
        self.selected_lot
            .as_ref()
            .and_then(|id| self.inventory.lot(id))
    }

    fn lot_picker(&mut self, ui: &mut egui::Ui, status: &mut String) {
        let mut selected = self.selected_lot.clone();
        let selected_text = selected
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "No material lot".to_string());
        egui::ComboBox::from_id_salt("inventory_lot_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for lot in self.inventory.sorted_lots() {
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

    fn summary_ui(&self, ui: &mut egui::Ui) {
        let alerts = self.inventory.alerts(DEMO_TODAY);
        let low_stock = alerts
            .iter()
            .filter(|alert| alert.kind == InventoryAlertKind::LowStock)
            .count();
        let expired = alerts
            .iter()
            .filter(|alert| alert.kind == InventoryAlertKind::Expired)
            .count();
        let expiring = alerts
            .iter()
            .filter(|alert| alert.kind == InventoryAlertKind::ExpiringSoon)
            .count();
        let usage_count = self
            .inventory
            .lots
            .values()
            .map(|lot| lot.usage.len())
            .sum::<usize>();

        ui.horizontal_wrapped(|ui| {
            ui_chrome::metric_tile(ui, "Material lots", self.inventory.lots.len(), "tracked");
            ui_chrome::metric_tile_tone(
                ui,
                "Low stock",
                low_stock,
                "at reorder point",
                if low_stock > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            );
            ui_chrome::metric_tile_tone(
                ui,
                "Expired",
                expired,
                "blocked for product",
                if expired > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            );
            ui_chrome::metric_tile_tone(ui, "Expiring", expiring, "within 30 days", Tone::Info);
            ui_chrome::metric_tile(ui, "Usage links", usage_count, "fab objects");
        });

        ui.add_space(4.0);
        self.alert_summary_ui(ui, &alerts);
    }

    fn alert_summary_ui(&self, ui: &mut egui::Ui, alerts: &[InventoryAlert]) {
        ui_chrome::section_label(ui, "Alerts");
        if alerts.is_empty() {
            ui_chrome::empty_state(ui, "No low-stock or expiration alerts");
            return;
        }

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
                    ui.label(alert.lot_id.to_string());
                    ui.label(&alert.material_name);
                    ui.label(&alert.message);
                    ui.end_row();
                }
            });
    }

    fn lot_table_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        ui_chrome::section_label(ui, "Material Lots");
        if self.inventory.lots.is_empty() {
            ui_chrome::empty_state(ui, "No inventory loaded");
            return;
        }

        egui::Grid::new("inventory_lot_table")
            .striped(true)
            .min_col_width(78.0)
            .show(ui, |ui| {
                ui.strong("Lot");
                ui.strong("Material");
                ui.strong("Stock");
                ui.strong("Expires");
                ui.end_row();

                for lot in self.inventory.sorted_lots() {
                    let selected = self.selected_lot.as_ref() == Some(&lot.id);
                    if ui.selectable_label(selected, lot.id.to_string()).clicked() {
                        self.selected_lot = Some(lot.id.clone());
                        *status = format!("inventory selected {}", lot.id);
                    }
                    ui.label(&lot.material_name);
                    ui.colored_label(stock_color(lot), lot.stock.format());
                    ui.label(
                        lot.expires_on
                            .map(format_date)
                            .unwrap_or_else(|| "none".to_string()),
                    );
                    ui.end_row();
                }
            });
    }

    fn detail_ui(&self, ui: &mut egui::Ui, lot: &MaterialLot) {
        ui_chrome::section_label(ui, "Selected Material");
        ui.label(RichText::new(&lot.material_name).strong());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, lot.category.label(), Tone::Neutral);
            if lot.is_expired(DEMO_TODAY) {
                ui_chrome::status_pill(ui, "Expired", Tone::Danger);
            } else if lot.is_low_stock() {
                ui_chrome::status_pill(ui, "Low stock", Tone::Warning);
            }
        });
        ui.label(format!("Lot: {}", lot.id));
        ui.label(format!("Stock: {}", lot.stock.format()));
        ui.label(format!("Reorder: {}", lot.reorder_threshold.format()));
        ui.label(format!("Location: {}", lot.location.label()));
        ui.label(format!("Storage: {}", lot.location.temperature));
        if let Some(expires_on) = lot.expires_on {
            ui.label(format!("Expires: {}", format_date(expires_on)));
        }
        if !lot.notes.is_empty() {
            ui.label(RichText::new(&lot.notes).color(ui.visuals().weak_text_color()));
        }

        ui.separator();
        ui_chrome::section_label(ui, "Supplier / Certificate");
        ui.label(format!("Supplier: {}", lot.supplier.supplier));
        ui.label(format!("Supplier lot: {}", lot.supplier.supplier_lot));
        ui.label(format!(
            "Received: {}",
            format_date(lot.supplier.received_date)
        ));
        ui.label(format!("Received by: {}", lot.supplier.received_by));
        if let Some(certificate_id) = &lot.supplier.certificate_id {
            ui.label(format!("Certificate: {certificate_id}"));
        }

        ui.separator();
        self.usage_history_ui(ui, lot);
    }

    fn usage_history_ui(&self, ui: &mut egui::Ui, lot: &MaterialLot) {
        ui_chrome::section_label(ui, "Usage History");
        if lot.usage.is_empty() {
            ui_chrome::empty_state(ui, "No usage recorded");
            return;
        }

        egui::Grid::new(("inventory_usage", lot.id.as_str()))
            .striped(true)
            .min_col_width(78.0)
            .show(ui, |ui| {
                ui.strong("Date");
                ui.strong("Qty");
                ui.strong("Actor");
                ui.strong("Fab links");
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
                    ui.end_row();
                }
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
