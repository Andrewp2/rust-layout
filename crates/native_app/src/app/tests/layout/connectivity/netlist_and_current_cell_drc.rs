#![allow(unused_imports)]
use super::*;
use crate::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_netlist_export_import_round_trips_connectivity() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("netlist export source");
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
        .expect("default technology should include annotations");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(800, 300))),
    )
    .expect("connected metal should be added");
    app.workspace.document.insert_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "clk".to_string(),
        },
    );
    app.mark_layout_dirty();
    let report = app
        .connectivity_report()
        .expect("connectivity should extract before export");
    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].net_name.as_deref(), Some("CLK"));

    let path =
        std::env::temp_dir().join(format!("glassworks-netlist-menu-{}.json", std::process::id()));
    let spice_path = std::env::temp_dir().join(format!(
        "glassworks-netlist-menu-{}.spice",
        std::process::id()
    ));
    let schematic_path = std::env::temp_dir().join(format!(
        "glassworks-schematic-menu-{}.spice",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&spice_path);
    let _ = std::fs::remove_file(&schematic_path);

    assert!(app.export_layout_netlist_to_path(&path));
    assert!(
        app.status_message().contains("Exported netlist"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("netlist export should write JSON");
    let exchange: LayoutExtractedNetlistExchange =
        serde_json::from_str(&contents).expect("exported netlist should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.components.len(), 1);
    assert_eq!(exchange.components[0].net_name.as_deref(), Some("CLK"));

    assert!(app.export_layout_spice_netlist_to_path(&spice_path));
    assert!(
        app.status_message().contains("Exported SPICE netlist"),
        "{}",
        app.status_message()
    );
    let spice =
        std::fs::read_to_string(&spice_path).expect("SPICE netlist export should be readable");
    assert!(spice.contains(".subckt NETLIST_EXPORT_SOURCE CLK"));
    assert!(spice.contains("* component 1 net=CLK"));
    assert!(spice.ends_with(".end\n"));

    std::fs::write(
        &schematic_path,
        ".subckt netlist_export_source CLK\n.ends netlist_export_source\n",
    )
    .expect("schematic SPICE fixture should be written");
    assert!(app.compare_layout_spice_schematic_from_path(&schematic_path));
    assert!(
        app.status_message().contains("matches layout connectivity"),
        "{}",
        app.status_message()
    );
    let comparison = app
        .layout_spice_comparison
        .as_ref()
        .expect("schematic compare should cache a report");
    assert!(comparison.is_match());
    assert_eq!(comparison.schematic_device_count, 0);
    let net_rows = layout_net_browser_rows(&app);
    assert!(
        net_rows
            .iter()
            .any(|(key, value)| key == "SPICE Compare" && value == "match"),
        "net browser rows should expose SPICE compare status: {net_rows:?}"
    );
    assert!(
        net_rows
            .iter()
            .any(|(key, value)| key == "SPICE Net" && value == "Matched schematic net"),
        "net browser rows should expose selected-net SPICE status: {net_rows:?}"
    );

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    imported.workspace.document = Document::new("netlist import target");
    imported.reset_layout_document_state();
    imported.connectivity_report_cache.borrow_mut().take();
    assert!(imported.import_layout_netlist_from_path(&path));
    assert!(
        imported.status_message().contains("Imported netlist"),
        "{}",
        imported.status_message()
    );
    let imported_report = imported
        .connectivity_report()
        .expect("imported netlist should populate the connectivity cache");
    assert_eq!(imported_report.components.len(), 1);
    assert_eq!(
        imported_report.components[0].net_name.as_deref(),
        Some("CLK")
    );
    assert_eq!(layout_net_browser_entries(&imported).len(), 1);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&spice_path);
    let _ = std::fs::remove_file(&schematic_path);

    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    let tools = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("tools menu should build");
    for node_name in [
        "glassworks.menu.item.layout.netlist_export",
        "glassworks.menu.item.layout.netlist_import",
        "glassworks.menu.item.layout.spice_netlist_export",
        "glassworks.menu.item.layout.spice_schematic_compare",
    ] {
        let node = tools
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Tools menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }

    let browser = imported
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("imported netlist browser should build");
    for node_name in [
        "glassworks.viewctl.layout.netlist_export",
        "glassworks.viewctl.layout.netlist_import",
        "glassworks.viewctl.layout.spice_netlist_export",
        "glassworks.viewctl.layout.spice_schematic_compare",
    ] {
        assert!(
            browser.nodes().iter().any(|node| node.name() == node_name),
            "Net browser should expose {node_name}"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_trace_state_export_import_round_trips_history() {
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

    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-1_000, -200), Point::new(200, 200))),
    )
    .expect("first connected shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(5_000, -200), Point::new(5_800, 200))),
    )
    .expect("second connected shape should be added");
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    let mut component_ids = report
        .components
        .iter()
        .map(|component| component.id)
        .collect::<Vec<_>>();
    component_ids.sort_unstable();
    assert_eq!(component_ids.len(), 2);
    let selected_component = component_ids[1];
    assert!(app.select_layout_net_component(&selected_component.to_string()));
    let other_component = component_ids[0];
    app.record_layout_trace_history(other_component);
    let expected_history = vec![other_component, selected_component];
    assert_eq!(app.layout_trace_history, expected_history);
    app.route_points = vec![Point::new(-500, 0), Point::new(5_400, 0)];

    let path = std::env::temp_dir().join(format!(
        "glassworks-trace-state-menu-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_layout_trace_state_to_path(&path));
    assert!(
        app.status_message().contains("Exported trace state"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("trace state export should write JSON");
    let exchange: LayoutTraceStateExchange =
        serde_json::from_str(&contents).expect("exported trace state should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.selected_component, Some(selected_component));
    assert_eq!(exchange.history, expected_history);
    assert_eq!(exchange.route_points, app.route_points);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    imported.workspace.document = app.workspace.document.clone();
    imported.reset_layout_document_state();
    imported.layout_trace_history = vec![usize::MAX];
    imported.route_points.clear();
    assert!(imported.import_layout_trace_state_from_path(&path));
    assert!(
        imported.status_message().contains("Imported trace state"),
        "{}",
        imported.status_message()
    );
    assert_eq!(imported.layout_trace_history, expected_history);
    assert_eq!(imported.route_points, app.route_points);
    let imported_selected_component =
        imported
            .selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| {
                imported
                    .connectivity_report()
                    .ok()
                    .and_then(|report| report.component_for_occurrence(occurrence))
            });
    assert_eq!(imported_selected_component, Some(selected_component));
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    let tools = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("tools menu should build");
    for node_name in [
        "glassworks.menu.item.layout.trace_state_export",
        "glassworks.menu.item.layout.trace_state_import",
    ] {
        let node = tools
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Tools menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }

    let browser = imported
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("imported trace browser should build");
    for node_name in [
        "glassworks.viewctl.layout.trace_state_export",
        "glassworks.viewctl.layout.trace_state_import",
    ] {
        assert!(
            browser.nodes().iter().any(|node| node.name() == node_name),
            "Net browser should expose {node_name}"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_l2n_database_export_import_round_trips_netlist_and_trace_state() {
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

    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-1_000, -200), Point::new(200, 200))),
    )
    .expect("first connected shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(5_000, -200), Point::new(5_800, 200))),
    )
    .expect("second connected shape should be added");
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    let mut component_ids = report
        .components
        .iter()
        .map(|component| component.id)
        .collect::<Vec<_>>();
    component_ids.sort_unstable();
    assert_eq!(component_ids.len(), 2);
    let selected_component = component_ids[1];
    assert!(app.select_layout_net_component(&selected_component.to_string()));
    let other_component = component_ids[0];
    app.record_layout_trace_history(other_component);
    let expected_history = vec![other_component, selected_component];
    app.route_points = vec![Point::new(-500, 0), Point::new(5_400, 0)];

    let path = std::env::temp_dir().join(format!(
        "glassworks-l2n-database-menu-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_layout_l2n_database_to_path(&path));
    assert!(
        app.status_message().contains("Exported L2N database"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.files.recent_files[0].kind, "l2n_database");
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        path.display().to_string()
    );
    let contents = std::fs::read_to_string(&path).expect("L2N database export should write JSON");
    let exchange: LayoutL2nDatabaseExchange =
        serde_json::from_str(&contents).expect("exported L2N database should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_L2N_DATABASE_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(
        exchange.netlist.schema_version,
        LAYOUT_EXTRACTED_NETLIST_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(
        exchange.trace_state.schema_version,
        LAYOUT_TRACE_STATE_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.netlist.components.len(), 2);
    assert_eq!(
        exchange.trace_state.selected_component,
        Some(selected_component)
    );
    assert_eq!(exchange.trace_state.history, expected_history);
    assert_eq!(exchange.trace_state.route_points, app.route_points);

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    imported.workspace.document = app.workspace.document.clone();
    imported.reset_layout_document_state();
    imported.connectivity_report_cache.borrow_mut().take();
    imported.layout_trace_history = vec![usize::MAX];
    imported.route_points.clear();
    assert!(imported.import_layout_l2n_database_from_path(&path));
    assert!(
        imported.status_message().contains("Imported L2N database"),
        "{}",
        imported.status_message()
    );
    let imported_report = imported
        .connectivity_report()
        .expect("imported L2N database should populate connectivity cache");
    assert_eq!(imported_report.components.len(), 2);
    assert_eq!(imported.layout_trace_history, expected_history);
    assert_eq!(imported.route_points, app.route_points);
    let imported_selected_component =
        imported
            .selected_layout_occurrence
            .as_ref()
            .and_then(|occurrence| {
                imported
                    .connectivity_report()
                    .ok()
                    .and_then(|report| report.component_for_occurrence(occurrence))
            });
    assert_eq!(imported_selected_component, Some(selected_component));
    assert_eq!(
        imported.app_options.files.recent_files[0].kind,
        "l2n_database"
    );

    app.layout_trace_history.clear();
    app.route_points.clear();
    app.connectivity_report_cache.borrow_mut().take();
    assert!(app.open_recent_layout_file("0"));
    assert!(
        app.status_message().contains("Imported L2N database"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.layout_trace_history, expected_history);
    assert_eq!(
        app.route_points,
        vec![Point::new(-500, 0), Point::new(5_400, 0)]
    );
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    let tools = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("tools menu should build");
    for node_name in [
        "glassworks.menu.item.layout.l2n_database_export",
        "glassworks.menu.item.layout.l2n_database_import",
    ] {
        let node = tools
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Tools menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }

    let browser = imported
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("imported L2N browser should build");
    for node_name in [
        "glassworks.viewctl.layout.l2n_database_export",
        "glassworks.viewctl.layout.l2n_database_import",
    ] {
        assert!(
            browser.nodes().iter().any(|node| node.name() == node_name),
            "Net browser should expose {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_trace_all_populates_history_and_selects_largest_component() {
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
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(5_000, -200), Point::new(5_800, 200))),
    )
    .expect("separate connected shape should be added");
    app.layout_browser_search = "stale".to_string();
    app.layout_net_browser_filter = LayoutNetBrowserFilter::Labeled;
    app.layout_net_browser_sort = LayoutNetBrowserSort::Name;

    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    let mut expected = report
        .components
        .iter()
        .map(|component| (component.id, component.shapes.len()))
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let expected_ids = expected
        .iter()
        .map(|(component_id, _)| *component_id)
        .collect::<Vec<_>>();
    let largest_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in the larger component");
    assert_eq!(Some(largest_component), expected_ids.first().copied());

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_all"));
    assert_eq!(app.layout_net_browser_filter, LayoutNetBrowserFilter::All);
    assert_eq!(app.layout_net_browser_sort, LayoutNetBrowserSort::Size);
    assert!(app.layout_browser_search.is_empty());
    assert_eq!(app.layout_trace_history, expected_ids);
    assert!(
        app.status_message().contains("Traced all 2 net component"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser_filter.history"));
    assert_eq!(
        app.layout_net_browser_filter,
        LayoutNetBrowserFilter::History
    );
    assert_eq!(
        layout_net_browser_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        expected_ids
    );
    assert!(
        layout_net_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Listed nets" && value == &expected_ids.len().to_string()),
        "history-filtered net browser rows should use trace history"
    );
    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(selected_component, Some(largest_component));

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with trace-all history should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.trace_history.title"),
        "trace-all should expose trace history rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.trace_history.clear"),
        "trace-all should expose a clear-history action"
    );
    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|button| button.name == "glassworks.viewctl.layout.trace_all"),
        "layout controls should expose trace-all action"
    );

    let filtered_component = expected_ids
        .get(1)
        .copied()
        .expect("trace-all fixture should have a second component");
    app.set_layout_browser_search(format!("component {filtered_component}"));
    assert_eq!(
        layout_trace_history_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![filtered_component]
    );
    assert!(
        layout_trace_history_rows(&app)
            .iter()
            .any(|(key, value)| key == "Search"
                && value == &format!("component {filtered_component}"))
    );
    let filtered_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with filtered trace history should build");
    assert!(
        filtered_document.nodes().iter().any(|node| {
            node.name() == format!("glassworks.viewctl.layout.trace_history.{filtered_component}")
        }),
        "trace history search should keep matching component"
    );
    assert!(
        filtered_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.trace_history.select_first"),
        "trace history search should expose select-first when a component matches"
    );
    assert!(
        !filtered_document.nodes().iter().any(|node| {
            node.name() == format!("glassworks.viewctl.layout.trace_history.{}", expected_ids[0])
        }),
        "trace history search should hide non-matching components"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_history.select_first"));
    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(selected_component, Some(filtered_component));
    assert!(app.status_message().contains("Selected trace history net"));

    app.set_layout_browser_search("no trace history match");
    assert!(layout_trace_history_entries(&app).is_empty());
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty trace search should build");
    assert!(
        empty_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.trace_history.empty"),
        "trace history search should show an empty state"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_history.clear"));
    assert!(app.layout_trace_history.is_empty());
    assert!(
        app.status_message().contains("Cleared"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_trace_tool_click_selects_net_component() {
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
    let shape_count = app.workspace.document.shapes.len();
    let component_id = app
        .connectivity_report()
        .expect("test connectivity should extract")
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in a component");

    assert!(app.apply_clicked_node_name("glassworks.tool.trace"));
    assert_eq!(app.active_tool(), ToolMode::Trace);
    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let click = app.layout_world_to_canvas(Point::new(0, 0), rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(click), canvas));
    assert_eq!(
        app.workspace.document.shapes.len(),
        shape_count,
        "trace clicks should select connectivity without editing geometry"
    );

    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(selected_component, Some(component_id));
    assert!(app.status_message().contains("Traced net component"));
    assert_eq!(app.layout_trace_history, vec![component_id]);

    let history_action = format!("glassworks.viewctl.layout.trace_history.{component_id}");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with trace history should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.trace_history.title"),
        "layout inspector should expose trace history after a trace"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == history_action),
        "trace history should expose {history_action}"
    );

    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(app.apply_clicked_node_name(&history_action));
    let restored_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(restored_component, Some(component_id));
    assert_eq!(app.layout_trace_history, vec![component_id]);

    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    let highlight_color = ColorRgba::new(112, 236, 214, 240);
    let highlight_lines = primitives
            .iter()
            .filter(|primitive| {
                matches!(primitive, ScenePrimitive::Line { stroke, .. } if stroke.color == highlight_color)
            })
            .count();
    assert!(
        highlight_lines >= 8,
        "trace tool should reuse selected-net highlighting"
    );
}

#[test]
pub(crate) fn layout_label_tool_places_net_label_for_selected_component() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("label tool test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
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
    let component_id = app
        .connectivity_report()
        .expect("test connectivity should extract")
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in a component");
    let expected_text = format!("NET{component_id}");
    assert!(app.select_layout_net_component(&component_id.to_string()));

    assert!(app.apply_clicked_node_name("glassworks.tool.label"));
    assert_eq!(app.active_tool(), ToolMode::Label);
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with label tool should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.tool.label"),
        "tool strip should expose the label tool"
    );

    let canvas = UiRect::new(0.0, 0.0, 900.0, 700.0);
    let click_world = Point::new(0, 0);
    let click = app.layout_world_to_canvas(click_world, rect_size(canvas));
    assert!(app.handle_layout_canvas_input(&operad::UiInputEvent::PointerDown(click), canvas));
    let label_id = app
        .selected_layout_shape()
        .expect("label tool should select the label it placed");
    let label = app
        .workspace
        .document
        .shapes
        .get(&label_id)
        .expect("placed label shape should exist");
    assert_eq!(label.layer, metal1);
    assert_eq!(
        label.kind,
        ShapeKind::Label {
            position: click_world,
            text: expected_text.clone()
        }
    );
    let report = app
        .connectivity_report()
        .expect("label should keep connectivity extractable");
    let labeled_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .and_then(|component_id| report.component(component_id))
        .expect("original shape should still be in a component");
    assert_eq!(
        labeled_component.net_name.as_deref(),
        Some(expected_text.as_str())
    );
    assert!(
        app.status_message().contains("Added label"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_trace_between_route_points_selects_connected_component() {
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
    let separate = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(5_000, -200), Point::new(5_800, 200))),
        )
        .expect("separate connected shape should be added");
    let shape_count = app.workspace.document.shapes.len();
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    let connected_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in a component");
    let separate_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(separate))
        .expect("separate shape should be in a component");
    assert_ne!(connected_component, separate_component);

    app.route_points = vec![Point::new(0, 0), Point::new(1_000, 0)];
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_between"));
    assert_eq!(
        app.workspace.document.shapes.len(),
        shape_count,
        "trace path should not edit geometry"
    );
    let selected_component = app
        .selected_layout_occurrence
        .as_ref()
        .and_then(|occurrence| {
            app.connectivity_report()
                .ok()
                .and_then(|report| report.component_for_occurrence(occurrence))
        });
    assert_eq!(selected_component, Some(connected_component));
    assert_eq!(app.layout_trace_history, vec![connected_component]);
    assert!(
        app.status_message().contains("Trace path connected"),
        "{}",
        app.status_message()
    );

    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|button| button.name == "glassworks.viewctl.layout.trace_between"),
        "layout control rows should expose trace path"
    );

    app.route_points = vec![Point::new(0, 0), Point::new(5_400, 0)];
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_between"));
    assert!(
        app.status_message().contains("disconnected"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "failed trace path should leave trace history intact"
    );
}

#[test]
pub(crate) fn layout_overlay_highlights_selected_net_component() {
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
    app.selected_layout_shape = Some(first);

    let report = extract_connectivity(&app.workspace.document, &layout_model::default_technology())
        .expect("test shapes should have connectivity");
    let component_id = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("selected shape should be in a component");
    assert_eq!(
        report
            .component(component_id)
            .expect("component should exist")
            .shapes
            .len(),
        2
    );

    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    let highlight_color = ColorRgba::new(112, 236, 214, 240);
    let highlight_lines = primitives
            .iter()
            .filter(|primitive| {
                matches!(primitive, ScenePrimitive::Line { stroke, .. } if stroke.color == highlight_color)
            })
            .count();

    assert!(
        highlight_lines >= 8,
        "selected net component should outline both connected rectangles"
    );
}

#[test]
pub(crate) fn layout_trace_highlight_mode_can_show_history_or_selected_only() {
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
    let separate = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(5_000, -200), Point::new(5_800, 200))),
        )
        .expect("separate connected shape should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(5_700, -200), Point::new(6_400, 200))),
    )
    .expect("second separate connected shape should be added");
    let report = app
        .connectivity_report()
        .expect("test connectivity should extract");
    let selected_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first shape should be in a component");
    let history_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(separate))
        .expect("separate shape should be in a component");
    assert_ne!(selected_component, history_component);
    assert!(app.select_layout_net_component(&selected_component.to_string()));
    app.layout_trace_history = vec![history_component, selected_component];

    let highlight_count = |app: &GlassworksApp| {
        layout_overlay_primitives(app, UiSize::new(900.0, 700.0), UiScale::new(1.0))
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive,
                    ScenePrimitive::Line { stroke, .. }
                        if stroke.color == ColorRgba::new(112, 236, 214, 240)
                )
            })
            .count()
    };
    let selected_lines = highlight_count(&app);
    assert!(
        selected_lines >= 8,
        "selected trace mode should highlight the selected component"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_highlight.history"));
    assert_eq!(
        app.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::History
    );
    let history_lines = highlight_count(&app);
    assert!(
        history_lines > selected_lines,
        "history mode should highlight more traced components"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_highlight.off"));
    assert_eq!(
        app.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::Off
    );
    let off_lines = highlight_count(&app);
    assert!(
        off_lines < selected_lines,
        "off mode should suppress component-wide trace highlighting"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with trace highlight controls should build");
    for node_name in [
        "glassworks.viewctl.layout.trace_highlight.selected",
        "glassworks.viewctl.layout.trace_highlight.history",
        "glassworks.viewctl.layout.trace_highlight.off",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "net browser should expose {node_name}"
        );
    }
}

#[test]
pub(crate) fn layout_overlay_draws_pending_route_point_markers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.active_tool = ToolMode::Route;
    app.route_points.push(Point::new(0, 0));

    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    let marker_color = ColorRgba::new(250, 210, 80, 255);

    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Circle { fill, radius, .. }
                    if *fill == marker_color && (radius - 5.0).abs() < f32::EPSILON
            )
        }),
        "route placement should show pending point markers"
    );
}

#[test]
pub(crate) fn layout_overlay_draws_drc_markers_when_violations_exist() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let layer = app.active_layer();
    app.add_layout_shape(
        layer,
        ShapeKind::Rectangle(Rect::new(Point::new(-20, -20), Point::new(20, 20))),
    )
    .expect("tiny shape should be added");
    let rules = RuleDeck::demo(&app.workspace().document);
    assert!(
        !run_drc(&app.workspace().document, &rules).is_empty(),
        "fixture should create a visible DRC violation"
    );
    let _ = app.run_drc_summary();

    let document = app
        .build_operad_document(UiSize::new(1280.0, 720.0))
        .expect("layout editor should build");
    let overlay = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.overlay")
        .expect("2D editor overlay should exist");
    let UiContent::Scene(primitives) = overlay.content() else {
        panic!("2D editor overlay should be a scene");
    };

    assert!(
        primitives.iter().any(|primitive| {
            matches!(primitive, ScenePrimitive::Text(text) if text.text.contains("DRC overlay"))
        }),
        "DRC overlay should report active violations"
    );
}

#[test]
pub(crate) fn layout_overlay_skips_hidden_or_waived_drc_markers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    let layer = app.active_layer();
    app.add_layout_shape(
        layer,
        ShapeKind::Rectangle(Rect::new(Point::new(-20, -20), Point::new(20, 20))),
    )
    .expect("tiny shape should be added");
    let rules = RuleDeck::demo(&app.workspace().document);
    let violations = run_drc(&app.workspace().document, &rules);
    assert!(
        !violations.is_empty(),
        "fixture should create visible DRC violations"
    );
    for violation in violations {
        app.workspace.document.marker_states.insert(
            violation.stable_key(),
            MarkerState {
                hidden: true,
                waived: false,
                note: None,
                ..MarkerState::default()
            },
        );
    }
    let _ = app.run_drc_summary();

    let primitives = layout_overlay_primitives(&app, UiSize::new(1280.0, 720.0), UiScale::new(1.0));

    assert!(
        !primitives.iter().any(|primitive| {
            matches!(primitive, ScenePrimitive::Text(text) if text.text.contains("DRC overlay"))
        }),
        "hidden DRC markers should not appear in the overlay legend"
    );
}

#[test]
pub(crate) fn layout_drc_can_run_inside_selected_region() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("selected region drc test");
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
    let region_id = app
        .add_layout_shape(
            annotation,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(50_000, 50_000))),
        )
        .expect("selected DRC region should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("inside narrow test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(200_000, 20_000),
            Point::new(200_020, 20_020),
        )),
    )
    .expect("outside narrow test rectangle should be added");
    let rules = RuleDeck::demo(&app.workspace.document);
    let full_violations = run_drc(&app.workspace.document, &rules);
    assert!(
        full_violations.len() >= 2,
        "fixture should create inside and outside DRC violations"
    );

    app.selected_layout_shape = Some(region_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(region_id));
    let status = app.run_drc_for_selected_region_summary();
    assert!(
        status.contains("DRC region found") && status.contains(" of "),
        "{status}"
    );
    let report = app
        .drc_report()
        .expect("selected-region DRC should populate the report cache");
    assert!(
        !report.violations.is_empty(),
        "selected-region DRC should keep the inside violation"
    );
    assert!(
        report.violations.len() < full_violations.len(),
        "selected-region DRC should omit outside violations"
    );
    let region_points =
        region_points_for_shape_kind(&app.workspace.document.shapes.get(&region_id).unwrap().kind)
            .expect("selected region should be rectangular");
    assert!(
        report
            .violations
            .iter()
            .all(|violation| drc_violation_intersects_region(violation, &region_points)),
        "cached DRC markers should intersect the selected region"
    );

    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|node| node.name == "glassworks.viewctl.layout.run_drc_region"),
        "layout controls should expose selected-region DRC"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.run_drc_region"));
    assert!(
        app.status_message().contains("DRC region found"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_drc_can_run_inside_selected_nonconvex_region() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("selected non-convex region drc test");
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
    let region_id = app
        .add_layout_shape(
            annotation,
            ShapeKind::Polygon(Polygon::new(vec![
                Point::new(0, 0),
                Point::new(50_000, 0),
                Point::new(50_000, 20_000),
                Point::new(20_000, 20_000),
                Point::new(20_000, 50_000),
                Point::new(0, 50_000),
            ])),
        )
        .expect("selected non-convex DRC region should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 10_000),
            Point::new(10_020, 10_020),
        )),
    )
    .expect("inside narrow test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(40_000, 40_000),
            Point::new(40_020, 40_020),
        )),
    )
    .expect("concavity narrow test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(100_000, 10_000),
            Point::new(100_020, 10_020),
        )),
    )
    .expect("outside narrow test rectangle should be added");
    let rules = RuleDeck::demo(&app.workspace.document);
    let full_violations = run_drc(&app.workspace.document, &rules);
    assert!(
        full_violations.len() >= 3,
        "fixture should create inside, concavity, and outside DRC violations"
    );

    app.selected_layout_shape = Some(region_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(region_id));
    let status = app.run_drc_for_selected_region_summary();
    assert!(
        status.contains("DRC region found") && status.contains(" of "),
        "{status}"
    );
    let report = app
        .drc_report()
        .expect("selected-region DRC should populate the report cache");
    assert!(
        !report.violations.is_empty(),
        "selected non-convex region DRC should keep the inside violation"
    );
    assert!(
        report.violations.len() < full_violations.len(),
        "selected non-convex region DRC should omit concavity and outside violations"
    );
    let region_parts = convex_region_parts_for_shape_kind(
        &app.workspace.document.shapes.get(&region_id).unwrap().kind,
    )
    .expect("selected region should decompose into convex parts");
    assert!(
        report
            .violations
            .iter()
            .all(|violation| drc_violation_intersects_any_region(violation, &region_parts)),
        "cached DRC markers should intersect the selected non-convex region"
    );
    assert!(
        report.violations.iter().all(|violation| {
            violation.bounds.min.x < 20_000 || violation.bounds.min.y < 20_000
        }),
        "cached DRC markers should not include the missing upper-right concavity"
    );
}

#[test]
pub(crate) fn layout_drc_can_run_inside_current_cell() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("current cell drc test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let top_shape = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(20, 20))),
        )
        .expect("top-level DRC fixture should be added");
    let child = app.workspace.document.create_cell("drc_child");
    let child_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(20, 20))),
        )
        .expect("child DRC fixture should be added");
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(200_000, 0))
        .expect("child instance should be inserted");

    let rules = RuleDeck::demo(&app.workspace.document);
    let full_violations = run_drc(&app.workspace.document, &rules);
    assert!(
        full_violations.len() >= 2,
        "fixture should create top-level and child DRC violations"
    );

    app.layout_view_top_cell = child;
    let status = app.run_drc_for_current_cell_summary();
    assert!(status.contains("DRC cell drc_child found"), "{status}");
    let report = app
        .drc_report()
        .expect("current-cell DRC should populate the report cache");
    assert!(
        !report.violations.is_empty(),
        "current-cell DRC should keep child-cell violations"
    );
    assert!(
        report.violations.len() < full_violations.len(),
        "current-cell DRC should omit violations outside the active cell"
    );
    assert!(
        report.violations.iter().all(|violation| {
            violation.shape_ids.contains(&child_shape) && !violation.shape_ids.contains(&top_shape)
        }),
        "current-cell DRC should only report source shapes from the active cell"
    );

    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|node| node.name == "glassworks.viewctl.layout.run_drc_cell"),
        "layout controls should expose current-cell DRC"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.run_drc_cell"));
    assert!(
        app.status_message().contains("DRC cell drc_child found"),
        "{}",
        app.status_message()
    );
}
