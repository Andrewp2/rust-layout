#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_cell_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let cells = layout_cell_browser_cells(app);
    let all_cells = ordered_layout_cells(&app.workspace.document);

    add_text(
        document,
        parent,
        "glassworks.layout.cell_browser.title",
        "Cell Browser",
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    add_text(
        document,
        parent,
        "glassworks.layout.library.title",
        "Library",
        text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    add_text(
        document,
        parent,
        "glassworks.layout.library.via_array.params",
        app.layout_via_array_parameter_status(),
        text_style(ui_scale.value(10.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for (slug, label) in [
        ("via_array.columns.dec", "Cols -"),
        ("via_array.columns.inc", "Cols +"),
        ("via_array.rows.dec", "Rows -"),
        ("via_array.rows.inc", "Rows +"),
        ("via_array.size.dec", "Size -"),
        ("via_array.size.inc", "Size +"),
        ("via_array.pitch.dec", "Pitch -"),
        ("via_array.pitch.inc", "Pitch +"),
    ] {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.library_macro.{slug}"),
            label,
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array.reset",
        "Reset Via Params",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array.create",
        "Create Via Array",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array.from_selection",
        "Via From Selection",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array.load_selected",
        "Load Macro Cell",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array.update_selected",
        "Update Macro Cell",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.make_static",
        "Make Static Cell",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.library_macro.via_array_3x3",
        "Quick Via 3x3",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        ui_scale,
    );
    add_text(
        document,
        parent,
        "glassworks.layout.library.presets.title",
        "Preset Slots",
        text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for (slug, label) in [
        ("catalog.export", "Export Catalog"),
        ("catalog.import", "Import Catalog"),
    ] {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.library_macro.{slug}"),
            label,
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }
    for slot in LAYOUT_LIBRARY_PRESET_SLOTS {
        add_text(
            document,
            parent,
            format!("glassworks.layout.library.preset.{slot}"),
            app.layout_via_array_library_preset_label(slot),
            text_style(ui_scale.value(10.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for (action, label) in [
            ("save", format!("Save P{slot}")),
            ("load", format!("Load P{slot}")),
            ("create", format!("Place P{slot}")),
            ("clear", format!("Clear P{slot}")),
        ] {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.library_macro.preset.{action}.{slot}"),
                label,
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    if all_cells.len() > 1 {
        for filter in LayoutCellBrowserFilter::ALL {
            add_button(
                document,
                parent,
                format!(
                    "glassworks.viewctl.layout.cell_browser_filter.{}",
                    filter.slug()
                ),
                filter.label(),
                app.layout_cell_browser_filter == filter,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        for sort in LayoutCellBrowserSort::ALL {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.cell_browser_sort.{}", sort.slug()),
                format!("Sort {}", sort.label()),
                app.layout_cell_browser_sort == sort,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    if cells.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.cell_browser.empty",
            "No matching cells",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.cell_browser.select_first",
        "View First Cell",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );

    for cell in cells {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.top_cell.{}", cell.id.0),
            layout_cell_browser_label(cell, app.workspace.document.top_cell),
            app.layout_view_top_cell == cell.id,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}
