#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_drc_marker_entries(app: &GlassworksApp) -> Vec<LayoutDrcMarkerEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    let search = layout_browser_search_query_lower(app);
    let mut violations = report
        .violations
        .iter()
        .filter(|violation| {
            let key = violation.stable_key();
            let state = document.marker_states.get(&key);
            let category = LayoutDrcMarkerCategory::from_rule(&violation.rule);
            layout_drc_marker_filter_matches(state, app.layout_drc_marker_filter)
                && app
                    .layout_drc_marker_category_filter
                    .is_none_or(|filter| filter == category)
                && search
                    .as_deref()
                    .is_none_or(|query| layout_drc_marker_matches_search(violation, state, query))
        })
        .collect::<Vec<_>>();
    violations.sort_by(|left, right| match app.layout_drc_marker_sort {
        LayoutDrcMarkerSort::Id => left.id.cmp(&right.id),
        LayoutDrcMarkerSort::Rule => left
            .rule
            .cmp(&right.rule)
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::State => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            let left_state = layout_drc_marker_state_label(document.marker_states.get(&left_key));
            let right_state = layout_drc_marker_state_label(document.marker_states.get(&right_key));
            left_state
                .cmp(&right_state)
                .then_with(|| left.id.cmp(&right.id))
        }
    });
    violations
        .into_iter()
        .take(MAX_LAYOUT_DRC_MARKER_ROWS)
        .map(|violation| {
            let key = violation.stable_key();
            let state_label = layout_drc_marker_state_label(document.marker_states.get(&key));
            LayoutDrcMarkerEntry {
                id: violation.id,
                key,
                label: compact_button_label(
                    &format!("#{} {} {}", violation.id, violation.rule, state_label),
                    30,
                ),
                state_label,
            }
        })
        .collect()
}

pub(crate) fn layout_drc_marker_category_entries(
    app: &GlassworksApp,
) -> Vec<LayoutDrcMarkerCategoryEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    if !report.findings.is_empty() {
        return Vec::new();
    }

    let search = layout_browser_search_query_lower(app);
    let mut categories: BTreeMap<LayoutDrcMarkerCategory, (usize, usize)> = BTreeMap::new();
    for violation in &report.violations {
        let key = violation.stable_key();
        let state = document.marker_states.get(&key);
        if !layout_drc_marker_filter_matches(state, app.layout_drc_marker_filter)
            || !search
                .as_deref()
                .is_none_or(|query| layout_drc_marker_matches_search(violation, state, query))
        {
            continue;
        }
        let entry = categories
            .entry(LayoutDrcMarkerCategory::from_rule(&violation.rule))
            .or_insert((0, 0));
        entry.0 += 1;
        if drc_violation_is_active(document, violation) {
            entry.1 += 1;
        }
    }

    categories
        .into_iter()
        .map(|(category, (total, active))| LayoutDrcMarkerCategoryEntry {
            category,
            total,
            active,
        })
        .collect()
}

pub(crate) fn layout_drc_marker_directory_entries(
    app: &GlassworksApp,
) -> Vec<LayoutDrcMarkerDirectoryEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    if !report.findings.is_empty() {
        return Vec::new();
    }

    let search = layout_browser_search_query_lower(app);
    let mut directories: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for violation in &report.violations {
        let key = violation.stable_key();
        let state = document.marker_states.get(&key);
        let category = LayoutDrcMarkerCategory::from_rule(&violation.rule);
        if !layout_drc_marker_filter_matches(state, app.layout_drc_marker_filter)
            || !app
                .layout_drc_marker_category_filter
                .is_none_or(|filter| filter == category)
            || !search
                .as_deref()
                .is_none_or(|query| layout_drc_marker_matches_search(violation, state, query))
        {
            continue;
        }
        let entry = directories
            .entry(layout_drc_marker_directory_path(violation))
            .or_insert((0, 0));
        entry.0 += 1;
        if drc_violation_is_active(document, violation) {
            entry.1 += 1;
        }
    }

    directories
        .into_iter()
        .map(|(path, (total, active))| LayoutDrcMarkerDirectoryEntry {
            path,
            total,
            active,
        })
        .collect()
}

pub(crate) fn layout_drc_marker_directory_path(violation: &DrcViolation) -> String {
    let category = LayoutDrcMarkerCategory::from_rule(&violation.rule).label();
    let mut parts = vec![category.to_string()];
    for part in violation
        .rule
        .split(['.', '/', ':'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        parts.push(part.to_string());
    }
    if parts.len() == 1 {
        parts.push(violation.rule.clone());
    }
    parts.join(" / ")
}

pub(crate) fn layout_drc_marker_directory_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_drc_marker_directory_entries(app);
    if entries.is_empty() {
        return vec![("Directories".to_string(), "None".to_string())];
    }
    let mut rows = vec![("Directories".to_string(), entries.len().to_string())];
    rows.extend(entries.iter().take(6).map(|entry| {
        (
            compact_button_label(&entry.path, 32),
            format!("{} active / {} total", entry.active, entry.total),
        )
    }));
    let remaining = entries.len().saturating_sub(6);
    if remaining > 0 {
        rows.push(("More".to_string(), remaining.to_string()));
    }
    rows
}

pub(crate) fn layout_drc_marker_matches_search(
    violation: &DrcViolation,
    state: Option<&MarkerState>,
    query_lower: &str,
) -> bool {
    let mut fields = vec![
        format!("#{}", violation.id),
        format!("marker {}", violation.id),
        violation.rule.clone(),
        LayoutDrcMarkerCategory::from_rule(&violation.rule)
            .label()
            .to_string(),
        layout_drc_marker_state_label(state).to_string(),
        violation.stable_key(),
        format!("{} shapes", violation.shape_ids.len()),
        rect_summary(violation.bounds),
        format!("required {}", violation.required),
        format!("actual {:.1}", violation.actual),
    ];
    for shape_id in &violation.shape_ids {
        push_layout_search_field(&mut fields, format!("shape {}", shape_id.0));
        push_layout_search_field(&mut fields, format!("#{}", shape_id.0));
    }
    if let Some(note) = state.and_then(|state| state.note.as_deref()) {
        push_layout_search_field(&mut fields, note);
    }
    if let Some(owner) = state.and_then(|state| state.owner.as_deref()) {
        push_layout_search_field(&mut fields, owner);
    }
    if let Some(signoff) = state.and_then(|state| state.signoff.as_deref()) {
        push_layout_search_field(&mut fields, signoff);
    }
    if let Some(state) = state {
        for (tag_key, tag_value) in &state.tags {
            push_layout_search_field(&mut fields, tag_key);
            push_layout_search_field(&mut fields, tag_value);
            push_layout_search_field(&mut fields, format!("{tag_key}={tag_value}"));
        }
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_drc_marker_filter_matches(
    state: Option<&MarkerState>,
    filter: LayoutDrcMarkerFilter,
) -> bool {
    match filter {
        LayoutDrcMarkerFilter::Active => state.is_none_or(|state| !state.hidden && !state.waived),
        LayoutDrcMarkerFilter::All => true,
        LayoutDrcMarkerFilter::Hidden => state.is_some_and(|state| state.hidden),
        LayoutDrcMarkerFilter::Waived => state.is_some_and(|state| state.waived),
        LayoutDrcMarkerFilter::Visited => state.is_some_and(|state| state.visited),
        LayoutDrcMarkerFilter::Important => state.is_some_and(|state| state.important),
        LayoutDrcMarkerFilter::Noted => state.is_some_and(|state| {
            state
                .note
                .as_deref()
                .is_some_and(|note| !note.trim().is_empty())
        }),
        LayoutDrcMarkerFilter::Owned => state.is_some_and(|state| {
            state
                .owner
                .as_deref()
                .is_some_and(|owner| !owner.trim().is_empty())
        }),
        LayoutDrcMarkerFilter::SignedOff => state.is_some_and(|state| {
            state
                .signoff
                .as_deref()
                .is_some_and(|signoff| !signoff.trim().is_empty())
        }),
        LayoutDrcMarkerFilter::Tagged => state.is_some_and(|state| !state.tags.is_empty()),
        LayoutDrcMarkerFilter::Snapshots => state.is_some_and(layout_marker_state_has_snapshot),
    }
}

pub(crate) fn layout_marker_state_has_snapshot(state: &MarkerState) -> bool {
    state
        .tags
        .get("screenshot")
        .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn layout_drc_marker_note_preset(slug: &str) -> Option<(&'static str, &'static str)> {
    LAYOUT_DRC_MARKER_NOTE_PRESETS
        .iter()
        .find(|(preset_slug, _, _)| *preset_slug == slug)
        .map(|(_, label, note)| (*label, *note))
}

pub(crate) fn layout_drc_marker_owner_preset(slug: &str) -> Option<(&'static str, &'static str)> {
    LAYOUT_DRC_MARKER_OWNER_PRESETS
        .iter()
        .find(|(preset_slug, _, _)| *preset_slug == slug)
        .map(|(_, label, owner)| (*label, *owner))
}

pub(crate) fn layout_drc_marker_signoff_preset(slug: &str) -> Option<(&'static str, &'static str)> {
    LAYOUT_DRC_MARKER_SIGNOFF_PRESETS
        .iter()
        .find(|(preset_slug, _, _)| *preset_slug == slug)
        .map(|(_, label, signoff)| (*label, *signoff))
}

pub(crate) fn layout_drc_marker_tag_preset(
    slug: &str,
) -> Option<(&'static str, &'static str, &'static str)> {
    LAYOUT_DRC_MARKER_TAG_PRESETS
        .iter()
        .find(|(preset_slug, _, _, _)| *preset_slug == slug)
        .map(|(_, label, tag_key, tag_value)| (*label, *tag_key, *tag_value))
}

pub(crate) fn layout_drc_marker_state_label(state: Option<&MarkerState>) -> String {
    let Some(state) = state else {
        return "active".to_string();
    };
    let mut tags = Vec::new();
    if state.hidden {
        tags.push("hidden");
    }
    if state.waived {
        tags.push("waived");
    }
    if state.visited {
        tags.push("visited");
    }
    if state.important {
        tags.push("important");
    }
    if state
        .owner
        .as_deref()
        .is_some_and(|owner| !owner.trim().is_empty())
    {
        tags.push("owned");
    }
    if state
        .signoff
        .as_deref()
        .is_some_and(|signoff| !signoff.trim().is_empty())
    {
        tags.push("signed");
    }
    if !state.tags.is_empty() {
        tags.push("tagged");
    }
    if tags.is_empty() {
        "active".to_string()
    } else {
        tags.join("+")
    }
}

pub(crate) fn layout_marker_tags_label(state: Option<&MarkerState>) -> String {
    let Some(state) = state else {
        return "None".to_string();
    };
    if state.tags.is_empty() {
        return "None".to_string();
    }
    let mut label = state
        .tags
        .iter()
        .take(3)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = state.tags.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 48)
}

pub(crate) fn layout_marker_snapshot_label(state: Option<&MarkerState>) -> String {
    let Some(state) = state else {
        return "None".to_string();
    };
    let Some(path) = state
        .tags
        .get("screenshot")
        .filter(|path| !path.trim().is_empty())
    else {
        return "None".to_string();
    };
    match state
        .tags
        .get("screenshot_size")
        .filter(|size| !size.trim().is_empty())
    {
        Some(size) => {
            compact_button_label(&format!("{size} {}", compact_button_label(path, 36)), 52)
        }
        None => compact_button_label(path, 52),
    }
}

pub(crate) fn layout_marker_snapshot_bounds_label(state: Option<&MarkerState>) -> String {
    state
        .and_then(|state| state.tags.get("screenshot_bounds"))
        .filter(|bounds| !bounds.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "None".to_string())
}

pub(crate) fn selected_layout_drc_marker_violation(app: &GlassworksApp) -> Option<DrcViolation> {
    let selected_key = app.layout_selected_drc_marker_key.as_ref()?;
    app.drc_report()?
        .violations
        .into_iter()
        .find(|violation| violation.stable_key() == *selected_key)
}

pub(crate) fn layout_drc_marker_info_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let Some(report) = app.drc_report() else {
        return vec![("Status".to_string(), "Not run".to_string())];
    };
    if !report.findings.is_empty() {
        return vec![(
            "Rule deck".to_string(),
            format!("{} issue(s)", report.findings.len()),
        )];
    }
    let entries = layout_drc_marker_entries(app);
    let selected_key = app
        .layout_selected_drc_marker_key
        .as_ref()
        .filter(|selected| entries.iter().any(|entry| &entry.key == *selected))
        .cloned()
        .or_else(|| entries.first().map(|entry| entry.key.clone()));
    let Some(selected_key) = selected_key else {
        return vec![("Marker".to_string(), "None".to_string())];
    };
    let Some(violation) = report
        .violations
        .iter()
        .find(|violation| violation.stable_key() == selected_key)
    else {
        return vec![("Marker".to_string(), "Missing".to_string())];
    };
    let state = app.workspace.document.marker_states.get(&selected_key);
    vec![
        ("Marker id".to_string(), format!("#{}", violation.id)),
        (
            "Directory".to_string(),
            compact_button_label(&layout_drc_marker_directory_path(violation), 52),
        ),
        ("Rule".to_string(), violation.rule.clone()),
        (
            "Message".to_string(),
            compact_button_label(&violation.message, 64),
        ),
        (
            "Stable key".to_string(),
            compact_button_label(&selected_key, 64),
        ),
        ("State".to_string(), layout_drc_marker_state_label(state)),
        ("Bounds".to_string(), rect_summary(violation.bounds)),
        ("Shapes".to_string(), violation.shape_ids.len().to_string()),
        (
            "Occurrences".to_string(),
            violation.occurrence_ids.len().to_string(),
        ),
        (
            "Required".to_string(),
            app.format_layout_length(violation.required as f64),
        ),
        ("Actual".to_string(), format!("{:.1}", violation.actual)),
        (
            "Note".to_string(),
            state
                .and_then(|state| state.note.as_deref())
                .filter(|note| !note.trim().is_empty())
                .unwrap_or("None")
                .to_string(),
        ),
        (
            "Owner".to_string(),
            state
                .and_then(|state| state.owner.as_deref())
                .filter(|owner| !owner.trim().is_empty())
                .unwrap_or("None")
                .to_string(),
        ),
        (
            "Signoff".to_string(),
            state
                .and_then(|state| state.signoff.as_deref())
                .filter(|signoff| !signoff.trim().is_empty())
                .unwrap_or("None")
                .to_string(),
        ),
        ("Tags".to_string(), layout_marker_tags_label(state)),
        ("Snapshot".to_string(), layout_marker_snapshot_label(state)),
        (
            "Snapshot bounds".to_string(),
            layout_marker_snapshot_bounds_label(state),
        ),
    ]
}

pub(crate) fn layout_drc_marker_property_rows(
    app: &GlassworksApp,
    report: &DrcReportCacheValue,
    entries: &[LayoutDrcMarkerEntry],
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let selected = app
        .layout_selected_drc_marker_key
        .as_ref()
        .filter(|selected| entries.iter().any(|entry| &entry.key == *selected))
        .cloned()
        .or_else(|| entries.first().map(|entry| entry.key.clone()));
    let Some(selected_key) = selected else {
        return Vec::new();
    };
    let Some(violation) = report
        .violations
        .iter()
        .find(|violation| violation.stable_key() == selected_key)
    else {
        return Vec::new();
    };
    let state = document.marker_states.get(&selected_key);
    let state_label = layout_drc_marker_state_label(state).to_string();
    let category_label = LayoutDrcMarkerCategory::from_rule(&violation.rule)
        .label()
        .to_string();
    let note_label = state
        .and_then(|state| state.note.as_deref())
        .filter(|note| !note.trim().is_empty())
        .unwrap_or("None")
        .to_string();
    let owner_label = state
        .and_then(|state| state.owner.as_deref())
        .filter(|owner| !owner.trim().is_empty())
        .unwrap_or("None")
        .to_string();
    let signoff_label = state
        .and_then(|state| state.signoff.as_deref())
        .filter(|signoff| !signoff.trim().is_empty())
        .unwrap_or("None")
        .to_string();
    let tags_label = layout_marker_tags_label(state);
    let snapshot_label = layout_marker_snapshot_label(state);
    let snapshot_bounds_label = layout_marker_snapshot_bounds_label(state);
    let required = app.format_layout_length(violation.required as f64);
    let actual = format!("{:.1}", violation.actual);
    match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => vec![
            ("Marker id".to_string(), format!("#{}", violation.id)),
            ("Category".to_string(), category_label),
            ("Rule".to_string(), violation.rule.clone()),
            ("State".to_string(), state_label),
            ("Note".to_string(), note_label),
            ("Owner".to_string(), owner_label),
            ("Signoff".to_string(), signoff_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Marker id".to_string(), format!("#{}", violation.id)),
            ("Bounds".to_string(), rect_summary(violation.bounds)),
            ("Required".to_string(), required),
            ("Actual".to_string(), actual),
        ],
        LayoutBrowserColumnSet::Relations => vec![
            ("Marker id".to_string(), format!("#{}", violation.id)),
            ("Category".to_string(), category_label),
            ("Rule".to_string(), violation.rule.clone()),
            ("State".to_string(), state_label),
            (
                "Visited".to_string(),
                state.is_some_and(|state| state.visited).to_string(),
            ),
            (
                "Important".to_string(),
                state.is_some_and(|state| state.important).to_string(),
            ),
            ("Note".to_string(), note_label),
            ("Owner".to_string(), owner_label),
            ("Signoff".to_string(), signoff_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
            ("Shapes".to_string(), violation.shape_ids.len().to_string()),
        ],
        LayoutBrowserColumnSet::All => vec![
            ("Marker id".to_string(), format!("#{}", violation.id)),
            ("Category".to_string(), category_label),
            ("Rule".to_string(), violation.rule.clone()),
            ("State".to_string(), state_label),
            (
                "Visited".to_string(),
                state.is_some_and(|state| state.visited).to_string(),
            ),
            (
                "Important".to_string(),
                state.is_some_and(|state| state.important).to_string(),
            ),
            ("Note".to_string(), note_label),
            ("Owner".to_string(), owner_label),
            ("Signoff".to_string(), signoff_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
            ("Snapshot bounds".to_string(), snapshot_bounds_label),
            ("Shapes".to_string(), violation.shape_ids.len().to_string()),
            ("Bounds".to_string(), rect_summary(violation.bounds)),
            ("Required".to_string(), required),
            ("Actual".to_string(), actual),
        ],
    }
}

pub(crate) fn connectivity_component_display_name(component: &NetComponent) -> String {
    component
        .net_name
        .clone()
        .or_else(|| component.net_id.map(|id| format!("net {}", id.0)))
        .unwrap_or_else(|| format!("component {}", component.id))
}

pub(crate) fn layout_label_text_for_component(component: &NetComponent) -> String {
    component
        .net_name
        .clone()
        .or_else(|| component.net_id.map(|id| format!("NET{}", id.0)))
        .unwrap_or_else(|| format!("NET{}", component.id))
}

pub(crate) fn connectivity_component_label(component: &NetComponent) -> String {
    format!(
        "{} {} shape{}",
        connectivity_component_display_name(component),
        component.shapes.len(),
        if component.shapes.len() == 1 { "" } else { "s" }
    )
}
