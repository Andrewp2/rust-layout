use eframe::egui::{self, Color32, Margin, RichText, Stroke, StrokeKind, Vec2, vec2};

pub(crate) const NAV_WIDTH: f32 = 108.0;
pub(crate) const INSPECTOR_WIDTH: f32 = 268.0;
pub(crate) const INSPECTOR_MIN_WIDTH: f32 = 220.0;
pub(crate) const INSPECTOR_MAX_WIDTH: f32 = 360.0;
pub(crate) const LAYERS_WIDTH: f32 = 228.0;
pub(crate) const LAYERS_MIN_WIDTH: f32 = 200.0;
pub(crate) const LAYERS_MAX_WIDTH: f32 = 320.0;
pub(crate) const INLINE_SIDE_PANEL_MIN_WIDTH: f32 = 980.0;
pub(crate) const COMPACT_MENU_WIDTH: f32 = 720.0;
pub(crate) const COMPACT_TOOL_STRIP_WIDTH: f32 = 620.0;

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
    let compact = ui.available_width() < 640.0;
    let title_block = |ui: &mut egui::Ui| {
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
    };

    if compact {
        ui.vertical(|ui| {
            title_block(ui);
            ui.horizontal_wrapped(actions);
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            title_block(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.horizontal(actions);
            });
        });
    }
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

pub(crate) fn metric_tiles(ui: &mut egui::Ui, metrics: &[(&str, String, &str, Tone)]) {
    let available_width = ui.available_width().min(ui.ctx().content_rect().width());
    if available_width < 360.0 {
        ui.vertical(|ui| {
            for (label, value, detail, tone) in metrics {
                metric_tile_tone(ui, label, value, detail, *tone);
            }
        });
    } else if available_width < 900.0 {
        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                for (index, (label, value, detail, tone)) in metrics.iter().enumerate() {
                    metric_tile_tone(ui, label, value, detail, *tone);
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
    } else {
        ui.horizontal_wrapped(|ui| {
            for (label, value, detail, tone) in metrics {
                metric_tile_tone(ui, label, value, detail, *tone);
            }
        });
    }
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
            let width = ui.available_width().clamp(108.0, 260.0);
            ui.set_min_width(width.min(180.0));
            ui.set_max_width(width);
            ui.vertical(|ui| {
                ui.set_max_width(width);
                ui.add(
                    egui::Label::new(
                        RichText::new(label)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    )
                    .wrap(),
                );
                ui.add(
                    egui::Label::new(RichText::new(value.to_string()).strong().size(18.0)).wrap(),
                );
                if !detail.is_empty() {
                    ui.add(
                        egui::Label::new(RichText::new(detail).small().color(tone.color())).wrap(),
                    );
                }
            });
        });
}

pub(crate) fn status_pill(ui: &mut egui::Ui, label: &str, tone: Tone) {
    let fill = tone.color();
    let text = readable_text_color(fill);
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, fill))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().strong().color(text));
        });
}

pub(crate) fn readable_text_color(background: Color32) -> Color32 {
    let black = Color32::BLACK;
    let white = Color32::WHITE;
    if contrast_ratio(black, background) >= contrast_ratio(white, background) {
        black
    } else {
        white
    }
}

fn contrast_ratio(foreground: Color32, background: Color32) -> f32 {
    let foreground_luminance = relative_luminance(foreground);
    let background_luminance = relative_luminance(background);
    let lighter = foreground_luminance.max(background_luminance);
    let darker = foreground_luminance.min(background_luminance);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Color32) -> f32 {
    let channel = |value: u8| {
        let normalized = value as f32 / 255.0;
        if normalized <= 0.04045 {
            normalized / 12.92
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_status_pills_have_readable_text_contrast() {
        for tone in [
            Tone::Neutral,
            Tone::Info,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
        ] {
            let background = tone.color();
            let text = readable_text_color(background);
            assert!(
                contrast_ratio(text, background) >= 4.5,
                "{tone:?} contrast was {}",
                contrast_ratio(text, background)
            );
        }
    }
}
