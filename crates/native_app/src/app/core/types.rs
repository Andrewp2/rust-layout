#![allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Benchmark3dOptions {
    pub frames: usize,
    pub warmup_frames: usize,
    pub width: u32,
    pub height: u32,
}

impl Default for Benchmark3dOptions {
    fn default() -> Self {
        Self {
            frames: 180,
            warmup_frames: 24,
            width: 1440,
            height: 920,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Benchmark3dReport {
    pub count: usize,
    pub frames: usize,
    pub warmup_frames: usize,
    pub viewport: UiSize,
    pub adapter: String,
    pub app_init_ms: f64,
    pub ui_build_ms: f64,
    pub ui_paint_items: usize,
    pub batch_build_ms: f64,
    pub batch_bytes: usize,
    pub mesh_triangles: usize,
    pub rect_slabs: usize,
    pub guide_segments: usize,
    pub first_upload_ms: f64,
    pub first_upload_bytes: usize,
    pub uniform_avg_ms: f64,
    pub cached_upload_avg_ms: f64,
    pub encode_avg_ms: f64,
    pub submit_wait_avg_ms: f64,
    pub submit_wait_p50_ms: f64,
    pub submit_wait_p95_ms: f64,
}

impl Benchmark3dReport {
    pub fn summary(&self) -> String {
        format!(
            "3d benchmark count={} viewport={}x{} adapter=\"{}\" frames={} warmup={} app_init_ms={:.3} ui_build_ms={:.3} ui_paint_items={} batch_build_ms={:.3} batch_mb={:.2} mesh_triangles={} rect_slabs={} guide_segments={} first_upload_ms={:.3} first_upload_mb={:.2} uniform_avg_ms={:.3} cached_upload_avg_ms={:.3} encode_avg_ms={:.3} submit_wait_avg_ms={:.3} submit_wait_p50_ms={:.3} submit_wait_p95_ms={:.3}",
            self.count,
            self.viewport.width.round() as u32,
            self.viewport.height.round() as u32,
            self.adapter,
            self.frames,
            self.warmup_frames,
            self.app_init_ms,
            self.ui_build_ms,
            self.ui_paint_items,
            self.batch_build_ms,
            self.batch_bytes as f64 / (1024.0 * 1024.0),
            self.mesh_triangles,
            self.rect_slabs,
            self.guide_segments,
            self.first_upload_ms,
            self.first_upload_bytes as f64 / (1024.0 * 1024.0),
            self.uniform_avg_ms,
            self.cached_upload_avg_ms,
            self.encode_avg_ms,
            self.submit_wait_avg_ms,
            self.submit_wait_p50_ms,
            self.submit_wait_p95_ms
        )
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
    More,
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
            Self::More => "More",
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
            Self::More => "more",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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
            "more" => Some(Self::More),
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
    Label,
    Measure,
    Route,
    Trace,
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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
pub enum ControlLoopFilter {
    All,
    Active,
    Attention,
}

impl ControlLoopFilter {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Active, Self::Attention];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "All loops",
            Self::Active => "Active",
            Self::Attention => "Attention",
        }
    }

    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Attention => "attention",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "active" => Some(Self::Active),
            "attention" => Some(Self::Attention),
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "critical" => Some(Self::Critical),
            "warning" => Some(Self::Warning),
            "advisory" => Some(Self::Advisory),
            _ => None,
        }
    }

    pub(crate) fn matches(self, severity: MonitorSeverity) -> bool {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "spc" => Some(Self::Spc),
            "fdc" => Some(Self::Fdc),
            "alarm" => Some(Self::Alarm),
            _ => None,
        }
    }

    pub(crate) fn matches(self, source: FindingSource) -> bool {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "errors" => Some(Self::Errors),
            "warnings" => Some(Self::Warnings),
            _ => None,
        }
    }

    pub(crate) fn matches(self, severity: MaskIssueSeverity) -> bool {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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
            Self::Demo => "Sample",
            Self::Hierarchy => "Hierarchy",
            Self::Empty => "Empty",
        }
    }

    pub const fn detail_label(self) -> &'static str {
        match self {
            Self::Current => "Current workspace",
            Self::Demo => "Sample layout",
            Self::Hierarchy => "Hierarchy sample",
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "current" => Some(Self::Current),
            "demo" => Some(Self::Demo),
            "hierarchy" => Some(Self::Hierarchy),
            "empty" => Some(Self::Empty),
            _ => None,
        }
    }

    pub(crate) fn document(self, current: &Document) -> Document {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "added" => Some(Self::Added),
            "removed" => Some(Self::Removed),
            "modified" => Some(Self::Modified),
            _ => None,
        }
    }

    pub(crate) fn matches(self, kind: ShapeChangeKind) -> bool {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "tool-run" => Some(Self::ToolRun),
            "step" => Some(Self::Step),
            "material" => Some(Self::Material),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TraceSelection {
    Lot(LotId),
    Wafer(WaferRef),
    Process(u64),
    MaterialUse(u64),
    Event(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExperimentRunFilter {
    All,
    NeedsAnyResponse,
    NeedsSelectedResponse,
    InProgress,
    Complete,
    OutOfSpec,
    Blocked,
}

impl ExperimentRunFilter {
    pub(crate) const ALL: [Self; 7] = [
        Self::All,
        Self::NeedsAnyResponse,
        Self::NeedsSelectedResponse,
        Self::InProgress,
        Self::Complete,
        Self::OutOfSpec,
        Self::Blocked,
    ];

    pub(crate) const fn label(self) -> &'static str {
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

    pub(crate) const fn short_label(self) -> &'static str {
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

    pub(crate) const fn slug(self) -> &'static str {
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

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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

    pub(crate) fn matches(self, row: &ExperimentRunMatrixRow) -> bool {
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
pub(crate) struct ExperimentRunMatrixRow {
    pub(crate) run_id: String,
    pub(crate) selected: bool,
    pub(crate) run_order: u32,
    pub(crate) lot_id: String,
    pub(crate) wafer_id: String,
    pub(crate) block: String,
    pub(crate) factor_values: Vec<String>,
    pub(crate) status: String,
    pub(crate) status_kind: ExperimentRunStatus,
    pub(crate) capture_summary: String,
    pub(crate) missing_count: usize,
    pub(crate) needs_selected_response: bool,
    pub(crate) has_out_of_spec: bool,
    pub(crate) primary_value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotebookEntryAction {
    AddFollowUpPlan,
    InsertMetrologyReview,
    RequestImageEvidence,
    TagHandoff,
}

impl NotebookEntryAction {
    pub(crate) const ALL: [Self; 4] = [
        Self::AddFollowUpPlan,
        Self::InsertMetrologyReview,
        Self::RequestImageEvidence,
        Self::TagHandoff,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::AddFollowUpPlan => "Add follow-up",
            Self::InsertMetrologyReview => "Metrology review",
            Self::RequestImageEvidence => "Image evidence",
            Self::TagHandoff => "Tag handoff",
        }
    }

    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::AddFollowUpPlan => "follow-up",
            Self::InsertMetrologyReview => "metrology-review",
            Self::RequestImageEvidence => "image-evidence",
            Self::TagHandoff => "handoff",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
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
    pub const ALL: [Self; 9] = [
        Self::Select,
        Self::Rect,
        Self::Polygon,
        Self::Path,
        Self::Via,
        Self::Label,
        Self::Measure,
        Self::Route,
        Self::Trace,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Rect => "Rect",
            Self::Polygon => "Poly",
            Self::Path => "Path",
            Self::Via => "Via",
            Self::Label => "Label",
            Self::Measure => "Measure",
            Self::Route => "Route",
            Self::Trace => "Trace",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Rect => "rect",
            Self::Polygon => "polygon",
            Self::Path => "path",
            Self::Via => "via",
            Self::Label => "label",
            Self::Measure => "measure",
            Self::Route => "route",
            Self::Trace => "trace",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "select" => Some(Self::Select),
            "rect" => Some(Self::Rect),
            "polygon" | "poly" => Some(Self::Polygon),
            "path" => Some(Self::Path),
            "via" => Some(Self::Via),
            "label" | "text" => Some(Self::Label),
            "measure" => Some(Self::Measure),
            "route" => Some(Self::Route),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
            Self::Layout3d => "3D Layout View",
            Self::MaskPrep => "Mask / Reticle Prep",
            Self::LayoutDiff => "Layout Diff Review",
            Self::FabControl => "Fab Control Room",
            Self::Inventory => "Inventory Tracker",
            Self::Maintenance => "Maintenance and Calibration",
            Self::Environment => "Cleanroom Environment",
            Self::Scheduler => "Scheduler / Dispatch",
            Self::Safety => "Safety and Interlock Dashboard",
            Self::Traceability => "Lot Traceability",
            Self::Metrology => "Metrology Wafer Map",
            Self::Yield => "Yield Dashboard",
            Self::SpcFdc => "SPC / FDC Monitor",
            Self::ProcessFlow => "Process-flow Designer",
            Self::ProcessControl => "Run-to-Run Control",
            Self::CrossSection => "Process Cross-Section",
            Self::Experiment => "DOE Planner",
            Self::Notebook => "Lab Notebook",
        }
    }

    pub const fn nav_label(self) -> &'static str {
        match self {
            Self::Workflow => "Workflow",
            Self::Layout2d => "Mask layout",
            Self::Layout3d => "3D viewport",
            Self::MaskPrep => "Reticle prep",
            Self::LayoutDiff => "Layout diff",
            Self::FabControl => "Equipment",
            Self::Inventory => "Inventory",
            Self::Maintenance => "Maintenance",
            Self::Environment => "Environment",
            Self::Scheduler => "Dispatch",
            Self::Safety => "Safety",
            Self::Traceability => "Traceability",
            Self::Metrology => "Metrology",
            Self::Yield => "Yield",
            Self::SpcFdc => "SPC / FDC",
            Self::ProcessFlow => "Process flow",
            Self::ProcessControl => "R2R control",
            Self::CrossSection => "Cross-section",
            Self::Experiment => "DOE",
            Self::Notebook => "Notebook",
        }
    }

    pub(crate) const fn group(self) -> ModuleGroup {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModuleGroup {
    Design,
    Operations,
    Analysis,
    Engineering,
}

impl ModuleGroup {
    pub(crate) const ALL: [Self; 4] = [
        Self::Design,
        Self::Operations,
        Self::Analysis,
        Self::Engineering,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Design => "Design",
            Self::Operations => "Operations",
            Self::Analysis => "Analysis",
            Self::Engineering => "Engineering",
        }
    }

    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Design => "design",
            Self::Operations => "operations",
            Self::Analysis => "analysis",
            Self::Engineering => "engineering",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "design" => Some(Self::Design),
            "operations" => Some(Self::Operations),
            "analysis" => Some(Self::Analysis),
            "engineering" => Some(Self::Engineering),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitDisplay {
    Auto,
    Nanometers,
    Microns,
    Dbu,
}

impl UnitDisplay {
    pub(crate) const ALL: [Self; 4] = [Self::Auto, Self::Nanometers, Self::Microns, Self::Dbu];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Nanometers => "Nanometers",
            Self::Microns => "Microns",
            Self::Dbu => "DBU",
        }
    }

    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Nanometers => "nanometers",
            Self::Microns => "microns",
            Self::Dbu => "dbu",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "nanometers" => Some(Self::Nanometers),
            "microns" => Some(Self::Microns),
            "dbu" => Some(Self::Dbu),
            _ => None,
        }
    }
}

pub(crate) fn default_nav_rail_views() -> BTreeSet<StartupView> {
    StartupView::ALL.into_iter().collect()
}

pub(crate) fn nav_rail_views_from_option_slugs(slugs: &[String]) -> BTreeSet<StartupView> {
    slugs
        .iter()
        .filter_map(|slug| StartupView::from_slug(slug))
        .collect()
}

pub(crate) fn option_enabled(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

pub(crate) fn option_scale_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "100" => Some(1.0),
        "125" => Some(1.25),
        "150" => Some(1.5),
        "200" => Some(2.0),
        _ => None,
    }
}

pub(crate) fn option_zoom_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "tiny" => Some(0.005),
        "default" => Some(0.02),
        "close" => Some(0.1),
        _ => None,
    }
}

pub(crate) fn option_speed_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "slow" => Some(1_000.0),
        "default" => Some(4_000.0),
        "fast" => Some(12_000.0),
        _ => None,
    }
}

pub(crate) fn option_fov_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "45" => Some(45.0),
        "58" => Some(58.0),
        "75" => Some(75.0),
        _ => None,
    }
}

pub(crate) fn option_sensitivity_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "low" => Some(0.0015),
        "default" => Some(0.003),
        "high" => Some(0.006),
        _ => None,
    }
}

pub(crate) fn option_fast_multiplier_from_slug(slug: &str) -> Option<f32> {
    match slug {
        "2" => Some(2.0),
        "4" => Some(4.0),
        "8" => Some(8.0),
        _ => None,
    }
}

pub(crate) fn option_fps_alpha_from_slug(slug: &str) -> Option<f64> {
    match slug {
        "10" => Some(0.10),
        "18" => Some(0.18),
        "30" => Some(0.30),
        _ => None,
    }
}

pub(crate) fn option_interval_from_slug(slug: &str) -> Option<u64> {
    match slug {
        "60" => Some(60),
        "120" => Some(120),
        "300" => Some(300),
        _ => None,
    }
}

pub(crate) fn option_recent_limit_from_slug(slug: &str) -> Option<usize> {
    match slug {
        "5" => Some(5),
        "10" => Some(10),
        "25" => Some(25),
        _ => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StartupOptions {
    pub demo_workspace: bool,
    pub stress_count: Option<usize>,
    pub hierarchy_demo: bool,
    pub view_3d: bool,
    pub view_mode: Option<StartupView>,
    pub benchmark_3d: Option<Benchmark3dOptions>,
    pub show_options: bool,
    pub app_options: Option<AppOptions>,
    pub options_file_path: Option<String>,
    pub select_first_shape: bool,
    pub move_first_vertex: bool,
    pub zoom: Option<f32>,
    pub pan: Option<[f32; 2]>,
    pub hierarchy_workflow_demo: bool,
    pub startup_actions: Vec<String>,
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
            "ui_runtime={} view={} viewport={}x{} paint_items={} layout_warnings={} shapes={} lots={} tools={}",
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

#[cfg(not(target_arch = "wasm32"))]
pub fn write_operad_snapshot_rgba(
    path: &Path,
    report: &OperadSnapshotReport,
) -> Result<(), String> {
    let image = report
        .render
        .snapshot
        .as_ref()
        .ok_or_else(|| "snapshot render did not produce image pixels".to_string())?;
    if image.format != ResourceFormat::Rgba8 {
        return Err(format!(
            "unsupported snapshot format {:?}; expected Rgba8",
            image.format
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create snapshot directory {}: {err}", parent.display()))?;
    }
    let mut file = fs::File::create(path)
        .map_err(|err| format!("create snapshot file {}: {err}", path.display()))?;
    file.write_all(&image.pixels)
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_operad_snapshot_ppm(path: &Path, report: &OperadSnapshotReport) -> Result<(), String> {
    let image = report
        .render
        .snapshot
        .as_ref()
        .ok_or_else(|| "snapshot render did not produce image pixels".to_string())?;
    if image.format != ResourceFormat::Rgba8 {
        return Err(format!(
            "unsupported snapshot format {:?}; expected Rgba8",
            image.format
        ));
    }
    let width = image.size.width;
    let height = image.size.height;
    let expected_len = width as usize * height as usize * 4;
    if image.pixels.len() != expected_len {
        return Err(format!(
            "snapshot pixel buffer has {} bytes; expected {expected_len}",
            image.pixels.len()
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create snapshot directory {}: {err}", parent.display()))?;
    }
    let mut file = fs::File::create(path)
        .map_err(|err| format!("create snapshot file {}: {err}", path.display()))?;
    write!(file, "P6\n{width} {height}\n255\n")
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    for rgba in image.pixels.chunks_exact(4) {
        file.write_all(&rgba[..3])
            .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_operad_snapshot_png(path: &Path, report: &OperadSnapshotReport) -> Result<(), String> {
    let image = report
        .render
        .snapshot
        .as_ref()
        .ok_or_else(|| "snapshot render did not produce image pixels".to_string())?;
    if image.format != ResourceFormat::Rgba8 {
        return Err(format!(
            "unsupported snapshot format {:?}; expected Rgba8",
            image.format
        ));
    }
    let width = image.size.width;
    let height = image.size.height;
    if width == 0 || height == 0 {
        return Err("snapshot PNG export requires non-zero dimensions".to_string());
    }
    let row_stride = width as usize * 4;
    let expected_len = row_stride * height as usize;
    if image.pixels.len() != expected_len {
        return Err(format!(
            "snapshot pixel buffer has {} bytes; expected {expected_len}",
            image.pixels.len()
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create snapshot directory {}: {err}", parent.display()))?;
    }

    let mut filtered = Vec::with_capacity((row_stride + 1) * height as usize);
    for row in 0..height as usize {
        filtered.push(0);
        let start = row * row_stride;
        filtered.extend_from_slice(&image.pixels[start..start + row_stride]);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder
        .write_all(&filtered)
        .map_err(|err| format!("compress PNG snapshot rows: {err}"))?;
    let compressed = encoder
        .finish()
        .map_err(|err| format!("finish PNG snapshot compression: {err}"))?;

    let mut file = fs::File::create(path)
        .map_err(|err| format!("create snapshot file {}: {err}", path.display()))?;
    file.write_all(b"\x89PNG\r\n\x1a\n")
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_png_chunk(&mut file, b"IHDR", &ihdr, path)?;
    write_png_chunk(&mut file, b"IDAT", &compressed, path)?;
    write_png_chunk(&mut file, b"IEND", &[], path)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_png_chunk(
    file: &mut fs::File,
    chunk_type: &[u8; 4],
    data: &[u8],
    path: &Path,
) -> Result<(), String> {
    let length = u32::try_from(data.len()).map_err(|_| {
        format!(
            "PNG chunk {:?} is too large",
            String::from_utf8_lossy(chunk_type)
        )
    })?;
    file.write_all(&length.to_be_bytes())
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    file.write_all(chunk_type)
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    file.write_all(data)
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    let checksum = png_crc32(chunk_type, data);
    file.write_all(&checksum.to_be_bytes())
        .map_err(|err| format!("write snapshot file {}: {err}", path.display()))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn png_crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in chunk_type.iter().chain(data.iter()) {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
