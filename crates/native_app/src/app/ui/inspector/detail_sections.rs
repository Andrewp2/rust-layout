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
    let mut rows = vec![(
        "Visible now".to_string(),
        current_state.visible_layers.len().to_string(),
    )];
    for slot in LAYOUT_LAYER_SET_SLOTS {
        let value = app
            .layout_layer_sets
            .get(&slot)
            .map(|state| {
                let suffix = if state == &current_state {
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
            })
            .unwrap_or_else(|| "empty".to_string());
        rows.push((format!("Set {slot}"), value));
    }
    rows
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
    let mut rows = vec![(
        "Current".to_string(),
        app.layout_layer_group_filter.path_label(),
    )];
    rows.extend(
        layout_layer_group_summaries(&app.workspace.document)
            .into_iter()
            .map(|summary| {
                (
                    summary.group.path_label(),
                    format_layout_layer_group_summary(&summary),
                )
            }),
    );
    rows
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
    let document = &app.workspace.document;
    let technology = app.active_layout_technology();
    let mut rows = vec![
        (
            "Technology".to_string(),
            display_technology_name(&technology.name),
        ),
        (
            "Database unit".to_string(),
            format!(
                "{} DBU/um, grid {}",
                technology.dbu_per_micron,
                app.format_layout_length(technology.grid as f64)
            ),
        ),
        (
            "Layer definitions".to_string(),
            format!(
                "{} tech, {} document",
                technology.layers.len(),
                document.layers.len()
            ),
        ),
        (
            "DRC rules".to_string(),
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
        ),
        (
            "DRC deck".to_string(),
            compact_button_label(&app.active_layout_drc_deck_label(), 48),
        ),
        (
            "Connectivity".to_string(),
            format!(
                "{}/{} stack link{} enabled",
                technology
                    .connectivity
                    .len()
                    .saturating_sub(app.layout_disabled_connectivity_links.len()),
                technology.connectivity.len(),
                if technology.connectivity.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ),
        ),
    ];
    for (index, connection) in technology.connectivity.iter().take(4).enumerate() {
        let state = if app.layout_disabled_connectivity_links.contains(&index) {
            "disabled"
        } else {
            "enabled"
        };
        rows.push((
            format!("Stack {}", index + 1),
            compact_button_label(
                &format!(
                    "{} / {} / {} ({state})",
                    connection.from, connection.through, connection.to
                ),
                48,
            ),
        ));
    }
    if technology.connectivity.len() > 4 {
        rows.push((
            "Stack links omitted".to_string(),
            (technology.connectivity.len() - 4).to_string(),
        ));
    }
    if let Some(layer) = document.layers.get(&app.active_layer) {
        rows.push((
            "Active layer".to_string(),
            format!("L{} {}", layer.id.0, compact_button_label(&layer.name, 30)),
        ));
        rows.push((
            "Active mapping".to_string(),
            layout_layer_gds_mapping_label(layer),
        ));
        rows.push((
            "Active stack links".to_string(),
            layout_active_layer_connectivity_label(
                layer,
                technology,
                &app.layout_disabled_connectivity_links,
            ),
        ));
        if let Some(technology_layer) = matching_technology_layer(&technology, layer)
            && let (Some(z_base), Some(z_thickness)) =
                (technology_layer.z_base, technology_layer.z_thickness)
        {
            rows.push((
                "Active z".to_string(),
                format!("{z_base:.2}..{:.2}", z_base + z_thickness),
            ));
        }
    }
    rows
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
            ("Parents".to_string(), parent_refs.len().to_string()),
            (
                "Child instances".to_string(),
                cell.instances.len().to_string(),
            ),
            ("Child cells".to_string(), child_cells.len().to_string()),
            ("Collapsed".to_string(), collapsed.to_string()),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Cell id".to_string(), format!("C{}", cell.id.0)),
            ("Name".to_string(), cell.name.clone()),
            ("Role".to_string(), role.to_string()),
            ("Shapes".to_string(), local_shape_count.to_string()),
            ("Bounds".to_string(), bounds),
            ("Parents".to_string(), parent_refs.len().to_string()),
            (
                "Child instances".to_string(),
                cell.instances.len().to_string(),
            ),
            ("Child cells".to_string(), child_cells.len().to_string()),
            ("Hidden".to_string(), hidden.to_string()),
            ("Collapsed".to_string(), collapsed.to_string()),
        ],
    };
    rows.extend(layout_cell_property_rows(app, cell));
    rows
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
    let document = &app.workspace.document;
    let mut rows = Vec::new();
    rows.push(("View top cell".to_string(), layout_view_top_cell_name(app)));
    rows.push((
        "Document top".to_string(),
        document
            .cell(document.top_cell)
            .map(|cell| cell.name.clone())
            .unwrap_or_else(|| format!("cell {}", document.top_cell.0)),
    ));
    rows.push((
        "Placed instances".to_string(),
        document
            .cells
            .values()
            .map(|cell| cell.instances.len())
            .sum::<usize>()
            .to_string(),
    ));
    for cell in document.cells.values().take(4) {
        let suffix = if cell.id == document.top_cell {
            " top"
        } else {
            ""
        };
        rows.push((
            cell.name.clone(),
            format!(
                "{} shapes / {} child instances{}",
                cell.shapes.len(),
                cell.instances.len(),
                suffix
            ),
        ));
    }
    if let Some(occurrence) = &app.selected_layout_occurrence
        && let Some((parent, instance_id)) = document
            .instance_parent_for_path_from_cell(app.layout_view_top_cell, &occurrence.instance_path)
        && let Some(instance) = document.instance(parent, instance_id)
    {
        rows.push((
            "Selected instance".to_string(),
            format!("inst #{} -> cell #{}", instance.id.0, instance.cell.0),
        ));
        rows.push((
            "Instance origin".to_string(),
            format!(
                "{},{}",
                instance.transform.translation.dx, instance.transform.translation.dy
            ),
        ));
    } else if let Some(shape) = app.selected_top_level_layout_shape() {
        rows.push((
            "Selected shape".to_string(),
            format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)),
        ));
    }
    rows
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
            let summary = report.summary(8);
            vec![
                (
                    "Health".to_string(),
                    summary
                        .skipped
                        .clone()
                        .unwrap_or_else(|| summary.health.label().to_string()),
                ),
                (
                    "Components".to_string(),
                    summary.component_count.to_string(),
                ),
                ("Devices".to_string(), summary.device_count.to_string()),
                (
                    "Labeled nets".to_string(),
                    summary.labeled_component_count.to_string(),
                ),
                ("Shorts".to_string(), summary.short_count.to_string()),
                ("Opens".to_string(), summary.open_count.to_string()),
                (
                    "Largest net".to_string(),
                    format!("{} shapes", summary.largest_component_shape_count),
                ),
                (
                    "Saved states".to_string(),
                    document.connectivity_issue_states.len().to_string(),
                ),
            ]
        }
        Err(error) => vec![("Status".to_string(), format!("Failed: {error}"))],
    }
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
                .and_then(|occurrence| report.component_for_occurrence(occurrence))
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
    vec![
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
    ]
    .into_iter()
    .chain((!comparison.missing_layout_devices.is_empty()).then(|| {
        (
            "Missing Device Detail".to_string(),
            compact_spice_signature_list(&comparison.missing_layout_devices),
        )
    }))
    .chain((!comparison.extra_layout_devices.is_empty()).then(|| {
        (
            "Extra Device Detail".to_string(),
            compact_spice_signature_list(&comparison.extra_layout_devices),
        )
    }))
    .collect()
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
    if !report.findings.is_empty() {
        return vec![
            ("Reports".to_string(), report_count.to_string()),
            ("Active report".to_string(), active_report),
            (
                "Rule deck".to_string(),
                format!("{} issue(s)", report.findings.len()),
            ),
        ];
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
                .and_then(|state| state.signoff.as_deref())
                .is_some_and(|signoff| !signoff.trim().is_empty())
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
    let category_entries = layout_drc_marker_category_entries(app);
    let category_filter = app
        .layout_drc_marker_category_filter
        .map(|category| category.label())
        .unwrap_or("All");
    let mut rows = vec![
        (
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ),
        ("Active".to_string(), active.to_string()),
        ("Total".to_string(), report.violations.len().to_string()),
        ("Listed".to_string(), entries.len().to_string()),
        ("Reports".to_string(), report_count.to_string()),
        ("Active report".to_string(), active_report),
        ("Visited".to_string(), visited.to_string()),
        ("Important".to_string(), important.to_string()),
        ("Noted".to_string(), noted.to_string()),
        ("Owned".to_string(), owned.to_string()),
        ("Signed off".to_string(), signed_off.to_string()),
        ("Tagged".to_string(), tagged.to_string()),
        (
            "Filter".to_string(),
            app.layout_drc_marker_filter.label().to_string(),
        ),
        ("Category".to_string(), category_filter.to_string()),
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
    rows.extend(layout_drc_marker_property_rows(app, &report, &entries));
    rows
}
