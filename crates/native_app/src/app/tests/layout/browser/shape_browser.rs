#![allow(unused_imports)]
use super::*;
use crate::*;

fn assert_browser_row(rows: &[(String, String)], key: &str, value: &str) {
    assert!(
        rows.iter()
            .any(|(row_key, row_value)| row_key == key && row_value == value),
        "browser rows should include {key}={value}: {rows:?}"
    );
}

#[test]
pub(crate) fn layout_cell_browser_filters_used_and_unused_cells() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("cell browser filter test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let top = app.workspace.document.top_cell;
    let used = app.workspace.document.create_cell("used leaf");
    let unused = app.workspace.document.create_cell("unused leaf");
    let empty = app.workspace.document.create_cell("empty leaf");
    app.workspace
        .document
        .cell_mut(unused)
        .expect("unused cell should exist")
        .properties
        .insert("purpose".to_string(), "scratch".to_string());
    app.workspace
        .document
        .insert_shape_in_cell(
            used,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
        )
        .expect("used cell shape should be inserted");
    app.workspace
        .document
        .insert_shape_in_cell(
            used,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(40, 0), 30, 30)),
        )
        .expect("second used cell shape should be inserted");
    app.workspace
        .document
        .insert_shape_in_cell(
            unused,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 40), 30, 30)),
        )
        .expect("unused cell shape should be inserted");
    app.workspace
        .document
        .insert_instance(top, used, Transform::IDENTITY)
        .expect("used cell should be instanced under top");

    let all = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(all, vec![top, empty, unused, used]);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.current"));
    assert_eq!(
        app.layout_cell_browser_filter,
        LayoutCellBrowserFilter::Current
    );
    let current_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(current_cells, vec![top]);
    assert!(
        layout_cell_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Current"),
        "cell browser rows should show the current-cell filter"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.used"));
    assert_eq!(
        app.layout_cell_browser_filter,
        LayoutCellBrowserFilter::Used
    );
    let used_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(used_cells, vec![top, used]);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.unused"));
    assert_eq!(
        app.layout_cell_browser_filter,
        LayoutCellBrowserFilter::Unused
    );
    let unused_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(unused_cells, vec![empty, unused]);

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.properties")
    );
    assert_eq!(
        app.layout_cell_browser_filter,
        LayoutCellBrowserFilter::Properties
    );
    let property_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(property_cells, vec![unused]);
    app.set_layout_browser_search("scratch");
    let property_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(
        property_cells,
        vec![unused],
        "cell property filter search should match stored property values"
    );
    app.set_layout_browser_search("purpose=scratch");
    let property_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(
        property_cells,
        vec![unused],
        "cell property filter should support key=value selectors"
    );
    app.set_layout_browser_search("unused");
    assert!(
        layout_cell_browser_cells(&app).is_empty(),
        "cell property filter search should not match broad cell name fields"
    );
    app.set_layout_browser_search("");
    assert!(
        layout_cell_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Properties"),
        "cell browser rows should show the property filter"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.empty"));
    assert_eq!(
        app.layout_cell_browser_filter,
        LayoutCellBrowserFilter::Empty
    );
    let empty_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(empty_cells, vec![empty]);
    assert!(
        layout_cell_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Empty"),
        "cell browser rows should show the empty filter"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.all"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_sort.shape_count"));
    assert_eq!(
        app.layout_cell_browser_sort,
        LayoutCellBrowserSort::ShapeCount
    );
    let shape_sorted = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<Vec<_>>();
    assert_eq!(shape_sorted, vec![top, used, unused, empty]);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.unused"));
    assert!(
        app.status_message()
            .contains("Cell browser filter Unused (2)"),
        "{}",
        app.status_message()
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.viewctl.layout.cell_browser_filter.all".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.current".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.used".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.unused".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.library".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.properties".to_string(),
        "glassworks.viewctl.layout.cell_browser_filter.empty".to_string(),
        "glassworks.viewctl.layout.cell_browser_sort.name".to_string(),
        "glassworks.viewctl.layout.cell_browser_sort.id".to_string(),
        "glassworks.viewctl.layout.cell_browser_sort.depth".to_string(),
        "glassworks.viewctl.layout.cell_browser_sort.shape_count".to_string(),
        "glassworks.viewctl.layout.cell_browser_sort.instance_count".to_string(),
        format!("glassworks.viewctl.layout.top_cell.{}", empty.0),
        format!("glassworks.viewctl.layout.top_cell.{}", unused.0),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "cell browser filter surface should expose {node_name}"
        );
    }
    let title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.cell_browser.title")
        .unwrap_or_else(|| panic!("cell browser title should exist"));
    let UiContent::Text(title_text) = title.content() else {
        panic!("cell browser title should be text");
    };
    assert_eq!(title_text.text, "Cell Browser - Unused (2 rows)");
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
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.all"),
        "All (4)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.current"),
        "Current (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.used"),
        "Used (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.unused"),
        "Unused (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.properties"),
        "Properties (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.cell_browser_filter.empty"),
        "Empty (1)"
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.top_cell.{}", used.0)),
        "unused filter should hide used cells"
    );

    app.layout_browser_search = "missing".to_string();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser.select_first"));
    assert!(
        app.status_message()
            .contains("Cell browser has no matching unused cells for search missing"),
        "{}",
        app.status_message()
    );
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty cell search should build");
    let empty_title = empty_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.cell_browser.title")
        .unwrap_or_else(|| panic!("empty cell browser title should exist"));
    let UiContent::Text(empty_title_text) = empty_title.content() else {
        panic!("empty cell browser title should be text");
    };
    assert_eq!(
        empty_title_text.text,
        "Cell Browser - Unused / search missing (0 rows)"
    );
}

#[test]
pub(crate) fn layout_cell_browser_select_first_uses_property_selector_search() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("cell browser select first");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let target = app.workspace.document.create_cell("opc review");
    app.workspace
        .document
        .cell_mut(target)
        .expect("target cell should exist")
        .properties
        .insert("purpose".to_string(), "scratch".to_string());
    app.workspace
        .document
        .insert_shape_in_cell(
            target,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 30, 30)),
        )
        .expect("target cell shape should be inserted");
    let other = app.workspace.document.create_cell("reticle review");
    app.workspace
        .document
        .cell_mut(other)
        .expect("other cell should exist")
        .properties
        .insert("purpose".to_string(), "reticle".to_string());

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.properties")
    );
    app.set_layout_browser_search("purpose=scratch");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with property-filtered cell browser should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.cell_browser.select_first"),
        "cell browser should expose select-first control when property matches exist"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser.select_first"));
    assert_eq!(app.layout_view_top_cell, target);
    assert!(
        app.status_message().contains("Viewing browser cell"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_cell_browser_filters_hierarchy_context_cells() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("cell browser hierarchy filters");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let top = app.workspace.document.top_cell;
    let mid = app.workspace.document.create_cell("mid branch");
    let leaf = app.workspace.document.create_cell("leaf child");
    let sibling = app.workspace.document.create_cell("sibling child");
    let unused = app.workspace.document.create_cell("unused leaf");
    for cell in [leaf, sibling, unused] {
        app.workspace
            .document
            .insert_shape_in_cell(
                cell,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
            )
            .expect("leaf test shape should be inserted");
    }
    app.workspace
        .document
        .insert_instance(mid, leaf, Transform::translate(100, 0))
        .expect("leaf should be instanced under mid");
    app.workspace
        .document
        .insert_instance(top, mid, Transform::translate(1_000, 0))
        .expect("mid should be instanced under top");
    app.workspace
        .document
        .insert_instance(top, sibling, Transform::translate(2_000, 0))
        .expect("sibling should be instanced under top");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.leaves"));
    let leaf_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(leaf_cells, BTreeSet::from([leaf, sibling, unused]));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.branches"));
    let branch_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(branch_cells, BTreeSet::from([top, mid]));

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", leaf.0)));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.parents"));
    assert_eq!(
        layout_cell_browser_cells(&app)
            .into_iter()
            .map(|cell| cell.id)
            .collect::<Vec<_>>(),
        vec![mid]
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.ancestors"));
    let ancestor_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(ancestor_cells, BTreeSet::from([top, mid]));

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", top.0)));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.children"));
    let child_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(child_cells, BTreeSet::from([mid, sibling]));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.siblings"));
    let sibling_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(sibling_cells, BTreeSet::new());

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", mid.0)));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.siblings"));
    let sibling_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(sibling_cells, BTreeSet::from([sibling]));

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", top.0)));
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.descendants")
    );
    let descendant_cells = layout_cell_browser_cells(&app)
        .into_iter()
        .map(|cell| cell.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(descendant_cells, BTreeSet::from([mid, leaf, sibling]));

    let rows = layout_cell_browser_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Filter" && value == "Descendants"),
        "cell browser rows should expose the active hierarchy filter: {rows:?}"
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with hierarchy cell filters should build");
    for node_name in [
        "glassworks.viewctl.layout.cell_browser_filter.leaves",
        "glassworks.viewctl.layout.cell_browser_filter.branches",
        "glassworks.viewctl.layout.cell_browser_filter.parents",
        "glassworks.viewctl.layout.cell_browser_filter.children",
        "glassworks.viewctl.layout.cell_browser_filter.siblings",
        "glassworks.viewctl.layout.cell_browser_filter.ancestors",
        "glassworks.viewctl.layout.cell_browser_filter.descendants",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "cell browser hierarchy filter control should exist: {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_library_macro_creates_via_array_in_current_cell() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro current-cell create test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let child = app.workspace.document.create_cell("macro_create_child");
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(20_000, 0))
        .expect("child should be placed");
    app.layout_view_top_cell = child;
    let cells_before = app.workspace.document.cells.len();
    let child_instances_before = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should exist")
        .instances
        .len();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array_3x3"));

    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 3x3"))
        .expect("current-cell macro should create a named library cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 9);
    let child_cell = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should still exist");
    assert_eq!(child_cell.instances.len(), child_instances_before + 1);
    let created_instance = child_cell
        .instances
        .values()
        .find(|instance| instance.cell == created_cell.id)
        .expect("macro cell should be placed in the current child cell");
    assert!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should still exist")
            .instances
            .values()
            .all(|instance| instance.cell != created_cell.id),
        "current-cell macro creation should not place the generated cell under the document top"
    );
    assert_eq!(app.layout_view_top_cell, child);
    assert!(
        app.selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| occurrence.instance_path == vec![created_instance.id]),
        "current-cell macro creation should select a shape occurrence inside the placed instance"
    );
    assert!(app.status_message().contains("Created"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.cell(created_cell.id).is_none(),
        "undo should remove the generated current-cell macro cell"
    );
    assert_eq!(
        app.workspace
            .document
            .cell(child)
            .expect("child cell should remain after undo")
            .instances
            .len(),
        child_instances_before
    );
}

#[test]
pub(crate) fn layout_library_macro_creates_reusable_via_array_cell() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let via_layer = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Via1)
        .expect("default technology should include via1");
    let cells_before = app.workspace.document.cells.len();
    let instances_before = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.library_macro.via_array_3x3" }),
        "cell browser should expose the via-array library macro"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array_3x3"));
    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 3x3"))
        .expect("macro should create a named library cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 9);
    assert_eq!(
        created_cell
            .properties
            .get("library.macro")
            .map(String::as_str),
        Some("via_array")
    );
    assert!(
        created_cell.shapes.values().all(|shape| {
            shape.layer == via_layer && matches!(shape.kind, ShapeKind::Via { size: 180, .. })
        }),
        "created cell should contain only default via shapes"
    );

    let top_cell = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should still exist");
    assert_eq!(top_cell.instances.len(), instances_before + 1);
    let created_instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == created_cell.id)
        .expect("macro cell should be placed as a top-level instance");
    assert_eq!(app.active_layer, via_layer);
    assert_eq!(app.layout_view_top_cell, top);
    let selected_occurrence = app
        .selected_layout_occurrence
        .as_ref()
        .expect("macro should select a placed via occurrence");
    assert_eq!(selected_occurrence.instance_path, vec![created_instance.id]);
    assert!(
        created_cell
            .shapes
            .contains_key(&selected_occurrence.source_shape_id()),
        "selection should refer to a source shape inside the created cell"
    );
    assert!(app.status_message().contains("reusable"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.cell_browser_filter.library"));
    assert_eq!(
        layout_cell_browser_cells(&app)
            .into_iter()
            .map(|cell| cell.id)
            .collect::<Vec<_>>(),
        vec![created_cell.id]
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.cell(created_cell.id).is_none(),
        "undo should remove the generated library cell"
    );
    assert_eq!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should remain after undo")
            .instances
            .len(),
        instances_before
    );
}

#[test]
pub(crate) fn layout_library_macro_parameter_controls_shape_generated_via_array() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro parameter test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.layout.library.via_array.params",
        "glassworks.viewctl.layout.library_macro.via_array.columns.inc",
        "glassworks.viewctl.layout.library_macro.via_array.rows.inc",
        "glassworks.viewctl.layout.library_macro.via_array.size.inc",
        "glassworks.viewctl.layout.library_macro.via_array.pitch.inc",
        "glassworks.viewctl.layout.library_macro.via_array.create",
        "glassworks.viewctl.layout.library_macro.via_array.from_selection",
        "glassworks.viewctl.layout.library_macro.via_array.load_selected",
        "glassworks.viewctl.layout.library_macro.via_array.update_selected",
        "glassworks.viewctl.layout.library_macro.make_static",
        "glassworks.layout.library.presets.title",
        "glassworks.viewctl.layout.library_macro.preset.save.1",
        "glassworks.viewctl.layout.library_macro.preset.load.1",
        "glassworks.viewctl.layout.library_macro.preset.create.1",
        "glassworks.viewctl.layout.library_macro.preset.clear.1",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "library controls should expose {node_name}"
        );
    }

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.library_macro.via_array.columns.inc"
        )
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.rows.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.size.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.pitch.inc")
    );
    assert_eq!(app.layout_library_via_array_columns, 4);
    assert_eq!(app.layout_library_via_array_rows, 4);
    assert_eq!(app.layout_library_via_array_size_grids, 20);
    assert_eq!(app.layout_library_via_array_pitch_grids, 32);
    assert!(
        layout_cell_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Library via array" && value.contains("4x4")),
        "cell browser details should expose current library parameters"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.create")
    );
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x4"))
        .expect("macro should create a named parameterized library cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 16);
    assert_eq!(
        created_cell
            .properties
            .get("library.columns")
            .map(String::as_str),
        Some("4")
    );
    assert_eq!(
        created_cell
            .properties
            .get("library.rows")
            .map(String::as_str),
        Some("4")
    );
    let library_rows = layout_cell_property_rows(&app, &created_cell);
    assert!(
        library_rows
            .iter()
            .any(|(key, value)| key == "Macro parameters" && value.contains("4x4")),
        "cell properties should expose stored macro parameters"
    );
    let mut xs = BTreeSet::new();
    let mut ys = BTreeSet::new();
    for shape in created_cell.shapes.values() {
        let ShapeKind::Via { center, size, .. } = &shape.kind else {
            panic!("parameterized via array should only contain via shapes");
        };
        assert_eq!(*size, 200);
        xs.insert(center.x);
        ys.insert(center.y);
    }
    let xs = xs.into_iter().collect::<Vec<_>>();
    let ys = ys.into_iter().collect::<Vec<_>>();
    assert_eq!(xs.len(), 4);
    assert_eq!(ys.len(), 4);
    assert!(
        xs.windows(2).all(|window| window[1] - window[0] == 320),
        "x pitch should follow the edited parameter"
    );
    assert!(
        ys.windows(2).all(|window| window[1] - window[0] == 320),
        "y pitch should follow the edited parameter"
    );
    let top_cell = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist");
    assert!(
        top_cell
            .instances
            .values()
            .any(|instance| instance.cell == created_cell.id),
        "parameterized library cell should be placed in the top cell"
    );
    assert!(app.status_message().contains("16-via"));

    let old_shape_ids = created_cell.shapes.keys().copied().collect::<BTreeSet<_>>();
    let cells_after_create = app.workspace.document.cells.len();
    let instances_after_create = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.reset"));
    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.library_macro.via_array.load_selected"
    ));
    assert_eq!(app.layout_library_via_array_columns, 4);
    assert_eq!(app.layout_library_via_array_rows, 4);
    assert_eq!(app.layout_library_via_array_size_grids, 20);
    assert_eq!(app.layout_library_via_array_pitch_grids, 32);

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.library_macro.via_array.columns.inc"
        )
    );
    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.library_macro.via_array.update_selected"
    ));
    assert_eq!(app.workspace.document.cells.len(), cells_after_create);
    assert_eq!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should exist")
            .instances
            .len(),
        instances_after_create
    );
    let updated_cell = app
        .workspace
        .document
        .cell(created_cell.id)
        .expect("update should preserve the generated cell id");
    assert_eq!(updated_cell.shapes.len(), 20);
    assert_eq!(
        updated_cell
            .properties
            .get("library.columns")
            .map(String::as_str),
        Some("5")
    );
    assert!(
        old_shape_ids
            .iter()
            .all(|shape_id| !updated_cell.shapes.contains_key(shape_id)),
        "updating a macro cell should replace old local generated geometry"
    );
    assert!(
        app.selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| !occurrence.instance_path.is_empty()),
        "updated generated shape selection should stay on the placed instance occurrence"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_cell = app
        .workspace
        .document
        .cell(created_cell.id)
        .expect("undo should keep the generated cell");
    assert_eq!(restored_cell.shapes.len(), 16);
    assert_eq!(
        restored_cell
            .properties
            .get("library.columns")
            .map(String::as_str),
        Some("4")
    );
}

#[test]
pub(crate) fn layout_library_macro_presets_persist_and_place_reusable_cells() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro preset test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.library_macro.via_array.columns.inc"
        )
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.rows.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.size.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.pitch.inc")
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.save.2"));
    assert!(
        app.status_message().contains("Saved library preset 2"),
        "{}",
        app.status_message()
    );
    assert!(
        app.layout_via_array_library_preset_label(2).contains("4x4"),
        "preset label should summarize saved parameters"
    );
    let saved_options = app.app_options().clone();
    let saved_preset = saved_options
        .layout
        .library_presets
        .iter()
        .find(|preset| preset.slot == 2)
        .expect("library preset should persist to app options");
    assert_eq!(saved_preset.columns, 4);
    assert_eq!(saved_preset.rows, 4);
    assert_eq!(saved_preset.size_grids, 20);
    assert_eq!(saved_preset.pitch_grids, 32);

    let mut restored = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        app_options: Some(saved_options),
        ..Default::default()
    });
    restored.workspace.document = Document::new("restored library macro preset test");
    restored.reset_layout_document_state();
    assert!(
        restored.layout_library_via_array_presets.contains_key(&2),
        "library preset should survive app-options restore"
    );
    assert!(
        restored.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.reset")
    );
    assert_eq!(restored.layout_library_via_array_columns, 3);
    assert!(
        restored.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.load.2")
    );
    assert_eq!(restored.layout_library_via_array_columns, 4);
    assert_eq!(restored.layout_library_via_array_rows, 4);
    assert_eq!(restored.layout_library_via_array_size_grids, 20);
    assert_eq!(restored.layout_library_via_array_pitch_grids, 32);

    let cells_before = restored.workspace.document.cells.len();
    let instances_before = restored
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();
    assert!(
        restored.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.create.2")
    );
    assert_eq!(restored.workspace.document.cells.len(), cells_before + 1);
    let created_cell = restored
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x4"))
        .expect("preset should place a parameterized via-array cell");
    assert_eq!(created_cell.shapes.len(), 16);
    assert_eq!(
        restored
            .workspace
            .document
            .cell(top)
            .expect("top cell should exist")
            .instances
            .len(),
        instances_before + 1
    );

    assert!(
        restored.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.clear.2")
    );
    assert!(!restored.layout_library_via_array_presets.contains_key(&2));
    assert!(
        !restored
            .app_options()
            .layout
            .library_presets
            .iter()
            .any(|preset| preset.slot == 2),
        "cleared preset should be removed from app options"
    );
}

#[test]
pub(crate) fn layout_library_macro_catalog_exchange_round_trips_presets() {
    let path = default_ui_library_catalog_exchange_path();
    let _ = std::fs::remove_file(&path);

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro catalog test");
    app.reset_layout_document_state();

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    for node_name in [
        "glassworks.viewctl.layout.library_macro.catalog.export",
        "glassworks.viewctl.layout.library_macro.catalog.import",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "library catalog controls should expose {node_name}"
        );
    }

    assert!(
        app.apply_clicked_node_name(
            "glassworks.viewctl.layout.library_macro.via_array.columns.inc"
        )
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.rows.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.size.inc")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array.pitch.inc")
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.save.2"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.catalog.export"));
    assert!(
        app.status_message().contains("Exported library catalog"),
        "{}",
        app.status_message()
    );

    let exchange_contents =
        std::fs::read_to_string(&path).expect("library catalog export should be written");
    let exchange = serde_json::from_str::<LayoutLibraryCatalogExchange>(&exchange_contents)
        .expect("library catalog exchange should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_LIBRARY_CATALOG_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.document_name, "library macro catalog test");
    assert_eq!(exchange.presets.len(), 1);
    let exported_preset = &exchange.presets[0];
    assert_eq!(exported_preset.slot, 2);
    assert_eq!(exported_preset.macro_name, "via_array");
    assert_eq!(exported_preset.columns, 4);
    assert_eq!(exported_preset.rows, 4);
    assert_eq!(exported_preset.size_grids, 20);
    assert_eq!(exported_preset.pitch_grids, 32);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    imported.workspace.document = Document::new("imported library macro catalog test");
    imported.reset_layout_document_state();
    assert!(!imported.layout_library_via_array_presets.contains_key(&2));
    assert!(
        imported.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.catalog.import")
    );
    assert!(
        imported
            .status_message()
            .contains("Imported library catalog"),
        "{}",
        imported.status_message()
    );

    let preset = imported
        .layout_library_via_array_presets
        .get(&2)
        .expect("import should restore preset 2");
    assert_eq!(preset.columns, 4);
    assert_eq!(preset.rows, 4);
    assert_eq!(preset.size_grids, 20);
    assert_eq!(preset.pitch_grids, 32);
    assert!(
        imported
            .app_options()
            .layout
            .library_presets
            .iter()
            .any(|preset| {
                preset.slot == 2
                    && preset.columns == 4
                    && preset.rows == 4
                    && preset.size_grids == 20
                    && preset.pitch_grids == 32
            }),
        "imported catalog should sync to app options"
    );

    let cells_before = imported.workspace.document.cells.len();
    assert!(
        imported.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.preset.create.2")
    );
    assert_eq!(imported.workspace.document.cells.len(), cells_before + 1);
    let created_cell = imported
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x4"))
        .expect("imported preset should place a via-array library cell");
    assert_eq!(created_cell.shapes.len(), 16);

    let _ = std::fs::remove_file(&path);
}

#[test]
pub(crate) fn layout_library_macro_can_be_detached_to_static_cell() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro static test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.via_array_3x3"));
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 3x3"))
        .expect("macro should create a named library cell")
        .clone();
    let top_instances_before = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.library_macro.make_static"),
        "library controls should expose static conversion"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.library_macro.make_static"));
    let static_cell = app
        .workspace
        .document
        .cell(created_cell.id)
        .expect("static conversion should keep the generated cell");
    assert_eq!(static_cell.shapes.len(), 9);
    assert!(
        !static_cell
            .properties
            .keys()
            .any(|key| key.starts_with("library.")),
        "static conversion should clear macro metadata"
    );
    assert!(!layout_cell_is_library_generated(static_cell));
    assert_eq!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should still exist")
            .instances
            .len(),
        top_instances_before
    );
    assert!(
        app.selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| !occurrence.instance_path.is_empty()),
        "static conversion should keep the placed occurrence selected"
    );
    app.layout_cell_browser_filter = LayoutCellBrowserFilter::Library;
    assert!(
        layout_cell_browser_cells(&app).is_empty(),
        "static conversion should remove the cell from the library filter"
    );
    assert!(app.status_message().contains("normal static cell"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let restored_cell = app
        .workspace
        .document
        .cell(created_cell.id)
        .expect("undo should keep the generated cell");
    assert_eq!(
        restored_cell
            .properties
            .get("library.macro")
            .map(String::as_str),
        Some("via_array")
    );
    assert!(layout_cell_is_library_generated(restored_cell));
}

#[test]
pub(crate) fn layout_library_macro_converts_current_cell_rectangle_guide() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro current-cell guide test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let child = app.workspace.document.create_cell("guide_child");
    let guide_bounds = Rect::from_min_size(Point::new(1_000, 2_000), 1_200, 900);
    let guide_id = app
        .workspace
        .document
        .insert_shape_in_cell(child, metal1, ShapeKind::Rectangle(guide_bounds))
        .expect("current-cell guide rectangle should be added");
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(10_000, 0))
        .expect("child should be placed");
    app.layout_view_top_cell = child;
    app.selected_layout_shape = Some(guide_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(guide_id));
    let cells_before = app.workspace.document.cells.len();
    let child_instances_before = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should exist")
        .instances
        .len();

    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.library_macro.via_array.from_selection"
    ));
    let child_cell = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should remain after conversion");
    assert!(
        !child_cell.shapes.contains_key(&guide_id),
        "current-cell guide rectangle should be replaced by a macro instance"
    );
    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    assert_eq!(app.layout_library_via_array_columns, 4);
    assert_eq!(app.layout_library_via_array_rows, 3);
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x3"))
        .expect("current-cell guide conversion should create a named via-array cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 12);
    assert_eq!(child_cell.instances.len(), child_instances_before + 1);
    let created_instance = child_cell
        .instances
        .values()
        .find(|instance| instance.cell == created_cell.id)
        .expect("converted cell should be placed in the current child cell");
    assert_eq!(
        created_instance.transform,
        Transform::translate(1_600, 2_450)
    );
    assert!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should remain after conversion")
            .instances
            .values()
            .all(|instance| instance.cell != created_cell.id),
        "current-cell guide conversion should not place the macro under the document top"
    );
    assert_eq!(app.layout_view_top_cell, child);
    assert!(
        app.selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| occurrence.instance_path == vec![created_instance.id]),
        "conversion should select a shape occurrence inside the placed current-cell macro instance"
    );
    assert!(app.status_message().contains("Converted"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    let child_cell = app
        .workspace
        .document
        .cell(child)
        .expect("child cell should remain after undo");
    assert!(
        child_cell.shapes.contains_key(&guide_id),
        "undo should restore the current-cell guide rectangle"
    );
    assert!(
        app.workspace.document.cell(created_cell.id).is_none(),
        "undo should remove the generated macro cell"
    );
    assert_eq!(child_cell.instances.len(), child_instances_before);
}

#[test]
pub(crate) fn layout_library_macro_converts_selected_rectangle_guide() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro guide test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let guide_bounds = Rect::from_min_size(Point::new(1_000, 2_000), 1_200, 900);
    let guide_id = app
        .add_layout_shape(metal1, ShapeKind::Rectangle(guide_bounds))
        .expect("guide rectangle should be added");
    let cells_before = app.workspace.document.cells.len();
    let instances_before = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.library_macro.via_array.from_selection"
        }),
        "library controls should expose guide-shape conversion"
    );

    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.library_macro.via_array.from_selection"
    ));
    assert!(
        !app.workspace.document.shapes.contains_key(&guide_id),
        "guide rectangle should be replaced by a macro instance"
    );
    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    assert_eq!(app.layout_library_via_array_columns, 4);
    assert_eq!(app.layout_library_via_array_rows, 3);
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x3"))
        .expect("guide conversion should create a named via-array cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 12);
    assert_eq!(
        created_cell
            .properties
            .get("library.columns")
            .map(String::as_str),
        Some("4")
    );
    assert_eq!(
        created_cell
            .properties
            .get("library.rows")
            .map(String::as_str),
        Some("3")
    );
    let top_cell = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should remain after conversion");
    assert_eq!(top_cell.instances.len(), instances_before + 1);
    let created_instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == created_cell.id)
        .expect("converted cell should be placed in the top cell");
    assert_eq!(
        created_instance.transform,
        Transform::translate(1_600, 2_450)
    );
    assert!(
        app.selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| occurrence.instance_path == vec![created_instance.id]),
        "conversion should select a shape occurrence inside the placed macro instance"
    );
    assert!(app.status_message().contains("Converted"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.shapes.contains_key(&guide_id),
        "undo should restore the guide rectangle"
    );
    assert!(
        app.workspace.document.cell(created_cell.id).is_none(),
        "undo should remove the generated macro cell"
    );
    assert_eq!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should remain after undo")
            .instances
            .len(),
        instances_before
    );
}

#[test]
pub(crate) fn layout_library_macro_converts_selected_polygon_guide() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("library macro polygon guide test");
    app.reset_layout_document_state();
    let top = app.workspace.document.top_cell;
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let guide_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(200, 1_000),
                Point::new(1_500, 1_000),
                Point::new(1_200, 1_650),
                Point::new(200, 1_650),
            ])),
        )
        .expect("guide polygon should be added");
    let cells_before = app.workspace.document.cells.len();
    let instances_before = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should exist")
        .instances
        .len();

    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.library_macro.via_array.from_selection"
    ));

    assert!(
        !app.workspace.document.shapes.contains_key(&guide_id),
        "guide polygon should be replaced by a macro instance"
    );
    assert_eq!(app.workspace.document.cells.len(), cells_before + 1);
    assert_eq!(app.layout_library_via_array_columns, 4);
    assert_eq!(app.layout_library_via_array_rows, 2);
    let created_cell = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.name.contains("lib via array 4x2"))
        .expect("polygon guide conversion should create a named via-array cell")
        .clone();
    assert_eq!(created_cell.shapes.len(), 8);
    let top_cell = app
        .workspace
        .document
        .cell(top)
        .expect("top cell should remain after conversion");
    assert_eq!(top_cell.instances.len(), instances_before + 1);
    let created_instance = top_cell
        .instances
        .values()
        .find(|instance| instance.cell == created_cell.id)
        .expect("converted cell should be placed in the top cell");
    assert_eq!(created_instance.transform, Transform::translate(850, 1_330));
    assert!(app.status_message().contains("Converted"));

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace.document.shapes.contains_key(&guide_id),
        "undo should restore the guide polygon"
    );
    assert!(
        app.workspace.document.cell(created_cell.id).is_none(),
        "undo should remove the generated macro cell"
    );
    assert_eq!(
        app.workspace
            .document
            .cell(top)
            .expect("top cell should remain after undo")
            .instances
            .len(),
        instances_before
    );
}

#[test]
pub(crate) fn layout_shape_browser_selects_visible_occurrences() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        hierarchy_demo: true,
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("hierarchy demo should have metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2_000, -500), 200, 120)),
    )
    .expect("test top-level shape should be added");
    app.layout_shape_browser_sort = LayoutShapeBrowserSort::Cell;
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.full"));
    let (occurrence, _) = layout_shape_browser_entries(&app)
        .into_iter()
        .find(|(occurrence, _)| !occurrence.is_top_level())
        .expect("hierarchy demo should expose nested shape browser entries");
    assert!(
        layout_shape_browser_entries(&app)
            .iter()
            .any(|(occurrence, _)| occurrence.is_top_level()),
        "full hierarchy shape browser should include current-root geometry before min-depth filtering"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_min.1"));
    assert!(
        layout_shape_browser_entries(&app)
            .iter()
            .all(|(occurrence, _)| occurrence.hierarchy_depth() >= 1),
        "minimum hierarchy depth should hide current-root shape-browser rows"
    );
    let action = format!(
        "glassworks.viewctl.layout.occurrence.{}",
        layout_occurrence_action_key(&occurrence)
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.shape_browser.title"),
        "layout inspector should expose a shape browser"
    );
    assert!(
        document.nodes().iter().any(|node| node.name() == action),
        "shape browser should expose {action}"
    );

    assert!(app.apply_clicked_node_name(&action));
    assert_eq!(app.selected_layout_occurrence, Some(occurrence));
}

#[test]
pub(crate) fn layout_shape_browser_searches_by_selector_metadata() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("shape selector search test");
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
    app.workspace
        .document
        .layers
        .get_mut(&metal2)
        .expect("metal2 layer should be mutable")
        .name = "selector metal2".to_string();
    let zeta = app.workspace.document.create_cell("zeta shape source");
    let alpha = app.workspace.document.create_cell("alpha shape source");
    let zeta_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            zeta,
            metal1,
            ShapeKind::Label {
                position: Point::new(0, 0),
                text: "zeta_pin".to_string(),
            },
        )
        .expect("zeta source shape should be inserted");
    let alpha_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            alpha,
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 80, 80)),
        )
        .expect("alpha source shape should be inserted");
    {
        let mut shape = app
            .workspace
            .document
            .cell_mut(alpha)
            .expect("alpha cell should be mutable")
            .shapes
            .get_mut(&alpha_shape)
            .expect("alpha shape should be mutable");
        shape.name = Some("alpha_contact".to_string());
        shape.net = Some(NetId(9));
    }
    app.workspace
        .document
        .insert_instance(top, zeta, Transform::translate(0, 0))
        .expect("zeta source instance should be inserted");
    app.workspace
        .document
        .insert_instance(top, alpha, Transform::translate(1_000, 0))
        .expect("alpha source instance should be inserted");

    let listed_shapes = |app: &GlassworksApp| {
        layout_shape_browser_entry_shapes_for_filter(
            app,
            LayoutShapeBrowserFilter::All,
            true,
            usize::MAX,
        )
        .into_iter()
        .map(|(_, source_cell, shape)| (source_cell, shape.id))
        .collect::<Vec<_>>()
    };

    app.set_layout_browser_search("source_cell=alpha shape");
    assert_eq!(listed_shapes(&app), vec![(alpha, alpha_shape)]);
    app.set_layout_browser_search(&format!("source_cell_id={}", zeta.0));
    assert_eq!(listed_shapes(&app), vec![(zeta, zeta_shape)]);
    app.set_layout_browser_search("layer=selector metal2");
    assert_eq!(listed_shapes(&app), vec![(alpha, alpha_shape)]);
    app.set_layout_browser_search("kind=label");
    assert_eq!(listed_shapes(&app), vec![(zeta, zeta_shape)]);
    app.set_layout_browser_search("name=alpha_contact");
    assert_eq!(listed_shapes(&app), vec![(alpha, alpha_shape)]);
    app.set_layout_browser_search("text=zeta_pin");
    assert_eq!(listed_shapes(&app), vec![(zeta, zeta_shape)]);
    app.set_layout_browser_search("net=N9");
    assert_eq!(listed_shapes(&app), vec![(alpha, alpha_shape)]);

    app.set_layout_browser_search("source_cell=alpha shape");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with source-cell shape search should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.shape_browser.select_first"),
        "shape browser should expose select-first control for source-cell selector search"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser.select_first"));
    assert_eq!(app.selected_layout_shape, Some(alpha_shape));
    let selected = app
        .selected_layout_occurrence
        .as_ref()
        .expect("source-cell selector should select a visible occurrence");
    let view = app
        .workspace
        .document
        .shape_view_for_occurrence_from_cell(app.layout_view_top_cell, selected)
        .expect("selected source-cell occurrence should resolve");
    assert_eq!(view.source_cell, alpha);
}

#[test]
pub(crate) fn layout_shape_browser_filters_by_shape_kind() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("shape kind browser filters");
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
    let via1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Via1)
        .expect("default technology should include via1");
    let annotation = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Annotation)
        .unwrap_or(metal1);

    let rectangle = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(-2_000, 0), 300, 200)),
        )
        .expect("rectangle should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&rectangle)
        .expect("rectangle should exist")
        .name = Some("named-rectangle".to_string());
    app.add_layout_shape(
        metal1,
        ShapeKind::Polygon(Polygon::new(vec![
            Point::new(-1_000, 0),
            Point::new(-700, 0),
            Point::new(-800, 250),
        ])),
    )
    .expect("polygon should be added");
    let path = app
        .add_layout_shape(
            metal2,
            ShapeKind::Path {
                points: vec![Point::new(0, 0), Point::new(500, 0)],
                width: 120,
            },
        )
        .expect("path should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&path)
        .expect("path should exist")
        .net = Some(NetId(7));
    app.add_layout_shape(
        via1,
        ShapeKind::Via {
            center: Point::new(900, 0),
            size: 180,
            lower: metal1,
            upper: metal2,
        },
    )
    .expect("via should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(1_400, 0),
            text: "label-filter".to_string(),
        },
    )
    .expect("label should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Measurement {
            a: Point::new(1_800, 0),
            b: Point::new(2_100, 400),
            label: "measurement-filter".to_string(),
            mode: MeasurementMode::Direct,
        },
    )
    .expect("measurement should be added");

    app.selected_layout_shape = Some(rectangle);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(rectangle));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_filter.selected"));
    let selected_entries = layout_shape_browser_entries(&app);
    assert_eq!(selected_entries.len(), 1);
    assert_eq!(
        selected_entries[0].0,
        ShapeOccurrenceId::top_level(rectangle)
    );
    assert!(
        layout_shape_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Selected"),
        "shape browser rows should show the selected filter"
    );

    for (slug, expected, filter_label) in [
        ("rectangles", "rectangle", "Rectangles"),
        ("polygons", "polygon", "Polygons"),
        ("paths", "path", "Paths"),
        ("vias", "via", "Vias"),
        ("labels", "label", "Labels"),
        ("measurements", "measurement", "Measurements"),
    ] {
        assert!(app.apply_clicked_node_name(&format!(
            "glassworks.viewctl.layout.shape_browser_filter.{slug}"
        )));
        let entries = layout_shape_browser_entries(&app);
        assert_eq!(entries.len(), 1, "{slug} filter should list one shape");
        let view = app
            .workspace
            .document
            .shape_view_for_occurrence_from_cell(app.layout_view_top_cell, &entries[0].0)
            .expect("filtered entry should reference a visible shape");
        assert_eq!(shape_kind_label(&view.shape.to_shape().kind), expected);
        assert!(
            layout_shape_browser_rows(&app)
                .iter()
                .any(|(key, value)| key == "Filter" && value == filter_label),
            "shape browser rows should show the active {expected} filter"
        );
    }

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_filter.properties")
    );
    let property_entries = layout_shape_browser_entries(&app);
    assert_eq!(
        property_entries.len(),
        2,
        "property filter should list named and net-tagged shapes"
    );
    assert!(
        layout_shape_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Filter" && value == "Properties"),
        "shape browser rows should show the property filter"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with shape kind filters should build");
    for node_name in [
        "glassworks.viewctl.layout.shape_browser_filter.selected",
        "glassworks.viewctl.layout.shape_browser_filter.properties",
        "glassworks.viewctl.layout.shape_browser_filter.rectangles",
        "glassworks.viewctl.layout.shape_browser_filter.polygons",
        "glassworks.viewctl.layout.shape_browser_filter.paths",
        "glassworks.viewctl.layout.shape_browser_filter.vias",
        "glassworks.viewctl.layout.shape_browser_filter.labels",
        "glassworks.viewctl.layout.shape_browser_filter.measurements",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "shape browser kind filter control should exist: {node_name}"
        );
    }
    let title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.shape_browser.title")
        .unwrap_or_else(|| panic!("shape browser title should exist"));
    let UiContent::Text(title_text) = title.content() else {
        panic!("shape browser title should be text");
    };
    assert_eq!(title_text.text, "Shape Browser - Properties (2 rows)");
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
        filter_button_label("glassworks.viewctl.layout.shape_browser_filter.all"),
        "All (6)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.shape_browser_filter.selected"),
        "Selected (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.shape_browser_filter.properties"),
        "Properties (2)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.shape_browser_filter.rectangles"),
        "Rectangles (1)"
    );
    assert_eq!(
        filter_button_label("glassworks.viewctl.layout.shape_browser_filter.measurements"),
        "Measurements (1)"
    );

    app.layout_browser_search = "missing".to_string();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser.select_first"));
    assert!(
        app.status_message()
            .contains("Shape browser has no matching property-bearing shapes for search missing"),
        "{}",
        app.status_message()
    );
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty shape search should build");
    let empty_title = empty_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.shape_browser.title")
        .unwrap_or_else(|| panic!("empty shape browser title should exist"));
    let UiContent::Text(empty_title_text) = empty_title.content() else {
        panic!("empty shape browser title should be text");
    };
    assert_eq!(
        empty_title_text.text,
        "Shape Browser - Properties / search missing (0 rows)"
    );
}

#[test]
pub(crate) fn layout_shape_browser_sorts_by_area() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("shape browser area sort");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let small = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 20, 20)),
        )
        .expect("small shape should be added");
    let large = app
        .add_layout_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(100, 0),
                Point::new(200, 0),
                Point::new(200, 80),
                Point::new(100, 80),
            ])),
        )
        .expect("large shape should be added");
    let medium = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 60, 40)),
        )
        .expect("medium shape should be added");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_sort.area"));
    assert_eq!(app.layout_shape_browser_sort, LayoutShapeBrowserSort::Area);
    let sorted = layout_shape_browser_entries(&app)
        .into_iter()
        .map(|(occurrence, _)| occurrence.source_shape_id())
        .collect::<Vec<_>>();
    assert_eq!(sorted, vec![large, medium, small]);
    assert!(
        layout_shape_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Area"),
        "shape browser rows should show area sort"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with area sort should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.shape_browser_sort.area"),
        "shape browser sort surface should expose area sort"
    );
}

#[test]
pub(crate) fn layout_shape_browser_exposes_stored_shape_properties() {
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
    app.workspace.document.ensure_hierarchy();
    app.layout_view_top_cell = app.workspace.document.top_cell;
    app.workspace
        .document
        .layers
        .get_mut(&metal1)
        .expect("metal1 layer should be mutable")
        .name = "reticle_metal".to_string();

    let shape_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 160, 80)),
        )
        .expect("test shape should be added");
    {
        let mut shape = app
            .workspace
            .document
            .shapes
            .get_mut(&shape_id)
            .expect("test shape should be mutable");
        shape.name = Some("property-bearing-shape".to_string());
        shape.properties.insert(
            "custom.browser.kind".to_string(),
            "alignment_guide".to_string(),
        );
        shape
            .properties
            .insert("mask.owner".to_string(), "reticle A".to_string());
    }
    let other_shape_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 160, 80)),
        )
        .expect("second property shape should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&other_shape_id)
        .expect("second property shape should be mutable")
        .properties
        .insert("mask.owner".to_string(), "opc".to_string());
    app.selected_layout_shape = Some(shape_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(shape_id));

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_filter.properties")
    );
    let property_entries = layout_shape_browser_entries(&app);
    assert_eq!(
        property_entries.len(),
        2,
        "stored properties should qualify a shape for the property filter"
    );
    assert!(
        property_entries
            .iter()
            .any(|(occurrence, _)| *occurrence == ShapeOccurrenceId::top_level(shape_id))
    );
    assert!(
        property_entries
            .iter()
            .any(|(occurrence, _)| *occurrence == ShapeOccurrenceId::top_level(other_shape_id))
    );

    app.set_layout_browser_search("reticle");
    let searched_entries = layout_shape_browser_entries(&app);
    assert_eq!(
        searched_entries.len(),
        1,
        "Properties filter search should match property values but not layer names"
    );
    assert_eq!(
        searched_entries[0].0,
        ShapeOccurrenceId::top_level(shape_id)
    );
    app.set_layout_browser_search("mask.owner=opc");
    let searched_entries = layout_shape_browser_entries(&app);
    assert_eq!(
        searched_entries.len(),
        1,
        "Properties filter should support key=value selectors"
    );
    assert_eq!(
        searched_entries[0].0,
        ShapeOccurrenceId::top_level(other_shape_id)
    );
    app.set_layout_browser_search("custom.browser.kind=alignment");
    let searched_entries = layout_shape_browser_entries(&app);
    assert_eq!(
        searched_entries.len(),
        1,
        "Properties filter should support partial key=value selectors"
    );
    assert_eq!(
        searched_entries[0].0,
        ShapeOccurrenceId::top_level(shape_id)
    );

    let rows = layout_shape_browser_rows(&app);
    for (key, value) in [
        ("Name", "property-bearing-shape"),
        ("Properties", "2 stored"),
        ("Property custom.browser.kind", "alignment_guide"),
        ("Property mask.owner", "reticle A"),
    ] {
        assert!(
            rows.iter()
                .any(|(row_key, row_value)| row_key == key && row_value == value),
            "shape browser should expose {key}={value}: {rows:?}"
        );
    }
}

#[test]
pub(crate) fn layout_shape_browser_exposes_kind_specific_object_rows() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("shape browser object rows");
    app.reset_layout_document_state();
    app.layout_browser_columns = LayoutBrowserColumnSet::All;
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
    let via1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Via1)
        .expect("default technology should include via1");
    let annotation = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Annotation)
        .unwrap_or(metal1);

    let rectangle = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 160, 80)),
        )
        .expect("rectangle should be added");
    let polygon = app
        .add_layout_shape(
            metal1,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 50),
                Point::new(0, 50),
            ])),
        )
        .expect("polygon should be added");
    let path = app
        .add_layout_shape(
            metal2,
            ShapeKind::Path {
                points: vec![Point::new(0, 0), Point::new(300, 400)],
                width: 120,
            },
        )
        .expect("path should be added");
    let via = app
        .add_layout_shape(
            via1,
            ShapeKind::Via {
                center: Point::new(900, 120),
                size: 180,
                lower: metal1,
                upper: metal2,
            },
        )
        .expect("via should be added");
    let label = app
        .add_layout_shape(
            annotation,
            ShapeKind::Label {
                position: Point::new(1_400, 50),
                text: "pin_a".to_string(),
            },
        )
        .expect("label should be added");

    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(rectangle));
    let rows = layout_shape_browser_rows(&app);
    assert_browser_row(&rows, "Area", "12800 dbu^2");

    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(polygon));
    let rows = layout_shape_browser_rows(&app);
    assert_browser_row(&rows, "Points", "4");
    assert_browser_row(&rows, "Area", "5000 dbu^2");

    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(path));
    let rows = layout_shape_browser_rows(&app);
    assert_browser_row(&rows, "Points", "2");
    assert_browser_row(&rows, "Width", &app.format_layout_length(120.0));
    assert_browser_row(&rows, "Length", &app.format_layout_length(500.0));

    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(via));
    let rows = layout_shape_browser_rows(&app);
    assert_browser_row(&rows, "Center", "900,120");
    assert_browser_row(&rows, "Size", &app.format_layout_length(180.0));
    assert_browser_row(
        &rows,
        "Layers",
        &format!(
            "{} -> {}",
            layout_layer_display_name(&app.workspace.document, metal1),
            layout_layer_display_name(&app.workspace.document, metal2)
        ),
    );

    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(label));
    let rows = layout_shape_browser_rows(&app);
    assert_browser_row(&rows, "Position", "1400,50");
    assert_browser_row(&rows, "Text", "pin_a");
}

#[test]
pub(crate) fn layout_shape_browser_select_first_uses_property_selector_search() {
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
    let first = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 160, 80)),
        )
        .expect("first property shape should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&first)
        .expect("first property shape should be mutable")
        .properties
        .insert("mask.owner".to_string(), "reticle".to_string());
    let second = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(300, 0), 160, 80)),
        )
        .expect("second property shape should be added");
    app.workspace
        .document
        .shapes
        .get_mut(&second)
        .expect("second property shape should be mutable")
        .properties
        .insert("mask.owner".to_string(), "opc".to_string());

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser_filter.properties")
    );
    app.set_layout_browser_search("mask.owner=opc");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with property-filtered shape browser should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.shape_browser.select_first"),
        "shape browser should expose select-first control when property matches exist"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.shape_browser.select_first"));
    assert_eq!(app.selected_layout_shape, Some(second));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(second))
    );
    assert_eq!(app.active_layer, metal1);
    assert!(
        app.status_message().contains("Selected browser shape"),
        "{}",
        app.status_message()
    );
}
