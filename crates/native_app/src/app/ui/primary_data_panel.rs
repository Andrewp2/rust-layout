#![allow(unused_imports)]
use super::*;

pub(crate) fn add_primary_data_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    title: &str,
    rows: Vec<PrimaryRow>,
    ui_scale: UiScale,
) {
    let row_height = ui_scale.value(36.0);
    let gap = ui_scale.value(6.0);
    let visible_rows = rows.len().max(1).min(12);
    let height = ui_scale.value(70.0)
        + row_height * visible_rows as f32
        + gap * visible_rows.saturating_sub(1) as f32;
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    gap,
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );

    add_text(
        document,
        panel,
        "glassworks.primary.title",
        format!("{title} Primary View"),
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );

    let header = document.add_child(
        panel,
        UiNode::container(
            "glassworks.primary.header",
            layout::with_gap_all(
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
                gap,
            ),
        ),
    );
    add_primary_cell(
        document,
        header,
        "glassworks.primary.header.title",
        "Item",
        1.6,
        true,
        ui_scale,
    );
    add_primary_cell(
        document,
        header,
        "glassworks.primary.header.value",
        "State",
        1.0,
        true,
        ui_scale,
    );
    add_primary_cell(
        document,
        header,
        "glassworks.primary.header.detail",
        "Detail",
        2.2,
        true,
        ui_scale,
    );

    if rows.is_empty() {
        let empty = document.add_child(
            panel,
            UiNode::container(
                "glassworks.primary.row.0",
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
            ),
        );
        add_primary_cell(
            document,
            empty,
            "glassworks.primary.row.0.empty",
            "No rows match the active filters",
            1.0,
            false,
            ui_scale,
        );
        return;
    }

    for (row_index, row_data) in rows.iter().take(12).enumerate() {
        let row_name = format!("glassworks.primary.row.{row_index}");
        let row = document.add_child(
            panel,
            UiNode::container(
                row_name.clone(),
                layout::with_gap_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
                    gap,
                ),
            )
            .with_visual(UiVisual::panel(
                if row_index % 2 == 0 {
                    COLOR_PANEL_ALT
                } else {
                    COLOR_PANEL_BG
                },
                Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
                ui_scale.value(2.0),
            )),
        );
        if let Some(action) = row_data.action.as_ref() {
            let button_name = action
                .strip_prefix("glassworks.viewctl.")
                .map(|action| format!("glassworks.primary.action.{row_index}.{action}"))
                .unwrap_or_else(|| format!("{row_name}.action.{}", row_data.name));
            add_button(
                document,
                row,
                button_name,
                compact_button_label(&row_data.title, 24),
                row_data.selected,
                primary_cell_layout(1.6),
                ui_scale,
            );
        } else {
            add_primary_cell(
                document,
                row,
                format!("{row_name}.title.{}", row_data.name),
                compact_button_label(&row_data.title, 28),
                1.6,
                false,
                ui_scale,
            );
        }
        add_primary_cell(
            document,
            row,
            format!("{row_name}.value.{}", row_data.name),
            compact_button_label(&row_data.value, 22),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("{row_name}.detail.{}", row_data.name),
            compact_button_label(&row_data.detail, 72),
            2.2,
            false,
            ui_scale,
        );
    }
}

pub(crate) fn primary_cell_layout(weight: f32) -> operad::LayoutStyle {
    layout::with_size(
        layout::flex_item(weight, 1.0, layout::px(0.0)),
        layout::auto(),
        layout::percent(1.0),
    )
}

pub(crate) fn add_primary_cell(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    weight: f32,
    header: bool,
    ui_scale: UiScale,
) {
    add_primary_text_cell(
        document,
        parent,
        name,
        text,
        header,
        ui_scale,
        layout::with_padding_all(primary_cell_layout(weight), ui_scale.value(7.0)),
    );
}

pub(crate) fn add_primary_text_cell(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    header: bool,
    ui_scale: UiScale,
    node_layout: impl Into<operad::LayoutStyle>,
) {
    add_text(
        document,
        parent,
        name,
        text,
        text_style(
            ui_scale.value(if header { 12.0 } else { 13.0 }),
            if header {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            },
            if header {
                ColorRgba::new(136, 207, 190, 255)
            } else {
                COLOR_TEXT_MUTED
            },
        ),
        node_layout,
    );
}
