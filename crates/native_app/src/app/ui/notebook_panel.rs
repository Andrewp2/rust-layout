#![allow(unused_imports)]
use super::*;

pub(crate) fn add_notebook_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    let Some(entry) = selected_notebook_entry(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let panel_height = if compact_rows { 486.0 } else { 386.0 };
    let body_height = if compact_rows { 96.0 } else { 112.0 };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.notebook.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(panel_height)),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 29, 36, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(55, 69, 83, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.notebook.primary.title",
        entry.title.clone(),
        text_style(
            ui_scale.value(16.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
    );

    let meta_items = vec![
        format!(
            "{} - updated {}",
            display_owner_identifier(&entry.author),
            entry.updated_at
        ),
        if app.notebook_preview_mode {
            "Preview mode".to_string()
        } else {
            "Edit mode".to_string()
        },
        notebook_tag_summary(entry),
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.notebook.meta",
            &meta_items,
            ui_scale,
        );
    } else {
        let meta = document.add_child(
            panel,
            UiNode::container(
                "glassworks.notebook.meta",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(40.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            meta,
            "glassworks.notebook.meta.author",
            meta_items[0].as_str(),
            1.1,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            meta,
            "glassworks.notebook.meta.mode",
            meta_items[1].as_str(),
            0.7,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            meta,
            "glassworks.notebook.meta.tags",
            meta_items[2].as_str(),
            1.2,
            false,
            ui_scale,
        );
    }

    add_text(
        document,
        panel,
        "glassworks.notebook.body",
        compact_button_label(
            &notebook_body_display_text(entry, app.notebook_preview_mode),
            360,
        ),
        text_style(
            ui_scale.value(13.0),
            FontWeight::NORMAL,
            ColorRgba::new(188, 198, 207, 255),
        ),
        layout::with_padding_all(
            layout::size(
                layout::percent(1.0),
                layout::px(ui_scale.value(body_height)),
            ),
            ui_scale.value(8.0),
        ),
    );

    let link_items = vec![
        format!("{} linked objects", entry.link_count()),
        if compact_rows {
            notebook_link_summary_compact(entry)
        } else {
            notebook_link_summary(entry)
        },
    ];
    if compact_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.notebook.links",
            &link_items,
            ui_scale,
        );
    } else {
        let links = document.add_child(
            panel,
            UiNode::container(
                "glassworks.notebook.links",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(44.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            links,
            "glassworks.notebook.links.count",
            link_items[0].as_str(),
            0.8,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            links,
            "glassworks.notebook.links.summary",
            link_items[1].as_str(),
            2.2,
            false,
            ui_scale,
        );
    }

    add_text(
        document,
        panel,
        "glassworks.notebook.related.title",
        "Related entries",
        text_style(
            ui_scale.value(13.0),
            FontWeight::BOLD,
            ColorRgba::new(136, 207, 190, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    for (index, related) in notebook_related_entries(app, entry)
        .iter()
        .take(3)
        .enumerate()
    {
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("glassworks.notebook.related.row.{index}"),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(34.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.notebook.related.row.{index}.title"),
            compact_button_label(&related.0, 30),
            1.3,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("glassworks.notebook.related.row.{index}.reason"),
            compact_button_label(&related.1, 70),
            2.0,
            false,
            ui_scale,
        );
    }
}
