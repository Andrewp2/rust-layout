#![allow(unused_imports)]
use super::*;

pub(crate) fn add_options_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport: UiSize,
    ui_scale: UiScale,
) {
    let width = (viewport.width - ui_scale.value(48.0))
        .min(ui_scale.value(1120.0))
        .max(ui_scale.value(420.0));
    let height = (viewport.height - ui_scale.value(120.0)).max(ui_scale.value(420.0));
    let left = ((viewport.width - width) * 0.5).max(ui_scale.value(12.0));
    let top = ui_scale.value(64.0);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.options_panel",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_absolute_position(
                        layout::with_size(layout::column(), layout::px(width), layout::px(height)),
                        left,
                        top,
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    document.node_mut(panel).style_mut().set_z_index(100);
    add_overlay_title_row(
        document,
        panel,
        "glassworks.options",
        "Options",
        "glassworks.options.close",
        width,
        ui_scale,
    );
    let content_height = (height - ui_scale.value(48.0)).max(ui_scale.value(240.0));
    let content = document.add_child(
        panel,
        UiNode::container(
            "glassworks.options.content",
            UiNodeStyle::new(layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(content_height),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(2.0),
            ))
            .with_clip(ClipBehavior::Clip),
        )
        .with_scroll(ScrollAxes::VERTICAL),
    );

    let file = add_options_section(
        document,
        content,
        "glassworks.options.section.file",
        "JSON file",
        ui_scale,
    );
    add_options_value_text(
        document,
        file,
        "glassworks.options.file.path",
        format!("Path: {}", app.options_file_path_for_display()),
        ui_scale,
    );
    add_options_button_row(
        document,
        file,
        "glassworks.options.file.actions",
        vec![
            (
                "glassworks.options.action.file.save".to_string(),
                "Save JSON".to_string(),
                false,
            ),
            (
                "glassworks.options.action.file.reload".to_string(),
                "Reload JSON".to_string(),
                false,
            ),
            (
                "glassworks.options.action.file.defaults".to_string(),
                "Defaults".to_string(),
                false,
            ),
        ],
        ui_scale,
    );

    let appearance = add_options_section(
        document,
        content,
        "glassworks.options.section.appearance",
        "Appearance",
        ui_scale,
    );
    add_options_button_row(
        document,
        appearance,
        "glassworks.options.appearance.theme",
        vec![
            (
                "glassworks.options.action.appearance.theme.dark".to_string(),
                "Dark".to_string(),
                app.app_options.appearance.theme == options::ThemePreference::Dark,
            ),
            (
                "glassworks.options.action.appearance.theme.light".to_string(),
                "Light".to_string(),
                app.app_options.appearance.theme == options::ThemePreference::Light,
            ),
            (
                "glassworks.options.action.appearance.theme.system".to_string(),
                "System".to_string(),
                app.app_options.appearance.theme == options::ThemePreference::System,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        appearance,
        "glassworks.options.appearance.scale",
        vec![
            (
                "glassworks.options.action.appearance.ui_scale.100".to_string(),
                "100%".to_string(),
                (app.app_options.appearance.ui_scale - 1.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.appearance.ui_scale.125".to_string(),
                "125%".to_string(),
                (app.app_options.appearance.ui_scale - 1.25).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.appearance.ui_scale.150".to_string(),
                "150%".to_string(),
                (app.app_options.appearance.ui_scale - 1.5).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.appearance.ui_scale.200".to_string(),
                "200%".to_string(),
                (app.app_options.appearance.ui_scale - 2.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.appearance.dense_mode".to_string(),
                "Dense".to_string(),
                app.app_options.appearance.dense_mode,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        appearance,
        "glassworks.options.appearance.units",
        UnitDisplay::ALL
            .iter()
            .map(|unit| {
                (
                    format!("glassworks.menu.item.display.units.{}", unit.slug()),
                    unit.label().to_string(),
                    app.unit_display == *unit,
                )
            })
            .collect(),
        ui_scale,
    );

    let shell = add_options_section(
        document,
        content,
        "glassworks.options.section.shell",
        "Shell and panels",
        ui_scale,
    );
    add_options_button_row(
        document,
        shell,
        "glassworks.options.shell.panels",
        vec![
            (
                "glassworks.menu.item.display.inspector".to_string(),
                "Details".to_string(),
                app.show_inspector,
            ),
            (
                "glassworks.menu.item.display.secondary_panel".to_string(),
                app.secondary_panel_label().to_string(),
                app.show_layers,
            ),
            (
                "glassworks.menu.item.view.sidebar_modules".to_string(),
                "Sidebar modules".to_string(),
                app.show_sidebar_modules,
            ),
            (
                "glassworks.menu.item.tools.diagnostics".to_string(),
                "Diagnostics".to_string(),
                app.show_diagnostics_panel,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        shell,
        "glassworks.options.shell.nav",
        vec![
            (
                "glassworks.sidebar.default".to_string(),
                "All nav".to_string(),
                false,
            ),
            (
                "glassworks.sidebar.none".to_string(),
                "No nav".to_string(),
                false,
            ),
            (
                "glassworks.options.action.shell.command_palette_on_start".to_string(),
                "Palette on start".to_string(),
                app.app_options.shell.show_command_palette_on_start,
            ),
        ],
        ui_scale,
    );
    add_options_button_rows(
        document,
        shell,
        "glassworks.options.shell.startup_view",
        StartupView::ALL
            .into_iter()
            .map(|view| {
                (
                    format!("glassworks.menu.item.view.{}", view.slug()),
                    view.nav_label().to_string(),
                    app.active_view == view,
                )
            })
            .collect(),
        4,
        ui_scale,
    );
    add_options_button_rows(
        document,
        shell,
        "glassworks.options.shell.nav_views",
        StartupView::ALL
            .into_iter()
            .map(|view| {
                (
                    format!("glassworks.sidebar.view.{}", view.slug()),
                    view.nav_label().to_string(),
                    app.nav_rail_views.contains(&view),
                )
            })
            .collect(),
        4,
        ui_scale,
    );

    let layout_section = add_options_section(
        document,
        content,
        "glassworks.options.section.layout",
        "2D layout editor",
        ui_scale,
    );
    add_options_button_row(
        document,
        layout_section,
        "glassworks.options.layout.display",
        vec![
            (
                "glassworks.menu.item.options.snap".to_string(),
                "Snap".to_string(),
                app.snap_enabled,
            ),
            (
                "glassworks.menu.item.display.grid2d".to_string(),
                "Grid".to_string(),
                app.show_grid,
            ),
            (
                "glassworks.menu.item.display.origin".to_string(),
                "Origin".to_string(),
                app.show_origin_marker,
            ),
            (
                "glassworks.menu.item.display.drc".to_string(),
                "DRC overlay".to_string(),
                app.show_drc_overlay,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        layout_section,
        "glassworks.options.layout.tools",
        ToolMode::ALL
            .iter()
            .map(|tool| {
                (
                    format!("glassworks.menu.item.tool.{}", tool.slug()),
                    tool.label().to_string(),
                    app.active_tool == *tool,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_rows(
        document,
        layout_section,
        "glassworks.options.layout.layers",
        app.workspace
            .document
            .layers
            .keys()
            .map(|layer| {
                (
                    format!("glassworks.options.action.layout.active_layer.{}", layer.0),
                    format!("L{}", layer.0),
                    app.active_layer == *layer,
                )
            })
            .collect(),
        4,
        ui_scale,
    );
    add_options_button_row(
        document,
        layout_section,
        "glassworks.options.layout.zoom",
        vec![
            (
                "glassworks.options.action.layout.zoom.tiny".to_string(),
                "Far".to_string(),
                (app.layout_zoom - 0.005).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.layout.zoom.default".to_string(),
                "Default".to_string(),
                (app.layout_zoom - 0.02).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.layout.zoom.close".to_string(),
                "Close".to_string(),
                (app.layout_zoom - 0.1).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.layout.pan.reset".to_string(),
                "Reset pan".to_string(),
                app.layout_pan == [0.0, 0.0],
            ),
            (
                "glassworks.options.action.layout.modifier.shift".to_string(),
                "Shift constrain".to_string(),
                app.app_options.layout.constrain_with_shift,
            ),
            (
                "glassworks.options.action.layout.modifier.ctrl".to_string(),
                "Ctrl bypass snap".to_string(),
                app.app_options.layout.bypass_snap_with_ctrl,
            ),
            (
                "glassworks.options.action.layout.modifier.alt".to_string(),
                "Alt duplicate".to_string(),
                app.app_options.layout.duplicate_drag_with_alt,
            ),
        ],
        ui_scale,
    );

    let viewport3d = add_options_section(
        document,
        content,
        "glassworks.options.section.viewport3d",
        "3D viewport",
        ui_scale,
    );
    add_options_button_row(
        document,
        viewport3d,
        "glassworks.options.viewport3d.display",
        vec![
            (
                "glassworks.menu.item.display.grid3d".to_string(),
                "3D grid".to_string(),
                app.show_3d_grid,
            ),
            (
                "glassworks.menu.item.tools.reset3d".to_string(),
                "Reset camera".to_string(),
                false,
            ),
            (
                "glassworks.menu.item.tools.fullscreen".to_string(),
                "Fullscreen".to_string(),
                app.viewport_fullscreen,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        viewport3d,
        "glassworks.options.viewport3d.camera",
        vec![
            (
                "glassworks.options.action.viewport3d.capture_flycam".to_string(),
                "Click capture".to_string(),
                app.app_options.viewport3d.capture_flycam_on_click,
            ),
            (
                "glassworks.options.action.viewport3d.speed.slow".to_string(),
                "Slow".to_string(),
                (app.camera_3d.speed - 1_000.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.speed.default".to_string(),
                "Default speed".to_string(),
                (app.camera_3d.speed - 4_000.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.speed.fast".to_string(),
                "Fast".to_string(),
                (app.camera_3d.speed - 12_000.0).abs() < f32::EPSILON,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        viewport3d,
        "glassworks.options.viewport3d.lens",
        vec![
            (
                "glassworks.options.action.viewport3d.fov.45".to_string(),
                "45 deg".to_string(),
                (app.app_options.viewport3d.fov_degrees - 45.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.fov.58".to_string(),
                "58 deg".to_string(),
                (app.app_options.viewport3d.fov_degrees - 58.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.fov.75".to_string(),
                "75 deg".to_string(),
                (app.app_options.viewport3d.fov_degrees - 75.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.sensitivity.low".to_string(),
                "Low look".to_string(),
                (app.app_options.viewport3d.mouse_sensitivity - 0.0015).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.sensitivity.default".to_string(),
                "Default look".to_string(),
                (app.app_options.viewport3d.mouse_sensitivity - 0.003).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.sensitivity.high".to_string(),
                "High look".to_string(),
                (app.app_options.viewport3d.mouse_sensitivity - 0.006).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.fast_multiplier.2".to_string(),
                "2x shift".to_string(),
                (app.app_options.viewport3d.fast_multiplier - 2.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.fast_multiplier.4".to_string(),
                "4x shift".to_string(),
                (app.app_options.viewport3d.fast_multiplier - 4.0).abs() < f32::EPSILON,
            ),
            (
                "glassworks.options.action.viewport3d.fast_multiplier.8".to_string(),
                "8x shift".to_string(),
                (app.app_options.viewport3d.fast_multiplier - 8.0).abs() < f32::EPSILON,
            ),
        ],
        ui_scale,
    );

    let performance = add_options_section(
        document,
        content,
        "glassworks.options.section.performance",
        "Performance",
        ui_scale,
    );
    add_options_button_row(
        document,
        performance,
        "glassworks.options.performance.live",
        vec![
            (
                "glassworks.options.action.performance.live_fps".to_string(),
                "Live FPS".to_string(),
                app.app_options.performance.live_fps_meter,
            ),
            (
                "glassworks.options.action.performance.idle_redraw".to_string(),
                "Idle redraw".to_string(),
                app.app_options.performance.idle_redraw_layout_viewports,
            ),
            (
                "glassworks.options.action.performance.dense_2d_lod".to_string(),
                "Dense 2D LOD".to_string(),
                app.app_options.performance.dense_2d_lod,
            ),
            (
                "glassworks.options.action.performance.dense_3d_instancing".to_string(),
                "3D instancing".to_string(),
                app.app_options.performance.dense_3d_instancing,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        performance,
        "glassworks.options.performance.caches",
        vec![
            (
                "glassworks.options.action.performance.cache_layout_index".to_string(),
                "Layout index".to_string(),
                app.app_options.performance.cache_layout_index,
            ),
            (
                "glassworks.options.action.performance.cache_drc".to_string(),
                "DRC reports".to_string(),
                app.app_options.performance.cache_drc_reports,
            ),
            (
                "glassworks.options.action.performance.cache_connectivity".to_string(),
                "Connectivity".to_string(),
                app.app_options.performance.cache_connectivity_reports,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        performance,
        "glassworks.options.performance.fps_alpha",
        vec![
            (
                "glassworks.options.action.performance.fps_alpha.10".to_string(),
                "Stable FPS".to_string(),
                (app.app_options.performance.fps_ema_alpha - 0.10).abs() < f64::EPSILON,
            ),
            (
                "glassworks.options.action.performance.fps_alpha.18".to_string(),
                "Balanced FPS".to_string(),
                (app.app_options.performance.fps_ema_alpha - 0.18).abs() < f64::EPSILON,
            ),
            (
                "glassworks.options.action.performance.fps_alpha.30".to_string(),
                "Responsive FPS".to_string(),
                (app.app_options.performance.fps_ema_alpha - 0.30).abs() < f64::EPSILON,
            ),
        ],
        ui_scale,
    );

    let files = add_options_section(
        document,
        content,
        "glassworks.options.section.files",
        "Files",
        ui_scale,
    );
    add_options_value_text(
        document,
        files,
        "glassworks.options.files.recent_summary",
        format!(
            "Recent files: {} / {}",
            app.app_options.files.recent_files.len(),
            app.app_options.files.recent_workspace_limit
        ),
        ui_scale,
    );
    for (index, file) in app
        .app_options
        .files
        .recent_files
        .iter()
        .take(3)
        .enumerate()
    {
        add_options_value_text(
            document,
            files,
            format!("glassworks.options.files.recent.{index}"),
            recent_file_menu_label(file),
            ui_scale,
        );
    }
    add_options_button_row(
        document,
        files,
        "glassworks.options.files.toggles",
        vec![
            (
                "glassworks.options.action.files.autosave".to_string(),
                "Autosave".to_string(),
                app.app_options.files.autosave_enabled,
            ),
            (
                "glassworks.options.action.files.remember_workspace".to_string(),
                "Remember workspace".to_string(),
                app.app_options.files.remember_last_workspace,
            ),
            (
                "glassworks.options.action.files.prompt_destructive".to_string(),
                "Destructive prompts".to_string(),
                app.app_options.files.prompt_before_destructive_actions,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        files,
        "glassworks.options.files.intervals",
        vec![
            (
                "glassworks.options.action.files.autosave_interval.60".to_string(),
                "60s save".to_string(),
                app.app_options.files.autosave_interval_seconds == 60,
            ),
            (
                "glassworks.options.action.files.autosave_interval.120".to_string(),
                "120s save".to_string(),
                app.app_options.files.autosave_interval_seconds == 120,
            ),
            (
                "glassworks.options.action.files.autosave_interval.300".to_string(),
                "300s save".to_string(),
                app.app_options.files.autosave_interval_seconds == 300,
            ),
            (
                "glassworks.options.action.files.recent_limit.5".to_string(),
                "5 recent".to_string(),
                app.app_options.files.recent_workspace_limit == 5,
            ),
            (
                "glassworks.options.action.files.recent_limit.10".to_string(),
                "10 recent".to_string(),
                app.app_options.files.recent_workspace_limit == 10,
            ),
            (
                "glassworks.options.action.files.recent_limit.25".to_string(),
                "25 recent".to_string(),
                app.app_options.files.recent_workspace_limit == 25,
            ),
        ],
        ui_scale,
    );

    add_domain_options_sections(document, content, app, ui_scale);
}

pub(crate) fn add_domain_options_sections(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let workflow = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.workflow",
        "Workflow defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        workflow,
        "glassworks.options.domain.workflow.toggles",
        vec![(
            "glassworks.options.action.domains.workflow.load_demo_on_start".to_string(),
            "Load sample".to_string(),
            app.app_options.domains.workflow.load_demo_on_start,
        )],
        ui_scale,
    );
    add_options_button_rows(
        document,
        workflow,
        "glassworks.options.domain.workflow.focus_lot",
        workflow_lot_ids(&app.workspace)
            .into_iter()
            .map(|lot_id| {
                (
                    format!("glassworks.viewctl.workflow.focus_lot.{lot_id}"),
                    workflow_lot_label(&app.workspace, lot_id.as_str(), 18),
                    app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
                )
            })
            .collect(),
        3,
        ui_scale,
    );

    let mask = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.mask",
        "Reticle prep defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        mask,
        "glassworks.options.domain.mask.severity",
        MaskIssueSeverityFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.mask.severity.{}", filter.slug()),
                    filter.label().to_string(),
                    app.mask_issue_severity_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        mask,
        "glassworks.options.domain.mask.grouping",
        MaskIssueGrouping::ALL
            .iter()
            .map(|grouping| {
                (
                    format!("glassworks.viewctl.mask.group.{}", grouping.slug()),
                    grouping.label().to_string(),
                    app.mask_issue_grouping == *grouping,
                )
            })
            .collect(),
        ui_scale,
    );

    let layout_diff = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.layout_diff",
        "Layout diff defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        layout_diff,
        "glassworks.options.domain.layout_diff.filters",
        vec![
            (
                "glassworks.viewctl.layout_diff.toggle_changed_only".to_string(),
                "Changed only".to_string(),
                app.layout_diff_changed_only,
            ),
            (
                "glassworks.viewctl.layout_diff.page_size.25".to_string(),
                "25 rows".to_string(),
                app.layout_diff_page_size == 25,
            ),
            (
                "glassworks.viewctl.layout_diff.page_size.50".to_string(),
                "50 rows".to_string(),
                app.layout_diff_page_size == 50,
            ),
            (
                "glassworks.viewctl.layout_diff.page_size.100".to_string(),
                "100 rows".to_string(),
                app.layout_diff_page_size == 100,
            ),
            (
                "glassworks.viewctl.layout_diff.page_size.200".to_string(),
                "200 rows".to_string(),
                app.layout_diff_page_size == 200,
            ),
        ],
        ui_scale,
    );

    let operations = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.operations",
        "Operations defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.inventory",
        InventoryQuickFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.inventory.filter.{}", filter.slug()),
                    filter.label().to_string(),
                    app.inventory_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.fab_control",
        vec![(
            "glassworks.options.action.domains.fab_control.auto_select_first_tool".to_string(),
            "Auto-select first tool".to_string(),
            app.app_options.domains.fab_control.auto_select_first_tool,
        )],
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.maintenance",
        MaintenanceWorkFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.maintenance.filter.{}", filter.slug()),
                    filter.label().to_string(),
                    app.maintenance_work_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.maintenance_history",
        MaintenanceHistoryFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.maintenance.history.{}", filter.slug()),
                    filter.label().to_string(),
                    app.maintenance_history_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.environment",
        vec![(
            "glassworks.options.action.domains.environment.alarm_sensors_first".to_string(),
            "Alarm sensors first".to_string(),
            app.app_options.domains.environment.show_alarm_sensors_first,
        )],
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.scheduler",
        vec![
            (
                format!(
                    "glassworks.viewctl.scheduler.policy.{}",
                    dispatch_policy_slug(DispatchPolicy::Fifo)
                ),
                "FIFO".to_string(),
                app.scheduler_policy == DispatchPolicy::Fifo,
            ),
            (
                format!(
                    "glassworks.viewctl.scheduler.policy.{}",
                    dispatch_policy_slug(DispatchPolicy::PriorityThenFifo)
                ),
                "Priority".to_string(),
                app.scheduler_policy == DispatchPolicy::PriorityThenFifo,
            ),
            (
                format!(
                    "glassworks.viewctl.scheduler.policy.{}",
                    dispatch_policy_slug(DispatchPolicy::DueDateThenPriority)
                ),
                "Due date".to_string(),
                app.scheduler_policy == DispatchPolicy::DueDateThenPriority,
            ),
            (
                "glassworks.viewctl.scheduler.toggle_conflicts".to_string(),
                "Conflicts".to_string(),
                app.scheduler_conflicts_only,
            ),
            (
                "glassworks.viewctl.scheduler.toggle_focus_tool".to_string(),
                "Focus tool".to_string(),
                app.scheduler_focus_selected_tool,
            ),
        ],
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.scheduler_priority",
        [0_u8, 1, 2, 3, 4, 5]
            .into_iter()
            .map(|priority| {
                (
                    format!("glassworks.viewctl.scheduler.priority.{priority}"),
                    format!("P{priority}+"),
                    app.scheduler_min_priority == priority,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        operations,
        "glassworks.options.domain.safety",
        vec![(
            "glassworks.options.action.domains.safety.show_acknowledged".to_string(),
            "Show acknowledged safety".to_string(),
            app.app_options.domains.safety.show_acknowledged,
        )],
        ui_scale,
    );
    let analysis = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.analysis",
        "Analysis defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.trace",
        TraceImpactMode::ALL
            .iter()
            .map(|mode| {
                (
                    format!("glassworks.viewctl.trace.impact.{}", mode.slug()),
                    mode.label().to_string(),
                    app.trace_impact_mode == *mode,
                )
            })
            .chain(std::iter::once((
                "glassworks.viewctl.trace.toggle_related".to_string(),
                "Related only".to_string(),
                app.trace_related_only,
            )))
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.metrology.mode",
        MetrologyMapMode::ALL
            .iter()
            .map(|mode| {
                (
                    format!("glassworks.viewctl.metrology.mode.{}", mode.slug()),
                    mode.short_label().to_string(),
                    app.metrology_map_mode == *mode,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.metrology.kind",
        MeasurementKind::NUMERIC
            .iter()
            .map(|kind| {
                (
                    format!(
                        "glassworks.viewctl.metrology.kind.{}",
                        measurement_kind_slug(*kind)
                    ),
                    kind.label().to_string(),
                    app.metrology_kind == *kind,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.metrology.filters",
        vec![(
            "glassworks.viewctl.metrology.failed_only".to_string(),
            "Failed only".to_string(),
            app.metrology_failed_only,
        )],
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.yield",
        YieldMapFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.yield.filter.{}", filter.slug()),
                    filter.label().to_string(),
                    app.yield_map_filter == *filter,
                )
            })
            .chain([
                (
                    "glassworks.viewctl.yield.attention".to_string(),
                    "Attention".to_string(),
                    app.show_only_attention_wafers,
                ),
                (
                    "glassworks.viewctl.yield.excursions".to_string(),
                    "Excursions".to_string(),
                    app.show_only_excursions,
                ),
            ])
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.spc.source",
        SpcSourceFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.spc.source.{}", filter.slug()),
                    filter.label().to_string(),
                    app.spc_source_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );

    add_options_button_row(
        document,
        analysis,
        "glassworks.options.domain.spc.severity",
        SpcSeverityFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.spc.severity.{}", filter.slug()),
                    filter.label().to_string(),
                    app.spc_severity_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    let monitor = spc_fdc_monitor(&app.workspace);
    let mut spc_context_buttons = vec![(
        "glassworks.viewctl.spc.clear_context".to_string(),
        "Any context".to_string(),
        app.spc_context_filter.is_empty(),
    )];
    spc_context_buttons.extend(monitor.charts.iter().take(3).map(|chart| {
        (
            format!("glassworks.viewctl.spc.chart.{}", chart.id),
            compact_button_label(&chart.name, 18),
            app.spc_context_filter == chart.id.as_str(),
        )
    }));
    spc_context_buttons.extend(monitor.traces.iter().take(2).map(|trace| {
        (
            format!("glassworks.viewctl.spc.trace.{}", trace.id),
            compact_button_label(&trace.display_name(), 18),
            app.spc_context_filter == trace.id.as_str(),
        )
    }));
    add_options_button_rows(
        document,
        analysis,
        "glassworks.options.domain.spc.context",
        spc_context_buttons,
        3,
        ui_scale,
    );

    let engineering = add_options_section(
        document,
        parent,
        "glassworks.options.section.domain.engineering",
        "Engineering defaults",
        ui_scale,
    );
    add_options_button_row(
        document,
        engineering,
        "glassworks.options.domain.process_flow",
        ProcessFlowNodeFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.process_flow.filter.{}", filter.slug()),
                    filter.label().to_string(),
                    app.process_flow_filter == *filter,
                )
            })
            .chain(std::iter::once((
                "glassworks.viewctl.process_flow.toggle_errors".to_string(),
                "Errors only".to_string(),
                app.process_flow_errors_only,
            )))
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        engineering,
        "glassworks.options.domain.run_to_run",
        ControlLoopFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!(
                        "glassworks.viewctl.process_control.filter.{}",
                        filter.slug()
                    ),
                    filter.label().to_string(),
                    app.process_control_loop_filter == *filter,
                )
            })
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        engineering,
        "glassworks.options.domain.cross_section",
        vec![
            (
                "glassworks.viewctl.cross_section.toggle_mask".to_string(),
                "Mask".to_string(),
                app.cross_section_show_mask,
            ),
            (
                "glassworks.viewctl.cross_section.toggle_dimensions".to_string(),
                "Dimensions".to_string(),
                app.cross_section_show_dimensions,
            ),
            (
                "glassworks.viewctl.cross_section.toggle_risks".to_string(),
                "Risks".to_string(),
                app.cross_section_show_risks,
            ),
            (
                "glassworks.viewctl.notebook.preview".to_string(),
                "Notebook preview".to_string(),
                app.notebook_preview_mode,
            ),
            (
                "glassworks.viewctl.notebook.toggle_followups".to_string(),
                "Follow-ups".to_string(),
                app.notebook_followups_only,
            ),
        ],
        ui_scale,
    );
    add_options_button_rows(
        document,
        engineering,
        "glassworks.options.domain.cross_section_step",
        (0..=app.workspace.cross_section.steps.len().min(6))
            .map(|step| {
                (
                    format!("glassworks.viewctl.cross_section.step.{step}"),
                    format!("Step {step}"),
                    app.cross_section_step == step,
                )
            })
            .collect(),
        4,
        ui_scale,
    );
    add_options_button_rows(
        document,
        engineering,
        "glassworks.options.domain.notebook_tags",
        std::iter::once((
            "glassworks.viewctl.notebook.tag.all".to_string(),
            "All notes".to_string(),
            app.notebook_tag_filter.is_none(),
        ))
        .chain(app.workspace.lab_notebook.tags().into_iter().map(|tag| {
            (
                format!("glassworks.viewctl.notebook.tag.{tag}"),
                compact_button_label(&tag, 18),
                app.notebook_tag_filter.as_deref() == Some(tag.as_str()),
            )
        }))
        .collect(),
        4,
        ui_scale,
    );
    add_options_button_row(
        document,
        engineering,
        "glassworks.options.domain.experiment",
        ExperimentRunFilter::ALL
            .iter()
            .map(|filter| {
                (
                    format!("glassworks.viewctl.experiment.filter.{}", filter.slug()),
                    filter.short_label().to_string(),
                    app.experiment_run_filter == *filter,
                )
            })
            .chain(std::iter::once((
                "glassworks.viewctl.experiment.pending_only".to_string(),
                "Missing only".to_string(),
                app.experiment_show_missing_only,
            )))
            .collect(),
        ui_scale,
    );
    add_options_button_row(
        document,
        engineering,
        "glassworks.options.domain.experiment_capture",
        vec![
            (
                "glassworks.viewctl.experiment.use_demo".to_string(),
                "Sample value".to_string(),
                false,
            ),
            (
                "glassworks.viewctl.experiment.use_target".to_string(),
                "Target value".to_string(),
                false,
            ),
            (
                "glassworks.viewctl.experiment.capture_next_demo".to_string(),
                "Next sample".to_string(),
                false,
            ),
        ],
        ui_scale,
    );
}

pub(crate) fn add_options_section(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    title: impl Into<String>,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let name = name.into();
    let title = title.into();
    let section = document.add_child(
        parent,
        UiNode::container(
            name.clone(),
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::auto()),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_ALT,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_text(
        document,
        section,
        format!("{name}.title"),
        title,
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    section
}

pub(crate) fn add_options_value_text(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    value: impl Into<String>,
    ui_scale: UiScale,
) {
    add_text(
        document,
        parent,
        name,
        value,
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
}

pub(crate) fn add_options_button_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    buttons: Vec<(String, String, bool)>,
    ui_scale: UiScale,
) {
    if buttons.is_empty() {
        return;
    }
    let row = document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_gap_all(
                layout::with_size(layout::row(), layout::percent(1.0), layout::auto()),
                ui_scale.value(4.0),
            ),
        ),
    );
    for (name, label, selected) in buttons {
        add_button(
            document,
            row,
            options_panel_action_node_name(&name),
            label,
            selected,
            layout::with_flex(layout::row(), 1.0, 1.0, layout::px(ui_scale.value(30.0))),
            ui_scale,
        );
    }
}

pub(crate) fn options_panel_action_node_name(action_name: &str) -> String {
    if action_name.starts_with("glassworks.options.") {
        action_name.to_string()
    } else {
        format!("glassworks.options.proxy.{action_name}")
    }
}

pub(crate) fn add_options_button_rows(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    base_name: impl Into<String>,
    buttons: Vec<(String, String, bool)>,
    per_row: usize,
    ui_scale: UiScale,
) {
    let base_name = base_name.into();
    for (index, chunk) in buttons.chunks(per_row.max(1)).enumerate() {
        add_options_button_row(
            document,
            parent,
            format!("{base_name}.{index}"),
            chunk.to_vec(),
            ui_scale,
        );
    }
}
