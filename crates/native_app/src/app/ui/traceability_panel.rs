#![allow(unused_imports)]
use super::*;

pub(crate) fn add_traceability_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
) {
    add_domain_panel(
        document,
        parent,
        app,
        "Lot Traceability",
        "Lot genealogy, wafer lineage, material provenance, and event impact",
        "Lot Traceability",
        vec![
            DomainSection::new(
                "glassworks.traceability.overview",
                "Traceability Summary",
                "No traceability summary available",
                dashboard_metric_rows("traceability", traceability_dashboard_metrics(app)),
            )
            .max_rows(5),
            DomainSection::new(
                "glassworks.traceability.lots",
                "Lots",
                "No lots available",
                traceability_lot_rows(app),
            )
            .max_rows(8),
            DomainSection::new(
                "glassworks.traceability.wafers",
                "Wafers",
                "No wafers available for the selected lot",
                traceability_wafer_rows(app),
            )
            .max_rows(8),
            DomainSection::new(
                "glassworks.traceability.detail",
                "Selected Evidence",
                "No selected genealogy evidence",
                trace_selected_wafer_rows(app),
            )
            .max_rows(10),
        ],
        ui_scale,
        compact_rows,
    );
}

pub(crate) fn add_fullscreen_3d_view(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    viewport: UiSize,
    ui_scale: UiScale,
) {
    let stack = document.add_child(
        parent,
        UiNode::container(
            "glassworks.fullscreen.viewport_stack",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::percent(1.0)),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(COLOR_APP_BG, None, 0.0)),
    );
    let header = document.add_child(
        stack,
        UiNode::container(
            "glassworks.fullscreen.header",
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
    add_button(
        document,
        header,
        "glassworks.toolbar.tools.fullscreen",
        "Exit fullscreen",
        false,
        layout::size(
            layout::px(ui_scale.value(132.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    add_text(
        document,
        header,
        "glassworks.fullscreen.hud",
        "3D Stack | click to capture flycam | mouse look | WASD move | Q/E down/up",
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    let canvas_height = (viewport.height - ui_scale.value(48.0)).max(ui_scale.value(280.0));
    let canvas_node = UiNode::gpu_canvas(
        "glassworks.layout.preview",
        "glassworks.layout.viewport.3d",
        fixed_height_layout(
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(canvas_height),
            ),
            canvas_height,
        ),
    );
    let canvas_content = UiContent::Canvas(
        operad::CanvasContent::from_context(operad::CanvasContextDescriptor::gpu_texture(
            "glassworks.layout.viewport.3d",
        ))
        .interaction(CanvasInteractionPolicy::EDITOR),
    );
    let canvas_id = document.add_child(
        stack,
        canvas_node
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::EditorSurface)
                    .label("3D layout viewport")
                    .focusable()
                    .action(AccessibilityAction::new("interact", "Interact")),
            )
            .with_visual(UiVisual::panel(
                COLOR_APP_BG,
                Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
                ui_scale.value(2.0),
            )),
    );
    document.set_node_content(canvas_id, canvas_content);
}
