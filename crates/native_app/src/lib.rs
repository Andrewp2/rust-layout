use std::{
    cell::{Cell as StdCell, RefCell},
    collections::{BTreeMap, BTreeSet},
};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use drc::{
    DerivedForbiddenOverlapRule, DerivedLayerOperation, DerivedLayerRule, DrcIssueStore,
    DrcValidationFinding, DrcValidationSeverity, DrcViolation, EnclosureRule, ForbiddenOverlapRule,
    RuleDeck, import_calibre_rve_markers, run_drc,
};
#[cfg(not(target_arch = "wasm32"))]
use flate2::read::GzDecoder;
use geometry_core::{Coord, Point, Polygon, Rect, Vector, distance_point_to_segment};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::ShapeKindView;
#[cfg(not(target_arch = "wasm32"))]
use layout_model::cif::{export_cif_with_report, import_cif_with_report};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::def::{export_def_with_report, import_def_with_report};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::dxf::{export_dxf_with_report, import_dxf_with_report};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::gdsii::{export_gdsii, export_gdsii_with_report, import_gdsii_with_report};
#[cfg(not(target_arch = "wasm32"))]
use layout_model::lef::{export_lef_with_report, import_lef_with_report};
pub use layout_model::workspace::BuiltinDemoWorkspaceReport;
use layout_model::{
    ArrayIndex, Cell, CellId, CellInstance, Document, IndexedShape, InstanceArray, InstanceId,
    Layer, LayerFillStyle, LayerId, LayerLineStyle, LayoutIndex, MarkerState, MeasurementMode,
    NetId, Operation, ProcessLayer, ReferenceImageLandmark, ReferenceImageOverlay, Shape, ShapeId,
    ShapeKind, ShapeOccurrenceId, TechnologyError, TechnologyFile, Transform,
    connectivity::{
        ConnectivityIssueKind, ConnectivityReport, ConnectivityValidationSeverity, ExtractedDevice,
        ExtractedDeviceTerminal, NetComponent, NetLabel, NetOpen, NetShort,
        SpiceConnectivityComparisonReport, compare_connectivity_to_spice_schematic,
        export_connectivity_spice, extract_connectivity, parse_spice_schematic_netlist,
    },
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
    workspace::{WORKSPACE_DATASET_SCHEMA_VERSION, WorkspaceDataset, WorkspaceSnapshotMetadata},
    yield_analysis::{DieOutcome, FailureMode, ProcessMeasurement, YieldAnalysis, YieldSummary},
};
use operad::renderer::{
    RenderFrameOutput, RenderFrameRequest, RenderOptions, RenderTarget, RendererAdapter,
    ResourceFormat,
};
#[cfg(not(target_arch = "wasm32"))]
use operad::wgpu_renderer::WgpuCanvasContext;
use operad::wgpu_renderer::WgpuRenderer;
use operad::widgets::{ButtonOptions, CollapsingHeaderOptions, button, collapsing_header};
use operad::{
    AccessibilityAction, AccessibilityMeta, AccessibilityRole, ApproxTextMeasurer,
    CanvasInteractionPolicy, ClipBehavior, ColorRgba, FontWeight, InputBehavior, LayoutStyle,
    PaintText, ScenePrimitive, ScrollAxes, StrokeStyle, TextHorizontalAlign, TextStyle,
    TextVerticalAlign, TextWrap, UiContent, UiDocument, UiNode, UiNodeStyle, UiPoint, UiRect,
    UiSize, UiVisual, WidgetActionBinding, layout, platform::PixelSize, root_style,
};
#[cfg(not(target_arch = "wasm32"))]
use operad_wgpu as wgpu;
#[cfg(not(target_arch = "wasm32"))]
use renderer::{
    BatchFingerprint, GpuRectSlabInstance, GpuVertex3d, RenderBatch3d, TileCache, TileFrameOptions,
    TileKey, TileLodConfig, TiledFrame,
    gpu::{
        LayoutGpuRenderer, OffscreenRenderRequest, VIEWPORT_3D_COLOR_FORMAT, ViewUniforms,
        Viewport3dRenderer, Viewport3dUniforms,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use uuid::Uuid;

mod camera;
mod labels;
mod options;
mod ui;
use camera::{CAMERA_NEAR_PLANE, Camera3d, Vec3, view_projection_3d};
use labels::{
    capitalize_ascii_first, compact_button_label, display_inventory_actor_identifier,
    display_layout_revision_identifier, display_lot_identifier, display_mask_identifier,
    display_measurement_identifier, display_owner_identifier, display_product_label,
    display_recipe_identifier, display_reticle_field_identifier, display_reticle_identifier,
    display_route_identifier, display_spc_control_identifier, display_step_identifier,
    display_technology_name, display_tool_run_identifier, display_wafer_identifier,
    humanize_identifier, inventory_usage_link_label, trace_lot_button_label, trace_lot_label,
    workflow_lot_label, yield_wafer_label, yield_wafer_short_label,
};
pub use options::{APP_OPTIONS_SCHEMA_VERSION, AppOptions, GLASSWORKS_OPTIONS_FILE_ENV};
#[cfg(not(target_arch = "wasm32"))]
pub use options::{default_options_path, load_options_file, save_options_file};
pub use ui::UiScale;
use ui::{
    ShellMetrics, ShellMetricsOptions, add_node_marker, inspector_panel_width,
    pad_pre_shell_node_ids, secondary_panel_width,
};

#[path = "app/mod.rs"]
mod app;
pub use app::*;
#[allow(unused_imports)]
pub(crate) use app::*;

#[cfg(test)]
#[path = "app/tests/mod.rs"]
mod tests;
