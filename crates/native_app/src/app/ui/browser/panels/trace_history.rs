#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_trace_history(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_trace_history_entries(app);
    if entries.is_empty()
        && (app.layout_trace_history.is_empty() || app.layout_browser_search.trim().is_empty())
    {
        return;
    }
    let selected_component = app
        .connectivity_report()
        .ok()
        .and_then(|report| selected_layout_net_component_id(app, &report));

    add_text(
        document,
        parent,
        "glassworks.layout.trace_history.title",
        "Trace History",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    if !app.layout_trace_history.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.trace_history.clear",
            "Clear History",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !entries.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.trace_history.select_first",
            "Select First History Net",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.trace_history.empty",
            "No trace history matches search",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
        );
        return;
    }

    for (component_id, label) in entries {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.trace_history.{component_id}"),
            label,
            selected_component == Some(component_id),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.trace_history.remove.{component_id}"),
            format!("Remove Net {component_id}"),
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
}
