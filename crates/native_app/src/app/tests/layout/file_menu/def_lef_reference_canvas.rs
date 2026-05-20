#![allow(unused_imports)]
use super::*;
use crate::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_exports_and_imports_def() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("def menu source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 600))),
    )
    .expect("test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Path {
            points: vec![Point::new(0, 1_000), Point::new(1_000, 1_000)],
            width: 120,
        },
    )
    .expect("test path should be added");
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-menu-export-{}.def",
        std::process::id()
    ));
    let gz_path = path.with_extension("def.gz");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.export_layout_def_to_path(&path));
    assert!(
        app.status_message().contains("Exported DEF"),
        "{}",
        app.status_message()
    );
    let exported = std::fs::read_to_string(&path).expect("DEF export should be readable");
    assert!(exported.contains("FILLS"));
    assert!(exported.contains("SPECIALNETS"));
    write_gzip_test_file(&gz_path, exported.as_bytes());

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("empty before DEF import");
    imported.reset_layout_document_state();
    assert!(imported.import_layout_def_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported.status_message().contains("Imported DEF"),
        "{}",
        imported.status_message()
    );
    assert!(
        imported.workspace.document.flattened_shape_count_estimate() >= 2,
        "DEF import should load layout geometry"
    );
    let expected_def_shapes = imported.workspace.document.flattened_shape_count_estimate();
    let expected_def_cells = imported.workspace.document.cells.len();
    let mut imported_gz = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_gz.workspace.document = Document::new("empty before compressed DEF import");
    imported_gz.reset_layout_document_state();
    assert!(imported_gz.import_layout_def_from_path(&gz_path));
    assert_eq!(imported_gz.active_view, StartupView::Layout2d);
    assert!(
        imported_gz.status_message().contains("Imported DEF"),
        "{}",
        imported_gz.status_message()
    );

    let mut imported_cell = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_cell.workspace.document = Document::new("target before DEF cell import");
    imported_cell.reset_layout_document_state();
    let import_top = imported_cell.workspace.document.top_cell;
    let import_target_layer = imported_cell
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    let target_shape = imported_cell
        .add_layout_shape(
            import_target_layer,
            ShapeKind::Rectangle(Rect::new(Point::new(2_000, 0), Point::new(2_400, 400))),
        )
        .expect("target rectangle should be added");
    let target_top_shapes_before = imported_cell.workspace.document.shapes.len();
    let target_cells_before = imported_cell.workspace.document.cells.len();
    let target_instances_before = imported_cell
        .workspace
        .document
        .cell(import_top)
        .expect("target top cell should exist")
        .instances
        .len();
    assert!(imported_cell.import_layout_def_as_cell_from_path(&path));
    assert_eq!(imported_cell.active_view, StartupView::Layout2d);
    assert!(
        imported_cell
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "DEF import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported_cell.workspace.document.shapes.len(),
        target_top_shapes_before,
        "DEF imported geometry should live inside the imported cell"
    );
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before + expected_def_cells
    );
    assert_eq!(
        imported_cell
            .workspace
            .document
            .cell(import_top)
            .expect("target top cell should exist")
            .instances
            .len(),
        target_instances_before + 1
    );
    assert!(
        imported_cell.status_message().contains("as cell"),
        "{}",
        imported_cell.status_message()
    );
    assert!(imported_cell.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before
    );

    let mut top_imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    top_imported.workspace.document = Document::new("target before DEF top-cell import");
    top_imported.reset_layout_document_state();
    let original_top = top_imported.workspace.document.top_cell;
    let top_cells_before = top_imported.workspace.document.cells.len();
    let top_instances_before = top_imported
        .workspace
        .document
        .cell(original_top)
        .expect("original top cell should exist")
        .instances
        .len();
    assert!(top_imported.import_layout_def_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_def_cells
    );
    assert_eq!(
        top_imported
            .workspace
            .document
            .cell(original_top)
            .expect("original top cell should still exist")
            .instances
            .len(),
        top_instances_before
    );
    assert!(
        top_imported.status_message().contains("extra top cell"),
        "{}",
        top_imported.status_message()
    );
    assert!(top_imported.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before
    );
    assert_eq!(top_imported.layout_view_top_cell, original_top);

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before DEF merge");
    merged.reset_layout_document_state();
    let merge_target_layer = merged
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    let merge_target_shape = merged
        .add_layout_shape(
            merge_target_layer,
            ShapeKind::Rectangle(Rect::new(Point::new(4_000, 0), Point::new(4_400, 400))),
        )
        .expect("merge target rectangle should be added");
    let merge_top_shapes_before = merged.workspace.document.shapes.len();
    let merge_cells_before = merged.workspace.document.cells.len();
    assert!(merged.merge_layout_def_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "DEF merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_def_shapes,
        "DEF merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert!(
        merged.status_message().contains("Merged DEF"),
        "{}",
        merged.status_message()
    );
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.export_def",
        "glassworks.menu.item.file.import_def",
        "glassworks.menu.item.file.import_def_cell",
        "glassworks.menu.item.file.import_def_top_cell",
        "glassworks.menu.item.file.merge_def",
        "glassworks.menu.item.file.merge_def_hierarchy",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_exports_and_imports_lef() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("lef menu source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let annotation = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Annotation)
        .expect("default technology should include annotation");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 600))),
    )
    .expect("test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Path {
            points: vec![Point::new(0, 1_000), Point::new(1_000, 1_000)],
            width: 120,
        },
    )
    .expect("test path should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(0, 2_000),
            text: "PAD_A".to_string(),
        },
    )
    .expect("test pin label should be added");
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-menu-export-{}.lef",
        std::process::id()
    ));
    let gz_path = path.with_extension("lef.gz");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.export_layout_lef_to_path(&path));
    assert!(
        app.status_message().contains("Exported LEF"),
        "{}",
        app.status_message()
    );
    let exported = std::fs::read_to_string(&path).expect("LEF export should be readable");
    assert!(exported.contains("MACRO lef_menu_source"));
    assert!(exported.contains("OBS"));
    assert!(exported.contains("PIN PAD_A"));
    write_gzip_test_file(&gz_path, exported.as_bytes());

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("empty before LEF import");
    imported.reset_layout_document_state();
    assert!(imported.import_layout_lef_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported.status_message().contains("Imported LEF"),
        "{}",
        imported.status_message()
    );
    assert!(
        imported.workspace.document.flattened_shape_count_estimate() >= 3,
        "LEF import should load layout geometry and pins"
    );
    let expected_lef_shapes = imported.workspace.document.flattened_shape_count_estimate();
    let expected_lef_cells = imported.workspace.document.cells.len();
    let mut imported_gz = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_gz.workspace.document = Document::new("empty before compressed LEF import");
    imported_gz.reset_layout_document_state();
    assert!(imported_gz.import_layout_lef_from_path(&gz_path));
    assert_eq!(imported_gz.active_view, StartupView::Layout2d);
    assert!(
        imported_gz.status_message().contains("Imported LEF"),
        "{}",
        imported_gz.status_message()
    );

    let mut imported_cell = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_cell.workspace.document = Document::new("target before LEF cell import");
    imported_cell.reset_layout_document_state();
    let import_top = imported_cell.workspace.document.top_cell;
    let import_target_layer = imported_cell
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    let target_shape = imported_cell
        .add_layout_shape(
            import_target_layer,
            ShapeKind::Rectangle(Rect::new(Point::new(2_000, 0), Point::new(2_400, 400))),
        )
        .expect("target rectangle should be added");
    let target_top_shapes_before = imported_cell.workspace.document.shapes.len();
    let target_cells_before = imported_cell.workspace.document.cells.len();
    let target_instances_before = imported_cell
        .workspace
        .document
        .cell(import_top)
        .expect("target top cell should exist")
        .instances
        .len();
    assert!(imported_cell.import_layout_lef_as_cell_from_path(&path));
    assert_eq!(imported_cell.active_view, StartupView::Layout2d);
    assert!(
        imported_cell
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "LEF import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported_cell.workspace.document.shapes.len(),
        target_top_shapes_before,
        "LEF imported geometry should live inside the imported cell"
    );
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before + expected_lef_cells
    );
    assert_eq!(
        imported_cell
            .workspace
            .document
            .cell(import_top)
            .expect("target top cell should exist")
            .instances
            .len(),
        target_instances_before + 1
    );
    assert!(
        imported_cell.status_message().contains("as cell"),
        "{}",
        imported_cell.status_message()
    );
    assert!(imported_cell.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before
    );

    let mut top_imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    top_imported.workspace.document = Document::new("target before LEF top-cell import");
    top_imported.reset_layout_document_state();
    let original_top = top_imported.workspace.document.top_cell;
    let top_cells_before = top_imported.workspace.document.cells.len();
    let top_instances_before = top_imported
        .workspace
        .document
        .cell(original_top)
        .expect("original top cell should exist")
        .instances
        .len();
    assert!(top_imported.import_layout_lef_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_lef_cells
    );
    assert_eq!(
        top_imported
            .workspace
            .document
            .cell(original_top)
            .expect("original top cell should still exist")
            .instances
            .len(),
        top_instances_before
    );
    assert!(
        top_imported.status_message().contains("extra top cell"),
        "{}",
        top_imported.status_message()
    );
    assert!(top_imported.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before
    );
    assert_eq!(top_imported.layout_view_top_cell, original_top);

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before LEF merge");
    merged.reset_layout_document_state();
    let merge_target_layer = merged
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    let merge_target_shape = merged
        .add_layout_shape(
            merge_target_layer,
            ShapeKind::Rectangle(Rect::new(Point::new(4_000, 0), Point::new(4_400, 400))),
        )
        .expect("merge target rectangle should be added");
    let merge_top_shapes_before = merged.workspace.document.shapes.len();
    let merge_cells_before = merged.workspace.document.cells.len();
    assert!(merged.merge_layout_lef_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "LEF merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_lef_shapes,
        "LEF merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert!(
        merged.status_message().contains("Merged LEF"),
        "{}",
        merged.status_message()
    );
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.export_lef",
        "glassworks.menu.item.file.import_lef",
        "glassworks.menu.item.file.import_lef_cell",
        "glassworks.menu.item.file.import_lef_top_cell",
        "glassworks.menu.item.file.merge_lef",
        "glassworks.menu.item.file.merge_lef_hierarchy",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_reference_image_exchange_aligns_and_draws_overlay() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let mut image = ReferenceImageOverlay::new(
        "sem-a",
        "SEM A",
        "images/sem-a.png",
        Rect::from_min_size(Point::new(0, 0), 10, 10),
    );
    image.pixel_size = Some(ReferenceImageSize::new(100, 50));
    image.opacity = 128;
    image.landmarks = vec![
        ReferenceImageLandmark::new("left", Point::new(10, 10), Point::new(-500, 500)),
        ReferenceImageLandmark::new("right", Point::new(60, 30), Point::new(0, 300)),
    ];
    app.workspace.document.reference_images.push(image);

    let path = std::env::temp_dir().join(format!(
        "glassworks-reference-images-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_layout_reference_images_to_path(&path));
    let exported =
        std::fs::read_to_string(&path).expect("reference image export should be readable");
    let exchange: LayoutReferenceImageExchange =
        serde_json::from_str(&exported).expect("reference image exchange should parse");
    assert_eq!(exchange.reference_images.len(), 1);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    assert!(imported.import_layout_reference_images_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(imported.show_reference_images);
    assert_eq!(
        imported.workspace.document.reference_images[0].bounds,
        Rect::new(Point::new(-600, 100), Point::new(400, 600))
    );
    assert_eq!(
        imported.app_options.files.recent_files[0].kind,
        "reference_images"
    );

    let overlay =
        layout_overlay_primitives(&imported, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, ScenePrimitive::Image { key, .. } if key.contains("sem-a.png"))),
            "reference image overlay should draw an image primitive"
        );
    assert!(
            overlay
                .iter()
                .filter(|primitive| matches!(primitive, ScenePrimitive::Line { stroke, .. } if stroke.color == ColorRgba::new(255, 216, 116, 230)))
                .count()
                >= 4,
            "reference image landmarks should draw crosshair lines"
        );
    assert!(imported.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.toggle"));
    assert!(!imported.show_reference_images);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.export_reference_images",
        "glassworks.menu.item.file.import_reference_images",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "File menu should expose {node_name}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
pub(crate) fn layout_reference_image_landmark_controls_seed_fit_and_clear() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document.reference_images.clear();
    let mut image = ReferenceImageOverlay::new(
        "sem-fit",
        "SEM fit",
        "images/sem-fit.png",
        Rect::from_min_size(Point::new(0, 0), 100, 50),
    );
    image.pixel_size = Some(ReferenceImageSize::new(100, 50));
    app.workspace.document.reference_images.push(image);
    let mut image = ReferenceImageOverlay::new(
        "sem-b",
        "SEM B",
        "images/sem-b.png",
        Rect::from_min_size(Point::new(20, 10), 80, 40),
    );
    image.pixel_size = Some(ReferenceImageSize::new(80, 40));
    app.workspace.document.reference_images.push(image);
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let target_bounds = Rect::new(Point::new(200, 300), Point::new(400, 500));
    let target = app
        .add_layout_shape(metal1, ShapeKind::Rectangle(target_bounds))
        .expect("target shape should be inserted");
    app.selected_layout_shape = Some(target);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(target));

    let control_names = layout_editor_control_rows(&app)
        .into_iter()
        .flatten()
        .map(|button| button.name)
        .collect::<Vec<_>>();
    for node_name in [
        "glassworks.viewctl.layout.reference_images.landmarks.fit_selection",
        "glassworks.viewctl.layout.reference_images.landmarks.seed",
        "glassworks.viewctl.layout.reference_images.landmarks.clear",
        "glassworks.viewctl.layout.reference_image.toggle.0",
        "glassworks.viewctl.layout.reference_image.landmarks.fit_selection.0",
        "glassworks.viewctl.layout.reference_image.landmarks.seed.0",
        "glassworks.viewctl.layout.reference_image.landmarks.align.0",
        "glassworks.viewctl.layout.reference_image.landmarks.clear.0",
        "glassworks.viewctl.layout.reference_image.opacity_less.0",
        "glassworks.viewctl.layout.reference_image.opacity_more.0",
        "glassworks.viewctl.layout.reference_image.focus.0",
        "glassworks.viewctl.layout.reference_image.remove.0",
        "glassworks.viewctl.layout.reference_image.toggle.1",
        "glassworks.viewctl.layout.reference_image.landmarks.fit_selection.1",
        "glassworks.viewctl.layout.reference_image.landmarks.seed.1",
        "glassworks.viewctl.layout.reference_image.landmarks.align.1",
        "glassworks.viewctl.layout.reference_image.landmarks.clear.1",
        "glassworks.viewctl.layout.reference_image.focus.1",
        "glassworks.viewctl.layout.reference_image.remove.1",
    ] {
        assert!(
            control_names.iter().any(|name| name == node_name),
            "layout view controls should expose {node_name}"
        );
    }

    assert!(
        layout_reference_image_rows(&app)
            .iter()
            .any(|(_, value)| value.contains("opacity 38%")),
        "reference image rows should expose per-image opacity"
    );
    let reference_rows = layout_reference_image_rows(&app);
    assert!(
        reference_rows
            .iter()
            .any(|(_, value)| value == "images/sem-fit.png"),
        "reference image rows should expose per-image source: {reference_rows:?}"
    );
    assert!(
        reference_rows.iter().any(|(_, value)| value == "100x50 px"),
        "reference image rows should expose per-image pixel size: {reference_rows:?}"
    );
    app.set_layout_browser_search("pixel_width=80");
    let reference_rows = layout_reference_image_rows(&app);
    assert!(
        reference_rows
            .iter()
            .any(|(key, value)| key == "Listed images" && value == "1 / 2 total"),
        "reference image rows should summarize filtered images: {reference_rows:?}"
    );
    assert!(
        reference_rows.iter().any(|(key, _)| key == "SEM B"),
        "reference image selector search should keep the matching image: {reference_rows:?}"
    );
    assert!(
        !reference_rows.iter().any(|(key, _)| key == "SEM fit"),
        "reference image selector search should hide non-matching images: {reference_rows:?}"
    );
    app.set_layout_browser_search("source=sem-fit.png");
    let reference_rows = layout_reference_image_rows(&app);
    assert!(
        reference_rows.iter().any(|(key, _)| key == "SEM fit"),
        "reference image selector search should match source paths: {reference_rows:?}"
    );
    app.set_layout_browser_search("");
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.opacity_more.0")
    );
    assert_eq!(app.workspace.document.reference_images[0].opacity, 128);
    assert!(app.status_message().contains("opacity 50%"));
    assert!(
        layout_reference_image_rows(&app)
            .iter()
            .any(|(_, value)| value.contains("opacity 50%")),
        "reference image rows should update per-image opacity"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.opacity_less.0")
    );
    assert_eq!(app.workspace.document.reference_images[0].opacity, 96);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.toggle.0"));
    assert!(!app.workspace.document.reference_images[0].visible);
    assert!(
        layout_reference_image_rows(&app)
            .iter()
            .any(|(_, value)| value.contains("hidden")),
        "reference image rows should expose per-image visibility"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.landmarks.seed.1")
    );
    assert!(
        app.workspace.document.reference_images[0]
            .landmarks
            .is_empty()
    );
    assert_eq!(
        app.workspace.document.reference_images[1].landmarks.len(),
        2
    );
    assert!(app.status_message().contains("SEM B"));
    app.workspace.document.reference_images[1].landmarks = vec![
        ReferenceImageLandmark::new("lower_left", Point::new(0, 40), target_bounds.min),
        ReferenceImageLandmark::new("upper_right", Point::new(80, 0), target_bounds.max),
    ];
    assert_ne!(
        app.workspace.document.reference_images[1].bounds,
        target_bounds
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.landmarks.align.1")
    );
    assert_eq!(
        app.workspace.document.reference_images[1].bounds,
        target_bounds
    );
    assert!(app.workspace.document.reference_images[1].visible);
    assert!(app.show_reference_images);
    assert!(
        app.status_message()
            .contains("Aligned reference image SEM B")
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.landmarks.clear.1")
    );
    assert!(
        app.workspace.document.reference_images[1]
            .landmarks
            .is_empty()
    );
    assert!(app.status_message().contains("SEM B"));

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.landmarks.seed")
    );
    assert_eq!(
        app.workspace.document.reference_images[0].landmarks.len(),
        2
    );
    assert_eq!(
        app.workspace.document.reference_images[1].landmarks.len(),
        2
    );
    assert!(
        layout_reference_image_rows(&app)
            .iter()
            .any(|(key, _)| key.contains("lower_left")),
        "reference image rows should expose seeded landmark details"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.landmarks.clear")
    );
    assert!(
        app.workspace.document.reference_images[0]
            .landmarks
            .is_empty()
    );
    assert!(
        app.workspace.document.reference_images[1]
            .landmarks
            .is_empty()
    );

    let first_bounds = app.workspace.document.reference_images[0].bounds;
    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.reference_image.landmarks.fit_selection.1"
    ));
    assert_eq!(
        app.workspace.document.reference_images[0].bounds,
        first_bounds
    );
    let image = &app.workspace.document.reference_images[1];
    assert_eq!(image.bounds, target_bounds);
    assert!(image.visible);
    assert_eq!(image.landmarks.len(), 2);
    assert!(app.show_reference_images);
    assert!(app.status_message().contains("SEM B"));

    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.reference_images.landmarks.fit_selection"
    ));
    let image = &app.workspace.document.reference_images[0];
    assert_eq!(image.bounds, target_bounds);
    assert!(image.visible);
    assert_eq!(image.landmarks.len(), 2);
    assert!(app.show_reference_images);
    assert!(app.status_message().contains("Fit reference image"));

    app.layout_canvas_size = Some(UiSize::new(800.0, 600.0));
    app.layout_zoom = 0.25;
    app.layout_pan = [42.0, -13.0];
    let previous_view = app.current_layout_view_state();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.focus.0"));
    let expected_zoom = (800.0_f32 / (target_bounds.width() as f32 * 1.12))
        .min(600.0_f32 / (target_bounds.height() as f32 * 1.12));
    let center = target_bounds.center();
    assert!((app.layout_zoom - expected_zoom).abs() < 0.0001);
    assert_eq!(
        app.layout_pan,
        [
            -(center.x as f32) * expected_zoom,
            center.y as f32 * expected_zoom,
        ]
    );
    assert_eq!(app.layout_previous_view, Some(previous_view));
    assert!(app.status_message().contains("Focused reference image"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.remove.0"));
    assert_eq!(app.workspace.document.reference_images.len(), 1);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.remove.0"));
    assert!(app.workspace.document.reference_images.is_empty());
    assert!(app.status_message().contains("Removed reference image"));
}

#[test]
pub(crate) fn layout_reference_image_order_controls_reorder_overlays() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document.reference_images.clear();
    app.workspace
        .document
        .reference_images
        .push(ReferenceImageOverlay::new(
            "sem-a",
            "SEM A",
            "images/sem-a.png",
            Rect::from_min_size(Point::new(0, 0), 100, 50),
        ));
    app.workspace
        .document
        .reference_images
        .push(ReferenceImageOverlay::new(
            "sem-b",
            "SEM B",
            "images/sem-b.png",
            Rect::from_min_size(Point::new(0, 0), 100, 50),
        ));

    let control_names = layout_editor_control_rows(&app)
        .into_iter()
        .flatten()
        .map(|button| button.name)
        .collect::<Vec<_>>();
    for node_name in [
        "glassworks.viewctl.layout.reference_image.move_up.0",
        "glassworks.viewctl.layout.reference_image.move_down.0",
        "glassworks.viewctl.layout.reference_image.move_up.1",
        "glassworks.viewctl.layout.reference_image.move_down.1",
        "glassworks.viewctl.layout.reference_images.show_all",
        "glassworks.viewctl.layout.reference_images.hide_all",
        "glassworks.viewctl.layout.reference_images.clear",
    ] {
        assert!(
            control_names.iter().any(|name| name == node_name),
            "layout view controls should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.move_up.0"));
    assert_eq!(app.workspace.document.reference_images[0].id, "sem-a");
    assert!(app.status_message().contains("already at edge"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.move_down.0"));
    assert_eq!(app.workspace.document.reference_images[0].id, "sem-b");
    assert_eq!(app.workspace.document.reference_images[1].id, "sem-a");
    assert!(app.status_message().contains("slot 2"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_image.move_up.1"));
    assert_eq!(app.workspace.document.reference_images[0].id, "sem-a");
    assert_eq!(app.workspace.document.reference_images[1].id, "sem-b");
    assert!(app.status_message().contains("slot 1"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.hide_all"));
    assert!(!app.show_reference_images);
    assert!(
        app.workspace
            .document
            .reference_images
            .iter()
            .all(|image| !image.visible)
    );
    assert!(app.status_message().contains("Hid 2 of 2"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.show_all"));
    assert!(app.show_reference_images);
    assert!(
        app.workspace
            .document
            .reference_images
            .iter()
            .all(|image| image.visible)
    );
    assert!(app.status_message().contains("Showed 2 of 2"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.clear"));
    assert!(app.workspace.document.reference_images.is_empty());
    assert!(app.status_message().contains("Cleared 2 reference images"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.reference_images.clear"));
    assert!(app.status_message().contains("No reference images"));
}

#[test]
pub(crate) fn layout_canvas_uses_editor_zoom_and_pan() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        pan: Some([25.0, -10.0]),
        ..Default::default()
    });
    let size = UiSize::new(1000.0, 500.0);

    assert_eq!(app.layout_zoom(), 0.1);
    assert_eq!(app.layout_pan(), [25.0, -10.0]);
    assert_eq!(
        app.layout_world_to_canvas(Point::ZERO, size),
        UiPoint::new(525.0, 240.0)
    );
    let viewport = app.layout_viewport_for_size(size);
    assert_eq!(viewport.width(), 10_000);
    assert_eq!(viewport.height(), 5_000);
    assert_eq!(
        app.layout_canvas_to_world_local(app.layout_world_to_canvas(Point::ZERO, size), size),
        Point::ZERO
    );
}

#[test]
pub(crate) fn layout_overlay_grid_tracks_zoom_and_pan() {
    fn vertical_grid_lines(primitives: &[ScenePrimitive]) -> Vec<i32> {
        primitives
            .iter()
            .filter_map(|primitive| match primitive {
                ScenePrimitive::Line { from, to, .. } if (from.x - to.x).abs() < 0.5 => {
                    Some(from.x.round() as i32)
                }
                _ => None,
            })
            .collect()
    }

    let size = UiSize::new(900.0, 700.0);
    let mut far = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    far.workspace.document.grid = 100;
    let mut near = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(1.0),
        ..Default::default()
    });
    near.workspace.document.grid = 100;

    let far_lines = vertical_grid_lines(&layout_overlay_primitives(&far, size, UiScale::new(1.0)));
    let near_lines =
        vertical_grid_lines(&layout_overlay_primitives(&near, size, UiScale::new(1.0)));
    assert!(
        far_lines.len() > near_lines.len(),
        "zoomed-out view should draw more world-grid lines: far={far_lines:?} near={near_lines:?}"
    );

    far.layout_pan = [20.0, 0.0];
    let panned_lines =
        vertical_grid_lines(&layout_overlay_primitives(&far, size, UiScale::new(1.0)));
    assert_ne!(
        far_lines, panned_lines,
        "panning should move the world-aligned grid instead of leaving a fixed screen grid"
    );
}

#[test]
pub(crate) fn layout_scale_bar_picks_readable_world_lengths() {
    assert_eq!(nice_layout_scale_length_dbu(80.0), 100);
    assert_eq!(nice_layout_scale_length_dbu(1_600.0), 2_000);
    assert_eq!(nice_layout_scale_length_dbu(52_000.0), 50_000);
    assert_eq!(layout_scale_bar_length_dbu(1.0), 100);
}

#[test]
pub(crate) fn layout_canvas_rect_tool_creates_rectangle() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let rect = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let shape_count = app.workspace().document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.tool.rect"));

    let start = app.layout_world_to_canvas(Point::new(0, 0), rect_size(rect));
    let end = app.layout_world_to_canvas(Point::new(1_000, 700), rect_size(rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(end), rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(end), rect));

    assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
    let shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("rectangle tool should select the shape it created");
    assert_eq!(
        shape.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 700)))
    );
}

#[test]
pub(crate) fn layout_canvas_zoom_and_pan_controls_update_viewport() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let rect = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let center = UiPoint::new(400.0, 300.0);

    assert!(app.handle_layout_canvas_input(
        &operad::UiInputEvent::wheel(center, UiPoint::new(0.0, -120.0)),
        rect
    ));
    assert!(
        app.layout_zoom() > 0.1,
        "wheel-up over the canvas should zoom in"
    );

    let origin_before = app.layout_world_to_canvas(Point::ZERO, rect_size(rect));
    assert!(app.pan_layout_canvas_by(UiPoint::new(40.0, -20.0)));
    let origin_after = app.layout_world_to_canvas(Point::ZERO, rect_size(rect));
    assert_eq!(
        origin_after,
        UiPoint::new(origin_before.x + 40.0, origin_before.y - 20.0)
    );
}

#[test]
pub(crate) fn layout_view_bookmark_and_previous_restore_view_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        hierarchy_demo: true,
        zoom: Some(0.05),
        pan: Some([12.0, -8.0]),
        ..Default::default()
    });
    let top_cell = app.workspace.document.top_cell;
    let child_cell = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .find(|cell| *cell != top_cell)
        .expect("hierarchy demo should have a child cell");
    app.layout_hierarchy_depth = LayoutHierarchyDepth::Numeric(4);
    app.layout_hierarchy_min_depth = 1;

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.save_layout_view.2"));
    assert_eq!(
        app.layout_view_bookmarks.get(&2).copied(),
        Some(app.current_layout_view_state())
    );

    let saved_options = app.app_options().clone();
    assert!(
        saved_options.layout.view_bookmarks[0]
            .name
            .contains("0.050x"),
        "saved bookmark should carry an auto-generated view name"
    );
    assert_eq!(
        saved_options.layout.view_bookmarks[0].hierarchy_depth,
        "depth_4"
    );
    assert_eq!(
        saved_options.layout.view_bookmarks[0].hierarchy_min_depth,
        1
    );

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::env::temp_dir().join(format!(
            "glassworks-view-bookmark-export-import-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        assert!(app.export_layout_view_bookmarks_to_path(&path));
        assert!(
            app.status_message().contains("Exported view bookmarks"),
            "{}",
            app.status_message()
        );
        let exchange = serde_json::from_str::<LayoutViewBookmarkExchange>(
            &std::fs::read_to_string(&path).expect("view bookmark exchange should be readable"),
        )
        .expect("view bookmark exchange should parse");
        assert_eq!(
            exchange.schema_version,
            LAYOUT_VIEW_BOOKMARK_EXCHANGE_SCHEMA_VERSION
        );
        assert_eq!(exchange.bookmarks.len(), 1);
        assert_eq!(exchange.bookmarks[0].slot, 2);
        assert_eq!(
            exchange.bookmarks[0].name,
            saved_options.layout.view_bookmarks[0].name
        );

        app.layout_view_bookmarks.clear();
        app.layout_view_bookmark_names.clear();
        app.sync_app_options_from_state();
        assert!(app.layout_view_bookmarks.is_empty());
        assert!(app.import_layout_view_bookmarks_from_path(&path));
        assert!(
            app.status_message().contains("Imported view bookmarks"),
            "{}",
            app.status_message()
        );
        assert_eq!(
            app.layout_view_bookmarks.get(&2).copied(),
            Some(app.current_layout_view_state())
        );
        assert_eq!(
            app.layout_view_bookmark_name(2),
            saved_options.layout.view_bookmarks[0].name
        );
        let _ = std::fs::remove_file(&path);
    }

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        hierarchy_demo: true,
        app_options: Some(saved_options),
        ..Default::default()
    });
    assert_eq!(app.layout_view_bookmarks.len(), 1);
    assert!(
        app.layout_view_bookmark_name(2).contains("0.050x"),
        "bookmark name should survive app-options restore"
    );
    assert_eq!(
        app.layout_view_bookmarks.get(&2).copied().unwrap().top_cell,
        top_cell
    );

    app.layout_zoom = 0.2;
    app.layout_pan = [91.0, 37.0];
    app.layout_view_top_cell = child_cell;
    app.layout_hierarchy_depth = LayoutHierarchyDepth::Boxes;
    app.layout_hierarchy_min_depth = 0;
    let changed_view = app.current_layout_view_state();

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.restore_layout_view.2"));
    assert_eq!(app.layout_zoom(), 0.05);
    assert_eq!(app.layout_pan(), [12.0, -8.0]);
    assert_eq!(app.layout_view_top_cell, top_cell);
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Numeric(4));
    assert_eq!(app.layout_hierarchy_min_depth, 1);
    assert_eq!(app.layout_previous_view, Some(changed_view));
    let bookmark_section = layout_editor_inspector_sections(&app)
        .into_iter()
        .find(|section| section.title == "View Bookmarks")
        .expect("layout inspector should expose view bookmark summaries");
    let bookmark_rows = bookmark_section
        .rows
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        bookmark_rows.get("Saved").map(String::as_str),
        Some("1 / 8")
    );
    let saved_view = bookmark_rows
        .get("View 2")
        .expect("saved view bookmark row should exist");
    assert!(
        saved_view.contains("0.050x")
            && saved_view.contains("pan 12,-8")
            && saved_view.contains("4 hierarchy levels")
            && saved_view.contains("active"),
        "saved bookmark row should summarize the active saved view: {saved_view}"
    );
    assert_eq!(
        bookmark_rows.get("View 8").map(String::as_str),
        Some("empty")
    );
    let previous_view = bookmark_rows
        .get("Previous")
        .expect("previous view row should exist");
    assert!(
        previous_view.contains("0.200x") && previous_view.contains("child cell boxes"),
        "previous view row should summarize the last view: {previous_view}"
    );

    app.set_layout_browser_search("slot=2");
    let bookmark_section = layout_editor_inspector_sections(&app)
        .into_iter()
        .find(|section| section.title == "View Bookmarks")
        .expect("layout inspector should expose searched bookmark summaries");
    let bookmark_rows = bookmark_section
        .rows
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        bookmark_rows.get("Listed views").map(String::as_str),
        Some("1 / 10 views")
    );
    assert!(
        bookmark_rows.contains_key("View 2"),
        "slot selector search should keep the matching bookmark row: {bookmark_rows:?}"
    );
    assert!(
        !bookmark_rows.contains_key("View 8"),
        "slot selector search should hide non-matching bookmark slots: {bookmark_rows:?}"
    );
    assert!(
        !bookmark_rows.contains_key("Current"),
        "slot selector search should hide non-matching current view: {bookmark_rows:?}"
    );

    app.set_layout_browser_search("state=previous");
    let bookmark_section = layout_editor_inspector_sections(&app)
        .into_iter()
        .find(|section| section.title == "View Bookmarks")
        .expect("layout inspector should expose searched previous-view summaries");
    let bookmark_rows = bookmark_section
        .rows
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        bookmark_rows.get("Listed views").map(String::as_str),
        Some("1 / 10 views")
    );
    assert!(
        bookmark_rows.contains_key("Previous"),
        "state selector search should keep the previous-view row: {bookmark_rows:?}"
    );
    assert!(
        !bookmark_rows.contains_key("View 2"),
        "state selector search should hide saved bookmark slots: {bookmark_rows:?}"
    );
    app.set_layout_browser_search("");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.previous_layout_view"));
    assert_eq!(app.current_layout_view_state(), changed_view);

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.clear_layout_view.2"));
    assert!(!app.layout_view_bookmarks.contains_key(&2));
    assert_eq!(app.layout_view_bookmark_name(2), "View 2");

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.save_layout_view.2"));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.save_layout_view.4"));
    assert_eq!(app.layout_view_bookmarks.len(), 2);
    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.clear_layout_views"));
    assert!(app.layout_view_bookmarks.is_empty());
    assert_eq!(app.layout_view_bookmark_name(2), "View 2");
    assert_eq!(app.layout_view_bookmark_name(4), "View 4");
    assert!(
        app.app_options().layout.view_bookmarks.is_empty(),
        "clearing all bookmarks should persist to app options"
    );
    assert!(
        app.status_message()
            .contains("Cleared 2 layout view bookmarks"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.clear_layout_views"));
    assert!(
        app.status_message()
            .contains("No layout view bookmarks saved"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_bookmark_menu_exposes_view_actions() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(app.apply_clicked_node_name("glassworks.menu.bookmarks"));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("bookmarks menu should build");
    for node_name in [
        "glassworks.menu.item.bookmarks.save_layout_view.1",
        "glassworks.menu.item.bookmarks.save_layout_view.8",
        "glassworks.menu.item.bookmarks.origin",
        "glassworks.menu.item.bookmarks.bounds",
        "glassworks.menu.item.bookmarks.export_layout_views",
        "glassworks.menu.item.bookmarks.import_layout_views",
        "glassworks.menu.item.bookmarks.clear_layout_views",
        "glassworks.menu.item.bookmarks.clear_layout_view.8",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("{node_name} should be in Bookmarks menu"));
        assert!(
            node.input().pointer
                || node_name.ends_with("clear_layout_view.8")
                || node_name.ends_with("clear_layout_views")
                || node_name.ends_with("export_layout_views"),
            "{node_name} should be enabled in layout view unless the slot is empty"
        );
    }
    let selection_focus = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.selection")
        .expect("selection focus should be in Bookmarks menu");
    assert!(
        !selection_focus.input().pointer,
        "selection focus should be disabled without a selected occurrence"
    );

    let selection_bounds = Rect::from_min_size(Point::new(2_000, 3_000), 1_000, 2_000);
    let selected = app
        .add_layout_shape(app.active_layer(), ShapeKind::Rectangle(selection_bounds))
        .expect("selection focus test shape should be inserted");
    app.selected_layout_shape = Some(selected);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selected));
    app.layout_canvas_size = Some(UiSize::new(800.0, 600.0));
    app.layout_zoom = 0.2;
    app.layout_pan = [81.0, -12.0];
    let previous_view = app.current_layout_view_state();
    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.selection"));
    let expected_zoom = (800.0_f32 / (selection_bounds.width() as f32 * 1.12))
        .min(600.0_f32 / (selection_bounds.height() as f32 * 1.12));
    let center = selection_bounds.center();
    assert!((app.layout_zoom - expected_zoom).abs() < 0.0001);
    assert_eq!(
        app.layout_pan,
        [
            -(center.x as f32) * expected_zoom,
            center.y as f32 * expected_zoom,
        ]
    );
    assert_eq!(app.layout_previous_view, Some(previous_view));
    assert!(app.status_message().contains("Focused selected"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.bookmarks.save_layout_view.3"));
    assert!(app.apply_clicked_node_name("glassworks.menu.bookmarks"));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("bookmarks menu with saved view should build");
    let restore = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.restore_layout_view.3")
        .expect("restore action should be visible");
    assert!(restore.input().pointer);
    let clear = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.clear_layout_view.3")
        .expect("clear action should be visible");
    assert!(clear.input().pointer);
    let export = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.export_layout_views")
        .expect("export action should be visible");
    assert!(export.input().pointer);
    let clear_all = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.clear_layout_views")
        .expect("clear-all action should be visible");
    assert!(clear_all.input().pointer);
    let empty_restore = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.bookmarks.restore_layout_view.8")
        .expect("empty restore action should stay visible");
    assert!(!empty_restore.input().pointer);
    assert!(
        document_visible_text(&document).contains("Clear View"),
        "bookmarks menu should expose clear-management actions"
    );
}

#[test]
pub(crate) fn layout_canvas_select_tool_moves_top_level_shape() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let rect = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let shape_id = app
        .workspace()
        .document
        .shapes
        .keys()
        .next()
        .copied()
        .expect("demo document should have a shape");
    let original = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("shape should exist")
        .clone();
    let center = original.kind.bounds().center();
    app.layout_pan = [
        -(center.x as f32) * app.layout_zoom,
        center.y as f32 * app.layout_zoom,
    ];
    let revision_before_drag = app.layout_revision;

    let start = app.layout_world_to_canvas(center, rect_size(rect));
    let end_world = Point::new(center.x + 100, center.y + 50);
    let end = app.layout_world_to_canvas(end_world, rect_size(rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(end), rect));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(end), rect));

    assert_eq!(app.selected_layout_shape(), Some(shape_id));
    let moved = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("shape should still exist");
    assert_eq!(
        moved.kind.bounds(),
        original.kind.bounds().translated(Vector::new(100, 50))
    );
    assert!(
        app.layout_revision > revision_before_drag,
        "dragging a shape should invalidate cached 2D canvas batches"
    );
}

#[test]
pub(crate) fn layout_canvas_shape_drag_commits_one_undo_entry() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let original_bounds = Rect::new(Point::new(140_000, 100_000), Point::new(141_000, 101_000));
    let shape_id = app
        .add_layout_shape(app.active_layer(), ShapeKind::Rectangle(original_bounds))
        .expect("test shape should be added");
    let center = original_bounds.center();
    app.layout_pan = [
        -(center.x as f32) * app.layout_zoom,
        center.y as f32 * app.layout_zoom,
    ];
    let undo_len = app.layout_undo.len();

    let start = app.layout_world_to_canvas(center, rect_size(canvas));
    let mid =
        app.layout_world_to_canvas(Point::new(center.x + 60, center.y + 30), rect_size(canvas));
    let end =
        app.layout_world_to_canvas(Point::new(center.x + 100, center.y + 50), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(mid), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(end), canvas));
    assert_eq!(
        app.layout_undo.len(),
        undo_len,
        "live drag frames should not add per-frame undo entries"
    );
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(end), canvas));
    assert_eq!(app.layout_undo.len(), undo_len + 1);

    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("shape should still exist")
            .kind
            .bounds(),
        original_bounds.translated(Vector::new(100, 50))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("shape should still exist after undo")
            .kind
            .bounds(),
        original_bounds
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.redo"));
    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("shape should still exist after redo")
            .kind
            .bounds(),
        original_bounds.translated(Vector::new(100, 50))
    );
}

#[test]
pub(crate) fn layout_canvas_vertex_drag_commits_one_undo_entry() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let original_bounds = Rect::new(Point::new(160_000, 100_000), Point::new(161_000, 101_000));
    let shape_id = app
        .add_layout_shape(app.active_layer(), ShapeKind::Rectangle(original_bounds))
        .expect("test shape should be added");
    app.layout_pan = [
        -(original_bounds.max.x as f32) * app.layout_zoom,
        original_bounds.max.y as f32 * app.layout_zoom,
    ];
    let undo_len = app.layout_undo.len();

    let start = app.layout_world_to_canvas(original_bounds.max, rect_size(canvas));
    let mid = app.layout_world_to_canvas(Point::new(161_150, 101_100), rect_size(canvas));
    let end = app.layout_world_to_canvas(Point::new(161_300, 101_200), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(mid), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(end), canvas));
    assert_eq!(
        app.layout_undo.len(),
        undo_len,
        "live vertex drags should not add per-frame undo entries"
    );
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(end), canvas));
    assert_eq!(app.layout_undo.len(), undo_len + 1);

    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("shape should still exist")
            .kind
            .bounds(),
        Rect::new(original_bounds.min, Point::new(161_300, 101_200))
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("shape should still exist after undo")
            .kind
            .bounds(),
        original_bounds
    );
}

#[test]
pub(crate) fn layout_canvas_shift_constrains_shape_drag_to_axis() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let original_bounds = Rect::new(Point::new(100_000, 100_000), Point::new(101_000, 101_000));
    let shape_id = app
        .add_layout_shape(app.active_layer(), ShapeKind::Rectangle(original_bounds))
        .expect("test shape should be added");
    let center = original_bounds.center();
    app.layout_pan = [
        -(center.x as f32) * app.layout_zoom,
        center.y as f32 * app.layout_zoom,
    ];
    let shift = operad::KeyModifiers {
        shift: true,
        ..operad::KeyModifiers::NONE
    };

    let start = app.layout_world_to_canvas(center, rect_size(canvas));
    let end =
        app.layout_world_to_canvas(Point::new(center.x + 100, center.y + 50), rect_size(canvas));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerDown(start),
        canvas,
        shift
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerMove(end),
        canvas,
        shift
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerUp(end),
        canvas,
        shift
    ));

    let moved = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("shape should still exist");
    assert_eq!(
        moved.kind.bounds(),
        original_bounds.translated(Vector::new(100, 0))
    );
}

#[test]
pub(crate) fn layout_canvas_alt_drag_duplicates_top_level_shape() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let original_bounds = Rect::new(Point::new(120_000, 100_000), Point::new(121_000, 101_000));
    let shape_id = app
        .add_layout_shape(app.active_layer(), ShapeKind::Rectangle(original_bounds))
        .expect("test shape should be added");
    let shape_count = app.workspace().document.shapes.len();
    let center = original_bounds.center();
    app.layout_pan = [
        -(center.x as f32) * app.layout_zoom,
        center.y as f32 * app.layout_zoom,
    ];
    let alt = operad::KeyModifiers {
        alt: true,
        ..operad::KeyModifiers::NONE
    };

    let start = app.layout_world_to_canvas(center, rect_size(canvas));
    let end =
        app.layout_world_to_canvas(Point::new(center.x + 100, center.y + 50), rect_size(canvas));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerDown(start),
        canvas,
        alt
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerMove(end),
        canvas,
        alt
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerUp(end),
        canvas,
        alt
    ));

    assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&shape_id)
            .expect("original shape should remain")
            .kind
            .bounds(),
        original_bounds
    );
    let duplicate_id = app
        .selected_layout_shape()
        .expect("alt-drag should select the duplicate");
    assert_ne!(duplicate_id, shape_id);
    assert_eq!(
        app.workspace()
            .document
            .shapes
            .get(&duplicate_id)
            .expect("duplicate shape should exist")
            .kind
            .bounds(),
        original_bounds.translated(Vector::new(100, 50))
    );
}

#[test]
pub(crate) fn layout_canvas_ctrl_temporarily_disables_snap_for_drawing() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(1.0),
        ..Default::default()
    });
    app.workspace.document.grid = 100;
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    assert!(app.apply_clicked_node_name("glassworks.tool.rect"));
    let ctrl = operad::KeyModifiers {
        ctrl: true,
        ..operad::KeyModifiers::NONE
    };
    let start_world = Point::new(13, 17);
    let end_world = Point::new(257, 189);
    let start = app.layout_world_to_canvas(start_world, rect_size(canvas));
    let end = app.layout_world_to_canvas(end_world, rect_size(canvas));

    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerDown(start),
        canvas,
        ctrl
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerMove(end),
        canvas,
        ctrl
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerUp(end),
        canvas,
        ctrl
    ));

    let shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("rectangle should be created");
    assert_eq!(
        shape.kind,
        ShapeKind::Rectangle(Rect::new(start_world, end_world))
    );
}

#[test]
pub(crate) fn layout_canvas_shift_constrains_rect_tool_to_square() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    assert!(app.apply_clicked_node_name("glassworks.tool.rect"));
    let shift = operad::KeyModifiers {
        shift: true,
        ..operad::KeyModifiers::NONE
    };
    let start = app.layout_world_to_canvas(Point::new(0, 0), rect_size(canvas));
    let end = app.layout_world_to_canvas(Point::new(1_000, 700), rect_size(canvas));

    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerDown(start),
        canvas,
        shift
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerMove(end),
        canvas,
        shift
    ));
    assert!(app.handle_layout_canvas_input_with_modifiers(
        &operad::UiInputEvent::PointerUp(end),
        canvas,
        shift
    ));

    let shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("rectangle should be created");
    assert_eq!(
        shape.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000)))
    );
}

#[test]
pub(crate) fn layout_canvas_select_tool_moves_hierarchy_instance() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        hierarchy_demo: true,
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let index = LayoutIndex::rebuild_hierarchical(&app.workspace.document);
    let flattened = app
        .workspace
        .document
        .visible_flattened_shapes()
        .into_iter()
        .find(|flattened| {
            !flattened.id.is_top_level()
                && index.hit_test_occurrence(flattened.bounds.center(), 1)
                    == Some(flattened.id.clone())
        })
        .expect("hierarchy demo should expose a directly pickable instance occurrence");
    let (parent, instance_id) = app
        .workspace
        .document
        .instance_parent_for_path(&flattened.id.instance_path)
        .expect("picked occurrence should have an instance parent");
    let before = app
        .workspace
        .document
        .instance(parent, instance_id)
        .expect("instance should exist before drag")
        .transform
        .translation;
    let center = flattened.bounds.center();
    app.layout_pan = [
        -(center.x as f32) * app.layout_zoom,
        center.y as f32 * app.layout_zoom,
    ];

    let start = app.layout_world_to_canvas(center, rect_size(canvas));
    let end_world = Point::new(center.x + 240, center.y - 160);
    let end = app.layout_world_to_canvas(end_world, rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(end), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(end), canvas));

    let after = app
        .workspace
        .document
        .instance(parent, instance_id)
        .expect("instance should exist after drag")
        .transform
        .translation;
    assert_eq!(after, Vector::new(before.dx + 240, before.dy - 160));
    assert_eq!(app.selected_layout_occurrence, Some(flattened.id));
}

#[test]
pub(crate) fn layout_canvas_select_tool_resizes_vertices_and_edges() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("test rectangle should be added");

    let corner_start = app.layout_world_to_canvas(Point::new(1_000, 1_000), rect_size(canvas));
    let corner_end = app.layout_world_to_canvas(Point::new(1_200, 1_300), rect_size(canvas));
    assert!(
        app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(corner_start), canvas)
    );
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(corner_end), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(corner_end), canvas));

    let resized = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("resized shape should exist");
    assert_eq!(
        resized.kind.bounds(),
        Rect::new(Point::new(0, 0), Point::new(1_200, 1_300))
    );

    let edge_start = app.layout_world_to_canvas(Point::new(1_200, 600), rect_size(canvas));
    let edge_end = app.layout_world_to_canvas(Point::new(1_500, 600), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(edge_start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(edge_end), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(edge_end), canvas));

    let resized = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("edge-resized shape should exist");
    assert_eq!(
        resized.kind.bounds(),
        Rect::new(Point::new(0, 0), Point::new(1_500, 1_300))
    );
}

#[test]
pub(crate) fn layout_canvas_double_click_inserts_selected_edge_vertex() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 1_000))),
        )
        .expect("test rectangle should be added");
    let edge_midpoint = Point::new(500, 1_000);
    let click = app.layout_world_to_canvas(edge_midpoint, rect_size(canvas));

    assert!(app.handle_layout_canvas_double_click(click, canvas));
    let shape = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("edited shape should exist");
    let ShapeKind::Polygon(poly) = shape.kind else {
        panic!("inserting a vertex into a rectangle edge should convert it to a polygon");
    };
    assert_eq!(poly.points.len(), 5);
    assert!(poly.points.contains(&edge_midpoint));
}

#[test]
pub(crate) fn layout_canvas_delete_key_removes_hovered_selected_vertex() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let shape_id = app
        .add_layout_shape(
            app.active_layer(),
            ShapeKind::Path {
                points: vec![Point::new(0, 0), Point::new(500, 0), Point::new(1_000, 0)],
                width: 180,
            },
        )
        .expect("test path should be added");
    let middle = app.layout_world_to_canvas(Point::new(500, 0), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerMove(middle), canvas));

    assert!(app.handle_layout_editor_key(operad::KeyCode::Delete, operad::KeyModifiers::NONE));
    let shape = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("path should still exist after deleting one vertex");
    let ShapeKind::Path { points, .. } = shape.kind else {
        panic!("shape should remain a path");
    };
    assert_eq!(points, vec![Point::new(0, 0), Point::new(1_000, 0)]);
}

#[test]
pub(crate) fn layout_canvas_secondary_click_finishes_polyline() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    let shape_count = app.workspace().document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.tool.poly"));
    for point in [
        Point::new(0, 0),
        Point::new(1_000, 0),
        Point::new(1_000, 1_000),
    ] {
        let canvas_point = app.layout_world_to_canvas(point, rect_size(canvas));
        assert!(
            app.handle_layout_canvas_input(
                &operad::UiInputEvent::PointerDown(canvas_point),
                canvas
            )
        );
        assert!(
            app.handle_layout_canvas_input(&operad::UiInputEvent::PointerUp(canvas_point), canvas)
        );
    }
    let click = app.layout_world_to_canvas(Point::new(1_000, 1_000), rect_size(canvas));

    assert!(app.handle_layout_canvas_secondary_click(click, canvas));
    assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
    let shape = app
        .selected_layout_shape()
        .and_then(|id| app.workspace().document.shapes.get(&id))
        .expect("finished polygon should become selected");
    assert!(matches!(shape.kind, ShapeKind::Polygon(_)));
}

#[test]
pub(crate) fn layout_transform_actions_rotate_and_mirror_selection() {
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

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.rotate90"));
    let rotated = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("rotated shape should exist");
    assert_eq!(
        rotated.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(250, -250), Point::new(750, 750)))
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.mirror_x"));
    let mirrored = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("mirrored shape should exist");
    assert_eq!(mirrored.kind.bounds(), rotated.kind.bounds());
    assert!(app.status_message().contains("Mirrored X"));
}

#[test]
pub(crate) fn layout_shapewise_sizing_grows_and_shrinks_selected_shape() {
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

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.grow"));
    let grown = app
        .workspace()
        .document
        .shapes
        .get(&shape_id)
        .expect("grown shape should exist");
    assert_eq!(
        grown.kind,
        ShapeKind::Rectangle(Rect::new(
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
        .expect("shrunk shape should exist");
    assert_eq!(
        restored.kind,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(1_000, 500)))
    );
    assert!(app.status_message().contains("Shrank"));
}
