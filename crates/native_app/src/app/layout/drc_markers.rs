#![allow(unused_imports)]
use super::*;
use base64::{Engine as _, engine::general_purpose};
use layout_model::MarkerSignoffRecord;

const LAYOUT_DRC_RDB_IMAGE_KEY_PREFIX: &str = "layout.drc_marker.rdb_image.";
pub const LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY: &str =
    "glassworks.layout.drc_marker_snapshot_thumbnail";

pub(crate) fn layout_drc_marker_entries(app: &GlassworksApp) -> Vec<LayoutDrcMarkerEntry> {
    layout_drc_marker_entries_with_filter(
        app,
        app.layout_drc_marker_filter,
        true,
        MAX_LAYOUT_DRC_MARKER_ROWS,
    )
}

pub(crate) fn layout_drc_marker_entries_with_filter(
    app: &GlassworksApp,
    filter: LayoutDrcMarkerFilter,
    apply_search: bool,
    limit: usize,
) -> Vec<LayoutDrcMarkerEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    let search = apply_search
        .then(|| layout_browser_search_query_lower(app))
        .flatten();
    let mut violations = report
        .violations
        .iter()
        .filter(|violation| {
            let key = violation.stable_key();
            let state = document.marker_states.get(&key);
            let category = LayoutDrcMarkerCategory::from_rule(&violation.rule);
            layout_drc_marker_filter_matches(app, state, violation, filter)
                && app
                    .layout_drc_marker_category_filter
                    .is_none_or(|filter| filter == category)
                && app
                    .layout_drc_marker_directory_filter
                    .as_ref()
                    .is_none_or(|filter| {
                        layout_drc_marker_directory_matches_filter(
                            &layout_drc_marker_directory_path_with_state(violation, state),
                            filter,
                        )
                    })
                && search.as_deref().is_none_or(|query| {
                    layout_drc_marker_matches_search(document, violation, state, query)
                })
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
        LayoutDrcMarkerSort::Category => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            let left_state = document.marker_states.get(&left_key);
            let right_state = document.marker_states.get(&right_key);
            let left_user_category = layout_drc_marker_user_category_path(left_state);
            let right_user_category = layout_drc_marker_user_category_path(right_state);
            match (left_user_category.as_ref(), right_user_category.as_ref()) {
                (Some(left_category), Some(right_category)) => left_category.cmp(right_category),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => LayoutDrcMarkerCategory::from_rule(&left.rule)
                    .cmp(&LayoutDrcMarkerCategory::from_rule(&right.rule)),
            }
            .then_with(|| left.rule.cmp(&right.rule))
            .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::Directory => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            layout_drc_marker_directory_path_with_state(left, document.marker_states.get(&left_key))
                .cmp(&layout_drc_marker_directory_path_with_state(
                    right,
                    document.marker_states.get(&right_key),
                ))
                .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::SourceCell => layout_drc_marker_source_cells_label(document, left)
            .cmp(&layout_drc_marker_source_cells_label(document, right))
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::SourceObject => {
            layout_drc_marker_source_objects_sort_key(document, left)
                .cmp(&layout_drc_marker_source_objects_sort_key(document, right))
                .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::SourceLayer => {
            layout_drc_marker_source_layers_sort_key(document, left)
                .cmp(&layout_drc_marker_source_layers_sort_key(document, right))
                .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::SourceKind => layout_drc_marker_source_kinds_sort_key(document, left)
            .cmp(&layout_drc_marker_source_kinds_sort_key(document, right))
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::SourceBounds => {
            layout_drc_marker_source_bounds_sort_key(document, left)
                .cmp(&layout_drc_marker_source_bounds_sort_key(document, right))
                .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::Signoff => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            layout_drc_marker_optional_text_sort_key(layout_drc_marker_signoff_status_sort_key(
                document.marker_states.get(&left_key),
            ))
            .cmp(&layout_drc_marker_optional_text_sort_key(
                layout_drc_marker_signoff_status_sort_key(document.marker_states.get(&right_key)),
            ))
            .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::SignoffRole => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            layout_drc_marker_optional_text_sort_key(layout_drc_marker_signoff_role_sort_key(
                document.marker_states.get(&left_key),
            ))
            .cmp(&layout_drc_marker_optional_text_sort_key(
                layout_drc_marker_signoff_role_sort_key(document.marker_states.get(&right_key)),
            ))
            .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::SignoffStamp => {
            let left_key = left.stable_key();
            let right_key = right.stable_key();
            let left_stamp = layout_drc_marker_signoff_recorded_at_sort_key(
                document.marker_states.get(&left_key),
            );
            let right_stamp = layout_drc_marker_signoff_recorded_at_sort_key(
                document.marker_states.get(&right_key),
            );
            match (left_stamp.as_ref(), right_stamp.as_ref()) {
                (Some(left_stamp), Some(right_stamp)) => right_stamp.cmp(left_stamp),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| left.id.cmp(&right.id))
        }
        LayoutDrcMarkerSort::Size => layout_drc_marker_size_sort_key(right)
            .cmp(&layout_drc_marker_size_sort_key(left))
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::Area => right
            .bounds
            .area()
            .cmp(&left.bounds.area())
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::Required => right
            .required
            .cmp(&left.required)
            .then_with(|| left.id.cmp(&right.id)),
        LayoutDrcMarkerSort::Actual => right
            .actual
            .total_cmp(&left.actual)
            .then_with(|| left.id.cmp(&right.id)),
    });
    violations
        .into_iter()
        .take(limit)
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

pub(crate) fn layout_drc_marker_filter_count(
    app: &GlassworksApp,
    filter: LayoutDrcMarkerFilter,
) -> usize {
    layout_drc_marker_entries_with_filter(app, filter, true, usize::MAX).len()
}

pub(crate) fn layout_drc_marker_filter_button_label(
    app: &GlassworksApp,
    filter: LayoutDrcMarkerFilter,
) -> String {
    format!(
        "{} ({})",
        filter.label(),
        layout_drc_marker_filter_count(app, filter)
    )
}

pub(crate) fn layout_drc_marker_browser_title(app: &GlassworksApp, listed_count: usize) -> String {
    let row_label = if listed_count == 1 { "row" } else { "rows" };
    let search = app.layout_browser_search.trim();
    if app.layout_drc_marker_filter == LayoutDrcMarkerFilter::Active
        && app.layout_drc_marker_category_filter.is_none()
        && app.layout_drc_marker_directory_filter.is_none()
        && search.is_empty()
    {
        return format!("DRC Marker Browser ({listed_count} {row_label})");
    }
    let mut context = Vec::new();
    if app.layout_drc_marker_filter != LayoutDrcMarkerFilter::Active
        || !search.is_empty()
        || app.layout_drc_marker_category_filter.is_some()
        || app.layout_drc_marker_directory_filter.is_some()
    {
        context.push(app.layout_drc_marker_filter.label().to_string());
    }
    if let Some(category) = app.layout_drc_marker_category_filter {
        context.push(format!("category {}", category.label()));
    }
    if let Some(directory) = app.layout_drc_marker_directory_filter.as_deref() {
        context.push(format!("directory {}", compact_button_label(directory, 24)));
    }
    if !search.is_empty() {
        context.push(format!("search {}", compact_button_label(search, 24)));
    }
    format!(
        "DRC Marker Browser - {} ({listed_count} {row_label})",
        context.join(" / ")
    )
}

pub(crate) fn layout_drc_marker_filter_status_label(filter: LayoutDrcMarkerFilter) -> String {
    match filter {
        LayoutDrcMarkerFilter::Active => "active markers".to_string(),
        LayoutDrcMarkerFilter::All => "markers".to_string(),
        LayoutDrcMarkerFilter::SelectedShape => "selected-shape markers".to_string(),
        LayoutDrcMarkerFilter::ActiveLayer => "active-layer markers".to_string(),
        LayoutDrcMarkerFilter::Hidden => "hidden markers".to_string(),
        LayoutDrcMarkerFilter::Waived => "waived markers".to_string(),
        LayoutDrcMarkerFilter::Visited => "visited markers".to_string(),
        LayoutDrcMarkerFilter::Important => "important markers".to_string(),
        LayoutDrcMarkerFilter::Noted => "noted markers".to_string(),
        LayoutDrcMarkerFilter::Owned => "owned markers".to_string(),
        LayoutDrcMarkerFilter::SignedOff => "signed-off markers".to_string(),
        LayoutDrcMarkerFilter::SignoffNeedsReview => "needs-review markers".to_string(),
        LayoutDrcMarkerFilter::SignoffAccepted => "accepted markers".to_string(),
        LayoutDrcMarkerFilter::SignoffRejected => "rejected markers".to_string(),
        LayoutDrcMarkerFilter::Tagged => "tagged markers".to_string(),
        LayoutDrcMarkerFilter::Snapshots => "snapshot markers".to_string(),
    }
}

pub(crate) fn layout_drc_marker_browser_status_scope_label(app: &GlassworksApp) -> String {
    let mut label = layout_drc_marker_filter_status_label(app.layout_drc_marker_filter);
    if let Some(category) = app.layout_drc_marker_category_filter {
        label = format!("{label} in {} category", category.label());
    }
    if let Some(directory) = app.layout_drc_marker_directory_filter.as_deref() {
        label = format!(
            "{label} in directory {}",
            compact_button_label(directory, 32)
        );
    }
    label
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
    if !report.findings.is_empty() && report.violations.is_empty() {
        return Vec::new();
    }

    let search = layout_browser_search_query_lower(app);
    let mut categories: BTreeMap<LayoutDrcMarkerCategory, (usize, usize)> = BTreeMap::new();
    for violation in &report.violations {
        let key = violation.stable_key();
        let state = document.marker_states.get(&key);
        if !layout_drc_marker_filter_matches(app, state, violation, app.layout_drc_marker_filter)
            || !search.as_deref().is_none_or(|query| {
                layout_drc_marker_matches_search(document, violation, state, query)
            })
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
    layout_drc_marker_directory_entries_with_options(app, true)
}

pub(crate) fn layout_drc_marker_directory_filter_entries(
    app: &GlassworksApp,
) -> Vec<LayoutDrcMarkerDirectoryEntry> {
    layout_drc_marker_directory_entries_with_options(app, false)
}

fn layout_drc_marker_directory_entries_with_options(
    app: &GlassworksApp,
    apply_directory_filter: bool,
) -> Vec<LayoutDrcMarkerDirectoryEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    if !report.findings.is_empty() && report.violations.is_empty() {
        return Vec::new();
    }

    let search = layout_browser_search_query_lower(app);
    let mut directories: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for violation in &report.violations {
        let key = violation.stable_key();
        let state = document.marker_states.get(&key);
        let category = LayoutDrcMarkerCategory::from_rule(&violation.rule);
        if !layout_drc_marker_filter_matches(app, state, violation, app.layout_drc_marker_filter)
            || !app
                .layout_drc_marker_category_filter
                .is_none_or(|filter| filter == category)
            || apply_directory_filter
                && !app
                    .layout_drc_marker_directory_filter
                    .as_ref()
                    .is_none_or(|filter| {
                        layout_drc_marker_directory_matches_filter(
                            &layout_drc_marker_directory_path_with_state(violation, state),
                            filter,
                        )
                    })
            || !search.as_deref().is_none_or(|query| {
                layout_drc_marker_matches_search(document, violation, state, query)
            })
        {
            continue;
        }
        let active = drc_violation_is_active(document, violation);
        for path in layout_drc_marker_directory_prefix_paths(
            &layout_drc_marker_directory_path_with_state(violation, state),
        ) {
            let entry = directories.entry(path).or_insert((0, 0));
            entry.0 += 1;
            if active {
                entry.1 += 1;
            }
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

pub(crate) fn layout_drc_marker_directory_path_with_state(
    violation: &DrcViolation,
    state: Option<&MarkerState>,
) -> String {
    layout_drc_marker_user_category_path(state)
        .unwrap_or_else(|| layout_drc_marker_directory_path(violation))
}

pub(crate) fn layout_drc_marker_directory_matches_filter(path: &str, filter: &str) -> bool {
    path == filter
        || path
            .strip_prefix(filter)
            .is_some_and(|suffix| suffix.starts_with(" / "))
}

fn layout_drc_marker_directory_prefix_paths(path: &str) -> Vec<String> {
    let parts = path
        .split(" / ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut prefixes = Vec::with_capacity(parts.len().max(1));
    for index in 0..parts.len() {
        prefixes.push(parts[..=index].join(" / "));
    }
    if prefixes.is_empty() {
        prefixes.push(path.to_string());
    }
    prefixes
}

pub(crate) fn layout_drc_marker_category_label(
    violation: &DrcViolation,
    state: Option<&MarkerState>,
) -> String {
    layout_drc_marker_user_category_path(state).unwrap_or_else(|| {
        LayoutDrcMarkerCategory::from_rule(&violation.rule)
            .label()
            .to_string()
    })
}

pub(crate) fn layout_drc_marker_user_category_path(state: Option<&MarkerState>) -> Option<String> {
    let state = state?;
    if let Some(path) = layout_drc_marker_rdb_category_part_path(state) {
        return Some(path);
    }
    for key in ["category", "marker_category", "rdb_category"] {
        let Some(value) = state.tags.get(key).map(|value| value.trim()) else {
            continue;
        };
        let parts = value
            .split(['/', ':', '.'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            return Some(format!("User / {}", parts.join(" / ")));
        }
    }
    None
}

fn layout_drc_marker_rdb_category_part_path(state: &MarkerState) -> Option<String> {
    let mut parts = Vec::new();
    for index in 1..=64 {
        let key = format!("rdb_category_part_{index}");
        let Some(part) = state
            .tags
            .get(&key)
            .map(String::as_str)
            .map(str::trim)
            .filter(|part| !part.is_empty())
        else {
            if index == 1 {
                return None;
            }
            break;
        };
        parts.push(part.to_string());
    }
    (!parts.is_empty()).then(|| format!("User / {}", parts.join(" / ")))
}

pub(crate) fn layout_drc_marker_size_sort_key(violation: &DrcViolation) -> (Coord, Coord, Coord) {
    let width = violation.bounds.width().max(0);
    let height = violation.bounds.height().max(0);
    (
        width.max(height),
        width.min(height),
        violation.bounds.area(),
    )
}

pub(crate) fn layout_drc_marker_snapshot_entries(
    app: &GlassworksApp,
) -> Vec<LayoutDrcMarkerSnapshotEntry> {
    let search = layout_browser_search_query_lower(app);
    layout_drc_marker_snapshot_entries_with_search(app, search.as_deref())
}

pub(crate) fn layout_drc_marker_snapshot_total_count(app: &GlassworksApp) -> usize {
    layout_drc_marker_snapshot_entries_with_search(app, None).len()
}

pub(crate) fn layout_drc_marker_snapshot_title(
    app: &GlassworksApp,
    listed_count: usize,
    total_count: usize,
) -> String {
    let search = app.layout_browser_search.trim();
    if search.is_empty() {
        return format!("Marker Snapshots ({listed_count})");
    }
    format!(
        "Marker Snapshots - search {} ({listed_count} / {total_count})",
        compact_button_label(search, 24)
    )
}

pub(crate) fn layout_drc_report_browser_entries(
    app: &GlassworksApp,
) -> Vec<&DrcReportHistoryEntry> {
    let search = layout_browser_search_query_lower(app);
    app.layout_drc_report_history
        .iter()
        .filter(|entry| {
            search
                .as_deref()
                .is_none_or(|query| layout_drc_report_matches_search(app, entry, query))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutDrcReportSourceEntry {
    pub(crate) source: String,
    pub(crate) total_reports: usize,
    pub(crate) listed_reports: usize,
    pub(crate) marker_count: usize,
    pub(crate) diagnostic_count: usize,
}

pub(crate) fn layout_drc_report_source_entries(
    app: &GlassworksApp,
) -> Vec<LayoutDrcReportSourceEntry> {
    let search = layout_browser_search_query_lower(app);
    let mut entries = BTreeMap::<String, LayoutDrcReportSourceEntry>::new();
    for entry in &app.layout_drc_report_history {
        let Some(source) = layout_drc_report_source(entry) else {
            continue;
        };
        let report_matches = search
            .as_deref()
            .is_none_or(|query| layout_drc_report_matches_search(app, entry, query));
        let source_entry =
            entries
                .entry(source.to_string())
                .or_insert_with(|| LayoutDrcReportSourceEntry {
                    source: source.to_string(),
                    total_reports: 0,
                    listed_reports: 0,
                    marker_count: 0,
                    diagnostic_count: 0,
                });
        source_entry.total_reports += 1;
        source_entry.marker_count += entry.value.violations.len();
        source_entry.diagnostic_count += entry.value.findings.len();
        if report_matches {
            source_entry.listed_reports += 1;
        }
    }
    entries.into_values().collect()
}

pub(crate) fn layout_drc_report_source_search_query(source: &str) -> String {
    format!("source={source}")
}

pub(crate) fn layout_drc_report_source_filter_is_active(app: &GlassworksApp, source: &str) -> bool {
    app.layout_browser_search
        .trim()
        .eq_ignore_ascii_case(&layout_drc_report_source_search_query(source))
}

pub(crate) fn layout_drc_report_source_summary_label(
    entries: &[LayoutDrcReportSourceEntry],
) -> String {
    if entries.is_empty() {
        return "None".to_string();
    }
    let mut label = entries
        .iter()
        .take(3)
        .map(|entry| {
            format!(
                "{}={}",
                compact_button_label(&entry.source, 18),
                entry.total_reports
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = entries.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 80)
}

fn layout_drc_report_source_count_summary_label(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "None".to_string();
    }
    let mut label = counts
        .iter()
        .take(3)
        .map(|(source, count)| format!("{}={count}", compact_button_label(source, 18)))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = counts.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 80)
}

pub(crate) fn layout_drc_report_browser_title(
    app: &GlassworksApp,
    listed_count: usize,
    total_count: usize,
) -> String {
    let search = app.layout_browser_search.trim();
    if search.is_empty() {
        return format!("DRC Reports ({total_count})");
    }
    format!(
        "DRC Reports - search {} ({listed_count} / {total_count})",
        compact_button_label(search, 24)
    )
}

pub(crate) fn layout_drc_report_button_label(
    app: &GlassworksApp,
    entry: &DrcReportHistoryEntry,
) -> String {
    compact_button_label(
        &format!(
            "#{} {} {} {}",
            entry.id,
            entry.label,
            layout_drc_report_summary_label(entry),
            layout_drc_report_freshness_label(entry, app.layout_revision)
        ),
        30,
    )
}

fn layout_drc_report_source(entry: &DrcReportHistoryEntry) -> Option<&str> {
    entry
        .source
        .as_deref()
        .map(str::trim)
        .filter(|source| !source.is_empty())
}

fn layout_drc_report_matches_search(
    app: &GlassworksApp,
    entry: &DrcReportHistoryEntry,
    query_lower: &str,
) -> bool {
    if layout_drc_report_selector_matches_search(app, entry, query_lower) {
        return true;
    }
    if layout_drc_report_marker_matches_search(app, entry, query_lower) {
        return true;
    }
    let marker_count = entry.value.violations.len();
    let diagnostic_count = entry.value.findings.len();
    let warning_count = entry
        .value
        .findings
        .iter()
        .filter(|finding| finding.severity == DrcValidationSeverity::Warning)
        .count();
    let error_count = diagnostic_count.saturating_sub(warning_count);
    let selection = if app.layout_selected_drc_report_id == Some(entry.id) {
        "active"
    } else {
        "stored"
    };
    let mut fields = vec![
        entry.id.to_string(),
        format!("#{}", entry.id),
        entry.label.clone(),
        "drc report".to_string(),
        layout_drc_report_summary_label(entry),
        layout_drc_report_freshness_label(entry, app.layout_revision).to_string(),
        selection.to_string(),
        format!("{marker_count} marker"),
        format!("{marker_count} markers"),
        format!("{diagnostic_count} diagnostic"),
        format!("{diagnostic_count} diagnostics"),
        format!("{diagnostic_count} issue"),
        format!("{diagnostic_count} issues"),
        format!("{warning_count} warning"),
        format!("{warning_count} warnings"),
        format!("{error_count} error"),
        format!("{error_count} errors"),
    ];
    if let Some(source) = layout_drc_report_source(entry) {
        fields.push(source.to_string());
        fields.push(format!("source {source}"));
        fields.push(format!("database {source}"));
    }
    for finding in &entry.value.findings {
        fields.push(finding.message.clone());
        fields.push(match finding.severity {
            DrcValidationSeverity::Error => "error diagnostic".to_string(),
            DrcValidationSeverity::Warning => "warning diagnostic".to_string(),
        });
    }
    layout_search_matches_any(query_lower, &fields)
}

fn layout_drc_report_marker_matches_search(
    app: &GlassworksApp,
    entry: &DrcReportHistoryEntry,
    query_lower: &str,
) -> bool {
    let document = &app.workspace.document;
    entry.value.violations.iter().any(|violation| {
        let key = violation.stable_key();
        layout_drc_marker_matches_search(
            document,
            violation,
            document.marker_states.get(&key),
            query_lower,
        )
    })
}

fn layout_drc_report_selector_matches_search(
    app: &GlassworksApp,
    entry: &DrcReportHistoryEntry,
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
    let marker_count = entry.value.violations.len().to_string();
    let diagnostic_count = entry.value.findings.len().to_string();
    let warning_count = entry
        .value
        .findings
        .iter()
        .filter(|finding| finding.severity == DrcValidationSeverity::Warning)
        .count()
        .to_string();
    let error_count = entry
        .value
        .findings
        .iter()
        .filter(|finding| finding.severity == DrcValidationSeverity::Error)
        .count()
        .to_string();
    let freshness = layout_drc_report_freshness_label(entry, app.layout_revision);
    let selection = if app.layout_selected_drc_report_id == Some(entry.id) {
        "active"
    } else {
        "stored"
    };
    let mut fields = vec![
        ("id", entry.id.to_string()),
        ("report", entry.label.clone()),
        ("label", entry.label.clone()),
        ("name", entry.label.clone()),
        ("type", "drc report".to_string()),
        ("markers", marker_count.clone()),
        ("marker_count", marker_count),
        ("diagnostics", diagnostic_count.clone()),
        ("diagnostic_count", diagnostic_count.clone()),
        ("issues", diagnostic_count.clone()),
        ("issue_count", diagnostic_count),
        ("warnings", warning_count.clone()),
        ("warning_count", warning_count),
        ("errors", error_count.clone()),
        ("error_count", error_count),
        ("status", freshness.to_string()),
        ("freshness", freshness.to_string()),
        ("state", freshness.to_string()),
        ("selection", selection.to_string()),
    ];
    if let Some(source) = layout_drc_report_source(entry) {
        fields.push(("source", source.to_string()));
        fields.push(("database", source.to_string()));
        fields.push(("file", source.to_string()));
        fields.push(("path", source.to_string()));
    }
    for finding in &entry.value.findings {
        let severity = match finding.severity {
            DrcValidationSeverity::Error => "error",
            DrcValidationSeverity::Warning => "warning",
        };
        fields.push(("diagnostic", finding.message.clone()));
        fields.push(("diagnostic_message", finding.message.clone()));
        fields.push(("message", finding.message.clone()));
        fields.push(("severity", severity.to_string()));
        match finding.severity {
            DrcValidationSeverity::Error => {
                fields.push(("error", finding.message.clone()));
                fields.push(("error_message", finding.message.clone()));
            }
            DrcValidationSeverity::Warning => {
                fields.push(("warning", finding.message.clone()));
                fields.push(("warning_message", finding.message.clone()));
            }
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

fn layout_drc_report_summary_label(entry: &DrcReportHistoryEntry) -> String {
    match (
        entry.value.violations.is_empty(),
        entry.value.findings.is_empty(),
    ) {
        (false, false) => format!(
            "{} marker(s), {}",
            entry.value.violations.len(),
            layout_drc_report_diagnostics_summary_label(&entry.value.findings)
        ),
        (false, true) => format!("{} marker(s)", entry.value.violations.len()),
        (true, false) => layout_drc_report_diagnostics_summary_label(&entry.value.findings),
        (true, true) => "0 marker(s)".to_string(),
    }
}

pub(crate) fn layout_drc_report_diagnostics_summary_label(
    findings: &[DrcValidationFinding],
) -> String {
    let errors = findings
        .iter()
        .filter(|finding| finding.severity == DrcValidationSeverity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == DrcValidationSeverity::Warning)
        .count();
    match (errors, warnings) {
        (0, 0) => "0 diagnostics".to_string(),
        (0, 1) => "1 warning".to_string(),
        (0, warnings) => format!("{warnings} warnings"),
        (1, 0) => "1 error".to_string(),
        (errors, 0) => format!("{errors} errors"),
        (1, 1) => "1 error / 1 warning".to_string(),
        (1, warnings) => format!("1 error / {warnings} warnings"),
        (errors, 1) => format!("{errors} errors / 1 warning"),
        (errors, warnings) => format!("{errors} errors / {warnings} warnings"),
    }
}

pub(crate) fn layout_drc_report_diagnostic_rows(
    findings: &[DrcValidationFinding],
    limit: usize,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if findings.is_empty() {
        return rows;
    }
    rows.push((
        "Report diagnostics".to_string(),
        layout_drc_report_diagnostics_summary_label(findings),
    ));
    for (index, finding) in findings.iter().take(limit).enumerate() {
        let severity = match finding.severity {
            DrcValidationSeverity::Error => "error",
            DrcValidationSeverity::Warning => "warning",
        };
        rows.push((
            format!("Report {severity} {}", index + 1),
            compact_button_label(&finding.message, 96),
        ));
    }
    let remaining = findings.len().saturating_sub(limit);
    if remaining > 0 {
        rows.push(("More diagnostics".to_string(), remaining.to_string()));
    }
    rows
}

fn layout_drc_report_freshness_label(
    entry: &DrcReportHistoryEntry,
    layout_revision: u64,
) -> &'static str {
    if entry.revision == layout_revision {
        "current"
    } else {
        "stale"
    }
}

fn layout_drc_marker_snapshot_entries_with_search(
    app: &GlassworksApp,
    search: Option<&str>,
) -> Vec<LayoutDrcMarkerSnapshotEntry> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_DRC_INSPECTOR_SHAPES {
        return Vec::new();
    }
    let Some(report) = app.drc_report() else {
        return Vec::new();
    };
    let mut entries = report
        .violations
        .iter()
        .filter_map(|violation| {
            let key = violation.stable_key();
            let state = document.marker_states.get(&key)?;
            if !layout_marker_state_has_snapshot(state) {
                return None;
            }
            let category = LayoutDrcMarkerCategory::from_rule(&violation.rule);
            if !app
                .layout_drc_marker_category_filter
                .is_none_or(|filter| filter == category)
                || !app
                    .layout_drc_marker_directory_filter
                    .as_ref()
                    .is_none_or(|filter| {
                        layout_drc_marker_directory_matches_filter(
                            &layout_drc_marker_directory_path_with_state(violation, Some(state)),
                            filter,
                        )
                    })
                || !search.is_none_or(|query| {
                    layout_drc_marker_matches_search(document, violation, Some(state), query)
                })
            {
                return None;
            }
            let image_key = layout_marker_snapshot_image_key(&key, state)?;
            Some(LayoutDrcMarkerSnapshotEntry {
                id: violation.id,
                key,
                image_key,
                label: compact_button_label(
                    &format!(
                        "#{} {}",
                        violation.id,
                        layout_marker_snapshot_label(Some(state))
                    ),
                    52,
                ),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries
}

pub(crate) fn layout_drc_marker_directory_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let entries = layout_drc_marker_directory_entries(app);
    let search = layout_browser_search_query_lower(app);
    let mut rows = Vec::new();
    if search.is_some() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push(("Listed directories".to_string(), entries.len().to_string()));
    }
    if entries.is_empty() {
        rows.push(("Directories".to_string(), "None".to_string()));
        return rows;
    }
    rows.push(("Directories".to_string(), entries.len().to_string()));
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
    document: &Document,
    violation: &DrcViolation,
    state: Option<&MarkerState>,
    query_lower: &str,
) -> bool {
    if layout_drc_marker_selector_matches_search(document, violation, state, query_lower) {
        return true;
    }
    let mut fields = vec![
        format!("#{}", violation.id),
        format!("marker {}", violation.id),
        violation.rule.clone(),
        layout_drc_marker_category_label(violation, state),
        LayoutDrcMarkerCategory::from_rule(&violation.rule)
            .label()
            .to_string(),
        layout_drc_marker_directory_path_with_state(violation, state),
        layout_drc_marker_source_cells_label(document, violation),
        layout_drc_marker_source_cells_search_label(document, violation),
        layout_drc_marker_source_objects_label(document, violation),
        layout_drc_marker_source_objects_search_label(document, violation),
        layout_drc_marker_source_layers_label(document, violation),
        layout_drc_marker_source_layers_search_label(document, violation),
        layout_drc_marker_source_kinds_label(document, violation),
        layout_drc_marker_source_bounds_label(document, violation),
        layout_drc_marker_state_label(state).to_string(),
        violation.stable_key(),
        format!("{} shapes", violation.shape_ids.len()),
        rect_summary(violation.bounds),
        format!("center {}", point_summary(violation.bounds.center())),
        format!("width {}", violation.bounds.width()),
        format!("height {}", violation.bounds.height()),
        format!("area {}", violation.bounds.area()),
        layout_drc_marker_shape_ids_label(violation),
        layout_drc_marker_occurrences_label(violation),
        format!("required {}", violation.required),
        format!("actual {:.1}", violation.actual),
    ];
    for shape_id in &violation.shape_ids {
        push_layout_search_field(&mut fields, format!("shape {}", shape_id.0));
        push_layout_search_field(&mut fields, format!("#{}", shape_id.0));
    }
    for (shape_id, shape) in layout_drc_marker_source_shapes(document, violation) {
        push_layout_search_field(&mut fields, format!("source shape {}", shape_id.0));
        push_layout_search_field(&mut fields, format!("source object {}", shape_id.0));
        push_layout_search_field(&mut fields, shape_kind_label(&shape.kind));
        push_layout_search_field(
            &mut fields,
            layout_drc_marker_layer_search_label(document, shape.layer),
        );
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
    if let Some(signoff_by) = state.and_then(|state| state.signoff_by.as_deref()) {
        push_layout_search_field(&mut fields, signoff_by);
    }
    if let Some(signoff_note) = state.and_then(|state| state.signoff_note.as_deref()) {
        push_layout_search_field(&mut fields, signoff_note);
    }
    let signoff_detail = layout_drc_marker_signoff_detail_label(state);
    if signoff_detail != "None" {
        push_layout_search_field(&mut fields, signoff_detail);
    }
    let signoff_records = layout_marker_signoff_records_label(state);
    if signoff_records != "None" {
        push_layout_search_field(&mut fields, signoff_records);
    }
    if let Some(state) = state {
        for (tag_key, tag_value) in &state.tags {
            push_layout_search_field(&mut fields, tag_key);
            push_layout_search_field(&mut fields, tag_value);
            push_layout_search_field(&mut fields, format!("{tag_key}={tag_value}"));
        }
    }
    let snapshot_label = layout_marker_snapshot_label(state);
    if snapshot_label != "None" {
        push_layout_search_field(&mut fields, snapshot_label);
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_drc_marker_selector_matches_search(
    document: &Document,
    violation: &DrcViolation,
    state: Option<&MarkerState>,
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
        ("id", violation.id.to_string()),
        ("marker", violation.id.to_string()),
        ("rule", violation.rule.clone()),
        (
            "category",
            layout_drc_marker_category_label(violation, state),
        ),
        (
            "rule_category",
            LayoutDrcMarkerCategory::from_rule(&violation.rule)
                .label()
                .to_string(),
        ),
        (
            "directory",
            layout_drc_marker_directory_path_with_state(violation, state),
        ),
        (
            "source_cell",
            layout_drc_marker_source_cells_search_label(document, violation),
        ),
        (
            "source_object",
            layout_drc_marker_source_objects_search_label(document, violation),
        ),
        (
            "source_layer",
            layout_drc_marker_source_layers_search_label(document, violation),
        ),
        (
            "source_kind",
            layout_drc_marker_source_kinds_label(document, violation),
        ),
        (
            "source_bounds",
            layout_drc_marker_source_bounds_label(document, violation),
        ),
        ("message", violation.message.clone()),
        ("state", layout_drc_marker_state_label(state).to_string()),
        ("stable_key", violation.stable_key()),
        ("shapes", violation.shape_ids.len().to_string()),
        ("bounds", rect_summary(violation.bounds)),
        ("center", point_summary(violation.bounds.center())),
        ("width", violation.bounds.width().to_string()),
        ("height", violation.bounds.height().to_string()),
        ("area", violation.bounds.area().to_string()),
        ("required", violation.required.to_string()),
        ("actual", format!("{:.1}", violation.actual)),
    ];
    for shape_id in &violation.shape_ids {
        fields.push(("shape", shape_id.0.to_string()));
    }
    for (shape_id, shape) in layout_drc_marker_source_shapes(document, violation) {
        let layer_label = layout_drc_marker_layer_search_label(document, shape.layer);
        fields.push(("source_shape", shape_id.0.to_string()));
        fields.push(("source_shape_id", shape_id.0.to_string()));
        fields.push((
            "source_object",
            format!(
                "#{} shape {} {} {layer_label}",
                shape_id.0,
                shape_id.0,
                shape_kind_label(&shape.kind),
            ),
        ));
        fields.push(("source_kind", shape_kind_label(&shape.kind).to_string()));
        fields.push(("source_layer", layer_label));
        fields.push(("source_layer_id", shape.layer.0.to_string()));
        fields.push(("source_bounds", rect_summary(shape.kind.bounds())));
    }
    for occurrence in &violation.occurrence_ids {
        fields.push(("occurrence", layout_occurrence_label(occurrence)));
    }
    if let Some(note) = state.and_then(|state| state.note.as_deref()) {
        fields.push(("note", note.to_string()));
    }
    if let Some(owner) = state.and_then(|state| state.owner.as_deref()) {
        fields.push(("owner", owner.to_string()));
    }
    if let Some(signoff) = state.and_then(|state| state.signoff.as_deref()) {
        fields.push(("signoff", signoff.to_string()));
    }
    if let Some(signoff_by) = state.and_then(|state| state.signoff_by.as_deref()) {
        fields.push(("signoff_by", signoff_by.to_string()));
    }
    if let Some(signoff_note) = state.and_then(|state| state.signoff_note.as_deref()) {
        fields.push(("signoff_note", signoff_note.to_string()));
    }
    let signoff_detail = layout_drc_marker_signoff_detail_label(state);
    if signoff_detail != "None" {
        fields.push(("signoff_detail", signoff_detail));
    }
    let signoff_records = layout_marker_signoff_records_label(state);
    if signoff_records != "None" {
        fields.push(("signoff_records", signoff_records));
    }
    if let Some(state) = state {
        for (party, signoff) in &state.signoff_records {
            fields.push(("signoff_party", party.clone()));
            fields.push(("signoff_status", signoff.status.clone()));
            fields.push((
                "signoff_record",
                layout_marker_signoff_record_label(party, signoff),
            ));
            if let Some(role) = signoff
                .role
                .as_deref()
                .filter(|role| !role.trim().is_empty())
            {
                fields.push(("signoff_role", role.to_string()));
            }
            if let Some(by) = signoff
                .by
                .as_deref()
                .filter(|signer| !signer.trim().is_empty())
            {
                fields.push(("signoff_record_by", by.to_string()));
            }
            if let Some(note) = signoff
                .note
                .as_deref()
                .filter(|note| !note.trim().is_empty())
            {
                fields.push(("signoff_record_note", note.to_string()));
            }
            if let Some(recorded_at) = signoff
                .recorded_at
                .as_deref()
                .filter(|recorded_at| !recorded_at.trim().is_empty())
            {
                fields.push(("signoff_at", recorded_at.to_string()));
                fields.push(("signoff_recorded_at", recorded_at.to_string()));
            }
        }
        if let Some(user_category) = layout_drc_marker_user_category_path(Some(state)) {
            fields.push(("user_category", user_category));
        }
        for (tag_key, tag_value) in &state.tags {
            if property_selector_key_value_matches(tag_key, tag_value, key_query, value_query)
                || property_selector_key_value_matches(
                    "tag",
                    &format!("{tag_key}={tag_value}"),
                    key_query,
                    value_query,
                )
            {
                return true;
            }
        }
        let snapshot_label = layout_marker_snapshot_label(Some(state));
        if snapshot_label != "None" {
            fields.push(("snapshot", snapshot_label));
        }
        let snapshot_bounds = layout_marker_snapshot_bounds_label(Some(state));
        if snapshot_bounds != "None" {
            fields.push(("snapshot_bounds", snapshot_bounds));
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_drc_marker_filter_matches(
    app: &GlassworksApp,
    state: Option<&MarkerState>,
    violation: &DrcViolation,
    filter: LayoutDrcMarkerFilter,
) -> bool {
    match filter {
        LayoutDrcMarkerFilter::Active => state.is_none_or(|state| !state.hidden && !state.waived),
        LayoutDrcMarkerFilter::All => true,
        LayoutDrcMarkerFilter::SelectedShape => {
            layout_drc_marker_matches_selected_shape(app, violation)
        }
        LayoutDrcMarkerFilter::ActiveLayer => {
            layout_drc_marker_matches_active_layer(app, violation)
        }
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
                || !state.signoff_records.is_empty()
        }),
        LayoutDrcMarkerFilter::SignoffNeedsReview => {
            layout_drc_marker_has_signoff_status(state, "needs_review")
        }
        LayoutDrcMarkerFilter::SignoffAccepted => {
            layout_drc_marker_has_signoff_status(state, "accepted")
        }
        LayoutDrcMarkerFilter::SignoffRejected => {
            layout_drc_marker_has_signoff_status(state, "rejected")
        }
        LayoutDrcMarkerFilter::Tagged => state.is_some_and(|state| !state.tags.is_empty()),
        LayoutDrcMarkerFilter::Snapshots => state.is_some_and(layout_marker_state_has_snapshot),
    }
}

fn layout_drc_marker_has_signoff_status(state: Option<&MarkerState>, status: &str) -> bool {
    let Some(state) = state else {
        return false;
    };
    let status = status.trim();
    state
        .signoff
        .as_deref()
        .is_some_and(|signoff| signoff.trim().eq_ignore_ascii_case(status))
        || state
            .signoff_records
            .values()
            .any(|signoff| signoff.status.trim().eq_ignore_ascii_case(status))
}

pub(crate) fn layout_drc_marker_matches_active_layer(
    app: &GlassworksApp,
    violation: &DrcViolation,
) -> bool {
    let document = &app.workspace.document;
    violation.occurrence_ids.iter().any(|occurrence| {
        document
            .shape_view_for_occurrence_from_cell(document.top_cell, occurrence)
            .is_some_and(|view| view.shape.layer == app.active_layer)
    }) || violation.shape_ids.iter().any(|shape_id| {
        document
            .shapes
            .get(shape_id)
            .is_some_and(|shape| shape.layer == app.active_layer)
            || document.cells.values().any(|cell| {
                cell.shapes
                    .get(shape_id)
                    .is_some_and(|shape| shape.layer == app.active_layer)
            })
    })
}

pub(crate) fn layout_drc_marker_matches_selected_shape(
    app: &GlassworksApp,
    violation: &DrcViolation,
) -> bool {
    let selected_shape = app
        .selected_layout_occurrence
        .as_ref()
        .map(ShapeOccurrenceId::source_shape_id)
        .or(app.selected_layout_shape);
    let Some(selected_shape) = selected_shape else {
        return false;
    };
    violation.shape_ids.contains(&selected_shape)
        || violation
            .occurrence_ids
            .iter()
            .any(|occurrence| occurrence.source_shape_id() == selected_shape)
}

pub(crate) fn layout_marker_state_has_snapshot(state: &MarkerState) -> bool {
    layout_marker_screenshot_path(state).is_some()
        || layout_marker_embedded_rdb_image_base64(state).is_some()
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
        || !state.signoff_records.is_empty()
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

pub(crate) fn layout_drc_marker_signoff_by_label(state: Option<&MarkerState>) -> String {
    state
        .and_then(|state| state.signoff_by.as_deref())
        .filter(|signoff_by| !signoff_by.trim().is_empty())
        .unwrap_or("None")
        .to_string()
}

pub(crate) fn layout_drc_marker_signoff_note_label(state: Option<&MarkerState>) -> String {
    state
        .and_then(|state| state.signoff_note.as_deref())
        .filter(|signoff_note| !signoff_note.trim().is_empty())
        .unwrap_or("None")
        .to_string()
}

pub(crate) fn layout_drc_marker_signoff_detail_label(state: Option<&MarkerState>) -> String {
    let Some(state) = state else {
        return "None".to_string();
    };
    let mut parts = Vec::new();
    if let Some(signoff) = state
        .signoff
        .as_deref()
        .filter(|signoff| !signoff.trim().is_empty())
    {
        parts.push(format!("status={signoff}"));
    }
    if let Some(signoff_by) = state
        .signoff_by
        .as_deref()
        .filter(|signoff_by| !signoff_by.trim().is_empty())
    {
        parts.push(format!("by={signoff_by}"));
    }
    if let Some(signoff_note) = state
        .signoff_note
        .as_deref()
        .filter(|signoff_note| !signoff_note.trim().is_empty())
    {
        parts.push(format!("note={signoff_note}"));
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("; ")
    }
}

fn layout_drc_marker_optional_text_sort_key(value: Option<String>) -> (bool, String) {
    match value {
        Some(value) if !value.trim().is_empty() => (false, value.to_ascii_lowercase()),
        _ => (true, String::new()),
    }
}

fn layout_drc_marker_signoff_status_sort_key(state: Option<&MarkerState>) -> Option<String> {
    let state = state?;
    state
        .signoff
        .as_deref()
        .filter(|signoff| !signoff.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            state
                .signoff_records
                .values()
                .map(|signoff| signoff.status.as_str())
                .filter(|status| !status.trim().is_empty())
                .min()
                .map(str::to_string)
        })
}

fn layout_drc_marker_signoff_role_sort_key(state: Option<&MarkerState>) -> Option<String> {
    let state = state?;
    state
        .signoff_records
        .values()
        .filter_map(|signoff| signoff.role.as_deref())
        .filter(|role| !role.trim().is_empty())
        .min()
        .map(str::to_string)
}

fn layout_drc_marker_signoff_recorded_at_sort_key(state: Option<&MarkerState>) -> Option<String> {
    let state = state?;
    state
        .signoff_records
        .values()
        .filter_map(|signoff| signoff.recorded_at.as_deref())
        .filter(|recorded_at| !recorded_at.trim().is_empty())
        .max()
        .map(str::to_string)
}

pub(crate) fn layout_marker_record_current_signoff(
    state: &mut MarkerState,
    recorded_at: Option<String>,
) {
    let Some(status) = state
        .signoff
        .as_deref()
        .filter(|signoff| !signoff.trim().is_empty())
        .map(str::to_string)
    else {
        return;
    };
    let Some(party) = state
        .signoff_by
        .as_deref()
        .or(state.owner.as_deref())
        .filter(|party| !party.trim().is_empty())
        .map(str::to_string)
    else {
        return;
    };
    state.signoff_records.insert(
        party,
        MarkerSignoffRecord {
            status,
            role: Some(layout_marker_signoff_role_for_party(
                state
                    .signoff_by
                    .as_deref()
                    .or(state.owner.as_deref())
                    .unwrap_or_default(),
            )),
            by: state
                .signoff_by
                .as_deref()
                .filter(|signer| !signer.trim().is_empty())
                .map(str::to_string),
            note: state
                .signoff_note
                .as_deref()
                .filter(|note| !note.trim().is_empty())
                .map(str::to_string),
            recorded_at,
        },
    );
}

pub(crate) fn layout_marker_signoff_recorded_at(layout_revision: u64) -> String {
    let unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("layout_revision:{layout_revision};unix_seconds:{unix_seconds}")
}

fn layout_marker_signoff_role_for_party(party: &str) -> String {
    let lower = party.to_ascii_lowercase();
    if lower.contains("layout") {
        "layout".to_string()
    } else if lower.contains("process") {
        "process".to_string()
    } else if lower.contains("qa") || lower.contains("quality") {
        "quality".to_string()
    } else if lower.contains("external") || lower.contains("rve") || lower.contains("calibre") {
        "external".to_string()
    } else {
        "review".to_string()
    }
}

pub(crate) fn layout_marker_signoff_records_label(state: Option<&MarkerState>) -> String {
    let Some(state) = state else {
        return "None".to_string();
    };
    if state.signoff_records.is_empty() {
        return "None".to_string();
    }
    let entries = state
        .signoff_records
        .iter()
        .map(|(party, signoff)| layout_marker_signoff_record_label(party, signoff))
        .collect::<Vec<_>>();
    format!(
        "{}: {}",
        layout_drc_marker_count_label(entries.len(), "party", "parties"),
        entries.join("; ")
    )
}

fn layout_marker_signoff_record_label(party: &str, signoff: &MarkerSignoffRecord) -> String {
    let mut parts = vec![format!("{party}={}", signoff.status)];
    if let Some(role) = signoff
        .role
        .as_deref()
        .filter(|role| !role.trim().is_empty())
    {
        parts.push(format!("role={role}"));
    }
    if let Some(by) = signoff
        .by
        .as_deref()
        .filter(|signer| !signer.trim().is_empty())
    {
        parts.push(format!("by={by}"));
    }
    if let Some(note) = signoff
        .note
        .as_deref()
        .filter(|note| !note.trim().is_empty())
    {
        parts.push(format!("note={note}"));
    }
    if let Some(recorded_at) = signoff
        .recorded_at
        .as_deref()
        .filter(|recorded_at| !recorded_at.trim().is_empty())
    {
        parts.push(format!("at={recorded_at}"));
    }
    parts.join(" ")
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
    if let Some(path) = layout_marker_screenshot_path(state) {
        let format = state
            .tags
            .get("screenshot_format")
            .filter(|format| !format.trim().is_empty());
        return match state
            .tags
            .get("screenshot_size")
            .filter(|size| !size.trim().is_empty())
        {
            Some(size) => compact_button_label(
                &format!(
                    "{} {} {}",
                    size,
                    format.map(String::as_str).unwrap_or("snapshot"),
                    compact_button_label(path, 36)
                ),
                52,
            ),
            None => compact_button_label(path, 52),
        };
    }
    if layout_marker_embedded_rdb_image_base64(state).is_some() {
        return "embedded KLayout RDB image".to_string();
    }
    "None".to_string()
}

pub(crate) fn layout_marker_snapshot_bounds_label(state: Option<&MarkerState>) -> String {
    state
        .and_then(|state| state.tags.get("screenshot_bounds"))
        .filter(|bounds| !bounds.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "None".to_string())
}

fn layout_marker_screenshot_path(state: &MarkerState) -> Option<&str> {
    state
        .tags
        .get("screenshot")
        .map(String::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
}

fn layout_marker_embedded_rdb_image_base64(state: &MarkerState) -> Option<&str> {
    state
        .tags
        .get("rdb_image_base64")
        .map(String::as_str)
        .map(str::trim)
        .filter(|image| !image.is_empty())
}

fn layout_marker_snapshot_image_key(marker_key: &str, state: &MarkerState) -> Option<String> {
    layout_marker_screenshot_path(state)
        .map(str::to_string)
        .or_else(|| {
            layout_marker_embedded_rdb_image_base64(state)
                .map(|_| format!("{LAYOUT_DRC_RDB_IMAGE_KEY_PREFIX}{marker_key}"))
        })
}

fn layout_marker_embedded_rdb_image_bytes(state: &MarkerState) -> Option<Vec<u8>> {
    if let Some(image) = layout_marker_embedded_rdb_image_base64(state)
        && let Ok(bytes) = general_purpose::STANDARD.decode(image.as_bytes())
        && !bytes.is_empty()
    {
        return Some(bytes);
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn layout_marker_screenshot_bytes(state: &MarkerState) -> Option<Vec<u8>> {
    layout_marker_screenshot_path(state)
        .and_then(|path| std::fs::read(path).ok().filter(|bytes| !bytes.is_empty()))
}

#[cfg(target_arch = "wasm32")]
fn layout_marker_screenshot_bytes(_state: &MarkerState) -> Option<Vec<u8>> {
    None
}

pub(crate) fn layout_drc_marker_snapshot_canvas_rgba(
    app: &GlassworksApp,
    image_key: &str,
    target_size: PixelSize,
) -> Option<Vec<u8>> {
    if target_size.width == 0 || target_size.height == 0 {
        return None;
    }
    let image = layout_drc_marker_snapshot_decoded_image(app, image_key)?;
    Some(layout_scale_snapshot_image_to_rgba_canvas(
        &image,
        target_size,
    ))
}

fn layout_drc_marker_snapshot_decoded_image(
    app: &GlassworksApp,
    image_key: &str,
) -> Option<LayoutDecodedImage> {
    let document = &app.workspace.document;
    if let Some(marker_key) = image_key.strip_prefix(LAYOUT_DRC_RDB_IMAGE_KEY_PREFIX) {
        let state = document.marker_states.get(marker_key)?;
        return layout_marker_embedded_rdb_image_bytes(state)
            .and_then(|bytes| layout_decode_png_rgba8(&bytes));
    }
    document
        .marker_states
        .values()
        .find(|state| layout_marker_screenshot_path(state) == Some(image_key))
        .and_then(layout_marker_screenshot_decoded_image)
}

fn layout_marker_screenshot_decoded_image(state: &MarkerState) -> Option<LayoutDecodedImage> {
    let bytes = layout_marker_screenshot_bytes(state)?;
    if layout_marker_snapshot_format(state)
        .is_some_and(|format| format.eq_ignore_ascii_case("rgba"))
    {
        return layout_decode_raw_rgba_snapshot(state, &bytes);
    }
    layout_decode_png_rgba8(&bytes)
}

fn layout_marker_snapshot_format(state: &MarkerState) -> Option<&str> {
    state
        .tags
        .get("screenshot_format")
        .map(String::as_str)
        .map(str::trim)
        .filter(|format| !format.is_empty())
}

fn layout_decode_raw_rgba_snapshot(
    state: &MarkerState,
    bytes: &[u8],
) -> Option<LayoutDecodedImage> {
    let (width, height) = layout_marker_snapshot_size(state)?;
    let expected_len = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(4)?;
    (bytes.len() == expected_len).then_some(LayoutDecodedImage {
        width,
        height,
        rgba: bytes.to_vec(),
    })
}

fn layout_marker_snapshot_size(state: &MarkerState) -> Option<(u32, u32)> {
    let value = state.tags.get("screenshot_size")?.trim();
    let (width, height) = value.split_once('x')?;
    let width = width.trim().parse::<u32>().ok()?;
    let height = height.trim().parse::<u32>().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

fn layout_scale_snapshot_image_to_rgba_canvas(
    image: &LayoutDecodedImage,
    target_size: PixelSize,
) -> Vec<u8> {
    let target_width = target_size.width as usize;
    let target_height = target_size.height as usize;
    let mut rgba = vec![0; target_width * target_height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[18, 24, 32, 255]);
    }
    let scale = ((target_size.width as f32 / image.width as f32)
        .min(target_size.height as f32 / image.height as f32))
    .max(f32::EPSILON);
    let scaled_width =
        ((image.width as f32 * scale).round() as u32).clamp(1, target_size.width) as usize;
    let scaled_height =
        ((image.height as f32 * scale).round() as u32).clamp(1, target_size.height) as usize;
    let x_offset = (target_width - scaled_width) / 2;
    let y_offset = (target_height - scaled_height) / 2;
    let source_width = image.width as usize;
    let source_height = image.height as usize;
    for y in 0..scaled_height {
        let source_y = (y * source_height / scaled_height).min(source_height - 1);
        for x in 0..scaled_width {
            let source_x = (x * source_width / scaled_width).min(source_width - 1);
            let source = (source_y * source_width + source_x) * 4;
            let target = ((y + y_offset) * target_width + x + x_offset) * 4;
            rgba[target..target + 4].copy_from_slice(&image.rgba[source..source + 4]);
        }
    }
    rgba
}

fn layout_decode_png_rgba8(bytes: &[u8]) -> Option<LayoutDecodedImage> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.width == 0 || info.height == 0 {
        return None;
    }
    let pixels = &buffer[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(pixels.len() / 3 * 4);
            for pixel in pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
            rgba
        }
        png::ColorType::Grayscale => pixels
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => {
            let mut rgba = Vec::with_capacity(pixels.len() / 2 * 4);
            for pixel in pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
            rgba
        }
        png::ColorType::Indexed => return None,
    };
    let expected_len = usize::try_from(info.width)
        .ok()?
        .checked_mul(usize::try_from(info.height).ok()?)?
        .checked_mul(4)?;
    (rgba.len() == expected_len).then_some(LayoutDecodedImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

struct LayoutDecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub(crate) fn selected_layout_drc_marker_violation(app: &GlassworksApp) -> Option<DrcViolation> {
    let selected_key = app.layout_selected_drc_marker_key.as_ref()?;
    app.drc_report()?
        .violations
        .into_iter()
        .find(|violation| violation.stable_key() == *selected_key)
}

pub(crate) fn layout_drc_marker_source_cells(
    document: &Document,
    violation: &DrcViolation,
) -> Vec<CellId> {
    let mut source_cells = BTreeSet::new();
    for occurrence in &violation.occurrence_ids {
        if let Some(view) =
            document.shape_view_for_occurrence_from_cell(document.top_cell, occurrence)
        {
            source_cells.insert(view.source_cell);
        }
    }
    for shape_id in &violation.shape_ids {
        if document.shapes.contains_key(shape_id) {
            source_cells.insert(document.top_cell);
        }
        for cell in document.cells.values() {
            if cell.shapes.contains_key(shape_id) {
                source_cells.insert(cell.id);
            }
        }
    }
    ordered_layout_cells(document)
        .into_iter()
        .map(|cell| cell.id)
        .filter(|cell_id| source_cells.contains(cell_id))
        .collect()
}

pub(crate) fn layout_drc_marker_source_cells_label(
    document: &Document,
    violation: &DrcViolation,
) -> String {
    let cells = layout_drc_marker_source_cells(document, violation);
    if cells.is_empty() {
        return "None".to_string();
    }
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
}

pub(crate) fn layout_drc_marker_source_cells_search_label(
    document: &Document,
    violation: &DrcViolation,
) -> String {
    let cells = layout_drc_marker_source_cells(document, violation);
    if cells.is_empty() {
        return "None".to_string();
    }
    cells
        .iter()
        .map(|cell_id| {
            document
                .cell(*cell_id)
                .map(|cell| format!("C{} {}", cell.id.0, cell.name))
                .unwrap_or_else(|| format!("C{}", cell_id.0))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn layout_drc_marker_source_shapes(
    document: &Document,
    violation: &DrcViolation,
) -> Vec<(ShapeId, Shape)> {
    let mut shape_ids = Vec::new();
    let mut seen = BTreeSet::new();
    for shape_id in violation.shape_ids.iter().copied().chain(
        violation
            .occurrence_ids
            .iter()
            .map(ShapeOccurrenceId::source_shape_id),
    ) {
        if seen.insert(shape_id) {
            shape_ids.push(shape_id);
        }
    }
    shape_ids
        .into_iter()
        .filter_map(|shape_id| {
            document
                .shapes
                .get(&shape_id)
                .or_else(|| {
                    document
                        .cells
                        .values()
                        .find_map(|cell| cell.shapes.get(&shape_id))
                })
                .map(|shape| (shape_id, shape))
        })
        .collect()
}

fn layout_drc_marker_source_objects_label(document: &Document, violation: &DrcViolation) -> String {
    let source_shapes = layout_drc_marker_source_shapes(document, violation);
    if source_shapes.is_empty() {
        return "None".to_string();
    }
    let mut label = source_shapes
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
    let remaining = source_shapes.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 64)
}

fn layout_drc_marker_source_objects_search_label(
    document: &Document,
    violation: &DrcViolation,
) -> String {
    let source_shapes = layout_drc_marker_source_shapes(document, violation);
    if source_shapes.is_empty() {
        return "None".to_string();
    }
    source_shapes
        .iter()
        .map(|(shape_id, shape)| {
            format!(
                "#{} shape {} {} {}",
                shape_id.0,
                shape_id.0,
                shape_kind_label(&shape.kind),
                layout_drc_marker_layer_search_label(document, shape.layer)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_drc_marker_source_layers_label(document: &Document, violation: &DrcViolation) -> String {
    let mut layer_ids = layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape.layer)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if layer_ids.is_empty() {
        return "None".to_string();
    }
    layer_ids.sort_by_key(|layer_id| {
        document
            .layer(*layer_id)
            .map(|layer| (layer.display_order, layer.id))
            .unwrap_or((i32::MAX, *layer_id))
    });
    let mut label = layer_ids
        .iter()
        .take(4)
        .map(|layer_id| layout_layer_display_name(document, *layer_id))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = layer_ids.len().saturating_sub(4);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 52)
}

fn layout_drc_marker_source_layers_search_label(
    document: &Document,
    violation: &DrcViolation,
) -> String {
    let mut layer_ids = layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape.layer)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if layer_ids.is_empty() {
        return "None".to_string();
    }
    layer_ids.sort_by_key(|layer_id| {
        document
            .layer(*layer_id)
            .map(|layer| (layer.display_order, layer.id))
            .unwrap_or((i32::MAX, *layer_id))
    });
    layer_ids
        .iter()
        .map(|layer_id| layout_drc_marker_layer_search_label(document, *layer_id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_drc_marker_layer_search_label(document: &Document, layer_id: LayerId) -> String {
    document
        .layer(layer_id)
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| format!("L{}", layer_id.0))
}

fn layout_drc_marker_source_kinds_label(document: &Document, violation: &DrcViolation) -> String {
    let mut kinds = Vec::new();
    for (_, shape) in layout_drc_marker_source_shapes(document, violation) {
        let kind = shape_kind_label(&shape.kind);
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        return "None".to_string();
    }
    kinds.join(", ")
}

fn layout_drc_marker_source_bounds_label(document: &Document, violation: &DrcViolation) -> String {
    let Some(bounds) = layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape.kind.bounds())
        .reduce(Rect::union)
    else {
        return "None".to_string();
    };
    rect_summary(bounds)
}

fn layout_drc_marker_source_objects_sort_key(
    document: &Document,
    violation: &DrcViolation,
) -> Vec<(ShapeId, &'static str, LayerId)> {
    layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(shape_id, shape)| (shape_id, shape_kind_label(&shape.kind), shape.layer))
        .collect()
}

fn layout_drc_marker_source_layers_sort_key(
    document: &Document,
    violation: &DrcViolation,
) -> Vec<(i32, LayerId)> {
    layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape.layer)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|layer_id| {
            document
                .layer(layer_id)
                .map(|layer| (layer.display_order, layer.id))
                .unwrap_or((i32::MAX, layer_id))
        })
        .collect()
}

fn layout_drc_marker_source_kinds_sort_key(
    document: &Document,
    violation: &DrcViolation,
) -> Vec<&'static str> {
    layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape_kind_label(&shape.kind))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn layout_drc_marker_source_bounds_sort_key(
    document: &Document,
    violation: &DrcViolation,
) -> Option<(Coord, Coord, Coord, Coord)> {
    layout_drc_marker_source_shapes(document, violation)
        .into_iter()
        .map(|(_, shape)| shape.kind.bounds())
        .reduce(Rect::union)
        .map(|bounds| (bounds.min.x, bounds.min.y, bounds.width(), bounds.height()))
}

pub(crate) fn layout_drc_marker_size_label(
    app: &GlassworksApp,
    violation: &DrcViolation,
) -> String {
    format!(
        "{} x {}",
        app.format_layout_length(violation.bounds.width() as f64),
        app.format_layout_length(violation.bounds.height() as f64)
    )
}

pub(crate) fn layout_drc_marker_area_label(violation: &DrcViolation) -> String {
    format!("{} dbu^2", violation.bounds.area())
}

pub(crate) fn layout_drc_marker_shape_ids_label(violation: &DrcViolation) -> String {
    if violation.shape_ids.is_empty() {
        return "None".to_string();
    }
    let mut label = violation
        .shape_ids
        .iter()
        .take(4)
        .map(|shape_id| format!("#{}", shape_id.0))
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = violation.shape_ids.len().saturating_sub(4);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 48)
}

pub(crate) fn layout_drc_marker_occurrences_label(violation: &DrcViolation) -> String {
    if violation.occurrence_ids.is_empty() {
        return "None".to_string();
    }
    let mut label = violation
        .occurrence_ids
        .iter()
        .take(3)
        .map(layout_occurrence_label)
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = violation.occurrence_ids.len().saturating_sub(3);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    compact_button_label(&label, 48)
}

pub(crate) fn layout_drc_marker_info_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let Some(report) = app.drc_report() else {
        return vec![("Status".to_string(), "Not run".to_string())];
    };
    let mut rows = layout_drc_marker_info_report_rows(app, &report);
    if !report.findings.is_empty() {
        rows.extend(layout_drc_report_diagnostic_rows(&report.findings, 3));
        if report.violations.is_empty() {
            return rows;
        }
    }
    let entries = layout_drc_marker_entries(app);
    rows.push((
        "Listed review states".to_string(),
        layout_drc_marker_review_summary_label(&app.workspace.document, &entries),
    ));
    rows.push((
        "Listed signoff audit".to_string(),
        layout_drc_marker_signoff_audit_summary_label(&app.workspace.document, &entries),
    ));
    rows.push((
        "Listed tags".to_string(),
        layout_drc_marker_tag_summary_label(&app.workspace.document, &entries),
    ));
    rows.push((
        "Listed snapshots".to_string(),
        layout_drc_marker_snapshot_summary_label(&app.workspace.document, &entries),
    ));
    let search = layout_browser_search_query_lower(app);
    if search.is_some() {
        rows.push((
            "Search".to_string(),
            layout_browser_search_display_value(app),
        ));
        rows.push(("Listed markers".to_string(), entries.len().to_string()));
    }
    let selected_key = app
        .layout_selected_drc_marker_key
        .as_ref()
        .filter(|selected| entries.iter().any(|entry| &entry.key == *selected))
        .cloned()
        .or_else(|| entries.first().map(|entry| entry.key.clone()));
    let Some(selected_key) = selected_key else {
        rows.push(("Marker".to_string(), "None".to_string()));
        return rows;
    };
    let Some(violation) = report
        .violations
        .iter()
        .find(|violation| violation.stable_key() == selected_key)
    else {
        rows.push(("Marker".to_string(), "Missing".to_string()));
        return rows;
    };
    rows.extend(layout_drc_marker_report_context_rows(app, &selected_key));
    let state = app.workspace.document.marker_states.get(&selected_key);
    let source_cell_ids = layout_drc_marker_source_cells(&app.workspace.document, violation);
    let source_object_count =
        layout_drc_marker_source_shapes(&app.workspace.document, violation).len();
    let source_cells = layout_drc_marker_source_cells_label(&app.workspace.document, violation);
    let cross_probe_targets = layout_drc_marker_cross_probe_targets_label(
        source_object_count,
        source_cell_ids.len(),
        violation.occurrence_ids.len(),
    );
    let signoff_by = layout_drc_marker_signoff_by_label(state);
    let signoff_note = layout_drc_marker_signoff_note_label(state);
    let signoff_detail = layout_drc_marker_signoff_detail_label(state);
    let signoff_records = layout_marker_signoff_records_label(state);
    rows.extend([
        ("Marker id".to_string(), format!("#{}", violation.id)),
        (
            "Directory".to_string(),
            compact_button_label(
                &layout_drc_marker_directory_path_with_state(violation, state),
                52,
            ),
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
        (
            "Center".to_string(),
            point_summary(violation.bounds.center()),
        ),
        (
            "Size".to_string(),
            layout_drc_marker_size_label(app, violation),
        ),
        ("Area".to_string(), layout_drc_marker_area_label(violation)),
        ("Shapes".to_string(), violation.shape_ids.len().to_string()),
        (
            "Shape ids".to_string(),
            layout_drc_marker_shape_ids_label(violation),
        ),
        (
            "Occurrences".to_string(),
            violation.occurrence_ids.len().to_string(),
        ),
        (
            "Occurrence labels".to_string(),
            layout_drc_marker_occurrences_label(violation),
        ),
        ("Source cells".to_string(), source_cells),
        (
            "Source objects".to_string(),
            layout_drc_marker_source_objects_label(&app.workspace.document, violation),
        ),
        (
            "Source layers".to_string(),
            layout_drc_marker_source_layers_label(&app.workspace.document, violation),
        ),
        (
            "Source kinds".to_string(),
            layout_drc_marker_source_kinds_label(&app.workspace.document, violation),
        ),
        (
            "Source bounds".to_string(),
            layout_drc_marker_source_bounds_label(&app.workspace.document, violation),
        ),
        ("Cross-probe targets".to_string(), cross_probe_targets),
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
        ("Signoff by".to_string(), signoff_by),
        ("Signoff note".to_string(), signoff_note),
        ("Signoff detail".to_string(), signoff_detail),
        ("Signoff records".to_string(), signoff_records),
        ("Tags".to_string(), layout_marker_tags_label(state)),
        ("Snapshot".to_string(), layout_marker_snapshot_label(state)),
        (
            "Snapshot bounds".to_string(),
            layout_marker_snapshot_bounds_label(state),
        ),
    ]);
    rows
}

fn layout_drc_marker_report_context_rows(
    app: &GlassworksApp,
    selected_key: &str,
) -> Vec<(String, String)> {
    let matching_reports = app
        .layout_drc_report_history
        .iter()
        .filter(|entry| {
            entry
                .value
                .violations
                .iter()
                .any(|violation| violation.stable_key() == selected_key)
        })
        .collect::<Vec<_>>();
    if matching_reports.is_empty() {
        return Vec::new();
    }
    let mut report_label = matching_reports
        .iter()
        .take(4)
        .map(|entry| {
            let selection = if app.layout_selected_drc_report_id == Some(entry.id) {
                "active"
            } else {
                "stored"
            };
            format!(
                "#{} {} {selection}",
                entry.id,
                compact_button_label(&entry.label, 18)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = matching_reports.len().saturating_sub(4);
    if remaining > 0 {
        report_label.push_str(&format!(" +{remaining}"));
    }
    let mut source_counts = BTreeMap::<String, usize>::new();
    for entry in &matching_reports {
        if let Some(source) = layout_drc_report_source(entry) {
            *source_counts.entry(source.to_string()).or_default() += 1;
        }
    }
    vec![
        (
            "Marker reports".to_string(),
            matching_reports.len().to_string(),
        ),
        (
            "Marker report labels".to_string(),
            compact_button_label(&report_label, 96),
        ),
        (
            "Marker report sources".to_string(),
            source_counts.len().to_string(),
        ),
        (
            "Marker report source labels".to_string(),
            layout_drc_report_source_count_summary_label(&source_counts),
        ),
    ]
}

pub(crate) fn layout_drc_marker_review_summary_label(
    document: &Document,
    entries: &[LayoutDrcMarkerEntry],
) -> String {
    let mut hidden = 0;
    let mut waived = 0;
    let mut visited = 0;
    let mut important = 0;
    let mut noted = 0;
    let mut owned = 0;
    let mut signed = 0;
    let mut needs_review = 0;
    let mut accepted = 0;
    let mut rejected = 0;
    let mut tagged = 0;
    let mut snapshots = 0;
    for entry in entries {
        let Some(state) = document.marker_states.get(&entry.key) else {
            continue;
        };
        hidden += usize::from(state.hidden);
        waived += usize::from(state.waived);
        visited += usize::from(state.visited);
        important += usize::from(state.important);
        noted += usize::from(
            state
                .note
                .as_deref()
                .is_some_and(|note| !note.trim().is_empty()),
        );
        owned += usize::from(
            state
                .owner
                .as_deref()
                .is_some_and(|owner| !owner.trim().is_empty()),
        );
        signed += usize::from(
            state
                .signoff
                .as_deref()
                .is_some_and(|signoff| !signoff.trim().is_empty())
                || !state.signoff_records.is_empty(),
        );
        needs_review += usize::from(layout_drc_marker_has_signoff_status(
            Some(state),
            "needs_review",
        ));
        accepted += usize::from(layout_drc_marker_has_signoff_status(
            Some(state),
            "accepted",
        ));
        rejected += usize::from(layout_drc_marker_has_signoff_status(
            Some(state),
            "rejected",
        ));
        tagged += usize::from(!state.tags.is_empty());
        snapshots += usize::from(layout_marker_state_has_snapshot(state));
    }
    let mut parts = Vec::new();
    for (label, count) in [
        ("hidden", hidden),
        ("waived", waived),
        ("visited", visited),
        ("important", important),
        ("noted", noted),
        ("owned", owned),
        ("signed", signed),
        ("needs-review", needs_review),
        ("accepted", accepted),
        ("rejected", rejected),
        ("tagged", tagged),
        ("snapshots", snapshots),
    ] {
        if count > 0 {
            parts.push(format!("{label}={count}"));
        }
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        compact_button_label(&parts.join(", "), 120)
    }
}

pub(crate) fn layout_drc_marker_signoff_audit_summary_label(
    document: &Document,
    entries: &[LayoutDrcMarkerEntry],
) -> String {
    let mut record_count = 0usize;
    let mut parties = BTreeSet::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut role_counts = BTreeMap::<String, usize>::new();
    let mut newest_stamp: Option<String> = None;
    for entry in entries {
        let Some(state) = document.marker_states.get(&entry.key) else {
            continue;
        };
        if state.signoff_records.is_empty() {
            let Some(status) = state
                .signoff
                .as_deref()
                .map(str::trim)
                .filter(|status| !status.is_empty())
            else {
                continue;
            };
            record_count += 1;
            let party = state
                .signoff_by
                .as_deref()
                .or(state.owner.as_deref())
                .map(str::trim)
                .filter(|party| !party.is_empty())
                .unwrap_or("current")
                .to_string();
            parties.insert(party.clone());
            *status_counts.entry(status.to_string()).or_default() += 1;
            let role = layout_marker_signoff_role_for_party(&party);
            *role_counts.entry(role).or_default() += 1;
            continue;
        }
        for (party, signoff) in &state.signoff_records {
            let status = signoff.status.trim();
            if status.is_empty() {
                continue;
            }
            record_count += 1;
            parties.insert(party.clone());
            *status_counts.entry(status.to_string()).or_default() += 1;
            if let Some(role) = signoff
                .role
                .as_deref()
                .map(str::trim)
                .filter(|role| !role.is_empty())
            {
                *role_counts.entry(role.to_string()).or_default() += 1;
            }
            if let Some(recorded_at) = signoff
                .recorded_at
                .as_deref()
                .map(str::trim)
                .filter(|recorded_at| !recorded_at.is_empty())
                && newest_stamp
                    .as_deref()
                    .is_none_or(|current| recorded_at > current)
            {
                newest_stamp = Some(recorded_at.to_string());
            }
        }
    }
    if record_count == 0 {
        return "None".to_string();
    }
    let mut parts = vec![
        layout_drc_marker_count_label(record_count, "signoff record", "signoff records"),
        layout_drc_marker_count_label(parties.len(), "party", "parties"),
    ];
    if !parties.is_empty() {
        parts.push(format!(
            "reviewers {}",
            layout_drc_marker_limited_set_label(&parties)
        ));
    }
    if !status_counts.is_empty() {
        parts.push(format!(
            "status {}",
            layout_drc_marker_count_map_label(&status_counts)
        ));
    }
    if !role_counts.is_empty() {
        parts.push(format!(
            "roles {}",
            layout_drc_marker_count_map_label(&role_counts)
        ));
    }
    if let Some(newest_stamp) = newest_stamp {
        parts.push(format!("newest {newest_stamp}"));
    }
    compact_button_label(&parts.join("; "), 180)
}

pub(crate) fn layout_drc_marker_tag_summary_label(
    document: &Document,
    entries: &[LayoutDrcMarkerEntry],
) -> String {
    let mut tagged_marker_count = 0usize;
    let mut key_counts = BTreeMap::<String, usize>::new();
    let mut tag_counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        let Some(state) = document.marker_states.get(&entry.key) else {
            continue;
        };
        if state.tags.is_empty() {
            continue;
        }
        tagged_marker_count += 1;
        for (key, value) in &state.tags {
            if key.trim().is_empty() || value.trim().is_empty() {
                continue;
            }
            *key_counts.entry(key.clone()).or_default() += 1;
            *tag_counts.entry(format!("{key}={value}")).or_default() += 1;
        }
    }
    if tagged_marker_count == 0 {
        return "None".to_string();
    }
    let mut parts = vec![
        layout_drc_marker_count_label(tagged_marker_count, "tagged marker", "tagged markers"),
        layout_drc_marker_count_label(key_counts.len(), "key", "keys"),
    ];
    if !key_counts.is_empty() {
        parts.push(format!(
            "keys {}",
            layout_drc_marker_count_map_label(&key_counts)
        ));
    }
    if !tag_counts.is_empty() {
        parts.push(format!(
            "tags {}",
            layout_drc_marker_tag_count_map_label(&tag_counts)
        ));
    }
    compact_button_label(&parts.join("; "), 140)
}

pub(crate) fn layout_drc_marker_snapshot_summary_label(
    document: &Document,
    entries: &[LayoutDrcMarkerEntry],
) -> String {
    let mut snapshot_count = 0usize;
    let mut format_counts = BTreeMap::<String, usize>::new();
    let mut size_counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        let Some(state) = document.marker_states.get(&entry.key) else {
            continue;
        };
        if !layout_marker_state_has_snapshot(state) {
            continue;
        }
        snapshot_count += 1;
        if let Some(format) = state
            .tags
            .get("screenshot_format")
            .map(String::as_str)
            .map(str::trim)
            .filter(|format| !format.is_empty())
        {
            *format_counts.entry(format.to_string()).or_default() += 1;
        } else if layout_marker_embedded_rdb_image_base64(state).is_some() {
            *format_counts.entry("embedded-rdb".to_string()).or_default() += 1;
        }
        if let Some(size) = state
            .tags
            .get("screenshot_size")
            .map(String::as_str)
            .map(str::trim)
            .filter(|size| !size.is_empty())
        {
            *size_counts.entry(size.to_string()).or_default() += 1;
        }
    }
    if snapshot_count == 0 {
        return "None".to_string();
    }
    let mut parts = vec![layout_drc_marker_count_label(
        snapshot_count,
        "snapshot",
        "snapshots",
    )];
    if !format_counts.is_empty() {
        parts.push(format!(
            "formats {}",
            layout_drc_marker_count_map_label(&format_counts)
        ));
    }
    if !size_counts.is_empty() {
        parts.push(format!(
            "sizes {}",
            layout_drc_marker_count_map_label(&size_counts)
        ));
    }
    compact_button_label(&parts.join("; "), 120)
}

fn layout_drc_marker_limited_set_label(values: &BTreeSet<String>) -> String {
    let mut label = values
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = values.len().saturating_sub(4);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    label
}

fn layout_drc_marker_count_map_label(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_drc_marker_tag_count_map_label(counts: &BTreeMap<String, usize>) -> String {
    let mut label = counts
        .iter()
        .take(4)
        .map(|(tag, count)| {
            if *count == 1 {
                tag.clone()
            } else {
                format!("{tag} x{count}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = counts.len().saturating_sub(4);
    if remaining > 0 {
        label.push_str(&format!(" +{remaining}"));
    }
    label
}

fn layout_drc_marker_info_report_rows(
    app: &GlassworksApp,
    report: &DrcReportCacheValue,
) -> Vec<(String, String)> {
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
        .and_then(layout_drc_report_source)
        .map(|source| compact_button_label(source, 64))
        .unwrap_or_else(|| "None".to_string());
    let report_label = app
        .selected_drc_report_history_entry()
        .map(|entry| {
            let status = if entry.revision == app.layout_revision {
                "current"
            } else {
                "stale"
            };
            format!(
                "#{} {} ({status})",
                entry.id,
                compact_button_label(&entry.label, 32)
            )
        })
        .unwrap_or_else(|| "Current cache".to_string());
    let mut rows = vec![
        (
            "Report database".to_string(),
            layout_drc_marker_report_database_label(app.layout_drc_report_history.len()),
        ),
        ("Report".to_string(), report_label),
        (
            "Report markers".to_string(),
            report.violations.len().to_string(),
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
    ];
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
    let remaining = app.layout_drc_report_history.len().saturating_sub(4);
    if remaining > 0 {
        rows.push(("More reports".to_string(), remaining.to_string()));
    }
    rows
}

fn layout_drc_marker_report_database_label(report_count: usize) -> String {
    if report_count == 1 {
        "1 report".to_string()
    } else {
        format!("{report_count} reports")
    }
}

pub(crate) fn layout_drc_marker_report_history_entry_label(
    entry: &DrcReportHistoryEntry,
    layout_revision: u64,
    selected_report_id: Option<u64>,
) -> String {
    let summary = layout_drc_report_summary_label(entry);
    let freshness = if entry.revision == layout_revision {
        "current"
    } else {
        "stale"
    };
    let selection = if selected_report_id == Some(entry.id) {
        "active"
    } else {
        "stored"
    };
    let source = entry
        .source
        .as_deref()
        .filter(|source| !source.trim().is_empty())
        .map(|source| format!(" - source {}", compact_button_label(source, 24)))
        .unwrap_or_default();
    compact_button_label(
        &format!(
            "{} - {summary} - {freshness} - {selection}{source}",
            entry.label
        ),
        64,
    )
}

pub(crate) fn layout_drc_marker_cross_probe_targets_label(
    shape_count: usize,
    source_cell_count: usize,
    occurrence_count: usize,
) -> String {
    format!(
        "{} / {} / {}",
        layout_drc_marker_count_label(shape_count, "shape", "shapes"),
        layout_drc_marker_count_label(source_cell_count, "cell", "cells"),
        layout_drc_marker_count_label(occurrence_count, "occurrence", "occurrences")
    )
}

fn layout_drc_marker_count_label(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
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
    let category_label = layout_drc_marker_category_label(violation, state);
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
    let signoff_by_label = layout_drc_marker_signoff_by_label(state);
    let signoff_note_label = layout_drc_marker_signoff_note_label(state);
    let signoff_detail_label = layout_drc_marker_signoff_detail_label(state);
    let signoff_records_label = layout_marker_signoff_records_label(state);
    let tags_label = layout_marker_tags_label(state);
    let snapshot_label = layout_marker_snapshot_label(state);
    let snapshot_bounds_label = layout_marker_snapshot_bounds_label(state);
    let center = point_summary(violation.bounds.center());
    let size = layout_drc_marker_size_label(app, violation);
    let area = layout_drc_marker_area_label(violation);
    let shape_ids = layout_drc_marker_shape_ids_label(violation);
    let occurrence_labels = layout_drc_marker_occurrences_label(violation);
    let source_cells = layout_drc_marker_source_cells_label(document, violation);
    let source_objects = layout_drc_marker_source_objects_label(document, violation);
    let source_layers = layout_drc_marker_source_layers_label(document, violation);
    let source_kinds = layout_drc_marker_source_kinds_label(document, violation);
    let source_bounds = layout_drc_marker_source_bounds_label(document, violation);
    let source_cell_count = layout_drc_marker_source_cells(document, violation).len();
    let source_object_count = layout_drc_marker_source_shapes(document, violation).len();
    let cross_probe_targets = layout_drc_marker_cross_probe_targets_label(
        source_object_count,
        source_cell_count,
        violation.occurrence_ids.len(),
    );
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
            ("Signoff by".to_string(), signoff_by_label),
            ("Signoff note".to_string(), signoff_note_label),
            ("Signoff records".to_string(), signoff_records_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
            ("Source cells".to_string(), source_cells),
            ("Source objects".to_string(), source_objects),
            ("Source layers".to_string(), source_layers),
            ("Source kinds".to_string(), source_kinds),
        ],
        LayoutBrowserColumnSet::Geometry => vec![
            ("Marker id".to_string(), format!("#{}", violation.id)),
            ("Bounds".to_string(), rect_summary(violation.bounds)),
            ("Source bounds".to_string(), source_bounds),
            ("Center".to_string(), center),
            ("Size".to_string(), size),
            ("Area".to_string(), area),
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
            ("Signoff by".to_string(), signoff_by_label),
            ("Signoff note".to_string(), signoff_note_label),
            ("Signoff records".to_string(), signoff_records_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
            ("Shapes".to_string(), violation.shape_ids.len().to_string()),
            ("Shape ids".to_string(), shape_ids),
            ("Occurrence labels".to_string(), occurrence_labels),
            ("Source cells".to_string(), source_cells),
            ("Source objects".to_string(), source_objects),
            ("Source layers".to_string(), source_layers),
            ("Source kinds".to_string(), source_kinds),
            ("Source bounds".to_string(), source_bounds),
            ("Cross-probe targets".to_string(), cross_probe_targets),
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
            ("Signoff by".to_string(), signoff_by_label),
            ("Signoff note".to_string(), signoff_note_label),
            ("Signoff detail".to_string(), signoff_detail_label),
            ("Signoff records".to_string(), signoff_records_label),
            ("Tags".to_string(), tags_label),
            ("Snapshot".to_string(), snapshot_label),
            ("Snapshot bounds".to_string(), snapshot_bounds_label),
            ("Shapes".to_string(), violation.shape_ids.len().to_string()),
            ("Shape ids".to_string(), shape_ids),
            ("Occurrence labels".to_string(), occurrence_labels),
            ("Source cells".to_string(), source_cells),
            ("Source objects".to_string(), source_objects),
            ("Source layers".to_string(), source_layers),
            ("Source kinds".to_string(), source_kinds),
            ("Source bounds".to_string(), source_bounds),
            ("Cross-probe targets".to_string(), cross_probe_targets),
            ("Bounds".to_string(), rect_summary(violation.bounds)),
            ("Center".to_string(), center),
            ("Size".to_string(), size),
            ("Area".to_string(), area),
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
