#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ViewControlButton {
    pub(crate) name: String,
    pub(crate) label: String,
    pub(crate) selected: bool,
}

impl ViewControlButton {
    pub(crate) fn new(name: impl Into<String>, label: impl Into<String>, selected: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            selected,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PrimaryRow {
    pub(crate) name: String,
    pub(crate) title: String,
    pub(crate) value: String,
    pub(crate) detail: String,
    pub(crate) action: Option<String>,
    pub(crate) selected: bool,
}

impl PrimaryRow {
    pub(crate) fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        value: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            value: value.into(),
            detail: detail.into(),
            action: None,
            selected: false,
        }
    }

    pub(crate) fn action(mut self, action: impl Into<String>, selected: bool) -> Self {
        self.action = Some(action.into());
        self.selected = selected;
        self
    }

    pub(crate) fn with_detail_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        if !suffix.is_empty() {
            self.detail = format!("{}; {}", self.detail, suffix);
        }
        self
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DomainSection {
    pub(crate) name: String,
    pub(crate) title: String,
    pub(crate) empty: String,
    pub(crate) rows: Vec<PrimaryRow>,
    pub(crate) max_rows: usize,
}

impl DomainSection {
    pub(crate) fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        empty: impl Into<String>,
        rows: Vec<PrimaryRow>,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            empty: empty.into(),
            rows,
            max_rows: 6,
        }
    }

    pub(crate) fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = max_rows;
        self
    }
}

pub(crate) fn domain_section_key(view: StartupView, section_index: usize) -> String {
    format!("{}.domain.{section_index}", view.slug())
}

pub(crate) fn domain_section_expanded(app: &GlassworksApp, section_index: usize) -> bool {
    let default_expanded = section_index == 0;
    let toggled = app
        .collapsed_detail_sections
        .contains(&domain_section_key(app.active_view, section_index));
    default_expanded ^ toggled
}

pub(crate) fn add_domain_section(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    section_index: usize,
    section: &DomainSection,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let key = domain_section_key(app.active_view, section_index);
    let expanded = domain_section_expanded(app, section_index);
    let row_count = if section.rows.is_empty() {
        0
    } else {
        section.rows.len().min(section.max_rows)
    };
    let title = if row_count == 0 {
        format!("{} - empty", section.title)
    } else {
        format!("{} - {row_count}", section.title)
    };
    let mut options = CollapsingHeaderOptions::default()
        .expanded(expanded)
        .with_toggle_action(format!("glassworks.inspector.section.toggle.{key}"));
    options.layout = LayoutStyle::column().with_width_percent(1.0);
    options.header_layout = layout::with_padding_all(
        layout::with_gap_all(
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(24.0)),
            ),
            ui_scale.value(4.0),
        ),
        ui_scale.value(3.0),
    );
    options.body_layout = layout::with_gap_all(
        layout::with_padding_all(layout::column(), ui_scale.value(3.0)),
        ui_scale.value(3.0),
    );
    options.header_visual = UiVisual::panel(
        COLOR_PANEL_BG,
        Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
        ui_scale.value(2.0),
    );
    options.hovered_visual = UiVisual::panel(
        COLOR_PANEL_ALT,
        Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
        ui_scale.value(2.0),
    );
    options.pressed_visual = UiVisual::panel(
        COLOR_BUTTON_SELECTED,
        Some(StrokeStyle::new(
            COLOR_BUTTON_STROKE_SELECTED,
            ui_scale.value(1.0),
        )),
        ui_scale.value(2.0),
    );
    options.body_visual = UiVisual::panel(COLOR_APP_BG, None, 0.0);
    options.text_style = text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT);
    options.indicator_text_style =
        text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED);
    options.accessibility_label = Some(section.title.clone());

    let nodes = collapsing_header(
        document,
        parent,
        format!("glassworks.domain.section.{section_index}"),
        title,
        options,
    );
    if let Some(body) = nodes.body {
        if section.rows.is_empty() {
            let empty_row = PrimaryRow::new(
                format!("{}.empty", section.name),
                section.title.clone(),
                section.empty.clone(),
                "",
            );
            add_primary_section_row(
                document,
                body,
                &section.name,
                section_index,
                0,
                &empty_row,
                ui_scale,
            );
        } else {
            for (row_index, row) in section.rows.iter().take(section.max_rows).enumerate() {
                add_primary_section_row(
                    document,
                    body,
                    &section.name,
                    section_index,
                    row_index,
                    row,
                    ui_scale,
                );
            }
        }
    }
}

pub(crate) fn add_primary_section_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    section_name: &str,
    section_index: usize,
    row_index: usize,
    row_data: &PrimaryRow,
    ui_scale: UiScale,
) {
    let row_name = format!("{section_name}.row.{row_index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        )
        .with_visual(UiVisual::panel(
            if row_index % 2 == 0 {
                COLOR_PANEL_ALT
            } else {
                COLOR_PANEL_BG
            },
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    if let Some(action) = row_data.action.as_ref() {
        let button_name = action
            .strip_prefix("glassworks.viewctl.")
            .map(|action| {
                format!("glassworks.primary.action.domain{section_index}_{row_index}.{action}")
            })
            .unwrap_or_else(|| format!("{row_name}.action.{}", row_data.name));
        add_button(
            document,
            row,
            button_name,
            compact_button_label(&row_data.title, 24),
            row_data.selected,
            primary_cell_layout(1.4),
            ui_scale,
        );
    } else {
        add_primary_cell(
            document,
            row,
            format!("{row_name}.title.{}", row_data.name),
            compact_button_label(&row_data.title, 26),
            1.4,
            false,
            ui_scale,
        );
    }
    add_primary_cell(
        document,
        row,
        format!("{row_name}.value.{}", row_data.name),
        compact_button_label(&row_data.value, 22),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        row,
        format!("{row_name}.detail.{}", row_data.name),
        compact_button_label(&row_data.detail, 72),
        2.4,
        false,
        ui_scale,
    );
}

pub(crate) fn dashboard_metric_rows(
    prefix: &str,
    metrics: Vec<DashboardMetric>,
) -> Vec<PrimaryRow> {
    metrics
        .into_iter()
        .enumerate()
        .map(|(index, metric)| {
            PrimaryRow::new(
                format!("{prefix}.metric.{index}"),
                metric.label,
                metric.value,
                metric.detail,
            )
        })
        .collect()
}

pub(crate) fn workflow_lot_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    workflow_primary_rows(app)
}

pub(crate) fn workflow_focus_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    workflow_primary_rows(app)
}

pub(crate) fn workflow_spine_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    workflow_cross_link_rows(app)
}

pub(crate) fn fab_tool_rows(app: &GlassworksApp) -> Vec<PrimaryRow> {
    fab_primary_rows(app)
}

pub(crate) fn add_domain_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    _name: &str,
    subtitle: &str,
    title: &str,
    sections: Vec<DomainSection>,
    ui_scale: UiScale,
    _compact_rows: bool,
) {
    let section_markers = sections.clone();
    let expanded_rows = sections
        .iter()
        .enumerate()
        .filter(|(section_index, _)| domain_section_expanded(app, *section_index))
        .map(|(_, section)| {
            if section.rows.is_empty() {
                1
            } else {
                section.rows.len().min(section.max_rows)
            }
        })
        .sum::<usize>();
    let title_height = ui_scale.value(24.0);
    let subtitle_height = ui_scale.value(18.0);
    let header_height = ui_scale.value(24.0);
    let row_height = ui_scale.value(30.0);
    let gap = ui_scale.value(4.0);
    let panel_height = (ui_scale.value(18.0)
        + title_height
        + subtitle_height
        + sections.len() as f32 * header_height
        + expanded_rows as f32 * row_height
        + sections.len().saturating_sub(1) as f32 * gap
        + ui_scale.value(16.0))
    .max(ui_scale.value(220.0));

    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(panel_height),
                    ),
                    gap,
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            COLOR_PANEL_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            ui_scale.value(2.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.primary.title",
        title,
        text_style(ui_scale.value(15.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(title_height)),
    );
    add_text(
        document,
        panel,
        "glassworks.primary.subtitle",
        subtitle,
        text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(subtitle_height)),
    );

    for (section_index, section) in sections.iter().enumerate() {
        add_domain_section(document, panel, section_index, section, app, ui_scale);
    }

    for (section_index, section) in section_markers.iter().enumerate() {
        if section.name.ends_with(".controls") || section.name.ends_with(".commands") {
            continue;
        }
        add_node_marker(document, parent, section.name.clone());
        if !section.rows.is_empty() && !domain_section_expanded(app, section_index) {
            add_node_marker(document, parent, format!("{}.row.0", section.name));
        }
    }
}

pub(crate) fn add_view_controls(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
    body_width: f32,
) {
    match app.active_view {
        StartupView::Layout2d | StartupView::Layout3d => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.layout",
            "Layout Editor Controls",
            layout_editor_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::Workflow => add_control_panel_with_button_width(
            document,
            parent,
            "glassworks.viewctl.workflow",
            "Workflow Controls",
            workflow_control_rows(app),
            116.0,
            ui_scale,
            body_width,
        ),
        StartupView::MaskPrep => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.mask",
            "Reticle Prep Controls",
            mask_prep_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::LayoutDiff => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.layout_diff",
            "Layout Diff Controls",
            layout_diff_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::FabControl => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.fab",
            "Fab Control",
            fab_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::Maintenance => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.maintenance",
            "Maintenance Controls",
            maintenance_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::Environment => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.environment",
            "Environment Controls",
            environment_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::Inventory => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.inventory",
            "Inventory Controls",
            inventory_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::Scheduler => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.scheduler",
            "Dispatch Controls",
            scheduler_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::Safety => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.safety",
            "Safety Controls",
            safety_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::Traceability => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.trace",
            "Traceability Controls",
            traceability_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::ProcessFlow => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.process_flow",
            "Process Flow Controls",
            process_flow_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::ProcessControl => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.process_control",
            "Run-to-Run Controls",
            process_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::SpcFdc => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.spc",
            "SPC / FDC Controls",
            spc_fdc_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::CrossSection => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.cross_section",
            "Cross-Section Controls",
            cross_section_control_rows(app),
            ui_scale,
            body_width,
        ),
        StartupView::Metrology => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.metrology",
            "Metrology Controls",
            metrology_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::Yield => add_control_panel(
            document,
            parent,
            "glassworks.viewctl.yield",
            "Yield Controls",
            yield_control_rows(app, compact_rows),
            ui_scale,
            body_width,
        ),
        StartupView::Experiment => add_control_panel_with_button_width(
            document,
            parent,
            "glassworks.viewctl.experiment",
            "DOE Controls",
            experiment_control_rows(app, compact_rows),
            132.0,
            ui_scale,
            body_width,
        ),
        StartupView::Notebook => add_control_panel_with_button_width(
            document,
            parent,
            "glassworks.viewctl.notebook",
            "Notebook Controls",
            notebook_control_rows(app, compact_rows),
            132.0,
            ui_scale,
            body_width,
        ),
    }
}

pub(crate) fn add_layout_canvas_mode_hud(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    active_view: StartupView,
    ui_scale: UiScale,
) {
    let label = match active_view {
        StartupView::Layout2d => {
            "2D Layout | wheel zoom | middle/right drag pan | Shift constrain | Ctrl bypass snap"
        }
        StartupView::Layout3d => {
            "3D Stack | click to capture flycam | mouse look | WASD move | Q/E down/up | Esc release"
        }
        _ => return,
    };
    document.add_child(
        parent,
        UiNode::scene(
            "glassworks.layout.mode_hud",
            vec![ScenePrimitive::Text(PaintText::new(
                label,
                UiRect::new(
                    ui_scale.value(8.0),
                    ui_scale.value(4.0),
                    ui_scale.value(680.0),
                    ui_scale.value(18.0),
                ),
                text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            ))],
            layout::with_absolute_position(
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
                0.0,
                0.0,
            ),
        ),
    );
}

pub(crate) fn add_layout_canvas_fps(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    preview_height: f32,
    ui_scale: UiScale,
) {
    add_text(
        document,
        parent,
        "glassworks.layout.fps",
        format_fps_label(app.layout_fps_frame_ms()),
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::with_absolute_position(
            layout::size(
                layout::px(ui_scale.value(96.0)),
                layout::px(ui_scale.value(18.0)),
            ),
            ui_scale.value(8.0),
            (preview_height - ui_scale.value(24.0)).max(0.0),
        ),
    );
}

pub(crate) fn add_primary_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    viewport: UiSize,
    ui_scale: UiScale,
    compact_rows: bool,
    wide_rows: bool,
    body_width: f32,
) {
    match app.active_view {
        StartupView::Layout2d | StartupView::Layout3d => {
            let strip_height = tool_strip_height(app.active_view, viewport.width, ui_scale);
            let preview_height =
                (viewport.height - ui_scale.value(30.0) - strip_height - ui_scale.value(70.0))
                    .max(ui_scale.value(360.0));
            let layout_inspector_width = if body_width >= ui_scale.value(760.0) {
                ui_scale.value(270.0)
            } else {
                0.0
            };
            let layout_workspace_gap = if layout_inspector_width > 0.0 {
                ui_scale.value(8.0)
            } else {
                0.0
            };
            let preview_width =
                (body_width - layout_inspector_width - layout_workspace_gap - ui_scale.value(20.0))
                    .max(ui_scale.value(1.0));
            let workspace = document.add_child(
                parent,
                UiNode::container(
                    "glassworks.layout.workspace",
                    UiNodeStyle::new(fixed_height_layout(
                        layout::with_size(
                            layout::with_gap_all(layout::row(), layout_workspace_gap),
                            layout::percent(1.0),
                            layout::px(preview_height),
                        ),
                        preview_height,
                    ))
                    .with_clip(ClipBehavior::Clip),
                ),
            );
            if layout_inspector_width > 0.0 {
                add_layout_inspector_rail(
                    document,
                    workspace,
                    app,
                    layout_inspector_width,
                    preview_height,
                    ui_scale,
                );
            }
            let viewport_stack = document.add_child(
                workspace,
                UiNode::container(
                    "glassworks.layout.viewport_stack",
                    UiNodeStyle::new(fixed_row_child_layout(
                        layout::with_size(
                            layout::column(),
                            layout::px(preview_width),
                            layout::px(preview_height),
                        ),
                        preview_width,
                        preview_height,
                    ))
                    .with_clip(ClipBehavior::Clip),
                )
                .with_visual(UiVisual::panel(
                    COLOR_APP_BG,
                    Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
                    ui_scale.value(2.0),
                )),
            );
            let canvas_key = if app.active_view == StartupView::Layout3d {
                "glassworks.layout.viewport.3d"
            } else {
                "glassworks.layout.viewport.2d"
            };
            let canvas_layout = layout::with_absolute_position(
                layout::with_size(layout::row(), layout::percent(1.0), layout::percent(1.0)),
                0.0,
                0.0,
            );
            let canvas_node =
                UiNode::gpu_canvas("glassworks.layout.preview", canvas_key, canvas_layout);
            let canvas_content = UiContent::Canvas(
                operad::CanvasContent::from_context(operad::CanvasContextDescriptor::gpu_texture(
                    canvas_key,
                ))
                .interaction(CanvasInteractionPolicy::EDITOR),
            );
            let canvas_label = if app.active_view == StartupView::Layout3d {
                "3D layout viewport"
            } else {
                "2D layout viewport"
            };
            let canvas_id = document.add_child(
                viewport_stack,
                canvas_node.with_accessibility(
                    AccessibilityMeta::new(AccessibilityRole::EditorSurface)
                        .label(canvas_label)
                        .focusable()
                        .action(AccessibilityAction::new("interact", "Interact")),
                ),
            );
            document.set_node_content(canvas_id, canvas_content);
            if app.active_view == StartupView::Layout2d {
                document.add_child(
                    viewport_stack,
                    UiNode::scene(
                        "glassworks.layout.overlay",
                        layout_overlay_primitives(
                            app,
                            UiSize::new(preview_width, preview_height),
                            ui_scale,
                        ),
                        layout::with_absolute_position(
                            layout::size(layout::percent(1.0), layout::percent(1.0)),
                            0.0,
                            0.0,
                        ),
                    ),
                );
            }
            add_layout_canvas_mode_hud(document, viewport_stack, app.active_view, ui_scale);
            if app.app_options.performance.live_fps_meter {
                add_layout_canvas_fps(document, viewport_stack, app, preview_height, ui_scale);
            }
        }
        StartupView::Workflow => {
            add_workflow_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::MaskPrep => add_mask_view_panel(document, parent, app, ui_scale, compact_rows),
        StartupView::LayoutDiff => {
            add_layout_diff_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::FabControl => {
            add_fab_control_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Maintenance => add_maintenance_view_panel(
            document,
            parent,
            app,
            ui_scale,
            compact_rows,
            wide_rows,
            body_width,
        ),
        StartupView::Inventory => {
            add_inventory_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Safety => add_safety_view_panel(document, parent, app, ui_scale, compact_rows),
        StartupView::Traceability => {
            add_traceability_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::CrossSection => {
            add_cross_section_view_panel(document, parent, app, ui_scale, compact_rows, body_width)
        }
        StartupView::ProcessFlow => {
            add_process_flow_view_panel(document, parent, app, ui_scale, compact_rows, body_width)
        }
        StartupView::Scheduler => add_scheduler_view_panel(document, parent, app, ui_scale),
        StartupView::Environment => {
            add_environment_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Metrology => {
            add_metrology_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Yield => add_yield_view_panel(document, parent, app, ui_scale, compact_rows),
        StartupView::SpcFdc => {
            add_spc_fdc_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Experiment => {
            add_experiment_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::ProcessControl => {
            add_process_control_view_panel(document, parent, app, ui_scale, compact_rows)
        }
        StartupView::Notebook => {
            add_notebook_view_panel(document, parent, app, ui_scale, compact_rows)
        }
    }
}
