#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use drc::{DrcViolation, RuleDeck, run_drc, run_drc_incremental};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Painter, PointerButton, Pos2, Rect as EguiRect,
    RichText, Sense, Stroke, StrokeKind, Vec2, vec2,
};
use geometry_core::{Coord, Point, Rect, Vector, distance_point_to_segment};
use layout_model::{
    Cell, CellId, CellInstance, ClientMessage, CrdtApplyResult, CrdtOperation, Document,
    InstanceArray, InstanceId, LayerId, LayoutIndex, LoroCrdtLog, LoroUpdate, MarkerState, NetId,
    Operation, ProcessLayer, ServerMessage, Shape, ShapeId, ShapeKind, ShapeOccurrenceId,
    TechnologyFile, Transform, builtin_technologies,
    connectivity::{ConnectivityReport, NetComponent, extract_connectivity},
    equipment::{
        AlarmSeverity, EquipmentEvent, EquipmentSimulator, HostCommand,
        RecipeId as EquipmentRecipeId, RecipeSelection, RunStatus, SensorSample,
        Tool as EquipmentTool, ToolId as EquipmentToolId, ToolKind as EquipmentToolKind,
        ToolState as EquipmentToolState,
    },
    experiment::ExperimentPlan,
    gdsii::{export_gdsii, import_gdsii},
    genealogy::LotGenealogy,
    mes::{
        AuditOutcome, FabMesData, Lot, LotId, OperatorAction, ProcessRoute, ToolId, TravelerState,
        TravelerStatus,
    },
    metrology::{
        DieCoord, FabObjectLinks, HistogramBin, Measurement, MeasurementKind, MeasurementStatus,
        WaferGeometry, WaferMap,
    },
    process_control::ProcessControlModel,
    recipe::RecipeCatalog,
    yield_analysis::{
        CorrelationRecord, DieOutcome, FailureMode, LotComparison, ProcessMeasurement,
        YieldAnalysis, YieldSummary,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use renderer::gpu::OffscreenRenderRequest;
use renderer::gpu::{
    BufferUploadResult, GpuPickRequest, GpuUploadStats, LayoutGpuRenderer, ViewUniforms,
    Viewport3dRenderer, Viewport3dUniforms, viewport_3d_target_size,
};
use router::{RouteRequest, RouterConfig, route};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use web_time::{Duration, Instant};

mod experiment_panel;
mod genealogy_panel;
mod mask_panel;
mod process_control_panel;
mod recipe_panel;
mod spc_fdc_panel;
mod ui_chrome;
use experiment_panel::ExperimentPlannerPanel;
use genealogy_panel::GenealogyPanel;
use mask_panel::MaskPrepPanel;
use process_control_panel::ProcessControlPanel;
use recipe_panel::RecipeManagerPanel;
use spc_fdc_panel::SpcFdcPanel;

#[cfg(not(target_arch = "wasm32"))]
use futures_util::{Sink, SinkExt, StreamExt};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

const SAVE_PATH: &str = "examples/fabricad_layout.json";
const WORKSPACE_PATH: &str = "examples/fabricad_workspace.json";
const DEMO_WORKSPACE_PATH: &str = "examples/fabricad_demo_workspace.json";
const GDS_PATH: &str = "examples/fabricad_layout.gds";
#[cfg(target_arch = "wasm32")]
const WASM_AUTOSAVE_KEY: &str = "fabricad.autosave.document";
const MAX_LORO_SEED_OBJECTS: usize = 5_000;
const MAX_CONNECTIVITY_OBJECTS: usize = 50_000;
const TILE_MEMORY_BUDGET_BYTES: usize = 96 * 1024 * 1024;
const MAX_3D_FACES: usize = 24_000;
const CAMERA_NEAR_PLANE: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Select,
    Rect,
    Polygon,
    Path,
    Via,
    Measure,
    Route,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewMode {
    Layout2d,
    Layout3d,
    FabControl,
    Metrology,
    Yield,
    MaskPrep,
    SpcFdc,
    ProcessControl,
    Traceability,
    Experiment,
}

impl ViewMode {
    const ALL: [Self; 10] = [
        Self::Layout2d,
        Self::Layout3d,
        Self::MaskPrep,
        Self::FabControl,
        Self::Metrology,
        Self::Yield,
        Self::SpcFdc,
        Self::ProcessControl,
        Self::Traceability,
        Self::Experiment,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Layout2d => "Layout Editor",
            Self::Layout3d => "3D Layout View",
            Self::FabControl => "Fab Control Room",
            Self::Metrology => "Metrology Wafer Map",
            Self::Yield => "Yield Dashboard",
            Self::MaskPrep => "Mask / Reticle Prep",
            Self::SpcFdc => "SPC / FDC Monitor",
            Self::ProcessControl => "Run-to-Run Control",
            Self::Traceability => "Lot Traceability",
            Self::Experiment => "DOE Planner",
        }
    }

    fn nav_label(self) -> &'static str {
        match self {
            Self::Layout2d => "Mask layout",
            Self::Layout3d => "3D viewport",
            Self::FabControl => "Equipment",
            Self::Metrology => "Metrology",
            Self::Yield => "Yield",
            Self::MaskPrep => "Reticle prep",
            Self::SpcFdc => "SPC / FDC",
            Self::ProcessControl => "R2R control",
            Self::Traceability => "Traceability",
            Self::Experiment => "DOE",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::Layout2d => "Geometry, hierarchy, routing, DRC, and collaboration",
            Self::Layout3d => "Extruded process stack preview with GPU depth rendering",
            Self::FabControl => "Tool state, alarms, recipes, runs, and sensor streams",
            Self::Metrology => "Wafer-level measurements, bins, defects, and outliers",
            Self::Yield => "Lot, wafer, failure, recipe, and correlation analysis",
            Self::MaskPrep => "Reticle fields, exposure blocks, layer tone, and mask checks",
            Self::SpcFdc => "Control charts, fault traces, active alarms, and findings",
            Self::ProcessControl => "EWMA loops, proposed adjustments, and approval audit",
            Self::Traceability => "Lot splits, material ancestry, process history, and impact",
            Self::Experiment => "Factor matrix, run status, response capture, and effects",
        }
    }

    fn status_message(self) -> &'static str {
        match self {
            Self::Layout2d => "layout editor",
            Self::Layout3d => "3D flycam view",
            Self::FabControl => "FabOS equipment control room",
            Self::Metrology => "metrology wafer map",
            Self::Yield => "FabOS yield dashboard",
            Self::MaskPrep => "FabOS mask editor / reticle prep",
            Self::SpcFdc => "FabOS SPC/FDC monitor",
            Self::ProcessControl => "FabOS run-to-run process control",
            Self::Traceability => "FabOS lot genealogy trace",
            Self::Experiment => "FabOS DOE planner",
        }
    }

    fn group(self) -> ModuleGroup {
        match self {
            Self::Layout2d | Self::Layout3d | Self::MaskPrep => ModuleGroup::Design,
            Self::FabControl | Self::Traceability => ModuleGroup::Operations,
            Self::Metrology | Self::Yield | Self::SpcFdc => ModuleGroup::Analysis,
            Self::ProcessControl | Self::Experiment => ModuleGroup::Engineering,
        }
    }

    fn is_layout(self) -> bool {
        matches!(self, Self::Layout2d | Self::Layout3d)
    }

    fn has_inspector_panel(self) -> bool {
        !matches!(self, Self::FabControl | Self::Yield)
    }

    fn has_secondary_panel(self) -> bool {
        matches!(
            self,
            Self::Layout2d | Self::Layout3d | Self::Metrology | Self::MaskPrep | Self::Experiment
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModuleGroup {
    Design,
    Operations,
    Analysis,
    Engineering,
}

impl ModuleGroup {
    const ALL: [Self; 4] = [
        Self::Design,
        Self::Operations,
        Self::Analysis,
        Self::Engineering,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Design => "Design",
            Self::Operations => "Operations",
            Self::Analysis => "Analysis",
            Self::Engineering => "Engineering",
        }
    }

    fn tone(self) -> ui_chrome::Tone {
        match self {
            Self::Design => ui_chrome::Tone::Info,
            Self::Operations => ui_chrome::Tone::Success,
            Self::Analysis => ui_chrome::Tone::Warning,
            Self::Engineering => ui_chrome::Tone::Neutral,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DataSource {
    Blank,
    Demo,
    File(String),
    Generated(String),
}

impl DataSource {
    fn label(&self) -> String {
        match self {
            Self::Blank => "blank".to_string(),
            Self::Demo => "demo".to_string(),
            Self::File(_) => "file".to_string(),
            Self::Generated(_) => "generated".to_string(),
        }
    }

    fn tone(&self) -> ui_chrome::Tone {
        match self {
            Self::Blank => ui_chrome::Tone::Neutral,
            Self::Demo | Self::Generated(_) => ui_chrome::Tone::Warning,
            Self::File(_) => ui_chrome::Tone::Success,
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::File(path) | Self::Generated(path) => Some(path.as_str()),
            Self::Blank | Self::Demo => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkspaceDataset {
    schema_version: u32,
    document: Document,
    mes: FabMesData,
    yield_analysis: YieldAnalysis,
    wafer_map: WaferMap,
    recipe_catalog: RecipeCatalog,
    genealogy: LotGenealogy,
    experiment_plan: ExperimentPlan,
    process_control: ProcessControlModel,
    equipment: EquipmentSimulator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitDisplay {
    Auto,
    Nanometers,
    Microns,
    Dbu,
}

impl UnitDisplay {
    const ALL: [Self; 4] = [Self::Auto, Self::Nanometers, Self::Microns, Self::Dbu];

    fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Nanometers => "Nanometers",
            Self::Microns => "Microns",
            Self::Dbu => "DBU",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTheme {
    Dark,
    Light,
}

impl EditorTheme {
    const ALL: [Self; 2] = [Self::Dark, Self::Light];

    fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct EditorSettings {
    snap_enabled: bool,
    show_grid_2d: bool,
    show_grid_3d: bool,
    show_origin_marker: bool,
    min_grid_pixels: f32,
    units: UnitDisplay,
    unit_precision: usize,
    theme: EditorTheme,
    autosave_enabled: bool,
    autosave_interval_seconds: u64,
    single_key_shortcuts: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            snap_enabled: true,
            show_grid_2d: true,
            show_grid_3d: true,
            show_origin_marker: true,
            min_grid_pixels: 24.0,
            units: UnitDisplay::Auto,
            unit_precision: 2,
            theme: EditorTheme::Dark,
            autosave_enabled: false,
            autosave_interval_seconds: 30,
            single_key_shortcuts: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Camera3d {
    position: Vec3f,
    yaw: f32,
    pitch: f32,
    speed: f32,
    fov_y: f32,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            position: Vec3f::new(-8_000.0, -9_000.0, 6_000.0),
            yaw: 45.0_f32.to_radians(),
            pitch: -28.0_f32.to_radians(),
            speed: 4_000.0,
            fov_y: 58.0_f32.to_radians(),
        }
    }
}

impl Camera3d {
    fn look_at(position: Vec3f, target: Vec3f, speed: f32) -> Self {
        let direction = (target - position).normalized();
        let yaw = direction.y.atan2(direction.x);
        let horizontal = (direction.x * direction.x + direction.y * direction.y).sqrt();
        let pitch = direction.z.atan2(horizontal);
        Self {
            position,
            yaw,
            pitch: pitch.clamp(-1.45, 1.45),
            speed: speed.max(100.0),
            fov_y: 58.0_f32.to_radians(),
        }
    }

    fn forward(self) -> Vec3f {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch.sin_cos();
        Vec3f::new(yaw_cos * pitch_cos, yaw_sin * pitch_cos, pitch_sin).normalized()
    }

    fn right(self) -> Vec3f {
        Vec3f::new(-self.yaw.sin(), self.yaw.cos(), 0.0)
    }

    fn up(self) -> Vec3f {
        self.forward().cross(self.right()).normalized()
    }

    fn basis(self) -> CameraBasis {
        CameraBasis {
            forward: self.forward(),
            right: self.right(),
            up: self.up(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Vec3f {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3f {
    const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            Self::ZERO
        } else {
            self / length
        }
    }
}

impl std::ops::Add for Vec3f {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for Vec3f {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Vec3f {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3f {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f32> for Vec3f {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Clone, Copy, Debug)]
struct CameraBasis {
    forward: Vec3f,
    right: Vec3f,
    up: Vec3f,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CameraPoint3d {
    x: f32,
    y: f32,
    depth: f32,
}

impl CameraPoint3d {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            depth: self.depth + (other.depth - self.depth) * t,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceSurface3d {
    Side,
    Top,
}

impl FaceSurface3d {
    fn draw_order(self) -> u8 {
        match self {
            Self::Side => 0,
            Self::Top => 1,
        }
    }
}

#[derive(Clone, Debug)]
struct Face3d {
    surface: FaceSurface3d,
    order: usize,
    points: Vec<Vec3f>,
    fill: Color32,
    stroke: Color32,
}

#[derive(Clone, Debug)]
struct ProjectedFace {
    surface: FaceSurface3d,
    depth: f32,
    order: usize,
    points: Vec<Pos2>,
    fill: Color32,
    stroke: Color32,
}

#[derive(Clone, Debug)]
struct UndoEntry {
    undo: Operation,
    redo: Operation,
}

#[derive(Clone, Debug, Default)]
struct RouteNetMetadata {
    net: Option<NetId>,
    name: Option<String>,
}

#[derive(Clone, Debug)]
enum MarkerAction {
    FocusAndSelect(DrcViolation),
    SetWaived(String, bool),
    SetHidden(String, bool),
}

#[derive(Clone, Debug, Default)]
struct PerfStats {
    visible_count: usize,
    query_ms: f64,
    gpu_build_ms: f64,
    gpu_vertices: usize,
    gpu_indices: usize,
    gpu_pick_vertices: usize,
    gpu_pick_indices: usize,
    gpu_upload_bytes: usize,
    gpu_resident_bytes: usize,
    gpu_layout_uploads: u64,
    gpu_layout_skips: u64,
    gpu_pick_uploads: u64,
    gpu_pick_skips: u64,
    tile_visible_count: usize,
    tile_resident_count: usize,
    tile_rebuilt_count: usize,
    tile_evicted_count: usize,
    tile_dirty_count: usize,
    tile_lod_count: usize,
    tile_lod_shape_count: usize,
    tile_precise_shape_count: usize,
    tile_cached_count: usize,
    tile_shape_count: usize,
    tile_pick_shape_count: usize,
    shape_cache_hits: usize,
    shape_cache_misses: usize,
    resident_shape_batches: usize,
    evicted_shape_batches: usize,
    draw_range_count: usize,
    tile_cache_bytes: usize,
    tile_memory_budget_bytes: usize,
    tile_over_budget_bytes: usize,
    drc_ms: f64,
    connectivity_ms: f64,
    route_ms: f64,
    pick_build_ms: f64,
    batch_bytes: usize,
    pick_batch_bytes: usize,
    frame_ms: f64,
}

#[derive(Clone, Debug, Default)]
struct RenderInvalidation {
    rects: Vec<Rect>,
    shape_ids: BTreeSet<ShapeId>,
    clear_all: bool,
}

impl RenderInvalidation {
    fn dirty_bounds(&self) -> Option<Rect> {
        if self.clear_all {
            return None;
        }
        self.rects
            .iter()
            .copied()
            .reduce(|left, right| left.union(right))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CollabClient {
    outbound: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    inbound: std::sync::mpsc::Receiver<ServerMessage>,
}

#[cfg(target_arch = "wasm32")]
struct CollabClient {
    socket: web_sys::WebSocket,
    inbound: Rc<RefCell<Vec<ServerMessage>>>,
    outbound: Rc<RefCell<Vec<ClientMessage>>>,
    _on_open: Closure<dyn FnMut(web_sys::Event)>,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
    _on_close: Closure<dyn FnMut(web_sys::CloseEvent)>,
}

#[derive(Clone, Copy, Debug)]
struct InstanceDrag {
    parent: CellId,
    id: InstanceId,
}

#[derive(Clone, Copy, Debug)]
struct VertexDrag {
    shape: ShapeId,
    vertex: usize,
}

#[derive(Clone, Copy, Debug)]
struct EdgeDrag {
    shape: ShapeId,
    edge: usize,
}

#[derive(Clone, Debug)]
struct SelectedInstanceInfo {
    parent: CellId,
    id: InstanceId,
    target_cell: CellId,
    target_cell_name: String,
    name: Option<String>,
    transform: Transform,
    translation: Vector,
    array: InstanceArray,
}

pub struct FabricadApp {
    document: Document,
    layout_source: DataSource,
    fabos_source: DataSource,
    index: LayoutIndex,
    yield_analysis: YieldAnalysis,
    selected_yield_lot: String,
    selected_yield_wafer: String,
    mask_panel: MaskPrepPanel,
    spc_fdc_panel: SpcFdcPanel,
    process_control_panel: ProcessControlPanel,
    genealogy_panel: GenealogyPanel,
    experiment_panel: ExperimentPlannerPanel,
    technologies: Vec<TechnologyFile>,
    active_technology: usize,
    rules: RuleDeck,
    violations: Vec<DrcViolation>,
    connectivity: ConnectivityReport,
    show_hidden_markers: bool,
    show_waived_markers: bool,
    selected: BTreeSet<ShapeId>,
    selected_occurrence: Option<ShapeOccurrenceId>,
    active_layer: LayerId,
    tool: Tool,
    view_mode: ViewMode,
    wafer_map: WaferMap,
    metrology_kind: MeasurementKind,
    selected_die: Option<DieCoord>,
    metrology_failed_only: bool,
    zoom: f32,
    pan: Vec2,
    camera_3d: Camera3d,
    settings: EditorSettings,
    show_options: bool,
    show_diagnostics: bool,
    drawing_start: Option<Point>,
    drawing_points: Vec<Point>,
    measure_start: Option<Point>,
    route_points: Vec<Point>,
    drag_shape: Option<ShapeId>,
    drag_instance: Option<InstanceDrag>,
    drag_vertex: Option<VertexDrag>,
    drag_edge: Option<EdgeDrag>,
    last_drag_world: Option<Point>,
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
    clipboard_shapes: Vec<Shape>,
    perf: PerfStats,
    last_drc_run: Instant,
    last_autosave: Instant,
    status: String,
    user_id: Uuid,
    loro_log: LoroCrdtLog,
    remote_cursors: BTreeMap<Uuid, Point>,
    remote_selections: BTreeMap<Uuid, Vec<ShapeOccurrenceId>>,
    last_cursor_position: Option<Point>,
    last_broadcast_selection: Vec<ShapeOccurrenceId>,
    last_view_center: Point,
    gpu_target_format: Option<egui_wgpu::wgpu::TextureFormat>,
    tile_cache: renderer::TileCache,
    gpu_pick_state: SharedGpuPickState,
    gpu_upload_state: SharedGpuUploadState,
    gpu_pick_serial: u64,
    collab: Option<CollabClient>,
    cell_name_drafts: BTreeMap<CellId, String>,
    instance_name_drafts: BTreeMap<(CellId, InstanceId), String>,
    mes: FabMesData,
    selected_mes_lot: Option<LotId>,
    show_mes_panel: bool,
    mes_operator: String,
    recipe_panel: RecipeManagerPanel,
    equipment_sim: EquipmentSimulator,
    selected_equipment_tool: Option<EquipmentToolId>,
    equipment_recipe_drafts: BTreeMap<EquipmentToolId, EquipmentRecipeId>,
    last_equipment_tick: Instant,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StartupOptions {
    pub stress_count: Option<usize>,
    pub hierarchy_demo: bool,
    pub view_3d: bool,
    pub show_options: bool,
    pub select_first_shape: bool,
    pub move_first_vertex: bool,
    pub zoom: Option<f32>,
    pub pan: Option<[f32; 2]>,
    pub hierarchy_workflow_demo: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub enum OffscreenScene {
    Demo,
    Hierarchy,
    Stress { count: usize },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub struct OffscreenRenderOptions {
    pub scene: OffscreenScene,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub pan: [f32; 2],
}

#[cfg(not(target_arch = "wasm32"))]
pub type OffscreenRenderReport = renderer::gpu::OffscreenRenderReport;

#[cfg(not(target_arch = "wasm32"))]
impl Default for OffscreenRenderOptions {
    fn default() -> Self {
        Self {
            scene: OffscreenScene::Demo,
            width: 1280,
            height: 720,
            zoom: 0.075,
            pan: [0.0, 0.0],
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_offscreen_render(
    options: OffscreenRenderOptions,
) -> Result<OffscreenRenderReport, Box<dyn std::error::Error>> {
    pollster::block_on(run_offscreen_render_async(options))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_demo_gds(
    path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    use layout_model::default_technology;

    let path = path.as_ref();
    let document = Document::demo();
    let technology = default_technology();
    let bytes = export_gdsii(&document, &technology)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

impl WorkspaceDataset {
    fn blank() -> Self {
        Self {
            schema_version: 1,
            document: Document::new("Untitled layout"),
            mes: empty_mes_data(),
            yield_analysis: empty_yield_analysis(),
            wafer_map: empty_wafer_map(),
            recipe_catalog: RecipeCatalog::default(),
            genealogy: LotGenealogy::new(),
            experiment_plan: ExperimentPlannerPanel::empty().plan().clone(),
            process_control: ProcessControlModel::default(),
            equipment: EquipmentSimulator::new(Vec::new()),
        }
    }

    fn demo() -> Self {
        let yield_analysis = YieldAnalysis::synthetic();
        Self {
            schema_version: 1,
            document: Document::demo(),
            mes: FabMesData::sample(),
            yield_analysis: yield_analysis.clone(),
            wafer_map: WaferMap::synthetic_demo(),
            recipe_catalog: RecipeCatalog::sample(),
            genealogy: LotGenealogy::sample(),
            experiment_plan: ExperimentPlan::sample(),
            process_control: ProcessControlModel::from_yield_analysis(&yield_analysis),
            equipment: EquipmentSimulator::demo_fab(),
        }
    }
}

fn empty_mes_data() -> FabMesData {
    FabMesData {
        routes: BTreeMap::new(),
        lots: BTreeMap::new(),
        travelers: BTreeMap::new(),
    }
}

fn empty_yield_analysis() -> YieldAnalysis {
    YieldAnalysis {
        lots: Vec::new(),
        recipes: Vec::new(),
        test_results: Vec::new(),
        process_measurements: Vec::new(),
        lot_summaries: Vec::new(),
        wafer_summaries: Vec::new(),
        lot_comparisons: Vec::new(),
        correlations: Vec::new(),
    }
}

fn empty_wafer_map() -> WaferMap {
    WaferMap {
        id: String::new(),
        name: "No wafer map loaded".to_string(),
        geometry: WaferGeometry::default(),
        links: FabObjectLinks::default(),
        dies: Vec::new(),
        measurements: Vec::new(),
        defects: Vec::new(),
        annotations: Vec::new(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_offscreen_render_async(
    options: OffscreenRenderOptions,
) -> Result<OffscreenRenderReport, Box<dyn std::error::Error>> {
    let width = options.width.max(1);
    let height = options.height.max(1);
    let zoom = options.zoom.clamp(0.001, 32.0);
    let (scene_name, document) = offscreen_document(&options.scene);
    renderer::gpu::render_document_offscreen(OffscreenRenderRequest {
        scene: scene_name,
        document,
        width,
        height,
        zoom,
        pan: options.pan,
    })
    .await
}

#[cfg(not(target_arch = "wasm32"))]
fn offscreen_document(scene: &OffscreenScene) -> (String, Document) {
    match scene {
        OffscreenScene::Demo => ("demo".to_string(), Document::demo()),
        OffscreenScene::Hierarchy => ("hierarchy".to_string(), Document::hierarchy_demo()),
        OffscreenScene::Stress { count } => (format!("stress:{count}"), Document::stress(*count)),
    }
}

impl FabricadApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let technologies = builtin_technologies();
        let dataset = WorkspaceDataset::blank();
        let document = dataset.document;
        let yield_analysis = dataset.yield_analysis;
        let selected_yield_lot = yield_analysis
            .lots
            .first()
            .map(|lot| lot.id.clone())
            .unwrap_or_default();
        let selected_yield_wafer = yield_analysis
            .wafer_ids_for_lot(&selected_yield_lot)
            .first()
            .cloned()
            .unwrap_or_default();
        let user_id = Uuid::new_v4();
        let mut loro_log = LoroCrdtLog::new(user_id).expect("create Loro CRDT log");
        if document_object_count(&document) <= MAX_LORO_SEED_OBJECTS {
            loro_log
                .seed_document_objects(&document)
                .expect("seed Loro object store");
        }
        let active_layer = document
            .layer_by_process(ProcessLayer::Metal1)
            .unwrap_or(LayerId(1));
        let active_technology = 0;
        let rules = rule_deck_for_document(&document, &technologies[active_technology]);
        let index = build_layout_index(&document);
        let violations = run_drc(&document, &rules);
        let connectivity =
            connectivity_report_for_document(&document, &technologies[active_technology]);
        let mes = dataset.mes;
        let selected_mes_lot = mes.lots.keys().next().cloned();
        let equipment_sim = dataset.equipment;
        let selected_equipment_tool = equipment_sim.tools().next().map(|tool| tool.id.clone());
        let mask_panel = MaskPrepPanel::new(&document, &mes, selected_mes_lot.as_ref());
        let spc_fdc_panel = SpcFdcPanel::new();
        let process_control_panel = ProcessControlPanel::from_model(dataset.process_control);
        let genealogy_panel = GenealogyPanel::from_genealogy(dataset.genealogy);
        let experiment_panel = ExperimentPlannerPanel::from_plan(dataset.experiment_plan);
        let mut app = Self {
            document,
            layout_source: DataSource::Blank,
            fabos_source: DataSource::Blank,
            index,
            yield_analysis,
            selected_yield_lot,
            selected_yield_wafer,
            mask_panel,
            spc_fdc_panel,
            process_control_panel,
            genealogy_panel,
            experiment_panel,
            technologies,
            active_technology,
            rules,
            violations,
            connectivity,
            show_hidden_markers: false,
            show_waived_markers: true,
            selected: BTreeSet::new(),
            selected_occurrence: None,
            active_layer,
            tool: Tool::Select,
            view_mode: ViewMode::Layout2d,
            wafer_map: dataset.wafer_map,
            metrology_kind: MeasurementKind::ThicknessNm,
            selected_die: None,
            metrology_failed_only: false,
            zoom: 0.075,
            pan: Vec2::ZERO,
            camera_3d: Camera3d::default(),
            settings: EditorSettings::default(),
            show_options: false,
            show_diagnostics: false,
            drawing_start: None,
            drawing_points: Vec::new(),
            measure_start: None,
            route_points: Vec::new(),
            drag_shape: None,
            drag_instance: None,
            drag_vertex: None,
            drag_edge: None,
            last_drag_world: None,
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard_shapes: Vec::new(),
            perf: PerfStats::default(),
            last_drc_run: Instant::now(),
            last_autosave: Instant::now(),
            status: "blank workspace".to_string(),
            user_id,
            loro_log,
            remote_cursors: BTreeMap::new(),
            remote_selections: BTreeMap::new(),
            last_cursor_position: None,
            last_broadcast_selection: Vec::new(),
            last_view_center: Point::ZERO,
            gpu_target_format: cc
                .wgpu_render_state
                .as_ref()
                .map(|render_state| render_state.target_format),
            tile_cache: renderer::TileCache::default(),
            gpu_pick_state: Arc::new(Mutex::new(GpuPickState::default())),
            gpu_upload_state: Arc::new(Mutex::new(GpuUploadStats::default())),
            gpu_pick_serial: 0,
            collab: None,
            cell_name_drafts: BTreeMap::new(),
            instance_name_drafts: BTreeMap::new(),
            mes,
            selected_mes_lot,
            show_mes_panel: true,
            mes_operator: "op.demo".to_string(),
            recipe_panel: RecipeManagerPanel::from_catalog(dataset.recipe_catalog),
            equipment_sim,
            selected_equipment_tool,
            equipment_recipe_drafts: BTreeMap::new(),
            last_equipment_tick: Instant::now(),
        };
        app.reset_3d_camera_to_document();
        app
    }

    pub fn new_with_options(cc: &eframe::CreationContext<'_>, options: StartupOptions) -> Self {
        let mut app = Self::new(cc);
        if options.hierarchy_demo {
            app.document = Document::hierarchy_demo();
            app.apply_current_technology_to_document();
            app.selected.clear();
            app.selected_occurrence = None;
            app.undo.clear();
            app.redo.clear();
            app.clear_edit_drafts();
            app.reset_loro_log_from_document();
            app.reset_render_cache();
            app.rebuild_indexes();
            app.rerun_drc();
            app.layout_source = DataSource::Demo;
            app.status = "test scene: hierarchy".to_string();
        } else if let Some(count) = options.stress_count {
            app.document = Document::stress(count);
            app.apply_current_technology_to_document();
            app.selected.clear();
            app.selected_occurrence = None;
            app.undo.clear();
            app.redo.clear();
            app.clear_edit_drafts();
            app.reset_loro_log_from_document();
            app.reset_render_cache();
            app.rebuild_indexes();
            app.rerun_drc();
            app.layout_source = DataSource::Generated(format!("{count} polygon stress"));
            app.status = format!("test scene: {count} polygons");
        }
        app.reset_3d_camera_to_document();
        if let Some(zoom) = options.zoom {
            app.zoom = zoom.clamp(0.008, 4.0);
        }
        if let Some([x, y]) = options.pan {
            app.pan = vec2(x, y);
        }
        if options.select_first_shape {
            app.select_first_top_level_shape();
        }
        if options.move_first_vertex {
            app.apply_first_vertex_demo_edit();
        }
        if options.hierarchy_workflow_demo {
            app.apply_hierarchy_workflow_demo();
        }
        if options.view_3d {
            app.view_mode = ViewMode::Layout3d;
            app.reset_3d_camera_to_document();
        }
        if options.show_options {
            app.show_options = true;
        }
        app
    }

    fn rebuild_indexes(&mut self) {
        self.index = build_layout_index(&self.document);
        self.rebuild_connectivity();
    }

    fn rerun_drc(&mut self) {
        let started = Instant::now();
        self.violations = run_drc(&self.document, &self.rules);
        self.perf.drc_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.last_drc_run = Instant::now();
    }

    fn rerun_drc_for_dirty_region(&mut self, dirty_region: Option<Rect>) {
        let Some(dirty_region) = dirty_region else {
            self.rerun_drc();
            return;
        };
        let started = Instant::now();
        self.violations =
            run_drc_incremental(&self.document, &self.rules, &self.violations, dirty_region);
        self.perf.drc_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.last_drc_run = Instant::now();
    }

    fn active_marker_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|violation| {
                let state = self.marker_state(violation);
                !state.hidden && !state.waived
            })
            .count()
    }

    fn marker_state(&self, violation: &DrcViolation) -> MarkerState {
        self.document
            .marker_states
            .get(&drc_marker_key(violation))
            .cloned()
            .unwrap_or_default()
    }

    fn set_marker_state(&mut self, key: String, update: impl FnOnce(&mut MarkerState)) {
        let state = self.document.marker_states.entry(key.clone()).or_default();
        update(state);
        if *state == MarkerState::default() {
            self.document.marker_states.remove(&key);
        }
    }

    fn clear_marker_states(&mut self) {
        self.document.marker_states.clear();
        self.status = "cleared marker waivers and hidden states".to_string();
    }

    fn rebuild_connectivity(&mut self) {
        let started = Instant::now();
        self.connectivity =
            connectivity_report_for_document(&self.document, self.current_technology());
        self.perf.connectivity_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    fn current_technology(&self) -> &TechnologyFile {
        &self.technologies[self
            .active_technology
            .min(self.technologies.len().saturating_sub(1))]
    }

    fn snap_grid(&self) -> Coord {
        self.document.grid.max(1)
    }

    fn snap_point(&self, point: Point) -> Point {
        if self.settings.snap_enabled {
            point.snap(self.snap_grid())
        } else {
            point
        }
    }

    fn edit_step(&self) -> f64 {
        self.snap_grid() as f64
    }

    fn minimum_draw_size(&self) -> Coord {
        if self.settings.snap_enabled {
            self.snap_grid()
        } else {
            1
        }
    }

    fn set_document_grid(&mut self, grid: Coord) {
        let grid = grid.clamp(1, 10_000_000);
        if self.document.grid == grid {
            return;
        }
        self.document.grid = grid;
        self.reset_render_cache();
        self.status = format!("grid spacing set to {grid} dbu");
    }

    fn format_length(&self, length_dbu: f64) -> String {
        format_physical_length_with_options(
            length_dbu,
            self.current_technology().dbu_per_micron,
            self.settings.units,
            self.settings.unit_precision,
        )
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ui_chrome::apply_workspace_style(ctx, matches!(self.settings.theme, EditorTheme::Dark));
    }

    fn maybe_autosave(&mut self) {
        if !self.settings.autosave_enabled {
            return;
        }
        let interval = Duration::from_secs(self.settings.autosave_interval_seconds.max(5));
        if self.last_autosave.elapsed() >= interval {
            self.autosave_document();
            self.last_autosave = Instant::now();
        }
    }

    fn rebuild_rules_from_technology(&mut self) {
        let technology = self.current_technology();
        match RuleDeck::from_technology(&self.document, technology) {
            Ok(rules) => {
                self.rules = rules;
            }
            Err(err) => {
                self.rules = RuleDeck::demo(&self.document);
                self.status = format!("technology rule load failed: {err}");
            }
        }
    }

    fn reset_active_layer(&mut self) {
        self.active_layer = self
            .document
            .layer_by_process(ProcessLayer::Metal1)
            .or_else(|| self.document.layers.keys().next().copied())
            .unwrap_or(LayerId(1));
    }

    fn apply_current_technology_to_document(&mut self) {
        let technology = self.current_technology().clone();
        if let Err(err) = self.document.apply_technology(&technology) {
            self.status = format!("technology load failed: {err}");
        }
        self.reset_active_layer();
        self.rebuild_rules_from_technology();
    }

    fn switch_technology(&mut self, index: usize) {
        if index >= self.technologies.len() || index == self.active_technology {
            return;
        }
        self.active_technology = index;
        self.apply_current_technology_to_document();
        self.reset_loro_log_from_document();
        self.reset_render_cache();
        self.rebuild_indexes();
        self.rerun_drc();
        self.reset_3d_camera_to_document();
        self.status = format!("technology: {}", self.current_technology().name);
    }

    fn reset_render_cache(&mut self) {
        self.tile_cache.clear();
        self.perf.tile_dirty_count = 0;
    }

    fn clear_edit_drafts(&mut self) {
        self.cell_name_drafts.clear();
        self.instance_name_drafts.clear();
    }

    fn reset_loro_log_from_document(&mut self) {
        let mut log = match LoroCrdtLog::new(self.user_id) {
            Ok(log) => log,
            Err(err) => {
                self.status = format!("failed to create Loro log: {err}");
                return;
            }
        };
        if document_object_count(&self.document) <= MAX_LORO_SEED_OBJECTS {
            if let Err(err) = log.seed_document_objects(&self.document) {
                self.status = format!("failed to seed Loro object store: {err}");
                return;
            }
        }
        self.loro_log = log;
    }

    fn render_invalidation_for_operation(&self, operation: &Operation) -> RenderInvalidation {
        let mut invalidation = RenderInvalidation::default();
        match operation {
            Operation::Batch { .. }
            | Operation::AddCell { .. }
            | Operation::DeleteCell { .. }
            | Operation::RenameCell { .. }
            | Operation::AddInstance { .. }
            | Operation::ReplaceInstance { .. }
            | Operation::DeleteInstance { .. }
            | Operation::RenameInstance { .. }
            | Operation::MoveInstance { .. } => {
                invalidation.clear_all = true;
            }
            Operation::AddShape { shape } => {
                invalidation.rects.push(shape.kind.bounds());
                invalidation.shape_ids.insert(shape.id);
            }
            Operation::DeleteShape { id } => {
                if let Some(shape) = self.document.shapes.get(id) {
                    invalidation.rects.push(shape.kind.bounds());
                }
                invalidation.shape_ids.insert(*id);
            }
            Operation::ReplaceShape { id, shape } => {
                if let Some(old_shape) = self.document.shapes.get(id) {
                    invalidation.rects.push(old_shape.kind.bounds());
                }
                invalidation.rects.push(shape.kind.bounds());
                invalidation.shape_ids.insert(*id);
                invalidation.shape_ids.insert(shape.id);
            }
            Operation::MoveShape { id, delta } => {
                if let Some(shape) = self.document.shapes.get(id) {
                    let old_bounds = shape.kind.bounds();
                    invalidation.rects.push(old_bounds);
                    invalidation.rects.push(old_bounds.translated(*delta));
                }
                invalidation.shape_ids.insert(*id);
            }
            Operation::SetLayerVisibility { layer, .. } => {
                if self.document.has_hierarchy_instances() {
                    invalidation.clear_all = true;
                } else {
                    invalidation
                        .rects
                        .extend(self.document.shapes.values().filter_map(|shape| {
                            (shape.layer == *layer).then_some(shape.kind.bounds())
                        }));
                }
            }
            Operation::AddLayer { .. } | Operation::Cursor { .. } => {}
        }
        invalidation
    }

    fn invalidate_render_cache(&mut self, invalidation: RenderInvalidation) {
        if invalidation.clear_all {
            self.reset_render_cache();
            return;
        }
        let mut dirty_tiles = 0;
        for id in invalidation.shape_ids {
            self.tile_cache.invalidate_shape(id);
        }
        for rect in invalidation.rects {
            dirty_tiles += self.tile_cache.invalidate_rect(rect);
        }
        self.perf.tile_dirty_count = dirty_tiles;
    }

    fn apply_operation_without_history(&mut self, operation: &Operation) {
        let invalidation = self.render_invalidation_for_operation(operation);
        self.document.apply_operation_without_log(operation);
        self.rebuild_indexes();
        self.invalidate_render_cache(invalidation);
    }

    fn wrap_crdt_operation(&self, operation: Operation) -> CrdtOperation {
        CrdtOperation {
            id: self.document.next_crdt_operation_id(self.user_id),
            deps: self.document.crdt_dependency_frontier(),
            operation,
        }
    }

    fn apply_crdt_operation_without_history(&mut self, operation: CrdtOperation) -> bool {
        if self.document.crdt_has_seen(operation.id) {
            return false;
        }
        let invalidation = self.render_invalidation_for_operation(&operation.operation);
        if self.document.apply_crdt_operation(operation) != CrdtApplyResult::Applied {
            return false;
        }
        self.rebuild_indexes();
        self.invalidate_render_cache(invalidation);
        true
    }

    fn apply_and_broadcast_operation(&mut self, operation: Operation) -> bool {
        let operation = self.wrap_crdt_operation(operation);
        let update = match self
            .loro_log
            .append_operation(self.user_id, operation.clone())
        {
            Ok(update) => update,
            Err(err) => {
                self.status = format!("failed to append Loro operation: {err}");
                return false;
            }
        };
        if !self.apply_crdt_operation_without_history(operation) {
            return false;
        }
        self.broadcast_loro_update(update);
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn connect_collaboration(&mut self) {
        if self.collab.is_some() {
            return;
        }
        let url = std::env::var("FABRICAD_SYNC_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:4141/ws".to_string());
        let user = self.user_id;
        let (outbound, mut outbound_rx) = tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
        let (inbound_tx, inbound) = std::sync::mpsc::channel::<ServerMessage>();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("collaboration runtime");
            runtime.block_on(async move {
				let Ok((socket, _)) = tokio_tungstenite::connect_async(&url).await else {
					let _ = inbound_tx.send(ServerMessage::Error {
						message: format!("failed to connect to {url}"),
					});
					return;
				};
				let (mut write, mut read) = socket.split();
				if send_client_message(&mut write, &ClientMessage::Join { user }).await.is_err() {
					return;
				}
				loop {
					tokio::select! {
						Some(message) = outbound_rx.recv() => {
							if send_client_message(&mut write, &message).await.is_err() {
								let _ = inbound_tx.send(ServerMessage::Error {
									message: "collaboration socket closed while sending".to_string(),
								});
								break;
							}
						}
						incoming = read.next() => {
							let Some(incoming) = incoming else {
								let _ = inbound_tx.send(ServerMessage::Error {
									message: "collaboration socket closed".to_string(),
								});
								break;
							};
							match incoming {
								Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
									match serde_json::from_str::<ServerMessage>(&text) {
										Ok(message) => {
											let _ = inbound_tx.send(message);
										}
										Err(err) => {
											let _ = inbound_tx.send(ServerMessage::Error {
												message: format!("invalid collaboration message: {err}"),
											});
										}
									}
								}
								Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
								Ok(_) => {}
								Err(err) => {
									let _ = inbound_tx.send(ServerMessage::Error {
										message: format!("collaboration socket error: {err}"),
									});
									break;
								}
							}
						}
					}
				}
			});
        });
        self.collab = Some(CollabClient { outbound, inbound });
        self.last_broadcast_selection.clear();
        self.broadcast_selection_if_changed();
        self.status = format!("connecting collaboration as {}", short_user(user));
    }

    #[cfg(target_arch = "wasm32")]
    fn connect_collaboration(&mut self) {
        if self.collab.is_some() {
            return;
        }
        let url = wasm_collaboration_url();
        let user = self.user_id;
        let initial_selection = self.current_selection_occurrences();
        let inbound = Rc::new(RefCell::new(Vec::<ServerMessage>::new()));
        let outbound = Rc::new(RefCell::new(Vec::<ClientMessage>::new()));
        let socket = match web_sys::WebSocket::new(&url) {
            Ok(socket) => socket,
            Err(_) => {
                self.status = format!("failed to create collaboration socket for {url}");
                return;
            }
        };

        let socket_for_open = socket.clone();
        let outbound_for_open = Rc::clone(&outbound);
        let open_selection = initial_selection.clone();
        let on_open = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            send_client_message_wasm(&socket_for_open, &ClientMessage::Join { user });
            if !open_selection.is_empty() {
                send_client_message_wasm(
                    &socket_for_open,
                    &ClientMessage::Selection {
                        user,
                        selection: open_selection.clone(),
                    },
                );
            }
            let queued = outbound_for_open.borrow_mut().drain(..).collect::<Vec<_>>();
            for message in queued {
                send_client_message_wasm(&socket_for_open, &message);
            }
        }) as Box<dyn FnMut(_)>);
        socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let inbound_for_message = Rc::clone(&inbound);
        let on_message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let Some(text) = event.data().as_string() else {
                inbound_for_message.borrow_mut().push(ServerMessage::Error {
                    message: "collaboration server sent a non-text message".to_string(),
                });
                return;
            };
            match serde_json::from_str::<ServerMessage>(&text) {
                Ok(message) => inbound_for_message.borrow_mut().push(message),
                Err(err) => inbound_for_message.borrow_mut().push(ServerMessage::Error {
                    message: format!("invalid collaboration message: {err}"),
                }),
            }
        }) as Box<dyn FnMut(_)>);
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let inbound_for_error = Rc::clone(&inbound);
        let on_error = Closure::wrap(Box::new(move |_event: web_sys::ErrorEvent| {
            inbound_for_error.borrow_mut().push(ServerMessage::Error {
                message: "collaboration socket error".to_string(),
            });
        }) as Box<dyn FnMut(_)>);
        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let inbound_for_close = Rc::clone(&inbound);
        let on_close = Closure::wrap(Box::new(move |event: web_sys::CloseEvent| {
            inbound_for_close.borrow_mut().push(ServerMessage::Error {
                message: format!("collaboration socket closed ({})", event.code()),
            });
        }) as Box<dyn FnMut(_)>);
        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        self.collab = Some(CollabClient {
            socket,
            inbound,
            outbound,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
            _on_close: on_close,
        });
        self.last_broadcast_selection.clear();
        self.status = format!("connecting collaboration as {}", short_user(user));
    }

    fn poll_collaboration(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_native_collaboration();
        #[cfg(target_arch = "wasm32")]
        self.poll_wasm_collaboration();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_native_collaboration(&mut self) {
        let mut messages = Vec::new();
        if let Some(collab) = &self.collab {
            while let Ok(message) = collab.inbound.try_recv() {
                messages.push(message);
            }
        }
        for message in messages {
            self.handle_server_message(message);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_wasm_collaboration(&mut self) {
        let messages = self
            .collab
            .as_ref()
            .map(|collab| collab.inbound.borrow_mut().drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for message in messages {
            self.handle_server_message(message);
        }
    }

    fn handle_server_message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Snapshot {
                mut document,
                cursors,
                selections,
                loro_snapshot,
            } => {
                let snapshot_status = match LoroCrdtLog::from_snapshot(self.user_id, &loro_snapshot)
                {
                    Ok(log) => {
                        let materialize_status = if loro_snapshot.is_empty() {
                            None
                        } else {
                            log.materialize_objects_into_document(&mut document)
                                .err()
                                .map(|err| format!("; materialize failed: {err}"))
                        };
                        self.loro_log = log;
                        format!(
                            "collaboration snapshot applied{}",
                            materialize_status.unwrap_or_default()
                        )
                    }
                    Err(err) => {
                        format!("failed to apply Loro snapshot: {err}")
                    }
                };
                self.document = document;
                self.rebuild_rules_from_technology();
                self.remote_cursors = cursors
                    .into_iter()
                    .filter(|(user, _)| *user != self.user_id)
                    .collect();
                self.remote_selections = selections
                    .into_iter()
                    .filter(|(user, _)| *user != self.user_id)
                    .collect();
                self.undo.clear();
                self.redo.clear();
                self.clear_edit_drafts();
                self.reset_render_cache();
                self.rebuild_indexes();
                self.rerun_drc();
                self.status = snapshot_status;
            }
            ServerMessage::Operation { operation } => {
                if operation.user != self.user_id {
                    self.apply_operation_without_history(&operation.operation);
                    self.rerun_drc();
                    self.status = format!("applied remote operation #{}", operation.sequence);
                }
            }
            ServerMessage::CrdtOperation { operation } => {
                if operation.id.actor != self.user_id
                    && self.apply_crdt_operation_without_history(operation.clone())
                {
                    self.rerun_drc();
                    self.status = format!(
                        "applied remote CRDT operation {}:{}",
                        short_user(operation.id.actor),
                        operation.id.counter
                    );
                }
            }
            ServerMessage::LoroUpdate { update } => {
                let operations = match self.loro_log.import_update(&update) {
                    Ok(operations) => operations,
                    Err(err) => {
                        self.status = format!("failed to import Loro update: {err}");
                        return;
                    }
                };
                let mut applied = 0;
                for operation in operations {
                    if operation.id.actor != self.user_id
                        && self.apply_crdt_operation_without_history(operation)
                    {
                        applied += 1;
                    }
                }
                if applied > 0 {
                    self.rerun_drc();
                    self.status = format!("applied {applied} Loro operation(s)");
                }
            }
            ServerMessage::Cursor { user, position } => {
                if user != self.user_id {
                    self.remote_cursors.insert(user, position);
                }
            }
            ServerMessage::Selection { user, selection } => {
                if user != self.user_id {
                    if selection.is_empty() {
                        self.remote_selections.remove(&user);
                    } else {
                        self.remote_selections.insert(user, selection);
                    }
                }
            }
            ServerMessage::UserJoined { user } => {
                if user != self.user_id {
                    self.status = format!("user {} joined", short_user(user));
                }
            }
            ServerMessage::UserLeft { user } => {
                self.remote_cursors.remove(&user);
                self.remote_selections.remove(&user);
            }
            ServerMessage::Error { message } => {
                if is_collaboration_disconnect(&message) {
                    self.collab = None;
                    self.last_broadcast_selection.clear();
                }
                self.status = message;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn broadcast_loro_update(&self, update: LoroUpdate) {
        self.send_collaboration_message(ClientMessage::LoroUpdate { update });
    }

    #[cfg(target_arch = "wasm32")]
    fn broadcast_loro_update(&self, update: LoroUpdate) {
        self.send_collaboration_message(ClientMessage::LoroUpdate { update });
    }

    fn broadcast_cursor(&mut self, position: Point) {
        if self.last_cursor_position == Some(position) {
            return;
        }
        self.last_cursor_position = Some(position);
        self.send_collaboration_message(ClientMessage::Cursor {
            user: self.user_id,
            position,
        });
    }

    fn broadcast_selection_if_changed(&mut self) {
        if self.collab.is_none() {
            return;
        }
        let selection = self.current_selection_occurrences();
        if selection == self.last_broadcast_selection {
            return;
        }
        self.last_broadcast_selection = selection.clone();
        self.send_collaboration_message(ClientMessage::Selection {
            user: self.user_id,
            selection,
        });
    }

    fn current_selection_occurrences(&self) -> Vec<ShapeOccurrenceId> {
        if let Some(occurrence) = &self.selected_occurrence {
            return vec![occurrence.clone()];
        }
        self.selected
            .iter()
            .copied()
            .map(ShapeOccurrenceId::top_level)
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn send_collaboration_message(&self, message: ClientMessage) {
        if let Some(collab) = &self.collab {
            let _ = collab.outbound.send(message);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn send_collaboration_message(&self, message: ClientMessage) {
        if let Some(collab) = &self.collab {
            if collab.socket.ready_state() == web_sys::WebSocket::OPEN {
                send_client_message_wasm(&collab.socket, &message);
            } else {
                collab.outbound.borrow_mut().push(message);
            }
        }
    }

    fn apply_with_history(&mut self, redo: Operation, undo: Operation) {
        let dirty_region = self.render_invalidation_for_operation(&redo).dirty_bounds();
        if !self.apply_and_broadcast_operation(redo.clone()) {
            return;
        }
        self.undo.push(UndoEntry { undo, redo });
        self.redo.clear();
        if self.last_drc_run.elapsed() > Duration::from_millis(120) {
            self.rerun_drc_for_dirty_region(dirty_region);
        }
    }

    fn undo(&mut self) {
        let Some(entry) = self.undo.pop() else {
            return;
        };
        let undo = entry.undo.clone();
        if !self.apply_and_broadcast_operation(undo) {
            self.undo.push(entry);
            return;
        }
        self.drag_shape = None;
        self.drag_instance = None;
        self.drag_vertex = None;
        self.drag_edge = None;
        self.redo.push(entry);
        self.rerun_drc();
    }

    fn redo(&mut self) {
        let Some(entry) = self.redo.pop() else {
            return;
        };
        let redo = entry.redo.clone();
        if !self.apply_and_broadcast_operation(redo) {
            self.redo.push(entry);
            return;
        }
        self.drag_shape = None;
        self.drag_instance = None;
        self.drag_vertex = None;
        self.drag_edge = None;
        self.undo.push(entry);
        self.rerun_drc();
    }

    fn add_shape(&mut self, layer: LayerId, kind: ShapeKind) -> ShapeId {
        self.add_shape_with_metadata(layer, kind, None, None)
    }

    fn add_shape_with_metadata(
        &mut self,
        layer: LayerId,
        kind: ShapeKind,
        net: Option<NetId>,
        name: Option<String>,
    ) -> ShapeId {
        let id = self.document.allocate_shape_id();
        let shape = Shape {
            id,
            layer,
            net,
            kind,
            name,
        };
        let redo = Operation::AddShape {
            shape: shape.clone(),
        };
        let undo = Operation::DeleteShape { id };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        self.selected.insert(id);
        self.selected_occurrence = Some(ShapeOccurrenceId::top_level(id));
        id
    }

    fn move_shape(&mut self, id: ShapeId, delta: Vector) {
        if delta == Vector::ZERO {
            return;
        }
        let redo = Operation::MoveShape { id, delta };
        let undo = Operation::MoveShape {
            id,
            delta: Vector::new(-delta.dx, -delta.dy),
        };
        self.apply_with_history(redo, undo);
    }

    fn move_vertex(&mut self, id: ShapeId, vertex: usize, position: Point) {
        let Some(old_shape) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let Some(new_shape) = shape_with_moved_vertex(&old_shape, vertex, position) else {
            return;
        };
        if old_shape.kind.bounds() == new_shape.kind.bounds()
            && editable_vertex_points(&old_shape.kind) == editable_vertex_points(&new_shape.kind)
        {
            return;
        }
        let redo = Operation::ReplaceShape {
            id,
            shape: new_shape,
        };
        let undo = Operation::ReplaceShape {
            id,
            shape: old_shape,
        };
        self.apply_with_history(redo, undo);
    }

    fn insert_vertex(&mut self, id: ShapeId, edge: usize, position: Point) {
        let Some(old_shape) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let Some(new_shape) = shape_with_inserted_vertex(&old_shape, edge, position) else {
            self.status = "vertex insertion needs a polygon or path edge".to_string();
            return;
        };
        self.replace_shape(id, old_shape, new_shape, "inserted vertex");
    }

    fn delete_vertex(&mut self, id: ShapeId, vertex: usize) {
        let Some(old_shape) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let Some(new_shape) = shape_with_deleted_vertex(&old_shape, vertex) else {
            self.status = "cannot delete that vertex".to_string();
            return;
        };
        self.replace_shape(id, old_shape, new_shape, "deleted vertex");
    }

    fn move_edge(&mut self, id: ShapeId, edge: usize, delta: Vector) {
        if delta == Vector::ZERO {
            return;
        }
        let Some(old_shape) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let Some(new_shape) = shape_with_moved_edge(&old_shape, edge, delta) else {
            return;
        };
        self.replace_shape(id, old_shape, new_shape, "moved edge");
    }

    fn replace_shape(&mut self, id: ShapeId, old_shape: Shape, new_shape: Shape, label: &str) {
        if old_shape == new_shape {
            return;
        }
        let redo = Operation::ReplaceShape {
            id,
            shape: new_shape,
        };
        let undo = Operation::ReplaceShape {
            id,
            shape: old_shape,
        };
        self.apply_with_history(redo, undo);
        self.status = format!("{label} on shape #{}", id.0);
    }

    fn move_instance(&mut self, parent: CellId, id: InstanceId, delta: Vector) {
        if delta == Vector::ZERO {
            return;
        }
        let redo = Operation::MoveInstance { parent, id, delta };
        let undo = Operation::MoveInstance {
            parent,
            id,
            delta: Vector::new(-delta.dx, -delta.dy),
        };
        self.apply_with_history(redo, undo);
    }

    fn replace_instance(
        &mut self,
        parent: CellId,
        id: InstanceId,
        new_instance: CellInstance,
        label: &str,
    ) {
        let Some(old_instance) = self.document.instance(parent, id).cloned() else {
            return;
        };
        if old_instance == new_instance {
            return;
        }
        let redo = Operation::ReplaceInstance {
            parent,
            id,
            instance: new_instance,
        };
        let undo = Operation::ReplaceInstance {
            parent,
            id,
            instance: old_instance,
        };
        self.apply_with_history(redo, undo);
        self.status = label.to_string();
    }

    fn transform_selected_instance(&mut self, transform: Transform, label: &str) -> bool {
        let Some(info) = self.selected_instance_info() else {
            return false;
        };
        let Some(mut instance) = self.document.instance(info.parent, info.id).cloned() else {
            return false;
        };
        instance.transform = instance.transform.compose(transform);
        self.replace_instance(info.parent, info.id, instance, label);
        true
    }

    fn copy_selection(&mut self) {
        let shapes = self.selected_top_level_shapes();
        if shapes.is_empty() {
            self.status = "select top-level shapes to copy".to_string();
            return;
        }
        self.clipboard_shapes = shapes;
        self.status = format!(
            "copied {} shape{}",
            self.clipboard_shapes.len(),
            if self.clipboard_shapes.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }

    fn paste_clipboard(&mut self) {
        if self.clipboard_shapes.is_empty() {
            self.status = "clipboard is empty".to_string();
            return;
        }
        let Some(bounds) = shape_bounds(self.clipboard_shapes.iter()) else {
            return;
        };
        let target = self.snap_point(self.last_view_center);
        let center = bounds.center();
        let delta = Vector::new(target.x - center.x, target.y - center.y);
        self.add_copied_shapes(delta, "pasted");
    }

    fn duplicate_selection(&mut self) {
        let shapes = self.selected_top_level_shapes();
        if shapes.is_empty() {
            self.status = "select top-level shapes to duplicate".to_string();
            return;
        }
        self.clipboard_shapes = shapes;
        let offset = (self.snap_grid().max(10) * 8).max(80);
        self.add_copied_shapes(Vector::new(offset, -offset), "duplicated");
    }

    fn add_copied_shapes(&mut self, delta: Vector, label: &str) {
        let mut added = Vec::with_capacity(self.clipboard_shapes.len());
        for source in self.clipboard_shapes.clone() {
            let mut shape = source;
            shape.id = self.document.allocate_shape_id();
            shape.kind.translate(delta);
            added.push(shape);
        }
        if added.is_empty() {
            return;
        }
        let redo = Operation::Batch {
            operations: added
                .iter()
                .cloned()
                .map(|shape| Operation::AddShape { shape })
                .collect(),
        };
        let undo = Operation::Batch {
            operations: added
                .iter()
                .rev()
                .map(|shape| Operation::DeleteShape { id: shape.id })
                .collect(),
        };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        for shape in &added {
            self.selected.insert(shape.id);
        }
        self.selected_occurrence = added
            .first()
            .map(|shape| ShapeOccurrenceId::top_level(shape.id));
        self.status = format!(
            "{label} {} shape{}",
            added.len(),
            if added.len() == 1 { "" } else { "s" }
        );
    }

    fn delete_selection(&mut self) {
        if let Some(info) = self.selected_instance_info() {
            let Some(instance) = self.document.instance(info.parent, info.id).cloned() else {
                return;
            };
            let redo = Operation::DeleteInstance {
                parent: info.parent,
                id: info.id,
            };
            let undo = Operation::AddInstance {
                parent: info.parent,
                instance,
            };
            self.apply_with_history(redo, undo);
            self.selected.clear();
            self.selected_occurrence = None;
            self.status = "deleted instance".to_string();
            return;
        }

        let shapes = self.selected_top_level_shapes();
        if shapes.is_empty() {
            return;
        }
        let redo = Operation::Batch {
            operations: shapes
                .iter()
                .map(|shape| Operation::DeleteShape { id: shape.id })
                .collect(),
        };
        let undo = Operation::Batch {
            operations: shapes
                .iter()
                .rev()
                .cloned()
                .map(|shape| Operation::AddShape { shape })
                .collect(),
        };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        self.selected_occurrence = None;
        self.status = format!(
            "deleted {} shape{}",
            shapes.len(),
            if shapes.len() == 1 { "" } else { "s" }
        );
    }

    fn rotate_selected_90(&mut self) {
        if !self.transform_selected_instance(Transform::rotate_cw90(), "rotated instance") {
            self.transform_selected_shapes(ShapeTransform::RotateCw90, "rotated");
        }
    }

    fn mirror_selected_x(&mut self) {
        if !self.transform_selected_instance(Transform::mirror_x(), "mirrored instance X") {
            self.transform_selected_shapes(ShapeTransform::MirrorX, "mirrored X");
        }
    }

    fn mirror_selected_y(&mut self) {
        if !self.transform_selected_instance(Transform::mirror_y(), "mirrored instance Y") {
            self.transform_selected_shapes(ShapeTransform::MirrorY, "mirrored Y");
        }
    }

    fn transform_selected_shapes(&mut self, transform: ShapeTransform, label: &str) {
        let shapes = self.selected_top_level_shapes();
        if shapes.is_empty() {
            self.status = "select top-level shapes to transform".to_string();
            return;
        }
        let Some(bounds) = shape_bounds(shapes.iter()) else {
            return;
        };
        let center = bounds.center();
        let replacements: Vec<_> = shapes
            .into_iter()
            .filter_map(|old_shape| {
                let mut new_shape = old_shape.clone();
                new_shape.kind = transform_shape_kind(&old_shape.kind, center, transform);
                (old_shape != new_shape).then_some((old_shape, new_shape))
            })
            .collect();
        if replacements.is_empty() {
            return;
        }
        let redo = Operation::Batch {
            operations: replacements
                .iter()
                .map(|(_, new_shape)| Operation::ReplaceShape {
                    id: new_shape.id,
                    shape: new_shape.clone(),
                })
                .collect(),
        };
        let undo = Operation::Batch {
            operations: replacements
                .iter()
                .rev()
                .map(|(old_shape, _)| Operation::ReplaceShape {
                    id: old_shape.id,
                    shape: old_shape.clone(),
                })
                .collect(),
        };
        let count = replacements.len();
        self.apply_with_history(redo, undo);
        self.status = format!(
            "{label} {} shape{}",
            count,
            if count == 1 { "" } else { "s" }
        );
    }

    fn rename_cell(&mut self, id: CellId, name: String) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let Some(old_name) = self.document.cell(id).map(|cell| cell.name.clone()) else {
            return;
        };
        if old_name == name {
            return;
        }
        let redo = Operation::RenameCell {
            id,
            name: name.clone(),
        };
        let undo = Operation::RenameCell { id, name: old_name };
        self.apply_with_history(redo, undo);
        self.status = format!("renamed cell to {name}");
    }

    fn rename_instance(&mut self, parent: CellId, id: InstanceId, name: Option<String>) {
        let name = name.and_then(|name| {
            let trimmed = name.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        });
        let Some(old_name) = self
            .document
            .instance(parent, id)
            .map(|instance| instance.name.clone())
        else {
            return;
        };
        if old_name == name {
            return;
        }
        let redo = Operation::RenameInstance {
            parent,
            id,
            name: name.clone(),
        };
        let undo = Operation::RenameInstance {
            parent,
            id,
            name: old_name,
        };
        self.apply_with_history(redo, undo);
        self.status = match name {
            Some(name) => format!("renamed instance to {name}"),
            None => "cleared instance name".to_string(),
        };
    }

    fn create_cell_from_selection(&mut self) {
        let mut shapes: Vec<Shape> = self
            .selected
            .iter()
            .filter_map(|id| self.document.shapes.get(id).cloned())
            .collect();
        if shapes.is_empty() {
            self.status = "select top-level shapes before creating a cell".to_string();
            return;
        }
        shapes.sort_by_key(|shape| shape.id);
        let Some(bounds) = shapes
            .iter()
            .map(|shape| shape.kind.bounds())
            .reduce(|bounds, rect| bounds.union(rect))
        else {
            return;
        };
        let origin = self.snap_point(bounds.min);
        let origin_delta = Vector::new(-origin.x, -origin.y);
        let cell_id = self.document.allocate_cell_id();
        let instance_id = self.document.allocate_instance_id();
        let mut cell = Cell::new(cell_id, format!("cell {}", cell_id.0));
        for mut shape in shapes.iter().cloned() {
            shape.kind.translate(origin_delta);
            cell.shapes.insert(shape.id, shape);
        }
        let instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", cell.name)),
            cell: cell_id,
            transform: Transform::translate(origin.x, origin.y),
            array: InstanceArray::single(),
        };
        let delete_shapes: Vec<_> = shapes
            .iter()
            .map(|shape| Operation::DeleteShape { id: shape.id })
            .collect();
        let redo = Operation::Batch {
            operations: delete_shapes
                .into_iter()
                .chain([
                    Operation::AddCell { cell: cell.clone() },
                    Operation::AddInstance {
                        parent: self.document.top_cell,
                        instance: instance.clone(),
                    },
                ])
                .collect(),
        };
        let undo = Operation::Batch {
            operations: vec![
                Operation::DeleteInstance {
                    parent: self.document.top_cell,
                    id: instance_id,
                },
                Operation::DeleteCell { id: cell_id },
            ]
            .into_iter()
            .chain(
                shapes
                    .iter()
                    .cloned()
                    .map(|shape| Operation::AddShape { shape }),
            )
            .collect(),
        };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        if let Some(first_shape) = cell.shapes.keys().next().copied() {
            let occurrence = ShapeOccurrenceId::from_instance_path(first_shape, &[instance_id]);
            self.selected.insert(first_shape);
            self.selected_occurrence = Some(occurrence);
        } else {
            self.selected_occurrence = None;
        }
        self.status = format!(
            "created {} from {} shape{}",
            cell.name,
            cell.shapes.len(),
            if cell.shapes.len() == 1 { "" } else { "s" }
        );
    }

    fn place_cell_instance(&mut self, cell_id: CellId) {
        if cell_id == self.document.top_cell {
            self.status = "cannot place the top cell inside itself".to_string();
            return;
        }
        let Some(cell) = self.document.cell(cell_id).cloned() else {
            self.status = "cell no longer exists".to_string();
            return;
        };
        let instance_id = self.document.allocate_instance_id();
        let location = self.snap_point(self.last_view_center);
        let instance = CellInstance {
            id: instance_id,
            name: Some(format!("{} inst", cell.name)),
            cell: cell_id,
            transform: Transform::translate(location.x, location.y),
            array: InstanceArray::single(),
        };
        let redo = Operation::AddInstance {
            parent: self.document.top_cell,
            instance: instance.clone(),
        };
        let undo = Operation::DeleteInstance {
            parent: self.document.top_cell,
            id: instance_id,
        };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        if let Some(first_shape) = cell.shapes.keys().next().copied() {
            let occurrence = ShapeOccurrenceId::from_instance_path(first_shape, &[instance_id]);
            self.selected.insert(first_shape);
            self.selected_occurrence = Some(occurrence);
        } else {
            self.selected_occurrence = None;
        }
        self.status = format!("placed {} at {}, {}", cell.name, location.x, location.y);
    }

    fn current_workspace_dataset(&self) -> WorkspaceDataset {
        WorkspaceDataset {
            schema_version: 1,
            document: self.document.clone(),
            mes: self.mes.clone(),
            yield_analysis: self.yield_analysis.clone(),
            wafer_map: self.wafer_map.clone(),
            recipe_catalog: self.recipe_panel.catalog().clone(),
            genealogy: self.genealogy_panel.genealogy().clone(),
            experiment_plan: self.experiment_panel.plan().clone(),
            process_control: self.process_control_panel.model().clone(),
            equipment: self.equipment_sim.clone(),
        }
    }

    fn apply_workspace_dataset(
        &mut self,
        dataset: WorkspaceDataset,
        layout_source: DataSource,
        fabos_source: DataSource,
        label: &str,
    ) {
        self.replace_document(dataset.document, label);
        self.layout_source = layout_source;
        self.fabos_source = fabos_source;
        self.yield_analysis = dataset.yield_analysis;
        self.reset_yield_selection();
        self.wafer_map = dataset.wafer_map;
        self.selected_die = self.wafer_map.dies.first().copied();
        self.mes = dataset.mes;
        self.selected_mes_lot = self.mes.lots.keys().next().cloned();
        self.mask_panel =
            MaskPrepPanel::new(&self.document, &self.mes, self.selected_mes_lot.as_ref());
        self.spc_fdc_panel = SpcFdcPanel::new();
        self.process_control_panel = ProcessControlPanel::from_model(dataset.process_control);
        self.genealogy_panel = GenealogyPanel::from_genealogy(dataset.genealogy);
        self.experiment_panel = ExperimentPlannerPanel::from_plan(dataset.experiment_plan);
        self.recipe_panel = RecipeManagerPanel::from_catalog(dataset.recipe_catalog);
        self.equipment_sim = dataset.equipment;
        self.selected_equipment_tool = self
            .equipment_sim
            .tools()
            .next()
            .map(|tool| tool.id.clone());
        self.equipment_recipe_drafts.clear();
        self.last_equipment_tick = Instant::now();
        self.status = label.to_string();
    }

    fn reset_yield_selection(&mut self) {
        self.selected_yield_lot = self
            .yield_analysis
            .lots
            .first()
            .map(|lot| lot.id.clone())
            .unwrap_or_default();
        self.selected_yield_wafer = self
            .yield_analysis
            .wafer_ids_for_lot(&self.selected_yield_lot)
            .first()
            .cloned()
            .unwrap_or_default();
    }

    fn new_blank_workspace(&mut self) {
        self.apply_workspace_dataset(
            WorkspaceDataset::blank(),
            DataSource::Blank,
            DataSource::Blank,
            "new blank workspace",
        );
    }

    fn save_workspace(&mut self) {
        let path = PathBuf::from(WORKSPACE_PATH);
        if let Some(parent) = path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            self.status = format!("workspace save failed: {err}");
            return;
        }
        match serde_json::to_string_pretty(&self.current_workspace_dataset())
            .and_then(|contents| fs::write(&path, contents).map_err(serde_json::Error::io))
        {
            Ok(()) => self.status = format!("saved workspace {WORKSPACE_PATH}"),
            Err(err) => self.status = format!("workspace save failed: {err}"),
        }
    }

    fn load_workspace(&mut self) {
        match fs::read_to_string(WORKSPACE_PATH)
            .map_err(serde_json::Error::io)
            .and_then(|contents| serde_json::from_str::<WorkspaceDataset>(&contents))
        {
            Ok(dataset) => self.apply_workspace_dataset(
                dataset,
                DataSource::File(WORKSPACE_PATH.to_string()),
                DataSource::File(WORKSPACE_PATH.to_string()),
                &format!("loaded workspace {WORKSPACE_PATH}"),
            ),
            Err(err) => self.status = format!("workspace load failed: {err}"),
        }
    }

    fn load_demo_workspace(&mut self) {
        let path = PathBuf::from(DEMO_WORKSPACE_PATH);
        let dataset = match fs::read_to_string(&path)
            .map_err(serde_json::Error::io)
            .and_then(|contents| serde_json::from_str::<WorkspaceDataset>(&contents))
        {
            Ok(dataset) => dataset,
            Err(_) => {
                let dataset = WorkspaceDataset::demo();
                if let Some(parent) = path.parent()
                    && let Err(err) = fs::create_dir_all(parent)
                {
                    self.status = format!("demo workspace setup failed: {err}");
                    return;
                }
                match serde_json::to_string_pretty(&dataset)
                    .and_then(|contents| fs::write(&path, contents).map_err(serde_json::Error::io))
                {
                    Ok(()) => {}
                    Err(err) => {
                        self.status = format!("demo workspace setup failed: {err}");
                        return;
                    }
                }
                dataset
            }
        };
        self.apply_workspace_dataset(
            dataset,
            DataSource::Demo,
            DataSource::Demo,
            &format!("loaded demo workspace {DEMO_WORKSPACE_PATH}"),
        );
    }

    fn save_document(&mut self) {
        let path = PathBuf::from(SAVE_PATH);
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                self.status = format!("save failed: {err}");
                return;
            }
        }
        match serde_json::to_string_pretty(&self.document)
            .and_then(|contents| fs::write(&path, contents).map_err(serde_json::Error::io))
        {
            Ok(()) => {
                self.last_autosave = Instant::now();
                self.status = format!("saved {SAVE_PATH}");
            }
            Err(err) => self.status = format!("save failed: {err}"),
        }
    }

    fn autosave_document(&mut self) {
        let contents = match serde_json::to_string_pretty(&self.document) {
            Ok(contents) => contents,
            Err(err) => {
                self.status = format!("autosave failed: {err}");
                return;
            }
        };

        #[cfg(target_arch = "wasm32")]
        {
            let Some(window) = web_sys::window() else {
                self.status = "autosave failed: no browser window".to_string();
                return;
            };
            let storage = match window.local_storage() {
                Ok(Some(storage)) => storage,
                Ok(None) => {
                    self.status = "autosave failed: local storage unavailable".to_string();
                    return;
                }
                Err(_) => {
                    self.status = "autosave failed: local storage blocked".to_string();
                    return;
                }
            };
            match storage.set_item(WASM_AUTOSAVE_KEY, &contents) {
                Ok(()) => self.status = "autosaved to browser storage".to_string(),
                Err(_) => self.status = "autosave failed: browser storage write failed".to_string(),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = PathBuf::from(SAVE_PATH);
            if let Some(parent) = path.parent()
                && let Err(err) = fs::create_dir_all(parent)
            {
                self.status = format!("autosave failed: {err}");
                return;
            }
            match fs::write(&path, contents) {
                Ok(()) => self.status = format!("autosaved {SAVE_PATH}"),
                Err(err) => self.status = format!("autosave failed: {err}"),
            }
        }
    }

    fn restore_autosave_document(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(window) = web_sys::window() else {
                self.status = "restore failed: no browser window".to_string();
                return;
            };
            let storage = match window.local_storage() {
                Ok(Some(storage)) => storage,
                Ok(None) => {
                    self.status = "restore failed: local storage unavailable".to_string();
                    return;
                }
                Err(_) => {
                    self.status = "restore failed: local storage blocked".to_string();
                    return;
                }
            };
            let contents = match storage.get_item(WASM_AUTOSAVE_KEY) {
                Ok(Some(contents)) => contents,
                Ok(None) => {
                    self.status = "restore failed: no browser autosave".to_string();
                    return;
                }
                Err(_) => {
                    self.status = "restore failed: browser storage read failed".to_string();
                    return;
                }
            };
            match serde_json::from_str::<Document>(&contents) {
                Ok(document) => {
                    self.replace_document(document, "restored browser autosave");
                    self.layout_source = DataSource::File("browser autosave".to_string());
                }
                Err(err) => self.status = format!("restore failed: {err}"),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.load_document();
        }
    }

    fn replace_document(&mut self, document: Document, label: &str) {
        self.document = document;
        self.reset_active_layer();
        self.rebuild_rules_from_technology();
        self.undo.clear();
        self.redo.clear();
        self.selected.clear();
        self.selected_occurrence = None;
        self.clear_edit_drafts();
        self.reset_loro_log_from_document();
        self.reset_render_cache();
        self.rebuild_indexes();
        self.rerun_drc();
        self.reset_3d_camera_to_document();
        self.status = label.to_string();
    }

    fn load_document(&mut self) {
        match fs::read_to_string(SAVE_PATH)
            .map_err(serde_json::Error::io)
            .and_then(|contents| serde_json::from_str::<Document>(&contents))
        {
            Ok(document) => {
                self.replace_document(document, &format!("loaded {SAVE_PATH}"));
                self.layout_source = DataSource::File(SAVE_PATH.to_string());
            }
            Err(err) => self.status = format!("load failed: {err}"),
        }
    }

    fn export_gds_document(&mut self) {
        let technology = self.current_technology().clone();
        let bytes = match export_gdsii(&self.document, &technology) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.status = format!("GDS export failed: {err}");
                return;
            }
        };
        let path = PathBuf::from(GDS_PATH);
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                self.status = format!("GDS export failed: {err}");
                return;
            }
        }
        match fs::write(&path, bytes) {
            Ok(()) => self.status = format!("exported {GDS_PATH}"),
            Err(err) => self.status = format!("GDS export failed: {err}"),
        }
    }

    fn import_gds_document(&mut self) {
        let technology = self.current_technology().clone();
        let bytes = match fs::read(GDS_PATH) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.status = format!("GDS import failed: {err}");
                return;
            }
        };
        match import_gdsii(&bytes, &technology) {
            Ok(document) => {
                self.replace_document(document, &format!("imported {GDS_PATH}"));
                self.layout_source = DataSource::File(GDS_PATH.to_string());
            }
            Err(err) => self.status = format!("GDS import failed: {err}"),
        }
    }

    fn make_stress_document(&mut self, count: usize) {
        self.replace_document(
            Document::stress(count),
            &format!("generated {count} polygons"),
        );
        self.layout_source = DataSource::Generated(format!("{count} polygon stress"));
        self.status = format!("generated {count} polygons");
    }

    fn make_hierarchy_document(&mut self) {
        self.document = Document::hierarchy_demo();
        self.apply_current_technology_to_document();
        self.selected.clear();
        self.selected_occurrence = None;
        self.undo.clear();
        self.redo.clear();
        self.clear_edit_drafts();
        self.reset_loro_log_from_document();
        self.reset_render_cache();
        self.rebuild_indexes();
        self.rerun_drc();
        self.reset_3d_camera_to_document();
        self.layout_source = DataSource::Demo;
        self.status = "generated hierarchy demo".to_string();
    }

    fn route_between_points(&mut self) {
        if self.route_points.len() < 2 {
            return;
        }
        let start = self.route_points[0];
        let goal = self.route_points[1];
        let started = Instant::now();
        let result = route(
            &self.document,
            RouteRequest {
                start,
                goal,
                layer: self.active_layer,
                wire_width: 180,
                bounds: Some(Rect::new(start, goal).expanded(10_000)),
            },
            RouterConfig::default(),
        );
        self.perf.route_ms = started.elapsed().as_secs_f64() * 1000.0;
        match result {
            Ok(route) => {
                let point_count = route.points.len();
                let visited = route.visited_nodes;
                let route_net = self.route_net_metadata(start, goal);
                let net_label = route_net
                    .name
                    .clone()
                    .or_else(|| route_net.net.map(|net| format!("NET{}", net.0)));
                self.add_shape_with_metadata(
                    self.active_layer,
                    ShapeKind::Path {
                        points: route.points,
                        width: 180,
                    },
                    route_net.net,
                    route_net.name,
                );
                self.status = if let Some(net_label) = net_label {
                    format!(
                        "route placed on {net_label} with {point_count} points after {visited} visited nodes"
                    )
                } else {
                    format!("route placed with {point_count} points after {visited} visited nodes")
                };
            }
            Err(err) => {
                self.status = format!("route failed: {err:?}");
            }
        }
        self.route_points.clear();
    }

    fn route_net_metadata(&self, start: Point, goal: Point) -> RouteNetMetadata {
        let tolerance = self.snap_grid() * 2;
        let start_component = self.component_at_point(start, tolerance);
        let goal_component = self.component_at_point(goal, tolerance);
        match (start_component, goal_component) {
            (Some(start), Some(goal)) if start.id == goal.id => component_route_metadata(start),
            (Some(start), Some(goal)) => {
                let start_meta = component_route_metadata(start);
                let goal_meta = component_route_metadata(goal);
                if start_meta.net.is_some() && start_meta.net == goal_meta.net {
                    start_meta
                } else if start_meta.name.is_some() && start_meta.name == goal_meta.name {
                    start_meta
                } else if start_meta.net.is_none() && start_meta.name.is_none() {
                    goal_meta
                } else if goal_meta.net.is_none() && goal_meta.name.is_none() {
                    start_meta
                } else {
                    RouteNetMetadata::default()
                }
            }
            (Some(component), None) | (None, Some(component)) => {
                component_route_metadata(component)
            }
            (None, None) => RouteNetMetadata::default(),
        }
    }

    fn component_at_point(&self, point: Point, tolerance: Coord) -> Option<&NetComponent> {
        let query = Rect::new(point, point).expanded(tolerance);
        self.index
            .query_occurrences(query)
            .into_iter()
            .filter_map(|occurrence| {
                let component = self.connectivity.component_for_occurrence(&occurrence)?;
                self.connectivity.component(component)
            })
            .next()
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_keyboard = ctx.wants_keyboard_input();
        ctx.input(|input| {
            if self.settings.single_key_shortcuts {
                if input.key_pressed(Key::Num1) {
                    self.tool = Tool::Select;
                }
                if input.key_pressed(Key::Num2) {
                    self.tool = Tool::Rect;
                }
                if input.key_pressed(Key::Num3) {
                    self.tool = Tool::Polygon;
                }
                if input.key_pressed(Key::Num4) {
                    self.tool = Tool::Path;
                }
                if input.key_pressed(Key::Num5) {
                    self.tool = Tool::Measure;
                }
                if input.key_pressed(Key::Num6) {
                    self.tool = Tool::Route;
                }
            }
            if input.modifiers.command && input.key_pressed(Key::Z) {
                self.undo();
            }
            if input.modifiers.command && input.key_pressed(Key::Y) {
                self.redo();
            }
            if !wants_keyboard {
                if input.modifiers.command && input.key_pressed(Key::C) {
                    self.copy_selection();
                }
                if input.modifiers.command && input.key_pressed(Key::V) {
                    self.paste_clipboard();
                }
                if input.modifiers.command && input.key_pressed(Key::D) {
                    self.duplicate_selection();
                }
                if self.settings.single_key_shortcuts && !input.modifiers.any() {
                    if input.key_pressed(Key::R) {
                        self.rotate_selected_90();
                    }
                    if input.key_pressed(Key::H) {
                        self.mirror_selected_x();
                    }
                    if input.key_pressed(Key::V) {
                        self.mirror_selected_y();
                    }
                }
            }
        });
    }

    fn advance_equipment_simulator(&mut self) {
        let seconds = self.last_equipment_tick.elapsed().as_secs().min(4);
        if seconds == 0 {
            return;
        }
        let events = self.equipment_sim.tick(seconds);
        self.last_equipment_tick = Instant::now();
        if events
            .iter()
            .any(|event| matches!(event, EquipmentEvent::AlarmRaised { .. }))
        {
            self.status = "equipment simulator raised an alarm".to_string();
        }
    }

    fn send_equipment_command(&mut self, tool_id: EquipmentToolId, command: HostCommand) {
        let label = command.label();
        match self.equipment_sim.command(&tool_id, command) {
            Ok(events) => {
                let state = self
                    .equipment_sim
                    .tool(&tool_id)
                    .map(|tool| tool.state.label())
                    .unwrap_or("unknown");
                let major_events = events
                    .iter()
                    .filter(|event| !matches!(event, EquipmentEvent::SensorSample { .. }))
                    .count();
                self.status = format!("{tool_id} {label}: {state}, {major_events} event(s)");
            }
            Err(err) => {
                self.status = err.to_string();
            }
        }
    }

    fn selected_recipe_for_tool(&self, tool: &EquipmentTool) -> Option<EquipmentRecipeId> {
        self.equipment_recipe_drafts
            .get(&tool.id)
            .cloned()
            .or_else(|| {
                tool.selected_recipe
                    .as_ref()
                    .map(|selection| selection.recipe_id.clone())
            })
            .or_else(|| tool.available_recipes.keys().next().cloned())
    }

    fn selection_for_tool(
        &self,
        tool: &EquipmentTool,
        recipe_id: EquipmentRecipeId,
    ) -> RecipeSelection {
        let version = tool
            .available_recipes
            .get(&recipe_id)
            .map(|recipe| recipe.version)
            .unwrap_or(1);
        RecipeSelection {
            recipe_id,
            recipe_version: version,
            lot_id: Some("LOT-FABOS-0042".to_string()),
            wafer_id: Some(format!("W{:02}", (self.equipment_sim.now_s % 25) + 1)),
            process_step_id: Some(equipment_process_step_label(tool.kind).to_string()),
            operator: Some(short_user(self.user_id)),
        }
    }

    fn fab_control_room(&mut self, ui: &mut egui::Ui) {
        let tools = self.equipment_sim.tools().cloned().collect::<Vec<_>>();
        if tools.is_empty() {
            ui_chrome::module_header(
                ui,
                "Fab operations",
                "Fab Control Room",
                "No data loaded",
                |_| {},
            );
            ui_chrome::empty_state(ui, "No equipment dataset loaded");
            return;
        }
        if self
            .selected_equipment_tool
            .as_ref()
            .is_none_or(|id| !tools.iter().any(|tool| &tool.id == id))
        {
            self.selected_equipment_tool = tools.first().map(|tool| tool.id.clone());
        }

        ui.vertical(|ui| {
            ui_chrome::module_header(
                ui,
                "Fab operations",
                "Fab Control Room",
                &format!("Sim time: {} s", self.equipment_sim.now_s),
                |_| {},
            );
            ui.horizontal_wrapped(|ui| {
                ui_chrome::metric_tile(ui, "Tools", tools.len(), "");
                ui_chrome::metric_tile_tone(
                    ui,
                    "Running",
                    tools
                        .iter()
                        .filter(|tool| tool.state == EquipmentToolState::Running)
                        .count(),
                    "",
                    ui_chrome::Tone::Success,
                );
                ui_chrome::metric_tile_tone(
                    ui,
                    "Active alarms",
                    self.equipment_sim.active_alarms().len(),
                    "",
                    if self.equipment_sim.active_alarms().is_empty() {
                        ui_chrome::Tone::Neutral
                    } else {
                        ui_chrome::Tone::Danger
                    },
                );
            });
            ui.separator();

            let wide = ui.available_width() > 980.0;
            if wide {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() * 0.62).max(560.0));
                        self.equipment_tool_grid(ui, &tools);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        self.selected_equipment_panel(ui, &tools);
                    });
                });
            } else {
                self.equipment_tool_grid(ui, &tools);
                ui.separator();
                self.selected_equipment_panel(ui, &tools);
            }
        });
    }

    fn equipment_tool_grid(&mut self, ui: &mut egui::Ui, tools: &[EquipmentTool]) {
        ui_chrome::section_label(ui, "Tool Grid");
        let mut pending_command: Option<(EquipmentToolId, HostCommand)> = None;
        egui::ScrollArea::vertical()
            .id_salt("equipment_tool_grid_scroll")
            .max_height(if ui.available_height() > 520.0 {
                ui.available_height() - 16.0
            } else {
                420.0
            })
            .show(ui, |ui| {
                egui::Grid::new("equipment_tool_grid")
                    .num_columns(5)
                    .striped(true)
                    .spacing(vec2(14.0, 8.0))
                    .show(ui, |ui| {
                        ui.strong("Tool");
                        ui.strong("State");
                        ui.strong("Recipe / run");
                        ui.strong("Sensors");
                        ui.strong("Commands");
                        ui.end_row();

                        for tool in tools {
                            let selected = self.selected_equipment_tool.as_ref() == Some(&tool.id);
                            let label = format!("{}\n{}", tool.name, tool.id);
                            if ui.selectable_label(selected, label).clicked() {
                                self.selected_equipment_tool = Some(tool.id.clone());
                            }

                            ui.label(
                                egui::RichText::new(tool.state.label())
                                    .color(equipment_state_color(tool.state))
                                    .strong(),
                            );
                            ui.label(equipment_recipe_run_summary(tool, self.equipment_sim.now_s));
                            ui.label(equipment_recent_sensor_summary(tool));

                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add_enabled(
                                        tool.state == EquipmentToolState::Offline,
                                        egui::Button::new("Online"),
                                    )
                                    .clicked()
                                {
                                    pending_command =
                                        Some((tool.id.clone(), HostCommand::BringOnline));
                                }
                                if let Some(recipe_id) = self.selected_recipe_for_tool(tool)
                                    && ui
                                        .add_enabled(
                                            tool.state.accepts_recipe_load(),
                                            egui::Button::new("Load"),
                                        )
                                        .clicked()
                                {
                                    pending_command = Some((
                                        tool.id.clone(),
                                        HostCommand::LoadRecipe {
                                            selection: self.selection_for_tool(tool, recipe_id),
                                        },
                                    ));
                                }
                                if ui
                                    .add_enabled(
                                        tool.state == EquipmentToolState::RecipeLoaded,
                                        egui::Button::new("Start"),
                                    )
                                    .clicked()
                                {
                                    pending_command = Some((tool.id.clone(), HostCommand::Start));
                                }
                                if ui
                                    .add_enabled(
                                        tool.state == EquipmentToolState::Running,
                                        egui::Button::new("Stop"),
                                    )
                                    .clicked()
                                {
                                    pending_command = Some((tool.id.clone(), HostCommand::Stop));
                                }
                                if ui
                                    .add_enabled(
                                        !matches!(
                                            tool.state,
                                            EquipmentToolState::Offline
                                                | EquipmentToolState::Maintenance
                                        ),
                                        egui::Button::new("Alarm"),
                                    )
                                    .clicked()
                                {
                                    pending_command = Some((
                                        tool.id.clone(),
                                        HostCommand::TriggerAlarm {
                                            code: "HOST-SIM".to_string(),
                                            message: "operator injected simulator alarm"
                                                .to_string(),
                                            severity: AlarmSeverity::Warning,
                                        },
                                    ));
                                }
                                if ui
                                    .add_enabled(
                                        tool.state == EquipmentToolState::Alarm,
                                        egui::Button::new("Clear"),
                                    )
                                    .clicked()
                                {
                                    pending_command =
                                        Some((tool.id.clone(), HostCommand::ClearAlarm));
                                }
                            });
                            ui.end_row();
                        }
                    });
            });

        if let Some((tool_id, command)) = pending_command {
            self.send_equipment_command(tool_id, command);
        }
    }

    fn selected_equipment_panel(&mut self, ui: &mut egui::Ui, tools: &[EquipmentTool]) {
        let selected_id = self
            .selected_equipment_tool
            .clone()
            .or_else(|| tools.first().map(|tool| tool.id.clone()));
        let Some(selected_id) = selected_id else {
            ui.label("No simulator tools configured");
            return;
        };
        let Some(tool) = tools.iter().find(|tool| tool.id == selected_id).cloned() else {
            ui.label("Selected simulator tool is unavailable");
            return;
        };

        ui.heading(&tool.name);
        ui.horizontal_wrapped(|ui| {
            ui.label(tool.id.to_string());
            ui.separator();
            ui.label(tool.kind.label());
            ui.separator();
            ui.label(tool.class.label());
            ui.separator();
            ui.label(
                egui::RichText::new(tool.state.label())
                    .color(equipment_state_color(tool.state))
                    .strong(),
            );
        });

        ui.separator();
        ui.strong("Recipe");
        let mut draft = self
            .selected_recipe_for_tool(&tool)
            .unwrap_or_else(|| EquipmentRecipeId::new(""));
        let selected_recipe_text = tool
            .available_recipes
            .get(&draft)
            .map(|recipe| format!("{} v{}", recipe.name, recipe.version))
            .unwrap_or_else(|| "No recipe".to_string());
        egui::ComboBox::from_id_salt(("equipment_recipe", tool.id.as_str()))
            .selected_text(selected_recipe_text)
            .show_ui(ui, |ui| {
                for recipe in tool.available_recipes.values() {
                    ui.selectable_value(
                        &mut draft,
                        recipe.id.clone(),
                        format!("{} v{}", recipe.name, recipe.version),
                    );
                }
            });
        if !draft.as_str().is_empty() {
            self.equipment_recipe_drafts
                .insert(tool.id.clone(), draft.clone());
        }

        let mut pending_command: Option<(EquipmentToolId, HostCommand)> = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::Offline,
                    egui::Button::new("Bring Online"),
                )
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::BringOnline));
            }
            if ui
                .add_enabled(
                    tool.state.accepts_recipe_load() && !draft.as_str().is_empty(),
                    egui::Button::new("Load Recipe"),
                )
                .clicked()
            {
                pending_command = Some((
                    tool.id.clone(),
                    HostCommand::LoadRecipe {
                        selection: self.selection_for_tool(&tool, draft.clone()),
                    },
                ));
            }
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::RecipeLoaded,
                    egui::Button::new("Start"),
                )
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::Start));
            }
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::Running,
                    egui::Button::new("Stop"),
                )
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::Stop));
            }
            if ui
                .add_enabled(
                    tool.state != EquipmentToolState::Running,
                    egui::Button::new("Reset"),
                )
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::Reset));
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !matches!(
                        tool.state,
                        EquipmentToolState::Offline | EquipmentToolState::Maintenance
                    ),
                    egui::Button::new("Trigger Alarm"),
                )
                .clicked()
            {
                pending_command = Some((
                    tool.id.clone(),
                    HostCommand::TriggerAlarm {
                        code: "HOST-SIM".to_string(),
                        message: "operator injected simulator alarm".to_string(),
                        severity: AlarmSeverity::Warning,
                    },
                ));
            }
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::Alarm,
                    egui::Button::new("Clear Alarm"),
                )
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::ClearAlarm));
            }
            let maintenance_label = if tool.state == EquipmentToolState::Maintenance {
                "Exit Maintenance"
            } else {
                "Maintenance"
            };
            if ui
                .add_enabled(
                    tool.state != EquipmentToolState::Running,
                    egui::Button::new(maintenance_label),
                )
                .clicked()
            {
                pending_command = Some((
                    tool.id.clone(),
                    if tool.state == EquipmentToolState::Maintenance {
                        HostCommand::ExitMaintenance
                    } else {
                        HostCommand::EnterMaintenance
                    },
                ));
            }
        });

        if let Some((tool_id, command)) = pending_command {
            self.send_equipment_command(tool_id, command);
        }

        ui.separator();
        self.equipment_run_panel(ui, &tool);
        ui.separator();
        self.equipment_sensor_panel(ui, &tool);
        ui.separator();
        self.equipment_alarm_panel(ui);
    }

    fn equipment_run_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui.strong("Current Run");
        if let Some(run) = &tool.active_run {
            ui.label(format!(
                "{} on {} for {} s",
                run.id,
                run.recipe.recipe_id,
                run.elapsed_s(self.equipment_sim.now_s)
            ));
            if let Some(recipe) = tool.selected_recipe_details() {
                let progress = (run.elapsed_s(self.equipment_sim.now_s) as f32
                    / recipe.duration_s as f32)
                    .clamp(0.0, 1.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .desired_width(ui.available_width().min(360.0))
                        .text(format!("{:.0}%", progress * 100.0)),
                );
            }
        } else if let Some(selection) = &tool.selected_recipe {
            ui.label(format!(
                "Loaded {} v{}",
                selection.recipe_id, selection.recipe_version
            ));
        } else {
            ui.label("No active run");
        }

        ui.strong("Recent Runs");
        if tool.recent_runs.is_empty() {
            ui.label("No completed runs");
            return;
        }
        for run in tool.recent_runs.iter().take(4) {
            ui.horizontal_wrapped(|ui| {
                ui.label(run.id.to_string());
                ui.label(
                    egui::RichText::new(run.status.label())
                        .color(equipment_run_status_color(run.status)),
                );
                ui.label(format!(
                    "{} s, {} samples",
                    run.elapsed_s(self.equipment_sim.now_s),
                    run.sensor_count
                ));
            });
        }
    }

    fn equipment_sensor_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui.strong("Sensor Streams");
        let names = equipment_sensor_names(tool);
        if names.is_empty() {
            ui.label("Waiting for samples");
            return;
        }
        for name in names {
            let values = equipment_sensor_series(tool, &name, 36);
            let latest = tool.latest_sensor(&name);
            ui.horizontal(|ui| {
                ui.set_min_height(42.0);
                ui.vertical(|ui| {
                    ui.label(&name);
                    if let Some(sample) = latest {
                        ui.label(equipment_sensor_value(sample));
                    }
                });
                equipment_sparkline(ui, &values, Color32::from_rgb(96, 178, 255));
            });
        }
    }

    fn equipment_alarm_panel(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Active Alarms");
        let alarms = self.equipment_sim.active_alarms();
        if alarms.is_empty() {
            ui_chrome::empty_state(ui, "No active alarms");
        } else {
            for alarm in alarms {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(alarm.severity.label())
                            .color(equipment_alarm_color(alarm.severity))
                            .strong(),
                    );
                    ui.label(format!("{} {}", alarm.tool_id, alarm.code));
                    ui.label(&alarm.message);
                });
            }
        }

        ui.separator();
        ui_chrome::section_label(ui, "Fab Run Log");
        let recent_runs = self.equipment_sim.recent_runs();
        if recent_runs.is_empty() {
            ui_chrome::empty_state(ui, "No logged runs");
        } else {
            for run in recent_runs.into_iter().take(5) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(run.tool_id.to_string());
                    ui.label(run.id.to_string());
                    ui.label(run.recipe.recipe_id.to_string());
                    ui.label(
                        egui::RichText::new(run.status.label())
                            .color(equipment_run_status_color(run.status)),
                    );
                });
            }
        }
    }

    fn navigation_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("app_navigation")
            .resizable(false)
            .default_width(ui_chrome::NAV_WIDTH)
            .show(ctx, |ui| {
                ui.set_width(ui_chrome::NAV_WIDTH - 18.0);
                ui.add_space(4.0);
                ui.label(RichText::new("Fabricad").heading().strong());
                ui_chrome::muted(ui, "FabOS workbench");
                ui.horizontal_wrapped(|ui| {
                    ui_chrome::status_pill(
                        ui,
                        &format!("Layout {}", self.layout_source.label()),
                        self.layout_source.tone(),
                    );
                    ui_chrome::status_pill(
                        ui,
                        &format!("FabOS {}", self.fabos_source.label()),
                        self.fabos_source.tone(),
                    );
                });
                if let Some(detail) = self.layout_source.detail() {
                    ui_chrome::muted(ui, format!("Layout source: {detail}"));
                }
                if let Some(detail) = self.fabos_source.detail() {
                    ui_chrome::muted(ui, format!("FabOS source: {detail}"));
                }
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("app_navigation_scroll")
                    .show(ui, |ui| {
                        for group in ModuleGroup::ALL {
                            ui_chrome::section_label(ui, group.label());
                            for mode in ViewMode::ALL
                                .into_iter()
                                .filter(|candidate| candidate.group() == group)
                            {
                                let selected = self.view_mode == mode;
                                let label = if selected {
                                    RichText::new(mode.nav_label()).strong()
                                } else {
                                    RichText::new(mode.nav_label())
                                };
                                if ui
                                    .selectable_label(selected, label)
                                    .on_hover_text(mode.detail())
                                    .clicked()
                                {
                                    self.select_view_mode(mode);
                                }
                                if selected {
                                    ui.indent(("nav_detail", mode.nav_label()), |ui| {
                                        ui_chrome::muted(ui, mode.detail());
                                    });
                                }
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.show_mes_panel, "Traveler");
                    ui_chrome::status_pill(
                        ui,
                        if self.show_mes_panel {
                            "open"
                        } else {
                            "hidden"
                        },
                        if self.show_mes_panel {
                            ui_chrome::Tone::Success
                        } else {
                            ui_chrome::Tone::Neutral
                        },
                    );
                });
            });
    }

    fn select_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode == mode {
            return;
        }
        self.view_mode = mode;
        self.status = mode.status_message().to_string();
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui_chrome::status_pill(
                    ui,
                    self.view_mode.group().label(),
                    self.view_mode.group().tone(),
                );
                ui_chrome::status_pill(
                    ui,
                    &format!("Layout {}", self.layout_source.label()),
                    self.layout_source.tone(),
                );
                ui_chrome::status_pill(
                    ui,
                    &format!("FabOS {}", self.fabos_source.label()),
                    self.fabos_source.tone(),
                );
                ui.label(RichText::new(self.view_mode.title()).strong());
                ui.label(
                    RichText::new(self.view_mode.detail()).color(ui.visuals().weak_text_color()),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("User {}", short_user(self.user_id))).small());
                    ui.label(RichText::new(&self.status).color(ui.visuals().weak_text_color()));
                });
            });
            ui.horizontal_wrapped(|ui| {
                let layout_mode = self.view_mode.is_layout();
                if layout_mode {
                    ui.label("Tool");
                    tool_button(ui, &mut self.tool, Tool::Select, "Select");
                    tool_button(ui, &mut self.tool, Tool::Rect, "Rect");
                    tool_button(ui, &mut self.tool, Tool::Polygon, "Poly");
                    tool_button(ui, &mut self.tool, Tool::Path, "Path");
                    tool_button(ui, &mut self.tool, Tool::Via, "Via");
                    tool_button(ui, &mut self.tool, Tool::Measure, "Measure");
                    tool_button(ui, &mut self.tool, Tool::Route, "Route");
                    ui.separator();
                }

                if matches!(self.view_mode, ViewMode::Layout3d) {
                    if ui.button("Reset 3D").clicked() {
                        self.reset_3d_camera_to_document();
                    }
                    if ui.button("Up").clicked() {
                        self.nudge_3d_camera_vertical(1.0);
                    }
                    if ui.button("Down").clicked() {
                        self.nudge_3d_camera_vertical(-1.0);
                    }
                    ui.separator();
                }

                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.redo();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Copy"))
                        .clicked()
                    {
                        self.copy_selection();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Paste"))
                        .clicked()
                    {
                        self.paste_clipboard();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Duplicate"))
                        .clicked()
                    {
                        self.duplicate_selection();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Delete"))
                        .clicked()
                    {
                        self.delete_selection();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Make Cell"))
                        .clicked()
                    {
                        self.create_cell_from_selection();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Rotate 90"))
                        .clicked()
                    {
                        self.rotate_selected_90();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Mirror X"))
                        .clicked()
                    {
                        self.mirror_selected_x();
                    }
                    if ui
                        .add_enabled(layout_mode, egui::Button::new("Mirror Y"))
                        .clicked()
                    {
                        self.mirror_selected_y();
                    }
                });

                ui.menu_button("Document", |ui| {
                    if ui.button("New Blank Workspace").clicked() {
                        self.new_blank_workspace();
                    }
                    if ui.button("Load Demo Workspace").clicked() {
                        self.load_demo_workspace();
                    }
                    ui.separator();
                    if ui.button("Save Workspace").clicked() {
                        self.save_workspace();
                    }
                    if ui.button("Load Workspace").clicked() {
                        self.load_workspace();
                    }
                    ui.separator();
                    ui.label("Layout only");
                    if ui.button("Save Layout JSON").clicked() {
                        self.save_document();
                    }
                    if ui.button("Load Layout JSON").clicked() {
                        self.load_document();
                    }
                    ui.separator();
                    if ui.button("Export GDS").clicked() {
                        self.export_gds_document();
                    }
                    if ui.button("Import GDS").clicked() {
                        self.import_gds_document();
                    }
                    ui.separator();
                    if ui.button("Connect collaboration").clicked() {
                        self.connect_collaboration();
                    }
                });

                ui.menu_button("Analyze", |ui| {
                    if ui.button("Run DRC").clicked() {
                        self.rerun_drc();
                    }
                    if ui.button("Diagnostics").clicked() {
                        self.show_diagnostics = true;
                    }
                });

                ui.menu_button("Demo Data", |ui| {
                    if ui.button("Load Full Demo Workspace").clicked() {
                        self.load_demo_workspace();
                    }
                    ui.separator();
                    ui.label("Layout test scenes");
                    if ui.button("10k stress").clicked() {
                        self.make_stress_document(10_000);
                    }
                    if ui.button("100k stress").clicked() {
                        self.make_stress_document(100_000);
                    }
                    if ui.button("1M stress").clicked() {
                        self.make_stress_document(1_000_000);
                    }
                    if ui.button("Hierarchy").clicked() {
                        self.make_hierarchy_document();
                    }
                });

                if ui.button("Options").clicked() {
                    self.show_options = true;
                }
            });
        });
    }

    fn options_window(&mut self, ctx: &egui::Context) {
        if !self.show_options {
            return;
        }
        let mut open = self.show_options;
        egui::Window::new("Options")
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| self.options_ui(ui));
        self.show_options = open;
    }

    fn diagnostics_window(&mut self, ctx: &egui::Context) {
        if !self.show_diagnostics {
            return;
        }
        let mut open = self.show_diagnostics;
        egui::Window::new("Diagnostics")
            .open(&mut open)
            .resizable(true)
            .default_width(380.0)
            .show(ctx, |ui| self.diagnostics_ui(ui));
        self.show_diagnostics = open;
    }

    fn options_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Snapping grid")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.settings.snap_enabled, "Snap");
                    ui.checkbox(&mut self.settings.show_grid_2d, "2D grid");
                    ui.checkbox(&mut self.settings.show_grid_3d, "3D grid");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.settings.show_origin_marker, "Origin");
                    ui.label("Min px");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.min_grid_pixels)
                            .range(8.0..=80.0)
                            .speed(1.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Spacing");
                    let mut grid = self.document.grid;
                    if ui
                        .add(
                            egui::DragValue::new(&mut grid)
                                .range(1..=10_000_000)
                                .speed(self.edit_step())
                                .suffix(" dbu"),
                        )
                        .changed()
                    {
                        self.set_document_grid(grid);
                    }
                    if ui.button("Tech").clicked() {
                        self.set_document_grid(self.current_technology().grid);
                    }
                });
                let dbu_per_micron = self.current_technology().dbu_per_micron.max(1);
                ui.label(format!(
                    "{} per snap, {} DBU/um",
                    self.format_length(self.document.grid as f64),
                    dbu_per_micron
                ));
            });

        egui::CollapsingHeader::new("Units")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Display");
                    egui::ComboBox::from_id_salt("unit_display_options")
                        .selected_text(self.settings.units.label())
                        .show_ui(ui, |ui| {
                            for unit in UnitDisplay::ALL {
                                ui.selectable_value(&mut self.settings.units, unit, unit.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Precision");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.unit_precision)
                            .range(0..=6)
                            .speed(1),
                    );
                    ui.label(format!(
                        "Preview: {}",
                        self.format_length((self.document.grid * 25) as f64)
                    ));
                });
            });

        egui::CollapsingHeader::new("Appearance")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    egui::ComboBox::from_id_salt("theme_options")
                        .selected_text(self.settings.theme.label())
                        .show_ui(ui, |ui| {
                            for theme in EditorTheme::ALL {
                                ui.selectable_value(&mut self.settings.theme, theme, theme.label());
                            }
                        });
                });
            });

        egui::CollapsingHeader::new("Autosave")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.settings.autosave_enabled, "Enable");
                    ui.label("Every");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.autosave_interval_seconds)
                            .range(5..=3_600)
                            .speed(5)
                            .suffix(" s"),
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button("Save now").clicked() {
                        self.save_document();
                    }
                    if ui.button("Autosave now").clicked() {
                        self.autosave_document();
                        self.last_autosave = Instant::now();
                    }
                    if ui.button("Restore").clicked() {
                        self.restore_autosave_document();
                    }
                });
            });

        egui::CollapsingHeader::new("Input")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(
                    &mut self.settings.single_key_shortcuts,
                    "Single-key shortcuts",
                );
            });

        egui::CollapsingHeader::new("Debug")
            .default_open(false)
            .show(ui, |ui| {
                ui.checkbox(&mut self.show_diagnostics, "Diagnostics window");
                ui.label(format!(
                    "Frame: {:.2} ms, visible shapes: {}",
                    self.perf.frame_ms, self.perf.visible_count
                ));
            });
    }

    fn technology_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Technology");
        let mut selected = self.active_technology;
        let selected_name = self.current_technology().name.clone();
        egui::ComboBox::from_id_salt("technology_picker")
            .selected_text(selected_name)
            .show_ui(ui, |ui| {
                for (index, technology) in self.technologies.iter().enumerate() {
                    ui.selectable_value(&mut selected, index, &technology.name);
                }
            });
        if selected != self.active_technology {
            self.switch_technology(selected);
        }
        let technology = self.current_technology();
        ui.label(format!(
            "Grid: {} dbu (tech {}), DBU/um: {}",
            self.document.grid, technology.grid, technology.dbu_per_micron
        ));
    }

    fn status_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Status");
        ui.label(format!("Shapes: {}", self.document.shapes.len()));
        ui.label(format!("Visible: {}", self.perf.visible_count));
        ui.label(format!(
            "DRC markers: {} active / {} total",
            self.active_marker_count(),
            self.violations.len()
        ));
        ui.label(format!("Frame: {:.2} ms", self.perf.frame_ms));
        if ui.small_button("Diagnostics...").clicked() {
            self.show_diagnostics = true;
        }
    }

    fn diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Renderer");
        ui.label(format!("Visible: {}", self.perf.visible_count));
        ui.label(format!("Query: {:.3} ms", self.perf.query_ms));
        ui.label(format!("GPU build: {:.3} ms", self.perf.gpu_build_ms));
        ui.label(format!("Pick build: {:.3} ms", self.perf.pick_build_ms));
        ui.label(format!(
            "GPU geometry: {} verts / {} idx",
            self.perf.gpu_vertices, self.perf.gpu_indices
        ));
        ui.label(format!(
            "GPU pick: {} verts / {} idx / {}",
            self.perf.gpu_pick_vertices,
            self.perf.gpu_pick_indices,
            self.gpu_pick_label()
        ));
        ui.label(format!(
            "GPU upload: {:.2} MB/frame / {:.2} MB resident",
            self.perf.gpu_upload_bytes as f64 / (1024.0 * 1024.0),
            self.perf.gpu_resident_bytes as f64 / (1024.0 * 1024.0)
        ));
        ui.label(format!(
            "GPU buffer cache: {} layout up / {} skip, {} pick up / {} skip",
            self.perf.gpu_layout_uploads,
            self.perf.gpu_layout_skips,
            self.perf.gpu_pick_uploads,
            self.perf.gpu_pick_skips
        ));

        ui.separator();
        ui.label("Tiles");
        ui.label(format!(
            "{} visible / {} resident / {} rebuilt / {} dirty / {} evicted",
            self.perf.tile_visible_count,
            self.perf.tile_resident_count,
            self.perf.tile_rebuilt_count,
            self.perf.tile_dirty_count,
            self.perf.tile_evicted_count
        ));
        ui.label(format!(
            "Shape cache: {} hit / {} miss / {} resident / {} evicted",
            self.perf.shape_cache_hits,
            self.perf.shape_cache_misses,
            self.perf.resident_shape_batches,
            self.perf.evicted_shape_batches
        ));
        ui.label(format!(
            "Tile shapes: {} visible / {} pick candidates",
            self.perf.tile_shape_count, self.perf.tile_pick_shape_count
        ));
        ui.label(format!("Draw ranges: {}", self.perf.draw_range_count));
        ui.label(format!(
            "LOD: {} tiles / {} summarized / {} precise",
            self.perf.tile_lod_count,
            self.perf.tile_lod_shape_count,
            self.perf.tile_precise_shape_count
        ));
        ui.label(format!(
            "Tile cache: {:.2} / {:.0} MB, over {:.2} MB",
            self.perf.tile_cache_bytes as f64 / (1024.0 * 1024.0),
            self.perf.tile_memory_budget_bytes as f64 / (1024.0 * 1024.0),
            self.perf.tile_over_budget_bytes as f64 / (1024.0 * 1024.0)
        ));

        ui.separator();
        ui.label("Analysis");
        ui.label(format!("Shapes: {}", self.document.shapes.len()));
        ui.label(format!(
            "Markers: {} active / {} total",
            self.active_marker_count(),
            self.violations.len()
        ));
        ui.label(format!("DRC: {:.3} ms", self.perf.drc_ms));
        ui.label(format!("Net: {:.3} ms", self.perf.connectivity_ms));
        ui.label(format!("Route: {:.3} ms", self.perf.route_ms));
        ui.label(format!("Frame: {:.2} ms", self.perf.frame_ms));
        ui.label(format!(
            "Batch: {:.2} MB",
            (self.perf.batch_bytes + self.perf.pick_batch_bytes) as f64 / (1024.0 * 1024.0)
        ));
    }

    fn mes_panel(&mut self, ctx: &egui::Context) {
        if !self.show_mes_panel {
            return;
        }
        egui::TopBottomPanel::bottom("mes_traveler")
            .resizable(true)
            .default_height(260.0)
            .show(ctx, |ui| self.mes_ui(ui));
    }

    fn mes_ui(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.vertical(|ui| self.mes_wip_board_ui(ui));
            ui.separator();
            ui.vertical(|ui| {
                action = self.mes_selected_lot_ui(ui);
                if self.mes.lots.is_empty() {
                    ui_chrome::empty_state(ui, "No traveler selected");
                }
            });
        });
        if let Some((lot_id, action)) = action {
            self.apply_mes_action(&lot_id, action);
        }
    }

    fn mes_wip_board_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "MES WIP");
        if self.selected_mes_lot.is_none() {
            self.selected_mes_lot = self.mes.lots.keys().next().cloned();
        }
        let lot_ids: Vec<_> = self.mes.lots.keys().cloned().collect();
        if lot_ids.is_empty() {
            ui_chrome::empty_state(ui, "No MES lots loaded");
            return;
        }
        for lot_id in lot_ids {
            let Some((label, detail)) = self.mes_wip_row(&lot_id) else {
                continue;
            };
            let selected = self.selected_mes_lot.as_ref() == Some(&lot_id);
            if ui.selectable_label(selected, label).clicked() {
                self.selected_mes_lot = Some(lot_id);
            }
            ui.small(detail);
        }
    }

    fn mes_wip_row(&self, lot_id: &LotId) -> Option<(String, String)> {
        let lot = self.mes.lots.get(lot_id)?;
        let traveler = self.mes.travelers.get(lot_id)?;
        let route = self.mes.routes.get(&lot.route_id)?;
        let step = traveler
            .current_step(route)
            .map(|step| step.name.as_str())
            .unwrap_or("route complete");
        Some((
            format!("{}  {}  {}", lot.id, traveler.status.label(), step),
            format!(
                "{} wafers, {} scrap, {} rework",
                lot.processable_wafer_count(),
                lot.scrapped_wafer_count(),
                lot.rework_wafer_count()
            ),
        ))
    }

    fn mes_selected_lot_ui(&mut self, ui: &mut egui::Ui) -> Option<(LotId, OperatorAction)> {
        let lot_id = self
            .selected_mes_lot
            .clone()
            .or_else(|| self.mes.lots.keys().next().cloned())?;
        self.selected_mes_lot = Some(lot_id.clone());
        let lot = self.mes.lots.get(&lot_id).cloned()?;
        let traveler = self.mes.travelers.get(&lot_id).cloned()?;
        let route = self.mes.routes.get(&lot.route_id).cloned()?;

        ui_chrome::section_label(ui, &format!("Traveler {}", lot.id));
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Product: {}", lot.product));
            ui.separator();
            ui.label(format!("Route: {} {}", route.name, route.revision));
            ui.separator();
            ui.label(format!("Mask: {}", lot.mask_design_id));
            ui.separator();
            ui.label(format!("Layout: {}", lot.layout_revision));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Status: {}", traveler.status.label()));
            ui.separator();
            ui.label(format!(
                "Wafers: {} processable / {} scrapped / {} rework",
                lot.processable_wafer_count(),
                lot.scrapped_wafer_count(),
                lot.rework_wafer_count()
            ));
            if let Some(run) = &traveler.active_run {
                ui.separator();
                ui.label(format!("Running: {} on {}", run.step_id, run.tool_id));
            }
            if let Some(hold) = &traveler.hold {
                ui.separator();
                ui.colored_label(Color32::YELLOW, format!("Hold: {}", hold.reason));
            }
        });

        if let Some(step) = traveler.current_step(&route) {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Current step: {} ({})", step.name, step.id));
                ui.separator();
                ui.label(format!("Area: {}", step.area));
                ui.separator();
                ui.label(format!(
                    "Requires: {} / {}",
                    step.required_tool_class, step.required_recipe
                ));
                ui.separator();
                let tools = step
                    .eligible_tools
                    .iter()
                    .map(ToolId::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!("Eligible tools: {tools}"));
            });
        } else {
            ui.separator();
            ui.label("No current step.");
        }

        ui.separator();
        let mut action = self.mes_actions_ui(ui, &lot, &route, &traveler);
        ui.separator();
        self.mes_audit_ui(ui, &traveler);
        action.take().map(|action| (lot_id, action))
    }

    fn mes_actions_ui(
        &mut self,
        ui: &mut egui::Ui,
        lot: &Lot,
        route: &ProcessRoute,
        traveler: &TravelerState,
    ) -> Option<OperatorAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label("Operator");
            ui.add(
                egui::TextEdit::singleline(&mut self.mes_operator)
                    .desired_width(120.0)
                    .hint_text("operator"),
            );
        });

        ui.horizontal_wrapped(|ui| {
            match traveler.status {
                TravelerStatus::WaitingForStep => {
                    if let Some(step) = traveler.current_step(route) {
                        let tool_id = step
                            .primary_tool()
                            .cloned()
                            .unwrap_or_else(|| ToolId::new("NO-ELIGIBLE-TOOL"));
                        if ui
                            .button(format!("Start {}", step.id))
                            .on_hover_text(format!(
                                "{} / {} on {}",
                                step.required_tool_class, step.required_recipe, tool_id
                            ))
                            .clicked()
                        {
                            action = Some(OperatorAction::StartStep {
                                step_id: step.id.clone(),
                                tool_id,
                                tool_class: step.required_tool_class,
                                recipe_id: step.required_recipe.clone(),
                                operator: self.mes_action_operator(),
                            });
                        }
                    }
                }
                TravelerStatus::Running => {
                    if let Some(run) = &traveler.active_run {
                        if ui.button(format!("Complete {}", run.step_id)).clicked() {
                            action = Some(OperatorAction::CompleteStep {
                                step_id: run.step_id.clone(),
                                operator: self.mes_action_operator(),
                            });
                        }
                    }
                }
                TravelerStatus::WaitingForSignoff => {
                    if let Some(pending) = &traveler.pending_signoff {
                        if ui
                            .button(format!("Sign off {}", pending.run.step_id))
                            .clicked()
                        {
                            action = Some(OperatorAction::SignOff {
                                step_id: pending.run.step_id.clone(),
                                operator: self.mes_action_operator(),
                            });
                        }
                    }
                }
                TravelerStatus::OnHold => {
                    if ui.button("Release hold").clicked() {
                        action = Some(OperatorAction::ReleaseHold {
                            operator: self.mes_action_operator(),
                        });
                    }
                }
                TravelerStatus::Complete => {
                    ui.label("Route complete");
                }
                TravelerStatus::Scrapped => {
                    ui.label("Lot scrapped");
                }
            }

            if !matches!(
                traveler.status,
                TravelerStatus::OnHold | TravelerStatus::Complete | TravelerStatus::Scrapped
            ) {
                if ui.button("Place hold").clicked() {
                    action = Some(OperatorAction::PlaceHold {
                        reason: "MES demo process review".to_string(),
                        operator: self.mes_action_operator(),
                    });
                }
                if let Some(wafer_id) = lot.last_processable_wafer_id() {
                    if ui.button(format!("Scrap {}", wafer_id)).clicked() {
                        action = Some(OperatorAction::ScrapWafer {
                            wafer_id,
                            reason: "MES demo edge defect".to_string(),
                            operator: self.mes_action_operator(),
                        });
                    }
                }
                if let (Some(step_id), Some(wafer_id)) = (
                    traveler.current_step_id.clone(),
                    lot.first_processable_wafer_id(),
                ) {
                    if ui
                        .button(format!("Rework {} to {}", wafer_id, step_id))
                        .clicked()
                    {
                        action = Some(OperatorAction::SendToRework {
                            wafer_ids: vec![wafer_id],
                            target_step: step_id,
                            reason: "MES demo rework verification".to_string(),
                            operator: self.mes_action_operator(),
                        });
                    }
                }
            }
        });
        action
    }

    fn mes_audit_ui(&self, ui: &mut egui::Ui, traveler: &TravelerState) {
        ui.label("Audit trail");
        egui::ScrollArea::vertical()
            .id_salt("mes_audit_scroll")
            .max_height(110.0)
            .show(ui, |ui| {
                for event in traveler.audit_events.iter().rev().take(10) {
                    ui.horizontal_wrapped(|ui| {
                        let color = match event.outcome {
                            AuditOutcome::Accepted => Color32::LIGHT_GREEN,
                            AuditOutcome::Rejected => Color32::LIGHT_RED,
                        };
                        ui.colored_label(
                            color,
                            format!("#{} {}", event.sequence, event.outcome.label()),
                        );
                        ui.label(event.action.summary());
                        ui.small(&event.message);
                    });
                }
            });
    }

    fn mes_action_operator(&self) -> String {
        let operator = self.mes_operator.trim();
        if operator.is_empty() {
            "op.demo".to_string()
        } else {
            operator.to_string()
        }
    }

    fn apply_mes_action(&mut self, lot_id: &LotId, action: OperatorAction) {
        let summary = action.summary();
        let Some(route_id) = self.mes.lots.get(lot_id).map(|lot| lot.route_id.clone()) else {
            self.status = format!("MES lot not found: {lot_id}");
            return;
        };
        let Some(route) = self.mes.routes.get(&route_id).cloned() else {
            self.status = format!("MES route not found: {route_id}");
            return;
        };
        let Some(lot) = self.mes.lots.get_mut(lot_id) else {
            self.status = format!("MES lot not found: {lot_id}");
            return;
        };
        let Some(traveler) = self.mes.travelers.get_mut(lot_id) else {
            self.status = format!("MES traveler not found: {lot_id}");
            return;
        };
        match traveler.apply_action(lot, &route, action) {
            Ok(()) => {
                self.status = format!("MES: {summary}");
            }
            Err(err) => {
                self.status = format!("MES blocked: {err}");
            }
        }
    }

    fn inspector_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("inspector")
            .resizable(true)
            .default_width(ui_chrome::INSPECTOR_WIDTH)
            .show(ctx, |ui| {
                ui_chrome::section_label(ui, "Context");
                egui::ScrollArea::vertical()
                    .id_salt("inspector_panel_scroll")
                    .show(ui, |ui| match self.view_mode {
                        ViewMode::Metrology => self.metrology_context_panel(ui),
                        ViewMode::MaskPrep => self.mask_panel.context_ui(
                            ui,
                            &self.document,
                            &self.mes,
                            self.selected_mes_lot.as_ref(),
                            &mut self.status,
                        ),
                        ViewMode::SpcFdc => {
                            self.spc_fdc_panel.context_ui(
                                ui,
                                &self.yield_analysis,
                                &self.equipment_sim,
                            );
                        }
                        ViewMode::ProcessControl => self
                            .process_control_panel
                            .context_ui(ui, &self.yield_analysis),
                        ViewMode::Traceability => self.genealogy_panel.context_ui(ui),
                        ViewMode::Experiment => {
                            self.experiment_panel.context_ui(ui, &mut self.status);
                        }
                        ViewMode::Layout2d | ViewMode::Layout3d => {
                            self.technology_panel(ui);
                            ui.separator();
                            self.recipe_panel.ui(ui, &mut self.status);
                            ui.separator();
                            self.hierarchy_panel(ui);
                            ui.separator();
                            self.status_panel(ui);
                            ui.separator();
                            self.net_panel(ui);
                            ui.separator();
                            self.marker_panel(ui);
                        }
                        ViewMode::FabControl | ViewMode::Yield => {}
                    });
            });
    }

    fn layers_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("layers")
            .resizable(true)
            .default_width(ui_chrome::LAYERS_WIDTH)
            .show(ctx, |ui| {
                if matches!(self.view_mode, ViewMode::Metrology) {
                    self.metrology_panel(ui);
                    return;
                }
                if matches!(self.view_mode, ViewMode::MaskPrep) {
                    self.mask_panel.layer_stack_ui(ui);
                    return;
                }
                if matches!(self.view_mode, ViewMode::Experiment) {
                    self.experiment_panel
                        .response_capture_ui(ui, &mut self.status);
                    return;
                }
                ui_chrome::section_label(ui, "Layers");
                let mut visibility_ops = Vec::new();
                let mut layer_rows: Vec<_> = self
                    .document
                    .layers
                    .values()
                    .map(|layer| {
                        (
                            layer.id,
                            layer.name.clone(),
                            layer.purpose.clone(),
                            layer.color,
                            layer.visible,
                            layer.display_order,
                        )
                    })
                    .collect();
                layer_rows.sort_by_key(|(id, _, _, _, _, order)| (*order, *id));
                egui::ScrollArea::vertical()
                    .id_salt("layers_panel_scroll")
                    .show(ui, |ui| {
                        for (layer_id, name, purpose, color, mut visible, _) in layer_rows {
                            ui.push_id(("layer_row", layer_id.0), |ui| {
                                ui.horizontal(|ui| {
                                    let color = layer_color32(color, 1.0);
                                    let (swatch_rect, _) =
                                        ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                    ui.painter().rect_filled(swatch_rect, 2.0, color);
                                    if ui
                                        .selectable_label(self.active_layer == layer_id, &name)
                                        .on_hover_text(purpose)
                                        .clicked()
                                    {
                                        self.active_layer = layer_id;
                                    }
                                    if ui.checkbox(&mut visible, "").changed() {
                                        visibility_ops.push((layer_id, visible));
                                    }
                                });
                            });
                        }
                    });
                for (layer, visible) in visibility_ops {
                    self.apply_operation_without_history(&Operation::SetLayerVisibility {
                        layer,
                        visible,
                    });
                }
            });
    }

    fn metrology_context_panel(&mut self, ui: &mut egui::Ui) {
        if self.wafer_map.dies.is_empty() {
            ui_chrome::empty_state(ui, "No metrology dataset loaded");
            return;
        }
        ui.label("FabOS Context");
        ui.label(&self.wafer_map.name);
        ui.separator();
        let links = &self.wafer_map.links;
        ui.label(format!("Lot: {}", links.lot_id));
        ui.label(format!("Wafer: {}", links.wafer_id));
        ui.label(format!("Step: {}", links.process_step_id));
        ui.label(format!("Recipe: {}", links.recipe_id));
        ui.label(format!("Tool run: {}", links.tool_run_id));

        ui.separator();
        let geometry = self.wafer_map.geometry;
        ui.label("Wafer");
        ui.label(format!(
            "{:.0} mm diameter, {:.1} mm edge exclusion",
            geometry.diameter_mm, geometry.edge_exclusion_mm
        ));
        ui.label(format!(
            "{} die, pitch {:.1} x {:.1} mm",
            self.wafer_map.dies.len(),
            geometry.die_pitch_mm[0],
            geometry.die_pitch_mm[1]
        ));

        ui.separator();
        ui.label("Inspection Annotations");
        for annotation in self.wafer_map.annotations.iter().take(5) {
            let location = annotation
                .die
                .map(|die| format!("C{} R{}", die.column, die.row))
                .unwrap_or_else(|| "wafer".to_string());
            ui.label(format!(
                "{:?} {}: {}",
                annotation.kind, location, annotation.note
            ));
        }
    }

    fn metrology_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Metrology");
        if self.wafer_map.dies.is_empty() {
            ui_chrome::empty_state(ui, "No wafer map loaded");
            return;
        }
        let previous_kind = self.metrology_kind;
        egui::ComboBox::from_id_salt("metrology_kind_picker")
            .selected_text(self.metrology_kind.label())
            .show_ui(ui, |ui| {
                for kind in MeasurementKind::ALL {
                    ui.selectable_value(&mut self.metrology_kind, kind, kind.label());
                }
            });
        if self.metrology_kind != previous_kind {
            self.status = format!("metrology filter: {}", self.metrology_kind.label());
        }
        ui.checkbox(&mut self.metrology_failed_only, "Fail/outlier only");

        ui.separator();
        let summary = self.wafer_map.summary(self.metrology_kind);
        self.metrology_summary_ui(ui, summary);

        ui.separator();
        self.metrology_legend_ui(ui, summary);

        ui.separator();
        let histogram = self.wafer_map.histogram(self.metrology_kind, 18);
        self.metrology_histogram_ui(ui, &histogram);

        ui.separator();
        self.metrology_selected_die_ui(ui);
    }

    fn metrology_summary_ui(
        &self,
        ui: &mut egui::Ui,
        summary: layout_model::metrology::MeasurementSummary,
    ) {
        ui.label("Summary");
        ui.label(format!("Samples: {}", summary.sample_count));
        ui.label(format!(
            "Pass: {}  Fail: {}  Outlier: {}",
            summary.pass_count, summary.fail_count, summary.outlier_count
        ));
        if let Some(mean) = summary.mean {
            ui.label(format!(
                "Mean: {}",
                format_metrology_value(summary.kind, mean)
            ));
        }
        if let Some(stddev) = summary.stddev {
            ui.label(format!(
                "Stddev: {}",
                format_metrology_delta(summary.kind, stddev)
            ));
        }
        if let (Some(min), Some(max)) = (summary.min, summary.max) {
            ui.label(format!(
                "Range: {} to {}",
                format_metrology_value(summary.kind, min),
                format_metrology_value(summary.kind, max)
            ));
        }
    }

    fn metrology_legend_ui(
        &self,
        ui: &mut egui::Ui,
        summary: layout_model::metrology::MeasurementSummary,
    ) {
        ui.label("Legend");
        if let (Some(min), Some(max)) = (summary.min, summary.max) {
            ui.horizontal(|ui| {
                metrology_swatch(ui, metrology_gradient_color(0.0));
                ui.label(format_metrology_value(summary.kind, min));
            });
            ui.horizontal(|ui| {
                metrology_swatch(ui, metrology_gradient_color(0.5));
                if let Some(target) = summary.kind.spec().target {
                    ui.label(format!(
                        "Target {}",
                        format_metrology_value(summary.kind, target)
                    ));
                } else {
                    ui.label("Nominal");
                }
            });
            ui.horizontal(|ui| {
                metrology_swatch(ui, metrology_gradient_color(1.0));
                ui.label(format_metrology_value(summary.kind, max));
            });
        }
        ui.horizontal(|ui| {
            metrology_swatch(ui, metrology_status_color(MeasurementStatus::Fail));
            ui.label("Fail");
        });
        ui.horizontal(|ui| {
            metrology_swatch(ui, metrology_status_color(MeasurementStatus::Outlier));
            ui.label("Outlier");
        });
    }

    fn metrology_histogram_ui(&self, ui: &mut egui::Ui, bins: &[HistogramBin]) {
        ui.label("Histogram");
        let desired = vec2(ui.available_width().max(120.0), 120.0);
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 24));
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(70, 82, 86)),
            StrokeKind::Inside,
        );
        if bins.is_empty() {
            return;
        }
        let max_count = bins.iter().map(|bin| bin.count).max().unwrap_or(1).max(1);
        let gap = 2.0;
        let width =
            (rect.width() - gap * (bins.len().saturating_sub(1) as f32)) / bins.len() as f32;
        for (index, bin) in bins.iter().enumerate() {
            let height = rect.height() * (bin.count as f32 / max_count as f32);
            let x = rect.left() + index as f32 * (width + gap);
            let bar = EguiRect::from_min_max(
                Pos2::new(x, rect.bottom() - height),
                Pos2::new(x + width.max(1.0), rect.bottom()),
            );
            painter.rect_filled(
                bar,
                1.0,
                metrology_gradient_color(index as f32 / bins.len() as f32),
            );
        }
    }

    fn metrology_selected_die_ui(&self, ui: &mut egui::Ui) {
        ui.label("Selected Die");
        let Some(die) = self.selected_die else {
            ui.label("None");
            return;
        };
        ui.label(format!("C{} R{}", die.column, die.row));
        for kind in MeasurementKind::ALL {
            if let Some(measurement) = self.wafer_map.measurement_for(die, kind) {
                ui.horizontal(|ui| {
                    metrology_swatch(
                        ui,
                        metrology_measurement_color(
                            measurement,
                            self.wafer_map.summary(measurement.kind),
                        ),
                    );
                    ui.label(kind.label());
                    ui.label(format_metrology_value(kind, measurement.value));
                    ui.label(measurement.status.label());
                });
            }
        }

        let defect_count = self.wafer_map.defects_for_die(die).count();
        if defect_count > 0 {
            ui.label(format!("Defect records: {defect_count}"));
        }
        for annotation in self.wafer_map.annotations_for_die(die) {
            ui.label(format!("{:?}: {}", annotation.kind, annotation.note));
        }
    }

    fn marker_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("DRC Markers");
        ui.label(format!(
            "Active: {} / Total: {} / Saved states: {}",
            self.active_marker_count(),
            self.violations.len(),
            self.document.marker_states.len()
        ));
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_waived_markers, "Waived");
            ui.checkbox(&mut self.show_hidden_markers, "Hidden");
            if ui.button("Clear").clicked() {
                self.clear_marker_states();
            }
        });

        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("drc_marker_scroll")
            .max_height(260.0)
            .show(ui, |ui| {
                for violation in &self.violations {
                    let key = drc_marker_key(violation);
                    let state = self
                        .document
                        .marker_states
                        .get(&key)
                        .cloned()
                        .unwrap_or_default();
                    if state.hidden && !self.show_hidden_markers {
                        continue;
                    }
                    if state.waived && !self.show_waived_markers {
                        continue;
                    }
                    ui.horizontal_wrapped(|ui| {
                        let status = marker_status_label(&state);
                        let label = format!("#{} {}{}", violation.id, violation.rule, status);
                        if ui
                            .selectable_label(false, label)
                            .on_hover_text(&violation.message)
                            .clicked()
                        {
                            action = Some(MarkerAction::FocusAndSelect(violation.clone()));
                        }
                        if ui
                            .small_button(if state.waived { "Unwaive" } else { "Waive" })
                            .clicked()
                        {
                            action = Some(MarkerAction::SetWaived(key.clone(), !state.waived));
                        }
                        if ui
                            .small_button(if state.hidden { "Show" } else { "Hide" })
                            .clicked()
                        {
                            action = Some(MarkerAction::SetHidden(key.clone(), !state.hidden));
                        }
                    });
                }
            });

        if let Some(action) = action {
            match action {
                MarkerAction::FocusAndSelect(violation) => {
                    self.focus_rect(violation.bounds);
                    self.select_violation_shapes(&violation);
                }
                MarkerAction::SetWaived(key, waived) => {
                    self.set_marker_state(key, |state| state.waived = waived);
                }
                MarkerAction::SetHidden(key, hidden) => {
                    self.set_marker_state(key, |state| state.hidden = hidden);
                }
            }
        }
    }

    fn net_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Connectivity");
        if let Some(reason) = &self.connectivity.skipped {
            ui.label(reason);
            return;
        }
        ui.label(format!(
            "Nets: {} / Shorts: {} / Opens: {}",
            self.connectivity.components.len(),
            self.connectivity.shorts.len(),
            self.connectivity.opens.len()
        ));

        if let Some(component) = self.selected_net_component().cloned() {
            let name = component
                .net_name
                .clone()
                .unwrap_or_else(|| format!("component {}", component.id));
            ui.label(format!("Selected net: {name}"));
            ui.label(format!("Shapes: {}", component.shapes.len()));
            if !component.labels.is_empty() {
                let labels = component
                    .labels
                    .iter()
                    .map(|label| label.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!("Labels: {labels}"));
            }
            if !component.explicit_nets.is_empty() {
                let nets = component
                    .explicit_nets
                    .iter()
                    .map(|net| net.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!("Net IDs: {nets}"));
            }
        }

        let mut focus_target = None;
        for short in &self.connectivity.shorts {
            if ui
                .selectable_label(
                    false,
                    format!("Short #{} {}", short.component, short.names.join(" / ")),
                )
                .clicked()
            {
                focus_target = Some(short.bounds);
            }
        }
        for open in &self.connectivity.opens {
            if ui
                .selectable_label(
                    false,
                    format!("Open {} ({} islands)", open.name, open.components.len()),
                )
                .clicked()
            {
                focus_target = Some(open.bounds);
            }
        }
        if let Some(bounds) = focus_target {
            self.focus_rect(bounds);
        }
    }

    fn hierarchy_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Cells");
        let cell_rows: Vec<_> = self
            .document
            .cells
            .values()
            .map(|cell| {
                (
                    cell.id,
                    cell.name.clone(),
                    cell.shapes.len(),
                    cell.instances.len(),
                    cell.id == self.document.top_cell,
                )
            })
            .collect();
        for (id, name, shape_count, instance_count, is_top) in cell_rows {
            ui.horizontal(|ui| {
                let mut edited_name = self
                    .cell_name_drafts
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                let response = ui.add(
                    egui::TextEdit::singleline(&mut edited_name)
                        .desired_width(110.0)
                        .id_salt(("cell_name", id.0)),
                );
                let commit = text_edit_commit_requested(ui, &response);
                if response.changed() || response.has_focus() {
                    self.cell_name_drafts.insert(id, edited_name.clone());
                }
                if commit {
                    self.cell_name_drafts.remove(&id);
                    self.rename_cell(id, edited_name);
                } else if !response.has_focus() && !response.changed() {
                    self.cell_name_drafts.remove(&id);
                }
                ui.label(format!("{shape_count} sh / {instance_count} inst"));
                if !is_top && ui.button("Place").clicked() {
                    self.place_cell_instance(id);
                }
            });
        }

        if let Some(info) = self.selected_instance_info() {
            ui.separator();
            ui.label("Selected instance");
            ui.label(format!(
                "inst #{} -> {} #{}",
                info.id.0, info.target_cell_name, info.target_cell.0
            ));
            ui.horizontal(|ui| {
                ui.label("Name");
                let key = (info.parent, info.id);
                let mut name = self
                    .instance_name_drafts
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| info.name.clone().unwrap_or_default());
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(142.0)
                        .hint_text("unnamed")
                        .id_salt(("instance_name", info.parent.0, info.id.0)),
                );
                let commit = text_edit_commit_requested(ui, &response);
                if response.changed() || response.has_focus() {
                    self.instance_name_drafts.insert(key, name.clone());
                }
                if commit {
                    self.instance_name_drafts.remove(&key);
                    self.rename_instance(info.parent, info.id, Some(name));
                } else if !response.has_focus() && !response.changed() {
                    self.instance_name_drafts.remove(&key);
                }
            });
            let mut x = info.translation.dx;
            let mut y = info.translation.dy;
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(egui::DragValue::new(&mut x).speed(self.edit_step()));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut y).speed(self.edit_step()));
            });
            let delta = Vector::new(x - info.translation.dx, y - info.translation.dy);
            if delta != Vector::ZERO {
                self.move_instance(info.parent, info.id, delta);
            }
            ui.horizontal(|ui| {
                if ui.button("Rot90").clicked() {
                    self.transform_selected_instance(Transform::rotate_cw90(), "rotated instance");
                }
                if ui.button("Mirror X").clicked() {
                    self.transform_selected_instance(Transform::mirror_x(), "mirrored instance X");
                }
                if ui.button("Mirror Y").clicked() {
                    self.transform_selected_instance(Transform::mirror_y(), "mirrored instance Y");
                }
            });
            ui.label(format!("Matrix: {:?}", info.transform.matrix));
            ui.label("Array");
            let mut array = info.array;
            ui.horizontal(|ui| {
                ui.label("Cols");
                ui.add(egui::DragValue::new(&mut array.columns).range(1..=512));
                ui.label("Rows");
                ui.add(egui::DragValue::new(&mut array.rows).range(1..=512));
            });
            ui.horizontal(|ui| {
                ui.label("Col dx");
                ui.add(egui::DragValue::new(&mut array.column_pitch.dx).speed(self.edit_step()));
                ui.label("dy");
                ui.add(egui::DragValue::new(&mut array.column_pitch.dy).speed(self.edit_step()));
            });
            ui.horizontal(|ui| {
                ui.label("Row dx");
                ui.add(egui::DragValue::new(&mut array.row_pitch.dx).speed(self.edit_step()));
                ui.label("dy");
                ui.add(egui::DragValue::new(&mut array.row_pitch.dy).speed(self.edit_step()));
            });
            let array = array.normalized();
            if array != info.array {
                if let Some(mut instance) = self.document.instance(info.parent, info.id).cloned() {
                    instance.array = array;
                    self.replace_instance(info.parent, info.id, instance, "updated instance array");
                }
            }
        } else if let Some(id) = self.selected_top_level_shape() {
            ui.separator();
            self.shape_property_panel(ui, id);
        }
    }

    fn shape_property_panel(&mut self, ui: &mut egui::Ui, id: ShapeId) {
        let Some(mut edited) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let original = edited.clone();
        ui.label(format!("Selected shape #{}", id.0));

        let layer_rows: Vec<_> = self
            .document
            .layers
            .values()
            .map(|layer| (layer.id, layer.name.clone(), layer.color))
            .collect();
        let selected_layer_name = layer_rows
            .iter()
            .find_map(|(layer, name, _)| (*layer == edited.layer).then_some(name.as_str()))
            .unwrap_or("unknown");
        egui::ComboBox::from_id_salt(("shape_layer", id.0))
            .selected_text(selected_layer_name)
            .show_ui(ui, |ui| {
                for (layer, name, color) in &layer_rows {
                    ui.horizontal(|ui| {
                        let swatch = layer_color32(*color, 1.0);
                        let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, swatch);
                        ui.selectable_value(&mut edited.layer, *layer, name);
                    });
                }
            });

        ui.horizontal(|ui| {
            ui.label("Name");
            let mut name = edited.name.clone().unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(150.0)
                        .hint_text("unnamed"),
                )
                .changed()
            {
                let trimmed = name.trim().to_string();
                edited.name = (!trimmed.is_empty()).then_some(trimmed);
            }
        });

        ui.horizontal(|ui| {
            let mut has_net = edited.net.is_some();
            if ui.checkbox(&mut has_net, "Net").changed() {
                edited.net = has_net.then_some(NetId(1));
            }
            if has_net {
                let mut net = edited.net.map_or(1, |net| net.0);
                if ui.add(egui::DragValue::new(&mut net).speed(1)).changed() {
                    edited.net = Some(NetId(net.max(1)));
                }
            }
        });
        if let Some(component) = self.selected_net_component() {
            let inferred = component
                .net_name
                .clone()
                .unwrap_or_else(|| format!("component {}", component.id));
            ui.label(format!("Inferred net: {inferred}"));
        }

        ui.separator();
        shape_kind_property_ui(ui, &mut edited.kind, self.snap_grid());

        if edited != original {
            self.replace_shape(id, original, edited, "updated properties");
        }
    }

    fn selected_net_component(&self) -> Option<&NetComponent> {
        let occurrence = self.selected_occurrence.as_ref()?;
        let component = self.connectivity.component_for_occurrence(occurrence)?;
        self.connectivity.component(component)
    }

    fn select_violation_shapes(&mut self, violation: &DrcViolation) {
        self.selected.clear();
        for id in &violation.shape_ids {
            if self.document.shapes.contains_key(id) {
                self.selected.insert(*id);
            }
        }
        self.selected_occurrence = self
            .selected
            .iter()
            .next()
            .copied()
            .map(ShapeOccurrenceId::top_level);
        self.status = if self.selected.is_empty() {
            format!("focused DRC marker #{}", violation.id)
        } else {
            format!(
                "selected {} shape{} from DRC marker #{}",
                self.selected.len(),
                if self.selected.len() == 1 { "" } else { "s" },
                violation.id
            )
        };
    }

    fn selected_instance_info(&self) -> Option<SelectedInstanceInfo> {
        let occurrence = self.selected_occurrence.as_ref()?;
        let (parent, id) = self
            .document
            .instance_parent_for_path(&occurrence.instance_path)?;
        let instance = self.document.instance(parent, id)?;
        let target_cell = self.document.cell(instance.cell)?;
        Some(SelectedInstanceInfo {
            parent,
            id,
            target_cell: instance.cell,
            target_cell_name: target_cell.name.clone(),
            name: instance.name.clone(),
            transform: instance.transform,
            translation: instance.transform.translation,
            array: instance.array.normalized(),
        })
    }

    fn selected_top_level_shape(&self) -> Option<ShapeId> {
        let id = self.selected.iter().next().copied()?;
        if self.selected_occurrence.as_ref().is_none_or(|occurrence| {
            occurrence.is_top_level() && occurrence.source_shape_id() == id
        }) {
            Some(id)
        } else {
            None
        }
    }

    fn selected_top_level_shapes(&self) -> Vec<Shape> {
        if self
            .selected_occurrence
            .as_ref()
            .is_some_and(|occurrence| !occurrence.is_top_level())
        {
            return Vec::new();
        }
        self.selected
            .iter()
            .filter_map(|id| self.document.shapes.get(id).cloned())
            .collect()
    }

    fn apply_first_vertex_demo_edit(&mut self) {
        self.select_first_top_level_shape();
        let Some(id) = self.selected_top_level_shape() else {
            return;
        };
        let Some(shape) = self.document.shapes.get(&id).cloned() else {
            return;
        };
        let Some(first) = editable_vertex_points(&shape.kind).first().copied() else {
            return;
        };
        let moved = Point::new(first.x - 500, first.y + 420).snap(self.snap_grid());
        if let Some(new_shape) = shape_with_moved_vertex(&shape, 0, moved) {
            self.document
                .apply_operation_without_log(&Operation::ReplaceShape {
                    id,
                    shape: new_shape,
                });
            self.rebuild_indexes();
            self.reset_render_cache();
            self.rerun_drc();
            self.status = "test scene: moved vertex".to_string();
        }
    }

    fn apply_hierarchy_workflow_demo(&mut self) {
        let selected: Vec<_> = self.document.shapes.keys().copied().take(2).collect();
        if selected.is_empty() {
            return;
        }
        self.selected = selected.iter().copied().collect();
        self.selected_occurrence = selected.first().copied().map(ShapeOccurrenceId::top_level);
        self.create_cell_from_selection();
        let Some(info) = self.selected_instance_info() else {
            return;
        };
        let cell_id = info.target_cell;
        if let Some(mut instance) = self.document.instance(info.parent, info.id).cloned() {
            instance.transform = instance.transform.compose(Transform::rotate_cw90());
            instance.array = InstanceArray {
                columns: 3,
                rows: 2,
                column_pitch: Vector::new(1_100, 0),
                row_pitch: Vector::new(0, 900),
            };
            self.replace_instance(info.parent, info.id, instance, "test scene: arrayed cell");
        }
        self.last_view_center = Point::new(4_200, 0);
        self.place_cell_instance(cell_id);
        self.undo.clear();
        self.redo.clear();
        self.status = "test scene: make/place hierarchy workflow".to_string();
    }

    fn hit_selected_vertex(&self, world: Point, tolerance: Coord) -> Option<VertexDrag> {
        let shape_id = self.selected_top_level_shape()?;
        let shape = self.document.shapes.get(&shape_id)?;
        editable_vertex_points(&shape.kind)
            .into_iter()
            .enumerate()
            .filter_map(|(vertex, point)| {
                let distance = point.distance_to(world);
                (distance <= tolerance as f64).then_some((vertex, distance))
            })
            .min_by(|(_, left), (_, right)| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(vertex, _)| VertexDrag {
                shape: shape_id,
                vertex,
            })
    }

    fn hit_selected_edge(&self, world: Point, tolerance: Coord) -> Option<EdgeDrag> {
        let shape_id = self.selected_top_level_shape()?;
        let shape = self.document.shapes.get(&shape_id)?;
        editable_edges(&shape.kind)
            .into_iter()
            .enumerate()
            .filter_map(|(edge, (a, b))| {
                let distance = distance_point_to_segment(world, a, b);
                (distance <= tolerance as f64).then_some((edge, distance))
            })
            .min_by(|(_, left), (_, right)| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(edge, _)| EdgeDrag {
                shape: shape_id,
                edge,
            })
    }

    fn hit_selected_draggable_edge(&self, world: Point, tolerance: Coord) -> Option<EdgeDrag> {
        let edge = self.hit_selected_edge(world, tolerance)?;
        let shape = self.document.shapes.get(&edge.shape)?;
        shape_edge_is_draggable(&shape.kind, edge.edge).then_some(edge)
    }

    fn select_first_top_level_shape(&mut self) {
        let Some(id) = self.document.shapes.keys().next().copied() else {
            return;
        };
        self.selected.clear();
        self.selected.insert(id);
        self.selected_occurrence = Some(ShapeOccurrenceId::top_level(id));
    }

    fn focus_rect(&mut self, rect: Rect) {
        let center = rect.center();
        self.pan = vec2(-(center.x as f32) * self.zoom, center.y as f32 * self.zoom);
    }

    fn reset_3d_camera_to_document(&mut self) {
        let bounds = self
            .layout_bounds()
            .unwrap_or_else(|| Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000)));
        let center = bounds.center();
        let span = bounds.width().abs().max(bounds.height().abs()).max(1_000) as f32;
        let target = Vec3f::new(center.x as f32, center.y as f32, 360.0);
        let position = Vec3f::new(
            center.x as f32 - span * 0.85,
            center.y as f32 - span * 0.95,
            span * 0.62 + 1_800.0,
        );
        self.camera_3d = Camera3d::look_at(position, target, span.max(2_000.0) * 0.85);
    }

    fn nudge_3d_camera_vertical(&mut self, direction: f32) {
        let amount = (self.camera_3d.speed * 0.18).clamp(120.0, 4_000.0);
        self.camera_3d.position += Vec3f::new(0.0, 0.0, direction.signum() * amount);
    }

    fn yield_dashboard(&mut self, ui: &mut egui::Ui) {
        let lot_ids = self
            .yield_analysis
            .lots
            .iter()
            .map(|lot| lot.id.clone())
            .collect::<Vec<_>>();
        if lot_ids.is_empty() {
            ui_chrome::module_header(
                ui,
                "Fab analysis",
                "Yield Dashboard",
                "No data loaded",
                |_| {},
            );
            ui_chrome::empty_state(ui, "No yield dataset loaded");
            return;
        }
        if self.selected_yield_lot.is_empty() {
            if let Some(lot_id) = lot_ids.first() {
                self.selected_yield_lot = lot_id.clone();
            }
        }
        if !lot_ids.contains(&self.selected_yield_lot) {
            self.selected_yield_lot = lot_ids.first().cloned().unwrap_or_default();
        }

        let wafer_ids = self
            .yield_analysis
            .wafer_ids_for_lot(&self.selected_yield_lot);
        if !wafer_ids.contains(&self.selected_yield_wafer) {
            self.selected_yield_wafer = wafer_ids.first().cloned().unwrap_or_default();
        }

        let lot_id = self.selected_yield_lot.clone();
        let wafer_id = self.selected_yield_wafer.clone();
        let lot_summary = self.yield_analysis.lot_summary(&lot_id).cloned();
        let wafer_summary = self
            .yield_analysis
            .wafer_summary(&lot_id, &wafer_id)
            .cloned();
        let die_outcomes = self
            .yield_analysis
            .die_outcomes_for_wafer(&lot_id, &wafer_id);
        let wafer_rows = self
            .yield_analysis
            .wafer_summaries_for_lot(&lot_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let measurements = self
            .yield_analysis
            .measurements_for_wafer(&lot_id, &wafer_id)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let comparisons = self.yield_analysis.lot_comparisons.clone();
        let correlations = self.yield_analysis.correlations.clone();

        egui::ScrollArea::vertical()
            .id_salt("yield_dashboard_scroll")
            .show(ui, |ui| {
                ui_chrome::module_header(ui, "Fab analysis", "Yield Dashboard", "", |ui| {
                    ui.label("Lot");
                    egui::ComboBox::from_id_salt("yield_lot_picker")
                        .selected_text(if lot_id.is_empty() {
                            "none"
                        } else {
                            lot_id.as_str()
                        })
                        .show_ui(ui, |ui| {
                            for candidate in &lot_ids {
                                ui.selectable_value(
                                    &mut self.selected_yield_lot,
                                    candidate.clone(),
                                    candidate,
                                );
                            }
                        });
                    ui.label("Wafer");
                    egui::ComboBox::from_id_salt("yield_wafer_picker")
                        .selected_text(if wafer_id.is_empty() {
                            "none"
                        } else {
                            wafer_id.as_str()
                        })
                        .show_ui(ui, |ui| {
                            for candidate in &wafer_ids {
                                ui.selectable_value(
                                    &mut self.selected_yield_wafer,
                                    candidate.clone(),
                                    candidate,
                                );
                            }
                        });
                });

                if let Some(summary) = &lot_summary {
                    ui.horizontal_wrapped(|ui| {
                        yield_metric_ui(
                            ui,
                            "Lot yield",
                            format_percent(summary.yield_fraction),
                            format!(
                                "{} pass / {} fail / {} dies",
                                summary.passing_dies, summary.failing_dies, summary.total_dies
                            ),
                        );
                        yield_metric_ui(
                            ui,
                            "Main fail",
                            summary
                                .dominant_failure
                                .map(FailureMode::label)
                                .unwrap_or("none")
                                .to_string(),
                            summary.spatial_pattern.label().to_string(),
                        );
                        yield_metric_ui(
                            ui,
                            "Recipe path",
                            lot_recipe_label(&self.yield_analysis, &lot_id),
                            lot_route_label(&self.yield_analysis, &lot_id),
                        );
                    });
                }

                ui.separator();
                ui.columns(2, |columns| {
                    ui_chrome::section_label(&mut columns[0], &format!("Wafer Map {wafer_id}"));
                    draw_yield_wafer_map(&mut columns[0], &die_outcomes);
                    if let Some(summary) = &wafer_summary {
                        columns[0].label(format!(
                            "{} yield, {} failures, {}",
                            format_percent(summary.yield_fraction),
                            summary.failing_dies,
                            summary.spatial_pattern.label()
                        ));
                        for hint in &summary.root_cause_hints {
                            columns[0].label(hint);
                        }
                    }

                    ui_chrome::section_label(&mut columns[1], "Wafer Yield");
                    self.yield_wafer_rows(&mut columns[1], &wafer_rows);
                    columns[1].separator();
                    ui_chrome::section_label(&mut columns[1], "Failure Modes");
                    if let Some(summary) = &lot_summary {
                        failure_breakdown_ui(&mut columns[1], summary);
                    }
                });

                ui.separator();
                ui.columns(2, |columns| {
                    ui_chrome::section_label(&mut columns[0], "Lot / Recipe Comparison");
                    lot_comparison_ui(&mut columns[0], &comparisons);
                    ui_chrome::section_label(
                        &mut columns[1],
                        "Selected Wafer Process Measurements",
                    );
                    wafer_measurements_ui(&mut columns[1], &measurements);
                });

                ui.separator();
                ui_chrome::section_label(ui, "Measurement Correlation");
                correlation_table_ui(ui, &correlations);
            });
    }

    fn yield_wafer_rows(&mut self, ui: &mut egui::Ui, rows: &[YieldSummary]) {
        let mut selected = None;
        egui::Grid::new("yield_wafer_rows")
            .striped(true)
            .min_col_width(62.0)
            .show(ui, |ui| {
                ui.strong("Wafer");
                ui.strong("Yield");
                ui.strong("Fails");
                ui.strong("Pattern");
                ui.end_row();
                for summary in rows {
                    let wafer_id = summary.wafer_id.as_deref().unwrap_or("lot");
                    if ui
                        .selectable_label(self.selected_yield_wafer == wafer_id, wafer_id)
                        .clicked()
                    {
                        selected = Some(wafer_id.to_string());
                    }
                    ui.label(format_percent(summary.yield_fraction));
                    ui.label(summary.failing_dies.to_string());
                    ui.label(summary.spatial_pattern.label());
                    ui.end_row();
                }
            });
        if let Some(wafer_id) = selected {
            self.selected_yield_wafer = wafer_id;
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let frame_start = Instant::now();
        let available = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let canvas = response.rect;
        if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
            return;
        }

        let pointer_delta = ui.input(|input| input.pointer.delta());
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.pan += pointer_delta;
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let previous_zoom = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.008, 4.0);
                if let Some(pointer) = response.hover_pos() {
                    let center = canvas.center();
                    let before = (pointer - center - self.pan) / previous_zoom;
                    self.pan = pointer - center - before * self.zoom;
                }
            }
            if let Some(pointer) = response.hover_pos() {
                let world = self.snap_point(self.screen_to_world(pointer, canvas));
                self.broadcast_cursor(world);
            }
        }

        let viewport = self.viewport_world(canvas);
        self.last_view_center = self.snap_point(viewport.center());
        self.draw_background(&painter, canvas, viewport);
        let query_started = Instant::now();
        let visible_occurrences = self.index.query_occurrences(viewport);
        self.perf.query_ms = query_started.elapsed().as_secs_f64() * 1000.0;
        self.perf.visible_count = visible_occurrences.len();
        let batch_started = Instant::now();
        let pick_request = if matches!(self.tool, Tool::Select) {
            response.hover_pos().map(|pointer| {
                self.gpu_pick_serial = self.gpu_pick_serial.wrapping_add(1);
                GpuPickRequest::new(
                    self.gpu_pick_serial,
                    [pointer.x, pointer.y],
                    [pointer.x - canvas.min.x, pointer.y - canvas.min.y],
                    [canvas.width(), canvas.height()],
                )
            })
        } else {
            None
        };
        let renderer::TiledFrame {
            render: batch,
            pick: pick_batch,
            stats: tile_stats,
            ..
        } = self.tile_cache.build_frame_with_options(
            &self.document,
            &self.index,
            viewport,
            renderer::TileFrameOptions {
                include_pick: pick_request.is_some(),
                zoom: self.zoom,
                memory_budget_bytes: Some(TILE_MEMORY_BUDGET_BYTES),
                ..Default::default()
            },
        );
        self.perf.gpu_build_ms = batch_started.elapsed().as_secs_f64() * 1000.0;
        self.perf.gpu_vertices = batch.vertices.len();
        self.perf.gpu_indices = batch.indices.len();
        self.perf.gpu_pick_vertices = pick_batch.as_ref().map_or(0, |batch| batch.vertices.len());
        self.perf.gpu_pick_indices = pick_batch.as_ref().map_or(0, |batch| batch.indices.len());
        self.perf.tile_visible_count = tile_stats.visible_tiles;
        self.perf.tile_resident_count = tile_stats.resident_tiles;
        self.perf.tile_rebuilt_count = tile_stats.rebuilt_tiles;
        self.perf.tile_evicted_count = tile_stats.evicted_tiles;
        self.perf.tile_lod_count = tile_stats.lod_tiles;
        self.perf.tile_lod_shape_count = tile_stats.lod_shapes;
        self.perf.tile_precise_shape_count = tile_stats.precise_shapes;
        self.perf.tile_cached_count = tile_stats.cached_tiles;
        self.perf.tile_shape_count = tile_stats.visible_shapes;
        self.perf.tile_pick_shape_count = tile_stats.pick_shapes;
        self.perf.shape_cache_hits = tile_stats.shape_cache_hits;
        self.perf.shape_cache_misses = tile_stats.shape_cache_misses;
        self.perf.resident_shape_batches = tile_stats.resident_shape_batches;
        self.perf.evicted_shape_batches = tile_stats.evicted_shape_batches;
        self.perf.draw_range_count = tile_stats.draw_ranges;
        self.perf.tile_cache_bytes = tile_stats.cache_bytes;
        self.perf.tile_memory_budget_bytes = tile_stats
            .memory_budget_bytes
            .unwrap_or(TILE_MEMORY_BUDGET_BYTES);
        self.perf.tile_over_budget_bytes = tile_stats.over_budget_bytes;
        self.perf.pick_build_ms = tile_stats.pick_build_ms;
        self.perf.batch_bytes = batch.estimate_bytes();
        self.perf.pick_batch_bytes = pick_batch
            .as_ref()
            .map_or(0, |batch| batch.estimate_bytes());
        if let Ok(upload) = self.gpu_upload_state.lock() {
            self.perf.gpu_upload_bytes = upload.last_upload_bytes;
            self.perf.gpu_resident_bytes = upload.resident_bytes;
            self.perf.gpu_layout_uploads = upload.layout_uploads;
            self.perf.gpu_layout_skips = upload.layout_skips;
            self.perf.gpu_pick_uploads = upload.pick_uploads;
            self.perf.gpu_pick_skips = upload.pick_skips;
        }

        if let Some(target_format) = self.gpu_target_format {
            painter.add(egui_wgpu::Callback::new_paint_callback(
                canvas,
                LayoutGpuCallback {
                    batch,
                    pick_batch,
                    pick_request,
                    pick_state: Arc::clone(&self.gpu_pick_state),
                    upload_state: Arc::clone(&self.gpu_upload_state),
                    uniforms: ViewUniforms::from_viewport(viewport),
                    target_format,
                },
            ));
            self.draw_visible_overlays(&painter, canvas, viewport, &visible_occurrences);
        } else {
            self.draw_visible_cpu_shapes(&painter, canvas, viewport, &visible_occurrences);
        }
        self.draw_selected_net_highlight(&painter, canvas, viewport);
        self.draw_violations(&painter, canvas);
        self.draw_selected_vertex_handles(&painter, canvas);
        self.draw_edit_preview(ui, &painter, canvas);
        self.draw_route_points(&painter, canvas);
        self.draw_remote_selections(&painter, canvas, viewport);
        self.draw_remote_cursors(&painter, canvas);
        self.draw_scale_bar(&painter, canvas);
        self.handle_canvas_input(ui, &response, canvas);
        self.broadcast_selection_if_changed();
        self.perf.frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
    }

    fn canvas_3d(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let canvas = response.rect;
        if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
            return;
        }

        self.handle_3d_input(ui, &response);
        let face_count = if let Some(target_format) = self.gpu_target_format {
            let (batch, face_count) = self.build_3d_render_batch();
            painter.add(egui_wgpu::Callback::new_paint_callback(
                canvas,
                Viewport3dGpuCallback {
                    batch,
                    uniforms: self.viewport_3d_uniforms(canvas),
                    viewport_size: [canvas.width(), canvas.height()],
                    target_format,
                },
            ));
            face_count
        } else {
            painter.rect_filled(canvas, 0.0, Color32::from_rgb(8, 11, 14));
            self.draw_3d_ground_grid(&painter, canvas);
            self.draw_3d_layout_cpu_fallback(&painter, canvas)
        };
        self.draw_3d_hud(&painter, canvas, face_count);
    }

    fn metrology_canvas(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(available, Sense::click());
        let canvas = response.rect;
        if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
            return;
        }

        painter.rect_filled(canvas, 0.0, Color32::from_rgb(12, 15, 17));
        if self.wafer_map.dies.is_empty() {
            painter.text(
                canvas.center(),
                Align2::CENTER_CENTER,
                "no metrology dataset loaded",
                FontId::proportional(16.0),
                Color32::from_rgb(190, 198, 196),
            );
            return;
        }
        let side = (canvas.width().min(canvas.height()) - 56.0).max(180.0);
        let wafer_rect = EguiRect::from_center_size(canvas.center(), vec2(side, side));
        let center = wafer_rect.center();
        let geometry = self.wafer_map.geometry;
        let wafer_radius_px = side * 0.5;
        let scale = side / geometry.diameter_mm as f32;
        let active_radius_px = geometry.active_radius_mm() as f32 * scale;
        let summary = self.wafer_map.summary(self.metrology_kind);

        painter.circle_filled(center, wafer_radius_px, Color32::from_rgb(24, 29, 31));
        painter.circle_stroke(
            center,
            wafer_radius_px,
            Stroke::new(2.0, Color32::from_rgb(135, 154, 158)),
        );
        painter.circle_stroke(
            center,
            active_radius_px,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(180, 190, 184, 105)),
        );

        let mut hovered_die = None;
        let pointer = response.hover_pos();
        for &die in &self.wafer_map.dies {
            let Some(measurement) = self.wafer_map.measurement_for(die, self.metrology_kind) else {
                continue;
            };
            let die_center = wafer_mm_to_screen(geometry.die_center_mm(die), center, scale);
            let die_size = vec2(
                (geometry.die_size_mm[0] as f32 * scale).max(2.0),
                (geometry.die_size_mm[1] as f32 * scale).max(2.0),
            ) * 0.92;
            let die_rect = EguiRect::from_center_size(die_center, die_size);
            if pointer.is_some_and(|point| die_rect.contains(point)) {
                hovered_die = Some(die);
            }

            let filtered =
                self.metrology_failed_only && measurement.status == MeasurementStatus::Pass;
            let fill = if filtered {
                Color32::from_rgba_unmultiplied(42, 48, 50, 72)
            } else {
                metrology_measurement_color(measurement, summary)
            };
            painter.rect_filled(die_rect, 1.0, fill);
            painter.rect_stroke(
                die_rect,
                1.0,
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(9, 12, 14, 150)),
                StrokeKind::Inside,
            );

            if self.selected_die == Some(die) {
                painter.rect_stroke(
                    die_rect.expand(1.5),
                    1.0,
                    Stroke::new(2.0, Color32::WHITE),
                    StrokeKind::Outside,
                );
            }
        }

        if self.metrology_kind == MeasurementKind::DefectCount || self.selected_die.is_some() {
            for defect in &self.wafer_map.defects {
                if self.metrology_kind != MeasurementKind::DefectCount
                    && Some(defect.die) != self.selected_die
                {
                    continue;
                }
                let pos = wafer_mm_to_screen(defect.position_mm, center, scale);
                painter.circle_filled(
                    pos,
                    (2.0 + defect.severity as f32).min(6.0),
                    Color32::from_rgb(28, 28, 28),
                );
                painter.circle_stroke(
                    pos,
                    (2.0 + defect.severity as f32).min(6.0),
                    Stroke::new(1.0, Color32::from_rgb(255, 224, 128)),
                );
            }
        }

        for annotation in &self.wafer_map.annotations {
            let pos = wafer_mm_to_screen(annotation.position_mm, center, scale);
            painter.circle_stroke(pos, 7.0, Stroke::new(1.5, Color32::from_rgb(130, 210, 230)));
        }

        if let Some(die) = hovered_die {
            let die_center = wafer_mm_to_screen(geometry.die_center_mm(die), center, scale);
            let die_size = vec2(
                geometry.die_size_mm[0] as f32 * scale,
                geometry.die_size_mm[1] as f32 * scale,
            ) * 0.96;
            painter.rect_stroke(
                EguiRect::from_center_size(die_center, die_size).expand(2.5),
                1.0,
                Stroke::new(1.5, Color32::from_rgb(255, 255, 190)),
                StrokeKind::Outside,
            );
        }

        if response.clicked() {
            self.selected_die = hovered_die;
            if let Some(die) = hovered_die {
                self.status = format!("selected wafer die C{} R{}", die.column, die.row);
            }
        }

        let header = format!(
            "{}  {}  {} die",
            self.wafer_map.links.lot_id,
            self.wafer_map.links.wafer_id,
            self.wafer_map.dies.len()
        );
        painter.text(
            canvas.left_top() + vec2(16.0, 14.0),
            Align2::LEFT_TOP,
            header,
            FontId::proportional(14.0),
            Color32::from_rgb(220, 230, 226),
        );
        painter.text(
            canvas.left_top() + vec2(16.0, 36.0),
            Align2::LEFT_TOP,
            self.metrology_kind.label(),
            FontId::proportional(18.0),
            Color32::from_rgb(240, 244, 236),
        );
    }

    fn handle_3d_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.dragged_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Secondary)
        {
            let delta = ui.input(|input| input.pointer.delta());
            self.camera_3d.yaw += delta.x * 0.006;
            self.camera_3d.pitch = (self.camera_3d.pitch - delta.y * 0.006).clamp(-1.45, 1.45);
        }

        if !response.hovered() && !response.has_focus() && !response.dragged() {
            return;
        }

        let basis = self.camera_3d.basis();
        ui.input(|input| {
            let mut direction = Vec3f::ZERO;
            if input.key_down(Key::W) || input.key_down(Key::ArrowUp) {
                direction += basis.forward;
            }
            if input.key_down(Key::S) || input.key_down(Key::ArrowDown) {
                direction += basis.forward * -1.0;
            }
            if input.key_down(Key::D) || input.key_down(Key::ArrowRight) {
                direction += basis.right;
            }
            if input.key_down(Key::A) || input.key_down(Key::ArrowLeft) {
                direction += basis.right * -1.0;
            }
            if input.key_down(Key::Space) {
                direction += Vec3f::new(0.0, 0.0, 1.0);
            }
            if input.key_down(Key::E) {
                direction += Vec3f::new(0.0, 0.0, 1.0);
            }
            if input.key_down(Key::Q) {
                direction += Vec3f::new(0.0, 0.0, -1.0);
            }

            let speed = if input.modifiers.shift {
                self.camera_3d.speed * 3.0
            } else {
                self.camera_3d.speed
            };
            let dt = input.stable_dt.clamp(1.0 / 240.0, 1.0 / 20.0);
            if direction.length() > 0.0 {
                self.camera_3d.position += direction.normalized() * speed * dt;
            }

            let scroll = input.raw_scroll_delta.y;
            if scroll.abs() > 0.0 {
                self.camera_3d.position += basis.forward * scroll * self.camera_3d.speed * 0.0015;
            }
        });
    }

    fn build_3d_render_batch(&self) -> (renderer::RenderBatch3d, usize) {
        let mut faces = Vec::new();
        for flattened in self.document.visible_flattened_shapes() {
            let shape = flattened.transformed_shape();
            self.add_shape_3d_faces(&mut faces, &shape);
            if faces.len() >= MAX_3D_FACES {
                break;
            }
        }

        let face_count = faces.len();
        let mut batch = renderer::RenderBatch3d::default();
        for face in &faces {
            append_face_to_3d_batch(&mut batch, face);
        }
        self.append_3d_scene_guides(&mut batch);
        (batch, face_count)
    }

    fn draw_3d_layout_cpu_fallback(&self, painter: &Painter, canvas: EguiRect) -> usize {
        let mut faces = Vec::new();
        for flattened in self.document.visible_flattened_shapes() {
            let shape = flattened.transformed_shape();
            self.add_shape_3d_faces(&mut faces, &shape);
            if faces.len() >= MAX_3D_FACES {
                break;
            }
        }

        let basis = self.camera_3d.basis();
        let mut projected = Vec::with_capacity(faces.len());
        for face in faces {
            let clipped = self.clip_3d_face_to_near_plane(&face.points, basis);
            if clipped.len() < 3 {
                continue;
            }

            let mut points = Vec::with_capacity(clipped.len());
            let mut depth = 0.0;
            for point in &clipped {
                points.push(self.project_camera_point(*point, canvas));
                depth += point.depth;
            }
            let average_depth = depth / points.len() as f32;
            projected.push(ProjectedFace {
                surface: face.surface,
                depth: average_depth,
                order: face.order,
                points,
                fill: face.fill,
                stroke: face.stroke,
            });
        }

        projected.sort_by(compare_projected_faces_3d_cpu_fallback);
        let rendered = projected.len();
        for face in projected {
            painter.add(egui::Shape::convex_polygon(
                face.points,
                face.fill,
                Stroke::new(0.75, face.stroke),
            ));
        }
        rendered
    }

    fn viewport_3d_uniforms(&self, canvas: EguiRect) -> Viewport3dUniforms {
        Viewport3dUniforms::from_view_projection(self.view_projection_3d(canvas))
    }

    fn view_projection_3d(&self, canvas: EguiRect) -> [f32; 16] {
        let basis = self.camera_3d.basis();
        let aspect = (canvas.width() / canvas.height()).max(0.001);
        let y_scale = 1.0 / (self.camera_3d.fov_y * 0.5).tan();
        let x_scale = y_scale / aspect;
        let near = CAMERA_NEAR_PLANE.max(0.001);
        let far = self.camera_3d_far_plane().max(near + 1.0);
        let z_scale = far / (far - near);
        let z_bias = -near * far / (far - near);
        let camera = self.camera_3d.position;

        let rows = [
            [
                basis.right.x * x_scale,
                basis.right.y * x_scale,
                basis.right.z * x_scale,
                -camera.dot(basis.right) * x_scale,
            ],
            [
                basis.up.x * y_scale,
                basis.up.y * y_scale,
                basis.up.z * y_scale,
                -camera.dot(basis.up) * y_scale,
            ],
            [
                basis.forward.x * z_scale,
                basis.forward.y * z_scale,
                basis.forward.z * z_scale,
                -camera.dot(basis.forward) * z_scale + z_bias,
            ],
            [
                basis.forward.x,
                basis.forward.y,
                basis.forward.z,
                -camera.dot(basis.forward),
            ],
        ];
        row_major_4x4_to_column_major(rows)
    }

    fn camera_3d_far_plane(&self) -> f32 {
        let Some(bounds) = self.layout_bounds() else {
            return 100_000.0;
        };
        let max_layer_z = self.document.layers.len().max(1) as f32 * 320.0 + 2_000.0;
        let corners = [
            Vec3f::new(bounds.min.x as f32, bounds.min.y as f32, 0.0),
            Vec3f::new(bounds.max.x as f32, bounds.min.y as f32, 0.0),
            Vec3f::new(bounds.max.x as f32, bounds.max.y as f32, 0.0),
            Vec3f::new(bounds.min.x as f32, bounds.max.y as f32, 0.0),
            Vec3f::new(bounds.min.x as f32, bounds.min.y as f32, max_layer_z),
            Vec3f::new(bounds.max.x as f32, bounds.min.y as f32, max_layer_z),
            Vec3f::new(bounds.max.x as f32, bounds.max.y as f32, max_layer_z),
            Vec3f::new(bounds.min.x as f32, bounds.max.y as f32, max_layer_z),
        ];
        let forward = self.camera_3d.forward();
        let max_forward_depth = corners
            .into_iter()
            .map(|corner| (corner - self.camera_3d.position).dot(forward))
            .fold(CAMERA_NEAR_PLANE * 4.0, f32::max);
        let margin =
            bounds.width().abs().max(bounds.height().abs()).max(1_000) as f32 * 0.15 + max_layer_z;
        (max_forward_depth + margin).max(50_000.0)
    }

    fn add_shape_3d_faces(&self, faces: &mut Vec<Face3d>, shape: &Shape) {
        if faces.len() >= MAX_3D_FACES {
            return;
        }
        let Some((base_z, top_z, color)) = self.layer_3d_style(shape.layer) else {
            return;
        };
        match &shape.kind {
            ShapeKind::Rectangle(rect) => {
                add_slab_faces(faces, &rect.corners(), base_z, top_z, color);
            }
            ShapeKind::Polygon(poly) if poly.points.len() >= 3 => {
                add_slab_faces(faces, &poly.points, base_z, top_z, color);
            }
            ShapeKind::Path { points, width } => {
                for segment in points.windows(2) {
                    if let [a, b] = segment {
                        add_path_segment_3d_faces(faces, *a, *b, *width, base_z, top_z, color);
                    }
                    if faces.len() >= MAX_3D_FACES {
                        break;
                    }
                }
            }
            ShapeKind::Via { center, size, .. } => {
                let half = *size / 2;
                let rect = Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                );
                add_slab_faces(faces, &rect.corners(), base_z, top_z + 120.0, color);
            }
            ShapeKind::Label { .. } | ShapeKind::Measurement { .. } | ShapeKind::Polygon(_) => {}
        }
    }

    fn draw_3d_ground_grid(&self, painter: &Painter, canvas: EguiRect) {
        if !self.settings.show_grid_3d {
            return;
        }
        let bounds = self
            .layout_bounds()
            .unwrap_or_else(|| Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000)));
        let span = bounds.width().abs().max(bounds.height().abs()).max(1_000);
        let expanded = bounds.expanded(span / 2);
        let step = nice_scale_length_dbu((span as f32 / 10.0).max(self.snap_grid() as f32));
        let basis = self.camera_3d.basis();
        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(115, 150, 160, 80));

        let min_x = expanded.min.x.div_euclid(step) * step;
        let max_x = expanded.max.x.div_euclid(step) * step + step;
        let min_y = expanded.min.y.div_euclid(step) * step;
        let max_y = expanded.max.y.div_euclid(step) * step + step;
        let mut x = min_x;
        while x <= max_x {
            self.draw_3d_line(
                painter,
                canvas,
                basis,
                Vec3f::new(x as f32, min_y as f32, 0.0),
                Vec3f::new(x as f32, max_y as f32, 0.0),
                stroke,
            );
            x += step;
        }
        let mut y = min_y;
        while y <= max_y {
            self.draw_3d_line(
                painter,
                canvas,
                basis,
                Vec3f::new(min_x as f32, y as f32, 0.0),
                Vec3f::new(max_x as f32, y as f32, 0.0),
                stroke,
            );
            y += step;
        }
    }

    fn append_3d_scene_guides(&self, batch: &mut renderer::RenderBatch3d) {
        if !self.settings.show_grid_3d {
            return;
        }
        let bounds = self
            .layout_bounds()
            .unwrap_or_else(|| Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000)));
        let span = bounds.width().abs().max(bounds.height().abs()).max(1_000);
        let expanded = bounds.expanded(span / 2);
        let step = nice_scale_length_dbu((span as f32 / 10.0).max(self.snap_grid() as f32));
        let min_x = expanded.min.x.div_euclid(step) * step;
        let max_x = expanded.max.x.div_euclid(step) * step + step;
        let min_y = expanded.min.y.div_euclid(step) * step;
        let max_y = expanded.max.y.div_euclid(step) * step + step;
        let grid_color = color32_to_gpu(Color32::from_rgba_unmultiplied(115, 150, 160, 82));
        let major_color = color32_to_gpu(Color32::from_rgba_unmultiplied(170, 200, 204, 118));
        let x_axis_color = color32_to_gpu(Color32::from_rgba_unmultiplied(235, 96, 90, 190));
        let y_axis_color = color32_to_gpu(Color32::from_rgba_unmultiplied(90, 210, 138, 190));

        let mut x = min_x;
        while x <= max_x {
            let color = if x == 0 || x % (step * 5) == 0 {
                major_color
            } else {
                grid_color
            };
            append_guide_line_to_3d_batch(
                batch,
                Vec3f::new(x as f32, min_y as f32, 0.0),
                Vec3f::new(x as f32, max_y as f32, 0.0),
                if x == 0 { x_axis_color } else { color },
            );
            x += step;
        }

        let mut y = min_y;
        while y <= max_y {
            let color = if y == 0 || y % (step * 5) == 0 {
                major_color
            } else {
                grid_color
            };
            append_guide_line_to_3d_batch(
                batch,
                Vec3f::new(min_x as f32, y as f32, 0.0),
                Vec3f::new(max_x as f32, y as f32, 0.0),
                if y == 0 { y_axis_color } else { color },
            );
            y += step;
        }
    }

    fn draw_3d_line(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        basis: CameraBasis,
        a: Vec3f,
        b: Vec3f,
        stroke: Stroke,
    ) {
        let a = self.camera_space_3d_point(a, basis);
        let b = self.camera_space_3d_point(b, basis);
        let Some((a, b)) = clip_camera_line_to_near(a, b, CAMERA_NEAR_PLANE) else {
            return;
        };
        painter.line_segment(
            [
                self.project_camera_point(a, canvas),
                self.project_camera_point(b, canvas),
            ],
            stroke,
        );
    }

    fn draw_3d_hud(&self, painter: &Painter, canvas: EguiRect, face_count: usize) {
        let position = self.camera_3d.position;
        let text = format!(
            "3D flycam  faces: {}  xyz: {:.0}, {:.0}, {:.0}",
            face_count, position.x, position.y, position.z
        );
        painter.text(
            canvas.left_top() + vec2(12.0, 12.0),
            Align2::LEFT_TOP,
            text,
            FontId::monospace(12.0),
            Color32::from_rgb(220, 232, 228),
        );
    }

    fn camera_space_3d_point(&self, point: Vec3f, basis: CameraBasis) -> CameraPoint3d {
        let relative = point - self.camera_3d.position;
        CameraPoint3d {
            x: relative.dot(basis.right),
            y: relative.dot(basis.up),
            depth: relative.dot(basis.forward),
        }
    }

    fn project_camera_point(&self, point: CameraPoint3d, canvas: EguiRect) -> Pos2 {
        let focal = 0.5 * canvas.height() / (0.5 * self.camera_3d.fov_y).tan();
        canvas.center()
            + vec2(
                point.x * focal / point.depth,
                -point.y * focal / point.depth,
            )
    }

    fn clip_3d_face_to_near_plane(
        &self,
        points: &[Vec3f],
        basis: CameraBasis,
    ) -> Vec<CameraPoint3d> {
        let camera_points: Vec<_> = points
            .iter()
            .map(|point| self.camera_space_3d_point(*point, basis))
            .collect();
        clip_camera_polygon_to_near(&camera_points, CAMERA_NEAR_PLANE)
    }

    fn layer_3d_style(&self, layer_id: LayerId) -> Option<(f32, f32, Color32)> {
        let layer = self.document.layers.get(&layer_id)?;
        if !layer.visible || matches!(layer.process, ProcessLayer::Annotation) {
            return None;
        }
        let mut layer_order: Vec<_> = self
            .document
            .layers
            .values()
            .filter(|layer| !matches!(layer.process, ProcessLayer::Annotation))
            .map(|layer| (layer.display_order, layer.id))
            .collect();
        layer_order.sort_unstable();
        let index = layer_order
            .iter()
            .position(|(_, id)| *id == layer_id)
            .unwrap_or(0) as f32;
        let base_z = index * 170.0;
        let thickness = match layer.process {
            ProcessLayer::Contact | ProcessLayer::Via1 => 240.0,
            ProcessLayer::Oxide => 55.0,
            _ => 95.0,
        };
        Some((base_z, base_z + thickness, layer_color_3d(layer.color)))
    }

    fn layout_bounds(&self) -> Option<Rect> {
        self.document
            .visible_flattened_shapes()
            .into_iter()
            .map(|shape| shape.bounds)
            .reduce(|left, right| left.union(right))
    }

    fn draw_background(&self, painter: &Painter, canvas: EguiRect, viewport: Rect) {
        painter.rect_filled(canvas, 0.0, Color32::from_rgb(13, 16, 18));
        if !self.settings.show_grid_2d {
            if self.settings.show_origin_marker {
                let origin = self.world_to_screen(Point::ZERO, canvas);
                painter.circle_filled(origin, 3.0, Color32::from_rgb(220, 220, 210));
            }
            return;
        }
        let grid = self.grid_step_for_zoom();
        let min_x = viewport.min.x.div_euclid(grid) * grid;
        let max_x = viewport.max.x.div_euclid(grid) * grid + grid;
        let min_y = viewport.min.y.div_euclid(grid) * grid;
        let max_y = viewport.max.y.div_euclid(grid) * grid + grid;
        let minor = Stroke::new(1.0, Color32::from_rgba_premultiplied(82, 94, 98, 38));
        let major = Stroke::new(1.0, Color32::from_rgba_premultiplied(116, 132, 138, 70));
        for x in (min_x..=max_x).step_by(grid as usize) {
            let stroke = if x == 0 || x % (grid * 5) == 0 {
                major
            } else {
                minor
            };
            let a = self.world_to_screen(Point::new(x, min_y), canvas);
            let b = self.world_to_screen(Point::new(x, max_y), canvas);
            painter.line_segment([a, b], stroke);
        }
        for y in (min_y..=max_y).step_by(grid as usize) {
            let stroke = if y == 0 || y % (grid * 5) == 0 {
                major
            } else {
                minor
            };
            let a = self.world_to_screen(Point::new(min_x, y), canvas);
            let b = self.world_to_screen(Point::new(max_x, y), canvas);
            painter.line_segment([a, b], stroke);
        }
        if self.settings.show_origin_marker {
            let origin = self.world_to_screen(Point::ZERO, canvas);
            painter.circle_filled(origin, 3.0, Color32::from_rgb(220, 220, 210));
        }
    }

    fn draw_scale_bar(&self, painter: &Painter, canvas: EguiRect) {
        if canvas.width() < 160.0 || canvas.height() < 80.0 {
            return;
        }
        let length_dbu = nice_scale_length_dbu(120.0 / self.zoom.max(0.000_1));
        let length_px = length_dbu as f32 * self.zoom;
        if length_px < 24.0 {
            return;
        }

        let left = canvas.left() + 18.0;
        let baseline = canvas.bottom() - 22.0;
        let right = left + length_px;
        let tick_top = baseline - 8.0;
        let label = self.format_length(length_dbu as f64);
        let text_pos = Pos2::new((left + right) * 0.5, tick_top - 5.0);
        let bg = EguiRect::from_min_max(
            Pos2::new(left - 10.0, tick_top - 26.0),
            Pos2::new(right + 10.0, baseline + 10.0),
        );

        painter.rect_filled(bg, 4.0, Color32::from_rgba_premultiplied(6, 8, 10, 178));
        let stroke = Stroke::new(2.0, Color32::from_rgb(238, 242, 232));
        painter.line_segment(
            [Pos2::new(left, baseline), Pos2::new(right, baseline)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(left, tick_top), Pos2::new(left, baseline)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(right, tick_top), Pos2::new(right, baseline)],
            stroke,
        );
        painter.text(
            text_pos,
            Align2::CENTER_BOTTOM,
            label,
            FontId::monospace(12.0),
            Color32::from_rgb(238, 242, 232),
        );
    }

    fn draw_shape_for_occurrence(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        occurrence: &ShapeOccurrenceId,
        shape: &Shape,
    ) {
        let selected = if let Some(selected) = &self.selected_occurrence {
            selected == occurrence
        } else {
            self.selected.contains(&shape.id)
        };
        self.draw_shape_with_selected(painter, canvas, shape, selected);
    }

    fn draw_shape_with_selected(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        shape: &Shape,
        selected: bool,
    ) {
        let color = layer_color32(
            self.document.layer_color(shape.layer),
            if selected { 1.25 } else { 1.0 },
        );
        let stroke = Stroke::new(
            if selected { 2.0 } else { 1.0 },
            if selected {
                Color32::WHITE
            } else {
                Color32::from_rgba_premultiplied(220, 224, 216, 120)
            },
        );
        match &shape.kind {
            ShapeKind::Rectangle(rect) => {
                self.draw_polygon_points(painter, canvas, &rect.corners(), color, stroke)
            }
            ShapeKind::Polygon(poly) => {
                self.draw_polygon_points(painter, canvas, &poly.points, color, stroke)
            }
            ShapeKind::Path { points, width } => {
                let screen_points: Vec<Pos2> = points
                    .iter()
                    .map(|point| self.world_to_screen(*point, canvas))
                    .collect();
                let stroke = Stroke::new((*width as f32 * self.zoom).max(2.0), color);
                painter.add(egui::Shape::line(screen_points, stroke));
            }
            ShapeKind::Via { center, size, .. } => {
                let half = *size / 2;
                let rect = Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                );
                self.draw_polygon_points(painter, canvas, &rect.corners(), color, stroke);
            }
            ShapeKind::Label { position, text } => {
                painter.text(
                    self.world_to_screen(*position, canvas),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::monospace(13.0),
                    color,
                );
            }
            ShapeKind::Measurement { a, b, label } => {
                let a = self.world_to_screen(*a, canvas);
                let b = self.world_to_screen(*b, canvas);
                painter.line_segment([a, b], Stroke::new(1.5, Color32::from_rgb(245, 240, 190)));
                painter.text(
                    a.lerp(b, 0.5),
                    Align2::CENTER_BOTTOM,
                    label,
                    FontId::monospace(12.0),
                    Color32::from_rgb(245, 240, 190),
                );
            }
        }
    }

    fn draw_shape_overlay_for_occurrence(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        occurrence: &ShapeOccurrenceId,
        shape: &Shape,
    ) {
        let selected = if let Some(selected) = &self.selected_occurrence {
            selected == occurrence
        } else {
            self.selected.contains(&shape.id)
        };
        self.draw_shape_overlay_with_selected(painter, canvas, shape, selected);
    }

    fn draw_shape_overlay_with_selected(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        shape: &Shape,
        selected: bool,
    ) {
        match &shape.kind {
            ShapeKind::Rectangle(rect) if selected => {
                self.draw_outline_points(
                    painter,
                    canvas,
                    &rect.corners(),
                    Stroke::new(2.0, Color32::WHITE),
                );
            }
            ShapeKind::Polygon(poly) if selected => {
                self.draw_outline_points(
                    painter,
                    canvas,
                    &poly.points,
                    Stroke::new(2.0, Color32::WHITE),
                );
            }
            ShapeKind::Path { points, width } if selected => {
                let screen_points: Vec<Pos2> = points
                    .iter()
                    .map(|point| self.world_to_screen(*point, canvas))
                    .collect();
                painter.add(egui::Shape::line(
                    screen_points,
                    Stroke::new((*width as f32 * self.zoom).max(3.0), Color32::WHITE),
                ));
            }
            ShapeKind::Via { center, size, .. } if selected => {
                let half = *size / 2;
                let rect = Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                );
                self.draw_outline_points(
                    painter,
                    canvas,
                    &rect.corners(),
                    Stroke::new(2.0, Color32::WHITE),
                );
            }
            ShapeKind::Label { position, text } => {
                painter.text(
                    self.world_to_screen(*position, canvas),
                    Align2::LEFT_CENTER,
                    text,
                    FontId::monospace(13.0),
                    layer_color32(self.document.layer_color(shape.layer), 1.0),
                );
            }
            ShapeKind::Measurement { a, b, label } => {
                let a = self.world_to_screen(*a, canvas);
                let b = self.world_to_screen(*b, canvas);
                painter.line_segment([a, b], Stroke::new(1.5, Color32::from_rgb(245, 240, 190)));
                painter.text(
                    a.lerp(b, 0.5),
                    Align2::CENTER_BOTTOM,
                    label,
                    FontId::monospace(12.0),
                    Color32::from_rgb(245, 240, 190),
                );
            }
            _ => {}
        }
    }

    fn draw_visible_cpu_shapes(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        viewport: Rect,
        visible_occurrences: &[ShapeOccurrenceId],
    ) {
        if self.document.has_hierarchy_instances() {
            for shape in self.document.visible_flattened_shapes() {
                if shape.bounds.intersects(viewport) {
                    self.draw_shape_for_occurrence(
                        painter,
                        canvas,
                        &shape.id,
                        &shape.transformed_shape(),
                    );
                }
            }
        } else {
            for occurrence in visible_occurrences {
                if let Some(shape) = self.document.shapes.get(&occurrence.source_shape_id()) {
                    self.draw_shape_for_occurrence(painter, canvas, occurrence, shape);
                }
            }
        }
    }

    fn draw_visible_overlays(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        viewport: Rect,
        visible_occurrences: &[ShapeOccurrenceId],
    ) {
        if self.document.has_hierarchy_instances() {
            for shape in self.document.visible_flattened_shapes() {
                if shape.bounds.intersects(viewport) {
                    self.draw_shape_overlay_for_occurrence(
                        painter,
                        canvas,
                        &shape.id,
                        &shape.transformed_shape(),
                    );
                }
            }
        } else {
            for occurrence in visible_occurrences {
                if let Some(shape) = self.document.shapes.get(&occurrence.source_shape_id()) {
                    self.draw_shape_overlay_for_occurrence(painter, canvas, occurrence, shape);
                }
            }
        }
    }

    fn draw_selected_net_highlight(&self, painter: &Painter, canvas: EguiRect, viewport: Rect) {
        let Some(component) = self.selected_net_component() else {
            return;
        };
        if component.shapes.len() <= 1 {
            return;
        }
        let highlighted = component.shapes.iter().cloned().collect::<BTreeSet<_>>();
        let stroke = Stroke::new(3.0, Color32::from_rgb(112, 236, 214));
        for flattened in self.document.visible_flattened_shapes() {
            if !flattened.bounds.intersects(viewport) || !highlighted.contains(&flattened.id) {
                continue;
            }
            let shape = flattened.transformed_shape();
            self.draw_net_highlight_shape(painter, canvas, &shape, stroke);
        }
    }

    fn draw_net_highlight_shape(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        shape: &Shape,
        stroke: Stroke,
    ) {
        match &shape.kind {
            ShapeKind::Rectangle(rect) => {
                self.draw_outline_points(painter, canvas, &rect.corners(), stroke);
            }
            ShapeKind::Polygon(poly) => {
                self.draw_outline_points(painter, canvas, &poly.points, stroke);
            }
            ShapeKind::Path { points, width } => {
                let screen_points: Vec<Pos2> = points
                    .iter()
                    .map(|point| self.world_to_screen(*point, canvas))
                    .collect();
                painter.add(egui::Shape::line(
                    screen_points,
                    Stroke::new((*width as f32 * self.zoom).max(5.0), stroke.color),
                ));
            }
            ShapeKind::Via { center, size, .. } => {
                let half = *size / 2;
                let rect = Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                );
                self.draw_outline_points(painter, canvas, &rect.corners(), stroke);
            }
            ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {}
        }
    }

    fn draw_selected_vertex_handles(&self, painter: &Painter, canvas: EguiRect) {
        let Some(id) = self.selected_top_level_shape() else {
            return;
        };
        let Some(shape) = self.document.shapes.get(&id) else {
            return;
        };
        for (a, b) in editable_edges(&shape.kind) {
            let midpoint = Point::new((a.x + b.x) / 2, (a.y + b.y) / 2);
            let center = self.world_to_screen(midpoint, canvas);
            painter.circle_filled(center, 3.5, Color32::from_rgb(242, 246, 236));
            painter.circle_stroke(center, 4.5, Stroke::new(1.0, Color32::from_rgb(20, 24, 26)));
        }
        for point in editable_vertex_points(&shape.kind) {
            let center = self.world_to_screen(point, canvas);
            let rect = EguiRect::from_center_size(center, vec2(8.0, 8.0));
            painter.rect_filled(rect, 2.0, Color32::from_rgb(242, 246, 236));
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, Color32::from_rgb(20, 24, 26)),
                StrokeKind::Outside,
            );
        }
    }

    fn draw_polygon_points(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        points: &[Point],
        fill: Color32,
        stroke: Stroke,
    ) {
        if points.len() < 3 {
            return;
        }
        let screen_points: Vec<Pos2> = points
            .iter()
            .map(|point| self.world_to_screen(*point, canvas))
            .collect();
        painter.add(egui::Shape::convex_polygon(screen_points, fill, stroke));
    }

    fn draw_outline_points(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        points: &[Point],
        stroke: Stroke,
    ) {
        if points.len() < 2 {
            return;
        }
        let mut screen_points: Vec<Pos2> = points
            .iter()
            .map(|point| self.world_to_screen(*point, canvas))
            .collect();
        screen_points.push(screen_points[0]);
        painter.add(egui::Shape::line(screen_points, stroke));
    }

    fn draw_violations(&self, painter: &Painter, canvas: EguiRect) {
        for violation in &self.violations {
            let state = self.marker_state(violation);
            if state.hidden || state.waived {
                continue;
            }
            let min = self.world_to_screen(violation.bounds.min, canvas);
            let max = self.world_to_screen(violation.bounds.max, canvas);
            let rect = EguiRect::from_two_pos(min, max);
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0, Color32::from_rgb(255, 70, 70)),
                StrokeKind::Outside,
            );
        }
    }

    fn draw_edit_preview(&self, ui: &egui::Ui, painter: &Painter, canvas: EguiRect) {
        let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        let current = self.snap_point(self.screen_to_world(pointer, canvas));
        match self.tool {
            Tool::Rect => {
                if let Some(start) = self.drawing_start {
                    let rect = Rect::new(start, current);
                    self.draw_polygon_points(
                        painter,
                        canvas,
                        &rect.corners(),
                        Color32::from_rgba_premultiplied(120, 180, 255, 65),
                        Stroke::new(1.5, Color32::from_rgb(190, 220, 255)),
                    );
                }
            }
            Tool::Polygon | Tool::Path => {
                if !self.drawing_points.is_empty() {
                    let mut points = self.drawing_points.clone();
                    points.push(current);
                    let screen_points: Vec<Pos2> = points
                        .iter()
                        .map(|point| self.world_to_screen(*point, canvas))
                        .collect();
                    painter.add(egui::Shape::line(
                        screen_points,
                        Stroke::new(1.5, Color32::from_rgb(200, 230, 255)),
                    ));
                }
            }
            Tool::Measure => {
                if let Some(start) = self.measure_start {
                    let a = self.world_to_screen(start, canvas);
                    let b = self.world_to_screen(current, canvas);
                    painter
                        .line_segment([a, b], Stroke::new(1.5, Color32::from_rgb(245, 240, 190)));
                }
            }
            Tool::Select | Tool::Via | Tool::Route => {}
        }
    }

    fn draw_route_points(&self, painter: &Painter, canvas: EguiRect) {
        for point in &self.route_points {
            let pos = self.world_to_screen(*point, canvas);
            painter.circle_filled(pos, 5.0, Color32::from_rgb(250, 210, 80));
            painter.circle_stroke(pos, 8.0, Stroke::new(1.0, Color32::from_rgb(250, 210, 80)));
        }
    }

    fn draw_remote_selections(&self, painter: &Painter, canvas: EguiRect, viewport: Rect) {
        if self.remote_selections.is_empty() {
            return;
        }
        for (user, selections) in &self.remote_selections {
            let color = remote_user_color(*user);
            let stroke = Stroke::new(2.0, color);
            let selected = selections.iter().cloned().collect::<BTreeSet<_>>();
            let mut label_position = None;
            for flattened in self.document.visible_flattened_shapes() {
                if !flattened.bounds.intersects(viewport) || !selected.contains(&flattened.id) {
                    continue;
                }
                let shape = flattened.transformed_shape();
                label_position.get_or_insert(flattened.bounds.min);
                self.draw_net_highlight_shape(painter, canvas, &shape, stroke);
            }
            if let Some(position) = label_position {
                painter.text(
                    self.world_to_screen(position, canvas) + vec2(0.0, -16.0),
                    Align2::LEFT_BOTTOM,
                    short_user(*user),
                    FontId::monospace(11.0),
                    color,
                );
            }
        }
    }

    fn draw_remote_cursors(&self, painter: &Painter, canvas: EguiRect) {
        for (user, point) in &self.remote_cursors {
            let pos = self.world_to_screen(*point, canvas);
            let color = remote_user_color(*user);
            painter.line_segment([pos, pos + vec2(12.0, 18.0)], Stroke::new(2.0, color));
            painter.text(
                pos + vec2(14.0, 18.0),
                Align2::LEFT_TOP,
                short_user(*user),
                FontId::monospace(11.0),
                color,
            );
        }
    }

    fn handle_canvas_input(&mut self, ui: &egui::Ui, response: &egui::Response, canvas: EguiRect) {
        let Some(pointer) = response.interact_pointer_pos() else {
            return;
        };
        let world = self.snap_point(self.screen_to_world(pointer, canvas));
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            self.drawing_start = None;
            self.drawing_points.clear();
            self.measure_start = None;
            self.route_points.clear();
            self.drag_shape = None;
            self.drag_instance = None;
            self.drag_vertex = None;
            self.drag_edge = None;
            self.last_drag_world = None;
        }
        if ui.input(|input| input.key_pressed(Key::Enter)) {
            self.finish_polyline();
        }

        match self.tool {
            Tool::Select => self.handle_select_input(ui, response, pointer, world),
            Tool::Rect => self.handle_rect_input(response, world),
            Tool::Polygon | Tool::Path => self.handle_polyline_input(response, world),
            Tool::Via => self.handle_via_input(response, world),
            Tool::Measure => self.handle_measure_input(response, world),
            Tool::Route => self.handle_route_input(response, world),
        }
    }

    fn handle_select_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        pointer: Pos2,
        world: Point,
    ) {
        let tolerance = (10.0 / self.zoom).max(self.snap_grid() as f32) as Coord;
        if response.double_clicked() {
            if let Some(edge) = self.hit_selected_edge(world, tolerance) {
                self.insert_vertex(edge.shape, edge.edge, world);
                return;
            }
        }
        if response.hovered()
            && ui.input(|input| {
                !input.modifiers.any()
                    && (input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace))
            })
        {
            if let Some(vertex) = self.hit_selected_vertex(world, tolerance) {
                self.delete_vertex(vertex.shape, vertex.vertex);
            } else {
                self.delete_selection();
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(vertex) = self.hit_selected_vertex(world, tolerance) {
                self.drag_vertex = Some(vertex);
                self.drag_shape = None;
                self.drag_instance = None;
                self.drag_edge = None;
                self.last_drag_world = Some(world);
                self.selected_occurrence = Some(ShapeOccurrenceId::top_level(vertex.shape));
                self.status = format!(
                    "editing vertex {} on shape #{}",
                    vertex.vertex + 1,
                    vertex.shape.0
                );
                return;
            }
            if let Some(edge) = self.hit_selected_draggable_edge(world, tolerance) {
                self.drag_edge = Some(edge);
                self.drag_vertex = None;
                self.drag_shape = None;
                self.drag_instance = None;
                self.last_drag_world = Some(world);
                self.selected_occurrence = Some(ShapeOccurrenceId::top_level(edge.shape));
                self.status = format!("editing edge {} on shape #{}", edge.edge + 1, edge.shape.0);
                return;
            }
            let picked = self
                .gpu_pick_at(pointer)
                .flatten()
                .or_else(|| self.index.hit_test_occurrence(world, tolerance));
            if let Some(occurrence) = picked {
                let id = occurrence.source_shape_id();
                if !self.selected.contains(&id) {
                    self.selected.clear();
                    self.selected.insert(id);
                }
                self.selected_occurrence = Some(occurrence.clone());
                if occurrence.is_top_level() && self.document.shapes.contains_key(&id) {
                    self.drag_shape = Some(id);
                    self.drag_instance = None;
                    self.drag_vertex = None;
                    self.drag_edge = None;
                    self.last_drag_world = Some(world);
                } else if let Some((parent, instance_id)) = self
                    .document
                    .instance_parent_for_path(&occurrence.instance_path)
                {
                    self.drag_shape = None;
                    self.drag_instance = Some(InstanceDrag {
                        parent,
                        id: instance_id,
                    });
                    self.drag_vertex = None;
                    self.drag_edge = None;
                    self.last_drag_world = Some(world);
                    self.status = format!("picked instance {}", occurrence_label(&occurrence));
                } else {
                    self.drag_shape = None;
                    self.drag_instance = None;
                    self.drag_vertex = None;
                    self.drag_edge = None;
                    self.last_drag_world = None;
                    self.status = format!("picked instance {}", occurrence_label(&occurrence));
                }
            } else {
                self.selected.clear();
                self.selected_occurrence = None;
                self.drag_vertex = None;
                self.drag_edge = None;
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Some(vertex) = self.drag_vertex {
                self.move_vertex(vertex.shape, vertex.vertex, world);
                self.last_drag_world = Some(world);
            } else if let (Some(edge), Some(last)) = (self.drag_edge, self.last_drag_world) {
                let delta = Vector::new(world.x - last.x, world.y - last.y);
                if delta != Vector::ZERO {
                    self.move_edge(edge.shape, edge.edge, delta);
                    self.last_drag_world = Some(world);
                }
            } else if let (Some(id), Some(last)) = (self.drag_shape, self.last_drag_world) {
                let delta = Vector::new(world.x - last.x, world.y - last.y);
                if delta != Vector::ZERO {
                    self.move_shape(id, delta);
                    self.last_drag_world = Some(world);
                }
            } else if let (Some(instance), Some(last)) = (self.drag_instance, self.last_drag_world)
            {
                let delta = Vector::new(world.x - last.x, world.y - last.y);
                if delta != Vector::ZERO {
                    self.move_instance(instance.parent, instance.id, delta);
                    self.last_drag_world = Some(world);
                }
            }
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            self.drag_shape = None;
            self.drag_instance = None;
            self.drag_vertex = None;
            self.drag_edge = None;
            self.last_drag_world = None;
        }
    }

    fn handle_rect_input(&mut self, response: &egui::Response, world: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            self.drawing_start = Some(world);
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            if let Some(start) = self.drawing_start.take() {
                let rect = Rect::new(start, world);
                let minimum_size = self.minimum_draw_size();
                if rect.width().abs() >= minimum_size && rect.height().abs() >= minimum_size {
                    self.add_shape(self.active_layer, ShapeKind::Rectangle(rect));
                }
            }
        }
    }

    fn handle_polyline_input(&mut self, response: &egui::Response, world: Point) {
        if response.clicked_by(PointerButton::Primary) {
            if self.drawing_points.last().copied() != Some(world) {
                self.drawing_points.push(world);
            }
        }
        if response.double_clicked() || response.clicked_by(PointerButton::Secondary) {
            self.finish_polyline();
        }
    }

    fn finish_polyline(&mut self) {
        match self.tool {
            Tool::Polygon if self.drawing_points.len() >= 3 => {
                let points = std::mem::take(&mut self.drawing_points);
                self.add_shape(
                    self.active_layer,
                    ShapeKind::Polygon(geometry_core::Polygon::new(points)),
                );
            }
            Tool::Path if self.drawing_points.len() >= 2 => {
                let points = std::mem::take(&mut self.drawing_points);
                self.add_shape(self.active_layer, ShapeKind::Path { points, width: 180 });
            }
            _ => {}
        }
    }

    fn handle_via_input(&mut self, response: &egui::Response, world: Point) {
        if response.clicked_by(PointerButton::Primary) {
            let via_layer = self
                .document
                .layer_by_process(ProcessLayer::Via1)
                .unwrap_or(self.active_layer);
            let lower = self
                .document
                .layer_by_process(ProcessLayer::Metal1)
                .unwrap_or(self.active_layer);
            let upper = self
                .document
                .layer_by_process(ProcessLayer::Metal2)
                .unwrap_or(self.active_layer);
            self.add_shape(
                via_layer,
                ShapeKind::Via {
                    center: world,
                    size: 180,
                    lower,
                    upper,
                },
            );
        }
    }

    fn handle_measure_input(&mut self, response: &egui::Response, world: Point) {
        if response.clicked_by(PointerButton::Primary) {
            if let Some(start) = self.measure_start.take() {
                let distance = start.distance_to(world);
                self.add_shape(
                    self.document
                        .layer_by_process(ProcessLayer::Annotation)
                        .unwrap_or(self.active_layer),
                    ShapeKind::Measurement {
                        a: start,
                        b: world,
                        label: self.format_length(distance),
                    },
                );
            } else {
                self.measure_start = Some(world);
            }
        }
    }

    fn handle_route_input(&mut self, response: &egui::Response, world: Point) {
        if response.clicked_by(PointerButton::Primary) {
            self.route_points.push(world);
            if self.route_points.len() >= 2 {
                self.route_between_points();
            }
        }
    }

    fn viewport_world(&self, canvas: EguiRect) -> Rect {
        Rect::new(
            self.screen_to_world(canvas.left_top(), canvas),
            self.screen_to_world(canvas.right_bottom(), canvas),
        )
    }

    fn world_to_screen(&self, point: Point, canvas: EguiRect) -> Pos2 {
        canvas.center() + self.pan + vec2(point.x as f32 * self.zoom, -(point.y as f32) * self.zoom)
    }

    fn screen_to_world(&self, point: Pos2, canvas: EguiRect) -> Point {
        let local = (point - canvas.center() - self.pan) / self.zoom;
        Point::new(local.x.round() as Coord, (-local.y).round() as Coord)
    }

    fn gpu_pick_at(&self, pointer: Pos2) -> Option<Option<ShapeOccurrenceId>> {
        let state = self.gpu_pick_state.lock().ok()?;
        let result = state.latest.as_ref()?;
        let dx = result.pointer_screen[0] - pointer.x;
        let dy = result.pointer_screen[1] - pointer.y;
        if dx.hypot(dy) <= 3.0 {
            Some(result.shape_id.clone())
        } else {
            None
        }
    }

    fn gpu_pick_label(&self) -> String {
        let Ok(state) = self.gpu_pick_state.lock() else {
            return "locked".to_string();
        };
        let Some(result) = &state.latest else {
            return "waiting".to_string();
        };
        match result.shape_id {
            Some(ref id) => occurrence_label(id),
            None => "miss".to_string(),
        }
    }

    fn grid_step_for_zoom(&self) -> Coord {
        let target_px = self.settings.min_grid_pixels.clamp(8.0, 80.0);
        let raw = (target_px / self.zoom).max(self.snap_grid() as f32) as Coord;
        let base = self.snap_grid();
        let mut step = base;
        while step < raw {
            step *= 2;
        }
        step
    }
}

impl eframe::App for FabricadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.poll_collaboration();
        self.maybe_autosave();
        self.advance_equipment_simulator();
        self.handle_shortcuts(ctx);
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        self.navigation_panel(ctx);
        if self.view_mode.has_inspector_panel() {
            self.inspector_panel(ctx);
        }
        if self.view_mode.has_secondary_panel() {
            self.layers_panel(ctx);
        }
        self.options_window(ctx);
        self.diagnostics_window(ctx);
        self.mes_panel(ctx);
        egui::CentralPanel::default().show(ctx, |ui| match self.view_mode {
            ViewMode::Layout2d => self.canvas(ui),
            ViewMode::Layout3d => self.canvas_3d(ui),
            ViewMode::FabControl => self.fab_control_room(ui),
            ViewMode::Metrology => self.metrology_canvas(ui),
            ViewMode::Yield => self.yield_dashboard(ui),
            ViewMode::MaskPrep => self.mask_panel.ui(
                ui,
                &self.document,
                &self.mes,
                &mut self.selected_mes_lot,
                &mut self.status,
            ),
            ViewMode::SpcFdc => {
                self.spc_fdc_panel
                    .ui(ui, &self.yield_analysis, &self.equipment_sim);
            }
            ViewMode::ProcessControl => {
                self.process_control_panel
                    .ui(ui, &self.yield_analysis, &mut self.status);
            }
            ViewMode::Traceability => self.genealogy_panel.ui(ui),
            ViewMode::Experiment => self.experiment_panel.dashboard_ui(ui, &mut self.status),
        });
        ctx.request_repaint();
    }
}

#[derive(Clone, Debug)]
struct GpuPickResult {
    serial: u64,
    pointer_screen: [f32; 2],
    shape_id: Option<ShapeOccurrenceId>,
}

#[derive(Default)]
struct GpuPickState {
    latest: Option<GpuPickResult>,
}

type SharedGpuPickState = Arc<Mutex<GpuPickState>>;

type SharedGpuUploadState = Arc<Mutex<GpuUploadStats>>;

struct Viewport3dGpuCallback {
    batch: renderer::RenderBatch3d,
    uniforms: Viewport3dUniforms,
    viewport_size: [f32; 2],
    target_format: egui_wgpu::wgpu::TextureFormat,
}

struct LayoutGpuCallback {
    batch: renderer::RenderBatch,
    pick_batch: Option<renderer::PickBatch>,
    pick_request: Option<GpuPickRequest>,
    pick_state: SharedGpuPickState,
    upload_state: SharedGpuUploadState,
    uniforms: ViewUniforms,
    target_format: egui_wgpu::wgpu::TextureFormat,
}

impl egui_wgpu::CallbackTrait for Viewport3dGpuCallback {
    fn prepare(
        &self,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        if callback_resources.get::<Viewport3dRenderer>().is_none() {
            callback_resources.insert(Viewport3dRenderer::new(device, self.target_format));
        }
        if let Some(resources) = callback_resources.get_mut::<Viewport3dRenderer>() {
            resources.upload(device, queue, &self.batch, self.uniforms);
            let target_size =
                viewport_3d_target_size(self.viewport_size, screen_descriptor.pixels_per_point);
            resources.render_to_texture(
                device,
                egui_encoder,
                target_size,
                egui_wgpu::wgpu::Color {
                    r: 8.0 / 255.0,
                    g: 11.0 / 255.0,
                    b: 14.0 / 255.0,
                    a: 1.0,
                },
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(resources) = callback_resources.get::<Viewport3dRenderer>() {
            resources.paint(render_pass);
        }
    }
}

impl egui_wgpu::CallbackTrait for LayoutGpuCallback {
    fn prepare(
        &self,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        if callback_resources.get::<LayoutGpuRenderer>().is_none() {
            if let Some(resources) = LayoutGpuRenderer::new(device, self.target_format) {
                callback_resources.insert(resources);
            }
        }
        if let Some(resources) = callback_resources.get_mut::<LayoutGpuRenderer>() {
            let layout_upload = resources.upload(device, queue, &self.batch, self.uniforms);
            let mut pick_upload = None;
            let mut stats_published = false;
            if let (Some(pick_batch), Some(pick_request)) = (&self.pick_batch, self.pick_request) {
                pick_upload = Some(resources.upload_pick(device, queue, pick_batch, self.uniforms));
                publish_upload_stats(
                    Arc::clone(&self.upload_state),
                    layout_upload,
                    pick_upload,
                    resources.resident_bytes(),
                );
                stats_published = true;
                let pick_state = Arc::clone(&self.pick_state);
                if let Some(command_buffer) = resources.prepare_pick_readback(
                    device,
                    screen_descriptor.pixels_per_point,
                    pick_batch,
                    pick_request,
                    move |request, shape_id| publish_pick_result(pick_state, request, shape_id),
                ) {
                    return vec![command_buffer];
                }
            }
            if !stats_published {
                publish_upload_stats(
                    Arc::clone(&self.upload_state),
                    layout_upload,
                    pick_upload,
                    resources.resident_bytes(),
                );
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(resources) = callback_resources.get::<LayoutGpuRenderer>() {
            resources.paint(render_pass);
        }
    }
}

fn publish_pick_result(
    pick_state: SharedGpuPickState,
    request: GpuPickRequest,
    shape_id: Option<ShapeOccurrenceId>,
) {
    if let Ok(mut state) = pick_state.lock() {
        let should_replace = state
            .latest
            .as_ref()
            .is_none_or(|latest| request.serial >= latest.serial);
        if should_replace {
            state.latest = Some(GpuPickResult {
                serial: request.serial,
                pointer_screen: request.pointer_screen,
                shape_id,
            });
        }
    }
}

fn publish_upload_stats(
    upload_state: SharedGpuUploadState,
    layout: BufferUploadResult,
    pick: Option<BufferUploadResult>,
    resident_bytes: usize,
) {
    if let Ok(mut stats) = upload_state.lock() {
        stats.last_upload_bytes =
            layout.bytes_uploaded + pick.as_ref().map_or(0, |upload| upload.bytes_uploaded);
        stats.resident_bytes = resident_bytes;
        if layout.uploaded {
            stats.layout_uploads += 1;
        } else if layout.skipped {
            stats.layout_skips += 1;
        }
        if let Some(pick) = pick {
            if pick.uploaded {
                stats.pick_uploads += 1;
            } else if pick.skipped {
                stats.pick_skips += 1;
            }
        }
    }
}

fn build_layout_index(document: &Document) -> LayoutIndex {
    if document.has_hierarchy_instances() {
        LayoutIndex::rebuild_hierarchical(document)
    } else {
        LayoutIndex::rebuild(document)
    }
}

fn rule_deck_for_document(document: &Document, technology: &TechnologyFile) -> RuleDeck {
    RuleDeck::from_technology(document, technology).unwrap_or_else(|_| RuleDeck::demo(document))
}

fn connectivity_report_for_document(
    document: &Document,
    technology: &TechnologyFile,
) -> ConnectivityReport {
    let object_count = document_object_count(document);
    if object_count > MAX_CONNECTIVITY_OBJECTS {
        return ConnectivityReport::skipped(format!(
            "net extraction skipped for {object_count} objects"
        ));
    }
    extract_connectivity(document, technology)
        .unwrap_or_else(|err| ConnectivityReport::skipped(format!("net extraction failed: {err}")))
}

fn document_object_count(document: &Document) -> usize {
    document.shapes.len()
        + document
            .cells
            .values()
            .map(|cell| 1 + cell.shapes.len() + cell.instances.len())
            .sum::<usize>()
}

fn yield_metric_ui(ui: &mut egui::Ui, label: &str, value: String, detail: String) {
    ui_chrome::metric_tile(ui, label, value, &detail);
}

fn draw_yield_wafer_map(ui: &mut egui::Ui, outcomes: &[DieOutcome]) {
    let size = ui.available_width().clamp(220.0, 360.0);
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().faint_bg_color);

    if outcomes.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no wafer data",
            FontId::proportional(14.0),
            ui.visuals().weak_text_color(),
        );
        return;
    }

    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.46;
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.0, Color32::from_rgb(132, 142, 154)),
    );
    painter.circle_stroke(
        center,
        radius * 0.72,
        Stroke::new(0.5, Color32::from_gray(78)),
    );

    let max_coordinate = outcomes
        .iter()
        .map(|outcome| outcome.die.column.abs().max(outcome.die.row.abs()))
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let die_size = ((radius * 2.0) / (max_coordinate * 2.0 + 1.0) * 0.72).clamp(5.0, 16.0);
    let scale = radius * 0.9 / max_coordinate;
    let mut hovered = None;

    for outcome in outcomes {
        let pos = Pos2::new(
            center.x + outcome.die.column as f32 * scale,
            center.y - outcome.die.row as f32 * scale,
        );
        let die_rect = EguiRect::from_center_size(pos, vec2(die_size, die_size));
        let color = outcome_color(outcome);
        painter.rect_filled(die_rect, 1.5, color);
        if !outcome.passed {
            painter.rect_stroke(
                die_rect,
                1.5,
                Stroke::new(0.75, Color32::BLACK),
                StrokeKind::Outside,
            );
        }
        if response
            .hover_pos()
            .is_some_and(|pointer| die_rect.expand(2.0).contains(pointer))
        {
            hovered = Some(outcome);
        }
    }

    if let Some(outcome) = hovered {
        let mode = outcome
            .failure_modes
            .first()
            .map(|mode| mode.label())
            .unwrap_or(if outcome.passed { "pass" } else { "fail" });
        response.on_hover_text(format!(
            "die ({}, {})\n{}\n{} failed / {} tests",
            outcome.die.column, outcome.die.row, mode, outcome.failed_tests, outcome.test_count
        ));
    }
}

fn outcome_color(outcome: &DieOutcome) -> Color32 {
    if outcome.passed {
        return Color32::from_rgb(72, 164, 108);
    }
    outcome
        .failure_modes
        .first()
        .copied()
        .map(failure_mode_color)
        .unwrap_or(Color32::from_rgb(202, 88, 88))
}

fn failure_mode_color(mode: FailureMode) -> Color32 {
    match mode {
        FailureMode::LowFrequency => Color32::from_rgb(222, 154, 58),
        FailureMode::HighLeakage => Color32::from_rgb(214, 82, 116),
        FailureMode::ContactResistance => Color32::from_rgb(168, 98, 210),
        FailureMode::OpenCircuit => Color32::from_rgb(207, 94, 72),
        FailureMode::ShortCircuit => Color32::from_rgb(190, 72, 72),
        FailureMode::ParametricDrift => Color32::from_rgb(76, 139, 205),
        FailureMode::EdgeDefect => Color32::from_rgb(218, 130, 70),
    }
}

fn failure_breakdown_ui(ui: &mut egui::Ui, summary: &YieldSummary) {
    if summary.failure_counts.is_empty() {
        ui.label("No failing dies in selected scope");
        return;
    }
    let mut rows = summary
        .failure_counts
        .iter()
        .map(|(mode, count)| (*mode, *count))
        .collect::<Vec<_>>();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let denominator = summary.failing_dies.max(1) as f32;
    for (mode, count) in rows {
        ui.horizontal(|ui| {
            let (swatch, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter()
                .rect_filled(swatch, 2.0, failure_mode_color(mode));
            ui.label(mode.label());
            ui.add(
                egui::ProgressBar::new(count as f32 / denominator)
                    .desired_width(120.0)
                    .text(format!("{} dies", count)),
            );
        });
    }
}

fn lot_comparison_ui(ui: &mut egui::Ui, comparisons: &[LotComparison]) {
    if comparisons.is_empty() {
        ui.label("No comparison lots loaded");
        return;
    }
    for comparison in comparisons {
        ui.label(format!(
            "{} / {} -> {} / {}",
            comparison.baseline_lot_id,
            comparison.baseline_recipe_id,
            comparison.candidate_lot_id,
            comparison.candidate_recipe_id
        ));
        ui.label(format!(
            "{} to {} ({})",
            format_percent(comparison.baseline_yield),
            format_percent(comparison.candidate_yield),
            format_signed_percent(comparison.yield_delta)
        ));
        ui.label(&comparison.root_cause_hint);
        egui::Grid::new(("lot_comparison_modes", &comparison.baseline_lot_id))
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Mode");
                ui.strong("Base");
                ui.strong("Candidate");
                ui.strong("Delta");
                ui.end_row();
                for delta in comparison.failure_mode_deltas.iter().take(5) {
                    ui.label(delta.mode.label());
                    ui.label(format_percent(delta.baseline_fraction));
                    ui.label(format_percent(delta.candidate_fraction));
                    ui.label(format_signed_percent(delta.delta_fraction));
                    ui.end_row();
                }
            });
    }
}

fn wafer_measurements_ui(ui: &mut egui::Ui, measurements: &[ProcessMeasurement]) {
    if measurements.is_empty() {
        ui.label("No measurements for selected wafer");
        return;
    }
    egui::Grid::new("yield_wafer_measurements")
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Measurement");
            ui.strong("Value");
            ui.strong("Target");
            ui.strong("Step");
            ui.end_row();
            for measurement in measurements {
                ui.label(measurement.name.replace('_', " "));
                ui.label(format_measurement(measurement.value, &measurement.unit));
                ui.label(
                    measurement
                        .target
                        .map(|target| format_measurement(target, &measurement.unit))
                        .unwrap_or_else(|| "-".to_string()),
                );
                ui.label(&measurement.step_id);
                ui.end_row();
            }
        });
}

fn correlation_table_ui(ui: &mut egui::Ui, correlations: &[CorrelationRecord]) {
    if correlations.is_empty() {
        ui.label("No wafer-level correlations");
        return;
    }
    egui::ScrollArea::horizontal()
        .id_salt("yield_correlation_horizontal")
        .show(ui, |ui| {
            egui::Grid::new("yield_correlation_table")
                .striped(true)
                .min_col_width(88.0)
                .show(ui, |ui| {
                    ui.strong("Measurement");
                    ui.strong("Corr");
                    ui.strong("High fail mean");
                    ui.strong("Low fail mean");
                    ui.strong("Mode");
                    ui.strong("Hint");
                    ui.end_row();
                    for record in correlations {
                        ui.label(record.measurement_name.replace('_', " "));
                        ui.label(format!("{:+.2}", record.correlation_to_failure_rate));
                        ui.label(format_measurement(
                            record.mean_high_failure_value,
                            &record.unit,
                        ));
                        ui.label(format_measurement(
                            record.mean_low_failure_value,
                            &record.unit,
                        ));
                        ui.label(
                            record
                                .likely_failure_mode
                                .map(FailureMode::label)
                                .unwrap_or("-"),
                        );
                        ui.label(&record.root_cause_hint);
                        ui.end_row();
                    }
                });
        });
}

fn lot_recipe_label(analysis: &YieldAnalysis, lot_id: &str) -> String {
    let Some(lot) = analysis.lots.iter().find(|lot| lot.id == lot_id) else {
        return "-".to_string();
    };
    let version = analysis
        .recipes
        .iter()
        .find(|recipe| recipe.id == lot.recipe_id)
        .map(|recipe| recipe.version)
        .unwrap_or(0);
    if version == 0 {
        lot.recipe_id.clone()
    } else {
        format!("{} v{}", lot.recipe_id, version)
    }
}

fn lot_route_label(analysis: &YieldAnalysis, lot_id: &str) -> String {
    analysis
        .lots
        .iter()
        .find(|lot| lot.id == lot_id)
        .map(|lot| format!("{} / {}", lot.product, lot.route_id))
        .unwrap_or_else(|| "-".to_string())
}

fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn format_signed_percent(value: f64) -> String {
    format!("{:+.1} pp", value * 100.0)
}

fn format_measurement(value: f64, unit: &str) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0} {unit}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.2} {unit}")
    }
}

fn component_route_metadata(component: &NetComponent) -> RouteNetMetadata {
    RouteNetMetadata {
        net: component.net_id,
        name: component.net_name.clone(),
    }
}

fn occurrence_label(id: &ShapeOccurrenceId) -> String {
    if id.instance_path.is_empty() {
        format!("#{}", id.source_shape_id().0)
    } else if let Some(array) = id.array_path.last() {
        format!(
            "#{}@{}[{},{}]",
            id.source_shape_id().0,
            id.instance_path.len(),
            array.column,
            array.row
        )
    } else {
        format!("#{}@{}", id.source_shape_id().0, id.instance_path.len())
    }
}

fn nice_scale_length_dbu(target_dbu: f32) -> Coord {
    if !target_dbu.is_finite() || target_dbu <= 1.0 {
        return 1;
    }
    let exponent = 10f32.powf(target_dbu.log10().floor());
    let normalized = target_dbu / exponent;
    let multiplier = if normalized < 1.5 {
        1.0
    } else if normalized < 3.5 {
        2.0
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };
    (multiplier * exponent).round().max(1.0) as Coord
}

#[cfg(test)]
fn format_scale_label(length_dbu: Coord, dbu_per_micron: Coord) -> String {
    format_physical_length(length_dbu as f64, dbu_per_micron)
}

#[cfg(test)]
fn format_physical_length(length_dbu: f64, dbu_per_micron: Coord) -> String {
    format_physical_length_with_options(length_dbu, dbu_per_micron, UnitDisplay::Auto, 2)
}

fn format_physical_length_with_options(
    length_dbu: f64,
    dbu_per_micron: Coord,
    units: UnitDisplay,
    precision: usize,
) -> String {
    let microns = length_dbu / dbu_per_micron.max(1) as f64;
    match units {
        UnitDisplay::Auto if microns.abs() < 1.0 => {
            let nanometers = microns * 1_000.0;
            let precision = if nanometers.abs() >= 100.0 {
                0
            } else if nanometers.abs() >= 10.0 {
                precision.min(1)
            } else {
                precision.min(3)
            };
            format!("{} nm", format_decimal(nanometers, precision))
        }
        UnitDisplay::Auto => {
            let precision = if is_effectively_integer(microns) {
                0
            } else if microns.abs() < 10.0 {
                precision
            } else {
                precision.min(1)
            };
            format!("{} um", format_decimal(microns, precision))
        }
        UnitDisplay::Nanometers => {
            format!("{} nm", format_decimal(microns * 1_000.0, precision))
        }
        UnitDisplay::Microns => format!("{} um", format_decimal(microns, precision)),
        UnitDisplay::Dbu => format!("{} dbu", format_decimal(length_dbu, precision.min(3))),
    }
}

fn format_decimal(value: f64, precision: usize) -> String {
    let precision = precision.min(6);
    if precision == 0 || is_effectively_integer(value) {
        return format!("{value:.0}");
    }
    format!("{value:.precision$}")
}

fn is_effectively_integer(value: f64) -> bool {
    (value - value.round()).abs() < 1.0e-9
}

fn text_edit_commit_requested(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.lost_focus()
        || (response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)))
}

fn editable_vertex_points(kind: &ShapeKind) -> Vec<Point> {
    match kind {
        ShapeKind::Rectangle(rect) => rect.corners().to_vec(),
        ShapeKind::Polygon(poly) => poly.points.clone(),
        ShapeKind::Path { points, .. } => points.clone(),
        ShapeKind::Via { .. } | ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {
            Vec::new()
        }
    }
}

fn editable_edges(kind: &ShapeKind) -> Vec<(Point, Point)> {
    match kind {
        ShapeKind::Rectangle(rect) => closed_edges(&rect.corners()),
        ShapeKind::Polygon(poly) => closed_edges(&poly.points),
        ShapeKind::Path { points, .. } => {
            points.windows(2).map(|edge| (edge[0], edge[1])).collect()
        }
        ShapeKind::Via { .. } | ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {
            Vec::new()
        }
    }
}

fn closed_edges(points: &[Point]) -> Vec<(Point, Point)> {
    if points.len() < 2 {
        return Vec::new();
    }
    (0..points.len())
        .map(|index| (points[index], points[(index + 1) % points.len()]))
        .collect()
}

fn shape_edge_is_draggable(kind: &ShapeKind, edge: usize) -> bool {
    match kind {
        ShapeKind::Rectangle(rect) => edge < rect.corners().len(),
        ShapeKind::Polygon(poly) => {
            let Some((a, b)) = closed_edges(&poly.points).get(edge).copied() else {
                return false;
            };
            a.x == b.x || a.y == b.y
        }
        ShapeKind::Path { .. }
        | ShapeKind::Via { .. }
        | ShapeKind::Label { .. }
        | ShapeKind::Measurement { .. } => false,
    }
}

fn shape_with_moved_vertex(shape: &Shape, vertex: usize, position: Point) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let corners = rect.corners();
            let opposite = *corners.get((vertex + 2) % corners.len())?;
            *rect = Rect::new(position, opposite);
        }
        ShapeKind::Polygon(poly) => {
            let point = poly.points.get_mut(vertex)?;
            *point = position;
        }
        ShapeKind::Path { points, .. } => {
            let point = points.get_mut(vertex)?;
            *point = position;
        }
        ShapeKind::Via { .. } | ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {
            return None;
        }
    }
    Some(shape)
}

fn shape_with_inserted_vertex(shape: &Shape, edge: usize, position: Point) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let mut points = rect.corners().to_vec();
            if edge >= points.len() {
                return None;
            }
            points.insert(edge + 1, position);
            shape.kind = ShapeKind::Polygon(geometry_core::Polygon::new(points));
        }
        ShapeKind::Polygon(poly) => {
            if poly.points.len() < 2 || edge >= poly.points.len() {
                return None;
            }
            poly.points.insert(edge + 1, position);
        }
        ShapeKind::Path { points, .. } => {
            if points.len() < 2 || edge + 1 >= points.len() {
                return None;
            }
            points.insert(edge + 1, position);
        }
        ShapeKind::Via { .. } | ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {
            return None;
        }
    }
    Some(shape)
}

fn shape_with_deleted_vertex(shape: &Shape, vertex: usize) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Polygon(poly) => {
            if poly.points.len() <= 3 || vertex >= poly.points.len() {
                return None;
            }
            poly.points.remove(vertex);
        }
        ShapeKind::Path { points, .. } => {
            if points.len() <= 2 || vertex >= points.len() {
                return None;
            }
            points.remove(vertex);
        }
        ShapeKind::Rectangle(_)
        | ShapeKind::Via { .. }
        | ShapeKind::Label { .. }
        | ShapeKind::Measurement { .. } => return None,
    }
    Some(shape)
}

fn shape_with_moved_edge(shape: &Shape, edge: usize, delta: Vector) -> Option<Shape> {
    let mut shape = shape.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            if edge >= 4 {
                return None;
            }
            let mut min = rect.min;
            let mut max = rect.max;
            match edge {
                0 => min.y += delta.dy,
                1 => max.x += delta.dx,
                2 => max.y += delta.dy,
                3 => min.x += delta.dx,
                _ => unreachable!(),
            }
            *rect = Rect::new(min, max);
        }
        ShapeKind::Polygon(poly) => {
            if poly.points.len() < 3 || edge >= poly.points.len() {
                return None;
            }
            let next = (edge + 1) % poly.points.len();
            let a = poly.points[edge];
            let b = poly.points[next];
            let constrained = rectilinear_edge_delta(a, b, delta)?;
            poly.points[edge] = poly.points[edge].translated(constrained);
            poly.points[next] = poly.points[next].translated(constrained);
        }
        ShapeKind::Path { .. }
        | ShapeKind::Via { .. }
        | ShapeKind::Label { .. }
        | ShapeKind::Measurement { .. } => return None,
    }
    Some(shape)
}

fn rectilinear_edge_delta(a: Point, b: Point, delta: Vector) -> Option<Vector> {
    if a.x == b.x {
        Some(Vector::new(delta.dx, 0))
    } else if a.y == b.y {
        Some(Vector::new(0, delta.dy))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug)]
enum ShapeTransform {
    RotateCw90,
    MirrorX,
    MirrorY,
}

fn transform_shape_kind(kind: &ShapeKind, center: Point, transform: ShapeTransform) -> ShapeKind {
    match kind {
        ShapeKind::Rectangle(rect) => {
            let points: Vec<_> = rect
                .corners()
                .into_iter()
                .map(|point| transform_point(point, center, transform))
                .collect();
            ShapeKind::Rectangle(Rect::from_points(&points).unwrap_or_default())
        }
        ShapeKind::Polygon(poly) => ShapeKind::Polygon(geometry_core::Polygon::new(
            poly.points
                .iter()
                .map(|point| transform_point(*point, center, transform))
                .collect(),
        )),
        ShapeKind::Path { points, width } => ShapeKind::Path {
            points: points
                .iter()
                .map(|point| transform_point(*point, center, transform))
                .collect(),
            width: *width,
        },
        ShapeKind::Via {
            center: via_center,
            size,
            lower,
            upper,
        } => ShapeKind::Via {
            center: transform_point(*via_center, center, transform),
            size: *size,
            lower: *lower,
            upper: *upper,
        },
        ShapeKind::Label { position, text } => ShapeKind::Label {
            position: transform_point(*position, center, transform),
            text: text.clone(),
        },
        ShapeKind::Measurement { a, b, label } => ShapeKind::Measurement {
            a: transform_point(*a, center, transform),
            b: transform_point(*b, center, transform),
            label: label.clone(),
        },
    }
}

fn transform_point(point: Point, center: Point, transform: ShapeTransform) -> Point {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    match transform {
        ShapeTransform::RotateCw90 => Point::new(center.x + dy, center.y - dx),
        ShapeTransform::MirrorX => Point::new(center.x - dx, point.y),
        ShapeTransform::MirrorY => Point::new(point.x, center.y - dy),
    }
}

fn shape_bounds<'a>(shapes: impl Iterator<Item = &'a Shape>) -> Option<Rect> {
    shapes
        .map(|shape| shape.kind.bounds())
        .reduce(|bounds, rect| bounds.union(rect))
}

fn shape_kind_property_ui(ui: &mut egui::Ui, kind: &mut ShapeKind, grid: Coord) {
    let speed = grid.max(1) as f64;
    match kind {
        ShapeKind::Rectangle(rect) => {
            ui.push_id("rectangle_properties", |ui| {
                ui.label("Rectangle");
                let mut min_x = rect.min.x;
                let mut min_y = rect.min.y;
                let mut max_x = rect.max.x;
                let mut max_y = rect.max.y;
                ui.horizontal(|ui| {
                    ui.label("Min");
                    ui.push_id("min_x", |ui| {
                        ui.add(egui::DragValue::new(&mut min_x).speed(speed));
                    });
                    ui.push_id("min_y", |ui| {
                        ui.add(egui::DragValue::new(&mut min_y).speed(speed));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Max");
                    ui.push_id("max_x", |ui| {
                        ui.add(egui::DragValue::new(&mut max_x).speed(speed));
                    });
                    ui.push_id("max_y", |ui| {
                        ui.add(egui::DragValue::new(&mut max_y).speed(speed));
                    });
                });
                *rect = Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y));
            });
        }
        ShapeKind::Polygon(poly) => {
            ui.label(format!("Polygon vertices: {}", poly.points.len()));
            egui::ScrollArea::vertical()
                .id_salt("polygon_point_scroll")
                .max_height(150.0)
                .show(ui, |ui| {
                    for (index, point) in poly.points.iter_mut().enumerate() {
                        ui.push_id(("polygon_point", index), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{index}"));
                                ui.push_id("x", |ui| {
                                    ui.add(egui::DragValue::new(&mut point.x).speed(speed));
                                });
                                ui.push_id("y", |ui| {
                                    ui.add(egui::DragValue::new(&mut point.y).speed(speed));
                                });
                            });
                        });
                    }
                });
        }
        ShapeKind::Path { points, width } => {
            ui.label(format!("Path points: {}", points.len()));
            ui.horizontal(|ui| {
                ui.label("Width");
                ui.push_id("path_width", |ui| {
                    ui.add(egui::DragValue::new(width).speed(speed).range(1..=10_000));
                });
            });
            egui::ScrollArea::vertical()
                .id_salt("path_point_scroll")
                .max_height(150.0)
                .show(ui, |ui| {
                    for (index, point) in points.iter_mut().enumerate() {
                        ui.push_id(("path_point", index), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{index}"));
                                ui.push_id("x", |ui| {
                                    ui.add(egui::DragValue::new(&mut point.x).speed(speed));
                                });
                                ui.push_id("y", |ui| {
                                    ui.add(egui::DragValue::new(&mut point.y).speed(speed));
                                });
                            });
                        });
                    }
                });
        }
        ShapeKind::Via {
            center,
            size,
            lower: _,
            upper: _,
        } => {
            ui.push_id("via_properties", |ui| {
                ui.label("Via");
                ui.horizontal(|ui| {
                    ui.label("Center");
                    ui.push_id("center_x", |ui| {
                        ui.add(egui::DragValue::new(&mut center.x).speed(speed));
                    });
                    ui.push_id("center_y", |ui| {
                        ui.add(egui::DragValue::new(&mut center.y).speed(speed));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.push_id("size", |ui| {
                        ui.add(egui::DragValue::new(size).speed(speed).range(1..=10_000));
                    });
                });
            });
        }
        ShapeKind::Label { position, text } => {
            ui.push_id("label_properties", |ui| {
                ui.label("Label");
                ui.horizontal(|ui| {
                    ui.label("Pos");
                    ui.push_id("position_x", |ui| {
                        ui.add(egui::DragValue::new(&mut position.x).speed(speed));
                    });
                    ui.push_id("position_y", |ui| {
                        ui.add(egui::DragValue::new(&mut position.y).speed(speed));
                    });
                });
                ui.add(
                    egui::TextEdit::singleline(text)
                        .desired_width(180.0)
                        .id_salt("label_text"),
                );
            });
        }
        ShapeKind::Measurement { a, b, label } => {
            ui.push_id("measurement_properties", |ui| {
                ui.label("Measurement");
                ui.horizontal(|ui| {
                    ui.label("A");
                    ui.push_id("a_x", |ui| {
                        ui.add(egui::DragValue::new(&mut a.x).speed(speed));
                    });
                    ui.push_id("a_y", |ui| {
                        ui.add(egui::DragValue::new(&mut a.y).speed(speed));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("B");
                    ui.push_id("b_x", |ui| {
                        ui.add(egui::DragValue::new(&mut b.x).speed(speed));
                    });
                    ui.push_id("b_y", |ui| {
                        ui.add(egui::DragValue::new(&mut b.y).speed(speed));
                    });
                });
                ui.add(
                    egui::TextEdit::singleline(label)
                        .desired_width(180.0)
                        .id_salt("measurement_label"),
                );
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn send_client_message<W>(write: &mut W, message: &ClientMessage) -> Result<(), W::Error>
where
    W: Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
{
    let text = serde_json::to_string(message).expect("client collaboration messages serialize");
    write
        .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
        .await
}

#[cfg(target_arch = "wasm32")]
fn send_client_message_wasm(socket: &web_sys::WebSocket, message: &ClientMessage) {
    let Ok(text) = serde_json::to_string(message) else {
        return;
    };
    let _ = socket.send_with_str(&text);
}

#[cfg(target_arch = "wasm32")]
fn wasm_collaboration_url() -> String {
    let Some(window) = web_sys::window() else {
        return "ws://127.0.0.1:4141/ws".to_string();
    };
    let location = window.location();
    let search = location.search().unwrap_or_default();
    if let Some(url) = query_param(&search, "sync") {
        return url;
    }
    let ws_scheme = if location.protocol().unwrap_or_default() == "https:" {
        "wss"
    } else {
        "ws"
    };
    let host = location
        .hostname()
        .ok()
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    format!("{ws_scheme}://{host}:4141/ws")
}

#[cfg(target_arch = "wasm32")]
fn query_param(search: &str, target: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = decode_query_component(parts.next()?);
            let value = decode_query_component(parts.next().unwrap_or_default());
            (key == target).then_some(value)
        })
}

#[cfg(target_arch = "wasm32")]
fn decode_query_component(value: &str) -> String {
    let mut decoded = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hi = hex_value(bytes[index + 1]);
                let lo = hex_value(bytes[index + 2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    decoded.push((hi * 16 + lo) as char);
                    index += 3;
                } else {
                    decoded.push('%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte as char);
                index += 1;
            }
        }
    }
    decoded
}

#[cfg(target_arch = "wasm32")]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn tool_button(ui: &mut egui::Ui, active: &mut Tool, value: Tool, label: &str) {
    if ui.selectable_label(*active == value, label).clicked() {
        *active = value;
    }
}

fn add_slab_faces(
    faces: &mut Vec<Face3d>,
    points: &[Point],
    base_z: f32,
    top_z: f32,
    color: Color32,
) {
    if points.len() < 3 || faces.len() >= MAX_3D_FACES {
        return;
    }
    let top: Vec<_> = points
        .iter()
        .map(|point| Vec3f::new(point.x as f32, point.y as f32, top_z))
        .collect();
    let bottom: Vec<_> = points
        .iter()
        .map(|point| Vec3f::new(point.x as f32, point.y as f32, base_z))
        .collect();
    push_face(
        faces,
        FaceSurface3d::Top,
        top.clone(),
        color,
        shade_color(color, 1.25),
    );

    for index in 0..points.len() {
        if faces.len() >= MAX_3D_FACES {
            return;
        }
        let next = (index + 1) % points.len();
        let side = vec![bottom[index], bottom[next], top[next], top[index]];
        push_face(
            faces,
            FaceSurface3d::Side,
            side,
            shade_color(color, 0.62),
            shade_color(color, 0.92),
        );
    }
}

fn add_path_segment_3d_faces(
    faces: &mut Vec<Face3d>,
    a: Point,
    b: Point,
    width: Coord,
    base_z: f32,
    top_z: f32,
    color: Color32,
) {
    let dx = (b.x - a.x) as f32;
    let dy = (b.y - a.y) as f32;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let half = width.max(1) as f32 * 0.5;
    let nx = -dy / length * half;
    let ny = dx / length * half;
    let points = [
        Point::new(
            (a.x as f32 + nx).round() as Coord,
            (a.y as f32 + ny).round() as Coord,
        ),
        Point::new(
            (b.x as f32 + nx).round() as Coord,
            (b.y as f32 + ny).round() as Coord,
        ),
        Point::new(
            (b.x as f32 - nx).round() as Coord,
            (b.y as f32 - ny).round() as Coord,
        ),
        Point::new(
            (a.x as f32 - nx).round() as Coord,
            (a.y as f32 - ny).round() as Coord,
        ),
    ];
    add_slab_faces(faces, &points, base_z, top_z, color);
}

fn push_face(
    faces: &mut Vec<Face3d>,
    surface: FaceSurface3d,
    points: Vec<Vec3f>,
    fill: Color32,
    stroke: Color32,
) {
    if faces.len() < MAX_3D_FACES {
        faces.push(Face3d {
            surface,
            order: faces.len(),
            points,
            fill,
            stroke,
        });
    }
}

fn append_face_to_3d_batch(batch: &mut renderer::RenderBatch3d, face: &Face3d) {
    if face.points.len() < 3 {
        return;
    }
    let base = batch.vertices.len().min(u32::MAX as usize) as u32;
    let color = color32_to_gpu(face.fill);
    let normal = face_normal(face);
    batch
        .vertices
        .extend(face.points.iter().map(|point| renderer::GpuVertex3d {
            position: [point.x, point.y, point.z],
            normal: [normal.x, normal.y, normal.z],
            color,
        }));
    for index in 1..face.points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

fn append_guide_line_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    a: Vec3f,
    b: Vec3f,
    color: [f32; 4],
) {
    let Ok(base) = u32::try_from(batch.guide_vertices.len()) else {
        return;
    };
    if base == u32::MAX {
        return;
    }
    batch.guide_vertices.extend([
        renderer::GpuVertex3d {
            position: [a.x, a.y, a.z],
            normal: [0.0, 0.0, 0.0],
            color,
        },
        renderer::GpuVertex3d {
            position: [b.x, b.y, b.z],
            normal: [0.0, 0.0, 0.0],
            color,
        },
    ]);
    batch.guide_indices.extend_from_slice(&[base, base + 1]);
}

fn face_normal(face: &Face3d) -> Vec3f {
    let normal = polygon_normal_3d(&face.points);
    if face.surface == FaceSurface3d::Top && normal.z < 0.0 {
        normal * -1.0
    } else {
        normal
    }
}

fn polygon_normal_3d(points: &[Vec3f]) -> Vec3f {
    let Some(origin) = points.first().copied() else {
        return Vec3f::new(0.0, 0.0, 1.0);
    };
    for index in 1..points.len().saturating_sub(1) {
        let normal = (points[index] - origin).cross(points[index + 1] - origin);
        if normal.length() > f32::EPSILON {
            return normal.normalized();
        }
    }
    Vec3f::new(0.0, 0.0, 1.0)
}

fn color32_to_gpu(color: Color32) -> [f32; 4] {
    [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        color.a() as f32 / 255.0,
    ]
}

fn row_major_4x4_to_column_major(rows: [[f32; 4]; 4]) -> [f32; 16] {
    [
        rows[0][0], rows[1][0], rows[2][0], rows[3][0], rows[0][1], rows[1][1], rows[2][1],
        rows[3][1], rows[0][2], rows[1][2], rows[2][2], rows[3][2], rows[0][3], rows[1][3],
        rows[2][3], rows[3][3],
    ]
}

fn compare_projected_faces_3d_cpu_fallback(
    left: &ProjectedFace,
    right: &ProjectedFace,
) -> Ordering {
    // CPU fallback only: egui's painter has no depth buffer, so draw farther faces first.
    right
        .depth
        .total_cmp(&left.depth)
        .then_with(|| left.surface.draw_order().cmp(&right.surface.draw_order()))
        .then_with(|| left.order.cmp(&right.order))
}

fn clip_camera_polygon_to_near(points: &[CameraPoint3d], near: f32) -> Vec<CameraPoint3d> {
    let Some(mut previous) = points.last().copied() else {
        return Vec::new();
    };

    let mut clipped = Vec::with_capacity(points.len() + 2);
    let mut previous_inside = previous.depth >= near;
    for current in points.iter().copied() {
        let current_inside = current.depth >= near;
        if current_inside != previous_inside {
            clipped.push(intersect_camera_depth(previous, current, near));
        }
        if current_inside {
            clipped.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
    clipped
}

fn clip_camera_line_to_near(
    mut a: CameraPoint3d,
    mut b: CameraPoint3d,
    near: f32,
) -> Option<(CameraPoint3d, CameraPoint3d)> {
    let a_inside = a.depth >= near;
    let b_inside = b.depth >= near;
    match (a_inside, b_inside) {
        (true, true) => Some((a, b)),
        (false, false) => None,
        (false, true) => {
            a = intersect_camera_depth(a, b, near);
            Some((a, b))
        }
        (true, false) => {
            b = intersect_camera_depth(a, b, near);
            Some((a, b))
        }
    }
}

fn intersect_camera_depth(a: CameraPoint3d, b: CameraPoint3d, depth: f32) -> CameraPoint3d {
    let denominator = b.depth - a.depth;
    let t = if denominator.abs() <= f32::EPSILON {
        0.0
    } else {
        ((depth - a.depth) / denominator).clamp(0.0, 1.0)
    };
    let mut point = a.lerp(b, t);
    point.depth = depth;
    point
}

fn layer_color_3d(color: [f32; 4]) -> Color32 {
    Color32::from_rgb(
        (color[0] * 255.0).clamp(0.0, 255.0) as u8,
        (color[1] * 255.0).clamp(0.0, 255.0) as u8,
        (color[2] * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn shade_color(color: Color32, multiplier: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * multiplier).clamp(0.0, 255.0) as u8,
        (color.g() as f32 * multiplier).clamp(0.0, 255.0) as u8,
        (color.b() as f32 * multiplier).clamp(0.0, 255.0) as u8,
        color.a(),
    )
}

fn layer_color32(color: [f32; 4], multiplier: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (color[0] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[1] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[2] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[3] * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn equipment_process_step_label(kind: EquipmentToolKind) -> &'static str {
    match kind {
        EquipmentToolKind::SpinCoater => "litho.coat",
        EquipmentToolKind::HotPlate => "litho.soft_bake",
        EquipmentToolKind::MaskAligner => "litho.expose",
        EquipmentToolKind::Etcher => "etch.pattern_transfer",
        EquipmentToolKind::Microscope => "metrology.visual_inspection",
        EquipmentToolKind::ProbeStation => "metrology.parametric_probe",
    }
}

fn equipment_state_color(state: EquipmentToolState) -> Color32 {
    match state {
        EquipmentToolState::Offline => Color32::from_rgb(145, 150, 158),
        EquipmentToolState::OnlineIdle => Color32::from_rgb(105, 190, 120),
        EquipmentToolState::RecipeLoaded => Color32::from_rgb(98, 170, 235),
        EquipmentToolState::Running => Color32::from_rgb(250, 198, 90),
        EquipmentToolState::Completed => Color32::from_rgb(120, 205, 180),
        EquipmentToolState::Alarm => Color32::from_rgb(245, 98, 98),
        EquipmentToolState::Maintenance => Color32::from_rgb(205, 150, 245),
    }
}

fn equipment_alarm_color(severity: AlarmSeverity) -> Color32 {
    match severity {
        AlarmSeverity::Advisory => Color32::from_rgb(120, 185, 235),
        AlarmSeverity::Warning => Color32::from_rgb(245, 185, 75),
        AlarmSeverity::Critical => Color32::from_rgb(245, 92, 92),
    }
}

fn equipment_run_status_color(status: RunStatus) -> Color32 {
    match status {
        RunStatus::Running => Color32::from_rgb(250, 198, 90),
        RunStatus::Completed => Color32::from_rgb(105, 190, 120),
        RunStatus::Aborted => Color32::from_rgb(180, 160, 130),
        RunStatus::Alarmed => Color32::from_rgb(245, 92, 92),
    }
}

fn equipment_recipe_run_summary(tool: &EquipmentTool, now_s: u64) -> String {
    if let Some(run) = &tool.active_run {
        return format!(
            "{}\n{} s active",
            run.recipe.recipe_id,
            run.elapsed_s(now_s)
        );
    }
    if let Some(selection) = &tool.selected_recipe {
        return format!(
            "{}\nv{} loaded",
            selection.recipe_id, selection.recipe_version
        );
    }
    "No recipe".to_string()
}

fn equipment_recent_sensor_summary(tool: &EquipmentTool) -> String {
    let names = equipment_sensor_names(tool);
    if names.is_empty() {
        return "No samples".to_string();
    }
    names
        .into_iter()
        .take(2)
        .filter_map(|name| {
            tool.latest_sensor(&name)
                .map(|sample| format!("{name}: {}", equipment_sensor_value(sample)))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn equipment_sensor_names(tool: &EquipmentTool) -> Vec<String> {
    let mut names = BTreeSet::new();
    for sample in &tool.recent_sensors {
        names.insert(sample.name.clone());
    }
    names.into_iter().collect()
}

fn equipment_sensor_series(tool: &EquipmentTool, name: &str, max_samples: usize) -> Vec<f64> {
    let mut values = tool
        .recent_sensors
        .iter()
        .rev()
        .filter(|sample| sample.name == name)
        .take(max_samples)
        .map(|sample| sample.value)
        .collect::<Vec<_>>();
    values.reverse();
    values
}

fn equipment_sensor_value(sample: &SensorSample) -> String {
    let precision = if sample.value.abs() >= 100.0 { 0 } else { 2 };
    format!("{:.*} {}", precision, sample.value, sample.unit)
}

fn equipment_sparkline(ui: &mut egui::Ui, values: &[f64], color: Color32) {
    let width = ui.available_width().clamp(120.0, 260.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 34.0), Sense::hover());
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_gray(78)),
        StrokeKind::Inside,
    );
    if values.len() < 2 {
        return;
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).max(0.000_001);
    let points = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = rect.left()
                + rect.width() * (index as f32 / (values.len().saturating_sub(1)) as f32);
            let normalized = ((*value - min) / span) as f32;
            let y = rect.bottom() - rect.height() * normalized;
            Pos2::new(x, y)
        })
        .collect::<Vec<_>>();
    for pair in points.windows(2) {
        ui.painter()
            .line_segment([pair[0], pair[1]], Stroke::new(1.6, color));
    }
}

fn wafer_mm_to_screen(point_mm: [f64; 2], center: Pos2, scale: f32) -> Pos2 {
    center + vec2(point_mm[0] as f32 * scale, -(point_mm[1] as f32) * scale)
}

fn metrology_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(230, 235, 228, 85)),
        StrokeKind::Inside,
    );
}

fn format_metrology_value(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::DefectCount => format!("{:.0}", value),
        MeasurementKind::PassFail => {
            if value >= 0.5 {
                "pass".to_string()
            } else {
                "fail".to_string()
            }
        }
        _ => format!("{:.2} {}", value, kind.unit()),
    }
}

fn format_metrology_delta(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::DefectCount => format!("{:.2}", value),
        MeasurementKind::PassFail => format!("{:.2}", value),
        _ => format!("{:.2} {}", value, kind.unit()),
    }
}

fn metrology_measurement_color(
    measurement: &Measurement,
    summary: layout_model::metrology::MeasurementSummary,
) -> Color32 {
    if measurement.status != MeasurementStatus::Pass {
        return metrology_status_color(measurement.status);
    }
    if measurement.kind == MeasurementKind::PassFail {
        return if measurement.value >= 0.5 {
            Color32::from_rgb(76, 178, 116)
        } else {
            metrology_status_color(MeasurementStatus::Fail)
        };
    }
    let Some(min) = summary.min else {
        return Color32::from_rgb(86, 130, 150);
    };
    let Some(max) = summary.max else {
        return Color32::from_rgb(86, 130, 150);
    };
    let t = if (max - min).abs() <= f64::EPSILON {
        0.5
    } else {
        ((measurement.value - min) / (max - min)).clamp(0.0, 1.0) as f32
    };
    metrology_gradient_color(t)
}

fn metrology_status_color(status: MeasurementStatus) -> Color32 {
    match status {
        MeasurementStatus::Pass => Color32::from_rgb(76, 178, 116),
        MeasurementStatus::Fail => Color32::from_rgb(224, 80, 75),
        MeasurementStatus::Outlier => Color32::from_rgb(238, 184, 72),
    }
}

fn metrology_gradient_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        lerp_color(
            Color32::from_rgb(64, 138, 194),
            Color32::from_rgb(78, 176, 118),
            t * 2.0,
        )
    } else {
        lerp_color(
            Color32::from_rgb(78, 176, 118),
            Color32::from_rgb(224, 150, 72),
            (t - 0.5) * 2.0,
        )
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let channel = |left: u8, right: u8| {
        (left as f32 + (right as f32 - left as f32) * t).clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}

fn short_user(user: Uuid) -> String {
    user.simple().to_string().chars().take(6).collect()
}

fn drc_marker_key(violation: &DrcViolation) -> String {
    let mut shapes = violation
        .shape_ids
        .iter()
        .map(|id| id.0.to_string())
        .collect::<Vec<_>>();
    shapes.sort();
    format!(
        "{}|{}|{},{},{},{}|{}|{:.3}",
        violation.rule,
        shapes.join(","),
        violation.bounds.min.x,
        violation.bounds.min.y,
        violation.bounds.max.x,
        violation.bounds.max.y,
        violation.required,
        violation.actual
    )
}

fn marker_status_label(state: &MarkerState) -> &'static str {
    match (state.waived, state.hidden) {
        (true, true) => " [waived hidden]",
        (true, false) => " [waived]",
        (false, true) => " [hidden]",
        (false, false) => "",
    }
}

fn is_collaboration_disconnect(message: &str) -> bool {
    message.starts_with("failed to connect")
        || message.starts_with("collaboration socket closed")
        || message.starts_with("collaboration socket error")
}

fn remote_user_color(user: Uuid) -> Color32 {
    const PALETTE: [Color32; 6] = [
        Color32::from_rgb(120, 230, 210),
        Color32::from_rgb(255, 204, 92),
        Color32::from_rgb(255, 128, 144),
        Color32::from_rgb(144, 196, 255),
        Color32::from_rgb(178, 238, 132),
        Color32::from_rgb(232, 160, 255),
    ];
    let index = (user.as_u128() as usize) % PALETTE.len();
    PALETTE[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_core::DBU_PER_MICRON;
    use std::collections::BTreeSet;

    #[test]
    fn navigation_metadata_covers_every_view_mode_once() {
        let labels = ViewMode::ALL
            .iter()
            .map(|mode| mode.nav_label())
            .collect::<BTreeSet<_>>();
        assert_eq!(labels.len(), ViewMode::ALL.len());
        assert!(ViewMode::ALL.iter().all(|mode| !mode.title().is_empty()));
        assert!(ViewMode::ALL.iter().all(|mode| !mode.detail().is_empty()));
        assert!(
            ModuleGroup::ALL
                .iter()
                .all(|group| ViewMode::ALL.iter().any(|mode| mode.group() == *group))
        );
    }

    #[test]
    fn navigation_groups_follow_fab_workflow_order() {
        let grouped = ModuleGroup::ALL
            .iter()
            .map(|group| {
                (
                    group.label(),
                    ViewMode::ALL
                        .iter()
                        .filter(|mode| mode.group() == *group)
                        .map(|mode| mode.nav_label())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            grouped,
            vec![
                ("Design", vec!["Mask layout", "3D viewport", "Reticle prep"]),
                ("Operations", vec!["Equipment", "Traceability"]),
                ("Analysis", vec!["Metrology", "Yield", "SPC / FDC"]),
                ("Engineering", vec!["R2R control", "DOE"]),
            ]
        );
    }

    #[test]
    fn side_panels_follow_view_context() {
        assert!(ViewMode::Layout2d.has_inspector_panel());
        assert!(ViewMode::Layout2d.has_secondary_panel());
        assert!(ViewMode::Layout3d.has_inspector_panel());
        assert!(ViewMode::Layout3d.has_secondary_panel());

        assert!(ViewMode::MaskPrep.has_inspector_panel());
        assert!(ViewMode::MaskPrep.has_secondary_panel());
        assert!(ViewMode::Metrology.has_inspector_panel());
        assert!(ViewMode::Metrology.has_secondary_panel());
        assert!(ViewMode::Experiment.has_inspector_panel());
        assert!(ViewMode::Experiment.has_secondary_panel());

        assert!(!ViewMode::Yield.has_inspector_panel());
        assert!(!ViewMode::Yield.has_secondary_panel());
        assert!(!ViewMode::FabControl.has_inspector_panel());
        assert!(!ViewMode::FabControl.has_secondary_panel());

        assert!(ViewMode::SpcFdc.has_inspector_panel());
        assert!(!ViewMode::SpcFdc.has_secondary_panel());
        assert!(ViewMode::ProcessControl.has_inspector_panel());
        assert!(!ViewMode::ProcessControl.has_secondary_panel());
        assert!(ViewMode::Traceability.has_inspector_panel());
        assert!(!ViewMode::Traceability.has_secondary_panel());
    }

    #[test]
    fn blank_workspace_dataset_has_no_demo_operational_data() {
        let dataset = WorkspaceDataset::blank();

        assert!(dataset.document.shapes.is_empty());
        assert!(dataset.mes.lots.is_empty());
        assert!(dataset.yield_analysis.lots.is_empty());
        assert!(dataset.wafer_map.dies.is_empty());
        assert!(dataset.recipe_catalog.recipes.is_empty());
        assert_eq!(dataset.genealogy.summary().lot_count, 0);
        assert!(dataset.experiment_plan.runs.is_empty());
        assert!(dataset.process_control.loops.is_empty());
        assert_eq!(dataset.equipment.tools().count(), 0);
    }

    #[test]
    fn demo_workspace_dataset_is_explicitly_populated() {
        let dataset = WorkspaceDataset::demo();

        assert!(!dataset.document.shapes.is_empty());
        assert!(!dataset.mes.lots.is_empty());
        assert!(!dataset.yield_analysis.lots.is_empty());
        assert!(!dataset.wafer_map.dies.is_empty());
        assert!(!dataset.recipe_catalog.recipes.is_empty());
        assert!(dataset.genealogy.summary().lot_count > 0);
        assert!(!dataset.experiment_plan.runs.is_empty());
        assert!(!dataset.process_control.loops.is_empty());
        assert!(dataset.equipment.tools().count() > 0);
    }

    #[test]
    fn scale_bar_picks_readable_lengths() {
        assert_eq!(nice_scale_length_dbu(80.0), 100);
        assert_eq!(nice_scale_length_dbu(1_600.0), 2_000);
        assert_eq!(nice_scale_length_dbu(52_000.0), 50_000);
    }

    #[test]
    fn scale_bar_formats_nm_and_um() {
        assert_eq!(format_scale_label(500, DBU_PER_MICRON), "500 nm");
        assert_eq!(format_scale_label(2_000, DBU_PER_MICRON), "2 um");
        assert_eq!(format_scale_label(2_500, DBU_PER_MICRON), "2.50 um");
        assert_eq!(format_scale_label(1_000, 2_000), "500 nm");
        assert_eq!(
            format_physical_length(1_414.213_562, DBU_PER_MICRON),
            "1.41 um"
        );
    }

    #[test]
    fn length_format_honors_unit_preferences_and_precision() {
        assert_eq!(
            format_physical_length_with_options(500.0, DBU_PER_MICRON, UnitDisplay::Microns, 3,),
            "0.500 um"
        );
        assert_eq!(
            format_physical_length_with_options(
                1_500.0,
                DBU_PER_MICRON,
                UnitDisplay::Nanometers,
                0,
            ),
            "1500 nm"
        );
        assert_eq!(
            format_physical_length_with_options(12.5, DBU_PER_MICRON, UnitDisplay::Dbu, 1),
            "12.5 dbu"
        );
    }

    #[test]
    fn fly_camera_looks_at_target() {
        let camera = Camera3d::look_at(Vec3f::new(-1_000.0, 0.0, 0.0), Vec3f::ZERO, 2_000.0);
        let forward = camera.forward();
        assert!(forward.x > 0.999);
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
        assert!(camera.up().z > 0.999);
    }

    #[test]
    fn near_plane_clips_partially_visible_faces() {
        let points = [
            CameraPoint3d {
                x: -20.0,
                y: -10.0,
                depth: 5.0,
            },
            CameraPoint3d {
                x: 20.0,
                y: -10.0,
                depth: 5.0,
            },
            CameraPoint3d {
                x: 20.0,
                y: 10.0,
                depth: 50.0,
            },
            CameraPoint3d {
                x: -20.0,
                y: 10.0,
                depth: 50.0,
            },
        ];
        let clipped = clip_camera_polygon_to_near(&points, CAMERA_NEAR_PLANE);
        assert_eq!(clipped.len(), 4);
        assert!(clipped.iter().all(|point| point.depth >= CAMERA_NEAR_PLANE));
        assert!(
            clipped
                .iter()
                .any(|point| (point.depth - CAMERA_NEAR_PLANE).abs() < 0.001)
        );
    }

    #[test]
    fn near_plane_clips_grid_lines() {
        let a = CameraPoint3d {
            x: -10.0,
            y: 0.0,
            depth: 4.0,
        };
        let b = CameraPoint3d {
            x: 10.0,
            y: 0.0,
            depth: 40.0,
        };
        let Some((clipped_a, clipped_b)) = clip_camera_line_to_near(a, b, CAMERA_NEAR_PLANE) else {
            panic!("partially visible line should survive clipping");
        };
        assert!((clipped_a.depth - CAMERA_NEAR_PLANE).abs() < 0.001);
        assert_eq!(clipped_b, b);
    }

    #[test]
    fn layer_color_3d_is_opaque() {
        let color = layer_color_3d([0.2, 0.4, 0.6, 0.25]);
        assert_eq!(color.a(), 255);
        assert_eq!(color.r(), 51);
        assert_eq!(color.g(), 102);
        assert_eq!(color.b(), 153);
    }

    #[test]
    fn cpu_fallback_3d_sort_draws_side_walls_before_top_caps_at_same_depth() {
        let mut faces = vec![
            ProjectedFace {
                surface: FaceSurface3d::Top,
                depth: 768.0,
                order: 0,
                points: Vec::new(),
                fill: Color32::WHITE,
                stroke: Color32::WHITE,
            },
            ProjectedFace {
                surface: FaceSurface3d::Side,
                depth: 768.0,
                order: 1,
                points: Vec::new(),
                fill: Color32::WHITE,
                stroke: Color32::WHITE,
            },
        ];
        faces.sort_by(compare_projected_faces_3d_cpu_fallback);
        assert_eq!(faces[0].surface, FaceSurface3d::Side);
        assert_eq!(faces[1].surface, FaceSurface3d::Top);
    }

    #[test]
    fn cpu_fallback_3d_sort_draws_far_primitive_before_near_primitive() {
        let mut faces = vec![
            ProjectedFace {
                surface: FaceSurface3d::Side,
                depth: 1_000.0,
                order: 0,
                points: Vec::new(),
                fill: Color32::from_rgb(255, 0, 0),
                stroke: Color32::WHITE,
            },
            ProjectedFace {
                surface: FaceSurface3d::Top,
                depth: 1_020.0,
                order: 1,
                points: Vec::new(),
                fill: Color32::from_rgb(0, 0, 255),
                stroke: Color32::WHITE,
            },
        ];

        faces.sort_by(compare_projected_faces_3d_cpu_fallback);

        assert_eq!(faces[0].depth, 1_020.0);
        assert_eq!(faces[1].depth, 1_000.0);
        assert_eq!(faces[1].fill, Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn cpu_fallback_3d_overlays_are_after_sorted_layout() {
        let mut layout_faces = vec![
            ProjectedFace {
                surface: FaceSurface3d::Top,
                depth: 1_200.0,
                order: 0,
                points: Vec::new(),
                fill: Color32::from_rgb(0, 0, 255),
                stroke: Color32::WHITE,
            },
            ProjectedFace {
                surface: FaceSurface3d::Top,
                depth: 900.0,
                order: 1,
                points: Vec::new(),
                fill: Color32::from_rgb(255, 0, 0),
                stroke: Color32::WHITE,
            },
        ];

        layout_faces.sort_by(compare_projected_faces_3d_cpu_fallback);
        let mut draw_order = layout_faces
            .iter()
            .map(|face| face.fill)
            .collect::<Vec<_>>();
        draw_order.push(Color32::from_rgb(255, 255, 255));

        assert_eq!(draw_order[0], Color32::from_rgb(0, 0, 255));
        assert_eq!(draw_order[1], Color32::from_rgb(255, 0, 0));
        assert_eq!(draw_order[2], Color32::from_rgb(255, 255, 255));
    }

    #[test]
    fn slab_faces_build_top_and_sides() {
        let mut faces = Vec::new();
        let rect = Rect::new(Point::new(0, 0), Point::new(100, 50));
        add_slab_faces(
            &mut faces,
            &rect.corners(),
            10.0,
            40.0,
            Color32::from_rgb(40, 200, 120),
        );
        assert_eq!(faces.len(), 5);
        assert_eq!(faces[0].surface, FaceSurface3d::Top);
        assert!(
            faces[1..]
                .iter()
                .all(|face| face.surface == FaceSurface3d::Side)
        );
        assert!(faces.iter().all(|face| face.points.len() >= 3));
    }

    #[test]
    fn path_segment_3d_faces_have_width() {
        let mut faces = Vec::new();
        add_path_segment_3d_faces(
            &mut faces,
            Point::new(0, 0),
            Point::new(100, 0),
            20,
            0.0,
            10.0,
            Color32::from_rgb(40, 120, 220),
        );
        assert_eq!(faces.len(), 5);
        let top = &faces[0].points;
        let y_values: Vec<_> = top.iter().map(|point| point.y.round() as i32).collect();
        assert!(y_values.contains(&10));
        assert!(y_values.contains(&-10));
    }

    #[test]
    fn face_3d_gpu_batch_keeps_world_depth_coordinates() {
        let face = Face3d {
            surface: FaceSurface3d::Top,
            order: 0,
            points: vec![
                Vec3f::new(0.0, 0.0, 20.0),
                Vec3f::new(100.0, 0.0, 20.0),
                Vec3f::new(100.0, 100.0, 20.0),
                Vec3f::new(0.0, 100.0, 20.0),
            ],
            fill: Color32::from_rgb(64, 128, 255),
            stroke: Color32::WHITE,
        };
        let mut batch = renderer::RenderBatch3d::default();
        append_face_to_3d_batch(&mut batch, &face);

        assert_eq!(batch.vertices.len(), 4);
        assert_eq!(batch.indices, vec![0, 1, 2, 0, 2, 3]);
        assert!(
            batch
                .vertices
                .iter()
                .all(|vertex| vertex.position[2] == 20.0)
        );
        assert!(
            batch
                .vertices
                .iter()
                .all(|vertex| vertex.normal == [0.0, 0.0, 1.0])
        );
        assert_eq!(
            batch.vertices[0].color,
            [64.0 / 255.0, 128.0 / 255.0, 1.0, 1.0]
        );
    }

    #[test]
    fn guide_lines_use_separate_unlit_3d_batch_stream() {
        let mut batch = renderer::RenderBatch3d::default();
        append_guide_line_to_3d_batch(
            &mut batch,
            Vec3f::new(-10.0, 0.0, 0.0),
            Vec3f::new(10.0, 0.0, 0.0),
            [1.0, 0.0, 0.0, 0.5],
        );

        assert!(batch.vertices.is_empty());
        assert!(batch.indices.is_empty());
        assert_eq!(batch.guide_indices, vec![0, 1]);
        assert_eq!(batch.guide_vertices.len(), 2);
        assert!(
            batch
                .guide_vertices
                .iter()
                .all(|vertex| vertex.normal == [0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn row_major_matrix_is_uploaded_column_major_for_wgsl() {
        let matrix = row_major_4x4_to_column_major([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);

        assert_eq!(
            matrix,
            [
                1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0,
                16.0,
            ]
        );
    }

    #[test]
    fn collaboration_disconnect_messages_allow_retry() {
        assert!(is_collaboration_disconnect(
            "failed to connect to ws://127.0.0.1:4141/ws"
        ));
        assert!(is_collaboration_disconnect("collaboration socket closed"));
        assert!(is_collaboration_disconnect(
            "collaboration socket error: reset"
        ));
        assert!(!is_collaboration_disconnect(
            "invalid collaboration message: nope"
        ));
    }

    #[test]
    fn drc_marker_keys_are_stable_across_shape_ordering() {
        let mut first = DrcViolation {
            id: 1,
            rule: "min_spacing".to_string(),
            message: "spacing".to_string(),
            shape_ids: vec![ShapeId(7), ShapeId(3)],
            bounds: Rect::from_min_size(Point::new(10, 20), 30, 40),
            required: 200,
            actual: 120.0,
        };
        let mut second = first.clone();
        second.id = 99;
        second.shape_ids.reverse();

        assert_eq!(drc_marker_key(&first), drc_marker_key(&second));
        assert_eq!(
            marker_status_label(&MarkerState {
                hidden: false,
                waived: true,
                note: None,
            }),
            " [waived]"
        );
        first.actual = 121.0;
        assert_ne!(drc_marker_key(&first), drc_marker_key(&second));
    }

    #[test]
    fn moving_polygon_vertex_replaces_only_that_point() {
        let shape = Shape {
            id: ShapeId(7),
            layer: LayerId(1),
            net: None,
            kind: ShapeKind::Polygon(geometry_core::Polygon::new(vec![
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 100),
            ])),
            name: None,
        };

        let moved = shape_with_moved_vertex(&shape, 1, Point::new(120, 20)).unwrap();

        let ShapeKind::Polygon(poly) = moved.kind else {
            panic!("expected polygon");
        };
        assert_eq!(poly.points[0], Point::new(0, 0));
        assert_eq!(poly.points[1], Point::new(120, 20));
        assert_eq!(poly.points[2], Point::new(100, 100));
    }

    #[test]
    fn moving_rectangle_vertex_keeps_opposite_corner_fixed() {
        let shape = Shape {
            id: ShapeId(8),
            layer: LayerId(1),
            net: None,
            kind: ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 80)),
            name: None,
        };

        let moved = shape_with_moved_vertex(&shape, 0, Point::new(-20, -10)).unwrap();

        let ShapeKind::Rectangle(rect) = moved.kind else {
            panic!("expected rectangle");
        };
        assert_eq!(rect, Rect::new(Point::new(-20, -10), Point::new(100, 80)));
    }

    #[test]
    fn inserting_and_deleting_polygon_vertices_round_trips() {
        let shape = Shape {
            id: ShapeId(9),
            layer: LayerId(1),
            net: None,
            kind: ShapeKind::Polygon(geometry_core::Polygon::new(vec![
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 100),
                Point::new(0, 100),
            ])),
            name: None,
        };

        let inserted = shape_with_inserted_vertex(&shape, 1, Point::new(120, 40)).unwrap();
        let ShapeKind::Polygon(poly) = &inserted.kind else {
            panic!("expected polygon");
        };
        assert_eq!(poly.points.len(), 5);
        assert_eq!(poly.points[2], Point::new(120, 40));

        let deleted = shape_with_deleted_vertex(&inserted, 2).unwrap();
        assert_eq!(deleted.kind, shape.kind);
    }

    #[test]
    fn moving_rectilinear_polygon_edge_constrains_to_edge_normal() {
        let shape = Shape {
            id: ShapeId(10),
            layer: LayerId(1),
            net: None,
            kind: ShapeKind::Polygon(geometry_core::Polygon::new(vec![
                Point::new(0, 0),
                Point::new(100, 0),
                Point::new(100, 100),
                Point::new(0, 100),
            ])),
            name: None,
        };

        let moved = shape_with_moved_edge(&shape, 0, Vector::new(60, 20)).unwrap();
        let ShapeKind::Polygon(poly) = moved.kind else {
            panic!("expected polygon");
        };
        assert_eq!(poly.points[0], Point::new(0, 20));
        assert_eq!(poly.points[1], Point::new(100, 20));
        assert_eq!(poly.points[2], Point::new(100, 100));
        assert_eq!(poly.points[3], Point::new(0, 100));
    }

    #[test]
    fn rotate_and_mirror_shape_points_around_center() {
        let kind = ShapeKind::Path {
            points: vec![Point::new(0, 0), Point::new(100, 0)],
            width: 10,
        };

        let rotated = transform_shape_kind(&kind, Point::new(50, 50), ShapeTransform::RotateCw90);
        let ShapeKind::Path { points, .. } = rotated else {
            panic!("expected path");
        };
        assert_eq!(points, vec![Point::new(0, 100), Point::new(0, 0)]);

        let mirrored = transform_shape_kind(&kind, Point::new(50, 50), ShapeTransform::MirrorX);
        let ShapeKind::Path { points, .. } = mirrored else {
            panic!("expected path");
        };
        assert_eq!(points, vec![Point::new(100, 0), Point::new(0, 0)]);
    }
}
