#![allow(unused_imports)]
use super::*;

pub(crate) fn add_cross_section_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
    compact_rows: bool,
    body_width: f32,
) {
    let snapshots = app.workspace.cross_section.simulate();
    let Some(snapshot) = snapshots.get(
        app.cross_section_step
            .min(snapshots.len().saturating_sub(1)),
    ) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            cross_section_primary_rows(app),
            ui_scale,
        );
        return;
    };
    let process = &app.workspace.cross_section;
    let surface = cross_section_surface_summary(process, snapshot);
    let selected_material = selected_cross_section_material_for_snapshot(app, snapshot);
    let mask_coverage =
        cross_section_mask_coverage_um(process, snapshot) / process.width_um.max(0.1);
    let medium_rows = !compact_rows && body_width < ui_scale.value(1120.0);
    let dense_rows = compact_rows || medium_rows;
    let panel_height = if compact_rows {
        446.0
    } else if medium_rows {
        282.0
    } else {
        430.0
    };
    let preview_height = if compact_rows {
        282.0
    } else if medium_rows {
        160.0
    } else {
        320.0
    };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "glassworks.cross_section.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(panel_height)),
                    ),
                    ui_scale.value(8.0),
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 29, 36, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(55, 69, 83, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        panel,
        "glassworks.cross_section.primary.title",
        format!(
            "Step {}: {} - {}",
            snapshot.step_index,
            snapshot.title,
            cross_section_step_kind_label_for_index(process, snapshot.step_index)
        ),
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );
    document.add_child(
        panel,
        UiNode::scene(
            "glassworks.cross_section.preview",
            cross_section_preview_primitives(
                process,
                snapshot,
                selected_material.as_ref(),
                CrossSectionPreviewOptions {
                    show_mask_overlay: app.cross_section_show_mask,
                    show_dimension_guides: app.cross_section_show_dimensions,
                    show_risk_cues: app.cross_section_show_risks,
                    compact_layout: compact_rows,
                    medium_layout: medium_rows,
                    body_width,
                },
                ui_scale,
            ),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(preview_height)),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(18, 24, 30, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(50, 64, 78, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );

    let material_summary = selected_material
        .as_ref()
        .and_then(|material| process.material(material))
        .map(|material| format!("Material {}", material.name))
        .unwrap_or_else(|| "Material n/a".to_string());
    let summary_items = if dense_rows {
        [
            format!("stack {:.2}-{:.2} um", surface.min_um, surface.max_um),
            format!("avg/rng {:.2}/{:.2}", surface.average_um, surface.range_um),
            format!("Mask open {:.0}%", mask_coverage.clamp(0.0, 1.0) * 100.0),
            material_summary.clone(),
        ]
    } else {
        [
            format!(
                "Stack {} max / {} min",
                format_um_f32(surface.max_um),
                format_um_f32(surface.min_um)
            ),
            format!(
                "Surface {} avg / {} range",
                format_um_f32(surface.average_um),
                format_um_f32(surface.range_um)
            ),
            format!("Mask open {:.0}%", mask_coverage.clamp(0.0, 1.0) * 100.0),
            material_summary,
        ]
    };
    if dense_rows {
        add_compact_metric_rows(
            document,
            panel,
            "glassworks.cross_section.summary",
            &summary_items,
            ui_scale,
        );
    } else {
        let summary = document.add_child(
            panel,
            UiNode::container(
                "glassworks.cross_section.summary",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(44.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.cross_section.summary.height",
            summary_items[0].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.cross_section.summary.surface",
            summary_items[1].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.cross_section.summary.mask",
            summary_items[2].clone(),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            summary,
            "glassworks.cross_section.summary.material",
            summary_items[3].clone(),
            1.0,
            false,
            ui_scale,
        );
    }
}
