#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_measurement_entries(
    app: &GlassworksApp,
) -> Vec<(ShapeOccurrenceId, CellId, Shape)> {
    layout_measurement_entries_with_options(app, true, true, MAX_LAYOUT_MEASUREMENTS)
}

pub(crate) fn layout_visible_measurement_count(app: &GlassworksApp) -> usize {
    layout_measurement_entries_with_options(app, false, false, usize::MAX).len()
}

pub(crate) fn selected_visible_layout_measurement(
    app: &GlassworksApp,
) -> Option<(ShapeOccurrenceId, CellId, Shape)> {
    let selected = app.selected_layout_occurrence.as_ref()?;
    layout_measurement_entries_with_options(app, false, false, usize::MAX)
        .into_iter()
        .find(|(occurrence, _, _)| occurrence == selected)
}

pub(crate) fn layout_measurement_entries_with_options(
    app: &GlassworksApp,
    apply_search: bool,
    apply_filter: bool,
    limit: usize,
) -> Vec<(ShapeOccurrenceId, CellId, Shape)> {
    let document = &app.workspace.document;
    let search = apply_search
        .then(|| layout_browser_search_query_lower(app))
        .flatten();
    let mut matches = Vec::new();
    document.for_each_visible_flattened_shape_view_for_cell_with_depth_range(
        app.layout_view_top_cell,
        app.layout_hierarchy_min_depth as usize,
        layout_traversal_max_depth(
            app.layout_hierarchy_depth.max_depth(),
            &app.layout_layer_depth_overrides,
        ),
        |occurrence, view| {
            if layout_occurrence_hidden_by_cells(
                document,
                app.layout_view_top_cell,
                &occurrence,
                &app.layout_hidden_cells,
            ) || !layout_occurrence_in_layer_depth(
                view.shape.layer,
                occurrence.hierarchy_depth(),
                app.layout_hierarchy_depth.max_depth(),
                &app.layout_layer_depth_overrides,
            ) {
                return;
            }
            let shape = view.shape.to_shape();
            if !matches!(shape.kind, ShapeKind::Measurement { .. }) {
                return;
            }
            if apply_filter && !layout_measurement_filter_matches(app, &shape) {
                return;
            }
            if let Some(query) = search.as_deref()
                && !layout_shape_matches_search(
                    document,
                    &occurrence,
                    view.source_cell,
                    &shape,
                    query,
                )
            {
                return;
            }
            matches.push((occurrence.clone(), view.source_cell, shape));
        },
    );
    matches.sort_by(|left, right| {
        left.2
            .id
            .cmp(&right.2.id)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.0.instance_path.cmp(&right.0.instance_path))
    });
    matches.truncate(limit);
    matches
}

pub(crate) fn layout_measurement_filter_matches(app: &GlassworksApp, shape: &Shape) -> bool {
    let Some((_, _, _, mode)) = layout_measurement_geometry(&shape.kind) else {
        return false;
    };
    match app.layout_measurement_filter {
        LayoutMeasurementFilter::All => true,
        LayoutMeasurementFilter::ActiveLayer => shape.layer == app.active_layer,
        LayoutMeasurementFilter::Direct => mode == MeasurementMode::Direct,
        LayoutMeasurementFilter::Horizontal => mode == MeasurementMode::Horizontal,
        LayoutMeasurementFilter::Vertical => mode == MeasurementMode::Vertical,
        LayoutMeasurementFilter::Manhattan => mode == MeasurementMode::Manhattan,
    }
}

pub(crate) fn layout_measurement_total_length(
    entries: &[(ShapeOccurrenceId, CellId, Shape)],
) -> f64 {
    entries
        .iter()
        .filter_map(|(_, _, shape)| layout_measurement_geometry(&shape.kind))
        .map(|(a, b, _, mode)| layout_measurement_mode_length(a, b, mode))
        .sum()
}

pub(crate) fn layout_measurement_mode_counts(
    entries: &[(ShapeOccurrenceId, CellId, Shape)],
) -> String {
    let mut parts = Vec::new();
    for mode in MeasurementMode::ALL {
        let count = entries
            .iter()
            .filter(|(_, _, shape)| {
                layout_measurement_geometry(&shape.kind)
                    .is_some_and(|(_, _, _, measurement_mode)| measurement_mode == mode)
            })
            .count();
        if count > 0 {
            parts.push(format!("{} {count}", mode.label()));
        }
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join(", ")
    }
}

pub(crate) fn layout_measurement_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_measurement_entries(app);
    let selected_entry = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|selected| {
            entries
                .iter()
                .find(|(occurrence, _, _)| occurrence == selected)
        })
        .or_else(|| entries.first());
    let selected = selected_entry
        .map(|(occurrence, _, shape)| {
            format!(
                "#{} {}",
                shape.id.0,
                layout_measurement_occurrence_length_label(app, occurrence, shape)
            )
        })
        .unwrap_or_else(|| "None".to_string());
    let mut rows = vec![
        ("View top".to_string(), layout_view_top_cell_name(app)),
        (
            "Visible measurements".to_string(),
            layout_visible_measurement_count(app).to_string(),
        ),
        ("Listed".to_string(), entries.len().to_string()),
        (
            "Listed total length".to_string(),
            app.format_layout_length(layout_measurement_total_length(&entries)),
        ),
        (
            "Listed ruler modes".to_string(),
            layout_measurement_mode_counts(&entries),
        ),
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
        (
            "Filter".to_string(),
            app.layout_measurement_filter.label().to_string(),
        ),
        (
            "Ruler mode".to_string(),
            app.layout_measurement_mode.label().to_string(),
        ),
        ("Selected".to_string(), selected),
    ];
    if let Some((occurrence, source_cell, shape)) = selected_entry {
        rows.extend(layout_measurement_property_rows(
            app,
            occurrence,
            *source_cell,
            shape,
        ));
    }
    rows
}

pub(crate) fn reference_image_corner_landmarks(
    image: &ReferenceImageOverlay,
    layout_bounds: Rect,
) -> Option<Vec<ReferenceImageLandmark>> {
    if layout_bounds.width() <= 0 || layout_bounds.height() <= 0 {
        return None;
    }
    let size = image.pixel_size?;
    if size.width <= 0 || size.height <= 0 {
        return None;
    }
    Some(vec![
        ReferenceImageLandmark::new("lower_left", Point::new(0, size.height), layout_bounds.min),
        ReferenceImageLandmark::new("upper_right", Point::new(size.width, 0), layout_bounds.max),
    ])
}

pub(crate) fn reference_image_opacity_label(opacity: u8) -> String {
    let percent = ((opacity as u16 * 100) + 127) / 255;
    format!("{percent}%")
}

pub(crate) fn layout_reference_image_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let visible = document
        .reference_images
        .iter()
        .filter(|image| image.visible && image.opacity > 0)
        .count();
    let landmarks = document
        .reference_images
        .iter()
        .map(|image| image.landmarks.len())
        .sum::<usize>();
    let mut rows = vec![
        (
            "Images".to_string(),
            document.reference_images.len().to_string(),
        ),
        ("Visible".to_string(), visible.to_string()),
        (
            "Overlay".to_string(),
            if app.show_reference_images {
                "shown"
            } else {
                "hidden"
            }
            .to_string(),
        ),
        ("Landmarks".to_string(), landmarks.to_string()),
    ];
    for image in document.reference_images.iter().take(4) {
        let alignment = match image.aligned_bounds_from_landmarks() {
            Ok(Some(bounds)) if bounds == image.bounds => "aligned",
            Ok(Some(_)) => "landmarks pending",
            Ok(None) => "manual bounds",
            Err(_) => "landmark warning",
        };
        rows.push((
            compact_button_label(image.display_name(), 18),
            format!(
                "{}; {}; opacity {}; {}; {} landmark(s)",
                rect_summary(image.bounds),
                if image.visible { "visible" } else { "hidden" },
                reference_image_opacity_label(image.opacity),
                alignment,
                image.landmarks.len()
            ),
        ));
        for landmark in image.landmarks.iter().take(2) {
            rows.push((
                format!("{} mark", compact_button_label(&landmark.name, 10)),
                format!(
                    "img {},{} -> layout {},{}",
                    landmark.image.x, landmark.image.y, landmark.layout.x, landmark.layout.y
                ),
            ));
        }
    }
    rows
}

pub(crate) fn layout_measurement_property_rows(
    app: &GlassworksApp,
    occurrence: &ShapeOccurrenceId,
    source_cell: CellId,
    shape: &Shape,
) -> Vec<(String, String)> {
    let Some((a, b, label, mode)) = layout_measurement_geometry(&shape.kind) else {
        return Vec::new();
    };
    let layer = layout_layer_display_name(&app.workspace.document, shape.layer);
    let source_cell = layout_cell_display_name(&app.workspace.document, source_cell);
    let bounds = rect_summary(shape.kind.bounds());
    let length = app.format_layout_length(layout_measurement_mode_length(a, b, mode));
    match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Measurement id".to_string(), format!("#{}", shape.id.0)),
            ("Label".to_string(), layout_measurement_label(label)),
            ("Mode".to_string(), mode.label().to_string()),
            ("Length".to_string(), length),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Measurement id".to_string(), format!("#{}", shape.id.0)),
            ("Mode".to_string(), mode.label().to_string()),
            ("Length".to_string(), length),
            (
                "Delta".to_string(),
                layout_measurement_delta_label(app, a, b),
            ),
            ("Angle".to_string(), layout_measurement_angle_label(a, b)),
            ("Endpoint A".to_string(), point_summary(a)),
            ("Endpoint B".to_string(), point_summary(b)),
            ("Bounds".to_string(), bounds),
        ],
        LayoutBrowserColumnSet::Relations => vec![
            ("Measurement id".to_string(), format!("#{}", shape.id.0)),
            ("Layer".to_string(), layer),
            ("Source cell".to_string(), source_cell),
            (
                "Occurrence".to_string(),
                layout_occurrence_label(occurrence),
            ),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Measurement id".to_string(), format!("#{}", shape.id.0)),
            ("Label".to_string(), layout_measurement_label(label)),
            ("Mode".to_string(), mode.label().to_string()),
            ("Length".to_string(), length),
            (
                "Delta".to_string(),
                layout_measurement_delta_label(app, a, b),
            ),
            ("Angle".to_string(), layout_measurement_angle_label(a, b)),
            ("Endpoint A".to_string(), point_summary(a)),
            ("Endpoint B".to_string(), point_summary(b)),
            ("Layer".to_string(), layer),
            ("Source cell".to_string(), source_cell),
            (
                "Occurrence".to_string(),
                layout_occurrence_label(occurrence),
            ),
            ("Bounds".to_string(), bounds),
        ],
    }
}

pub(crate) fn layout_measurement_shape_property_rows(
    app: &GlassworksApp,
    kind: &ShapeKind,
) -> Vec<(String, String)> {
    let Some((a, b, label, mode)) = layout_measurement_geometry(kind) else {
        return Vec::new();
    };
    vec![
        ("Ruler label".to_string(), layout_measurement_label(label)),
        ("Ruler mode".to_string(), mode.label().to_string()),
        (
            "Ruler length".to_string(),
            app.format_layout_length(layout_measurement_mode_length(a, b, mode)),
        ),
        (
            "Ruler delta".to_string(),
            layout_measurement_delta_label(app, a, b),
        ),
        (
            "Ruler angle".to_string(),
            layout_measurement_angle_label(a, b),
        ),
    ]
}

pub(crate) fn layout_measurement_browser_label(
    app: &GlassworksApp,
    occurrence: &ShapeOccurrenceId,
    shape: &Shape,
) -> String {
    compact_button_label(
        &format!(
            "#{} {} {}",
            shape.id.0,
            layout_measurement_occurrence_length_label(app, occurrence, shape),
            layout_measurement_label_for_button(&shape.kind)
        ),
        30,
    )
}

pub(crate) fn layout_measurement_occurrence_length_label(
    app: &GlassworksApp,
    occurrence: &ShapeOccurrenceId,
    shape: &Shape,
) -> String {
    layout_measurement_geometry(&shape.kind)
        .map(|(a, b, _, mode)| {
            format!(
                "{} {}",
                app.format_layout_length(layout_measurement_mode_length(a, b, mode)),
                if occurrence.is_top_level() {
                    "local"
                } else {
                    "inst"
                }
            )
        })
        .unwrap_or_else(|| "not a measurement".to_string())
}
