use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, vec2,
};
use layout_model::cross_section::{
    CrossSectionProcess, CrossSectionSegment, CrossSectionSnapshot, MaterialId, ProcessStepKind,
};

use crate::ui_chrome;

pub(crate) struct CrossSectionPanel {
    process: CrossSectionProcess,
    snapshots: Vec<CrossSectionSnapshot>,
    selected_step: usize,
}

impl CrossSectionPanel {
    pub(crate) fn sample() -> Self {
        let process = CrossSectionProcess::sample_sequence();
        let snapshots = process.simulate();
        Self {
            process,
            snapshots,
            selected_step: 0,
        }
    }

    pub(crate) fn context_ui(&mut self, ui: &mut egui::Ui) {
        self.clamp_selection();
        ui_chrome::section_label(ui, "Process Cross-Section");
        let Some(snapshot) = self.selected_snapshot() else {
            ui_chrome::empty_state(ui, "No process sequence loaded");
            return;
        };

        let selected_step_index = snapshot.step_index;
        let selected_title = snapshot.title.clone();
        let selected_detail = snapshot.detail.clone();

        ui.label(RichText::new(selected_title).strong());
        ui.label(selected_detail);
        ui.separator();

        ui_chrome::section_label(ui, "Step Sequence");
        egui::ScrollArea::vertical()
            .id_salt("cross_section_step_list")
            .max_height(220.0)
            .show(ui, |ui| {
                if ui
                    .selectable_label(self.selected_step == 0, "0 - Starting substrate")
                    .clicked()
                {
                    self.selected_step = 0;
                }
                for (index, step) in self.process.steps.iter().enumerate() {
                    let step_index = index + 1;
                    let selected = self.selected_step == step_index;
                    if ui
                        .selectable_label(selected, format!("{step_index} - {}", step.name))
                        .clicked()
                    {
                        self.selected_step = step_index;
                    }
                }
            });

        ui.separator();
        ui_chrome::section_label(ui, "Selected Step");
        if selected_step_index == 0 {
            ui.label("Initial substrate stack");
        } else if let Some(step) = self.process.steps.get(selected_step_index - 1) {
            match &step.kind {
                ProcessStepKind::Deposit {
                    material,
                    thickness_um,
                } => {
                    ui.label(format!(
                        "Deposit {} by {:.2} um",
                        self.material_name(material),
                        thickness_um
                    ));
                }
                ProcessStepKind::Etch { material, depth_um } => {
                    ui.label(format!(
                        "Etch {} by {:.2} um in mask openings",
                        self.material_name(material),
                        depth_um
                    ));
                }
                ProcessStepKind::Pattern { openings } => {
                    ui.label(format!("Pattern {} mask opening(s)", openings.len()));
                    for opening in openings {
                        ui.label(format!(
                            "{:.2} to {:.2} um",
                            opening.start_um, opening.end_um
                        ));
                    }
                }
            }
        }
    }

    pub(crate) fn layer_stack_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Layer Coloring");
        for material in &self.process.materials {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    2.0,
                    Color32::from_rgb(
                        material.color_rgb[0],
                        material.color_rgb[1],
                        material.color_rgb[2],
                    ),
                );
                ui.label(&material.name);
            });
        }

        ui.separator();
        ui_chrome::section_label(ui, "Mask Openings");
        if let Some(snapshot) = self.selected_snapshot() {
            if snapshot.active_mask.is_empty() {
                ui.label("No active openings");
            } else {
                for opening in &snapshot.active_mask {
                    ui.label(format!(
                        "{:.2} to {:.2} um",
                        opening.start_um, opening.end_um
                    ));
                }
            }
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status: &mut String) {
        self.clamp_selection();
        egui::ScrollArea::vertical()
            .id_salt("cross_section_dashboard_scroll")
            .show(ui, |ui| {
                ui_chrome::module_header(
                    ui,
                    "Process integration",
                    "3D/2D Process Cross-Section",
                    "",
                    |ui| {
                        ui.label("Step");
                        let last = self.snapshots.len().saturating_sub(1);
                        let previous = self.selected_step;
                        ui.add(
                            egui::Slider::new(&mut self.selected_step, 0..=last).show_value(true),
                        );
                        if self.selected_step != previous {
                            *status = format!("cross-section step {}", self.selected_step);
                        }
                    },
                );

                let Some(snapshot) = self.selected_snapshot() else {
                    ui_chrome::empty_state(ui, "No cross-section snapshots loaded");
                    return;
                };

                ui.horizontal_wrapped(|ui| {
                    process_metric_ui(ui, "Sequence steps", self.process.steps.len().to_string());
                    process_metric_ui(ui, "Window", format!("{:.1} um", self.process.width_um));
                    process_metric_ui(ui, "Segments", snapshot.segments.len().to_string());
                    process_metric_ui(
                        ui,
                        "Active step",
                        format!("{} / {}", snapshot.step_index, self.snapshots.len() - 1),
                    );
                });

                ui.separator();
                ui.label(RichText::new(&snapshot.title).strong());
                ui.label(&snapshot.detail);
                ui.add_space(6.0);
                draw_cross_section(ui, &self.process, snapshot);
            });
    }

    fn selected_snapshot(&self) -> Option<&CrossSectionSnapshot> {
        self.snapshots.get(self.selected_step)
    }

    fn clamp_selection(&mut self) {
        self.selected_step = self
            .selected_step
            .min(self.snapshots.len().saturating_sub(1));
    }

    fn material_name(&self, id: &MaterialId) -> String {
        self.process
            .material(id)
            .map(|material| material.name.clone())
            .unwrap_or_else(|| id.as_str().to_string())
    }
}

fn process_metric_ui(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(130.0);
            ui.label(RichText::new(value).strong());
            ui.label(RichText::new(label).small().color(Color32::GRAY));
        });
}

fn draw_cross_section(
    ui: &mut egui::Ui,
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
) {
    let desired = vec2(ui.available_width(), 430.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 28));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::from_rgb(68, 76, 84)),
        StrokeKind::Outside,
    );

    let bounds = content_bounds(rect);
    let max_y = snapshot
        .segments
        .iter()
        .map(|segment| segment.y1_um)
        .fold(process.substrate_thickness_um.max(1.0), f32::max);
    draw_mask_overlay(&painter, process, snapshot, bounds, max_y);
    for segment in &snapshot.segments {
        draw_segment(&painter, process, segment, bounds, max_y);
    }
    draw_axes(&painter, bounds, process.width_um, max_y);
}

fn content_bounds(rect: Rect) -> Rect {
    Rect::from_min_max(rect.min + vec2(44.0, 24.0), rect.max - vec2(18.0, 42.0))
}

fn draw_segment(
    painter: &egui::Painter,
    process: &CrossSectionProcess,
    segment: &CrossSectionSegment,
    bounds: Rect,
    max_y: f32,
) {
    let x0 = bounds.left() + bounds.width() * segment.x0_um / process.width_um.max(0.1);
    let x1 = bounds.left() + bounds.width() * segment.x1_um / process.width_um.max(0.1);
    let y0 = bounds.bottom() - bounds.height() * segment.y0_um / max_y.max(0.1);
    let y1 = bounds.bottom() - bounds.height() * segment.y1_um / max_y.max(0.1);
    let rect = Rect::from_min_max(Pos2::new(x0, y1), Pos2::new(x1, y0));
    let color = process
        .material(&segment.material)
        .map(|material| {
            Color32::from_rgb(
                material.color_rgb[0],
                material.color_rgb[1],
                material.color_rgb[2],
            )
        })
        .unwrap_or(Color32::LIGHT_GRAY);
    painter.rect_filled(rect, 0.0, color);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(0.5, Color32::from_black_alpha(80)),
        StrokeKind::Inside,
    );
}

fn draw_mask_overlay(
    painter: &egui::Painter,
    process: &CrossSectionProcess,
    snapshot: &CrossSectionSnapshot,
    bounds: Rect,
    max_y: f32,
) {
    let top = bounds.bottom() - bounds.height() * max_y / max_y.max(0.1);
    for opening in &snapshot.active_mask {
        let x0 = bounds.left() + bounds.width() * opening.start_um / process.width_um.max(0.1);
        let x1 = bounds.left() + bounds.width() * opening.end_um / process.width_um.max(0.1);
        let rect = Rect::from_min_max(Pos2::new(x0, top - 18.0), Pos2::new(x1, top - 6.0));
        painter.rect_filled(
            rect,
            1.0,
            Color32::from_rgba_unmultiplied(250, 210, 80, 135),
        );
    }
    painter.text(
        Pos2::new(bounds.left(), top - 22.0),
        Align2::LEFT_BOTTOM,
        "active mask openings",
        FontId::proportional(12.0),
        Color32::from_rgb(214, 206, 178),
    );
}

fn draw_axes(painter: &egui::Painter, bounds: Rect, width_um: f32, max_y: f32) {
    painter.line_segment(
        [bounds.left_bottom(), bounds.right_bottom()],
        Stroke::new(1.0, Color32::from_rgb(160, 168, 176)),
    );
    painter.line_segment(
        [bounds.left_bottom(), bounds.left_top()],
        Stroke::new(1.0, Color32::from_rgb(160, 168, 176)),
    );
    painter.text(
        bounds.left_bottom() + Vec2::new(0.0, 18.0),
        Align2::LEFT_CENTER,
        "0 um",
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
    painter.text(
        bounds.right_bottom() + Vec2::new(0.0, 18.0),
        Align2::RIGHT_CENTER,
        format!("{width_um:.1} um"),
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
    painter.text(
        bounds.left_top() + Vec2::new(-8.0, 0.0),
        Align2::RIGHT_CENTER,
        format!("{max_y:.1} um"),
        FontId::proportional(12.0),
        Color32::from_rgb(190, 196, 204),
    );
}
