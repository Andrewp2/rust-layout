use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Shape, Stroke};
use operad::{ColorRgba, FontFamily, PaintKind, PaintTransform, UiDocument, UiPoint, UiRect};

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
            PaintKind::Canvas(_) | PaintKind::Image { .. } => {}
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

fn font_id(family: &FontFamily, size: f32, scale: f32) -> FontId {
    let size = size * scale.max(0.0);
    match family {
        FontFamily::Monospace => FontId::monospace(size),
        FontFamily::Serif | FontFamily::SansSerif | FontFamily::Named(_) => {
            FontId::proportional(size)
        }
    }
}
