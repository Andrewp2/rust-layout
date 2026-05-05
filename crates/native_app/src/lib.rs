#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use drc::{DrcViolation, RuleDeck, run_drc, run_drc_incremental};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Painter, PointerButton, Pos2, Rect as EguiRect, Sense,
    Stroke, StrokeKind, Vec2, vec2,
};
use geometry_core::{Coord, Point, Rect, Vector, distance_point_to_segment};
use layout_model::{
    Cell, CellId, CellInstance, ClientMessage, CrdtApplyResult, CrdtOperation, Document,
    InstanceArray, InstanceId, LayerId, LayoutIndex, LoroCrdtLog, LoroUpdate, MarkerState, NetId,
    Operation, ProcessLayer, ServerMessage, Shape, ShapeId, ShapeKind, ShapeOccurrenceId,
    TechnologyFile, Transform, builtin_technologies,
    connectivity::{ConnectivityReport, NetComponent, extract_connectivity},
    gdsii::{export_gdsii, import_gdsii},
};
#[cfg(not(target_arch = "wasm32"))]
use renderer::gpu::OffscreenRenderRequest;
use renderer::gpu::{
    BufferUploadResult, GpuPickRequest, GpuUploadStats, LayoutGpuRenderer, ViewUniforms,
};
use router::{RouteRequest, RouterConfig, route};
use uuid::Uuid;
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use futures_util::{Sink, SinkExt, StreamExt};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

const SAVE_PATH: &str = "examples/fabricad_layout.json";
const GDS_PATH: &str = "examples/fabricad_layout.gds";
const MAX_LORO_SEED_OBJECTS: usize = 5_000;
const MAX_CONNECTIVITY_OBJECTS: usize = 50_000;
const TILE_MEMORY_BUDGET_BYTES: usize = 96 * 1024 * 1024;

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
    index: LayoutIndex,
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
    zoom: f32,
    pan: Vec2,
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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StartupOptions {
    pub stress_count: Option<usize>,
    pub hierarchy_demo: bool,
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
        let document = Document::demo();
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
        Self {
            document,
            index,
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
            zoom: 0.075,
            pan: Vec2::ZERO,
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
            status: "ready".to_string(),
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
        }
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
            app.status = format!("test scene: {count} polygons");
        }
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
        let target = self.last_view_center.snap(self.document.grid);
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
        let offset = (self.document.grid.max(10) * 8).max(80);
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
        let origin = bounds.min.snap(self.document.grid);
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
        let location = self.last_view_center.snap(self.document.grid);
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
            Ok(()) => self.status = format!("saved {SAVE_PATH}"),
            Err(err) => self.status = format!("save failed: {err}"),
        }
    }

    fn load_document(&mut self) {
        match fs::read_to_string(SAVE_PATH)
            .map_err(serde_json::Error::io)
            .and_then(|contents| serde_json::from_str::<Document>(&contents))
        {
            Ok(document) => {
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
                self.status = format!("loaded {SAVE_PATH}");
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
                self.status = format!("imported {GDS_PATH}");
            }
            Err(err) => self.status = format!("GDS import failed: {err}"),
        }
    }

    fn make_stress_document(&mut self, count: usize) {
        self.document = Document::stress(count);
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
        let tolerance = self.document.grid.max(1) * 2;
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
                if !input.modifiers.any() {
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

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            tool_button(ui, &mut self.tool, Tool::Select, "Select");
            tool_button(ui, &mut self.tool, Tool::Rect, "Rect");
            tool_button(ui, &mut self.tool, Tool::Polygon, "Poly");
            tool_button(ui, &mut self.tool, Tool::Path, "Path");
            tool_button(ui, &mut self.tool, Tool::Via, "Via");
            tool_button(ui, &mut self.tool, Tool::Measure, "Measure");
            tool_button(ui, &mut self.tool, Tool::Route, "Route");
            ui.separator();
            if ui.button("Undo").clicked() {
                self.undo();
            }
            if ui.button("Redo").clicked() {
                self.redo();
            }
            if ui.button("Copy").clicked() {
                self.copy_selection();
            }
            if ui.button("Paste").clicked() {
                self.paste_clipboard();
            }
            if ui.button("Dup").clicked() {
                self.duplicate_selection();
            }
            if ui.button("Delete").clicked() {
                self.delete_selection();
            }
            if ui.button("Rot90").clicked() {
                self.rotate_selected_90();
            }
            if ui.button("Mirror X").clicked() {
                self.mirror_selected_x();
            }
            if ui.button("Mirror Y").clicked() {
                self.mirror_selected_y();
            }
            if ui.button("DRC").clicked() {
                self.rerun_drc();
            }
            if ui.button("Save").clicked() {
                self.save_document();
            }
            if ui.button("Load").clicked() {
                self.load_document();
            }
            if ui.button("Export GDS").clicked() {
                self.export_gds_document();
            }
            if ui.button("Import GDS").clicked() {
                self.import_gds_document();
            }
            if ui.button("Connect").clicked() {
                self.connect_collaboration();
            }
            if ui.button("Make Cell").clicked() {
                self.create_cell_from_selection();
            }
            ui.separator();
            if ui.button("10k").clicked() {
                self.make_stress_document(10_000);
            }
            if ui.button("100k").clicked() {
                self.make_stress_document(100_000);
            }
            if ui.button("1M").clicked() {
                self.make_stress_document(1_000_000);
            }
            if ui.button("Hierarchy").clicked() {
                self.make_hierarchy_document();
            }
            ui.separator();
            ui.label(format!("User {}", short_user(self.user_id)));
            ui.label(&self.status);
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
            "Grid: {} dbu, DBU/um: {}",
            technology.grid, technology.dbu_per_micron
        ));
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("layers")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Fabricad");
                ui.separator();
                self.technology_panel(ui);
                ui.separator();
                ui.label("Layers");
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
                for (layer, visible) in visibility_ops {
                    self.apply_operation_without_history(&Operation::SetLayerVisibility {
                        layer,
                        visible,
                    });
                }
                ui.separator();
                self.hierarchy_panel(ui);
                ui.separator();
                ui.label(format!("Shapes: {}", self.document.shapes.len()));
                ui.label(format!("Visible: {}", self.perf.visible_count));
                ui.label(format!(
                    "Markers: {} active / {} total",
                    self.active_marker_count(),
                    self.violations.len()
                ));
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
                ui.label(format!(
                    "Tiles: {} visible / {} resident / {} rebuilt / {} dirty / {} evicted",
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
                ui.label(format!("DRC: {:.3} ms", self.perf.drc_ms));
                ui.label(format!("Net: {:.3} ms", self.perf.connectivity_ms));
                ui.label(format!("Route: {:.3} ms", self.perf.route_ms));
                ui.label(format!("Frame: {:.2} ms", self.perf.frame_ms));
                ui.label(format!(
                    "Batch: {:.2} MB",
                    (self.perf.batch_bytes + self.perf.pick_batch_bytes) as f64 / (1024.0 * 1024.0)
                ));
                ui.label(format!(
                    "Tile cache: {:.2} / {:.0} MB, over {:.2} MB",
                    self.perf.tile_cache_bytes as f64 / (1024.0 * 1024.0),
                    self.perf.tile_memory_budget_bytes as f64 / (1024.0 * 1024.0),
                    self.perf.tile_over_budget_bytes as f64 / (1024.0 * 1024.0)
                ));
                ui.separator();
                self.net_panel(ui);
                ui.separator();
                self.marker_panel(ui);
            });
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
                ui.add(egui::DragValue::new(&mut x).speed(self.document.grid.max(1) as f64));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut y).speed(self.document.grid.max(1) as f64));
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
                ui.add(
                    egui::DragValue::new(&mut array.column_pitch.dx)
                        .speed(self.document.grid.max(1) as f64),
                );
                ui.label("dy");
                ui.add(
                    egui::DragValue::new(&mut array.column_pitch.dy)
                        .speed(self.document.grid.max(1) as f64),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Row dx");
                ui.add(
                    egui::DragValue::new(&mut array.row_pitch.dx)
                        .speed(self.document.grid.max(1) as f64),
                );
                ui.label("dy");
                ui.add(
                    egui::DragValue::new(&mut array.row_pitch.dy)
                        .speed(self.document.grid.max(1) as f64),
                );
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
        shape_kind_property_ui(ui, &mut edited.kind, self.document.grid);

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
        let moved = Point::new(first.x - 500, first.y + 420).snap(self.document.grid);
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
                let world = self
                    .screen_to_world(pointer, canvas)
                    .snap(self.document.grid);
                self.broadcast_cursor(world);
            }
        }

        let viewport = self.viewport_world(canvas);
        self.last_view_center = viewport.center().snap(self.document.grid);
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

    fn draw_background(&self, painter: &Painter, canvas: EguiRect, viewport: Rect) {
        painter.rect_filled(canvas, 0.0, Color32::from_rgb(13, 16, 18));
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
        let origin = self.world_to_screen(Point::ZERO, canvas);
        painter.circle_filled(origin, 3.0, Color32::from_rgb(220, 220, 210));
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
        let label = format_scale_label(length_dbu, self.current_technology().dbu_per_micron);
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
        let current = self
            .screen_to_world(pointer, canvas)
            .snap(self.document.grid);
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
        let world = self
            .screen_to_world(pointer, canvas)
            .snap(self.document.grid);
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
        let tolerance = (10.0 / self.zoom).max(self.document.grid as f32) as Coord;
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
                if rect.width().abs() >= self.document.grid
                    && rect.height().abs() >= self.document.grid
                {
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
                        label: format_physical_length(
                            distance,
                            self.current_technology().dbu_per_micron,
                        ),
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
        let target_px = 24.0;
        let raw = (target_px / self.zoom).max(self.document.grid as f32) as Coord;
        let base = self.document.grid.max(1);
        let mut step = base;
        while step < raw {
            step *= 2;
        }
        step
    }
}

impl eframe::App for FabricadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_collaboration();
        self.handle_shortcuts(ctx);
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        self.side_panel(ctx);
        egui::CentralPanel::default().show(ctx, |ui| self.canvas(ui));
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

struct LayoutGpuCallback {
    batch: renderer::RenderBatch,
    pick_batch: Option<renderer::PickBatch>,
    pick_request: Option<GpuPickRequest>,
    pick_state: SharedGpuPickState,
    upload_state: SharedGpuUploadState,
    uniforms: ViewUniforms,
    target_format: egui_wgpu::wgpu::TextureFormat,
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

fn format_scale_label(length_dbu: Coord, dbu_per_micron: Coord) -> String {
    format_physical_length(length_dbu as f64, dbu_per_micron)
}

fn format_physical_length(length_dbu: f64, dbu_per_micron: Coord) -> String {
    let microns = length_dbu / dbu_per_micron.max(1) as f64;
    if microns.abs() < 1.0 {
        let nanometers = microns * 1_000.0;
        if nanometers.abs() >= 100.0 {
            format!("{nanometers:.0} nm")
        } else if nanometers.abs() >= 10.0 {
            format!("{nanometers:.1} nm")
        } else {
            format!("{nanometers:.2} nm")
        }
    } else if is_effectively_integer(microns) {
        format!("{microns:.0} um")
    } else if microns.abs() < 10.0 {
        format!("{microns:.2} um")
    } else {
        format!("{microns:.1} um")
    }
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

fn layer_color32(color: [f32; 4], multiplier: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (color[0] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[1] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[2] * 255.0 * multiplier).clamp(0.0, 255.0) as u8,
        (color[3] * 255.0).clamp(0.0, 255.0) as u8,
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
