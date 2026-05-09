use std::collections::BTreeSet;

use eframe::egui;
use layout_model::{
    genealogy::{
        ExcursionQuery, GenealogyEvent, GenealogyEventKind, GenealogyLot, GenealogyWafer,
        ImpactRelationship, LotDisposition, LotGenealogy, MaterialLot, MaterialUse,
        WaferGenealogyState, WaferProcessRecord, WaferRef,
    },
    mes::{LotId, WaferId},
};

use crate::ui_chrome;

pub(crate) struct GenealogyPanel {
    genealogy: LotGenealogy,
    selected_lot: Option<LotId>,
    selected_wafer: Option<WaferId>,
    selected_detail: Option<TraceSelection>,
    impact_mode: ImpactMode,
    search_query: String,
    related_only: bool,
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
        let selected_detail = selected_lot.clone().map(|lot_id| {
            selected_wafer
                .clone()
                .map(|wafer_id| {
                    TraceSelection::Wafer(WaferRef {
                        lot_id: lot_id.clone(),
                        wafer_id,
                    })
                })
                .unwrap_or(TraceSelection::Lot(lot_id))
        });
        Self {
            genealogy,
            selected_lot,
            selected_wafer,
            selected_detail,
            impact_mode: ImpactMode::LatestToolRun,
            search_query: String::new(),
            related_only: false,
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
                    "Genealogy, inherited materials, tool runs, and excursion impact",
                    |ui| {
                        self.lot_picker(ui);
                        self.wafer_picker(ui);
                    },
                );

                if self.genealogy.lots.is_empty() {
                    ui_chrome::empty_state(ui, "No traceability data loaded");
                } else {
                    self.summary_ui(ui);
                    self.filter_ui(ui);
                    ui.separator();
                    self.trace_workspace_ui(ui);
                    ui.separator();
                    self.history_workspace_ui(ui);
                }
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
                ui.label(format!("Parent lots: {}", lot.created_from.len()));
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
            ui.label(format!(
                "Inherited records: {}",
                self.genealogy
                    .inherited_process_history_for_wafer(&wafer)
                    .len()
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
            self.selected_detail = None;
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

        if self
            .selected_detail
            .as_ref()
            .is_none_or(|selection| !self.selection_exists(selection))
        {
            self.selected_detail = self.default_trace_selection();
        }
    }

    fn selection_exists(&self, selection: &TraceSelection) -> bool {
        match selection {
            TraceSelection::Lot(lot_id) => self.genealogy.lots.contains_key(lot_id),
            TraceSelection::Wafer(wafer) => self.genealogy.wafer(wafer).is_some(),
            TraceSelection::Process(sequence) => self.process_record(*sequence).is_some(),
            TraceSelection::MaterialUse(sequence) => self.material_use(*sequence).is_some(),
            TraceSelection::Event(sequence) => self.event(*sequence).is_some(),
        }
    }

    fn default_trace_selection(&self) -> Option<TraceSelection> {
        self.selected_wafer_ref()
            .filter(|wafer| self.genealogy.wafer(wafer).is_some())
            .map(TraceSelection::Wafer)
            .or_else(|| self.selected_lot.clone().map(TraceSelection::Lot))
    }

    fn selected_wafer_ref(&self) -> Option<WaferRef> {
        Some(WaferRef {
            lot_id: self.selected_lot.clone()?,
            wafer_id: self.selected_wafer.clone()?,
        })
    }

    fn select_lot(&mut self, lot_id: LotId) {
        self.selected_lot = Some(lot_id.clone());
        self.selected_wafer = self
            .genealogy
            .wafer_refs_for_lot(&lot_id)
            .first()
            .map(|wafer| wafer.wafer_id.clone());
        self.selected_detail = Some(TraceSelection::Lot(lot_id));
    }

    fn select_wafer(&mut self, wafer: WaferRef) {
        self.selected_lot = Some(wafer.lot_id.clone());
        self.selected_wafer = Some(wafer.wafer_id.clone());
        self.selected_detail = Some(TraceSelection::Wafer(wafer));
    }

    fn select_trace(&mut self, selection: TraceSelection) {
        match &selection {
            TraceSelection::Lot(lot_id) => {
                self.selected_lot = Some(lot_id.clone());
                let wafer_still_in_lot = self.selected_wafer.as_ref().is_some_and(|wafer_id| {
                    self.genealogy
                        .wafer_refs_for_lot(lot_id)
                        .iter()
                        .any(|wafer| &wafer.wafer_id == wafer_id)
                });
                if !wafer_still_in_lot {
                    self.selected_wafer = self
                        .genealogy
                        .wafer_refs_for_lot(lot_id)
                        .first()
                        .map(|wafer| wafer.wafer_id.clone());
                }
            }
            TraceSelection::Wafer(wafer) => {
                self.selected_lot = Some(wafer.lot_id.clone());
                self.selected_wafer = Some(wafer.wafer_id.clone());
            }
            TraceSelection::Process(sequence) => {
                if let Some(wafer) = self
                    .process_record(*sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_lot = Some(wafer.lot_id);
                    self.selected_wafer = Some(wafer.wafer_id);
                }
            }
            TraceSelection::MaterialUse(sequence) => {
                if let Some(wafer) = self
                    .material_use(*sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_lot = Some(wafer.lot_id);
                    self.selected_wafer = Some(wafer.wafer_id);
                }
            }
            TraceSelection::Event(_) => {}
        }
        self.selected_detail = Some(selection);
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
            if let Some(lot_id) = selected {
                self.select_lot(lot_id);
            }
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
        if selected != self.selected_wafer {
            if let Some(wafer_id) = selected {
                self.select_wafer(WaferRef { lot_id, wafer_id });
            }
        }
    }

    fn summary_ui(&self, ui: &mut egui::Ui) {
        let summary = self.genealogy.summary();
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Lots",
                    summary.lot_count.to_string(),
                    "genealogy nodes",
                    ui_chrome::Tone::Info,
                ),
                (
                    "Wafers",
                    summary.wafer_count.to_string(),
                    "tracked substrates",
                    ui_chrome::Tone::Neutral,
                ),
                (
                    "Splits",
                    summary.split_count.to_string(),
                    "branch events",
                    ui_chrome::Tone::Warning,
                ),
                (
                    "Merges",
                    summary.merge_count.to_string(),
                    "rejoin events",
                    ui_chrome::Tone::Warning,
                ),
                (
                    "Materials",
                    summary.material_lot_count.to_string(),
                    "qualified lots",
                    ui_chrome::Tone::Success,
                ),
                (
                    "Process Records",
                    summary.process_record_count.to_string(),
                    "wafer history rows",
                    ui_chrome::Tone::Neutral,
                ),
            ],
        );
    }

    fn filter_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("lot, wafer, material, tool, step")
                    .desired_width(260.0),
            );
            ui.checkbox(&mut self.related_only, "Related wafers only");
            if !self.search_query.is_empty() && ui.button("Clear").clicked() {
                self.search_query.clear();
            }
        });
    }

    fn trace_workspace_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 920.0 {
            self.lot_explorer_ui(ui);
            ui.separator();
            self.lineage_ui(ui);
            ui.separator();
            self.selected_detail_ui(ui);
            ui.separator();
            self.impact_ui(ui);
        } else {
            ui.columns(3, |columns| {
                self.lot_explorer_ui(&mut columns[0]);
                self.lineage_ui(&mut columns[1]);
                self.selected_detail_ui(&mut columns[1]);
                self.impact_ui(&mut columns[2]);
            });
        }
    }

    fn history_workspace_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 860.0 {
            self.process_history_ui(ui);
            ui.separator();
            self.material_history_ui(ui);
            ui.separator();
            self.events_ui(ui);
        } else {
            ui.columns(2, |columns| {
                self.process_history_ui(&mut columns[0]);
                self.material_history_ui(&mut columns[0]);
                self.events_ui(&mut columns[1]);
            });
        }
    }

    fn lot_explorer_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Lot Genealogy");
        let search = self.normalized_search();
        let related_lots = self.related_lot_ids();
        let rows = self
            .genealogy
            .lots
            .iter()
            .filter(|(lot_id, lot)| {
                (!self.related_only || related_lots.is_empty() || related_lots.contains(*lot_id))
                    && lot_matches_search(&self.genealogy, lot_id, lot, &search)
            })
            .map(|(lot_id, _)| lot_id.clone())
            .collect::<Vec<_>>();

        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No lots match the current filter");
        } else {
            let mut pending_selection = None;
            egui::ScrollArea::horizontal()
                .id_salt("genealogy_lot_rows_scroll")
                .show(ui, |ui| {
                    egui::Grid::new("genealogy_lot_rows")
                        .striped(true)
                        .min_col_width(68.0)
                        .show(ui, |ui| {
                            ui.strong("Lot");
                            ui.strong("Wafers");
                            ui.strong("State");
                            ui.strong("Relationship");
                            ui.end_row();

                            for lot_id in rows {
                                let Some(lot) = self.genealogy.lots.get(&lot_id) else {
                                    continue;
                                };
                                let selected = self.selected_lot.as_ref() == Some(&lot_id);
                                if ui.selectable_label(selected, lot_id.to_string()).clicked() {
                                    pending_selection = Some(TraceSelection::Lot(lot_id.clone()));
                                }
                                ui.label(lot.wafers.len().to_string());
                                ui_chrome::status_pill(
                                    ui,
                                    lot.disposition.label(),
                                    lot_disposition_tone(&lot.disposition),
                                );
                                ui.label(lot_relationship_label(&self.genealogy, &lot_id, lot));
                                ui.end_row();
                            }
                        });
                });

            if let Some(selection) = pending_selection {
                self.select_trace(selection);
            }
        }

        if let Some(lot_id) = self.selected_lot.clone() {
            self.wafer_rows_ui(ui, &lot_id);
        }
    }

    fn wafer_rows_ui(&mut self, ui: &mut egui::Ui, lot_id: &LotId) {
        ui_chrome::section_label(ui, "Lot Wafers");
        let search = self.normalized_search();
        let related_wafers = self.related_wafer_refs();
        let rows = self
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .into_iter()
            .filter(|wafer_ref| {
                (!self.related_only
                    || related_wafers.is_empty()
                    || related_wafers.contains(wafer_ref))
                    && self
                        .genealogy
                        .wafer(wafer_ref)
                        .is_some_and(|wafer| wafer_matches_search(wafer_ref, wafer, &search))
            })
            .collect::<Vec<_>>();

        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No wafers match the current filter");
            return;
        }

        let mut pending_selection = None;
        egui::ScrollArea::horizontal()
            .id_salt("genealogy_wafer_rows_scroll")
            .show(ui, |ui| {
                egui::Grid::new("genealogy_wafer_rows")
                    .striped(true)
                    .min_col_width(64.0)
                    .show(ui, |ui| {
                        ui.strong("Wafer");
                        ui.strong("Slot");
                        ui.strong("State");
                        ui.strong("Parent");
                        ui.strong("Children");
                        ui.end_row();
                        for wafer_ref in rows {
                            let Some(wafer) = self.genealogy.wafer(&wafer_ref) else {
                                continue;
                            };
                            let selected =
                                self.selected_wafer.as_ref() == Some(&wafer_ref.wafer_id);
                            if ui
                                .selectable_label(selected, wafer_ref.wafer_id.to_string())
                                .clicked()
                            {
                                pending_selection = Some(TraceSelection::Wafer(wafer_ref.clone()));
                            }
                            ui.label(wafer.slot.to_string());
                            ui_chrome::status_pill(
                                ui,
                                wafer.state.label(),
                                wafer_state_tone(&wafer.state),
                            );
                            ui.label(
                                wafer
                                    .parent
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_else(|| "-".to_string()),
                            );
                            ui.label(self.direct_children(&wafer_ref).len().to_string());
                            ui.end_row();
                        }
                    });
            });
        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
    }

    fn lineage_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Parent / Child Flow");
        let Some(selected_wafer) = self.selected_wafer_ref() else {
            ui_chrome::empty_state(ui, "No wafer selected");
            return;
        };

        let lineage = self.genealogy.wafer_lineage(&selected_wafer);
        if lineage.is_empty() {
            ui_chrome::empty_state(ui, "No lineage records");
        } else {
            let mut pending_selection = None;
            egui::Grid::new("genealogy_lineage_rows")
                .striped(true)
                .min_col_width(62.0)
                .show(ui, |ui| {
                    ui.strong("Hop");
                    ui.strong("Wafer");
                    ui.strong("Lot state");
                    ui.strong("Latest step");
                    ui.end_row();

                    for (index, wafer_ref) in lineage.iter().enumerate() {
                        let relation = if wafer_ref == &selected_wafer {
                            "selected".to_string()
                        } else {
                            format!("parent {}", lineage.len().saturating_sub(index + 1))
                        };
                        ui.label(relation);
                        if ui
                            .selectable_label(
                                wafer_ref == &selected_wafer,
                                format!("{}/{}", wafer_ref.lot_id, wafer_ref.wafer_id),
                            )
                            .clicked()
                        {
                            pending_selection = Some(TraceSelection::Wafer(wafer_ref.clone()));
                        }
                        ui.label(
                            self.genealogy
                                .wafer(wafer_ref)
                                .map(|wafer| wafer_state_detail(&wafer.state))
                                .unwrap_or_else(|| "-".to_string()),
                        );
                        ui.label(self.latest_step_label(wafer_ref));
                        ui.end_row();
                    }
                });
            if let Some(selection) = pending_selection {
                self.select_trace(selection);
            }
        }

        ui_chrome::section_label(ui, "Child Wafers");
        let search = self.normalized_search();
        let descendants = self
            .genealogy
            .wafer_descendants(&selected_wafer)
            .into_iter()
            .filter(|wafer_ref| {
                self.genealogy.wafer(wafer_ref).is_some_and(|wafer| {
                    wafer_matches_search(wafer_ref, wafer, &search)
                        || contains_query(&search, self.latest_step_label(wafer_ref))
                })
            })
            .collect::<Vec<_>>();
        if descendants.is_empty() {
            ui_chrome::empty_state(ui, "No child wafers");
            return;
        }

        let mut pending_selection = None;
        egui::Grid::new("genealogy_descendant_rows")
            .striped(true)
            .min_col_width(70.0)
            .show(ui, |ui| {
                ui.strong("Wafer");
                ui.strong("State");
                ui.strong("Latest step");
                ui.end_row();
                for wafer_ref in descendants.iter().take(12) {
                    let selected = self.selected_wafer_ref().as_ref() == Some(wafer_ref);
                    if ui
                        .selectable_label(selected, wafer_ref.to_string())
                        .clicked()
                    {
                        pending_selection = Some(TraceSelection::Wafer(wafer_ref.clone()));
                    }
                    ui.label(
                        self.genealogy
                            .wafer(wafer_ref)
                            .map(|wafer| wafer_state_detail(&wafer.state))
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.label(self.latest_step_label(wafer_ref));
                    ui.end_row();
                }
            });
        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
    }

    fn selected_detail_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Selected Trace Node");
        let Some(selection) = &self.selected_detail else {
            ui_chrome::empty_state(ui, "No trace node selected");
            return;
        };

        match selection {
            TraceSelection::Lot(lot_id) => {
                let Some(lot) = self.genealogy.lots.get(lot_id) else {
                    ui_chrome::empty_state(ui, "Selected lot is unavailable");
                    return;
                };
                self.lot_detail_ui(ui, lot_id, lot);
            }
            TraceSelection::Wafer(wafer_ref) => {
                let Some(wafer) = self.genealogy.wafer(wafer_ref) else {
                    ui_chrome::empty_state(ui, "Selected wafer is unavailable");
                    return;
                };
                self.wafer_detail_ui(ui, wafer_ref, wafer);
            }
            TraceSelection::Process(sequence) => {
                let Some(record) = self.process_record(*sequence) else {
                    ui_chrome::empty_state(ui, "Selected process record is unavailable");
                    return;
                };
                self.process_detail_ui(ui, record);
            }
            TraceSelection::MaterialUse(sequence) => {
                let Some(record) = self.material_use(*sequence) else {
                    ui_chrome::empty_state(ui, "Selected material use is unavailable");
                    return;
                };
                self.material_use_detail_ui(ui, record);
            }
            TraceSelection::Event(sequence) => {
                let Some(event) = self.event(*sequence) else {
                    ui_chrome::empty_state(ui, "Selected audit event is unavailable");
                    return;
                };
                self.event_detail_ui(ui, event);
            }
        }
    }

    fn lot_detail_ui(&self, ui: &mut egui::Ui, lot_id: &LotId, lot: &GenealogyLot) {
        let child_lots = self.child_lot_ids(lot_id);
        let active_wafers = lot
            .wafers
            .values()
            .filter(|wafer| matches!(&wafer.state, WaferGenealogyState::Active))
            .count();
        let transfer_wafers = lot.wafers.len().saturating_sub(active_wafers);
        detail_grid(
            ui,
            "genealogy_lot_detail",
            &[
                ("Lot", lot.id.to_string()),
                ("Product", lot.product.clone()),
                ("Route", lot.route_id.to_string()),
                ("Disposition", lot_disposition_detail(&lot.disposition)),
                ("Parent lots", display_lot_list(&lot.created_from)),
                ("Child lots", display_lot_list(&child_lots)),
                ("Wafers", lot.wafers.len().to_string()),
                (
                    "Active / moved",
                    format!("{active_wafers} / {transfer_wafers}"),
                ),
            ],
        );
    }

    fn wafer_detail_ui(&self, ui: &mut egui::Ui, wafer_ref: &WaferRef, wafer: &GenealogyWafer) {
        let children = self.direct_children(wafer_ref);
        let process_records = self
            .genealogy
            .inherited_process_history_for_wafer(wafer_ref);
        let material_records = self.genealogy.material_ancestry_for_wafer(wafer_ref);
        let direct_process_count = process_records
            .iter()
            .filter(|record| &record.wafer == wafer_ref)
            .count();
        let direct_material_count = material_records
            .iter()
            .filter(|record| &record.wafer == wafer_ref)
            .count();
        detail_grid(
            ui,
            "genealogy_wafer_detail",
            &[
                ("Wafer", wafer_ref.to_string()),
                ("Slot", wafer.slot.to_string()),
                ("Substrate", wafer.substrate.clone()),
                ("State", wafer_state_detail(&wafer.state)),
                (
                    "Parent",
                    wafer
                        .parent
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                ("Children", display_wafer_list(&children)),
                (
                    "Lineage depth",
                    self.genealogy.wafer_lineage(wafer_ref).len().to_string(),
                ),
                (
                    "Descendants",
                    self.genealogy
                        .wafer_descendants(wafer_ref)
                        .len()
                        .to_string(),
                ),
                (
                    "Process rows",
                    format!(
                        "{direct_process_count} direct / {} inherited",
                        process_records.len()
                    ),
                ),
                (
                    "Material rows",
                    format!(
                        "{direct_material_count} direct / {} inherited",
                        material_records.len()
                    ),
                ),
                ("Latest step", self.latest_step_label(wafer_ref)),
            ],
        );
    }

    fn process_detail_ui(&self, ui: &mut egui::Ui, record: &WaferProcessRecord) {
        let material_uses = self
            .genealogy
            .material_uses
            .iter()
            .filter(|material| {
                material.tool_run_id == record.tool_run_id && material.wafer == record.wafer
            })
            .count();
        detail_grid(
            ui,
            "genealogy_process_detail",
            &[
                ("Record", format!("#{}", record.sequence)),
                ("Wafer", record.wafer.to_string()),
                ("Step", format!("{} {}", record.step_id, record.step_name)),
                ("Layer", process_layer_label(record)),
                ("Recipe", record.recipe_id.to_string()),
                ("Tool", record.tool_id.to_string()),
                ("Tool run", record.tool_run_id.to_string()),
                ("Completed", record.completed_at.clone()),
                ("Material uses", material_uses.to_string()),
            ],
        );
    }

    fn material_use_detail_ui(&self, ui: &mut egui::Ui, record: &MaterialUse) {
        let material = self.material_lot(record);
        detail_grid(
            ui,
            "genealogy_material_detail",
            &[
                ("Record", format!("#{}", record.sequence)),
                ("Wafer", record.wafer.to_string()),
                ("Material lot", record.material_lot_id.to_string()),
                (
                    "Material",
                    material
                        .map(|material| material.name.clone())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                (
                    "Supplier",
                    material
                        .map(|material| material.supplier.clone())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                (
                    "Certificate",
                    material
                        .map(|material| material.certificate_id.clone())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                (
                    "Received",
                    material
                        .map(|material| material.received_at.clone())
                        .unwrap_or_else(|| "-".to_string()),
                ),
                ("Step", record.step_id.to_string()),
                ("Tool run", record.tool_run_id.to_string()),
                (
                    "Quantity",
                    format!("{} {}", compact_number(record.quantity), record.unit),
                ),
            ],
        );
    }

    fn event_detail_ui(&self, ui: &mut egui::Ui, event: &GenealogyEvent) {
        detail_grid(
            ui,
            "genealogy_event_detail",
            &[
                ("Event", format!("#{}", event.sequence)),
                ("Operation", event_kind_name(&event.kind).to_string()),
                ("Scope", event_scope(&event.kind)),
                ("Reason", event_reason(&event.kind)),
                ("Audit text", event_label(&event.kind)),
            ],
        );
    }

    fn process_history_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Inherited Process History");
        let Some(wafer) = self.selected_wafer_ref() else {
            ui_chrome::empty_state(ui, "No wafer selected");
            return;
        };
        let search = self.normalized_search();
        let records = self
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .into_iter()
            .rev()
            .filter(|record| process_record_matches(record, &search))
            .take(24)
            .collect::<Vec<_>>();
        if records.is_empty() {
            ui_chrome::empty_state(ui, "No process records match the current filter");
            return;
        }

        let mut pending_selection = None;
        egui::ScrollArea::horizontal()
            .id_salt("genealogy_process_history_scroll")
            .show(ui, |ui| {
                egui::Grid::new("genealogy_process_history")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Seq");
                        ui.strong("Source");
                        ui.strong("Step");
                        ui.strong("Tool run");
                        ui.strong("Recipe");
                        ui.strong("Completed");
                        ui.end_row();
                        for record in records {
                            let selected = self.selected_detail
                                == Some(TraceSelection::Process(record.sequence));
                            if ui
                                .selectable_label(selected, record.sequence.to_string())
                                .clicked()
                            {
                                pending_selection = Some(TraceSelection::Process(record.sequence));
                            }
                            ui.label(if record.wafer == wafer {
                                "direct"
                            } else {
                                "parent"
                            });
                            ui.label(format!("{} {}", record.step_id, record.step_name));
                            ui.label(format!("{} / {}", record.tool_id, record.tool_run_id));
                            ui.label(record.recipe_id.to_string());
                            ui.label(record.completed_at.clone());
                            ui.end_row();
                        }
                    });
            });

        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
    }

    fn material_history_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Material Ancestry");
        let Some(wafer) = self.selected_wafer_ref() else {
            ui_chrome::empty_state(ui, "No wafer selected");
            return;
        };
        let search = self.normalized_search();
        let records = self
            .genealogy
            .material_ancestry_for_wafer(&wafer)
            .into_iter()
            .rev()
            .filter(|record| material_use_matches(&self.genealogy, record, &search))
            .take(18)
            .collect::<Vec<_>>();
        if records.is_empty() {
            ui_chrome::empty_state(ui, "No material records match the current filter");
            return;
        }

        let mut pending_selection = None;
        egui::ScrollArea::horizontal()
            .id_salt("genealogy_material_history_scroll")
            .show(ui, |ui| {
                egui::Grid::new("genealogy_material_history")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Seq");
                        ui.strong("Source");
                        ui.strong("Material");
                        ui.strong("Step");
                        ui.strong("Tool run");
                        ui.strong("Qty");
                        ui.end_row();
                        for record in records {
                            let selected = self.selected_detail
                                == Some(TraceSelection::MaterialUse(record.sequence));
                            if ui
                                .selectable_label(selected, record.sequence.to_string())
                                .clicked()
                            {
                                pending_selection =
                                    Some(TraceSelection::MaterialUse(record.sequence));
                            }
                            ui.label(if record.wafer == wafer {
                                "direct"
                            } else {
                                "parent"
                            });
                            ui.label(material_label(&self.genealogy, record));
                            ui.label(record.step_id.to_string());
                            ui.label(record.tool_run_id.to_string());
                            ui.label(format!(
                                "{} {}",
                                compact_number(record.quantity),
                                record.unit
                            ));
                            ui.end_row();
                        }
                    });
            });

        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
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
        ui.label(egui::RichText::new(query.label()).strong());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} direct", impact.direct_wafers.len()),
                ui_chrome::Tone::Info,
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} impacted", impact.impacted_wafers.len()),
                ui_chrome::Tone::Warning,
            );
            ui_chrome::status_pill(
                ui,
                &format!(
                    "{} evidence",
                    impact.matching_process_records.len() + impact.matching_material_uses.len()
                ),
                ui_chrome::Tone::Neutral,
            );
        });

        let search = self.normalized_search();
        let rows = impact
            .impacted_wafers
            .iter()
            .filter(|impact| impact_row_matches(impact, &search))
            .take(18)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No impacted wafers match the current filter");
            return;
        }

        let mut pending_selection = None;
        egui::ScrollArea::horizontal()
            .id_salt("genealogy_impact_rows_scroll")
            .show(ui, |ui| {
                egui::Grid::new("genealogy_impact_rows")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Wafer");
                        ui.strong("Relationship");
                        ui.strong("Latest step");
                        ui.end_row();
                        for wafer in rows {
                            let selected = self.selected_wafer_ref().as_ref() == Some(&wafer.wafer);
                            if ui
                                .selectable_label(selected, wafer.wafer.to_string())
                                .clicked()
                            {
                                pending_selection =
                                    Some(TraceSelection::Wafer(wafer.wafer.clone()));
                            }
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
            });

        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
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

    fn events_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Audit Trail");
        let search = self.normalized_search();
        let events = self
            .genealogy
            .events
            .iter()
            .rev()
            .filter(|event| event_matches(event, &search))
            .take(24)
            .collect::<Vec<_>>();
        if events.is_empty() {
            ui_chrome::empty_state(ui, "No audit events match the current filter");
            return;
        }

        let mut pending_selection = None;
        egui::ScrollArea::horizontal()
            .id_salt("genealogy_event_rows_scroll")
            .show(ui, |ui| {
                egui::Grid::new("genealogy_event_rows")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Seq");
                        ui.strong("Operation");
                        ui.strong("Scope");
                        ui.strong("Reason");
                        ui.end_row();
                        for event in events {
                            let selected =
                                self.selected_detail == Some(TraceSelection::Event(event.sequence));
                            if ui
                                .selectable_label(selected, event.sequence.to_string())
                                .clicked()
                            {
                                pending_selection = Some(TraceSelection::Event(event.sequence));
                            }
                            ui.label(event_kind_name(&event.kind));
                            ui.label(event_scope(&event.kind));
                            ui.label(event_reason(&event.kind));
                            ui.end_row();
                        }
                    });
            });

        if let Some(selection) = pending_selection {
            self.select_trace(selection);
        }
    }

    fn normalized_search(&self) -> String {
        self.search_query.trim().to_ascii_lowercase()
    }

    fn related_lot_ids(&self) -> BTreeSet<LotId> {
        self.related_wafer_refs()
            .into_iter()
            .map(|wafer| wafer.lot_id)
            .collect()
    }

    fn related_wafer_refs(&self) -> BTreeSet<WaferRef> {
        let Some(wafer) = self.selected_wafer_ref() else {
            return BTreeSet::new();
        };
        self.genealogy
            .wafer_lineage(&wafer)
            .into_iter()
            .chain(self.genealogy.wafer_descendants(&wafer))
            .collect()
    }

    fn child_lot_ids(&self, parent_lot_id: &LotId) -> Vec<LotId> {
        self.genealogy
            .lots
            .values()
            .filter(|lot| {
                lot.created_from
                    .iter()
                    .any(|lot_id| lot_id == parent_lot_id)
            })
            .map(|lot| lot.id.clone())
            .collect()
    }

    fn direct_children(&self, parent: &WaferRef) -> Vec<WaferRef> {
        self.genealogy
            .lots
            .iter()
            .flat_map(|(lot_id, lot)| {
                lot.wafers.values().filter_map(|wafer| {
                    if wafer.parent.as_ref() == Some(parent) {
                        Some(WaferRef {
                            lot_id: lot_id.clone(),
                            wafer_id: wafer.id.clone(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn latest_step_label(&self, wafer: &WaferRef) -> String {
        self.genealogy
            .inherited_process_history_for_wafer(wafer)
            .last()
            .map(|record| format!("{} {}", record.step_id, record.step_name))
            .unwrap_or_else(|| "-".to_string())
    }

    fn process_record(&self, sequence: u64) -> Option<&WaferProcessRecord> {
        self.genealogy
            .process_history
            .iter()
            .find(|record| record.sequence == sequence)
    }

    fn material_use(&self, sequence: u64) -> Option<&MaterialUse> {
        self.genealogy
            .material_uses
            .iter()
            .find(|record| record.sequence == sequence)
    }

    fn material_lot(&self, record: &MaterialUse) -> Option<&MaterialLot> {
        self.genealogy.material_lots.get(&record.material_lot_id)
    }

    fn event(&self, sequence: u64) -> Option<&GenealogyEvent> {
        self.genealogy
            .events
            .iter()
            .find(|event| event.sequence == sequence)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TraceSelection {
    Lot(LotId),
    Wafer(WaferRef),
    Process(u64),
    MaterialUse(u64),
    Event(u64),
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

fn detail_grid(ui: &mut egui::Ui, id: &'static str, rows: &[(&str, String)]) {
    egui::Grid::new(id)
        .striped(false)
        .min_col_width(86.0)
        .show(ui, |ui| {
            for (label, value) in rows {
                ui.label(egui::RichText::new(*label).color(ui.visuals().weak_text_color()));
                ui.add(egui::Label::new(value.as_str()).wrap());
                ui.end_row();
            }
        });
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

fn event_kind_name(kind: &GenealogyEventKind) -> &'static str {
    match kind {
        GenealogyEventKind::LotStarted { .. } => "Lot started",
        GenealogyEventKind::LotSplit { .. } => "Lot split",
        GenealogyEventKind::LotMerge { .. } => "Lot merge",
    }
}

fn event_scope(kind: &GenealogyEventKind) -> String {
    match kind {
        GenealogyEventKind::LotStarted { lot_id } => lot_id.to_string(),
        GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            wafer_count,
            ..
        } => format!("{source_lot_id} -> {target_lot_id} ({wafer_count} wafers)"),
        GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            wafer_count,
            ..
        } => format!(
            "{} -> {target_lot_id} ({wafer_count} wafers)",
            display_lot_list(source_lot_ids)
        ),
    }
}

fn event_reason(kind: &GenealogyEventKind) -> String {
    match kind {
        GenealogyEventKind::LotStarted { .. } => "initial registration".to_string(),
        GenealogyEventKind::LotSplit { reason, .. }
        | GenealogyEventKind::LotMerge { reason, .. } => reason.clone(),
    }
}

fn compact_number(value: f64) -> String {
    if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn lot_matches_search(
    genealogy: &LotGenealogy,
    lot_id: &LotId,
    lot: &GenealogyLot,
    query: &str,
) -> bool {
    matches_any(
        query,
        vec![
            lot_id.to_string(),
            lot.product.clone(),
            lot.route_id.to_string(),
            lot.disposition.label().to_string(),
            lot_relationship_label(genealogy, lot_id, lot),
            display_lot_list(&lot.created_from),
            lot.wafers
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        ],
    )
}

fn wafer_matches_search(wafer_ref: &WaferRef, wafer: &GenealogyWafer, query: &str) -> bool {
    matches_any(
        query,
        vec![
            wafer_ref.to_string(),
            wafer_ref.lot_id.to_string(),
            wafer_ref.wafer_id.to_string(),
            wafer.slot.to_string(),
            wafer.substrate.clone(),
            wafer_state_detail(&wafer.state),
            wafer
                .parent
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ],
    )
}

fn process_record_matches(record: &WaferProcessRecord, query: &str) -> bool {
    matches_any(
        query,
        vec![
            record.sequence.to_string(),
            record.wafer.to_string(),
            record.step_id.to_string(),
            record.step_name.clone(),
            process_layer_label(record),
            record.recipe_id.to_string(),
            record.tool_id.to_string(),
            record.tool_run_id.to_string(),
            record.completed_at.clone(),
        ],
    )
}

fn material_use_matches(genealogy: &LotGenealogy, record: &MaterialUse, query: &str) -> bool {
    let material = genealogy.material_lots.get(&record.material_lot_id);
    matches_any(
        query,
        vec![
            record.sequence.to_string(),
            record.wafer.to_string(),
            record.material_lot_id.to_string(),
            material
                .map(|material| material.name.clone())
                .unwrap_or_default(),
            material
                .map(|material| material.supplier.clone())
                .unwrap_or_default(),
            material
                .map(|material| material.certificate_id.clone())
                .unwrap_or_default(),
            record.step_id.to_string(),
            record.tool_run_id.to_string(),
            record.unit.clone(),
        ],
    )
}

fn event_matches(event: &GenealogyEvent, query: &str) -> bool {
    matches_any(
        query,
        vec![
            event.sequence.to_string(),
            event_kind_name(&event.kind).to_string(),
            event_scope(&event.kind),
            event_reason(&event.kind),
            event_label(&event.kind),
        ],
    )
}

fn impact_row_matches(impact: &layout_model::genealogy::ImpactedWafer, query: &str) -> bool {
    matches_any(
        query,
        vec![
            impact.wafer.to_string(),
            relationship_label(&impact.relationship),
            impact
                .latest_step_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ],
    )
}

fn contains_query(query: &str, value: impl ToString) -> bool {
    query.is_empty() || value.to_string().to_ascii_lowercase().contains(query)
}

fn matches_any(query: &str, values: Vec<String>) -> bool {
    query.is_empty()
        || values
            .into_iter()
            .any(|value| value.to_ascii_lowercase().contains(query))
}

fn lot_relationship_label(genealogy: &LotGenealogy, lot_id: &LotId, lot: &GenealogyLot) -> String {
    let child_count = genealogy
        .lots
        .values()
        .filter(|candidate| candidate.created_from.iter().any(|parent| parent == lot_id))
        .count();
    let origin = match lot.created_from.len() {
        0 => "root lot".to_string(),
        1 => format!("split from {}", lot.created_from[0]),
        _ => format!("merge of {}", display_lot_list(&lot.created_from)),
    };
    if child_count == 0 {
        origin
    } else {
        format!("{origin}; {child_count} child lot(s)")
    }
}

fn lot_disposition_detail(disposition: &LotDisposition) -> String {
    match disposition {
        LotDisposition::Active => "active".to_string(),
        LotDisposition::Closed { reason } => format!("closed: {reason}"),
    }
}

fn lot_disposition_tone(disposition: &LotDisposition) -> ui_chrome::Tone {
    match disposition {
        LotDisposition::Active => ui_chrome::Tone::Success,
        LotDisposition::Closed { .. } => ui_chrome::Tone::Warning,
    }
}

fn wafer_state_detail(state: &WaferGenealogyState) -> String {
    match state {
        WaferGenealogyState::Active => "active".to_string(),
        WaferGenealogyState::SplitTo { lot_id } => format!("split to {lot_id}"),
        WaferGenealogyState::MergedTo { lot_id } => format!("merged to {lot_id}"),
        WaferGenealogyState::Scrapped { reason } => format!("scrapped: {reason}"),
    }
}

fn wafer_state_tone(state: &WaferGenealogyState) -> ui_chrome::Tone {
    match state {
        WaferGenealogyState::Active => ui_chrome::Tone::Success,
        WaferGenealogyState::SplitTo { .. } | WaferGenealogyState::MergedTo { .. } => {
            ui_chrome::Tone::Info
        }
        WaferGenealogyState::Scrapped { .. } => ui_chrome::Tone::Danger,
    }
}

fn process_layer_label(record: &WaferProcessRecord) -> String {
    record
        .process_layer
        .map(|layer| layer.as_technology_name().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn display_lot_list(lots: &[LotId]) -> String {
    if lots.is_empty() {
        "-".to_string()
    } else {
        lots.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn display_wafer_list(wafers: &[WaferRef]) -> String {
    if wafers.is_empty() {
        "-".to_string()
    } else {
        wafers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
