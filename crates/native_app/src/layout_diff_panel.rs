use eframe::egui::{self, Color32};
use geometry_core::Rect;
use layout_model::{
    Document,
    layout_diff::{LayoutDiffReport, ShapeChange, ShapeChangeKind, diff_documents},
};

use crate::ui_chrome::{self, Tone};

const DEFAULT_CHANGE_PAGE_SIZE: usize = 50;
const CHANGE_PAGE_SIZE_OPTIONS: [usize; 4] = [25, 50, 100, 200];

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

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, current: &Document) {
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
    fn label(self) -> &'static str {
        match self {
            Self::Current => "Current workspace",
            Self::Demo => "Demo layout",
            Self::Hierarchy => "Hierarchy demo",
            Self::Empty => "Empty layout",
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
    fn label(self) -> &'static str {
        match self {
            Self::All => "All changes",
            Self::Added => "Added only",
            Self::Removed => "Removed only",
            Self::Modified => "Modified only",
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
    fn label(self) -> &'static str {
        match self {
            Self::NeedsReview => "needs review",
            Self::Approved => "approved",
            Self::ChangesRequested => "changes requested",
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
