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
    FlattenedShapeView, InstanceArray, InstanceId, Layer, LayerId, LayoutIndex, LoroCrdtLog,
    LoroUpdate, MarkerState, NetId, Operation, ProcessLayer, ServerMessage, Shape, ShapeId,
    ShapeKind, ShapeKindView, ShapeOccurrenceId, TechnologyFile, Transform, builtin_technologies,
    connectivity::{ConnectivityReport, NetComponent, extract_connectivity},
    environment::CleanroomEnvironment,
    equipment::{
        AlarmSeverity, EquipmentEvent, EquipmentSimulator, HostCommand,
        RecipeId as EquipmentRecipeId, RecipeSelection, RunStatus, SensorSample,
        Tool as EquipmentTool, ToolId as EquipmentToolId, ToolKind as EquipmentToolKind,
        ToolState as EquipmentToolState,
    },
    experiment::ExperimentPlan,
    fab_ref::{FabObjectKind, FabObjectRef},
    gdsii::{export_gdsii, import_gdsii},
    genealogy::LotGenealogy,
    inventory::Inventory,
    maintenance::MaintenanceModel,
    mes::{
        AuditOutcome, FabMesData, Lot, LotId, OperatorAction, ProcessRoute, ToolId, TravelerState,
        TravelerStatus,
    },
    metrology::{
        DefectClass, DieCoord, FabObjectLinks, HistogramBin, Measurement, MeasurementKind,
        MeasurementStatus, WaferGeometry, WaferMap,
    },
    notebook::LabNotebook,
    process_control::ProcessControlModel,
    process_flow::ProcessFlowModel,
    recipe::RecipeCatalog,
    safety::SafetySystem,
    scheduler::DispatchSchedule,
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
use tracing::{error, warn};
use uuid::Uuid;
use web_time::{Duration, Instant};

mod cross_section_panel;
mod environment_panel;
mod experiment_panel;
mod genealogy_panel;
mod inventory_panel;
mod layout_diff_panel;
mod maintenance_panel;
mod mask_panel;
mod notebook_panel;
mod process_control_panel;
mod process_flow_panel;
mod recipe_panel;
mod safety_panel;
mod scheduler_panel;
mod spc_fdc_panel;
mod ui_chrome;
mod workflow_panel;
use cross_section_panel::CrossSectionPanel;
use environment_panel::EnvironmentPanel;
use experiment_panel::ExperimentPlannerPanel;
use genealogy_panel::GenealogyPanel;
use inventory_panel::InventoryPanel;
use layout_diff_panel::LayoutDiffPanel;
use maintenance_panel::MaintenancePanel;
use mask_panel::MaskPrepPanel;
use notebook_panel::LabNotebookPanel;
use process_control_panel::ProcessControlPanel;
use process_flow_panel::ProcessFlowPanel;
use recipe_panel::RecipeManagerPanel;
use safety_panel::SafetyPanel;
use scheduler_panel::SchedulerPanel;
use spc_fdc_panel::SpcFdcPanel;
use workflow_panel::{WorkflowAction, WorkflowData, WorkflowDestination, WorkflowPanel};

#[cfg(not(target_arch = "wasm32"))]
use futures_util::{Sink, SinkExt, StreamExt};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

const SAVE_PATH: &str = "examples/fabricad_layout.json";
const WORKSPACE_PATH: &str = "examples/fabricad_workspace.json";
#[cfg(not(target_arch = "wasm32"))]
const DEMO_WORKSPACE_PATH: &str = "examples/fabricad_demo_workspace.json";
const GDS_PATH: &str = "examples/fabricad_layout.gds";
#[cfg(target_arch = "wasm32")]
const WASM_AUTOSAVE_KEY: &str = "fabricad.autosave.document";
const MAX_LORO_SEED_OBJECTS: usize = 5_000;
const MAX_CONNECTIVITY_OBJECTS: usize = 50_000;
const MAX_DRC_OBJECTS: usize = 50_000;
const TILE_MEMORY_BUDGET_BYTES: usize = 96 * 1024 * 1024;
const MAX_3D_RENDERED_SHAPES: usize = 1_000_000;
const MAX_3D_CPU_FALLBACK_FACES: usize = 24_000;
const CAMERA_NEAR_PLANE: f32 = 10.0;
const CAMERA_FAR_PLANE_MIN: f32 = 250_000.0;
const CAMERA_FAR_PLANE_SPAN_MULTIPLIER: f32 = 4.0;
const CAMERA_FAR_PLANE_MARGIN_MULTIPLIER: f32 = 1.5;
const MAX_CONNECTIVITY_ROWS: usize = 40;
const MAX_DRC_OVERLAY_MARKERS: usize = 500;
const LAYOUT_MIN_ZOOM: f32 = 0.000_05;
const LAYOUT_MAX_ZOOM: f32 = 512.0;
const PROCESS_LAYER_CHOICES: [ProcessLayer; 8] = [
    ProcessLayer::Diffusion,
    ProcessLayer::Poly,
    ProcessLayer::Contact,
    ProcessLayer::Metal1,
    ProcessLayer::Via1,
    ProcessLayer::Metal2,
    ProcessLayer::Oxide,
    ProcessLayer::Annotation,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tool {
    Select,
    Rect,
    Polygon,
    Path,
    Via,
    Measure,
    Route,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ViewMode {
    Workflow,
    Layout2d,
    Layout3d,
    FabControl,
    Metrology,
    Yield,
    MaskPrep,
    LayoutDiff,
    Inventory,
    Maintenance,
    Environment,
    Scheduler,
    Safety,
    SpcFdc,
    ProcessFlow,
    ProcessControl,
    CrossSection,
    Traceability,
    Experiment,
    Notebook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetrologyMapMode {
    ValueMap,
    DeviationMap,
    SpecWindow,
    DefectReview,
    ReviewQueue,
    OverlayVectors,
}

impl MetrologyMapMode {
    const ALL: [Self; 6] = [
        Self::ValueMap,
        Self::DeviationMap,
        Self::SpecWindow,
        Self::DefectReview,
        Self::ReviewQueue,
        Self::OverlayVectors,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::ValueMap => "Value map",
            Self::DeviationMap => "Deviation map",
            Self::SpecWindow => "Spec window",
            Self::DefectReview => "Defect review",
            Self::ReviewQueue => "Review queue",
            Self::OverlayVectors => "Overlay vectors",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            Self::ValueMap => "Value",
            Self::DeviationMap => "Delta",
            Self::SpecWindow => "Spec",
            Self::DefectReview => "Defects",
            Self::ReviewQueue => "Review",
            Self::OverlayVectors => "Overlay",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::ValueMap => "Color dies by the selected metrology result.",
            Self::DeviationMap => "Center color around the recipe target and spec limits.",
            Self::SpecWindow => "Show remaining process window against lower and upper specs.",
            Self::DefectReview => "Show defect density and review markers.",
            Self::ReviewQueue => "Prioritize sites with failed measurements, outliers, or defects.",
            Self::OverlayVectors => "Draw synthetic overlay-error vectors across the wafer.",
        }
    }
}

impl ViewMode {
    const ALL: [Self; 20] = [
        Self::Workflow,
        Self::Layout2d,
        Self::Layout3d,
        Self::MaskPrep,
        Self::LayoutDiff,
        Self::FabControl,
        Self::Inventory,
        Self::Maintenance,
        Self::Environment,
        Self::Scheduler,
        Self::Safety,
        Self::Traceability,
        Self::Metrology,
        Self::Yield,
        Self::SpcFdc,
        Self::ProcessFlow,
        Self::ProcessControl,
        Self::CrossSection,
        Self::Experiment,
        Self::Notebook,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Workflow => "Fab Workflow",
            Self::Layout2d => "Layout Editor",
            Self::Layout3d => "3D Layout View",
            Self::FabControl => "Fab Control Room",
            Self::Metrology => "Metrology Wafer Map",
            Self::Yield => "Yield Dashboard",
            Self::MaskPrep => "Mask / Reticle Prep",
            Self::LayoutDiff => "Layout Diff Review",
            Self::Inventory => "Inventory Tracker",
            Self::Maintenance => "Maintenance and Calibration",
            Self::Environment => "Cleanroom Environment",
            Self::Scheduler => "Scheduler / Dispatch",
            Self::Safety => "Safety and Interlock Dashboard",
            Self::SpcFdc => "SPC / FDC Monitor",
            Self::ProcessFlow => "Process-flow Designer",
            Self::ProcessControl => "Run-to-Run Control",
            Self::CrossSection => "Process Cross-Section",
            Self::Traceability => "Lot Traceability",
            Self::Experiment => "DOE Planner",
            Self::Notebook => "Lab Notebook",
        }
    }

    fn nav_label(self) -> &'static str {
        match self {
            Self::Workflow => "Workflow",
            Self::Layout2d => "Mask layout",
            Self::Layout3d => "3D viewport",
            Self::FabControl => "Equipment",
            Self::Metrology => "Metrology",
            Self::Yield => "Yield",
            Self::MaskPrep => "Reticle prep",
            Self::LayoutDiff => "Layout diff",
            Self::Inventory => "Inventory",
            Self::Maintenance => "Maintenance",
            Self::Environment => "Environment",
            Self::Scheduler => "Dispatch",
            Self::Safety => "Safety",
            Self::SpcFdc => "SPC / FDC",
            Self::ProcessFlow => "Process flow",
            Self::ProcessControl => "R2R control",
            Self::CrossSection => "Cross-section",
            Self::Traceability => "Traceability",
            Self::Experiment => "DOE",
            Self::Notebook => "Notebook",
        }
    }

    fn rail_label(self) -> &'static str {
        match self {
            Self::Workflow => "Home",
            Self::Layout2d => "Layout",
            Self::Layout3d => "3D",
            Self::MaskPrep => "Reticle",
            Self::LayoutDiff => "Diff",
            Self::FabControl => "Fab",
            Self::Inventory => "Inv",
            Self::Maintenance => "Maint",
            Self::Environment => "Env",
            Self::Scheduler => "Dispatch",
            Self::Safety => "Safety",
            Self::SpcFdc => "SPC",
            Self::ProcessFlow => "Flow",
            Self::ProcessControl => "R2R",
            Self::CrossSection => "X-sec",
            Self::Traceability => "Trace",
            Self::Experiment => "DOE",
            Self::Metrology => "Metro",
            Self::Yield => "Yield",
            Self::Notebook => "Notes",
        }
    }

    fn status_message(self) -> &'static str {
        match self {
            Self::Workflow => "integrated FabOS workflow",
            Self::Layout2d => "layout editor",
            Self::Layout3d => "3D flycam view",
            Self::FabControl => "FabOS equipment control room",
            Self::Metrology => "metrology wafer map",
            Self::Yield => "FabOS yield dashboard",
            Self::MaskPrep => "FabOS mask editor / reticle prep",
            Self::LayoutDiff => "layout diff review",
            Self::Inventory => "FabOS inventory and materials tracker",
            Self::Maintenance => "FabOS maintenance and calibration manager",
            Self::Environment => "FabOS cleanroom environmental monitor",
            Self::Scheduler => "FabOS scheduler and dispatch",
            Self::Safety => "FabOS simulated safety interlocks",
            Self::SpcFdc => "FabOS SPC/FDC monitor",
            Self::ProcessFlow => "FabOS process-flow designer",
            Self::ProcessControl => "FabOS run-to-run process control",
            Self::CrossSection => "FabOS process cross-section simulator",
            Self::Traceability => "FabOS lot genealogy trace",
            Self::Experiment => "FabOS DOE planner",
            Self::Notebook => "FabOS linked lab notebook",
        }
    }

    fn group(self) -> ModuleGroup {
        match self {
            Self::Layout2d | Self::Layout3d | Self::MaskPrep | Self::LayoutDiff => {
                ModuleGroup::Design
            }
            Self::Workflow
            | Self::FabControl
            | Self::Inventory
            | Self::Maintenance
            | Self::Environment
            | Self::Scheduler
            | Self::Safety
            | Self::Traceability => ModuleGroup::Operations,
            Self::Metrology | Self::Yield | Self::SpcFdc => ModuleGroup::Analysis,
            Self::ProcessFlow
            | Self::ProcessControl
            | Self::CrossSection
            | Self::Experiment
            | Self::Notebook => ModuleGroup::Engineering,
        }
    }

    fn has_inspector_panel(self) -> bool {
        !matches!(self, Self::FabControl | Self::Yield)
    }

    fn has_secondary_panel(self) -> bool {
        matches!(
            self,
            Self::Layout2d
                | Self::Layout3d
                | Self::Metrology
                | Self::MaskPrep
                | Self::CrossSection
                | Self::Experiment
                | Self::Notebook
        )
    }

    fn supports_viewport_fullscreen(self) -> bool {
        matches!(self, Self::Layout2d | Self::Layout3d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModuleGroup {
    Design,
    Operations,
    Analysis,
    Engineering,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandAction {
    SelectView(ViewMode),
    ToggleRail(ViewMode),
    SelectTool(Tool),
    RunDrc,
    LoadDemo,
    NewBlank,
    LoadStress(usize),
    LoadHierarchy,
    ShowOptions,
    ToggleGrid2d,
    ToggleGrid3d,
    ToggleDrcOverlay,
    ToggleOrigin,
    Reset3d,
    ToggleFullscreen,
}

#[derive(Clone, Debug)]
struct CommandEntry {
    label: String,
    detail: String,
    action: CommandAction,
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
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AppContext {
    active: Option<FabObjectRef>,
    lot: Option<String>,
    wafer: Option<String>,
    tool: Option<String>,
    recipe: Option<String>,
    material_lot: Option<String>,
}

impl AppContext {
    fn from_selections(
        selected_mes_lot: Option<&LotId>,
        selected_yield_lot: &str,
        selected_yield_wafer: &str,
        selected_tool: Option<&EquipmentToolId>,
    ) -> Self {
        let mut context = Self::default();
        if let Some(lot_id) = selected_mes_lot {
            context.set_lot(lot_id.as_str().to_string());
        } else if !selected_yield_lot.is_empty() {
            context.set_lot(selected_yield_lot.to_string());
        }
        if !selected_yield_wafer.is_empty() {
            context.wafer = Some(selected_yield_wafer.to_string());
        }
        if let Some(tool_id) = selected_tool {
            context.tool = Some(tool_id.as_str().to_string());
        }
        context
    }

    fn focus_lot(&self) -> Option<&str> {
        self.lot.as_deref()
    }

    fn focus_tool(&self) -> Option<&str> {
        self.tool.as_deref()
    }

    fn focus_recipe(&self) -> Option<&str> {
        self.recipe.as_deref()
    }

    fn set_lot(&mut self, lot_id: String) {
        self.active = Some(FabObjectRef::lot(lot_id.clone()));
        self.lot = Some(lot_id);
    }

    fn set_wafer(&mut self, wafer_id: String) {
        self.active = Some(FabObjectRef::wafer(wafer_id.clone()));
        self.wafer = Some(wafer_id);
    }

    fn set_tool(&mut self, tool_id: String) {
        self.active = Some(FabObjectRef::tool(tool_id.clone()));
        self.tool = Some(tool_id);
    }

    fn set_recipe(&mut self, recipe_id: String) {
        self.active = Some(FabObjectRef::recipe(recipe_id.clone()));
        self.recipe = Some(recipe_id);
    }

    fn set_material_lot(&mut self, material_lot_id: String) {
        self.active = Some(FabObjectRef::material_lot(material_lot_id.clone()));
        self.material_lot = Some(material_lot_id);
    }

    fn active_label(&self) -> String {
        self.active
            .as_ref()
            .map(FabObjectRef::label)
            .unwrap_or_else(|| "No object selected".to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DataSource {
    Blank,
    Demo,
    File(String),
    Generated(String),
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
    inventory: Inventory,
    maintenance: MaintenanceModel,
    environment: CleanroomEnvironment,
    scheduler: DispatchSchedule,
    safety: SafetySystem,
    experiment_plan: ExperimentPlan,
    process_control: ProcessControlModel,
    process_flow: ProcessFlowModel,
    lab_notebook: LabNotebook,
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
    show_drc_overlay: bool,
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
            show_origin_marker: false,
            show_drc_overlay: false,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Render3dStats {
    shapes: usize,
    faces: usize,
    capped: bool,
    top_caps_only: bool,
    cpu_face_capped: bool,
}

#[derive(Clone, Copy, Debug)]
struct Layer3dStyle {
    base_z: f32,
    top_z: f32,
    color: Color32,
}

#[derive(Clone, Debug)]
struct Cached3dScene {
    batch: Arc<renderer::RenderBatch3d>,
    fingerprint: renderer::BatchFingerprint,
    stats: Render3dStats,
    show_grid_3d: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GpuAdapterSummary {
    backend: String,
    device_type: String,
    name: String,
}

impl GpuAdapterSummary {
    fn from_wgpu_info(info: egui_wgpu::wgpu::AdapterInfo) -> Self {
        Self {
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            name: info.name,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Benchmark3dOptions {
    pub frames: usize,
    pub warmup_frames: usize,
}

impl Default for Benchmark3dOptions {
    fn default() -> Self {
        Self {
            frames: 180,
            warmup_frames: 30,
        }
    }
}

#[derive(Clone, Debug)]
struct Benchmark3dState {
    options: Benchmark3dOptions,
    intervals: Vec<f64>,
    canvas_cpu_ms: Vec<f64>,
    update_cpu_ms: Vec<f64>,
    phase_totals: Benchmark3dPhaseTimes,
    phase_samples: usize,
    warmup_seen: usize,
    gpu_frames: usize,
    cpu_frames: usize,
    last_gpu_completed_frames: u64,
    last_stats: Render3dStats,
    last_target_size: [u32; 2],
    completed: bool,
}

impl Benchmark3dState {
    fn new(options: Benchmark3dOptions) -> Self {
        Self {
            options,
            intervals: Vec::with_capacity(options.frames),
            canvas_cpu_ms: Vec::with_capacity(options.frames),
            update_cpu_ms: Vec::with_capacity(options.frames),
            phase_totals: Benchmark3dPhaseTimes::default(),
            phase_samples: 0,
            warmup_seen: 0,
            gpu_frames: 0,
            cpu_frames: 0,
            last_gpu_completed_frames: 0,
            last_stats: Render3dStats::default(),
            last_target_size: [0, 0],
            completed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Benchmark3dPhaseTimes {
    overhead: f64,
    toolbar: f64,
    navigation: f64,
    inspector: f64,
    layers: f64,
    windows: f64,
    mes: f64,
    central: f64,
}

impl Benchmark3dPhaseTimes {
    fn add(&mut self, other: Self) {
        self.overhead += other.overhead;
        self.toolbar += other.toolbar;
        self.navigation += other.navigation;
        self.inspector += other.inspector;
        self.layers += other.layers;
        self.windows += other.windows;
        self.mes += other.mes;
        self.central += other.central;
    }
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
    layout_bounds_cache: Option<Rect>,
    yield_analysis: YieldAnalysis,
    selected_yield_lot: String,
    selected_yield_wafer: String,
    app_context: AppContext,
    workflow_panel: WorkflowPanel,
    mask_panel: MaskPrepPanel,
    layout_diff_panel: LayoutDiffPanel,
    inventory_panel: InventoryPanel,
    maintenance_panel: MaintenancePanel,
    environment_panel: EnvironmentPanel,
    scheduler_panel: SchedulerPanel,
    safety_panel: SafetyPanel,
    spc_fdc_panel: SpcFdcPanel,
    process_flow_panel: ProcessFlowPanel,
    process_control_panel: ProcessControlPanel,
    cross_section_panel: CrossSectionPanel,
    genealogy_panel: GenealogyPanel,
    experiment_panel: ExperimentPlannerPanel,
    notebook_panel: LabNotebookPanel,
    technologies: Vec<TechnologyFile>,
    active_technology: usize,
    rules: RuleDeck,
    violations: Vec<DrcViolation>,
    connectivity: ConnectivityReport,
    show_hidden_markers: bool,
    show_waived_markers: bool,
    drc_marker_filter: String,
    connectivity_filter: String,
    nav_rail_modes: BTreeSet<ViewMode>,
    show_command_palette: bool,
    show_sidebar_modules: bool,
    show_inspector_drawer: bool,
    show_layers_drawer: bool,
    command_palette_query: String,
    viewport_fullscreen: bool,
    selected: BTreeSet<ShapeId>,
    selected_occurrence: Option<ShapeOccurrenceId>,
    active_layer: LayerId,
    tool: Tool,
    view_mode: ViewMode,
    wafer_map: WaferMap,
    metrology_map_mode: MetrologyMapMode,
    metrology_kind: MeasurementKind,
    selected_die: Option<DieCoord>,
    metrology_failed_only: bool,
    zoom: f32,
    pan: Vec2,
    camera_3d: Camera3d,
    flycam_captured: bool,
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
    gpu_adapter: Option<GpuAdapterSummary>,
    tile_cache: renderer::TileCache,
    gpu_pick_state: SharedGpuPickState,
    gpu_upload_state: SharedGpuUploadState,
    gpu_pick_serial: u64,
    collab: Option<CollabClient>,
    cell_name_drafts: BTreeMap<CellId, String>,
    instance_name_drafts: BTreeMap<(CellId, InstanceId), String>,
    layer_name_drafts: BTreeMap<LayerId, String>,
    mes: FabMesData,
    selected_mes_lot: Option<LotId>,
    show_mes_panel: bool,
    mes_operator: String,
    recipe_panel: RecipeManagerPanel,
    equipment_sim: EquipmentSimulator,
    selected_equipment_tool: Option<EquipmentToolId>,
    equipment_recipe_drafts: BTreeMap<EquipmentToolId, EquipmentRecipeId>,
    last_equipment_tick: Instant,
    logged_3d_shape_cap: bool,
    logged_3d_cpu_face_cap: bool,
    last_3d_ui_frame_at: Option<Instant>,
    smoothed_3d_ui_frame_ms: Option<f64>,
    gpu_frame_pacing: SharedGpuFramePacingState,
    cached_3d_scene: Option<Cached3dScene>,
    benchmark_3d: Option<Benchmark3dState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupView {
    Workflow,
    Layout2d,
    Layout3d,
    MaskPrep,
    LayoutDiff,
    FabControl,
    Inventory,
    Maintenance,
    Environment,
    Scheduler,
    Safety,
    Traceability,
    Metrology,
    Yield,
    SpcFdc,
    ProcessFlow,
    ProcessControl,
    CrossSection,
    Experiment,
    Notebook,
}

impl StartupView {
    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "workflow" | "home" => Some(Self::Workflow),
            "layout" | "layout2d" | "mask-layout" => Some(Self::Layout2d),
            "3d" | "layout3d" | "viewport3d" => Some(Self::Layout3d),
            "reticle" | "mask-prep" | "maskprep" => Some(Self::MaskPrep),
            "diff" | "layout-diff" | "layoutdiff" => Some(Self::LayoutDiff),
            "fab" | "equipment" | "fab-control" => Some(Self::FabControl),
            "inventory" => Some(Self::Inventory),
            "maintenance" => Some(Self::Maintenance),
            "environment" => Some(Self::Environment),
            "dispatch" | "scheduler" => Some(Self::Scheduler),
            "safety" => Some(Self::Safety),
            "traceability" | "trace" => Some(Self::Traceability),
            "metrology" | "metro" => Some(Self::Metrology),
            "yield" => Some(Self::Yield),
            "spc" | "spc-fdc" | "spcfdc" => Some(Self::SpcFdc),
            "process-flow" | "processflow" | "flow" => Some(Self::ProcessFlow),
            "r2r" | "process-control" | "processcontrol" => Some(Self::ProcessControl),
            "cross-section" | "crosssection" | "xsec" => Some(Self::CrossSection),
            "doe" | "experiment" => Some(Self::Experiment),
            "notebook" | "notes" => Some(Self::Notebook),
            _ => None,
        }
    }

    fn view_mode(self) -> ViewMode {
        match self {
            Self::Workflow => ViewMode::Workflow,
            Self::Layout2d => ViewMode::Layout2d,
            Self::Layout3d => ViewMode::Layout3d,
            Self::MaskPrep => ViewMode::MaskPrep,
            Self::LayoutDiff => ViewMode::LayoutDiff,
            Self::FabControl => ViewMode::FabControl,
            Self::Inventory => ViewMode::Inventory,
            Self::Maintenance => ViewMode::Maintenance,
            Self::Environment => ViewMode::Environment,
            Self::Scheduler => ViewMode::Scheduler,
            Self::Safety => ViewMode::Safety,
            Self::Traceability => ViewMode::Traceability,
            Self::Metrology => ViewMode::Metrology,
            Self::Yield => ViewMode::Yield,
            Self::SpcFdc => ViewMode::SpcFdc,
            Self::ProcessFlow => ViewMode::ProcessFlow,
            Self::ProcessControl => ViewMode::ProcessControl,
            Self::CrossSection => ViewMode::CrossSection,
            Self::Experiment => ViewMode::Experiment,
            Self::Notebook => ViewMode::Notebook,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StartupOptions {
    pub demo_workspace: bool,
    pub stress_count: Option<usize>,
    pub hierarchy_demo: bool,
    pub view_3d: bool,
    pub view_mode: Option<StartupView>,
    pub benchmark_3d: Option<Benchmark3dOptions>,
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
            document: blank_layout_document("Untitled layout"),
            mes: empty_mes_data(),
            yield_analysis: empty_yield_analysis(),
            wafer_map: empty_wafer_map(),
            recipe_catalog: RecipeCatalog::default(),
            genealogy: LotGenealogy::new(),
            inventory: Inventory::default(),
            maintenance: MaintenanceModel::default(),
            environment: CleanroomEnvironment::default(),
            scheduler: SchedulerPanel::empty().schedule().clone(),
            safety: SafetySystem::default(),
            experiment_plan: ExperimentPlannerPanel::empty().plan().clone(),
            process_control: ProcessControlModel::default(),
            process_flow: ProcessFlowModel::default(),
            lab_notebook: LabNotebook::default(),
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
            inventory: Inventory::sample(),
            maintenance: MaintenanceModel::sample(),
            environment: CleanroomEnvironment::sample(),
            scheduler: DispatchSchedule::sample(),
            safety: SafetySystem::simulated_demo(),
            experiment_plan: ExperimentPlan::sample(),
            process_control: ProcessControlModel::from_yield_analysis(&yield_analysis),
            process_flow: ProcessFlowModel::sample(),
            lab_notebook: LabNotebook::sample(),
            equipment: EquipmentSimulator::demo_fab(),
        }
    }
}

fn blank_layout_document(name: impl Into<String>) -> Document {
    let mut document = Document::new(name);
    document.layers.clear();
    document.next_layer_id = 1;
    document
}

fn empty_mes_data() -> FabMesData {
    FabMesData {
        routes: BTreeMap::new(),
        lots: BTreeMap::new(),
        travelers: BTreeMap::new(),
    }
}

fn default_nav_rail_modes() -> BTreeSet<ViewMode> {
    ViewMode::ALL.into_iter().collect()
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
    let zoom = options.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
    if width != options.width {
        warn!(
            requested_width = options.width,
            effective_width = width,
            "offscreen render width was outside the supported range"
        );
    }
    if height != options.height {
        warn!(
            requested_height = options.height,
            effective_height = height,
            "offscreen render height was outside the supported range"
        );
    }
    if (zoom - options.zoom).abs() > f32::EPSILON {
        warn!(
            requested_zoom = options.zoom,
            effective_zoom = zoom,
            "offscreen render zoom was outside the supported range"
        );
    }
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
        if selected_yield_lot.is_empty() {
            warn!("no yield lots available at startup; selected yield lot defaults to empty");
        }
        let selected_yield_wafer = yield_analysis
            .wafer_ids_for_lot(&selected_yield_lot)
            .first()
            .cloned()
            .unwrap_or_default();
        if selected_yield_wafer.is_empty() {
            warn!("no yield wafers available at startup; selected yield wafer defaults to empty");
        }
        let user_id = Uuid::new_v4();
        let gpu_adapter = cc.wgpu_render_state.as_ref().map(|render_state| {
            let info = render_state.adapter.get_info();
            if info.device_type == egui_wgpu::wgpu::DeviceType::Cpu {
                warn!(
                    backend = ?info.backend,
                    adapter = %info.name,
                    "wgpu selected a CPU adapter; 3D benchmark results will not reflect hardware GPU performance"
                );
            }
            GpuAdapterSummary::from_wgpu_info(info)
        });
        let mut loro_log = LoroCrdtLog::new(user_id).expect("create Loro CRDT log");
        if document_object_count(&document) <= MAX_LORO_SEED_OBJECTS {
            loro_log
                .seed_document_objects(&document)
                .expect("seed Loro object store");
        } else {
            warn!(
                object_count = document_object_count(&document),
                max_seed_objects = MAX_LORO_SEED_OBJECTS,
                "document exceeds Loro startup seed budget"
            );
        }
        let active_layer = document
            .layer_by_process(ProcessLayer::Metal1)
            .unwrap_or(LayerId(1));
        if document.layer_by_process(ProcessLayer::Metal1).is_none() {
            warn!("Metal1 layer missing at startup; active layer defaults to LayerId(1)");
        }
        let active_technology = 0;
        let rules = rule_deck_for_document(&document, &technologies[active_technology]);
        let index = build_layout_index(&document);
        let layout_bounds_cache = index.bounds();
        let violations = run_drc(&document, &rules);
        let connectivity =
            connectivity_report_for_document(&document, &technologies[active_technology]);
        let mes = dataset.mes;
        let selected_mes_lot = mes.lots.keys().next().cloned();
        let equipment_sim = dataset.equipment;
        let selected_equipment_tool = equipment_sim.tools().next().map(|tool| tool.id.clone());
        let app_context = AppContext::from_selections(
            selected_mes_lot.as_ref(),
            &selected_yield_lot,
            &selected_yield_wafer,
            selected_equipment_tool.as_ref(),
        );
        let workflow_panel = WorkflowPanel::default();
        let mask_panel = MaskPrepPanel::new(&document, &mes, selected_mes_lot.as_ref());
        let layout_diff_panel = LayoutDiffPanel::default();
        let inventory_panel = InventoryPanel::from_inventory(dataset.inventory);
        let maintenance_panel = MaintenancePanel::from_model(dataset.maintenance);
        let environment_panel = EnvironmentPanel::from_model(dataset.environment);
        let scheduler_panel = SchedulerPanel::from_schedule(dataset.scheduler);
        let safety_panel = SafetyPanel::from_model(dataset.safety);
        let spc_fdc_panel = SpcFdcPanel::new();
        let process_flow_panel = ProcessFlowPanel::from_model(dataset.process_flow);
        let process_control_panel = ProcessControlPanel::from_model(dataset.process_control);
        let cross_section_panel = CrossSectionPanel::sample();
        let genealogy_panel = GenealogyPanel::from_genealogy(dataset.genealogy);
        let experiment_panel = ExperimentPlannerPanel::from_plan(dataset.experiment_plan);
        let notebook_panel = LabNotebookPanel::from_notebook(dataset.lab_notebook);
        let mut app = Self {
            document,
            layout_source: DataSource::Blank,
            fabos_source: DataSource::Blank,
            index,
            layout_bounds_cache,
            yield_analysis,
            selected_yield_lot,
            selected_yield_wafer,
            app_context,
            workflow_panel,
            mask_panel,
            layout_diff_panel,
            inventory_panel,
            maintenance_panel,
            environment_panel,
            scheduler_panel,
            safety_panel,
            spc_fdc_panel,
            process_flow_panel,
            process_control_panel,
            cross_section_panel,
            genealogy_panel,
            experiment_panel,
            notebook_panel,
            technologies,
            active_technology,
            rules,
            violations,
            connectivity,
            show_hidden_markers: false,
            show_waived_markers: true,
            drc_marker_filter: String::new(),
            connectivity_filter: String::new(),
            nav_rail_modes: default_nav_rail_modes(),
            show_command_palette: false,
            show_sidebar_modules: false,
            show_inspector_drawer: false,
            show_layers_drawer: false,
            command_palette_query: String::new(),
            viewport_fullscreen: false,
            selected: BTreeSet::new(),
            selected_occurrence: None,
            active_layer,
            tool: Tool::Select,
            view_mode: ViewMode::Workflow,
            wafer_map: dataset.wafer_map,
            metrology_map_mode: MetrologyMapMode::ValueMap,
            metrology_kind: MeasurementKind::ThicknessNm,
            selected_die: None,
            metrology_failed_only: false,
            zoom: 0.075,
            pan: Vec2::ZERO,
            camera_3d: Camera3d::default(),
            flycam_captured: false,
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
            gpu_adapter,
            tile_cache: renderer::TileCache::default(),
            gpu_pick_state: Arc::new(Mutex::new(GpuPickState::default())),
            gpu_upload_state: Arc::new(Mutex::new(GpuUploadStats::default())),
            gpu_pick_serial: 0,
            collab: None,
            cell_name_drafts: BTreeMap::new(),
            instance_name_drafts: BTreeMap::new(),
            layer_name_drafts: BTreeMap::new(),
            mes,
            selected_mes_lot,
            show_mes_panel: true,
            mes_operator: "op.demo".to_string(),
            recipe_panel: RecipeManagerPanel::from_catalog(dataset.recipe_catalog),
            equipment_sim,
            selected_equipment_tool,
            equipment_recipe_drafts: BTreeMap::new(),
            last_equipment_tick: Instant::now(),
            logged_3d_shape_cap: false,
            logged_3d_cpu_face_cap: false,
            last_3d_ui_frame_at: None,
            smoothed_3d_ui_frame_ms: None,
            gpu_frame_pacing: Arc::new(Mutex::new(GpuFramePacingState::default())),
            cached_3d_scene: None,
            benchmark_3d: None,
        };
        app.reset_3d_camera_to_document();
        app
    }

    pub fn new_with_options(cc: &eframe::CreationContext<'_>, options: StartupOptions) -> Self {
        let mut app = Self::new(cc);
        if options.demo_workspace {
            app.apply_workspace_dataset(
                WorkspaceDataset::demo(),
                DataSource::Demo,
                DataSource::Demo,
                "test workspace: demo",
            );
        }
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
            app.layout_source = DataSource::Generated(format!("{count} layout stress"));
            app.status = format!("test scene: {count} layout objects");
        }
        app.reset_3d_camera_to_document();
        if let Some(zoom) = options.zoom {
            let clamped = zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
            if (clamped - zoom).abs() > f32::EPSILON {
                warn!(
                    requested_zoom = zoom,
                    effective_zoom = clamped,
                    "startup zoom was outside the supported range"
                );
            }
            app.zoom = clamped;
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
        if let Some(view_mode) = options.view_mode.map(StartupView::view_mode) {
            app.view_mode = view_mode;
            if matches!(view_mode, ViewMode::Layout3d) {
                app.reset_3d_camera_to_document();
            }
        } else if options.view_3d {
            app.view_mode = ViewMode::Layout3d;
            app.reset_3d_camera_to_document();
        }
        if options.show_options {
            app.show_options = true;
        }
        if let Some(benchmark) = options.benchmark_3d {
            app.benchmark_3d = Some(Benchmark3dState::new(benchmark));
            app.show_diagnostics = false;
            app.show_mes_panel = false;
        }
        app
    }

    fn set_warn_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        warn!("{message}");
        self.status = message;
    }

    fn set_error_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        error!("{message}");
        self.status = message;
    }

    fn rebuild_indexes(&mut self) {
        self.index = build_layout_index(&self.document);
        self.layout_bounds_cache = self.index.bounds();
        self.cached_3d_scene = None;
        self.rebuild_connectivity();
    }

    fn rerun_drc(&mut self) {
        if let Some(message) = drc_skip_message(&self.document) {
            warn!("{message}");
            self.violations.clear();
            self.perf.drc_ms = 0.0;
            self.last_drc_run = Instant::now();
            self.status = message;
            return;
        }
        let started = Instant::now();
        self.violations = run_drc(&self.document, &self.rules);
        self.perf.drc_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.last_drc_run = Instant::now();
    }

    fn rerun_drc_for_dirty_region(&mut self, dirty_region: Option<Rect>) {
        if let Some(message) = drc_skip_message(&self.document) {
            warn!("{message}");
            self.violations.clear();
            self.perf.drc_ms = 0.0;
            self.last_drc_run = Instant::now();
            self.status = message;
            return;
        }
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
        if self.document.marker_states.is_empty() {
            return self.violations.len();
        }
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
        let effective = self
            .active_technology
            .min(self.technologies.len().saturating_sub(1));
        if effective != self.active_technology {
            warn!(
                active_technology = self.active_technology,
                effective_technology = effective,
                "active technology index exceeded available technologies"
            );
        }
        &self.technologies[effective]
    }

    fn snap_grid(&self) -> Coord {
        if self.document.grid < 1 {
            warn!(
                document_grid = self.document.grid,
                "document grid was below supported range; using 1 dbu"
            );
        }
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
        let requested_grid = grid;
        let grid = requested_grid.clamp(1, 10_000_000);
        if grid != requested_grid {
            warn!(
                requested_grid,
                effective_grid = grid,
                "document grid was outside the supported range"
            );
        }
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
        let effective_interval = self.settings.autosave_interval_seconds.max(5);
        if effective_interval != self.settings.autosave_interval_seconds {
            warn!(
                configured_seconds = self.settings.autosave_interval_seconds,
                effective_seconds = effective_interval,
                "autosave interval was below supported range"
            );
        }
        let interval = Duration::from_secs(effective_interval);
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
                self.rules = empty_rule_deck(self.document.grid);
                self.set_error_status(format!(
                    "technology rule load failed: {err}; DRC rules disabled"
                ));
            }
        }
    }

    fn reset_active_layer(&mut self) {
        let fallback = self
            .document
            .layer_by_process(ProcessLayer::Metal1)
            .is_none();
        self.active_layer = self
            .document
            .layer_by_process(ProcessLayer::Metal1)
            .or_else(|| self.document.layers.keys().next().copied())
            .unwrap_or(LayerId(1));
        if fallback {
            warn!(
                active_layer = self.active_layer.0,
                "Metal1 layer missing; active layer selected from fallback"
            );
        }
    }

    fn apply_current_technology_to_document(&mut self) {
        let technology = self.current_technology().clone();
        if let Err(err) = self.document.apply_technology(&technology) {
            self.set_error_status(format!("technology load failed: {err}"));
        }
        self.reset_active_layer();
        self.rebuild_rules_from_technology();
    }

    fn switch_technology(&mut self, index: usize) {
        if index >= self.technologies.len() || index == self.active_technology {
            if index >= self.technologies.len() {
                warn!(
                    requested_index = index,
                    technology_count = self.technologies.len(),
                    "requested technology index exceeded available technologies"
                );
                self.set_warn_status(format!(
                    "technology index {index} is out of range for {} technologies",
                    self.technologies.len()
                ));
            }
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
        self.cached_3d_scene = None;
        self.perf.tile_dirty_count = 0;
    }

    fn clear_edit_drafts(&mut self) {
        self.cell_name_drafts.clear();
        self.instance_name_drafts.clear();
        self.layer_name_drafts.clear();
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
            Operation::AddLayer { .. } | Operation::DeleteLayer { .. } => {
                invalidation.clear_all = true;
            }
            Operation::Cursor { .. } => {}
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
            warn!(
                actor = %operation.id.actor,
                counter = operation.id.counter,
                "ignored duplicate CRDT operation"
            );
            return false;
        }
        let invalidation = self.render_invalidation_for_operation(&operation.operation);
        if self.document.apply_crdt_operation(operation) != CrdtApplyResult::Applied {
            warn!("CRDT operation did not apply and was ignored");
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
                self.set_error_status(format!("failed to append Loro operation: {err}"));
                return false;
            }
        };
        if !self.apply_crdt_operation_without_history(operation) {
            warn!("local operation was not applied after Loro append");
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
        let url = std::env::var("FABRICAD_SYNC_URL").unwrap_or_else(|err| {
            warn!(
                error = %err,
                "FABRICAD_SYNC_URL missing or invalid Unicode; using local collaboration URL"
            );
            "ws://127.0.0.1:4141/ws".to_string()
        });
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
					let message = format!("failed to connect to {url}");
					error!("{message}");
					if let Err(err) = inbound_tx.send(ServerMessage::Error { message }) {
						warn!(error = %err, "failed to report collaboration connection error");
					}
					return;
				};
				let (mut write, mut read) = socket.split();
				if send_client_message(&mut write, &ClientMessage::Join { user }).await.is_err() {
					error!("failed to send collaboration join message");
					return;
				}
				loop {
					tokio::select! {
						Some(message) = outbound_rx.recv() => {
							if send_client_message(&mut write, &message).await.is_err() {
								let error_message = "collaboration socket closed while sending".to_string();
								error!("{error_message}");
								if let Err(err) = inbound_tx.send(ServerMessage::Error {
									message: "collaboration socket closed while sending".to_string(),
								}) {
									warn!(error = %err, "failed to report collaboration send closure");
								}
								break;
							}
						}
						incoming = read.next() => {
							let Some(incoming) = incoming else {
								let message = "collaboration socket closed".to_string();
								warn!("{message}");
								if let Err(err) = inbound_tx.send(ServerMessage::Error { message }) {
									warn!(error = %err, "failed to report collaboration socket closure");
								}
								break;
							};
							match incoming {
								Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
									match serde_json::from_str::<ServerMessage>(&text) {
										Ok(message) => {
											if let Err(err) = inbound_tx.send(message) {
												warn!(error = %err, "failed to enqueue collaboration message");
											}
										}
										Err(err) => {
											let message = format!("invalid collaboration message: {err}");
											error!("{message}");
											if let Err(err) = inbound_tx.send(ServerMessage::Error { message }) {
												warn!(error = %err, "failed to report invalid collaboration message");
											}
										}
									}
								}
								Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
								Ok(_) => {}
								Err(err) => {
									let message = format!("collaboration socket error: {err}");
									error!("{message}");
									if let Err(err) = inbound_tx.send(ServerMessage::Error { message }) {
										warn!(error = %err, "failed to report collaboration socket error");
									}
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
                self.set_error_status(format!("failed to create collaboration socket for {url}"));
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
                error!("collaboration server sent a non-text message");
                inbound_for_message.borrow_mut().push(ServerMessage::Error {
                    message: "collaboration server sent a non-text message".to_string(),
                });
                return;
            };
            match serde_json::from_str::<ServerMessage>(&text) {
                Ok(message) => inbound_for_message.borrow_mut().push(message),
                Err(err) => {
                    error!("invalid collaboration message: {err}");
                    inbound_for_message.borrow_mut().push(ServerMessage::Error {
                        message: format!("invalid collaboration message: {err}"),
                    });
                }
            }
        }) as Box<dyn FnMut(_)>);
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let inbound_for_error = Rc::clone(&inbound);
        let on_error = Closure::wrap(Box::new(move |_event: web_sys::ErrorEvent| {
            error!("collaboration socket error");
            inbound_for_error.borrow_mut().push(ServerMessage::Error {
                message: "collaboration socket error".to_string(),
            });
        }) as Box<dyn FnMut(_)>);
        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let inbound_for_close = Rc::clone(&inbound);
        let on_close = Closure::wrap(Box::new(move |event: web_sys::CloseEvent| {
            warn!(code = event.code(), "collaboration socket closed");
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
                                .map(|err| {
                                    error!("failed to materialize Loro snapshot objects: {err}");
                                    format!("; materialize failed: {err}")
                                })
                        };
                        self.loro_log = log;
                        format!(
                            "collaboration snapshot applied{}",
                            materialize_status.unwrap_or_default()
                        )
                    }
                    Err(err) => {
                        error!("failed to apply Loro snapshot: {err}");
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
                        self.set_error_status(format!("failed to import Loro update: {err}"));
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
                error!("{message}");
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
            if let Err(err) = collab.outbound.send(message) {
                warn!(error = %err, "failed to enqueue outbound collaboration message");
            }
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

    fn add_shape(&mut self, layer: LayerId, kind: ShapeKind) -> Option<ShapeId> {
        self.add_shape_with_metadata(layer, kind, None, None)
    }

    fn add_shape_with_metadata(
        &mut self,
        layer: LayerId,
        kind: ShapeKind,
        net: Option<NetId>,
        name: Option<String>,
    ) -> Option<ShapeId> {
        if !self.document.layers.contains_key(&layer) {
            self.status = "add a layer before drawing".to_string();
            return None;
        }
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
        Some(id)
    }

    fn add_layer_from_ui(&mut self) {
        let id = LayerId(self.document.next_layer_id);
        let display_order = self
            .document
            .layers
            .values()
            .map(|layer| layer.display_order)
            .max()
            .unwrap_or(0)
            + 10;
        let palette = [
            [0.18, 0.56, 0.82, 1.0],
            [0.88, 0.37, 0.25, 1.0],
            [0.38, 0.68, 0.31, 1.0],
            [0.86, 0.62, 0.18, 1.0],
            [0.62, 0.44, 0.78, 1.0],
            [0.28, 0.68, 0.66, 1.0],
        ];
        let layer = Layer {
            id,
            name: format!("Layer {}", self.document.layers.len() + 1),
            process: ProcessLayer::Metal1,
            purpose: "custom".to_string(),
            color: palette[id.0 as usize % palette.len()],
            display_order,
            gds_layer: u16::try_from(id.0).ok(),
            gds_datatype: 0,
            gds_texttype: 0,
            visible: true,
            locked: false,
        };
        let redo = Operation::AddLayer {
            layer: layer.clone(),
        };
        let undo = Operation::DeleteLayer { id };
        self.apply_with_history(redo, undo);
        self.active_layer = id;
        self.status = format!("added {}", layer.name);
    }

    fn remove_layer_by_id(&mut self, id: LayerId) {
        let Some(layer) = self.document.layers.get(&id).cloned() else {
            self.reset_active_layer();
            self.status = "active layer was missing".to_string();
            return;
        };
        let shapes = self
            .document
            .shapes
            .values()
            .filter(|shape| shape.layer == id)
            .collect::<Vec<_>>();
        let shape_count = shapes.len();
        let mut undo_operations = Vec::with_capacity(shape_count + 1);
        undo_operations.push(Operation::AddLayer {
            layer: layer.clone(),
        });
        undo_operations.extend(
            shapes
                .into_iter()
                .map(|shape| Operation::AddShape { shape }),
        );
        let redo = Operation::DeleteLayer { id };
        let undo = Operation::Batch {
            operations: undo_operations,
        };
        self.apply_with_history(redo, undo);
        self.selected.clear();
        self.selected_occurrence = None;
        self.drag_shape = None;
        self.drag_vertex = None;
        self.drag_edge = None;
        self.reset_active_layer();
        self.status = format!(
            "removed {} and {} shape{}",
            layer.name,
            shape_count,
            if shape_count == 1 { "" } else { "s" }
        );
    }

    fn rename_layer(&mut self, id: LayerId, name: String) {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            self.status = "layer name cannot be empty".to_string();
            return;
        }
        let Some(layer) = self.document.layers.get_mut(&id) else {
            self.status = "layer was missing".to_string();
            return;
        };
        if layer.name != trimmed {
            layer.name = trimmed.to_string();
            self.status = format!("renamed layer {}", layer.name);
        }
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
        let Some(old_shape) = self.document.shapes.get(&id) else {
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
        let Some(old_shape) = self.document.shapes.get(&id) else {
            return;
        };
        let Some(new_shape) = shape_with_inserted_vertex(&old_shape, edge, position) else {
            self.status = "vertex insertion needs a polygon or path edge".to_string();
            return;
        };
        self.replace_shape(id, old_shape, new_shape, "inserted vertex");
    }

    fn delete_vertex(&mut self, id: ShapeId, vertex: usize) {
        let Some(old_shape) = self.document.shapes.get(&id) else {
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
        let Some(old_shape) = self.document.shapes.get(&id) else {
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
        let Some(old_instance) = self.document.instance(parent, id) else {
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
        let Some(mut instance) = self.document.instance(info.parent, info.id) else {
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
            let Some(instance) = self.document.instance(info.parent, info.id) else {
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

    fn create_empty_cell(&mut self) {
        let cell_id = self.document.allocate_cell_id();
        let cell = Cell::new(cell_id, format!("cell {}", cell_id.0));
        let redo = Operation::AddCell { cell: cell.clone() };
        let undo = Operation::DeleteCell { id: cell_id };
        self.apply_with_history(redo, undo);
        self.status = format!("created {}", cell.name);
    }

    fn delete_cell(&mut self, id: CellId) {
        if id == self.document.top_cell {
            self.status = "cannot delete the top cell".to_string();
            return;
        }
        let instance_count = self.cell_instance_count(id);
        if instance_count > 0 {
            self.status = format!("cannot delete cell with {instance_count} instance(s)");
            return;
        }
        let Some(cell) = self.document.cell(id).cloned() else {
            self.status = "cell no longer exists".to_string();
            return;
        };
        let redo = Operation::DeleteCell { id };
        let undo = Operation::AddCell { cell: cell.clone() };
        self.apply_with_history(redo, undo);
        self.cell_name_drafts.remove(&id);
        self.selected.clear();
        self.selected_occurrence = None;
        self.status = format!("deleted {}", cell.name);
    }

    fn cell_instance_count(&self, id: CellId) -> usize {
        self.document
            .cells
            .values()
            .map(|cell| {
                cell.instances
                    .values()
                    .filter(|instance| instance.cell == id)
                    .count()
            })
            .sum()
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
            .filter_map(|id| self.document.shapes.get(id))
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
            inventory: self.inventory_panel.inventory().clone(),
            maintenance: self.maintenance_panel.model().clone(),
            environment: self.environment_panel.model().clone(),
            scheduler: self.scheduler_panel.schedule().clone(),
            safety: self.safety_panel.model().clone(),
            experiment_plan: self.experiment_panel.plan().clone(),
            process_control: self.process_control_panel.model().clone(),
            process_flow: self.process_flow_panel.model().clone(),
            lab_notebook: self.notebook_panel.notebook().clone(),
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
        self.workflow_panel = WorkflowPanel::default();
        self.mask_panel =
            MaskPrepPanel::new(&self.document, &self.mes, self.selected_mes_lot.as_ref());
        self.layout_diff_panel = LayoutDiffPanel::default();
        self.inventory_panel = InventoryPanel::from_inventory(dataset.inventory);
        self.maintenance_panel = MaintenancePanel::from_model(dataset.maintenance);
        self.environment_panel = EnvironmentPanel::from_model(dataset.environment);
        self.scheduler_panel = SchedulerPanel::from_schedule(dataset.scheduler);
        self.safety_panel = SafetyPanel::from_model(dataset.safety);
        self.spc_fdc_panel = SpcFdcPanel::new();
        self.process_flow_panel = ProcessFlowPanel::from_model(dataset.process_flow);
        self.process_control_panel = ProcessControlPanel::from_model(dataset.process_control);
        self.cross_section_panel = CrossSectionPanel::sample();
        self.genealogy_panel = GenealogyPanel::from_genealogy(dataset.genealogy);
        self.experiment_panel = ExperimentPlannerPanel::from_plan(dataset.experiment_plan);
        self.notebook_panel = LabNotebookPanel::from_notebook(dataset.lab_notebook);
        self.recipe_panel = RecipeManagerPanel::from_catalog(dataset.recipe_catalog);
        self.equipment_sim = dataset.equipment;
        self.selected_equipment_tool = self
            .equipment_sim
            .tools()
            .next()
            .map(|tool| tool.id.clone());
        self.app_context = AppContext::from_selections(
            self.selected_mes_lot.as_ref(),
            &self.selected_yield_lot,
            &self.selected_yield_wafer,
            self.selected_equipment_tool.as_ref(),
        );
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
        if self.selected_yield_lot.is_empty() {
            warn!("yield dataset has no lots; selected yield lot defaults to empty");
        }
        self.selected_yield_wafer = self
            .yield_analysis
            .wafer_ids_for_lot(&self.selected_yield_lot)
            .first()
            .cloned()
            .unwrap_or_default();
        if self.selected_yield_wafer.is_empty() {
            warn!("selected yield lot has no wafers; selected yield wafer defaults to empty");
        }
    }

    fn ensure_app_context(&mut self) {
        let lot_ids = self.context_lot_ids();
        if !lot_ids.is_empty()
            && self
                .app_context
                .focus_lot()
                .is_none_or(|lot_id| !lot_ids.iter().any(|candidate| candidate == lot_id))
        {
            let lot_id = lot_ids
                .iter()
                .find(|lot_id| lot_id.as_str() == "L-00042")
                .cloned()
                .or_else(|| {
                    self.selected_mes_lot
                        .as_ref()
                        .map(|lot_id| lot_id.as_str().to_string())
                })
                .or_else(|| lot_ids.first().cloned());
            if let Some(lot_id) = lot_id {
                self.apply_focus_object(FabObjectRef::lot(lot_id), false);
            }
        }

        let tool_ids = self.context_tool_ids();
        if !tool_ids.is_empty()
            && self
                .app_context
                .focus_tool()
                .is_none_or(|tool_id| !tool_ids.iter().any(|candidate| candidate == tool_id))
        {
            if let Some(tool_id) = self
                .selected_equipment_tool
                .as_ref()
                .map(|tool_id| tool_id.as_str().to_string())
                .or_else(|| tool_ids.first().cloned())
            {
                self.app_context.set_tool(tool_id);
            }
        }

        let recipe_ids = self.context_recipe_ids();
        if !recipe_ids.is_empty()
            && self
                .app_context
                .focus_recipe()
                .is_none_or(|recipe_id| !recipe_ids.iter().any(|candidate| candidate == recipe_id))
        {
            if let Some(recipe_id) = recipe_ids.first().cloned() {
                self.app_context.set_recipe(recipe_id);
            }
        }
    }

    fn context_lot_ids(&self) -> Vec<String> {
        let mut ids = BTreeSet::new();
        ids.extend(self.mes.lots.keys().map(|id| id.as_str().to_string()));
        ids.extend(
            self.scheduler_panel
                .schedule()
                .lots
                .iter()
                .map(|lot| lot.id.as_str().to_string()),
        );
        ids.extend(self.yield_analysis.lots.iter().map(|lot| lot.id.clone()));
        ids.extend(
            self.notebook_panel
                .notebook()
                .entries
                .iter()
                .flat_map(|entry| entry.links.lots.iter().map(|lot| lot.as_str().to_string())),
        );
        ids.into_iter().collect()
    }

    fn context_tool_ids(&self) -> Vec<String> {
        let mut ids = BTreeSet::new();
        ids.extend(
            self.equipment_sim
                .tools()
                .map(|tool| tool.id.as_str().to_string()),
        );
        ids.extend(
            self.scheduler_panel
                .schedule()
                .tools
                .iter()
                .map(|tool| tool.id.as_str().to_string()),
        );
        ids.extend(self.mes.routes.values().flat_map(|route| {
            route.steps.iter().flat_map(|step| {
                step.eligible_tools
                    .iter()
                    .map(|tool| tool.as_str().to_string())
            })
        }));
        ids.into_iter().collect()
    }

    fn context_recipe_ids(&self) -> Vec<String> {
        let mut ids = BTreeSet::new();
        ids.extend(
            self.recipe_panel
                .catalog()
                .recipes
                .keys()
                .map(|id| id.as_str().to_string()),
        );
        ids.extend(
            self.process_flow_panel
                .model()
                .route
                .nodes
                .iter()
                .filter_map(|node| {
                    node.recipe
                        .as_ref()
                        .map(|binding| binding.recipe_id.as_str().to_string())
                }),
        );
        ids.extend(self.mes.routes.values().flat_map(|route| {
            route
                .steps
                .iter()
                .map(|step| step.required_recipe.as_str().to_string())
        }));
        ids.into_iter().collect()
    }

    fn set_focus_object(&mut self, object: FabObjectRef) {
        self.apply_focus_object(object, true);
    }

    fn apply_focus_object(&mut self, object: FabObjectRef, announce: bool) {
        match object.kind {
            FabObjectKind::Lot => {
                let lot_id = object.id;
                self.app_context.set_lot(lot_id.clone());
                self.workflow_panel.set_focus_lot(lot_id.clone());
                let mes_lot_id = LotId::new(lot_id.clone());
                if self.mes.lots.contains_key(&mes_lot_id) {
                    self.selected_mes_lot = Some(mes_lot_id);
                }
                if self.yield_analysis.lots.iter().any(|lot| lot.id == lot_id) {
                    self.selected_yield_lot = lot_id.clone();
                    let wafer_ids = self.yield_analysis.wafer_ids_for_lot(&lot_id);
                    if !wafer_ids.contains(&self.selected_yield_wafer) {
                        self.selected_yield_wafer = wafer_ids.first().cloned().unwrap_or_default();
                    }
                }
            }
            FabObjectKind::Wafer => {
                self.app_context.set_wafer(object.id.clone());
                self.selected_yield_wafer = object.id;
            }
            FabObjectKind::Tool => {
                let tool_id = object.id;
                self.app_context.set_tool(tool_id.clone());
                if self
                    .equipment_sim
                    .tools()
                    .any(|tool| tool.id.as_str() == tool_id)
                {
                    self.selected_equipment_tool = Some(EquipmentToolId::new(tool_id));
                }
            }
            FabObjectKind::Recipe => {
                let recipe_id = object.id;
                self.app_context.set_recipe(recipe_id.clone());
                self.recipe_panel.select_recipe_id(&recipe_id);
            }
            FabObjectKind::MaterialLot => {
                self.app_context.set_material_lot(object.id);
            }
            _ => {
                self.app_context.active = Some(object);
            }
        }
        if announce {
            self.status = format!("context {}", self.app_context.active_label());
        }
    }

    fn sync_workflow_focus_from_context(&mut self) {
        if let Some(lot_id) = self.app_context.focus_lot() {
            if self.workflow_panel.focus_lot() != lot_id {
                self.workflow_panel.set_focus_lot(lot_id.to_string());
            }
        }
    }

    fn sync_context_from_workflow(&mut self) {
        let lot_id = self.workflow_panel.focus_lot().to_string();
        if !lot_id.is_empty() && self.app_context.focus_lot() != Some(lot_id.as_str()) {
            self.apply_focus_object(FabObjectRef::lot(lot_id), false);
        }
    }

    fn context_mes_action(&self) -> Option<(LotId, OperatorAction, String)> {
        let lot_id = LotId::new(self.app_context.focus_lot()?.to_string());
        let lot = self.mes.lots.get(&lot_id)?;
        let traveler = self.mes.travelers.get(&lot_id)?;
        let route = self.mes.routes.get(&lot.route_id)?;
        match traveler.status {
            TravelerStatus::WaitingForStep => {
                let step = traveler.current_step(route)?;
                let tool_id = step
                    .primary_tool()
                    .cloned()
                    .unwrap_or_else(|| ToolId::new("NO-ELIGIBLE-TOOL"));
                Some((
                    lot_id,
                    OperatorAction::StartStep {
                        step_id: step.id.clone(),
                        tool_id,
                        tool_class: step.required_tool_class,
                        recipe_id: step.required_recipe.clone(),
                        operator: self.mes_action_operator(),
                    },
                    format!("Start {}", step.id),
                ))
            }
            TravelerStatus::Running => {
                let run = traveler.active_run.as_ref()?;
                Some((
                    lot_id,
                    OperatorAction::CompleteStep {
                        step_id: run.step_id.clone(),
                        operator: self.mes_action_operator(),
                    },
                    format!("Complete {}", run.step_id),
                ))
            }
            TravelerStatus::WaitingForSignoff => {
                let pending = traveler.pending_signoff.as_ref()?;
                Some((
                    lot_id,
                    OperatorAction::SignOff {
                        step_id: pending.run.step_id.clone(),
                        operator: self.mes_action_operator(),
                    },
                    format!("Sign off {}", pending.run.step_id),
                ))
            }
            TravelerStatus::OnHold => Some((
                lot_id,
                OperatorAction::ReleaseHold {
                    operator: self.mes_action_operator(),
                },
                "Release hold".to_string(),
            )),
            TravelerStatus::Complete | TravelerStatus::Scrapped => None,
        }
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
            self.set_error_status(format!("workspace save failed: {err}"));
            return;
        }
        match serde_json::to_string_pretty(&self.current_workspace_dataset())
            .and_then(|contents| fs::write(&path, contents).map_err(serde_json::Error::io))
        {
            Ok(()) => self.status = format!("saved workspace {WORKSPACE_PATH}"),
            Err(err) => self.set_error_status(format!("workspace save failed: {err}")),
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
            Err(err) => self.set_error_status(format!("workspace load failed: {err}")),
        }
    }

    fn load_demo_workspace(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.apply_workspace_dataset(
                WorkspaceDataset::demo(),
                DataSource::Demo,
                DataSource::Demo,
                "loaded built-in demo workspace",
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = PathBuf::from(DEMO_WORKSPACE_PATH);
            let dataset = match fs::read_to_string(&path)
                .map_err(serde_json::Error::io)
                .and_then(|contents| serde_json::from_str::<WorkspaceDataset>(&contents))
            {
                Ok(dataset) => dataset,
                Err(err) => {
                    warn!(
                        path = %path.display(),
                        error = %err,
                        "demo workspace load failed; regenerating demo workspace"
                    );
                    let dataset = WorkspaceDataset::demo();
                    if let Some(parent) = path.parent()
                        && let Err(err) = fs::create_dir_all(parent)
                    {
                        self.set_error_status(format!("demo workspace setup failed: {err}"));
                        return;
                    }
                    match serde_json::to_string_pretty(&dataset).and_then(|contents| {
                        fs::write(&path, contents).map_err(serde_json::Error::io)
                    }) {
                        Ok(()) => {}
                        Err(err) => {
                            self.set_error_status(format!("demo workspace setup failed: {err}"));
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
    }

    fn save_document(&mut self) {
        let path = PathBuf::from(SAVE_PATH);
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                self.set_error_status(format!("save failed: {err}"));
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
            Err(err) => self.set_error_status(format!("save failed: {err}")),
        }
    }

    fn autosave_document(&mut self) {
        let contents = match serde_json::to_string_pretty(&self.document) {
            Ok(contents) => contents,
            Err(err) => {
                self.set_error_status(format!("autosave failed: {err}"));
                return;
            }
        };

        #[cfg(target_arch = "wasm32")]
        {
            let Some(window) = web_sys::window() else {
                self.set_error_status("autosave failed: no browser window");
                return;
            };
            let storage = match window.local_storage() {
                Ok(Some(storage)) => storage,
                Ok(None) => {
                    self.set_error_status("autosave failed: local storage unavailable");
                    return;
                }
                Err(_) => {
                    self.set_error_status("autosave failed: local storage blocked");
                    return;
                }
            };
            match storage.set_item(WASM_AUTOSAVE_KEY, &contents) {
                Ok(()) => self.status = "autosaved to browser storage".to_string(),
                Err(_) => self.set_error_status("autosave failed: browser storage write failed"),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = PathBuf::from(SAVE_PATH);
            if let Some(parent) = path.parent()
                && let Err(err) = fs::create_dir_all(parent)
            {
                self.set_error_status(format!("autosave failed: {err}"));
                return;
            }
            match fs::write(&path, contents) {
                Ok(()) => self.status = format!("autosaved {SAVE_PATH}"),
                Err(err) => self.set_error_status(format!("autosave failed: {err}")),
            }
        }
    }

    fn restore_autosave_document(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(window) = web_sys::window() else {
                self.set_error_status("restore failed: no browser window");
                return;
            };
            let storage = match window.local_storage() {
                Ok(Some(storage)) => storage,
                Ok(None) => {
                    self.set_error_status("restore failed: local storage unavailable");
                    return;
                }
                Err(_) => {
                    self.set_error_status("restore failed: local storage blocked");
                    return;
                }
            };
            let contents = match storage.get_item(WASM_AUTOSAVE_KEY) {
                Ok(Some(contents)) => contents,
                Ok(None) => {
                    self.set_warn_status("restore failed: no browser autosave");
                    return;
                }
                Err(_) => {
                    self.set_error_status("restore failed: browser storage read failed");
                    return;
                }
            };
            match serde_json::from_str::<Document>(&contents) {
                Ok(document) => {
                    self.replace_document(document, "restored browser autosave");
                    self.layout_source = DataSource::File("browser autosave".to_string());
                }
                Err(err) => self.set_error_status(format!("restore failed: {err}")),
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
        if let Some(message) = drc_skip_message(&self.document) {
            warn!("{message}");
            self.status = format!("{label}; {message}");
        }
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
            Err(err) => self.set_error_status(format!("load failed: {err}")),
        }
    }

    fn export_gds_document(&mut self) {
        let technology = self.current_technology().clone();
        let bytes = match export_gdsii(&self.document, &technology) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.set_error_status(format!("GDS export failed: {err}"));
                return;
            }
        };
        let path = PathBuf::from(GDS_PATH);
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                self.set_error_status(format!("GDS export failed: {err}"));
                return;
            }
        }
        match fs::write(&path, bytes) {
            Ok(()) => self.status = format!("exported {GDS_PATH}"),
            Err(err) => self.set_error_status(format!("GDS export failed: {err}")),
        }
    }

    fn import_gds_document(&mut self) {
        let technology = self.current_technology().clone();
        let bytes = match fs::read(GDS_PATH) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.set_error_status(format!("GDS import failed: {err}"));
                return;
            }
        };
        match import_gdsii(&bytes, &technology) {
            Ok(document) => {
                self.replace_document(document, &format!("imported {GDS_PATH}"));
                self.layout_source = DataSource::File(GDS_PATH.to_string());
            }
            Err(err) => self.set_error_status(format!("GDS import failed: {err}")),
        }
    }

    fn make_stress_document(&mut self, count: usize) {
        self.replace_document(
            Document::stress(count),
            &format!("generated {count} layout objects"),
        );
        self.layout_source = DataSource::Generated(format!("{count} layout stress"));
        self.status = format!("generated {count} layout objects");
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
                let placed = self.add_shape_with_metadata(
                    self.active_layer,
                    ShapeKind::Path {
                        points: route.points,
                        width: 180,
                    },
                    route_net.net,
                    route_net.name,
                );
                if placed.is_some() {
                    self.status = if let Some(net_label) = net_label {
                        format!(
                            "route placed on {net_label} with {point_count} points after {visited} visited nodes"
                        )
                    } else {
                        format!(
                            "route placed with {point_count} points after {visited} visited nodes"
                        )
                    };
                }
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
            if (input.modifiers.ctrl || input.modifiers.command)
                && input.modifiers.shift
                && input.key_pressed(Key::P)
            {
                self.show_command_palette = true;
                self.command_palette_query.clear();
            }
            if input.key_pressed(Key::F11) {
                self.toggle_viewport_fullscreen(ctx);
            }
            if input.key_pressed(Key::Escape) && self.viewport_fullscreen {
                self.set_viewport_fullscreen(false, ctx);
            }
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
                if self.settings.single_key_shortcuts
                    && !input.modifiers.any()
                    && matches!(self.view_mode, ViewMode::Layout2d)
                {
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

        let running_count = tools
            .iter()
            .filter(|tool| tool.state == EquipmentToolState::Running)
            .count();
        let ready_count = tools
            .iter()
            .filter(|tool| {
                matches!(
                    tool.state,
                    EquipmentToolState::OnlineIdle | EquipmentToolState::Completed
                )
            })
            .count();
        let loaded_count = tools
            .iter()
            .filter(|tool| tool.state == EquipmentToolState::RecipeLoaded)
            .count();
        let alarm_count = tools
            .iter()
            .map(|tool| equipment_active_alarm_count(tool))
            .sum::<usize>();
        let critical_count = tools
            .iter()
            .flat_map(|tool| tool.active_alarms.iter())
            .filter(|alarm| alarm.active && alarm.severity == AlarmSeverity::Critical)
            .count();
        let sample_count = tools
            .iter()
            .map(|tool| tool.recent_sensors.len())
            .sum::<usize>();
        let selected_label = self
            .selected_equipment_tool
            .as_ref()
            .map(|tool_id| tool_id.as_str())
            .unwrap_or("none");
        let detail = format!(
            "Sim time: {} s | Selected tool: {}",
            self.equipment_sim.now_s, selected_label
        );

        ui.vertical(|ui| {
            ui_chrome::module_header(ui, "Fab operations", "Fab Control Room", &detail, |_| {});
            ui_chrome::metric_tiles(
                ui,
                &[
                    (
                        "Tools",
                        tools.len().to_string(),
                        "",
                        ui_chrome::Tone::Neutral,
                    ),
                    (
                        "Running",
                        running_count.to_string(),
                        "active runs",
                        ui_chrome::Tone::Success,
                    ),
                    (
                        "Ready",
                        ready_count.to_string(),
                        "idle or complete",
                        if ready_count == 0 {
                            ui_chrome::Tone::Warning
                        } else {
                            ui_chrome::Tone::Info
                        },
                    ),
                    (
                        "Loaded",
                        loaded_count.to_string(),
                        "waiting start",
                        ui_chrome::Tone::Info,
                    ),
                    (
                        "Active alarms",
                        alarm_count.to_string(),
                        if critical_count > 0 {
                            "critical present"
                        } else {
                            "interlocks"
                        },
                        if alarm_count == 0 {
                            ui_chrome::Tone::Neutral
                        } else {
                            ui_chrome::Tone::Danger
                        },
                    ),
                    (
                        "Samples",
                        sample_count.to_string(),
                        "recent sensor points",
                        ui_chrome::Tone::Neutral,
                    ),
                ],
            );
            ui.separator();
            self.equipment_fleet_overview(ui, &tools);
            ui.separator();

            let wide = ui.available_width() > 980.0;
            if wide {
                let total_width = ui.available_width();
                let fleet_width = (total_width * 0.58).clamp(540.0, 760.0);
                let detail_width = (total_width - fleet_width - 18.0).max(360.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(fleet_width);
                        self.equipment_tool_grid(ui, &tools);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(detail_width);
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

    fn equipment_fleet_overview(&self, ui: &mut egui::Ui, tools: &[EquipmentTool]) {
        ui_chrome::section_label(ui, "Fleet Overview");
        ui.horizontal_wrapped(|ui| {
            for state in [
                EquipmentToolState::Running,
                EquipmentToolState::RecipeLoaded,
                EquipmentToolState::OnlineIdle,
                EquipmentToolState::Completed,
                EquipmentToolState::Alarm,
                EquipmentToolState::Maintenance,
                EquipmentToolState::Offline,
            ] {
                let count = tools.iter().filter(|tool| tool.state == state).count();
                if count > 0 {
                    ui_chrome::status_pill(
                        ui,
                        &format!("{} {}", state.label(), count),
                        equipment_state_tone(state),
                    );
                }
            }
        });

        let active_runs = tools
            .iter()
            .filter_map(|tool| {
                let run = tool.active_run.as_ref()?;
                Some(format!(
                    "{} {} {} s",
                    tool.id,
                    run.recipe.recipe_id,
                    equipment_run_elapsed_s(run, self.equipment_sim.now_s)
                ))
            })
            .collect::<Vec<_>>();
        if active_runs.is_empty() {
            ui_chrome::muted(ui, "No process run is currently executing.");
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Active runs").strong());
                for run in active_runs {
                    ui.label(run);
                }
            });
        }
    }

    fn equipment_tool_grid(&mut self, ui: &mut egui::Ui, tools: &[EquipmentTool]) {
        ui_chrome::section_label(ui, "Tool Fleet");
        let mut pending_command: Option<(EquipmentToolId, HostCommand)> = None;
        let mut ordered_tools = tools.iter().collect::<Vec<_>>();
        ordered_tools.sort_by(|left, right| {
            equipment_state_sort_rank(left.state)
                .cmp(&equipment_state_sort_rank(right.state))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        egui::ScrollArea::vertical()
            .id_salt("equipment_tool_grid_scroll")
            .max_height(if ui.available_height() > 520.0 {
                ui.available_height() - 16.0
            } else {
                420.0
            })
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("equipment_tool_grid_horizontal")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("equipment_tool_grid")
                            .num_columns(6)
                            .striped(true)
                            .spacing(vec2(16.0, 10.0))
                            .show(ui, |ui| {
                                ui.strong("Tool");
                                ui.strong("State");
                                ui.strong("Recipe / run");
                                ui.strong("Sensors");
                                ui.strong("Alarms");
                                ui.strong("Host commands");
                                ui.end_row();

                                for tool in ordered_tools {
                                    let selected =
                                        self.selected_equipment_tool.as_ref() == Some(&tool.id);
                                    ui.vertical(|ui| {
                                        if ui
                                            .add(egui::Button::selectable(
                                                selected,
                                                RichText::new(&tool.name).strong(),
                                            ))
                                            .on_hover_text("Select this tool for detailed control")
                                            .clicked()
                                        {
                                            self.set_focus_object(FabObjectRef::tool(
                                                tool.id.as_str().to_string(),
                                            ));
                                        }
                                        ui.label(
                                            RichText::new(format!(
                                                "{} | {}",
                                                tool.id,
                                                tool.kind.label()
                                            ))
                                            .small()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                    });

                                    ui.vertical(|ui| {
                                        ui_chrome::status_pill(
                                            ui,
                                            tool.state.label(),
                                            equipment_state_tone(tool.state),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "updated {} s ago",
                                                self.equipment_sim
                                                    .now_s
                                                    .saturating_sub(tool.last_updated_at_s)
                                            ))
                                            .small()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                    });

                                    ui.vertical(|ui| {
                                        ui.label(equipment_recipe_run_summary(
                                            tool,
                                            self.equipment_sim.now_s,
                                        ));
                                        if let Some((progress, elapsed_s, duration_s)) =
                                            equipment_run_progress(tool, self.equipment_sim.now_s)
                                        {
                                            ui.add(
                                                egui::ProgressBar::new(progress)
                                                    .desired_width(150.0)
                                                    .text(format!("{elapsed_s}/{duration_s} s")),
                                            );
                                        }
                                    });

                                    ui.vertical(|ui| {
                                        ui.label(equipment_recent_sensor_summary(tool));
                                        ui.label(
                                            RichText::new(format!(
                                                "{} streams | {} samples",
                                                equipment_sensor_names(tool).len(),
                                                tool.recent_sensors.len()
                                            ))
                                            .small()
                                            .color(ui.visuals().weak_text_color()),
                                        );
                                    });

                                    ui.label(equipment_tool_alarm_summary(tool));

                                    if let Some(command) = self.equipment_host_command_controls(
                                        ui,
                                        tool,
                                        self.selected_recipe_for_tool(tool),
                                        true,
                                        false,
                                    ) {
                                        pending_command = Some(command);
                                    }
                                    ui.end_row();
                                }
                            });
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

        ui.horizontal_wrapped(|ui| {
            ui.heading(&tool.name);
            ui_chrome::status_pill(ui, tool.state.label(), equipment_state_tone(tool.state));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tool.id.to_string()).strong());
            ui.label(tool.kind.label());
            ui.label(tool.class.label());
            ui.label(format!(
                "last update {} s ago",
                self.equipment_sim
                    .now_s
                    .saturating_sub(tool.last_updated_at_s)
            ));
        });

        egui::Grid::new(("equipment_selected_summary", tool.id.as_str()))
            .num_columns(4)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Active alarms").small());
                ui.label(equipment_tool_alarm_summary(&tool));
                ui.label(RichText::new("Sensor streams").small());
                ui.label(format!(
                    "{} streams, {} samples",
                    equipment_sensor_names(&tool).len(),
                    tool.recent_sensors.len()
                ));
                ui.end_row();
                ui.label(RichText::new("Loaded context").small());
                ui.label(
                    tool.selected_recipe
                        .as_ref()
                        .map(equipment_selection_context)
                        .unwrap_or_else(|| "No loaded lot / wafer context".to_string()),
                );
                ui.label(RichText::new("Run state").small());
                ui.label(equipment_recipe_run_summary(
                    &tool,
                    self.equipment_sim.now_s,
                ));
                ui.end_row();
            });

        ui.separator();
        ui_chrome::section_label(ui, "Recipe Setup");
        let mut draft = self
            .selected_recipe_for_tool(&tool)
            .unwrap_or_else(|| EquipmentRecipeId::new(""));
        ui.horizontal_wrapped(|ui| {
            ui.label("Recipe");
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
            if let Some(recipe) = tool.available_recipes.get(&draft) {
                ui.label(format!(
                    "{} s process, {} parameter{}",
                    recipe.duration_s,
                    recipe.parameters.len(),
                    if recipe.parameters.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                ));
            }
        });
        if !draft.as_str().is_empty() {
            self.equipment_recipe_drafts
                .insert(tool.id.clone(), draft.clone());
        }

        ui_chrome::section_label(ui, "Host Commands");
        ui_chrome::muted(ui, equipment_command_hint(&tool));
        let command_recipe = (!draft.as_str().is_empty()).then_some(draft.clone());
        let pending_command =
            self.equipment_host_command_controls(ui, &tool, command_recipe, false, true);

        if let Some((tool_id, command)) = pending_command {
            self.send_equipment_command(tool_id, command);
        }

        ui.separator();
        if ui.available_width() > 660.0 {
            ui.columns(2, |columns| {
                self.equipment_run_panel(&mut columns[0], &tool);
                columns[0].separator();
                self.equipment_event_log_panel(&mut columns[0], &tool);
                self.equipment_sensor_panel(&mut columns[1], &tool);
                columns[1].separator();
                self.equipment_alarm_panel(&mut columns[1], &tool);
            });
        } else {
            self.equipment_run_panel(ui, &tool);
            ui.separator();
            self.equipment_sensor_panel(ui, &tool);
            ui.separator();
            self.equipment_alarm_panel(ui, &tool);
            ui.separator();
            self.equipment_event_log_panel(ui, &tool);
        }
    }

    fn equipment_host_command_controls(
        &self,
        ui: &mut egui::Ui,
        tool: &EquipmentTool,
        recipe_id: Option<EquipmentRecipeId>,
        compact: bool,
        include_service: bool,
    ) -> Option<(EquipmentToolId, HostCommand)> {
        let mut pending_command = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::Offline,
                    egui::Button::new(if compact { "Online" } else { "Bring Online" }),
                )
                .on_hover_text("Establish host control for an offline tool")
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::BringOnline));
            }
            if let Some(recipe_id) = recipe_id {
                if ui
                    .add_enabled(
                        tool.state.accepts_recipe_load(),
                        egui::Button::new(if compact { "Load" } else { "Load Recipe" }),
                    )
                    .on_hover_text(format!("Load recipe {recipe_id}"))
                    .clicked()
                {
                    pending_command = Some((
                        tool.id.clone(),
                        HostCommand::LoadRecipe {
                            selection: self.selection_for_tool(tool, recipe_id),
                        },
                    ));
                }
            } else {
                ui.add_enabled(
                    false,
                    egui::Button::new(if compact { "Load" } else { "Load Recipe" }),
                )
                .on_hover_text("No recipe is available for this tool");
            }
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::RecipeLoaded,
                    egui::Button::new("Start"),
                )
                .on_hover_text("Start the loaded process recipe")
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::Start));
            }
            if ui
                .add_enabled(
                    tool.state == EquipmentToolState::Running,
                    egui::Button::new("Stop"),
                )
                .on_hover_text("Abort the active process run")
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::Stop));
            }
            if ui
                .add_enabled(
                    !matches!(
                        tool.state,
                        EquipmentToolState::Offline | EquipmentToolState::Maintenance
                    ),
                    egui::Button::new(if compact { "Alarm" } else { "Trigger Alarm" }),
                )
                .on_hover_text("Inject a simulator alarm for host-response testing")
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
                    egui::Button::new(if compact { "Clear" } else { "Clear Alarm" }),
                )
                .on_hover_text("Clear active alarms and return the tool to online idle")
                .clicked()
            {
                pending_command = Some((tool.id.clone(), HostCommand::ClearAlarm));
            }
            if include_service {
                if ui
                    .add_enabled(
                        tool.state != EquipmentToolState::Running,
                        egui::Button::new("Reset"),
                    )
                    .on_hover_text("Clear loaded recipe and run context")
                    .clicked()
                {
                    pending_command = Some((tool.id.clone(), HostCommand::Reset));
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
                    .on_hover_text("Toggle maintenance state for the selected tool")
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
            }
        });
        pending_command
    }

    fn equipment_event_log_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui_chrome::section_label(ui, "Host / Tool Log");
        if tool.event_log.is_empty() {
            ui_chrome::empty_state(ui, "No host events logged");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt(("equipment_event_log", tool.id.as_str()))
            .max_height(132.0)
            .show(ui, |ui| {
                for entry in tool.event_log.iter().rev().take(8) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("t+{} s", entry.at_s))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        );
                        ui.label(&entry.message);
                    });
                }
            });
    }

    fn equipment_run_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui_chrome::section_label(ui, "Recipe / Run State");
        if let Some(run) = &tool.active_run {
            let elapsed_s = equipment_run_elapsed_s(run, self.equipment_sim.now_s);
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(ui, run.status.label(), ui_chrome::Tone::Success);
                ui.label(format!("{} on {}", run.id, run.recipe.recipe_id));
            });
            if let Some((progress, _, duration_s)) =
                equipment_run_progress(tool, self.equipment_sim.now_s)
            {
                ui.add(
                    egui::ProgressBar::new(progress)
                        .desired_width(ui.available_width().min(420.0))
                        .text(format!("{elapsed_s}/{duration_s} s")),
                );
            }
            egui::Grid::new(("equipment_active_run", tool.id.as_str()))
                .num_columns(2)
                .spacing(vec2(10.0, 4.0))
                .show(ui, |ui| {
                    ui.label("Context");
                    ui.label(equipment_selection_context(&run.recipe));
                    ui.end_row();
                    ui.label("Sensor points");
                    ui.label(run.sensor_count.to_string());
                    ui.end_row();
                });
        } else if let Some(selection) = &tool.selected_recipe {
            ui.horizontal_wrapped(|ui| {
                ui_chrome::status_pill(ui, "Loaded", ui_chrome::Tone::Info);
                ui.label(format!(
                    "{} v{}",
                    selection.recipe_id, selection.recipe_version
                ));
            });
            ui_chrome::muted(ui, equipment_selection_context(selection));
        } else {
            ui_chrome::empty_state(ui, "No recipe is loaded");
        }

        ui_chrome::section_label(ui, "Recent Runs");
        if tool.recent_runs.is_empty() {
            ui_chrome::empty_state(ui, "No completed runs");
            return;
        }
        egui::Grid::new(("equipment_recent_runs", tool.id.as_str()))
            .num_columns(5)
            .striped(true)
            .spacing(vec2(10.0, 4.0))
            .show(ui, |ui| {
                ui.strong("Run");
                ui.strong("Status");
                ui.strong("Recipe");
                ui.strong("Duration");
                ui.strong("Samples");
                ui.end_row();
                for run in tool.recent_runs.iter().take(5) {
                    ui.label(run.id.to_string());
                    ui.label(
                        RichText::new(run.status.label())
                            .color(equipment_run_status_color(run.status))
                            .strong(),
                    );
                    ui.label(run.recipe.recipe_id.to_string());
                    ui.label(format!(
                        "{} s",
                        equipment_run_elapsed_s(run, self.equipment_sim.now_s)
                    ));
                    ui.label(run.sensor_count.to_string());
                    ui.end_row();
                }
            });
    }

    fn equipment_sensor_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui_chrome::section_label(ui, "Sensor Streams");
        let names = equipment_sensor_names(tool);
        if names.is_empty() {
            ui_chrome::empty_state(ui, "Waiting for samples");
            return;
        }
        egui::Grid::new(("equipment_sensor_grid", tool.id.as_str()))
            .num_columns(4)
            .striped(true)
            .spacing(vec2(10.0, 6.0))
            .show(ui, |ui| {
                ui.strong("Signal");
                ui.strong("Latest");
                ui.strong("Freshness");
                ui.strong("Trace");
                ui.end_row();
                for name in names {
                    let values = equipment_sensor_series(tool, &name, 42);
                    let latest = tool.latest_sensor(&name);
                    ui.label(&name);
                    if let Some(sample) = latest {
                        ui.label(equipment_sensor_value(sample));
                        ui.label(equipment_sensor_age_text(sample, self.equipment_sim.now_s));
                    } else {
                        ui.label("No value");
                        ui.label("-");
                    }
                    equipment_sparkline(ui, &values, Color32::from_rgb(96, 178, 255));
                    ui.end_row();
                }
            });
    }

    fn equipment_alarm_panel(&self, ui: &mut egui::Ui, tool: &EquipmentTool) {
        ui_chrome::section_label(ui, "Alarms");
        let active_tool_alarms = tool
            .active_alarms
            .iter()
            .filter(|alarm| alarm.active)
            .collect::<Vec<_>>();
        if active_tool_alarms.is_empty() {
            ui_chrome::status_pill(ui, "Selected tool clear", ui_chrome::Tone::Success);
        } else {
            for alarm in active_tool_alarms {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(alarm.severity.label())
                            .color(equipment_alarm_color(alarm.severity))
                            .strong(),
                    );
                    ui.label(format!("{} at t+{} s", alarm.code, alarm.occurred_at_s));
                    ui.label(&alarm.message);
                });
            }
        }

        let other_alarms = self
            .equipment_sim
            .active_alarms()
            .into_iter()
            .filter(|alarm| alarm.tool_id != tool.id)
            .collect::<Vec<_>>();
        if !other_alarms.is_empty() {
            ui.separator();
            ui_chrome::muted(ui, "Other active fab alarms");
            for alarm in other_alarms.into_iter().take(4) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(alarm.severity.label())
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
            egui::Grid::new("equipment_fab_run_log")
                .num_columns(5)
                .striped(true)
                .spacing(vec2(10.0, 4.0))
                .show(ui, |ui| {
                    ui.strong("Tool");
                    ui.strong("Run");
                    ui.strong("Recipe");
                    ui.strong("Status");
                    ui.strong("Duration");
                    ui.end_row();
                    for run in recent_runs.into_iter().take(6) {
                        ui.label(run.tool_id.to_string());
                        ui.label(run.id.to_string());
                        ui.label(run.recipe.recipe_id.to_string());
                        ui.label(
                            RichText::new(run.status.label())
                                .color(equipment_run_status_color(run.status))
                                .strong(),
                        );
                        ui.label(format!(
                            "{} s",
                            equipment_run_elapsed_s(run, self.equipment_sim.now_s)
                        ));
                        ui.end_row();
                    }
                });
        }
    }

    fn navigation_panel(&mut self, ctx: &egui::Context) {
        let width = if Self::viewport_width(ctx) < 600.0 {
            84.0
        } else {
            ui_chrome::NAV_WIDTH
        };
        egui::SidePanel::left("app_navigation")
            .resizable(false)
            .default_width(width)
            .show(ctx, |ui| {
                ui.set_width(width - 18.0);
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("app_navigation_scroll")
                    .show(ui, |ui| {
                        let visible_modes = ViewMode::ALL
                            .iter()
                            .copied()
                            .filter(|mode| self.nav_rail_modes.contains(mode))
                            .collect::<Vec<_>>();
                        if visible_modes.is_empty() {
                            ui_chrome::empty_state(ui, "No pinned modules");
                        }
                        for mode in visible_modes {
                            let selected = self.view_mode == mode;
                            let label = if selected {
                                RichText::new(mode.rail_label()).strong()
                            } else {
                                RichText::new(mode.rail_label())
                            };
                            if ui
                                .add_sized(
                                    [ui.available_width(), 24.0],
                                    egui::Button::selectable(selected, label),
                                )
                                .on_hover_text(mode.title())
                                .clicked()
                            {
                                self.select_view_mode(mode);
                            }
                        }
                    });
            });
    }

    fn select_view_mode(&mut self, mode: ViewMode) {
        if self.view_mode == mode {
            return;
        }
        self.view_mode = mode;
        if !matches!(mode, ViewMode::Layout2d) {
            self.tool = Tool::Select;
        }
        if !mode.supports_viewport_fullscreen() {
            self.viewport_fullscreen = false;
        }
        self.status = mode.status_message().to_string();
    }

    fn open_workflow_destination(&mut self, destination: WorkflowDestination) {
        let mode = match destination {
            WorkflowDestination::Layout => ViewMode::Layout2d,
            WorkflowDestination::ProcessFlow => ViewMode::ProcessFlow,
            WorkflowDestination::Inventory => ViewMode::Inventory,
            WorkflowDestination::Scheduler => ViewMode::Scheduler,
            WorkflowDestination::FabControl => ViewMode::FabControl,
            WorkflowDestination::Maintenance => ViewMode::Maintenance,
            WorkflowDestination::Environment => ViewMode::Environment,
            WorkflowDestination::Safety => ViewMode::Safety,
            WorkflowDestination::Metrology => ViewMode::Metrology,
            WorkflowDestination::Yield => ViewMode::Yield,
            WorkflowDestination::Notebook => ViewMode::Notebook,
            WorkflowDestination::Traceability => ViewMode::Traceability,
        };
        self.select_view_mode(mode);
    }

    fn handle_workflow_action(&mut self, action: WorkflowAction) {
        match action {
            WorkflowAction::Open(destination) => self.open_workflow_destination(destination),
            WorkflowAction::LoadDemoWorkspace => self.load_demo_workspace(),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            self.menu_bar(ui);
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 30.0),
                egui::Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_min_height(30.0);
                    self.tool_strip(ui);
                },
            );
        });
    }

    fn viewport_width(ctx: &egui::Context) -> f32 {
        ctx.content_rect().width()
    }

    fn inline_side_panels(ctx: &egui::Context) -> bool {
        Self::viewport_width(ctx) >= ui_chrome::INLINE_SIDE_PANEL_MIN_WIDTH
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        self.ensure_app_context();
        let compact = ui.available_width() < ui_chrome::COMPACT_MENU_WIDTH;
        ui.scope(|ui| {
            let visuals = ui.visuals_mut();
            visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            visuals.widgets.inactive.corner_radius = 0.into();
            visuals.widgets.hovered.corner_radius = 0.into();
            visuals.widgets.active.corner_radius = 0.into();
            ui.spacing_mut().button_padding = vec2(3.0, 2.0);
            ui.spacing_mut().item_spacing = if compact {
                vec2(8.0, 2.0)
            } else {
                vec2(10.0, 2.0)
            };
            ui.horizontal(|ui| {
                ui.menu_button(menu_hotkey_label(ui, "File", 0), |ui| self.file_menu(ui));
                ui.menu_button(menu_hotkey_label(ui, "Edit", 0), |ui| self.edit_menu(ui));
                ui.menu_button(menu_hotkey_label(ui, "View", 0), |ui| self.view_menu(ui));
                if compact {
                    ui.menu_button("More", |ui| self.menu_bar_more(ui));
                } else {
                    ui.menu_button(menu_hotkey_label(ui, "Bookmarks", 0), |ui| {
                        self.bookmarks_menu(ui)
                    });
                    ui.menu_button(menu_hotkey_label(ui, "Display", 0), |ui| {
                        self.display_menu(ui)
                    });
                    ui.menu_button(menu_hotkey_label(ui, "Tools", 0), |ui| self.tools_menu(ui));
                    ui.menu_button(menu_hotkey_label(ui, "Macros", 0), |ui| {
                        self.macros_menu(ui)
                    });
                    ui.menu_button(menu_hotkey_label(ui, "Help", 0), |ui| self.help_menu(ui));

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("User {}", short_user(self.user_id))).small(),
                        );
                        if let Some(status) = self.visible_status() {
                            ui.label(RichText::new(status).color(ui.visuals().weak_text_color()));
                        }
                    });
                }
            });
        });
    }

    fn menu_bar_more(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Bookmarks", |ui| self.bookmarks_menu(ui));
        ui.menu_button("Display", |ui| self.display_menu(ui));
        ui.menu_button("Tools", |ui| self.tools_menu(ui));
        ui.menu_button("Macros", |ui| self.macros_menu(ui));
        ui.menu_button("Help", |ui| self.help_menu(ui));
    }

    fn tool_strip(&mut self, ui: &mut egui::Ui) {
        let compact = ui.available_width() < ui_chrome::COMPACT_TOOL_STRIP_WIDTH;
        let drawer_mode = !Self::inline_side_panels(ui.ctx());
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = vec2(5.0, 3.0);
            ui.visuals_mut().widgets.inactive.corner_radius = 0.into();
            ui.visuals_mut().widgets.hovered.corner_radius = 0.into();
            ui.visuals_mut().widgets.active.corner_radius = 0.into();
            ui.horizontal_wrapped(|ui| {
                if matches!(self.view_mode, ViewMode::Layout2d | ViewMode::Layout3d) {
                    if ui
                        .selectable_label(matches!(self.view_mode, ViewMode::Layout2d), "2D")
                        .clicked()
                    {
                        self.select_view_mode(ViewMode::Layout2d);
                    }
                    if ui
                        .selectable_label(matches!(self.view_mode, ViewMode::Layout3d), "3D")
                        .clicked()
                    {
                        self.select_view_mode(ViewMode::Layout3d);
                    }
                    ui.separator();
                }
                if matches!(self.view_mode, ViewMode::Layout2d) {
                    let can_delete = self.selected_instance_info().is_some()
                        || !self.selected_top_level_shapes().is_empty();
                    tool_button(ui, &mut self.tool, Tool::Select, "Select");
                    tool_button(ui, &mut self.tool, Tool::Rect, "Rect");
                    tool_button(ui, &mut self.tool, Tool::Polygon, "Poly");
                    tool_button(ui, &mut self.tool, Tool::Path, "Path");
                    tool_button(ui, &mut self.tool, Tool::Via, "Via");
                    if compact {
                        ui.menu_button("More", |ui| {
                            ui.selectable_value(&mut self.tool, Tool::Measure, "Measure");
                            ui.selectable_value(&mut self.tool, Tool::Route, "Route");
                            if ui
                                .add_enabled(can_delete, egui::Button::new("Delete"))
                                .clicked()
                            {
                                self.delete_selection();
                                ui.close();
                            }
                        });
                    } else {
                        tool_button(ui, &mut self.tool, Tool::Measure, "Measure");
                        tool_button(ui, &mut self.tool, Tool::Route, "Route");
                        if can_delete && ui.button("Delete").clicked() {
                            self.delete_selection();
                        }
                    }
                } else if matches!(self.view_mode, ViewMode::Layout3d) {
                    if compact {
                        ui.menu_button("More", |ui| {
                            if ui.button("Reset 3D").clicked() {
                                self.reset_3d_camera_to_document();
                                ui.close();
                            }
                            if ui.button("Fullscreen").clicked() {
                                self.toggle_viewport_fullscreen(ui.ctx());
                                ui.close();
                            }
                        });
                    } else {
                        if ui.button("Reset 3D").clicked() {
                            self.reset_3d_camera_to_document();
                        }
                        if ui.button("Fullscreen").clicked() {
                            self.toggle_viewport_fullscreen(ui.ctx());
                        }
                    }
                }
                if drawer_mode {
                    self.panel_drawer_buttons(ui);
                }
            });
        });
        ui.add_space(4.0);
    }

    fn panel_drawer_buttons(&mut self, ui: &mut egui::Ui) {
        if self.view_mode.has_inspector_panel() {
            if ui
                .selectable_label(self.show_inspector_drawer, "Inspector")
                .clicked()
            {
                self.show_inspector_drawer = !self.show_inspector_drawer;
            }
        }
        if self.view_mode.has_secondary_panel() {
            if ui
                .selectable_label(self.show_layers_drawer, self.secondary_panel_label())
                .clicked()
            {
                self.show_layers_drawer = !self.show_layers_drawer;
            }
        }
    }

    fn secondary_panel_label(&self) -> &'static str {
        match self.view_mode {
            ViewMode::Metrology => "Map",
            ViewMode::MaskPrep
            | ViewMode::CrossSection
            | ViewMode::Layout2d
            | ViewMode::Layout3d => "Layers",
            ViewMode::Experiment => "Responses",
            ViewMode::Notebook => "Links",
            _ => "Panel",
        }
    }

    fn visible_status(&self) -> Option<&str> {
        let status = self.status.trim();
        if status.is_empty()
            || status == self.view_mode.title()
            || status == self.view_mode.status_message()
        {
            None
        } else {
            Some(status)
        }
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("New Blank Workspace").clicked() {
            self.new_blank_workspace();
            ui.close();
        }
        if ui.button("Load Demo Workspace").clicked() {
            self.load_demo_workspace();
            ui.close();
        }
        ui.separator();
        if ui.button("Save Workspace").clicked() {
            self.save_workspace();
            ui.close();
        }
        if ui.button("Load Workspace").clicked() {
            self.load_workspace();
            ui.close();
        }
        ui.separator();
        ui.label("Layout");
        if ui.button("Save Layout JSON").clicked() {
            self.save_document();
            ui.close();
        }
        if ui.button("Load Layout JSON").clicked() {
            self.load_document();
            ui.close();
        }
        if ui.button("Export GDS").clicked() {
            self.export_gds_document();
            ui.close();
        }
        if ui.button("Import GDS").clicked() {
            self.import_gds_document();
            ui.close();
        }
        ui.separator();
        if ui.button("Connect Collaboration").clicked() {
            self.connect_collaboration();
            ui.close();
        }
    }

    fn edit_menu(&mut self, ui: &mut egui::Ui) {
        let editing_mode = matches!(self.view_mode, ViewMode::Layout2d);
        let can_delete = editing_mode
            && (self.selected_instance_info().is_some()
                || !self.selected_top_level_shapes().is_empty());
        if ui
            .add_enabled(editing_mode, egui::Button::new("Undo"))
            .clicked()
        {
            self.undo();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Redo"))
            .clicked()
        {
            self.redo();
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(editing_mode, egui::Button::new("Copy"))
            .clicked()
        {
            self.copy_selection();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Paste"))
            .clicked()
        {
            self.paste_clipboard();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Duplicate"))
            .clicked()
        {
            self.duplicate_selection();
            ui.close();
        }
        if ui
            .add_enabled(can_delete, egui::Button::new("Delete"))
            .clicked()
        {
            self.delete_selection();
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(editing_mode, egui::Button::new("Make Cell"))
            .clicked()
        {
            self.create_cell_from_selection();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Rotate 90"))
            .clicked()
        {
            self.rotate_selected_90();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Mirror X"))
            .clicked()
        {
            self.mirror_selected_x();
            ui.close();
        }
        if ui
            .add_enabled(editing_mode, egui::Button::new("Mirror Y"))
            .clicked()
        {
            self.mirror_selected_y();
            ui.close();
        }
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Command Palette...").clicked() {
            self.show_command_palette = true;
            self.command_palette_query.clear();
            ui.close();
        }
        if ui.button("Sidebar Modules...").clicked() {
            self.show_sidebar_modules = true;
            ui.close();
        }
        ui.separator();
        for group in ModuleGroup::ALL {
            ui.menu_button(group.label(), |ui| {
                for mode in ViewMode::ALL
                    .into_iter()
                    .filter(|candidate| candidate.group() == group)
                {
                    if ui
                        .selectable_label(self.view_mode == mode, mode.nav_label())
                        .clicked()
                    {
                        self.select_view_mode(mode);
                        ui.close();
                    }
                }
            });
        }
    }

    fn bookmarks_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Origin").clicked() {
            self.pan = Vec2::ZERO;
            self.status = "focused origin".to_string();
            ui.close();
        }
        if ui
            .add_enabled(
                self.layout_bounds().is_some(),
                egui::Button::new("Layout Bounds"),
            )
            .clicked()
        {
            if let Some(bounds) = self.layout_bounds() {
                self.focus_rect(bounds);
                self.status = "focused layout bounds".to_string();
            }
            ui.close();
        }
    }

    fn display_menu(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.settings.show_grid_2d, "2D grid");
        ui.checkbox(&mut self.settings.show_grid_3d, "3D grid");
        ui.checkbox(&mut self.settings.show_origin_marker, "Origin crosshair");
        ui.checkbox(&mut self.settings.show_drc_overlay, "DRC overlay");
        ui.separator();
        ui.label("Theme");
        for theme in EditorTheme::ALL {
            ui.selectable_value(&mut self.settings.theme, theme, theme.label());
        }
        ui.separator();
        ui.label("Units");
        for unit in UnitDisplay::ALL {
            ui.selectable_value(&mut self.settings.units, unit, unit.label());
        }
        ui.separator();
        if ui.button("Options...").clicked() {
            self.show_options = true;
            ui.close();
        }
    }

    fn tools_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Command Palette...").clicked() {
            self.show_command_palette = true;
            self.command_palette_query.clear();
            ui.close();
        }
        ui.separator();
        let editing_mode = matches!(self.view_mode, ViewMode::Layout2d);
        if editing_mode {
            ui.label("Layout Tools");
            ui.selectable_value(&mut self.tool, Tool::Select, "Select");
            ui.selectable_value(&mut self.tool, Tool::Rect, "Rectangle");
            ui.selectable_value(&mut self.tool, Tool::Polygon, "Polygon");
            ui.selectable_value(&mut self.tool, Tool::Path, "Path");
            ui.selectable_value(&mut self.tool, Tool::Via, "Via");
            ui.selectable_value(&mut self.tool, Tool::Measure, "Measure");
            ui.selectable_value(&mut self.tool, Tool::Route, "Route");
            ui.separator();
        }

        ui.label("Technology");
        let technology_rows: Vec<_> = self
            .technologies
            .iter()
            .enumerate()
            .map(|(index, technology)| (index, technology.name.clone()))
            .collect();
        for (index, name) in technology_rows {
            if ui
                .selectable_label(index == self.active_technology, name)
                .clicked()
            {
                self.switch_technology(index);
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Run DRC").clicked() {
            self.rerun_drc();
            ui.close();
        }
        if ui.button("Diagnostics").clicked() {
            self.show_diagnostics = true;
            ui.close();
        }
        if let Some((lot_id, action, label)) = self.context_mes_action() {
            ui.separator();
            if ui.button(label).clicked() {
                self.apply_mes_action(&lot_id, action);
                ui.close();
            }
        }
        if let Some(lot_id) = self.app_context.focus_lot().map(str::to_string) {
            if ui.button("Add Focus Note").clicked() {
                let entry_id = self.notebook_panel.add_quick_lot_note(&lot_id);
                self.app_context
                    .active
                    .replace(FabObjectRef::notebook_entry(entry_id.to_string()));
                self.status = format!("added notebook entry {entry_id}");
                ui.close();
            }
        }
    }

    fn macros_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Load Full Demo Workspace").clicked() {
            self.load_demo_workspace();
            ui.close();
        }
        ui.separator();
        ui.label("Layout Test Scenes");
        if ui.button("10k Stress").clicked() {
            self.make_stress_document(10_000);
            ui.close();
        }
        if ui.button("100k Stress").clicked() {
            self.make_stress_document(100_000);
            ui.close();
        }
        if ui.button("1M Stress").clicked() {
            self.make_stress_document(1_000_000);
            ui.close();
        }
        if ui.button("Hierarchy").clicked() {
            self.make_hierarchy_document();
            ui.close();
        }
    }

    fn help_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("About Fabricad").clicked() {
            self.status = "Fabricad mask-layout editor".to_string();
            ui.close();
        }
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

    fn command_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }
        let mut open = self.show_command_palette;
        let mut pending = None;
        egui::Window::new("Command Palette")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Type a command")
                        .desired_width(ui.available_width()),
                );
                response.request_focus();
                let query = self.command_palette_query.trim().to_ascii_lowercase();
                let commands = self.command_entries();
                let matches = commands
                    .into_iter()
                    .filter(|entry| {
                        query.is_empty()
                            || entry.label.to_ascii_lowercase().contains(&query)
                            || entry.detail.to_ascii_lowercase().contains(&query)
                    })
                    .take(14)
                    .collect::<Vec<_>>();

                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    self.show_command_palette = false;
                }
                if ui.input(|input| input.key_pressed(Key::Enter))
                    && let Some(first) = matches.first()
                {
                    pending = Some(first.action);
                }

                ui.separator();
                if matches.is_empty() {
                    ui_chrome::empty_state(ui, "No commands match");
                }
                for entry in matches {
                    if ui
                        .selectable_label(false, &entry.label)
                        .on_hover_text(&entry.detail)
                        .clicked()
                    {
                        pending = Some(entry.action);
                    }
                    ui_chrome::muted(ui, entry.detail);
                }
            });
        self.show_command_palette = open && self.show_command_palette;
        if let Some(action) = pending {
            self.execute_command(action, ctx);
            self.show_command_palette = false;
            self.command_palette_query.clear();
        }
    }

    fn sidebar_modules_window(&mut self, ctx: &egui::Context) {
        if !self.show_sidebar_modules {
            return;
        }
        let mut open = self.show_sidebar_modules;
        egui::Window::new("Sidebar Modules")
            .open(&mut open)
            .resizable(true)
            .default_size(vec2(360.0, 560.0))
            .min_width(300.0)
            .min_height(300.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Default").clicked() {
                        self.nav_rail_modes = default_nav_rail_modes();
                    }
                    if ui.button("All").clicked() {
                        self.nav_rail_modes = ViewMode::ALL.into_iter().collect();
                    }
                    if ui.button("None").clicked() {
                        self.nav_rail_modes.clear();
                    }
                });
                ui.separator();
                let scroll_width = ui.available_width();
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_modules_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(scroll_width);
                        for group in ModuleGroup::ALL {
                            ui_chrome::section_label(ui, group.label());
                            for mode in ViewMode::ALL
                                .into_iter()
                                .filter(|candidate| candidate.group() == group)
                            {
                                let mut shown = self.nav_rail_modes.contains(&mode);
                                if ui.checkbox(&mut shown, mode.nav_label()).changed() {
                                    if shown {
                                        self.nav_rail_modes.insert(mode);
                                    } else {
                                        self.nav_rail_modes.remove(&mode);
                                    }
                                }
                            }
                        }
                    });
            });
        self.show_sidebar_modules = open;
    }

    fn command_entries(&self) -> Vec<CommandEntry> {
        let mut entries = Vec::new();
        for mode in ViewMode::ALL {
            entries.push(CommandEntry {
                label: format!("Go to {}", mode.title()),
                detail: "View".to_string(),
                action: CommandAction::SelectView(mode),
            });
            entries.push(CommandEntry {
                label: format!(
                    "{} {} in sidebar",
                    if self.nav_rail_modes.contains(&mode) {
                        "Hide"
                    } else {
                        "Show"
                    },
                    mode.nav_label()
                ),
                detail: "Sidebar modules".to_string(),
                action: CommandAction::ToggleRail(mode),
            });
        }
        for (tool, label) in [
            (Tool::Select, "Select tool"),
            (Tool::Rect, "Rectangle tool"),
            (Tool::Polygon, "Polygon tool"),
            (Tool::Path, "Path tool"),
            (Tool::Via, "Via tool"),
            (Tool::Measure, "Measure tool"),
            (Tool::Route, "Route tool"),
        ] {
            entries.push(CommandEntry {
                label: label.to_string(),
                detail: "Layout tools".to_string(),
                action: CommandAction::SelectTool(tool),
            });
        }
        entries.extend([
            CommandEntry {
                label: "Run DRC".to_string(),
                detail: "Design rules".to_string(),
                action: CommandAction::RunDrc,
            },
            CommandEntry {
                label: "New blank workspace".to_string(),
                detail: "File".to_string(),
                action: CommandAction::NewBlank,
            },
            CommandEntry {
                label: "Load demo workspace".to_string(),
                detail: "File".to_string(),
                action: CommandAction::LoadDemo,
            },
            CommandEntry {
                label: "Load 10k stress layout".to_string(),
                detail: "Macros".to_string(),
                action: CommandAction::LoadStress(10_000),
            },
            CommandEntry {
                label: "Load hierarchy demo".to_string(),
                detail: "Macros".to_string(),
                action: CommandAction::LoadHierarchy,
            },
            CommandEntry {
                label: "Open options".to_string(),
                detail: "Display".to_string(),
                action: CommandAction::ShowOptions,
            },
            CommandEntry {
                label: "Toggle 2D grid".to_string(),
                detail: "Display".to_string(),
                action: CommandAction::ToggleGrid2d,
            },
            CommandEntry {
                label: "Toggle 3D grid".to_string(),
                detail: "Display".to_string(),
                action: CommandAction::ToggleGrid3d,
            },
            CommandEntry {
                label: "Toggle DRC overlay".to_string(),
                detail: "Display".to_string(),
                action: CommandAction::ToggleDrcOverlay,
            },
            CommandEntry {
                label: "Toggle origin crosshair".to_string(),
                detail: "Display".to_string(),
                action: CommandAction::ToggleOrigin,
            },
            CommandEntry {
                label: "Reset 3D camera".to_string(),
                detail: "3D viewport".to_string(),
                action: CommandAction::Reset3d,
            },
            CommandEntry {
                label: "Toggle viewport fullscreen".to_string(),
                detail: "Layout viewport".to_string(),
                action: CommandAction::ToggleFullscreen,
            },
        ]);
        entries
    }

    fn execute_command(&mut self, action: CommandAction, ctx: &egui::Context) {
        match action {
            CommandAction::SelectView(mode) => self.select_view_mode(mode),
            CommandAction::ToggleRail(mode) => {
                if !self.nav_rail_modes.remove(&mode) {
                    self.nav_rail_modes.insert(mode);
                }
            }
            CommandAction::SelectTool(tool) => {
                self.select_view_mode(ViewMode::Layout2d);
                self.tool = tool;
            }
            CommandAction::RunDrc => self.rerun_drc(),
            CommandAction::LoadDemo => self.load_demo_workspace(),
            CommandAction::NewBlank => self.new_blank_workspace(),
            CommandAction::LoadStress(count) => self.make_stress_document(count),
            CommandAction::LoadHierarchy => self.make_hierarchy_document(),
            CommandAction::ShowOptions => self.show_options = true,
            CommandAction::ToggleGrid2d => self.settings.show_grid_2d = !self.settings.show_grid_2d,
            CommandAction::ToggleGrid3d => self.settings.show_grid_3d = !self.settings.show_grid_3d,
            CommandAction::ToggleDrcOverlay => {
                self.settings.show_drc_overlay = !self.settings.show_drc_overlay;
            }
            CommandAction::ToggleOrigin => {
                self.settings.show_origin_marker = !self.settings.show_origin_marker;
            }
            CommandAction::Reset3d => self.reset_3d_camera_to_document(),
            CommandAction::ToggleFullscreen => self.toggle_viewport_fullscreen(ctx),
        }
    }

    fn toggle_viewport_fullscreen(&mut self, ctx: &egui::Context) {
        self.set_viewport_fullscreen(!self.viewport_fullscreen, ctx);
    }

    fn set_viewport_fullscreen(&mut self, fullscreen: bool, ctx: &egui::Context) {
        if fullscreen && !self.view_mode.supports_viewport_fullscreen() {
            self.status = "viewport fullscreen is available in layout views".to_string();
            return;
        }
        self.viewport_fullscreen = fullscreen;
        self.status = if self.viewport_fullscreen {
            "entered viewport fullscreen".to_string()
        } else {
            "exited viewport fullscreen".to_string()
        };
        ctx.request_repaint();
    }

    fn viewport_fullscreen_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                match self.view_mode {
                    ViewMode::Layout2d => self.canvas(ui),
                    ViewMode::Layout3d => self.canvas_3d(ui),
                    _ => {}
                }
            });

        egui::Area::new(egui::Id::new("viewport_fullscreen_exit"))
            .anchor(Align2::RIGHT_TOP, vec2(-12.0, 12.0))
            .show(ctx, |ui| {
                if ui
                    .button("Exit fullscreen")
                    .on_hover_text("Return to the full Fabricad workspace.")
                    .clicked()
                {
                    self.set_viewport_fullscreen(false, ctx);
                }
            });
    }

    fn options_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Process technology and grid")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.settings.snap_enabled, "Snap");
                    ui.checkbox(&mut self.settings.show_grid_2d, "2D grid");
                    ui.checkbox(&mut self.settings.show_grid_3d, "3D grid");
                    ui.checkbox(&mut self.settings.show_drc_overlay, "DRC overlay");
                });
                ui.horizontal(|ui| {
                    ui.label("Process technology");
                    let mut selected = self.active_technology;
                    egui::ComboBox::from_id_salt("options_technology_picker")
                        .selected_text(self.current_technology().name.clone())
                        .show_ui(ui, |ui| {
                            for (index, technology) in self.technologies.iter().enumerate() {
                                ui.selectable_value(&mut selected, index, &technology.name);
                            }
                        });
                    if selected != self.active_technology {
                        self.switch_technology(selected);
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.settings.show_origin_marker, "Origin crosshair");
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
                    "{} per snap, {} database units per um",
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
        ui.label("Process technology");
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
            "Snap grid: {} ({})",
            self.document.grid,
            self.format_length(self.document.grid as f64)
        ));
        ui.label(format!(
            "Technology grid: {} dbu, database units: {} per um",
            technology.grid, technology.dbu_per_micron
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
        if !self.show_mes_panel || !matches!(self.view_mode, ViewMode::FabControl) {
            return;
        }
        egui::TopBottomPanel::bottom("mes_traveler")
            .resizable(true)
            .default_height(260.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("mes_panel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.mes_ui(ui));
            });
    }

    fn mes_ui(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        if ui.available_width() < 720.0 {
            self.mes_wip_board_ui(ui);
            ui.separator();
            action = self.mes_selected_lot_ui(ui);
            if self.mes.lots.is_empty() {
                ui_chrome::empty_state(ui, "No traveler selected");
            }
        } else {
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
        }
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
                self.set_focus_object(FabObjectRef::lot(lot_id.as_str().to_string()));
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
            warn!("MES operator is empty; using demo operator id");
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
                egui::ScrollArea::vertical()
                    .id_salt("inspector_panel_scroll")
                    .show(ui, |ui| match self.view_mode {
                        ViewMode::Workflow => {
                            self.sync_workflow_focus_from_context();
                            let data = WorkflowData {
                                document: &self.document,
                                process_flow: self.process_flow_panel.model(),
                                recipes: self.recipe_panel.catalog(),
                                mes: &self.mes,
                                inventory: self.inventory_panel.inventory(),
                                maintenance: self.maintenance_panel.model(),
                                environment: self.environment_panel.model(),
                                scheduler: self.scheduler_panel.schedule(),
                                safety: self.safety_panel.model(),
                                equipment: &self.equipment_sim,
                                wafer_map: &self.wafer_map,
                                yield_analysis: &self.yield_analysis,
                                notebook: self.notebook_panel.notebook(),
                            };
                            self.workflow_panel.context_ui(ui, data);
                            self.sync_context_from_workflow();
                        }
                        ViewMode::Metrology => self.metrology_context_panel(ui),
                        ViewMode::MaskPrep => self.mask_panel.context_ui(
                            ui,
                            &self.document,
                            &self.mes,
                            self.selected_mes_lot.as_ref(),
                            &mut self.status,
                        ),
                        ViewMode::LayoutDiff => {
                            self.layout_diff_panel.context_ui(ui, &self.document)
                        }
                        ViewMode::Inventory => self.inventory_panel.context_ui(ui),
                        ViewMode::Maintenance => self.maintenance_panel.context_ui(ui),
                        ViewMode::Environment => self.environment_panel.context_ui(ui),
                        ViewMode::Scheduler => self.scheduler_panel.context_ui(ui),
                        ViewMode::Safety => self.safety_panel.context_ui(ui),
                        ViewMode::SpcFdc => {
                            self.spc_fdc_panel.context_ui(
                                ui,
                                &self.yield_analysis,
                                &self.equipment_sim,
                            );
                        }
                        ViewMode::ProcessFlow => {
                            self.process_flow_panel.context_ui(ui, &mut self.status);
                        }
                        ViewMode::ProcessControl => self
                            .process_control_panel
                            .context_ui(ui, &self.yield_analysis),
                        ViewMode::CrossSection => self.cross_section_panel.context_ui(ui),
                        ViewMode::Traceability => self.genealogy_panel.context_ui(ui),
                        ViewMode::Experiment => {
                            self.experiment_panel.context_ui(ui, &mut self.status);
                        }
                        ViewMode::Notebook => self.notebook_panel.context_ui(ui),
                        ViewMode::Layout2d | ViewMode::Layout3d => {
                            egui::CollapsingHeader::new("Document")
                                .default_open(false)
                                .show(ui, |ui| self.technology_panel(ui));
                            self.recipe_panel.ui(ui, &mut self.status);
                            self.hierarchy_panel(ui);
                            egui::CollapsingHeader::new("Connectivity")
                                .default_open(false)
                                .show(ui, |ui| self.net_panel(ui));
                            egui::CollapsingHeader::new("DRC Markers")
                                .default_open(false)
                                .show(ui, |ui| self.marker_panel(ui));
                            egui::CollapsingHeader::new("Diagnostics")
                                .default_open(false)
                                .show(ui, |ui| self.status_panel(ui));
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
                    let max_height = ui.available_height();
                    egui::ScrollArea::vertical()
                        .id_salt("metrology_panel_scroll")
                        .max_height(max_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.metrology_panel(ui));
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
                if matches!(self.view_mode, ViewMode::CrossSection) {
                    self.cross_section_panel.layer_stack_ui(ui);
                    return;
                }
                if matches!(self.view_mode, ViewMode::Notebook) {
                    self.notebook_panel.link_filter_ui(ui);
                    return;
                }
                if matches!(self.view_mode, ViewMode::Layout3d) {
                    self.stack_3d_panel(ui);
                    ui.separator();
                }
                ui_chrome::section_label(ui, "Layers");
                ui.horizontal(|ui| {
                    if ui.button("Add layer").clicked() {
                        self.add_layer_from_ui();
                    }
                });
                ui.add_space(4.0);
                let mut visibility_ops = Vec::new();
                let mut remove_layer = None;
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
                if layer_rows.is_empty() {
                    ui_chrome::empty_state(ui, "No layers. Add a layer to draw.");
                    return;
                }
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
                                        .selectable_label(self.active_layer == layer_id, "")
                                        .clicked()
                                    {
                                        self.active_layer = layer_id;
                                    }
                                    let mut edited_name = self
                                        .layer_name_drafts
                                        .get(&layer_id)
                                        .cloned()
                                        .unwrap_or(name);
                                    let response = ui
                                        .add(
                                            egui::TextEdit::singleline(&mut edited_name)
                                                .desired_width(118.0)
                                                .id_salt(("layer_name", layer_id.0)),
                                        )
                                        .on_hover_text(purpose);
                                    if response.has_focus() {
                                        self.active_layer = layer_id;
                                    }
                                    let commit = text_edit_commit_requested(ui, &response);
                                    if response.changed() || response.has_focus() {
                                        self.layer_name_drafts
                                            .insert(layer_id, edited_name.clone());
                                    }
                                    let remove_label = edited_name.clone();
                                    if commit {
                                        self.layer_name_drafts.remove(&layer_id);
                                        self.rename_layer(layer_id, edited_name);
                                    } else if !response.has_focus() && !response.changed() {
                                        self.layer_name_drafts.remove(&layer_id);
                                    }
                                    if ui.checkbox(&mut visible, "").changed() {
                                        visibility_ops.push((layer_id, visible));
                                    }
                                    if ui
                                        .small_button("x")
                                        .on_hover_text(format!("Remove {remove_label}"))
                                        .clicked()
                                    {
                                        remove_layer = Some(layer_id);
                                    }
                                });
                            });
                        }
                    });
                ui.separator();
                self.active_layer_properties_ui(ui);
                for (layer, visible) in visibility_ops {
                    self.apply_operation_without_history(&Operation::SetLayerVisibility {
                        layer,
                        visible,
                    });
                }
                if let Some(layer) = remove_layer {
                    self.remove_layer_by_id(layer);
                }
            });
    }

    fn responsive_panel_windows(&mut self, ctx: &egui::Context) {
        if Self::inline_side_panels(ctx) {
            self.show_inspector_drawer = false;
            self.show_layers_drawer = false;
            return;
        }

        if self.view_mode.has_inspector_panel() && self.show_inspector_drawer {
            let mut open = true;
            egui::Window::new("Inspector")
                .open(&mut open)
                .resizable(true)
                .default_size(vec2(340.0, 560.0))
                .min_width(280.0)
                .constrain_to(ctx.content_rect())
                .show(ctx, |ui| self.inspector_drawer_contents(ui));
            self.show_inspector_drawer = open;
        }

        if self.view_mode.has_secondary_panel() && self.show_layers_drawer {
            let mut open = true;
            egui::Window::new(self.secondary_panel_label())
                .open(&mut open)
                .resizable(true)
                .default_size(vec2(320.0, 560.0))
                .min_width(260.0)
                .constrain_to(ctx.content_rect())
                .show(ctx, |ui| self.layers_drawer_contents(ui));
            self.show_layers_drawer = open;
        }
    }

    fn inspector_drawer_contents(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("inspector_drawer_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.view_mode {
                ViewMode::Workflow => {
                    self.sync_workflow_focus_from_context();
                    let data = WorkflowData {
                        document: &self.document,
                        process_flow: self.process_flow_panel.model(),
                        recipes: self.recipe_panel.catalog(),
                        mes: &self.mes,
                        inventory: self.inventory_panel.inventory(),
                        maintenance: self.maintenance_panel.model(),
                        environment: self.environment_panel.model(),
                        scheduler: self.scheduler_panel.schedule(),
                        safety: self.safety_panel.model(),
                        equipment: &self.equipment_sim,
                        wafer_map: &self.wafer_map,
                        yield_analysis: &self.yield_analysis,
                        notebook: self.notebook_panel.notebook(),
                    };
                    self.workflow_panel.context_ui(ui, data);
                    self.sync_context_from_workflow();
                }
                ViewMode::Metrology => self.metrology_context_panel(ui),
                ViewMode::MaskPrep => self.mask_panel.context_ui(
                    ui,
                    &self.document,
                    &self.mes,
                    self.selected_mes_lot.as_ref(),
                    &mut self.status,
                ),
                ViewMode::LayoutDiff => self.layout_diff_panel.context_ui(ui, &self.document),
                ViewMode::Inventory => self.inventory_panel.context_ui(ui),
                ViewMode::Maintenance => self.maintenance_panel.context_ui(ui),
                ViewMode::Environment => self.environment_panel.context_ui(ui),
                ViewMode::Scheduler => self.scheduler_panel.context_ui(ui),
                ViewMode::Safety => self.safety_panel.context_ui(ui),
                ViewMode::SpcFdc => {
                    self.spc_fdc_panel
                        .context_ui(ui, &self.yield_analysis, &self.equipment_sim);
                }
                ViewMode::ProcessFlow => {
                    self.process_flow_panel.context_ui(ui, &mut self.status);
                }
                ViewMode::ProcessControl => self
                    .process_control_panel
                    .context_ui(ui, &self.yield_analysis),
                ViewMode::CrossSection => self.cross_section_panel.context_ui(ui),
                ViewMode::Traceability => self.genealogy_panel.context_ui(ui),
                ViewMode::Experiment => {
                    self.experiment_panel.context_ui(ui, &mut self.status);
                }
                ViewMode::Notebook => self.notebook_panel.context_ui(ui),
                ViewMode::Layout2d | ViewMode::Layout3d => {
                    egui::CollapsingHeader::new("Document")
                        .default_open(false)
                        .show(ui, |ui| self.technology_panel(ui));
                    self.recipe_panel.ui(ui, &mut self.status);
                    self.hierarchy_panel(ui);
                    egui::CollapsingHeader::new("Connectivity")
                        .default_open(false)
                        .show(ui, |ui| self.net_panel(ui));
                    egui::CollapsingHeader::new("DRC Markers")
                        .default_open(false)
                        .show(ui, |ui| self.marker_panel(ui));
                    egui::CollapsingHeader::new("Diagnostics")
                        .default_open(false)
                        .show(ui, |ui| self.status_panel(ui));
                }
                ViewMode::FabControl | ViewMode::Yield => {}
            });
    }

    fn layers_drawer_contents(&mut self, ui: &mut egui::Ui) {
        if matches!(self.view_mode, ViewMode::Metrology) {
            let max_height = ui.available_height();
            egui::ScrollArea::vertical()
                .id_salt("metrology_drawer_scroll")
                .max_height(max_height)
                .auto_shrink([false, false])
                .show(ui, |ui| self.metrology_panel(ui));
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
        if matches!(self.view_mode, ViewMode::CrossSection) {
            self.cross_section_panel.layer_stack_ui(ui);
            return;
        }
        if matches!(self.view_mode, ViewMode::Notebook) {
            self.notebook_panel.link_filter_ui(ui);
            return;
        }
        if matches!(self.view_mode, ViewMode::Layout3d) {
            self.stack_3d_panel(ui);
            ui.separator();
        }

        ui_chrome::section_label(ui, "Layers");
        ui.horizontal(|ui| {
            if ui.button("Add layer").clicked() {
                self.add_layer_from_ui();
            }
        });
        ui.add_space(4.0);

        let mut visibility_ops = Vec::new();
        let mut remove_layer = None;
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
        if layer_rows.is_empty() {
            ui_chrome::empty_state(ui, "No layers. Add a layer to draw.");
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("layers_drawer_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (layer_id, name, purpose, color, mut visible, _) in layer_rows {
                    ui.push_id(("layer_drawer_row", layer_id.0), |ui| {
                        ui.horizontal(|ui| {
                            let color = layer_color32(color, 1.0);
                            let (swatch_rect, _) =
                                ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                            ui.painter().rect_filled(swatch_rect, 2.0, color);
                            if ui
                                .selectable_label(self.active_layer == layer_id, "")
                                .clicked()
                            {
                                self.active_layer = layer_id;
                            }
                            let mut edited_name = self
                                .layer_name_drafts
                                .get(&layer_id)
                                .cloned()
                                .unwrap_or(name);
                            let response = ui
                                .add(
                                    egui::TextEdit::singleline(&mut edited_name)
                                        .desired_width(118.0)
                                        .id_salt(("layer_drawer_name", layer_id.0)),
                                )
                                .on_hover_text(purpose);
                            if response.has_focus() {
                                self.active_layer = layer_id;
                            }
                            let commit = text_edit_commit_requested(ui, &response);
                            if response.changed() || response.has_focus() {
                                self.layer_name_drafts.insert(layer_id, edited_name.clone());
                            }
                            let remove_label = edited_name.clone();
                            if commit {
                                self.layer_name_drafts.remove(&layer_id);
                                self.rename_layer(layer_id, edited_name);
                            } else if !response.has_focus() && !response.changed() {
                                self.layer_name_drafts.remove(&layer_id);
                            }
                            if ui.checkbox(&mut visible, "").changed() {
                                visibility_ops.push((layer_id, visible));
                            }
                            if ui
                                .small_button("x")
                                .on_hover_text(format!("Remove {remove_label}"))
                                .clicked()
                            {
                                remove_layer = Some(layer_id);
                            }
                        });
                    });
                }
            });

        ui.separator();
        self.active_layer_properties_ui(ui);
        for (layer, visible) in visibility_ops {
            self.apply_operation_without_history(&Operation::SetLayerVisibility { layer, visible });
        }
        if let Some(layer) = remove_layer {
            self.remove_layer_by_id(layer);
        }
    }

    fn active_layer_properties_ui(&mut self, ui: &mut egui::Ui) {
        let mut visual_dirty = false;
        let mut analysis_dirty = false;
        let mut changed_name = None;
        let Some(layer) = self.document.layers.get_mut(&self.active_layer) else {
            ui_chrome::empty_state(ui, "No active layer");
            return;
        };
        egui::CollapsingHeader::new("Layer Properties")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!("Layer ID: {}", layer.id.0));
                ui.horizontal(|ui| {
                    ui.label("Process");
                    egui::ComboBox::from_id_salt(("layer_process", layer.id.0))
                        .selected_text(process_layer_label(layer.process))
                        .show_ui(ui, |ui| {
                            for process in PROCESS_LAYER_CHOICES {
                                if ui
                                    .selectable_value(
                                        &mut layer.process,
                                        process,
                                        process_layer_label(process),
                                    )
                                    .changed()
                                {
                                    analysis_dirty = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Purpose");
                    if ui.text_edit_singleline(&mut layer.purpose).changed() {
                        visual_dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Display order");
                    if ui
                        .add(egui::DragValue::new(&mut layer.display_order).speed(1))
                        .changed()
                    {
                        visual_dirty = true;
                    }
                    if ui.checkbox(&mut layer.locked, "Locked").changed() {
                        visual_dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Color");
                    let mut rgb = [layer.color[0], layer.color[1], layer.color[2]];
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        layer.color[0] = rgb[0];
                        layer.color[1] = rgb[1];
                        layer.color[2] = rgb[2];
                        visual_dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    let mut has_gds_layer = layer.gds_layer.is_some();
                    if ui.checkbox(&mut has_gds_layer, "GDS geometry").changed() {
                        layer.gds_layer =
                            has_gds_layer.then_some(u16::try_from(layer.id.0).unwrap_or(u16::MAX));
                        visual_dirty = true;
                    }
                    if let Some(gds_layer) = layer.gds_layer.as_mut() {
                        if ui
                            .add(egui::DragValue::new(gds_layer).range(0..=u16::MAX).speed(1))
                            .changed()
                        {
                            visual_dirty = true;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Datatype");
                    if ui
                        .add(
                            egui::DragValue::new(&mut layer.gds_datatype)
                                .range(0..=u16::MAX)
                                .speed(1),
                        )
                        .changed()
                    {
                        visual_dirty = true;
                    }
                    ui.label("Texttype");
                    if ui
                        .add(
                            egui::DragValue::new(&mut layer.gds_texttype)
                                .range(0..=u16::MAX)
                                .speed(1),
                        )
                        .changed()
                    {
                        visual_dirty = true;
                    }
                });
                if visual_dirty || analysis_dirty {
                    changed_name = Some(layer.name.clone());
                }
            });

        if let Some(name) = changed_name {
            self.reset_render_cache();
            if analysis_dirty {
                self.rebuild_rules_from_technology();
                self.rerun_drc();
                self.rebuild_connectivity();
            }
            self.status = format!("updated layer {name}");
        }
    }

    fn stack_3d_panel(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "3D Stack");
        let mut rows: Vec<_> = self
            .document
            .layers
            .values()
            .filter(|layer| !matches!(layer.process, ProcessLayer::Annotation))
            .collect();
        rows.sort_by_key(|layer| (layer.display_order, layer.id));
        if rows.is_empty() {
            ui_chrome::empty_state(ui, "No printable layers");
            return;
        }
        egui::Grid::new("layer_3d_stack_grid")
            .striped(true)
            .min_col_width(46.0)
            .show(ui, |ui| {
                ui.strong("Layer");
                ui.strong("Process");
                ui.strong("Z base");
                ui.strong("Z top");
                ui.end_row();
                for layer in rows {
                    let (base_z, thickness) = layer_3d_stack_position(layer.process);
                    ui.label(&layer.name);
                    ui.label(process_layer_label(layer.process));
                    ui.label(format!("{base_z:.0}"));
                    ui.label(format!("{:.0}", base_z + thickness));
                    ui.end_row();
                }
            });
    }

    fn metrology_context_panel(&mut self, ui: &mut egui::Ui) {
        if self.wafer_map.dies.is_empty() {
            ui_chrome::empty_state(ui, "No metrology dataset loaded");
            return;
        }
        let links = &self.wafer_map.links;
        let triage_sites = metrology_triage_sites(&self.wafer_map);
        let pass_summary = self.wafer_map.summary(MeasurementKind::PassFail);
        let review_annotations = self
            .wafer_map
            .annotations
            .iter()
            .filter(|annotation| {
                matches!(
                    annotation.kind,
                    layout_model::metrology::AnnotationKind::Review
                )
            })
            .count();

        ui_chrome::section_label(ui, "FabOS Context");
        ui.strong(&self.wafer_map.name);
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(ui, &links.lot_id, ui_chrome::Tone::Info);
            ui_chrome::status_pill(ui, &links.wafer_id, ui_chrome::Tone::Neutral);
            ui_chrome::status_pill(
                ui,
                &format!("{} review", triage_sites.len()),
                if triage_sites.is_empty() {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Warning
                },
            );
        });

        ui.separator();
        ui_chrome::section_label(ui, "Lot / Process");
        egui::Grid::new("metrology_context_links")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Lot");
                ui.label(&links.lot_id);
                ui.end_row();
                ui.label("Wafer");
                ui.label(&links.wafer_id);
                ui.end_row();
                ui.label("Step");
                ui.label(&links.process_step_id);
                ui.end_row();
                ui.label("Recipe");
                ui.label(&links.recipe_id);
                ui.end_row();
                ui.label("Tool run");
                ui.label(&links.tool_run_id);
                ui.end_row();
            });

        ui.separator();
        ui_chrome::section_label(ui, "Wafer");
        let geometry = self.wafer_map.geometry;
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
        ui_chrome::section_label(ui, "Data Products");
        let pass_detail = format!(
            "{} pass / {} fail / {} outlier",
            pass_summary.pass_count, pass_summary.fail_count, pass_summary.outlier_count
        );
        let defect_detail = format!("{review_annotations} review annotation(s)");
        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Measurements",
                    self.wafer_map.measurements.len().to_string(),
                    pass_detail.as_str(),
                    if pass_summary.fail_count > 0 {
                        ui_chrome::Tone::Danger
                    } else if pass_summary.outlier_count > 0 {
                        ui_chrome::Tone::Warning
                    } else {
                        ui_chrome::Tone::Success
                    },
                ),
                (
                    "Defects",
                    self.wafer_map.defects.len().to_string(),
                    defect_detail.as_str(),
                    if self.wafer_map.defects.is_empty() {
                        ui_chrome::Tone::Success
                    } else {
                        ui_chrome::Tone::Warning
                    },
                ),
            ],
        );
        ui_chrome::muted(ui, "Map data: die and site measurement records");
        ui_chrome::muted(ui, "Review data: defect annotations and image evidence");

        ui.separator();
        ui_chrome::section_label(ui, "Review Queue");
        if triage_sites.is_empty() {
            ui_chrome::status_pill(ui, "No sites need review", ui_chrome::Tone::Success);
        } else {
            for site in triage_sites.iter().take(5) {
                ui.label(format!(
                    "C{} R{}  {} issue(s), {} defect(s)",
                    site.die.column,
                    site.die.row,
                    site.fail_count + site.outlier_count,
                    site.defect_count
                ));
            }
        }

        ui.separator();
        ui_chrome::section_label(ui, "Inspection Annotations");
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
        ui_chrome::section_label(ui, "Metrology");
        if self.wafer_map.dies.is_empty() {
            ui_chrome::empty_state(ui, "No wafer map loaded");
            return;
        }
        let triage_sites = metrology_triage_sites(&self.wafer_map);

        let previous_mode = self.metrology_map_mode;
        ui_chrome::section_label(ui, "Wafer Map Mode");
        ui.horizontal_wrapped(|ui| {
            for mode in MetrologyMapMode::ALL {
                ui.selectable_value(&mut self.metrology_map_mode, mode, mode.short_label());
            }
        });
        egui::ComboBox::from_id_salt("metrology_map_mode_picker")
            .selected_text(self.metrology_map_mode.label())
            .show_ui(ui, |ui| {
                for mode in MetrologyMapMode::ALL {
                    ui.selectable_value(&mut self.metrology_map_mode, mode, mode.label());
                }
            });
        if self.metrology_map_mode != previous_mode {
            self.status = format!("metrology map: {}", self.metrology_map_mode.label());
        }
        ui.label(
            RichText::new(self.metrology_map_mode.detail())
                .small()
                .color(ui.visuals().weak_text_color()),
        );

        let previous_kind = self.metrology_kind;
        ui_chrome::section_label(ui, "Measurement");
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
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} review site(s)", triage_sites.len()),
                if triage_sites.is_empty() {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Warning
                },
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} defect(s)", self.wafer_map.defects.len()),
                if self.wafer_map.defects.is_empty() {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Warning
                },
            );
        });

        ui.separator();
        let summary = self.wafer_map.summary(self.metrology_kind);
        self.metrology_summary_ui(ui, summary);

        ui.separator();
        self.metrology_legend_ui(ui, summary);

        ui.separator();
        self.metrology_triage_ui(ui, &triage_sites);

        ui.separator();
        let histogram = self.wafer_map.histogram(self.metrology_kind, 18);
        self.metrology_histogram_ui(ui, &histogram, summary);

        ui.separator();
        self.metrology_radial_profile_ui(ui);

        ui.separator();
        self.metrology_selected_die_ui(ui);
    }

    fn metrology_summary_ui(
        &self,
        ui: &mut egui::Ui,
        summary: layout_model::metrology::MeasurementSummary,
    ) {
        ui_chrome::section_label(ui, "SPC Summary");
        let issue_count = summary.fail_count + summary.outlier_count;
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                if issue_count == 0 {
                    "In control"
                } else if summary.fail_count > 0 {
                    "Spec violation"
                } else {
                    "Outlier review"
                },
                if summary.fail_count > 0 {
                    ui_chrome::Tone::Danger
                } else if summary.outlier_count > 0 {
                    ui_chrome::Tone::Warning
                } else {
                    ui_chrome::Tone::Success
                },
            );
            ui_chrome::status_pill(
                ui,
                &metrology_spec_window_label(summary.kind),
                ui_chrome::Tone::Neutral,
            );
        });

        let pass_rate = if summary.sample_count == 0 {
            0.0
        } else {
            summary.pass_count as f64 / summary.sample_count as f64
        };
        let pass_detail = format!(
            "{} pass / {} fail / {} outlier",
            summary.pass_count, summary.fail_count, summary.outlier_count
        );
        let mean_text = summary
            .mean
            .map(|mean| format_metrology_value(summary.kind, mean))
            .unwrap_or_else(|| "-".to_string());
        let mean_detail = summary
            .mean
            .and_then(|mean| format_metrology_target_delta(summary.kind, mean))
            .unwrap_or_else(|| "no target".to_string());
        let spread_text = summary
            .stddev
            .map(|stddev| format_metrology_delta(summary.kind, stddev))
            .unwrap_or_else(|| "-".to_string());
        let spread_detail = summary
            .stddev
            .map(|stddev| format!("3s {}", format_metrology_delta(summary.kind, stddev * 3.0)))
            .unwrap_or_else(|| "no variance".to_string());
        let range_text = match (summary.min, summary.max) {
            (Some(min), Some(max)) => format!(
                "{}..{}",
                format_metrology_value(summary.kind, min),
                format_metrology_value(summary.kind, max)
            ),
            _ => "-".to_string(),
        };
        let cpk = metrology_capability_index(summary);
        let cpk_text = cpk
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "-".to_string());
        let cpk_detail = cpk
            .map(metrology_capability_label)
            .unwrap_or_else(|| "no two-sided capability".to_string());

        ui_chrome::metric_tiles(
            ui,
            &[
                (
                    "Pass rate",
                    format_percent(pass_rate),
                    pass_detail.as_str(),
                    if summary.fail_count > 0 {
                        ui_chrome::Tone::Danger
                    } else if summary.outlier_count > 0 {
                        ui_chrome::Tone::Warning
                    } else {
                        ui_chrome::Tone::Success
                    },
                ),
                (
                    "Mean",
                    mean_text,
                    mean_detail.as_str(),
                    ui_chrome::Tone::Info,
                ),
                (
                    "Sigma",
                    spread_text,
                    spread_detail.as_str(),
                    ui_chrome::Tone::Neutral,
                ),
                (
                    "Cpk",
                    cpk_text,
                    cpk_detail.as_str(),
                    metrology_capability_tone(cpk),
                ),
                ("Range", range_text, "min..max", ui_chrome::Tone::Neutral),
            ],
        );
    }

    fn metrology_legend_ui(
        &self,
        ui: &mut egui::Ui,
        summary: layout_model::metrology::MeasurementSummary,
    ) {
        ui_chrome::section_label(ui, "Legend");
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
        ui.horizontal(|ui| {
            metrology_swatch(ui, metrology_spec_window_color_for_ratio(0.92));
            ui.label("Near spec limit");
        });
        ui.horizontal(|ui| {
            metrology_swatch(ui, metrology_review_queue_color(85));
            ui.label("Review priority");
        });
    }

    fn metrology_histogram_ui(
        &self,
        ui: &mut egui::Ui,
        bins: &[HistogramBin],
        summary: layout_model::metrology::MeasurementSummary,
    ) {
        ui_chrome::section_label(ui, "Distribution");
        let desired = metrology_panel_plot_size(ui, 120.0, 420.0, 120.0);
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
        if let (Some(min), Some(max)) = (summary.min, summary.max) {
            let span = (max - min).abs().max(0.000_001);
            let draw_limit = |value: f64, color: Color32, label: &str| {
                if value < min || value > max {
                    return;
                }
                let x = rect.left() + rect.width() * (((value - min) / span) as f32);
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(1.0, color),
                );
                painter.text(
                    Pos2::new((x + 3.0).min(rect.right() - 20.0), rect.top() + 4.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::monospace(9.0),
                    color,
                );
            };
            let spec = summary.kind.spec();
            if let Some(lower) = spec.lower {
                draw_limit(lower, Color32::from_rgb(238, 184, 72), "LSL");
            }
            if let Some(target) = spec.target {
                draw_limit(target, Color32::from_rgb(180, 230, 176), "T");
            }
            if let Some(upper) = spec.upper {
                draw_limit(upper, Color32::from_rgb(238, 184, 72), "USL");
            }
        }
    }

    fn metrology_radial_profile_ui(&self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Radial Profile");
        let desired = metrology_panel_plot_size(ui, 120.0, 420.0, 96.0);
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 24));
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(70, 82, 86)),
            StrokeKind::Inside,
        );

        let profile = metrology_radial_profile(&self.wafer_map, self.metrology_kind, 10);
        let values = profile
            .iter()
            .filter_map(|(_, value)| *value)
            .collect::<Vec<_>>();
        if values.len() < 2 {
            return;
        }
        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let span = (max - min).max(0.000_001);
        let points = profile
            .iter()
            .filter_map(|(radius, value)| {
                let value = (*value)?;
                let x = rect.left() + rect.width() * radius.clamp(0.0, 1.0);
                let y = rect.bottom() - rect.height() * (((value - min) / span) as f32);
                Some(Pos2::new(x, y))
            })
            .collect::<Vec<_>>();
        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(1.8, Color32::from_rgb(132, 205, 226)),
            );
        }
        for point in points {
            painter.circle_filled(point, 2.5, Color32::from_rgb(215, 238, 242));
        }
        if let Some(target) = self.metrology_kind.spec().target {
            if target >= min && target <= max {
                let y = rect.bottom() - rect.height() * (((target - min) / span) as f32);
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(180, 230, 176, 135)),
                );
            }
        }
        painter.text(
            rect.left_bottom() + vec2(4.0, -4.0),
            Align2::LEFT_BOTTOM,
            "center",
            FontId::monospace(9.0),
            Color32::from_rgb(170, 180, 176),
        );
        painter.text(
            rect.right_bottom() + vec2(-4.0, -4.0),
            Align2::RIGHT_BOTTOM,
            "edge",
            FontId::monospace(9.0),
            Color32::from_rgb(170, 180, 176),
        );
    }

    fn metrology_triage_ui(&mut self, ui: &mut egui::Ui, sites: &[MetrologyTriageSite]) {
        ui_chrome::section_label(ui, "Defect / Review Triage");
        let fail_count: usize = sites.iter().map(|site| site.fail_count).sum();
        let outlier_count: usize = sites.iter().map(|site| site.outlier_count).sum();
        let review_count: usize = sites.iter().map(|site| site.review_count).sum();
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{fail_count} fail"),
                if fail_count == 0 {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Danger
                },
            );
            ui_chrome::status_pill(
                ui,
                &format!("{outlier_count} outlier"),
                if outlier_count == 0 {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Warning
                },
            );
            ui_chrome::status_pill(
                ui,
                &format!("{review_count} review mark"),
                if review_count == 0 {
                    ui_chrome::Tone::Neutral
                } else {
                    ui_chrome::Tone::Info
                },
            );
        });

        if sites.is_empty() {
            ui_chrome::empty_state(ui, "No defects or metrology excursions");
        } else {
            if ui.button("Select highest priority").clicked() {
                self.selected_die = sites.first().map(|site| site.die);
                if let Some(die) = self.selected_die {
                    self.status = format!("selected review site C{} R{}", die.column, die.row);
                }
            }
            for site in sites.iter().take(7) {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(
                            self.selected_die == Some(site.die),
                            format!("C{} R{}", site.die.column, site.die.row),
                        )
                        .clicked()
                    {
                        self.selected_die = Some(site.die);
                        self.status = format!(
                            "selected review site C{} R{}",
                            site.die.column, site.die.row
                        );
                    }
                    ui_chrome::status_pill(
                        ui,
                        metrology_triage_label(site.score),
                        metrology_triage_tone(site.score),
                    );
                    ui.label(format!(
                        "{} fail, {} outlier, {} defect",
                        site.fail_count, site.outlier_count, site.defect_count
                    ));
                });
            }
        }

        let class_counts = metrology_defect_class_counts(&self.wafer_map);
        if class_counts.iter().any(|(_, count)| *count > 0) {
            ui.add_space(4.0);
            egui::Grid::new("metrology_defect_class_counts")
                .num_columns(2)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    for (class, count) in class_counts {
                        if count == 0 {
                            continue;
                        }
                        ui.label(defect_class_label(class));
                        ui.label(count.to_string());
                        ui.end_row();
                    }
                });
        }
    }

    fn metrology_selected_die_ui(&mut self, ui: &mut egui::Ui) {
        ui_chrome::section_label(ui, "Selected Site");
        let triage_sites = metrology_triage_sites(&self.wafer_map);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Next attention").clicked() {
                self.selected_die = metrology_next_triage_die(&triage_sites, self.selected_die);
                if let Some(die) = self.selected_die {
                    self.status = format!("selected review site C{} R{}", die.column, die.row);
                } else {
                    self.status = "no metrology sites need review".to_string();
                }
            }
            if self.selected_die.is_some() && ui.button("Clear").clicked() {
                self.selected_die = None;
            }
        });
        let Some(die) = self.selected_die else {
            ui_chrome::empty_state(ui, "No site selected");
            return;
        };
        let site = metrology_triage_for_die(&self.wafer_map, die);
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("C{} R{}", die.column, die.row));
            ui_chrome::status_pill(
                ui,
                metrology_triage_label(site.score),
                metrology_triage_tone(site.score),
            );
        });
        self.metrology_review_image_ui(ui, die);

        ui.separator();
        ui_chrome::section_label(ui, "Measurement Data");
        if ui.available_width() < 340.0 {
            for kind in MeasurementKind::ALL {
                if let Some(measurement) = self.wafer_map.measurement_for(die, kind) {
                    ui.horizontal_wrapped(|ui| {
                        metrology_swatch(ui, metrology_status_color(measurement.status));
                        ui.strong(kind.label());
                        ui.label(format_metrology_value(kind, measurement.value));
                        ui.colored_label(
                            metrology_status_color(measurement.status),
                            measurement.status.label(),
                        );
                    });
                    ui_chrome::muted(
                        ui,
                        format!(
                            "{}  {}",
                            format_metrology_target_delta(kind, measurement.value)
                                .unwrap_or_else(|| "-".to_string()),
                            metrology_compact_record_id(&measurement.id)
                        ),
                    );
                }
            }
        } else {
            egui::Grid::new("metrology_selected_measurement_data")
                .striped(true)
                .num_columns(5)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Metric");
                    ui.strong("Value");
                    ui.strong("Target delta");
                    ui.strong("Status");
                    ui.strong("Record");
                    ui.end_row();
                    for kind in MeasurementKind::ALL {
                        if let Some(measurement) = self.wafer_map.measurement_for(die, kind) {
                            ui.label(kind.label());
                            ui.label(format_metrology_value(kind, measurement.value));
                            ui.label(
                                format_metrology_target_delta(kind, measurement.value)
                                    .unwrap_or_else(|| "-".to_string()),
                            );
                            ui.colored_label(
                                metrology_status_color(measurement.status),
                                measurement.status.label(),
                            );
                            ui.label(metrology_compact_record_id(&measurement.id));
                            ui.end_row();
                        }
                    }
                });
        }

        ui.separator();
        ui_chrome::section_label(ui, "Site Status");
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

        ui.separator();
        let defects = self.wafer_map.defects_for_die(die).collect::<Vec<_>>();
        ui_chrome::section_label(ui, &format!("Defect Records: {}", defects.len()));
        for defect in defects.iter().take(5) {
            ui.label(format!(
                "{}  {:.2} um  severity {}",
                defect_class_label(defect.class),
                defect.size_um,
                defect.severity
            ));
            if let Some(linked) = &defect.linked_measurement_id {
                ui_chrome::muted(ui, metrology_compact_record_id(linked));
            }
        }

        ui.separator();
        ui_chrome::section_label(ui, "Annotations");
        for annotation in self.wafer_map.annotations_for_die(die) {
            ui.label(format!(
                "{:?}: {} ({})",
                annotation.kind, annotation.note, annotation.author
            ));
        }
    }

    fn metrology_review_image_ui(&self, ui: &mut egui::Ui, die: DieCoord) {
        ui_chrome::section_label(ui, "Review Image");
        let desired = metrology_panel_plot_size(ui, 160.0, 420.0, 138.0);
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_rgb(36, 39, 39));
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(88, 96, 98)),
            StrokeKind::Inside,
        );

        let image_rect = rect.shrink(10.0);
        painter.rect_filled(image_rect, 2.0, Color32::from_rgb(75, 78, 76));
        let stripe_count = 7;
        for index in 0..stripe_count {
            let x = image_rect.left()
                + image_rect.width() * ((index as f32 + 0.5) / stripe_count as f32);
            let stripe = EguiRect::from_center_size(
                Pos2::new(x, image_rect.center().y),
                vec2(image_rect.width() * 0.055, image_rect.height()),
            );
            painter.rect_filled(stripe, 0.0, Color32::from_rgb(104, 108, 105));
        }

        if let Some(cd) = self
            .wafer_map
            .measurement_for(die, MeasurementKind::CriticalDimensionNm)
        {
            let target = MeasurementKind::CriticalDimensionNm
                .spec()
                .target
                .unwrap_or(cd.value);
            let offset = ((cd.value - target) as f32 * 2.0).clamp(-16.0, 16.0);
            let y = image_rect.center().y + offset;
            painter.line_segment(
                [
                    Pos2::new(image_rect.left() + 22.0, y),
                    Pos2::new(image_rect.right() - 22.0, y),
                ],
                Stroke::new(2.0, Color32::from_rgb(245, 225, 96)),
            );
            painter.text(
                image_rect.left_top() + vec2(8.0, 8.0),
                Align2::LEFT_TOP,
                format!("CD {}", format_metrology_value(cd.kind, cd.value)),
                FontId::monospace(10.0),
                Color32::from_rgb(238, 236, 210),
            );
        }

        for defect in self.wafer_map.defects_for_die(die).take(4) {
            let local = defect_local_position(&self.wafer_map, defect.position_mm, die);
            let point = Pos2::new(
                image_rect.left() + image_rect.width() * local[0],
                image_rect.top() + image_rect.height() * local[1],
            );
            painter.circle_stroke(
                point,
                (5.0 + defect.severity as f32).min(10.0),
                Stroke::new(1.5, Color32::from_rgb(255, 112, 92)),
            );
        }

        painter.text(
            image_rect.left_bottom() + vec2(8.0, -8.0),
            Align2::LEFT_BOTTOM,
            "SEM review frame",
            FontId::monospace(10.0),
            Color32::from_rgb(215, 220, 216),
        );
        painter.text(
            image_rect.right_bottom() + vec2(-8.0, -8.0),
            Align2::RIGHT_BOTTOM,
            format!("C{} R{}", die.column, die.row),
            FontId::monospace(10.0),
            Color32::from_rgb(215, 220, 216),
        );
    }

    fn marker_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} active", self.active_marker_count()),
                if self.active_marker_count() == 0 {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Danger
                },
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} saved states", self.document.marker_states.len()),
                ui_chrome::Tone::Neutral,
            );
        });
        if self.violations.is_empty() {
            ui_chrome::empty_state(ui, "No DRC markers");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.settings.show_drc_overlay, "Overlay")
                .on_hover_text("Draw active DRC marker bounds in the layout viewport.");
            ui.checkbox(&mut self.show_waived_markers, "Show waived")
                .on_hover_text("Waived markers are accepted exceptions that stay recorded.");
            ui.checkbox(&mut self.show_hidden_markers, "Show hidden")
                .on_hover_text("Hidden markers are suppressed from this marker list.");
            if ui.button("Clear states").clicked() {
                self.clear_marker_states();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.add(
                egui::TextEdit::singleline(&mut self.drc_marker_filter)
                    .desired_width(ui.available_width())
                    .hint_text("rule or message"),
            );
        });

        let filter = self.drc_marker_filter.trim().to_ascii_lowercase();
        let mut visible_rows = Vec::new();
        let mut rule_counts = BTreeMap::new();
        for (index, violation) in self.violations.iter().enumerate() {
            let state = self.marker_state(violation);
            if (state.hidden && !self.show_hidden_markers)
                || (state.waived && !self.show_waived_markers)
            {
                continue;
            }
            if !filter.is_empty()
                && !violation.rule.to_ascii_lowercase().contains(&filter)
                && !violation.message.to_ascii_lowercase().contains(&filter)
            {
                continue;
            }
            *rule_counts.entry(violation.rule.clone()).or_insert(0usize) += 1;
            visible_rows.push(index);
        }
        if visible_rows.is_empty() {
            ui_chrome::empty_state(ui, "No DRC markers match the filter");
            return;
        }

        egui::CollapsingHeader::new(format!("Rules ({} visible)", visible_rows.len()))
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("drc_marker_rule_summary")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Rule");
                        ui.strong("Count");
                        ui.end_row();
                        for (rule, count) in rule_counts.iter().take(12) {
                            ui.label(rule);
                            ui.label(count.to_string());
                            ui.end_row();
                        }
                    });
                if rule_counts.len() > 12 {
                    ui_chrome::muted(ui, format!("{} more rules", rule_counts.len() - 12));
                }
            });

        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("drc_marker_scroll")
            .max_height(260.0)
            .show_rows(ui, 28.0, visible_rows.len(), |ui, row_range| {
                for row in row_range {
                    let Some(violation_index) = visible_rows.get(row).copied() else {
                        continue;
                    };
                    let Some(violation) = self.violations.get(violation_index) else {
                        continue;
                    };
                    let key = drc_marker_key(violation);
                    let state = if self.document.marker_states.is_empty() {
                        MarkerState::default()
                    } else {
                        self.document
                            .marker_states
                            .get(&key)
                            .cloned()
                            .unwrap_or_default()
                    };
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
        if let Some(reason) = &self.connectivity.skipped {
            ui_chrome::empty_state(ui, reason);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui_chrome::status_pill(
                ui,
                &format!("{} nets", self.connectivity.components.len()),
                ui_chrome::Tone::Neutral,
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} shorts", self.connectivity.shorts.len()),
                if self.connectivity.shorts.is_empty() {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Danger
                },
            );
            ui_chrome::status_pill(
                ui,
                &format!("{} opens", self.connectivity.opens.len()),
                if self.connectivity.opens.is_empty() {
                    ui_chrome::Tone::Success
                } else {
                    ui_chrome::Tone::Warning
                },
            );
        });
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.add(
                egui::TextEdit::singleline(&mut self.connectivity_filter)
                    .desired_width(ui.available_width())
                    .hint_text("net name or component"),
            );
        });

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

        let filter = self.connectivity_filter.trim().to_ascii_lowercase();
        let short_matches: Vec<_> = self
            .connectivity
            .shorts
            .iter()
            .filter(|short| {
                filter.is_empty()
                    || short.component.to_string().contains(&filter)
                    || short
                        .names
                        .iter()
                        .any(|name| name.to_ascii_lowercase().contains(&filter))
            })
            .collect();
        let open_matches: Vec<_> = self
            .connectivity
            .opens
            .iter()
            .filter(|open| {
                filter.is_empty()
                    || open.name.to_ascii_lowercase().contains(&filter)
                    || open
                        .components
                        .iter()
                        .any(|component| component.to_string().contains(&filter))
            })
            .collect();

        let mut focus_target = None;
        if !short_matches.is_empty() {
            egui::CollapsingHeader::new(format!(
                "Shorts (showing {} of {} matching, {} total)",
                short_matches.len().min(MAX_CONNECTIVITY_ROWS),
                short_matches.len(),
                self.connectivity.shorts.len()
            ))
            .default_open(false)
            .show(ui, |ui| {
                for short in short_matches.iter().take(MAX_CONNECTIVITY_ROWS) {
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
            });
        }
        if !open_matches.is_empty() {
            egui::CollapsingHeader::new(format!(
                "Opens (showing {} of {} matching, {} total)",
                open_matches.len().min(MAX_CONNECTIVITY_ROWS),
                open_matches.len(),
                self.connectivity.opens.len()
            ))
            .default_open(false)
            .show(ui, |ui| {
                for open in open_matches.iter().take(MAX_CONNECTIVITY_ROWS) {
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
            });
        }
        if self.connectivity.shorts.is_empty() && self.connectivity.opens.is_empty() {
            ui_chrome::empty_state(ui, "No shorts or opens");
        } else if short_matches.is_empty() && open_matches.is_empty() {
            ui_chrome::empty_state(ui, "No connectivity issues match the filter");
        }
        drop(short_matches);
        drop(open_matches);
        if let Some(bounds) = focus_target {
            self.focus_rect(bounds);
        }
    }

    fn hierarchy_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Cells");
        if ui.button("Add cell").clicked() {
            self.create_empty_cell();
        }
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
                    self.cell_instance_count(cell.id),
                    cell.id == self.document.top_cell,
                )
            })
            .collect();
        for (id, name, shape_count, child_instance_count, placed_instance_count, is_top) in
            cell_rows
        {
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
                ui.label(format!(
                    "{shape_count} shapes / {child_instance_count} child instances"
                ))
                .on_hover_text(format!(
                    "{} placed instance{}",
                    placed_instance_count,
                    if placed_instance_count == 1 { "" } else { "s" }
                ));
                if !is_top && ui.button("Place").clicked() {
                    self.place_cell_instance(id);
                }
                let can_delete = !is_top && placed_instance_count == 0;
                if ui
                    .add_enabled(can_delete, egui::Button::new("Delete"))
                    .on_disabled_hover_text(if is_top {
                        "The top cell cannot be deleted"
                    } else {
                        "Delete placed instances before deleting this cell"
                    })
                    .clicked()
                {
                    self.delete_cell(id);
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
                if let Some(mut instance) = self.document.instance(info.parent, info.id) {
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
        let Some(mut edited) = self.document.shapes.get(&id) else {
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
            .filter_map(|id| self.document.shapes.get(id))
            .collect()
    }

    fn apply_first_vertex_demo_edit(&mut self) {
        self.select_first_top_level_shape();
        let Some(id) = self.selected_top_level_shape() else {
            return;
        };
        let Some(shape) = self.document.shapes.get(&id) else {
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
        if let Some(mut instance) = self.document.instance(info.parent, info.id) {
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
        let bounds = self.layout_bounds().unwrap_or_else(|| {
            warn!("3D camera reset found no layout bounds; using default view bounds");
            Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000))
        });
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

    fn set_flycam_capture(&mut self, ctx: &egui::Context, captured: bool) {
        if self.flycam_captured == captured {
            return;
        }
        self.flycam_captured = captured;
        let grab = if captured {
            egui::CursorGrab::Locked
        } else {
            egui::CursorGrab::None
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(grab));
        ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(!captured));
        self.status = if captured {
            "3D flycam captured; press Esc to release mouse".to_string()
        } else {
            "3D flycam released".to_string()
        };
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
        let previous_context_lot = lot_id.clone();
        let previous_context_wafer = wafer_id.clone();
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
                    let lot_yield_detail = format!(
                        "{} pass / {} fail / {} dies",
                        summary.passing_dies, summary.failing_dies, summary.total_dies
                    );
                    let main_fail_detail = summary.spatial_pattern.label().to_string();
                    let recipe_detail = lot_route_label(&self.yield_analysis, &lot_id);
                    ui_chrome::metric_tiles(
                        ui,
                        &[
                            (
                                "Lot yield",
                                format_percent(summary.yield_fraction),
                                lot_yield_detail.as_str(),
                                ui_chrome::Tone::Neutral,
                            ),
                            (
                                "Main fail",
                                summary
                                    .dominant_failure
                                    .map(FailureMode::label)
                                    .unwrap_or("none")
                                    .to_string(),
                                main_fail_detail.as_str(),
                                ui_chrome::Tone::Neutral,
                            ),
                            (
                                "Recipe path",
                                lot_recipe_label(&self.yield_analysis, &lot_id),
                                recipe_detail.as_str(),
                                ui_chrome::Tone::Neutral,
                            ),
                        ],
                    );
                }

                ui.separator();
                if ui.available_width() < 760.0 {
                    ui_chrome::section_label(ui, &format!("Wafer Map {wafer_id}"));
                    draw_yield_wafer_map(ui, &die_outcomes);
                    if let Some(summary) = &wafer_summary {
                        ui.label(format!(
                            "{} yield, {} failures, {}",
                            format_percent(summary.yield_fraction),
                            summary.failing_dies,
                            summary.spatial_pattern.label()
                        ));
                        for hint in &summary.root_cause_hints {
                            ui.label(hint);
                        }
                    }
                    ui.separator();
                    ui_chrome::section_label(ui, "Wafer Yield");
                    self.yield_wafer_rows(ui, &wafer_rows);
                    ui.separator();
                    ui_chrome::section_label(ui, "Failure Modes");
                    if let Some(summary) = &lot_summary {
                        failure_breakdown_ui(ui, summary);
                    }
                } else {
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
                }

                ui.separator();
                if ui.available_width() < 760.0 {
                    ui_chrome::section_label(ui, "Lot / Recipe Comparison");
                    lot_comparison_ui(ui, &comparisons);
                    ui.separator();
                    ui_chrome::section_label(ui, "Selected Wafer Process Measurements");
                    wafer_measurements_ui(ui, &measurements);
                } else {
                    ui.columns(2, |columns| {
                        ui_chrome::section_label(&mut columns[0], "Lot / Recipe Comparison");
                        lot_comparison_ui(&mut columns[0], &comparisons);
                        ui_chrome::section_label(
                            &mut columns[1],
                            "Selected Wafer Process Measurements",
                        );
                        wafer_measurements_ui(&mut columns[1], &measurements);
                    });
                }

                ui.separator();
                ui_chrome::section_label(ui, "Measurement Correlation");
                correlation_table_ui(ui, &correlations);
            });

        if self.selected_yield_lot != previous_context_lot {
            self.app_context.set_lot(self.selected_yield_lot.clone());
            self.workflow_panel
                .set_focus_lot(self.selected_yield_lot.clone());
        }
        if self.selected_yield_wafer != previous_context_wafer {
            self.app_context
                .set_wafer(self.selected_yield_wafer.clone());
        }
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
            self.app_context
                .set_wafer(self.selected_yield_wafer.clone());
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
                self.zoom =
                    (self.zoom * (scroll * 0.0015).exp()).clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
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
        self.perf.tile_memory_budget_bytes = tile_stats.memory_budget_bytes.unwrap_or_else(|| {
            warn!(
                default_budget_bytes = TILE_MEMORY_BUDGET_BYTES,
                "tile frame omitted memory budget; using default tile memory budget"
            );
            TILE_MEMORY_BUDGET_BYTES
        });
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
        if self.settings.show_drc_overlay {
            self.draw_violations(&painter, canvas);
            self.draw_drc_overlay_legend(&painter, canvas);
        }
        self.draw_selected_vertex_handles(&painter, canvas);
        self.draw_edit_preview(ui, &painter, canvas);
        self.draw_route_points(&painter, canvas);
        self.draw_remote_selections(&painter, canvas, viewport);
        self.draw_remote_cursors(&painter, canvas);
        self.draw_scale_bar(&painter, canvas);
        self.handle_canvas_input(ui, &response, canvas);
        self.broadcast_selection_if_changed();
        self.perf.frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
        draw_fps_overlay(&painter, canvas, Some(self.perf.frame_ms), None);
    }

    fn canvas_3d(&mut self, ui: &mut egui::Ui) {
        let frame_start = Instant::now();
        let available = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let canvas = response.rect;
        if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
            return;
        }
        let ui_frame_interval_ms = self.record_3d_ui_frame_pacing(frame_start);

        if response.clicked_by(PointerButton::Primary) {
            response.request_focus();
            self.set_flycam_capture(ui.ctx(), true);
        }
        if self.flycam_captured {
            response.request_focus();
            ui.ctx().request_repaint();
        }

        self.handle_3d_input(ui, &response);
        let used_gpu = self.gpu_target_format.is_some();
        let target_size = viewport_3d_target_size(
            [canvas.width(), canvas.height()],
            ui.ctx().pixels_per_point(),
        );
        let view_projection = self.view_projection_3d(canvas);
        let gpu_frame_pacing = if used_gpu {
            self.gpu_frame_pacing_snapshot()
        } else {
            GpuFramePacingSnapshot::default()
        };
        let stats = if let Some(target_format) = self.gpu_target_format {
            let scene = self.cached_3d_scene();
            let stats = scene.stats;
            painter.add(egui_wgpu::Callback::new_paint_callback(
                canvas,
                Viewport3dGpuCallback {
                    batch: scene.batch,
                    fingerprint: scene.fingerprint,
                    uniforms: self.viewport_3d_uniforms(view_projection),
                    viewport_size: [canvas.width(), canvas.height()],
                    target_format,
                    frame_pacing: Arc::clone(&self.gpu_frame_pacing),
                },
            ));
            stats
        } else {
            painter.rect_filled(canvas, 0.0, Color32::from_rgb(8, 11, 14));
            self.draw_3d_ground_grid(&painter, canvas);
            self.draw_3d_layout_cpu_fallback(&painter, canvas)
        };
        self.log_3d_stats_if_needed(stats);
        let canvas_cpu_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
        self.perf.frame_ms = canvas_cpu_ms;
        self.draw_3d_hud(&painter, canvas, stats, gpu_frame_pacing.smoothed_frame_ms);
        self.record_3d_benchmark_frame(
            ui_frame_interval_ms,
            gpu_frame_pacing,
            canvas_cpu_ms,
            used_gpu,
            stats,
            target_size,
        );
    }

    fn record_3d_ui_frame_pacing(&mut self, frame_start: Instant) -> Option<f64> {
        let mut recorded_frame_ms = None;
        if let Some(previous) = self.last_3d_ui_frame_at {
            let frame_ms = frame_start.duration_since(previous).as_secs_f64() * 1000.0;
            if frame_ms.is_finite() && frame_ms > f64::EPSILON {
                recorded_frame_ms = Some(frame_ms);
                self.smoothed_3d_ui_frame_ms =
                    smoothed_frame_interval_ms(self.smoothed_3d_ui_frame_ms, frame_ms);
            }
        }
        self.last_3d_ui_frame_at = Some(frame_start);
        recorded_frame_ms
    }

    fn gpu_frame_pacing_snapshot(&self) -> GpuFramePacingSnapshot {
        self.gpu_frame_pacing
            .lock()
            .map(|pacing| pacing.snapshot())
            .unwrap_or_default()
    }

    fn record_3d_benchmark_frame(
        &mut self,
        ui_frame_ms: Option<f64>,
        gpu_frame_pacing: GpuFramePacingSnapshot,
        canvas_cpu_ms: f64,
        used_gpu: bool,
        stats: Render3dStats,
        target_size: [u32; 2],
    ) {
        let Some(benchmark) = &mut self.benchmark_3d else {
            return;
        };
        if benchmark.completed {
            return;
        }
        let frame_ms = if used_gpu {
            if gpu_frame_pacing.completed_frames == benchmark.last_gpu_completed_frames {
                return;
            }
            benchmark.last_gpu_completed_frames = gpu_frame_pacing.completed_frames;
            let Some(frame_ms) = gpu_frame_pacing.latest_frame_ms else {
                return;
            };
            frame_ms
        } else {
            let Some(frame_ms) = ui_frame_ms else {
                return;
            };
            frame_ms
        };
        if benchmark.warmup_seen < benchmark.options.warmup_frames {
            benchmark.warmup_seen += 1;
            return;
        }
        if used_gpu {
            benchmark.gpu_frames += 1;
        } else {
            benchmark.cpu_frames += 1;
        }
        benchmark.last_stats = stats;
        benchmark.last_target_size = target_size;
        benchmark.intervals.push(frame_ms);
        benchmark.canvas_cpu_ms.push(canvas_cpu_ms);
    }

    fn finish_3d_benchmark_update(
        &mut self,
        update_cpu_ms: f64,
        phase_times: Benchmark3dPhaseTimes,
        ctx: &egui::Context,
    ) {
        let gpu_adapter = self.gpu_adapter.clone();
        let Some(benchmark) = &mut self.benchmark_3d else {
            return;
        };
        if benchmark.completed {
            return;
        }
        if benchmark.update_cpu_ms.len() < benchmark.intervals.len() {
            benchmark.update_cpu_ms.push(update_cpu_ms);
            benchmark.phase_totals.add(phase_times);
            benchmark.phase_samples += 1;
        }
        if benchmark.update_cpu_ms.len() >= benchmark.options.frames {
            benchmark.completed = true;
            print_3d_benchmark_report(
                &benchmark.intervals,
                &benchmark.canvas_cpu_ms,
                &benchmark.update_cpu_ms,
                benchmark.phase_totals,
                benchmark.phase_samples,
                benchmark.gpu_frames,
                benchmark.cpu_frames,
                benchmark.last_stats,
                benchmark.last_target_size,
                gpu_adapter.as_ref(),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn cached_3d_scene(&mut self) -> Cached3dScene {
        if let Some(scene) = &self.cached_3d_scene
            && scene.show_grid_3d == self.settings.show_grid_3d
        {
            return scene.clone();
        }
        let scene = self.build_cached_3d_scene();
        self.cached_3d_scene = Some(scene.clone());
        scene
    }

    fn build_cached_3d_scene(&self) -> Cached3dScene {
        let mut batch = renderer::RenderBatch3d::default();
        let styles = self.layer_3d_styles();
        let object_hint = document_object_count(&self.document);
        let mesh_object_hint = object_hint.min(1_024);
        batch
            .vertices
            .reserve(mesh_object_hint.saturating_mul(5).saturating_mul(4));
        batch
            .indices
            .reserve(mesh_object_hint.saturating_mul(5).saturating_mul(6));
        batch.rect_slabs.reserve(object_hint);
        let mut stats = Render3dStats::default();

        self.document
            .visit_visible_flattened_shape_views(|_, flattened| {
                let Some(style) = styles.get(&flattened.shape.layer).copied() else {
                    return true;
                };
                let added_faces = append_shape_view_3d_to_batch(&mut batch, flattened, style, true);
                if added_faces > 0 {
                    stats.shapes += 1;
                    stats.faces += added_faces;
                }
                true
            });

        self.append_3d_scene_guides(&mut batch);
        let fingerprint = batch.fingerprint();
        Cached3dScene {
            batch: Arc::new(batch),
            fingerprint,
            stats,
            show_grid_3d: self.settings.show_grid_3d,
        }
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
        let canvas_min = canvas.width().min(canvas.height());
        let margin = if canvas_min < 320.0 { 24.0 } else { 56.0 };
        let side = (canvas_min - margin)
            .max(80.0)
            .min((canvas_min - 8.0).max(1.0));
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

            let filtered = self.metrology_failed_only
                && !metrology_site_needs_attention(&self.wafer_map, die, self.metrology_kind);
            let fill = if filtered {
                Color32::from_rgba_unmultiplied(42, 48, 50, 72)
            } else {
                match self.metrology_map_mode {
                    MetrologyMapMode::ValueMap => metrology_measurement_color(measurement, summary),
                    MetrologyMapMode::DeviationMap => metrology_deviation_color(measurement),
                    MetrologyMapMode::SpecWindow => metrology_spec_window_color(measurement),
                    MetrologyMapMode::DefectReview => {
                        metrology_defect_density_color(self.wafer_map.defects_for_die(die).count())
                    }
                    MetrologyMapMode::ReviewQueue => metrology_review_queue_color(
                        metrology_triage_for_die(&self.wafer_map, die).score,
                    ),
                    MetrologyMapMode::OverlayVectors => {
                        Color32::from_rgba_unmultiplied(52, 66, 70, 185)
                    }
                }
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

        if matches!(self.metrology_map_mode, MetrologyMapMode::OverlayVectors) {
            for &die in &self.wafer_map.dies {
                if (die.column + die.row).rem_euclid(2) != 0 {
                    continue;
                }
                let Some(vector) = metrology_overlay_vector(&self.wafer_map, die) else {
                    continue;
                };
                let origin = wafer_mm_to_screen(geometry.die_center_mm(die), center, scale);
                let delta = vec2(vector[0], -vector[1]);
                if delta.length_sq() <= 0.1 {
                    continue;
                }
                painter.line_segment(
                    [origin, origin + delta],
                    Stroke::new(1.1, Color32::from_rgb(255, 222, 116)),
                );
                painter.circle_filled(origin + delta, 2.0, Color32::from_rgb(255, 222, 116));
            }
        }

        if matches!(
            self.metrology_map_mode,
            MetrologyMapMode::DefectReview | MetrologyMapMode::ReviewQueue
        ) || self.metrology_kind == MeasurementKind::DefectCount
            || self.selected_die.is_some()
        {
            for defect in &self.wafer_map.defects {
                if !matches!(
                    self.metrology_map_mode,
                    MetrologyMapMode::DefectReview | MetrologyMapMode::ReviewQueue
                ) && self.metrology_kind != MeasurementKind::DefectCount
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

        if let Some(die) = self.selected_die {
            if canvas.width() > 340.0 {
                let badge_width = (canvas.width() - 32.0).min(230.0);
                let badge = EguiRect::from_min_size(
                    Pos2::new(canvas.right() - badge_width - 16.0, canvas.top() + 14.0),
                    vec2(badge_width, 58.0),
                );
                painter.rect_filled(badge, 4.0, Color32::from_rgba_unmultiplied(16, 19, 21, 220));
                painter.rect_stroke(
                    badge,
                    4.0,
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 220, 216, 80)),
                    StrokeKind::Inside,
                );
                let value = self
                    .wafer_map
                    .measurement_for(die, self.metrology_kind)
                    .map(|measurement| format_metrology_value(measurement.kind, measurement.value))
                    .unwrap_or_else(|| "no record".to_string());
                let site = metrology_triage_for_die(&self.wafer_map, die);
                painter.text(
                    badge.left_top() + vec2(10.0, 9.0),
                    Align2::LEFT_TOP,
                    format!("Selected C{} R{}", die.column, die.row),
                    FontId::proportional(13.0),
                    Color32::from_rgb(238, 244, 240),
                );
                painter.text(
                    badge.left_top() + vec2(10.0, 30.0),
                    Align2::LEFT_TOP,
                    format!("{}  {} defect(s)", value, site.defect_count),
                    FontId::monospace(11.0),
                    Color32::from_rgb(204, 215, 210),
                );
            }
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
            format!(
                "{} / {}",
                self.metrology_map_mode.label(),
                self.metrology_kind.label()
            ),
            FontId::proportional(18.0),
            Color32::from_rgb(240, 244, 236),
        );

        if let Some(die) = hovered_die {
            let measurement_text = self
                .wafer_map
                .measurement_for(die, self.metrology_kind)
                .map(|measurement| {
                    format!(
                        "{} {} ({})",
                        measurement.kind.label(),
                        format_metrology_value(measurement.kind, measurement.value),
                        measurement.status.label()
                    )
                })
                .unwrap_or_else(|| "no selected measurement".to_string());
            let site = metrology_triage_for_die(&self.wafer_map, die);
            response.on_hover_text(format!(
                "C{} R{}\n{}\n{} fail / {} outlier / {} defect",
                die.column,
                die.row,
                measurement_text,
                site.fail_count,
                site.outlier_count,
                site.defect_count
            ));
        }
    }

    fn handle_3d_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if self.flycam_captured && ui.input(|input| input.key_pressed(Key::Escape)) {
            self.set_flycam_capture(ui.ctx(), false);
            return;
        }
        if self.flycam_captured && response.clicked_by(PointerButton::Secondary) {
            self.set_flycam_capture(ui.ctx(), false);
            return;
        }

        if self.flycam_captured
            || response.dragged_by(PointerButton::Primary)
            || response.dragged_by(PointerButton::Secondary)
        {
            let delta = ui.input(|input| {
                if self.flycam_captured {
                    input
                        .pointer
                        .motion()
                        .unwrap_or_else(|| input.pointer.delta())
                } else {
                    input.pointer.delta()
                }
            });
            let sensitivity = if self.flycam_captured { 0.003 } else { 0.006 };
            self.camera_3d.yaw += delta.x * sensitivity;
            self.camera_3d.pitch =
                (self.camera_3d.pitch - delta.y * sensitivity).clamp(-1.45, 1.45);
        }

        if !self.flycam_captured
            && !response.hovered()
            && !response.has_focus()
            && !response.dragged()
        {
            return;
        }

        let basis = self.camera_3d.basis();
        ui.input(|input| {
            if input.key_pressed(Key::R) && !input.modifiers.any() {
                self.camera_3d.speed = (self.camera_3d.speed * 1.5).min(2_000_000.0);
                self.status = format!("3D speed {:.0}", self.camera_3d.speed);
            }
            if input.key_pressed(Key::F) && !input.modifiers.any() {
                self.camera_3d.speed = (self.camera_3d.speed / 1.5).max(100.0);
                self.status = format!("3D speed {:.0}", self.camera_3d.speed);
            }
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

    fn draw_3d_layout_cpu_fallback(&self, painter: &Painter, canvas: EguiRect) -> Render3dStats {
        let mut faces = Vec::new();
        let mut stats = self.collect_visible_3d_faces(canvas, &mut faces);

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
        stats.faces = rendered;
        stats
    }

    fn collect_visible_3d_faces(&self, canvas: EguiRect, faces: &mut Vec<Face3d>) -> Render3dStats {
        let (occurrences, capped) = self.visible_3d_occurrences(canvas);
        let mut stats = Render3dStats {
            capped,
            ..Default::default()
        };
        for occurrence in occurrences {
            if faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
                stats.cpu_face_capped = true;
                break;
            }
            let Some(flattened) = self.document.shape_view_for_occurrence(&occurrence) else {
                continue;
            };
            let shape = flattened.transformed_shape();
            let before = faces.len();
            self.add_shape_3d_faces(faces, &shape);
            if faces.len() > before {
                stats.shapes += 1;
                stats.faces = faces.len();
            }
            if faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
                stats.cpu_face_capped = true;
                break;
            }
        }
        stats
    }

    fn visible_3d_occurrences(&self, canvas: EguiRect) -> (Vec<ShapeOccurrenceId>, bool) {
        let Some(query) = self.visible_3d_query_rect(canvas) else {
            return (Vec::new(), false);
        };
        let mut occurrences = self
            .index
            .query_occurrences_limited(query, MAX_3D_RENDERED_SHAPES.saturating_add(1));
        let capped = occurrences.len() > MAX_3D_RENDERED_SHAPES;
        if capped {
            occurrences.truncate(MAX_3D_RENDERED_SHAPES);
        }
        (occurrences, capped)
    }

    fn visible_3d_query_rect(&self, canvas: EguiRect) -> Option<Rect> {
        let query = camera_frustum_xy_rect(
            self.camera_3d,
            canvas,
            CAMERA_NEAR_PLANE,
            self.camera_3d_far_plane(),
            0.0,
            self.max_scene_3d_z(),
        )
        .or_else(|| self.layout_bounds())?;
        self.layout_bounds()
            .and_then(|bounds| query.intersection(bounds.expanded(self.snap_grid() * 8)))
            .map(|rect| rect.expanded(self.snap_grid() * 4))
    }

    fn max_scene_3d_z(&self) -> f32 {
        self.document.layers.len().max(1) as f32 * 320.0 + 2_000.0
    }

    fn log_3d_stats_if_needed(&mut self, stats: Render3dStats) {
        if stats.capped && !self.logged_3d_shape_cap {
            warn!(
                rendered_shapes = stats.shapes,
                shape_budget = MAX_3D_RENDERED_SHAPES,
                "3D renderer visible shape count exceeded render budget"
            );
            self.logged_3d_shape_cap = true;
        } else if !stats.capped {
            self.logged_3d_shape_cap = false;
        }

        if stats.cpu_face_capped && !self.logged_3d_cpu_face_cap {
            warn!(
                rendered_faces = stats.faces,
                face_budget = MAX_3D_CPU_FALLBACK_FACES,
                "3D CPU fallback exceeded face budget; truncating rendered faces"
            );
            self.logged_3d_cpu_face_cap = true;
        } else if !stats.cpu_face_capped {
            self.logged_3d_cpu_face_cap = false;
        }
    }

    fn viewport_3d_uniforms(&self, view_projection: [f32; 16]) -> Viewport3dUniforms {
        Viewport3dUniforms::from_view_projection(view_projection)
            .with_rect_side_faces(self.rect_slab_side_faces_3d())
    }

    fn rect_slab_side_faces_3d(&self) -> [u32; 2] {
        rect_slab_side_faces_for_forward(self.camera_3d.forward())
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
            return CAMERA_FAR_PLANE_MIN;
        };
        let max_layer_z = self.max_scene_3d_z();
        let span = bounds.width().abs().max(bounds.height().abs()).max(1_000) as f32;
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
        let margin = span * CAMERA_FAR_PLANE_MARGIN_MULTIPLIER + max_layer_z * 2.0;
        let minimum_far =
            CAMERA_FAR_PLANE_MIN.max(span * CAMERA_FAR_PLANE_SPAN_MULTIPLIER + max_layer_z);
        (max_forward_depth + margin).max(minimum_far)
    }

    fn add_shape_3d_faces(&self, faces: &mut Vec<Face3d>, shape: &Shape) {
        if faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
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
                    if faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
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
                add_slab_faces(faces, &rect.corners(), base_z, top_z, color);
            }
            ShapeKind::Label { .. } | ShapeKind::Measurement { .. } | ShapeKind::Polygon(_) => {}
        }
    }

    fn draw_3d_ground_grid(&self, painter: &Painter, canvas: EguiRect) {
        if !self.settings.show_grid_3d {
            return;
        }
        let bounds = self.layout_bounds().unwrap_or_else(|| {
            warn!("3D ground grid found no layout bounds; using default grid bounds");
            Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000))
        });
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
        let bounds = self.layout_bounds().unwrap_or_else(|| {
            warn!("3D scene guides found no layout bounds; using default grid bounds");
            Rect::new(Point::new(-5_000, -5_000), Point::new(5_000, 5_000))
        });
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

    fn draw_3d_hud(
        &self,
        painter: &Painter,
        canvas: EguiRect,
        stats: Render3dStats,
        gpu_frame_ms: Option<f64>,
    ) {
        if stats.capped || stats.cpu_face_capped {
            painter.text(
                canvas.left_top() + vec2(12.0, 12.0),
                Align2::LEFT_TOP,
                "3D render capped",
                FontId::monospace(11.0),
                Color32::from_rgba_premultiplied(220, 232, 228, 170),
            );
        }
        if canvas.width() >= 520.0 && canvas.height() >= 150.0 {
            painter.text(
                canvas.left_bottom() + vec2(12.0, -12.0),
                Align2::LEFT_BOTTOM,
                "WASD move  Q/E down/up  R/F speed  Shift faster  Esc release  F11 fullscreen",
                FontId::monospace(11.0),
                Color32::from_rgba_premultiplied(220, 232, 228, 150),
            );
        } else if canvas.width() >= 300.0 && canvas.height() >= 150.0 {
            painter.text(
                canvas.left_bottom() + vec2(12.0, -12.0),
                Align2::LEFT_BOTTOM,
                "WASD  Q/E  R/F  F11",
                FontId::monospace(11.0),
                Color32::from_rgba_premultiplied(220, 232, 228, 150),
            );
        }
        draw_fps_overlay(
            painter,
            canvas,
            gpu_frame_ms.or(self.smoothed_3d_ui_frame_ms),
            (canvas.width() >= 520.0).then(|| format!("speed {:.0}", self.camera_3d.speed)),
        );
    }
}

fn draw_fps_overlay(
    painter: &Painter,
    canvas: EguiRect,
    frame_ms: Option<f64>,
    detail: Option<String>,
) {
    if canvas.width() < 120.0 || canvas.height() < 80.0 {
        return;
    }
    let mut text = format_3d_fps_label(frame_ms, None);
    if let Some(detail) = detail {
        text.push_str("  ");
        text.push_str(&detail);
    }
    painter.text(
        canvas.right_bottom() + vec2(-12.0, -10.0),
        Align2::RIGHT_BOTTOM,
        text,
        FontId::monospace(11.0),
        Color32::from_rgba_premultiplied(220, 232, 228, 155),
    );
}

fn format_3d_fps_label(gpu_frame_ms: Option<f64>, _ui_frame_ms: Option<f64>) -> String {
    match frames_per_second(gpu_frame_ms) {
        Some(fps) => format!("FPS: {fps:.1}"),
        None => "FPS: --".to_string(),
    }
}

fn frames_per_second(frame_ms: Option<f64>) -> Option<f64> {
    let frame_ms = frame_ms?;
    if frame_ms.is_finite() && frame_ms > f64::EPSILON {
        Some(1000.0 / frame_ms)
    } else {
        None
    }
}

fn smoothed_frame_interval_ms(previous: Option<f64>, frame_ms: f64) -> Option<f64> {
    if frame_ms > 1_000.0 {
        None
    } else {
        Some(match previous {
            Some(previous_ms) if previous_ms.is_finite() => previous_ms * 0.82 + frame_ms * 0.18,
            _ => frame_ms,
        })
    }
}

fn print_3d_benchmark_report(
    intervals: &[f64],
    canvas_cpu_ms: &[f64],
    update_cpu_ms: &[f64],
    phase_totals: Benchmark3dPhaseTimes,
    phase_samples: usize,
    gpu_frames: usize,
    cpu_frames: usize,
    stats: Render3dStats,
    target_size: [u32; 2],
    gpu_adapter: Option<&GpuAdapterSummary>,
) {
    if intervals.is_empty() {
        println!("3d benchmark: no frame intervals collected");
        return;
    }
    let mut sorted = intervals.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let average = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let p50 = percentile_sorted(&sorted, 0.50);
    let p95 = percentile_sorted(&sorted, 0.95);
    let mut canvas_sorted = canvas_cpu_ms.to_vec();
    canvas_sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let mut update_sorted = update_cpu_ms.to_vec();
    update_sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let canvas_average = canvas_sorted.iter().sum::<f64>() / canvas_sorted.len().max(1) as f64;
    let update_average = update_sorted.iter().sum::<f64>() / update_sorted.len().max(1) as f64;
    let canvas_p50 = percentile_sorted(&canvas_sorted, 0.50);
    let update_p50 = percentile_sorted(&update_sorted, 0.50);
    let phase_divisor = phase_samples.max(1) as f64;
    let interval_source = match (gpu_frames > 0, cpu_frames > 0) {
        (true, false) => "gpu_completion",
        (false, true) => "ui_frame",
        (true, true) => "mixed",
        (false, false) => "unknown",
    };
    println!(
        "3d benchmark: frames={} interval_source={} gpu_frames={} cpu_frames={} adapter_backend={} adapter_type={} adapter=\"{}\" target={}x{} shapes={} faces={} top_caps_only={} capped={} cpu_face_capped={} avg_ms={:.3} p50_ms={:.3} p95_ms={:.3} min_ms={:.3} max_ms={:.3} avg_fps={:.1} p50_fps={:.1} canvas_avg_ms={:.3} canvas_p50_ms={:.3} update_avg_ms={:.3} update_p50_ms={:.3} phase_overhead_ms={:.3} phase_toolbar_ms={:.3} phase_navigation_ms={:.3} phase_inspector_ms={:.3} phase_layers_ms={:.3} phase_windows_ms={:.3} phase_mes_ms={:.3} phase_central_ms={:.3}",
        sorted.len(),
        interval_source,
        gpu_frames,
        cpu_frames,
        gpu_adapter.map_or("unknown", |adapter| adapter.backend.as_str()),
        gpu_adapter.map_or("unknown", |adapter| adapter.device_type.as_str()),
        gpu_adapter.map_or("unknown", |adapter| adapter.name.as_str()),
        target_size[0],
        target_size[1],
        stats.shapes,
        stats.faces,
        stats.top_caps_only,
        stats.capped,
        stats.cpu_face_capped,
        average,
        p50,
        p95,
        min,
        max,
        1000.0 / average,
        1000.0 / p50,
        canvas_average,
        canvas_p50,
        update_average,
        update_p50,
        phase_totals.overhead / phase_divisor,
        phase_totals.toolbar / phase_divisor,
        phase_totals.navigation / phase_divisor,
        phase_totals.inspector / phase_divisor,
        phase_totals.layers / phase_divisor,
        phase_totals.windows / phase_divisor,
        phase_totals.mes / phase_divisor,
        phase_totals.central / phase_divisor
    );
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let index = ((values.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
    values[index]
}

fn take_elapsed_ms(cursor: &mut Instant) -> f64 {
    let now = Instant::now();
    let elapsed = now.duration_since(*cursor).as_secs_f64() * 1000.0;
    *cursor = now;
    elapsed
}

impl FabricadApp {
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
        let (base_z, thickness) = layer_3d_stack_position(layer.process);
        Some((base_z, base_z + thickness, layer_color_3d(layer.color)))
    }

    fn layer_3d_styles(&self) -> BTreeMap<LayerId, Layer3dStyle> {
        let mut layer_order: Vec<_> = self
            .document
            .layers
            .values()
            .filter(|layer| !matches!(layer.process, ProcessLayer::Annotation))
            .map(|layer| {
                (
                    layer.display_order,
                    layer.id,
                    layer.process,
                    layer.color,
                    layer.visible,
                )
            })
            .collect();
        layer_order.sort_unstable_by_key(|(display_order, id, _, _, _)| (*display_order, *id));
        layer_order
            .iter()
            .filter_map(|(_, id, process, color, visible)| {
                if !visible {
                    return None;
                }
                let (base_z, thickness) = layer_3d_stack_position(*process);
                Some((
                    *id,
                    Layer3dStyle {
                        base_z,
                        top_z: base_z + thickness,
                        color: layer_color_3d(*color),
                    },
                ))
            })
            .collect()
    }

    fn layout_bounds(&self) -> Option<Rect> {
        self.layout_bounds_cache
    }

    fn draw_background(&self, painter: &Painter, canvas: EguiRect, viewport: Rect) {
        painter.rect_filled(canvas, 0.0, Color32::from_rgb(13, 16, 18));
        if !self.settings.show_grid_2d {
            if self.settings.show_origin_marker {
                self.draw_origin_marker(painter, canvas);
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
            self.draw_origin_marker(painter, canvas);
        }
    }

    fn draw_origin_marker(&self, painter: &Painter, canvas: EguiRect) {
        let origin = self.world_to_screen(Point::ZERO, canvas);
        if !canvas.expand(18.0).contains(origin) {
            return;
        }
        let stroke = Stroke::new(1.0, Color32::from_rgba_premultiplied(220, 220, 210, 145));
        painter.line_segment([origin + vec2(-6.0, 0.0), origin + vec2(6.0, 0.0)], stroke);
        painter.line_segment([origin + vec2(0.0, -6.0), origin + vec2(0.0, 6.0)], stroke);
        painter.text(
            origin + vec2(8.0, -8.0),
            Align2::LEFT_BOTTOM,
            "0,0",
            FontId::monospace(10.0),
            Color32::from_rgba_premultiplied(220, 220, 210, 130),
        );
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
            for occurrence in visible_occurrences {
                let Some(shape) = self.document.shape_view_for_occurrence(occurrence) else {
                    continue;
                };
                if shape.bounds.intersects(viewport) {
                    let shape = shape.transformed_shape();
                    self.draw_shape_for_occurrence(painter, canvas, occurrence, &shape);
                }
            }
        } else {
            for occurrence in visible_occurrences {
                if let Some(shape) = self.document.shapes.get(&occurrence.source_shape_id()) {
                    self.draw_shape_for_occurrence(painter, canvas, occurrence, &shape);
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
            for occurrence in visible_occurrences {
                let Some(shape) = self.document.shape_view_for_occurrence(occurrence) else {
                    continue;
                };
                if shape.bounds.intersects(viewport) {
                    let shape = shape.transformed_shape();
                    self.draw_shape_overlay_for_occurrence(painter, canvas, occurrence, &shape);
                }
            }
        } else {
            for occurrence in visible_occurrences {
                if let Some(shape) = self.document.shapes.get(&occurrence.source_shape_id()) {
                    self.draw_shape_overlay_for_occurrence(painter, canvas, occurrence, &shape);
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
        for occurrence in &highlighted {
            let Some(flattened) = self.document.shape_view_for_occurrence(occurrence) else {
                continue;
            };
            if !flattened.bounds.intersects(viewport) {
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
        let mut drawn = 0usize;
        for violation in &self.violations {
            let state = self.marker_state(violation);
            if state.hidden || state.waived {
                continue;
            }
            if drawn >= MAX_DRC_OVERLAY_MARKERS {
                break;
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
            drawn += 1;
        }
    }

    fn draw_drc_overlay_legend(&self, painter: &Painter, canvas: EguiRect) {
        let active = self.active_marker_count();
        if active == 0 || canvas.width() < 220.0 || canvas.height() < 90.0 {
            return;
        }
        let drawn = active.min(MAX_DRC_OVERLAY_MARKERS);
        let text = if drawn < active {
            format!("DRC overlay: first {drawn} of {active} active")
        } else {
            format!("DRC overlay: {active} active")
        };
        painter.text(
            canvas.right_top() + vec2(-12.0, 12.0),
            Align2::RIGHT_TOP,
            text,
            FontId::monospace(11.0),
            Color32::from_rgba_premultiplied(255, 190, 190, 165),
        );
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
            for occurrence in &selected {
                let Some(flattened) = self.document.shape_view_for_occurrence(occurrence) else {
                    continue;
                };
                if !flattened.bounds.intersects(viewport) {
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
        let state = match self.gpu_pick_state.lock() {
            Ok(state) => state,
            Err(err) => {
                error!(error = %err, "GPU pick state mutex was poisoned");
                return None;
            }
        };
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
        let update_start = Instant::now();
        let mut phase_cursor = update_start;
        let mut phase_times = Benchmark3dPhaseTimes::default();
        self.apply_theme(ctx);
        self.poll_collaboration();
        self.maybe_autosave();
        self.advance_equipment_simulator();
        self.handle_shortcuts(ctx);
        if !matches!(self.view_mode, ViewMode::Layout3d) {
            self.set_flycam_capture(ctx, false);
        }
        phase_times.overhead += take_elapsed_ms(&mut phase_cursor);
        if self.viewport_fullscreen && self.view_mode.supports_viewport_fullscreen() {
            self.viewport_fullscreen_ui(ctx);
            phase_times.central += take_elapsed_ms(&mut phase_cursor);
            let update_cpu_ms = update_start.elapsed().as_secs_f64() * 1000.0;
            self.finish_3d_benchmark_update(update_cpu_ms, phase_times, ctx);
            ctx.request_repaint();
            return;
        }
        if self.viewport_fullscreen {
            self.viewport_fullscreen = false;
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        phase_times.toolbar += take_elapsed_ms(&mut phase_cursor);
        self.navigation_panel(ctx);
        phase_times.navigation += take_elapsed_ms(&mut phase_cursor);
        let inline_side_panels = Self::inline_side_panels(ctx);
        if inline_side_panels && self.view_mode.has_inspector_panel() {
            self.inspector_panel(ctx);
        }
        phase_times.inspector += take_elapsed_ms(&mut phase_cursor);
        if inline_side_panels && self.view_mode.has_secondary_panel() {
            self.layers_panel(ctx);
        }
        phase_times.layers += take_elapsed_ms(&mut phase_cursor);
        self.options_window(ctx);
        self.diagnostics_window(ctx);
        self.sidebar_modules_window(ctx);
        self.command_palette_window(ctx);
        self.responsive_panel_windows(ctx);
        phase_times.windows += take_elapsed_ms(&mut phase_cursor);
        self.mes_panel(ctx);
        phase_times.mes += take_elapsed_ms(&mut phase_cursor);
        egui::CentralPanel::default().show(ctx, |ui| match self.view_mode {
            ViewMode::Workflow => {
                self.sync_workflow_focus_from_context();
                let data = WorkflowData {
                    document: &self.document,
                    process_flow: self.process_flow_panel.model(),
                    recipes: self.recipe_panel.catalog(),
                    mes: &self.mes,
                    inventory: self.inventory_panel.inventory(),
                    maintenance: self.maintenance_panel.model(),
                    environment: self.environment_panel.model(),
                    scheduler: self.scheduler_panel.schedule(),
                    safety: self.safety_panel.model(),
                    equipment: &self.equipment_sim,
                    wafer_map: &self.wafer_map,
                    yield_analysis: &self.yield_analysis,
                    notebook: self.notebook_panel.notebook(),
                };
                if let Some(action) = self.workflow_panel.ui(ui, data) {
                    self.handle_workflow_action(action);
                }
                self.sync_context_from_workflow();
            }
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
            ViewMode::LayoutDiff => self.layout_diff_panel.ui(ui, &self.document),
            ViewMode::Inventory => self.inventory_panel.ui(ui, &mut self.status),
            ViewMode::Maintenance => self.maintenance_panel.ui(ui, &mut self.status),
            ViewMode::Environment => self.environment_panel.ui(ui),
            ViewMode::Scheduler => self.scheduler_panel.ui(ui, &mut self.status),
            ViewMode::Safety => self.safety_panel.ui(ui, &mut self.status),
            ViewMode::SpcFdc => {
                self.spc_fdc_panel
                    .ui(ui, &self.yield_analysis, &self.equipment_sim);
            }
            ViewMode::ProcessFlow => {
                self.process_flow_panel.ui(ui, &mut self.status);
            }
            ViewMode::ProcessControl => {
                self.process_control_panel
                    .ui(ui, &self.yield_analysis, &mut self.status);
            }
            ViewMode::CrossSection => self.cross_section_panel.ui(ui, &mut self.status),
            ViewMode::Traceability => self.genealogy_panel.ui(ui),
            ViewMode::Experiment => self.experiment_panel.dashboard_ui(ui, &mut self.status),
            ViewMode::Notebook => self.notebook_panel.ui(ui, &mut self.status),
        });
        phase_times.central += take_elapsed_ms(&mut phase_cursor);
        let update_cpu_ms = update_start.elapsed().as_secs_f64() * 1000.0;
        self.finish_3d_benchmark_update(update_cpu_ms, phase_times, ctx);
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

type SharedGpuFramePacingState = Arc<Mutex<GpuFramePacingState>>;

#[derive(Clone, Copy, Debug, Default)]
struct GpuFramePacingSnapshot {
    smoothed_frame_ms: Option<f64>,
    latest_frame_ms: Option<f64>,
    completed_frames: u64,
}

#[derive(Debug, Default)]
struct GpuFramePacingState {
    completion_pending: bool,
    last_completed_at: Option<Instant>,
    smoothed_frame_ms: Option<f64>,
    latest_frame_ms: Option<f64>,
    completed_frames: u64,
}

impl GpuFramePacingState {
    fn snapshot(&self) -> GpuFramePacingSnapshot {
        GpuFramePacingSnapshot {
            smoothed_frame_ms: self.smoothed_frame_ms,
            latest_frame_ms: self.latest_frame_ms,
            completed_frames: self.completed_frames,
        }
    }

    fn record_completed(&mut self, completed_at: Instant) {
        self.completion_pending = false;
        if let Some(previous) = self.last_completed_at {
            let frame_ms = completed_at.duration_since(previous).as_secs_f64() * 1000.0;
            if frame_ms.is_finite() && frame_ms > f64::EPSILON {
                self.latest_frame_ms = Some(frame_ms);
                self.smoothed_frame_ms =
                    smoothed_frame_interval_ms(self.smoothed_frame_ms, frame_ms);
                self.completed_frames += 1;
            }
        }
        self.last_completed_at = Some(completed_at);
    }
}

struct Viewport3dGpuCallback {
    batch: Arc<renderer::RenderBatch3d>,
    fingerprint: renderer::BatchFingerprint,
    uniforms: Viewport3dUniforms,
    viewport_size: [f32; 2],
    target_format: egui_wgpu::wgpu::TextureFormat,
    frame_pacing: SharedGpuFramePacingState,
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
            resources.upload_with_fingerprint(
                device,
                queue,
                &self.batch,
                self.fingerprint,
                self.uniforms,
            );
            let target_size =
                viewport_3d_target_size(self.viewport_size, screen_descriptor.pixels_per_point);
            let clear_color = egui_wgpu::wgpu::Color {
                r: 8.0 / 255.0,
                g: 11.0 / 255.0,
                b: 14.0 / 255.0,
                a: 1.0,
            };
            resources.render_to_texture(device, egui_encoder, target_size, clear_color);
            schedule_3d_gpu_frame_completion(queue, Arc::clone(&self.frame_pacing));
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

fn schedule_3d_gpu_frame_completion(
    queue: &egui_wgpu::wgpu::Queue,
    frame_pacing: SharedGpuFramePacingState,
) {
    let Ok(mut pacing) = frame_pacing.lock() else {
        return;
    };
    if pacing.completion_pending {
        return;
    }
    pacing.completion_pending = true;
    drop(pacing);

    queue.on_submitted_work_done(move || {
        if let Ok(mut pacing) = frame_pacing.lock() {
            pacing.record_completed(Instant::now());
        }
    });
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

fn camera_frustum_xy_rect(
    camera: Camera3d,
    canvas: EguiRect,
    near_depth: f32,
    far_depth: f32,
    min_z: f32,
    max_z: f32,
) -> Option<Rect> {
    if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
        return None;
    }
    let near_depth = near_depth.max(0.001);
    let far_depth = far_depth.max(near_depth + 1.0);
    let (min_z, max_z) = if min_z <= max_z {
        (min_z, max_z)
    } else {
        (max_z, min_z)
    };
    let basis = camera.basis();
    let aspect = (canvas.width() / canvas.height()).max(0.001);
    let tan_y = (camera.fov_y * 0.5).tan();
    let tan_x = tan_y * aspect;
    let ndc = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let mut vertices = Vec::with_capacity(8);
    for depth in [near_depth, far_depth] {
        for (x, y) in ndc {
            let ray = basis.forward + basis.right * (x * tan_x) + basis.up * (y * tan_y);
            vertices.push(camera.position + ray * depth);
        }
    }

    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let mut points = Vec::with_capacity(16);
    for vertex in &vertices {
        if vertex.z >= min_z && vertex.z <= max_z {
            points.push(*vertex);
        }
    }
    for (a, b) in edges {
        let a = vertices[a];
        let b = vertices[b];
        for z in [min_z, max_z] {
            let dz = b.z - a.z;
            if dz.abs() <= 0.0001 {
                continue;
            }
            let t = (z - a.z) / dz;
            if (0.0..=1.0).contains(&t) {
                points.push(a + (b - a) * t);
            }
        }
    }

    let mut bounds = None;
    for point in points {
        let Some(point) = finite_point_from_xy(point.x, point.y) else {
            continue;
        };
        bounds = Some(bounds.map_or(Rect::new(point, point), |rect: Rect| {
            rect.union(Rect::new(point, point))
        }));
    }
    bounds
}

fn finite_point_from_xy(x: f32, y: f32) -> Option<Point> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(Point::new(coord_from_f32(x), coord_from_f32(y)))
}

fn rect_slab_side_faces_for_forward(forward: Vec3f) -> [u32; 2] {
    let x_face = if forward.x >= 0.0 { 4 } else { 2 };
    let y_face = if forward.y >= 0.0 { 1 } else { 3 };
    let abs_x = forward.x.abs();
    let abs_y = forward.y.abs();
    let edge_on_threshold = 0.25;
    if abs_x < abs_y * edge_on_threshold {
        [y_face, y_face]
    } else if abs_y < abs_x * edge_on_threshold {
        [x_face, x_face]
    } else {
        [x_face, y_face]
    }
}

fn coord_from_f32(value: f32) -> Coord {
    let limit = (Coord::MAX / 4) as f32;
    let clamped = value.clamp(-limit, limit);
    if clamped != value {
        warn!(
            value,
            effective_value = clamped,
            "f32 coordinate exceeded supported Coord conversion range"
        );
    }
    clamped.round() as Coord
}

fn rule_deck_for_document(document: &Document, technology: &TechnologyFile) -> RuleDeck {
    if document.layers.is_empty() {
        return empty_rule_deck(document.grid);
    }
    RuleDeck::from_technology(document, technology).unwrap_or_else(|err| {
        error!("technology rule load failed: {err}; DRC rules disabled for this document");
        empty_rule_deck(document.grid)
    })
}

fn empty_rule_deck(grid: Coord) -> RuleDeck {
    RuleDeck {
        grid,
        min_width: BTreeMap::new(),
        min_spacing: BTreeMap::new(),
        via_enclosure: Vec::new(),
        forbidden_overlaps: Vec::new(),
    }
}

fn connectivity_report_for_document(
    document: &Document,
    technology: &TechnologyFile,
) -> ConnectivityReport {
    let object_count = document_object_count(document);
    if object_count > MAX_CONNECTIVITY_OBJECTS {
        warn!(
            object_count,
            max_connectivity_objects = MAX_CONNECTIVITY_OBJECTS,
            "net extraction skipped because document exceeds connectivity budget"
        );
        return ConnectivityReport::skipped(format!(
            "net extraction skipped for {object_count} objects"
        ));
    }
    extract_connectivity(document, technology).unwrap_or_else(|err| {
        error!("net extraction failed: {err}");
        ConnectivityReport::skipped(format!("net extraction failed: {err}"))
    })
}

fn drc_skip_message(document: &Document) -> Option<String> {
    let object_count = document_object_count(document);
    (object_count > MAX_DRC_OBJECTS)
        .then(|| format!("DRC skipped for {object_count} objects; limit is {MAX_DRC_OBJECTS}"))
}

fn document_object_count(document: &Document) -> usize {
    document.shapes.len()
        + document
            .cells
            .values()
            .map(|cell| 1 + cell.shapes.len() + cell.instances.len())
            .sum::<usize>()
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
            ShapeKind::Rectangle(Rect::from_points(&points).unwrap_or_else(|| {
                warn!("rectangle transform produced no points; using default bounds");
                Rect::default()
            }))
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
    let text = match serde_json::to_string(message) {
        Ok(text) => text,
        Err(err) => {
            error!("failed to serialize collaboration client message: {err}");
            return;
        }
    };
    if let Err(err) = socket.send_with_str(&text) {
        warn!(?err, "failed to send collaboration client message");
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_collaboration_url() -> String {
    let Some(window) = web_sys::window() else {
        warn!("browser window missing; using default collaboration URL");
        return "ws://127.0.0.1:4141/ws".to_string();
    };
    let location = window.location();
    let search = location.search().unwrap_or_else(|err| {
        warn!(
            ?err,
            "failed to read URL search parameters; using default collaboration URL"
        );
        String::new()
    });
    if let Some(url) = query_param(&search, "sync") {
        return url;
    }
    let protocol = location.protocol().unwrap_or_else(|err| {
        warn!(
            ?err,
            "failed to read browser protocol; using ws collaboration URL"
        );
        String::new()
    });
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    let host = location
        .hostname()
        .ok()
        .filter(|host| !host.is_empty())
        .unwrap_or_else(|| {
            warn!("browser hostname missing; using 127.0.0.1 for collaboration URL");
            "127.0.0.1".to_string()
        });
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

fn menu_hotkey_label(ui: &egui::Ui, label: &str, hotkey_index: usize) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    let base = egui::TextFormat {
        font_id: FontId::proportional(14.0),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let underline = egui::TextFormat {
        underline: Stroke::new(1.0, ui.visuals().text_color()),
        ..base.clone()
    };
    for (index, character) in label.chars().enumerate() {
        let format = if index == hotkey_index {
            underline.clone()
        } else {
            base.clone()
        };
        job.append(&character.to_string(), 0.0, format);
    }
    job.into()
}

fn append_shape_view_3d_to_batch(
    batch: &mut renderer::RenderBatch3d,
    flattened: FlattenedShapeView<'_>,
    style: Layer3dStyle,
    include_sides: bool,
) -> usize {
    match flattened.shape.kind {
        ShapeKindView::Rectangle(rect) => append_rect_slab_to_3d_batch(
            batch,
            flattened.transform.apply_rect(rect),
            style.base_z,
            style.top_z,
            style.color,
            include_sides,
        ),
        ShapeKindView::Polygon(poly) if poly.points.len() >= 3 => {
            if flattened.transform == Transform::IDENTITY {
                append_slab_to_3d_batch(
                    batch,
                    &poly.points,
                    style.base_z,
                    style.top_z,
                    style.color,
                    include_sides,
                )
            } else {
                let points = poly
                    .points
                    .iter()
                    .copied()
                    .map(|point| flattened.transform.apply_point(point))
                    .collect::<Vec<_>>();
                append_slab_to_3d_batch(
                    batch,
                    &points,
                    style.base_z,
                    style.top_z,
                    style.color,
                    include_sides,
                )
            }
        }
        ShapeKindView::Path { points, width } => {
            let mut faces = 0;
            if flattened.transform == Transform::IDENTITY {
                for segment in points.windows(2) {
                    if let [a, b] = segment {
                        faces += append_path_segment_to_3d_batch(
                            batch,
                            *a,
                            *b,
                            width,
                            style.base_z,
                            style.top_z,
                            style.color,
                            include_sides,
                        );
                    }
                }
            } else {
                let points = points
                    .iter()
                    .copied()
                    .map(|point| flattened.transform.apply_point(point))
                    .collect::<Vec<_>>();
                for segment in points.windows(2) {
                    if let [a, b] = segment {
                        faces += append_path_segment_to_3d_batch(
                            batch,
                            *a,
                            *b,
                            width,
                            style.base_z,
                            style.top_z,
                            style.color,
                            include_sides,
                        );
                    }
                }
            }
            faces
        }
        ShapeKindView::Via { center, size, .. } => {
            let center = flattened.transform.apply_point(center);
            let half = size / 2;
            let rect = Rect::new(
                Point::new(center.x - half, center.y - half),
                Point::new(center.x + half, center.y + half),
            );
            append_rect_slab_to_3d_batch(
                batch,
                rect,
                style.base_z,
                style.top_z,
                style.color,
                include_sides,
            )
        }
        ShapeKindView::Label { .. }
        | ShapeKindView::Measurement { .. }
        | ShapeKindView::Polygon(_) => 0,
    }
}

fn append_rect_slab_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    rect: Rect,
    base_z: f32,
    top_z: f32,
    color: Color32,
    include_sides: bool,
) -> usize {
    if include_sides && append_rect_slab_instance_to_3d_batch(batch, rect, base_z, top_z, color) {
        return 5;
    }

    let points = rect.corners();
    let top = [
        Vec3f::new(points[0].x as f32, points[0].y as f32, top_z),
        Vec3f::new(points[1].x as f32, points[1].y as f32, top_z),
        Vec3f::new(points[2].x as f32, points[2].y as f32, top_z),
        Vec3f::new(points[3].x as f32, points[3].y as f32, top_z),
    ];
    let mut faces = usize::from(append_quad_to_3d_batch(
        batch,
        top,
        Vec3f::new(0.0, 0.0, 1.0),
        color,
    ));

    if include_sides {
        let bottom = [
            Vec3f::new(points[0].x as f32, points[0].y as f32, base_z),
            Vec3f::new(points[1].x as f32, points[1].y as f32, base_z),
            Vec3f::new(points[2].x as f32, points[2].y as f32, base_z),
            Vec3f::new(points[3].x as f32, points[3].y as f32, base_z),
        ];
        let side_color = shade_color(color, 0.62);
        let normals = [
            Vec3f::new(0.0, -1.0, 0.0),
            Vec3f::new(1.0, 0.0, 0.0),
            Vec3f::new(0.0, 1.0, 0.0),
            Vec3f::new(-1.0, 0.0, 0.0),
        ];
        for index in 0..points.len() {
            let next = (index + 1) % points.len();
            let side = [bottom[index], bottom[next], top[next], top[index]];
            faces += usize::from(append_quad_to_3d_batch(
                batch,
                side,
                normals[index],
                side_color,
            ));
        }
    }
    faces
}

fn append_rect_slab_instance_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    rect: Rect,
    base_z: f32,
    top_z: f32,
    color: Color32,
) -> bool {
    if batch.rect_slabs.len() >= u32::MAX as usize {
        return false;
    }
    batch.rect_slabs.push(renderer::GpuRectSlabInstance {
        rect: [
            rect.min.x as f32,
            rect.min.y as f32,
            rect.max.x as f32,
            rect.max.y as f32,
        ],
        z_range: [base_z, top_z],
        color: color32_to_gpu(color),
    });
    true
}

fn append_slab_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    points: &[Point],
    base_z: f32,
    top_z: f32,
    color: Color32,
    include_sides: bool,
) -> usize {
    if points.len() < 3 {
        return 0;
    }
    let mut top = Vec::with_capacity(points.len());
    if polygon_signed_area_twice(points) >= 0 {
        top.extend(
            points
                .iter()
                .map(|point| Vec3f::new(point.x as f32, point.y as f32, top_z)),
        );
    } else {
        top.extend(
            points
                .iter()
                .rev()
                .map(|point| Vec3f::new(point.x as f32, point.y as f32, top_z)),
        );
    }
    let mut faces = usize::from(append_top_face_points_to_3d_batch(batch, &top, color));

    if include_sides {
        let bottom = top
            .iter()
            .map(|point| Vec3f::new(point.x, point.y, base_z))
            .collect::<Vec<_>>();
        let side_color = shade_color(color, 0.62);
        for index in 0..points.len() {
            let next = (index + 1) % points.len();
            let side = [bottom[index], bottom[next], top[next], top[index]];
            faces += usize::from(append_face_points_to_3d_batch(
                batch,
                FaceSurface3d::Side,
                &side,
                side_color,
            ));
        }
    }
    faces
}

fn polygon_signed_area_twice(points: &[Point]) -> i128 {
    let Some(first) = points.first() else {
        return 0;
    };
    let mut previous = *first;
    let mut area = 0i128;
    for point in &points[1..] {
        area += previous.x as i128 * point.y as i128 - point.x as i128 * previous.y as i128;
        previous = *point;
    }
    area + previous.x as i128 * first.y as i128 - first.x as i128 * previous.y as i128
}

fn append_path_segment_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    a: Point,
    b: Point,
    width: Coord,
    base_z: f32,
    top_z: f32,
    color: Color32,
    include_sides: bool,
) -> usize {
    if let Some(rect) = axis_aligned_path_segment_rect(a, b, width) {
        return append_rect_slab_to_3d_batch(batch, rect, base_z, top_z, color, include_sides);
    }
    let Some(points) = path_segment_polygon_points(a, b, width) else {
        return 0;
    };
    append_slab_to_3d_batch(batch, &points, base_z, top_z, color, include_sides)
}

fn add_slab_faces(
    faces: &mut Vec<Face3d>,
    points: &[Point],
    base_z: f32,
    top_z: f32,
    color: Color32,
) {
    if points.len() < 3 || faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
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
        if faces.len() >= MAX_3D_CPU_FALLBACK_FACES {
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
    let Some(points) = path_segment_polygon_points(a, b, width) else {
        return;
    };
    add_slab_faces(faces, &points, base_z, top_z, color);
}

fn path_segment_polygon_points(a: Point, b: Point, width: Coord) -> Option<[Point; 4]> {
    let dx = (b.x - a.x) as f32;
    let dy = (b.y - a.y) as f32;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        warn!("3D path segment has zero length; skipping segment");
        return None;
    }
    let clamped_width = width.max(1);
    if clamped_width != width {
        warn!(
            width,
            clamped_width, "3D path segment width below one dbu; clamping"
        );
    }
    let half = clamped_width as f32 * 0.5;
    let nx = -dy / length * half;
    let ny = dx / length * half;
    Some([
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
    ])
}

fn axis_aligned_path_segment_rect(a: Point, b: Point, width: Coord) -> Option<Rect> {
    if a == b || (a.x != b.x && a.y != b.y) {
        return None;
    }
    let half = width.max(1) / 2;
    if a.y == b.y {
        Some(Rect::new(
            Point::new(a.x.min(b.x), a.y - half),
            Point::new(a.x.max(b.x), a.y + half),
        ))
    } else {
        Some(Rect::new(
            Point::new(a.x - half, a.y.min(b.y)),
            Point::new(a.x + half, a.y.max(b.y)),
        ))
    }
}

fn push_face(
    faces: &mut Vec<Face3d>,
    surface: FaceSurface3d,
    points: Vec<Vec3f>,
    fill: Color32,
    stroke: Color32,
) {
    if faces.len() < MAX_3D_CPU_FALLBACK_FACES {
        faces.push(Face3d {
            surface,
            order: faces.len(),
            points,
            fill,
            stroke,
        });
    }
}

fn append_face_points_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    surface: FaceSurface3d,
    points: &[Vec3f],
    fill: Color32,
) -> bool {
    if points.len() < 3 || batch.vertices.len() > u32::MAX as usize - points.len() {
        return false;
    }
    let base = batch.vertices.len() as u32;
    let color = color32_to_gpu(fill);
    let normal = face_points_normal(surface, points);
    batch
        .vertices
        .extend(points.iter().map(|point| renderer::GpuVertex3d {
            position: [point.x, point.y, point.z],
            normal: [normal.x, normal.y, normal.z],
            color,
        }));
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
    true
}

fn append_top_face_points_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    points: &[Vec3f],
    fill: Color32,
) -> bool {
    if points.len() < 3 || batch.vertices.len() > u32::MAX as usize - points.len() {
        return false;
    }
    let triangles = triangulate_xy_polygon(points);
    if triangles.is_empty() {
        return false;
    }
    let base = batch.vertices.len() as u32;
    let color = color32_to_gpu(fill);
    batch
        .vertices
        .extend(points.iter().map(|point| renderer::GpuVertex3d {
            position: [point.x, point.y, point.z],
            normal: [0.0, 0.0, 1.0],
            color,
        }));
    for [a, b, c] in triangles {
        batch
            .indices
            .extend_from_slice(&[base + a as u32, base + b as u32, base + c as u32]);
    }
    true
}

fn triangulate_xy_polygon(points: &[Vec3f]) -> Vec<[usize; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }
    if points.len() == 3 {
        return vec![[0, 1, 2]];
    }
    let mut vertices: Vec<usize> = (0..points.len()).collect();
    if polygon_area_xy(points) < 0.0 {
        vertices.reverse();
    }
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    let mut guard = 0usize;
    while vertices.len() > 3 && guard < points.len() * points.len() {
        guard += 1;
        let mut ear_index = None;
        for index in 0..vertices.len() {
            let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
            let current = vertices[index];
            let next = vertices[(index + 1) % vertices.len()];
            if !is_convex_xy(points[previous], points[current], points[next]) {
                continue;
            }
            let contains_vertex = vertices.iter().any(|candidate| {
                *candidate != previous
                    && *candidate != current
                    && *candidate != next
                    && point_in_triangle_xy(
                        points[*candidate],
                        points[previous],
                        points[current],
                        points[next],
                    )
            });
            if !contains_vertex {
                ear_index = Some(index);
                triangles.push([previous, current, next]);
                break;
            }
        }
        let Some(index) = ear_index else {
            triangulate_xy_fan(&vertices, &mut triangles);
            return triangles;
        };
        vertices.remove(index);
    }
    if vertices.len() == 3 {
        triangles.push([vertices[0], vertices[1], vertices[2]]);
    }
    triangles
}

fn triangulate_xy_fan(vertices: &[usize], triangles: &mut Vec<[usize; 3]>) {
    for index in 1..vertices.len().saturating_sub(1) {
        triangles.push([vertices[0], vertices[index], vertices[index + 1]]);
    }
}

fn polygon_area_xy(points: &[Vec3f]) -> f32 {
    let Some(first) = points.first() else {
        return 0.0;
    };
    let mut previous = *first;
    let mut area = 0.0;
    for point in &points[1..] {
        area += previous.x * point.y - point.x * previous.y;
        previous = *point;
    }
    (area + previous.x * first.y - first.x * previous.y) * 0.5
}

fn is_convex_xy(a: Vec3f, b: Vec3f, c: Vec3f) -> bool {
    let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    cross > 0.0
}

fn point_in_triangle_xy(point: Vec3f, a: Vec3f, b: Vec3f, c: Vec3f) -> bool {
    let area = triangle_area_sign_xy(point, a, b);
    let b_area = triangle_area_sign_xy(point, b, c);
    let c_area = triangle_area_sign_xy(point, c, a);
    let has_negative = area < -0.001 || b_area < -0.001 || c_area < -0.001;
    let has_positive = area > 0.001 || b_area > 0.001 || c_area > 0.001;
    !(has_negative && has_positive)
}

fn triangle_area_sign_xy(a: Vec3f, b: Vec3f, c: Vec3f) -> f32 {
    (a.x - c.x) * (b.y - c.y) - (b.x - c.x) * (a.y - c.y)
}

fn append_quad_to_3d_batch(
    batch: &mut renderer::RenderBatch3d,
    points: [Vec3f; 4],
    normal: Vec3f,
    fill: Color32,
) -> bool {
    if batch.vertices.len() > u32::MAX as usize - 4 {
        return false;
    }
    let base = batch.vertices.len() as u32;
    let color = color32_to_gpu(fill);
    batch
        .vertices
        .extend(points.into_iter().map(|point| renderer::GpuVertex3d {
            position: [point.x, point.y, point.z],
            normal: [normal.x, normal.y, normal.z],
            color,
        }));
    batch
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    true
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

fn face_points_normal(surface: FaceSurface3d, points: &[Vec3f]) -> Vec3f {
    let normal = polygon_normal_3d(points);
    if surface == FaceSurface3d::Top && normal.z < 0.0 {
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

fn layer_3d_stack_position(process: ProcessLayer) -> (f32, f32) {
    match process {
        ProcessLayer::Diffusion => (0.0, 80.0),
        ProcessLayer::Oxide => (95.0, 40.0),
        ProcessLayer::Poly => (155.0, 75.0),
        ProcessLayer::Contact => (230.0, 110.0),
        ProcessLayer::Metal1 => (340.0, 90.0),
        ProcessLayer::Via1 => (430.0, 120.0),
        ProcessLayer::Metal2 => (550.0, 90.0),
        ProcessLayer::Annotation => (0.0, 0.0),
    }
}

fn process_layer_label(process: ProcessLayer) -> &'static str {
    match process {
        ProcessLayer::Diffusion => "Diffusion",
        ProcessLayer::Poly => "Poly",
        ProcessLayer::Contact => "Contact",
        ProcessLayer::Metal1 => "Metal 1",
        ProcessLayer::Via1 => "Via 1",
        ProcessLayer::Metal2 => "Metal 2",
        ProcessLayer::Oxide => "Oxide",
        ProcessLayer::Annotation => "Annotation",
    }
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

fn equipment_state_tone(state: EquipmentToolState) -> ui_chrome::Tone {
    match state {
        EquipmentToolState::Offline => ui_chrome::Tone::Neutral,
        EquipmentToolState::OnlineIdle => ui_chrome::Tone::Success,
        EquipmentToolState::RecipeLoaded => ui_chrome::Tone::Info,
        EquipmentToolState::Running => ui_chrome::Tone::Warning,
        EquipmentToolState::Completed => ui_chrome::Tone::Success,
        EquipmentToolState::Alarm => ui_chrome::Tone::Danger,
        EquipmentToolState::Maintenance => ui_chrome::Tone::Warning,
    }
}

fn equipment_state_sort_rank(state: EquipmentToolState) -> u8 {
    match state {
        EquipmentToolState::Alarm => 0,
        EquipmentToolState::Running => 1,
        EquipmentToolState::RecipeLoaded => 2,
        EquipmentToolState::Completed => 3,
        EquipmentToolState::OnlineIdle => 4,
        EquipmentToolState::Maintenance => 5,
        EquipmentToolState::Offline => 6,
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
        let elapsed_s = equipment_run_elapsed_s(run, now_s);
        let duration = tool
            .selected_recipe_details()
            .map(|recipe| format!(" / {} s", recipe.duration_s))
            .unwrap_or_default();
        return format!("{}\n{}{} active", run.recipe.recipe_id, elapsed_s, duration);
    }
    if let Some(selection) = &tool.selected_recipe {
        return format!(
            "{}\nv{} loaded",
            selection.recipe_id, selection.recipe_version
        );
    }
    "No recipe".to_string()
}

fn equipment_run_elapsed_s(run: &layout_model::equipment::ToolRun, now_s: u64) -> u64 {
    run.completed_at_s
        .unwrap_or(now_s)
        .saturating_sub(run.started_at_s)
}

fn equipment_run_progress(tool: &EquipmentTool, now_s: u64) -> Option<(f32, u64, u64)> {
    let run = tool.active_run.as_ref()?;
    let duration_s = tool
        .selected_recipe_details()
        .map(|recipe| recipe.duration_s)
        .unwrap_or(1)
        .max(1);
    let elapsed_s = equipment_run_elapsed_s(run, now_s);
    Some((
        (elapsed_s as f32 / duration_s as f32).clamp(0.0, 1.0),
        elapsed_s,
        duration_s,
    ))
}

fn equipment_active_alarm_count(tool: &EquipmentTool) -> usize {
    tool.active_alarms
        .iter()
        .filter(|alarm| alarm.active)
        .count()
}

fn equipment_tool_alarm_summary(tool: &EquipmentTool) -> String {
    let active = tool
        .active_alarms
        .iter()
        .filter(|alarm| alarm.active)
        .collect::<Vec<_>>();
    if active.is_empty() {
        return "Clear".to_string();
    }
    let critical = active
        .iter()
        .filter(|alarm| alarm.severity == AlarmSeverity::Critical)
        .count();
    if critical > 0 {
        format!("{} active, {} critical", active.len(), critical)
    } else {
        format!("{} active", active.len())
    }
}

fn equipment_selection_context(selection: &RecipeSelection) -> String {
    let lot = selection.lot_id.as_deref().unwrap_or("no lot");
    let wafer = selection.wafer_id.as_deref().unwrap_or("no wafer");
    let step = selection
        .process_step_id
        .as_deref()
        .unwrap_or("no process step");
    let operator = selection.operator.as_deref().unwrap_or("no operator");
    format!("{lot} | {wafer} | {step} | {operator}")
}

fn equipment_command_hint(tool: &EquipmentTool) -> &'static str {
    match tool.state {
        EquipmentToolState::Offline => "Bring the tool online before loading a recipe.",
        EquipmentToolState::OnlineIdle => "Select a recipe, load it, then start the run.",
        EquipmentToolState::RecipeLoaded => {
            "Start executes the loaded recipe; reset clears the context."
        }
        EquipmentToolState::Running => {
            "Stop aborts the active run; alarms simulate host interlocks."
        }
        EquipmentToolState::Completed => "Load the next recipe or reset the completed context.",
        EquipmentToolState::Alarm => "Clear alarms before loading or starting another process.",
        EquipmentToolState::Maintenance => {
            "Exit maintenance returns the tool to offline host control."
        }
    }
}

fn equipment_recent_sensor_summary(tool: &EquipmentTool) -> String {
    let names = equipment_sensor_names(tool);
    if names.is_empty() {
        return "No samples".to_string();
    }
    let summary = names
        .into_iter()
        .take(2)
        .filter_map(|name| {
            tool.latest_sensor(&name)
                .map(|sample| format!("{name}: {}", equipment_sensor_value(sample)))
        })
        .collect::<Vec<_>>()
        .join("\n");
    if summary.is_empty() {
        "No samples".to_string()
    } else {
        summary
    }
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

fn equipment_sensor_age_text(sample: &SensorSample, now_s: u64) -> String {
    let age_s = now_s.saturating_sub(sample.at_s);
    if age_s == 0 {
        "live".to_string()
    } else {
        format!("{age_s} s ago")
    }
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

fn defect_local_position(map: &WaferMap, position_mm: [f64; 2], die: DieCoord) -> [f32; 2] {
    let center = map.geometry.die_center_mm(die);
    let x = ((position_mm[0] - center[0]) / map.geometry.die_size_mm[0] + 0.5).clamp(0.0, 1.0);
    let y = (0.5 - (position_mm[1] - center[1]) / map.geometry.die_size_mm[1]).clamp(0.0, 1.0);
    [x as f32, y as f32]
}

#[derive(Clone, Copy, Debug)]
struct MetrologyTriageSite {
    die: DieCoord,
    fail_count: usize,
    outlier_count: usize,
    defect_count: usize,
    review_count: usize,
    score: usize,
}

fn metrology_panel_plot_size(ui: &egui::Ui, min_width: f32, max_width: f32, height: f32) -> Vec2 {
    let available = ui.available_width().max(1.0);
    let width = if available < min_width {
        available
    } else {
        available.min(max_width)
    };
    vec2(width, height)
}

fn metrology_spec_window_label(kind: MeasurementKind) -> String {
    let spec = kind.spec();
    let unit = kind.unit();
    let suffix = if unit.is_empty() { "" } else { unit };
    match (spec.lower, spec.target, spec.upper) {
        (Some(lower), Some(target), Some(upper)) => {
            format!(
                "{} / {} / {} {}",
                format_metrology_spec_bound(kind, lower),
                format_metrology_spec_bound(kind, target),
                format_metrology_spec_bound(kind, upper),
                suffix
            )
        }
        (Some(lower), _, Some(upper)) => {
            format!(
                "{}..{} {}",
                format_metrology_spec_bound(kind, lower),
                format_metrology_spec_bound(kind, upper),
                suffix
            )
        }
        (Some(lower), _, None) => {
            format!(">= {} {}", format_metrology_spec_bound(kind, lower), suffix)
        }
        (None, _, Some(upper)) => {
            format!("<= {} {}", format_metrology_spec_bound(kind, upper), suffix)
        }
        (None, Some(target), None) => {
            format!(
                "target {} {}",
                format_metrology_spec_bound(kind, target),
                suffix
            )
        }
        (None, None, None) => "no spec".to_string(),
    }
}

fn format_metrology_spec_bound(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::DefectCount => format!("{value:.0}"),
        MeasurementKind::PassFail => {
            if value >= 0.5 {
                "pass".to_string()
            } else {
                "fail".to_string()
            }
        }
        _ if value.abs() >= 100.0 => format!("{value:.0}"),
        _ => format!("{value:.2}"),
    }
}

fn format_metrology_target_delta(kind: MeasurementKind, value: f64) -> Option<String> {
    let target = kind.spec().target?;
    Some(format_metrology_signed_delta(kind, value - target))
}

fn format_metrology_signed_delta(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::DefectCount => format!("{value:+.0}"),
        MeasurementKind::PassFail => format!("{value:+.2}"),
        _ => format!("{value:+.2} {}", kind.unit()),
    }
}

fn metrology_compact_record_id(id: &str) -> String {
    const MAX_LEN: usize = 22;
    if id.len() <= MAX_LEN {
        id.to_string()
    } else {
        format!("...{}", &id[id.len() - (MAX_LEN - 3)..])
    }
}

fn metrology_capability_label(cpk: f64) -> String {
    if cpk >= 1.67 {
        "high capability".to_string()
    } else if cpk >= 1.33 {
        "capable".to_string()
    } else if cpk >= 1.0 {
        "watch".to_string()
    } else {
        "below target".to_string()
    }
}

fn metrology_capability_tone(cpk: Option<f64>) -> ui_chrome::Tone {
    match cpk {
        Some(value) if value >= 1.33 => ui_chrome::Tone::Success,
        Some(value) if value >= 1.0 => ui_chrome::Tone::Warning,
        Some(_) => ui_chrome::Tone::Danger,
        None => ui_chrome::Tone::Neutral,
    }
}

fn metrology_triage_for_die(map: &WaferMap, die: DieCoord) -> MetrologyTriageSite {
    let mut fail_count = 0;
    let mut outlier_count = 0;
    for kind in MeasurementKind::NUMERIC {
        if let Some(measurement) = map.measurement_for(die, kind) {
            match measurement.status {
                MeasurementStatus::Pass => {}
                MeasurementStatus::Fail => fail_count += 1,
                MeasurementStatus::Outlier => outlier_count += 1,
            }
        }
    }

    let mut defect_count = 0;
    let mut max_severity = 0;
    for defect in map.defects_for_die(die) {
        defect_count += 1;
        max_severity = max_severity.max(defect.severity as usize);
    }

    let review_count = map
        .annotations_for_die(die)
        .filter(|annotation| {
            matches!(
                annotation.kind,
                layout_model::metrology::AnnotationKind::Review
                    | layout_model::metrology::AnnotationKind::ProcessExcursion
            )
        })
        .count();
    let score = fail_count * 80
        + outlier_count * 45
        + max_severity * 12
        + defect_count * 8
        + review_count * 10;

    MetrologyTriageSite {
        die,
        fail_count,
        outlier_count,
        defect_count,
        review_count,
        score,
    }
}

fn metrology_triage_sites(map: &WaferMap) -> Vec<MetrologyTriageSite> {
    let mut sites = map
        .dies
        .iter()
        .copied()
        .map(|die| metrology_triage_for_die(map, die))
        .filter(|site| site.score > 0)
        .collect::<Vec<_>>();
    sites.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.fail_count.cmp(&a.fail_count))
            .then_with(|| b.outlier_count.cmp(&a.outlier_count))
            .then_with(|| b.defect_count.cmp(&a.defect_count))
            .then_with(|| a.die.cmp(&b.die))
    });
    sites
}

fn metrology_next_triage_die(
    sites: &[MetrologyTriageSite],
    selected: Option<DieCoord>,
) -> Option<DieCoord> {
    if sites.is_empty() {
        return None;
    }
    let Some(selected) = selected else {
        return sites.first().map(|site| site.die);
    };
    let next_index = sites
        .iter()
        .position(|site| site.die == selected)
        .map(|index| (index + 1) % sites.len())
        .unwrap_or(0);
    sites.get(next_index).map(|site| site.die)
}

fn metrology_triage_label(score: usize) -> &'static str {
    match score {
        0 => "No action",
        1..=39 => "Low",
        40..=99 => "Review",
        100..=159 => "High",
        _ => "Critical",
    }
}

fn metrology_triage_tone(score: usize) -> ui_chrome::Tone {
    match score {
        0 => ui_chrome::Tone::Success,
        1..=39 => ui_chrome::Tone::Info,
        40..=99 => ui_chrome::Tone::Warning,
        _ => ui_chrome::Tone::Danger,
    }
}

fn metrology_defect_class_counts(map: &WaferMap) -> [(DefectClass, usize); 5] {
    let mut counts = [
        (DefectClass::Particle, 0),
        (DefectClass::Scratch, 0),
        (DefectClass::PatternBridge, 0),
        (DefectClass::MissingFeature, 0),
        (DefectClass::Unknown, 0),
    ];
    for defect in &map.defects {
        let index = match defect.class {
            DefectClass::Particle => 0,
            DefectClass::Scratch => 1,
            DefectClass::PatternBridge => 2,
            DefectClass::MissingFeature => 3,
            DefectClass::Unknown => 4,
        };
        counts[index].1 += 1;
    }
    counts
}

fn metrology_capability_index(summary: layout_model::metrology::MeasurementSummary) -> Option<f64> {
    let spec = summary.kind.spec();
    let mean = summary.mean?;
    let stddev = summary.stddev?;
    if stddev <= f64::EPSILON {
        return None;
    }
    match (spec.lower, spec.upper) {
        (Some(lower), Some(upper)) => {
            Some(((upper - mean) / (3.0 * stddev)).min((mean - lower) / (3.0 * stddev)))
        }
        (Some(lower), None) => Some((mean - lower) / (3.0 * stddev)),
        (None, Some(upper)) => Some((upper - mean) / (3.0 * stddev)),
        (None, None) => None,
    }
}

fn metrology_radial_profile(
    map: &WaferMap,
    kind: MeasurementKind,
    bin_count: usize,
) -> Vec<(f32, Option<f64>)> {
    if bin_count == 0 {
        return Vec::new();
    }
    let active_radius = map.geometry.active_radius_mm().max(1.0);
    let mut sums = vec![0.0; bin_count];
    let mut counts = vec![0_usize; bin_count];
    for measurement in map
        .measurements
        .iter()
        .filter(|measurement| measurement.kind == kind)
    {
        let [x, y] = map.geometry.die_center_mm(measurement.die);
        let radial = (x.hypot(y) / active_radius).clamp(0.0, 1.0);
        let index = ((radial * bin_count as f64).floor() as usize).min(bin_count - 1);
        sums[index] += measurement.value;
        counts[index] += 1;
    }
    (0..bin_count)
        .map(|index| {
            let radius = (index as f32 + 0.5) / bin_count as f32;
            let value = (counts[index] > 0).then(|| sums[index] / counts[index] as f64);
            (radius, value)
        })
        .collect()
}

fn metrology_site_needs_attention(map: &WaferMap, die: DieCoord, kind: MeasurementKind) -> bool {
    let measurement_attention = map
        .measurement_for(die, kind)
        .is_some_and(|measurement| measurement.status != MeasurementStatus::Pass);
    measurement_attention || map.defects_for_die(die).next().is_some()
}

fn metrology_overlay_vector(map: &WaferMap, die: DieCoord) -> Option<[f32; 2]> {
    let cd = map
        .measurement_for(die, MeasurementKind::CriticalDimensionNm)?
        .value;
    let sheet_r = map
        .measurement_for(die, MeasurementKind::SheetResistanceOhmsPerSq)?
        .value;
    let cd_target = MeasurementKind::CriticalDimensionNm.spec().target?;
    let sheet_target = MeasurementKind::SheetResistanceOhmsPerSq.spec().target?;
    let x = ((cd - cd_target) as f32 * 3.0).clamp(-14.0, 14.0);
    let y = ((sheet_r - sheet_target) as f32 * 3.6).clamp(-14.0, 14.0);
    Some([x, y])
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

fn metrology_deviation_color(measurement: &Measurement) -> Color32 {
    if measurement.kind == MeasurementKind::PassFail {
        return metrology_measurement_color(
            measurement,
            layout_model::metrology::MeasurementSummary {
                kind: measurement.kind,
                sample_count: 1,
                pass_count: usize::from(measurement.status == MeasurementStatus::Pass),
                fail_count: usize::from(measurement.status == MeasurementStatus::Fail),
                outlier_count: usize::from(measurement.status == MeasurementStatus::Outlier),
                min: Some(0.0),
                max: Some(1.0),
                mean: Some(measurement.value),
                stddev: None,
            },
        );
    }
    let spec = measurement.kind.spec();
    let target = spec.target.unwrap_or(measurement.value);
    let lower_span = spec.lower.map_or(1.0, |lower| (target - lower).abs());
    let upper_span = spec.upper.map_or(1.0, |upper| (upper - target).abs());
    let span = if measurement.value < target {
        lower_span
    } else {
        upper_span
    }
    .max(0.000_001);
    let normalized = ((measurement.value - target) / span).clamp(-1.0, 1.0) as f32;
    if normalized < 0.0 {
        lerp_color(
            Color32::from_rgb(74, 138, 214),
            Color32::from_rgb(78, 176, 118),
            1.0 + normalized,
        )
    } else {
        lerp_color(
            Color32::from_rgb(78, 176, 118),
            Color32::from_rgb(224, 80, 75),
            normalized,
        )
    }
}

fn metrology_spec_window_color(measurement: &Measurement) -> Color32 {
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

    let spec = measurement.kind.spec();
    let target = spec.target.unwrap_or(measurement.value);
    let span = if measurement.value < target {
        spec.lower.map(|lower| (target - lower).abs())
    } else {
        spec.upper.map(|upper| (upper - target).abs())
    }
    .unwrap_or(0.0);
    if span <= f64::EPSILON {
        return metrology_spec_window_color_for_ratio(0.0);
    }

    let ratio = ((measurement.value - target).abs() / span).clamp(0.0, 1.0) as f32;
    metrology_spec_window_color_for_ratio(ratio)
}

fn metrology_spec_window_color_for_ratio(ratio: f32) -> Color32 {
    let ratio = ratio.clamp(0.0, 1.0);
    if ratio < 0.65 {
        lerp_color(
            Color32::from_rgb(68, 160, 116),
            Color32::from_rgb(218, 190, 84),
            ratio / 0.65,
        )
    } else {
        lerp_color(
            Color32::from_rgb(218, 190, 84),
            Color32::from_rgb(224, 80, 75),
            (ratio - 0.65) / 0.35,
        )
    }
}

fn metrology_review_queue_color(score: usize) -> Color32 {
    match score {
        0 => Color32::from_rgb(50, 84, 74),
        1..=39 => Color32::from_rgb(74, 112, 142),
        40..=99 => Color32::from_rgb(196, 150, 68),
        100..=159 => Color32::from_rgb(214, 104, 68),
        _ => Color32::from_rgb(226, 72, 78),
    }
}

fn metrology_defect_density_color(count: usize) -> Color32 {
    match count {
        0 => Color32::from_rgb(54, 82, 76),
        1 => Color32::from_rgb(132, 128, 72),
        2..=3 => Color32::from_rgb(196, 122, 62),
        _ => Color32::from_rgb(224, 80, 75),
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

fn defect_class_label(class: DefectClass) -> &'static str {
    match class {
        DefectClass::Particle => "particle",
        DefectClass::Scratch => "scratch",
        DefectClass::PatternBridge => "pattern bridge",
        DefectClass::MissingFeature => "missing feature",
        DefectClass::Unknown => "unknown",
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
                (
                    "Design",
                    vec!["Mask layout", "3D viewport", "Reticle prep", "Layout diff"]
                ),
                (
                    "Operations",
                    vec![
                        "Workflow",
                        "Equipment",
                        "Inventory",
                        "Maintenance",
                        "Environment",
                        "Dispatch",
                        "Safety",
                        "Traceability",
                    ]
                ),
                ("Analysis", vec!["Metrology", "Yield", "SPC / FDC"]),
                (
                    "Engineering",
                    vec![
                        "Process flow",
                        "R2R control",
                        "Cross-section",
                        "DOE",
                        "Notebook"
                    ]
                ),
            ]
        );
    }

    #[test]
    fn side_panels_follow_view_context() {
        assert!(ViewMode::Layout2d.has_inspector_panel());
        assert!(ViewMode::Layout2d.has_secondary_panel());
        assert!(ViewMode::Layout2d.supports_viewport_fullscreen());
        assert!(ViewMode::Layout3d.has_inspector_panel());
        assert!(ViewMode::Layout3d.has_secondary_panel());
        assert!(ViewMode::Layout3d.supports_viewport_fullscreen());
        assert!(ViewMode::Workflow.has_inspector_panel());
        assert!(!ViewMode::Workflow.has_secondary_panel());
        assert!(!ViewMode::Workflow.supports_viewport_fullscreen());

        assert!(ViewMode::MaskPrep.has_inspector_panel());
        assert!(ViewMode::MaskPrep.has_secondary_panel());
        assert!(!ViewMode::MaskPrep.supports_viewport_fullscreen());
        assert!(ViewMode::LayoutDiff.has_inspector_panel());
        assert!(!ViewMode::LayoutDiff.has_secondary_panel());
        assert!(ViewMode::Metrology.has_inspector_panel());
        assert!(ViewMode::Metrology.has_secondary_panel());
        assert!(ViewMode::CrossSection.has_inspector_panel());
        assert!(ViewMode::CrossSection.has_secondary_panel());
        assert!(ViewMode::Experiment.has_inspector_panel());
        assert!(ViewMode::Experiment.has_secondary_panel());
        assert!(ViewMode::Notebook.has_inspector_panel());
        assert!(ViewMode::Notebook.has_secondary_panel());

        assert!(!ViewMode::Yield.has_inspector_panel());
        assert!(!ViewMode::Yield.has_secondary_panel());
        assert!(!ViewMode::FabControl.has_inspector_panel());
        assert!(!ViewMode::FabControl.has_secondary_panel());

        assert!(ViewMode::SpcFdc.has_inspector_panel());
        assert!(!ViewMode::SpcFdc.has_secondary_panel());
        assert!(ViewMode::Inventory.has_inspector_panel());
        assert!(!ViewMode::Inventory.has_secondary_panel());
        assert!(ViewMode::Maintenance.has_inspector_panel());
        assert!(!ViewMode::Maintenance.has_secondary_panel());
        assert!(ViewMode::Environment.has_inspector_panel());
        assert!(!ViewMode::Environment.has_secondary_panel());
        assert!(ViewMode::Scheduler.has_inspector_panel());
        assert!(!ViewMode::Scheduler.has_secondary_panel());
        assert!(ViewMode::Safety.has_inspector_panel());
        assert!(!ViewMode::Safety.has_secondary_panel());
        assert!(ViewMode::ProcessFlow.has_inspector_panel());
        assert!(!ViewMode::ProcessFlow.has_secondary_panel());
        assert!(ViewMode::ProcessControl.has_inspector_panel());
        assert!(!ViewMode::ProcessControl.has_secondary_panel());
        assert!(ViewMode::Traceability.has_inspector_panel());
        assert!(!ViewMode::Traceability.has_secondary_panel());
    }

    #[test]
    fn blank_workspace_dataset_has_no_demo_operational_data() {
        let dataset = WorkspaceDataset::blank();

        assert!(dataset.document.layers.is_empty());
        assert!(dataset.document.shapes.is_empty());
        assert!(dataset.mes.lots.is_empty());
        assert!(dataset.yield_analysis.lots.is_empty());
        assert!(dataset.wafer_map.dies.is_empty());
        assert!(dataset.recipe_catalog.recipes.is_empty());
        assert_eq!(dataset.genealogy.summary().lot_count, 0);
        assert!(dataset.inventory.lots.is_empty());
        assert!(dataset.maintenance.tools.is_empty());
        assert!(dataset.environment.sensors.is_empty());
        assert!(dataset.scheduler.is_empty());
        assert!(dataset.safety.sensors.is_empty());
        assert!(dataset.experiment_plan.runs.is_empty());
        assert!(dataset.process_control.loops.is_empty());
        assert!(dataset.process_flow.route.nodes.is_empty());
        assert!(dataset.lab_notebook.entries.is_empty());
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
        assert!(!dataset.inventory.lots.is_empty());
        assert!(!dataset.maintenance.tools.is_empty());
        assert!(!dataset.environment.sensors.is_empty());
        assert!(!dataset.scheduler.tools.is_empty());
        assert!(!dataset.scheduler.lots.is_empty());
        assert!(!dataset.safety.sensors.is_empty());
        assert!(dataset.safety.summary().locked_out_tool_count > 0);
        assert!(!dataset.experiment_plan.runs.is_empty());
        assert!(!dataset.process_control.loops.is_empty());
        assert!(!dataset.process_flow.route.nodes.is_empty());
        assert!(!dataset.lab_notebook.entries.is_empty());
        assert!(dataset.equipment.tools().count() > 0);
    }

    #[test]
    fn demo_workspace_links_focus_lot_across_workflow() {
        let dataset = WorkspaceDataset::demo();
        let focus_lot = "L-00042";

        assert!(dataset.mes.lots.contains_key(&LotId::new(focus_lot)));
        assert_eq!(
            dataset.process_flow.route.mes_route_id,
            "ROUTE-DEMO-INVERTER-POLY-A"
        );
        for binding in dataset
            .process_flow
            .route
            .nodes
            .iter()
            .filter_map(|node| node.recipe.as_ref())
        {
            assert!(
                dataset
                    .recipe_catalog
                    .recipe(&layout_model::recipe::RecipeId::from(
                        binding.recipe_id.as_str()
                    ))
                    .is_some(),
                "missing recipe {}",
                binding.recipe_id
            );
        }
        assert!(
            dataset
                .scheduler
                .lots
                .iter()
                .any(|lot| lot.id.as_str() == focus_lot)
        );
        assert!(dataset.inventory.lots.values().any(|material| {
            material.usage.iter().any(|usage| {
                usage.links.iter().any(|link| {
                    matches!(
                        link,
                        layout_model::inventory::FabObjectLink::Lot { lot_id }
                            if lot_id == focus_lot
                    )
                })
            })
        }));
        assert!(dataset.yield_analysis.lot_summary(focus_lot).is_some());
        assert!(
            dataset
                .lab_notebook
                .entries
                .iter()
                .any(|entry| { entry.links.lots.iter().any(|lot| lot.as_str() == focus_lot) })
        );
    }

    #[test]
    fn app_context_lot_selection_updates_linked_views() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new(&cc);
        app.apply_workspace_dataset(
            WorkspaceDataset::demo(),
            DataSource::Demo,
            DataSource::Demo,
            "demo",
        );

        app.set_focus_object(FabObjectRef::lot("L-00042"));

        assert_eq!(app.app_context.focus_lot(), Some("L-00042"));
        assert_eq!(app.workflow_panel.focus_lot(), "L-00042");
        assert_eq!(
            app.selected_mes_lot.as_ref().map(LotId::as_str),
            Some("L-00042")
        );
        assert_eq!(app.selected_yield_lot, "L-00042");
        assert!(
            app.yield_analysis
                .wafer_ids_for_lot("L-00042")
                .contains(&app.selected_yield_wafer)
        );
    }

    #[test]
    fn context_mes_action_advances_focus_lot_traveler() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new(&cc);
        app.apply_workspace_dataset(
            WorkspaceDataset::demo(),
            DataSource::Demo,
            DataSource::Demo,
            "demo",
        );
        app.set_focus_object(FabObjectRef::lot("L-00042"));

        let (lot_id, action, label) = app.context_mes_action().expect("focus lot has action");
        assert!(label.starts_with("Start "));
        assert!(matches!(action, OperatorAction::StartStep { .. }));
        app.apply_mes_action(&lot_id, action);

        let traveler = app.mes.travelers.get(&lot_id).expect("traveler exists");
        assert_eq!(traveler.status, TravelerStatus::Running);
    }

    #[test]
    fn quick_workflow_note_links_to_focus_lot() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new(&cc);
        app.apply_workspace_dataset(
            WorkspaceDataset::demo(),
            DataSource::Demo,
            DataSource::Demo,
            "demo",
        );
        let before = app.notebook_panel.notebook().entries.len();
        let entry_id = app.notebook_panel.add_quick_lot_note("L-00042");

        assert_eq!(app.notebook_panel.notebook().entries.len(), before + 1);
        let entry = app
            .notebook_panel
            .notebook()
            .entry(&entry_id)
            .expect("created note is present");
        assert!(entry.links.lots.iter().any(|lot| lot.as_str() == "L-00042"));
    }

    #[test]
    fn large_documents_skip_synchronous_drc() {
        let small = Document::stress(1_000);
        assert!(drc_skip_message(&small).is_none());

        let large = Document::stress(MAX_DRC_OBJECTS + 1);
        assert!(drc_skip_message(&large).is_some());
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
    fn fps_label_formats_frame_time() {
        assert_eq!(
            format_3d_fps_label(Some(66.666_666), Some(8.333_333)),
            "FPS: 15.0"
        );
        assert_eq!(format_3d_fps_label(Some(16.666_666), None), "FPS: 60.0");
        assert_eq!(format_3d_fps_label(None, Some(8.333_333)), "FPS: --");
        assert_eq!(format_3d_fps_label(Some(0.0), None), "FPS: --");
        assert_eq!(format_3d_fps_label(Some(f64::NAN), None), "FPS: --");
        assert_eq!(format_3d_fps_label(None, None), "FPS: --");
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
    fn rect_slab_side_faces_ignore_edge_on_axis_faces() {
        assert_eq!(
            rect_slab_side_faces_for_forward(Vec3f::new(1.0, 0.0, 0.0)),
            [4, 4]
        );
        assert_eq!(
            rect_slab_side_faces_for_forward(Vec3f::new(0.0, 1.0, 0.0)),
            [1, 1]
        );
        assert_eq!(
            rect_slab_side_faces_for_forward(Vec3f::new(0.7, 0.7, 0.0).normalized()),
            [4, 1]
        );
        assert_eq!(
            rect_slab_side_faces_for_forward(Vec3f::new(-0.7, -0.7, 0.0).normalized()),
            [2, 3]
        );
    }

    #[test]
    fn fly_camera_frustum_xy_rect_tracks_look_target() {
        let camera = Camera3d::look_at(
            Vec3f::new(-1_000.0, -1_000.0, 1_000.0),
            Vec3f::ZERO,
            2_000.0,
        );
        let canvas = EguiRect::from_min_size(Pos2::ZERO, vec2(1_280.0, 720.0));
        let rect =
            camera_frustum_xy_rect(camera, canvas, CAMERA_NEAR_PLANE, 10_000.0, 0.0, 2_000.0)
                .expect("camera frustum should intersect scene z range");

        assert!(rect.contains_point(Point::ZERO));
    }

    #[test]
    fn fly_camera_frustum_xy_rect_includes_horizontal_slab_views() {
        let camera = Camera3d {
            position: Vec3f::new(0.0, 0.0, 1_000.0),
            yaw: 0.0,
            pitch: 0.0,
            speed: 2_000.0,
            fov_y: 58.0_f32.to_radians(),
        };
        let canvas = EguiRect::from_min_size(Pos2::ZERO, vec2(1_280.0, 720.0));
        let rect =
            camera_frustum_xy_rect(camera, canvas, CAMERA_NEAR_PLANE, 10_000.0, 0.0, 2_000.0)
                .expect("horizontal frustum should intersect scene z range");

        assert!(rect.contains_point(Point::new(10_000, 0)));
    }

    #[test]
    fn camera_3d_far_plane_scales_with_large_layouts() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new(&cc);
        app.layout_bounds_cache = Some(Rect::from_min_size(
            Point::new(-500_000, -500_000),
            1_000_000,
            1_000_000,
        ));
        app.camera_3d = Camera3d::look_at(
            Vec3f::new(-850_000.0, -950_000.0, 620_000.0),
            Vec3f::new(0.0, 0.0, 360.0),
            850_000.0,
        );

        assert!(app.camera_3d_far_plane() >= 4_000_000.0);
    }

    #[test]
    fn camera_3d_far_plane_has_large_default_without_layout_bounds() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let app = FabricadApp::new(&cc);

        assert_eq!(app.camera_3d_far_plane(), CAMERA_FAR_PLANE_MIN);
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
    fn axis_aligned_path_segments_use_rect_slab_batch() {
        let mut batch = renderer::RenderBatch3d::default();
        let faces = append_path_segment_to_3d_batch(
            &mut batch,
            Point::new(0, 0),
            Point::new(100, 0),
            20,
            0.0,
            10.0,
            Color32::from_rgb(40, 120, 220),
            true,
        );

        assert_eq!(faces, 5);
        assert_eq!(batch.rect_slabs.len(), 1);
        assert!(batch.vertices.is_empty());
        assert!(batch.indices.is_empty());
        assert_eq!(batch.rect_slabs[0].rect, [0.0, -10.0, 100.0, 10.0]);
    }

    #[test]
    fn diagonal_path_segments_keep_mesh_batch() {
        let mut batch = renderer::RenderBatch3d::default();
        let faces = append_path_segment_to_3d_batch(
            &mut batch,
            Point::new(0, 0),
            Point::new(100, 100),
            20,
            0.0,
            10.0,
            Color32::from_rgb(40, 120, 220),
            true,
        );

        assert_eq!(faces, 5);
        assert!(batch.rect_slabs.is_empty());
        assert_eq!(batch.vertices.len(), 20);
        assert_eq!(batch.indices.len(), 30);
    }

    #[test]
    fn rect_3d_batch_can_emit_top_only_or_full_slab() {
        let rect = Rect::from_min_size(Point::new(0, 0), 100, 50);
        let color = Color32::from_rgb(64, 128, 255);
        let mut top_only = renderer::RenderBatch3d::default();
        let top_faces = append_rect_slab_to_3d_batch(&mut top_only, rect, 0.0, 20.0, color, false);

        assert_eq!(top_faces, 1);
        assert_eq!(top_only.vertices.len(), 4);
        assert_eq!(top_only.indices.len(), 6);

        let mut full_slab = renderer::RenderBatch3d::default();
        let slab_faces = append_rect_slab_to_3d_batch(&mut full_slab, rect, 0.0, 20.0, color, true);

        assert_eq!(slab_faces, 5);
        assert_eq!(full_slab.rect_slabs.len(), 1);
        assert!(full_slab.vertices.is_empty());
        assert!(full_slab.indices.is_empty());
    }

    #[test]
    fn rect_3d_top_cap_winding_matches_normals_for_culling() {
        let rect = Rect::from_min_size(Point::new(0, 0), 100, 50);
        let mut batch = renderer::RenderBatch3d::default();
        append_rect_slab_to_3d_batch(&mut batch, rect, 0.0, 20.0, Color32::WHITE, false);

        assert_triangle_winding_matches_vertex_normals(&batch);
    }

    #[test]
    fn slab_3d_batch_rewinds_clockwise_input_for_culling() {
        let points = [
            Point::new(0, 10),
            Point::new(100, 10),
            Point::new(100, -10),
            Point::new(0, -10),
        ];
        assert!(polygon_signed_area_twice(&points) < 0);

        let mut batch = renderer::RenderBatch3d::default();
        let faces = append_slab_to_3d_batch(&mut batch, &points, 0.0, 20.0, Color32::WHITE, true);

        assert_eq!(faces, 5);
        assert_triangle_winding_matches_vertex_normals(&batch);
        assert!(batch.vertices[0].normal[2] > 0.99);
    }

    #[test]
    fn concave_3d_top_caps_use_ear_clipping() {
        let points = [
            Point::new(0, 0),
            Point::new(100, 0),
            Point::new(100, 40),
            Point::new(40, 40),
            Point::new(40, 100),
            Point::new(0, 100),
        ];
        let mut batch = renderer::RenderBatch3d::default();
        let faces = append_slab_to_3d_batch(&mut batch, &points, 0.0, 20.0, Color32::WHITE, true);

        assert_eq!(faces, 7);
        let top_points = batch.vertices[..6]
            .iter()
            .map(|vertex| Vec3f::new(vertex.position[0], vertex.position[1], vertex.position[2]))
            .collect::<Vec<_>>();
        assert_eq!(triangulate_xy_polygon(&top_points).len(), 4);
        assert_eq!(batch.indices.len(), 48);
        assert_triangle_winding_matches_vertex_normals(&batch);
    }

    #[test]
    fn default_3d_shape_budget_covers_million_shape_stress_scene() {
        assert!(MAX_3D_RENDERED_SHAPES >= 1_000_000);
    }

    #[test]
    fn ten_k_3d_stress_scene_reuses_cached_gpu_mesh() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new_with_options(
            &cc,
            StartupOptions {
                stress_count: Some(10_000),
                view_3d: true,
                ..Default::default()
            },
        );

        let first = app.cached_3d_scene();
        assert_eq!(first.stats.shapes, 10_000);
        assert_eq!(first.stats.faces, 50_000);
        assert!(!first.stats.capped);
        assert!(!first.stats.top_caps_only);
        assert_eq!(first.batch.rect_slabs.len(), 10_000);
        assert!(first.batch.vertices.is_empty());
        assert!(first.batch.indices.is_empty());

        let second = app.cached_3d_scene();
        assert!(Arc::ptr_eq(&first.batch, &second.batch));
        assert_eq!(first.fingerprint, second.fingerprint);
    }

    #[test]
    fn large_3d_stress_scene_uses_full_gpu_slabs() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = FabricadApp::new_with_options(
            &cc,
            StartupOptions {
                stress_count: Some(60_000),
                view_3d: true,
                ..Default::default()
            },
        );

        let scene = app.cached_3d_scene();
        assert_eq!(scene.stats.shapes, 60_000);
        assert_eq!(scene.stats.faces, 300_000);
        assert!(!scene.stats.capped);
        assert!(!scene.stats.top_caps_only);
        assert_eq!(scene.batch.rect_slabs.len(), 60_000);
        assert!(scene.batch.vertices.is_empty());
        assert!(scene.batch.indices.is_empty());
    }

    #[test]
    fn face_3d_gpu_batch_keeps_world_depth_coordinates() {
        let points = [
            Vec3f::new(0.0, 0.0, 20.0),
            Vec3f::new(100.0, 0.0, 20.0),
            Vec3f::new(100.0, 100.0, 20.0),
            Vec3f::new(0.0, 100.0, 20.0),
        ];
        let mut batch = renderer::RenderBatch3d::default();
        append_face_points_to_3d_batch(
            &mut batch,
            FaceSurface3d::Top,
            &points,
            Color32::from_rgb(64, 128, 255),
        );

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

    fn assert_triangle_winding_matches_vertex_normals(batch: &renderer::RenderBatch3d) {
        for (triangle_index, triangle) in batch.indices.chunks_exact(3).enumerate() {
            let a = batch.vertices[triangle[0] as usize];
            let b = batch.vertices[triangle[1] as usize];
            let c = batch.vertices[triangle[2] as usize];
            let a_pos = Vec3f::new(a.position[0], a.position[1], a.position[2]);
            let b_pos = Vec3f::new(b.position[0], b.position[1], b.position[2]);
            let c_pos = Vec3f::new(c.position[0], c.position[1], c.position[2]);
            let winding_normal = (b_pos - a_pos).cross(c_pos - a_pos).normalized();
            let vertex_normal = Vec3f::new(a.normal[0], a.normal[1], a.normal[2]);

            assert!(
                winding_normal.dot(vertex_normal) > 0.99,
                "triangle {triangle_index} winding does not match vertex normal"
            );
        }
    }
}
