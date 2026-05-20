#![allow(unused_imports)]
use super::*;
use crate::*;
use ::drc::import_klayout_rdb_markers;

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

#[test]
pub(crate) fn layout_drc_marker_directory_uses_preserved_klayout_category_parts() {
    let violation = DrcViolation {
        id: 1,
        rule: "klayout.layer_s_group.rule_name".to_string(),
        message: "KLayout imported marker".to_string(),
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds: Rect::new(Point::new(0, 0), Point::new(10, 10)),
        required: 0,
        actual: 0.0,
    };
    let state = MarkerState {
        tags: BTreeMap::from([
            (
                "rdb_category".to_string(),
                "Layer's.Group/Rule.Name".to_string(),
            ),
            (
                "rdb_category_part_1".to_string(),
                "Layer's.Group".to_string(),
            ),
            ("rdb_category_part_2".to_string(), "Rule.Name".to_string()),
        ]),
        ..MarkerState::default()
    };

    assert_eq!(
        layout_drc_marker_user_category_path(Some(&state)).as_deref(),
        Some("User / Layer's.Group / Rule.Name")
    );
    assert_eq!(
        layout_drc_marker_directory_path_with_state(&violation, Some(&state)),
        "User / Layer's.Group / Rule.Name"
    );
    assert_eq!(
        layout_drc_marker_category_label(&violation, Some(&state)),
        "User / Layer's.Group / Rule.Name"
    );
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
    let poly = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Poly)
        .expect("default technology should include poly");
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
    let object_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected marker object controls should build");
    assert!(
        object_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.drc_marker_objects.title"),
        "selected DRC marker should expose source-object cross-probe controls"
    );
    let source_shape_action = format!(
        "glassworks.viewctl.layout.drc_marker_source_shape.{}",
        shape_id.0
    );
    assert!(
        object_document
            .nodes()
            .iter()
            .any(|node| node.name() == source_shape_action),
        "selected DRC marker should expose source shape cross-probe action"
    );
    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    assert!(app.apply_clicked_node_name(&source_shape_action));
    assert_eq!(app.selected_layout_shape, Some(shape_id));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(shape_id)),
        "source-shape cross-probe should restore the marker source occurrence"
    );
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
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("signoff action should persist marker signoff metadata");
    assert_eq!(state.signoff.as_deref(), Some("accepted"));
    assert_eq!(state.signoff_by.as_deref(), Some("layout-team"));
    assert_eq!(state.signoff_note.as_deref(), Some("needs owner follow-up"));
    let layout_signoff = state.signoff_records.get("layout-team").unwrap();
    assert_eq!(layout_signoff.status, "accepted");
    assert_eq!(layout_signoff.role.as_deref(), Some("layout"));
    assert_eq!(layout_signoff.by.as_deref(), Some("layout-team"));
    assert_eq!(
        layout_signoff.note.as_deref(),
        Some("needs owner follow-up")
    );
    assert!(
        layout_signoff
            .recorded_at
            .as_deref()
            .is_some_and(|recorded_at| recorded_at.starts_with("layout_revision:"))
    );
    assert!(
        layout_signoff
            .recorded_at
            .as_deref()
            .is_some_and(|recorded_at| recorded_at.contains(";unix_seconds:")),
        "signoff audit stamp should include a wall-clock component"
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
    let selected_rule = app
        .drc_report()
        .and_then(|report| {
            report
                .violations
                .into_iter()
                .find(|violation| violation.stable_key() == selected_key)
                .map(|violation| violation.rule)
        })
        .expect("selected marker should still be present in the DRC report");
    for query in [
        "note=owner".to_string(),
        "owner=layout-team".to_string(),
        "signoff=accepted".to_string(),
        "signoff_by=layout-team".to_string(),
        "signoff_note=owner follow-up".to_string(),
        "signoff_detail=layout-team".to_string(),
        "signoff_party=layout-team".to_string(),
        "signoff_status=accepted".to_string(),
        "signoff_role=layout".to_string(),
        "signoff_at=layout_revision".to_string(),
        "signoff_record=owner follow-up".to_string(),
        "action=fix".to_string(),
        "tag=fix".to_string(),
        format!("rule={selected_rule}"),
        format!("source_shape={}", shape_id.0),
        "source_object=rectangle".to_string(),
        format!("source_layer=L{}", metal1.0),
        format!("source_layer_id={}", metal1.0),
        "source_kind=rectangle".to_string(),
        "source_bounds=10000,20000".to_string(),
    ] {
        app.set_layout_browser_search(&query);
        assert!(
            layout_drc_marker_entries(&app)
                .iter()
                .any(|entry| entry.key == selected_key),
            "DRC marker selector search should match {query:?}"
        );
    }
    app.set_layout_browser_search("owner=process-team");
    assert!(
        !layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "DRC marker selector search should reject mismatched owner metadata"
    );
    app.set_layout_browser_search(&format!("source_layer=L{}", poly.0));
    assert!(
        !layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "DRC marker selector search should reject mismatched source layers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_note.reviewed"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_owner.qa"));
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.needs_review")
    );
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("multi-party signoff records should persist on the marker");
    assert_eq!(state.signoff_records.len(), 2);
    let qa_signoff = state.signoff_records.get("qa-review").unwrap();
    assert_eq!(qa_signoff.status, "needs_review");
    assert_eq!(qa_signoff.role.as_deref(), Some("quality"));
    assert_eq!(qa_signoff.by.as_deref(), Some("qa-review"));
    assert_eq!(
        qa_signoff.note.as_deref(),
        Some("reviewed in marker browser")
    );
    assert!(
        qa_signoff
            .recorded_at
            .as_deref()
            .is_some_and(|recorded_at| recorded_at.starts_with("layout_revision:"))
    );
    assert!(
        qa_signoff
            .recorded_at
            .as_deref()
            .is_some_and(|recorded_at| recorded_at.contains(";unix_seconds:")),
        "multi-party signoff audit stamp should include a wall-clock component"
    );
    for query in [
        "signoff_party=qa-review",
        "signoff_status=needs_review",
        "signoff_role=quality",
        "signoff_recorded_at=layout_revision",
        "signoff_at=unix_seconds",
        "signoff_record=layout-team",
        "signoff_record_note=reviewed in marker browser",
    ] {
        app.set_layout_browser_search(query);
        assert!(
            layout_drc_marker_entries(&app)
                .iter()
                .any(|entry| entry.key == selected_key),
            "DRC marker selector search should match multi-party signoff {query:?}"
        );
    }
    app.set_layout_browser_search("");
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
    assert!(app.apply_clicked_node_name(
        "glassworks.viewctl.layout.drc_marker_filter.signoff_needs_review"
    ));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "needs-review filter should include markers with current or recorded needs-review signoffs"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.signoff_accepted")
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "accepted filter should include markers with recorded accepted signoffs"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.signoff_rejected")
    );
    assert!(
        !layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "rejected filter should omit markers without rejected signoffs"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.tagged"));
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "tagged filter should include markers with tagged values"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.active_layer")
    );
    assert_eq!(
        app.layout_drc_marker_filter,
        LayoutDrcMarkerFilter::ActiveLayer
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "active-layer filter should include markers touching the active layer"
    );
    app.active_layer = poly;
    assert!(
        layout_drc_marker_entries(&app).is_empty(),
        "active-layer filter should omit markers on other layers"
    );
    app.active_layer = metal1;
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_filter.selected_shape")
    );
    assert_eq!(
        app.layout_drc_marker_filter,
        LayoutDrcMarkerFilter::SelectedShape
    );
    let selected_shape_entries = layout_drc_marker_entries(&app);
    assert!(
        selected_shape_entries
            .iter()
            .any(|entry| entry.key == selected_key),
        "selected-shape filter should include markers touching the selected source shape"
    );
    let report = app.drc_report().expect("DRC report should be cached");
    for entry in &selected_shape_entries {
        let violation = report
            .violations
            .iter()
            .find(|violation| violation.stable_key() == entry.key)
            .expect("entry should reference a DRC violation");
        assert!(
            violation.shape_ids.contains(&shape_id)
                || violation
                    .occurrence_ids
                    .iter()
                    .any(|occurrence| occurrence.source_shape_id() == shape_id),
            "selected-shape filter should only list markers touching the selected source shape"
        );
    }
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
        "Signoff by",
        "Signoff note",
        "Signoff records",
        "Tags",
        "Shapes",
        "Shape ids",
        "Occurrence labels",
        "Source cells",
        "Source objects",
        "Source layers",
        "Source kinds",
        "Source bounds",
        "Cross-probe targets",
        "Bounds",
        "Center",
        "Size",
        "Area",
        "Required",
        "Listed review states",
        "Listed signoff audit",
        "Listed tags",
        "Listed snapshots",
    ] {
        assert!(
            marker_rows.iter().any(|(row_key, _)| row_key == key),
            "DRC marker browser detail rows should expose property column {key}"
        );
    }
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Source objects"
                && value.contains(&format!("#{}", shape_id.0))
                && value.contains("rectangle")
                && value.contains(&format!("L{}", metal1.0))
        }),
        "DRC marker browser detail rows should expose source object kind/layer summaries: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Source layers" && value.contains(&format!("L{}", metal1.0))),
        "DRC marker browser detail rows should expose source layers: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Source kinds" && value.contains("rectangle")),
        "DRC marker browser detail rows should expose source shape kinds: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Source bounds" && value.contains(',')),
        "DRC marker browser detail rows should expose source shape bounds: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Cross-probe targets"
                && value.contains("shape")
                && value.contains("cell")
                && value.contains("occurrence")),
        "DRC marker browser detail rows should summarize cross-probe target counts: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Signoff records"
                && value.contains("layout-team=accepted")
                && value.contains("role=layout")
                && value.contains("qa-review=needs_review")
                && value.contains("role=quality")
                && value.contains("at=layout_revision:")
        }),
        "DRC marker browser detail rows should expose multi-party signoff records: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed review states"
                && value.contains("waived=1")
                && value.contains("visited=1")
                && value.contains("important=1")
                && value.contains("noted=1")
                && value.contains("owned=1")
                && value.contains("signed=1")
                && value.contains("needs-review=1")
                && value.contains("accepted=1")
                && value.contains("tagged=1")
        }),
        "DRC marker rows should summarize listed marker review state counts: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed signoff audit"
                && value.contains("2 signoff records")
                && value.contains("2 parties")
                && value.contains("layout-team")
                && value.contains("qa-review")
                && value.contains("accepted=1")
                && value.contains("needs_review=1")
                && value.contains("layout=1")
                && value.contains("quality=1")
                && value.contains("newest layout_revision:")
        }),
        "DRC marker rows should summarize listed marker signoff audit records: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed tags"
                && value.contains("1 tagged marker")
                && value.contains("1 key")
                && value.contains("action=1")
                && value.contains("action=fix")
        }),
        "DRC marker rows should summarize listed marker tags: {marker_rows:?}"
    );
    let directory_rows = layout_drc_marker_directory_rows(&app);
    assert!(
        directory_rows
            .iter()
            .any(|(row_key, value)| row_key != "Directories" && value.contains("active")),
        "DRC marker directory rows should expose rule-path counts"
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    for key in [
        "Report database",
        "Report",
        "Report markers",
        "Listed review states",
        "Listed signoff audit",
        "Listed tags",
        "Directory",
        "Message",
        "Stable key",
        "Center",
        "Size",
        "Area",
        "Shape ids",
        "Occurrences",
        "Occurrence labels",
        "Source cells",
        "Source objects",
        "Source layers",
        "Source kinds",
        "Source bounds",
        "Cross-probe targets",
        "Signoff by",
        "Signoff note",
        "Signoff detail",
        "Signoff records",
    ] {
        assert!(
            info_rows.iter().any(|(row_key, _)| row_key == key),
            "DRC marker info pane should expose selected marker detail {key}"
        );
    }
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Center" && value.contains(',')),
        "DRC marker info pane should expose marker center coordinates: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Size" && value.contains(" x ")),
        "DRC marker info pane should expose marker width/height: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Area" && value.ends_with("dbu^2")),
        "DRC marker info pane should expose marker area: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Shape ids" && value.contains('#')),
        "DRC marker info pane should expose source shape IDs: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Source objects"
                && value.contains(&format!("#{}", shape_id.0))
                && value.contains("rectangle")
                && value.contains(&format!("L{}", metal1.0))
        }),
        "DRC marker info pane should expose source object kind/layer summaries: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Source layers" && value.contains(&format!("L{}", metal1.0))),
        "DRC marker info pane should expose source layers: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Source kinds" && value.contains("rectangle")),
        "DRC marker info pane should expose source shape kinds: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Source bounds" && value.contains(',')),
        "DRC marker info pane should expose source shape bounds: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Cross-probe targets"
                && value.contains("shape")
                && value.contains("cell")
                && value.contains("occurrence")),
        "DRC marker info pane should summarize cross-probe target counts: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Signoff records"
                && value.contains("layout-team=accepted")
                && value.contains("role=layout")
                && value.contains("qa-review=needs_review")
                && value.contains("role=quality")
                && value.contains("at=layout_revision:")
        }),
        "DRC marker info pane should expose multi-party signoff records: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Listed review states"
                && value.contains("waived=1")
                && value.contains("visited=1")
                && value.contains("important=1")
                && value.contains("noted=1")
                && value.contains("owned=1")
                && value.contains("signed=1")
                && value.contains("needs-review=1")
                && value.contains("accepted=1")
                && value.contains("tagged=1")
        }),
        "DRC marker info pane should summarize listed marker review state counts: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Listed signoff audit"
                && value.contains("2 signoff records")
                && value.contains("2 parties")
                && value.contains("layout-team")
                && value.contains("qa-review")
                && value.contains("accepted=1")
                && value.contains("needs_review=1")
                && value.contains("layout=1")
                && value.contains("quality=1")
                && value.contains("newest layout_revision:")
        }),
        "DRC marker info pane should summarize listed marker signoff audit records: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Listed tags"
                && value.contains("1 tagged marker")
                && value.contains("1 key")
                && value.contains("action=1")
                && value.contains("action=fix")
        }),
        "DRC marker info pane should summarize listed marker tags: {info_rows:?}"
    );
    let filtered = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with waived DRC marker should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_report_export",
        "glassworks.viewctl.layout.drc_report_import",
        "glassworks.viewctl.layout.drc_markers.write_layer",
        "glassworks.viewctl.layout.drc_markers.clear_states",
        "glassworks.viewctl.layout.drc_markers.clear_tags",
        "glassworks.viewctl.layout.drc_marker_filter.waived",
        "glassworks.viewctl.layout.drc_marker_filter.selected_shape",
        "glassworks.viewctl.layout.drc_marker_filter.active_layer",
        "glassworks.viewctl.layout.drc_marker_filter.visited",
        "glassworks.viewctl.layout.drc_marker_filter.important",
        "glassworks.viewctl.layout.drc_marker_filter.noted",
        "glassworks.viewctl.layout.drc_marker_filter.owned",
        "glassworks.viewctl.layout.drc_marker_filter.signed_off",
        "glassworks.viewctl.layout.drc_marker_filter.signoff_needs_review",
        "glassworks.viewctl.layout.drc_marker_filter.signoff_accepted",
        "glassworks.viewctl.layout.drc_marker_filter.signoff_rejected",
        "glassworks.viewctl.layout.drc_marker_filter.tagged",
        "glassworks.viewctl.layout.drc_marker_filter.snapshots",
        "glassworks.viewctl.layout.drc_marker_sort.rule",
        "glassworks.viewctl.layout.drc_marker_sort.source_cell",
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
        "glassworks.viewctl.layout.drc_marker_tag.category_litho",
        "glassworks.viewctl.layout.drc_marker_snapshot",
        "glassworks.viewctl.layout.drc_marker_snapshot_png",
        "glassworks.viewctl.layout.drc_marker_tag.clear",
        "glassworks.viewctl.layout.drc_marker_clear_state",
    ] {
        assert!(
            filtered.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker browser control should exist: {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_tags"));
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("bulk tag clearing should preserve non-tag marker state");
    assert!(
        state.tags.is_empty(),
        "bulk tag clearing should remove active marker tags"
    );
    assert!(
        state.important
            && state.note.as_deref() == Some("reviewed in marker browser")
            && state.owner.as_deref() == Some("qa-review")
            && state.signoff.as_deref() == Some("needs_review")
            && state.signoff_by.as_deref() == Some("qa-review")
            && state.signoff_note.as_deref() == Some("reviewed in marker browser")
            && state.signoff_records.len() == 2,
        "bulk tag clearing should preserve review state"
    );
    assert!(
        app.status_message()
            .contains("Cleared 1 active DRC marker tag set"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_tags"));
    assert!(
        app.status_message()
            .contains("No active DRC marker tags to clear"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.clear"));
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("signoff clearing should preserve other marker state");
    assert_eq!(state.signoff, None);
    assert_eq!(state.signoff_by, None);
    assert_eq!(state.signoff_note, None);
    assert!(state.signoff_records.is_empty());

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_clear_state"));
    assert!(
        !app.workspace
            .document
            .marker_states
            .contains_key(&selected_key),
        "clearing marker state should remove default marker state records"
    );

    let active_marker_keys = app
        .drc_report()
        .expect("active DRC report should still be available")
        .violations
        .iter()
        .map(|violation| violation.stable_key())
        .collect::<Vec<_>>();
    assert!(
        active_marker_keys.iter().any(|key| key == &selected_key),
        "selected marker should be part of the active report"
    );
    app.workspace.document.marker_states.insert(
        selected_key.clone(),
        MarkerState {
            important: true,
            note: Some("active marker note".to_string()),
            ..MarkerState::default()
        },
    );
    let unrelated_marker_key = "external-report-marker".to_string();
    app.workspace.document.marker_states.insert(
        unrelated_marker_key.clone(),
        MarkerState {
            hidden: true,
            ..MarkerState::default()
        },
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_states"));
    assert!(
        active_marker_keys.iter().all(|key| !app
            .workspace
            .document
            .marker_states
            .contains_key(key)),
        "bulk clearing marker states should remove states for every active report marker"
    );
    assert!(
        app.workspace
            .document
            .marker_states
            .contains_key(&unrelated_marker_key),
        "bulk clearing active marker states should not remove unrelated marker metadata"
    );
    assert!(
        app.drc_report()
            .is_some_and(|report| !report.violations.is_empty()),
        "bulk clearing marker state should leave the active report available"
    );
    assert!(
        app.status_message()
            .contains("Cleared 1 active DRC marker state"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_states"));
    assert!(
        app.status_message()
            .contains("No active DRC marker states to clear"),
        "{}",
        app.status_message()
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
    let title = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.drc_marker_browser.title")
        .unwrap_or_else(|| panic!("DRC marker browser title should exist"));
    let UiContent::Text(title_text) = title.content() else {
        panic!("DRC marker browser title should be text");
    };
    assert_eq!(
        title_text.text,
        "DRC Marker Browser - Active / search target (1 row)"
    );
    let filter_button_label = |document: &UiDocument, node_name: &str| -> String {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .and_then(|node| node.accessibility())
            .and_then(|accessibility| accessibility.label.clone())
            .unwrap_or_else(|| panic!("{node_name} should expose an accessibility label"))
    };
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.drc_marker_filter.active"
        ),
        "Active (1)"
    );
    assert_eq!(
        filter_button_label(&document, "glassworks.viewctl.layout.drc_marker_filter.all"),
        "All (1)"
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.drc_marker_filter.visited"
        ),
        "Visited (0)"
    );

    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_browser.select_first")
    );
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

    app.set_layout_browser_search("missing");
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_browser.select_first")
    );
    assert!(
        app.status_message()
            .contains("DRC marker browser has no matching active markers for search missing"),
        "{}",
        app.status_message()
    );
    let empty_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with empty marker search should build");
    let empty_title = empty_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.drc_marker_browser.title")
        .unwrap_or_else(|| panic!("empty DRC marker browser title should exist"));
    let UiContent::Text(empty_title_text) = empty_title.content() else {
        panic!("empty DRC marker browser title should be text");
    };
    assert_eq!(
        empty_title_text.text,
        "DRC Marker Browser - Active / search missing (0 rows)"
    );
    assert_eq!(
        filter_button_label(
            &empty_document,
            "glassworks.viewctl.layout.drc_marker_filter.active"
        ),
        "Active (0)"
    );
}

#[test]
pub(crate) fn layout_drc_marker_browser_cross_probes_source_occurrences() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("drc marker occurrence cross probe test");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let leaf = app.workspace.document.create_cell("drc occurrence leaf");
    let local_bounds = Rect::new(Point::new(0, 0), Point::new(20, 20));
    let shape_id = app
        .workspace
        .document
        .insert_shape_in_cell(leaf, metal1, ShapeKind::Rectangle(local_bounds))
        .expect("leaf marker shape should be inserted");
    let transform = Transform::translate(10_000, 20_000);
    let instance_id = app
        .workspace
        .document
        .insert_instance_in_top(leaf, transform)
        .expect("leaf marker cell should be instantiated");
    let occurrence = ShapeOccurrenceId::from_instance_path(shape_id, &[instance_id]);
    let occurrence_key = layout_occurrence_action_key(&occurrence);
    app.push_layout_drc_report(
        "Occurrence DRC",
        DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![DrcViolation {
                id: 7,
                rule: "min_width".to_string(),
                message: "hierarchical marker".to_string(),
                shape_ids: vec![shape_id],
                occurrence_ids: vec![occurrence.clone()],
                bounds: transform.apply_rect(local_bounds),
                required: 40,
                actual: 20.0,
            }],
        },
    );
    assert!(app.select_layout_drc_marker("7"));
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed source objects" && value == "1")
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed source object labels"
                && value.contains(&format!("#{}", shape_id.0))
                && value.contains("rectangle")
                && value.contains(&format!("L{}", metal1.0))
        }),
        "DRC marker rows should summarize listed source object labels: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed source cells" && value == "1")
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed source cell labels" && value.contains(&format!("C{}", leaf.0))
        }),
        "DRC marker rows should summarize listed source cell labels: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed source layers" && value.contains(&format!("L{}", metal1.0))
        }),
        "DRC marker rows should summarize listed source layers: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed source kinds" && value.contains("rectangle")),
        "DRC marker rows should summarize listed source kinds: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed source bounds" && value == &rect_summary(local_bounds)
        }),
        "DRC marker rows should summarize listed source-local bounds: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed source occurrences" && value == "1")
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed occurrence labels"
                && value.contains(&format!("shape #{}", shape_id.0))
                && value.contains("instance")
        }),
        "DRC marker rows should summarize listed source occurrence labels: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed cross-probe targets"
                && value.contains("1 shape")
                && value.contains("1 cell")
                && value.contains("1 occurrence")),
        "DRC marker rows should summarize listed cross-probe targets: {marker_rows:?}"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with occurrence marker controls should build");
    let occurrence_action =
        format!("glassworks.viewctl.layout.drc_marker_source_occurrence.{occurrence_key}");
    let source_cell_action = format!(
        "glassworks.viewctl.layout.drc_marker_source_cell.{}",
        leaf.0
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == occurrence_action),
        "selected DRC marker should expose source occurrence cross-probe action"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == source_cell_action),
        "selected DRC marker should expose source cell cross-probe action"
    );

    app.selected_layout_shape = None;
    app.selected_layout_occurrence = None;
    app.active_layer = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Poly)
        .expect("default technology should include poly");
    assert!(app.apply_clicked_node_name(&occurrence_action));
    assert_eq!(app.layout_view_top_cell, app.workspace.document.top_cell);
    assert_eq!(app.selected_layout_shape, Some(shape_id));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(occurrence),
        "source occurrence cross-probe should restore the exact marker occurrence"
    );
    assert_eq!(app.active_layer, metal1);
    assert!(
        app.status_message()
            .contains("Selected DRC marker occurrence"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name(&source_cell_action));
    assert_eq!(app.layout_view_top_cell, leaf);
    assert_eq!(app.selected_layout_shape, Some(shape_id));
    assert_eq!(
        app.selected_layout_occurrence,
        Some(ShapeOccurrenceId::top_level(shape_id)),
        "source cell cross-probe should switch to the marker source cell and select its local shape"
    );
    assert_eq!(app.active_layer, metal1);
    assert!(
        app.status_message()
            .contains("Viewing DRC marker source cell"),
        "{}",
        app.status_message()
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Source objects"
                && value.contains(&format!("#{}", shape_id.0))
                && value.contains("rectangle")
                && value.contains(&format!("L{}", metal1.0))
        }),
        "hierarchical DRC marker info should summarize source objects: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| { key == "Source bounds" && value == &rect_summary(local_bounds) }),
        "hierarchical DRC marker info should use source-local bounds: {info_rows:?}"
    );

    for query in [
        "source_cell=occurrence leaf".to_string(),
        "source_object=rectangle".to_string(),
        format!("source_layer=L{}", metal1.0),
        "source_kind=rectangle".to_string(),
        "source_bounds=0,0".to_string(),
    ] {
        app.set_layout_browser_search(&query);
        assert!(
            layout_drc_marker_entries(&app)
                .iter()
                .any(|entry| entry.id == 7),
            "source-object selector search should match hierarchical marker query {query:?}"
        );
    }
    app.set_layout_browser_search("");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_cell"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::SourceCell);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![7],
        "source-cell sort should keep the marker visible"
    );
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
        "glassworks.layout.drc_marker_directories.title",
        "glassworks.viewctl.layout.drc_marker_directory.all",
        "glassworks.viewctl.layout.drc_marker_directory.0",
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
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed rules" && value.contains("min_width")),
        "DRC marker rows should summarize listed marker rules: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed categories" && value.contains("Width")),
        "DRC marker rows should summarize listed marker categories: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed directories" && value.contains("Width") && value.contains("min_width")
        }),
        "DRC marker rows should summarize listed marker directories: {marker_rows:?}"
    );
    let directory_entries = layout_drc_marker_directory_filter_entries(&app);
    let (directory_index, directory_path) = directory_entries
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.path.contains("Width") && entry.path.contains("min_width"))
        .map(|(index, entry)| (index, entry.path.clone()))
        .expect("width category should expose a width marker directory");
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_marker_directory.{directory_index}"
    )));
    assert_eq!(
        app.layout_drc_marker_directory_filter.as_deref(),
        Some(directory_path.as_str())
    );
    let directory_filtered_entries = layout_drc_marker_entries(&app);
    assert!(
        !directory_filtered_entries.is_empty(),
        "directory filter should keep matching markers"
    );
    let report = app.drc_report().expect("DRC report should be cached");
    for entry in &directory_filtered_entries {
        let violation = report
            .violations
            .iter()
            .find(|violation| violation.stable_key() == entry.key)
            .expect("entry should reference a DRC violation");
        assert_eq!(layout_drc_marker_directory_path(violation), directory_path);
    }
    assert!(
        layout_drc_marker_rows(&app)
            .iter()
            .any(|(key, value)| key == "Directory" && value.contains("Width")),
        "DRC marker rows should report the active directory filter"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_directory.all"));
    assert_eq!(app.layout_drc_marker_directory_filter, None);

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
pub(crate) fn layout_drc_marker_browser_sorts_by_size_and_area() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let source_layer_alpha = app.workspace.document.create_layer(
        "sort source alpha",
        ProcessLayer::Metal1,
        [0.2, 0.6, 0.9, 1.0],
    );
    let source_layer_beta = app.workspace.document.create_layer(
        "sort source beta",
        ProcessLayer::Metal1,
        [0.4, 0.7, 0.4, 1.0],
    );
    let source_layer_gamma = app.workspace.document.create_layer(
        "sort source gamma",
        ProcessLayer::Metal1,
        [0.8, 0.4, 0.2, 1.0],
    );
    let source_for_marker_2 = app.workspace.document.insert_shape(
        source_layer_gamma,
        ShapeKind::Polygon(Polygon::new(vec![
            Point::new(0, 0),
            Point::new(8, 0),
            Point::new(8, 8),
        ])),
    );
    let source_for_marker_3 = app.workspace.document.insert_shape(
        source_layer_alpha,
        ShapeKind::Path {
            points: vec![Point::new(10, 0), Point::new(20, 0)],
            width: 4,
        },
    );
    let source_for_marker_1 = app.workspace.document.insert_shape(
        source_layer_beta,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(40, 0), 5, 5)),
    );
    let violations = vec![
        DrcViolation {
            id: 1,
            rule: "off_grid".to_string(),
            message: "small marker".to_string(),
            shape_ids: vec![source_for_marker_1],
            occurrence_ids: Vec::new(),
            bounds: Rect::from_min_size(Point::new(0, 0), 10, 10),
            required: 20,
            actual: 4.0,
        },
        DrcViolation {
            id: 2,
            rule: "min_area".to_string(),
            message: "largest area marker".to_string(),
            shape_ids: vec![source_for_marker_2],
            occurrence_ids: Vec::new(),
            bounds: Rect::from_min_size(Point::new(20, 0), 10, 30),
            required: 40,
            actual: 1.0,
        },
        DrcViolation {
            id: 3,
            rule: "min_width".to_string(),
            message: "largest extent marker".to_string(),
            shape_ids: vec![source_for_marker_3],
            occurrence_ids: Vec::new(),
            bounds: Rect::from_min_size(Point::new(40, 0), 40, 5),
            required: 10,
            actual: 8.0,
        },
    ];
    app.push_layout_drc_report(
        "Sort DRC",
        DrcReportCacheValue {
            findings: Vec::new(),
            violations,
        },
    );
    let sort_report = app.drc_report().expect("sort report should be cached");
    let signoff_marker_1 = sort_report.violations[0].stable_key();
    let signoff_marker_2 = sort_report.violations[1].stable_key();
    app.workspace.document.marker_states.insert(
        signoff_marker_1,
        MarkerState {
            signoff: Some("needs_review".to_string()),
            signoff_by: Some("qa-review".to_string()),
            signoff_records: std::collections::BTreeMap::from([(
                "qa-review".to_string(),
                layout_model::MarkerSignoffRecord {
                    status: "needs_review".to_string(),
                    role: Some("quality".to_string()),
                    by: Some("qa-review".to_string()),
                    note: Some("quality review".to_string()),
                    recorded_at: Some("layout_revision:3".to_string()),
                },
            )]),
            ..MarkerState::default()
        },
    );
    app.workspace.document.marker_states.insert(
        signoff_marker_2,
        MarkerState {
            signoff: Some("accepted".to_string()),
            signoff_by: Some("process-owner".to_string()),
            signoff_records: std::collections::BTreeMap::from([(
                "process-owner".to_string(),
                layout_model::MarkerSignoffRecord {
                    status: "accepted".to_string(),
                    role: Some("process".to_string()),
                    by: Some("process-owner".to_string()),
                    note: Some("process review".to_string()),
                    recorded_at: Some("layout_revision:1".to_string()),
                },
            )]),
            ..MarkerState::default()
        },
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.area"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Area);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert!(
        layout_drc_marker_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Area"),
        "DRC marker rows should expose area sort"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed marker bounds" && value == "0,0 80x30"),
        "DRC marker rows should summarize listed marker bounds: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed marker ids"
                && value.contains("#1")
                && value.contains("#2")
                && value.contains("#3")
        }),
        "DRC marker rows should summarize listed marker IDs: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed messages"
                && value.contains("small marker")
                && value.contains("largest area marker")
        }),
        "DRC marker rows should summarize listed marker messages: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed marker area" && value.contains("sum 600") && value.contains("max 300")
        }),
        "DRC marker rows should summarize listed marker area rollups: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Listed required"
                && value.contains(&app.format_layout_length(10.0))
                && value.contains(&app.format_layout_length(40.0))
        }),
        "DRC marker rows should summarize listed required range: {marker_rows:?}"
    );
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Listed actual" && value == "1.0..8.0"),
        "DRC marker rows should summarize listed actual range: {marker_rows:?}"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.size"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Size);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.required"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Required);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 1, 3]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.actual"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Actual);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![3, 1, 2]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.category"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Category);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![1, 3, 2]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.directory"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Directory);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 1, 3]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_cell"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::SourceCell);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_object"));
    assert_eq!(
        app.layout_drc_marker_sort,
        LayoutDrcMarkerSort::SourceObject
    );
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_layer"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::SourceLayer);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![3, 1, 2]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_kind"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::SourceKind);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.source_bounds"));
    assert_eq!(
        app.layout_drc_marker_sort,
        LayoutDrcMarkerSort::SourceBounds
    );
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.signoff"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::Signoff);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 1, 3]
    );
    assert!(
        layout_drc_marker_rows(&app)
            .iter()
            .any(|(key, value)| key == "Sort" && value == "Signoff"),
        "DRC marker rows should expose signoff sort"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.signoff_role"));
    assert_eq!(app.layout_drc_marker_sort, LayoutDrcMarkerSort::SignoffRole);
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![2, 1, 3]
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_sort.signoff_stamp"));
    assert_eq!(
        app.layout_drc_marker_sort,
        LayoutDrcMarkerSort::SignoffStamp
    );
    assert_eq!(
        layout_drc_marker_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with DRC marker sort controls should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_marker_sort.area",
        "glassworks.viewctl.layout.drc_marker_sort.size",
        "glassworks.viewctl.layout.drc_marker_sort.required",
        "glassworks.viewctl.layout.drc_marker_sort.actual",
        "glassworks.viewctl.layout.drc_marker_sort.category",
        "glassworks.viewctl.layout.drc_marker_sort.directory",
        "glassworks.viewctl.layout.drc_marker_sort.source_cell",
        "glassworks.viewctl.layout.drc_marker_sort.source_object",
        "glassworks.viewctl.layout.drc_marker_sort.source_layer",
        "glassworks.viewctl.layout.drc_marker_sort.source_kind",
        "glassworks.viewctl.layout.drc_marker_sort.source_bounds",
        "glassworks.viewctl.layout.drc_marker_sort.signoff",
        "glassworks.viewctl.layout.drc_marker_sort.signoff_role",
        "glassworks.viewctl.layout.drc_marker_sort.signoff_stamp",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker browser sort control should exist: {node_name}"
        );
    }
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
    let outside_shape_id = app
        .add_layout_shape(
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
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Listed reports" && value == "2")
    );
    assert!(rows.iter().any(|(key, value)| {
        key == "Stored markers" && value == &(full_count + region_count).to_string()
    }));
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Stored diagnostics" && value == "0")
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Current reports" && value == "2")
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Stale reports" && value == "0")
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key.starts_with("Stored report #")
                && value.contains("Full DRC")
                && value.contains("active"))
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key.starts_with("Stored report #")
                && value.contains("Selected Region DRC")
                && value.contains("stored"))
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report database" && value == "2 reports")
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report" && value.contains("Full DRC"))
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report markers" && value == &full_count.to_string())
    );
    assert!(info_rows.iter().any(|(key, value)| {
        key == "Stored markers" && value == &(full_count + region_count).to_string()
    }));
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Stored diagnostics" && value == "0")
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Current reports" && value == "2")
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Stale reports" && value == "0")
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key.starts_with("Stored report #")
                && value.contains("Full DRC")
                && value.contains("active"))
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key.starts_with("Stored report #")
                && value.contains("Selected Region DRC")
                && value.contains("stored"))
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with DRC report browser should build");
    for node_name in [
        "glassworks.layout.drc_reports.title".to_string(),
        "glassworks.viewctl.layout.drc_report_database_export".to_string(),
        "glassworks.viewctl.layout.drc_report_database_import".to_string(),
        "glassworks.viewctl.layout.klayout_rdb_export".to_string(),
        "glassworks.viewctl.layout.klayout_rdb_import".to_string(),
        full_select.clone(),
        format!("glassworks.viewctl.layout.drc_report.select.{region_report_id}"),
        format!("glassworks.viewctl.layout.drc_report.delete.{full_report_id}"),
        format!("glassworks.viewctl.layout.drc_report.delete.{region_report_id}"),
        "glassworks.viewctl.layout.drc_reports.clear".to_string(),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC report browser control should exist: {node_name}"
        );
    }
    let text_by_node = document_text_by_node(&document);
    assert_eq!(
        text_by_node
            .get("glassworks.layout.drc_reports.title")
            .map(String::as_str),
        Some("DRC Reports (2)")
    );

    app.layout_browser_search = "Full DRC".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![full_report_id]
    );
    assert!(
        layout_drc_marker_rows(&app)
            .iter()
            .any(|(key, value)| key == "Listed reports" && value == "1")
    );
    let searched_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("searched DRC report browser should build");
    let searched_text_by_node = document_text_by_node(&searched_document);
    assert_eq!(
        searched_text_by_node
            .get("glassworks.layout.drc_reports.title")
            .map(String::as_str),
        Some("DRC Reports - search Full DRC (1 / 2)")
    );
    assert!(
        searched_document
            .nodes()
            .iter()
            .any(|node| node.name() == full_select)
    );
    assert!(searched_document.nodes().iter().any(|node| {
        node.name() == format!("glassworks.viewctl.layout.drc_report.delete.{full_report_id}")
    }));
    assert!(searched_document.nodes().iter().any(|node| {
        node.name() == "glassworks.viewctl.layout.drc_report_browser.select_first"
    }));
    assert!(!searched_document.nodes().iter().any(|node| {
        node.name() == format!("glassworks.viewctl.layout.drc_report.select.{region_report_id}")
    }));
    assert!(!searched_document.nodes().iter().any(|node| {
        node.name() == format!("glassworks.viewctl.layout.drc_report.delete.{region_report_id}")
    }));

    app.layout_browser_search = "report=Selected Region".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![region_report_id]
    );
    app.layout_browser_search = "selection=active".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![full_report_id]
    );
    let outside_marker_key = app
        .layout_drc_report_history
        .iter()
        .find(|entry| entry.id == full_report_id)
        .and_then(|entry| {
            entry
                .value
                .violations
                .iter()
                .find(|violation| violation.shape_ids.contains(&outside_shape_id))
                .map(DrcViolation::stable_key)
        })
        .expect("full report should retain marker for outside shape");
    app.workspace.document.marker_states.insert(
        outside_marker_key,
        MarkerState {
            tags: std::collections::BTreeMap::from([(
                "review_bucket".to_string(),
                "outside-only".to_string(),
            )]),
            ..MarkerState::default()
        },
    );
    app.layout_browser_search = format!("source_object={}", outside_shape_id.0);
    assert_eq!(
        layout_drc_report_browser_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![full_report_id],
        "report browser should search marker source objects inside stored reports"
    );
    app.layout_browser_search = "tag=review_bucket=outside-only".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&app)
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![full_report_id],
        "report browser should search marker tags inside stored reports"
    );
    app.layout_browser_search = "report=Selected Region".to_string();
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_report_browser.select_first")
    );
    assert_eq!(app.layout_selected_drc_report_id, Some(region_report_id));
    assert_eq!(
        app.drc_report()
            .expect("selected region report should become active")
            .violations
            .len(),
        region_count
    );
    assert!(
        app.status_message()
            .contains("Selected DRC report Selected Region DRC")
    );

    app.layout_browser_search = "not-present-report".to_string();
    assert!(layout_drc_report_browser_entries(&app).is_empty());
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_report_browser.select_first")
    );
    assert!(
        app.status_message()
            .contains("DRC report browser has no matching reports for search not-present-report"),
        "{}",
        app.status_message()
    );
    let empty_searched_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("searched-empty DRC report browser should build");
    let empty_searched_text_by_node = document_text_by_node(&empty_searched_document);
    assert_eq!(
        empty_searched_text_by_node
            .get("glassworks.layout.drc_reports.title")
            .map(String::as_str),
        Some("DRC Reports - search not-present-report (0 / 2)")
    );
    assert!(
        empty_searched_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.drc_reports.empty")
    );
    app.layout_browser_search.clear();

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
    app.set_layout_browser_search("review_bucket");
    app.set_layout_browser_replace("A");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_custom_tag.apply"));
    let review_bucket_tag_index = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("selected marker should have tag state")
        .tags
        .keys()
        .position(|key| key == "review_bucket")
        .expect("custom marker tag should be directly removable");
    let tag_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with selected marker tag rows should build");
    let remove_review_bucket =
        format!("glassworks.viewctl.layout.drc_marker_tag.remove.{review_bucket_tag_index}");
    assert!(
        tag_document
            .nodes()
            .iter()
            .any(|node| node.name() == remove_review_bucket),
        "DRC marker browser should expose direct selected-tag removal"
    );
    assert!(app.apply_clicked_node_name(&remove_review_bucket));
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("selected marker state should remain after direct tag removal");
    assert!(
        !state.tags.contains_key("review_bucket"),
        "direct selected-tag removal should remove only the selected tag"
    );
    assert_eq!(
        state.tags.get("severity").map(String::as_str),
        Some("critical"),
        "direct selected-tag removal should preserve unrelated tags"
    );

    app.set_layout_browser_search("category");
    app.set_layout_browser_replace("Litho/Hotspots");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_custom_tag.apply"));
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&selected_key)
            .and_then(|state| state.tags.get("category"))
            .map(String::as_str)
            == Some("Litho/Hotspots"),
        "custom marker category tag should persist in marker state"
    );
    app.set_layout_browser_search("");
    assert!(
        layout_drc_marker_directory_entries(&app)
            .iter()
            .any(|entry| entry.path == "User / Litho / Hotspots"),
        "custom category tags should create user-defined marker directory groups"
    );
    let user_directory_paths = layout_drc_marker_directory_entries(&app)
        .into_iter()
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    for expected_path in ["User", "User / Litho", "User / Litho / Hotspots"] {
        assert!(
            user_directory_paths
                .iter()
                .any(|path| path == expected_path),
            "custom category hierarchy should expose ancestor directory {expected_path}: {user_directory_paths:?}"
        );
    }
    assert!(
        layout_drc_marker_directory_rows(&app)
            .iter()
            .any(|(key, _)| key.contains("User / Litho / Hotspots")),
        "marker directory rows should expose user-defined category groups"
    );
    assert!(
        layout_drc_marker_info_rows(&app)
            .iter()
            .any(|(key, value)| key == "Directory" && value.contains("Litho / Hotspots")),
        "marker info rows should expose user-defined category directories"
    );
    for query in [
        "category=hotspots",
        "user_category=litho",
        "directory=hotspots",
    ] {
        app.set_layout_browser_search(query);
        assert!(
            layout_drc_marker_entries(&app)
                .iter()
                .any(|entry| entry.key == selected_key),
            "custom marker category should be selector-searchable by {query:?}"
        );
        let directory_rows = layout_drc_marker_directory_rows(&app)
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            directory_rows.get("Search").map(String::as_str),
            Some(query)
        );
        assert!(
            directory_rows
                .get("Listed directories")
                .is_some_and(|value| value != "0"),
            "marker directory rows should summarize filtered directories for {query:?}: {directory_rows:?}"
        );
        let info_rows = layout_drc_marker_info_rows(&app)
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(info_rows.get("Search").map(String::as_str), Some(query));
        assert!(
            info_rows
                .get("Listed markers")
                .is_some_and(|value| value != "0"),
            "marker info rows should summarize filtered marker matches for {query:?}: {info_rows:?}"
        );
        assert!(
            info_rows
                .get("Directory")
                .is_some_and(|value| value.contains("Litho / Hotspots")),
            "marker info rows should keep the matching category marker for {query:?}: {info_rows:?}"
        );
    }
    app.set_layout_browser_search("");
    let parent_category_index = layout_drc_marker_directory_filter_entries(&app)
        .iter()
        .position(|entry| entry.path == "User / Litho")
        .expect("custom marker category ancestor should be available as a directory filter");
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_marker_directory.{parent_category_index}"
    )));
    assert_eq!(
        app.layout_drc_marker_directory_filter.as_deref(),
        Some("User / Litho"),
        "custom category ancestor directory filter should become active"
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "custom category ancestor directory filter should retain descendant markers"
    );
    assert!(
        layout_drc_marker_directory_entries(&app)
            .iter()
            .any(|entry| entry.path == "User / Litho / Hotspots"),
        "parent directory filter should keep descendant directory rows visible"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_directory.all"));
    let category_index = layout_drc_marker_directory_filter_entries(&app)
        .iter()
        .position(|entry| entry.path == "User / Litho / Hotspots")
        .expect("custom marker category should be available as a directory filter");
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_marker_directory.{category_index}"
    )));
    assert_eq!(
        app.layout_drc_marker_directory_filter.as_deref(),
        Some("User / Litho / Hotspots"),
        "custom category directory filter should become active"
    );
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "custom category directory filter should retain matching markers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_directory.all"));
    assert!(app.select_layout_drc_marker(&entry.id.to_string()));

    app.set_layout_browser_search("severity");
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
pub(crate) fn layout_drc_marker_snapshot_exports_rgba_png_and_tags_marker() {
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
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.drc_marker_snapshot_png"),
        "DRC marker browser should expose PNG snapshot export"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.drc_marker_snapshot.clear"),
        "DRC marker browser should expose snapshot clearing"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.drc_markers.clear_snapshots"),
        "DRC marker browser should expose active-report snapshot clearing"
    );

    let snapshot_path = std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-test-{}.rgba",
        std::process::id()
    ));
    let snapshot_png_path = std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-test-{}.png",
        std::process::id()
    ));
    let report_path = std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-report-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&snapshot_path);
    let _ = std::fs::remove_file(&snapshot_png_path);
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
    assert_eq!(
        state.tags.get("screenshot_format").map(String::as_str),
        Some("RGBA")
    );
    assert!(
        state
            .tags
            .get("screenshot_bounds")
            .is_some_and(|bounds| !bounds.trim().is_empty()),
        "snapshot export should record the marker view bounds"
    );
    let snapshot_entries = layout_drc_marker_snapshot_entries(&app);
    assert_eq!(
        snapshot_entries.len(),
        1,
        "snapshot gallery should list exported active-report marker snapshots"
    );
    let snapshot_entry = snapshot_entries
        .first()
        .expect("snapshot gallery should contain the exported marker");
    assert_eq!(snapshot_entry.key.as_str(), selected_key.as_str());
    assert_eq!(
        snapshot_entry.image_key.as_str(),
        snapshot_path.to_string_lossy().as_ref()
    );
    assert!(
        snapshot_entry.label.contains("320x200"),
        "snapshot gallery label should include screenshot size: {snapshot_entry:?}"
    );
    let snapshot_gallery = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with marker snapshot gallery should build");
    let snapshot_title_text = |document: &UiDocument| -> String {
        let title = document
            .nodes()
            .iter()
            .find(|node| node.name() == "glassworks.layout.drc_marker_snapshots.title")
            .unwrap_or_else(|| {
                panic!("DRC marker browser should expose a snapshot gallery section")
            });
        let UiContent::Text(content) = title.content() else {
            panic!("DRC marker snapshot title should be text");
        };
        content.text.clone()
    };
    assert_eq!(
        snapshot_title_text(&snapshot_gallery),
        "Marker Snapshots (1)"
    );
    assert!(
        snapshot_gallery.nodes().iter().any(|node| {
            node.name()
                == format!(
                    "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}",
                    snapshot_entry.id
                )
        }),
        "DRC marker browser should expose selectable snapshot gallery entries"
    );
    let snapshot_thumbnail = snapshot_gallery
        .nodes()
        .iter()
        .find(|node| {
            node.name()
                == format!(
                    "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}.image",
                    snapshot_entry.id
                )
        })
        .unwrap_or_else(|| panic!("DRC marker snapshot gallery should expose thumbnail image"));
    let UiContent::Canvas(snapshot_thumbnail_canvas) = snapshot_thumbnail.content() else {
        panic!("DRC marker snapshot thumbnail should be a canvas node");
    };
    assert_eq!(
        snapshot_thumbnail_canvas.key.as_str(),
        LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY
    );
    assert_eq!(
        snapshot_thumbnail_canvas.surface_key(),
        snapshot_path.to_string_lossy().as_ref()
    );
    app.set_layout_browser_search("no snapshot match");
    assert!(
        layout_drc_marker_snapshot_entries(&app).is_empty(),
        "snapshot gallery entries should honor the shared marker search"
    );
    assert_eq!(
        layout_drc_marker_snapshot_total_count(&app),
        1,
        "snapshot gallery total should ignore the shared search"
    );
    let empty_snapshot_gallery = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with searched-empty marker snapshot gallery should build");
    assert_eq!(
        snapshot_title_text(&empty_snapshot_gallery),
        "Marker Snapshots - search no snapshot match (0 / 1)"
    );
    assert!(
        empty_snapshot_gallery
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.drc_marker_snapshots.empty"),
        "DRC marker browser should retain searched-empty snapshot gallery context"
    );
    app.set_layout_browser_search("");
    app.layout_selected_drc_marker_key = None;
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}",
        snapshot_entry.id
    )));
    assert_eq!(
        app.layout_selected_drc_marker_key.as_deref(),
        Some(selected_key.as_str()),
        "selecting a snapshot gallery entry should cross-probe to the marker"
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
        info_rows.iter().any(|(key, value)| {
            key == "Listed snapshots"
                && value.contains("1 snapshot")
                && value.contains("RGBA=1")
                && value.contains("320x200=1")
        }),
        "marker info pane should summarize listed snapshot metadata: {info_rows:?}"
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
    assert!(app.export_selected_layout_drc_marker_snapshot_png_to_path(&snapshot_png_path, 96, 64));
    assert!(
        app.status_message().contains("PNG"),
        "{}",
        app.status_message()
    );
    let png =
        std::fs::read(&snapshot_png_path).expect("PNG marker snapshot should write a PNG file");
    assert!(
        png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "PNG marker snapshot should include a PNG signature"
    );
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 96);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 64);
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("PNG snapshot export should retag the selected marker");
    assert_eq!(
        state.tags.get("screenshot").map(String::as_str),
        Some(snapshot_png_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        state.tags.get("screenshot_size").map(String::as_str),
        Some("96x64")
    );
    assert_eq!(
        state.tags.get("screenshot_format").map(String::as_str),
        Some("PNG")
    );
    assert!(
        layout_marker_snapshot_label(Some(state)).contains("PNG"),
        "snapshot labels should expose the exported snapshot format"
    );
    let snapshot_canvas_rgba = layout_drc_marker_snapshot_canvas_rgba(
        &app,
        snapshot_png_path.to_string_lossy().as_ref(),
        PixelSize::new(12, 8),
    )
    .expect("PNG marker snapshots should decode into canvas thumbnail pixels");
    assert_eq!(
        snapshot_canvas_rgba.len(),
        12 * 8 * 4,
        "PNG marker snapshot canvas pixels should match requested canvas size"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_tag.fix"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_snapshot.clear"));
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("snapshot clearing should preserve non-snapshot marker state");
    assert!(
        !layout_marker_state_has_snapshot(state),
        "snapshot clearing should remove screenshot metadata"
    );
    assert_eq!(
        state.tags.get("action").map(String::as_str),
        Some("fix"),
        "snapshot clearing should preserve unrelated marker tags"
    );
    assert!(
        !state.tags.contains_key("screenshot_format"),
        "snapshot clearing should remove snapshot format metadata"
    );
    assert!(
        !layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == selected_key),
        "snapshot filter should omit markers after snapshot metadata is cleared"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_snapshot.clear"));
    assert!(
        app.status_message()
            .contains("No DRC marker snapshot to clear"),
        "{}",
        app.status_message()
    );
    let mut state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .cloned()
        .expect("selected marker should still have non-snapshot metadata");
    state
        .tags
        .insert("screenshot".to_string(), "bulk-snapshot.rgba".to_string());
    state
        .tags
        .insert("screenshot_bounds".to_string(), "1,2 3x4".to_string());
    state
        .tags
        .insert("screenshot_size".to_string(), "64x48".to_string());
    state
        .tags
        .insert("screenshot_format".to_string(), "PNG".to_string());
    app.workspace
        .document
        .marker_states
        .insert(selected_key.clone(), state);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_snapshots"));
    let state = app
        .workspace
        .document
        .marker_states
        .get(&selected_key)
        .expect("bulk snapshot clearing should preserve unrelated marker metadata");
    assert!(
        !layout_marker_state_has_snapshot(state),
        "bulk snapshot clearing should remove screenshot metadata"
    );
    assert_eq!(
        state.tags.get("action").map(String::as_str),
        Some("fix"),
        "bulk snapshot clearing should preserve unrelated marker tags"
    );
    assert!(
        !state.tags.contains_key("screenshot_format"),
        "bulk snapshot clearing should remove snapshot format metadata"
    );
    assert!(
        app.status_message()
            .contains("Cleared 1 active DRC marker snapshot"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_markers.clear_snapshots"));
    assert!(
        app.status_message()
            .contains("No active DRC marker snapshots to clear"),
        "{}",
        app.status_message()
    );
    let _ = std::fs::remove_file(&snapshot_path);
    let _ = std::fs::remove_file(&snapshot_png_path);
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
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_note.reviewed"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_owner.qa"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.accepted"));
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
            .is_some_and(|state| {
                state.visited
                    && state.important
                    && state.signoff.as_deref() == Some("accepted")
                    && state.signoff_by.as_deref() == Some("qa-review")
                    && state.signoff_note.as_deref() == Some("reviewed in marker browser")
                    && state
                        .signoff_records
                        .get("qa-review")
                        .is_some_and(|signoff| {
                            signoff.status == "accepted"
                                && signoff.role.as_deref() == Some("quality")
                                && signoff.by.as_deref() == Some("qa-review")
                                && signoff.note.as_deref() == Some("reviewed in marker browser")
                                && signoff.recorded_at.as_deref().is_some_and(|recorded_at| {
                                    recorded_at.starts_with("layout_revision:")
                                })
                        })
            })
    );
    let append_path = std::env::temp_dir().join(format!(
        "glassworks-drc-report-database-append-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&append_path);
    let mut append_exchange = exchange.clone();
    for report in &mut append_exchange.reports {
        report.source = Some("external-review-rdb-b.rdb".to_string());
    }
    std::fs::write(
        &append_path,
        serde_json::to_vec_pretty(&append_exchange)
            .expect("append report database should serialize"),
    )
    .expect("append report database should write");
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
    assert!(
        active_report
            .source
            .as_deref()
            .is_some_and(|source| source.contains("glassworks-drc-report-database-")),
        "imported report should retain its report-database source"
    );
    assert_eq!(active_report.value.violations.len(), full_count);
    imported.layout_browser_search = "database=glassworks-drc-report-database".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&imported).len(),
        2,
        "report browser should find imported reports by source database"
    );
    imported.layout_browser_search.clear();
    assert!(imported.append_layout_drc_report_database_from_path(&append_path));
    assert!(
        imported
            .status_message()
            .contains("Appended DRC report database"),
        "{}",
        imported.status_message()
    );
    assert_eq!(imported.layout_drc_report_history.len(), 4);
    let appended_active_report = imported
        .selected_drc_report_history_entry()
        .expect("appended database should select its active report");
    assert_eq!(appended_active_report.label, "Full DRC");
    assert_eq!(
        appended_active_report.source.as_deref(),
        Some("external-review-rdb-b.rdb"),
        "appended database should preserve explicit report source metadata"
    );
    let report_source_entries = layout_drc_report_source_entries(&imported);
    assert_eq!(
        report_source_entries.len(),
        2,
        "imported and appended databases should show as separate report sources"
    );
    assert!(
        report_source_entries.iter().any(|entry| {
            entry.source.contains("glassworks-drc-report-database-")
                && entry.total_reports == 2
                && entry.marker_count > 0
        }),
        "original import path should be visible as a report source"
    );
    let appended_source_index = report_source_entries
        .iter()
        .position(|entry| entry.source == "external-review-rdb-b.rdb")
        .expect("appended explicit source should be selectable");
    assert_eq!(
        report_source_entries[appended_source_index].total_reports, 2,
        "appended source should own both appended reports"
    );
    let marker_rows = layout_drc_marker_rows(&imported);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Report sources" && value == "2"),
        "DRC marker rows should summarize loaded report database sources: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Report source labels" && value.contains("external-review")
        }),
        "DRC marker rows should name loaded report database sources: {marker_rows:?}"
    );
    assert!(
        marker_rows.iter().any(|(key, value)| {
            key == "Active report source" && value.contains("external-review-rdb-b")
        }),
        "DRC marker rows should expose the active report source: {marker_rows:?}"
    );
    assert!(imported.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_report_source.{appended_source_index}"
    )));
    assert_eq!(
        imported.layout_browser_search,
        "source=external-review-rdb-b.rdb"
    );
    assert!(
        imported
            .status_message()
            .contains("DRC report source external-review-rdb-b.rdb"),
        "{}",
        imported.status_message()
    );
    assert_eq!(
        layout_drc_report_browser_entries(&imported).len(),
        2,
        "source button should filter the report browser to the selected database source"
    );
    assert!(imported.apply_clicked_node_name("glassworks.viewctl.layout.drc_report_source.all"));
    assert!(imported.layout_browser_search.is_empty());
    imported.layout_browser_search = "source=external-review-rdb-b".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&imported).len(),
        2,
        "report browser should find the appended report database by explicit source"
    );
    imported.layout_browser_search = "database=glassworks-drc-report-database".to_string();
    assert_eq!(
        layout_drc_report_browser_entries(&imported).len(),
        2,
        "report browser should still find the originally imported database by path source"
    );
    imported.layout_browser_search.clear();
    imported.layout_selected_drc_marker_key = Some(selected_key.clone());
    let info_rows = layout_drc_marker_info_rows(&imported);
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Marker reports" && value.parse::<usize>().is_ok_and(|count| count >= 2)
        }),
        "selected marker info should count matching reports across loaded databases: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Marker report sources" && value == "2"),
        "selected marker info should count matching report database sources: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Marker report source labels" && value.contains("external-review")
        }),
        "selected marker info should name matching report database sources: {info_rows:?}"
    );
    assert!(
        imported
            .workspace
            .document
            .marker_states
            .get(&selected_key)
            .is_some_and(|state| {
                state.visited
                    && state.important
                    && state.signoff.as_deref() == Some("accepted")
                    && state.signoff_by.as_deref() == Some("qa-review")
                    && state.signoff_note.as_deref() == Some("reviewed in marker browser")
                    && state
                        .signoff_records
                        .get("qa-review")
                        .is_some_and(|signoff| {
                            signoff.status == "accepted"
                                && signoff.role.as_deref() == Some("quality")
                                && signoff.by.as_deref() == Some("qa-review")
                                && signoff.note.as_deref() == Some("reviewed in marker browser")
                                && signoff.recorded_at.as_deref().is_some_and(|recorded_at| {
                                    recorded_at.starts_with("layout_revision:")
                                })
                        })
            })
    );
    let document = imported
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("imported report database UI should build");
    for node_name in [
        "glassworks.viewctl.layout.drc_report_database_export".to_string(),
        "glassworks.viewctl.layout.drc_report_database_import".to_string(),
        "glassworks.viewctl.layout.drc_report_database_append".to_string(),
        "glassworks.viewctl.layout.klayout_rdb_export".to_string(),
        "glassworks.viewctl.layout.klayout_rdb_import".to_string(),
        "glassworks.viewctl.layout.klayout_rdb_append".to_string(),
        "glassworks.layout.drc_report_sources.title".to_string(),
        "glassworks.viewctl.layout.drc_report_source.all".to_string(),
        format!("glassworks.viewctl.layout.drc_report_source.{appended_source_index}"),
        format!("glassworks.viewctl.layout.drc_report_source.delete.{appended_source_index}"),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC report database control should exist: {node_name}"
        );
    }
    assert!(imported.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.drc_report_source.delete.{appended_source_index}"
    )));
    assert!(
        imported
            .status_message()
            .contains("Deleted DRC report source external-review-rdb-b.rdb (2 report(s))"),
        "{}",
        imported.status_message()
    );
    assert_eq!(
        imported.layout_drc_report_history.len(),
        2,
        "deleting one report source should preserve the other loaded database"
    );
    assert!(
        layout_drc_report_source_entries(&imported)
            .iter()
            .all(|entry| !entry.source.contains("external-review-rdb-b")),
        "deleted report source should no longer appear in source groups"
    );
    assert!(
        imported
            .selected_drc_report_history_entry()
            .and_then(|entry| entry.source.as_deref())
            .is_some_and(|source| source.contains("glassworks-drc-report-database-")),
        "active report should move to a remaining report after deleting the active source"
    );
    assert!(
        imported.layout_selected_drc_marker_key.is_none(),
        "deleting the active report source should clear selected marker context"
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&append_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_import_reads_klayout_lyrdb() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let path = std::env::temp_dir().join(format!(
        "glassworks-klayout-rdb-{}.lyrdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        r#"<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description>KLayout external review</description>
 <original-file>/tmp/klayout-source.lyrdb</original-file>
 <generator>KLayout DRC deck run</generator>
 <top-cell>TOP</top-cell>
 <tags>
  <tag>
   <name>important</name>
   <description>KLayout important review marker</description>
  </tag>
  <tag>
   <name>custom-rdb-tag</name>
   <description>Custom KLayout RDB tag</description>
  </tag>
  <tag>
   <name>review&apos;s item tag</name>
   <description>Quoted KLayout item tag</description>
  </tag>
  <tag>
   <name>measured-width</name>
   <description>Measured width scalar</description>
  </tag>
  <tag>
   <name>review-note</name>
   <description>Tagged text scalar</description>
  </tag>
  <tag>
   <name>geometry-review</name>
   <description>Tagged box geometry</description>
  </tag>
  <tag>
   <name>raw-review</name>
   <description>Tagged raw value</description>
  </tag>
  <tag>
   <name>unused-rdb-tag</name>
   <description>Unused KLayout RDB tag declaration</description>
  </tag>
 </tags>
 <categories>
  <category>
   <name>Metal</name>
   <description>Metal checks from KLayout</description>
   <categories>
    <category>
     <name>Width</name>
     <description>Width rule from KLayout</description>
    </category>
   </categories>
  </category>
  <category>
   <name>Unused</name>
   <description>Unused KLayout category declaration</description>
  </category>
 </categories>
 <cells>
  <cell>
   <name>TOP</name>
   <layout-name>TOP_LAYOUT</layout-name>
   <references>
    <ref>
     <parent>ROOT</parent>
     <trans>r90 *1 17.5,-25</trans>
    </ref>
   </references>
  </cell>
  <cell>
   <name>CHILD</name>
   <layout-name>CHILD_LAYOUT</layout-name>
   <references>
    <ref>
     <parent>TOP</parent>
     <trans>r90 *1 17.5,-25</trans>
    </ref>
    <reference>
     <parent>TOP</parent>
     <trans>*2 30,40</trans>
    </reference>
    <reference>
     <parent>TOP</parent>
     <trans>r45</trans>
    </reference>
   </references>
  </cell>
  <cell>
   <name>VARIANT_CHILD</name>
   <variant>2</variant>
   <layout-name>VARIANT_CHILD_LAYOUT</layout-name>
   <references>
    <reference>
     <parent>TOP:1</parent>
     <trans>r0 4,5</trans>
    </reference>
   </references>
  </cell>
  <cell>
   <name>UNUSED</name>
   <variant>7</variant>
   <layout-name>UNUSED_LAYOUT</layout-name>
   <references>
    <reference>
     <parent>TOP</parent>
     <trans>r0 1,2</trans>
    </reference>
   </references>
  </cell>
 </cells>
 <items>
  <item>
   <tags>important, 'custom-rdb-tag', 'review\'s item tag'</tags>
   <category>Metal.Width</category>
   <cell>CHILD</cell>
   <visited>true</visited>
   <multiplicity>7</multiplicity>
   <comment>reviewed in KLayout</comment>
   <image>iVBORw0KGgo=</image>
   <values>
    <value>text: "width marker"</value>
    <value>[#'geometry-review'] box: (1.0,2.0;1.5,2.5)</value>
    <value>[#'review-reference'] reference: TOP/r0/7.0/8.0</value>
    <value>string: "scalar review lane"</value>
    <value>float: 1.25</value>
    <value>[#'measured-width'] float: 0.42</value>
    <value>[#'review-note'] text: 'tagged scalar note'</value>
    <value>text: ' leading scalar '</value>
    <value>text: ''</value>
    <value>text: '   '</value>
    <value>[#'raw-review'] unknown: preserved raw payload</value>
   </values>
  </item>
  <item>
   <category>Via.Point</category>
   <values>
    <value>point: (7.0,8.0)</value>
   </values>
  </item>
  <item>
   <category>Space.EdgePair</category>
   <values>
    <value>edge-pair: (0.0,0.0;1.0,0.0)/(0.0,0.2;1.0,0.2)</value>
   </values>
  </item>
  <item>
   <category>Routing.Path</category>
   <values>
    <value>path: (0,0;1,0;1,1) w=0.2 bx=0.4 ex=0.3 r=false</value>
   </values>
  </item>
  <item>
   <category>Texts.DText</category>
   <values>
    <value>text: ('layout text',r0 9.0,10.0)</value>
   </values>
  </item>
  <item>
   <category>Variant.Child</category>
   <cell>VARIANT_CHILD</cell>
   <values>
    <value>box: (0,0;0.2,0.3)</value>
   </values>
  </item>
  <item>
   <category>Warnings.TextOnly</category>
   <values>
    <value>text: "text-only RDB diagnostic"</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB fixture should write");

    assert!(app.import_layout_drc_report_database_from_path(&path));
    assert!(
        app.status_message()
            .contains("Imported KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.layout_drc_report_history.len(), 1);
    assert_eq!(
        app.app_options.files.recent_files[0].kind,
        "drc_report_database"
    );
    let report = app
        .drc_report()
        .expect("KLayout RDB report should be active");
    assert_eq!(report.violations.len(), 8);
    assert_eq!(report.findings.len(), 2);
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.message.contains("reference: TOP/r0/7.0/8.0")),
        "typed KLayout reference values should not surface as raw import diagnostics: {:?}",
        report.findings
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("text-only RDB diagnostic")),
        "text-only KLayout RDB diagnostics should still import: {:?}",
        report.findings
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("unknown: preserved raw payload")),
        "tagged raw KLayout RDB values should surface as import warnings: {:?}",
        report.findings
    );
    assert_eq!(report.violations[0].rule, "klayout.metal_width");
    assert_eq!(
        report.violations[0].bounds,
        Rect::new(Point::new(15000, -24000), Point::new(15500, -23500))
    );
    assert_eq!(
        report.violations[1].bounds,
        Rect::new(Point::new(32000, 44000), Point::new(33000, 45000))
    );
    assert_eq!(
        report.violations[2].bounds,
        Rect::new(Point::new(-1061, 2121), Point::new(-354, 2828))
    );
    assert_eq!(report.violations[3].rule, "klayout.via_point");
    assert_eq!(
        report.violations[3].bounds,
        Rect::new(Point::new(6999, 7999), Point::new(7001, 8001))
    );
    assert_eq!(report.violations[4].rule, "klayout.space_edgepair");
    assert_eq!(
        report.violations[4].bounds,
        Rect::new(Point::new(-1, -1), Point::new(1001, 201))
    );
    assert_eq!(report.violations[5].rule, "klayout.routing_path");
    assert_eq!(
        report.violations[5].bounds,
        Rect::new(Point::new(-500, -100), Point::new(1100, 1400))
    );
    assert_eq!(report.violations[6].rule, "klayout.texts_dtext");
    assert_eq!(
        report.violations[6].bounds,
        Rect::new(Point::new(8999, 9999), Point::new(9001, 10001))
    );
    assert_eq!(report.violations[7].rule, "klayout.variant_child");
    assert_eq!(
        report.violations[7].bounds,
        Rect::new(Point::new(4000, 5000), Point::new(4200, 5300))
    );
    let text_marker_key = report.violations[6].stable_key();
    let text_state = app
        .workspace
        .document
        .marker_states
        .get(&text_marker_key)
        .expect("KLayout RDB text geometry should seed marker state");
    assert_eq!(
        text_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("text: ('layout text',r0 9.0,10.0)")
    );
    let variant_marker_key = report.violations[7].stable_key();
    let variant_state = app
        .workspace
        .document
        .marker_states
        .get(&variant_marker_key)
        .expect("KLayout unique cell variant should seed marker state");
    assert_eq!(
        variant_state.tags.get("source_cell").map(String::as_str),
        Some("VARIANT_CHILD")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_layout_name")
            .map(String::as_str),
        Some("VARIANT_CHILD_LAYOUT")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("TOP:1")
    );
    assert_eq!(
        variant_state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r0 4,5")
    );
    let marker_key = report.violations[0].stable_key();
    let state = app
        .workspace
        .document
        .marker_states
        .get(&marker_key)
        .expect("KLayout RDB item tags should seed marker state");
    assert!(state.visited);
    assert!(state.important);
    assert_eq!(state.note.as_deref(), Some("reviewed in KLayout"));
    assert_eq!(
        state.tags.get("rdb_multiplicity").map(String::as_str),
        Some("7")
    );
    assert_eq!(
        state.tags.get("rdb_category").map(String::as_str),
        Some("Metal/Width")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_1")
            .map(String::as_str),
        Some("Metal checks from KLayout")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_category_description_2")
            .map(String::as_str),
        Some("Width rule from KLayout")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_category_3_part_1")
            .map(String::as_str),
        Some("Unused")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_category_3_description")
            .map(String::as_str),
        Some("Unused KLayout category declaration")
    );
    assert_eq!(
        state.tags.get("source_cell").map(String::as_str),
        Some("CHILD")
    );
    assert_eq!(
        state.tags.get("custom_rdb_tag").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state.tags.get("review_s_item_tag").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_review_s_item_tag")
            .map(String::as_str),
        Some("review's item tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_custom_rdb_tag")
            .map(String::as_str),
        Some("custom-rdb-tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_important")
            .map(String::as_str),
        Some("KLayout important review marker")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_custom_rdb_tag")
            .map(String::as_str),
        Some("Custom KLayout RDB tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_review_s_item_tag")
            .map(String::as_str),
        Some("Quoted KLayout item tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_declared_tag_unused_rdb_tag")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_name_unused_rdb_tag")
            .map(String::as_str),
        Some("unused-rdb-tag")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_tag_description_unused_rdb_tag")
            .map(String::as_str),
        Some("Unused KLayout RDB tag declaration")
    );
    assert_eq!(
        state.tags.get("rdb_image_base64").map(String::as_str),
        Some("iVBORw0KGgo=")
    );
    assert_eq!(
        state.tags.get("rdb_report_description").map(String::as_str),
        Some("KLayout external review")
    );
    assert_eq!(
        state.tags.get("rdb_report_top_cell").map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_original_file")
            .map(String::as_str),
        Some("/tmp/klayout-source.lyrdb")
    );
    assert_eq!(
        state.tags.get("rdb_report_generator").map(String::as_str),
        Some("KLayout DRC deck run")
    );
    assert_eq!(
        state.tags.get("rdb_geometry_value_1").map(String::as_str),
        Some("box: (1.0,2.0;1.5,2.5)")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_geometry_value_1_tag")
            .map(String::as_str),
        Some("geometry-review")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_1")
            .map(String::as_str),
        Some("text: \"width marker\"")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_2")
            .map(String::as_str),
        Some("[#'geometry-review'] box: (1.0,2.0;1.5,2.5)")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_3")
            .map(String::as_str),
        Some("[#'review-reference'] reference: TOP/r0/7.0/8.0")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_4")
            .map(String::as_str),
        Some("string: \"scalar review lane\"")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_5")
            .map(String::as_str),
        Some("float: 1.25")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_6")
            .map(String::as_str),
        Some("[#'measured-width'] float: 0.42")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_7")
            .map(String::as_str),
        Some("[#'review-note'] text: 'tagged scalar note'")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_8")
            .map(String::as_str),
        Some("text: ' leading scalar '")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_9")
            .map(String::as_str),
        Some("text: ''")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_10")
            .map(String::as_str),
        Some("text: '   '")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_ordered_value_11")
            .map(String::as_str),
        Some("[#'raw-review'] unknown: preserved raw payload")
    );
    assert_eq!(
        state.tags.get("rdb_item_reference_1").map(String::as_str),
        Some("TOP/r0/7.0/8.0")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_reference_1_tag")
            .map(String::as_str),
        Some("review-reference")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_1_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_1_value").map(String::as_str),
        Some("width marker")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_2_type").map(String::as_str),
        Some("string")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_2_value").map(String::as_str),
        Some("scalar review lane")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_3_type").map(String::as_str),
        Some("float")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_3_value").map(String::as_str),
        Some("1.25")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_type").map(String::as_str),
        Some("float")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_value").map(String::as_str),
        Some("0.42")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_4_tag").map(String::as_str),
        Some("measured-width")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_value").map(String::as_str),
        Some("tagged scalar note")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_5_tag").map(String::as_str),
        Some("review-note")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_6_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_6_value").map(String::as_str),
        Some(" leading scalar ")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_7_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_7_empty").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state.tags.get("rdb_item_value_8_type").map(String::as_str),
        Some("text")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_item_value_8_value_hex")
            .map(String::as_str),
        Some("202020")
    );
    assert_eq!(
        state.tags.get("rdb_raw_value_1").map(String::as_str),
        Some("unknown: preserved raw payload")
    );
    assert_eq!(
        state.tags.get("rdb_raw_value_1_tag").map(String::as_str),
        Some("raw-review")
    );
    assert_eq!(
        state.tags.get("rdb_cell_layout_name").map(String::as_str),
        Some("CHILD_LAYOUT")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r90 *1 17.5,-25")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_2_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_2_trans")
            .map(String::as_str),
        Some("*2 30,40")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_3_parent")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_cell_reference_3_trans")
            .map(String::as_str),
        Some("r45")
    );
    let unused_declared_cell_index = (1..=16)
        .find(|index| {
            state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "UNUSED")
        })
        .expect("unused declared KLayout cell should be retained");
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_variant"
            ))
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_layout_name"
            ))
            .map(String::as_str),
        Some("UNUSED_LAYOUT")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_parent"
            ))
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_trans"
            ))
            .map(String::as_str),
        Some("r0 1,2")
    );
    assert!(
        layout_marker_state_has_snapshot(state),
        "embedded KLayout RDB image payloads should count as marker snapshots"
    );
    assert!(
        layout_marker_snapshot_label(Some(state)).contains("embedded KLayout RDB image"),
        "embedded KLayout RDB image payloads should be visible in marker snapshot labels"
    );
    assert_eq!(layout_drc_marker_entries(&app).len(), 8);
    let snapshot_entries = layout_drc_marker_snapshot_entries(&app);
    assert_eq!(
        snapshot_entries.len(),
        3,
        "embedded KLayout RDB image payloads should appear in the snapshot gallery"
    );
    let snapshot_entry = snapshot_entries
        .first()
        .expect("KLayout RDB embedded image should produce a snapshot gallery entry");
    assert_eq!(snapshot_entry.key.as_str(), marker_key.as_str());
    assert_eq!(
        snapshot_entry.image_key.as_str(),
        format!("layout.drc_marker.rdb_image.{marker_key}").as_str()
    );
    assert!(
        snapshot_entry.label.contains("embedded"),
        "snapshot gallery label should expose embedded RDB image evidence: {snapshot_entry:?}"
    );
    let snapshot_gallery = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with KLayout RDB snapshot gallery should build");
    let snapshot_thumbnail = snapshot_gallery
        .nodes()
        .iter()
        .find(|node| {
            node.name()
                == format!(
                    "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}.image",
                    snapshot_entry.id
                )
        })
        .unwrap_or_else(|| {
            panic!("KLayout RDB embedded image should expose a snapshot thumbnail node")
        });
    let UiContent::Canvas(snapshot_thumbnail_canvas) = snapshot_thumbnail.content() else {
        panic!("KLayout RDB snapshot thumbnail should be a canvas node");
    };
    assert_eq!(
        snapshot_thumbnail_canvas.key.as_str(),
        LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY
    );
    assert_eq!(
        snapshot_thumbnail_canvas.surface_key(),
        snapshot_entry.image_key.as_str()
    );
    app.layout_drc_marker_filter = LayoutDrcMarkerFilter::Snapshots;
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == marker_key),
        "snapshot filter should include imported KLayout RDB image payloads"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Snapshot" && value.contains("embedded KLayout RDB image")),
        "marker detail rows should expose embedded KLayout RDB image metadata: {marker_rows:?}"
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Listed snapshots"
                && value.contains("3 snapshots")
                && value.contains("embedded-rdb=3")
        }),
        "marker info pane should summarize embedded KLayout RDB image metadata: {info_rows:?}"
    );
    app.layout_drc_marker_filter = LayoutDrcMarkerFilter::Active;

    assert!(app.append_layout_drc_report_database_from_path(&path));
    assert!(
        app.status_message()
            .contains("Appended KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.layout_drc_report_history.len(), 2);
    let source_entries = layout_drc_report_source_entries(&app);
    assert_eq!(source_entries.len(), 1);
    assert_eq!(source_entries[0].total_reports, 2);

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_import_reads_compressed_klayout_lyrdb() {
    let contents = r#"<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description>Compressed KLayout review</description>
 <top-cell>TOP</top-cell>
 <items>
  <item>
   <category>Compressed.RDB</category>
   <cell>TOP</cell>
   <values>
    <value>box: (1.0,2.0;1.5,2.5)</value>
    <value>text: "compressed marker"</value>
   </values>
  </item>
 </items>
</report-database>
"#;
    let base_path = std::env::temp_dir().join(format!(
        "glassworks-compressed-klayout-rdb-{}.lyrdb",
        std::process::id()
    ));
    let gz_path = base_path.with_extension("lyrdb.gz");
    let zip_path = base_path.with_extension("lyrdb.zip");
    let _ = std::fs::remove_file(&gz_path);
    let _ = std::fs::remove_file(&zip_path);
    write_gzip_test_file(&gz_path, contents.as_bytes());
    write_single_file_zip_test_file(&zip_path, "markers.lyrdb", contents.as_bytes());

    for path in [&gz_path, &zip_path] {
        let mut app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        assert!(app.import_layout_drc_report_database_from_path(path));
        assert!(
            app.status_message()
                .contains("Imported KLayout RDB report database"),
            "{}",
            app.status_message()
        );
        assert_eq!(
            app.app_options.files.recent_files[0].path,
            path.display().to_string()
        );
        let report = app
            .drc_report()
            .expect("compressed KLayout RDB should populate the active report");
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "klayout.compressed_rdb");
        assert_eq!(report.violations[0].message, "compressed marker");
        assert_eq!(
            report.violations[0].bounds,
            Rect::new(Point::new(1000, 2000), Point::new(1500, 2500))
        );
    }

    let _ = std::fs::remove_file(&gz_path);
    let _ = std::fs::remove_file(&zip_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_round_trips_empty_klayout_root_metadata() {
    let input_path = std::env::temp_dir().join(format!(
        "glassworks-empty-klayout-rdb-root-{}.lyrdb",
        std::process::id()
    ));
    let output_path = input_path.with_file_name(format!(
        "glassworks-empty-klayout-rdb-root-export-{}.lyrdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    std::fs::write(
        &input_path,
        r#"<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description/>
 <original-file/>
 <generator/>
 <top-cell/>
 <tags>
 </tags>
 <categories>
  <category>
   <name>EmptyMetadata</name>
   <description/>
   <categories>
   </categories>
  </category>
 </categories>
 <cells>
  <cell>
   <name>TOP</name>
   <variant/>
   <layout-name/>
   <references>
   </references>
  </cell>
 </cells>
 <items>
  <item>
   <tags/>
   <category>EmptyMetadata</category>
   <cell>TOP</cell>
   <visited>false</visited>
   <multiplicity>1</multiplicity>
   <comment/>
   <image/>
   <values>
    <value>box: (0,0;1,1)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("empty root KLayout RDB fixture should write");

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(app.import_layout_drc_report_database_from_path(&input_path));
    let report = app
        .drc_report()
        .expect("empty root metadata KLayout RDB should import");
    assert_eq!(report.violations.len(), 1);
    let marker_key = report.violations[0].stable_key();
    let state = app
        .workspace
        .document
        .marker_states
        .get(&marker_key)
        .expect("empty root metadata should be stored on marker state");
    assert_eq!(
        state
            .tags
            .get("rdb_report_description_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_original_file_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_generator_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        state
            .tags
            .get("rdb_report_top_cell_empty")
            .map(String::as_str),
        Some("true")
    );

    assert!(app.export_layout_drc_report_database_to_path(&output_path));
    let contents =
        std::fs::read_to_string(&output_path).expect("empty root metadata export should write XML");
    assert!(contents.contains(" <description/>\n"));
    assert!(contents.contains(" <original-file/>\n"));
    assert!(contents.contains(" <generator/>\n"));
    assert!(contents.contains(" <top-cell/>\n"));
    assert!(!contents.contains("Glassworks DRC report database"));
    assert!(!contents.contains("Glassworks KLayout RDB subset exporter"));
    assert!(!contents.contains("rdb_report_description_empty"));
    assert!(!contents.contains("rdb_report_generator_empty"));

    let imported =
        import_klayout_rdb_markers(&contents).expect("empty root metadata export should parse");
    assert_eq!(imported.report.description, None);
    assert_eq!(imported.report.original_file, None);
    assert_eq!(imported.report.generator, None);
    assert_eq!(imported.report.top_cell, None);

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_import_uses_layout_hierarchy_for_klayout_rdb_refs() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let top_cell = app.workspace.document.top_cell;
    app.workspace.document.cell_mut(top_cell).unwrap().name = "TOP".to_string();
    let child = app.workspace.document.create_cell("CHILD");
    app.workspace
        .document
        .insert_instance_in_top(
            child,
            Transform {
                matrix: [0, -1, 1, 0],
                translation: geometry_core::Vector::new(17_500, -25_000),
            },
        )
        .expect("fallback hierarchy fixture should insert child instance");
    let path = std::env::temp_dir().join(format!(
        "glassworks-klayout-rdb-layout-fallback-{}.lyrdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        r#"<?xml version="1.0" encoding="utf-8"?>
<report-database>
 <description>KLayout hierarchy fallback</description>
 <top-cell>TOP</top-cell>
 <cells>
  <cell>
   <name>CHILD</name>
   <layout-name>CHILD_LAYOUT</layout-name>
   <references/>
  </cell>
 </cells>
 <items>
  <item>
   <category>Metal.Width</category>
   <cell>CHILD</cell>
   <values>
    <value>box: (1.0,2.0;1.5,2.5)</value>
   </values>
  </item>
 </items>
</report-database>
"#,
    )
    .expect("KLayout RDB hierarchy fallback fixture should write");

    assert!(app.import_layout_drc_report_database_from_path(&path));
    assert!(
        app.status_message()
            .contains("Imported KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    let report = app
        .drc_report()
        .expect("KLayout RDB hierarchy fallback report should be active");
    assert_eq!(report.violations.len(), 1);
    assert_eq!(
        report.violations[0].bounds,
        Rect::new(Point::new(15000, -24000), Point::new(15500, -23500))
    );
    let state = app
        .workspace
        .document
        .marker_states
        .get(&report.violations[0].stable_key())
        .expect("KLayout RDB hierarchy fallback state should import");
    assert_eq!(
        state.tags.get("source_cell").map(String::as_str),
        Some("CHILD")
    );
    assert_eq!(
        state.tags.get("rdb_cell_layout_name").map(String::as_str),
        Some("CHILD_LAYOUT")
    );
    assert!(
        !state.tags.contains_key("rdb_cell_reference_1_parent"),
        "fallback should not invent RDB cell-reference metadata"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
pub(crate) fn layout_drc_marker_snapshot_canvas_decodes_embedded_lyrdb_png() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("KLayout embedded image update source");
    app.reset_layout_document_state();
    let violation = DrcViolation {
        id: 7,
        rule: "external.image".to_string(),
        message: "embedded image marker".to_string(),
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds: Rect::new(Point::new(0, 0), Point::new(1000, 1000)),
        required: 0,
        actual: 0.0,
    };
    let marker_key = violation.stable_key();
    app.push_layout_drc_report(
        "Embedded Image",
        DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![violation],
        },
    );
    let rgba = [255, 0, 0, 255, 0, 128, 255, 192];
    let image_base64 = test_png_rgba_base64(2, 1, &rgba);
    app.workspace.document.marker_states.insert(
        marker_key.clone(),
        MarkerState {
            tags: BTreeMap::from([
                ("rdb_image".to_string(), "embedded".to_string()),
                ("rdb_image_base64".to_string(), image_base64),
            ]),
            ..MarkerState::default()
        },
    );

    let entries = layout_drc_marker_snapshot_entries(&app);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].image_key,
        format!("layout.drc_marker.rdb_image.{marker_key}")
    );
    let canvas_rgba =
        layout_drc_marker_snapshot_canvas_rgba(&app, &entries[0].image_key, PixelSize::new(2, 1))
            .expect("embedded KLayout PNG payload should decode for canvas thumbnails");
    assert_eq!(canvas_rgba, rgba.to_vec());
    #[cfg(not(target_arch = "wasm32"))]
    {
        let report = app
            .render_operad_snapshot_scaled(320, 240, UiScale::new(1.0))
            .expect(
                "snapshot rendering should render embedded image canvases without resource updates",
            );
        assert!(
            report.render.snapshot.is_some(),
            "snapshot render should produce an image"
        );
    }
    let gallery = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("embedded image snapshot gallery should build");
    let snapshot_thumbnail = gallery
        .nodes()
        .iter()
        .find(|node| {
            node.name()
                == format!(
                    "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}.image",
                    entries[0].id
                )
        })
        .unwrap_or_else(|| panic!("embedded image gallery should expose a thumbnail canvas"));
    let UiContent::Canvas(snapshot_thumbnail_canvas) = snapshot_thumbnail.content() else {
        panic!("embedded image thumbnail should be a canvas node");
    };
    assert_eq!(
        snapshot_thumbnail_canvas.key.as_str(),
        LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY
    );
    assert_eq!(
        snapshot_thumbnail_canvas.surface_key(),
        entries[0].image_key.as_str()
    );
}

fn test_png_rgba_base64(width: u32, height: u32, rgba: &[u8]) -> String {
    use base64::Engine as _;

    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("test PNG header should encode");
        writer
            .write_image_data(rgba)
            .expect("test PNG data should encode");
    }
    base64::engine::general_purpose::STANDARD.encode(png_bytes)
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_export_writes_klayout_lyrdb() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("KLayout RDB export source");
    app.reset_layout_document_state();
    let violation = DrcViolation {
        id: 1,
        rule: "external.width".to_string(),
        message: "width marker from Glassworks".to_string(),
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds: Rect::new(Point::new(1000, 2000), Point::new(1500, 2500)),
        required: 0,
        actual: 0.0,
    };
    let marker_key = violation.stable_key();
    let text_violation = DrcViolation {
        id: 2,
        rule: "external.text".to_string(),
        message: "text marker from KLayout".to_string(),
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds: Rect::new(Point::new(4000, 5000), Point::new(4500, 5500)),
        required: 0,
        actual: 0.0,
    };
    let text_marker_key = text_violation.stable_key();
    let scalar_violation = DrcViolation {
        id: 3,
        rule: "external.scalar".to_string(),
        message: "tagged scalar marker from Glassworks".to_string(),
        shape_ids: Vec::new(),
        occurrence_ids: Vec::new(),
        bounds: Rect::new(Point::new(6000, 7000), Point::new(6500, 7500)),
        required: 0,
        actual: 0.0,
    };
    let scalar_marker_key = scalar_violation.stable_key();
    app.push_layout_drc_report(
        "KLayout Export",
        DrcReportCacheValue {
            findings: vec![DrcValidationFinding {
                severity: DrcValidationSeverity::Warning,
                message: "text-only export diagnostic".to_string(),
            }],
            violations: vec![violation, text_violation, scalar_violation],
        },
    );
    let image_path = std::env::temp_dir().join(format!(
        "glassworks-klayout-rdb-export-image-{}.png",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&image_path);
    std::fs::write(&image_path, [137, 80, 78, 71, 13, 10, 26, 10])
        .expect("KLayout RDB export fixture image should write");
    app.workspace.document.marker_states.insert(
        marker_key,
        MarkerState {
            hidden: true,
            waived: true,
            visited: true,
            important: true,
            note: Some("reviewed before export".to_string()),
            tags: BTreeMap::from([
                (
                    "rdb_category".to_string(),
                    "Layer's.Group/Rule.Name".to_string(),
                ),
                (
                    "rdb_category_part_1".to_string(),
                    "Layer's.Group".to_string(),
                ),
                ("rdb_category_part_2".to_string(), "Rule.Name".to_string()),
                (
                    "rdb_category_description_1".to_string(),
                    "Layer apostrophe category from KLayout".to_string(),
                ),
                (
                    "rdb_category_description_2".to_string(),
                    "Quoted dot category from KLayout".to_string(),
                ),
                (
                    "rdb_declared_category_1_part_1".to_string(),
                    "Unused".to_string(),
                ),
                (
                    "rdb_declared_category_1_part_2".to_string(),
                    "Review".to_string(),
                ),
                (
                    "rdb_declared_category_1_description".to_string(),
                    "Unused declared KLayout category".to_string(),
                ),
                (
                    "rdb_declared_cell_1_name".to_string(),
                    "UNUSED_CELL".to_string(),
                ),
                ("rdb_declared_cell_1_variant".to_string(), "7".to_string()),
                (
                    "rdb_declared_cell_1_layout_name".to_string(),
                    "UNUSED_LAYOUT".to_string(),
                ),
                (
                    "rdb_declared_cell_1_reference_1_parent".to_string(),
                    "TOP".to_string(),
                ),
                (
                    "rdb_declared_cell_1_reference_1_trans".to_string(),
                    "r0 1,2".to_string(),
                ),
                ("rdb_declared_cell_2_name".to_string(), "TOP".to_string()),
                (
                    "rdb_declared_cell_2_variant".to_string(),
                    "ANNOTATION".to_string(),
                ),
                (
                    "rdb_declared_cell_2_layout_name".to_string(),
                    "TOP_VARIANT_LAYOUT".to_string(),
                ),
                (
                    "rdb_tag_description_important".to_string(),
                    "KLayout important review marker".to_string(),
                ),
                (
                    "rdb_tag_description_review_lane_final".to_string(),
                    "Review lane tag from KLayout".to_string(),
                ),
                (
                    "rdb_tag_description_measured_width".to_string(),
                    "Measured width scalar".to_string(),
                ),
                (
                    "rdb_tag_description_review_note".to_string(),
                    "Tagged text scalar".to_string(),
                ),
                (
                    "rdb_tag_description_review_reference".to_string(),
                    "Tagged item reference".to_string(),
                ),
                (
                    "rdb_tag_description_review_s_item_tag".to_string(),
                    "Quoted KLayout item tag".to_string(),
                ),
                ("rdb_multiplicity".to_string(), "7".to_string()),
                ("source_cell".to_string(), "TOP:ANNOTATION".to_string()),
                ("review_lane".to_string(), "final".to_string()),
                ("custom_rdb_tag".to_string(), "true".to_string()),
                (
                    "rdb_tag_name_custom_rdb_tag".to_string(),
                    "custom-rdb-tag".to_string(),
                ),
                ("review_s_item_tag".to_string(), "true".to_string()),
                (
                    "rdb_tag_name_review_s_item_tag".to_string(),
                    "review's item tag".to_string(),
                ),
                (
                    "rdb_tag_description_custom_rdb_tag".to_string(),
                    "Custom KLayout RDB tag".to_string(),
                ),
                (
                    "rdb_declared_tag_unused_declared_tag".to_string(),
                    "true".to_string(),
                ),
                (
                    "rdb_tag_name_unused_declared_tag".to_string(),
                    "unused-declared-tag".to_string(),
                ),
                (
                    "rdb_tag_description_unused_declared_tag".to_string(),
                    "Unused declared KLayout tag".to_string(),
                ),
                (
                    "rdb_report_description".to_string(),
                    "KLayout external review export".to_string(),
                ),
                ("rdb_report_top_cell".to_string(), "TOP".to_string()),
                (
                    "rdb_report_original_file".to_string(),
                    "/tmp/original-klayout-run.lyrdb".to_string(),
                ),
                (
                    "rdb_report_generator".to_string(),
                    "KLayout DRC deck run".to_string(),
                ),
                ("rdb_cell_layout_name".to_string(), "TOP_LAYOUT".to_string()),
                (
                    "rdb_cell_reference_1_parent".to_string(),
                    "ROOT".to_string(),
                ),
                (
                    "rdb_cell_reference_1_trans".to_string(),
                    "r90 *1 17.5,-25".to_string(),
                ),
                (
                    "rdb_item_reference_1".to_string(),
                    "TOP/r0/9.0/10.0".to_string(),
                ),
                (
                    "rdb_item_reference_1_tag".to_string(),
                    "review-reference".to_string(),
                ),
                (
                    "rdb_item_ordered_value_1".to_string(),
                    "text: 'width marker from Glassworks'".to_string(),
                ),
                (
                    "rdb_item_ordered_value_2".to_string(),
                    "['review-reference'] reference: TOP/r0/9.0/10.0".to_string(),
                ),
                (
                    "rdb_item_ordered_value_3".to_string(),
                    "string: 'KLayout scalar note'".to_string(),
                ),
                (
                    "rdb_item_ordered_value_4".to_string(),
                    "float: 1.25".to_string(),
                ),
                (
                    "rdb_item_ordered_value_5".to_string(),
                    "unknown: preserved payload".to_string(),
                ),
                (
                    "rdb_item_ordered_value_6".to_string(),
                    "polygon: (1,2;1.5,2;1.5,2.5;1,2.5/1.1,2.1;1.2,2.1;1.2,2.2)".to_string(),
                ),
                (
                    "rdb_item_ordered_value_7".to_string(),
                    "['review\\'s scalar'] string: 'Tagged legacy scalar'".to_string(),
                ),
                (
                    "rdb_item_ordered_value_8".to_string(),
                    "rect: (1.05,2.05;1.1,2.1)".to_string(),
                ),
                (
                    "rdb_item_ordered_value_9".to_string(),
                    "edge_pair: (1.05,2.05;1.1,2.05)/(1.05,2.1;1.1,2.1)".to_string(),
                ),
                (
                    "rdb_item_ordered_value_10".to_string(),
                    "point: (1.25,2.25)".to_string(),
                ),
                ("rdb_item_value_1_type".to_string(), "string".to_string()),
                (
                    "rdb_item_value_1_value".to_string(),
                    "KLayout scalar note".to_string(),
                ),
                ("rdb_item_value_2_type".to_string(), "float".to_string()),
                ("rdb_item_value_2_value".to_string(), "1.25".to_string()),
                (
                    "rdb_geometry_value_1".to_string(),
                    "polygon: (1,2;1.5,2;1.5,2.5;1,2.5/1.1,2.1;1.2,2.1;1.2,2.2)".to_string(),
                ),
                (
                    "rdb_raw_value_2".to_string(),
                    "unknown: preserved payload".to_string(),
                ),
                ("screenshot".to_string(), image_path.display().to_string()),
                ("screenshot_format".to_string(), "PNG".to_string()),
            ]),
            ..MarkerState::default()
        },
    );
    app.workspace.document.marker_states.insert(
        text_marker_key,
        MarkerState {
            tags: BTreeMap::from([
                ("rdb_category".to_string(), "Texts/DText".to_string()),
                ("rdb_category_part_1".to_string(), "Texts".to_string()),
                ("rdb_category_part_2".to_string(), "DText".to_string()),
                (
                    "rdb_item_ordered_value_1".to_string(),
                    "text: ('KLayout text marker',r90 2.0,3.0)".to_string(),
                ),
                (
                    "rdb_item_ordered_value_2".to_string(),
                    "['geometry\\'s label'] text: ('KLayout \\'quoted text marker',r90 2.0,3.0)"
                        .to_string(),
                ),
                (
                    "rdb_geometry_value_1".to_string(),
                    "text: ('KLayout text marker',r90 2.0,3.0)".to_string(),
                ),
                ("source_cell".to_string(), "TOP".to_string()),
            ]),
            ..MarkerState::default()
        },
    );
    app.workspace.document.marker_states.insert(
        scalar_marker_key,
        MarkerState {
            tags: BTreeMap::from([
                (
                    "rdb_category".to_string(),
                    "Metrics/Tagged-Values".to_string(),
                ),
                ("rdb_category_part_1".to_string(), "Metrics".to_string()),
                (
                    "rdb_category_part_2".to_string(),
                    "Tagged-Values".to_string(),
                ),
                ("rdb_multiplicity".to_string(), "1.5".to_string()),
                ("rdb_item_value_1_type".to_string(), "float".to_string()),
                ("rdb_item_value_1_value".to_string(), "0.42".to_string()),
                (
                    "rdb_item_value_1_tag".to_string(),
                    "measured-width".to_string(),
                ),
                (
                    "rdb_item_reference_1".to_string(),
                    "TOP/r0/11.0/12.0".to_string(),
                ),
                (
                    "rdb_item_reference_1_tag".to_string(),
                    "review-reference".to_string(),
                ),
                ("rdb_item_value_2_type".to_string(), "text".to_string()),
                (
                    "rdb_item_value_2_value".to_string(),
                    "tagged scalar note".to_string(),
                ),
                (
                    "rdb_item_value_2_tag".to_string(),
                    "review-note".to_string(),
                ),
                ("rdb_item_value_3_type".to_string(), "integer".to_string()),
                ("rdb_item_value_3_value".to_string(), "7".to_string()),
                (
                    "rdb_item_value_3_tag".to_string(),
                    "count-review".to_string(),
                ),
                ("rdb_item_value_4_type".to_string(), "float".to_string()),
                ("rdb_item_value_4_value".to_string(), "abc".to_string()),
                (
                    "rdb_item_value_4_tag".to_string(),
                    "invalid-float".to_string(),
                ),
                ("rdb_item_value_5_type".to_string(), "text".to_string()),
                (
                    "rdb_item_value_5_value".to_string(),
                    "alpha_123".to_string(),
                ),
                ("rdb_item_value_6_type".to_string(), "text".to_string()),
                (
                    "rdb_item_value_6_value".to_string(),
                    " leading scalar ".to_string(),
                ),
                ("rdb_item_value_7_type".to_string(), "text".to_string()),
                ("rdb_item_value_7_empty".to_string(), "true".to_string()),
                ("rdb_item_value_8_type".to_string(), "text".to_string()),
                (
                    "rdb_item_value_8_value_hex".to_string(),
                    "202020".to_string(),
                ),
                (
                    "rdb_geometry_value_1".to_string(),
                    "box: (6,7;6.5,7.5)".to_string(),
                ),
                (
                    "rdb_geometry_value_1_tag".to_string(),
                    "geometry-review".to_string(),
                ),
                (
                    "rdb_geometry_value_2".to_string(),
                    "box: (6.1,7.1;6.2,7.2)".to_string(),
                ),
                (
                    "rdb_geometry_value_2_tag".to_string(),
                    "geometry_review_word".to_string(),
                ),
                (
                    "rdb_raw_value_1".to_string(),
                    "unknown-tagged: preserved tagged raw".to_string(),
                ),
                ("rdb_raw_value_1_tag".to_string(), "raw-review".to_string()),
                (
                    "rdb_tag_description_measured_width".to_string(),
                    "Measured width scalar".to_string(),
                ),
                (
                    "rdb_tag_description_review_note".to_string(),
                    "Tagged text scalar".to_string(),
                ),
                (
                    "rdb_tag_description_geometry_review".to_string(),
                    "Tagged box geometry".to_string(),
                ),
                (
                    "rdb_tag_description_geometry_review_word".to_string(),
                    "Native word value tag".to_string(),
                ),
                (
                    "rdb_tag_description_raw_review".to_string(),
                    "Tagged raw value".to_string(),
                ),
                (
                    "rdb_tag_description_count_review".to_string(),
                    "Unsupported scalar count".to_string(),
                ),
                (
                    "rdb_tag_description_invalid_float".to_string(),
                    "Invalid scalar float".to_string(),
                ),
                ("source_cell".to_string(), "TOP".to_string()),
            ]),
            ..MarkerState::default()
        },
    );

    let path = std::env::temp_dir().join(format!(
        "glassworks-klayout-rdb-export-{}.lyrdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    assert!(app.export_layout_drc_report_database_to_path(&path));
    assert!(
        app.status_message()
            .contains("Exported KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("KLayout RDB export should write XML");
    assert!(contents.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(contents.contains("<report-database>"));
    assert!(contents.contains("<description>KLayout external review export</description>"));
    assert!(contents.contains("<original-file>/tmp/original-klayout-run.lyrdb</original-file>"));
    assert!(contents.contains("<generator>KLayout DRC deck run</generator>"));
    assert!(contents.contains("<top-cell>TOP</top-cell>"));
    assert!(contents.contains("<name>important</name>"));
    assert!(contents.contains("<description>KLayout important review marker</description>"));
    assert!(contents.contains("<name>custom-rdb-tag</name>"));
    assert!(contents.contains("<description>Custom KLayout RDB tag</description>"));
    assert!(contents.contains("<name>hidden</name>"));
    assert!(contents.contains("<description>Glassworks marker tag hidden</description>"));
    assert!(contents.contains("<name>measured-width</name>"));
    assert!(contents.contains("<description>Measured width scalar</description>"));
    assert!(contents.contains("<name>review-note</name>"));
    assert!(contents.contains("<description>Tagged text scalar</description>"));
    assert!(contents.contains("<name>review-reference</name>"));
    assert!(contents.contains("<description>Tagged item reference</description>"));
    assert!(contents.contains("<name>review&apos;s item tag</name>"));
    assert!(contents.contains("<description>Quoted KLayout item tag</description>"));
    assert!(contents.contains("<name>geometry&apos;s label</name>"));
    assert!(contents.contains("<name>review&apos;s scalar</name>"));
    assert!(contents.contains("<name>geometry-review</name>"));
    assert!(contents.contains("<description>Tagged box geometry</description>"));
    assert!(contents.contains("<name>geometry_review_word</name>"));
    assert!(contents.contains("<description>Native word value tag</description>"));
    assert!(contents.contains("<name>raw-review</name>"));
    assert!(contents.contains("<description>Tagged raw value</description>"));
    assert!(contents.contains("<name>count-review</name>"));
    assert!(contents.contains("<description>Unsupported scalar count</description>"));
    assert!(contents.contains("<name>invalid-float</name>"));
    assert!(contents.contains("<description>Invalid scalar float</description>"));
    assert!(contents.contains("<name>review_lane_final</name>"));
    assert!(contents.contains("<description>Review lane tag from KLayout</description>"));
    assert!(contents.contains("<name>unused-declared-tag</name>"));
    assert!(contents.contains("<description>Unused declared KLayout tag</description>"));
    assert!(
        contents.contains(
            "<category>&apos;Layer\\&apos;s.Group&apos;.&apos;Rule.Name&apos;</category>"
        )
    );
    assert!(contents.contains("<description>Layer apostrophe category from KLayout</description>"));
    assert!(contents.contains("<description>Quoted dot category from KLayout</description>"));
    assert!(contents.contains("<name>Unused</name>"));
    assert!(contents.contains("<name>Review</name>"));
    assert!(contents.contains("<description>Unused declared KLayout category</description>"));
    assert!(contents.contains("<cell>TOP</cell>"));
    assert!(
        contents.contains(
            "  <cell>\n   <name></name>\n   <variant/>\n   <layout-name/>\n   <references>\n   </references>\n  </cell>"
        ),
        "cell-less diagnostic items should declare KLayout's empty-name All cells cell"
    );
    assert!(contents.contains("<layout-name/>"));
    assert!(!contents.contains("<references/>"));
    assert!(contents.contains("<name>UNUSED_CELL</name>"));
    assert!(contents.contains("<variant>7</variant>"));
    assert!(contents.contains(
        "<name>TOP:ANNOTATION</name>\n   <variant/>\n   <layout-name>TOP_LAYOUT</layout-name>"
    ));
    assert!(contents.contains(
        "<name>TOP</name>\n   <variant>ANNOTATION</variant>\n   <layout-name>TOP_VARIANT_LAYOUT</layout-name>"
    ));
    assert!(contents.contains("<layout-name>UNUSED_LAYOUT</layout-name>"));
    assert!(contents.contains("<trans>r0 1,2</trans>"));
    assert!(contents.contains("<layout-name>TOP_LAYOUT</layout-name>"));
    assert!(contents.contains("<parent>ROOT</parent>"));
    assert!(contents.contains("<trans>r90 *1 17.5,-25</trans>"));
    assert!(contents.contains(
        "<ref>\n     <parent>ROOT</parent>\n     <trans>r90 *1 17.5,-25</trans>\n    </ref>"
    ));
    assert!(!contents.contains("<reference>"));
    assert!(!contents.contains("</reference>"));
    assert!(contents.contains(
        "    <category>\n     <name>Tagged-Values</name>\n     <description/>\n     <categories>\n     </categories>\n    </category>"
    ));
    assert!(!contents.contains("<description>Tagged-Values</description>"));
    assert!(contents.contains("<visited>true</visited>"));
    assert!(contents.contains("<multiplicity>7</multiplicity>"));
    assert!(!contents.contains("<multiplicity>1.5</multiplicity>"));
    assert!(contents.contains("<comment>reviewed before export</comment>"));
    assert!(contents.contains("<image>iVBORw0KGgo=</image>"));
    assert!(contents.contains(
        "<value>[#&apos;review-reference&apos;] text: &apos;reference: TOP/r0/9.0/10.0&apos;</value>"
    ));
    assert!(contents.contains(
        "<value>[#&apos;review-reference&apos;] text: &apos;reference: TOP/r0/11.0/12.0&apos;</value>"
    ));
    assert!(
        !contents
            .contains("<value>[#&apos;review-reference&apos;] reference: TOP/r0/9.0/10.0</value>")
    );
    assert!(
        !contents
            .contains("<value>[#&apos;review-reference&apos;] reference: TOP/r0/11.0/12.0</value>")
    );
    assert!(contents.contains("<value>text: &apos;KLayout scalar note&apos;</value>"));
    assert!(!contents.contains("<value>string: &apos;KLayout scalar note&apos;</value>"));
    assert!(contents.contains("<value>float: 1.25</value>"));
    assert!(contents.contains("<value>[#&apos;measured-width&apos;] float: 0.42</value>"));
    assert!(contents.contains(
        "<value>[#&apos;review-note&apos;] text: &apos;tagged scalar note&apos;</value>"
    ));
    assert!(contents.contains("<value>text: alpha_123</value>"));
    assert!(!contents.contains("<value>text: &apos;alpha_123&apos;</value>"));
    assert!(contents.contains("<value>text: &apos; leading scalar &apos;</value>"));
    assert!(contents.contains("<value>text: &apos;&apos;</value>"));
    assert!(contents.contains("<value>text: &apos;   &apos;</value>"));
    assert!(contents.contains("<category>Metrics.&apos;Tagged-Values&apos;</category>"));
    assert!(contents.contains("<value>[#&apos;geometry-review&apos;] box: (6,7;6.5,7.5)</value>"));
    assert!(contents.contains("<value>[#geometry_review_word] box: (6.1,7.1;6.2,7.2)</value>"));
    assert!(
        !contents
            .contains("<value>[#&apos;geometry_review_word&apos;] box: (6.1,7.1;6.2,7.2)</value>")
    );
    assert!(
        contents.contains(
            "<value>[#&apos;raw-review&apos;] text: &apos;glassworks-raw-value: unknown-tagged: preserved tagged raw&apos;</value>"
        )
    );
    assert!(
        contents.contains(
            "<value>[#&apos;count-review&apos;] text: &apos;glassworks-raw-value: integer: 7&apos;</value>"
        )
    );
    assert!(
        contents.contains(
            "<value>[#&apos;invalid-float&apos;] text: &apos;glassworks-raw-value: float: abc&apos;</value>"
        )
    );
    assert!(!contents.contains("<value>[#&apos;count-review&apos;] integer: 7</value>"));
    assert!(!contents.contains("<value>[#&apos;invalid-float&apos;] float: abc</value>"));
    assert!(contents.contains(
        "<value>text: &apos;glassworks-raw-value: unknown: preserved payload&apos;</value>"
    ));
    assert!(
        !contents.contains(
            "<value>[#&apos;raw-review&apos;] unknown-tagged: preserved tagged raw</value>"
        )
    );
    assert!(!contents.contains("<value>unknown: preserved payload</value>"));
    assert!(
        contents
            .contains("<value>polygon: (1,2;1.5,2;1.5,2.5;1,2.5/1.1,2.1;1.2,2.1;1.2,2.2)</value>")
    );
    assert!(contents.contains(
        "<value>[#&apos;review\\&apos;s scalar&apos;] text: &apos;Tagged legacy scalar&apos;</value>"
    ));
    assert!(!contents.contains(
        "<value>[#&apos;review\\&apos;s scalar&apos;] string: &apos;Tagged legacy scalar&apos;</value>"
    ));
    assert!(contents.contains("<value>box: (1.05,2.05;1.1,2.1)</value>"));
    assert!(!contents.contains("<value>rect: (1.05,2.05;1.1,2.1)</value>"));
    assert!(contents.contains("<value>edge-pair: (1.05,2.05;1.1,2.05)/(1.05,2.1;1.1,2.1)</value>"));
    assert!(
        !contents.contains("<value>edge_pair: (1.05,2.05;1.1,2.05)/(1.05,2.1;1.1,2.1)</value>")
    );
    assert!(contents.contains("<value>box: (1.25,2.25;1.25,2.25)</value>"));
    assert!(!contents.contains("<value>point: (1.25,2.25)</value>"));
    assert!(
        contents.contains("<value>label: (&apos;KLayout text marker&apos;,r90 2.0,3.0)</value>")
    );
    assert!(contents.contains(
        "<value>[#&apos;geometry\\&apos;s label&apos;] label: (&apos;KLayout \\&apos;quoted text marker&apos;,r90 2.0,3.0)</value>"
    ));
    assert!(
        !contents.contains("<value>text: (&apos;KLayout text marker&apos;,r90 2.0,3.0)</value>")
    );
    assert!(!contents.contains(
        "<value>[#&apos;geometry\\&apos;s label&apos;] text: (&apos;KLayout \\&apos;quoted text marker&apos;,r90 2.0,3.0)</value>"
    ));
    assert!(contents.contains("<value>text: &apos;width marker from Glassworks&apos;</value>"));
    let text_index = contents
        .find("text: &apos;width marker from Glassworks&apos;")
        .unwrap();
    let reference_index = contents
        .find("[#&apos;review-reference&apos;] text: &apos;reference: TOP/r0/9.0/10.0&apos;")
        .unwrap();
    let scalar_text_index = contents
        .find("text: &apos;KLayout scalar note&apos;")
        .unwrap();
    let float_index = contents.find("float: 1.25").unwrap();
    let unknown_index = contents
        .find("text: &apos;glassworks-raw-value: unknown: preserved payload&apos;")
        .unwrap();
    let polygon_index = contents.find("polygon: (1,2;1.5,2;1.5,2.5").unwrap();
    assert!(
        text_index < reference_index
            && reference_index < scalar_text_index
            && scalar_text_index < float_index
            && float_index < unknown_index
            && unknown_index < polygon_index,
        "imported item values should preserve original order"
    );
    assert!(!contents.contains("<value>box: (1,2;1.5,2.5)</value>"));
    assert!(!contents.contains("<value>box: (4,5;4.5,5.5)</value>"));
    assert!(contents.contains("<value>text: &apos;text-only export diagnostic&apos;</value>"));
    assert!(contents.contains(
        "<tags>#&apos;custom-rdb-tag&apos;,#hidden,#important,#&apos;review\\&apos;s item tag&apos;,#review_lane_final,#waived</tags>"
    ));
    assert!(!contents.contains(
        "<tags>#&apos;custom-rdb-tag&apos;,#hidden,#important,#&apos;review\\&apos;s item tag&apos;,#review_lane_final,#&apos;unused-declared-tag&apos;,#waived</tags>"
    ));
    assert!(contents.contains("<tags/>"));
    assert!(!contents.contains("<tags></tags>"));
    assert!(!contents.contains("rdb_raw_value"));
    assert!(!contents.contains("rdb_declared_cell"));
    assert!(!contents.contains("rdb_declared_category"));
    assert!(!contents.contains("rdb_declared_tag"));
    assert!(!contents.contains("rdb_geometry_value"));
    assert!(!contents.contains("rdb_item_ordered_value"));
    assert!(!contents.contains("rdb_category_description"));
    assert!(!contents.contains("rdb_category_part"));
    assert!(!contents.contains("rdb_multiplicity"));
    assert!(!contents.contains("rdb_tag_description"));
    assert!(!contents.contains("rdb_tag_name"));
    assert!(!contents.contains("rdb_item_value"));
    assert!(!contents.contains("rdb_item_reference"));
    assert!(!contents.contains("rdb_report_description"));
    assert!(!contents.contains("rdb_report_original_file"));
    assert!(!contents.contains("rdb_report_generator"));
    assert!(!contents.contains("rdb_report_top_cell"));
    assert!(!contents.contains("rdb_cell_layout_name"));
    assert!(!contents.contains("rdb_cell_reference"));

    let imported =
        import_klayout_rdb_markers(&contents).expect("exported KLayout RDB should parse");
    assert_eq!(
        imported.report.original_file.as_deref(),
        Some("/tmp/original-klayout-run.lyrdb")
    );
    assert_eq!(
        imported.report.generator.as_deref(),
        Some("KLayout DRC deck run")
    );
    assert_eq!(imported.report.top_cell.as_deref(), Some("TOP"));
    assert_eq!(imported.report.marker_count, 3);
    assert_eq!(imported.report.skipped_item_count, 0);
    assert_eq!(imported.findings.len(), 1);
    assert_eq!(
        imported.findings[0].severity,
        DrcValidationSeverity::Warning
    );
    assert!(
        imported.findings[0]
            .message
            .contains("text-only export diagnostic"),
        "{:?}",
        imported.findings
    );
    assert_eq!(
        imported.violations[0].rule,
        "klayout.layer_s_group_rule_name"
    );
    assert_eq!(
        imported.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1500, 2500))
    );
    assert_eq!(imported.violations[1].rule, "klayout.texts_dtext");
    assert_eq!(
        imported.violations[1].bounds,
        Rect::new(Point::new(1999, 2999), Point::new(2001, 3001))
    );
    let imported_text_state = imported
        .marker_states
        .get(&imported.violations[1].stable_key())
        .expect("exported KLayout text geometry marker state should parse");
    assert_eq!(
        imported_text_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("label: ('KLayout text marker',r90 2.0,3.0)")
    );
    assert_eq!(
        imported_text_state
            .tags
            .get("rdb_item_ordered_value_1")
            .map(String::as_str),
        Some("label: ('KLayout text marker',r90 2.0,3.0)")
    );
    assert_eq!(
        imported_text_state
            .tags
            .get("rdb_geometry_value_2")
            .map(String::as_str),
        Some("label: ('KLayout \\'quoted text marker',r90 2.0,3.0)")
    );
    assert_eq!(
        imported_text_state
            .tags
            .get("rdb_geometry_value_2_tag")
            .map(String::as_str),
        Some("geometry's label")
    );
    assert_eq!(
        imported_text_state
            .tags
            .get("rdb_item_ordered_value_2")
            .map(String::as_str),
        Some("[#'geometry\\'s label'] label: ('KLayout \\'quoted text marker',r90 2.0,3.0)")
    );
    let imported_scalar_state = imported
        .marker_states
        .values()
        .find(|state| {
            state
                .tags
                .get("rdb_item_value_1_tag")
                .is_some_and(|tag| tag == "measured-width")
        })
        .expect("exported tagged scalar values should re-import");
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_1_type")
            .map(String::as_str),
        Some("float")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_1_value")
            .map(String::as_str),
        Some("0.42")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_2_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_2_value")
            .map(String::as_str),
        Some("tagged scalar note")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_2_tag")
            .map(String::as_str),
        Some("review-note")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_3_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_3_value")
            .map(String::as_str),
        Some("alpha_123")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_4_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_4_value")
            .map(String::as_str),
        Some(" leading scalar ")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_5_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_5_empty")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_6_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_value_6_value_hex")
            .map(String::as_str),
        Some("202020")
    );
    assert!(
        !imported_scalar_state
            .tags
            .iter()
            .any(|(key, value)| key.starts_with("rdb_item_value_")
                && matches!(
                    value.as_str(),
                    "integer" | "abc" | "count-review" | "invalid-float"
                )),
        "unsupported scalar values should re-import as raw metadata, not typed scalar values"
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_reference_1")
            .map(String::as_str),
        Some("TOP/r0/11.0/12.0")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_item_reference_1_tag")
            .map(String::as_str),
        Some("review-reference")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("box: (6,7;6.5,7.5)")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_geometry_value_1_tag")
            .map(String::as_str),
        Some("geometry-review")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_geometry_value_2")
            .map(String::as_str),
        Some("box: (6.1,7.1;6.2,7.2)")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_geometry_value_2_tag")
            .map(String::as_str),
        Some("geometry_review_word")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_1")
            .map(String::as_str),
        Some("integer: 7")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_1_tag")
            .map(String::as_str),
        Some("count-review")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_2")
            .map(String::as_str),
        Some("float: abc")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_2_tag")
            .map(String::as_str),
        Some("invalid-float")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_3")
            .map(String::as_str),
        Some("unknown-tagged: preserved tagged raw")
    );
    assert_eq!(
        imported_scalar_state
            .tags
            .get("rdb_raw_value_3_tag")
            .map(String::as_str),
        Some("raw-review")
    );
    assert!(
        !imported_scalar_state.tags.contains_key("rdb_multiplicity"),
        "invalid exported multiplicity should fall back to KLayout-compatible default"
    );
    let imported_state = imported
        .marker_states
        .get(&imported.violations[0].stable_key())
        .expect("exported KLayout RDB marker state should parse");
    assert!(imported_state.visited);
    assert!(imported_state.hidden);
    assert!(imported_state.important);
    assert!(imported_state.waived);
    assert_eq!(
        imported_state.note.as_deref(),
        Some("reviewed before export")
    );
    assert_eq!(
        imported_state.tags.get("source_cell").map(String::as_str),
        Some("TOP:ANNOTATION")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_multiplicity")
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        imported_state
            .tags
            .get("review_lane_final")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        imported_state
            .tags
            .get("custom_rdb_tag")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        imported_state
            .tags
            .get("review_s_item_tag")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_name_review_s_item_tag")
            .map(String::as_str),
        Some("review's item tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_name_custom_rdb_tag")
            .map(String::as_str),
        Some("custom-rdb-tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_description_custom_rdb_tag")
            .map(String::as_str),
        Some("Custom KLayout RDB tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_description_review_s_item_tag")
            .map(String::as_str),
        Some("Quoted KLayout item tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_description_important")
            .map(String::as_str),
        Some("KLayout important review marker")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_description_review_lane_final")
            .map(String::as_str),
        Some("Review lane tag from KLayout")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_declared_tag_unused_declared_tag")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_name_unused_declared_tag")
            .map(String::as_str),
        Some("unused-declared-tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_tag_description_unused_declared_tag")
            .map(String::as_str),
        Some("Unused declared KLayout tag")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_image_base64")
            .map(String::as_str),
        Some("iVBORw0KGgo=")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_report_description")
            .map(String::as_str),
        Some("KLayout external review export")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_report_top_cell")
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_report_original_file")
            .map(String::as_str),
        Some("/tmp/original-klayout-run.lyrdb")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_report_generator")
            .map(String::as_str),
        Some("KLayout DRC deck run")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_1")
            .map(String::as_str),
        Some("text: 'width marker from Glassworks'")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_2")
            .map(String::as_str),
        Some("[#'review-reference'] text: 'reference: TOP/r0/9.0/10.0'")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_3")
            .map(String::as_str),
        Some("text: 'KLayout scalar note'")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_5")
            .map(String::as_str),
        Some("unknown: preserved payload")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_6")
            .map(String::as_str),
        Some("polygon: (1,2;1.5,2;1.5,2.5;1,2.5/1.1,2.1;1.2,2.1;1.2,2.2)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_7")
            .map(String::as_str),
        Some("[#'review\\'s scalar'] text: 'Tagged legacy scalar'")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_8")
            .map(String::as_str),
        Some("box: (1.05,2.05;1.1,2.1)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_9")
            .map(String::as_str),
        Some("edge-pair: (1.05,2.05;1.1,2.05)/(1.05,2.1;1.1,2.1)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_ordered_value_10")
            .map(String::as_str),
        Some("box: (1.25,2.25;1.25,2.25)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_reference_1")
            .map(String::as_str),
        Some("TOP/r0/9.0/10.0")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_reference_1_tag")
            .map(String::as_str),
        Some("review-reference")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_1_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_1_value")
            .map(String::as_str),
        Some("width marker from Glassworks")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_2_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_2_value")
            .map(String::as_str),
        Some("KLayout scalar note")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_3_type")
            .map(String::as_str),
        Some("float")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_3_value")
            .map(String::as_str),
        Some("1.25")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_4_type")
            .map(String::as_str),
        Some("text")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_4_value")
            .map(String::as_str),
        Some("Tagged legacy scalar")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_item_value_4_tag")
            .map(String::as_str),
        Some("review's scalar")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_raw_value_1")
            .map(String::as_str),
        Some("unknown: preserved payload")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_geometry_value_1")
            .map(String::as_str),
        Some("polygon: (1,2;1.5,2;1.5,2.5;1,2.5/1.1,2.1;1.2,2.1;1.2,2.2)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_geometry_value_2")
            .map(String::as_str),
        Some("box: (1.05,2.05;1.1,2.1)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_geometry_value_3")
            .map(String::as_str),
        Some("edge-pair: (1.05,2.05;1.1,2.05)/(1.05,2.1;1.1,2.1)")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_geometry_value_4")
            .map(String::as_str),
        Some("box: (1.25,2.25;1.25,2.25)")
    );
    assert!(!imported_state.tags.contains_key("rdb_raw_value_2"));
    assert_eq!(
        imported_state.tags.get("rdb_category").map(String::as_str),
        Some("Layer's.Group/Rule.Name")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_category_part_1")
            .map(String::as_str),
        Some("Layer's.Group")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_category_part_2")
            .map(String::as_str),
        Some("Rule.Name")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_category_description_1")
            .map(String::as_str),
        Some("Layer apostrophe category from KLayout")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_category_description_2")
            .map(String::as_str),
        Some("Quoted dot category from KLayout")
    );
    let unused_declared_category_index = (1..=16)
        .find(|index| {
            imported_state
                .tags
                .get(&format!("rdb_declared_category_{index}_part_1"))
                .is_some_and(|part| part == "Unused")
                && imported_state
                    .tags
                    .get(&format!("rdb_declared_category_{index}_part_2"))
                    .is_some_and(|part| part == "Review")
        })
        .expect("unused declared category should re-import");
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_category_{unused_declared_category_index}_description"
            ))
            .map(String::as_str),
        Some("Unused declared KLayout category")
    );
    let unused_declared_cell_index = (1..=16)
        .find(|index| {
            imported_state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "UNUSED_CELL")
        })
        .expect("unused declared cell should re-import");
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_variant"
            ))
            .map(String::as_str),
        Some("7")
    );
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_layout_name"
            ))
            .map(String::as_str),
        Some("UNUSED_LAYOUT")
    );
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_parent"
            ))
            .map(String::as_str),
        Some("TOP")
    );
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_cell_{unused_declared_cell_index}_reference_1_trans"
            ))
            .map(String::as_str),
        Some("r0 1,2")
    );
    let colliding_variant_cell_index = (1..=16)
        .find(|index| {
            imported_state
                .tags
                .get(&format!("rdb_declared_cell_{index}_name"))
                .is_some_and(|name| name == "TOP")
                && imported_state
                    .tags
                    .get(&format!("rdb_declared_cell_{index}_variant"))
                    .is_some_and(|variant| variant == "ANNOTATION")
        })
        .expect("declared cell variant should not overwrite literal colon cell metadata");
    assert_eq!(
        imported_state
            .tags
            .get(&format!(
                "rdb_declared_cell_{colliding_variant_cell_index}_layout_name"
            ))
            .map(String::as_str),
        Some("TOP_VARIANT_LAYOUT")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_cell_layout_name")
            .map(String::as_str),
        Some("TOP_LAYOUT")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_cell_reference_1_parent")
            .map(String::as_str),
        Some("ROOT")
    );
    assert_eq!(
        imported_state
            .tags
            .get("rdb_cell_reference_1_trans")
            .map(String::as_str),
        Some("r90 *1 17.5,-25")
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&image_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_drc_report_database_export_writes_compressed_klayout_lyrdb() {
    use std::io::Read as _;

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("Compressed KLayout RDB export source");
    app.reset_layout_document_state();
    app.push_layout_drc_report(
        "Compressed KLayout Export",
        DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![DrcViolation {
                id: 1,
                rule: "external.compressed".to_string(),
                message: "compressed export marker".to_string(),
                shape_ids: Vec::new(),
                occurrence_ids: Vec::new(),
                bounds: Rect::new(Point::new(1000, 2000), Point::new(1500, 2500)),
                required: 0,
                actual: 0.0,
            }],
        },
    );
    let path = std::env::temp_dir().join(format!(
        "glassworks-compressed-klayout-rdb-export-{}.lyrdb.gz",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.export_layout_drc_report_database_to_path(&path));
    assert!(
        app.status_message()
            .contains("Exported KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.app_options.files.recent_files[0].kind,
        "drc_report_database"
    );
    let raw = std::fs::read(&path).expect("compressed KLayout RDB export should write");
    assert!(raw.starts_with(&[0x1f, 0x8b]));
    let mut decoder = flate2::read::GzDecoder::new(raw.as_slice());
    let mut contents = String::new();
    decoder
        .read_to_string(&mut contents)
        .expect("compressed KLayout RDB export should decompress");
    assert!(contents.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(contents.contains("<report-database>"));
    assert!(
        contents
            .contains("<description>Glassworks DRC report database (1 report(s))</description>")
    );
    assert!(contents.contains("<category>compressed</category>"));
    assert!(contents.contains("<value>text: &apos;compressed export marker&apos;</value>"));
    assert!(!contents.contains("\"schema_version\""));

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(imported.import_layout_drc_report_database_from_path(&path));
    assert!(
        imported
            .status_message()
            .contains("Imported KLayout RDB report database"),
        "{}",
        imported.status_message()
    );
    let report = imported
        .drc_report()
        .expect("compressed exported KLayout RDB should import");
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].rule, "klayout.compressed");
    assert_eq!(report.violations[0].message, "compressed export marker");
    assert_eq!(
        report.violations[0].bounds,
        Rect::new(Point::new(1000, 2000), Point::new(1500, 2500))
    );

    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_klayout_rdb_report_database_actions_use_lyrdb_exchange_path() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("KLayout RDB action source");
    app.reset_layout_document_state();
    app.push_layout_drc_report(
        "KLayout RDB Action Export",
        DrcReportCacheValue {
            findings: Vec::new(),
            violations: vec![DrcViolation {
                id: 1,
                rule: "external.action".to_string(),
                message: "KLayout action marker".to_string(),
                shape_ids: Vec::new(),
                occurrence_ids: Vec::new(),
                bounds: Rect::new(Point::new(2000, 3000), Point::new(2600, 3600)),
                required: 0,
                actual: 0.0,
            }],
        },
    );
    let path = default_ui_klayout_rdb_exchange_path();
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.klayout_rdb_export"));
    assert!(
        app.status_message()
            .contains("Exported KLayout RDB report database"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.app_options.files.recent_files[0].path,
        path.display().to_string()
    );
    let contents = std::fs::read_to_string(&path).expect("KLayout RDB action should write XML");
    assert!(contents.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(contents.contains("<report-database>"));
    assert!(contents.contains("<value>text: &apos;KLayout action marker&apos;</value>"));
    assert!(!contents.contains("\"schema_version\""));

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(imported.apply_clicked_node_name("glassworks.viewctl.layout.klayout_rdb_import"));
    assert!(
        imported
            .status_message()
            .contains("Imported KLayout RDB report database"),
        "{}",
        imported.status_message()
    );
    assert_eq!(imported.layout_drc_report_history.len(), 1);
    let report = imported
        .drc_report()
        .expect("KLayout RDB action import should populate a report");
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].rule, "klayout.action");
    assert_eq!(report.violations[0].message, "KLayout action marker");

    assert!(imported.apply_clicked_node_name("glassworks.viewctl.layout.klayout_rdb_append"));
    assert!(
        imported
            .status_message()
            .contains("Appended KLayout RDB report database"),
        "{}",
        imported.status_message()
    );
    assert_eq!(imported.layout_drc_report_history.len(), 2);

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
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_signoff.needs_review")
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.drc_marker_tag.source_external")
    );
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
                    && state.signoff_by.as_deref() == Some("process-owner")
                    && state.signoff_note.as_deref() == Some("reviewed in marker browser")
                    && state
                        .signoff_records
                        .get("process-owner")
                        .is_some_and(|signoff| {
                            signoff.status == "needs_review"
                                && signoff.role.as_deref() == Some("process")
                                && signoff.by.as_deref() == Some("process-owner")
                                && signoff.note.as_deref() == Some("reviewed in marker browser")
                                && signoff.recorded_at.as_deref().is_some_and(|recorded_at| {
                                    recorded_at.starts_with("layout_revision:")
                                })
                        })
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
                    && state.signoff_by.as_deref() == Some("process-owner")
                    && state.signoff_note.as_deref() == Some("reviewed in marker browser")
                    && state
                        .signoff_records
                        .get("process-owner")
                        .is_some_and(|signoff| {
                            signoff.status == "needs_review"
                                && signoff.role.as_deref() == Some("process")
                                && signoff.by.as_deref() == Some("process-owner")
                                && signoff.note.as_deref() == Some("reviewed in marker browser")
                                && signoff.recorded_at.as_deref().is_some_and(|recorded_at| {
                                    recorded_at.starts_with("layout_revision:")
                                })
                        })
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
        "glassworks.menu.item.layout.klayout_rdb_export",
        "glassworks.menu.item.layout.klayout_rdb_import",
        "glassworks.menu.item.layout.klayout_rdb_append",
        "glassworks.menu.item.layout.calibre_rve_import",
        "glassworks.menu.item.layout.drc_markers.write_layer",
        "glassworks.menu.item.layout.drc_marker_snapshot",
        "glassworks.menu.item.layout.drc_marker_snapshot_png",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("Tools menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}
