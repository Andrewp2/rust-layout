#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_measurement_entries(
    app: &GlassworksApp,
) -> Vec<(ShapeOccurrenceId, CellId, Shape)> {
    layout_measurement_entries_with_options(app, true, true, MAX_LAYOUT_MEASUREMENTS)
}

pub(crate) fn layout_measurement_browser_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if app.layout_measurement_filter == LayoutMeasurementFilter::All && search.is_empty() {
        return format!("Measurements ({listed_count} {row_label})");
    }
    let mut context = Vec::new();
    if app.layout_measurement_filter != LayoutMeasurementFilter::All {
        context.push(app.layout_measurement_filter.label().to_string());
    }
    if !search.is_empty() {
        context.push(format!("search {}", compact_button_label(search, 24)));
    }
    format!(
        "Measurements - {} ({listed_count} {row_label})",
        context.join(" / ")
    )
}

pub(crate) fn layout_measurement_filter_status_label(filter: LayoutMeasurementFilter) -> String {
    match filter {
        LayoutMeasurementFilter::All => "rulers".to_string(),
        _ => format!("{} rulers", filter.label()),
    }
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
            if apply_filter
                && !layout_measurement_filter_matches_filter(
                    app,
                    app.layout_measurement_filter,
                    &occurrence,
                    &shape,
                )
            {
                return;
            }
            if let Some(query) = search.as_deref()
                && !layout_measurement_matches_search(
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
    sort_layout_measurement_entries(document, app.layout_measurement_browser_sort, &mut matches);
    matches.truncate(limit);
    matches
}

pub(crate) fn sort_layout_measurement_entries(
    document: &Document,
    sort: LayoutMeasurementBrowserSort,
    entries: &mut [(ShapeOccurrenceId, CellId, Shape)],
) {
    entries.sort_by(|left, right| match sort {
        LayoutMeasurementBrowserSort::Id => layout_measurement_entry_id_order(left, right),
        LayoutMeasurementBrowserSort::Length => {
            let left_length = layout_measurement_entry_length(&left.2);
            let right_length = layout_measurement_entry_length(&right.2);
            right_length
                .partial_cmp(&left_length)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| layout_measurement_entry_id_order(left, right))
        }
        LayoutMeasurementBrowserSort::Angle => layout_measurement_entry_angle(&left.2)
            .total_cmp(&layout_measurement_entry_angle(&right.2))
            .then_with(|| layout_measurement_entry_id_order(left, right)),
        LayoutMeasurementBrowserSort::Mode => layout_measurement_entry_mode_label(&left.2)
            .cmp(layout_measurement_entry_mode_label(&right.2))
            .then_with(|| layout_measurement_entry_id_order(left, right)),
        LayoutMeasurementBrowserSort::Layer => layout_measurement_layer_sort_key(document, &left.2)
            .cmp(&layout_measurement_layer_sort_key(document, &right.2))
            .then_with(|| layout_measurement_entry_id_order(left, right)),
        LayoutMeasurementBrowserSort::SourceCell => {
            layout_measurement_source_cell_sort_key(document, left.1)
                .cmp(&layout_measurement_source_cell_sort_key(document, right.1))
                .then_with(|| layout_measurement_entry_id_order(left, right))
        }
    });
}

fn layout_measurement_entry_id_order(
    left: &(ShapeOccurrenceId, CellId, Shape),
    right: &(ShapeOccurrenceId, CellId, Shape),
) -> std::cmp::Ordering {
    left.2
        .id
        .cmp(&right.2.id)
        .then_with(|| left.1.cmp(&right.1))
        .then_with(|| left.0.instance_path.cmp(&right.0.instance_path))
}

fn layout_measurement_entry_length(shape: &Shape) -> f64 {
    layout_measurement_geometry(&shape.kind)
        .map(|(a, b, _, mode)| layout_measurement_mode_length(a, b, mode))
        .unwrap_or(0.0)
}

fn layout_measurement_entry_angle(shape: &Shape) -> f64 {
    layout_measurement_geometry(&shape.kind)
        .map(|(a, b, _, _)| {
            let dx = (b.x - a.x) as f64;
            let dy = (b.y - a.y) as f64;
            if dx == 0.0 && dy == 0.0 {
                0.0
            } else {
                dy.atan2(dx).to_degrees()
            }
        })
        .unwrap_or(0.0)
}

fn layout_measurement_entry_mode_label(shape: &Shape) -> &'static str {
    layout_measurement_geometry(&shape.kind)
        .map(|(_, _, _, mode)| mode.label())
        .unwrap_or("")
}

fn layout_measurement_layer_sort_key<'a>(
    document: &'a Document,
    shape: &Shape,
) -> (i32, LayerId, &'a str) {
    document
        .layer(shape.layer)
        .map(|layer| (layer.display_order, layer.id, layer.name.as_str()))
        .unwrap_or((i32::MAX, shape.layer, ""))
}

fn layout_measurement_source_cell_sort_key<'a>(
    document: &'a Document,
    source_cell: CellId,
) -> (&'a str, CellId) {
    document
        .cell(source_cell)
        .map(|cell| (cell.name.as_str(), cell.id))
        .unwrap_or(("", source_cell))
}

pub(crate) fn layout_measurement_matches_search(
    document: &Document,
    occurrence: &ShapeOccurrenceId,
    source_cell: CellId,
    shape: &Shape,
    query_lower: &str,
) -> bool {
    if layout_measurement_selector_matches_search(
        document,
        occurrence,
        source_cell,
        shape,
        query_lower,
    ) {
        return true;
    }
    layout_shape_matches_search(document, occurrence, source_cell, shape, query_lower)
}

pub(crate) fn layout_measurement_selector_matches_search(
    document: &Document,
    occurrence: &ShapeOccurrenceId,
    source_cell: CellId,
    shape: &Shape,
    query_lower: &str,
) -> bool {
    let Some((key_query, value_query)) = query_lower.split_once('=') else {
        return false;
    };
    let key_query = key_query.trim();
    let value_query = value_query.trim();
    if key_query.is_empty() && value_query.is_empty() {
        return true;
    }
    let Some((a, b, label, mode)) = layout_measurement_geometry(&shape.kind) else {
        return false;
    };
    let source_cell_label = document
        .cell(source_cell)
        .map(|cell| format!("C{} {}", cell.id.0, cell.name))
        .unwrap_or_else(|| format!("C{}", source_cell.0));
    let layer_label = document
        .layer(shape.layer)
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| format!("L{}", shape.layer.0));
    let fields = [
        ("id", shape.id.0.to_string()),
        ("measurement", shape.id.0.to_string()),
        ("ruler", shape.id.0.to_string()),
        ("label", layout_measurement_label(label)),
        ("mode", mode.label().to_string()),
        ("source_cell", source_cell_label.clone()),
        ("cell", source_cell_label),
        ("source_cell_id", source_cell.0.to_string()),
        ("layer", layer_label),
        ("layer_id", shape.layer.0.to_string()),
        ("occurrence", layout_occurrence_label(occurrence)),
        ("start", point_summary(a)),
        ("end", point_summary(b)),
        ("from", point_summary(a)),
        ("to", point_summary(b)),
        ("endpoint_a", point_summary(a)),
        ("endpoint_b", point_summary(b)),
        ("x1", a.x.to_string()),
        ("y1", a.y.to_string()),
        ("x2", b.x.to_string()),
        ("y2", b.y.to_string()),
        ("delta", format!("{},{}", b.x - a.x, b.y - a.y)),
        ("dx", (b.x - a.x).to_string()),
        ("dy", (b.y - a.y).to_string()),
        (
            "length",
            format!("{:.3}", layout_measurement_mode_length(a, b, mode)),
        ),
        (
            "length_dbu",
            format!("{:.0}", layout_measurement_mode_length(a, b, mode)),
        ),
        ("angle", layout_measurement_angle_label(a, b)),
        ("bounds", rect_summary(shape.kind.bounds())),
    ];
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_measurement_filter_matches_filter(
    app: &GlassworksApp,
    filter: LayoutMeasurementFilter,
    occurrence: &ShapeOccurrenceId,
    shape: &Shape,
) -> bool {
    let Some((_, _, _, mode)) = layout_measurement_geometry(&shape.kind) else {
        return false;
    };
    match filter {
        LayoutMeasurementFilter::All => true,
        LayoutMeasurementFilter::Selected => {
            app.selected_layout_occurrence.as_ref() == Some(occurrence)
                || (app.selected_layout_occurrence.is_none()
                    && app.selected_layout_shape == Some(shape.id))
        }
        LayoutMeasurementFilter::ActiveLayer => shape.layer == app.active_layer,
        LayoutMeasurementFilter::Direct => mode == MeasurementMode::Direct,
        LayoutMeasurementFilter::Horizontal => mode == MeasurementMode::Horizontal,
        LayoutMeasurementFilter::Vertical => mode == MeasurementMode::Vertical,
        LayoutMeasurementFilter::Manhattan => mode == MeasurementMode::Manhattan,
    }
}

pub(crate) fn layout_measurement_filter_count(
    app: &GlassworksApp,
    filter: LayoutMeasurementFilter,
) -> usize {
    let document = &app.workspace.document;
    let search = layout_browser_search_query_lower(app);
    let mut count = 0usize;
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
            if !layout_measurement_filter_matches_filter(app, filter, &occurrence, &shape) {
                return;
            }
            if let Some(query) = search.as_deref()
                && !layout_measurement_matches_search(
                    document,
                    &occurrence,
                    view.source_cell,
                    &shape,
                    query,
                )
            {
                return;
            }
            count += 1;
        },
    );
    count
}

pub(crate) fn layout_measurement_filter_button_label(
    app: &GlassworksApp,
    filter: LayoutMeasurementFilter,
) -> String {
    format!(
        "{} ({})",
        filter.label(),
        layout_measurement_filter_count(app, filter)
    )
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
            "Sort".to_string(),
            app.layout_measurement_browser_sort.label().to_string(),
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

pub(crate) fn layout_reference_image_entries(
    app: &GlassworksApp,
) -> Vec<(usize, &ReferenceImageOverlay)> {
    let search = layout_browser_search_query_lower(app);
    app.workspace
        .document
        .reference_images
        .iter()
        .enumerate()
        .filter(|(index, image)| {
            search
                .as_deref()
                .is_none_or(|query| layout_reference_image_matches_search(*index, image, query))
        })
        .collect()
}

pub(crate) fn layout_reference_image_matches_search(
    index: usize,
    image: &ReferenceImageOverlay,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    if layout_reference_image_selector_matches_search(index, image, &query_lower) {
        return true;
    }
    let mut fields = vec![
        "reference image".to_string(),
        "ref image".to_string(),
        format!("reference {}", index + 1),
        format!("image {}", index + 1),
        image.display_name().to_string(),
        image.id.clone(),
        image.name.clone(),
        image.uri.clone(),
        reference_image_source_label(image),
        if image.visible { "visible" } else { "hidden" }.to_string(),
        reference_image_opacity_label(image.opacity),
        layout_reference_image_alignment_label(image),
        rect_summary(image.bounds),
        reference_image_pixel_size_label(image),
        format!("{} landmarks", image.landmarks.len()),
    ];
    for landmark in &image.landmarks {
        push_layout_search_field(&mut fields, landmark.name.clone());
        push_layout_search_field(&mut fields, format!("landmark {}", landmark.name));
        push_layout_search_field(
            &mut fields,
            format!("image {},{}", landmark.image.x, landmark.image.y),
        );
        push_layout_search_field(
            &mut fields,
            format!("layout {},{}", landmark.layout.x, landmark.layout.y),
        );
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_reference_image_selector_matches_search(
    index: usize,
    image: &ReferenceImageOverlay,
    query_lower: &str,
) -> bool {
    let Some((key_query, value_query)) = query_lower.split_once('=') else {
        return false;
    };
    let key_query = key_query.trim();
    let value_query = value_query.trim();
    if key_query.is_empty() && value_query.is_empty() {
        return true;
    }
    let index = index + 1;
    let mut fields = vec![
        ("index", index.to_string()),
        ("image", index.to_string()),
        ("reference_image", index.to_string()),
        ("id", image.id.clone()),
        ("name", image.name.clone()),
        ("display_name", image.display_name().to_string()),
        ("uri", image.uri.clone()),
        ("source", reference_image_source_label(image)),
        ("visible", image.visible.to_string()),
        ("opacity", reference_image_opacity_label(image.opacity)),
        (
            "opacity_percent",
            reference_image_opacity_label(image.opacity).replace('%', ""),
        ),
        ("alignment", layout_reference_image_alignment_label(image)),
        ("bounds", rect_summary(image.bounds)),
        ("landmarks", image.landmarks.len().to_string()),
        ("landmark_count", image.landmarks.len().to_string()),
    ];
    if let Some(size) = image.pixel_size {
        fields.push(("pixel_size", format!("{}x{}", size.width, size.height)));
        fields.push(("pixel_width", size.width.to_string()));
        fields.push(("pixel_height", size.height.to_string()));
    }
    for landmark in &image.landmarks {
        fields.push(("landmark", landmark.name.clone()));
        fields.push(("landmark_name", landmark.name.clone()));
        fields.push((
            "landmark_image",
            format!("{},{}", landmark.image.x, landmark.image.y),
        ));
        fields.push((
            "landmark_layout",
            format!("{},{}", landmark.layout.x, landmark.layout.y),
        ));
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_reference_image_alignment_label(image: &ReferenceImageOverlay) -> String {
    match image.aligned_bounds_from_landmarks() {
        Ok(Some(bounds)) if bounds == image.bounds => "aligned".to_string(),
        Ok(Some(_)) => "landmarks pending".to_string(),
        Ok(None) => "manual bounds".to_string(),
        Err(_) => "landmark warning".to_string(),
    }
}

pub(crate) fn layout_reference_image_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let entries = layout_reference_image_entries(app);
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
    if !app.layout_browser_search.trim().is_empty() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed images".to_string(),
            format!(
                "{} / {} total",
                entries.len(),
                document.reference_images.len()
            ),
        ));
    }
    for (_, image) in entries.into_iter().take(4) {
        let alignment = layout_reference_image_alignment_label(image);
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
        rows.push((
            format!("{} source", compact_button_label(image.display_name(), 12)),
            reference_image_source_label(image),
        ));
        rows.push((
            format!("{} pixels", compact_button_label(image.display_name(), 12)),
            reference_image_pixel_size_label(image),
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

pub(crate) fn reference_image_source_label(image: &ReferenceImageOverlay) -> String {
    if !image.uri.trim().is_empty() {
        compact_button_label(&image.uri, 64)
    } else if !image.id.trim().is_empty() {
        compact_button_label(&image.id, 64)
    } else {
        "None".to_string()
    }
}

pub(crate) fn reference_image_pixel_size_label(image: &ReferenceImageOverlay) -> String {
    image
        .pixel_size
        .filter(|size| size.width > 0 && size.height > 0)
        .map(|size| format!("{}x{} px", size.width, size.height))
        .unwrap_or_else(|| "Unknown".to_string())
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
