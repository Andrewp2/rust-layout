use eframe::egui::{self, Color32, Sense, vec2};
use geometry_core::Rect;
use layout_model::{
    Document,
    layout_diff::{LayoutDiffReport, ShapeChange, ShapeChangeKind, diff_documents},
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

const DEFAULT_CHANGE_PAGE_SIZE: usize = 50;
const CHANGE_PAGE_SIZE_OPTIONS: [usize; 4] = [25, 50, 100, 200];
const OPERAD_HEADER_HEIGHT: f32 = 104.0;
const OPERAD_METRIC_HEIGHT: f32 = 88.0;
const OPERAD_SECTION_TITLE_HEIGHT: f32 = 26.0;
const OPERAD_ROW_HEIGHT: f32 = 58.0;
const OPERAD_EMPTY_ROW_HEIGHT: f32 = 44.0;
const OPERAD_GAP: f32 = 10.0;
const OPERAD_PAD: f32 = 12.0;
const OPERAD_ACTION_SET_BASELINE: &str = "layout_diff.action.set_baseline.";
const OPERAD_ACTION_SET_CANDIDATE: &str = "layout_diff.action.set_candidate.";
const OPERAD_ACTION_SWAP: &str = "layout_diff.action.swap_sources";
const OPERAD_ACTION_DEMO_CURRENT: &str = "layout_diff.action.demo_current";
const OPERAD_ACTION_EMPTY_CURRENT: &str = "layout_diff.action.empty_current";
const OPERAD_ACTION_TOGGLE_CHANGED_ONLY: &str = "layout_diff.action.toggle_changed_only";
const OPERAD_ACTION_SET_CHANGE_FILTER: &str = "layout_diff.action.set_change_filter.";
const OPERAD_ACTION_SET_PAGE_SIZE: &str = "layout_diff.action.set_page_size.";
const OPERAD_ACTION_PREV_PAGE: &str = "layout_diff.action.prev_page";
const OPERAD_ACTION_NEXT_PAGE: &str = "layout_diff.action.next_page";
const OPERAD_ACTION_RESET_FILTERS: &str = "layout_diff.action.reset_filters";
const OPERAD_ACTION_SET_REVIEW: &str = "layout_diff.action.set_review.";

pub(crate) struct LayoutDiffPanel {
    baseline: DiffSource,
    candidate: DiffSource,
    changed_only: bool,
    change_filter: ChangeKindFilter,
    change_query: String,
    change_page: usize,
    change_page_size: usize,
    review_state: ReviewDisposition,
}

#[derive(Debug)]
struct LayoutDiffOperadView {
    document: UiDocument,
    size: UiSize,
}

#[derive(Clone, Debug)]
struct LayoutDiffMetricTile {
    label: String,
    value: String,
    detail: String,
    tone: Tone,
}

#[derive(Clone, Debug)]
struct LayoutDiffOperadRow {
    title: String,
    detail: String,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffSource {
    Current,
    Demo,
    Hierarchy,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeKindFilter {
    All,
    Added,
    Removed,
    Modified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReviewDisposition {
    NeedsReview,
    Approved,
    ChangesRequested,
}

impl Default for LayoutDiffPanel {
    fn default() -> Self {
        Self {
            baseline: DiffSource::Demo,
            candidate: DiffSource::Current,
            changed_only: true,
            change_filter: ChangeKindFilter::All,
            change_query: String::new(),
            change_page: 0,
            change_page_size: DEFAULT_CHANGE_PAGE_SIZE,
            review_state: ReviewDisposition::NeedsReview,
        }
    }
}

impl LayoutDiffPanel {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        if let Err(error) = self.operad_ui(ui, current) {
            ui.colored_label(Color32::from_rgb(226, 96, 96), error);
            self.egui_dashboard_ui(ui, current);
        }
    }

    fn operad_ui(&mut self, ui: &mut egui::Ui, current: &Document) -> Result<(), String> {
        let mut result = Ok(());
        egui::ScrollArea::vertical()
            .id_salt("layout_diff_review_operad_scroll")
            .show(ui, |ui| {
                let width = ui.available_width().max(320.0);
                let report = self.report(current);
                let mut view = self.build_operad_view(width, &report);
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
                    && self.handle_operad_action(&node_name, &report)
                {
                    let updated_report = self.report(current);
                    view = self.build_operad_view(width, &updated_report);
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

    fn egui_dashboard_ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        let mut report = self.report(current);
        egui::ScrollArea::vertical()
            .id_salt("layout_diff_review")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Design review",
                    "Layout Diff Review",
                    "Compare a reference baseline against the candidate layout result.",
                    |_| {},
                );

                if self.comparison_setup_ui(ui, &report) {
                    report = self.report(current);
                }
                if self.baseline == self.candidate {
                    ui_chrome::empty_state(
                        ui,
                        "Baseline and candidate use the same source; select different sources for a meaningful diff.",
                    );
                }

                ui.separator();
                summary_ui(ui, &report);

                ui.separator();
                if ui.available_width() < 680.0 {
                    bounds_ui(
                        ui,
                        "Baseline Geometry",
                        report.baseline_name.as_str(),
                        report.baseline_bounds,
                    );
                    ui.separator();
                    bounds_ui(
                        ui,
                        "Candidate Geometry",
                        report.candidate_name.as_str(),
                        report.candidate_bounds,
                    );
                } else {
                    ui.columns(2, |columns| {
                        bounds_ui(
                            &mut columns[0],
                            "Baseline Geometry",
                            report.baseline_name.as_str(),
                            report.baseline_bounds,
                        );
                        bounds_ui(
                            &mut columns[1],
                            "Candidate Geometry",
                            report.candidate_name.as_str(),
                            report.candidate_bounds,
                        );
                    });
                }

                ui.separator();
                layers_ui(ui, &report, self.changed_only);

                ui.separator();
                self.changes_ui(ui, &report);

                ui.separator();
                self.review_actions_ui(ui, &report);
            });
    }

    fn build_operad_view(&self, width: f32, report: &LayoutDiffReport) -> LayoutDiffOperadView {
        let metrics = self.operad_metrics(report);
        let setup_rows = self.operad_setup_rows(report);
        let summary_rows = self.operad_summary_rows(report);
        let layer_rows = self.operad_layer_rows(report);
        let change_rows = self.operad_change_rows(report);
        let review_rows = self.operad_review_rows(report);
        let height = layout_diff_operad_view_height(
            width,
            metrics.len(),
            &[
                setup_rows.len(),
                summary_rows.len(),
                layer_rows.len(),
                change_rows.len(),
                review_rows.len(),
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

        add_layout_diff_operad_header(
            &mut document,
            root,
            "DESIGN REVIEW",
            "Layout Diff Review",
            "Compare a reference baseline against the candidate layout result",
            &format!(
                "{} -> {} · {}",
                report.baseline_name,
                report.candidate_name,
                comparison_status_label(report)
            ),
        );
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_metric_grid(&mut document, root, width, &metrics);
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_section(
            &mut document,
            root,
            width,
            "layout_diff.setup",
            "Comparison Setup",
            "No comparison setup available",
            &setup_rows,
        );
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_section(
            &mut document,
            root,
            width,
            "layout_diff.summary",
            "Summary",
            "No summary available",
            &summary_rows,
        );
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_section(
            &mut document,
            root,
            width,
            "layout_diff.layers",
            "Layer Summary",
            "No layers match the active layer filter",
            &layer_rows,
        );
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_section(
            &mut document,
            root,
            width,
            "layout_diff.changes",
            "Review Changes",
            "No shape-level differences match the active filters",
            &change_rows,
        );
        add_layout_diff_operad_spacer(&mut document, root, OPERAD_GAP);
        add_layout_diff_operad_section(
            &mut document,
            root,
            width,
            "layout_diff.review",
            "Review Actions",
            "No review actions available",
            &review_rows,
        );

        LayoutDiffOperadView { document, size }
    }

    fn operad_metrics(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffMetricTile> {
        vec![
            LayoutDiffMetricTile {
                label: "Shape changes".to_string(),
                value: total_changes(report).to_string(),
                detail: format!(
                    "+{} / -{} / ~{}",
                    report.summary.added_shapes,
                    report.summary.removed_shapes,
                    report.summary.modified_shapes
                ),
                tone: comparison_status_tone(report),
            },
            LayoutDiffMetricTile {
                label: "Shape count".to_string(),
                value: format!(
                    "{} -> {}",
                    report.summary.baseline_shapes, report.summary.candidate_shapes
                ),
                detail: signed_delta(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
                tone: delta_tone(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
            },
            LayoutDiffMetricTile {
                label: "Layer count".to_string(),
                value: format!(
                    "{} -> {}",
                    report.summary.baseline_layers, report.summary.candidate_layers
                ),
                detail: signed_delta(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
                tone: delta_tone(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
            },
        ]
    }

    fn operad_setup_rows(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffOperadRow> {
        let mut rows = vec![
            layout_diff_operad_row(
                format!("Baseline: {}", self.baseline.label()),
                format!("Reference geometry · {}", report.baseline_name),
                Tone::Info,
                None,
                false,
            ),
            layout_diff_operad_row(
                format!("Candidate: {}", self.candidate.label()),
                format!("Result under review · {}", report.candidate_name),
                Tone::Info,
                None,
                false,
            ),
        ];
        rows.extend(DiffSource::ALL.into_iter().map(|source| {
            layout_diff_operad_row(
                format!("Set baseline: {}", source.label()),
                "Choose the reference source".to_string(),
                if self.baseline == source {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_BASELINE}{}|setup",
                    source.slug()
                )),
                self.baseline == source,
            )
        }));
        rows.extend(DiffSource::ALL.into_iter().map(|source| {
            layout_diff_operad_row(
                format!("Set candidate: {}", source.label()),
                "Choose the result source".to_string(),
                if self.candidate == source {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_CANDIDATE}{}|setup",
                    source.slug()
                )),
                self.candidate == source,
            )
        }));
        rows.extend([
            layout_diff_operad_row(
                "Swap baseline and candidate".to_string(),
                "Reverse the comparison direction".to_string(),
                Tone::Neutral,
                Some(OPERAD_ACTION_SWAP.to_string()),
                false,
            ),
            layout_diff_operad_row(
                "Demo -> current".to_string(),
                "Use demo layout as reference and current workspace as candidate".to_string(),
                Tone::Neutral,
                Some(OPERAD_ACTION_DEMO_CURRENT.to_string()),
                false,
            ),
            layout_diff_operad_row(
                "Empty -> current".to_string(),
                "Review the candidate as entirely new geometry".to_string(),
                Tone::Neutral,
                Some(OPERAD_ACTION_EMPTY_CURRENT.to_string()),
                false,
            ),
            layout_diff_operad_row(
                if self.changed_only {
                    "Changed layers only: on"
                } else {
                    "Changed layers only: off"
                },
                "Toggle layer summary filtering".to_string(),
                if self.changed_only {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(OPERAD_ACTION_TOGGLE_CHANGED_ONLY.to_string()),
                self.changed_only,
            ),
        ]);
        if self.baseline == self.candidate {
            rows.push(layout_diff_operad_row(
                "Baseline and candidate are the same source".to_string(),
                "Select different sources for a meaningful diff".to_string(),
                Tone::Warning,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_summary_rows(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffOperadRow> {
        vec![
            layout_diff_operad_row(
                "Status".to_string(),
                comparison_status_label(report).to_string(),
                comparison_status_tone(report),
                None,
                false,
            ),
            layout_diff_operad_row(
                "Shapes".to_string(),
                format!(
                    "{} baseline, {} candidate, delta {}",
                    report.summary.baseline_shapes,
                    report.summary.candidate_shapes,
                    signed_delta(
                        report.summary.candidate_shapes,
                        report.summary.baseline_shapes
                    )
                ),
                delta_tone(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
                None,
                false,
            ),
            layout_diff_operad_row(
                "Layers".to_string(),
                format!(
                    "{} baseline, {} candidate, delta {}",
                    report.summary.baseline_layers,
                    report.summary.candidate_layers,
                    signed_delta(
                        report.summary.candidate_layers,
                        report.summary.baseline_layers
                    )
                ),
                delta_tone(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
                None,
                false,
            ),
            layout_diff_operad_row(
                "Baseline geometry".to_string(),
                bounds_label(report.baseline_bounds),
                Tone::Neutral,
                None,
                false,
            ),
            layout_diff_operad_row(
                "Candidate geometry".to_string(),
                format!(
                    "{} · {}",
                    bounds_label(report.candidate_bounds),
                    bounds_delta_label(report.baseline_bounds, report.candidate_bounds)
                ),
                Tone::Neutral,
                None,
                false,
            ),
        ]
    }

    fn operad_layer_rows(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffOperadRow> {
        let visible_layers = report
            .layers
            .iter()
            .filter(|layer| !self.changed_only || layer_total_changes(layer) > 0)
            .collect::<Vec<_>>();
        let total = visible_layers.len();
        let mut rows = visible_layers
            .into_iter()
            .take(28)
            .map(|layer| {
                layout_diff_operad_row(
                    format!("{} {}", layer.layer.0, layer.name),
                    format!(
                        "{} -> {} · +{} / -{} / ~{}",
                        layer.baseline_shapes,
                        layer.candidate_shapes,
                        layer.added_shapes,
                        layer.removed_shapes,
                        layer.modified_shapes
                    ),
                    if layer.removed_shapes > 0 {
                        Tone::Danger
                    } else if layer.added_shapes + layer.modified_shapes > 0 {
                        Tone::Warning
                    } else {
                        Tone::Neutral
                    },
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if total > rows.len() {
            rows.push(layout_diff_operad_row(
                format!("Showing first {} of {total} layers", rows.len()),
                "Toggle changed-only or use the fallback table for all rows".to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        rows
    }

    fn operad_change_rows(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffOperadRow> {
        let filter = self.change_query.trim().to_ascii_lowercase();
        let filtered_indices = report
            .changes
            .iter()
            .enumerate()
            .filter_map(|(index, change)| {
                (self.change_filter.matches(change.kind) && change_matches_query(change, &filter))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let page_size = self.normalized_page_size();
        let page_count = filtered_indices.len().div_ceil(page_size).max(1);
        let page = self.change_page.min(page_count - 1);
        let start = page * page_size;
        let end = (start + page_size).min(filtered_indices.len());
        let mut rows = vec![
            layout_diff_operad_row(
                format!(
                    "Showing {}-{} of {} filtered, {} total",
                    if filtered_indices.is_empty() {
                        0
                    } else {
                        start + 1
                    },
                    end,
                    filtered_indices.len(),
                    report.changes.len()
                ),
                format!(
                    "Page {} / {}, {} rows per page",
                    page + 1,
                    page_count,
                    page_size
                ),
                Tone::Info,
                None,
                false,
            ),
            layout_diff_operad_row(
                format!("Change filter: {}", self.change_filter.label()),
                "Filter shape-level changes".to_string(),
                Tone::Info,
                None,
                false,
            ),
        ];
        rows.extend(ChangeKindFilter::ALL.into_iter().map(|filter| {
            layout_diff_operad_row(
                format!("Show: {}", filter.label()),
                "Set change-kind filter".to_string(),
                if self.change_filter == filter {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!(
                    "{OPERAD_ACTION_SET_CHANGE_FILTER}{}|changes",
                    filter.slug()
                )),
                self.change_filter == filter,
            )
        }));
        rows.extend(CHANGE_PAGE_SIZE_OPTIONS.into_iter().map(|page_size| {
            layout_diff_operad_row(
                format!("Rows per page: {page_size}"),
                "Set review page size".to_string(),
                if self.normalized_page_size() == page_size {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                Some(format!("{OPERAD_ACTION_SET_PAGE_SIZE}{page_size}|changes")),
                self.normalized_page_size() == page_size,
            )
        }));
        rows.extend([
            layout_diff_operad_row(
                "Previous page".to_string(),
                "Move backward in the filtered change list".to_string(),
                if page > 0 { Tone::Info } else { Tone::Neutral },
                (page > 0).then(|| OPERAD_ACTION_PREV_PAGE.to_string()),
                false,
            ),
            layout_diff_operad_row(
                "Next page".to_string(),
                "Move forward in the filtered change list".to_string(),
                if page + 1 < page_count {
                    Tone::Info
                } else {
                    Tone::Neutral
                },
                (page + 1 < page_count).then(|| OPERAD_ACTION_NEXT_PAGE.to_string()),
                false,
            ),
            layout_diff_operad_row(
                "Reset filters".to_string(),
                "Clear search, show all change kinds, and restore default page size".to_string(),
                Tone::Neutral,
                Some(OPERAD_ACTION_RESET_FILTERS.to_string()),
                false,
            ),
        ]);
        if !self.change_query.trim().is_empty() {
            rows.push(layout_diff_operad_row(
                format!("Search: {}", self.change_query.trim()),
                "Text search stays on the fallback egui path until Operad has full edit routing"
                    .to_string(),
                Tone::Neutral,
                None,
                false,
            ));
        }
        for index in &filtered_indices[start..end] {
            let change = &report.changes[*index];
            rows.push(layout_diff_operad_row(
                format!("{} · shape #{}", change.kind.label(), change.id.0),
                format!(
                    "layer {} {} · {} x {} · {}",
                    change.layer.0,
                    change.layer_name,
                    change.bounds.width().abs(),
                    change.bounds.height().abs(),
                    change.detail
                ),
                shape_change_tone(change.kind),
                None,
                false,
            ));
        }
        rows
    }

    fn operad_review_rows(&self, report: &LayoutDiffReport) -> Vec<LayoutDiffOperadRow> {
        let mut rows = ReviewDisposition::ALL
            .into_iter()
            .map(|state| {
                layout_diff_operad_row(
                    state.label().to_string(),
                    review_guidance(report, state),
                    state.tone(),
                    Some(format!("{OPERAD_ACTION_SET_REVIEW}{}|review", state.slug())),
                    self.review_state == state,
                )
            })
            .collect::<Vec<_>>();
        rows.push(layout_diff_operad_row(
            "Current guidance".to_string(),
            review_guidance(report, self.review_state),
            self.review_state.tone(),
            None,
            false,
        ));
        rows
    }

    fn handle_operad_action(&mut self, node_name: &str, report: &LayoutDiffReport) -> bool {
        if node_name == OPERAD_ACTION_SWAP {
            std::mem::swap(&mut self.baseline, &mut self.candidate);
            self.reset_result_position();
            self.review_state = ReviewDisposition::NeedsReview;
            return true;
        }
        if node_name == OPERAD_ACTION_DEMO_CURRENT {
            self.baseline = DiffSource::Demo;
            self.candidate = DiffSource::Current;
            self.reset_result_position();
            self.review_state = ReviewDisposition::NeedsReview;
            return true;
        }
        if node_name == OPERAD_ACTION_EMPTY_CURRENT {
            self.baseline = DiffSource::Empty;
            self.candidate = DiffSource::Current;
            self.reset_result_position();
            self.review_state = ReviewDisposition::NeedsReview;
            return true;
        }
        if node_name == OPERAD_ACTION_TOGGLE_CHANGED_ONLY {
            self.changed_only = !self.changed_only;
            return true;
        }
        if node_name == OPERAD_ACTION_PREV_PAGE && self.change_page > 0 {
            self.change_page -= 1;
            return true;
        }
        if node_name == OPERAD_ACTION_NEXT_PAGE {
            let page_size = self.normalized_page_size();
            let page_count = report.changes.len().div_ceil(page_size).max(1);
            if self.change_page + 1 < page_count {
                self.change_page += 1;
                return true;
            }
        }
        if node_name == OPERAD_ACTION_RESET_FILTERS {
            self.change_filter = ChangeKindFilter::All;
            self.change_query.clear();
            self.change_page = 0;
            self.change_page_size = DEFAULT_CHANGE_PAGE_SIZE;
            return true;
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_BASELINE) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(source) = DiffSource::from_slug(slug) {
                self.baseline = source;
                self.reset_result_position();
                self.review_state = ReviewDisposition::NeedsReview;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_CANDIDATE) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(source) = DiffSource::from_slug(slug) {
                self.candidate = source;
                self.reset_result_position();
                self.review_state = ReviewDisposition::NeedsReview;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_CHANGE_FILTER) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(filter) = ChangeKindFilter::from_slug(slug) {
                self.change_filter = filter;
                self.change_page = 0;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_PAGE_SIZE) {
            let value = value
                .split_once('|')
                .map(|(value, _)| value)
                .unwrap_or(value);
            if let Ok(page_size) = value.parse::<usize>()
                && CHANGE_PAGE_SIZE_OPTIONS.contains(&page_size)
            {
                self.change_page_size = page_size;
                self.change_page = 0;
                return true;
            }
        }
        if let Some(value) = node_name.strip_prefix(OPERAD_ACTION_SET_REVIEW) {
            let slug = value.split_once('|').map(|(slug, _)| slug).unwrap_or(value);
            if let Some(state) = ReviewDisposition::from_slug(slug) {
                self.review_state = state;
                return true;
            }
        }
        false
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        if self.operad_context_ui(ui, current).is_err() {
            self.egui_context_ui(ui, current);
        }
    }

    fn operad_context_ui(&mut self, ui: &mut egui::Ui, current: &Document) -> Result<(), String> {
        let report = self.report(current);
        let sections = vec![
            SidecarSection::new("Layout Diff")
                .row(SidecarRow::new(
                    "Baseline",
                    report.baseline_name.clone(),
                    Tone::Neutral,
                ))
                .row(SidecarRow::new(
                    "Candidate",
                    report.candidate_name.clone(),
                    if self.baseline == self.candidate {
                        Tone::Warning
                    } else {
                        Tone::Info
                    },
                )),
            SidecarSection::new("Change Summary")
                .row(SidecarRow::new(
                    format!("{} added", report.summary.added_shapes),
                    "New candidate geometry",
                    if report.summary.added_shapes == 0 {
                        Tone::Neutral
                    } else {
                        Tone::Success
                    },
                ))
                .row(SidecarRow::new(
                    format!("{} removed", report.summary.removed_shapes),
                    "Missing from candidate",
                    if report.summary.removed_shapes == 0 {
                        Tone::Neutral
                    } else {
                        Tone::Danger
                    },
                ))
                .row(SidecarRow::new(
                    format!("{} modified", report.summary.modified_shapes),
                    self.review_state.label(),
                    if report.summary.modified_shapes == 0 {
                        Tone::Neutral
                    } else {
                        Tone::Warning
                    },
                )),
        ];
        render_sidecar(ui, "layout_diff.context", &sections)
    }

    fn egui_context_ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        let report = self.report(current);

        ui_chrome::section_label(ui, "Layout Diff");
        ui.label(format!("Baseline: {}", report.baseline_name));
        ui.label(format!("Candidate: {}", report.candidate_name));
        if self.baseline == self.candidate {
            ui_chrome::status_pill(ui, "same source", Tone::Warning);
        }
        ui.separator();
        ui_chrome::status_pill(
            ui,
            &format!("{} added", report.summary.added_shapes),
            if report.summary.added_shapes == 0 {
                Tone::Neutral
            } else {
                Tone::Success
            },
        );
        ui_chrome::status_pill(
            ui,
            &format!("{} removed", report.summary.removed_shapes),
            if report.summary.removed_shapes == 0 {
                Tone::Neutral
            } else {
                Tone::Danger
            },
        );
        ui_chrome::status_pill(
            ui,
            &format!("{} modified", report.summary.modified_shapes),
            if report.summary.modified_shapes == 0 {
                Tone::Neutral
            } else {
                Tone::Warning
            },
        );
        ui_chrome::status_pill(ui, self.review_state.label(), self.review_state.tone());
    }

    fn report(&self, current: &Document) -> LayoutDiffReport {
        let baseline = self.baseline.document(current);
        let candidate = self.candidate.document(current);
        diff_documents(
            self.baseline.label(),
            &baseline,
            self.candidate.label(),
            &candidate,
        )
    }

    fn comparison_setup_ui(&mut self, ui: &mut egui::Ui, report: &LayoutDiffReport) -> bool {
        ui_chrome::section_label(ui, "Comparison Setup");
        let before = (self.baseline, self.candidate);
        egui::Grid::new("layout_diff_source_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Baseline (reference / before)");
                source_picker(ui, "layout_diff_baseline", &mut self.baseline);
                ui.end_row();

                ui.strong("Candidate (result / after)");
                source_picker(ui, "layout_diff_candidate", &mut self.candidate);
                ui.end_row();
            });

        ui.horizontal_wrapped(|ui| {
            if ui.button("Swap").clicked() {
                std::mem::swap(&mut self.baseline, &mut self.candidate);
                self.reset_result_position();
                self.review_state = ReviewDisposition::NeedsReview;
            }
            if ui.button("Demo -> current").clicked() {
                self.baseline = DiffSource::Demo;
                self.candidate = DiffSource::Current;
                self.reset_result_position();
                self.review_state = ReviewDisposition::NeedsReview;
            }
            if ui.button("Empty -> current").clicked() {
                self.baseline = DiffSource::Empty;
                self.candidate = DiffSource::Current;
                self.reset_result_position();
                self.review_state = ReviewDisposition::NeedsReview;
            }
            ui.checkbox(&mut self.changed_only, "Changed layers only");
            ui_chrome::status_pill(
                ui,
                comparison_status_label(report),
                comparison_status_tone(report),
            );
        });

        ui_chrome::muted(
            ui,
            format!(
                "{} is treated as expected geometry; {} is the result under review.",
                report.baseline_name, report.candidate_name
            ),
        );

        if before != (self.baseline, self.candidate) {
            self.reset_result_position();
            self.review_state = ReviewDisposition::NeedsReview;
            true
        } else {
            false
        }
    }

    fn changes_ui(&mut self, ui: &mut egui::Ui, report: &LayoutDiffReport) {
        ui_chrome::section_label(ui, "Review Changes");
        if report.changes.is_empty() {
            ui_chrome::empty_state(
                ui,
                "No shape-level differences between baseline and candidate",
            );
            return;
        }

        self.change_filters_ui(ui);

        let filter = self.change_query.trim().to_ascii_lowercase();
        let filtered_indices = report
            .changes
            .iter()
            .enumerate()
            .filter_map(|(index, change)| {
                if self.change_filter.matches(change.kind) && change_matches_query(change, &filter)
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if filtered_indices.is_empty() {
            ui_chrome::empty_state(ui, "No shape-level differences match the active filters");
            return;
        }

        let page_size = self.normalized_page_size();
        let page_count = filtered_indices.len().div_ceil(page_size).max(1);
        self.change_page = self.change_page.min(page_count - 1);
        let start = self.change_page * page_size;
        let end = (start + page_size).min(filtered_indices.len());

        ui.horizontal_wrapped(|ui| {
            ui_chrome::muted(
                ui,
                format!(
                    "showing {}-{} of {} filtered, {} total",
                    start + 1,
                    end,
                    filtered_indices.len(),
                    report.changes.len()
                ),
            );
            if ui
                .add_enabled(self.change_page > 0, egui::Button::new("Prev"))
                .clicked()
            {
                self.change_page -= 1;
            }
            ui.label(format!("Page {} / {}", self.change_page + 1, page_count));
            if ui
                .add_enabled(self.change_page + 1 < page_count, egui::Button::new("Next"))
                .clicked()
            {
                self.change_page += 1;
            }
        });

        egui::ScrollArea::horizontal()
            .id_salt("layout_diff_changes_horizontal")
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("layout_diff_changes_vertical")
                    .max_height(360.0)
                    .show(ui, |ui| {
                        egui::Grid::new("layout_diff_changes_grid")
                            .striped(true)
                            .min_col_width(76.0)
                            .show(ui, |ui| {
                                ui.strong("Type");
                                ui.strong("Shape");
                                ui.strong("Layer");
                                ui.strong("Bounds");
                                ui.strong("Detail");
                                ui.end_row();
                                for index in &filtered_indices[start..end] {
                                    let change = &report.changes[*index];
                                    ui.colored_label(
                                        change_color(change.kind),
                                        change.kind.label(),
                                    );
                                    ui.label(format!("#{}", change.id.0));
                                    ui.label(format!("{} {}", change.layer.0, change.layer_name));
                                    ui.label(format!(
                                        "{} x {}",
                                        change.bounds.width().abs(),
                                        change.bounds.height().abs()
                                    ));
                                    ui.add(egui::Label::new(&change.detail).wrap());
                                    ui.end_row();
                                }
                            });
                    });
            });
    }

    fn change_filters_ui(&mut self, ui: &mut egui::Ui) {
        let before = (self.change_filter, self.change_page_size);
        ui.horizontal_wrapped(|ui| {
            ui.label("Show");
            egui::ComboBox::from_id_salt("layout_diff_change_filter")
                .selected_text(self.change_filter.label())
                .show_ui(ui, |ui| {
                    for filter in [
                        ChangeKindFilter::All,
                        ChangeKindFilter::Added,
                        ChangeKindFilter::Removed,
                        ChangeKindFilter::Modified,
                    ] {
                        ui.selectable_value(&mut self.change_filter, filter, filter.label());
                    }
                });

            ui.label("Search");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.change_query)
                        .desired_width(180.0)
                        .hint_text("shape, layer, detail"),
                )
                .changed()
            {
                self.change_page = 0;
            }

            ui.label("Rows");
            egui::ComboBox::from_id_salt("layout_diff_change_page_size")
                .selected_text(self.normalized_page_size().to_string())
                .show_ui(ui, |ui| {
                    for page_size in CHANGE_PAGE_SIZE_OPTIONS {
                        ui.selectable_value(
                            &mut self.change_page_size,
                            page_size,
                            page_size.to_string(),
                        );
                    }
                });

            if ui.button("Reset filters").clicked() {
                self.change_filter = ChangeKindFilter::All;
                self.change_query.clear();
                self.change_page = 0;
                self.change_page_size = DEFAULT_CHANGE_PAGE_SIZE;
            }
        });

        if before != (self.change_filter, self.change_page_size) {
            self.change_page = 0;
        }
    }

    fn review_actions_ui(&mut self, ui: &mut egui::Ui, report: &LayoutDiffReport) {
        ui_chrome::section_label(ui, "Review Actions");
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, self.review_state.label(), self.review_state.tone());
            if ui
                .selectable_label(
                    self.review_state == ReviewDisposition::NeedsReview,
                    "Needs review",
                )
                .clicked()
            {
                self.review_state = ReviewDisposition::NeedsReview;
            }
            if ui
                .selectable_label(
                    self.review_state == ReviewDisposition::Approved,
                    "Approve result",
                )
                .clicked()
            {
                self.review_state = ReviewDisposition::Approved;
            }
            if ui
                .selectable_label(
                    self.review_state == ReviewDisposition::ChangesRequested,
                    "Request changes",
                )
                .clicked()
            {
                self.review_state = ReviewDisposition::ChangesRequested;
            }
        });

        ui_chrome::muted(ui, review_guidance(report, self.review_state));
    }

    fn reset_result_position(&mut self) {
        self.change_page = 0;
    }

    fn normalized_page_size(&self) -> usize {
        if CHANGE_PAGE_SIZE_OPTIONS.contains(&self.change_page_size) {
            self.change_page_size
        } else {
            DEFAULT_CHANGE_PAGE_SIZE
        }
    }
}

impl DiffSource {
    const ALL: [Self; 4] = [Self::Current, Self::Demo, Self::Hierarchy, Self::Empty];

    fn label(self) -> &'static str {
        match self {
            Self::Current => "Current workspace",
            Self::Demo => "Demo layout",
            Self::Hierarchy => "Hierarchy demo",
            Self::Empty => "Empty layout",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Demo => "demo",
            Self::Hierarchy => "hierarchy",
            Self::Empty => "empty",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "current" => Some(Self::Current),
            "demo" => Some(Self::Demo),
            "hierarchy" => Some(Self::Hierarchy),
            "empty" => Some(Self::Empty),
            _ => None,
        }
    }

    fn document(self, current: &Document) -> Document {
        match self {
            Self::Current => current.clone(),
            Self::Demo => Document::demo(),
            Self::Hierarchy => Document::hierarchy_demo(),
            Self::Empty => Document::new("empty layout"),
        }
    }
}

impl ChangeKindFilter {
    const ALL: [Self; 4] = [Self::All, Self::Added, Self::Removed, Self::Modified];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All changes",
            Self::Added => "Added only",
            Self::Removed => "Removed only",
            Self::Modified => "Modified only",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "all" => Some(Self::All),
            "added" => Some(Self::Added),
            "removed" => Some(Self::Removed),
            "modified" => Some(Self::Modified),
            _ => None,
        }
    }

    fn matches(self, kind: ShapeChangeKind) -> bool {
        match self {
            Self::All => true,
            Self::Added => kind == ShapeChangeKind::Added,
            Self::Removed => kind == ShapeChangeKind::Removed,
            Self::Modified => kind == ShapeChangeKind::Modified,
        }
    }
}

impl ReviewDisposition {
    const ALL: [Self; 3] = [Self::NeedsReview, Self::Approved, Self::ChangesRequested];

    fn label(self) -> &'static str {
        match self {
            Self::NeedsReview => "needs review",
            Self::Approved => "approved",
            Self::ChangesRequested => "changes requested",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::NeedsReview => "needs-review",
            Self::Approved => "approved",
            Self::ChangesRequested => "changes-requested",
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "needs-review" => Some(Self::NeedsReview),
            "approved" => Some(Self::Approved),
            "changes-requested" => Some(Self::ChangesRequested),
            _ => None,
        }
    }

    fn tone(self) -> Tone {
        match self {
            Self::NeedsReview => Tone::Warning,
            Self::Approved => Tone::Success,
            Self::ChangesRequested => Tone::Danger,
        }
    }
}

fn source_picker(ui: &mut egui::Ui, id: &'static str, selected: &mut DiffSource) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected.label())
        .show_ui(ui, |ui| {
            for source in [
                DiffSource::Current,
                DiffSource::Demo,
                DiffSource::Hierarchy,
                DiffSource::Empty,
            ] {
                ui.selectable_value(selected, source, source.label());
            }
        });
}

fn summary_ui(ui: &mut egui::Ui, report: &LayoutDiffReport) {
    ui_chrome::section_label(ui, "Summary");
    ui_chrome::metric_tiles(
        ui,
        &[
            (
                "Shape changes",
                total_changes(report).to_string(),
                &format!(
                    "+{} / -{} / ~{}",
                    report.summary.added_shapes,
                    report.summary.removed_shapes,
                    report.summary.modified_shapes
                ),
                comparison_status_tone(report),
            ),
            (
                "Shape count",
                format!(
                    "{} -> {}",
                    report.summary.baseline_shapes, report.summary.candidate_shapes
                ),
                &signed_delta(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
                delta_tone(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
            ),
            (
                "Layer count",
                format!(
                    "{} -> {}",
                    report.summary.baseline_layers, report.summary.candidate_layers
                ),
                &signed_delta(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
                delta_tone(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
            ),
        ],
    );

    egui::Grid::new("layout_diff_summary_grid")
        .num_columns(4)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Metric");
            ui.strong("Baseline");
            ui.strong("Candidate");
            ui.strong("Delta");
            ui.end_row();
            summary_row(
                ui,
                "Shapes",
                report.summary.baseline_shapes.to_string(),
                report.summary.candidate_shapes.to_string(),
                signed_delta(
                    report.summary.candidate_shapes,
                    report.summary.baseline_shapes,
                ),
            );
            summary_row(
                ui,
                "Layers",
                report.summary.baseline_layers.to_string(),
                report.summary.candidate_layers.to_string(),
                signed_delta(
                    report.summary.candidate_layers,
                    report.summary.baseline_layers,
                ),
            );
            summary_row(
                ui,
                "Bounds",
                bounds_label(report.baseline_bounds),
                bounds_label(report.candidate_bounds),
                bounds_delta_label(report.baseline_bounds, report.candidate_bounds),
            );
            summary_row(
                ui,
                "Shape-level changes",
                "reference".to_string(),
                "candidate".to_string(),
                format!(
                    "+{} / -{} / ~{}",
                    report.summary.added_shapes,
                    report.summary.removed_shapes,
                    report.summary.modified_shapes
                ),
            );
        });
}

fn summary_row(ui: &mut egui::Ui, label: &str, baseline: String, candidate: String, delta: String) {
    ui.strong(label);
    ui.label(baseline);
    ui.label(candidate);
    ui.label(delta);
    ui.end_row();
}

fn bounds_ui(ui: &mut egui::Ui, label: &str, empty_subject: &str, bounds: Option<Rect>) {
    ui_chrome::section_label(ui, label);
    if let Some(bounds) = bounds {
        ui.label(format!(
            "{} x {} dbu",
            bounds.width().abs(),
            bounds.height().abs()
        ));
        ui.small(format!(
            "({},{}) to ({},{})",
            bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y
        ));
    } else {
        ui_chrome::empty_state(ui, &format!("No shapes in {empty_subject}"));
    }
}

fn layers_ui(ui: &mut egui::Ui, report: &LayoutDiffReport, changed_only: bool) {
    ui_chrome::section_label(ui, "Layer Summary");
    let visible_layers = report
        .layers
        .iter()
        .filter(|layer| !changed_only || layer_total_changes(layer) > 0)
        .collect::<Vec<_>>();
    ui.horizontal_wrapped(|ui| {
        ui_chrome::muted(
            ui,
            format!(
                "showing {} of {} layers{}",
                visible_layers.len(),
                report.layers.len(),
                if changed_only { " with changes" } else { "" }
            ),
        );
    });
    if visible_layers.is_empty() {
        ui_chrome::empty_state(ui, "No layers match the active layer filter");
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("layout_diff_layer_horizontal")
        .show(ui, |ui| {
            egui::Grid::new("layout_diff_layer_grid")
                .striped(true)
                .min_col_width(72.0)
                .show(ui, |ui| {
                    ui.strong("Layer");
                    ui.strong("Baseline");
                    ui.strong("Candidate");
                    ui.strong("Delta");
                    ui.end_row();
                    for layer in visible_layers {
                        ui.label(format!("{} {}", layer.layer.0, layer.name));
                        ui.label(layer.baseline_shapes.to_string());
                        ui.label(layer.candidate_shapes.to_string());
                        ui.label(format!(
                            "+{} / -{} / ~{}",
                            layer.added_shapes, layer.removed_shapes, layer.modified_shapes
                        ));
                        ui.end_row();
                    }
                });
        });
}

fn change_color(kind: ShapeChangeKind) -> Color32 {
    match kind {
        ShapeChangeKind::Added => Color32::from_rgb(105, 190, 120),
        ShapeChangeKind::Removed => Color32::from_rgb(245, 98, 98),
        ShapeChangeKind::Modified => Color32::from_rgb(230, 180, 80),
    }
}

fn change_matches_query(change: &ShapeChange, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }

    change.kind.label().contains(filter)
        || change.id.0.to_string().contains(filter)
        || format!("#{}", change.id.0).contains(filter)
        || change.layer.0.to_string().contains(filter)
        || change.layer_name.to_ascii_lowercase().contains(filter)
        || change.detail.to_ascii_lowercase().contains(filter)
        || format!(
            "{}x{}",
            change.bounds.width().abs(),
            change.bounds.height().abs()
        )
        .contains(filter)
}

fn layer_total_changes(layer: &layout_model::layout_diff::LayerDiffSummary) -> usize {
    layer.added_shapes + layer.removed_shapes + layer.modified_shapes
}

fn total_changes(report: &LayoutDiffReport) -> usize {
    report.summary.added_shapes + report.summary.removed_shapes + report.summary.modified_shapes
}

fn comparison_status_label(report: &LayoutDiffReport) -> &'static str {
    if total_changes(report) == 0 {
        "clean"
    } else {
        "differences found"
    }
}

fn comparison_status_tone(report: &LayoutDiffReport) -> Tone {
    if total_changes(report) == 0 {
        Tone::Success
    } else if report.summary.removed_shapes > 0 {
        Tone::Danger
    } else {
        Tone::Warning
    }
}

fn delta_tone(candidate: usize, baseline: usize) -> Tone {
    if candidate == baseline {
        Tone::Neutral
    } else if candidate > baseline {
        Tone::Success
    } else {
        Tone::Danger
    }
}

fn signed_delta(candidate: usize, baseline: usize) -> String {
    if candidate == baseline {
        "0".to_string()
    } else if candidate > baseline {
        format!("+{}", candidate - baseline)
    } else {
        format!("-{}", baseline - candidate)
    }
}

fn bounds_label(bounds: Option<Rect>) -> String {
    bounds
        .map(|bounds| format!("{} x {} dbu", bounds.width().abs(), bounds.height().abs()))
        .unwrap_or_else(|| "empty".to_string())
}

fn bounds_delta_label(baseline: Option<Rect>, candidate: Option<Rect>) -> String {
    match (baseline, candidate) {
        (Some(baseline), Some(candidate)) => format!(
            "{} width, {} height",
            signed_coord_delta(candidate.width().abs(), baseline.width().abs()),
            signed_coord_delta(candidate.height().abs(), baseline.height().abs())
        ),
        (None, Some(_)) => "candidate adds geometry".to_string(),
        (Some(_), None) => "candidate removes geometry".to_string(),
        (None, None) => "both empty".to_string(),
    }
}

fn signed_coord_delta(candidate: i64, baseline: i64) -> String {
    if candidate == baseline {
        "0".to_string()
    } else if candidate > baseline {
        format!("+{}", candidate - baseline)
    } else {
        format!("-{}", baseline - candidate)
    }
}

fn review_guidance(report: &LayoutDiffReport, disposition: ReviewDisposition) -> String {
    match disposition {
        ReviewDisposition::NeedsReview if total_changes(report) == 0 => {
            "No differences are currently blocking review.".to_string()
        }
        ReviewDisposition::NeedsReview => format!(
            "Review {} shape-level changes before approving the candidate.",
            total_changes(report)
        ),
        ReviewDisposition::Approved => {
            "This comparison is marked approved for the current source pair.".to_string()
        }
        ReviewDisposition::ChangesRequested => {
            "This comparison is marked for candidate follow-up.".to_string()
        }
    }
}

fn shape_change_tone(kind: ShapeChangeKind) -> Tone {
    match kind {
        ShapeChangeKind::Added => Tone::Success,
        ShapeChangeKind::Removed => Tone::Danger,
        ShapeChangeKind::Modified => Tone::Warning,
    }
}

fn layout_diff_operad_view_height(width: f32, metric_count: usize, row_counts: &[usize]) -> f32 {
    let mut height = OPERAD_HEADER_HEIGHT + OPERAD_GAP;
    height += layout_diff_operad_metric_grid_height(width, metric_count) + OPERAD_GAP;
    for row_count in row_counts {
        height += layout_diff_operad_section_height(*row_count) + OPERAD_GAP;
    }
    height + OPERAD_PAD
}

fn layout_diff_operad_metric_columns(width: f32) -> usize {
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

fn layout_diff_operad_metric_grid_height(width: f32, metric_count: usize) -> f32 {
    let columns = layout_diff_operad_metric_columns(width).max(1);
    let rows = metric_count.div_ceil(columns).max(1);
    rows as f32 * OPERAD_METRIC_HEIGHT
}

fn layout_diff_operad_section_height(row_count: usize) -> f32 {
    OPERAD_PAD * 2.0
        + OPERAD_SECTION_TITLE_HEIGHT
        + if row_count == 0 {
            OPERAD_EMPTY_ROW_HEIGHT
        } else {
            row_count as f32 * OPERAD_ROW_HEIGHT
        }
}

fn add_layout_diff_operad_header(
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
            "layout_diff.header",
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
    add_layout_diff_operad_text(
        document,
        header,
        "layout_diff.header.eyebrow",
        eyebrow,
        layout_diff_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(146, 154, 162, 255)),
        16.0,
    );
    add_layout_diff_operad_text(
        document,
        header,
        "layout_diff.header.title",
        title,
        layout_diff_operad_text_style(24.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        30.0,
    );
    add_layout_diff_operad_text(
        document,
        header,
        "layout_diff.header.detail",
        detail,
        layout_diff_operad_text_style(14.0, FontWeight::NORMAL, ColorRgba::new(178, 185, 194, 255)),
        20.0,
    );
    add_layout_diff_operad_text(
        document,
        header,
        "layout_diff.header.meta",
        meta,
        layout_diff_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(112, 183, 239, 255)),
        18.0,
    );
}

fn add_layout_diff_operad_metric_grid(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    metrics: &[LayoutDiffMetricTile],
) {
    let columns = layout_diff_operad_metric_columns(width);
    let grid_height = layout_diff_operad_metric_grid_height(width, metrics.len());
    let grid = document.add_child(
        parent,
        UiNode::container(
            "layout_diff.metrics",
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
                format!("layout_diff.metrics.row.{row_index}"),
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
            add_layout_diff_operad_metric_tile(
                document,
                row,
                &format!("layout_diff.metrics.{row_index}.{column}"),
                tile_width - 6.0,
                metric,
            );
        }
    }
}

fn add_layout_diff_operad_metric_tile(
    document: &mut UiDocument,
    parent: UiNodeId,
    name: &str,
    width: f32,
    metric: &LayoutDiffMetricTile,
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
                layout_diff_operad_tone_color(metric.tone),
                1.0,
            )),
            6.0,
        )),
    );
    add_layout_diff_operad_text(
        document,
        tile,
        &format!("{name}.label"),
        &metric.label,
        layout_diff_operad_text_style(12.0, FontWeight::BOLD, ColorRgba::new(158, 166, 174, 255)),
        18.0,
    );
    add_layout_diff_operad_text(
        document,
        tile,
        &format!("{name}.value"),
        &metric.value,
        layout_diff_operad_text_style(20.0, FontWeight::BOLD, ColorRgba::new(239, 243, 247, 255)),
        26.0,
    );
    add_layout_diff_operad_text(
        document,
        tile,
        &format!("{name}.detail"),
        truncate_middle(&metric.detail, 52),
        layout_diff_operad_text_style(
            12.0,
            FontWeight::NORMAL,
            layout_diff_operad_tone_color(metric.tone),
        ),
        18.0,
    );
}

fn add_layout_diff_operad_section(
    document: &mut UiDocument,
    parent: UiNodeId,
    width: f32,
    name: &str,
    title: &str,
    empty: &str,
    rows: &[LayoutDiffOperadRow],
) {
    let height = layout_diff_operad_section_height(rows.len());
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
    add_layout_diff_operad_text(
        document,
        section,
        &format!("{name}.title"),
        title,
        layout_diff_operad_text_style(15.0, FontWeight::BOLD, ColorRgba::new(242, 246, 250, 255)),
        OPERAD_SECTION_TITLE_HEIGHT,
    );
    if rows.is_empty() {
        add_layout_diff_operad_empty_row(document, section, name, empty);
    } else {
        let row_width = (width - OPERAD_PAD * 2.0).max(240.0);
        for (index, row) in rows.iter().enumerate() {
            add_layout_diff_operad_data_row(document, section, name, index, row_width, row);
        }
    }
}

fn add_layout_diff_operad_empty_row(
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
    add_layout_diff_operad_text(
        document,
        row,
        &format!("{name}.empty.label"),
        label,
        layout_diff_operad_text_style(13.0, FontWeight::NORMAL, ColorRgba::new(154, 163, 172, 255)),
        24.0,
    );
}

fn add_layout_diff_operad_data_row(
    document: &mut UiDocument,
    parent: UiNodeId,
    section_name: &str,
    index: usize,
    row_width: f32,
    row: &LayoutDiffOperadRow,
) {
    let row_name = row
        .action_name
        .clone()
        .unwrap_or_else(|| format!("{section_name}.row.{index}"));
    let stroke_color = if row.selected {
        layout_diff_operad_tone_color(Tone::Info)
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
            layout_diff_operad_tone_color(row.tone),
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
    add_layout_diff_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.title"),
        truncate_middle(&row.title, 72),
        layout_diff_operad_text_style(14.0, FontWeight::BOLD, ColorRgba::new(232, 237, 242, 255)),
        21.0,
    );
    add_layout_diff_operad_text(
        document,
        text_column,
        &format!("{section_name}.row.{index}.detail"),
        truncate_middle(&row.detail, 108),
        layout_diff_operad_text_style(12.0, FontWeight::NORMAL, ColorRgba::new(162, 171, 180, 255)),
        19.0,
    );
}

fn add_layout_diff_operad_spacer(document: &mut UiDocument, parent: UiNodeId, height: f32) {
    document.add_child(
        parent,
        UiNode::container(
            format!("layout_diff.spacer.{}", document.node_count()),
            UiNodeStyle {
                layout: layout::with_size(layout::row(), layout::percent(1.0), layout::px(height)),
                ..Default::default()
            },
        ),
    );
}

fn add_layout_diff_operad_text(
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

fn layout_diff_operad_text_style(
    font_size: f32,
    weight: FontWeight,
    color: ColorRgba,
) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size + 4.0,
        weight,
        color,
        wrap: TextWrap::None,
        ..Default::default()
    }
}

fn layout_diff_operad_tone_color(tone: Tone) -> ColorRgba {
    let color = tone.color();
    ColorRgba::new(color.r(), color.g(), color.b(), color.a())
}

fn layout_diff_operad_row(
    title: impl Into<String>,
    detail: impl Into<String>,
    tone: Tone,
    action_name: Option<String>,
    selected: bool,
) -> LayoutDiffOperadRow {
    LayoutDiffOperadRow {
        title: title.into(),
        detail: detail.into(),
        tone,
        action_name,
        selected,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_diff_operad_view_audits_common_widths() {
        let current = Document::hierarchy_demo();
        let panel = LayoutDiffPanel {
            candidate: DiffSource::Current,
            ..Default::default()
        };
        let report = panel.report(&current);
        for width in [360.0, 760.0, 1200.0] {
            let mut view = panel.build_operad_view(width, &report);
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
    fn layout_diff_operad_actions_update_panel_state() {
        let current = Document::hierarchy_demo();
        let mut panel = LayoutDiffPanel::default();
        let report = panel.report(&current);

        assert!(panel.handle_operad_action(
            &format!(
                "{OPERAD_ACTION_SET_BASELINE}{}|test",
                DiffSource::Empty.slug()
            ),
            &report,
        ));
        assert_eq!(panel.baseline, DiffSource::Empty);
        assert_eq!(panel.review_state, ReviewDisposition::NeedsReview);

        assert!(panel.handle_operad_action(
            &format!(
                "{OPERAD_ACTION_SET_CHANGE_FILTER}{}|test",
                ChangeKindFilter::Removed.slug()
            ),
            &report,
        ));
        assert_eq!(panel.change_filter, ChangeKindFilter::Removed);
        assert_eq!(panel.change_page, 0);

        assert!(
            panel.handle_operad_action(&format!("{OPERAD_ACTION_SET_PAGE_SIZE}25|test"), &report)
        );
        assert_eq!(panel.change_page_size, 25);

        assert!(panel.handle_operad_action(
            &format!(
                "{OPERAD_ACTION_SET_REVIEW}{}|test",
                ReviewDisposition::Approved.slug()
            ),
            &report,
        ));
        assert_eq!(panel.review_state, ReviewDisposition::Approved);

        assert!(panel.changed_only);
        assert!(panel.handle_operad_action(OPERAD_ACTION_TOGGLE_CHANGED_ONLY, &report));
        assert!(!panel.changed_only);
    }
}
