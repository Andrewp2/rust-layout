#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_measurement_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_measurement_entries(app);
    if entries.is_empty()
        && app.layout_browser_search.trim().is_empty()
        && app.layout_measurement_filter == LayoutMeasurementFilter::All
        && app.active_tool != ToolMode::Measure
    {
        return;
    }

    add_text(
        document,
        parent,
        "glassworks.layout.measurement_browser.title",
        layout_measurement_browser_title(app, entries.len()),
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    add_text(
        document,
        parent,
        "glassworks.layout.measurement_browser.mode",
        format!("Ruler mode: {}", app.layout_measurement_mode.label()),
        text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for mode in MeasurementMode::ALL {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.measurement_mode.{}", mode.slug()),
            mode.label(),
            app.layout_measurement_mode == mode,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    for filter in LayoutMeasurementFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.measurement_filter.{}",
                filter.slug()
            ),
            format!(
                "Filter {}",
                layout_measurement_filter_button_label(app, filter)
            ),
            app.layout_measurement_filter == filter,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    for sort in LayoutMeasurementBrowserSort::ALL {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.measurement_sort.{}", sort.slug()),
            format!("Sort {}", sort.label()),
            app.layout_measurement_browser_sort == sort,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    if layout_visible_measurement_count(app) > 0 {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.measurements.clear",
            "Clear Visible Measurements",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }
    if selected_visible_layout_measurement(app).is_some() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.measurements.focus_selected",
            "Focus Selected Measurement",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.measurements.delete_selected",
            "Delete Selected Measurement",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !entries.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.measurement_browser.select_first",
            "Select First Measurement",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.measurement_browser.empty",
            "No matching measurements",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    for (occurrence, _, shape) in entries {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.measurement.{}",
                layout_occurrence_action_key(&occurrence)
            ),
            layout_measurement_browser_label(app, &occurrence, &shape),
            app.selected_layout_occurrence.as_ref() == Some(&occurrence),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
