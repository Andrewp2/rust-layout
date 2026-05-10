use std::collections::BTreeSet;

use eframe::egui::{self, Color32, Sense, vec2};
use layout_model::{
    genealogy::{
        ExcursionQuery, GenealogyEvent, GenealogyEventKind, GenealogyLot, GenealogyWafer,
        ImpactRelationship, LotDisposition, LotGenealogy, MaterialLot, MaterialUse,
        WaferGenealogyState, WaferProcessRecord, WaferRef,
    },
    mes::{LotId, WaferId},
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

const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SELECT_LOT: &str = "genealogy.action.select_lot.";
const OPERAD_ACTION_SELECT_WAFER: &str = "genealogy.action.select_wafer.";
const OPERAD_ACTION_SELECT_TRACE: &str = "genealogy.action.select_trace.";
const OPERAD_ACTION_IMPACT_MODE: &str = "genealogy.action.impact_mode.";
const OPERAD_ACTION_TOGGLE_RELATED: &str = "genealogy.action.toggle_related";
const OPERAD_ACTION_CLEAR_SEARCH: &str = "genealogy.action.clear_search";

pub(crate) struct GenealogyPanel {
    genealogy: LotGenealogy,
    selected_lot: Option<LotId>,
    selected_wafer: Option<WaferId>,
    selected_detail: Option<TraceSelection>,
    impact_mode: ImpactMode,
    search_query: String,
    related_only: bool,
}

#[derive(Debug)]
struct GenealogyOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct GenealogyMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct GenealogyOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
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
        if let Err(error) = self.operad_ui(ui) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("genealogy_dashboard_operad_scroll")
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
                    && self.handle_operad_action(&node_name)
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

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui) {
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
        if self.operad_context_ui(ui).is_err() {
            self.egui_context_ui(ui);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui) -> Result<(), String> {
        self.ensure_selection();
        let summary = self.genealogy.summary();
        let mut sections = vec![
            SidecarSection::new("Traceability")
                .row(SidecarRow::new(
                    "Lots / wafers",
                    format!(
                        "{} lots | {} wafers",
                        summary.lot_count, summary.wafer_count
                    ),
                    Tone::Info,
                ))
                .row(SidecarRow::new(
                    "Materials / records",
                    format!(
                        "{} material lots | {} process records",
                        summary.material_lot_count, summary.process_record_count
                    ),
                    Tone::Neutral,
                )),
        ];
        if let Some(lot_id) = &self.selected_lot
            && let Some(lot) = self.genealogy.lots.get(lot_id)
        {
            sections.push(
                SidecarSection::new("Selected Lot")
                    .row(
                        SidecarRow::new(
                            lot_id.to_string(),
                            format!("{} | {}", lot.product, lot.disposition.label()),
                            lot_disposition_tone(&lot.disposition),
                        )
                        .selected(true),
                    )
                    .row(SidecarRow::new(
                        "Route / parents",
                        format!("{} | {} parent lots", lot.route_id, lot.created_from.len()),
                        Tone::Neutral,
                    )),
            );
        }
        if let Some(wafer) = self.selected_wafer_ref() {
            sections.push(
                SidecarSection::new("Selected Wafer")
                    .row(
                        SidecarRow::new(
                            wafer.to_string(),
                            format!(
                                "{} lineage links | {} descendants",
                                self.genealogy.wafer_lineage(&wafer).len(),
                                self.genealogy.wafer_descendants(&wafer).len()
                            ),
                            Tone::Info,
                        )
                        .selected(true),
                    )
                    .row(SidecarRow::new(
                        "Inherited records",
                        self.genealogy
                            .inherited_process_history_for_wafer(&wafer)
                            .len()
                            .to_string(),
                        Tone::Neutral,
                    )),
            );
        }
        render_sidecar(ui, "genealogy.context", &sections)
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui) {
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

    fn build_operad_view(&self, width: f32) -> GenealogyOperadView {
        let metrics = self.operad_metrics();
        let control_rows = self.operad_control_rows();
        let lot_rows = self.operad_lot_rows();
        let wafer_rows = self.operad_wafer_rows();
        let lineage_rows = self.operad_lineage_rows();
        let detail_rows = self.operad_detail_rows();
        let impact_rows = self.operad_impact_rows();
        let process_rows = self.operad_process_rows();
        let material_rows = self.operad_material_rows();
        let event_rows = self.operad_event_rows();
        let height = genealogy_operad_view_height(
            width,
            metrics.len(),
            &[
                control_rows.len(),
                lot_rows.len(),
                wafer_rows.len(),
                lineage_rows.len(),
                detail_rows.len(),
                impact_rows.len(),
                process_rows.len(),
                material_rows.len(),
                event_rows.len(),
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

        add_genealogy_operad_header(
            &mut document,
            root,
            "MANUFACTURING HISTORY",
            "Lot Traceability",
            "Genealogy, inherited materials, process history, and excursion impact",
            &format!(
                "{} lot(s), {} wafer(s), {} audit event(s)",
                self.genealogy.lots.len(),
                self.genealogy
                    .lots
                    .values()
                    .map(|lot| lot.wafers.len())
                    .sum::<usize>(),
                self.genealogy.events.len()
            ),
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_metric_grid(&mut document, root, width, &metrics);
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.controls",
            "Trace Controls",
            "No trace controls available",
            &control_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.lots",
            "Lot Genealogy",
            "No lots match the current filter",
            &lot_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.wafers",
            "Lot Wafers",
            "No wafers match the current filter",
            &wafer_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.lineage",
            "Parent / Child Flow",
            "No lineage records loaded",
            &lineage_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.detail",
            "Selected Trace Node",
            "No trace node selected",
            &detail_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.impact",
            "Excursion Impact",
            "No matching excursion context for the selected wafer",
            &impact_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.process",
            "Inherited Process History",
            "No process records match the current filter",
            &process_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.materials",
            "Material Ancestry",
            "No material records match the current filter",
            &material_rows,
        );
        add_genealogy_operad_spacer(&mut document, root, OPERAD_GAP);
        add_genealogy_operad_section(
            &mut document,
            root,
            width,
            "genealogy.events",
            "Audit Trail",
            "No audit events match the current filter",
            &event_rows,
        );

        GenealogyOperadView { document, size }
    }

    fn operad_metrics(&self) -> Vec<GenealogyMetricTile> {
        let summary = self.genealogy.summary();
        vec![
            GenealogyMetricTile {
                label: "Lots".to_string(),
                value: summary.lot_count.to_string(),
                detail: "genealogy nodes".to_string(),
                tone: Tone::Info,
            },
            GenealogyMetricTile {
                label: "Wafers".to_string(),
                value: summary.wafer_count.to_string(),
                detail: "tracked substrates".to_string(),
                tone: Tone::Neutral,
            },
            GenealogyMetricTile {
                label: "Splits".to_string(),
                value: summary.split_count.to_string(),
                detail: "branch events".to_string(),
                tone: Tone::Warning,
            },
            GenealogyMetricTile {
                label: "Merges".to_string(),
                value: summary.merge_count.to_string(),
                detail: "rejoin events".to_string(),
                tone: Tone::Warning,
            },
            GenealogyMetricTile {
                label: "Materials".to_string(),
                value: summary.material_lot_count.to_string(),
                detail: "qualified lots".to_string(),
                tone: Tone::Success,
            },
            GenealogyMetricTile {
                label: "Process Records".to_string(),
                value: summary.process_record_count.to_string(),
                detail: "wafer history rows".to_string(),
                tone: Tone::Neutral,
            },
        ]
    }

    fn operad_control_rows(&self) -> Vec<GenealogyOperadRow> {
        let selected_lot = self
            .selected_lot
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "No lot selected".to_string());
        let selected_wafer = self
            .selected_wafer_ref()
            .map(|wafer| wafer.to_string())
            .unwrap_or_else(|| "No wafer selected".to_string());
        let mut rows = vec![
            genealogy_operad_row(
                format!("Selected lot: {selected_lot}"),
                format!("Selected wafer: {selected_wafer}"),
                Tone::Info,
                self.selected_lot.as_ref().map(|lot_id| {
                    format!("{OPERAD_ACTION_SELECT_LOT}{}:controls", lot_id.as_str())
                }),
                true,
            ),
            genealogy_operad_row(
                if self.search_query.trim().is_empty() {
                    "Search inactive".to_string()
                } else {
                    format!("Search: {}", self.search_query.trim())
                },
                "Text search stays on the fallback egui path until Operad has full edit routing"
                    .to_string(),
                Tone::Neutral,
                (!self.search_query.trim().is_empty())
                    .then(|| OPERAD_ACTION_CLEAR_SEARCH.to_string()),
                false,
            ),
            genealogy_operad_row(
                if self.related_only {
                    "Related wafers only: on".to_string()
                } else {
                    "Related wafers only: off".to_string()
                },
                "Toggle parent, selected, and descendant scope".to_string(),
                if self.related_only {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(OPERAD_ACTION_TOGGLE_RELATED.to_string()),
                self.related_only,
            ),
        ];
        rows.extend(ImpactMode::ALL.into_iter().map(|mode| {
            genealogy_operad_row(
                format!("Impact mode: {}", mode.label()),
                "Use this context for excursion impact rows".to_string(),
                if self.impact_mode == mode {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_IMPACT_MODE}{}:controls",
                    mode.slug()
                )),
                self.impact_mode == mode,
            )
        }));
        rows
    }

    fn operad_lot_rows(&self) -> Vec<GenealogyOperadRow> {
        let search = self.normalized_search();
        let related_lots = self.related_lot_ids();
        let lots = self
            .genealogy
            .lots
            .iter()
            .filter(|(lot_id, lot)| {
                (!self.related_only || related_lots.is_empty() || related_lots.contains(*lot_id))
                    && lot_matches_search(&self.genealogy, lot_id, lot, &search)
            })
            .map(|(lot_id, _)| lot_id.clone())
            .collect::<Vec<_>>();
        let total = lots.len();
        let mut rows = lots
            .into_iter()
            .take(28)
            .enumerate()
            .filter_map(|(index, lot_id)| {
                let lot = self.genealogy.lots.get(&lot_id)?;
                Some(genealogy_operad_row(
                    format!("{} · {}", lot_id, lot.disposition.label()),
                    format!(
                        "{} wafer(s) · {} · {}",
                        lot.wafers.len(),
                        lot_relationship_label(&self.genealogy, &lot_id, lot),
                        lot.route_id
                    ),
                    lot_disposition_tone(&lot.disposition),
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_TRACE}lot:{}:lot.{index}",
                        lot_id.as_str()
                    )),
                    self.selected_lot.as_ref() == Some(&lot_id),
                ))
            })
            .collect::<Vec<_>>();
        if total > rows.len() {
            rows.push(genealogy_operad_row(
                format!("Showing first {} of {total} matching lots", rows.len()),
                "Narrow the search or turn on related-only scope to reduce this list".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_wafer_rows(&self) -> Vec<GenealogyOperadRow> {
        let Some(lot_id) = self.selected_lot.as_ref() else {
            return Vec::new();
        };
        let search = self.normalized_search();
        let related_wafers = self.related_wafer_refs();
        let wafers = self
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
        let total = wafers.len();
        let mut rows = wafers
            .into_iter()
            .take(32)
            .enumerate()
            .filter_map(|(index, wafer_ref)| {
                let wafer = self.genealogy.wafer(&wafer_ref)?;
                Some(genealogy_operad_row(
                    format!("{} · slot {}", wafer_ref.wafer_id, wafer.slot),
                    format!(
                        "{} · parent {} · {} child wafer(s)",
                        wafer_state_detail(&wafer.state),
                        wafer
                            .parent
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "-".to_string()),
                        self.direct_children(&wafer_ref).len()
                    ),
                    wafer_state_tone(&wafer.state),
                    Some(format!(
                        "{OPERAD_ACTION_SELECT_WAFER}{}:{}:wafer.{index}",
                        wafer_ref.lot_id.as_str(),
                        wafer_ref.wafer_id.as_str()
                    )),
                    self.selected_wafer.as_ref() == Some(&wafer_ref.wafer_id),
                ))
            })
            .collect::<Vec<_>>();
        if total > rows.len() {
            rows.push(genealogy_operad_row(
                format!("Showing first {} of {total} matching wafers", rows.len()),
                "Narrow the search or turn on related-only scope to reduce this list".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_lineage_rows(&self) -> Vec<GenealogyOperadRow> {
        let Some(selected_wafer) = self.selected_wafer_ref() else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let lineage = self.genealogy.wafer_lineage(&selected_wafer);
        for (index, wafer_ref) in lineage.iter().enumerate() {
            let relation = if wafer_ref == &selected_wafer {
                "selected".to_string()
            } else {
                format!("parent {}", lineage.len().saturating_sub(index + 1))
            };
            rows.push(genealogy_operad_row(
                format!("{relation} · {wafer_ref}"),
                format!(
                    "{} · latest {}",
                    self.genealogy
                        .wafer(wafer_ref)
                        .map(|wafer| wafer_state_detail(&wafer.state))
                        .unwrap_or_else(|| "-".to_string()),
                    self.latest_step_label(wafer_ref)
                ),
                if wafer_ref == &selected_wafer {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(trace_action_name(
                    &TraceSelection::Wafer(wafer_ref.clone()),
                    &format!("lineage.{index}"),
                )),
                wafer_ref == &selected_wafer,
            ));
        }

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
            rows.push(genealogy_operad_row(
                "No child wafers".to_string(),
                "Selected wafer has no downstream split or merge descendants".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        } else {
            let total = descendants.len();
            for (index, wafer_ref) in descendants.into_iter().take(14).enumerate() {
                rows.push(genealogy_operad_row(
                    format!("child · {wafer_ref}"),
                    format!(
                        "{} · latest {}",
                        self.genealogy
                            .wafer(&wafer_ref)
                            .map(|wafer| wafer_state_detail(&wafer.state))
                            .unwrap_or_else(|| "-".to_string()),
                        self.latest_step_label(&wafer_ref)
                    ),
                    Tone::Info,
                    Some(trace_action_name(
                        &TraceSelection::Wafer(wafer_ref.clone()),
                        &format!("descendant.{index}"),
                    )),
                    self.selected_wafer_ref().as_ref() == Some(&wafer_ref),
                ));
            }
            if total > 14 {
                rows.push(genealogy_operad_row(
                    format!("Showing first 14 of {total} child wafers"),
                    "Use search to focus the descendant list".to_string(),
                    Tone::Neutral,
                    None,
                    false,
                ));
            }
        }
        rows
    }

    fn operad_detail_rows(&self) -> Vec<GenealogyOperadRow> {
        let Some(selection) = &self.selected_detail else {
            return Vec::new();
        };
        match selection {
            TraceSelection::Lot(lot_id) => {
                let Some(lot) = self.genealogy.lots.get(lot_id) else {
                    return vec![genealogy_operad_row(
                        "Selected lot is unavailable".to_string(),
                        lot_id.to_string(),
                        Tone::Warning,
                        None,
                        false,
                    )];
                };
                let child_lots = self.child_lot_ids(lot_id);
                let active_wafers = lot
                    .wafers
                    .values()
                    .filter(|wafer| matches!(&wafer.state, WaferGenealogyState::Active))
                    .count();
                let transfer_wafers = lot.wafers.len().saturating_sub(active_wafers);
                vec![
                    genealogy_detail_row("Lot", lot.id.to_string(), Tone::Info),
                    genealogy_detail_row("Product", lot.product.clone(), Tone::Neutral),
                    genealogy_detail_row("Route", lot.route_id.to_string(), Tone::Neutral),
                    genealogy_detail_row(
                        "Disposition",
                        lot_disposition_detail(&lot.disposition),
                        lot_disposition_tone(&lot.disposition),
                    ),
                    genealogy_detail_row(
                        "Parent lots",
                        display_lot_list(&lot.created_from),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row("Child lots", display_lot_list(&child_lots), Tone::Info),
                    genealogy_detail_row(
                        "Active / moved",
                        format!("{active_wafers} / {transfer_wafers}"),
                        Tone::Neutral,
                    ),
                ]
            }
            TraceSelection::Wafer(wafer_ref) => {
                let Some(wafer) = self.genealogy.wafer(wafer_ref) else {
                    return vec![genealogy_operad_row(
                        "Selected wafer is unavailable".to_string(),
                        wafer_ref.to_string(),
                        Tone::Warning,
                        None,
                        false,
                    )];
                };
                let children = self.direct_children(wafer_ref);
                let process_records = self
                    .genealogy
                    .inherited_process_history_for_wafer(wafer_ref);
                let material_records = self.genealogy.material_ancestry_for_wafer(wafer_ref);
                vec![
                    genealogy_detail_row("Wafer", wafer_ref.to_string(), Tone::Info),
                    genealogy_detail_row(
                        "Slot / substrate",
                        format!("{} / {}", wafer.slot, wafer.substrate),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "State",
                        wafer_state_detail(&wafer.state),
                        wafer_state_tone(&wafer.state),
                    ),
                    genealogy_detail_row(
                        "Parent",
                        wafer
                            .parent
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "-".to_string()),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row("Children", display_wafer_list(&children), Tone::Info),
                    genealogy_detail_row(
                        "Lineage / descendants",
                        format!(
                            "{} / {}",
                            self.genealogy.wafer_lineage(wafer_ref).len(),
                            self.genealogy.wafer_descendants(wafer_ref).len()
                        ),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "Process / material rows",
                        format!("{} / {}", process_records.len(), material_records.len()),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "Latest step",
                        self.latest_step_label(wafer_ref),
                        Tone::Info,
                    ),
                ]
            }
            TraceSelection::Process(sequence) => {
                let Some(record) = self.process_record(*sequence) else {
                    return vec![genealogy_operad_row(
                        "Selected process record is unavailable".to_string(),
                        format!("#{sequence}"),
                        Tone::Warning,
                        None,
                        false,
                    )];
                };
                vec![
                    genealogy_detail_row(
                        "Process record",
                        format!("#{}", record.sequence),
                        Tone::Info,
                    ),
                    genealogy_detail_row("Wafer", record.wafer.to_string(), Tone::Neutral),
                    genealogy_detail_row(
                        "Step",
                        format!("{} {}", record.step_id, record.step_name),
                        Tone::Info,
                    ),
                    genealogy_detail_row("Layer", process_layer_label(record), Tone::Neutral),
                    genealogy_detail_row(
                        "Recipe / tool",
                        format!("{} / {}", record.recipe_id, record.tool_id),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row("Tool run", record.tool_run_id.to_string(), Tone::Neutral),
                    genealogy_detail_row("Completed", record.completed_at.clone(), Tone::Neutral),
                ]
            }
            TraceSelection::MaterialUse(sequence) => {
                let Some(record) = self.material_use(*sequence) else {
                    return vec![genealogy_operad_row(
                        "Selected material use is unavailable".to_string(),
                        format!("#{sequence}"),
                        Tone::Warning,
                        None,
                        false,
                    )];
                };
                let material = self.material_lot(record);
                vec![
                    genealogy_detail_row(
                        "Material use",
                        format!("#{}", record.sequence),
                        Tone::Info,
                    ),
                    genealogy_detail_row("Wafer", record.wafer.to_string(), Tone::Neutral),
                    genealogy_detail_row(
                        "Material lot",
                        record.material_lot_id.to_string(),
                        Tone::Success,
                    ),
                    genealogy_detail_row(
                        "Material",
                        material
                            .map(|material| material.name.clone())
                            .unwrap_or_else(|| "-".to_string()),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "Supplier / certificate",
                        material
                            .map(|material| {
                                format!("{} / {}", material.supplier, material.certificate_id)
                            })
                            .unwrap_or_else(|| "-".to_string()),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "Step / tool run",
                        format!("{} / {}", record.step_id, record.tool_run_id),
                        Tone::Neutral,
                    ),
                    genealogy_detail_row(
                        "Quantity",
                        format!("{} {}", compact_number(record.quantity), record.unit),
                        Tone::Neutral,
                    ),
                ]
            }
            TraceSelection::Event(sequence) => {
                let Some(event) = self.event(*sequence) else {
                    return vec![genealogy_operad_row(
                        "Selected audit event is unavailable".to_string(),
                        format!("#{sequence}"),
                        Tone::Warning,
                        None,
                        false,
                    )];
                };
                vec![
                    genealogy_detail_row("Audit event", format!("#{}", event.sequence), Tone::Info),
                    genealogy_detail_row("Operation", event_kind_name(&event.kind), Tone::Neutral),
                    genealogy_detail_row("Scope", event_scope(&event.kind), Tone::Neutral),
                    genealogy_detail_row("Reason", event_reason(&event.kind), Tone::Warning),
                    genealogy_detail_row("Audit text", event_label(&event.kind), Tone::Neutral),
                ]
            }
        }
    }

    fn operad_impact_rows(&self) -> Vec<GenealogyOperadRow> {
        let mut rows = ImpactMode::ALL
            .into_iter()
            .map(|mode| {
                genealogy_operad_row(
                    mode.label().to_string(),
                    "Select excursion context".to_string(),
                    if self.impact_mode == mode {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                    Some(format!("{OPERAD_ACTION_IMPACT_MODE}{}:impact", mode.slug())),
                    self.impact_mode == mode,
                )
            })
            .collect::<Vec<_>>();
        let Some(query) = self.impact_query() else {
            rows.push(genealogy_operad_row(
                "No matching excursion context".to_string(),
                "Selected wafer has no matching process or material history for this impact mode"
                    .to_string(),
                Tone::Neutral,
                None,
                false,
            ));
            return rows;
        };
        let impact = self.genealogy.impact_for(query.clone());
        rows.push(genealogy_operad_row(
            query.label(),
            format!(
                "{} direct, {} impacted, {} evidence row(s)",
                impact.direct_wafers.len(),
                impact.impacted_wafers.len(),
                impact.matching_process_records.len() + impact.matching_material_uses.len()
            ),
            Tone::Warning,
            None,
            false,
        ));
        let search = self.normalized_search();
        let impact_rows = impact
            .impacted_wafers
            .iter()
            .filter(|impact| impact_row_matches(impact, &search))
            .collect::<Vec<_>>();
        let total = impact_rows.len();
        for (index, impact) in impact_rows.into_iter().take(18).enumerate() {
            rows.push(genealogy_operad_row(
                format!(
                    "{} · {}",
                    impact.wafer,
                    relationship_label(&impact.relationship)
                ),
                format!(
                    "Latest step {}",
                    impact
                        .latest_step_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "-".to_string())
                ),
                match impact.relationship {
                    ImpactRelationship::Direct => Tone::Info,
                    ImpactRelationship::Descendant { .. } => Tone::Warning,
                },
                Some(trace_action_name(
                    &TraceSelection::Wafer(impact.wafer.clone()),
                    &format!("impact.{index}"),
                )),
                self.selected_wafer_ref().as_ref() == Some(&impact.wafer),
            ));
        }
        if total > 18 {
            rows.push(genealogy_operad_row(
                format!("Showing first 18 of {total} impacted wafers"),
                "Use search to focus the impact list".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_process_rows(&self) -> Vec<GenealogyOperadRow> {
        let Some(wafer) = self.selected_wafer_ref() else {
            return Vec::new();
        };
        let search = self.normalized_search();
        self.genealogy
            .inherited_process_history_for_wafer(&wafer)
            .into_iter()
            .rev()
            .filter(|record| process_record_matches(record, &search))
            .take(24)
            .map(|record| {
                genealogy_operad_row(
                    format!(
                        "#{} · {} {}",
                        record.sequence, record.step_id, record.step_name
                    ),
                    format!(
                        "{} · {} / {} · {}",
                        if record.wafer == wafer {
                            "direct"
                        } else {
                            "parent"
                        },
                        record.tool_id,
                        record.tool_run_id,
                        record.completed_at
                    ),
                    if record.wafer == wafer {
                        Tone::Info
                    } else {
                        Tone::Neutral
                    },
                    Some(trace_action_name(
                        &TraceSelection::Process(record.sequence),
                        "process",
                    )),
                    self.selected_detail == Some(TraceSelection::Process(record.sequence)),
                )
            })
            .collect()
    }

    fn operad_material_rows(&self) -> Vec<GenealogyOperadRow> {
        let Some(wafer) = self.selected_wafer_ref() else {
            return Vec::new();
        };
        let search = self.normalized_search();
        self.genealogy
            .material_ancestry_for_wafer(&wafer)
            .into_iter()
            .rev()
            .filter(|record| material_use_matches(&self.genealogy, record, &search))
            .take(18)
            .map(|record| {
                genealogy_operad_row(
                    format!(
                        "#{} · {}",
                        record.sequence,
                        material_label(&self.genealogy, record)
                    ),
                    format!(
                        "{} · step {} · run {} · {} {}",
                        if record.wafer == wafer {
                            "direct"
                        } else {
                            "parent"
                        },
                        record.step_id,
                        record.tool_run_id,
                        compact_number(record.quantity),
                        record.unit
                    ),
                    if record.wafer == wafer {
                        Tone::Success
                    } else {
                        Tone::Neutral
                    },
                    Some(trace_action_name(
                        &TraceSelection::MaterialUse(record.sequence),
                        "material",
                    )),
                    self.selected_detail == Some(TraceSelection::MaterialUse(record.sequence)),
                )
            })
            .collect()
    }

    fn operad_event_rows(&self) -> Vec<GenealogyOperadRow> {
        let search = self.normalized_search();
        self.genealogy
            .events
            .iter()
            .rev()
            .filter(|event| event_matches(event, &search))
            .take(24)
            .map(|event| {
                genealogy_operad_row(
                    format!("#{} · {}", event.sequence, event_kind_name(&event.kind)),
                    format!(
                        "{} · {}",
                        event_scope(&event.kind),
                        event_reason(&event.kind)
                    ),
                    Tone::Neutral,
                    Some(trace_action_name(
                        &TraceSelection::Event(event.sequence),
                        "event",
                    )),
                    self.selected_detail == Some(TraceSelection::Event(event.sequence)),
                )
            })
            .collect()
    }

    fn handle_operad_action(&mut self, node_name: &str) -> bool {
        if node_name == OPERAD_ACTION_TOGGLE_RELATED {
            self.related_only = !self.related_only;
            return true;
        }
        if node_name == OPERAD_ACTION_CLEAR_SEARCH {
            self.search_query.clear();
            return true;
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_IMPACT_MODE) {
            let slug = value.split(':').next().unwrap_or(value);
            if let Some(mode) = ImpactMode::from_slug(slug) {
                self.impact_mode = mode;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_LOT) {
            let lot_id = LotId::new(value.split(':').next().unwrap_or(value));
            if self.genealogy.lots.contains_key(&lot_id) {
                self.select_lot(lot_id);
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_WAFER) {
            let mut parts = value.split(':');
            let Some(lot_id) = parts.next() else {
                return false;
            };
            let Some(wafer_id) = parts.next() else {
                return false;
            };
            let wafer = WaferRef::new(lot_id.to_string(), wafer_id.to_string());
            if self.genealogy.wafer(&wafer).is_some() {
                self.select_wafer(wafer);
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SELECT_TRACE)
            && let Some(selection) = trace_selection_from_action(value)
            && self.selection_exists(&selection)
        {
            self.select_trace(selection);
            return true;
        }
        false
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

    fn slug(self) -> &'static str {
        match self {
            Self::LatestToolRun => "latest-tool-run",
            Self::LatestStep => "latest-step",
            Self::LatestMaterial => "latest-material",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "latest-tool-run" => Some(Self::LatestToolRun),
            "latest-step" => Some(Self::LatestStep),
            "latest-material" => Some(Self::LatestMaterial),
            _ => None,
        }
    }
}

fn genealogy_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += genealogy_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += genealogy_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn genealogy_operad_metric_columns(width: f32) -> usize {
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

fn genealogy_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = genealogy_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn genealogy_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_genealogy_operad_header(
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
            "genealogy.header",
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
    add_genealogy_operad_text(
        document,
        header,
        "genealogy.header.eyebrow",
        eyebrow,
        genealogy_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_genealogy_operad_text(
        document,
        header,
        "genealogy.header.title",
        title,
        genealogy_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_genealogy_operad_text(
        document,
        header,
        "genealogy.header.detail",
        detail,
        genealogy_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_genealogy_operad_text(
        document,
        header,
        "genealogy.header.meta",
        meta,
        genealogy_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_genealogy_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[GenealogyMetricTile],
) {
    let columns = genealogy_operad_metric_columns(width);
    let grid_height = genealogy_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "genealogy.metrics",
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
                format!("genealogy.metrics.row.{row_index}"),
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
            add_genealogy_operad_metric_tile(
                document,
                row,
                &format!("genealogy.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_genealogy_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &GenealogyMetricTile,
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
                genealogy_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_genealogy_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        genealogy_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_genealogy_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        genealogy_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_genealogy_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        genealogy_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            genealogy_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_genealogy_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[GenealogyOperadRow],
) {
    let height = genealogy_operad_section_height(rows.len());
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
    add_genealogy_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        genealogy_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_genealogy_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_genealogy_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_genealogy_operad_empty_row(
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
    add_genealogy_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        genealogy_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_genealogy_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &GenealogyOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        genealogy_operad_tone_color(Tone::Info)
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
        node = node.with_input(InputBehavior::BUTTON);
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
            genealogy_operad_tone_color(row.tone),
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
    add_genealogy_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        genealogy_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_genealogy_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        genealogy_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_genealogy_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("genealogy.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_genealogy_operad_text(
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

fn genealogy_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn genealogy_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn genealogy_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> GenealogyOperadRow {
    GenealogyOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
    }
}

fn genealogy_detail_row(
    label: impl Into<String>,
    value: impl Into<String>,
    tone: Tone,
) -> GenealogyOperadRow {
    genealogy_operad_row(label, value, tone, None, false)
}

fn trace_action_name(selection: &TraceSelection, suffix: &str) -> String {
    match selection {
        TraceSelection::Lot(lot_id) => {
            format!(
                "{OPERAD_ACTION_SELECT_TRACE}lot:{}:{suffix}",
                lot_id.as_str()
            )
        }
        TraceSelection::Wafer(wafer) => format!(
            "{OPERAD_ACTION_SELECT_TRACE}wafer:{}:{}:{suffix}",
            wafer.lot_id.as_str(),
            wafer.wafer_id.as_str()
        ),
        TraceSelection::Process(sequence) => {
            format!("{OPERAD_ACTION_SELECT_TRACE}process:{sequence}:{suffix}")
        }
        TraceSelection::MaterialUse(sequence) => {
            format!("{OPERAD_ACTION_SELECT_TRACE}material:{sequence}:{suffix}")
        }
        TraceSelection::Event(sequence) => {
            format!("{OPERAD_ACTION_SELECT_TRACE}event:{sequence}:{suffix}")
        }
    }
}

fn trace_selection_from_action(value: &str) -> Option<TraceSelection> {
    let mut parts = value.split(':');
    match parts.next()? {
        "lot" => Some(TraceSelection::Lot(LotId::new(parts.next()?))),
        "wafer" => Some(TraceSelection::Wafer(WaferRef::new(
            parts.next()?.to_string(),
            parts.next()?.to_string(),
        ))),
        "process" => Some(TraceSelection::Process(parts.next()?.parse().ok()?)),
        "material" => Some(TraceSelection::MaterialUse(parts.next()?.parse().ok()?)),
        "event" => Some(TraceSelection::Event(parts.next()?.parse().ok()?)),
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genealogy_operad_view_audits_common_widths() {
        let mut panel = GenealogyPanel::from_genealogy(LotGenealogy::sample());
        panel.ensure_selection();
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
    fn genealogy_operad_actions_update_panel_state() {
        let mut panel = GenealogyPanel::from_genealogy(LotGenealogy::sample());
        let target_lot = panel.genealogy.lot_ids().last().unwrap().clone();
        let target_wafer = panel
            .genealogy
            .wafer_refs_for_lot(&target_lot)
            .last()
            .unwrap()
            .clone();

        assert!(panel.handle_operad_action(&format!(
            "{OPERAD_ACTION_SELECT_WAFER}{}:{}:test",
            target_wafer.lot_id.as_str(),
            target_wafer.wafer_id.as_str()
        )));
        assert_eq!(panel.selected_lot.as_ref(), Some(&target_wafer.lot_id));
        assert_eq!(panel.selected_wafer.as_ref(), Some(&target_wafer.wafer_id));
        assert_eq!(
            panel.selected_detail.as_ref(),
            Some(&TraceSelection::Wafer(target_wafer.clone()))
        );

        let process_sequence = panel.genealogy.process_history.first().unwrap().sequence;
        assert!(panel.handle_operad_action(&format!(
            "{OPERAD_ACTION_SELECT_TRACE}process:{process_sequence}:test"
        )));
        assert_eq!(
            panel.selected_detail,
            Some(TraceSelection::Process(process_sequence))
        );

        assert!(panel.handle_operad_action(&format!(
            "{OPERAD_ACTION_IMPACT_MODE}{}:test",
            ImpactMode::LatestMaterial.slug()
        )));
        assert_eq!(panel.impact_mode, ImpactMode::LatestMaterial);

        assert!(!panel.related_only);
        assert!(panel.handle_operad_action(OPERAD_ACTION_TOGGLE_RELATED));
        assert!(panel.related_only);
    }
}
