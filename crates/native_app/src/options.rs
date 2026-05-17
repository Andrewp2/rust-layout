use serde::{Deserialize, Serialize};

pub const APP_OPTIONS_SCHEMA_VERSION: u32 = 1;
pub const FABRICAD_OPTIONS_FILE_ENV: &str = "FABRICAD_OPTIONS_FILE";

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
    "select", "rect", "polygon", "path", "via", "measure", "route",
];
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
    pub active_layer: u32,
    pub snap_enabled: bool,
    pub show_2d_grid: bool,
    pub show_origin_marker: bool,
    pub show_drc_overlay: bool,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub constrain_with_shift: bool,
    pub bypass_snap_with_ctrl: bool,
    pub duplicate_drag_with_alt: bool,
}

impl Default for LayoutEditorOptions {
    fn default() -> Self {
        Self {
            default_tool: "select".to_string(),
            active_layer: 4,
            snap_enabled: true,
            show_2d_grid: true,
            show_origin_marker: true,
            show_drc_overlay: true,
            zoom: 0.02,
            pan: [0.0, 0.0],
            constrain_with_shift: true,
            bypass_snap_with_ctrl: true,
            duplicate_drag_with_alt: true,
        }
    }
}

impl LayoutEditorOptions {
    fn normalized(mut self) -> Self {
        self.default_tool = normalize_slug(self.default_tool, TOOL_SLUGS, "select");
        self.zoom = clamp_f32(self.zoom, 0.000_05, 512.0, 0.02);
        self.pan = sanitize_pan(self.pan);
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
    pub prompt_before_destructive_actions: bool,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            autosave_enabled: false,
            autosave_interval_seconds: 120,
            remember_last_workspace: true,
            recent_workspace_limit: 10,
            prompt_before_destructive_actions: true,
        }
    }
}

impl FileOptions {
    fn normalized(mut self) -> Self {
        self.autosave_interval_seconds = self.autosave_interval_seconds.clamp(15, 3600);
        self.recent_workspace_limit = self.recent_workspace_limit.clamp(0, 50);
        self
    }
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
    if let Ok(path) = std::env::var(FABRICAD_OPTIONS_FILE_ENV) {
        let path = std::path::PathBuf::from(path);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        let config_home = std::path::PathBuf::from(config_home);
        if !config_home.as_os_str().is_empty() {
            return config_home.join("fabricad").join("options.json");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = std::path::PathBuf::from(home);
        if !home.as_os_str().is_empty() {
            return home.join(".config").join("fabricad").join("options.json");
        }
    }
    std::path::PathBuf::from("fabricad-options.json")
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
        options.performance.fps_ema_alpha = 3.0;
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
        assert_eq!(parsed.performance.fps_ema_alpha, 1.0);
        assert_eq!(
            parsed.shell.nav_rail_views,
            vec!["workflow".to_string(), "layout2d".to_string()]
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn app_options_file_loads_and_saves_pretty_json() {
        let path =
            std::env::temp_dir().join(format!("fabricad-options-test-{}.json", std::process::id()));
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
