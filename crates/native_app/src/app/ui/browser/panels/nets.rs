#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_net_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_net_browser_entries(app);
    let trace_point_entries = layout_trace_point_entries(app);
    let trace_path_entries = layout_trace_path_segment_entries(app);
    let trace_path_net_entries = layout_trace_path_segment_net_entries(app);
    let trace_path_endpoint_net_entries = layout_trace_path_endpoint_net_entries(app);
    let trace_path_segment_endpoint_entries = layout_trace_path_segment_endpoint_entries(app);
    let trace_path_blocker = layout_trace_path_first_blocker(app);
    let trace_path_blocker_endpoint_entries = layout_trace_path_blocker_endpoint_entries(app);
    let trace_path_connected_net = layout_trace_path_connected_component(app);
    let has_trace_path_entries = !trace_path_entries.is_empty();
    if entries.is_empty()
        && trace_point_entries.is_empty()
        && !has_trace_path_entries
        && trace_path_blocker.is_none()
        && app.layout_net_browser_filter == LayoutNetBrowserFilter::All
        && app.layout_browser_search.trim().is_empty()
    {
        return;
    }
    let report = app.connectivity_report().ok();
    let selected_component = report
        .as_ref()
        .and_then(|report| selected_layout_net_component_id(app, report));
    let search = layout_browser_search_query_lower(app);

    add_text(
        document,
        parent,
        "glassworks.layout.net_browser.title",
        "Net Browser",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.netlist_export",
        "Export Netlist",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.netlist_import",
        "Import Netlist",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.spice_netlist_export",
        "Export SPICE Netlist",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.spice_schematic_compare",
        "Compare SPICE Schematic",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.trace_state_export",
        "Export Trace State",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.trace_state_import",
        "Import Trace State",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.l2n_database_export",
        "Export L2N DB",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.l2n_database_import",
        "Import L2N DB",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    if let Some(report) = report.as_ref() {
        add_text(
            document,
            parent,
            "glassworks.layout.netlist_summary.title",
            "Netlist Summary",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (slug, label, value) in layout_netlist_summary_rows(report) {
            add_text(
                document,
                parent,
                format!("glassworks.layout.netlist_summary.{slug}"),
                format!("{label}: {value}"),
                text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(18.0))),
            );
        }
        let spice_compare_rows = layout_spice_comparison_summary_rows(app, Some(report));
        if !spice_compare_rows.is_empty() {
            add_text(
                document,
                parent,
                "glassworks.layout.spice_compare_summary.title",
                "SPICE Compare",
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (slug, label, value) in spice_compare_rows {
                add_text(
                    document,
                    parent,
                    format!("glassworks.layout.spice_compare_summary.{slug}"),
                    format!("{label}: {value}"),
                    text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(18.0))),
                );
            }
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.spice_compare.clear",
                "Clear SPICE Compare",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        let connectivity_source_rows = layout_connectivity_source_summary_rows(app);
        if !connectivity_source_rows.is_empty() {
            add_text(
                document,
                parent,
                "glassworks.layout.connectivity_source.title",
                "Connectivity Source",
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (slug, label, value) in connectivity_source_rows {
                add_text(
                    document,
                    parent,
                    format!("glassworks.layout.connectivity_source.{slug}"),
                    format!("{label}: {value}"),
                    text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(18.0))),
                );
            }
        }
        let trace_state_rows = layout_trace_state_summary_rows(app, report);
        if !trace_state_rows.is_empty() {
            add_text(
                document,
                parent,
                "glassworks.layout.trace_state_summary.title",
                "Trace State",
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (slug, label, value) in trace_state_rows {
                add_text(
                    document,
                    parent,
                    format!("glassworks.layout.trace_state_summary.{slug}"),
                    format!("{label}: {value}"),
                    text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(18.0))),
                );
            }
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.trace_state.clear",
                "Clear Trace State",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        let device_entries = layout_netlist_device_entries(report, search.as_deref());
        let has_device_entries = !device_entries.is_empty();
        if has_device_entries {
            add_text(
                document,
                parent,
                "glassworks.layout.netlist_devices.title",
                "Extracted Devices",
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (device_id, label) in device_entries {
                add_button(
                    document,
                    parent,
                    format!("glassworks.viewctl.layout.netlist_device.{device_id}"),
                    label,
                    false,
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                    ui_scale,
                );
            }
        }
        let issue_entries = layout_netlist_issue_entries(report, search.as_deref());
        let has_issue_entries = !issue_entries.is_empty();
        if has_issue_entries {
            add_text(
                document,
                parent,
                "glassworks.layout.netlist_issues.title",
                "Connectivity Issues",
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (issue_key, label) in issue_entries {
                add_button(
                    document,
                    parent,
                    format!("glassworks.viewctl.layout.netlist_issue.{issue_key}"),
                    label,
                    false,
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                    ui_scale,
                );
            }
        }
        let spice_issue_entries = app
            .layout_spice_comparison
            .as_ref()
            .map(|comparison| {
                layout_spice_comparison_issue_entries_for_filter(
                    comparison,
                    search.as_deref(),
                    app.layout_net_browser_filter,
                )
            })
            .unwrap_or_default();
        let has_spice_issue_entries = !spice_issue_entries.is_empty();
        let spice_issue_entry_count = spice_issue_entries.len();
        if has_spice_issue_entries {
            let issue_row_label = if spice_issue_entry_count == 1 {
                "row"
            } else {
                "rows"
            };
            let spice_issue_title = match app.layout_net_browser_filter {
                LayoutNetBrowserFilter::SpiceMissing
                | LayoutNetBrowserFilter::SpiceExtra
                | LayoutNetBrowserFilter::SpiceNets
                | LayoutNetBrowserFilter::SpiceDevices => format!(
                    "SPICE Compare Issues - {} ({spice_issue_entry_count} {issue_row_label})",
                    app.layout_net_browser_filter.label(),
                ),
                _ => {
                    format!("SPICE Compare Issues ({spice_issue_entry_count} {issue_row_label})")
                }
            };
            add_text(
                document,
                parent,
                "glassworks.layout.spice_compare_issues.title",
                spice_issue_title,
                text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
            for (issue_key, label) in spice_issue_entries {
                add_button(
                    document,
                    parent,
                    format!("glassworks.viewctl.layout.spice_compare_issue.{issue_key}"),
                    label,
                    false,
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                    ui_scale,
                );
            }
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.spice_compare_issue.select_first",
                "Select First SPICE Issue",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        if has_device_entries || has_issue_entries || has_spice_issue_entries {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.netlist.select_first",
                "Select First Netlist Item",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
    }
    if !trace_point_entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.trace_points.title",
            "Trace Points",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (index, label) in trace_point_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_point.{index}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_point.remove.{index}"),
                format!("Remove Trace Point {}", index + 1),
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.trace_point.select_first",
            "Select First Trace Point",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }
    if has_trace_path_entries
        || trace_path_blocker.is_some()
        || trace_path_connected_net.is_some()
        || !trace_path_endpoint_net_entries.is_empty()
    {
        add_text(
            document,
            parent,
            "glassworks.layout.trace_path_segments.title",
            "Trace Path Segments",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (index, label) in trace_path_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_path_segment.{index}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        for (index, label) in trace_path_net_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_path_segment_net.{index}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        if trace_path_connected_net.is_some() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.trace_path.select_net",
                "Select Trace Path Net",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        for (key, label) in trace_path_endpoint_net_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_path_endpoint.{key}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        for (key, label) in trace_path_segment_endpoint_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_path_segment_endpoint.{key}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        if has_trace_path_entries {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.trace_path_segment.select_first",
                "Focus First Trace Segment",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        if trace_path_blocker.is_some() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.trace_path_segment.focus_blocker",
                "Focus Trace Blocker",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
                ui_scale,
            );
        }
        for (key, label) in trace_path_blocker_endpoint_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.trace_path_blocker_endpoint.{key}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }
    for mode in LayoutTraceHighlightMode::ALL {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.trace_highlight.{}", mode.slug()),
            format!("Highlight {}", mode.label()),
            app.layout_trace_highlight_mode == mode,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if selected_component.is_some() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.net_browser.clear_selected",
            "Clear Selected Net",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    let source_objects = selected_layout_net_component_source_objects(app);
    if !source_objects.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_objects.title",
            "Net Objects",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (occurrence, label) in source_objects {
            add_button(
                document,
                parent,
                format!(
                    "glassworks.viewctl.layout.net_component_source.{}",
                    layout_occurrence_action_key(&occurrence)
                ),
                label,
                app.selected_layout_occurrence.as_ref() == Some(&occurrence),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let label_objects = selected_layout_net_component_label_objects(app);
    if !label_objects.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_labels.title",
            "Net Labels",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (occurrence, label) in label_objects {
            add_button(
                document,
                parent,
                format!(
                    "glassworks.viewctl.layout.net_component_label.{}",
                    layout_occurrence_action_key(&occurrence)
                ),
                label,
                app.selected_layout_occurrence.as_ref() == Some(&occurrence),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let device_objects = selected_layout_net_component_device_objects(app);
    if !device_objects.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_devices.title",
            "Net Devices",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (device_id, label) in device_objects {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.net_component_device.{device_id}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let device_peers = selected_layout_net_component_device_peers(app);
    if !device_peers.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_device_peers.title",
            "Device Peer Nets",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (peer_key, label) in device_peers {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.net_component_device_peer.{peer_key}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let issue_objects = selected_layout_net_component_issue_objects(app);
    if !issue_objects.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_issues.title",
            "Net Issues",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (issue_key, label) in issue_objects {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.net_component_issue.{issue_key}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let open_peers = selected_layout_net_component_open_peers(app);
    if !open_peers.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_component_open_peers.title",
            "Open Peers",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (component_id, label) in open_peers {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.net_component_open_peer.{component_id}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    for filter in LayoutNetBrowserFilter::ALL {
        let label = layout_net_browser_filter_button_label(
            report.as_ref(),
            filter,
            app.layout_spice_comparison.as_ref(),
        );
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.net_browser_filter.{}",
                filter.slug()
            ),
            label,
            app.layout_net_browser_filter == filter,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    for sort in LayoutNetBrowserSort::ALL {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.net_browser_sort.{}", sort.slug()),
            format!("Sort {}", sort.label()),
            app.layout_net_browser_sort == sort,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !entries.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.net_browser.select_first",
            "Select First Net",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.net_browser.empty",
            "No matching nets",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    for (component_id, label) in entries {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.net_component.{component_id}"),
            label,
            selected_component == Some(component_id),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
