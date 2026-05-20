#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_overlay_primitives(
    app: &GlassworksApp,
    size: UiSize,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::new();
    if app.show_reference_images {
        add_layout_reference_image_primitives(&mut primitives, app, size, ui_scale);
    }
    if app.show_grid {
        add_layout_grid_overlay_primitives(&mut primitives, app, size, ui_scale);
    }
    add_layout_scale_bar_primitives(&mut primitives, app, size, ui_scale);
    if app.layout_hierarchy_depth.shows_instance_boxes() {
        add_layout_hierarchy_box_primitives(&mut primitives, app, size, ui_scale);
    }

    let highlight = StrokeStyle::new(ColorRgba::new(112, 236, 214, 240), ui_scale.value(2.0));
    let mut drew_highlight = false;
    for (root_cell, occurrence) in layout_trace_highlight_occurrences(app) {
        if let Some(view) = app
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(root_cell, &occurrence)
        {
            add_overlay_rect_outline(&mut primitives, app, size, view.bounds, highlight);
            drew_highlight = true;
        }
    }
    if !drew_highlight
        && let Some(selected) = app.selected_layout_occurrence.as_ref()
        && let Some(view) = app
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(app.layout_view_top_cell, selected)
    {
        add_overlay_rect_outline(&mut primitives, app, size, view.bounds, highlight);
    } else if !drew_highlight && let Some(shape) = app.selected_layout_shape_ref() {
        add_overlay_rect_outline(&mut primitives, app, size, shape.kind.bounds(), highlight);
    }

    if app.route_points.len() >= 2 {
        for pair in app.route_points.windows(2) {
            let color = if app.route_points.len() == 2 {
                layout_route_point_path_color(app)
            } else {
                layout_route_point_segment_color(app, pair[0], pair[1])
            };
            let stroke = StrokeStyle::new(color, ui_scale.value(1.8));
            primitives.push(ScenePrimitive::Line {
                from: app.layout_world_to_canvas(pair[0], size),
                to: app.layout_world_to_canvas(pair[1], size),
                stroke,
            });
        }
    }

    if layout_should_show_route_point_markers(app) {
        let fill = ColorRgba::new(250, 210, 80, 255);
        for point in &app.route_points {
            primitives.push(ScenePrimitive::Circle {
                center: app.layout_world_to_canvas(*point, size),
                radius: ui_scale.value(5.0),
                fill,
                stroke: Some(StrokeStyle::new(
                    ColorRgba::new(18, 22, 28, 255),
                    ui_scale.value(1.0),
                )),
            });
        }
    }

    if app.show_drc_overlay {
        if let Some(violation) = selected_layout_drc_marker_violation(app) {
            add_overlay_rect_outline(
                &mut primitives,
                app,
                size,
                violation.bounds,
                StrokeStyle::new(ColorRgba::new(250, 210, 80, 255), ui_scale.value(2.2)),
            );
        }
        let shape_count = app.workspace.document.flattened_shape_count_estimate();
        let active = app
            .drc_report()
            .filter(|_| shape_count <= MAX_DRC_INSPECTOR_SHAPES)
            .map(|report| {
                report
                    .violations
                    .iter()
                    .filter(|violation| drc_violation_is_active(&app.workspace.document, violation))
                    .count()
            })
            .unwrap_or(0);
        if active > 0 {
            primitives.push(ScenePrimitive::Text(PaintText::new(
                format!("DRC overlay: {active} active"),
                UiRect::new(
                    ui_scale.value(10.0),
                    ui_scale.value(28.0),
                    ui_scale.value(240.0),
                    ui_scale.value(20.0),
                ),
                text_style(
                    ui_scale.value(12.0),
                    FontWeight::BOLD,
                    ColorRgba::new(238, 181, 82, 255),
                ),
            )));
        }
    }

    primitives
}

pub(crate) fn layout_should_show_route_point_markers(app: &GlassworksApp) -> bool {
    matches!(app.active_tool, ToolMode::Route) || app.route_points.len() >= 2
}

pub(crate) fn layout_route_point_path_color(app: &GlassworksApp) -> ColorRgba {
    if app.route_points.len() < 2 {
        return ColorRgba::new(250, 210, 80, 210);
    }
    let start = app.route_points[0];
    let goal = *app.route_points.last().unwrap_or(&start);
    layout_route_point_segment_color(app, start, goal)
}

pub(crate) fn layout_route_point_segment_color(
    app: &GlassworksApp,
    start: Point,
    goal: Point,
) -> ColorRgba {
    match layout_trace_path_segment_status(app, start, goal) {
        LayoutTracePathSegmentStatus::Connected { .. } => ColorRgba::new(105, 201, 135, 220),
        LayoutTracePathSegmentStatus::Disconnected { .. } => ColorRgba::new(236, 91, 88, 220),
        _ => ColorRgba::new(250, 210, 80, 210),
    }
}

pub(crate) fn add_layout_reference_image_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    app: &GlassworksApp,
    size: UiSize,
    ui_scale: UiScale,
) {
    let viewport = app.layout_viewport_for_size(size).expanded(1);
    let image_stroke = StrokeStyle::new(ColorRgba::new(120, 176, 255, 220), ui_scale.value(1.2));
    let landmark_stroke = StrokeStyle::new(ColorRgba::new(255, 216, 116, 230), ui_scale.value(1.4));
    for image in &app.workspace.document.reference_images {
        if !image.visible
            || image.opacity == 0
            || image.bounds.width() <= 0
            || image.bounds.height() <= 0
            || !image.bounds.intersects(viewport)
        {
            continue;
        }

        let min = app.layout_world_to_canvas(image.bounds.min, size);
        let max = app.layout_world_to_canvas(image.bounds.max, size);
        let rect = UiRect::new(
            min.x.min(max.x),
            min.y.min(max.y),
            (max.x - min.x).abs(),
            (max.y - min.y).abs(),
        );
        primitives.push(ScenePrimitive::Image {
            key: image.image_key(),
            rect,
            tint: Some(ColorRgba::new(255, 255, 255, image.opacity)),
        });
        add_overlay_rect_outline(primitives, app, size, image.bounds, image_stroke);

        let label = compact_button_label(image.display_name(), 24);
        if !label.is_empty()
            && rect.width >= ui_scale.value(36.0)
            && rect.height >= ui_scale.value(18.0)
        {
            primitives.push(ScenePrimitive::Text(PaintText::new(
                label,
                UiRect::new(
                    rect.x + ui_scale.value(4.0),
                    rect.y + ui_scale.value(3.0),
                    rect.width.min(ui_scale.value(180.0)),
                    ui_scale.value(18.0),
                ),
                text_style(
                    ui_scale.value(11.0),
                    FontWeight::BOLD,
                    ColorRgba::new(188, 216, 255, 235),
                ),
            )));
        }

        for landmark in &image.landmarks {
            let center = app.layout_world_to_canvas(landmark.layout, size);
            let radius = ui_scale.value(5.0);
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(center.x - radius, center.y),
                to: UiPoint::new(center.x + radius, center.y),
                stroke: landmark_stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(center.x, center.y - radius),
                to: UiPoint::new(center.x, center.y + radius),
                stroke: landmark_stroke,
            });
        }
    }
}

pub(crate) fn layout_trace_highlight_occurrences(
    app: &GlassworksApp,
) -> Vec<(CellId, ShapeOccurrenceId)> {
    if app.layout_trace_highlight_mode == LayoutTraceHighlightMode::Off
        || app.workspace.document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES
    {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let selected_component = selected_layout_net_component_id(app, &report);
    let mut component_ids = Vec::new();
    match app.layout_trace_highlight_mode {
        LayoutTraceHighlightMode::Selected => {
            if let Some(component_id) = selected_component {
                component_ids.push(component_id);
            }
        }
        LayoutTraceHighlightMode::History => {
            component_ids.extend(app.layout_trace_history.iter().copied());
            if let Some(component_id) = selected_component {
                component_ids.push(component_id);
            }
        }
        LayoutTraceHighlightMode::Off => {}
    }

    let mut seen_components = BTreeSet::new();
    let mut seen_occurrences = BTreeSet::new();
    let mut highlighted = Vec::new();
    for component_id in component_ids {
        if !seen_components.insert(component_id) {
            continue;
        }
        let Some(component) = report.component(component_id) else {
            continue;
        };
        for occurrence in &component.shapes {
            if seen_occurrences.insert(occurrence.clone())
                && !layout_occurrence_hidden_by_cells(
                    &app.workspace.document,
                    app.workspace.document.top_cell,
                    occurrence,
                    &app.layout_hidden_cells,
                )
            {
                highlighted.push((app.workspace.document.top_cell, occurrence.clone()));
            }
        }
    }
    highlighted
}

pub(crate) fn add_layout_hierarchy_box_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    app: &GlassworksApp,
    size: UiSize,
    ui_scale: UiScale,
) {
    let stroke = StrokeStyle::new(ColorRgba::new(126, 184, 232, 220), ui_scale.value(1.4));
    for bounds in layout_hierarchy_box_bounds(app, size) {
        add_overlay_rect_outline(primitives, app, size, bounds, stroke);
    }
}

pub(crate) fn layout_hierarchy_box_bounds(app: &GlassworksApp, size: UiSize) -> Vec<Rect> {
    let document = &app.workspace.document;
    let viewport = app.layout_viewport_for_size(size).expanded(1);
    let Some(root) = document.cell(app.layout_view_top_cell) else {
        return Vec::new();
    };
    let mut boxes = Vec::new();
    let mut child_bounds_cache = BTreeMap::new();
    for instance in root.instances.values() {
        if app.layout_hidden_cells.contains(&instance.cell) {
            continue;
        }
        let child_bounds = if let Some(bounds) = child_bounds_cache.get(&instance.cell).copied() {
            bounds
        } else if let Some(bounds) =
            LayoutIndex::rebuild_hierarchical_for_cell(document, instance.cell, None).bounds()
        {
            child_bounds_cache.insert(instance.cell, bounds);
            bounds
        } else {
            continue;
        };
        let array = instance.array.normalized();
        for row in 0..array.rows {
            for column in 0..array.columns {
                if boxes.len() >= MAX_HIERARCHY_BOX_OVERLAY_INSTANCES {
                    return boxes;
                }
                let transform = Transform::from_translation(array.element_offset(column, row))
                    .compose(instance.transform);
                let bounds = transform.apply_rect(child_bounds);
                if bounds.intersects(viewport) {
                    boxes.push(bounds);
                }
            }
        }
    }
    boxes
}

pub(crate) fn add_layout_grid_overlay_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    app: &GlassworksApp,
    size: UiSize,
    ui_scale: UiScale,
) {
    let grid = app.layout_grid_step_for_zoom().max(1);
    let viewport = app.layout_viewport_for_size(size);
    let min_x = viewport.min.x.div_euclid(grid) * grid;
    let max_x = viewport.max.x.div_euclid(grid) * grid + grid;
    let min_y = viewport.min.y.div_euclid(grid) * grid;
    let max_y = viewport.max.y.div_euclid(grid) * grid + grid;
    let minor = StrokeStyle::new(ColorRgba::new(82, 94, 98, 46), ui_scale.value(1.0));
    let major = StrokeStyle::new(ColorRgba::new(116, 132, 138, 82), ui_scale.value(1.0));
    let major_step = grid.saturating_mul(5).max(grid);

    for x in (min_x..=max_x).step_by(grid as usize) {
        let stroke = if x == 0 || x.rem_euclid(major_step) == 0 {
            major
        } else {
            minor
        };
        primitives.push(ScenePrimitive::Line {
            from: app.layout_world_to_canvas(Point::new(x, min_y), size),
            to: app.layout_world_to_canvas(Point::new(x, max_y), size),
            stroke,
        });
    }
    for y in (min_y..=max_y).step_by(grid as usize) {
        let stroke = if y == 0 || y.rem_euclid(major_step) == 0 {
            major
        } else {
            minor
        };
        primitives.push(ScenePrimitive::Line {
            from: app.layout_world_to_canvas(Point::new(min_x, y), size),
            to: app.layout_world_to_canvas(Point::new(max_x, y), size),
            stroke,
        });
    }
}

pub(crate) fn add_layout_scale_bar_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    app: &GlassworksApp,
    size: UiSize,
    ui_scale: UiScale,
) {
    if size.width < ui_scale.value(160.0) || size.height < ui_scale.value(80.0) {
        return;
    }
    let length_dbu = layout_scale_bar_length_dbu(app.layout_zoom);
    let length_px = length_dbu as f32 * app.layout_zoom;
    if length_px < ui_scale.value(24.0) || length_px > size.width - ui_scale.value(40.0) {
        return;
    }

    let left = ui_scale.value(18.0);
    let baseline = size.height - ui_scale.value(22.0);
    let right = left + length_px;
    let tick_top = baseline - ui_scale.value(8.0);
    let bg = UiRect::new(
        left - ui_scale.value(10.0),
        tick_top - ui_scale.value(26.0),
        length_px + ui_scale.value(20.0),
        ui_scale.value(44.0),
    );
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        bg,
        ColorRgba::new(6, 8, 10, 178),
    )));

    let stroke = StrokeStyle::new(ColorRgba::new(238, 242, 232, 255), ui_scale.value(2.0));
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(left, baseline),
        to: UiPoint::new(right, baseline),
        stroke,
    });
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(left, tick_top),
        to: UiPoint::new(left, baseline),
        stroke,
    });
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(right, tick_top),
        to: UiPoint::new(right, baseline),
        stroke,
    });
    primitives.push(ScenePrimitive::Text(
        PaintText::new(
            app.format_layout_length(length_dbu as f64),
            UiRect::new(
                left - ui_scale.value(10.0),
                tick_top - ui_scale.value(22.0),
                length_px + ui_scale.value(20.0),
                ui_scale.value(18.0),
            ),
            text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT),
        )
        .horizontal_align(TextHorizontalAlign::Center)
        .vertical_align(TextVerticalAlign::Bottom)
        .multiline(false),
    ));
}

pub(crate) fn layout_scale_bar_length_dbu(zoom: f32) -> Coord {
    nice_layout_scale_length_dbu(120.0 / zoom.max(LAYOUT_MIN_ZOOM))
}

pub(crate) fn nice_layout_scale_length_dbu(target_dbu: f32) -> Coord {
    if !target_dbu.is_finite() || target_dbu <= 1.0 {
        return 1;
    }
    let exponent = 10f32.powf(target_dbu.log10().floor());
    let normalized = target_dbu / exponent;
    let multiplier = if normalized < 1.5 {
        1.0
    } else if normalized < 3.5 {
        2.0
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };
    (multiplier * exponent).round().max(1.0) as Coord
}

pub(crate) fn add_overlay_rect_outline(
    primitives: &mut Vec<ScenePrimitive>,
    app: &GlassworksApp,
    size: UiSize,
    bounds: Rect,
    stroke: StrokeStyle,
) {
    let min = app.layout_world_to_canvas(bounds.min, size);
    let max = app.layout_world_to_canvas(bounds.max, size);
    let a = UiPoint::new(min.x, min.y);
    let b = UiPoint::new(max.x, min.y);
    let c = UiPoint::new(max.x, max.y);
    let d = UiPoint::new(min.x, max.y);
    primitives.push(ScenePrimitive::Line {
        from: a,
        to: b,
        stroke,
    });
    primitives.push(ScenePrimitive::Line {
        from: b,
        to: c,
        stroke,
    });
    primitives.push(ScenePrimitive::Line {
        from: c,
        to: d,
        stroke,
    });
    primitives.push(ScenePrimitive::Line {
        from: d,
        to: a,
        stroke,
    });
}

pub(crate) fn shape_kind_label(kind: &ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Rectangle(_) => "rectangle",
        ShapeKind::Polygon(_) => "polygon",
        ShapeKind::Path { .. } => "path",
        ShapeKind::Via { .. } => "via",
        ShapeKind::Label { .. } => "label",
        ShapeKind::Measurement { .. } => "measurement",
    }
}

pub(crate) fn compact_layer_label(layer_id: LayerId, name: &str) -> String {
    let mut label = name.chars().take(12).collect::<String>();
    if name.chars().count() > 12 {
        label.push_str("...");
    }
    format!("L{} {label}", layer_id.0)
}

pub(crate) fn layout_layer_display_name(document: &Document, layer_id: LayerId) -> String {
    document
        .layer(layer_id)
        .map(|layer| compact_layer_label(layer.id, &layer.name))
        .unwrap_or_else(|| format!("L{}", layer_id.0))
}

pub(crate) fn layout_cell_display_name(document: &Document, cell_id: CellId) -> String {
    document
        .cell(cell_id)
        .map(|cell| format!("C{} {}", cell.id.0, compact_button_label(&cell.name, 14)))
        .unwrap_or_else(|| format!("C{}", cell_id.0))
}

pub(crate) fn compact_cell_button_label(cell: &Cell, document_top_cell: CellId) -> String {
    if cell.id == document_top_cell {
        "Doc top".to_string()
    } else {
        format!("C{} {}", cell.id.0, compact_button_label(&cell.name, 8))
    }
}

pub(crate) fn rect_summary(rect: Rect) -> String {
    format!(
        "{},{} {}x{}",
        rect.min.x,
        rect.min.y,
        rect.width(),
        rect.height()
    )
}

pub(crate) fn point_summary(point: Point) -> String {
    format!("{},{}", point.x, point.y)
}

pub(crate) fn layout_measurement_geometry(
    kind: &ShapeKind,
) -> Option<(Point, Point, &str, MeasurementMode)> {
    match kind {
        ShapeKind::Measurement { a, b, label, mode } => Some((*a, *b, label.as_str(), *mode)),
        _ => None,
    }
}

pub(crate) fn layout_measurement_mode_endpoint(
    start: Point,
    end: Point,
    mode: MeasurementMode,
) -> Point {
    match mode {
        MeasurementMode::Direct | MeasurementMode::Manhattan => end,
        MeasurementMode::Horizontal => Point::new(end.x, start.y),
        MeasurementMode::Vertical => Point::new(start.x, end.y),
    }
}

pub(crate) fn layout_measurement_mode_length(a: Point, b: Point, mode: MeasurementMode) -> f64 {
    match mode {
        MeasurementMode::Direct => a.distance_to(b),
        MeasurementMode::Horizontal => (b.x - a.x).unsigned_abs() as f64,
        MeasurementMode::Vertical => (b.y - a.y).unsigned_abs() as f64,
        MeasurementMode::Manhattan => {
            (b.x - a.x).unsigned_abs() as f64 + (b.y - a.y).unsigned_abs() as f64
        }
    }
}

pub(crate) fn layout_measurement_label(label: &str) -> String {
    if label.trim().is_empty() {
        "Unlabeled".to_string()
    } else {
        label.to_string()
    }
}

pub(crate) fn layout_measurement_label_for_button(kind: &ShapeKind) -> String {
    layout_measurement_geometry(kind)
        .map(|(_, _, label, _)| {
            let label = layout_measurement_label(label);
            if label == "Unlabeled" {
                String::new()
            } else {
                label
            }
        })
        .unwrap_or_default()
}

pub(crate) fn layout_measurement_delta_label(app: &GlassworksApp, a: Point, b: Point) -> String {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    format!(
        "dx={} dy={}",
        app.format_layout_length(dx.unsigned_abs() as f64),
        app.format_layout_length(dy.unsigned_abs() as f64)
    )
}

pub(crate) fn layout_measurement_angle_label(a: Point, b: Point) -> String {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    if dx == 0.0 && dy == 0.0 {
        return "0.0 deg".to_string();
    }
    format!("{:.1} deg", dy.atan2(dx).to_degrees())
}

pub(crate) fn layer_color(color: [f32; 4], alpha: u8) -> ColorRgba {
    ColorRgba::new(
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        alpha,
    )
}
