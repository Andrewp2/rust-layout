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

    let path = std::env::temp_dir().join(format!(
        "glassworks-netlist-menu-{}.json",
        std::process::id()
    ));
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
    assert_eq!(app.app_options.files.recent_files[0].kind, "netlist");
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        path.display().to_string()
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
    assert_eq!(app.layout_net_browser_filter, LayoutNetBrowserFilter::All);
    assert_eq!(
        app.app_options.files.recent_files[0].kind,
        "spice_schematic"
    );
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        schematic_path.display().to_string()
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
    assert_eq!(imported.app_options.files.recent_files[0].kind, "netlist");
    let imported_net_rows = layout_net_browser_rows(&imported);
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Connectivity source" && value == "Imported netlist"),
        "imported netlist rows should expose source metadata: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Connectivity document" && value == "netlist export source"),
        "imported netlist rows should expose source document: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows.iter().any(|(key, value)| {
            key == "Connectivity technology" && value == &app.active_layout_technology().name
        }),
        "imported netlist rows should expose source technology: {imported_net_rows:?}"
    );
    app.layout_spice_comparison = None;
    assert!(app.open_recent_layout_file("0"));
    assert!(
        app.status_message().contains("matches layout connectivity"),
        "{}",
        app.status_message()
    );
    assert!(
        app.layout_spice_comparison
            .as_ref()
            .is_some_and(|comparison| comparison.is_match()),
        "recent SPICE schematic reload should rerun compare"
    );
    app.connectivity_report_cache.borrow_mut().take();
    assert!(app.open_recent_layout_file("1"));
    assert!(
        app.status_message().contains("Imported netlist"),
        "{}",
        app.status_message()
    );
    let reloaded_report = app
        .connectivity_report()
        .expect("reloaded netlist should populate the connectivity cache");
    assert_eq!(reloaded_report.components.len(), 1);
    assert_eq!(
        reloaded_report.components[0].net_name.as_deref(),
        Some("CLK")
    );
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
        "glassworks.layout.connectivity_source.title",
        "glassworks.layout.connectivity_source.source",
        "glassworks.layout.connectivity_source.document",
        "glassworks.layout.connectivity_source.revision",
        "glassworks.layout.connectivity_source.technology",
    ] {
        assert!(
            browser.nodes().iter().any(|node| node.name() == node_name),
            "Net browser should expose {node_name}"
        );
    }
    for (node_name, expected_text) in [
        (
            "glassworks.layout.netlist_summary.components",
            "Components: 1",
        ),
        ("glassworks.layout.netlist_summary.shapes", "Shapes: 1"),
        ("glassworks.layout.netlist_summary.labeled", "Labeled: 1"),
        (
            "glassworks.layout.netlist_summary.issues",
            "Issues: 0 short / 0 open",
        ),
    ] {
        let node = browser
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Net browser should expose {node_name}"));
        let UiContent::Text(text) = node.content() else {
            panic!("{node_name} should be a text summary row");
        };
        assert_eq!(text.text, expected_text);
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
    app.layout_trace_highlight_mode = LayoutTraceHighlightMode::History;
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
    assert_eq!(app.app_options.files.recent_files[0].kind, "trace_state");
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        path.display().to_string()
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
    assert_eq!(exchange.highlight_mode, "history");

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
    assert_eq!(
        imported.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::History
    );
    assert_eq!(
        imported.layout_net_browser_filter,
        LayoutNetBrowserFilter::History
    );
    assert_eq!(
        imported.app_options.files.recent_files[0].kind,
        "trace_state"
    );
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
    app.layout_trace_history.clear();
    app.route_points.clear();
    app.layout_trace_highlight_mode = LayoutTraceHighlightMode::Off;
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(app.open_recent_layout_file("0"));
    assert!(
        app.status_message().contains("Imported trace state"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.layout_trace_history, expected_history);
    assert_eq!(
        app.route_points,
        vec![Point::new(-500, 0), Point::new(5_400, 0)]
    );
    assert_eq!(
        app.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::History
    );
    assert_eq!(
        app.layout_net_browser_filter,
        LayoutNetBrowserFilter::History
    );
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
    app.layout_trace_highlight_mode = LayoutTraceHighlightMode::History;
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
    assert_eq!(exchange.trace_state.highlight_mode, "history");

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
    assert_eq!(
        imported.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::History
    );
    assert_eq!(
        imported.layout_net_browser_filter,
        LayoutNetBrowserFilter::History
    );
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
    let imported_net_rows = layout_net_browser_rows(&imported);
    assert!(
        imported_net_rows.iter().any(|(key, value)| {
            key == "Trace selected" && value.contains(&format!("#{selected_component}"))
        }),
        "imported L2N net browser rows should expose selected trace state: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Connectivity source" && value == "Imported L2N DB"),
        "imported L2N net browser rows should expose source metadata: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows.iter().any(|(key, value)| {
            key == "Connectivity document" && value == &app.workspace.document.name
        }),
        "imported L2N net browser rows should expose source document: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows.iter().any(|(key, value)| {
            key == "Connectivity technology" && value == &app.active_layout_technology().name
        }),
        "imported L2N net browser rows should expose source technology: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Trace history" && value == "2 total"),
        "imported L2N net browser rows should expose trace history count: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Trace route points" && value == "2 points / 1 segment"),
        "imported L2N net browser rows should expose pending route points: {imported_net_rows:?}"
    );
    assert!(
        imported_net_rows
            .iter()
            .any(|(key, value)| key == "Trace highlight" && value == "History"),
        "imported L2N net browser rows should expose trace highlight mode: {imported_net_rows:?}"
    );
    let imported_connectivity_rows = layout_connectivity_rows(&imported);
    assert!(
        imported_connectivity_rows
            .iter()
            .any(|(key, value)| { key == "Connectivity source" && value == "Imported L2N DB" }),
        "Connectivity inspector rows should expose imported source metadata: {imported_connectivity_rows:?}"
    );
    assert!(
        imported_connectivity_rows.iter().any(|(key, value)| {
            key == "Trace selected" && value.contains(&format!("#{selected_component}"))
        }),
        "Connectivity inspector rows should expose imported trace state: {imported_connectivity_rows:?}"
    );
    let total_connectivity_rows = imported_connectivity_rows.len();
    imported.set_layout_browser_search("source=Imported L2N DB");
    let source_rows = layout_connectivity_rows(&imported)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        source_rows.get("Listed connectivity rows"),
        Some(&format!("1 / {total_connectivity_rows} rows"))
    );
    assert!(
        source_rows.contains_key("Connectivity source"),
        "Connectivity source selector should keep the imported source row: {source_rows:?}"
    );
    assert!(
        !source_rows.contains_key("Connectivity document"),
        "Connectivity source selector should hide non-matching source metadata rows: {source_rows:?}"
    );
    imported.set_layout_browser_search("history=2");
    let history_rows = layout_connectivity_rows(&imported)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(
        history_rows
            .get("Trace history")
            .is_some_and(|value| value.contains("listed") && value.contains("2 total")),
        "Connectivity trace-history selector should keep the trace-history summary: {history_rows:?}"
    );
    imported.set_layout_browser_search("");

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
        app.layout_net_browser_filter,
        LayoutNetBrowserFilter::History
    );
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
        "glassworks.layout.connectivity_source.title",
        "glassworks.layout.connectivity_source.source",
        "glassworks.layout.connectivity_source.document",
        "glassworks.layout.connectivity_source.revision",
        "glassworks.layout.connectivity_source.technology",
        "glassworks.layout.trace_state_summary.title",
        "glassworks.layout.trace_state_summary.selected",
        "glassworks.layout.trace_state_summary.history",
        "glassworks.layout.trace_state_summary.route_points",
        "glassworks.layout.trace_state_summary.highlight",
        "glassworks.viewctl.layout.trace_state.clear",
    ] {
        assert!(
            browser.nodes().iter().any(|node| node.name() == node_name),
            "Net browser should expose {node_name}"
        );
    }
    let history_node = browser
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.trace_state_summary.history")
        .expect("Net browser should expose trace history state text");
    let UiContent::Text(history_text) = history_node.content() else {
        panic!("Trace history state should be text");
    };
    assert_eq!(history_text.text, "Trace history: 2 total");
    assert!(imported.apply_clicked_node_name("glassworks.viewctl.layout.trace_state.clear"));
    assert_eq!(imported.selected_layout_shape, None);
    assert_eq!(imported.selected_layout_occurrence, None);
    assert!(imported.layout_trace_history.is_empty());
    assert!(imported.route_points.is_empty());
    assert_eq!(
        imported.layout_trace_highlight_mode,
        LayoutTraceHighlightMode::Selected
    );
    assert!(
        imported.status_message().contains("Cleared trace state")
            && imported.status_message().contains("2 traced nets")
            && imported.status_message().contains("2 route points"),
        "{}",
        imported.status_message()
    );
    assert!(
        layout_net_browser_rows(&imported)
            .iter()
            .all(|(key, _)| !key.starts_with("Trace ")),
        "cleared trace state should remove trace-state and route-point rows from the net browser"
    );
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
    app.set_layout_browser_search("history=2");
    assert_eq!(
        layout_trace_history_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![filtered_component],
        "trace history selector search should match history positions"
    );
    assert_eq!(
        layout_net_browser_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![filtered_component],
        "history-filtered net browser search should match trace-history positions"
    );
    assert!(
        layout_net_browser_rows(&app)
            .iter()
            .any(|(key, value)| key == "Trace history" && value == "1 listed / 2 total"),
        "trace state summary should count history-position search matches"
    );
    app.set_layout_browser_search("latest=true");
    assert_eq!(
        layout_trace_history_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![expected_ids[0]],
        "trace history selector search should match the latest entry"
    );
    assert_eq!(
        layout_net_browser_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![expected_ids[0]],
        "history-filtered net browser search should match the latest trace"
    );
    app.set_layout_browser_search(format!("component={filtered_component}"));
    assert_eq!(
        layout_trace_history_entries(&app)
            .into_iter()
            .map(|(component_id, _)| component_id)
            .collect::<Vec<_>>(),
        vec![filtered_component],
        "trace history selector search should still match component ids"
    );
    app.set_layout_browser_search(format!("component {filtered_component}"));
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
        filtered_document.nodes().iter().any(|node| {
            node.name()
                == format!("glassworks.viewctl.layout.trace_history.remove.{filtered_component}")
        }),
        "trace history search should expose per-entry removal"
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
            node.name()
                == format!(
                    "glassworks.viewctl.layout.trace_history.{}",
                    expected_ids[0]
                )
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

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.trace_history.remove.{filtered_component}"
    )));
    assert!(
        !app.layout_trace_history.contains(&filtered_component),
        "per-entry removal should remove the filtered component from trace history"
    );
    assert!(
        app.status_message().contains("Removed trace history net"),
        "{}",
        app.status_message()
    );

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
    let second = app
        .add_layout_shape(
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
    let second_object_action = format!(
        "glassworks.viewctl.layout.net_component_source.{}",
        layout_occurrence_action_key(&ShapeOccurrenceId::top_level(second))
    );
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
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.net_browser.clear_selected"),
        "net browser should expose selected-trace clearing"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_objects.title"),
        "net browser should expose selected-net source objects"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == second_object_action),
        "net browser should expose {second_object_action}"
    );
    assert!(app.apply_clicked_node_name(&second_object_action));
    assert_eq!(app.selected_layout_shape, Some(second));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(second))
    );
    assert_eq!(
        app.layout_trace_history,
        vec![component_id],
        "selecting a source object should not add duplicate trace history"
    );
    assert!(
        app.status_message()
            .contains("Selected net component object"),
        "{}",
        app.status_message()
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
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.net_browser.clear_selected"));
    assert_eq!(app.selected_layout_occurrence, None);
    assert_eq!(
        app.layout_trace_history,
        vec![component_id],
        "clearing the selected traced net should preserve trace history"
    );
    assert!(
        app.status_message().contains("Cleared selected traced net"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name(&history_action));

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
    assert!(
        app.status_message().contains("Added label"),
        "{}",
        app.status_message()
    );
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
    assert_eq!(
        selected_layout_net_component_id(&app, &report),
        Some(component_id),
        "selected label should keep the net browser tied to its component"
    );
    let label_action = format!(
        "glassworks.viewctl.layout.net_component_label.{}",
        layout_occurrence_action_key(&ShapeOccurrenceId::top_level(label_id))
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected net label should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_labels.title"),
        "net browser should expose selected-net label rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == label_action),
        "net browser should expose {label_action}"
    );

    assert!(app.select_layout_net_component(&component_id.to_string()));
    let trace_history = app.layout_trace_history.clone();
    assert!(app.apply_clicked_node_name(&label_action));
    assert_eq!(app.selected_layout_shape, Some(label_id));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(label_id))
    );
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("selected label should keep connectivity extractable")
        ),
        Some(component_id)
    );
    assert_eq!(
        app.layout_trace_history, trace_history,
        "selecting a net label should not add duplicate trace history"
    );
    assert!(
        app.status_message().contains("Selected net label"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_net_browser_cross_probes_selected_component_devices() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("net device browser test");
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
    let lower = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 1_000, 1_000)),
        )
        .expect("lower capacitor plate should be added");
    let upper = app
        .add_layout_shape(
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 200), 600, 600)),
        )
        .expect("upper capacitor plate should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Label {
            position: Point::new(300, 300),
            text: "bottom".to_string(),
        },
    )
    .expect("lower label should be added");
    app.add_layout_shape(
        metal2,
        ShapeKind::Label {
            position: Point::new(700, 700),
            text: "top".to_string(),
        },
    )
    .expect("upper label should be added");

    let report = app
        .connectivity_report()
        .expect("MIM capacitor connectivity should extract");
    let lower_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(lower))
        .expect("lower plate should be a component");
    let upper_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(upper))
        .expect("upper plate should be a component");
    assert_ne!(lower_component, upper_component);
    let device = report
        .devices
        .iter()
        .find(|device| device.kind == "capacitor")
        .expect("overlapping labeled plates should extract a capacitor");
    let device_id = device.id;
    assert_eq!(
        layout_net_component_device_count(&report, upper_component),
        1
    );
    assert!(
        layout_net_component_device_text(device, upper_component).contains("mimcap"),
        "selected-net device rows should use extracted model labels"
    );
    assert!(
        layout_netlist_device_text(&report, device).contains("mimcap"),
        "global netlist device rows should use extracted model labels"
    );
    assert_eq!(
        layout_netlist_device_entries(&report, Some("mimcap"))
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        vec![device_id],
        "global extracted-device rows should match shared browser search"
    );
    assert!(
        layout_netlist_device_entries(&report, Some("resistor")).is_empty(),
        "global extracted-device rows should hide nonmatching devices"
    );
    let peer_terminal_search = layout_net_browser_entries_from_report(
        &report,
        LayoutNetBrowserFilter::All,
        LayoutNetBrowserSort::Id,
        Some("peer A"),
    )
    .into_iter()
    .map(|(id, _)| id)
    .collect::<Vec<_>>();
    assert_eq!(
        peer_terminal_search,
        vec![upper_component],
        "net browser search should match peer terminal names on connected devices"
    );

    let netlist_device_action = format!("glassworks.viewctl.layout.netlist_device.{device_id}");
    let global_device_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with extracted devices should build");
    assert!(
        global_device_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.netlist_devices.title"),
        "net browser should expose global extracted-device rows"
    );
    assert!(
        global_device_document
            .nodes()
            .iter()
            .any(|node| node.name() == netlist_device_action),
        "net browser should expose {netlist_device_action}"
    );
    assert!(
        global_device_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.netlist.select_first"),
        "net browser should expose first matching global netlist item selection"
    );
    assert!(app.apply_clicked_node_name(&netlist_device_action));
    assert!(
        [lower, upper].contains(&app.selected_layout_shape.expect("device source selected")),
        "global device cross-probe should select one extracted source shape"
    );
    assert!(
        app.layout_trace_history.is_empty(),
        "global device cross-probe should not append trace history"
    );
    assert!(
        app.status_message().contains("Selected extracted device"),
        "{}",
        app.status_message()
    );
    app.layout_browser_search = "mimcap".to_string();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.netlist.select_first"));
    assert!(
        [lower, upper].contains(&app.selected_layout_shape.expect("device source selected")),
        "global netlist select-first should select the matching extracted device source"
    );
    assert!(
        app.status_message().contains("Selected extracted device"),
        "{}",
        app.status_message()
    );
    app.layout_browser_search.clear();

    assert!(app.select_layout_net_component(&upper_component.to_string()));
    let device_action = format!("glassworks.viewctl.layout.net_component_device.{device_id}");
    let device_peer_action = format!(
        "glassworks.viewctl.layout.net_component_device_peer.{device_id}.{lower_component}"
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected net device should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_devices.title"),
        "net browser should expose selected-net device rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == device_action),
        "net browser should expose {device_action}"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_device_peers.title"),
        "net browser should expose selected-net device peer rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == device_peer_action),
        "net browser should expose {device_peer_action}"
    );

    let trace_history = app.layout_trace_history.clone();
    assert!(app.apply_clicked_node_name(&device_action));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(upper)),
        "device cross-probe should prefer the source object on the selected component"
    );
    assert_eq!(app.selected_layout_shape, Some(upper));
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("device cross-probe should keep connectivity extractable")
        ),
        Some(upper_component)
    );
    assert_eq!(
        app.layout_trace_history, trace_history,
        "selecting a net device should not add duplicate trace history"
    );
    assert!(
        app.status_message().contains("Selected net device"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name(&device_peer_action));
    assert_eq!(app.selected_layout_shape, Some(lower));
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("device peer cross-probe should keep connectivity extractable")
        ),
        Some(lower_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![lower_component, upper_component],
        "selecting a device peer should use normal trace-history selection"
    );
    assert!(
        app.status_message().contains("Selected device peer"),
        "{}",
        app.status_message()
    );
}

#[test]
pub(crate) fn layout_net_browser_cross_probes_selected_component_issues() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.2),
        ..Default::default()
    });
    app.workspace.document = Document::new("net issue browser test");
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
    let first = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 800, 300)),
        )
        .expect("first metal should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(100, 100),
            text: "clk".to_string(),
        },
    )
    .expect("first clk label should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(700, 100),
            text: "data".to_string(),
        },
    )
    .expect("conflicting data label should be added");
    let second = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(1_000, 0), 300, 300)),
        )
        .expect("second metal should be added");
    app.add_layout_shape(
        annotation,
        ShapeKind::Label {
            position: Point::new(1_100, 100),
            text: "clk".to_string(),
        },
    )
    .expect("second clk label should be added");

    let report = app
        .connectivity_report()
        .expect("connectivity issues should extract");
    assert_eq!(report.shorts.len(), 1);
    assert_eq!(report.opens.len(), 1);
    let first_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(first))
        .expect("first metal should be a component");
    let second_component = report
        .component_for_occurrence(&ShapeOccurrenceId::top_level(second))
        .expect("second metal should be a component");
    assert_ne!(first_component, second_component);
    assert_eq!(
        layout_net_component_issue_label(&report, first_component),
        "1 short / 1 open"
    );
    assert_eq!(
        layout_netlist_short_issue_text(&report.shorts[0]),
        format!("Short CLK/DATA #{first_component}")
    );
    assert_eq!(
        layout_netlist_open_issue_text(&report.opens[0]),
        format!("Open CLK #{first_component}/#{second_component}")
    );
    assert_eq!(
        layout_netlist_issue_entries(&report, Some("short data"))
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>(),
        vec!["short.0".to_string()],
        "global issue rows should match short details in shared browser search"
    );
    assert_eq!(
        layout_netlist_issue_entries(&report, Some("open clk"))
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>(),
        vec!["open.0".to_string()],
        "global issue rows should match open details in shared browser search"
    );

    let netlist_short_action = "glassworks.viewctl.layout.netlist_issue.short.0";
    let netlist_open_action = "glassworks.viewctl.layout.netlist_issue.open.0";
    app.layout_browser_search = "short data".to_string();
    let searched_issue_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with searched netlist issues should build");
    assert!(
        searched_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == netlist_short_action),
        "shared browser search should keep matching global short issue row"
    );
    assert!(
        !searched_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == netlist_open_action),
        "shared browser search should hide nonmatching global open issue row"
    );
    assert!(
        searched_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.netlist.select_first"),
        "searched global issue rows should expose first matching netlist item selection"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.netlist.select_first"));
    assert_eq!(app.selected_layout_shape, Some(first));
    assert!(
        app.status_message().contains("Selected extracted issue"),
        "{}",
        app.status_message()
    );
    app.layout_browser_search.clear();
    let global_issue_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with extracted netlist issues should build");
    assert!(
        global_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.netlist_issues.title"),
        "net browser should expose global connectivity issue rows"
    );
    assert!(
        global_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == netlist_short_action),
        "net browser should expose {netlist_short_action}"
    );
    assert!(
        global_issue_document
            .nodes()
            .iter()
            .any(|node| node.name() == netlist_open_action),
        "net browser should expose {netlist_open_action}"
    );
    assert!(app.apply_clicked_node_name(netlist_short_action));
    assert_eq!(app.selected_layout_shape, Some(first));
    assert!(
        app.layout_trace_history.is_empty(),
        "global issue cross-probe should not append trace history"
    );
    assert!(
        app.status_message().contains("Selected extracted issue"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name(netlist_open_action));
    assert_eq!(app.selected_layout_shape, Some(first));
    assert!(
        app.layout_trace_history.is_empty(),
        "global open issue cross-probe should not append trace history"
    );
    assert!(
        app.status_message().contains("Selected extracted issue"),
        "{}",
        app.status_message()
    );

    assert!(app.select_layout_net_component(&first_component.to_string()));
    let short_action = "glassworks.viewctl.layout.net_component_issue.short.0";
    let open_action = "glassworks.viewctl.layout.net_component_issue.open.0";
    let peer_action =
        format!("glassworks.viewctl.layout.net_component_open_peer.{second_component}");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected net issues should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_issues.title"),
        "net browser should expose selected-net issue rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == short_action),
        "net browser should expose {short_action}"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == open_action),
        "net browser should expose {open_action}"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.net_component_open_peers.title"),
        "net browser should expose selected-net open peer rows"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == peer_action),
        "net browser should expose {peer_action}"
    );

    let trace_history = app.layout_trace_history.clone();
    assert!(app.apply_clicked_node_name(open_action));
    assert_eq!(app.selected_layout_shape, Some(first));
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("open issue cross-probe should keep connectivity extractable")
        ),
        Some(first_component)
    );
    assert_eq!(
        app.layout_trace_history, trace_history,
        "selecting a net issue should not add duplicate trace history"
    );
    assert!(
        app.status_message().contains("Selected net issue"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name(&peer_action));
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("open peer cross-probe should keep connectivity extractable")
        ),
        Some(second_component)
    );
    assert_eq!(app.selected_layout_shape, Some(second));
    assert_eq!(
        app.layout_trace_history,
        vec![second_component, first_component],
        "selecting an open peer should use normal trace-history selection"
    );
    assert!(
        app.status_message().contains("Selected open peer"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name(short_action));
    assert_eq!(
        app.status_message(),
        format!("Short #0 is not on selected net {second_component}")
    );
    assert!(app.select_layout_net_component(&first_component.to_string()));
    assert!(app.apply_clicked_node_name(short_action));
    assert_eq!(app.selected_layout_shape, Some(first));
    assert!(
        app.status_message().contains("Selected net issue"),
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
    let route_path_color = ColorRgba::new(105, 201, 135, 220);
    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Line { stroke, .. } if stroke.color == route_path_color
            )
        }),
        "trace path should show connected route endpoints in the overlay"
    );
    let connected_route_point_markers = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Circle { fill, .. } if *fill == ColorRgba::new(250, 210, 80, 255)
            )
        })
        .count();
    assert!(
        connected_route_point_markers >= 2,
        "trace path should keep pending route point markers visible outside the Route tool"
    );

    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|button| button.name == "glassworks.viewctl.layout.trace_between"),
        "layout control rows should expose trace path"
    );
    assert!(
        layout_editor_control_rows(&app)
            .iter()
            .flatten()
            .any(|button| button.name == "glassworks.viewctl.layout.route_points.clear"),
        "layout control rows should expose route-point clearing"
    );
    let connected_path_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with connected trace path should build");
    assert!(
        connected_path_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.trace_path.select_net"),
        "connected trace path should expose direct net selection"
    );
    for node_name in [
        "glassworks.viewctl.layout.trace_path_endpoint.start",
        "glassworks.viewctl.layout.trace_path_endpoint.end",
    ] {
        assert!(
            connected_path_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "connected trace path should expose {node_name}"
        );
    }
    app.selected_layout_shape = Some(separate);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(separate));
    let route_points_before_path_net = app.route_points.clone();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path.select_net"));
    assert_eq!(app.route_points, route_points_before_path_net);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(connected_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "trace path net selection should preserve deduplicated trace history"
    );
    assert!(
        app.status_message().contains("Selected trace path net"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_endpoint.end"));
    assert_eq!(app.route_points, route_points_before_path_net);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(connected_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "trace path endpoint net selection should preserve deduplicated trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path end net #{} {}",
            connected_component,
            layout_trace_path_component_name(&app, connected_component)
        )),
        "{}",
        app.status_message()
    );

    let selected_before_invalid_path = app.selected_layout_occurrence.clone();
    app.route_points = vec![Point::new(0, 0), Point::new(3_000, 0), Point::new(1_000, 0)];
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_between"));
    assert_eq!(app.selected_layout_occurrence, selected_before_invalid_path);
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "trace path with an invalid middle segment should leave trace history intact"
    );
    assert!(
        app.status_message().contains("segment 1") && app.status_message().contains("untraceable"),
        "{}",
        app.status_message()
    );

    app.route_points = vec![Point::new(0, 0), Point::new(5_400, 0)];
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_between"));
    assert!(
        app.status_message().contains("disconnected"),
        "{}",
        app.status_message()
    );
    let disconnected_path_color = ColorRgba::new(236, 91, 88, 220);
    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Line { stroke, .. } if stroke.color == disconnected_path_color
            )
        }),
        "trace path should show disconnected route endpoints in the overlay"
    );
    let disconnected_route_point_markers = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Circle { fill, .. } if *fill == ColorRgba::new(250, 210, 80, 255)
            )
        })
        .count();
    assert!(
        disconnected_route_point_markers >= 2,
        "disconnected trace path should keep pending route point markers visible outside the Route tool"
    );
    app.route_points = vec![Point::new(0, 0), Point::new(1_000, 0), Point::new(5_400, 0)];
    let primitives = layout_overlay_primitives(&app, UiSize::new(900.0, 700.0), UiScale::new(1.0));
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Line { stroke, .. } if stroke.color == route_path_color
            )
        }),
        "multi-point trace path preview should color connected segments independently"
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                ScenePrimitive::Line { stroke, .. } if stroke.color == disconnected_path_color
            )
        }),
        "multi-point trace path preview should color disconnected segments independently"
    );
    let segment_entries = layout_trace_path_segment_entries(&app);
    assert_eq!(segment_entries.len(), 2);
    let connected_segment_length = app.format_layout_length(1_000.0);
    let blocked_segment_length = app.format_layout_length(4_400.0);
    assert!(
        segment_entries[0].1.contains("Connected"),
        "first route path segment should summarize connected status: {:?}",
        segment_entries
    );
    assert!(
        segment_entries[0].1.contains(&connected_segment_length),
        "first route path segment should include segment length: {:?}",
        segment_entries
    );
    assert!(
        segment_entries[0].1.contains("0,0..1000,0"),
        "first route path segment should include endpoint coordinates: {:?}",
        segment_entries
    );
    assert!(
        segment_entries[1].1.contains("Disconnected"),
        "second route path segment should summarize disconnected status: {:?}",
        segment_entries
    );
    assert!(
        segment_entries[1].1.contains(&blocked_segment_length),
        "second route path segment should include segment length: {:?}",
        segment_entries
    );
    assert!(
        segment_entries[1].1.contains("1000,0..5400,0"),
        "second route path segment should include endpoint coordinates: {:?}",
        segment_entries
    );
    app.set_layout_browser_search("1000 0 5400 0");
    assert_eq!(
        layout_trace_path_segment_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![1],
        "trace path segment search should match endpoint coordinates"
    );
    app.set_layout_browser_search(&blocked_segment_length);
    assert_eq!(
        layout_trace_path_segment_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![1],
        "trace path segment search should match segment length"
    );
    app.set_layout_browser_search("segment=2");
    assert_eq!(
        layout_trace_path_segment_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![1],
        "trace path segment selector search should match segment indices"
    );
    app.set_layout_browser_search("status=disconnected");
    assert_eq!(
        layout_trace_path_segment_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![1],
        "trace path segment selector search should match segment status"
    );
    app.set_layout_browser_search(&format!("start_component={connected_component}"));
    assert_eq!(
        layout_trace_path_segment_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![1],
        "trace path segment selector search should match disconnected endpoint components"
    );
    app.set_layout_browser_search("");
    let segment_rows = layout_net_browser_rows(&app);
    assert!(
        segment_rows.iter().any(|(key, value)| {
            key == "Trace path result"
                && value.contains("Blocked at S2")
                && value.contains("1000,0..5400,0")
                && value.contains("Disconnected")
        }),
        "net browser detail rows should summarize trace path result: {segment_rows:?}"
    );
    assert!(
        segment_rows.iter().any(|(key, value)| {
            key == "Trace path endpoints"
                && value.contains("P1 (0, 0)")
                && value.contains(&format!("#{connected_component}"))
                && value.contains("P3 (5400, 0)")
                && value.contains(&format!("#{separate_component}"))
        }),
        "net browser detail rows should summarize trace path endpoints: {segment_rows:?}"
    );
    assert!(
        segment_rows
            .iter()
            .any(|(key, value)| key == "Trace path segments" && value == "2 total"),
        "net browser detail rows should summarize trace path segments: {segment_rows:?}"
    );
    assert!(
        segment_rows.iter().any(|(key, value)| {
            key == "Trace path status" && value == "1 connected / 1 disconnected"
        }),
        "net browser detail rows should summarize trace path segment status: {segment_rows:?}"
    );
    let total_path_length = app.format_layout_length(5_400.0);
    let connected_path_length = app.format_layout_length(1_000.0);
    let blocked_path_length = app.format_layout_length(4_400.0);
    assert!(
        segment_rows.iter().any(|(key, value)| {
            key == "Trace path length"
                && value.contains(&total_path_length)
                && value.contains(&connected_path_length)
                && value.contains(&blocked_path_length)
        }),
        "net browser detail rows should summarize trace path lengths: {segment_rows:?}"
    );
    assert!(
        segment_rows.iter().any(|(key, value)| {
            key == "Trace path blocker"
                && value.contains("S2 1000,0..5400,0")
                && value.contains("Disconnected")
        }),
        "net browser detail rows should expose the first blocking trace path segment: {segment_rows:?}"
    );
    assert!(
        segment_rows
            .iter()
            .any(|(key, value)| key == "Trace points" && value == "3 total"),
        "net browser detail rows should summarize trace points: {segment_rows:?}"
    );
    assert!(
        segment_rows
            .iter()
            .any(|(key, value)| key == "Trace point nets" && value == "3 netted"),
        "net browser detail rows should summarize trace point nets: {segment_rows:?}"
    );
    let trace_point_entries = layout_trace_point_entries(&app);
    assert_eq!(trace_point_entries.len(), 3);
    assert!(
        trace_point_entries[0]
            .1
            .contains(&format!("#{connected_component}")),
        "first route point should summarize the connected net: {trace_point_entries:?}"
    );
    assert!(
        trace_point_entries[0].1.contains("(0, 0)"),
        "first route point should include its coordinate: {trace_point_entries:?}"
    );
    assert!(
        trace_point_entries[2]
            .1
            .contains(&format!("#{separate_component}")),
        "third route point should summarize the separate net: {trace_point_entries:?}"
    );
    assert!(
        trace_point_entries[2].1.contains("(5400, 0)"),
        "third route point should include its coordinate: {trace_point_entries:?}"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_between"));
    assert!(
        app.status_message().contains("segment 2 disconnected"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "multi-segment trace failure should leave trace history intact"
    );
    let segment_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with route path segments should build");
    for node_name in [
        "glassworks.layout.trace_points.title",
        "glassworks.viewctl.layout.trace_point.0",
        "glassworks.viewctl.layout.trace_point.1",
        "glassworks.viewctl.layout.trace_point.2",
        "glassworks.viewctl.layout.trace_point.remove.0",
        "glassworks.viewctl.layout.trace_point.remove.1",
        "glassworks.viewctl.layout.trace_point.remove.2",
        "glassworks.viewctl.layout.trace_point.select_first",
        "glassworks.layout.trace_path_segments.title",
        "glassworks.viewctl.layout.trace_path_segment.0",
        "glassworks.viewctl.layout.trace_path_segment.1",
        "glassworks.viewctl.layout.trace_path_segment.focus_blocker",
    ] {
        assert!(
            segment_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "net browser should expose {node_name}"
        );
    }
    let route_points_before_focus = app.route_points.clone();
    let trace_history_before_focus = app.layout_trace_history.clone();
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_segment.focus_blocker")
    );
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert!(
        app.status_message()
            .contains("Focused trace path segment 2 1000,0..5400,0"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_point.2"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(separate_component)
    );
    assert!(
        app.status_message().contains("Selected trace point 3")
            && app.status_message().contains("(5400, 0)")
            && app
                .status_message()
                .contains(&format!("#{separate_component}")),
        "{}",
        app.status_message()
    );
    app.set_layout_browser_search("5400 0");
    assert_eq!(
        layout_trace_point_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![2],
        "trace point browser search should match point coordinates"
    );
    app.set_layout_browser_search("point=3");
    assert_eq!(
        layout_trace_point_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![2],
        "trace point selector search should match point indices"
    );
    app.set_layout_browser_search(&format!("component={separate_component}"));
    assert_eq!(
        layout_trace_point_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![2],
        "trace point selector search should match connected component ids"
    );
    app.set_layout_browser_search("p3");
    let trace_point_search_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with searched route points should build");
    assert!(
        !trace_point_search_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_point.0" }),
        "shared browser search should hide nonmatching trace points"
    );
    assert!(
        trace_point_search_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_point.2" }),
        "shared browser search should keep matching trace points"
    );
    assert!(
        trace_point_search_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_point.select_first" }),
        "searched trace points should expose first matching selection"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_point.select_first"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(separate_component)
    );
    assert!(
        app.status_message().contains("Selected trace point 3"),
        "{}",
        app.status_message()
    );
    let trace_point_search_rows = layout_net_browser_rows(&app);
    assert!(
        trace_point_search_rows
            .iter()
            .any(|(key, value)| key == "Trace points" && value == "1 listed / 3 total"),
        "net browser detail rows should summarize searched trace points: {trace_point_search_rows:?}"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_segment.1"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert!(
        app.status_message()
            .contains("Focused trace path segment 2 1000,0..5400,0"),
        "{}",
        app.status_message()
    );
    app.set_layout_browser_search("s1");
    assert_eq!(
        layout_trace_path_segment_net_entries(&app)
            .into_iter()
            .map(|(index, _)| index)
            .collect::<Vec<_>>(),
        vec![0],
        "connected trace path segment net cross-probes should use shared browser search"
    );
    let connected_segment_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with connected route path segment should build");
    assert!(
        connected_segment_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_path_segment_net.0" }),
        "connected route path segments should expose net selection"
    );
    assert!(
        !connected_segment_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_path_segment_net.1" }),
        "disconnected route path segments should not expose net selection"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_segment_net.0"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(connected_component)
    );
    assert!(
        app.status_message()
            .contains("Selected trace path segment 1 net"),
        "{}",
        app.status_message()
    );
    app.set_layout_browser_search("disconnected");
    let filtered_segment_entries = layout_trace_path_segment_entries(&app);
    assert_eq!(
        filtered_segment_entries
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>(),
        vec![1],
        "shared browser search should filter route path segments"
    );
    assert_eq!(
        layout_trace_path_segment_endpoint_entries(&app)
            .into_iter()
            .map(|(key, label)| (key, label))
            .collect::<Vec<_>>(),
        vec![
            (
                "1.start".to_string(),
                compact_button_label(
                    &format!(
                        "Select S2 Start Net #{} {}",
                        connected_component,
                        layout_trace_path_component_name(&app, connected_component)
                    ),
                    30
                )
            ),
            (
                "1.end".to_string(),
                compact_button_label(
                    &format!(
                        "Select S2 End Net #{} {}",
                        separate_component,
                        layout_trace_path_component_name(&app, separate_component)
                    ),
                    30
                )
            )
        ],
        "disconnected trace path segments should expose start/end net cross-probes"
    );
    let filtered_segment_rows = layout_net_browser_rows(&app);
    assert!(
        filtered_segment_rows
            .iter()
            .any(|(key, value)| { key == "Trace path segments" && value == "1 listed / 2 total" }),
        "net browser detail rows should summarize filtered trace path segments: {filtered_segment_rows:?}"
    );
    let filtered_segment_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with filtered route path segments should build");
    assert!(
        !filtered_segment_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_path_segment.0" }),
        "shared browser search should hide nonmatching route path segments"
    );
    assert!(
        filtered_segment_document
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.trace_path_segment.1" }),
        "shared browser search should keep matching route path segments"
    );
    assert!(
        filtered_segment_document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.trace_path_segment.select_first"
        }),
        "filtered route path segments should expose first matching focus"
    );
    for node_name in [
        "glassworks.viewctl.layout.trace_path_segment_endpoint.1.start",
        "glassworks.viewctl.layout.trace_path_segment_endpoint.1.end",
        "glassworks.viewctl.layout.trace_path_endpoint.start",
        "glassworks.viewctl.layout.trace_path_endpoint.end",
        "glassworks.viewctl.layout.trace_path_blocker_endpoint.start",
        "glassworks.viewctl.layout.trace_path_blocker_endpoint.end",
    ] {
        assert!(
            filtered_segment_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "filtered disconnected route path segment should expose {node_name}"
        );
    }
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_segment.select_first")
    );
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(app.layout_trace_history, trace_history_before_focus);
    assert!(
        app.status_message()
            .contains("Focused trace path segment 2 1000,0..5400,0"),
        "{}",
        app.status_message()
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_blocker_endpoint.start")
    );
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(connected_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component],
        "blocker endpoint net cross-probe should preserve deduplicated trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path blocker segment 2 start net #{} {}",
            connected_component,
            layout_trace_path_component_name(&app, connected_component)
        )),
        "{}",
        app.status_message()
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_blocker_endpoint.end")
    );
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(separate_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![separate_component, connected_component],
        "blocker end net cross-probe should append selected endpoint net to trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path blocker segment 2 end net #{} {}",
            separate_component,
            layout_trace_path_component_name(&app, separate_component)
        )),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_endpoint.start"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(connected_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![connected_component, separate_component],
        "path start net cross-probe should move the start endpoint net to the front of trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path start net #{} {}",
            connected_component,
            layout_trace_path_component_name(&app, connected_component)
        )),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_endpoint.end"));
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(separate_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![separate_component, connected_component],
        "path end net cross-probe should move the end endpoint net to the front of trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path end net #{} {}",
            separate_component,
            layout_trace_path_component_name(&app, separate_component)
        )),
        "{}",
        app.status_message()
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.trace_path_segment_endpoint.1.end")
    );
    assert_eq!(app.route_points, route_points_before_focus);
    assert_eq!(
        selected_layout_net_component_id(
            &app,
            &app.connectivity_report()
                .expect("test connectivity should still extract")
        ),
        Some(separate_component)
    );
    assert_eq!(
        app.layout_trace_history,
        vec![separate_component, connected_component],
        "disconnected endpoint net cross-probe should append selected endpoint net to trace history"
    );
    assert!(
        app.status_message().contains(&format!(
            "Selected trace path segment 2 end net #{} {}",
            separate_component,
            layout_trace_path_component_name(&app, separate_component)
        )),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.route_points, route_points_before_focus,
        "endpoint net cross-probe should preserve pending route points"
    );
    app.set_layout_browser_search("");
    let full_trace_point_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with unfiltered trace points should build");
    assert!(
        full_trace_point_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.trace_point.remove.1"),
        "unfiltered trace point rows should expose individual removal"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.trace_point.remove.1"));
    assert_eq!(
        app.route_points,
        vec![Point::new(0, 0), Point::new(5_400, 0)]
    );
    assert_eq!(
        app.layout_trace_history,
        vec![separate_component, connected_component],
        "trace-point removal should preserve trace history"
    );
    assert!(
        app.status_message()
            .contains("Removed trace point 2 (2 remaining)"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.route_points.clear"));
    assert!(
        app.route_points.is_empty(),
        "route-point clearing should remove pending trace path points"
    );
    assert_eq!(
        app.layout_trace_history,
        vec![separate_component, connected_component],
        "route-point clearing should preserve trace history"
    );
    assert!(
        app.status_message().contains("Cleared 2 route points"),
        "{}",
        app.status_message()
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
pub(crate) fn layout_drc_can_run_inside_current_cell_selected_region() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("current cell selected region drc test");
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
    let top_shape = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(20, 20))),
        )
        .expect("top-level DRC fixture should be added");
    let child = app.workspace.document.create_cell("drc_region_child");
    let region_id = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            annotation,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(50_000, 50_000))),
        )
        .expect("current-cell selected DRC region should be added");
    let inside_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::new(
                Point::new(10_000, 20_000),
                Point::new(10_020, 20_020),
            )),
        )
        .expect("inside current-cell narrow test rectangle should be added");
    let outside_shape = app
        .workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::new(
                Point::new(200_000, 20_000),
                Point::new(200_020, 20_020),
            )),
        )
        .expect("outside current-cell narrow test rectangle should be added");
    app.workspace
        .document
        .insert_instance_in_top(child, Transform::translate(300_000, 0))
        .expect("child should be inserted");

    let rules = RuleDeck::demo(&app.workspace.document);
    let full_violations = run_drc(&app.workspace.document, &rules);
    assert!(
        full_violations.len() >= 3,
        "fixture should create top-level, inside, and outside DRC violations"
    );

    app.layout_view_top_cell = child;
    app.selected_layout_shape = Some(region_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(region_id));
    let status = app.run_drc_for_selected_region_summary();
    assert!(
        status.contains("DRC region found") && status.contains(" of "),
        "{status}"
    );
    let report = app
        .drc_report()
        .expect("current-cell selected-region DRC should populate the report cache");
    assert!(
        !report.violations.is_empty(),
        "current-cell selected-region DRC should keep the inside violation"
    );
    assert!(
        report.violations.iter().all(|violation| {
            violation.shape_ids.contains(&inside_shape)
                && !violation.shape_ids.contains(&outside_shape)
                && !violation.shape_ids.contains(&top_shape)
        }),
        "current-cell selected-region DRC should only report local markers inside the selected region"
    );
    let region_points = region_points_for_shape_kind(
        &app.workspace
            .document
            .cell(child)
            .and_then(|cell| cell.shapes.get(&region_id))
            .expect("selected region should still exist")
            .kind,
    )
    .expect("selected current-cell region should be rectangular");
    assert!(
        report
            .violations
            .iter()
            .all(|violation| drc_violation_intersects_region(violation, &region_points)),
        "cached DRC markers should intersect the selected current-cell region"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.run_drc_region"));
    assert!(
        app.status_message().contains("DRC region found"),
        "{}",
        app.status_message()
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
