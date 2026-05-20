#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_shape_browser_entries(
    app: &GlassworksApp,
) -> Vec<(ShapeOccurrenceId, String)> {
    layout_shape_browser_entry_shapes_for_filter(
        app,
        app.layout_shape_browser_filter,
        true,
        MAX_LAYOUT_BROWSER_SHAPES,
    )
    .into_iter()
    .map(|(occurrence, source_cell, shape)| {
        (
            occurrence.clone(),
            layout_shape_browser_label(&app.workspace.document, &occurrence, source_cell, &shape),
        )
    })
    .collect()
}

pub(crate) fn layout_shape_browser_entry_shapes_for_filter(
    app: &GlassworksApp,
    filter: LayoutShapeBrowserFilter,
    apply_search: bool,
    limit: usize,
) -> Vec<(ShapeOccurrenceId, CellId, Shape)> {
    let document = &app.workspace.document;
    let search = apply_search
        .then(|| layout_browser_search_query_lower(app))
        .flatten();
    let selected_net_report = (filter == LayoutShapeBrowserFilter::SelectedNet)
        .then(|| app.connectivity_report().ok())
        .flatten();
    let selected_component = selected_net_report
        .as_ref()
        .and_then(|report| selected_layout_net_component_id(app, report));
    let selected_occurrence = app
        .selected_layout_occurrence
        .clone()
        .or_else(|| app.selected_layout_shape.map(ShapeOccurrenceId::top_level));
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
            if !layout_shape_browser_filter_matches(
                app,
                filter,
                selected_net_report.as_ref(),
                selected_component,
                selected_occurrence.as_ref(),
                &occurrence,
                &shape,
            ) {
                return;
            }
            if let Some(query) = search.as_deref()
                && if filter == LayoutShapeBrowserFilter::Properties {
                    !layout_shape_selector_matches_search(
                        document,
                        &occurrence,
                        view.source_cell,
                        &shape,
                        query,
                    ) && !layout_shape_property_selector_matches_search(&shape, query)
                } else {
                    !layout_shape_matches_search(
                        document,
                        &occurrence,
                        view.source_cell,
                        &shape,
                        query,
                    )
                }
            {
                return;
            }
            matches.push((occurrence.clone(), view.source_cell, shape));
        },
    );
    sort_layout_shape_browser_entries(document, app.layout_shape_browser_sort, &mut matches);
    matches.truncate(limit);
    matches
}

pub(crate) fn layout_shape_browser_filter_matches(
    app: &GlassworksApp,
    filter: LayoutShapeBrowserFilter,
    selected_net_report: Option<&ConnectivityReport>,
    selected_component: Option<usize>,
    selected_occurrence: Option<&ShapeOccurrenceId>,
    occurrence: &ShapeOccurrenceId,
    shape: &Shape,
) -> bool {
    match filter {
        LayoutShapeBrowserFilter::All => true,
        LayoutShapeBrowserFilter::ActiveLayer => shape.layer == app.active_layer,
        LayoutShapeBrowserFilter::Selected => selected_occurrence
            .as_ref()
            .is_some_and(|selected| *selected == occurrence),
        LayoutShapeBrowserFilter::SelectedNet => {
            selected_component.is_some()
                && selected_net_report
                    .and_then(|report| report.component_for_occurrence(occurrence))
                    == selected_component
        }
        LayoutShapeBrowserFilter::Properties => layout_shape_has_property_selector(shape),
        LayoutShapeBrowserFilter::Rectangles => matches!(shape.kind, ShapeKind::Rectangle(_)),
        LayoutShapeBrowserFilter::Polygons => matches!(shape.kind, ShapeKind::Polygon(_)),
        LayoutShapeBrowserFilter::Paths => matches!(shape.kind, ShapeKind::Path { .. }),
        LayoutShapeBrowserFilter::Vias => matches!(shape.kind, ShapeKind::Via { .. }),
        LayoutShapeBrowserFilter::Labels => matches!(shape.kind, ShapeKind::Label { .. }),
        LayoutShapeBrowserFilter::Measurements => {
            matches!(shape.kind, ShapeKind::Measurement { .. })
        }
    }
}

pub(crate) fn layout_shape_browser_filter_count(
    app: &GlassworksApp,
    filter: LayoutShapeBrowserFilter,
) -> usize {
    layout_shape_browser_entry_shapes_for_filter(app, filter, true, usize::MAX).len()
}

pub(crate) fn layout_shape_browser_filter_button_label(
    app: &GlassworksApp,
    filter: LayoutShapeBrowserFilter,
) -> String {
    format!(
        "{} ({})",
        filter.label(),
        layout_shape_browser_filter_count(app, filter)
    )
}

pub(crate) fn layout_shape_browser_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if app.layout_shape_browser_filter == LayoutShapeBrowserFilter::All && search.is_empty() {
        return format!("Shape Browser ({listed_count} {row_label})");
    }
    let mut context = Vec::new();
    if app.layout_shape_browser_filter != LayoutShapeBrowserFilter::All {
        context.push(app.layout_shape_browser_filter.label().to_string());
    }
    if !search.is_empty() {
        context.push(format!("search {}", compact_button_label(search, 24)));
    }
    format!(
        "Shape Browser - {} ({listed_count} {row_label})",
        context.join(" / ")
    )
}

pub(crate) fn layout_shape_browser_filter_status_label(filter: LayoutShapeBrowserFilter) -> String {
    match filter {
        LayoutShapeBrowserFilter::All => "shapes".to_string(),
        LayoutShapeBrowserFilter::ActiveLayer => "active-layer shapes".to_string(),
        LayoutShapeBrowserFilter::Selected => "selected shapes".to_string(),
        LayoutShapeBrowserFilter::SelectedNet => "selected-net shapes".to_string(),
        LayoutShapeBrowserFilter::Properties => "property-bearing shapes".to_string(),
        _ => format!("{} shapes", filter.label()),
    }
}

pub(crate) fn layout_shape_matches_search(
    document: &Document,
    occurrence: &ShapeOccurrenceId,
    source_cell: CellId,
    shape: &Shape,
    query_lower: &str,
) -> bool {
    if layout_shape_selector_matches_search(document, occurrence, source_cell, shape, query_lower) {
        return true;
    }
    let mut fields = vec![
        format!("#{}", shape.id.0),
        format!("shape {}", shape.id.0),
        shape_kind_label(&shape.kind).to_string(),
        format!("L{}", shape.layer.0),
        format!("layer {}", shape.layer.0),
        format!("C{}", source_cell.0),
        format!("cell {}", source_cell.0),
        layout_occurrence_label(occurrence),
        rect_summary(shape.kind.bounds()),
    ];
    if let Some(name) = &shape.name {
        push_layout_search_field(&mut fields, name.clone());
    }
    for (key, value) in &shape.properties {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
        push_layout_search_field(&mut fields, format!("property {key}"));
    }
    if let Some(net) = shape.net {
        push_layout_search_field(&mut fields, format!("net {}", net.0));
        push_layout_search_field(&mut fields, format!("N{}", net.0));
    }
    if let Some(layer) = document.layer(shape.layer) {
        push_layout_search_field(&mut fields, layer.name.clone());
    }
    if let Some(cell) = document.cell(source_cell) {
        push_layout_search_field(&mut fields, cell.name.clone());
    }
    match &shape.kind {
        ShapeKind::Label { text, .. } => push_layout_search_field(&mut fields, text.clone()),
        ShapeKind::Measurement { label, .. } => {
            push_layout_search_field(&mut fields, label.clone());
        }
        ShapeKind::Via { lower, upper, .. } => {
            push_layout_search_field(&mut fields, format!("L{}", lower.0));
            push_layout_search_field(&mut fields, format!("L{}", upper.0));
        }
        ShapeKind::Rectangle(_) | ShapeKind::Polygon(_) | ShapeKind::Path { .. } => {}
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_shape_selector_matches_search(
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
    let source_cell_label = document
        .cell(source_cell)
        .map(|cell| format!("C{} {}", cell.id.0, cell.name))
        .unwrap_or_else(|| format!("C{}", source_cell.0));
    let layer_label = |layer_id: LayerId| {
        document
            .layer(layer_id)
            .map(|layer| format!("L{} {}", layer.id.0, layer.name))
            .unwrap_or_else(|| format!("L{}", layer_id.0))
    };
    let kind_label = shape_kind_label(&shape.kind).to_string();
    let mut fields = vec![
        ("id", shape.id.0.to_string()),
        ("shape", shape.id.0.to_string()),
        ("shape_id", shape.id.0.to_string()),
        ("kind", kind_label.clone()),
        ("type", kind_label),
        ("layer", layer_label(shape.layer)),
        ("layer_id", shape.layer.0.to_string()),
        ("source_cell", source_cell_label.clone()),
        ("cell", source_cell_label),
        ("source_cell_id", source_cell.0.to_string()),
        ("cell_id", source_cell.0.to_string()),
        ("occurrence", layout_occurrence_label(occurrence)),
        ("bounds", rect_summary(shape.kind.bounds())),
    ];
    if let Some(name) = &shape.name {
        fields.push(("name", name.clone()));
    }
    if let Some(net) = shape.net {
        fields.push(("net", format!("N{} net {}", net.0, net.0)));
        fields.push(("net_id", net.0.to_string()));
    }
    match &shape.kind {
        ShapeKind::Label { text, .. } => {
            fields.push(("text", text.clone()));
            fields.push(("label", text.clone()));
        }
        ShapeKind::Measurement { label, .. } => {
            fields.push(("label", label.clone()));
            fields.push(("measurement_label", label.clone()));
        }
        ShapeKind::Via { lower, upper, .. } => {
            fields.push(("lower_layer", layer_label(*lower)));
            fields.push(("upper_layer", layer_label(*upper)));
        }
        ShapeKind::Rectangle(_) | ShapeKind::Polygon(_) | ShapeKind::Path { .. } => {}
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
        || layout_properties_selector_matches_search(&shape.properties, query_lower)
}

pub(crate) fn layout_shape_property_selector_matches_search(
    shape: &Shape,
    query_lower: &str,
) -> bool {
    if let Some((key_query, value_query)) = query_lower.split_once('=') {
        let key_query = key_query.trim();
        let value_query = value_query.trim();
        if key_query.is_empty() && value_query.is_empty() {
            return true;
        }
        if shape.name.as_deref().is_some_and(|name| {
            property_selector_key_value_matches("name", name, key_query, value_query)
        }) {
            return true;
        }
        if shape.net.is_some_and(|net| {
            let value = net.0.to_string();
            property_selector_key_value_matches("net", &value, key_query, value_query)
        }) {
            return true;
        }
        return layout_properties_selector_matches_search(&shape.properties, query_lower);
    }
    shape
        .name
        .as_deref()
        .is_some_and(|name| layout_search_matches_value(query_lower, name))
        || shape.net.is_some_and(|net| {
            layout_search_matches_value(query_lower, &format!("net {}", net.0))
                || layout_search_matches_value(query_lower, &format!("N{}", net.0))
        })
        || layout_properties_selector_matches_search(&shape.properties, query_lower)
}

pub(crate) fn layout_properties_selector_matches_search(
    properties: &BTreeMap<String, String>,
    query_lower: &str,
) -> bool {
    if let Some((key_query, value_query)) = query_lower.split_once('=') {
        let key_query = key_query.trim();
        let value_query = value_query.trim();
        if key_query.is_empty() && value_query.is_empty() {
            return true;
        }
        return properties.iter().any(|(key, value)| {
            property_selector_key_value_matches(key, value, key_query, value_query)
        });
    }
    properties.iter().any(|(key, value)| {
        layout_search_matches_value(query_lower, key)
            || layout_search_matches_value(query_lower, value)
            || layout_search_matches_value(query_lower, &format!("{key}={value}"))
    })
}

pub(crate) fn property_selector_key_value_matches(
    key: &str,
    value: &str,
    key_query: &str,
    value_query: &str,
) -> bool {
    (key_query.is_empty() || layout_search_matches_value(key_query, key))
        && (value_query.is_empty() || layout_search_matches_value(value_query, value))
}

pub(crate) fn layout_shape_has_property_selector(shape: &Shape) -> bool {
    shape
        .name
        .as_ref()
        .is_some_and(|name| !name.trim().is_empty())
        || shape.net.is_some()
        || !shape.properties.is_empty()
}

pub(crate) fn sort_layout_shape_browser_entries(
    document: &Document,
    sort: LayoutShapeBrowserSort,
    entries: &mut [(ShapeOccurrenceId, CellId, Shape)],
) {
    entries.sort_by(|left, right| {
        let left_layer_name = document
            .layer(left.2.layer)
            .map(|layer| layer.name.as_str());
        let right_layer_name = document
            .layer(right.2.layer)
            .map(|layer| layer.name.as_str());
        let left_cell_name = document.cell(left.1).map(|cell| cell.name.as_str());
        let right_cell_name = document.cell(right.1).map(|cell| cell.name.as_str());
        match sort {
            LayoutShapeBrowserSort::Id => left.2.id.cmp(&right.2.id),
            LayoutShapeBrowserSort::Layer => left
                .2
                .layer
                .cmp(&right.2.layer)
                .then_with(|| left_layer_name.cmp(&right_layer_name))
                .then_with(|| left.2.id.cmp(&right.2.id)),
            LayoutShapeBrowserSort::Cell => left
                .1
                .cmp(&right.1)
                .then_with(|| left_cell_name.cmp(&right_cell_name))
                .then_with(|| left.2.id.cmp(&right.2.id)),
            LayoutShapeBrowserSort::Kind => shape_kind_label(&left.2.kind)
                .cmp(shape_kind_label(&right.2.kind))
                .then_with(|| left.2.id.cmp(&right.2.id)),
            LayoutShapeBrowserSort::Area => layout_shape_sort_area2(&right.2.kind)
                .cmp(&layout_shape_sort_area2(&left.2.kind))
                .then_with(|| left.2.id.cmp(&right.2.id)),
        }
    });
}

pub(crate) fn layout_shape_sort_area2(kind: &ShapeKind) -> u128 {
    match kind {
        ShapeKind::Rectangle(rect) => layout_rect_area2(*rect),
        ShapeKind::Polygon(poly) => poly.signed_area2().unsigned_abs(),
        ShapeKind::Path { .. }
        | ShapeKind::Via { .. }
        | ShapeKind::Label { .. }
        | ShapeKind::Measurement { .. } => layout_rect_area2(kind.bounds()),
    }
}

pub(crate) fn layout_shape_browser_label(
    document: &Document,
    occurrence: &ShapeOccurrenceId,
    source_cell: CellId,
    shape: &Shape,
) -> String {
    let layer = document
        .layer(shape.layer)
        .map(|layer| format!("L{} {}", layer.id.0, compact_button_label(&layer.name, 7)))
        .unwrap_or_else(|| format!("L{}", shape.layer.0));
    let scope = if occurrence.is_top_level() {
        "local".to_string()
    } else {
        format!("C{} d{}", source_cell.0, occurrence.instance_path.len())
    };
    compact_button_label(
        &format!(
            "#{} {} {} {}",
            shape.id.0,
            shape_kind_label(&shape.kind),
            layer,
            scope
        ),
        30,
    )
}

pub(crate) fn layout_shape_browser_property_rows(
    app: &GlassworksApp,
    entries: &[(ShapeOccurrenceId, String)],
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let occurrence = app
        .selected_layout_occurrence
        .as_ref()
        .filter(|selected| {
            entries
                .iter()
                .any(|(occurrence, _)| occurrence == *selected)
        })
        .or_else(|| entries.first().map(|(occurrence, _)| occurrence));
    let Some(occurrence) = occurrence else {
        return Vec::new();
    };
    let Some(view) =
        document.shape_view_for_occurrence_from_cell(app.layout_view_top_cell, occurrence)
    else {
        return Vec::new();
    };
    let shape = view.transformed_shape();
    let source_cell = layout_cell_display_name(document, view.source_cell);
    let layer = layout_layer_display_name(document, shape.layer);
    let net = if document.flattened_shape_count_estimate() <= MAX_CONNECTIVITY_OVERLAY_SHAPES {
        app.connectivity_report()
            .ok()
            .and_then(|report| {
                report
                    .component_for_occurrence(occurrence)
                    .map(|id| (report, id))
            })
            .and_then(|(report, id)| {
                report
                    .component(id)
                    .map(|component| connectivity_component_display_name(component))
            })
            .unwrap_or_else(|| "None".to_string())
    } else {
        "Skipped".to_string()
    };
    let mut rows = match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Shape id".to_string(), format!("#{}", shape.id.0)),
            (
                "Kind".to_string(),
                shape_kind_label(&shape.kind).to_string(),
            ),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Shape id".to_string(), format!("#{}", shape.id.0)),
            (
                "Kind".to_string(),
                shape_kind_label(&shape.kind).to_string(),
            ),
            ("Layer".to_string(), layer),
            (
                "Occurrence".to_string(),
                layout_occurrence_label(occurrence),
            ),
            ("Bounds".to_string(), rect_summary(view.bounds)),
        ],
        LayoutBrowserColumnSet::Relations => vec![
            ("Shape id".to_string(), format!("#{}", shape.id.0)),
            ("Layer".to_string(), layer),
            ("Source cell".to_string(), source_cell),
            (
                "Occurrence".to_string(),
                layout_occurrence_label(occurrence),
            ),
            ("Net".to_string(), net),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Shape id".to_string(), format!("#{}", shape.id.0)),
            (
                "Kind".to_string(),
                shape_kind_label(&shape.kind).to_string(),
            ),
            ("Layer".to_string(), layer),
            ("Source cell".to_string(), source_cell),
            (
                "Occurrence".to_string(),
                layout_occurrence_label(occurrence),
            ),
            ("Bounds".to_string(), rect_summary(view.bounds)),
            ("Net".to_string(), net),
        ],
    };
    if matches!(shape.kind, ShapeKind::Measurement { .. }) {
        rows.extend(layout_measurement_shape_property_rows(app, &shape.kind));
    }
    rows.extend(layout_shape_kind_property_rows(app, &shape.kind));
    rows.extend(layout_shape_stored_property_rows(&shape));
    rows
}

pub(crate) fn layout_shape_kind_property_rows(
    app: &GlassworksApp,
    kind: &ShapeKind,
) -> Vec<(String, String)> {
    match kind {
        ShapeKind::Rectangle(rect) => vec![("Area".to_string(), layout_rect_area_label(*rect))],
        ShapeKind::Polygon(poly) => vec![
            ("Points".to_string(), poly.points.len().to_string()),
            (
                "Area".to_string(),
                layout_area2_label(poly.signed_area2().unsigned_abs()),
            ),
        ],
        ShapeKind::Path { points, width } => vec![
            ("Points".to_string(), points.len().to_string()),
            ("Width".to_string(), app.format_layout_length(*width as f64)),
            (
                "Length".to_string(),
                app.format_layout_length(layout_path_centerline_length(points)),
            ),
        ],
        ShapeKind::Via {
            center,
            size,
            lower,
            upper,
        } => vec![
            ("Center".to_string(), point_summary(*center)),
            ("Size".to_string(), app.format_layout_length(*size as f64)),
            (
                "Layers".to_string(),
                format!(
                    "{} -> {}",
                    layout_layer_display_name(&app.workspace.document, *lower),
                    layout_layer_display_name(&app.workspace.document, *upper)
                ),
            ),
        ],
        ShapeKind::Label { position, text } => vec![
            ("Position".to_string(), point_summary(*position)),
            ("Text".to_string(), compact_button_label(text, 48)),
        ],
        ShapeKind::Measurement { .. } => Vec::new(),
    }
}

pub(crate) fn layout_rect_area_label(rect: Rect) -> String {
    layout_area2_label(layout_rect_area2(rect))
}

pub(crate) fn layout_rect_area2(rect: Rect) -> u128 {
    (rect.width().max(0) as u128) * (rect.height().max(0) as u128) * 2
}

pub(crate) fn layout_area2_label(area2: u128) -> String {
    if area2 % 2 == 0 {
        format!("{} dbu^2", area2 / 2)
    } else {
        format!("{}.5 dbu^2", area2 / 2)
    }
}

pub(crate) fn layout_path_centerline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|segment| segment[0].distance_to(segment[1]))
        .sum()
}

pub(crate) fn layout_shape_stored_property_rows(shape: &Shape) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if let Some(name) = shape
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        rows.push(("Name".to_string(), compact_button_label(name, 48)));
    }
    rows.extend(layout_stored_property_rows(&shape.properties));
    rows
}

pub(crate) fn layout_stored_property_rows(
    properties: &BTreeMap<String, String>,
) -> Vec<(String, String)> {
    const MAX_STORED_PROPERTY_ROWS: usize = 6;
    let mut rows = Vec::new();
    if !properties.is_empty() {
        rows.push((
            "Properties".to_string(),
            format!("{} stored", properties.len()),
        ));
        for (key, value) in properties.iter().take(MAX_STORED_PROPERTY_ROWS) {
            rows.push((
                format!("Property {}", compact_button_label(key, 24)),
                compact_button_label(value, 48),
            ));
        }
        if properties.len() > MAX_STORED_PROPERTY_ROWS {
            rows.push((
                "Properties omitted".to_string(),
                (properties.len() - MAX_STORED_PROPERTY_ROWS).to_string(),
            ));
        }
    }
    rows
}
