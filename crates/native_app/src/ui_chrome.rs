use eframe::egui::{self, Color32, Margin, RichText, Stroke, StrokeKind, Vec2, vec2};

pub(crate) const NAV_WIDTH: f32 = 196.0;
pub(crate) const INSPECTOR_WIDTH: f32 = 268.0;
pub(crate) const LAYERS_WIDTH: f32 = 228.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    pub(crate) fn color(self) -> Color32 {
        match self {
            Self::Neutral => Color32::from_rgb(145, 152, 160),
            Self::Info => Color32::from_rgb(93, 168, 232),
            Self::Success => Color32::from_rgb(93, 184, 132),
            Self::Warning => Color32::from_rgb(220, 176, 72),
            Self::Danger => Color32::from_rgb(226, 96, 96),
        }
    }
}

pub(crate) fn apply_workspace_style(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if dark {
        visuals.panel_fill = Color32::from_rgb(20, 23, 26);
        visuals.window_fill = Color32::from_rgb(24, 27, 30);
        visuals.extreme_bg_color = Color32::from_rgb(12, 14, 16);
        visuals.faint_bg_color = Color32::from_rgb(31, 36, 40);
        visuals.selection.bg_fill = Color32::from_rgb(43, 94, 137);
    } else {
        visuals.panel_fill = Color32::from_rgb(247, 248, 250);
        visuals.window_fill = Color32::from_rgb(255, 255, 255);
        visuals.extreme_bg_color = Color32::from_rgb(238, 241, 245);
        visuals.faint_bg_color = Color32::from_rgb(230, 235, 240);
        visuals.selection.bg_fill = Color32::from_rgb(178, 212, 238);
    }

    visuals.widgets.noninteractive.corner_radius = 5.into();
    visuals.widgets.inactive.corner_radius = 5.into();
    visuals.widgets.hovered.corner_radius = 5.into();
    visuals.widgets.active.corner_radius = 5.into();
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(9.0, 4.0);
    style.spacing.indent = 12.0;
    style.spacing.window_margin = Margin::symmetric(10, 10);
    ctx.set_style(style);
}

pub(crate) fn module_header(
    ui: &mut egui::Ui,
    eyebrow: &str,
    title: &str,
    detail: &str,
    actions: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal_wrapped(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(eyebrow.to_uppercase())
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.heading(title);
            if !detail.is_empty() {
                ui.label(RichText::new(detail).color(ui.visuals().weak_text_color()));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.horizontal(actions);
        });
    });
    ui.add_space(4.0);
}

pub(crate) fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(label).strong());
    ui.add_space(2.0);
}

pub(crate) fn muted(ui: &mut egui::Ui, text: impl ToString) {
    ui.label(RichText::new(text.to_string()).color(ui.visuals().weak_text_color()));
}

pub(crate) fn metric_tile(ui: &mut egui::Ui, label: &str, value: impl ToString, detail: &str) {
    metric_tile_tone(ui, label, value, detail, Tone::Neutral);
}

pub(crate) fn metric_tile_tone(
    ui: &mut egui::Ui,
    label: &str,
    value: impl ToString,
    detail: &str,
    tone: Tone,
) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(132.0);
            ui.set_max_width(210.0);
            ui.label(
                RichText::new(label)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(RichText::new(value.to_string()).strong().size(18.0));
            if !detail.is_empty() {
                ui.label(RichText::new(detail).small().color(tone.color()));
            }
        });
}

pub(crate) fn status_pill(ui: &mut egui::Ui, label: &str, tone: Tone) {
    let color = tone.color();
    let fill = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 32);
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, color))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().strong().color(color));
        });
}

pub(crate) fn empty_state(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_height(44.0);
            ui.centered_and_justified(|ui| {
                muted(ui, text);
            });
        });
}

pub(crate) fn plot_background(ui: &egui::Ui, rect: egui::Rect) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 5.0, ui.visuals().extreme_bg_color);
    painter.rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        StrokeKind::Inside,
    );
}

pub(crate) fn stable_plot_size(ui: &egui::Ui, height: f32) -> Vec2 {
    vec2(ui.available_width().clamp(260.0, 640.0), height)
}
