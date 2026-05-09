use eframe::egui::{self, Color32};
use geometry_core::Rect;
use layout_model::{
    Document,
    layout_diff::{LayoutDiffReport, ShapeChangeKind, diff_documents},
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct LayoutDiffPanel {
    baseline: DiffSource,
    candidate: DiffSource,
    changed_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffSource {
    Current,
    Demo,
    Hierarchy,
    Empty,
}

impl Default for LayoutDiffPanel {
    fn default() -> Self {
        Self {
            baseline: DiffSource::Demo,
            candidate: DiffSource::Current,
            changed_only: true,
        }
    }
}

impl LayoutDiffPanel {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        let report = self.report(current);

        egui::ScrollArea::vertical()
            .id_salt("layout_diff_review")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Design review",
                    "Layout Diff Review",
                    &format!("{} -> {}", self.baseline.label(), self.candidate.label()),
                    |ui| {
                        ui.label("From");
                        source_picker(ui, "layout_diff_baseline", &mut self.baseline);
                        ui.label("To");
                        source_picker(ui, "layout_diff_candidate", &mut self.candidate);
                        ui.checkbox(&mut self.changed_only, "Changed layers only");
                    },
                );

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
                changes_ui(ui, &report);
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, current: &Document) {
        let report = self.report(current);

        ui_chrome::section_label(ui, "Layout Diff");
        ui.label(format!("Baseline: {}", report.baseline_name));
        ui.label(format!("Candidate: {}", report.candidate_name));
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
    egui::Grid::new("layout_diff_summary_grid")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            summary_value(ui, "Baseline shapes", report.summary.baseline_shapes);
            summary_value(ui, "Candidate shapes", report.summary.candidate_shapes);
            summary_value(ui, "Added", report.summary.added_shapes);
            summary_value(ui, "Removed", report.summary.removed_shapes);
            summary_value(ui, "Modified", report.summary.modified_shapes);
        });
}

fn summary_value(ui: &mut egui::Ui, label: &str, value: usize) {
    ui.strong(label);
    ui.label(value.to_string());
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
                    for layer in &report.layers {
                        let delta =
                            layer.added_shapes + layer.removed_shapes + layer.modified_shapes;
                        if changed_only && delta == 0 {
                            continue;
                        }
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

fn changes_ui(ui: &mut egui::Ui, report: &LayoutDiffReport) {
    ui_chrome::section_label(ui, "Review Changes");
    if report.changes.is_empty() {
        ui_chrome::empty_state(ui, "No shape-level differences");
        return;
    }

    egui::ScrollArea::horizontal()
        .id_salt("layout_diff_changes_horizontal")
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
                    for change in report.changes.iter().take(200) {
                        ui.colored_label(change_color(change.kind), change.kind.label());
                        ui.label(format!("#{}", change.id.0));
                        ui.label(format!("{} {}", change.layer.0, change.layer_name));
                        ui.label(format!(
                            "{} x {}",
                            change.bounds.width().abs(),
                            change.bounds.height().abs()
                        ));
                        ui.label(&change.detail);
                        ui.end_row();
                    }
                });
        });
    if report.changes.len() > 200 {
        ui_chrome::muted(
            ui,
            format!("showing first 200 of {} changes", report.changes.len()),
        );
    }
}

fn change_color(kind: ShapeChangeKind) -> Color32 {
    match kind {
        ShapeChangeKind::Added => Color32::from_rgb(105, 190, 120),
        ShapeChangeKind::Removed => Color32::from_rgb(245, 98, 98),
        ShapeChangeKind::Modified => Color32::from_rgb(230, 180, 80),
    }
}
