#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn operations_allocate_past_remote_shape_ids() {
    let mut doc = Document::new("test");
    let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape = Shape {
        id: ShapeId(99),
        layer,
        net: None,
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
        name: None,
        properties: BTreeMap::new(),
    };
    doc.apply_operation_without_log(&Operation::AddShape { shape });
    assert_eq!(doc.allocate_shape_id(), ShapeId(100));
}

#[test]
pub(crate) fn new_documents_have_a_top_cell_for_hierarchy() {
    let doc = Document::new("hierarchy");
    assert_eq!(doc.top_cell, DEFAULT_TOP_CELL_ID);
    assert!(doc.cell(doc.top_cell).is_some());
    assert_eq!(doc.next_cell_id, DEFAULT_TOP_CELL_ID.0 + 1);
}

#[test]
pub(crate) fn default_technology_builds_document_layers() {
    let technology = default_technology();
    let doc = Document::from_technology("tech", &technology).unwrap();

    assert_eq!(doc.grid, 10);
    assert_eq!(
        doc.layer_by_process(ProcessLayer::Metal1),
        technology.layer_id("metal1").ok()
    );
    let metal1 = doc
        .layer(doc.layer_by_process(ProcessLayer::Metal1).unwrap())
        .unwrap();
    assert_eq!(metal1.purpose, "first routing metal");
    assert_eq!(metal1.fill_style, LayerFillStyle::Solid);
    assert_eq!(metal1.line_style, LayerLineStyle::Solid);
    assert_eq!(metal1.display_order, 40);
    assert_eq!(metal1.gds_layer, Some(4));
    assert_eq!(metal1.gds_datatype, 0);
    assert_eq!(
        technology.layer_for_gds_geometry(4, 0).unwrap().name,
        "metal1"
    );
    assert_eq!(
        technology.layer_stack_range_for_process(ProcessLayer::Metal1),
        Some((340.0, 430.0))
    );
}

#[test]
pub(crate) fn layer_display_styles_default_serialize_and_apply() {
    let mut doc = Document::new("layer display styles");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let mut layer_value =
        serde_json::to_value(doc.layer(metal1).unwrap()).expect("layer should serialize");
    let layer_object = layer_value
        .as_object_mut()
        .expect("serialized layer should be an object");
    layer_object.remove("fill_style");
    layer_object.remove("line_style");
    let restored: Layer =
        serde_json::from_value(layer_value).expect("old layer should deserialize");
    assert_eq!(restored.fill_style, LayerFillStyle::Solid);
    assert_eq!(restored.line_style, LayerLineStyle::Solid);

    let operation = Operation::SetLayerDisplayStyle {
        layer: metal1,
        fill_style: LayerFillStyle::CrossHatched,
        line_style: LayerLineStyle::DashDot,
    };
    let serialized =
        serde_json::to_string(&operation).expect("operation should serialize new fill style");
    assert!(serialized.contains("cross_hatched"));
    assert!(serialized.contains("dash_dot"));
    let decoded: Operation =
        serde_json::from_str(&serialized).expect("operation should deserialize");
    let Operation::SetLayerDisplayStyle {
        fill_style,
        line_style,
        ..
    } = decoded
    else {
        panic!("decoded operation should set layer display style");
    };
    assert_eq!(fill_style, LayerFillStyle::CrossHatched);
    assert_eq!(line_style, LayerLineStyle::DashDot);

    let original_alpha = doc.layer_color(metal1)[3];
    doc.apply_operation_without_log(&operation);
    let layer = doc.layer(metal1).unwrap();
    assert_eq!(layer.fill_style, LayerFillStyle::CrossHatched);
    assert_eq!(layer.line_style, LayerLineStyle::DashDot);
    assert!(doc.layer_display_color(metal1)[3] < original_alpha);

    doc.layers.get_mut(&metal1).unwrap().fill_style = LayerFillStyle::Stippled;
    assert!(doc.layer_display_color(metal1)[3] < original_alpha);
    doc.layers.get_mut(&metal1).unwrap().fill_style = LayerFillStyle::DenseStippled;
    assert!(doc.layer_display_color(metal1)[3] < original_alpha);
    doc.layers.get_mut(&metal1).unwrap().fill_style = LayerFillStyle::SparseStippled;
    assert!(doc.layer_display_color(metal1)[3] < original_alpha);
}

#[test]
pub(crate) fn rename_layer_operation_serializes_and_applies() {
    let mut doc = Document::new("rename layer");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let operation = Operation::RenameLayer {
        id: metal1,
        name: "metal1_bus".to_string(),
    };
    let decoded: Operation = serde_json::from_str(
        &serde_json::to_string(&operation).expect("operation should serialize"),
    )
    .expect("operation should deserialize");
    let Operation::RenameLayer { id, name } = decoded else {
        panic!("decoded operation should rename layer");
    };
    assert_eq!(id, metal1);
    assert_eq!(name, "metal1_bus");

    doc.apply_operation_without_log(&operation);

    assert_eq!(doc.layer(metal1).unwrap().name, "metal1_bus");
}

#[test]
pub(crate) fn delete_layer_removes_shapes_on_that_layer() {
    let mut doc = Document::new("delete layer");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let metal2 = doc.layer_by_process(ProcessLayer::Metal2).unwrap();
    let removed_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
    );
    let kept_shape = doc.insert_shape(
        metal2,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(20, 0), 10, 10)),
    );

    doc.apply_operation_without_log(&Operation::DeleteLayer { id: metal1 });

    assert!(!doc.layers.contains_key(&metal1));
    assert!(!doc.shapes.contains_key(&removed_shape));
    assert!(doc.shapes.contains_key(&kept_shape));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_layer_names() {
    let mut technology = default_technology();
    technology.layers[1].name = technology.layers[0].name.clone();

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate layer name"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_gds_mapping() {
    let mut technology = default_technology();
    technology.layers[1].gds_layer = technology.layers[0].gds_layer;
    technology.layers[1].gds_datatype = technology.layers[0].gds_datatype;

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate GDSII geometry mapping"));
}

#[test]
pub(crate) fn invalid_technology_reports_incomplete_stack_metadata() {
    let mut technology = default_technology();
    technology.layers[0].z_thickness = None;

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("z_base and z_thickness"));
}

#[test]
pub(crate) fn invalid_technology_reports_out_of_range_color_components() {
    let mut technology = default_technology();
    technology.layers[0].color[3] = 1.25;

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("color components"));
}

#[test]
pub(crate) fn invalid_technology_reports_overlapping_stack_ranges() {
    let mut technology = default_technology();
    let poly = technology
        .layers
        .iter_mut()
        .find(|layer| layer.process == "poly")
        .unwrap();
    poly.z_base = Some(120.0);

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("3D stack range"));
    assert!(err.to_string().contains("overlaps"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_drc_width_and_spacing_rules() {
    let mut technology = default_technology();
    technology.drc.min_width.push(TechnologyLayerRule {
        layer: "diffusion".to_string(),
        value: 250,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate min_width"));

    let mut technology = default_technology();
    technology.drc.min_spacing.push(TechnologyLayerRule {
        layer: "metal-1".to_string(),
        value: 300,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate min_spacing"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_drc_max_width_rules() {
    let mut technology = default_technology();
    technology.drc.max_width.push(TechnologyLayerRule {
        layer: "metal-1".to_string(),
        value: 2_000,
    });
    technology.drc.max_width.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 3_000,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate max_width"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_drc_area_rules() {
    let mut technology = default_technology();
    technology.drc.min_area.push(TechnologyLayerRule {
        layer: "metal-1".to_string(),
        value: 20_000,
    });
    technology.drc.min_area.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 30_000,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate min_area"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_drc_max_area_rules() {
    let mut technology = default_technology();
    technology.drc.max_area.push(TechnologyLayerRule {
        layer: "metal-1".to_string(),
        value: 200_000,
    });
    technology.drc.max_area.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 300_000,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate max_area"));
}

#[test]
pub(crate) fn invalid_technology_reports_duplicate_drc_edge_spacing_rules() {
    let mut technology = default_technology();
    technology.drc.min_edge_spacing.push(TechnologyLayerRule {
        layer: "metal-1".to_string(),
        value: 120,
    });
    technology.drc.min_edge_spacing.push(TechnologyLayerRule {
        layer: "metal1".to_string(),
        value: 160,
    });

    let err = technology.validate().unwrap_err();

    assert!(err.to_string().contains("duplicate min_edge_spacing"));
}

#[test]
pub(crate) fn custom_technology_stack_fixture_validates_and_builds_layers() {
    let technology = TechnologyFile::from_json_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/technology/custom_stack.json"
    )))
    .unwrap();
    let document = Document::from_technology("custom stack", &technology).unwrap();

    assert_eq!(technology.dbu_per_micron, 2_000);
    assert_eq!(document.grid, 5);
    assert_eq!(
        technology.layer_stack_range_for_process(ProcessLayer::Diffusion),
        Some((0.0, 50.0))
    );
    assert_eq!(
        technology.layer_stack_range_for_process(ProcessLayer::Contact),
        Some((50.0, 150.0))
    );
    assert_eq!(
        technology.layer_stack_range_for_process(ProcessLayer::Metal1),
        Some((150.0, 220.0))
    );
    assert_eq!(
        document.layer_by_process(ProcessLayer::Metal1),
        Some(LayerId(23))
    );
    assert_eq!(
        technology.gds_mapping_for_layer(LayerId(23)),
        Some((23, 0, 0))
    );
}

#[test]
pub(crate) fn technology_application_assigns_missing_ids_without_colliding_with_explicit_ids() {
    let mut technology = default_technology();
    technology.layers[0].id = None;
    technology.layers[1].id = Some(LayerId(1));
    technology.connectivity.clear();
    technology.drc = TechnologyDrc::default();
    technology.validate().unwrap();

    let document = Document::from_technology("implicit ids", &technology).unwrap();
    let diffusion = document.layer_by_process(ProcessLayer::Diffusion).unwrap();
    let poly = document.layer_by_process(ProcessLayer::Poly).unwrap();

    assert_ne!(diffusion, poly);
    assert_eq!(poly, LayerId(1));
    assert_eq!(document.layers.len(), technology.layers.len());
    assert_eq!(
        document.layer(diffusion).unwrap().name.as_str(),
        "diffusion"
    );
    assert_eq!(document.layer(poly).unwrap().name.as_str(), "poly");
}

#[test]
pub(crate) fn flattened_shapes_include_translated_cell_instances() {
    let mut doc = Document::new("hierarchy");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("unit");
    let child_shape = doc
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    let first = doc
        .insert_instance_in_top(child, Transform::translate(1_000, 2_000))
        .unwrap();
    let second = doc
        .insert_instance_in_top(child, Transform::translate(-500, 300))
        .unwrap();

    let flattened = doc.visible_flattened_shapes();
    let instance_shapes: Vec<_> = flattened
        .iter()
        .filter(|shape| shape.source_shape_id() == child_shape)
        .collect();

    assert_eq!(instance_shapes.len(), 2);
    assert!(instance_shapes.iter().any(|shape| {
        shape.instance_path == vec![first]
            && shape.bounds == Rect::from_min_size(Point::new(1_000, 2_000), 100, 60)
    }));
    assert!(instance_shapes.iter().any(|shape| {
        shape.instance_path == vec![second]
            && shape.bounds == Rect::from_min_size(Point::new(-500, 300), 100, 60)
    }));
}

#[test]
pub(crate) fn hierarchical_index_queries_transformed_instance_bounds() {
    let mut doc = Document::new("hierarchy index");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("unit");
    let child_shape = doc
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    doc.insert_instance_in_top(child, Transform::translate(10_000, 0))
        .unwrap();

    let flat_index = LayoutIndex::rebuild(&doc);
    let hierarchy_index = LayoutIndex::rebuild_hierarchical(&doc);
    let query = Rect::from_min_size(Point::new(9_950, -50), 200, 200);

    assert!(!flat_index.query_rect(query).contains(&child_shape));
    assert!(hierarchy_index.query_rect(query).contains(&child_shape));
}

#[test]
pub(crate) fn flattened_shapes_can_limit_visible_hierarchy_depth() {
    let mut doc = Document::new("hierarchy depth");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let top_shape = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-100, 0), 40, 40)),
    );
    let mid = doc.create_cell("mid");
    let leaf = doc.create_cell("leaf");
    let mid_shape = doc
        .insert_shape_in_cell(
            mid,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    let leaf_shape = doc
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(20, 0), 80, 60)),
        )
        .unwrap();
    doc.insert_instance(mid, leaf, Transform::translate(1_000, 0))
        .unwrap();
    doc.insert_instance_in_top(mid, Transform::translate(2_000, 0))
        .unwrap();

    let top_only = doc
        .visible_flattened_shapes_with_max_depth(Some(0))
        .into_iter()
        .map(|shape| shape.source_shape_id())
        .collect::<BTreeSet<_>>();
    let one_level = doc
        .visible_flattened_shapes_with_max_depth(Some(1))
        .into_iter()
        .map(|shape| shape.source_shape_id())
        .collect::<BTreeSet<_>>();
    let full = doc
        .visible_flattened_shapes_with_max_depth(None)
        .into_iter()
        .map(|shape| shape.source_shape_id())
        .collect::<BTreeSet<_>>();
    let child_only = doc
        .visible_flattened_shapes_with_depth_range(1, None)
        .into_iter()
        .map(|shape| shape.source_shape_id())
        .collect::<BTreeSet<_>>();
    let leaf_only = doc
        .visible_flattened_shapes_with_depth_range(2, None)
        .into_iter()
        .map(|shape| shape.source_shape_id())
        .collect::<BTreeSet<_>>();

    assert_eq!(top_only, BTreeSet::from([top_shape]));
    assert_eq!(one_level, BTreeSet::from([top_shape, mid_shape]));
    assert_eq!(full, BTreeSet::from([top_shape, mid_shape, leaf_shape]));
    assert_eq!(child_only, BTreeSet::from([mid_shape, leaf_shape]));
    assert_eq!(leaf_only, BTreeSet::from([leaf_shape]));
}

#[test]
pub(crate) fn hierarchical_index_can_limit_visible_depth() {
    let mut doc = Document::new("hierarchy depth index");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let mid = doc.create_cell("mid");
    let leaf = doc.create_cell("leaf");
    let mid_shape = doc
        .insert_shape_in_cell(
            mid,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    let leaf_shape = doc
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    doc.insert_instance(mid, leaf, Transform::translate(1_000, 0))
        .unwrap();
    doc.insert_instance_in_top(mid, Transform::translate(2_000, 0))
        .unwrap();

    let limited = LayoutIndex::rebuild_hierarchical_with_max_depth(&doc, Some(1));
    let leaf_only =
        LayoutIndex::rebuild_hierarchical_for_cell_with_depth_range(&doc, doc.top_cell, 2, None);
    let full = LayoutIndex::rebuild_hierarchical_with_max_depth(&doc, None);
    let mid_query = Rect::from_min_size(Point::new(1_950, -50), 200, 200);
    let query = Rect::from_min_size(Point::new(2_950, -50), 200, 200);

    assert!(full.query_rect(mid_query).contains(&mid_shape));
    assert!(!leaf_only.query_rect(mid_query).contains(&mid_shape));
    assert!(!limited.query_rect(query).contains(&leaf_shape));
    assert!(full.query_rect(query).contains(&leaf_shape));
    assert!(leaf_only.query_rect(query).contains(&leaf_shape));
}

#[test]
pub(crate) fn flattened_shapes_and_index_can_use_child_cell_as_view_root() {
    let mut doc = Document::new("view root");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let child = doc.create_cell("child");
    let shape_id = doc
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 60)),
        )
        .unwrap();
    doc.insert_instance_in_top(child, Transform::translate(5_000, 0))
        .unwrap();

    let root_shapes = doc.visible_flattened_shapes_for_cell(child, None);
    assert_eq!(root_shapes.len(), 1);
    assert_eq!(root_shapes[0].id, ShapeOccurrenceId::top_level(shape_id));
    assert_eq!(
        root_shapes[0].bounds,
        Rect::from_min_size(Point::new(0, 0), 100, 60)
    );
    assert_eq!(
        doc.shape_view_for_occurrence_from_cell(child, &root_shapes[0].id)
            .unwrap()
            .bounds,
        Rect::from_min_size(Point::new(0, 0), 100, 60)
    );

    let document_root_index = LayoutIndex::rebuild_hierarchical(&doc);
    let child_root_index = LayoutIndex::rebuild_hierarchical_for_cell(&doc, child, None);

    assert!(
        document_root_index
            .query_rect(Rect::from_min_size(Point::new(4_950, -50), 200, 200))
            .contains(&shape_id)
    );
    assert!(
        child_root_index
            .query_rect(Rect::from_min_size(Point::new(-50, -50), 200, 200))
            .contains(&shape_id)
    );
}

#[test]
pub(crate) fn tiled_layout_index_deduplicates_shapes_spanning_tiles() {
    let mut doc = Document::new("wide shape");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-20_000, -100), 40_000, 200)),
    );
    let index = LayoutIndex::rebuild(&doc);

    let hits = index.query_rect(Rect::from_min_size(
        Point::new(-30_000, -1_000),
        60_000,
        2_000,
    ));

    assert_eq!(hits, vec![id]);
}

#[test]
pub(crate) fn layout_index_reports_visible_bounds() {
    let mut doc = Document::new("index bounds");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-100, -50), 40, 20)),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 300), 80, 60)),
    );
    let index = LayoutIndex::rebuild(&doc);

    assert_eq!(
        index.bounds(),
        Some(Rect::new(Point::new(-100, -50), Point::new(280, 360)))
    );
}

#[test]
pub(crate) fn limited_layout_index_query_deduplicates_and_caps_results() {
    let mut doc = Document::new("limited index query");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let wide = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-20_000, -100), 40_000, 200)),
    );
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(24_000, -100), 100, 100)),
    );
    let index = LayoutIndex::rebuild(&doc);

    let hits = index.query_occurrences_limited(
        Rect::from_min_size(Point::new(-30_000, -1_000), 60_000, 2_000),
        1,
    );

    assert_eq!(hits, vec![ShapeOccurrenceId::top_level(wide)]);
}

#[test]
pub(crate) fn shape_store_serializes_as_legacy_shape_map() {
    let mut doc = Document::new("shape store serde");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
    );

    let encoded = serde_json::to_value(&doc).unwrap();
    let shapes = encoded
        .get("shapes")
        .and_then(|value| value.as_object())
        .unwrap();
    assert!(shapes.contains_key(&id.0.to_string()));

    let restored: Document = serde_json::from_value(encoded).unwrap();
    assert_eq!(
        restored.shapes.get(&id).unwrap().kind.bounds(),
        doc.shapes.get(&id).unwrap().kind.bounds()
    );
}

#[test]
pub(crate) fn old_measurement_shapes_deserialize_with_direct_mode() {
    let mut doc = Document::new("old measurement mode");
    let annotation = doc.layer_by_process(ProcessLayer::Annotation).unwrap();
    let id = doc.insert_shape(
        annotation,
        ShapeKind::Measurement {
            a: Point::new(0, 0),
            b: Point::new(10, 20),
            label: "legacy ruler".to_string(),
            mode: MeasurementMode::Manhattan,
        },
    );
    let mut json = serde_json::to_value(&doc).unwrap();
    let shape = json
        .get_mut("shapes")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|shapes| shapes.get_mut(&id.0.to_string()))
        .expect("shape should serialize in legacy shape map");
    let measurement = shape
        .get_mut("kind")
        .and_then(|kind| kind.get_mut("Measurement"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("measurement shape kind should serialize as an object");
    measurement.remove("mode");

    let restored: Document = serde_json::from_value(json).unwrap();
    match restored.shapes.get(&id).unwrap().kind {
        ShapeKind::Measurement { mode, .. } => assert_eq!(mode, MeasurementMode::Direct),
        _ => panic!("expected measurement"),
    }
}

#[test]
pub(crate) fn hierarchy_serializes_and_round_trips() {
    let mut doc = Document::new("round trip");
    let poly = doc.layer_by_process(ProcessLayer::Poly).unwrap();
    let child = doc.create_cell("nand2");
    doc.cell_mut(child)
        .unwrap()
        .properties
        .insert("library.macro".to_string(), "via_array".to_string());
    let shape_id = doc
        .insert_shape_in_cell(
            child,
            poly,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(25, 40), 120, 600)),
        )
        .unwrap();
    let instance_id = doc
        .insert_instance_in_top(child, Transform::translate(800, -300))
        .unwrap();

    let json = serde_json::to_string(&doc).unwrap();
    let restored: Document = serde_json::from_str(&json).unwrap();
    let restored_cell = restored.cell(child).unwrap();

    assert!(restored_cell.shapes.contains_key(&shape_id));
    assert_eq!(
        restored_cell
            .properties
            .get("library.macro")
            .map(String::as_str),
        Some("via_array")
    );
    assert!(
        restored
            .cell(restored.top_cell)
            .unwrap()
            .instances
            .contains_key(&instance_id)
    );
    assert_eq!(restored.visible_flattened_shapes().len(), 1);
}

#[test]
pub(crate) fn old_flat_documents_deserialize_with_default_hierarchy_fields() {
    let mut doc = Document::new("old");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 10)),
    );
    let mut json = serde_json::to_value(&doc).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("next_cell_id");
    object.remove("next_instance_id");
    object.remove("top_cell");
    object.remove("cells");
    object.remove("marker_states");
    object.remove("reference_images");

    let restored: Document = serde_json::from_value(json).unwrap();
    assert_eq!(restored.top_cell, DEFAULT_TOP_CELL_ID);
    assert_eq!(restored.visible_shapes().count(), 1);
    assert_eq!(restored.visible_flattened_shapes().len(), 1);
    assert!(restored.marker_states.is_empty());
    assert!(restored.reference_images.is_empty());
}

#[test]
pub(crate) fn old_documents_deserialize_with_default_crdt_metadata() {
    let doc = Document::new("old crdt");
    let mut json = serde_json::to_value(&doc).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("crdt_seen");
    object.remove("crdt_actor_clocks");
    object.remove("crdt_operation_log");

    let restored: Document = serde_json::from_value(json).unwrap();

    assert!(restored.crdt_seen.is_empty());
    assert!(restored.crdt_actor_clocks.is_empty());
    assert!(restored.crdt_operation_log.is_empty());
}

#[test]
pub(crate) fn old_instance_documents_deserialize_with_identity_matrix_and_single_array() {
    let mut doc = Document::new("old instance");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell = doc.create_cell("unit");
    doc.insert_shape_in_cell(
        cell,
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 10, 20)),
    )
    .unwrap();
    let instance = doc
        .insert_instance_in_top(cell, Transform::translate(30, 40))
        .unwrap();
    let mut json = serde_json::to_value(&doc).unwrap();
    let instance_json = json
        .pointer_mut(&format!("/cells/1/instances/{}", instance.0))
        .unwrap()
        .as_object_mut()
        .unwrap();
    instance_json.remove("array");
    instance_json
        .get_mut("transform")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("matrix");

    let restored: Document = serde_json::from_value(json).unwrap();
    let restored_instance = restored.instance(restored.top_cell, instance).unwrap();

    assert_eq!(
        restored_instance.transform.matrix,
        Transform::IDENTITY.matrix
    );
    assert_eq!(restored_instance.transform.translation, Vector::new(30, 40));
    assert_eq!(restored_instance.array, InstanceArray::single());
    assert_eq!(restored.visible_flattened_shapes().len(), 1);
}

#[test]
pub(crate) fn marker_states_round_trip_with_document() {
    let mut doc = Document::new("markers");
    doc.marker_states.insert(
        "min_width|1|0,0,10,10|200|80.000".to_string(),
        MarkerState {
            hidden: true,
            waived: true,
            visited: true,
            important: true,
            note: Some("known demo marker".to_string()),
            owner: Some("layout-team".to_string()),
            signoff: Some("accepted".to_string()),
            signoff_by: Some("layout-team".to_string()),
            signoff_note: Some("known demo marker".to_string()),
            signoff_records: BTreeMap::from([(
                "layout-team".to_string(),
                MarkerSignoffRecord {
                    status: "accepted".to_string(),
                    role: Some("layout".to_string()),
                    by: Some("layout-team".to_string()),
                    note: Some("known demo marker".to_string()),
                    recorded_at: Some("2026-05-19T12:00:00Z".to_string()),
                },
            )]),
            tags: BTreeMap::from([("action".to_string(), "fix".to_string())]),
        },
    );

    let restored: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    let state = restored
        .marker_states
        .get("min_width|1|0,0,10,10|200|80.000")
        .unwrap();

    assert!(state.hidden);
    assert!(state.waived);
    assert!(state.visited);
    assert!(state.important);
    assert_eq!(state.note.as_deref(), Some("known demo marker"));
    assert_eq!(state.owner.as_deref(), Some("layout-team"));
    assert_eq!(state.signoff.as_deref(), Some("accepted"));
    assert_eq!(state.signoff_by.as_deref(), Some("layout-team"));
    assert_eq!(state.signoff_note.as_deref(), Some("known demo marker"));
    let signoff = state.signoff_records.get("layout-team").unwrap();
    assert_eq!(signoff.status, "accepted");
    assert_eq!(signoff.role.as_deref(), Some("layout"));
    assert_eq!(signoff.by.as_deref(), Some("layout-team"));
    assert_eq!(signoff.note.as_deref(), Some("known demo marker"));
    assert_eq!(signoff.recorded_at.as_deref(), Some("2026-05-19T12:00:00Z"));
    assert_eq!(state.tags.get("action").map(String::as_str), Some("fix"));
}

#[test]
pub(crate) fn reference_images_round_trip_and_align_from_landmarks() {
    let mut doc = Document::new("reference images");
    let mut image = ReferenceImageOverlay::new(
        "sem-a",
        "SEM A",
        "images/sem-a.png",
        Rect::from_min_size(Point::new(10, 20), 30, 40),
    );
    image.pixel_size = Some(ReferenceImageSize::new(100, 50));
    image.opacity = 128;
    image.landmarks = vec![
        ReferenceImageLandmark::new("left", Point::new(10, 10), Point::new(1_000, 2_000)),
        ReferenceImageLandmark::new("right", Point::new(60, 30), Point::new(1_500, 1_800)),
    ];

    assert!(image.apply_landmark_alignment().unwrap());
    assert_eq!(
        image.bounds,
        Rect::new(Point::new(900, 1_600), Point::new(1_900, 2_100))
    );
    doc.reference_images.push(image);

    let restored: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    let restored_image = &restored.reference_images[0];
    assert_eq!(restored_image.id, "sem-a");
    assert_eq!(restored_image.display_name(), "SEM A");
    assert_eq!(restored_image.opacity, 128);
    assert_eq!(restored_image.landmarks.len(), 2);
    assert!(restored_image.image_key().contains("images/sem-a.png"));
}

#[test]
pub(crate) fn reference_image_alignment_ignores_single_landmark() {
    let mut image = ReferenceImageOverlay::new(
        "single",
        "single",
        "image.png",
        Rect::from_min_size(Point::new(0, 0), 10, 10),
    );
    image.pixel_size = Some(ReferenceImageSize::new(10, 10));
    image.landmarks.push(ReferenceImageLandmark::new(
        "only",
        Point::new(1, 1),
        Point::new(100, 100),
    ));

    assert!(!image.apply_landmark_alignment().unwrap());
    assert_eq!(image.bounds, Rect::from_min_size(Point::new(0, 0), 10, 10));
}

#[test]
pub(crate) fn connectivity_issue_states_round_trip_with_document() {
    let mut doc = Document::new("connectivity issue states");
    doc.connectivity_issue_states.insert(
        "short|VDD,VSS|0,0,100,50".to_string(),
        MarkerState {
            hidden: true,
            waived: false,
            note: Some("bench-waived after ECO-17".to_string()),
            ..MarkerState::default()
        },
    );

    let restored: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    let state = restored
        .connectivity_issue_states
        .get("short|VDD,VSS|0,0,100,50")
        .unwrap();

    assert!(state.hidden);
    assert!(!state.waived);
    assert!(!state.visited);
    assert!(!state.important);
    assert_eq!(state.note.as_deref(), Some("bench-waived after ECO-17"));
    assert_eq!(state.owner, None);
    assert_eq!(state.signoff, None);
}

#[test]
pub(crate) fn marker_state_operations_set_and_remove_review_state() {
    let mut doc = Document::new("review state operations");
    let marker_key = "min_width|1|0,0,10,10|200|80.000".to_string();
    let issue_key = "short|VDD,VSS|0,0,100,50".to_string();
    let marker_state = MarkerState {
        hidden: true,
        waived: true,
        visited: true,
        important: true,
        note: Some("known test fixture marker".to_string()),
        owner: Some("signoff-owner".to_string()),
        signoff: Some("accepted".to_string()),
        signoff_by: Some("signoff-owner".to_string()),
        signoff_note: Some("known test fixture marker".to_string()),
        signoff_records: BTreeMap::from([(
            "signoff-owner".to_string(),
            MarkerSignoffRecord {
                status: "accepted".to_string(),
                role: Some("owner".to_string()),
                by: Some("signoff-owner".to_string()),
                note: Some("known test fixture marker".to_string()),
                recorded_at: Some("2026-05-19T12:05:00Z".to_string()),
            },
        )]),
        tags: BTreeMap::from([("disposition".to_string(), "false_positive".to_string())]),
    };
    let issue_state = MarkerState {
        hidden: false,
        waived: true,
        note: Some("accepted process exception".to_string()),
        ..MarkerState::default()
    };

    doc.apply_operation_without_log(&Operation::SetMarkerState {
        key: marker_key.clone(),
        state: Some(marker_state.clone()),
    });
    doc.apply_operation_without_log(&Operation::SetConnectivityIssueState {
        key: issue_key.clone(),
        state: Some(issue_state.clone()),
    });

    assert_eq!(doc.marker_states.get(&marker_key), Some(&marker_state));
    assert_eq!(
        doc.connectivity_issue_states.get(&issue_key),
        Some(&issue_state)
    );

    doc.apply_operation_without_log(&Operation::SetMarkerState {
        key: marker_key.clone(),
        state: Some(MarkerState::default()),
    });
    doc.apply_operation_without_log(&Operation::SetConnectivityIssueState {
        key: issue_key.clone(),
        state: None,
    });

    assert!(!doc.marker_states.contains_key(&marker_key));
    assert!(!doc.connectivity_issue_states.contains_key(&issue_key));

    let encoded = serde_json::to_string(&Operation::SetMarkerState {
        key: marker_key.clone(),
        state: Some(marker_state.clone()),
    })
    .unwrap();
    let decoded: Operation = serde_json::from_str(&encoded).unwrap();
    let Operation::SetMarkerState { key, state } = decoded else {
        panic!("expected marker state operation");
    };
    assert_eq!(key, marker_key);
    assert_eq!(state, Some(marker_state));
}

#[test]
pub(crate) fn server_snapshot_json_round_trips_numeric_id_map_keys() {
    let doc = Document::new("snapshot json");
    let message = ServerMessage::Snapshot {
        document: doc.clone(),
        cursors: BTreeMap::new(),
        selections: BTreeMap::new(),
        loro_snapshot: vec![1, 2, 3],
    };
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"layers\":{\"1\""));

    let restored: ServerMessage = serde_json::from_str(&json).unwrap();
    let ServerMessage::Snapshot {
        document,
        loro_snapshot,
        ..
    } = restored
    else {
        panic!("expected snapshot message");
    };
    assert_eq!(document.layers.len(), doc.layers.len());
    assert_eq!(document.cell(DEFAULT_TOP_CELL_ID).unwrap().name, "top");
    assert_eq!(loro_snapshot, vec![1, 2, 3]);
}

#[test]
pub(crate) fn crdt_operations_are_idempotent_by_actor_counter() {
    let mut doc = Document::new("crdt idempotency");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape_id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
    );
    let actor = Uuid::from_u128(7);
    let operation = CrdtOperation {
        id: CrdtOpId { actor, counter: 1 },
        deps: Vec::new(),
        operation: Operation::MoveShape {
            id: shape_id,
            delta: Vector::new(20, -10),
        },
    };

    assert_eq!(
        doc.apply_crdt_operation(operation.clone()),
        CrdtApplyResult::Applied
    );
    assert_eq!(
        doc.apply_crdt_operation(operation),
        CrdtApplyResult::Duplicate
    );

    let shape = doc.shapes.get(&shape_id).unwrap();
    assert_eq!(
        shape.kind.bounds(),
        Rect::from_min_size(Point::new(20, -10), 100, 50)
    );
    assert_eq!(doc.crdt_operation_log.len(), 1);
    assert_eq!(doc.crdt_actor_clock(actor), 1);
}

#[test]
pub(crate) fn crdt_operation_ids_and_dependency_frontier_use_actor_clocks() {
    let mut doc = Document::new("crdt frontier");
    let actor_a = Uuid::from_u128(11);
    let actor_b = Uuid::from_u128(12);
    let layer = doc.layer_by_process(ProcessLayer::Annotation).unwrap();

    let add_layer = CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 3,
        },
        deps: Vec::new(),
        operation: Operation::SetLayerVisibility {
            layer,
            visible: false,
        },
    };
    assert_eq!(
        doc.apply_crdt_operation(add_layer),
        CrdtApplyResult::Applied
    );

    let next = doc.next_crdt_operation_id(actor_a);
    let mut deps = doc.crdt_dependency_frontier();
    deps.sort();

    assert_eq!(
        next,
        CrdtOpId {
            actor: actor_a,
            counter: 4
        }
    );
    assert_eq!(
        deps,
        vec![CrdtOpId {
            actor: actor_a,
            counter: 3
        }]
    );
    assert_eq!(doc.next_crdt_operation_id(actor_b).counter, 1);
}

#[test]
pub(crate) fn loro_log_replication_emits_each_operation_once() {
    let actor_a = Uuid::from_u128(21);
    let actor_b = Uuid::from_u128(22);
    let mut source = LoroCrdtLog::new(actor_a).unwrap();
    let mut target = LoroCrdtLog::new(actor_b).unwrap();
    let operation = CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 1,
        },
        deps: Vec::new(),
        operation: Operation::Cursor {
            user: actor_a,
            position: Point::new(10, 20),
        },
    };

    let update = source.append_operation(actor_a, operation.clone()).unwrap();

    assert!(source.import_update(&update).unwrap().is_empty());
    let imported = target.import_update(&update).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].id, operation.id);
    assert!(target.import_update(&update).unwrap().is_empty());
}

#[test]
pub(crate) fn loro_log_replication_carries_review_state_operations() {
    let actor_a = Uuid::from_u128(121);
    let actor_b = Uuid::from_u128(122);
    let mut source = LoroCrdtLog::new(actor_a).unwrap();
    let mut target = LoroCrdtLog::new(actor_b).unwrap();
    let mut target_doc = Document::new("review state replica");
    let marker_key = "min_width|1|0,0,10,10|200|80.000".to_string();
    let issue_key = "short|VDD,VSS|0,0,100,50".to_string();
    let marker_state = MarkerState {
        hidden: true,
        waived: false,
        note: Some("reviewed remotely".to_string()),
        owner: Some("remote-layout".to_string()),
        signoff: Some("needs_review".to_string()),
        signoff_by: Some("remote-layout".to_string()),
        signoff_note: Some("reviewed remotely".to_string()),
        tags: BTreeMap::from([("source".to_string(), "external".to_string())]),
        ..MarkerState::default()
    };
    let issue_state = MarkerState {
        hidden: false,
        waived: true,
        note: Some("waived remotely".to_string()),
        ..MarkerState::default()
    };

    let update = source
        .append_operation(
            actor_a,
            CrdtOperation {
                id: CrdtOpId {
                    actor: actor_a,
                    counter: 1,
                },
                deps: Vec::new(),
                operation: Operation::Batch {
                    operations: vec![
                        Operation::SetMarkerState {
                            key: marker_key.clone(),
                            state: Some(marker_state.clone()),
                        },
                        Operation::SetConnectivityIssueState {
                            key: issue_key.clone(),
                            state: Some(issue_state.clone()),
                        },
                    ],
                },
            },
        )
        .unwrap();

    for operation in target.import_update(&update).unwrap() {
        assert_eq!(
            target_doc.apply_crdt_operation(operation),
            CrdtApplyResult::Applied
        );
    }
    assert_eq!(
        target_doc.marker_states.get(&marker_key),
        Some(&marker_state)
    );
    assert_eq!(
        target_doc.connectivity_issue_states.get(&issue_key),
        Some(&issue_state)
    );

    let clear_update = source
        .append_operation(
            actor_a,
            CrdtOperation {
                id: CrdtOpId {
                    actor: actor_a,
                    counter: 2,
                },
                deps: vec![CrdtOpId {
                    actor: actor_a,
                    counter: 1,
                }],
                operation: Operation::SetConnectivityIssueState {
                    key: issue_key.clone(),
                    state: None,
                },
            },
        )
        .unwrap();
    for operation in target.import_update(&clear_update).unwrap() {
        assert_eq!(
            target_doc.apply_crdt_operation(operation),
            CrdtApplyResult::Applied
        );
    }
    assert!(
        !target_doc
            .connectivity_issue_states
            .contains_key(&issue_key)
    );
}

#[test]
pub(crate) fn loro_snapshots_seed_log_without_reemitting_history() {
    let actor_a = Uuid::from_u128(23);
    let actor_b = Uuid::from_u128(24);
    let mut source = LoroCrdtLog::new(actor_a).unwrap();
    let first = CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 1,
        },
        deps: Vec::new(),
        operation: Operation::Cursor {
            user: actor_a,
            position: Point::new(1, 2),
        },
    };
    let second = CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 2,
        },
        deps: vec![first.id],
        operation: Operation::Cursor {
            user: actor_a,
            position: Point::new(3, 4),
        },
    };
    source.append_operation(actor_a, first).unwrap();
    let snapshot = source.export_snapshot().unwrap();
    let mut restored = LoroCrdtLog::from_snapshot(actor_b, &snapshot).unwrap();

    assert!(restored.drain_unemitted_operations().unwrap().is_empty());

    let update = source.append_operation(actor_a, second.clone()).unwrap();
    let imported = restored.import_update(&update).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].id, second.id);
}

#[test]
pub(crate) fn loro_shape_store_uses_registers_and_tombstones() {
    let actor = Uuid::from_u128(25);
    let mut log = LoroCrdtLog::new(actor).unwrap();
    let shape = Shape {
        id: ShapeId(7),
        layer: LayerId(3),
        net: None,
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 40)),
        name: Some("m1 strap".to_string()),
        properties: BTreeMap::from([("custom.owner".to_string(), "unit_test".to_string())]),
    };

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::AddShape {
                shape: shape.clone(),
            },
        },
    )
    .unwrap();

    let stored = log.shape_from_store(shape.id).unwrap().unwrap();
    assert_eq!(stored.layer, shape.layer);
    assert_eq!(stored.name.as_deref(), Some("m1 strap"));
    assert_eq!(stored.properties, shape.properties);
    assert_eq!(stored.kind.bounds(), shape.kind.bounds());

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 2 },
            deps: Vec::new(),
            operation: Operation::MoveShape {
                id: shape.id,
                delta: Vector::new(50, -10),
            },
        },
    )
    .unwrap();
    let moved = log.shape_from_store(shape.id).unwrap().unwrap();
    assert_eq!(
        moved.kind.bounds(),
        Rect::from_min_size(Point::new(50, -10), 100, 40)
    );
    assert_eq!(moved.properties, shape.properties);

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 3 },
            deps: Vec::new(),
            operation: Operation::DeleteShape { id: shape.id },
        },
    )
    .unwrap();
    assert!(log.shape_is_deleted(shape.id).unwrap());
    assert!(log.shape_from_store(shape.id).unwrap().is_none());
}

#[test]
pub(crate) fn shape_operations_can_target_child_cells() {
    let mut doc = Document::new("cell shape ops");
    let cell_id = doc.create_cell("unit");
    let shape = Shape {
        id: doc.allocate_shape_id(),
        layer: LayerId(1),
        net: None,
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 20), 30, 40)),
        name: Some("local".to_string()),
        properties: BTreeMap::new(),
    };

    doc.apply_operation_without_log(&Operation::AddShapeToCell {
        cell: cell_id,
        shape: shape.clone(),
    });
    assert!(doc.shapes.get(&shape.id).is_none());
    assert_eq!(
        doc.cell(cell_id)
            .and_then(|cell| cell.shapes.get(&shape.id))
            .expect("child-cell shape should be stored")
            .kind
            .bounds(),
        shape.kind.bounds()
    );
    assert_eq!(
        doc.shape_view_for_occurrence_from_cell(cell_id, &ShapeOccurrenceId::top_level(shape.id),)
            .expect("child-cell shape should be addressable")
            .source_cell,
        cell_id
    );

    doc.apply_operation_without_log(&Operation::DeleteShapeFromCell {
        cell: cell_id,
        id: shape.id,
    });
    assert!(
        doc.cell(cell_id)
            .and_then(|cell| cell.shapes.get(&shape.id))
            .is_none()
    );

    let actor = Uuid::from_u128(260);
    let mut log = LoroCrdtLog::new(actor).unwrap();
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::AddCell {
                cell: Cell::new(cell_id, "unit"),
            },
        },
    )
    .unwrap();
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 2 },
            deps: Vec::new(),
            operation: Operation::AddShapeToCell {
                cell: cell_id,
                shape: shape.clone(),
            },
        },
    )
    .unwrap();
    assert_eq!(log.shape_parent_cell(shape.id).unwrap(), Some(cell_id));
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 3 },
            deps: Vec::new(),
            operation: Operation::DeleteShapeFromCell {
                cell: cell_id,
                id: shape.id,
            },
        },
    )
    .unwrap();
    assert!(log.shape_is_deleted(shape.id).unwrap());
}

#[test]
pub(crate) fn loro_hierarchy_store_tracks_cells_instances_and_child_shapes() {
    let actor = Uuid::from_u128(26);
    let mut log = LoroCrdtLog::new(actor).unwrap();
    let cell_id = CellId(5);
    let shape_id = ShapeId(9);
    let instance_id = InstanceId(11);
    let mut cell = Cell::new(cell_id, "unit");
    cell.shapes.insert(
        shape_id,
        Shape {
            id: shape_id,
            layer: LayerId(4),
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 30)),
            name: None,
            properties: BTreeMap::new(),
        },
    );
    let instance = CellInstance {
        id: instance_id,
        name: Some("u0".to_string()),
        cell: cell_id,
        transform: Transform::translate(100, 200),
        array: InstanceArray::single(),
        properties: BTreeMap::from([("placement.role".to_string(), "leaf_anchor".to_string())]),
    };

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::AddCell { cell },
        },
    )
    .unwrap();
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 2 },
            deps: Vec::new(),
            operation: Operation::AddInstance {
                parent: DEFAULT_TOP_CELL_ID,
                instance,
            },
        },
    )
    .unwrap();

    assert_eq!(log.shape_parent_cell(shape_id).unwrap(), Some(cell_id));
    let stored_instance = log
        .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored_instance.name.as_deref(), Some("u0"));
    assert_eq!(stored_instance.transform, Transform::translate(100, 200));
    assert_eq!(
        stored_instance
            .properties
            .get("placement.role")
            .map(String::as_str),
        Some("leaf_anchor")
    );

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 3 },
            deps: Vec::new(),
            operation: Operation::MoveInstance {
                parent: DEFAULT_TOP_CELL_ID,
                id: instance_id,
                delta: Vector::new(-50, 25),
            },
        },
    )
    .unwrap();
    let moved_instance = log
        .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
        .unwrap()
        .unwrap();
    assert_eq!(moved_instance.transform, Transform::translate(50, 225));
    assert_eq!(
        moved_instance
            .properties
            .get("placement.role")
            .map(String::as_str),
        Some("leaf_anchor")
    );

    let mut arrayed_instance = moved_instance.clone();
    arrayed_instance.array = InstanceArray {
        columns: 4,
        rows: 2,
        column_pitch: Vector::new(120, 0),
        row_pitch: Vector::new(0, 90),
    };
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 4 },
            deps: Vec::new(),
            operation: Operation::ReplaceInstance {
                parent: DEFAULT_TOP_CELL_ID,
                id: instance_id,
                instance: arrayed_instance,
            },
        },
    )
    .unwrap();
    let stored_array = log
        .instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
        .unwrap()
        .unwrap()
        .array;
    assert_eq!(stored_array.columns, 4);
    assert_eq!(stored_array.rows, 2);

    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 5 },
            deps: Vec::new(),
            operation: Operation::DeleteInstance {
                parent: DEFAULT_TOP_CELL_ID,
                id: instance_id,
            },
        },
    )
    .unwrap();
    assert!(
        log.instance_is_deleted(DEFAULT_TOP_CELL_ID, instance_id)
            .unwrap()
    );
    assert!(
        log.instance_from_store(DEFAULT_TOP_CELL_ID, instance_id)
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn loro_object_store_replicates_through_update_bytes() {
    let actor_a = Uuid::from_u128(27);
    let actor_b = Uuid::from_u128(28);
    let mut source = LoroCrdtLog::new(actor_a).unwrap();
    let mut target = LoroCrdtLog::new(actor_b).unwrap();
    let shape = Shape {
        id: ShapeId(31),
        layer: LayerId(2),
        net: Some(NetId(4)),
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(5, 6), 70, 80)),
        name: None,
        properties: BTreeMap::from([("review.intent".to_string(), "replicate".to_string())]),
    };
    let update = source
        .append_operation(
            actor_a,
            CrdtOperation {
                id: CrdtOpId {
                    actor: actor_a,
                    counter: 1,
                },
                deps: Vec::new(),
                operation: Operation::AddShape {
                    shape: shape.clone(),
                },
            },
        )
        .unwrap();

    let operations = target.import_update(&update).unwrap();
    let stored = target.shape_from_store(shape.id).unwrap().unwrap();

    assert_eq!(operations.len(), 1);
    assert_eq!(stored.layer, shape.layer);
    assert_eq!(stored.net, shape.net);
    assert_eq!(stored.properties, shape.properties);
    assert_eq!(stored.kind.bounds(), shape.kind.bounds());
}
