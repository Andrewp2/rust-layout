#![allow(unused_imports)]
use super::*;
use crate::*;

#[test]
pub(crate) fn layout_layer_panel_marks_used_and_unused_layers() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(app.add_layout_layer());
    let removable_empty_layer = app.active_layer();
    assert!(app.add_layout_layer());
    let unused_layer = app.active_layer();
    assert_eq!(
        layout_layer_usage_counts(&app.workspace.document)
            .get(&unused_layer)
            .copied()
            .unwrap_or(0),
        0
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    let text_by_node = document_text_by_node(&document);
    assert!(
        text_by_node
            .get("glassworks.layers.usage.summary")
            .is_some_and(|text| text.starts_with("Used layers: ")),
        "layer panel should summarize used vs total layers"
    );
    assert!(
        text_by_node
            .get("glassworks.layers.usage_filter.summary")
            .is_some_and(|text| text.starts_with("Layer rows: All")),
        "layer panel should summarize the layer-row usage filter"
    );
    assert!(
        document_visible_text(&document).contains("Usage: 0 shapes"),
        "active empty layer should be marked as unused in properties"
    );
    assert!(
        text_by_node
            .get(&format!(
                "glassworks.viewctl.layout.layer.{}.label",
                unused_layer.0
            ))
            .is_some_and(|text| text.starts_with("0 ")),
        "empty layer row should use the unused marker"
    );
    for node_name in [
        "glassworks.viewctl.layout.layer_visibility.show_all",
        "glassworks.viewctl.layout.layer_visibility.show_used",
        "glassworks.viewctl.layout.layer_visibility.hide_empty",
        "glassworks.viewctl.layout.layer_visibility.isolate_active",
        "glassworks.viewctl.layout.layer_visibility.invert",
        "glassworks.viewctl.layout.layer_cleanup.delete_empty",
        "glassworks.viewctl.layout.layer_group.all",
        "glassworks.viewctl.layout.layer_group.front_end",
        "glassworks.viewctl.layout.layer_group.feol_diffusion",
        "glassworks.viewctl.layout.layer_group.feol_gate",
        "glassworks.viewctl.layout.layer_group.feol_dielectric",
        "glassworks.viewctl.layout.layer_group.routing",
        "glassworks.viewctl.layout.layer_group.routing_contact",
        "glassworks.viewctl.layout.layer_group.routing_metal",
        "glassworks.viewctl.layout.layer_group.routing_via",
        "glassworks.viewctl.layout.layer_group.annotation",
        "glassworks.viewctl.layout.layer_usage_filter.all",
        "glassworks.viewctl.layout.layer_usage_filter.used",
        "glassworks.viewctl.layout.layer_usage_filter.empty",
        "glassworks.viewctl.layout.layer_usage_filter.visible",
        "glassworks.viewctl.layout.layer_usage_filter.hidden",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "layer panel should expose group/visibility control {node_name}"
        );
    }
    let filter_button_label = |document: &UiDocument, node_name: &str| -> String {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .and_then(|node| node.accessibility())
            .and_then(|accessibility| accessibility.label.clone())
            .unwrap_or_else(|| panic!("{node_name} should expose an accessibility label"))
    };
    let expected_filter_label = |filter: LayoutLayerUsageFilter| -> String {
        let usage = layout_layer_usage_counts(&app.workspace.document);
        let count = app
            .workspace
            .document
            .layers
            .values()
            .filter(|layer| app.layout_layer_group_filter.matches(layer.process))
            .filter(|layer| {
                filter.matches(usage.get(&layer.id).copied().unwrap_or(0), layer.visible)
            })
            .count();
        format!("{} ({count})", filter.label())
    };
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_usage_filter.all"
        ),
        expected_filter_label(LayoutLayerUsageFilter::All)
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_usage_filter.used"
        ),
        expected_filter_label(LayoutLayerUsageFilter::Used)
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_usage_filter.empty"
        ),
        expected_filter_label(LayoutLayerUsageFilter::Empty)
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_usage_filter.visible"
        ),
        expected_filter_label(LayoutLayerUsageFilter::Visible)
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_usage_filter.hidden"
        ),
        expected_filter_label(LayoutLayerUsageFilter::Hidden)
    );
    let expected_visibility_label = |mode: &str, label: &str| -> String {
        format!(
            "{label} ({})",
            layout_layer_visibility_preset_change_count(
                &app.workspace.document,
                app.active_layer,
                mode
            )
            .expect("known layer visibility preset should count")
        )
    };
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_visibility.show_all"
        ),
        expected_visibility_label("show_all", "All On")
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_visibility.show_used"
        ),
        expected_visibility_label("show_used", "Used On")
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_visibility.hide_empty"
        ),
        expected_visibility_label("hide_empty", "Empty Off")
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_visibility.isolate_active"
        ),
        expected_visibility_label("isolate_active", "Solo Active")
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_visibility.invert"
        ),
        expected_visibility_label("invert", "Invert")
    );
    assert_eq!(
        filter_button_label(
            &document,
            "glassworks.viewctl.layout.layer_cleanup.delete_empty"
        ),
        format!(
            "Prune Empty ({})",
            layout_inactive_empty_layer_count(&app.workspace.document, app.active_layer)
        )
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_cleanup.delete_empty"));
    assert!(
        app.workspace
            .document
            .layers
            .get(&removable_empty_layer)
            .is_none(),
        "empty inactive layer should be removed by bulk cleanup"
    );
    assert!(
        app.workspace.document.layers.contains_key(&unused_layer),
        "bulk cleanup should preserve the active empty layer"
    );
    assert!(app.status_message().contains("inactive empty layer"));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        app.workspace
            .document
            .layers
            .contains_key(&removable_empty_layer),
        "bulk empty-layer cleanup should be undoable"
    );

    let used_layer = app
        .workspace
        .document
        .shapes
        .values()
        .next()
        .map(|shape| shape.layer)
        .expect("demo document should include a used layer");
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.used"));
    assert_eq!(app.layout_layer_usage_filter, LayoutLayerUsageFilter::Used);
    assert!(
        app.status_message().contains("Layer rows Used ("),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.layout.layer_usage_filter, "used");
    let used_doc = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("used layer filter document should build");
    assert!(
        used_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", used_layer.0))
    );
    assert!(
        !used_doc.nodes().iter().any(
            |node| node.name() == format!("glassworks.viewctl.layout.layer.{}", unused_layer.0)
        ),
        "used-only row filter should hide empty layer rows without changing visibility"
    );
    assert!(
        app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused layer should exist")
            .visible,
        "usage row filter should not hide layers in the layout"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.empty"));
    let empty_doc = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("empty layer filter document should build");
    assert!(
        empty_doc.nodes().iter().any(
            |node| node.name() == format!("glassworks.viewctl.layout.layer.{}", unused_layer.0)
        )
    );
    assert!(
        !empty_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", used_layer.0))
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.all"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_visibility.hide_empty"));
    assert!(
        !app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused layer should exist")
            .visible,
        "hide-empty preset should hide unused layers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.hidden"));
    assert_eq!(
        app.layout_layer_usage_filter,
        LayoutLayerUsageFilter::Hidden
    );
    app.sync_app_options_from_state();
    assert_eq!(app.app_options.layout.layer_usage_filter, "hidden");
    let hidden_doc = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("hidden layer filter document should build");
    assert!(
        hidden_doc.nodes().iter().any(
            |node| node.name() == format!("glassworks.viewctl.layout.layer.{}", unused_layer.0)
        )
    );
    assert!(
        !hidden_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", used_layer.0)),
        "hidden row filter should hide visible layer rows without changing visibility"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.visible"));
    assert_eq!(
        app.layout_layer_usage_filter,
        LayoutLayerUsageFilter::Visible
    );
    app.sync_app_options_from_state();
    assert_eq!(app.app_options.layout.layer_usage_filter, "visible");
    let visible_doc = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("visible layer filter document should build");
    assert!(
        visible_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", used_layer.0))
    );
    assert!(
        !visible_doc.nodes().iter().any(
            |node| node.name() == format!("glassworks.viewctl.layout.layer.{}", unused_layer.0)
        ),
        "visible row filter should hide hidden layer rows without changing visibility"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_visibility.show_all"));
    assert!(
        app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused layer should exist")
            .visible,
        "show-all preset should restore unused layers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_visibility.show_used"));
    assert!(
        app.workspace
            .document
            .layers
            .get(&used_layer)
            .expect("used layer should exist")
            .visible,
        "show-used preset should show used layers"
    );
    assert!(
        !app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused layer should exist")
            .visible,
        "show-used preset should hide unused layers"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.layout.layer_visibility.isolate_active")
    );
    assert!(
        app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused active layer should exist")
            .visible,
        "active-layer isolation should show the active layer even when empty"
    );
    assert!(
        !app.workspace
            .document
            .layers
            .get(&used_layer)
            .expect("used layer should exist")
            .visible,
        "active-layer isolation should hide non-active layers"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_visibility.invert"));
    assert!(
        !app.workspace
            .document
            .layers
            .get(&unused_layer)
            .expect("unused active layer should exist")
            .visible,
        "visibility invert should hide layers that were visible"
    );
    assert!(
        app.workspace
            .document
            .layers
            .get(&used_layer)
            .expect("used layer should exist")
            .visible,
        "visibility invert should show layers that were hidden"
    );
}

#[test]
pub(crate) fn layout_technology_stack_browser_exposes_connectivity_and_rule_counts() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.active_layer = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");

    let rows = layout_technology_stack_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| { key == "Connectivity" && value.contains("stack link") }),
        "technology stack rows should summarize connectivity links"
    );
    assert!(
        rows.iter().any(|(key, value)| {
            key == "DRC rules" && value.contains("width") && value.contains("spacing")
        }),
        "technology stack rows should summarize DRC rule families"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Active mapping" && value.contains("GDS")),
        "technology stack rows should expose active-layer exchange mapping"
    );
    assert!(
        rows.iter().any(|(key, value)| {
            key == "Active stack links" && !value.contains("No stack links")
        }),
        "metal1 should participate in the default connectivity stack"
    );
    let total_technology_rows = rows.len();
    app.set_layout_browser_search("stack=1");
    let stack_rows = layout_technology_stack_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        stack_rows.get("Listed entries"),
        Some(&format!("1 / {total_technology_rows} entries"))
    );
    assert!(
        stack_rows.contains_key("Stack 1"),
        "technology stack selector search should keep the matching stack link: {stack_rows:?}"
    );
    assert!(
        !stack_rows.contains_key("Stack 2"),
        "technology stack selector search should hide non-matching stack links: {stack_rows:?}"
    );
    app.set_layout_browser_search("active_layer=metal1");
    let active_layer_rows = layout_technology_stack_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(
        active_layer_rows.contains_key("Active layer")
            && active_layer_rows.contains_key("Active stack links"),
        "technology stack active-layer selector should keep active-layer rows: {active_layer_rows:?}"
    );
    app.set_layout_browser_search("");
    assert!(
        layout_editor_inspector_sections(&app)
            .iter()
            .any(|section| section.title == "Technology Stack"),
        "layout inspector should expose technology stack details"
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layers.technology.title"),
        "layer side panel should expose technology stack title"
    );
    let visible_text = document_visible_text(&document);
    assert!(
        visible_text.contains("Technology Stack")
            && visible_text.contains("Connectivity:")
            && visible_text.contains("DRC rules:"),
        "layer side panel should list connectivity and DRC technology details"
    );
}

#[test]
pub(crate) fn layout_technology_selection_rebuilds_rules_and_persists_options() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    assert_eq!(app.workspace.document.grid, 10);
    assert_eq!(
        app.active_layout_drc_deck_for(&app.workspace.document)
            .min_width
            .get(&metal1),
        Some(&220)
    );

    assert!(app.add_layout_layer());
    let extra_layer = app.active_layer;
    assert!(
        app.workspace.document.layers.contains_key(&extra_layer),
        "test should create an extra non-technology layer"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.tools.technology.1"));
    assert_eq!(app.app_options.layout.technology, "glassworks_high_density");
    assert_eq!(app.layout_active_technology_index, 1);
    assert_eq!(app.workspace.document.grid, 5);
    assert!(
        app.workspace.document.layers.contains_key(&extra_layer),
        "technology switching should preserve imported/custom extra layers"
    );
    assert!(
        layout_technology_stack_rows(&app)
            .iter()
            .any(|(key, value)| { key == "Technology" && value.contains("High-density") }),
        "technology stack rows should reflect the selected built-in technology"
    );
    assert_eq!(
        app.active_layout_drc_deck_for(&app.workspace.document)
            .min_width
            .get(&metal1),
        Some(&150),
        "built-in DRC deck should come from the selected technology"
    );

    let mut options = AppOptions::default();
    options.layout.technology = "glassworks_high_density".to_string();
    let app = GlassworksApp::new_with_options(StartupOptions {
        app_options: Some(options),
        ..Default::default()
    });
    assert_eq!(app.layout_active_technology_index, 1);
    assert_eq!(app.workspace.document.grid, 5);
}

#[test]
pub(crate) fn layout_connectivity_stack_link_toggle_limits_net_extraction() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("connectivity stack toggle");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let via1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Via1)
        .expect("default technology should include via1");
    let metal2 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal2)
        .expect("default technology should include metal2");
    for layer in [metal1, via1, metal2] {
        app.add_layout_shape(
            layer,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(300, 300))),
        )
        .expect("stack fixture shape should be added");
    }

    let enabled_report = app
        .connectivity_report()
        .expect("enabled stack should extract");
    assert_eq!(
        enabled_report.components.len(),
        1,
        "metal1/via1/metal2 overlap should be connected before disabling the stack link"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.connectivity_link.toggle.1"));
    assert_eq!(app.app_options.layout.disabled_connectivity_links, vec![1]);
    let disabled_report = app
        .connectivity_report()
        .expect("disabled stack should still extract");
    let enabled_shape_count = enabled_report
        .components
        .iter()
        .map(|component| component.shapes.len())
        .sum::<usize>();
    let disabled_shape_count = disabled_report
        .components
        .iter()
        .map(|component| component.shapes.len())
        .sum::<usize>();
    assert!(
        disabled_shape_count < enabled_shape_count,
        "disabling the metal1/via1/metal2 stack link should remove that stack from net extraction"
    );
    let rows = layout_technology_stack_rows(&app);
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Connectivity" && value.contains("1/2")),
        "technology stack rows should summarize disabled links"
    );
    assert!(
        rows.iter()
            .any(|(key, value)| key == "Stack 2" && value.contains("disabled")),
        "technology stack rows should mark the disabled link"
    );
    let total_technology_rows = rows.len();
    app.set_layout_browser_search("state=disabled");
    let disabled_stack_rows = layout_technology_stack_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        disabled_stack_rows.get("Listed entries"),
        Some(&format!("1 / {total_technology_rows} entries"))
    );
    assert!(
        disabled_stack_rows.contains_key("Stack 2"),
        "technology stack state selector search should keep the disabled link: {disabled_stack_rows:?}"
    );
    assert!(
        !disabled_stack_rows.contains_key("Stack 1"),
        "technology stack state selector search should hide enabled stack links: {disabled_stack_rows:?}"
    );
    app.set_layout_browser_search("");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document.nodes().iter().any(|node| {
            node.name() == "glassworks.viewctl.layout.connectivity_links.enable_all"
        }),
        "layer panel should expose a control to re-enable disabled stack links"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.connectivity_links.enable_all"));
    assert!(app.layout_disabled_connectivity_links.is_empty());
    assert!(
        app.app_options
            .layout
            .disabled_connectivity_links
            .is_empty()
    );
}

#[test]
pub(crate) fn layout_layer_group_filter_limits_layer_panel_rows_and_options() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let diffusion = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Diffusion)
        .expect("default technology should include diffusion");
    let poly = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Poly)
        .expect("default technology should include poly");
    let contact = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Contact)
        .expect("default technology should include contact");
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

    let initial = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document_visible_text(&initial).contains("Layer group: All"),
        "layer panel should summarize the active layer group"
    );
    let initial_text = document_visible_text(&initial);
    assert!(initial_text.contains("Layer Group Tree"), "{initial_text}");
    assert!(initial_text.contains("FEOL / Diffusion"), "{initial_text}");
    assert!(initial_text.contains("Routing / Metals"), "{initial_text}");
    assert!(
        layout_editor_inspector_sections(&app)
            .iter()
            .any(|section| section.title == "Layer Groups"),
        "layout inspector should expose the layer group directory"
    );
    app.set_layout_browser_search("group_slug=routing_metal");
    let group_rows = layout_layer_group_directory_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        group_rows.get("Listed groups").map(String::as_str),
        Some("1 / 10 groups")
    );
    assert!(
        group_rows.contains_key("Routing / Metals"),
        "layer-group selector search should keep the matching group: {group_rows:?}"
    );
    assert!(
        !group_rows.contains_key("Routing / Vias"),
        "layer-group selector search should hide non-matching groups: {group_rows:?}"
    );
    app.set_layout_browser_search("parent=routing");
    let group_rows = layout_layer_group_directory_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        group_rows.get("Listed groups").map(String::as_str),
        Some("3 / 10 groups")
    );
    assert!(
        group_rows.contains_key("Routing / Contacts")
            && group_rows.contains_key("Routing / Metals")
            && group_rows.contains_key("Routing / Vias"),
        "layer-group parent selector search should keep child groups: {group_rows:?}"
    );
    app.set_layout_browser_search("");
    assert!(
        initial
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", diffusion.0))
    );
    assert!(
        initial
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", metal1.0))
    );
    for node_name in [
        "glassworks.viewctl.layout.layer_group.feol_diffusion",
        "glassworks.viewctl.layout.layer_group.feol_gate",
        "glassworks.viewctl.layout.layer_group.feol_dielectric",
        "glassworks.viewctl.layout.layer_group.routing_contact",
        "glassworks.viewctl.layout.layer_group.routing_metal",
        "glassworks.viewctl.layout.layer_group.routing_via",
    ] {
        assert!(
            initial.nodes().iter().any(|node| node.name() == node_name),
            "layer panel should expose hierarchical group control {node_name}"
        );
    }
    let group_button_label = |document: &UiDocument, node_name: &str| -> String {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .and_then(|node| node.accessibility())
            .and_then(|accessibility| accessibility.label.clone())
            .unwrap_or_else(|| panic!("{node_name} should expose an accessibility label"))
    };
    let expected_group_label = |group: LayoutLayerGroupFilter, label: &str| -> String {
        let count = layout_layer_row_filter_count(
            &app.workspace.document,
            group,
            app.layout_layer_usage_filter,
        );
        format!("{label} ({count})")
    };
    assert_eq!(
        group_button_label(&initial, "glassworks.viewctl.layout.layer_group.all"),
        expected_group_label(LayoutLayerGroupFilter::All, "All")
    );
    assert_eq!(
        group_button_label(&initial, "glassworks.viewctl.layout.layer_group.routing"),
        expected_group_label(LayoutLayerGroupFilter::Routing, "Routing")
    );
    assert_eq!(
        group_button_label(
            &initial,
            "glassworks.viewctl.layout.layer_group.routing_metal"
        ),
        expected_group_label(LayoutLayerGroupFilter::RoutingMetal, "Metal")
    );
    assert_eq!(
        group_button_label(&initial, "glassworks.viewctl.layout.layer_group.annotation"),
        expected_group_label(LayoutLayerGroupFilter::Annotation, "Text")
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.routing"));
    assert_eq!(
        app.layout_layer_group_filter,
        LayoutLayerGroupFilter::Routing
    );
    assert!(
        app.status_message().contains("Layer group Routing ("),
        "{}",
        app.status_message()
    );
    app.set_layout_browser_search("state=current");
    let active_group_rows = layout_layer_group_directory_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        active_group_rows.get("Listed groups").map(String::as_str),
        Some("1 / 10 groups")
    );
    assert!(
        active_group_rows.contains_key("Routing"),
        "layer-group current-state selector search should keep the active group: {active_group_rows:?}"
    );
    assert!(
        !active_group_rows.contains_key("All"),
        "layer-group current-state selector search should hide inactive groups: {active_group_rows:?}"
    );
    app.set_layout_browser_search("");
    let routing = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("routing layer document should build");
    assert!(
        document_visible_text(&routing).contains("Layer group: Routing"),
        "routing layer group should be visible"
    );
    assert!(
        routing
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", metal1.0))
    );
    assert!(
        !routing
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", diffusion.0))
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.routing_metal"));
    assert_eq!(
        app.layout_layer_group_filter,
        LayoutLayerGroupFilter::RoutingMetal
    );
    assert!(app.status_message().contains("Routing / Metals"));
    let routing_metal = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("routing metal layer document should build");
    let routing_metal_text = document_visible_text(&routing_metal);
    assert!(
        routing_metal_text.contains("Layer group: Routing / Metals"),
        "{routing_metal_text}"
    );
    assert!(
        routing_metal
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", metal1.0))
    );
    assert!(
        !routing_metal
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", contact.0))
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.annotation"));
    let annotation_doc = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("annotation layer document should build");
    assert!(
        annotation_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", annotation.0))
    );
    assert!(
        !annotation_doc
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", metal1.0))
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.feol_diffusion"));
    let feol_diffusion = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("FEOL diffusion layer document should build");
    assert!(document_visible_text(&feol_diffusion).contains("Layer group: FEOL / Diffusion"));
    assert!(
        feol_diffusion
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", diffusion.0))
    );
    assert!(
        !feol_diffusion
            .nodes()
            .iter()
            .any(|node| node.name() == format!("glassworks.viewctl.layout.layer.{}", poly.0))
    );

    app.sync_app_options_from_state();
    assert_eq!(app.app_options.layout.layer_group_filter, "feol_diffusion");
}

#[test]
pub(crate) fn layout_layer_display_style_controls_update_active_layer() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.layer.{}", metal1.0)));
    let original_alpha = app.workspace.document.layer_color(metal1)[3];
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    let visible_text = document_visible_text(&document);
    assert!(visible_text.contains("Fill style: Solid"), "{visible_text}");
    assert!(visible_text.contains("Line style: Solid"), "{visible_text}");
    for node_name in [
        "glassworks.viewctl.layout.layer_fill_style.cycle",
        "glassworks.viewctl.layout.layer_line_style.cycle",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "layer properties should expose {node_name}"
        );
    }

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::Outline);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
        assert!(app.workspace.document.layer_display_color(metal1)[3] < original_alpha);
    }
    assert!(app.status_message().contains("fill Outline"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::Hatched);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
    }
    assert!(app.status_message().contains("fill Hatched"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::CrossHatched);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
    }
    assert!(app.status_message().contains("fill Cross Hatch"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::Stippled);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
    }
    assert!(app.status_message().contains("fill Stipple"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::DenseStippled);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
    }
    assert!(app.status_message().contains("fill Dense Stipple"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_fill_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::SparseStippled);
        assert_eq!(layer.line_style, LayerLineStyle::Solid);
    }
    assert!(app.status_message().contains("fill Sparse Stipple"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_line_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::SparseStippled);
        assert_eq!(layer.line_style, LayerLineStyle::Dashed);
    }
    assert!(app.status_message().contains("line Dashed"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_line_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::SparseStippled);
        assert_eq!(layer.line_style, LayerLineStyle::Dotted);
    }
    assert!(app.status_message().contains("line Dotted"));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_line_style.cycle"));
    {
        let layer = app.workspace.document.layer(metal1).unwrap();
        assert_eq!(layer.fill_style, LayerFillStyle::SparseStippled);
        assert_eq!(layer.line_style, LayerLineStyle::DashDot);
    }
    assert!(app.status_message().contains("line Dash Dot"));

    let styled = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("styled layer document should build");
    let styled_text = document_visible_text(&styled);
    assert!(
        styled_text.contains("Fill style: Sparse Stipple"),
        "{styled_text}"
    );
    assert!(
        styled_text.contains("Line style: Dash Dot"),
        "{styled_text}"
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        app.workspace.document.layer(metal1).unwrap().line_style,
        LayerLineStyle::Dotted
    );
}

#[test]
pub(crate) fn layout_layer_hierarchy_depth_overrides_filter_displayed_occurrences() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("layer hierarchy depth override");
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
    let top = app.workspace.document.top_cell;
    let child = app.workspace.document.create_cell("layered child");
    app.workspace.document.insert_shape(
        metal1,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 50, 50)),
    );
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal1,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(100, 0), 50, 50)),
        )
        .expect("child metal1 shape should be inserted");
    app.workspace
        .document
        .insert_shape_in_cell(
            child,
            metal2,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(200, 0), 50, 50)),
        )
        .expect("child metal2 shape should be inserted");
    app.workspace
        .document
        .insert_instance(top, child, Transform::IDENTITY)
        .expect("child instance should be inserted");
    app.active_layer = metal1;
    app.mark_layout_dirty();

    assert_eq!(app.with_layout_display_index(|index| index.len()), 3);
    app.layout_shape_browser_filter = LayoutShapeBrowserFilter::ActiveLayer;
    assert_eq!(layout_shape_browser_entries(&app).len(), 2);

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    let visible_text = document_visible_text(&document);
    assert!(
        visible_text.contains("Hierarchy depth: inherited"),
        "{visible_text}"
    );
    for node_name in [
        "glassworks.viewctl.layout.layer_depth.inherit",
        "glassworks.viewctl.layout.layer_depth.top",
        "glassworks.viewctl.layout.layer_depth.one",
        "glassworks.viewctl.layout.layer_depth.two",
        "glassworks.viewctl.layout.layer_depth.full",
        "glassworks.viewctl.layout.layer_group_depth.inherit",
        "glassworks.viewctl.layout.layer_group_depth.top",
        "glassworks.viewctl.layout.layer_group_depth.one",
        "glassworks.viewctl.layout.layer_group_depth.full",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "layer properties should expose {node_name}"
        );
    }

    let layout_revision = app.layout_revision;
    let view_revision = app.layout_view_revision;
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_depth.top"));
    assert_eq!(app.layout_revision, layout_revision);
    assert_ne!(app.layout_view_revision, view_revision);
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal1),
        Some(&LayoutHierarchyDepth::Top)
    );
    assert_eq!(app.with_layout_display_index(|index| index.len()), 2);
    assert_eq!(layout_shape_browser_entries(&app).len(), 1);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.top"));
    assert_eq!(app.with_layout_display_index(|index| index.len()), 1);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_depth.full"));
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        2,
        "active-layer full override should show deeper active-layer shapes even under global top depth"
    );

    app.sync_app_options_from_state();
    assert_eq!(app.app_options.layout.layer_depths.len(), 1);
    assert_eq!(app.app_options.layout.layer_depths[0].layer, metal1.0);
    assert_eq!(app.app_options.layout.layer_depths[0].max_depth, "full");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_depth.inherit"));
    assert!(!app.layout_layer_depth_overrides.contains_key(&metal1));

    app.layout_hierarchy_depth = LayoutHierarchyDepth::Full;
    app.layout_hierarchy_min_depth = 0;
    app.invalidate_layout_view_caches();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.routing_metal"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group_depth.top"));
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal1),
        Some(&LayoutHierarchyDepth::Top)
    );
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal2),
        Some(&LayoutHierarchyDepth::Top)
    );
    assert!(app.status_message().contains("Routing / Metals"));
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        1,
        "routing-metal group top override should hide child metal shapes"
    );

    app.layout_hierarchy_depth = LayoutHierarchyDepth::Top;
    app.invalidate_layout_view_caches();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group_depth.full"));
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal1),
        Some(&LayoutHierarchyDepth::Full)
    );
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal2),
        Some(&LayoutHierarchyDepth::Full)
    );
    assert_eq!(
        app.with_layout_display_index(|index| index.len()),
        3,
        "routing-metal group full override should show child metal shapes under global top depth"
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group_depth.inherit"));
    assert!(!app.layout_layer_depth_overrides.contains_key(&metal1));
    assert!(!app.layout_layer_depth_overrides.contains_key(&metal2));
}

#[test]
pub(crate) fn layout_layer_sets_save_restore_visibility_and_options() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
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

    assert!(
        app.workspace
            .document
            .layers
            .get(&metal1)
            .expect("metal1 should exist")
            .visible
    );
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.toggle_layer.{}",
        metal2.0
    )));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.routing"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.used"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_depth.top"));
    assert!(
        !app.workspace
            .document
            .layers
            .get(&metal2)
            .expect("metal2 should exist")
            .visible
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.save.2"));
    assert!(
        app.status_message().contains("Saved Layer Set 2"),
        "{}",
        app.status_message()
    );

    let saved_options = app.app_options().clone();
    let saved_set = saved_options
        .layout
        .layer_sets
        .iter()
        .find(|layer_set| layer_set.slot == 2)
        .expect("layer set should persist to app options");
    assert!(saved_set.visible_layers.contains(&metal1.0));
    assert!(!saved_set.visible_layers.contains(&metal2.0));
    assert_eq!(saved_set.layer_group_filter, "routing");
    assert_eq!(saved_set.layer_usage_filter, "used");
    assert_eq!(saved_set.layer_depths.len(), 1);
    assert_eq!(saved_set.layer_depths[0].layer, metal1.0);
    assert_eq!(saved_set.layer_depths[0].max_depth, "top");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::env::temp_dir().join(format!(
            "glassworks-layer-set-export-import-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        assert!(app.export_layout_layer_sets_to_path(&path));
        assert!(
            app.status_message().contains("Exported layer sets"),
            "{}",
            app.status_message()
        );
        let exchange = serde_json::from_str::<LayoutLayerSetExchange>(
            &std::fs::read_to_string(&path).expect("layer set exchange should be readable"),
        )
        .expect("layer set exchange should parse");
        assert_eq!(
            exchange.schema_version,
            LAYOUT_LAYER_SET_EXCHANGE_SCHEMA_VERSION
        );
        assert_eq!(exchange.layer_sets.len(), 1);
        assert_eq!(exchange.layer_sets[0].name, "Layer Set 2");
        assert_eq!(exchange.layer_sets[0].layer_group_filter, "routing");
        assert_eq!(exchange.layer_sets[0].layer_usage_filter, "used");
        assert_eq!(exchange.layer_sets[0].layer_depths.len(), 1);

        app.layout_layer_sets.clear();
        app.sync_app_options_from_state();
        assert!(app.layout_layer_sets.is_empty());
        assert!(app.import_layout_layer_sets_from_path(&path));
        assert!(
            app.status_message().contains("Imported layer sets"),
            "{}",
            app.status_message()
        );
        assert!(app.layout_layer_sets.get(&2).is_some_and(|state| {
            state.visible_layers.contains(&metal1)
                && !state.visible_layers.contains(&metal2)
                && state.layer_group_filter == LayoutLayerGroupFilter::Routing
                && state.layer_usage_filter == LayoutLayerUsageFilter::Used
                && state
                    .layer_depth_overrides
                    .get(&metal1)
                    .is_some_and(|depth| *depth == LayoutHierarchyDepth::Top)
        }));
        let _ = std::fs::remove_file(&path);
    }

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.toggle_layer.{}",
        metal2.0
    )));
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.layout.toggle_layer.{}",
        metal1.0
    )));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_group.all"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_usage_filter.empty"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_depth.inherit"));
    assert!(
        !app.workspace
            .document
            .layers
            .get(&metal1)
            .expect("metal1 should exist")
            .visible
    );
    assert!(
        app.workspace
            .document
            .layers
            .get(&metal2)
            .expect("metal2 should exist")
            .visible
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.restore.2"));
    assert!(
        app.workspace
            .document
            .layers
            .get(&metal1)
            .expect("metal1 should exist")
            .visible
    );
    assert!(
        !app.workspace
            .document
            .layers
            .get(&metal2)
            .expect("metal2 should exist")
            .visible
    );
    assert!(
        app.status_message().contains("Restored Layer Set 2"),
        "{}",
        app.status_message()
    );
    assert_eq!(
        app.layout_layer_group_filter,
        LayoutLayerGroupFilter::Routing
    );
    assert_eq!(app.layout_layer_usage_filter, LayoutLayerUsageFilter::Used);
    assert_eq!(
        app.layout_layer_depth_overrides.get(&metal1),
        Some(&LayoutHierarchyDepth::Top)
    );

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.layer_set.restore.2"),
        "layer panel should expose restore controls"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.layer_set.tab.2"),
        "layer panel should expose layer-set tabs"
    );
    let layer_set_tab_accessibility_label = |document: &UiDocument, node_name: &str| -> String {
        document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .and_then(|node| node.accessibility())
            .and_then(|accessibility| accessibility.label.clone())
            .unwrap_or_else(|| panic!("{node_name} should expose an accessibility label"))
    };
    let saved_visible_layers = app
        .layout_layer_sets
        .get(&2)
        .expect("slot 2 should still be saved")
        .visible_layers
        .len();
    let text_by_node = document_text_by_node(&document);
    let expected_tab_2 = format!("2:{saved_visible_layers}");
    assert_eq!(
        text_by_node
            .get("glassworks.viewctl.layout.layer_set.tab.2.label")
            .map(String::as_str),
        Some(expected_tab_2.as_str()),
        "saved layer-set tabs should show compact visible-layer counts"
    );
    assert_eq!(
        text_by_node
            .get("glassworks.viewctl.layout.layer_set.tab.8.label")
            .map(String::as_str),
        Some("8:-"),
        "empty layer-set tabs should show compact empty status"
    );
    let tab_2_label =
        layer_set_tab_accessibility_label(&document, "glassworks.viewctl.layout.layer_set.tab.2");
    assert!(
        tab_2_label.contains(&format!("Layer Set 2: {saved_visible_layers} visible"))
            && tab_2_label.contains("Routing")
            && tab_2_label.contains("Used rows")
            && tab_2_label.contains("1 depth override"),
        "saved layer-set tab should summarize saved setup details: {tab_2_label}"
    );
    assert_eq!(
        layer_set_tab_accessibility_label(&document, "glassworks.viewctl.layout.layer_set.tab.8"),
        "Layer Set 8: empty slot"
    );
    for node_name in [
        "glassworks.viewctl.layout.layer_set.tab.8",
        "glassworks.viewctl.layout.layer_set.save.8",
        "glassworks.viewctl.layout.layer_set.restore.8",
        "glassworks.viewctl.layout.layer_set.clear.8",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "layer panel should expose extended layer-set slot control {node_name}"
        );
    }
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.layer_set.export"),
        "layer panel should expose layer-set export"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.layer_set.import"),
        "layer panel should expose layer-set import"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout.layer_set.clear_all"),
        "layer panel should expose layer-set clear-all"
    );
    assert!(
        document_visible_text(&document).contains("Layer Sets"),
        "layer panel should label saved layer sets"
    );

    app.set_layout_browser_search("slot=2");
    let layer_set_rows = layout_layer_set_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        layer_set_rows.get("Listed layer sets").map(String::as_str),
        Some("1 / 8 slots")
    );
    assert!(
        layer_set_rows.contains_key("Set 2"),
        "layer-set slot selector search should keep the matching set: {layer_set_rows:?}"
    );
    assert!(
        !layer_set_rows.contains_key("Set 8"),
        "layer-set slot selector search should hide non-matching slots: {layer_set_rows:?}"
    );
    app.set_layout_browser_search("group=routing");
    let layer_set_rows = layout_layer_set_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        layer_set_rows.get("Listed layer sets").map(String::as_str),
        Some("1 / 8 slots")
    );
    assert!(
        layer_set_rows.contains_key("Set 2"),
        "layer-set group selector search should keep the saved routing setup: {layer_set_rows:?}"
    );
    app.set_layout_browser_search(&format!("visible_layer_id={}", metal1.0));
    let layer_set_rows = layout_layer_set_rows(&app)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(
        layer_set_rows.contains_key("Set 2"),
        "layer-set visible-layer selector search should match saved visible layers: {layer_set_rows:?}"
    );
    app.set_layout_browser_search("");

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.clear.2"));
    assert!(!app.layout_layer_sets.contains_key(&2));
    assert_eq!(app.layout_layer_set_name(2), "Layer Set 2");
    assert!(
        app.status_message().contains("Cleared Layer Set 2"),
        "{}",
        app.status_message()
    );
    let cleared_options = app.app_options().clone();
    assert!(
        cleared_options
            .layout
            .layer_sets
            .iter()
            .all(|layer_set| layer_set.slot != 2),
        "cleared layer set should not persist to app options"
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.restore.2"));
    assert!(
        app.status_message().contains("No Layer Set 2 saved"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.save.2"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.save.4"));
    assert_eq!(app.layout_layer_sets.len(), 2);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.clear_all"));
    assert!(app.layout_layer_sets.is_empty());
    assert_eq!(app.layout_layer_set_name(2), "Layer Set 2");
    assert_eq!(app.layout_layer_set_name(4), "Layer Set 4");
    assert!(
        app.app_options().layout.layer_sets.is_empty(),
        "clearing all layer sets should persist to app options"
    );
    assert!(
        app.status_message().contains("Cleared 2 layer sets"),
        "{}",
        app.status_message()
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.clear_all"));
    assert!(
        app.status_message().contains("No layer sets saved"),
        "{}",
        app.status_message()
    );

    assert!(app.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert!(
        !app.workspace
            .document
            .layers
            .get(&metal1)
            .expect("metal1 should exist")
            .visible
    );
    assert!(
        app.workspace
            .document
            .layers
            .get(&metal2)
            .expect("metal2 should exist")
            .visible
    );

    let mut restored_options = saved_options.clone();
    restored_options
        .layout
        .layer_sets
        .iter_mut()
        .find(|layer_set| layer_set.slot == 2)
        .expect("saved layer set should still exist")
        .name = "Routing".to_string();
    let mut restored_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        app_options: Some(restored_options),
        ..Default::default()
    });
    assert_eq!(restored_app.layout_layer_set_name(2), "Routing");
    let restored_document = restored_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("restored layer-set document should build");
    assert!(
        document_visible_text(&restored_document).contains("Routing"),
        "named layer-set tab should appear in the layer panel"
    );
    assert!(
        layer_set_tab_accessibility_label(
            &restored_document,
            "glassworks.viewctl.layout.layer_set.tab.2"
        )
        .starts_with("Routing: "),
        "named layer-set tab should expose its saved setup name"
    );
    assert!(restored_app.layout_layer_sets.get(&2).is_some_and(|state| {
        state.visible_layers.contains(&metal1)
            && !state.visible_layers.contains(&metal2)
            && state.layer_group_filter == LayoutLayerGroupFilter::Routing
            && state.layer_usage_filter == LayoutLayerUsageFilter::Used
            && state
                .layer_depth_overrides
                .get(&metal1)
                .is_some_and(|depth| *depth == LayoutHierarchyDepth::Top)
    }));
    restored_app.layout_layer_group_filter = LayoutLayerGroupFilter::All;
    restored_app.layout_layer_usage_filter = LayoutLayerUsageFilter::Empty;
    restored_app.layout_layer_depth_overrides.clear();
    assert!(restored_app.apply_clicked_node_name("glassworks.viewctl.layout.layer_set.restore.2"));
    assert!(
        !restored_app
            .workspace
            .document
            .layers
            .get(&metal2)
            .expect("metal2 should exist")
            .visible
    );
    assert_eq!(
        restored_app.layout_layer_group_filter,
        LayoutLayerGroupFilter::Routing
    );
    assert_eq!(
        restored_app.layout_layer_usage_filter,
        LayoutLayerUsageFilter::Used
    );
    assert_eq!(
        restored_app.layout_layer_depth_overrides.get(&metal1),
        Some(&LayoutHierarchyDepth::Top)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_saves_and_loads_layout_json() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("layout json menu source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(-200, -100), Point::new(500, 400))),
    )
    .expect("test rectangle should be added");
    let source_top = app.workspace.document.top_cell;
    let source_child = app.workspace.document.create_cell("layout json child");
    app.workspace
        .document
        .insert_shape_in_cell(
            source_child,
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(0, 0), Point::new(300, 200))),
        )
        .expect("source child rectangle should be inserted");
    app.workspace
        .document
        .insert_instance(source_top, source_child, Transform::translate(800, 100))
        .expect("source child should be instanced under source top");
    let expected_shapes = app.workspace.document.flattened_shape_count_estimate();
    let expected_cells = app.workspace.document.cells.len();
    let path = std::env::temp_dir().join(format!(
        "glassworks-layout-menu-save-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.save_layout_json_to_path(&path));
    assert!(
        app.status_message().contains("Saved layout JSON"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("layout save should write JSON");
    let parsed: Document =
        serde_json::from_str(&contents).expect("saved layout should parse as a document");
    assert_eq!(parsed.flattened_shape_count_estimate(), expected_shapes);

    let mut loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    loaded.workspace.document = Document::new("empty before layout load");
    loaded.reset_layout_document_state();
    assert_eq!(
        loaded.workspace.document.flattened_shape_count_estimate(),
        0
    );
    assert!(loaded.load_layout_json_from_path(&path));
    assert_eq!(loaded.active_view, StartupView::Layout2d);
    assert_eq!(
        loaded.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(loaded.workspace.document.cells.len(), expected_cells);
    assert!(
        loaded.status_message().contains("Loaded layout JSON"),
        "{}",
        loaded.status_message()
    );

    let mut imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    imported.workspace.document = Document::new("target before layout import");
    imported.reset_layout_document_state();
    let import_top = imported.workspace.document.top_cell;
    let target_shape = imported
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(2_000, 0), Point::new(2_400, 400))),
        )
        .expect("target rectangle should be added");
    let target_top_shapes_before = imported.workspace.document.shapes.len();
    let target_cells_before = imported.workspace.document.cells.len();
    let target_instances_before = imported
        .workspace
        .document
        .cell(import_top)
        .expect("target top cell should exist")
        .instances
        .len();
    assert!(imported.import_layout_json_as_cell_from_path(&path));
    assert_eq!(imported.active_view, StartupView::Layout2d);
    assert!(
        imported
            .workspace
            .document
            .shapes
            .contains_key(&target_shape),
        "import-as-cell should preserve existing top-level geometry"
    );
    assert_eq!(
        imported.workspace.document.shapes.len(),
        target_top_shapes_before,
        "imported top-level geometry should live inside the imported cell"
    );
    assert_eq!(
        imported.workspace.document.cells.len(),
        target_cells_before + expected_cells
    );
    let top_cell = imported
        .workspace
        .document
        .cell(import_top)
        .expect("target top cell should exist");
    assert_eq!(top_cell.instances.len(), target_instances_before + 1);
    let import_instance = top_cell
        .instances
        .values()
        .find(|instance| {
            imported
                .workspace
                .document
                .cell(instance.cell)
                .is_some_and(|cell| {
                    cell.properties
                        .get("import.source_document")
                        .map(String::as_str)
                        == Some("layout json menu source")
                })
        })
        .expect("import should place the imported root cell");
    let imported_cell = imported
        .workspace
        .document
        .cell(import_instance.cell)
        .expect("imported root cell should exist")
        .clone();
    assert_eq!(imported_cell.shapes.len(), 1);
    assert_eq!(
        imported_cell
            .properties
            .get("import.source_document")
            .map(String::as_str),
        Some("layout json menu source")
    );
    assert!(
        imported
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(|occurrence| occurrence.instance_path == vec![import_instance.id]),
        "import should select a shape occurrence inside the placed imported cell"
    );
    assert!(
        imported.status_message().contains("as cell"),
        "{}",
        imported.status_message()
    );
    assert!(imported.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(imported.workspace.document.cells.len(), target_cells_before);
    assert_eq!(
        imported
            .workspace
            .document
            .cell(import_top)
            .expect("target top cell should exist after undo")
            .instances
            .len(),
        target_instances_before
    );

    let mut top_imported = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    top_imported.workspace.document = Document::new("target before top-cell import");
    top_imported.reset_layout_document_state();
    let original_top = top_imported.workspace.document.top_cell;
    let top_cells_before = top_imported.workspace.document.cells.len();
    let top_instances_before = top_imported
        .workspace
        .document
        .cell(original_top)
        .expect("original top cell should exist")
        .instances
        .len();
    let top_shapes_before = top_imported.workspace.document.shapes.len();
    assert!(top_imported.import_layout_json_as_top_cell_from_path(&path));
    assert_eq!(top_imported.active_view, StartupView::Layout2d);
    assert_eq!(top_imported.workspace.document.top_cell, original_top);
    assert_ne!(top_imported.layout_view_top_cell, original_top);
    assert_eq!(
        top_imported.workspace.document.shapes.len(),
        top_shapes_before
    );
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before + expected_cells
    );
    assert_eq!(
        top_imported
            .workspace
            .document
            .cell(original_top)
            .expect("original top cell should still exist")
            .instances
            .len(),
        top_instances_before
    );
    let imported_root = top_imported
        .workspace
        .document
        .cell(top_imported.layout_view_top_cell)
        .expect("imported top cell should exist");
    assert_eq!(imported_root.shapes.len(), 1);
    assert_eq!(
        imported_root
            .properties
            .get("import.source_document")
            .map(String::as_str),
        Some("layout json menu source")
    );
    assert!(
        top_imported
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(ShapeOccurrenceId::is_top_level),
        "top-cell import should select a local shape in the imported view top cell"
    );
    assert!(
        top_imported.status_message().contains("extra top cell"),
        "{}",
        top_imported.status_message()
    );
    assert!(top_imported.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        top_imported.workspace.document.cells.len(),
        top_cells_before
    );
    assert_eq!(
        top_imported
            .workspace
            .document
            .cell(original_top)
            .expect("original top cell should exist after undo")
            .instances
            .len(),
        top_instances_before
    );
    assert_eq!(top_imported.layout_view_top_cell, original_top);

    let mut merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    merged.workspace.document = Document::new("target before layout merge");
    merged.reset_layout_document_state();
    let merge_top = merged.workspace.document.top_cell;
    let merge_target_shape = merged
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(4_000, 0), Point::new(4_400, 400))),
        )
        .expect("merge target rectangle should be added");
    let merge_top_shapes_before = merged.workspace.document.shapes.len();
    let merge_cells_before = merged.workspace.document.cells.len();
    let merge_instances_before = merged
        .workspace
        .document
        .cell(merge_top)
        .expect("merge top cell should exist")
        .instances
        .len();
    assert!(merged.merge_layout_json_from_path(&path));
    assert_eq!(merged.active_view, StartupView::Layout2d);
    assert!(
        merged
            .workspace
            .document
            .shapes
            .contains_key(&merge_target_shape),
        "merge should preserve existing top-level geometry"
    );
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before + expected_shapes,
        "merge should add flattened imported geometry to the top level"
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);
    assert_eq!(
        merged
            .workspace
            .document
            .cell(merge_top)
            .expect("merge top cell should exist")
            .instances
            .len(),
        merge_instances_before
    );
    assert!(
        merged
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(ShapeOccurrenceId::is_top_level),
        "merge should select a newly merged top-level shape"
    );
    assert!(
        merged.status_message().contains("Merged layout JSON"),
        "{}",
        merged.status_message()
    );
    assert!(merged.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        merged.workspace.document.shapes.len(),
        merge_top_shapes_before
    );
    assert_eq!(merged.workspace.document.cells.len(), merge_cells_before);

    let mut hierarchy_merged = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    hierarchy_merged.workspace.document = Document::new("target before hierarchy merge");
    hierarchy_merged.reset_layout_document_state();
    let hierarchy_merge_top = hierarchy_merged.workspace.document.top_cell;
    let hierarchy_target_shape = hierarchy_merged
        .add_layout_shape(
            metal1,
            ShapeKind::Rectangle(Rect::new(Point::new(6_000, 0), Point::new(6_400, 400))),
        )
        .expect("hierarchy merge target rectangle should be added");
    let hierarchy_flattened_before = hierarchy_merged
        .workspace
        .document
        .flattened_shape_count_estimate();
    let hierarchy_top_shapes_before = hierarchy_merged.workspace.document.shapes.len();
    let hierarchy_cells_before = hierarchy_merged.workspace.document.cells.len();
    let hierarchy_instances_before = hierarchy_merged
        .workspace
        .document
        .cell(hierarchy_merge_top)
        .expect("hierarchy merge top cell should exist")
        .instances
        .len();
    assert!(hierarchy_merged.merge_layout_json_hierarchy_from_path(&path));
    assert_eq!(hierarchy_merged.active_view, StartupView::Layout2d);
    assert!(
        hierarchy_merged
            .workspace
            .document
            .shapes
            .contains_key(&hierarchy_target_shape),
        "hierarchy merge should preserve existing top-level geometry"
    );
    assert_eq!(
        hierarchy_merged.workspace.document.shapes.len(),
        hierarchy_top_shapes_before + 1,
        "hierarchy merge should splice only source-root local shapes into the top level"
    );
    assert_eq!(
        hierarchy_merged.workspace.document.cells.len(),
        hierarchy_cells_before + expected_cells - 1,
        "hierarchy merge should preserve source child cells without adding a wrapper root cell"
    );
    let hierarchy_top_cell = hierarchy_merged
        .workspace
        .document
        .cell(hierarchy_merge_top)
        .expect("hierarchy merge top cell should exist");
    assert_eq!(
        hierarchy_top_cell.instances.len(),
        hierarchy_instances_before + 1,
        "hierarchy merge should splice source-root child instances into the target top cell"
    );
    let hierarchy_instance = hierarchy_top_cell
        .instances
        .values()
        .find(|instance| {
            hierarchy_merged
                .workspace
                .document
                .cell(instance.cell)
                .is_some_and(|cell| {
                    cell.properties
                        .get("import.source_cell")
                        .map(String::as_str)
                        == Some("layout json child")
                })
        })
        .expect("hierarchy merge should preserve the imported child instance");
    assert_eq!(hierarchy_instance.transform, Transform::translate(800, 100));
    assert_eq!(
        hierarchy_merged
            .workspace
            .document
            .flattened_shape_count_estimate(),
        hierarchy_flattened_before + expected_shapes
    );
    assert!(
        hierarchy_merged
            .selected_layout_occurrence
            .as_ref()
            .is_some_and(ShapeOccurrenceId::is_top_level),
        "hierarchy merge should select a newly merged source-root local shape"
    );
    assert!(
        hierarchy_merged
            .status_message()
            .contains("hierarchy into top cell"),
        "{}",
        hierarchy_merged.status_message()
    );
    assert!(hierarchy_merged.apply_clicked_node_name("glassworks.menu.item.edit.undo"));
    assert_eq!(
        hierarchy_merged.workspace.document.shapes.len(),
        hierarchy_top_shapes_before
    );
    assert_eq!(
        hierarchy_merged.workspace.document.cells.len(),
        hierarchy_cells_before
    );
    assert_eq!(
        hierarchy_merged
            .workspace
            .document
            .cell(hierarchy_merge_top)
            .expect("hierarchy merge top cell should exist after undo")
            .instances
            .len(),
        hierarchy_instances_before
    );

    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.save_layout",
        "glassworks.menu.item.file.load_layout",
        "glassworks.menu.item.file.import_layout_cell",
        "glassworks.menu.item.file.import_layout_top_cell",
        "glassworks.menu.item.file.merge_layout",
        "glassworks.menu.item.file.merge_layout_hierarchy",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_saves_and_loads_workspace_json() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("workspace menu source");
    app.reset_layout_document_state();
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    app.add_layout_shape(
        metal1,
        ShapeKind::Rectangle(Rect::new(Point::new(100, 200), Point::new(900, 700))),
    )
    .expect("test rectangle should be added");
    let expected_shapes = app.workspace.document.flattened_shape_count_estimate();
    let expected_cells = app.workspace.document.cells.len();
    let path = std::env::temp_dir().join(format!(
        "glassworks-workspace-menu-save-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.save_workspace_session_to_path(&path));
    assert!(
        app.status_message().contains("Saved workspace"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("workspace save should write JSON");
    let parsed =
        WorkspaceDataset::from_json_str(&contents).expect("saved workspace should reparse");
    assert_eq!(parsed.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION);
    assert_eq!(
        parsed.metadata.workspace_schema_version,
        WORKSPACE_DATASET_SCHEMA_VERSION
    );
    assert_eq!(
        parsed.document.flattened_shape_count_estimate(),
        expected_shapes
    );

    let mut loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    loaded.workspace.document = Document::new("empty before workspace load");
    loaded.reset_layout_document_state();
    assert_eq!(
        loaded.workspace.document.flattened_shape_count_estimate(),
        0
    );
    assert!(loaded.load_workspace_session_from_path(&path));
    assert_eq!(
        loaded.workspace.document.flattened_shape_count_estimate(),
        expected_shapes
    );
    assert_eq!(loaded.workspace.document.cells.len(), expected_cells);
    assert!(
        loaded.status_message().contains("Loaded workspace"),
        "{}",
        loaded.status_message()
    );
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.save",
        "glassworks.menu.item.file.load",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_file_menu_saves_and_loads_app_session() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::hierarchy_demo();
    app.workspace.document.name = "session source".to_string();
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
    .expect("narrow session DRC rectangle should be added");
    assert!(app.run_drc_summary().contains("violation"));
    let marker = layout_drc_marker_entries(&app)
        .first()
        .cloned()
        .expect("session source should have a DRC marker");
    assert!(app.select_layout_drc_marker(&marker.id.to_string()));
    assert!(app.set_selected_layout_drc_marker_tag("fix"));
    let selected_marker_key = app
        .layout_selected_drc_marker_key
        .clone()
        .expect("session source marker should be selected");
    let child_cell = app
        .workspace
        .document
        .cells
        .keys()
        .copied()
        .find(|cell| *cell != app.workspace.document.top_cell)
        .expect("hierarchy demo should include child cells");
    app.layout_view_top_cell = child_cell;
    app.layout_zoom = 0.125;
    app.layout_pan = [123.0, -456.0];
    app.layout_hierarchy_depth = LayoutHierarchyDepth::Top;
    app.show_grid = false;
    app.show_inspector = true;
    app.show_reference_images = false;
    app.active_tool = ToolMode::Path;
    app.sync_app_options_from_state();

    let path = std::env::temp_dir().join(format!(
        "glassworks-app-session-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    assert!(app.save_app_session_to_path(&path));
    assert!(
        app.status_message().contains("Saved session"),
        "{}",
        app.status_message()
    );
    let contents = std::fs::read_to_string(&path).expect("session save should write JSON");
    let exchange: AppSessionExchange =
        serde_json::from_str(&contents).expect("session JSON should parse");
    assert_eq!(exchange.schema_version, APP_SESSION_EXCHANGE_SCHEMA_VERSION);
    assert_eq!(exchange.workspace.document.name, "session source");
    assert_eq!(exchange.app_options.shell.startup_view, "layout2d");
    assert!(!exchange.app_options.layout.show_2d_grid);
    assert_eq!(exchange.app_options.layout.default_tool, "path");
    assert_eq!(exchange.current_layout_view.top_cell, child_cell.0);
    assert_eq!(exchange.drc_reports.len(), 1);
    assert_eq!(exchange.drc_active_report_index, Some(0));
    assert!(!exchange.drc_reports[0].violations.is_empty());

    let mut loaded = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Workflow),
        ..Default::default()
    });
    loaded.workspace.document = Document::new("mutated before session load");
    loaded.reset_layout_document_state();
    loaded.show_grid = true;
    loaded.show_inspector = false;
    loaded.show_reference_images = true;
    assert!(loaded.load_app_session_from_path(&path));
    assert_eq!(loaded.workspace.document.name, "session source");
    assert_eq!(loaded.active_view, StartupView::Layout2d);
    assert_eq!(loaded.active_tool, ToolMode::Path);
    assert!(!loaded.show_grid);
    assert!(loaded.show_inspector);
    assert!(!loaded.show_reference_images);
    assert_eq!(loaded.layout_view_top_cell, child_cell);
    assert_eq!(loaded.layout_hierarchy_depth, LayoutHierarchyDepth::Top);
    assert!((loaded.layout_zoom - 0.125).abs() < f32::EPSILON);
    assert_eq!(loaded.layout_pan, [123.0, -456.0]);
    assert_eq!(loaded.layout_drc_report_history.len(), 1);
    assert!(loaded.layout_selected_drc_report_id.is_some());
    assert!(
        loaded
            .drc_report()
            .is_some_and(|report| !report.violations.is_empty()),
        "session load should restore the active DRC report"
    );
    assert_eq!(
        loaded
            .workspace
            .document
            .marker_states
            .get(&selected_marker_key)
            .and_then(|state| state.tags.get("action"))
            .map(String::as_str),
        Some("fix"),
        "session load should keep DRC marker review metadata"
    );
    assert_eq!(loaded.app_options.files.recent_files[0].kind, "session");
    assert!(
        loaded.status_message().contains("Loaded session"),
        "{}",
        loaded.status_message()
    );
    let _ = std::fs::remove_file(&path);

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("file menu should build");
    for node_name in [
        "glassworks.menu.item.file.save_session",
        "glassworks.menu.item.file.load_session",
    ] {
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("File menu should expose {node_name}"));
        assert!(node.input().pointer, "{node_name} should be enabled");
    }
}
