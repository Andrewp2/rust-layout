#![allow(unused_imports)]
use super::*;

pub(crate) fn add_scrollbar_rail(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let rail = document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_padding_all(
                layout::with_size(
                    layout::column(),
                    layout::px(ui_scale.value(8.0)),
                    layout::percent(1.0),
                ),
                ui_scale.value(2.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(10, 14, 17, 255),
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        )),
    );
    document.add_child(
        rail,
        UiNode::container(
            format!("{name}.thumb"),
            layout::size(
                layout::px(ui_scale.value(4.0)),
                layout::px(ui_scale.value(130.0)),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(82, 96, 109, 255),
            None,
            ui_scale.value(2.0),
        )),
    );
    rail
}

pub(crate) fn add_command_palette_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport: UiSize,
    ui_scale: UiScale,
) {
    let width = (viewport.width - ui_scale.value(48.0))
        .min(ui_scale.value(520.0))
        .max(ui_scale.value(320.0));
    let height = (viewport.height - ui_scale.value(96.0))
        .min(ui_scale.value(520.0))
        .max(ui_scale.value(280.0));
    let left = ((viewport.width - width) * 0.5).max(ui_scale.value(12.0));
    let top = ui_scale.value(64.0);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.command_palette",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(layout::column(), layout::px(width), layout::px(height)),
                        left,
                        top,
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(100);

    let header = document.add_child(
        panel,
        UiNode::container(
            "glassworks.command.header",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_text(
        document,
        header,
        "glassworks.command.title",
        "Command Palette",
        text_style(ui_scale.value(16.0), FontWeight::NORMAL, COLOR_TEXT),
        layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
    );
    add_button(
        document,
        header,
        "glassworks.command.close",
        "Close",
        false,
        layout::size(
            layout::px(ui_scale.value(72.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );

    let list_height = height - ui_scale.value(58.0);
    let list = document.add_child(
        panel,
        UiNode::container(
            "glassworks.command.list",
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(list_height),
                ),
                ui_scale.value(4.0),
            ),
        )
        .with_scroll(ScrollAxes::VERTICAL),
    );

    let command_views = StartupView::ALL;
    let button_height = ui_scale.value(28.0);
    let row_gap = ui_scale.value(4.0);
    let rows_fit = ((list_height + row_gap) / (button_height + row_gap))
        .floor()
        .max(1.0) as usize;
    let column_count = if command_views.len() > rows_fit && width >= ui_scale.value(420.0) {
        command_views.len().div_ceil(rows_fit).clamp(1, 2)
    } else {
        1
    };

    if column_count == 1 {
        for view in command_views {
            add_button(
                document,
                list,
                format!("glassworks.command.view.{}", view.slug()),
                format!("Go to {}", view.label()),
                app.active_view == view,
                layout::size(layout::percent(1.0), layout::px(button_height)),
                ui_scale,
            );
        }
    } else {
        let rows = command_views.len().div_ceil(column_count);
        for row_index in 0..rows {
            let row = document.add_child(
                list,
                UiNode::container(
                    format!("glassworks.command.row.{row_index}"),
                    layout::with_gap_all(
                        layout::with_size(
                            layout::row(),
                            layout::percent(1.0),
                            layout::px(button_height),
                        ),
                        ui_scale.value(6.0),
                    ),
                ),
            );
            for column_index in 0..column_count {
                let view_index = column_index * rows + row_index;
                let Some(view) = command_views.get(view_index).copied() else {
                    continue;
                };
                add_button(
                    document,
                    row,
                    format!("glassworks.command.view.{}", view.slug()),
                    format!("Go to {}", view.nav_label()),
                    app.active_view == view,
                    layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
                    ui_scale,
                );
            }
        }
    }
}

pub(crate) fn add_sidebar_modules_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport: UiSize,
    ui_scale: UiScale,
) {
    let width = (viewport.width - ui_scale.value(48.0))
        .min(ui_scale.value(380.0))
        .max(ui_scale.value(300.0));
    let height = (viewport.height - ui_scale.value(96.0))
        .min(ui_scale.value(760.0))
        .max(ui_scale.value(360.0));
    let left = ((viewport.width - width) * 0.5).max(ui_scale.value(12.0));
    let top = ui_scale.value(64.0);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.sidebar_modules",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(layout::column(), layout::px(width), layout::px(height)),
                        left,
                        top,
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(100);

    let header = document.add_child(
        panel,
        UiNode::container(
            "glassworks.sidebar.header",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_text(
        document,
        header,
        "glassworks.sidebar.title",
        "Sidebar Modules",
        text_style(ui_scale.value(16.0), FontWeight::NORMAL, COLOR_TEXT),
        layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
    );
    add_button(
        document,
        header,
        "glassworks.sidebar.close",
        "Close",
        false,
        layout::size(
            layout::px(ui_scale.value(72.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );

    let controls = document.add_child(
        panel,
        UiNode::container(
            "glassworks.sidebar.controls",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for (name, label) in [
        ("glassworks.sidebar.default", "Default"),
        ("glassworks.sidebar.all", "All"),
        ("glassworks.sidebar.none", "None"),
    ] {
        add_button(
            document,
            controls,
            name,
            label,
            false,
            layout::size(
                layout::px(ui_scale.value(92.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }

    let list_height = height - ui_scale.value(96.0);
    let list = document.add_child(
        panel,
        UiNode::container(
            "glassworks.sidebar.list",
            layout::with_gap_all(
                layout::with_size(
                    layout::column(),
                    layout::percent(1.0),
                    layout::px(list_height.max(ui_scale.value(180.0))),
                ),
                ui_scale.value(2.0),
            ),
        )
        .with_scroll(ScrollAxes::VERTICAL),
    );
    for group in ModuleGroup::ALL {
        add_text(
            document,
            list,
            format!("glassworks.sidebar.group.{}", group.slug()),
            group.label(),
            text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(16.0))),
        );
        for view in StartupView::ALL
            .into_iter()
            .filter(|view| view.group() == group)
        {
            add_button(
                document,
                list,
                format!("glassworks.sidebar.view.{}", view.slug()),
                view.nav_label(),
                app.nav_rail_views.contains(&view),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
                ui_scale,
            );
        }
    }
}
