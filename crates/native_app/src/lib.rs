use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io::Write, path::Path};

use drc::{DrcIssueStore, RuleDeck, run_drc};
use geometry_core::{Coord, Point, Rect, Vector};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::gdsii::export_gdsii;
pub use layout_model::workspace::BuiltinDemoWorkspaceReport;
use layout_model::{
    Document, LayerId, MarkerState, Operation, ProcessLayer, Shape, ShapeId, ShapeKind,
    connectivity::{ConnectivityIssueKind, extract_connectivity},
    cross_section::MaterialId,
    environment::{
        CleanroomEnvironment, EnvironmentAlarmSeverity, EnvironmentSensor, TrendDirection,
    },
    equipment::{
        AlarmSeverity, HostCommand, RecipeId as EquipmentRecipeId, RecipeSelection,
        Tool as EquipmentTool, ToolId as EquipmentToolId, ToolState as EquipmentToolState,
    },
    experiment::{
        ExperimentAnalysisSummary, ExperimentPlan, ExperimentRun, ExperimentRunId,
        ExperimentRunStatus, FactorId, ResponseSpec, ResponseSpecId, ResponseValue,
        ResponseValueStatus, format_compact_number,
    },
    genealogy::WaferRef,
    inventory::{InventoryAlertKind, MaterialLot, MaterialLotId},
    layout_diff::{LayoutDiffReport, ShapeChangeKind, diff_documents},
    maintenance::{DueState, FabDate, MaintenanceKind, ToolReleaseState},
    mask::{MaskCheckReport, MaskIssueSeverity, ReticlePrep},
    mes::{LotId, ToolId as SchedulerToolId, WaferId},
    metrology::{DieCoord, MeasurementKind, MeasurementStatus, MeasurementSummary, WaferMap},
    notebook::{NotebookEntryId, NotebookFilter, NotebookLinkKind},
    process_control::{
        ControlAction, ControlActionId, ControlActionState, ControlLoop, ControlLoopId,
        ControlTrendPoint,
    },
    process_flow::{ProcessFlowNodeId, ProcessFlowNodeKind},
    safety::SafetySeverity,
    scheduler::DispatchPolicy,
    spc_fdc::{ControlChart, FindingSource, MonitorSeverity, SensorTrace, SpcFdcMonitor},
    workspace::{WORKSPACE_DATASET_SCHEMA_VERSION, WorkspaceDataset},
    yield_analysis::{DieOutcome, FailureMode, ProcessMeasurement, YieldAnalysis, YieldSummary},
};
use operad::{
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, ApproxTextMeasurer, ClipBehavior,
    ColorRgba, FontWeight, InputBehavior, RenderFrameOutput, RenderFrameRequest, RenderOptions,
    RenderTarget, RendererAdapter, ScenePrimitive, ScrollAxes, StrokeStyle, TextStyle, TextWrap,
    UiDocument, UiNode, UiNodeStyle, UiPoint, UiRect, UiSize, UiVisual, WgpuRenderer, layout,
    platform::PixelSize, root_style,
};
#[cfg(not(target_arch = "wasm32"))]
use renderer::gpu::OffscreenRenderRequest;
#[cfg(not(target_arch = "wasm32"))]
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
const LAYOUT_MIN_ZOOM: f32 = 0.000_05;
#[cfg(not(target_arch = "wasm32"))]
const LAYOUT_MAX_ZOOM: f32 = 512.0;
const INVENTORY_DEMO_TODAY: u32 = 20260508;
const MASK_ISSUE_PAGE_SIZE: usize = 50;
const LAYOUT_DIFF_DEFAULT_PAGE_SIZE: usize = 50;
const LAYOUT_DIFF_PAGE_SIZE_OPTIONS: [usize; 4] = [25, 50, 100, 200];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Benchmark3dOptions {
    pub frames: usize,
    pub warmup_frames: usize,
}

impl Default for Benchmark3dOptions {
    fn default() -> Self {
        Self {
            frames: 180,
            warmup_frames: 24,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMenu {
    File,
    Edit,
    View,
    Bookmarks,
    Display,
    Options,
    Tools,
    Macros,
    Help,
}

impl AppMenu {
    pub const ALL: [Self; 9] = [
        Self::File,
        Self::Edit,
        Self::View,
        Self::Bookmarks,
        Self::Display,
        Self::Options,
        Self::Tools,
        Self::Macros,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::View => "View",
            Self::Bookmarks => "Bookmarks",
            Self::Display => "Display",
            Self::Options => "Options",
            Self::Tools => "Tools",
            Self::Macros => "Macros",
            Self::Help => "Help",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Edit => "edit",
            Self::View => "view",
            Self::Bookmarks => "bookmarks",
            Self::Display => "display",
            Self::Options => "options",
            Self::Tools => "tools",
            Self::Macros => "macros",
            Self::Help => "help",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "file" => Some(Self::File),
            "edit" => Some(Self::Edit),
            "view" => Some(Self::View),
            "bookmarks" => Some(Self::Bookmarks),
            "display" => Some(Self::Display),
            "options" => Some(Self::Options),
            "tools" => Some(Self::Tools),
            "macros" => Some(Self::Macros),
            "help" => Some(Self::Help),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolMode {
    Select,
    Rect,
    Polygon,
    Path,
    Via,
    Measure,
    Route,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetrologyMapMode {
    ValueMap,
    DeviationMap,
    SpecWindow,
    DefectReview,
    ReviewQueue,
    OverlayVectors,
}

impl MetrologyMapMode {
    pub const ALL: [Self; 6] = [
        Self::ValueMap,
        Self::DeviationMap,
        Self::SpecWindow,
        Self::DefectReview,
        Self::ReviewQueue,
        Self::OverlayVectors,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::ValueMap => "Value map",
            Self::DeviationMap => "Deviation map",
            Self::SpecWindow => "Spec window",
            Self::DefectReview => "Defect review",
            Self::ReviewQueue => "Review queue",
            Self::OverlayVectors => "Overlay vectors",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::ValueMap => "Value",
            Self::DeviationMap => "Delta",
            Self::SpecWindow => "Spec",
            Self::DefectReview => "Defects",
            Self::ReviewQueue => "Review",
            Self::OverlayVectors => "Overlay",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::ValueMap => "value",
            Self::DeviationMap => "delta",
            Self::SpecWindow => "spec",
            Self::DefectReview => "defects",
            Self::ReviewQueue => "review",
            Self::OverlayVectors => "overlay",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "value" => Some(Self::ValueMap),
            "delta" => Some(Self::DeviationMap),
            "spec" => Some(Self::SpecWindow),
            "defects" => Some(Self::DefectReview),
            "review" => Some(Self::ReviewQueue),
            "overlay" => Some(Self::OverlayVectors),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YieldMapFilter {
    All,
    Failing,
    Passing,
}

impl YieldMapFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Failing, Self::Passing];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Failing => "Failing",
            Self::Passing => "Passing",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Failing => "failing",
            Self::Passing => "passing",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "failing" => Some(Self::Failing),
            "passing" => Some(Self::Passing),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceWorkFilter {
    Actionable,
    Upcoming,
    Calibration,
    Locked,
    All,
}

impl MaintenanceWorkFilter {
    pub const ALL: [Self; 5] = [
        Self::Actionable,
        Self::Upcoming,
        Self::Calibration,
        Self::Locked,
        Self::All,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Actionable => "Action",
            Self::Upcoming => "Upcoming",
            Self::Calibration => "Cal",
            Self::Locked => "Locked",
            Self::All => "All",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::Actionable => "Action required",
            Self::Upcoming => "Upcoming",
            Self::Calibration => "Calibration",
            Self::Locked => "Locked tools",
            Self::All => "All tasks",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Actionable => "actionable",
            Self::Upcoming => "upcoming",
            Self::Calibration => "calibration",
            Self::Locked => "locked",
            Self::All => "all",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "actionable" => Some(Self::Actionable),
            "upcoming" => Some(Self::Upcoming),
            "calibration" => Some(Self::Calibration),
            "locked" => Some(Self::Locked),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaintenanceHistoryFilter {
    SelectedTool,
    AllTools,
}

impl MaintenanceHistoryFilter {
    pub const ALL: [Self; 2] = [Self::SelectedTool, Self::AllTools];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SelectedTool => "Selected",
            Self::AllTools => "All tools",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::SelectedTool => "selected",
            Self::AllTools => "all",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "selected" => Some(Self::SelectedTool),
            "all" => Some(Self::AllTools),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryQuickFilter {
    All,
    NeedsAction,
    ProductHold,
    LowStock,
    ExpiringSoon,
    InUse,
}

impl InventoryQuickFilter {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::NeedsAction,
        Self::ProductHold,
        Self::LowStock,
        Self::ExpiringSoon,
        Self::InUse,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NeedsAction => "Action",
            Self::ProductHold => "Hold",
            Self::LowStock => "Low",
            Self::ExpiringSoon => "Expiring",
            Self::InUse => "In use",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::All => "All material lots",
            Self::NeedsAction => "Needs action",
            Self::ProductHold => "Product hold",
            Self::LowStock => "Low stock",
            Self::ExpiringSoon => "Expiring soon",
            Self::InUse => "In use",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsAction => "action",
            Self::ProductHold => "hold",
            Self::LowStock => "low-stock",
            Self::ExpiringSoon => "expiring",
            Self::InUse => "in-use",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "action" => Some(Self::NeedsAction),
            "hold" => Some(Self::ProductHold),
            "low-stock" => Some(Self::LowStock),
            "expiring" => Some(Self::ExpiringSoon),
            "in-use" => Some(Self::InUse),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessFlowNodeFilter {
    All,
    RecipeSteps,
    Metrology,
    Holds,
    Rework,
}

impl ProcessFlowNodeFilter {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::RecipeSteps,
        Self::Metrology,
        Self::Holds,
        Self::Rework,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::RecipeSteps => "Recipes",
            Self::Metrology => "Metrology",
            Self::Holds => "Holds",
            Self::Rework => "Rework",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::RecipeSteps => "recipes",
            Self::Metrology => "metrology",
            Self::Holds => "holds",
            Self::Rework => "rework",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "recipes" => Some(Self::RecipeSteps),
            "metrology" => Some(Self::Metrology),
            "holds" => Some(Self::Holds),
            "rework" => Some(Self::Rework),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpcSeverityFilter {
    All,
    Critical,
    Warning,
    Advisory,
}

impl SpcSeverityFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Critical, Self::Warning, Self::Advisory];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Critical => "Critical",
            Self::Warning => "Warning",
            Self::Advisory => "Advisory",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::All => "All severities",
            Self::Critical => "Critical findings",
            Self::Warning => "Warning findings",
            Self::Advisory => "Advisory findings",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Advisory => "advisory",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "critical" => Some(Self::Critical),
            "warning" => Some(Self::Warning),
            "advisory" => Some(Self::Advisory),
            _ => None,
        }
    }

    fn matches(self, severity: MonitorSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Critical => severity == MonitorSeverity::Critical,
            Self::Warning => severity == MonitorSeverity::Warning,
            Self::Advisory => severity == MonitorSeverity::Advisory,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpcSourceFilter {
    All,
    Spc,
    Fdc,
    Alarm,
}

impl SpcSourceFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Spc, Self::Fdc, Self::Alarm];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Spc => "SPC",
            Self::Fdc => "FDC",
            Self::Alarm => "Alarm",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::All => "All sources",
            Self::Spc => "SPC rules",
            Self::Fdc => "FDC traces",
            Self::Alarm => "Equipment alarms",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Spc => "spc",
            Self::Fdc => "fdc",
            Self::Alarm => "alarm",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "spc" => Some(Self::Spc),
            "fdc" => Some(Self::Fdc),
            "alarm" => Some(Self::Alarm),
            _ => None,
        }
    }

    fn matches(self, source: FindingSource) -> bool {
        match self {
            Self::All => true,
            Self::Spc => source == FindingSource::SpcRule,
            Self::Fdc => source == FindingSource::FdcTrace,
            Self::Alarm => source == FindingSource::EquipmentAlarm,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskIssueSeverityFilter {
    All,
    Errors,
    Warnings,
}

impl MaskIssueSeverityFilter {
    pub const ALL: [Self; 3] = [Self::All, Self::Errors, Self::Warnings];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Errors => "Errors",
            Self::Warnings => "Warnings",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::All => "all severities",
            Self::Errors => "errors only",
            Self::Warnings => "warnings only",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Errors => "errors",
            Self::Warnings => "warnings",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "errors" => Some(Self::Errors),
            "warnings" => Some(Self::Warnings),
            _ => None,
        }
    }

    fn matches(self, severity: MaskIssueSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Errors => severity == MaskIssueSeverity::Error,
            Self::Warnings => severity == MaskIssueSeverity::Warning,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskIssueGrouping {
    Code,
    Layer,
    Field,
    Block,
}

impl MaskIssueGrouping {
    pub const ALL: [Self; 4] = [Self::Code, Self::Layer, Self::Field, Self::Block];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::Layer => "Layer",
            Self::Field => "Field",
            Self::Block => "Block",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Layer => "layer",
            Self::Field => "field",
            Self::Block => "block",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "code" => Some(Self::Code),
            "layer" => Some(Self::Layer),
            "field" => Some(Self::Field),
            "block" => Some(Self::Block),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDiffSource {
    Current,
    Demo,
    Hierarchy,
    Empty,
}

impl LayoutDiffSource {
    pub const ALL: [Self; 4] = [Self::Current, Self::Demo, Self::Hierarchy, Self::Empty];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Current => "Current",
            Self::Demo => "Demo",
            Self::Hierarchy => "Hierarchy",
            Self::Empty => "Empty",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::Current => "Current workspace",
            Self::Demo => "Demo layout",
            Self::Hierarchy => "Hierarchy demo",
            Self::Empty => "Empty layout",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Demo => "demo",
            Self::Hierarchy => "hierarchy",
            Self::Empty => "empty",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "current" => Some(Self::Current),
            "demo" => Some(Self::Demo),
            "hierarchy" => Some(Self::Hierarchy),
            "empty" => Some(Self::Empty),
            _ => None,
        }
    }

    fn document(self, current: &Document) -> Document {
        match self {
            Self::Current => current.clone(),
            Self::Demo => Document::demo(),
            Self::Hierarchy => Document::hierarchy_demo(),
            Self::Empty => Document::new("empty layout"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutChangeFilter {
    All,
    Added,
    Removed,
    Modified,
}

impl LayoutChangeFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Added, Self::Removed, Self::Modified];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Added => "Added",
            Self::Removed => "Removed",
            Self::Modified => "Modified",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::All => "All changes",
            Self::Added => "Added only",
            Self::Removed => "Removed only",
            Self::Modified => "Modified only",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "added" => Some(Self::Added),
            "removed" => Some(Self::Removed),
            "modified" => Some(Self::Modified),
            _ => None,
        }
    }

    fn matches(self, kind: ShapeChangeKind) -> bool {
        match self {
            Self::All => true,
            Self::Added => kind == ShapeChangeKind::Added,
            Self::Removed => kind == ShapeChangeKind::Removed,
            Self::Modified => kind == ShapeChangeKind::Modified,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutReviewDisposition {
    NeedsReview,
    Approved,
    ChangesRequested,
}

impl LayoutReviewDisposition {
    pub const ALL: [Self; 3] = [Self::NeedsReview, Self::Approved, Self::ChangesRequested];

    pub const fn label(self) -> &'static str {
        match self {
            Self::NeedsReview => "Needs review",
            Self::Approved => "Approved",
            Self::ChangesRequested => "Changes requested",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::NeedsReview => "needs-review",
            Self::Approved => "approved",
            Self::ChangesRequested => "changes-requested",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "needs-review" => Some(Self::NeedsReview),
            "approved" => Some(Self::Approved),
            "changes-requested" => Some(Self::ChangesRequested),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceImpactMode {
    ToolRun,
    Step,
    Material,
}

impl TraceImpactMode {
    pub const ALL: [Self; 3] = [Self::ToolRun, Self::Step, Self::Material];

    pub const fn label(self) -> &'static str {
        match self {
            Self::ToolRun => "Tool run",
            Self::Step => "Step",
            Self::Material => "Material",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::ToolRun => "tool-run",
            Self::Step => "step",
            Self::Material => "material",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "tool-run" => Some(Self::ToolRun),
            "step" => Some(Self::Step),
            "material" => Some(Self::Material),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TraceSelection {
    Lot(LotId),
    Wafer(WaferRef),
    Process(u64),
    MaterialUse(u64),
    Event(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExperimentRunFilter {
    All,
    NeedsAnyResponse,
    NeedsSelectedResponse,
    InProgress,
    Complete,
    OutOfSpec,
    Blocked,
}

impl ExperimentRunFilter {
    const ALL: [Self; 7] = [
        Self::All,
        Self::NeedsAnyResponse,
        Self::NeedsSelectedResponse,
        Self::InProgress,
        Self::Complete,
        Self::OutOfSpec,
        Self::Blocked,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::All => "All runs",
            Self::NeedsAnyResponse => "Needs any response",
            Self::NeedsSelectedResponse => "Needs selected response",
            Self::InProgress => "In progress",
            Self::Complete => "Complete",
            Self::OutOfSpec => "Out of spec",
            Self::Blocked => "Blocked",
        }
    }

    const fn short_label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NeedsAnyResponse => "Any pending",
            Self::NeedsSelectedResponse => "Primary pending",
            Self::InProgress => "In prog",
            Self::Complete => "Complete",
            Self::OutOfSpec => "OOS",
            Self::Blocked => "Blocked",
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsAnyResponse => "needs-any-response",
            Self::NeedsSelectedResponse => "needs-selected-response",
            Self::InProgress => "in-progress",
            Self::Complete => "complete",
            Self::OutOfSpec => "out-of-spec",
            Self::Blocked => "blocked",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "needs-any-response" => Some(Self::NeedsAnyResponse),
            "needs-selected-response" => Some(Self::NeedsSelectedResponse),
            "in-progress" => Some(Self::InProgress),
            "complete" => Some(Self::Complete),
            "out-of-spec" => Some(Self::OutOfSpec),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }

    fn matches(self, row: &ExperimentRunMatrixRow) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAnyResponse => row.missing_count > 0,
            Self::NeedsSelectedResponse => row.needs_selected_response,
            Self::InProgress => row.status_kind == ExperimentRunStatus::InProgress,
            Self::Complete => row.status_kind == ExperimentRunStatus::Complete,
            Self::OutOfSpec => row.has_out_of_spec,
            Self::Blocked => row.status_kind == ExperimentRunStatus::Blocked,
        }
    }
}

#[derive(Clone, Debug)]
struct ExperimentRunMatrixRow {
    run_id: String,
    selected: bool,
    run_order: u32,
    lot_id: String,
    wafer_id: String,
    slot: u8,
    block: String,
    factor_values: Vec<String>,
    status: String,
    status_kind: ExperimentRunStatus,
    capture_summary: String,
    missing_count: usize,
    needs_selected_response: bool,
    has_out_of_spec: bool,
    primary_value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NotebookEntryAction {
    AddFollowUpPlan,
    InsertMetrologyReview,
    RequestImageEvidence,
    TagHandoff,
}

impl NotebookEntryAction {
    const ALL: [Self; 4] = [
        Self::AddFollowUpPlan,
        Self::InsertMetrologyReview,
        Self::RequestImageEvidence,
        Self::TagHandoff,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::AddFollowUpPlan => "Add follow-up",
            Self::InsertMetrologyReview => "Metrology review",
            Self::RequestImageEvidence => "Image evidence",
            Self::TagHandoff => "Tag handoff",
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::AddFollowUpPlan => "follow-up",
            Self::InsertMetrologyReview => "metrology-review",
            Self::RequestImageEvidence => "image-evidence",
            Self::TagHandoff => "handoff",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "follow-up" => Some(Self::AddFollowUpPlan),
            "metrology-review" => Some(Self::InsertMetrologyReview),
            "image-evidence" => Some(Self::RequestImageEvidence),
            "handoff" => Some(Self::TagHandoff),
            _ => None,
        }
    }
}

impl ToolMode {
    pub const ALL: [Self; 7] = [
        Self::Select,
        Self::Rect,
        Self::Polygon,
        Self::Path,
        Self::Via,
        Self::Measure,
        Self::Route,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Rect => "Rect",
            Self::Polygon => "Poly",
            Self::Path => "Path",
            Self::Via => "Via",
            Self::Measure => "Measure",
            Self::Route => "Route",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Rect => "rect",
            Self::Polygon => "polygon",
            Self::Path => "path",
            Self::Via => "via",
            Self::Measure => "measure",
            Self::Route => "route",
        }
    }

    fn from_slug(value: &str) -> Option<Self> {
        match value {
            "select" => Some(Self::Select),
            "rect" => Some(Self::Rect),
            "polygon" | "poly" => Some(Self::Polygon),
            "path" => Some(Self::Path),
            "via" => Some(Self::Via),
            "measure" => Some(Self::Measure),
            "route" => Some(Self::Route),
            _ => None,
        }
    }
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
    pub const ALL: [Self; 20] = [
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

    pub const fn label(self) -> &'static str {
        match self {
            Self::Workflow => "Fab Workflow",
            Self::Layout2d => "Layout Editor",
            Self::Layout3d => "3D Layout",
            Self::MaskPrep => "Reticle Prep",
            Self::LayoutDiff => "Layout Diff",
            Self::FabControl => "Fab Control",
            Self::Inventory => "Inventory",
            Self::Maintenance => "Maintenance",
            Self::Environment => "Cleanroom",
            Self::Scheduler => "Dispatch",
            Self::Safety => "Safety",
            Self::Traceability => "Lot Traceability",
            Self::Metrology => "Metrology",
            Self::Yield => "Yield Dashboard",
            Self::SpcFdc => "SPC / FDC Monitor",
            Self::ProcessFlow => "Process Flow",
            Self::ProcessControl => "Run-to-Run Control",
            Self::CrossSection => "Process Cross-Section",
            Self::Experiment => "DOE Planner",
            Self::Notebook => "Lab Notebook",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Layout2d => "layout2d",
            Self::Layout3d => "layout3d",
            Self::MaskPrep => "mask-prep",
            Self::LayoutDiff => "layout-diff",
            Self::FabControl => "fab-control",
            Self::Inventory => "inventory",
            Self::Maintenance => "maintenance",
            Self::Environment => "environment",
            Self::Scheduler => "scheduler",
            Self::Safety => "safety",
            Self::Traceability => "traceability",
            Self::Metrology => "metrology",
            Self::Yield => "yield",
            Self::SpcFdc => "spc-fdc",
            Self::ProcessFlow => "process-flow",
            Self::ProcessControl => "process-control",
            Self::CrossSection => "cross-section",
            Self::Experiment => "experiment",
            Self::Notebook => "notebook",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct OperadAuditReport {
    pub operad_version: &'static str,
    pub view: StartupView,
    pub viewport: UiSize,
    pub paint_items: usize,
    pub layout_warnings: usize,
    pub document_shapes: usize,
    pub workspace_lots: usize,
    pub equipment_tools: usize,
}

impl OperadAuditReport {
    pub fn summary(&self) -> String {
        format!(
            "Operad {} view={} viewport={}x{} paint_items={} layout_warnings={} shapes={} lots={} tools={}",
            self.operad_version,
            self.view.slug(),
            self.viewport.width.round() as u32,
            self.viewport.height.round() as u32,
            self.paint_items,
            self.layout_warnings,
            self.document_shapes,
            self.workspace_lots,
            self.equipment_tools
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperadSnapshotReport {
    pub audit: OperadAuditReport,
    pub render: RenderFrameOutput,
}

impl OperadSnapshotReport {
    pub fn summary(&self) -> String {
        let snapshot = self
            .render
            .snapshot
            .as_ref()
            .map(|image| format!(" snapshot={}x{}", image.size.width, image.size.height))
            .unwrap_or_default();
        format!(
            "{} batches={}{}",
            self.audit.summary(),
            self.render.batches.len(),
            snapshot
        )
    }
}

pub struct FabricadApp {
    workspace: WorkspaceDataset,
    active_view: StartupView,
    active_menu: Option<AppMenu>,
    active_tool: ToolMode,
    active_layer: LayerId,
    selected_layout_shape: Option<ShapeId>,
    layout_clipboard_shapes: Vec<Shape>,
    workflow_focus_lot: Option<String>,
    selected_equipment_tool: Option<EquipmentToolId>,
    equipment_recipe_drafts: BTreeMap<EquipmentToolId, EquipmentRecipeId>,
    selected_maintenance_tool: Option<EquipmentToolId>,
    maintenance_work_filter: MaintenanceWorkFilter,
    maintenance_history_filter: MaintenanceHistoryFilter,
    selected_environment_sensor: Option<String>,
    selected_inventory_lot: Option<MaterialLotId>,
    inventory_filter: InventoryQuickFilter,
    mask_source_lot: Option<LotId>,
    mask_issue_severity_filter: MaskIssueSeverityFilter,
    mask_issue_grouping: MaskIssueGrouping,
    mask_issue_page: usize,
    layout_diff_baseline: LayoutDiffSource,
    layout_diff_candidate: LayoutDiffSource,
    layout_diff_changed_only: bool,
    layout_diff_change_filter: LayoutChangeFilter,
    layout_diff_change_page: usize,
    layout_diff_page_size: usize,
    layout_diff_review_state: LayoutReviewDisposition,
    selected_trace_lot: Option<LotId>,
    selected_trace_wafer: Option<WaferId>,
    selected_trace_detail: Option<TraceSelection>,
    trace_impact_mode: TraceImpactMode,
    trace_related_only: bool,
    selected_notebook_entry: Option<NotebookEntryId>,
    notebook_tag_filter: Option<String>,
    notebook_link_kind_filter: Option<NotebookLinkKind>,
    notebook_preview_mode: bool,
    notebook_followups_only: bool,
    scheduler_policy: DispatchPolicy,
    selected_scheduler_tool: Option<SchedulerToolId>,
    scheduler_min_priority: u8,
    scheduler_conflicts_only: bool,
    scheduler_focus_selected_tool: bool,
    selected_safety_tool: Option<String>,
    acknowledged_conditions: BTreeSet<String>,
    acknowledged_lockouts: BTreeSet<String>,
    acknowledged_incidents: BTreeSet<String>,
    selected_process_node: Option<ProcessFlowNodeId>,
    process_flow_filter: ProcessFlowNodeFilter,
    process_flow_errors_only: bool,
    selected_control_loop: Option<ControlLoopId>,
    selected_control_action: Option<ControlActionId>,
    selected_spc_chart: Option<String>,
    selected_fdc_trace: Option<String>,
    spc_severity_filter: SpcSeverityFilter,
    spc_source_filter: SpcSourceFilter,
    spc_context_filter: String,
    cross_section_step: usize,
    selected_cross_section_material: Option<MaterialId>,
    cross_section_show_mask: bool,
    cross_section_show_dimensions: bool,
    cross_section_show_risks: bool,
    metrology_map_mode: MetrologyMapMode,
    metrology_kind: MeasurementKind,
    metrology_failed_only: bool,
    selected_die: Option<DieCoord>,
    yield_map_filter: YieldMapFilter,
    show_only_attention_wafers: bool,
    show_only_excursions: bool,
    selected_yield_lot: Option<String>,
    selected_yield_wafer: Option<String>,
    selected_experiment_run: Option<ExperimentRunId>,
    selected_experiment_response: Option<ResponseSpecId>,
    experiment_capture_value: f64,
    experiment_run_filter: ExperimentRunFilter,
    experiment_lot_filter: Option<String>,
    experiment_show_missing_only: bool,
    status_message: String,
    dark_theme: bool,
    snap_enabled: bool,
    show_grid: bool,
    show_3d_grid: bool,
    show_origin_marker: bool,
    show_drc_overlay: bool,
    show_inspector: bool,
    show_layers: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiScale {
    factor: f32,
}

impl UiScale {
    pub fn new(factor: f32) -> Self {
        Self {
            factor: factor.clamp(1.0, 3.0),
        }
    }

    pub fn factor(self) -> f32 {
        self.factor
    }

    fn value(self, value: f32) -> f32 {
        value * self.factor
    }
}

impl FabricadApp {
    pub fn new_with_options(options: StartupOptions) -> Self {
        let mut workspace = if let Some(count) = options.stress_count {
            let mut workspace = WorkspaceDataset::demo();
            workspace.document = Document::stress(count);
            workspace
        } else if options.hierarchy_demo || options.hierarchy_workflow_demo {
            let mut workspace = WorkspaceDataset::demo();
            workspace.document = Document::hierarchy_demo();
            workspace
        } else {
            WorkspaceDataset::demo()
        };
        let moved_shape = options
            .move_first_vertex
            .then(|| move_first_layout_shape_vertex(&mut workspace.document))
            .flatten();
        let active_layer = default_active_layer(&workspace);
        let selected_layout_shape = moved_shape.or_else(|| {
            options
                .select_first_shape
                .then(|| default_layout_shape(&workspace))
                .flatten()
        });
        let selected_yield_lot = default_yield_lot(&workspace);
        let selected_yield_wafer = default_yield_wafer(&workspace, selected_yield_lot.as_deref());
        let workflow_focus_lot = default_workflow_focus_lot(&workspace);
        let selected_equipment_tool = default_equipment_tool(&workspace);
        let selected_maintenance_tool = default_maintenance_tool(&workspace);
        let selected_environment_sensor = default_environment_sensor(&workspace);
        let selected_inventory_lot = default_inventory_lot(&workspace, InventoryQuickFilter::All);
        let mask_source_lot = default_mask_lot(&workspace);
        let selected_scheduler_tool = default_scheduler_tool(&workspace);
        let selected_safety_tool = default_safety_tool(&workspace);
        let selected_process_node = default_process_flow_node(&workspace);
        let selected_control_loop = default_control_loop(&workspace);
        let selected_control_action =
            default_control_action(&workspace, selected_control_loop.as_ref());
        let selected_spc_chart = default_spc_chart(&workspace);
        let selected_fdc_trace = default_fdc_trace(&workspace);
        let selected_cross_section_material = default_cross_section_material(&workspace);
        let selected_trace_lot = default_trace_lot(&workspace);
        let selected_trace_wafer = default_trace_wafer(&workspace, selected_trace_lot.as_ref());
        let selected_trace_detail = default_trace_selection(
            &workspace,
            selected_trace_lot.as_ref(),
            selected_trace_wafer.as_ref(),
        );
        let selected_notebook_entry = default_notebook_entry(&workspace);
        let selected_experiment_run = default_experiment_run(&workspace);
        let selected_experiment_response = default_experiment_response(&workspace);

        let active_view = options
            .view_mode
            .or_else(|| options.view_3d.then_some(StartupView::Layout3d))
            .unwrap_or(StartupView::Workflow);

        Self {
            workspace,
            active_view,
            active_menu: options.show_options.then_some(AppMenu::Options),
            active_tool: ToolMode::Select,
            active_layer,
            selected_layout_shape,
            layout_clipboard_shapes: Vec::new(),
            workflow_focus_lot,
            selected_equipment_tool,
            equipment_recipe_drafts: BTreeMap::new(),
            selected_maintenance_tool,
            maintenance_work_filter: MaintenanceWorkFilter::Actionable,
            maintenance_history_filter: MaintenanceHistoryFilter::SelectedTool,
            selected_environment_sensor,
            selected_inventory_lot,
            inventory_filter: InventoryQuickFilter::All,
            mask_source_lot,
            mask_issue_severity_filter: MaskIssueSeverityFilter::All,
            mask_issue_grouping: MaskIssueGrouping::Code,
            mask_issue_page: 0,
            layout_diff_baseline: LayoutDiffSource::Demo,
            layout_diff_candidate: LayoutDiffSource::Current,
            layout_diff_changed_only: true,
            layout_diff_change_filter: LayoutChangeFilter::All,
            layout_diff_change_page: 0,
            layout_diff_page_size: LAYOUT_DIFF_DEFAULT_PAGE_SIZE,
            layout_diff_review_state: LayoutReviewDisposition::NeedsReview,
            selected_trace_lot,
            selected_trace_wafer,
            selected_trace_detail,
            trace_impact_mode: TraceImpactMode::ToolRun,
            trace_related_only: false,
            selected_notebook_entry,
            notebook_tag_filter: None,
            notebook_link_kind_filter: None,
            notebook_preview_mode: true,
            notebook_followups_only: false,
            scheduler_policy: DispatchPolicy::PriorityThenFifo,
            selected_scheduler_tool,
            scheduler_min_priority: 0,
            scheduler_conflicts_only: false,
            scheduler_focus_selected_tool: false,
            selected_safety_tool,
            acknowledged_conditions: BTreeSet::new(),
            acknowledged_lockouts: BTreeSet::new(),
            acknowledged_incidents: BTreeSet::new(),
            selected_process_node,
            process_flow_filter: ProcessFlowNodeFilter::All,
            process_flow_errors_only: false,
            selected_control_loop,
            selected_control_action,
            selected_spc_chart,
            selected_fdc_trace,
            spc_severity_filter: SpcSeverityFilter::All,
            spc_source_filter: SpcSourceFilter::All,
            spc_context_filter: String::new(),
            cross_section_step: 0,
            selected_cross_section_material,
            cross_section_show_mask: true,
            cross_section_show_dimensions: true,
            cross_section_show_risks: true,
            metrology_map_mode: MetrologyMapMode::ValueMap,
            metrology_kind: MeasurementKind::CriticalDimensionNm,
            metrology_failed_only: false,
            selected_die: None,
            yield_map_filter: YieldMapFilter::All,
            show_only_attention_wafers: false,
            show_only_excursions: false,
            selected_yield_lot,
            selected_yield_wafer,
            selected_experiment_run,
            selected_experiment_response,
            experiment_capture_value: 72.0,
            experiment_run_filter: ExperimentRunFilter::All,
            experiment_lot_filter: None,
            experiment_show_missing_only: false,
            status_message: "Ready".to_string(),
            dark_theme: true,
            snap_enabled: true,
            show_grid: true,
            show_3d_grid: true,
            show_origin_marker: true,
            show_drc_overlay: true,
            show_inspector: true,
            show_layers: true,
        }
    }

    pub fn workspace(&self) -> &WorkspaceDataset {
        &self.workspace
    }

    pub fn active_view(&self) -> StartupView {
        self.active_view
    }

    pub fn active_menu(&self) -> Option<AppMenu> {
        self.active_menu
    }

    pub fn active_tool(&self) -> ToolMode {
        self.active_tool
    }

    pub fn active_layer(&self) -> LayerId {
        self.active_layer
    }

    pub fn selected_layout_shape(&self) -> Option<ShapeId> {
        self.selected_layout_shape
    }

    pub fn workflow_focus_lot(&self) -> Option<&str> {
        self.workflow_focus_lot.as_deref()
    }

    pub fn selected_equipment_tool(&self) -> Option<&EquipmentToolId> {
        self.selected_equipment_tool.as_ref()
    }

    pub fn selected_maintenance_tool(&self) -> Option<&EquipmentToolId> {
        self.selected_maintenance_tool.as_ref()
    }

    pub fn maintenance_work_filter(&self) -> MaintenanceWorkFilter {
        self.maintenance_work_filter
    }

    pub fn maintenance_history_filter(&self) -> MaintenanceHistoryFilter {
        self.maintenance_history_filter
    }

    pub fn selected_environment_sensor(&self) -> Option<&str> {
        self.selected_environment_sensor.as_deref()
    }

    pub fn selected_inventory_lot(&self) -> Option<&MaterialLotId> {
        self.selected_inventory_lot.as_ref()
    }

    pub fn inventory_filter(&self) -> InventoryQuickFilter {
        self.inventory_filter
    }

    pub fn mask_source_lot(&self) -> Option<&LotId> {
        self.mask_source_lot.as_ref()
    }

    pub fn mask_issue_severity_filter(&self) -> MaskIssueSeverityFilter {
        self.mask_issue_severity_filter
    }

    pub fn mask_issue_grouping(&self) -> MaskIssueGrouping {
        self.mask_issue_grouping
    }

    pub fn layout_diff_baseline(&self) -> LayoutDiffSource {
        self.layout_diff_baseline
    }

    pub fn layout_diff_candidate(&self) -> LayoutDiffSource {
        self.layout_diff_candidate
    }

    pub fn layout_diff_changed_only(&self) -> bool {
        self.layout_diff_changed_only
    }

    pub fn layout_diff_change_filter(&self) -> LayoutChangeFilter {
        self.layout_diff_change_filter
    }

    pub fn layout_diff_review_state(&self) -> LayoutReviewDisposition {
        self.layout_diff_review_state
    }

    pub fn selected_trace_lot(&self) -> Option<&LotId> {
        self.selected_trace_lot.as_ref()
    }

    pub fn selected_trace_wafer(&self) -> Option<&WaferId> {
        self.selected_trace_wafer.as_ref()
    }

    pub fn trace_impact_mode(&self) -> TraceImpactMode {
        self.trace_impact_mode
    }

    pub fn trace_related_only(&self) -> bool {
        self.trace_related_only
    }

    pub fn selected_notebook_entry(&self) -> Option<&NotebookEntryId> {
        self.selected_notebook_entry.as_ref()
    }

    pub fn notebook_tag_filter(&self) -> Option<&str> {
        self.notebook_tag_filter.as_deref()
    }

    pub fn notebook_link_kind_filter(&self) -> Option<NotebookLinkKind> {
        self.notebook_link_kind_filter
    }

    pub fn notebook_preview_mode(&self) -> bool {
        self.notebook_preview_mode
    }

    pub fn notebook_followups_only(&self) -> bool {
        self.notebook_followups_only
    }

    pub fn scheduler_policy(&self) -> DispatchPolicy {
        self.scheduler_policy
    }

    pub fn selected_scheduler_tool(&self) -> Option<&SchedulerToolId> {
        self.selected_scheduler_tool.as_ref()
    }

    pub fn scheduler_min_priority(&self) -> u8 {
        self.scheduler_min_priority
    }

    pub fn scheduler_conflicts_only(&self) -> bool {
        self.scheduler_conflicts_only
    }

    pub fn scheduler_focus_selected_tool(&self) -> bool {
        self.scheduler_focus_selected_tool
    }

    pub fn selected_safety_tool(&self) -> Option<&str> {
        self.selected_safety_tool.as_deref()
    }

    pub fn acknowledged_count(&self) -> usize {
        self.acknowledged_conditions.len()
            + self.acknowledged_lockouts.len()
            + self.acknowledged_incidents.len()
    }

    pub fn selected_process_node(&self) -> Option<&ProcessFlowNodeId> {
        self.selected_process_node.as_ref()
    }

    pub fn process_flow_filter(&self) -> ProcessFlowNodeFilter {
        self.process_flow_filter
    }

    pub fn process_flow_errors_only(&self) -> bool {
        self.process_flow_errors_only
    }

    pub fn selected_control_loop(&self) -> Option<&ControlLoopId> {
        self.selected_control_loop.as_ref()
    }

    pub fn selected_control_action(&self) -> Option<&ControlActionId> {
        self.selected_control_action.as_ref()
    }

    pub fn selected_spc_chart(&self) -> Option<&str> {
        self.selected_spc_chart.as_deref()
    }

    pub fn selected_fdc_trace(&self) -> Option<&str> {
        self.selected_fdc_trace.as_deref()
    }

    pub fn spc_severity_filter(&self) -> SpcSeverityFilter {
        self.spc_severity_filter
    }

    pub fn spc_source_filter(&self) -> SpcSourceFilter {
        self.spc_source_filter
    }

    pub fn cross_section_step(&self) -> usize {
        self.cross_section_step
    }

    pub fn cross_section_show_mask(&self) -> bool {
        self.cross_section_show_mask
    }

    pub fn metrology_map_mode(&self) -> MetrologyMapMode {
        self.metrology_map_mode
    }

    pub fn metrology_kind(&self) -> MeasurementKind {
        self.metrology_kind
    }

    pub fn metrology_failed_only(&self) -> bool {
        self.metrology_failed_only
    }

    pub fn selected_die(&self) -> Option<DieCoord> {
        self.selected_die
    }

    pub fn yield_map_filter(&self) -> YieldMapFilter {
        self.yield_map_filter
    }

    pub fn show_only_attention_wafers(&self) -> bool {
        self.show_only_attention_wafers
    }

    pub fn show_only_excursions(&self) -> bool {
        self.show_only_excursions
    }

    pub fn selected_yield_lot(&self) -> Option<&str> {
        self.selected_yield_lot.as_deref()
    }

    pub fn selected_yield_wafer(&self) -> Option<&str> {
        self.selected_yield_wafer.as_deref()
    }

    pub fn experiment_show_missing_only(&self) -> bool {
        self.experiment_show_missing_only
    }

    fn ensure_experiment_selection(&mut self) {
        if self
            .selected_experiment_run
            .as_ref()
            .is_none_or(|run_id| self.workspace.experiment_plan.run(run_id).is_none())
        {
            self.selected_experiment_run = default_experiment_run(&self.workspace);
        }
        if self
            .selected_experiment_response
            .as_ref()
            .is_none_or(|response_id| {
                self.workspace
                    .experiment_plan
                    .response(response_id)
                    .is_none()
            })
        {
            self.selected_experiment_response = default_experiment_response(&self.workspace);
        }
        if self.experiment_lot_filter.as_deref().is_some_and(|lot_id| {
            !self
                .workspace
                .experiment_plan
                .runs
                .iter()
                .any(|run| run.assignment.lot_id.as_str() == lot_id)
        }) {
            self.experiment_lot_filter = None;
        }
    }

    fn selected_experiment_response_if_valid(&self) -> Option<ResponseSpecId> {
        self.selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
            .map(|response| response.id.clone())
    }

    fn selected_experiment_response_label(&self) -> String {
        self.selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
            .map(|response| response.name.clone())
            .unwrap_or_else(|| "primary response".to_string())
    }

    fn selected_experiment_run_label(&self) -> String {
        let Some(run) = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| self.workspace.experiment_plan.run(run_id))
        else {
            return "none".to_string();
        };
        format!(
            "{} {} W{:02}",
            run.id, run.assignment.lot_id, run.assignment.slot
        )
    }

    fn experiment_lot_options(&self) -> Vec<String> {
        self.workspace
            .experiment_plan
            .runs
            .iter()
            .map(|run| run.assignment.lot_id.as_str().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn experiment_run_matrix_rows(&self) -> Vec<ExperimentRunMatrixRow> {
        let plan = &self.workspace.experiment_plan;
        let primary_response_id = self.selected_experiment_response_if_valid();
        plan.runs
            .iter()
            .map(|run| {
                let primary_value = primary_response_id
                    .as_ref()
                    .and_then(|response_id| {
                        let response = plan.response(response_id)?;
                        let value = run.responses.get(response_id)?;
                        Some(format_experiment_response_value(value.value, response))
                    })
                    .unwrap_or_else(|| "pending".to_string());
                let missing_count = plan.responses.len().saturating_sub(run.responses.len());
                let has_out_of_spec = run.responses.iter().any(|(response_id, value)| {
                    plan.response(response_id).is_some_and(|response| {
                        response.value_status(value.value) != ResponseValueStatus::InSpec
                    })
                });
                let needs_selected_response = primary_response_id
                    .as_ref()
                    .is_some_and(|response_id| !run.responses.contains_key(response_id));
                ExperimentRunMatrixRow {
                    run_id: run.id.to_string(),
                    selected: self.selected_experiment_run.as_ref() == Some(&run.id),
                    run_order: run.assignment.run_order,
                    lot_id: run.assignment.lot_id.as_str().to_string(),
                    wafer_id: run.assignment.wafer_id.as_str().to_string(),
                    slot: run.assignment.slot,
                    block: run
                        .assignment
                        .block
                        .clone()
                        .unwrap_or_else(|| "unblocked".to_string()),
                    factor_values: plan
                        .factors
                        .iter()
                        .map(|factor| plan.run_factor_label(run, factor))
                        .collect(),
                    status: run.status.label().to_string(),
                    status_kind: run.status,
                    capture_summary: format!("{}/{}", run.responses.len(), plan.responses.len()),
                    missing_count,
                    needs_selected_response,
                    has_out_of_spec,
                    primary_value,
                }
            })
            .collect()
    }

    fn filtered_experiment_run_rows(&self) -> Vec<ExperimentRunMatrixRow> {
        self.experiment_run_matrix_rows()
            .into_iter()
            .filter(|row| {
                self.experiment_lot_filter
                    .as_deref()
                    .is_none_or(|lot_id| row.lot_id == lot_id)
            })
            .filter(|row| self.experiment_run_filter.matches(row))
            .filter(|row| !self.experiment_show_missing_only || row.missing_count > 0)
            .collect()
    }

    fn select_next_pending_experiment_response(&mut self) -> bool {
        let plan = &self.workspace.experiment_plan;
        if plan.runs.is_empty() || plan.responses.is_empty() {
            return false;
        }
        let run_count = plan.runs.len();
        let selected_run_index = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| plan.runs.iter().position(|run| &run.id == run_id))
            .unwrap_or(0);

        for offset in 0..run_count {
            let index = (selected_run_index + offset) % run_count;
            let run = &plan.runs[index];
            for response in &plan.responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                self.selected_experiment_run = Some(run.id.clone());
                self.selected_experiment_response = Some(response.id.clone());
                self.experiment_capture_value = response
                    .target
                    .or_else(|| demo_experiment_response_value(run, &response.id))
                    .unwrap_or_default();
                return true;
            }
        }
        false
    }

    fn capture_selected_experiment_response(&mut self) -> bool {
        let Some(run_id) = self.selected_experiment_run.clone() else {
            self.status_message = "DOE: select a run before capturing a response".to_string();
            return false;
        };
        let Some(response_id) = self.selected_experiment_response.clone() else {
            self.status_message = "DOE: select a response before capturing a value".to_string();
            return false;
        };
        let response_label = self.selected_experiment_response_label();
        let value = ResponseValue {
            value: self.experiment_capture_value,
            measurement_id: Some(format!("DOE-{run_id}-{response_id}")),
            captured_at: "2026-05-06T16:00:00Z".to_string(),
        };
        match self
            .workspace
            .experiment_plan
            .capture_response(&run_id, &response_id, value)
        {
            Ok(()) => {
                self.status_message = format!("DOE: captured {response_label} for {run_id}");
                true
            }
            Err(error) => {
                self.status_message = format!("DOE capture blocked: {error}");
                false
            }
        }
    }

    fn capture_next_demo_experiment_response(&mut self) {
        let runs = self.workspace.experiment_plan.runs.clone();
        let responses = self.workspace.experiment_plan.responses.clone();
        for run in runs {
            for response in &responses {
                if run.responses.contains_key(&response.id) {
                    continue;
                }
                let Some(value) = demo_experiment_response_value(&run, &response.id) else {
                    continue;
                };
                let run_id = run.id.clone();
                let response_id = response.id.clone();
                let capture = ResponseValue {
                    value,
                    measurement_id: Some(format!("SIM-{run_id}-{response_id}")),
                    captured_at: "2026-05-06T16:30:00Z".to_string(),
                };
                match self.workspace.experiment_plan.capture_response(
                    &run_id,
                    &response_id,
                    capture,
                ) {
                    Ok(()) => {
                        self.selected_experiment_run = Some(run_id.clone());
                        self.selected_experiment_response = Some(response_id.clone());
                        self.experiment_capture_value = value;
                        self.status_message =
                            format!("DOE: captured demo {response_id} for {run_id}");
                    }
                    Err(error) => {
                        self.status_message = format!("DOE capture blocked: {error}");
                    }
                }
                return;
            }
        }
        self.status_message = "DOE: no pending demo responses".to_string();
    }

    fn use_demo_experiment_capture_value(&mut self) {
        let Some(run) = self
            .selected_experiment_run
            .as_ref()
            .and_then(|run_id| self.workspace.experiment_plan.run(run_id))
        else {
            self.status_message = "DOE: select a run before loading a demo value".to_string();
            return;
        };
        let Some(response_id) = self.selected_experiment_response.as_ref() else {
            self.status_message = "DOE: select a response before loading a demo value".to_string();
            return;
        };
        let Some(value) = demo_experiment_response_value(run, response_id) else {
            self.status_message = "DOE: no demo value available for this response".to_string();
            return;
        };
        self.experiment_capture_value = value;
        self.status_message = format!("DOE: loaded demo value {}", format_compact_number(value));
    }

    fn use_experiment_response_target(&mut self) {
        let Some(response) = self
            .selected_experiment_response
            .as_ref()
            .and_then(|response_id| self.workspace.experiment_plan.response(response_id))
        else {
            self.status_message = "DOE: select a response before using a target".to_string();
            return;
        };
        let Some(target) = response.target else {
            self.status_message = format!("DOE: {} has no target value", response.name);
            return;
        };
        let response_name = response.name.clone();
        self.experiment_capture_value = target;
        self.status_message = format!("DOE: loaded target for {response_name}");
    }

    pub fn show_inspector(&self) -> bool {
        self.show_inspector
    }

    pub fn show_layers(&self) -> bool {
        self.show_layers
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn set_active_view(&mut self, view: StartupView) {
        self.active_view = view;
        self.active_menu = None;
        self.status_message = format!("Switched to {}", view.label());
    }

    pub fn apply_clicked_node_name(&mut self, name: &str) -> bool {
        if let Some(slug) = name.strip_prefix("fabricad.menu.") {
            if let Some(menu) = AppMenu::from_slug(slug) {
                self.active_menu = (self.active_menu != Some(menu)).then_some(menu);
                self.status_message = format!("{} menu", menu.label());
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("fabricad.nav.action.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("fabricad.tool.") {
            if let Some(tool) = ToolMode::from_slug(slug) {
                self.active_tool = tool;
                self.status_message = format!("{} tool selected", tool.label());
                return true;
            }
        }

        if let Some(slug) = name.strip_prefix("fabricad.drawer.") {
            match slug {
                "inspector" => {
                    self.show_inspector = !self.show_inspector;
                    self.status_message = format!(
                        "Inspector {}",
                        if self.show_inspector {
                            "shown"
                        } else {
                            "hidden"
                        }
                    );
                    return true;
                }
                "layers" => {
                    self.show_layers = !self.show_layers;
                    self.status_message = format!(
                        "{} {}",
                        self.secondary_panel_label(),
                        if self.show_layers { "shown" } else { "hidden" }
                    );
                    return true;
                }
                _ => {}
            }
        }

        if let Some(raw) = name.strip_prefix("fabricad.primary.action.") {
            if let Some((_, action)) = raw.split_once('.') {
                return self.apply_view_control_action(action);
            }
        }

        if let Some(action) = name.strip_prefix("fabricad.viewctl.") {
            return self.apply_view_control_action(action);
        }

        if let Some(slug) = name.strip_prefix("fabricad.menu.item.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(action) = name.strip_prefix("fabricad.menu.item.") {
            self.apply_menu_action(action);
            return true;
        }

        false
    }

    fn trace_selection_exists(&self, selection: &TraceSelection) -> bool {
        match selection {
            TraceSelection::Lot(lot_id) => self.workspace.genealogy.lots.contains_key(lot_id),
            TraceSelection::Wafer(wafer) => self.workspace.genealogy.wafer(wafer).is_some(),
            TraceSelection::Process(sequence) => self
                .workspace
                .genealogy
                .process_history
                .iter()
                .any(|record| record.sequence == *sequence),
            TraceSelection::MaterialUse(sequence) => self
                .workspace
                .genealogy
                .material_uses
                .iter()
                .any(|record| record.sequence == *sequence),
            TraceSelection::Event(sequence) => self
                .workspace
                .genealogy
                .events
                .iter()
                .any(|event| event.sequence == *sequence),
        }
    }

    fn select_trace_detail(&mut self, selection: TraceSelection) -> bool {
        if !self.trace_selection_exists(&selection) {
            return false;
        }
        match &selection {
            TraceSelection::Lot(lot_id) => {
                self.selected_trace_lot = Some(lot_id.clone());
                let wafer_still_in_lot =
                    self.selected_trace_wafer.as_ref().is_some_and(|wafer_id| {
                        self.workspace
                            .genealogy
                            .wafer_refs_for_lot(lot_id)
                            .iter()
                            .any(|wafer| &wafer.wafer_id == wafer_id)
                    });
                if !wafer_still_in_lot {
                    self.selected_trace_wafer = default_trace_wafer(&self.workspace, Some(lot_id));
                }
                self.status_message = format!("Traceability lot {lot_id}");
            }
            TraceSelection::Wafer(wafer) => {
                self.selected_trace_lot = Some(wafer.lot_id.clone());
                self.selected_trace_wafer = Some(wafer.wafer_id.clone());
                self.status_message = format!("Traceability wafer {}", wafer.wafer_id);
            }
            TraceSelection::Process(sequence) => {
                if let Some(wafer) = self
                    .workspace
                    .genealogy
                    .process_history
                    .iter()
                    .find(|record| record.sequence == *sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_trace_lot = Some(wafer.lot_id);
                    self.selected_trace_wafer = Some(wafer.wafer_id);
                }
                self.status_message = format!("Traceability process record #{sequence}");
            }
            TraceSelection::MaterialUse(sequence) => {
                if let Some(wafer) = self
                    .workspace
                    .genealogy
                    .material_uses
                    .iter()
                    .find(|record| record.sequence == *sequence)
                    .map(|record| record.wafer.clone())
                {
                    self.selected_trace_lot = Some(wafer.lot_id);
                    self.selected_trace_wafer = Some(wafer.wafer_id);
                }
                self.status_message = format!("Traceability material use #{sequence}");
            }
            TraceSelection::Event(sequence) => {
                self.status_message = format!("Traceability audit event #{sequence}");
            }
        }
        self.selected_trace_detail = Some(selection);
        true
    }

    fn apply_view_control_action(&mut self, action: &str) -> bool {
        if action.starts_with("experiment.") {
            self.ensure_experiment_selection();
        }

        if let Some(slug) = action.strip_prefix("layout.view.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(layer_id) = action.strip_prefix("layout.layer.") {
            return self.select_layout_layer(layer_id);
        }

        if let Some(layer_id) = action.strip_prefix("layout.toggle_layer.") {
            return self.toggle_layout_layer_visibility(layer_id);
        }

        if let Some(shape_id) = action.strip_prefix("layout.shape.") {
            return self.select_layout_shape(shape_id);
        }

        if let Some(slug) = action.strip_prefix("workflow.open.") {
            if let Some(view) = StartupView::from_slug(slug) {
                self.set_active_view(view);
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("workflow.focus_lot.") {
            if workflow_lot_ids(&self.workspace)
                .iter()
                .any(|id| id == lot_id)
            {
                self.workflow_focus_lot = Some(lot_id.to_string());
                self.status_message = format!("Workflow focus lot {lot_id}");
                return true;
            }
        }

        if action == "workflow.load_demo" {
            self.replace_workspace(WorkspaceDataset::demo(), "Full demo workspace loaded");
            return true;
        }

        if let Some(tool_id) = action.strip_prefix("fab.select.") {
            return self.select_equipment_tool(tool_id);
        }

        if let Some(raw) = action.strip_prefix("fab.recipe.") {
            let mut parts = raw.split('|');
            let Some(tool_id) = parts.next() else {
                return false;
            };
            let Some(recipe_id) = parts.next() else {
                return false;
            };
            return self.load_equipment_recipe(tool_id, recipe_id);
        }

        if let Some(raw) = action.strip_prefix("fab.command.") {
            return self.apply_equipment_command(raw);
        }

        if let Some(slug) = action.strip_prefix("maintenance.filter.") {
            if let Some(filter) = MaintenanceWorkFilter::from_slug(slug) {
                self.maintenance_work_filter = filter;
                self.status_message = format!("Maintenance filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("maintenance.history.") {
            if let Some(filter) = MaintenanceHistoryFilter::from_slug(slug) {
                self.maintenance_history_filter = filter;
                self.status_message = format!("Maintenance history {}", filter.label());
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("maintenance.tool.") {
            let tool_id = EquipmentToolId::new(tool_id);
            if self.workspace.maintenance.tool(&tool_id).is_some() {
                self.selected_maintenance_tool = Some(tool_id.clone());
                self.status_message = format!("Selected maintenance tool {tool_id}");
                return true;
            }
        }

        if let Some(raw) = action.strip_prefix("maintenance.status.") {
            let Some((kind, tool_id)) = raw.split_once('|') else {
                return false;
            };
            let tool_id = EquipmentToolId::new(tool_id);
            if self.workspace.maintenance.tool(&tool_id).is_none() {
                return false;
            }
            self.status_message = match kind {
                "schedule" => format!("Maintenance scheduling opened for {tool_id}"),
                "calibration" => format!("Calibration capture opened for {tool_id}"),
                "release" => format!("Release hold review opened for {tool_id}"),
                _ => return false,
            };
            return true;
        }

        if let Some(sensor_id) = action.strip_prefix("environment.sensor.") {
            if self
                .workspace
                .environment
                .sensors
                .iter()
                .any(|sensor| sensor.id == sensor_id)
            {
                self.selected_environment_sensor = Some(sensor_id.to_string());
                self.status_message = format!("Selected environment sensor {sensor_id}");
                return true;
            }
        }

        if let Some(zone) = action.strip_prefix("environment.zone.") {
            if let Some(sensor) = self
                .workspace
                .environment
                .sensors
                .iter()
                .find(|sensor| sensor.zone == zone)
            {
                self.selected_environment_sensor = Some(sensor.id.clone());
                self.status_message = format!("Environment zone {zone}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("inventory.filter.") {
            if let Some(filter) = InventoryQuickFilter::from_slug(slug) {
                self.inventory_filter = filter;
                self.selected_inventory_lot = default_inventory_lot(&self.workspace, filter);
                self.status_message = format!("Inventory filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("inventory.lot.") {
            let lot_id = MaterialLotId::new(lot_id);
            if self.workspace.inventory.lot(&lot_id).is_some() {
                self.selected_inventory_lot = Some(lot_id.clone());
                self.status_message = format!("Selected inventory lot {lot_id}");
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("mask.lot.") {
            let lot_id = LotId::new(lot_id);
            if self.workspace.mes.lots.contains_key(&lot_id) {
                self.mask_source_lot = Some(lot_id.clone());
                self.mask_issue_page = 0;
                self.status_message = format!("Reticle prep lot {lot_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("mask.severity.") {
            if let Some(filter) = MaskIssueSeverityFilter::from_slug(slug) {
                self.mask_issue_severity_filter = filter;
                self.mask_issue_page = 0;
                self.status_message = format!("Reticle issues {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("mask.group.") {
            if let Some(grouping) = MaskIssueGrouping::from_slug(slug) {
                self.mask_issue_grouping = grouping;
                self.status_message = format!("Reticle issues grouped by {}", grouping.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.baseline.") {
            if let Some(source) = LayoutDiffSource::from_slug(slug) {
                self.layout_diff_baseline = source;
                self.reset_layout_diff_review();
                self.status_message = format!("Diff baseline {}", source.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.candidate.") {
            if let Some(source) = LayoutDiffSource::from_slug(slug) {
                self.layout_diff_candidate = source;
                self.reset_layout_diff_review();
                self.status_message = format!("Diff candidate {}", source.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.filter.") {
            if let Some(filter) = LayoutChangeFilter::from_slug(slug) {
                self.layout_diff_change_filter = filter;
                self.layout_diff_change_page = 0;
                self.status_message = format!("Diff filter {}", filter.detail_label());
                return true;
            }
        }

        if let Some(value) = action.strip_prefix("layout_diff.page_size.") {
            if let Ok(page_size) = value.parse::<usize>()
                && LAYOUT_DIFF_PAGE_SIZE_OPTIONS.contains(&page_size)
            {
                self.layout_diff_page_size = page_size;
                self.layout_diff_change_page = 0;
                self.status_message = format!("Diff rows per page {page_size}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("layout_diff.review.") {
            if let Some(state) = LayoutReviewDisposition::from_slug(slug) {
                self.layout_diff_review_state = state;
                self.status_message = format!("Diff review {}", state.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("trace.lot.") {
            let lot_id = LotId::new(lot_id);
            return self.select_trace_detail(TraceSelection::Lot(lot_id));
        }

        if let Some(raw) = action.strip_prefix("trace.wafer.") {
            let Some((lot_id, wafer_id)) = raw.split_once('|') else {
                return false;
            };
            let lot_id = LotId::new(lot_id);
            let wafer_id = WaferId::new(wafer_id);
            let wafer = WaferRef {
                lot_id: lot_id.clone(),
                wafer_id: wafer_id.clone(),
            };
            return self.select_trace_detail(TraceSelection::Wafer(wafer));
        }

        if let Some(raw) = action.strip_prefix("trace.detail.") {
            if let Some(selection) = trace_selection_from_view_action(raw) {
                return self.select_trace_detail(selection);
            }
        }

        if let Some(slug) = action.strip_prefix("trace.impact.") {
            if let Some(mode) = TraceImpactMode::from_slug(slug) {
                self.trace_impact_mode = mode;
                self.status_message = format!("Traceability impact {}", mode.label());
                return true;
            }
        }

        if let Some(entry_id) = action.strip_prefix("notebook.entry.") {
            let entry_id = NotebookEntryId::new(entry_id);
            if self.workspace.lab_notebook.entry(&entry_id).is_some() {
                self.selected_notebook_entry = Some(entry_id.clone());
                self.status_message = format!("Notebook entry {entry_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("notebook.entry_action.") {
            if let Some(entry_action) = NotebookEntryAction::from_slug(slug) {
                return self.apply_notebook_entry_action(entry_action);
            }
        }

        if let Some(token) = action.strip_prefix("notebook.focus_link.") {
            let Some((kind_slug, query)) = token.split_once(':') else {
                return false;
            };
            if let Some(kind) = notebook_link_kind_from_slug(kind_slug) {
                return self.focus_notebook_link(kind, query);
            }
        }

        if let Some(tag) = action.strip_prefix("notebook.tag.") {
            if tag == "all" {
                self.notebook_tag_filter = None;
                self.ensure_notebook_selection();
                self.status_message = "Notebook tag filter all".to_string();
                return true;
            }
            if self
                .workspace
                .lab_notebook
                .tags()
                .iter()
                .any(|candidate| candidate == tag)
            {
                self.notebook_tag_filter = Some(tag.to_string());
                self.ensure_notebook_selection();
                self.status_message = format!("Notebook tag {tag}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("notebook.link.") {
            self.notebook_link_kind_filter = if slug == "any" {
                None
            } else {
                let Some(kind) = notebook_link_kind_from_slug(slug) else {
                    return false;
                };
                Some(kind)
            };
            self.ensure_notebook_selection();
            self.status_message = format!(
                "Notebook link {}",
                self.notebook_link_kind_filter
                    .map(|kind| kind.label())
                    .unwrap_or("any")
            );
            return true;
        }

        if let Some(response_id) = action.strip_prefix("experiment.response.") {
            let response_id = ResponseSpecId::new(response_id);
            if self
                .workspace
                .experiment_plan
                .response(&response_id)
                .is_some()
            {
                self.selected_experiment_response = Some(response_id.clone());
                self.status_message = format!("DOE primary response set to {response_id}");
                return true;
            }
        }

        if let Some(run_id) = action.strip_prefix("experiment.run.") {
            let run_id = ExperimentRunId::new(run_id);
            if self.workspace.experiment_plan.run(&run_id).is_some() {
                self.selected_experiment_run = Some(run_id.clone());
                self.status_message = format!("DOE run selected: {run_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("experiment.filter.") {
            if let Some(filter) = ExperimentRunFilter::from_slug(slug) {
                self.experiment_run_filter = filter;
                self.status_message = format!("DOE run filter set to {}", filter.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("experiment.lot.") {
            if self
                .experiment_lot_options()
                .iter()
                .any(|candidate| candidate == lot_id)
            {
                if self.experiment_lot_filter.as_deref() == Some(lot_id) {
                    self.experiment_lot_filter = None;
                    self.status_message = "DOE lot filter cleared".to_string();
                } else {
                    self.experiment_lot_filter = Some(lot_id.to_string());
                    self.status_message = format!("DOE lot filter set to {lot_id}");
                }
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("scheduler.policy.") {
            if let Some(policy) = dispatch_policy_from_slug(slug) {
                self.scheduler_policy = policy;
                self.status_message = format!("Dispatch policy {}", policy.label());
                return true;
            }
        }

        if let Some(value) = action.strip_prefix("scheduler.priority.") {
            if let Ok(priority) = value.parse::<u8>()
                && priority <= 5
            {
                self.scheduler_min_priority = priority;
                self.status_message = format!("Minimum dispatch priority P{priority}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("scheduler.tool.") {
            if self
                .workspace
                .scheduler
                .tools
                .iter()
                .any(|tool| tool.id.as_str() == tool_id)
            {
                self.selected_scheduler_tool = Some(SchedulerToolId::new(tool_id));
                self.status_message = format!("Selected dispatch tool {tool_id}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("safety.tool.") {
            if self
                .workspace
                .safety
                .evaluate_lockouts()
                .iter()
                .any(|lockout| lockout.tool_id == tool_id)
            {
                self.selected_safety_tool = Some(tool_id.to_string());
                self.status_message = format!("Selected safety tool {tool_id}");
                return true;
            }
        }

        if let Some(sensor_id) = action.strip_prefix("safety.ack.condition.") {
            if self
                .workspace
                .safety
                .sensors
                .iter()
                .any(|sensor| sensor.id.to_string() == sensor_id)
            {
                self.acknowledged_conditions.insert(sensor_id.to_string());
                self.status_message = format!("Acknowledged safety condition {sensor_id}");
                return true;
            }
        }

        if let Some(tool_id) = action.strip_prefix("safety.ack.lockout.") {
            if self
                .workspace
                .safety
                .evaluate_lockouts()
                .iter()
                .any(|lockout| lockout.tool_id == tool_id)
            {
                self.acknowledged_lockouts.insert(tool_id.to_string());
                self.status_message = format!("Acknowledged safety lockout {tool_id}");
                return true;
            }
        }

        if let Some(incident_id) = action.strip_prefix("safety.ack.incident.") {
            if self
                .workspace
                .safety
                .incidents
                .iter()
                .any(|incident| incident.id.to_string() == incident_id)
            {
                self.acknowledged_incidents.insert(incident_id.to_string());
                self.status_message = format!("Acknowledged safety incident {incident_id}");
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("process_flow.filter.") {
            if let Some(filter) = ProcessFlowNodeFilter::from_slug(slug) {
                self.process_flow_filter = filter;
                self.selected_process_node =
                    default_process_flow_node_for_filter(&self.workspace, filter);
                self.status_message = format!("Process flow filter {}", filter.label());
                return true;
            }
        }

        if let Some(node_id) = action.strip_prefix("process_flow.node.") {
            let node_id = ProcessFlowNodeId::new(node_id);
            if self
                .workspace
                .process_flow
                .route
                .nodes
                .iter()
                .any(|node| node.id == node_id)
            {
                self.selected_process_node = Some(node_id.clone());
                self.status_message = format!("Selected process node {node_id}");
                return true;
            }
        }

        if let Some(loop_id) = action.strip_prefix("process_control.loop.") {
            return self.select_control_loop(loop_id);
        }

        if let Some(action_id) = action.strip_prefix("process_control.action.") {
            return self.select_control_action(action_id);
        }

        if let Some(raw) = action.strip_prefix("process_control.transition.") {
            let Some((action_id, transition)) = raw.split_once('|') else {
                return false;
            };
            return self.apply_process_control_transition(action_id, transition);
        }

        if let Some(slug) = action.strip_prefix("spc.severity.") {
            if let Some(filter) = SpcSeverityFilter::from_slug(slug) {
                self.spc_severity_filter = filter;
                self.status_message = format!("SPC/FDC severity {}", filter.detail_label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("spc.source.") {
            if let Some(filter) = SpcSourceFilter::from_slug(slug) {
                self.spc_source_filter = filter;
                self.status_message = format!("SPC/FDC source {}", filter.detail_label());
                return true;
            }
        }

        if let Some(chart_id) = action.strip_prefix("spc.chart.") {
            let monitor = spc_fdc_monitor(&self.workspace);
            if monitor
                .charts
                .iter()
                .any(|chart| chart.id.as_str() == chart_id)
            {
                self.selected_spc_chart = Some(chart_id.to_string());
                self.spc_context_filter = chart_id.to_string();
                self.status_message = format!("Selected SPC chart {chart_id}");
                return true;
            }
        }

        if let Some(trace_id) = action.strip_prefix("spc.trace.") {
            let monitor = spc_fdc_monitor(&self.workspace);
            if monitor.traces.iter().any(|trace| trace.id == trace_id) {
                self.selected_fdc_trace = Some(trace_id.to_string());
                self.spc_context_filter = trace_id.to_string();
                self.status_message = format!("Selected FDC trace {trace_id}");
                return true;
            }
        }

        if let Some(step) = action.strip_prefix("cross_section.step.") {
            if let Ok(step) = step.parse::<usize>() {
                let max_step = self.workspace.cross_section.steps.len();
                if step <= max_step {
                    self.cross_section_step = step;
                    self.status_message = format!("Cross-section step {step}");
                    return true;
                }
            }
        }

        if let Some(material_id) = action.strip_prefix("cross_section.material.") {
            let material_id = MaterialId::new(material_id);
            if self
                .workspace
                .cross_section
                .materials
                .iter()
                .any(|material| material.id == material_id)
            {
                self.selected_cross_section_material = Some(material_id.clone());
                self.status_message =
                    format!("Selected cross-section material {}", material_id.as_str());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("metrology.mode.") {
            if let Some(mode) = MetrologyMapMode::from_slug(slug) {
                self.metrology_map_mode = mode;
                self.status_message = format!("Metrology {}", mode.label());
                return true;
            }
        }

        if let Some(slug) = action.strip_prefix("metrology.kind.") {
            if let Some(kind) = measurement_kind_from_slug(slug) {
                self.metrology_kind = kind;
                self.status_message = format!("Metrology measurement {}", kind.label());
                return true;
            }
        }

        if let Some(raw) = action.strip_prefix("metrology.die.") {
            let Some((column, row)) = raw.split_once('|') else {
                return false;
            };
            if let (Ok(column), Ok(row)) = (column.parse::<i32>(), row.parse::<i32>()) {
                let die = DieCoord::new(column, row);
                if self
                    .workspace
                    .wafer_map
                    .dies
                    .iter()
                    .any(|entry| *entry == die)
                {
                    self.selected_die = Some(die);
                    self.status_message = format!("Metrology die {}", die_coord_label(Some(die)));
                    return true;
                }
            }
        }

        if let Some(slug) = action.strip_prefix("yield.filter.") {
            if let Some(filter) = YieldMapFilter::from_slug(slug) {
                self.yield_map_filter = filter;
                self.status_message = format!("Yield map filter {}", filter.label());
                return true;
            }
        }

        if let Some(lot_id) = action.strip_prefix("yield.lot.") {
            if self
                .workspace
                .yield_analysis
                .lots
                .iter()
                .any(|lot| lot.id == lot_id)
            {
                self.selected_yield_lot = Some(lot_id.to_string());
                self.selected_yield_wafer =
                    default_yield_wafer(&self.workspace, self.selected_yield_lot.as_deref());
                self.status_message = format!("Yield lot {lot_id}");
                return true;
            }
        }

        if let Some(wafer_id) = action.strip_prefix("yield.wafer.") {
            if let Some(lot_id) = self.selected_yield_lot.as_deref() {
                let wafer_ids = self.workspace.yield_analysis.wafer_ids_for_lot(lot_id);
                if wafer_ids.iter().any(|id| id == wafer_id) {
                    self.selected_yield_wafer = Some(wafer_id.to_string());
                    self.status_message = format!("Yield wafer {wafer_id}");
                    return true;
                }
            }
        }

        match action {
            "layout.add_layer" => self.add_layout_layer(),
            "layout.next_shape" => self.select_next_layout_shape(),
            "layout.clear_selection" => {
                self.selected_layout_shape = None;
                self.status_message = "Layout selection cleared".to_string();
                true
            }
            "layout.copy" => self.copy_layout_selection(),
            "layout.paste" => self.paste_layout_clipboard(),
            "layout.duplicate" => self.duplicate_layout_selection(),
            "layout.delete" => self.delete_layout_selection(),
            "layout.toggle_grid" => {
                self.show_grid = !self.show_grid;
                self.status_message = format!(
                    "2D grid {}",
                    if self.show_grid { "shown" } else { "hidden" }
                );
                true
            }
            "layout.toggle_snap" => {
                self.snap_enabled = !self.snap_enabled;
                self.status_message = format!(
                    "Snap {}",
                    if self.snap_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                true
            }
            "layout.toggle_drc" => {
                self.show_drc_overlay = !self.show_drc_overlay;
                self.status_message = format!(
                    "DRC overlay {}",
                    if self.show_drc_overlay {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "layout.run_drc" => {
                self.status_message = self.run_drc_summary();
                true
            }
            "layout.connectivity" => {
                self.status_message = self.connectivity_summary();
                true
            }
            "metrology.failed_only" => {
                self.metrology_failed_only = !self.metrology_failed_only;
                self.status_message = if self.metrology_failed_only {
                    "Metrology showing failed/outlier sites".to_string()
                } else {
                    "Metrology showing all sites".to_string()
                };
                true
            }
            "metrology.next_attention" => {
                self.selected_die = self.first_attention_die();
                self.status_message = self
                    .selected_die
                    .map(|die| format!("Selected attention die {}", die_coord_label(Some(die))))
                    .unwrap_or_else(|| "No attention die found".to_string());
                true
            }
            "metrology.clear_die" => {
                self.selected_die = None;
                self.status_message = "Metrology selection cleared".to_string();
                true
            }
            "yield.attention" => {
                self.show_only_attention_wafers = !self.show_only_attention_wafers;
                self.status_message = format!(
                    "{} attention wafers",
                    if self.show_only_attention_wafers {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "yield.excursions" => {
                self.show_only_excursions = !self.show_only_excursions;
                self.status_message = format!(
                    "{} yield excursions",
                    if self.show_only_excursions {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "experiment.pending_only" => {
                self.experiment_show_missing_only = !self.experiment_show_missing_only;
                self.status_message = format!(
                    "{} pending DOE runs",
                    if self.experiment_show_missing_only {
                        "Showing"
                    } else {
                        "Including"
                    }
                );
                true
            }
            "experiment.next_pending" => {
                if self.select_next_pending_experiment_response() {
                    let run = self.selected_experiment_run_label();
                    self.status_message = format!(
                        "DOE: queued {} for {run}",
                        self.selected_experiment_response_label()
                    );
                } else {
                    self.status_message = "DOE: no pending responses remain".to_string();
                }
                true
            }
            "experiment.capture" => self.capture_selected_experiment_response(),
            "experiment.capture_advance" => {
                if self.capture_selected_experiment_response()
                    && self.select_next_pending_experiment_response()
                {
                    self.status_message
                        .push_str("; queued next pending response");
                }
                true
            }
            "experiment.use_demo" => {
                self.use_demo_experiment_capture_value();
                true
            }
            "experiment.use_target" => {
                self.use_experiment_response_target();
                true
            }
            "experiment.capture_next_demo" => {
                self.capture_next_demo_experiment_response();
                true
            }
            "experiment.clear_filters" => {
                self.experiment_run_filter = ExperimentRunFilter::All;
                self.experiment_lot_filter = None;
                self.experiment_show_missing_only = false;
                self.status_message = "DOE filters cleared".to_string();
                true
            }
            "scheduler.toggle_conflicts" => {
                self.scheduler_conflicts_only = !self.scheduler_conflicts_only;
                self.status_message = "Dispatch conflict filter toggled".to_string();
                true
            }
            "scheduler.toggle_focus_tool" => {
                self.scheduler_focus_selected_tool = !self.scheduler_focus_selected_tool;
                self.status_message = "Dispatch selected-tool filter toggled".to_string();
                true
            }
            "scheduler.reset" => {
                self.scheduler_min_priority = 0;
                self.scheduler_conflicts_only = false;
                self.scheduler_focus_selected_tool = false;
                self.status_message = "Dispatch filters reset".to_string();
                true
            }
            "mask.rebuild" => {
                self.mask_issue_page = 0;
                let report = mask_check_report(self);
                self.status_message = format!(
                    "Reticle prep rebuilt: {} issue(s)",
                    report.total_issue_count()
                );
                true
            }
            "mask.prev_page" => {
                self.mask_issue_page = self.mask_issue_page.saturating_sub(1);
                self.status_message = format!("Reticle issue page {}", self.mask_issue_page + 1);
                true
            }
            "mask.next_page" => {
                let report = mask_check_report(self);
                let page_count = mask_issue_page_count(self, &report);
                if self.mask_issue_page + 1 < page_count {
                    self.mask_issue_page += 1;
                }
                self.status_message = format!("Reticle issue page {}", self.mask_issue_page + 1);
                true
            }
            "mask.clear_filters" => {
                self.mask_issue_severity_filter = MaskIssueSeverityFilter::All;
                self.mask_issue_grouping = MaskIssueGrouping::Code;
                self.mask_issue_page = 0;
                self.status_message = "Reticle issue filters cleared".to_string();
                true
            }
            "layout_diff.swap" => {
                std::mem::swap(
                    &mut self.layout_diff_baseline,
                    &mut self.layout_diff_candidate,
                );
                self.reset_layout_diff_review();
                self.status_message = "Diff sources swapped".to_string();
                true
            }
            "layout_diff.demo_current" => {
                self.layout_diff_baseline = LayoutDiffSource::Demo;
                self.layout_diff_candidate = LayoutDiffSource::Current;
                self.reset_layout_diff_review();
                self.status_message = "Diff set to demo -> current".to_string();
                true
            }
            "layout_diff.empty_current" => {
                self.layout_diff_baseline = LayoutDiffSource::Empty;
                self.layout_diff_candidate = LayoutDiffSource::Current;
                self.reset_layout_diff_review();
                self.status_message = "Diff set to empty -> current".to_string();
                true
            }
            "layout_diff.toggle_changed_only" => {
                self.layout_diff_changed_only = !self.layout_diff_changed_only;
                self.status_message = format!(
                    "Diff changed layers {}",
                    if self.layout_diff_changed_only {
                        "only"
                    } else {
                        "and unchanged"
                    }
                );
                true
            }
            "layout_diff.prev_page" => {
                self.layout_diff_change_page = self.layout_diff_change_page.saturating_sub(1);
                self.status_message =
                    format!("Diff change page {}", self.layout_diff_change_page + 1);
                true
            }
            "layout_diff.next_page" => {
                let report = layout_diff_report(self);
                let page_count = layout_diff_page_count(self, &report);
                if self.layout_diff_change_page + 1 < page_count {
                    self.layout_diff_change_page += 1;
                }
                self.status_message =
                    format!("Diff change page {}", self.layout_diff_change_page + 1);
                true
            }
            "layout_diff.reset_filters" => {
                self.layout_diff_change_filter = LayoutChangeFilter::All;
                self.layout_diff_page_size = LAYOUT_DIFF_DEFAULT_PAGE_SIZE;
                self.layout_diff_change_page = 0;
                self.status_message = "Diff filters reset".to_string();
                true
            }
            "trace.toggle_related" => {
                self.trace_related_only = !self.trace_related_only;
                self.status_message = "Traceability related-only filter toggled".to_string();
                true
            }
            "notebook.toggle_followups" => {
                self.notebook_followups_only = !self.notebook_followups_only;
                self.ensure_notebook_selection();
                self.status_message = "Notebook follow-up filter toggled".to_string();
                true
            }
            "notebook.preview" => {
                self.notebook_preview_mode = true;
                self.status_message = "Notebook preview mode".to_string();
                true
            }
            "notebook.edit" => {
                self.notebook_preview_mode = false;
                self.status_message = "Notebook edit mode".to_string();
                true
            }
            "notebook.clear_filters" => {
                self.notebook_tag_filter = None;
                self.notebook_link_kind_filter = None;
                self.notebook_followups_only = false;
                self.ensure_notebook_selection();
                self.status_message = "Notebook filters cleared".to_string();
                true
            }
            "process_flow.toggle_errors" => {
                self.process_flow_errors_only = !self.process_flow_errors_only;
                self.status_message = if self.process_flow_errors_only {
                    "Process flow showing errors only".to_string()
                } else {
                    "Process flow showing all findings".to_string()
                };
                true
            }
            "process_flow.validate" => {
                let findings = self.workspace.process_flow.findings();
                self.status_message = format!("Process flow has {} finding(s)", findings.len());
                true
            }
            "process_flow.export" => {
                let route = self.workspace.process_flow.export_to_mes_route();
                self.status_message = format!(
                    "Prepared {} MES step(s) for {}",
                    route.steps.len(),
                    route.id
                );
                true
            }
            "spc.clear_context" => {
                self.spc_context_filter.clear();
                self.status_message = "SPC/FDC context filter cleared".to_string();
                true
            }
            "cross_section.toggle_mask" => {
                self.cross_section_show_mask = !self.cross_section_show_mask;
                self.status_message = format!(
                    "Cross-section mask {}",
                    if self.cross_section_show_mask {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "cross_section.toggle_dimensions" => {
                self.cross_section_show_dimensions = !self.cross_section_show_dimensions;
                self.status_message = format!(
                    "Cross-section dimensions {}",
                    if self.cross_section_show_dimensions {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            "cross_section.toggle_risks" => {
                self.cross_section_show_risks = !self.cross_section_show_risks;
                self.status_message = format!(
                    "Cross-section risks {}",
                    if self.cross_section_show_risks {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
                true
            }
            _ => false,
        }
    }

    fn apply_menu_action(&mut self, action: &str) {
        if let Some(slug) = action.strip_prefix("tool.") {
            if let Some(tool) = ToolMode::from_slug(slug) {
                self.active_tool = tool;
                self.status_message = format!("{} tool selected", tool.label());
                self.active_menu = None;
                return;
            }
        }
        match action {
            "file.new_blank" => {
                self.replace_workspace(WorkspaceDataset::blank(), "New blank workspace loaded");
            }
            "file.load_demo" | "file.new_demo" => {
                self.replace_workspace(WorkspaceDataset::demo(), "Demo workspace loaded");
            }
            "file.save" => {
                self.status_message = "Save is not wired in this Operad shell yet".to_string()
            }
            "file.load" => {
                self.status_message = "Load workspace is not wired in this shell yet".to_string()
            }
            "file.save_layout" => {
                self.status_message = "Save layout JSON is not wired in this shell yet".to_string()
            }
            "file.load_layout" => {
                self.status_message = "Load layout JSON is not wired in this shell yet".to_string()
            }
            "file.export_gds" => {
                self.status_message = "Use --export-gds PATH for GDSII export".to_string()
            }
            "file.import_gds" => {
                self.status_message = "GDS import is not wired in this shell yet".to_string()
            }
            "file.collaboration" => {
                self.status_message = "Collaboration connects through the sync server".to_string()
            }
            "edit.undo" => self.status_message = "Undo stack is not wired yet".to_string(),
            "edit.redo" => self.status_message = "Redo stack is not wired yet".to_string(),
            "edit.copy" => {
                self.copy_layout_selection();
            }
            "edit.paste" => {
                self.paste_layout_clipboard();
            }
            "edit.duplicate" => {
                self.duplicate_layout_selection();
            }
            "edit.delete" => {
                self.delete_layout_selection();
            }
            "edit.make_cell" => self.status_message = "Make cell is not wired yet".to_string(),
            "edit.rotate90" => self.status_message = "Rotate 90 is not wired yet".to_string(),
            "edit.mirror_x" => self.status_message = "Mirror X is not wired yet".to_string(),
            "edit.mirror_y" => self.status_message = "Mirror Y is not wired yet".to_string(),
            "bookmarks.origin" => self.status_message = "Focused origin".to_string(),
            "bookmarks.bounds" => self.status_message = "Focused layout bounds".to_string(),
            "display.grid2d" | "options.grid" => {
                self.show_grid = !self.show_grid;
                self.status_message = format!(
                    "2D grid {}",
                    if self.show_grid { "shown" } else { "hidden" }
                );
            }
            "display.grid3d" => {
                self.show_3d_grid = !self.show_3d_grid;
                self.status_message = format!(
                    "3D grid {}",
                    if self.show_3d_grid { "shown" } else { "hidden" }
                );
            }
            "display.origin" => {
                self.show_origin_marker = !self.show_origin_marker;
                self.status_message = format!(
                    "Origin crosshair {}",
                    if self.show_origin_marker {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            "display.drc" => {
                self.show_drc_overlay = !self.show_drc_overlay;
                self.status_message = format!(
                    "DRC overlay {}",
                    if self.show_drc_overlay {
                        "shown"
                    } else {
                        "hidden"
                    }
                );
            }
            "display.theme" | "options.theme" => {
                self.dark_theme = !self.dark_theme;
                self.status_message =
                    format!("{} theme", if self.dark_theme { "Dark" } else { "Light" });
            }
            "display.units" => self.status_message = "Unit display cycled".to_string(),
            "options.snap" => {
                self.snap_enabled = !self.snap_enabled;
                self.status_message = format!(
                    "Snap {}",
                    if self.snap_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            "tools.palette" => self.status_message = "Command palette is not wired yet".to_string(),
            "tools.diagnostics" => self.status_message = "Diagnostics requested".to_string(),
            "tools.rundrc" => self.status_message = self.run_drc_summary(),
            "tools.reset3d" => self.status_message = "3D camera reset requested".to_string(),
            "tools.fullscreen" => self.status_message = "Viewport fullscreen toggled".to_string(),
            "macros.validate" => self.status_message = "Workspace validation clean".to_string(),
            "macros.demo" => {
                self.replace_workspace(WorkspaceDataset::demo(), "Full demo workspace loaded");
            }
            "macros.stress10k" => {
                self.workspace.document = Document::stress(10_000);
                self.reset_layout_document_state();
                self.status_message = "10k stress scene loaded".to_string();
            }
            "macros.stress100k" => {
                self.workspace.document = Document::stress(100_000);
                self.reset_layout_document_state();
                self.status_message = "100k stress scene loaded".to_string();
            }
            "macros.stress1m" => {
                self.workspace.document = Document::stress(1_000_000);
                self.reset_layout_document_state();
                self.status_message = "1M stress scene loaded".to_string();
            }
            "macros.hierarchy" => {
                self.workspace.document = Document::hierarchy_demo();
                self.reset_layout_document_state();
                self.status_message = "Hierarchy scene loaded".to_string();
            }
            "macros.drc" => {
                self.status_message = self.run_drc_summary();
            }
            "macros.snapshot" => {
                self.status_message = "Use --operad-snapshot to render a snapshot".to_string()
            }
            "help.shortcuts" => {
                self.status_message =
                    "Shortcuts: click menus or left nav to switch views".to_string()
            }
            "help.about" => self.status_message = "Fabricad on Operad v4.0.0".to_string(),
            _ => self.status_message = action.replace('.', " "),
        }
        self.active_menu = None;
    }

    fn replace_workspace(
        &mut self,
        workspace: WorkspaceDataset,
        status_message: impl Into<String>,
    ) {
        self.workspace = workspace;
        self.reset_layout_document_state();
        self.workflow_focus_lot = default_workflow_focus_lot(&self.workspace);
        self.selected_equipment_tool = default_equipment_tool(&self.workspace);
        self.equipment_recipe_drafts.clear();
        self.selected_maintenance_tool = default_maintenance_tool(&self.workspace);
        self.maintenance_work_filter = MaintenanceWorkFilter::Actionable;
        self.maintenance_history_filter = MaintenanceHistoryFilter::SelectedTool;
        self.selected_environment_sensor = default_environment_sensor(&self.workspace);
        self.selected_inventory_lot =
            default_inventory_lot(&self.workspace, InventoryQuickFilter::All);
        self.inventory_filter = InventoryQuickFilter::All;
        self.mask_source_lot = default_mask_lot(&self.workspace);
        self.mask_issue_severity_filter = MaskIssueSeverityFilter::All;
        self.mask_issue_grouping = MaskIssueGrouping::Code;
        self.mask_issue_page = 0;
        self.layout_diff_baseline = LayoutDiffSource::Demo;
        self.layout_diff_candidate = LayoutDiffSource::Current;
        self.layout_diff_changed_only = true;
        self.layout_diff_change_filter = LayoutChangeFilter::All;
        self.layout_diff_change_page = 0;
        self.layout_diff_page_size = LAYOUT_DIFF_DEFAULT_PAGE_SIZE;
        self.layout_diff_review_state = LayoutReviewDisposition::NeedsReview;
        self.selected_trace_lot = default_trace_lot(&self.workspace);
        self.selected_trace_wafer =
            default_trace_wafer(&self.workspace, self.selected_trace_lot.as_ref());
        self.selected_trace_detail = default_trace_selection(
            &self.workspace,
            self.selected_trace_lot.as_ref(),
            self.selected_trace_wafer.as_ref(),
        );
        self.trace_impact_mode = TraceImpactMode::ToolRun;
        self.trace_related_only = false;
        self.selected_notebook_entry = default_notebook_entry(&self.workspace);
        self.notebook_tag_filter = None;
        self.notebook_link_kind_filter = None;
        self.notebook_preview_mode = true;
        self.notebook_followups_only = false;
        self.scheduler_policy = DispatchPolicy::PriorityThenFifo;
        self.selected_scheduler_tool = default_scheduler_tool(&self.workspace);
        self.scheduler_min_priority = 0;
        self.scheduler_conflicts_only = false;
        self.scheduler_focus_selected_tool = false;
        self.selected_safety_tool = default_safety_tool(&self.workspace);
        self.acknowledged_conditions.clear();
        self.acknowledged_lockouts.clear();
        self.acknowledged_incidents.clear();
        self.selected_process_node = default_process_flow_node(&self.workspace);
        self.process_flow_filter = ProcessFlowNodeFilter::All;
        self.process_flow_errors_only = false;
        self.selected_control_loop = default_control_loop(&self.workspace);
        self.selected_control_action =
            default_control_action(&self.workspace, self.selected_control_loop.as_ref());
        self.selected_spc_chart = default_spc_chart(&self.workspace);
        self.selected_fdc_trace = default_fdc_trace(&self.workspace);
        self.spc_severity_filter = SpcSeverityFilter::All;
        self.spc_source_filter = SpcSourceFilter::All;
        self.spc_context_filter.clear();
        self.cross_section_step = 0;
        self.selected_cross_section_material = default_cross_section_material(&self.workspace);
        self.cross_section_show_mask = true;
        self.cross_section_show_dimensions = true;
        self.cross_section_show_risks = true;
        self.selected_die = None;
        self.selected_yield_lot = default_yield_lot(&self.workspace);
        self.selected_yield_wafer =
            default_yield_wafer(&self.workspace, self.selected_yield_lot.as_deref());
        self.status_message = status_message.into();
    }

    fn select_equipment_tool(&mut self, tool_id: &str) -> bool {
        if self
            .workspace
            .equipment
            .tools()
            .any(|tool| tool.id.as_str() == tool_id)
        {
            self.selected_equipment_tool = Some(EquipmentToolId::new(tool_id));
            self.status_message = format!("Selected tool {tool_id}");
            return true;
        }
        false
    }

    fn load_equipment_recipe(&mut self, tool_id: &str, recipe_id: &str) -> bool {
        let tool_id = EquipmentToolId::new(tool_id);
        let recipe_id = EquipmentRecipeId::new(recipe_id);
        let Some(tool) = self.workspace.equipment.tool(&tool_id) else {
            return false;
        };
        if !tool.available_recipes.contains_key(&recipe_id) {
            return false;
        }
        self.selected_equipment_tool = Some(tool_id.clone());
        self.equipment_recipe_drafts
            .insert(tool_id.clone(), recipe_id.clone());
        if !tool.state.accepts_recipe_load() {
            self.status_message = format!("Selected recipe {recipe_id}; tool is not loadable");
            return true;
        }
        let selection = self.equipment_selection_for_tool(tool, recipe_id);
        self.send_equipment_command(&tool_id, HostCommand::LoadRecipe { selection })
    }

    fn apply_equipment_command(&mut self, raw: &str) -> bool {
        let mut parts = raw.split('|');
        let Some(command) = parts.next() else {
            return false;
        };
        let Some(tool_id) = parts.next() else {
            return false;
        };
        let tool_id = EquipmentToolId::new(tool_id);
        let Some(tool) = self.workspace.equipment.tool(&tool_id) else {
            return false;
        };
        self.selected_equipment_tool = Some(tool_id.clone());
        let host_command = match command {
            "online" => HostCommand::BringOnline,
            "load" => {
                let Some(recipe_id) = self.selected_recipe_for_tool(tool) else {
                    self.status_message = format!("{tool_id} has no recipe to load");
                    return true;
                };
                HostCommand::LoadRecipe {
                    selection: self.equipment_selection_for_tool(tool, recipe_id),
                }
            }
            "start" => HostCommand::Start,
            "stop" => HostCommand::Stop,
            "alarm" => HostCommand::TriggerAlarm {
                code: "HOST-SIM".to_string(),
                message: "operator injected simulator alarm".to_string(),
                severity: AlarmSeverity::Warning,
            },
            "clear" => HostCommand::ClearAlarm,
            "reset" => HostCommand::Reset,
            "maintenance" => {
                if tool.state == EquipmentToolState::Maintenance {
                    HostCommand::ExitMaintenance
                } else {
                    HostCommand::EnterMaintenance
                }
            }
            _ => return false,
        };
        self.send_equipment_command(&tool_id, host_command)
    }

    fn send_equipment_command(&mut self, tool_id: &EquipmentToolId, command: HostCommand) -> bool {
        let label = command.label();
        match self.workspace.equipment.command(tool_id, command) {
            Ok(events) => {
                let state = self
                    .workspace
                    .equipment
                    .tool(tool_id)
                    .map(|tool| tool.state.label())
                    .unwrap_or("unknown");
                let major_events = events
                    .iter()
                    .filter(|event| {
                        !matches!(
                            event,
                            layout_model::equipment::EquipmentEvent::SensorSample { .. }
                        )
                    })
                    .count();
                self.status_message =
                    format!("{tool_id} {label}: {state}, {major_events} event(s)");
                true
            }
            Err(err) => {
                self.status_message = err.to_string();
                true
            }
        }
    }

    fn select_control_loop(&mut self, loop_id: &str) -> bool {
        let loop_id = ControlLoopId::new(loop_id);
        if self
            .workspace
            .process_control
            .loop_by_id(&loop_id)
            .is_some()
        {
            self.selected_control_loop = Some(loop_id.clone());
            self.selected_control_action = default_control_action(&self.workspace, Some(&loop_id));
            self.status_message = format!("Process-control loop {loop_id}");
            return true;
        }
        false
    }

    fn select_control_action(&mut self, action_id: &str) -> bool {
        let action_id = ControlActionId::new(action_id);
        let Some(loop_id) = self
            .workspace
            .process_control
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .map(|action| action.loop_id.clone())
        else {
            return false;
        };
        self.selected_control_loop = Some(loop_id);
        self.selected_control_action = Some(action_id.clone());
        self.status_message = format!("Process-control action {action_id}");
        true
    }

    fn apply_process_control_transition(&mut self, action_id: &str, transition: &str) -> bool {
        let action_id = ControlActionId::new(action_id);
        let actor = "process.engineer";
        let timestamp = "2026-05-12T12:00:00Z";
        let result = match transition {
            "approve" => self.workspace.process_control.approve_action(
                &action_id,
                actor,
                timestamp,
                "approved from Operad controls",
            ),
            "reject" => self.workspace.process_control.reject_action(
                &action_id,
                actor,
                timestamp,
                "rejected from Operad controls",
            ),
            "apply" => self.workspace.process_control.apply_action(
                &action_id,
                actor,
                timestamp,
                "applied from Operad controls",
            ),
            _ => return false,
        };
        match result {
            Ok(()) => {
                self.selected_control_action = Some(action_id.clone());
                if let Some(loop_id) = self
                    .workspace
                    .process_control
                    .actions
                    .iter()
                    .find(|action| action.id == action_id)
                    .map(|action| action.loop_id.clone())
                {
                    self.selected_control_loop = Some(loop_id);
                }
                self.status_message = format!("Process-control {transition} {action_id}");
            }
            Err(error) => {
                self.status_message = format!("Process-control blocked: {error}");
            }
        }
        true
    }

    fn select_layout_layer(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u32>() else {
            return false;
        };
        let layer_id = LayerId(id);
        let Some(layer) = self.workspace.document.layers.get(&layer_id) else {
            return false;
        };
        self.active_layer = layer_id;
        self.status_message = format!("Active layer L{} {}", layer.id.0, layer.name);
        true
    }

    fn toggle_layout_layer_visibility(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u32>() else {
            return false;
        };
        let layer_id = LayerId(id);
        let Some(layer) = self.workspace.document.layers.get(&layer_id) else {
            return false;
        };
        let visible = !layer.visible;
        let name = layer.name.clone();
        self.workspace
            .document
            .apply_operation_without_log(&Operation::SetLayerVisibility {
                layer: layer_id,
                visible,
            });
        self.status_message = format!("Layer {name} {}", if visible { "shown" } else { "hidden" });
        true
    }

    fn select_layout_shape(&mut self, value: &str) -> bool {
        let Ok(id) = value.parse::<u64>() else {
            return false;
        };
        let shape_id = ShapeId(id);
        let Some(shape) = self.workspace.document.shapes.get(&shape_id) else {
            return false;
        };
        self.selected_layout_shape = Some(shape_id);
        self.active_layer = shape.layer;
        self.status_message = format!("Selected shape #{}", shape.id.0);
        true
    }

    fn select_next_layout_shape(&mut self) -> bool {
        let shape_ids = layout_shape_ids(&self.workspace.document);
        let Some(next) = next_layout_shape_id(self.selected_layout_shape, &shape_ids) else {
            self.selected_layout_shape = None;
            self.status_message = "No layout shapes to select".to_string();
            return true;
        };
        self.select_layout_shape(&next.0.to_string())
    }

    fn add_layout_layer(&mut self) -> bool {
        let index = self.workspace.document.layers.len() + 1;
        let id = self.workspace.document.create_layer(
            format!("Layer {index}"),
            ProcessLayer::Annotation,
            [0.55, 0.76, 0.92, 0.35],
        );
        self.active_layer = id;
        self.status_message = format!("Added layer L{}", id.0);
        true
    }

    fn selected_layout_shape_ref(&self) -> Option<Shape> {
        self.selected_layout_shape
            .and_then(|id| self.workspace.document.shapes.get(&id))
    }

    fn copy_layout_selection(&mut self) -> bool {
        let Some(shape) = self.selected_layout_shape_ref() else {
            self.status_message = "Select a layout shape to copy".to_string();
            return true;
        };
        self.layout_clipboard_shapes.clear();
        self.layout_clipboard_shapes.push(shape);
        self.status_message = "Copied 1 shape".to_string();
        true
    }

    fn paste_layout_clipboard(&mut self) -> bool {
        if self.layout_clipboard_shapes.is_empty() {
            self.status_message = "Layout clipboard is empty".to_string();
            return true;
        }
        let shapes = self.layout_clipboard_shapes.clone();
        self.add_copied_layout_shapes(shapes, "Pasted")
    }

    fn duplicate_layout_selection(&mut self) -> bool {
        let Some(shape) = self.selected_layout_shape_ref() else {
            self.status_message = "Select a layout shape to duplicate".to_string();
            return true;
        };
        self.layout_clipboard_shapes.clear();
        self.layout_clipboard_shapes.push(shape.clone());
        self.add_copied_layout_shapes(vec![shape], "Duplicated")
    }

    fn add_copied_layout_shapes(&mut self, shapes: Vec<Shape>, label: &str) -> bool {
        let offset = (self.workspace.document.grid.max(10) * 8).max(80);
        let mut added = Vec::with_capacity(shapes.len());
        for mut shape in shapes {
            shape.id = self.workspace.document.allocate_shape_id();
            shape.kind.translate(Vector::new(offset, -offset));
            added.push(shape);
        }
        if added.is_empty() {
            return true;
        }
        let selected = added.first().map(|shape| shape.id);
        for shape in added.iter().cloned() {
            self.workspace
                .document
                .apply_operation_without_log(&Operation::AddShape { shape });
        }
        self.selected_layout_shape = selected;
        if let Some(shape) = self.selected_layout_shape_ref() {
            self.active_layer = shape.layer;
        }
        self.status_message = format!(
            "{label} {} shape{}",
            added.len(),
            if added.len() == 1 { "" } else { "s" }
        );
        true
    }

    fn delete_layout_selection(&mut self) -> bool {
        let Some(shape_id) = self.selected_layout_shape else {
            self.status_message = "No layout selection to delete".to_string();
            return true;
        };
        if !self.workspace.document.shapes.contains_key(&shape_id) {
            self.selected_layout_shape = None;
            self.status_message = "Layout selection was missing".to_string();
            return true;
        }
        self.workspace
            .document
            .apply_operation_without_log(&Operation::DeleteShape { id: shape_id });
        self.selected_layout_shape = None;
        self.status_message = format!("Deleted shape #{}", shape_id.0);
        true
    }

    fn connectivity_summary(&self) -> String {
        match extract_connectivity(
            &self.workspace.document,
            &layout_model::default_technology(),
        ) {
            Ok(report) => format!(
                "Connectivity: {} component(s), {} short(s), {} open(s)",
                report.components.len(),
                report.shorts.len(),
                report.opens.len()
            ),
            Err(error) => format!("Connectivity failed: {error}"),
        }
    }

    fn reset_layout_diff_review(&mut self) {
        self.layout_diff_change_page = 0;
        self.layout_diff_review_state = LayoutReviewDisposition::NeedsReview;
    }

    fn reset_layout_document_state(&mut self) {
        self.active_layer = default_active_layer(&self.workspace);
        self.selected_layout_shape = None;
        self.layout_clipboard_shapes.clear();
    }

    fn ensure_notebook_selection(&mut self) {
        let selected_valid = self
            .selected_notebook_entry
            .as_ref()
            .is_some_and(|entry_id| {
                notebook_filtered_entries(self)
                    .iter()
                    .any(|entry| &entry.id == entry_id)
            });
        if !selected_valid {
            self.selected_notebook_entry = notebook_filtered_entries(self)
                .first()
                .map(|entry| entry.id.clone());
        }
    }

    fn apply_notebook_entry_action(&mut self, action: NotebookEntryAction) -> bool {
        let Some(entry_id) = self.selected_notebook_entry.clone() else {
            self.status_message = "Notebook: select an entry before applying an action".to_string();
            return false;
        };
        let Some(entry) = self.workspace.lab_notebook.entry_mut(&entry_id) else {
            return false;
        };
        self.status_message = match action {
            NotebookEntryAction::AddFollowUpPlan => {
                add_notebook_tag_once(entry, "follow-up");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Follow-up",
                    "- [ ] Review the open disposition before the next run.\n- [ ] Attach updated metrology or yield evidence.",
                );
                format!("added follow-up plan to notebook entry {}", entry.id)
            }
            NotebookEntryAction::InsertMetrologyReview => {
                add_notebook_tag_once(entry, "metrology-review");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Metrology Review",
                    "- Confirm linked metrology covers the selected lot and wafer.\n- Record any spec excursions and owner.",
                );
                format!("inserted metrology review for notebook entry {}", entry.id)
            }
            NotebookEntryAction::RequestImageEvidence => {
                add_notebook_tag_once(entry, "image-needed");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Image Evidence",
                    "- [ ] Add inspection image or SEM reference.\n- [ ] Link image evidence to the impacted lot.",
                );
                format!("requested image evidence for notebook entry {}", entry.id)
            }
            NotebookEntryAction::TagHandoff => {
                add_notebook_tag_once(entry, "handoff");
                append_notebook_section_once(
                    &mut entry.body_markdown,
                    "## Shift Handoff",
                    "- [ ] Summarize open disposition, owner, and next lot to inspect.",
                );
                format!("tagged notebook entry {} for handoff", entry.id)
            }
        };
        entry.updated_at = "2026-05-06T17:00:00Z".to_string();
        true
    }

    fn focus_notebook_link(&mut self, kind: NotebookLinkKind, query: &str) -> bool {
        let Some(entry) = self
            .workspace
            .lab_notebook
            .entries
            .iter()
            .find(|entry| notebook_entry_has_link(entry, kind, query))
        else {
            return false;
        };
        self.selected_notebook_entry = Some(entry.id.clone());
        self.notebook_link_kind_filter = Some(kind);
        self.notebook_tag_filter = None;
        self.notebook_followups_only = false;
        self.status_message = format!("Notebook focused {} link {query}", kind.label());
        true
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

    fn equipment_selection_for_tool(
        &self,
        tool: &EquipmentTool,
        recipe_id: EquipmentRecipeId,
    ) -> RecipeSelection {
        let mut selection = self.workspace.equipment.selection_for(&tool.id, recipe_id);
        if let Some(lot_id) = self.workflow_focus_lot.as_deref() {
            selection.lot_id = Some(lot_id.to_string());
            selection.wafer_id = self
                .workspace
                .mes
                .lots
                .values()
                .find(|lot| lot.id.as_str() == lot_id)
                .and_then(|lot| lot.wafers.first())
                .map(|wafer| wafer.id.as_str().to_string());
        }
        selection
    }

    fn first_attention_die(&self) -> Option<DieCoord> {
        self.workspace
            .wafer_map
            .measurements
            .iter()
            .find(|measurement| {
                measurement.kind == self.metrology_kind
                    && measurement.status != MeasurementStatus::Pass
            })
            .or_else(|| {
                self.workspace
                    .wafer_map
                    .measurements
                    .iter()
                    .find(|measurement| measurement.status != MeasurementStatus::Pass)
            })
            .map(|measurement| measurement.die)
            .or_else(|| {
                self.workspace
                    .wafer_map
                    .defects
                    .first()
                    .map(|defect| defect.die)
            })
    }

    fn experiment_pending_count(&self) -> usize {
        let response_count = self.workspace.experiment_plan.responses.len();
        self.workspace
            .experiment_plan
            .runs
            .iter()
            .filter(|run| response_count == 0 || run.responses.len() < response_count)
            .count()
    }

    fn run_drc_summary(&self) -> String {
        let rules = RuleDeck::demo(&self.workspace.document);
        let rule_findings = rules.validate_for_document(&self.workspace.document);
        if !rule_findings.is_empty() {
            return format!("DRC rule deck has {} issue(s)", rule_findings.len());
        }
        let violations = run_drc(&self.workspace.document, &rules);
        format!("DRC found {} violation(s)", violations.len())
    }

    fn secondary_panel_label(&self) -> &'static str {
        match self.active_view {
            StartupView::Metrology => "Map",
            StartupView::MaskPrep
            | StartupView::CrossSection
            | StartupView::Layout2d
            | StartupView::Layout3d => "Layers",
            StartupView::Experiment => "Responses",
            StartupView::Notebook => "Links",
            _ => "Panel",
        }
    }

    fn has_inspector_panel(&self) -> bool {
        !matches!(
            self.active_view,
            StartupView::FabControl | StartupView::Yield | StartupView::Notebook
        )
    }

    fn has_secondary_panel(&self) -> bool {
        matches!(
            self.active_view,
            StartupView::Layout2d
                | StartupView::Layout3d
                | StartupView::Metrology
                | StartupView::MaskPrep
                | StartupView::CrossSection
                | StartupView::Experiment
                | StartupView::Notebook
        )
    }

    pub fn build_operad_document(&self, viewport: UiSize) -> Result<UiDocument, String> {
        self.build_operad_document_scaled(viewport, UiScale::new(1.0))
    }

    pub fn build_operad_document_scaled(
        &self,
        viewport: UiSize,
        ui_scale: UiScale,
    ) -> Result<UiDocument, String> {
        let mut document = UiDocument::new(root_style(viewport.width, viewport.height));
        document.set_node_visual(
            document.root,
            UiVisual::panel(ColorRgba::new(14, 18, 22, 255), None, 0.0),
        );

        let app = document.add_child(
            document.root,
            UiNode::container(
                "fabricad.app",
                UiNodeStyle {
                    layout: layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::percent(1.0),
                    )
                    .as_taffy_style()
                    .clone(),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );

        let menu_bar = document.add_child(
            app,
            UiNode::container(
                "fabricad.menu_bar",
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::row(),
                            layout::percent(1.0),
                            layout::px(ui_scale.value(42.0)),
                        ),
                        ui_scale.value(6.0),
                    ),
                    ui_scale.value(8.0),
                ),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(18, 23, 28, 255),
                Some(StrokeStyle::new(ColorRgba::new(48, 59, 70, 255), 1.0)),
                0.0,
            )),
        );

        for menu in AppMenu::ALL {
            add_button(
                &mut document,
                menu_bar,
                format!("fabricad.menu.{}", menu.slug()),
                menu.label(),
                self.active_menu == Some(menu),
                layout::size(
                    layout::px(ui_scale.value(98.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
        add_text(
            &mut document,
            menu_bar,
            "fabricad.status",
            self.status_message.clone(),
            text_style(
                ui_scale.value(12.0),
                FontWeight::NORMAL,
                ColorRgba::new(166, 176, 186, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(26.0))),
        );

        add_tool_strip(&mut document, app, self, ui_scale);

        if let Some(menu) = self.active_menu {
            add_menu_panel(
                &mut document,
                app,
                menu,
                self.active_view,
                self,
                viewport.width,
                ui_scale,
            );
        }

        let shell = document.add_child(
            app,
            UiNode::container(
                "fabricad.shell",
                UiNodeStyle {
                    layout: layout::with_min_size(
                        layout::with_size(
                            layout::with_flex(layout::row(), 1.0, 1.0, layout::px(0.0)),
                            layout::percent(1.0),
                            layout::auto(),
                        ),
                        layout::px(0.0),
                        layout::px(0.0),
                    )
                    .as_taffy_style()
                    .clone(),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            ),
        );

        let nav = document.add_child(
            shell,
            UiNode::container(
                "fabricad.nav",
                UiNodeStyle {
                    layout: layout::with_min_size(
                        layout::with_padding_all(
                            layout::with_gap_all(
                                layout::with_size(
                                    layout::column(),
                                    layout::px(ui_scale.value(220.0)),
                                    layout::percent(1.0),
                                ),
                                ui_scale.value(8.0),
                            ),
                            ui_scale.value(14.0),
                        ),
                        layout::px(0.0),
                        layout::px(0.0),
                    )
                    .as_taffy_style()
                    .clone(),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(24, 30, 36, 255),
                Some(StrokeStyle::new(ColorRgba::new(52, 63, 74, 255), 1.0)),
                0.0,
            ))
            .with_scroll(ScrollAxes::VERTICAL),
        );

        add_text(
            &mut document,
            nav,
            "fabricad.nav.title",
            "Fabricad",
            text_style(
                ui_scale.value(22.0),
                FontWeight::BOLD,
                ColorRgba::new(238, 243, 247, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(34.0))),
        );
        add_text(
            &mut document,
            nav,
            "fabricad.nav.subtitle",
            "Operad v4 shell",
            text_style(
                ui_scale.value(12.0),
                FontWeight::NORMAL,
                ColorRgba::new(142, 153, 164, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
        );

        for view in StartupView::ALL {
            let selected = view == self.active_view;
            let fill = if selected {
                ColorRgba::new(44, 68, 92, 255)
            } else {
                ColorRgba::new(31, 38, 46, 255)
            };
            document.add_child(
                nav,
                UiNode::text(
                    format!("fabricad.nav.action.{}", view.slug()),
                    view.label(),
                    text_style(
                        ui_scale.value(13.0),
                        if selected {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        },
                        ColorRgba::new(225, 231, 237, 255),
                    ),
                    layout::with_padding_all(
                        layout::size(layout::percent(1.0), layout::px(ui_scale.value(34.0))),
                        ui_scale.value(8.0),
                    ),
                )
                .with_input(InputBehavior::BUTTON)
                .with_accessibility(button_accessibility(view.label()))
                .with_visual(UiVisual::panel(
                    fill,
                    Some(StrokeStyle::new(
                        ColorRgba::new(64, 76, 88, 255),
                        ui_scale.value(1.0),
                    )),
                    ui_scale.value(4.0),
                )),
            );
        }

        let body = document.add_child(
            shell,
            UiNode::container(
                "fabricad.body",
                UiNodeStyle {
                    layout: layout::with_min_size(
                        layout::with_padding_all(
                            layout::with_gap_all(
                                layout::with_flex(layout::column(), 1.0, 1.0, layout::px(0.0)),
                                ui_scale.value(16.0),
                            ),
                            ui_scale.value(18.0),
                        ),
                        layout::px(0.0),
                        layout::px(0.0),
                    )
                    .as_taffy_style()
                    .clone(),
                    clip: ClipBehavior::Clip,
                    ..Default::default()
                },
            )
            .with_scroll(ScrollAxes::VERTICAL),
        );

        add_text(
            &mut document,
            body,
            "fabricad.header.eyebrow",
            "SEMICONDUCTOR LAYOUT WORKSPACE",
            text_style(
                ui_scale.value(12.0),
                FontWeight::BOLD,
                ColorRgba::new(123, 202, 184, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
        );
        add_text(
            &mut document,
            body,
            "fabricad.header.title",
            self.active_view.label(),
            text_style(
                ui_scale.value(28.0),
                FontWeight::BOLD,
                ColorRgba::new(246, 249, 252, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(42.0))),
        );

        let metrics = document.add_child(
            body,
            UiNode::container(
                "fabricad.metrics",
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(104.0)),
                    ),
                    ui_scale.value(12.0),
                ),
            ),
        );
        add_metric(
            &mut document,
            metrics,
            "shapes",
            self.workspace
                .document
                .flattened_shape_count_estimate()
                .to_string(),
            "layout objects",
            ui_scale,
        );
        add_metric(
            &mut document,
            metrics,
            "lots",
            self.workspace.mes.lots.len().to_string(),
            "active MES lots",
            ui_scale,
        );
        add_metric(
            &mut document,
            metrics,
            "recipes",
            self.workspace.recipe_catalog.recipes.len().to_string(),
            "qualified recipes",
            ui_scale,
        );
        add_metric(
            &mut document,
            metrics,
            "tools",
            self.workspace.equipment.tools().count().to_string(),
            "equipment tools",
            ui_scale,
        );

        add_view_controls(&mut document, body, self, ui_scale);

        add_primary_view_panel(&mut document, body, self, ui_scale);

        add_detail_sections(&mut document, body, self, ui_scale);

        let validation = self.workspace.validate();
        let status_text = if validation.is_valid() {
            "Workspace validation clean".to_string()
        } else {
            validation.error_summary()
        };
        add_text(
            &mut document,
            body,
            "fabricad.validation",
            format!("{status_text} - {}", self.status_message),
            text_style(
                ui_scale.value(14.0),
                FontWeight::NORMAL,
                ColorRgba::new(181, 190, 199, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
        );

        if self.show_inspector && self.has_inspector_panel() {
            add_inspector_panel(&mut document, shell, self, ui_scale);
        }
        if self.show_layers && self.has_secondary_panel() {
            add_secondary_panel(&mut document, shell, self, ui_scale);
        }

        document
            .compute_layout(viewport, &mut ApproxTextMeasurer)
            .map_err(|err| format!("Operad layout failed: {err}"))?;
        Ok(document)
    }

    pub fn audit_operad_document(&self, viewport: UiSize) -> Result<OperadAuditReport, String> {
        let document = self.build_operad_document(viewport)?;
        let paint_items = document.paint_list().items.len();
        Ok(OperadAuditReport {
            operad_version: "4.0.0",
            view: self.active_view,
            viewport,
            paint_items,
            layout_warnings: document.audit_layout().len(),
            document_shapes: self.workspace.document.flattened_shape_count_estimate(),
            workspace_lots: self.workspace.mes.lots.len(),
            equipment_tools: self.workspace.equipment.tools().count(),
        })
    }

    pub fn render_operad_snapshot(
        &self,
        width: u32,
        height: u32,
    ) -> Result<OperadSnapshotReport, String> {
        let width = width.max(1);
        let height = height.max(1);
        let viewport = UiSize::new(width as f32, height as f32);
        let document = self.build_operad_document(viewport)?;
        let paint = document.paint_list();
        let audit = OperadAuditReport {
            operad_version: "4.0.0",
            view: self.active_view,
            viewport,
            paint_items: paint.items.len(),
            layout_warnings: document.audit_layout().len(),
            document_shapes: self.workspace.document.flattened_shape_count_estimate(),
            workspace_lots: self.workspace.mes.lots.len(),
            equipment_tools: self.workspace.equipment.tools().count(),
        };
        let request = RenderFrameRequest::new(
            RenderTarget::snapshot(PixelSize::new(width, height)),
            viewport,
            paint,
        )
        .options(RenderOptions {
            clear_color: ColorRgba::new(14, 18, 22, 255),
            ..Default::default()
        });
        let mut renderer = WgpuRenderer::new();
        let render = renderer
            .render_frame(request, &operad::EmptyResourceResolver)
            .map_err(|err| format!("Operad wgpu render failed: {err}"))?;
        Ok(OperadSnapshotReport { audit, render })
    }
}

pub fn run_operad_audit(options: StartupOptions) -> Result<OperadAuditReport, String> {
    FabricadApp::new_with_options(options).audit_operad_document(UiSize::new(1440.0, 920.0))
}

pub fn render_operad_snapshot(
    options: StartupOptions,
    width: u32,
    height: u32,
) -> Result<OperadSnapshotReport, String> {
    FabricadApp::new_with_options(options).render_operad_snapshot(width, height)
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
    let path = path.as_ref();
    let document = Document::demo();
    let technology = layout_model::default_technology();
    let bytes = export_gdsii(&document, &technology)?;
    atomic_write_bytes(path, &bytes)?;
    Ok(())
}

pub fn validate_builtin_demo_workspace() -> Result<BuiltinDemoWorkspaceReport, String> {
    WorkspaceDataset::demo().builtin_demo_report()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistenceFixtureReport {
    pub migrated_legacy_schema_zero: usize,
    pub rejected_future_workspace_schema: usize,
    pub rejected_future_metadata_schema: usize,
    pub rejected_future_document_schema: usize,
    pub rejected_malformed_schema_fields: usize,
    pub rejected_malformed_metadata_arrays: usize,
    pub rejected_unsupported_feature_flags: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityFixtureReport {
    pub drc_violations: usize,
    pub drc_rule_families: usize,
    pub connectivity_components: usize,
    pub connectivity_shorts: usize,
    pub connectivity_opens: usize,
    pub connectivity_issue_keys: usize,
    pub connectivity_issue_states: usize,
}

pub fn validate_persistence_fixtures() -> Result<PersistenceFixtureReport, String> {
    let legacy = WorkspaceDataset::from_json_str(include_str!(
        "../../../fixtures/persistence/workspace_legacy_schema0_blank.json"
    ))?;
    if legacy.schema_version != WORKSPACE_DATASET_SCHEMA_VERSION {
        return Err(format!(
            "legacy persistence fixture migrated to workspace schema {}, expected {}",
            legacy.schema_version, WORKSPACE_DATASET_SCHEMA_VERSION
        ));
    }
    if !legacy.validate().is_valid() {
        return Err(format!(
            "legacy persistence fixture migrated into invalid workspace: {}",
            legacy.validate().error_summary()
        ));
    }

    assert_persistence_fixture_rejected(
        "future workspace schema",
        include_str!("../../../fixtures/persistence/workspace_future_schema_preflight.json"),
        &["workspace schema", "unsupported"],
    )?;
    assert_persistence_fixture_rejected(
        "future metadata schema",
        include_str!("../../../fixtures/persistence/workspace_future_metadata_preflight.json"),
        &["metadata schema", "unsupported"],
    )?;
    assert_persistence_fixture_rejected(
        "future document schema",
        include_str!(
            "../../../fixtures/persistence/workspace_future_document_schema_preflight.json"
        ),
        &["document schema", "unsupported"],
    )?;
    let malformed_schema_field = malformed_schema_field_workspace_json()?;
    assert_persistence_fixture_rejected(
        "malformed schema field",
        &malformed_schema_field,
        &["workspace schema version", "unsigned integer"],
    )?;
    assert_persistence_fixture_rejected(
        "malformed metadata arrays",
        include_str!(
            "../../../fixtures/persistence/workspace_malformed_metadata_arrays_preflight.json"
        ),
        &[
            "workspace metadata feature flags",
            "entry 1",
            "must be a string",
        ],
    )?;
    assert_persistence_fixture_rejected(
        "unsupported feature flag",
        include_str!("../../../fixtures/persistence/workspace_unsupported_feature_flag.json"),
        &["unsupported feature flag", "future_mask_revision_model"],
    )?;

    Ok(PersistenceFixtureReport {
        migrated_legacy_schema_zero: 1,
        rejected_future_workspace_schema: 1,
        rejected_future_metadata_schema: 1,
        rejected_future_document_schema: 1,
        rejected_malformed_schema_fields: 1,
        rejected_malformed_metadata_arrays: 1,
        rejected_unsupported_feature_flags: 1,
    })
}

pub fn validate_quality_fixtures() -> Result<QualityFixtureReport, String> {
    let drc = validate_drc_quality_fixture()?;
    let connectivity = validate_connectivity_quality_fixture()?;
    Ok(QualityFixtureReport {
        drc_violations: drc.0,
        drc_rule_families: drc.1,
        connectivity_components: connectivity.component_count,
        connectivity_shorts: connectivity.short_count,
        connectivity_opens: connectivity.open_count,
        connectivity_issue_keys: connectivity.issue_key_count,
        connectivity_issue_states: connectivity.issue_state_count,
    })
}

fn default_active_layer(workspace: &WorkspaceDataset) -> LayerId {
    workspace
        .document
        .layer_by_process(ProcessLayer::Metal1)
        .or_else(|| workspace.document.layers.keys().next().copied())
        .unwrap_or(LayerId(1))
}

fn default_layout_shape(workspace: &WorkspaceDataset) -> Option<ShapeId> {
    layout_shape_ids(&workspace.document).first().copied()
}

fn layout_shape_ids(document: &Document) -> Vec<ShapeId> {
    document.shapes.keys().copied().collect()
}

fn next_layout_shape_id(current: Option<ShapeId>, shape_ids: &[ShapeId]) -> Option<ShapeId> {
    if shape_ids.is_empty() {
        return None;
    }
    let Some(current) = current else {
        return shape_ids.first().copied();
    };
    let index = shape_ids.iter().position(|id| *id == current);
    let next = index.map_or(0, |index| (index + 1) % shape_ids.len());
    shape_ids.get(next).copied()
}

fn move_first_layout_shape_vertex(document: &mut Document) -> Option<ShapeId> {
    let shape_id = layout_shape_ids(document).first().copied()?;
    let mut shape = document.shapes.get(&shape_id)?.clone();
    match &mut shape.kind {
        ShapeKind::Rectangle(rect) => {
            let moved = Point::new(rect.min.x - 500, rect.min.y + 420);
            *rect = Rect::new(moved, rect.max);
        }
        ShapeKind::Polygon(polygon) => {
            if let Some(point) = polygon.points.first_mut() {
                *point = Point::new(point.x - 500, point.y + 420);
            } else {
                return None;
            }
        }
        ShapeKind::Path { points, .. } => {
            if let Some(point) = points.first_mut() {
                *point = Point::new(point.x - 500, point.y + 420);
            } else {
                return None;
            }
        }
        ShapeKind::Via { center, .. } => {
            *center = Point::new(center.x - 500, center.y + 420);
        }
        ShapeKind::Label { position, .. } => {
            *position = Point::new(position.x - 500, position.y + 420);
        }
        ShapeKind::Measurement { a, .. } => {
            *a = Point::new(a.x - 500, a.y + 420);
        }
    }
    document.apply_operation_without_log(&Operation::ReplaceShape {
        id: shape_id,
        shape,
    });
    Some(shape_id)
}

fn default_yield_lot(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .yield_analysis
        .lots
        .first()
        .map(|lot| lot.id.clone())
}

fn default_yield_wafer(workspace: &WorkspaceDataset, lot_id: Option<&str>) -> Option<String> {
    lot_id.and_then(|lot_id| {
        workspace
            .yield_analysis
            .wafer_ids_for_lot(lot_id)
            .first()
            .cloned()
    })
}

fn default_experiment_run(workspace: &WorkspaceDataset) -> Option<ExperimentRunId> {
    workspace
        .experiment_plan
        .runs
        .first()
        .map(|run| run.id.clone())
}

fn default_experiment_response(workspace: &WorkspaceDataset) -> Option<ResponseSpecId> {
    workspace
        .experiment_plan
        .responses
        .first()
        .map(|response| response.id.clone())
}

fn workflow_lot_ids(workspace: &WorkspaceDataset) -> Vec<String> {
    let mut ids = workspace
        .mes
        .lots
        .values()
        .map(|lot| lot.id.as_str().to_string())
        .collect::<Vec<_>>();
    for lot in &workspace.yield_analysis.lots {
        if !ids.iter().any(|id| id == &lot.id) {
            ids.push(lot.id.clone());
        }
    }
    ids
}

fn default_workflow_focus_lot(workspace: &WorkspaceDataset) -> Option<String> {
    let lot_ids = workflow_lot_ids(workspace);
    lot_ids
        .iter()
        .find(|lot_id| lot_id.as_str() == "L-00042")
        .cloned()
        .or_else(|| lot_ids.first().cloned())
}

fn default_equipment_tool(workspace: &WorkspaceDataset) -> Option<EquipmentToolId> {
    workspace
        .equipment
        .tools()
        .next()
        .map(|tool| tool.id.clone())
}

fn default_maintenance_tool(workspace: &WorkspaceDataset) -> Option<EquipmentToolId> {
    workspace
        .maintenance
        .tools
        .first()
        .map(|tool| tool.tool_id.clone())
}

fn default_environment_sensor(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .environment
        .sensors
        .first()
        .map(|sensor| sensor.id.clone())
}

fn inventory_filter_matches(filter: InventoryQuickFilter, lot: &MaterialLot) -> bool {
    match filter {
        InventoryQuickFilter::All => true,
        InventoryQuickFilter::NeedsAction => {
            lot.is_expired(INVENTORY_DEMO_TODAY)
                || lot.is_low_stock()
                || lot.expires_within_days(INVENTORY_DEMO_TODAY, 30)
        }
        InventoryQuickFilter::ProductHold => lot.is_expired(INVENTORY_DEMO_TODAY),
        InventoryQuickFilter::LowStock => lot.is_low_stock(),
        InventoryQuickFilter::ExpiringSoon => lot.expires_within_days(INVENTORY_DEMO_TODAY, 30),
        InventoryQuickFilter::InUse => !lot.usage.is_empty(),
    }
}

fn inventory_filter_count(workspace: &WorkspaceDataset, filter: InventoryQuickFilter) -> usize {
    workspace
        .inventory
        .lots
        .values()
        .filter(|lot| inventory_filter_matches(filter, lot))
        .count()
}

fn default_inventory_lot(
    workspace: &WorkspaceDataset,
    filter: InventoryQuickFilter,
) -> Option<MaterialLotId> {
    workspace
        .inventory
        .sorted_lots()
        .into_iter()
        .find(|lot| inventory_filter_matches(filter, lot))
        .map(|lot| lot.id.clone())
        .or_else(|| {
            workspace
                .inventory
                .sorted_lots()
                .first()
                .map(|lot| lot.id.clone())
        })
}

fn selected_inventory_lot<'a>(app: &'a FabricadApp) -> Option<&'a MaterialLot> {
    app.selected_inventory_lot
        .as_ref()
        .and_then(|id| app.workspace.inventory.lot(id))
        .or_else(|| app.workspace.inventory.sorted_lots().first().copied())
}

fn default_mask_lot(workspace: &WorkspaceDataset) -> Option<LotId> {
    default_workflow_focus_lot(workspace)
        .map(LotId::new)
        .filter(|lot_id| workspace.mes.lots.contains_key(lot_id))
        .or_else(|| workspace.mes.lots.keys().next().cloned())
}

fn reticle_prep_for_app(app: &FabricadApp) -> ReticlePrep {
    app.mask_source_lot
        .as_ref()
        .and_then(|lot_id| app.workspace.mes.lots.get(lot_id))
        .and_then(|lot| app.workspace.mes.routes.get(&lot.route_id))
        .map(|route| ReticlePrep::from_document_and_route(&app.workspace.document, route))
        .unwrap_or_else(|| ReticlePrep::from_document(&app.workspace.document))
}

fn mask_check_report(app: &FabricadApp) -> MaskCheckReport {
    reticle_prep_for_app(app).validate_document(&app.workspace.document)
}

fn mask_issue_matches_filter(app: &FabricadApp, issue: &layout_model::mask::MaskPrepIssue) -> bool {
    app.mask_issue_severity_filter.matches(issue.severity)
}

fn mask_filtered_issue_count(app: &FabricadApp, report: &MaskCheckReport) -> usize {
    report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
        .count()
}

fn mask_issue_page_count(app: &FabricadApp, report: &MaskCheckReport) -> usize {
    mask_filtered_issue_count(app, report)
        .div_ceil(MASK_ISSUE_PAGE_SIZE)
        .max(1)
}

fn mask_issue_group_count(app: &FabricadApp, report: &MaskCheckReport) -> usize {
    report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
        .map(|issue| mask_issue_group_key(issue, app.mask_issue_grouping))
        .collect::<BTreeSet<_>>()
        .len()
}

fn mask_issue_group_key(
    issue: &layout_model::mask::MaskPrepIssue,
    grouping: MaskIssueGrouping,
) -> String {
    match grouping {
        MaskIssueGrouping::Code => issue.code.clone(),
        MaskIssueGrouping::Layer => issue
            .layer
            .map(|layer| format!("layer {}", layer.0))
            .unwrap_or_else(|| "no layer".to_string()),
        MaskIssueGrouping::Field => issue
            .field_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no field".to_string()),
        MaskIssueGrouping::Block => issue
            .block_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "no block".to_string()),
    }
}

fn layout_diff_report(app: &FabricadApp) -> LayoutDiffReport {
    let baseline = app.layout_diff_baseline.document(&app.workspace.document);
    let candidate = app.layout_diff_candidate.document(&app.workspace.document);
    diff_documents(
        app.layout_diff_baseline.detail_label(),
        &baseline,
        app.layout_diff_candidate.detail_label(),
        &candidate,
    )
}

fn layout_diff_filtered_change_count(app: &FabricadApp, report: &LayoutDiffReport) -> usize {
    report
        .changes
        .iter()
        .filter(|change| app.layout_diff_change_filter.matches(change.kind))
        .count()
}

fn layout_diff_page_count(app: &FabricadApp, report: &LayoutDiffReport) -> usize {
    let page_size = normalized_layout_diff_page_size(app);
    layout_diff_filtered_change_count(app, report)
        .div_ceil(page_size)
        .max(1)
}

fn normalized_layout_diff_page_size(app: &FabricadApp) -> usize {
    if LAYOUT_DIFF_PAGE_SIZE_OPTIONS.contains(&app.layout_diff_page_size) {
        app.layout_diff_page_size
    } else {
        LAYOUT_DIFF_DEFAULT_PAGE_SIZE
    }
}

fn layout_diff_total_changes(report: &LayoutDiffReport) -> usize {
    report.summary.added_shapes + report.summary.removed_shapes + report.summary.modified_shapes
}

fn default_trace_lot(workspace: &WorkspaceDataset) -> Option<LotId> {
    workspace.genealogy.lot_ids().first().cloned()
}

fn default_trace_wafer(workspace: &WorkspaceDataset, lot_id: Option<&LotId>) -> Option<WaferId> {
    lot_id.and_then(|lot_id| {
        workspace
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .first()
            .map(|wafer| wafer.wafer_id.clone())
    })
}

fn default_trace_selection(
    workspace: &WorkspaceDataset,
    lot_id: Option<&LotId>,
    wafer_id: Option<&WaferId>,
) -> Option<TraceSelection> {
    if let (Some(lot_id), Some(wafer_id)) = (lot_id, wafer_id) {
        let wafer = WaferRef {
            lot_id: lot_id.clone(),
            wafer_id: wafer_id.clone(),
        };
        if workspace.genealogy.wafer(&wafer).is_some() {
            return Some(TraceSelection::Wafer(wafer));
        }
    }
    lot_id
        .filter(|lot_id| workspace.genealogy.lots.contains_key(lot_id))
        .cloned()
        .map(TraceSelection::Lot)
}

fn selected_trace_wafer_ref(app: &FabricadApp) -> Option<WaferRef> {
    Some(WaferRef {
        lot_id: app.selected_trace_lot.clone()?,
        wafer_id: app.selected_trace_wafer.clone()?,
    })
}

fn trace_selection_from_view_action(value: &str) -> Option<TraceSelection> {
    if let Some(lot_id) = value.strip_prefix("lot.") {
        return Some(TraceSelection::Lot(LotId::new(lot_id)));
    }
    if let Some(raw) = value.strip_prefix("wafer.") {
        let (lot_id, wafer_id) = raw.split_once('|')?;
        return Some(TraceSelection::Wafer(WaferRef {
            lot_id: LotId::new(lot_id),
            wafer_id: WaferId::new(wafer_id),
        }));
    }
    if let Some(sequence) = value.strip_prefix("process.") {
        return Some(TraceSelection::Process(sequence.parse().ok()?));
    }
    if let Some(sequence) = value.strip_prefix("material.") {
        return Some(TraceSelection::MaterialUse(sequence.parse().ok()?));
    }
    if let Some(sequence) = value.strip_prefix("event.") {
        return Some(TraceSelection::Event(sequence.parse().ok()?));
    }
    None
}

fn trace_detail_action(selection: &TraceSelection) -> String {
    match selection {
        TraceSelection::Lot(lot_id) => format!("fabricad.viewctl.trace.detail.lot.{lot_id}"),
        TraceSelection::Wafer(wafer) => format!(
            "fabricad.viewctl.trace.detail.wafer.{}|{}",
            wafer.lot_id, wafer.wafer_id
        ),
        TraceSelection::Process(sequence) => {
            format!("fabricad.viewctl.trace.detail.process.{sequence}")
        }
        TraceSelection::MaterialUse(sequence) => {
            format!("fabricad.viewctl.trace.detail.material.{sequence}")
        }
        TraceSelection::Event(sequence) => {
            format!("fabricad.viewctl.trace.detail.event.{sequence}")
        }
    }
}

fn trace_related_wafer_count(app: &FabricadApp) -> usize {
    let Some(wafer) = selected_trace_wafer_ref(app) else {
        return 0;
    };
    app.workspace.genealogy.wafer_lineage(&wafer).len()
        + app.workspace.genealogy.wafer_descendants(&wafer).len()
}

fn trace_impact_count(app: &FabricadApp) -> usize {
    trace_impact_query(app)
        .map(|query| {
            app.workspace
                .genealogy
                .impact_for(query)
                .impacted_wafers
                .len()
        })
        .unwrap_or(0)
}

fn trace_impact_query(app: &FabricadApp) -> Option<layout_model::genealogy::ExcursionQuery> {
    use layout_model::genealogy::ExcursionQuery;
    let wafer = selected_trace_wafer_ref(app)?;
    match app.trace_impact_mode {
        TraceImpactMode::ToolRun => app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::ToolRun {
                tool_run_id: record.tool_run_id.clone(),
            }),
        TraceImpactMode::Step => app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::ProcessStep {
                step_id: record.step_id.clone(),
            }),
        TraceImpactMode::Material => app
            .workspace
            .genealogy
            .material_ancestry_for_wafer(&wafer)
            .last()
            .map(|record| ExcursionQuery::MaterialLot {
                material_lot_id: record.material_lot_id.clone(),
            }),
    }
}

fn default_notebook_entry(workspace: &WorkspaceDataset) -> Option<NotebookEntryId> {
    workspace
        .lab_notebook
        .entries
        .first()
        .map(|entry| entry.id.clone())
}

fn notebook_filter(app: &FabricadApp) -> NotebookFilter {
    NotebookFilter {
        query: String::new(),
        tag: app.notebook_tag_filter.clone(),
        link_kind: app.notebook_link_kind_filter,
    }
}

fn notebook_filtered_entries(app: &FabricadApp) -> Vec<&layout_model::notebook::NotebookEntry> {
    app.workspace
        .lab_notebook
        .filtered_entries(&notebook_filter(app))
        .into_iter()
        .filter(|entry| !app.notebook_followups_only || notebook_entry_has_followup(entry))
        .collect()
}

fn selected_notebook_entry<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::notebook::NotebookEntry> {
    app.selected_notebook_entry
        .as_ref()
        .and_then(|id| app.workspace.lab_notebook.entry(id))
        .or_else(|| notebook_filtered_entries(app).first().copied())
}

fn notebook_entry_has_followup(entry: &layout_model::notebook::NotebookEntry) -> bool {
    entry.tags.iter().any(|tag| tag.contains("follow"))
        || entry
            .body_markdown
            .to_ascii_lowercase()
            .contains("follow-up")
}

fn notebook_link_count_for_kind(app: &FabricadApp, kind: NotebookLinkKind) -> usize {
    app.workspace
        .lab_notebook
        .entries
        .iter()
        .filter(|entry| entry.has_link_kind(kind))
        .count()
}

fn notebook_link_kind_slug(kind: NotebookLinkKind) -> &'static str {
    match kind {
        NotebookLinkKind::Lot => "lot",
        NotebookLinkKind::Wafer => "wafer",
        NotebookLinkKind::Recipe => "recipe",
        NotebookLinkKind::ToolRun => "tool-run",
        NotebookLinkKind::Metrology => "metrology",
        NotebookLinkKind::Image => "image",
    }
}

fn notebook_link_kind_from_slug(value: &str) -> Option<NotebookLinkKind> {
    match value {
        "lot" => Some(NotebookLinkKind::Lot),
        "wafer" => Some(NotebookLinkKind::Wafer),
        "recipe" => Some(NotebookLinkKind::Recipe),
        "tool-run" => Some(NotebookLinkKind::ToolRun),
        "metrology" => Some(NotebookLinkKind::Metrology),
        "image" => Some(NotebookLinkKind::Image),
        _ => None,
    }
}

fn dispatch_policy_slug(policy: DispatchPolicy) -> &'static str {
    match policy {
        DispatchPolicy::Fifo => "fifo",
        DispatchPolicy::PriorityThenFifo => "priority",
        DispatchPolicy::DueDateThenPriority => "due-date",
    }
}

fn dispatch_policy_from_slug(value: &str) -> Option<DispatchPolicy> {
    match value {
        "fifo" => Some(DispatchPolicy::Fifo),
        "priority" => Some(DispatchPolicy::PriorityThenFifo),
        "due-date" => Some(DispatchPolicy::DueDateThenPriority),
        _ => None,
    }
}

fn default_scheduler_tool(workspace: &WorkspaceDataset) -> Option<SchedulerToolId> {
    workspace
        .scheduler
        .tools
        .first()
        .map(|tool| tool.id.clone())
}

fn scheduler_assignment_count(app: &FabricadApp) -> usize {
    app.workspace
        .scheduler
        .dispatch(app.scheduler_policy)
        .assignments
        .len()
}

fn scheduler_unscheduled_count(app: &FabricadApp) -> usize {
    app.workspace
        .scheduler
        .dispatch(app.scheduler_policy)
        .unscheduled_lots
        .len()
}

fn scheduler_filtered_lot_count(app: &FabricadApp) -> usize {
    app.workspace
        .scheduler
        .lots
        .iter()
        .filter(|lot| lot.priority >= app.scheduler_min_priority)
        .filter(|lot| {
            !app.scheduler_focus_selected_tool
                || app
                    .selected_scheduler_tool
                    .as_ref()
                    .and_then(|tool_id| {
                        app.workspace
                            .scheduler
                            .tools
                            .iter()
                            .find(|tool| &tool.id == tool_id)
                    })
                    .is_some_and(|tool| tool.can_process(lot))
        })
        .count()
}

fn default_safety_tool(workspace: &WorkspaceDataset) -> Option<String> {
    workspace
        .safety
        .evaluate_lockouts()
        .first()
        .map(|lockout| lockout.tool_id.clone())
}

fn selected_safety_lockout(app: &FabricadApp) -> Option<layout_model::safety::ToolLockout> {
    let lockouts = app.workspace.safety.evaluate_lockouts();
    app.selected_safety_tool
        .as_deref()
        .and_then(|tool_id| {
            lockouts
                .iter()
                .find(|lockout| lockout.tool_id == tool_id)
                .cloned()
        })
        .or_else(|| lockouts.first().cloned())
}

fn safety_highest_label(app: &FabricadApp) -> &'static str {
    app.workspace
        .safety
        .summary()
        .highest_severity
        .unwrap_or(SafetySeverity::Normal)
        .label()
}

fn process_flow_node_matches(
    workspace: &WorkspaceDataset,
    filter: ProcessFlowNodeFilter,
    node: &layout_model::process_flow::ProcessFlowNode,
) -> bool {
    match filter {
        ProcessFlowNodeFilter::All => true,
        ProcessFlowNodeFilter::RecipeSteps => node.kind.requires_recipe() || node.recipe.is_some(),
        ProcessFlowNodeFilter::Metrology => {
            node.kind == ProcessFlowNodeKind::Measurement || node.measurement_checkpoint.is_some()
        }
        ProcessFlowNodeFilter::Holds => node.kind == ProcessFlowNodeKind::Hold || node.hold_point,
        ProcessFlowNodeFilter::Rework => workspace.process_flow.route.edges.iter().any(|edge| {
            edge.kind == layout_model::process_flow::ProcessFlowEdgeKind::Rework
                && (edge.from == node.id || edge.to == node.id)
        }),
    }
}

fn process_flow_filter_count(workspace: &WorkspaceDataset, filter: ProcessFlowNodeFilter) -> usize {
    workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(workspace, filter, node))
        .count()
}

fn default_process_flow_node(workspace: &WorkspaceDataset) -> Option<ProcessFlowNodeId> {
    workspace
        .process_flow
        .route
        .nodes
        .first()
        .map(|node| node.id.clone())
}

fn default_process_flow_node_for_filter(
    workspace: &WorkspaceDataset,
    filter: ProcessFlowNodeFilter,
) -> Option<ProcessFlowNodeId> {
    workspace
        .process_flow
        .route
        .nodes
        .iter()
        .find(|node| process_flow_node_matches(workspace, filter, node))
        .map(|node| node.id.clone())
        .or_else(|| default_process_flow_node(workspace))
}

fn selected_process_flow_node<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::process_flow::ProcessFlowNode> {
    app.selected_process_node
        .as_ref()
        .and_then(|id| {
            app.workspace
                .process_flow
                .route
                .nodes
                .iter()
                .find(|node| &node.id == id)
        })
        .or_else(|| app.workspace.process_flow.route.nodes.first())
}

fn process_flow_error_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .process_flow
        .findings()
        .iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .count()
}

fn default_control_loop(workspace: &WorkspaceDataset) -> Option<ControlLoopId> {
    workspace
        .process_control
        .loops
        .first()
        .map(|loop_definition| loop_definition.id.clone())
}

fn default_control_action(
    workspace: &WorkspaceDataset,
    loop_id: Option<&ControlLoopId>,
) -> Option<ControlActionId> {
    loop_id
        .and_then(|loop_id| {
            workspace
                .process_control
                .actions_for_loop(loop_id)
                .first()
                .map(|action| action.id.clone())
        })
        .or_else(|| {
            workspace
                .process_control
                .actions
                .first()
                .map(|action| action.id.clone())
        })
}

fn selected_control_loop<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::process_control::ControlLoop> {
    app.selected_control_loop
        .as_ref()
        .and_then(|id| app.workspace.process_control.loop_by_id(id))
        .or_else(|| app.workspace.process_control.loops.first())
}

fn selected_control_action<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::process_control::ControlAction> {
    app.selected_control_action
        .as_ref()
        .and_then(|id| {
            app.workspace
                .process_control
                .actions
                .iter()
                .find(|action| &action.id == id)
        })
        .or_else(|| {
            selected_control_loop(app).and_then(|loop_definition| {
                app.workspace
                    .process_control
                    .actions_for_loop(&loop_definition.id)
                    .first()
                    .copied()
            })
        })
}

fn process_control_action_count(app: &FabricadApp, state: ControlActionState) -> usize {
    app.workspace
        .process_control
        .actions
        .iter()
        .filter(|action| action.state == state)
        .count()
}

fn process_control_transition_options(state: ControlActionState) -> &'static [&'static str] {
    match state {
        ControlActionState::Proposed => &["approve", "reject"],
        ControlActionState::Approved => &["apply", "reject"],
        ControlActionState::Held => &["reject"],
        ControlActionState::Rejected | ControlActionState::Applied => &[],
    }
}

fn process_control_transition_label(value: &str) -> &'static str {
    match value {
        "approve" => "Approve",
        "reject" => "Reject",
        "apply" => "Apply",
        _ => "Transition",
    }
}

fn short_control_action_label(action_id: &ControlActionId) -> String {
    let value = action_id.as_str();
    value
        .rsplit_once('-')
        .map(|(_, suffix)| format!("PCA-{suffix}"))
        .unwrap_or_else(|| value.to_string())
}

fn compact_button_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let prefix_len = max_chars.saturating_sub(1);
    let prefix = value.chars().take(prefix_len).collect::<String>();
    format!("{prefix}..")
}

fn spc_fdc_monitor(workspace: &WorkspaceDataset) -> SpcFdcMonitor {
    let sensor_samples = workspace
        .equipment
        .tools()
        .flat_map(|tool| tool.recent_sensors.iter().cloned())
        .collect::<Vec<_>>();
    let alarms = workspace
        .equipment
        .tools()
        .flat_map(|tool| tool.active_alarms.iter().cloned())
        .collect::<Vec<_>>();
    SpcFdcMonitor::from_fab_context(
        &workspace.yield_analysis.process_measurements,
        &sensor_samples,
        &alarms,
    )
}

fn default_spc_chart(workspace: &WorkspaceDataset) -> Option<String> {
    spc_fdc_monitor(workspace)
        .charts
        .first()
        .map(|chart| chart.id.as_str().to_string())
}

fn default_fdc_trace(workspace: &WorkspaceDataset) -> Option<String> {
    spc_fdc_monitor(workspace)
        .traces
        .first()
        .map(|trace| trace.id.clone())
}

fn selected_spc_chart<'a>(
    app: &'a FabricadApp,
    monitor: &'a SpcFdcMonitor,
) -> Option<&'a layout_model::spc_fdc::ControlChart> {
    app.selected_spc_chart
        .as_deref()
        .and_then(|id| monitor.charts.iter().find(|chart| chart.id.as_str() == id))
        .or_else(|| monitor.charts.first())
}

fn selected_fdc_trace<'a>(
    app: &'a FabricadApp,
    monitor: &'a SpcFdcMonitor,
) -> Option<&'a layout_model::spc_fdc::SensorTrace> {
    app.selected_fdc_trace
        .as_deref()
        .and_then(|id| monitor.traces.iter().find(|trace| trace.id == id))
        .or_else(|| monitor.traces.first())
}

fn spc_filtered_finding_count(app: &FabricadApp, monitor: &SpcFdcMonitor) -> usize {
    monitor
        .findings
        .iter()
        .filter(|finding| app.spc_severity_filter.matches(finding.severity))
        .filter(|finding| app.spc_source_filter.matches(finding.source))
        .filter(|finding| {
            app.spc_context_filter.is_empty()
                || finding.title.contains(&app.spc_context_filter)
                || finding.detail.contains(&app.spc_context_filter)
                || finding
                    .tool_id
                    .as_deref()
                    .is_some_and(|tool_id| tool_id.contains(&app.spc_context_filter))
                || finding
                    .lot_id
                    .as_deref()
                    .is_some_and(|lot_id| lot_id.contains(&app.spc_context_filter))
                || finding
                    .recipe_id
                    .as_deref()
                    .is_some_and(|recipe_id| recipe_id.contains(&app.spc_context_filter))
        })
        .count()
}

fn default_cross_section_material(workspace: &WorkspaceDataset) -> Option<MaterialId> {
    Some(workspace.cross_section.substrate_material.clone()).or_else(|| {
        workspace
            .cross_section
            .materials
            .first()
            .map(|material| material.id.clone())
    })
}

fn cross_section_snapshot_count(workspace: &WorkspaceDataset) -> usize {
    workspace.cross_section.steps.len() + 1
}

fn cross_section_selected_title(app: &FabricadApp) -> String {
    if app.cross_section_step == 0 {
        "Starting substrate".to_string()
    } else {
        app.workspace
            .cross_section
            .steps
            .get(app.cross_section_step.saturating_sub(1))
            .map(|step| step.name.clone())
            .unwrap_or_else(|| format!("Step {}", app.cross_section_step))
    }
}

fn maintenance_today(workspace: &WorkspaceDataset) -> FabDate {
    workspace
        .maintenance
        .today
        .unwrap_or(FabDate::new(2026, 5, 11))
}

fn maintenance_work_matches(
    workspace: &WorkspaceDataset,
    filter: MaintenanceWorkFilter,
    tool_id: &EquipmentToolId,
    kind: MaintenanceKind,
    due_state: DueState,
    due_date: FabDate,
) -> bool {
    let today = maintenance_today(workspace);
    let release = workspace.maintenance.release_for_tool(tool_id, today);
    let blocked = !release.state.released_to_production();
    let days_until = today.days_until(due_date);
    match filter {
        MaintenanceWorkFilter::Actionable => blocked || due_state.blocks_release(),
        MaintenanceWorkFilter::Upcoming => due_state != DueState::Overdue && days_until <= 14,
        MaintenanceWorkFilter::Calibration => {
            kind == MaintenanceKind::Calibration
                || release.state == ToolReleaseState::CalibrationLockout
        }
        MaintenanceWorkFilter::Locked => blocked,
        MaintenanceWorkFilter::All => true,
    }
}

fn maintenance_filtered_work_count(
    workspace: &WorkspaceDataset,
    filter: MaintenanceWorkFilter,
) -> usize {
    let today = maintenance_today(workspace);
    workspace
        .maintenance
        .tools
        .iter()
        .flat_map(|tool| {
            tool.schedules.iter().filter(move |schedule| {
                maintenance_work_matches(
                    workspace,
                    filter,
                    &tool.tool_id,
                    schedule.kind,
                    schedule.due_state(today, tool.run_count),
                    schedule.next_due,
                )
            })
        })
        .count()
}

fn maintenance_locked_count(workspace: &WorkspaceDataset) -> usize {
    let today = maintenance_today(workspace);
    workspace
        .maintenance
        .tools
        .iter()
        .filter(|tool| {
            !workspace
                .maintenance
                .release_for_tool(&tool.tool_id, today)
                .state
                .released_to_production()
        })
        .count()
}

fn selected_maintenance_tool<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::maintenance::ToolMaintenanceState> {
    app.selected_maintenance_tool
        .as_ref()
        .and_then(|id| app.workspace.maintenance.tool(id))
        .or_else(|| app.workspace.maintenance.tools.first())
}

fn selected_environment_sensor<'a>(
    app: &'a FabricadApp,
) -> Option<&'a layout_model::environment::EnvironmentSensor> {
    app.selected_environment_sensor
        .as_deref()
        .and_then(|id| {
            app.workspace
                .environment
                .sensors
                .iter()
                .find(|sensor| sensor.id == id)
        })
        .or_else(|| app.workspace.environment.sensors.first())
}

fn environment_sensor_status(app: &FabricadApp) -> String {
    let Some(sensor) = selected_environment_sensor(app) else {
        return "No sensor".to_string();
    };
    app.workspace
        .environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            let state = severity.map_or("Nominal", |severity| severity.label());
            format!("{:.2} {} {}", reading.value, sensor.unit, state)
        })
        .unwrap_or_else(|| "No samples".to_string())
}

fn equipment_active_alarm_count(tool: &EquipmentTool) -> usize {
    tool.active_alarms
        .iter()
        .filter(|alarm| alarm.active)
        .count()
}

fn equipment_running_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .filter(|tool| tool.state == EquipmentToolState::Running)
        .count()
}

fn equipment_alarm_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .map(equipment_active_alarm_count)
        .sum()
}

fn equipment_sample_count(workspace: &WorkspaceDataset) -> usize {
    workspace
        .equipment
        .tools()
        .map(|tool| tool.recent_sensors.len())
        .sum()
}

fn selected_equipment_tool<'a>(app: &'a FabricadApp) -> Option<&'a EquipmentTool> {
    app.selected_equipment_tool
        .as_ref()
        .and_then(|id| app.workspace.equipment.tool(id))
        .or_else(|| app.workspace.equipment.tools().next())
}

fn workflow_focus_traveler_label(app: &FabricadApp) -> String {
    let Some(lot_id) = app.workflow_focus_lot.as_deref() else {
        return "No lot".to_string();
    };
    app.workspace
        .mes
        .travelers
        .values()
        .find(|traveler| traveler.lot_id.as_str() == lot_id)
        .map(|traveler| {
            format!(
                "{} {}",
                traveler.status.label(),
                traveler
                    .current_step_id
                    .as_ref()
                    .map(|step| step.as_str())
                    .unwrap_or("complete")
            )
        })
        .unwrap_or_else(|| "No traveler".to_string())
}

fn workflow_focus_yield_label(app: &FabricadApp) -> String {
    let Some(lot_id) = app.workflow_focus_lot.as_deref() else {
        return "No lot".to_string();
    };
    app.workspace
        .yield_analysis
        .lot_summary(lot_id)
        .map(|summary| format!("{:.1}%", summary.yield_fraction * 100.0))
        .unwrap_or_else(|| "No yield summary".to_string())
}

fn equipment_recipe_summary(tool: &EquipmentTool) -> String {
    if let Some(run) = &tool.active_run {
        return format!("{} {}", run.recipe.recipe_id, run.status.label());
    }
    if let Some(selection) = &tool.selected_recipe {
        return format!("{} loaded", selection.recipe_id);
    }
    "No recipe".to_string()
}

fn measurement_kind_slug(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::ThicknessNm => "thickness",
        MeasurementKind::SheetResistanceOhmsPerSq => "sheet-r",
        MeasurementKind::CriticalDimensionNm => "cd",
        MeasurementKind::DefectCount => "defects",
        MeasurementKind::PassFail => "pass-fail",
    }
}

fn measurement_kind_from_slug(value: &str) -> Option<MeasurementKind> {
    match value {
        "thickness" => Some(MeasurementKind::ThicknessNm),
        "sheet-r" => Some(MeasurementKind::SheetResistanceOhmsPerSq),
        "cd" => Some(MeasurementKind::CriticalDimensionNm),
        "defects" => Some(MeasurementKind::DefectCount),
        "pass-fail" => Some(MeasurementKind::PassFail),
        _ => None,
    }
}

fn die_coord_label(die: Option<DieCoord>) -> String {
    die.map(|die| format!("C{} R{}", die.column, die.row))
        .unwrap_or_else(|| "None".to_string())
}

#[derive(Clone, Debug)]
struct ViewControlButton {
    name: String,
    label: String,
    selected: bool,
}

impl ViewControlButton {
    fn new(name: impl Into<String>, label: impl Into<String>, selected: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            selected,
        }
    }
}

#[derive(Clone, Debug)]
struct PrimaryRow {
    name: String,
    title: String,
    value: String,
    detail: String,
    action: Option<String>,
    selected: bool,
}

impl PrimaryRow {
    fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        value: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            value: value.into(),
            detail: detail.into(),
            action: None,
            selected: false,
        }
    }

    fn action(mut self, action: impl Into<String>, selected: bool) -> Self {
        self.action = Some(action.into());
        self.selected = selected;
        self
    }

    fn with_detail_suffix(mut self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        if !suffix.is_empty() {
            self.detail = format!("{}; {}", self.detail, suffix);
        }
        self
    }
}

fn add_view_controls(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    match app.active_view {
        StartupView::Layout2d | StartupView::Layout3d => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.layout",
            "Layout Editor Controls",
            layout_editor_control_rows(app),
            ui_scale,
        ),
        StartupView::Workflow => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.workflow",
            "Workflow Controls",
            workflow_control_rows(app),
            ui_scale,
        ),
        StartupView::MaskPrep => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.mask",
            "Reticle Prep Controls",
            mask_prep_control_rows(app),
            ui_scale,
        ),
        StartupView::LayoutDiff => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.layout_diff",
            "Layout Diff Controls",
            layout_diff_control_rows(app),
            ui_scale,
        ),
        StartupView::FabControl => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.fab",
            "Fab Control",
            fab_control_rows(app),
            ui_scale,
        ),
        StartupView::Maintenance => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.maintenance",
            "Maintenance Controls",
            maintenance_control_rows(app),
            ui_scale,
        ),
        StartupView::Environment => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.environment",
            "Environment Controls",
            environment_control_rows(app),
            ui_scale,
        ),
        StartupView::Inventory => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.inventory",
            "Inventory Controls",
            inventory_control_rows(app),
            ui_scale,
        ),
        StartupView::Scheduler => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.scheduler",
            "Dispatch Controls",
            scheduler_control_rows(app),
            ui_scale,
        ),
        StartupView::Safety => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.safety",
            "Safety Controls",
            safety_control_rows(app),
            ui_scale,
        ),
        StartupView::Traceability => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.trace",
            "Traceability Controls",
            traceability_control_rows(app),
            ui_scale,
        ),
        StartupView::ProcessFlow => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.process_flow",
            "Process Flow Controls",
            process_flow_control_rows(app),
            ui_scale,
        ),
        StartupView::ProcessControl => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.process_control",
            "Run-to-Run Controls",
            process_control_rows(app),
            ui_scale,
        ),
        StartupView::SpcFdc => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.spc",
            "SPC / FDC Controls",
            spc_fdc_control_rows(app),
            ui_scale,
        ),
        StartupView::CrossSection => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.cross_section",
            "Cross-Section Controls",
            cross_section_control_rows(app),
            ui_scale,
        ),
        StartupView::Metrology => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.metrology",
            "Metrology Controls",
            metrology_control_rows(app),
            ui_scale,
        ),
        StartupView::Yield => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.yield",
            "Yield Controls",
            yield_control_rows(app),
            ui_scale,
        ),
        StartupView::Experiment => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.experiment",
            "DOE Controls",
            experiment_control_rows(app),
            ui_scale,
        ),
        StartupView::Notebook => add_control_panel(
            document,
            parent,
            "fabricad.viewctl.notebook",
            "Notebook Controls",
            notebook_control_rows(app),
            ui_scale,
        ),
    }
}

fn add_primary_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    match app.active_view {
        StartupView::Layout2d | StartupView::Layout3d => {
            document.add_child(
                parent,
                UiNode::scene(
                    "fabricad.layout.preview",
                    layout_preview_primitives(
                        &app.workspace.document,
                        app.selected_layout_shape,
                        ui_scale,
                    ),
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(360.0)),
                    ),
                )
                .with_visual(UiVisual::panel(
                    ColorRgba::new(18, 24, 30, 255),
                    Some(StrokeStyle::new(
                        ColorRgba::new(54, 68, 82, 255),
                        ui_scale.value(1.0),
                    )),
                    ui_scale.value(4.0),
                )),
            );
        }
        StartupView::Workflow => add_workflow_view_panel(document, parent, app, ui_scale),
        StartupView::MaskPrep => add_mask_view_panel(document, parent, app, ui_scale),
        StartupView::LayoutDiff => add_layout_diff_view_panel(document, parent, app, ui_scale),
        StartupView::FabControl => add_fab_control_view_panel(document, parent, app, ui_scale),
        StartupView::Maintenance => add_maintenance_view_panel(document, parent, app, ui_scale),
        StartupView::Inventory => add_inventory_view_panel(document, parent, app, ui_scale),
        StartupView::Safety => add_safety_view_panel(document, parent, app, ui_scale),
        StartupView::Traceability => add_traceability_view_panel(document, parent, app, ui_scale),
        StartupView::CrossSection => add_cross_section_view_panel(document, parent, app, ui_scale),
        StartupView::ProcessFlow => add_process_flow_view_panel(document, parent, app, ui_scale),
        StartupView::Scheduler => add_scheduler_view_panel(document, parent, app, ui_scale),
        StartupView::Environment => add_environment_view_panel(document, parent, app, ui_scale),
        StartupView::Metrology => add_metrology_view_panel(document, parent, app, ui_scale),
        StartupView::Yield => add_yield_view_panel(document, parent, app, ui_scale),
        StartupView::SpcFdc => add_spc_fdc_view_panel(document, parent, app, ui_scale),
        StartupView::Experiment => add_experiment_view_panel(document, parent, app, ui_scale),
        StartupView::ProcessControl => {
            add_process_control_view_panel(document, parent, app, ui_scale)
        }
        StartupView::Notebook => add_notebook_view_panel(document, parent, app, ui_scale),
    }
}

fn add_workflow_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.workflow.dashboard",
        "Workflow Dashboard",
        workflow_dashboard_metrics(app),
        workflow_primary_rows(app),
        ui_scale,
    );
}

fn add_mask_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.mask.overview",
        "Mask Prep Overview",
        mask_dashboard_metrics(app),
        mask_primary_rows(app),
        ui_scale,
    );
}

fn add_layout_diff_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.layout_diff.overview",
        "Layout Diff Overview",
        layout_diff_dashboard_metrics(app),
        layout_diff_primary_rows(app),
        ui_scale,
    );
}

fn add_fab_control_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.fab_control.overview",
        "Fab Control Overview",
        fab_control_dashboard_metrics(app),
        fab_primary_rows(app),
        ui_scale,
    );
}

fn add_maintenance_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.maintenance.overview",
        "Maintenance Overview",
        maintenance_dashboard_metrics(app),
        maintenance_primary_rows(app),
        ui_scale,
    );
}

fn add_inventory_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.inventory.overview",
        "Inventory Overview",
        inventory_dashboard_metrics(app),
        inventory_primary_rows(app),
        ui_scale,
    );
}

fn add_safety_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.safety.overview",
        "Safety Overview",
        safety_dashboard_metrics(app),
        safety_primary_rows(app),
        ui_scale,
    );
}

fn add_traceability_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    add_overview_view_panel(
        document,
        parent,
        "fabricad.traceability.overview",
        "Traceability Overview",
        traceability_dashboard_metrics(app),
        traceability_primary_rows(app),
        ui_scale,
    );
}

fn add_overview_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    scene_name: &'static str,
    title: &'static str,
    metrics: Vec<DashboardMetric>,
    rows: Vec<PrimaryRow>,
    ui_scale: UiScale,
) {
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.primary.title",
        title,
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );

    let metric_row = document.add_child(
        panel,
        UiNode::container(
            "fabricad.primary.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(46.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, metric) in metrics.iter().take(4).enumerate() {
        add_dashboard_metric_cell(document, metric_row, index, metric, ui_scale);
    }

    document.add_child(
        panel,
        UiNode::scene(
            scene_name,
            overview_scene_primitives(&rows, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(210.0)),
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

    if rows.is_empty() {
        let empty = document.add_child(
            panel,
            UiNode::container(
                "fabricad.primary.row.0",
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
            ),
        );
        add_primary_cell(
            document,
            empty,
            "fabricad.primary.row.0.empty",
            "No rows match the active filters",
            1.0,
            false,
            ui_scale,
        );
        return;
    }

    for (row_index, row_data) in rows.iter().take(2).enumerate() {
        add_primary_summary_row(document, panel, row_index, row_data, ui_scale);
    }
}

fn add_primary_summary_row(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    row_index: usize,
    row_data: &PrimaryRow,
    ui_scale: UiScale,
) {
    let row_name = format!("fabricad.primary.row.{row_index}");
    let row = document.add_child(
        parent,
        UiNode::container(
            row_name.clone(),
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            if row_index % 2 == 0 {
                ColorRgba::new(26, 34, 42, 255)
            } else {
                ColorRgba::new(24, 31, 38, 255)
            },
            Some(StrokeStyle::new(
                ColorRgba::new(45, 57, 69, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(3.0),
        )),
    );
    if let Some(action) = row_data.action.as_ref() {
        let button_name = action
            .strip_prefix("fabricad.viewctl.")
            .map(|action| format!("fabricad.primary.action.{row_index}.{action}"))
            .unwrap_or_else(|| format!("{row_name}.action.{}", row_data.name));
        add_button(
            document,
            row,
            button_name,
            compact_button_label(&row_data.title, 24),
            row_data.selected,
            primary_cell_layout(1.4),
            ui_scale,
        );
    } else {
        add_primary_cell(
            document,
            row,
            format!("{row_name}.title.{}", row_data.name),
            compact_button_label(&row_data.title, 26),
            1.4,
            false,
            ui_scale,
        );
    }
    add_primary_cell(
        document,
        row,
        format!("{row_name}.value.{}", row_data.name),
        compact_button_label(&row_data.value, 22),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        row,
        format!("{row_name}.detail.{}", row_data.name),
        compact_button_label(&row_data.detail, 72),
        2.4,
        false,
        ui_scale,
    );
}

fn add_environment_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let environment = &app.workspace.environment;
    let Some(sensor) = selected_environment_sensor(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let active_alarm_count = environment.active_alarms().len();
    let zones = environment_zone_summaries(environment);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.environment.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.environment.primary.title",
        format!(
            "Cleanroom environment - {} sensors / {} zones / {} active alarms",
            environment.sensors.len(),
            zones.len(),
            active_alarm_count
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
            "fabricad.environment.trend",
            environment_view_primitives(app, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.environment.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.environment.metrics.selected",
        environment_selected_sensor_label(environment, sensor),
        1.4,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.environment.metrics.trend",
        environment
            .trend_summary(&sensor.id)
            .map(|summary| {
                format!(
                    "{} avg {:.2} {}",
                    summary.direction.label(),
                    summary.average_value,
                    sensor.unit
                )
            })
            .unwrap_or_else(|| "No trend".to_string()),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.environment.metrics.readings",
        format!(
            "{} readings / {} events",
            environment.readings.len(),
            environment.events.len()
        ),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.environment.metrics.alarms",
        format!(
            "{} active / {} total alarms",
            active_alarm_count,
            environment.alarms.len()
        ),
        1.0,
        false,
        ui_scale,
    );

    let detail = document.add_child(
        panel,
        UiNode::container(
            "fabricad.environment.detail",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        detail,
        "fabricad.environment.detail.zone",
        environment_selected_zone_label(&zones, &sensor.zone),
        1.3,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        detail,
        "fabricad.environment.detail.nominal",
        format!("Nominal {}", sensor.thresholds.nominal_label(&sensor.unit)),
        1.1,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        detail,
        "fabricad.environment.detail.correlation",
        environment_correlation_label(environment, &sensor.id),
        1.2,
        false,
        ui_scale,
    );

    let sensors = document.add_child(
        panel,
        UiNode::container(
            "fabricad.environment.sensors",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, sensor) in environment.sensors.iter().take(5).enumerate() {
        add_button(
            document,
            sensors,
            format!(
                "fabricad.primary.action.environment.{index}.environment.sensor.{}",
                sensor.id
            ),
            compact_button_label(&sensor.id, 12),
            app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}

fn add_metrology_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let map = &app.workspace.wafer_map;
    if map.dies.is_empty() {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    }
    let summary = map.summary(app.metrology_kind);
    let attention_sites = metrology_attention_sites(map, app.metrology_kind);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.metrology.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.metrology.primary.title",
        format!(
            "{} - {} / {}",
            map.name,
            app.metrology_map_mode.label(),
            app.metrology_kind.label()
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
            "fabricad.metrology.wafer_map",
            metrology_view_primitives(app, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.metrology.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.metrology.metrics.samples",
        format!("{} samples", summary.sample_count),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.metrology.metrics.status",
        format!(
            "{} pass / {} fail / {} outlier",
            summary.pass_count, summary.fail_count, summary.outlier_count
        ),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.metrology.metrics.center",
        metrology_summary_label(summary),
        1.2,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.metrology.metrics.defects",
        format!(
            "{} defects / {} notes",
            map.defects.len(),
            map.annotations.len()
        ),
        1.0,
        false,
        ui_scale,
    );

    let selected = document.add_child(
        panel,
        UiNode::container(
            "fabricad.metrology.selected",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.metrology.selected.die",
        metrology_selected_die_label(map, app.selected_die),
        2.2,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.metrology.selected.filters",
        if app.metrology_failed_only {
            "Showing failed/outlier sites"
        } else {
            "Showing all measured dies"
        },
        1.0,
        false,
        ui_scale,
    );

    let triage = document.add_child(
        panel,
        UiNode::container(
            "fabricad.metrology.triage",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    if attention_sites.is_empty() {
        add_primary_cell(
            document,
            triage,
            "fabricad.metrology.triage.clear",
            "No defects or measurement excursions",
            1.0,
            false,
            ui_scale,
        );
    } else {
        for (index, (die, score)) in attention_sites.iter().take(5).enumerate() {
            add_button(
                document,
                triage,
                format!("fabricad.viewctl.metrology.die.{}|{}", die.column, die.row),
                format!("{} score {score}", die_coord_label(Some(*die))),
                app.selected_die == Some(*die),
                primary_cell_layout(1.0),
                ui_scale,
            );
            if index == 4 {
                break;
            }
        }
    }
}

fn add_yield_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let analysis = &app.workspace.yield_analysis;
    let Some(lot_id) = app
        .selected_yield_lot
        .as_deref()
        .or_else(|| analysis.lots.first().map(|lot| lot.id.as_str()))
    else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let selected_wafer = app
        .selected_yield_wafer
        .as_deref()
        .map(str::to_string)
        .or_else(|| analysis.wafer_ids_for_lot(lot_id).first().cloned());
    let Some(wafer_id) = selected_wafer else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            view_primary_rows(app),
            ui_scale,
        );
        return;
    };
    let wafer_id = wafer_id.as_str();
    let lot_summary = analysis.lot_summary(lot_id);
    let wafer_summary = analysis.wafer_summary(lot_id, wafer_id);
    let outcomes = analysis.die_outcomes_for_wafer(lot_id, wafer_id);
    let measurements = analysis.measurements_for_wafer(lot_id, wafer_id);
    let wafer_rows = analysis.wafer_summaries_for_lot(lot_id);

    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.yield.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.yield.primary.title",
        format!(
            "Yield dashboard - {lot_id} / {wafer_id} / {}",
            app.yield_map_filter.label()
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
            "fabricad.yield.wafer_map",
            yield_view_primitives(app, lot_id, wafer_id, &outcomes, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.yield.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.yield.metrics.lot",
        lot_summary
            .map(|summary| format!("lot {}", percent_label(summary.yield_fraction)))
            .unwrap_or_else(|| "lot n/a".to_string()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.yield.metrics.wafer",
        wafer_summary
            .map(|summary| {
                format!(
                    "wafer {} / {} fail",
                    percent_label(summary.yield_fraction),
                    summary.failing_dies
                )
            })
            .unwrap_or_else(|| "wafer n/a".to_string()),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.yield.metrics.failure",
        wafer_summary
            .or(lot_summary)
            .map(yield_failure_label)
            .unwrap_or_else(|| "No failure mode".to_string()),
        1.4,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.yield.metrics.measurements",
        format!(
            "{} measurements / {} excursions",
            measurements.len(),
            yield_excursion_count(&measurements)
        ),
        1.0,
        false,
        ui_scale,
    );

    let selected = document.add_child(
        panel,
        UiNode::container(
            "fabricad.yield.selected",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.yield.selected.root_cause",
        wafer_summary
            .or(lot_summary)
            .and_then(|summary| summary.root_cause_hints.first())
            .map(|hint| compact_button_label(hint, 82))
            .unwrap_or_else(|| "No root-cause hint".to_string()),
        2.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.yield.selected.filters",
        format!(
            "{} / attention {} / excursions {}",
            app.yield_map_filter.label(),
            app.show_only_attention_wafers,
            app.show_only_excursions
        ),
        1.2,
        false,
        ui_scale,
    );

    let wafers = document.add_child(
        panel,
        UiNode::container(
            "fabricad.yield.wafers",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, summary) in wafer_rows
        .into_iter()
        .filter(|summary| yield_summary_matches(app, summary))
        .take(5)
        .enumerate()
    {
        let Some(row_wafer_id) = summary.wafer_id.as_deref() else {
            continue;
        };
        add_button(
            document,
            wafers,
            format!("fabricad.primary.action.yield.{index}.yield.wafer.{row_wafer_id}"),
            format!("{} {}", row_wafer_id, percent_label(summary.yield_fraction)),
            app.selected_yield_wafer.as_deref() == Some(row_wafer_id),
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}

fn add_experiment_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let plan = &app.workspace.experiment_plan;
    if plan.runs.is_empty() && plan.factors.is_empty() && plan.responses.is_empty() {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    }
    let primary_response_id = app.selected_experiment_response_if_valid();
    let analysis = plan.analysis_summary(primary_response_id.as_ref());
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.experiment.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.experiment.primary.title",
        format!(
            "{} - {} / owner {}",
            plan.title,
            plan.status.label(),
            plan.owner
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
            "fabricad.experiment.matrix",
            experiment_view_primitives(plan, &analysis, app.experiment_show_missing_only, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.experiment.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.experiment.metrics.runs",
        format!(
            "{} / {} complete",
            analysis.completed_runs, analysis.run_count
        ),
        0.9,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.experiment.metrics.pending",
        format!("{} pending values", analysis.missing_response_count),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.experiment.metrics.scope",
        format!(
            "{} factors / {} responses",
            plan.factors.len(),
            plan.responses.len()
        ),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.experiment.metrics.best",
        analysis
            .best_run_id
            .as_ref()
            .map(|run_id| format!("best observed {run_id}"))
            .unwrap_or_else(|| "best run pending".to_string()),
        1.2,
        false,
        ui_scale,
    );

    let note_row = document.add_child(
        panel,
        UiNode::container(
            "fabricad.experiment.note",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        note_row,
        "fabricad.experiment.note.objective",
        compact_button_label(&plan.objective, 96),
        2.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        note_row,
        "fabricad.experiment.note.readiness",
        experiment_readiness_label(&analysis, plan.responses.len()),
        1.0,
        false,
        ui_scale,
    );

    let actions = document.add_child(
        panel,
        UiNode::container(
            "fabricad.experiment.actions",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, (action, label, selected)) in [
        (
            "experiment.pending_only",
            "Pending",
            app.experiment_show_missing_only,
        ),
        ("experiment.next_pending", "Next", false),
        ("experiment.capture_next_demo", "Next demo", false),
        ("experiment.use_target", "Target", false),
    ]
    .into_iter()
    .enumerate()
    {
        add_button(
            document,
            actions,
            format!("fabricad.primary.action.experiment.{index}.{action}"),
            label,
            selected,
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}

fn add_process_control_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let Some(loop_definition) = selected_control_loop(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let trend = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id);
    let actions = app
        .workspace
        .process_control
        .actions_for_loop(&loop_definition.id);
    let selected_action =
        selected_control_action(app).filter(|action| action.loop_id == loop_definition.id);
    let latest = trend.last();
    let audit_count = app
        .workspace
        .process_control
        .audit_for_loop(&loop_definition.id)
        .len();

    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.process_control.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(414.0)),
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
        "fabricad.process_control.primary.title",
        format!(
            "{} - {} on {}",
            loop_definition.name, loop_definition.output.measurement_name, loop_definition.tool_id
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
            "fabricad.process_control.trend",
            process_control_view_primitives(app, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(238.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.process_control.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.process_control.metrics.latest",
        latest
            .map(|point| {
                format!(
                    "latest {:.3} {} error {:+.3}",
                    point.value, point.unit, point.error
                )
            })
            .unwrap_or_else(|| "No trend samples".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.process_control.metrics.target",
        format!(
            "target {:.3} +/-{:.3} {}",
            loop_definition.output.target,
            loop_definition.deadband.abs(),
            loop_definition.output.unit
        ),
        1.2,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.process_control.metrics.actions",
        format!("{} actions / {audit_count} audits", actions.len()),
        1.0,
        false,
        ui_scale,
    );

    let detail = document.add_child(
        panel,
        UiNode::container(
            "fabricad.process_control.detail",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(42.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        detail,
        "fabricad.process_control.detail.action",
        selected_action
            .map(process_control_action_label)
            .unwrap_or_else(|| "No selected recommendation".to_string()),
        1.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        detail,
        "fabricad.process_control.detail.yield",
        process_control_yield_link_label(app, loop_definition),
        1.4,
        false,
        ui_scale,
    );

    let loops = document.add_child(
        panel,
        UiNode::container(
            "fabricad.process_control.loops",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(38.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    for (index, loop_definition) in app
        .workspace
        .process_control
        .loops
        .iter()
        .take(5)
        .enumerate()
    {
        add_button(
            document,
            loops,
            format!(
                "fabricad.primary.action.process_control.{index}.process_control.loop.{}",
                loop_definition.id
            ),
            compact_button_label(&loop_definition.name, 16),
            app.selected_control_loop.as_ref() == Some(&loop_definition.id),
            primary_cell_layout(1.0),
            ui_scale,
        );
    }
}

fn add_spc_fdc_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let monitor = spc_fdc_monitor(&app.workspace);
    let selected_chart = selected_spc_chart(app, &monitor);
    let selected_trace = selected_fdc_trace(app, &monitor);
    let chart_violation_count = monitor
        .charts
        .iter()
        .map(|chart| chart.violations.len())
        .sum::<usize>();
    let trace_excursion_count = monitor
        .traces
        .iter()
        .map(|trace| trace.violations.len())
        .sum::<usize>();

    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.spc.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(386.0)),
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
        "fabricad.spc.primary.title",
        format!(
            "SPC / FDC monitor - {} findings across {} charts / {} traces",
            monitor.findings.len(),
            monitor.charts.len(),
            monitor.traces.len()
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
            "fabricad.spc.chart",
            spc_fdc_monitor_primitives(selected_chart, selected_trace, &monitor, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(226.0)),
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

    let metrics = document.add_child(
        panel,
        UiNode::container(
            "fabricad.spc.metrics",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(46.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.spc.metrics.critical",
        format!(
            "{} critical",
            monitor.finding_count_by_severity(MonitorSeverity::Critical)
        ),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.spc.metrics.warning",
        format!(
            "{} warning",
            monitor.finding_count_by_severity(MonitorSeverity::Warning)
        ),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.spc.metrics.spc",
        format!("{chart_violation_count} SPC rule hits"),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.spc.metrics.fdc",
        format!("{trace_excursion_count} FDC excursions"),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        metrics,
        "fabricad.spc.metrics.alarms",
        format!("{} active alarms", monitor.alarm_summary.active_count),
        1.0,
        false,
        ui_scale,
    );

    let selected = document.add_child(
        panel,
        UiNode::container(
            "fabricad.spc.selected",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(46.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.spc.selected.chart",
        selected_chart
            .map(spc_chart_summary_label)
            .unwrap_or_else(|| "No selected SPC chart".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.spc.selected.trace",
        selected_trace
            .map(|trace| spc_trace_summary_label(&monitor, trace))
            .unwrap_or_else(|| "No selected FDC trace".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        selected,
        "fabricad.spc.selected.filters",
        format!(
            "{} / {} / {} filtered",
            app.spc_severity_filter.label(),
            app.spc_source_filter.label(),
            spc_filtered_finding_count(app, &monitor)
        ),
        1.0,
        false,
        ui_scale,
    );
}

fn add_scheduler_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let result = app.workspace.scheduler.dispatch(app.scheduler_policy);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.scheduler.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(352.0)),
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
        "fabricad.scheduler.primary.title",
        format!(
            "{} dispatch - {} waiting lots / {} tools",
            app.scheduler_policy.label(),
            app.workspace.scheduler.lots.len(),
            app.workspace.scheduler.tools.len()
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
            "fabricad.scheduler.timeline",
            scheduler_timeline_primitives(app, &result, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(226.0)),
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

    let queue = result
        .queue_summaries
        .iter()
        .max_by_key(|summary| summary.total_process_minutes);
    let summary = document.add_child(
        panel,
        UiNode::container(
            "fabricad.scheduler.summary",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(54.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.scheduler.summary.assignments",
        format!("{} assignments", result.assignments.len()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.scheduler.summary.unscheduled",
        format!("{} unscheduled", result.unscheduled_lots.len()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.scheduler.summary.queue",
        queue
            .map(|summary| {
                format!(
                    "{} bottleneck: {} lots / {} min",
                    summary.tool_class.label(),
                    summary.waiting_lots,
                    summary.total_process_minutes
                )
            })
            .unwrap_or_else(|| "No queued bottleneck".to_string()),
        1.6,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.scheduler.summary.selected",
        app.selected_scheduler_tool
            .as_ref()
            .map(|tool| format!("Selected {tool}"))
            .unwrap_or_else(|| "Selected none".to_string()),
        1.0,
        false,
        ui_scale,
    );
}

fn add_notebook_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let Some(entry) = selected_notebook_entry(app) else {
        add_primary_data_panel(
            document,
            parent,
            app.active_view.label(),
            Vec::new(),
            ui_scale,
        );
        return;
    };
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.notebook.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(386.0)),
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
        "fabricad.notebook.primary.title",
        entry.title.clone(),
        text_style(
            ui_scale.value(16.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
    );

    let meta = document.add_child(
        panel,
        UiNode::container(
            "fabricad.notebook.meta",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(40.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        meta,
        "fabricad.notebook.meta.author",
        format!("{} - updated {}", entry.author, entry.updated_at),
        1.1,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        meta,
        "fabricad.notebook.meta.mode",
        if app.notebook_preview_mode {
            "Preview mode"
        } else {
            "Edit mode"
        },
        0.7,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        meta,
        "fabricad.notebook.meta.tags",
        notebook_tag_summary(entry),
        1.2,
        false,
        ui_scale,
    );

    add_text(
        document,
        panel,
        "fabricad.notebook.body",
        compact_button_label(&entry.body_markdown.replace('\n', " "), 360),
        text_style(
            ui_scale.value(13.0),
            FontWeight::NORMAL,
            ColorRgba::new(188, 198, 207, 255),
        ),
        layout::with_padding_all(
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(112.0))),
            ui_scale.value(8.0),
        ),
    );

    let links = document.add_child(
        panel,
        UiNode::container(
            "fabricad.notebook.links",
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
        links,
        "fabricad.notebook.links.count",
        format!("{} linked objects", entry.link_count()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        links,
        "fabricad.notebook.links.summary",
        notebook_link_summary(entry),
        2.2,
        false,
        ui_scale,
    );

    add_text(
        document,
        panel,
        "fabricad.notebook.related.title",
        "Related entries",
        text_style(
            ui_scale.value(13.0),
            FontWeight::BOLD,
            ColorRgba::new(136, 207, 190, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
    for (index, related) in notebook_related_entries(app, entry)
        .iter()
        .take(3)
        .enumerate()
    {
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("fabricad.notebook.related.row.{index}"),
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(34.0)),
                    ),
                    ui_scale.value(8.0),
                ),
            ),
        );
        add_primary_cell(
            document,
            row,
            format!("fabricad.notebook.related.row.{index}.title"),
            compact_button_label(&related.0, 30),
            1.3,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("fabricad.notebook.related.row.{index}.reason"),
            compact_button_label(&related.1, 70),
            2.0,
            false,
            ui_scale,
        );
    }
}

fn add_process_flow_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let route = &app.workspace.process_flow.route;
    let findings = app.workspace.process_flow.findings();
    let selected = selected_process_flow_node(app);
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.process_flow.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(350.0)),
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
        "fabricad.process_flow.primary.title",
        format!("{} v{} - {}", route.name, route.version, route.owner),
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
            "fabricad.process_flow.timeline",
            process_flow_timeline_primitives(app, &findings, ui_scale),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(232.0)),
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

    let summary = document.add_child(
        panel,
        UiNode::container(
            "fabricad.process_flow.summary",
            layout::with_gap_all(
                layout::with_size(
                    layout::row(),
                    layout::percent(1.0),
                    layout::px(ui_scale.value(54.0)),
                ),
                ui_scale.value(8.0),
            ),
        ),
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.process_flow.summary.selected",
        selected
            .map(|node| format!("Selected {} {}", node.id, node.name))
            .unwrap_or_else(|| "Selected none".to_string()),
        1.5,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.process_flow.summary.dependencies",
        selected
            .map(|node| {
                let incoming = route.edges.iter().filter(|edge| edge.to == node.id).count();
                let outgoing = route
                    .edges
                    .iter()
                    .filter(|edge| edge.from == node.id)
                    .count();
                format!("{incoming} in / {outgoing} out")
            })
            .unwrap_or_else(|| "0 in / 0 out".to_string()),
        0.8,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.process_flow.summary.controls",
        format!(
            "{} checkpoints / {} holds",
            route
                .nodes
                .iter()
                .filter(|node| node.measurement_checkpoint.is_some())
                .count(),
            route
                .nodes
                .iter()
                .filter(|node| node.hold_point || node.kind == ProcessFlowNodeKind::Hold)
                .count()
        ),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.process_flow.summary.findings",
        format!("{} findings", findings.len()),
        0.7,
        false,
        ui_scale,
    );
}

fn add_cross_section_view_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
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
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.cross_section.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(430.0)),
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
        "fabricad.cross_section.primary.title",
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
            "fabricad.cross_section.preview",
            cross_section_preview_primitives(
                process,
                snapshot,
                selected_material.as_ref(),
                CrossSectionPreviewOptions {
                    show_mask_overlay: app.cross_section_show_mask,
                    show_dimension_guides: app.cross_section_show_dimensions,
                    show_risk_cues: app.cross_section_show_risks,
                },
                ui_scale,
            ),
            layout::with_size(
                layout::row(),
                layout::percent(1.0),
                layout::px(ui_scale.value(320.0)),
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

    let summary = document.add_child(
        panel,
        UiNode::container(
            "fabricad.cross_section.summary",
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
        "fabricad.cross_section.summary.height",
        format!(
            "Stack {} max, {} min",
            format_um_f32(surface.max_um),
            format_um_f32(surface.min_um)
        ),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.cross_section.summary.surface",
        format!(
            "Surface {} avg, {} range",
            format_um_f32(surface.average_um),
            format_um_f32(surface.range_um)
        ),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.cross_section.summary.mask",
        format!("Mask open {:.0}%", mask_coverage.clamp(0.0, 1.0) * 100.0),
        1.0,
        false,
        ui_scale,
    );
    add_primary_cell(
        document,
        summary,
        "fabricad.cross_section.summary.material",
        selected_material
            .as_ref()
            .and_then(|material| process.material(material))
            .map(|material| format!("Material {}", material.name))
            .unwrap_or_else(|| "Material n/a".to_string()),
        1.0,
        false,
        ui_scale,
    );
}

fn add_primary_data_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    title: &str,
    rows: Vec<PrimaryRow>,
    ui_scale: UiScale,
) {
    let row_height = ui_scale.value(36.0);
    let gap = ui_scale.value(6.0);
    let visible_rows = rows.len().max(1).min(12);
    let height = ui_scale.value(70.0)
        + row_height * visible_rows as f32
        + gap * visible_rows.saturating_sub(1) as f32;
    let panel = document.add_child(
        parent,
        UiNode::container(
            "fabricad.primary",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    gap,
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
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );

    add_text(
        document,
        panel,
        "fabricad.primary.title",
        format!("{title} Primary View"),
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );

    let header = document.add_child(
        panel,
        UiNode::container(
            "fabricad.primary.header",
            layout::with_gap_all(
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
                gap,
            ),
        ),
    );
    add_primary_cell(
        document,
        header,
        "fabricad.primary.header.title",
        "Item",
        1.6,
        true,
        ui_scale,
    );
    add_primary_cell(
        document,
        header,
        "fabricad.primary.header.value",
        "State",
        1.0,
        true,
        ui_scale,
    );
    add_primary_cell(
        document,
        header,
        "fabricad.primary.header.detail",
        "Detail",
        2.2,
        true,
        ui_scale,
    );

    if rows.is_empty() {
        let empty = document.add_child(
            panel,
            UiNode::container(
                "fabricad.primary.row.0",
                layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
            ),
        );
        add_primary_cell(
            document,
            empty,
            "fabricad.primary.row.0.empty",
            "No rows match the active filters",
            1.0,
            false,
            ui_scale,
        );
        return;
    }

    for (row_index, row_data) in rows.iter().take(12).enumerate() {
        let row_name = format!("fabricad.primary.row.{row_index}");
        let row = document.add_child(
            panel,
            UiNode::container(
                row_name.clone(),
                layout::with_gap_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
                    gap,
                ),
            )
            .with_visual(UiVisual::panel(
                if row_index % 2 == 0 {
                    ColorRgba::new(26, 34, 42, 255)
                } else {
                    ColorRgba::new(24, 31, 38, 255)
                },
                Some(StrokeStyle::new(
                    ColorRgba::new(45, 57, 69, 255),
                    ui_scale.value(1.0),
                )),
                ui_scale.value(3.0),
            )),
        );
        if let Some(action) = row_data.action.as_ref() {
            let button_name = action
                .strip_prefix("fabricad.viewctl.")
                .map(|action| format!("fabricad.primary.action.{row_index}.{action}"))
                .unwrap_or_else(|| format!("{row_name}.action.{}", row_data.name));
            add_button(
                document,
                row,
                button_name,
                compact_button_label(&row_data.title, 24),
                row_data.selected,
                primary_cell_layout(1.6),
                ui_scale,
            );
        } else {
            add_primary_cell(
                document,
                row,
                format!("{row_name}.title.{}", row_data.name),
                compact_button_label(&row_data.title, 28),
                1.6,
                false,
                ui_scale,
            );
        }
        add_primary_cell(
            document,
            row,
            format!("{row_name}.value.{}", row_data.name),
            compact_button_label(&row_data.value, 22),
            1.0,
            false,
            ui_scale,
        );
        add_primary_cell(
            document,
            row,
            format!("{row_name}.detail.{}", row_data.name),
            compact_button_label(&row_data.detail, 72),
            2.2,
            false,
            ui_scale,
        );
    }
}

fn primary_cell_layout(weight: f32) -> operad::LayoutStyle {
    layout::with_size(
        layout::flex_item(weight, 1.0, layout::px(0.0)),
        layout::auto(),
        layout::percent(1.0),
    )
}

fn add_primary_cell(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    weight: f32,
    header: bool,
    ui_scale: UiScale,
) {
    add_text(
        document,
        parent,
        name,
        text,
        text_style(
            ui_scale.value(if header { 12.0 } else { 13.0 }),
            if header {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            },
            if header {
                ColorRgba::new(136, 207, 190, 255)
            } else {
                ColorRgba::new(182, 193, 203, 255)
            },
        ),
        layout::with_padding_all(primary_cell_layout(weight), ui_scale.value(7.0)),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverviewTone {
    Danger,
    Warning,
    Success,
    Info,
    Neutral,
}

#[derive(Clone, Debug)]
struct DashboardMetric {
    label: String,
    value: String,
    detail: String,
    tone: OverviewTone,
}

impl DashboardMetric {
    fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        detail: impl Into<String>,
        tone: OverviewTone,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            detail: detail.into(),
            tone,
        }
    }
}

fn add_dashboard_metric_cell(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    index: usize,
    metric: &DashboardMetric,
    ui_scale: UiScale,
) {
    add_text(
        document,
        parent,
        format!("fabricad.primary.metrics.{index}"),
        compact_button_label(
            &format!("{}: {} - {}", metric.label, metric.value, metric.detail),
            54,
        ),
        text_style(
            ui_scale.value(12.0),
            FontWeight::BOLD,
            overview_tone_color(metric.tone, 255),
        ),
        layout::with_padding_all(primary_cell_layout(1.0), ui_scale.value(7.0)),
    );
}

fn overview_scene_primitives(rows: &[PrimaryRow], ui_scale: UiScale) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(rows.len() * 8 + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let row_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(24.0),
        ui_scale.value(430.0),
        ui_scale.value(154.0),
    );
    let status_bounds = UiRect::new(
        frame.x + ui_scale.value(492.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(172.0),
        ui_scale.value(154.0),
    );
    let matrix_bounds = UiRect::new(
        frame.x + ui_scale.value(704.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(174.0),
        ui_scale.value(154.0),
    );
    add_overview_row_primitives(&mut primitives, row_bounds, rows, ui_scale);
    add_overview_status_primitives(&mut primitives, status_bounds, rows, ui_scale);
    add_overview_matrix_primitives(&mut primitives, matrix_bounds, rows, ui_scale);
    primitives
}

fn add_overview_row_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    rows: &[PrimaryRow],
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if rows.is_empty() {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }
    let visible = rows.len().min(8);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, row_data) in rows.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        let tone = overview_row_tone(row_data);
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                row,
                if row_data.selected {
                    ColorRgba::new(72, 128, 188, 72)
                } else {
                    ColorRgba::new(25, 32, 39, 255)
                },
            )
            .stroke(StrokeStyle::new(
                if row_data.selected {
                    ColorRgba::new(252, 253, 255, 210)
                } else {
                    ColorRgba::new(41, 52, 63, 180)
                },
                ui_scale.value(if row_data.selected { 1.2 } else { 0.7 }),
            )),
        ));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                row.x,
                row.y,
                row.width * overview_row_ratio(row_data, index),
                row.height,
            ),
            overview_tone_color(tone, 155),
        )));
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(row.x + ui_scale.value(9.0), row.y + row.height * 0.5),
            radius: ui_scale.value(3.6),
            fill: overview_tone_color(tone, 245),
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(0.8),
            )),
        });
    }
}

fn add_overview_status_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    rows: &[PrimaryRow],
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if rows.is_empty() {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }
    let tones = [
        OverviewTone::Danger,
        OverviewTone::Warning,
        OverviewTone::Success,
        OverviewTone::Info,
        OverviewTone::Neutral,
    ];
    let total = rows.len().max(1) as f32;
    let gap = ui_scale.value(6.0);
    let row_height =
        (bounds.height - gap * tones.len().saturating_sub(1) as f32) / tones.len() as f32;
    for (index, tone) in tones.iter().enumerate() {
        let count = rows
            .iter()
            .filter(|row_data| overview_row_tone(row_data) == *tone)
            .count();
        let y = bounds.y + index as f32 * (row_height + gap);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, bounds.width, row_height),
            ColorRgba::new(25, 32, 39, 255),
        )));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x,
                y,
                (bounds.width * count as f32 / total).max(ui_scale.value(2.0)),
                row_height,
            ),
            overview_tone_color(*tone, 210),
        )));
    }
}

fn add_overview_matrix_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    rows: &[PrimaryRow],
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if rows.is_empty() {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }
    let columns = 6usize;
    let visible = rows.len().min(18);
    let row_count = visible.div_ceil(columns).max(1);
    let gap = ui_scale.value(4.0);
    let cell_width = (bounds.width - gap * columns.saturating_sub(1) as f32) / columns as f32;
    let cell_height = (bounds.height - gap * row_count.saturating_sub(1) as f32) / row_count as f32;
    for (index, row_data) in rows.iter().take(visible).enumerate() {
        let column = index % columns;
        let row = index / columns;
        let rect = UiRect::new(
            bounds.x + column as f32 * (cell_width + gap),
            bounds.y + row as f32 * (cell_height + gap),
            cell_width,
            cell_height,
        );
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(rect, overview_tone_color(overview_row_tone(row_data), 215))
                .stroke(StrokeStyle::new(
                    if row_data.selected {
                        ColorRgba::new(252, 253, 255, 230)
                    } else {
                        ColorRgba::new(16, 20, 24, 140)
                    },
                    ui_scale.value(if row_data.selected { 1.4 } else { 0.8 }),
                )),
        ));
    }
}

fn overview_row_tone(row: &PrimaryRow) -> OverviewTone {
    let text = format!("{} {} {}", row.title, row.value, row.detail).to_lowercase();
    if text.contains("critical")
        || text.contains("locked")
        || text.contains("overdue")
        || text.contains("error")
        || text.contains("alarm")
        || text.contains("fail")
        || text.contains("blocked")
        || text.contains("excursion")
    {
        OverviewTone::Danger
    } else if text.contains("warning")
        || text.contains("due")
        || text.contains("risk")
        || text.contains("hold")
        || text.contains("maintenance")
        || text.contains("changed")
    {
        OverviewTone::Warning
    } else if text.contains("complete")
        || text.contains("available")
        || text.contains("online")
        || text.contains("running")
        || text.contains("pass")
        || text.contains("nominal")
        || text.contains("ready")
    {
        OverviewTone::Success
    } else if text.contains("selected")
        || text.contains("lineage")
        || text.contains("recipe")
        || text.contains("yield")
        || text.contains("wafer")
    {
        OverviewTone::Info
    } else {
        OverviewTone::Neutral
    }
}

fn overview_tone_color(tone: OverviewTone, alpha: u8) -> ColorRgba {
    match tone {
        OverviewTone::Danger => ColorRgba::new(236, 91, 88, alpha),
        OverviewTone::Warning => ColorRgba::new(238, 181, 82, alpha),
        OverviewTone::Success => ColorRgba::new(91, 190, 130, alpha),
        OverviewTone::Info => ColorRgba::new(93, 168, 232, alpha),
        OverviewTone::Neutral => ColorRgba::new(130, 145, 160, alpha),
    }
}

fn overview_row_ratio(row: &PrimaryRow, index: usize) -> f32 {
    if row.selected {
        return 1.0;
    }
    let hash = row
        .title
        .bytes()
        .chain(row.value.bytes())
        .chain(row.detail.bytes())
        .fold(index as u32 + 17, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(byte as u32)
        });
    0.32 + (hash % 58) as f32 / 100.0
}

fn workflow_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let dispatch = workspace.scheduler.dispatch(app.scheduler_policy);
    vec![
        DashboardMetric::new(
            "Layout",
            workspace
                .document
                .flattened_shape_count_estimate()
                .to_string(),
            format!("{} layers", workspace.document.layers.len()),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Flow",
            workspace.process_flow.route.nodes.len().to_string(),
            format!("{} findings", workspace.process_flow.findings().len()),
            if process_flow_error_count(workspace) > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "WIP",
            workspace.mes.lots.len().to_string(),
            format!("{} dispatches", dispatch.assignments.len()),
            OverviewTone::Warning,
        ),
        DashboardMetric::new(
            "Factory",
            workspace.equipment.tools().count().to_string(),
            format!(
                "{} running / {} alarms",
                equipment_running_count(workspace),
                equipment_alarm_count(workspace)
            ),
            if equipment_alarm_count(workspace) > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Quality",
            workflow_focus_yield_label(app),
            format!("{} notebook entries", workspace.lab_notebook.entries.len()),
            OverviewTone::Info,
        ),
    ]
}

fn mask_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let prep = reticle_prep_for_app(app);
    let report = mask_check_report(app);
    let filtered = mask_filtered_issue_count(app, &report);
    vec![
        DashboardMetric::new(
            "Reticle",
            prep.reticle.id.to_string(),
            format!("{} fields", report.field_count),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Issues",
            filtered.to_string(),
            format!(
                "{} errors / {} warnings",
                report.error_count(),
                report.warning_count()
            ),
            if report.error_count() > 0 {
                OverviewTone::Danger
            } else if report.warning_count() > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Layers",
            report.layer_count.to_string(),
            format!("{} printable shapes", report.printable_shape_count),
            OverviewTone::Neutral,
        ),
        DashboardMetric::new(
            "Grouping",
            app.mask_issue_grouping.label(),
            format!("{} groups", mask_issue_group_count(app, &report)),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Exposures",
            report.exposure_block_count.to_string(),
            app.mask_source_lot
                .as_ref()
                .map(|lot| format!("linked {lot}"))
                .unwrap_or_else(|| "unlinked lot".to_string()),
            OverviewTone::Neutral,
        ),
    ]
}

fn layout_diff_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let report = layout_diff_report(app);
    vec![
        DashboardMetric::new(
            "Changes",
            layout_diff_filtered_change_count(app, &report).to_string(),
            format!("{} total", layout_diff_total_changes(&report)),
            if report.summary.modified_shapes + report.summary.removed_shapes > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Info
            },
        ),
        DashboardMetric::new(
            "Added",
            report.summary.added_shapes.to_string(),
            format!("{} removed", report.summary.removed_shapes),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Modified",
            report.summary.modified_shapes.to_string(),
            format!("{} layers", report.layers.len()),
            if report.summary.modified_shapes > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Sources",
            app.layout_diff_baseline.label(),
            format!("candidate {}", app.layout_diff_candidate.label()),
            OverviewTone::Neutral,
        ),
        DashboardMetric::new(
            "Review",
            app.layout_diff_review_state.label(),
            app.layout_diff_change_filter.detail_label(),
            OverviewTone::Info,
        ),
    ]
}

fn fab_control_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let tools = workspace.equipment.tools().count();
    let selected = selected_equipment_tool(app);
    vec![
        DashboardMetric::new(
            "Tools",
            tools.to_string(),
            format!("{} running", equipment_running_count(workspace)),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Alarms",
            equipment_alarm_count(workspace).to_string(),
            "active equipment alarms",
            if equipment_alarm_count(workspace) > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Telemetry",
            equipment_sample_count(workspace).to_string(),
            "recent sensor samples",
            OverviewTone::Neutral,
        ),
        DashboardMetric::new(
            "Selected",
            selected
                .map(|tool| tool.id.to_string())
                .unwrap_or_else(|| "None".to_string()),
            selected
                .map(|tool| tool.state.label().to_string())
                .unwrap_or_else(|| "no tool selected".to_string()),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Recipe",
            selected
                .map(equipment_recipe_summary)
                .unwrap_or_else(|| "n/a".to_string()),
            "loaded/draft recipe",
            OverviewTone::Neutral,
        ),
    ]
}

fn maintenance_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let today = maintenance_today(workspace);
    let actionable = maintenance_filtered_work_count(workspace, MaintenanceWorkFilter::Actionable);
    let overdue = workspace
        .maintenance
        .tools
        .iter()
        .flat_map(|tool| tool.schedules.iter().map(move |schedule| (tool, schedule)))
        .filter(|(tool, schedule)| schedule.due_state(today, tool.run_count) == DueState::Overdue)
        .count();
    vec![
        DashboardMetric::new(
            "Queue",
            actionable.to_string(),
            format!("{overdue} overdue"),
            if overdue > 0 {
                OverviewTone::Danger
            } else if actionable > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Tools",
            workspace.maintenance.tools.len().to_string(),
            format!("{} locked", maintenance_locked_count(workspace)),
            if maintenance_locked_count(workspace) > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Calibration",
            workspace.maintenance.calibration_records.len().to_string(),
            format!(
                "{} quals",
                workspace.maintenance.qualification_results.len()
            ),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Downtime",
            workspace.maintenance.downtime.len().to_string(),
            format!("audit date {today}"),
            OverviewTone::Neutral,
        ),
        DashboardMetric::new(
            "Selected",
            selected_maintenance_tool(app)
                .map(|tool| tool.tool_id.to_string())
                .unwrap_or_else(|| "None".to_string()),
            app.maintenance_work_filter.detail_label(),
            OverviewTone::Info,
        ),
    ]
}

fn inventory_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let workspace = &app.workspace;
    let alerts = workspace.inventory.alerts(INVENTORY_DEMO_TODAY);
    vec![
        DashboardMetric::new(
            "Lots",
            workspace.inventory.lots.len().to_string(),
            format!(
                "{} visible",
                inventory_filter_count(workspace, app.inventory_filter)
            ),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Alerts",
            alerts.len().to_string(),
            "stock and expiry",
            if alerts.is_empty() {
                OverviewTone::Success
            } else {
                OverviewTone::Warning
            },
        ),
        DashboardMetric::new(
            "Low Stock",
            inventory_filter_count(workspace, InventoryQuickFilter::LowStock).to_string(),
            "lots below reorder",
            if inventory_filter_count(workspace, InventoryQuickFilter::LowStock) > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Hold",
            inventory_filter_count(workspace, InventoryQuickFilter::ProductHold).to_string(),
            format!("today {INVENTORY_DEMO_TODAY}"),
            if inventory_filter_count(workspace, InventoryQuickFilter::ProductHold) > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Selected",
            selected_inventory_lot(app)
                .map(|lot| lot.id.to_string())
                .unwrap_or_else(|| "None".to_string()),
            app.inventory_filter.detail_label(),
            OverviewTone::Info,
        ),
    ]
}

fn safety_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let summary = app.workspace.safety.summary();
    vec![
        DashboardMetric::new(
            "State",
            summary
                .highest_severity
                .map(SafetySeverity::label)
                .unwrap_or("normal"),
            format!("{} active conditions", summary.active_condition_count),
            if summary.active_condition_count > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Lockouts",
            summary.locked_out_tool_count.to_string(),
            "tool interlocks",
            if summary.locked_out_tool_count > 0 {
                OverviewTone::Danger
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Incidents",
            summary.open_incident_count.to_string(),
            format!("{} total", app.workspace.safety.incidents.len()),
            if summary.open_incident_count > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Sensors",
            summary.sensor_count.to_string(),
            format!("{} routes", app.workspace.safety.alarm_routes.len()),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Ack",
            app.acknowledged_count().to_string(),
            app.selected_safety_tool
                .as_deref()
                .unwrap_or("no selected tool")
                .to_string(),
            OverviewTone::Neutral,
        ),
    ]
}

fn traceability_dashboard_metrics(app: &FabricadApp) -> Vec<DashboardMetric> {
    let summary = app.workspace.genealogy.summary();
    vec![
        DashboardMetric::new(
            "Lots",
            summary.lot_count.to_string(),
            format!("{} wafers", summary.wafer_count),
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Lineage",
            trace_related_wafer_count(app).to_string(),
            "related wafers",
            OverviewTone::Info,
        ),
        DashboardMetric::new(
            "Impact",
            trace_impact_count(app).to_string(),
            app.trace_impact_mode.label(),
            if trace_impact_count(app) > 0 {
                OverviewTone::Warning
            } else {
                OverviewTone::Success
            },
        ),
        DashboardMetric::new(
            "Records",
            summary.process_record_count.to_string(),
            format!("{} material lots", summary.material_lot_count),
            OverviewTone::Neutral,
        ),
        DashboardMetric::new(
            "Events",
            app.workspace.genealogy.events.len().to_string(),
            format!(
                "{} splits / {} merges",
                summary.split_count, summary.merge_count
            ),
            OverviewTone::Neutral,
        ),
    ]
}

fn view_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    match app.active_view {
        StartupView::Workflow => workflow_primary_rows(app),
        StartupView::Layout2d | StartupView::Layout3d => Vec::new(),
        StartupView::MaskPrep => mask_primary_rows(app),
        StartupView::LayoutDiff => layout_diff_primary_rows(app),
        StartupView::FabControl => fab_primary_rows(app),
        StartupView::Maintenance => maintenance_primary_rows(app),
        StartupView::Environment => environment_primary_rows(app),
        StartupView::Inventory => inventory_primary_rows(app),
        StartupView::Scheduler => scheduler_primary_rows(app),
        StartupView::Safety => safety_primary_rows(app),
        StartupView::Traceability => traceability_primary_rows(app),
        StartupView::ProcessFlow => process_flow_primary_rows(app),
        StartupView::ProcessControl => process_control_primary_rows(app),
        StartupView::SpcFdc => spc_fdc_primary_rows(app),
        StartupView::CrossSection => cross_section_primary_rows(app),
        StartupView::Metrology => metrology_primary_rows(app),
        StartupView::Yield => yield_primary_rows(app),
        StartupView::Experiment => experiment_primary_rows(app),
        StartupView::Notebook => notebook_primary_rows(app),
    }
}

fn workflow_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let mut rows = Vec::new();
    if let Some(lot_id) = app
        .workflow_focus_lot
        .clone()
        .or_else(|| workflow_lot_ids(&app.workspace).into_iter().next())
    {
        rows.push(
            PrimaryRow::new(
                format!("workflow-focus-{lot_id}"),
                format!("Focus lot {lot_id}"),
                workflow_focus_traveler_label(app),
                workflow_focus_yield_label(app),
            )
            .action(
                format!("fabricad.viewctl.workflow.focus_lot.{lot_id}"),
                app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
            ),
        );
    }
    rows.extend(
        workflow_lot_ids(&app.workspace)
            .into_iter()
            .take(3)
            .map(|lot_id| {
                let lot = app
                    .workspace
                    .mes
                    .lots
                    .values()
                    .find(|lot| lot.id.as_str() == lot_id);
                let traveler = app
                    .workspace
                    .mes
                    .travelers
                    .values()
                    .find(|traveler| traveler.lot_id.as_str() == lot_id);
                let value = traveler
                    .map(|traveler| {
                        let step = traveler
                            .current_step_id
                            .as_ref()
                            .map(|step| step.as_str())
                            .unwrap_or("complete");
                        format!("{} {step}", traveler.status.label())
                    })
                    .unwrap_or_else(|| "No traveler".to_string());
                let yield_label = app
                    .workspace
                    .yield_analysis
                    .lot_summary(&lot_id)
                    .map(|summary| percent_label(summary.yield_fraction))
                    .unwrap_or_else(|| "yield n/a".to_string());
                let detail = lot
                    .map(|lot| {
                        format!(
                            "{} P{} {} wafers; {yield_label}",
                            lot.product,
                            lot.priority,
                            lot.wafers.len()
                        )
                    })
                    .unwrap_or(yield_label);
                PrimaryRow::new(format!("lot-{lot_id}"), lot_id.clone(), value, detail).action(
                    format!("fabricad.viewctl.workflow.focus_lot.{lot_id}"),
                    app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
                )
            }),
    );
    rows.extend(
        app.workspace
            .process_flow
            .route
            .nodes
            .iter()
            .take(2)
            .map(|node| {
                PrimaryRow::new(
                    format!("workflow-node-{}", node.id),
                    format!("{} {}", node.id, node.name),
                    format!("{} {}", node.kind.label(), node.area),
                    format!(
                        "{} tools; {}",
                        node.eligible_tools.len(),
                        node.recipe
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "no recipe".to_string())
                    ),
                )
                .action(
                    "fabricad.viewctl.workflow.open.process-flow",
                    app.active_view == StartupView::ProcessFlow,
                )
            }),
    );
    let dispatch = app.workspace.scheduler.dispatch(app.scheduler_policy);
    rows.extend(dispatch.assignments.iter().take(2).map(|assignment| {
        PrimaryRow::new(
            format!(
                "workflow-dispatch-{}-{}",
                assignment.lot_id, assignment.tool_id
            ),
            format!("Dispatch {}", assignment.lot_id),
            assignment.tool_id.to_string(),
            format!(
                "P{} start {} finish {}; tardy {} min",
                assignment.priority,
                assignment.start_minute,
                assignment.finish_minute,
                assignment.tardy_minutes
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.scheduler",
            app.active_view == StartupView::Scheduler,
        )
    }));
    rows.extend(
        app.workspace
            .inventory
            .alerts(INVENTORY_DEMO_TODAY)
            .into_iter()
            .take(1)
            .map(|alert| {
                PrimaryRow::new(
                    format!("workflow-inventory-alert-{}", alert.lot_id),
                    format!("Inventory {}", alert.kind.label()),
                    alert.lot_id.to_string(),
                    format!("{}; {}", alert.material_name, alert.message),
                )
                .action(
                    "fabricad.viewctl.workflow.open.inventory",
                    app.active_view == StartupView::Inventory,
                )
            }),
    );
    rows.extend(
        app.workspace
            .safety
            .active_conditions()
            .into_iter()
            .take(1)
            .map(|sensor| {
                PrimaryRow::new(
                    format!("workflow-safety-{}", sensor.id),
                    format!("Safety {}", sensor.severity.label()),
                    sensor.name.clone(),
                    sensor.message.clone(),
                )
                .action(
                    "fabricad.viewctl.workflow.open.safety",
                    app.active_view == StartupView::Safety,
                )
            }),
    );
    rows.extend(workflow_cross_link_rows(app));
    rows.truncate(16);
    rows
}

fn workflow_cross_link_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let workspace = &app.workspace;
    let focus_lot = app.workflow_focus_lot.as_deref().unwrap_or("no focus lot");
    let today = maintenance_today(workspace);
    let actionable = maintenance_filtered_work_count(workspace, MaintenanceWorkFilter::Actionable);
    let locked = maintenance_locked_count(workspace);
    let active_alarms = workspace.environment.active_alarms().len();
    let safety = workspace.safety.summary();
    let metrology_summary = workspace.wafer_map.summary(app.metrology_kind);
    let trace_summary = workspace.genealogy.summary();
    vec![
        PrimaryRow::new(
            "workflow-maintenance",
            "Maintenance",
            format!("{actionable} due"),
            format!("{locked} locked tools; today {today}"),
        )
        .action(
            "fabricad.viewctl.workflow.open.maintenance",
            app.active_view == StartupView::Maintenance,
        ),
        PrimaryRow::new(
            "workflow-environment",
            "Cleanroom",
            format!("{active_alarms} alarms"),
            format!(
                "{} sensors; {} readings",
                workspace.environment.sensors.len(),
                workspace.environment.readings.len()
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.environment",
            app.active_view == StartupView::Environment,
        ),
        PrimaryRow::new(
            "workflow-safety-guardrails",
            "Safety",
            format!("{} active", safety.active_condition_count),
            format!(
                "{} locked; {} open incidents; highest {}",
                safety.locked_out_tool_count,
                safety.open_incident_count,
                safety
                    .highest_severity
                    .map(SafetySeverity::label)
                    .unwrap_or("normal")
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.safety",
            app.active_view == StartupView::Safety,
        ),
        PrimaryRow::new(
            "workflow-metrology",
            "Metrology",
            format!("{} sites", metrology_summary.sample_count),
            format!(
                "{} fail / {} outlier on {}",
                metrology_summary.fail_count,
                metrology_summary.outlier_count,
                app.metrology_kind.label()
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.metrology",
            app.active_view == StartupView::Metrology,
        ),
        PrimaryRow::new(
            "workflow-traceability",
            "Traceability",
            format!("{} lots", trace_summary.lot_count),
            format!(
                "{} wafers; {} process records for {focus_lot}",
                trace_summary.wafer_count, trace_summary.process_record_count
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.traceability",
            app.active_view == StartupView::Traceability,
        ),
        PrimaryRow::new(
            "workflow-notebook",
            "Notebook",
            format!("{} entries", workspace.lab_notebook.entries.len()),
            format!(
                "{} tags; {} links",
                workspace.lab_notebook.tags().len(),
                workspace
                    .lab_notebook
                    .entries
                    .iter()
                    .map(|entry| entry.link_count())
                    .sum::<usize>()
            ),
        )
        .action(
            "fabricad.viewctl.workflow.open.notebook",
            app.active_view == StartupView::Notebook,
        ),
    ]
}

fn mask_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let prep = reticle_prep_for_app(app);
    let report = mask_check_report(app);
    let mut rows = mask_reticle_rows(&prep);
    rows.extend(mask_exposure_rows(&prep, &report).into_iter().take(2));
    rows.extend(mask_layer_rows(&prep).into_iter().take(2));
    rows.extend(mask_issue_group_rows(app, &report).into_iter().take(2));
    rows.extend(
        report
            .issues
            .iter()
            .filter(|issue| mask_issue_matches_filter(app, issue))
            .skip(app.mask_issue_page * MASK_ISSUE_PAGE_SIZE)
            .take(10usize.saturating_sub(rows.len()))
            .map(|issue| mask_issue_primary_row(app, issue)),
    );
    if rows.is_empty() {
        rows.push(PrimaryRow::new(
            "mask-clean",
            "Checks clean",
            "No mask prep issues",
            format!(
                "{} printable shapes across {} layers",
                report.printable_shape_count, report.layer_count
            ),
        ));
    }
    rows.truncate(10);
    rows
}

fn mask_reticle_rows(prep: &ReticlePrep) -> Vec<PrimaryRow> {
    let printable = prep.reticle.printable_bounds();
    let mut rows = vec![
        PrimaryRow::new(
            "mask-design",
            "Mask design",
            prep.mask_design_id.clone(),
            format!(
                "{}; route {}",
                prep.layout_revision,
                prep.route_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unlinked".to_string())
            ),
        ),
        PrimaryRow::new(
            "mask-reticle",
            format!("Reticle {}", prep.reticle.id),
            prep.reticle.name.clone(),
            format!(
                "size {} x {}; printable {} x {}",
                prep.reticle.size.width,
                prep.reticle.size.height,
                printable.width(),
                printable.height()
            ),
        ),
    ];
    rows.extend(prep.fields.iter().take(1).map(|field| {
        PrimaryRow::new(
            format!("mask-field-{}", field.id),
            format!("Field {}", field.id),
            field.name.clone(),
            format!(
                "cell {}; bounds {} x {}; stepping {} x {}",
                field.source_cell.0,
                field.layout_bounds.width(),
                field.layout_bounds.height(),
                field.stepping.columns,
                field.stepping.rows
            ),
        )
    }));
    rows
}

fn mask_exposure_rows(prep: &ReticlePrep, report: &MaskCheckReport) -> Vec<PrimaryRow> {
    prep.exposure_blocks
        .iter()
        .map(|block| {
            let issue_count = report
                .issues
                .iter()
                .filter(|issue| issue.block_id.as_ref() == Some(&block.id))
                .count();
            PrimaryRow::new(
                format!("mask-exposure-{}", block.id),
                format!("Exposure {}", block.id),
                block.name.clone(),
                format!(
                    "field {}; dose {:.1}; focus {:+.2}; passes {}; layers {}; {} issue(s)",
                    block.field_id,
                    block.dose_mj_cm2,
                    block.focus_offset_um,
                    block.passes,
                    block.layer_ids.len(),
                    issue_count
                ),
            )
        })
        .collect()
}

fn mask_layer_rows(prep: &ReticlePrep) -> Vec<PrimaryRow> {
    prep.layer_stack
        .iter()
        .map(|layer| {
            PrimaryRow::new(
                format!("mask-layer-{}", layer.layer.0),
                format!("Layer {} {}", layer.layer.0, layer.name),
                layer.tone.label(),
                format!(
                    "{:?}; feature {}; spacing {}; order {}{}",
                    layer.process,
                    layer.min_feature,
                    layer.min_spacing,
                    layer.display_order,
                    if layer.critical { "; critical" } else { "" }
                ),
            )
        })
        .collect()
}

fn mask_issue_group_rows(app: &FabricadApp, report: &MaskCheckReport) -> Vec<PrimaryRow> {
    let mut groups = BTreeMap::<String, (usize, usize)>::new();
    for issue in report
        .issues
        .iter()
        .filter(|issue| mask_issue_matches_filter(app, issue))
    {
        let entry = groups
            .entry(mask_issue_group_key(issue, app.mask_issue_grouping))
            .or_default();
        match issue.severity {
            MaskIssueSeverity::Error => entry.0 += 1,
            MaskIssueSeverity::Warning => entry.1 += 1,
        }
    }
    let mut rows = groups.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_total = left.1.0 + left.1.1;
        let right_total = right.1.0 + right.1.1;
        right_total
            .cmp(&left_total)
            .then_with(|| left.0.cmp(&right.0))
    });
    rows.into_iter()
        .map(|(group, (errors, warnings))| {
            PrimaryRow::new(
                format!("mask-group-{group}"),
                format!("{group} group"),
                format!("{} total", errors + warnings),
                format!("{errors} error(s), {warnings} warning(s)"),
            )
        })
        .collect()
}

fn mask_issue_primary_row(
    app: &FabricadApp,
    issue: &layout_model::mask::MaskPrepIssue,
) -> PrimaryRow {
    let location = [
        issue.layer.map(|layer| format!("L{}", layer.0)),
        issue
            .field_id
            .as_ref()
            .map(|field| format!("field {field}")),
        issue
            .block_id
            .as_ref()
            .map(|block| format!("block {block}")),
        issue.shape_id.map(|shape| format!("shape {}", shape.0)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let row = PrimaryRow::new(
        format!("mask-{}", issue.code),
        format!("{} {}", issue.severity.label(), issue.code),
        if location.is_empty() {
            "Document".to_string()
        } else {
            location
        },
        issue.message.clone(),
    );
    if let Some(shape) = issue.shape_id {
        row.action(
            format!("fabricad.viewctl.layout.shape.{}", shape.0),
            app.selected_layout_shape == Some(shape),
        )
    } else {
        row
    }
}

fn layout_diff_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let report = layout_diff_report(app);
    let page_size = normalized_layout_diff_page_size(app);
    let mut rows = vec![
        PrimaryRow::new(
            "diff-setup",
            "Comparison",
            format!(
                "{} -> {}",
                app.layout_diff_baseline.label(),
                app.layout_diff_candidate.label()
            ),
            format!(
                "changed only {}; filter {}; page size {}",
                app.layout_diff_changed_only,
                app.layout_diff_change_filter.detail_label(),
                page_size
            ),
        ),
        PrimaryRow::new(
            "diff-summary",
            "Shape summary",
            format!("{} changes", layout_diff_total_changes(&report)),
            format!(
                "{} added; {} removed; {} modified",
                report.summary.added_shapes,
                report.summary.removed_shapes,
                report.summary.modified_shapes
            ),
        ),
        PrimaryRow::new(
            "diff-review",
            "Review",
            app.layout_diff_review_state.label(),
            format!(
                "{} baseline shapes; {} candidate shapes",
                report.summary.baseline_shapes, report.summary.candidate_shapes
            ),
        ),
    ];
    rows.extend(report.layers.iter().take(3).map(|layer| {
        PrimaryRow::new(
            format!("diff-layer-{}", layer.layer.0),
            format!("Layer {} {}", layer.layer.0, layer.name),
            format!("{} -> {}", layer.baseline_shapes, layer.candidate_shapes),
            format!(
                "{} added; {} removed; {} modified",
                layer.added_shapes, layer.removed_shapes, layer.modified_shapes
            ),
        )
    }));
    rows.extend(
        report
            .changes
            .iter()
            .filter(|change| app.layout_diff_change_filter.matches(change.kind))
            .skip(app.layout_diff_change_page * page_size)
            .take(10usize.saturating_sub(rows.len()))
            .map(|change| {
                PrimaryRow::new(
                    format!("diff-{}-{}", change.kind.label(), change.id.0),
                    format!("{} shape {}", change.kind.label(), change.id.0),
                    format!("L{} {}", change.layer.0, change.layer_name),
                    change.detail.clone(),
                )
                .action(
                    format!("fabricad.viewctl.layout.shape.{}", change.id.0),
                    app.selected_layout_shape == Some(change.id),
                )
            }),
    );
    rows.truncate(10);
    rows
}

fn fab_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let mut rows = app
        .workspace
        .equipment
        .tools()
        .take(5)
        .map(|tool| {
            let alarms = equipment_active_alarm_count(tool);
            let alarm_label = if alarms == 1 { "alarm" } else { "alarms" };
            PrimaryRow::new(
                format!("tool-{}", tool.id),
                format!("{} {}", tool.id, tool.name),
                tool.state.label(),
                format!(
                    "{}; {alarms} active {alarm_label}",
                    equipment_recipe_summary(tool)
                ),
            )
            .action(
                format!("fabricad.viewctl.fab.select.{}", tool.id),
                app.selected_equipment_tool.as_ref() == Some(&tool.id),
            )
        })
        .collect::<Vec<_>>();
    if let Some(tool) = selected_equipment_tool(app) {
        rows.extend(
            fab_selected_tool_rows(tool)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }
    rows.truncate(10);
    rows
}

fn fab_selected_tool_rows(tool: &EquipmentTool) -> Vec<PrimaryRow> {
    let mut rows = vec![PrimaryRow::new(
        format!("fab-selected-{}", tool.id),
        format!("Selected {}", tool.name),
        tool.state.label(),
        format!(
            "{}; {} recipes; last update t+{}s",
            tool.kind.label(),
            tool.available_recipes.len(),
            tool.last_updated_at_s
        ),
    )];
    if let Some(selection) = tool.selected_recipe.as_ref() {
        rows.push(PrimaryRow::new(
            format!("fab-recipe-{}", tool.id),
            "Selected recipe",
            selection.recipe_id.to_string(),
            format!("version {}", selection.recipe_version),
        ));
    } else {
        rows.push(PrimaryRow::new(
            format!("fab-recipe-{}", tool.id),
            "Selected recipe",
            "none",
            "load a recipe before running",
        ));
    }
    rows.extend(tool.active_alarms.iter().take(2).map(|alarm| {
        PrimaryRow::new(
            format!("fab-alarm-{}-{}", tool.id, alarm.id),
            format!("{} {}", alarm.severity.label(), alarm.code),
            if alarm.active { "active" } else { "cleared" },
            format!("{} at t+{}s", alarm.message, alarm.occurred_at_s),
        )
    }));
    if let Some(run) = tool.active_run.as_ref() {
        rows.push(PrimaryRow::new(
            format!("fab-run-{}", run.id),
            format!("Run {}", run.id),
            run.status.label(),
            format!(
                "{} since t+{}s; {} samples",
                run.recipe.recipe_id, run.started_at_s, run.sensor_count
            ),
        ));
    }
    rows.extend(tool.recent_sensors.iter().rev().take(3).map(|sample| {
        PrimaryRow::new(
            format!("fab-sensor-{}-{}-{}", tool.id, sample.name, sample.at_s),
            sample.name.clone(),
            format!("{} {}", spc_compact_number(sample.value), sample.unit),
            format!("sample t+{}s", sample.at_s),
        )
    }));
    rows.extend(tool.event_log.iter().rev().take(2).map(|event| {
        PrimaryRow::new(
            format!("fab-event-{}-{}", tool.id, event.at_s),
            "Event",
            format!("t+{}s", event.at_s),
            event.message.clone(),
        )
    }));
    rows
}

fn inventory_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let mut rows = Vec::new();
    for (index, alert) in app
        .workspace
        .inventory
        .alerts(INVENTORY_DEMO_TODAY)
        .into_iter()
        .take(3)
        .enumerate()
    {
        rows.push(
            PrimaryRow::new(
                format!("inventory-alert-{}-{index}", alert.lot_id),
                format!("{} {}", alert.kind.label(), alert.lot_id),
                alert.material_name,
                alert.message,
            )
            .action(
                format!("fabricad.viewctl.inventory.lot.{}", alert.lot_id),
                app.selected_inventory_lot.as_ref() == Some(&alert.lot_id),
            ),
        );
    }

    rows.extend(
        app.workspace
            .inventory
            .sorted_lots()
            .into_iter()
            .filter(|lot| inventory_filter_matches(app.inventory_filter, lot))
            .take(6usize.saturating_sub(rows.len()).max(2))
            .map(inventory_lot_primary_row(app)),
    );

    if let Some(lot) = selected_inventory_lot(app) {
        rows.extend(
            inventory_selected_lot_rows(lot)
                .into_iter()
                .take(10 - rows.len().min(10)),
        );
    }

    rows.truncate(10);
    rows
}

fn inventory_lot_primary_row<'a>(app: &'a FabricadApp) -> impl Fn(&'a MaterialLot) -> PrimaryRow {
    |lot| {
        let expiry = lot
            .expires_on
            .map(layout_model::inventory::format_date)
            .map(|date| format!("expires {date}"))
            .unwrap_or_else(|| "no expiry".to_string());
        PrimaryRow::new(
            format!("inventory-{}", lot.id),
            format!("{} {}", lot.id, inventory_material_status_label(lot)),
            lot.material_name.clone(),
            format!(
                "{} {}; {}; {}",
                lot.stock.format(),
                lot.category.label(),
                lot.location.label(),
                expiry
            ),
        )
        .action(
            format!("fabricad.viewctl.inventory.lot.{}", lot.id),
            app.selected_inventory_lot.as_ref() == Some(&lot.id),
        )
    }
}

fn inventory_selected_lot_rows(lot: &MaterialLot) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            format!("inventory-detail-stock-{}", lot.id),
            "Stock",
            lot.stock.format(),
            format!("reorder at {}", lot.reorder_threshold.format()),
        ),
        PrimaryRow::new(
            format!("inventory-detail-expiration-{}", lot.id),
            "Expiration",
            inventory_expiration_label(lot),
            inventory_release_cue(lot),
        ),
        PrimaryRow::new(
            format!("inventory-detail-location-{}", lot.id),
            "Location",
            lot.location.label(),
            format!(
                "cabinet {} bin {} {}",
                lot.location.cabinet, lot.location.bin, lot.location.temperature
            ),
        ),
        PrimaryRow::new(
            format!("inventory-detail-supplier-{}", lot.id),
            "Supplier",
            lot.supplier.supplier.clone(),
            format!(
                "lot {} received {} by {}",
                lot.supplier.supplier_lot,
                layout_model::inventory::format_date(lot.supplier.received_date),
                lot.supplier.received_by
            ),
        ),
    ];
    rows.extend(lot.usage.iter().take(3).enumerate().map(|(index, usage)| {
        PrimaryRow::new(
            format!("inventory-usage-{}-{index}", lot.id),
            format!(
                "{} {}",
                layout_model::inventory::format_date(usage.timestamp),
                usage.quantity.format()
            ),
            usage.actor.clone(),
            format!(
                "{}; {}",
                usage
                    .links
                    .iter()
                    .map(|link| link.label())
                    .collect::<Vec<_>>()
                    .join(", "),
                usage.note
            ),
        )
    }));
    if let Some(certificate_id) = &lot.supplier.certificate_id {
        rows.push(PrimaryRow::new(
            format!("inventory-certificate-{}", lot.id),
            "Certificate",
            certificate_id.clone(),
            lot.supplier
                .certificate_url
                .clone()
                .unwrap_or_else(|| "no record URL".to_string()),
        ));
    }
    rows
}

fn inventory_material_status_label(lot: &MaterialLot) -> &'static str {
    if lot.is_expired(INVENTORY_DEMO_TODAY) {
        "expired"
    } else if lot.is_low_stock() {
        "low"
    } else if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
        "expiring"
    } else if !lot.usage.is_empty() {
        "in use"
    } else {
        "released"
    }
}

fn inventory_expiration_label(lot: &MaterialLot) -> String {
    lot.expires_on
        .map(layout_model::inventory::format_date)
        .map(|date| {
            if lot.is_expired(INVENTORY_DEMO_TODAY) {
                format!("expired {date}")
            } else if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
                format!("expires soon {date}")
            } else {
                format!("expires {date}")
            }
        })
        .unwrap_or_else(|| "no expiry".to_string())
}

fn inventory_release_cue(lot: &MaterialLot) -> String {
    let mut cues = Vec::new();
    if lot.is_expired(INVENTORY_DEMO_TODAY) {
        cues.push(InventoryAlertKind::Expired.label());
    }
    if lot.is_low_stock() {
        cues.push(InventoryAlertKind::LowStock.label());
    }
    if lot.expires_within_days(INVENTORY_DEMO_TODAY, 30) {
        cues.push(InventoryAlertKind::ExpiringSoon.label());
    }
    if cues.is_empty() {
        "released for use".to_string()
    } else {
        cues.join(", ")
    }
}

fn maintenance_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let today = maintenance_today(&app.workspace);
    let mut rows = Vec::new();
    for tool in &app.workspace.maintenance.tools {
        for schedule in &tool.schedules {
            let due_state = schedule.due_state(today, tool.run_count);
            if !maintenance_work_matches(
                &app.workspace,
                app.maintenance_work_filter,
                &tool.tool_id,
                schedule.kind,
                due_state,
                schedule.next_due,
            ) {
                continue;
            }
            rows.push(
                PrimaryRow::new(
                    format!("maint-{}-{}", tool.tool_id, schedule.id),
                    format!("{} {}", tool.tool_id, schedule.task),
                    due_state.label(),
                    format!(
                        "{} due {}; runs {}",
                        schedule.kind.label(),
                        schedule.next_due,
                        tool.run_count
                    ),
                )
                .action(
                    format!("fabricad.viewctl.maintenance.tool.{}", tool.tool_id),
                    app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
                ),
            );
            if rows.len() >= 4 {
                break;
            }
        }
        if rows.len() >= 4 {
            break;
        }
    }

    if let Some(tool) = selected_maintenance_tool(app) {
        rows.extend(
            maintenance_selected_tool_rows(app, tool, today)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }

    rows.extend(
        maintenance_release_rows(app, today)
            .into_iter()
            .take(10usize.saturating_sub(rows.len())),
    );

    rows.extend(
        maintenance_history_rows(app)
            .into_iter()
            .take(10usize.saturating_sub(rows.len())),
    );

    if rows.is_empty() {
        rows.extend(app.workspace.maintenance.tools.iter().take(10).map(|tool| {
            PrimaryRow::new(
                format!("maint-{}", tool.tool_id),
                format!("{} {}", tool.tool_id, tool.tool_name),
                format!("{} schedules", tool.schedules.len()),
                format!("{} tool runs", tool.run_count),
            )
            .action(
                format!("fabricad.viewctl.maintenance.tool.{}", tool.tool_id),
                app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
            )
        }));
    }
    rows.truncate(10);
    rows
}

fn maintenance_selected_tool_rows(
    app: &FabricadApp,
    tool: &layout_model::maintenance::ToolMaintenanceState,
    today: FabDate,
) -> Vec<PrimaryRow> {
    let release = app
        .workspace
        .maintenance
        .release_for_tool(&tool.tool_id, today);
    let mut rows = vec![
        PrimaryRow::new(
            format!("maint-selected-{}", tool.tool_id),
            tool.tool_name.clone(),
            format!("{} runs", tool.run_count),
            release.state.label().to_string(),
        )
        .action(
            format!("fabricad.viewctl.maintenance.tool.{}", tool.tool_id),
            app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
        ),
    ];
    if release.reasons.is_empty() {
        rows.push(PrimaryRow::new(
            format!("maint-release-context-{}", tool.tool_id),
            "Release context",
            "No active holds",
            "released workflow is clear",
        ));
    } else {
        rows.extend(
            release
                .reasons
                .iter()
                .take(2)
                .enumerate()
                .map(|(index, reason)| {
                    PrimaryRow::new(
                        format!("maint-release-hold-{}-{index}", tool.tool_id),
                        format!("Hold {}", index + 1),
                        release.state.label(),
                        reason.clone(),
                    )
                }),
        );
    }
    if let Some(next) = tool
        .schedules
        .iter()
        .min_by_key(|schedule| (schedule.next_due, schedule.id.clone()))
    {
        let due_state = next.due_state(today, tool.run_count);
        rows.push(PrimaryRow::new(
            format!("maint-next-{}-{}", tool.tool_id, next.id),
            format!("Next {}", next.task),
            due_state.label(),
            format!(
                "due {}; {} checklist item(s)",
                next.next_due,
                next.checklist.len()
            ),
        ));
        if !next.checklist.is_empty() {
            rows.push(PrimaryRow::new(
                format!("maint-checklist-{}-{}", tool.tool_id, next.id),
                "Checklist",
                format!("{} items", next.checklist.len()),
                next.checklist
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
    }
    if let Some(record) = app
        .workspace
        .maintenance
        .calibration_records_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|record| record.performed_at)
    {
        rows.push(PrimaryRow::new(
            format!("maint-calibration-{}", record.id),
            "Latest calibration",
            record.outcome.label(),
            format!(
                "{} {} {:.2} vs {} by {}",
                record.performed_at,
                record.parameter,
                record.measured_value,
                record.tolerance,
                record.technician
            ),
        ));
    }
    if let Some(result) = app
        .workspace
        .maintenance
        .qualification_results_for(&tool.tool_id)
        .into_iter()
        .max_by_key(|result| result.performed_at)
    {
        rows.push(PrimaryRow::new(
            format!("maint-qualification-{}", result.id),
            "Latest qualification",
            result.outcome.label(),
            format!(
                "{} {} {} {:.2} ({})",
                result.performed_at, result.wafer_id, result.metric, result.value, result.spec
            ),
        ));
    }
    rows
}

fn maintenance_release_rows(app: &FabricadApp, today: FabDate) -> Vec<PrimaryRow> {
    app.workspace
        .maintenance
        .tools
        .iter()
        .map(|tool| {
            let release = app
                .workspace
                .maintenance
                .release_for_tool(&tool.tool_id, today);
            PrimaryRow::new(
                format!("maint-release-{}", tool.tool_id),
                format!("{} {}", tool.tool_id, release.state.label()),
                tool.tool_name.clone(),
                if release.reasons.is_empty() {
                    format!("{} runs; no release holds", tool.run_count)
                } else {
                    format!(
                        "{} runs; {}",
                        tool.run_count,
                        release
                            .reasons
                            .iter()
                            .take(2)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                },
            )
            .action(
                format!("fabricad.viewctl.maintenance.tool.{}", tool.tool_id),
                app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
            )
        })
        .collect()
}

fn maintenance_history_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let selected = app.selected_maintenance_tool.as_ref();
    let mut rows = Vec::new();
    rows.extend(
        app.workspace
            .maintenance
            .calibration_records
            .iter()
            .filter(|record| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&record.tool_id)
            })
            .take(3)
            .map(|record| {
                PrimaryRow::new(
                    format!("maint-history-cal-{}", record.id),
                    format!("{} calibration {}", record.performed_at, record.tool_id),
                    record.outcome.label(),
                    format!(
                        "{} {} by {}",
                        record.instrument, record.parameter, record.technician
                    ),
                )
                .action(
                    format!("fabricad.viewctl.maintenance.tool.{}", record.tool_id),
                    selected == Some(&record.tool_id),
                )
            }),
    );
    rows.extend(
        app.workspace
            .maintenance
            .qualification_results
            .iter()
            .filter(|result| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&result.tool_id)
            })
            .take(3)
            .map(|result| {
                PrimaryRow::new(
                    format!("maint-history-qual-{}", result.id),
                    format!("{} qual {}", result.performed_at, result.tool_id),
                    result.outcome.label(),
                    format!("{} {} {:.2}", result.wafer_id, result.metric, result.value),
                )
                .action(
                    format!("fabricad.viewctl.maintenance.tool.{}", result.tool_id),
                    selected == Some(&result.tool_id),
                )
            }),
    );
    rows.extend(
        app.workspace
            .maintenance
            .downtime
            .iter()
            .filter(|record| {
                app.maintenance_history_filter == MaintenanceHistoryFilter::AllTools
                    || selected == Some(&record.tool_id)
            })
            .take(3)
            .map(|record| {
                PrimaryRow::new(
                    format!("maint-history-down-{}", record.id),
                    format!("{} downtime {}", record.started_at, record.tool_id),
                    record
                        .ended_at
                        .map(|date| format!("ended {date}"))
                        .unwrap_or_else(|| "open".to_string()),
                    format!("{}; owner {}", record.reason, record.owner),
                )
                .action(
                    format!("fabricad.viewctl.maintenance.tool.{}", record.tool_id),
                    selected == Some(&record.tool_id),
                )
            }),
    );
    rows
}

fn environment_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    app.workspace
        .environment
        .sensors
        .iter()
        .take(10)
        .map(|sensor| {
            let tags = if sensor.process_tags.is_empty() {
                "untagged".to_string()
            } else {
                sensor.process_tags.join(", ")
            };
            PrimaryRow::new(
                format!("sensor-{}", sensor.id),
                format!("{} {}", sensor.id, sensor.name),
                environment_sensor_status_for(&app.workspace, sensor),
                format!("{}; {}; {}", sensor.zone, sensor.kind.label(), tags),
            )
            .action(
                format!("fabricad.viewctl.environment.sensor.{}", sensor.id),
                app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
            )
        })
        .collect()
}

#[derive(Clone, Debug, Default)]
struct EnvironmentZoneSummary {
    name: String,
    sensor_count: usize,
    advisory_count: usize,
    warning_count: usize,
    critical_count: usize,
    excursion_count: usize,
    latest_timestamp_min: Option<u32>,
}

fn environment_view_primitives(app: &FabricadApp, ui_scale: UiScale) -> Vec<ScenePrimitive> {
    let environment = &app.workspace.environment;
    let mut primitives =
        Vec::with_capacity(environment.sensors.len() * 8 + environment.alarms.len() + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let trend_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(24.0),
        ui_scale.value(404.0),
        ui_scale.value(158.0),
    );
    let zone_bounds = UiRect::new(
        frame.x + ui_scale.value(468.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(186.0),
        ui_scale.value(154.0),
    );
    let alarm_bounds = UiRect::new(
        frame.x + ui_scale.value(696.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(184.0),
        ui_scale.value(154.0),
    );
    if let Some(sensor) = selected_environment_sensor(app) {
        add_environment_trend_primitives(
            &mut primitives,
            trend_bounds,
            environment,
            sensor,
            ui_scale,
        );
    } else {
        add_metrology_empty_line(&mut primitives, trend_bounds, ui_scale);
    }
    add_environment_zone_primitives(
        &mut primitives,
        zone_bounds,
        &environment_zone_summaries(environment),
        selected_environment_sensor(app).map(|sensor| sensor.zone.as_str()),
        ui_scale,
    );
    add_environment_alarm_primitives(&mut primitives, alarm_bounds, environment, ui_scale);
    primitives
}

fn add_environment_trend_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    environment: &CleanroomEnvironment,
    sensor: &EnvironmentSensor,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let readings = environment.readings_for_sensor(&sensor.id);
    if readings.len() < 2 {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let min_x = readings
        .first()
        .map(|reading| reading.timestamp_min)
        .unwrap_or_default() as f32;
    let max_x = readings
        .last()
        .map(|reading| reading.timestamp_min)
        .unwrap_or_default() as f32;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for reading in &readings {
        add_spc_plot_value(&mut min_value, &mut max_value, reading.value);
    }
    for threshold in [
        sensor.thresholds.critical_low,
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
        sensor.thresholds.critical_high,
    ]
    .into_iter()
    .flatten()
    {
        add_spc_plot_value(&mut min_value, &mut max_value, threshold);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);
    add_environment_threshold_band(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_low,
        sensor.thresholds.warning_high,
        ColorRgba::new(80, 166, 114, 54),
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_low,
        ColorRgba::new(238, 181, 82, 165),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.warning_high,
        ColorRgba::new(238, 181, 82, 165),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.critical_low,
        ColorRgba::new(236, 91, 88, 175),
        ui_scale,
    );
    add_environment_threshold_line(
        primitives,
        bounds,
        min_value,
        max_value,
        sensor.thresholds.critical_high,
        ColorRgba::new(236, 91, 88, 175),
        ui_scale,
    );
    let trend_color = environment
        .trend_summary(&sensor.id)
        .map(|summary| environment_trend_color(summary.direction, 230))
        .unwrap_or(ColorRgba::new(93, 168, 232, 230));
    for pair in readings.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].timestamp_min as f32),
                spc_plot_y(bounds, min_value, max_value, pair[0].value),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].timestamp_min as f32),
                spc_plot_y(bounds, min_value, max_value, pair[1].value),
            ),
            stroke: StrokeStyle::new(trend_color, ui_scale.value(1.8)),
        });
    }
    for reading in &readings {
        let severity = sensor.thresholds.evaluate(reading.value);
        let point = UiPoint::new(
            spc_plot_x(bounds, min_x, max_x, reading.timestamp_min as f32),
            spc_plot_y(bounds, min_value, max_value, reading.value),
        );
        if let Some(severity) = severity {
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(point.x, bounds.y),
                to: UiPoint::new(point.x, bounds.bottom()),
                stroke: StrokeStyle::new(
                    environment_alarm_severity_color(severity, 92),
                    ui_scale.value(1.0),
                ),
            });
        }
        primitives.push(ScenePrimitive::Circle {
            center: point,
            radius: ui_scale.value(if severity.is_some() { 3.7 } else { 2.6 }),
            fill: environment_sensor_state_color(severity, 235),
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(0.8),
            )),
        });
    }
}

fn add_environment_threshold_band(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    min_value: f64,
    max_value: f64,
    low: Option<f64>,
    high: Option<f64>,
    color: ColorRgba,
) {
    let Some(low) = low else {
        return;
    };
    let Some(high) = high else {
        return;
    };
    let y_high = spc_plot_y(bounds, min_value, max_value, high);
    let y_low = spc_plot_y(bounds, min_value, max_value, low);
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            bounds.x,
            y_high.min(y_low),
            bounds.width,
            (y_low - y_high).abs(),
        ),
        color,
    )));
}

fn add_environment_threshold_line(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    min_value: f64,
    max_value: f64,
    value: Option<f64>,
    color: ColorRgba,
    ui_scale: UiScale,
) {
    let Some(value) = value else {
        return;
    };
    let y = spc_plot_y(bounds, min_value, max_value, value);
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, y),
        to: UiPoint::new(bounds.right(), y),
        stroke: StrokeStyle::new(color, ui_scale.value(1.0)),
    });
}

fn add_environment_zone_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    zones: &[EnvironmentZoneSummary],
    selected_zone: Option<&str>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if zones.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = zones.len().min(7);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, zone) in zones.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        let fill = if selected_zone == Some(zone.name.as_str()) {
            ColorRgba::new(72, 128, 188, 70)
        } else {
            ColorRgba::new(25, 32, 39, 255)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(row, fill)));
        let total = zone.sensor_count.max(1) as f32;
        let critical_width = row.width * zone.critical_count as f32 / total;
        let warning_width = row.width * zone.warning_count as f32 / total;
        let advisory_width = row.width * zone.advisory_count as f32 / total;
        let mut x = row.x;
        for (width, color) in [
            (
                critical_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Critical, 215),
            ),
            (
                warning_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Warning, 215),
            ),
            (
                advisory_width,
                environment_alarm_severity_color(EnvironmentAlarmSeverity::Advisory, 190),
            ),
        ] {
            if width <= 0.0 {
                continue;
            }
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, row.y, width.max(ui_scale.value(1.0)), row.height),
                color,
            )));
            x += width;
        }
    }
}

fn add_environment_alarm_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    environment: &CleanroomEnvironment,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if environment.alarms.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 150),
        )));
        return;
    }
    let visible = environment.alarms.len().min(10);
    let gap = ui_scale.value(4.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, alarm) in environment.alarms.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(bounds.x, y, bounds.width, row_height),
                environment_alarm_severity_color(
                    alarm.severity,
                    if alarm.active { 225 } else { 115 },
                ),
            )
            .stroke(StrokeStyle::new(
                if alarm.active {
                    ColorRgba::new(252, 253, 255, 210)
                } else {
                    ColorRgba::new(16, 20, 24, 130)
                },
                ui_scale.value(if alarm.active { 1.2 } else { 0.6 }),
            )),
        ));
    }
}

fn environment_zone_summaries(environment: &CleanroomEnvironment) -> Vec<EnvironmentZoneSummary> {
    let mut zones: BTreeMap<String, EnvironmentZoneSummary> = BTreeMap::new();
    for sensor in &environment.sensors {
        let entry = zones
            .entry(sensor.zone.clone())
            .or_insert_with(|| EnvironmentZoneSummary {
                name: sensor.zone.clone(),
                ..Default::default()
            });
        entry.sensor_count += 1;
        if let Some(reading) = environment.latest_reading(&sensor.id) {
            entry.latest_timestamp_min = Some(
                entry
                    .latest_timestamp_min
                    .map_or(reading.timestamp_min, |current| {
                        current.max(reading.timestamp_min)
                    }),
            );
            match sensor.thresholds.evaluate(reading.value) {
                Some(EnvironmentAlarmSeverity::Critical) => entry.critical_count += 1,
                Some(EnvironmentAlarmSeverity::Warning) => entry.warning_count += 1,
                Some(EnvironmentAlarmSeverity::Advisory) => entry.advisory_count += 1,
                None => {}
            }
        }
        entry.excursion_count += environment
            .alarms
            .iter()
            .filter(|alarm| alarm.sensor_id == sensor.id)
            .count();
    }
    zones.into_values().collect()
}

fn environment_selected_sensor_label(
    environment: &CleanroomEnvironment,
    sensor: &EnvironmentSensor,
) -> String {
    environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            format!(
                "{} {} {:.2} {} {}",
                sensor.id,
                sensor.zone,
                reading.value,
                sensor.unit,
                severity.map_or("Nominal", |severity| severity.label())
            )
        })
        .unwrap_or_else(|| format!("{} {} no samples", sensor.id, sensor.zone))
}

fn environment_selected_zone_label(
    zones: &[EnvironmentZoneSummary],
    selected_zone: &str,
) -> String {
    zones
        .iter()
        .find(|zone| zone.name == selected_zone)
        .map(|zone| {
            format!(
                "{}: {} sensors, {} excursions",
                zone.name, zone.sensor_count, zone.excursion_count
            )
        })
        .unwrap_or_else(|| format!("{selected_zone}: no zone summary"))
}

fn environment_correlation_label(environment: &CleanroomEnvironment, sensor_id: &str) -> String {
    let labels = environment
        .correlations
        .iter()
        .filter(|label| label.sensor_ids.iter().any(|id| id == sensor_id))
        .map(|label| label.label.as_str())
        .take(3)
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "No process correlation labels".to_string()
    } else {
        compact_button_label(&labels.join(", "), 54)
    }
}

fn environment_sensor_state_color(
    severity: Option<EnvironmentAlarmSeverity>,
    alpha: u8,
) -> ColorRgba {
    severity.map_or(ColorRgba::new(91, 190, 130, alpha), |severity| {
        environment_alarm_severity_color(severity, alpha)
    })
}

fn environment_alarm_severity_color(severity: EnvironmentAlarmSeverity, alpha: u8) -> ColorRgba {
    match severity {
        EnvironmentAlarmSeverity::Advisory => ColorRgba::new(102, 190, 236, alpha),
        EnvironmentAlarmSeverity::Warning => ColorRgba::new(238, 181, 82, alpha),
        EnvironmentAlarmSeverity::Critical => ColorRgba::new(236, 91, 88, alpha),
    }
}

fn environment_trend_color(direction: TrendDirection, alpha: u8) -> ColorRgba {
    match direction {
        TrendDirection::Falling => ColorRgba::new(102, 190, 236, alpha),
        TrendDirection::Stable => ColorRgba::new(91, 190, 130, alpha),
        TrendDirection::Rising => ColorRgba::new(238, 181, 82, alpha),
    }
}

fn experiment_view_primitives(
    plan: &ExperimentPlan,
    analysis: &ExperimentAnalysisSummary,
    missing_only: bool,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(plan.runs.len() * plan.factors.len() + 100);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let matrix_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(20.0),
        ui_scale.value(400.0),
        ui_scale.value(162.0),
    );
    let response_bounds = UiRect::new(
        frame.x + ui_scale.value(462.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(184.0),
        ui_scale.value(70.0),
    );
    let effect_bounds = UiRect::new(
        frame.x + ui_scale.value(462.0),
        frame.y + ui_scale.value(126.0),
        ui_scale.value(184.0),
        ui_scale.value(56.0),
    );
    let status_bounds = UiRect::new(
        frame.x + ui_scale.value(690.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(184.0),
        ui_scale.value(154.0),
    );
    add_experiment_matrix_primitives(&mut primitives, matrix_bounds, plan, missing_only, ui_scale);
    add_experiment_response_primitives(&mut primitives, response_bounds, plan, analysis, ui_scale);
    add_experiment_effect_primitives(&mut primitives, effect_bounds, analysis, ui_scale);
    add_experiment_status_primitives(&mut primitives, status_bounds, plan, ui_scale);
    primitives
}

fn add_experiment_matrix_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    missing_only: bool,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let response_count = plan.responses.len();
    let runs = plan
        .runs
        .iter()
        .filter(|run| !missing_only || response_count == 0 || run.responses.len() < response_count)
        .take(14)
        .collect::<Vec<_>>();
    if runs.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(3.0);
    let row_height =
        (bounds.height - gap * runs.len().saturating_sub(1) as f32) / runs.len().max(1) as f32;
    let columns = plan.factors.len().max(1) + 1;
    let cell_width = (bounds.width - gap * columns.saturating_sub(1) as f32) / columns as f32;
    for (row_index, run) in runs.iter().enumerate() {
        let y = bounds.y + row_index as f32 * (row_height + gap);
        for (column, factor) in plan.factors.iter().enumerate() {
            let x = bounds.x + column as f32 * (cell_width + gap);
            let fill = run
                .factor_levels
                .get(&factor.id)
                .and_then(|level_id| factor.levels.iter().position(|level| level.id == *level_id))
                .map(|index| experiment_level_color(index, 210))
                .unwrap_or(ColorRgba::new(45, 54, 62, 160));
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, y, cell_width, row_height),
                fill,
            )));
        }
        let x = bounds.x + (columns - 1) as f32 * (cell_width + gap);
        let progress = if response_count == 0 {
            0.0
        } else {
            run.responses.len() as f32 / response_count as f32
        };
        let cell = UiRect::new(x, y, cell_width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            cell,
            ColorRgba::new(28, 36, 44, 255),
        )));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(cell.x, cell.y, cell.width * progress, cell.height),
            experiment_run_status_color(run.status, 215),
        )));
    }
}

fn add_experiment_response_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    analysis: &ExperimentAnalysisSummary,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if analysis.response_stats.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = analysis.response_stats.len().min(5);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, stat) in analysis.response_stats.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let progress = if analysis.run_count == 0 {
            0.0
        } else {
            stat.sample_count as f32 / analysis.run_count as f32
        };
        let color = plan
            .response(&stat.response_id)
            .zip(stat.mean)
            .map(|(response, mean)| {
                experiment_response_status_color(response.value_status(mean), 215)
            })
            .unwrap_or(ColorRgba::new(93, 168, 232, 190));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(row.x, row.y, row.width * progress, row.height),
            color,
        )));
    }
}

fn add_experiment_effect_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    analysis: &ExperimentAnalysisSummary,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if analysis.factor_effects.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = analysis.factor_effects.len().min(5);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    let max_abs = analysis
        .factor_effects
        .iter()
        .map(|effect| effect.delta_from_overall.abs())
        .fold(0.0_f64, f64::max)
        .max(0.001) as f32;
    let center_x = bounds.x + bounds.width * 0.5;
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(center_x, bounds.y),
        to: UiPoint::new(center_x, bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(130, 145, 160, 145), ui_scale.value(1.0)),
    });
    for (index, effect) in analysis.factor_effects.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = (effect.delta_from_overall.abs() as f32 / max_abs) * bounds.width * 0.48;
        let x = if effect.delta_from_overall >= 0.0 {
            center_x
        } else {
            center_x - width
        };
        let color = if effect.delta_from_overall.abs() < 0.01 {
            ColorRgba::new(91, 190, 130, 190)
        } else if effect.delta_from_overall > 0.0 {
            ColorRgba::new(238, 181, 82, 215)
        } else {
            ColorRgba::new(102, 190, 236, 215)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(x, y, width.max(ui_scale.value(2.0)), row_height),
            color,
        )));
    }
}

fn add_experiment_status_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    plan: &ExperimentPlan,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if plan.runs.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let statuses = [
        ExperimentRunStatus::Ready,
        ExperimentRunStatus::InProgress,
        ExperimentRunStatus::Complete,
        ExperimentRunStatus::Blocked,
    ];
    let gap = ui_scale.value(6.0);
    let row_height =
        (bounds.height - gap * statuses.len().saturating_sub(1) as f32) / statuses.len() as f32;
    let total = plan.runs.len().max(1) as f32;
    for (index, status) in statuses.iter().enumerate() {
        let count = plan.runs.iter().filter(|run| run.status == *status).count();
        let y = bounds.y + index as f32 * (row_height + gap);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, bounds.width, row_height),
            ColorRgba::new(25, 32, 39, 255),
        )));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, bounds.width * count as f32 / total, row_height),
            experiment_run_status_color(*status, 215),
        )));
    }
}

fn experiment_run_status_color(status: ExperimentRunStatus, alpha: u8) -> ColorRgba {
    match status {
        ExperimentRunStatus::Ready => ColorRgba::new(102, 190, 236, alpha),
        ExperimentRunStatus::InProgress => ColorRgba::new(238, 181, 82, alpha),
        ExperimentRunStatus::Complete => ColorRgba::new(91, 190, 130, alpha),
        ExperimentRunStatus::Blocked => ColorRgba::new(236, 91, 88, alpha),
    }
}

fn experiment_response_status_color(status: ResponseValueStatus, alpha: u8) -> ColorRgba {
    match status {
        ResponseValueStatus::InSpec => ColorRgba::new(91, 190, 130, alpha),
        ResponseValueStatus::BelowSpec => ColorRgba::new(102, 190, 236, alpha),
        ResponseValueStatus::AboveSpec => ColorRgba::new(236, 91, 88, alpha),
    }
}

fn demo_experiment_response_value(
    run: &ExperimentRun,
    response_id: &ResponseSpecId,
) -> Option<f64> {
    let dose_level = run
        .factor_levels
        .get(&FactorId::new("dose"))
        .map(|level| level.as_str())
        .unwrap_or("dose_nominal");
    let focus_level = run
        .factor_levels
        .get(&FactorId::new("focus"))
        .map(|level| level.as_str())
        .unwrap_or("focus_minus");

    let dose_cd: f64 = match dose_level {
        "dose_low" => -2.2,
        "dose_high" => 2.4,
        _ => 0.0,
    };
    let focus_cd = match focus_level {
        "focus_plus" => -0.7,
        _ => 0.4,
    };
    let process_bonus = match (dose_level, focus_level) {
        ("dose_nominal", "focus_plus") => 0.03,
        ("dose_high", "focus_plus") => 0.01,
        ("dose_low", "focus_plus") => -0.02,
        _ => 0.0,
    };

    match response_id.as_str() {
        "poly_cd_nm" => Some(72.0 + dose_cd + focus_cd),
        "defect_count" => Some(match dose_level {
            "dose_low" => 8.0,
            "dose_high" if focus_level == "focus_minus" => 6.0,
            "dose_high" => 4.0,
            _ => 3.0,
        }),
        "yield_fraction" => Some((0.88 + process_bonus - dose_cd.abs() * 0.01).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn format_experiment_response_value(value: f64, response: &ResponseSpec) -> String {
    if response.unit == "%" && value.abs() <= 1.0 {
        format!("{:.1}%", value * 100.0)
    } else if response.unit.is_empty() {
        format_compact_number(value)
    } else {
        format!("{} {}", format_compact_number(value), response.unit)
    }
}

fn experiment_response_spec_label(response: &ResponseSpec) -> String {
    let target = response
        .target
        .map(|value| format_experiment_response_value(value, response))
        .unwrap_or_else(|| "no target".to_string());
    let spec = match (response.lower_spec, response.upper_spec) {
        (Some(lower), Some(upper)) => format!(
            "{}..{}",
            format_experiment_response_value(lower, response),
            format_experiment_response_value(upper, response)
        ),
        (Some(lower), None) => format!(">= {}", format_experiment_response_value(lower, response)),
        (None, Some(upper)) => format!("<= {}", format_experiment_response_value(upper, response)),
        (None, None) => "no spec".to_string(),
    };
    format!("Target {target}; spec {spec}")
}

fn experiment_level_color(index: usize, alpha: u8) -> ColorRgba {
    match index % 6 {
        0 => ColorRgba::new(93, 168, 232, alpha),
        1 => ColorRgba::new(136, 207, 190, alpha),
        2 => ColorRgba::new(238, 181, 82, alpha),
        3 => ColorRgba::new(157, 126, 226, alpha),
        4 => ColorRgba::new(226, 126, 74, alpha),
        _ => ColorRgba::new(105, 201, 135, alpha),
    }
}

fn experiment_readiness_label(
    analysis: &ExperimentAnalysisSummary,
    response_count: usize,
) -> String {
    if analysis.run_count == 0 {
        "No run matrix".to_string()
    } else if response_count == 0 {
        "No responses defined".to_string()
    } else if analysis.missing_response_count == 0 {
        "Ready for analysis".to_string()
    } else if analysis.completed_runs > 0 {
        "Partial analysis ready".to_string()
    } else {
        "Capture responses".to_string()
    }
}

fn process_control_view_primitives(app: &FabricadApp, ui_scale: UiScale) -> Vec<ScenePrimitive> {
    let Some(loop_definition) = selected_control_loop(app) else {
        return Vec::new();
    };
    let trend = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id);
    let actions = app
        .workspace
        .process_control
        .actions_for_loop(&loop_definition.id);
    let selected_action = selected_control_action(app)
        .filter(|action| action.loop_id == loop_definition.id)
        .or_else(|| actions.first().copied());
    let mut primitives = Vec::with_capacity(trend.len() * 4 + actions.len() * 4 + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let trend_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(24.0),
        ui_scale.value(430.0),
        ui_scale.value(154.0),
    );
    let action_bounds = UiRect::new(
        frame.x + ui_scale.value(492.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(172.0),
        ui_scale.value(154.0),
    );
    let adjustment_bounds = UiRect::new(
        frame.x + ui_scale.value(704.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(174.0),
        ui_scale.value(154.0),
    );
    add_process_control_trend_primitives(
        &mut primitives,
        trend_bounds,
        loop_definition,
        trend,
        ui_scale,
    );
    add_process_control_action_primitives(
        &mut primitives,
        action_bounds,
        &actions,
        selected_action,
        ui_scale,
    );
    add_process_control_adjustment_primitives(
        &mut primitives,
        adjustment_bounds,
        selected_action,
        ui_scale,
    );
    primitives
}

fn add_process_control_trend_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    loop_definition: &ControlLoop,
    trend: &[ControlTrendPoint],
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, bounds, ui_scale);
    if trend.len() < 2 {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }

    let (min_x, max_x) = trend
        .first()
        .zip(trend.last())
        .map(|(first, last)| (first.source.run_index as f32, last.source.run_index as f32))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for point in trend {
        add_spc_plot_value(&mut min_value, &mut max_value, point.value);
        add_spc_plot_value(&mut min_value, &mut max_value, point.target);
        add_spc_plot_value(
            &mut min_value,
            &mut max_value,
            point.target + point.ewma_error,
        );
    }
    let deadband = loop_definition.deadband.abs();
    add_spc_plot_value(
        &mut min_value,
        &mut max_value,
        loop_definition.output.target - deadband,
    );
    add_spc_plot_value(
        &mut min_value,
        &mut max_value,
        loop_definition.output.target + deadband,
    );
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    let band_top = spc_plot_y(
        bounds,
        min_value,
        max_value,
        loop_definition.output.target + deadband,
    );
    let band_bottom = spc_plot_y(
        bounds,
        min_value,
        max_value,
        loop_definition.output.target - deadband,
    );
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            bounds.x,
            band_top.min(band_bottom),
            bounds.width,
            (band_bottom - band_top).abs().max(ui_scale.value(1.0)),
        ),
        ColorRgba::new(80, 166, 114, 48),
    )));
    add_spc_plot_guides(
        primitives,
        bounds,
        min_value,
        max_value,
        &[SpcPlotGuide {
            value: loop_definition.output.target,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.1),
        }],
    );

    for pair in trend.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].source.run_index as f32),
                spc_plot_y(bounds, min_value, max_value, pair[0].value),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].source.run_index as f32),
                spc_plot_y(bounds, min_value, max_value, pair[1].value),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(93, 168, 232, 235), ui_scale.value(2.0)),
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[0].source.run_index as f32),
                spc_plot_y(
                    bounds,
                    min_value,
                    max_value,
                    pair[0].target + pair[0].ewma_error,
                ),
            ),
            to: UiPoint::new(
                spc_plot_x(bounds, min_x, max_x, pair[1].source.run_index as f32),
                spc_plot_y(
                    bounds,
                    min_value,
                    max_value,
                    pair[1].target + pair[1].ewma_error,
                ),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(238, 181, 82, 210), ui_scale.value(1.3)),
        });
    }

    for point in trend {
        let actionable = point.actionable(loop_definition);
        let center = UiPoint::new(
            spc_plot_x(bounds, min_x, max_x, point.source.run_index as f32),
            spc_plot_y(bounds, min_value, max_value, point.value),
        );
        if actionable {
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(center.x, bounds.y),
                to: UiPoint::new(center.x, bounds.bottom()),
                stroke: StrokeStyle::new(ColorRgba::new(236, 91, 88, 74), ui_scale.value(1.0)),
            });
        }
        primitives.push(ScenePrimitive::Circle {
            center,
            radius: ui_scale.value(if actionable { 4.4 } else { 3.0 }),
            fill: if actionable {
                ColorRgba::new(244, 112, 104, 245)
            } else if point.in_spec {
                ColorRgba::new(105, 201, 135, 245)
            } else {
                ColorRgba::new(238, 181, 82, 245)
            },
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(1.0),
            )),
        });
    }
}

fn add_process_control_action_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    actions: &[&ControlAction],
    selected_action: Option<&ControlAction>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if actions.is_empty() {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    }
    let visible = actions.len().min(7);
    let gap = ui_scale.value(5.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, action) in actions.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        let selected = selected_action.is_some_and(|selected| selected.id == action.id);
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                row,
                if selected {
                    ColorRgba::new(72, 128, 188, 72)
                } else {
                    ColorRgba::new(25, 32, 39, 255)
                },
            )
            .stroke(StrokeStyle::new(
                if selected {
                    ColorRgba::new(252, 253, 255, 210)
                } else {
                    ColorRgba::new(41, 52, 63, 180)
                },
                ui_scale.value(if selected { 1.2 } else { 0.7 }),
            )),
        ));
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                row.x,
                row.y,
                row.width * action.confidence.clamp(0.0, 1.0) as f32,
                row.height,
            ),
            process_control_action_state_color(action.state, 170),
        )));
        let center_y = row.y + row.height * 0.5;
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(row.x + ui_scale.value(8.0), center_y),
            radius: ui_scale.value(3.6),
            fill: process_control_action_state_color(action.state, 245),
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(0.8),
            )),
        });
    }
}

fn add_process_control_adjustment_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    action: Option<&ControlAction>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let Some(action) = action else {
        add_spc_empty_plot(primitives, bounds, ui_scale);
        return;
    };
    if action.adjustments.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 145),
        )));
        return;
    }

    let visible = action.adjustments.len().min(6);
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    let max_delta = action
        .adjustments
        .iter()
        .map(|adjustment| adjustment.delta.abs())
        .fold(0.0_f64, f64::max)
        .max(0.001);
    let center_x = bounds.x + bounds.width * 0.5;
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(center_x, bounds.y),
        to: UiPoint::new(center_x, bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(130, 145, 160, 145), ui_scale.value(1.0)),
    });
    for (index, adjustment) in action.adjustments.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = (adjustment.delta.abs() / max_delta) as f32
            * (bounds.width * 0.46 - ui_scale.value(2.0));
        let bar_height = row_height * 0.68;
        let x = if adjustment.delta >= 0.0 {
            center_x
        } else {
            center_x - width
        };
        let color = if adjustment.delta.abs() < f64::EPSILON {
            ColorRgba::new(91, 190, 130, 180)
        } else if adjustment.delta > 0.0 {
            ColorRgba::new(238, 181, 82, 220)
        } else {
            ColorRgba::new(102, 190, 236, 220)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                x,
                y + (row_height - bar_height) * 0.5,
                width.max(ui_scale.value(2.0)),
                bar_height,
            ),
            color,
        )));

        let range_span = (adjustment.upper_bound - adjustment.lower_bound)
            .abs()
            .max(f64::EPSILON);
        let range_fraction =
            ((adjustment.delta - adjustment.lower_bound) / range_span).clamp(0.0, 1.0) as f32;
        let marker_x = bounds.x + bounds.width * range_fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(marker_x, y),
            to: UiPoint::new(marker_x, y + row_height),
            stroke: StrokeStyle::new(ColorRgba::new(252, 253, 255, 120), ui_scale.value(0.8)),
        });
    }
}

fn process_control_action_state_color(state: ControlActionState, alpha: u8) -> ColorRgba {
    match state {
        ControlActionState::Proposed => ColorRgba::new(93, 168, 232, alpha),
        ControlActionState::Approved => ColorRgba::new(91, 190, 130, alpha),
        ControlActionState::Rejected => ColorRgba::new(236, 91, 88, alpha),
        ControlActionState::Applied => ColorRgba::new(136, 207, 190, alpha),
        ControlActionState::Held => ColorRgba::new(238, 181, 82, alpha),
    }
}

fn process_control_action_label(action: &ControlAction) -> String {
    let adjustment_label = action
        .adjustments
        .first()
        .map(|adjustment| {
            let unit = adjustment.unit.map(|unit| unit.symbol()).unwrap_or("");
            format!("{} {:+.3}{unit}", adjustment.label, adjustment.delta)
        })
        .unwrap_or_else(|| "no recipe adjustment".to_string());
    compact_button_label(
        &format!(
            "{} {} error {:+.3}, confidence {:.0}%, {}",
            short_control_action_label(&action.id),
            action.state.label(),
            action.error,
            action.confidence.clamp(0.0, 1.0) * 100.0,
            adjustment_label
        ),
        92,
    )
}

fn process_control_yield_link_label(app: &FabricadApp, loop_definition: &ControlLoop) -> String {
    let Some(point) = app
        .workspace
        .process_control
        .trend_for_loop(&loop_definition.id)
        .last()
    else {
        return "No linked yield sample".to_string();
    };
    let yield_label = app
        .workspace
        .yield_analysis
        .wafer_summary(&point.source.lot_id, &point.source.wafer_id)
        .map(|summary| percent_label(summary.yield_fraction))
        .or_else(|| point.yield_fraction.map(percent_label))
        .unwrap_or_else(|| "yield n/a".to_string());
    compact_button_label(
        &format!(
            "{yield_label} yield for {}/{} run {}",
            point.source.lot_id, point.source.wafer_id, point.source.tool_run_id
        ),
        70,
    )
}

fn yield_view_primitives(
    app: &FabricadApp,
    lot_id: &str,
    wafer_id: &str,
    outcomes: &[DieOutcome],
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(outcomes.len() * 2 + 90);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let wafer_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(12.0),
        ui_scale.value(178.0),
        ui_scale.value(178.0),
    );
    let wafer_strip = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(286.0),
        ui_scale.value(52.0),
    );
    let failure_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(112.0),
        ui_scale.value(286.0),
        ui_scale.value(70.0),
    );
    let measurement_bounds = UiRect::new(
        frame.x + ui_scale.value(576.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(292.0),
        ui_scale.value(154.0),
    );
    add_yield_die_map_primitives(&mut primitives, wafer_bounds, outcomes, app, ui_scale);
    add_yield_wafer_strip_primitives(
        &mut primitives,
        wafer_strip,
        &app.workspace.yield_analysis,
        lot_id,
        wafer_id,
        ui_scale,
    );
    add_yield_failure_primitives(
        &mut primitives,
        failure_bounds,
        app.workspace
            .yield_analysis
            .wafer_summary(lot_id, wafer_id)
            .or_else(|| app.workspace.yield_analysis.lot_summary(lot_id)),
        ui_scale,
    );
    add_yield_measurement_primitives(
        &mut primitives,
        measurement_bounds,
        &app.workspace
            .yield_analysis
            .measurements_for_wafer(lot_id, wafer_id),
        ui_scale,
    );
    primitives
}

fn add_yield_die_map_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    outcomes: &[DieOutcome],
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let center = UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    let radius = bounds.width.min(bounds.height) * 0.5;
    primitives.push(ScenePrimitive::Circle {
        center,
        radius,
        fill: ColorRgba::new(20, 27, 32, 255),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(96, 111, 126, 255),
            ui_scale.value(1.2),
        )),
    });
    if outcomes.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let min_col = outcomes
        .iter()
        .map(|outcome| outcome.die.column)
        .min()
        .unwrap_or(0);
    let max_col = outcomes
        .iter()
        .map(|outcome| outcome.die.column)
        .max()
        .unwrap_or(0);
    let min_row = outcomes
        .iter()
        .map(|outcome| outcome.die.row)
        .min()
        .unwrap_or(0);
    let max_row = outcomes
        .iter()
        .map(|outcome| outcome.die.row)
        .max()
        .unwrap_or(0);
    let columns = (max_col - min_col + 1).max(1) as f32;
    let rows = (max_row - min_row + 1).max(1) as f32;
    let cell =
        (bounds.width.min(bounds.height) * 0.82 / columns.max(rows)).max(ui_scale.value(3.0));
    for outcome in outcomes {
        if !yield_outcome_matches_filter(outcome, app.yield_map_filter) {
            continue;
        }
        let x = center.x + (outcome.die.column as f32 - (min_col + max_col) as f32 * 0.5) * cell;
        let y = center.y - (outcome.die.row as f32 - (min_row + max_row) as f32 * 0.5) * cell;
        if ((x - center.x).powi(2) + (y - center.y).powi(2)).sqrt() > radius * 0.96 {
            continue;
        }
        let rect = UiRect::new(
            x - cell * 0.42,
            y - cell * 0.42,
            (cell * 0.84).max(ui_scale.value(2.0)),
            (cell * 0.84).max(ui_scale.value(2.0)),
        );
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(rect, yield_outcome_color(outcome, 225)).stroke(
                StrokeStyle::new(ColorRgba::new(16, 20, 24, 150), ui_scale.value(0.7)),
            ),
        ));
        if !outcome.passed {
            primitives.push(ScenePrimitive::Circle {
                center: UiPoint::new(x, y),
                radius: (cell * 0.18).max(ui_scale.value(1.4)),
                fill: ColorRgba::new(252, 253, 255, 210),
                stroke: None,
            });
        }
    }
}

fn add_yield_wafer_strip_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    analysis: &YieldAnalysis,
    lot_id: &str,
    selected_wafer: &str,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let wafers = analysis.wafer_summaries_for_lot(lot_id);
    if wafers.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(2.0);
    let width = (bounds.width - gap * wafers.len().saturating_sub(1) as f32) / wafers.len() as f32;
    for (index, summary) in wafers.iter().enumerate() {
        let wafer_id = summary.wafer_id.as_deref().unwrap_or("");
        let rect = UiRect::new(
            bounds.x + index as f32 * (width + gap),
            bounds.y + ui_scale.value(5.0),
            width.max(ui_scale.value(3.0)),
            bounds.height - ui_scale.value(10.0),
        );
        let stroke = if wafer_id == selected_wafer {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(1.4))
        } else {
            StrokeStyle::new(ColorRgba::new(16, 20, 24, 150), ui_scale.value(0.7))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(rect, yield_fraction_color(summary.yield_fraction, 225))
                .stroke(stroke),
        ));
    }
}

fn add_yield_failure_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    summary: Option<&YieldSummary>,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let Some(summary) = summary else {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    };
    if summary.failure_counts.is_empty() {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                bounds.x + ui_scale.value(8.0),
                bounds.y + ui_scale.value(8.0),
                bounds.width - ui_scale.value(16.0),
                bounds.height - ui_scale.value(16.0),
            ),
            ColorRgba::new(91, 190, 130, 155),
        )));
        return;
    }
    let total = summary.failure_counts.values().sum::<u32>().max(1) as f32;
    let gap = ui_scale.value(4.0);
    let row_height = (bounds.height - gap * summary.failure_counts.len().saturating_sub(1) as f32)
        / summary.failure_counts.len() as f32;
    for (index, (mode, count)) in summary.failure_counts.iter().enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let width = bounds.width * (*count as f32 / total);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(bounds.x, y, width.max(ui_scale.value(2.0)), row_height),
            yield_failure_mode_color(*mode, 215),
        )));
    }
}

fn add_yield_measurement_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    measurements: &[&ProcessMeasurement],
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    if measurements.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let visible = measurements.len().min(8);
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, measurement) in measurements.iter().take(visible).enumerate() {
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let ratio = yield_measurement_ratio(measurement);
        let marker_x = row.x + row.width * ratio.clamp(0.0, 1.0);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(marker_x, row.y),
            to: UiPoint::new(marker_x, row.bottom()),
            stroke: StrokeStyle::new(
                if yield_measurement_excursion(measurement) {
                    ColorRgba::new(236, 91, 88, 230)
                } else {
                    ColorRgba::new(136, 207, 190, 220)
                },
                ui_scale.value(1.4),
            ),
        });
    }
}

fn yield_outcome_matches_filter(outcome: &DieOutcome, filter: YieldMapFilter) -> bool {
    match filter {
        YieldMapFilter::All => true,
        YieldMapFilter::Failing => !outcome.passed,
        YieldMapFilter::Passing => outcome.passed,
    }
}

fn yield_outcome_color(outcome: &DieOutcome, alpha: u8) -> ColorRgba {
    if outcome.passed {
        return ColorRgba::new(91, 190, 130, alpha);
    }
    outcome
        .failure_modes
        .first()
        .map(|mode| yield_failure_mode_color(*mode, alpha))
        .unwrap_or(ColorRgba::new(236, 91, 88, alpha))
}

fn yield_failure_mode_color(mode: FailureMode, alpha: u8) -> ColorRgba {
    match mode {
        FailureMode::OpenCircuit => ColorRgba::new(236, 91, 88, alpha),
        FailureMode::ShortCircuit => ColorRgba::new(226, 126, 74, alpha),
        FailureMode::HighLeakage => ColorRgba::new(238, 181, 82, alpha),
        FailureMode::LowFrequency => ColorRgba::new(102, 190, 236, alpha),
        FailureMode::ParametricDrift => ColorRgba::new(157, 126, 226, alpha),
        FailureMode::ContactResistance => ColorRgba::new(210, 146, 92, alpha),
        FailureMode::EdgeDefect => ColorRgba::new(244, 112, 104, alpha),
    }
}

fn yield_fraction_color(value: f64, alpha: u8) -> ColorRgba {
    if value >= 0.98 {
        ColorRgba::new(91, 190, 130, alpha)
    } else if value >= 0.94 {
        ColorRgba::new(102, 190, 236, alpha)
    } else if value >= 0.90 {
        ColorRgba::new(238, 181, 82, alpha)
    } else {
        ColorRgba::new(236, 91, 88, alpha)
    }
}

fn yield_measurement_ratio(measurement: &ProcessMeasurement) -> f32 {
    match (measurement.lower_spec, measurement.upper_spec) {
        (Some(lower), Some(upper)) if (upper - lower).abs() > f64::EPSILON => {
            ((measurement.value - lower) / (upper - lower)).clamp(0.0, 1.0) as f32
        }
        (Some(lower), Some(_)) => {
            ((measurement.value - lower) / lower.abs().max(1.0) * 0.5 + 0.5).clamp(0.0, 1.0) as f32
        }
        (Some(lower), None) => {
            ((measurement.value - lower) / lower.abs().max(1.0) * 0.5 + 0.5).clamp(0.0, 1.0) as f32
        }
        (None, Some(upper)) => (measurement.value / upper.abs().max(1.0)).clamp(0.0, 1.0) as f32,
        (None, None) => 0.5,
    }
}

fn yield_measurement_excursion(measurement: &ProcessMeasurement) -> bool {
    measurement
        .lower_spec
        .is_some_and(|lower| measurement.value < lower)
        || measurement
            .upper_spec
            .is_some_and(|upper| measurement.value > upper)
}

fn yield_excursion_count(measurements: &[&ProcessMeasurement]) -> usize {
    measurements
        .iter()
        .filter(|measurement| yield_measurement_excursion(measurement))
        .count()
}

fn yield_failure_label(summary: &YieldSummary) -> String {
    compact_button_label(
        &format!(
            "{}; {}; {}",
            summary
                .dominant_failure
                .map(|failure| failure.label())
                .unwrap_or("no dominant failure"),
            summary.spatial_pattern.label(),
            summary
                .root_cause_hints
                .first()
                .map(String::as_str)
                .unwrap_or("no root-cause hint")
        ),
        68,
    )
}

fn metrology_view_primitives(app: &FabricadApp, ui_scale: UiScale) -> Vec<ScenePrimitive> {
    let map = &app.workspace.wafer_map;
    let mut primitives = Vec::with_capacity(map.dies.len() * 3 + map.defects.len() + 80);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(202.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    let wafer_bounds = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(12.0),
        ui_scale.value(178.0),
        ui_scale.value(178.0),
    );
    let histogram_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(26.0),
        ui_scale.value(286.0),
        ui_scale.value(72.0),
    );
    let radial_bounds = UiRect::new(
        frame.x + ui_scale.value(246.0),
        frame.y + ui_scale.value(126.0),
        ui_scale.value(286.0),
        ui_scale.value(56.0),
    );
    let band_bounds = UiRect::new(
        frame.x + ui_scale.value(576.0),
        frame.y + ui_scale.value(28.0),
        ui_scale.value(292.0),
        ui_scale.value(154.0),
    );
    add_metrology_wafer_primitives(&mut primitives, wafer_bounds, app, ui_scale);
    add_metrology_histogram_primitives(
        &mut primitives,
        histogram_bounds,
        map,
        app.metrology_kind,
        ui_scale,
    );
    add_metrology_radial_profile_primitives(
        &mut primitives,
        radial_bounds,
        map,
        app.metrology_kind,
        ui_scale,
    );
    add_metrology_kind_band_primitives(&mut primitives, band_bounds, map, ui_scale);
    primitives
}

fn add_metrology_wafer_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let map = &app.workspace.wafer_map;
    let summary = map.summary(app.metrology_kind);
    let center = UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    let wafer_radius = bounds.width.min(bounds.height) * 0.5;
    primitives.push(ScenePrimitive::Circle {
        center,
        radius: wafer_radius,
        fill: ColorRgba::new(20, 27, 32, 255),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(96, 111, 126, 255),
            ui_scale.value(1.2),
        )),
    });
    primitives.push(ScenePrimitive::Circle {
        center,
        radius: wafer_radius * 0.92,
        fill: ColorRgba::new(0, 0, 0, 0),
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(76, 92, 108, 170),
            ui_scale.value(1.0),
        )),
    });

    let scale = metrology_scene_scale(map, bounds);
    let die_w = (map.geometry.die_size_mm[0] as f32 * scale * 0.78)
        .clamp(ui_scale.value(2.4), ui_scale.value(10.0));
    let die_h = (map.geometry.die_size_mm[1] as f32 * scale * 0.78)
        .clamp(ui_scale.value(2.4), ui_scale.value(10.0));
    for &die in &map.dies {
        let score = metrology_die_attention_score(map, die, app.metrology_kind);
        let selected = app.selected_die == Some(die);
        if app.metrology_failed_only && score == 0 && !selected {
            continue;
        }
        let point = metrology_mm_to_scene(map.geometry.die_center_mm(die), map, bounds);
        let rect = UiRect::new(point.x - die_w * 0.5, point.y - die_h * 0.5, die_w, die_h);
        let stroke = if selected {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(1.6))
        } else if score > 0 {
            StrokeStyle::new(ColorRgba::new(238, 181, 82, 205), ui_scale.value(0.9))
        } else {
            StrokeStyle::new(ColorRgba::new(15, 19, 23, 145), ui_scale.value(0.7))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                rect,
                metrology_die_fill(
                    map,
                    die,
                    app.metrology_kind,
                    app.metrology_map_mode,
                    summary,
                ),
            )
            .stroke(stroke),
        ));
    }

    if app.metrology_map_mode == MetrologyMapMode::OverlayVectors {
        for &die in &map.dies {
            if app.metrology_failed_only
                && metrology_die_attention_score(map, die, app.metrology_kind) == 0
            {
                continue;
            }
            let Some(vector) = metrology_overlay_vector(map, die) else {
                continue;
            };
            let origin = metrology_mm_to_scene(map.geometry.die_center_mm(die), map, bounds);
            primitives.push(ScenePrimitive::Line {
                from: origin,
                to: UiPoint::new(
                    origin.x + vector[0] * ui_scale.value(12.0),
                    origin.y - vector[1] * ui_scale.value(12.0),
                ),
                stroke: StrokeStyle::new(ColorRgba::new(102, 190, 236, 220), ui_scale.value(1.1)),
            });
        }
    }

    if matches!(
        app.metrology_map_mode,
        MetrologyMapMode::DefectReview | MetrologyMapMode::ReviewQueue
    ) {
        for defect in &map.defects {
            if app.metrology_failed_only && app.selected_die != Some(defect.die) {
                continue;
            }
            let point = metrology_mm_to_scene(defect.position_mm, map, bounds);
            primitives.push(ScenePrimitive::Circle {
                center: point,
                radius: ui_scale.value((2.2 + defect.severity as f32).min(6.0)),
                fill: ColorRgba::new(244, 112, 104, 225),
                stroke: Some(StrokeStyle::new(
                    ColorRgba::new(16, 20, 24, 190),
                    ui_scale.value(0.8),
                )),
            });
        }
    }

    for annotation in map.annotations.iter().take(24) {
        let point = metrology_mm_to_scene(annotation.position_mm, map, bounds);
        let size = ui_scale.value(3.8);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(point.x - size, point.y),
            to: UiPoint::new(point.x + size, point.y),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 210), ui_scale.value(1.0)),
        });
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(point.x, point.y - size),
            to: UiPoint::new(point.x, point.y + size),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 210), ui_scale.value(1.0)),
        });
    }
}

fn add_metrology_histogram_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    kind: MeasurementKind,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let bins = map.histogram(kind, 14);
    let max_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0).max(1) as f32;
    if bins.is_empty() {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let gap = ui_scale.value(2.0);
    let bar_width = (bounds.width - gap * bins.len().saturating_sub(1) as f32) / bins.len() as f32;
    for (index, bin) in bins.iter().enumerate() {
        let height = (bin.count as f32 / max_count) * (bounds.height - ui_scale.value(8.0));
        let x = bounds.x + index as f32 * (bar_width + gap);
        let bar = UiRect::new(
            x,
            bounds.bottom() - height,
            bar_width.max(ui_scale.value(2.0)),
            height.max(ui_scale.value(1.0)),
        );
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            bar,
            ColorRgba::new(93, 168, 232, 210),
        )));
    }
}

fn add_metrology_radial_profile_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    kind: MeasurementKind,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let profile = metrology_radial_profile(map, kind, 10);
    if profile.len() < 2 {
        add_metrology_empty_line(primitives, bounds, ui_scale);
        return;
    }
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value) in &profile {
        if value.is_finite() {
            min_value = min_value.min(*value);
            max_value = max_value.max(*value);
        }
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);
    for pair in profile.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                bounds.x + pair[0].0 * bounds.width,
                spc_plot_y(bounds, min_value, max_value, pair[0].1),
            ),
            to: UiPoint::new(
                bounds.x + pair[1].0 * bounds.width,
                spc_plot_y(bounds, min_value, max_value, pair[1].1),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(136, 207, 190, 230), ui_scale.value(1.8)),
        });
    }
}

fn add_metrology_kind_band_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    map: &WaferMap,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    let gap = ui_scale.value(6.0);
    let row_height = (bounds.height - gap * MeasurementKind::ALL.len().saturating_sub(1) as f32)
        / MeasurementKind::ALL.len() as f32;
    for (index, kind) in MeasurementKind::ALL.iter().enumerate() {
        let summary = map.summary(*kind);
        let y = bounds.y + index as f32 * (row_height + gap);
        let row = UiRect::new(bounds.x, y, bounds.width, row_height);
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            row,
            ColorRgba::new(25, 32, 39, 255),
        )));
        let total = summary.sample_count.max(1) as f32;
        let pass_w = row.width * summary.pass_count as f32 / total;
        let fail_w = row.width * summary.fail_count as f32 / total;
        let outlier_w = row.width * summary.outlier_count as f32 / total;
        let mut x = row.x;
        for (width, color) in [
            (pass_w, metrology_status_color(MeasurementStatus::Pass, 205)),
            (fail_w, metrology_status_color(MeasurementStatus::Fail, 215)),
            (
                outlier_w,
                metrology_status_color(MeasurementStatus::Outlier, 215),
            ),
        ] {
            if width <= 0.0 {
                continue;
            }
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x, row.y, width.max(ui_scale.value(1.0)), row.height),
                color,
            )));
            x += width;
        }
    }
}

fn add_metrology_empty_line(
    primitives: &mut Vec<ScenePrimitive>,
    bounds: UiRect,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(
            bounds.x + ui_scale.value(18.0),
            bounds.y + bounds.height * 0.5,
        ),
        to: UiPoint::new(
            bounds.right() - ui_scale.value(18.0),
            bounds.y + bounds.height * 0.5,
        ),
        stroke: StrokeStyle::new(ColorRgba::new(92, 106, 120, 190), ui_scale.value(1.0)),
    });
}

fn metrology_scene_scale(map: &WaferMap, bounds: UiRect) -> f32 {
    let radius = map.geometry.active_radius_mm().max(1.0) as f32;
    bounds.width.min(bounds.height) * 0.5 / radius
}

fn metrology_mm_to_scene(point_mm: [f64; 2], map: &WaferMap, bounds: UiRect) -> UiPoint {
    let scale = metrology_scene_scale(map, bounds);
    UiPoint::new(
        bounds.x + bounds.width * 0.5 + point_mm[0] as f32 * scale,
        bounds.y + bounds.height * 0.5 - point_mm[1] as f32 * scale,
    )
}

fn metrology_die_fill(
    map: &WaferMap,
    die: DieCoord,
    kind: MeasurementKind,
    mode: MetrologyMapMode,
    summary: MeasurementSummary,
) -> ColorRgba {
    let defect_count = map.defects_for_die(die).count();
    let score = metrology_die_attention_score(map, die, kind);
    let Some(measurement) = map.measurement_for(die, kind) else {
        return ColorRgba::new(45, 54, 62, 130);
    };
    match mode {
        MetrologyMapMode::ValueMap => metrology_value_color(measurement.value, summary, 225),
        MetrologyMapMode::DeviationMap => {
            metrology_deviation_color(kind, measurement.value, summary, 225)
        }
        MetrologyMapMode::SpecWindow => metrology_status_color(measurement.status, 225),
        MetrologyMapMode::DefectReview => metrology_defect_density_color(defect_count, 225),
        MetrologyMapMode::ReviewQueue => metrology_attention_score_color(score, 225),
        MetrologyMapMode::OverlayVectors => metrology_status_color(measurement.status, 145),
    }
}

fn metrology_value_color(value: f64, summary: MeasurementSummary, alpha: u8) -> ColorRgba {
    let ratio = match (summary.min, summary.max) {
        (Some(min), Some(max)) if (max - min).abs() > f64::EPSILON => {
            ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
        }
        _ => 0.5,
    };
    if ratio < 0.5 {
        metrology_lerp_color(
            ColorRgba::new(76, 145, 218, alpha),
            ColorRgba::new(91, 190, 130, alpha),
            ratio * 2.0,
        )
    } else {
        metrology_lerp_color(
            ColorRgba::new(91, 190, 130, alpha),
            ColorRgba::new(224, 168, 68, alpha),
            (ratio - 0.5) * 2.0,
        )
    }
}

fn metrology_deviation_color(
    kind: MeasurementKind,
    value: f64,
    summary: MeasurementSummary,
    alpha: u8,
) -> ColorRgba {
    let spec = kind.spec();
    let Some(target) = spec.target else {
        return metrology_value_color(value, summary, alpha);
    };
    let span = spec
        .lower
        .map(|lower| (target - lower).abs())
        .into_iter()
        .chain(spec.upper.map(|upper| (upper - target).abs()))
        .chain(summary.stddev.map(|stddev| stddev * 3.0))
        .fold(1.0_f64, f64::max);
    let ratio = ((value - target) / span).clamp(-1.0, 1.0);
    if ratio.abs() < 0.12 {
        ColorRgba::new(91, 190, 130, alpha)
    } else if ratio > 0.0 {
        metrology_lerp_color(
            ColorRgba::new(224, 168, 68, alpha),
            ColorRgba::new(236, 91, 88, alpha),
            ratio as f32,
        )
    } else {
        metrology_lerp_color(
            ColorRgba::new(102, 190, 236, alpha),
            ColorRgba::new(76, 114, 202, alpha),
            ratio.abs() as f32,
        )
    }
}

fn metrology_status_color(status: MeasurementStatus, alpha: u8) -> ColorRgba {
    match status {
        MeasurementStatus::Pass => ColorRgba::new(91, 190, 130, alpha),
        MeasurementStatus::Fail => ColorRgba::new(236, 91, 88, alpha),
        MeasurementStatus::Outlier => ColorRgba::new(238, 181, 82, alpha),
    }
}

fn metrology_defect_density_color(count: usize, alpha: u8) -> ColorRgba {
    match count {
        0 => ColorRgba::new(72, 130, 105, alpha.saturating_sub(50)),
        1 => ColorRgba::new(238, 181, 82, alpha),
        2 | 3 => ColorRgba::new(226, 126, 74, alpha),
        _ => ColorRgba::new(236, 91, 88, alpha),
    }
}

fn metrology_attention_score_color(score: usize, alpha: u8) -> ColorRgba {
    if score >= 72 {
        ColorRgba::new(236, 91, 88, alpha)
    } else if score >= 42 {
        ColorRgba::new(238, 181, 82, alpha)
    } else if score > 0 {
        ColorRgba::new(102, 190, 236, alpha)
    } else {
        ColorRgba::new(91, 190, 130, alpha.saturating_sub(45))
    }
}

fn metrology_lerp_color(left: ColorRgba, right: ColorRgba, t: f32) -> ColorRgba {
    let t = t.clamp(0.0, 1.0);
    ColorRgba::new(
        lerp_u8(left.r, right.r, t),
        lerp_u8(left.g, right.g, t),
        lerp_u8(left.b, right.b, t),
        lerp_u8(left.a, right.a, t),
    )
}

fn lerp_u8(left: u8, right: u8, t: f32) -> u8 {
    (left as f32 + (right as f32 - left as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn metrology_attention_sites(map: &WaferMap, kind: MeasurementKind) -> Vec<(DieCoord, usize)> {
    let mut sites = map
        .dies
        .iter()
        .copied()
        .map(|die| (die, metrology_die_attention_score(map, die, kind)))
        .filter(|(_, score)| *score > 0)
        .collect::<Vec<_>>();
    sites.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.column.cmp(&right.0.column))
            .then_with(|| left.0.row.cmp(&right.0.row))
    });
    sites
}

fn metrology_die_attention_score(map: &WaferMap, die: DieCoord, kind: MeasurementKind) -> usize {
    let selected_kind_score = map
        .measurement_for(die, kind)
        .map(metrology_measurement_score)
        .unwrap_or_default();
    let other_measurement_score = map
        .measurements_for_die(die)
        .filter(|measurement| measurement.kind != kind)
        .map(|measurement| metrology_measurement_score(measurement) / 2)
        .sum::<usize>();
    let defect_score = map
        .defects_for_die(die)
        .map(|defect| 8 + defect.severity as usize * 4)
        .sum::<usize>();
    let annotation_score = map.annotations_for_die(die).count() * 3;
    selected_kind_score + other_measurement_score + defect_score + annotation_score
}

fn metrology_measurement_score(measurement: &layout_model::metrology::Measurement) -> usize {
    match measurement.status {
        MeasurementStatus::Pass => 0,
        MeasurementStatus::Fail => 44,
        MeasurementStatus::Outlier => 28,
    }
}

fn metrology_overlay_vector(map: &WaferMap, die: DieCoord) -> Option<[f32; 2]> {
    let cd = map.measurement_for(die, MeasurementKind::CriticalDimensionNm)?;
    let thickness = map.measurement_for(die, MeasurementKind::ThicknessNm)?;
    let cd_target = MeasurementKind::CriticalDimensionNm.spec().target?;
    let thickness_target = MeasurementKind::ThicknessNm.spec().target?;
    let x = ((cd.value - cd_target) / 4.0).clamp(-1.0, 1.0) as f32;
    let y = ((thickness.value - thickness_target) / 50.0).clamp(-1.0, 1.0) as f32;
    ((x.abs() + y.abs()) > 0.08).then_some([x, y])
}

fn metrology_radial_profile(
    map: &WaferMap,
    kind: MeasurementKind,
    bin_count: usize,
) -> Vec<(f32, f64)> {
    if bin_count == 0 {
        return Vec::new();
    }
    let radius = map.geometry.active_radius_mm().max(1.0);
    let mut bins = vec![(0.0_f64, 0usize); bin_count];
    for measurement in map
        .measurements
        .iter()
        .filter(|measurement| measurement.kind == kind)
    {
        let [x, y] = map.geometry.die_center_mm(measurement.die);
        let radial = (x.hypot(y) / radius).clamp(0.0, 1.0);
        let index = ((radial * bin_count as f64).floor() as usize).min(bin_count - 1);
        bins[index].0 += measurement.value;
        bins[index].1 += 1;
    }
    bins.into_iter()
        .enumerate()
        .filter_map(|(index, (sum, count))| {
            (count > 0).then_some(((index as f32 + 0.5) / bin_count as f32, sum / count as f64))
        })
        .collect()
}

fn metrology_summary_label(summary: MeasurementSummary) -> String {
    match (summary.mean, summary.stddev) {
        (Some(mean), Some(stddev)) => format!(
            "mean {} +/- {} {}",
            metrology_format_value(summary.kind, mean),
            metrology_format_value(summary.kind, stddev),
            summary.kind.unit()
        ),
        _ => "No summary".to_string(),
    }
}

fn metrology_selected_die_label(map: &WaferMap, selected_die: Option<DieCoord>) -> String {
    let Some(die) = selected_die else {
        return "No selected die - use Next or attention buttons".to_string();
    };
    let measurements = MeasurementKind::ALL
        .iter()
        .filter_map(|kind| {
            map.measurement_for(die, *kind).map(|measurement| {
                format!(
                    "{} {} {}",
                    kind.label(),
                    metrology_format_value(*kind, measurement.value),
                    measurement.status.label()
                )
            })
        })
        .take(3)
        .collect::<Vec<_>>()
        .join("; ");
    let defects = map.defects_for_die(die).count();
    compact_button_label(
        &format!(
            "{}: {}; {} defect(s)",
            die_coord_label(Some(die)),
            measurements,
            defects
        ),
        96,
    )
}

fn metrology_format_value(kind: MeasurementKind, value: f64) -> String {
    match kind {
        MeasurementKind::PassFail | MeasurementKind::DefectCount => format!("{value:.0}"),
        MeasurementKind::SheetResistanceOhmsPerSq => format!("{value:.1}"),
        MeasurementKind::ThicknessNm | MeasurementKind::CriticalDimensionNm => {
            format!("{value:.2}")
        }
    }
}

#[derive(Clone, Copy)]
struct SpcPlotGuide {
    value: f64,
    color: ColorRgba,
    width: f32,
}

#[derive(Clone, Copy)]
struct SpcPlotHighlight {
    start_x: f32,
    end_x: f32,
    severity: MonitorSeverity,
}

fn spc_fdc_monitor_primitives(
    chart: Option<&ControlChart>,
    trace: Option<&SensorTrace>,
    monitor: &SpcFdcMonitor,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let mut primitives = Vec::with_capacity(120);
    let frame = UiRect::new(
        ui_scale.value(30.0),
        ui_scale.value(18.0),
        ui_scale.value(900.0),
        ui_scale.value(190.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));

    let chart_plot = UiRect::new(
        frame.x + ui_scale.value(22.0),
        frame.y + ui_scale.value(22.0),
        ui_scale.value(402.0),
        ui_scale.value(132.0),
    );
    let trace_plot = UiRect::new(
        frame.x + ui_scale.value(476.0),
        frame.y + ui_scale.value(22.0),
        ui_scale.value(402.0),
        ui_scale.value(132.0),
    );
    add_spc_chart_plot_primitives(&mut primitives, chart_plot, chart, ui_scale);
    add_fdc_trace_plot_primitives(&mut primitives, trace_plot, trace, ui_scale);
    add_spc_alarm_strip_primitives(
        &mut primitives,
        UiRect::new(
            frame.x + ui_scale.value(22.0),
            frame.bottom() - ui_scale.value(24.0),
            frame.width - ui_scale.value(44.0),
            ui_scale.value(10.0),
        ),
        monitor,
        ui_scale,
    );
    primitives
}

fn add_spc_chart_plot_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    chart: Option<&ControlChart>,
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, plot, ui_scale);
    let Some(chart) = chart else {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    };
    if chart.points.is_empty() {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    }

    let values = chart
        .points
        .iter()
        .map(|point| {
            (
                point.sequence as f32,
                point.value,
                !chart.limits.contains(point.value),
            )
        })
        .collect::<Vec<_>>();
    let (min_x, max_x) = values
        .first()
        .zip(values.last())
        .map(|(first, last)| (first.0, last.0))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value, _) in &values {
        add_spc_plot_value(&mut min_value, &mut max_value, *value);
    }
    for guide in [
        chart.limits.lower_control,
        chart.limits.upper_control,
        chart.limits.center,
        chart.limits.two_sigma_low(),
        chart.limits.two_sigma_high(),
    ] {
        add_spc_plot_value(&mut min_value, &mut max_value, guide);
    }
    if let Some(lower_spec) = chart.limits.lower_spec {
        add_spc_plot_value(&mut min_value, &mut max_value, lower_spec);
    }
    if let Some(upper_spec) = chart.limits.upper_spec {
        add_spc_plot_value(&mut min_value, &mut max_value, upper_spec);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    for violation in &chart.violations {
        if let Some(highlight) = spc_chart_highlight(chart, violation) {
            add_spc_plot_highlight(primitives, plot, min_x, max_x, highlight, ui_scale);
        }
    }

    let mut guides = vec![
        SpcPlotGuide {
            value: chart.limits.upper_control,
            color: ColorRgba::new(130, 145, 160, 200),
            width: ui_scale.value(1.2),
        },
        SpcPlotGuide {
            value: chart.limits.lower_control,
            color: ColorRgba::new(130, 145, 160, 200),
            width: ui_scale.value(1.2),
        },
        SpcPlotGuide {
            value: chart.limits.center,
            color: ColorRgba::new(93, 168, 232, 230),
            width: ui_scale.value(1.1),
        },
        SpcPlotGuide {
            value: chart.limits.two_sigma_high(),
            color: ColorRgba::new(220, 176, 72, 165),
            width: ui_scale.value(0.9),
        },
        SpcPlotGuide {
            value: chart.limits.two_sigma_low(),
            color: ColorRgba::new(220, 176, 72, 165),
            width: ui_scale.value(0.9),
        },
    ];
    if let Some(upper_spec) = chart.limits.upper_spec {
        guides.push(SpcPlotGuide {
            value: upper_spec,
            color: ColorRgba::new(226, 96, 96, 180),
            width: ui_scale.value(1.0),
        });
    }
    if let Some(lower_spec) = chart.limits.lower_spec {
        guides.push(SpcPlotGuide {
            value: lower_spec,
            color: ColorRgba::new(226, 96, 96, 180),
            width: ui_scale.value(1.0),
        });
    }
    add_spc_plot_guides(primitives, plot, min_value, max_value, &guides);
    add_spc_plot_values(
        primitives, plot, min_x, max_x, min_value, max_value, &values, ui_scale,
    );
}

fn add_fdc_trace_plot_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    trace: Option<&SensorTrace>,
    ui_scale: UiScale,
) {
    add_spc_plot_background(primitives, plot, ui_scale);
    let Some(trace) = trace else {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    };
    if trace.samples.is_empty() {
        add_spc_empty_plot(primitives, plot, ui_scale);
        return;
    }

    let values = trace
        .samples
        .iter()
        .map(|point| {
            (
                point.at_s as f32,
                point.value,
                !trace.limit.contains(point.value),
            )
        })
        .collect::<Vec<_>>();
    let (min_x, max_x) = values
        .first()
        .zip(values.last())
        .map(|(first, last)| (first.0, last.0))
        .unwrap_or((0.0, 1.0));
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for (_, value, _) in &values {
        add_spc_plot_value(&mut min_value, &mut max_value, *value);
    }
    if let Some(lower) = trace.limit.lower {
        add_spc_plot_value(&mut min_value, &mut max_value, lower);
    }
    if let Some(upper) = trace.limit.upper {
        add_spc_plot_value(&mut min_value, &mut max_value, upper);
    }
    let (min_value, max_value) = padded_plot_range(min_value, max_value);

    let span = trace_sample_span(trace);
    for violation in &trace.violations {
        add_spc_plot_highlight(
            primitives,
            plot,
            min_x,
            max_x,
            SpcPlotHighlight {
                start_x: violation.at_s as f32 - span * 0.35,
                end_x: violation.at_s as f32 + span * 0.35,
                severity: violation.severity,
            },
            ui_scale,
        );
    }

    let mut guides = Vec::new();
    if let Some(lower) = trace.limit.lower {
        guides.push(SpcPlotGuide {
            value: lower,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.2),
        });
    }
    if let Some(upper) = trace.limit.upper {
        guides.push(SpcPlotGuide {
            value: upper,
            color: ColorRgba::new(130, 145, 160, 205),
            width: ui_scale.value(1.2),
        });
    }
    add_spc_plot_guides(primitives, plot, min_value, max_value, &guides);
    add_spc_plot_values(
        primitives, plot, min_x, max_x, min_value, max_value, &values, ui_scale,
    );
}

fn add_spc_plot_background(primitives: &mut Vec<ScenePrimitive>, plot: UiRect, ui_scale: UiScale) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(plot, ColorRgba::new(17, 23, 28, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(55, 68, 82, 255),
            ui_scale.value(1.0),
        )),
    ));
    for fraction in [0.25_f32, 0.5, 0.75] {
        let x = plot.x + plot.width * fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(122, 138, 154, 46), ui_scale.value(1.0)),
        });
        let y = plot.y + plot.height * fraction;
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, y),
            to: UiPoint::new(plot.right(), y),
            stroke: StrokeStyle::new(ColorRgba::new(122, 138, 154, 46), ui_scale.value(1.0)),
        });
    }
}

fn add_spc_empty_plot(primitives: &mut Vec<ScenePrimitive>, plot: UiRect, ui_scale: UiScale) {
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(plot.x + ui_scale.value(24.0), plot.y + plot.height * 0.5),
        to: UiPoint::new(
            plot.right() - ui_scale.value(24.0),
            plot.y + plot.height * 0.5,
        ),
        stroke: StrokeStyle::new(ColorRgba::new(92, 106, 120, 190), ui_scale.value(1.0)),
    });
}

fn add_spc_plot_guides(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_value: f64,
    max_value: f64,
    guides: &[SpcPlotGuide],
) {
    for guide in guides {
        let y = spc_plot_y(plot, min_value, max_value, guide.value);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, y),
            to: UiPoint::new(plot.right(), y),
            stroke: StrokeStyle::new(guide.color, guide.width),
        });
    }
}

fn add_spc_plot_highlight(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_x: f32,
    max_x: f32,
    highlight: SpcPlotHighlight,
    ui_scale: UiScale,
) {
    let left = spc_plot_x(plot, min_x, max_x, highlight.start_x);
    let right = spc_plot_x(plot, min_x, max_x, highlight.end_x);
    primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
        UiRect::new(
            left.min(right),
            plot.y,
            (right - left).abs().max(ui_scale.value(2.0)),
            plot.height,
        ),
        spc_severity_color(highlight.severity, 42),
    )));
}

fn add_spc_plot_values(
    primitives: &mut Vec<ScenePrimitive>,
    plot: UiRect,
    min_x: f32,
    max_x: f32,
    min_value: f64,
    max_value: f64,
    values: &[(f32, f64, bool)],
    ui_scale: UiScale,
) {
    for pair in values.windows(2) {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, pair[0].0),
                spc_plot_y(plot, min_value, max_value, pair[0].1),
            ),
            to: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, pair[1].0),
                spc_plot_y(plot, min_value, max_value, pair[1].1),
            ),
            stroke: StrokeStyle::new(ColorRgba::new(82, 156, 219, 235), ui_scale.value(2.0)),
        });
    }
    for (index, (x, value, outside)) in values.iter().enumerate() {
        let latest = index + 1 == values.len();
        primitives.push(ScenePrimitive::Circle {
            center: UiPoint::new(
                spc_plot_x(plot, min_x, max_x, *x),
                spc_plot_y(plot, min_value, max_value, *value),
            ),
            radius: ui_scale.value(if latest { 4.7 } else { 3.4 }),
            fill: if *outside {
                ColorRgba::new(244, 112, 104, 255)
            } else {
                ColorRgba::new(105, 201, 135, 245)
            },
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(15, 19, 23, 190),
                ui_scale.value(1.0),
            )),
        });
    }
}

fn add_spc_alarm_strip_primitives(
    primitives: &mut Vec<ScenePrimitive>,
    strip: UiRect,
    monitor: &SpcFdcMonitor,
    ui_scale: UiScale,
) {
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(strip, ColorRgba::new(27, 35, 42, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(54, 66, 78, 255),
            ui_scale.value(1.0),
        )),
    ));
    if monitor.alarm_summary.active_count == 0 {
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(strip.x, strip.y, strip.width, strip.height),
            ColorRgba::new(80, 166, 114, 135),
        )));
        return;
    }
    let visible = monitor.alarm_summary.by_tool.len().max(1).min(10);
    let gap = ui_scale.value(3.0);
    let item_width = (strip.width - gap * visible.saturating_sub(1) as f32) / visible as f32;
    for (index, count) in monitor
        .alarm_summary
        .by_tool
        .values()
        .take(visible)
        .enumerate()
    {
        let color = if *count > 1 {
            ColorRgba::new(236, 91, 88, 185)
        } else {
            ColorRgba::new(238, 181, 82, 175)
        };
        primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
            UiRect::new(
                strip.x + index as f32 * (item_width + gap),
                strip.y,
                item_width,
                strip.height,
            ),
            color,
        )));
    }
}

fn spc_plot_x(plot: UiRect, min_x: f32, max_x: f32, x: f32) -> f32 {
    let span = (max_x - min_x).abs().max(1.0);
    plot.x + ((x - min_x) / span).clamp(0.0, 1.0) * plot.width
}

fn spc_plot_y(plot: UiRect, min_value: f64, max_value: f64, value: f64) -> f32 {
    let span = (max_value - min_value).abs().max(f64::EPSILON);
    let fraction = ((value - min_value) / span) as f32;
    plot.bottom() - fraction.clamp(0.0, 1.0) * plot.height
}

fn add_spc_plot_value(min_value: &mut f64, max_value: &mut f64, value: f64) {
    if value.is_finite() {
        *min_value = min_value.min(value);
        *max_value = max_value.max(value);
    }
}

fn padded_plot_range(min_value: f64, max_value: f64) -> (f64, f64) {
    if !min_value.is_finite() || !max_value.is_finite() {
        return (0.0, 1.0);
    }
    let padding = ((max_value - min_value).abs() * 0.12).max(0.5);
    (min_value - padding, max_value + padding)
}

fn spc_chart_highlight(
    chart: &ControlChart,
    violation: &layout_model::spc_fdc::RuleViolation,
) -> Option<SpcPlotHighlight> {
    let mut points = violation
        .point_indices
        .iter()
        .filter_map(|index| chart.points.get(*index));
    let first = points.next()?;
    let mut start_x = first.sequence as f32;
    let mut end_x = start_x;
    for point in points {
        let x = point.sequence as f32;
        start_x = start_x.min(x);
        end_x = end_x.max(x);
    }
    let padding = if (end_x - start_x).abs() < f32::EPSILON {
        0.35
    } else {
        0.2
    };
    Some(SpcPlotHighlight {
        start_x: start_x - padding,
        end_x: end_x + padding,
        severity: violation.severity,
    })
}

fn trace_sample_span(trace: &SensorTrace) -> f32 {
    let span = trace
        .samples
        .windows(2)
        .filter_map(|pair| {
            let span = pair[1].at_s.saturating_sub(pair[0].at_s);
            (span > 0).then_some(span as f32)
        })
        .fold(f32::INFINITY, f32::min);
    if span.is_finite() { span.max(1.0) } else { 1.0 }
}

fn spc_severity_color(severity: MonitorSeverity, alpha: u8) -> ColorRgba {
    match severity {
        MonitorSeverity::Advisory => ColorRgba::new(88, 178, 232, alpha),
        MonitorSeverity::Warning => ColorRgba::new(238, 181, 82, alpha),
        MonitorSeverity::Critical => ColorRgba::new(236, 91, 88, alpha),
    }
}

fn spc_chart_summary_label(chart: &ControlChart) -> String {
    let latest = chart
        .latest_point()
        .map(|point| {
            format!(
                "{} {} on {}",
                spc_compact_number(point.value),
                chart.unit,
                point.wafer_id
            )
        })
        .unwrap_or_else(|| "no samples".to_string());
    compact_button_label(
        &format!(
            "{}: {latest}; {}; {} rules",
            chart.metric.replace('_', " "),
            spc_chart_trend_label(chart),
            chart.violations.len()
        ),
        78,
    )
}

fn spc_trace_summary_label(monitor: &SpcFdcMonitor, trace: &SensorTrace) -> String {
    let latest = trace
        .latest_point()
        .map(|point| {
            format!(
                "{} {} at t+{}s",
                spc_compact_number(point.value),
                trace.unit,
                point.at_s
            )
        })
        .unwrap_or_else(|| "no samples".to_string());
    compact_button_label(
        &format!(
            "{}: {latest}; {}; {} alarms",
            trace.display_name(),
            spc_trace_range_label(trace),
            same_tool_alarm_count(monitor, &trace.tool_id)
        ),
        78,
    )
}

fn spc_chart_trend_label(chart: &ControlChart) -> String {
    let Some(latest) = chart.points.last() else {
        return "n/a".to_string();
    };
    let Some(previous) = chart.points.iter().rev().nth(1) else {
        return "single point".to_string();
    };
    let delta = latest.value - previous.value;
    let direction = if delta.abs() <= chart.limits.one_sigma * 0.05 {
        "flat"
    } else if delta > 0.0 {
        "rising"
    } else {
        "falling"
    };
    let signed_delta = if delta >= 0.0 {
        format!("+{}", spc_compact_number(delta))
    } else {
        spc_compact_number(delta)
    };
    format!("{direction} ({signed_delta} {})", chart.unit)
}

fn spc_trace_range_label(trace: &SensorTrace) -> String {
    let Some(first) = trace.samples.first() else {
        return "n/a".to_string();
    };
    let (min_value, max_value) = trace.samples.iter().fold(
        (first.value, first.value),
        |(min_value, max_value), sample| (min_value.min(sample.value), max_value.max(sample.value)),
    );
    format!(
        "{}..{} {}",
        spc_compact_number(min_value),
        spc_compact_number(max_value),
        trace.unit
    )
}

fn same_tool_alarm_count(monitor: &SpcFdcMonitor, tool_id: &str) -> usize {
    monitor
        .findings
        .iter()
        .filter(|finding| {
            finding.source == FindingSource::EquipmentAlarm
                && finding.tool_id.as_deref() == Some(tool_id)
        })
        .count()
}

fn spc_compact_number(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn scheduler_timeline_primitives(
    app: &FabricadApp,
    result: &layout_model::scheduler::DispatchResult,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let tools = app
        .workspace
        .scheduler
        .tools
        .iter()
        .take(8)
        .collect::<Vec<_>>();
    let mut primitives = Vec::with_capacity(tools.len() * 8 + result.assignments.len() * 4 + 20);
    let frame = UiRect::new(
        ui_scale.value(42.0),
        ui_scale.value(22.0),
        ui_scale.value(864.0),
        ui_scale.value(174.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(frame, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    if tools.is_empty() {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(frame.x + ui_scale.value(24.0), frame.y + frame.height * 0.5),
            to: UiPoint::new(
                frame.right() - ui_scale.value(24.0),
                frame.y + frame.height * 0.5,
            ),
            stroke: StrokeStyle::new(ColorRgba::new(80, 94, 108, 255), ui_scale.value(1.0)),
        });
        return primitives;
    }

    let start = result
        .assignments
        .iter()
        .map(|assignment| assignment.start_minute)
        .min()
        .unwrap_or(app.workspace.scheduler.now_minute)
        .min(app.workspace.scheduler.now_minute);
    let end = result
        .assignments
        .iter()
        .map(|assignment| assignment.finish_minute)
        .max()
        .unwrap_or(start + 60)
        .max(start + 60);
    let label_width = ui_scale.value(92.0);
    let plot = UiRect::new(
        frame.x + label_width,
        frame.y + ui_scale.value(22.0),
        frame.width - label_width - ui_scale.value(10.0),
        frame.height - ui_scale.value(32.0),
    );

    for tick in scheduler_hourly_ticks(start, end) {
        let x = scheduler_time_x(plot, start, end, tick);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(116, 132, 148, 70), ui_scale.value(1.0)),
        });
    }
    if app.workspace.scheduler.now_minute >= start && app.workspace.scheduler.now_minute <= end {
        let x = scheduler_time_x(plot, start, end, app.workspace.scheduler.now_minute);
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, plot.y - ui_scale.value(6.0)),
            to: UiPoint::new(x, plot.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(88, 178, 232, 240), ui_scale.value(1.7)),
        });
    }

    let row_h = (plot.height / tools.len().max(1) as f32).max(ui_scale.value(20.0));
    for (index, tool) in tools.iter().enumerate() {
        let row_y = plot.y + index as f32 * row_h;
        let row = UiRect::new(
            plot.x,
            row_y + ui_scale.value(3.0),
            plot.width,
            row_h - ui_scale.value(6.0),
        );
        if app.selected_scheduler_tool.as_ref() == Some(&tool.id) {
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                row,
                ColorRgba::new(72, 128, 188, 58),
            )));
        }
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(
                    frame.x + ui_scale.value(12.0),
                    row.y + row.height * 0.5 - ui_scale.value(5.0),
                    ui_scale.value(64.0),
                    ui_scale.value(10.0),
                ),
                scheduler_tool_state_color(tool.state),
            )
            .stroke(StrokeStyle::new(
                ColorRgba::new(14, 18, 22, 170),
                ui_scale.value(1.0),
            )),
        ));
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(plot.x, row.bottom()),
            to: UiPoint::new(plot.right(), row.bottom()),
            stroke: StrokeStyle::new(ColorRgba::new(42, 52, 62, 190), ui_scale.value(1.0)),
        });
        for window in &tool.maintenance_windows {
            if window.end_minute < start || window.start_minute > end {
                continue;
            }
            let x0 = scheduler_time_x(plot, start, end, window.start_minute.max(start));
            let x1 = scheduler_time_x(plot, start, end, window.end_minute.min(end));
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x0, row.y, (x1 - x0).max(ui_scale.value(2.0)), row.height),
                ColorRgba::new(224, 178, 72, 72),
            )));
        }
    }

    for assignment in &result.assignments {
        let Some(row_index) = tools.iter().position(|tool| tool.id == assignment.tool_id) else {
            continue;
        };
        let row_y = plot.y + row_index as f32 * row_h;
        let x0 = scheduler_time_x(plot, start, end, assignment.start_minute);
        let x1 = scheduler_time_x(plot, start, end, assignment.finish_minute);
        let bar = UiRect::new(
            x0,
            row_y + ui_scale.value(6.0),
            (x1 - x0).max(ui_scale.value(8.0)),
            (row_h - ui_scale.value(12.0)).max(ui_scale.value(8.0)),
        );
        if assignment.due_at_minute >= start && assignment.due_at_minute <= end {
            let due_x = scheduler_time_x(plot, start, end, assignment.due_at_minute);
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(due_x, bar.y - ui_scale.value(3.0)),
                to: UiPoint::new(due_x, bar.bottom() + ui_scale.value(3.0)),
                stroke: StrokeStyle::new(
                    if assignment.tardy_minutes > 0 {
                        ColorRgba::new(236, 91, 88, 245)
                    } else {
                        ColorRgba::new(178, 188, 198, 190)
                    },
                    ui_scale.value(1.4),
                ),
            });
        }
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(bar, scheduler_assignment_color(assignment.priority)).stroke(
                StrokeStyle::new(
                    if assignment.tardy_minutes > 0 {
                        ColorRgba::new(236, 91, 88, 255)
                    } else {
                        ColorRgba::new(16, 20, 24, 170)
                    },
                    ui_scale.value(if assignment.tardy_minutes > 0 {
                        2.0
                    } else {
                        1.0
                    }),
                ),
            ),
        ));
    }
    primitives
}

fn scheduler_time_x(plot: UiRect, start: u32, end: u32, minute: u32) -> f32 {
    let span = end.saturating_sub(start).max(1) as f32;
    plot.x + plot.width * minute.saturating_sub(start) as f32 / span
}

fn scheduler_hourly_ticks(start: u32, end: u32) -> Vec<u32> {
    let mut tick = start - start % 60;
    if tick < start {
        tick += 60;
    }
    let mut ticks = Vec::new();
    while tick <= end {
        ticks.push(tick);
        tick += 60;
    }
    ticks
}

fn scheduler_tool_state_color(state: layout_model::scheduler::ToolDispatchState) -> ColorRgba {
    match state {
        layout_model::scheduler::ToolDispatchState::Available => ColorRgba::new(66, 166, 118, 230),
        layout_model::scheduler::ToolDispatchState::Maintenance => {
            ColorRgba::new(224, 176, 72, 230)
        }
        layout_model::scheduler::ToolDispatchState::Down => ColorRgba::new(224, 84, 88, 230),
    }
}

fn scheduler_assignment_color(priority: u8) -> ColorRgba {
    match priority {
        0 | 1 => ColorRgba::new(86, 158, 216, 230),
        2 | 3 => ColorRgba::new(72, 174, 132, 230),
        _ => ColorRgba::new(230, 170, 76, 235),
    }
}

fn scheduler_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let dispatch = app.workspace.scheduler.dispatch(app.scheduler_policy);
    let mut rows = dispatch
        .assignments
        .iter()
        .filter(|assignment| {
            !app.scheduler_focus_selected_tool
                || app.selected_scheduler_tool.as_ref() == Some(&assignment.tool_id)
        })
        .take(10)
        .map(|assignment| {
            PrimaryRow::new(
                format!("dispatch-{}-{}", assignment.lot_id, assignment.tool_id),
                assignment.lot_id.to_string(),
                assignment.tool_id.to_string(),
                format!(
                    "P{} start {} finish {}; tardy {} min",
                    assignment.priority,
                    assignment.start_minute,
                    assignment.finish_minute,
                    assignment.tardy_minutes
                ),
            )
            .action(
                format!("fabricad.viewctl.scheduler.tool.{}", assignment.tool_id),
                app.selected_scheduler_tool.as_ref() == Some(&assignment.tool_id),
            )
        })
        .collect::<Vec<_>>();
    if rows.len() < 10 {
        rows.extend(
            dispatch
                .recommendations
                .iter()
                .take(10 - rows.len())
                .map(|recommendation| {
                    let lot = recommendation
                        .lot_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "idle".to_string());
                    PrimaryRow::new(
                        format!("recommend-{}", recommendation.tool_id),
                        format!("Recommend {}", recommendation.tool_id),
                        lot,
                        recommendation.reason.clone(),
                    )
                    .action(
                        format!("fabricad.viewctl.scheduler.tool.{}", recommendation.tool_id),
                        app.selected_scheduler_tool.as_ref() == Some(&recommendation.tool_id),
                    )
                }),
        );
    }
    rows
}

fn safety_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let safety = &app.workspace.safety;
    let lockouts = safety.evaluate_lockouts();
    let mut rows = safety
        .active_conditions()
        .into_iter()
        .take(3)
        .map(|sensor| {
            let sensor_id = sensor.id.to_string();
            PrimaryRow::new(
                format!("condition-{sensor_id}"),
                format!("{} {}", sensor.severity.label(), sensor.name),
                format!("{} {}", sensor.state.label(), sensor.domain.label()),
                format!(
                    "{} {}; limit {}; routed {}",
                    spc_compact_number(sensor.value),
                    sensor.unit,
                    safety_limit_label(&sensor.limit),
                    safety
                        .route_targets_for(sensor)
                        .into_iter()
                        .map(|target| target.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .action(
                format!("fabricad.viewctl.safety.ack.condition.{sensor_id}"),
                app.acknowledged_conditions.contains(&sensor_id),
            )
        })
        .collect::<Vec<_>>();

    rows.extend(
        lockouts
            .iter()
            .take(3)
            .map(|lockout| {
                PrimaryRow::new(
                    format!("lockout-{}", lockout.tool_id),
                    format!("{} {}", lockout.tool_id, lockout.tool_name),
                    if lockout.locked_out {
                        "Locked out"
                    } else {
                        "Clear"
                    },
                    if lockout.reasons.is_empty() {
                        "All simulated interlocks clear".to_string()
                    } else {
                        lockout.reasons.join("; ")
                    },
                )
                .action(
                    format!("fabricad.viewctl.safety.tool.{}", lockout.tool_id),
                    app.selected_safety_tool.as_deref() == Some(lockout.tool_id.as_str()),
                )
            })
            .take(10usize.saturating_sub(rows.len())),
    );

    if let Some(lockout) = selected_safety_lockout(app) {
        rows.extend(
            safety_selected_tool_rows(app, &lockout)
                .into_iter()
                .take(10usize.saturating_sub(rows.len())),
        );
    }

    rows.extend(
        safety
            .alarm_routes
            .iter()
            .take(10usize.saturating_sub(rows.len()).min(2))
            .map(|route| {
                PrimaryRow::new(
                    format!(
                        "route-{}-{}-{}",
                        route.domain.label(),
                        route.minimum_severity.label(),
                        route.target.label()
                    ),
                    format!(
                        "{} {}",
                        route.domain.label(),
                        route.minimum_severity.label()
                    ),
                    route.target.label(),
                    format!("channel {}", route.channel),
                )
            }),
    );

    if rows.len() < 10 {
        rows.extend(
            safety
                .incidents
                .iter()
                .take(10 - rows.len())
                .map(|incident| {
                    let incident_id = incident.id.to_string();
                    PrimaryRow::new(
                        format!("incident-{incident_id}"),
                        incident_id.clone(),
                        format!("{} {}", incident.severity.label(), incident.status.label()),
                        format!("{}; {}", incident.domain.label(), incident.summary),
                    )
                    .action(
                        format!("fabricad.viewctl.safety.ack.incident.{incident_id}"),
                        app.acknowledged_incidents.contains(&incident_id),
                    )
                }),
        );
    }
    if rows.len() < 10 {
        rows.extend(
            safety
                .audit_events
                .iter()
                .take(10 - rows.len())
                .map(|event| {
                    PrimaryRow::new(
                        format!("audit-{}", event.sequence),
                        format!("#{} {}", event.sequence, event.kind.label()),
                        event.actor.clone(),
                        format!("{}; {}", event.timestamp, event.message),
                    )
                }),
        );
    }
    rows
}

fn safety_selected_tool_rows(
    app: &FabricadApp,
    lockout: &layout_model::safety::ToolLockout,
) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            format!("selected-lockout-{}", lockout.tool_id),
            format!("Selected {}", lockout.tool_name),
            if lockout.locked_out {
                "blocked"
            } else {
                "clear"
            },
            if lockout.reasons.is_empty() {
                "All simulated interlocks clear".to_string()
            } else {
                lockout.reasons.join("; ")
            },
        )
        .action(
            format!("fabricad.viewctl.safety.ack.lockout.{}", lockout.tool_id),
            app.acknowledged_lockouts.contains(&lockout.tool_id),
        ),
    ];
    let Some(interlock) = app
        .workspace
        .safety
        .tool_interlocks
        .iter()
        .find(|interlock| interlock.tool_id == lockout.tool_id)
    else {
        return rows;
    };
    for sensor_id in &interlock.required_sensors {
        let Some(sensor) = app
            .workspace
            .safety
            .sensors
            .iter()
            .find(|sensor| &sensor.id == sensor_id)
        else {
            continue;
        };
        rows.push(PrimaryRow::new(
            format!("permissive-{}-{}", lockout.tool_id, sensor.id),
            format!(
                "{} {}",
                if sensor.fails_interlock() {
                    "blocked"
                } else {
                    "pass"
                },
                sensor.name
            ),
            sensor.domain.label(),
            format!(
                "{} {} / state {} / limit {} / last {}s",
                spc_compact_number(sensor.value),
                sensor.unit,
                sensor.state.label(),
                safety_limit_label(&sensor.limit),
                sensor.last_seen_s
            ),
        ));
    }
    rows
}

fn safety_limit_label(limit: &layout_model::safety::SafetyLimit) -> String {
    match (limit.lower, limit.upper) {
        (Some(lower), Some(upper)) => {
            format!(
                "{}..{}",
                spc_compact_number(lower),
                spc_compact_number(upper)
            )
        }
        (Some(lower), None) => format!(">= {}", spc_compact_number(lower)),
        (None, Some(upper)) => format!("<= {}", spc_compact_number(upper)),
        (None, None) => "none".to_string(),
    }
}

fn traceability_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let Some(lot_id) = app
        .selected_trace_lot
        .clone()
        .or_else(|| app.workspace.genealogy.lot_ids().into_iter().next())
    else {
        return Vec::new();
    };
    let related_wafers = selected_trace_wafer_ref(app)
        .map(|wafer| {
            app.workspace
                .genealogy
                .wafer_lineage(&wafer)
                .into_iter()
                .chain(app.workspace.genealogy.wafer_descendants(&wafer))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    app.workspace
        .genealogy
        .wafer_refs_for_lot(&lot_id)
        .into_iter()
        .filter(|wafer| {
            !app.trace_related_only || related_wafers.is_empty() || related_wafers.contains(wafer)
        })
        .take(4)
        .map(|wafer| trace_wafer_primary_row(app, wafer))
        .chain(trace_selected_wafer_rows(app).into_iter().take(6))
        .take(10)
        .collect()
}

fn trace_wafer_primary_row(app: &FabricadApp, wafer: WaferRef) -> PrimaryRow {
    let lineage = app.workspace.genealogy.wafer_lineage(&wafer).len();
    let descendants = app.workspace.genealogy.wafer_descendants(&wafer).len();
    let slot = app
        .workspace
        .genealogy
        .wafer(&wafer)
        .map(|wafer| format!("slot {}", wafer.slot))
        .unwrap_or_else(|| "missing wafer".to_string());
    PrimaryRow::new(
        format!("trace-{}-{}", wafer.lot_id, wafer.wafer_id),
        wafer.wafer_id.to_string(),
        format!("{} {slot}", wafer.lot_id),
        format!("lineage {lineage}; descendants {descendants}"),
    )
    .action(
        trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
        app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
    )
}

fn trace_selected_wafer_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let Some(selected_wafer) = selected_trace_wafer_ref(app) else {
        return Vec::new();
    };
    let genealogy = &app.workspace.genealogy;
    let mut rows = Vec::new();
    rows.extend(
        genealogy
            .wafer_lineage(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|wafer| {
                PrimaryRow::new(
                    format!("trace-lineage-{}-{}", wafer.lot_id, wafer.wafer_id),
                    if wafer == selected_wafer {
                        "Selected wafer".to_string()
                    } else {
                        "Parent wafer".to_string()
                    },
                    wafer.to_string(),
                    trace_latest_step_label(app, &wafer),
                )
                .action(
                    trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
                    app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
                )
            }),
    );
    rows.extend(
        genealogy
            .wafer_descendants(&selected_wafer)
            .into_iter()
            .take(2)
            .map(|wafer| {
                PrimaryRow::new(
                    format!("trace-child-{}-{}", wafer.lot_id, wafer.wafer_id),
                    "Child wafer",
                    wafer.to_string(),
                    trace_latest_step_label(app, &wafer),
                )
                .action(
                    trace_detail_action(&TraceSelection::Wafer(wafer.clone())),
                    app.selected_trace_detail == Some(TraceSelection::Wafer(wafer)),
                )
            }),
    );
    if let Some(query) = trace_impact_query(app) {
        let impact = genealogy.impact_for(query.clone());
        rows.push(PrimaryRow::new(
            "trace-impact-summary",
            app.trace_impact_mode.label(),
            query.label(),
            format!(
                "{} direct, {} impacted, {} evidence rows",
                impact.direct_wafers.len(),
                impact.impacted_wafers.len(),
                impact.matching_process_records.len() + impact.matching_material_uses.len()
            ),
        ));
        rows.extend(impact.impacted_wafers.into_iter().take(2).map(|impact| {
            PrimaryRow::new(
                format!(
                    "trace-impact-{}-{}",
                    impact.wafer.lot_id, impact.wafer.wafer_id
                ),
                "Impacted wafer",
                impact.wafer.to_string(),
                format!(
                    "{}; latest {}",
                    impact.relationship.label(),
                    impact
                        .latest_step_id
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "n/a".to_string())
                ),
            )
            .action(
                trace_detail_action(&TraceSelection::Wafer(impact.wafer.clone())),
                app.selected_trace_detail == Some(TraceSelection::Wafer(impact.wafer)),
            )
        }));
    }
    rows.extend(
        genealogy
            .inherited_process_history_for_wafer(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|record| {
                PrimaryRow::new(
                    format!("trace-process-{}", record.sequence),
                    format!("#{} {}", record.sequence, record.step_id),
                    record.tool_id.to_string(),
                    format!(
                        "{}; run {}; {}",
                        record.step_name, record.tool_run_id, record.completed_at
                    ),
                )
                .action(
                    trace_detail_action(&TraceSelection::Process(record.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::Process(record.sequence)),
                )
            }),
    );
    rows.extend(
        genealogy
            .material_ancestry_for_wafer(&selected_wafer)
            .into_iter()
            .rev()
            .take(2)
            .map(|record| {
                PrimaryRow::new(
                    format!("trace-material-{}", record.sequence),
                    format!("#{} material", record.sequence),
                    record.material_lot_id.to_string(),
                    format!(
                        "{} {} at step {}; run {}",
                        spc_compact_number(record.quantity),
                        record.unit,
                        record.step_id,
                        record.tool_run_id
                    ),
                )
                .action(
                    trace_detail_action(&TraceSelection::MaterialUse(record.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::MaterialUse(record.sequence)),
                )
            }),
    );
    rows.extend(
        app.workspace
            .genealogy
            .events
            .iter()
            .rev()
            .take(2)
            .map(|event| {
                let (title, detail) = trace_event_labels(&event.kind);
                PrimaryRow::new(
                    format!("trace-event-{}", event.sequence),
                    format!("#{} {title}", event.sequence),
                    "event",
                    detail,
                )
                .action(
                    trace_detail_action(&TraceSelection::Event(event.sequence)),
                    app.selected_trace_detail == Some(TraceSelection::Event(event.sequence)),
                )
            }),
    );
    rows
}

fn trace_latest_step_label(app: &FabricadApp, wafer: &WaferRef) -> String {
    app.workspace
        .genealogy
        .inherited_process_history_for_wafer(wafer)
        .last()
        .map(|record| format!("latest {} on {}", record.step_id, record.tool_id))
        .unwrap_or_else(|| "no process history".to_string())
}

fn trace_event_labels(
    kind: &layout_model::genealogy::GenealogyEventKind,
) -> (&'static str, String) {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { lot_id } => {
            ("lot started", lot_id.to_string())
        }
        layout_model::genealogy::GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            wafer_count,
            reason,
        } => (
            "lot split",
            format!("{source_lot_id} -> {target_lot_id}; {wafer_count} wafers; {reason}"),
        ),
        layout_model::genealogy::GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            wafer_count,
            reason,
        } => (
            "lot merge",
            format!(
                "{} -> {target_lot_id}; {wafer_count} wafers; {reason}",
                source_lot_ids
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
    }
}

fn trace_selected_detail_rows(app: &FabricadApp) -> Vec<(String, String)> {
    let selection = app.selected_trace_detail.clone().or_else(|| {
        default_trace_selection(
            &app.workspace,
            app.selected_trace_lot.as_ref(),
            app.selected_trace_wafer.as_ref(),
        )
    });
    let Some(selection) = selection else {
        return vec![("Selected node".to_string(), "None".to_string())];
    };
    let genealogy = &app.workspace.genealogy;
    match selection {
        TraceSelection::Lot(lot_id) => {
            let Some(lot) = genealogy.lots.get(&lot_id) else {
                return vec![("Selected node".to_string(), format!("Missing lot {lot_id}"))];
            };
            let child_count = genealogy
                .lots
                .values()
                .filter(|candidate| {
                    candidate
                        .created_from
                        .iter()
                        .any(|parent| parent == &lot_id)
                })
                .count();
            let active_wafers = lot
                .wafers
                .values()
                .filter(|wafer| {
                    matches!(
                        wafer.state,
                        layout_model::genealogy::WaferGenealogyState::Active
                    )
                })
                .count();
            vec![
                ("Selected node".to_string(), format!("Lot {lot_id}")),
                ("Product".to_string(), lot.product.clone()),
                ("Route".to_string(), lot.route_id.to_string()),
                ("Disposition".to_string(), trace_lot_disposition_label(lot)),
                (
                    "Parent / child lots".to_string(),
                    format!("{} / {child_count}", lot.created_from.len()),
                ),
                (
                    "Active / moved".to_string(),
                    format!("{} / {}", active_wafers, lot.wafers.len() - active_wafers),
                ),
            ]
        }
        TraceSelection::Wafer(wafer_ref) => {
            let Some(wafer) = genealogy.wafer(&wafer_ref) else {
                return vec![(
                    "Selected node".to_string(),
                    format!("Missing wafer {wafer_ref}"),
                )];
            };
            let process_records = genealogy.inherited_process_history_for_wafer(&wafer_ref);
            let material_records = genealogy.material_ancestry_for_wafer(&wafer_ref);
            vec![
                ("Selected node".to_string(), format!("Wafer {wafer_ref}")),
                (
                    "Slot / substrate".to_string(),
                    format!("{} / {}", wafer.slot, wafer.substrate),
                ),
                ("State".to_string(), trace_wafer_state_label(wafer)),
                (
                    "Parent".to_string(),
                    wafer
                        .parent
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Lineage / descendants".to_string(),
                    format!(
                        "{} / {}",
                        genealogy.wafer_lineage(&wafer_ref).len(),
                        genealogy.wafer_descendants(&wafer_ref).len()
                    ),
                ),
                (
                    "Process / material".to_string(),
                    format!("{} / {}", process_records.len(), material_records.len()),
                ),
            ]
        }
        TraceSelection::Process(sequence) => {
            let Some(record) = trace_process_record(app, sequence) else {
                return vec![(
                    "Selected node".to_string(),
                    format!("Missing process #{sequence}"),
                )];
            };
            let material_uses = genealogy
                .material_uses
                .iter()
                .filter(|material| {
                    material.tool_run_id == record.tool_run_id && material.wafer == record.wafer
                })
                .count();
            vec![
                ("Selected node".to_string(), format!("Process #{sequence}")),
                ("Wafer".to_string(), record.wafer.to_string()),
                (
                    "Step".to_string(),
                    format!("{} {}", record.step_id, record.step_name),
                ),
                ("Layer".to_string(), trace_process_layer_label(record)),
                (
                    "Recipe / tool".to_string(),
                    format!("{} / {}", record.recipe_id, record.tool_id),
                ),
                (
                    "Run / materials".to_string(),
                    format!("{} / {}", record.tool_run_id, material_uses),
                ),
            ]
        }
        TraceSelection::MaterialUse(sequence) => {
            let Some(record) = trace_material_use(app, sequence) else {
                return vec![(
                    "Selected node".to_string(),
                    format!("Missing material #{sequence}"),
                )];
            };
            let material = genealogy.material_lots.get(&record.material_lot_id);
            vec![
                ("Selected node".to_string(), format!("Material #{sequence}")),
                ("Wafer".to_string(), record.wafer.to_string()),
                (
                    "Material lot".to_string(),
                    record.material_lot_id.to_string(),
                ),
                (
                    "Material".to_string(),
                    material
                        .map(|material| material.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                (
                    "Supplier / cert".to_string(),
                    material
                        .map(|material| {
                            format!("{} / {}", material.supplier, material.certificate_id)
                        })
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
                (
                    "Step / qty".to_string(),
                    format!(
                        "{} / {} {}",
                        record.step_id,
                        spc_compact_number(record.quantity),
                        record.unit
                    ),
                ),
            ]
        }
        TraceSelection::Event(sequence) => {
            let Some(event) = trace_event(app, sequence) else {
                return vec![(
                    "Selected node".to_string(),
                    format!("Missing event #{sequence}"),
                )];
            };
            let (title, detail) = trace_event_labels(&event.kind);
            vec![
                ("Selected node".to_string(), format!("Event #{sequence}")),
                ("Operation".to_string(), title.to_string()),
                ("Scope".to_string(), trace_event_scope(&event.kind)),
                ("Reason".to_string(), trace_event_reason(&event.kind)),
                ("Audit text".to_string(), detail),
            ]
        }
    }
}

fn trace_process_record(
    app: &FabricadApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::WaferProcessRecord> {
    app.workspace
        .genealogy
        .process_history
        .iter()
        .find(|record| record.sequence == sequence)
}

fn trace_material_use(
    app: &FabricadApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::MaterialUse> {
    app.workspace
        .genealogy
        .material_uses
        .iter()
        .find(|record| record.sequence == sequence)
}

fn trace_event(
    app: &FabricadApp,
    sequence: u64,
) -> Option<&layout_model::genealogy::GenealogyEvent> {
    app.workspace
        .genealogy
        .events
        .iter()
        .find(|event| event.sequence == sequence)
}

fn trace_lot_disposition_label(lot: &layout_model::genealogy::GenealogyLot) -> String {
    match &lot.disposition {
        layout_model::genealogy::LotDisposition::Active => "active".to_string(),
        layout_model::genealogy::LotDisposition::Closed { reason } => {
            format!("closed: {reason}")
        }
    }
}

fn trace_wafer_state_label(wafer: &layout_model::genealogy::GenealogyWafer) -> String {
    match &wafer.state {
        layout_model::genealogy::WaferGenealogyState::Active => "active".to_string(),
        layout_model::genealogy::WaferGenealogyState::SplitTo { lot_id } => {
            format!("split to {lot_id}")
        }
        layout_model::genealogy::WaferGenealogyState::MergedTo { lot_id } => {
            format!("merged to {lot_id}")
        }
        layout_model::genealogy::WaferGenealogyState::Scrapped { reason } => {
            format!("scrapped: {reason}")
        }
    }
}

fn trace_process_layer_label(record: &layout_model::genealogy::WaferProcessRecord) -> String {
    record
        .process_layer
        .map(ProcessLayer::as_technology_name)
        .unwrap_or("n/a")
        .to_string()
}

fn trace_event_scope(kind: &layout_model::genealogy::GenealogyEventKind) -> String {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { lot_id } => lot_id.to_string(),
        layout_model::genealogy::GenealogyEventKind::LotSplit {
            source_lot_id,
            target_lot_id,
            ..
        } => format!("{source_lot_id} -> {target_lot_id}"),
        layout_model::genealogy::GenealogyEventKind::LotMerge {
            source_lot_ids,
            target_lot_id,
            ..
        } => format!(
            "{} -> {target_lot_id}",
            source_lot_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn trace_event_reason(kind: &layout_model::genealogy::GenealogyEventKind) -> String {
    match kind {
        layout_model::genealogy::GenealogyEventKind::LotStarted { .. } => "initial lot".to_string(),
        layout_model::genealogy::GenealogyEventKind::LotSplit { reason, .. }
        | layout_model::genealogy::GenealogyEventKind::LotMerge { reason, .. } => reason.clone(),
    }
}

fn process_flow_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let error_nodes = app
        .workspace
        .process_flow
        .findings()
        .into_iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .filter_map(|finding| finding.node_id)
        .collect::<BTreeSet<_>>();
    app.workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .filter(|node| !app.process_flow_errors_only || error_nodes.contains(&node.id))
        .take(10)
        .map(|node| {
            let recipe = node
                .recipe
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "no recipe".to_string());
            PrimaryRow::new(
                format!("process-node-{}", node.id),
                format!("{} {}", node.id, node.name),
                format!("{} {}", node.kind.label(), node.area),
                format!(
                    "{}; {} eligible tools; {}",
                    recipe,
                    node.eligible_tools.len(),
                    node.notes
                ),
            )
            .action(
                format!("fabricad.viewctl.process_flow.node.{}", node.id),
                app.selected_process_node.as_ref() == Some(&node.id),
            )
        })
        .collect()
}

fn process_control_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let mut rows = selected_control_loop(app)
        .map(|loop_definition| {
            app.workspace
                .process_control
                .actions_for_loop(&loop_definition.id)
                .into_iter()
                .take(10)
                .map(|action| {
                    PrimaryRow::new(
                        format!("control-action-{}", action.id),
                        short_control_action_label(&action.id),
                        format!(
                            "{} {:.0}% confidence",
                            action.state.label(),
                            action.confidence * 100.0
                        ),
                        format!(
                            "error {:.3}; {} adjustments; {}",
                            action.error,
                            action.adjustments.len(),
                            action.rationale
                        ),
                    )
                    .action(
                        format!("fabricad.viewctl.process_control.action.{}", action.id),
                        app.selected_control_action.as_ref() == Some(&action.id),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rows.is_empty() {
        rows.extend(
            app.workspace
                .process_control
                .loops
                .iter()
                .take(10)
                .map(|loop_definition| {
                    PrimaryRow::new(
                        format!("control-loop-{}", loop_definition.id),
                        loop_definition.name.clone(),
                        loop_definition.id.to_string(),
                        format!(
                            "{} target {:.3} {}",
                            loop_definition.output.label,
                            loop_definition.output.target,
                            loop_definition.output.unit
                        ),
                    )
                    .action(
                        format!(
                            "fabricad.viewctl.process_control.loop.{}",
                            loop_definition.id
                        ),
                        app.selected_control_loop.as_ref() == Some(&loop_definition.id),
                    )
                }),
        );
    }
    rows
}

fn spc_fdc_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let monitor = spc_fdc_monitor(&app.workspace);
    let mut rows = monitor
        .findings
        .iter()
        .filter(|finding| app.spc_severity_filter.matches(finding.severity))
        .filter(|finding| app.spc_source_filter.matches(finding.source))
        .filter(|finding| {
            app.spc_context_filter.is_empty()
                || finding.title.contains(&app.spc_context_filter)
                || finding.detail.contains(&app.spc_context_filter)
                || finding
                    .tool_id
                    .as_deref()
                    .is_some_and(|tool_id| tool_id.contains(&app.spc_context_filter))
                || finding
                    .lot_id
                    .as_deref()
                    .is_some_and(|lot_id| lot_id.contains(&app.spc_context_filter))
                || finding
                    .recipe_id
                    .as_deref()
                    .is_some_and(|recipe_id| recipe_id.contains(&app.spc_context_filter))
        })
        .take(10)
        .map(|finding| {
            let context = [
                finding.tool_id.as_ref().map(|id| format!("tool {id}")),
                finding.lot_id.as_ref().map(|id| format!("lot {id}")),
                finding.wafer_id.as_ref().map(|id| format!("wafer {id}")),
                finding.recipe_id.as_ref().map(|id| format!("recipe {id}")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
            PrimaryRow::new(
                format!("spc-{}-{}", finding.source.label(), finding.title),
                finding.title.clone(),
                format!("{} {}", finding.severity.label(), finding.source.label()),
                if context.is_empty() {
                    finding.detail.clone()
                } else {
                    format!("{context}; {}", finding.detail)
                },
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.extend(monitor.charts.iter().take(10).map(|chart| {
            PrimaryRow::new(
                format!("spc-chart-{}", chart.id),
                chart.metric.clone(),
                format!("{} violations", chart.violations.len()),
                format!(
                    "center {:.2}; limits {:.2}..{:.2}",
                    chart.limits.center, chart.limits.lower_control, chart.limits.upper_control
                ),
            )
            .action(
                format!("fabricad.viewctl.spc.chart.{}", chart.id),
                app.selected_spc_chart.as_deref() == Some(chart.id.as_str()),
            )
        }));
    }
    rows
}

fn cross_section_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    let mut rows = vec![
        PrimaryRow::new(
            "cross-section-substrate",
            "Step 0 Starting substrate",
            app.workspace.cross_section.substrate_material.as_str(),
            format!(
                "{:.2} um thick; width {:.1} um",
                app.workspace.cross_section.substrate_thickness_um,
                app.workspace.cross_section.width_um
            ),
        )
        .action(
            "fabricad.viewctl.cross_section.step.0",
            app.cross_section_step == 0,
        ),
    ];
    rows.extend(
        app.workspace
            .cross_section
            .steps
            .iter()
            .enumerate()
            .take(9)
            .map(|(index, step)| {
                let step_index = index + 1;
                PrimaryRow::new(
                    format!("cross-section-step-{step_index}"),
                    format!("Step {step_index} {}", step.name),
                    cross_section_step_kind_label(&step.kind),
                    step.detail.clone(),
                )
                .action(
                    format!("fabricad.viewctl.cross_section.step.{step_index}"),
                    app.cross_section_step == step_index,
                )
            }),
    );
    rows
}

fn metrology_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    app.workspace
        .wafer_map
        .measurements
        .iter()
        .filter(|measurement| measurement.kind == app.metrology_kind)
        .filter(|measurement| {
            !app.metrology_failed_only || measurement.status != MeasurementStatus::Pass
        })
        .take(10)
        .map(|measurement| {
            PrimaryRow::new(
                format!("measurement-{}", measurement.id),
                measurement.id.clone(),
                measurement.status.label(),
                format!(
                    "{} {:.2} {}; {} / {}",
                    die_coord_label(Some(measurement.die)),
                    measurement.value,
                    measurement.kind.unit(),
                    measurement.links.lot_id,
                    measurement.links.wafer_id
                ),
            )
            .action(
                format!(
                    "fabricad.viewctl.metrology.die.{}|{}",
                    measurement.die.column, measurement.die.row
                ),
                app.selected_die == Some(measurement.die),
            )
        })
        .collect()
}

fn yield_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    if let Some(lot_id) = app.selected_yield_lot.as_deref() {
        let rows = app
            .workspace
            .yield_analysis
            .wafer_summaries_for_lot(lot_id)
            .into_iter()
            .filter(|summary| yield_summary_matches(app, summary))
            .take(10)
            .map(|summary| {
                let wafer_id = summary
                    .wafer_id
                    .clone()
                    .unwrap_or_else(|| "lot".to_string());
                PrimaryRow::new(
                    format!("yield-wafer-{wafer_id}"),
                    wafer_id.clone(),
                    percent_label(summary.yield_fraction),
                    yield_summary_detail(summary),
                )
                .action(
                    format!("fabricad.viewctl.yield.wafer.{wafer_id}"),
                    app.selected_yield_wafer.as_deref() == Some(wafer_id.as_str()),
                )
            })
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            return rows;
        }
    }

    app.workspace
        .yield_analysis
        .lot_summaries
        .iter()
        .filter(|summary| yield_summary_matches(app, summary))
        .take(10)
        .map(|summary| {
            PrimaryRow::new(
                format!("yield-lot-{}", summary.lot_id),
                summary.lot_id.clone(),
                percent_label(summary.yield_fraction),
                yield_summary_detail(summary),
            )
            .action(
                format!("fabricad.viewctl.yield.lot.{}", summary.lot_id),
                app.selected_yield_lot.as_deref() == Some(summary.lot_id.as_str()),
            )
        })
        .collect()
}

fn experiment_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    app.filtered_experiment_run_rows()
        .into_iter()
        .take(10)
        .map(|row| {
            let factors = app
                .workspace
                .experiment_plan
                .factors
                .iter()
                .take(2)
                .map(|factor| factor.name.as_str())
                .zip(row.factor_values.iter().map(String::as_str))
                .map(|(factor, value)| format!("{factor}={value}"))
                .collect::<Vec<_>>()
                .join(", ");
            PrimaryRow::new(
                format!("experiment-run-{}", row.run_id),
                row.run_id.clone(),
                row.status.clone(),
                format!(
                    "order {} / {} W{:02} / split {} / responses {} / primary {}",
                    row.run_order,
                    row.wafer_id,
                    row.slot,
                    row.block,
                    row.capture_summary,
                    row.primary_value
                ),
            )
            .with_detail_suffix(factors)
            .action(
                format!("fabricad.viewctl.experiment.run.{}", row.run_id),
                row.selected,
            )
        })
        .collect()
}

fn notebook_primary_rows(app: &FabricadApp) -> Vec<PrimaryRow> {
    notebook_filtered_entries(app)
        .into_iter()
        .take(10)
        .map(|entry| {
            let tags = if entry.tags.is_empty() {
                "untagged".to_string()
            } else {
                entry
                    .tags
                    .iter()
                    .map(|tag| format!("#{tag}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            PrimaryRow::new(
                format!("notebook-{}", entry.id),
                entry.title.clone(),
                format!("{} {}", entry.updated_at, entry.author),
                format!("{} links; {tags}", entry.link_count()),
            )
        })
        .collect()
}

fn notebook_tag_summary(entry: &layout_model::notebook::NotebookEntry) -> String {
    if entry.tags.is_empty() {
        "No tags".to_string()
    } else {
        entry
            .tags
            .iter()
            .take(6)
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn notebook_link_summary(entry: &layout_model::notebook::NotebookEntry) -> String {
    let links = &entry.links;
    format!(
        "lots {}, wafers {}, recipes {}, runs {}, metrology {}, images {}",
        links.lots.len(),
        links.wafers.len(),
        links.recipes.len(),
        links.tool_runs.len(),
        links.metrology.len(),
        links.images.len()
    )
}

fn notebook_entry_link_focus_tokens(
    entry: &layout_model::notebook::NotebookEntry,
) -> Vec<(NotebookLinkKind, String)> {
    let mut tokens = Vec::new();
    tokens.extend(
        entry
            .links
            .lots
            .iter()
            .map(|value| (NotebookLinkKind::Lot, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .wafers
            .iter()
            .map(|value| (NotebookLinkKind::Wafer, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .recipes
            .iter()
            .map(|value| (NotebookLinkKind::Recipe, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .tool_runs
            .iter()
            .map(|value| (NotebookLinkKind::ToolRun, value.to_string())),
    );
    tokens.extend(
        entry
            .links
            .metrology
            .iter()
            .map(|value| (NotebookLinkKind::Metrology, value.id.to_string())),
    );
    tokens.extend(
        entry
            .links
            .images
            .iter()
            .map(|value| (NotebookLinkKind::Image, value.id.to_string())),
    );
    tokens
}

fn notebook_entry_has_link(
    entry: &layout_model::notebook::NotebookEntry,
    kind: NotebookLinkKind,
    query: &str,
) -> bool {
    match kind {
        NotebookLinkKind::Lot => entry.links.lots.iter().any(|value| value.as_str() == query),
        NotebookLinkKind::Wafer => entry
            .links
            .wafers
            .iter()
            .any(|value| value.as_str() == query),
        NotebookLinkKind::Recipe => entry
            .links
            .recipes
            .iter()
            .any(|value| value.as_str() == query),
        NotebookLinkKind::ToolRun => entry
            .links
            .tool_runs
            .iter()
            .any(|value| value.to_string() == query),
        NotebookLinkKind::Metrology => entry
            .links
            .metrology
            .iter()
            .any(|value| value.id.as_str() == query),
        NotebookLinkKind::Image => entry
            .links
            .images
            .iter()
            .any(|value| value.id.as_str() == query),
    }
}

fn add_notebook_tag_once(entry: &mut layout_model::notebook::NotebookEntry, tag: &str) {
    if !entry.tags.iter().any(|candidate| candidate == tag) {
        entry.tags.push(tag.to_string());
        entry.tags.sort();
    }
}

fn append_notebook_section_once(body: &mut String, heading: &str, content: &str) {
    if body.contains(heading) {
        return;
    }
    if !body.ends_with('\n') {
        body.push('\n');
    }
    if !body.ends_with("\n\n") {
        body.push('\n');
    }
    body.push_str(heading);
    body.push('\n');
    body.push_str(content);
}

fn notebook_related_entries(
    app: &FabricadApp,
    entry: &layout_model::notebook::NotebookEntry,
) -> Vec<(String, String)> {
    let entry_tags = entry.tags.iter().cloned().collect::<BTreeSet<_>>();
    let entry_links = entry
        .links
        .search_labels()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut related = app
        .workspace
        .lab_notebook
        .entries
        .iter()
        .filter(|candidate| candidate.id != entry.id)
        .filter_map(|candidate| {
            let shared_tags = candidate
                .tags
                .iter()
                .filter(|tag| entry_tags.contains(*tag))
                .cloned()
                .collect::<Vec<_>>();
            let candidate_links = candidate
                .links
                .search_labels()
                .into_iter()
                .collect::<BTreeSet<_>>();
            let shared_links = candidate_links
                .intersection(&entry_links)
                .cloned()
                .collect::<Vec<_>>();
            let score = shared_tags.len() + shared_links.len();
            if score == 0 {
                return None;
            }
            let reason = if !shared_tags.is_empty() && !shared_links.is_empty() {
                format!(
                    "shared tags {}; shared links {}",
                    shared_tags.join(", "),
                    shared_links.join(", ")
                )
            } else if !shared_tags.is_empty() {
                format!("shared tags {}", shared_tags.join(", "))
            } else {
                format!("shared links {}", shared_links.join(", "))
            };
            Some((
                score,
                candidate.updated_at.clone(),
                candidate.title.clone(),
                reason,
            ))
        })
        .collect::<Vec<_>>();
    related.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    related
        .into_iter()
        .map(|(_, _, title, reason)| (title, reason))
        .collect()
}

fn environment_sensor_status_for(
    workspace: &WorkspaceDataset,
    sensor: &layout_model::environment::EnvironmentSensor,
) -> String {
    workspace
        .environment
        .latest_reading(&sensor.id)
        .map(|reading| {
            let severity = sensor.thresholds.evaluate(reading.value);
            let state = severity.map_or("Nominal", |severity| severity.label());
            format!("{:.2} {} {state}", reading.value, sensor.unit)
        })
        .unwrap_or_else(|| "No samples".to_string())
}

fn cross_section_step_kind_label(kind: &layout_model::cross_section::ProcessStepKind) -> String {
    match kind {
        layout_model::cross_section::ProcessStepKind::Deposit {
            material,
            thickness_um,
        } => format!("deposit {} {:.2} um", material.as_str(), thickness_um),
        layout_model::cross_section::ProcessStepKind::Etch { material, depth_um } => {
            format!("etch {} {:.2} um", material.as_str(), depth_um)
        }
        layout_model::cross_section::ProcessStepKind::Pattern { openings } => {
            format!("pattern {} openings", openings.len())
        }
    }
}

fn yield_summary_matches(
    app: &FabricadApp,
    summary: &layout_model::yield_analysis::YieldSummary,
) -> bool {
    let filter_matches = match app.yield_map_filter {
        YieldMapFilter::All => true,
        YieldMapFilter::Failing => summary.failing_dies > 0,
        YieldMapFilter::Passing => summary.failing_dies == 0,
    };
    filter_matches
        && (!app.show_only_attention_wafers || summary.yield_fraction < 0.98)
        && (!app.show_only_excursions || !summary.root_cause_hints.is_empty())
}

fn yield_summary_detail(summary: &layout_model::yield_analysis::YieldSummary) -> String {
    let failure = summary
        .dominant_failure
        .map(|failure| failure.label())
        .unwrap_or("no dominant failure");
    let root_cause = summary
        .root_cause_hints
        .first()
        .map(String::as_str)
        .unwrap_or("no root-cause hint");
    format!(
        "{} failing dies; {}; {}; {}",
        summary.failing_dies,
        failure,
        summary.spatial_pattern.label(),
        root_cause
    )
}

fn percent_label(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

#[derive(Clone, Debug)]
struct ProcessFlowNodePlacement {
    id: ProcessFlowNodeId,
    center: UiPoint,
    rect: UiRect,
}

fn process_flow_timeline_primitives(
    app: &FabricadApp,
    findings: &[layout_model::process_flow::ProcessFlowFinding],
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let nodes = process_flow_visible_nodes(app, findings);
    let mut primitives =
        Vec::with_capacity(nodes.len() * 4 + app.workspace.process_flow.route.edges.len());
    let track = UiRect::new(
        ui_scale.value(42.0),
        ui_scale.value(26.0),
        ui_scale.value(860.0),
        ui_scale.value(178.0),
    );
    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(track, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(70, 84, 98, 255),
            ui_scale.value(1.0),
        )),
    ));
    if nodes.is_empty() {
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(track.x + ui_scale.value(24.0), track.y + track.height * 0.5),
            to: UiPoint::new(
                track.right() - ui_scale.value(24.0),
                track.y + track.height * 0.5,
            ),
            stroke: StrokeStyle::new(ColorRgba::new(80, 94, 108, 255), ui_scale.value(1.0)),
        });
        return primitives;
    }

    let placements = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let fraction = if nodes.len() == 1 {
                0.5
            } else {
                index as f32 / (nodes.len() - 1) as f32
            };
            let center_x =
                track.x + ui_scale.value(48.0) + (track.width - ui_scale.value(96.0)) * fraction;
            let center_y = match node.kind {
                ProcessFlowNodeKind::Measurement => track.y + ui_scale.value(58.0),
                ProcessFlowNodeKind::Hold => track.y + ui_scale.value(132.0),
                ProcessFlowNodeKind::Start
                | ProcessFlowNodeKind::End
                | ProcessFlowNodeKind::Operation => track.y + ui_scale.value(96.0),
            };
            let rect_width = (track.width / nodes.len() as f32 * 0.62)
                .clamp(ui_scale.value(42.0), ui_scale.value(88.0));
            let rect_height = ui_scale.value(42.0);
            ProcessFlowNodePlacement {
                id: node.id.clone(),
                center: UiPoint::new(center_x, center_y),
                rect: UiRect::new(
                    center_x - rect_width * 0.5,
                    center_y - rect_height * 0.5,
                    rect_width,
                    rect_height,
                ),
            }
        })
        .collect::<Vec<_>>();
    let placement_by_id = placements
        .iter()
        .map(|placement| (placement.id.clone(), placement))
        .collect::<BTreeMap<_, _>>();

    for edge in &app.workspace.process_flow.route.edges {
        let (Some(from), Some(to)) = (
            placement_by_id.get(&edge.from),
            placement_by_id.get(&edge.to),
        ) else {
            continue;
        };
        add_process_flow_edge_primitives(
            edge.kind,
            from.center,
            to.center,
            ui_scale,
            &mut primitives,
        );
    }

    for (node, placement) in nodes.iter().zip(placements.iter()) {
        let selected = app.selected_process_node.as_ref() == Some(&node.id);
        let issue_count = process_flow_node_findings(&node.id, findings).len();
        let rework_count = app
            .workspace
            .process_flow
            .route
            .edges
            .iter()
            .filter(|edge| edge.kind == layout_model::process_flow::ProcessFlowEdgeKind::Rework)
            .filter(|edge| edge.from == node.id || edge.to == node.id)
            .count();
        let stroke_color = if selected {
            ColorRgba::new(250, 219, 112, 255)
        } else if issue_count > 0 {
            ColorRgba::new(236, 91, 88, 255)
        } else {
            ColorRgba::new(24, 29, 34, 180)
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(placement.rect, process_flow_node_color(node.kind, 220))
                .stroke(StrokeStyle::new(
                    stroke_color,
                    ui_scale.value(if selected { 2.2 } else { 1.0 }),
                )),
        ));
        let mut badge_x = placement.rect.right() - ui_scale.value(8.0);
        if node.measurement_checkpoint.is_some() {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                ColorRgba::new(78, 188, 135, 245),
                ui_scale,
            ));
            badge_x -= ui_scale.value(12.0);
        }
        if node.hold_point || node.kind == ProcessFlowNodeKind::Hold {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                ColorRgba::new(238, 181, 82, 245),
                ui_scale,
            ));
            badge_x -= ui_scale.value(12.0);
        }
        if rework_count > 0 || issue_count > 0 {
            primitives.push(process_flow_badge(
                badge_x,
                placement.rect.y + ui_scale.value(8.0),
                if issue_count > 0 {
                    ColorRgba::new(236, 91, 88, 245)
                } else {
                    ColorRgba::new(206, 111, 220, 245)
                },
                ui_scale,
            ));
        }
    }

    primitives
}

fn process_flow_visible_nodes<'a>(
    app: &'a FabricadApp,
    findings: &[layout_model::process_flow::ProcessFlowFinding],
) -> Vec<&'a layout_model::process_flow::ProcessFlowNode> {
    let error_nodes = findings
        .iter()
        .filter(|finding| {
            finding.severity == layout_model::process_flow::ProcessFlowFindingSeverity::Error
        })
        .filter_map(|finding| finding.node_id.clone())
        .collect::<BTreeSet<_>>();
    app.workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .filter(|node| !app.process_flow_errors_only || error_nodes.contains(&node.id))
        .take(14)
        .collect()
}

fn add_process_flow_edge_primitives(
    kind: layout_model::process_flow::ProcessFlowEdgeKind,
    from: UiPoint,
    to: UiPoint,
    ui_scale: UiScale,
    primitives: &mut Vec<ScenePrimitive>,
) {
    let color = process_flow_edge_color(kind);
    let stroke = StrokeStyle::new(color, ui_scale.value(1.5));
    match kind {
        layout_model::process_flow::ProcessFlowEdgeKind::Rework => {
            let y = from.y.min(to.y) - ui_scale.value(34.0);
            primitives.push(ScenePrimitive::Line {
                from,
                to: UiPoint::new(from.x, y),
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(from.x, y),
                to: UiPoint::new(to.x, y),
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(to.x, y),
                to,
                stroke,
            });
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Branch => {
            let mid = UiPoint::new(
                (from.x + to.x) * 0.5,
                (from.y + to.y) * 0.5 + ui_scale.value(20.0),
            );
            primitives.push(ScenePrimitive::Line {
                from,
                to: mid,
                stroke,
            });
            primitives.push(ScenePrimitive::Line {
                from: mid,
                to,
                stroke,
            });
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Sequence => {
            primitives.push(ScenePrimitive::Line { from, to, stroke });
        }
    }
}

fn process_flow_badge(x: f32, y: f32, fill: ColorRgba, ui_scale: UiScale) -> ScenePrimitive {
    ScenePrimitive::Circle {
        center: UiPoint::new(x, y),
        radius: ui_scale.value(4.4),
        fill,
        stroke: Some(StrokeStyle::new(
            ColorRgba::new(16, 20, 24, 185),
            ui_scale.value(1.0),
        )),
    }
}

fn process_flow_node_findings<'a>(
    node_id: &ProcessFlowNodeId,
    findings: &'a [layout_model::process_flow::ProcessFlowFinding],
) -> Vec<&'a layout_model::process_flow::ProcessFlowFinding> {
    findings
        .iter()
        .filter(|finding| finding.node_id.as_ref() == Some(node_id))
        .collect()
}

fn process_flow_node_color(kind: ProcessFlowNodeKind, alpha: u8) -> ColorRgba {
    match kind {
        ProcessFlowNodeKind::Start | ProcessFlowNodeKind::End => {
            ColorRgba::new(95, 108, 120, alpha)
        }
        ProcessFlowNodeKind::Operation => ColorRgba::new(62, 132, 196, alpha),
        ProcessFlowNodeKind::Measurement => ColorRgba::new(54, 162, 122, alpha),
        ProcessFlowNodeKind::Hold => ColorRgba::new(208, 153, 69, alpha),
    }
}

fn process_flow_edge_color(kind: layout_model::process_flow::ProcessFlowEdgeKind) -> ColorRgba {
    match kind {
        layout_model::process_flow::ProcessFlowEdgeKind::Sequence => {
            ColorRgba::new(96, 160, 220, 210)
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Branch => {
            ColorRgba::new(232, 176, 78, 220)
        }
        layout_model::process_flow::ProcessFlowEdgeKind::Rework => {
            ColorRgba::new(220, 105, 198, 220)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CrossSectionPreviewOptions {
    show_mask_overlay: bool,
    show_dimension_guides: bool,
    show_risk_cues: bool,
}

#[derive(Clone, Copy, Debug)]
struct CrossSectionSurfaceSummary {
    min_um: f32,
    max_um: f32,
    average_um: f32,
    range_um: f32,
}

fn cross_section_preview_primitives(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
    selected_material: Option<&MaterialId>,
    options: CrossSectionPreviewOptions,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let bounds = UiRect::new(
        ui_scale.value(54.0),
        ui_scale.value(40.0),
        ui_scale.value(850.0),
        ui_scale.value(236.0),
    );
    let surface = cross_section_surface_summary(process, snapshot);
    let max_y = surface.max_um.max(process.substrate_thickness_um).max(1.0);
    let mut primitives = Vec::with_capacity(snapshot.segments.len() + 32);

    primitives.push(ScenePrimitive::Rect(
        operad::PaintRect::solid(bounds, ColorRgba::new(14, 19, 24, 255)).stroke(StrokeStyle::new(
            ColorRgba::new(72, 86, 100, 255),
            ui_scale.value(1.0),
        )),
    ));

    if options.show_dimension_guides {
        for fraction in [0.25_f32, 0.5, 0.75] {
            let x = bounds.x + bounds.width * fraction;
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(x, bounds.y),
                to: UiPoint::new(x, bounds.bottom()),
                stroke: StrokeStyle::new(ColorRgba::new(160, 176, 192, 58), ui_scale.value(1.0)),
            });
            let y = bounds.bottom() - bounds.height * fraction;
            primitives.push(ScenePrimitive::Line {
                from: UiPoint::new(bounds.x, y),
                to: UiPoint::new(bounds.right(), y),
                stroke: StrokeStyle::new(ColorRgba::new(160, 176, 192, 58), ui_scale.value(1.0)),
            });
        }
    }

    if options.show_mask_overlay && !snapshot.active_mask.is_empty() {
        let mask_top = ui_scale.value(18.0);
        for opening in &snapshot.active_mask {
            let x0 = cross_section_x_to_scene(bounds, process.width_um, opening.start_um);
            let x1 = cross_section_x_to_scene(bounds, process.width_um, opening.end_um);
            primitives.push(ScenePrimitive::Rect(operad::PaintRect::solid(
                UiRect::new(x0, mask_top, (x1 - x0).max(1.0), ui_scale.value(11.0)),
                ColorRgba::new(248, 208, 78, 150),
            )));
        }
    }

    for segment in &snapshot.segments {
        let selected = selected_material == Some(&segment.material);
        let fill = cross_section_material_color(process, &segment.material, 224);
        let stroke = if selected {
            StrokeStyle::new(ColorRgba::new(252, 253, 255, 255), ui_scale.value(2.0))
        } else {
            StrokeStyle::new(ColorRgba::new(16, 20, 24, 145), ui_scale.value(0.8))
        };
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                cross_section_segment_scene_rect(process, segment, bounds, max_y),
                fill,
            )
            .stroke(stroke),
        ));
    }

    if options.show_risk_cues {
        add_cross_section_risk_primitives(
            process,
            snapshot,
            bounds,
            max_y,
            ui_scale,
            &mut primitives,
        );
    }

    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, bounds.bottom()),
        to: UiPoint::new(bounds.right(), bounds.bottom()),
        stroke: StrokeStyle::new(ColorRgba::new(188, 198, 208, 255), ui_scale.value(1.0)),
    });
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(bounds.x, bounds.bottom()),
        to: UiPoint::new(bounds.x, bounds.y),
        stroke: StrokeStyle::new(ColorRgba::new(188, 198, 208, 255), ui_scale.value(1.0)),
    });

    let swatch_y = ui_scale.value(292.0);
    let mut swatch_x = bounds.x;
    for material in process.materials.iter().take(8) {
        primitives.push(ScenePrimitive::Rect(
            operad::PaintRect::solid(
                UiRect::new(
                    swatch_x,
                    swatch_y,
                    ui_scale.value(22.0),
                    ui_scale.value(12.0),
                ),
                ColorRgba::new(
                    material.color_rgb[0],
                    material.color_rgb[1],
                    material.color_rgb[2],
                    240,
                ),
            )
            .stroke(StrokeStyle::new(
                ColorRgba::new(18, 22, 26, 190),
                ui_scale.value(1.0),
            )),
        ));
        swatch_x += ui_scale.value(34.0);
    }

    primitives
}

fn add_cross_section_risk_primitives(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
    bounds: UiRect,
    max_y: f32,
    ui_scale: UiScale,
    primitives: &mut Vec<ScenePrimitive>,
) {
    let heights = cross_section_surface_heights(process, snapshot);
    if heights.len() < 2 {
        return;
    }
    let column_width = process.width_um.max(0.1) / heights.len() as f32;
    let mut last_marker_x = f32::NEG_INFINITY;
    for index in 1..heights.len() {
        let delta = (heights[index] - heights[index - 1]).abs();
        if delta < 0.08 {
            continue;
        }
        let x_um = index as f32 * column_width;
        let x = cross_section_x_to_scene(bounds, process.width_um, x_um);
        if (x - last_marker_x).abs() < ui_scale.value(12.0) {
            continue;
        }
        last_marker_x = x;
        let y = cross_section_y_to_scene(bounds, heights[index].max(heights[index - 1]), max_y);
        let color = if delta > 0.25 {
            ColorRgba::new(236, 91, 88, 240)
        } else {
            ColorRgba::new(238, 181, 82, 235)
        };
        primitives.push(ScenePrimitive::Line {
            from: UiPoint::new(x, bounds.y),
            to: UiPoint::new(x, bounds.bottom()),
            stroke: StrokeStyle::new(color, ui_scale.value(1.2)),
        });
        primitives.push(ScenePrimitive::Polygon {
            points: vec![
                UiPoint::new(x, y - ui_scale.value(10.0)),
                UiPoint::new(x - ui_scale.value(5.0), y - ui_scale.value(1.0)),
                UiPoint::new(x + ui_scale.value(5.0), y - ui_scale.value(1.0)),
            ],
            fill: color,
            stroke: Some(StrokeStyle::new(
                ColorRgba::new(16, 20, 24, 180),
                ui_scale.value(1.0),
            )),
        });
    }
}

fn cross_section_segment_scene_rect(
    process: &layout_model::cross_section::CrossSectionProcess,
    segment: &layout_model::cross_section::CrossSectionSegment,
    bounds: UiRect,
    max_y: f32,
) -> UiRect {
    let x0 = cross_section_x_to_scene(bounds, process.width_um, segment.x0_um);
    let x1 = cross_section_x_to_scene(bounds, process.width_um, segment.x1_um);
    let y0 = cross_section_y_to_scene(bounds, segment.y0_um, max_y);
    let y1 = cross_section_y_to_scene(bounds, segment.y1_um, max_y);
    UiRect::new(x0, y1, (x1 - x0).max(1.0), (y0 - y1).max(1.0))
}

fn cross_section_x_to_scene(bounds: UiRect, width_um: f32, value_um: f32) -> f32 {
    bounds.x + bounds.width * value_um / width_um.max(0.1)
}

fn cross_section_y_to_scene(bounds: UiRect, value_um: f32, max_y: f32) -> f32 {
    bounds.bottom() - bounds.height * value_um / max_y.max(0.1)
}

fn cross_section_material_color(
    process: &layout_model::cross_section::CrossSectionProcess,
    material_id: &MaterialId,
    alpha: u8,
) -> ColorRgba {
    process
        .material(material_id)
        .map(|material| {
            ColorRgba::new(
                material.color_rgb[0],
                material.color_rgb[1],
                material.color_rgb[2],
                alpha,
            )
        })
        .unwrap_or_else(|| ColorRgba::new(174, 184, 194, alpha))
}

fn cross_section_surface_summary(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> CrossSectionSurfaceSummary {
    let heights = cross_section_surface_heights(process, snapshot);
    if heights.is_empty() {
        return CrossSectionSurfaceSummary {
            min_um: 0.0,
            max_um: process.substrate_thickness_um,
            average_um: process.substrate_thickness_um,
            range_um: 0.0,
        };
    }
    let min_um = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let max_um = heights.iter().copied().fold(0.0, f32::max);
    let average_um = heights.iter().sum::<f32>() / heights.len() as f32;
    CrossSectionSurfaceSummary {
        min_um,
        max_um,
        average_um,
        range_um: max_um - min_um,
    }
}

fn cross_section_surface_heights(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Vec<f32> {
    let columns = process.columns.max(1);
    let column_width = process.width_um.max(0.1) / columns as f32;
    (0..columns)
        .map(|index| {
            let x_um = (index as f32 + 0.5) * column_width;
            snapshot
                .segments
                .iter()
                .filter(|segment| x_um >= segment.x0_um && x_um <= segment.x1_um)
                .map(|segment| segment.y1_um)
                .fold(0.0, f32::max)
        })
        .collect()
}

fn cross_section_mask_coverage_um(
    process: &layout_model::cross_section::CrossSectionProcess,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> f32 {
    snapshot
        .active_mask
        .iter()
        .map(|opening| {
            let start = opening.start_um.clamp(0.0, process.width_um);
            let end = opening.end_um.clamp(0.0, process.width_um);
            (end - start).max(0.0)
        })
        .sum::<f32>()
        .min(process.width_um.max(0.0))
}

fn selected_cross_section_material_for_snapshot(
    app: &FabricadApp,
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Option<MaterialId> {
    if let Some(material) = &app.selected_cross_section_material
        && (app.workspace.cross_section.material(material).is_some()
            || snapshot
                .segments
                .iter()
                .any(|segment| segment.material == *material))
    {
        return Some(material.clone());
    }
    cross_section_step_material_id(&app.workspace.cross_section, snapshot.step_index)
        .or_else(|| cross_section_dominant_material(snapshot))
}

fn cross_section_step_material_id(
    process: &layout_model::cross_section::CrossSectionProcess,
    step_index: usize,
) -> Option<MaterialId> {
    if step_index == 0 {
        return Some(process.substrate_material.clone());
    }
    process
        .steps
        .get(step_index.saturating_sub(1))
        .and_then(|step| match &step.kind {
            layout_model::cross_section::ProcessStepKind::Deposit { material, .. }
            | layout_model::cross_section::ProcessStepKind::Etch { material, .. } => {
                Some(material.clone())
            }
            layout_model::cross_section::ProcessStepKind::Pattern { .. } => None,
        })
}

fn cross_section_dominant_material(
    snapshot: &layout_model::cross_section::CrossSectionSnapshot,
) -> Option<MaterialId> {
    let mut best_material = None;
    let mut best_area = 0.0;
    for segment in &snapshot.segments {
        let area =
            (segment.x1_um - segment.x0_um).max(0.0) * (segment.y1_um - segment.y0_um).max(0.0);
        if area > best_area {
            best_area = area;
            best_material = Some(segment.material.clone());
        }
    }
    best_material
}

fn cross_section_step_kind_label_for_index(
    process: &layout_model::cross_section::CrossSectionProcess,
    step_index: usize,
) -> String {
    if step_index == 0 {
        "substrate".to_string()
    } else {
        process
            .steps
            .get(step_index.saturating_sub(1))
            .map(|step| cross_section_step_kind_label(&step.kind))
            .unwrap_or_else(|| "unknown step".to_string())
    }
}

fn format_um_f32(value: f32) -> String {
    format!("{value:.2} um")
}

fn layout_editor_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let document = &app.workspace.document;
    let mut rows = vec![
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.layout.view.layout2d",
                "2D",
                app.active_view == StartupView::Layout2d,
            ),
            ViewControlButton::new(
                "fabricad.viewctl.layout.view.layout3d",
                "3D",
                app.active_view == StartupView::Layout3d,
            ),
        ],
        vec![
            ViewControlButton::new("fabricad.viewctl.layout.toggle_grid", "Grid", app.show_grid),
            ViewControlButton::new(
                "fabricad.viewctl.layout.toggle_snap",
                "Snap",
                app.snap_enabled,
            ),
        ],
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.layout.toggle_drc",
                "DRC overlay",
                app.show_drc_overlay,
            ),
            ViewControlButton::new("fabricad.viewctl.layout.next_shape", "Next shape", false),
        ],
    ];

    let mut layers = document.layers.values().collect::<Vec<_>>();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.layout.add_layer",
        "Add layer",
        false,
    )]);
    for chunk in layers.iter().take(8).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|layer| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout.layer.{}", layer.id.0),
                        compact_layer_label(layer.id, &layer.name),
                        app.active_layer == layer.id,
                    )
                })
                .collect(),
        );
    }
    for chunk in layers.iter().take(8).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|layer| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout.toggle_layer.{}", layer.id.0),
                        format!(
                            "{} {}",
                            if layer.visible { "Hide" } else { "Show" },
                            layer.id.0
                        ),
                        layer.visible,
                    )
                })
                .collect(),
        );
    }

    let mut shape_rows = layout_shape_ids(document)
        .into_iter()
        .filter_map(|shape_id| document.shapes.get(&shape_id))
        .filter(|shape| shape.layer == app.active_layer)
        .take(6)
        .collect::<Vec<_>>();
    if shape_rows.is_empty() {
        shape_rows = layout_shape_ids(document)
            .into_iter()
            .filter_map(|shape_id| document.shapes.get(&shape_id))
            .take(6)
            .collect();
    }
    for chunk in shape_rows.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|shape| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout.shape.{}", shape.id.0),
                        format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)),
                        app.selected_layout_shape == Some(shape.id),
                    )
                })
                .collect(),
        );
    }

    rows.push(vec![
        ViewControlButton::new("fabricad.viewctl.layout.copy", "Copy", false),
        ViewControlButton::new(
            "fabricad.viewctl.layout.paste",
            "Paste",
            !app.layout_clipboard_shapes.is_empty(),
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new("fabricad.viewctl.layout.duplicate", "Duplicate", false),
        ViewControlButton::new(
            "fabricad.viewctl.layout.delete",
            "Delete",
            app.selected_layout_shape.is_some(),
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.layout.clear_selection",
            "Clear",
            app.selected_layout_shape.is_some(),
        ),
        ViewControlButton::new("fabricad.viewctl.layout.run_drc", "Run DRC", false),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.layout.connectivity",
        "Connectivity",
        false,
    )]);
    rows.retain(|row| !row.is_empty());
    rows
}

fn workflow_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = vec![
        vec![
            ViewControlButton::new("fabricad.viewctl.workflow.open.layout2d", "Layout", false),
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.process-flow",
                "Process",
                false,
            ),
        ],
        vec![
            ViewControlButton::new("fabricad.viewctl.workflow.open.fab-control", "Tools", false),
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.scheduler",
                "Dispatch",
                false,
            ),
        ],
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.inventory",
                "Inventory",
                false,
            ),
            ViewControlButton::new("fabricad.viewctl.workflow.open.yield", "Yield", false),
        ],
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.maintenance",
                "Maintenance",
                false,
            ),
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.environment",
                "Cleanroom",
                false,
            ),
        ],
        vec![
            ViewControlButton::new("fabricad.viewctl.workflow.open.safety", "Safety", false),
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.metrology",
                "Metrology",
                false,
            ),
        ],
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.workflow.open.traceability",
                "Trace",
                false,
            ),
            ViewControlButton::new("fabricad.viewctl.workflow.open.notebook", "Notebook", false),
        ],
        vec![ViewControlButton::new(
            "fabricad.viewctl.workflow.load_demo",
            "Load demo",
            false,
        )],
    ];
    let lot_ids = workflow_lot_ids(&app.workspace);
    for chunk in lot_ids.iter().take(4).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.workflow.focus_lot.{lot_id}"),
                        lot_id.to_string(),
                        app.workflow_focus_lot.as_deref() == Some(lot_id.as_str()),
                    )
                })
                .collect(),
        );
    }
    rows
}

fn mask_prep_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let report = mask_check_report(app);
    let page_count = mask_issue_page_count(app, &report);
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in app
        .workspace
        .mes
        .lots
        .keys()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.mask.lot.{lot_id}"),
                        lot_id.to_string(),
                        app.mask_source_lot.as_ref() == Some(lot_id),
                    )
                })
                .collect(),
        );
    }
    rows.push(
        MaskIssueSeverityFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.mask.severity.{}", filter.slug()),
                    filter.label(),
                    app.mask_issue_severity_filter == *filter,
                )
            })
            .collect(),
    );
    rows.push(
        MaskIssueGrouping::ALL
            .iter()
            .map(|grouping| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.mask.group.{}", grouping.slug()),
                    grouping.label(),
                    app.mask_issue_grouping == *grouping,
                )
            })
            .collect(),
    );
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.mask.prev_page",
            "Prev issues",
            app.mask_issue_page > 0,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.mask.next_page",
            format!(
                "Next {}/{}",
                app.mask_issue_page.min(page_count - 1) + 1,
                page_count
            ),
            app.mask_issue_page + 1 < page_count,
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new("fabricad.viewctl.mask.rebuild", "Rebuild", false),
        ViewControlButton::new("fabricad.viewctl.mask.clear_filters", "Clear", false),
    ]);
    rows.retain(|row| !row.is_empty());
    rows
}

fn layout_diff_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let report = layout_diff_report(app);
    let page_count = layout_diff_page_count(app, &report);
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in LayoutDiffSource::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|source| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout_diff.baseline.{}", source.slug()),
                        format!("Base {}", source.label()),
                        app.layout_diff_baseline == *source,
                    )
                })
                .collect(),
        );
    }
    for chunk in LayoutDiffSource::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|source| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout_diff.candidate.{}", source.slug()),
                        format!("Cand {}", source.label()),
                        app.layout_diff_candidate == *source,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new("fabricad.viewctl.layout_diff.swap", "Swap", false),
        ViewControlButton::new(
            "fabricad.viewctl.layout_diff.toggle_changed_only",
            "Changed",
            app.layout_diff_changed_only,
        ),
    ]);
    for chunk in LayoutChangeFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout_diff.filter.{}", filter.slug()),
                        filter.label(),
                        app.layout_diff_change_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    for chunk in LAYOUT_DIFF_PAGE_SIZE_OPTIONS.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|page_size| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout_diff.page_size.{page_size}"),
                        format!("{page_size} rows"),
                        normalized_layout_diff_page_size(app) == *page_size,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.layout_diff.prev_page",
            "Prev",
            app.layout_diff_change_page > 0,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.layout_diff.next_page",
            format!(
                "Next {}/{}",
                app.layout_diff_change_page.min(page_count - 1) + 1,
                page_count
            ),
            app.layout_diff_change_page + 1 < page_count,
        ),
    ]);
    for chunk in LayoutReviewDisposition::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|state| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.layout_diff.review.{}", state.slug()),
                        state.label(),
                        app.layout_diff_review_state == *state,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.layout_diff.demo_current",
            "Demo -> Cur",
            false,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.layout_diff.empty_current",
            "Empty -> Cur",
            false,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.layout_diff.reset_filters",
        "Reset filters",
        false,
    )]);
    rows.retain(|row| !row.is_empty());
    rows
}

fn fab_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in app
        .workspace
        .equipment
        .tools()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.fab.select.{}", tool.id),
                        tool.id.to_string(),
                        app.selected_equipment_tool.as_ref() == Some(&tool.id),
                    )
                })
                .collect(),
        );
    }
    if let Some(tool) = selected_equipment_tool(app) {
        for chunk in tool
            .available_recipes
            .values()
            .take(4)
            .collect::<Vec<_>>()
            .chunks(2)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|recipe| {
                        let selected = app.equipment_recipe_drafts.get(&tool.id).or_else(|| {
                            tool.selected_recipe
                                .as_ref()
                                .map(|selection| &selection.recipe_id)
                        }) == Some(&recipe.id);
                        ViewControlButton::new(
                            format!("fabricad.viewctl.fab.recipe.{}|{}", tool.id, recipe.id),
                            recipe.id.to_string(),
                            selected,
                        )
                    })
                    .collect(),
            );
        }
        rows.push(vec![
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.online|{}", tool.id),
                "Online",
                tool.state == EquipmentToolState::OnlineIdle,
            ),
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.load|{}", tool.id),
                "Load",
                tool.state == EquipmentToolState::RecipeLoaded,
            ),
        ]);
        rows.push(vec![
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.start|{}", tool.id),
                "Start",
                tool.state == EquipmentToolState::Running,
            ),
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.stop|{}", tool.id),
                "Stop",
                false,
            ),
        ]);
        rows.push(vec![
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.alarm|{}", tool.id),
                "Alarm",
                equipment_active_alarm_count(tool) > 0,
            ),
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.clear|{}", tool.id),
                "Clear",
                false,
            ),
        ]);
        rows.push(vec![
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.reset|{}", tool.id),
                "Reset",
                false,
            ),
            ViewControlButton::new(
                format!("fabricad.viewctl.fab.command.maintenance|{}", tool.id),
                "Maint",
                tool.state == EquipmentToolState::Maintenance,
            ),
        ]);
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn maintenance_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in MaintenanceWorkFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.maintenance.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            maintenance_filtered_work_count(&app.workspace, *filter)
                        ),
                        app.maintenance_work_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    rows.push(
        MaintenanceHistoryFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.maintenance.history.{}", filter.slug()),
                    filter.label(),
                    app.maintenance_history_filter == *filter,
                )
            })
            .collect(),
    );
    for chunk in app
        .workspace
        .maintenance
        .tools
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.maintenance.tool.{}", tool.tool_id),
                        tool.tool_id.to_string(),
                        app.selected_maintenance_tool.as_ref() == Some(&tool.tool_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(tool) = selected_maintenance_tool(app) {
        rows.push(vec![
            ViewControlButton::new(
                format!(
                    "fabricad.viewctl.maintenance.status.schedule|{}",
                    tool.tool_id
                ),
                "Schedule",
                false,
            ),
            ViewControlButton::new(
                format!(
                    "fabricad.viewctl.maintenance.status.calibration|{}",
                    tool.tool_id
                ),
                "Calibrate",
                false,
            ),
        ]);
        rows.push(vec![ViewControlButton::new(
            format!(
                "fabricad.viewctl.maintenance.status.release|{}",
                tool.tool_id
            ),
            "Release",
            !app.workspace
                .maintenance
                .release_for_tool(&tool.tool_id, maintenance_today(&app.workspace))
                .state
                .released_to_production(),
        )]);
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn environment_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in app
        .workspace
        .environment
        .sensors
        .iter()
        .take(8)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|sensor| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.environment.sensor.{}", sensor.id),
                        sensor.id.clone(),
                        app.selected_environment_sensor.as_deref() == Some(sensor.id.as_str()),
                    )
                })
                .collect(),
        );
    }
    let mut zones = app
        .workspace
        .environment
        .sensors
        .iter()
        .map(|sensor| sensor.zone.clone())
        .collect::<Vec<_>>();
    zones.sort();
    zones.dedup();
    for chunk in zones.iter().take(4).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|zone| {
                    let selected = selected_environment_sensor(app)
                        .is_some_and(|sensor| sensor.zone == **zone);
                    ViewControlButton::new(
                        format!("fabricad.viewctl.environment.zone.{zone}"),
                        zone.to_string(),
                        selected,
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn inventory_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in InventoryQuickFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.inventory.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            inventory_filter_count(&app.workspace, *filter)
                        ),
                        app.inventory_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    for chunk in app
        .workspace
        .inventory
        .sorted_lots()
        .into_iter()
        .filter(|lot| inventory_filter_matches(app.inventory_filter, lot))
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.inventory.lot.{}", lot.id),
                        lot.id.to_string(),
                        app.selected_inventory_lot.as_ref() == Some(&lot.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn scheduler_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    rows.push(
        [
            DispatchPolicy::PriorityThenFifo,
            DispatchPolicy::DueDateThenPriority,
        ]
        .iter()
        .map(|policy| {
            ViewControlButton::new(
                format!(
                    "fabricad.viewctl.scheduler.policy.{}",
                    dispatch_policy_slug(*policy)
                ),
                policy.label(),
                app.scheduler_policy == *policy,
            )
        })
        .collect(),
    );
    rows.push(vec![ViewControlButton::new(
        format!(
            "fabricad.viewctl.scheduler.policy.{}",
            dispatch_policy_slug(DispatchPolicy::Fifo)
        ),
        DispatchPolicy::Fifo.label(),
        app.scheduler_policy == DispatchPolicy::Fifo,
    )]);
    for chunk in [0_u8, 1, 2, 3, 4, 5].chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|priority| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.scheduler.priority.{priority}"),
                        format!("P{priority}+"),
                        app.scheduler_min_priority == *priority,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.scheduler.toggle_conflicts",
            "Conflicts",
            app.scheduler_conflicts_only,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.scheduler.toggle_focus_tool",
            "Tool",
            app.scheduler_focus_selected_tool,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.scheduler.reset",
        "Reset",
        false,
    )]);
    for chunk in app
        .workspace
        .scheduler
        .tools
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|tool| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.scheduler.tool.{}", tool.id),
                        tool.id.to_string(),
                        app.selected_scheduler_tool.as_ref() == Some(&tool.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn safety_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let lockouts = app.workspace.safety.evaluate_lockouts();
    for chunk in lockouts.iter().take(6).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|lockout| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.safety.tool.{}", lockout.tool_id),
                        lockout.tool_id.clone(),
                        app.selected_safety_tool.as_deref() == Some(lockout.tool_id.as_str()),
                    )
                })
                .collect(),
        );
    }
    for chunk in app
        .workspace
        .safety
        .active_conditions()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|sensor| {
                    let sensor_id = sensor.id.to_string();
                    ViewControlButton::new(
                        format!("fabricad.viewctl.safety.ack.condition.{sensor_id}"),
                        format!("Ack {sensor_id}"),
                        app.acknowledged_conditions.contains(&sensor_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(lockout) = selected_safety_lockout(app) {
        rows.push(vec![ViewControlButton::new(
            format!("fabricad.viewctl.safety.ack.lockout.{}", lockout.tool_id),
            "Ack lockout",
            app.acknowledged_lockouts.contains(&lockout.tool_id),
        )]);
    }
    for chunk in app
        .workspace
        .safety
        .incidents
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|incident| {
                    let id = incident.id.to_string();
                    ViewControlButton::new(
                        format!("fabricad.viewctl.safety.ack.incident.{id}"),
                        format!("Ack {id}"),
                        app.acknowledged_incidents.contains(&id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn traceability_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    let related_wafers = selected_trace_wafer_ref(app)
        .map(|wafer| {
            app.workspace
                .genealogy
                .wafer_lineage(&wafer)
                .into_iter()
                .chain(app.workspace.genealogy.wafer_descendants(&wafer))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    for chunk in app
        .workspace
        .genealogy
        .lot_ids()
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.trace.lot.{lot_id}"),
                        lot_id.to_string(),
                        app.selected_trace_lot.as_ref() == Some(lot_id),
                    )
                })
                .collect(),
        );
    }
    if let Some(lot_id) = app.selected_trace_lot.as_ref() {
        for chunk in app
            .workspace
            .genealogy
            .wafer_refs_for_lot(lot_id)
            .into_iter()
            .filter(|wafer| {
                !app.trace_related_only
                    || related_wafers.is_empty()
                    || related_wafers.contains(wafer)
            })
            .take(6)
            .collect::<Vec<_>>()
            .chunks(2)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|wafer| {
                        ViewControlButton::new(
                            format!(
                                "fabricad.viewctl.trace.wafer.{}|{}",
                                wafer.lot_id, wafer.wafer_id
                            ),
                            wafer.wafer_id.to_string(),
                            app.selected_trace_lot.as_ref() == Some(&wafer.lot_id)
                                && app.selected_trace_wafer.as_ref() == Some(&wafer.wafer_id),
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.trace.toggle_related",
        "Related",
        app.trace_related_only,
    )]);
    rows.push(
        TraceImpactMode::ALL
            .iter()
            .map(|mode| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.trace.impact.{}", mode.slug()),
                    mode.label(),
                    app.trace_impact_mode == *mode,
                )
            })
            .collect(),
    );
    if let Some(wafer) = selected_trace_wafer_ref(app) {
        let process_buttons = app
            .workspace
            .genealogy
            .inherited_process_history_for_wafer(&wafer)
            .into_iter()
            .rev()
            .take(4)
            .map(|record| {
                ViewControlButton::new(
                    trace_detail_action(&TraceSelection::Process(record.sequence)),
                    format!("P{}", record.sequence),
                    app.selected_trace_detail == Some(TraceSelection::Process(record.sequence)),
                )
            })
            .collect::<Vec<_>>();
        for chunk in process_buttons.chunks(2) {
            rows.push(chunk.to_vec());
        }

        let material_buttons = app
            .workspace
            .genealogy
            .material_ancestry_for_wafer(&wafer)
            .into_iter()
            .rev()
            .take(4)
            .map(|record| {
                ViewControlButton::new(
                    trace_detail_action(&TraceSelection::MaterialUse(record.sequence)),
                    format!("M{}", record.sequence),
                    app.selected_trace_detail == Some(TraceSelection::MaterialUse(record.sequence)),
                )
            })
            .collect::<Vec<_>>();
        for chunk in material_buttons.chunks(2) {
            rows.push(chunk.to_vec());
        }
    }
    let event_buttons = app
        .workspace
        .genealogy
        .events
        .iter()
        .rev()
        .take(4)
        .map(|event| {
            ViewControlButton::new(
                trace_detail_action(&TraceSelection::Event(event.sequence)),
                format!("E{}", event.sequence),
                app.selected_trace_detail == Some(TraceSelection::Event(event.sequence)),
            )
        })
        .collect::<Vec<_>>();
    for chunk in event_buttons.chunks(2) {
        rows.push(chunk.to_vec());
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn process_flow_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in ProcessFlowNodeFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.process_flow.filter.{}", filter.slug()),
                        format!(
                            "{} ({})",
                            filter.label(),
                            process_flow_filter_count(&app.workspace, *filter)
                        ),
                        app.process_flow_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.process_flow.toggle_errors",
            "Errors",
            app.process_flow_errors_only,
        ),
        ViewControlButton::new("fabricad.viewctl.process_flow.validate", "Validate", false),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.process_flow.export",
        "Export",
        false,
    )]);
    for chunk in app
        .workspace
        .process_flow
        .route
        .nodes
        .iter()
        .filter(|node| process_flow_node_matches(&app.workspace, app.process_flow_filter, node))
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|node| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.process_flow.node.{}", node.id),
                        node.id.to_string(),
                        app.selected_process_node.as_ref() == Some(&node.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn process_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    for chunk in app
        .workspace
        .process_control
        .loops
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|loop_definition| {
                    ViewControlButton::new(
                        format!(
                            "fabricad.viewctl.process_control.loop.{}",
                            loop_definition.id
                        ),
                        compact_button_label(&loop_definition.name, 16),
                        app.selected_control_loop.as_ref() == Some(&loop_definition.id),
                    )
                })
                .collect(),
        );
    }

    if let Some(loop_definition) = selected_control_loop(app) {
        let actions = app
            .workspace
            .process_control
            .actions_for_loop(&loop_definition.id);
        for chunk in actions.iter().take(6).collect::<Vec<_>>().chunks(2) {
            rows.push(
                chunk
                    .iter()
                    .map(|action| {
                        ViewControlButton::new(
                            format!("fabricad.viewctl.process_control.action.{}", action.id),
                            format!(
                                "{} {}",
                                short_control_action_label(&action.id),
                                action.state.label()
                            ),
                            app.selected_control_action.as_ref() == Some(&action.id),
                        )
                    })
                    .collect(),
            );
        }
    }

    if let Some(action) = selected_control_action(app) {
        let transitions = process_control_transition_options(action.state);
        if !transitions.is_empty() {
            rows.push(
                transitions
                    .iter()
                    .map(|transition| {
                        ViewControlButton::new(
                            format!(
                                "fabricad.viewctl.process_control.transition.{}|{}",
                                action.id, transition
                            ),
                            process_control_transition_label(transition),
                            false,
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn spc_fdc_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let monitor = spc_fdc_monitor(&app.workspace);
    let selected_chart = selected_spc_chart(app, &monitor)
        .map(|chart| chart.id.as_str().to_string())
        .or_else(|| app.selected_spc_chart.clone());
    let selected_trace = selected_fdc_trace(app, &monitor)
        .map(|trace| trace.id.clone())
        .or_else(|| app.selected_fdc_trace.clone());
    let mut rows = Vec::new();

    rows.push(
        SpcSeverityFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.spc.severity.{}", filter.slug()),
                    filter.label(),
                    app.spc_severity_filter == *filter,
                )
            })
            .collect(),
    );
    rows.push(
        SpcSourceFilter::ALL
            .iter()
            .map(|filter| {
                ViewControlButton::new(
                    format!("fabricad.viewctl.spc.source.{}", filter.slug()),
                    filter.label(),
                    app.spc_source_filter == *filter,
                )
            })
            .collect(),
    );

    for chunk in monitor.charts.iter().take(6).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|chart| {
                    let id = chart.id.as_str();
                    ViewControlButton::new(
                        format!("fabricad.viewctl.spc.chart.{id}"),
                        format!(
                            "{} ({})",
                            compact_button_label(&chart.metric, 10),
                            chart.violations.len()
                        ),
                        selected_chart.as_deref() == Some(id),
                    )
                })
                .collect(),
        );
    }

    for chunk in monitor.traces.iter().take(6).collect::<Vec<_>>().chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|trace| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.spc.trace.{}", trace.id),
                        format!(
                            "{} ({})",
                            compact_button_label(&trace.sensor_name, 10),
                            trace.violations.len()
                        ),
                        selected_trace.as_deref() == Some(trace.id.as_str()),
                    )
                })
                .collect(),
        );
    }

    if !app.spc_context_filter.is_empty() {
        rows.push(vec![ViewControlButton::new(
            "fabricad.viewctl.spc.clear_context",
            "Clear context",
            true,
        )]);
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn cross_section_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in (0..cross_section_snapshot_count(&app.workspace))
        .take(8)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|step| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.cross_section.step.{step}"),
                        format!("Step {step}"),
                        app.cross_section_step == *step,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.cross_section.toggle_mask",
            "Mask",
            app.cross_section_show_mask,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.cross_section.toggle_dimensions",
            "Dims",
            app.cross_section_show_dimensions,
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.cross_section.toggle_risks",
        "Risks",
        app.cross_section_show_risks,
    )]);
    for chunk in app
        .workspace
        .cross_section
        .materials
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|material| {
                    ViewControlButton::new(
                        format!(
                            "fabricad.viewctl.cross_section.material.{}",
                            material.id.as_str()
                        ),
                        material.name.clone(),
                        app.selected_cross_section_material.as_ref() == Some(&material.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn metrology_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in MetrologyMapMode::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|mode| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.metrology.mode.{}", mode.slug()),
                        mode.short_label(),
                        app.metrology_map_mode == *mode,
                    )
                })
                .collect(),
        );
    }
    for chunk in MeasurementKind::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|kind| {
                    ViewControlButton::new(
                        format!(
                            "fabricad.viewctl.metrology.kind.{}",
                            measurement_kind_slug(*kind)
                        ),
                        kind.label(),
                        app.metrology_kind == *kind,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.metrology.failed_only",
            "Failed",
            app.metrology_failed_only,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.metrology.next_attention",
            "Next",
            app.selected_die.is_some(),
        ),
    ]);
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.metrology.clear_die",
        "Clear",
        false,
    )]);
    rows
}

fn yield_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = Vec::new();
    for chunk in YieldMapFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.yield.filter.{}", filter.slug()),
                        filter.label(),
                        app.yield_map_filter == *filter,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.yield.attention",
            "Attention",
            app.show_only_attention_wafers,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.yield.excursions",
            "Excursions",
            app.show_only_excursions,
        ),
    ]);
    for chunk in app
        .workspace
        .yield_analysis
        .lots
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.yield.lot.{}", lot.id),
                        lot.id.clone(),
                        app.selected_yield_lot.as_deref() == Some(lot.id.as_str()),
                    )
                })
                .collect(),
        );
    }
    if let Some(lot_id) = app.selected_yield_lot.as_deref() {
        let wafers = app.workspace.yield_analysis.wafer_ids_for_lot(lot_id);
        for chunk in wafers.iter().take(4).collect::<Vec<_>>().chunks(2) {
            rows.push(
                chunk
                    .iter()
                    .map(|wafer_id| {
                        ViewControlButton::new(
                            format!("fabricad.viewctl.yield.wafer.{wafer_id}"),
                            wafer_id.to_string(),
                            app.selected_yield_wafer.as_deref() == Some(wafer_id.as_str()),
                        )
                    })
                    .collect(),
            );
        }
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn notebook_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let filtered_entries = notebook_filtered_entries(app);
    let mut rows: Vec<Vec<ViewControlButton>> = Vec::new();
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.notebook.preview",
            "Preview",
            app.notebook_preview_mode,
        ),
        ViewControlButton::new(
            "fabricad.viewctl.notebook.edit",
            "Edit",
            !app.notebook_preview_mode,
        ),
    ]);
    rows.push(vec![
        ViewControlButton::new(
            "fabricad.viewctl.notebook.toggle_followups",
            format!("Follow-ups ({})", filtered_entries.len()),
            app.notebook_followups_only,
        ),
        ViewControlButton::new("fabricad.viewctl.notebook.clear_filters", "Clear", false),
    ]);
    for chunk in NotebookEntryAction::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|action| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.notebook.entry_action.{}", action.slug()),
                        action.label(),
                        false,
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.notebook.tag.all",
        "All tags",
        app.notebook_tag_filter.is_none(),
    )]);
    for chunk in app
        .workspace
        .lab_notebook
        .tags()
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|tag| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.notebook.tag.{tag}"),
                        format!("#{tag}"),
                        app.notebook_tag_filter.as_deref() == Some(tag.as_str()),
                    )
                })
                .collect(),
        );
    }
    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.notebook.link.any",
        "Any link",
        app.notebook_link_kind_filter.is_none(),
    )]);
    for chunk in NotebookLinkKind::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|kind| {
                    ViewControlButton::new(
                        format!(
                            "fabricad.viewctl.notebook.link.{}",
                            notebook_link_kind_slug(*kind)
                        ),
                        format!(
                            "{} ({})",
                            kind.label(),
                            notebook_link_count_for_kind(app, *kind)
                        ),
                        app.notebook_link_kind_filter == Some(*kind),
                    )
                })
                .collect(),
        );
    }
    if let Some(entry) = selected_notebook_entry(app) {
        for chunk in notebook_entry_link_focus_tokens(entry)
            .iter()
            .take(4)
            .collect::<Vec<_>>()
            .chunks(2)
        {
            rows.push(
                chunk
                    .iter()
                    .map(|(kind, value)| {
                        ViewControlButton::new(
                            format!(
                                "fabricad.viewctl.notebook.focus_link.{}:{}",
                                notebook_link_kind_slug(*kind),
                                value
                            ),
                            compact_button_label(value, 14),
                            app.notebook_link_kind_filter == Some(*kind),
                        )
                    })
                    .collect(),
            );
        }
    }
    for chunk in filtered_entries
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|entry| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.notebook.entry.{}", entry.id),
                        compact_button_label(&entry.title, 14),
                        app.selected_notebook_entry.as_ref() == Some(&entry.id),
                    )
                })
                .collect(),
        );
    }
    rows.retain(|row| !row.is_empty());
    rows
}

fn experiment_control_rows(app: &FabricadApp) -> Vec<Vec<ViewControlButton>> {
    let mut rows = vec![
        vec![
            ViewControlButton::new(
                "fabricad.viewctl.experiment.pending_only",
                "Pending",
                app.experiment_show_missing_only,
            ),
            ViewControlButton::new("fabricad.viewctl.experiment.next_pending", "Next", false),
        ],
        vec![
            ViewControlButton::new("fabricad.viewctl.experiment.capture", "Capture", false),
            ViewControlButton::new(
                "fabricad.viewctl.experiment.capture_advance",
                "Advance",
                false,
            ),
        ],
        vec![
            ViewControlButton::new("fabricad.viewctl.experiment.use_demo", "Demo", false),
            ViewControlButton::new("fabricad.viewctl.experiment.use_target", "Target", false),
        ],
        vec![ViewControlButton::new(
            "fabricad.viewctl.experiment.capture_next_demo",
            "Next demo",
            false,
        )],
    ];

    rows.push(vec![ViewControlButton::new(
        "fabricad.viewctl.experiment.clear_filters",
        "Clear filters",
        false,
    )]);

    for chunk in app
        .workspace
        .experiment_plan
        .responses
        .iter()
        .take(6)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|response| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.experiment.response.{}", response.id),
                        compact_button_label(&response.name, 14),
                        app.selected_experiment_response.as_ref() == Some(&response.id),
                    )
                })
                .collect(),
        );
    }

    for chunk in ExperimentRunFilter::ALL.chunks(2) {
        rows.push(
            chunk
                .iter()
                .map(|filter| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.experiment.filter.{}", filter.slug()),
                        filter.short_label(),
                        app.experiment_run_filter == *filter,
                    )
                })
                .collect(),
        );
    }

    for chunk in app
        .experiment_lot_options()
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|lot_id| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.experiment.lot.{lot_id}"),
                        lot_id.to_string(),
                        app.experiment_lot_filter.as_deref() == Some(lot_id.as_str()),
                    )
                })
                .collect(),
        );
    }

    for chunk in app
        .filtered_experiment_run_rows()
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
    {
        rows.push(
            chunk
                .iter()
                .map(|row| {
                    ViewControlButton::new(
                        format!("fabricad.viewctl.experiment.run.{}", row.run_id),
                        row.run_id.clone(),
                        row.selected,
                    )
                })
                .collect(),
        );
    }

    rows.retain(|row| !row.is_empty());
    rows
}

fn add_control_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: &str,
    title: &str,
    rows: Vec<Vec<ViewControlButton>>,
    ui_scale: UiScale,
) {
    let row_height = ui_scale.value(30.0);
    let gap = ui_scale.value(6.0);
    let height = ui_scale.value(44.0)
        + rows.len() as f32 * row_height
        + rows.len().saturating_sub(1) as f32 * gap;
    let panel = document.add_child(
        parent,
        UiNode::container(
            name,
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                    gap,
                ),
                ui_scale.value(10.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(23, 30, 36, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(56, 70, 84, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        panel,
        format!("{name}.title"),
        title,
        text_style(
            ui_scale.value(15.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
    );

    for (row_index, row_buttons) in rows.iter().enumerate() {
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("{name}.row.{row_index}"),
                layout::with_gap_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::px(row_height)),
                    gap,
                ),
            ),
        );
        for button in row_buttons {
            add_button(
                document,
                row,
                button.name.clone(),
                button.label.clone(),
                button.selected,
                layout::size(
                    layout::px(ui_scale.value(132.0)),
                    layout::px(ui_scale.value(28.0)),
                ),
                ui_scale,
            );
        }
    }
}

fn add_tool_strip(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let strip = document.add_child(
        parent,
        UiNode::container(
            "fabricad.tool_strip",
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::row(),
                        layout::percent(1.0),
                        layout::px(ui_scale.value(42.0)),
                    ),
                    ui_scale.value(6.0),
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(20, 25, 30, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(46, 57, 68, 255),
                ui_scale.value(1.0),
            )),
            0.0,
        )),
    );

    if matches!(
        app.active_view,
        StartupView::Layout2d | StartupView::Layout3d
    ) {
        add_button(
            document,
            strip,
            "fabricad.menu.item.view.layout2d",
            "2D",
            app.active_view == StartupView::Layout2d,
            layout::size(
                layout::px(ui_scale.value(52.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            strip,
            "fabricad.menu.item.view.layout3d",
            "3D",
            app.active_view == StartupView::Layout3d,
            layout::size(
                layout::px(ui_scale.value(52.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }

    if app.active_view == StartupView::Layout2d {
        for tool in ToolMode::ALL {
            add_button(
                document,
                strip,
                format!("fabricad.tool.{}", tool.slug()),
                tool.label(),
                app.active_tool == tool,
                layout::size(
                    layout::px(ui_scale.value(82.0)),
                    layout::px(ui_scale.value(26.0)),
                ),
                ui_scale,
            );
        }
        add_button(
            document,
            strip,
            "fabricad.menu.item.edit.delete",
            "Delete",
            false,
            layout::size(
                layout::px(ui_scale.value(82.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    } else if app.active_view == StartupView::Layout3d {
        add_button(
            document,
            strip,
            "fabricad.menu.item.tools.reset3d",
            "Reset 3D",
            false,
            layout::size(
                layout::px(ui_scale.value(96.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
        add_button(
            document,
            strip,
            "fabricad.menu.item.tools.fullscreen",
            "Fullscreen",
            false,
            layout::size(
                layout::px(ui_scale.value(108.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }

    if app.has_inspector_panel() {
        add_button(
            document,
            strip,
            "fabricad.drawer.inspector",
            "Inspector",
            app.show_inspector,
            layout::size(
                layout::px(ui_scale.value(112.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
    if app.has_secondary_panel() {
        add_button(
            document,
            strip,
            "fabricad.drawer.layers",
            app.secondary_panel_label(),
            app.show_layers,
            layout::size(
                layout::px(ui_scale.value(96.0)),
                layout::px(ui_scale.value(26.0)),
            ),
            ui_scale,
        );
    }
}

#[derive(Clone, Debug)]
struct DetailSection {
    title: String,
    rows: Vec<(String, String)>,
}

impl DetailSection {
    fn new(title: impl Into<String>, rows: Vec<(String, String)>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }
}

fn layout_editor_detail_rows(app: &FabricadApp) -> Vec<(String, String)> {
    let document = &app.workspace.document;
    let active_layer = document
        .layers
        .get(&app.active_layer)
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| format!("L{} missing", app.active_layer.0));
    let selected = app.selected_layout_shape_ref();
    let selected_shape = selected
        .as_ref()
        .map(|shape| format!("#{} {}", shape.id.0, shape_kind_label(&shape.kind)))
        .unwrap_or_else(|| "None".to_string());
    let selected_layer = selected
        .as_ref()
        .and_then(|shape| document.layers.get(&shape.layer))
        .map(|layer| format!("L{} {}", layer.id.0, layer.name))
        .unwrap_or_else(|| "None".to_string());
    let selected_bounds = selected
        .as_ref()
        .map(|shape| rect_summary(shape.kind.bounds()))
        .unwrap_or_else(|| "None".to_string());
    vec![
        (
            "Active view".to_string(),
            app.active_view.label().to_string(),
        ),
        ("Active layer".to_string(), active_layer),
        ("Selected shape".to_string(), selected_shape),
        ("Selection layer".to_string(), selected_layer),
        ("Selection bounds".to_string(), selected_bounds),
        (
            "Display".to_string(),
            format!(
                "grid={}, snap={}, drc={}",
                app.show_grid, app.snap_enabled, app.show_drc_overlay
            ),
        ),
        (
            "Markers".to_string(),
            format!(
                "drc={}, connectivity={}",
                document.marker_states.len(),
                document.connectivity_issue_states.len()
            ),
        ),
    ]
}

fn view_detail_sections(app: &FabricadApp) -> Vec<DetailSection> {
    let workspace = &app.workspace;
    let document = &workspace.document;
    let overview = DetailSection::new(
        "Workspace Overview",
        vec![
            ("Document".to_string(), document.name.clone()),
            (
                "Shapes".to_string(),
                document.flattened_shape_count_estimate().to_string(),
            ),
            ("Layers".to_string(), document.layers.len().to_string()),
            ("Cells".to_string(), document.cells.len().to_string()),
            ("Tool".to_string(), app.active_tool.label().to_string()),
            (
                "Display".to_string(),
                format!(
                    "grid={}, snap={}, drc={}",
                    app.show_grid, app.snap_enabled, app.show_drc_overlay
                ),
            ),
        ],
    );

    let specific = match app.active_view {
        StartupView::Workflow => DetailSection::new(
            "Fab Workflow",
            vec![
                (
                    "Focus lot".to_string(),
                    app.workflow_focus_lot
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                ("Lots".to_string(), workspace.mes.lots.len().to_string()),
                ("Traveler".to_string(), workflow_focus_traveler_label(app)),
                (
                    "Dispatch lots".to_string(),
                    workspace.scheduler.lots.len().to_string(),
                ),
                (
                    "Tools online model".to_string(),
                    workspace.equipment.tools().count().to_string(),
                ),
                ("Yield".to_string(), workflow_focus_yield_label(app)),
                (
                    "Safety sensors".to_string(),
                    workspace.safety.sensors.len().to_string(),
                ),
                (
                    "Notebook entries".to_string(),
                    workspace.lab_notebook.entries.len().to_string(),
                ),
            ],
        ),
        StartupView::Layout2d | StartupView::Layout3d => {
            DetailSection::new("Layout Editor", layout_editor_detail_rows(app))
        }
        StartupView::MaskPrep => {
            let prep = reticle_prep_for_app(app);
            let report = prep.validate_document(document);
            DetailSection::new(
                "Reticle Prep",
                vec![
                    (
                        "Lot".to_string(),
                        app.mask_source_lot
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "Unlinked".to_string()),
                    ),
                    ("Reticle".to_string(), prep.reticle.id.to_string()),
                    (
                        "Severity".to_string(),
                        app.mask_issue_severity_filter.detail_label().to_string(),
                    ),
                    (
                        "Grouping".to_string(),
                        app.mask_issue_grouping.label().to_string(),
                    ),
                    (
                        "Filtered issues".to_string(),
                        mask_filtered_issue_count(app, &report).to_string(),
                    ),
                    (
                        "Groups".to_string(),
                        mask_issue_group_count(app, &report).to_string(),
                    ),
                    ("Layers".to_string(), report.layer_count.to_string()),
                    ("Fields".to_string(), report.field_count.to_string()),
                ],
            )
        }
        StartupView::LayoutDiff => {
            let report = layout_diff_report(app);
            DetailSection::new(
                "Layout Diff",
                vec![
                    (
                        "Baseline".to_string(),
                        app.layout_diff_baseline.detail_label().to_string(),
                    ),
                    (
                        "Candidate".to_string(),
                        app.layout_diff_candidate.detail_label().to_string(),
                    ),
                    (
                        "Review".to_string(),
                        app.layout_diff_review_state.label().to_string(),
                    ),
                    (
                        "Filter".to_string(),
                        app.layout_diff_change_filter.detail_label().to_string(),
                    ),
                    (
                        "Changed only".to_string(),
                        app.layout_diff_changed_only.to_string(),
                    ),
                    (
                        "Filtered changes".to_string(),
                        layout_diff_filtered_change_count(app, &report).to_string(),
                    ),
                    (
                        "Total changes".to_string(),
                        layout_diff_total_changes(&report).to_string(),
                    ),
                    ("Layers".to_string(), report.layers.len().to_string()),
                ],
            )
        }
        StartupView::FabControl => DetailSection::new("Fab Control", {
            let mut rows = vec![
                (
                    "Selected".to_string(),
                    selected_equipment_tool(app)
                        .map(|tool| tool.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Tools".to_string(),
                    workspace.equipment.tools().count().to_string(),
                ),
                (
                    "Running".to_string(),
                    equipment_running_count(workspace).to_string(),
                ),
                (
                    "Active alarms".to_string(),
                    equipment_alarm_count(workspace).to_string(),
                ),
                (
                    "Samples".to_string(),
                    equipment_sample_count(workspace).to_string(),
                ),
            ];
            if let Some(tool) = selected_equipment_tool(app) {
                rows.push((tool.name.clone(), tool.state.label().to_string()));
                rows.push(("Recipe".to_string(), equipment_recipe_summary(tool)));
            }
            rows
        }),
        StartupView::Inventory => DetailSection::new(
            "Inventory",
            vec![
                (
                    "Selected".to_string(),
                    selected_inventory_lot(app)
                        .map(|lot| lot.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Filter".to_string(),
                    app.inventory_filter.detail_label().to_string(),
                ),
                (
                    "Material lots".to_string(),
                    workspace.inventory.lots.len().to_string(),
                ),
                (
                    "Visible".to_string(),
                    inventory_filter_count(workspace, app.inventory_filter).to_string(),
                ),
                (
                    "Alerts".to_string(),
                    workspace
                        .inventory
                        .alerts(INVENTORY_DEMO_TODAY)
                        .len()
                        .to_string(),
                ),
                (
                    "Linked MES lots".to_string(),
                    workspace.mes.lots.len().to_string(),
                ),
            ],
        ),
        StartupView::Maintenance => DetailSection::new(
            "Maintenance",
            vec![
                (
                    "Selected".to_string(),
                    selected_maintenance_tool(app)
                        .map(|tool| tool.tool_id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Work filter".to_string(),
                    app.maintenance_work_filter.detail_label().to_string(),
                ),
                (
                    "History".to_string(),
                    app.maintenance_history_filter.label().to_string(),
                ),
                (
                    "Tools".to_string(),
                    workspace.maintenance.tools.len().to_string(),
                ),
                (
                    "Action queue".to_string(),
                    maintenance_filtered_work_count(workspace, MaintenanceWorkFilter::Actionable)
                        .to_string(),
                ),
                (
                    "Locked".to_string(),
                    maintenance_locked_count(workspace).to_string(),
                ),
                (
                    "Calibrations".to_string(),
                    workspace.maintenance.calibration_records.len().to_string(),
                ),
                (
                    "Qualifications".to_string(),
                    workspace
                        .maintenance
                        .qualification_results
                        .len()
                        .to_string(),
                ),
                (
                    "Downtime".to_string(),
                    workspace.maintenance.downtime.len().to_string(),
                ),
            ],
        ),
        StartupView::Environment => DetailSection::new(
            "Cleanroom Environment",
            vec![
                (
                    "Selected".to_string(),
                    selected_environment_sensor(app)
                        .map(|sensor| sensor.id.clone())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                ("Latest".to_string(), environment_sensor_status(app)),
                (
                    "Sensors".to_string(),
                    workspace.environment.sensors.len().to_string(),
                ),
                (
                    "Active alarms".to_string(),
                    workspace.environment.active_alarms().len().to_string(),
                ),
                (
                    "Readings".to_string(),
                    workspace.environment.readings.len().to_string(),
                ),
                (
                    "Alarms".to_string(),
                    workspace.environment.alarms.len().to_string(),
                ),
                (
                    "Events".to_string(),
                    workspace.environment.events.len().to_string(),
                ),
            ],
        ),
        StartupView::Scheduler => DetailSection::new(
            "Dispatch",
            vec![
                (
                    "Policy".to_string(),
                    app.scheduler_policy.label().to_string(),
                ),
                (
                    "Shift minute".to_string(),
                    workspace.scheduler.now_minute.to_string(),
                ),
                (
                    "Selected tool".to_string(),
                    app.selected_scheduler_tool
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Min priority".to_string(),
                    format!("P{}+", app.scheduler_min_priority),
                ),
                (
                    "Tools".to_string(),
                    workspace.scheduler.tools.len().to_string(),
                ),
                (
                    "Lots".to_string(),
                    workspace.scheduler.lots.len().to_string(),
                ),
                (
                    "Visible lots".to_string(),
                    scheduler_filtered_lot_count(app).to_string(),
                ),
                (
                    "Assignments".to_string(),
                    scheduler_assignment_count(app).to_string(),
                ),
                (
                    "Unscheduled".to_string(),
                    scheduler_unscheduled_count(app).to_string(),
                ),
            ],
        ),
        StartupView::Safety => DetailSection::new(
            "Safety",
            vec![
                (
                    "Sensors".to_string(),
                    workspace.safety.sensors.len().to_string(),
                ),
                ("State".to_string(), safety_highest_label(app).to_string()),
                (
                    "Selected tool".to_string(),
                    app.selected_safety_tool
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Acknowledged".to_string(),
                    app.acknowledged_count().to_string(),
                ),
                (
                    "Active conditions".to_string(),
                    workspace.safety.active_conditions().len().to_string(),
                ),
                (
                    "Alarm routes".to_string(),
                    workspace.safety.alarm_routes.len().to_string(),
                ),
                (
                    "Interlocks".to_string(),
                    workspace.safety.tool_interlocks.len().to_string(),
                ),
                (
                    "Incidents".to_string(),
                    workspace.safety.incidents.len().to_string(),
                ),
            ],
        ),
        StartupView::Traceability => {
            let selected_wafer = selected_trace_wafer_ref(app);
            let mut rows = trace_selected_detail_rows(app);
            rows.push((
                "Selected lot".to_string(),
                app.selected_trace_lot
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "None".to_string()),
            ));
            rows.push((
                "Selected wafer".to_string(),
                selected_wafer
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "None".to_string()),
            ));
            rows.push((
                "Impact mode".to_string(),
                app.trace_impact_mode.label().to_string(),
            ));
            rows.push((
                "Related / impacted".to_string(),
                format!(
                    "{} / {}",
                    trace_related_wafer_count(app),
                    trace_impact_count(app)
                ),
            ));
            DetailSection::new("Lot Traceability", rows)
        }
        StartupView::Metrology => DetailSection::new(
            "Metrology",
            vec![
                ("Wafer".to_string(), workspace.wafer_map.name.clone()),
                (
                    "Map".to_string(),
                    app.metrology_map_mode.label().to_string(),
                ),
                (
                    "Measurement".to_string(),
                    app.metrology_kind.label().to_string(),
                ),
                (
                    "Failed only".to_string(),
                    app.metrology_failed_only.to_string(),
                ),
                (
                    "Selected die".to_string(),
                    die_coord_label(app.selected_die),
                ),
                (
                    "Dies".to_string(),
                    workspace.wafer_map.dies.len().to_string(),
                ),
                (
                    "Measurements".to_string(),
                    workspace.wafer_map.measurements.len().to_string(),
                ),
                (
                    "Defects".to_string(),
                    workspace.wafer_map.defects.len().to_string(),
                ),
                (
                    "Annotations".to_string(),
                    workspace.wafer_map.annotations.len().to_string(),
                ),
            ],
        ),
        StartupView::Yield => DetailSection::new(
            "Yield Dashboard",
            vec![
                (
                    "Lot".to_string(),
                    app.selected_yield_lot
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Wafer".to_string(),
                    app.selected_yield_wafer
                        .clone()
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Filter".to_string(),
                    app.yield_map_filter.label().to_string(),
                ),
                (
                    "Attention only".to_string(),
                    app.show_only_attention_wafers.to_string(),
                ),
                (
                    "Excursions only".to_string(),
                    app.show_only_excursions.to_string(),
                ),
                (
                    "Lots".to_string(),
                    workspace.yield_analysis.lots.len().to_string(),
                ),
                (
                    "Wafer summaries".to_string(),
                    workspace.yield_analysis.wafer_summaries.len().to_string(),
                ),
                (
                    "Lot summaries".to_string(),
                    workspace.yield_analysis.lot_summaries.len().to_string(),
                ),
                (
                    "Correlations".to_string(),
                    workspace.yield_analysis.correlations.len().to_string(),
                ),
            ],
        ),
        StartupView::SpcFdc => {
            let monitor = spc_fdc_monitor(workspace);
            let selected_chart = selected_spc_chart(app, &monitor);
            let selected_trace = selected_fdc_trace(app, &monitor);
            DetailSection::new(
                "SPC / FDC",
                vec![
                    (
                        "Selected chart".to_string(),
                        selected_chart
                            .map(|chart| chart.id.as_str().to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Selected trace".to_string(),
                        selected_trace
                            .map(|trace| trace.id.clone())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Severity".to_string(),
                        app.spc_severity_filter.detail_label().to_string(),
                    ),
                    (
                        "Source".to_string(),
                        app.spc_source_filter.detail_label().to_string(),
                    ),
                    (
                        "Filtered findings".to_string(),
                        spc_filtered_finding_count(app, &monitor).to_string(),
                    ),
                    (
                        "Critical".to_string(),
                        monitor
                            .finding_count_by_severity(MonitorSeverity::Critical)
                            .to_string(),
                    ),
                    ("Charts".to_string(), monitor.charts.len().to_string()),
                    ("Traces".to_string(), monitor.traces.len().to_string()),
                ],
            )
        }
        StartupView::ProcessFlow => DetailSection::new(
            "Process Flow",
            vec![
                (
                    "Route".to_string(),
                    workspace.process_flow.route.name.clone(),
                ),
                (
                    "Selected node".to_string(),
                    selected_process_flow_node(app)
                        .map(|node| node.id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Filter".to_string(),
                    app.process_flow_filter.label().to_string(),
                ),
                (
                    "Errors only".to_string(),
                    app.process_flow_errors_only.to_string(),
                ),
                (
                    "Nodes".to_string(),
                    workspace.process_flow.route.nodes.len().to_string(),
                ),
                (
                    "Edges".to_string(),
                    workspace.process_flow.route.edges.len().to_string(),
                ),
                (
                    "Errors".to_string(),
                    process_flow_error_count(workspace).to_string(),
                ),
                (
                    "Versions".to_string(),
                    workspace.process_flow.versions.len().to_string(),
                ),
            ],
        ),
        StartupView::ProcessControl => {
            let selected_loop = selected_control_loop(app);
            let selected_action = selected_control_action(app);
            DetailSection::new(
                "Run-to-Run Control",
                vec![
                    (
                        "Selected loop".to_string(),
                        selected_loop
                            .map(|loop_definition| loop_definition.id.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Selected action".to_string(),
                        selected_action
                            .map(|action| action.id.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Action state".to_string(),
                        selected_action
                            .map(|action| action.state.label().to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Proposed".to_string(),
                        process_control_action_count(app, ControlActionState::Proposed).to_string(),
                    ),
                    (
                        "Approved".to_string(),
                        process_control_action_count(app, ControlActionState::Approved).to_string(),
                    ),
                    (
                        "Loops".to_string(),
                        workspace.process_control.loops.len().to_string(),
                    ),
                    (
                        "Actions".to_string(),
                        workspace.process_control.actions.len().to_string(),
                    ),
                    (
                        "Audit events".to_string(),
                        workspace.process_control.audit_events.len().to_string(),
                    ),
                ],
            )
        }
        StartupView::CrossSection => DetailSection::new(
            "Process Cross-Section",
            vec![
                (
                    "Step".to_string(),
                    format!(
                        "{} / {}",
                        app.cross_section_step,
                        cross_section_snapshot_count(workspace).saturating_sub(1)
                    ),
                ),
                ("Selected".to_string(), cross_section_selected_title(app)),
                (
                    "Material".to_string(),
                    app.selected_cross_section_material
                        .as_ref()
                        .map(|material| material.as_str().to_string())
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Overlays".to_string(),
                    format!(
                        "mask={}, dims={}, risks={}",
                        app.cross_section_show_mask,
                        app.cross_section_show_dimensions,
                        app.cross_section_show_risks
                    ),
                ),
                (
                    "Width".to_string(),
                    format!("{:.1} um", workspace.cross_section.width_um),
                ),
                (
                    "Columns".to_string(),
                    workspace.cross_section.columns.to_string(),
                ),
                (
                    "Materials".to_string(),
                    workspace.cross_section.materials.len().to_string(),
                ),
                (
                    "Steps".to_string(),
                    workspace.cross_section.steps.len().to_string(),
                ),
            ],
        ),
        StartupView::Experiment => DetailSection::new(
            "DOE Planner",
            vec![
                ("Plan".to_string(), workspace.experiment_plan.title.clone()),
                (
                    "Status".to_string(),
                    format!("{:?}", workspace.experiment_plan.status),
                ),
                (
                    "Selected run".to_string(),
                    app.selected_experiment_run_label(),
                ),
                (
                    "Primary response".to_string(),
                    app.selected_experiment_response_label(),
                ),
                (
                    "Response spec".to_string(),
                    app.selected_experiment_response
                        .as_ref()
                        .and_then(|response_id| workspace.experiment_plan.response(response_id))
                        .map(experiment_response_spec_label)
                        .unwrap_or_else(|| "None".to_string()),
                ),
                (
                    "Run filter".to_string(),
                    app.experiment_run_filter.label().to_string(),
                ),
                (
                    "Lot filter".to_string(),
                    app.experiment_lot_filter
                        .clone()
                        .unwrap_or_else(|| "All lots".to_string()),
                ),
                (
                    "Capture value".to_string(),
                    format_compact_number(app.experiment_capture_value),
                ),
                (
                    "Factors".to_string(),
                    workspace.experiment_plan.factors.len().to_string(),
                ),
                (
                    "Runs".to_string(),
                    workspace.experiment_plan.runs.len().to_string(),
                ),
                (
                    "Pending runs".to_string(),
                    app.experiment_pending_count().to_string(),
                ),
                (
                    "Pending only".to_string(),
                    app.experiment_show_missing_only.to_string(),
                ),
            ],
        ),
        StartupView::Notebook => {
            let filtered_entries = notebook_filtered_entries(app);
            let selected_entry = selected_notebook_entry(app);
            DetailSection::new(
                "Lab Notebook",
                vec![
                    (
                        "Selected".to_string(),
                        selected_entry
                            .map(|entry| entry.id.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ),
                    (
                        "Mode".to_string(),
                        if app.notebook_preview_mode {
                            "Preview"
                        } else {
                            "Edit"
                        }
                        .to_string(),
                    ),
                    (
                        "Tag".to_string(),
                        app.notebook_tag_filter
                            .clone()
                            .unwrap_or_else(|| "All tags".to_string()),
                    ),
                    (
                        "Link".to_string(),
                        app.notebook_link_kind_filter
                            .map(|kind| kind.label().to_string())
                            .unwrap_or_else(|| "Any link".to_string()),
                    ),
                    (
                        "Follow-ups only".to_string(),
                        app.notebook_followups_only.to_string(),
                    ),
                    ("Matches".to_string(), filtered_entries.len().to_string()),
                    (
                        "Entries".to_string(),
                        workspace.lab_notebook.entries.len().to_string(),
                    ),
                ],
            )
        }
    };

    let selection = DetailSection::new(
        "Current Controls",
        vec![
            (
                "Menu".to_string(),
                app.active_menu.map_or("Closed", AppMenu::label).to_string(),
            ),
            ("Inspector".to_string(), app.show_inspector.to_string()),
            (
                app.secondary_panel_label().to_string(),
                app.show_layers.to_string(),
            ),
            (
                "Theme".to_string(),
                if app.dark_theme { "Dark" } else { "Light" }.to_string(),
            ),
        ],
    );

    vec![overview, specific, selection]
}

fn add_detail_sections(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let sections = view_detail_sections(app);
    let total_rows = sections
        .iter()
        .map(|section| section.rows.len().min(6))
        .sum::<usize>();
    let height = ui_scale.value(52.0 + total_rows as f32 * 28.0 + sections.len() as f32 * 24.0);
    let root = document.add_child(
        parent,
        UiNode::container(
            "fabricad.view.detail",
            layout::with_gap_all(
                layout::with_size(layout::column(), layout::percent(1.0), layout::px(height)),
                ui_scale.value(10.0),
            ),
        ),
    );

    for (section_index, section) in sections.iter().enumerate() {
        let row_count = section.rows.len().min(6);
        let section_height = ui_scale.value(38.0 + row_count as f32 * 28.0);
        let card = document.add_child(
            root,
            UiNode::container(
                format!("fabricad.view.detail.section.{section_index}"),
                layout::with_padding_all(
                    layout::with_gap_all(
                        layout::with_size(
                            layout::column(),
                            layout::percent(1.0),
                            layout::px(section_height),
                        ),
                        ui_scale.value(6.0),
                    ),
                    ui_scale.value(10.0),
                ),
            )
            .with_visual(UiVisual::panel(
                ColorRgba::new(24, 31, 38, 255),
                Some(StrokeStyle::new(
                    ColorRgba::new(55, 68, 82, 255),
                    ui_scale.value(1.0),
                )),
                ui_scale.value(4.0),
            )),
        );
        add_text(
            document,
            card,
            format!("fabricad.view.detail.section.{section_index}.title"),
            section.title.clone(),
            text_style(
                ui_scale.value(15.0),
                FontWeight::BOLD,
                ColorRgba::new(236, 242, 246, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(24.0))),
        );
        for (row_index, (label, value)) in section.rows.iter().take(6).enumerate() {
            add_text(
                document,
                card,
                format!("fabricad.view.detail.section.{section_index}.row.{row_index}"),
                format!("{label}: {value}"),
                text_style(
                    ui_scale.value(13.0),
                    FontWeight::NORMAL,
                    ColorRgba::new(177, 187, 197, 255),
                ),
                layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
            );
        }
    }
}

fn add_inspector_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
    let mut rows = vec![
        ("View".to_string(), app.active_view.label().to_string()),
        ("Tool".to_string(), app.active_tool.label().to_string()),
        (
            "Active layer".to_string(),
            app.workspace
                .document
                .layers
                .get(&app.active_layer)
                .map(|layer| format!("L{} {}", layer.id.0, layer.name))
                .unwrap_or_else(|| format!("L{} missing", app.active_layer.0)),
        ),
        ("Status".to_string(), app.status_message.clone()),
        (
            "Workspace".to_string(),
            format!(
                "{} shapes, {} lots",
                app.workspace.document.flattened_shape_count_estimate(),
                app.workspace.mes.lots.len()
            ),
        ),
    ];
    if let Some(shape) = app.selected_layout_shape_ref() {
        rows.push((
            format!("Selected #{}", shape.id.0),
            format!(
                "{} {}",
                shape_kind_label(&shape.kind),
                rect_summary(shape.kind.bounds())
            ),
        ));
    }
    rows.extend(app.workspace.document.shapes.values().take(5).map(|shape| {
        (
            format!("Shape {}", shape.id.0),
            format!("{} on L{}", shape_kind_label(&shape.kind), shape.layer.0),
        )
    }));
    add_side_panel(
        document,
        parent,
        "fabricad.inspector",
        "Inspector",
        rows,
        ui_scale.value(280.0),
        ui_scale,
    );
}

fn layout_layer_panel_rows(app: &FabricadApp) -> Vec<(String, String)> {
    let mut layers = app.workspace.document.layers.values().collect::<Vec<_>>();
    layers.sort_by_key(|layer| (layer.display_order, layer.id));
    layers
        .into_iter()
        .take(12)
        .map(|layer| {
            let shape_count = app
                .workspace
                .document
                .shapes
                .values()
                .filter(|shape| shape.layer == layer.id)
                .count();
            let label = if app.active_layer == layer.id {
                format!("* L{} {}", layer.id.0, layer.name)
            } else {
                format!("L{} {}", layer.id.0, layer.name)
            };
            let state = format!(
                "{}{}; {} shape{}",
                if layer.visible { "visible" } else { "hidden" },
                if layer.locked { ", locked" } else { "" },
                shape_count,
                if shape_count == 1 { "" } else { "s" }
            );
            (label, state)
        })
        .collect()
}

fn add_secondary_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    app: &FabricadApp,
    ui_scale: UiScale,
) {
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
        StartupView::Layout2d | StartupView::Layout3d => layout_layer_panel_rows(app),
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
        "fabricad.secondary",
        app.secondary_panel_label(),
        rows,
        ui_scale.value(244.0),
        ui_scale,
    );
}

fn add_side_panel(
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
            UiNodeStyle {
                layout: layout::with_min_size(
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
                )
                .as_taffy_style()
                .clone(),
                clip: ClipBehavior::Clip,
                ..Default::default()
            },
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 28, 34, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(52, 64, 76, 255),
                ui_scale.value(1.0),
            )),
            0.0,
        ))
        .with_scroll(ScrollAxes::VERTICAL),
    );
    add_text(
        document,
        panel,
        format!("{name}.title"),
        title,
        text_style(
            ui_scale.value(16.0),
            FontWeight::BOLD,
            ColorRgba::new(238, 243, 247, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(28.0))),
    );
    for (index, (label, value)) in rows.iter().take(12).enumerate() {
        add_text(
            document,
            panel,
            format!("{name}.row.{index}"),
            format!("{label}: {value}"),
            text_style(
                ui_scale.value(12.0),
                FontWeight::NORMAL,
                ColorRgba::new(172, 183, 194, 255),
            ),
            layout::size(layout::percent(1.0), layout::px(ui_scale.value(30.0))),
        );
    }
}

fn add_menu_panel(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    menu: AppMenu,
    active_view: StartupView,
    app: &FabricadApp,
    viewport_width: f32,
    ui_scale: UiScale,
) {
    let item_width = ui_scale.value(160.0);
    let item_height = ui_scale.value(if menu == AppMenu::View { 40.0 } else { 34.0 });
    let gap = ui_scale.value(8.0);

    let items: Vec<(String, String)> = match menu {
        AppMenu::File => vec![
            (
                "file.new_blank".to_string(),
                "New Blank Workspace".to_string(),
            ),
            (
                "file.load_demo".to_string(),
                "Load Demo Workspace".to_string(),
            ),
            ("file.save".to_string(), "Save Workspace".to_string()),
            ("file.load".to_string(), "Load Workspace".to_string()),
            (
                "file.save_layout".to_string(),
                "Save Layout JSON".to_string(),
            ),
            (
                "file.load_layout".to_string(),
                "Load Layout JSON".to_string(),
            ),
            ("file.export_gds".to_string(), "Export GDS".to_string()),
            ("file.import_gds".to_string(), "Import GDS".to_string()),
            (
                "file.collaboration".to_string(),
                "Connect Collaboration".to_string(),
            ),
        ],
        AppMenu::Edit => vec![
            ("edit.undo".to_string(), "Undo".to_string()),
            ("edit.redo".to_string(), "Redo".to_string()),
            ("edit.copy".to_string(), "Copy".to_string()),
            ("edit.paste".to_string(), "Paste".to_string()),
            ("edit.duplicate".to_string(), "Duplicate".to_string()),
            ("edit.delete".to_string(), "Delete".to_string()),
            ("edit.make_cell".to_string(), "Make Cell".to_string()),
            ("edit.rotate90".to_string(), "Rotate 90".to_string()),
            ("edit.mirror_x".to_string(), "Mirror X".to_string()),
            ("edit.mirror_y".to_string(), "Mirror Y".to_string()),
        ],
        AppMenu::View => StartupView::ALL
            .iter()
            .map(|view| (format!("view.{}", view.slug()), view.label().to_string()))
            .collect(),
        AppMenu::Bookmarks => vec![
            ("bookmarks.origin".to_string(), "Origin".to_string()),
            ("bookmarks.bounds".to_string(), "Layout Bounds".to_string()),
        ],
        AppMenu::Display => vec![
            (
                "display.grid2d".to_string(),
                (if app.show_grid {
                    "Hide 2D Grid"
                } else {
                    "Show 2D Grid"
                })
                .to_string(),
            ),
            (
                "display.grid3d".to_string(),
                (if app.show_3d_grid {
                    "Hide 3D Grid"
                } else {
                    "Show 3D Grid"
                })
                .to_string(),
            ),
            (
                "display.origin".to_string(),
                (if app.show_origin_marker {
                    "Hide Origin"
                } else {
                    "Show Origin"
                })
                .to_string(),
            ),
            (
                "display.drc".to_string(),
                (if app.show_drc_overlay {
                    "Hide DRC Overlay"
                } else {
                    "Show DRC Overlay"
                })
                .to_string(),
            ),
            (
                "display.theme".to_string(),
                (if app.dark_theme {
                    "Light Theme"
                } else {
                    "Dark Theme"
                })
                .to_string(),
            ),
            ("display.units".to_string(), "Units".to_string()),
        ],
        AppMenu::Options => vec![
            (
                "options.grid".to_string(),
                (if app.show_grid {
                    "Hide Grid"
                } else {
                    "Show Grid"
                })
                .to_string(),
            ),
            (
                "options.snap".to_string(),
                if app.snap_enabled {
                    "Disable Snap"
                } else {
                    "Enable Snap"
                }
                .to_string(),
            ),
            (
                "options.theme".to_string(),
                if app.dark_theme {
                    "Light Theme"
                } else {
                    "Dark Theme"
                }
                .to_string(),
            ),
        ],
        AppMenu::Tools => {
            let mut items = vec![
                ("tools.palette".to_string(), "Command Palette".to_string()),
                ("tools.rundrc".to_string(), "Run DRC".to_string()),
                ("tools.diagnostics".to_string(), "Diagnostics".to_string()),
            ];
            for tool in ToolMode::ALL {
                items.push((format!("tool.{}", tool.slug()), tool.label().to_string()));
            }
            items
        }
        AppMenu::Macros => vec![
            ("macros.demo".to_string(), "Load Full Demo".to_string()),
            ("macros.stress10k".to_string(), "10k Stress".to_string()),
            ("macros.stress100k".to_string(), "100k Stress".to_string()),
            ("macros.stress1m".to_string(), "1M Stress".to_string()),
            ("macros.hierarchy".to_string(), "Hierarchy".to_string()),
            ("macros.validate".to_string(), "Validate".to_string()),
            ("macros.drc".to_string(), "Run DRC".to_string()),
            ("macros.snapshot".to_string(), "Snapshot".to_string()),
        ],
        AppMenu::Help => vec![
            ("help.shortcuts".to_string(), "Shortcuts".to_string()),
            ("help.about".to_string(), "About".to_string()),
        ],
    };
    let available_width = (viewport_width - 16.0).max(item_width);
    let columns = (((available_width + gap) / (item_width + gap)).floor() as usize)
        .max(1)
        .min(items.len().max(1));
    let row_count = (items.len() + columns - 1) / columns;
    let panel_height =
        row_count as f32 * item_height + row_count.saturating_sub(1) as f32 * gap + 16.0;

    let panel = document.add_child(
        parent,
        UiNode::container(
            format!("fabricad.menu_panel.{}", menu.slug()),
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_size(
                        layout::column(),
                        layout::percent(1.0),
                        layout::px(panel_height),
                    ),
                    gap,
                ),
                ui_scale.value(8.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(22, 28, 34, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(58, 72, 86, 255),
                ui_scale.value(1.0),
            )),
            0.0,
        )),
    );

    for (row_index, row_items) in items.chunks(columns).enumerate() {
        let row = document.add_child(
            panel,
            UiNode::container(
                format!("fabricad.menu_row.{}.{}", menu.slug(), row_index),
                layout::with_gap_all(
                    layout::with_size(layout::row(), layout::percent(1.0), layout::px(item_height)),
                    gap,
                ),
            ),
        );

        for (action, label) in row_items {
            let selected = action
                .strip_prefix("view.")
                .and_then(StartupView::from_slug)
                == Some(active_view)
                || action.strip_prefix("tool.").and_then(ToolMode::from_slug)
                    == Some(app.active_tool);
            add_button(
                document,
                row,
                format!("fabricad.menu.item.{action}"),
                label,
                selected,
                layout::size(layout::px(item_width), layout::px(item_height)),
                ui_scale,
            );
        }
    }
}

fn add_button(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    label: impl Into<String>,
    selected: bool,
    node_layout: impl Into<operad::LayoutStyle>,
    ui_scale: UiScale,
) -> operad::UiNodeId {
    let label = label.into();
    let fill = if selected {
        ColorRgba::new(45, 78, 103, 255)
    } else {
        ColorRgba::new(31, 39, 47, 255)
    };
    document.add_child(
        parent,
        UiNode::text(
            name,
            label.clone(),
            text_style(
                ui_scale.value(13.0),
                if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                },
                ColorRgba::new(229, 235, 240, 255),
            ),
            layout::with_padding_all(node_layout.into(), ui_scale.value(7.0)),
        )
        .with_input(InputBehavior::BUTTON)
        .with_accessibility(button_accessibility(&label))
        .with_visual(UiVisual::panel(
            fill,
            Some(StrokeStyle::new(
                ColorRgba::new(68, 82, 96, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    )
}

fn button_accessibility(label: &str) -> AccessibilityMeta {
    AccessibilityMeta::new(AccessibilityRole::Button)
        .label(label)
        .focusable()
        .action(AccessibilityAction::new("activate", "Activate"))
}

fn add_text(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    name: impl Into<String>,
    text: impl Into<String>,
    style: TextStyle,
    node_layout: impl Into<operad::LayoutStyle>,
) {
    document.add_child(parent, UiNode::text(name, text, style, node_layout));
}

fn add_metric(
    document: &mut UiDocument,
    parent: operad::UiNodeId,
    key: &str,
    value: String,
    label: &str,
    ui_scale: UiScale,
) {
    let tile = document.add_child(
        parent,
        UiNode::container(
            format!("fabricad.metric.{key}"),
            layout::with_padding_all(
                layout::with_gap_all(
                    layout::with_flex(layout::column(), 1.0, 1.0, layout::percent(0.0)),
                    ui_scale.value(4.0),
                ),
                ui_scale.value(12.0),
            ),
        )
        .with_visual(UiVisual::panel(
            ColorRgba::new(27, 34, 42, 255),
            Some(StrokeStyle::new(
                ColorRgba::new(56, 70, 84, 255),
                ui_scale.value(1.0),
            )),
            ui_scale.value(4.0),
        )),
    );
    add_text(
        document,
        tile,
        format!("fabricad.metric.{key}.value"),
        value,
        text_style(
            ui_scale.value(24.0),
            FontWeight::BOLD,
            ColorRgba::new(242, 247, 250, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(34.0))),
    );
    add_text(
        document,
        tile,
        format!("fabricad.metric.{key}.label"),
        label,
        text_style(
            ui_scale.value(12.0),
            FontWeight::NORMAL,
            ColorRgba::new(154, 165, 176, 255),
        ),
        layout::size(layout::percent(1.0), layout::px(ui_scale.value(22.0))),
    );
}

fn text_style(font_size: f32, weight: FontWeight, color: ColorRgba) -> TextStyle {
    TextStyle {
        font_size,
        line_height: (font_size * 1.25).ceil(),
        weight,
        color,
        wrap: TextWrap::WordOrGlyph,
        ..Default::default()
    }
}

fn layout_preview_primitives(
    document: &Document,
    selected_shape: Option<ShapeId>,
    ui_scale: UiScale,
) -> Vec<ScenePrimitive> {
    let shapes = document
        .shapes
        .values()
        .filter(|shape| {
            document
                .layers
                .get(&shape.layer)
                .is_none_or(|layer| layer.visible)
        })
        .take(320)
        .collect::<Vec<_>>();
    if shapes.is_empty() {
        return vec![ScenePrimitive::Line {
            from: UiPoint::new(ui_scale.value(24.0), ui_scale.value(180.0)),
            to: UiPoint::new(ui_scale.value(936.0), ui_scale.value(180.0)),
            stroke: StrokeStyle::new(ColorRgba::new(76, 92, 108, 255), ui_scale.value(1.0)),
        }];
    }

    let mut bounds: Option<Rect> = None;
    for shape in &shapes {
        if let Some(shape_bounds) = shape_bounds(&shape.kind) {
            bounds = Some(match bounds {
                Some(current) => current.union(shape_bounds),
                None => shape_bounds,
            });
        }
    }
    let bounds = bounds.unwrap_or_else(|| Rect::from_min_size(Point::ZERO, 1, 1));
    let scale_x: f32 = ui_scale.value(900.0) / bounds.width().max(1) as f32;
    let scale_y: f32 = ui_scale.value(300.0) / bounds.height().max(1) as f32;
    let scale: f32 = scale_x.min(scale_y).max(0.0001);
    let offset = UiPoint::new(
        ui_scale.value(30.0) - bounds.min.x as f32 * scale,
        ui_scale.value(30.0) - bounds.min.y as f32 * scale,
    );

    let mut primitives = Vec::with_capacity(shapes.len() + 2);
    primitives.push(ScenePrimitive::Line {
        from: UiPoint::new(ui_scale.value(24.0), ui_scale.value(328.0)),
        to: UiPoint::new(ui_scale.value(936.0), ui_scale.value(328.0)),
        stroke: StrokeStyle::new(ColorRgba::new(44, 57, 70, 255), ui_scale.value(1.0)),
    });

    for shape in shapes {
        let layer = document.layers.get(&shape.layer);
        let selected = selected_shape == Some(shape.id);
        let fill_alpha = if selected { 210 } else { 150 };
        let stroke_width = if selected {
            ui_scale.value(2.5)
        } else {
            ui_scale.value(1.0)
        };
        let fill = layer
            .map(|layer| layer_color(layer.color, fill_alpha))
            .unwrap_or_else(|| ColorRgba::new(92, 160, 220, fill_alpha));
        let stroke_color = if selected {
            ColorRgba::new(250, 219, 112, 255)
        } else {
            layer
                .map(|layer| layer_color(layer.color, 240))
                .unwrap_or_else(|| ColorRgba::new(132, 195, 245, 240))
        };
        let stroke = Some(StrokeStyle::new(stroke_color, stroke_width));
        match shape.kind {
            ShapeKind::Rectangle(rect) => {
                let mut rect = operad::PaintRect::solid(map_rect(rect, scale, offset), fill);
                if let Some(stroke) = stroke {
                    rect = rect.stroke(stroke);
                }
                primitives.push(ScenePrimitive::Rect(rect));
            }
            ShapeKind::Polygon(ref polygon) => {
                primitives.push(ScenePrimitive::Polygon {
                    points: polygon
                        .points
                        .iter()
                        .map(|point| map_point(*point, scale, offset))
                        .collect(),
                    fill,
                    stroke,
                });
            }
            ShapeKind::Path { ref points, .. } => {
                for pair in points.windows(2) {
                    primitives.push(ScenePrimitive::Line {
                        from: map_point(pair[0], scale, offset),
                        to: map_point(pair[1], scale, offset),
                        stroke: StrokeStyle::new(fill, ui_scale.value(2.0)),
                    });
                }
            }
            ShapeKind::Via { center, size, .. } => {
                primitives.push(ScenePrimitive::Circle {
                    center: map_point(center, scale, offset),
                    radius: ((size as f32 * scale) * 0.5)
                        .clamp(ui_scale.value(3.0), ui_scale.value(18.0)),
                    fill,
                    stroke,
                });
            }
            ShapeKind::Label { position, .. } => {
                primitives.push(ScenePrimitive::Circle {
                    center: map_point(position, scale, offset),
                    radius: ui_scale.value(4.0),
                    fill: ColorRgba::new(234, 218, 132, 220),
                    stroke: None,
                });
            }
            ShapeKind::Measurement { a, b, .. } => {
                primitives.push(ScenePrimitive::Line {
                    from: map_point(a, scale, offset),
                    to: map_point(b, scale, offset),
                    stroke: StrokeStyle::new(
                        ColorRgba::new(238, 188, 116, 230),
                        ui_scale.value(1.5),
                    ),
                });
            }
        }
        if selected && let Some(bounds) = shape_bounds(&shape.kind) {
            primitives.push(ScenePrimitive::Rect(
                operad::PaintRect::solid(
                    map_rect(bounds.expanded(40), scale, offset),
                    ColorRgba::new(0, 0, 0, 0),
                )
                .stroke(StrokeStyle::new(
                    ColorRgba::new(250, 219, 112, 255),
                    ui_scale.value(1.5),
                )),
            ));
        }
    }

    primitives
}

fn shape_bounds(kind: &ShapeKind) -> Option<Rect> {
    Rect::from_points(&kind.key_points())
}

fn shape_kind_label(kind: &ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Rectangle(_) => "rectangle",
        ShapeKind::Polygon(_) => "polygon",
        ShapeKind::Path { .. } => "path",
        ShapeKind::Via { .. } => "via",
        ShapeKind::Label { .. } => "label",
        ShapeKind::Measurement { .. } => "measurement",
    }
}

fn compact_layer_label(layer_id: LayerId, name: &str) -> String {
    let mut label = name.chars().take(12).collect::<String>();
    if name.chars().count() > 12 {
        label.push_str("...");
    }
    format!("L{} {label}", layer_id.0)
}

fn rect_summary(rect: Rect) -> String {
    format!(
        "{},{} {}x{}",
        rect.min.x,
        rect.min.y,
        rect.width(),
        rect.height()
    )
}

fn map_rect(rect: Rect, scale: f32, offset: UiPoint) -> UiRect {
    let min = map_point(rect.min, scale, offset);
    UiRect::new(
        min.x,
        min.y,
        (rect.width().max(1) as f32 * scale).max(1.0),
        (rect.height().max(1) as f32 * scale).max(1.0),
    )
}

fn map_point(point: Point, scale: f32, offset: UiPoint) -> UiPoint {
    UiPoint::new(
        point.x as f32 * scale + offset.x,
        point.y as f32 * scale + offset.y,
    )
}

fn layer_color(color: [f32; 4], alpha: u8) -> ColorRgba {
    ColorRgba::new(
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        alpha,
    )
}

fn malformed_schema_field_workspace_json() -> Result<String, String> {
    let mut value = serde_json::to_value(WorkspaceDataset::blank())
        .map_err(|err| format!("failed to build malformed schema fixture: {err}"))?;
    value["schema_version"] = serde_json::Value::String("future".to_string());
    serde_json::to_string(&value)
        .map_err(|err| format!("failed to encode malformed schema fixture: {err}"))
}

fn assert_persistence_fixture_rejected(
    label: &str,
    contents: &str,
    required_messages: &[&str],
) -> Result<(), String> {
    let err = WorkspaceDataset::from_json_str(contents)
        .err()
        .ok_or_else(|| format!("{label} persistence fixture unexpectedly loaded"))?;
    for required in required_messages {
        if !err.contains(required) {
            return Err(format!(
                "{label} persistence fixture error {err:?} did not contain {required:?}"
            ));
        }
    }
    if err.contains("missing field") {
        return Err(format!(
            "{label} persistence fixture failed with generic missing-field parse error: {err}"
        ));
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct QualityConnectivityFixture {
    name: String,
    shapes: Vec<QualityFixtureShape>,
    expect: QualityConnectivityExpectation,
    #[serde(default)]
    issue_states: Vec<QualityConnectivityIssueState>,
}

#[derive(Debug, serde::Deserialize)]
struct QualityConnectivityExpectation {
    health: String,
    component_count: usize,
    labeled_component_count: usize,
    short_count: usize,
    short_names: Vec<String>,
    short_key: String,
    open_count: usize,
    open_name: String,
    open_key: String,
    open_component_count: usize,
    data_component_shape_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct QualityConnectivityIssueState {
    key: String,
    hidden: bool,
    waived: bool,
    note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConnectivityQualityValidation {
    component_count: usize,
    short_count: usize,
    open_count: usize,
    issue_key_count: usize,
    issue_state_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct QualityDrcFixture {
    name: String,
    shapes: Vec<QualityFixtureShape>,
    expect: QualityDrcExpectation,
}

#[derive(Debug, serde::Deserialize)]
struct QualityDrcExpectation {
    total_count: usize,
    omitted_count: usize,
    rule_counts: BTreeMap<String, usize>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum QualityFixtureShape {
    Rect {
        layer: String,
        x: Coord,
        y: Coord,
        w: Coord,
        h: Coord,
    },
    Label {
        layer: String,
        x: Coord,
        y: Coord,
        text: String,
    },
}

fn validate_drc_quality_fixture() -> Result<(usize, usize), String> {
    let fixture: QualityDrcFixture =
        serde_json::from_str(include_str!("../../../fixtures/quality/drc_golden.json"))
            .map_err(|err| format!("failed to parse DRC quality fixture: {err}"))?;
    let document = document_from_quality_fixture(&fixture.name, &fixture.shapes)?;
    let rules = RuleDeck::demo(&document);
    let rule_findings = rules.validate_for_document(&document);
    if !rule_findings.is_empty() {
        let detail = rule_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "DRC quality fixture has an invalid rule deck: {detail}"
        ));
    }
    let violations = run_drc(&document, &rules);
    let store = DrcIssueStore::from_violations(violations.clone());
    let validation_findings = store.validate();
    if !validation_findings.is_empty() {
        let detail = validation_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "DRC quality fixture generated an invalid issue store: {detail}"
        ));
    }
    let summary = store.summary(16);
    let mut rule_counts = BTreeMap::new();
    for violation in violations {
        *rule_counts.entry(violation.rule).or_insert(0usize) += 1;
    }

    if summary.total_count != fixture.expect.total_count {
        return Err(format!(
            "DRC quality fixture expected {} violations, got {}",
            fixture.expect.total_count, summary.total_count
        ));
    }
    if summary.omitted_count != fixture.expect.omitted_count {
        return Err(format!(
            "DRC quality fixture expected {} omitted rows, got {}",
            fixture.expect.omitted_count, summary.omitted_count
        ));
    }
    if rule_counts != fixture.expect.rule_counts {
        return Err(format!(
            "DRC quality fixture rule counts differ: expected {:?}, got {:?}",
            fixture.expect.rule_counts, rule_counts
        ));
    }
    Ok((summary.total_count, rule_counts.len()))
}

fn validate_connectivity_quality_fixture() -> Result<ConnectivityQualityValidation, String> {
    let fixture: QualityConnectivityFixture = serde_json::from_str(include_str!(
        "../../../fixtures/quality/connectivity_golden.json"
    ))
    .map_err(|err| format!("failed to parse connectivity quality fixture: {err}"))?;
    let document = document_from_quality_fixture(&fixture.name, &fixture.shapes)?;
    let report = extract_connectivity(&document, &layout_model::default_technology())
        .map_err(|err| format!("connectivity quality fixture failed: {err}"))?;
    let validation_findings = report.validate();
    if !validation_findings.is_empty() {
        let detail = validation_findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "connectivity quality fixture generated an invalid report: {detail}"
        ));
    }
    let summary = report.summary(8);

    if summary.health.label() != fixture.expect.health {
        return Err(format!(
            "connectivity quality fixture expected health {:?}, got {:?}",
            fixture.expect.health,
            summary.health.label()
        ));
    }
    if summary.component_count != fixture.expect.component_count {
        return Err(format!(
            "connectivity quality fixture expected {} components, got {}",
            fixture.expect.component_count, summary.component_count
        ));
    }
    if summary.labeled_component_count != fixture.expect.labeled_component_count {
        return Err(format!(
            "connectivity quality fixture expected {} labeled components, got {}",
            fixture.expect.labeled_component_count, summary.labeled_component_count
        ));
    }
    if report.shorts.len() != fixture.expect.short_count {
        return Err(format!(
            "connectivity quality fixture expected {} shorts, got {}",
            fixture.expect.short_count,
            report.shorts.len()
        ));
    }
    if report
        .shorts
        .first()
        .is_none_or(|short| short.names != fixture.expect.short_names)
    {
        return Err(format!(
            "connectivity quality fixture short names differ: expected {:?}, got {:?}",
            fixture.expect.short_names,
            report.shorts.first().map(|short| &short.names)
        ));
    }
    if report
        .shorts
        .first()
        .is_none_or(|short| short.stable_key() != fixture.expect.short_key)
    {
        return Err(format!(
            "connectivity quality fixture short key differs: expected {:?}, got {:?}",
            fixture.expect.short_key,
            report.shorts.first().map(|short| short.stable_key())
        ));
    }
    if report.opens.len() != fixture.expect.open_count {
        return Err(format!(
            "connectivity quality fixture expected {} opens, got {}",
            fixture.expect.open_count,
            report.opens.len()
        ));
    }
    if report.opens.first().is_none_or(|open| {
        open.name != fixture.expect.open_name
            || open.components.len() != fixture.expect.open_component_count
    }) {
        return Err(format!(
            "connectivity quality fixture open differs: expected {} with {} components, got {:?}",
            fixture.expect.open_name,
            fixture.expect.open_component_count,
            report.opens.first()
        ));
    }
    if report
        .opens
        .first()
        .is_none_or(|open| open.stable_key() != fixture.expect.open_key)
    {
        return Err(format!(
            "connectivity quality fixture open key differs: expected {:?}, got {:?}",
            fixture.expect.open_key,
            report.opens.first().map(|open| open.stable_key())
        ));
    }
    if !report.components.iter().any(|component| {
        component.net_name.as_deref() == Some("DATA")
            && component.shapes.len() == fixture.expect.data_component_shape_count
    }) {
        return Err(format!(
            "connectivity quality fixture missing DATA component with {} shapes",
            fixture.expect.data_component_shape_count
        ));
    }

    let store = report.issue_store();
    let states = fixture
        .issue_states
        .iter()
        .map(|state| {
            (
                state.key.clone(),
                MarkerState {
                    hidden: state.hidden,
                    waived: state.waived,
                    note: state.note.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for expected in &fixture.issue_states {
        let record = store.get(&expected.key).ok_or_else(|| {
            format!(
                "connectivity quality fixture missing expected issue key {:?}",
                expected.key
            )
        })?;
        if !matches!(
            record.kind(),
            ConnectivityIssueKind::Short | ConnectivityIssueKind::Open
        ) {
            return Err(format!(
                "connectivity quality fixture issue key {:?} resolved to unexpected kind {:?}",
                expected.key,
                record.kind()
            ));
        }
        let state = states.get(&expected.key).ok_or_else(|| {
            format!(
                "connectivity quality fixture missing expected issue state {:?}",
                expected.key
            )
        })?;
        if state.hidden != expected.hidden
            || state.waived != expected.waived
            || state.note != expected.note
        {
            return Err(format!(
                "connectivity quality fixture issue state {:?} differs: expected hidden={} waived={} note={:?}, got {:?}",
                expected.key, expected.hidden, expected.waived, expected.note, state
            ));
        }
    }

    Ok(ConnectivityQualityValidation {
        component_count: summary.component_count,
        short_count: report.shorts.len(),
        open_count: report.opens.len(),
        issue_key_count: fixture.issue_states.len(),
        issue_state_count: states.len(),
    })
}

fn document_from_quality_fixture(
    name: &str,
    shapes: &[QualityFixtureShape],
) -> Result<Document, String> {
    let mut document = Document::new(name);
    for shape in shapes {
        match shape {
            QualityFixtureShape::Rect { layer, x, y, w, h } => {
                let layer_id = quality_fixture_layer(&document, layer)?;
                document.insert_shape(
                    layer_id,
                    ShapeKind::Rectangle(Rect::from_min_size(Point::new(*x, *y), *w, *h)),
                );
            }
            QualityFixtureShape::Label { layer, x, y, text } => {
                let layer_id = quality_fixture_layer(&document, layer)?;
                document.insert_shape(
                    layer_id,
                    ShapeKind::Label {
                        position: Point::new(*x, *y),
                        text: text.clone(),
                    },
                );
            }
        }
    }
    Ok(document)
}

fn quality_fixture_layer(document: &Document, layer: &str) -> Result<LayerId, String> {
    let process = ProcessLayer::from_technology_name(layer)
        .ok_or_else(|| format!("unknown fixture layer {layer:?}"))?;
    document
        .layer_by_process(process)
        .ok_or_else(|| format!("fixture layer {layer:?} missing from document"))
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_offscreen_render_async(
    options: OffscreenRenderOptions,
) -> Result<OffscreenRenderReport, Box<dyn std::error::Error>> {
    let width = options.width.max(1);
    let height = options.height.max(1);
    let zoom = options.zoom.clamp(LAYOUT_MIN_ZOOM, LAYOUT_MAX_ZOOM);
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

#[cfg(not(target_arch = "wasm32"))]
fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "write".into());
    let temp_path = path.with_file_name(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_view_slugs_cover_known_routes() {
        assert_eq!(
            StartupView::from_slug("metrology"),
            Some(StartupView::Metrology)
        );
        assert_eq!(
            StartupView::from_slug("process-flow"),
            Some(StartupView::ProcessFlow)
        );
        assert_eq!(StartupView::from_slug("3d"), Some(StartupView::Layout3d));
    }

    #[test]
    fn operad_document_builds_without_layout_warnings() {
        let app = FabricadApp::new_with_options(StartupOptions::default());
        let report = app
            .audit_operad_document(UiSize::new(1024.0, 720.0))
            .expect("Operad document should build");
        assert!(report.paint_items > 0);
        let document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .expect("Operad document should build");
        assert_eq!(document.audit_layout(), Vec::new());

        for menu in AppMenu::ALL {
            let mut app = FabricadApp::new_with_options(StartupOptions::default());
            assert!(app.apply_clicked_node_name(&format!("fabricad.menu.{}", menu.slug())));
            let document = app
                .build_operad_document(UiSize::new(1024.0, 720.0))
                .expect("Operad document with open menu should build");
            assert_eq!(document.audit_layout(), Vec::new());
        }
    }

    #[test]
    fn builtin_workspace_report_still_validates_domain_data() {
        let report = validate_builtin_demo_workspace().expect("demo workspace should validate");
        assert!(report.document_shapes > 0);
        assert!(report.equipment_tools > 0);
    }

    #[test]
    fn operad_click_actions_switch_views_and_menus() {
        let mut app = FabricadApp::new_with_options(StartupOptions::default());

        assert!(app.apply_clicked_node_name("fabricad.menu.file"));
        assert_eq!(app.active_menu(), Some(AppMenu::File));

        assert!(app.apply_clicked_node_name("fabricad.nav.action.metrology"));
        assert_eq!(app.active_view(), StartupView::Metrology);
        assert_eq!(app.active_menu(), None);

        assert!(app.apply_clicked_node_name("fabricad.menu.view"));
        assert_eq!(app.active_menu(), Some(AppMenu::View));
        assert!(app.apply_clicked_node_name("fabricad.menu.item.view.process-flow"));
        assert_eq!(app.active_view(), StartupView::ProcessFlow);
    }

    #[test]
    fn restored_shell_actions_cover_tools_and_drawers() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });

        assert!(app.apply_clicked_node_name("fabricad.tool.via"));
        assert_eq!(app.active_tool(), ToolMode::Via);

        assert!(app.apply_clicked_node_name("fabricad.menu.tools"));
        assert_eq!(app.active_menu(), Some(AppMenu::Tools));
        assert!(app.apply_clicked_node_name("fabricad.menu.item.tool.route"));
        assert_eq!(app.active_tool(), ToolMode::Route);

        assert!(app.show_inspector());
        assert!(app.apply_clicked_node_name("fabricad.drawer.inspector"));
        assert!(!app.show_inspector());

        assert!(app.show_layers());
        assert!(app.apply_clicked_node_name("fabricad.drawer.layers"));
        assert!(!app.show_layers());
    }

    #[test]
    fn layout_editor_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let layer_id = app
            .workspace()
            .document
            .layers
            .keys()
            .next()
            .copied()
            .expect("demo document should have layers");
        let shape_id = app
            .workspace()
            .document
            .shapes
            .keys()
            .next()
            .copied()
            .expect("demo document should have shapes");

        assert!(
            app.apply_clicked_node_name(&format!("fabricad.viewctl.layout.layer.{}", layer_id.0))
        );
        assert_eq!(app.active_layer(), layer_id);

        let visible = app
            .workspace()
            .document
            .layers
            .get(&layer_id)
            .expect("selected layer should exist")
            .visible;
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.layout.toggle_layer.{}",
            layer_id.0
        )));
        assert_ne!(
            app.workspace()
                .document
                .layers
                .get(&layer_id)
                .expect("selected layer should exist")
                .visible,
            visible
        );

        assert!(
            app.apply_clicked_node_name(&format!("fabricad.viewctl.layout.shape.{}", shape_id.0))
        );
        assert_eq!(app.selected_layout_shape(), Some(shape_id));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.copy"));
        let shape_count = app.workspace().document.shapes.len();
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.paste"));
        assert_eq!(app.workspace().document.shapes.len(), shape_count + 1);
        assert!(app.selected_layout_shape().is_some());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.clear_selection"));
        assert_eq!(app.selected_layout_shape(), None);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.next_shape"));
        assert!(app.selected_layout_shape().is_some());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.delete"));
        assert_eq!(app.workspace().document.shapes.len(), shape_count);

        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.run_drc"));
        assert!(app.status_message().contains("DRC"));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout.connectivity"));
        assert!(app.status_message().contains("Connectivity"));

        let mut document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("layout editor controls should build");
        assert_clicked_node(
            &mut document,
            &format!("fabricad.viewctl.layout.layer.{}", layer_id.0),
        );
    }

    #[test]
    fn operad_document_nodes_are_clickable() {
        let app = FabricadApp::new_with_options(StartupOptions::default());
        let mut document = app
            .build_operad_document(UiSize::new(1024.0, 720.0))
            .expect("Operad document should build");

        assert_clicked_node(&mut document, "fabricad.menu.file");
        assert_clicked_node(&mut document, "fabricad.nav.action.metrology");
    }

    #[test]
    fn primary_view_panels_show_domain_rows() {
        for (view, scene_name) in [
            (StartupView::Workflow, "fabricad.workflow.dashboard"),
            (StartupView::MaskPrep, "fabricad.mask.overview"),
            (StartupView::LayoutDiff, "fabricad.layout_diff.overview"),
            (StartupView::FabControl, "fabricad.fab_control.overview"),
            (StartupView::Maintenance, "fabricad.maintenance.overview"),
            (StartupView::Inventory, "fabricad.inventory.overview"),
            (StartupView::Safety, "fabricad.safety.overview"),
            (StartupView::Traceability, "fabricad.traceability.overview"),
        ] {
            let app = FabricadApp::new_with_options(StartupOptions {
                view_mode: Some(view),
                ..Default::default()
            });
            let document = app
                .build_operad_document(UiSize::new(1440.0, 920.0))
                .expect("domain primary panel should build");
            assert!(
                document
                    .nodes()
                    .iter()
                    .any(|node| node.name == "fabricad.primary"),
                "view {view:?} should include a primary panel"
            );
            assert!(
                document
                    .nodes()
                    .iter()
                    .any(|node| node.name == "fabricad.primary.row.0"),
                "view {view:?} should include primary rows"
            );
            assert!(
                document.nodes().iter().any(|node| node.name == scene_name),
                "view {view:?} should include overview scene {scene_name}"
            );
            assert!(
                document
                    .nodes()
                    .iter()
                    .any(|node| node.name == "fabricad.primary.metrics.0"),
                "view {view:?} should include dashboard metrics"
            );
        }

        let layout_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Layout2d),
            ..Default::default()
        });
        let layout_document = layout_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("layout primary scene should build");
        assert!(
            layout_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.layout.preview")
        );

        let cross_section_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::CrossSection),
            ..Default::default()
        });
        let cross_section_document = cross_section_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("cross-section scene should build");
        assert!(
            cross_section_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.cross_section.preview")
        );

        let process_flow_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::ProcessFlow),
            ..Default::default()
        });
        let process_flow_document = process_flow_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("process-flow timeline should build");
        assert!(
            process_flow_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.process_flow.timeline")
        );

        let scheduler_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Scheduler),
            ..Default::default()
        });
        let scheduler_document = scheduler_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("scheduler timeline should build");
        assert!(
            scheduler_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.scheduler.timeline")
        );

        let environment_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Environment),
            ..Default::default()
        });
        let environment_document = environment_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("environment trend scene should build");
        assert!(
            environment_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.environment.trend")
        );

        let metrology_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Metrology),
            ..Default::default()
        });
        let metrology_document = metrology_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("metrology wafer-map scene should build");
        assert!(
            metrology_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.metrology.wafer_map")
        );

        let yield_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Yield),
            ..Default::default()
        });
        let yield_document = yield_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("yield wafer-map scene should build");
        assert!(
            yield_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.yield.wafer_map")
        );

        let experiment_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Experiment),
            ..Default::default()
        });
        let experiment_document = experiment_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("experiment matrix scene should build");
        assert!(
            experiment_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.experiment.matrix")
        );

        let process_control_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::ProcessControl),
            ..Default::default()
        });
        let process_control_document = process_control_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("process-control trend scene should build");
        assert!(
            process_control_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.process_control.trend")
        );

        let spc_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::SpcFdc),
            ..Default::default()
        });
        let spc_document = spc_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("SPC/FDC chart scene should build");
        assert!(
            spc_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.spc.chart")
        );

        let notebook_app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Notebook),
            ..Default::default()
        });
        let notebook_document = notebook_app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("notebook selected-entry preview should build");
        assert!(
            notebook_document
                .nodes()
                .iter()
                .any(|node| node.name == "fabricad.notebook.body")
        );
    }

    #[test]
    fn operad_document_scales_for_hidpi_windows() {
        let app = FabricadApp::new_with_options(StartupOptions::default());
        let document = app
            .build_operad_document_scaled(UiSize::new(2048.0, 1440.0), UiScale::new(2.0))
            .expect("scaled Operad document should build");

        assert_eq!(document.audit_layout(), Vec::new());
        let menu = document
            .nodes()
            .iter()
            .find(|node| node.name == "fabricad.menu.file")
            .expect("file menu should exist");
        assert!(menu.layout.rect.height >= 50.0, "{:?}", menu.layout.rect);
        let nav = document
            .nodes()
            .iter()
            .find(|node| node.name == "fabricad.nav")
            .expect("nav should exist");
        assert!(nav.layout.rect.width >= 400.0, "{:?}", nav.layout.rect);
    }

    #[test]
    fn metrology_view_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Metrology),
            ..Default::default()
        });

        assert!(app.apply_clicked_node_name("fabricad.viewctl.metrology.mode.defects"));
        assert_eq!(app.metrology_map_mode(), MetrologyMapMode::DefectReview);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.metrology.kind.thickness"));
        assert_eq!(app.metrology_kind(), MeasurementKind::ThicknessNm);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.metrology.failed_only"));
        assert!(app.metrology_failed_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.metrology.next_attention"));
        assert!(app.selected_die().is_some());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.metrology.clear_die"));
        assert_eq!(app.selected_die(), None);

        let mut document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("metrology controls should build");
        assert_clicked_node(&mut document, "fabricad.viewctl.metrology.mode.defects");
        assert_clicked_node(&mut document, "fabricad.viewctl.metrology.kind.thickness");
    }

    #[test]
    fn yield_and_experiment_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Yield),
            ..Default::default()
        });
        let lot_id = app
            .workspace
            .yield_analysis
            .lots
            .first()
            .expect("demo yield lot should exist")
            .id
            .clone();
        let wafer_id = app
            .workspace
            .yield_analysis
            .wafer_ids_for_lot(&lot_id)
            .first()
            .expect("demo yield wafer should exist")
            .clone();

        assert!(app.apply_clicked_node_name("fabricad.viewctl.yield.filter.failing"));
        assert_eq!(app.yield_map_filter(), YieldMapFilter::Failing);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.yield.attention"));
        assert!(app.show_only_attention_wafers());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.yield.excursions"));
        assert!(app.show_only_excursions());
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.yield.lot.{lot_id}")));
        assert_eq!(app.selected_yield_lot(), Some(lot_id.as_str()));
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.yield.wafer.{wafer_id}")));
        assert_eq!(app.selected_yield_wafer(), Some(wafer_id.as_str()));

        let mut yield_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("yield controls should build");
        assert_clicked_node(&mut yield_document, "fabricad.viewctl.yield.filter.failing");

        app.set_active_view(StartupView::Experiment);
        assert!(!app.experiment_show_missing_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.experiment.pending_only"));
        assert!(app.experiment_show_missing_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.experiment.next_pending"));
        assert!(app.status_message().contains("DOE: queued"));

        let response_id = app
            .workspace
            .experiment_plan
            .responses
            .first()
            .expect("demo DOE response should exist")
            .id
            .clone();
        let run_id = app
            .workspace
            .experiment_plan
            .runs
            .first()
            .expect("demo DOE run should exist")
            .id
            .clone();
        let lot_id = app
            .workspace
            .experiment_plan
            .runs
            .first()
            .expect("demo DOE run should exist")
            .assignment
            .lot_id
            .to_string();
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.experiment.response.{response_id}"
        )));
        assert_eq!(
            app.selected_experiment_response.as_ref(),
            Some(&response_id)
        );
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.experiment.run.{run_id}")));
        assert_eq!(app.selected_experiment_run.as_ref(), Some(&run_id));
        assert!(
            app.apply_clicked_node_name(
                "fabricad.viewctl.experiment.filter.needs-selected-response"
            )
        );
        assert_eq!(
            app.experiment_run_filter,
            ExperimentRunFilter::NeedsSelectedResponse
        );
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.experiment.lot.{lot_id}")));
        assert_eq!(app.experiment_lot_filter.as_deref(), Some(lot_id.as_str()));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.experiment.use_target"));
        assert!(app.status_message().contains("DOE: loaded target"));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.experiment.capture_next_demo"));
        assert!(app.status_message().contains("DOE: captured demo"));

        let mut experiment_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("experiment controls should build");
        assert_clicked_node(
            &mut experiment_document,
            "fabricad.viewctl.experiment.pending_only",
        );
        assert_clicked_node(
            &mut experiment_document,
            &format!("fabricad.viewctl.experiment.response.{response_id}"),
        );
    }

    #[test]
    fn workflow_controls_switch_focus_and_open_views() {
        let mut app = FabricadApp::new_with_options(StartupOptions::default());
        let initial_focus = app.workflow_focus_lot().map(str::to_string);
        let target_lot = workflow_lot_ids(app.workspace())
            .into_iter()
            .find(|lot_id| Some(lot_id.as_str()) != app.workflow_focus_lot())
            .or(initial_focus)
            .expect("demo workflow should have a lot");

        assert!(
            app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.workflow.focus_lot.{target_lot}"
            ))
        );
        assert_eq!(app.workflow_focus_lot(), Some(target_lot.as_str()));

        assert!(app.apply_clicked_node_name("fabricad.viewctl.workflow.open.fab-control"));
        assert_eq!(app.active_view(), StartupView::FabControl);
        for (action, expected_view) in [
            (
                "fabricad.viewctl.workflow.open.maintenance",
                StartupView::Maintenance,
            ),
            (
                "fabricad.viewctl.workflow.open.environment",
                StartupView::Environment,
            ),
            ("fabricad.viewctl.workflow.open.safety", StartupView::Safety),
            (
                "fabricad.viewctl.workflow.open.metrology",
                StartupView::Metrology,
            ),
            (
                "fabricad.viewctl.workflow.open.traceability",
                StartupView::Traceability,
            ),
            (
                "fabricad.viewctl.workflow.open.notebook",
                StartupView::Notebook,
            ),
        ] {
            app.set_active_view(StartupView::Workflow);
            assert!(app.apply_clicked_node_name(action), "{action}");
            assert_eq!(app.active_view(), expected_view);
        }

        app.set_active_view(StartupView::Workflow);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.workflow.load_demo"));
        assert!(app.status_message().contains("demo workspace loaded"));
        let mut document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("workflow document should build");
        assert_clicked_node(&mut document, "fabricad.viewctl.workflow.open.traceability");
        assert_clicked_node(&mut document, "fabricad.viewctl.workflow.load_demo");
    }

    #[test]
    fn fab_control_actions_select_and_command_equipment() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::FabControl),
            ..Default::default()
        });
        let offline_tool = app
            .workspace()
            .equipment
            .tools()
            .find(|tool| tool.state == EquipmentToolState::Offline)
            .expect("demo fab should include an offline tool")
            .id
            .clone();

        assert!(
            app.apply_clicked_node_name(&format!("fabricad.viewctl.fab.select.{offline_tool}"))
        );
        assert_eq!(app.selected_equipment_tool(), Some(&offline_tool));
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.fab.command.online|{offline_tool}"
        )));
        assert_eq!(
            app.workspace()
                .equipment
                .tool(&offline_tool)
                .expect("tool should remain present")
                .state,
            EquipmentToolState::OnlineIdle
        );

        let recipe_id = app
            .workspace()
            .equipment
            .tool(&offline_tool)
            .expect("tool should remain present")
            .available_recipes
            .keys()
            .next()
            .expect("tool should have recipes")
            .clone();
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.fab.recipe.{offline_tool}|{recipe_id}"
        )));
        assert_eq!(
            app.workspace()
                .equipment
                .tool(&offline_tool)
                .expect("tool should remain present")
                .state,
            EquipmentToolState::RecipeLoaded
        );

        let mut document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("fab control controls should build");
        assert_clicked_node(
            &mut document,
            &format!("fabricad.viewctl.fab.command.start|{offline_tool}"),
        );
    }

    #[test]
    fn maintenance_and_environment_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Maintenance),
            ..Default::default()
        });
        let maintenance_tool = app
            .workspace()
            .maintenance
            .tools
            .first()
            .expect("demo maintenance tool should exist")
            .tool_id
            .clone();

        assert!(app.apply_clicked_node_name("fabricad.viewctl.maintenance.filter.calibration"));
        assert_eq!(
            app.maintenance_work_filter(),
            MaintenanceWorkFilter::Calibration
        );
        assert!(app.apply_clicked_node_name("fabricad.viewctl.maintenance.history.all"));
        assert_eq!(
            app.maintenance_history_filter(),
            MaintenanceHistoryFilter::AllTools
        );
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.maintenance.tool.{maintenance_tool}"
        )));
        assert_eq!(app.selected_maintenance_tool(), Some(&maintenance_tool));
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.maintenance.status.schedule|{maintenance_tool}"
        )));
        assert!(app.status_message().contains("Maintenance scheduling"));

        let mut maintenance_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("maintenance controls should build");
        assert_clicked_node(
            &mut maintenance_document,
            "fabricad.viewctl.maintenance.filter.calibration",
        );

        app.set_active_view(StartupView::Environment);
        let sensor_id = app
            .workspace()
            .environment
            .sensors
            .first()
            .expect("demo environment sensor should exist")
            .id
            .clone();
        let zone = app
            .workspace()
            .environment
            .sensors
            .first()
            .expect("demo environment sensor should exist")
            .zone
            .clone();
        assert!(
            app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.environment.sensor.{sensor_id}"
            ))
        );
        assert_eq!(app.selected_environment_sensor(), Some(sensor_id.as_str()));
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.environment.zone.{zone}")));
        assert!(
            app.selected_environment_sensor().is_some(),
            "zone selection should choose a sensor"
        );

        let mut environment_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("environment controls should build");
        assert_clicked_node(
            &mut environment_document,
            &format!("fabricad.viewctl.environment.sensor.{sensor_id}"),
        );
    }

    #[test]
    fn inventory_scheduler_and_safety_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Inventory),
            ..Default::default()
        });
        let lot_id = app
            .workspace()
            .inventory
            .sorted_lots()
            .first()
            .expect("demo inventory lot should exist")
            .id
            .clone();
        assert!(app.apply_clicked_node_name("fabricad.viewctl.inventory.filter.low-stock"));
        assert_eq!(app.inventory_filter(), InventoryQuickFilter::LowStock);
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.inventory.lot.{lot_id}")));
        assert_eq!(app.selected_inventory_lot(), Some(&lot_id));

        let mut inventory_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("inventory controls should build");
        assert_clicked_node(
            &mut inventory_document,
            "fabricad.viewctl.inventory.filter.low-stock",
        );

        app.set_active_view(StartupView::Scheduler);
        let scheduler_tool = app
            .workspace()
            .scheduler
            .tools
            .first()
            .expect("demo dispatch tool should exist")
            .id
            .clone();
        assert!(app.apply_clicked_node_name("fabricad.viewctl.scheduler.policy.due-date"));
        assert_eq!(app.scheduler_policy(), DispatchPolicy::DueDateThenPriority);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.scheduler.priority.3"));
        assert_eq!(app.scheduler_min_priority(), 3);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.scheduler.toggle_conflicts"));
        assert!(app.scheduler_conflicts_only());
        assert!(
            app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.scheduler.tool.{scheduler_tool}"
            ))
        );
        assert_eq!(app.selected_scheduler_tool(), Some(&scheduler_tool));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.scheduler.reset"));
        assert_eq!(app.scheduler_min_priority(), 0);
        assert!(!app.scheduler_conflicts_only());

        let mut scheduler_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("scheduler controls should build");
        assert_clicked_node(
            &mut scheduler_document,
            "fabricad.viewctl.scheduler.policy.due-date",
        );

        app.set_active_view(StartupView::Safety);
        let lockout = app
            .workspace()
            .safety
            .evaluate_lockouts()
            .first()
            .expect("demo safety lockout should exist")
            .clone();
        assert!(
            app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.safety.tool.{}",
                lockout.tool_id
            ))
        );
        assert_eq!(app.selected_safety_tool(), Some(lockout.tool_id.as_str()));
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.safety.ack.lockout.{}",
            lockout.tool_id
        )));
        assert!(app.acknowledged_count() > 0);
        if let Some(sensor) = app.workspace().safety.active_conditions().first() {
            let sensor_id = sensor.id.to_string();
            assert!(app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.safety.ack.condition.{sensor_id}"
            )));
        }

        let mut safety_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("safety controls should build");
        assert_clicked_node(
            &mut safety_document,
            &format!("fabricad.viewctl.safety.tool.{}", lockout.tool_id),
        );
    }

    #[test]
    fn mask_prep_and_layout_diff_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::MaskPrep),
            ..Default::default()
        });
        let lot_id = app
            .workspace()
            .mes
            .lots
            .keys()
            .next()
            .expect("demo MES lot should exist")
            .clone();
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.mask.lot.{lot_id}")));
        assert_eq!(app.mask_source_lot(), Some(&lot_id));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.mask.severity.errors"));
        assert_eq!(
            app.mask_issue_severity_filter(),
            MaskIssueSeverityFilter::Errors
        );
        assert!(app.apply_clicked_node_name("fabricad.viewctl.mask.group.layer"));
        assert_eq!(app.mask_issue_grouping(), MaskIssueGrouping::Layer);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.mask.rebuild"));
        assert!(app.status_message().contains("Reticle prep rebuilt"));

        let mut mask_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("reticle prep controls should build");
        assert_clicked_node(&mut mask_document, "fabricad.viewctl.mask.severity.errors");

        app.set_active_view(StartupView::LayoutDiff);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.baseline.empty"));
        assert_eq!(app.layout_diff_baseline(), LayoutDiffSource::Empty);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.candidate.current"));
        assert_eq!(app.layout_diff_candidate(), LayoutDiffSource::Current);
        assert!(app.layout_diff_changed_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.toggle_changed_only"));
        assert!(!app.layout_diff_changed_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.filter.added"));
        assert_eq!(app.layout_diff_change_filter(), LayoutChangeFilter::Added);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.page_size.25"));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.layout_diff.review.approved"));
        assert_eq!(
            app.layout_diff_review_state(),
            LayoutReviewDisposition::Approved
        );

        let mut diff_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("layout diff controls should build");
        assert_clicked_node(
            &mut diff_document,
            "fabricad.viewctl.layout_diff.baseline.empty",
        );
    }

    #[test]
    fn traceability_and_notebook_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::Traceability),
            ..Default::default()
        });
        let lot_id = app
            .workspace()
            .genealogy
            .lot_ids()
            .first()
            .expect("demo trace lot should exist")
            .clone();
        let wafer = app
            .workspace()
            .genealogy
            .wafer_refs_for_lot(&lot_id)
            .first()
            .expect("demo trace wafer should exist")
            .clone();
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.trace.lot.{lot_id}")));
        assert_eq!(app.selected_trace_lot(), Some(&lot_id));
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.trace.wafer.{}|{}",
            wafer.lot_id, wafer.wafer_id
        )));
        assert_eq!(app.selected_trace_wafer(), Some(&wafer.wafer_id));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.trace.impact.material"));
        assert_eq!(app.trace_impact_mode(), TraceImpactMode::Material);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.trace.toggle_related"));
        assert!(app.trace_related_only());
        let process_sequence = app
            .workspace()
            .genealogy
            .process_history
            .first()
            .expect("demo trace process history should exist")
            .sequence;
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.trace.detail.process.{process_sequence}"
        )));
        assert_eq!(
            app.selected_trace_detail,
            Some(TraceSelection::Process(process_sequence))
        );
        let material_sequence = app
            .workspace()
            .genealogy
            .material_uses
            .first()
            .expect("demo trace material use should exist")
            .sequence;
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.trace.detail.material.{material_sequence}"
        )));
        assert_eq!(
            app.selected_trace_detail,
            Some(TraceSelection::MaterialUse(material_sequence))
        );
        let event_sequence = app
            .workspace()
            .genealogy
            .events
            .first()
            .expect("demo trace event should exist")
            .sequence;
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.trace.detail.event.{event_sequence}"
        )));
        assert_eq!(
            app.selected_trace_detail,
            Some(TraceSelection::Event(event_sequence))
        );

        let mut trace_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("traceability controls should build");
        assert_clicked_node(
            &mut trace_document,
            "fabricad.viewctl.trace.impact.material",
        );
        assert_clicked_node(
            &mut trace_document,
            &format!("fabricad.viewctl.trace.detail.event.{event_sequence}"),
        );

        app.set_active_view(StartupView::Notebook);
        let entry_id = app
            .workspace()
            .lab_notebook
            .entries
            .first()
            .expect("demo notebook entry should exist")
            .id
            .clone();
        let tag = app
            .workspace()
            .lab_notebook
            .tags()
            .first()
            .expect("demo notebook tag should exist")
            .clone();
        assert!(
            app.apply_clicked_node_name(&format!("fabricad.viewctl.notebook.entry.{entry_id}"))
        );
        assert_eq!(app.selected_notebook_entry(), Some(&entry_id));
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.notebook.tag.{tag}")));
        assert_eq!(app.notebook_tag_filter(), Some(tag.as_str()));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.notebook.link.lot"));
        assert_eq!(app.notebook_link_kind_filter(), Some(NotebookLinkKind::Lot));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.notebook.toggle_followups"));
        assert!(app.notebook_followups_only());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.notebook.edit"));
        assert!(!app.notebook_preview_mode());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.notebook.clear_filters"));
        assert_eq!(app.notebook_tag_filter(), None);
        assert_eq!(app.notebook_link_kind_filter(), None);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.notebook.entry_action.handoff"));
        assert!(
            app.workspace()
                .lab_notebook
                .entry(&entry_id)
                .expect("selected notebook entry should exist")
                .tags
                .iter()
                .any(|candidate| candidate == "handoff")
        );
        let (link_kind, link_value) = notebook_entry_link_focus_tokens(
            app.workspace()
                .lab_notebook
                .entry(&entry_id)
                .expect("selected notebook entry should exist"),
        )
        .first()
        .cloned()
        .expect("demo notebook entry should have a focusable link");
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.notebook.focus_link.{}:{}",
            notebook_link_kind_slug(link_kind),
            link_value
        )));
        assert_eq!(app.notebook_link_kind_filter(), Some(link_kind));

        let mut notebook_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("notebook controls should build");
        assert_clicked_node(&mut notebook_document, "fabricad.viewctl.notebook.preview");
        assert_clicked_node(
            &mut notebook_document,
            "fabricad.viewctl.notebook.entry_action.handoff",
        );
    }

    #[test]
    fn process_flow_and_cross_section_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::ProcessFlow),
            ..Default::default()
        });
        let node_id = app
            .workspace()
            .process_flow
            .route
            .nodes
            .first()
            .expect("demo process node should exist")
            .id
            .clone();

        assert!(app.apply_clicked_node_name("fabricad.viewctl.process_flow.filter.recipes"));
        assert_eq!(
            app.process_flow_filter(),
            ProcessFlowNodeFilter::RecipeSteps
        );
        assert!(app.apply_clicked_node_name("fabricad.viewctl.process_flow.toggle_errors"));
        assert!(app.process_flow_errors_only());
        assert!(
            app.apply_clicked_node_name(&format!("fabricad.viewctl.process_flow.node.{node_id}"))
        );
        assert_eq!(app.selected_process_node(), Some(&node_id));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.process_flow.validate"));
        assert!(app.status_message().contains("Process flow"));

        let mut process_flow_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("process-flow controls should build");
        assert_clicked_node(
            &mut process_flow_document,
            "fabricad.viewctl.process_flow.filter.recipes",
        );

        app.set_active_view(StartupView::CrossSection);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.cross_section.step.1"));
        assert_eq!(app.cross_section_step(), 1);
        assert!(app.cross_section_show_mask());
        assert!(app.apply_clicked_node_name("fabricad.viewctl.cross_section.toggle_mask"));
        assert!(!app.cross_section_show_mask());
        let material_id = app
            .workspace()
            .cross_section
            .materials
            .first()
            .expect("demo cross-section material should exist")
            .id
            .clone();
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.cross_section.material.{}",
            material_id.as_str()
        )));

        let mut cross_section_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("cross-section controls should build");
        assert_clicked_node(
            &mut cross_section_document,
            "fabricad.viewctl.cross_section.step.1",
        );
    }

    #[test]
    fn process_control_and_spc_controls_update_state() {
        let mut app = FabricadApp::new_with_options(StartupOptions {
            view_mode: Some(StartupView::ProcessControl),
            ..Default::default()
        });
        let target_loop = app
            .workspace()
            .process_control
            .loops
            .iter()
            .take(6)
            .last()
            .expect("demo process-control loop should exist")
            .id
            .clone();
        assert!(app.apply_clicked_node_name(&format!(
            "fabricad.viewctl.process_control.loop.{target_loop}"
        )));
        assert_eq!(app.selected_control_loop(), Some(&target_loop));

        if let Some(action_id) = app
            .workspace()
            .process_control
            .actions_for_loop(&target_loop)
            .first()
            .map(|action| action.id.clone())
        {
            assert!(app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.process_control.action.{action_id}"
            )));
            assert_eq!(app.selected_control_action(), Some(&action_id));
        }

        if let Some(action_id) = app
            .workspace()
            .process_control
            .actions
            .iter()
            .find(|action| action.state == ControlActionState::Proposed)
            .map(|action| action.id.clone())
        {
            assert!(app.apply_clicked_node_name(&format!(
                "fabricad.viewctl.process_control.transition.{action_id}|approve"
            )));
            let state = app
                .workspace()
                .process_control
                .actions
                .iter()
                .find(|action| action.id == action_id)
                .expect("transitioned action should remain present")
                .state;
            assert_eq!(state, ControlActionState::Approved);
        }

        let mut process_control_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("process-control controls should build");
        assert_clicked_node(
            &mut process_control_document,
            &format!("fabricad.viewctl.process_control.loop.{target_loop}"),
        );

        app.set_active_view(StartupView::SpcFdc);
        let monitor = spc_fdc_monitor(app.workspace());
        let chart_id = monitor
            .charts
            .last()
            .expect("demo SPC chart should exist")
            .id
            .as_str()
            .to_string();
        let trace_id = monitor
            .traces
            .last()
            .expect("demo FDC trace should exist")
            .id
            .clone();
        assert!(app.apply_clicked_node_name("fabricad.viewctl.spc.severity.critical"));
        assert_eq!(app.spc_severity_filter(), SpcSeverityFilter::Critical);
        assert!(app.apply_clicked_node_name("fabricad.viewctl.spc.source.fdc"));
        assert_eq!(app.spc_source_filter(), SpcSourceFilter::Fdc);
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.spc.chart.{chart_id}")));
        assert_eq!(app.selected_spc_chart(), Some(chart_id.as_str()));
        assert!(app.apply_clicked_node_name(&format!("fabricad.viewctl.spc.trace.{trace_id}")));
        assert_eq!(app.selected_fdc_trace(), Some(trace_id.as_str()));
        assert!(app.apply_clicked_node_name("fabricad.viewctl.spc.clear_context"));

        let mut spc_document = app
            .build_operad_document(UiSize::new(1440.0, 920.0))
            .expect("SPC/FDC controls should build");
        assert_clicked_node(&mut spc_document, "fabricad.viewctl.spc.severity.critical");
    }

    #[test]
    fn restored_view_controls_build_at_common_sizes() {
        for view in [
            StartupView::Workflow,
            StartupView::Layout2d,
            StartupView::Layout3d,
            StartupView::MaskPrep,
            StartupView::LayoutDiff,
            StartupView::FabControl,
            StartupView::Maintenance,
            StartupView::Environment,
            StartupView::Inventory,
            StartupView::Scheduler,
            StartupView::Safety,
            StartupView::Traceability,
            StartupView::ProcessFlow,
            StartupView::ProcessControl,
            StartupView::SpcFdc,
            StartupView::CrossSection,
            StartupView::Metrology,
            StartupView::Yield,
            StartupView::Experiment,
            StartupView::Notebook,
        ] {
            for viewport in [UiSize::new(1024.0, 720.0), UiSize::new(2048.0, 1440.0)] {
                let app = FabricadApp::new_with_options(StartupOptions {
                    view_mode: Some(view),
                    ..Default::default()
                });
                let document = app
                    .build_operad_document(viewport)
                    .expect("view control document should build");
                assert_eq!(
                    document.audit_layout(),
                    Vec::new(),
                    "view {view:?} viewport {viewport:?}"
                );
            }
        }
    }

    fn assert_clicked_node(document: &mut UiDocument, name: &str) {
        let (node_id, point) = document
            .nodes()
            .iter()
            .enumerate()
            .find(|(_, node)| node.name == name)
            .map(|(index, node)| {
                (
                    operad::UiNodeId(index),
                    UiPoint::new(
                        node.layout.rect.x + node.layout.rect.width * 0.5,
                        node.layout.rect.y + node.layout.rect.height * 0.5,
                    ),
                )
            })
            .unwrap_or_else(|| panic!("expected node {name} to exist"));

        let down = document.handle_input(operad::UiInputEvent::PointerDown(point));
        assert_eq!(down.pressed, Some(node_id));
        let up = document.handle_input(operad::UiInputEvent::PointerUp(point));
        assert_eq!(up.clicked, Some(node_id));
    }
}
