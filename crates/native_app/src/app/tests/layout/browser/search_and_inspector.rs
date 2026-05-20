#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_selected_shape_custom_properties_use_browser_fields() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("shape property edit");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;

    let top_shape = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 160, 80)),
        )
        .expect("top shape should be added");
    app.set_layout_browser_search("review.owner");
    app.set_layout_browser_replace("layout");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected shape property controls should build");
    for node_name in [
        "glassworks.viewctl.layout.shape_property.apply_selected",
        "glassworks.viewctl.layout.shape_property.remove_selected",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "selected shape property controls should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_property.apply_selected"));
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&top_shape)
            .and_then(|shape| shape.properties.get("review.owner").cloned()),
        Some("layout".to_string())
    );
    assert!(app.undo_layout_operation());
    assert!(
        app.workspace
            .document
            .shapes
            .get(&top_shape)
            .is_some_and(|shape| !shape.properties.contains_key("review.owner"))
    );
    assert!(app.redo_layout_operation());
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&top_shape)
            .and_then(|shape| shape.properties.get("review.owner").cloned()),
        Some("layout".to_string())
    );

    let child = app.workspace.document.create_cell("prop_child");
    let child_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("child shape should be inserted");
    let instance = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(500, 0))
        .expect("child instance should be inserted");
    app.selected_layout_shape = Some(child_shape);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::from_instance_path(
        child_shape,
        &[instance],
    ));
    app.set_layout_browser_search("review.stage");
    app.set_layout_browser_replace("process");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_property.apply_selected"));
    assert_eq!(
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&child_shape))
            .and_then(|shape| shape.properties.get("review.stage").cloned()),
        Some("process".to_string())
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.shape_property.remove_selected")
    );
    assert!(
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&child_shape))
            .is_some_and(|shape| !shape.properties.contains_key("review.stage"))
    );
    assert!(app.undo_layout_operation());
    assert_eq!(
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&child_shape))
            .and_then(|shape| shape.properties.get("review.stage").cloned()),
        Some("process".to_string())
    );
}

#[test]
pub(crate) fn layout_selected_instance_custom_properties_use_browser_fields() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance property edit");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("instance_prop_child");
    let child_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("child shape should be inserted");
    let instance = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(300, 0))
        .expect("child instance should be inserted");
    app.selected_layout_shape = Some(child_shape);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::from_instance_path(
        child_shape,
        &[instance],
    ));
    app.set_layout_browser_search("review.owner");
    app.set_layout_browser_replace("layout");

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected instance property controls should build");
    for node_name in [
        "glassworks.viewctl.layout.instance_property.apply_selected",
        "glassworks.viewctl.layout.instance_property.remove_selected",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "selected instance property controls should expose {node_name}"
        );
    }

    let instance_property = |app: &GlassworksApp, key: &str| {
        app.workspace
            .document
            .instance(top, instance)
            .and_then(|instance| instance.properties.get(key).cloned())
    };
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_property.apply_selected")
    );
    assert_eq!(
        instance_property(&app, "review.owner"),
        Some("layout".to_string())
    );
    assert!(app.undo_layout_operation());
    assert!(instance_property(&app, "review.owner").is_none());
    assert!(app.redo_layout_operation());
    assert_eq!(
        instance_property(&app, "review.owner"),
        Some("layout".to_string())
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_property.remove_selected")
    );
    assert!(instance_property(&app, "review.owner").is_none());
    assert!(app.undo_layout_operation());
    assert_eq!(
        instance_property(&app, "review.owner"),
        Some("layout".to_string())
    );
}

#[test]
pub(crate) fn layout_current_cell_custom_properties_use_browser_fields() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("cell property edit");
    app.reset_layout_document_state();
    let child = app.workspace.document.create_cell("prop_cell");
    app.layout_view_top_cell = child;
    app.set_layout_browser_search("review.owner");
    app.set_layout_browser_replace("layout");

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with current cell property controls should build");
    for node_name in [
        "glassworks.viewctl.layout.cell_property.apply_current",
        "glassworks.viewctl.layout.cell_property.remove_current",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "current cell property controls should expose {node_name}"
        );
    }

    let cell_property = |app: &GlassworksApp, key: &str| {
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.properties.get(key).cloned())
    };
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_property.apply_current"));
    assert_eq!(
        cell_property(&app, "review.owner"),
        Some("layout".to_string())
    );
    assert!(app.undo_layout_operation());
    assert!(cell_property(&app, "review.owner").is_none());
    assert!(app.redo_layout_operation());
    assert_eq!(
        cell_property(&app, "review.owner"),
        Some("layout".to_string())
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_property.remove_current"));
    assert!(cell_property(&app, "review.owner").is_none());
    assert!(app.undo_layout_operation());
    assert_eq!(
        cell_property(&app, "review.owner"),
        Some("layout".to_string())
    );
}

#[test]
pub(crate) fn layout_measurement_browser_exposes_ruler_properties() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let annotation = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Annotation)
        .unwrap_or(app.active_layer);
    app.workspace.document.shapes.clear();
    app.workspace.document.cells.clear();
    let measurement = app
        .add_layout_shape(
            annotation,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(3_000, 4_000),
                label: "probe gap".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("test measurement should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(9_000, 9_000), 100, 100)),
    )
    .expect("non-measurement shape should be added");
    app.selected_layout_shape = Some(measurement);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(measurement));

    let entries = layout_measurement_entries(&app);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, ShapeOccurrenceId::top_level(measurement));

    let rows = layout_measurement_rows(&app);
    for key in [
        "Visible measurements",
        "Listed total length",
        "Listed ruler modes",
        "Ruler mode",
        "Measurement id",
        "Label",
        "Mode",
        "Length",
        "Delta",
        "Angle",
        "Endpoint A",
        "Endpoint B",
        "Layer",
        "Source cell",
        "Occurrence",
    ] {
        assert!(
            rows.iter().any(|(row_key, _)| row_key == key),
            "measurement rows should expose {key}: {rows:?}"
        );
    }
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Length" && value.contains("5.00")),
        "3-4-5 measurement should report a 5 micron-equivalent length: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Listed total length" && value.contains("5.00")),
        "measurement browser should summarize listed ruler length: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Listed ruler modes" && value == "Direct 1"),
        "measurement browser should summarize listed ruler modes: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Angle" && value == "53.1 deg"),
        "measurement should report endpoint angle: {rows:?}"
    );

    let shape_rows = layout_shape_browser_rows(&app);
    for key in [
        "Ruler label",
        "Ruler mode",
        "Ruler length",
        "Ruler delta",
        "Ruler angle",
    ] {
        assert!(
            shape_rows.iter().any(|(row_key, _)| row_key == key),
            "shape browser should expose measurement property {key}"
        );
    }

    let action = format!(
        "glassworks.viewctl.layout.measurement.{}",
        layout_occurrence_action_key(&ShapeOccurrenceId::top_level(measurement))
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with measurements should build");
    for node_name in [
        "glassworks.layout.measurement_browser.title".to_string(),
        "glassworks.viewctl.layout.measurement_mode.direct".to_string(),
        "glassworks.viewctl.layout.measurement_mode.horizontal".to_string(),
        "glassworks.viewctl.layout.measurement_mode.vertical".to_string(),
        "glassworks.viewctl.layout.measurement_mode.manhattan".to_string(),
        "glassworks.viewctl.layout.measurements.clear".to_string(),
        "glassworks.viewctl.layout.measurements.focus_selected".to_string(),
        "glassworks.viewctl.layout.measurements.delete_selected".to_string(),
        action.clone(),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "measurement browser should expose {node_name}"
        );
    }

    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(app.apply_clicked_node_name(&action));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(measurement))
    );
    app.layout_canvas_size = Some(UiSize::new(800.0, 600.0));
    app.layout_zoom = 0.1;
    app.layout_pan = [27.0, -14.0];
    let previous_view = app.current_layout_view_state();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurements.focus_selected"));
    let measurement_bounds = ShapeKind::Measurement {
        a: Point::new(0, 0),
        b: Point::new(3_000, 4_000),
        label: "probe gap".to_string(),
        mode: MeasurementMode::Direct,
    }
    .bounds();
    let expected_zoom = (800.0_f32 / (measurement_bounds.width() as f32 * 1.12))
        .min(600.0_f32 / (measurement_bounds.height() as f32 * 1.12));
    let center = measurement_bounds.center();
    assert!((app.layout_zoom - expected_zoom).abs() < 0.0001);
    assert_eq!(
        app.layout_pan,
        [
            -(center.x as f32) * expected_zoom,
            center.y as f32 * expected_zoom,
        ]
    );
    assert_eq!(app.layout_previous_view, Some(previous_view));
    assert!(app.status_message().contains("Focused measurement"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurements.delete_selected"));
    assert!(layout_measurement_entries(&app).is_empty());
    assert_eq!(app.selected_layout_occurrence, None);
    assert!(
        app.status_message()
            .contains("Deleted selected measurement"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(layout_measurement_entries(&app).len(), 1);

    app.layout_browser_search = "probe".to_string();
    assert_eq!(layout_measurement_entries(&app).len(), 1);
    app.layout_browser_search = "missing".to_string();
    assert!(layout_measurement_entries(&app).is_empty());
    let search_rows = layout_measurement_rows(&app);
    assert!(
        search_rows
            .iter()
            .any(|(key, value)| key == "Visible measurements" && value == "1")
    );
    assert!(
        search_rows
            .iter()
            .any(|(key, value)| key == "Listed" && value == "0")
    );
    assert!(
        search_rows
            .iter()
            .any(|(key, value)| key == "Listed ruler modes" && value == "None")
    );

    app.layout_browser_search.clear();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurements.clear"));
    assert!(layout_measurement_entries(&app).is_empty());
    assert_eq!(app.selected_layout_shape, None);
    assert!(
        app.workspace
            .document
            .shapes
            .values()
            .any(|shape| !matches!(shape.kind, ShapeKind::Measurement { .. })),
        "clearing measurements should leave non-measurement geometry intact"
    );
    assert!(
        app.status_message()
            .contains("Cleared 1 visible measurement"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(layout_measurement_entries(&app).len(), 1);
}

#[test]
pub(crate) fn layout_measurement_browser_select_first_uses_search() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("measurement select first");
    app.reset_layout_document_state();
    let annotation = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Annotation)
        .unwrap_or(app.active_layer);
    let _probe = app
        .add_layout_shape(
            annotation,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(1_000, 0),
                label: "probe gap".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("probe measurement should be added");
    let target = app
        .add_layout_shape(
            annotation,
            ShapeKind::Measurement {
                a: Point::new(2_000, 0),
                b: Point::new(2_000, 3_000),
                label: "target span".to_string(),
                mode: MeasurementMode::Vertical,
            },
        )
        .expect("target measurement should be added");
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    app.set_layout_browser_search("target");

    let entries = layout_measurement_entries(&app);
    assert_eq!(entries.len(), 1, "search should isolate the target ruler");
    assert_eq!(entries[0].0, ShapeOccurrenceId::top_level(target));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with measurement select-first should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.measurement_browser.select_first"
        }),
        "measurement browser should expose select-first when search has a match"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_browser.select_first")
    );
    assert_eq!(app.selected_layout_shape, Some(target));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(target))
    );
    assert!(
        app.status_message()
            .contains("Selected browser measurement")
    );
}

#[test]
pub(crate) fn layout_measurement_modes_constrain_and_label_rulers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document.shapes.clear();
    let canvas = UiRect::new(0.0, 0.0, 800.0, 600.0);
    assert!(app.apply_clicked_node_name("glassworks.tool.measure"));

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("measure tool document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.measurement_mode.horizontal"),
        "measure tool should expose ruler mode controls"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_mode.horizontal"));
    assert_eq!(app.layout_measurement_mode, MeasurementMode::Horizontal);
    assert_eq!(app.app_options.layout.measurement_mode, "horizontal");
    let start = app.layout_world_to_canvas(Point::new(0, 0), rect_size(canvas));
    let end = app.layout_world_to_canvas(Point::new(3_000, 2_000), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(end), canvas));
    let horizontal = app
        .selected_layout_shape
        .and_then(|id| app.workspace.document.shapes.get(&id))
        .expect("horizontal ruler should be selected");
    match horizontal.kind {
        ShapeKind::Measurement { a, b, label, mode } => {
            assert_eq!(a, Point::new(0, 0));
            assert_eq!(b, Point::new(3_000, 0));
            assert_eq!(mode, MeasurementMode::Horizontal);
            assert!(label.contains("3.00"), "{label}");
        }
        _ => panic!("expected a measurement"),
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_mode.manhattan"));
    let start = app.layout_world_to_canvas(Point::new(0, 0), rect_size(canvas));
    let end = app.layout_world_to_canvas(Point::new(3_000, 2_000), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(end), canvas));
    let manhattan = app
        .selected_layout_shape
        .and_then(|id| app.workspace.document.shapes.get(&id))
        .expect("manhattan ruler should be selected");
    match manhattan.kind {
        ShapeKind::Measurement { a, b, label, mode } => {
            assert_eq!(a, Point::new(0, 0));
            assert_eq!(b, Point::new(3_000, 2_000));
            assert_eq!(mode, MeasurementMode::Manhattan);
            assert!(label.contains("5.00"), "{label}");
        }
        _ => panic!("expected a measurement"),
    }
    let rows = layout_measurement_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Mode" && value == "Manhattan"),
        "measurement browser rows should expose ruler mode: {rows:?}"
    );
}

#[test]
pub(crate) fn layout_measurement_browser_filters_by_layer_and_mode() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("measurement filter test");
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
    let mut ruler_ids = Vec::new();
    for (layer, mode, label, offset) in [
        (metal1, MeasurementMode::Direct, "direct-ruler", 0),
        (
            metal1,
            MeasurementMode::Horizontal,
            "horizontal-ruler",
            1_000,
        ),
        (metal2, MeasurementMode::Vertical, "vertical-ruler", 2_000),
        (metal2, MeasurementMode::Manhattan, "manhattan-ruler", 3_000),
    ] {
        let ruler_id = app
            .add_layout_shape(
                layer,
                ShapeKind::Measurement {
                    a: Point::new(offset, 0),
                    b: Point::new(offset + 300, 400),
                    label: label.to_string(),
                    mode,
                },
            )
            .expect("measurement should be added");
        ruler_ids.push((ruler_id, layer, mode));
    }
    app.active_layer = metal2;

    let selected_ruler = ruler_ids
        .iter()
        .find(|(_, layer, mode)| *layer == metal2 && *mode == MeasurementMode::Vertical)
        .map(|(id, _, _)| *id)
        .expect("test should create a selected vertical ruler");
    app.selected_layout_shape = Some(selected_ruler);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(selected_ruler));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_filter.selected"));
    let selected_entries = layout_measurement_entries(&app);
    assert_eq!(selected_entries.len(), 1);
    assert_eq!(
        selected_entries[0].0,
        ShapeOccurrenceId::top_level(selected_ruler)
    );
    assert!(
        layout_measurement_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Selected")
    );
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(layout_measurement_entries(&app).is_empty());

    let fallback_selected_ruler = ruler_ids
        .iter()
        .find(|(_, _, mode)| *mode == MeasurementMode::Direct)
        .map(|(id, _, _)| *id)
        .expect("test should create a fallback selected ruler");
    app.selected_layout_shape = Some(fallback_selected_ruler);
    let fallback_entries = layout_measurement_entries(&app);
    assert_eq!(fallback_entries.len(), 1);
    assert_eq!(
        fallback_entries[0].0,
        ShapeOccurrenceId::top_level(fallback_selected_ruler)
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_filter.active_layer")
    );
    let active_layer_entries = layout_measurement_entries(&app);
    assert_eq!(active_layer_entries.len(), 2);
    assert!(
        active_layer_entries
            .iter()
            .all(|(_, _, shape)| shape.layer == metal2)
    );
    assert!(
        layout_measurement_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Active Layer")
    );
    assert!(
        app.status_message()
            .contains("Measurement filter Active Layer (2)"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_filter.all"));
    assert!(
        app.status_message().contains("Measurement filter All (4)"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_sort.length"));
    assert_eq!(
        app.layout_measurement_browser_sort,
        LayoutMeasurementBrowserSort::Length
    );
    let length_entries = layout_measurement_entries(&app);
    let Some((_, _, _, length_mode)) = layout_measurement_geometry(&length_entries[0].2.kind)
    else {
        panic!("length-sorted first entry should be a measurement");
    };
    assert_eq!(length_mode, MeasurementMode::Manhattan);
    assert!(
        layout_measurement_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Length")
    );
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_browser.select_first")
    );
    let manhattan_ruler = ruler_ids
        .iter()
        .find(|(_, _, mode)| *mode == MeasurementMode::Manhattan)
        .map(|(id, _, _)| *id)
        .expect("test should create a manhattan ruler");
    assert_eq!(app.selected_layout_shape, Some(manhattan_ruler));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_sort.mode"));
    let mode_entries = layout_measurement_entries(&app);
    let Some((_, _, _, first_mode)) = layout_measurement_geometry(&mode_entries[0].2.kind) else {
        panic!("mode-sorted first entry should be a measurement");
    };
    assert_eq!(first_mode, MeasurementMode::Direct);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_sort.layer"));
    let layer_entries = layout_measurement_entries(&app);
    assert_eq!(layer_entries[0].2.layer, metal1);
    assert_eq!(layer_entries[1].2.layer, metal1);
    assert_eq!(layer_entries[2].2.layer, metal2);
    assert_eq!(layer_entries[3].2.layer, metal2);

    for (slug, expected_mode) in [
        ("direct", MeasurementMode::Direct),
        ("horizontal", MeasurementMode::Horizontal),
        ("vertical", MeasurementMode::Vertical),
        ("manhattan", MeasurementMode::Manhattan),
    ] {
        assert!(app.apply_clicked_node_name(&format!(
            "glassworks.viewctl.layout.measurement_filter.{slug}"
        )));
        let entries = layout_measurement_entries(&app);
        assert_eq!(entries.len(), 1, "{slug} filter should list one ruler");
        let Some((_, _, _, mode)) = layout_measurement_geometry(&entries[0].2.kind) else {
            panic!("filtered entry should be a measurement");
        };
        assert_eq!(mode, expected_mode);
    }

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with measurement filters should build");
    for node_name in [
        "glassworks.viewctl.layout.measurement_filter.all",
        "glassworks.viewctl.layout.measurement_filter.selected",
        "glassworks.viewctl.layout.measurement_filter.active_layer",
        "glassworks.viewctl.layout.measurement_filter.direct",
        "glassworks.viewctl.layout.measurement_filter.horizontal",
        "glassworks.viewctl.layout.measurement_filter.vertical",
        "glassworks.viewctl.layout.measurement_filter.manhattan",
        "glassworks.viewctl.layout.measurement_sort.id",
        "glassworks.viewctl.layout.measurement_sort.length",
        "glassworks.viewctl.layout.measurement_sort.angle",
        "glassworks.viewctl.layout.measurement_sort.mode",
        "glassworks.viewctl.layout.measurement_sort.layer",
        "glassworks.viewctl.layout.measurement_sort.source_cell",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "measurement browser filter should exist: {node_name}"
        );
    }
    let measurement_title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.measurement_browser.title")
        .unwrap_or_else(|| panic!("measurement browser title should exist"));
    let UiContent::Text(measurement_title_text) = measurement_title.content() else {
        panic!("measurement browser title should be text");
    };
    assert_eq!(
        measurement_title_text.text,
        "Measurements - Manhattan (1 row)"
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
        filter_button_label("glassworks.viewctl.layout.measurement_filter.all"),
        "Filter All (4)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.selected"),
        "Filter Selected (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.active_layer"),
        "Filter Active Layer (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.direct"),
        "Filter Direct (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.horizontal"),
        "Filter Horizontal (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.vertical"),
        "Filter Vertical (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.measurement_filter.manhattan"),
        "Filter Manhattan (1)"
    );

    app.layout_browser_search = "missing".to_string();
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_browser.select_first")
    );
    assert!(
        app.status_message()
            .contains("Measurement browser has no matching Manhattan rulers for search missing"),
        "{}",
        app.status_message()
    );
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty measurement search should build");
    let empty_title = empty_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.measurement_browser.title")
        .unwrap_or_else(|| panic!("empty measurement browser title should exist"));
    let UiContent::Text(empty_title_text) = empty_title.content() else {
        panic!("empty measurement browser title should be text");
    };
    assert_eq!(
        empty_title_text.text,
        "Measurements - Manhattan / search missing (0 rows)"
    );
}

#[test]
pub(crate) fn layout_measurement_browser_searches_and_sorts_by_source_cell() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("measurement source cell sort test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let zeta = app.workspace.document.create_cell("zeta ruler source");
    let alpha = app.workspace.document.create_cell("alpha ruler source");
    let zeta_ruler = app
        .workspace
        .document
        .insert_shape_in_cell(
            zeta,
            metal1,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(100, 0),
                label: "zeta-ruler".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("zeta source measurement should be inserted");
    let alpha_ruler = app
        .workspace
        .document
        .insert_shape_in_cell(
            alpha,
            metal1,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(200, 0),
                label: "alpha-ruler".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("alpha source measurement should be inserted");
    app.workspace
        .document
        .insert_instance(top, zeta, Transform::translate(0, 0))
        .expect("zeta source instance should be inserted");
    app.workspace
        .document
        .insert_instance(top, alpha, Transform::translate(1_000, 0))
        .expect("alpha source instance should be inserted");

    app.set_layout_browser_search("source_cell=alpha ruler");
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].2.id, alpha_ruler);
    app.set_layout_browser_search(&format!("source_cell_id={}", zeta.0));
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].2.id, zeta_ruler);
    app.set_layout_browser_search("length_dbu=200");
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(
        searched[0].2.id, alpha_ruler,
        "measurement selector search should match integer DBU lengths"
    );
    app.set_layout_browser_search("end=100,0");
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(
        searched[0].2.id, zeta_ruler,
        "measurement selector search should match endpoint aliases"
    );
    app.set_layout_browser_search("x2=100");
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(
        searched[0].2.id, zeta_ruler,
        "measurement selector search should match endpoint coordinate aliases"
    );
    app.set_layout_browser_search("dx=200");
    let searched = layout_measurement_entries(&app);
    assert_eq!(searched.len(), 1);
    assert_eq!(
        searched[0].2.id, alpha_ruler,
        "measurement selector search should match delta aliases"
    );
    app.set_layout_browser_search("");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_sort.source_cell"));
    assert_eq!(
        app.layout_measurement_browser_sort,
        LayoutMeasurementBrowserSort::SourceCell
    );
    let sorted = layout_measurement_entries(&app)
        .into_iter()
        .map(|(_, source_cell, shape)| (source_cell, shape.id))
        .collect::<Vec<_>>();
    assert_eq!(sorted, vec![(alpha, alpha_ruler), (zeta, zeta_ruler)]);
    assert!(
        layout_measurement_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Source Cell"),
        "measurement browser rows should expose source-cell sort"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with source-cell measurement sort should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.measurement_sort.source_cell"
        }),
        "measurement browser sort surface should expose source-cell sort"
    );
}

#[test]
pub(crate) fn layout_measurement_browser_sorts_by_angle() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("measurement angle sort test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let zero = app
        .add_layout_shape(
            metal1,
            ShapeKind::Measurement {
                a: Point::new(0, 0),
                b: Point::new(100, 0),
                label: "zero".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("zero-degree measurement should be added");
    let positive = app
        .add_layout_shape(
            metal1,
            ShapeKind::Measurement {
                a: Point::new(200, 0),
                b: Point::new(200, 100),
                label: "positive".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("positive-angle measurement should be added");
    let negative = app
        .add_layout_shape(
            metal1,
            ShapeKind::Measurement {
                a: Point::new(400, 0),
                b: Point::new(400, -100),
                label: "negative".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("negative-angle measurement should be added");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.measurement_sort.angle"));
    assert_eq!(
        app.layout_measurement_browser_sort,
        LayoutMeasurementBrowserSort::Angle
    );
    let sorted = layout_measurement_entries(&app)
        .into_iter()
        .map(|(occurrence, _, _)| occurrence.source_shape_id())
        .collect::<Vec<_>>();
    assert_eq!(sorted, vec![negative, zero, positive]);
    assert!(
        layout_measurement_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Angle"),
        "measurement browser rows should expose angle sort"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with angle sort should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.measurement_sort.angle"),
        "measurement browser sort surface should expose angle sort"
    );
}

#[test]
pub(crate) fn layout_browser_quick_filters_limit_shape_and_net_rows() {
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
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("demo document should have metal2");
    app.workspace.document.shapes.clear();

    app.add_layout_shape_with_metadata(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-1_000, -200), Point::new(-200, 200))),
        Some(NetId(7)),
        Some("DATA".to_string()),
    )
    .expect("labeled metal1 shape should be added");
    app.add_layout_shape(
        metal2,
        ShapeKind::Rectangle(Rect::new(Point::new(400, -200), Point::new(1_200, 200))),
    )
    .expect("unlabeled metal2 shape should be added");
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    app.active_layer = metal2;

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_filter.active_layer")
    );
    let shape_entries = layout_shape_browser_entries(&app);
    assert!(!shape_entries.is_empty());
    assert!(shape_entries.iter().all(|(occurrence, _)| {
        app.workspace
            .document
            .shape_view_for_occurrence_from_cell(app.layout_view_top_cell, occurrence)
            .is_some_and(|view| view.shape.layer == metal2)
    }));
    let shape_rows = layout_shape_browser_rows(&app);
    for key in ["Shape id", "Kind", "Layer", "Source cell", "Bounds"] {
        assert!(
            shape_rows.iter().any(|(row_key, _)| row_key == key),
            "shape browser detail rows should expose property column {key}"
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_sort.layer"));
    assert_eq!(app.layout_shape_browser_sort, LayoutShapeBrowserSort::Layer);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.labeled"));
    let labeled_entries = layout_net_browser_entries(&app);
    assert_eq!(labeled_entries.len(), 1);
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    assert!(
        report
            .component(labeled_entries[0].0)
            .is_some_and(connectivity_component_is_labeled)
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.unlabeled"));
    let unlabeled_entries = layout_net_browser_entries(&app);
    assert_eq!(unlabeled_entries.len(), 1);
    assert!(
        report
            .component(unlabeled_entries[0].0)
            .is_some_and(|component| !connectivity_component_is_labeled(component))
    );
    let net_rows = layout_net_browser_rows(&app);
    for key in ["Component id", "Display name", "Shapes", "Labels", "Bounds"] {
        assert!(
            net_rows.iter().any(|(row_key, _)| row_key == key),
            "net browser detail rows should expose property column {key}"
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_sort.size"));
    assert_eq!(app.layout_net_browser_sort, LayoutNetBrowserSort::Size);

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with browser filters should build");
    for node_name in [
        "glassworks.viewctl.layout.shape_browser_filter.active_layer",
        "glassworks.viewctl.layout.shape_browser_sort.layer",
        "glassworks.viewctl.layout.net_browser_filter.labeled",
        "glassworks.viewctl.layout.net_browser_filter.unlabeled",
        "glassworks.viewctl.layout.net_browser_filter.devices",
        "glassworks.viewctl.layout.net_browser_filter.history",
        "glassworks.viewctl.layout.net_browser_filter.spice_extra",
        "glassworks.viewctl.layout.net_browser_filter.shorted",
        "glassworks.viewctl.layout.net_browser_filter.open",
        "glassworks.viewctl.layout.net_browser_sort.size",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "browser filter control should exist: {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_net_browser_filters_short_and_open_components() {
    let occurrence_a = ShapeOccurrenceId::top_level(ShapeId(1));
    let occurrence_b = ShapeOccurrenceId::top_level(ShapeId(2));
    let occurrence_c = ShapeOccurrenceId::top_level(ShapeId(3));
    let component = |id, occurrence: ShapeOccurrenceId, name: Option<&str>| NetComponent {
        id,
        shapes: vec![occurrence],
        bounds: Rect::from_min_size(Point::new(id as Coord * 1_000, 0), 200, 100),
        labels: Vec::new(),
        explicit_nets: Vec::new(),
        net_name: name.map(str::to_string),
        net_id: None,
    };
    let report = ConnectivityReport {
        components: vec![
            component(1, occurrence_a.clone(), Some("DATA")),
            component(2, occurrence_b.clone(), Some("CLK")),
            component(3, occurrence_c.clone(), Some("CLK")),
        ],
        shape_to_component: BTreeMap::from([
            (occurrence_a, 1),
            (occurrence_b, 2),
            (occurrence_c, 3),
        ]),
        shorts: vec![NetShort {
            component: 1,
            names: vec!["DATA".to_string(), "VSS".to_string()],
            bounds: Rect::from_min_size(Point::new(0, 0), 200, 100),
        }],
        opens: vec![NetOpen {
            name: "CLK".to_string(),
            components: vec![2, 3],
            bounds: Rect::from_min_size(Point::new(2_000, 0), 1_200, 100),
        }],
        devices: vec![ExtractedDevice {
            id: 1,
            kind: "mos".to_string(),
            model: "nmos".to_string(),
            terminals: vec![
                ExtractedDeviceTerminal {
                    name: "D".to_string(),
                    component: Some(2),
                    net_name: Some("CLK".to_string()),
                },
                ExtractedDeviceTerminal {
                    name: "G".to_string(),
                    component: None,
                    net_name: Some("GATE".to_string()),
                },
            ],
            bounds: Rect::from_min_size(Point::new(2_000, 0), 300, 100),
            width: 300,
            length: 100,
            occurrences: Vec::new(),
        }],
        ..Default::default()
    };

    let shorted = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::Shorted,
        LayoutNetBrowserSort::Id,
        None,
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(shorted, vec![1]);
    assert_eq!(layout_net_component_issue_label(&report, 1), "1 short");
    assert_eq!(
        layout_net_component_issue_summary(&report, 1).as_deref(),
        Some("Short DATA/VSS")
    );
    assert_eq!(layout_net_component_device_count(&report, 1), 0);

    let devices = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::Devices,
        LayoutNetBrowserSort::Id,
        None,
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(devices, vec![2]);
    assert_eq!(layout_net_component_device_count(&report, 2), 1);
    assert_eq!(layout_net_component_device_summary(&report, 2), "#1 nmos D");
    assert_eq!(
        layout_net_component_device_peer_summary(&report, 2).as_deref(),
        Some("#1 nmos D -> G GATE")
    );
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.layout_browser_columns = LayoutBrowserColumnSet::Summary;
    let short_rows = layout_net_browser_property_rows(&app, &report, &[(1, "DATA".to_string())]);
    assert!(
        short_rows
            .iter()
            .any(|(key, value)| key == "Issue Detail" && value == "Short DATA/VSS"),
        "net browser selected-component rows should expose short details: {short_rows:?}"
    );
    let rows = layout_net_browser_property_rows(&app, &report, &[(2, "CLK".to_string())]);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Device Detail" && value == "#1 nmos D"),
        "net browser selected-component rows should expose device terminals: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Issue Detail" && value == "Open CLK"),
        "net browser selected-component rows should expose open details: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Open Peers" && value == "Open CLK -> #3 CLK"),
        "net browser selected-component rows should expose open peer nets: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Device Peers" && value == "#1 nmos D -> G GATE"),
        "net browser selected-component rows should expose peer device terminals: {rows:?}"
    );

    let history = layout_net_browser_entries_from_report_with_history(
        &report,
        LayoutNetBrowserFilter::History,
        LayoutNetBrowserSort::Id,
        None,
        &[3, 1],
        None,
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(history, vec![1, 3]);

    let open = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::Open,
        LayoutNetBrowserSort::Id,
        None,
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(open, vec![2, 3]);
    assert_eq!(layout_net_component_issue_label(&report, 2), "1 open");
    assert_eq!(
        layout_net_component_issue_summary(&report, 2).as_deref(),
        Some("Open CLK")
    );
    assert_eq!(
        layout_net_component_open_peer_summary(&report, 2).as_deref(),
        Some("Open CLK -> #3 CLK")
    );

    let device_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("nmos"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        device_search,
        vec![2],
        "net browser search should match connected device model names"
    );

    let short_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("short DATA"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        short_search,
        vec![1],
        "net browser search should match selected-component short details"
    );

    let open_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("open CLK"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        open_search,
        vec![2, 3],
        "net browser search should match selected-component open details"
    );

    let component_selector_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("component_id=1"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        component_selector_search,
        vec![1],
        "net browser selector search should match component ids"
    );

    let name_selector_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("net_name=CLK"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        name_selector_search,
        vec![2, 3],
        "net browser selector search should match net names"
    );

    let device_selector_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("device_model=nmos"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        device_selector_search,
        vec![2],
        "net browser selector search should match connected device metadata"
    );

    let issue_selector_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("issue=Short DATA/VSS"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        issue_selector_search,
        vec![1],
        "net browser selector search should match component issue summaries"
    );

    let bounds_selector_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("bounds=2000,0 200x100"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        bounds_selector_search,
        vec![2],
        "net browser selector search should match component bounds"
    );
}

#[test]
pub(crate) fn layout_instance_browser_selects_child_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 40)),
        )
        .expect("test child shape should be inserted");
    let grandchild = app.workspace.document.create_cell("nested_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            grandchild,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(8, 8), 24, 24)),
        )
        .expect("test nested child shape should be inserted");
    let nested_instance_id = app
        .workspace
        .document
        .insert_instance(child, grandchild, Transform::translate(20, 10))
        .expect("test nested instance should be inserted");
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(120, 80))
        .expect("test child instance should be inserted");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.top"));
    assert!(
        !layout_instance_browser_entries(&app)
            .iter()
            .any(|(parent, id, _)| *parent == child && *id == nested_instance_id),
        "current-cell instance scope should only list direct children of the view root"
    );
    let action = format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(app.workspace.document.top_cell, instance_id)
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.instance_browser.title"),
        "layout inspector should expose an instance browser"
    );
    assert!(
        document.nodes().iter().any(|node| node.name() == action),
        "instance browser should expose {action}"
    );
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.instance_browser_scope.hierarchy"
        }),
        "instance browser should expose hierarchy scope control"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.instance_browser_sort.child" }),
        "instance browser should expose sortable property controls"
    );
    let instance_rows = layout_instance_browser_rows(&app);
    for key in [
        "Instance id",
        "Parent cell",
        "Child cell",
        "Translation",
        "Matrix",
        "Array",
    ] {
        assert!(
            instance_rows.iter().any(|(row_key, _)| row_key == key),
            "instance browser detail rows should expose property column {key}"
        );
    }

    assert!(app.apply_clicked_node_name(&action));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::One);
    assert_eq!(
        app.selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| occurrence.instance_path.first().copied()),
        Some(instance_id)
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_scope.hierarchy")
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_sort.child"));
    assert_eq!(
        app.layout_instance_browser_sort,
        LayoutInstanceBrowserSort::Child
    );
    assert!(
        layout_instance_browser_entries(&app)
            .iter()
            .any(|(parent, id, _)| *parent == child && *id == nested_instance_id),
        "hierarchy instance scope should include instances owned by reachable child cells"
    );
    let nested_action = format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(child, nested_instance_id)
    );
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy instance scope");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == nested_action),
        "hierarchy instance browser should expose {nested_action}"
    );
    assert!(app.apply_clicked_node_name(&nested_action));
    assert_eq!(app.layout_view_top_cell, child);
    assert_eq!(
        app.selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| occurrence.instance_path.first().copied()),
        Some(nested_instance_id)
    );
}

#[test]
pub(crate) fn layout_instance_browser_filters_named_and_array_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance browser filter test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let child = app.workspace.document.create_cell("leaf");
    let named = app
        .workspace
        .document
        .insert_instance(top, child, Transform::IDENTITY)
        .expect("named instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, named)
            .expect("named instance should be mutable");
        instance.name = Some("placed_leaf".to_string());
    }
    let array = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(100, 0))
        .expect("array instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, array)
            .expect("array instance should be mutable");
        instance.array = InstanceArray {
            columns: 3,
            rows: 2,
            column_pitch: Vector::new(20, 0),
            row_pitch: Vector::new(0, 20),
        };
    }
    let plain = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(200, 0))
        .expect("plain instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, plain)
            .expect("plain instance should be mutable");
        instance
            .properties
            .insert("review.owner".to_string(), "layout".to_string());
    }

    let all = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(all, vec![named, array, plain]);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.named"));
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Named
    );
    let named_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(named_entries, vec![named]);

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.arrays")
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Arrays
    );
    let array_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(array_entries, vec![array]);

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.properties")
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Properties
    );
    let property_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(property_entries, vec![plain]);
    app.set_layout_browser_search("review.owner=layout");
    let property_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(
        property_entries,
        vec![plain],
        "instance property filter should support key=value selectors"
    );
    app.set_layout_browser_search("leaf");
    assert!(
        layout_instance_browser_entries(&app).is_empty(),
        "instance property filter search should not match child-cell names"
    );
    app.set_layout_browser_search("");
    let instance_rows = layout_instance_browser_rows(&app);
    assert!(
        instance_rows
            .iter()
            .any(|(key, value)| { key == "Property review.owner" && value == "layout" }),
        "instance browser rows should expose stored instance properties"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.identity")
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Identity
    );
    let identity_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(identity_entries, vec![named]);
    assert!(
        layout_instance_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Identity"),
        "instance browser rows should show the identity filter"
    );

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.instance_browser_filter.transformed"
        )
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Transformed
    );
    assert!(
        app.status_message()
            .contains("Instance browser filter Transformed (2)"),
        "{}",
        app.status_message()
    );
    let transformed_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(transformed_entries, vec![array, plain]);
    assert!(
        layout_instance_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Transformed"),
        "instance browser rows should show the transformed filter"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.viewctl.layout.instance_browser_filter.all".to_string(),
        "glassworks.viewctl.layout.instance_browser_filter.named".to_string(),
        "glassworks.viewctl.layout.instance_browser_filter.properties".to_string(),
        "glassworks.viewctl.layout.instance_browser_filter.arrays".to_string(),
        "glassworks.viewctl.layout.instance_browser_filter.identity".to_string(),
        "glassworks.viewctl.layout.instance_browser_filter.transformed".to_string(),
        format!(
            "glassworks.viewctl.layout.instance.{}",
            layout_instance_action_key(top, array)
        ),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "instance browser filter surface should expose {node_name}"
        );
    }
    let title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.instance_browser.title")
        .unwrap_or_else(|| panic!("instance browser title should exist"));
    let UiContent::Text(title_text) = title.content() else {
        panic!("instance browser title should be text");
    };
    assert_eq!(title_text.text, "Instance Browser - Transformed (2 rows)");
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
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.all"),
        "All (3)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.named"),
        "Named (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.properties"),
        "Properties (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.arrays"),
        "Arrays (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.identity"),
        "Identity (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.instance_browser_filter.transformed"),
        "Transformed (2)"
    );
    assert!(
        !document.nodes().iter().any(|node| {
            node.name()
                == format!(
                    "glassworks.viewctl.layout.instance.{}",
                    layout_instance_action_key(top, named)
                )
        }),
        "transformed filter should hide identity instances"
    );

    app.layout_browser_search = "missing".to_string();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser.select_first"));
    assert!(
        app.status_message()
            .contains("Instance browser has no matching transformed instances for search missing"),
        "{}",
        app.status_message()
    );
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty instance search should build");
    let empty_title = empty_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.instance_browser.title")
        .unwrap_or_else(|| panic!("empty instance browser title should exist"));
    let UiContent::Text(empty_title_text) = empty_title.content() else {
        panic!("empty instance browser title should be text");
    };
    assert_eq!(
        empty_title_text.text,
        "Instance Browser - Transformed / search missing (0 rows)"
    );
}

#[test]
pub(crate) fn layout_instance_browser_searches_by_selector_metadata() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance selector search test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let parent_cell = app.workspace.document.create_cell("alpha parent cell");
    let alpha_target = app.workspace.document.create_cell("alpha instance target");
    let zeta_target = app.workspace.document.create_cell("zeta instance target");
    app.workspace
        .document
        .insert_shape_in_cell(
            alpha_target,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 30, 40)),
        )
        .expect("alpha target should have local geometry");
    app.workspace
        .document
        .insert_shape_in_cell(
            zeta_target,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 60, 70)),
        )
        .expect("zeta target should have local geometry");
    let parent_placement = app
        .workspace
        .document
        .insert_instance(top, parent_cell, Transform::translate(1_000, 0))
        .expect("parent cell should be placed under top");
    let alpha_instance = app
        .workspace
        .document
        .insert_instance(parent_cell, alpha_target, Transform::translate(300, 40))
        .expect("alpha target instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(parent_cell, alpha_instance)
            .expect("alpha target instance should be mutable");
        instance.name = Some("alpha_inst".to_string());
        instance.array = InstanceArray {
            columns: 2,
            rows: 1,
            column_pitch: Vector::new(40, 0),
            row_pitch: Vector::new(0, 40),
        };
        instance
            .properties
            .insert("review.owner".to_string(), "layout".to_string());
    }
    let zeta_instance = app
        .workspace
        .document
        .insert_instance(top, zeta_target, Transform::translate(700, 0))
        .expect("zeta target instance should be inserted");

    app.layout_instance_browser_scope = LayoutInstanceBrowserScope::Hierarchy;
    let listed_instances = |app: &GlassworksApp| {
        layout_instance_browser_entries_with_filter(
            app,
            LayoutInstanceBrowserFilter::All,
            true,
            usize::MAX,
        )
        .into_iter()
        .map(|(parent, id, _)| (parent, id))
        .collect::<Vec<_>>()
    };

    app.set_layout_browser_search("parent_cell=alpha parent");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search("target_cell=alpha instance target");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search(&format!("target_cell_id={}", zeta_target.0));
    assert_eq!(listed_instances(&app), vec![(top, zeta_instance)]);
    app.set_layout_browser_search("name=alpha_inst");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search("array=2x1");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search("translation=300,40");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search("target_bounds=10,20 30x40");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);
    app.set_layout_browser_search("review.owner=layout");
    assert_eq!(listed_instances(&app), vec![(parent_cell, alpha_instance)]);

    app.set_layout_browser_search("target_cell=alpha instance target");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with target-cell instance search should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.instance_browser.select_first"
        }),
        "instance browser should expose select-first control for target-cell selector search"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser.select_first"));
    assert_eq!(
        app.selected_layout_instance_context()
            .map(|(parent, id, _)| (parent, id)),
        Some((parent_cell, alpha_instance))
    );
    assert_ne!(
        app.selected_layout_instance_context()
            .map(|(parent, id, _)| (parent, id)),
        Some((top, parent_placement)),
        "selector search should select the matching nested instance, not its parent placement"
    );
}

#[test]
pub(crate) fn layout_instance_browser_filters_hidden_and_visible_target_cells() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance browser hidden filter test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let hidden_child = app.workspace.document.create_cell("hidden child");
    let visible_child = app.workspace.document.create_cell("visible child");
    let hidden_instance = app
        .workspace
        .document
        .insert_instance(top, hidden_child, Transform::translate(100, 0))
        .expect("hidden-target instance should be inserted");
    let visible_instance = app
        .workspace
        .document
        .insert_instance(top, visible_child, Transform::translate(200, 0))
        .expect("visible-target instance should be inserted");
    app.layout_hidden_cells.insert(hidden_child);

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.hidden")
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Hidden
    );
    let hidden_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(hidden_entries, vec![hidden_instance]);

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.visible")
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::Visible
    );
    let visible_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(visible_entries, vec![visible_instance]);

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.viewctl.layout.instance_browser_filter.hidden",
        "glassworks.viewctl.layout.instance_browser_filter.visible",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "instance browser filter surface should expose {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_instance_browser_filters_leaf_and_branch_target_cells() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance browser target role filter test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let leaf_target = app.workspace.document.create_cell("leaf target");
    let branch_target = app.workspace.document.create_cell("branch target");
    let nested_leaf = app.workspace.document.create_cell("nested leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            leaf_target,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 30, 40)),
        )
        .expect("leaf target should have local geometry");
    app.workspace
        .document
        .insert_shape_in_cell(
            branch_target,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 60, 70)),
        )
        .expect("branch target should have local geometry");
    app.workspace
        .document
        .insert_instance(branch_target, nested_leaf, Transform::IDENTITY)
        .expect("branch target should contain a child placement");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(top, leaf_target, Transform::translate(100, 0))
        .expect("leaf-target instance should be inserted");
    let branch_instance = app
        .workspace
        .document
        .insert_instance(top, branch_target, Transform::translate(200, 0))
        .expect("branch-target instance should be inserted");

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.instance_browser_filter.leaf_targets"
        )
    );
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::LeafTargets
    );
    let leaf_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(leaf_entries, vec![leaf_instance]);
    let leaf_rows = layout_instance_browser_rows(&app);
    assert!(
        leaf_rows
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Leaf Targets"),
        "instance browser rows should show the leaf-target filter"
    );
    for (key, value) in [
        ("Target role", "leaf"),
        ("Target shapes", "1"),
        ("Target child instances", "0"),
        ("Target local bounds", "10,20 30x40"),
    ] {
        assert!(
            leaf_rows
                .iter()
                .any(|(row_key, row_value)| row_key == key && row_value == value),
            "leaf-target instance rows should include {key}={value}: {leaf_rows:?}"
        );
    }

    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.instance_browser_filter.branch_targets"
    ));
    assert_eq!(
        app.layout_instance_browser_filter,
        LayoutInstanceBrowserFilter::BranchTargets
    );
    let branch_entries = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(branch_entries, vec![branch_instance]);
    let branch_rows = layout_instance_browser_rows(&app);
    assert!(
        branch_rows
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Branch Targets"),
        "instance browser rows should show the branch-target filter"
    );
    for (key, value) in [
        ("Target role", "branch"),
        ("Target shapes", "1"),
        ("Target child instances", "1"),
        ("Target local bounds", "100,200 60x70"),
    ] {
        assert!(
            branch_rows
                .iter()
                .any(|(row_key, row_value)| row_key == key && row_value == value),
            "branch-target instance rows should include {key}={value}: {branch_rows:?}"
        );
    }

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.viewctl.layout.instance_browser_filter.leaf_targets",
        "glassworks.viewctl.layout.instance_browser_filter.branch_targets",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "instance browser target-role filter surface should expose {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_instance_browser_select_first_uses_property_selector_search() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("instance browser select first");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let top = app.workspace.document.top_cell;
    let child = app.workspace.document.create_cell("leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
        )
        .expect("child shape should be inserted");
    let other = app
        .workspace
        .document
        .insert_instance(top, child, Transform::IDENTITY)
        .expect("other instance should be inserted");
    app.workspace
        .document
        .instance_mut(top, other)
        .expect("other instance should be mutable")
        .properties
        .insert("review.owner".to_string(), "reticle".to_string());
    let target = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(100, 0))
        .expect("target instance should be inserted");
    app.workspace
        .document
        .instance_mut(top, target)
        .expect("target instance should be mutable")
        .properties
        .insert("review.owner".to_string(), "layout".to_string());

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser_filter.properties")
    );
    app.set_layout_browser_search("review.owner=layout");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with property-filtered instance browser should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.instance_browser.select_first"
        }),
        "instance browser should expose select-first control when property matches exist"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.instance_browser.select_first"));
    assert_eq!(
        app.selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| occurrence.instance_path.first().copied()),
        Some(target)
    );
    assert_eq!(
        app.selected_layout_instance_context()
            .map(|(parent, id, _)| (parent, id)),
        Some((top, target))
    );
    assert!(
        app.status_message().contains("Selected browser instance"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_browser_search_filters_inspector_browsers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("browser search test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
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
    app.active_layer = metal1;

    let needle_cell = app.workspace.document.create_cell("needle_leaf");
    let other_cell = app.workspace.document.create_cell("other_leaf");
    let needle_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            needle_cell,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("needle cell shape should be inserted");
    let other_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            other_cell,
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("other cell shape should be inserted");
    app.workspace
        .document
        .cell_mut(needle_cell)
        .expect("needle cell should be mutable")
        .shapes
        .get_mut(&needle_shape)
        .expect("needle shape should be mutable")
        .name = Some("needle_contact".to_string());
    app.workspace
        .document
        .cell_mut(other_cell)
        .expect("other cell should be mutable")
        .shapes
        .get_mut(&other_shape)
        .expect("other shape should be mutable")
        .name = Some("plain_contact".to_string());

    let needle_instance = app
        .workspace
        .document
        .insert_instance(top, needle_cell, Transform::translate(100, 0))
        .expect("needle instance should be inserted");
    app.workspace
        .document
        .instance_mut(top, needle_instance)
        .expect("needle instance should be mutable")
        .name = Some("needle_inst".to_string());
    let other_instance = app
        .workspace
        .document
        .insert_instance(top, other_cell, Transform::translate(300, 0))
        .expect("other instance should be inserted");

    let net_shape = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(1_000, 0), 200, 120)),
            Some(NetId(42)),
            Some("needle_net_shape".to_string()),
        )
        .expect("needle net shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(1_040, 40),
            text: "needle".to_string(),
        },
    )
    .expect("needle label should be added");
    let other_top_shape = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(2_000, 0), 200, 120)),
        )
        .expect("other top shape should be added");

    app.begin_layout_browser_search();
    for character in "needle".chars() {
        assert!(app.handle_layout_browser_search_key(
            operad::KeyCode::Character(character),
            operad::KeyModifiers::NONE,
        ));
    }
    assert_eq!(app.layout_browser_search, "needle");

    let cell_ids = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert!(cell_ids.contains(&needle_cell));
    assert!(!cell_ids.contains(&other_cell));

    let shape_ids = layout_shape_browser_entries(&app)
        .into_iter()
        .map(|(occurrence, _)| occurrence.shape)
        .collect::<Vec<_>>();
    assert!(shape_ids.contains(&needle_shape));
    assert!(shape_ids.contains(&net_shape));
    assert!(!shape_ids.contains(&other_shape));
    assert!(!shape_ids.contains(&other_top_shape));

    let instance_ids = layout_instance_browser_entries(&app)
        .into_iter()
        .map(|(_, id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(instance_ids, vec![needle_instance]);
    assert!(!instance_ids.contains(&other_instance));

    let net_entries = layout_net_browser_entries(&app);
    assert_eq!(net_entries.len(), 1);
    let report = app
        .connectivity_report()
        .expect("needle connectivity should extract");
    assert!(
        report
            .component(net_entries[0].0)
            .is_some_and(|component| component.net_name.as_deref() == Some("NEEDLE"))
    );

    app.set_layout_browser_search("width");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow DRC test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    assert!(
        !layout_drc_marker_entries(&app).is_empty(),
        "DRC marker search should match rule text"
    );
    app.set_layout_browser_search("no such marker");
    assert!(
        layout_drc_marker_entries(&app).is_empty(),
        "DRC marker search should hide non-matching markers"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with browser search should build");
    for node_name in [
        "glassworks.layout.browser_search.title",
        "glassworks.viewctl.layout.browser_search.start",
        "glassworks.viewctl.layout.browser_replace.start",
        "glassworks.viewctl.layout.browser_search.clear",
        "glassworks.viewctl.layout.browser_replace.apply",
        "glassworks.layout.drc_marker_browser.empty",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "browser search surface should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_search.clear"));
    assert!(app.layout_browser_search.is_empty());
    assert!(!app.layout_browser_search_active);
}
