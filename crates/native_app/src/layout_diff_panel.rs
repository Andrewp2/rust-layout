use eframe::egui::{self, Color32};
use geometry_core::Rect;
use layout_model::{
    Document,
    layout_diff::{LayoutDiffReport, ShapeChangeKind, diff_documents},
};

use crate::ui_chrome::{self, Tone};

pub(crate) struct LayoutDiffPanel {
    baseline: DiffBaseline,
    changed_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffBaseline {
    Demo,
    Hierarchy,
    Blank,
}

impl Default for LayoutDiffPanel {
    fn default() -> Self {
        Self {
            baseline: DiffBaseline::Demo,
            changed_only: true,
        }
    }
}

impl LayoutDiffPanel {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, candidate: &Document) {
        let baseline = self.baseline.document();
        let report = diff_documents(
            self.baseline.label(),
            &baseline,
            "current workspace",
            candidate,
        );

        egui::ScrollArea::vertical()
            .id_salt("layout_diff_review")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Design review",
                    "Layout Diff Review",
                    &format!("{} vs current", self.baseline.label()),
                    |ui| {
                        self.baseline_picker(ui);
                        ui.checkbox(&mut self.changed_only, "Changed layers only");
                    },
                );

                summary_ui(ui, &report);

                ui.separator();
                ui.columns(2, |columns| {
                    bounds_ui(&mut columns[0], "Baseline bounds", report.baseline_bounds);
                    bounds_ui(&mut columns[1], "Candidate bounds", report.candidate_bounds);
                });

                ui.separator();
                layers_ui(ui, &report, self.changed_only);

                ui.separator();
                changes_ui(ui, &report);
            });
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui, candidate: &Document) {
        let baseline = self.baseline.document();
        let report = diff_documents(
            self.baseline.label(),
            &baseline,
            "current workspace",
            candidate,
        );

        ui_chrome::section_label(ui, "Layout Diff");
        self.baseline_picker(ui);
        ui.separator();
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

    fn baseline_picker(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("layout_diff_baseline")
            .selected_text(self.baseline.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.baseline,
                    DiffBaseline::Demo,
                    DiffBaseline::Demo.label(),
                );
                ui.selectable_value(
                    &mut self.baseline,
                    DiffBaseline::Hierarchy,
                    DiffBaseline::Hierarchy.label(),
                );
                ui.selectable_value(
                    &mut self.baseline,
                    DiffBaseline::Blank,
                    DiffBaseline::Blank.label(),
                );
            });
    }
}

impl DiffBaseline {
    fn label(self) -> &'static str {
        match self {
            Self::Demo => "demo layout",
            Self::Hierarchy => "hierarchy demo",
            Self::Blank => "blank layout",
        }
    }

    fn document(self) -> Document {
        match self {
            Self::Demo => Document::demo(),
            Self::Hierarchy => Document::hierarchy_demo(),
            Self::Blank => Document::new("blank layout"),
        }
    }
}

fn summary_ui(ui: &mut egui::Ui, report: &LayoutDiffReport) {
    ui.horizontal_wrapped(|ui| {
        ui_chrome::metric_tile(ui, "Baseline", report.summary.baseline_shapes, "shapes");
        ui_chrome::metric_tile(ui, "Candidate", report.summary.candidate_shapes, "shapes");
        ui_chrome::metric_tile_tone(
            ui,
            "Added",
            report.summary.added_shapes,
            "shapes",
            Tone::Success,
        );
        ui_chrome::metric_tile_tone(
            ui,
            "Removed",
            report.summary.removed_shapes,
            "shapes",
            Tone::Danger,
        );
        ui_chrome::metric_tile_tone(
            ui,
            "Modified",
            report.summary.modified_shapes,
            "shapes",
            Tone::Warning,
        );
    });
}

fn bounds_ui(ui: &mut egui::Ui, label: &str, bounds: Option<Rect>) {
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
        ui_chrome::empty_state(ui, "No geometry");
    }
}

fn layers_ui(ui: &mut egui::Ui, report: &LayoutDiffReport, changed_only: bool) {
    ui_chrome::section_label(ui, "Layer Summary");
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
                let delta = layer.added_shapes + layer.removed_shapes + layer.modified_shapes;
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
}

fn changes_ui(ui: &mut egui::Ui, report: &LayoutDiffReport) {
    ui_chrome::section_label(ui, "Review Changes");
    if report.changes.is_empty() {
        ui_chrome::empty_state(ui, "No shape-level differences");
        return;
    }

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
