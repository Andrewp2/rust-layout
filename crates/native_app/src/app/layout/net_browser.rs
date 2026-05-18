#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_net_browser_entries(app: &GlassworksApp) -> Vec<(usize, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    app.connectivity_report()
        .map(|report| {
            layout_net_browser_entries_from_report_with_history(
                &report,
                app.layout_net_browser_filter,
                app.layout_net_browser_sort,
                layout_browser_search_query_lower(app).as_deref(),
                &app.layout_trace_history,
                app.layout_spice_comparison.as_ref(),
            )
        })
        .unwrap_or_default()
}

pub(crate) fn layout_net_browser_entries_from_report(
    report: &ConnectivityReport,
    filter: LayoutNetBrowserFilter,
    sort: LayoutNetBrowserSort,
    search: Option<&str>,
) -> Vec<(usize, String)> {
    layout_net_browser_entries_from_report_with_history(report, filter, sort, search, &[], None)
}

pub(crate) fn layout_net_browser_entries_from_report_with_history(
    report: &ConnectivityReport,
    filter: LayoutNetBrowserFilter,
    sort: LayoutNetBrowserSort,
    search: Option<&str>,
    trace_history: &[usize],
    spice_comparison: Option<&SpiceConnectivityComparisonReport>,
) -> Vec<(usize, String)> {
    let trace_history = (filter == LayoutNetBrowserFilter::History)
        .then(|| trace_history.iter().copied().collect::<BTreeSet<_>>());
    let mut components = report.components.iter().collect::<Vec<_>>();
    components.sort_by(|left, right| match sort {
        LayoutNetBrowserSort::Name => (!connectivity_component_is_labeled(left))
            .cmp(&!connectivity_component_is_labeled(right))
            .then_with(|| {
                connectivity_component_display_name(left)
                    .cmp(&connectivity_component_display_name(right))
            })
            .then_with(|| left.id.cmp(&right.id)),
        LayoutNetBrowserSort::Size => right
            .shapes
            .len()
            .cmp(&left.shapes.len())
            .then_with(|| left.id.cmp(&right.id)),
        LayoutNetBrowserSort::Id => left.id.cmp(&right.id),
    });
    components
        .into_iter()
        .filter(|component| match filter {
            LayoutNetBrowserFilter::All => true,
            LayoutNetBrowserFilter::Labeled => connectivity_component_is_labeled(component),
            LayoutNetBrowserFilter::Unlabeled => !connectivity_component_is_labeled(component),
            LayoutNetBrowserFilter::Devices => {
                layout_net_component_device_count(report, component.id) > 0
            }
            LayoutNetBrowserFilter::History => trace_history
                .as_ref()
                .is_some_and(|history| history.contains(&component.id)),
            LayoutNetBrowserFilter::SpiceExtra => {
                layout_net_component_is_spice_extra(component, spice_comparison)
            }
            LayoutNetBrowserFilter::Shorted => layout_net_component_has_short(report, component.id),
            LayoutNetBrowserFilter::Open => layout_net_component_has_open(report, component.id),
        })
        .filter(|component| {
            search.is_none_or(|query| layout_net_component_matches_search(component, query))
        })
        .take(MAX_LAYOUT_NET_BROWSER_COMPONENTS)
        .map(|component| {
            (
                component.id,
                compact_button_label(&connectivity_component_label(component), 30),
            )
        })
        .collect()
}

pub(crate) fn layout_net_component_matches_search(
    component: &NetComponent,
    query_lower: &str,
) -> bool {
    let mut fields = vec![
        connectivity_component_label(component),
        connectivity_component_display_name(component),
        format!("component {}", component.id),
        format!("#{}", component.id),
        format!("{} shapes", component.shapes.len()),
    ];
    if let Some(name) = &component.net_name {
        push_layout_search_field(&mut fields, name.clone());
    }
    if let Some(net_id) = component.net_id {
        push_layout_search_field(&mut fields, format!("net {}", net_id.0));
        push_layout_search_field(&mut fields, format!("N{}", net_id.0));
    }
    for explicit_net in &component.explicit_nets {
        push_layout_search_field(&mut fields, format!("net {}", explicit_net.0));
        push_layout_search_field(&mut fields, format!("N{}", explicit_net.0));
    }
    for label in &component.labels {
        push_layout_search_field(&mut fields, label.text.clone());
    }
    layout_search_matches_any(query_lower, &fields)
}

pub(crate) fn layout_connectivity_component_at_point(
    document: &Document,
    report: &ConnectivityReport,
    point: Point,
) -> Option<usize> {
    report.components.iter().find_map(|component| {
        if !component.bounds.expanded(1).contains_point(point) {
            return None;
        }
        component.shapes.iter().find_map(|occurrence| {
            document
                .shape_view_for_occurrence(occurrence)
                .filter(|view| view.bounds.expanded(1).contains_point(point))
                .map(|_| component.id)
        })
    })
}

pub(crate) fn connectivity_component_is_labeled(component: &NetComponent) -> bool {
    component.net_name.is_some() || component.net_id.is_some()
}

pub(crate) fn layout_net_component_is_spice_extra(
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    let Some(net_name) = component.net_name.as_deref() else {
        return false;
    };
    comparison.is_some_and(|comparison| {
        comparison
            .extra_layout_nets
            .iter()
            .any(|name| name.eq_ignore_ascii_case(net_name))
    })
}

pub(crate) fn layout_net_component_has_short(
    report: &ConnectivityReport,
    component_id: usize,
) -> bool {
    report
        .shorts
        .iter()
        .any(|short| short.component == component_id)
}

pub(crate) fn layout_net_component_has_open(
    report: &ConnectivityReport,
    component_id: usize,
) -> bool {
    report
        .opens
        .iter()
        .any(|open| open.components.contains(&component_id))
}

pub(crate) fn layout_net_component_device_count(
    report: &ConnectivityReport,
    component_id: usize,
) -> usize {
    report
        .devices
        .iter()
        .filter(|device| {
            device
                .terminals
                .iter()
                .any(|terminal| terminal.component == Some(component_id))
        })
        .count()
}

pub(crate) fn layout_net_component_device_summary(
    report: &ConnectivityReport,
    component_id: usize,
) -> String {
    let mut entries = report
        .devices
        .iter()
        .filter_map(|device| {
            let mut terminals = device
                .terminals
                .iter()
                .filter(|terminal| terminal.component == Some(component_id))
                .map(|terminal| terminal.name.clone())
                .collect::<Vec<_>>();
            if terminals.is_empty() {
                return None;
            }
            terminals.sort();
            terminals.dedup();
            let model = if device.model.trim().is_empty() {
                device.kind.as_str()
            } else {
                device.model.as_str()
            };
            Some(format!("#{} {} {}", device.id, model, terminals.join("/")))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return "None".to_string();
    }
    entries.sort();
    let mut preview = entries
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if entries.len() > 3 {
        preview.push_str(&format!(" +{}", entries.len() - 3));
    }
    compact_button_label(&preview, 64)
}

pub(crate) fn layout_net_component_issue_label(
    report: &ConnectivityReport,
    component_id: usize,
) -> String {
    let short_count = report
        .shorts
        .iter()
        .filter(|short| short.component == component_id)
        .count();
    let open_count = report
        .opens
        .iter()
        .filter(|open| open.components.contains(&component_id))
        .count();
    match (short_count, open_count) {
        (0, 0) => "None".to_string(),
        (shorts, 0) => format!("{shorts} short{}", if shorts == 1 { "" } else { "s" }),
        (0, opens) => format!("{opens} open{}", if opens == 1 { "" } else { "s" }),
        (shorts, opens) => format!(
            "{shorts} short{} / {opens} open{}",
            if shorts == 1 { "" } else { "s" },
            if opens == 1 { "" } else { "s" }
        ),
    }
}

pub(crate) fn layout_net_browser_property_rows(
    app: &GlassworksApp,
    report: &ConnectivityReport,
    entries: &[(usize, String)],
) -> Vec<(String, String)> {
    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| report.component_for_occurrence(occurrence))
        .filter(|component_id| entries.iter().any(|(id, _)| id == component_id))
        .or_else(|| entries.first().map(|(id, _)| *id));
    let Some(component_id) = selected_component else {
        return Vec::new();
    };
    let Some(component) = report.component(component_id) else {
        return Vec::new();
    };
    let issue_label = layout_net_component_issue_label(report, component_id);
    let device_count = layout_net_component_device_count(report, component_id);
    let device_summary =
        (device_count > 0).then(|| layout_net_component_device_summary(report, component_id));
    let spice_status =
        layout_spice_component_status_label(component, app.layout_spice_comparison.as_ref());
    match app.layout_browser_columns {
        LayoutBrowserColumnSet::Summary => {
            let mut rows = vec![
                ("Component id".to_string(), component.id.to_string()),
                (
                    "Display name".to_string(),
                    connectivity_component_display_name(component),
                ),
                ("Shapes".to_string(), component.shapes.len().to_string()),
                ("Devices".to_string(), device_count.to_string()),
                ("Issues".to_string(), issue_label),
            ];
            if let Some(summary) = device_summary.clone() {
                rows.push(("Device Detail".to_string(), summary));
            }
            if let Some(status) = spice_status.clone() {
                rows.push(("SPICE Net".to_string(), status));
            }
            rows
        }
        LayoutBrowserColumnSet::Geometry => vec![
            ("Component id".to_string(), component.id.to_string()),
            ("Shapes".to_string(), component.shapes.len().to_string()),
            ("Bounds".to_string(), rect_summary(component.bounds)),
        ],
        LayoutBrowserColumnSet::Relations => {
            let mut rows = vec![
                ("Component id".to_string(), component.id.to_string()),
                (
                    "Display name".to_string(),
                    connectivity_component_display_name(component),
                ),
                ("Labels".to_string(), component.labels.len().to_string()),
                (
                    "Explicit nets".to_string(),
                    component.explicit_nets.len().to_string(),
                ),
                ("Devices".to_string(), device_count.to_string()),
                (
                    "Labeled".to_string(),
                    connectivity_component_is_labeled(component).to_string(),
                ),
                ("Issues".to_string(), issue_label),
            ];
            if let Some(summary) = device_summary.clone() {
                rows.push(("Device Detail".to_string(), summary));
            }
            if let Some(status) = spice_status.clone() {
                rows.push(("SPICE Net".to_string(), status));
            }
            rows
        }
        LayoutBrowserColumnSet::All => {
            let mut rows = vec![
                ("Component id".to_string(), component.id.to_string()),
                (
                    "Display name".to_string(),
                    connectivity_component_display_name(component),
                ),
                ("Shapes".to_string(), component.shapes.len().to_string()),
                ("Labels".to_string(), component.labels.len().to_string()),
                (
                    "Explicit nets".to_string(),
                    component.explicit_nets.len().to_string(),
                ),
                ("Devices".to_string(), device_count.to_string()),
                (
                    "Labeled".to_string(),
                    connectivity_component_is_labeled(component).to_string(),
                ),
                ("Issues".to_string(), issue_label),
                ("Bounds".to_string(), rect_summary(component.bounds)),
            ];
            if let Some(summary) = device_summary {
                rows.push(("Device Detail".to_string(), summary));
            }
            if let Some(status) = spice_status {
                rows.push(("SPICE Net".to_string(), status));
            }
            rows
        }
    }
}

pub(crate) fn layout_spice_component_status_label(
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> Option<String> {
    let comparison = comparison?;
    let net_name = component.net_name.as_deref()?;
    if comparison
        .extra_layout_nets
        .iter()
        .any(|name| name.eq_ignore_ascii_case(net_name))
    {
        return Some("Extra layout net".to_string());
    }
    if comparison
        .schematic_referenced_nets
        .iter()
        .any(|name| name.eq_ignore_ascii_case(net_name))
    {
        return Some("Matched schematic net".to_string());
    }
    Some("Unmatched layout net".to_string())
}

pub(crate) fn layout_trace_history_entries(app: &GlassworksApp) -> Vec<(usize, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let search = layout_browser_search_query_lower(app);
    app.layout_trace_history
        .iter()
        .filter_map(|component_id| {
            let component = report.component(*component_id)?;
            if search
                .as_deref()
                .is_some_and(|query| !layout_net_component_matches_search(component, query))
            {
                return None;
            }
            Some((
                *component_id,
                compact_button_label(
                    &format!(
                        "#{} {}",
                        component_id,
                        connectivity_component_label(component)
                    ),
                    30,
                ),
            ))
        })
        .collect()
}
