#![allow(unused_imports)]
use super::*;

pub(crate) fn add_layout_drc_marker_browser(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &GlassworksApp,
    ui_scale: UiScale,
) {
    let entries = layout_drc_marker_entries(app);
    let category_entries = layout_drc_marker_category_entries(app);
    let report_source_entries = layout_drc_report_source_entries(app);
    let snapshot_entries = layout_drc_marker_snapshot_entries(app);
    let snapshot_total_count = layout_drc_marker_snapshot_total_count(app);
    let show_snapshot_gallery = !snapshot_entries.is_empty()
        || snapshot_total_count > 0 && !app.layout_browser_search.trim().is_empty();
    if entries.is_empty()
        && snapshot_entries.is_empty()
        && app.layout_drc_marker_filter == LayoutDrcMarkerFilter::Active
        && app.layout_drc_marker_category_filter.is_none()
        && app.layout_drc_marker_directory_filter.is_none()
        && app.layout_browser_search.trim().is_empty()
        && app.layout_drc_report_history.is_empty()
    {
        return;
    }

    add_text(
        document,
        parent,
        "glassworks.layout.drc_marker_browser.title",
        layout_drc_marker_browser_title(app, entries.len()),
        text_style(ui_scale.value(12.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );

    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_report_export",
        "Export DRC Report",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_report_import",
        "Import DRC Report",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_report_database_export",
        "Export Report Database",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_report_database_import",
        "Import Report Database",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_report_database_append",
        "Append Report Database",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.klayout_rdb_export",
        "Export KLayout RDB",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.klayout_rdb_import",
        "Import KLayout RDB",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.klayout_rdb_append",
        "Append KLayout RDB",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.calibre_rve_import",
        "Import Calibre/RVE Markers",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    if !report_source_entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_report_sources.title",
            "Report Database Sources",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_report_source.all",
            "All Report Sources",
            !app.layout_browser_search
                .trim()
                .to_ascii_lowercase()
                .starts_with("source="),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        for (index, entry) in report_source_entries.iter().take(8).enumerate() {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report_source.{index}"),
                compact_button_label(
                    &format!(
                        "{} {}/{} reports, {} markers",
                        entry.source, entry.listed_reports, entry.total_reports, entry.marker_count
                    ),
                    34,
                ),
                layout_drc_report_source_filter_is_active(app, &entry.source),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report_source.delete.{index}"),
                compact_button_label(&format!("Delete Source {}", entry.source), 34),
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_markers.write_layer",
        "Write Active Markers To Layer",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    if app
        .drc_report()
        .is_some_and(|report| !report.violations.is_empty())
    {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_markers.clear_states",
            "Clear Active Marker States",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_markers.clear_snapshots",
            "Clear Active Marker Snapshots",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_markers.clear_tags",
            "Clear Active Marker Tags",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !app.layout_drc_report_history.is_empty() {
        let report_entries = layout_drc_report_browser_entries(app);
        let listed_report_ids = report_entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        add_text(
            document,
            parent,
            "glassworks.layout.drc_reports.title",
            layout_drc_report_browser_title(
                app,
                report_entries.len(),
                app.layout_drc_report_history.len(),
            ),
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        if report_entries.is_empty() {
            add_text(
                document,
                parent,
                "glassworks.layout.drc_reports.empty",
                "No matching reports",
                text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
        }
        if !report_entries.is_empty() {
            add_button(
                document,
                parent,
                "glassworks.viewctl.layout.drc_report_browser.select_first",
                "Select First Report",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        for entry in &report_entries {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report.select.{}", entry.id),
                layout_drc_report_button_label(app, entry),
                app.layout_selected_drc_report_id == Some(entry.id),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report.delete.{}", entry.id),
                format!("Delete Report #{}", entry.id),
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        if let Some(report_id) = app.layout_selected_drc_report_id
            && !listed_report_ids.contains(&report_id)
        {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report.delete.{report_id}"),
                "Delete Active Report",
                false,
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_reports.clear",
            "Clear DRC Reports",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
    }

    if !category_entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_marker_categories.title",
            "Marker Categories",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_category.all",
            "All Categories",
            app.layout_drc_marker_category_filter.is_none(),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        for entry in category_entries {
            add_button(
                document,
                parent,
                format!(
                    "glassworks.viewctl.layout.drc_marker_category.{}",
                    entry.category.slug()
                ),
                format!(
                    "{} {}/{}",
                    entry.category.label(),
                    entry.active,
                    entry.total
                ),
                app.layout_drc_marker_category_filter == Some(entry.category),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    let directory_entries = layout_drc_marker_directory_filter_entries(app);
    if !directory_entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_marker_directories.title",
            "Marker Directories",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_directory.all",
            "All Directories",
            app.layout_drc_marker_directory_filter.is_none(),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        for (index, entry) in directory_entries.iter().take(8).enumerate() {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_marker_directory.{index}"),
                compact_button_label(
                    &format!("{} {}/{}", entry.path, entry.active, entry.total),
                    30,
                ),
                app.layout_drc_marker_directory_filter.as_ref() == Some(&entry.path),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
    }

    for filter in LayoutDrcMarkerFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.drc_marker_filter.{}",
                filter.slug()
            ),
            layout_drc_marker_filter_button_label(app, filter),
            app.layout_drc_marker_filter == filter,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    for sort in LayoutDrcMarkerSort::ALL {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.drc_marker_sort.{}", sort.slug()),
            format!("Sort {}", sort.label()),
            app.layout_drc_marker_sort == sort,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if show_snapshot_gallery {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_marker_snapshots.title",
            layout_drc_marker_snapshot_title(app, snapshot_entries.len(), snapshot_total_count),
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        if snapshot_entries.is_empty() {
            add_text(
                document,
                parent,
                "glassworks.layout.drc_marker_snapshots.empty",
                "No matching snapshots",
                text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
            );
        }
        for entry in snapshot_entries.iter().take(8) {
            add_layout_drc_marker_snapshot_button(
                document,
                parent,
                entry,
                app.layout_selected_drc_marker_key.as_ref() == Some(&entry.key),
                ui_scale,
            );
        }
    }

    if app.layout_selected_drc_marker_key.is_some() {
        if let Some(violation) = selected_layout_drc_marker_violation(app) {
            let mut source_shapes = violation.shape_ids.clone();
            for occurrence in &violation.occurrence_ids {
                let shape_id = occurrence.source_shape_id();
                if !source_shapes.contains(&shape_id) {
                    source_shapes.push(shape_id);
                }
            }
            if !source_shapes.is_empty() {
                add_text(
                    document,
                    parent,
                    "glassworks.layout.drc_marker_objects.title",
                    "Marker Objects",
                    text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
                );
                for shape_id in source_shapes.iter().take(8) {
                    add_button(
                        document,
                        parent,
                        format!(
                            "glassworks.viewctl.layout.drc_marker_source_shape.{}",
                            shape_id.0
                        ),
                        format!("Select Shape #{}", shape_id.0),
                        app.selected_layout_occurrence
                            .as_ref()
                            .is_some_and(|occurrence| occurrence.source_shape_id() == *shape_id)
                            || app.selected_layout_shape == Some(*shape_id),
                        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                        ui_scale,
                    );
                }
                for occurrence in violation.occurrence_ids.iter().take(8) {
                    add_button(
                        document,
                        parent,
                        format!(
                            "glassworks.viewctl.layout.drc_marker_source_occurrence.{}",
                            layout_occurrence_action_key(occurrence)
                        ),
                        compact_button_label(
                            &format!("Select {}", layout_occurrence_label(occurrence)),
                            30,
                        ),
                        app.selected_layout_occurrence.as_ref() == Some(occurrence),
                        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                        ui_scale,
                    );
                }
                for cell_id in layout_drc_marker_source_cells(&app.workspace.document, &violation)
                    .into_iter()
                    .take(8)
                {
                    add_button(
                        document,
                        parent,
                        format!(
                            "glassworks.viewctl.layout.drc_marker_source_cell.{}",
                            cell_id.0
                        ),
                        compact_button_label(
                            &format!(
                                "View {}",
                                layout_cell_display_name(&app.workspace.document, cell_id)
                            ),
                            30,
                        ),
                        app.layout_view_top_cell == cell_id,
                        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                        ui_scale,
                    );
                }
            }
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_toggle.hidden",
            "Hide / Show Selected",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_toggle.waived",
            "Waive / Unwaive Selected",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_toggle.visited",
            "Visit / Unvisit Selected",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_toggle.important",
            "Important / Normal",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
        let selected_note = app
            .layout_selected_drc_marker_key
            .as_ref()
            .and_then(|key| app.workspace.document.marker_states.get(key))
            .and_then(|state| state.note.as_deref());
        let selected_owner = app
            .layout_selected_drc_marker_key
            .as_ref()
            .and_then(|key| app.workspace.document.marker_states.get(key))
            .and_then(|state| state.owner.as_deref());
        let selected_signoff = app
            .layout_selected_drc_marker_key
            .as_ref()
            .and_then(|key| app.workspace.document.marker_states.get(key))
            .and_then(|state| state.signoff.as_deref());
        let selected_tags = app
            .layout_selected_drc_marker_key
            .as_ref()
            .and_then(|key| app.workspace.document.marker_states.get(key))
            .map(|state| &state.tags);
        for (slug, label, note) in LAYOUT_DRC_MARKER_NOTE_PRESETS {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_marker_note.{slug}"),
                label,
                selected_note == Some(note),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        for (slug, label, owner) in LAYOUT_DRC_MARKER_OWNER_PRESETS {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_marker_owner.{slug}"),
                label,
                selected_owner == Some(owner),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_owner.clear",
            "Clear Owner",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        for (slug, label, signoff) in LAYOUT_DRC_MARKER_SIGNOFF_PRESETS {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_marker_signoff.{slug}"),
                label,
                selected_signoff == Some(signoff),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_signoff.clear",
            "Clear Signoff",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        for (slug, label, tag_key, tag_value) in LAYOUT_DRC_MARKER_TAG_PRESETS {
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_marker_tag.{slug}"),
                label,
                selected_tags.is_some_and(|tags| {
                    tags.get(tag_key)
                        .is_some_and(|value| value.as_str() == tag_value)
                }),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_custom_tag.apply",
            "Apply Custom Tag",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_custom_tag.remove",
            "Remove Custom Tag",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        if let Some(tags) = selected_tags {
            for (index, (tag_key, tag_value)) in tags.iter().take(6).enumerate() {
                add_button(
                    document,
                    parent,
                    format!("glassworks.viewctl.layout.drc_marker_tag.remove.{index}"),
                    compact_button_label(&format!("Remove {tag_key}={tag_value}"), 40),
                    false,
                    layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                    ui_scale,
                );
            }
        }
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_snapshot",
            "Export Marker Snapshot",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_snapshot_png",
            "Export Marker PNG",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_snapshot.clear",
            "Clear Marker Snapshot",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_tag.clear",
            "Clear Tags",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_note.clear",
            "Clear Note",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
            ui_scale,
        );
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_clear_state",
            "Clear Selected State",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if !entries.is_empty() {
        add_button(
            document,
            parent,
            "glassworks.viewctl.layout.drc_marker_browser.select_first",
            "Select First Marker",
            false,
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
            ui_scale,
        );
    }

    if entries.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_marker_browser.empty",
            "No matching markers",
            text_style(ui_scale.value(11.0), FontWeight::NORMAL, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        return;
    }

    for entry in entries {
        add_button(
            document,
            parent,
            format!("glassworks.viewctl.layout.drc_marker.{}", entry.id),
            entry.label,
            app.layout_selected_drc_marker_key.as_ref() == Some(&entry.key),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
            ui_scale,
        );
    }
}

fn add_layout_drc_marker_snapshot_button(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    entry: &LayoutDrcMarkerSnapshotEntry,
    selected: bool,
    ui_scale: UiScale,
) {
    let name = format!(
        "glassworks.viewctl.layout.drc_marker_snapshot_entry.{}",
        entry.id
    );
    let fill = if selected {
        COLOR_BUTTON_SELECTED
    } else {
        COLOR_BUTTON_BG
    };
    let stroke = Some(StrokeStyle::new(
        if selected {
            COLOR_BUTTON_STROKE_SELECTED
        } else {
            COLOR_PANEL_STROKE
        },
        ui_scale.value(1.0),
    ));
    let weight = if selected {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    let button_layout = layout::Layout::row()
        .size(layout::LayoutSize::new(
            layout::LayoutDimension::Percent(1.0),
            layout::LayoutDimension::Points(ui_scale.value(42.0)),
        ))
        .align_items(layout::LayoutAlignment::Center)
        .justify_content(layout::LayoutJustifyContent::FlexStart)
        .gap(layout::LayoutGap::points(ui_scale.value(6.0), 0.0))
        .to_layout_style();
    let button_id = document.add_child(
        parent,
        UiNode::container(name.clone(), UiNodeStyle::clipped(button_layout))
            .with_visual(UiVisual::panel(fill, stroke, ui_scale.value(2.0)))
            .with_input(InputBehavior::BUTTON)
            .with_action(WidgetActionBinding::action(name.clone()))
            .with_accessibility(
                AccessibilityMeta::new(AccessibilityRole::Button)
                    .label(entry.label.clone())
                    .focusable(),
            ),
    );
    let canvas_layout =
        layout::Layout::fixed(ui_scale.value(54.0), ui_scale.value(32.0)).to_layout_style();
    let canvas_id = document.add_child(
        button_id,
        UiNode::canvas(
            format!("{name}.image"),
            LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY,
            canvas_layout,
        )
        .with_input(InputBehavior::NONE)
        .with_accessibility(
            AccessibilityMeta::new(AccessibilityRole::Image).label(entry.label.clone()),
        ),
    );
    document.set_node_content(
        canvas_id,
        UiContent::Canvas(
            CanvasContent::new(LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY).context(
                CanvasContextDescriptor::gpu_texture(entry.image_key.clone()),
            ),
        ),
    );
    add_text(
        document,
        button_id,
        format!("{name}.label"),
        entry.label.clone(),
        text_style(ui_scale.value(12.0), weight, COLOR_TEXT),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(32.0))),
    );
}
