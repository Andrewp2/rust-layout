#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_view_top_cell_buttons(
    document: &Document,
    current_top_cell: CellId,
) -> Vec<ViewControlButton> {
    ordered_layout_cells(document)
        .into_iter()
        .take(3)
        .map(|cell| {
            ViewControlButton::new(
                format!("glassworks.viewctl.layout.top_cell.{}", cell.id.0),
                compact_cell_button_label(cell, document.top_cell),
                current_top_cell == cell.id,
            )
        })
        .collect()
}

pub(crate) fn ordered_layout_cells(document: &Document) -> Vec<&Cell> {
    let mut cells = document.cells.values().collect::<Vec<_>>();
    cells.sort_by(|left, right| {
        (left.id != document.top_cell)
            .cmp(&(right.id != document.top_cell))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
    cells
}

pub(crate) fn layout_browser_search_has_command_modifier(modifiers: operad::KeyModifiers) -> bool {
    modifiers.ctrl || modifiers.meta || modifiers.alt
}

pub(crate) fn layout_browser_search_accepts_character(character: char) -> bool {
    !character.is_control()
}

pub(crate) fn push_layout_browser_search_character(query: &mut String, character: char) {
    query.push(character);
    truncate_layout_browser_search(query);
}

pub(crate) fn sanitize_layout_browser_search(query: String) -> String {
    let mut query = query
        .chars()
        .filter(|character| layout_browser_search_accepts_character(*character))
        .collect::<String>();
    truncate_layout_browser_search(&mut query);
    query
}

pub(crate) fn truncate_layout_browser_search(query: &mut String) {
    if let Some((index, _)) = query.char_indices().nth(MAX_LAYOUT_BROWSER_SEARCH_CHARS) {
        query.truncate(index);
    }
}

pub(crate) fn layout_browser_search_status(query: &str) -> String {
    let query = query.trim();
    if query.is_empty() {
        "Browser search empty".to_string()
    } else {
        format!("Browser search {}", compact_button_label(query, 32))
    }
}

pub(crate) fn layout_browser_replace_status(replacement: &str) -> String {
    let replacement = replacement.trim();
    if replacement.is_empty() {
        "Browser replace empty".to_string()
    } else {
        format!("Browser replace {}", compact_button_label(replacement, 32))
    }
}

pub(crate) fn layout_browser_search_query_lower(app: &GlassworksApp) -> Option<String> {
    let query = app.layout_browser_search.trim();
    (!query.is_empty()).then(|| query.to_ascii_lowercase())
}

pub(crate) fn layout_search_matches_value(query_lower: &str, value: &str) -> bool {
    value.to_ascii_lowercase().contains(query_lower)
}

pub(crate) fn shape_with_layout_search_replacement(
    shape: &Shape,
    query: &str,
    replacement: &str,
) -> Option<Shape> {
    let mut next = shape.clone();
    let mut changed = false;
    if let Some(name) = next.name.as_mut()
        && let Some(replaced) = replace_ascii_case_insensitive(name, query, replacement)
    {
        *name = replaced;
        changed = true;
    }
    if let ShapeKind::Label { text, .. } = &mut next.kind
        && let Some(replaced) = replace_ascii_case_insensitive(text, query, replacement)
    {
        *text = replaced;
        changed = true;
    }
    if let ShapeKind::Measurement { label, .. } = &mut next.kind
        && let Some(replaced) = replace_ascii_case_insensitive(label, query, replacement)
    {
        *label = replaced;
        changed = true;
    }
    if let Some(properties) =
        layout_properties_with_search_replacement(&next.properties, query, replacement)
    {
        next.properties = properties;
        changed = true;
    }
    changed.then_some(next)
}

pub(crate) fn instance_with_layout_search_replacement(
    instance: &CellInstance,
    query: &str,
    replacement: &str,
) -> Option<CellInstance> {
    let mut next = instance.clone();
    let mut changed = false;
    if let Some(name) = next.name.as_mut()
        && let Some(replaced) = replace_ascii_case_insensitive(name, query, replacement)
    {
        *name = replaced;
        changed = true;
    }
    if let Some(properties) =
        layout_properties_with_search_replacement(&next.properties, query, replacement)
    {
        next.properties = properties;
        changed = true;
    }
    changed.then_some(next)
}

pub(crate) fn layout_properties_with_search_replacement(
    properties: &BTreeMap<String, String>,
    query: &str,
    replacement: &str,
) -> Option<BTreeMap<String, String>> {
    let mut next = BTreeMap::new();
    for (key, value) in properties {
        let replaced_key = replace_ascii_case_insensitive(key, query, replacement)
            .filter(|candidate| !candidate.trim().is_empty())
            .unwrap_or_else(|| key.clone());
        let replaced_value = replace_ascii_case_insensitive(value, query, replacement)
            .unwrap_or_else(|| value.clone());
        if next.insert(replaced_key, replaced_value).is_some() {
            return None;
        }
    }
    (next != *properties).then_some(next)
}

pub(crate) fn replace_ascii_case_insensitive(
    value: &str,
    query: &str,
    replacement: &str,
) -> Option<String> {
    if query.is_empty() {
        return None;
    }
    let lower_value = value.to_ascii_lowercase();
    let lower_query = query.to_ascii_lowercase();
    let mut start = 0;
    let mut output = String::with_capacity(value.len());
    let mut changed = false;
    while let Some(relative_index) = lower_value[start..].find(&lower_query) {
        let match_start = start + relative_index;
        let match_end = match_start + lower_query.len();
        output.push_str(&value[start..match_start]);
        output.push_str(replacement);
        start = match_end;
        changed = true;
    }
    if !changed {
        return None;
    }
    output.push_str(&value[start..]);
    Some(output)
}

pub(crate) fn layout_search_matches_any(query_lower: &str, fields: &[String]) -> bool {
    fields
        .iter()
        .any(|field| layout_search_matches_value(query_lower, field))
}

pub(crate) fn push_layout_search_field(fields: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.trim().is_empty() {
        fields.push(value);
    }
}

pub(crate) fn layout_cell_browser_cells(app: &GlassworksApp) -> Vec<&Cell> {
    let document = &app.workspace.document;
    let search = layout_browser_search_query_lower(app);
    let mut cells = ordered_layout_cells(document)
        .into_iter()
        .filter(|cell| match app.layout_cell_browser_filter {
            LayoutCellBrowserFilter::All => true,
            LayoutCellBrowserFilter::Current => cell.id == app.layout_view_top_cell,
            LayoutCellBrowserFilter::Used => {
                cell.id == document.top_cell
                    || !layout_cell_instance_refs(document, cell.id).is_empty()
            }
            LayoutCellBrowserFilter::Unused => {
                cell.id != document.top_cell
                    && layout_cell_instance_refs(document, cell.id).is_empty()
            }
            LayoutCellBrowserFilter::Library => layout_cell_is_library_generated(cell),
            LayoutCellBrowserFilter::Properties => !cell.properties.is_empty(),
            LayoutCellBrowserFilter::Empty => cell.shapes.is_empty() && cell.instances.is_empty(),
            LayoutCellBrowserFilter::Leaves => cell.instances.is_empty(),
            LayoutCellBrowserFilter::Branches => !cell.instances.is_empty(),
            LayoutCellBrowserFilter::Parents => {
                cell.id != app.layout_view_top_cell
                    && layout_cell_instance_refs(document, app.layout_view_top_cell)
                        .iter()
                        .any(|(parent, _)| *parent == cell.id)
            }
            LayoutCellBrowserFilter::Children => {
                cell.id != app.layout_view_top_cell
                    && document
                        .cell(app.layout_view_top_cell)
                        .is_some_and(|current| {
                            current
                                .instances
                                .values()
                                .any(|instance| instance.cell == cell.id)
                        })
            }
        })
        .filter(|cell| {
            search.as_deref().is_none_or(|query| {
                if app.layout_cell_browser_filter == LayoutCellBrowserFilter::Properties {
                    layout_properties_selector_matches_search(&cell.properties, query)
                } else {
                    layout_cell_matches_search(document, cell, query)
                }
            })
        })
        .collect::<Vec<_>>();
    sort_layout_cell_browser_cells(document, &mut cells, app.layout_cell_browser_sort);
    cells
}

pub(crate) fn sort_layout_cell_browser_cells(
    document: &Document,
    cells: &mut [&Cell],
    sort: LayoutCellBrowserSort,
) {
    cells.sort_by(|left, right| {
        (left.id != document.top_cell)
            .cmp(&(right.id != document.top_cell))
            .then_with(|| match sort {
                LayoutCellBrowserSort::Name => {
                    left.name.to_lowercase().cmp(&right.name.to_lowercase())
                }
                LayoutCellBrowserSort::Id => left.id.cmp(&right.id),
                LayoutCellBrowserSort::ShapeCount => right.shapes.len().cmp(&left.shapes.len()),
                LayoutCellBrowserSort::InstanceCount => {
                    right.instances.len().cmp(&left.instances.len())
                }
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub(crate) fn layout_cell_is_library_generated(cell: &Cell) -> bool {
    cell.properties
        .get("library.kind")
        .is_some_and(|kind| kind == "macro")
        || cell.properties.contains_key("library.macro")
}

pub(crate) fn layout_via_array_parameters_from_cell(cell: &Cell) -> Option<(u8, u8, Coord, Coord)> {
    (cell.properties.get("library.macro").map(String::as_str) == Some("via_array")).then_some(())?;
    let columns = cell
        .properties
        .get("library.columns")?
        .parse::<u8>()
        .ok()?
        .clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
    let rows = cell
        .properties
        .get("library.rows")?
        .parse::<u8>()
        .ok()?
        .clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
    let size_grids = cell
        .properties
        .get("library.size_grids")?
        .parse::<Coord>()
        .ok()?
        .clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
        );
    let pitch_grids = clamp_layout_library_via_array_pitch_grids(
        size_grids,
        cell.properties
            .get("library.pitch_grids")?
            .parse::<Coord>()
            .ok()?,
    );
    Some((columns, rows, size_grids, pitch_grids))
}

pub(crate) fn layout_cell_matches_search(
    document: &Document,
    cell: &Cell,
    query_lower: &str,
) -> bool {
    let refs = layout_cell_instance_refs(document, cell.id);
    let mut fields = vec![
        cell.name.clone(),
        format!("C{}", cell.id.0),
        format!("cell {}", cell.id.0),
        format!("{} shapes", cell.shapes.len()),
        format!("{} instances", cell.instances.len()),
    ];
    if cell.id == document.top_cell {
        push_layout_search_field(&mut fields, "top");
    }
    if refs.is_empty() && cell.id != document.top_cell {
        push_layout_search_field(&mut fields, "unused");
    } else {
        push_layout_search_field(&mut fields, "used");
    }
    if cell.instances.is_empty() {
        push_layout_search_field(&mut fields, "leaf");
    } else {
        push_layout_search_field(&mut fields, "branch");
    }
    if layout_cell_is_library_generated(cell) {
        push_layout_search_field(&mut fields, "library");
    }
    for (key, value) in &cell.properties {
        push_layout_search_field(&mut fields, key.clone());
        push_layout_search_field(&mut fields, value.clone());
    }
    for (parent, _) in refs {
        if let Some(parent_cell) = document.cell(parent) {
            push_layout_search_field(&mut fields, parent_cell.name.clone());
            push_layout_search_field(&mut fields, format!("parent C{}", parent.0));
        }
    }
    for instance in cell.instances.values() {
        if let Some(child) = document.cell(instance.cell) {
            push_layout_search_field(&mut fields, child.name.clone());
        }
    }
    layout_search_matches_any(query_lower, &fields)
}
