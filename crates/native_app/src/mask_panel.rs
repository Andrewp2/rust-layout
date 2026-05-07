use eframe::egui::{self, Color32};
use geometry_core::Coord;
use layout_model::{
    Document,
    mask::{MaskCheckReport, MaskIssueSeverity, ReticlePrep},
    mes::{FabMesData, LotId},
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct MaskPrepPanel {
    prep: ReticlePrep,
    source_lot: Option<LotId>,
}

impl MaskPrepPanel {
    pub(crate) fn new(document: &Document, mes: &FabMesData, selected_lot: Option<&LotId>) -> Self {
        Self {
            prep: reticle_prep_for_context(document, mes, selected_lot),
            source_lot: selected_lot.cloned(),
        }
    }

    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_mes_lot: &mut Option<LotId>,
        status: &mut String,
    ) {
        self.sync_lot_context(document, mes, selected_mes_lot.as_ref());
        let report = self.prep.validate_document(document);

        egui::ScrollArea::vertical()
            .id_salt("mask_prep_dashboard")
            .show(ui, |ui| {
                let detail = format!(
                    "{} / {}",
                    self.prep.mask_design_id, self.prep.layout_revision
                );
                ui_chrome::module_header(
                    ui,
                    "Design handoff",
                    "Mask / Reticle Prep",
                    &detail,
                    |ui| {
                        if ui.button("Rebuild from layout").clicked() {
                            self.rebuild(document, mes, selected_mes_lot.as_ref());
                            let report = self.prep.validate_document(document);
                            *status = format!(
                                "mask prep rebuilt: {} error(s), {} warning(s)",
                                report.error_count(),
                                report.warning_count()
                            );
                        }
                        self.lot_picker(ui, mes, selected_mes_lot, status);
                    },
                );

                metric_row(ui, &report);

                ui.separator();
                ui.columns(2, |columns| {
                    self.reticle_summary_ui(&mut columns[0]);
                    self.exposure_blocks_ui(&mut columns[1], &report);
                });

                ui.separator();
                ui_chrome::section_label(ui, "Reticle Fields");
                self.fields_ui(ui);

                ui.separator();
                issues_ui(ui, &report);
            });
    }

    pub(crate) fn context_ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_lot: Option<&LotId>,
        status: &mut String,
    ) {
        self.sync_lot_context(document, mes, selected_lot);
        let report = self.prep.validate_document(document);

        ui_chrome::section_label(ui, "Reticle Prep");
        ui.label(format!("Mask: {}", self.prep.mask_design_id));
        if let Some(route_id) = &self.prep.route_id {
            ui.label(format!("Route: {route_id}"));
        }
        ui.label(format!("Reticle: {}", self.prep.reticle.id));
        ui.label(format!(
            "Printable: {} x {}",
            format_coord(self.prep.reticle.printable_bounds().width()),
            format_coord(self.prep.reticle.printable_bounds().height())
        ));

        ui.separator();
        report_summary(ui, &report);

        ui.separator();
        if ui.button("Sync reticle prep").clicked() {
            self.rebuild(document, mes, selected_lot);
            *status = "mask prep synced to current Fabricad layout".to_string();
        }

        ui.separator();
        ui_chrome::section_label(ui, "Open Issues");
        for issue in report.issues.iter().take(8) {
            issue_row(ui, issue.severity, &issue.message);
        }
        if report.issues.is_empty() {
            ui_chrome::empty_state(ui, "No mask prep issues");
        }
    }

    pub(crate) fn layer_stack_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Mask Layers");
        egui::ScrollArea::vertical()
            .id_salt("mask_layer_stack")
            .show(ui, |ui| {
                egui::Grid::new("mask_layer_stack_grid")
                    .striped(true)
                    .min_col_width(48.0)
                    .show(ui, |ui| {
                        ui.strong("Layer");
                        ui.strong("Tone");
                        ui.strong("Rule");
                        ui.end_row();
                        for layer in &self.prep.layer_stack {
                            ui.label(format!("{} {}", layer.layer.0, layer.name));
                            ui.label(layer.tone.label());
                            ui.label(format!(
                                "{} / {}",
                                format_coord(layer.min_feature),
                                format_coord(layer.min_spacing)
                            ));
                            ui.end_row();
                        }
                    });
            });
    }

    fn lot_picker(
        &mut self,
        ui: &mut egui::Ui,
        mes: &FabMesData,
        selected_mes_lot: &mut Option<LotId>,
        status: &mut String,
    ) {
        let mut selected = selected_mes_lot.clone();
        let selected_text = selected
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unlinked layout".to_string());

        egui::ComboBox::from_id_salt("mask_prep_lot_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, "unlinked layout");
                for lot_id in mes.lots.keys() {
                    ui.selectable_value(&mut selected, Some(lot_id.clone()), lot_id.to_string());
                }
            });

        if selected != *selected_mes_lot {
            *selected_mes_lot = selected;
            *status = "mask prep lot context selected".to_string();
        }
    }

    fn sync_lot_context(
        &mut self,
        document: &Document,
        mes: &FabMesData,
        selected_lot: Option<&LotId>,
    ) {
        if self.source_lot.as_ref() != selected_lot {
            self.rebuild(document, mes, selected_lot);
        }
    }

    fn rebuild(&mut self, document: &Document, mes: &FabMesData, selected_lot: Option<&LotId>) {
        self.prep = reticle_prep_for_context(document, mes, selected_lot);
        self.source_lot = selected_lot.cloned();
    }

    fn reticle_summary_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Reticle");
        ui.label(format!("ID: {}", self.prep.reticle.id));
        ui.label(format!("Name: {}", self.prep.reticle.name));
        ui.label(format!(
            "Size: {} x {}",
            format_coord(self.prep.reticle.size.width),
            format_coord(self.prep.reticle.size.height)
        ));
        ui.label(format!(
            "Max field: {} x {}",
            format_coord(self.prep.reticle.size.max_field_width),
            format_coord(self.prep.reticle.size.max_field_height)
        ));
        ui.label(format!(
            "Edge clearance: {}",
            format_coord(self.prep.reticle.edge_clearance)
        ));
    }

    fn fields_ui(&self, ui: &mut egui::Ui) {
        egui::Grid::new("reticle_fields_grid")
            .striped(true)
            .min_col_width(72.0)
            .show(ui, |ui| {
                ui.strong("Field");
                ui.strong("Source");
                ui.strong("Bounds");
                ui.strong("Stepping");
                ui.end_row();
                for field in &self.prep.fields {
                    ui.label(format!("{} {}", field.id, field.name));
                    ui.label(format!("cell {}", field.source_cell.0));
                    ui.label(format!(
                        "{} x {}",
                        format_coord(field.layout_bounds.width()),
                        format_coord(field.layout_bounds.height())
                    ));
                    ui.label(format!(
                        "{} x {}",
                        field.stepping.columns, field.stepping.rows
                    ));
                    ui.end_row();
                }
            });
    }

    fn exposure_blocks_ui(&self, ui: &mut egui::Ui, report: &MaskCheckReport) {
        ui_chrome::section_label(ui, "Exposure Blocks");
        egui::Grid::new("exposure_blocks_grid")
            .striped(true)
            .min_col_width(70.0)
            .show(ui, |ui| {
                ui.strong("Block");
                ui.strong("Dose");
                ui.strong("Focus");
                ui.strong("Layers");
                ui.end_row();
                for block in &self.prep.exposure_blocks {
                    let has_error = report.issues.iter().any(|issue| {
                        issue.block_id.as_ref() == Some(&block.id)
                            && issue.severity == MaskIssueSeverity::Error
                    });
                    ui.colored_label(
                        if has_error {
                            Color32::LIGHT_RED
                        } else {
                            ui.visuals().text_color()
                        },
                        format!("{} {}", block.id, block.name),
                    );
                    ui.label(format!("{:.1} mJ/cm2", block.dose_mj_cm2));
                    ui.label(format!("{:+.2} um", block.focus_offset_um));
                    ui.label(
                        block
                            .layer_ids
                            .iter()
                            .map(|layer| layer.0.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ui.end_row();
                }
            });
    }
}

fn reticle_prep_for_context(
    document: &Document,
    mes: &FabMesData,
    selected_lot: Option<&LotId>,
) -> ReticlePrep {
    selected_lot
        .and_then(|lot_id| mes.lots.get(lot_id))
        .and_then(|lot| mes.routes.get(&lot.route_id))
        .map(|route| ReticlePrep::from_document_and_route(document, route))
        .unwrap_or_else(|| ReticlePrep::from_document(document))
}

fn metric_row(ui: &mut egui::Ui, report: &MaskCheckReport) {
    ui.horizontal_wrapped(|ui| {
        metric(ui, "Layers", report.layer_count.to_string());
        metric(ui, "Fields", report.field_count.to_string());
        metric(ui, "Blocks", report.exposure_block_count.to_string());
        metric(ui, "Shapes", report.printable_shape_count.to_string());
        metric(ui, "Errors", report.error_count().to_string());
        metric(ui, "Warnings", report.warning_count().to_string());
    });
}

fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    ui_chrome::metric_tile(ui, label, value, "");
}

fn report_summary(ui: &mut egui::Ui, report: &MaskCheckReport) {
    if report.is_clean() {
        ui_chrome::status_pill(ui, "checks clean", Tone::Success);
    } else {
        let tone = if report.error_count() > 0 {
            Tone::Danger
        } else {
            Tone::Warning
        };
        ui_chrome::status_pill(
            ui,
            &format!(
                "{} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            ),
            tone,
        );
    }
}

fn issues_ui(ui: &mut egui::Ui, report: &MaskCheckReport) {
    ui_chrome::section_label(ui, "Mask Checks");
    if report.issues.is_empty() {
        ui_chrome::empty_state(ui, "No mask check issues");
        return;
    }
    egui::Grid::new("mask_checks_grid")
        .striped(true)
        .min_col_width(72.0)
        .show(ui, |ui| {
            ui.strong("Severity");
            ui.strong("Code");
            ui.strong("Message");
            ui.end_row();
            for issue in &report.issues {
                ui.colored_label(issue_color(issue.severity), issue.severity.label());
                ui.label(&issue.code);
                ui.label(&issue.message);
                ui.end_row();
            }
        });
}

fn issue_row(ui: &mut egui::Ui, severity: MaskIssueSeverity, message: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(issue_color(severity), severity.label());
        ui.label(message);
    });
}

fn issue_color(severity: MaskIssueSeverity) -> Color32 {
    match severity {
        MaskIssueSeverity::Error => Color32::LIGHT_RED,
        MaskIssueSeverity::Warning => Color32::YELLOW,
    }
}

fn format_coord(value: Coord) -> String {
    format!("{value} dbu")
}
