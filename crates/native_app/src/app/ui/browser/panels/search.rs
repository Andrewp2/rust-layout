#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_browser_search(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    add_text(
        document,
        parent,
        "glassworks.layout.browser_search.title",
        "Browser Search",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.browser_search.start",
        layout_browser_search_button_label(app),
        app.layout_browser_search_active,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.browser_replace.start",
        layout_browser_replace_button_label(app),
        app.layout_browser_replace_active,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );

    if !app.layout_browser_search.trim().is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.browser_search.clear",
            "Clear Search",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.browser_replace.apply",
            "Apply Replace",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    if app.selected_layout_occurrence.is_some() && !app.layout_browser_search.trim().is_empty() {
        if !app.layout_browser_replace.trim().is_empty() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.shape_property.apply_selected",
                "Set Shape Prop",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.shape_property.remove_selected",
            "Remove Shape Prop",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    if app.selected_layout_instance_context().is_some()
        && !app.layout_browser_search.trim().is_empty()
    {
        if !app.layout_browser_replace.trim().is_empty() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.instance_property.apply_selected",
                "Set Instance Prop",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.instance_property.remove_selected",
            "Remove Instance Prop",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    if app
        .workspace
        .document
        .cell(app.layout_view_top_cell)
        .is_some()
        && !app.layout_browser_search.trim().is_empty()
    {
        if !app.layout_browser_replace.trim().is_empty() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.cell_property.apply_current",
                "Set Cell Prop",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.cell_property.remove_current",
            "Remove Cell Prop",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }

    for scope in LayoutBrowserReplaceScope::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.browser_replace_scope.{}",
                scope.slug()
            ),
            format!("Replace {}", scope.label()),
            app.layout_browser_replace_scope == scope,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }

    for columns in LayoutBrowserColumnSet::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.browser_columns.{}",
                columns.slug()
            ),
            format!("Cols {}", columns.label()),
            app.layout_browser_columns == columns,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
}

pub(crate) fn layout_browser_replace_button_label(app: &GlassworksApp) -> String {
    if app.layout_browser_replace.is_empty() {
        if app.layout_browser_replace_active {
            "Replace: |".to_string()
        } else {
            "Replace".to_string()
        }
    } else {
        let suffix = if app.layout_browser_replace_active {
            "|"
        } else {
            ""
        };
        format!(
            "Replace: {}{suffix}",
            compact_button_label(&app.layout_browser_replace, 22)
        )
    }
}

pub(crate) fn layout_browser_search_button_label(app: &GlassworksApp) -> String {
    let query = app.layout_browser_search.trim();
    if query.is_empty() {
        if app.layout_browser_search_active {
            "Search: |".to_string()
        } else {
            "Search".to_string()
        }
    } else {
        let suffix = if app.layout_browser_search_active {
            "|"
        } else {
            ""
        };
        format!("Search: {}{suffix}", compact_button_label(query, 22))
    }
}
