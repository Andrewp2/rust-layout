use std::collections::BTreeMap;

use eframe::egui::{self, Color32, Sense, vec2};
use geometry_core::{Coord, Vector};
use layout_model::{
    Document,
    mask::{MaskCheckReport, MaskIssueSeverity, MaskPrepIssue, ReticlePrep},
    mes::{FabMesData, LotId},
};
use operad::{
    ApproxTextMeasurer, ClipBehavior, ColorRgba, FontWeight, InputBehavior, StrokeStyle, TextStyle,
    TextWrap, UiDocument, UiNode, UiNodeId, UiNodeStyle, UiSize, UiVisual, layout, root_style,
    widgets,
};

use crate::{
    operad_egui,
    operad_sidecar::{SidecarRow, SidecarSection, render_sidecar, render_sidecar_interactive},
    ui_chrome::{self, Tone},
};

const MAX_MASK_ISSUE_ROWS: usize = 50;
const MAX_ISSUE_GROUP_ROWS: usize = 10;
const MAX_MASK_LAYER_ROWS: usize = 80;
const MAX_MASK_LAYER_SIDECAR_ROWS: usize = 10;
const MAX_RETICLE_FIELD_ROWS: usize = 40;
const MAX_EXPOSURE_BLOCK_ROWS: usize = 40;
const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 60.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_LAYER_ROW_LIMIT: usize = 40;
const OPERAD_ACTION_REBUILD: &str = "mask.action.rebuild";
const OPERAD_ACTION_SELECT_LOT: &str = "mask.action.select_lot.";
const OPERAD_ACTION_SET_SEVERITY: &str = "mask.action.set_severity.";
const OPERAD_ACTION_SET_GROUPING: &str = "mask.action.set_grouping.";
const OPERAD_ACTION_PREV_PAGE: &str = "mask.action.prev_page";
const OPERAD_ACTION_NEXT_PAGE: &str = "mask.action.next_page";
const OPERAD_ACTION_CLEAR_FILTERS: &str = "mask.action.clear_filters";

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

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Errors => "errors",
            Self::Warnings => "warnings",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "errors" => Some(Self::Errors),
            "warnings" => Some(Self::Warnings),
            _ => None,
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

    fn slug(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Layer => "layer",
            Self::Field => "field",
            Self::Block => "block",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "code" => Some(Self::Code),
            "layer" => Some(Self::Layer),
            "field" => Some(Self::Field),
            "block" => Some(Self::Block),
            _ => None,
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

#[derive(Debug)]
struct MaskOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct MaskMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct MaskOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
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
        if let Err(error) = self.operad_ui(ui, document, mes, selected_mes_lot, status, &report) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, document, mes, selected_mes_lot, status, &report);
        }
    }

    fn operad_ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_mes_lot: &mut Option<LotId>,
        status: &mut String,
        report: &MaskCheckReport,
    ) -> Result<(), String> {
        let mut action_status = None;
        let mut result_state = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("mask_prep_dashboard_operad")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let mut view =
                    self.build_operad_view(width, report, selected_mes_lot.as_ref(), mes);
                if let Err(error) = view
                    .document
                    .compute_layout(view.size, &mut ApproxTextMeasurer)
                    .map_err(|error| error.to_string())
                {
                    result_state = Err(error);
                    return;
                }

                let (rect, response) =
                    ui.allocate_exact_size(vec2(width, view.size.height), Sense::click());
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(node_name) =
                        operad_egui::hit_test_name(&view.document, rect, pointer)
                    && let Some(message) =
                        self.handle_operad_action(&node_name, document, mes, selected_mes_lot)
                {
                    if !message.is_empty() {
                        action_status = Some(message);
                    }
                    let updated_report = self.prep.validate_document(document);
                    view = self.build_operad_view(
                        width,
                        &updated_report,
                        selected_mes_lot.as_ref(),
                        mes,
                    );
                    if let Err(error) = view
                        .document
                        .compute_layout(view.size, &mut ApproxTextMeasurer)
                        .map_err(|error| error.to_string())
                    {
                        result_state = Err(error);
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
        if let Some(message) = action_status {
            *status = message;
        }
        result_state
    }

    fn egui_dashboard_ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_mes_lot: &mut Option<LotId>,
        status: &mut String,
        report: &MaskCheckReport,
    ) {
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

                metric_row(ui, report);

                ui.separator();
                if ui.available_width() < 700.0 {
                    self.reticle_summary_ui(ui);
                    ui.separator();
                    self.exposure_blocks_ui(ui, report);
                } else {
                    ui.columns(2, |columns| {
                        self.reticle_summary_ui(&mut columns[0]);
                        self.exposure_blocks_ui(&mut columns[1], report);
                    });
                }

                ui.separator();
                ui_chrome::section_label(ui, "Reticle Fields");
                self.fields_ui(ui);

                ui.separator();
                self.issues_ui(ui, report);
            });
    }

    fn build_operad_view(
        &self,
        width: f32,
        report: &MaskCheckReport,
        selected_lot: Option<&LotId>,
        mes: &FabMesData,
    ) -> MaskOperadView {
        let metrics = self.operad_metrics(report);
        let control_rows = self.operad_control_rows(report, selected_lot, mes);
        let reticle_rows = self.operad_reticle_rows();
        let field_rows = self.operad_field_rows();
        let exposure_rows = self.operad_exposure_rows(report);
        let layer_rows = self.operad_layer_rows();
        let issue_group_rows = self.operad_issue_group_rows(report);
        let issue_rows = self.operad_issue_rows(report);
        let height = mask_operad_view_height(
            width,
            metrics.len(),
            &[
                control_rows.len(),
                reticle_rows.len(),
                field_rows.len(),
                exposure_rows.len(),
                layer_rows.len(),
                issue_group_rows.len(),
                issue_rows.len(),
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

        add_mask_operad_header(
            &mut document,
            root,
            "DESIGN HANDOFF",
            &self.prep.mask_design_id,
            "Reticle prep, layer stack, exposure blocks, fields, and retained mask checks",
            &format!(
                "{} · {} · {} retained issue row(s)",
                self.prep.layout_revision,
                selected_lot
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unlinked layout".to_string()),
                report.issues.len()
            ),
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_metric_grid(&mut document, root, width, &metrics);
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.controls",
            "Reticle Prep Controls",
            "No reticle prep controls available",
            &control_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.reticle",
            "Reticle",
            "No reticle loaded",
            &reticle_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.fields",
            "Reticle Fields",
            "No reticle fields",
            &field_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.exposure_blocks",
            "Exposure Blocks",
            "No exposure blocks",
            &exposure_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.layers",
            "Mask Layers",
            "No mask layers",
            &layer_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.issue_groups",
            &format!("Issue Summary by {}", self.issue_grouping.label()),
            "No retained mask issues match the filter",
            &issue_group_rows,
        );
        add_mask_operad_spacer(&mut document, root, OPERAD_GAP);
        add_mask_operad_section(
            &mut document,
            root,
            width,
            "mask.issues",
            "Mask Checks",
            "No mask check issues",
            &issue_rows,
        );

        MaskOperadView { document, size }
    }

    fn operad_metrics(&self, report: &MaskCheckReport) -> Vec<MaskMetricTile> {
        let check_tone = if report.error_count() > 0 {
            Tone::Danger
        } else if report.warning_count() > 0 {
            Tone::Warning
        } else {
            Tone::Success
        };
        let filtered_count = self.filtered_issue_indices(report).len();
        vec![
            MaskMetricTile {
                label: "Layers".to_string(),
                value: report.layer_count.to_string(),
                detail: "mask stack".to_string(),
                tone: Tone::Neutral,
            },
            MaskMetricTile {
                label: "Fields".to_string(),
                value: report.field_count.to_string(),
                detail: "reticle fields".to_string(),
                tone: Tone::Neutral,
            },
            MaskMetricTile {
                label: "Blocks".to_string(),
                value: report.exposure_block_count.to_string(),
                detail: "exposure plan".to_string(),
                tone: Tone::Neutral,
            },
            MaskMetricTile {
                label: "Shapes".to_string(),
                value: report.printable_shape_count.to_string(),
                detail: "printable".to_string(),
                tone: Tone::Neutral,
            },
            MaskMetricTile {
                label: "Open checks".to_string(),
                value: report.total_issue_count().to_string(),
                detail: if report.is_clean() {
                    "ready".to_string()
                } else if report.error_count() > 0 {
                    format!("{} blocking error(s)", report.error_count())
                } else {
                    format!("{} warning(s)", report.warning_count())
                },
                tone: check_tone,
            },
            MaskMetricTile {
                label: "Filtered rows".to_string(),
                value: filtered_count.to_string(),
                detail: format!(
                    "{} retained, {} omitted",
                    report.issues.len(),
                    report.omitted_issue_count()
                ),
                tone: Tone::Info,
            },
        ]
    }

    fn operad_control_rows(
        &self,
        report: &MaskCheckReport,
        selected_lot: Option<&LotId>,
        mes: &FabMesData,
    ) -> Vec<MaskOperadRow> {
        let filtered = self.filtered_issue_indices(report);
        let page_count = filtered.len().div_ceil(MAX_MASK_ISSUE_ROWS).max(1);
        let mut rows = vec![
            mask_operad_row(
                "Rebuild from current layout",
                "Regenerate reticle fields, layer stack, and exposure blocks from the active layout and lot route",
                Tone::Info,
                Some(OPERAD_ACTION_REBUILD.to_string()),
                false,
            ),
            mask_operad_row(
                "Current lot route sync",
                selected_lot
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unlinked layout".to_string()),
                Tone::Neutral,
                None,
                false,
            ),
            mask_operad_row(
                "Unlinked layout",
                "Build reticle prep directly from the active layout without MES route context",
                Tone::Neutral,
                Some(format!("{OPERAD_ACTION_SELECT_LOT}unlinked|lot")),
                selected_lot.is_none(),
            ),
        ];
        rows.extend(mes.lots.keys().map(|lot_id| {
            mask_operad_row(
                format!("Lot route: {lot_id}"),
                "Sync reticle prep to the selected lot process route",
                Tone::Neutral,
                Some(format!("{OPERAD_ACTION_SELECT_LOT}{}|lot", lot_id.as_str())),
                selected_lot == Some(lot_id),
            )
        }));
        rows.extend(IssueSeverityFilter::ALL.into_iter().map(|filter| {
            mask_operad_row(
                format!("Severity: {}", filter.label()),
                format!(
                    "{} retained row(s) match",
                    self.issue_count_for_severity(report, filter)
                ),
                if self.issue_severity_filter == filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_SEVERITY}{}|severity",
                    filter.slug()
                )),
                self.issue_severity_filter == filter,
            )
        }));
        rows.extend(IssueGrouping::ALL.into_iter().map(|grouping| {
            mask_operad_row(
                format!("Group by: {}", grouping.label()),
                "Changes the issue summary grouping",
                if self.issue_grouping == grouping {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_GROUPING}{}|grouping",
                    grouping.slug()
                )),
                self.issue_grouping == grouping,
            )
        }));
        rows.push(mask_operad_row(
            "Previous issue page",
            format!(
                "Page {} / {}",
                self.issue_page.min(page_count - 1) + 1,
                page_count
            ),
            Tone::Neutral,
            (self.issue_page > 0).then(|| OPERAD_ACTION_PREV_PAGE.to_string()),
            false,
        ));
        rows.push(mask_operad_row(
            "Next issue page",
            format!(
                "Page {} / {}",
                self.issue_page.min(page_count - 1) + 1,
                page_count
            ),
            Tone::Neutral,
            (self.issue_page + 1 < page_count).then(|| OPERAD_ACTION_NEXT_PAGE.to_string()),
            false,
        ));
        rows.push(mask_operad_row(
            "Clear issue filters",
            format!(
                "Search '{}', severity {}, group {}",
                if self.issue_filter.trim().is_empty() {
                    "<empty>"
                } else {
                    self.issue_filter.trim()
                },
                self.issue_severity_filter.label(),
                self.issue_grouping.label()
            ),
            Tone::Neutral,
            Some(OPERAD_ACTION_CLEAR_FILTERS.to_string()),
            false,
        ));
        rows
    }

    fn operad_reticle_rows(&self) -> Vec<MaskOperadRow> {
        let printable = self.prep.reticle.printable_bounds();
        vec![
            mask_operad_row(
                "Mask design",
                format!(
                    "{} · {}",
                    self.prep.mask_design_id, self.prep.layout_revision
                ),
                Tone::Info,
                None,
                false,
            ),
            mask_operad_row(
                "Route",
                self.prep
                    .route_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unlinked layout".to_string()),
                Tone::Neutral,
                None,
                false,
            ),
            mask_operad_row(
                format!(
                    "Reticle {} · {}",
                    self.prep.reticle.id, self.prep.reticle.name
                ),
                format!(
                    "size {} x {} · printable {} x {}",
                    format_coord(self.prep.reticle.size.width),
                    format_coord(self.prep.reticle.size.height),
                    format_coord(printable.width()),
                    format_coord(printable.height())
                ),
                Tone::Neutral,
                None,
                false,
            ),
            mask_operad_row(
                "Field and alignment limits",
                format!(
                    "max field {} x {} · edge clearance {} · alignment clearance {}",
                    format_coord(self.prep.reticle.size.max_field_width),
                    format_coord(self.prep.reticle.size.max_field_height),
                    format_coord(self.prep.reticle.edge_clearance),
                    format_coord(self.prep.reticle.alignment_clearance)
                ),
                Tone::Neutral,
                None,
                false,
            ),
        ]
    }

    fn operad_field_rows(&self) -> Vec<MaskOperadRow> {
        let mut rows = self
            .prep
            .fields
            .iter()
            .take(MAX_RETICLE_FIELD_ROWS)
            .map(|field| {
                mask_operad_row(
                    format!("{} · {}", field.id, field.name),
                    format!(
                        "cell {} · origin {} · bounds {} x {} · stepping {} x {} · pitch {}",
                        field.source_cell.0,
                        format_vector(field.reticle_origin),
                        format_coord(field.layout_bounds.width()),
                        format_coord(field.layout_bounds.height()),
                        field.stepping.columns,
                        field.stepping.rows,
                        format_vector(field.stepping.pitch)
                    ),
                    Tone::Neutral,
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if self.prep.fields.len() > MAX_RETICLE_FIELD_ROWS {
            rows.push(mask_operad_row(
                format!(
                    "Showing first {} of {} field definitions",
                    MAX_RETICLE_FIELD_ROWS,
                    self.prep.fields.len()
                ),
                "Additional fields are retained in the model but not expanded in this dashboard",
                Tone::Info,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_exposure_rows(&self, report: &MaskCheckReport) -> Vec<MaskOperadRow> {
        let mut rows = self
            .prep
            .exposure_blocks
            .iter()
            .take(MAX_EXPOSURE_BLOCK_ROWS)
            .map(|block| {
                let (errors, warnings) = issue_counts_for_block(report, &block.id.to_string());
                let tone = if errors > 0 {
                    Tone::Danger
                } else if warnings > 0 {
                    Tone::Warning
                } else {
                    Tone::Success
                };
                mask_operad_row(
                    format!("{} · {}", block.id, block.name),
                    format!(
                        "field {} · dose {:.1} mJ/cm2 · focus {:+.2} um · passes {} · layers {} · {} issue(s)",
                        block.field_id,
                        block.dose_mj_cm2,
                        block.focus_offset_um,
                        block.passes,
                        block
                            .layer_ids
                            .iter()
                            .map(|layer| layer.0.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        errors + warnings
                    ),
                    tone,
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if self.prep.exposure_blocks.len() > MAX_EXPOSURE_BLOCK_ROWS {
            rows.push(mask_operad_row(
                format!(
                    "Showing first {} of {} exposure blocks",
                    MAX_EXPOSURE_BLOCK_ROWS,
                    self.prep.exposure_blocks.len()
                ),
                "Additional exposure blocks are retained in the model but not expanded in this dashboard",
                Tone::Info,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_layer_rows(&self) -> Vec<MaskOperadRow> {
        let mut rows = self
            .prep
            .layer_stack
            .iter()
            .take(OPERAD_LAYER_ROW_LIMIT)
            .map(|layer| {
                mask_operad_row(
                    format!(
                        "Layer {} · {} · {}",
                        layer.layer.0,
                        layer.name,
                        layer.tone.label()
                    ),
                    format!(
                        "{:?} · feature {} · spacing {} · display order {}{}",
                        layer.process,
                        format_coord(layer.min_feature),
                        format_coord(layer.min_spacing),
                        layer.display_order,
                        if layer.critical { " · critical" } else { "" }
                    ),
                    if layer.critical {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if self.prep.layer_stack.len() > OPERAD_LAYER_ROW_LIMIT {
            rows.push(mask_operad_row(
                format!(
                    "Showing first {} of {} mask layers",
                    OPERAD_LAYER_ROW_LIMIT,
                    self.prep.layer_stack.len()
                ),
                "Additional layers remain available in the layer-stack side panel",
                Tone::Info,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_issue_group_rows(&self, report: &MaskCheckReport) -> Vec<MaskOperadRow> {
        let filtered_indices = self.filtered_issue_indices(report);
        let mut counts = BTreeMap::<String, IssueCounts>::new();
        for index in &filtered_indices {
            let issue = &report.issues[*index];
            counts
                .entry(issue_group_key(issue, self.issue_grouping))
                .or_default()
                .add(issue.severity);
        }
        let total_groups = counts.len();
        let mut rows = counts.into_iter().collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .1
                .total()
                .cmp(&left.1.total())
                .then_with(|| left.0.cmp(&right.0))
        });
        let mut operad_rows = rows
            .iter()
            .take(MAX_ISSUE_GROUP_ROWS)
            .map(|(group, counts)| {
                mask_operad_row(
                    format!("{group} · {} total", counts.total()),
                    format!("{} error(s), {} warning(s)", counts.errors, counts.warnings),
                    if counts.errors > 0 {
                        Tone::Danger
                    } else {
                        Tone::Warning
                    },
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if total_groups > MAX_ISSUE_GROUP_ROWS {
            operad_rows.push(mask_operad_row(
                format!("Showing first {MAX_ISSUE_GROUP_ROWS} of {total_groups} issue groups"),
                "Narrow issue filters or switch grouping to inspect additional groups",
                Tone::Info,
                None,
                false,
            ));
        }
        if report.omitted_issue_count() > 0 {
            operad_rows.push(mask_operad_row(
                "Retained issue rows only",
                format!(
                    "{} additional issue row(s) were summarized by the validator",
                    report.omitted_issue_count()
                ),
                Tone::Warning,
                None,
                false,
            ));
        }
        operad_rows
    }

    fn operad_issue_rows(&self, report: &MaskCheckReport) -> Vec<MaskOperadRow> {
        if report.is_clean() {
            return vec![mask_operad_row(
                "Checks clean",
                "No mask prep issues",
                Tone::Success,
                None,
                false,
            )];
        }
        let filtered_indices = self.filtered_issue_indices(report);
        if filtered_indices.is_empty() {
            return Vec::new();
        }
        let page_count = filtered_indices.len().div_ceil(MAX_MASK_ISSUE_ROWS).max(1);
        let page = self.issue_page.min(page_count - 1);
        let start = page * MAX_MASK_ISSUE_ROWS;
        let end = (start + MAX_MASK_ISSUE_ROWS).min(filtered_indices.len());
        let mut rows = vec![mask_operad_row(
            format!(
                "Showing retained rows {}-{} of {}",
                start + 1,
                end,
                filtered_indices.len()
            ),
            format!(
                "{} retained, {} total, {} omitted, page {} / {}",
                report.issues.len(),
                report.total_issue_count(),
                report.omitted_issue_count(),
                page + 1,
                page_count
            ),
            Tone::Info,
            None,
            false,
        )];
        rows.extend(filtered_indices[start..end].iter().map(|index| {
            let issue = &report.issues[*index];
            mask_operad_row(
                format!("{} · {}", issue.severity.label(), issue.code),
                format!("{} · {}", issue_target_label(issue), issue.message),
                issue_tone(issue.severity),
                None,
                false,
            )
        }));
        rows
    }

    fn filtered_issue_indices(&self, report: &MaskCheckReport) -> Vec<usize> {
        let filter = self.issue_filter.trim().to_ascii_lowercase();
        report
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
            .collect()
    }

    fn issue_count_for_severity(
        &self,
        report: &MaskCheckReport,
        severity_filter: IssueSeverityFilter,
    ) -> usize {
        let filter = self.issue_filter.trim().to_ascii_lowercase();
        report
            .issues
            .iter()
            .filter(|issue| severity_filter.allows(issue.severity))
            .filter(|issue| issue_matches_filter(issue, &filter))
            .count()
    }

    fn handle_operad_action(
        &mut self,
        node_name: &str,
        document: &Document,
        mes: &FabMesData,
        selected_mes_lot: &mut Option<LotId>,
    ) -> Option<String> {
        if node_name == OPERAD_ACTION_REBUILD {
            self.rebuild(document, mes, selected_mes_lot.as_ref());
            let report = self.prep.validate_document(document);
            return Some(format!(
                "mask prep rebuilt from current layout: {} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            ));
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SELECT_LOT) {
            let lot = rest.split('|').next().unwrap_or_default();
            if lot == "unlinked" {
                *selected_mes_lot = None;
                self.rebuild(document, mes, None);
                return Some("mask prep route sync cleared".to_string());
            }
            let lot_id = LotId::new(lot);
            if mes.lots.contains_key(&lot_id) {
                *selected_mes_lot = Some(lot_id.clone());
                self.rebuild(document, mes, selected_mes_lot.as_ref());
                return Some(format!("mask prep synced to lot route {lot_id}"));
            }
            return None;
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SET_SEVERITY) {
            let filter =
                IssueSeverityFilter::from_slug(rest.split('|').next().unwrap_or_default())?;
            self.issue_severity_filter = filter;
            self.issue_page = 0;
            return Some(format!(
                "mask issue severity filter set to {}",
                filter.label()
            ));
        }
        if let Some(rest) = node_name.strip_prefix(OPERAD_ACTION_SET_GROUPING) {
            let grouping = IssueGrouping::from_slug(rest.split('|').next().unwrap_or_default())?;
            self.issue_grouping = grouping;
            return Some(format!("mask issues grouped by {}", grouping.label()));
        }
        match node_name {
            OPERAD_ACTION_PREV_PAGE => {
                if self.issue_page > 0 {
                    self.issue_page -= 1;
                }
                Some(format!("mask issue page {}", self.issue_page + 1))
            }
            OPERAD_ACTION_NEXT_PAGE => {
                self.issue_page = self.issue_page.saturating_add(1);
                Some(format!("mask issue page {}", self.issue_page + 1))
            }
            OPERAD_ACTION_CLEAR_FILTERS => {
                self.issue_filter.clear();
                self.issue_severity_filter = IssueSeverityFilter::All;
                self.issue_grouping = IssueGrouping::Code;
                self.issue_page = 0;
                Some("mask issue filters cleared".to_string())
            }
            _ => None,
        }
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
        if self
            .operad_context_ui(ui, document, mes, selected_lot, status)
            .is_err()
        {
            self.egui_context_ui(ui, document, mes, selected_lot, status);
        }
    }

    fn operad_context_ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_lot: Option<&LotId>,
        status: &mut String,
    ) -> Result<(), String> {
        let report = self.prep.validate_document(document);
        let printable = self.prep.reticle.printable_bounds();

        let mut sections = Vec::new();
        let mut mask = SidecarSection::new("Mask")
            .row(SidecarRow::new(
                "Mask design",
                self.prep.mask_design_id.clone(),
                Tone::Info,
            ))
            .row(SidecarRow::new(
                "Reticle",
                self.prep.reticle.id.to_string(),
                Tone::Neutral,
            ))
            .row(SidecarRow::new(
                "Printable",
                format!(
                    "{} x {}",
                    format_coord(printable.width()),
                    format_coord(printable.height())
                ),
                Tone::Neutral,
            ));
        if let Some(route_id) = &self.prep.route_id {
            mask = mask.row(SidecarRow::new(
                "Route",
                route_id.to_string(),
                Tone::Neutral,
            ));
        }
        sections.push(mask);

        sections.push(
            SidecarSection::new("Check Summary")
                .row(SidecarRow::new(
                    if report.is_clean() {
                        "Checks clean"
                    } else if report.error_count() > 0 {
                        "Errors require review"
                    } else {
                        "Warnings require review"
                    },
                    format!(
                        "{} error(s), {} warning(s)",
                        report.error_count(),
                        report.warning_count()
                    ),
                    if report.is_clean() {
                        Tone::Success
                    } else if report.error_count() > 0 {
                        Tone::Danger
                    } else {
                        Tone::Warning
                    },
                ))
                .row(SidecarRow::new(
                    "Geometry",
                    format!(
                        "{} layers / {} fields / {} blocks / {} shapes",
                        report.layer_count,
                        report.field_count,
                        report.exposure_block_count,
                        report.printable_shape_count
                    ),
                    Tone::Neutral,
                ))
                .row(
                    SidecarRow::new(
                        "Rebuild from current layout",
                        "Refresh reticle prep and route context",
                        Tone::Info,
                    )
                    .action(OPERAD_ACTION_REBUILD),
                ),
        );

        let total_issues = report.total_issue_count();
        let shown = report.issues.len().min(8);
        let mut issues = SidecarSection::new(format!("Open Issues ({shown} of {total_issues})"))
            .empty("No mask prep issues");
        for issue in report.issues.iter().take(shown) {
            issues = issues.row(SidecarRow::new(
                format!("{} · {}", issue.severity.label(), issue.code),
                format!("{} · {}", issue_target_label(issue), issue.message),
                issue_tone(issue.severity),
            ));
        }
        if report.omitted_issue_count() > 0 {
            issues = issues.row(SidecarRow::new(
                "Additional retained rows summarized",
                format!("{} omitted issue row(s)", report.omitted_issue_count()),
                Tone::Warning,
            ));
        }
        sections.push(issues);

        if let Some(action) = render_sidecar_interactive(ui, "mask.context", &sections)?
            && action == OPERAD_ACTION_REBUILD
        {
            self.rebuild(document, mes, selected_lot);
            *status = "mask prep rebuilt from current layout and lot route".to_string();
        }
        Ok(())
    }

    fn egui_context_ui(
        &mut self,
        ui: &mut egui::Ui,
        document: &Document,
        mes: &FabMesData,
        selected_lot: Option<&LotId>,
        status: &mut String,
    ) {
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
        if self.operad_layer_stack_ui(ui).is_err() {
            self.egui_layer_stack_ui(ui);
        }
    }

    fn operad_layer_stack_ui(&self, ui: &mut egui::Ui) -> Result<(), String> {
        let shown = self.prep.layer_stack.len().min(MAX_MASK_LAYER_SIDECAR_ROWS);
        let mut section = SidecarSection::new(format!(
            "Mask Layers ({shown} of {})",
            self.prep.layer_stack.len()
        ))
        .empty("No mask layers");
        for layer in self
            .prep
            .layer_stack
            .iter()
            .take(MAX_MASK_LAYER_SIDECAR_ROWS)
        {
            let process = format!("{:?}", layer.process);
            let tone = layer.tone.label();
            let min_feature = format_coord(layer.min_feature);
            let min_spacing = format_coord(layer.min_spacing);
            section = section.row(SidecarRow::new(
                format!("{} · {}", layer.layer.0, layer.name),
                format!("{process} / {tone} / feature {min_feature} / spacing {min_spacing}"),
                if layer.critical {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            ));
        }
        if self.prep.layer_stack.len() > MAX_MASK_LAYER_SIDECAR_ROWS {
            section = section.row(SidecarRow::new(
                "Additional layers summarized",
                format!(
                    "{} more mask layer(s)",
                    self.prep
                        .layer_stack
                        .len()
                        .saturating_sub(MAX_MASK_LAYER_SIDECAR_ROWS)
                ),
                Tone::Info,
            ));
        }
        render_sidecar(ui, "mask.layers", &[section])
    }

    fn egui_layer_stack_ui(&self, ui: &mut egui::Ui) {
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

fn mask_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += mask_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += mask_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn mask_operad_metric_columns(width: f32) -> usize {
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

fn mask_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = mask_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn mask_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_mask_operad_header(
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
            "mask.header",
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
    add_mask_operad_text(
        document,
        header,
        "mask.header.eyebrow",
        eyebrow,
        mask_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_mask_operad_text(
        document,
        header,
        "mask.header.title",
        truncate_middle(title, 80),
        mask_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_mask_operad_text(
        document,
        header,
        "mask.header.detail",
        detail,
        mask_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_mask_operad_text(
        document,
        header,
        "mask.header.meta",
        truncate_middle(meta, 120),
        mask_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_mask_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[MaskMetricTile],
) {
    let columns = mask_operad_metric_columns(width);
    let grid_height = mask_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "mask.metrics",
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
                format!("mask.metrics.row.{row_index}"),
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
            add_mask_operad_metric_tile(
                document,
                row,
                &format!("mask.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_mask_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &MaskMetricTile,
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
            Some(StrokeStyle::new(mask_operad_tone_color(metric.tone), 1.0)),
            6.0,
        )),
    );
    add_mask_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        mask_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_mask_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        truncate_middle(&metric.value, 42),
        mask_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_mask_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 56),
        mask_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            mask_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_mask_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[MaskOperadRow],
) {
    let height = mask_operad_section_height(rows.len());
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
    add_mask_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        mask_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_mask_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_mask_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_mask_operad_empty_row(document: &mut UiDocument, parent: UiNodeId, name: &str, label: &str) {
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
    add_mask_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        mask_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_mask_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &MaskOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        mask_operad_tone_color(Tone::Info)
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
        .with_visual(UiVisual::panel(mask_operad_tone_color(row.tone), None, 2.0)),
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
    add_mask_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 78),
        mask_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        22.0,
    );
    add_mask_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 116),
        mask_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        20.0,
    );
}

fn add_mask_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("mask.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_mask_operad_text(
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

fn mask_operad_text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn mask_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn mask_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> MaskOperadRow {
    MaskOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
    }
}

fn issue_tone(severity: MaskIssueSeverity) -> Tone {
    match severity {
        MaskIssueSeverity::Error => Tone::Danger,
        MaskIssueSeverity::Warning => Tone::Warning,
    }
}

fn issue_counts_for_block(report: &MaskCheckReport, block_id: &str) -> (usize, usize) {
    let mut counts = IssueCounts::default();
    for issue in &report.issues {
        if issue
            .block_id
            .as_ref()
            .is_some_and(|candidate| candidate.to_string() == block_id)
        {
            counts.add(issue.severity);
        }
    }
    (counts.errors, counts.warnings)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_operad_view_audits_common_widths() {
        let document = Document::demo();
        let mes = FabMesData::sample();
        let selected_lot = mes.lots.keys().next().cloned();
        let panel = MaskPrepPanel::new(&document, &mes, selected_lot.as_ref());
        let report = panel.prep.validate_document(&document);

        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width, &report, selected_lot.as_ref(), &mes);
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
    fn mask_operad_actions_update_panel_state() {
        let document = Document::demo();
        let mes = FabMesData::sample();
        let target_lot = mes.lots.keys().next().unwrap().clone();
        let mut selected_lot = None;
        let mut panel = MaskPrepPanel::new(&document, &mes, None);

        assert_eq!(
            panel.handle_operad_action(
                &format!("{OPERAD_ACTION_SELECT_LOT}{}|test", target_lot.as_str()),
                &document,
                &mes,
                &mut selected_lot,
            ),
            Some(format!("mask prep synced to lot route {target_lot}"))
        );
        assert_eq!(selected_lot.as_ref(), Some(&target_lot));
        assert_eq!(panel.source_lot.as_ref(), Some(&target_lot));

        assert_eq!(
            panel.handle_operad_action(
                &format!(
                    "{OPERAD_ACTION_SET_SEVERITY}{}|test",
                    IssueSeverityFilter::Errors.slug()
                ),
                &document,
                &mes,
                &mut selected_lot,
            ),
            Some("mask issue severity filter set to errors only".to_string())
        );
        assert_eq!(panel.issue_severity_filter, IssueSeverityFilter::Errors);

        assert_eq!(
            panel.handle_operad_action(
                &format!(
                    "{OPERAD_ACTION_SET_GROUPING}{}|test",
                    IssueGrouping::Layer.slug()
                ),
                &document,
                &mes,
                &mut selected_lot,
            ),
            Some("mask issues grouped by layer".to_string())
        );
        assert_eq!(panel.issue_grouping, IssueGrouping::Layer);

        panel.issue_filter = "metal".to_string();
        panel.issue_page = 3;
        assert_eq!(
            panel.handle_operad_action(
                OPERAD_ACTION_CLEAR_FILTERS,
                &document,
                &mes,
                &mut selected_lot,
            ),
            Some("mask issue filters cleared".to_string())
        );
        assert!(panel.issue_filter.is_empty());
        assert_eq!(panel.issue_severity_filter, IssueSeverityFilter::All);
        assert_eq!(panel.issue_grouping, IssueGrouping::Code);
        assert_eq!(panel.issue_page, 0);

        assert_eq!(
            panel.handle_operad_action(
                &format!("{OPERAD_ACTION_SELECT_LOT}unlinked|test"),
                &document,
                &mes,
                &mut selected_lot,
            ),
            Some("mask prep route sync cleared".to_string())
        );
        assert!(selected_lot.is_none());
        assert!(panel.source_lot.is_none());
    }
}
