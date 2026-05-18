#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_layer_convex_polygon_not_writes_polygon_fragments() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let cut = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(500, 250),
                Point::new(750, 500),
                Point::new(500, 750),
                Point::new(250, 500),
            ])),
        )
        .expect("cut polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(cut);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(cut));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_not_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&cut));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert_eq!(active_shapes.len(), 4);
    assert!(
        active_shapes
            .iter()
            .any(|shape| matches!(shape.kind, ShapeKind::Polygon(_))),
        "subtracting a diamond from a rectangle should require polygon fragments"
    );
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 1_750_000);
    assert!(
        app.status_message()
            .contains("Subtracted selection from 1 active-layer shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&cut));
}

#[test]
pub(crate) fn layout_layer_not_accepts_selected_nonconvex_polygon_region() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let cut = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex cut polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(cut);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(cut));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_not_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&cut));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 720_000);
    let union_bounds = active_shapes
        .iter()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right))
        .expect("remaining fragments should have bounds");
    assert_eq!(
        union_bounds,
        Rect::new(Point::new(400, 400), Point::new(1_000, 1_000))
    );
}

#[test]
pub(crate) fn layout_selection_rectangle_not_layer_writes_selected_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let selection = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("selection rectangle should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.selection_not_layer"));
    assert!(
        !app.workspace.document.shapes.contains_key(&source),
        "selection NOT layer should replace original active-layer rectangles"
    );
    assert!(
        app.workspace.document.shapes.contains_key(&selection),
        "selection on another layer should be preserved"
    );
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(
        active_rects,
        vec![Rect::new(Point::new(1_000, 250), Point::new(1_300, 750))]
    );
    assert!(
        app.status_message()
            .contains("Subtracted 1 active-layer shape from selection")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
}

#[test]
pub(crate) fn layout_selection_convex_polygon_not_layer_writes_selected_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let selection = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(500, 250),
                Point::new(750, 500),
                Point::new(500, 750),
                Point::new(250, 500),
            ])),
        )
        .expect("selection polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.selection_not_layer"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert_eq!(active_shapes.len(), 1);
    assert_eq!(
        active_shapes[0].kind.bounds(),
        Rect::from_points(&[
            Point::new(250, 500),
            Point::new(500, 250),
            Point::new(500, 750),
        ])
        .expect("triangle fragment should have bounds")
    );
    assert_eq!(shape_kind_area2_abs(&active_shapes[0].kind), 125_000);
    assert!(
        app.status_message()
            .contains("Subtracted 1 active-layer shape from selection")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
}

#[test]
pub(crate) fn layout_selection_nonconvex_polygon_not_layer_writes_selected_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(400, 400))),
        )
        .expect("source rectangle should be added");
    let selection = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex selection polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.selection_not_layer"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 960_000);
    let union_bounds = active_shapes
        .iter()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right))
        .expect("remaining fragments should have bounds");
    assert_eq!(
        union_bounds,
        Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))
    );
    assert!(
        app.status_message()
            .contains("Subtracted 1 active-layer shape from selection")
    );
}

#[test]
pub(crate) fn layout_layer_rectangle_xor_writes_symmetric_difference_with_selection() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let xor_rect = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("xor rectangle should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(xor_rect);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(xor_rect));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_xor_selection"));
    assert!(
        !app.workspace.document.shapes.contains_key(&source),
        "layer XOR should replace original active-layer rectangles"
    );
    assert!(
        app.workspace.document.shapes.contains_key(&xor_rect),
        "XOR selection on another layer should be preserved"
    );
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 4);
    for expected in [
        Rect::new(Point::new(0, 0), Point::new(1_000, 250)),
        Rect::new(Point::new(0, 750), Point::new(1_000, 1_000)),
        Rect::new(Point::new(0, 250), Point::new(500, 750)),
        Rect::new(Point::new(1_000, 250), Point::new(1_300, 750)),
    ] {
        assert!(
            active_rects.contains(&expected),
            "missing XOR fragment {expected:?}"
        );
    }
    assert!(app.status_message().contains("XORed 1 active-layer shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&xor_rect));
}

#[test]
pub(crate) fn layout_layer_convex_polygon_xor_writes_symmetric_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let xor_region = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(500, 250),
                Point::new(750, 500),
                Point::new(500, 750),
                Point::new(250, 500),
            ])),
        )
        .expect("xor polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(xor_region);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(xor_region));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_xor_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&xor_region));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert_eq!(active_shapes.len(), 4);
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 1_750_000);
    assert!(app.status_message().contains("XORed 1 active-layer shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&xor_region));
}

#[test]
pub(crate) fn layout_layer_nonconvex_polygon_xor_writes_symmetric_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    let xor_region = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex xor polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(xor_region);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(xor_region));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_xor_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&xor_region));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 720_000);
    let union_bounds = active_shapes
        .iter()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right))
        .expect("xor fragments should have bounds");
    assert_eq!(
        union_bounds,
        Rect::new(Point::new(400, 400), Point::new(1_000, 1_000))
    );
    assert!(app.status_message().contains("XORed 1 active-layer shape"));
}

#[test]
pub(crate) fn layout_shape_clipboard_and_replaces_selected_shape() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("clipboard rectangle should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_and_clipboard"));
    assert!(
        !app.workspace.document.shapes.contains_key(&source),
        "shape AND should replace the selected source shape"
    );
    assert!(
        app.workspace.document.shapes.contains_key(&clipboard),
        "the copied operand should stay in the document"
    );
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(
        active_rects,
        vec![Rect::new(Point::new(500, 250), Point::new(1_000, 750))]
    );
    assert!(
        app.status_message()
            .contains("Shape AND clipboard replaced shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.edit"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("Edit menu should build");
    for node_name in [
        "glassworks.menu.item.edit.shape_and_clipboard",
        "glassworks.menu.item.edit.shape_or_clipboard",
        "glassworks.menu.item.edit.shape_not_clipboard",
        "glassworks.menu.item.edit.shape_xor_clipboard",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "Edit menu should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
}

#[test]
pub(crate) fn layout_shape_clipboard_or_writes_union_fragments() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("clipboard rectangle should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_or_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 4);
    for expected in [
        Rect::new(Point::new(0, 0), Point::new(1_000, 250)),
        Rect::new(Point::new(0, 750), Point::new(1_000, 1_000)),
        Rect::new(Point::new(0, 250), Point::new(500, 750)),
        Rect::new(Point::new(500, 250), Point::new(1_300, 750)),
    ] {
        assert!(
            active_rects.contains(&expected),
            "missing shape OR fragment {expected:?}"
        );
    }
    let area2 = active_rects
        .iter()
        .map(|rect| i128::from(rect.width()) * i128::from(rect.height()) * 2)
        .sum::<i128>();
    assert_eq!(area2, 2_300_000);
    assert!(
        app.status_message()
            .contains("Shape OR clipboard replaced shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
}

#[test]
pub(crate) fn layout_shape_clipboard_not_writes_selected_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(250, 250), Point::new(750, 750))),
        )
        .expect("clipboard rectangle should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_not_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 4);
    for expected in [
        Rect::new(Point::new(0, 0), Point::new(1_000, 250)),
        Rect::new(Point::new(0, 750), Point::new(1_000, 1_000)),
        Rect::new(Point::new(0, 250), Point::new(250, 750)),
        Rect::new(Point::new(750, 250), Point::new(1_000, 750)),
    ] {
        assert!(
            active_rects.contains(&expected),
            "missing shape NOT fragment {expected:?}"
        );
    }
    assert!(
        app.status_message()
            .contains("Shape NOT clipboard replaced shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
}

#[test]
pub(crate) fn layout_shape_clipboard_xor_writes_symmetric_difference() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("clipboard rectangle should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_xor_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 4);
    for expected in [
        Rect::new(Point::new(0, 0), Point::new(1_000, 250)),
        Rect::new(Point::new(0, 750), Point::new(1_000, 1_000)),
        Rect::new(Point::new(0, 250), Point::new(500, 750)),
        Rect::new(Point::new(1_000, 250), Point::new(1_300, 750)),
    ] {
        assert!(
            active_rects.contains(&expected),
            "missing shape XOR fragment {expected:?}"
        );
    }
    assert!(
        app.status_message()
            .contains("Shape XOR clipboard replaced shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
}

#[test]
pub(crate) fn layout_shape_clipboard_and_accepts_nonconvex_clipboard_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex clipboard polygon should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_and_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert!(
        active_shapes.len() > 1,
        "non-convex clipboard AND should decompose the result"
    );
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 1_280_000);
    assert!(
        app.status_message()
            .contains("Shape AND clipboard replaced shape")
    );
}

#[test]
pub(crate) fn layout_shape_clipboard_not_accepts_nonconvex_selected_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(400, 400))),
        )
        .expect("clipboard rectangle should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex selected polygon should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_not_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert!(
        active_shapes.len() > 1,
        "non-convex selected shape NOT should write decomposed fragments"
    );
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 960_000);
    assert!(
        app.status_message()
            .contains("Shape NOT clipboard replaced shape")
    );
}

#[test]
pub(crate) fn layout_shape_clipboard_xor_accepts_nonconvex_clipboard_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    app.workspace.document.shapes.clear();
    let clipboard = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("non-convex clipboard polygon should be added");
    app.selected_layout_shape = Some(clipboard);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));

    let source = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("source rectangle should be added");
    app.selected_layout_shape = Some(source);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shape_xor_clipboard"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clipboard));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 720_000);
    let union_bounds = active_shapes
        .iter()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right))
        .expect("xor fragments should have bounds");
    assert_eq!(
        union_bounds,
        Rect::new(Point::new(400, 400), Point::new(1_000, 1_000))
    );
    assert!(
        app.status_message()
            .contains("Shape XOR clipboard replaced shape")
    );
}

#[test]
pub(crate) fn layout_canvas_route_tool_uses_router_output() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.05),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let shape_count = app.workspace().document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.tool.route"));

    let start_world = Point::new(-12_000, -10_000);
    let goal_world = Point::new(-10_000, -8_000);
    app.layout_pan = [
        -(start_world.x as f32) * app.layout_zoom,
        start_world.y as f32 * app.layout_zoom,
    ];
    let start = app.layout_world_to_canvas(start_world, rect_size(canvas));
    let goal = app.layout_world_to_canvas(goal_world, rect_size(canvas));

    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(goal), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(goal), canvas));

    assert_eq!(
        app.workspace().document.shapes.len(),
        shape_count,
        "route endpoints should not run the router until Run Route is clicked"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.run_route"));
    assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
    let route_shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("route tool should select the routed path");
    let ShapeKind::Path { points, width } = route_shape.kind else {
        panic!("route tool should create a path shape");
    };
    assert_eq!(width, 180);
    assert!(points.len() >= 2, "router should produce route points");
    assert!(app.status_message().contains("Route placed"));
}

#[test]
pub(crate) fn layout_canvas_route_inherits_endpoint_net_metadata() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("demo document should have metal1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("demo document should have metal2");
    app.workspace.document.shapes.clear();
    app.workspace.document.cells.clear();
    app.add_layout_shape_with_metadata(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-200, -200), Point::new(200, 200))),
        Some(NetId(42)),
        None,
    )
    .expect("source net shape should be added");

    app.active_layer = metal2;
    app.route_points = vec![Point::new(0, 0), Point::new(2_000, 0)];
    app.route_layout_between_points();

    let route_shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("route tool should select the routed path");
    let ShapeKind::Path { .. } = route_shape.kind else {
        panic!("route tool should create a path shape");
    };
    assert_eq!(route_shape.layer, metal2);
    assert_eq!(route_shape.net, Some(NetId(42)));
    assert_eq!(route_shape.name.as_deref(), Some("NET42"));
    assert!(app.status_message().contains("NET42"));
}

#[test]
pub(crate) fn layout_canvas_route_uses_known_net_geometry_as_context() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("same-net route context");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    let start_pad = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(-120, -120), Point::new(120, 120))),
            Some(NetId(7)),
            Some("NET7".to_string()),
        )
        .expect("start pad should be added");
    let middle_segment = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(360, -120), Point::new(640, 120))),
            Some(NetId(7)),
            Some("NET7".to_string()),
        )
        .expect("same-net segment should be added");
    let goal_pad = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(880, -120), Point::new(1_120, 120))),
            Some(NetId(7)),
            Some("NET7".to_string()),
        )
        .expect("goal pad should be added");
    let ignored =
        app.layout_route_ignored_shape_ids(Point::new(0, 0), Point::new(1_000, 0), Some(NetId(7)));
    assert!(ignored.contains(&start_pad));
    assert!(ignored.contains(&middle_segment));
    assert!(ignored.contains(&goal_pad));

    app.route_points = vec![Point::new(0, 0), Point::new(1_000, 0)];
    assert!(app.route_layout_between_points());
    let route_shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("route tool should select the routed path");
    let ShapeKind::Path { points, .. } = &route_shape.kind else {
        panic!("route tool should create a path shape");
    };
    assert!(
        points.iter().all(|point| point.y == 0),
        "same-net geometry should not force a detour: {points:?}"
    );
    assert_eq!(route_shape.net, Some(NetId(7)));
    assert!(
        app.status_message().contains("existing net shape"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_overlay_contains_editor_affordances() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        select_first_shape: true,
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(1280.0, 720.0))
        .expect("layout editor should build");
    assert_eq!(document.audit_layout(), Vec::new());
    let overlay = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.overlay")
        .expect("2D editor overlay should exist");
    let UiContent::Scene(primitives) = overlay.content() else {
        panic!("2D editor overlay should be a scene");
    };
    let preview = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.preview")
        .expect("2D editor canvas should exist");
    let inspector = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.inspector_rail")
        .expect("2D editor should keep detail sections in a left inspector rail");
    assert!(
        inspector.layout().rect.x < preview.layout().rect.x,
        "layout inspector should sit to the left of the canvas; inspector={:?} preview={:?}",
        inspector.layout().rect,
        preview.layout().rect
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.inspector.section.0.header"),
        "layout inspector should expose collapsible detail headers"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.inspector.section.0.body"),
        "layout inspector detail section should start expanded"
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.details"),
        "2D layout should not render the generic below-canvas detail panel"
    );

    assert!(
        overlay.layout().rect.height >= 300.0,
        "overlay should cover the canvas instead of collapsing to zero height: {:?}",
        overlay.layout().rect
    );
    assert!(
        (overlay.layout().rect.x - preview.layout().rect.x).abs() < 0.5
            && (overlay.layout().rect.y - preview.layout().rect.y).abs() < 0.5
            && (overlay.layout().rect.width - preview.layout().rect.width).abs() < 0.5
            && (overlay.layout().rect.height - preview.layout().rect.height).abs() < 0.5,
        "overlay and canvas should be stacked on the same viewport rect; overlay={:?} preview={:?}",
        overlay.layout().rect,
        preview.layout().rect
    );

    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, ScenePrimitive::Line { .. })),
        "grid/selection/scale affordances should draw line primitives"
    );
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            ScenePrimitive::Text(text)
                if text.text.ends_with(" nm")
                    || text.text.ends_with(" um")
                    || text.text.ends_with(" dbu")
        )),
        "2D editor should draw a bottom-left scale ruler label"
    );
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.layout.fps"
                && matches!(node.content(), UiContent::Text(text) if text.text.starts_with("FPS:"))
        }),
        "2D editor should expose an FPS label"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.mode_hud"),
        "2D editor should expose the shared canvas HUD"
    );
    assert!(document_visible_text(&document).contains("2D Layout"));
}

#[test]
pub(crate) fn layout_detail_sections_collapse_in_inline_inspector() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let expanded = app
        .build_operad_document(UiSize::new(1280.0, 720.0))
        .expect("layout editor should build");
    assert!(
        expanded
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.inspector.section.0.body"),
        "layout inspector detail section should start expanded"
    );

    assert!(app.apply_clicked_node_name("glassworks.inspector.section.toggle.layout2d.0.document"));
    let collapsed = app
        .build_operad_document(UiSize::new(1280.0, 720.0))
        .expect("layout editor should rebuild with collapsed inspector");
    assert!(
        collapsed
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.inspector.section.0.header"),
        "collapsed layout inspector should keep the header"
    );
    assert!(
        !collapsed
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.inspector.section.0.body"),
        "collapsed layout inspector should omit the section body"
    );
}

#[test]
pub(crate) fn layout_net_browser_selects_connectivity_components() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("demo document should have metal1");
    app.workspace.document.shapes.clear();
    app.workspace.document.cells.clear();
    app.active_layer = metal1;

    let first = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(-1_000, -200), Point::new(200, 200))),
        )
        .expect("first connected shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(100, -200), Point::new(1_300, 200))),
    )
    .expect("second connected shape should be added");
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;

    let component_id = app
        .connectivity_report()
        .expect("test connectivity should extract")
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in a component");
    let action = format!("glassworks.viewctl.layout.net_component.{component_id}");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_browser.title"),
        "layout inspector should expose a net browser"
    );
    assert!(
        document.nodes().iter().any(|node| node.name() == action),
        "net browser should expose {action}"
    );

    assert!(app.apply_clicked_node_name(&action));
    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(selected_component, Some(component_id));
    assert!(app.status_message().contains("Net component"));
}

#[test]
pub(crate) fn layout_net_browser_select_first_uses_search() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("net browser select first");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");

    let _data = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(-1_000, -200), Point::new(-200, 200))),
            Some(NetId(7)),
            Some("DATA".to_string()),
        )
        .expect("data shape should be added");
    let clk = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(400, -200), Point::new(1_200, 200))),
            Some(NetId(9)),
            Some("CLK".to_string()),
        )
        .expect("clock shape should be added");
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    app.set_layout_browser_search("N9");

    let entries = layout_net_browser_entries(&app);
    assert_eq!(entries.len(), 1, "search should isolate the N9 component");
    let component_id = entries[0].0;
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    assert_eq!(
        report.component_for_occurrence(&ShapeOccurrenceId::top_level(clk)),
        Some(component_id)
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with net select-first should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.net_browser.select_first"),
        "net browser should expose select-first when search has a match"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser.select_first"));
    assert_eq!(app.selected_layout_shape, Some(clk));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(clk))
    );
    assert!(app.status_message().contains("Selected browser net"));
}

#[test]
pub(crate) fn layout_extracted_netlist_exchange_round_trips_devices() {
    let document = Document::new("device netlist exchange");
    let occurrence = ShapeOccurrenceId::top_level(ShapeId(1));
    let report = ConnectivityReport {
        components: vec![NetComponent {
            id: 1,
            shapes: vec![occurrence.clone()],
            bounds: Rect::from_min_size(Point::new(0, 0), 1_000, 300),
            labels: vec![NetLabel {
                occurrence: occurrence.clone(),
                text: "SD".to_string(),
                position: Point::new(100, 100),
            }],
            explicit_nets: Vec::new(),
            net_name: Some("SD".to_string()),
            net_id: None,
        }],
        shape_to_component: BTreeMap::from([(occurrence.clone(), 1)]),
        devices: vec![ExtractedDevice {
            id: 1,
            kind: "mos".to_string(),
            model: "nmos".to_string(),
            terminals: vec![
                ExtractedDeviceTerminal {
                    name: "D".to_string(),
                    component: Some(1),
                    net_name: Some("SD".to_string()),
                },
                ExtractedDeviceTerminal {
                    name: "G".to_string(),
                    component: None,
                    net_name: Some("G".to_string()),
                },
                ExtractedDeviceTerminal {
                    name: "S".to_string(),
                    component: Some(1),
                    net_name: Some("SD".to_string()),
                },
                ExtractedDeviceTerminal {
                    name: "B".to_string(),
                    component: None,
                    net_name: Some("0".to_string()),
                },
            ],
            bounds: Rect::from_min_size(Point::new(450, 0), 100, 300),
            width: 300,
            length: 100,
            occurrences: vec![occurrence.clone()],
        }],
        ..Default::default()
    };

    let exchange =
        LayoutExtractedNetlistExchange::from_report(&document, 7, "demo".to_string(), report);
    assert_eq!(exchange.devices.len(), 1);
    assert_eq!(exchange.devices[0].terminals.len(), 4);

    let round_tripped = exchange.into_report();

    assert_eq!(round_tripped.devices.len(), 1);
    assert_eq!(round_tripped.devices[0].model, "nmos");
    assert_eq!(
        round_tripped.devices[0].terminals[1].net_name.as_deref(),
        Some("G")
    );
    assert_eq!(round_tripped.validate(), Vec::new());
}

#[test]
pub(crate) fn layout_spice_comparison_rows_expose_mismatch_details() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.layout_spice_comparison = Some(SpiceConnectivityComparisonReport {
        circuit_name: "INV".to_string(),
        layout_component_count: 2,
        layout_device_count: 1,
        layout_named_nets: vec!["CLK".to_string(), "SPARE".to_string()],
        schematic_pins: vec!["CLK".to_string()],
        schematic_referenced_nets: vec!["CLK".to_string(), "DATA".to_string()],
        schematic_device_count: 2,
        layout_device_signatures: vec!["M|NMOS|CLK,G,S,0".to_string()],
        schematic_device_signatures: vec![
            "M|NMOS|CLK,G,S,0".to_string(),
            "R|RES|DATA,CLK".to_string(),
        ],
        missing_layout_devices: vec!["R|RES|DATA,CLK".to_string()],
        extra_layout_devices: vec!["C|CAP|SPARE,CLK".to_string()],
        missing_layout_nets: vec!["DATA".to_string()],
        extra_layout_nets: vec!["SPARE".to_string()],
        layout_short_count: 0,
        layout_open_count: 0,
        status: layout_model::connectivity::SpiceConnectivityComparisonStatus::Mismatch,
    });

    let rows = layout_spice_comparison_rows(&app);

    assert!(
        rows.iter()
            .any(|(key, value)| key == "Missing Device Detail" && value.contains("R|RES")),
        "SPICE comparison rows should preview missing device signatures: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Extra Device Detail" && value.contains("C|CAP")),
        "SPICE comparison rows should preview extra device signatures: {rows:?}"
    );

    let component = NetComponent {
        id: 1,
        shapes: Vec::new(),
        bounds: Rect::from_min_size(Point::new(0, 0), 10, 10),
        labels: Vec::new(),
        explicit_nets: Vec::new(),
        net_name: Some("SPARE".to_string()),
        net_id: None,
    };
    assert_eq!(
        layout_spice_component_status_label(&component, app.layout_spice_comparison.as_ref())
            .as_deref(),
        Some("Extra layout net")
    );
    let report = ConnectivityReport {
        components: vec![
            component.clone(),
            NetComponent {
                id: 2,
                shapes: Vec::new(),
                bounds: Rect::from_min_size(Point::new(20, 0), 10, 10),
                labels: Vec::new(),
                explicit_nets: Vec::new(),
                net_name: Some("CLK".to_string()),
                net_id: None,
            },
        ],
        ..Default::default()
    };
    let entries = layout_net_browser_entries_from_report_with_history(
        &report,
        LayoutNetBrowserFilter::SpiceExtra,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    );
    assert_eq!(
        entries.into_iter().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![1]
    );
}
