#![allow(unused_imports)]
use super::*;
use crate::*;
use layout_model::MarkerSignoffRecord;

#[test]
pub(crate) fn layout_browser_search_replace_updates_shape_text_and_properties() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("browser replace test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.workspace
        .document
        .layers
        .get_mut(&metal1)
        .expect("metal1 layer should be mutable")
        .name = "needle_metal1".to_string();

    let top_named = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            None,
            Some("sig_needle_a".to_string()),
        )
        .expect("named top shape should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&top_named)
        .expect("top named shape should be mutable")
        .properties
        .insert("shape.needle.kind".to_string(), "needle_value".to_string());
    let top_label = app
        .add_layout_shape(
            metal1,
            ShapeKind::Label {
                position: Point::new(20, 20),
                text: "NEEDLE_NODE".to_string(),
            },
        )
        .expect("top label should be added");
    let measurement = app
        .add_layout_shape(
            metal1,
            ShapeKind::Measurement {
                a: Point::new(0, 200),
                b: Point::new(100, 200),
                label: "needle ruler".to_string(),
                mode: MeasurementMode::Direct,
            },
        )
        .expect("measurement should be added");
    let untouched = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 0), 100, 80)),
            None,
            Some("plain_contact".to_string()),
        )
        .expect("untouched shape should be added");
    let child = app.workspace.document.create_cell("needle_leaf");
    let child_named = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 40)),
        )
        .expect("child named shape should be inserted");
    let child_label = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Label {
                position: Point::new(10, 10),
                text: "needle_label".to_string(),
            },
        )
        .expect("child label should be inserted");
    app.workspace
        .document
        .cell_mut(child)
        .expect("child cell should be mutable")
        .shapes
        .get_mut(&child_named)
        .expect("child named shape should be mutable")
        .name = Some("inner_needle".to_string());
    app.workspace
        .document
        .cell_mut(child)
        .expect("child cell should be mutable")
        .shapes
        .get_mut(&child_named)
        .expect("child named shape should be mutable")
        .properties
        .insert(
            "shape.needle.kind".to_string(),
            "child_needle_value".to_string(),
        );
    app.workspace
        .document
        .cell_mut(child)
        .expect("child cell should be mutable")
        .properties
        .insert("custom.needle.kind".to_string(), "needle_value".to_string());
    let child_instance = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(500, 0))
        .expect("child instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, child_instance)
            .expect("child instance should be mutable");
        instance.name = Some("needle_inst".to_string());
        instance.properties.insert(
            "instance.needle.kind".to_string(),
            "needle_value".to_string(),
        );
    }
    let top_shape_name = |app: &GlassworksApp, id: ShapeId| {
        app.workspace
            .document
            .shapes
            .get(&id)
            .and_then(|shape| shape.name.clone())
    };
    let top_shape_property = |app: &GlassworksApp, id: ShapeId, key: &str| {
        app.workspace
            .document
            .shapes
            .get(&id)
            .and_then(|shape| shape.properties.get(key).cloned())
    };
    let top_label_text = |app: &GlassworksApp, id: ShapeId| {
        app.workspace
            .document
            .shapes
            .get(&id)
            .and_then(|shape| match &shape.kind {
                ShapeKind::Label { text, .. } => Some(text.clone()),
                _ => None,
            })
    };
    let measurement_label = |app: &GlassworksApp, id: ShapeId| {
        app.workspace
            .document
            .shapes
            .get(&id)
            .and_then(|shape| match &shape.kind {
                ShapeKind::Measurement { label, .. } => Some(label.clone()),
                _ => None,
            })
    };
    let cell_shape_name = |app: &GlassworksApp, id: ShapeId| {
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&id))
            .and_then(|shape| shape.name.clone())
    };
    let cell_shape_property = |app: &GlassworksApp, id: ShapeId, key: &str| {
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&id))
            .and_then(|shape| shape.properties.get(key).cloned())
    };
    let cell_label_text = |app: &GlassworksApp, id: ShapeId| {
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&id))
            .and_then(|shape| match &shape.kind {
                ShapeKind::Label { text, .. } => Some(text.clone()),
                _ => None,
            })
    };
    let cell_name = |app: &GlassworksApp| {
        app.workspace
            .document
            .cell(child)
            .map(|cell| cell.name.clone())
    };
    let cell_property = |app: &GlassworksApp, key: &str| {
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.properties.get(key).cloned())
    };
    let instance_name = |app: &GlassworksApp| {
        app.workspace
            .document
            .instance(top, child_instance)
            .and_then(|instance| instance.name.clone())
    };
    let instance_property = |app: &GlassworksApp, key: &str| {
        app.workspace
            .document
            .instance(top, child_instance)
            .and_then(|instance| instance.properties.get(key).cloned())
    };
    let layer_name = |app: &GlassworksApp| {
        app.workspace
            .document
            .layer(metal1)
            .map(|layer| layer.name.clone())
    };

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_search.set.needle"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.set.bus"));
    let undo_len = app.layout_undo.len();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.apply"));
    assert_eq!(app.layout_undo.len(), undo_len + 1);
    assert!(!app.layout_browser_search_active);
    assert!(!app.layout_browser_replace_active);

    assert_eq!(
        top_shape_name(&app, top_named).as_deref(),
        Some("sig_bus_a")
    );
    assert_eq!(
        top_shape_property(&app, top_named, "shape.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert!(top_shape_property(&app, top_named, "shape.needle.kind").is_none());
    assert_eq!(top_label_text(&app, top_label).as_deref(), Some("bus_NODE"));
    assert_eq!(
        top_shape_name(&app, untouched).as_deref(),
        Some("plain_contact")
    );
    assert_eq!(
        measurement_label(&app, measurement).as_deref(),
        Some("bus ruler")
    );
    assert_eq!(
        cell_shape_name(&app, child_named).as_deref(),
        Some("inner_bus")
    );
    assert_eq!(
        cell_shape_property(&app, child_named, "shape.bus.kind").as_deref(),
        Some("child_bus_value")
    );
    assert!(cell_shape_property(&app, child_named, "shape.needle.kind").is_none());
    assert_eq!(
        cell_label_text(&app, child_label).as_deref(),
        Some("bus_label")
    );
    assert_eq!(cell_name(&app).as_deref(), Some("bus_leaf"));
    assert_eq!(
        cell_property(&app, "custom.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert!(cell_property(&app, "custom.needle.kind").is_none());
    assert_eq!(instance_name(&app).as_deref(), Some("bus_inst"));
    assert_eq!(
        instance_property(&app, "instance.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert!(instance_property(&app, "instance.needle.kind").is_none());
    assert_eq!(layer_name(&app).as_deref(), Some("bus_metal1"));

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with browser replace should build");
    for node_name in [
        "glassworks.viewctl.layout.browser_replace.start",
        "glassworks.viewctl.layout.browser_replace.apply",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "browser replace surface should expose {node_name}"
        );
    }

    assert!(app.undo_layout_operation());
    assert_eq!(
        top_shape_name(&app, top_named).as_deref(),
        Some("sig_needle_a")
    );
    assert_eq!(
        top_shape_property(&app, top_named, "shape.needle.kind").as_deref(),
        Some("needle_value")
    );
    assert!(top_shape_property(&app, top_named, "shape.bus.kind").is_none());
    assert_eq!(
        top_label_text(&app, top_label).as_deref(),
        Some("NEEDLE_NODE")
    );
    assert_eq!(
        measurement_label(&app, measurement).as_deref(),
        Some("needle ruler")
    );
    assert_eq!(
        cell_shape_name(&app, child_named).as_deref(),
        Some("inner_needle")
    );
    assert_eq!(
        cell_shape_property(&app, child_named, "shape.needle.kind").as_deref(),
        Some("child_needle_value")
    );
    assert!(cell_shape_property(&app, child_named, "shape.bus.kind").is_none());
    assert_eq!(
        cell_label_text(&app, child_label).as_deref(),
        Some("needle_label")
    );
    assert_eq!(cell_name(&app).as_deref(), Some("needle_leaf"));
    assert_eq!(
        cell_property(&app, "custom.needle.kind").as_deref(),
        Some("needle_value")
    );
    assert!(cell_property(&app, "custom.bus.kind").is_none());
    assert_eq!(instance_name(&app).as_deref(), Some("needle_inst"));
    assert_eq!(
        instance_property(&app, "instance.needle.kind").as_deref(),
        Some("needle_value")
    );
    assert!(instance_property(&app, "instance.bus.kind").is_none());
    assert_eq!(layer_name(&app).as_deref(), Some("needle_metal1"));

    assert!(app.redo_layout_operation());
    assert_eq!(
        top_shape_name(&app, top_named).as_deref(),
        Some("sig_bus_a")
    );
    assert_eq!(
        top_shape_property(&app, top_named, "shape.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert_eq!(
        measurement_label(&app, measurement).as_deref(),
        Some("bus ruler")
    );
    assert_eq!(
        cell_shape_name(&app, child_named).as_deref(),
        Some("inner_bus")
    );
    assert_eq!(
        cell_shape_property(&app, child_named, "shape.bus.kind").as_deref(),
        Some("child_bus_value")
    );
    assert_eq!(cell_name(&app).as_deref(), Some("bus_leaf"));
    assert_eq!(
        cell_property(&app, "custom.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert_eq!(instance_name(&app).as_deref(), Some("bus_inst"));
    assert_eq!(
        instance_property(&app, "instance.bus.kind").as_deref(),
        Some("bus_value")
    );
    assert_eq!(layer_name(&app).as_deref(), Some("bus_metal1"));
}

#[test]
pub(crate) fn layout_browser_replace_updates_active_drc_marker_review_metadata() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("browser replace marker metadata test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let shape = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 40)),
        )
        .expect("marker source shape should be added");
    let violation = DrcViolation {
        id: 1,
        rule: "min_width".to_string(),
        message: "review marker".to_string(),
        shape_ids: vec![shape],
        occurrence_ids: vec![ShapeOccurrenceId::top_level(shape)],
        bounds: Rect::from_min_size(Point::new(0, 0), 40, 40),
        required: 100,
        actual: 40.0,
    };
    let marker_key = violation.stable_key();
    *app.drc_report_cache.get_mut() = Some(DrcReportCacheEntry {
        revision: app.layout_revision,
        value: DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![violation],
        },
    });
    app.workspace.document.marker_states.insert(
        marker_key.clone(),
        MarkerState {
            waived: true,
            note: Some("needle note".to_string()),
            owner: Some("needle owner".to_string()),
            signoff: Some("needs needle signoff".to_string()),
            signoff_by: Some("needle signer".to_string()),
            signoff_note: Some("needle signoff note".to_string()),
            signoff_records: std::collections::BTreeMap::from([(
                "needle signer".to_string(),
                MarkerSignoffRecord {
                    status: "needs needle signoff".to_string(),
                    role: Some("needle role".to_string()),
                    by: Some("needle signer".to_string()),
                    note: Some("needle signoff note".to_string()),
                    recorded_at: Some("needle review timestamp".to_string()),
                },
            )]),
            tags: std::collections::BTreeMap::from([
                ("review.needle".to_string(), "needle value".to_string()),
                ("keep".to_string(), "stable".to_string()),
            ]),
            ..Default::default()
        },
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_search.set.needle"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.set.bus"));
    let revision = app.layout_revision;
    let undo_len = app.layout_undo.len();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.apply"));
    assert_eq!(
        app.layout_revision, revision,
        "marker metadata replace should not invalidate layout geometry"
    );
    assert_eq!(app.layout_undo.len(), undo_len + 1);
    assert!(
        app.drc_report().is_some(),
        "marker metadata replace should keep the active DRC report available"
    );
    let state = app
        .workspace
        .document
        .marker_states
        .get(&marker_key)
        .expect("marker state should remain after replace");
    assert!(state.waived);
    assert_eq!(state.note.as_deref(), Some("bus note"));
    assert_eq!(state.owner.as_deref(), Some("bus owner"));
    assert_eq!(state.signoff.as_deref(), Some("needs bus signoff"));
    assert_eq!(state.signoff_by.as_deref(), Some("bus signer"));
    assert_eq!(state.signoff_note.as_deref(), Some("bus signoff note"));
    let signoff = state.signoff_records.get("bus signer").unwrap();
    assert_eq!(signoff.status, "needs bus signoff");
    assert_eq!(signoff.role.as_deref(), Some("bus role"));
    assert_eq!(signoff.by.as_deref(), Some("bus signer"));
    assert_eq!(signoff.note.as_deref(), Some("bus signoff note"));
    assert_eq!(signoff.recorded_at.as_deref(), Some("bus review timestamp"));
    assert_eq!(
        state.tags.get("review.bus").map(String::as_str),
        Some("bus value")
    );
    assert!(state.tags.get("review.needle").is_none());
    assert_eq!(state.tags.get("keep").map(String::as_str), Some("stable"));

    assert!(app.undo_layout_operation());
    let state = app
        .workspace
        .document
        .marker_states
        .get(&marker_key)
        .expect("undo should restore marker state");
    assert_eq!(state.note.as_deref(), Some("needle note"));
    assert_eq!(state.owner.as_deref(), Some("needle owner"));
    assert_eq!(state.signoff.as_deref(), Some("needs needle signoff"));
    assert_eq!(state.signoff_by.as_deref(), Some("needle signer"));
    assert_eq!(state.signoff_note.as_deref(), Some("needle signoff note"));
    let signoff = state.signoff_records.get("needle signer").unwrap();
    assert_eq!(signoff.role.as_deref(), Some("needle role"));
    assert_eq!(
        signoff.recorded_at.as_deref(),
        Some("needle review timestamp")
    );
    assert_eq!(
        state.tags.get("review.needle").map(String::as_str),
        Some("needle value")
    );

    assert!(app.redo_layout_operation());
    let state = app
        .workspace
        .document
        .marker_states
        .get(&marker_key)
        .expect("redo should reapply marker state replacement");
    assert_eq!(state.note.as_deref(), Some("bus note"));
    assert_eq!(state.signoff_by.as_deref(), Some("bus signer"));
    assert_eq!(state.signoff_note.as_deref(), Some("bus signoff note"));
    let signoff = state.signoff_records.get("bus signer").unwrap();
    assert_eq!(signoff.role.as_deref(), Some("bus role"));
    assert_eq!(signoff.recorded_at.as_deref(), Some("bus review timestamp"));
    assert_eq!(
        state.tags.get("review.bus").map(String::as_str),
        Some("bus value")
    );
}

#[test]
pub(crate) fn layout_browser_replace_scope_limits_current_and_visible_hierarchy() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("browser replace scope test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;

    let top_shape = app
        .add_layout_shape_with_metadata(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            None,
            Some("top_needle".to_string()),
        )
        .expect("top shape should be added");
    let child = app.workspace.document.create_cell("child_needle");
    let child_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 40)),
        )
        .expect("child shape should be inserted");
    app.workspace
        .document
        .cell_mut(child)
        .expect("child cell should be mutable")
        .shapes
        .get_mut(&child_shape)
        .expect("child shape should be mutable")
        .name = Some("child_needle_shape".to_string());
    let child_instance = app
        .workspace
        .document
        .insert_instance(top, child, Transform::translate(200, 0))
        .expect("child instance should be inserted");
    app.workspace
        .document
        .instance_mut(top, child_instance)
        .expect("child instance should be mutable")
        .name = Some("child_needle_inst".to_string());

    let sibling = app.workspace.document.create_cell("sibling_needle");
    let sibling_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            sibling,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
        )
        .expect("sibling shape should be inserted");
    app.workspace
        .document
        .cell_mut(sibling)
        .expect("sibling cell should be mutable")
        .shapes
        .get_mut(&sibling_shape)
        .expect("sibling shape should be mutable")
        .name = Some("sibling_needle_shape".to_string());
    let sibling_instance = app
        .workspace
        .document
        .insert_instance(top, sibling, Transform::translate(400, 0))
        .expect("sibling instance should be inserted");
    app.workspace
        .document
        .instance_mut(top, sibling_instance)
        .expect("sibling instance should be mutable")
        .name = Some("sibling_needle_inst".to_string());

    let top_shape_name = |app: &GlassworksApp| {
        app.workspace
            .document
            .shapes
            .get(&top_shape)
            .and_then(|shape| shape.name.clone())
    };
    let cell_name = |app: &GlassworksApp, cell: CellId| {
        app.workspace
            .document
            .cell(cell)
            .map(|cell| cell.name.clone())
    };
    let cell_shape_name = |app: &GlassworksApp, cell: CellId, shape: ShapeId| {
        app.workspace
            .document
            .cell(cell)
            .and_then(|cell| cell.shapes.get(&shape))
            .and_then(|shape| shape.name.clone())
    };
    let instance_name = |app: &GlassworksApp, instance: InstanceId| {
        app.workspace
            .document
            .instance(top, instance)
            .and_then(|instance| instance.name.clone())
    };

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_search.set.needle"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.set.bus"));
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace_scope.current_cell")
    );
    assert_eq!(
        app.layout_browser_replace_scope,
        LayoutBrowserReplaceScope::CurrentCell
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.apply"));

    assert_eq!(top_shape_name(&app).as_deref(), Some("top_bus"));
    assert_eq!(
        instance_name(&app, child_instance).as_deref(),
        Some("child_bus_inst")
    );
    assert_eq!(cell_name(&app, child).as_deref(), Some("child_needle"));
    assert_eq!(
        cell_shape_name(&app, child, child_shape).as_deref(),
        Some("child_needle_shape")
    );

    assert!(app.undo_layout_operation());
    app.layout_hidden_cells.insert(sibling);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.one"));
    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.browser_replace_scope.visible_hierarchy"
    ));
    assert_eq!(
        app.layout_browser_replace_scope,
        LayoutBrowserReplaceScope::VisibleHierarchy
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_replace.apply"));

    assert_eq!(top_shape_name(&app).as_deref(), Some("top_bus"));
    assert_eq!(cell_name(&app, child).as_deref(), Some("child_bus"));
    assert_eq!(
        cell_shape_name(&app, child, child_shape).as_deref(),
        Some("child_bus_shape")
    );
    assert_eq!(
        instance_name(&app, child_instance).as_deref(),
        Some("child_bus_inst")
    );
    assert_eq!(cell_name(&app, sibling).as_deref(), Some("sibling_needle"));
    assert_eq!(
        cell_shape_name(&app, sibling, sibling_shape).as_deref(),
        Some("sibling_needle_shape")
    );
    assert_eq!(
        instance_name(&app, sibling_instance).as_deref(),
        Some("sibling_needle_inst")
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with browser replace scope should build");
    for node_name in [
        "glassworks.viewctl.layout.browser_replace_scope.document",
        "glassworks.viewctl.layout.browser_replace_scope.current_cell",
        "glassworks.viewctl.layout.browser_replace_scope.visible_hierarchy",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "browser replace scope surface should expose {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_browser_column_presets_filter_property_rows() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("browser column test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;

    let child = app.workspace.document.create_cell("column_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("child shape should be inserted");
    app.workspace
        .document
        .cell_mut(child)
        .expect("child cell should be mutable")
        .properties
        .insert(
            "custom.browser.kind".to_string(),
            "alignment_guide".to_string(),
        );
    app.workspace
        .document
        .insert_instance(top, child, Transform::translate(100, 0))
        .expect("child instance should be inserted");
    app.add_layout_shape_with_metadata(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(1_000, 0), 200, 120)),
        Some(NetId(9)),
        Some("column_net_shape".to_string()),
    )
    .expect("net shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(1_040, 40),
            text: "COLUMN_NET".to_string(),
        },
    )
    .expect("net label should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow DRC test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_columns.geometry"));
    assert_eq!(app.layout_browser_columns, LayoutBrowserColumnSet::Geometry);
    let shape_rows = layout_shape_browser_rows(&app);
    assert!(shape_rows.iter().any(|(key, _)| key == "Bounds"));
    assert!(!shape_rows.iter().any(|(key, _)| key == "Net"));
    let instance_rows = layout_instance_browser_rows(&app);
    assert!(instance_rows.iter().any(|(key, _)| key == "Matrix"));
    assert!(!instance_rows.iter().any(|(key, _)| key == "Parent cell"));
    let cell_rows = layout_cell_browser_rows(&app);
    assert!(cell_rows.iter().any(|(key, _)| key == "Bounds"));
    assert!(!cell_rows.iter().any(|(key, _)| key == "Parents"));
    let net_rows = layout_net_browser_rows(&app);
    assert!(net_rows.iter().any(|(key, _)| key == "Bounds"));
    assert!(!net_rows.iter().any(|(key, _)| key == "Labels"));
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(marker_rows.iter().any(|(key, _)| key == "Required"));
    assert!(!marker_rows.iter().any(|(key, _)| key == "Shapes"));
    let child_cell = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should still exist");
    let child_property_rows = layout_cell_property_rows(&app, child_cell);
    assert!(
        child_property_rows.iter().any(|(key, value)| {
            key == "Property custom.browser.kind" && value == "alignment_guide"
        }),
        "cell property rows should expose stored key/value properties"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.browser_columns.relations"));
    assert_eq!(
        app.layout_browser_columns,
        LayoutBrowserColumnSet::Relations
    );
    let shape_rows = layout_shape_browser_rows(&app);
    assert!(shape_rows.iter().any(|(key, _)| key == "Net"));
    assert!(!shape_rows.iter().any(|(key, _)| key == "Bounds"));
    let instance_rows = layout_instance_browser_rows(&app);
    assert!(instance_rows.iter().any(|(key, _)| key == "Parent cell"));
    assert!(!instance_rows.iter().any(|(key, _)| key == "Matrix"));
    let cell_rows = layout_cell_browser_rows(&app);
    assert!(cell_rows.iter().any(|(key, _)| key == "Parents"));
    assert!(!cell_rows.iter().any(|(key, _)| key == "Bounds"));
    let net_rows = layout_net_browser_rows(&app);
    assert!(net_rows.iter().any(|(key, _)| key == "Labels"));
    assert!(!net_rows.iter().any(|(key, _)| key == "Bounds"));
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(marker_rows.iter().any(|(key, _)| key == "Shapes"));
    assert!(!marker_rows.iter().any(|(key, _)| key == "Required"));

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with browser columns should build");
    for node_name in [
        "glassworks.viewctl.layout.browser_columns.summary",
        "glassworks.viewctl.layout.browser_columns.geometry",
        "glassworks.viewctl.layout.browser_columns.relations",
        "glassworks.viewctl.layout.browser_columns.all",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "browser column preset control should exist: {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_flatten_instance_replaces_top_instance_with_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("flatten_me");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 40)),
        )
        .expect("test child shape should be inserted");
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(120, 80))
        .expect("test child instance should be inserted");
    let top = app.workspace.document.top_cell;
    let shape_count = app.workspace.document.shapes.len();

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(top, instance_id)
    )));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.flatten_instance"));
    assert!(
        app.workspace.document.instance(top, instance_id).is_none(),
        "flatten should remove the selected top-level instance"
    );
    assert_eq!(app.workspace.document.shapes.len(), shape_count + 1);
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened replacement shape should be selected");
    let flattened = app
        .workspace()
        .document
        .shapes
        .get(&flattened_id)
        .expect("flattened shape should exist");
    assert_eq!(
        flattened.kind.bounds(),
        Rect::from_min_size(Point::new(120, 80), 40, 40)
    );
    assert!(app.status_message().contains("Flattened instance"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.instance(top, instance_id).is_some(),
        "undo should restore the instance"
    );
    assert!(
        !app.workspace.document.shapes.contains_key(&flattened_id),
        "undo should remove flattened replacement geometry"
    );
}

#[test]
pub(crate) fn layout_flatten_instance_replaces_current_cell_instance_with_local_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("flatten_mid");
    let leaf = app.workspace.document.create_cell("flatten_leaf");
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 20)),
        )
        .expect("leaf shape should be inserted");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(30, 40))
        .expect("leaf instance should be inserted");
    let top = app.workspace.document.top_cell;
    app.workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 2_000))
        .expect("mid instance should be inserted");
    let before_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == leaf_shape)
        .expect("flattened leaf shape should be visible")
        .bounds;

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(mid, leaf_instance)
    )));
    assert_eq!(app.layout_view_top_cell, mid);
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.flatten_instance"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after flattening");
    assert!(
        mid_cell.instances.get(&leaf_instance).is_none(),
        "flatten should remove the selected current-cell instance"
    );
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened replacement shape should be selected");
    let flattened = mid_cell
        .shapes
        .get(&flattened_id)
        .expect("flattened shape should be stored in the current cell");
    assert_eq!(
        flattened.kind.bounds(),
        Rect::from_min_size(Point::new(30, 40), 40, 20)
    );
    let after_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == flattened_id)
        .expect("flattened replacement should be visible from the document top")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "flattened current-cell geometry should preserve placed document geometry"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after undo");
    assert!(mid_cell.instances.get(&leaf_instance).is_some());
    assert!(mid_cell.shapes.get(&flattened_id).is_none());
}

#[test]
pub(crate) fn layout_make_instance_variant_retargets_only_selected_instance_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("variant_source");
    let source_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 20)),
        )
        .expect("source shape should be inserted");
    let top = app.workspace.document.top_cell;
    let first_instance = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(100, 200))
        .expect("first instance should be inserted");
    let second_instance = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(300, 200))
        .expect("second instance should be inserted");
    let before_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(
            top,
            &ShapeOccurrenceId::from_instance_path(source_shape, &[first_instance]),
        )
        .expect("first instance shape should be visible")
        .bounds;

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(top, first_instance)
    )));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with selected instance context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.make_cell_variant"),
        "hierarchy context should expose make-variant action"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.make_cell_variant"));
    let variant = app
        .workspace
        .document
        .instance(top, first_instance)
        .expect("first instance should remain")
        .cell;
    assert_ne!(variant, child);
    assert_eq!(
        app.workspace
            .document
            .instance(top, second_instance)
            .expect("second instance should remain")
            .cell,
        child,
        "making a variant should not retarget sibling instances"
    );
    let variant_cell = app
        .workspace
        .document
        .cell(variant)
        .expect("variant cell should exist");
    assert_eq!(variant_cell.shapes.values().count(), 1);
    assert!(
        variant_cell.shapes.get(&source_shape).is_none(),
        "variant should copy local shapes with fresh shape ids"
    );
    let occurrence =
        first_layout_occurrence_for_instance(&app.workspace.document, top, first_instance)
            .expect("variant instance should expose copied shape geometry");
    let after_view = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(top, &occurrence)
        .expect("variant occurrence should resolve");
    assert_eq!(
        after_view.bounds, before_bounds,
        "variant retargeting should preserve selected instance placement"
    );
    assert_ne!(occurrence.source_shape_id(), source_shape);
    assert!(app.status_message().contains("Made cell variant"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .instance(top, first_instance)
            .expect("undo should restore first instance")
            .cell,
        child
    );
    assert!(
        app.workspace.document.cell(variant).is_none(),
        "undo should delete the unreferenced variant cell"
    );
}

#[test]
pub(crate) fn layout_make_nested_instance_variant_retargets_deep_selected_instance_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("nested_variant_mid");
    let leaf = app.workspace.document.create_cell("nested_variant_leaf");
    let source_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 20)),
        )
        .expect("source shape should be inserted");
    let selected_leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(70, 90))
        .expect("selected leaf instance should be inserted");
    let sibling_leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(170, 90))
        .expect("sibling leaf instance should be inserted");
    let top = app.workspace.document.top_cell;
    let mid_instance = app
        .workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 2_000))
        .expect("mid instance should be inserted");
    let selected_occurrence = ShapeOccurrenceId::from_instance_path(
        source_shape,
        &[mid_instance, selected_leaf_instance],
    );
    let before_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(top, &selected_occurrence)
        .expect("nested selected instance shape should be visible")
        .bounds;

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.occurrence.{}",
        layout_occurrence_action_key(&selected_occurrence)
    )));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with nested variant context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.make_cell_variant"),
        "hierarchy context should expose make-variant for nested selected instances"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.make_cell_variant"));
    assert_eq!(
        app.layout_view_top_cell, mid,
        "making a nested variant should switch to the edited parent cell"
    );
    let variant = app
        .workspace
        .document
        .instance(mid, selected_leaf_instance)
        .expect("selected leaf instance should remain")
        .cell;
    assert_ne!(variant, leaf);
    assert_eq!(
        app.workspace
            .document
            .instance(mid, sibling_leaf_instance)
            .expect("sibling leaf instance should remain")
            .cell,
        leaf,
        "making a nested variant should not retarget sibling child instances"
    );
    let variant_cell = app
        .workspace
        .document
        .cell(variant)
        .expect("variant cell should exist");
    assert_eq!(variant_cell.shapes.values().count(), 1);
    assert!(
        variant_cell.shapes.get(&source_shape).is_none(),
        "nested variant should copy local shapes with fresh shape ids"
    );
    let copied_occurrence =
        first_layout_occurrence_for_instance(&app.workspace.document, mid, selected_leaf_instance)
            .expect("variant instance should expose copied shape geometry");
    let top_occurrence = ShapeOccurrenceId::from_instance_path(
        copied_occurrence.source_shape_id(),
        &[mid_instance, selected_leaf_instance],
    );
    let after_view = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(top, &top_occurrence)
        .expect("nested variant occurrence should resolve from the document top");
    assert_eq!(
        after_view.bounds, before_bounds,
        "nested variant retargeting should preserve selected instance placement"
    );
    assert_ne!(copied_occurrence.source_shape_id(), source_shape);
    assert!(app.status_message().contains("Made cell variant"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace
            .document
            .instance(mid, selected_leaf_instance)
            .expect("undo should restore selected instance")
            .cell,
        leaf
    );
    assert!(
        app.workspace.document.cell(variant).is_none(),
        "undo should delete the unreferenced nested variant cell"
    );
}

#[test]
pub(crate) fn layout_flatten_current_cell_replaces_child_instances_with_local_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("mid");
    let leaf = app.workspace.document.create_cell("leaf");
    let local_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            mid,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-50, -50), 10, 10)),
        )
        .expect("mid local shape should be inserted");
    app.workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
        )
        .expect("leaf shape should be inserted");
    let nested_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(30, 40))
        .expect("nested instance should be inserted");
    let top = app.workspace.document.top_cell;
    let top_instance = app
        .workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 0))
        .expect("top instance should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", mid.0)));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.flatten_current_cell"),
        "hierarchy context should expose current-cell flattening"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.flatten_current_cell"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after flattening");
    assert!(mid_cell.instances.get(&nested_instance).is_none());
    assert!(mid_cell.shapes.get(&local_shape).is_some());
    assert!(
        app.workspace.document.instance(top, top_instance).is_some(),
        "flattening the current cell should not remove references to that cell"
    );
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened local shape should be selected");
    let flattened = mid_cell
        .shapes
        .get(&flattened_id)
        .expect("flattened shape should be stored in the current cell");
    assert_eq!(
        flattened.kind.bounds(),
        Rect::from_min_size(Point::new(30, 40), 20, 20)
    );
    assert!(app.status_message().contains("Flattened cell mid"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after undo");
    assert!(mid_cell.instances.get(&nested_instance).is_some());
    assert!(mid_cell.shapes.get(&local_shape).is_some());
    assert!(mid_cell.shapes.get(&flattened_id).is_none());
}

#[test]
pub(crate) fn layout_flatten_current_cell_one_level_promotes_child_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("one_level_mid");
    let child = app.workspace.document.create_cell("one_level_child");
    let leaf = app.workspace.document.create_cell("one_level_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 10)),
        )
        .expect("child local shape should be inserted");
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 6, 6)),
        )
        .expect("leaf shape should be inserted");
    let child_leaf_instance = app
        .workspace
        .document
        .insert_instance(child, leaf, Transform::translate(5, 7))
        .expect("child leaf instance should be inserted");
    let mid_child_instance = app
        .workspace
        .document
        .insert_instance(mid, child, Transform::translate(30, 40))
        .expect("mid child instance should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", mid.0)));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.flatten_current_cell_one"),
        "hierarchy context should expose one-level cell flattening"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.flatten_current_cell_one"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after one-level flattening");
    assert!(mid_cell.instances.get(&mid_child_instance).is_none());
    assert_eq!(mid_cell.shapes.values().count(), 1);
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened child local shape should be selected");
    assert_eq!(
        mid_cell
            .shapes
            .get(&flattened_id)
            .expect("flattened child local shape should be stored in the current cell")
            .kind
            .bounds(),
        Rect::from_min_size(Point::new(30, 40), 20, 10)
    );
    let promoted = mid_cell
        .instances
        .values()
        .find(|instance| instance.cell == leaf)
        .expect("nested child instance should be promoted into the current cell");
    assert_ne!(promoted.id, child_leaf_instance);
    let promoted_view = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(
            mid,
            &ShapeOccurrenceId::from_instance_path(leaf_shape, &[promoted.id]),
        )
        .expect("promoted leaf geometry should remain visible");
    assert_eq!(
        promoted_view.bounds,
        Rect::from_min_size(Point::new(35, 47), 6, 6)
    );
    assert!(app.status_message().contains("one level"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after undo");
    assert!(mid_cell.instances.get(&mid_child_instance).is_some());
    assert!(mid_cell.shapes.get(&flattened_id).is_none());
    assert!(
        mid_cell
            .instances
            .values()
            .all(|instance| instance.cell != leaf),
        "undo should remove promoted nested instances"
    );
}

#[test]
pub(crate) fn layout_flatten_selected_instance_one_level_promotes_nested_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app
        .workspace
        .document
        .create_cell("selected_one_level_child");
    let leaf = app
        .workspace
        .document
        .create_cell("selected_one_level_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
        )
        .expect("child local shape should be inserted");
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 4, 4)),
        )
        .expect("leaf shape should be inserted");
    app.workspace
        .document
        .insert_instance(child, leaf, Transform::translate(3, 4))
        .expect("nested leaf instance should be inserted");
    let top = app.workspace.document.top_cell;
    let selected_instance = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(100, 200))
        .expect("top child instance should be inserted");

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(top, selected_instance)
    )));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with selected instance context");
    assert!(
        hierarchy_document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.flatten_selected_instance_one"
        }),
        "hierarchy context should expose one-level selected-instance flattening"
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.edit"));
    let menu_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("edit menu should build with hierarchy flatten actions");
    for node_name in [
        "glassworks.menu.item.edit.flatten_instance_one",
        "glassworks.menu.item.edit.flatten_cell_one",
    ] {
        assert!(
            menu_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "Edit menu should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.flatten_selected_instance_one"));
    assert!(
        app.workspace
            .document
            .instance(top, selected_instance)
            .is_none(),
        "one-level flatten should remove the selected instance"
    );
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened child local shape should be selected");
    assert_eq!(
        app.workspace
            .document
            .shapes
            .get(&flattened_id)
            .expect("flattened top-level shape should exist")
            .kind
            .bounds(),
        Rect::from_min_size(Point::new(100, 200), 10, 10)
    );
    let promoted = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .values()
        .find(|instance| instance.cell == leaf)
        .expect("nested leaf instance should be promoted into the top cell");
    let promoted_view = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(
            top,
            &ShapeOccurrenceId::from_instance_path(leaf_shape, &[promoted.id]),
        )
        .expect("promoted leaf geometry should remain visible");
    assert_eq!(
        promoted_view.bounds,
        Rect::from_min_size(Point::new(103, 204), 4, 4)
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .instance(top, selected_instance)
            .is_some(),
        "undo should restore the selected instance"
    );
    assert!(!app.workspace.document.shapes.contains_key(&flattened_id));
    assert!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should exist after undo")
            .instances
            .values()
            .all(|instance| instance.cell != leaf),
        "undo should remove promoted top-level instances"
    );
}

#[test]
pub(crate) fn layout_flatten_selected_nested_occurrence_replaces_deep_instance() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("nested_flatten_mid");
    let leaf = app.workspace.document.create_cell("nested_flatten_leaf");
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 12, 16)),
        )
        .expect("leaf shape should be inserted");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(30, 40))
        .expect("leaf instance should be inserted");
    let top = app.workspace.document.top_cell;
    let mid_instance = app
        .workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 2_000))
        .expect("mid instance should be inserted");
    let occurrence =
        ShapeOccurrenceId::from_instance_path(leaf_shape, &[mid_instance, leaf_instance]);
    let before_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(top, &occurrence)
        .expect("nested occurrence should be visible from the document top")
        .bounds;

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.occurrence.{}",
        layout_occurrence_action_key(&occurrence)
    )));
    assert_eq!(app.selected_layout_occurrence, Some(occurrence));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with deep selected-instance flatten action");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.flatten_selected_instance"),
        "hierarchy context should expose deep selected-instance flattening"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.flatten_selected_instance"));
    assert_eq!(
        app.layout_view_top_cell, mid,
        "flattening a nested occurrence should switch to the edited parent cell"
    );
    assert!(
        app.workspace
            .document
            .instance(mid, leaf_instance)
            .is_none(),
        "flatten should remove the deep selected instance"
    );
    assert!(
        app.workspace.document.instance(top, mid_instance).is_some(),
        "flattening the deep instance should keep its ancestor placement"
    );
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened nested replacement should be selected");
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after flattening");
    assert_eq!(
        mid_cell
            .shapes
            .get(&flattened_id)
            .expect("flattened replacement should be stored in the deep parent")
            .kind
            .bounds(),
        Rect::from_min_size(Point::new(30, 40), 12, 16)
    );
    let after_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(
            top,
            &ShapeOccurrenceId::from_instance_path(flattened_id, &[mid_instance]),
        )
        .expect("flattened replacement should remain visible through the ancestor instance")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "nested flatten should preserve document-top placed geometry"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after undo");
    assert!(mid_cell.instances.get(&leaf_instance).is_some());
    assert!(mid_cell.shapes.get(&flattened_id).is_none());
}

#[test]
pub(crate) fn layout_flatten_current_top_cell_creates_top_level_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("top_flatten_child");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 40, 20)),
        )
        .expect("child shape should be inserted");
    let instance = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(70, 90))
        .expect("top instance should be inserted");
    let top = app.workspace.document.top_cell;
    let top_shape_count = app.workspace.document.shapes.len();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.flatten_current_cell"));
    assert!(app.workspace.document.instance(top, instance).is_none());
    assert_eq!(app.workspace.document.shapes.len(), top_shape_count + 1);
    let flattened_id = app
        .selected_layout_shape()
        .expect("flattened top-level shape should be selected");
    let flattened = app
        .workspace
        .document
        .shapes
        .get(&flattened_id)
        .expect("flattened shape should be top-level");
    assert_eq!(
        flattened.kind.bounds(),
        Rect::from_min_size(Point::new(70, 90), 40, 20)
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.instance(top, instance).is_some());
    assert!(!app.workspace.document.shapes.contains_key(&flattened_id));
}

#[test]
pub(crate) fn layout_cell_origin_to_selection_preserves_parent_placement() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("origin_child");
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 30, 40)),
        )
        .expect("child shape should be inserted");
    let top = app.workspace.document.top_cell;
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
        .expect("top instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, instance_id)
            .expect("top instance should be mutable");
        instance.transform = Transform::translate(1_000, 2_000).compose(Transform::rotate_cw90());
    }
    let before_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should be visible")
        .bounds;

    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", child.0))
    );
    app.selected_layout_shape = Some(shape_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.cell_origin.selection"),
        "hierarchy context should expose origin-to-selection"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_origin.selection"));
    let shifted_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("shifted child shape should remain in the child cell");
    assert_eq!(
        shifted_shape.kind.bounds(),
        Rect::from_min_size(Point::ZERO, 30, 40)
    );
    let after_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should remain visible")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "parent instance compensation should preserve placed geometry"
    );
    assert!(app.status_message().contains("Set origin_child origin"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("undo should restore the child shape");
    assert_eq!(
        restored_shape.kind.bounds(),
        Rect::from_min_size(Point::new(100, 200), 30, 40)
    );
}

#[test]
pub(crate) fn layout_cell_origin_nudge_preserves_parent_placement_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("origin_nudge_child");
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 30, 40)),
        )
        .expect("child shape should be inserted");
    let top = app.workspace.document.top_cell;
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
        .expect("top instance should be inserted");
    let before_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should be visible")
        .bounds;
    let step = app.layout_size_step();

    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", child.0))
    );
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.cell_origin.x_pos"),
        "hierarchy context should expose origin nudges"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_origin.x_pos"));
    let shifted_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("shifted child shape should remain in the child cell");
    assert_eq!(
        shifted_shape.kind.bounds(),
        Rect::from_min_size(Point::new(100 - step, 200), 30, 40)
    );
    let after_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should remain visible")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "parent instance compensation should preserve placed geometry"
    );
    assert!(
        app.status_message()
            .contains("Set origin_nudge_child origin"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("undo should restore the child shape");
    assert_eq!(
        restored_shape.kind.bounds(),
        Rect::from_min_size(Point::new(100, 200), 30, 40)
    );
}

#[test]
pub(crate) fn layout_cell_origin_exact_from_search_preserves_parent_placement_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("origin_exact_child");
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 200), 30, 40)),
        )
        .expect("child shape should be inserted");
    let top = app.workspace.document.top_cell;
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
        .expect("top instance should be inserted");
    let before_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should be visible")
        .bounds;

    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", child.0))
    );
    app.set_layout_browser_search("35, 45");
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.cell_origin.exact"),
        "hierarchy context should expose exact origin action"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_origin.exact"));
    let shifted_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("shifted child shape should remain in the child cell");
    assert_eq!(
        shifted_shape.kind.bounds(),
        Rect::from_min_size(Point::new(65, 155), 30, 40),
        "exact origin should use the typed DBU values without grid snapping"
    );
    let after_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should remain visible")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "parent instance compensation should preserve placed geometry"
    );
    assert!(
        app.status_message().contains("exact 35,45"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_shape = app
        .workspace
        .document
        .cell(child)
        .and_then(|cell| cell.shapes.get(&shape_id))
        .expect("undo should restore the child shape");
    assert_eq!(
        restored_shape.kind.bounds(),
        Rect::from_min_size(Point::new(100, 200), 30, 40)
    );
}

#[test]
pub(crate) fn layout_top_cell_origin_to_selection_moves_top_level_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let shape_id = app.workspace.document.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(70, 90), 20, 30)),
    );
    app.selected_layout_shape = Some(shape_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.cell_origin_selection"));
    let shifted = app
        .workspace
        .document
        .shapes
        .get(&shape_id)
        .expect("top shape should remain");
    assert_eq!(
        shifted.kind.bounds(),
        Rect::from_min_size(Point::ZERO, 20, 30)
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace
        .document
        .shapes
        .get(&shape_id)
        .expect("undo should restore the top shape");
    assert_eq!(
        restored.kind.bounds(),
        Rect::from_min_size(Point::new(70, 90), 20, 30)
    );
}

#[test]
pub(crate) fn layout_move_shape_up_materializes_parent_shape_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("move_up_child");
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 30, 40)),
        )
        .expect("child shape should be inserted");
    let top = app.workspace.document.top_cell;
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(500, 600))
        .expect("top instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, instance_id)
            .expect("top instance should be mutable");
        instance.transform = Transform::translate(500, 600).compose(Transform::rotate_cw90());
    }
    let before_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == shape_id)
        .expect("flattened child shape should be visible")
        .bounds;

    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", child.0))
    );
    app.selected_layout_shape = Some(shape_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.move_shape_up"),
        "hierarchy context should expose move-shape-up"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.move_shape_up"));
    assert_eq!(app.layout_view_top_cell, top);
    assert!(
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&shape_id))
            .is_none(),
        "move up should remove the source child-cell shape"
    );
    let moved_id = app
        .selected_layout_shape()
        .expect("moved parent shape should be selected");
    let moved = app
        .workspace
        .document
        .shapes
        .get(&moved_id)
        .expect("moved shape should be top-level");
    assert_eq!(moved.kind.bounds(), before_bounds);
    assert!(app.status_message().contains("Moved shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&shape_id))
            .is_some(),
        "undo should restore the child-cell shape"
    );
    assert!(!app.workspace.document.shapes.contains_key(&moved_id));
}

#[test]
pub(crate) fn layout_move_nested_shape_up_materializes_parent_shape_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("move_nested_shape_mid");
    let leaf = app.workspace.document.create_cell("move_nested_shape_leaf");
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 30, 40)),
        )
        .expect("leaf shape should be inserted");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(70, 90))
        .expect("leaf instance should be inserted");
    let top = app.workspace.document.top_cell;
    let mid_instance = app
        .workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(500, 600))
        .expect("mid instance should be inserted");
    let occurrence =
        ShapeOccurrenceId::from_instance_path(shape_id, &[mid_instance, leaf_instance]);
    let before_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(top, &occurrence)
        .expect("nested shape should be visible from the document top")
        .bounds;

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.occurrence.{}",
        layout_occurrence_action_key(&occurrence)
    )));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with nested shape context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.move_shape_up"),
        "hierarchy context should expose move-shape-up for nested selected shapes"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.move_shape_up"));
    assert_eq!(app.layout_view_top_cell, mid);
    assert!(
        app.workspace
            .document
            .cell(leaf)
            .and_then(|cell| cell.shapes.get(&shape_id))
            .is_none(),
        "move up should remove the source leaf-cell shape"
    );
    let moved_id = app
        .selected_layout_shape()
        .expect("moved parent shape should be selected");
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should remain after moving the shape");
    assert_eq!(
        mid_cell
            .shapes
            .get(&moved_id)
            .expect("moved shape should be stored in the parent cell")
            .kind
            .bounds(),
        Rect::from_min_size(Point::new(80, 110), 30, 40)
    );
    let after_bounds = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(
            top,
            &ShapeOccurrenceId::from_instance_path(moved_id, &[mid_instance]),
        )
        .expect("moved shape should remain visible through the ancestor instance")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "nested shape move-up should preserve document-top placed geometry"
    );
    assert!(app.status_message().contains("Moved shape"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .cell(leaf)
            .and_then(|cell| cell.shapes.get(&shape_id))
            .is_some(),
        "undo should restore the leaf-cell shape"
    );
    assert!(
        app.workspace
            .document
            .cell(mid)
            .is_some_and(|cell| !cell.shapes.contains_key(&moved_id))
    );
}
