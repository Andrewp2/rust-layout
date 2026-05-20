#![allow(unused_imports)]
use super::*;
use crate::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_loads_compressed_json_snapshots() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("compressed layout source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-100, -100), Point::new(600, 400))),
    )
    .expect("test rectangle should be added");
    let expected_shapes = app.workspace.document.flattened_shape_count_estimate();
    let expected_cells = app.workspace.document.cells.len();
    let layout_path = std::env::temp_dir().join(format!(
        "glassworks-compressed-layout-{}.json",
        std::process::id()
    ));
    let layout_gz_path = layout_path.with_extension("json.gz");
    let layout_zip_path = layout_path.with_extension("json.zip");
    let workspace_path = std::env::temp_dir().join(format!(
        "glassworks-compressed-workspace-{}.json",
        std::process::id()
    ));
    let workspace_gz_path = workspace_path.with_extension("json.gz");
    let workspace_zip_path = workspace_path.with_extension("json.zip");
    for path in [
        &layout_path,
        &layout_gz_path,
        &layout_zip_path,
        &workspace_path,
        &workspace_gz_path,
        &workspace_zip_path,
    ] {
        let _ = std::fs::remove_file(path);
    }

    assert!(app.save_layout_json_to_path(&layout_path));
    let layout_bytes = std::fs::read(&layout_path).expect("layout JSON should be readable");
    write_gzip_test_file(&layout_gz_path, &layout_bytes);
    write_single_file_zip_test_file(&layout_zip_path, "layout.json", &layout_bytes);
    assert!(app.save_workspace_session_to_path(&workspace_path));
    let workspace_bytes =
        std::fs::read(&workspace_path).expect("workspace JSON should be readable");
    write_gzip_test_file(&workspace_gz_path, &workspace_bytes);
    write_single_file_zip_test_file(&workspace_zip_path, "workspace.json", &workspace_bytes);

    let mut loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    loaded.workspace.document = Document::new("empty before compressed layout load");
    loaded.reset_layout_document_state();
    assert!(loaded.load_layout_json_from_path(&layout_gz_path));
    assert_eq!(loaded.active_view, StartupView::Layout2d);
    assert_eq!(
        loaded.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(loaded.workspace.document.cells.len(), expected_cells);
    assert!(
        loaded.status_message().contains("Loaded layout JSON"),
        "{}",
        loaded.status_message()
    );

    let mut loaded_zip = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    loaded_zip.workspace.document = Document::new("empty before zipped layout load");
    loaded_zip.reset_layout_document_state();
    assert!(loaded_zip.load_layout_json_from_path(&layout_zip_path));
    assert_eq!(loaded_zip.active_view, StartupView::Layout2d);
    assert_eq!(
        loaded_zip
            .workspace
            .document
            .flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(loaded_zip.workspace.document.cells.len(), expected_cells);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("target before compressed import");
    imported.reset_layout_document_state();
    let cells_before = imported.workspace.document.cells.len();
    assert!(imported.import_layout_json_as_cell_from_path(&layout_gz_path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert_eq!(
        imported.workspace.document.cells.len(),
        cells_before + expected_cells
    );
    assert!(
        imported.status_message().contains("as cell"),
        "{}",
        imported.status_message()
    );

    let mut top_imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    top_imported.workspace.document = Document::new("target before compressed top import");
    top_imported.reset_layout_document_state();
    let original_top = top_imported.workspace.document.top_cell;
    assert!(top_imported.import_layout_json_as_top_cell_from_path(&layout_gz_path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert!(
        top_imported.status_message().contains("extra top cell"),
        "{}",
        top_imported.status_message()
    );

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before compressed merge");
    merged.reset_layout_document_state();
    assert!(merged.merge_layout_json_from_path(&layout_gz_path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert_eq!(
        merged.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert!(
        merged.status_message().contains("Merged layout JSON"),
        "{}",
        merged.status_message()
    );

    let mut workspace_loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    workspace_loaded.workspace.document = Document::new("empty before compressed workspace load");
    workspace_loaded.reset_layout_document_state();
    assert!(workspace_loaded.load_workspace_session_from_path(&workspace_gz_path));
    assert_eq!(
        workspace_loaded
            .workspace
            .document
            .flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(
        workspace_loaded.workspace.document.cells.len(),
        expected_cells
    );
    assert!(
        workspace_loaded
            .status_message()
            .contains("Loaded workspace"),
        "{}",
        workspace_loaded.status_message()
    );

    let mut workspace_zip_loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    workspace_zip_loaded.workspace.document = Document::new("empty before zipped workspace load");
    workspace_zip_loaded.reset_layout_document_state();
    assert!(workspace_zip_loaded.load_workspace_session_from_path(&workspace_zip_path));
    assert_eq!(
        workspace_zip_loaded
            .workspace
            .document
            .flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(
        workspace_zip_loaded.workspace.document.cells.len(),
        expected_cells
    );

    for path in [
        &layout_path,
        &layout_gz_path,
        &layout_zip_path,
        &workspace_path,
        &workspace_gz_path,
        &workspace_zip_path,
    ] {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_json_import_offset_applies_to_cell_import_and_merge() {
    let mut source = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    source.workspace.document = Document::new("offset import source");
    source.reset_layout_document_state();
    let metal1 = source
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    source
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        )
        .expect("source rectangle should be added");
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-import-offset-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(source.save_layout_json_to_path(&path));

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    imported.workspace.document = Document::new("target before offset import");
    imported.reset_layout_document_state();
    let step = imported.layout_import_offset_step();
    assert!(imported.apply_clicked_node_name("glassworks.menu.item.file.import_offset.x_pos"));
    assert!(imported.apply_clicked_node_name("glassworks.menu.item.file.import_offset.y_neg"));
    assert_eq!(imported.layout_import_offset, Vector::new(step, -step));
    assert!(
        imported.status_message().contains("Import offset"),
        "{}",
        imported.status_message()
    );
    assert!(imported.import_layout_json_as_cell_from_path(&path));
    let top_cell = imported
        .workspace
        .document
        .cell(imported.workspace.document.top_cell)
        .expect("target top cell should exist");
    let imported_instance = top_cell
        .instances
        .values()
        .find(|instance| {
            imported
                .workspace
                .document
                .cell(instance.cell)
                .is_some_and(|cell| {
                    cell.properties
                        .get("import.source_document")
                        .map(String::as_str)
                        == Some("offset import source")
                })
        })
        .expect("offset import should place a root instance");
    assert_eq!(
        imported_instance.transform.translation,
        Vector::new(step, -step)
    );

    assert!(imported.apply_clicked_node_name("glassworks.menu.item.file.import_offset.reset"));
    assert_eq!(imported.layout_import_offset, Vector::ZERO);

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before offset merge");
    merged.reset_layout_document_state();
    let merge_step = merged.layout_import_offset_step();
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.file.import_offset.x_pos"));
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.file.import_offset.y_pos"));
    assert!(merged.merge_layout_json_from_path(&path));
    let merged_bounds = merged
        .workspace
        .document
        .shapes
        .values()
        .next()
        .expect("merge should add a top-level shape")
        .kind
        .bounds();
    assert_eq!(merged_bounds.min, Point::new(merge_step, merge_step));
    assert_eq!(
        merged_bounds.max,
        Point::new(merge_step + 100, merge_step + 50)
    );

    assert!(source.apply_clicked_node_name("glassworks.menu.file"));
    let document = source
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.import_offset.reset",
        "glassworks.menu.item.file.import_offset.x_neg",
        "glassworks.menu.item.file.import_offset.x_pos",
        "glassworks.menu.item.file.import_offset.y_neg",
        "glassworks.menu.item.file.import_offset.y_pos",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("Import Offset"), "{visible_text}");

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_json_import_layer_policy_maps_by_id_name_or_copy() {
    let mut source = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    source.workspace.document = Document::new("layer policy source");
    source.reset_layout_document_state();
    let source_metal1 = source
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let source_metal1_layer = source
        .workspace
        .document
        .layer(source_metal1)
        .expect("source metal1 layer should exist")
        .clone();
    let alternate_source_layer = LayerId(200);
    let mut named_like_metal1 = source_metal1_layer.clone();
    named_like_metal1.id = alternate_source_layer;
    named_like_metal1.gds_layer = Some(200);
    named_like_metal1.display_order += 1;
    source
        .workspace
        .document
        .layers
        .insert(alternate_source_layer, named_like_metal1);
    source.workspace.document.next_layer_id = source
        .workspace
        .document
        .next_layer_id
        .max(alternate_source_layer.0 + 1);
    source.workspace.document.insert_shape(
        alternate_source_layer,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(11, 17), 90, 45)),
    );
    let source_layer_count = source.workspace.document.layers.len();
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-import-layer-policy-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(source.save_layout_json_to_path(&path));

    let imported_shape_layer = |app: &GlassworksApp| {
        let shapes = app.workspace.document.shapes.values().collect::<Vec<_>>();
        assert_eq!(shapes.len(), 1, "merge should import exactly one shape");
        shapes[0].layer
    };

    let mut by_id = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    by_id.workspace.document = Document::new("target by id");
    by_id.reset_layout_document_state();
    let target_layer_count = by_id.workspace.document.layers.len();
    let target_metal1 = by_id
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    assert_eq!(
        by_id.layout_import_layer_policy,
        LayoutImportLayerPolicy::Id
    );
    assert!(by_id.merge_layout_json_from_path(&path));
    assert_ne!(imported_shape_layer(&by_id), target_metal1);
    assert_eq!(
        by_id.workspace.document.layers.len(),
        target_layer_count + 1
    );

    let mut by_name = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    by_name.workspace.document = Document::new("target by name");
    by_name.reset_layout_document_state();
    let target_metal1 = by_name
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    assert!(by_name.apply_clicked_node_name("glassworks.menu.item.file.import_layer_policy.name"));
    assert_eq!(
        by_name.layout_import_layer_policy,
        LayoutImportLayerPolicy::Name
    );
    assert!(
        by_name.status_message().contains("layer name"),
        "{}",
        by_name.status_message()
    );
    assert!(by_name.merge_layout_json_from_path(&path));
    assert_eq!(imported_shape_layer(&by_name), target_metal1);
    assert_eq!(by_name.workspace.document.layers.len(), target_layer_count);

    let mut copy = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    copy.workspace.document = Document::new("target copy");
    copy.reset_layout_document_state();
    let target_metal1 = copy
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("target should include metal1");
    assert!(copy.apply_clicked_node_name("glassworks.menu.item.file.import_layer_policy.copy"));
    assert_eq!(
        copy.layout_import_layer_policy,
        LayoutImportLayerPolicy::Copy
    );
    assert!(copy.merge_layout_json_from_path(&path));
    assert_ne!(imported_shape_layer(&copy), target_metal1);
    assert_eq!(
        copy.workspace.document.layers.len(),
        target_layer_count + source_layer_count
    );

    assert!(source.apply_clicked_node_name("glassworks.menu.file"));
    let document = source
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.import_layer_policy.id",
        "glassworks.menu.item.file.import_layer_policy.name",
        "glassworks.menu.item.file.import_layer_policy.copy",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("Layer Map"), "{visible_text}");

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_json_import_layer_offset_prefers_shifted_layer_ids() {
    let mut source = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    source.workspace.document = Document::new("layer offset source");
    source.reset_layout_document_state();
    let source_layer = source.workspace.document.create_layer(
        "JSON offset layer",
        ProcessLayer::Annotation,
        [0.9, 0.4, 0.1, 0.45],
    );
    source.workspace.document.insert_shape(
        source_layer,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(5, 7), 40, 30)),
    );
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-import-layer-offset-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(source.save_layout_json_to_path(&path));

    let mut target = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    target.workspace.document = Document::new("target before layer offset merge");
    target.reset_layout_document_state();
    assert!(target.apply_clicked_node_name("glassworks.menu.item.file.import_layer_offset.pos"));
    assert_eq!(
        target.layout_import_layer_id_offset,
        LAYOUT_IMPORT_LAYER_ID_OFFSET_STEP
    );
    assert!(
        target.status_message().contains("layer ID offset"),
        "{}",
        target.status_message()
    );
    assert!(target.merge_layout_json_from_path(&path));

    let expected_layer = LayerId(source_layer.0 + LAYOUT_IMPORT_LAYER_ID_OFFSET_STEP as u32);
    let imported_shape = target
        .workspace
        .document
        .shapes
        .values()
        .next()
        .expect("merge should import a shape");
    assert_eq!(imported_shape.layer, expected_layer);
    let imported_layer = target
        .workspace
        .document
        .layer(expected_layer)
        .expect("offset import should create the shifted layer");
    assert_eq!(imported_layer.name, "Imported JSON offset layer");
    assert!(target.workspace.document.next_layer_id > expected_layer.0);

    assert!(target.apply_clicked_node_name("glassworks.menu.item.file.import_layer_offset.reset"));
    assert_eq!(target.layout_import_layer_id_offset, 0);
    assert!(target.apply_clicked_node_name("glassworks.menu.file"));
    let document = target
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.import_layer_offset.reset",
        "glassworks.menu.item.file.import_layer_offset.neg",
        "glassworks.menu.item.file.import_layer_offset.pos",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("Offset 0"), "{visible_text}");

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_tracks_and_opens_recent_files() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.app_options.files.recent_workspace_limit = 2;
    app.workspace.document = Document::new("recent layout source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(10, 20), Point::new(400, 500))),
    )
    .expect("test rectangle should be added");
    let expected_shapes = app.workspace.document.flattened_shape_count_estimate();
    let layout_path = std::env::temp_dir().join(format!(
        "glassworks-recent-layout-{}.json",
        std::process::id()
    ));
    let workspace_path = std::env::temp_dir().join(format!(
        "glassworks-recent-workspace-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&layout_path);
    let _ = std::fs::remove_file(&workspace_path);

    assert!(app.save_layout_json_to_path(&layout_path));
    assert!(app.save_workspace_session_to_path(&workspace_path));
    assert!(app.save_layout_json_to_path(&layout_path));
    assert_eq!(app.app_options.files.recent_files.len(), 2);
    assert_eq!(app.app_options.files.recent_files[0].kind, "layout_json");
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        layout_path.display().to_string()
    );
    assert_eq!(app.app_options.files.recent_files[1].kind, "workspace");

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu with recent files should build");
    for node_name in [
        "glassworks.menu.item.file.reload_recent",
        "glassworks.menu.item.file.recent.0",
        "glassworks.menu.item.file.recent.1",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("Recent"), "{visible_text}");
    assert!(visible_text.contains("Layout"), "{visible_text}");
    assert!(visible_text.contains("Workspace"), "{visible_text}");

    app.workspace.document = Document::new("mutated before recent open");
    app.reset_layout_document_state();
    assert_eq!(app.workspace.document.flattened_shape_count_estimate(), 0);
    assert!(app.apply_clicked_node_name("glassworks.menu.item.file.reload_recent"));
    assert_eq!(app.active_view, StartupView::Layout2d);
    assert_eq!(app.workspace.document.name, "recent layout source");
    assert_eq!(
        app.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert!(
        app.status_message().contains("Loaded layout JSON"),
        "{}",
        app.status_message()
    );

    app.workspace.document = Document::new("mutated before recent open");
    app.reset_layout_document_state();
    assert!(app.apply_clicked_node_name("glassworks.menu.item.file.recent.0"));
    assert_eq!(app.active_view, StartupView::Layout2d);
    assert_eq!(app.workspace.document.name, "recent layout source");
    assert_eq!(
        app.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert!(
        app.status_message().contains("Loaded layout JSON"),
        "{}",
        app.status_message()
    );

    app.app_options.files.recent_workspace_limit = 1;
    app.record_recent_layout_file("workspace", &workspace_path);
    assert_eq!(app.app_options.files.recent_files.len(), 1);
    assert_eq!(app.app_options.files.recent_files[0].kind, "workspace");

    app.show_options_panel = true;
    let options_document = app
        .build_operad_document(UiSize::new(1400.0, 1600.0))
        .expect("options panel with recent files should build");
    assert!(options_document.nodes().iter().any(|node| {
            node.name() == "glassworks.options.files.recent_summary"
                && matches!(node.content(), UiContent::Text(text) if text.text.contains("Recent files: 1 / 1"))
        }));

    let _ = std::fs::remove_file(&layout_path);
    let _ = std::fs::remove_file(&workspace_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_ui_screenshot_export_writes_rgba_ppm_and_png_snapshots() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let path = std::env::temp_dir().join(format!(
        "glassworks-ui-screenshot-export-{}.rgba",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_ui_screenshot_to_path(&path, 320, 200));
    assert!(
        app.status_message().contains("Exported screenshot"),
        "{}",
        app.status_message()
    );
    let metadata = std::fs::metadata(&path).expect("screenshot export should write a file");
    assert_eq!(metadata.len(), 320 * 200 * 4);
    let _ = std::fs::remove_file(&path);
    let ppm_path = std::env::temp_dir().join(format!(
        "glassworks-ui-screenshot-export-{}.ppm",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&ppm_path);
    assert!(app.export_ui_screenshot_ppm_to_path(&ppm_path, 64, 40));
    assert!(
        app.status_message().contains("PPM"),
        "{}",
        app.status_message()
    );
    let ppm = std::fs::read(&ppm_path).expect("PPM screenshot export should write a file");
    let header = b"P6\n64 40\n255\n";
    assert!(
        ppm.starts_with(header),
        "PPM header should include dimensions"
    );
    assert_eq!(ppm.len(), header.len() + 64 * 40 * 3);
    let _ = std::fs::remove_file(&ppm_path);

    let png_path = std::env::temp_dir().join(format!(
        "glassworks-ui-screenshot-export-{}.png",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&png_path);
    assert!(app.export_ui_screenshot_png_to_path(&png_path, 80, 48));
    assert!(
        app.status_message().contains("PNG"),
        "{}",
        app.status_message()
    );
    let png = std::fs::read(&png_path).expect("PNG screenshot export should write a file");
    assert!(
        png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "PNG export should include a PNG signature"
    );
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 80);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 48);
    assert!(
        png.windows(4).any(|chunk| chunk == b"IDAT"),
        "PNG export should contain image data"
    );
    assert!(
        png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]),
        "PNG export should end with the IEND chunk"
    );
    let _ = std::fs::remove_file(&png_path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.file.export_screenshot"),
        "File menu should expose screenshot export"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.file.export_screenshot_ppm"),
        "File menu should expose PPM screenshot export"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.file.export_screenshot_png"),
        "File menu should expose PNG screenshot export"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_exports_and_imports_gds() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("gds menu source");
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
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-menu-export-{}.gds",
        std::process::id()
    ));
    let gz_path = path.with_extension("gds.gz");
    let zip_path = path.with_extension("gds.zip");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);
    let _ = std::fs::remove_file(&zip_path);

    assert!(app.export_layout_gds_to_path(&path));
    assert!(
        app.status_message().contains("Exported GDS"),
        "{}",
        app.status_message()
    );
    assert!(
        std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 0),
        "GDS export should write nonempty bytes"
    );
    let exported_bytes = std::fs::read(&path).expect("GDS export should be readable");
    write_gzip_test_file(&gz_path, &exported_bytes);
    write_single_file_zip_test_file(&zip_path, "layout.gds", &exported_bytes);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("empty before import");
    imported.reset_layout_document_state();
    assert_eq!(
        imported.workspace.document.flattened_shape_count_estimate(),
        0
    );
    assert!(imported.import_layout_gds_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported.status_message().contains("Imported GDS"),
        "{}",
        imported.status_message()
    );
    assert!(
        imported.workspace.document.flattened_shape_count_estimate() >= 1,
        "GDS import should load layout geometry"
    );
    let expected_gds_shapes = imported.workspace.document.flattened_shape_count_estimate();
    let expected_gds_cells = imported.workspace.document.cells.len();
    let mut imported_gz = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_gz.workspace.document = Document::new("empty before compressed import");
    imported_gz.reset_layout_document_state();
    assert!(imported_gz.import_layout_gds_from_path(&gz_path));
    assert_eq!(imported_gz.active_view, StartupView::Layout2d);
    assert!(
        imported_gz.status_message().contains("Imported GDS"),
        "{}",
        imported_gz.status_message()
    );
    assert!(
        imported_gz
            .workspace
            .document
            .flattened_shape_count_estimate()
            >= 1,
        "compressed GDS import should load layout geometry"
    );
    let mut imported_zip = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_zip.workspace.document = Document::new("empty before zipped GDS import");
    imported_zip.reset_layout_document_state();
    assert!(imported_zip.import_layout_gds_from_path(&zip_path));
    assert_eq!(imported_zip.active_view, StartupView::Layout2d);
    assert!(
        imported_zip.status_message().contains("Imported GDS"),
        "{}",
        imported_zip.status_message()
    );
    assert_eq!(
        imported_zip
            .workspace
            .document
            .flattened_shape_count_estimate(),
        expected_gds_shapes
    );
    assert_eq!(
        imported_zip.workspace.document.cells.len(),
        expected_gds_cells
    );

    let mut imported_cell = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_cell.workspace.document = Document::new("target before gds cell import");
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
    assert!(imported_cell.import_layout_gds_as_cell_from_path(&path));
    assert_eq!(imported_cell.active_view, StartupView::Layout2d);
    assert!(
        imported_cell
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "GDS import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported_cell.workspace.document.shapes.len(),
        target_top_shapes_before,
        "GDS imported geometry should live inside the imported cell"
    );
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before + expected_gds_cells
    );
    let top_cell = imported_cell
        .workspace
        .document
        .cell(import_top)
        .expect("target top cell should exist");
    assert_eq!(top_cell.instances.len(), target_instances_before + 1);
    assert!(
        imported_cell
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| !occurrence.instance_path.is_empty()),
        "GDS import-as-cell should select a shape occurrence inside the placed imported cell"
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
    assert_eq!(
        imported_cell
            .workspace
            .document
            .cell(import_top)
            .expect("target top cell should exist after undo")
            .instances
            .len(),
        target_instances_before
    );

    let mut top_imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    top_imported.workspace.document = Document::new("target before gds top-cell import");
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
    let top_shapes_before = top_imported.workspace.document.shapes.len();
    assert!(top_imported.import_layout_gds_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.shapes.len(),
        top_shapes_before
    );
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_gds_cells
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
    assert_eq!(
        top_imported
            .workspace
            .document
            .cell(original_top)
            .expect("original top cell should exist after undo")
            .instances
            .len(),
        top_instances_before
    );
    assert_eq!(top_imported.layout_view_top_cell, original_top);

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before gds merge");
    merged.reset_layout_document_state();
    let merge_top = merged.workspace.document.top_cell;
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
    let merge_instances_before = merged
        .workspace
        .document
        .cell(merge_top)
        .expect("merge top cell should exist")
        .instances
        .len();
    assert!(merged.merge_layout_gds_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "GDS merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_gds_shapes,
        "GDS merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert_eq!(
        merged
            .workspace
            .document
            .cell(merge_top)
            .expect("merge top cell should exist")
            .instances
            .len(),
        merge_instances_before
    );
    assert!(
        merged
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(ShapeOccurrenceId::is_top_level),
        "GDS merge should select a newly merged top-level shape"
    );
    assert!(
        merged.status_message().contains("Merged GDS"),
        "{}",
        merged.status_message()
    );
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);
    let _ = std::fs::remove_file(&zip_path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.export_gds",
        "glassworks.menu.item.file.import_gds",
        "glassworks.menu.item.file.import_gds_cell",
        "glassworks.menu.item.file.import_gds_top_cell",
        "glassworks.menu.item.file.merge_gds",
        "glassworks.menu.item.file.merge_gds_hierarchy",
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
pub(crate) fn layout_file_menu_hierarchy_merges_gds() {
    let mut source = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    source.workspace.document = Document::new("gds hierarchy source");
    source.reset_layout_document_state();
    let metal1 = source
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    source
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(500, 300))),
        )
        .expect("source top rectangle should be added");
    let source_top = source.workspace.document.top_cell;
    let source_child = source.workspace.document.create_cell("gds hierarchy child");
    source
        .workspace
        .document
        .insert_shape_in_cell(
            source_child,
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(160, 120))),
        )
        .expect("source child shape should be inserted");
    source
        .workspace
        .document
        .insert_instance(source_top, source_child, Transform::translate(1_200, 450))
        .expect("source child instance should be inserted");
    let expected_shapes = source.workspace.document.flattened_shape_count_estimate();
    let expected_cells = source.workspace.document.cells.len();
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-menu-hierarchy-merge-{}.gds",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(source.export_layout_gds_to_path(&path));

    let mut target = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    target.workspace.document = Document::new("target before gds hierarchy merge");
    target.reset_layout_document_state();
    let target_top = target.workspace.document.top_cell;
    let target_shape = target
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(5_000, 0), Point::new(5_500, 400))),
        )
        .expect("target rectangle should be added");
    let flattened_before = target.workspace.document.flattened_shape_count_estimate();
    let top_shapes_before = target.workspace.document.shapes.len();
    let cells_before = target.workspace.document.cells.len();
    let instances_before = target
        .workspace
        .document
        .cell(target_top)
        .expect("target top cell should exist")
        .instances
        .len();
    assert!(target.merge_layout_gds_hierarchy_from_path(&path));
    assert_eq!(target.active_view, StartupView::Layout2d);
    assert!(
        target.workspace.document.shapes.contains_key(&target_shape),
        "hierarchy merge should preserve existing top-level geometry"
    );
    assert_eq!(
        target.workspace.document.shapes.len(),
        top_shapes_before + 1,
        "hierarchy merge should splice source-root local geometry into the target top level"
    );
    assert_eq!(
        target.workspace.document.cells.len(),
        cells_before + expected_cells - 1
    );
    let target_top_cell = target
        .workspace
        .document
        .cell(target_top)
        .expect("target top cell should exist after hierarchy merge");
    assert_eq!(target_top_cell.instances.len(), instances_before + 1);
    assert_eq!(
        target.workspace.document.flattened_shape_count_estimate(),
        flattened_before + expected_shapes
    );
    assert!(
        target.status_message().contains("Merged GDS")
            && target.status_message().contains("hierarchy into top cell"),
        "{}",
        target.status_message()
    );
    assert!(target.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(target.workspace.document.shapes.len(), top_shapes_before);
    assert_eq!(target.workspace.document.cells.len(), cells_before);
    assert_eq!(
        target
            .workspace
            .document
            .cell(target_top)
            .expect("target top cell should exist after undo")
            .instances
            .len(),
        instances_before
    );

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_exports_and_imports_cif() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("cif menu source");
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
        "glassworks-layout-menu-export-{}.cif",
        std::process::id()
    ));
    let gz_path = path.with_extension("cif.gz");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.export_layout_cif_to_path(&path));
    assert!(
        app.status_message().contains("Exported CIF"),
        "{}",
        app.status_message()
    );
    let exported = std::fs::read_to_string(&path).expect("CIF export should be readable");
    assert!(exported.contains("L metal1;"));
    write_gzip_test_file(&gz_path, exported.as_bytes());

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("empty before CIF import");
    imported.reset_layout_document_state();
    assert!(imported.import_layout_cif_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported.status_message().contains("Imported CIF"),
        "{}",
        imported.status_message()
    );
    assert!(
        imported.workspace.document.flattened_shape_count_estimate() >= 2,
        "CIF import should load layout geometry"
    );
    let expected_cif_shapes = imported.workspace.document.flattened_shape_count_estimate();
    let expected_cif_cells = imported.workspace.document.cells.len();
    let mut imported_gz = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_gz.workspace.document = Document::new("empty before compressed CIF import");
    imported_gz.reset_layout_document_state();
    assert!(imported_gz.import_layout_cif_from_path(&gz_path));
    assert_eq!(imported_gz.active_view, StartupView::Layout2d);
    assert!(
        imported_gz.status_message().contains("Imported CIF"),
        "{}",
        imported_gz.status_message()
    );
    let mut imported_cell = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_cell.workspace.document = Document::new("target before CIF cell import");
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
    assert!(imported_cell.import_layout_cif_as_cell_from_path(&path));
    assert_eq!(imported_cell.active_view, StartupView::Layout2d);
    assert!(
        imported_cell
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "CIF import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported_cell.workspace.document.shapes.len(),
        target_top_shapes_before,
        "CIF imported geometry should live inside the imported cell"
    );
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before + expected_cif_cells
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
    top_imported.workspace.document = Document::new("target before CIF top-cell import");
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
    assert!(top_imported.import_layout_cif_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_cif_cells
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
    merged.workspace.document = Document::new("target before CIF merge");
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
    assert!(merged.merge_layout_cif_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "CIF merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_cif_shapes,
        "CIF merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert!(
        merged.status_message().contains("Merged CIF"),
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
        "glassworks.menu.item.file.export_cif",
        "glassworks.menu.item.file.import_cif",
        "glassworks.menu.item.file.import_cif_cell",
        "glassworks.menu.item.file.import_cif_top_cell",
        "glassworks.menu.item.file.merge_cif",
        "glassworks.menu.item.file.merge_cif_hierarchy",
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
pub(crate) fn layout_file_menu_exports_and_imports_dxf() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("dxf menu source");
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
        "glassworks-layout-menu-export-{}.dxf",
        std::process::id()
    ));
    let gz_path = path.with_extension("dxf.gz");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&gz_path);

    assert!(app.export_layout_dxf_to_path(&path));
    assert!(
        app.status_message().contains("Exported DXF"),
        "{}",
        app.status_message()
    );
    let exported = std::fs::read_to_string(&path).expect("DXF export should be readable");
    assert!(exported.contains("LWPOLYLINE"));
    write_gzip_test_file(&gz_path, exported.as_bytes());

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("empty before DXF import");
    imported.reset_layout_document_state();
    assert!(imported.import_layout_dxf_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported.status_message().contains("Imported DXF"),
        "{}",
        imported.status_message()
    );
    assert!(
        imported.workspace.document.flattened_shape_count_estimate() >= 2,
        "DXF import should load layout geometry"
    );
    let expected_dxf_shapes = imported.workspace.document.flattened_shape_count_estimate();
    let expected_dxf_cells = imported.workspace.document.cells.len();
    let mut imported_gz = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_gz.workspace.document = Document::new("empty before compressed DXF import");
    imported_gz.reset_layout_document_state();
    assert!(imported_gz.import_layout_dxf_from_path(&gz_path));
    assert_eq!(imported_gz.active_view, StartupView::Layout2d);
    assert!(
        imported_gz.status_message().contains("Imported DXF"),
        "{}",
        imported_gz.status_message()
    );

    let mut imported_cell = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported_cell.workspace.document = Document::new("target before DXF cell import");
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
    assert!(imported_cell.import_layout_dxf_as_cell_from_path(&path));
    assert_eq!(imported_cell.active_view, StartupView::Layout2d);
    assert!(
        imported_cell
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "DXF import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported_cell.workspace.document.shapes.len(),
        target_top_shapes_before,
        "DXF imported geometry should live inside the imported cell"
    );
    assert_eq!(
        imported_cell.workspace.document.cells.len(),
        target_cells_before + expected_dxf_cells
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
    top_imported.workspace.document = Document::new("target before DXF top-cell import");
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
    assert!(top_imported.import_layout_dxf_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_dxf_cells
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
    merged.workspace.document = Document::new("target before DXF merge");
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
    assert!(merged.merge_layout_dxf_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "DXF merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_dxf_shapes,
        "DXF merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert!(
        merged.status_message().contains("Merged DXF"),
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
        "glassworks.menu.item.file.export_dxf",
        "glassworks.menu.item.file.import_dxf",
        "glassworks.menu.item.file.import_dxf_cell",
        "glassworks.menu.item.file.import_dxf_top_cell",
        "glassworks.menu.item.file.merge_dxf",
        "glassworks.menu.item.file.merge_dxf_hierarchy",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}
