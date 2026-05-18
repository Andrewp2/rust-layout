#![allow(unused_imports)]
use super::*;
use crate::*;
use layout_model::{ReferenceImageLandmark, ReferenceImageSize};

pub(crate) fn document_visible_text(document: &UiDocument) -> String {
    let mut text = Vec::new();
    for node in document.nodes() {
        match node.content() {
            UiContent::Text(content) => text.push(content.text.as_str()),
            UiContent::Scene(primitives) => {
                for primitive in primitives {
                    if let ScenePrimitive::Text(content) = primitive {
                        text.push(content.text.as_str());
                    }
                }
            }
            _ => {}
        }
    }
    text.join("\n")
}

pub(crate) fn document_text_by_node(
    document: &UiDocument,
) -> std::collections::BTreeMap<String, String> {
    let mut text_by_node = std::collections::BTreeMap::new();
    for node in document.nodes() {
        match node.content() {
            UiContent::Text(content) => {
                text_by_node.insert(node.name().to_string(), content.text.clone());
            }
            UiContent::Scene(primitives) => {
                let scene_text = primitives
                    .iter()
                    .filter_map(|primitive| match primitive {
                        ScenePrimitive::Text(content) => Some(content.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !scene_text.is_empty() {
                    text_by_node.insert(node.name().to_string(), scene_text);
                }
            }
            _ => {}
        }
    }
    text_by_node
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_gzip_test_file(path: &Path, bytes: &[u8]) {
    use std::io::Write as _;

    let file = std::fs::File::create(path).expect("gzip test file should be created");
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    encoder
        .write_all(bytes)
        .expect("gzip test file should be written");
    encoder.finish().expect("gzip test file should finish");
}

pub(crate) fn shape_kind_area2_abs(kind: &ShapeKind) -> i128 {
    match kind {
        ShapeKind::Rectangle(rect) => rect.width() as i128 * rect.height() as i128 * 2,
        ShapeKind::Polygon(poly) => poly.signed_area2().abs(),
        _ => 0,
    }
}

#[test]
pub(crate) fn layout_hierarchy_depth_accepts_numeric_slugs() {
    assert_eq!(
        LayoutHierarchyDepth::from_slug(" 12 "),
        Some(LayoutHierarchyDepth::Numeric(12))
    );
    assert_eq!(
        LayoutHierarchyDepth::from_slug("depth-4"),
        Some(LayoutHierarchyDepth::Numeric(4))
    );
    assert_eq!(
        LayoutHierarchyDepth::from_slug("max_32"),
        Some(LayoutHierarchyDepth::Numeric(
            LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH
        ))
    );
    assert_eq!(
        LayoutHierarchyDepth::from_slug("depth_2"),
        Some(LayoutHierarchyDepth::Two)
    );
    assert_eq!(LayoutHierarchyDepth::from_slug("33"), None);
    assert_eq!(
        LayoutHierarchyDepth::for_minimum_depth(7),
        LayoutHierarchyDepth::Numeric(7)
    );
    assert_eq!(
        LayoutHierarchyDepth::Three.deeper(),
        LayoutHierarchyDepth::Numeric(4)
    );
    assert_eq!(
        LayoutHierarchyDepth::Numeric(4).shallower(),
        LayoutHierarchyDepth::Three
    );
    assert_eq!(
        LayoutHierarchyDepth::Numeric(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH).deeper(),
        LayoutHierarchyDepth::Full
    );
    assert_eq!(
        LayoutHierarchyDepth::Full.shallower(),
        LayoutHierarchyDepth::Numeric(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH)
    );
    assert_eq!(parse_layout_hierarchy_min_depth("min-2"), Some(2));
    assert_eq!(parse_layout_hierarchy_min_depth("5"), Some(5));
    assert_eq!(parse_layout_hierarchy_min_depth("33"), None);
    assert_eq!(
        clamp_layout_hierarchy_min_depth_for_max(5, LayoutHierarchyDepth::Three),
        3
    );
}

#[test]
pub(crate) fn startup_view_slugs_cover_known_routes() {
    assert_eq!(
        StartupView::from_slug("metrology"),
        Some(StartupView::Metrology)
    );
    assert_eq!(
        StartupView::from_slug("process-flow"),
        Some(StartupView::ProcessFlow)
    );
    assert_eq!(StartupView::from_slug("3d"), Some(StartupView::Layout3d));
}

#[test]
pub(crate) fn app_menu_contract_matches_d43277f_shell() {
    let labels = AppMenu::ALL.map(AppMenu::label);
    assert_eq!(
        labels,
        [
            "File",
            "Edit",
            "View",
            "Bookmarks",
            "Display",
            "Options",
            "Tools",
            "Macros",
            "Help"
        ]
    );

    let mut compact_app = GlassworksApp::new_with_options(StartupOptions::default());
    let compact_document = compact_app
        .build_operad_document(UiSize::new(640.0, 720.0))
        .expect("compact menu bar should build");
    assert!(
        compact_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.more")
    );
    assert!(
        !compact_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.bookmarks")
    );
    assert!(compact_app.apply_clicked_node_name("glassworks.menu.more"));
    let compact_document = compact_app
        .build_operad_document(UiSize::new(640.0, 720.0))
        .expect("compact More menu should build");
    assert!(
        compact_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.more.display")
    );
    assert!(
        compact_document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.more.options")
    );
    assert!(compact_app.apply_clicked_node_name("glassworks.menu.item.more.display"));
    assert_eq!(compact_app.active_menu(), Some(AppMenu::Display));
    assert!(compact_app.apply_clicked_node_name("glassworks.menu.more"));
    assert!(compact_app.apply_clicked_node_name("glassworks.menu.item.more.options"));
    assert_eq!(compact_app.active_menu(), Some(AppMenu::Options));

    let app = GlassworksApp::new_with_options(StartupOptions::default());
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("standard menu bar should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.options")
    );

    let mut app = GlassworksApp::new_with_options(StartupOptions::default());
    assert!(app.apply_clicked_node_name("glassworks.menu.display"));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("display menu should build");
    for node_name in [
        "glassworks.menu.item.display.grid2d",
        "glassworks.menu.item.display.grid3d",
        "glassworks.menu.item.display.origin",
        "glassworks.menu.item.display.drc",
        "glassworks.menu.item.display.inspector",
        "glassworks.menu.item.display.units.auto",
        "glassworks.menu.item.display.units.nanometers",
        "glassworks.menu.item.display.units.microns",
        "glassworks.menu.item.display.units.dbu",
        "glassworks.menu.item.display.options",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "{node_name} should be in Display menu"
        );
    }
    let selected_grid = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.display.grid2d")
        .expect("selected display item should exist");
    let UiContent::Scene(primitives) = selected_grid.content() else {
        panic!("selected menu items should use scene primitives");
    };
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, ScenePrimitive::Line { .. })),
        "selected menu item should draw a check indicator instead of a full selected button"
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.display.inspector"));
    assert!(app.show_inspector);
    assert!(app.apply_clicked_node_name("glassworks.menu.item.display.options"));
    assert!(app.show_options_panel);
    let document = app
        .build_operad_document(UiSize::new(1400.0, 1600.0))
        .expect("options panel should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.options_panel")
    );
    for node_name in [
        "glassworks.options.section.file",
        "glassworks.options.section.appearance",
        "glassworks.options.section.shell",
        "glassworks.options.section.layout",
        "glassworks.options.section.viewport3d",
        "glassworks.options.section.performance",
        "glassworks.options.section.files",
        "glassworks.options.section.domain.workflow",
        "glassworks.options.section.domain.mask",
        "glassworks.options.section.domain.layout_diff",
        "glassworks.options.section.domain.operations",
        "glassworks.options.section.domain.analysis",
        "glassworks.options.section.domain.engineering",
        "glassworks.options.action.file.save",
        "glassworks.options.action.appearance.theme.system",
        "glassworks.options.action.layout.pan.reset",
        "glassworks.options.action.viewport3d.fast_multiplier.4",
        "glassworks.options.action.performance.live_fps",
        "glassworks.options.action.files.autosave_interval.120",
        "glassworks.options.action.domains.safety.show_acknowledged",
        "glassworks.options.proxy.glassworks.viewctl.mask.severity.errors",
        "glassworks.options.proxy.glassworks.viewctl.trace.toggle_related",
        "glassworks.options.proxy.glassworks.viewctl.scheduler.priority.5",
        "glassworks.options.proxy.glassworks.viewctl.spc.source.fdc",
        "glassworks.options.proxy.glassworks.viewctl.spc.clear_context",
        "glassworks.options.proxy.glassworks.viewctl.process_flow.toggle_errors",
        "glassworks.options.proxy.glassworks.viewctl.process_control.filter.active",
        "glassworks.options.proxy.glassworks.viewctl.cross_section.step.1",
        "glassworks.options.proxy.glassworks.viewctl.notebook.tag.all",
        "glassworks.options.proxy.glassworks.viewctl.experiment.use_target",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "{node_name} should be exposed in grouped Options UI"
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.options.close"));
    assert!(!app.show_options_panel);

    let mut panel_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(panel_app.apply_clicked_node_name("glassworks.menu.display"));
    let document = panel_app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("display panel menu should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.display.secondary_panel")
    );
    assert!(panel_app.apply_clicked_node_name("glassworks.menu.item.display.secondary_panel"));
    assert!(panel_app.show_layers);

    assert!(app.apply_clicked_node_name("glassworks.menu.options"));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("options menu should build");
    assert_eq!(document.audit_layout(), Vec::new());
    for node_name in [
        "glassworks.menu.item.options.snap",
        "glassworks.menu.item.display.options",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "{node_name} should be in Options menu"
        );
    }

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("tools menu should build");
    for node_name in [
        "glassworks.menu.item.tools.palette",
        "glassworks.menu.item.tool.select",
        "glassworks.menu.item.tool.rect",
        "glassworks.menu.item.tool.polygon",
        "glassworks.menu.item.tool.path",
        "glassworks.menu.item.tool.via",
        "glassworks.menu.item.tool.measure",
        "glassworks.menu.item.tool.route",
        "glassworks.menu.item.tools.rundrc",
        "glassworks.menu.item.tools.rundrc_region",
        "glassworks.menu.item.tools.rundrc_cell",
        "glassworks.menu.item.tools.diagnostics",
    ] {
        assert!(
            document.nodes().iter().any(|node| node.name() == node_name),
            "{node_name} should be in Tools menu"
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.menu.item.tools.diagnostics"));
    assert!(app.show_diagnostics_panel);
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("diagnostics panel should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.diagnostics_panel")
    );
}

#[test]
pub(crate) fn app_options_apply_and_ui_actions_share_state() {
    let mut options = AppOptions::default();
    options.appearance.theme = options::ThemePreference::Light;
    options.appearance.unit_display = "microns".to_string();
    options.shell.startup_view = "layout2d".to_string();
    options.shell.show_details_panel = true;
    options.layout.default_tool = "rect".to_string();
    options.layout.show_2d_grid = false;
    options.layout.pan = [10.0, 20.0];
    options.viewport3d.capture_flycam_on_click = false;
    options.viewport3d.mouse_sensitivity = 0.006;
    options.viewport3d.fast_multiplier = 8.0;
    options.performance.live_fps_meter = false;
    options.domains.fab_control.auto_select_first_tool = false;
    options.domains.mask_prep.severity_filter = "warnings".to_string();
    options.domains.mask_prep.grouping = "layer".to_string();
    options.domains.layout_diff.changed_only = false;
    options.domains.layout_diff.page_size = 100;
    options.domains.inventory.quick_filter = "low-stock".to_string();
    options.domains.maintenance.work_filter = "calibration".to_string();
    options.domains.maintenance.history_filter = "all".to_string();
    options.domains.scheduler.dispatch_policy = "due-date".to_string();
    options.domains.scheduler.minimum_priority = 3;
    options.domains.scheduler.conflicts_only = true;
    options.domains.scheduler.focus_selected_tool = true;
    options.domains.traceability.impact_mode = "material".to_string();
    options.domains.traceability.related_only = true;
    options.domains.metrology.map_mode = "defects".to_string();
    options.domains.metrology.measurement_kind = "thickness".to_string();
    options.domains.metrology.failed_only = true;
    options.domains.yield_dashboard.map_filter = "failing".to_string();
    options.domains.yield_dashboard.attention_only = true;
    options.domains.yield_dashboard.excursions_only = true;
    options.domains.spc_fdc.severity_filter = "critical".to_string();
    options.domains.spc_fdc.source_filter = "fdc".to_string();
    options.domains.spc_fdc.context_filter = "context-token".to_string();
    options.domains.process_flow.node_filter = "recipes".to_string();
    options.domains.process_flow.errors_only = true;
    options.domains.run_to_run.loop_filter = "attention".to_string();
    options.domains.cross_section.step = 1;
    options.domains.cross_section.show_mask = false;
    options.domains.cross_section.show_dimensions = false;
    options.domains.cross_section.show_risks = false;
    options.domains.notebook.preview_mode = false;
    options.domains.notebook.followups_only = true;
    options.domains.experiment.run_filter = "needs-selected-response".to_string();
    options.domains.experiment.show_missing_only = true;
    options.domains.experiment.capture_value = 88.0;

    let mut app = GlassworksApp::new_with_options(StartupOptions {
        app_options: Some(options),
        ..Default::default()
    });

    assert_eq!(app.active_view(), StartupView::Layout2d);
    assert_eq!(app.active_tool(), ToolMode::Rect);
    assert!(!app.dark_theme);
    assert_eq!(app.unit_display, UnitDisplay::Microns);
    assert!(app.show_inspector());
    assert!(!app.show_grid);
    assert_eq!(app.layout_pan(), [10.0, 20.0]);
    assert!(!app.app_options().viewport3d.capture_flycam_on_click);
    assert_eq!(app.app_options().viewport3d.mouse_sensitivity, 0.006);
    assert_eq!(app.app_options().viewport3d.fast_multiplier, 8.0);
    assert!(!app.app_options().performance.live_fps_meter);
    assert_eq!(app.selected_equipment_tool, None);
    assert_eq!(
        app.mask_issue_severity_filter,
        MaskIssueSeverityFilter::Warnings
    );
    assert_eq!(app.mask_issue_grouping, MaskIssueGrouping::Layer);
    assert!(!app.layout_diff_changed_only);
    assert_eq!(app.layout_diff_page_size, 100);
    assert_eq!(app.inventory_filter, InventoryQuickFilter::LowStock);
    assert_eq!(
        app.maintenance_work_filter,
        MaintenanceWorkFilter::Calibration
    );
    assert_eq!(
        app.maintenance_history_filter,
        MaintenanceHistoryFilter::AllTools
    );
    assert_eq!(app.scheduler_policy, DispatchPolicy::DueDateThenPriority);
    assert_eq!(app.scheduler_min_priority, 3);
    assert!(app.scheduler_conflicts_only);
    assert!(app.scheduler_focus_selected_tool);
    assert_eq!(app.trace_impact_mode, TraceImpactMode::Material);
    assert!(app.trace_related_only);
    assert_eq!(app.metrology_map_mode, MetrologyMapMode::DefectReview);
    assert_eq!(app.metrology_kind, MeasurementKind::ThicknessNm);
    assert!(app.metrology_failed_only);
    assert_eq!(app.yield_map_filter, YieldMapFilter::Failing);
    assert!(app.show_only_attention_wafers);
    assert!(app.show_only_excursions);
    assert_eq!(app.spc_severity_filter, SpcSeverityFilter::Critical);
    assert_eq!(app.spc_source_filter, SpcSourceFilter::Fdc);
    assert_eq!(app.spc_context_filter, "context-token");
    assert_eq!(app.process_flow_filter, ProcessFlowNodeFilter::RecipeSteps);
    assert!(app.process_flow_errors_only);
    assert_eq!(
        app.process_control_loop_filter,
        ControlLoopFilter::Attention
    );
    assert_eq!(app.cross_section_step, 1);
    assert!(!app.cross_section_show_mask);
    assert!(!app.cross_section_show_dimensions);
    assert!(!app.cross_section_show_risks);
    assert!(!app.notebook_preview_mode);
    assert!(app.notebook_followups_only);
    assert_eq!(
        app.experiment_run_filter,
        ExperimentRunFilter::NeedsSelectedResponse
    );
    assert!(app.experiment_show_missing_only);
    assert_eq!(app.experiment_capture_value, 88.0);

    assert!(app.apply_clicked_node_name("glassworks.menu.item.display.grid2d"));
    assert!(app.show_grid);
    assert!(app.app_options().layout.show_2d_grid);

    assert!(app.apply_clicked_node_name("glassworks.options.action.performance.live_fps"));
    assert!(app.app_options().performance.live_fps_meter);

    assert!(app.apply_clicked_node_name("glassworks.viewctl.mask.severity.errors"));
    assert_eq!(
        app.app_options().domains.mask_prep.severity_filter,
        "errors"
    );
    assert!(
        app.apply_clicked_node_name("glassworks.options.proxy.glassworks.viewctl.mask.severity.all")
    );
    assert_eq!(app.app_options().domains.mask_prep.severity_filter, "all");

    assert!(app.apply_clicked_node_name("glassworks.options.action.layout.pan.reset"));
    assert_eq!(app.layout_pan(), [0.0, 0.0]);
    assert_eq!(app.app_options().layout.pan, [0.0, 0.0]);
    assert!(
        app.apply_clicked_node_name("glassworks.options.proxy.glassworks.viewctl.spc.clear_context")
    );
    assert!(app.app_options().domains.spc_fdc.context_filter.is_empty());

    assert!(app.apply_clicked_node_name("glassworks.options.action.appearance.theme.system"));
    assert_eq!(
        app.app_options().appearance.theme,
        options::ThemePreference::System
    );
    assert!(app.apply_clicked_node_name("glassworks.options.action.viewport3d.fast_multiplier.4"));
    assert_eq!(app.app_options().viewport3d.fast_multiplier, 4.0);
    assert!(
        app.apply_clicked_node_name("glassworks.options.action.domains.safety.show_acknowledged")
    );
    assert!(!app.app_options().domains.safety.show_acknowledged);
    assert!(app.apply_clicked_node_name("glassworks.viewctl.process_control.filter.active"));
    assert_eq!(app.app_options().domains.run_to_run.loop_filter, "active");
}

#[test]
pub(crate) fn ui_document_builds_without_layout_warnings() {
    let app = GlassworksApp::new_with_options(StartupOptions::default());
    let report = app
        .audit_operad_document(UiSize::new(1024.0, 720.0))
        .expect("UI document should build");
    assert!(report.paint_items > 0);
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("UI document should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.user_id")
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.status")
    );
    assert!(
        !document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.validation")
    );

    let closed_shell_y = node_rect(&document, "glassworks.shell").y;
    for menu in AppMenu::ALL {
        let mut app = GlassworksApp::new_with_options(StartupOptions::default());
        assert!(app.apply_clicked_node_name(&format!("glassworks.menu.{}", menu.slug())));
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .expect("UI document with open menu should build");
        assert_eq!(document.audit_layout(), Vec::new());
        let panel = document
            .nodes()
            .iter()
            .find(|node| node.name() == format!("glassworks.menu_panel.{}", menu.slug()))
            .expect("open menu should render a popup panel");
        assert!(
            panel.layout().rect.width <= 430.0,
            "{:?}",
            panel.layout().rect
        );
        let shell = document
            .nodes()
            .iter()
            .find(|node| node.name() == "glassworks.shell")
            .expect("shell should exist");
        assert!(
            (shell.layout().rect.y - closed_shell_y).abs() < 0.5,
            "menu popup should not push shell down: {:?}",
            shell.layout().rect
        );
    }
}

#[test]
pub(crate) fn visible_ui_surfaces_do_not_include_stale_chrome_copy() {
    assert_eq!(
        display_technology_name("Glassworks demo process"),
        "Sample process"
    );
    assert_eq!(
        display_technology_name("Glassworks high-density process"),
        "High-density process"
    );
    assert_eq!(
        display_document_name("Glassworks demo inverter"),
        "Sample inverter"
    );

    fn assert_no_stale_copy(surface: &str, document: &UiDocument) {
        let visible_text = document_visible_text(document);
        for stale_copy in [
            "Glassworks",
            "Inspector",
            "Operad v4 shell",
            "Semiconductor Layout Workspace",
            "Switched to",
        ] {
            assert!(
                !visible_text.contains(stale_copy),
                "{surface} should not include stale chrome copy {stale_copy:?}\n{visible_text}"
            );
        }
    }

    for view in StartupView::ALL {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .unwrap_or_else(|error| panic!("{view:?} document should build: {error}"));
        assert_no_stale_copy(&format!("{view:?} view"), &document);
    }

    for menu in AppMenu::ALL.into_iter().chain([AppMenu::More]) {
        let mut app = GlassworksApp::new_with_options(StartupOptions::default());
        assert!(app.apply_clicked_node_name(&format!("glassworks.menu.{}", menu.slug())));
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .unwrap_or_else(|error| panic!("{menu:?} menu document should build: {error}"));
        assert_no_stale_copy(&format!("{menu:?} menu"), &document);
    }

    for (surface, startup_actions) in [
        (
            "command palette",
            vec![
                "glassworks.menu.view".to_string(),
                "glassworks.menu.item.view.command_palette".to_string(),
            ],
        ),
        (
            "sidebar modules",
            vec![
                "glassworks.menu.view".to_string(),
                "glassworks.menu.item.view.sidebar_modules".to_string(),
            ],
        ),
        (
            "options panel",
            vec![
                "glassworks.menu.options".to_string(),
                "glassworks.menu.item.display.options".to_string(),
            ],
        ),
        (
            "details panel",
            vec![
                "glassworks.menu.display".to_string(),
                "glassworks.menu.item.display.inspector".to_string(),
            ],
        ),
        (
            "view engineering menu",
            vec![
                "glassworks.menu.view".to_string(),
                "glassworks.menu.item.view.group.engineering".to_string(),
            ],
        ),
    ] {
        let app = GlassworksApp::new_with_options(StartupOptions {
            startup_actions,
            ..Default::default()
        });
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .unwrap_or_else(|error| panic!("{surface} document should build: {error}"));
        assert_no_stale_copy(surface, &document);
    }

    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        startup_actions: vec![
            "glassworks.menu.display".to_string(),
            "glassworks.menu.item.display.secondary_panel".to_string(),
        ],
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("secondary panel document should build");
    assert_no_stale_copy("secondary panel", &document);
}

#[test]
pub(crate) fn builtin_workspace_report_still_validates_domain_data() {
    let report = validate_builtin_demo_workspace().expect("demo workspace should validate");
    assert!(report.document_shapes > 0);
    assert!(report.equipment_tools > 0);
}

#[test]
pub(crate) fn operad_click_actions_switch_views_and_menus() {
    let mut app = GlassworksApp::new_with_options(StartupOptions::default());

    assert!(app.apply_clicked_node_name("glassworks.menu.file"));
    assert_eq!(app.active_menu(), Some(AppMenu::File));

    assert!(app.apply_clicked_node_name("glassworks.nav.action.metrology"));
    assert_eq!(app.active_view(), StartupView::Metrology);
    assert_eq!(app.active_menu(), None);

    assert!(app.apply_clicked_node_name("glassworks.menu.view"));
    assert_eq!(app.active_menu(), Some(AppMenu::View));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("view menu should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.view.command_palette")
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu.item.view.sidebar_modules")
    );
    for group in ModuleGroup::ALL {
        assert!(
            document.nodes().iter().any(
                |node| node.name() == format!("glassworks.menu.item.view.group.{}", group.slug())
            ),
            "{} group should be in View menu",
            group.label()
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.menu.item.view.group.engineering"));
    assert_eq!(app.active_view_group, Some(ModuleGroup::Engineering));
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("view submenu should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu_subpanel.view.engineering")
    );
    assert!(app.apply_clicked_node_name("glassworks.menu.item.view.process-flow"));
    assert_eq!(app.active_view(), StartupView::ProcessFlow);

    assert!(app.apply_clicked_node_name("glassworks.menu.view"));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.view.command_palette"));
    assert!(app.show_command_palette);
    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("command palette should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.command_palette")
    );
    for view in StartupView::ALL {
        assert!(
            document
                .nodes()
                .iter()
                .any(|node| node.name() == format!("glassworks.command.view.{}", view.slug())),
            "command palette should include {:?}",
            view
        );
    }
    assert!(app.apply_clicked_node_name("glassworks.command.close"));
    assert!(!app.show_command_palette);

    assert!(app.apply_clicked_node_name("glassworks.menu.view"));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.view.sidebar_modules"));
    assert!(app.show_sidebar_modules);
    assert!(app.nav_rail_views.contains(&StartupView::Metrology));
    assert!(app.apply_clicked_node_name("glassworks.sidebar.view.metrology"));
    assert!(!app.nav_rail_views.contains(&StartupView::Metrology));
    assert!(app.apply_clicked_node_name("glassworks.sidebar.default"));
    assert!(app.nav_rail_views.contains(&StartupView::Metrology));
}

#[test]
pub(crate) fn command_palette_commands_stay_visible_at_hidpi() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        startup_actions: vec![
            "glassworks.menu.view".to_string(),
            "glassworks.menu.item.view.command_palette".to_string(),
        ],
        ..Default::default()
    });
    let document = app
        .build_operad_document_scaled(UiSize::new(2048.0, 1440.0), UiScale::new(2.0))
        .expect("HiDPI command palette should build");
    assert_eq!(document.audit_layout(), Vec::new());
    let list = document
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.command.list")
        .expect("command list should exist");
    for view in StartupView::ALL {
        let command = document
            .nodes()
            .iter()
            .find(|node| node.name() == format!("glassworks.command.view.{}", view.slug()))
            .unwrap_or_else(|| panic!("missing command for {view:?}"));
        assert!(
            command.layout().rect.x + command.layout().rect.width
                <= list.layout().rect.x + list.layout().rect.width + 0.5
                && command.layout().rect.y + command.layout().rect.height
                    <= list.layout().rect.y + list.layout().rect.height + 0.5,
            "command {:?} should fit visible command list: command {:?}, list {:?}",
            view,
            command.layout().rect,
            list.layout().rect
        );
    }
}

#[test]
pub(crate) fn startup_actions_can_render_open_menu_states() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        startup_actions: vec![
            "glassworks.menu.view".to_string(),
            "glassworks.menu.item.view.group.engineering".to_string(),
        ],
        ..Default::default()
    });
    assert_eq!(app.active_menu(), Some(AppMenu::View));
    assert_eq!(app.active_view_group, Some(ModuleGroup::Engineering));

    let document = app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("open View menu state should build");
    assert_eq!(document.audit_layout(), Vec::new());
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu_panel.view")
    );
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.menu_subpanel.view.engineering")
    );
}

#[test]
pub(crate) fn restored_shell_actions_cover_tools_and_panels() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });

    assert!(app.apply_clicked_node_name("glassworks.tool.via"));
    assert_eq!(app.active_tool(), ToolMode::Via);

    assert!(app.apply_clicked_node_name("glassworks.menu.tools"));
    assert_eq!(app.active_menu(), Some(AppMenu::Tools));
    assert!(app.apply_clicked_node_name("glassworks.menu.item.tool.route"));
    assert_eq!(app.active_tool(), ToolMode::Route);

    assert!(!app.show_inspector());
    assert!(app.apply_clicked_node_name("glassworks.menu.item.display.inspector"));
    assert!(app.show_inspector());

    assert!(!app.show_layers());
    assert!(app.apply_clicked_node_name("glassworks.menu.item.display.secondary_panel"));
    assert!(app.show_layers());

    let desktop = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("desktop shell should build");
    assert!(
        desktop
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.inspector")
    );
    assert!(
        desktop
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.secondary")
    );

    let mut compact_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let compact = compact_app
        .build_operad_document(UiSize::new(820.0, 620.0))
        .expect("compact shell should build");
    assert!(
        !compact
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.inspector")
    );
    assert!(
        !compact
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.secondary")
    );
    assert!(compact_app.apply_clicked_node_name("glassworks.menu.item.display.secondary_panel"));
    let compact = compact_app
        .build_operad_document(UiSize::new(820.0, 620.0))
        .expect("compact drawer shell should build");
    assert!(
        compact
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.secondary")
    );
}

#[test]
pub(crate) fn unwired_menu_items_are_visible_but_disabled() {
    fn input_state(input: InputBehavior) -> (bool, bool, bool) {
        (input.pointer, input.focusable, input.keyboard)
    }

    let mut file_app = GlassworksApp::new_with_options(StartupOptions::default());
    assert!(file_app.apply_clicked_node_name("glassworks.menu.file"));
    let file_document = file_app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("File menu should build");
    for node_name in ["glassworks.menu.item.file.collaboration"] {
        let node = file_document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("{node_name} should stay visible in the File menu"));
        assert_eq!(
            input_state(node.input()),
            input_state(InputBehavior::NONE),
            "{node_name} should be disabled until its UI action is wired"
        );
    }
    for node_name in [
        "glassworks.menu.item.file.new_blank",
        "glassworks.menu.item.file.load_demo",
        "glassworks.menu.item.file.save",
        "glassworks.menu.item.file.load",
        "glassworks.menu.item.file.save_session",
        "glassworks.menu.item.file.load_session",
        "glassworks.menu.item.file.save_layout",
        "glassworks.menu.item.file.load_layout",
        "glassworks.menu.item.file.export_gds",
        "glassworks.menu.item.file.import_gds",
        "glassworks.menu.item.file.export_cif",
        "glassworks.menu.item.file.import_cif",
        "glassworks.menu.item.file.export_dxf",
        "glassworks.menu.item.file.import_dxf",
        "glassworks.menu.item.file.export_def",
        "glassworks.menu.item.file.import_def",
        "glassworks.menu.item.file.export_lef",
        "glassworks.menu.item.file.import_lef",
        "glassworks.menu.item.file.export_reference_images",
        "glassworks.menu.item.file.import_reference_images",
    ] {
        let node = file_document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("{node_name} should stay visible in the File menu"));
        assert_eq!(
            input_state(node.input()),
            input_state(InputBehavior::BUTTON)
        );
    }

    let mut edit_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    assert!(edit_app.apply_clicked_node_name("glassworks.menu.edit"));
    let edit_document = edit_app
        .build_operad_document(UiSize::new(1024.0, 720.0))
        .expect("Edit menu should build");
    for node_name in [
        "glassworks.menu.item.edit.undo",
        "glassworks.menu.item.edit.redo",
        "glassworks.menu.item.edit.copy",
        "glassworks.menu.item.edit.paste",
        "glassworks.menu.item.edit.duplicate",
        "glassworks.menu.item.edit.make_cell",
        "glassworks.menu.item.edit.make_variant",
        "glassworks.menu.item.edit.rotate90",
        "glassworks.menu.item.edit.mirror_x",
        "glassworks.menu.item.edit.mirror_y",
    ] {
        let node = edit_document
            .nodes()
            .iter()
            .find(|node| node.name() == node_name)
            .unwrap_or_else(|| panic!("{node_name} should stay visible in the Edit menu"));
        assert_eq!(
            input_state(node.input()),
            input_state(InputBehavior::BUTTON)
        );
    }

    for (menu, disabled_node) in [
        (AppMenu::Bookmarks, "glassworks.menu.item.bookmarks.origin"),
        (AppMenu::Help, "glassworks.menu.item.help.about"),
    ] {
        let mut app = GlassworksApp::new_with_options(StartupOptions::default());
        assert!(app.apply_clicked_node_name(&format!("glassworks.menu.{}", menu.slug())));
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .expect("menu with disabled item should build");
        let node = document
            .nodes()
            .iter()
            .find(|node| node.name() == disabled_node)
            .unwrap_or_else(|| panic!("{disabled_node} should stay visible"));
        assert_eq!(
            input_state(node.input()),
            input_state(InputBehavior::NONE),
            "{disabled_node} should be disabled until its UI action is wired"
        );
        if menu == AppMenu::Help {
            let visible_text = document_visible_text(&document);
            assert!(visible_text.contains("About"));
            assert!(
                !visible_text.contains("Glassworks"),
                "Help menu should avoid redundant product copy\n{visible_text}"
            );
        }
    }
}

#[test]
pub(crate) fn compact_layout_edit_menu_has_no_layout_warnings() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        startup_actions: vec!["glassworks.menu.edit".to_string()],
        ..Default::default()
    });
    let document = app
        .build_operad_document(UiSize::new(480.0, 900.0))
        .expect("compact Layout Editor edit menu should build");
    assert_eq!(document.audit_layout(), Vec::new());
}

#[test]
pub(crate) fn compact_editor_tool_strip_keeps_actions_visible() {
    let layout_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let layout_doc = layout_app
        .build_operad_document(UiSize::new(480.0, 900.0))
        .expect("narrow layout editor should build");
    assert_eq!(layout_doc.audit_layout(), Vec::new());
    let select = layout_doc
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.tool.select")
        .expect("select tool should exist");
    let route = layout_doc
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.tool.route")
        .expect("route tool should exist");
    let delete = layout_doc
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.edit.delete")
        .expect("delete action should exist");
    let canvas = layout_doc
        .nodes()
        .iter()
        .find(|node| {
            matches!(
                node.content(),
                UiContent::Canvas(canvas) if canvas.key == "glassworks.layout.viewport.2d"
            )
        })
        .expect("2D canvas should exist");
    assert!(
        route.layout().rect.y > select.layout().rect.y,
        "narrow editor actions should use a second toolbar row instead of a clipped single row"
    );
    for node in [select, route, delete] {
        assert!(
            node.layout().rect.right() <= 480.0,
            "{} should stay inside the narrow viewport: {:?}",
            node.name(),
            node.layout().rect
        );
    }
    assert!(
        delete.layout().rect.bottom() <= canvas.layout().rect.y,
        "compact toolbar should finish before the 2D canvas starts: delete {:?}, canvas {:?}",
        delete.layout().rect,
        canvas.layout().rect
    );

    let viewport_app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    let viewport_doc = viewport_app
        .build_operad_document(UiSize::new(480.0, 900.0))
        .expect("narrow 3D viewport should build");
    assert_eq!(viewport_doc.audit_layout(), Vec::new());
    let fullscreen = viewport_doc
        .nodes()
        .iter()
        .find(|node| node.name() == "glassworks.menu.item.tools.fullscreen")
        .expect("fullscreen action should exist");
    assert!(
        fullscreen.layout().rect.right() <= 480.0,
        "3D compact toolbar actions should stay inside the viewport: {:?}",
        fullscreen.layout().rect
    );
}

#[test]
pub(crate) fn hidpi_editor_tool_strip_centers_button_labels() {
    let app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let document = app
        .build_operad_document_scaled(UiSize::new(1815.0, 900.0), UiScale::new(2.0))
        .expect("HiDPI layout editor should build");
    assert_eq!(document.audit_layout(), Vec::new());
    let strip = node_rect(&document, "glassworks.tool_strip");

    for name in [
        "glassworks.toolbar.view.layout2d",
        "glassworks.toolbar.view.layout3d",
        "glassworks.tool.select",
        "glassworks.tool.rect",
        "glassworks.toolbar.edit.delete",
    ] {
        let button = node_rect(&document, name);
        let label = node_rect(&document, &format!("{name}.label"));
        assert!(
            label.x >= button.x
                && label.right() <= button.right()
                && label.y >= button.y
                && label.bottom() <= button.bottom(),
            "{name} label should stay inside its toolbar button: button {button:?}, label {label:?}"
        );
        let button_center_y = button.y + button.height / 2.0;
        let label_center_y = label.y + label.height / 2.0;
        assert!(
            (button_center_y - label_center_y).abs() <= 2.0,
            "{name} label should be vertically centered: button {button:?}, label {label:?}"
        );
        let top_margin = button.y - strip.y;
        let bottom_margin = strip.bottom() - button.bottom();
        assert!(
            top_margin >= 6.0 && bottom_margin >= 6.0,
            "{name} should keep top and bottom toolbar padding: strip {strip:?}, button {button:?}"
        );
        assert!(
            (top_margin - bottom_margin).abs() <= 2.0,
            "{name} toolbar padding should be balanced: strip {strip:?}, button {button:?}"
        );
    }
}

#[test]
pub(crate) fn non_layout_views_reserve_editor_tool_strip_slot() {
    for viewport in [UiSize::new(1440.0, 920.0), UiSize::new(480.0, 900.0)] {
        let layout_app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let layout_document = layout_app
            .build_operad_document(viewport)
            .expect("layout editor document should build");
        let mask_app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::MaskPrep),
            ..Default::default()
        });
        let mask_document = mask_app
            .build_operad_document(viewport)
            .expect("mask prep document should build");

        let layout_shell = node_rect(&layout_document, "glassworks.shell");
        let mask_shell = node_rect(&mask_document, "glassworks.shell");
        assert!(
            (layout_shell.y - mask_shell.y).abs() < 0.5,
            "non-layout views should reserve the editor tool-strip height at {viewport:?}: layout shell {:?}, mask shell {:?}",
            layout_shell,
            mask_shell
        );
        assert!(
            mask_document
                .nodes()
                .iter()
                .any(|node| node.name() == "glassworks.tool_strip_spacer"),
            "mask prep should reserve an inert tool-strip spacer"
        );
        assert!(
            !mask_document
                .nodes()
                .iter()
                .any(|node| node.name() == "glassworks.tool.rect"),
            "reserved non-layout strip should not expose editor tool buttons"
        );
    }
}

#[test]
pub(crate) fn layout_views_restore_wide_inline_secondary_panel() {
    for (view, expected_title) in [
        (StartupView::Layout2d, "Layers"),
        (StartupView::Layout3d, "3D Stack"),
    ] {
        let app = GlassworksApp::new_with_options(StartupOptions {
            view_mode: Some(view),
            ..Default::default()
        });
        assert!(
            !app.show_layers(),
            "wide inline panels should not depend on the drawer toggle"
        );
        let wide = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("wide layout view should build");
        assert_eq!(wide.audit_layout(), Vec::new());
        let secondary = node_rect(&wide, "glassworks.secondary");
        let preview = node_rect(&wide, "glassworks.layout.preview");
        assert!(
            preview.right() <= secondary.x + 1.0,
            "secondary panel should reserve right-side space instead of overlapping the canvas: preview={preview:?} secondary={secondary:?}"
        );
        let title = wide
            .nodes()
            .iter()
            .find(|node| node.name() == "glassworks.secondary.title")
            .expect("secondary panel should have a title");
        assert!(
            matches!(title.content(), UiContent::Text(text) if text.text == expected_title),
            "secondary panel title should match the active layout view"
        );
        if view == StartupView::Layout3d {
            assert!(
                wide.nodes()
                    .iter()
                    .any(|node| node.name() == "glassworks.secondary.layers.title"),
                "3D side panel should include the layer stack summary above the layer controls"
            );
        }

        let compact = app
            .build_operad_document(UiSize::new(820.0, 620.0))
            .expect("compact layout view should build");
        assert!(
            !compact
                .nodes()
                .iter()
                .any(|node| node.name() == "glassworks.secondary"),
            "compact layout views should keep the secondary panel in drawer mode until requested"
        );
    }
}

#[test]
pub(crate) fn layout_3d_canvas_input_updates_camera() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    let canvas = UiRect::new(10.0, 20.0, 900.0, 560.0);
    let start = UiPoint::new(450.0, 300.0);
    let moved = UiPoint::new(510.0, 265.0);
    let before = app.camera_3d;

    assert!(app.handle_layout_3d_canvas_input(&operad::UiInputEvent::PointerDown(start), canvas));
    assert!(app.flycam_captured());
    assert!(app.handle_layout_3d_canvas_input(&operad::UiInputEvent::PointerMove(moved), canvas));
    assert_ne!(
        app.camera_3d, before,
        "dragging the 3D canvas should update flycam look direction"
    );
    assert_eq!(
        app.camera_3d.position, before.position,
        "mouse look should not orbit or translate the camera position"
    );

    let before_scroll = app.camera_3d;
    assert!(app.handle_layout_3d_canvas_input(
        &operad::UiInputEvent::wheel(moved, UiPoint::new(0.0, -120.0)),
        canvas
    ));
    assert!(
        app.camera_3d.position != before_scroll.position,
        "wheel-up should move the flycam along its view direction"
    );
    assert!(app.handle_layout_3d_canvas_input(&operad::UiInputEvent::PointerUp(moved), canvas));
}

#[test]
pub(crate) fn layout_3d_toolbar_and_keyboard_controls_are_wired() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    assert!(app.apply_layout_3d_mouse_delta(UiPoint::new(70.0, -30.0), false));
    assert_ne!(app.camera_3d, Camera3d::default());

    let moved_camera = app.camera_3d;
    assert!(app.apply_clicked_node_name("glassworks.toolbar.tools.reset3d"));
    assert_ne!(app.camera_3d, moved_camera);

    let before_move = app.camera_3d;
    assert!(app.handle_layout_3d_key(operad::KeyCode::Character('w'), operad::KeyModifiers::NONE));
    assert_ne!(app.camera_3d, before_move, "W should move the 3D camera");

    assert!(!app.viewport_fullscreen);
    assert!(app.apply_clicked_node_name("glassworks.toolbar.tools.fullscreen"));
    assert!(app.viewport_fullscreen);
    assert!(app.handle_layout_3d_key(operad::KeyCode::Escape, operad::KeyModifiers::NONE));
    assert!(!app.viewport_fullscreen);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_3d_batch_cache_ignores_camera_only_changes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout3d),
        ..Default::default()
    });
    let mut resources = Viewport3dCanvasResources::default();

    let first_fingerprint = refresh_layout_3d_batch_cache(&app, &mut resources);
    let first_batch = resources.batch.as_ref().expect("3D batch should be cached") as *const _;
    let first_revision = resources.batch_revision;

    app.camera_3d.position.x += 250.0;
    let camera_fingerprint = refresh_layout_3d_batch_cache(&app, &mut resources);
    let camera_batch = resources
        .batch
        .as_ref()
        .expect("3D batch should stay cached") as *const _;
    assert_eq!(first_fingerprint, camera_fingerprint);
    assert_eq!(first_revision, resources.batch_revision);
    assert_eq!(
        first_batch, camera_batch,
        "camera-only movement must not rebuild the 3D geometry batch"
    );

    app.show_3d_grid = !app.show_3d_grid;
    let grid_fingerprint = refresh_layout_3d_batch_cache(&app, &mut resources);
    assert_ne!(
        first_fingerprint, grid_fingerprint,
        "3D grid visibility changes should invalidate the cached batch"
    );

    app.app_options.performance.dense_3d_instancing = false;
    let mesh_fingerprint = refresh_layout_3d_batch_cache(&app, &mut resources);
    assert_ne!(
        grid_fingerprint, mesh_fingerprint,
        "switching 3D instancing should invalidate the cached batch"
    );
    let mesh_batch = resources
        .batch
        .as_ref()
        .expect("mesh batch should be cached");
    assert!(mesh_batch.rect_slabs.is_empty());
    assert!(!mesh_batch.vertices.is_empty());

    let grid_revision = resources.batch_revision;
    app.mark_layout_dirty();
    refresh_layout_3d_batch_cache(&app, &mut resources);
    assert_ne!(grid_revision, resources.batch_revision);
    assert_eq!(app.layout_revision, resources.batch_revision);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_2d_frame_cache_ignores_pointer_only_changes() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let mut resources = LayoutCanvasResources::default();
    let size = UiSize::new(900.0, 700.0);
    let viewport = app.layout_viewport_for_size(size);
    let zoom = app.layout_zoom;

    let first_fingerprint = refresh_layout_2d_frame_cache(&app, &mut resources, viewport, zoom);
    let first_frame = resources.frame.as_ref().expect("2D frame should be cached") as *const _;
    let first_revision = resources.layout_revision;
    let first_tiles = resources.frame_tiles.clone();

    app.layout_pointer = Some(UiPoint::new(120.0, 80.0));
    let pointer_fingerprint = refresh_layout_2d_frame_cache(&app, &mut resources, viewport, zoom);
    let pointer_frame = resources
        .frame
        .as_ref()
        .expect("2D frame should stay cached") as *const _;
    assert_eq!(first_fingerprint, pointer_fingerprint);
    assert_eq!(first_revision, resources.layout_revision);
    assert_eq!(
        first_frame, pointer_frame,
        "pointer-only movement must not rebuild the 2D tiled frame"
    );

    app.pan_layout_canvas_by(UiPoint::new(20.0, 0.0));
    let panned_viewport = app.layout_viewport_for_size(size);
    let small_pan_fingerprint =
        refresh_layout_2d_frame_cache(&app, &mut resources, panned_viewport, app.layout_zoom);
    let small_pan_frame = resources
        .frame
        .as_ref()
        .expect("2D frame should stay cached for same tile coverage")
        as *const _;
    assert_eq!(first_fingerprint, small_pan_fingerprint);
    assert_eq!(first_frame, small_pan_frame);
    assert_eq!(
        first_tiles, resources.frame_tiles,
        "small pans inside the same tile coverage should not rebuild the 2D frame"
    );

    app.pan_layout_canvas_by(UiPoint::new(3_000.0, 0.0));
    let large_panned_viewport = app.layout_viewport_for_size(size);
    refresh_layout_2d_frame_cache(&app, &mut resources, large_panned_viewport, app.layout_zoom);
    assert_ne!(
        first_tiles, resources.frame_tiles,
        "panning across tile coverage should update the cached 2D tile key"
    );

    let panned_revision = resources.layout_revision;
    app.mark_layout_dirty();
    let dirty_viewport = app.layout_viewport_for_size(size);
    refresh_layout_2d_frame_cache(&app, &mut resources, dirty_viewport, app.layout_zoom);
    assert_ne!(panned_revision, resources.layout_revision);
    assert_eq!(app.layout_revision, resources.layout_revision);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_2d_frame_cache_rebuilds_for_hierarchy_depth_changes() {
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
    let mut resources = LayoutCanvasResources::default();
    let viewport = app
        .with_layout_index(|index| index.bounds())
        .expect("hierarchy demo should have layout bounds")
        .expanded(1_000);

    refresh_layout_2d_frame_cache(&app, &mut resources, viewport, app.layout_zoom);
    let full_shapes = resources
        .frame
        .as_ref()
        .expect("2D frame should be cached")
        .stats
        .visible_shapes;
    let layout_revision = app.layout_revision;
    let view_revision = app.layout_view_revision;

    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.top"));
    assert_eq!(app.layout_revision, layout_revision);
    assert_ne!(app.layout_view_revision, view_revision);

    refresh_layout_2d_frame_cache(&app, &mut resources, viewport, app.layout_zoom);
    let top_shapes = resources
        .frame
        .as_ref()
        .expect("2D frame should be cached after depth change")
        .stats
        .visible_shapes;

    assert!(
        top_shapes < full_shapes,
        "top-only hierarchy display should render fewer shapes than full hierarchy"
    );
    assert_eq!(resources.layout_revision, app.layout_revision);
    assert_eq!(resources.layout_view_revision, app.layout_view_revision);

    let top_view_revision = app.layout_view_revision;
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.boxes"));
    assert_eq!(app.layout_revision, layout_revision);
    assert_ne!(app.layout_view_revision, top_view_revision);

    refresh_layout_2d_frame_cache(&app, &mut resources, viewport, app.layout_zoom);
    let box_shapes = resources
        .frame
        .as_ref()
        .expect("2D frame should be cached after box-mode change")
        .stats
        .visible_shapes;
    assert_eq!(
        box_shapes, top_shapes,
        "box-only hierarchy display should render current-cell geometry through the normal tile frame"
    );
    assert!(
        !layout_hierarchy_box_bounds(&app, UiSize::new(900.0, 700.0)).is_empty(),
        "box-only hierarchy display should overlay child instance bounds"
    );

    let box_view_revision = app.layout_view_revision;
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy.full"));
    assert!(app.apply_clicked_node_name("glassworks.viewctl.layout.hierarchy_min.1"));
    assert_eq!(app.layout_revision, layout_revision);
    assert_ne!(app.layout_view_revision, box_view_revision);

    refresh_layout_2d_frame_cache(&app, &mut resources, viewport, app.layout_zoom);
    let min_depth_shapes = resources
        .frame
        .as_ref()
        .expect("2D frame should be cached after min-depth change")
        .stats
        .visible_shapes;
    assert!(
        min_depth_shapes < full_shapes,
        "minimum hierarchy depth should filter current-root geometry from the normal tile frame"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
pub(crate) fn layout_2d_view_top_cell_uses_child_cell_coordinates() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        hierarchy_demo: true,
        ..Default::default()
    });
    let child = app
        .workspace
        .document
        .cells
        .values()
        .find(|cell| cell.id != app.workspace.document.top_cell && !cell.shapes.is_empty())
        .map(|cell| cell.id)
        .expect("hierarchy demo should have a child cell with shapes");
    let child_bounds =
        LayoutIndex::rebuild_hierarchical_for_cell(&app.workspace.document, child, None)
            .bounds()
            .expect("child cell should have local bounds");
    let full_bounds = app
        .with_layout_display_index(|index| index.bounds())
        .expect("document view should have bounds");
    let layout_revision = app.layout_revision;
    let view_revision = app.layout_view_revision;

    assert_ne!(full_bounds, child_bounds);
    assert!(app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", child.0)));

    assert_eq!(app.layout_revision, layout_revision);
    assert_ne!(app.layout_view_revision, view_revision);
    assert_eq!(app.layout_view_top_cell, child);
    assert_eq!(
        app.with_layout_display_index(|index| index.bounds()),
        Some(child_bounds)
    );
}

#[test]
pub(crate) fn layout_cell_browser_exposes_every_cell_as_view_top_action() {
    let mut app = GlassworksApp::new_with_options(StartupOptions {
        view_mode: Some(StartupView::Layout2d),
        ..Default::default()
    });
    let metal1 = app
        .workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .expect("default technology should include metal1");
    let mut cell_ids = vec![app.workspace.document.top_cell];
    for index in 0..5 {
        let cell = app.workspace.document.create_cell(format!("leaf_{index}"));
        app.workspace
            .document
            .insert_shape_in_cell(
                cell,
                metal1,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(index * 100, 0), 40, 40)),
            )
            .expect("test child shape should be inserted");
        cell_ids.push(cell);
    }

    let document = app
        .build_operad_document(UiSize::new(1440.0, 920.0))
        .expect("layout document should build");
    assert!(
        document
            .nodes()
            .iter()
            .any(|node| node.name() == "glassworks.layout.cell_browser.title"),
        "layout inspector should expose a cell browser"
    );
    for cell_id in &cell_ids {
        let action = format!("glassworks.viewctl.layout.top_cell.{}", cell_id.0);
        assert!(
            document.nodes().iter().any(|node| node.name() == action),
            "cell browser should expose {action}"
        );
    }

    let last_cell = *cell_ids.last().expect("test should have cells");
    assert!(
        app.apply_clicked_node_name(&format!("glassworks.viewctl.layout.top_cell.{}", last_cell.0))
    );
    assert_eq!(app.layout_view_top_cell, last_cell);
}
