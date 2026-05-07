use eframe::egui;
use layout_model::{
    genealogy::{
        ExcursionQuery, GenealogyEventKind, ImpactRelationship, LotGenealogy, MaterialUse, WaferRef,
    },
    mes::{LotId, WaferId},
};

use crate::ui_chrome;

pub(crate) struct GenealogyPanel {
    genealogy: LotGenealogy,
    selected_lot: Option<LotId>,
    selected_wafer: Option<WaferId>,
    impact_mode: ImpactMode,
}

impl GenealogyPanel {
    pub(crate) fn from_genealogy(genealogy: LotGenealogy) -> Self {
        let selected_lot = genealogy.lot_ids().first().cloned();
        let selected_wafer = selected_lot.as_ref().and_then(|lot_id| {
            genealogy
                .wafer_refs_for_lot(lot_id)
                .first()
                .map(|wafer| wafer.wafer_id.clone())
        });
        Self {
            genealogy,
            selected_lot,
            selected_wafer,
            impact_mode: ImpactMode::LatestToolRun,
        }
    }

    pub(crate) fn genealogy(&self) -> &LotGenealogy {
        &self.genealogy
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        egui::ScrollArea::vertical()
            .id_salt("genealogy_dashboard")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Manufacturing history",
                    "Lot Traceability",
                    "",
                    |ui| {
                        self.lot_picker(ui);
                        self.wafer_picker(ui);
                    },
                );

                let summary = self.genealogy.summary();
                ui.horizontal_wrapped(|ui| {
                    metric(ui, "Lots", summary.lot_count);
                    metric(ui, "Wafers", summary.wafer_count);
                    metric(ui, "Splits", summary.split_count);
                    metric(ui, "Merges", summary.merge_count);
                    metric(ui, "Materials", summary.material_lot_count);
                    metric(ui, "Process records", summary.process_record_count);
                });

                ui.separator();
                ui.columns(2, |columns| {
                    if let Some(lot_id) = self.selected_lot.clone() {
                        self.wafer_rows_ui(&mut columns[0], &lot_id);
                    }

                    if let Some(wafer) = self.selected_wafer_ref() {
                        self.lineage_ui(&mut columns[0], &wafer);
                        self.process_history_ui(&mut columns[1], &wafer);
                        self.material_history_ui(&mut columns[1], &wafer);
                    } else {
                        ui_chrome::empty_state(&mut columns[1], "No wafer selected");
                    }
                });

                ui.separator();
                ui.columns(2, |columns| {
                    self.impact_ui(&mut columns[0]);
                    self.events_ui(&mut columns[1]);
                });
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_selection();
        let summary = self.genealogy.summary();
        ui_chrome::section_label(ui, "Traceability");
        ui.label(format!("Lots: {}", summary.lot_count));
        ui.label(format!("Wafers: {}", summary.wafer_count));
        ui.label(format!("Materials: {}", summary.material_lot_count));
        ui.label(format!("Process records: {}", summary.process_record_count));

        ui.separator();
        if let Some(lot_id) = &self.selected_lot {
            ui.label(format!("Selected lot: {lot_id}"));
            if let Some(lot) = self.genealogy.lots.get(lot_id) {
                ui.label(format!("Product: {}", lot.product));
                ui.label(format!("Route: {}", lot.route_id));
                ui.label(format!("Disposition: {}", lot.disposition.label()));
            }
        }

        ui.separator();
        if let Some(wafer) = self.selected_wafer_ref() {
            ui.label(format!("Selected wafer: {wafer}"));
            ui.label(format!(
                "Lineage depth: {}",
                self.genealogy.wafer_lineage(&wafer).len()
            ));
            ui.label(format!(
                "Descendants: {}",
                self.genealogy.wafer_descendants(&wafer).len()
            ));
        }
    }

    fn ensure_selection(&mut self) {
        let lot_ids = self.genealogy.lot_ids();
        if self
            .selected_lot
            .as_ref()
            .is_none_or(|lot_id| !self.genealogy.lots.contains_key(lot_id))
        {
            self.selected_lot = lot_ids.first().cloned();
        }

        let Some(lot_id) = self.selected_lot.clone() else {
            self.selected_wafer = None;
            return;
        };
        let wafer_refs = self.genealogy.wafer_refs_for_lot(&lot_id);
        if self.selected_wafer.as_ref().is_none_or(|wafer_id| {
            !wafer_refs
                .iter()
                .any(|wafer_ref| &wafer_ref.wafer_id == wafer_id)
        }) {
            self.selected_wafer = wafer_refs.first().map(|wafer| wafer.wafer_id.clone());
        }
    }

    fn selected_wafer_ref(&self) -> Option<WaferRef> {
        Some(WaferRef {
            lot_id: self.selected_lot.clone()?,
            wafer_id: self.selected_wafer.clone()?,
        })
    }

    fn lot_picker(&mut self, ui: &mut egui::Ui) {
        let mut selected = self.selected_lot.clone();
        let selected_text = selected
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "No lot".to_string());
        egui::ComboBox::from_id_salt("genealogy_lot_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for lot_id in self.genealogy.lot_ids() {
                    ui.selectable_value(&mut selected, Some(lot_id.clone()), lot_id.to_string());
                }
            });
        if selected != self.selected_lot {
            self.selected_lot = selected;
            self.selected_wafer = self.selected_lot.as_ref().and_then(|lot_id| {
                self.genealogy
                    .wafer_refs_for_lot(lot_id)
                    .first()
                    .map(|wafer| wafer.wafer_id.clone())
            });
        }
    }

    fn wafer_picker(&mut self, ui: &mut egui::Ui) {
        let Some(lot_id) = self.selected_lot.clone() else {
            return;
        };
        let wafer_refs = self.genealogy.wafer_refs_for_lot(&lot_id);
        let mut selected = self.selected_wafer.clone();
        let selected_text = selected
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "No wafer".to_string());
        egui::ComboBox::from_id_salt("genealogy_wafer_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for wafer in wafer_refs {
                    ui.selectable_value(
                        &mut selected,
                        Some(wafer.wafer_id.clone()),
                        wafer.wafer_id.to_string(),
                    );
                }
            });
        self.selected_wafer = selected;
    }

    fn wafer_rows_ui(&mut self, ui: &mut egui::Ui, lot_id: &LotId) {
        ui_chrome::section_label(ui, "Lot Wafers");
        let rows = self
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .into_iter()
            .filter_map(|wafer_ref| {
                let wafer = self.genealogy.wafer(&wafer_ref)?;
                Some((wafer_ref, wafer.slot, wafer.state.label().to_string()))
            })
            .collect::<Vec<_>>();

        egui::Grid::new("genealogy_wafer_rows")
            .striped(true)
            .min_col_width(64.0)
            .show(ui, |ui| {
                ui.strong("Wafer");
                ui.strong("Slot");
                ui.strong("State");
                ui.end_row();
                for (wafer_ref, slot, state) in rows {
                    let selected = self.selected_wafer.as_ref() == Some(&wafer_ref.wafer_id);
                    if ui
                        .selectable_label(selected, wafer_ref.wafer_id.to_string())
                        .clicked()
                    {
                        self.selected_lot = Some(wafer_ref.lot_id.clone());
                        self.selected_wafer = Some(wafer_ref.wafer_id.clone());
                    }
                    ui.label(slot.to_string());
                    ui.label(state);
                    ui.end_row();
                }
            });
    }

    fn lineage_ui(&self, ui: &mut egui::Ui, wafer: &WaferRef) {
        ui.separator();
        ui_chrome::section_label(ui, "Lineage");
        for item in self.genealogy.wafer_lineage(wafer) {
            let detail = self
                .genealogy
                .wafer(&item)
                .map(|wafer| format!("slot {} / {}", wafer.slot, wafer.state.label()))
                .unwrap_or_default();
            ui.label(format!("{item} {detail}"));
        }
    }

    fn process_history_ui(&self, ui: &mut egui::Ui, wafer: &WaferRef) {
        ui_chrome::section_label(ui, "Inherited Process History");
        let records = self.genealogy.inherited_process_history_for_wafer(wafer);
        if records.is_empty() {
            ui_chrome::empty_state(ui, "No process records");
            return;
        }
        egui::Grid::new("genealogy_process_history")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Seq");
                ui.strong("Step");
                ui.strong("Tool run");
                ui.end_row();
                for record in records.iter().rev().take(10) {
                    ui.label(record.sequence.to_string());
                    ui.label(format!("{} {}", record.step_id, record.step_name));
                    ui.label(record.tool_run_id.to_string());
                    ui.end_row();
                }
            });
    }

    fn material_history_ui(&self, ui: &mut egui::Ui, wafer: &WaferRef) {
        ui.separator();
        ui_chrome::section_label(ui, "Material Ancestry");
        let records = self.genealogy.material_ancestry_for_wafer(wafer);
        if records.is_empty() {
            ui_chrome::empty_state(ui, "No material records");
            return;
        }
        egui::Grid::new("genealogy_material_history")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Material");
                ui.strong("Step");
                ui.strong("Qty");
                ui.end_row();
                for record in records.iter().rev().take(8) {
                    ui.label(material_label(&self.genealogy, record));
                    ui.label(record.step_id.to_string());
                    ui.label(format!(
                        "{} {}",
                        compact_number(record.quantity),
                        record.unit
                    ));
                    ui.end_row();
                }
            });
    }

    fn impact_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Excursion Impact");
        egui::ComboBox::from_id_salt("genealogy_impact_mode")
            .selected_text(self.impact_mode.label())
            .show_ui(ui, |ui| {
                for mode in ImpactMode::ALL {
                    ui.selectable_value(&mut self.impact_mode, mode, mode.label());
                }
            });

        let Some(query) = self.impact_query() else {
            ui_chrome::empty_state(ui, "No matching excursion context for the selected wafer");
            return;
        };
        let impact = self.genealogy.impact_for(query.clone());
        ui.label(query.label());
        ui.label(format!(
            "{} direct wafer(s), {} impacted wafer(s)",
            impact.direct_wafers.len(),
            impact.impacted_wafers.len()
        ));

        egui::Grid::new("genealogy_impact_rows")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Wafer");
                ui.strong("Relationship");
                ui.strong("Latest step");
                ui.end_row();
                for wafer in impact.impacted_wafers.iter().take(12) {
                    ui.label(wafer.wafer.to_string());
                    ui.label(relationship_label(&wafer.relationship));
                    ui.label(
                        wafer
                            .latest_step_id
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.end_row();
                }
            });
    }

    fn impact_query(&self) -> Option<ExcursionQuery> {
        let wafer = self.selected_wafer_ref()?;
        match self.impact_mode {
            ImpactMode::LatestToolRun => self
                .genealogy
                .inherited_process_history_for_wafer(&wafer)
                .last()
                .map(|record| ExcursionQuery::ToolRun {
                    tool_run_id: record.tool_run_id.clone(),
                }),
            ImpactMode::LatestStep => self
                .genealogy
                .inherited_process_history_for_wafer(&wafer)
                .last()
                .map(|record| ExcursionQuery::ProcessStep {
                    step_id: record.step_id.clone(),
                }),
            ImpactMode::LatestMaterial => self
                .genealogy
                .material_ancestry_for_wafer(&wafer)
                .last()
                .map(|record| ExcursionQuery::MaterialLot {
                    material_lot_id: record.material_lot_id.clone(),
                }),
        }
    }

    fn events_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Recent Genealogy Events");
        egui::Grid::new("genealogy_event_rows")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Seq");
                ui.strong("Event");
                ui.end_row();
                for event in self.genealogy.events.iter().rev().take(12) {
                    ui.label(event.sequence.to_string());
                    ui.label(event_label(&event.kind));
                    ui.end_row();
                }
            });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImpactMode {
    LatestToolRun,
    LatestStep,
    LatestMaterial,
}

impl ImpactMode {
    const ALL: [Self; 3] = [Self::LatestToolRun, Self::LatestStep, Self::LatestMaterial];

    fn label(self) -> &'static str {
        match self {
            Self::LatestToolRun => "Latest tool run",
            Self::LatestStep => "Latest step",
            Self::LatestMaterial => "Latest material",
        }
    }
}

fn metric(ui: &mut egui::Ui, label: &str, value: usize) {
    ui_chrome::metric_tile(ui, label, value, "");
}

fn material_label(genealogy: &LotGenealogy, record: &MaterialUse) -> String {
    genealogy
        .material_lots
        .get(&record.material_lot_id)
        .map(|material| format!("{} {}", material.id, material.name))
        .unwrap_or_else(|| record.material_lot_id.to_string())
}

fn relationship_label(relationship: &ImpactRelationship) -> String {
    match relationship {
        ImpactRelationship::Direct => "direct".to_string(),
        ImpactRelationship::Descendant {
            ancestor,
            generations,
        } => format!("{generations} gen from {ancestor}"),
    }
}

fn event_label(kind: &GenealogyEventKind) -> String {
    match kind {
        GenealogyEventKind::LotStarted { lot_id } => format!("started {lot_id}"),
        GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            wafer_count,
            reason,
        } => format!("split {source_lot_id} to {target_lot_id}: {wafer_count} wafers, {reason}"),
        GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            wafer_count,
            reason,
        } => format!(
            "merged {} to {target_lot_id}: {wafer_count} wafers, {reason}",
            source_lot_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn compact_number(value: f64) -> String {
    if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}
