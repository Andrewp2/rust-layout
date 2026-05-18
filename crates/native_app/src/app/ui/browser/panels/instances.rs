#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_instance_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_instance_browser_entries(app);
    let search_active = !app.layout_browser_search.trim().is_empty();
    if entries.is_empty()
        && !search_active
        && app.layout_instance_browser_scope == LayoutInstanceBrowserScope::CurrentCell
        && app.layout_instance_browser_filter == LayoutInstanceBrowserFilter::All
    {
        return;
    }
    let selected = selected_layout_instance_key(app);

    add_text(
        document,
        parent,
        "glassworks.layout.instance_browser.title",
        "Instance Browser",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    for scope in LayoutInstanceBrowserScope::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.instance_browser_scope.{}",
                scope.slug()
            ),
            scope.label(),
            app.layout_instance_browser_scope == scope,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    for filter in LayoutInstanceBrowserFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.instance_browser_filter.{}",
                filter.slug()
            ),
            filter.label(),
            app.layout_instance_browser_filter == filter,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }

    for sort in LayoutInstanceBrowserSort::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.instance_browser_sort.{}",
                sort.slug()
            ),
            format!("Sort {}", sort.label()),
            app.layout_instance_browser_sort == sort,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.instance_browser.empty",
            "No matching instances",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.instance_browser.select_first",
        "Select First Instance",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );

    for (parent_cell, instance_id, label) in entries {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.instance.{}",
                layout_instance_action_key(parent_cell, instance_id)
            ),
            label,
            selected == Some((parent_cell, instance_id)),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
