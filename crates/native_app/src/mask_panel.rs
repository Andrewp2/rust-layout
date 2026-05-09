use eframe::egui::{self, Color32};
use geometry_core::Coord;
use layout_model::{
    Document,
    mask::{MaskCheckReport, MaskIssueSeverity, ReticlePrep},
    mes::{FabMesData, LotId},
};

use crate::ui_chrome::{self, Tone};

const MAX_MASK_ISSUE_ROWS: usize = 80;

pub(crate) struct MaskPrepPanel {
    prep: ReticlePrep,
    source_lot: Option<LotId>,
    issue_filter: String,
    issue_page: usize,
}

impl MaskPrepPanel {
    pub(crate) fn new(document: &Document, mes: &FabMesData, selected_lot: Option<&LotId>) -> Self {
        Self {
            prep: reticle_prep_for_context(document, mes, selected_lot),
            source_lot: selected_lot.cloned(),
            issue_filter: String::new(),
            issue_page: 0,
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
                let detail = self.prep.layout_revision.clone();
                let title = self.prep.mask_design_id.clone();
                ui_chrome::module_header(ui, "Design handoff", &title, &detail, |ui| {
                    self.lot_picker(ui, mes, selected_mes_lot, status);
                    if ui.button("Rebuild from layout").clicked() {
                        self.rebuild(document, mes, selected_mes_lot.as_ref());
                        let report = self.prep.validate_document(document);
                        *status = format!(
                            "mask prep rebuilt: {} error(s), {} warning(s)",
                            report.error_count(),
                            report.warning_count()
                        );
                    }
                });

                metric_row(ui, &report);

                ui.separator();
                if ui.available_width() < 700.0 {
                    self.reticle_summary_ui(ui);
                    ui.separator();
                    self.exposure_blocks_ui(ui, &report);
                } else {
                    ui.columns(2, |columns| {
                        self.reticle_summary_ui(&mut columns[0]);
                        self.exposure_blocks_ui(&mut columns[1], &report);
                    });
                }

                ui.separator();
                ui_chrome::section_label(ui, "Reticle Fields");
                self.fields_ui(ui);

                ui.separator();
                self.issues_ui(ui, &report);
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

        ui_chrome::section_label(ui, "Mask");
        ui.label(format!("Mask design ID: {}", self.prep.mask_design_id));
        if let Some(route_id) = &self.prep.route_id {
            ui.label(format!("Route: {route_id}"));
        }
        ui.label(format!("Reticle ID: {}", self.prep.reticle.id));
        ui.label(format!(
            "Printable: {} x {}",
            format_coord(self.prep.reticle.printable_bounds().width()),
            format_coord(self.prep.reticle.printable_bounds().height())
        ));

        ui.separator();
        report_summary(ui, &report);

        ui.separator();
        if ui.button("Rebuild from layout").clicked() {
            self.rebuild(document, mes, selected_lot);
            *status = "mask prep rebuilt from current layout".to_string();
        }

        ui.separator();
        let total_issues = report.total_issue_count();
        let shown = report.issues.len().min(8);
        ui_chrome::section_label(
            ui,
            &format!("Open Issues (showing {shown} of {total_issues})"),
        );
        for issue in report.issues.iter().take(shown) {
            issue_row(ui, issue.severity, &issue.message);
        }
        if report.omitted_issue_count() > 0 {
            ui_chrome::muted(
                ui,
                format!(
                    "{} additional issue rows summarized",
                    report.omitted_issue_count()
                ),
            );
        }
        if report.is_clean() {
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
                        ui.strong("Layer ID");
                        ui.strong("Name");
                        ui.strong("Tone");
                        ui.strong("Rule (feature / spacing)");
                        ui.end_row();
                        for layer in &self.prep.layer_stack {
                            ui.label(layer.layer.0.to_string());
                            ui.label(&layer.name);
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
        egui::Grid::new("reticle_summary_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                label_value(ui, "Reticle ID", self.prep.reticle.id.to_string());
                label_value(ui, "Name", self.prep.reticle.name.clone());
                label_value(
                    ui,
                    "Size",
                    format!(
                        "{} x {}",
                        format_coord(self.prep.reticle.size.width),
                        format_coord(self.prep.reticle.size.height)
                    ),
                );
                label_value(
                    ui,
                    "Max field",
                    format!(
                        "{} x {}",
                        format_coord(self.prep.reticle.size.max_field_width),
                        format_coord(self.prep.reticle.size.max_field_height)
                    ),
                );
                label_value(
                    ui,
                    "Edge clearance",
                    format_coord(self.prep.reticle.edge_clearance),
                );
            });
    }

    fn fields_ui(&self, ui: &mut egui::Ui) {
        if ui.available_width() < 520.0 {
            for field in &self.prep.fields {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.strong(field.id.to_string());
                    ui.label(&field.name);
                    ui.small(format!("Source: cell {}", field.source_cell.0));
                    ui.small(format!(
                        "Bounds: {} x {}",
                        format_coord(field.layout_bounds.width()),
                        format_coord(field.layout_bounds.height())
                    ));
                    ui.small(format!(
                        "Stepping: {} x {}",
                        field.stepping.columns, field.stepping.rows
                    ));
                });
            }
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("reticle_fields_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("reticle_fields_grid")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Field ID");
                        ui.strong("Name");
                        ui.strong("Source");
                        ui.strong("Bounds");
                        ui.strong("Stepping");
                        ui.end_row();
                        for field in &self.prep.fields {
                            ui.label(field.id.to_string());
                            ui.label(&field.name);
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
            });
    }

    fn exposure_blocks_ui(&self, ui: &mut egui::Ui, report: &MaskCheckReport) {
        ui_chrome::section_label(ui, "Exposure Blocks");
        if ui.available_width() < 520.0 {
            for block in &self.prep.exposure_blocks {
                let has_error = report.issues.iter().any(|issue| {
                    issue.block_id.as_ref() == Some(&block.id)
                        && issue.severity == MaskIssueSeverity::Error
                });
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.colored_label(
                        if has_error {
                            Color32::LIGHT_RED
                        } else {
                            ui.visuals().text_color()
                        },
                        block.id.to_string(),
                    );
                    ui.label(&block.name);
                    ui.small(format!(
                        "Dose {:.1} mJ/cm2 / focus {:+.2} um",
                        block.dose_mj_cm2, block.focus_offset_um
                    ));
                    ui.small(format!(
                        "Layers {}",
                        block
                            .layer_ids
                            .iter()
                            .map(|layer| layer.0.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                });
            }
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("exposure_blocks_horizontal")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("exposure_blocks_grid")
                    .striped(true)
                    .min_col_width(70.0)
                    .show(ui, |ui| {
                        ui.strong("Block ID");
                        ui.strong("Name");
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
                                block.id.to_string(),
                            );
                            ui.label(&block.name);
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
            });
    }

    fn issues_ui(&mut self, ui: &mut egui::Ui, report: &MaskCheckReport) {
        let total_issues = report.total_issue_count();
        ui_chrome::section_label(ui, "Mask Checks");
        if report.is_clean() {
            ui_chrome::empty_state(ui, "No mask check issues");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Filter");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.issue_filter)
                        .desired_width(180.0)
                        .hint_text("code or message"),
                )
                .changed()
            {
                self.issue_page = 0;
            }
        });

        let filter = self.issue_filter.trim().to_ascii_lowercase();
        let filtered_indices: Vec<_> = report
            .issues
            .iter()
            .enumerate()
            .filter_map(|(index, issue)| {
                if filter.is_empty()
                    || issue.code.to_ascii_lowercase().contains(&filter)
                    || issue.message.to_ascii_lowercase().contains(&filter)
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();
        if filtered_indices.is_empty() {
            ui_chrome::empty_state(ui, "No retained mask issues match the filter");
            return;
        }

        let page_count = filtered_indices.len().div_ceil(MAX_MASK_ISSUE_ROWS).max(1);
        self.issue_page = self.issue_page.min(page_count - 1);
        let start = self.issue_page * MAX_MASK_ISSUE_ROWS;
        let end = (start + MAX_MASK_ISSUE_ROWS).min(filtered_indices.len());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::muted(
                ui,
                format!(
                    "showing {}-{} of {} retained, {} total",
                    start + 1,
                    end,
                    filtered_indices.len(),
                    total_issues
                ),
            );
            if ui
                .add_enabled(self.issue_page > 0, egui::Button::new("Prev"))
                .clicked()
            {
                self.issue_page -= 1;
            }
            ui.label(format!("Page {} / {}", self.issue_page + 1, page_count));
            if ui
                .add_enabled(self.issue_page + 1 < page_count, egui::Button::new("Next"))
                .clicked()
            {
                self.issue_page += 1;
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("mask_checks_scroll")
            .max_height(280.0)
            .show(ui, |ui| {
                egui::Grid::new("mask_checks_grid")
                    .striped(true)
                    .min_col_width(72.0)
                    .show(ui, |ui| {
                        ui.strong("Severity");
                        ui.strong("Code");
                        ui.strong("Message");
                        ui.end_row();
                        for index in &filtered_indices[start..end] {
                            let issue = &report.issues[*index];
                            ui.colored_label(issue_color(issue.severity), issue.severity.label());
                            ui.label(&issue.code);
                            ui.add(egui::Label::new(&issue.message).wrap());
                            ui.end_row();
                        }
                    });
            });
        if report.omitted_issue_count() > 0 {
            ui_chrome::muted(
                ui,
                format!(
                    "{} additional issue rows summarized",
                    report.omitted_issue_count()
                ),
            );
        }
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

fn label_value(ui: &mut egui::Ui, label: &str, value: impl ToString) {
    ui.strong(label);
    ui.label(value.to_string());
    ui.end_row();
}

fn metric_row(ui: &mut egui::Ui, report: &MaskCheckReport) {
    ui.horizontal_wrapped(|ui| {
        summary_badge(ui, "Layers", report.layer_count);
        summary_badge(ui, "Fields", report.field_count);
        summary_badge(ui, "Blocks", report.exposure_block_count);
        summary_badge(ui, "Shapes", report.printable_shape_count);
        summary_badge(ui, "Errors", report.error_count());
        summary_badge(ui, "Warnings", report.warning_count());
    });
}

fn summary_badge(ui: &mut egui::Ui, label: &str, value: usize) {
    ui.label(format!("{label}: {value}"));
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
