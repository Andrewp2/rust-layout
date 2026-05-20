#![allow(unused_imports)]
use super::*;

pub(crate) const LAYOUT_MIN_ZOOM: f32 = 0.000_05;
pub(crate) const LAYOUT_MAX_ZOOM: f32 = 512.0;
pub(crate) const CAMERA_FAR_PLANE_MIN: f32 = 250_000.0;
pub(crate) const CAMERA_FAR_PLANE_SPAN_MULTIPLIER: f32 = 4.0;
pub(crate) const CAMERA_FAR_PLANE_MARGIN_MULTIPLIER: f32 = 1.5;
pub(crate) const LAYOUT_FPS_EMA_ALPHA: f64 = 0.18;
pub(crate) const LAYOUT_COMPACT_TOOL_STRIP_WIDTH: f32 = 900.0;
pub(crate) const REFERENCE_IMAGE_OPACITY_STEP: u8 = 32;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_SCREENSHOT_EXPORT_PATH: &str = "target/glassworks-ui-snapshot.rgba";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_SCREENSHOT_PPM_EXPORT_PATH: &str = "target/glassworks-ui-snapshot.ppm";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_SCREENSHOT_PNG_EXPORT_PATH: &str = "target/glassworks-ui-snapshot.png";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_GDS_EXCHANGE_PATH: &str = "target/glassworks-layout.gds";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_CIF_EXCHANGE_PATH: &str = "target/glassworks-layout.cif";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DXF_EXCHANGE_PATH: &str = "target/glassworks-layout.dxf";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DEF_EXCHANGE_PATH: &str = "target/glassworks-layout.def";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_LEF_EXCHANGE_PATH: &str = "target/glassworks-layout.lef";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_LAYOUT_JSON_EXCHANGE_PATH: &str = "target/glassworks-layout.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DRC_REPORT_EXCHANGE_PATH: &str = "target/glassworks-drc-report.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DRC_REPORT_DATABASE_EXCHANGE_PATH: &str = "target/glassworks-drc-reports.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_KLAYOUT_RDB_EXCHANGE_PATH: &str = "target/glassworks-drc-reports.lyrdb";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DRC_DECK_EXCHANGE_PATH: &str = "target/glassworks-drc-deck.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DRC_MARKER_SNAPSHOT_PATH: &str = "target/glassworks-drc-marker-snapshot.rgba";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_DRC_MARKER_SNAPSHOT_PNG_PATH: &str =
    "target/glassworks-drc-marker-snapshot.png";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_CALIBRE_RVE_EXCHANGE_PATH: &str = "target/glassworks-calibre-rve.txt";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_EXTRACTED_NETLIST_EXCHANGE_PATH: &str = "target/glassworks-netlist.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_SPICE_NETLIST_EXCHANGE_PATH: &str = "target/glassworks-netlist.spice";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_SCHEMATIC_SPICE_EXCHANGE_PATH: &str = "target/glassworks-schematic.spice";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_TRACE_STATE_EXCHANGE_PATH: &str = "target/glassworks-trace-state.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_L2N_DATABASE_EXCHANGE_PATH: &str = "target/glassworks-l2n-database.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_WORKSPACE_SESSION_PATH: &str = "target/glassworks-workspace.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_APP_SESSION_PATH: &str = "target/glassworks-session.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_LAYER_SET_EXCHANGE_PATH: &str = "target/glassworks-layer-sets.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_VIEW_BOOKMARK_EXCHANGE_PATH: &str = "target/glassworks-view-bookmarks.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_REFERENCE_IMAGE_EXCHANGE_PATH: &str = "target/glassworks-reference-images.json";
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) const UI_LIBRARY_CATALOG_EXCHANGE_PATH: &str = "target/glassworks-library-catalog.json";
pub(crate) const UI_SCREENSHOT_EXPORT_WIDTH: u32 = 1440;
pub(crate) const UI_SCREENSHOT_EXPORT_HEIGHT: u32 = 920;
pub(crate) const LAYOUT_SIZE_STEP_GRIDS: Coord = 10;
pub(crate) const LAYOUT_VIEW_BOOKMARK_SLOTS: std::ops::RangeInclusive<u8> = 1..=8;
pub(crate) const LAYOUT_LAYER_SET_SLOTS: std::ops::RangeInclusive<u8> = 1..=8;
pub(crate) const LAYOUT_LIBRARY_PRESET_SLOTS: std::ops::RangeInclusive<u8> = 1..=4;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_COLUMNS: u8 = 3;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_ROWS: u8 = 3;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION: u8 = 1;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION: u8 = 16;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_SIZE_GRIDS: Coord = 18;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_DEFAULT_PITCH_GRIDS: Coord = 30;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS: Coord = 4;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS: Coord = 80;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MIN_SPACING_GRIDS: Coord = 2;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS: Coord = 160;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_SIZE_STEP_GRIDS: Coord = 2;
pub(crate) const LAYOUT_LIBRARY_VIA_ARRAY_PITCH_STEP_GRIDS: Coord = 2;
pub(crate) const LAYOUT_IMPORT_OFFSET_STEP_GRIDS: Coord = 100;
pub(crate) const LAYOUT_IMPORT_LAYER_ID_OFFSET_STEP: i32 = 10;
pub(crate) const LAYOUT_IMPORT_LAYER_ID_OFFSET_MIN: i32 = -10_000;
pub(crate) const LAYOUT_IMPORT_LAYER_ID_OFFSET_MAX: i32 = 10_000;
pub(crate) const LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH: u8 = 32;
pub(crate) const LAYOUT_HIERARCHY_MIN_DEPTH_BUTTONS: [u8; 6] = [0, 1, 2, 3, 4, 5];
pub(crate) const MAX_LAYOUT_TRACE_HISTORY: usize = 8;
pub(crate) const MAX_CONNECTIVITY_OVERLAY_SHAPES: usize = 50_000;
pub(crate) const MAX_HIERARCHY_BOX_OVERLAY_INSTANCES: usize = 10_000;
pub(crate) const MAX_LAYOUT_BROWSER_SHAPES: usize = 18;
pub(crate) const MAX_LAYOUT_BROWSER_INSTANCES: usize = 18;
pub(crate) const MAX_LAYOUT_NET_BROWSER_COMPONENTS: usize = 12;
pub(crate) const MAX_LAYOUT_DRC_REPORT_HISTORY: usize = 8;
pub(crate) const MAX_LAYOUT_DRC_MARKER_ROWS: usize = 12;
pub(crate) const MAX_LAYOUT_MEASUREMENTS: usize = 12;
pub(crate) const MAX_LAYOUT_BROWSER_SEARCH_CHARS: usize = 48;
pub(crate) const LAYOUT_DRC_MARKER_NOTE_PRESETS: [(&str, &str, &str); 3] = [
    ("reviewed", "Note Reviewed", "reviewed in marker browser"),
    ("owner", "Needs Owner", "needs owner follow-up"),
    ("waiver", "Waiver Candidate", "process waiver candidate"),
];
pub(crate) const LAYOUT_DRC_MARKER_OWNER_PRESETS: [(&str, &str, &str); 3] = [
    ("layout", "Owner Layout", "layout-team"),
    ("process", "Owner Process", "process-owner"),
    ("qa", "Owner QA", "qa-review"),
];
pub(crate) const LAYOUT_DRC_MARKER_SIGNOFF_PRESETS: [(&str, &str, &str); 3] = [
    ("needs_review", "Signoff Needs Review", "needs_review"),
    ("accepted", "Signoff Accepted", "accepted"),
    ("rejected", "Signoff Rejected", "rejected"),
];
pub(crate) const LAYOUT_DRC_MARKER_TAG_PRESETS: [(&str, &str, &str, &str); 4] = [
    ("fix", "Tag Needs Fix", "action", "fix"),
    (
        "false_positive",
        "Tag False Positive",
        "disposition",
        "false_positive",
    ),
    ("source_external", "Tag External", "source", "external"),
    (
        "category_litho",
        "Tag Category Litho",
        "category",
        "Litho/Hotspots",
    ),
];
pub(crate) const INVENTORY_DEMO_TODAY: u32 = 20260508;
pub(crate) const MASK_ISSUE_PAGE_SIZE: usize = 50;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_export_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-ui-snapshot-{}.rgba",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_ppm_export_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-ui-snapshot-{}.ppm", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_png_export_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-ui-snapshot-{}.png", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_gds_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.gds", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_cif_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.cif", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_dxf_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.dxf", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_def_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.def", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_lef_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.lef", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_layout_json_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layout-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_report_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-drc-report-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_report_database_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-drc-reports-{}.json",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_klayout_rdb_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-drc-reports-{}.lyrdb",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_deck_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-drc-deck-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_marker_snapshot_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-{}.rgba",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_marker_snapshot_png_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-drc-marker-snapshot-{}.png",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_calibre_rve_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-calibre-rve-{}.txt", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_extracted_netlist_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-netlist-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_spice_netlist_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-netlist-{}.spice", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_schematic_spice_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-schematic-{}.spice", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_trace_state_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-trace-state-{}.json",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_l2n_database_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-l2n-database-{}.json",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_workspace_session_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-workspace-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_app_session_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-session-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_layer_set_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!("glassworks-layer-sets-{}.json", std::process::id()))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_view_bookmark_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-view-bookmarks-{}.json",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_reference_image_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-reference-images-{}.json",
        std::process::id()
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn default_ui_library_catalog_exchange_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "glassworks-library-catalog-{}.json",
        std::process::id()
    ))
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_export_path() -> PathBuf {
    PathBuf::from(UI_SCREENSHOT_EXPORT_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_ppm_export_path() -> PathBuf {
    PathBuf::from(UI_SCREENSHOT_PPM_EXPORT_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_screenshot_png_export_path() -> PathBuf {
    PathBuf::from(UI_SCREENSHOT_PNG_EXPORT_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_gds_exchange_path() -> PathBuf {
    PathBuf::from(UI_GDS_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_cif_exchange_path() -> PathBuf {
    PathBuf::from(UI_CIF_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_dxf_exchange_path() -> PathBuf {
    PathBuf::from(UI_DXF_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_def_exchange_path() -> PathBuf {
    PathBuf::from(UI_DEF_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_lef_exchange_path() -> PathBuf {
    PathBuf::from(UI_LEF_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_layout_json_exchange_path() -> PathBuf {
    PathBuf::from(UI_LAYOUT_JSON_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_report_exchange_path() -> PathBuf {
    PathBuf::from(UI_DRC_REPORT_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_report_database_exchange_path() -> PathBuf {
    PathBuf::from(UI_DRC_REPORT_DATABASE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_klayout_rdb_exchange_path() -> PathBuf {
    PathBuf::from(UI_KLAYOUT_RDB_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_deck_exchange_path() -> PathBuf {
    PathBuf::from(UI_DRC_DECK_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_marker_snapshot_path() -> PathBuf {
    PathBuf::from(UI_DRC_MARKER_SNAPSHOT_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_drc_marker_snapshot_png_path() -> PathBuf {
    PathBuf::from(UI_DRC_MARKER_SNAPSHOT_PNG_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_calibre_rve_exchange_path() -> PathBuf {
    PathBuf::from(UI_CALIBRE_RVE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_extracted_netlist_exchange_path() -> PathBuf {
    PathBuf::from(UI_EXTRACTED_NETLIST_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_spice_netlist_exchange_path() -> PathBuf {
    PathBuf::from(UI_SPICE_NETLIST_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_schematic_spice_exchange_path() -> PathBuf {
    PathBuf::from(UI_SCHEMATIC_SPICE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_trace_state_exchange_path() -> PathBuf {
    PathBuf::from(UI_TRACE_STATE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_l2n_database_exchange_path() -> PathBuf {
    PathBuf::from(UI_L2N_DATABASE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_workspace_session_path() -> PathBuf {
    PathBuf::from(UI_WORKSPACE_SESSION_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_app_session_path() -> PathBuf {
    PathBuf::from(UI_APP_SESSION_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_layer_set_exchange_path() -> PathBuf {
    PathBuf::from(UI_LAYER_SET_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_view_bookmark_exchange_path() -> PathBuf {
    PathBuf::from(UI_VIEW_BOOKMARK_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_reference_image_exchange_path() -> PathBuf {
    PathBuf::from(UI_REFERENCE_IMAGE_EXCHANGE_PATH)
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) fn default_ui_library_catalog_exchange_path() -> PathBuf {
    PathBuf::from(UI_LIBRARY_CATALOG_EXCHANGE_PATH)
}
pub(crate) const LAYOUT_DIFF_DEFAULT_PAGE_SIZE: usize = 50;
pub(crate) const LAYOUT_DIFF_PAGE_SIZE_OPTIONS: [usize; 4] = [25, 50, 100, 200];
pub(crate) const MAX_DRC_INSPECTOR_SHAPES: usize = 50_000;

pub(crate) const COLOR_APP_BG: ColorRgba = ColorRgba::new(8, 11, 14, 255);
pub(crate) const COLOR_CHROME_BG: ColorRgba = ColorRgba::new(12, 17, 21, 255);
pub(crate) const COLOR_PANEL_BG: ColorRgba = ColorRgba::new(15, 21, 26, 255);
pub(crate) const COLOR_PANEL_ALT: ColorRgba = ColorRgba::new(18, 25, 31, 255);
pub(crate) const COLOR_PANEL_STROKE: ColorRgba = ColorRgba::new(31, 41, 50, 255);
pub(crate) const COLOR_BUTTON_BG: ColorRgba = ColorRgba::new(18, 25, 31, 255);
pub(crate) const COLOR_BUTTON_SELECTED: ColorRgba = ColorRgba::new(43, 105, 153, 255);
pub(crate) const COLOR_BUTTON_STROKE_SELECTED: ColorRgba = ColorRgba::new(72, 148, 204, 255);
pub(crate) const COLOR_TEXT: ColorRgba = ColorRgba::new(222, 229, 235, 255);
pub(crate) const COLOR_TEXT_MUTED: ColorRgba = ColorRgba::new(142, 152, 162, 255);
pub(crate) const OPERAD_UI_RUNTIME_VERSION: &str = "7.0.0";
pub(crate) const USER_ID_LABEL: &str = "User 6a04b2";
