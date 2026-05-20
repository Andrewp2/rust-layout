#![allow(unused_imports)]
use super::*;
use crate::*;

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_calibre_rve_import_populates_marker_browser() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        zoom: Some(0.1),
        ..Default::default()
    });
    app.workspace.document = Document::new("calibre rve import target");
    app.reset_layout_document_state();
    let path = std::env::temp_dir().join(format!(
        "glassworks-calibre-rve-menu-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        "M1_WIDTH
@ minimum width from calibre
@ owner: external-rve
@ note: external review note
@ approval_status: accepted
@ approved_by: external-rve
@ approval_note: external approval note
@ approval_role: foundry
@ approval_at: 2026-05-19T12:20:00Z
@ tag category=Litho/Hotspots
p 4 100 200 400 200 400 360 100 360
M2_SPACE
r 800 900 980 1040
BROKEN_POLYGON
p 4
0 0
100 0
",
    )
    .expect("Calibre/RVE fixture should be written");

    assert!(app.import_layout_calibre_rve_markers_from_path(&path));
    assert!(
        app.status_message()
            .contains("Imported Calibre/RVE markers"),
        "{}",
        app.status_message()
    );
    assert!(
        app.status_message()
            .contains("incomplete polygon marker geometry"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.files.recent_files[0].kind, "calibre_rve");
    let report = app
        .drc_report()
        .expect("Calibre/RVE import should populate the marker cache");
    assert!(
        report.findings.iter().any(|finding| {
            finding.severity == DrcValidationSeverity::Warning
                && finding
                    .message
                    .contains("incomplete polygon marker geometry")
        }),
        "Calibre/RVE parser warnings should be stored with the imported report"
    );
    assert_eq!(report.violations.len(), 2);
    assert_eq!(report.violations[0].rule, "calibre.m1_width");
    assert_eq!(report.violations[0].shape_ids.len(), 0);
    assert_eq!(
        report.violations[0].bounds,
        Rect::new(Point::new(100, 200), Point::new(400, 360))
    );
    let first_key = report.violations[0].stable_key();
    assert!(
        app.workspace
            .document
            .marker_states
            .get(&first_key)
            .is_some_and(|state| {
                state.owner.as_deref() == Some("external-rve")
                    && state.note.as_deref() == Some("external review note")
                    && state.signoff.as_deref() == Some("accepted")
                    && state.signoff_by.as_deref() == Some("external-rve")
                    && state.signoff_note.as_deref() == Some("external approval note")
                    && state
                        .signoff_records
                        .get("external-rve")
                        .is_some_and(|signoff| {
                            signoff.status == "accepted"
                                && signoff.role.as_deref() == Some("foundry")
                                && signoff.by.as_deref() == Some("external-rve")
                                && signoff.note.as_deref() == Some("external approval note")
                                && signoff.recorded_at.as_deref() == Some("2026-05-19T12:20:00Z")
                        })
                    && state.tags.get("category").map(String::as_str) == Some("Litho/Hotspots")
            })
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report diagnostics" && value == "1 warning"),
        "Calibre/RVE parser warnings should be summarized without hiding markers: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Report warning 1" && value.contains("incomplete polygon marker geometry")
        }),
        "Calibre/RVE parser warning text should stay visible in marker info rows: {info_rows:?}"
    );
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report markers" && value == "2"),
        "Calibre/RVE report diagnostics should not suppress marker summaries: {info_rows:?}"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Report diagnostics" && value == "1 warning"),
        "Calibre/RVE parser warnings should be visible in DRC marker rows: {marker_rows:?}"
    );
    assert!(
        !layout_drc_marker_category_entries(&app).is_empty(),
        "Calibre/RVE warning diagnostics should not suppress marker category rows"
    );
    assert!(
        layout_drc_marker_directory_entries(&app)
            .iter()
            .any(|entry| entry.path.contains("Litho")),
        "Calibre/RVE warning diagnostics should not suppress marker directory rows"
    );
    let entries = layout_drc_marker_entries(&app);
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .any(|entry| entry.label.contains("calibre.m1_width")),
        "marker browser should list imported Calibre rule names"
    );
    assert!(app.select_layout_drc_marker(&entries[0].id.to_string()));
    assert!(app.layout_selected_drc_marker_key.is_some());
    assert!(
        app.status_message().contains("minimum width from calibre"),
        "{}",
        app.status_message()
    );
    app.set_layout_browser_search("signoff_by=external-rve");
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == first_key),
        "Calibre/RVE review metadata should be searchable in the marker browser"
    );
    app.set_layout_browser_search("signoff_party=external-rve");
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == first_key),
        "Calibre/RVE imported signoff records should be searchable in the marker browser"
    );
    app.set_layout_browser_search("signoff_role=foundry");
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == first_key),
        "Calibre/RVE imported signoff roles should be searchable in the marker browser"
    );
    app.set_layout_browser_search("signoff_at=2026-05-19");
    assert!(
        layout_drc_marker_entries(&app)
            .iter()
            .any(|entry| entry.key == first_key),
        "Calibre/RVE imported signoff timestamps should be searchable in the marker browser"
    );
    app.set_layout_browser_search("");
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document with imported Calibre markers should build");
    for node_name in [
        "glassworks.viewctl.layout.calibre_rve_import".to_string(),
        format!("glassworks.viewctl.layout.drc_marker.{}", entries[0].id),
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "DRC marker browser should expose {node_name}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_calibre_rve_import_preserves_warning_only_report() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    app.workspace.document = Document::new("calibre rve warning target");
    app.reset_layout_document_state();
    let path = std::env::temp_dir().join(format!(
        "glassworks-calibre-rve-warning-only-{}.txt",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        "BROKEN_ONLY
p 4
0 0
100 0
",
    )
    .expect("Calibre/RVE warning-only fixture should be written");

    assert!(app.import_layout_calibre_rve_markers_from_path(&path));
    assert!(
        app.status_message().contains("found no markers")
            && app.status_message().contains("1 diagnostic")
            && app
                .status_message()
                .contains("incomplete polygon marker geometry"),
        "{}",
        app.status_message()
    );
    assert_eq!(app.app_options.files.recent_files[0].kind, "calibre_rve");
    let report = app
        .drc_report()
        .expect("Calibre/RVE warning-only import should preserve report diagnostics");
    assert!(report.violations.is_empty());
    assert!(
        report.findings.iter().any(|finding| {
            finding.severity == DrcValidationSeverity::Warning
                && finding
                    .message
                    .contains("incomplete polygon marker geometry")
        }),
        "warning-only Calibre/RVE imports should store parser diagnostics"
    );
    let info_rows = layout_drc_marker_info_rows(&app);
    assert!(
        info_rows
            .iter()
            .any(|(key, value)| key == "Report diagnostics" && value == "1 warning"),
        "warning-only Calibre/RVE diagnostics should be visible in marker info rows: {info_rows:?}"
    );
    assert!(
        info_rows.iter().any(|(key, value)| {
            key == "Report warning 1" && value.contains("incomplete polygon marker geometry")
        }),
        "warning-only Calibre/RVE warning text should be visible in marker info rows: {info_rows:?}"
    );
    let marker_rows = layout_drc_marker_rows(&app);
    assert!(
        marker_rows
            .iter()
            .any(|(key, value)| key == "Report diagnostics" && value == "1 warning"),
        "warning-only Calibre/RVE diagnostics should be visible in marker rows: {marker_rows:?}"
    );
    let report_id = app
        .layout_selected_drc_report_id
        .expect("warning-only Calibre/RVE import should select its diagnostic report");
    for query in [
        "incomplete polygon marker geometry",
        "diagnostic=incomplete polygon",
        "warning=incomplete polygon",
        "severity=warning",
    ] {
        app.layout_browser_search = query.to_string();
        assert_eq!(
            layout_drc_report_browser_entries(&app)
                .into_iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec![report_id],
            "warning-only Calibre/RVE diagnostic report should be searchable by {query:?}"
        );
    }
    assert!(layout_drc_marker_entries(&app).is_empty());
    assert!(layout_drc_marker_category_entries(&app).is_empty());
    assert!(layout_drc_marker_directory_entries(&app).is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
pub(crate) fn ui_document_nodes_are_clickable() {
    let app = GlassworksApp::new_with_options(StartupOptions::default());
    let mut document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("UI document should build");

    assert_clicked_node(&mut document, "glassworks.menu.file");
    assert_clicked_node(&mut document, "glassworks.nav.action.metrology");
}

#[test]
pub(crate) fn nav_rail_buttons_center_single_line_text() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Safety),
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("UI document should build");
    let nav_button = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.nav.action.safety")
        .expect("safety nav button should exist");
    assert!(nav_button.input().pointer);
    assert_eq!(
        nav_button.action().and_then(|action| action.action_id()),
        Some(&operad::WidgetActionId::new("glassworks.nav.action.safety"))
    );
    let accessibility = nav_button
        .accessibility()
        .expect("nav button should expose accessibility metadata");
    assert_eq!(accessibility.role, AccessibilityRole::Button);
    assert_eq!(
        accessibility.label.as_deref(),
        Some("Safety and Interlock Dashboard")
    );
    let nav_label = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.nav.action.safety.label")
        .expect("safety nav button should use the Operad button label node");
    let UiContent::Text(text) = nav_label.content() else {
        panic!("nav button label should be text");
    };
    assert_eq!(text.style.wrap, TextWrap::None);
    assert!(
        nav_label.layout().rect.y >= nav_button.layout().rect.y
            && nav_label.layout().rect.bottom() <= nav_button.layout().rect.bottom(),
        "label should fit vertically inside button: label {:?}, button {:?}",
        nav_label.layout().rect,
        nav_button.layout().rect
    );
}

#[test]
pub(crate) fn nav_rail_node_ids_stay_stable_between_views() {
    let base_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let base_document = base_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout editor document should build");

    for active_view in StartupView::ALL {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(active_view),
            ..Default::default()
        });
        let document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .unwrap_or_else(|error| panic!("{active_view:?} document should build: {error}"));
        for nav_view in StartupView::ALL {
            let name = format!("glassworks.nav.action.{}", nav_view.slug());
            let base_id = node_id(&base_document, &name);
            let active_id = node_id(&document, &name);
            assert_eq!(
                base_id, active_id,
                "{name} should keep the same node id in {active_view:?}"
            );
        }
    }
}

#[test]
pub(crate) fn all_view_pointer_controls_have_handlers() {
    for view in StartupView::ALL {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        let document = app
            .build_operad_document(UiSize::new(1280.0, 720.0))
            .unwrap_or_else(|error| panic!("{view:?} document should build: {error}"));
        let control_names = document
            .nodes()
            .iter()
            .filter(|node| node.input().pointer)
            .filter(|node| !matches!(node.content(), UiContent::Canvas(_)))
            .map(|node| node.name().to_string())
            .filter(|name| name.starts_with("glassworks."))
            .collect::<Vec<_>>();
        assert!(
            !control_names.is_empty(),
            "{view:?} should expose pointer controls"
        );

        for name in control_names {
            let mut app = GlassworksApp::new_with_options(StartupOptions {
                view_mode: Some(view),
                ..Default::default()
            });
            assert!(
                app.apply_clicked_node_name(&name),
                "{view:?} pointer control has no handler: {name}"
            );
        }
    }
}

#[test]
pub(crate) fn open_menu_pointer_items_have_handlers() {
    fn enabled_menu_item_names(document: &UiDocument) -> Vec<String> {
        document
            .nodes()
            .iter()
            .filter(|node| node.input().pointer)
            .map(|node| node.name().to_string())
            .filter(|name| name.starts_with("glassworks.menu.item."))
            .collect()
    }

    fn expensive_menu_item(name: &str) -> bool {
        matches!(
            name.strip_prefix("glassworks.menu.item."),
            Some("macros.stress10k" | "macros.stress100k" | "macros.stress1m")
        )
    }

    for view in [
        StartupView::Workflow,
        StartupView::Layout2d,
        StartupView::Layout3d,
        StartupView::Metrology,
    ] {
        for menu in AppMenu::ALL {
            let mut app = GlassworksApp::new_with_options(StartupOptions {
                view_mode: Some(view),
                ..Default::default()
            });
            assert!(
                app.apply_clicked_node_name(&format!("glassworks.menu.{}", menu.slug())),
                "{view:?} should open {menu:?} menu"
            );
            let document = app
                .build_operad_document(UiSize::new(1280.0, 720.0))
                .unwrap_or_else(|error| panic!("{view:?} {menu:?} menu should build: {error}"));
            for name in enabled_menu_item_names(&document) {
                if expensive_menu_item(&name) {
                    continue;
                }
                let mut app = GlassworksApp::new_with_options(StartupOptions {
                    view_mode: Some(view),
                    ..Default::default()
                });
                assert!(
                    app.apply_clicked_node_name(&name),
                    "{view:?} {menu:?} menu item has no handler: {name}"
                );
            }
        }
    }

    for group in ModuleGroup::ALL {
        let mut app = GlassworksApp::new_with_options(StartupOptions::default());
        assert!(app.apply_clicked_node_name("glassworks.menu.view"));
        assert!(
            app.apply_clicked_node_name(&format!(
                "glassworks.menu.item.view.group.{}",
                group.slug()
            ))
        );
        let document = app
            .build_operad_document(UiSize::new(1280.0, 720.0))
            .unwrap_or_else(|error| panic!("View {group:?} submenu should build: {error}"));
        for name in enabled_menu_item_names(&document) {
            let mut app = GlassworksApp::new_with_options(StartupOptions::default());
            assert!(
                app.apply_clicked_node_name(&name),
                "View {group:?} submenu item has no handler: {name}"
            );
        }
    }

    let mut app = GlassworksApp::new_with_options(StartupOptions::default());
    assert!(app.apply_clicked_node_name("glassworks.menu.more"));
    let document = app
        .build_operad_document(UiSize::new(640.0, 720.0))
        .expect("compact More menu should build");
    for name in enabled_menu_item_names(&document) {
        let mut app = GlassworksApp::new_with_options(StartupOptions::default());
        assert!(
            app.apply_clicked_node_name(&name),
            "compact More menu item has no handler: {name}"
        );
    }
}

#[test]
pub(crate) fn primary_view_panels_show_domain_rows() {
    for (view, required_nodes) in [
        (
            StartupView::Workflow,
            &[
                "glassworks.workflow.dashboard",
                "glassworks.workflow.lots",
                "glassworks.workflow.focus",
                "glassworks.workflow.spine",
                "glassworks.workflow.cross_links",
                "glassworks.workflow.lots.row.0",
            ][..],
        ),
        (
            StartupView::MaskPrep,
            &[
                "glassworks.mask.overview",
                "glassworks.mask.reticle",
                "glassworks.mask.fields",
                "glassworks.mask.layers",
                "glassworks.mask.checks",
                "glassworks.mask.reticle.row.0",
            ][..],
        ),
        (
            StartupView::LayoutDiff,
            &[
                "glassworks.layout_diff.overview",
                "glassworks.layout_diff.layers",
                "glassworks.layout_diff.changes",
                "glassworks.layout_diff.layers.row.0",
            ][..],
        ),
        (
            StartupView::FabControl,
            &[
                "glassworks.fab_control.overview",
                "glassworks.fab_control.tools",
                "glassworks.fab_control.detail",
                "glassworks.fab_control.tools.row.0",
            ][..],
        ),
        (
            StartupView::Traceability,
            &[
                "glassworks.traceability.overview",
                "glassworks.traceability.lots",
                "glassworks.traceability.wafers",
                "glassworks.traceability.detail",
                "glassworks.traceability.lots.row.0",
            ][..],
        ),
    ] {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        let document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("domain primary panel should build");
        assert!(
            document
                .nodes()
                .iter()
                .any(|node| node.name() == "glassworks.primary"),
            "view {view:?} should include a primary panel"
        );
        for node_name in required_nodes {
            assert!(
                document
                    .nodes()
                    .iter()
                    .any(|node| node.name() == *node_name),
                "view {view:?} should include {node_name}"
            );
        }
    }

    for (view, duplicate_control_section) in [
        (StartupView::Workflow, "glassworks.workflow.actions"),
        (StartupView::MaskPrep, "glassworks.mask.controls"),
        (StartupView::LayoutDiff, "glassworks.layout_diff.controls"),
        (StartupView::FabControl, "glassworks.fab_control.commands"),
        (
            StartupView::Traceability,
            "glassworks.traceability.controls",
        ),
    ] {
        let document = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        })
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("domain primary panel should build");
        assert!(
            !document
                .nodes()
                .iter()
                .any(|node| node.name() == duplicate_control_section),
            "view {view:?} should not duplicate top controls in {duplicate_control_section}"
        );
    }
    for (view, duplicate_header_prefix) in [
        (StartupView::Workflow, "Actions -"),
        (StartupView::MaskPrep, "Reticle Prep Controls -"),
        (StartupView::LayoutDiff, "Diff Controls -"),
        (StartupView::FabControl, "Tool Commands -"),
    ] {
        let document = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        })
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("domain primary panel should build");
        let visible_text = document_visible_text(&document);
        assert!(
            !visible_text.contains(duplicate_header_prefix),
            "view {view:?} should not show duplicate control header {duplicate_header_prefix:?}\n{visible_text}"
        );
    }

    let maintenance_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Maintenance),
        ..Default::default()
    });
    let maintenance_document = maintenance_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("maintenance primary panel should build");
    for node_name in [
        "glassworks.maintenance.overview",
        "glassworks.maintenance.summary",
        "glassworks.maintenance.queue",
        "glassworks.maintenance.detail",
        "glassworks.maintenance.audit",
    ] {
        assert!(
            maintenance_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "maintenance should include {node_name}"
        );
    }
    assert!(
        !maintenance_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.maintenance.filters"),
        "maintenance should not duplicate top filter controls inside the body"
    );

    let inventory_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Inventory),
        ..Default::default()
    });
    let inventory_document = inventory_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("inventory primary panel should build");
    for node_name in [
        "glassworks.inventory.overview",
        "glassworks.inventory.summary",
        "glassworks.inventory.alerts",
        "glassworks.inventory.lots",
        "glassworks.inventory.detail",
    ] {
        assert!(
            inventory_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "inventory should include {node_name}"
        );
    }
    assert!(
        !inventory_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.inventory.filters"),
        "inventory should not duplicate top filter controls inside the body"
    );

    let safety_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Safety),
        ..Default::default()
    });
    let safety_document = safety_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("safety primary panel should build");
    for node_name in [
        "glassworks.safety.overview",
        "glassworks.safety.summary",
        "glassworks.safety.interlocks",
        "glassworks.safety.sensors",
        "glassworks.safety.tools",
        "glassworks.safety.incidents",
    ] {
        assert!(
            safety_document
                .nodes()
                .iter()
                .any(|node| node.name() == node_name),
            "safety should include {node_name}"
        );
    }

    let layout_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let layout_document = layout_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout primary scene should build");
    assert!(
        layout_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.preview")
    );
    let layout_preview = layout_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.preview")
        .expect("layout preview should exist");
    match layout_preview.content() {
        UiContent::Canvas(canvas) => {
            assert_eq!(canvas.key, "glassworks.layout.viewport.2d");
            assert_eq!(
                canvas.render_mode,
                operad::CanvasRenderMode::AttachedContext
            );
            assert!(canvas.context.kind.is_gpu_backed());
            assert_eq!(canvas.interaction, CanvasInteractionPolicy::EDITOR);
        }
        other => panic!("layout preview should be a canvas, got {other:?}"),
    }
    assert!(
        layout_preview.layout().rect.height >= 300.0,
        "{:?}",
        layout_preview.layout().rect
    );
    let layout_canvas_requests = RenderFrameRequest::new(
        RenderTarget::window("test", UiSize::new(1440.0, 920.0)),
        UiSize::new(1440.0, 920.0),
        layout_document.paint_list(),
    )
    .canvas_requests();
    assert!(
        layout_canvas_requests
            .iter()
            .any(|request| request.canvas.key == "glassworks.layout.viewport.2d"),
        "layout editor should publish a canvas render request"
    );
    assert!(
        !layout_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.layout"),
        "layout controls belong in the tool strip and side panels, not below the canvas"
    );

    let layout_3d_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    let layout_3d_document = layout_3d_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("3D layout primary canvas should build");
    let layout_3d_requests = RenderFrameRequest::new(
        RenderTarget::window("test", UiSize::new(1440.0, 920.0)),
        UiSize::new(1440.0, 920.0),
        layout_3d_document.paint_list(),
    )
    .canvas_requests();
    assert!(
        layout_3d_requests
            .iter()
            .any(|request| request.canvas.key == "glassworks.layout.viewport.3d"),
        "3D layout editor should publish a canvas render request"
    );
    let layout_3d_canvas = layout_3d_requests
        .iter()
        .find(|request| request.canvas.key == "glassworks.layout.viewport.3d")
        .expect("3D layout canvas request should exist");
    assert_eq!(
        layout_3d_canvas.canvas.render_mode,
        operad::CanvasRenderMode::AttachedContext
    );
    assert!(layout_3d_canvas.canvas.context.kind.is_gpu_backed());

    let cross_section_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::CrossSection),
        ..Default::default()
    });
    let cross_section_document = cross_section_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("cross-section scene should build");
    assert!(
        cross_section_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.cross_section.preview")
    );

    let process_flow_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::ProcessFlow),
        ..Default::default()
    });
    let process_flow_document = process_flow_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("process-flow timeline should build");
    assert!(
        process_flow_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.process_flow.timeline")
    );

    let scheduler_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Scheduler),
        ..Default::default()
    });
    let scheduler_document = scheduler_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("scheduler timeline should build");
    assert!(
        scheduler_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.scheduler.timeline")
    );

    let environment_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Environment),
        ..Default::default()
    });
    let environment_document = environment_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("environment trend scene should build");
    assert!(
        environment_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.environment.trend")
    );

    let metrology_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Metrology),
        ..Default::default()
    });
    let metrology_document = metrology_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("metrology wafer-map scene should build");
    assert!(
        metrology_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.metrology.wafer_map")
    );

    let yield_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Yield),
        ..Default::default()
    });
    let yield_document = yield_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("yield wafer-map scene should build");
    assert!(
        yield_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.yield.wafer_map")
    );

    let experiment_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Experiment),
        ..Default::default()
    });
    let experiment_document = experiment_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("experiment matrix scene should build");
    assert!(
        experiment_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.experiment.matrix")
    );

    let process_control_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::ProcessControl),
        ..Default::default()
    });
    let process_control_document = process_control_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("process-control trend scene should build");
    assert!(
        process_control_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.process_control.trend")
    );

    let spc_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::SpcFdc),
        ..Default::default()
    });
    let spc_document = spc_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("SPC/FDC chart scene should build");
    assert!(
        spc_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.spc.chart")
    );

    let notebook_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Notebook),
        ..Default::default()
    });
    let notebook_document = notebook_app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("notebook selected-entry preview should build");
    assert!(
        notebook_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.notebook.body")
    );
}

#[test]
pub(crate) fn ui_document_scales_for_hidpi_windows() {
    let app = GlassworksApp::new_with_options(StartupOptions::default());
    let document = app
        .build_operad_document_scaled(UiSize::new(2048.0, 1440.0), UiScale::new(2.0))
        .expect("scaled UI document should build");

    assert_eq!(document.audit_layout(), Vec::new());
    let menu = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.file")
        .expect("file menu should exist");
    assert!(
        menu.layout().rect.height >= 40.0,
        "{:?}",
        menu.layout().rect
    );
    let nav = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.nav")
        .expect("nav should exist");
    assert!(nav.layout().rect.width >= 400.0, "{:?}", nav.layout().rect);
}

#[test]
pub(crate) fn narrow_ui_preserves_body_width() {
    let app = GlassworksApp::new_with_options(StartupOptions::default());
    let document = app
        .build_operad_document(UiSize::new(480.0, 900.0))
        .expect("narrow UI document should build");

    assert_eq!(document.audit_layout(), Vec::new());
    let nav = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.nav")
        .expect("nav should exist");
    assert!(nav.layout().rect.width <= 170.0, "{:?}", nav.layout().rect);
    let body_frame = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.body_frame")
        .expect("body frame should exist");
    assert!(
        body_frame.layout().rect.width >= 300.0,
        "{:?}",
        body_frame.layout().rect
    );

    let layout_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let layout_document = layout_app
        .build_operad_document(UiSize::new(480.0, 900.0))
        .expect("narrow layout editor should build");
    assert_eq!(layout_document.audit_layout(), Vec::new());
    let preview = layout_document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.layout.preview")
        .expect("layout preview should exist");
    assert!(
        preview.layout().rect.width >= 280.0,
        "{:?}",
        preview.layout().rect
    );
}

#[test]
pub(crate) fn metrology_view_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Metrology),
        ..Default::default()
    });

    assert!(app.apply_clicked_node_name("glassworks.viewctl.metrology.mode.defects"));
    assert_eq!(app.metrology_map_mode(), MetrologyMapMode::DefectReview);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.metrology.kind.thickness"));
    assert_eq!(app.metrology_kind(), MeasurementKind::ThicknessNm);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.metrology.failed_only"));
    assert!(app.metrology_failed_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.metrology.next_attention"));
    assert!(app.selected_die().is_some());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.metrology.clear_die"));
    assert_eq!(app.selected_die(), None);

    let mut document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("metrology controls should build");
    assert_clicked_node(&mut document, "glassworks.viewctl.metrology.mode.defects");
    assert_clicked_node(&mut document, "glassworks.viewctl.metrology.kind.thickness");
}

#[test]
pub(crate) fn yield_and_experiment_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Yield),
        ..Default::default()
    });
    let lot_id = app
        .workspace
        .yield_analysis
        .lots
        .first()
        .expect("demo yield lot should exist")
        .id
        .clone();
    let wafer_id = app
        .workspace
        .yield_analysis
        .wafer_ids_for_lot(&lot_id)
        .first()
        .expect("demo yield wafer should exist")
        .clone();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.yield.filter.failing"));
    assert_eq!(app.yield_map_filter(), YieldMapFilter::Failing);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.yield.attention"));
    assert!(app.show_only_attention_wafers());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.yield.excursions"));
    assert!(app.show_only_excursions());
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.yield.lot.{lot_id}")));
    assert_eq!(app.selected_yield_lot(), Some(lot_id.as_str()));
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.yield.wafer.{wafer_id}")));
    assert_eq!(app.selected_yield_wafer(), Some(wafer_id.as_str()));

    let mut yield_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("yield controls should build");
    assert_clicked_node(
        &mut yield_document,
        "glassworks.viewctl.yield.filter.failing",
    );

    app.set_active_view(StartupView::Experiment);
    assert!(!app.experiment_show_missing_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.experiment.pending_only"));
    assert!(app.experiment_show_missing_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.experiment.next_pending"));
    assert!(app.status_message().contains("DOE: queued"));

    let response_id = app
        .workspace
        .experiment_plan
        .responses
        .first()
        .expect("demo DOE response should exist")
        .id
        .clone();
    let run_id = app
        .workspace
        .experiment_plan
        .runs
        .first()
        .expect("demo DOE run should exist")
        .id
        .clone();
    let lot_id = app
        .workspace
        .experiment_plan
        .runs
        .first()
        .expect("demo DOE run should exist")
        .assignment
        .lot_id
        .to_string();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.experiment.response.{response_id}"
    )));
    assert_eq!(
        app.selected_experiment_response.as_ref(),
        Some(&response_id)
    );
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.experiment.run.{run_id}")));
    assert_eq!(app.selected_experiment_run.as_ref(), Some(&run_id));
    assert!(
        app.apply_clicked_node_name("glassworks.viewctl.experiment.filter.needs-selected-response")
    );
    assert_eq!(
        app.experiment_run_filter,
        ExperimentRunFilter::NeedsSelectedResponse
    );
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.experiment.lot.{lot_id}")));
    assert_eq!(app.experiment_lot_filter.as_deref(), Some(lot_id.as_str()));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.experiment.use_target"));
    assert!(app.status_message().contains("DOE: loaded target"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.experiment.capture_next_demo"));
    assert!(app.status_message().contains("DOE: captured demo"));

    let mut experiment_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("experiment controls should build");
    assert_clicked_node(
        &mut experiment_document,
        "glassworks.viewctl.experiment.pending_only",
    );
    assert_clicked_node(
        &mut experiment_document,
        &format!("glassworks.viewctl.experiment.response.{response_id}"),
    );
}

#[test]
pub(crate) fn workflow_controls_switch_focus_and_open_views() {
    let mut app = GlassworksApp::new_with_options(StartupOptions::default());
    let initial_focus = app.workflow_focus_lot().map(str::to_string);
    let target_lot = workflow_lot_ids(app.workspace())
        .into_iter()
        .find(|lot_id| Some(lot_id.as_str()) != app.workflow_focus_lot())
        .or(initial_focus)
        .expect("demo workflow should have a lot");

    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.workflow.focus_lot.{target_lot}"
    )));
    assert_eq!(app.workflow_focus_lot(), Some(target_lot.as_str()));

    assert!(app.apply_clicked_node_name("glassworks.viewctl.workflow.open.fab-control"));
    assert_eq!(app.active_view(), StartupView::FabControl);
    for (action, expected_view) in [
        (
            "glassworks.viewctl.workflow.open.maintenance",
            StartupView::Maintenance,
        ),
        (
            "glassworks.viewctl.workflow.open.environment",
            StartupView::Environment,
        ),
        (
            "glassworks.viewctl.workflow.open.safety",
            StartupView::Safety,
        ),
        (
            "glassworks.viewctl.workflow.open.metrology",
            StartupView::Metrology,
        ),
        (
            "glassworks.viewctl.workflow.open.traceability",
            StartupView::Traceability,
        ),
        (
            "glassworks.viewctl.workflow.open.notebook",
            StartupView::Notebook,
        ),
    ] {
        app.set_active_view(StartupView::Workflow);
        assert!(app.apply_clicked_node_name(action), "{action}");
        assert_eq!(app.active_view(), expected_view);
    }

    app.set_active_view(StartupView::Workflow);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.workflow.load_demo"));
    assert!(app.status_message().contains("demo workspace loaded"));
    let mut document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("workflow document should build");
    assert_clicked_node(
        &mut document,
        "glassworks.viewctl.workflow.open.traceability",
    );
    assert_clicked_node(&mut document, "glassworks.viewctl.workflow.load_demo");
}

#[test]
pub(crate) fn fab_control_actions_select_and_command_equipment() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::FabControl),
        ..Default::default()
    });
    let offline_tool = app
        .workspace()
        .equipment
        .tools()
        .find(|tool| tool.state == EquipmentToolState::Offline)
        .expect("demo fab should include an offline tool")
        .id
        .clone();

    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.fab.select.{offline_tool}")));
    assert_eq!(app.selected_equipment_tool(), Some(&offline_tool));
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.fab.command.online|{offline_tool}"
    )));
    assert_eq!(
        app.workspace()
            .equipment
            .tool(&offline_tool)
            .expect("tool should remain present")
            .state,
        EquipmentToolState::OnlineIdle
    );

    let recipe_id = app
        .workspace()
        .equipment
        .tool(&offline_tool)
        .expect("tool should remain present")
        .available_recipes
        .keys()
        .next()
        .expect("tool should have recipes")
        .clone();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.fab.recipe.{offline_tool}|{recipe_id}"
    )));
    assert_eq!(
        app.workspace()
            .equipment
            .tool(&offline_tool)
            .expect("tool should remain present")
            .state,
        EquipmentToolState::RecipeLoaded
    );

    let mut document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("fab control controls should build");
    assert_clicked_node(
        &mut document,
        &format!("glassworks.viewctl.fab.command.start|{offline_tool}"),
    );
}

#[test]
pub(crate) fn maintenance_and_environment_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Maintenance),
        ..Default::default()
    });
    let maintenance_tool = app
        .workspace()
        .maintenance
        .tools
        .first()
        .expect("demo maintenance tool should exist")
        .tool_id
        .clone();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.maintenance.filter.calibration"));
    assert_eq!(
        app.maintenance_work_filter(),
        MaintenanceWorkFilter::Calibration
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.maintenance.history.all"));
    assert_eq!(
        app.maintenance_history_filter(),
        MaintenanceHistoryFilter::AllTools
    );
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.maintenance.tool.{maintenance_tool}"
    )));
    assert_eq!(app.selected_maintenance_tool(), Some(&maintenance_tool));
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.maintenance.status.schedule|{maintenance_tool}"
    )));
    assert!(app.status_message().contains("Maintenance scheduling"));

    let mut maintenance_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("maintenance controls should build");
    assert_clicked_node(
        &mut maintenance_document,
        "glassworks.viewctl.maintenance.filter.calibration",
    );

    app.set_active_view(StartupView::Environment);
    let sensor_id = app
        .workspace()
        .environment
        .sensors
        .first()
        .expect("demo environment sensor should exist")
        .id
        .clone();
    let zone = app
        .workspace()
        .environment
        .sensors
        .first()
        .expect("demo environment sensor should exist")
        .zone
        .clone();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.environment.sensor.{sensor_id}"
    )));
    assert_eq!(app.selected_environment_sensor(), Some(sensor_id.as_str()));
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.environment.zone.{zone}")));
    assert!(
        app.selected_environment_sensor().is_some(),
        "zone selection should choose a sensor"
    );

    let mut environment_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("environment controls should build");
    assert_clicked_node(
        &mut environment_document,
        &format!("glassworks.viewctl.environment.sensor.{sensor_id}"),
    );
}

#[test]
pub(crate) fn inventory_scheduler_and_safety_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Inventory),
        ..Default::default()
    });
    let lot_id = app
        .workspace()
        .inventory
        .sorted_lots()
        .first()
        .expect("demo inventory lot should exist")
        .id
        .clone();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.inventory.filter.low-stock"));
    assert_eq!(app.inventory_filter(), InventoryQuickFilter::LowStock);
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.inventory.lot.{lot_id}")));
    assert_eq!(app.selected_inventory_lot(), Some(&lot_id));

    let mut inventory_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("inventory controls should build");
    assert_clicked_node(
        &mut inventory_document,
        "glassworks.viewctl.inventory.filter.low-stock",
    );

    app.set_active_view(StartupView::Scheduler);
    let scheduler_tool = app
        .workspace()
        .scheduler
        .tools
        .first()
        .expect("demo dispatch tool should exist")
        .id
        .clone();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.scheduler.policy.due-date"));
    assert_eq!(app.scheduler_policy(), DispatchPolicy::DueDateThenPriority);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.scheduler.priority.3"));
    assert_eq!(app.scheduler_min_priority(), 3);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.scheduler.toggle_conflicts"));
    assert!(app.scheduler_conflicts_only());
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.scheduler.tool.{scheduler_tool}"
    )));
    assert_eq!(app.selected_scheduler_tool(), Some(&scheduler_tool));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.scheduler.reset"));
    assert_eq!(app.scheduler_min_priority(), 0);
    assert!(!app.scheduler_conflicts_only());

    let mut scheduler_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("scheduler controls should build");
    assert_clicked_node(
        &mut scheduler_document,
        "glassworks.viewctl.scheduler.policy.due-date",
    );

    app.set_active_view(StartupView::Safety);
    let lockout = app
        .workspace()
        .safety
        .evaluate_lockouts()
        .first()
        .expect("demo safety lockout should exist")
        .clone();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.safety.tool.{}",
        lockout.tool_id
    )));
    assert_eq!(app.selected_safety_tool(), Some(lockout.tool_id.as_str()));
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.safety.ack.lockout.{}",
        lockout.tool_id
    )));
    assert!(app.acknowledged_count() > 0);
    if let Some(sensor) = app.workspace().safety.active_conditions().first() {
        let sensor_id = sensor.id.to_string();
        assert!(app.apply_clicked_node_name(&format!(
            "glassworks.viewctl.safety.ack.condition.{sensor_id}"
        )));
    }

    let mut safety_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("safety controls should build");
    assert_clicked_node(
        &mut safety_document,
        &format!("glassworks.viewctl.safety.tool.{}", lockout.tool_id),
    );
}

#[test]
pub(crate) fn mask_prep_and_layout_diff_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::MaskPrep),
        ..Default::default()
    });
    let lot_id = app
        .workspace()
        .mes
        .lots
        .keys()
        .next()
        .expect("demo MES lot should exist")
        .clone();
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.mask.lot.{lot_id}")));
    assert_eq!(app.mask_source_lot(), Some(&lot_id));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.mask.severity.errors"));
    assert_eq!(
        app.mask_issue_severity_filter(),
        MaskIssueSeverityFilter::Errors
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.mask.group.layer"));
    assert_eq!(app.mask_issue_grouping(), MaskIssueGrouping::Layer);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.mask.rebuild"));
    assert!(app.status_message().contains("Reticle prep rebuilt"));

    let mut mask_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("reticle prep controls should build");
    assert_clicked_node(
        &mut mask_document,
        "glassworks.viewctl.mask.severity.errors",
    );

    app.set_active_view(StartupView::LayoutDiff);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.baseline.empty"));
    assert_eq!(app.layout_diff_baseline(), LayoutDiffSource::Empty);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.candidate.current"));
    assert_eq!(app.layout_diff_candidate(), LayoutDiffSource::Current);
    assert!(app.layout_diff_changed_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.toggle_changed_only"));
    assert!(!app.layout_diff_changed_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.filter.added"));
    assert_eq!(app.layout_diff_change_filter(), LayoutChangeFilter::Added);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.page_size.25"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout_diff.review.approved"));
    assert_eq!(
        app.layout_diff_review_state(),
        LayoutReviewDisposition::Approved
    );

    let mut diff_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout diff controls should build");
    assert_clicked_node(
        &mut diff_document,
        "glassworks.viewctl.layout_diff.baseline.empty",
    );
}

#[test]
pub(crate) fn traceability_and_notebook_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Traceability),
        ..Default::default()
    });
    let lot_id = app
        .workspace()
        .genealogy
        .lot_ids()
        .first()
        .expect("demo trace lot should exist")
        .clone();
    let wafer = app
        .workspace()
        .genealogy
        .wafer_refs_for_lot(&lot_id)
        .first()
        .expect("demo trace wafer should exist")
        .clone();
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.trace.lot.{lot_id}")));
    assert_eq!(app.selected_trace_lot(), Some(&lot_id));
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.trace.wafer.{}|{}",
        wafer.lot_id, wafer.wafer_id
    )));
    assert_eq!(app.selected_trace_wafer(), Some(&wafer.wafer_id));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.trace.impact.material"));
    assert_eq!(app.trace_impact_mode(), TraceImpactMode::Material);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.trace.toggle_related"));
    assert!(app.trace_related_only());
    let process_sequence = app
        .workspace()
        .genealogy
        .process_history
        .first()
        .expect("demo trace process history should exist")
        .sequence;
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.trace.detail.process.{process_sequence}"
    )));
    assert_eq!(
        app.selected_trace_detail,
        Some(TraceSelection::Process(process_sequence))
    );
    let material_sequence = app
        .workspace()
        .genealogy
        .material_uses
        .first()
        .expect("demo trace material use should exist")
        .sequence;
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.trace.detail.material.{material_sequence}"
    )));
    assert_eq!(
        app.selected_trace_detail,
        Some(TraceSelection::MaterialUse(material_sequence))
    );
    let event_sequence = app
        .workspace()
        .genealogy
        .events
        .first()
        .expect("demo trace event should exist")
        .sequence;
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.trace.detail.event.{event_sequence}"
    )));
    assert_eq!(
        app.selected_trace_detail,
        Some(TraceSelection::Event(event_sequence))
    );

    let mut trace_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("traceability controls should build");
    assert_clicked_node(
        &mut trace_document,
        "glassworks.viewctl.trace.impact.material",
    );
    assert_clicked_node(
        &mut trace_document,
        &format!("glassworks.viewctl.trace.detail.event.{event_sequence}"),
    );

    app.set_active_view(StartupView::Notebook);
    let entry_id = app
        .workspace()
        .lab_notebook
        .entries
        .first()
        .expect("demo notebook entry should exist")
        .id
        .clone();
    let tag = app
        .workspace()
        .lab_notebook
        .tags()
        .first()
        .expect("demo notebook tag should exist")
        .clone();
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.notebook.entry.{entry_id}")));
    assert_eq!(app.selected_notebook_entry(), Some(&entry_id));
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.notebook.tag.{tag}")));
    assert_eq!(app.notebook_tag_filter(), Some(tag.as_str()));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.notebook.link.lot"));
    assert_eq!(app.notebook_link_kind_filter(), Some(NotebookLinkKind::Lot));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.notebook.toggle_followups"));
    assert!(app.notebook_followups_only());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.notebook.edit"));
    assert!(!app.notebook_preview_mode());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.notebook.clear_filters"));
    assert_eq!(app.notebook_tag_filter(), None);
    assert_eq!(app.notebook_link_kind_filter(), None);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.notebook.entry_action.handoff"));
    assert!(
        app.workspace()
            .lab_notebook
            .entry(&entry_id)
            .expect("selected notebook entry should exist")
            .tags
            .iter()
            .any(|candidate| candidate == "handoff")
    );
    let (link_kind, link_value) = notebook_entry_link_focus_tokens(
        app.workspace()
            .lab_notebook
            .entry(&entry_id)
            .expect("selected notebook entry should exist"),
    )
    .first()
    .cloned()
    .expect("demo notebook entry should have a focusable link");
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.notebook.focus_link.{}:{}",
        notebook_link_kind_slug(link_kind),
        link_value
    )));
    assert_eq!(app.notebook_link_kind_filter(), Some(link_kind));

    let mut notebook_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("notebook controls should build");
    assert_clicked_node(
        &mut notebook_document,
        "glassworks.viewctl.notebook.preview",
    );
    assert_clicked_node(
        &mut notebook_document,
        "glassworks.viewctl.notebook.entry_action.handoff",
    );
}

#[test]
pub(crate) fn process_flow_and_cross_section_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::ProcessFlow),
        ..Default::default()
    });
    let node_id = app
        .workspace()
        .process_flow
        .route
        .nodes
        .first()
        .expect("demo process node should exist")
        .id
        .clone();

    assert!(app.apply_clicked_node_name("glassworks.viewctl.process_flow.filter.recipes"));
    assert_eq!(
        app.process_flow_filter(),
        ProcessFlowNodeFilter::RecipeSteps
    );
    assert!(app.apply_clicked_node_name("glassworks.viewctl.process_flow.toggle_errors"));
    assert!(app.process_flow_errors_only());
    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.process_flow.node.{node_id}"))
    );
    assert_eq!(app.selected_process_node(), Some(&node_id));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.process_flow.validate"));
    assert!(app.status_message().contains("Process flow"));

    let mut process_flow_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("process-flow controls should build");
    assert_clicked_node(
        &mut process_flow_document,
        "glassworks.viewctl.process_flow.filter.recipes",
    );

    app.set_active_view(StartupView::CrossSection);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.cross_section.step.1"));
    assert_eq!(app.cross_section_step(), 1);
    assert!(app.cross_section_show_mask());
    assert!(app.apply_clicked_node_name("glassworks.viewctl.cross_section.toggle_mask"));
    assert!(!app.cross_section_show_mask());
    let material_id = app
        .workspace()
        .cross_section
        .materials
        .first()
        .expect("demo cross-section material should exist")
        .id
        .clone();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.cross_section.material.{}",
        material_id.as_str()
    )));

    let mut cross_section_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("cross-section controls should build");
    assert_clicked_node(
        &mut cross_section_document,
        "glassworks.viewctl.cross_section.step.1",
    );
}

#[test]
pub(crate) fn process_control_and_spc_controls_update_state() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::ProcessControl),
        ..Default::default()
    });
    let target_loop = app
        .workspace()
        .process_control
        .loops
        .iter()
        .take(6)
        .last()
        .expect("demo process-control loop should exist")
        .id
        .clone();
    assert!(app.apply_clicked_node_name(&format!(
        "glassworks.viewctl.process_control.loop.{target_loop}"
    )));
    assert_eq!(app.selected_control_loop(), Some(&target_loop));

    if let Some(action_id) = app
        .workspace()
        .process_control
        .actions_for_loop(&target_loop)
        .first()
        .map(|action| action.id.clone())
    {
        assert!(app.apply_clicked_node_name(&format!(
            "glassworks.viewctl.process_control.action.{action_id}"
        )));
        assert_eq!(app.selected_control_action(), Some(&action_id));
    }

    if let Some(action_id) = app
        .workspace()
        .process_control
        .actions
        .iter()
        .find(|action| action.state == ControlActionState::Proposed)
        .map(|action| action.id.clone())
    {
        assert!(app.apply_clicked_node_name(&format!(
            "glassworks.viewctl.process_control.transition.{action_id}|approve"
        )));
        let state = app
            .workspace()
            .process_control
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .expect("transitioned action should remain present")
            .state;
        assert_eq!(state, ControlActionState::Approved);
    }

    let mut process_control_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("process-control controls should build");
    assert_clicked_node(
        &mut process_control_document,
        &format!("glassworks.viewctl.process_control.loop.{target_loop}"),
    );

    app.set_active_view(StartupView::SpcFdc);
    let monitor = spc_fdc_monitor(app.workspace());
    let chart_id = monitor
        .charts
        .last()
        .expect("demo SPC chart should exist")
        .id
        .as_str()
        .to_string();
    let trace_id = monitor
        .traces
        .last()
        .expect("demo FDC trace should exist")
        .id
        .clone();
    assert!(app.apply_clicked_node_name("glassworks.viewctl.spc.severity.critical"));
    assert_eq!(app.spc_severity_filter(), SpcSeverityFilter::Critical);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.spc.source.fdc"));
    assert_eq!(app.spc_source_filter(), SpcSourceFilter::Fdc);
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.spc.chart.{chart_id}")));
    assert_eq!(app.selected_spc_chart(), Some(chart_id.as_str()));
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.spc.trace.{trace_id}")));
    assert_eq!(app.selected_fdc_trace(), Some(trace_id.as_str()));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.spc.clear_context"));

    let mut spc_document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("SPC/FDC controls should build");
    assert_clicked_node(
        &mut spc_document,
        "glassworks.viewctl.spc.severity.critical",
    );
}

#[test]
pub(crate) fn restored_view_controls_build_at_common_sizes() {
    for view in [
        StartupView::Workflow,
        StartupView::Layout2d,
        StartupView::Layout3d,
        StartupView::MaskPrep,
        StartupView::LayoutDiff,
        StartupView::FabControl,
        StartupView::Maintenance,
        StartupView::Environment,
        StartupView::Inventory,
        StartupView::Scheduler,
        StartupView::Safety,
        StartupView::Traceability,
        StartupView::ProcessFlow,
        StartupView::ProcessControl,
        StartupView::SpcFdc,
        StartupView::CrossSection,
        StartupView::Metrology,
        StartupView::Yield,
        StartupView::Experiment,
        StartupView::Notebook,
    ] {
        for viewport in [UiSize::new(1024.0, 720.0), UiSize::new(2048.0, 1440.0)] {
            let app = GlassworksApp::new_with_options(StartupOptions {
                view_mode: Some(view),
                ..Default::default()
            });
            let document = app
                .build_operad_document(viewport)
                .expect("view control document should build");
            assert_eq!(
                document.audit_layout(),
                Vec::new(),
                "view {view:?} viewport {viewport:?}"
            );
        }
    }
}

#[test]
pub(crate) fn view_controls_explain_button_groups_and_details_collapse() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::MaskPrep),
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("mask prep view should build");
    assert_eq!(document.audit_layout(), Vec::new());
    let visible_text = document_visible_text(&document);
    for label in ["Source lot", "Severity", "Group by", "Issues", "Actions"] {
        assert!(
            visible_text.contains(label),
            "mask controls should label what each button group does: {label}\n{visible_text}"
        );
    }
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.mask.row.1.label"),
        "control rows should publish semantic row labels, not just a flat button grid"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.mask.group_row.0"),
        "control panels should pack related button rows into visible labeled groups"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.viewctl.mask.row.1.buttons"),
        "each control label should be attached to the button row it describes"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.domain.section.0.header"),
        "primary domain panels should group body content into collapsible sections"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.domain.section.1.header"),
        "collapsed primary sections should still expose clear named headers"
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.details.section.0.header"),
        "below-view detail panels should use collapsible headers"
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.details.0.title"),
        "detail panels should not render as an always-expanded title-and-row dump"
    );
}
