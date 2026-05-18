#![allow(unused_imports)]
use super::*;

pub(crate) fn metrology_view_primitives(
    app: &GlassworksApp,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let map = &app.workspace.wafer_map;
    let mut primitives = Vec::with_capacity(map.dies.len() * 3 + map.defects.len() + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let wafer_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(12.0),
        ui_scale.value(178.0),
        ui_scale.value(178.0),
    );
    let histogram_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(26.0),
        ui_scale.value(286.0),
        ui_scale.value(72.0),
    );
    let radial_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(126.0),
        ui_scale.value(286.0),
        ui_scale.value(56.0),
    );
    let band_bounds = UiRect::new(
        frame.x + ui_scale.value(576.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(292.0),
        ui_scale.value(154.0),
    );
    add_metrology_wafer_primitives(&mut primitives, wafer_bounds, app, ui_scale);
    add_metrology_histogram_primitives(
        &mut primitives,
        histogram_bounds,
        map,
        app.metrology_kind,
        ui_scale,
    );
    add_metrology_radial_profile_primitives(
        &mut primitives,
        radial_bounds,
        map,
        app.metrology_kind,
        ui_scale,
    );
    add_metrology_kind_band_primitives(&mut primitives, band_bounds, map, ui_scale);
    primitives
}

pub(crate) fn add_metrology_wafer_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let map = &app.workspace.wafer_map;
    let summary = map.summary(app.metrology_kind);
    let center = UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    let wafer_radius = bounds.width.min(bounds.height) * 0.5;
    primitives.push(ScenePrimitive::Circle {
        center,
        radius: wafer_radius,
        fill: ColorRgba::new(20, 27, 32, 255),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(96, 111, 126, 255),
            ui_scale.value(1.2),
        )),
    });
    primitives.push(ScenePrimitive::Circle {
        center,
        radius: wafer_radius * 0.92,
        fill: ColorRgba::new(0, 0, 0, 0),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(76, 92, 108, 170),
            ui_scale.value(1.0),
        )),
    });

    let scale = metrology_scene_scale(map, bounds);
    let die_w = (map.geometry.die_size_mm[0] as f32 * scale * 0.78)
        .clamp(ui_scale.value(2.4), ui_scale.value(10.0));
    let die_h = (map.geometry.die_size_mm[1] as f32 * scale * 0.78)
        .clamp(ui_scale.value(2.4), ui_scale.value(10.0));
    for &die in &map.dies {
        let score = metrology_die_attention_score(map, die, app.metrology_kind);
        let selected = app.selected_die == Some(die);
        if app.metrology_failed_only && score == 0 && !selected {
            continue;
        }
        let point = metrology_mm_to_scene(map.geometry.die_center_mm(die), map, bounds);
        let rect = UiRect::new(point.x - die_w * 0.5, point.y - die_h * 0.5, die_w, die_h);
        let stroke = if selected {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(1.6))
        } else if score > 0 {
            StrokeStyle::new(ColorRgba::new(238, 181, 82, 205), ui_scale.value(0.9))
        } else {
            StrokeStyle::new(ColorRgba::new(15, 19, 23, 145), ui_scale.value(0.7))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                rect,
                metrology_die_fill(
                    map,
                    die,
                    app.metrology_kind,
                    app.metrology_map_mode,
                    summary,
                ),
            )
            .stroke(stroke),
        ));
    }

    if app.metrology_map_mode == MetrologyMapMode::OverlayVectors {
        for &die in &map.dies {
            if app.metrology_failed_only
                && metrology_die_attention_score(map, die, app.metrology_kind) == 0
            {
                continue;
            }
            let Some(vector) = metrology_overlay_vector(map, die) else {
                continue;
            };
            let origin = metrology_mm_to_scene(map.geometry.die_center_mm(die), map, bounds);
            primitives.push(ScenePrimitive::Line {
                from: origin,
                to: UiPoint::new(
                    origin.x + vector[0] * ui_scale.value(12.0),
                    origin.y - vector[1] * ui_scale.value(12.0),
                ),
                stroke: StrokeStyle::new(ColorRgba::new(102, 190, 236, 220), ui_scale.value(1.1)),
            });
        }
    }

    if matches!(
        app.metrology_map_mode,
        MetrologyMapMode::DefectReview | MetrologyMapMode::ReviewQueue
    ) {
        for defect in &map.defects {
            if app.metrology_failed_only && app.selected_die != Some(defect.die) {
                continue;
            }
            let point = metrology_mm_to_scene(defect.position_mm, map, bounds);
            primitives.push(ScenePrimitive::Circle {
                center: point,
                radius: ui_scale.value((2.2 + defect.severity as f32).min(6.0)),
                fill: ColorRgba::new(244, 112, 104, 225),
                stroke: Some(StrokeStyle::new(
                    ColorRgba::new(16, 20, 24, 190),
                    ui_scale.value(0.8),
                )),
            });
        }
    }

    for annotation in map.annotations.iter().take(24) {
        let point = metrology_mm_to_scene(annotation.position_mm, map, bounds);
        let size = ui_scale.value(3.8);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(point.x - size, point.y),
            to: UiPoint::new(point.x + size, point.y),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 210), ui_scale.value(1.0)),
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(point.x, point.y - size),
            to: UiPoint::new(point.x, point.y + size),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 210), ui_scale.value(1.0)),
        });
    }
}

pub(crate) fn add_metrology_histogram_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    kind: MeasurementKind,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let bins = map.histogram(kind, 14);
    let max_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0).max(1) as f32;
    if bins.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(2.0);
    let bar_width = (bounds.width - gap * bins.len().saturating_sub(1) as f32) / bins.len() as f32;
    for (index, bin) in bins.iter().enumerate() {
        let height = (bin.count as f32 / max_count) * (bounds.height - ui_scale.value(8.0));
        let x = bounds.x + index as f32 * (bar_width + gap);
        let bar = UiRect::new(
            x,
            bounds.bottom() - height,
            bar_width.max(ui_scale.value(2.0)),
            height.max(ui_scale.value(1.0)),
        );
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            bar,
            ColorRgba::new(93, 168, 232, 210),
        )));
    }
}

pub(crate) fn add_metrology_radial_profile_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    kind: MeasurementKind,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let profile = metrology_radial_profile(map, kind, 10);
    if profile.len() < 2 {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value) in &profile {
        if value.is_finite() {
            min_value = min_value.min(*value);
            max_value = max_value.max(*value);
        }
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);
    for pair in profile.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                bounds.x + pair[0].0 * bounds.width,
                spc_plot_y(bounds, min_value, max_value, pair[0].1),
            ),
            to: UiPoint::new(
                bounds.x + pair[1].0 * bounds.width,
                spc_plot_y(bounds, min_value, max_value, pair[1].1),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 230), ui_scale.value(1.8)),
        });
    }
}

pub(crate) fn add_metrology_kind_band_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * MeasurementKind::ALL.len().saturating_sub(1) as f32)
        / MeasurementKind::ALL.len() as f32;
    for (index, kind) in MeasurementKind::ALL.iter().enumerate() {
        let summary = map.summary(*kind);
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let total = summary.sample_count.max(1) as f32;
        let pass_w = row.width * summary.pass_count as f32 / total;
        let fail_w = row.width * summary.fail_count as f32 / total;
        let outlier_w = row.width * summary.outlier_count as f32 / total;
        let mut x = row.x;
        for (width, color) in [
            (pass_w, metrology_status_color(MeasurementStatus::Pass, 205)),
            (fail_w, metrology_status_color(MeasurementStatus::Fail, 215)),
            (
                outlier_w,
                metrology_status_color(MeasurementStatus::Outlier, 215),
            ),
        ] {
            if width <= 0.0 {
                continue;
            }
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, row.y, width.max(ui_scale.value(1.0)), row.height),
                color,
            )));
            x += width;
        }
    }
}

pub(crate) fn add_metrology_empty_line(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(
            bounds.x + ui_scale.value(18.0),
            bounds.y + bounds.height * 0.5,
        ),
        to: UiPoint::new(
            bounds.right() - ui_scale.value(18.0),
            bounds.y + bounds.height * 0.5,
        ),
        stroke: StrokeStyle::new(ColorRgba::new(92, 106, 120, 190), ui_scale.value(1.0)),
    });
}

pub(crate) fn metrology_scene_scale(map: &WaferMap, bounds: UiRect) -> f32 {
    let radius = map.geometry.active_radius_mm().max(1.0) as f32;
    bounds.width.min(bounds.height) * 0.5 / radius
}

pub(crate) fn metrology_mm_to_scene(point_mm: [f64; 2], map: &WaferMap, bounds: UiRect) -> UiPoint {
    let scale = metrology_scene_scale(map, bounds);
    UiPoint::new(
        bounds.x + bounds.width * 0.5 + point_mm[0] as f32 * scale,
        bounds.y + bounds.height * 0.5 - point_mm[1] as f32 * scale,
    )
}

pub(crate) fn metrology_die_fill(
    map: &WaferMap,
    die: DieCoord,
    kind: MeasurementKind,
    mode: MetrologyMapMode,
    summary: MeasurementSummary,
) -> ColorRgba {
    let defect_count = map.defects_for_die(die).count();
    let score = metrology_die_attention_score(map, die, kind);
    let Some(measurement) = map.measurement_for(die, kind) else {
        return ColorRgba::new(45, 54, 62, 130);
    };
    match mode {
        MetrologyMapMode::ValueMap => metrology_value_color(measurement.value, summary, 225),
        MetrologyMapMode::DeviationMap => {
            metrology_deviation_color(kind, measurement.value, summary, 225)
        }
        MetrologyMapMode::SpecWindow => metrology_status_color(measurement.status, 225),
        MetrologyMapMode::DefectReview => metrology_defect_density_color(defect_count, 225),
        MetrologyMapMode::ReviewQueue => metrology_attention_score_color(score, 225),
        MetrologyMapMode::OverlayVectors => metrology_status_color(measurement.status, 145),
    }
}

pub(crate) fn metrology_value_color(
    value: f64,
    summary: MeasurementSummary,
    alpha: u8,
) -> ColorRgba {
    let ratio = match (summary.min, summary.max) {
        (Some(min), Some(max)) if (max - min).abs() > f64::EPSILON => {
            ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
        }
        _ => 0.5,
    };
    if ratio < 0.5 {
        metrology_lerp_color(
            ColorRgba::new(76, 145, 218, alpha),
            ColorRgba::new(91, 190, 130, alpha),
            ratio * 2.0,
        )
    } else {
        metrology_lerp_color(
            ColorRgba::new(91, 190, 130, alpha),
            ColorRgba::new(224, 168, 68, alpha),
            (ratio - 0.5) * 2.0,
        )
    }
}

pub(crate) fn metrology_deviation_color(
    kind: MeasurementKind,
    value: f64,
    summary: MeasurementSummary,
    alpha: u8,
) -> ColorRgba {
    let spec = kind.spec();
    let Some(target) = spec.target else {
        return metrology_value_color(value, summary, alpha);
    };
    let span = spec
        .lower
        .map(|lower| (target - lower).abs())
        .into_iter()
        .chain(spec.upper.map(|upper| (upper - target).abs()))
        .chain(summary.stddev.map(|stddev| stddev * 3.0))
        .fold(1.0_f64, f64::max);
    let ratio = ((value - target) / span).clamp(-1.0, 1.0);
    if ratio.abs() < 0.12 {
        ColorRgba::new(91, 190, 130, alpha)
    } else if ratio > 0.0 {
        metrology_lerp_color(
            ColorRgba::new(224, 168, 68, alpha),
            ColorRgba::new(236, 91, 88, alpha),
            ratio as f32,
        )
    } else {
        metrology_lerp_color(
            ColorRgba::new(102, 190, 236, alpha),
            ColorRgba::new(76, 114, 202, alpha),
            ratio.abs() as f32,
        )
    }
}

pub(crate) fn metrology_status_color(status: MeasurementStatus, alpha: u8) -> ColorRgba {
    match status {
        MeasurementStatus::Pass => ColorRgba::new(91, 190, 130, alpha),
        MeasurementStatus::Fail => ColorRgba::new(236, 91, 88, alpha),
        MeasurementStatus::Outlier => ColorRgba::new(238, 181, 82, alpha),
    }
}

pub(crate) fn metrology_defect_density_color(count: usize, alpha: u8) -> ColorRgba {
    match count {
        0 => ColorRgba::new(72, 130, 105, alpha.saturating_sub(50)),
        1 => ColorRgba::new(238, 181, 82, alpha),
        2 | 3 => ColorRgba::new(226, 126, 74, alpha),
        _ => ColorRgba::new(236, 91, 88, alpha),
    }
}

pub(crate) fn metrology_attention_score_color(score: usize, alpha: u8) -> ColorRgba {
    if score >= 72 {
        ColorRgba::new(236, 91, 88, alpha)
    } else if score >= 42 {
        ColorRgba::new(238, 181, 82, alpha)
    } else if score > 0 {
        ColorRgba::new(102, 190, 236, alpha)
    } else {
        ColorRgba::new(91, 190, 130, alpha.saturating_sub(45))
    }
}

pub(crate) fn metrology_lerp_color(left: ColorRgba, right: ColorRgba, t: f32) -> ColorRgba {
    let t = t.clamp(0.0, 1.0);
    ColorRgba::new(
        lerp_u8(left.r, right.r, t),
        lerp_u8(left.g, right.g, t),
        lerp_u8(left.b, right.b, t),
        lerp_u8(left.a, right.a, t),
    )
}

pub(crate) fn lerp_u8(left: u8, right: u8, t: f32) -> u8 {
    (left as f32 + (right as f32 - left as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

pub(crate) fn metrology_attention_sites(
    map: &WaferMap,
    kind: MeasurementKind,
) -> Vec<(DieCoord, usize)> {
    let mut sites = map
        .dies
        .iter()
        .copied()
        .map(|die| (die, metrology_die_attention_score(map, die, kind)))
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();
    sites.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.column.cmp(&right.0.column))
            .then_with(|| left.0.row.cmp(&right.0.row))
    });
    sites
}

pub(crate) fn metrology_die_attention_score(
    map: &WaferMap,
    die: DieCoord,
    kind: MeasurementKind,
) -> usize {
    let selected_kind_score = map
        .measurement_for(die, kind)
        .map(metrology_measurement_score)
        .unwrap_or_default();
    let other_measurement_score = map
        .measurements_for_die(die)
        .filter(|measurement| measurement.kind != kind)
        .map(|measurement| metrology_measurement_score(measurement) / 2)
        .sum::<usize>();
    let defect_score = map
        .defects_for_die(die)
        .map(|defect| 8 + defect.severity as usize * 4)
        .sum::<usize>();
    let annotation_score = map.annotations_for_die(die).count() * 3;
    selected_kind_score + other_measurement_score + defect_score + annotation_score
}

pub(crate) fn metrology_measurement_score(
    measurement: &layout_model::metrology::Measurement,
) -> usize {
    match measurement.status {
        MeasurementStatus::Pass => 0,
        MeasurementStatus::Fail => 44,
        MeasurementStatus::Outlier => 28,
    }
}

pub(crate) fn metrology_overlay_vector(map: &WaferMap, die: DieCoord) -> Option<[f32; 2]> {
    let cd = map.measurement_for(die, MeasurementKind::CriticalDimensionNm)?;
    let thickness = map.measurement_for(die, MeasurementKind::ThicknessNm)?;
    let cd_target = MeasurementKind::CriticalDimensionNm.spec().target?;
    let thickness_target = MeasurementKind::ThicknessNm.spec().target?;
    let x = ((cd.value - cd_target) / 4.0).clamp(-1.0, 1.0) as f32;
    let y = ((thickness.value - thickness_target) / 50.0).clamp(-1.0, 1.0) as f32;
    ((x.abs() + y.abs()) > 0.08).then_some([x, y])
}

pub(crate) fn metrology_radial_profile(
    map: &WaferMap,
    kind: MeasurementKind,
    bin_count: usize,
) -> Vec<(f32, f64)> {
    if bin_count == 0 {
        return Vec::new();
    }
    let radius = map.geometry.active_radius_mm().max(1.0);
    let mut bins = vec![(0.0_f64, 0usize); bin_count];
    for measurement in map
        .measurements
        .iter()
        .filter(|measurement| measurement.kind == kind)
    {
        let [x, y] = map.geometry.die_center_mm(measurement.die);
        let radial = (x.hypot(y) / radius).clamp(0.0, 1.0);
        let index = ((radial * bin_count as f64).floor() as usize).min(bin_count - 1);
        bins[index].0 += measurement.value;
        bins[index].1 += 1;
    }
    bins.into_iter()
        .enumerate()
        .filter_map(|(index, (sum, count))| {
            (count > 0).then_some(((index as f32 + 0.5) / bin_count as f32, sum / count as f64))
        })
        .collect()
}

pub(crate) fn metrology_summary_label(summary: MeasurementSummary) -> String {
    match (summary.mean, summary.stddev) {
        (Some(mean), Some(stddev)) => format!(
            "mean {} +/- {} {}",
            metrology_format_value(summary.kind, mean),
            metrology_format_value(summary.kind, stddev),
            summary.kind.unit()
        ),
        _ => "No summary".to_string(),
    }
}

pub(crate) fn metrology_selected_die_label(
    map: &WaferMap,
    selected_die: Option<DieCoord>,
) -> String {
    let Some(die) = selected_die else {
        return "No selected die - use Next or attention buttons".to_string();
    };
    let measurements = MeasurementKind::ALL
        .iter()
        .filter_map(|kind| {
            map.measurement_for(die, *kind).map(|measurement| {
                format!(
                    "{} {} {}",
                    kind.label(),
                    metrology_format_value(*kind, measurement.value),
                    measurement.status.label()
                )
            })
        })
        .take(3)
        .collect::<Vec<_>>()
        .join("; ");
    let defects = map.defects_for_die(die).count();
    compact_button_label(
        &format!(
            "{}: {}; {} defect(s)",
            die_coord_label(Some(die)),
            measurements,
            defects
        ),
        96,
    )
}

pub(crate) fn metrology_format_value(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::PassFail | MeasurementKind::DefectCount => format!("{value:.0}"),
        MeasurementKind::SheetResistanceOhmsPerSq => format!("{value:.1}"),
        MeasurementKind::ThicknessNm | MeasurementKind::CriticalDimensionNm => {
            format!("{value:.2}")
        }
    }
}
