#![allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CrossSectionPreviewOptions {
    pub(crate) show_mask_overlay: bool,
    pub(crate) show_dimension_guides: bool,
    pub(crate) show_risk_cues: bool,
    pub(crate) compact_layout: bool,
    pub(crate) medium_layout: bool,
    pub(crate) body_width: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CrossSectionSurfaceSummary {
    pub(crate) min_um: f32,
    pub(crate) max_um: f32,
    pub(crate) average_um: f32,
    pub(crate) range_um: f32,
}

pub(crate) fn cross_section_preview_primitives(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
    selected_material: Option<&MaterialId>,
    options: CrossSectionPreviewOptions,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let scene_width = (options.body_width - ui_scale.value(40.0)).max(ui_scale.value(260.0));
    let bounds = if options.compact_layout {
        UiRect::new(
            ui_scale.value(24.0),
            ui_scale.value(40.0),
            ui_scale.value(236.0),
            ui_scale.value(204.0),
        )
    } else if options.medium_layout {
        let width = ui_scale
            .value(520.0)
            .min((scene_width - ui_scale.value(32.0)).max(ui_scale.value(260.0)));
        let x = ((scene_width - width) * 0.5).max(ui_scale.value(18.0));
        UiRect::new(x, ui_scale.value(36.0), width, ui_scale.value(110.0))
    } else {
        let width = ui_scale
            .value(850.0)
            .min((scene_width - ui_scale.value(40.0)).max(ui_scale.value(360.0)));
        let x = ((scene_width - width) * 0.5).max(ui_scale.value(24.0));
        UiRect::new(x, ui_scale.value(40.0), width, ui_scale.value(236.0))
    };
    let surface = cross_section_surface_summary(process, snapshot);
    let max_y = surface.max_um.max(process.substrate_thickness_um).max(1.0);
    let mut primitives = Vec::with_capacity(snapshot.segments.len() + 32);

    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(72, 86, 100, 255),
            ui_scale.value(1.0),
        )),
    ));

    if options.show_dimension_guides {
        for fraction in [0.25_f32, 0.5, 0.75] {
            let x = bounds.x + bounds.width * fraction;
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(x, bounds.y),
                to: UiPoint::new(x, bounds.bottom()),
                stroke: StrokeStyle::new(ColorRgba::new(160, 176, 192, 58), ui_scale.value(1.0)),
            });
            let y = bounds.bottom() - bounds.height * fraction;
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(bounds.x, y),
                to: UiPoint::new(bounds.right(), y),
                stroke: StrokeStyle::new(ColorRgba::new(160, 176, 192, 58), ui_scale.value(1.0)),
            });
        }
    }

    if options.show_mask_overlay && !snapshot.active_mask.is_empty() {
        let mask_top = ui_scale.value(18.0);
        for opening in &snapshot.active_mask {
            let x0 = cross_section_x_to_scene(bounds, process.width_um, opening.start_um);
            let x1 = cross_section_x_to_scene(bounds, process.width_um, opening.end_um);
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x0, mask_top, (x1 - x0).max(1.0), ui_scale.value(11.0)),
                ColorRgba::new(248, 208, 78, 150),
            )));
        }
    }

    for segment in &snapshot.segments {
        let selected = selected_material == Some(&segment.material);
        let fill = cross_section_material_color(process, &segment.material, 224);
        let stroke = if selected {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(2.0))
        } else {
            StrokeStyle::new(ColorRgba::new(16, 20, 24, 145), ui_scale.value(0.8))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                cross_section_segment_scene_rect(process, segment, bounds, max_y),
                fill,
            )
            .stroke(stroke),
        ));
    }

    if options.show_risk_cues {
        add_cross_section_risk_primitives(
            process,
            snapshot,
            bounds,
            max_y,
            ui_scale,
            &mut primitives,
        );
    }

    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, bounds.bottom()),
        to: UiPoint::new(bounds.right(), bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(188, 198, 208, 255), ui_scale.value(1.0)),
    });
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, bounds.bottom()),
        to: UiPoint::new(bounds.x, bounds.y),
        stroke: StrokeStyle::new(ColorRgba::new(188, 198, 208, 255), ui_scale.value(1.0)),
    });

    let swatch_y = ui_scale.value(if options.compact_layout {
        256.0
    } else if options.medium_layout {
        158.0
    } else {
        292.0
    });
    let mut swatch_x = bounds.x;
    for material in process.materials.iter().take(8) {
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(
                    swatch_x,
                    swatch_y,
                    ui_scale.value(22.0),
                    ui_scale.value(12.0),
                ),
                ColorRgba::new(
                    material.color_rgb[0],
                    material.color_rgb[1],
                    material.color_rgb[2],
                    240,
                ),
            )
            .stroke(StrokeStyle::new(
                ColorRgba::new(18, 22, 26, 190),
                ui_scale.value(1.0),
            )),
        ));
        swatch_x += ui_scale.value(34.0);
    }

    primitives
}

pub(crate) fn add_cross_section_risk_primitives(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
    bounds: UiRect,
    max_y: f32,
    ui_scale: UiScale,
    primitives: &mut Vec<ScenePrimitive>,
) {
    let heights = cross_section_surface_heights(process, snapshot);
    if heights.len() < 2 {
        return;
    }
    let column_width = process.width_um.max(0.1) / heights.len() as f32;
    let mut last_marker_x = f32::NEG_INFINITY;
    for index in 1..heights.len() {
        let delta = (heights[index] - heights[index - 1]).abs();
        if delta < 0.08 {
            continue;
        }
        let x_um = index as f32 * column_width;
        let x = cross_section_x_to_scene(bounds, process.width_um, x_um);
        if (x - last_marker_x).abs() < ui_scale.value(12.0) {
            continue;
        }
        last_marker_x = x;
        let y = cross_section_y_to_scene(bounds, heights[index].max(heights[index - 1]), max_y);
        let color = if delta > 0.25 {
            ColorRgba::new(236, 91, 88, 240)
        } else {
            ColorRgba::new(238, 181, 82, 235)
        };
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, bounds.y),
            to: UiPoint::new(x, bounds.bottom()),
            stroke: StrokeStyle::new(color, ui_scale.value(1.2)),
        });
        primitives.push(ScenePrimitive::Polygon {
            points: vec![
                UiPoint::new(x, y - ui_scale.value(10.0)),
                UiPoint::new(x - ui_scale.value(5.0), y - ui_scale.value(1.0)),
                UiPoint::new(x + ui_scale.value(5.0), y - ui_scale.value(1.0)),
            ],
            fill: color,
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(16, 20, 24, 180),
                ui_scale.value(1.0),
            )),
        });
    }
}

pub(crate) fn cross_section_segment_scene_rect(
    process: &layout_model::cross_section::CrossSectionProcess,
    segment: &layout_model::cross_section::CrossSectionSegment,
    bounds: UiRect,
    max_y: f32,
) -> UiRect {
    let x0 = cross_section_x_to_scene(bounds, process.width_um, segment.x0_um);
    let x1 = cross_section_x_to_scene(bounds, process.width_um, segment.x1_um);
    let y0 = cross_section_y_to_scene(bounds, segment.y0_um, max_y);
    let y1 = cross_section_y_to_scene(bounds, segment.y1_um, max_y);
    UiRect::new(x0, y1, (x1 - x0).max(1.0), (y0 - y1).max(1.0))
}

pub(crate) fn cross_section_x_to_scene(bounds: UiRect, width_um: f32, value_um: f32) -> f32 {
    bounds.x + bounds.width * value_um / width_um.max(0.1)
}

pub(crate) fn cross_section_y_to_scene(bounds: UiRect, value_um: f32, max_y: f32) -> f32 {
    bounds.bottom() - bounds.height * value_um / max_y.max(0.1)
}

pub(crate) fn cross_section_material_color(
    process: &layout_model::cross_section::CrossSectionProcess,
    material_id: &MaterialId,
    alpha: u8,
) -> ColorRgba {
    process
        .material(material_id)
        .map(|material| {
            ColorRgba::new(
                material.color_rgb[0],
                material.color_rgb[1],
                material.color_rgb[2],
                alpha,
            )
        })
        .unwrap_or_else(|| ColorRgba::new(174, 184, 194, alpha))
}

pub(crate) fn cross_section_surface_summary(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> CrossSectionSurfaceSummary {
    let heights = cross_section_surface_heights(process, snapshot);
    if heights.is_empty() {
        return CrossSectionSurfaceSummary {
            min_um: 0.0,
            max_um: process.substrate_thickness_um,
            average_um: process.substrate_thickness_um,
            range_um: 0.0,
        };
    }
    let min_um = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let max_um = heights.iter().copied().fold(0.0, f32::max);
    let average_um = heights.iter().sum::<f32>() / heights.len() as f32;
    CrossSectionSurfaceSummary {
        min_um,
        max_um,
        average_um,
        range_um: max_um - min_um,
    }
}

pub(crate) fn cross_section_surface_heights(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Vec<f32> {
    let columns = process.columns.max(1);
    let column_width = process.width_um.max(0.1) / columns as f32;
    (0..columns)
        .map(|index| {
            let x_um = (index as f32 + 0.5) * column_width;
            snapshot
                .segments
                .iter()
                .filter(|segment| x_um >= segment.x0_um && x_um <= segment.x1_um)
                .map(|segment| segment.y1_um)
                .fold(0.0, f32::max)
        })
        .collect()
}

pub(crate) fn cross_section_mask_coverage_um(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> f32 {
    snapshot
        .active_mask
        .iter()
        .map(|opening| {
            let start = opening.start_um.clamp(0.0, process.width_um);
            let end = opening.end_um.clamp(0.0, process.width_um);
            (end - start).max(0.0)
        })
        .sum::<f32>()
        .min(process.width_um.max(0.0))
}

pub(crate) fn selected_cross_section_material_for_snapshot(
    app: &GlassworksApp,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Option<MaterialId> {
    if let Some(material) = &app.selected_cross_section_material
        && (app.workspace.cross_section.material(material).is_some()
            || snapshot
                .segments
                .iter()
                .any(|segment| segment.material == *material))
    {
        return Some(material.clone());
    }
    cross_section_step_material_id(&app.workspace.cross_section, snapshot.step_index)
        .or_else(|| cross_section_dominant_material(snapshot))
}

pub(crate) fn cross_section_step_material_id(
    process: &layout_model::cross_section::CrossSectionProcess,
    step_index: usize,
) -> Option<MaterialId> {
    if step_index == 0 {
        return Some(process.substrate_material.clone());
    }
    process
        .steps
        .get(step_index.saturating_sub(1))
        .and_then(|step| match &step.kind {
            layout_model::cross_section::ProcessStepKind::Deposit { material, .. }
            | layout_model::cross_section::ProcessStepKind::Etch { material, .. } => {
                Some(material.clone())
            }
            layout_model::cross_section::ProcessStepKind::Pattern { .. } => None,
        })
}

pub(crate) fn cross_section_dominant_material(
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Option<MaterialId> {
    let mut best_material = None;
    let mut best_area = 0.0;
    for segment in &snapshot.segments {
        let area =
            (segment.x1_um - segment.x0_um).max(0.0) * (segment.y1_um - segment.y0_um).max(0.0);
        if area > best_area {
            best_area = area;
            best_material = Some(segment.material.clone());
        }
    }
    best_material
}

pub(crate) fn cross_section_step_kind_label_for_index(
    process: &layout_model::cross_section::CrossSectionProcess,
    step_index: usize,
) -> String {
    if step_index == 0 {
        "substrate".to_string()
    } else {
        process
            .steps
            .get(step_index.saturating_sub(1))
            .map(|step| cross_section_step_kind_label(&step.kind))
            .unwrap_or_else(|| "unknown step".to_string())
    }
}

pub(crate) fn format_um_f32(value: f32) -> String {
    format!("{value:.2} um")
}
