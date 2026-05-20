#![allow(unused_imports)]
use super::*;
use layout_model::MarkerSignoffRecord;

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

pub(crate) fn marker_state_with_layout_search_replacement(
    state: &MarkerState,
    query: &str,
    replacement: &str,
) -> Option<MarkerState> {
    let mut next = state.clone();
    let mut changed = false;
    replace_marker_state_optional_text(&mut next.note, query, replacement, &mut changed);
    replace_marker_state_optional_text(&mut next.owner, query, replacement, &mut changed);
    replace_marker_state_optional_text(&mut next.signoff, query, replacement, &mut changed);
    replace_marker_state_optional_text(&mut next.signoff_by, query, replacement, &mut changed);
    replace_marker_state_optional_text(&mut next.signoff_note, query, replacement, &mut changed);
    if let Some(signoff_records) =
        marker_signoff_records_with_search_replacement(&next.signoff_records, query, replacement)
    {
        next.signoff_records = signoff_records;
        changed = true;
    }
    if let Some(tags) = marker_tags_with_search_replacement(&next.tags, query, replacement) {
        next.tags = tags;
        changed = true;
    }
    changed.then_some(next)
}

fn replace_marker_state_optional_text(
    value: &mut Option<String>,
    query: &str,
    replacement: &str,
    changed: &mut bool,
) {
    let Some(current) = value.as_ref() else {
        return;
    };
    let Some(replaced) = replace_ascii_case_insensitive(current, query, replacement) else {
        return;
    };
    *value = (!replaced.trim().is_empty()).then_some(replaced);
    *changed = true;
}

fn marker_signoff_records_with_search_replacement(
    signoff_records: &BTreeMap<String, MarkerSignoffRecord>,
    query: &str,
    replacement: &str,
) -> Option<BTreeMap<String, MarkerSignoffRecord>> {
    let mut next = BTreeMap::new();
    for (party, signoff) in signoff_records {
        let replaced_party = replace_ascii_case_insensitive(party, query, replacement)
            .filter(|candidate| !candidate.trim().is_empty())
            .unwrap_or_else(|| party.to_string());
        let replaced_status = replace_ascii_case_insensitive(&signoff.status, query, replacement)
            .unwrap_or_else(|| signoff.status.clone());
        if replaced_status.trim().is_empty() {
            continue;
        }
        let mut replaced = signoff.clone();
        replaced.status = replaced_status;
        let mut field_changed = false;
        replace_marker_state_optional_text(
            &mut replaced.role,
            query,
            replacement,
            &mut field_changed,
        );
        replace_marker_state_optional_text(
            &mut replaced.by,
            query,
            replacement,
            &mut field_changed,
        );
        replace_marker_state_optional_text(
            &mut replaced.note,
            query,
            replacement,
            &mut field_changed,
        );
        replace_marker_state_optional_text(
            &mut replaced.recorded_at,
            query,
            replacement,
            &mut field_changed,
        );
        if next.insert(replaced_party, replaced).is_some() {
            return None;
        }
    }
    (next != *signoff_records).then_some(next)
}

fn marker_tags_with_search_replacement(
    tags: &BTreeMap<String, String>,
    query: &str,
    replacement: &str,
) -> Option<BTreeMap<String, String>> {
    let mut next = BTreeMap::new();
    for (key, value) in tags {
        let replaced_key = replace_ascii_case_insensitive(key, query, replacement)
            .filter(|candidate| !candidate.trim().is_empty())
            .unwrap_or_else(|| key.clone());
        let replaced_value = replace_ascii_case_insensitive(value, query, replacement)
            .unwrap_or_else(|| value.clone());
        if replaced_value.trim().is_empty() {
            continue;
        }
        if next.insert(replaced_key, replaced_value).is_some() {
            return None;
        }
    }
    (next != *tags).then_some(next)
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
    layout_cell_browser_cells_with_filter(app, app.layout_cell_browser_filter, true)
}

pub(crate) fn layout_cell_browser_cells_with_filter<'a>(
    app: &'a GlassworksApp,
    filter: LayoutCellBrowserFilter,
    apply_search: bool,
) -> Vec<&'a Cell> {
    let document = &app.workspace.document;
    let search = apply_search
        .then(|| layout_browser_search_query_lower(app))
        .flatten();
    let mut cells = ordered_layout_cells(document)
        .into_iter()
        .filter(|cell| layout_cell_browser_filter_matches(app, filter, cell))
        .filter(|cell| {
            search.as_deref().is_none_or(|query| {
                if filter == LayoutCellBrowserFilter::Properties {
                    layout_cell_selector_matches_search(document, cell, query)
                        || layout_properties_selector_matches_search(&cell.properties, query)
                } else {
                    layout_cell_matches_search(document, cell, query)
                }
            })
        })
        .collect::<Vec<_>>();
    sort_layout_cell_browser_cells(document, &mut cells, app.layout_cell_browser_sort);
    cells
}

pub(crate) fn layout_cell_browser_filter_matches(
    app: &GlassworksApp,
    filter: LayoutCellBrowserFilter,
    cell: &Cell,
) -> bool {
    let document = &app.workspace.document;
    match filter {
        LayoutCellBrowserFilter::All => true,
        LayoutCellBrowserFilter::Current => cell.id == app.layout_view_top_cell,
        LayoutCellBrowserFilter::Used => {
            cell.id == document.top_cell || !layout_cell_instance_refs(document, cell.id).is_empty()
        }
        LayoutCellBrowserFilter::Unused => {
            cell.id != document.top_cell && layout_cell_instance_refs(document, cell.id).is_empty()
        }
        LayoutCellBrowserFilter::Hidden => app.layout_hidden_cells.contains(&cell.id),
        LayoutCellBrowserFilter::Visible => !app.layout_hidden_cells.contains(&cell.id),
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
        LayoutCellBrowserFilter::Siblings => {
            layout_sibling_cell_ids(document, app.layout_view_top_cell).contains(&cell.id)
        }
        LayoutCellBrowserFilter::Ancestors => {
            layout_ancestor_cell_ids(document, app.layout_view_top_cell).contains(&cell.id)
        }
        LayoutCellBrowserFilter::Descendants => {
            cell.id != app.layout_view_top_cell
                && layout_descendant_cell_ids(document, app.layout_view_top_cell).contains(&cell.id)
        }
    }
}

pub(crate) fn layout_cell_browser_filter_count(
    app: &GlassworksApp,
    filter: LayoutCellBrowserFilter,
) -> usize {
    layout_cell_browser_cells_with_filter(app, filter, true).len()
}

pub(crate) fn layout_cell_browser_filter_button_label(
    app: &GlassworksApp,
    filter: LayoutCellBrowserFilter,
) -> String {
    format!(
        "{} ({})",
        filter.label(),
        layout_cell_browser_filter_count(app, filter)
    )
}

pub(crate) fn layout_cell_browser_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if app.layout_cell_browser_filter == LayoutCellBrowserFilter::All && search.is_empty() {
        return format!("Cell Browser ({listed_count} {row_label})");
    }
    let mut context = Vec::new();
    if app.layout_cell_browser_filter != LayoutCellBrowserFilter::All {
        context.push(app.layout_cell_browser_filter.label().to_string());
    }
    if !search.is_empty() {
        context.push(format!("search {}", compact_button_label(search, 24)));
    }
    format!(
        "Cell Browser - {} ({listed_count} {row_label})",
        context.join(" / ")
    )
}

pub(crate) fn layout_cell_browser_filter_status_label(filter: LayoutCellBrowserFilter) -> String {
    match filter {
        LayoutCellBrowserFilter::All => "cells".to_string(),
        LayoutCellBrowserFilter::Current => "current cells".to_string(),
        LayoutCellBrowserFilter::Used => "used cells".to_string(),
        LayoutCellBrowserFilter::Unused => "unused cells".to_string(),
        LayoutCellBrowserFilter::Hidden => "hidden cells".to_string(),
        LayoutCellBrowserFilter::Visible => "visible cells".to_string(),
        LayoutCellBrowserFilter::Library => "library cells".to_string(),
        LayoutCellBrowserFilter::Properties => "property-bearing cells".to_string(),
        LayoutCellBrowserFilter::Empty => "empty cells".to_string(),
        LayoutCellBrowserFilter::Leaves => "leaf cells".to_string(),
        LayoutCellBrowserFilter::Branches => "branch cells".to_string(),
        LayoutCellBrowserFilter::Parents => "parent cells".to_string(),
        LayoutCellBrowserFilter::Children => "child cells".to_string(),
        LayoutCellBrowserFilter::Siblings => "sibling cells".to_string(),
        LayoutCellBrowserFilter::Ancestors => "ancestor cells".to_string(),
        LayoutCellBrowserFilter::Descendants => "descendant cells".to_string(),
    }
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
                LayoutCellBrowserSort::Depth => layout_cell_min_hierarchy_depth(document, left.id)
                    .unwrap_or(usize::MAX)
                    .cmp(
                        &layout_cell_min_hierarchy_depth(document, right.id).unwrap_or(usize::MAX),
                    ),
                LayoutCellBrowserSort::ShapeCount => right.shapes.len().cmp(&left.shapes.len()),
                LayoutCellBrowserSort::InstanceCount => {
                    right.instances.len().cmp(&left.instances.len())
                }
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
}

pub(crate) fn layout_cell_min_hierarchy_depth(
    document: &Document,
    target_cell: CellId,
) -> Option<usize> {
    if target_cell == document.top_cell {
        return Some(0);
    }
    let mut visited = BTreeSet::new();
    let mut queue = vec![(document.top_cell, 0usize)];
    let mut index = 0usize;
    while index < queue.len() {
        let (cell_id, depth) = queue[index];
        index += 1;
        if !visited.insert(cell_id) {
            continue;
        }
        let Some(cell) = document.cell(cell_id) else {
            continue;
        };
        for instance in cell.instances.values() {
            if instance.cell == target_cell {
                return Some(depth + 1);
            }
            queue.push((instance.cell, depth + 1));
        }
    }
    None
}

pub(crate) fn layout_cell_hierarchy_depth_label(document: &Document, cell_id: CellId) -> String {
    layout_cell_min_hierarchy_depth(document, cell_id)
        .map(|depth| depth.to_string())
        .unwrap_or_else(|| "unreachable".to_string())
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
    if layout_cell_selector_matches_search(document, cell, query_lower) {
        return true;
    }
    let refs = layout_cell_instance_refs(document, cell.id);
    let depth_label = layout_cell_hierarchy_depth_label(document, cell.id);
    let mut fields = vec![
        cell.name.clone(),
        format!("C{}", cell.id.0),
        format!("cell {}", cell.id.0),
        format!("hierarchy depth {depth_label}"),
        format!("depth {depth_label}"),
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
            push_layout_search_field(&mut fields, format!("parent {}", parent_cell.name));
        }
    }
    for ancestor in layout_ancestor_cell_ids(document, cell.id) {
        if let Some(ancestor_cell) = document.cell(ancestor) {
            push_layout_search_field(&mut fields, ancestor_cell.name.clone());
            push_layout_search_field(&mut fields, format!("ancestor C{}", ancestor.0));
            push_layout_search_field(&mut fields, format!("ancestor {}", ancestor_cell.name));
        }
    }
    for sibling in layout_sibling_cell_ids(document, cell.id) {
        if let Some(sibling_cell) = document.cell(sibling) {
            push_layout_search_field(&mut fields, sibling_cell.name.clone());
            push_layout_search_field(&mut fields, format!("sibling C{}", sibling.0));
            push_layout_search_field(&mut fields, format!("sibling {}", sibling_cell.name));
        }
    }
    for instance in cell.instances.values() {
        if let Some(child) = document.cell(instance.cell) {
            push_layout_search_field(&mut fields, child.name.clone());
            push_layout_search_field(&mut fields, format!("child C{}", child.id.0));
            push_layout_search_field(&mut fields, format!("child {}", child.name));
        }
    }
    for descendant in layout_descendant_cell_ids(document, cell.id) {
        if descendant == cell.id {
            continue;
        }
        if let Some(descendant_cell) = document.cell(descendant) {
            push_layout_search_field(&mut fields, descendant_cell.name.clone());
            push_layout_search_field(&mut fields, format!("descendant C{}", descendant.0));
            push_layout_search_field(&mut fields, format!("descendant {}", descendant_cell.name));
        }
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_cell_selector_matches_search(
    document: &Document,
    cell: &Cell,
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
    let parent_refs = layout_cell_instance_refs(document, cell.id);
    let parent_ids = parent_refs
        .iter()
        .map(|(parent, _)| *parent)
        .collect::<BTreeSet<_>>();
    let ancestor_ids = layout_ancestor_cell_ids(document, cell.id);
    let sibling_ids = layout_sibling_cell_ids(document, cell.id);
    let child_ids = cell
        .instances
        .values()
        .map(|instance| instance.cell)
        .collect::<BTreeSet<_>>();
    let mut descendant_ids = layout_descendant_cell_ids(document, cell.id);
    descendant_ids.remove(&cell.id);
    let local_shape_count = layout_cell_local_shape_count_for_search(document, cell);
    let bounds = layout_cell_local_bounds_for_search(document, cell)
        .map(rect_summary)
        .unwrap_or_else(|| "None".to_string());
    let usage_role = if cell.id == document.top_cell {
        "top"
    } else if parent_refs.is_empty() {
        "unused"
    } else {
        "used"
    };
    let tree_role = if cell.instances.is_empty() {
        "leaf"
    } else {
        "branch"
    };
    let depth = layout_cell_hierarchy_depth_label(document, cell.id);
    let mut fields = vec![
        ("id", cell.id.0.to_string()),
        ("cell", format!("C{} {}", cell.id.0, cell.name)),
        ("cell_id", cell.id.0.to_string()),
        ("name", cell.name.clone()),
        ("role", usage_role.to_string()),
        ("usage", usage_role.to_string()),
        ("tree_role", tree_role.to_string()),
        ("type", tree_role.to_string()),
        ("depth", depth.clone()),
        ("hierarchy_depth", depth),
        ("shapes", local_shape_count.to_string()),
        ("shape_count", local_shape_count.to_string()),
        ("instances", cell.instances.len().to_string()),
        ("instance_count", cell.instances.len().to_string()),
        ("parents", parent_refs.len().to_string()),
        ("parent_count", parent_refs.len().to_string()),
        ("ancestors", ancestor_ids.len().to_string()),
        ("ancestor_count", ancestor_ids.len().to_string()),
        ("siblings", sibling_ids.len().to_string()),
        ("sibling_count", sibling_ids.len().to_string()),
        ("child_instances", cell.instances.len().to_string()),
        ("child_cells", child_ids.len().to_string()),
        ("child_cell_count", child_ids.len().to_string()),
        ("descendants", descendant_ids.len().to_string()),
        ("descendant_count", descendant_ids.len().to_string()),
        ("bounds", bounds),
        (
            "library",
            layout_cell_is_library_generated(cell).to_string(),
        ),
    ];
    push_layout_cell_selector_fields(&mut fields, "parent", document, &parent_ids);
    push_layout_cell_selector_fields(&mut fields, "parent_cell", document, &parent_ids);
    push_layout_cell_selector_id_fields(&mut fields, "parent_cell_id", &parent_ids);
    push_layout_cell_selector_fields(&mut fields, "ancestor", document, &ancestor_ids);
    push_layout_cell_selector_fields(&mut fields, "ancestor_cell", document, &ancestor_ids);
    push_layout_cell_selector_id_fields(&mut fields, "ancestor_cell_id", &ancestor_ids);
    push_layout_cell_selector_fields(&mut fields, "sibling", document, &sibling_ids);
    push_layout_cell_selector_fields(&mut fields, "sibling_cell", document, &sibling_ids);
    push_layout_cell_selector_id_fields(&mut fields, "sibling_cell_id", &sibling_ids);
    push_layout_cell_selector_fields(&mut fields, "child", document, &child_ids);
    push_layout_cell_selector_fields(&mut fields, "child_cell", document, &child_ids);
    push_layout_cell_selector_id_fields(&mut fields, "child_cell_id", &child_ids);
    push_layout_cell_selector_fields(&mut fields, "descendant", document, &descendant_ids);
    push_layout_cell_selector_fields(&mut fields, "descendant_cell", document, &descendant_ids);
    push_layout_cell_selector_id_fields(&mut fields, "descendant_cell_id", &descendant_ids);
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
        || layout_properties_selector_matches_search(&cell.properties, query_lower)
}

pub(crate) fn push_layout_cell_selector_fields(
    fields: &mut Vec<(&'static str, String)>,
    key: &'static str,
    document: &Document,
    cell_ids: &BTreeSet<CellId>,
) {
    for cell_id in cell_ids {
        let value = document
            .cell(*cell_id)
            .map(|cell| format!("C{} {}", cell.id.0, cell.name))
            .unwrap_or_else(|| format!("C{}", cell_id.0));
        fields.push((key, value));
    }
}

pub(crate) fn push_layout_cell_selector_id_fields(
    fields: &mut Vec<(&'static str, String)>,
    key: &'static str,
    cell_ids: &BTreeSet<CellId>,
) {
    fields.extend(cell_ids.iter().map(|cell_id| (key, cell_id.0.to_string())));
}

pub(crate) fn layout_cell_local_shape_count_for_search(document: &Document, cell: &Cell) -> usize {
    cell.shapes.len()
        + if cell.id == document.top_cell {
            document.shapes.len()
        } else {
            0
        }
}

pub(crate) fn layout_cell_local_bounds_for_search(
    document: &Document,
    cell: &Cell,
) -> Option<Rect> {
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
