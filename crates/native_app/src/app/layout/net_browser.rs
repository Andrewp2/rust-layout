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

pub(crate) fn layout_net_browser_filter_button_label(
    report: Option<&ConnectivityReport>,
    filter: LayoutNetBrowserFilter,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> String {
    let Some(report) = report else {
        return filter.label().to_string();
    };
    let Some(comparison) = comparison else {
        return filter.label().to_string();
    };
    let count = match filter {
        LayoutNetBrowserFilter::SpiceMissing => {
            layout_spice_missing_issue_net_count(report, comparison)
        }
        LayoutNetBrowserFilter::SpiceExtra => {
            layout_spice_extra_issue_net_count(report, comparison)
        }
        LayoutNetBrowserFilter::SpiceNets => layout_spice_net_issue_net_count(report, comparison),
        LayoutNetBrowserFilter::SpiceDevices => {
            layout_spice_device_issue_net_count(report, comparison)
        }
        LayoutNetBrowserFilter::SpiceIssues => layout_spice_issue_net_count(report, comparison),
        _ => return filter.label().to_string(),
    };
    format!("{} ({count})", filter.label())
}

pub(crate) fn layout_netlist_summary_rows(
    report: &ConnectivityReport,
) -> Vec<(&'static str, &'static str, String)> {
    let labeled_count = report
        .components
        .iter()
        .filter(|component| connectivity_component_is_labeled(component))
        .count();
    let shape_count = report
        .components
        .iter()
        .map(|component| component.shapes.len())
        .sum::<usize>();
    vec![
        (
            "components",
            "Components",
            report.components.len().to_string(),
        ),
        ("shapes", "Shapes", shape_count.to_string()),
        ("labeled", "Labeled", labeled_count.to_string()),
        (
            "unlabeled",
            "Unlabeled",
            report
                .components
                .len()
                .saturating_sub(labeled_count)
                .to_string(),
        ),
        ("devices", "Devices", report.devices.len().to_string()),
        (
            "issues",
            "Issues",
            format!(
                "{} short / {} open",
                report.shorts.len(),
                report.opens.len()
            ),
        ),
    ]
}

pub(crate) fn layout_spice_comparison_summary_rows(
    app: &GlassworksApp,
    report: Option<&ConnectivityReport>,
) -> Vec<(&'static str, &'static str, String)> {
    let Some(comparison) = app.layout_spice_comparison.as_ref() else {
        return Vec::new();
    };
    let mut rows = vec![
        ("status", "Status", comparison.status.label().to_string()),
        (
            "circuit",
            "Circuit",
            compact_button_label(&comparison.circuit_name, 40),
        ),
        (
            "nets",
            "Nets",
            format!(
                "{} schematic / {} layout",
                comparison.schematic_referenced_nets.len(),
                comparison.layout_named_nets.len()
            ),
        ),
        (
            "devices",
            "Devices",
            format!(
                "{} schematic / {} layout",
                comparison.schematic_device_count, comparison.layout_device_count
            ),
        ),
        (
            "net_mismatches",
            "Net mismatches",
            format!(
                "{} missing / {} extra",
                comparison.missing_layout_nets.len(),
                comparison.extra_layout_nets.len()
            ),
        ),
        (
            "device_mismatches",
            "Device mismatches",
            format!(
                "{} missing / {} extra",
                comparison.missing_layout_devices.len(),
                comparison.extra_layout_devices.len()
            ),
        ),
        (
            "layout_issues",
            "Layout issues",
            comparison.layout_issue_count().to_string(),
        ),
    ];
    if let Some(report) = report {
        rows.push((
            "missing_issue_nets",
            "Missing issue nets",
            layout_spice_missing_issue_net_count(report, comparison).to_string(),
        ));
        rows.push((
            "extra_issue_nets",
            "Extra issue nets",
            layout_spice_extra_issue_net_count(report, comparison).to_string(),
        ));
        rows.push((
            "net_issue_nets",
            "Net issue nets",
            layout_spice_net_issue_net_count(report, comparison).to_string(),
        ));
        rows.push((
            "device_issue_nets",
            "Device issue nets",
            layout_spice_device_issue_net_count(report, comparison).to_string(),
        ));
        rows.push((
            "issue_nets",
            "Issue nets",
            layout_spice_issue_net_count(report, comparison).to_string(),
        ));
    }
    rows
}

pub(crate) fn layout_connectivity_source_summary_rows(
    app: &GlassworksApp,
) -> Vec<(&'static str, &'static str, String)> {
    let Some(source) = app
        .connectivity_report_cache
        .borrow()
        .as_ref()
        .and_then(|cache| cache.source.clone())
    else {
        return Vec::new();
    };
    if source.label == "Extracted" {
        return Vec::new();
    }
    vec![
        ("source", "Connectivity source", source.label),
        (
            "document",
            "Connectivity document",
            compact_button_label(&source.document_name, 40),
        ),
        (
            "revision",
            "Connectivity revision",
            source.layout_revision.to_string(),
        ),
        (
            "technology",
            "Connectivity technology",
            compact_button_label(&source.technology_name, 40),
        ),
    ]
}

pub(crate) fn layout_trace_state_summary_rows(
    app: &GlassworksApp,
    report: &ConnectivityReport,
) -> Vec<(&'static str, &'static str, String)> {
    let selected_component = selected_layout_net_component_id(app, report);
    if selected_component.is_none()
        && app.layout_trace_history.is_empty()
        && app.route_points.is_empty()
    {
        return Vec::new();
    }

    let selected = selected_component
        .and_then(|component_id| {
            report.component(component_id).map(|component| {
                compact_button_label(
                    &format!(
                        "#{component_id} {}",
                        connectivity_component_display_name(component)
                    ),
                    40,
                )
            })
        })
        .unwrap_or_else(|| "None".to_string());
    let search = layout_browser_search_query_lower(app);
    let listed_history = app
        .layout_trace_history
        .iter()
        .enumerate()
        .filter(|(history_index, component_id)| {
            let Some(component) = report.component(**component_id) else {
                return false;
            };
            search.as_deref().is_none_or(|query| {
                layout_trace_history_entry_matches_search(report, *history_index, component, query)
            })
        })
        .count();
    let history = if search.is_some() {
        format!(
            "{listed_history} listed / {} total",
            app.layout_trace_history.len()
        )
    } else {
        format!("{} total", app.layout_trace_history.len())
    };
    let route_points = if app.route_points.is_empty() {
        "None".to_string()
    } else {
        format!(
            "{} point{} / {} segment{}",
            app.route_points.len(),
            if app.route_points.len() == 1 { "" } else { "s" },
            app.route_points.len().saturating_sub(1),
            if app.route_points.len().saturating_sub(1) == 1 {
                ""
            } else {
                "s"
            }
        )
    };
    vec![
        ("selected", "Trace selected", selected),
        ("history", "Trace history", history),
        ("route_points", "Trace route points", route_points),
        (
            "highlight",
            "Trace highlight",
            app.layout_trace_highlight_mode.label().to_string(),
        ),
    ]
}

pub(crate) fn layout_netlist_device_entries(
    report: &ConnectivityReport,
    search: Option<&str>,
) -> Vec<(usize, String)> {
    report
        .devices
        .iter()
        .filter(|device| {
            search.is_none_or(|query| layout_netlist_device_matches_search(report, device, query))
        })
        .map(|device| {
            (
                device.id,
                compact_button_label(&layout_netlist_device_text(report, device), 30),
            )
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_netlist_device_text(
    report: &ConnectivityReport,
    device: &ExtractedDevice,
) -> String {
    let model = if device.model.trim().is_empty() {
        device.kind.as_str()
    } else {
        device.model.as_str()
    };
    let mut terminals = device
        .terminals
        .iter()
        .map(|terminal| {
            let net = terminal
                .component
                .and_then(|component_id| report.component(component_id))
                .map(connectivity_component_display_name)
                .or_else(|| terminal.net_name.clone())
                .unwrap_or_else(|| "unconnected".to_string());
            format!("{}={net}", terminal.name)
        })
        .collect::<Vec<_>>();
    terminals.sort();
    terminals.dedup();
    if terminals.is_empty() {
        return format!("#{} {model}", device.id);
    }
    format!("#{} {} {}", device.id, model, terminals.join(", "))
}

pub(crate) fn layout_netlist_device_matches_search(
    report: &ConnectivityReport,
    device: &ExtractedDevice,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    let mut fields = vec![
        "device".to_string(),
        format!("device {}", device.id),
        format!("device #{}", device.id),
        device.kind.clone(),
        device.model.clone(),
        layout_netlist_device_text(report, device),
    ];
    for terminal in &device.terminals {
        push_layout_search_field(&mut fields, terminal.name.clone());
        push_layout_search_field(&mut fields, format!("terminal {}", terminal.name));
        if let Some(component_id) = terminal.component {
            push_layout_search_field(&mut fields, format!("component {component_id}"));
            push_layout_search_field(&mut fields, format!("#{component_id}"));
            if let Some(component) = report.component(component_id) {
                push_layout_search_field(
                    &mut fields,
                    connectivity_component_display_name(component),
                );
            }
        }
        if let Some(net_name) = terminal.net_name.as_ref() {
            push_layout_search_field(&mut fields, net_name.clone());
        }
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_netlist_issue_entries(
    report: &ConnectivityReport,
    search: Option<&str>,
) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for (index, short) in report.shorts.iter().enumerate() {
        if search.is_some_and(|query| !layout_netlist_short_issue_matches_search(short, query)) {
            continue;
        }
        entries.push((
            format!("short.{index}"),
            compact_button_label(&layout_netlist_short_issue_text(short), 30),
        ));
    }
    for (index, open) in report.opens.iter().enumerate() {
        if search.is_some_and(|query| !layout_netlist_open_issue_matches_search(open, query)) {
            continue;
        }
        entries.push((
            format!("open.{index}"),
            compact_button_label(&layout_netlist_open_issue_text(open), 30),
        ));
    }
    entries.into_iter().take(8).collect()
}

pub(crate) fn layout_netlist_short_issue_text(short: &NetShort) -> String {
    let mut names = short.names.clone();
    names.sort();
    format!("Short {} #{}", names.join("/"), short.component)
}

pub(crate) fn layout_netlist_open_issue_text(open: &NetOpen) -> String {
    let mut components = open.components.clone();
    components.sort();
    let component_text = components
        .iter()
        .take(3)
        .map(|component_id| format!("#{component_id}"))
        .collect::<Vec<_>>()
        .join("/");
    if components.len() > 3 {
        format!(
            "Open {} {} +{}",
            open.name,
            component_text,
            components.len() - 3
        )
    } else {
        format!("Open {} {}", open.name, component_text)
    }
}

pub(crate) fn layout_netlist_short_issue_matches_search(
    short: &NetShort,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    let mut names = short.names.clone();
    names.sort();
    let mut fields = vec![
        "short".to_string(),
        format!("short {}", names.join(" ")),
        format!("short {}", names.join("/")),
        layout_netlist_short_issue_text(short),
        format!("component {}", short.component),
        format!("#{}", short.component),
    ];
    for name in &names {
        push_layout_search_field(&mut fields, name.clone());
        push_layout_search_field(&mut fields, format!("short {name}"));
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_netlist_open_issue_matches_search(open: &NetOpen, query_lower: &str) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    let mut fields = vec![
        "open".to_string(),
        open.name.clone(),
        format!("open {}", open.name),
        layout_netlist_open_issue_text(open),
    ];
    for component_id in &open.components {
        push_layout_search_field(&mut fields, format!("component {component_id}"));
        push_layout_search_field(&mut fields, format!("#{component_id}"));
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_spice_comparison_issue_entries_for_filter(
    comparison: &SpiceConnectivityComparisonReport,
    search: Option<&str>,
    filter: LayoutNetBrowserFilter,
) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    if filter == LayoutNetBrowserFilter::SpiceMissing {
        push_layout_spice_comparison_device_issue_entries(
            &mut entries,
            search,
            "missing_device",
            &comparison.missing_layout_devices,
        );
        push_layout_spice_comparison_net_issue_entries(
            &mut entries,
            search,
            "missing_net",
            &comparison.missing_layout_nets,
        );
        return entries.into_iter().take(8).collect();
    }
    if filter == LayoutNetBrowserFilter::SpiceNets {
        push_layout_spice_comparison_net_issue_entries(
            &mut entries,
            search,
            "extra_net",
            &comparison.extra_layout_nets,
        );
        push_layout_spice_comparison_net_issue_entries(
            &mut entries,
            search,
            "missing_net",
            &comparison.missing_layout_nets,
        );
        return entries.into_iter().take(8).collect();
    }
    if layout_spice_comparison_issue_kind_visible(filter, "missing_net") {
        push_layout_spice_comparison_net_issue_entries(
            &mut entries,
            search,
            "missing_net",
            &comparison.missing_layout_nets,
        );
    }
    if layout_spice_comparison_issue_kind_visible(filter, "extra_net") {
        push_layout_spice_comparison_net_issue_entries(
            &mut entries,
            search,
            "extra_net",
            &comparison.extra_layout_nets,
        );
    }
    if layout_spice_comparison_issue_kind_visible(filter, "missing_device") {
        push_layout_spice_comparison_device_issue_entries(
            &mut entries,
            search,
            "missing_device",
            &comparison.missing_layout_devices,
        );
    }
    if layout_spice_comparison_issue_kind_visible(filter, "extra_device") {
        push_layout_spice_comparison_device_issue_entries(
            &mut entries,
            search,
            "extra_device",
            &comparison.extra_layout_devices,
        );
    }
    entries.into_iter().take(8).collect()
}

fn push_layout_spice_comparison_net_issue_entries(
    entries: &mut Vec<(String, String)>,
    search: Option<&str>,
    kind: &'static str,
    net_names: &[String],
) {
    for (index, net_name) in net_names.iter().enumerate() {
        let issue = layout_spice_comparison_issue_text(kind, net_name);
        if search.is_some_and(|query| {
            !layout_spice_comparison_issue_matches_search(kind, net_name, query)
        }) {
            continue;
        }
        entries.push((format!("{kind}.{index}"), compact_button_label(&issue, 30)));
    }
}

fn push_layout_spice_comparison_device_issue_entries(
    entries: &mut Vec<(String, String)>,
    search: Option<&str>,
    kind: &'static str,
    signatures: &[String],
) {
    for (index, signature) in signatures.iter().enumerate() {
        let issue = layout_spice_comparison_issue_text(kind, signature);
        if search.is_some_and(|query| {
            !layout_spice_comparison_issue_matches_search(kind, signature, query)
        }) {
            continue;
        }
        entries.push((format!("{kind}.{index}"), compact_button_label(&issue, 30)));
    }
}

pub(crate) fn layout_spice_comparison_issue_kind_visible(
    filter: LayoutNetBrowserFilter,
    kind: &str,
) -> bool {
    match filter {
        LayoutNetBrowserFilter::SpiceMissing => matches!(kind, "missing_net" | "missing_device"),
        LayoutNetBrowserFilter::SpiceExtra => matches!(kind, "extra_net" | "extra_device"),
        LayoutNetBrowserFilter::SpiceNets => matches!(kind, "missing_net" | "extra_net"),
        LayoutNetBrowserFilter::SpiceDevices => {
            matches!(kind, "missing_device" | "extra_device")
        }
        _ => true,
    }
}

pub(crate) fn layout_spice_comparison_issue_text(kind: &str, detail: &str) -> String {
    match kind {
        "missing_net" => format!("Missing layout net {detail}"),
        "extra_net" => format!("Extra layout net {detail}"),
        "missing_device" => format!("Missing layout device {detail}"),
        "extra_device" => format!("Extra layout device {detail}"),
        _ => detail.to_string(),
    }
}

pub(crate) fn layout_spice_comparison_issue_matches_search(
    kind: &str,
    detail: &str,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    let issue = layout_spice_comparison_issue_text(kind, detail);
    let mut fields = vec![
        "spice".to_string(),
        "lvs".to_string(),
        "compare".to_string(),
        "comparison".to_string(),
        detail.to_string(),
        issue,
    ];
    match kind {
        "missing_net" => {
            push_layout_search_field(&mut fields, "missing".to_string());
            push_layout_search_field(&mut fields, "net".to_string());
            push_layout_search_field(&mut fields, "missing net".to_string());
            push_layout_search_field(&mut fields, "missing layout net".to_string());
        }
        "extra_net" => {
            push_layout_search_field(&mut fields, "extra".to_string());
            push_layout_search_field(&mut fields, "net".to_string());
            push_layout_search_field(&mut fields, "extra net".to_string());
            push_layout_search_field(&mut fields, "extra layout net".to_string());
        }
        "missing_device" => {
            push_layout_search_field(&mut fields, "missing".to_string());
            push_layout_search_field(&mut fields, "device".to_string());
            push_layout_search_field(&mut fields, "missing device".to_string());
            push_layout_search_field(&mut fields, "missing layout device".to_string());
        }
        "extra_device" => {
            push_layout_search_field(&mut fields, "extra".to_string());
            push_layout_search_field(&mut fields, "device".to_string());
            push_layout_search_field(&mut fields, "extra device".to_string());
            push_layout_search_field(&mut fields, "extra layout device".to_string());
        }
        _ => {}
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_spice_comparison_device_signature_keys(signature: &str) -> Vec<&'static str> {
    let Some(parameters) = signature.split('|').nth(3) else {
        return Vec::new();
    };
    ["L", "W"]
        .iter()
        .copied()
        .filter(|key| {
            parameters
                .split(',')
                .any(|parameter| parameter.starts_with(&format!("{key}=")))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayoutTracePathSegmentStatus {
    Connected {
        component_id: usize,
        component_name: String,
    },
    Disconnected {
        start_component: usize,
        goal_component: usize,
    },
    TooManyShapes,
    StartUntraceable,
    EndUntraceable,
    ConnectivityUnavailable,
    StartUnnetted,
    EndUnnetted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayoutTracePointStatus {
    Netted {
        component_id: usize,
        component_name: String,
    },
    Unnetted,
    Untraceable,
    ConnectivityUnavailable,
}

pub(crate) fn layout_trace_point_entries(app: &GlassworksApp) -> Vec<(usize, String)> {
    if app.route_points.is_empty() {
        return Vec::new();
    }
    let search = layout_browser_search_query_lower(app);
    app.route_points
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, point)| {
            let status = layout_trace_point_status(app, point);
            if search.as_deref().is_some_and(|query| {
                !layout_trace_point_matches_search(index, point, &status, query)
            }) {
                return None;
            }
            Some((
                index,
                compact_button_label(
                    &format!(
                        "P{} {} {}",
                        index + 1,
                        layout_trace_point_coordinate_label(point),
                        layout_trace_point_status_label(&status)
                    ),
                    30,
                ),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_trace_point_status(
    app: &GlassworksApp,
    point: Point,
) -> LayoutTracePointStatus {
    let Ok(occurrence) = app.top_layout_occurrence_at_trace_point(point) else {
        return LayoutTracePointStatus::Untraceable;
    };
    let Ok(report) = app.connectivity_report() else {
        return LayoutTracePointStatus::ConnectivityUnavailable;
    };
    let Some(component_id) = report.component_for_occurrence(&occurrence) else {
        return LayoutTracePointStatus::Unnetted;
    };
    LayoutTracePointStatus::Netted {
        component_id,
        component_name: layout_trace_path_component_name(app, component_id),
    }
}

pub(crate) fn layout_trace_point_coordinate_label(point: Point) -> String {
    format!("({}, {})", point.x, point.y)
}

pub(crate) fn layout_trace_point_status_label(status: &LayoutTracePointStatus) -> String {
    match status {
        LayoutTracePointStatus::Netted {
            component_id,
            component_name,
        } => format!("#{component_id} {component_name}"),
        LayoutTracePointStatus::Unnetted => "No net".to_string(),
        LayoutTracePointStatus::Untraceable => "Untraceable".to_string(),
        LayoutTracePointStatus::ConnectivityUnavailable => "Trace unavailable".to_string(),
    }
}

pub(crate) fn layout_trace_point_brief_status_label(status: &LayoutTracePointStatus) -> String {
    match status {
        LayoutTracePointStatus::Netted { component_id, .. } => format!("#{component_id}"),
        _ => layout_trace_point_status_label(status),
    }
}

pub(crate) fn layout_trace_point_matches_search(
    index: usize,
    point: Point,
    status: &LayoutTracePointStatus,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    if layout_trace_point_selector_matches_search(index, point, status, &query_lower) {
        return true;
    }
    let mut fields = vec![
        "trace".to_string(),
        "trace point".to_string(),
        "route point".to_string(),
        format!("point {}", index + 1),
        format!("p{}", index + 1),
        layout_trace_point_coordinate_label(point),
        format!("{},{}", point.x, point.y),
        format!("{} {}", point.x, point.y),
        format!("point {} {}", point.x, point.y),
        layout_trace_point_status_label(status),
    ];
    match status {
        LayoutTracePointStatus::Netted {
            component_id,
            component_name,
        } => {
            push_layout_search_field(&mut fields, "net");
            push_layout_search_field(&mut fields, format!("component {component_id}"));
            push_layout_search_field(&mut fields, format!("#{component_id}"));
            push_layout_search_field(&mut fields, component_name.clone());
        }
        LayoutTracePointStatus::Unnetted => {
            push_layout_search_field(&mut fields, "no net");
            push_layout_search_field(&mut fields, "unnetted");
        }
        LayoutTracePointStatus::Untraceable => {
            push_layout_search_field(&mut fields, "untraceable");
        }
        LayoutTracePointStatus::ConnectivityUnavailable => {
            push_layout_search_field(&mut fields, "trace unavailable");
            push_layout_search_field(&mut fields, "connectivity unavailable");
        }
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_trace_point_selector_matches_search(
    index: usize,
    point: Point,
    status: &LayoutTracePointStatus,
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
    let point_index = index + 1;
    let mut fields = vec![
        ("point", point_index.to_string()),
        ("trace_point", point_index.to_string()),
        ("route_point", point_index.to_string()),
        ("index", point_index.to_string()),
        ("x", point.x.to_string()),
        ("y", point.y.to_string()),
        ("coordinate", format!("{},{}", point.x, point.y)),
        ("coordinates", format!("{},{}", point.x, point.y)),
        ("status", layout_trace_point_status_label(status)),
    ];
    match status {
        LayoutTracePointStatus::Netted {
            component_id,
            component_name,
        } => {
            fields.push(("status", "netted".to_string()));
            fields.push(("component", component_id.to_string()));
            fields.push(("component_id", component_id.to_string()));
            fields.push(("net", component_name.clone()));
        }
        LayoutTracePointStatus::Unnetted => {
            fields.push(("status", "unnetted".to_string()));
            fields.push(("net", "none".to_string()));
        }
        LayoutTracePointStatus::Untraceable => {
            fields.push(("status", "untraceable".to_string()));
        }
        LayoutTracePointStatus::ConnectivityUnavailable => {
            fields.push(("status", "trace unavailable".to_string()));
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_trace_point_summary_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let total = app.route_points.len();
    if total == 0 {
        return Vec::new();
    }
    let listed = layout_trace_point_entries(app).len();
    let search = layout_browser_search_query_lower(app);
    let mut netted = 0;
    let mut unnetted = 0;
    let mut untraceable = 0;
    let mut unavailable = 0;
    for point in &app.route_points {
        match layout_trace_point_status(app, *point) {
            LayoutTracePointStatus::Netted { .. } => netted += 1,
            LayoutTracePointStatus::Unnetted => unnetted += 1,
            LayoutTracePointStatus::Untraceable => untraceable += 1,
            LayoutTracePointStatus::ConnectivityUnavailable => unavailable += 1,
        }
    }
    let mut rows = vec![
        (
            "Trace points".to_string(),
            if search.is_some() {
                format!("{listed} listed / {total} total")
            } else {
                format!("{total} total")
            },
        ),
        ("Trace point nets".to_string(), format!("{netted} netted")),
    ];
    if unnetted + untraceable + unavailable > 0 {
        rows.push((
            "Trace point unresolved".to_string(),
            format!("{untraceable} untraceable / {unnetted} no net / {unavailable} unavailable"),
        ));
    }
    rows
}

pub(crate) fn layout_trace_path_segment_entries(app: &GlassworksApp) -> Vec<(usize, String)> {
    let search = layout_browser_search_query_lower(app);
    layout_trace_path_segment_entries_with_search(app, search.as_deref())
}

pub(crate) fn layout_trace_path_segment_entries_with_search(
    app: &GlassworksApp,
    search: Option<&str>,
) -> Vec<(usize, String)> {
    if app.route_points.len() < 2 {
        return Vec::new();
    }
    app.route_points
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
            if search.is_some_and(|query| {
                !layout_trace_path_segment_matches_search(
                    app, index, pair[0], pair[1], &status, query,
                )
            }) {
                return None;
            }
            Some((
                index,
                compact_button_label(
                    &format!(
                        "S{} {} {} {}",
                        index + 1,
                        layout_trace_path_segment_brief_status_label(&status),
                        layout_trace_path_segment_length_label(app, pair[0], pair[1]),
                        layout_trace_path_segment_coordinate_label(pair[0], pair[1])
                    ),
                    64,
                ),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_trace_path_segment_net_entries(app: &GlassworksApp) -> Vec<(usize, String)> {
    let search = layout_browser_search_query_lower(app);
    if app.route_points.len() < 2 {
        return Vec::new();
    }
    app.route_points
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
            if search.as_deref().is_some_and(|query| {
                !layout_trace_path_segment_matches_search(
                    app, index, pair[0], pair[1], &status, query,
                )
            }) {
                return None;
            }
            let LayoutTracePathSegmentStatus::Connected {
                component_id,
                component_name,
            } = status
            else {
                return None;
            };
            Some((
                index,
                compact_button_label(
                    &format!("Select S{} Net #{component_id} {component_name}", index + 1),
                    30,
                ),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_trace_path_segment_endpoint_entries(
    app: &GlassworksApp,
) -> Vec<(String, String)> {
    let search = layout_browser_search_query_lower(app);
    if app.route_points.len() < 2 {
        return Vec::new();
    }
    app.route_points
        .windows(2)
        .enumerate()
        .flat_map(|(index, pair)| {
            let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
            if search.as_deref().is_some_and(|query| {
                !layout_trace_path_segment_matches_search(
                    app, index, pair[0], pair[1], &status, query,
                )
            }) {
                return Vec::new();
            }
            let LayoutTracePathSegmentStatus::Disconnected {
                start_component,
                goal_component,
            } = status
            else {
                return Vec::new();
            };
            vec![
                (
                    format!("{index}.start"),
                    compact_button_label(
                        &format!(
                            "Select S{} Start Net #{} {}",
                            index + 1,
                            start_component,
                            layout_trace_path_component_name(app, start_component)
                        ),
                        30,
                    ),
                ),
                (
                    format!("{index}.end"),
                    compact_button_label(
                        &format!(
                            "Select S{} End Net #{} {}",
                            index + 1,
                            goal_component,
                            layout_trace_path_component_name(app, goal_component)
                        ),
                        30,
                    ),
                ),
            ]
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_trace_path_blocker_endpoint_entries(
    app: &GlassworksApp,
) -> Vec<(String, String)> {
    let Some((index, _, _, status)) = layout_trace_path_first_blocker(app) else {
        return Vec::new();
    };
    let LayoutTracePathSegmentStatus::Disconnected {
        start_component,
        goal_component,
    } = status
    else {
        return Vec::new();
    };
    vec![
        (
            "start".to_string(),
            compact_button_label(
                &format!(
                    "Select Blocker S{} Start Net #{} {}",
                    index + 1,
                    start_component,
                    layout_trace_path_component_name(app, start_component)
                ),
                32,
            ),
        ),
        (
            "end".to_string(),
            compact_button_label(
                &format!(
                    "Select Blocker S{} End Net #{} {}",
                    index + 1,
                    goal_component,
                    layout_trace_path_component_name(app, goal_component)
                ),
                32,
            ),
        ),
    ]
}

pub(crate) fn layout_trace_path_endpoint_net_entries(app: &GlassworksApp) -> Vec<(String, String)> {
    if app.route_points.len() < 2 {
        return Vec::new();
    }
    let mut entries = Vec::new();
    if let Some(start) = app.route_points.first().copied()
        && let LayoutTracePointStatus::Netted {
            component_id,
            component_name,
        } = layout_trace_point_status(app, start)
    {
        entries.push((
            "start".to_string(),
            compact_button_label(
                &format!("Select Path Start Net #{component_id} {component_name}"),
                32,
            ),
        ));
    }
    if let Some(goal) = app.route_points.last().copied()
        && let LayoutTracePointStatus::Netted {
            component_id,
            component_name,
        } = layout_trace_point_status(app, goal)
    {
        entries.push((
            "end".to_string(),
            compact_button_label(
                &format!("Select Path End Net #{component_id} {component_name}"),
                32,
            ),
        ));
    }
    entries
}

pub(crate) fn layout_trace_path_component_name(app: &GlassworksApp, component_id: usize) -> String {
    app.connectivity_report()
        .ok()
        .and_then(|report| {
            report
                .component(component_id)
                .map(connectivity_component_display_name)
        })
        .unwrap_or_else(|| format!("component {component_id}"))
}

pub(crate) fn layout_trace_path_first_blocker(
    app: &GlassworksApp,
) -> Option<(usize, Point, Point, LayoutTracePathSegmentStatus)> {
    app.route_points
        .windows(2)
        .enumerate()
        .find_map(|(index, pair)| {
            let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
            (!matches!(&status, LayoutTracePathSegmentStatus::Connected { .. }))
                .then_some((index, pair[0], pair[1], status))
        })
}

pub(crate) fn layout_trace_path_connected_component(
    app: &GlassworksApp,
) -> Option<(usize, String)> {
    if layout_trace_path_first_blocker(app).is_some() {
        return None;
    }
    let pair = app.route_points.windows(2).next()?;
    match layout_trace_path_segment_status(app, pair[0], pair[1]) {
        LayoutTracePathSegmentStatus::Connected {
            component_id,
            component_name,
        } => Some((component_id, component_name)),
        _ => None,
    }
}

pub(crate) fn layout_trace_path_segment_length(start: Point, goal: Point) -> f64 {
    start.distance_to(goal)
}

pub(crate) fn layout_trace_path_segment_length_label(
    app: &GlassworksApp,
    start: Point,
    goal: Point,
) -> String {
    app.format_layout_length(layout_trace_path_segment_length(start, goal))
}

pub(crate) fn layout_trace_path_length_summary(app: &GlassworksApp) -> (f64, f64, f64) {
    let mut total = 0.0;
    let mut connected = 0.0;
    let mut blocked = 0.0;
    for pair in app.route_points.windows(2) {
        let length = layout_trace_path_segment_length(pair[0], pair[1]);
        total += length;
        let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
        if matches!(status, LayoutTracePathSegmentStatus::Connected { .. }) {
            connected += length;
        } else {
            blocked += length;
        }
    }
    (total, connected, blocked)
}

pub(crate) fn layout_trace_path_result_label(app: &GlassworksApp) -> String {
    if let Some((index, start, goal, status)) = layout_trace_path_first_blocker(app) {
        return compact_button_label(
            &format!(
                "Blocked at S{} {} {}",
                index + 1,
                layout_trace_path_segment_coordinate_label(start, goal),
                layout_trace_path_segment_status_label(&status)
            ),
            64,
        );
    }
    let Some(pair) = app.route_points.windows(2).next() else {
        return "No pending path".to_string();
    };
    match layout_trace_path_segment_status(app, pair[0], pair[1]) {
        LayoutTracePathSegmentStatus::Connected {
            component_id,
            component_name,
        } => compact_button_label(&format!("Connected #{component_id} {component_name}"), 64),
        status => compact_button_label(&layout_trace_path_segment_status_label(&status), 64),
    }
}

pub(crate) fn layout_trace_path_endpoint_summary_label(app: &GlassworksApp) -> Option<String> {
    let start = app.route_points.first().copied()?;
    let goal = app.route_points.last().copied()?;
    if app.route_points.len() < 2 {
        return None;
    }
    let start_status = layout_trace_point_status(app, start);
    let goal_status = layout_trace_point_status(app, goal);
    Some(compact_button_label(
        &format!(
            "P1 {} {} -> P{} {} {}",
            layout_trace_point_coordinate_label(start),
            layout_trace_point_brief_status_label(&start_status),
            app.route_points.len(),
            layout_trace_point_coordinate_label(goal),
            layout_trace_point_brief_status_label(&goal_status)
        ),
        64,
    ))
}

pub(crate) fn layout_trace_path_segment_summary_rows(app: &GlassworksApp) -> Vec<(String, String)> {
    let total = app.route_points.len().saturating_sub(1);
    if total == 0 {
        return Vec::new();
    }
    let search = layout_browser_search_query_lower(app);
    let listed = layout_trace_path_segment_entries_with_search(app, search.as_deref()).len();
    let mut connected = 0;
    let mut disconnected = 0;
    let mut untraceable = 0;
    let mut unnetted = 0;
    let mut unavailable = 0;
    for pair in app.route_points.windows(2) {
        let status = layout_trace_path_segment_status(app, pair[0], pair[1]);
        match &status {
            LayoutTracePathSegmentStatus::Connected { .. } => connected += 1,
            LayoutTracePathSegmentStatus::Disconnected { .. } => disconnected += 1,
            LayoutTracePathSegmentStatus::StartUntraceable
            | LayoutTracePathSegmentStatus::EndUntraceable => untraceable += 1,
            LayoutTracePathSegmentStatus::StartUnnetted
            | LayoutTracePathSegmentStatus::EndUnnetted => unnetted += 1,
            LayoutTracePathSegmentStatus::TooManyShapes
            | LayoutTracePathSegmentStatus::ConnectivityUnavailable => unavailable += 1,
        }
    }
    let mut rows = vec![
        (
            "Trace path result".to_string(),
            layout_trace_path_result_label(app),
        ),
        (
            "Trace path endpoints".to_string(),
            layout_trace_path_endpoint_summary_label(app).unwrap_or_else(|| "None".to_string()),
        ),
        (
            "Trace path segments".to_string(),
            if search.is_some() {
                format!("{listed} listed / {total} total")
            } else {
                format!("{total} total")
            },
        ),
        (
            "Trace path status".to_string(),
            format!("{connected} connected / {disconnected} disconnected"),
        ),
    ];
    let (total_length, connected_length, blocked_length) = layout_trace_path_length_summary(app);
    rows.push((
        "Trace path length".to_string(),
        format!(
            "{} total / {} connected / {} blocked",
            app.format_layout_length(total_length),
            app.format_layout_length(connected_length),
            app.format_layout_length(blocked_length)
        ),
    ));
    if untraceable + unnetted + unavailable > 0 {
        rows.push((
            "Trace path unresolved".to_string(),
            format!("{untraceable} untraceable / {unnetted} no net / {unavailable} unavailable"),
        ));
    }
    if let Some((index, start, goal, status)) = layout_trace_path_first_blocker(app) {
        rows.push((
            "Trace path blocker".to_string(),
            compact_button_label(
                &format!(
                    "S{} {} {}",
                    index + 1,
                    layout_trace_path_segment_coordinate_label(start, goal),
                    layout_trace_path_segment_status_label(&status)
                ),
                64,
            ),
        ));
    }
    rows
}

pub(crate) fn layout_trace_path_segment_status(
    app: &GlassworksApp,
    start: Point,
    goal: Point,
) -> LayoutTracePathSegmentStatus {
    if app.workspace.document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return LayoutTracePathSegmentStatus::TooManyShapes;
    }
    let Ok(start_occurrence) = app.top_layout_occurrence_at_trace_point(start) else {
        return LayoutTracePathSegmentStatus::StartUntraceable;
    };
    let Ok(goal_occurrence) = app.top_layout_occurrence_at_trace_point(goal) else {
        return LayoutTracePathSegmentStatus::EndUntraceable;
    };
    let Ok(report) = app.connectivity_report() else {
        return LayoutTracePathSegmentStatus::ConnectivityUnavailable;
    };
    let Some(start_component) = report.component_for_occurrence(&start_occurrence) else {
        return LayoutTracePathSegmentStatus::StartUnnetted;
    };
    let Some(goal_component) = report.component_for_occurrence(&goal_occurrence) else {
        return LayoutTracePathSegmentStatus::EndUnnetted;
    };
    if start_component == goal_component {
        let component_name = report
            .component(start_component)
            .map(connectivity_component_display_name)
            .unwrap_or_else(|| format!("component {start_component}"));
        LayoutTracePathSegmentStatus::Connected {
            component_id: start_component,
            component_name,
        }
    } else {
        LayoutTracePathSegmentStatus::Disconnected {
            start_component,
            goal_component,
        }
    }
}

pub(crate) fn layout_trace_path_segment_status_label(
    status: &LayoutTracePathSegmentStatus,
) -> String {
    match status {
        LayoutTracePathSegmentStatus::Connected {
            component_id,
            component_name,
        } => format!("Connected #{component_id} {component_name}"),
        LayoutTracePathSegmentStatus::Disconnected {
            start_component,
            goal_component,
        } => format!("Disconnected #{start_component} -> #{goal_component}"),
        LayoutTracePathSegmentStatus::TooManyShapes
        | LayoutTracePathSegmentStatus::ConnectivityUnavailable => "Trace unavailable".to_string(),
        LayoutTracePathSegmentStatus::StartUntraceable => "Untraceable start".to_string(),
        LayoutTracePathSegmentStatus::EndUntraceable => "Untraceable end".to_string(),
        LayoutTracePathSegmentStatus::StartUnnetted => "No start net".to_string(),
        LayoutTracePathSegmentStatus::EndUnnetted => "No end net".to_string(),
    }
}

pub(crate) fn layout_trace_path_segment_brief_status_label(
    status: &LayoutTracePathSegmentStatus,
) -> String {
    match status {
        LayoutTracePathSegmentStatus::Connected { component_id, .. } => {
            format!("Connected #{component_id}")
        }
        LayoutTracePathSegmentStatus::Disconnected {
            start_component,
            goal_component,
        } => format!("Disconnected #{start_component}->#{goal_component}"),
        _ => layout_trace_path_segment_status_label(status),
    }
}

pub(crate) fn layout_trace_path_segment_coordinate_label(start: Point, goal: Point) -> String {
    format!("{},{}..{},{}", start.x, start.y, goal.x, goal.y)
}

pub(crate) fn layout_trace_path_segment_matches_search(
    app: &GlassworksApp,
    index: usize,
    start: Point,
    goal: Point,
    status: &LayoutTracePathSegmentStatus,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    if layout_trace_path_segment_selector_matches_search(
        app,
        index,
        start,
        goal,
        status,
        &query_lower,
    ) {
        return true;
    }
    let mut fields = vec![
        "trace".to_string(),
        "trace path".to_string(),
        "trace path segment".to_string(),
        "segment".to_string(),
        format!("segment {}", index + 1),
        format!("s{}", index + 1),
        layout_trace_path_segment_coordinate_label(start, goal),
        format!(
            "{} -> {}",
            layout_trace_point_coordinate_label(start),
            layout_trace_point_coordinate_label(goal)
        ),
        format!("{} {} {} {}", start.x, start.y, goal.x, goal.y),
        format!("start {} {}", start.x, start.y),
        format!("end {} {}", goal.x, goal.y),
        format!("from {} {} to {} {}", start.x, start.y, goal.x, goal.y),
        layout_trace_path_segment_length_label(app, start, goal),
        format!(
            "length {}",
            layout_trace_path_segment_length_label(app, start, goal)
        ),
        format!(
            "length {:.0}",
            layout_trace_path_segment_length(start, goal)
        ),
        layout_trace_path_segment_status_label(status),
    ];
    match status {
        LayoutTracePathSegmentStatus::Connected {
            component_id,
            component_name,
        } => {
            push_layout_search_field(&mut fields, "connected");
            push_layout_search_field(&mut fields, format!("connected {}", index + 1));
            push_layout_search_field(&mut fields, format!("component {component_id}"));
            push_layout_search_field(&mut fields, format!("#{component_id}"));
            push_layout_search_field(&mut fields, component_name.clone());
        }
        LayoutTracePathSegmentStatus::Disconnected {
            start_component,
            goal_component,
        } => {
            push_layout_search_field(&mut fields, "disconnected");
            push_layout_search_field(&mut fields, format!("disconnected {}", index + 1));
            push_layout_search_field(&mut fields, format!("component {start_component}"));
            push_layout_search_field(&mut fields, format!("component {goal_component}"));
            push_layout_search_field(&mut fields, format!("#{start_component}"));
            push_layout_search_field(&mut fields, format!("#{goal_component}"));
        }
        LayoutTracePathSegmentStatus::TooManyShapes => {
            push_layout_search_field(&mut fields, "trace unavailable");
            push_layout_search_field(&mut fields, "too many shapes");
        }
        LayoutTracePathSegmentStatus::ConnectivityUnavailable => {
            push_layout_search_field(&mut fields, "trace unavailable");
            push_layout_search_field(&mut fields, "connectivity unavailable");
        }
        LayoutTracePathSegmentStatus::StartUntraceable => {
            push_layout_search_field(&mut fields, "untraceable");
            push_layout_search_field(&mut fields, "untraceable start");
            push_layout_search_field(&mut fields, "start");
        }
        LayoutTracePathSegmentStatus::EndUntraceable => {
            push_layout_search_field(&mut fields, "untraceable");
            push_layout_search_field(&mut fields, "untraceable end");
            push_layout_search_field(&mut fields, "end");
        }
        LayoutTracePathSegmentStatus::StartUnnetted => {
            push_layout_search_field(&mut fields, "no net");
            push_layout_search_field(&mut fields, "no start net");
            push_layout_search_field(&mut fields, "start");
        }
        LayoutTracePathSegmentStatus::EndUnnetted => {
            push_layout_search_field(&mut fields, "no net");
            push_layout_search_field(&mut fields, "no end net");
            push_layout_search_field(&mut fields, "end");
        }
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_trace_path_segment_selector_matches_search(
    app: &GlassworksApp,
    index: usize,
    start: Point,
    goal: Point,
    status: &LayoutTracePathSegmentStatus,
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
    let segment_index = index + 1;
    let mut fields = vec![
        ("segment", segment_index.to_string()),
        ("trace_segment", segment_index.to_string()),
        ("index", segment_index.to_string()),
        ("start", format!("{},{}", start.x, start.y)),
        ("end", format!("{},{}", goal.x, goal.y)),
        ("from", format!("{},{}", start.x, start.y)),
        ("to", format!("{},{}", goal.x, goal.y)),
        ("start_x", start.x.to_string()),
        ("start_y", start.y.to_string()),
        ("end_x", goal.x.to_string()),
        ("end_y", goal.y.to_string()),
        (
            "coordinates",
            layout_trace_path_segment_coordinate_label(start, goal),
        ),
        (
            "length",
            layout_trace_path_segment_length_label(app, start, goal),
        ),
        (
            "length_dbu",
            format!("{:.0}", layout_trace_path_segment_length(start, goal)),
        ),
        ("status", layout_trace_path_segment_status_label(status)),
    ];
    match status {
        LayoutTracePathSegmentStatus::Connected {
            component_id,
            component_name,
        } => {
            fields.push(("status", "connected".to_string()));
            fields.push(("component", component_id.to_string()));
            fields.push(("component_id", component_id.to_string()));
            fields.push(("net", component_name.clone()));
        }
        LayoutTracePathSegmentStatus::Disconnected {
            start_component,
            goal_component,
        } => {
            fields.push(("status", "disconnected".to_string()));
            fields.push(("start_component", start_component.to_string()));
            fields.push(("goal_component", goal_component.to_string()));
            fields.push(("end_component", goal_component.to_string()));
        }
        LayoutTracePathSegmentStatus::TooManyShapes => {
            fields.push(("status", "too many shapes".to_string()));
        }
        LayoutTracePathSegmentStatus::StartUntraceable => {
            fields.push(("status", "start untraceable".to_string()));
        }
        LayoutTracePathSegmentStatus::EndUntraceable => {
            fields.push(("status", "end untraceable".to_string()));
        }
        LayoutTracePathSegmentStatus::ConnectivityUnavailable => {
            fields.push(("status", "trace unavailable".to_string()));
        }
        LayoutTracePathSegmentStatus::StartUnnetted => {
            fields.push(("status", "start unnetted".to_string()));
        }
        LayoutTracePathSegmentStatus::EndUnnetted => {
            fields.push(("status", "end unnetted".to_string()));
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
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
    let trace_history_positions = (filter == LayoutNetBrowserFilter::History).then(|| {
        trace_history
            .iter()
            .copied()
            .enumerate()
            .map(|(index, component_id)| (component_id, index))
            .collect::<BTreeMap<_, _>>()
    });
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
            LayoutNetBrowserFilter::History => trace_history_positions
                .as_ref()
                .is_some_and(|history| history.contains_key(&component.id)),
            LayoutNetBrowserFilter::SpiceMissing => {
                layout_net_component_has_spice_missing_issue(report, component, spice_comparison)
            }
            LayoutNetBrowserFilter::SpiceExtra => {
                layout_net_component_is_spice_extra(report, component, spice_comparison)
            }
            LayoutNetBrowserFilter::SpiceNets => {
                layout_net_component_has_spice_net_issue(component, spice_comparison)
            }
            LayoutNetBrowserFilter::SpiceDevices => {
                layout_net_component_has_spice_device_issue(report, component.id, spice_comparison)
            }
            LayoutNetBrowserFilter::SpiceIssues => {
                layout_net_component_has_spice_issue(report, component, spice_comparison)
            }
            LayoutNetBrowserFilter::Shorted => layout_net_component_has_short(report, component.id),
            LayoutNetBrowserFilter::Open => layout_net_component_has_open(report, component.id),
        })
        .filter(|component| {
            let history_index = trace_history_positions
                .as_ref()
                .and_then(|history| history.get(&component.id).copied());
            search.is_none_or(|query| {
                if let Some(history_index) = history_index
                    && layout_trace_history_entry_selector_matches_search(
                        history_index,
                        component,
                        query,
                    )
                {
                    return true;
                }
                layout_net_component_matches_search_with_spice(
                    report,
                    component,
                    query,
                    spice_comparison,
                )
            })
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
    report: &ConnectivityReport,
    component: &NetComponent,
    query_lower: &str,
) -> bool {
    layout_net_component_matches_search_with_spice(report, component, query_lower, None)
}

pub(crate) fn layout_net_component_matches_search_with_spice(
    report: &ConnectivityReport,
    component: &NetComponent,
    query_lower: &str,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    if layout_net_component_selector_matches_search(report, component, &query_lower, comparison) {
        return true;
    }
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
    for device in layout_net_component_devices(report, component.id) {
        push_layout_search_field(&mut fields, "device");
        push_layout_search_field(&mut fields, format!("device {}", device.id));
        push_layout_search_field(&mut fields, format!("device #{}", device.id));
        push_layout_search_field(&mut fields, device.kind.clone());
        push_layout_search_field(&mut fields, device.model.clone());
        push_layout_search_field(
            &mut fields,
            layout_net_component_device_text(device, component.id),
        );
        for terminal in &device.terminals {
            let is_current = terminal.component == Some(component.id);
            if is_current {
                push_layout_search_field(&mut fields, terminal.name.clone());
                push_layout_search_field(&mut fields, format!("terminal {}", terminal.name));
            } else {
                push_layout_search_field(&mut fields, "peer");
                push_layout_search_field(&mut fields, format!("peer {}", terminal.name));
                push_layout_search_field(&mut fields, format!("peer terminal {}", terminal.name));
                if let Some(peer_id) = terminal.component {
                    push_layout_search_field(&mut fields, format!("peer component {peer_id}"));
                    if let Some(peer_component) = report.component(peer_id) {
                        push_layout_search_field(
                            &mut fields,
                            connectivity_component_display_name(peer_component),
                        );
                    }
                }
            }
            if let Some(net_name) = &terminal.net_name {
                push_layout_search_field(&mut fields, net_name.clone());
                if !is_current {
                    push_layout_search_field(&mut fields, format!("peer net {net_name}"));
                }
            }
        }
    }
    for short in report
        .shorts
        .iter()
        .filter(|short| short.component == component.id)
    {
        let mut names = short.names.clone();
        names.sort();
        push_layout_search_field(&mut fields, "short");
        push_layout_search_field(&mut fields, names.join(" "));
        push_layout_search_field(&mut fields, format!("short {}", names.join(" ")));
        push_layout_search_field(&mut fields, format!("short {}", names.join("/")));
    }
    for open in report
        .opens
        .iter()
        .filter(|open| open.components.contains(&component.id))
    {
        push_layout_search_field(&mut fields, "open");
        push_layout_search_field(&mut fields, open.name.clone());
        push_layout_search_field(&mut fields, format!("open {}", open.name));
    }
    if let Some(comparison) = comparison {
        if let Some(status) = layout_spice_component_status_label(component, Some(comparison)) {
            push_layout_search_field(&mut fields, "spice");
            push_layout_search_field(&mut fields, "spice net");
            push_layout_search_field(&mut fields, "lvs net");
            push_layout_search_field(&mut fields, status);
        }
        if let Some(status) =
            layout_spice_component_device_status_label(report, component.id, Some(comparison))
        {
            push_layout_search_field(&mut fields, "spice device");
            push_layout_search_field(&mut fields, "lvs device");
            push_layout_search_field(&mut fields, status);
        }
    }
    layout_search_matches_any(&query_lower, &fields)
}

pub(crate) fn layout_net_component_selector_matches_search(
    report: &ConnectivityReport,
    component: &NetComponent,
    query_lower: &str,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    let Some((key_query, value_query)) = query_lower.split_once('=') else {
        return false;
    };
    let key_query = key_query.trim();
    let value_query = value_query.trim();
    if key_query.is_empty() && value_query.is_empty() {
        return true;
    }
    let short_count = report
        .shorts
        .iter()
        .filter(|short| short.component == component.id)
        .count();
    let open_count = report
        .opens
        .iter()
        .filter(|open| open.components.contains(&component.id))
        .count();
    let device_count = layout_net_component_device_count(report, component.id);
    let mut fields = vec![
        ("id", component.id.to_string()),
        ("component", component.id.to_string()),
        ("component_id", component.id.to_string()),
        ("name", connectivity_component_display_name(component)),
        (
            "display_name",
            connectivity_component_display_name(component),
        ),
        (
            "labeled",
            connectivity_component_is_labeled(component).to_string(),
        ),
        ("shapes", component.shapes.len().to_string()),
        ("shape_count", component.shapes.len().to_string()),
        ("labels", component.labels.len().to_string()),
        ("label_count", component.labels.len().to_string()),
        ("explicit_nets", component.explicit_nets.len().to_string()),
        (
            "explicit_net_count",
            component.explicit_nets.len().to_string(),
        ),
        ("devices", device_count.to_string()),
        ("device_count", device_count.to_string()),
        ("shorts", short_count.to_string()),
        ("short_count", short_count.to_string()),
        ("opens", open_count.to_string()),
        ("open_count", open_count.to_string()),
        (
            "issues",
            layout_net_component_issue_label(report, component.id),
        ),
        ("bounds", rect_summary(component.bounds)),
    ];
    if let Some(name) = &component.net_name {
        fields.push(("net", name.clone()));
        fields.push(("net_name", name.clone()));
    }
    if let Some(net_id) = component.net_id {
        fields.push(("net", format!("N{} net {}", net_id.0, net_id.0)));
        fields.push(("net_id", net_id.0.to_string()));
    }
    for explicit_net in &component.explicit_nets {
        fields.push(("net", format!("N{} net {}", explicit_net.0, explicit_net.0)));
        fields.push(("explicit_net", explicit_net.0.to_string()));
    }
    for label in &component.labels {
        fields.push(("label", label.text.clone()));
    }
    if let Some(summary) = layout_net_component_issue_summary(report, component.id) {
        fields.push(("issue", summary.clone()));
        fields.push(("issues", summary));
    }
    for device in layout_net_component_devices(report, component.id) {
        fields.push((
            "device",
            layout_net_component_device_text(device, component.id),
        ));
        fields.push(("device_id", device.id.to_string()));
        fields.push(("device_kind", device.kind.clone()));
        fields.push(("device_model", device.model.clone()));
        for terminal in &device.terminals {
            if terminal.component == Some(component.id) {
                fields.push(("terminal", terminal.name.clone()));
            } else {
                fields.push(("peer_terminal", terminal.name.clone()));
                if let Some(peer_id) = terminal.component {
                    fields.push(("peer_component", peer_id.to_string()));
                }
            }
            if let Some(net_name) = &terminal.net_name {
                fields.push(("device_net", net_name.clone()));
            }
        }
    }
    if let Some(comparison) = comparison {
        if let Some(status) = layout_spice_component_status_label(component, Some(comparison)) {
            fields.push(("spice", status.clone()));
            fields.push(("spice_net", status));
        }
        if let Some(status) =
            layout_spice_component_device_status_label(report, component.id, Some(comparison))
        {
            fields.push(("spice", status.clone()));
            fields.push(("spice_device", status));
        }
    }
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}

pub(crate) fn layout_component_for_occurrence_or_label(
    report: &ConnectivityReport,
    occurrence: &ShapeOccurrenceId,
) -> Option<usize> {
    report.component_for_occurrence(occurrence).or_else(|| {
        report
            .components
            .iter()
            .find(|component| {
                component
                    .labels
                    .iter()
                    .any(|label| label.occurrence == *occurrence)
            })
            .map(|component| component.id)
    })
}

pub(crate) fn selected_layout_net_component_id(
    app: &GlassworksApp,
    report: &ConnectivityReport,
) -> Option<usize> {
    app.selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| layout_component_for_occurrence_or_label(report, occurrence))
}

pub(crate) fn selected_layout_net_component_source_objects(
    app: &GlassworksApp,
) -> Vec<(ShapeOccurrenceId, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    let Some(component) = report.component(component_id) else {
        return Vec::new();
    };
    component
        .shapes
        .iter()
        .filter_map(|occurrence| {
            let view =
                document.shape_view_for_occurrence_from_cell(document.top_cell, occurrence)?;
            let shape = view.transformed_shape();
            Some((
                occurrence.clone(),
                layout_shape_browser_label(document, occurrence, view.source_cell, &shape),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn selected_layout_net_component_label_objects(
    app: &GlassworksApp,
) -> Vec<(ShapeOccurrenceId, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    let Some(component) = report.component(component_id) else {
        return Vec::new();
    };
    component
        .labels
        .iter()
        .filter_map(|label| {
            let view = document
                .shape_view_for_occurrence_from_cell(document.top_cell, &label.occurrence)?;
            let shape = view.transformed_shape();
            let object =
                layout_shape_browser_label(document, &label.occurrence, view.source_cell, &shape);
            Some((
                label.occurrence.clone(),
                compact_button_label(&format!("{} {object}", label.text), 30),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn layout_net_component_devices<'a>(
    report: &'a ConnectivityReport,
    component_id: usize,
) -> Vec<&'a ExtractedDevice> {
    report
        .devices
        .iter()
        .filter(|device| {
            device
                .terminals
                .iter()
                .any(|terminal| terminal.component == Some(component_id))
        })
        .collect()
}

pub(crate) fn layout_net_component_device_text(
    device: &ExtractedDevice,
    component_id: usize,
) -> String {
    let mut terminals = device
        .terminals
        .iter()
        .filter(|terminal| terminal.component == Some(component_id))
        .map(|terminal| terminal.name.clone())
        .collect::<Vec<_>>();
    terminals.sort();
    terminals.dedup();
    let model = if device.model.trim().is_empty() {
        device.kind.as_str()
    } else {
        device.model.as_str()
    };
    if terminals.is_empty() {
        return format!("#{} {}", device.id, model);
    }
    format!("#{} {} {}", device.id, model, terminals.join("/"))
}

pub(crate) fn selected_layout_net_component_device_objects(
    app: &GlassworksApp,
) -> Vec<(usize, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    layout_net_component_devices(&report, component_id)
        .into_iter()
        .map(|device| {
            (
                device.id,
                compact_button_label(&layout_net_component_device_text(device, component_id), 30),
            )
        })
        .take(8)
        .collect()
}

pub(crate) fn selected_layout_net_component_device_peers(
    app: &GlassworksApp,
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    let mut peers = BTreeSet::new();
    for device in layout_net_component_devices(&report, component_id) {
        let device_label = layout_net_component_device_text(device, component_id);
        for terminal in &device.terminals {
            let Some(peer_id) = terminal.component else {
                continue;
            };
            if peer_id == component_id {
                continue;
            }
            peers.insert((
                device.id,
                peer_id,
                terminal.name.clone(),
                device_label.clone(),
            ));
        }
    }
    peers
        .into_iter()
        .filter_map(|(device_id, peer_id, terminal, device_label)| {
            let component = report.component(peer_id)?;
            Some((
                format!("{device_id}.{peer_id}"),
                compact_button_label(
                    &format!(
                        "{} -> {} #{} {}",
                        device_label,
                        terminal,
                        peer_id,
                        connectivity_component_display_name(component)
                    ),
                    30,
                ),
            ))
        })
        .take(8)
        .collect()
}

pub(crate) fn selected_layout_net_component_issue_objects(
    app: &GlassworksApp,
) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for (index, short) in report.shorts.iter().enumerate() {
        if short.component != component_id {
            continue;
        }
        let mut names = short.names.clone();
        names.sort();
        entries.push((
            format!("short.{index}"),
            compact_button_label(&format!("Short {}", names.join("/")), 30),
        ));
    }
    for (index, open) in report.opens.iter().enumerate() {
        if !open.components.contains(&component_id) {
            continue;
        }
        entries.push((
            format!("open.{index}"),
            compact_button_label(&format!("Open {}", open.name), 30),
        ));
    }
    entries.into_iter().take(8).collect()
}

pub(crate) fn selected_layout_net_component_open_peers(
    app: &GlassworksApp,
) -> Vec<(usize, String)> {
    let document = &app.workspace.document;
    if document.flattened_shape_count_estimate() > MAX_CONNECTIVITY_OVERLAY_SHAPES {
        return Vec::new();
    }
    let Ok(report) = app.connectivity_report() else {
        return Vec::new();
    };
    let Some(component_id) = selected_layout_net_component_id(app, &report) else {
        return Vec::new();
    };
    let mut peers = BTreeSet::new();
    for open in &report.opens {
        if !open.components.contains(&component_id) {
            continue;
        }
        for peer_id in &open.components {
            if *peer_id != component_id {
                peers.insert((*peer_id, open.name.clone()));
            }
        }
    }
    peers
        .into_iter()
        .filter_map(|(peer_id, name)| {
            let component = report.component(peer_id)?;
            Some((
                peer_id,
                compact_button_label(
                    &format!(
                        "Open {} -> #{} {}",
                        name,
                        peer_id,
                        connectivity_component_display_name(component)
                    ),
                    30,
                ),
            ))
        })
        .take(8)
        .collect()
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
    report: &ConnectivityReport,
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    let Some(comparison) = comparison else {
        return false;
    };
    if component.net_name.as_deref().is_some_and(|net_name| {
        comparison
            .extra_layout_nets
            .iter()
            .any(|name| name.eq_ignore_ascii_case(net_name))
    }) {
        return true;
    }
    layout_net_component_devices(report, component.id)
        .into_iter()
        .any(|device| {
            comparison.extra_layout_devices.iter().any(|signature| {
                let mos_dimension_keys = layout_spice_comparison_device_signature_keys(signature);
                layout_model::connectivity::layout_spice_device_signature(
                    report,
                    device,
                    &mos_dimension_keys,
                )
                .is_some_and(|device_signature| device_signature.eq_ignore_ascii_case(signature))
            })
        })
}

pub(crate) fn layout_net_component_has_spice_net_issue(
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    layout_spice_component_status_label(component, comparison)
        .is_some_and(|status| status != "Matched schematic net")
}

pub(crate) fn layout_net_component_has_spice_missing_issue(
    report: &ConnectivityReport,
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    let Some(comparison) = comparison else {
        return false;
    };
    if component.net_name.as_deref().is_some_and(|net_name| {
        comparison
            .missing_layout_nets
            .iter()
            .any(|name| name.eq_ignore_ascii_case(net_name))
    }) {
        return true;
    }
    comparison
        .missing_layout_devices
        .iter()
        .any(|signature| layout_spice_signature_mentions_component(report, component.id, signature))
}

pub(crate) fn layout_net_component_has_spice_device_issue(
    report: &ConnectivityReport,
    component_id: usize,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    layout_spice_component_device_status_label(report, component_id, comparison).is_some()
}

pub(crate) fn layout_net_component_has_spice_issue(
    report: &ConnectivityReport,
    component: &NetComponent,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> bool {
    if layout_net_component_has_spice_net_issue(component, comparison) {
        return true;
    }
    layout_net_component_has_spice_device_issue(report, component.id, comparison)
}

pub(crate) fn layout_spice_issue_net_count(
    report: &ConnectivityReport,
    comparison: &SpiceConnectivityComparisonReport,
) -> usize {
    report
        .components
        .iter()
        .filter(|component| {
            layout_net_component_has_spice_issue(report, component, Some(comparison))
        })
        .count()
}

pub(crate) fn layout_spice_net_issue_net_count(
    report: &ConnectivityReport,
    comparison: &SpiceConnectivityComparisonReport,
) -> usize {
    report
        .components
        .iter()
        .filter(|component| layout_net_component_has_spice_net_issue(component, Some(comparison)))
        .count()
}

pub(crate) fn layout_spice_missing_issue_net_count(
    report: &ConnectivityReport,
    comparison: &SpiceConnectivityComparisonReport,
) -> usize {
    report
        .components
        .iter()
        .filter(|component| {
            layout_net_component_has_spice_missing_issue(report, component, Some(comparison))
        })
        .count()
}

pub(crate) fn layout_spice_extra_issue_net_count(
    report: &ConnectivityReport,
    comparison: &SpiceConnectivityComparisonReport,
) -> usize {
    report
        .components
        .iter()
        .filter(|component| {
            layout_net_component_is_spice_extra(report, component, Some(comparison))
        })
        .count()
}

pub(crate) fn layout_spice_device_issue_net_count(
    report: &ConnectivityReport,
    comparison: &SpiceConnectivityComparisonReport,
) -> usize {
    report
        .components
        .iter()
        .filter(|component| {
            layout_net_component_has_spice_device_issue(report, component.id, Some(comparison))
        })
        .count()
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
    layout_net_component_devices(report, component_id).len()
}

pub(crate) fn layout_net_component_device_summary(
    report: &ConnectivityReport,
    component_id: usize,
) -> String {
    let entries = report
        .devices
        .iter()
        .filter(|device| {
            device
                .terminals
                .iter()
                .any(|terminal| terminal.component == Some(component_id))
        })
        .map(|device| layout_net_component_device_text(device, component_id))
        .collect::<Vec<_>>();
    layout_net_component_detail_summary(entries, 64).unwrap_or_else(|| "None".to_string())
}

fn layout_net_component_detail_summary(
    mut entries: Vec<String>,
    max_chars: usize,
) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    entries.sort();
    entries.dedup();
    let mut preview = entries
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if entries.len() > 3 {
        preview.push_str(&format!(" +{}", entries.len() - 3));
    }
    Some(compact_button_label(&preview, max_chars))
}

pub(crate) fn layout_net_component_issue_summary(
    report: &ConnectivityReport,
    component_id: usize,
) -> Option<String> {
    let mut entries = Vec::new();
    for short in report
        .shorts
        .iter()
        .filter(|short| short.component == component_id)
    {
        let mut names = short.names.clone();
        names.sort();
        entries.push(format!("Short {}", names.join("/")));
    }
    for open in report
        .opens
        .iter()
        .filter(|open| open.components.contains(&component_id))
    {
        entries.push(format!("Open {}", open.name));
    }
    layout_net_component_detail_summary(entries, 64)
}

pub(crate) fn layout_net_component_open_peer_summary(
    report: &ConnectivityReport,
    component_id: usize,
) -> Option<String> {
    let mut entries = Vec::new();
    for open in &report.opens {
        if !open.components.contains(&component_id) {
            continue;
        }
        for peer_id in &open.components {
            if *peer_id == component_id {
                continue;
            }
            let peer_label = report
                .component(*peer_id)
                .map(connectivity_component_display_name)
                .unwrap_or_else(|| "missing".to_string());
            entries.push(format!("Open {} -> #{} {}", open.name, peer_id, peer_label));
        }
    }
    layout_net_component_detail_summary(entries, 64)
}

pub(crate) fn layout_net_component_device_peer_summary(
    report: &ConnectivityReport,
    component_id: usize,
) -> Option<String> {
    let mut entries = Vec::new();
    for device in layout_net_component_devices(report, component_id) {
        let device_label = layout_net_component_device_text(device, component_id);
        for terminal in &device.terminals {
            if terminal.component == Some(component_id) {
                continue;
            }
            let peer_label = match terminal.component {
                Some(peer_id) => report
                    .component(peer_id)
                    .map(|component| {
                        format!(
                            "#{} {}",
                            peer_id,
                            connectivity_component_display_name(component)
                        )
                    })
                    .unwrap_or_else(|| format!("#{peer_id} missing")),
                None => terminal
                    .net_name
                    .clone()
                    .unwrap_or_else(|| "unconnected".to_string()),
            };
            entries.push(format!(
                "{} -> {} {}",
                device_label, terminal.name, peer_label
            ));
        }
    }
    layout_net_component_detail_summary(entries, 64)
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
        .and_then(|occurrence| layout_component_for_occurrence_or_label(report, occurrence))
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
    let issue_summary = layout_net_component_issue_summary(report, component_id);
    let open_peer_summary = layout_net_component_open_peer_summary(report, component_id);
    let device_peer_summary = layout_net_component_device_peer_summary(report, component_id);
    let spice_status =
        layout_spice_component_status_label(component, app.layout_spice_comparison.as_ref());
    let spice_device_status = layout_spice_component_device_status_label(
        report,
        component_id,
        app.layout_spice_comparison.as_ref(),
    );
    let push_detail_rows = |rows: &mut Vec<(String, String)>| {
        if let Some(summary) = device_summary.as_ref() {
            rows.push(("Device Detail".to_string(), summary.clone()));
        }
        if let Some(summary) = issue_summary.as_ref() {
            rows.push(("Issue Detail".to_string(), summary.clone()));
        }
        if let Some(summary) = open_peer_summary.as_ref() {
            rows.push(("Open Peers".to_string(), summary.clone()));
        }
        if let Some(summary) = device_peer_summary.as_ref() {
            rows.push(("Device Peers".to_string(), summary.clone()));
        }
        if let Some(status) = spice_status.as_ref() {
            rows.push(("SPICE Net".to_string(), status.clone()));
        }
        if let Some(status) = spice_device_status.as_ref() {
            rows.push(("SPICE Device".to_string(), status.clone()));
        }
    };
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
            push_detail_rows(&mut rows);
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
            push_detail_rows(&mut rows);
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
            push_detail_rows(&mut rows);
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

pub(crate) fn layout_spice_component_device_status_label(
    report: &ConnectivityReport,
    component_id: usize,
    comparison: Option<&SpiceConnectivityComparisonReport>,
) -> Option<String> {
    let comparison = comparison?;
    let mut entries = Vec::new();
    for signature in &comparison.missing_layout_devices {
        if layout_spice_signature_mentions_component(report, component_id, signature) {
            entries.push(format!("Missing layout device {signature}"));
        }
    }
    for device in layout_net_component_devices(report, component_id) {
        for signature in &comparison.extra_layout_devices {
            let mos_dimension_keys = layout_spice_comparison_device_signature_keys(signature);
            if layout_model::connectivity::layout_spice_device_signature(
                report,
                device,
                &mos_dimension_keys,
            )
            .is_some_and(|device_signature| device_signature.eq_ignore_ascii_case(signature))
            {
                entries.push(format!("Extra layout device {signature}"));
            }
        }
    }
    layout_net_component_detail_summary(entries, 64)
}

pub(crate) fn layout_spice_signature_mentions_component(
    report: &ConnectivityReport,
    component_id: usize,
    signature: &str,
) -> bool {
    let Some(component) = report.component(component_id) else {
        return false;
    };
    let Some(signature_nets) = signature.split('|').nth(2) else {
        return false;
    };
    let signature_nets = signature_nets
        .split(',')
        .filter(|net| !net.trim().is_empty())
        .collect::<Vec<_>>();
    component.net_name.as_deref().is_some_and(|net_name| {
        signature_nets
            .iter()
            .any(|signature_net| signature_net.eq_ignore_ascii_case(net_name))
    })
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
        .enumerate()
        .filter_map(|(history_index, component_id)| {
            let component = report.component(*component_id)?;
            if search.as_deref().is_some_and(|query| {
                !layout_trace_history_entry_matches_search(&report, history_index, component, query)
            }) {
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

pub(crate) fn layout_trace_history_entry_matches_search(
    report: &ConnectivityReport,
    history_index: usize,
    component: &NetComponent,
    query_lower: &str,
) -> bool {
    let query_lower = query_lower.to_ascii_lowercase();
    layout_trace_history_entry_selector_matches_search(history_index, component, &query_lower)
        || layout_net_component_matches_search(report, component, &query_lower)
}

pub(crate) fn layout_trace_history_entry_selector_matches_search(
    history_index: usize,
    component: &NetComponent,
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
    let history_position = history_index + 1;
    let fields = vec![
        ("history", history_position.to_string()),
        ("history_index", history_position.to_string()),
        ("trace", history_position.to_string()),
        ("trace_index", history_position.to_string()),
        ("recent", history_position.to_string()),
        ("rank", history_position.to_string()),
        ("latest", (history_index == 0).to_string()),
        ("component", component.id.to_string()),
        ("component_id", component.id.to_string()),
        ("name", connectivity_component_display_name(component)),
    ];
    fields
        .iter()
        .any(|(key, value)| property_selector_key_value_matches(key, value, key_query, value_query))
}
