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
    if entries.is_empty()
        && app.layout_drc_marker_filter == LayoutDrcMarkerFilter::Active
        && app.layout_drc_marker_category_filter.is_none()
        && app.layout_browser_search.trim().is_empty()
        && app.layout_drc_report_history.is_empty()
    {
        return;
    }

    add_text(
        document,
        parent,
        "glassworks.layout.drc_marker_browser.title",
        "DRC Marker Browser",
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
        "glassworks.viewctl.layout.calibre_rve_import",
        "Import Calibre/RVE Markers",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );
    add_button(
        document,
        parent,
        "glassworks.viewctl.layout.drc_markers.write_layer",
        "Write Active Markers To Layer",
        false,
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        ui_scale,
    );

    if !app.layout_drc_report_history.is_empty() {
        add_text(
            document,
            parent,
            "glassworks.layout.drc_reports.title",
            "DRC Reports",
            text_style(ui_scale.value(11.0), FontWeight::BOLD, COLOR_TEXT_MUTED),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(20.0))),
        );
        for entry in &app.layout_drc_report_history {
            let summary = if entry.value.findings.is_empty() {
                format!("{} marker(s)", entry.value.violations.len())
            } else {
                format!("{} issue(s)", entry.value.findings.len())
            };
            let freshness = if entry.revision == app.layout_revision {
                "current"
            } else {
                "stale"
            };
            add_button(
                document,
                parent,
                format!("glassworks.viewctl.layout.drc_report.select.{}", entry.id),
                compact_button_label(
                    &format!("#{} {} {summary} {freshness}", entry.id, entry.label),
                    30,
                ),
                app.layout_selected_drc_report_id == Some(entry.id),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
                ui_scale,
            );
        }
        if let Some(report_id) = app.layout_selected_drc_report_id {
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

    for filter in LayoutDrcMarkerFilter::ALL {
        add_button(
            document,
            parent,
            format!(
                "glassworks.viewctl.layout.drc_marker_filter.{}",
                filter.slug()
            ),
            filter.label(),
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

    if app.layout_selected_drc_marker_key.is_some() {
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
