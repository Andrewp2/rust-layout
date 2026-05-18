#![allow(unused_imports)]
use super::*;

pub(crate) fn add_inspector_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let sections = view_detail_sections(app)
        .into_iter()
        .filter(|section| section.title != "Current Controls")
        .collect::<Vec<_>>();
    let title = if matches!(
        app.active_view,
        StartupView::Layout2d | StartupView::Layout3d
    ) {
        ""
    } else {
        app.active_view.label()
    };
    add_sectioned_side_panel(
        document,
        parent,
        "glassworks.inspector",
        title,
        app,
        sections,
        inspector_panel_width(ui_scale),
        ui_scale,
    );
}

pub(crate) fn add_layout_layers_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let width = secondary_panel_width(ui_scale);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.secondary",
            UiNodeStyle::new(layout::with_min_size(
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width),
                            layout::percent(1.0),
                        ),
                        ui_scale.value(8.0),
                    ),
                    ui_scale.value(10.0),
                ),
                layout::px(0.0),
                layout::px(0.0),
            ))
            .with_clip(ClipBehavior::Clip),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    add_text(
        document,
        panel,
        "glassworks.secondary.title",
        app.secondary_panel_label(),
        text_style(ui_scale.value(16.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
    );
    if app.active_view == StartupView::Layout3d {
        add_layout_3d_stack_panel_rows(document, panel, app, ui_scale);
        add_text(
            document,
            panel,
            "glassworks.secondary.layers.title",
            "Layers",
            text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
    }
    add_button(
        document,
        panel,
        "glassworks.viewctl.layout.add_layer",
        "Add layer",
        false,
        layout::size(
            layout::px(ui_scale.value(88.0)),
            layout::px(ui_scale.value(28.0)),
        ),
        ui_scale,
    );

    let mut layers = app.workspace.document.layers.values().collect::<Vec<_>>();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    let layer_usage = layout_layer_usage_counts(&app.workspace.document);
    let used_layer_count = layers
        .iter()
        .filter(|layer| layer_usage.get(&layer.id).copied().unwrap_or(0) > 0)
        .count();
    let filtered_layers = layers
        .iter()
        .copied()
        .filter(|layer| {
            app.layout_layer_group_filter.matches(layer.process)
                && app
                    .layout_layer_usage_filter
                    .matches(layer_usage.get(&layer.id).copied().unwrap_or(0))
        })
        .collect::<Vec<_>>();
    add_text(
        document,
        panel,
        "glassworks.layers.usage.summary",
        format!("Used layers: {used_layer_count} / {}", layers.len()),
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    add_text(
        document,
        panel,
        "glassworks.layers.group.summary",
        format!(
            "Layer group: {} ({} / {})",
            app.layout_layer_group_filter.path_label(),
            filtered_layers.len(),
            layers.len()
        ),
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    add_text(
        document,
        panel,
        "glassworks.layers.usage_filter.summary",
        format!(
            "Layer rows: {} ({} / {})",
            app.layout_layer_usage_filter.label(),
            filtered_layers.len(),
            layers.len()
        ),
        text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    let usage_filter_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.usage_filter.controls",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for filter in LayoutLayerUsageFilter::ALL {
        let width = match filter {
            LayoutLayerUsageFilter::All => 44.0,
            LayoutLayerUsageFilter::Used => 54.0,
            LayoutLayerUsageFilter::Empty => 62.0,
        };
        add_button(
            document,
            usage_filter_row,
            format!(
                "glassworks.viewctl.layout.layer_usage_filter.{}",
                filter.slug()
            ),
            filter.label(),
            app.layout_layer_usage_filter == filter,
            layout::size(
                layout::px(ui_scale.value(width)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    let group_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.group.controls",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for group in LayoutLayerGroupFilter::PRIMARY {
        let width = match group {
            LayoutLayerGroupFilter::All => 44.0,
            LayoutLayerGroupFilter::FrontEnd => 50.0,
            LayoutLayerGroupFilter::Routing => 64.0,
            LayoutLayerGroupFilter::Annotation => 46.0,
            _ => 58.0,
        };
        add_button(
            document,
            group_row,
            format!("glassworks.viewctl.layout.layer_group.{}", group.slug()),
            group.label(),
            app.layout_layer_group_filter == group,
            layout::size(
                layout::px(ui_scale.value(width)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    for (parent_group, row_name) in [
        (
            LayoutLayerGroupFilter::FrontEnd,
            "glassworks.layers.group.feol_children",
        ),
        (
            LayoutLayerGroupFilter::Routing,
            "glassworks.layers.group.routing_children",
        ),
    ] {
        let child_row = document.add_child(
            panel,
            UiNode::container(
                row_name,
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        for group in parent_group.child_groups() {
            let (label, width) = match group {
                LayoutLayerGroupFilter::FrontEndDiffusion => ("Diff", 52.0),
                LayoutLayerGroupFilter::FrontEndGate => ("Gate", 54.0),
                LayoutLayerGroupFilter::FrontEndDielectric => ("Ox", 42.0),
                LayoutLayerGroupFilter::RoutingContact => ("Contact", 74.0),
                LayoutLayerGroupFilter::RoutingMetal => ("Metal", 58.0),
                LayoutLayerGroupFilter::RoutingVia => ("Via", 42.0),
                _ => (group.label(), 58.0),
            };
            add_button(
                document,
                child_row,
                format!("glassworks.viewctl.layout.layer_group.{}", group.slug()),
                label,
                app.layout_layer_group_filter == *group,
                layout::size(
                    layout::px(ui_scale.value(width)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
    }
    add_text(
        document,
        panel,
        "glassworks.layers.group_tree.title",
        "Layer Group Tree",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for (index, (label, value)) in layout_layer_group_directory_rows(app)
        .into_iter()
        .skip(1)
        .take(10)
        .enumerate()
    {
        add_text(
            document,
            panel,
            format!("glassworks.layers.group_tree.row.{index}"),
            format!("{label}: {}", compact_button_label(&value, 42)),
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
        );
    }
    let visibility_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.visibility.presets",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for (slug, label, width) in [
        ("show_all", "All On", 60.0),
        ("show_used", "Used On", 70.0),
        ("hide_empty", "Empty Off", 78.0),
    ] {
        add_button(
            document,
            visibility_row,
            format!("glassworks.viewctl.layout.layer_visibility.{slug}"),
            label,
            false,
            layout::size(
                layout::px(ui_scale.value(width)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    let isolate_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.visibility.isolate",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_button(
        document,
        isolate_row,
        "glassworks.viewctl.layout.layer_visibility.isolate_active",
        "Solo Active",
        false,
        layout::size(
            layout::px(ui_scale.value(104.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    add_button(
        document,
        isolate_row,
        "glassworks.viewctl.layout.layer_visibility.invert",
        "Invert",
        false,
        layout::size(
            layout::px(ui_scale.value(72.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    let cleanup_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.cleanup",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_button(
        document,
        cleanup_row,
        "glassworks.viewctl.layout.layer_cleanup.delete_empty",
        "Prune Empty",
        false,
        layout::size(
            layout::px(ui_scale.value(108.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    add_text(
        document,
        panel,
        "glassworks.layers.technology.title",
        "Technology Stack",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    for (index, (label, value)) in layout_technology_stack_rows(app)
        .into_iter()
        .take(10)
        .enumerate()
    {
        add_text(
            document,
            panel,
            format!("glassworks.layers.technology.row.{index}"),
            format!("{label}: {}", compact_button_label(&value, 48)),
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
        );
    }
    if !app.active_layout_technology().connectivity.is_empty() {
        let link_row = document.add_child(
            panel,
            UiNode::container(
                "glassworks.layers.technology.connectivity_links",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        for index in 0..app.active_layout_technology().connectivity.len().min(3) {
            let enabled = !app.layout_disabled_connectivity_links.contains(&index);
            add_button(
                document,
                link_row,
                format!("glassworks.viewctl.layout.connectivity_link.toggle.{index}"),
                format!("Stack {} {}", index + 1, if enabled { "On" } else { "Off" }),
                enabled,
                layout::size(
                    layout::px(ui_scale.value(88.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
        if !app.layout_disabled_connectivity_links.is_empty() {
            add_button(
                document,
                link_row,
                "glassworks.viewctl.layout.connectivity_links.enable_all",
                "All On",
                false,
                layout::size(
                    layout::px(ui_scale.value(72.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
    }
    add_text(
        document,
        panel,
        "glassworks.layers.layer_sets.title",
        "Layer Sets",
        text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
    );
    let exchange_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.layer_sets.exchange",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    add_button(
        document,
        exchange_row,
        "glassworks.viewctl.layout.layer_set.export",
        "Export Sets",
        false,
        layout::size(
            layout::px(ui_scale.value(104.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    add_button(
        document,
        exchange_row,
        "glassworks.viewctl.layout.layer_set.import",
        "Import Sets",
        false,
        layout::size(
            layout::px(ui_scale.value(104.0)),
            layout::px(ui_scale.value(26.0)),
        ),
        ui_scale,
    );
    let current_layer_set_state = app.current_layout_layer_set_state();
    let tab_row = document.add_child(
        panel,
        UiNode::container(
            "glassworks.layers.layer_sets.tabs",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale.value(6.0),
            ),
        ),
    );
    for slot in LAYOUT_LAYER_SET_SLOTS {
        let saved_state = app.layout_layer_sets.get(&slot);
        add_button(
            document,
            tab_row,
            format!("glassworks.viewctl.layout.layer_set.tab.{slot}"),
            slot.to_string(),
            saved_state == Some(&current_layer_set_state),
            layout::size(
                layout::px(ui_scale.value(48.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    for slot in LAYOUT_LAYER_SET_SLOTS {
        let saved_state = app.layout_layer_sets.get(&slot);
        let set_name = app.layout_layer_set_name(slot);
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("glassworks.layers.layer_sets.row.{slot}"),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        add_button(
            document,
            row,
            format!("glassworks.viewctl.layout.layer_set.restore.{slot}"),
            compact_button_label(&set_name, 12),
            saved_state == Some(&current_layer_set_state),
            layout::size(
                layout::px(ui_scale.value(92.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            row,
            format!("glassworks.viewctl.layout.layer_set.save.{slot}"),
            "Save",
            false,
            layout::size(
                layout::px(ui_scale.value(64.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_text(
            document,
            row,
            format!("glassworks.layers.layer_sets.count.{slot}"),
            saved_state
                .map(|state| {
                    let group = compact_button_label(&state.layer_group_filter.path_label(), 12);
                    format!("{} {}", state.visible_layers.len(), group)
                })
                .unwrap_or_else(|| "empty".to_string()),
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(
                layout::px(ui_scale.value(72.0)),
                layout::px(ui_scale.value(26.0)),
            ),
        );
    }
    for layer in filtered_layers.into_iter().take(10) {
        let usage_count = layer_usage.get(&layer.id).copied().unwrap_or(0);
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("glassworks.layers.row.{}", layer.id.0),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        document.add_child(
            row,
            UiNode::container(
                format!("glassworks.layers.swatch.{}", layer.id.0),
                layout::size(
                    layout::px(ui_scale.value(14.0)),
                    layout::px(ui_scale.value(14.0)),
                ),
            )
            .with_visual(UiVisual::panel(
                layer_color(layer.color, 255),
                None,
                ui_scale.value(2.0),
            )),
        );
        add_button(
            document,
            row,
            format!("glassworks.viewctl.layout.layer.{}", layer.id.0),
            format!(
                "{} {}",
                if usage_count > 0 { "*" } else { "0" },
                compact_button_label(&layer.name, 13)
            ),
            app.active_layer == layer.id,
            layout::size(
                layout::px(ui_scale.value(124.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            row,
            format!("glassworks.viewctl.layout.toggle_layer.{}", layer.id.0),
            if layer.visible { "v" } else { "-" },
            layer.visible,
            layout::size(
                layout::px(ui_scale.value(28.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            row,
            format!("glassworks.layers.delete.{}", layer.id.0),
            "x",
            false,
            layout::size(
                layout::px(ui_scale.value(28.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }

    if let Some(layer) = app.workspace.document.layers.get(&app.active_layer) {
        add_text(
            document,
            panel,
            "glassworks.layers.properties.title",
            "Layer Properties",
            text_style(ui_scale.value(13.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
        );
        for (index, (label, value)) in [
            ("Layer ID", layer.id.0.to_string()),
            ("Process", format!("{:?}", layer.process)),
            (
                "Group",
                layout_layer_group_for_process(layer.process).path_label(),
            ),
            ("Purpose", layer.name.clone()),
            (
                "Usage",
                format!(
                    "{} shape{}",
                    layer_usage.get(&layer.id).copied().unwrap_or(0),
                    if layer_usage.get(&layer.id).copied().unwrap_or(0) == 1 {
                        ""
                    } else {
                        "s"
                    }
                ),
            ),
            ("Display order", layer.display_order.to_string()),
            ("Fill style", layer.fill_style.label().to_string()),
            ("Line style", layer.line_style.label().to_string()),
            (
                "Hierarchy depth",
                layout_layer_depth_override_label(
                    app.layout_layer_depth_overrides
                        .get(&app.active_layer)
                        .copied(),
                ),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            add_text(
                document,
                panel,
                format!("glassworks.layers.properties.row.{index}"),
                format!("{label}: {value}"),
                text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
            );
        }
        let style_row = document.add_child(
            panel,
            UiNode::container(
                "glassworks.layers.properties.style_controls",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        add_button(
            document,
            style_row,
            "glassworks.viewctl.layout.layer_fill_style.cycle",
            format!("Fill {}", layer.fill_style.label()),
            false,
            layout::size(
                layout::px(ui_scale.value(104.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            style_row,
            "glassworks.viewctl.layout.layer_line_style.cycle",
            format!("Line {}", layer.line_style.label()),
            false,
            layout::size(
                layout::px(ui_scale.value(104.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        let depth_row = document.add_child(
            panel,
            UiNode::container(
                "glassworks.layers.properties.depth_controls",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        let active_depth = app
            .layout_layer_depth_overrides
            .get(&app.active_layer)
            .copied();
        for (slug, label, depth, width) in [
            ("inherit", "Inherit", None, 62.0),
            ("top", "Top", Some(LayoutHierarchyDepth::Top), 42.0),
            ("one", "1", Some(LayoutHierarchyDepth::One), 30.0),
            ("two", "2", Some(LayoutHierarchyDepth::Two), 30.0),
            ("full", "Full", Some(LayoutHierarchyDepth::Full), 42.0),
        ] {
            add_button(
                document,
                depth_row,
                format!("glassworks.viewctl.layout.layer_depth.{slug}"),
                label,
                active_depth == depth,
                layout::size(
                    layout::px(ui_scale.value(width)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
        let group_depth_row = document.add_child(
            panel,
            UiNode::container(
                "glassworks.layers.properties.group_depth_controls",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(28.0)),
                    ),
                    ui_scale.value(6.0),
                ),
            ),
        );
        for (slug, label, width) in [
            ("inherit", "Grp Inh", 68.0),
            ("top", "Grp Top", 58.0),
            ("one", "Grp 1", 46.0),
            ("full", "Grp Full", 66.0),
        ] {
            add_button(
                document,
                group_depth_row,
                format!("glassworks.viewctl.layout.layer_group_depth.{slug}"),
                label,
                false,
                layout::size(
                    layout::px(ui_scale.value(width)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
    }
}

pub(crate) fn add_layout_3d_stack_panel_rows(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let mut layers = app
        .workspace
        .document
        .layers
        .values()
        .filter(|layer| !matches!(layer.process, ProcessLayer::Annotation))
        .collect::<Vec<_>>();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    if layers.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.secondary.stack.empty",
            "No printable layers",
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
        );
        return;
    }

    for layer in layers.into_iter().take(8) {
        let (z_min, z_max) = layer_3d_stack_range(layer.process);
        add_text(
            document,
            parent,
            format!("glassworks.secondary.stack.row.{}", layer.id.0),
            format!("{:?}: {:.0}-{:.0}", layer.process, z_min, z_max),
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
    }
}

pub(crate) fn add_secondary_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    if matches!(
        app.active_view,
        StartupView::Layout2d | StartupView::Layout3d
    ) {
        add_layout_layers_panel(document, parent, app, ui_scale);
        return;
    }

    let rows = match app.active_view {
        StartupView::Metrology => vec![
            (
                "Wafer map".to_string(),
                app.workspace.wafer_map.name.clone(),
            ),
            (
                "Dies".to_string(),
                app.workspace.wafer_map.dies.len().to_string(),
            ),
            (
                "Measurements".to_string(),
                app.workspace.wafer_map.measurements.len().to_string(),
            ),
            (
                "Defects".to_string(),
                app.workspace.wafer_map.defects.len().to_string(),
            ),
        ],
        StartupView::Experiment => vec![
            (
                "Plan".to_string(),
                app.workspace.experiment_plan.title.clone(),
            ),
            (
                "Runs".to_string(),
                app.workspace.experiment_plan.runs.len().to_string(),
            ),
            (
                "Responses".to_string(),
                app.workspace.experiment_plan.responses.len().to_string(),
            ),
            (
                "Factors".to_string(),
                app.workspace.experiment_plan.factors.len().to_string(),
            ),
        ],
        StartupView::Notebook => vec![
            (
                "Entries".to_string(),
                app.workspace.lab_notebook.entries.len().to_string(),
            ),
            (
                "Latest".to_string(),
                app.workspace
                    .lab_notebook
                    .entries
                    .first()
                    .map(|entry| entry.title.clone())
                    .unwrap_or_else(|| "No entries".to_string()),
            ),
        ],
        _ => app
            .workspace
            .document
            .layers
            .values()
            .take(9)
            .map(|layer| {
                (
                    layer.name.clone(),
                    format!(
                        "{:?} {}{}",
                        layer.process,
                        if layer.visible { "visible" } else { "hidden" },
                        if layer.locked { ", locked" } else { "" }
                    ),
                )
            })
            .collect(),
    };
    add_side_panel(
        document,
        parent,
        "glassworks.secondary",
        app.secondary_panel_label(),
        rows,
        secondary_panel_width(ui_scale),
        ui_scale,
    );
}

pub(crate) fn add_side_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    rows: Vec<(String, String)>,
    width: f32,
    ui_scale: UiScale,
) {
    let panel = document.add_child(
        parent,
        UiNode::container(
            name,
            UiNodeStyle::new(layout::with_min_size(
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::px(width),
                            layout::percent(1.0),
                        ),
                        ui_scale.value(8.0),
                    ),
                    ui_scale.value(12.0),
                ),
                layout::px(0.0),
                layout::px(0.0),
            ))
            .with_clip(ClipBehavior::Clip),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    add_text(
        document,
        panel,
        format!("{name}.title"),
        title,
        text_style(ui_scale.value(16.0), FontWeight::BOLD, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
    );
    for (index, (label, value)) in rows.iter().take(12).enumerate() {
        add_text(
            document,
            panel,
            format!("{name}.row.{index}"),
            format!("{label}: {value}"),
            text_style(ui_scale.value(12.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(30.0))),
        );
    }
}

pub(crate) fn add_layout_inspector_rail(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    width: f32,
    height: f32,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let rail = document.add_child(
        parent,
        UiNode::container(
            "glassworks.layout.inspector_rail",
            UiNodeStyle::new(fixed_row_child_layout(
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(layout::column(), layout::px(width), layout::px(height)),
                        ui_scale.value(4.0),
                    ),
                    ui_scale.value(6.0),
                ),
                width,
                height,
            ))
            .with_clip(ClipBehavior::Clip),
        )
        .with_visual(UiVisual::panel(
            COLOR_CHROME_BG,
            Some(StrokeStyle::new(COLOR_PANEL_STROKE, ui_scale.value(1.0))),
            0.0,
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    add_collapsible_detail_sections(
        document,
        rail,
        "glassworks.layout.inspector",
        app,
        &layout_editor_inspector_sections(app),
        8,
        8,
        ui_scale,
    );
    if app.active_view == StartupView::Layout2d {
        add_layout_browser_search(document, rail, app, ui_scale);
        add_layout_hierarchy_tree(document, rail, app, ui_scale);
        add_layout_hierarchy_context(document, rail, app, ui_scale);
        add_layout_cell_browser(document, rail, app, ui_scale);
        add_layout_instance_browser(document, rail, app, ui_scale);
        add_layout_shape_browser(document, rail, app, ui_scale);
        add_layout_measurement_browser(document, rail, app, ui_scale);
        add_layout_net_browser(document, rail, app, ui_scale);
        add_layout_trace_history(document, rail, app, ui_scale);
        add_layout_drc_marker_browser(document, rail, app, ui_scale);
    }
    rail
}
