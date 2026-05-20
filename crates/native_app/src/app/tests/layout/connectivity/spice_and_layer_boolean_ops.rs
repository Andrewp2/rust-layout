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
pub(crate) fn layout_layer_not_selection_not_and_xor_use_current_cell_shapes() {
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
    let child = app.workspace.document.create_cell("layer_not_current_cell");
    let source = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("current-cell source rectangle should be added");
    let selection = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 250), Point::new(1_300, 750))),
        )
        .expect("current-cell selection rectangle should be added");
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(10_000, 0))
        .expect("child should be instanced");
    app.layout_view_top_cell = child;
    app.active_layer = metal1;
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_not_selection"));
    let active_rects = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should exist")
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 3);
    for expected in [
        Rect::new(Point::new(0, 0), Point::new(1_000, 250)),
        Rect::new(Point::new(0, 750), Point::new(1_000, 1_000)),
        Rect::new(Point::new(0, 250), Point::new(500, 750)),
    ] {
        assert!(
            active_rects.contains(&expected),
            "missing current-cell layer NOT fragment {expected:?}"
        );
    }
    assert!(app.workspace.document.shapes.is_empty());
    assert!(
        app.status_message()
            .contains("Subtracted selection from 1 active-layer shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .cell(child)
            .expect("child cell should exist after undo")
            .shapes
            .contains_key(&source)
    );
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.selection_not_layer"));
    let active_rects = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should exist after selection NOT")
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(
        active_rects,
        vec![Rect::new(Point::new(1_000, 250), Point::new(1_300, 750))]
    );
    assert!(app.workspace.document.shapes.is_empty());
    assert!(
        app.status_message()
            .contains("Subtracted 1 active-layer shape from selection")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    app.selected_layout_shape = Some(selection);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selection));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_xor_selection"));
    let active_rects = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should exist after XOR")
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
            "missing current-cell layer XOR fragment {expected:?}"
        );
    }
    assert!(app.workspace.document.shapes.is_empty());
    assert!(app.status_message().contains("XORed 1 active-layer shape"));
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
pub(crate) fn layout_shape_clipboard_booleans_use_current_cell_shapes() {
    for (node_name, label, expected_fragments, expected_area2) in [
        (
            "glassworks.menu.item.edit.shape_and_clipboard",
            "AND",
            1,
            500_000,
        ),
        (
            "glassworks.menu.item.edit.shape_or_clipboard",
            "OR",
            5,
            2_000_000,
        ),
        (
            "glassworks.menu.item.edit.shape_not_clipboard",
            "NOT",
            4,
            1_500_000,
        ),
        (
            "glassworks.menu.item.edit.shape_xor_clipboard",
            "XOR",
            4,
            1_500_000,
        ),
    ] {
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
        let child = app
            .workspace
            .document
            .create_cell("shape_clipboard_current_cell");
        let clipboard = app
            .workspace
            .document
            .insert_shape_in_cell(
                child,
                metal2,
                ShapeKind::Rectangle(Rect::new(Point::new(250, 250), Point::new(750, 750))),
            )
            .expect("current-cell clipboard rectangle should be added");
        let source = app
            .workspace
            .document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
            )
            .expect("current-cell source rectangle should be added");
        app.workspace
            .document
            .insert_instance_in_top(child, Transform::translate(10_000, 0))
            .expect("child should be instanced");
        app.layout_view_top_cell = child;
        app.selected_layout_shape = Some(clipboard);
        app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clipboard));
        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy"));
        assert_eq!(
            app.layout_clipboard_shapes.first().map(|shape| shape.id),
            Some(clipboard),
            "copy should store the current-cell operand for {label}"
        );

        app.selected_layout_shape = Some(source);
        app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));
        assert!(
            app.apply_clicked_node_name(node_name),
            "{label} should apply"
        );
        {
            let child_cell = app
                .workspace
                .document
                .cell(child)
                .expect("child cell should exist");
            assert!(
                !child_cell.shapes.contains_key(&source),
                "{label} should replace the current-cell source shape"
            );
            assert!(
                child_cell.shapes.contains_key(&clipboard),
                "{label} should preserve the copied operand"
            );
            let active_shapes = child_cell
                .shapes
                .values()
                .filter(|shape| shape.layer == metal1)
                .collect::<Vec<_>>();
            assert_eq!(
                active_shapes.len(),
                expected_fragments,
                "{label} should write the expected current-cell fragment count"
            );
            let area2 = active_shapes
                .iter()
                .map(|shape| shape_kind_area2_abs(&shape.kind))
                .sum::<i128>();
            assert_eq!(
                area2, expected_area2,
                "{label} should preserve the expected current-cell result area"
            );
            assert!(
                app.selected_layout_shape
                    .is_some_and(|id| child_cell.shapes.contains_key(&id)),
                "{label} should select a result fragment in the current cell"
            );
        }
        assert!(app.workspace.document.shapes.is_empty());
        assert!(
            app.status_message()
                .contains(&format!("Shape {label} clipboard replaced shape")),
            "{}",
            app.status_message()
        );

        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
        let child_cell = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should still exist after undo");
        assert!(child_cell.shapes.contains_key(&source));
        assert!(child_cell.shapes.contains_key(&clipboard));
        assert_eq!(
            child_cell
                .shapes
                .values()
                .filter(|shape| shape.layer == metal1)
                .count(),
            1,
            "undo should restore the single current-cell source shape for {label}"
        );
        assert!(app.workspace.document.shapes.is_empty());
    }
}

#[test]
pub(crate) fn layout_layer_clipboard_booleans_use_current_cell_active_layer_shapes() {
    let left = Rect::new(Point::new(0, 0), Point::new(200, 1_000));
    let clip_a_bounds = Rect::new(Point::new(200, 0), Point::new(400, 1_000));
    let middle = Rect::new(Point::new(400, 0), Point::new(600, 1_000));
    let clip_b_bounds = Rect::new(Point::new(600, 0), Point::new(800, 1_000));
    let right = Rect::new(Point::new(800, 0), Point::new(1_000, 1_000));
    for (node_name, label, expected_bounds) in [
        (
            "glassworks.menu.item.edit.layer_and_clipboard",
            "AND",
            vec![clip_a_bounds, clip_b_bounds],
        ),
        (
            "glassworks.menu.item.edit.layer_or_clipboard",
            "OR",
            vec![left, clip_a_bounds, middle, clip_b_bounds, right],
        ),
        (
            "glassworks.menu.item.edit.layer_not_clipboard",
            "NOT",
            vec![left, middle, right],
        ),
        (
            "glassworks.menu.item.edit.layer_xor_clipboard",
            "XOR",
            vec![left, middle, right],
        ),
    ] {
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
        let child = app
            .workspace
            .document
            .create_cell("layer_clipboard_current_cell");
        let clip_a = app
            .workspace
            .document
            .insert_shape_in_cell(child, metal2, ShapeKind::Rectangle(clip_a_bounds))
            .expect("first current-cell clipboard rectangle should be added");
        let clip_b = app
            .workspace
            .document
            .insert_shape_in_cell(child, metal2, ShapeKind::Rectangle(clip_b_bounds))
            .expect("second current-cell clipboard rectangle should be added");
        let source = app
            .workspace
            .document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
            )
            .expect("current-cell source rectangle should be added");
        app.workspace
            .document
            .insert_instance_in_top(child, Transform::translate(10_000, 0))
            .expect("child should be instanced");
        app.layout_view_top_cell = child;
        app.active_layer = metal2;
        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy_active_layer"));
        assert_eq!(app.layout_clipboard_shapes.len(), 2);

        app.active_layer = metal1;
        assert!(
            app.apply_clicked_node_name(node_name),
            "{label} should apply"
        );
        let child_cell = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should exist after layer clipboard boolean");
        assert!(!child_cell.shapes.contains_key(&source));
        assert!(child_cell.shapes.contains_key(&clip_a));
        assert!(child_cell.shapes.contains_key(&clip_b));
        let mut active_bounds = child_cell
            .shapes
            .values()
            .filter(|shape| shape.layer == metal1)
            .map(|shape| shape.kind.bounds())
            .collect::<Vec<_>>();
        active_bounds.sort_by_key(|rect| (rect.min.x, rect.min.y, rect.max.x, rect.max.y));
        assert_eq!(
            active_bounds, expected_bounds,
            "{label} should boolean active-layer shapes with both clipboard operands"
        );
        assert!(app.workspace.document.shapes.is_empty());
        assert!(
            app.status_message().contains(&format!(
                "Layer {label} clipboard replaced 1 active-layer shape"
            )),
            "{}",
            app.status_message()
        );
        assert!(
            app.status_message().contains("from 2 clipboard shapes"),
            "{}",
            app.status_message()
        );

        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
        let child_cell = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should exist after undo");
        assert!(child_cell.shapes.contains_key(&source));
        assert!(child_cell.shapes.contains_key(&clip_a));
        assert!(child_cell.shapes.contains_key(&clip_b));
        assert_eq!(
            child_cell
                .shapes
                .values()
                .filter(|shape| shape.layer == metal1)
                .count(),
            1,
            "undo should restore the single current-cell active-layer source for {label}"
        );
    }
}

#[test]
pub(crate) fn layout_shape_clipboard_booleans_use_all_clipboard_shapes() {
    let left = Rect::new(Point::new(0, 0), Point::new(200, 1_000));
    let clip_a_bounds = Rect::new(Point::new(200, 0), Point::new(400, 1_000));
    let middle = Rect::new(Point::new(400, 0), Point::new(600, 1_000));
    let clip_b_bounds = Rect::new(Point::new(600, 0), Point::new(800, 1_000));
    let right = Rect::new(Point::new(800, 0), Point::new(1_000, 1_000));
    for (node_name, label, expected_bounds) in [
        (
            "glassworks.menu.item.edit.shape_and_clipboard",
            "AND",
            vec![clip_a_bounds, clip_b_bounds],
        ),
        (
            "glassworks.menu.item.edit.shape_or_clipboard",
            "OR",
            vec![left, clip_a_bounds, middle, clip_b_bounds, right],
        ),
        (
            "glassworks.menu.item.edit.shape_not_clipboard",
            "NOT",
            vec![left, middle, right],
        ),
        (
            "glassworks.menu.item.edit.shape_xor_clipboard",
            "XOR",
            vec![left, middle, right],
        ),
    ] {
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
        let child = app
            .workspace
            .document
            .create_cell("shape_multi_clipboard_current_cell");
        let clip_a = app
            .workspace
            .document
            .insert_shape_in_cell(child, metal2, ShapeKind::Rectangle(clip_a_bounds))
            .expect("first current-cell clipboard rectangle should be added");
        let clip_b = app
            .workspace
            .document
            .insert_shape_in_cell(child, metal2, ShapeKind::Rectangle(clip_b_bounds))
            .expect("second current-cell clipboard rectangle should be added");
        let source = app
            .workspace
            .document
            .insert_shape_in_cell(
                child,
                metal1,
                ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
            )
            .expect("current-cell source rectangle should be added");
        app.workspace
            .document
            .insert_instance_in_top(child, Transform::translate(10_000, 0))
            .expect("child should be instanced");
        app.layout_view_top_cell = child;
        app.active_layer = metal2;
        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.copy_active_layer"));
        assert_eq!(app.layout_clipboard_shapes.len(), 2);
        assert!(
            app.status_message()
                .contains("Copied 2 active-layer shapes")
        );
        app.selected_layout_shape = Some(source);
        app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source));

        assert!(
            app.apply_clicked_node_name(node_name),
            "{label} should apply"
        );
        let mut active_bounds = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should exist after boolean")
            .shapes
            .values()
            .filter(|shape| shape.layer == metal1)
            .map(|shape| shape.kind.bounds())
            .collect::<Vec<_>>();
        active_bounds.sort_by_key(|rect| (rect.min.x, rect.min.y, rect.max.x, rect.max.y));
        assert_eq!(
            active_bounds, expected_bounds,
            "{label} should use both clipboard shapes"
        );
        let child_cell = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should still exist");
        assert!(!child_cell.shapes.contains_key(&source));
        assert!(child_cell.shapes.contains_key(&clip_a));
        assert!(child_cell.shapes.contains_key(&clip_b));
        assert!(app.workspace.document.shapes.is_empty());
        assert!(
            app.status_message().contains("from 2 clipboard shapes"),
            "{}",
            app.status_message()
        );

        assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
        let child_cell = app
            .workspace
            .document
            .cell(child)
            .expect("child cell should exist after undo");
        assert!(child_cell.shapes.contains_key(&source));
        assert!(child_cell.shapes.contains_key(&clip_a));
        assert!(child_cell.shapes.contains_key(&clip_b));
    }
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
    app.workspace.document = Document::new("spice compare issue browser");
    app.reset_layout_document_state();
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
    let spare_shape = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 1_000)),
        )
        .expect("SPARE layout net should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(300, 300),
            text: "spare".to_string(),
        },
    )
    .expect("SPARE label should be added");
    let clk_shape = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 600, 600)),
        )
        .expect("CLK capacitor plate should be added");
    app.add_layout_shape(
        metal2,
        ShapeKind::Label {
            position: Point::new(700, 700),
            text: "clk".to_string(),
        },
    )
    .expect("CLK label should be added");
    let extracted = app
        .connectivity_report()
        .expect("SPARE/CLK nets should extract for compare issue cross-probe");
    let spare_component = extracted
        .component_for_occurrence(&ShapeOccurrenceId::top_level(spare_shape))
        .expect("SPARE shape should map to an extracted component");
    let clk_component = extracted
        .component_for_occurrence(&ShapeOccurrenceId::top_level(clk_shape))
        .expect("CLK shape should map to an extracted component");
    let extra_device = extracted
        .devices
        .iter()
        .find(|device| device.kind == "capacitor")
        .expect("overlapping plates should extract a capacitor");
    assert_eq!(
        layout_model::connectivity::layout_spice_device_signature(&extracted, extra_device, &[])
            .as_deref(),
        Some("C|CAP|CLK,SPARE")
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let schematic_path = std::env::temp_dir().join(format!(
            "glassworks-spice-device-mismatch-{}.spice",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&schematic_path);
        std::fs::write(
            &schematic_path,
            ".subckt inv CLK SPARE\nR1 CLK SPARE 1\n.ends inv\n",
        )
        .expect("device-mismatch SPICE fixture should be written");

        assert!(app.compare_layout_spice_schematic_from_path(&schematic_path));
        assert_eq!(
            app.layout_net_browser_filter,
            LayoutNetBrowserFilter::SpiceIssues
        );
        assert!(
            app.status_message()
                .contains("SPICE schematic compare mismatch"),
            "{}",
            app.status_message()
        );
        let comparison = app
            .layout_spice_comparison
            .as_ref()
            .expect("device-mismatch compare should cache a report");
        assert!(comparison.missing_layout_nets.is_empty());
        assert!(comparison.extra_layout_nets.is_empty());
        assert_eq!(
            comparison.missing_layout_devices.len() + comparison.extra_layout_devices.len(),
            2
        );
        let _ = std::fs::remove_file(&schematic_path);
    }
    app.layout_spice_comparison = Some(SpiceConnectivityComparisonReport {
        circuit_name: "INV".to_string(),
        layout_component_count: 2,
        layout_device_count: 1,
        layout_named_nets: vec!["CLK".to_string(), "SPARE".to_string()],
        schematic_pins: vec!["CLK".to_string()],
        schematic_referenced_nets: vec!["CLK".to_string(), "DATA".to_string()],
        schematic_device_count: 2,
        layout_device_signatures: vec!["C|CAP|CLK,SPARE".to_string()],
        schematic_device_signatures: vec![
            "M|NMOS|CLK,G,S,0".to_string(),
            "R|RES|DATA,CLK".to_string(),
        ],
        missing_layout_devices: vec!["R|RES|DATA,CLK".to_string()],
        extra_layout_devices: vec!["C|CAP|CLK,SPARE".to_string()],
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
    assert!(
        rows.iter()
            .any(|(key, value)| key == "SPICE Issue Nets" && value == "2"),
        "SPICE comparison rows should expose issue-net count: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "SPICE Missing Issue Nets" && value == "1"),
        "SPICE comparison rows should expose missing issue-net count: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "SPICE Extra Issue Nets" && value == "2"),
        "SPICE comparison rows should expose extra issue-net count: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "SPICE Net Issue Nets" && value == "1"),
        "SPICE comparison rows should expose net-level issue-net count: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "SPICE Device Issue Nets" && value == "2"),
        "SPICE comparison rows should expose device-level issue-net count: {rows:?}"
    );
    let comparison = app
        .layout_spice_comparison
        .as_ref()
        .expect("comparison fixture should be set");
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            None,
            LayoutNetBrowserFilter::All,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec![
            "missing_net.0".to_string(),
            "extra_net.0".to_string(),
            "missing_device.0".to_string(),
            "extra_device.0".to_string()
        ],
        "SPICE compare browser should expose all mismatch classes"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            Some("extra layout net"),
            LayoutNetBrowserFilter::All,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["extra_net.0".to_string()],
        "SPICE compare issue rows should share browser search"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            Some("R|RES"),
            LayoutNetBrowserFilter::All,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["missing_device.0".to_string()],
        "SPICE compare issue search should match device signatures"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            None,
            LayoutNetBrowserFilter::SpiceNets,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["extra_net.0".to_string(), "missing_net.0".to_string()],
        "SPICE Nets filter should list cross-probeable extra-net rows before schematic-only missing nets"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            None,
            LayoutNetBrowserFilter::SpiceMissing,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["missing_device.0".to_string(), "missing_net.0".to_string()],
        "SPICE Missing filter should list cross-probeable missing-device rows before schematic-only missing nets"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            None,
            LayoutNetBrowserFilter::SpiceDevices,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["missing_device.0".to_string(), "extra_device.0".to_string()],
        "SPICE Devices filter should list only device-level compare issue rows"
    );
    assert_eq!(
        layout_spice_comparison_issue_entries_for_filter(
            comparison,
            None,
            LayoutNetBrowserFilter::SpiceExtra,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>(),
        vec!["extra_net.0".to_string(), "extra_device.0".to_string()],
        "SPICE Extra filter should list only extra-layout compare issue rows"
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
    let spice_extra_entries = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::SpiceExtra,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert!(
        spice_extra_entries.contains(&spare_component),
        "SPICE Extra filter should include the explicitly extra layout net: {spice_extra_entries:?}"
    );
    assert!(
        spice_extra_entries.contains(&clk_component),
        "SPICE Extra filter should include nets connected only through an extra layout device: {spice_extra_entries:?}"
    );
    let spice_issue_entries = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::SpiceIssues,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        spice_issue_entries,
        vec![spare_component, clk_component],
        "SPICE Issues filter should include all nets with compare mismatch status"
    );
    let spice_net_issue_entries = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::SpiceNets,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        spice_net_issue_entries,
        vec![spare_component],
        "SPICE Nets filter should include only net-level compare mismatches"
    );
    let spice_missing_issue_entries = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::SpiceMissing,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        spice_missing_issue_entries,
        vec![clk_component],
        "SPICE Missing filter should include nets referenced by missing layout device issues"
    );
    let spice_device_issue_entries = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::SpiceDevices,
        LayoutNetBrowserSort::Id,
        None,
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        spice_device_issue_entries,
        vec![spare_component, clk_component],
        "SPICE Devices filter should include nets with device-level compare mismatches"
    );
    let extra_device_search = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("extra layout device"),
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        extra_device_search,
        vec![spare_component, clk_component],
        "net browser search should match nets implicated by extra layout devices"
    );
    let missing_device_search = layout_net_browser_entries_from_report_with_history(
        &extracted,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("missing layout device"),
        &[],
        app.layout_spice_comparison.as_ref(),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        missing_device_search,
        vec![clk_component],
        "net browser search should match nets referenced by schematic-only devices"
    );
    let clk_rows =
        layout_net_browser_property_rows(&app, &extracted, &[(clk_component, "CLK".to_string())]);
    assert!(
        clk_rows
            .iter()
            .any(|(key, value)| key == "SPICE Net" && value == "Matched schematic net"),
        "selected matched net should keep its net-level SPICE status: {clk_rows:?}"
    );
    assert!(
        clk_rows.iter().any(|(key, value)| {
            key == "SPICE Device" && value.contains("Extra layout device C|CAP|CLK,SPARE")
        }),
        "selected net should expose extra layout device status separately: {clk_rows:?}"
    );
    assert!(
        clk_rows.iter().any(|(key, value)| {
            key == "SPICE Device" && value.contains("Missing layout device")
        }),
        "selected net should expose missing layout device status when a schematic-only device references the net: {clk_rows:?}"
    );

    let extra_net_action = "glassworks.viewctl.layout.spice_compare_issue.extra_net.0";
    let extra_device_action = "glassworks.viewctl.layout.spice_compare_issue.extra_device.0";
    let first_spice_issue_action = "glassworks.viewctl.layout.spice_compare_issue.select_first";
    let clear_spice_compare_action = "glassworks.viewctl.layout.spice_compare.clear";
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with SPICE compare issues should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.spice_compare_issues.title"),
        "net browser should expose SPICE compare issue rows"
    );
    let compare_issue_title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_issues.title")
        .unwrap_or_else(|| panic!("net browser should expose SPICE compare issue title"));
    let UiContent::Text(compare_issue_title_text) = compare_issue_title.content() else {
        panic!("SPICE compare issue title should be text");
    };
    assert_eq!(
        compare_issue_title_text.text,
        "SPICE Compare Issues (4 rows)"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.net_browser_filter.spice_issues"),
        "net browser should expose the SPICE Issues filter"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.net_browser_filter.spice_nets"),
        "net browser should expose the SPICE Nets filter"
    );
    assert!(
        document.nodes().iter().any(|node| node.name()
            == "glassworks.viewctl.layout.net_browser_filter.spice_missing"),
        "net browser should expose the SPICE Missing filter"
    );
    assert!(
        document.nodes().iter().any(|node| node.name()
            == "glassworks.viewctl.layout.net_browser_filter.spice_devices"),
        "net browser should expose the SPICE Devices filter"
    );
    let filter_button_label = |node_name: &str| -> String {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .and_then(|node| node.accessibility())
            .and_then(|accessibility| accessibility.label.clone())
            .unwrap_or_else(|| panic!("{node_name} should expose an accessibility label"))
    };
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.net_browser_filter.spice_extra"),
        "SPICE Extra (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.net_browser_filter.spice_missing"),
        "SPICE Missing (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.net_browser_filter.spice_nets"),
        "SPICE Nets (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.net_browser_filter.spice_devices"),
        "SPICE Devices (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.net_browser_filter.spice_issues"),
        "SPICE Issues (2)"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.spice_nets"));
    assert_eq!(
        app.layout_net_browser_filter,
        LayoutNetBrowserFilter::SpiceNets
    );
    assert!(
        app.status_message()
            .contains("Net browser filter SPICE Nets (1)"),
        "{}",
        app.status_message()
    );
    let net_filtered_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with SPICE net issue filter should build");
    let net_filtered_issue_title = net_filtered_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_issues.title")
        .unwrap_or_else(|| panic!("SPICE Nets filter should expose compare issue title"));
    let UiContent::Text(net_filtered_issue_title_text) = net_filtered_issue_title.content() else {
        panic!("SPICE Nets compare issue title should be text");
    };
    assert_eq!(
        net_filtered_issue_title_text.text,
        "SPICE Compare Issues - SPICE Nets (2 rows)"
    );
    assert!(
        net_filtered_document
            .nodes()
            .iter()
            .any(|node| node.name() == extra_net_action),
        "SPICE Nets filter should keep net-level compare issue rows"
    );
    assert!(
        !net_filtered_document
            .nodes()
            .iter()
            .any(|node| node.name() == extra_device_action),
        "SPICE Nets filter should hide device-level compare issue rows"
    );
    let compare_status = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.status")
        .unwrap_or_else(|| panic!("net browser should expose SPICE compare status summary"));
    let UiContent::Text(compare_status_text) = compare_status.content() else {
        panic!("SPICE compare status summary should be text");
    };
    assert_eq!(compare_status_text.text, "Status: mismatch");
    let compare_devices = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.device_mismatches")
        .unwrap_or_else(|| panic!("net browser should expose SPICE device mismatch summary"));
    let UiContent::Text(compare_devices_text) = compare_devices.content() else {
        panic!("SPICE device mismatch summary should be text");
    };
    assert_eq!(
        compare_devices_text.text,
        "Device mismatches: 1 missing / 1 extra"
    );
    let issue_nets = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.issue_nets")
        .unwrap_or_else(|| panic!("net browser should expose SPICE issue net summary"));
    let UiContent::Text(issue_nets_text) = issue_nets.content() else {
        panic!("SPICE issue net summary should be text");
    };
    assert_eq!(issue_nets_text.text, "Issue nets: 2");
    let missing_issue_nets = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.missing_issue_nets")
        .unwrap_or_else(|| panic!("net browser should expose SPICE missing issue summary"));
    let UiContent::Text(missing_issue_nets_text) = missing_issue_nets.content() else {
        panic!("SPICE missing issue summary should be text");
    };
    assert_eq!(missing_issue_nets_text.text, "Missing issue nets: 1");
    let extra_issue_nets = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.extra_issue_nets")
        .unwrap_or_else(|| panic!("net browser should expose SPICE extra issue summary"));
    let UiContent::Text(extra_issue_nets_text) = extra_issue_nets.content() else {
        panic!("SPICE extra issue summary should be text");
    };
    assert_eq!(extra_issue_nets_text.text, "Extra issue nets: 2");
    let net_issue_nets = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.net_issue_nets")
        .unwrap_or_else(|| panic!("net browser should expose SPICE net issue summary"));
    let UiContent::Text(net_issue_nets_text) = net_issue_nets.content() else {
        panic!("SPICE net issue summary should be text");
    };
    assert_eq!(net_issue_nets_text.text, "Net issue nets: 1");
    let device_issue_nets = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.spice_compare_summary.device_issue_nets")
        .unwrap_or_else(|| panic!("net browser should expose SPICE device issue summary"));
    let UiContent::Text(device_issue_nets_text) = device_issue_nets.content() else {
        panic!("SPICE device issue summary should be text");
    };
    assert_eq!(device_issue_nets_text.text, "Device issue nets: 2");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == extra_net_action),
        "net browser should expose {extra_net_action}"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == extra_device_action),
        "net browser should expose {extra_device_action}"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == first_spice_issue_action),
        "net browser should expose a direct select-first SPICE issue action"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == clear_spice_compare_action),
        "net browser should expose a clear SPICE compare action"
    );
    app.layout_browser_search = "extra layout net".to_string();
    assert!(app.apply_clicked_node_name(first_spice_issue_action));
    assert_eq!(app.selected_layout_shape, Some(spare_shape));
    assert_eq!(app.layout_trace_history.first(), Some(&spare_component));
    assert!(
        app.status_message()
            .contains("Selected SPICE extra layout net SPARE"),
        "{}",
        app.status_message()
    );
    app.layout_browser_search.clear();
    assert!(app.apply_clicked_node_name(extra_net_action));
    assert_eq!(app.selected_layout_shape, Some(spare_shape));
    assert_eq!(app.layout_trace_history.first(), Some(&spare_component));
    assert!(
        app.status_message()
            .contains("Selected SPICE extra layout net SPARE"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name(extra_device_action));
    assert!(
        [spare_shape, clk_shape].contains(
            &app.selected_layout_shape
                .expect("extra device source should be selected")
        ),
        "extra device cross-probe should select one extracted source shape"
    );
    assert!(
        app.status_message()
            .contains("Selected SPICE extra layout device C|CAP|CLK,SPARE"),
        "{}",
        app.status_message()
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.spice_compare_issue.missing_net.0")
    );
    assert!(
        app.status_message()
            .contains("SPICE missing layout net DATA is not present in current connectivity"),
        "{}",
        app.status_message()
    );
    app.layout_browser_search.clear();
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.spice_missing")
    );
    assert!(app.apply_clicked_node_name(first_spice_issue_action));
    assert_eq!(app.selected_layout_shape, Some(clk_shape));
    assert_eq!(app.layout_trace_history.first(), Some(&clk_component));
    assert!(
        app.status_message()
            .contains("Selected SPICE missing layout device reference R|RES"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.spice_nets"));
    assert!(app.apply_clicked_node_name(first_spice_issue_action));
    assert_eq!(app.selected_layout_shape, Some(spare_shape));
    assert_eq!(app.layout_trace_history.first(), Some(&spare_component));
    assert!(
        app.status_message()
            .contains("Selected SPICE extra layout net SPARE"),
        "{}",
        app.status_message()
    );

    app.layout_browser_search = "R|RES".to_string();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.netlist.select_first"));
    assert!(
        app.status_message().contains(
            "Netlist browser has no matching devices, issues, or SPICE Nets compare issues for search R|RES"
        ),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name(first_spice_issue_action));
    assert!(
        app.status_message()
            .contains("SPICE compare has no matching SPICE Nets issues for search R|RES"),
        "{}",
        app.status_message()
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.spice_devices")
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.netlist.select_first"));
    assert_eq!(app.selected_layout_shape, Some(clk_shape));
    assert_eq!(app.layout_trace_history.first(), Some(&clk_component));
    assert!(
        app.status_message()
            .contains("Selected SPICE missing layout device reference R|RES"),
        "{}",
        app.status_message()
    );
    app.layout_net_browser_filter = LayoutNetBrowserFilter::SpiceMissing;
    assert!(app.apply_clicked_node_name(clear_spice_compare_action));
    assert!(app.layout_spice_comparison.is_none());
    assert_eq!(app.layout_net_browser_filter, LayoutNetBrowserFilter::All);
    assert!(
        app.status_message()
            .contains("Cleared SPICE schematic compare"),
        "{}",
        app.status_message()
    );
    let cleared_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document without SPICE compare should build");
    assert!(
        !cleared_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.spice_compare_summary.title"),
        "clear action should remove the SPICE compare summary"
    );
}
