#![allow(unused_imports)]
use super::*;

pub(crate) fn add_diagnostics_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport: UiSize,
    ui_scale: UiScale,
) {
    let width = (viewport.width - ui_scale.value(48.0))
        .min(ui_scale.value(380.0))
        .max(ui_scale.value(300.0));
    let height = ui_scale.value(300.0);
    let left = ((viewport.width - width) * 0.5).max(ui_scale.value(12.0));
    let top = ui_scale.value(64.0);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.diagnostics_panel",
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
    add_overlay_title_row(
        document,
        panel,
        "glassworks.diagnostics",
        "Diagnostics",
        "glassworks.diagnostics.close",
        width,
        ui_scale,
    );
    for (index, (label, value)) in [
        ("View", app.active_view.nav_label().to_string()),
        (
            "Layout shapes",
            app.workspace
                .document
                .flattened_shape_count_estimate()
                .to_string(),
        ),
        ("Lots", app.workspace.mes.lots.len().to_string()),
        (
            "Equipment tools",
            app.workspace.equipment.tools().count().to_string(),
        ),
        ("Active tool", app.active_tool.label().to_string()),
    ]
    .into_iter()
    .enumerate()
    {
        add_text(
            document,
            panel,
            format!("glassworks.diagnostics.row.{index}"),
            format!("{label}: {value}"),
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        );
    }
}

pub(crate) fn add_overlay_title_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    close_name: &str,
    width: f32,
    ui_scale: UiScale,
) {
    let header = document.add_child(
        parent,
        UiNode::container(
            format!("{name}.header"),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::px(width - ui_scale.value(20.0)),
                    layout::px(ui_scale.value(30.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_text(
        document,
        header,
        format!("{name}.title"),
        title,
        text_style(ui_scale.value(16.0), FontWeight::NORMAL, COLOR_TEXT),
        layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
    );
    add_button(
        document,
        header,
        close_name,
        "Close",
        false,
        layout::size(
            layout::px(ui_scale.value(72.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
}

pub(crate) fn add_menu_item_button(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    node_layout: impl Into<operad::LayoutStyle>,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let name = name.into();
    let label = label.into();
    let text_color = if enabled {
        COLOR_TEXT
    } else {
        COLOR_TEXT_MUTED
    };
    let mut primitives = Vec::new();
    if selected {
        let check_stroke =
            StrokeStyle::new(ColorRgba::new(102, 190, 236, 255), ui_scale.value(1.6));
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(ui_scale.value(8.0), ui_scale.value(14.0)),
            to: UiPoint::new(ui_scale.value(12.0), ui_scale.value(18.0)),
            stroke: check_stroke,
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(ui_scale.value(12.0), ui_scale.value(18.0)),
            to: UiPoint::new(ui_scale.value(19.0), ui_scale.value(9.0)),
            stroke: check_stroke,
        });
    }
    primitives.push(ScenePrimitive::Text(PaintText::new(
        label.clone(),
        UiRect::new(
            ui_scale.value(28.0),
            ui_scale.value(6.0),
            ui_scale.value(720.0),
            ui_scale.value(18.0),
        ),
        text_style(ui_scale.value(13.0), FontWeight::NORMAL, text_color),
    )));
    let mut node = UiNode::scene(name, primitives, node_layout)
        .with_accessibility(button_accessibility(&label))
        .with_visual(UiVisual::panel(COLOR_BUTTON_BG, None, 0.0));
    if enabled {
        node = node.with_input(InputBehavior::BUTTON);
    }
    document.add_child(parent, node)
}

pub(crate) fn add_button(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    node_layout: impl Into<operad::LayoutStyle>,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let label = label.into();
    add_button_with_accessibility_label(
        document,
        parent,
        name,
        label.clone(),
        label,
        selected,
        node_layout,
        ui_scale,
    )
}

pub(crate) fn add_button_with_accessibility_label(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    accessibility_label: impl Into<String>,
    selected: bool,
    node_layout: impl Into<operad::LayoutStyle>,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let name = name.into();
    let label = label.into();
    let accessibility_label = accessibility_label.into();
    let fill = if selected {
        COLOR_BUTTON_SELECTED
    } else {
        COLOR_BUTTON_BG
    };
    let stroke = Some(StrokeStyle::new(
        if selected {
            ColorRgba::new(60, 134, 190, 255)
        } else {
            COLOR_PANEL_STROKE
        },
        ui_scale.value(1.0),
    ));
    let text_style = text_style(
        ui_scale.value(13.0),
        if selected {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        },
        COLOR_TEXT,
    );
    button(
        document,
        parent,
        name.clone(),
        label.clone(),
        ButtonOptions::new(node_layout)
            .with_visual(UiVisual::panel(fill, stroke, ui_scale.value(2.0)))
            .with_text_style(text_style)
            .with_action(WidgetActionBinding::action(name))
            .with_accessibility_label(accessibility_label),
    )
}

pub(crate) fn attach_default_pointer_actions(document: &mut UiDocument) {
    for index in 0..document.node_count() {
        let id = operad::UiNodeId::from_index(index);
        let action = {
            let node = document.node(id);
            let is_canvas = matches!(node.content(), UiContent::Canvas(_));
            (node.action().is_none() && node.input().pointer && !is_canvas)
                .then(|| node.name().to_string())
        };
        if let Some(action) = action {
            document.set_node_action(id, WidgetActionBinding::action(action));
        }
    }
}

pub(crate) fn button_accessibility(label: &str) -> AccessibilityMeta {
    AccessibilityMeta::new(AccessibilityRole::Button)
        .label(label)
        .focusable()
        .action(AccessibilityAction::new("activate", "Activate"))
}

pub(crate) fn add_text(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    style: TextStyle,
    node_layout: impl Into<operad::LayoutStyle>,
) {
    document.add_child(parent, UiNode::text(name, text, style, node_layout));
}

pub(crate) fn text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: (font_size * 1.25).ceil(),
        weight,
        color,
        wrap: TextWrap::WordOrGlyph,
        ..Default::default()
    }
}
