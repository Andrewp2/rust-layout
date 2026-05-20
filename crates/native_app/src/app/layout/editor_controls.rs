#![allow(unused_imports)]
use super::*;

pub(crate) fn layout_editor_control_rows(app: &GlassworksApp) -> Vec<Vec<ViewControlButton>> {
    let document = &app.workspace.document;
    let mut rows = vec![
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.layout.view.layout2d",
                "2D",
                app.active_view == StartupView::Layout2d,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.layout.view.layout3d",
                "3D",
                app.active_view == StartupView::Layout3d,
            ),
        ],
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.layout.toggle_grid",
                "Grid",
                app.show_grid,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.layout.toggle_snap",
                "Snap",
                app.snap_enabled,
            ),
        ],
        LayoutHierarchyDepth::ALL
            .iter()
            .map(|depth| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.hierarchy.{}", depth.slug()),
                    depth.label(),
                    app.layout_hierarchy_depth == *depth,
                )
            })
            .collect(),
        vec![
            ViewControlButton::new("glassworks.viewctl.layout.hierarchy_step.less", "-", false),
            ViewControlButton::new("glassworks.viewctl.layout.hierarchy_step.more", "+", false),
        ],
        LAYOUT_HIERARCHY_MIN_DEPTH_BUTTONS
            .iter()
            .map(|depth| {
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.hierarchy_min.{depth}"),
                    format!("Min {depth}"),
                    app.layout_hierarchy_min_depth == *depth,
                )
            })
            .collect(),
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.layout.hierarchy_min_step.less",
                "Min -",
                false,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.layout.hierarchy_min_step.more",
                "Min +",
                false,
            ),
        ],
        layout_view_top_cell_buttons(document, app.layout_view_top_cell),
        vec![
            ViewControlButton::new(
                "glassworks.viewctl.layout.toggle_drc",
                "DRC overlay",
                app.show_drc_overlay,
            ),
            ViewControlButton::new(
                "glassworks.viewctl.layout.reference_images.toggle",
                "Ref images",
                app.show_reference_images,
            ),
            ViewControlButton::new("glassworks.viewctl.layout.next_shape", "Next shape", false),
        ],
    ];

    let mut layers = document.layers.values().collect::<Vec<_>>();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.layout.add_layer",
        "Add layer",
        false,
    )]);
    for chunk in layers.iter().take(8).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|layer| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout.layer.{}", layer.id.0),
                        compact_layer_label(layer.id, &layer.name),
                        app.active_layer == layer.id,
                    )
                })
                .collect(),
        );
    }
    for chunk in layers.iter().take(8).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|layer| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout.toggle_layer.{}", layer.id.0),
                        format!(
                            "{} {}",
                            if layer.visible { "Hide" } else { "Show" },
                            layer.id.0
                        ),
                        layer.visible,
                    )
                })
                .collect(),
        );
    }

    let mut shape_rows = app
        .layout_cell_shapes(app.layout_view_top_cell)
        .into_iter()
        .filter(|shape| shape.layer == app.active_layer)
        .take(6)
        .collect::<Vec<_>>();
    if shape_rows.is_empty() {
        shape_rows = app
            .layout_cell_shapes(app.layout_view_top_cell)
            .into_iter()
            .take(6)
            .collect();
    }
    for chunk in shape_rows.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|shape| {
                    ViewControlButton::new(
                        format!("glassworks.viewctl.layout.shape.{}", shape.id.0),
                        format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)),
                        app.selected_layout_shape == Some(shape.id),
                    )
                })
                .collect(),
        );
    }

    rows.push(vec![
        ViewControlButton::new("glassworks.viewctl.layout.copy", "Copy", false),
        ViewControlButton::new(
            "glassworks.viewctl.layout.paste",
            "Paste",
            !app.layout_clipboard_shapes.is_empty(),
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new("glassworks.viewctl.layout.duplicate", "Duplicate", false),
        ViewControlButton::new(
            "glassworks.viewctl.layout.delete",
            "Delete",
            app.selected_layout_shape.is_some(),
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.clear_selection",
            "Clear",
            app.selected_layout_shape.is_some(),
        ),
        ViewControlButton::new("glassworks.viewctl.layout.run_drc", "Run DRC", false),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.run_drc_region",
            "DRC Region",
            app.selected_layout_shape.is_some(),
        ),
        ViewControlButton::new("glassworks.viewctl.layout.run_drc_cell", "DRC Cell", false),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.run_route",
            "Run Route",
            app.route_points.len() >= 2,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.layout.trace_between",
            "Trace Path",
            app.route_points.len() >= 2,
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.route_points.clear",
            "Clear Points",
            !app.route_points.is_empty(),
        ),
        ViewControlButton::new("glassworks.viewctl.layout.trace_all", "Trace All", false),
    ]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.layout.connectivity",
        "Connectivity",
        false,
    )]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.reference_images.align",
            "Align Refs",
            false,
        ),
        ViewControlButton::new(
            "glassworks.viewctl.layout.reference_images.import",
            "Import Refs",
            false,
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "glassworks.viewctl.layout.reference_images.landmarks.fit_selection",
            "Fit Ref",
            app.selected_layout_shape.is_some(),
        ),
        ViewControlButton::new(
            "glassworks.viewctl.layout.reference_images.landmarks.seed",
            "Seed Marks",
            !app.workspace.document.reference_images.is_empty(),
        ),
    ]);
    let reference_image_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .map(|(index, image)| {
            ViewControlButton::new(
                format!("glassworks.viewctl.layout.reference_image.toggle.{index}"),
                format!(
                    "Ref {} {}",
                    index + 1,
                    if image.visible { "on" } else { "off" }
                ),
                image.visible,
            )
        })
        .collect::<Vec<_>>();
    if !reference_image_buttons.is_empty() {
        rows.push(reference_image_buttons);
    }
    let reference_image_landmark_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .flat_map(|(index, image)| {
            [
                ViewControlButton::new(
                    format!(
                        "glassworks.viewctl.layout.reference_image.landmarks.fit_selection.{index}"
                    ),
                    format!("Ref {} fit", index + 1),
                    app.selected_layout_shape.is_some(),
                ),
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.landmarks.seed.{index}"),
                    format!("Ref {} seed", index + 1),
                    image.pixel_size.is_some(),
                ),
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.landmarks.align.{index}"),
                    format!("Ref {} align", index + 1),
                    image.landmarks.len() >= 2 && image.pixel_size.is_some(),
                ),
            ]
        })
        .collect::<Vec<_>>();
    for chunk in reference_image_landmark_buttons.chunks(2) {
        rows.push(chunk.to_vec());
    }
    if !app.workspace.document.reference_images.is_empty() {
        rows.push(vec![
            ViewControlButton::new(
                "glassworks.viewctl.layout.reference_images.show_all",
                "Show Refs",
                app.show_reference_images
                    && app
                        .workspace
                        .document
                        .reference_images
                        .iter()
                        .all(|image| image.visible),
            ),
            ViewControlButton::new(
                "glassworks.viewctl.layout.reference_images.hide_all",
                "Hide Refs",
                app.workspace
                    .document
                    .reference_images
                    .iter()
                    .all(|image| !image.visible),
            ),
        ]);
    }
    let reference_image_opacity_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .flat_map(|(index, _)| {
            [
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.opacity_less.{index}"),
                    format!("Ref {} fade", index + 1),
                    false,
                ),
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.opacity_more.{index}"),
                    format!("Ref {} opaque", index + 1),
                    false,
                ),
            ]
        })
        .collect::<Vec<_>>();
    for chunk in reference_image_opacity_buttons.chunks(2) {
        rows.push(chunk.to_vec());
    }
    let reference_image_order_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .flat_map(|(index, _)| {
            [
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.move_up.{index}"),
                    format!("Ref {} up", index + 1),
                    false,
                ),
                ViewControlButton::new(
                    format!("glassworks.viewctl.layout.reference_image.move_down.{index}"),
                    format!("Ref {} down", index + 1),
                    false,
                ),
            ]
        })
        .collect::<Vec<_>>();
    for chunk in reference_image_order_buttons.chunks(2) {
        rows.push(chunk.to_vec());
    }
    let reference_image_focus_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .map(|(index, _)| {
            ViewControlButton::new(
                format!("glassworks.viewctl.layout.reference_image.focus.{index}"),
                format!("View Ref {}", index + 1),
                false,
            )
        })
        .collect::<Vec<_>>();
    if !reference_image_focus_buttons.is_empty() {
        rows.push(reference_image_focus_buttons);
    }
    let reference_image_remove_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .map(|(index, _)| {
            ViewControlButton::new(
                format!("glassworks.viewctl.layout.reference_image.remove.{index}"),
                format!("Remove Ref {}", index + 1),
                false,
            )
        })
        .collect::<Vec<_>>();
    if !reference_image_remove_buttons.is_empty() {
        rows.push(reference_image_remove_buttons);
    }
    let reference_image_landmark_clear_buttons = app
        .workspace
        .document
        .reference_images
        .iter()
        .take(2)
        .enumerate()
        .map(|(index, image)| {
            ViewControlButton::new(
                format!("glassworks.viewctl.layout.reference_image.landmarks.clear.{index}"),
                format!("Clear Ref {} Marks", index + 1),
                !image.landmarks.is_empty(),
            )
        })
        .collect::<Vec<_>>();
    if !reference_image_landmark_clear_buttons.is_empty() {
        rows.push(reference_image_landmark_clear_buttons);
    }
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.layout.reference_images.landmarks.clear",
        "Clear Ref Marks",
        app.workspace
            .document
            .reference_images
            .iter()
            .any(|image| !image.landmarks.is_empty()),
    )]);
    rows.push(vec![ViewControlButton::new(
        "glassworks.viewctl.layout.reference_images.clear",
        "Clear Refs",
        !app.workspace.document.reference_images.is_empty(),
    )]);
    rows.retain(|row| !row.is_empty());
    rows
}
