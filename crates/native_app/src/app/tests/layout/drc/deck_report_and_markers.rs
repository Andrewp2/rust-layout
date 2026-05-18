#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_drc_deck_exchange_round_trips_max_width_area_rules_and_accepts_legacy_json() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let poly = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Poly)
        .expect("default technology should include poly");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    let metal1_name = app
        .workspace
        .document
        .layers
        .get(&metal1)
        .expect("metal1 layer should exist")
        .name
        .clone();
    let poly_name = app
        .workspace
        .document
        .layers
        .get(&poly)
        .expect("poly layer should exist")
        .name
        .clone();
    let metal2_name = app
        .workspace
        .document
        .layers
        .get(&metal2)
        .expect("metal2 layer should exist")
        .name
        .clone();
    let mut deck = RuleDeck::demo(&app.workspace.document);
    deck.derived_layers.push(DerivedLayerRule {
        name: "m1_poly_overlap".to_string(),
        operation: DerivedLayerOperation::And,
        a: metal1,
        b: poly,
    });
    deck.derived_layers.push(DerivedLayerRule {
        name: "metal_routing".to_string(),
        operation: DerivedLayerOperation::Or,
        a: metal1,
        b: metal2,
    });
    deck.derived_layers.push(DerivedLayerRule {
        name: "m1_without_poly".to_string(),
        operation: DerivedLayerOperation::Not,
        a: metal1,
        b: poly,
    });
    deck.derived_min_width
        .insert("m1_poly_overlap".to_string(), 120);
    deck.derived_max_width
        .insert("m1_poly_overlap".to_string(), 900);
    deck.derived_min_area
        .insert("m1_poly_overlap".to_string(), 200);
    deck.derived_max_area
        .insert("m1_poly_overlap".to_string(), 2_000);
    deck.derived_min_spacing
        .insert("m1_poly_overlap".to_string(), 350);
    deck.derived_min_edge_spacing
        .insert("m1_poly_overlap".to_string(), 275);
    deck.derived_forbidden_overlaps
        .push(DerivedForbiddenOverlapRule {
            derived: "m1_poly_overlap".to_string(),
            layer: metal2,
            name: "derived_overlap".to_string(),
        });
    deck.max_width.insert(metal1, 1_500);
    deck.min_area.insert(metal1, 50_000);
    deck.max_area.insert(metal1, 500_000);
    deck.min_edge_spacing.insert(metal1, 250);

    let exchange = LayoutDrcDeckExchange::from_app(&app, &deck);
    assert!(exchange.derived_layers.iter().any(|rule| {
        rule.name == "m1_poly_overlap"
            && rule.operation == DerivedLayerOperation::And
            && rule.a == metal1_name
            && rule.b == poly_name
    }));
    assert!(exchange.derived_layers.iter().any(|rule| {
        rule.name == "metal_routing"
            && rule.operation == DerivedLayerOperation::Or
            && rule.a == metal1_name
            && rule.b == metal2_name
    }));
    assert!(exchange.derived_layers.iter().any(|rule| {
        rule.name == "m1_without_poly"
            && rule.operation == DerivedLayerOperation::Not
            && rule.a == metal1_name
            && rule.b == poly_name
    }));
    assert_eq!(
        exchange
            .derived_min_width
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(120)
    );
    assert_eq!(
        exchange
            .derived_max_width
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(900)
    );
    assert_eq!(
        exchange
            .derived_min_area
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(200)
    );
    assert_eq!(
        exchange
            .derived_max_area
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(2_000)
    );
    assert_eq!(
        exchange
            .derived_min_spacing
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(350)
    );
    assert_eq!(
        exchange
            .derived_min_edge_spacing
            .iter()
            .find(|rule| rule.layer == "m1_poly_overlap")
            .map(|rule| rule.value),
        Some(275)
    );
    assert!(exchange.derived_forbidden_overlaps.iter().any(|rule| {
        rule.derived == "m1_poly_overlap"
            && rule.layer == metal2_name
            && rule.name == "derived_overlap"
    }));
    assert_eq!(
        exchange
            .max_width
            .iter()
            .find(|rule| rule.layer == metal1_name)
            .map(|rule| rule.value),
        Some(1_500)
    );
    assert_eq!(
        exchange
            .min_area
            .iter()
            .find(|rule| rule.layer == metal1_name)
            .map(|rule| rule.value),
        Some(50_000)
    );
    assert_eq!(
        exchange
            .max_area
            .iter()
            .find(|rule| rule.layer == metal1_name)
            .map(|rule| rule.value),
        Some(500_000)
    );
    assert_eq!(
        exchange
            .min_edge_spacing
            .iter()
            .find(|rule| rule.layer == metal1_name)
            .map(|rule| rule.value),
        Some(250)
    );
    let imported = exchange
        .clone()
        .into_rule_deck(&app.workspace.document)
        .expect("exported width/area deck should import");
    assert_eq!(imported.derived_layers.len(), 3);
    assert!(imported.derived_layers.iter().any(|rule| {
        rule.name == "m1_poly_overlap"
            && rule.operation == DerivedLayerOperation::And
            && rule.a == metal1
            && rule.b == poly
    }));
    assert!(imported.derived_layers.iter().any(|rule| {
        rule.name == "metal_routing"
            && rule.operation == DerivedLayerOperation::Or
            && rule.a == metal1
            && rule.b == metal2
    }));
    assert!(imported.derived_layers.iter().any(|rule| {
        rule.name == "m1_without_poly"
            && rule.operation == DerivedLayerOperation::Not
            && rule.a == metal1
            && rule.b == poly
    }));
    assert_eq!(
        imported.derived_min_width.get("m1_poly_overlap"),
        Some(&120)
    );
    assert_eq!(
        imported.derived_max_width.get("m1_poly_overlap"),
        Some(&900)
    );
    assert_eq!(imported.derived_min_area.get("m1_poly_overlap"), Some(&200));
    assert_eq!(
        imported.derived_max_area.get("m1_poly_overlap"),
        Some(&2_000)
    );
    assert_eq!(
        imported.derived_min_spacing.get("m1_poly_overlap"),
        Some(&350)
    );
    assert_eq!(
        imported.derived_min_edge_spacing.get("m1_poly_overlap"),
        Some(&275)
    );
    assert_eq!(imported.derived_forbidden_overlaps.len(), 1);
    assert_eq!(
        imported.derived_forbidden_overlaps[0].derived,
        "m1_poly_overlap"
    );
    assert_eq!(imported.derived_forbidden_overlaps[0].layer, metal2);
    assert_eq!(
        imported.derived_forbidden_overlaps[0].name,
        "derived_overlap"
    );
    assert_eq!(imported.max_width.get(&metal1), Some(&1_500));
    assert_eq!(imported.min_area.get(&metal1), Some(&50_000));
    assert_eq!(imported.max_area.get(&metal1), Some(&500_000));
    assert_eq!(imported.min_edge_spacing.get(&metal1), Some(&250));

    let legacy_json = serde_json::json!({
        "schema_version": LAYOUT_DRC_DECK_EXCHANGE_SCHEMA_VERSION,
        "document_id": "legacy",
        "document_name": "legacy deck",
        "layout_revision": 0,
        "grid": 10,
        "min_width": [],
        "min_spacing": [],
        "via_enclosure": [],
        "forbidden_overlaps": []
    });
    let legacy = serde_json::from_value::<LayoutDrcDeckExchange>(legacy_json)
        .expect("legacy DRC deck JSON without max_width/min_area/max_area should parse");

    assert!(legacy.max_width.is_empty());
    assert!(legacy.min_area.is_empty());
    assert!(legacy.max_area.is_empty());
    assert!(legacy.min_edge_spacing.is_empty());
    assert!(legacy.derived_layers.is_empty());
    assert!(legacy.derived_min_width.is_empty());
    assert!(legacy.derived_max_width.is_empty());
    assert!(legacy.derived_min_area.is_empty());
    assert!(legacy.derived_max_area.is_empty());
    assert!(legacy.derived_min_spacing.is_empty());
    assert!(legacy.derived_min_edge_spacing.is_empty());
    assert!(legacy.derived_forbidden_overlaps.is_empty());
}

#[test]
pub(crate) fn layout_drc_marker_category_classifies_min_area_rules() {
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("max_width"),
        LayoutDrcMarkerCategory::Width
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("min_area"),
        LayoutDrcMarkerCategory::Area
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("max_area"),
        LayoutDrcMarkerCategory::Area
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("derived_min_area.m1_poly_overlap"),
        LayoutDrcMarkerCategory::Area
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("derived_min_width.gate"),
        LayoutDrcMarkerCategory::Width
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("min_edge_spacing"),
        LayoutDrcMarkerCategory::Spacing
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("derived_min_spacing.gate"),
        LayoutDrcMarkerCategory::Spacing
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("derived_min_edge_spacing.gate"),
        LayoutDrcMarkerCategory::Spacing
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_rule("derived_overlap"),
        LayoutDrcMarkerCategory::Overlap
    );
    assert_eq!(
        LayoutDrcMarkerCategory::from_slug("area"),
        Some(LayoutDrcMarkerCategory::Area)
    );
    assert_eq!(LayoutDrcMarkerCategory::Area.slug(), "area");
    assert_eq!(LayoutDrcMarkerCategory::Area.label(), "Area");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_deck_export_import_overrides_future_runs_and_sessions() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc deck exchange source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    assert!(
        app.drc_report()
            .is_some_and(|report| !report.violations.is_empty()),
        "built-in DRC deck should flag the narrow rectangle"
    );

    let deck_path = std::env::temp_dir().join(format!(
        "glassworks-drc-deck-exchange-{}.json",
        std::process::id()
    ));
    let session_path = std::env::temp_dir().join(format!(
        "glassworks-drc-deck-session-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&deck_path);
    let _ = std::fs::remove_file(&session_path);
    assert!(app.export_layout_drc_deck_to_path(&deck_path));
    assert_eq!(app.app_options.files.recent_files[0].kind, "drc_deck");
    let mut exchange: LayoutDrcDeckExchange = serde_json::from_str(
        &std::fs::read_to_string(&deck_path).expect("DRC deck JSON should be readable"),
    )
    .expect("DRC deck JSON should parse");
    let metal1_name = app
        .workspace
        .document
        .layers
        .get(&metal1)
        .expect("metal1 layer should exist")
        .name
        .clone();
    let rule = exchange
        .min_width
        .iter_mut()
        .find(|rule| rule.layer == metal1_name)
        .expect("exported DRC deck should include metal1 width");
    rule.value = 10;
    std::fs::write(
        &deck_path,
        serde_json::to_vec_pretty(&exchange).expect("modified DRC deck should serialize"),
    )
    .expect("modified DRC deck should be written");

    assert!(app.import_layout_drc_deck_from_path(&deck_path));
    assert!(app.layout_custom_drc_deck.is_some());
    assert!(
        app.status_message().contains("Imported DRC deck"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.files.recent_files[0].kind, "drc_deck");
    let status = app.run_drc_summary();
    assert!(
        status.contains("0 violation") && status.contains("custom"),
        "{status}"
    );
    assert!(
        app.drc_report()
            .is_some_and(|report| report.violations.is_empty()),
        "custom DRC deck should override future full DRC runs"
    );
    let rows = layout_technology_stack_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "DRC deck" && value.contains("custom")),
        "technology stack rows should show the custom DRC deck"
    );

    assert!(app.save_app_session_to_path(&session_path));
    let mut loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(loaded.load_app_session_from_path(&session_path));
    assert!(loaded.layout_custom_drc_deck.is_some());
    let loaded_status = loaded.run_drc_summary();
    assert!(
        loaded_status.contains("0 violation") && loaded_status.contains("custom"),
        "{loaded_status}"
    );

    assert!(app.reset_layout_drc_deck());
    assert!(app.layout_custom_drc_deck.is_none());
    let status = app.run_drc_summary();
    assert!(status.contains("built-in"), "{status}");
    assert!(
        app.drc_report()
            .is_some_and(|report| !report.violations.is_empty()),
        "reset should restore built-in DRC behavior"
    );

    let _ = std::fs::remove_file(&deck_path);
    let _ = std::fs::remove_file(&session_path);
}

#[test]
pub(crate) fn layout_drc_marker_browser_selects_filters_and_updates_marker_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker browser test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    let shape_id = app
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(
                Point::new(10_000, 20_000),
                Point::new(10_020, 20_020),
            )),
        )
        .expect("narrow test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));

    let entries = layout_drc_marker_entries(&app);
    let entry = entries
        .first()
        .cloned()
        .expect("DRC marker browser should list active violations");
    let action = format!("glassworks.viewctl.layout.drc_marker.{}", entry.id);
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with DRC marker browser should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.drc_marker_browser.title"),
        "layout side panel should expose a DRC marker browser"
    );
    assert!(
        document.nodes().iter().any(|node| node.name() == action),
        "DRC marker browser should expose {action}"
    );

    let before_pan = app.layout_pan();
    assert!(app.apply_clicked_node_name(&action));
    assert_eq!(
        app.layout_selected_drc_marker_key.as_deref(),
        Some(entry.key.as_str())
    );
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(shape_id))
    );
    assert!(app.show_drc_overlay);
    assert_ne!(
        app.layout_pan(),
        before_pan,
        "selecting a DRC marker should center the layout view on the marker"
    );

    let selected_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("selected marker key should be retained");
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| state.visited),
        "selecting a DRC marker should mark it visited"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_toggle.important"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| state.important),
        "important action should persist marker priority"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_note.owner"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.note.as_deref())
            == Some("needs owner follow-up"),
        "note action should persist marker review notes"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_owner.layout"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.owner.as_deref())
            == Some("layout-team"),
        "owner action should persist marker ownership"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.accepted"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.signoff.as_deref())
            == Some("accepted"),
        "signoff action should persist marker signoff metadata"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_tag.fix"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.tags.get("action"))
            .map(String::as_str)
            == Some("fix"),
        "tag action should persist marker tagged values"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.noted"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "noted filter should include markers with review notes"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.visited"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "visited filter should include visited markers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.important"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "important filter should include important markers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.owned"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "owned filter should include markers with review owners"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.signed_off"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "signed-off filter should include markers with signoff metadata"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.tagged"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "tagged filter should include markers with tagged values"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.active"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_toggle.waived"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| state.waived),
        "waive action should persist marker state"
    );
    assert!(
        !layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "active filter should omit waived markers"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.waived"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.rule"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Rule);
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "waived filter should include waived markers"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    for key in [
        "Marker id",
        "Rule",
        "State",
        "Visited",
        "Important",
        "Note",
        "Owner",
        "Signoff",
        "Tags",
        "Shapes",
        "Bounds",
        "Required",
    ] {
        assert!(
            marker_rows.iter().any(|(row_key, _)| row_key == key),
            "DRC marker browser detail rows should expose property column {key}"
        );
    }
    let directory_rows = layout_drc_marker_directory_rows(&app);
    assert!(
        directory_rows
            .iter()
            .any(|(row_key, value)| row_key != "Directories" && value.contains("active")),
        "DRC marker directory rows should expose rule-path counts"
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    for key in ["Directory", "Message", "Stable key", "Occurrences"] {
        assert!(
            info_rows.iter().any(|(row_key, _)| row_key == key),
            "DRC marker info pane should expose selected marker detail {key}"
        );
    }
    let filtered = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with waived DRC marker should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_report_export",
        "glassworks.viewctl.layout.drc_report_import",
        "glassworks.viewctl.layout.drc_markers.write_layer",
        "glassworks.viewctl.layout.drc_marker_filter.waived",
        "glassworks.viewctl.layout.drc_marker_filter.visited",
        "glassworks.viewctl.layout.drc_marker_filter.important",
        "glassworks.viewctl.layout.drc_marker_filter.noted",
        "glassworks.viewctl.layout.drc_marker_filter.owned",
        "glassworks.viewctl.layout.drc_marker_filter.signed_off",
        "glassworks.viewctl.layout.drc_marker_filter.tagged",
        "glassworks.viewctl.layout.drc_marker_filter.snapshots",
        "glassworks.viewctl.layout.drc_marker_sort.rule",
        "glassworks.viewctl.layout.drc_marker_toggle.hidden",
        "glassworks.viewctl.layout.drc_marker_toggle.visited",
        "glassworks.viewctl.layout.drc_marker_toggle.important",
        "glassworks.viewctl.layout.drc_marker_note.reviewed",
        "glassworks.viewctl.layout.drc_marker_note.owner",
        "glassworks.viewctl.layout.drc_marker_note.waiver",
        "glassworks.viewctl.layout.drc_marker_note.clear",
        "glassworks.viewctl.layout.drc_marker_owner.layout",
        "glassworks.viewctl.layout.drc_marker_owner.process",
        "glassworks.viewctl.layout.drc_marker_owner.qa",
        "glassworks.viewctl.layout.drc_marker_owner.clear",
        "glassworks.viewctl.layout.drc_marker_signoff.needs_review",
        "glassworks.viewctl.layout.drc_marker_signoff.accepted",
        "glassworks.viewctl.layout.drc_marker_signoff.rejected",
        "glassworks.viewctl.layout.drc_marker_signoff.clear",
        "glassworks.viewctl.layout.drc_marker_tag.fix",
        "glassworks.viewctl.layout.drc_marker_tag.false_positive",
        "glassworks.viewctl.layout.drc_marker_tag.source_external",
        "glassworks.viewctl.layout.drc_marker_snapshot",
        "glassworks.viewctl.layout.drc_marker_tag.clear",
        "glassworks.viewctl.layout.drc_marker_clear_state",
    ] {
        assert!(
            filtered.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker browser control should exist: {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_clear_state"));
    assert!(
        !app.workspace
            .document
            .marker_states
            .contains_key(&selected_key),
        "clearing marker state should remove default marker state records"
    );
}

#[test]
pub(crate) fn layout_drc_marker_browser_select_first_uses_search() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker select first");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let probe_bounds = Rect::new(Point::new(0, 0), Point::new(20, 20));
    let target_bounds = Rect::new(Point::new(2_000, 0), Point::new(2_020, 20));
    let probe = app
        .add_layout_shape(metal1, ShapeKind::Rectangle(probe_bounds))
        .expect("probe shape should be added");
    let target = app
        .add_layout_shape(metal1, ShapeKind::Rectangle(target_bounds))
        .expect("target shape should be added");
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;

    let probe_violation = DrcViolation {
        id: 1,
        rule: "probe".to_string(),
        message: "probe marker".to_string(),
        shape_ids: vec![probe],
        occurrence_ids: vec![ShapeOccurrenceId::top_level(probe)],
        bounds: probe_bounds,
        required: 100,
        actual: 20.0,
    };
    let target_violation = DrcViolation {
        id: 2,
        rule: "target".to_string(),
        message: "target marker".to_string(),
        shape_ids: vec![target],
        occurrence_ids: vec![ShapeOccurrenceId::top_level(target)],
        bounds: target_bounds,
        required: 100,
        actual: 20.0,
    };
    let target_key = target_violation.stable_key();
    *app.drc_report_cache.get_mut() = Some(DrcReportCacheEntry {
        revision: app.layout_revision,
        value: DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![probe_violation, target_violation],
        },
    });
    app.set_layout_browser_search("target");

    let entries = layout_drc_marker_entries(&app);
    assert_eq!(entries.len(), 1, "search should isolate the target marker");
    assert_eq!(entries[0].id, 2);
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with marker select-first should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.drc_marker_browser.select_first"
        }),
        "DRC marker browser should expose select-first when search has a match"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_browser.select_first"));
    assert_eq!(
        app.layout_selected_drc_marker_key.as_deref(),
        Some(target_key.as_str())
    );
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(target))
    );
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&target_key)
            .is_some_and(|state| state.visited),
        "select-first should mark the DRC marker visited"
    );
    assert!(app.status_message().contains("Selected browser DRC marker"));
}

#[test]
pub(crate) fn layout_drc_marker_report_writes_annotation_geometry() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker annotation output");
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
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(40_003, 20_007),
            Point::new(41_003, 21_007),
        )),
    )
    .expect("off-grid test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    let report = app.drc_report().expect("DRC report should be cached");
    assert!(
        report.violations.len() >= 2,
        "test fixture should produce at least two DRC markers"
    );
    let hidden_key = report.violations[0].stable_key();
    app.workspace.document.marker_states.insert(
        hidden_key,
        MarkerState {
            hidden: true,
            ..MarkerState::default()
        },
    );
    let active_count = report
        .violations
        .iter()
        .filter(|violation| drc_violation_is_active(&app.workspace.document, violation))
        .count();
    assert!(active_count > 0);
    let before_count = app.workspace.document.shapes.len();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.write_layer"));
    assert!(
        app.status_message().contains("Wrote")
            && app.status_message().contains("skipped 1 hidden/waived"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.workspace.document.shapes.len(),
        before_count + active_count * 2
    );
    let generated = app
        .workspace
        .document
        .shapes
        .values()
        .filter(|shape| {
            shape.layer == annotation
                && shape
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with("drc marker #"))
        })
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), active_count * 2);
    assert!(
        generated
            .iter()
            .any(|shape| matches!(shape.kind, ShapeKind::Rectangle(_)))
    );
    assert!(generated.iter().any(|shape| {
        matches!(
            &shape.kind,
            ShapeKind::Label { position, text }
                if text.starts_with("DRC #")
                    && position.x % app.workspace.document.grid == 0
                    && position.y % app.workspace.document.grid == 0
        )
    }));
    assert!(
        generated
            .iter()
            .any(|shape| app.selected_layout_shape == Some(shape.id))
    );
    assert_eq!(app.active_layer, annotation);

    assert!(app.undo_layout_operation());
    assert_eq!(app.workspace.document.shapes.len(), before_count);
    assert!(app.redo_layout_operation());
    assert_eq!(
        app.workspace.document.shapes.len(),
        before_count + active_count * 2
    );
}

#[test]
pub(crate) fn layout_drc_marker_browser_groups_and_filters_rule_categories() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker category test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(40_003, 20_007),
            Point::new(41_003, 21_007),
        )),
    )
    .expect("off-grid test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));

    let category_entries = layout_drc_marker_category_entries(&app);
    assert!(
        category_entries
            .iter()
            .any(|entry| entry.category == LayoutDrcMarkerCategory::Grid),
        "category browser should expose grid markers"
    );
    assert!(
        category_entries
            .iter()
            .any(|entry| entry.category == LayoutDrcMarkerCategory::Width),
        "category browser should expose width markers"
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with DRC marker categories should build");
    for node_name in [
        "glassworks.layout.drc_marker_categories.title",
        "glassworks.viewctl.layout.drc_marker_category.all",
        "glassworks.viewctl.layout.drc_marker_category.grid",
        "glassworks.viewctl.layout.drc_marker_category.width",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker category control should exist: {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_category.width"));
    assert_eq!(
        app.layout_drc_marker_category_filter,
        Some(LayoutDrcMarkerCategory::Width)
    );
    let width_entries = layout_drc_marker_entries(&app);
    assert!(
        !width_entries.is_empty(),
        "width category should include width markers"
    );
    let report = app.drc_report().expect("DRC report should be cached");
    for entry in &width_entries {
        let violation = report
            .violations
            .iter()
            .find(|violation| violation.stable_key() == entry.key)
            .expect("entry should reference a DRC violation");
        assert_eq!(
            LayoutDrcMarkerCategory::from_rule(&violation.rule),
            LayoutDrcMarkerCategory::Width
        );
    }
    assert!(
        layout_drc_marker_rows(&app)
            .iter()
            .any(|(key, value)| key == "Category" && value == "Width"),
        "DRC marker rows should report the active category filter"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_category.grid"));
    let grid_entries = layout_drc_marker_entries(&app);
    assert!(
        !grid_entries.is_empty(),
        "grid category should include off-grid markers"
    );
    let report = app.drc_report().expect("DRC report should be cached");
    for entry in &grid_entries {
        let violation = report
            .violations
            .iter()
            .find(|violation| violation.stable_key() == entry.key)
            .expect("entry should reference a DRC violation");
        assert_eq!(
            LayoutDrcMarkerCategory::from_rule(&violation.rule),
            LayoutDrcMarkerCategory::Grid
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_category.all"));
    assert_eq!(app.layout_drc_marker_category_filter, None);
    assert!(layout_drc_marker_entries(&app).len() >= width_entries.len() + grid_entries.len());
}

#[test]
pub(crate) fn layout_drc_report_browser_tracks_multiple_reports() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc report history test");
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
    app.active_layer = metal1;
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

    assert!(app.run_drc_summary().contains("violation"));
    let full_report_id = app
        .layout_selected_drc_report_id
        .expect("full DRC report should become active");
    let full_count = app
        .drc_report()
        .expect("full DRC report should be active")
        .violations
        .len();
    assert!(
        full_count >= 2,
        "fixture should create inside and outside markers"
    );

    app.selected_layout_shape = Some(region_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(region_id));
    assert!(
        app.run_drc_for_selected_region_summary()
            .contains("DRC region found")
    );
    let region_report_id = app
        .layout_selected_drc_report_id
        .expect("region DRC report should become active");
    assert_ne!(region_report_id, full_report_id);
    assert_eq!(app.layout_drc_report_history.len(), 2);
    let region_count = app
        .drc_report()
        .expect("region DRC report should be active")
        .violations
        .len();
    assert!(
        region_count < full_count,
        "region report should keep fewer markers than full DRC"
    );

    let full_select = format!("glassworks.viewctl.layout.drc_report.select.{full_report_id}");
    assert!(app.apply_clicked_node_name(&full_select));
    assert_eq!(app.layout_selected_drc_report_id, Some(full_report_id));
    assert_eq!(
        app.drc_report()
            .expect("selected full DRC report should be active")
            .violations
            .len(),
        full_count
    );
    let rows = layout_drc_marker_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Reports" && value == "2")
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Active report" && value.contains("Full DRC"))
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with DRC report browser should build");
    for node_name in [
        "glassworks.layout.drc_reports.title".to_string(),
        "glassworks.viewctl.layout.drc_report_database_export".to_string(),
        "glassworks.viewctl.layout.drc_report_database_import".to_string(),
        full_select,
        format!("glassworks.viewctl.layout.drc_report.select.{region_report_id}"),
        format!("glassworks.viewctl.layout.drc_report.delete.{full_report_id}"),
        "glassworks.viewctl.layout.drc_reports.clear".to_string(),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC report browser control should exist: {node_name}"
        );
    }

    let delete_full = format!("glassworks.viewctl.layout.drc_report.delete.{full_report_id}");
    assert!(app.apply_clicked_node_name(&delete_full));
    assert_eq!(app.layout_drc_report_history.len(), 1);
    assert_eq!(app.layout_selected_drc_report_id, Some(region_report_id));
    assert_eq!(
        app.drc_report()
            .expect("remaining region report should be active")
            .violations
            .len(),
        region_count
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_reports.clear"));
    assert!(app.layout_drc_report_history.is_empty());
    assert!(app.layout_selected_drc_report_id.is_none());
    assert!(app.drc_report().is_none());
}

#[test]
pub(crate) fn layout_drc_marker_browser_applies_custom_tags_from_search_replace_fields() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc custom marker tag test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    let entry = layout_drc_marker_entries(&app)
        .first()
        .cloned()
        .expect("DRC should produce a marker entry");
    assert!(app.select_layout_drc_marker(&entry.id.to_string()));
    let selected_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("selected marker key should be retained");

    app.set_layout_browser_search("severity");
    app.set_layout_browser_replace("critical");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with custom marker tag controls should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_marker_custom_tag.apply",
        "glassworks.viewctl.layout.drc_marker_custom_tag.remove",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker browser should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_custom_tag.apply"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.tags.get("severity"))
            .map(String::as_str)
            == Some("critical"),
        "custom marker tag should persist in marker state"
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "custom marker tag should be searchable by key"
    );
    let rows = layout_drc_marker_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Tags" && value.contains("severity=critical")),
        "marker detail rows should show custom tags"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_custom_tag.remove"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| !state.tags.contains_key("severity")),
        "custom marker tag removal should persist"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_marker_snapshot_exports_rgba_and_tags_marker() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker snapshot test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    let entry = layout_drc_marker_entries(&app)
        .first()
        .cloned()
        .expect("DRC should produce a marker entry");
    assert!(app.select_layout_drc_marker(&entry.id.to_string()));
    let selected_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("selected marker key should be retained");

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with marker snapshot control should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.drc_marker_snapshot"),
        "DRC marker browser should expose snapshot export"
    );

    let snapshot_path = std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-test-{}.rgba",
        std::process::id()
    ));
    let report_path = std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-report-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&snapshot_path);
    let _ = std::fs::remove_file(&report_path);
    assert!(app.export_selected_layout_drc_marker_snapshot_to_path(&snapshot_path, 320, 200));
    assert!(
        app.status_message()
            .contains("Exported DRC marker snapshot"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        std::fs::metadata(&snapshot_path)
            .expect("snapshot export should write raw RGBA")
            .len(),
        320 * 200 * 4
    );
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("snapshot export should tag the selected marker");
    assert!(layout_marker_state_has_snapshot(state));
    assert_eq!(
        state.tags.get("screenshot").map(String::as_str),
        Some(snapshot_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        state.tags.get("screenshot_size").map(String::as_str),
        Some("320x200")
    );
    assert!(
        state
            .tags
            .get("screenshot_bounds")
            .is_some_and(|bounds| !bounds.trim().is_empty()),
        "snapshot export should record the marker view bounds"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.snapshots"));
    assert_eq!(
        app.layout_drc_marker_filter,
        LayoutDrcMarkerFilter::Snapshots
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "snapshot filter should include markers with exported screenshot tags"
    );
    let filtered = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with snapshot marker filter should build");
    assert!(
        filtered
            .nodes()
            .iter()
            .any(|node| { node.name() == "glassworks.viewctl.layout.drc_marker_filter.snapshots" }),
        "DRC marker browser should expose snapshot filter"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Snapshot" && value.contains("320x200")),
        "marker detail rows should expose exported snapshot metadata: {marker_rows:?}"
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Snapshot" && value.contains("320x200")),
        "marker info pane should expose exported snapshot metadata: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Snapshot bounds" && value != "None"),
        "marker info pane should expose exported snapshot bounds: {info_rows:?}"
    );

    assert!(app.export_layout_drc_report_to_path(&report_path));
    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(imported.import_layout_drc_report_from_path(&report_path));
    assert_eq!(
        imported
            .workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.tags.get("screenshot_size"))
            .map(String::as_str),
        Some("320x200"),
        "DRC report JSON should carry marker snapshot metadata"
    );
    let _ = std::fs::remove_file(&snapshot_path);
    let _ = std::fs::remove_file(&report_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_export_import_round_trips_report_history() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc report database source");
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
    app.active_layer = metal1;
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

    assert!(app.run_drc_summary().contains("violation"));
    let full_report_id = app
        .layout_selected_drc_report_id
        .expect("full DRC should be selected");
    let full_count = app
        .drc_report()
        .expect("full DRC report should be active")
        .violations
        .len();
    app.selected_layout_shape = Some(region_id);
    app.selected_layout_occurrence = Some(ShapeOccurrenceId::top_level(region_id));
    assert!(
        app.run_drc_for_selected_region_summary()
            .contains("DRC region found")
    );
    assert!(app.select_layout_drc_report(&full_report_id.to_string()));
    let entry = layout_drc_marker_entries(&app)
        .first()
        .cloned()
        .expect("full DRC should list markers");
    assert!(app.select_layout_drc_marker(&entry.id.to_string()));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_toggle.important"));
    let selected_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("selected marker key should be retained");

    let path = std::env::temp_dir().join(format!(
        "glassworks-drc-report-database-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(app.export_layout_drc_report_database_to_path(&path));
    assert!(
        app.status_message()
            .contains("Exported DRC report database"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.app_options.files.recent_files[0].kind,
        "drc_report_database"
    );
    let contents = std::fs::read_to_string(&path).expect("report database should write JSON");
    let exchange: LayoutDrcReportDatabaseExchange =
        serde_json::from_str(&contents).expect("report database should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_DRC_REPORT_DATABASE_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.reports.len(), 2);
    assert_eq!(exchange.active_report_index, Some(1));
    assert!(
        exchange
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| state.visited && state.important)
    );
    app.layout_drc_report_history.clear();
    app.layout_selected_drc_report_id = None;
    app.drc_report_cache.get_mut().take();
    assert!(app.reload_recent_layout_file());
    assert_eq!(app.layout_drc_report_history.len(), 2);
    assert!(
        app.status_message()
            .contains("Imported DRC report database"),
        "{}",
        app.status_message()
    );

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(imported.import_layout_drc_report_database_from_path(&path));
    assert!(
        imported
            .status_message()
            .contains("Imported DRC report database"),
        "{}",
        imported.status_message()
    );
    assert_eq!(imported.layout_drc_report_history.len(), 2);
    let active_report = imported
        .selected_drc_report_history_entry()
        .expect("imported database should restore active report");
    assert_eq!(active_report.label, "Full DRC");
    assert_eq!(active_report.value.violations.len(), full_count);
    assert!(
        imported
            .workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| state.visited && state.important)
    );
    let document = imported
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("imported report database UI should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_report_database_export",
        "glassworks.viewctl.layout.drc_report_database_import",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC report database control should exist: {node_name}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_export_import_round_trips_markers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc report source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.active_layer = metal1;
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(
            Point::new(10_000, 20_000),
            Point::new(10_020, 20_020),
        )),
    )
    .expect("narrow test rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    let entry = layout_drc_marker_entries(&app)
        .first()
        .cloned()
        .expect("DRC should produce a marker entry");
    assert!(app.select_layout_drc_marker(&entry.id.to_string()));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_toggle.important"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_note.reviewed"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_owner.process"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.needs_review"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_tag.source_external"));
    let selected_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("selected marker key should be retained");
    let path = std::env::temp_dir().join(format!(
        "glassworks-drc-report-menu-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_layout_drc_report_to_path(&path));
    assert!(
        app.status_message().contains("Exported DRC report"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.files.recent_files[0].kind, "drc_report");
    let contents = std::fs::read_to_string(&path).expect("DRC report should write JSON");
    let exchange: LayoutDrcReportExchange =
        serde_json::from_str(&contents).expect("exported DRC report should parse");
    assert_eq!(
        exchange.schema_version,
        LAYOUT_DRC_REPORT_EXCHANGE_SCHEMA_VERSION
    );
    assert_eq!(exchange.violations.len(), 1);
    assert!(
        exchange
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| {
                state.visited
                    && state.important
                    && state.note.as_deref() == Some("reviewed in marker browser")
                    && state.owner.as_deref() == Some("process-owner")
                    && state.signoff.as_deref() == Some("needs_review")
                    && state.tags.get("source").map(String::as_str) == Some("external")
            })
    );

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(imported.drc_report().is_none());
    assert!(imported.import_layout_drc_report_from_path(&path));
    assert!(imported.show_drc_overlay);
    assert!(
        imported.status_message().contains("Imported DRC report"),
        "{}",
        imported.status_message()
    );
    assert_eq!(
        imported.app_options.files.recent_files[0].kind,
        "drc_report"
    );
    let report = imported
        .drc_report()
        .expect("imported DRC report should populate the cache");
    assert_eq!(report.violations.len(), 1);
    assert!(
        imported
            .workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| {
                state.visited
                    && state.important
                    && state.note.as_deref() == Some("reviewed in marker browser")
                    && state.owner.as_deref() == Some("process-owner")
                    && state.signoff.as_deref() == Some("needs_review")
                    && state.tags.get("source").map(String::as_str) == Some("external")
            })
    );
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("tools menu should build");
    for node_name in [
        "glassworks.menu.item.layout.drc_report_export",
        "glassworks.menu.item.layout.drc_report_import",
        "glassworks.menu.item.layout.drc_report_database_export",
        "glassworks.menu.item.layout.drc_report_database_import",
        "glassworks.menu.item.layout.calibre_rve_import",
        "glassworks.menu.item.layout.drc_markers.write_layer",
        "glassworks.menu.item.layout.drc_marker_snapshot",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Tools menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}
