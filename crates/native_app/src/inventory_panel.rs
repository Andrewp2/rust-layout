use std::collections::BTreeMap;

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use layout_model::inventory::{
    Inventory, InventoryAlert, InventoryAlertKind, MaterialLot, MaterialLotId, format_date,
};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{
    operad_egui,
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar},
    ui_chrome::{self, Tone},
};

const DEMO_TODAY: u32 = 20260508;
const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_FILTER: &str = "inventory.action.filter.";
const OPERAD_ACTION_SELECT_LOT: &str = "inventory.action.select_lot.";

pub(crate) struct InventoryPanel {
    inventory: Inventory,
    selected_lot: Option<MaterialLotId>,
    quick_filter: InventoryQuickFilter,
}

#[derive(Debug)]
struct InventoryOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct InventoryMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct InventoryOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
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
        if let Err(error) = self.operad_ui(ui, status) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, status);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui, status: &mut String) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("inventory_panel_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view = self.build_operad_view(width);
                if let Err(error) = view
                    .document
                    .compute_layout(view.size, &mut ApproxTextMeasurer)
                    .map_err(|error| error.to_string())
                {
                    result = Err(error);
                    return;
                }

                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(node_name) =
                        operad_egui::hit_test_name(&view.document, rect, pointer)
                    && self.handle_operad_action(&node_name, status)
                {
                    view = self.build_operad_view(width);
                    if let Err(error) = view
                        .document
                        .compute_layout(view.size, &mut ApproxTextMeasurer)
                        .map_err(|error| error.to_string())
                    {
                        result = Err(error);
                        return;
                    }
                }

                if response.hovered()
                    && let Some(pointer) = ui.ctx().pointer_hover_pos()
                    && operad_egui::hit_test_name(&view.document, rect, pointer).is_some()
                {
                    ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
                }
                operad_egui::paint_document_at(ui, &view.document, rect);
            });
        result
    }

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
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

    fn build_operad_view(&self, width: f32) -> InventoryOperadView {
        let metrics = self.operad_metrics();
        let filter_rows = self.operad_filter_rows();
        let alert_rows = self.operad_alert_rows();
        let location_rows = self.operad_location_rows();
        let lot_rows = self.operad_lot_rows();
        let detail_rows = self.operad_detail_rows();
        let usage_rows = self.operad_usage_rows();
        let height = inventory_operad_view_height(
            width,
            metrics.len(),
            &[
                filter_rows.len(),
                alert_rows.len(),
                location_rows.len(),
                lot_rows.len(),
                detail_rows.len(),
                usage_rows.len(),
            ],
        );
        let size = UiSize::new(width, height);
        let mut document = UiDocument::new(root_style(width, height));
        let root = document.root;
        document.set_node_visual(
            root,
            UiVisual::panel(
                ColorRgba::new(15, 18, 21, 255),
                Some(StrokeStyle::new(ColorRgba::new(39, 46, 52, 255), 1.0)),
                0.0,
            ),
        );

        let alert_count = self.inventory.alerts(DEMO_TODAY).len();
        add_inventory_operad_header(
            &mut document,
            root,
            "MATERIALS CONTROL",
            "Inventory Tracker",
            "Material lots, storage posture, usage links, certificates, and release cues",
            &format!(
                "{} visible of {} lots · {} alert(s)",
                lot_rows.len(),
                self.inventory.lots.len(),
                alert_count
            ),
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_metric_grid(&mut document, root, width, &metrics);
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.filters",
            "Quick Filters",
            "No filters available",
            &filter_rows,
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.alerts",
            "Alerts",
            "No low-stock or expiration alerts",
            &alert_rows,
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.locations",
            "Locations",
            "No storage locations loaded",
            &location_rows,
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.lots",
            "Material Lots",
            "No material lots match this filter",
            &lot_rows,
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.detail",
            "Selected Material",
            "No material lot selected",
            &detail_rows,
        );
        add_inventory_operad_spacer(&mut document, root, OPERAD_GAP);
        add_inventory_operad_section(
            &mut document,
            root,
            width,
            "inventory.usage",
            "Usage History",
            "No usage recorded",
            &usage_rows,
        );

        InventoryOperadView { document, size }
    }

    fn operad_metrics(&self) -> Vec<InventoryMetricTile> {
        let lots = self.inventory.lots.values().collect::<Vec<_>>();
        let action_lots = lots
            .iter()
            .filter(|lot| InventoryQuickFilter::NeedsAction.matches(lot))
            .count();
        let product_hold = lots.iter().filter(|lot| lot.is_expired(DEMO_TODAY)).count();
        let shortages = lots.iter().filter(|lot| lot.is_low_stock()).count();
        let expiring = lots.iter().filter(|lot| is_expiring_soon(lot)).count();
        let usage_count = lots.iter().map(|lot| lot.usage.len()).sum::<usize>();
        let storage_areas = self.location_counts().len();
        vec![
            InventoryMetricTile {
                label: "Material lots".to_string(),
                value: self.inventory.lots.len().to_string(),
                detail: "tracked".to_string(),
                tone: Tone::Neutral,
            },
            InventoryMetricTile {
                label: "Action required".to_string(),
                value: action_lots.to_string(),
                detail: "hold, reorder, or use-first".to_string(),
                tone: if action_lots > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            },
            InventoryMetricTile {
                label: "Product hold".to_string(),
                value: product_hold.to_string(),
                detail: "expired lots".to_string(),
                tone: if product_hold > 0 {
                    Tone::Danger
                } else {
                    Tone::Success
                },
            },
            InventoryMetricTile {
                label: "Shortages".to_string(),
                value: shortages.to_string(),
                detail: "below reorder point".to_string(),
                tone: if shortages > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
            },
            InventoryMetricTile {
                label: "Use first".to_string(),
                value: expiring.to_string(),
                detail: "within 30 days".to_string(),
                tone: Tone::Info,
            },
            InventoryMetricTile {
                label: "Locations".to_string(),
                value: storage_areas.to_string(),
                detail: "storage areas".to_string(),
                tone: Tone::Neutral,
            },
            InventoryMetricTile {
                label: "Usage links".to_string(),
                value: usage_count.to_string(),
                detail: "fab objects".to_string(),
                tone: Tone::Neutral,
            },
        ]
    }

    fn operad_filter_rows(&self) -> Vec<InventoryOperadRow> {
        InventoryQuickFilter::ALL
            .into_iter()
            .map(|filter| {
                let count = self
                    .inventory
                    .lots
                    .values()
                    .filter(|lot| filter.matches(lot))
                    .count();
                InventoryOperadRow {
                    title: format!("{} ({count})", filter.label()),
                    detail: if filter == self.quick_filter {
                        "Current inventory filter".to_string()
                    } else {
                        "Click to filter material lots".to_string()
                    },
                    tone: if filter == self.quick_filter {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                    action_name: Some(format!("{OPERAD_ACTION_FILTER}{}", filter.slug())),
                    selected: filter == self.quick_filter,
                }
            })
            .collect()
    }

    fn operad_alert_rows(&self) -> Vec<InventoryOperadRow> {
        self.inventory
            .alerts(DEMO_TODAY)
            .into_iter()
            .enumerate()
            .map(|(index, alert)| InventoryOperadRow {
                title: format!("{} · {}", alert.kind.label(), alert.lot_id),
                detail: format!("{} · {}", alert.material_name, alert.message),
                tone: alert_tone(alert.kind),
                action_name: Some(format!(
                    "{OPERAD_ACTION_SELECT_LOT}{}|alert.{index}",
                    alert.lot_id
                )),
                selected: self.selected_lot.as_ref() == Some(&alert.lot_id),
            })
            .collect()
    }

    fn operad_location_rows(&self) -> Vec<InventoryOperadRow> {
        self.location_counts()
            .into_iter()
            .map(|(area, summary)| InventoryOperadRow {
                title: area,
                detail: if summary.action_required > 0 {
                    format!(
                        "{} lot(s), {} need action",
                        summary.total, summary.action_required
                    )
                } else {
                    format!("{} lot(s), released stock", summary.total)
                },
                tone: if summary.action_required > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                },
                action_name: None,
                selected: false,
            })
            .collect()
    }

    fn operad_lot_rows(&self) -> Vec<InventoryOperadRow> {
        self.visible_lots()
            .into_iter()
            .enumerate()
            .map(|(index, lot)| InventoryOperadRow {
                title: format!("{} · {}", lot.id, material_status_label(lot)),
                detail: format!(
                    "{} · {} · {} · {}",
                    truncate_middle(&lot.material_name, 34),
                    lot.stock.format(),
                    expiration_label(lot),
                    lot.location.label()
                ),
                tone: material_status_tone(lot),
                action_name: Some(format!("{OPERAD_ACTION_SELECT_LOT}{}|lot.{index}", lot.id)),
                selected: self.selected_lot.as_ref() == Some(&lot.id),
            })
            .collect()
    }

    fn operad_detail_rows(&self) -> Vec<InventoryOperadRow> {
        let Some(lot) = self.selected_lot() else {
            return Vec::new();
        };
        let mut rows = vec![
            InventoryOperadRow {
                title: lot.material_name.clone(),
                detail: format!("{} · {}", lot.id, material_status_label(lot)),
                tone: material_status_tone(lot),
                action_name: None,
                selected: true,
            },
            InventoryOperadRow {
                title: "Stock".to_string(),
                detail: format!(
                    "{} on hand · reorder {}{}",
                    lot.stock.format(),
                    lot.reorder_threshold.format(),
                    shortage_label(lot)
                        .map(|shortage| format!(" · {shortage}"))
                        .unwrap_or_default()
                ),
                tone: if lot.is_low_stock() {
                    Tone::Warning
                } else {
                    Tone::Success
                },
                action_name: None,
                selected: false,
            },
            InventoryOperadRow {
                title: "Expiration".to_string(),
                detail: expiration_label(lot),
                tone: if lot.is_expired(DEMO_TODAY) {
                    Tone::Danger
                } else if is_expiring_soon(lot) {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                action_name: None,
                selected: false,
            },
            InventoryOperadRow {
                title: "Release cue".to_string(),
                detail: release_cue(lot),
                tone: material_status_tone(lot),
                action_name: None,
                selected: false,
            },
            InventoryOperadRow {
                title: "Location / handling".to_string(),
                detail: format!(
                    "{} · cabinet {} · bin {} · {}",
                    lot.location.area,
                    lot.location.cabinet,
                    lot.location.bin,
                    lot.location.temperature
                ),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            },
            InventoryOperadRow {
                title: "Supplier".to_string(),
                detail: format!(
                    "{} · lot {} · received {} by {}",
                    lot.supplier.supplier,
                    lot.supplier.supplier_lot,
                    format_date(lot.supplier.received_date),
                    lot.supplier.received_by
                ),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            },
        ];
        if let Some(certificate_id) = &lot.supplier.certificate_id {
            rows.push(InventoryOperadRow {
                title: "Certificate".to_string(),
                detail: certificate_id.clone(),
                tone: Tone::Info,
                action_name: None,
                selected: false,
            });
        }
        if let Some(certificate_url) = &lot.supplier.certificate_url {
            rows.push(InventoryOperadRow {
                title: "Record".to_string(),
                detail: certificate_url.clone(),
                tone: Tone::Info,
                action_name: None,
                selected: false,
            });
        }
        if !lot.notes.is_empty() {
            rows.push(InventoryOperadRow {
                title: "Notes".to_string(),
                detail: lot.notes.clone(),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            });
        }
        rows
    }

    fn operad_usage_rows(&self) -> Vec<InventoryOperadRow> {
        let Some(lot) = self.selected_lot() else {
            return Vec::new();
        };
        lot.usage
            .iter()
            .map(|usage| InventoryOperadRow {
                title: format!(
                    "{} · {}",
                    format_date(usage.timestamp),
                    usage.quantity.format()
                ),
                detail: format!(
                    "{} · {} · {}",
                    usage.actor,
                    usage
                        .links
                        .iter()
                        .map(|link| link.label())
                        .collect::<Vec<_>>()
                        .join(", "),
                    usage.note
                ),
                tone: Tone::Neutral,
                action_name: None,
                selected: false,
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str, status: &mut String) -> bool {
        if let Some(slug) = node_name.strip_prefix(OPERAD_ACTION_FILTER)
            && let Some(filter) = InventoryQuickFilter::from_slug(slug)
        {
            if self.quick_filter != filter {
                self.quick_filter = filter;
                self.ensure_selection();
                *status = format!("inventory filter: {}", self.quick_filter.label());
            }
            return true;
        }
        if let Some(lot_id) = node_name.strip_prefix(OPERAD_ACTION_SELECT_LOT) {
            let lot_id = lot_id
                .split_once('|')
                .map(|(lot_id, _)| lot_id)
                .unwrap_or(lot_id);
            let lot_id = MaterialLotId::new(lot_id.to_string());
            if self.inventory.lot(&lot_id).is_some() {
                self.select_alert_lot(lot_id, status);
                return true;
            }
        }
        false
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        let sections = self.context_sections();
        render_sidecar(ui, "inventory.context", &sections)
    }

    fn context_sections(&self) -> Vec<SidecarSection> {
        let alerts = self.inventory.alerts(DEMO_TODAY);
        let needs_action = self
            .inventory
            .lots
            .values()
            .filter(|lot| InventoryQuickFilter::NeedsAction.matches(lot))
            .count();
        let low_stock = alerts
            .iter()
            .filter(|alert| alert.kind == InventoryAlertKind::LowStock)
            .count();
        let expired = alerts
            .iter()
            .filter(|alert| alert.kind == InventoryAlertKind::Expired)
            .count();
        let mut sections = vec![
            SidecarSection::new("Inventory")
                .row(SidecarRow::new(
                    format!("{} material lots", self.inventory.lots.len()),
                    format!("{needs_action} need action | {} alerts", alerts.len()),
                    if needs_action > 0 {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                ))
                .row(SidecarRow::new(
                    "Stock alerts",
                    format!("{low_stock} low stock | {expired} expired"),
                    if expired > 0 {
                        Tone::Danger
                    } else if low_stock > 0 {
                        Tone::Warning
                    } else {
                        Tone::Success
                    },
                )),
        ];

        let selected = self
            .selected_lot()
            .map(|lot| {
                SidecarSection::new("Selected Lot")
                    .row(
                        SidecarRow::new(
                            &lot.material_name,
                            format!("Lot {} | {}", lot.id, material_status_label(lot)),
                            if InventoryQuickFilter::NeedsAction.matches(lot) {
                                Tone::Warning
                            } else {
                                Tone::Info
                            },
                        )
                        .selected(true),
                    )
                    .row(SidecarRow::new(
                        "Stock",
                        lot.stock.format(),
                        if lot.is_low_stock() {
                            Tone::Warning
                        } else {
                            Tone::Neutral
                        },
                    ))
                    .row(SidecarRow::new(
                        "Location / expiry",
                        format!("{} | {}", lot.location.label(), expiration_label(lot)),
                        if lot.is_expired(DEMO_TODAY) {
                            Tone::Danger
                        } else {
                            Tone::Neutral
                        },
                    ))
            })
            .unwrap_or_else(|| {
                SidecarSection::new("Selected Lot").empty("No material lot selected")
            });
        sections.push(selected);
        sections
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
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

fn inventory_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += inventory_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += inventory_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn inventory_operad_metric_columns(width: f32) -> usize {
    if width >= 1020.0 {
        4
    } else if width >= 680.0 {
        3
    } else if width >= 440.0 {
        2
    } else {
        1
    }
}

fn inventory_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = inventory_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn inventory_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_inventory_operad_header(
    document: &mut UiDocument,
    parent: UiNodeId,
    eyebrow: &str,
    title: &str,
    detail: &str,
    meta: &str,
) {
    let header = document.add_child(
        parent,
        UiNode::container(
            "inventory.header",
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(OPERAD_HEADER_HEIGHT),
                    ),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 27, 32, 255),
            Some(StrokeStyle::new(ColorRgba::new(46, 55, 64, 255), 1.0)),
            6.0,
        )),
    );
    add_inventory_operad_text(
        document,
        header,
        "inventory.header.eyebrow",
        eyebrow,
        inventory_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_inventory_operad_text(
        document,
        header,
        "inventory.header.title",
        title,
        inventory_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_inventory_operad_text(
        document,
        header,
        "inventory.header.detail",
        detail,
        inventory_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_inventory_operad_text(
        document,
        header,
        "inventory.header.meta",
        meta,
        inventory_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_inventory_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[InventoryMetricTile],
) {
    let columns = inventory_operad_metric_columns(width);
    let grid_height = inventory_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "inventory.metrics",
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(grid_height),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    let tile_width =
        ((width - OPERAD_GAP * (columns.saturating_sub(1) as f32)) / columns as f32).max(120.0);
    for (row_index, chunk) in metrics.chunks(columns).enumerate() {
        let row = document.add_child(
            grid,
            UiNode::container(
                format!("inventory.metrics.row.{row_index}"),
                UiNodeStyle {
                    layout: layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_METRIC_HEIGHT),
                    ),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );
        for (column, metric) in chunk.iter().enumerate() {
            add_inventory_operad_metric_tile(
                document,
                row,
                &format!("inventory.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_inventory_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &InventoryMetricTile,
) {
    let tile = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_margin_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width.max(116.0)),
                            layout::px(OPERAD_METRIC_HEIGHT - 8.0),
                        ),
                        3.0,
                    ),
                    9.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(29, 35, 40, 255),
            Some(StrokeStyle::new(
                inventory_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_inventory_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        inventory_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_inventory_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        inventory_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_inventory_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        inventory_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            inventory_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_inventory_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[InventoryOperadRow],
) {
    let height = inventory_operad_section_height(rows.len());
    let section = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    OPERAD_PAD,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(21, 26, 31, 255),
            Some(StrokeStyle::new(ColorRgba::new(45, 53, 61, 255), 1.0)),
            6.0,
        )),
    );
    add_inventory_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        inventory_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_inventory_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_inventory_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_inventory_operad_empty_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    label: &str,
) {
    let row = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.empty"),
            UiNodeStyle {
                layout: layout::with_padding_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(OPERAD_EMPTY_ROW_HEIGHT),
                    ),
                    8.0,
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(27, 32, 37, 255),
            Some(StrokeStyle::new(ColorRgba::new(43, 50, 58, 255), 1.0)),
            5.0,
        )),
    );
    add_inventory_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        inventory_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_inventory_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &InventoryOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        inventory_operad_tone_color(Tone::Info)
    } else {
        ColorRgba::new(42, 50, 58, 255)
    };
    let fill = if row.selected {
        ColorRgba::new(26, 42, 56, 255)
    } else {
        ColorRgba::new(26, 31, 36, 255)
    };
    let mut node = UiNode::container(
        row_name,
        UiNodeStyle {
            layout: layout::with_padding_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(OPERAD_ROW_HEIGHT),
                ),
                6.0,
            ),
            clip: ClipBehavior::Clip,
            ..Default::default()
        },
    )
    .with_visual(UiVisual::panel(
        fill,
        Some(StrokeStyle::new(stroke_color, 1.0)),
        4.0,
    ));
    if row.action_name.is_some() {
        node = node.with_input(InputBehavior::BUTTON).with_accessibility(
            crate::ui_chrome::operad_button_accessibility(&row.title, &row.detail),
        );
    }
    let row_node = document.add_child(parent, node);
    document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.tone"),
            UiNodeStyle {
                layout: layout::fixed(5.0, OPERAD_ROW_HEIGHT - 12.0),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            inventory_operad_tone_color(row.tone),
            None,
            2.0,
        )),
    );
    let text_column = document.add_child(
        row_node,
        UiNode::container(
            format!("{section_name}.row.{index}.text"),
            UiNodeStyle {
                layout: layout::with_size(
                    layout::column(),
                    layout::px((row_width - 28.0).max(120.0)),
                    layout::px(OPERAD_ROW_HEIGHT - 12.0),
                ),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        ),
    );
    add_inventory_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        inventory_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_inventory_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        inventory_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_inventory_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("inventory.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_inventory_operad_text(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    text: impl Into<String>,
    style: TextStyle,
    height: f32,
) {
    widgets::label(
        document,
        parent,
        name,
        text,
        style,
        layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
    );
}

fn inventory_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn inventory_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn alert_tone(kind: InventoryAlertKind) -> Tone {
    match kind {
        InventoryAlertKind::LowStock => Tone::Warning,
        InventoryAlertKind::Expired => Tone::Danger,
        InventoryAlertKind::ExpiringSoon => Tone::Info,
    }
}

fn truncate_middle(text: impl AsRef<str>, max_chars: usize) -> String {
    let text = text.as_ref();
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let head = keep / 2;
    let tail = keep - head;
    let start = text.chars().take(head).collect::<String>();
    let end = text
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{start}...{end}")
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

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsAction => "action",
            Self::ProductHold => "hold",
            Self::LowStock => "low-stock",
            Self::ExpiringSoon => "expiring",
            Self::InUse => "in-use",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "all" => Some(Self::All),
            "action" => Some(Self::NeedsAction),
            "hold" => Some(Self::ProductHold),
            "low-stock" => Some(Self::LowStock),
            "expiring" => Some(Self::ExpiringSoon),
            "in-use" => Some(Self::InUse),
            _ => None,
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
    if let Some(days) = days_between(DEMO_TODAY, expires_on)
        && days <= 30
    {
        return format!("Expires in {days}d ({date})");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_operad_view_audits_common_widths() {
        let panel = InventoryPanel::from_inventory(Inventory::sample());
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width);
            view.document
                .compute_layout(view.size, &mut ApproxTextMeasurer)
                .unwrap();
            let warnings = view.document.audit_layout();
            assert!(warnings.is_empty(), "{warnings:#?}");
            assert!(view.document.node_count() > 20);
            assert!(!view.document.paint_list().items.is_empty());
        }
    }

    #[test]
    fn inventory_operad_actions_update_panel_state() {
        let mut panel = InventoryPanel::from_inventory(Inventory::sample());
        let mut status = String::new();

        assert!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_FILTER}low-stock"), &mut status)
        );
        assert_eq!(panel.quick_filter, InventoryQuickFilter::LowStock);
        assert!(status.contains("Low stock"));

        let target_lot = panel.visible_lots().first().unwrap().id.clone();
        assert!(panel.handle_operad_action(
            &format!("{OPERAD_ACTION_SELECT_LOT}{target_lot}"),
            &mut status
        ));
        assert_eq!(panel.selected_lot.as_ref(), Some(&target_lot));
        assert!(status.contains(&target_lot.to_string()));
    }
}
