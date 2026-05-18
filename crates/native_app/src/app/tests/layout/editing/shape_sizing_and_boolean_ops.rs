#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_shapewise_sizing_grows_and_shrinks_selected_convex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 500),
                Point::new(0, 500),
            ])),
        )
        .expect("test polygon should be added");
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow"));
    let grown = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("grown polygon should exist");
    let ShapeKind::Polygon(poly) = &grown.kind else {
        panic!("polygon sizing should preserve polygon geometry");
    };
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(
            Point::new(-amount, -amount),
            Point::new(1_000 + amount, 500 + amount)
        ))
    );
    assert!(app.status_message().contains("Grew"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shrink"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("shrunk polygon should exist");
    let ShapeKind::Polygon(poly) = &restored.kind else {
        panic!("polygon sizing should preserve polygon geometry");
    };
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );
    assert_eq!(poly.signed_area2().abs(), 1_000_000);
    assert!(app.status_message().contains("Shrank"));
}

#[test]
pub(crate) fn layout_shapewise_sizing_accepts_selected_nonconvex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let original = ShapeKind::Polygon(Polygon::new(vec![
        Point::new(0, 0),
        Point::new(1_000, 0),
        Point::new(1_000, 400),
        Point::new(400, 400),
        Point::new(400, 1_000),
        Point::new(0, 1_000),
    ]));
    let shape_id = app
        .add_layout_shape(app.active_layer(), original.clone())
        .expect("test non-convex polygon should be added");
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow"));
    let grown = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("grown polygon should exist");
    let ShapeKind::Polygon(poly) = &grown.kind else {
        panic!("non-convex polygon sizing should preserve polygon geometry");
    };
    assert!(!is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(
            Point::new(-amount, -amount),
            Point::new(1_000 + amount, 1_000 + amount)
        ))
    );
    assert!(poly.signed_area2().abs() > 1_280_000);
    assert!(app.status_message().contains("Grew"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shrink"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("shrunk polygon should exist");
    assert_eq!(restored.kind, original);
    assert!(app.status_message().contains("Shrank"));
}

#[test]
pub(crate) fn layout_chamfer_corners_converts_selected_rectangle_to_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
        )
        .expect("test rectangle should be added");
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.chamfer_corners"));
    let chamfered = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("chamfered shape should exist");
    let ShapeKind::Polygon(poly) = &chamfered.kind else {
        panic!("chamfering should convert a rectangle to a polygon");
    };
    assert_eq!(
        poly.points,
        vec![
            Point::new(amount, 0),
            Point::new(1_000 - amount, 0),
            Point::new(1_000, amount),
            Point::new(1_000, 500 - amount),
            Point::new(1_000 - amount, 500),
            Point::new(amount, 500),
            Point::new(0, 500 - amount),
            Point::new(0, amount),
        ]
    );
    assert!(app.status_message().contains("Chamfered shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("restored shape should exist");
    assert_eq!(
        restored.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );
}

#[test]
pub(crate) fn layout_chamfer_corners_accepts_selected_convex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("test polygon should be added");
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.chamfer_corners"));
    let chamfered = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("chamfered shape should exist");
    let ShapeKind::Polygon(poly) = &chamfered.kind else {
        panic!("chamfering should keep polygon geometry");
    };
    assert_eq!(poly.points.len(), 8);
    assert!(is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000)))
    );
    assert_eq!(
        poly.signed_area2().abs(),
        2_000_000 - 4 * i128::from(amount) * i128::from(amount)
    );
    assert!(app.status_message().contains("Chamfered shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("restored shape should exist");
    assert_eq!(
        restored.kind,
        ShapeKind::Polygon(Polygon::new(vec![
            Point::new(0, 0),
            Point::new(1_000, 0),
            Point::new(1_000, 1_000),
            Point::new(0, 1_000),
        ]))
    );
}

#[test]
pub(crate) fn layout_chamfer_corners_accepts_selected_nonconvex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("test non-convex polygon should be added");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.chamfer_corners"));
    let chamfered = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("chamfered shape should exist");
    let ShapeKind::Polygon(poly) = &chamfered.kind else {
        panic!("chamfering should keep polygon geometry");
    };
    assert_eq!(poly.points.len(), 12);
    assert!(!is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000)))
    );
    assert!(app.status_message().contains("Chamfered shape"));
}

#[test]
pub(crate) fn layout_round_corners_converts_selected_rectangle_to_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
        )
        .expect("test rectangle should be added");
    let radius = app.layout_size_step();
    let diagonal = ((radius as i128 * 293 + 500) / 1_000) as Coord;

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.round_corners"));
    let rounded = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("rounded shape should exist");
    let ShapeKind::Polygon(poly) = &rounded.kind else {
        panic!("rounding should convert a rectangle to a polygon");
    };
    assert_eq!(
        poly.points,
        vec![
            Point::new(radius, 0),
            Point::new(1_000 - radius, 0),
            Point::new(1_000 - diagonal, diagonal),
            Point::new(1_000, radius),
            Point::new(1_000, 500 - radius),
            Point::new(1_000 - diagonal, 500 - diagonal),
            Point::new(1_000 - radius, 500),
            Point::new(radius, 500),
            Point::new(diagonal, 500 - diagonal),
            Point::new(0, 500 - radius),
            Point::new(0, radius),
            Point::new(diagonal, diagonal),
        ]
    );
    assert!(app.status_message().contains("Rounded shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("restored shape should exist");
    assert_eq!(
        restored.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );
}

#[test]
pub(crate) fn layout_round_corners_accepts_selected_convex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("test polygon should be added");
    let radius = app.layout_size_step();
    let diagonal = ((radius as i128 * 293 + 500) / 1_000) as Coord;

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.round_corners"));
    let rounded = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("rounded shape should exist");
    let ShapeKind::Polygon(poly) = &rounded.kind else {
        panic!("rounding should keep polygon geometry");
    };
    assert_eq!(poly.points.len(), 12);
    assert!(is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000)))
    );
    assert_eq!(
        poly.signed_area2().abs(),
        2_000_000 - 8 * i128::from(radius) * i128::from(diagonal)
    );
    assert!(app.status_message().contains("Rounded shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("restored shape should exist");
    assert_eq!(
        restored.kind,
        ShapeKind::Polygon(Polygon::new(vec![
            Point::new(0, 0),
            Point::new(1_000, 0),
            Point::new(1_000, 1_000),
            Point::new(0, 1_000),
        ]))
    );
}

#[test]
pub(crate) fn layout_round_corners_accepts_selected_nonconvex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 400),
                Point::new(400, 400),
                Point::new(400, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("test non-convex polygon should be added");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.round_corners"));
    let rounded = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("rounded shape should exist");
    let ShapeKind::Polygon(poly) = &rounded.kind else {
        panic!("rounding should keep polygon geometry");
    };
    assert_eq!(poly.points.len(), 18);
    assert!(!is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000)))
    );
    assert!(app.status_message().contains("Rounded shape"));
}

#[test]
pub(crate) fn layout_layer_sizing_grows_and_shrinks_active_layer_shapes() {
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
    app.active_layer = metal1;
    let rect_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
        )
        .expect("active layer rectangle should be added");
    let path_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Path {
                points: vec![Point::new(0, 1_000), Point::new(1_000, 1_000)],
                width: 100,
            },
        )
        .expect("active layer path should be added");
    let via_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Via {
                center: Point::new(2_000, 2_000),
                size: 120,
                lower: metal1,
                upper: metal2,
            },
        )
        .expect("active layer via should be added");
    let polygon_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(3_000, 0),
                Point::new(4_000, 0),
                Point::new(4_000, 500),
                Point::new(3_000, 500),
            ])),
        )
        .expect("active layer polygon should be added");
    let other_id = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(5_000, 0), Point::new(5_500, 500))),
        )
        .expect("other layer rectangle should be added");
    app.active_layer = metal1;
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow_layer"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&rect_id)
            .map(|shape| shape.kind.clone()),
        Some(ShapeKind::Rectangle(Rect::new(
            Point::new(-amount, -amount),
            Point::new(1_000 + amount, 500 + amount)
        )))
    );
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&path_id)
            .map(|shape| shape.kind.clone()),
        Some(ShapeKind::Path {
            points: vec![Point::new(0, 1_000), Point::new(1_000, 1_000)],
            width: 100 + amount * 2,
        })
    );
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&via_id)
            .map(|shape| shape.kind.clone()),
        Some(ShapeKind::Via {
            center: Point::new(2_000, 2_000),
            size: 120 + amount * 2,
            lower: metal1,
            upper: metal2,
        })
    );
    let grown_polygon = app
        .workspace
        .document
        .shapes
        .get(&polygon_id)
        .expect("active layer polygon should still exist");
    assert!(matches!(grown_polygon.kind, ShapeKind::Polygon(_)));
    assert_eq!(
        grown_polygon.kind.bounds(),
        Rect::new(
            Point::new(3_000 - amount, -amount),
            Point::new(4_000 + amount, 500 + amount)
        )
    );
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&other_id)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(5_000, 0), Point::new(5_500, 500))),
        "layer sizing should leave other layers unchanged"
    );
    assert!(app.status_message().contains("Grew 4 active-layer shapes"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&rect_id)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow_layer"));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shrink_layer"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&rect_id)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );
    assert!(
        app.status_message()
            .contains("Shrank 4 active-layer shapes")
    );
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&polygon_id)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(3_000, 0), Point::new(4_000, 500)))
    );
}

#[test]
pub(crate) fn layout_layer_sizing_accepts_active_layer_nonconvex_polygon() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.workspace.document.shapes.clear();
    app.active_layer = metal1;
    let original = ShapeKind::Polygon(Polygon::new(vec![
        Point::new(0, 0),
        Point::new(1_000, 0),
        Point::new(1_000, 400),
        Point::new(400, 400),
        Point::new(400, 1_000),
        Point::new(0, 1_000),
    ]));
    let polygon_id = app
        .add_layout_shape(metal1, original.clone())
        .expect("active layer non-convex polygon should be added");
    let amount = app.layout_size_step();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow_layer"));
    let grown = app
        .workspace
        .document
        .shapes
        .get(&polygon_id)
        .expect("active layer polygon should still exist");
    let ShapeKind::Polygon(poly) = &grown.kind else {
        panic!("layer sizing should preserve polygon geometry");
    };
    assert!(!is_convex_polygon(&poly.points));
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(
            Point::new(-amount, -amount),
            Point::new(1_000 + amount, 1_000 + amount)
        ))
    );
    assert!(app.status_message().contains("Grew 1 active-layer shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.shrink_layer"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&polygon_id)
            .map(|shape| shape.kind.clone()),
        Some(original)
    );
}

#[test]
pub(crate) fn layout_alignment_moves_selected_shape_to_active_layer_edges() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.workspace.document.shapes.clear();
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, -200), Point::new(800, 1_200))),
    )
    .expect("anchor rectangle should be added");
    let selected = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(1_000, 500), Point::new(1_300, 900))),
        )
        .expect("selected rectangle should be added");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_left"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, 500), Point::new(300, 900)))
    );
    assert!(app.status_message().contains("Aligned shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_bottom"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, -200), Point::new(300, 200)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_right"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(500, -200), Point::new(800, 200)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_top"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(500, 800), Point::new(800, 1_200)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_center_x"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(250, 800), Point::new(550, 1_200)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_center_y"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(250, 300), Point::new(550, 700)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_origin_x"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(-150, 300), Point::new(150, 700)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.align_origin_y"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(-150, -200), Point::new(150, 200)))
    );

    app.active_menu = Some(AppMenu::Edit);
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.menu.item.edit.align_center_x",
        "glassworks.menu.item.edit.align_center_y",
        "glassworks.menu.item.edit.align_origin_x",
        "glassworks.menu.item.edit.align_origin_y",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "Edit menu should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(-150, 300), Point::new(150, 700)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(250, 300), Point::new(550, 700)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(250, 800), Point::new(550, 1_200)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(500, 800), Point::new(800, 1_200)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(500, -200), Point::new(800, 200)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, -200), Point::new(300, 200)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(0, 500), Point::new(300, 900)))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&selected)
            .map(|shape| shape.kind.bounds()),
        Some(Rect::new(Point::new(1_000, 500), Point::new(1_300, 900)))
    );
}

#[test]
pub(crate) fn layout_layer_rectangle_merge_combines_touching_active_layer_rects() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.workspace.document.shapes.clear();
    app.workspace.document.cells.clear();
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
    )
    .expect("first rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(900, 0), Point::new(1_600, 500))),
    )
    .expect("overlapping rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(3_000, 0), Point::new(3_500, 500))),
    )
    .expect("separate rectangle should be added");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.merge_layer_rects"));
    assert_eq!(app.workspace.document.shapes.len(), 2);
    assert!(
        app.workspace.document.shapes.values().any(|shape| {
            shape.layer == metal1
                && matches!(
                    shape.kind,
                    ShapeKind::Rectangle(rect)
                        if rect == Rect::new(Point::new(0, 0), Point::new(1_600, 500))
                )
        }),
        "merge should replace touching rectangles with exact union geometry"
    );
    assert!(app.status_message().contains("Merged 2 rectangles"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(app.workspace.document.shapes.len(), 3);
}

#[test]
pub(crate) fn layout_layer_rectangle_merge_preserves_l_shape_area() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.workspace.document.shapes.clear();
    app.workspace.document.cells.clear();
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
    )
    .expect("first L rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(500, 1_000))),
    )
    .expect("second L rectangle should be added");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.merge_layer_rects"));
    let merged_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .filter_map(|shape| match shape.kind {
            ShapeKind::Rectangle(rect) => Some(rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(merged_rects.len(), 2);
    assert_eq!(
        merged_rects.iter().map(|rect| rect.area()).sum::<Coord>(),
        750_000,
        "L-shape merge should not fill the missing quadrant"
    );
    assert!(
        !merged_rects
            .iter()
            .any(|rect| *rect == Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        "L-shape merge must not collapse to a bounding box"
    );
    assert!(app.status_message().contains("Merged 2 rectangles"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(app.workspace.document.shapes.len(), 2);
}

#[test]
pub(crate) fn layout_layer_rectangle_and_clips_active_layer_to_selected_rect() {
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
    let inside = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
        )
        .expect("inside rectangle should be added");
    let partial = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(900, 100), Point::new(1_600, 600))),
        )
        .expect("partial rectangle should be added");
    let outside = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(3_000, 0), Point::new(3_500, 500))),
        )
        .expect("outside rectangle should be added");
    let clip = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 50), Point::new(1_200, 450))),
        )
        .expect("clip rectangle should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_and_selection"));
    assert!(
        !app.workspace.document.shapes.contains_key(&inside),
        "layer AND should replace original active-layer rectangles"
    );
    assert!(!app.workspace.document.shapes.contains_key(&partial));
    assert!(!app.workspace.document.shapes.contains_key(&outside));
    assert!(
        app.workspace.document.shapes.contains_key(&clip),
        "clip selection on another layer should be preserved"
    );
    let active_rects = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(active_rects.len(), 2);
    assert!(active_rects.contains(&Rect::new(Point::new(500, 50), Point::new(1_000, 450))));
    assert!(active_rects.contains(&Rect::new(Point::new(900, 100), Point::new(1_200, 450))));
    assert!(
        app.status_message()
            .contains("Intersected 3 active-layer shapes")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&inside));
    assert!(app.workspace.document.shapes.contains_key(&partial));
    assert!(app.workspace.document.shapes.contains_key(&outside));
    assert!(app.workspace.document.shapes.contains_key(&clip));
}

#[test]
pub(crate) fn layout_layer_and_clips_active_polygon_to_selected_convex_polygon() {
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
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(-100, -100),
                Point::new(1_100, -100),
                Point::new(1_100, 1_100),
                Point::new(-100, 1_100),
            ])),
        )
        .expect("source polygon should be added");
    let clip = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(250, 250),
                Point::new(750, 250),
                Point::new(500, 750),
            ])),
        )
        .expect("clip polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_and_selection"));
    assert!(
        !app.workspace.document.shapes.contains_key(&source),
        "layer AND should replace original active-layer polygon"
    );
    assert!(
        app.workspace.document.shapes.contains_key(&clip),
        "clip polygon on another layer should be preserved"
    );
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert_eq!(active_shapes.len(), 1);
    let ShapeKind::Polygon(poly) = &active_shapes[0].kind else {
        panic!("convex-polygon clip should write polygon geometry");
    };
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(250, 250), Point::new(750, 750)))
    );
    assert_eq!(poly.signed_area2().abs(), 250_000);
    assert!(
        app.status_message()
            .contains("Intersected 1 active-layer shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clip));
}

#[test]
pub(crate) fn layout_layer_and_accepts_selected_nonconvex_polygon_region() {
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
    let clip = app
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
        .expect("non-convex clip polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_and_selection"));
    assert!(
        !app.workspace.document.shapes.contains_key(&source),
        "layer AND should replace original active-layer rectangle"
    );
    assert!(app.workspace.document.shapes.contains_key(&clip));
    let active_shapes = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| shape.layer == metal1)
        .collect::<Vec<_>>();
    assert!(
        active_shapes.len() > 1,
        "non-convex clipping should decompose into multiple convex fragments"
    );
    let area2 = active_shapes
        .iter()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 1_280_000);
    assert!(
        app.status_message()
            .contains("Intersected 1 active-layer shape")
    );
}

#[test]
pub(crate) fn layout_layer_rectangle_or_writes_union_fragments_with_selection() {
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

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_or_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
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
            "missing layer OR fragment {expected:?}"
        );
    }
    let area2 = active_rects
        .iter()
        .map(|rect| i128::from(rect.width()) * i128::from(rect.height()) * 2)
        .sum::<i128>();
    assert_eq!(area2, 2_300_000);
    assert!(app.status_message().contains("ORed 1 active-layer shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.edit"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("Edit menu should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.edit.layer_or_selection"),
        "Edit menu should expose Layer OR Selection"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&selection));
}

#[test]
pub(crate) fn layout_layer_or_accepts_selected_nonconvex_polygon_region() {
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

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_or_selection"));
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
    assert_eq!(area2, 2_000_000);
    let union_bounds = active_shapes
        .iter()
        .map(|shape| shape.kind.bounds())
        .reduce(|left, right| left.union(right))
        .expect("layer OR fragments should have bounds");
    assert_eq!(
        union_bounds,
        Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))
    );
    assert!(app.status_message().contains("ORed 1 active-layer shape"));
}

#[test]
pub(crate) fn layout_create_clip_cell_clips_active_layer_rectangles_into_new_cell() {
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
    let inside = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500))),
        )
        .expect("inside rectangle should be added");
    let partial = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(900, 100), Point::new(1_600, 600))),
        )
        .expect("partial rectangle should be added");
    let outside = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(3_000, 0), Point::new(3_500, 500))),
        )
        .expect("outside rectangle should be added");
    let clip = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::new(Point::new(500, 50), Point::new(1_200, 450))),
        )
        .expect("clip rectangle should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    let before_cells = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let shape_count = app.workspace.document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.create_clip_cell"));
    assert_eq!(
        app.workspace.document.shapes.len(),
        shape_count,
        "clip cell creation should preserve source top-level shapes"
    );
    for id in [inside, partial, outside, clip] {
        assert!(
            app.workspace.document.shapes.contains_key(&id),
            "source shape #{id:?} should remain in place"
        );
    }

    let clip_cell_id = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .find(|cell| !before_cells.contains(cell))
        .expect("clip cell should be added");
    let clip_cell = app
        .workspace
        .document
        .cell(clip_cell_id)
        .expect("new clip cell should exist");
    assert_eq!(clip_cell.name, format!("clip {}", clip_cell_id.0));
    let local_rects = clip_cell
        .shapes
        .values()
        .map(|shape| shape.kind.bounds())
        .collect::<Vec<_>>();
    assert_eq!(local_rects.len(), 2);
    assert!(local_rects.contains(&Rect::new(Point::new(0, 0), Point::new(500, 400))));
    assert!(local_rects.contains(&Rect::new(Point::new(400, 50), Point::new(700, 400))));
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("top cell should exist");
    let instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == clip_cell_id)
        .expect("clip cell should be instanced under the top cell");
    assert_eq!(instance.transform, Transform::translate(500, 50));
    assert!(
        app.status_message().contains("Created clip"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.cell(clip_cell_id).is_none(),
        "undo should remove the clip cell"
    );
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("top cell should still exist");
    assert!(
        top_cell
            .instances
            .values()
            .all(|instance| instance.cell != clip_cell_id),
        "undo should remove the clip cell instance"
    );
    assert_eq!(app.workspace.document.shapes.len(), shape_count);
}

#[test]
pub(crate) fn layout_create_clip_cell_accepts_selected_convex_polygon_region() {
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
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(1_000, 0),
                Point::new(1_000, 1_000),
                Point::new(0, 1_000),
            ])),
        )
        .expect("source polygon should be added");
    let clip = app
        .add_layout_shape(
            metal2,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(250, 250),
                Point::new(750, 250),
                Point::new(500, 750),
            ])),
        )
        .expect("clip polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    let before_cells = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let shape_count = app.workspace.document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.create_clip_cell"));
    assert_eq!(
        app.workspace.document.shapes.len(),
        shape_count,
        "clip cell creation should preserve source top-level shapes"
    );
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clip));

    let clip_cell_id = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .find(|cell| !before_cells.contains(cell))
        .expect("clip cell should be added");
    let clip_cell = app
        .workspace
        .document
        .cell(clip_cell_id)
        .expect("new clip cell should exist");
    let local_shape = clip_cell
        .shapes
        .values()
        .next()
        .expect("clip cell should contain clipped geometry");
    let ShapeKind::Polygon(poly) = &local_shape.kind else {
        panic!("convex-polygon clip cell should contain polygon geometry");
    };
    assert_eq!(
        poly.bounds(),
        Some(Rect::new(Point::new(0, 0), Point::new(500, 500)))
    );
    assert_eq!(poly.signed_area2().abs(), 250_000);
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("top cell should exist");
    let instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == clip_cell_id)
        .expect("clip cell should be instanced under the top cell");
    assert_eq!(instance.transform, Transform::translate(250, 250));
    assert!(
        app.status_message().contains("Created clip"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_create_clip_cell_accepts_selected_nonconvex_polygon_region() {
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
    let clip = app
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
        .expect("non-convex clip polygon should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(clip);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(clip));

    let before_cells = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.create_clip_cell"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&clip));

    let clip_cell_id = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .find(|cell| !before_cells.contains(cell))
        .expect("clip cell should be added");
    let clip_cell = app
        .workspace
        .document
        .cell(clip_cell_id)
        .expect("new clip cell should exist");
    assert!(
        clip_cell.shapes.len() > 1,
        "non-convex clip cell should store decomposed clipped geometry"
    );
    let area2 = clip_cell
        .shapes
        .values()
        .map(|shape| shape_kind_area2_abs(&shape.kind))
        .sum::<i128>();
    assert_eq!(area2, 1_280_000);
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("top cell should exist");
    let instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == clip_cell_id)
        .expect("clip cell should be instanced under the top cell");
    assert_eq!(instance.transform, Transform::translate(0, 0));
}

#[test]
pub(crate) fn layout_layer_rectangle_not_subtracts_selected_rect_from_active_layer() {
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
            ShapeKind::Rectangle(Rect::new(Point::new(250, 250), Point::new(750, 750))),
        )
        .expect("cut rectangle should be added");
    app.active_layer = metal1;
    app.selected_layout_shape = Some(cut);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(cut));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.layer_not_selection"));
    assert!(!app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&cut));
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
            "missing fragment {expected:?}"
        );
    }
    assert!(
        app.status_message()
            .contains("Subtracted selection from 1 active-layer shape")
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.shapes.contains_key(&source));
    assert!(app.workspace.document.shapes.contains_key(&cut));
}
