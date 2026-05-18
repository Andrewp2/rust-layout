use serde::{Deserialize, Serialize};

pub const APP_OPTIONS_SCHEMA_VERSION: u32 = 1;
pub const GLASSWORKS_OPTIONS_FILE_ENV: &str = "GLASSWORKS_OPTIONS_FILE";

const VIEW_SLUGS: &[&str] = &[
    "workflow",
    "layout2d",
    "layout3d",
    "mask-prep",
    "layout-diff",
    "fab-control",
    "inventory",
    "maintenance",
    "environment",
    "scheduler",
    "safety",
    "traceability",
    "metrology",
    "yield",
    "spc-fdc",
    "process-flow",
    "process-control",
    "cross-section",
    "experiment",
    "notebook",
];
const TOOL_SLUGS: &[&str] = &[
    "select", "rect", "polygon", "path", "via", "measure", "route", "trace",
];
const LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH: u8 = 32;
const LAYOUT_VIEW_BOOKMARK_SLOTS: std::ops::RangeInclusive<u8> = 1..=8;
const LAYOUT_LAYER_SET_SLOTS: std::ops::RangeInclusive<u8> = 1..=8;
const LAYOUT_LAYER_GROUP_SLUGS: &[&str] = &[
    "all",
    "front_end",
    "feol_diffusion",
    "feol_gate",
    "feol_dielectric",
    "routing",
    "routing_contact",
    "routing_metal",
    "routing_via",
    "annotation",
];
const LAYOUT_LAYER_USAGE_FILTER_SLUGS: &[&str] = &["all", "used", "empty"];
const LAYOUT_MEASUREMENT_MODE_SLUGS: &[&str] = &["direct", "horizontal", "vertical", "manhattan"];
const LAYOUT_TECHNOLOGY_SLUGS: &[&str] = &["glassworks_demo", "glassworks_high_density"];
const LAYOUT_LIBRARY_PRESET_SLOTS: std::ops::RangeInclusive<u8> = 1..=4;
const LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION: u8 = 1;
const LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION: u8 = 16;
const LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS: i64 = 4;
const LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS: i64 = 80;
const LAYOUT_LIBRARY_VIA_ARRAY_MIN_SPACING_GRIDS: i64 = 2;
const LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS: i64 = 160;
const UNIT_SLUGS: &[&str] = &["auto", "nanometers", "microns", "dbu"];
const MASK_SEVERITY_SLUGS: &[&str] = &["all", "errors", "warnings"];
const MASK_GROUPING_SLUGS: &[&str] = &["code", "layer", "field", "block"];
const INVENTORY_FILTER_SLUGS: &[&str] =
    &["all", "action", "hold", "low-stock", "expiring", "in-use"];
const MAINTENANCE_WORK_SLUGS: &[&str] = &["actionable", "upcoming", "calibration", "locked", "all"];
const MAINTENANCE_HISTORY_SLUGS: &[&str] = &["selected", "all"];
const DISPATCH_POLICY_SLUGS: &[&str] = &["priority", "fifo", "due-date"];
const TRACE_IMPACT_SLUGS: &[&str] = &["tool-run", "step", "material"];
const METROLOGY_MODE_SLUGS: &[&str] = &["value", "delta", "spec", "defects", "review", "overlay"];
const MEASUREMENT_KIND_SLUGS: &[&str] = &["thickness", "sheet-r", "cd", "defects", "pass-fail"];
const YIELD_FILTER_SLUGS: &[&str] = &["all", "failing", "passing"];
const SPC_SEVERITY_SLUGS: &[&str] = &["all", "critical", "warning", "advisory"];
const SPC_SOURCE_SLUGS: &[&str] = &["all", "spc", "fdc", "alarm"];
const PROCESS_FLOW_FILTER_SLUGS: &[&str] = &["all", "recipes", "metrology", "holds", "rework"];
const CONTROL_LOOP_FILTER_SLUGS: &[&str] = &["all", "active", "attention"];
const EXPERIMENT_RUN_FILTER_SLUGS: &[&str] = &[
    "all",
    "needs-any-response",
    "needs-selected-response",
    "in-progress",
    "complete",
    "out-of-spec",
    "blocked",
];
const RECENT_FILE_KIND_SLUGS: &[&str] = &[
    "session",
    "workspace",
    "layout_json",
    "gds",
    "cif",
    "dxf",
    "def",
    "lef",
    "reference_images",
    "drc_report",
    "drc_report_database",
    "drc_deck",
    "calibre_rve",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppOptions {
    pub schema_version: u32,
    pub appearance: AppearanceOptions,
    pub shell: ShellOptions,
    pub layout: LayoutEditorOptions,
    pub viewport3d: Viewport3dOptions,
    pub performance: PerformanceOptions,
    pub files: FileOptions,
    pub domains: DomainOptions,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            schema_version: APP_OPTIONS_SCHEMA_VERSION,
            appearance: AppearanceOptions::default(),
            shell: ShellOptions::default(),
            layout: LayoutEditorOptions::default(),
            viewport3d: Viewport3dOptions::default(),
            performance: PerformanceOptions::default(),
            files: FileOptions::default(),
            domains: DomainOptions::default(),
        }
    }
}

impl AppOptions {
    pub fn normalized(mut self) -> Self {
        self.schema_version = APP_OPTIONS_SCHEMA_VERSION;
        self.appearance = self.appearance.normalized();
        self.shell = self.shell.normalized();
        self.layout = self.layout.normalized();
        self.viewport3d = self.viewport3d.normalized();
        self.performance = self.performance.normalized();
        self.files = self.files.normalized();
        self.domains = self.domains.normalized();
        self
    }

    pub fn from_json_str(input: &str) -> Result<Self, String> {
        serde_json::from_str::<Self>(input)
            .map(Self::normalized)
            .map_err(|error| format!("parse options JSON: {error}"))
    }

    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.clone().normalized())
            .map_err(|error| format!("serialize options JSON: {error}"))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceOptions {
    pub theme: ThemePreference,
    pub unit_display: String,
    pub ui_scale: f32,
    pub dense_mode: bool,
}

impl Default for AppearanceOptions {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            unit_display: "auto".to_string(),
            ui_scale: 1.0,
            dense_mode: true,
        }
    }
}

impl AppearanceOptions {
    fn normalized(mut self) -> Self {
        self.unit_display = normalize_slug(self.unit_display, UNIT_SLUGS, "auto");
        self.ui_scale = clamp_f32(self.ui_scale, 1.0, 3.0, 1.0);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellOptions {
    pub startup_view: String,
    pub show_command_palette_on_start: bool,
    pub show_sidebar_modules: bool,
    pub show_options_panel: bool,
    pub show_diagnostics_panel: bool,
    pub show_details_panel: bool,
    pub show_secondary_panel: bool,
    pub nav_rail_views: Vec<String>,
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            startup_view: "workflow".to_string(),
            show_command_palette_on_start: false,
            show_sidebar_modules: false,
            show_options_panel: false,
            show_diagnostics_panel: false,
            show_details_panel: false,
            show_secondary_panel: false,
            nav_rail_views: VIEW_SLUGS.iter().map(|slug| (*slug).to_string()).collect(),
        }
    }
}

impl ShellOptions {
    fn normalized(mut self) -> Self {
        self.startup_view = normalize_slug(self.startup_view, VIEW_SLUGS, "workflow");
        self.nav_rail_views = normalize_slug_list(self.nav_rail_views, VIEW_SLUGS);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutEditorOptions {
    pub default_tool: String,
    pub technology: String,
    pub disabled_connectivity_links: Vec<usize>,
    pub active_layer: u32,
    pub snap_enabled: bool,
    pub show_2d_grid: bool,
    pub show_origin_marker: bool,
    pub show_drc_overlay: bool,
    pub show_reference_images: bool,
    pub hierarchy_depth: String,
    pub hierarchy_min_depth: u8,
    pub layer_group_filter: String,
    pub layer_usage_filter: String,
    pub measurement_mode: String,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub view_bookmarks: Vec<LayoutViewBookmarkOptions>,
    pub layer_sets: Vec<LayoutLayerSetOptions>,
    pub layer_depths: Vec<LayoutLayerDepthOptions>,
    pub library_presets: Vec<LayoutLibraryPresetOptions>,
    pub constrain_with_shift: bool,
    pub bypass_snap_with_ctrl: bool,
    pub duplicate_drag_with_alt: bool,
}

impl Default for LayoutEditorOptions {
    fn default() -> Self {
        Self {
            default_tool: "select".to_string(),
            technology: "glassworks_demo".to_string(),
            disabled_connectivity_links: Vec::new(),
            active_layer: 4,
            snap_enabled: true,
            show_2d_grid: true,
            show_origin_marker: true,
            show_drc_overlay: true,
            show_reference_images: true,
            hierarchy_depth: "full".to_string(),
            hierarchy_min_depth: 0,
            layer_group_filter: "all".to_string(),
            layer_usage_filter: "all".to_string(),
            measurement_mode: "direct".to_string(),
            zoom: 0.02,
            pan: [0.0, 0.0],
            view_bookmarks: Vec::new(),
            layer_sets: Vec::new(),
            layer_depths: Vec::new(),
            library_presets: Vec::new(),
            constrain_with_shift: true,
            bypass_snap_with_ctrl: true,
            duplicate_drag_with_alt: true,
        }
    }
}

impl LayoutEditorOptions {
    fn normalized(mut self) -> Self {
        self.default_tool = normalize_slug(self.default_tool, TOOL_SLUGS, "select");
        self.technology = normalize_slug(self.technology, LAYOUT_TECHNOLOGY_SLUGS, "glassworks_demo");
        self.disabled_connectivity_links.retain(|index| *index < 64);
        self.disabled_connectivity_links.sort_unstable();
        self.disabled_connectivity_links.dedup();
        self.hierarchy_depth = normalize_layout_hierarchy_depth(self.hierarchy_depth);
        self.hierarchy_min_depth = self
            .hierarchy_min_depth
            .min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH);
        self.layer_group_filter =
            normalize_slug(self.layer_group_filter, LAYOUT_LAYER_GROUP_SLUGS, "all");
        self.layer_usage_filter = normalize_slug(
            self.layer_usage_filter,
            LAYOUT_LAYER_USAGE_FILTER_SLUGS,
            "all",
        );
        self.measurement_mode = normalize_slug(
            self.measurement_mode,
            LAYOUT_MEASUREMENT_MODE_SLUGS,
            "direct",
        );
        self.zoom = clamp_f32(self.zoom, 0.000_05, 512.0, 0.02);
        self.pan = sanitize_pan(self.pan);
        self.view_bookmarks = normalize_layout_view_bookmarks(self.view_bookmarks);
        self.layer_sets = normalize_layout_layer_sets(self.layer_sets);
        self.layer_depths = normalize_layout_layer_depths(self.layer_depths);
        self.library_presets = normalize_layout_library_presets(self.library_presets);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutViewBookmarkOptions {
    pub slot: u8,
    pub name: String,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub top_cell: u64,
    pub hierarchy_depth: String,
    pub hierarchy_min_depth: u8,
}

impl Default for LayoutViewBookmarkOptions {
    fn default() -> Self {
        Self {
            slot: 1,
            name: "View 1".to_string(),
            zoom: 0.02,
            pan: [0.0, 0.0],
            top_cell: 1,
            hierarchy_depth: "full".to_string(),
            hierarchy_min_depth: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutLayerSetOptions {
    pub slot: u8,
    pub name: String,
    pub visible_layers: Vec<u32>,
    pub layer_group_filter: String,
    pub layer_usage_filter: String,
    pub layer_depths: Vec<LayoutLayerDepthOptions>,
}

impl Default for LayoutLayerSetOptions {
    fn default() -> Self {
        Self {
            slot: 1,
            name: "Layer Set 1".to_string(),
            visible_layers: Vec::new(),
            layer_group_filter: "all".to_string(),
            layer_usage_filter: "all".to_string(),
            layer_depths: Vec::new(),
        }
    }
}

impl LayoutLayerSetOptions {
    fn normalized(mut self) -> Self {
        if !LAYOUT_LAYER_SET_SLOTS.contains(&self.slot) {
            self.slot = 1;
        }
        if self.name.trim().is_empty() {
            self.name = format!("Layer Set {}", self.slot);
        } else {
            self.name = self.name.trim().to_string();
        }
        self.visible_layers.sort_unstable();
        self.visible_layers.dedup();
        self.layer_group_filter =
            normalize_slug(self.layer_group_filter, LAYOUT_LAYER_GROUP_SLUGS, "all");
        self.layer_usage_filter = normalize_slug(
            self.layer_usage_filter,
            LAYOUT_LAYER_USAGE_FILTER_SLUGS,
            "all",
        );
        self.layer_depths = normalize_layout_layer_depths(self.layer_depths);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutLayerDepthOptions {
    pub layer: u32,
    pub max_depth: String,
}

impl Default for LayoutLayerDepthOptions {
    fn default() -> Self {
        Self {
            layer: 1,
            max_depth: "full".to_string(),
        }
    }
}

impl LayoutLayerDepthOptions {
    fn normalized(mut self) -> Self {
        if self.layer == 0 {
            self.layer = 1;
        }
        self.max_depth = normalize_layout_hierarchy_depth(self.max_depth);
        if self.max_depth == "boxes" {
            self.max_depth = "full".to_string();
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutLibraryPresetOptions {
    pub slot: u8,
    pub name: String,
    pub macro_name: String,
    pub columns: u8,
    pub rows: u8,
    pub size_grids: i64,
    pub pitch_grids: i64,
}

impl Default for LayoutLibraryPresetOptions {
    fn default() -> Self {
        Self {
            slot: 1,
            name: "Via Array 1".to_string(),
            macro_name: "via_array".to_string(),
            columns: 3,
            rows: 3,
            size_grids: 18,
            pitch_grids: 30,
        }
    }
}

impl LayoutLibraryPresetOptions {
    fn normalized(mut self) -> Self {
        if !LAYOUT_LIBRARY_PRESET_SLOTS.contains(&self.slot) {
            self.slot = 1;
        }
        self.macro_name = normalize_slug(self.macro_name, &["via_array"], "via_array");
        if self.name.trim().is_empty() {
            self.name = format!("Via Array {}", self.slot);
        } else {
            self.name = self.name.trim().chars().take(32).collect();
        }
        self.columns = self.columns.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.rows = self.rows.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_DIMENSION,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_DIMENSION,
        );
        self.size_grids = self.size_grids.clamp(
            LAYOUT_LIBRARY_VIA_ARRAY_MIN_SIZE_GRIDS,
            LAYOUT_LIBRARY_VIA_ARRAY_MAX_SIZE_GRIDS,
        );
        self.pitch_grids =
            normalize_layout_library_via_array_pitch_grids(self.size_grids, self.pitch_grids);
        self
    }
}

impl LayoutViewBookmarkOptions {
    fn normalized(mut self) -> Self {
        if !LAYOUT_VIEW_BOOKMARK_SLOTS.contains(&self.slot) {
            self.slot = 1;
        }
        if self.name.trim().is_empty() {
            self.name = format!("View {}", self.slot);
        } else {
            self.name = self.name.trim().chars().take(32).collect();
        }
        self.zoom = clamp_f32(self.zoom, 0.000_05, 512.0, 0.02);
        self.pan = sanitize_pan(self.pan);
        self.hierarchy_depth = normalize_layout_hierarchy_depth(self.hierarchy_depth);
        self.hierarchy_min_depth = self
            .hierarchy_min_depth
            .min(LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Viewport3dOptions {
    pub show_grid: bool,
    pub capture_flycam_on_click: bool,
    pub mouse_sensitivity: f32,
    pub movement_speed: f32,
    pub fast_multiplier: f32,
    pub fov_degrees: f32,
    pub fullscreen_on_start: bool,
}

impl Default for Viewport3dOptions {
    fn default() -> Self {
        Self {
            show_grid: true,
            capture_flycam_on_click: true,
            mouse_sensitivity: 0.003,
            movement_speed: 4_000.0,
            fast_multiplier: 4.0,
            fov_degrees: 58.0,
            fullscreen_on_start: false,
        }
    }
}

impl Viewport3dOptions {
    fn normalized(mut self) -> Self {
        self.mouse_sensitivity = clamp_f32(self.mouse_sensitivity, 0.0005, 0.02, 0.003);
        self.movement_speed = clamp_f32(self.movement_speed, 100.0, 250_000.0, 4_000.0);
        self.fast_multiplier = clamp_f32(self.fast_multiplier, 1.0, 20.0, 4.0);
        self.fov_degrees = clamp_f32(self.fov_degrees, 20.0, 120.0, 58.0);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceOptions {
    pub live_fps_meter: bool,
    pub fps_ema_alpha: f64,
    pub idle_redraw_layout_viewports: bool,
    pub cache_layout_index: bool,
    pub cache_drc_reports: bool,
    pub cache_connectivity_reports: bool,
    pub dense_2d_lod: bool,
    pub dense_3d_instancing: bool,
}

impl Default for PerformanceOptions {
    fn default() -> Self {
        Self {
            live_fps_meter: true,
            fps_ema_alpha: 0.18,
            idle_redraw_layout_viewports: true,
            cache_layout_index: true,
            cache_drc_reports: true,
            cache_connectivity_reports: true,
            dense_2d_lod: true,
            dense_3d_instancing: true,
        }
    }
}

impl PerformanceOptions {
    fn normalized(mut self) -> Self {
        self.fps_ema_alpha = clamp_f64(self.fps_ema_alpha, 0.01, 1.0, 0.18);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileOptions {
    pub autosave_enabled: bool,
    pub autosave_interval_seconds: u64,
    pub remember_last_workspace: bool,
    pub recent_workspace_limit: usize,
    pub recent_files: Vec<RecentFileOptions>,
    pub prompt_before_destructive_actions: bool,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            autosave_enabled: false,
            autosave_interval_seconds: 120,
            remember_last_workspace: true,
            recent_workspace_limit: 10,
            recent_files: Vec::new(),
            prompt_before_destructive_actions: true,
        }
    }
}

impl FileOptions {
    fn normalized(mut self) -> Self {
        self.autosave_interval_seconds = self.autosave_interval_seconds.clamp(15, 3600);
        self.recent_workspace_limit = self.recent_workspace_limit.clamp(0, 50);
        self.recent_files = normalize_recent_files(self.recent_files, self.recent_workspace_limit);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentFileOptions {
    pub kind: String,
    pub path: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DomainOptions {
    pub workflow: WorkflowOptions,
    pub mask_prep: MaskPrepOptions,
    pub layout_diff: LayoutDiffOptions,
    pub fab_control: FabControlOptions,
    pub inventory: InventoryOptions,
    pub maintenance: MaintenanceOptions,
    pub environment: EnvironmentOptions,
    pub scheduler: SchedulerOptions,
    pub safety: SafetyOptions,
    pub traceability: TraceabilityOptions,
    pub metrology: MetrologyOptions,
    pub yield_dashboard: YieldOptions,
    pub spc_fdc: SpcFdcOptions,
    pub process_flow: ProcessFlowOptions,
    pub run_to_run: RunToRunOptions,
    pub cross_section: CrossSectionOptions,
    pub notebook: NotebookOptions,
    pub experiment: ExperimentOptions,
}

impl DomainOptions {
    fn normalized(mut self) -> Self {
        self.workflow = self.workflow.normalized();
        self.mask_prep = self.mask_prep.normalized();
        self.layout_diff = self.layout_diff.normalized();
        self.fab_control = self.fab_control.normalized();
        self.inventory = self.inventory.normalized();
        self.maintenance = self.maintenance.normalized();
        self.environment = self.environment.normalized();
        self.scheduler = self.scheduler.normalized();
        self.safety = self.safety.normalized();
        self.traceability = self.traceability.normalized();
        self.metrology = self.metrology.normalized();
        self.yield_dashboard = self.yield_dashboard.normalized();
        self.spc_fdc = self.spc_fdc.normalized();
        self.process_flow = self.process_flow.normalized();
        self.run_to_run = self.run_to_run.normalized();
        self.cross_section = self.cross_section.normalized();
        self.notebook = self.notebook.normalized();
        self.experiment = self.experiment.normalized();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowOptions {
    pub load_demo_on_start: bool,
    pub focus_lot: Option<String>,
}

impl Default for WorkflowOptions {
    fn default() -> Self {
        Self {
            load_demo_on_start: true,
            focus_lot: None,
        }
    }
}

impl WorkflowOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaskPrepOptions {
    pub severity_filter: String,
    pub grouping: String,
}

impl Default for MaskPrepOptions {
    fn default() -> Self {
        Self {
            severity_filter: "all".to_string(),
            grouping: "code".to_string(),
        }
    }
}

impl MaskPrepOptions {
    fn normalized(mut self) -> Self {
        self.severity_filter = normalize_slug(self.severity_filter, MASK_SEVERITY_SLUGS, "all");
        self.grouping = normalize_slug(self.grouping, MASK_GROUPING_SLUGS, "code");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutDiffOptions {
    pub changed_only: bool,
    pub page_size: usize,
}

impl Default for LayoutDiffOptions {
    fn default() -> Self {
        Self {
            changed_only: true,
            page_size: 50,
        }
    }
}

impl LayoutDiffOptions {
    fn normalized(mut self) -> Self {
        if ![25, 50, 100, 200].contains(&self.page_size) {
            self.page_size = 50;
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FabControlOptions {
    pub auto_select_first_tool: bool,
}

impl Default for FabControlOptions {
    fn default() -> Self {
        Self {
            auto_select_first_tool: true,
        }
    }
}

impl FabControlOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InventoryOptions {
    pub quick_filter: String,
}

impl Default for InventoryOptions {
    fn default() -> Self {
        Self {
            quick_filter: "all".to_string(),
        }
    }
}

impl InventoryOptions {
    fn normalized(mut self) -> Self {
        self.quick_filter = normalize_slug(self.quick_filter, INVENTORY_FILTER_SLUGS, "all");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaintenanceOptions {
    pub work_filter: String,
    pub history_filter: String,
}

impl Default for MaintenanceOptions {
    fn default() -> Self {
        Self {
            work_filter: "actionable".to_string(),
            history_filter: "selected".to_string(),
        }
    }
}

impl MaintenanceOptions {
    fn normalized(mut self) -> Self {
        self.work_filter = normalize_slug(self.work_filter, MAINTENANCE_WORK_SLUGS, "actionable");
        self.history_filter =
            normalize_slug(self.history_filter, MAINTENANCE_HISTORY_SLUGS, "selected");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvironmentOptions {
    pub show_alarm_sensors_first: bool,
}

impl Default for EnvironmentOptions {
    fn default() -> Self {
        Self {
            show_alarm_sensors_first: true,
        }
    }
}

impl EnvironmentOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SchedulerOptions {
    pub dispatch_policy: String,
    pub minimum_priority: u8,
    pub conflicts_only: bool,
    pub focus_selected_tool: bool,
}

impl Default for SchedulerOptions {
    fn default() -> Self {
        Self {
            dispatch_policy: "priority".to_string(),
            minimum_priority: 0,
            conflicts_only: false,
            focus_selected_tool: false,
        }
    }
}

impl SchedulerOptions {
    fn normalized(mut self) -> Self {
        self.dispatch_policy =
            normalize_slug(self.dispatch_policy, DISPATCH_POLICY_SLUGS, "priority");
        self.minimum_priority = self.minimum_priority.min(9);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyOptions {
    pub show_acknowledged: bool,
}

impl Default for SafetyOptions {
    fn default() -> Self {
        Self {
            show_acknowledged: true,
        }
    }
}

impl SafetyOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceabilityOptions {
    pub impact_mode: String,
    pub related_only: bool,
}

impl Default for TraceabilityOptions {
    fn default() -> Self {
        Self {
            impact_mode: "tool-run".to_string(),
            related_only: false,
        }
    }
}

impl TraceabilityOptions {
    fn normalized(mut self) -> Self {
        self.impact_mode = normalize_slug(self.impact_mode, TRACE_IMPACT_SLUGS, "tool-run");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetrologyOptions {
    pub map_mode: String,
    pub measurement_kind: String,
    pub failed_only: bool,
}

impl Default for MetrologyOptions {
    fn default() -> Self {
        Self {
            map_mode: "value".to_string(),
            measurement_kind: "cd".to_string(),
            failed_only: false,
        }
    }
}

impl MetrologyOptions {
    fn normalized(mut self) -> Self {
        self.map_mode = normalize_slug(self.map_mode, METROLOGY_MODE_SLUGS, "value");
        self.measurement_kind = normalize_slug(self.measurement_kind, MEASUREMENT_KIND_SLUGS, "cd");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct YieldOptions {
    pub map_filter: String,
    pub attention_only: bool,
    pub excursions_only: bool,
}

impl Default for YieldOptions {
    fn default() -> Self {
        Self {
            map_filter: "all".to_string(),
            attention_only: false,
            excursions_only: false,
        }
    }
}

impl YieldOptions {
    fn normalized(mut self) -> Self {
        self.map_filter = normalize_slug(self.map_filter, YIELD_FILTER_SLUGS, "all");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpcFdcOptions {
    pub severity_filter: String,
    pub source_filter: String,
    pub context_filter: String,
}

impl Default for SpcFdcOptions {
    fn default() -> Self {
        Self {
            severity_filter: "all".to_string(),
            source_filter: "all".to_string(),
            context_filter: String::new(),
        }
    }
}

impl SpcFdcOptions {
    fn normalized(mut self) -> Self {
        self.severity_filter = normalize_slug(self.severity_filter, SPC_SEVERITY_SLUGS, "all");
        self.source_filter = normalize_slug(self.source_filter, SPC_SOURCE_SLUGS, "all");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessFlowOptions {
    pub node_filter: String,
    pub errors_only: bool,
}

impl Default for ProcessFlowOptions {
    fn default() -> Self {
        Self {
            node_filter: "all".to_string(),
            errors_only: false,
        }
    }
}

impl ProcessFlowOptions {
    fn normalized(mut self) -> Self {
        self.node_filter = normalize_slug(self.node_filter, PROCESS_FLOW_FILTER_SLUGS, "all");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunToRunOptions {
    pub loop_filter: String,
}

impl Default for RunToRunOptions {
    fn default() -> Self {
        Self {
            loop_filter: "all".to_string(),
        }
    }
}

impl RunToRunOptions {
    fn normalized(mut self) -> Self {
        self.loop_filter = normalize_slug(self.loop_filter, CONTROL_LOOP_FILTER_SLUGS, "all");
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CrossSectionOptions {
    pub step: usize,
    pub show_mask: bool,
    pub show_dimensions: bool,
    pub show_risks: bool,
}

impl Default for CrossSectionOptions {
    fn default() -> Self {
        Self {
            step: 0,
            show_mask: true,
            show_dimensions: true,
            show_risks: true,
        }
    }
}

impl CrossSectionOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotebookOptions {
    pub preview_mode: bool,
    pub followups_only: bool,
    pub tag_filter: Option<String>,
}

impl Default for NotebookOptions {
    fn default() -> Self {
        Self {
            preview_mode: true,
            followups_only: false,
            tag_filter: None,
        }
    }
}

impl NotebookOptions {
    fn normalized(self) -> Self {
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExperimentOptions {
    pub run_filter: String,
    pub show_missing_only: bool,
    pub capture_value: f64,
}

impl Default for ExperimentOptions {
    fn default() -> Self {
        Self {
            run_filter: "all".to_string(),
            show_missing_only: false,
            capture_value: 72.0,
        }
    }
}

impl ExperimentOptions {
    fn normalized(mut self) -> Self {
        self.run_filter = normalize_slug(self.run_filter, EXPERIMENT_RUN_FILTER_SLUGS, "all");
        if !self.capture_value.is_finite() {
            self.capture_value = 72.0;
        }
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_options_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var(GLASSWORKS_OPTIONS_FILE_ENV) {
        let path = std::path::PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        let config_home = std::path::PathBuf::from(config_home);
        if !config_home.as_os_str().is_empty() {
            return config_home.join("glassworks").join("options.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = std::path::PathBuf::from(home);
        if !home.as_os_str().is_empty() {
            return home.join(".config").join("glassworks").join("options.json");
        }
    }
    std::path::PathBuf::from("glassworks-options.json")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_options_file(path: impl AsRef<std::path::Path>) -> Result<AppOptions, String> {
    let path = path.as_ref();
    let json = std::fs::read_to_string(path)
        .map_err(|error| format!("read options file {}: {error}", path.display()))?;
    AppOptions::from_json_str(&json)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_options_file(
    path: impl AsRef<std::path::Path>,
    options: &AppOptions,
) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create options directory {}: {error}", parent.display()))?;
    }
    let json = options.to_pretty_json()?;
    std::fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("write options file {}: {error}", path.display()))
}

fn normalize_slug(value: String, allowed: &[&str], fallback: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if allowed.iter().any(|slug| *slug == value) {
        value
    } else {
        fallback.to_string()
    }
}

fn normalize_slug_list(values: Vec<String>, allowed: &[&str]) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_ascii_lowercase();
        if allowed.iter().any(|slug| *slug == value) && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalize_layout_hierarchy_depth(value: String) -> String {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "full" => "full".to_string(),
        "top" | "0" | "depth_0" | "depth-0" | "max_0" | "max-0" => "top".to_string(),
        "one" | "1" | "depth_1" | "depth-1" | "max_1" | "max-1" => "one".to_string(),
        "two" | "2" | "depth_2" | "depth-2" | "max_2" | "max-2" => "two".to_string(),
        "three" | "3" | "depth_3" | "depth-3" | "max_3" | "max-3" => "three".to_string(),
        "boxes" | "box" => "boxes".to_string(),
        _ => normalize_numeric_layout_hierarchy_depth(&value).unwrap_or_else(|| "full".to_string()),
    }
}

fn normalize_numeric_layout_hierarchy_depth(value: &str) -> Option<String> {
    let candidate = value
        .strip_prefix("depth_")
        .or_else(|| value.strip_prefix("depth-"))
        .or_else(|| value.strip_prefix("max_"))
        .or_else(|| value.strip_prefix("max-"))
        .unwrap_or(value);
    let depth = candidate.parse::<u8>().ok()?;
    (4..=LAYOUT_MAX_NUMERIC_HIERARCHY_DEPTH)
        .contains(&depth)
        .then(|| format!("depth_{depth}"))
}

fn normalize_layout_view_bookmarks(
    values: Vec<LayoutViewBookmarkOptions>,
) -> Vec<LayoutViewBookmarkOptions> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.normalized();
        if normalized
            .iter()
            .any(|bookmark: &LayoutViewBookmarkOptions| bookmark.slot == value.slot)
        {
            continue;
        }
        normalized.push(value);
    }
    normalized.sort_by_key(|bookmark| bookmark.slot);
    normalized
}

fn normalize_layout_layer_sets(values: Vec<LayoutLayerSetOptions>) -> Vec<LayoutLayerSetOptions> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.normalized();
        if normalized
            .iter()
            .any(|layer_set: &LayoutLayerSetOptions| layer_set.slot == value.slot)
        {
            continue;
        }
        normalized.push(value);
    }
    normalized.sort_by_key(|layer_set| layer_set.slot);
    normalized
}

fn normalize_layout_layer_depths(
    values: Vec<LayoutLayerDepthOptions>,
) -> Vec<LayoutLayerDepthOptions> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.normalized();
        if normalized
            .iter()
            .any(|depth: &LayoutLayerDepthOptions| depth.layer == value.layer)
        {
            continue;
        }
        normalized.push(value);
    }
    normalized.sort_by_key(|depth| depth.layer);
    normalized
}

fn normalize_layout_library_presets(
    values: Vec<LayoutLibraryPresetOptions>,
) -> Vec<LayoutLibraryPresetOptions> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.normalized();
        if normalized
            .iter()
            .any(|preset: &LayoutLibraryPresetOptions| preset.slot == value.slot)
        {
            continue;
        }
        normalized.push(value);
    }
    normalized.sort_by_key(|preset| preset.slot);
    normalized
}

fn normalize_layout_library_via_array_pitch_grids(size_grids: i64, pitch_grids: i64) -> i64 {
    let minimum_pitch = (size_grids + LAYOUT_LIBRARY_VIA_ARRAY_MIN_SPACING_GRIDS)
        .min(LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS);
    pitch_grids.clamp(minimum_pitch, LAYOUT_LIBRARY_VIA_ARRAY_MAX_PITCH_GRIDS)
}

fn normalize_recent_files(values: Vec<RecentFileOptions>, limit: usize) -> Vec<RecentFileOptions> {
    let mut normalized = Vec::new();
    for mut value in values {
        value.kind = normalize_slug(value.kind, RECENT_FILE_KIND_SLUGS, "workspace");
        value.path = value.path.trim().to_string();
        if value.path.is_empty()
            || normalized
                .iter()
                .any(|file: &RecentFileOptions| file.kind == value.kind && file.path == value.path)
        {
            continue;
        }
        normalized.push(value);
        if normalized.len() >= limit {
            break;
        }
    }
    normalized
}

fn sanitize_pan(pan: [f32; 2]) -> [f32; 2] {
    [
        if pan[0].is_finite() { pan[0] } else { 0.0 },
        if pan[1].is_finite() { pan[1] } else { 0.0 },
    ]
}

fn clamp_f32(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

fn clamp_f64(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_options_json_round_trips_and_normalizes() {
        let mut options = AppOptions::default();
        options.appearance.ui_scale = 9.0;
        options.layout.default_tool = "bad".to_string();
        options.layout.technology = "bad".to_string();
        options.layout.disabled_connectivity_links = vec![2, 1, 2, 200];
        options.layout.hierarchy_depth = "bad".to_string();
        options.layout.hierarchy_min_depth = 99;
        options.layout.layer_group_filter = "bad".to_string();
        options.layout.layer_usage_filter = "bad".to_string();
        options.layout.measurement_mode = "bad".to_string();
        options.layout.view_bookmarks = vec![
            LayoutViewBookmarkOptions {
                slot: 2,
                name: "  Core view  ".to_string(),
                zoom: f32::INFINITY,
                pan: [f32::NAN, 12.0],
                top_cell: 7,
                hierarchy_depth: "boxes".to_string(),
                hierarchy_min_depth: 2,
            },
            LayoutViewBookmarkOptions {
                slot: 2,
                name: "Duplicate".to_string(),
                ..Default::default()
            },
            LayoutViewBookmarkOptions {
                slot: 9,
                name: String::new(),
                hierarchy_depth: "bad".to_string(),
                ..Default::default()
            },
            LayoutViewBookmarkOptions {
                slot: 3,
                name: "Deep hierarchy".to_string(),
                hierarchy_depth: "max-6".to_string(),
                hierarchy_min_depth: 99,
                ..Default::default()
            },
            LayoutViewBookmarkOptions {
                slot: 8,
                name: "  Far view  ".to_string(),
                zoom: 0.5,
                pan: [4.0, 8.0],
                top_cell: 8,
                hierarchy_depth: "top".to_string(),
                ..Default::default()
            },
        ];
        options.layout.layer_sets = vec![
            LayoutLayerSetOptions {
                slot: 3,
                name: "  FEOL  ".to_string(),
                visible_layers: vec![7, 4, 7],
                layer_group_filter: "routing".to_string(),
                layer_usage_filter: "used".to_string(),
                layer_depths: vec![
                    LayoutLayerDepthOptions {
                        layer: 7,
                        max_depth: "max-2".to_string(),
                    },
                    LayoutLayerDepthOptions {
                        layer: 7,
                        max_depth: "top".to_string(),
                    },
                ],
            },
            LayoutLayerSetOptions {
                slot: 3,
                name: "Duplicate".to_string(),
                visible_layers: vec![99],
                layer_group_filter: "all".to_string(),
                layer_usage_filter: "all".to_string(),
                layer_depths: Vec::new(),
            },
            LayoutLayerSetOptions {
                slot: 12,
                name: String::new(),
                visible_layers: vec![2, 1],
                layer_group_filter: "bad".to_string(),
                layer_usage_filter: "bad".to_string(),
                layer_depths: vec![LayoutLayerDepthOptions {
                    layer: 1,
                    max_depth: "bad".to_string(),
                }],
            },
            LayoutLayerSetOptions {
                slot: 8,
                name: "  Metals  ".to_string(),
                visible_layers: vec![5, 4],
                layer_group_filter: "routing_metal".to_string(),
                layer_usage_filter: "empty".to_string(),
                layer_depths: Vec::new(),
            },
        ];
        options.layout.layer_depths = vec![
            LayoutLayerDepthOptions {
                layer: 4,
                max_depth: "max-2".to_string(),
            },
            LayoutLayerDepthOptions {
                layer: 4,
                max_depth: "top".to_string(),
            },
            LayoutLayerDepthOptions {
                layer: 0,
                max_depth: "boxes".to_string(),
            },
        ];
        options.layout.library_presets = vec![
            LayoutLibraryPresetOptions {
                slot: 2,
                name: "  dense vias  ".to_string(),
                macro_name: "via_array".to_string(),
                columns: 20,
                rows: 0,
                size_grids: 200,
                pitch_grids: 5,
            },
            LayoutLibraryPresetOptions {
                slot: 2,
                name: "Duplicate".to_string(),
                ..Default::default()
            },
            LayoutLibraryPresetOptions {
                slot: 9,
                name: String::new(),
                macro_name: "bad".to_string(),
                columns: 4,
                rows: 5,
                size_grids: 12,
                pitch_grids: 16,
            },
        ];
        options.performance.fps_ema_alpha = 3.0;
        options.files.recent_workspace_limit = 5;
        options.files.recent_files = vec![
            RecentFileOptions {
                kind: "session".to_string(),
                path: "target/session.json".to_string(),
            },
            RecentFileOptions {
                kind: "reference_images".to_string(),
                path: "target/ref-images.json".to_string(),
            },
            RecentFileOptions {
                kind: "def".to_string(),
                path: "target/import.def".to_string(),
            },
            RecentFileOptions {
                kind: "lef".to_string(),
                path: "target/import.lef".to_string(),
            },
            RecentFileOptions {
                kind: "layout_json".to_string(),
                path: "  target/a.json  ".to_string(),
            },
            RecentFileOptions {
                kind: "layout_json".to_string(),
                path: "target/a.json".to_string(),
            },
            RecentFileOptions {
                kind: "unknown".to_string(),
                path: "target/workspace.json".to_string(),
            },
            RecentFileOptions {
                kind: "gds".to_string(),
                path: String::new(),
            },
        ];
        options.shell.nav_rail_views = vec![
            "workflow".to_string(),
            "workflow".to_string(),
            "unknown".to_string(),
            "layout2d".to_string(),
        ];

        let json = options.to_pretty_json().expect("options should serialize");
        let parsed = AppOptions::from_json_str(&json).expect("options should parse");

        assert_eq!(parsed.appearance.ui_scale, 3.0);
        assert_eq!(parsed.layout.default_tool, "select");
        assert_eq!(parsed.layout.technology, "glassworks_demo");
        assert_eq!(parsed.layout.disabled_connectivity_links, vec![1, 2]);
        assert_eq!(parsed.layout.hierarchy_depth, "full");
        assert_eq!(parsed.layout.hierarchy_min_depth, 32);
        assert_eq!(parsed.layout.layer_group_filter, "all");
        assert_eq!(parsed.layout.layer_usage_filter, "all");
        assert_eq!(parsed.layout.measurement_mode, "direct");
        assert!(parsed.layout.show_reference_images);
        assert_eq!(parsed.layout.view_bookmarks.len(), 4);
        assert_eq!(parsed.layout.view_bookmarks[0].slot, 1);
        assert_eq!(parsed.layout.view_bookmarks[0].name, "View 1");
        assert_eq!(parsed.layout.view_bookmarks[0].hierarchy_depth, "full");
        assert_eq!(parsed.layout.view_bookmarks[1].slot, 2);
        assert_eq!(parsed.layout.view_bookmarks[1].name, "Core view");
        assert_eq!(parsed.layout.view_bookmarks[1].zoom, 0.02);
        assert_eq!(parsed.layout.view_bookmarks[1].pan, [0.0, 12.0]);
        assert_eq!(parsed.layout.view_bookmarks[1].hierarchy_depth, "boxes");
        assert_eq!(parsed.layout.view_bookmarks[1].hierarchy_min_depth, 2);
        assert_eq!(parsed.layout.view_bookmarks[2].slot, 3);
        assert_eq!(parsed.layout.view_bookmarks[2].hierarchy_depth, "depth_6");
        assert_eq!(parsed.layout.view_bookmarks[2].hierarchy_min_depth, 32);
        assert_eq!(parsed.layout.view_bookmarks[3].slot, 8);
        assert_eq!(parsed.layout.view_bookmarks[3].name, "Far view");
        assert_eq!(parsed.layout.view_bookmarks[3].zoom, 0.5);
        assert_eq!(parsed.layout.view_bookmarks[3].pan, [4.0, 8.0]);
        assert_eq!(parsed.layout.view_bookmarks[3].hierarchy_depth, "top");
        assert_eq!(parsed.layout.layer_sets.len(), 3);
        assert_eq!(parsed.layout.layer_sets[0].slot, 1);
        assert_eq!(parsed.layout.layer_sets[0].name, "Layer Set 1");
        assert_eq!(parsed.layout.layer_sets[0].visible_layers, vec![1, 2]);
        assert_eq!(parsed.layout.layer_sets[0].layer_group_filter, "all");
        assert_eq!(parsed.layout.layer_sets[0].layer_usage_filter, "all");
        assert_eq!(parsed.layout.layer_sets[0].layer_depths.len(), 1);
        assert_eq!(parsed.layout.layer_sets[0].layer_depths[0].layer, 1);
        assert_eq!(
            parsed.layout.layer_sets[0].layer_depths[0].max_depth,
            "full"
        );
        assert_eq!(parsed.layout.layer_sets[1].slot, 3);
        assert_eq!(parsed.layout.layer_sets[1].name, "FEOL");
        assert_eq!(parsed.layout.layer_sets[1].visible_layers, vec![4, 7]);
        assert_eq!(parsed.layout.layer_sets[1].layer_group_filter, "routing");
        assert_eq!(parsed.layout.layer_sets[1].layer_usage_filter, "used");
        assert_eq!(parsed.layout.layer_sets[1].layer_depths.len(), 1);
        assert_eq!(parsed.layout.layer_sets[1].layer_depths[0].layer, 7);
        assert_eq!(parsed.layout.layer_sets[1].layer_depths[0].max_depth, "two");
        assert_eq!(parsed.layout.layer_sets[2].slot, 8);
        assert_eq!(parsed.layout.layer_sets[2].name, "Metals");
        assert_eq!(parsed.layout.layer_sets[2].visible_layers, vec![4, 5]);
        assert_eq!(
            parsed.layout.layer_sets[2].layer_group_filter,
            "routing_metal"
        );
        assert_eq!(parsed.layout.layer_sets[2].layer_usage_filter, "empty");
        assert_eq!(parsed.layout.layer_depths.len(), 2);
        assert_eq!(parsed.layout.layer_depths[0].layer, 1);
        assert_eq!(parsed.layout.layer_depths[0].max_depth, "full");
        assert_eq!(parsed.layout.layer_depths[1].layer, 4);
        assert_eq!(parsed.layout.layer_depths[1].max_depth, "two");
        assert_eq!(parsed.layout.library_presets.len(), 2);
        assert_eq!(parsed.layout.library_presets[0].slot, 1);
        assert_eq!(parsed.layout.library_presets[0].name, "Via Array 1");
        assert_eq!(parsed.layout.library_presets[0].macro_name, "via_array");
        assert_eq!(parsed.layout.library_presets[0].columns, 4);
        assert_eq!(parsed.layout.library_presets[0].rows, 5);
        assert_eq!(parsed.layout.library_presets[0].size_grids, 12);
        assert_eq!(parsed.layout.library_presets[0].pitch_grids, 16);
        assert_eq!(parsed.layout.library_presets[1].slot, 2);
        assert_eq!(parsed.layout.library_presets[1].name, "dense vias");
        assert_eq!(parsed.layout.library_presets[1].columns, 16);
        assert_eq!(parsed.layout.library_presets[1].rows, 1);
        assert_eq!(parsed.layout.library_presets[1].size_grids, 80);
        assert_eq!(parsed.layout.library_presets[1].pitch_grids, 82);
        assert_eq!(parsed.performance.fps_ema_alpha, 1.0);
        assert_eq!(parsed.files.recent_workspace_limit, 5);
        assert_eq!(parsed.files.recent_files.len(), 5);
        assert_eq!(parsed.files.recent_files[0].kind, "session");
        assert_eq!(parsed.files.recent_files[0].path, "target/session.json");
        assert_eq!(parsed.files.recent_files[1].kind, "reference_images");
        assert_eq!(parsed.files.recent_files[1].path, "target/ref-images.json");
        assert_eq!(parsed.files.recent_files[2].kind, "def");
        assert_eq!(parsed.files.recent_files[2].path, "target/import.def");
        assert_eq!(parsed.files.recent_files[3].kind, "lef");
        assert_eq!(parsed.files.recent_files[3].path, "target/import.lef");
        assert_eq!(parsed.files.recent_files[4].kind, "layout_json");
        assert_eq!(parsed.files.recent_files[4].path, "target/a.json");
        assert_eq!(
            parsed.shell.nav_rail_views,
            vec!["workflow".to_string(), "layout2d".to_string()]
        );

        let mut valid = AppOptions::default();
        valid.layout.default_tool = "trace".to_string();
        valid.layout.hierarchy_depth = "4".to_string();
        valid.layout.hierarchy_min_depth = 1;
        valid.layout.layer_group_filter = "routing_metal".to_string();
        let json = valid.to_pretty_json().expect("options should serialize");
        let parsed = AppOptions::from_json_str(&json).expect("options should parse");
        assert_eq!(parsed.layout.default_tool, "trace");
        assert_eq!(parsed.layout.hierarchy_depth, "depth_4");
        assert_eq!(parsed.layout.hierarchy_min_depth, 1);
        assert_eq!(parsed.layout.layer_group_filter, "routing_metal");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn app_options_file_loads_and_saves_pretty_json() {
        let path =
            std::env::temp_dir().join(format!("glassworks-options-test-{}.json", std::process::id()));
        let mut options = AppOptions::default();
        options.appearance.unit_display = "microns".to_string();
        options.layout.show_2d_grid = false;

        save_options_file(&path, &options).expect("options file should save");
        let loaded = load_options_file(&path).expect("options file should load");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.appearance.unit_display, "microns");
        assert!(!loaded.layout.show_2d_grid);
    }
}
