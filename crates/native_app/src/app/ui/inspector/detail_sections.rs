#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct DetailSection {
    pub(crate) title: String,
    pub(crate) rows: Vec<(String, String)>,
}

impl DetailSection {
    pub(crate) fn new(title: impl Into<String>, rows: Vec<(String, String)>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }
}

pub(crate) fn layout_editor_detail_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let active_layer = document
        .layers
        .get(&app.active_layer)
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| format!("L{} missing", app.active_layer.0));
    let selected = app.selected_layout_shape_ref();
    let selected_shape = selected
        .as_ref()
        .map(|shape| format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)))
        .unwrap_or_else(|| "None".to_string());
    let selected_layer = selected
        .as_ref()
        .and_then(|shape| document.layers.get(&shape.layer))
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| "None".to_string());
    let selected_bounds = selected
        .as_ref()
        .map(|shape| rect_summary(shape.kind.bounds()))
        .unwrap_or_else(|| "None".to_string());
    vec![
        (
            "Active view".to_string(),
            app.active_view.label().to_string(),
        ),
        ("Active layer".to_string(), active_layer),
        ("Selected shape".to_string(), selected_shape),
        ("Selection layer".to_string(), selected_layer),
        ("Selection bounds".to_string(), selected_bounds),
        (
            "Display".to_string(),
            format!(
                "grid={}, snap={}, drc={}, hierarchy={}, top={}",
                app.show_grid,
                app.snap_enabled,
                app.show_drc_overlay,
                layout_hierarchy_display_label(
                    app.layout_hierarchy_depth,
                    app.layout_hierarchy_min_depth
                ),
                layout_view_top_cell_name(app)
            ),
        ),
        (
            "Markers".to_string(),
            format!(
                "drc={}, connectivity={}",
                document.marker_states.len(),
                document.connectivity_issue_states.len()
            ),
        ),
    ]
}

pub(crate) fn layout_editor_inspector_sections(app: &GlassworksApp) -> Vec<DetailSection> {
    let document = &app.workspace.document;
    let technology = app.active_layout_technology();
    let active_layer = document
        .layers
        .get(&app.active_layer)
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| format!("L{} missing", app.active_layer.0));

    vec![
        DetailSection::new(
            "Document",
            vec![
                (
                    "Technology".to_string(),
                    display_technology_name(&technology.name),
                ),
                ("Name".to_string(), display_document_name(&document.name)),
                (
                    "Grid".to_string(),
                    app.format_layout_length(document.grid as f64),
                ),
                (
                    "Shapes".to_string(),
                    document.flattened_shape_count_estimate().to_string(),
                ),
                ("Layers".to_string(), document.layers.len().to_string()),
                ("Cells".to_string(), document.cells.len().to_string()),
            ],
        ),
        DetailSection::new("Cells", layout_cell_hierarchy_rows(app)),
        DetailSection::new("Hierarchy Tree", layout_hierarchy_tree_summary_rows(app)),
        DetailSection::new("View Bookmarks", layout_view_bookmark_rows(app)),
        DetailSection::new("Layer Sets", layout_layer_set_rows(app)),
        DetailSection::new("Layer Groups", layout_layer_group_directory_rows(app)),
        DetailSection::new("Technology Stack", layout_technology_stack_rows(app)),
        DetailSection::new("Browser Search", layout_browser_search_rows(app)),
        DetailSection::new("Cell Browser", layout_cell_browser_rows(app)),
        DetailSection::new("Shape Browser", layout_shape_browser_rows(app)),
        DetailSection::new("Measurements", layout_measurement_rows(app)),
        DetailSection::new("Reference Images", layout_reference_image_rows(app)),
        DetailSection::new("Instance Browser", layout_instance_browser_rows(app)),
        DetailSection::new("Connectivity", layout_connectivity_rows(app)),
        DetailSection::new("Net Browser", layout_net_browser_rows(app)),
        DetailSection::new("Trace History", layout_trace_history_rows(app)),
        DetailSection::new("DRC Markers", layout_drc_marker_rows(app)),
        DetailSection::new(
            "DRC Marker Directory",
            layout_drc_marker_directory_rows(app),
        ),
        DetailSection::new("DRC Marker Info", layout_drc_marker_info_rows(app)),
        DetailSection::new(
            "Diagnostics",
            vec![
                ("Tool".to_string(), app.active_tool.label().to_string()),
                ("Active layer".to_string(), active_layer),
                (
                    "Selection".to_string(),
                    app.selected_layout_occurrence
                        .as_ref()
                        .map(layout_occurrence_label)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Selection bounds".to_string(),
                    app.selected_layout_shape_ref()
                        .map(|shape| rect_summary(shape.kind.bounds()))
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Display".to_string(),
                    format!(
                        "grid={}, snap={}, drc={}, hierarchy={}, top={}",
                        app.show_grid,
                        app.snap_enabled,
                        app.show_drc_overlay,
                        layout_hierarchy_display_label(
                            app.layout_hierarchy_depth,
                            app.layout_hierarchy_min_depth
                        ),
                        layout_view_top_cell_name(app)
                    ),
                ),
            ],
        ),
    ]
}

pub(crate) fn layout_browser_search_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let query = app.layout_browser_search.trim();
    vec![
        (
            "Search".to_string(),
            if query.is_empty() {
                "None".to_string()
            } else {
                query.to_string()
            },
        ),
        (
            "Capture".to_string(),
            if app.layout_browser_search_active {
                "active"
            } else {
                "inactive"
            }
            .to_string(),
        ),
        (
            "Replace".to_string(),
            layout_browser_replace_display_value(app),
        ),
        (
            "Replace capture".to_string(),
            if app.layout_browser_replace_active {
                "active"
            } else {
                "inactive"
            }
            .to_string(),
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
    ]
}

pub(crate) fn layout_layer_set_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let current_state = app.current_layout_layer_set_state();
    let search = layout_browser_search_query_lower(app);
    let mut rows = vec![(
        "Visible now".to_string(),
        current_state.visible_layers.len().to_string(),
    )];
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed layer sets".to_string(),
            format!(
                "{} / {} slots",
                layout_layer_set_listed_count(app, &current_state, query),
                LAYOUT_LAYER_SET_SLOTS.count()
            ),
        ));
    }
    for slot in LAYOUT_LAYER_SET_SLOTS {
        let state = app.layout_layer_sets.get(&slot);
        if search.as_deref().is_some_and(|query| {
            !layout_layer_set_slot_matches_search(app, &current_state, slot, state, query)
        }) {
            continue;
        }
        let value = state
            .map(|state| layout_layer_set_state_summary(state, &current_state))
            .unwrap_or_else(|| "empty".to_string());
        rows.push((format!("Set {slot}"), value));
    }
    rows
}

fn layout_layer_set_listed_count(
    app: &GlassworksApp,
    current_state: &LayoutLayerSetState,
    query: &str,
) -> usize {
    LAYOUT_LAYER_SET_SLOTS
        .filter(|slot| {
            let state = app.layout_layer_sets.get(slot);
            layout_layer_set_slot_matches_search(app, current_state, *slot, state, query)
        })
        .count()
}

fn layout_layer_set_state_summary(
    state: &LayoutLayerSetState,
    current_state: &LayoutLayerSetState,
) -> String {
    let suffix = if state == current_state {
        " active"
    } else {
        ""
    };
    format!(
        "{} visible, {}, {} rows, {} depth{}{}",
        state.visible_layers.len(),
        state.layer_group_filter.label(),
        state.layer_usage_filter.label(),
        state.layer_depth_overrides.len(),
        if state.layer_depth_overrides.len() == 1 {
            ""
        } else {
            "s"
        },
        suffix
    )
}

fn layout_layer_set_slot_matches_search(
    app: &GlassworksApp,
    current_state: &LayoutLayerSetState,
    slot: u8,
    state: Option<&LayoutLayerSetState>,
    query_lower: &str,
) -> bool {
    let name = app.layout_layer_set_name(slot);
    if layout_layer_set_selector_matches_search(
        &app.workspace.document,
        slot,
        &name,
        state,
        state == Some(current_state),
        query_lower,
    ) {
        return true;
    }
    let mut fields = vec![
        format!("set {slot}"),
        format!("slot {slot}"),
        name,
        if state.is_some() { "saved" } else { "empty" }.to_string(),
    ];
    if let Some(state) = state {
        push_layout_search_field(
            &mut fields,
            layout_layer_set_state_summary(state, current_state),
        );
        push_layout_search_field(&mut fields, state.layer_group_filter.label());
        push_layout_search_field(&mut fields, state.layer_group_filter.path_label());
        push_layout_search_field(&mut fields, state.layer_group_filter.slug());
        push_layout_search_field(&mut fields, state.layer_usage_filter.label());
        push_layout_search_field(&mut fields, state.layer_usage_filter.slug());
        push_layout_search_field(
            &mut fields,
            format!("{} visible", state.visible_layers.len()),
        );
        push_layout_search_field(
            &mut fields,
            format!("{} depth", state.layer_depth_overrides.len()),
        );
        for layer_id in &state.visible_layers {
            push_layout_search_field(&mut fields, format!("L{}", layer_id.0));
            if let Some(layer) = app.workspace.document.layer(*layer_id) {
                push_layout_search_field(&mut fields, layer.name.clone());
                push_layout_search_field(&mut fields, format!("visible {}", layer.name));
            }
        }
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_layer_set_selector_matches_search(
    document: &Document,
    slot: u8,
    name: &str,
    state: Option<&LayoutLayerSetState>,
    active: bool,
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
    let mut fields = vec![
        ("slot", slot.to_string()),
        ("set", slot.to_string()),
        ("layer_set", slot.to_string()),
        ("name", name.to_string()),
        (
            "state",
            if state.is_some() { "saved" } else { "empty" }.to_string(),
        ),
        (
            "status",
            if state.is_some() { "saved" } else { "empty" }.to_string(),
        ),
        ("active", active.to_string()),
    ];
    if let Some(state) = state {
        fields.extend([
            ("visible_layers", state.visible_layers.len().to_string()),
            ("visible_count", state.visible_layers.len().to_string()),
            ("group", state.layer_group_filter.path_label()),
            ("group_slug", state.layer_group_filter.slug().to_string()),
            ("layer_group", state.layer_group_filter.path_label()),
            ("row_filter", state.layer_usage_filter.label().to_string()),
            ("usage", state.layer_usage_filter.label().to_string()),
            ("usage_slug", state.layer_usage_filter.slug().to_string()),
            (
                "depth_overrides",
                state.layer_depth_overrides.len().to_string(),
            ),
            ("depth_count", state.layer_depth_overrides.len().to_string()),
        ]);
        for layer_id in &state.visible_layers {
            fields.push(("visible_layer", format!("L{}", layer_id.0)));
            fields.push(("visible_layer_id", layer_id.0.to_string()));
            if let Some(layer) = document.layer(*layer_id) {
                fields.push(("visible_layer", layer.name.clone()));
            }
        }
        for (layer_id, depth) in &state.layer_depth_overrides {
            fields.push(("depth_layer", format!("L{}", layer_id.0)));
            fields.push(("depth_layer_id", layer_id.0.to_string()));
            fields.push(("depth", depth.slug()));
            if let Some(layer) = document.layer(*layer_id) {
                fields.push(("depth_layer", layer.name.clone()));
            }
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutLayerGroupSummary {
    pub(crate) group: LayoutLayerGroupFilter,
    pub(crate) layer_count: usize,
    pub(crate) used_layer_count: usize,
    pub(crate) visible_layer_count: usize,
    pub(crate) shape_count: usize,
}

pub(crate) fn layout_layer_group_directory_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let summaries = layout_layer_group_summaries(&app.workspace.document);
    let search = layout_browser_search_query_lower(app);
    let mut rows = vec![(
        "Current".to_string(),
        app.layout_layer_group_filter.path_label(),
    )];
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed groups".to_string(),
            format!(
                "{} / {} groups",
                layout_layer_group_listed_count(&summaries, app.layout_layer_group_filter, query),
                summaries.len()
            ),
        ));
    }
    rows.extend(
        summaries
            .iter()
            .filter(|summary| {
                search.as_deref().is_none_or(|query| {
                    layout_layer_group_summary_matches_search(
                        summary,
                        app.layout_layer_group_filter,
                        query,
                    )
                })
            })
            .map(|summary| {
                (
                    summary.group.path_label(),
                    format_layout_layer_group_summary(summary),
                )
            }),
    );
    rows
}

fn layout_layer_group_listed_count(
    summaries: &[LayoutLayerGroupSummary],
    current_group: LayoutLayerGroupFilter,
    query: &str,
) -> usize {
    summaries
        .iter()
        .filter(|summary| layout_layer_group_summary_matches_search(summary, current_group, query))
        .count()
}

fn layout_layer_group_summary_matches_search(
    summary: &LayoutLayerGroupSummary,
    current_group: LayoutLayerGroupFilter,
    query_lower: &str,
) -> bool {
    let active = summary.group == current_group;
    if layout_layer_group_selector_matches_search(summary, active, query_lower) {
        return true;
    }
    let mut fields = vec![
        summary.group.label().to_string(),
        summary.group.path_label(),
        summary.group.slug().to_string(),
        format_layout_layer_group_summary(summary),
        if active { "current" } else { "available" }.to_string(),
    ];
    if active {
        push_layout_search_field(&mut fields, "active");
    }
    if let Some(parent) = summary.group.parent() {
        push_layout_search_field(&mut fields, parent.label());
        push_layout_search_field(&mut fields, parent.path_label());
        push_layout_search_field(&mut fields, parent.slug());
        push_layout_search_field(&mut fields, format!("parent {}", parent.label()));
    }
    push_layout_search_field(&mut fields, format!("{} layers", summary.layer_count));
    push_layout_search_field(&mut fields, format!("{} used", summary.used_layer_count));
    push_layout_search_field(
        &mut fields,
        format!("{} visible", summary.visible_layer_count),
    );
    push_layout_search_field(&mut fields, format!("{} shapes", summary.shape_count));
    layout_search_matches_any(query_lower, &fields)
}

fn layout_layer_group_selector_matches_search(
    summary: &LayoutLayerGroupSummary,
    active: bool,
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
    let state = if active { "current" } else { "available" };
    let mut fields = vec![
        ("group", summary.group.path_label()),
        ("group", summary.group.slug().to_string()),
        ("group_slug", summary.group.slug().to_string()),
        ("layer_group", summary.group.path_label()),
        ("path", summary.group.path_label()),
        ("label", summary.group.label().to_string()),
        ("name", summary.group.label().to_string()),
        ("state", state.to_string()),
        ("status", state.to_string()),
        ("active", active.to_string()),
        ("current", active.to_string()),
        ("layers", summary.layer_count.to_string()),
        ("layer_count", summary.layer_count.to_string()),
        ("used", summary.used_layer_count.to_string()),
        ("used_layers", summary.used_layer_count.to_string()),
        ("used_layer_count", summary.used_layer_count.to_string()),
        ("visible", summary.visible_layer_count.to_string()),
        ("visible_layers", summary.visible_layer_count.to_string()),
        (
            "visible_layer_count",
            summary.visible_layer_count.to_string(),
        ),
        ("shapes", summary.shape_count.to_string()),
        ("shape_count", summary.shape_count.to_string()),
    ];
    if let Some(parent) = summary.group.parent() {
        fields.extend([
            ("parent", parent.path_label()),
            ("parent", parent.slug().to_string()),
            ("parent_slug", parent.slug().to_string()),
            ("parent_label", parent.label().to_string()),
        ]);
    } else {
        fields.push(("parent", "none".to_string()));
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_layer_group_summaries(document: &Document) -> Vec<LayoutLayerGroupSummary> {
    let usage = layout_layer_usage_counts(document);
    LayoutLayerGroupFilter::ALL
        .into_iter()
        .map(|group| {
            let mut layer_count = 0;
            let mut used_layer_count = 0;
            let mut visible_layer_count = 0;
            let mut shape_count = 0;
            for layer in document.layers.values() {
                if !group.matches(layer.process) {
                    continue;
                }
                let layer_shapes = usage.get(&layer.id).copied().unwrap_or(0);
                layer_count += 1;
                shape_count += layer_shapes;
                if layer_shapes > 0 {
                    used_layer_count += 1;
                }
                if layer.visible {
                    visible_layer_count += 1;
                }
            }
            LayoutLayerGroupSummary {
                group,
                layer_count,
                used_layer_count,
                visible_layer_count,
                shape_count,
            }
        })
        .collect()
}

pub(crate) fn format_layout_layer_group_summary(summary: &LayoutLayerGroupSummary) -> String {
    format!(
        "{} layer{}, {} used, {} visible, {} shape{}",
        summary.layer_count,
        if summary.layer_count == 1 { "" } else { "s" },
        summary.used_layer_count,
        summary.visible_layer_count,
        summary.shape_count,
        if summary.shape_count == 1 { "" } else { "s" }
    )
}

pub(crate) fn layout_technology_stack_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_technology_stack_entries(app);
    let search = layout_browser_search_query_lower(app);
    let mut rows = Vec::new();
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed entries".to_string(),
            format!(
                "{} / {} entries",
                layout_technology_stack_listed_count(&entries, query),
                entries.len()
            ),
        ));
    }
    rows.extend(
        entries
            .into_iter()
            .filter(|entry| {
                search
                    .as_deref()
                    .is_none_or(|query| layout_technology_stack_entry_matches_search(entry, query))
            })
            .map(|entry| (entry.key, entry.value)),
    );
    rows
}

#[derive(Clone, Debug)]
struct LayoutTechnologyStackEntry {
    key: String,
    value: String,
    fields: Vec<(String, String)>,
}

fn layout_technology_stack_entries(app: &GlassworksApp) -> Vec<LayoutTechnologyStackEntry> {
    let document = &app.workspace.document;
    let technology = app.active_layout_technology();
    let technology_name = display_technology_name(&technology.name);
    let enabled_stack_links = technology
        .connectivity
        .len()
        .saturating_sub(app.layout_disabled_connectivity_links.len());
    let mut entries = vec![
        layout_technology_stack_entry(
            "Technology",
            technology_name.clone(),
            vec![
                ("technology", technology_name),
                ("technology_slug", technology.name.clone()),
                ("name", technology.name.clone()),
            ],
        ),
        layout_technology_stack_entry(
            "Database unit",
            format!(
                "{} DBU/um, grid {}",
                technology.dbu_per_micron,
                app.format_layout_length(technology.grid as f64)
            ),
            vec![
                ("dbu", technology.dbu_per_micron.to_string()),
                ("dbu_per_micron", technology.dbu_per_micron.to_string()),
                ("grid", technology.grid.to_string()),
                ("grid_dbu", technology.grid.to_string()),
            ],
        ),
        layout_technology_stack_entry(
            "Layer definitions",
            format!(
                "{} tech, {} document",
                technology.layers.len(),
                document.layers.len()
            ),
            vec![
                ("tech_layers", technology.layers.len().to_string()),
                ("technology_layers", technology.layers.len().to_string()),
                ("document_layers", document.layers.len().to_string()),
                ("layers", document.layers.len().to_string()),
            ],
        ),
        layout_technology_stack_entry(
            "DRC rules",
            format!(
                "{} min-width, {} max-width, {} min-area, {} max-area, {} spacing, {} edge-spacing, {} enclosure, {} overlap",
                technology.drc.min_width.len(),
                technology.drc.max_width.len(),
                technology.drc.min_area.len(),
                technology.drc.max_area.len(),
                technology.drc.min_spacing.len(),
                technology.drc.min_edge_spacing.len(),
                technology.drc.via_enclosure.len(),
                technology.drc.forbidden_overlaps.len()
            ),
            vec![
                ("min_width", technology.drc.min_width.len().to_string()),
                ("max_width", technology.drc.max_width.len().to_string()),
                ("min_area", technology.drc.min_area.len().to_string()),
                ("max_area", technology.drc.max_area.len().to_string()),
                ("spacing", technology.drc.min_spacing.len().to_string()),
                (
                    "edge_spacing",
                    technology.drc.min_edge_spacing.len().to_string(),
                ),
                ("enclosure", technology.drc.via_enclosure.len().to_string()),
                (
                    "overlap",
                    technology.drc.forbidden_overlaps.len().to_string(),
                ),
                ("rules", technology_drc_rule_count(technology).to_string()),
            ],
        ),
        layout_technology_stack_entry(
            "DRC deck",
            compact_button_label(&app.active_layout_drc_deck_label(), 48),
            vec![
                ("drc_deck", app.active_layout_drc_deck_label()),
                ("deck", app.active_layout_drc_deck_label()),
            ],
        ),
        layout_technology_stack_entry(
            "Connectivity",
            format!(
                "{}/{} stack link{} enabled",
                enabled_stack_links,
                technology.connectivity.len(),
                if technology.connectivity.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ),
            vec![
                ("stack_links", technology.connectivity.len().to_string()),
                ("enabled_links", enabled_stack_links.to_string()),
                (
                    "disabled_links",
                    app.layout_disabled_connectivity_links.len().to_string(),
                ),
                (
                    "state",
                    if app.layout_disabled_connectivity_links.is_empty() {
                        "enabled"
                    } else {
                        "partial"
                    }
                    .to_string(),
                ),
            ],
        ),
    ];
    for (index, connection) in technology.connectivity.iter().take(4).enumerate() {
        let state = if app.layout_disabled_connectivity_links.contains(&index) {
            "disabled"
        } else {
            "enabled"
        };
        entries.push(layout_technology_stack_entry(
            format!("Stack {}", index + 1),
            compact_button_label(
                &format!(
                    "{} / {} / {} ({state})",
                    connection.from, connection.through, connection.to
                ),
                48,
            ),
            vec![
                ("stack", (index + 1).to_string()),
                ("stack_index", (index + 1).to_string()),
                ("from", connection.from.clone()),
                ("through", connection.through.clone()),
                ("to", connection.to.clone()),
                ("layer", connection.from.clone()),
                ("layer", connection.through.clone()),
                ("layer", connection.to.clone()),
                (
                    "connectivity",
                    format!(
                        "{} {} {}",
                        connection.from, connection.through, connection.to
                    ),
                ),
                ("state", state.to_string()),
                ("status", state.to_string()),
                (
                    "enabled",
                    (!app.layout_disabled_connectivity_links.contains(&index)).to_string(),
                ),
            ],
        ));
    }
    if technology.connectivity.len() > 4 {
        entries.push(layout_technology_stack_entry(
            "Stack links omitted",
            (technology.connectivity.len() - 4).to_string(),
            vec![("omitted", (technology.connectivity.len() - 4).to_string())],
        ));
    }
    if let Some(layer) = document.layers.get(&app.active_layer) {
        entries.push(layout_technology_stack_entry(
            "Active layer",
            format!("L{} {}", layer.id.0, compact_button_label(&layer.name, 30)),
            vec![
                ("active_layer", format!("L{}", layer.id.0)),
                ("active_layer_id", layer.id.0.to_string()),
                ("active_layer", layer.name.clone()),
                ("active_layer_name", layer.name.clone()),
                ("process", layer.process.as_technology_name().to_string()),
                ("purpose", layer.purpose.clone()),
            ],
        ));
        entries.push(layout_technology_stack_entry(
            "Active mapping",
            layout_layer_gds_mapping_label(layer),
            vec![
                (
                    "gds_layer",
                    layer
                        .gds_layer
                        .map(|gds_layer| gds_layer.to_string())
                        .unwrap_or_else(|| "unmapped".to_string()),
                ),
                ("gds_datatype", layer.gds_datatype.to_string()),
                ("gds_texttype", layer.gds_texttype.to_string()),
                ("active_layer_id", layer.id.0.to_string()),
            ],
        ));
        entries.push(layout_technology_stack_entry(
            "Active stack links",
            layout_active_layer_connectivity_label(
                layer,
                technology,
                &app.layout_disabled_connectivity_links,
            ),
            vec![
                ("active_layer_id", layer.id.0.to_string()),
                ("active_layer", layer.name.clone()),
                (
                    "active_links",
                    layout_active_layer_connectivity_label(
                        layer,
                        technology,
                        &app.layout_disabled_connectivity_links,
                    ),
                ),
            ],
        ));
        if let Some(technology_layer) = matching_technology_layer(&technology, layer)
            && let (Some(z_base), Some(z_thickness)) =
                (technology_layer.z_base, technology_layer.z_thickness)
        {
            entries.push(layout_technology_stack_entry(
                "Active z",
                format!("{z_base:.2}..{:.2}", z_base + z_thickness),
                vec![
                    ("z_base", format!("{z_base:.2}")),
                    ("z_thickness", format!("{z_thickness:.2}")),
                    ("z_top", format!("{:.2}", z_base + z_thickness)),
                    ("active_layer_id", layer.id.0.to_string()),
                ],
            ));
        }
    }
    entries
}

fn layout_technology_stack_entry(
    key: impl Into<String>,
    value: impl Into<String>,
    fields: Vec<(&'static str, String)>,
) -> LayoutTechnologyStackEntry {
    let key = key.into();
    let value = value.into();
    let mut entry_fields = vec![
        ("section".to_string(), "technology_stack".to_string()),
        ("row".to_string(), key.clone()),
        ("label".to_string(), key.clone()),
        ("value".to_string(), value.clone()),
    ];
    entry_fields.extend(
        fields
            .into_iter()
            .map(|(field_key, field_value)| (field_key.to_string(), field_value)),
    );
    LayoutTechnologyStackEntry {
        key,
        value,
        fields: entry_fields,
    }
}

fn layout_technology_stack_listed_count(
    entries: &[LayoutTechnologyStackEntry],
    query: &str,
) -> usize {
    entries
        .iter()
        .filter(|entry| layout_technology_stack_entry_matches_search(entry, query))
        .count()
}

fn layout_technology_stack_entry_matches_search(
    entry: &LayoutTechnologyStackEntry,
    query_lower: &str,
) -> bool {
    if layout_technology_stack_selector_matches_search(entry, query_lower) {
        return true;
    }
    let mut fields = vec![entry.key.clone(), entry.value.clone()];
    for (key, value) in &entry.fields {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
        push_layout_search_field(&mut fields, format!("{key}={value}"));
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_technology_stack_selector_matches_search(
    entry: &LayoutTechnologyStackEntry,
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
    entry
        .fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn technology_drc_rule_count(technology: &layout_model::TechnologyFile) -> usize {
    technology.drc.min_width.len()
        + technology.drc.max_width.len()
        + technology.drc.min_area.len()
        + technology.drc.max_area.len()
        + technology.drc.min_spacing.len()
        + technology.drc.min_edge_spacing.len()
        + technology.drc.via_enclosure.len()
        + technology.drc.forbidden_overlaps.len()
}

pub(crate) fn layout_layer_gds_mapping_label(layer: &Layer) -> String {
    match layer.gds_layer {
        Some(gds_layer) => format!(
            "GDS {}/{}, text {}",
            gds_layer, layer.gds_datatype, layer.gds_texttype
        ),
        None => "GDS unmapped".to_string(),
    }
}

pub(crate) fn layout_active_layer_connectivity_label(
    layer: &Layer,
    technology: &layout_model::TechnologyFile,
    disabled_links: &BTreeSet<usize>,
) -> String {
    let links = technology
        .connectivity
        .iter()
        .enumerate()
        .filter(|(index, connection)| {
            !disabled_links.contains(index)
                && (layout_layer_matches_technology_reference(layer, &connection.from)
                    || layout_layer_matches_technology_reference(layer, &connection.through)
                    || layout_layer_matches_technology_reference(layer, &connection.to))
        })
        .map(|(_, connection)| {
            format!(
                "{}-{}-{}",
                connection.from, connection.through, connection.to
            )
        })
        .collect::<Vec<_>>();
    if links.is_empty()
        && technology.connectivity.iter().any(|connection| {
            layout_layer_matches_technology_reference(layer, &connection.from)
                || layout_layer_matches_technology_reference(layer, &connection.through)
                || layout_layer_matches_technology_reference(layer, &connection.to)
        })
    {
        return "Stack links disabled".to_string();
    }
    if links.is_empty() {
        return "No stack links".to_string();
    }
    let mut label = links.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
    if links.len() > 2 {
        label.push_str(&format!(" +{}", links.len() - 2));
    }
    compact_button_label(&label, 48)
}

pub(crate) fn matching_technology_layer<'a>(
    technology: &'a layout_model::TechnologyFile,
    layer: &Layer,
) -> Option<&'a layout_model::TechnologyLayer> {
    technology
        .layers
        .iter()
        .find(|technology_layer| technology_layer.id == Some(layer.id))
        .or_else(|| {
            technology.layers.iter().find(|technology_layer| {
                layout_layer_matches_technology_reference(layer, &technology_layer.name)
                    || layout_layer_matches_technology_reference(layer, &technology_layer.process)
            })
        })
}

pub(crate) fn layout_drc_deck_layer_reference(document: &Document, id: LayerId) -> String {
    document
        .layers
        .get(&id)
        .map(|layer| layer.name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("#{}", id.0))
}

pub(crate) fn resolve_layout_drc_deck_layer(
    document: &Document,
    reference: &str,
    rule_family: &str,
) -> Result<LayerId, String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(format!("{rule_family} rule references an empty layer"));
    }
    if let Some(id) = reference
        .strip_prefix('#')
        .or_else(|| reference.strip_prefix("layer:"))
        .and_then(|value| value.parse::<u32>().ok())
        .map(LayerId)
        .filter(|id| document.layers.contains_key(id))
    {
        return Ok(id);
    }
    if let Ok(raw_id) = reference.parse::<u32>() {
        let id = LayerId(raw_id);
        if document.layers.contains_key(&id) {
            return Ok(id);
        }
    }

    let normalized = normalize_technology_reference(reference);
    document
        .layers
        .values()
        .find(|layer| {
            normalize_technology_reference(&layer.name) == normalized
                || normalize_technology_reference(&layer.purpose) == normalized
                || normalize_technology_reference(layer.process.as_technology_name()) == normalized
        })
        .map(|layer| layer.id)
        .ok_or_else(|| {
            format!("{rule_family} rule references missing document layer {reference:?}")
        })
}

pub(crate) fn layout_layer_matches_technology_reference(layer: &Layer, reference: &str) -> bool {
    let reference = normalize_technology_reference(reference);
    reference == normalize_technology_reference(&layer.name)
        || reference == normalize_technology_reference(&layer.purpose)
        || reference == normalize_technology_reference(layer.process.as_technology_name())
        || reference == layer.id.0.to_string()
        || reference == format!("l{}", layer.id.0)
}

pub(crate) fn normalize_technology_reference(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn layout_browser_search_display_value(app: &GlassworksApp) -> String {
    let query = app.layout_browser_search.trim();
    if query.is_empty() {
        "None".to_string()
    } else {
        query.to_string()
    }
}

pub(crate) fn layout_browser_replace_display_value(app: &GlassworksApp) -> String {
    if app.layout_browser_replace.is_empty() {
        "None".to_string()
    } else {
        app.layout_browser_replace.clone()
    }
}

pub(crate) fn layout_view_bookmark_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let current = app.current_layout_view_state();
    let slot_count = LAYOUT_VIEW_BOOKMARK_SLOTS.count();
    let search = layout_browser_search_query_lower(app);
    let current_row = (
        "Current".to_string(),
        layout_view_state_summary(&app.workspace.document, current, false),
    );
    let previous_row = app.layout_previous_view.map(|state| {
        (
            "Previous".to_string(),
            if app.layout_view_state_is_valid(state) {
                layout_view_state_summary(&app.workspace.document, state, state == current)
            } else {
                "invalid".to_string()
            },
        )
    });
    let mut rows = Vec::new();
    if search.as_deref().is_none_or(|query| {
        layout_view_bookmark_named_state_matches_search(
            &app.workspace.document,
            "Current",
            current,
            true,
            "current",
            query,
        )
    }) {
        rows.push(current_row);
    }
    if let Some(previous_row) = previous_row
        && search.as_deref().is_none_or(|query| {
            app.layout_previous_view.is_some_and(|previous| {
                layout_view_bookmark_named_state_matches_search(
                    &app.workspace.document,
                    "Previous",
                    previous,
                    previous == current,
                    "previous",
                    query,
                )
            })
        })
    {
        rows.push(previous_row);
    } else if search.is_none() {
        rows.push(("Previous".to_string(), "None".to_string()));
    }
    rows.push((
        "Saved".to_string(),
        format!("{} / {slot_count}", app.layout_view_bookmarks.len()),
    ));
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        let listed = layout_view_bookmark_listed_count(app, current, query);
        rows.push((
            "Listed views".to_string(),
            format!("{listed} / {} views", slot_count + 2),
        ));
    }
    for slot in LAYOUT_VIEW_BOOKMARK_SLOTS {
        let state = app.layout_view_bookmarks.get(&slot).copied();
        if search.as_deref().is_some_and(|query| {
            !layout_view_bookmark_slot_matches_search(app, current, slot, state, query)
        }) {
            continue;
        }
        let value = state
            .map(|state| layout_view_bookmark_slot_summary(app, current, slot, state))
            .unwrap_or_else(|| "empty".to_string());
        rows.push((format!("View {slot}"), value));
    }
    rows
}

fn layout_view_bookmark_named_state_matches_search(
    document: &Document,
    name: &str,
    state: LayoutViewState,
    active: bool,
    state_label: &str,
    query_lower: &str,
) -> bool {
    if layout_view_bookmark_selector_matches_search(
        document,
        None,
        name,
        Some(state),
        active,
        state_label,
        query_lower,
    ) {
        return true;
    }
    let mut fields = vec![
        name.to_string(),
        state_label.to_string(),
        layout_view_state_summary(document, state, active),
    ];
    push_layout_view_state_search_fields(&mut fields, document, state);
    layout_search_matches_any(query_lower, &fields)
}

fn layout_view_bookmark_listed_count(
    app: &GlassworksApp,
    current: LayoutViewState,
    query: &str,
) -> usize {
    let current_count = layout_view_bookmark_named_state_matches_search(
        &app.workspace.document,
        "Current",
        current,
        true,
        "current",
        query,
    ) as usize;
    let previous_count = app
        .layout_previous_view
        .filter(|previous| app.layout_view_state_is_valid(*previous))
        .filter(|previous| {
            layout_view_bookmark_named_state_matches_search(
                &app.workspace.document,
                "Previous",
                *previous,
                *previous == current,
                "previous",
                query,
            )
        })
        .map(|_| 1)
        .unwrap_or(0);
    let slot_count = LAYOUT_VIEW_BOOKMARK_SLOTS
        .filter(|slot| {
            let state = app.layout_view_bookmarks.get(slot).copied();
            layout_view_bookmark_slot_matches_search(app, current, *slot, state, query)
        })
        .count();
    current_count + previous_count + slot_count
}

fn layout_view_bookmark_slot_summary(
    app: &GlassworksApp,
    current: LayoutViewState,
    slot: u8,
    state: LayoutViewState,
) -> String {
    format!(
        "{}: {}",
        compact_button_label(&app.layout_view_bookmark_name(slot), 18),
        layout_view_state_summary(&app.workspace.document, state, state == current)
    )
}

fn layout_view_bookmark_slot_matches_search(
    app: &GlassworksApp,
    current: LayoutViewState,
    slot: u8,
    state: Option<LayoutViewState>,
    query_lower: &str,
) -> bool {
    let name = app.layout_view_bookmark_name(slot);
    if layout_view_bookmark_selector_matches_search(
        &app.workspace.document,
        Some(slot),
        &name,
        state,
        state == Some(current),
        if state.is_some() { "saved" } else { "empty" },
        query_lower,
    ) {
        return true;
    }
    let mut fields = vec![
        format!("view {slot}"),
        format!("slot {slot}"),
        name,
        if state.is_some() { "saved" } else { "empty" }.to_string(),
    ];
    if let Some(state) = state {
        push_layout_search_field(
            &mut fields,
            layout_view_bookmark_slot_summary(app, current, slot, state),
        );
        push_layout_view_state_search_fields(&mut fields, &app.workspace.document, state);
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_view_bookmark_selector_matches_search(
    document: &Document,
    slot: Option<u8>,
    name: &str,
    state: Option<LayoutViewState>,
    active: bool,
    state_label: &str,
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
    let mut fields = vec![
        ("name", name.to_string()),
        ("state", state_label.to_string()),
        ("status", state_label.to_string()),
        ("active", active.to_string()),
    ];
    if let Some(slot) = slot {
        fields.push(("slot", slot.to_string()));
        fields.push(("view", slot.to_string()));
        fields.push(("bookmark", slot.to_string()));
    }
    if let Some(state) = state {
        let cell = document
            .cell(state.top_cell)
            .map(|cell| format!("C{} {}", cell.id.0, cell.name))
            .unwrap_or_else(|| format!("C{}", state.top_cell.0));
        fields.extend([
            ("top_cell", cell.clone()),
            ("cell", cell),
            ("top_cell_id", state.top_cell.0.to_string()),
            ("cell_id", state.top_cell.0.to_string()),
            ("zoom", format!("{:.3}", state.zoom)),
            ("pan", format!("{:.0},{:.0}", state.pan[0], state.pan[1])),
            ("pan_x", format!("{:.0}", state.pan[0])),
            ("pan_y", format!("{:.0}", state.pan[1])),
            ("hierarchy", state.hierarchy_depth.detail_label()),
            ("hierarchy_depth", state.hierarchy_depth.slug()),
            ("hierarchy_min_depth", state.hierarchy_min_depth.to_string()),
        ]);
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn push_layout_view_state_search_fields(
    fields: &mut Vec<String>,
    document: &Document,
    state: LayoutViewState,
) {
    push_layout_search_field(fields, layout_view_state_summary(document, state, false));
    push_layout_search_field(fields, format!("{:.3}x", state.zoom));
    push_layout_search_field(
        fields,
        format!("pan {:.0},{:.0}", state.pan[0], state.pan[1]),
    );
    push_layout_search_field(fields, state.hierarchy_depth.detail_label());
    push_layout_search_field(fields, state.hierarchy_depth.slug());
    if let Some(cell) = document.cell(state.top_cell) {
        push_layout_search_field(fields, cell.name.clone());
        push_layout_search_field(fields, format!("cell {}", cell.name));
        push_layout_search_field(fields, format!("C{}", cell.id.0));
    } else {
        push_layout_search_field(fields, format!("C{}", state.top_cell.0));
    }
}

fn layout_view_state_summary(document: &Document, state: LayoutViewState, active: bool) -> String {
    let cell = document
        .cell(state.top_cell)
        .map(|cell| compact_button_label(&cell.name, 16))
        .unwrap_or_else(|| format!("cell {}", state.top_cell.0));
    let mut summary = format!(
        "{cell} at {:.3}x, pan {:.0},{:.0}, {}",
        state.zoom,
        state.pan[0],
        state.pan[1],
        layout_hierarchy_display_label(state.hierarchy_depth, state.hierarchy_min_depth)
    );
    if active {
        summary.push_str(", active");
    }
    summary
}

pub(crate) fn layout_cell_browser_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let cells = layout_cell_browser_cells(app);
    let selected = cells
        .iter()
        .find(|cell| cell.id == app.layout_view_top_cell)
        .copied()
        .or_else(|| cells.first().copied());
    let mut rows = vec![
        ("View top".to_string(), layout_view_top_cell_name(app)),
        ("Listed cells".to_string(), cells.len().to_string()),
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        (
            "Filter".to_string(),
            app.layout_cell_browser_filter.label().to_string(),
        ),
        (
            "Sort".to_string(),
            app.layout_cell_browser_sort.label().to_string(),
        ),
        (
            "Library via array".to_string(),
            app.layout_via_array_parameter_status(),
        ),
        (
            "Library presets".to_string(),
            if app.layout_library_via_array_presets.is_empty() {
                "None".to_string()
            } else {
                app.layout_library_via_array_presets
                    .keys()
                    .map(|slot| format!("P{slot}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
        (
            "Selected".to_string(),
            selected
                .map(|cell| layout_cell_display_name(&app.workspace.document, cell.id))
                .unwrap_or_else(|| "None".to_string()),
        ),
    ];
    if let Some(cell) = selected {
        rows.extend(layout_cell_browser_property_rows(app, cell));
    }
    rows
}

pub(crate) fn layout_cell_browser_property_rows(
    app: &GlassworksApp,
    cell: &Cell,
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let local_shape_count = layout_cell_local_shape_count(document, cell);
    let child_cells = ordered_child_cells_for_tree(document, cell);
    let parent_refs = layout_cell_instance_refs(document, cell.id);
    let ancestor_cells = layout_cell_ancestor_ids(document, cell.id);
    let sibling_cells = layout_cell_sibling_ids(document, cell.id);
    let descendant_cells = layout_cell_descendant_ids(document, cell.id);
    let parent_cells = layout_cell_parent_label(document, cell.id);
    let child_cell_names = layout_cell_child_label(document, &child_cells);
    let ancestor_cell_names = layout_cell_list_label(document, &ancestor_cells, 3);
    let sibling_cell_names = layout_cell_list_label(document, &sibling_cells, 3);
    let descendant_cell_names = layout_cell_list_label(document, &descendant_cells, 3);
    let depth = layout_cell_hierarchy_depth_label(document, cell.id);
    let role = if cell.id == document.top_cell {
        "top"
    } else if parent_refs.is_empty() {
        "unused"
    } else {
        "used"
    };
    let hidden = app.layout_hidden_cells.contains(&cell.id);
    let collapsed = app.layout_tree_collapsed_cells.contains(&cell.id);
    let bounds = layout_cell_local_bounds(document, cell)
        .map(rect_summary)
        .unwrap_or_else(|| "None".to_string());
    let mut rows = match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Cell id".to_string(), format!("C{}", cell.id.0)),
            ("Name".to_string(), cell.name.clone()),
            ("Role".to_string(), role.to_string()),
            ("Depth".to_string(), depth),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Cell id".to_string(), format!("C{}", cell.id.0)),
            ("Shapes".to_string(), local_shape_count.to_string()),
            ("Bounds".to_string(), bounds),
            ("Hidden".to_string(), hidden.to_string()),
        ],
        LayoutBrowserColumnSet::Relations => vec![
            ("Cell id".to_string(), format!("C{}", cell.id.0)),
            ("Role".to_string(), role.to_string()),
            ("Depth".to_string(), depth),
            ("Parents".to_string(), parent_refs.len().to_string()),
            ("Parent cells".to_string(), parent_cells),
            ("Ancestors".to_string(), ancestor_cells.len().to_string()),
            ("Ancestor cells".to_string(), ancestor_cell_names),
            ("Siblings".to_string(), sibling_cells.len().to_string()),
            ("Sibling cells".to_string(), sibling_cell_names),
            (
                "Child instances".to_string(),
                cell.instances.len().to_string(),
            ),
            ("Child cells".to_string(), child_cells.len().to_string()),
            ("Child cell names".to_string(), child_cell_names),
            (
                "Descendants".to_string(),
                descendant_cells.len().to_string(),
            ),
            ("Descendant cells".to_string(), descendant_cell_names),
            ("Collapsed".to_string(), collapsed.to_string()),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Cell id".to_string(), format!("C{}", cell.id.0)),
            ("Name".to_string(), cell.name.clone()),
            ("Role".to_string(), role.to_string()),
            ("Depth".to_string(), depth),
            ("Shapes".to_string(), local_shape_count.to_string()),
            ("Bounds".to_string(), bounds),
            ("Parents".to_string(), parent_refs.len().to_string()),
            ("Parent cells".to_string(), parent_cells),
            ("Ancestors".to_string(), ancestor_cells.len().to_string()),
            ("Ancestor cells".to_string(), ancestor_cell_names),
            ("Siblings".to_string(), sibling_cells.len().to_string()),
            ("Sibling cells".to_string(), sibling_cell_names),
            (
                "Child instances".to_string(),
                cell.instances.len().to_string(),
            ),
            ("Child cells".to_string(), child_cells.len().to_string()),
            ("Child cell names".to_string(), child_cell_names),
            (
                "Descendants".to_string(),
                descendant_cells.len().to_string(),
            ),
            ("Descendant cells".to_string(), descendant_cell_names),
            ("Hidden".to_string(), hidden.to_string()),
            ("Collapsed".to_string(), collapsed.to_string()),
        ],
    };
    rows.extend(layout_cell_property_rows(app, cell));
    rows
}

pub(crate) fn layout_cell_ancestor_ids(document: &Document, cell_id: CellId) -> Vec<CellId> {
    layout_cell_ids_in_browser_order(document, &layout_ancestor_cell_ids(document, cell_id))
}

pub(crate) fn layout_cell_sibling_ids(document: &Document, cell_id: CellId) -> Vec<CellId> {
    layout_cell_ids_in_browser_order(document, &layout_sibling_cell_ids(document, cell_id))
}

pub(crate) fn layout_cell_descendant_ids(document: &Document, cell_id: CellId) -> Vec<CellId> {
    let mut descendants = layout_descendant_cell_ids(document, cell_id);
    descendants.remove(&cell_id);
    layout_cell_ids_in_browser_order(document, &descendants)
}

pub(crate) fn layout_cell_parent_label(document: &Document, cell_id: CellId) -> String {
    let parents = layout_parent_cells(document, cell_id)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    layout_cell_list_label(document, &parents, 3)
}

pub(crate) fn layout_cell_child_label(document: &Document, child_cells: &[CellId]) -> String {
    layout_cell_list_label(document, child_cells, 3)
}

pub(crate) fn layout_cell_ids_in_browser_order(
    document: &Document,
    cell_ids: &BTreeSet<CellId>,
) -> Vec<CellId> {
    ordered_layout_cells(document)
        .into_iter()
        .map(|cell| cell.id)
        .filter(|cell_id| cell_ids.contains(cell_id))
        .collect()
}

pub(crate) fn layout_cell_list_label(
    document: &Document,
    cell_ids: &[CellId],
    limit: usize,
) -> String {
    if cell_ids.is_empty() {
        return "None".to_string();
    }
    let mut label = cell_ids
        .iter()
        .take(limit)
        .map(|cell_id| layout_cell_display_name(document, *cell_id))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = cell_ids.len().saturating_sub(limit);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 64)
}

pub(crate) fn layout_cell_property_rows(app: &GlassworksApp, cell: &Cell) -> Vec<(String, String)> {
    const MAX_CELL_PROPERTY_ROWS: usize = 6;
    let mut rows = Vec::new();
    if let Some(macro_name) = cell.properties.get("library.macro") {
        if macro_name == "via_array" {
            let columns = cell
                .properties
                .get("library.columns")
                .map(String::as_str)
                .unwrap_or("?");
            let rows_value = cell
                .properties
                .get("library.rows")
                .map(String::as_str)
                .unwrap_or("?");
            let size = cell
                .properties
                .get("library.via_size_dbu")
                .and_then(|value| value.parse::<Coord>().ok())
                .map(|value| app.format_layout_length(value as f64))
                .unwrap_or_else(|| "unknown".to_string());
            let pitch = cell
                .properties
                .get("library.pitch_dbu")
                .and_then(|value| value.parse::<Coord>().ok())
                .map(|value| app.format_layout_length(value as f64))
                .unwrap_or_else(|| "unknown".to_string());
            rows.push(("Library macro".to_string(), "Via array".to_string()));
            rows.push((
                "Macro parameters".to_string(),
                format!("{columns}x{rows_value}, size {size}, pitch {pitch}"),
            ));
        } else {
            rows.push(("Library macro".to_string(), macro_name.clone()));
        }
    }
    if !cell.properties.is_empty() {
        rows.push((
            "Properties".to_string(),
            format!("{} stored", cell.properties.len()),
        ));
        for (key, value) in cell.properties.iter().take(MAX_CELL_PROPERTY_ROWS) {
            rows.push((
                format!("Property {}", compact_button_label(key, 24)),
                compact_button_label(value, 48),
            ));
        }
        if cell.properties.len() > MAX_CELL_PROPERTY_ROWS {
            rows.push((
                "Properties omitted".to_string(),
                (cell.properties.len() - MAX_CELL_PROPERTY_ROWS).to_string(),
            ));
        }
    }
    rows
}

pub(crate) fn layout_cell_local_shape_count(document: &Document, cell: &Cell) -> usize {
    cell.shapes.len()
        + if cell.id == document.top_cell {
            document.shapes.len()
        } else {
            0
        }
}

pub(crate) fn layout_cell_local_bounds(document: &Document, cell: &Cell) -> Option<Rect> {
    let cell_bounds = cell
        .shapes
        .values()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right));
    if cell.id == document.top_cell {
        document
            .shapes
            .values()
            .map(|shape| shape.kind.bounds())
            .chain(cell_bounds)
            .reduce(|left, right| left.union(right))
    } else {
        cell_bounds
    }
}

pub(crate) fn layout_cell_hierarchy_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let search = layout_browser_search_query_lower(app);
    let entries = layout_cell_hierarchy_entries(app, search.is_some());
    let mut rows = Vec::new();
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed cell rows".to_string(),
            format!(
                "{} / {} rows",
                layout_cell_hierarchy_listed_count(&entries, query),
                entries.len()
            ),
        ));
    }
    rows.extend(
        entries
            .into_iter()
            .filter(|entry| {
                search
                    .as_deref()
                    .is_none_or(|query| layout_cell_hierarchy_entry_matches_search(entry, query))
            })
            .map(|entry| (entry.key, entry.value)),
    );
    rows
}

#[derive(Clone, Debug)]
struct LayoutCellHierarchyEntry {
    key: String,
    value: String,
    fields: Vec<(String, String)>,
}

fn layout_cell_hierarchy_entries(
    app: &GlassworksApp,
    include_all_cells: bool,
) -> Vec<LayoutCellHierarchyEntry> {
    let document = &app.workspace.document;
    let mut rows = Vec::new();
    let view_top_cell_name = layout_view_top_cell_name(app);
    rows.push(layout_cell_hierarchy_entry(
        "View top cell",
        view_top_cell_name.clone(),
        vec![
            ("view_top".to_string(), view_top_cell_name.clone()),
            ("view_top_cell".to_string(), view_top_cell_name),
            (
                "view_top_cell_id".to_string(),
                app.layout_view_top_cell.0.to_string(),
            ),
        ],
    ));
    let document_top_cell_name = document
        .cell(document.top_cell)
        .map(|cell| cell.name.clone())
        .unwrap_or_else(|| format!("cell {}", document.top_cell.0));
    rows.push(layout_cell_hierarchy_entry(
        "Document top",
        document_top_cell_name.clone(),
        vec![
            ("document_top".to_string(), document_top_cell_name.clone()),
            ("document_top_cell".to_string(), document_top_cell_name),
            (
                "document_top_cell_id".to_string(),
                document.top_cell.0.to_string(),
            ),
        ],
    ));
    let placed_instances = document
        .cells
        .values()
        .map(|cell| cell.instances.len())
        .sum::<usize>();
    rows.push(layout_cell_hierarchy_entry(
        "Placed instances",
        placed_instances.to_string(),
        vec![
            ("placed_instances".to_string(), placed_instances.to_string()),
            ("instances".to_string(), placed_instances.to_string()),
        ],
    ));
    let cell_limit = if include_all_cells { usize::MAX } else { 4 };
    for cell in document.cells.values().take(cell_limit) {
        let suffix = if cell.id == document.top_cell {
            " top"
        } else {
            ""
        };
        let local_shape_count = layout_cell_local_shape_count(document, cell);
        let child_instances = cell.instances.len();
        let role = if cell.id == document.top_cell {
            "top"
        } else if layout_parent_cells(document, cell.id).is_empty() {
            "unused"
        } else {
            "used"
        };
        let mut fields = vec![
            ("cell".to_string(), cell.name.clone()),
            ("cell_id".to_string(), cell.id.0.to_string()),
            ("name".to_string(), cell.name.clone()),
            ("role".to_string(), role.to_string()),
            ("shapes".to_string(), local_shape_count.to_string()),
            ("local_shapes".to_string(), local_shape_count.to_string()),
            ("child_instances".to_string(), child_instances.to_string()),
            ("instances".to_string(), child_instances.to_string()),
        ];
        if cell.id == app.layout_view_top_cell {
            fields.push(("state".to_string(), "view_top".to_string()));
        }
        if cell.id == document.top_cell {
            fields.push(("state".to_string(), "document_top".to_string()));
        }
        rows.push(layout_cell_hierarchy_entry(
            cell.name.clone(),
            format!(
                "{} shapes / {} child instances{}",
                local_shape_count, child_instances, suffix
            ),
            fields,
        ));
    }
    if let Some(occurrence) = &app.selected_layout_occurrence
        && let Some((parent, instance_id)) = document
            .instance_parent_for_path_from_cell(app.layout_view_top_cell, &occurrence.instance_path)
        && let Some(instance) = document.instance(parent, instance_id)
    {
        rows.push(layout_cell_hierarchy_entry(
            "Selected instance",
            format!("inst #{} -> cell #{}", instance.id.0, instance.cell.0),
            vec![
                ("selected".to_string(), "instance".to_string()),
                ("selected_instance".to_string(), instance.id.0.to_string()),
                ("instance_id".to_string(), instance.id.0.to_string()),
                ("parent_cell_id".to_string(), parent.0.to_string()),
                ("target_cell_id".to_string(), instance.cell.0.to_string()),
            ],
        ));
        rows.push(layout_cell_hierarchy_entry(
            "Instance origin",
            format!(
                "{},{}",
                instance.transform.translation.dx, instance.transform.translation.dy
            ),
            vec![
                ("selected".to_string(), "instance".to_string()),
                (
                    "origin".to_string(),
                    format!(
                        "{},{}",
                        instance.transform.translation.dx, instance.transform.translation.dy
                    ),
                ),
                (
                    "origin_x".to_string(),
                    instance.transform.translation.dx.to_string(),
                ),
                (
                    "origin_y".to_string(),
                    instance.transform.translation.dy.to_string(),
                ),
            ],
        ));
    } else if let Some(shape) = app.selected_top_level_layout_shape() {
        rows.push(layout_cell_hierarchy_entry(
            "Selected shape",
            format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)),
            vec![
                ("selected".to_string(), "shape".to_string()),
                ("selected_shape".to_string(), shape.id.0.to_string()),
                ("shape_id".to_string(), shape.id.0.to_string()),
                (
                    "kind".to_string(),
                    shape_kind_label(&shape.kind).to_string(),
                ),
            ],
        ));
    }
    rows
}

fn layout_cell_hierarchy_entry(
    key: impl Into<String>,
    value: impl Into<String>,
    fields: Vec<(String, String)>,
) -> LayoutCellHierarchyEntry {
    let key = key.into();
    let value = value.into();
    let row_selector = layout_cell_hierarchy_row_selector_key(&key);
    let mut entry_fields = vec![
        ("section".to_string(), "cells".to_string()),
        ("row".to_string(), key.clone()),
        ("label".to_string(), key.clone()),
        ("value".to_string(), value.clone()),
        (row_selector, value.clone()),
    ];
    entry_fields.extend(fields);
    LayoutCellHierarchyEntry {
        key,
        value,
        fields: entry_fields,
    }
}

fn layout_cell_hierarchy_listed_count(entries: &[LayoutCellHierarchyEntry], query: &str) -> usize {
    entries
        .iter()
        .filter(|entry| layout_cell_hierarchy_entry_matches_search(entry, query))
        .count()
}

fn layout_cell_hierarchy_entry_matches_search(
    entry: &LayoutCellHierarchyEntry,
    query_lower: &str,
) -> bool {
    if layout_cell_hierarchy_selector_matches_search(entry, query_lower) {
        return true;
    }
    let mut fields = vec![entry.key.clone(), entry.value.clone()];
    for (key, value) in &entry.fields {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
        push_layout_search_field(&mut fields, format!("{key}={value}"));
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_cell_hierarchy_selector_matches_search(
    entry: &LayoutCellHierarchyEntry,
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
    entry
        .fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn layout_cell_hierarchy_row_selector_key(label: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !output.is_empty() {
            output.push('_');
            last_was_separator = true;
        }
    }
    if output.ends_with('_') {
        output.pop();
    }
    output
}

pub(crate) fn layout_shape_browser_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_shape_browser_entries(app);
    let selected = app
        .selected_layout_occurrence
        .as_ref()
        .map(layout_occurrence_label)
        .unwrap_or_else(|| "None".to_string());
    let mut rows = vec![
        ("View top".to_string(), layout_view_top_cell_name(app)),
        ("Visible listed".to_string(), entries.len().to_string()),
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        (
            "Filter".to_string(),
            app.layout_shape_browser_filter.label().to_string(),
        ),
        (
            "Sort".to_string(),
            app.layout_shape_browser_sort.label().to_string(),
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
        ("Selected".to_string(), selected),
    ];
    rows.extend(layout_shape_browser_property_rows(app, &entries));
    rows
}

pub(crate) fn layout_hierarchy_tree_summary_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let rows = layout_hierarchy_tree_rows(app);
    if layout_browser_search_query_lower(app).is_some() {
        let mut summary_rows = vec![
            (
                "Search".to_string(),
                layout_browser_search_display_value(app),
            ),
            ("Listed tree rows".to_string(), rows.len().to_string()),
        ];
        summary_rows.extend(rows.iter().map(|row| {
            (
                format!("Tree C{}", row.cell.0),
                layout_hierarchy_tree_row_summary(app, row),
            )
        }));
        return summary_rows;
    }
    vec![
        ("Root".to_string(), layout_view_top_cell_name(app)),
        ("Visible rows".to_string(), rows.len().to_string()),
        (
            "Collapsed cells".to_string(),
            app.layout_tree_collapsed_cells.len().to_string(),
        ),
        (
            "Hidden cells".to_string(),
            app.layout_hidden_cells.len().to_string(),
        ),
    ]
}

fn layout_hierarchy_tree_row_summary(app: &GlassworksApp, row: &LayoutHierarchyTreeRow) -> String {
    let name = app
        .workspace
        .document
        .cell(row.cell)
        .map(|cell| compact_button_label(&cell.name, 24))
        .unwrap_or_else(|| format!("cell {}", row.cell.0));
    let mut states = Vec::new();
    if row.current {
        states.push("current");
    }
    if row.hidden {
        states.push("hidden");
    }
    if row.collapsed {
        states.push("collapsed");
    }
    if row.has_children {
        states.push("branch");
    } else {
        states.push("leaf");
    }
    format!("{} depth {}, {}", name, row.depth, states.join(", "))
}

pub(crate) fn layout_instance_browser_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_instance_browser_entries(app);
    let selected = selected_layout_instance_key(app)
        .map(|(parent, id)| format!("C{} instance #{}", parent.0, id.0))
        .unwrap_or_else(|| "None".to_string());
    let mut rows = vec![
        ("View top".to_string(), layout_view_top_cell_name(app)),
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        (
            "Scope".to_string(),
            app.layout_instance_browser_scope.label().to_string(),
        ),
        (
            "Filter".to_string(),
            app.layout_instance_browser_filter.label().to_string(),
        ),
        (
            "Sort".to_string(),
            app.layout_instance_browser_sort.label().to_string(),
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
        ("Child instances".to_string(), entries.len().to_string()),
        ("Selected".to_string(), selected),
    ];
    rows.extend(layout_instance_browser_property_rows(app, &entries));
    rows
}

pub(crate) fn layout_view_top_cell_name(app: &GlassworksApp) -> String {
    app.workspace
        .document
        .cell(app.layout_view_top_cell)
        .map(|cell| cell.name.clone())
        .unwrap_or_else(|| format!("cell {}", app.layout_view_top_cell.0))
}

pub(crate) fn layout_connectivity_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_connectivity_entries(app);
    let search = layout_browser_search_query_lower(app);
    let mut rows = Vec::new();
    if let Some(query) = search.as_deref() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push((
            "Listed connectivity rows".to_string(),
            format!(
                "{} / {} rows",
                layout_connectivity_listed_count(&entries, query),
                entries.len()
            ),
        ));
    }
    rows.extend(
        entries
            .into_iter()
            .filter(|entry| {
                search
                    .as_deref()
                    .is_none_or(|query| layout_connectivity_entry_matches_search(entry, query))
            })
            .map(|entry| (entry.key, entry.value)),
    );
    rows
}

#[derive(Clone, Debug)]
struct LayoutConnectivityEntry {
    key: String,
    value: String,
    fields: Vec<(String, String)>,
}

fn layout_connectivity_entries(app: &GlassworksApp) -> Vec<LayoutConnectivityEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        let shape_count = document.flattened_shape_count_estimate();
        return vec![layout_connectivity_entry(
            "Status",
            format!("Skipped over {} shapes", shape_count),
            vec![
                ("status".to_string(), "skipped".to_string()),
                ("shapes".to_string(), shape_count.to_string()),
                ("shape_count".to_string(), shape_count.to_string()),
            ],
        )];
    }
    match app.connectivity_report() {
        Ok(report) => {
            let summary = report.summary(8);
            let health = summary
                .skipped
                .clone()
                .unwrap_or_else(|| summary.health.label().to_string());
            let mut entries = vec![
                layout_connectivity_entry(
                    "Health",
                    health.clone(),
                    vec![
                        ("health".to_string(), health.clone()),
                        ("status".to_string(), health),
                    ],
                ),
                layout_connectivity_entry(
                    "Components",
                    summary.component_count.to_string(),
                    vec![
                        (
                            "components".to_string(),
                            summary.component_count.to_string(),
                        ),
                        (
                            "component_count".to_string(),
                            summary.component_count.to_string(),
                        ),
                    ],
                ),
                layout_connectivity_entry(
                    "Devices",
                    summary.device_count.to_string(),
                    vec![
                        ("devices".to_string(), summary.device_count.to_string()),
                        ("device_count".to_string(), summary.device_count.to_string()),
                    ],
                ),
                layout_connectivity_entry(
                    "Labeled nets",
                    summary.labeled_component_count.to_string(),
                    vec![
                        (
                            "labeled_nets".to_string(),
                            summary.labeled_component_count.to_string(),
                        ),
                        (
                            "labeled_components".to_string(),
                            summary.labeled_component_count.to_string(),
                        ),
                    ],
                ),
                layout_connectivity_entry(
                    "Shorts",
                    summary.short_count.to_string(),
                    vec![
                        ("shorts".to_string(), summary.short_count.to_string()),
                        ("short_count".to_string(), summary.short_count.to_string()),
                    ],
                ),
                layout_connectivity_entry(
                    "Opens",
                    summary.open_count.to_string(),
                    vec![
                        ("opens".to_string(), summary.open_count.to_string()),
                        ("open_count".to_string(), summary.open_count.to_string()),
                    ],
                ),
                layout_connectivity_entry(
                    "Largest net",
                    format!("{} shapes", summary.largest_component_shape_count),
                    vec![
                        (
                            "largest_net".to_string(),
                            summary.largest_component_shape_count.to_string(),
                        ),
                        (
                            "largest_shapes".to_string(),
                            summary.largest_component_shape_count.to_string(),
                        ),
                    ],
                ),
                layout_connectivity_entry(
                    "Saved states",
                    document.connectivity_issue_states.len().to_string(),
                    vec![
                        (
                            "saved_states".to_string(),
                            document.connectivity_issue_states.len().to_string(),
                        ),
                        (
                            "issue_states".to_string(),
                            document.connectivity_issue_states.len().to_string(),
                        ),
                    ],
                ),
            ];
            entries.extend(
                layout_connectivity_source_summary_rows(app)
                    .into_iter()
                    .map(|(selector, label, value)| {
                        let mut fields = vec![(selector.to_string(), value.clone())];
                        fields.push(("source_metadata".to_string(), value.clone()));
                        layout_connectivity_entry(label, value, fields)
                    }),
            );
            entries.extend(
                layout_trace_state_summary_rows(app, &report)
                    .into_iter()
                    .map(|(selector, label, value)| {
                        layout_connectivity_entry(
                            label,
                            value.clone(),
                            vec![(selector.to_string(), value)],
                        )
                    }),
            );
            entries.extend(
                layout_spice_comparison_rows(app)
                    .into_iter()
                    .map(|(label, value)| layout_connectivity_generic_entry(label, value)),
            );
            entries
        }
        Err(error) => vec![layout_connectivity_entry(
            "Status",
            format!("Failed: {error}"),
            vec![
                ("status".to_string(), "failed".to_string()),
                ("error".to_string(), error.to_string()),
            ],
        )],
    }
}

fn layout_connectivity_entry(
    key: impl Into<String>,
    value: impl Into<String>,
    fields: Vec<(String, String)>,
) -> LayoutConnectivityEntry {
    let key = key.into();
    let value = value.into();
    let row_selector = layout_connectivity_row_selector_key(&key);
    let mut entry_fields = vec![
        ("section".to_string(), "connectivity".to_string()),
        ("row".to_string(), key.clone()),
        ("label".to_string(), key.clone()),
        ("value".to_string(), value.clone()),
        (row_selector, value.clone()),
    ];
    entry_fields.extend(fields);
    LayoutConnectivityEntry {
        key,
        value,
        fields: entry_fields,
    }
}

fn layout_connectivity_generic_entry(
    key: impl Into<String>,
    value: impl Into<String>,
) -> LayoutConnectivityEntry {
    layout_connectivity_entry(key, value, Vec::new())
}

fn layout_connectivity_listed_count(entries: &[LayoutConnectivityEntry], query: &str) -> usize {
    entries
        .iter()
        .filter(|entry| layout_connectivity_entry_matches_search(entry, query))
        .count()
}

fn layout_connectivity_entry_matches_search(
    entry: &LayoutConnectivityEntry,
    query_lower: &str,
) -> bool {
    if layout_connectivity_selector_matches_search(entry, query_lower) {
        return true;
    }
    let mut fields = vec![entry.key.clone(), entry.value.clone()];
    for (key, value) in &entry.fields {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
        push_layout_search_field(&mut fields, format!("{key}={value}"));
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_connectivity_selector_matches_search(
    entry: &LayoutConnectivityEntry,
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
    entry
        .fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn layout_connectivity_row_selector_key(label: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !output.is_empty() {
            output.push('_');
            last_was_separator = true;
        }
    }
    if output.ends_with('_') {
        output.pop();
    }
    output
}

pub(crate) fn layout_net_browser_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return vec![(
            "Status".to_string(),
            format!(
                "Skipped over {} shapes",
                document.flattened_shape_count_estimate()
            ),
        )];
    }
    match app.connectivity_report() {
        Ok(report) => {
            let entries = layout_net_browser_entries_from_report_with_history(
                &report,
                app.layout_net_browser_filter,
                app.layout_net_browser_sort,
                layout_browser_search_query_lower(app).as_deref(),
                &app.layout_trace_history,
                app.layout_spice_comparison.as_ref(),
            );
            let selected = app
                .selected_layout_occurrence
                .as_ref()
                .and_then(|occurrence| {
                    layout_component_for_occurrence_or_label(&report, occurrence)
                })
                .and_then(|component_id| report.component(component_id))
                .map(connectivity_component_display_name)
                .unwrap_or_else(|| "None".to_string());
            let mut rows = vec![
                (
                    "Search".to_string(),
                    layout_browser_search_display_value(app),
                ),
                ("Listed nets".to_string(), entries.len().to_string()),
                (
                    "Total nets".to_string(),
                    report.components.len().to_string(),
                ),
                (
                    "Filter".to_string(),
                    app.layout_net_browser_filter.label().to_string(),
                ),
                (
                    "Sort".to_string(),
                    app.layout_net_browser_sort.label().to_string(),
                ),
                (
                    "Columns".to_string(),
                    app.layout_browser_columns.label().to_string(),
                ),
                ("Selected".to_string(), selected),
            ];
            rows.extend(
                layout_connectivity_source_summary_rows(app)
                    .into_iter()
                    .map(|(_, label, value)| (label.to_string(), value)),
            );
            rows.extend(
                layout_trace_state_summary_rows(app, &report)
                    .into_iter()
                    .map(|(_, label, value)| (label.to_string(), value)),
            );
            rows.extend(layout_trace_point_summary_rows(app));
            rows.extend(layout_trace_path_segment_summary_rows(app));
            rows.extend(layout_spice_comparison_rows(app));
            rows.extend(layout_net_browser_property_rows(app, &report, &entries));
            rows
        }
        Err(error) => vec![("Status".to_string(), format!("Failed: {error}"))],
    }
}

pub(crate) fn layout_spice_comparison_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let Some(comparison) = app.layout_spice_comparison.as_ref() else {
        return Vec::new();
    };
    let issue_counts = app.connectivity_report().ok().map(|report| {
        (
            layout_spice_missing_issue_net_count(&report, comparison),
            layout_spice_extra_issue_net_count(&report, comparison),
            layout_spice_net_issue_net_count(&report, comparison),
            layout_spice_device_issue_net_count(&report, comparison),
            layout_spice_issue_net_count(&report, comparison),
        )
    });
    let mut rows = vec![
        (
            "SPICE Compare".to_string(),
            comparison.status.label().to_string(),
        ),
        (
            "SPICE Circuit".to_string(),
            compact_button_label(&comparison.circuit_name, 32),
        ),
        (
            "SPICE Devices".to_string(),
            comparison.schematic_device_count.to_string(),
        ),
        (
            "Layout Devices".to_string(),
            comparison.layout_device_count.to_string(),
        ),
        (
            "SPICE Nets".to_string(),
            comparison.schematic_referenced_nets.len().to_string(),
        ),
        (
            "Missing Layout Nets".to_string(),
            compact_net_name_list(&comparison.missing_layout_nets),
        ),
        (
            "Extra Layout Nets".to_string(),
            compact_net_name_list(&comparison.extra_layout_nets),
        ),
        (
            "Layout Issues".to_string(),
            comparison.layout_issue_count().to_string(),
        ),
        (
            "Missing Layout Devices".to_string(),
            comparison.missing_layout_devices.len().to_string(),
        ),
        (
            "Extra Layout Devices".to_string(),
            comparison.extra_layout_devices.len().to_string(),
        ),
    ];
    if let Some((missing_count, extra_count, net_count, device_count, issue_count)) = issue_counts {
        rows.push((
            "SPICE Missing Issue Nets".to_string(),
            missing_count.to_string(),
        ));
        rows.push((
            "SPICE Extra Issue Nets".to_string(),
            extra_count.to_string(),
        ));
        rows.push(("SPICE Net Issue Nets".to_string(), net_count.to_string()));
        rows.push((
            "SPICE Device Issue Nets".to_string(),
            device_count.to_string(),
        ));
        rows.push(("SPICE Issue Nets".to_string(), issue_count.to_string()));
    }
    if !comparison.missing_layout_devices.is_empty() {
        rows.push((
            "Missing Device Detail".to_string(),
            compact_spice_signature_list(&comparison.missing_layout_devices),
        ));
    }
    if !comparison.extra_layout_devices.is_empty() {
        rows.push((
            "Extra Device Detail".to_string(),
            compact_spice_signature_list(&comparison.extra_layout_devices),
        ));
    }
    rows
}

pub(crate) fn compact_net_name_list(names: &[String]) -> String {
    if names.is_empty() {
        return "0".to_string();
    }
    let mut preview = names.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > 3 {
        preview.push_str(&format!(" +{}", names.len() - 3));
    }
    compact_button_label(&preview, 42)
}

pub(crate) fn compact_spice_signature_list(signatures: &[String]) -> String {
    if signatures.is_empty() {
        return "0".to_string();
    }
    let mut preview = signatures
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    if signatures.len() > 2 {
        preview.push_str(&format!(" +{}", signatures.len() - 2));
    }
    compact_button_label(&preview, 64)
}

pub(crate) fn layout_trace_history_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_trace_history_entries(app);
    let latest = entries
        .first()
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| "None".to_string());
    vec![
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        ("Recent traces".to_string(), entries.len().to_string()),
        (
            "Stored traces".to_string(),
            app.layout_trace_history.len().to_string(),
        ),
        ("Latest".to_string(), latest),
    ]
}

pub(crate) fn layout_drc_marker_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let shape_count = document.flattened_shape_count_estimate();
    if shape_count > MAX_DRC_INSPECTOR_SHAPES {
        return vec![(
            "Status".to_string(),
            format!("Skipped over {shape_count} shapes"),
        )];
    }
    let report_count = app.layout_drc_report_history.len();
    let listed_report_count = layout_drc_report_browser_entries(app).len();
    let stored_marker_count = app
        .layout_drc_report_history
        .iter()
        .map(|entry| entry.value.violations.len())
        .sum::<usize>();
    let stored_diagnostic_count = app
        .layout_drc_report_history
        .iter()
        .map(|entry| entry.value.findings.len())
        .sum::<usize>();
    let current_report_count = app
        .layout_drc_report_history
        .iter()
        .filter(|entry| entry.revision == app.layout_revision)
        .count();
    let stale_report_count = app
        .layout_drc_report_history
        .len()
        .saturating_sub(current_report_count);
    let report_source_entries = layout_drc_report_source_entries(app);
    let active_report_source = app
        .selected_drc_report_history_entry()
        .and_then(|entry| entry.source.as_deref())
        .map(str::trim)
        .filter(|source| !source.is_empty())
        .map(|source| compact_button_label(source, 64))
        .unwrap_or_else(|| "None".to_string());
    let active_report = app
        .selected_drc_report_history_entry()
        .map(|entry| {
            let status = if entry.revision == app.layout_revision {
                "current"
            } else {
                "stale"
            };
            format!("{} ({status})", entry.label)
        })
        .unwrap_or_else(|| "None".to_string());
    let Some(report) = app.drc_report() else {
        return vec![
            ("Status".to_string(), "Not run".to_string()),
            ("Shapes".to_string(), shape_count.to_string()),
            ("Reports".to_string(), report_count.to_string()),
            ("Active report".to_string(), active_report),
            (
                "Overlay".to_string(),
                if app.show_drc_overlay {
                    "waiting"
                } else {
                    "hidden"
                }
                .to_string(),
            ),
        ];
    };
    if !report.findings.is_empty() && report.violations.is_empty() {
        let mut rows = vec![
            ("Reports".to_string(), report_count.to_string()),
            ("Active report".to_string(), active_report),
        ];
        rows.extend(layout_drc_report_diagnostic_rows(&report.findings, 3));
        return rows;
    }

    let active = report
        .violations
        .iter()
        .filter(|violation| drc_violation_is_active(document, violation))
        .count();
    let visited = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .is_some_and(|state| state.visited)
        })
        .count();
    let important = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .is_some_and(|state| state.important)
        })
        .count();
    let noted = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .and_then(|state| state.note.as_deref())
                .is_some_and(|note| !note.trim().is_empty())
        })
        .count();
    let owned = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .and_then(|state| state.owner.as_deref())
                .is_some_and(|owner| !owner.trim().is_empty())
        })
        .count();
    let signed_off = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .is_some_and(|state| {
                    state
                        .signoff
                        .as_deref()
                        .is_some_and(|signoff| !signoff.trim().is_empty())
                        || !state.signoff_records.is_empty()
                })
        })
        .count();
    let tagged = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .is_some_and(|state| !state.tags.is_empty())
        })
        .count();
    let snapshots = report
        .violations
        .iter()
        .filter(|violation| {
            document
                .marker_states
                .get(&violation.stable_key())
                .is_some_and(layout_marker_state_has_snapshot)
        })
        .count();
    let selected = app
        .layout_selected_drc_marker_key
        .as_ref()
        .and_then(|key| {
            report
                .violations
                .iter()
                .find(|violation| violation.stable_key() == *key)
        })
        .map(|violation| format!("#{} {}", violation.id, violation.rule))
        .unwrap_or_else(|| "None".to_string());
    let rule_count = report
        .violations
        .iter()
        .map(|violation| violation.rule.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let entries = layout_drc_marker_entries(app);
    let entry_keys = entries
        .iter()
        .map(|entry| entry.key.as_str())
        .collect::<BTreeSet<_>>();
    let listed_violations = report
        .violations
        .iter()
        .filter(|violation| entry_keys.contains(violation.stable_key().as_str()))
        .collect::<Vec<_>>();
    let listed_count_map_label = |counts: &BTreeMap<String, usize>, max_chars: usize| {
        if counts.is_empty() {
            "None".to_string()
        } else {
            compact_button_label(
                &counts
                    .iter()
                    .map(|(label, count)| format!("{label}={count}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                max_chars,
            )
        }
    };
    let mut listed_rule_counts = BTreeMap::<String, usize>::new();
    let mut listed_category_counts = BTreeMap::<String, usize>::new();
    let mut listed_directory_counts = BTreeMap::<String, usize>::new();
    let mut listed_message_counts = BTreeMap::<String, usize>::new();
    for violation in &listed_violations {
        let state = document.marker_states.get(&violation.stable_key());
        *listed_rule_counts
            .entry(violation.rule.clone())
            .or_default() += 1;
        *listed_category_counts
            .entry(layout_drc_marker_category_label(violation, state))
            .or_default() += 1;
        *listed_directory_counts
            .entry(layout_drc_marker_directory_path_with_state(
                violation, state,
            ))
            .or_default() += 1;
        let message = violation.message.trim();
        if !message.is_empty() {
            *listed_message_counts
                .entry(message.to_string())
                .or_default() += 1;
        }
    }
    let listed_marker_ids_label = if listed_violations.is_empty() {
        "None".to_string()
    } else {
        let mut label = listed_violations
            .iter()
            .take(6)
            .map(|violation| format!("#{}", violation.id))
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = listed_violations.len().saturating_sub(6);
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
        compact_button_label(&label, 64)
    };
    let listed_messages_label = listed_count_map_label(&listed_message_counts, 120);
    let listed_rules_label = listed_count_map_label(&listed_rule_counts, 96);
    let listed_categories_label = listed_count_map_label(&listed_category_counts, 96);
    let listed_directories_label = listed_count_map_label(&listed_directory_counts, 120);
    let listed_marker_bounds_label = listed_violations
        .iter()
        .map(|violation| violation.bounds)
        .reduce(Rect::union)
        .map(rect_summary)
        .unwrap_or_else(|| "None".to_string());
    let listed_marker_area_label = if listed_violations.is_empty() {
        "None".to_string()
    } else {
        let area_sum = listed_violations
            .iter()
            .map(|violation| violation.bounds.area())
            .sum::<i64>();
        let max_area = listed_violations
            .iter()
            .map(|violation| violation.bounds.area())
            .max()
            .unwrap_or_default();
        format!("sum {area_sum} dbu^2, max {max_area} dbu^2")
    };
    let listed_required_label = match (
        listed_violations
            .iter()
            .map(|violation| violation.required)
            .min(),
        listed_violations
            .iter()
            .map(|violation| violation.required)
            .max(),
    ) {
        (Some(min), Some(max)) if min == max => app.format_layout_length(min as f64),
        (Some(min), Some(max)) => format!(
            "{}..{}",
            app.format_layout_length(min as f64),
            app.format_layout_length(max as f64)
        ),
        _ => "None".to_string(),
    };
    let listed_actual_label = if listed_violations.is_empty() {
        "None".to_string()
    } else {
        let min = listed_violations
            .iter()
            .map(|violation| violation.actual)
            .fold(f64::INFINITY, f64::min);
        let max = listed_violations
            .iter()
            .map(|violation| violation.actual)
            .fold(f64::NEG_INFINITY, f64::max);
        if (min - max).abs() < f64::EPSILON {
            format!("{min:.1}")
        } else {
            format!("{min:.1}..{max:.1}")
        }
    };
    let listed_source_shapes = listed_violations
        .iter()
        .flat_map(|violation| layout_drc_marker_source_shapes(document, violation))
        .collect::<Vec<_>>();
    let mut listed_source_objects = BTreeSet::new();
    let mut listed_source_object_entries = Vec::new();
    for (shape_id, shape) in &listed_source_shapes {
        if listed_source_objects.insert(*shape_id) {
            listed_source_object_entries.push((*shape_id, shape));
        }
    }
    let listed_source_objects_label = if listed_source_object_entries.is_empty() {
        "None".to_string()
    } else {
        let mut label = listed_source_object_entries
            .iter()
            .take(3)
            .map(|(shape_id, shape)| {
                format!(
                    "#{} {} {}",
                    shape_id.0,
                    shape_kind_label(&shape.kind),
                    layout_layer_display_name(document, shape.layer)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = listed_source_object_entries.len().saturating_sub(3);
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
        compact_button_label(&label, 64)
    };
    let mut listed_source_layers = listed_source_shapes
        .iter()
        .map(|(_, shape)| shape.layer)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    listed_source_layers.sort_by_key(|layer_id| {
        document
            .layer(*layer_id)
            .map(|layer| (layer.display_order, layer.id))
            .unwrap_or((i32::MAX, *layer_id))
    });
    let listed_source_layers_label = if listed_source_layers.is_empty() {
        "None".to_string()
    } else {
        let mut label = listed_source_layers
            .iter()
            .take(4)
            .map(|layer_id| layout_layer_display_name(document, *layer_id))
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = listed_source_layers.len().saturating_sub(4);
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
        compact_button_label(&label, 52)
    };
    let mut listed_source_kinds = Vec::new();
    for (_, shape) in &listed_source_shapes {
        let kind = shape_kind_label(&shape.kind);
        if !listed_source_kinds.contains(&kind) {
            listed_source_kinds.push(kind);
        }
    }
    let listed_source_kinds_label = if listed_source_kinds.is_empty() {
        "None".to_string()
    } else {
        compact_button_label(&listed_source_kinds.join(", "), 52)
    };
    let listed_source_bounds_label = listed_source_shapes
        .iter()
        .map(|(_, shape)| shape.kind.bounds())
        .reduce(Rect::union)
        .map(rect_summary)
        .unwrap_or_else(|| "None".to_string());
    let listed_source_cells = listed_violations
        .iter()
        .flat_map(|violation| layout_drc_marker_source_cells(document, violation))
        .collect::<BTreeSet<_>>();
    let listed_source_cells_label = if listed_source_cells.is_empty() {
        "None".to_string()
    } else {
        let cells = listed_source_cells.iter().copied().collect::<Vec<_>>();
        let mut label = cells
            .iter()
            .take(3)
            .map(|cell_id| layout_cell_display_name(document, *cell_id))
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = cells.len().saturating_sub(3);
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
        compact_button_label(&label, 52)
    };
    let listed_source_occurrences = listed_violations
        .iter()
        .flat_map(|violation| violation.occurrence_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let listed_source_occurrences_label = if listed_source_occurrences.is_empty() {
        "None".to_string()
    } else {
        let mut label = listed_source_occurrences
            .iter()
            .take(3)
            .map(layout_occurrence_label)
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = listed_source_occurrences.len().saturating_sub(3);
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
        compact_button_label(&label, 48)
    };
    let listed_cross_probe_targets = layout_drc_marker_cross_probe_targets_label(
        listed_source_objects.len(),
        listed_source_cells.len(),
        listed_source_occurrences.len(),
    );
    let listed_review_states = layout_drc_marker_review_summary_label(document, &entries);
    let listed_signoff_audit = layout_drc_marker_signoff_audit_summary_label(document, &entries);
    let listed_tags = layout_drc_marker_tag_summary_label(document, &entries);
    let listed_snapshots = layout_drc_marker_snapshot_summary_label(document, &entries);
    let category_entries = layout_drc_marker_category_entries(app);
    let category_filter = app
        .layout_drc_marker_category_filter
        .map(|category| category.label())
        .unwrap_or("All");
    let directory_filter = app
        .layout_drc_marker_directory_filter
        .as_deref()
        .map(|path| compact_button_label(path, 52))
        .unwrap_or_else(|| "All".to_string());
    let mut rows = vec![
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        ("Active".to_string(), active.to_string()),
        ("Total".to_string(), report.violations.len().to_string()),
        ("Listed".to_string(), entries.len().to_string()),
        ("Listed marker ids".to_string(), listed_marker_ids_label),
        ("Listed messages".to_string(), listed_messages_label),
        ("Listed rules".to_string(), listed_rules_label),
        ("Listed categories".to_string(), listed_categories_label),
        ("Listed directories".to_string(), listed_directories_label),
        (
            "Listed marker bounds".to_string(),
            listed_marker_bounds_label,
        ),
        ("Listed marker area".to_string(), listed_marker_area_label),
        ("Listed required".to_string(), listed_required_label),
        ("Listed actual".to_string(), listed_actual_label),
        ("Listed review states".to_string(), listed_review_states),
        ("Listed signoff audit".to_string(), listed_signoff_audit),
        ("Listed tags".to_string(), listed_tags),
        ("Listed snapshots".to_string(), listed_snapshots),
        (
            "Listed source objects".to_string(),
            listed_source_objects.len().to_string(),
        ),
        (
            "Listed source object labels".to_string(),
            listed_source_objects_label,
        ),
        (
            "Listed source cells".to_string(),
            listed_source_cells.len().to_string(),
        ),
        (
            "Listed source cell labels".to_string(),
            listed_source_cells_label,
        ),
        (
            "Listed source layers".to_string(),
            listed_source_layers_label,
        ),
        ("Listed source kinds".to_string(), listed_source_kinds_label),
        (
            "Listed source bounds".to_string(),
            listed_source_bounds_label,
        ),
        (
            "Listed source occurrences".to_string(),
            listed_source_occurrences.len().to_string(),
        ),
        (
            "Listed occurrence labels".to_string(),
            listed_source_occurrences_label,
        ),
        (
            "Listed cross-probe targets".to_string(),
            listed_cross_probe_targets,
        ),
        ("Reports".to_string(), report_count.to_string()),
        ("Active report".to_string(), active_report),
        (
            "Listed reports".to_string(),
            listed_report_count.to_string(),
        ),
        (
            "Stored markers".to_string(),
            stored_marker_count.to_string(),
        ),
        (
            "Stored diagnostics".to_string(),
            stored_diagnostic_count.to_string(),
        ),
        (
            "Report sources".to_string(),
            report_source_entries.len().to_string(),
        ),
        (
            "Report source labels".to_string(),
            layout_drc_report_source_summary_label(&report_source_entries),
        ),
        ("Active report source".to_string(), active_report_source),
        (
            "Current reports".to_string(),
            current_report_count.to_string(),
        ),
        ("Stale reports".to_string(), stale_report_count.to_string()),
        ("Visited".to_string(), visited.to_string()),
        ("Important".to_string(), important.to_string()),
        ("Noted".to_string(), noted.to_string()),
        ("Owned".to_string(), owned.to_string()),
        ("Signed off".to_string(), signed_off.to_string()),
        ("Tagged".to_string(), tagged.to_string()),
        ("Snapshots".to_string(), snapshots.to_string()),
        (
            "Filter".to_string(),
            app.layout_drc_marker_filter.label().to_string(),
        ),
        ("Category".to_string(), category_filter.to_string()),
        ("Directory".to_string(), directory_filter),
        (
            "Sort".to_string(),
            app.layout_drc_marker_sort.label().to_string(),
        ),
        (
            "Columns".to_string(),
            app.layout_browser_columns.label().to_string(),
        ),
        ("Selected".to_string(), selected),
        ("Categories".to_string(), category_entries.len().to_string()),
        ("Rule families".to_string(), rule_count.to_string()),
        (
            "Saved states".to_string(),
            document.marker_states.len().to_string(),
        ),
        (
            "Overlay".to_string(),
            if app.show_drc_overlay {
                "shown"
            } else {
                "hidden"
            }
            .to_string(),
        ),
    ];
    if !report.findings.is_empty() {
        rows.extend(layout_drc_report_diagnostic_rows(&report.findings, 2));
    }
    rows.extend(app.layout_drc_report_history.iter().take(4).map(|entry| {
        (
            format!("Stored report #{}", entry.id),
            layout_drc_marker_report_history_entry_label(
                entry,
                app.layout_revision,
                app.layout_selected_drc_report_id,
            ),
        )
    }));
    let remaining_reports = app.layout_drc_report_history.len().saturating_sub(4);
    if remaining_reports > 0 {
        rows.push(("More reports".to_string(), remaining_reports.to_string()));
    }
    rows.extend(layout_drc_marker_property_rows(app, &report, &entries));
    rows
}
