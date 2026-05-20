#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_shape_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_shape_browser_entries(app);
    if entries.is_empty()
        && app.layout_shape_browser_filter == LayoutShapeBrowserFilter::All
        && app.layout_browser_search.trim().is_empty()
    {
        return;
    }

    add_text(
        document,
        parent,
        "glassworks.layout.shape_browser.title",
        layout_shape_browser_title(app, entries.len()),
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    for filter in LayoutShapeBrowserFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.shape_browser_filter.{}",
                filter.slug()
            ),
            layout_shape_browser_filter_button_label(app, filter),
            app.layout_shape_browser_filter == filter,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    for sort in LayoutShapeBrowserSort::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.shape_browser_sort.{}",
                sort.slug()
            ),
            format!("Sort {}", sort.label()),
            app.layout_shape_browser_sort == sort,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !entries.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.shape_browser.select_first",
            "Select First Shape",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.shape_browser.empty",
            "No matching shapes",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    for (occurrence, label) in entries {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.occurrence.{}",
                layout_occurrence_action_key(&occurrence)
            ),
            label,
            app.selected_layout_occurrence.as_ref() == Some(&occurrence),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
