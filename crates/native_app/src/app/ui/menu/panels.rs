#![allow(unused_imports)]
use super::*;

pub(crate) fn menu_panel_items_height(
    items: &[MenuPanelItem],
    ui_scale: UiScale,
    item_height: f32,
    gap: f32,
) -> f32 {
    items
        .iter()
        .map(|item| item.height(ui_scale, item_height))
        .sum::<f32>()
        + items.len().saturating_sub(1) as f32 * gap
}

pub(crate) fn add_menu_panel_item(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    menu_slug: &str,
    index: usize,
    item: &MenuPanelItem,
    item_width: f32,
    item_height: f32,
    ui_scale: UiScale,
) {
    match item {
        MenuPanelItem::Action {
            action,
            label,
            selected,
            enabled,
            ..
        } => {
            add_menu_item_button(
                document,
                parent,
                format!("glassworks.menu.item.{action}"),
                label,
                *selected,
                *enabled,
                layout::size(layout::px(item_width), layout::px(item_height)),
                ui_scale,
            );
        }
        MenuPanelItem::Label(label) => {
            add_text(
                document,
                parent,
                format!("glassworks.menu.label.{menu_slug}.{index}"),
                label,
                text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                layout::size(layout::px(item_width), layout::px(ui_scale.value(22.0))),
            );
        }
        MenuPanelItem::Separator => {
            document.add_child(
                parent,
                UiNode::container(
                    format!("glassworks.menu.separator.{menu_slug}.{index}"),
                    layout::size(layout::px(item_width), layout::px(ui_scale.value(1.0))),
                )
                .with_visual(UiVisual::panel(COLOR_PANEL_STROKE, None, 0.0)),
            );
        }
    }
}

pub(crate) fn add_file_menu_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport_width: f32,
    ui_scale: UiScale,
) {
    let items = file_menu_items(app);
    let item_width = ui_scale.value(190.0);
    let item_height = ui_scale.value(16.0);
    let gap = ui_scale.value(1.0);
    let split_at = (items.len() + 1) / 2;
    let first_column = &items[..split_at];
    let second_column = &items[split_at..];
    let first_column_height = menu_panel_items_height(first_column, ui_scale, item_height, gap);
    let second_column_height = menu_panel_items_height(second_column, ui_scale, item_height, gap);
    let panel_width = item_width * 2.0 + gap + ui_scale.value(12.0);
    let panel_height = first_column_height.max(second_column_height) + ui_scale.value(12.0);
    let left = menu_popup_left(AppMenu::File, panel_width, viewport_width, ui_scale);

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.menu_panel.file",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(
                            layout::row(),
                            layout::px(panel_width),
                            layout::px(panel_height),
                        ),
                        left,
                        ui_scale.value(30.0),
                    ),
                    gap,
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(80);

    for (column_index, column_items) in [first_column, second_column].into_iter().enumerate() {
        let column_height = if column_index == 0 {
            first_column_height
        } else {
            second_column_height
        };
        let column = document.add_child(
            panel,
            UiNode::container(
                format!("glassworks.menu_panel.file.column.{column_index}"),
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::px(item_width),
                        layout::px(column_height),
                    ),
                    gap,
                ),
            ),
        );
        let index_offset = if column_index == 0 { 0 } else { split_at };
        for (index, item) in column_items.iter().enumerate() {
            add_menu_panel_item(
                document,
                column,
                "file",
                index_offset + index,
                item,
                item_width,
                item_height,
                ui_scale,
            );
        }
    }
}

pub(crate) fn add_menu_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    menu: AppMenu,
    active_view: StartupView,
    app: &GlassworksApp,
    viewport_width: f32,
    ui_scale: UiScale,
) {
    if menu == AppMenu::View {
        add_view_menu_panel(document, parent, active_view, app, viewport_width, ui_scale);
        return;
    }
    if menu == AppMenu::File {
        add_file_menu_panel(document, parent, app, viewport_width, ui_scale);
        return;
    }

    let editing_mode = active_view == StartupView::Layout2d;
    let can_delete = editing_mode && app.selected_layout_shape.is_some();
    let items: Vec<MenuPanelItem> = match menu {
        AppMenu::File => file_menu_items(app),
        AppMenu::Edit => vec![
            menu_action_enabled("edit.undo", "Undo", editing_mode),
            menu_action_enabled("edit.redo", "Redo", editing_mode),
            MenuPanelItem::Separator,
            menu_action_enabled("edit.copy", "Copy", editing_mode),
            menu_action_enabled("edit.copy_active_layer", "Copy Active Layer", editing_mode),
            menu_action_enabled("edit.paste", "Paste", editing_mode),
            menu_action_enabled("edit.duplicate", "Duplicate", editing_mode),
            menu_action_enabled("edit.delete", "Delete", can_delete),
            MenuPanelItem::Separator,
            menu_action_enabled("edit.make_cell", "Make Cell", editing_mode),
            menu_action_enabled("edit.duplicate_cell", "Duplicate Cell", editing_mode),
            menu_action_enabled(
                "edit.delete_unused_cells",
                "Delete Unused Cells",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.delete_cell_shallow",
                "Shallow Delete Cell",
                editing_mode,
            ),
            menu_action_enabled("edit.delete_cell_deep", "Deep Delete Cell", editing_mode),
            menu_action_enabled(
                "edit.delete_cell_complete",
                "Complete Delete Cell",
                editing_mode,
            ),
            menu_action_enabled("edit.make_variant", "Make Variant", editing_mode),
            menu_action_enabled(
                "edit.make_child_variants",
                "Make Child Variants",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.make_descendant_child_variants",
                "Make Descendant Variants",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.make_document_child_variants",
                "Make Document Variants",
                editing_mode,
            ),
            menu_action_enabled("edit.flatten_instance", "Flatten Instance", editing_mode),
            menu_action_enabled(
                "edit.flatten_instance_one",
                "Flatten Instance 1 Level",
                editing_mode,
            ),
            menu_action_enabled("edit.flatten_cell", "Flatten Cell", editing_mode),
            menu_action_enabled(
                "edit.flatten_cell_one",
                "Flatten Cell 1 Level",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.flatten_descendant_cells",
                "Flatten Descendant Cells",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.flatten_descendant_cells_one",
                "Flatten Descendant Cells 1 Level",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.flatten_document_cells",
                "Flatten Document Cells",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.flatten_document_cells_one",
                "Flatten Document Cells 1 Level",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.cell_origin_selection",
                "Origin to Selection",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.cell_origin_descendant_leaves",
                "Origin Descendant Leaves",
                editing_mode,
            ),
            menu_action_enabled("edit.move_shape_up", "Move Shape Up", editing_mode),
            menu_action_enabled("edit.move_instance_up", "Move Instance Up", editing_mode),
            menu_action_enabled("edit.resolve_array", "Resolve Array", editing_mode),
            menu_action_enabled(
                "edit.resolve_cell_arrays",
                "Resolve Cell Arrays",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.resolve_descendant_cell_arrays",
                "Resolve Descendant Arrays",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.resolve_document_arrays",
                "Resolve Document Arrays",
                editing_mode,
            ),
            menu_action_enabled("edit.merge_layer_rects", "Merge Layer Rects", editing_mode),
            menu_action_enabled(
                "edit.layer_and_selection",
                "Layer AND Selection",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_or_selection",
                "Layer OR Selection",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_not_selection",
                "Layer NOT Selection",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.selection_not_layer",
                "Selection NOT Layer",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_xor_selection",
                "Layer XOR Selection",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_and_clipboard",
                "Layer AND Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_or_clipboard",
                "Layer OR Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_not_clipboard",
                "Layer NOT Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.layer_xor_clipboard",
                "Layer XOR Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.shape_and_clipboard",
                "Shape AND Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.shape_or_clipboard",
                "Shape OR Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.shape_not_clipboard",
                "Shape NOT Clipboard",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.shape_xor_clipboard",
                "Shape XOR Clipboard",
                editing_mode,
            ),
            menu_action_enabled("edit.create_clip_cell", "Create Clip Cell", editing_mode),
            menu_action_enabled(
                "edit.create_clip_cell_clipboard",
                "Clip Cell Clipboard",
                editing_mode,
            ),
            menu_action_enabled("edit.rotate90", "Rotate 90", editing_mode),
            menu_action_enabled("edit.mirror_x", "Mirror X", editing_mode),
            menu_action_enabled("edit.mirror_y", "Mirror Y", editing_mode),
            menu_action_enabled("edit.grow", "Grow Shape", editing_mode),
            menu_action_enabled("edit.shrink", "Shrink Shape", editing_mode),
            menu_action_enabled("edit.grow_x", "Grow Shape X", editing_mode),
            menu_action_enabled("edit.shrink_x", "Shrink Shape X", editing_mode),
            menu_action_enabled("edit.grow_y", "Grow Shape Y", editing_mode),
            menu_action_enabled("edit.shrink_y", "Shrink Shape Y", editing_mode),
            menu_action_enabled("edit.chamfer_corners", "Chamfer Corners", editing_mode),
            menu_action_enabled("edit.round_corners", "Round Corners", editing_mode),
            menu_action_enabled("edit.chamfer_layer_corners", "Chamfer Layer", editing_mode),
            menu_action_enabled("edit.round_layer_corners", "Round Layer", editing_mode),
            menu_action_enabled("edit.grow_layer", "Grow Layer", editing_mode),
            menu_action_enabled("edit.shrink_layer", "Shrink Layer", editing_mode),
            menu_action_enabled("edit.grow_layer_x", "Grow Layer X", editing_mode),
            menu_action_enabled("edit.shrink_layer_x", "Shrink Layer X", editing_mode),
            menu_action_enabled("edit.grow_layer_y", "Grow Layer Y", editing_mode),
            menu_action_enabled("edit.shrink_layer_y", "Shrink Layer Y", editing_mode),
            menu_action_enabled("edit.align_left", "Align Left", editing_mode),
            menu_action_enabled("edit.align_right", "Align Right", editing_mode),
            menu_action_enabled("edit.align_top", "Align Top", editing_mode),
            menu_action_enabled("edit.align_bottom", "Align Bottom", editing_mode),
            menu_action_enabled("edit.align_center_x", "Align Center X", editing_mode),
            menu_action_enabled("edit.align_center_y", "Align Center Y", editing_mode),
            menu_action_enabled("edit.align_origin_x", "Align Origin X", editing_mode),
            menu_action_enabled("edit.align_origin_y", "Align Origin Y", editing_mode),
            menu_action_enabled("edit.align_layer_left", "Align Layer Left", editing_mode),
            menu_action_enabled("edit.align_layer_right", "Align Layer Right", editing_mode),
            menu_action_enabled("edit.align_layer_top", "Align Layer Top", editing_mode),
            menu_action_enabled(
                "edit.align_layer_bottom",
                "Align Layer Bottom",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.align_layer_center_x",
                "Align Layer Center X",
                editing_mode,
            ),
            menu_action_enabled(
                "edit.align_layer_center_y",
                "Align Layer Center Y",
                editing_mode,
            ),
        ],
        AppMenu::View => unreachable!("view menu is handled by add_view_menu_panel"),
        AppMenu::Bookmarks => {
            let mut items = vec![
                menu_action_enabled(
                    "bookmarks.previous_layout_view",
                    "Previous Layout View",
                    editing_mode && app.layout_previous_view.is_some(),
                ),
                menu_action_enabled("bookmarks.origin", "Origin", editing_mode),
                menu_action_enabled("bookmarks.bounds", "Layout Bounds", editing_mode),
                menu_action_enabled(
                    "bookmarks.selection",
                    "Selection",
                    editing_mode && app.selected_layout_occurrence.is_some(),
                ),
                MenuPanelItem::Separator,
                menu_action_enabled(
                    "bookmarks.export_layout_views",
                    "Export Views",
                    editing_mode && !app.layout_view_bookmarks.is_empty(),
                ),
                menu_action_enabled(
                    "bookmarks.import_layout_views",
                    "Import Views",
                    editing_mode,
                ),
                menu_action_enabled(
                    "bookmarks.clear_layout_views",
                    "Clear All Views",
                    editing_mode && !app.layout_view_bookmarks.is_empty(),
                ),
                MenuPanelItem::Separator,
                MenuPanelItem::Label("Save View".to_string()),
            ];
            for slot in LAYOUT_VIEW_BOOKMARK_SLOTS {
                let label = compact_button_label(&app.layout_view_bookmark_name(slot), 18);
                items.push(menu_action_enabled(
                    format!("bookmarks.save_layout_view.{slot}"),
                    format!("Save {label}"),
                    editing_mode,
                ));
            }
            items.push(MenuPanelItem::Separator);
            items.push(MenuPanelItem::Label("Restore View".to_string()));
            for slot in LAYOUT_VIEW_BOOKMARK_SLOTS {
                let label = compact_button_label(&app.layout_view_bookmark_name(slot), 18);
                items.push(menu_action_enabled(
                    format!("bookmarks.restore_layout_view.{slot}"),
                    format!("Restore {label}"),
                    editing_mode && app.layout_view_bookmarks.contains_key(&slot),
                ));
            }
            items.push(MenuPanelItem::Separator);
            items.push(MenuPanelItem::Label("Clear View".to_string()));
            for slot in LAYOUT_VIEW_BOOKMARK_SLOTS {
                let label = compact_button_label(&app.layout_view_bookmark_name(slot), 18);
                items.push(menu_action_enabled(
                    format!("bookmarks.clear_layout_view.{slot}"),
                    format!("Clear {label}"),
                    editing_mode && app.layout_view_bookmarks.contains_key(&slot),
                ));
            }
            items
        }
        AppMenu::Display => vec![
            MenuPanelItem::selected("display.grid2d", "2D grid", app.show_grid),
            MenuPanelItem::selected("display.grid3d", "3D grid", app.show_3d_grid),
            MenuPanelItem::selected("display.origin", "Origin crosshair", app.show_origin_marker),
            MenuPanelItem::selected("display.drc", "DRC overlay", app.show_drc_overlay),
            MenuPanelItem::selected(
                "display.reference_images",
                "Reference images",
                app.show_reference_images,
            ),
            MenuPanelItem::selected("display.inspector", "Details panel", app.show_inspector),
            MenuPanelItem::selected(
                "display.secondary_panel",
                app.secondary_panel_label(),
                app.show_layers,
            ),
            MenuPanelItem::Separator,
            MenuPanelItem::Label("Theme".to_string()),
            MenuPanelItem::selected(
                "display.theme.dark",
                "Dark",
                app.theme_preference == options::ThemePreference::Dark,
            ),
            MenuPanelItem::selected(
                "display.theme.light",
                "Light",
                app.theme_preference == options::ThemePreference::Light,
            ),
            MenuPanelItem::selected(
                "display.theme.system",
                "System",
                app.theme_preference == options::ThemePreference::System,
            ),
            MenuPanelItem::Separator,
            MenuPanelItem::Label("Units".to_string()),
            MenuPanelItem::selected(
                "display.units.auto",
                UnitDisplay::Auto.label(),
                app.unit_display == UnitDisplay::Auto,
            ),
            MenuPanelItem::selected(
                "display.units.nanometers",
                UnitDisplay::Nanometers.label(),
                app.unit_display == UnitDisplay::Nanometers,
            ),
            MenuPanelItem::selected(
                "display.units.microns",
                UnitDisplay::Microns.label(),
                app.unit_display == UnitDisplay::Microns,
            ),
            MenuPanelItem::selected(
                "display.units.dbu",
                UnitDisplay::Dbu.label(),
                app.unit_display == UnitDisplay::Dbu,
            ),
            MenuPanelItem::Separator,
            MenuPanelItem::action("display.options", "Options..."),
        ],
        AppMenu::Options => vec![
            MenuPanelItem::selected("options.snap", "Snap", app.snap_enabled),
            MenuPanelItem::action("display.options", "Options..."),
        ],
        AppMenu::Tools => {
            let mut items = vec![
                MenuPanelItem::action("tools.palette", "Command Palette..."),
                MenuPanelItem::Separator,
            ];
            if editing_mode {
                items.push(MenuPanelItem::Label("Layout Tools".to_string()));
                for tool in ToolMode::ALL {
                    items.push(MenuPanelItem::selected(
                        format!("tool.{}", tool.slug()),
                        tool.label(),
                        app.active_tool == tool,
                    ));
                }
                items.push(MenuPanelItem::Separator);
            }
            items.push(MenuPanelItem::Label("Technology".to_string()));
            for (index, technology) in layout_model::builtin_technologies().into_iter().enumerate()
            {
                items.push(MenuPanelItem::selected(
                    format!("tools.technology.{index}"),
                    display_technology_name(&technology.name),
                    index == app.layout_active_technology_index,
                ));
            }
            items.push(MenuPanelItem::Separator);
            items.push(MenuPanelItem::action("tools.runroute", "Run Route"));
            items.push(MenuPanelItem::action("tools.tracebetween", "Trace Path"));
            items.push(MenuPanelItem::action("tools.rundrc", "Run DRC"));
            items.push(menu_action_enabled(
                "tools.rundrc_region",
                "Run DRC In Selection",
                editing_mode && app.selected_layout_shape.is_some(),
            ));
            items.push(MenuPanelItem::action(
                "tools.rundrc_cell",
                "Run DRC In Cell",
            ));
            items.push(MenuPanelItem::action(
                "layout.drc_report_export",
                "Export DRC Report",
            ));
            items.push(MenuPanelItem::action(
                "layout.drc_report_import",
                "Import DRC Report",
            ));
            items.push(MenuPanelItem::action(
                "layout.drc_report_database_export",
                "Export DRC Report Database",
            ));
            items.push(MenuPanelItem::action(
                "layout.drc_report_database_import",
                "Import DRC Report Database",
            ));
            items.push(MenuPanelItem::action(
                "layout.drc_report_database_append",
                "Append DRC Report Database",
            ));
            items.push(MenuPanelItem::action(
                "layout.klayout_rdb_export",
                "Export KLayout RDB",
            ));
            items.push(MenuPanelItem::action(
                "layout.klayout_rdb_import",
                "Import KLayout RDB",
            ));
            items.push(MenuPanelItem::action(
                "layout.klayout_rdb_append",
                "Append KLayout RDB",
            ));
            items.push(MenuPanelItem::action(
                "layout.calibre_rve_import",
                "Import Calibre/RVE Markers",
            ));
            items.push(menu_action_enabled(
                "layout.drc_markers.write_layer",
                "Write DRC Markers To Layer",
                editing_mode && app.drc_report().is_some(),
            ));
            items.push(menu_action_enabled(
                "layout.drc_marker_snapshot",
                "Export DRC Marker Snapshot",
                editing_mode && app.layout_selected_drc_marker_key.is_some(),
            ));
            items.push(menu_action_enabled(
                "layout.drc_marker_snapshot_png",
                "Export DRC Marker PNG",
                editing_mode && app.layout_selected_drc_marker_key.is_some(),
            ));
            items.push(MenuPanelItem::action(
                "layout.netlist_export",
                "Export Netlist",
            ));
            items.push(MenuPanelItem::action(
                "layout.netlist_import",
                "Import Netlist",
            ));
            items.push(MenuPanelItem::action(
                "layout.spice_netlist_export",
                "Export SPICE Netlist",
            ));
            items.push(MenuPanelItem::action(
                "layout.spice_schematic_compare",
                "Compare SPICE Schematic",
            ));
            items.push(MenuPanelItem::action(
                "layout.trace_state_export",
                "Export Trace State",
            ));
            items.push(MenuPanelItem::action(
                "layout.trace_state_import",
                "Import Trace State",
            ));
            items.push(MenuPanelItem::action(
                "layout.l2n_database_export",
                "Export L2N Database",
            ));
            items.push(MenuPanelItem::action(
                "layout.l2n_database_import",
                "Import L2N Database",
            ));
            items.push(MenuPanelItem::action(
                "layout.reference_images.align",
                "Align Ref Images",
            ));
            items.push(MenuPanelItem::action(
                "layout.reference_images.export",
                "Export Ref Images",
            ));
            items.push(MenuPanelItem::action(
                "layout.reference_images.import",
                "Import Ref Images",
            ));
            items.push(MenuPanelItem::action("tools.diagnostics", "Diagnostics"));
            if let Some(lot_id) = app.workflow_focus_lot.as_deref() {
                items.push(MenuPanelItem::Separator);
                items.push(MenuPanelItem::action(
                    format!("tools.focus_note.{lot_id}"),
                    "Add Focus Note",
                ));
            }
            items
        }
        AppMenu::Macros => vec![
            MenuPanelItem::action("macros.demo", "Load Full Sample Workspace"),
            MenuPanelItem::Separator,
            MenuPanelItem::Label("Layout Test Scenes".to_string()),
            MenuPanelItem::action("macros.stress10k", "10k Stress"),
            MenuPanelItem::action("macros.stress100k", "100k Stress"),
            MenuPanelItem::action("macros.stress1m", "1M Stress"),
            MenuPanelItem::action("macros.hierarchy", "Hierarchy"),
        ],
        AppMenu::Help => vec![MenuPanelItem::disabled("help.about", "About")],
        AppMenu::More => vec![
            MenuPanelItem::action("more.bookmarks", "Bookmarks"),
            MenuPanelItem::action("more.display", "Display"),
            MenuPanelItem::action("more.options", "Options"),
            MenuPanelItem::action("more.tools", "Tools"),
            MenuPanelItem::action("more.macros", "Macros"),
            MenuPanelItem::action("more.help", "Help"),
        ],
    };

    let item_width = ui_scale.value(match menu {
        AppMenu::File => 220.0,
        AppMenu::Display | AppMenu::Tools => 220.0,
        AppMenu::Macros => 210.0,
        AppMenu::View | AppMenu::Options | AppMenu::More => 190.0,
        _ => 180.0,
    });
    let item_height = ui_scale.value(match menu {
        AppMenu::Edit => 16.0,
        AppMenu::File => 16.0,
        AppMenu::Tools => 22.0,
        _ => 28.0,
    });
    let gap = ui_scale.value(if matches!(menu, AppMenu::Edit | AppMenu::File) {
        1.0
    } else {
        2.0
    });
    let panel_width = item_width + ui_scale.value(12.0);
    let panel_height =
        menu_panel_items_height(&items, ui_scale, item_height, gap) + ui_scale.value(12.0);
    let left = menu_popup_left(menu, panel_width, viewport_width, ui_scale);

    if matches!(menu, AppMenu::Edit | AppMenu::Bookmarks | AppMenu::Tools) {
        let column_item_width = if menu == AppMenu::Tools {
            item_width.min(ui_scale.value(204.0))
        } else {
            item_width
        };
        let split_at = (items.len() + 1) / 2;
        let first_column = &items[..split_at];
        let second_column = &items[split_at..];
        let first_column_height = menu_panel_items_height(first_column, ui_scale, item_height, gap);
        let second_column_height =
            menu_panel_items_height(second_column, ui_scale, item_height, gap);
        let panel_width = column_item_width * 2.0 + gap + ui_scale.value(12.0);
        let panel_height = first_column_height.max(second_column_height) + ui_scale.value(12.0);
        let left = menu_popup_left(menu, panel_width, viewport_width, ui_scale);
        let panel = document.add_child(
            parent,
            UiNode::container(
                format!("glassworks.menu_panel.{}", menu.slug()),
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_absolute_position(
                            layout::with_size(
                                layout::row(),
                                layout::px(panel_width),
                                layout::px(panel_height),
                            ),
                            left,
                            ui_scale.value(30.0),
                        ),
                        gap,
                    ),
                    ui_scale.value(6.0),
                ),
            )
            .with_visual(UiVisual::panel(
                COLOR_CHROME_BG,
                Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
                0.0,
            )),
        );
        document.node_mut(panel).style_mut().set_z_index(80);

        for (column_index, column_items) in [first_column, second_column].into_iter().enumerate() {
            let column_height = if column_index == 0 {
                first_column_height
            } else {
                second_column_height
            };
            let column = document.add_child(
                panel,
                UiNode::container(
                    format!(
                        "glassworks.menu_panel.{}.column.{column_index}",
                        menu.slug()
                    ),
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(column_item_width),
                            layout::px(column_height),
                        ),
                        gap,
                    ),
                ),
            );
            let index_offset = if column_index == 0 { 0 } else { split_at };
            for (index, item) in column_items.iter().enumerate() {
                add_menu_panel_item(
                    document,
                    column,
                    menu.slug(),
                    index_offset + index,
                    item,
                    column_item_width,
                    item_height,
                    ui_scale,
                );
            }
        }
        return;
    }

    let panel = document.add_child(
        parent,
        UiNode::container(
            format!("glassworks.menu_panel.{}", menu.slug()),
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(
                            layout::column(),
                            layout::px(panel_width),
                            layout::px(panel_height),
                        ),
                        left,
                        ui_scale.value(30.0),
                    ),
                    gap,
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(80);

    for (index, item) in items.iter().enumerate() {
        add_menu_panel_item(
            document,
            panel,
            menu.slug(),
            index,
            item,
            item_width,
            item_height,
            ui_scale,
        );
    }
}

pub(crate) fn add_view_menu_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    active_view: StartupView,
    app: &GlassworksApp,
    viewport_width: f32,
    ui_scale: UiScale,
) {
    let item_width = ui_scale.value(220.0);
    let item_height = ui_scale.value(28.0);
    let gap = ui_scale.value(2.0);
    let items = [
        (
            "view.command_palette".to_string(),
            "Command Palette...".to_string(),
        ),
        (
            "view.sidebar_modules".to_string(),
            "Sidebar Modules...".to_string(),
        ),
        (
            format!("view.group.{}", ModuleGroup::Design.slug()),
            ModuleGroup::Design.label().to_string(),
        ),
        (
            format!("view.group.{}", ModuleGroup::Operations.slug()),
            ModuleGroup::Operations.label().to_string(),
        ),
        (
            format!("view.group.{}", ModuleGroup::Analysis.slug()),
            ModuleGroup::Analysis.label().to_string(),
        ),
        (
            format!("view.group.{}", ModuleGroup::Engineering.slug()),
            ModuleGroup::Engineering.label().to_string(),
        ),
    ];
    let panel_width = item_width + ui_scale.value(12.0);
    let panel_height =
        items.len() as f32 * item_height + items.len() as f32 * gap + ui_scale.value(13.0);
    let left = menu_popup_left(AppMenu::View, panel_width, viewport_width, ui_scale);

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.menu_panel.view",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(
                            layout::column(),
                            layout::px(panel_width),
                            layout::px(panel_height),
                        ),
                        left,
                        ui_scale.value(30.0),
                    ),
                    gap,
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(80);

    for (index, (action, label)) in items.iter().enumerate() {
        let selected = action
            .strip_prefix("view.group.")
            .and_then(ModuleGroup::from_slug)
            .is_some_and(|group| app.active_view_group == Some(group));
        add_button(
            document,
            panel,
            format!("glassworks.menu.item.{action}"),
            label,
            selected,
            layout::size(layout::px(item_width), layout::px(item_height)),
            ui_scale,
        );

        if index == 1 {
            document.add_child(
                panel,
                UiNode::container(
                    "glassworks.menu.view.separator",
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(1.0))),
                )
                .with_visual(UiVisual::panel(COLOR_PANEL_STROKE, None, 0.0)),
            );
        }
    }

    let Some(group) = app.active_view_group else {
        return;
    };
    let group_views = StartupView::ALL
        .into_iter()
        .filter(|view| view.group() == group)
        .collect::<Vec<_>>();
    let sub_item_width = ui_scale.value(190.0);
    let sub_panel_width = sub_item_width + ui_scale.value(12.0);
    let sub_panel_height = group_views.len() as f32 * item_height
        + group_views.len().saturating_sub(1) as f32 * gap
        + ui_scale.value(12.0);
    let group_index = ModuleGroup::ALL
        .into_iter()
        .position(|candidate| candidate == group)
        .unwrap_or(0);
    let sub_left = (left + panel_width + ui_scale.value(4.0))
        .min((viewport_width - sub_panel_width - ui_scale.value(4.0)).max(ui_scale.value(4.0)));
    let sub_top = ui_scale.value(30.0 + 6.0) + (2 + group_index) as f32 * (item_height + gap);
    let sub_panel = document.add_child(
        parent,
        UiNode::container(
            format!("glassworks.menu_subpanel.view.{}", group.slug()),
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(
                            layout::column(),
                            layout::px(sub_panel_width),
                            layout::px(sub_panel_height),
                        ),
                        sub_left,
                        sub_top,
                    ),
                    gap,
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );
    document.node_mut(sub_panel).style_mut().set_z_index(81);

    for view in group_views {
        add_button(
            document,
            sub_panel,
            format!("glassworks.menu.item.view.{}", view.slug()),
            view.nav_label(),
            active_view == view,
            layout::size(layout::px(sub_item_width), layout::px(item_height)),
            ui_scale,
        );
    }
}

pub(crate) fn add_menu_button(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    menu: AppMenu,
    selected: bool,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let label = menu.label();
    let width = menu_button_width(menu, ui_scale);
    let height = ui_scale.value(22.0);
    let text_rect = UiRect::new(
        ui_scale.value(4.0),
        ui_scale.value(2.0),
        width - ui_scale.value(8.0),
        height - ui_scale.value(4.0),
    );
    let underline_y = ui_scale.value(18.0);
    document.add_child(
        parent,
        UiNode::scene(
            format!("glassworks.menu.{}", menu.slug()),
            vec![
                ScenePrimitive::Text(
                    PaintText::new(
                        label,
                        text_rect,
                        text_style(
                            ui_scale.value(13.0),
                            FontWeight::NORMAL,
                            if selected {
                                COLOR_TEXT
                            } else {
                                COLOR_TEXT_MUTED
                            },
                        ),
                    )
                    .multiline(false),
                ),
                ScenePrimitive::Line {
                    from: UiPoint::new(ui_scale.value(4.0), underline_y),
                    to: UiPoint::new(ui_scale.value(11.0), underline_y),
                    stroke: StrokeStyle::new(
                        if selected {
                            COLOR_TEXT
                        } else {
                            COLOR_TEXT_MUTED
                        },
                        ui_scale.value(1.0),
                    ),
                },
            ],
            layout::size(layout::px(width), layout::px(height)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_accessibility(button_accessibility(label))
        .with_visual(UiVisual::panel(
            if selected {
                COLOR_PANEL_ALT
            } else {
                COLOR_CHROME_BG
            },
            None,
            0.0,
        )),
    )
}

pub(crate) fn menu_button_width(menu: AppMenu, ui_scale: UiScale) -> f32 {
    ui_scale.value((menu.label().len() as f32 * 9.8 + 20.0).max(44.0))
}

pub(crate) fn menu_popup_left(
    menu: AppMenu,
    panel_width: f32,
    viewport_width: f32,
    ui_scale: UiScale,
) -> f32 {
    let compact_preceding = [AppMenu::File, AppMenu::Edit, AppMenu::View];
    let (preceding_width, index) = if menu == AppMenu::More {
        (
            compact_preceding
                .into_iter()
                .map(|candidate| menu_button_width(candidate, ui_scale))
                .sum::<f32>(),
            compact_preceding.len(),
        )
    } else {
        let index = AppMenu::ALL
            .iter()
            .position(|candidate| *candidate == menu)
            .unwrap_or(0);
        (
            AppMenu::ALL
                .iter()
                .take(index)
                .map(|candidate| menu_button_width(*candidate, ui_scale))
                .sum::<f32>(),
            index,
        )
    };
    let gaps = ui_scale.value(4.0) * index as f32;
    let left = ui_scale.value(4.0) + preceding_width + gaps;
    left.min((viewport_width - panel_width - ui_scale.value(4.0)).max(ui_scale.value(4.0)))
}
