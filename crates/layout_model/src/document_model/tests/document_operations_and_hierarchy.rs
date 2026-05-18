#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn loro_object_store_can_materialize_document_objects() {
    let actor = Uuid::from_u128(29);
    let mut log = LoroCrdtLog::new(actor).unwrap();
    let cell_id = CellId(6);
    let shape_id = ShapeId(41);
    let instance_id = InstanceId(3);
    let mut cell = Cell::new(cell_id, "slice");
    cell.properties
        .insert("library.macro".to_string(), "via_array".to_string());
    cell.shapes.insert(
        shape_id,
        Shape {
            id: shape_id,
            layer: LayerId(4),
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 10)),
            name: None,
            properties: BTreeMap::new(),
        },
    );
    let instance = CellInstance {
        id: instance_id,
        name: Some("x0".to_string()),
        cell: cell_id,
        transform: Transform::translate(500, -100),
        array: InstanceArray::single(),
        properties: BTreeMap::new(),
    };
    log.append_operation(
        actor,
        CrdtOperation {
            id: CrdtOpId { actor, counter: 1 },
            deps: Vec::new(),
            operation: Operation::Batch {
                operations: vec![
                    Operation::AddCell { cell },
                    Operation::AddInstance {
                        parent: DEFAULT_TOP_CELL_ID,
                        instance,
                    },
                ],
            },
        },
    )
    .unwrap();

    let mut doc = Document::new("materialized");
    log.materialize_objects_into_document(&mut doc).unwrap();

    assert_eq!(doc.cell(cell_id).unwrap().name, "slice");
    assert_eq!(
        doc.cell(cell_id)
            .unwrap()
            .properties
            .get("library.macro")
            .map(String::as_str),
        Some("via_array")
    );
    assert!(doc.cell(cell_id).unwrap().shapes.contains_key(&shape_id));
    assert!(
        doc.cell(DEFAULT_TOP_CELL_ID)
            .unwrap()
            .instances
            .contains_key(&instance_id)
    );
    assert_eq!(doc.visible_flattened_shapes().len(), 1);
    assert_eq!(
        doc.visible_flattened_shapes()[0].bounds,
        Rect::from_min_size(Point::new(500, -100), 20, 10)
    );
}

#[test]
pub(crate) fn operations_add_and_move_cell_instances() {
    let mut doc = Document::new("instance ops");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell_id = doc.allocate_cell_id();
    let shape_id = doc.allocate_shape_id();
    let instance_id = doc.allocate_instance_id();
    let mut cell = Cell::new(cell_id, "unit");
    cell.shapes.insert(
        shape_id,
        Shape {
            id: shape_id,
            layer: metal1,
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            name: None,
            properties: BTreeMap::new(),
        },
    );
    let instance = CellInstance {
        id: instance_id,
        name: None,
        cell: cell_id,
        transform: Transform::translate(1_000, 2_000),
        array: InstanceArray::single(),
        properties: BTreeMap::new(),
    };

    doc.apply_operation_without_log(&Operation::Batch {
        operations: vec![
            Operation::AddCell { cell },
            Operation::AddInstance {
                parent: doc.top_cell,
                instance,
            },
        ],
    });

    let flattened = doc.visible_flattened_shapes();
    assert_eq!(flattened.len(), 1);
    assert_eq!(
        flattened[0].bounds,
        Rect::from_min_size(Point::new(1_000, 2_000), 100, 80)
    );

    doc.apply_operation_without_log(&Operation::MoveInstance {
        parent: doc.top_cell,
        id: instance_id,
        delta: Vector::new(300, -500),
    });

    let flattened = doc.visible_flattened_shapes();
    assert_eq!(
        flattened[0].bounds,
        Rect::from_min_size(Point::new(1_300, 1_500), 100, 80)
    );
}

#[test]
pub(crate) fn instance_arrays_flatten_to_distinct_occurrences() {
    let mut doc = Document::new("array instance ops");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell_id = doc.create_cell("unit");
    let shape_id = doc
        .insert_shape_in_cell(
            cell_id,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 50)),
        )
        .unwrap();
    let instance_id = doc
        .insert_instance_in_top(cell_id, Transform::translate(10, 20))
        .unwrap();
    let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
    instance.array = InstanceArray {
        columns: 3,
        rows: 2,
        column_pitch: Vector::new(200, 0),
        row_pitch: Vector::new(0, 300),
    };
    doc.apply_operation_without_log(&Operation::ReplaceInstance {
        parent: doc.top_cell,
        id: instance_id,
        instance,
    });

    let flattened = doc.visible_flattened_shapes();
    let occurrences = flattened
        .iter()
        .map(|shape| shape.id.clone())
        .collect::<BTreeSet<_>>();

    assert_eq!(flattened.len(), 6);
    assert_eq!(occurrences.len(), 6);
    assert!(
        occurrences.contains(&ShapeOccurrenceId::from_instance_array_path(
            shape_id,
            &[instance_id],
            &[ArrayIndex { column: 2, row: 1 }],
        ))
    );
    assert!(
        flattened
            .iter()
            .any(|shape| shape.bounds == Rect::from_min_size(Point::new(410, 320), 100, 50))
    );
}

#[test]
pub(crate) fn instance_transform_rotation_and_mirror_affect_flattened_bounds() {
    let mut doc = Document::new("oriented instance ops");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell_id = doc.create_cell("unit");
    doc.insert_shape_in_cell(
        cell_id,
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 40)),
    )
    .unwrap();
    let instance_id = doc
        .insert_instance_in_top(cell_id, Transform::translate(1_000, 2_000))
        .unwrap();
    let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
    instance.transform = instance.transform.compose(Transform::rotate_cw90());
    doc.apply_operation_without_log(&Operation::ReplaceInstance {
        parent: doc.top_cell,
        id: instance_id,
        instance,
    });
    assert_eq!(
        doc.visible_flattened_shapes()[0].bounds,
        Rect::from_min_size(Point::new(1_000, 1_900), 40, 100)
    );

    let mut instance = doc.instance(doc.top_cell, instance_id).unwrap().clone();
    instance.transform = Transform::translate(1_000, 2_000).compose(Transform::mirror_x());
    doc.apply_operation_without_log(&Operation::ReplaceInstance {
        parent: doc.top_cell,
        id: instance_id,
        instance,
    });
    assert_eq!(
        doc.visible_flattened_shapes()[0].bounds,
        Rect::from_min_size(Point::new(900, 2_000), 100, 40)
    );
}

#[test]
pub(crate) fn operations_delete_instances_and_cells() {
    let mut doc = Document::new("delete instance ops");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell_id = doc.create_cell("unit");
    doc.insert_shape_in_cell(
        cell_id,
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
    )
    .unwrap();
    let instance_id = doc
        .insert_instance_in_top(cell_id, Transform::translate(100, 200))
        .unwrap();
    assert_eq!(doc.visible_flattened_shapes().len(), 1);

    doc.apply_operation_without_log(&Operation::Batch {
        operations: vec![
            Operation::DeleteInstance {
                parent: doc.top_cell,
                id: instance_id,
            },
            Operation::DeleteCell { id: cell_id },
        ],
    });

    assert!(doc.cell(cell_id).is_none());
    assert_eq!(doc.visible_flattened_shapes().len(), 0);
}

#[test]
pub(crate) fn replace_shape_does_not_resurrect_deleted_shape() {
    let mut doc = Document::new("delete wins over replace");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
    );
    let replacement = Shape {
        id,
        layer: metal1,
        net: None,
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(10, 10), 100, 80)),
        name: None,
        properties: BTreeMap::new(),
    };

    doc.apply_operation_without_log(&Operation::DeleteShape { id });
    doc.apply_operation_without_log(&Operation::ReplaceShape {
        id,
        shape: replacement,
    });

    assert!(!doc.shapes.contains_key(&id));
}

#[test]
pub(crate) fn crdt_delete_wins_over_concurrent_move() {
    let mut doc = Document::new("move delete conflict");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
    );
    let actor_a = Uuid::from_u128(101);
    let actor_b = Uuid::from_u128(102);

    assert_eq!(
        doc.apply_crdt_operation(CrdtOperation {
            id: CrdtOpId {
                actor: actor_a,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::DeleteShape { id },
        }),
        CrdtApplyResult::Applied
    );
    assert_eq!(
        doc.apply_crdt_operation(CrdtOperation {
            id: CrdtOpId {
                actor: actor_b,
                counter: 1,
            },
            deps: Vec::new(),
            operation: Operation::MoveShape {
                id,
                delta: Vector::new(50, 25),
            },
        }),
        CrdtApplyResult::Applied
    );

    assert!(!doc.shapes.contains_key(&id));
}

#[test]
pub(crate) fn crdt_delete_wins_over_concurrent_vertex_replace() {
    let mut doc = Document::new("vertex delete conflict");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let id = doc.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
    );
    let actor_a = Uuid::from_u128(103);
    let actor_b = Uuid::from_u128(104);
    let replacement = Shape {
        id,
        layer: metal1,
        net: None,
        kind: ShapeKind::Polygon(Polygon {
            points: vec![
                Point::new(0, 0),
                Point::new(140, 0),
                Point::new(100, 80),
                Point::new(0, 80),
            ],
        }),
        name: None,
        properties: BTreeMap::new(),
    };

    doc.apply_crdt_operation(CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 1,
        },
        deps: Vec::new(),
        operation: Operation::DeleteShape { id },
    });
    doc.apply_crdt_operation(CrdtOperation {
        id: CrdtOpId {
            actor: actor_b,
            counter: 1,
        },
        deps: Vec::new(),
        operation: Operation::ReplaceShape {
            id,
            shape: replacement,
        },
    });

    assert!(!doc.shapes.contains_key(&id));
}

#[test]
pub(crate) fn crdt_duplicate_insert_is_idempotent() {
    let mut doc = Document::new("duplicate insert conflict");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let actor = Uuid::from_u128(105);
    let shape = Shape {
        id: ShapeId(90),
        layer: metal1,
        net: None,
        kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
        name: None,
        properties: BTreeMap::new(),
    };
    let operation = CrdtOperation {
        id: CrdtOpId { actor, counter: 1 },
        deps: Vec::new(),
        operation: Operation::AddShape {
            shape: shape.clone(),
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

    assert_eq!(doc.shapes.len(), 1);
    assert_eq!(
        doc.shapes.get(&shape.id).unwrap().kind.bounds(),
        shape.kind.bounds()
    );
}

#[test]
pub(crate) fn crdt_ordered_concurrent_layer_changes_converge() {
    let mut doc = Document::new("layer change conflict");
    let layer = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let actor_a = Uuid::from_u128(106);
    let actor_b = Uuid::from_u128(107);
    let hide = CrdtOperation {
        id: CrdtOpId {
            actor: actor_a,
            counter: 1,
        },
        deps: Vec::new(),
        operation: Operation::SetLayerVisibility {
            layer,
            visible: false,
        },
    };
    let show = CrdtOperation {
        id: CrdtOpId {
            actor: actor_b,
            counter: 1,
        },
        deps: vec![hide.id],
        operation: Operation::SetLayerVisibility {
            layer,
            visible: true,
        },
    };

    doc.apply_crdt_operation(hide.clone());
    doc.apply_crdt_operation(show);
    doc.apply_crdt_operation(hide);

    assert!(doc.layer(layer).unwrap().visible);
}

#[test]
pub(crate) fn operations_rename_cells_and_instances() {
    let mut doc = Document::new("rename ops");
    let metal1 = doc.layer_by_process(ProcessLayer::Metal1).unwrap();
    let cell_id = doc.create_cell("unit");
    doc.insert_shape_in_cell(
        cell_id,
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
    )
    .unwrap();
    let instance_id = doc
        .insert_instance_in_top(cell_id, Transform::translate(100, 200))
        .unwrap();

    doc.apply_operation_without_log(&Operation::RenameCell {
        id: cell_id,
        name: "renamed unit".to_string(),
    });
    doc.apply_operation_without_log(&Operation::SetCellProperties {
        id: cell_id,
        properties: BTreeMap::from([("library.macro".to_string(), "via_array".to_string())]),
    });
    doc.apply_operation_without_log(&Operation::RenameInstance {
        parent: doc.top_cell,
        id: instance_id,
        name: Some("u0".to_string()),
    });

    assert_eq!(doc.cell(cell_id).unwrap().name, "renamed unit");
    assert_eq!(
        doc.cell(cell_id)
            .unwrap()
            .properties
            .get("library.macro")
            .map(String::as_str),
        Some("via_array")
    );
    assert_eq!(
        doc.instance(doc.top_cell, instance_id)
            .unwrap()
            .name
            .as_deref(),
        Some("u0")
    );
}
