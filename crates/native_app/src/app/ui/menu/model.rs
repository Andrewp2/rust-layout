#![allow(unused_imports)]
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum MenuPanelItem {
    Action {
        action: String,
        label: String,
        selected: bool,
        enabled: bool,
    },
    Label(String),
    Separator,
}

impl MenuPanelItem {
    pub(crate) fn action(action: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Action {
            action: action.into(),
            label: label.into(),
            selected: false,
            enabled: true,
        }
    }

    pub(crate) fn selected(
        action: impl Into<String>,
        label: impl Into<String>,
        selected: bool,
    ) -> Self {
        Self::Action {
            action: action.into(),
            label: label.into(),
            selected,
            enabled: true,
        }
    }

    pub(crate) fn disabled(action: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Action {
            action: action.into(),
            label: label.into(),
            selected: false,
            enabled: false,
        }
    }

    pub(crate) fn height(&self, ui_scale: UiScale, action_height: f32) -> f32 {
        match self {
            Self::Action { .. } => action_height,
            Self::Label(_) => ui_scale.value(22.0),
            Self::Separator => ui_scale.value(1.0),
        }
    }
}

pub(crate) fn menu_action_enabled(
    action: impl Into<String>,
    label: impl Into<String>,
    enabled: bool,
) -> MenuPanelItem {
    if enabled {
        MenuPanelItem::action(action, label)
    } else {
        MenuPanelItem::disabled(action, label)
    }
}

pub(crate) fn file_menu_items(app: &GlassworksApp) -> Vec<MenuPanelItem> {
    let mut items = vec![
        MenuPanelItem::action("file.new_blank", "New Blank Workspace"),
        MenuPanelItem::action("file.load_demo", "Load Sample Workspace"),
        MenuPanelItem::Separator,
        MenuPanelItem::action("file.save", "Save Workspace"),
        MenuPanelItem::action("file.load", "Load Workspace"),
        MenuPanelItem::action("file.save_session", "Save Session"),
        MenuPanelItem::action("file.load_session", "Load Session"),
        MenuPanelItem::Separator,
        MenuPanelItem::Label("Layout".to_string()),
        MenuPanelItem::action("file.save_layout", "Save Layout JSON"),
        MenuPanelItem::action("file.load_layout", "Load Layout JSON"),
        MenuPanelItem::action("file.import_layout_cell", "Import Layout As Cell"),
        MenuPanelItem::action("file.import_layout_top_cell", "Import Extra Top Cell"),
        MenuPanelItem::action("file.merge_layout", "Merge Layout JSON"),
        MenuPanelItem::action("file.merge_layout_hierarchy", "Merge Layout Hierarchy"),
        MenuPanelItem::Separator,
        MenuPanelItem::Label(format!(
            "Layer Map {} Offset {}",
            app.layout_import_layer_policy.label(),
            app.layout_import_layer_id_offset_label()
        )),
        MenuPanelItem::selected(
            format!(
                "file.import_layer_policy.{}",
                LayoutImportLayerPolicy::Id.slug()
            ),
            "Layer Map By ID",
            app.layout_import_layer_policy == LayoutImportLayerPolicy::Id,
        ),
        MenuPanelItem::selected(
            format!(
                "file.import_layer_policy.{}",
                LayoutImportLayerPolicy::Name.slug()
            ),
            "Layer Map By Name",
            app.layout_import_layer_policy == LayoutImportLayerPolicy::Name,
        ),
        MenuPanelItem::selected(
            format!(
                "file.import_layer_policy.{}",
                LayoutImportLayerPolicy::Copy.slug()
            ),
            "Copy Source Layers",
            app.layout_import_layer_policy == LayoutImportLayerPolicy::Copy,
        ),
        MenuPanelItem::action("file.import_layer_offset.reset", "Layer Offset Reset"),
        MenuPanelItem::action("file.import_layer_offset.neg", "Layer Offset -10"),
        MenuPanelItem::action("file.import_layer_offset.pos", "Layer Offset +10"),
        MenuPanelItem::Separator,
        MenuPanelItem::Label(format!(
            "Import Offset {}",
            app.layout_import_offset_label()
        )),
        MenuPanelItem::action("file.import_offset.reset", "Import Offset Reset"),
        MenuPanelItem::action("file.import_offset.x_neg", "Import Offset -X"),
        MenuPanelItem::action("file.import_offset.x_pos", "Import Offset +X"),
        MenuPanelItem::action("file.import_offset.y_neg", "Import Offset -Y"),
        MenuPanelItem::action("file.import_offset.y_pos", "Import Offset +Y"),
        menu_action_enabled(
            "file.reload_recent",
            "Reload Recent",
            !app.app_options.files.recent_files.is_empty(),
        ),
        MenuPanelItem::action("file.export_gds", "Export GDS"),
        MenuPanelItem::action("file.import_gds", "Import GDS"),
        MenuPanelItem::action("file.import_gds_cell", "Import GDS As Cell"),
        MenuPanelItem::action("file.import_gds_top_cell", "Import GDS Extra Top Cell"),
        MenuPanelItem::action("file.merge_gds", "Merge GDS"),
        MenuPanelItem::action("file.merge_gds_hierarchy", "Merge GDS Hierarchy"),
        MenuPanelItem::action("file.export_cif", "Export CIF"),
        MenuPanelItem::action("file.import_cif", "Import CIF"),
        MenuPanelItem::action("file.import_cif_cell", "Import CIF As Cell"),
        MenuPanelItem::action("file.import_cif_top_cell", "Import CIF Extra Top Cell"),
        MenuPanelItem::action("file.merge_cif", "Merge CIF"),
        MenuPanelItem::action("file.merge_cif_hierarchy", "Merge CIF Hierarchy"),
        MenuPanelItem::action("file.export_dxf", "Export DXF"),
        MenuPanelItem::action("file.import_dxf", "Import DXF"),
        MenuPanelItem::action("file.import_dxf_cell", "Import DXF As Cell"),
        MenuPanelItem::action("file.import_dxf_top_cell", "Import DXF Extra Top Cell"),
        MenuPanelItem::action("file.merge_dxf", "Merge DXF"),
        MenuPanelItem::action("file.merge_dxf_hierarchy", "Merge DXF Hierarchy"),
        MenuPanelItem::action("file.export_def", "Export DEF"),
        MenuPanelItem::action("file.import_def", "Import DEF"),
        MenuPanelItem::action("file.import_def_cell", "Import DEF As Cell"),
        MenuPanelItem::action("file.import_def_top_cell", "Import DEF Extra Top Cell"),
        MenuPanelItem::action("file.merge_def", "Merge DEF"),
        MenuPanelItem::action("file.merge_def_hierarchy", "Merge DEF Hierarchy"),
        MenuPanelItem::action("file.export_lef", "Export LEF"),
        MenuPanelItem::action("file.import_lef", "Import LEF"),
        MenuPanelItem::action("file.import_lef_cell", "Import LEF As Cell"),
        MenuPanelItem::action("file.import_lef_top_cell", "Import LEF Extra Top Cell"),
        MenuPanelItem::action("file.merge_lef", "Merge LEF"),
        MenuPanelItem::action("file.merge_lef_hierarchy", "Merge LEF Hierarchy"),
        MenuPanelItem::action("file.export_reference_images", "Export Ref Images"),
        MenuPanelItem::action("file.import_reference_images", "Import Ref Images"),
        MenuPanelItem::action("file.export_screenshot", "Export Screenshot"),
        MenuPanelItem::action("file.export_screenshot_ppm", "Export Screenshot PPM"),
        MenuPanelItem::action("file.export_screenshot_png", "Export Screenshot PNG"),
    ];
    if !app.app_options.files.recent_files.is_empty() {
        items.push(MenuPanelItem::Separator);
        items.push(MenuPanelItem::Label("Recent".to_string()));
        for (index, file) in app
            .app_options
            .files
            .recent_files
            .iter()
            .take(5)
            .enumerate()
        {
            items.push(MenuPanelItem::action(
                format!("file.recent.{index}"),
                recent_file_menu_label(file),
            ));
        }
    }
    items.push(MenuPanelItem::Separator);
    items.push(MenuPanelItem::disabled(
        "file.collaboration",
        "Connect Collaboration",
    ));
    items
}

pub(crate) fn recent_file_kind_label(kind: &str) -> &'static str {
    match kind {
        "session" => "Session",
        "layout_json" => "Layout",
        "gds" => "GDS",
        "cif" => "CIF",
        "dxf" => "DXF",
        "def" => "DEF",
        "lef" => "LEF",
        "reference_images" => "Ref Images",
        "drc_report" => "DRC Report",
        "drc_report_database" => "DRC DB",
        "drc_deck" => "DRC Deck",
        "calibre_rve" => "Calibre/RVE",
        "netlist" => "Netlist",
        "spice_schematic" => "SPICE Schematic",
        "trace_state" => "Trace State",
        "l2n_database" => "L2N DB",
        _ => "Workspace",
    }
}

pub(crate) fn recent_file_menu_label(file: &options::RecentFileOptions) -> String {
    let name = std::path::Path::new(&file.path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(file.path.as_str());
    format!(
        "{} {}",
        recent_file_kind_label(&file.kind),
        compact_button_label(name, 28)
    )
}
