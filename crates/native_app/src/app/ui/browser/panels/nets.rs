#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_net_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_net_browser_entries(app);
    if entries.is_empty()
        && app.layout_net_browser_filter == LayoutNetBrowserFilter::All
        && app.layout_browser_search.trim().is_empty()
    {
        return;
    }
    let selected_component = app.connectivity_report().ok().and_then(|report| {
        app.selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| report.component_for_occurrence(occurrence))
    });

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

    for filter in LayoutNetBrowserFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.net_browser_filter.{}",
                filter.slug()
            ),
            filter.label(),
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
