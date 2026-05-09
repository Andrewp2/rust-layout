use std::collections::BTreeMap;

use eframe::egui::{self, Color32};
use geometry_core::{Coord, Vector};
use layout_model::{
    Document,
    mask::{MaskCheckReport, MaskIssueSeverity, MaskPrepIssue, ReticlePrep},
    mes::{FabMesData, LotId},
};

use crate::ui_chrome::{self, Tone};

const MAX_MASK_ISSUE_ROWS: usize = 50;
const MAX_ISSUE_GROUP_ROWS: usize = 10;
const MAX_MASK_LAYER_ROWS: usize = 80;
const MAX_RETICLE_FIELD_ROWS: usize = 40;
const MAX_EXPOSURE_BLOCK_ROWS: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IssueSeverityFilter {
    All,
    Errors,
    Warnings,
}

impl IssueSeverityFilter {
    const ALL: [Self; 3] = [Self::All, Self::Errors, Self::Warnings];

    fn label(self) -> &'static str {
        match self {
            Self::All => "all severities",
            Self::Errors => "errors only",
            Self::Warnings => "warnings only",
        }
    }

    fn allows(self, severity: MaskIssueSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Errors => severity == MaskIssueSeverity::Error,
            Self::Warnings => severity == MaskIssueSeverity::Warning,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IssueGrouping {
    Code,
    Layer,
    Field,
    Block,
}

impl IssueGrouping {
    const ALL: [Self; 4] = [Self::Code, Self::Layer, Self::Field, Self::Block];

    fn label(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Layer => "layer",
            Self::Field => "field",
            Self::Block => "block",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct IssueCounts {
    errors: usize,
    warnings: usize,
}

impl IssueCounts {
    fn add(&mut self, severity: MaskIssueSeverity) {
        match severity {
            MaskIssueSeverity::Error => self.errors += 1,
            MaskIssueSeverity::Warning => self.warnings += 1,
        }
    }

    fn total(self) -> usize {
        self.errors + self.warnings
    }
}

pub(crate) struct MaskPrepPanel {
    prep: ReticlePrep,
    source_lot: Option<LotId>,
    issue_filter: String,
    issue_severity_filter: IssueSeverityFilter,
    issue_grouping: IssueGrouping,
    issue_page: usize,
}

impl MaskPrepPanel {
    pub(crate) fn new(document: &Document, mes: &FabMesData, selected_lot: Option<&LotId>) -> Self {
        Self {
            prep: reticle_prep_for_context(document, mes, selected_lot),
            source_lot: selected_lot.cloned(),
            issue_filter: String::new(),
            issue_severity_filter: IssueSeverityFilter::All,
            issue_grouping: IssueGrouping::Code,
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
                    if ui
                        .button("Rebuild mask prep from current layout")
                        .on_hover_text(
                            "Regenerate reticle fields, layer stack, and exposure blocks from the active layout and selected lot route.",
                        )
                        .clicked()
                    {
                        self.rebuild(document, mes, selected_mes_lot.as_ref());
                        let report = self.prep.validate_document(document);
                        *status = format!(
                            "mask prep rebuilt from current layout: {} error(s), {} warning(s)",
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
        if ui
            .button("Rebuild mask prep from current layout")
            .on_hover_text(
                "Refresh this reticle prep from the current layout and selected lot route.",
            )
            .clicked()
        {
            self.rebuild(document, mes, selected_lot);
            *status = "mask prep rebuilt from current layout and lot route".to_string();
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
        let shown = self.prep.layer_stack.len().min(MAX_MASK_LAYER_ROWS);
        ui_chrome::muted(
            ui,
            format!(
                "showing {} of {} mask layer(s)",
                shown,
                self.prep.layer_stack.len()
            ),
        );
        if self.prep.layer_stack.is_empty() {
            ui_chrome::empty_state(ui, "No mask layers");
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("mask_layer_stack")
            .max_height(260.0)
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
                        for layer in self.prep.layer_stack.iter().take(MAX_MASK_LAYER_ROWS) {
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
        truncated_note(
            ui,
            self.prep
                .layer_stack
                .len()
                .saturating_sub(MAX_MASK_LAYER_ROWS),
            "mask layer",
        );
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

        ui.label("Lot route sync");
        egui::ComboBox::from_id_salt("mask_prep_lot_picker")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, "unlinked layout");
                for lot_id in mes.lots.keys() {
                    ui.selectable_value(&mut selected, Some(lot_id.clone()), lot_id.to_string());
                }
            })
            .response
            .on_hover_text("Selecting a lot syncs reticle prep to that lot's process route.");

        if selected != *selected_mes_lot {
            *selected_mes_lot = selected;
            *status = "mask prep lot route selected; reticle prep will sync".to_string();
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
        self.issue_page = 0;
    }

    fn reticle_summary_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Reticle");
        let printable = self.prep.reticle.printable_bounds();
        egui::Grid::new("reticle_summary_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                label_value(ui, "Mask design", &self.prep.mask_design_id);
                label_value(ui, "Layout revision", &self.prep.layout_revision);
                label_value(
                    ui,
                    "Route",
                    self.prep
                        .route_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "unlinked layout".to_string()),
                );
                label_value(ui, "Reticle ID", self.prep.reticle.id.to_string());
                label_value(ui, "Name", &self.prep.reticle.name);
                label_value(
                    ui,
                    "Reticle size",
                    format!(
                        "{} x {}",
                        format_coord(self.prep.reticle.size.width),
                        format_coord(self.prep.reticle.size.height)
                    ),
                );
                label_value(
                    ui,
                    "Printable area",
                    format!(
                        "{} x {}",
                        format_coord(printable.width()),
                        format_coord(printable.height())
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
                label_value(
                    ui,
                    "Alignment clearance",
                    format_coord(self.prep.reticle.alignment_clearance),
                );
            });
    }

    fn fields_ui(&self, ui: &mut egui::Ui) {
        if self.prep.fields.is_empty() {
            ui_chrome::empty_state(ui, "No reticle fields");
            return;
        }

        let shown = self.prep.fields.len().min(MAX_RETICLE_FIELD_ROWS);
        ui_chrome::muted(
            ui,
            format!(
                "showing {} of {} field definition(s)",
                shown,
                self.prep.fields.len()
            ),
        );
        if ui.available_width() < 520.0 {
            for field in self.prep.fields.iter().take(MAX_RETICLE_FIELD_ROWS) {
                ui.group(|ui| {
                    ui.set_width(ui.available_width().clamp(220.0, 420.0));
                    ui.strong(field.id.to_string());
                    ui.label(&field.name);
                    ui.small(format!("Source: cell {}", field.source_cell.0));
                    ui.small(format!("Origin: {}", format_vector(field.reticle_origin)));
                    ui.small(format!(
                        "Bounds: {} x {}",
                        format_coord(field.layout_bounds.width()),
                        format_coord(field.layout_bounds.height())
                    ));
                    ui.small(format!(
                        "Stepping: {} x {}",
                        field.stepping.columns, field.stepping.rows
                    ));
                    ui.small(format!("Pitch: {}", format_vector(field.stepping.pitch)));
                });
            }
            truncated_note(
                ui,
                self.prep
                    .fields
                    .len()
                    .saturating_sub(MAX_RETICLE_FIELD_ROWS),
                "field definition",
            );
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
                        ui.strong("Origin");
                        ui.strong("Bounds");
                        ui.strong("Stepping");
                        ui.strong("Pitch");
                        ui.end_row();
                        for field in self.prep.fields.iter().take(MAX_RETICLE_FIELD_ROWS) {
                            ui.label(field.id.to_string());
                            ui.label(&field.name);
                            ui.label(format!("cell {}", field.source_cell.0));
                            ui.label(format_vector(field.reticle_origin));
                            ui.label(format!(
                                "{} x {}",
                                format_coord(field.layout_bounds.width()),
                                format_coord(field.layout_bounds.height())
                            ));
                            ui.label(format!(
                                "{} x {}",
                                field.stepping.columns, field.stepping.rows
                            ));
                            ui.label(format_vector(field.stepping.pitch));
                            ui.end_row();
                        }
                    });
            });
        truncated_note(
            ui,
            self.prep
                .fields
                .len()
                .saturating_sub(MAX_RETICLE_FIELD_ROWS),
            "field definition",
        );
    }

    fn exposure_blocks_ui(&self, ui: &mut egui::Ui, report: &MaskCheckReport) {
        ui_chrome::section_label(ui, "Exposure Blocks");
        if self.prep.exposure_blocks.is_empty() {
            ui_chrome::empty_state(ui, "No exposure blocks");
            return;
        }

        let shown = self.prep.exposure_blocks.len().min(MAX_EXPOSURE_BLOCK_ROWS);
        ui_chrome::muted(
            ui,
            format!(
                "showing {} of {} exposure block(s)",
                shown,
                self.prep.exposure_blocks.len()
            ),
        );
        if ui.available_width() < 520.0 {
            for block in self
                .prep
                .exposure_blocks
                .iter()
                .take(MAX_EXPOSURE_BLOCK_ROWS)
            {
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
                    ui.small(format!("Field: {}", block.field_id));
                    ui.small(format!(
                        "Dose {:.1} mJ/cm2 / focus {:+.2} um",
                        block.dose_mj_cm2, block.focus_offset_um
                    ));
                    ui.small(format!("Passes: {}", block.passes));
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
            truncated_note(
                ui,
                self.prep
                    .exposure_blocks
                    .len()
                    .saturating_sub(MAX_EXPOSURE_BLOCK_ROWS),
                "exposure block",
            );
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
                        ui.strong("Field");
                        ui.strong("Dose");
                        ui.strong("Focus");
                        ui.strong("Passes");
                        ui.strong("Layers");
                        ui.end_row();
                        for block in self
                            .prep
                            .exposure_blocks
                            .iter()
                            .take(MAX_EXPOSURE_BLOCK_ROWS)
                        {
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
                            ui.label(block.field_id.to_string());
                            ui.label(format!("{:.1} mJ/cm2", block.dose_mj_cm2));
                            ui.label(format!("{:+.2} um", block.focus_offset_um));
                            ui.label(block.passes.to_string());
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
        truncated_note(
            ui,
            self.prep
                .exposure_blocks
                .len()
                .saturating_sub(MAX_EXPOSURE_BLOCK_ROWS),
            "exposure block",
        );
    }

    fn issues_ui(&mut self, ui: &mut egui::Ui, report: &MaskCheckReport) {
        let total_issues = report.total_issue_count();
        ui_chrome::section_label(ui, "Mask Checks");
        check_summary_ui(ui, report);
        if report.is_clean() {
            ui_chrome::empty_state(ui, "No mask check issues");
            return;
        }

        let mut filters_changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Search retained checks");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.issue_filter)
                        .desired_width(ui.available_width().clamp(180.0, 360.0))
                        .hint_text("code, message, layer, field, block, shape"),
                )
                .changed()
            {
                filters_changed = true;
            }
            if ui.button("Clear").clicked() {
                self.issue_filter.clear();
                self.issue_severity_filter = IssueSeverityFilter::All;
                filters_changed = true;
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("Severity");
            egui::ComboBox::from_id_salt("mask_check_severity_filter")
                .selected_text(self.issue_severity_filter.label())
                .show_ui(ui, |ui| {
                    for option in IssueSeverityFilter::ALL {
                        filters_changed |= ui
                            .selectable_value(
                                &mut self.issue_severity_filter,
                                option,
                                option.label(),
                            )
                            .changed();
                    }
                });

            ui.label("Group by");
            egui::ComboBox::from_id_salt("mask_check_grouping")
                .selected_text(self.issue_grouping.label())
                .show_ui(ui, |ui| {
                    for option in IssueGrouping::ALL {
                        ui.selectable_value(&mut self.issue_grouping, option, option.label());
                    }
                });
        });
        if filters_changed {
            self.issue_page = 0;
        }

        let filter = self.issue_filter.trim().to_ascii_lowercase();
        let filtered_indices: Vec<_> = report
            .issues
            .iter()
            .enumerate()
            .filter_map(|(index, issue)| {
                if self.issue_severity_filter.allows(issue.severity)
                    && issue_matches_filter(issue, &filter)
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

        issue_group_summary_ui(ui, report, &filtered_indices, self.issue_grouping);

        let page_count = filtered_indices.len().div_ceil(MAX_MASK_ISSUE_ROWS).max(1);
        self.issue_page = self.issue_page.min(page_count - 1);
        let start = self.issue_page * MAX_MASK_ISSUE_ROWS;
        let end = (start + MAX_MASK_ISSUE_ROWS).min(filtered_indices.len());
        ui.horizontal_wrapped(|ui| {
            ui_chrome::muted(
                ui,
                format!(
                    "showing retained rows {}-{} of {} match(es), {} retained, {} total",
                    start + 1,
                    end,
                    filtered_indices.len(),
                    report.issues.len(),
                    total_issues
                ),
            );
            if ui
                .add_enabled(self.issue_page > 0, egui::Button::new("Previous page"))
                .clicked()
            {
                self.issue_page -= 1;
            }
            ui.label(format!("Page {} / {}", self.issue_page + 1, page_count));
            if ui
                .add_enabled(
                    self.issue_page + 1 < page_count,
                    egui::Button::new("Next page"),
                )
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
                        ui.strong("Target");
                        ui.strong("Message");
                        ui.end_row();
                        for index in &filtered_indices[start..end] {
                            let issue = &report.issues[*index];
                            ui.colored_label(issue_color(issue.severity), issue.severity.label());
                            ui.label(&issue.code);
                            ui.label(issue_target_label(issue));
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

fn check_summary_ui(ui: &mut egui::Ui, report: &MaskCheckReport) {
    ui.horizontal_wrapped(|ui| {
        report_summary(ui, report);
        ui_chrome::status_pill(
            ui,
            &format!("{} retained", report.issues.len()),
            Tone::Neutral,
        );
        if report.omitted_issue_count() > 0 {
            ui_chrome::status_pill(
                ui,
                &format!("{} omitted", report.omitted_issue_count()),
                Tone::Warning,
            );
        }
    });
}

fn issue_group_summary_ui(
    ui: &mut egui::Ui,
    report: &MaskCheckReport,
    filtered_indices: &[usize],
    grouping: IssueGrouping,
) {
    let mut counts: BTreeMap<String, IssueCounts> = BTreeMap::new();
    for index in filtered_indices {
        let issue = &report.issues[*index];
        counts
            .entry(issue_group_key(issue, grouping))
            .or_default()
            .add(issue.severity);
    }

    let group_count = counts.len();
    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .1
            .total()
            .cmp(&left.1.total())
            .then_with(|| left.0.cmp(&right.0))
    });

    egui::CollapsingHeader::new(format!(
        "Issue summary by {} ({} group(s))",
        grouping.label(),
        group_count
    ))
    .default_open(true)
    .show(ui, |ui| {
        egui::Grid::new("mask_check_group_summary")
            .striped(true)
            .min_col_width(76.0)
            .show(ui, |ui| {
                ui.strong(grouping.label());
                ui.strong("Total");
                ui.strong("Errors");
                ui.strong("Warnings");
                ui.end_row();
                for (group, counts) in rows.iter().take(MAX_ISSUE_GROUP_ROWS) {
                    ui.label(group);
                    ui.label(counts.total().to_string());
                    ui.colored_label(
                        issue_color(MaskIssueSeverity::Error),
                        counts.errors.to_string(),
                    );
                    ui.colored_label(
                        issue_color(MaskIssueSeverity::Warning),
                        counts.warnings.to_string(),
                    );
                    ui.end_row();
                }
            });
        truncated_note(
            ui,
            group_count.saturating_sub(MAX_ISSUE_GROUP_ROWS),
            "issue group",
        );
        if report.omitted_issue_count() > 0 {
            ui_chrome::muted(
                ui,
                "Summary covers retained checks only; omitted checks are counted above.",
            );
        }
    });
}

fn issue_group_key(issue: &MaskPrepIssue, grouping: IssueGrouping) -> String {
    match grouping {
        IssueGrouping::Code => issue.code.clone(),
        IssueGrouping::Layer => issue
            .layer
            .map(|layer| format!("layer {}", layer.0))
            .unwrap_or_else(|| "no layer".to_string()),
        IssueGrouping::Field => issue
            .field_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no field".to_string()),
        IssueGrouping::Block => issue
            .block_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no block".to_string()),
    }
}

fn issue_matches_filter(issue: &MaskPrepIssue, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }

    issue.code.to_ascii_lowercase().contains(filter)
        || issue.message.to_ascii_lowercase().contains(filter)
        || issue_target_label(issue)
            .to_ascii_lowercase()
            .contains(filter)
}

fn issue_target_label(issue: &MaskPrepIssue) -> String {
    let mut parts = Vec::new();
    if let Some(layer) = issue.layer {
        parts.push(format!("layer {}", layer.0));
    }
    if let Some(field_id) = &issue.field_id {
        parts.push(format!("field {field_id}"));
    }
    if let Some(block_id) = &issue.block_id {
        parts.push(format!("block {block_id}"));
    }
    if let Some(shape_id) = issue.shape_id {
        parts.push(format!("shape {}", shape_id.0));
    }
    if let Some(bounds) = issue.bounds {
        parts.push(format!(
            "bounds {} x {}",
            format_coord(bounds.width()),
            format_coord(bounds.height())
        ));
    }

    if parts.is_empty() {
        "reticle".to_string()
    } else {
        parts.join(" / ")
    }
}

fn truncated_note(ui: &mut egui::Ui, omitted: usize, noun: &str) {
    if omitted > 0 {
        ui_chrome::muted(
            ui,
            format!("{omitted} additional {noun}(s) not shown in this view"),
        );
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
    let check_tone = if report.error_count() > 0 {
        Tone::Danger
    } else if report.warning_count() > 0 {
        Tone::Warning
    } else {
        Tone::Success
    };
    let check_detail = if report.is_clean() {
        "ready"
    } else if report.error_count() > 0 {
        "blocking"
    } else {
        "review"
    };
    let retained_detail = if report.omitted_issue_count() > 0 {
        format!("{} omitted", report.omitted_issue_count())
    } else {
        "all retained".to_string()
    };
    let metrics = [
        (
            "Layers",
            report.layer_count.to_string(),
            "mask stack",
            Tone::Neutral,
        ),
        (
            "Fields",
            report.field_count.to_string(),
            "reticle fields",
            Tone::Neutral,
        ),
        (
            "Blocks",
            report.exposure_block_count.to_string(),
            "exposure plan",
            Tone::Neutral,
        ),
        (
            "Shapes",
            report.printable_shape_count.to_string(),
            "printable",
            Tone::Neutral,
        ),
        (
            "Open checks",
            report.total_issue_count().to_string(),
            check_detail,
            check_tone,
        ),
        (
            "Check rows",
            report.issues.len().to_string(),
            retained_detail.as_str(),
            Tone::Neutral,
        ),
    ];
    ui_chrome::metric_tiles(ui, &metrics);
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

fn format_vector(value: Vector) -> String {
    format!(
        "dx {}, dy {}",
        format_coord(value.dx),
        format_coord(value.dy)
    )
}

fn format_coord(value: Coord) -> String {
    format!("{value} dbu")
}
