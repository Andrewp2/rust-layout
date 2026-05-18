#![allow(unused_imports)]
use super::*;

pub(crate) fn add_sectioned_side_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    app: &GlassworksApp,
    sections: Vec<DetailSection>,
    width: f32,
    ui_scale: UiScale,
) {
    let panel = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle::new(layout::with_min_size(
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width),
                            layout::percent(1.0),
                        ),
                        ui_scale.value(8.0),
                    ),
                    ui_scale.value(12.0),
                ),
                layout::px(0.0),
                layout::px(0.0),
            ))
            .with_clip(ClipBehavior::Clip),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    if !title.is_empty() {
        add_text(
            document,
            panel,
            format!("{name}.title"),
            title,
            text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
        );
    }
    add_collapsible_detail_sections(document, panel, name, app, &sections, 6, 6, ui_scale);
}

pub(crate) fn add_collapsible_detail_sections(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    base_name: &str,
    app: &GlassworksApp,
    sections: &[DetailSection],
    max_sections: usize,
    max_rows: usize,
    ui_scale: UiScale,
) {
    for (section_index, section) in sections.iter().take(max_sections).enumerate() {
        let key = detail_section_key(app.active_view, section_index, &section.title);
        let expanded = !app.collapsed_detail_sections.contains(&key);
        let mut options = CollapsingHeaderOptions::default()
            .expanded(expanded)
            .with_toggle_action(format!("glassworks.inspector.section.toggle.{key}"));
        options.layout = LayoutStyle::column().with_width_percent(1.0);
        options.header_layout = layout::with_padding_all(
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(24.0)),
                ),
                ui_scale.value(4.0),
            ),
            ui_scale.value(3.0),
        );
        options.body_layout = layout::with_gap_all(
            layout::with_padding_all(layout::column(), ui_scale.value(4.0)),
            ui_scale.value(2.0),
        );
        options.header_visual = UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        );
        options.hovered_visual = UiVisual::panel(
            COLOR_PANEL_ALT,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        );
        options.pressed_visual = UiVisual::panel(
            COLOR_BUTTON_SELECTED,
            Some(StrokeStyle::new(
                COLOR_BUTTON_STROKE_SELECTED,
                ui_scale.value(1.0),
            )),
            ui_scale.value(2.0),
        );
        options.body_visual = UiVisual::panel(COLOR_APP_BG, None, 0.0);
        options.text_style = text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT);
        options.indicator_text_style =
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED);
        options.accessibility_label = Some(section.title.clone());

        let nodes = collapsing_header(
            document,
            parent,
            format!("{base_name}.section.{section_index}"),
            section.title.clone(),
            options,
        );
        if let Some(body) = nodes.body {
            for (row_index, (label, value)) in section.rows.iter().take(max_rows).enumerate() {
                add_text(
                    document,
                    body,
                    format!("{base_name}.section.{section_index}.row.{row_index}"),
                    format!("{label}: {value}"),
                    text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(18.0))),
                );
            }
        }
    }
}

pub(crate) fn detail_section_key(view: StartupView, section_index: usize, title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let slug = if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    };
    format!("{}.{}.{}", view.slug(), section_index, slug)
}

pub(crate) fn add_detail_sections(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let sections = view_detail_sections(app);
    if sections.is_empty() {
        return;
    }
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.details",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(220.0)),
                    ),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_collapsible_detail_sections(
        document,
        panel,
        "glassworks.details",
        app,
        &sections,
        4,
        5,
        ui_scale,
    );
}
