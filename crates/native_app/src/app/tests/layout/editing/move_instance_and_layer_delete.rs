#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_move_instance_up_materializes_parent_instance_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("move_up_mid");
    let leaf = app.workspace.document.create_cell("move_up_leaf");
    let leaf_shape = app
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
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, mid_instance)
            .expect("top instance should be mutable");
        instance.transform = Transform::translate(500, 600).compose(Transform::rotate_cw90());
    }
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
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.move_instance_up"),
        "hierarchy context should expose move-instance-up"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.move_instance_up"));
    assert_eq!(app.layout_view_top_cell, top);
    assert!(
        app.workspace
            .document
            .instance(mid, leaf_instance)
            .is_none(),
        "move up should remove the source child-cell instance"
    );
    let moved_instance_id = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| occurrence.instance_path.first().copied())
        .expect("moved parent instance should be selected");
    let moved = app
        .workspace
        .document
        .instance(top, moved_instance_id)
        .expect("moved instance should be top-level");
    assert_eq!(moved.cell, leaf);
    let after_bounds = app
        .workspace
        .document
        .visible_flattened_shapes_for_cell(top, None)
        .into_iter()
        .find(|shape| shape.source_shape_id() == leaf_shape)
        .expect("flattened leaf shape should remain visible")
        .bounds;
    assert_eq!(
        after_bounds, before_bounds,
        "moved parent instance should preserve placed geometry"
    );
    assert!(app.status_message().contains("Moved instance"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .instance(mid, leaf_instance)
            .is_some(),
        "undo should restore the child-cell instance"
    );
    assert!(
        app.workspace
            .document
            .instance(top, moved_instance_id)
            .is_none()
    );
}

#[test]
pub(crate) fn layout_resolve_array_replaces_instance_array_with_single_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("array_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
        )
        .expect("test child shape should be inserted");
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(child, Transform::translate(100, 200))
        .expect("test child instance should be inserted");
    let top = app.workspace.document.top_cell;
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(top, instance_id)
            .expect("test instance should be mutable");
        instance.array = InstanceArray {
            columns: 2,
            rows: 2,
            column_pitch: Vector::new(30, 0),
            row_pitch: Vector::new(0, 40),
        };
    }
    let instance_count = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .values()
        .count();

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(top, instance_id)
    )));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.resolve_array"));
    let top_cell = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should still exist");
    assert!(top_cell.instances.get(&instance_id).is_none());
    assert_eq!(top_cell.instances.values().count(), instance_count + 3);
    let translations = top_cell
        .instances
        .values()
        .filter(|instance| instance.cell == child)
        .map(|instance| instance.transform.translation)
        .collect::<Vec<_>>();
    assert!(translations.contains(&Vector::new(100, 200)));
    assert!(translations.contains(&Vector::new(130, 200)));
    assert!(translations.contains(&Vector::new(100, 240)));
    assert!(translations.contains(&Vector::new(130, 240)));
    assert!(
        top_cell
            .instances
            .values()
            .filter(|instance| instance.cell == child)
            .all(|instance| instance.array.normalized().is_single()),
        "resolved array elements should be ordinary single instances"
    );
    assert!(app.status_message().contains("Resolved instance array"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace
        .document
        .instance(top, instance_id)
        .expect("undo should restore the original array instance");
    assert_eq!(restored.array.normalized().columns, 2);
    assert_eq!(restored.array.normalized().rows, 2);
}

#[test]
pub(crate) fn layout_resolve_array_replaces_current_cell_array_with_single_instances() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("array_mid");
    let leaf = app.workspace.document.create_cell("array_leaf_nested");
    app.workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
        )
        .expect("leaf shape should be inserted");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(70, 90))
        .expect("leaf instance should be inserted");
    {
        let mut instance = app
            .workspace
            .document
            .instance_mut(mid, leaf_instance)
            .expect("leaf instance should be mutable");
        instance.array = InstanceArray {
            columns: 2,
            rows: 1,
            column_pitch: Vector::new(25, 0),
            row_pitch: Vector::ZERO,
        };
    }
    app.workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 2_000))
        .expect("mid instance should be inserted");

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(mid, leaf_instance)
    )));
    assert_eq!(app.layout_view_top_cell, mid);
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.resolve_array"));
    let mid_cell = app
        .workspace
        .document
        .cell(mid)
        .expect("mid cell should still exist");
    assert!(mid_cell.instances.get(&leaf_instance).is_none());
    let replacements = mid_cell
        .instances
        .values()
        .filter(|instance| instance.cell == leaf)
        .collect::<Vec<_>>();
    assert_eq!(replacements.len(), 2);
    assert!(replacements.iter().all(|instance| {
        instance.array.normalized().is_single()
            && [Vector::new(70, 90), Vector::new(95, 90)].contains(&instance.transform.translation)
    }));
    assert!(app.status_message().contains("Resolved instance array"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace
        .document
        .instance(mid, leaf_instance)
        .expect("undo should restore the original current-cell array instance");
    assert_eq!(restored.array.normalized().columns, 2);
    assert_eq!(restored.array.normalized().rows, 1);
}

#[test]
pub(crate) fn layout_hierarchy_context_exposes_parent_and_child_navigation() {
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
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 50, 50)),
        )
        .expect("leaf shape should be inserted");
    let leaf_instance = app
        .workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(100, 0))
        .expect("leaf instance should be inserted");
    app.workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 0))
        .expect("mid instance should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", mid.0)));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");

    for node_name in [
        "glassworks.layout.hierarchy_context.title".to_string(),
        "glassworks.layout.hierarchy_context.current".to_string(),
        format!(
            "glassworks.viewctl.layout.context_parent_cell.{}",
            app.workspace.document.top_cell.0
        ),
        format!("glassworks.viewctl.layout.context_child_cell.{}", leaf.0),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "hierarchy context should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.context_child_cell.{}",
        leaf.0
    )));
    assert_eq!(app.layout_view_top_cell, leaf);
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.context_parent_cell.{}",
        mid.0
    )));
    assert_eq!(app.layout_view_top_cell, mid);

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.instance.{}",
        layout_instance_action_key(mid, leaf_instance)
    )));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build after instance selection");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.descend_selected_instance"),
        "hierarchy context should expose selected-instance descend action"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.descend_selected_instance"));
    assert_eq!(app.layout_view_top_cell, leaf);
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(leaf_shape))
    );
    assert_eq!(
        app.layout_previous_view
            .expect("descend should preserve parent view")
            .top_cell,
        mid
    );
}

#[test]
pub(crate) fn layout_duplicate_current_cell_copies_local_geometry_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let source = app.workspace.document.create_cell("duplicate_source");
    let child = app.workspace.document.create_cell("duplicate_child");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 50, 50)),
        )
        .expect("child shape should be inserted");
    let source_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            source,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 120, 80)),
        )
        .expect("source shape should be inserted");
    let nested_instance = app
        .workspace
        .document
        .insert_instance(source, child, Transform::translate(40, 50))
        .expect("source child instance should be inserted");
    app.workspace
        .document
        .insert_instance_in_top(source, Transform::translate(1_000, 0))
        .expect("source should be instanced under the document top");
    let cells_before = app.workspace.document.cells.len();

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", source.0)));
    app.selected_layout_shape = Some(source_shape);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(source_shape));
    let hierarchy_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hierarchy context");
    assert!(
        hierarchy_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.duplicate_current_cell"),
        "hierarchy context should expose current-cell duplication"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.duplicate_current_cell"));
    let duplicate = app.layout_view_top_cell;
    assert_ne!(duplicate, source);
    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    let duplicate_cell = app
        .workspace
        .document
        .cell(duplicate)
        .expect("duplicate cell should exist");
    assert_eq!(duplicate_cell.shapes.len(), 1);
    assert!(
        duplicate_cell.shapes.get(&source_shape).is_none(),
        "duplicated local shape should get a fresh id"
    );
    let copied_shape = duplicate_cell
        .shapes
        .keys()
        .next()
        .copied()
        .expect("duplicate should contain the copied local shape");
    assert_eq!(
        duplicate_cell
            .shapes
            .get(&copied_shape)
            .expect("copied shape should exist")
            .kind
            .bounds(),
        Rect::from_min_size(Point::new(0, 0), 120, 80)
    );
    let copied_instance = duplicate_cell
        .instances
        .values()
        .next()
        .expect("duplicate should copy child instances");
    assert_ne!(copied_instance.id, nested_instance);
    assert_eq!(copied_instance.cell, child);
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(copied_shape))
    );
    assert!(app.status_message().contains("Duplicated cell"));

    assert!(app.apply_clicked_node_name("glassworks.menu.edit"));
    let menu_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("edit menu should build");
    assert!(
        menu_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.edit.duplicate_cell"),
        "Edit menu should expose duplicate-cell action"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.cell(duplicate).is_none());
    assert_eq!(app.workspace.document.cells.len(), cells_before);
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
}

#[test]
pub(crate) fn layout_unused_cell_delete_is_guarded_and_undoable() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let unused = app.workspace.document.create_cell("unused");
    app.workspace
        .document
        .insert_shape_in_cell(
            unused,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 50, 50)),
        )
        .expect("unused cell shape should be inserted");
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", unused.0)));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.delete_unused_cell"),
        "hierarchy context should expose unused-cell deletion"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_unused_cell"));
    assert!(app.workspace.document.cell(unused).is_none());
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
    assert!(app.status_message().contains("Deleted unused cell"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.cell(unused).is_some());

    let used = app.workspace.document.create_cell("used");
    app.workspace
        .document
        .insert_shape_in_cell(
            used,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 0), 50, 50)),
        )
        .expect("used cell shape should be inserted");
    app.workspace
        .document
        .insert_instance_in_top(used, Transform::translate(1_000, 0))
        .expect("used cell instance should be inserted");
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", used.0)));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_unused_cell"));
    assert!(app.workspace.document.cell(used).is_some());
    assert!(
        app.status_message().contains("still instanced"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_shallow_delete_current_cell_removes_references_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let used = app.workspace.document.create_cell("delete_me");
    let leaf = app.workspace.document.create_cell("preserved_leaf");
    app.workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 50, 50)),
        )
        .expect("leaf shape should be inserted");
    app.workspace
        .document
        .insert_shape_in_cell(
            used,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("used cell shape should be inserted");
    let child_instance = app
        .workspace
        .document
        .insert_instance(used, leaf, Transform::translate(100, 0))
        .expect("used cell child instance should be inserted");
    let top_instance_a = app
        .workspace
        .document
        .insert_instance_in_top(used, Transform::translate(1_000, 0))
        .expect("first top reference should be inserted");
    let top_instance_b = app
        .workspace
        .document
        .insert_instance_in_top(used, Transform::translate(2_000, 0))
        .expect("second top reference should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", used.0)));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.delete_cell_shallow"),
        "hierarchy context should expose shallow current-cell deletion"
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.edit"));
    let menu_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("edit menu should build");
    assert!(
        menu_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.edit.delete_cell_shallow"),
        "Edit menu should expose shallow current-cell deletion"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_cell_shallow"));
    assert!(app.workspace.document.cell(used).is_none());
    assert!(app.workspace.document.cell(leaf).is_some());
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(!top_cell.instances.contains_key(&top_instance_a));
    assert!(!top_cell.instances.contains_key(&top_instance_b));
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
    assert!(
        app.status_message()
            .contains("Deleted cell delete_me and 2 references"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored = app
        .workspace
        .document
        .cell(used)
        .expect("deleted cell should be restored by undo");
    assert!(restored.instances.contains_key(&child_instance));
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(top_cell.instances.contains_key(&top_instance_a));
    assert!(top_cell.instances.contains_key(&top_instance_b));
    assert!(app.workspace.document.cell(leaf).is_some());

    app.layout_view_top_cell = app.workspace.document.top_cell;
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_cell_shallow"));
    assert!(
        app.status_message()
            .contains("Document top cell cannot be deleted"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_deep_delete_current_cell_prunes_unshared_subtree_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let root = app.workspace.document.create_cell("deep_root");
    let owned_child = app.workspace.document.create_cell("owned_child");
    let owned_leaf = app.workspace.document.create_cell("owned_leaf");
    let shared_child = app.workspace.document.create_cell("shared_child");
    let shared_leaf = app.workspace.document.create_cell("shared_leaf");
    for (index, cell) in [root, owned_child, owned_leaf, shared_child, shared_leaf]
        .into_iter()
        .enumerate()
    {
        app.workspace
            .document
            .insert_shape_in_cell(
                cell,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(
                    Point::new((index as Coord) * 100, 0),
                    50,
                    50,
                )),
            )
            .expect("cell shape should be inserted");
    }
    let owned_child_instance = app
        .workspace
        .document
        .insert_instance(root, owned_child, Transform::translate(100, 0))
        .expect("owned child instance should be inserted");
    app.workspace
        .document
        .insert_instance(owned_child, owned_leaf, Transform::translate(100, 0))
        .expect("owned leaf instance should be inserted");
    app.workspace
        .document
        .insert_instance(root, shared_child, Transform::translate(200, 0))
        .expect("shared child instance should be inserted under root");
    app.workspace
        .document
        .insert_instance(shared_child, shared_leaf, Transform::translate(100, 0))
        .expect("shared leaf instance should be inserted");
    let root_ref_a = app
        .workspace
        .document
        .insert_instance_in_top(root, Transform::translate(1_000, 0))
        .expect("first root reference should be inserted");
    let root_ref_b = app
        .workspace
        .document
        .insert_instance_in_top(root, Transform::translate(2_000, 0))
        .expect("second root reference should be inserted");
    let shared_ref = app
        .workspace
        .document
        .insert_instance_in_top(shared_child, Transform::translate(3_000, 0))
        .expect("shared child top reference should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", root.0)));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.delete_cell_deep"),
        "hierarchy context should expose deep current-cell deletion"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_cell_deep"));
    assert!(app.workspace.document.cell(root).is_none());
    assert!(app.workspace.document.cell(owned_child).is_none());
    assert!(app.workspace.document.cell(owned_leaf).is_none());
    assert!(app.workspace.document.cell(shared_child).is_some());
    assert!(app.workspace.document.cell(shared_leaf).is_some());
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(!top_cell.instances.contains_key(&root_ref_a));
    assert!(!top_cell.instances.contains_key(&root_ref_b));
    assert!(top_cell.instances.contains_key(&shared_ref));
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
    assert!(
        app.status_message()
            .contains("Deep deleted cell deep_root: 3 cells, 2 references"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_root = app
        .workspace
        .document
        .cell(root)
        .expect("root should be restored");
    assert!(restored_root.instances.contains_key(&owned_child_instance));
    assert!(app.workspace.document.cell(owned_child).is_some());
    assert!(app.workspace.document.cell(owned_leaf).is_some());
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(top_cell.instances.contains_key(&root_ref_a));
    assert!(top_cell.instances.contains_key(&root_ref_b));
    assert!(top_cell.instances.contains_key(&shared_ref));
}

#[test]
pub(crate) fn layout_complete_delete_current_cell_removes_shared_subtree_and_undoes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let root = app.workspace.document.create_cell("complete_root");
    let owned_child = app.workspace.document.create_cell("owned_child");
    let owned_leaf = app.workspace.document.create_cell("owned_leaf");
    let shared_child = app.workspace.document.create_cell("shared_child");
    let shared_leaf = app.workspace.document.create_cell("shared_leaf");
    for (index, cell) in [root, owned_child, owned_leaf, shared_child, shared_leaf]
        .into_iter()
        .enumerate()
    {
        app.workspace
            .document
            .insert_shape_in_cell(
                cell,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(
                    Point::new((index as Coord) * 100, 0),
                    50,
                    50,
                )),
            )
            .expect("cell shape should be inserted");
    }
    app.workspace
        .document
        .insert_instance(root, owned_child, Transform::translate(100, 0))
        .expect("owned child instance should be inserted");
    app.workspace
        .document
        .insert_instance(owned_child, owned_leaf, Transform::translate(100, 0))
        .expect("owned leaf instance should be inserted");
    app.workspace
        .document
        .insert_instance(root, shared_child, Transform::translate(200, 0))
        .expect("shared child instance should be inserted under root");
    app.workspace
        .document
        .insert_instance(shared_child, shared_leaf, Transform::translate(100, 0))
        .expect("shared leaf instance should be inserted");
    let root_ref_a = app
        .workspace
        .document
        .insert_instance_in_top(root, Transform::translate(1_000, 0))
        .expect("first root reference should be inserted");
    let root_ref_b = app
        .workspace
        .document
        .insert_instance_in_top(root, Transform::translate(2_000, 0))
        .expect("second root reference should be inserted");
    let shared_ref = app
        .workspace
        .document
        .insert_instance_in_top(shared_child, Transform::translate(3_000, 0))
        .expect("shared child top reference should be inserted");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", root.0)));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.delete_cell_complete"),
        "hierarchy context should expose complete current-cell deletion"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete_cell_complete"));
    assert!(app.workspace.document.cell(root).is_none());
    assert!(app.workspace.document.cell(owned_child).is_none());
    assert!(app.workspace.document.cell(owned_leaf).is_none());
    assert!(app.workspace.document.cell(shared_child).is_none());
    assert!(app.workspace.document.cell(shared_leaf).is_none());
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(!top_cell.instances.contains_key(&root_ref_a));
    assert!(!top_cell.instances.contains_key(&root_ref_b));
    assert!(!top_cell.instances.contains_key(&shared_ref));
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
    assert!(
        app.status_message()
            .contains("Complete deleted cell complete_root: 5 cells, 3 references"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace.document.cell(root).is_some());
    assert!(app.workspace.document.cell(owned_child).is_some());
    assert!(app.workspace.document.cell(owned_leaf).is_some());
    assert!(app.workspace.document.cell(shared_child).is_some());
    assert!(app.workspace.document.cell(shared_leaf).is_some());
    let top_cell = app
        .workspace
        .document
        .cell(app.workspace.document.top_cell)
        .expect("document top should exist");
    assert!(top_cell.instances.contains_key(&root_ref_a));
    assert!(top_cell.instances.contains_key(&root_ref_b));
    assert!(top_cell.instances.contains_key(&shared_ref));
}

#[test]
pub(crate) fn layout_hierarchy_tree_exposes_and_collapses_cell_paths() {
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
    app.workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 50, 50)),
        )
        .expect("leaf shape should be inserted");
    app.workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(100, 0))
        .expect("leaf instance should be inserted");
    app.workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 0))
        .expect("mid instance should be inserted");

    let expanded = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.layout.hierarchy_tree.title".to_string(),
        "glassworks.viewctl.layout.tree_expand_all".to_string(),
        "glassworks.viewctl.layout.tree_collapse_all".to_string(),
        format!(
            "glassworks.viewctl.layout.tree_cell.{}",
            app.workspace.document.top_cell.0
        ),
        format!("glassworks.viewctl.layout.tree_cell.{}", mid.0),
        format!("glassworks.viewctl.layout.tree_cell.{}", leaf.0),
        format!("glassworks.viewctl.layout.tree_toggle.{}", mid.0),
    ] {
        assert!(
            expanded.nodes().iter().any(|node| node.name() == node_name),
            "hierarchy tree should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.tree_collapse_all"));
    let fully_collapsed = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should rebuild after bulk collapse");
    assert!(
        !fully_collapsed
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.tree_cell.{}", mid.0)),
        "bulk collapse should hide descendants below the document top"
    );
    assert!(app.status_message().contains("Collapsed"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.tree_expand_all"));
    let reexpanded = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should rebuild after bulk expand");
    assert!(
        reexpanded
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.tree_cell.{}", leaf.0)),
        "bulk expand should show descendants again"
    );
    assert!(app.status_message().contains("Expanded"));

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.tree_toggle.{}", mid.0)));
    let collapsed = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should rebuild after collapse");
    assert!(
        collapsed
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.tree_cell.{}", mid.0)),
        "collapsed tree should keep the collapsed cell row"
    );
    assert!(
        !collapsed
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.tree_cell.{}", leaf.0)),
        "collapsed tree should hide descendants"
    );

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.tree_cell.{}", mid.0)));
    assert_eq!(app.layout_view_top_cell, mid);
}

#[test]
pub(crate) fn layout_cell_visibility_hides_geometry_from_2d_view_only() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mid = app.workspace.document.create_cell("hide_parent");
    let leaf = app.workspace.document.create_cell("hide_me");
    let leaf_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            leaf,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 50, 50)),
        )
        .expect("leaf shape should be inserted");
    app.workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(100, 0))
        .expect("leaf instance should be inserted");
    app.workspace
        .document
        .insert_instance_in_top(mid, Transform::translate(1_000, 0))
        .expect("mid instance should be inserted");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.full"));
    let full_display_count = app.with_layout_display_index(|index| index.len());
    assert!(
        app.workspace
            .document
            .visible_flattened_shapes()
            .iter()
            .any(|shape| shape.source_shape_id() == leaf_shape),
        "physical flattened hierarchy should include the test child shape"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    let hide_action = format!("glassworks.viewctl.layout.cell_visibility.{}", leaf.0);
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == hide_action),
        "hierarchy tree should expose a per-cell hide action"
    );
    for node_name in [
        "glassworks.viewctl.layout.cell_visibility.hide_children",
        "glassworks.viewctl.layout.cell_visibility.show_children",
        "glassworks.viewctl.layout.cell_visibility.hide_descendants",
        "glassworks.viewctl.layout.cell_visibility.show_descendants",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "hierarchy context should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_visibility.hide_children"));
    assert!(app.layout_hidden_cells.contains(&mid));
    assert!(!app.layout_hidden_cells.contains(&leaf));
    assert!(
        app.with_layout_display_index(|index| index.len()) < full_display_count,
        "bulk-hidden child cells should be removed from the 2D display index"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_visibility.show_children"));
    assert!(!app.layout_hidden_cells.contains(&mid));
    assert!(!app.layout_hidden_cells.contains(&leaf));
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        full_display_count
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.cell_visibility.hide_descendants")
    );
    assert!(app.layout_hidden_cells.contains(&mid));
    assert!(app.layout_hidden_cells.contains(&leaf));
    assert!(
        app.with_layout_display_index(|index| index.len()) < full_display_count,
        "bulk-hidden descendant cells should be removed from the 2D display index"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.cell_visibility.show_descendants")
    );
    assert!(!app.layout_hidden_cells.contains(&mid));
    assert!(!app.layout_hidden_cells.contains(&leaf));
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        full_display_count
    );

    assert!(app.apply_clicked_node_name(&hide_action));
    assert!(app.layout_hidden_cells.contains(&leaf));
    assert!(
        app.with_layout_display_index(|index| index.len()) < full_display_count,
        "hidden child cells should be removed from the 2D display index"
    );
    assert!(
        !layout_shape_browser_entries(&app)
            .iter()
            .any(|(occurrence, _)| occurrence.source_shape_id() == leaf_shape),
        "hidden child geometry should be removed from the shape browser"
    );
    assert!(
        app.workspace
            .document
            .visible_flattened_shapes()
            .iter()
            .any(|shape| shape.source_shape_id() == leaf_shape),
        "cell hide/show should not change the physical hierarchy used by analysis"
    );
    let hidden_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build with hidden cell controls");
    assert!(
        hidden_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.cell_visibility.show_all"),
        "hierarchy context should expose a bulk show action when cells are hidden"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_visibility.show_all"));
    assert!(!app.layout_hidden_cells.contains(&leaf));
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        full_display_count
    );
}

#[test]
pub(crate) fn layout_fps_uses_smoothed_frame_interval() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let first = std::time::Instant::now();
    app.record_layout_frame_timing(first, 100.0);
    assert_eq!(app.layout_frame_ms.get(), Some(100.0));
    assert_eq!(app.layout_fps_frame_ms(), Some(100.0));

    app.record_layout_frame_timing(first + std::time::Duration::from_millis(20), 1.0);
    let latest = app
        .layout_frame_ms
        .get()
        .expect("latest frame interval should be recorded");
    let smoothed = app
        .layout_fps_frame_ms()
        .expect("smoothed frame interval should be recorded");
    assert!((latest - 20.0).abs() < 0.001);
    assert!(
        smoothed > latest && smoothed < 100.0,
        "EMA should move toward the latest frame interval without jumping instantly"
    );

    app.set_active_view(StartupView::Layout3d);
    assert_eq!(app.layout_frame_ms.get(), None);
    assert_eq!(app.layout_fps_frame_ms(), None);
}

#[test]
pub(crate) fn layout_fps_tick_updates_without_dirtying_layout() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let first = std::time::Instant::now();
    let revision = app.layout_revision;

    app.record_layout_frame_tick(first);
    assert_eq!(app.layout_revision, revision);
    assert_eq!(app.layout_fps_frame_ms(), None);

    app.record_layout_frame_tick(first + std::time::Duration::from_millis(16));
    let latest = app
        .layout_frame_ms
        .get()
        .expect("tick should record frame interval after first sample");
    assert!((latest - 16.0).abs() < 0.001);
    assert_eq!(app.layout_revision, revision);
}

#[test]
pub(crate) fn layout_fps_tick_only_changes_overlay_text() {
    for view in [StartupView::Layout2d, StartupView::Layout3d] {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        let first = std::time::Instant::now();
        app.record_layout_frame_tick(first);
        app.record_layout_frame_tick(first + std::time::Duration::from_millis(16));
        let before = app
            .build_operad_document(UiSize::new(1280.0, 900.0))
            .expect("layout document should build");
        app.record_layout_frame_tick(first + std::time::Duration::from_millis(100));
        let after = app
            .build_operad_document(UiSize::new(1280.0, 900.0))
            .expect("layout document should rebuild for FPS text");

        let before_text = document_text_by_node(&before);
        let after_text = document_text_by_node(&after);
        let changed = before_text
            .keys()
            .chain(after_text.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|name| before_text.get(*name) != after_text.get(*name))
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(
            changed,
            vec!["glassworks.layout.fps".to_string()],
            "{view:?} FPS tick should only update the live FPS text"
        );
    }
}

#[test]
pub(crate) fn layout_analysis_reports_are_revision_cached() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });

    let _ = app.connectivity_report();
    assert!(
        app.drc_report().is_none(),
        "DRC should not run implicitly from UI/report reads"
    );
    let _ = app.run_drc_summary();
    let first_connectivity_revision = app
        .connectivity_report_cache
        .borrow()
        .as_ref()
        .expect("connectivity report should be cached")
        .revision;
    let first_drc_revision = app
        .drc_report_cache
        .borrow()
        .as_ref()
        .expect("DRC report should be cached")
        .revision;

    let _ = app.connectivity_report();
    assert!(app.drc_report().is_some());
    assert_eq!(
        first_connectivity_revision,
        app.connectivity_report_cache
            .borrow()
            .as_ref()
            .expect("connectivity cache should remain populated")
            .revision
    );
    assert_eq!(
        first_drc_revision,
        app.drc_report_cache
            .borrow()
            .as_ref()
            .expect("DRC cache should remain populated")
            .revision
    );

    app.mark_layout_dirty();
    assert!(app.connectivity_report_cache.borrow().is_none());
    assert!(app.drc_report_cache.borrow().is_none());
}

#[test]
pub(crate) fn layout_3d_view_exposes_stack_panel_hud_and_fullscreen_viewport() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(1800.0, 980.0))
        .expect("wide 3D layout should build");
    assert_eq!(document.audit_layout(), Vec::new());
    let inspector = node_rect(&document, "glassworks.layout.inspector_rail");
    let preview = node_rect(&document, "glassworks.layout.preview");
    assert!(
        inspector.x < preview.x,
        "layout inspector should sit to the left of the canvas; inspector={inspector:?} preview={preview:?}"
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.details"),
        "3D layout should not render the generic below-canvas detail panel"
    );
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("3D Stack"), "{visible_text}");
    assert!(visible_text.contains("flycam"), "{visible_text}");
    assert!(visible_text.contains("mouse look"), "{visible_text}");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.mode_hud"),
        "3D viewport should publish a HUD overlay"
    );
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.layout.fps"
                && matches!(node.content(), UiContent::Text(text) if text.text.starts_with("FPS:"))
        }),
        "3D viewport should expose the same FPS label as the 2D viewport"
    );
    assert!(
        document.nodes().iter().any(|node| {
            matches!(
                node.content(),
                UiContent::Canvas(canvas) if canvas.key == "glassworks.layout.viewport.3d"
            )
        }),
        "3D viewport should keep using the WGPU canvas"
    );

    assert!(app.apply_clicked_node_name("glassworks.toolbar.tools.fullscreen"));
    let fullscreen = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("fullscreen 3D layout should build");
    assert_eq!(fullscreen.audit_layout(), Vec::new());
    assert!(
        fullscreen
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.fullscreen.viewport_stack")
    );
    assert!(
        !fullscreen
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.nav"),
        "viewport fullscreen should hide the workspace side chrome"
    );
    assert!(document_visible_text(&fullscreen).contains("Exit fullscreen"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_3d_grid_option_adds_ground_guides() {
    let document = Document::demo();
    let basic = build_layout_3d_batch(&document);
    let with_grid = build_layout_3d_batch_with_options(&document, true);
    let mesh = build_layout_3d_batch_with_bounds_and_options(
        &document,
        false,
        LayoutIndex::rebuild_hierarchical(&document).bounds(),
        false,
    );
    let basic_validation = basic
        .validate_geometry()
        .expect("basic 3D batch should validate");
    let grid_validation = with_grid
        .validate_geometry()
        .expect("3D grid batch should validate");
    let mesh_validation = mesh
        .validate_geometry()
        .expect("mesh 3D batch should validate");

    assert_eq!(basic_validation.guide_segments, 2);
    assert_eq!(mesh_validation.rect_slabs, 0);
    assert!(mesh_validation.mesh_triangles > 0);
    assert!(
        grid_validation.guide_segments > basic_validation.guide_segments,
        "3D grid should add ground guide segments"
    );
    let max_rect_extent = with_grid
        .rect_slabs
        .iter()
        .flat_map(|slab| slab.rect)
        .map(f32::abs)
        .fold(0.0, f32::max);
    assert!(
        max_rect_extent > 100.0,
        "3D slab coordinates should stay in layout world space so the flycam and renderer share units"
    );
}

#[test]
pub(crate) fn layout_editor_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let layer_id = app
        .workspace()
        .document
        .layers
        .keys()
        .next()
        .copied()
        .expect("demo document should have layers");
    let shape_id = app
        .workspace()
        .document
        .shapes
        .keys()
        .next()
        .copied()
        .expect("demo document should have shapes");

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.layer.{}", layer_id.0)));
    assert_eq!(app.active_layer(), layer_id);

    let visible = app
        .workspace()
        .document
        .layers
        .get(&layer_id)
        .expect("selected layer should exist")
        .visible;
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.toggle_layer.{}",
        layer_id.0
    )));
    assert_ne!(
        app.workspace()
            .document
            .layers
            .get(&layer_id)
            .expect("selected layer should exist")
            .visible,
        visible
    );

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.shape.{}", shape_id.0)));
    assert_eq!(app.selected_layout_shape(), Some(shape_id));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.top"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Top);
    assert_eq!(app.app_options.layout.hierarchy_depth, "top");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.more"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::One);
    assert_eq!(app.app_options.layout.hierarchy_depth, "one");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.two"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Two);
    assert_eq!(app.app_options.layout.hierarchy_depth, "two");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.more"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Three);
    assert_eq!(app.app_options.layout.hierarchy_depth, "three");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.more"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Numeric(4));
    assert_eq!(app.app_options.layout.hierarchy_depth, "depth_4");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.depth_5"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Numeric(5));
    assert_eq!(app.app_options.layout.hierarchy_depth, "depth_5");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.less"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Numeric(4));
    assert_eq!(app.app_options.layout.hierarchy_depth, "depth_4");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.less"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Three);
    assert_eq!(app.app_options.layout.hierarchy_depth, "three");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_step.less"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Two);
    assert_eq!(app.app_options.layout.hierarchy_depth, "two");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.boxes"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Boxes);
    assert_eq!(app.app_options.layout.hierarchy_depth, "boxes");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.full"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Full);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_min.1"));
    assert_eq!(app.layout_hierarchy_min_depth, 1);
    assert_eq!(app.app_options.layout.hierarchy_min_depth, 1);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_min_step.more"));
    assert_eq!(app.layout_hierarchy_min_depth, 2);
    assert_eq!(app.app_options.layout.hierarchy_min_depth, 2);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.top"));
    assert_eq!(app.layout_hierarchy_depth, LayoutHierarchyDepth::Top);
    assert_eq!(app.layout_hierarchy_min_depth, 0);
    assert_eq!(app.app_options.layout.hierarchy_min_depth, 0);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.full"));
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.shape.{}", shape_id.0)));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.copy"));
    let shape_count = app.workspace().document.shapes.len();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.paste"));
    assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
    assert!(app.selected_layout_shape().is_some());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.clear_selection"));
    assert_eq!(app.selected_layout_shape(), None);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.next_shape"));
    assert!(app.selected_layout_shape().is_some());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.delete"));
    assert_eq!(app.workspace().document.shapes.len(), shape_count);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.run_drc"));
    assert!(app.status_message().contains("DRC"));
    let mut document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout editor shell should build");
    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|node| node.name == "glassworks.viewctl.layout.run_drc_region"),
        "layout controls should expose selected-region DRC"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.connectivity"));
    assert!(app.status_message().contains("Connectivity"));

    assert_clicked_node(&mut document, "glassworks.tool.rect");
}

#[test]
pub(crate) fn layout_layer_delete_removes_layer_and_undo_restores_shapes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let shape = app
        .workspace()
        .document
        .shapes
        .values()
        .next()
        .expect("demo document should have shapes");
    let layer_id = shape.layer;
    let shape_id = shape.id;
    let layer_count = app.workspace().document.layers.len();
    let shape_count = app.workspace().document.shapes.len();

    assert!(app.apply_clicked_node_name(&format!("glassworks.layers.delete.{}", layer_id.0)));
    assert!(app.workspace().document.layers.get(&layer_id).is_none());
    assert!(app.workspace().document.shapes.get(&shape_id).is_none());
    assert!(app.workspace().document.shapes.len() < shape_count);

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(app.workspace().document.layers.get(&layer_id).is_some());
    assert!(app.workspace().document.shapes.get(&shape_id).is_some());
    assert_eq!(app.workspace().document.layers.len(), layer_count);
}
