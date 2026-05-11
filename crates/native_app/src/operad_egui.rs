use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Shape, Stroke};
use operad::{
    CanvasContent, ColorRgba, FontFamily, PaintEffectKind, PaintKind, PaintPath, PaintRect,
    PaintText, PaintTransform, PathVerb, StrokeAlignment, TextHorizontalAlign, TextVerticalAlign,
    UiDocument, UiPoint, UiRect,
};

pub(crate) fn paint_document_at(ui: &egui::Ui, document: &UiDocument, rect: Rect) {
    let outer = UiRect::new(0.0, 0.0, rect.width(), rect.height());
    let painter = ui.painter_at(rect);
    for item in document.paint_list().items {
        let Some(local_clip) = item.clip_rect.intersection(outer) else {
            continue;
        };
        if local_clip.width <= f32::EPSILON || local_clip.height <= f32::EPSILON {
            continue;
        }
        let clip = egui_rect(offset_rect(local_clip, rect.min));
        let item_rect = egui_rect(offset_rect(
            transform_rect(item.rect, item.transform),
            rect.min,
        ));
        match &item.kind {
            PaintKind::Rect {
                fill,
                stroke,
                corner_radius,
            } => {
                let node_painter = painter.with_clip_rect(clip);
                if fill.a > 0 {
                    node_painter.rect_filled(
                        item_rect,
                        *corner_radius,
                        egui_color(*fill, item.opacity),
                    );
                }
                if let Some(stroke) = *stroke {
                    node_painter.rect_stroke(
                        item_rect,
                        *corner_radius,
                        Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            PaintKind::Text(text) => {
                painter.with_clip_rect(clip).text(
                    item_rect.min,
                    Align2::LEFT_TOP,
                    &text.text,
                    font_id(
                        &text.style.family,
                        text.style.font_size,
                        item.transform.scale,
                    ),
                    egui_color(text.style.color, item.opacity),
                );
            }
            PaintKind::RichRect(rect_primitive) => {
                paint_rich_rect(
                    &painter.with_clip_rect(clip),
                    item_rect,
                    rect_primitive,
                    item.opacity,
                );
            }
            PaintKind::SceneText(text) => {
                let text_rect = egui_rect(offset_rect(
                    transform_rect(text.rect, item.transform),
                    rect.min,
                ));
                painter.with_clip_rect(clip).text(
                    scene_text_pos(text_rect, text),
                    scene_text_align(text),
                    scene_text_content(text),
                    font_id(
                        &text.style.family,
                        text.style.font_size,
                        item.transform.scale,
                    ),
                    egui_color(text.style.color, item.opacity),
                );
            }
            PaintKind::Line { from, to, stroke } => {
                painter.with_clip_rect(clip).line_segment(
                    [
                        egui_pos(offset_point(
                            transform_point(*from, item.transform),
                            rect.min,
                        )),
                        egui_pos(offset_point(transform_point(*to, item.transform), rect.min)),
                    ],
                    Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                );
            }
            PaintKind::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let node_painter = painter.with_clip_rect(clip);
                let center = egui_pos(offset_point(
                    transform_point(*center, item.transform),
                    rect.min,
                ));
                let radius = radius * item.transform.scale.max(0.0);
                if fill.a > 0 {
                    node_painter.circle_filled(center, radius, egui_color(*fill, item.opacity));
                }
                if let Some(stroke) = *stroke {
                    node_painter.circle_stroke(
                        center,
                        radius,
                        Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                    );
                }
            }
            PaintKind::Polygon {
                points,
                fill,
                stroke,
            } => {
                let points = points
                    .iter()
                    .copied()
                    .map(|point| {
                        egui_pos(offset_point(
                            transform_point(point, item.transform),
                            rect.min,
                        ))
                    })
                    .collect::<Vec<_>>();
                if fill.a > 0 && points.len() >= 3 {
                    painter.with_clip_rect(clip).add(Shape::convex_polygon(
                        points.clone(),
                        egui_color(*fill, item.opacity),
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = *stroke {
                    painter.with_clip_rect(clip).add(Shape::line(
                        points,
                        Stroke::new(stroke.width, egui_color(stroke.color, item.opacity)),
                    ));
                }
            }
            PaintKind::Path(path) => {
                let points = paint_path_points(path)
                    .into_iter()
                    .map(|point| {
                        egui_pos(offset_point(
                            transform_point(point, item.transform),
                            rect.min,
                        ))
                    })
                    .collect::<Vec<_>>();
                if let Some(fill) = &path.fill
                    && points.len() >= 3
                {
                    painter.with_clip_rect(clip).add(Shape::convex_polygon(
                        points.clone(),
                        egui_color(fill.fallback_color(), item.opacity),
                        Stroke::NONE,
                    ));
                }
                if let Some(stroke) = path.stroke
                    && points.len() >= 2
                {
                    painter.with_clip_rect(clip).add(Shape::line(
                        points,
                        Stroke::new(
                            stroke.style.width,
                            egui_color(stroke.style.color, item.opacity),
                        ),
                    ));
                }
            }
            PaintKind::Image { key, tint } => {
                paint_image_placeholder(
                    &painter.with_clip_rect(clip),
                    item_rect,
                    key,
                    *tint,
                    item.opacity,
                );
            }
            PaintKind::ImagePlacement(image) => {
                let image_rect = egui_rect(offset_rect(
                    transform_rect(image.rect, item.transform),
                    rect.min,
                ));
                paint_image_placeholder(
                    &painter.with_clip_rect(clip),
                    image_rect,
                    &image.key,
                    image.tint,
                    item.opacity,
                );
            }
            PaintKind::Canvas(canvas) => {
                paint_canvas_placeholder(
                    &painter.with_clip_rect(clip),
                    item_rect,
                    canvas,
                    item.opacity,
                );
            }
        }
    }
}

pub(crate) fn hit_test_name(document: &UiDocument, rect: Rect, screen_pos: Pos2) -> Option<String> {
    if !rect.contains(screen_pos) {
        return None;
    }
    let local = UiPoint::new(screen_pos.x - rect.min.x, screen_pos.y - rect.min.y);
    document
        .hit_test(local)
        .map(|node_id| document.node(node_id).name.clone())
}

fn offset_rect(rect: UiRect, offset: Pos2) -> UiRect {
    UiRect::new(
        rect.x + offset.x,
        rect.y + offset.y,
        rect.width,
        rect.height,
    )
}

fn offset_point(point: UiPoint, offset: Pos2) -> UiPoint {
    UiPoint::new(point.x + offset.x, point.y + offset.y)
}

fn transform_point(point: UiPoint, transform: PaintTransform) -> UiPoint {
    UiPoint::new(
        point.x * transform.scale + transform.translation.x,
        point.y * transform.scale + transform.translation.y,
    )
}

fn transform_rect(rect: UiRect, transform: PaintTransform) -> UiRect {
    let top_left = transform_point(UiPoint::new(rect.x, rect.y), transform);
    UiRect::new(
        top_left.x,
        top_left.y,
        rect.width * transform.scale,
        rect.height * transform.scale,
    )
}

fn egui_rect(rect: UiRect) -> Rect {
    Rect::from_min_size(
        Pos2::new(rect.x, rect.y),
        egui::vec2(rect.width, rect.height),
    )
}

fn egui_pos(point: UiPoint) -> Pos2 {
    Pos2::new(point.x, point.y)
}

fn egui_color(color: ColorRgba, opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r,
        color.g,
        color.b,
        ((color.a as f32) * opacity.clamp(0.0, 1.0)).round() as u8,
    )
}

fn paint_rich_rect(painter: &egui::Painter, rect: Rect, primitive: &PaintRect, opacity: f32) {
    let radius = primitive.corner_radii.max_radius();
    for effect in &primitive.effects {
        let color = egui_color(effect.color, opacity);
        match effect.kind {
            PaintEffectKind::Shadow | PaintEffectKind::Glow => {
                let spread = effect.spread.max(0.0) + effect.blur_radius.max(0.0) * 0.25;
                let effect_rect = rect
                    .expand(spread)
                    .translate(egui::vec2(effect.offset.x, effect.offset.y));
                painter.rect_filled(effect_rect, radius + spread, color);
            }
            PaintEffectKind::InsetShadow => {
                painter.rect_stroke(
                    rect.shrink(effect.spread.max(0.0)),
                    radius,
                    Stroke::new(effect.blur_radius.max(1.0), color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    let fill = primitive.fill.fallback_color();
    if fill.a > 0 {
        painter.rect_filled(rect, radius, egui_color(fill, opacity));
    }
    if let Some(stroke) = primitive.stroke {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(stroke.style.width, egui_color(stroke.style.color, opacity)),
            egui_stroke_kind(stroke.alignment),
        );
    }
}

fn egui_stroke_kind(alignment: StrokeAlignment) -> egui::StrokeKind {
    match alignment {
        StrokeAlignment::Inside => egui::StrokeKind::Inside,
        StrokeAlignment::Center => egui::StrokeKind::Middle,
        StrokeAlignment::Outside => egui::StrokeKind::Outside,
    }
}

fn scene_text_pos(rect: Rect, text: &PaintText) -> Pos2 {
    let x = match text.horizontal_align {
        TextHorizontalAlign::Start => rect.min.x,
        TextHorizontalAlign::Center => rect.center().x,
        TextHorizontalAlign::End => rect.max.x,
    };
    let y = match text.vertical_align {
        TextVerticalAlign::Top | TextVerticalAlign::Baseline => rect.min.y,
        TextVerticalAlign::Center => rect.center().y,
        TextVerticalAlign::Bottom => rect.max.y,
    };
    Pos2::new(x, y)
}

fn scene_text_align(text: &PaintText) -> Align2 {
    match (text.horizontal_align, text.vertical_align) {
        (TextHorizontalAlign::Start, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            Align2::LEFT_TOP
        }
        (TextHorizontalAlign::Center, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            Align2::CENTER_TOP
        }
        (TextHorizontalAlign::End, TextVerticalAlign::Top | TextVerticalAlign::Baseline) => {
            Align2::RIGHT_TOP
        }
        (TextHorizontalAlign::Start, TextVerticalAlign::Center) => Align2::LEFT_CENTER,
        (TextHorizontalAlign::Center, TextVerticalAlign::Center) => Align2::CENTER_CENTER,
        (TextHorizontalAlign::End, TextVerticalAlign::Center) => Align2::RIGHT_CENTER,
        (TextHorizontalAlign::Start, TextVerticalAlign::Bottom) => Align2::LEFT_BOTTOM,
        (TextHorizontalAlign::Center, TextVerticalAlign::Bottom) => Align2::CENTER_BOTTOM,
        (TextHorizontalAlign::End, TextVerticalAlign::Bottom) => Align2::RIGHT_BOTTOM,
    }
}

fn scene_text_content(text: &PaintText) -> &str {
    if text.multiline {
        &text.text
    } else {
        text.text.lines().next().unwrap_or("")
    }
}

fn paint_path_points(path: &PaintPath) -> Vec<UiPoint> {
    path.verbs
        .iter()
        .filter_map(|verb| match *verb {
            PathVerb::MoveTo(point) | PathVerb::LineTo(point) => Some(point),
            PathVerb::QuadraticTo { to, .. } | PathVerb::CubicTo { to, .. } => Some(to),
            PathVerb::Close => None,
        })
        .collect()
}

fn paint_image_placeholder(
    painter: &egui::Painter,
    rect: Rect,
    key: &str,
    tint: Option<ColorRgba>,
    opacity: f32,
) {
    if rect.width() <= f32::EPSILON || rect.height() <= f32::EPSILON || opacity <= 0.0 {
        return;
    }
    let base = tint.unwrap_or_else(|| resource_color_from_key(key, 235));
    painter.rect_filled(rect, 3.0, egui_color(base, opacity));

    let hash = resource_hash(key);
    let stripe = ColorRgba::new(
        base.r.saturating_sub(((hash >> 8) & 31) as u8),
        base.g.saturating_sub(((hash >> 16) & 31) as u8),
        base.b.saturating_sub(((hash >> 24) & 31) as u8),
        base.a,
    );
    let mut x = rect.left();
    while x < rect.right() {
        let stripe_rect = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            egui::vec2(2.0_f32.min(rect.right() - x), rect.height()),
        );
        painter.rect_filled(stripe_rect, 0.0, egui_color(stripe, opacity * 0.8));
        x += 6.0;
    }

    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, egui_color(stripe, opacity)),
        egui::StrokeKind::Inside,
    );
}

fn paint_canvas_placeholder(
    painter: &egui::Painter,
    rect: Rect,
    canvas: &CanvasContent,
    opacity: f32,
) {
    if rect.width() <= f32::EPSILON || rect.height() <= f32::EPSILON || opacity <= 0.0 {
        return;
    }
    let base = resource_color_from_key(&canvas.key, 210);
    painter.rect_filled(rect, 3.0, egui_color(base, opacity));
    let accent = ColorRgba::new(
        base.r.saturating_add(34),
        base.g.saturating_add(24),
        base.b.saturating_add(18),
        255,
    );
    let mut x = rect.left() - rect.height();
    while x < rect.right() {
        painter.line_segment(
            [
                Pos2::new(x, rect.top()),
                Pos2::new(x + rect.height(), rect.bottom()),
            ],
            Stroke::new(1.0, egui_color(accent, opacity)),
        );
        x += 12.0;
    }
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, egui_color(accent, opacity)),
        egui::StrokeKind::Inside,
    );
}

fn resource_color_from_key(key: &str, alpha: u8) -> ColorRgba {
    let hash = resource_hash(key);
    ColorRgba::new(
        96_u8.saturating_add((hash & 63) as u8),
        112_u8.saturating_add(((hash >> 8) & 63) as u8),
        128_u8.saturating_add(((hash >> 16) & 63) as u8),
        alpha,
    )
}

fn resource_hash(key: &str) -> u32 {
    key.bytes().fold(0x811c_9dc5, |hash, byte| {
        hash.wrapping_mul(16_777_619) ^ u32::from(byte)
    })
}

fn font_id(family: &FontFamily, size: f32, scale: f32) -> FontId {
    let size = size * scale.max(0.0);
    match family {
        FontFamily::Monospace => FontId::monospace(size),
        FontFamily::Serif | FontFamily::SansSerif | FontFamily::Named(_) => {
            FontId::proportional(size)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_placeholder_color_is_stable_and_uses_requested_alpha() {
        let first = resource_color_from_key("icons.warning", 210);
        let second = resource_color_from_key("icons.warning", 210);
        let other = resource_color_from_key("icons.browser", 210);

        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_eq!(first.a, 210);
    }
}
