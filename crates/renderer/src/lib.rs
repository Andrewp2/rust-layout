pub mod gpu;
pub mod shader;

use std::{
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    fmt,
    hash::{Hash, Hasher},
};

use geometry_core::{Point, Rect};
use layout_model::{
    Document, InstanceId, LayerId, LayoutIndex, Shape, ShapeId, ShapeKind, ShapeKindView,
    ShapeOccurrenceId, ShapeView, Transform,
};
use tracing::warn;
use web_time::Instant;

pub const DEFAULT_TILE_SIZE: i64 = 16_384;
pub const DEFAULT_LOD_MAX_TILE_SCREEN_PX: f32 = 220.0;
pub const DEFAULT_LOD_MIN_SHAPES_PER_TILE: usize = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuVertex3d {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuRectSlabInstance {
    pub rect: [f32; 4],
    pub z_range: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickVertex {
    pub position: [f32; 2],
    pub pick_id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct RenderBatch {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct RenderBatch3d {
    pub vertices: Vec<GpuVertex3d>,
    pub indices: Vec<u32>,
    pub rect_slabs: Vec<GpuRectSlabInstance>,
    pub guide_vertices: Vec<GpuVertex3d>,
    pub guide_indices: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct PickBatch {
    pub vertices: Vec<PickVertex>,
    pub indices: Vec<u32>,
    pub occurrences: Vec<ShapeOccurrenceId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchFingerprint {
    pub vertex_count: usize,
    pub index_count: usize,
    pub hash: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderBatch3dValidation {
    pub mesh_vertices: usize,
    pub mesh_triangles: usize,
    pub rect_slabs: usize,
    pub guide_vertices: usize,
    pub guide_segments: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderBatch3dValidationError {
    MeshIndexCountNotMultipleOfThree {
        index_count: usize,
    },
    GuideIndexCountNotMultipleOfTwo {
        index_count: usize,
    },
    MeshIndexOutOfBounds {
        index: u32,
        vertex_count: usize,
    },
    GuideIndexOutOfBounds {
        index: u32,
        vertex_count: usize,
    },
    NonFiniteVertex {
        stream: &'static str,
        vertex: usize,
        component: &'static str,
        value: f32,
    },
    NonFiniteColor {
        stream: &'static str,
        vertex: usize,
        channel: usize,
        value: f32,
    },
    InvalidNormal {
        vertex: usize,
    },
    DegenerateTriangle {
        triangle: usize,
    },
    InvalidRectSlab {
        slab: usize,
        reason: &'static str,
    },
}

impl fmt::Display for RenderBatch3dValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MeshIndexCountNotMultipleOfThree { index_count } => {
                write!(f, "3D mesh index count {index_count} is not divisible by 3")
            }
            Self::GuideIndexCountNotMultipleOfTwo { index_count } => {
                write!(
                    f,
                    "3D guide index count {index_count} is not divisible by 2"
                )
            }
            Self::MeshIndexOutOfBounds {
                index,
                vertex_count,
            } => write!(
                f,
                "3D mesh index {index} is outside {vertex_count} mesh vertices"
            ),
            Self::GuideIndexOutOfBounds {
                index,
                vertex_count,
            } => write!(
                f,
                "3D guide index {index} is outside {vertex_count} guide vertices"
            ),
            Self::NonFiniteVertex {
                stream,
                vertex,
                component,
                value,
            } => write!(
                f,
                "3D {stream} vertex {vertex} has non-finite {component} component {value}"
            ),
            Self::NonFiniteColor {
                stream,
                vertex,
                channel,
                value,
            } => write!(
                f,
                "3D {stream} vertex {vertex} has non-finite color channel {channel}: {value}"
            ),
            Self::InvalidNormal { vertex } => {
                write!(f, "3D mesh vertex {vertex} has an invalid normal")
            }
            Self::DegenerateTriangle { triangle } => {
                write!(f, "3D mesh triangle {triangle} is degenerate")
            }
            Self::InvalidRectSlab { slab, reason } => {
                write!(f, "3D rect slab {slab} is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for RenderBatch3dValidationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileKey {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug, Default)]
pub struct TileFrameStats {
    pub visible_tiles: usize,
    pub rebuilt_tiles: usize,
    pub resident_tiles: usize,
    pub evicted_tiles: usize,
    pub lod_tiles: usize,
    pub lod_shapes: usize,
    pub precise_shapes: usize,
    pub cached_tiles: usize,
    pub visible_shapes: usize,
    pub pick_shapes: usize,
    pub shape_cache_hits: usize,
    pub shape_cache_misses: usize,
    pub resident_shape_batches: usize,
    pub evicted_shape_batches: usize,
    pub draw_ranges: usize,
    pub cache_bytes: usize,
    pub memory_budget_bytes: Option<usize>,
    pub over_budget_bytes: usize,
    pub pick_build_ms: f64,
}

#[derive(Clone, Debug, Default)]
pub struct TiledFrame {
    pub render: RenderBatch,
    pub pick: Option<PickBatch>,
    pub draw_ranges: Vec<DrawBatchRange>,
    pub stats: TileFrameStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawBatchRange {
    pub kind: DrawBatchRangeKind,
    pub index_start: usize,
    pub index_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawBatchRangeKind {
    TileOverview { tile: TileKey },
    Shape { occurrence: ShapeOccurrenceId },
}

#[derive(Clone, Copy, Debug)]
pub struct TileFrameOptions {
    pub include_pick: bool,
    pub zoom: f32,
    pub lod: TileLodConfig,
    pub memory_budget_bytes: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct TileLodConfig {
    pub enabled: bool,
    pub max_tile_screen_px: f32,
    pub min_shapes_per_tile: usize,
}

#[derive(Clone, Debug)]
pub struct TileCache {
    tile_size: i64,
    tiles: BTreeMap<TileKey, CachedTile>,
    shapes: BTreeMap<ShapeOccurrenceId, RenderBatch>,
    frame_counter: u64,
}

#[derive(Clone, Debug)]
struct CachedTile {
    occurrences: Vec<ShapeOccurrenceId>,
    overview: Option<OverviewTile>,
    last_used_frame: u64,
}

#[derive(Clone, Debug)]
struct OverviewTile {
    batch: RenderBatch,
    shape_count: usize,
}

impl Default for TileCache {
    fn default() -> Self {
        Self::new(DEFAULT_TILE_SIZE)
    }
}

impl Default for TileFrameOptions {
    fn default() -> Self {
        Self {
            include_pick: false,
            zoom: 1.0,
            lod: TileLodConfig::default(),
            memory_budget_bytes: None,
        }
    }
}

impl Default for TileLodConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tile_screen_px: DEFAULT_LOD_MAX_TILE_SCREEN_PX,
            min_shapes_per_tile: DEFAULT_LOD_MIN_SHAPES_PER_TILE,
        }
    }
}

impl RenderBatch {
    pub fn estimate_bytes(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<GpuVertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }

    pub fn fingerprint(&self) -> BatchFingerprint {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.vertices.len().hash(&mut hasher);
        self.indices.len().hash(&mut hasher);
        for vertex in &self.vertices {
            vertex.position[0].to_bits().hash(&mut hasher);
            vertex.position[1].to_bits().hash(&mut hasher);
            for channel in vertex.color {
                channel.to_bits().hash(&mut hasher);
            }
        }
        self.indices.hash(&mut hasher);
        BatchFingerprint {
            vertex_count: self.vertices.len(),
            index_count: self.indices.len(),
            hash: hasher.finish(),
        }
    }
}

impl RenderBatch3d {
    pub fn estimate_bytes(&self) -> usize {
        (self.vertices.len() + self.guide_vertices.len()) * std::mem::size_of::<GpuVertex3d>()
            + (self.indices.len() + self.guide_indices.len()) * std::mem::size_of::<u32>()
            + self.rect_slabs.len() * std::mem::size_of::<GpuRectSlabInstance>()
    }

    pub fn fingerprint(&self) -> BatchFingerprint {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.vertices.len().hash(&mut hasher);
        self.indices.len().hash(&mut hasher);
        self.rect_slabs.len().hash(&mut hasher);
        self.guide_vertices.len().hash(&mut hasher);
        self.guide_indices.len().hash(&mut hasher);
        for vertex in &self.vertices {
            for component in vertex.position {
                component.to_bits().hash(&mut hasher);
            }
            for component in vertex.normal {
                component.to_bits().hash(&mut hasher);
            }
            for channel in vertex.color {
                channel.to_bits().hash(&mut hasher);
            }
        }
        for instance in &self.rect_slabs {
            for component in instance.rect {
                component.to_bits().hash(&mut hasher);
            }
            for component in instance.z_range {
                component.to_bits().hash(&mut hasher);
            }
            for channel in instance.color {
                channel.to_bits().hash(&mut hasher);
            }
        }
        for vertex in &self.guide_vertices {
            for component in vertex.position {
                component.to_bits().hash(&mut hasher);
            }
            for component in vertex.normal {
                component.to_bits().hash(&mut hasher);
            }
            for channel in vertex.color {
                channel.to_bits().hash(&mut hasher);
            }
        }
        self.indices.hash(&mut hasher);
        self.guide_indices.hash(&mut hasher);
        BatchFingerprint {
            vertex_count: self.vertices.len() + self.guide_vertices.len() + self.rect_slabs.len(),
            index_count: self.indices.len() + self.guide_indices.len(),
            hash: hasher.finish(),
        }
    }

    pub fn validate_geometry(
        &self,
    ) -> Result<RenderBatch3dValidation, RenderBatch3dValidationError> {
        if self.indices.len() % 3 != 0 {
            return Err(
                RenderBatch3dValidationError::MeshIndexCountNotMultipleOfThree {
                    index_count: self.indices.len(),
                },
            );
        }
        if self.guide_indices.len() % 2 != 0 {
            return Err(
                RenderBatch3dValidationError::GuideIndexCountNotMultipleOfTwo {
                    index_count: self.guide_indices.len(),
                },
            );
        }

        validate_3d_vertices("mesh", &self.vertices, true)?;
        validate_3d_vertices("guide", &self.guide_vertices, false)?;

        for &index in &self.indices {
            if index as usize >= self.vertices.len() {
                return Err(RenderBatch3dValidationError::MeshIndexOutOfBounds {
                    index,
                    vertex_count: self.vertices.len(),
                });
            }
        }
        for &index in &self.guide_indices {
            if index as usize >= self.guide_vertices.len() {
                return Err(RenderBatch3dValidationError::GuideIndexOutOfBounds {
                    index,
                    vertex_count: self.guide_vertices.len(),
                });
            }
        }
        for (triangle, indices) in self.indices.chunks_exact(3).enumerate() {
            let a = self.vertices[indices[0] as usize].position;
            let b = self.vertices[indices[1] as usize].position;
            let c = self.vertices[indices[2] as usize].position;
            if triangle_area2_3d(a, b, c) <= f32::EPSILON {
                return Err(RenderBatch3dValidationError::DegenerateTriangle { triangle });
            }
        }
        for (slab, instance) in self.rect_slabs.iter().enumerate() {
            validate_rect_slab_instance(slab, instance)?;
        }

        Ok(RenderBatch3dValidation {
            mesh_vertices: self.vertices.len(),
            mesh_triangles: self.indices.len() / 3,
            rect_slabs: self.rect_slabs.len(),
            guide_vertices: self.guide_vertices.len(),
            guide_segments: self.guide_indices.len() / 2,
        })
    }
}

fn validate_3d_vertices(
    stream: &'static str,
    vertices: &[GpuVertex3d],
    require_normal: bool,
) -> Result<(), RenderBatch3dValidationError> {
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        for (component_index, value) in vertex.position.into_iter().enumerate() {
            if !value.is_finite() {
                return Err(RenderBatch3dValidationError::NonFiniteVertex {
                    stream,
                    vertex: vertex_index,
                    component: match component_index {
                        0 => "x",
                        1 => "y",
                        _ => "z",
                    },
                    value,
                });
            }
        }
        for (component_index, value) in vertex.normal.into_iter().enumerate() {
            if !value.is_finite() {
                return Err(RenderBatch3dValidationError::NonFiniteVertex {
                    stream,
                    vertex: vertex_index,
                    component: match component_index {
                        0 => "normal x",
                        1 => "normal y",
                        _ => "normal z",
                    },
                    value,
                });
            }
        }
        if require_normal && vector_length2_3d(vertex.normal) <= f32::EPSILON {
            return Err(RenderBatch3dValidationError::InvalidNormal {
                vertex: vertex_index,
            });
        }
        for (channel, value) in vertex.color.into_iter().enumerate() {
            if !value.is_finite() {
                return Err(RenderBatch3dValidationError::NonFiniteColor {
                    stream,
                    vertex: vertex_index,
                    channel,
                    value,
                });
            }
        }
    }
    Ok(())
}

fn validate_rect_slab_instance(
    slab: usize,
    instance: &GpuRectSlabInstance,
) -> Result<(), RenderBatch3dValidationError> {
    for value in instance
        .rect
        .into_iter()
        .chain(instance.z_range)
        .chain(instance.color)
    {
        if !value.is_finite() {
            return Err(RenderBatch3dValidationError::InvalidRectSlab {
                slab,
                reason: "non-finite value",
            });
        }
    }
    if instance.rect[0] >= instance.rect[2] || instance.rect[1] >= instance.rect[3] {
        return Err(RenderBatch3dValidationError::InvalidRectSlab {
            slab,
            reason: "empty rectangle",
        });
    }
    if instance.z_range[0] >= instance.z_range[1] {
        return Err(RenderBatch3dValidationError::InvalidRectSlab {
            slab,
            reason: "empty z range",
        });
    }
    Ok(())
}

fn triangle_area2_3d(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    vector_length2_3d(cross)
}

fn vector_length2_3d(vector: [f32; 3]) -> f32 {
    vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]
}

impl PickBatch {
    pub fn estimate_bytes(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<PickVertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
            + self.occurrences.len() * std::mem::size_of::<ShapeOccurrenceId>()
            + self
                .occurrences
                .iter()
                .map(|id| id.instance_path.len() * std::mem::size_of::<InstanceId>())
                .sum::<usize>()
    }

    pub fn gpu_fingerprint(&self) -> BatchFingerprint {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.vertices.len().hash(&mut hasher);
        self.indices.len().hash(&mut hasher);
        for vertex in &self.vertices {
            vertex.position[0].to_bits().hash(&mut hasher);
            vertex.position[1].to_bits().hash(&mut hasher);
            vertex.pick_id.hash(&mut hasher);
        }
        self.indices.hash(&mut hasher);
        BatchFingerprint {
            vertex_count: self.vertices.len(),
            index_count: self.indices.len(),
            hash: hasher.finish(),
        }
    }

    pub fn occurrence_for_pick_id(&self, pick_id: u32) -> Option<ShapeOccurrenceId> {
        let index = pick_id.checked_sub(1)? as usize;
        self.occurrences.get(index).cloned()
    }

    pub fn shape_for_pick_id(&self, pick_id: u32) -> Option<ShapeId> {
        self.occurrence_for_pick_id(pick_id)
            .map(|id| id.source_shape_id())
    }
}

impl TileKey {
    pub fn bounds(self, tile_size: i64) -> Rect {
        Rect::from_min_size(
            Point::new(self.x * tile_size, self.y * tile_size),
            tile_size,
            tile_size,
        )
    }
}

impl TileCache {
    pub fn new(tile_size: i64) -> Self {
        let effective_tile_size = tile_size.max(1);
        if effective_tile_size != tile_size {
            warn!(
                tile_size,
                effective_tile_size, "tile cache size below one dbu; clamping"
            );
        }
        Self {
            tile_size: effective_tile_size,
            tiles: BTreeMap::new(),
            shapes: BTreeMap::new(),
            frame_counter: 0,
        }
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.shapes.clear();
        self.frame_counter = 0;
    }

    pub fn invalidate_rect(&mut self, rect: Rect) -> usize {
        let keys = tile_keys_for_rect(rect, self.tile_size);
        let mut removed = 0;
        for key in keys {
            if self.tiles.remove(&key).is_some() {
                removed += 1;
            }
        }
        removed
    }

    pub fn invalidate_shape(&mut self, id: ShapeId) -> bool {
        let keys: Vec<_> = self
            .shapes
            .keys()
            .filter(|occurrence| occurrence.source_shape_id() == id)
            .cloned()
            .collect();
        let removed = !keys.is_empty();
        for key in keys {
            self.shapes.remove(&key);
        }
        removed
    }

    pub fn estimate_bytes(&self) -> usize {
        let tile_bytes = self
            .tiles
            .values()
            .map(|tile| {
                tile.occurrences.len() * std::mem::size_of::<ShapeOccurrenceId>()
                    + tile
                        .overview
                        .as_ref()
                        .map_or(0, |overview| overview.batch.estimate_bytes())
            })
            .sum::<usize>();
        let shape_bytes = self
            .shapes
            .values()
            .map(RenderBatch::estimate_bytes)
            .sum::<usize>();
        tile_bytes + shape_bytes
    }

    pub fn build_frame(
        &mut self,
        document: &Document,
        index: &LayoutIndex,
        viewport: Rect,
        include_pick: bool,
    ) -> TiledFrame {
        self.build_frame_with_options(
            document,
            index,
            viewport,
            TileFrameOptions {
                include_pick,
                ..Default::default()
            },
        )
    }

    pub fn build_frame_with_options(
        &mut self,
        document: &Document,
        index: &LayoutIndex,
        viewport: Rect,
        options: TileFrameOptions,
    ) -> TiledFrame {
        let keys = tile_keys_for_rect(viewport, self.tile_size);
        let next_frame = self.frame_counter.wrapping_add(1);
        if next_frame == 0 {
            warn!("tile cache frame counter wrapped; restarting at frame 1");
        }
        self.frame_counter = next_frame.max(1);
        let frame_id = self.frame_counter;
        let visible_tile_keys = keys.iter().copied().collect::<BTreeSet<_>>();
        let mut stats = TileFrameStats {
            visible_tiles: keys.len(),
            cached_tiles: self.tiles.len(),
            memory_budget_bytes: options.memory_budget_bytes,
            ..Default::default()
        };
        let mut visible_occurrences = BTreeSet::new();
        let mut precise_occurrences = BTreeSet::new();
        let mut pick_occurrences = options.include_pick.then(BTreeSet::new);
        let mut render = RenderBatch::default();
        let mut draw_ranges = Vec::new();

        for key in keys {
            let tile = self.tiles.entry(key).or_insert_with(|| {
                stats.rebuilt_tiles += 1;
                let bounds = key.bounds(self.tile_size);
                let mut occurrences = index.query_occurrences(bounds);
                occurrences.sort_unstable();
                occurrences.dedup();
                CachedTile {
                    occurrences,
                    overview: None,
                    last_used_frame: frame_id,
                }
            });
            tile.last_used_frame = frame_id;
            visible_occurrences.extend(tile.occurrences.iter().cloned());
            if let Some(pick_occurrences) = &mut pick_occurrences {
                pick_occurrences.extend(tile.occurrences.iter().cloned());
            }

            if should_use_overview(self.tile_size, tile.occurrences.len(), options) {
                let tile_bounds = key.bounds(self.tile_size);
                let overview = tile.overview.get_or_insert_with(|| {
                    build_overview_tile(document, tile_bounds, &tile.occurrences)
                });
                stats.lod_tiles += 1;
                stats.lod_shapes += overview.shape_count;
                let (index_start, index_count) =
                    append_render_batch_with_range(&mut render, &overview.batch);
                if index_count > 0 {
                    draw_ranges.push(DrawBatchRange {
                        kind: DrawBatchRangeKind::TileOverview { tile: key },
                        index_start,
                        index_count,
                    });
                }
            } else {
                precise_occurrences.extend(tile.occurrences.iter().cloned());
            }
        }

        stats.visible_shapes = visible_occurrences.len();
        stats.precise_shapes = precise_occurrences.len();
        let mut pick = options.include_pick.then(PickBatch::default);

        for id in precise_occurrences {
            let Some(shape) = document.shape_view_for_occurrence(&id) else {
                continue;
            };
            if !document
                .layer(shape.shape.layer)
                .is_some_and(|layer| layer.visible)
            {
                continue;
            }
            let geometry = match self.shapes.entry(id.clone()) {
                Entry::Occupied(entry) => {
                    stats.shape_cache_hits += 1;
                    entry.into_mut()
                }
                Entry::Vacant(entry) => {
                    stats.shape_cache_misses += 1;
                    entry.insert(build_shape_view_triangles(
                        document,
                        shape.shape,
                        shape.transform,
                    ))
                }
            };
            let (index_start, index_count) = append_render_batch_with_range(&mut render, geometry);
            if index_count > 0 {
                draw_ranges.push(DrawBatchRange {
                    kind: DrawBatchRangeKind::Shape { occurrence: id },
                    index_start,
                    index_count,
                });
            }
        }

        if let (Some(pick), Some(pick_occurrences)) = (&mut pick, pick_occurrences) {
            let pick_started = Instant::now();
            stats.pick_shapes = pick_occurrences.len();
            for id in pick_occurrences {
                let Some(shape) = document.shape_view_for_occurrence(&id) else {
                    continue;
                };
                if !document
                    .layer(shape.shape.layer)
                    .is_some_and(|layer| layer.visible)
                {
                    continue;
                }
                let geometry = match self.shapes.entry(id.clone()) {
                    Entry::Occupied(entry) => {
                        stats.shape_cache_hits += 1;
                        entry.into_mut()
                    }
                    Entry::Vacant(entry) => {
                        stats.shape_cache_misses += 1;
                        entry.insert(build_shape_view_triangles(
                            document,
                            shape.shape,
                            shape.transform,
                        ))
                    }
                };
                append_pick_from_render(pick, id, geometry);
            }
            stats.pick_build_ms = pick_started.elapsed().as_secs_f64() * 1000.0;
        }

        if let Some(budget) = options.memory_budget_bytes {
            let eviction = self.evict_to_budget(budget, &visible_tile_keys);
            stats.evicted_tiles = eviction.tiles;
            stats.evicted_shape_batches = eviction.shape_batches;
            stats.over_budget_bytes = self.estimate_bytes().saturating_sub(budget);
        }
        stats.cached_tiles = self.tiles.len();
        stats.resident_tiles = self.tiles.len();
        stats.resident_shape_batches = self.shapes.len();
        stats.draw_ranges = draw_ranges.len();
        stats.cache_bytes = self.estimate_bytes();
        TiledFrame {
            render,
            pick,
            draw_ranges,
            stats,
        }
    }

    fn evict_to_budget(
        &mut self,
        budget: usize,
        visible_tile_keys: &BTreeSet<TileKey>,
    ) -> EvictionStats {
        let mut stats = EvictionStats::default();
        while self.estimate_bytes() > budget {
            let Some(key) = self
                .tiles
                .iter()
                .filter(|(key, _)| !visible_tile_keys.contains(key))
                .min_by_key(|(_, tile)| tile.last_used_frame)
                .map(|(key, _)| *key)
            else {
                break;
            };
            self.tiles.remove(&key);
            stats.tiles += 1;
        }

        let resident_occurrences = self
            .tiles
            .values()
            .flat_map(|tile| tile.occurrences.iter().cloned())
            .collect::<BTreeSet<_>>();
        let orphan_shapes = self
            .shapes
            .keys()
            .filter(|occurrence| !resident_occurrences.contains(*occurrence))
            .cloned()
            .collect::<Vec<_>>();
        for occurrence in orphan_shapes {
            self.shapes.remove(&occurrence);
            stats.shape_batches += 1;
        }
        stats
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct EvictionStats {
    tiles: usize,
    shape_batches: usize,
}

pub fn build_layout_triangles(document: &Document, viewport: Rect) -> RenderBatch {
    let mut batch = RenderBatch::default();
    document.for_each_visible_flattened_shape_view(|_, shape| {
        if !shape.bounds.intersects(viewport) {
            return;
        }
        append_shape_view_triangles(&mut batch, document, shape.shape, shape.transform);
    });
    batch
}

pub fn build_pick_triangles(document: &Document, viewport: Rect) -> PickBatch {
    let mut batch = PickBatch::default();
    document.for_each_visible_flattened_shape_view(|id, shape| {
        if !shape.bounds.intersects(viewport) {
            return;
        }
        let requested_pick_id = batch.occurrences.len() + 1;
        if requested_pick_id > u32::MAX as usize {
            warn!(
                pick_id = requested_pick_id,
                max_pick_id = u32::MAX,
                "pick triangle occurrence count exceeded u32 pick id range; pick ids will be clamped"
            );
        }
        let pick_id = requested_pick_id.min(u32::MAX as usize) as u32;
        batch.occurrences.push(id);
        match shape.shape.kind {
            ShapeKindView::Rectangle(rect) => {
                push_pick_rect(&mut batch, shape.transform.apply_rect(rect), pick_id)
            }
            ShapeKindView::Polygon(poly) => {
                if poly.points.len() >= 3 {
                    push_transformed_pick_fan(&mut batch, &poly.points, shape.transform, pick_id);
                }
            }
            ShapeKindView::Path { points, width } => {
                for window in points.windows(2) {
                    push_pick_segment_as_rect(
                        &mut batch,
                        shape.transform.apply_point(window[0]),
                        shape.transform.apply_point(window[1]),
                        width,
                        pick_id,
                    );
                }
            }
            ShapeKindView::Via { center, size, .. } => {
                let center = shape.transform.apply_point(center);
                let half = size / 2;
                push_pick_rect(
                    &mut batch,
                    Rect::new(
                        Point::new(center.x - half, center.y - half),
                        Point::new(center.x + half, center.y + half),
                    ),
                    pick_id,
                );
            }
            ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. } => {}
        }
    });
    batch
}

pub fn build_shape_triangles(document: &Document, shape: &Shape) -> RenderBatch {
    let mut batch = RenderBatch::default();
    append_shape_triangles(&mut batch, document, shape);
    batch
}

pub fn build_shape_view_triangles(
    document: &Document,
    shape: ShapeView<'_>,
    transform: Transform,
) -> RenderBatch {
    let mut batch = RenderBatch::default();
    append_shape_view_triangles(&mut batch, document, shape, transform);
    batch
}

fn should_use_overview(tile_size: i64, shape_count: usize, options: TileFrameOptions) -> bool {
    options.lod.enabled
        && shape_count >= options.lod.min_shapes_per_tile
        && (tile_size as f32 * options.zoom) <= options.lod.max_tile_screen_px
}

fn tile_keys_for_rect(rect: Rect, tile_size: i64) -> Vec<TileKey> {
    let min_x = rect.min.x.div_euclid(tile_size);
    let max_x = rect.max.x.div_euclid(tile_size);
    let min_y = rect.min.y.div_euclid(tile_size);
    let max_y = rect.max.y.div_euclid(tile_size);
    let mut keys = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            keys.push(TileKey { x, y });
        }
    }
    keys
}

fn build_overview_tile(
    document: &Document,
    tile_bounds: Rect,
    occurrences: &[ShapeOccurrenceId],
) -> OverviewTile {
    let mut layers: BTreeMap<LayerId, (Rect, usize)> = BTreeMap::new();
    for id in occurrences {
        let Some(shape) = document.shape_view_for_occurrence(id) else {
            continue;
        };
        if !document
            .layer(shape.shape.layer)
            .is_some_and(|layer| layer.visible)
            || !is_geometry_shape_view(shape.shape.kind)
        {
            continue;
        }
        let Some(bounds) = shape.bounds.intersection(tile_bounds) else {
            continue;
        };
        layers
            .entry(shape.shape.layer)
            .and_modify(|(union, count)| {
                *union = union.union(bounds);
                *count += 1;
            })
            .or_insert((bounds, 1));
    }

    let mut batch = RenderBatch::default();
    let mut shape_count = 0;
    for (layer, (bounds, layer_shape_count)) in layers {
        let mut color = document.layer_color(layer);
        color[3] = (0.16 + (layer_shape_count as f32).ln_1p() * 0.08).min(0.68);
        push_rect(&mut batch, bounds, color);
        shape_count += layer_shape_count;
    }
    OverviewTile { batch, shape_count }
}

fn is_geometry_shape_view(kind: ShapeKindView<'_>) -> bool {
    !matches!(
        kind,
        ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. }
    )
}

fn append_shape_triangles(batch: &mut RenderBatch, document: &Document, shape: &Shape) {
    let color = document.layer_color(shape.layer);
    match &shape.kind {
        ShapeKind::Rectangle(rect) => push_rect(batch, *rect, color),
        ShapeKind::Polygon(poly) => {
            if poly.points.len() >= 3 {
                push_fan(batch, &poly.points, color);
            }
        }
        ShapeKind::Path { points, width } => {
            for window in points.windows(2) {
                push_segment_as_rect(batch, window[0], window[1], *width, color);
            }
        }
        ShapeKind::Via { center, size, .. } => {
            let half = *size / 2;
            push_rect(
                batch,
                Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                ),
                color,
            );
        }
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {}
    }
}

fn append_shape_view_triangles(
    batch: &mut RenderBatch,
    document: &Document,
    shape: ShapeView<'_>,
    transform: Transform,
) {
    let color = document.layer_color(shape.layer);
    if transform == Transform::IDENTITY {
        match shape.kind {
            ShapeKindView::Rectangle(rect) => push_rect(batch, rect, color),
            ShapeKindView::Polygon(poly) => {
                if poly.points.len() >= 3 {
                    push_fan(batch, &poly.points, color);
                }
            }
            ShapeKindView::Path { points, width } => {
                for window in points.windows(2) {
                    push_segment_as_rect(batch, window[0], window[1], width, color);
                }
            }
            ShapeKindView::Via { center, size, .. } => {
                let half = size / 2;
                push_rect(
                    batch,
                    Rect::new(
                        Point::new(center.x - half, center.y - half),
                        Point::new(center.x + half, center.y + half),
                    ),
                    color,
                );
            }
            ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. } => {}
        }
        return;
    }

    match shape.kind {
        ShapeKindView::Rectangle(rect) => push_rect(batch, transform.apply_rect(rect), color),
        ShapeKindView::Polygon(poly) => {
            if poly.points.len() >= 3 {
                push_transformed_fan(batch, &poly.points, transform, color);
            }
        }
        ShapeKindView::Path { points, width } => {
            for window in points.windows(2) {
                push_segment_as_rect(
                    batch,
                    transform.apply_point(window[0]),
                    transform.apply_point(window[1]),
                    width,
                    color,
                );
            }
        }
        ShapeKindView::Via { center, size, .. } => {
            let center = transform.apply_point(center);
            let half = size / 2;
            push_rect(
                batch,
                Rect::new(
                    Point::new(center.x - half, center.y - half),
                    Point::new(center.x + half, center.y + half),
                ),
                color,
            );
        }
        ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. } => {}
    }
}

fn append_render_batch_with_range(out: &mut RenderBatch, batch: &RenderBatch) -> (usize, usize) {
    let index_start = out.indices.len();
    let base_len = out.vertices.len();
    if base_len > u32::MAX as usize {
        warn!(
            vertex_count = base_len,
            max_indexable_vertices = u32::MAX,
            "render batch vertex count exceeded u32 index range; indices will be clamped"
        );
    }
    let base = base_len.min(u32::MAX as usize) as u32;
    out.vertices.extend_from_slice(&batch.vertices);
    out.indices.extend(batch.indices.iter().map(|index| {
        base.checked_add(*index).unwrap_or_else(|| {
            warn!(
                base,
                index,
                max_indexable_vertices = u32::MAX,
                "render batch index exceeded u32 range; clamping"
            );
            u32::MAX
        })
    }));
    (index_start, out.indices.len() - index_start)
}

fn append_pick_from_render(
    out: &mut PickBatch,
    occurrence: ShapeOccurrenceId,
    batch: &RenderBatch,
) {
    let requested_pick_id = out.occurrences.len() + 1;
    if requested_pick_id > u32::MAX as usize {
        warn!(
            pick_id = requested_pick_id,
            max_pick_id = u32::MAX,
            "pick batch occurrence count exceeded u32 pick id range; pick ids will be clamped"
        );
    }
    let pick_id = requested_pick_id.min(u32::MAX as usize) as u32;
    let base_len = out.vertices.len();
    if base_len > u32::MAX as usize {
        warn!(
            vertex_count = base_len,
            max_indexable_vertices = u32::MAX,
            "pick batch vertex count exceeded u32 index range; indices will be clamped"
        );
    }
    let base = base_len.min(u32::MAX as usize) as u32;
    out.occurrences.push(occurrence);
    out.vertices
        .extend(batch.vertices.iter().map(|vertex| PickVertex {
            position: vertex.position,
            pick_id,
        }));
    out.indices.extend(batch.indices.iter().map(|index| {
        base.checked_add(*index).unwrap_or_else(|| {
            warn!(
                base,
                index,
                max_indexable_vertices = u32::MAX,
                "pick batch index exceeded u32 range; clamping"
            );
            u32::MAX
        })
    }));
}

fn push_rect(batch: &mut RenderBatch, rect: Rect, color: [f32; 4]) {
    push_fan(batch, &rect.corners(), color);
}

fn push_fan(batch: &mut RenderBatch, points: &[Point], color: [f32; 4]) {
    if batch.vertices.len() > u32::MAX as usize - points.len() {
        warn!(
            vertex_count = batch.vertices.len(),
            added_points = points.len(),
            max_indexable_vertices = u32::MAX,
            "render fan would exceed u32 index range"
        );
        return;
    }
    let base = batch.vertices.len() as u32;
    for point in points {
        batch.vertices.push(GpuVertex {
            position: [point.x as f32, point.y as f32],
            color,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

fn push_transformed_fan(
    batch: &mut RenderBatch,
    points: &[Point],
    transform: Transform,
    color: [f32; 4],
) {
    if batch.vertices.len() > u32::MAX as usize - points.len() {
        warn!(
            vertex_count = batch.vertices.len(),
            added_points = points.len(),
            max_indexable_vertices = u32::MAX,
            "transformed render fan would exceed u32 index range"
        );
        return;
    }
    let base = batch.vertices.len() as u32;
    for point in points {
        let point = transform.apply_point(*point);
        batch.vertices.push(GpuVertex {
            position: [point.x as f32, point.y as f32],
            color,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

fn push_segment_as_rect(batch: &mut RenderBatch, a: Point, b: Point, width: i64, color: [f32; 4]) {
    if a.x == b.x {
        let half = width / 2;
        push_rect(
            batch,
            Rect::new(Point::new(a.x - half, a.y), Point::new(b.x + half, b.y)),
            color,
        );
    } else if a.y == b.y {
        let half = width / 2;
        push_rect(
            batch,
            Rect::new(Point::new(a.x, a.y - half), Point::new(b.x, b.y + half)),
            color,
        );
    }
}

fn push_pick_rect(batch: &mut PickBatch, rect: Rect, pick_id: u32) {
    push_pick_fan(batch, &rect.corners(), pick_id);
}

fn push_pick_fan(batch: &mut PickBatch, points: &[Point], pick_id: u32) {
    if batch.vertices.len() > u32::MAX as usize - points.len() {
        warn!(
            vertex_count = batch.vertices.len(),
            added_points = points.len(),
            max_indexable_vertices = u32::MAX,
            "pick fan would exceed u32 index range"
        );
        return;
    }
    let base = batch.vertices.len() as u32;
    for point in points {
        batch.vertices.push(PickVertex {
            position: [point.x as f32, point.y as f32],
            pick_id,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

fn push_transformed_pick_fan(
    batch: &mut PickBatch,
    points: &[Point],
    transform: Transform,
    pick_id: u32,
) {
    if batch.vertices.len() > u32::MAX as usize - points.len() {
        warn!(
            vertex_count = batch.vertices.len(),
            added_points = points.len(),
            max_indexable_vertices = u32::MAX,
            "transformed pick fan would exceed u32 index range"
        );
        return;
    }
    let base = batch.vertices.len() as u32;
    for point in points {
        let point = transform.apply_point(*point);
        batch.vertices.push(PickVertex {
            position: [point.x as f32, point.y as f32],
            pick_id,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

fn push_pick_segment_as_rect(batch: &mut PickBatch, a: Point, b: Point, width: i64, pick_id: u32) {
    if a.x == b.x {
        let half = width / 2;
        push_pick_rect(
            batch,
            Rect::new(Point::new(a.x - half, a.y), Point::new(b.x + half, b.y)),
            pick_id,
        );
    } else if a.y == b.y {
        let half = width / 2;
        push_pick_rect(
            batch,
            Rect::new(Point::new(a.x, a.y - half), Point::new(b.x, b.y + half)),
            pick_id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_model::{ProcessLayer, ShapeKind};

    #[test]
    fn tile_cache_deduplicates_shapes_spanning_tiles() {
        let mut document = Document::new("tile cache");
        let metal = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let id = document.insert_shape(
            metal,
            ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 200, 200)),
        );
        let index = LayoutIndex::rebuild(&document);
        let mut cache = TileCache::new(100);

        let frame = cache.build_frame(
            &document,
            &index,
            Rect::from_min_size(Point::new(0, 0), 199, 199),
            true,
        );

        assert_eq!(frame.stats.visible_shapes, 1);
        assert_eq!(frame.render.indices.len(), 6);
        let pick = frame.pick.unwrap();
        assert_eq!(pick.shape_for_pick_id(1), Some(id));
    }

    #[test]
    fn tile_cache_reuses_tiles_and_shape_geometry() {
        let document = Document::stress(100);
        let index = LayoutIndex::rebuild(&document);
        let viewport = Rect::from_min_size(Point::new(-600, -600), 1200, 1200);
        let mut cache = TileCache::new(256);

        let first = cache.build_frame(&document, &index, viewport, false);
        let second = cache.build_frame(&document, &index, viewport, false);

        assert!(first.stats.rebuilt_tiles > 0);
        assert_eq!(second.stats.rebuilt_tiles, 0);
        assert!(second.stats.shape_cache_hits > 0);
        assert_eq!(first.render.indices.len(), second.render.indices.len());
    }

    #[test]
    fn render_batch_fingerprint_tracks_gpu_buffer_contents() {
        let document = Document::stress(4);
        let viewport = Rect::from_min_size(Point::new(-600, -600), 1200, 1200);
        let first = build_layout_triangles(&document, viewport);
        let second = build_layout_triangles(&document, viewport);
        let mut changed = second.clone();
        changed.indices.reverse();

        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_ne!(first.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn render_batch_3d_fingerprint_tracks_depth_geometry() {
        let first = RenderBatch3d {
            vertices: vec![
                GpuVertex3d {
                    position: [0.0, 0.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                GpuVertex3d {
                    position: [10.0, 0.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                GpuVertex3d {
                    position: [0.0, 10.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            rect_slabs: vec![GpuRectSlabInstance {
                rect: [0.0, 0.0, 10.0, 10.0],
                z_range: [0.0, 20.0],
                color: [0.0, 1.0, 0.0, 1.0],
            }],
            guide_vertices: vec![
                GpuVertex3d {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 0.0],
                    color: [0.5, 0.5, 0.5, 0.5],
                },
                GpuVertex3d {
                    position: [10.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 0.0],
                    color: [0.5, 0.5, 0.5, 0.5],
                },
            ],
            guide_indices: vec![0, 1],
        };
        let mut changed = first.clone();
        changed.vertices[0].position[2] = 20.0;
        let mut changed_guide = first.clone();
        changed_guide.guide_vertices[0].position[0] = 5.0;
        let mut changed_normal = first.clone();
        changed_normal.vertices[0].normal = [1.0, 0.0, 0.0];
        let mut changed_rect = first.clone();
        changed_rect.rect_slabs[0].rect[2] = 12.0;

        assert_ne!(first.fingerprint(), changed.fingerprint());
        assert_ne!(first.fingerprint(), changed_guide.fingerprint());
        assert_ne!(first.fingerprint(), changed_normal.fingerprint());
        assert_ne!(first.fingerprint(), changed_rect.fingerprint());
        assert_eq!(
            first.estimate_bytes(),
            5 * std::mem::size_of::<GpuVertex3d>()
                + 5 * std::mem::size_of::<u32>()
                + std::mem::size_of::<GpuRectSlabInstance>()
        );
    }

    #[test]
    fn render_batch_3d_validation_accepts_valid_mesh_slabs_and_guides() {
        let batch = RenderBatch3d {
            vertices: vec![
                GpuVertex3d {
                    position: [0.0, 0.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                GpuVertex3d {
                    position: [10.0, 0.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                GpuVertex3d {
                    position: [0.0, 10.0, 10.0],
                    normal: [0.0, 0.0, 1.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            rect_slabs: vec![GpuRectSlabInstance {
                rect: [0.0, 0.0, 10.0, 10.0],
                z_range: [0.0, 20.0],
                color: [0.0, 1.0, 0.0, 1.0],
            }],
            guide_vertices: vec![
                GpuVertex3d {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 0.0],
                    color: [0.5, 0.5, 0.5, 0.5],
                },
                GpuVertex3d {
                    position: [10.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 0.0],
                    color: [0.5, 0.5, 0.5, 0.5],
                },
            ],
            guide_indices: vec![0, 1],
        };

        let validation = batch.validate_geometry().unwrap();

        assert_eq!(validation.mesh_vertices, 3);
        assert_eq!(validation.mesh_triangles, 1);
        assert_eq!(validation.rect_slabs, 1);
        assert_eq!(validation.guide_vertices, 2);
        assert_eq!(validation.guide_segments, 1);
    }

    #[test]
    fn render_batch_3d_validation_rejects_invalid_geometry() {
        let valid_vertex = GpuVertex3d {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        let valid_batch = RenderBatch3d {
            vertices: vec![
                valid_vertex,
                GpuVertex3d {
                    position: [10.0, 0.0, 0.0],
                    ..valid_vertex
                },
                GpuVertex3d {
                    position: [0.0, 10.0, 0.0],
                    ..valid_vertex
                },
            ],
            indices: vec![0, 1, 2],
            rect_slabs: vec![GpuRectSlabInstance {
                rect: [0.0, 0.0, 10.0, 10.0],
                z_range: [0.0, 10.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }],
            guide_vertices: Vec::new(),
            guide_indices: Vec::new(),
        };

        let mut bad_index = valid_batch.clone();
        bad_index.indices[2] = 42;
        assert!(matches!(
            bad_index.validate_geometry(),
            Err(RenderBatch3dValidationError::MeshIndexOutOfBounds { .. })
        ));

        let mut bad_triangle = valid_batch.clone();
        bad_triangle.vertices[2].position = [20.0, 0.0, 0.0];
        assert!(matches!(
            bad_triangle.validate_geometry(),
            Err(RenderBatch3dValidationError::DegenerateTriangle { .. })
        ));

        let mut bad_normal = valid_batch.clone();
        bad_normal.vertices[0].normal = [0.0, 0.0, 0.0];
        assert!(matches!(
            bad_normal.validate_geometry(),
            Err(RenderBatch3dValidationError::InvalidNormal { .. })
        ));

        let mut bad_slab = valid_batch;
        bad_slab.rect_slabs[0].rect = [10.0, 0.0, 0.0, 10.0];
        assert!(matches!(
            bad_slab.validate_geometry(),
            Err(RenderBatch3dValidationError::InvalidRectSlab { .. })
        ));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn viewport_3d_renderer_initializes_gpu_pipelines() {
        let Some((device, _queue)) = test_wgpu_device() else {
            return;
        };

        let _renderer = gpu::Viewport3dRenderer::new(&device, gpu::VIEWPORT_3D_COLOR_FORMAT);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn viewport_3d_renderer_draws_ten_k_instanced_slabs() {
        let Some((device, queue)) = test_wgpu_device() else {
            return;
        };
        let mut batch = RenderBatch3d::default();
        batch.rect_slabs.reserve(10_000);
        for y in 0..100 {
            for x in 0..100 {
                let min_x = -0.95 + x as f32 * 0.019;
                let min_y = -0.95 + y as f32 * 0.019;
                batch.rect_slabs.push(GpuRectSlabInstance {
                    rect: [min_x, min_y, min_x + 0.012, min_y + 0.012],
                    z_range: [0.2, 0.3],
                    color: [0.25, 0.65, 1.0, 1.0],
                });
            }
        }
        let fingerprint = batch.fingerprint();
        let uniforms = gpu::Viewport3dUniforms::from_view_projection([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]);
        let mut renderer = gpu::Viewport3dRenderer::new(&device, gpu::VIEWPORT_3D_COLOR_FORMAT);

        let upload_started = std::time::Instant::now();
        let first_upload =
            renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, uniforms);
        let first_upload_ms = upload_started.elapsed().as_secs_f64() * 1000.0;
        let skipped_upload_started = std::time::Instant::now();
        let second_upload =
            renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, uniforms);
        let skipped_upload_ms = skipped_upload_started.elapsed().as_secs_f64() * 1000.0;
        assert!(first_upload.uploaded);
        assert!(second_upload.skipped);
        assert_eq!(second_upload.bytes_uploaded, 0);

        const TEST_WIDTH: u32 = 1634;
        const TEST_HEIGHT: u32 = 1705;
        let composite_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Fabricad 10k 3D renderer test composite target"),
            size: wgpu::Extent3d {
                width: TEST_WIDTH,
                height: TEST_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::VIEWPORT_3D_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let composite_view = composite_texture.create_view(&Default::default());
        let (warmup_encode_ms, warmup_submit_wait_ms) = submit_3d_test_frame(
            &device,
            &queue,
            &mut renderer,
            &composite_view,
            [TEST_WIDTH, TEST_HEIGHT],
        );
        let mut encode_samples = Vec::with_capacity(12);
        let mut submit_wait_samples = Vec::with_capacity(12);
        let mut scene_submit_wait_samples = Vec::with_capacity(6);
        let mut composite_submit_wait_samples = Vec::with_capacity(6);
        for _ in 0..12 {
            let (encode_ms, submit_wait_ms) = submit_3d_test_frame(
                &device,
                &queue,
                &mut renderer,
                &composite_view,
                [TEST_WIDTH, TEST_HEIGHT],
            );
            encode_samples.push(encode_ms);
            submit_wait_samples.push(submit_wait_ms);
        }
        for _ in 0..6 {
            let (_encode_ms, submit_wait_ms) = submit_3d_test_scene_only(
                &device,
                &queue,
                &mut renderer,
                [TEST_WIDTH, TEST_HEIGHT],
            );
            scene_submit_wait_samples.push(submit_wait_ms);
        }
        for _ in 0..6 {
            let (_encode_ms, submit_wait_ms) =
                submit_3d_test_composite_only(&device, &queue, &renderer, &composite_view);
            composite_submit_wait_samples.push(submit_wait_ms);
        }
        let encode_avg = average(&encode_samples);
        let submit_wait_avg = average(&submit_wait_samples);
        let submit_wait_p50 = percentile(&submit_wait_samples, 0.50);
        let submit_wait_p95 = percentile(&submit_wait_samples, 0.95);
        let scene_submit_wait_p50 = percentile(&scene_submit_wait_samples, 0.50);
        let composite_submit_wait_p50 = percentile(&composite_submit_wait_samples, 0.50);
        eprintln!(
            "10k 3D instanced slabs {TEST_WIDTH}x{TEST_HEIGHT}: upload={first_upload_ms:.3}ms skipped_upload={skipped_upload_ms:.3}ms warmup_encode={warmup_encode_ms:.3}ms warmup_submit_wait={warmup_submit_wait_ms:.3}ms encode_avg={encode_avg:.3}ms submit_wait_avg={submit_wait_avg:.3}ms submit_wait_p50={submit_wait_p50:.3}ms submit_wait_p95={submit_wait_p95:.3}ms scene_submit_wait_p50={scene_submit_wait_p50:.3}ms composite_submit_wait_p50={composite_submit_wait_p50:.3}ms"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn viewport_3d_renderer_screenshot_tracks_camera_facing_slab_side() {
        let Some((device, queue)) = test_wgpu_device() else {
            return;
        };
        let mut batch = RenderBatch3d::default();
        batch.rect_slabs.push(GpuRectSlabInstance {
            rect: [-1.0, -1.0, 1.0, 1.0],
            z_range: [0.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        });
        let fingerprint = batch.fingerprint();
        let front_uniforms = gpu::Viewport3dUniforms::from_view_projection(
            view_projection_3d_test([0.0, -4.0, 0.5], [0.0, 0.0, 0.5], 1.0),
        )
        .with_rect_side_faces([4, 1]);
        let mut renderer = gpu::Viewport3dRenderer::new(&device, gpu::VIEWPORT_3D_COLOR_FORMAT);
        renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, front_uniforms);

        let pixels = render_3d_screenshot_pixels(&device, &queue, &mut renderer, [96, 96]);
        let center = pixels.pixel(48, 48);
        let expected_negative_y_side = (0.4543_f32 * 255.0).round() as u8;
        let expected_positive_y_side = (0.3796_f32 * 255.0).round() as u8;

        assert!(
            center[0].abs_diff(expected_negative_y_side) <= 3
                && center[1].abs_diff(expected_negative_y_side) <= 3
                && center[2].abs_diff(expected_negative_y_side) <= 3,
            "center pixel should show the camera-facing -Y slab side; center={center:?} expected~{expected_negative_y_side} far_side~{expected_positive_y_side}"
        );
        assert!(
            center[0].abs_diff(expected_positive_y_side) > 8
                || center[1].abs_diff(expected_positive_y_side) > 8
                || center[2].abs_diff(expected_positive_y_side) > 8,
            "center pixel matched the far +Y slab side instead of the camera-facing side: {center:?}"
        );

        let back_uniforms = gpu::Viewport3dUniforms::from_view_projection(view_projection_3d_test(
            [0.0, 4.0, 0.5],
            [0.0, 0.0, 0.5],
            1.0,
        ))
        .with_rect_side_faces([4, 3]);
        let upload =
            renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, back_uniforms);
        assert!(upload.skipped);

        let pixels = render_3d_screenshot_pixels(&device, &queue, &mut renderer, [96, 96]);
        let center = pixels.pixel(48, 48);
        assert!(
            center[0].abs_diff(expected_positive_y_side) <= 3
                && center[1].abs_diff(expected_positive_y_side) <= 3
                && center[2].abs_diff(expected_positive_y_side) <= 3,
            "center pixel should update to the camera-facing +Y slab side; center={center:?} expected~{expected_positive_y_side} far_side~{expected_negative_y_side}"
        );
        assert!(
            center[0].abs_diff(expected_negative_y_side) > 8
                || center[1].abs_diff(expected_negative_y_side) > 8
                || center[2].abs_diff(expected_negative_y_side) > 8,
            "center pixel stayed on the stale -Y slab side after the camera moved: {center:?}"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn viewport_3d_renderer_screenshot_keeps_mesh_above_rect_slabs() {
        let Some((device, queue)) = test_wgpu_device() else {
            return;
        };
        let mut batch = RenderBatch3d::default();
        batch.rect_slabs.push(GpuRectSlabInstance {
            rect: [-1.0, -1.0, 1.0, 1.0],
            z_range: [0.0, 0.5],
            color: [0.0, 1.0, 0.0, 1.0],
        });
        batch.vertices.extend([
            GpuVertex3d {
                position: [-0.55, -0.55, 1.1],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            GpuVertex3d {
                position: [-0.55, 0.55, 1.1],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            GpuVertex3d {
                position: [0.55, 0.55, 1.1],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            GpuVertex3d {
                position: [0.55, -0.55, 1.1],
                normal: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ]);
        batch.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        let fingerprint = batch.fingerprint();
        let uniforms = gpu::Viewport3dUniforms::from_view_projection(view_projection_3d_test(
            [0.0, 0.0, 5.0],
            [0.0, 0.0, 0.5],
            1.0,
        ));
        let mut renderer = gpu::Viewport3dRenderer::new(&device, gpu::VIEWPORT_3D_COLOR_FORMAT);
        renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, uniforms);

        let pixels = render_3d_screenshot_pixels(&device, &queue, &mut renderer, [96, 96]);
        let center = pixels.pixel(48, 48);
        assert!(
            center[2] > 180 && center[1] < 80,
            "center pixel should show the elevated blue mesh, not the lower green slab: {center:?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_3d_test_frame(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut gpu::Viewport3dRenderer,
        composite_view: &wgpu::TextureView,
        target_size: [u32; 2],
    ) -> (f64, f64) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fabricad 10k 3D renderer test encoder"),
        });
        let encode_started = std::time::Instant::now();
        renderer.render_to_texture(
            device,
            &mut encoder,
            target_size,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        {
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Fabricad 10k 3D renderer test composite pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: composite_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            renderer.paint(&mut render_pass);
        }
        let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
        let submit_started = std::time::Instant::now();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("10k 3D renderer test submission should complete");
        let submit_wait_ms = submit_started.elapsed().as_secs_f64() * 1000.0;
        (encode_ms, submit_wait_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_3d_test_scene_only(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut gpu::Viewport3dRenderer,
        target_size: [u32; 2],
    ) -> (f64, f64) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fabricad 10k 3D renderer scene-only test encoder"),
        });
        let encode_started = std::time::Instant::now();
        renderer.render_to_texture(
            device,
            &mut encoder,
            target_size,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
        let submit_started = std::time::Instant::now();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("10k 3D renderer scene-only test submission should complete");
        let submit_wait_ms = submit_started.elapsed().as_secs_f64() * 1000.0;
        (encode_ms, submit_wait_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_3d_test_composite_only(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &gpu::Viewport3dRenderer,
        composite_view: &wgpu::TextureView,
    ) -> (f64, f64) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fabricad 10k 3D renderer composite-only test encoder"),
        });
        let encode_started = std::time::Instant::now();
        {
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Fabricad 10k 3D renderer composite-only test pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: composite_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            renderer.paint(&mut render_pass);
        }
        let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
        let submit_started = std::time::Instant::now();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("10k 3D renderer composite-only test submission should complete");
        let submit_wait_ms = submit_started.elapsed().as_secs_f64() * 1000.0;
        (encode_ms, submit_wait_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    struct TestPixels {
        bytes_per_row: u32,
        data: Vec<u8>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl TestPixels {
        fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
            let offset = (y * self.bytes_per_row + x * 4) as usize;
            [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_3d_screenshot_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut gpu::Viewport3dRenderer,
        target_size: [u32; 2],
    ) -> TestPixels {
        let width = target_size[0];
        let height = target_size[1];
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Fabricad 3D screenshot test target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu::VIEWPORT_3D_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bytes_per_row = align_to_test(width * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fabricad 3D screenshot test readback"),
            size: bytes_per_row as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fabricad 3D screenshot test encoder"),
        });
        renderer.render_to_texture(
            device,
            &mut encoder,
            target_size,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        {
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Fabricad 3D screenshot test composite pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            renderer.paint(&mut render_pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("3D screenshot test submission should complete");

        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result.map_err(|err| err.to_string()))
                    .expect("3D screenshot test readback receiver should exist");
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("3D screenshot test readback should complete");
        rx.recv_timeout(std::time::Duration::from_secs(30))
            .expect("3D screenshot test readback should report")
            .expect("3D screenshot test readback should map");
        let data = readback.slice(..).get_mapped_range().to_vec();
        readback.unmap();
        TestPixels {
            bytes_per_row,
            data,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn view_projection_3d_test(position: [f32; 3], target: [f32; 3], aspect: f32) -> [f32; 16] {
        let position = Vec3Test::new(position[0], position[1], position[2]);
        let target = Vec3Test::new(target[0], target[1], target[2]);
        let forward = (target - position).normalized();
        let yaw = forward.y.atan2(forward.x);
        let right = Vec3Test::new(-yaw.sin(), yaw.cos(), 0.0);
        let up = forward.cross(right).normalized();
        let y_scale = 1.0 / (58.0_f32.to_radians() * 0.5).tan();
        let x_scale = y_scale / aspect.max(0.001);
        let near = 0.01;
        let far = 100.0;
        let z_scale = far / (far - near);
        let z_bias = -near * far / (far - near);
        row_major_4x4_to_column_major_test([
            [
                right.x * x_scale,
                right.y * x_scale,
                right.z * x_scale,
                -position.dot(right) * x_scale,
            ],
            [
                up.x * y_scale,
                up.y * y_scale,
                up.z * y_scale,
                -position.dot(up) * y_scale,
            ],
            [
                forward.x * z_scale,
                forward.y * z_scale,
                forward.z * z_scale,
                -position.dot(forward) * z_scale + z_bias,
            ],
            [forward.x, forward.y, forward.z, -position.dot(forward)],
        ])
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[derive(Clone, Copy)]
    struct Vec3Test {
        x: f32,
        y: f32,
        z: f32,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl Vec3Test {
        fn new(x: f32, y: f32, z: f32) -> Self {
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

        fn normalized(self) -> Self {
            let length = self.dot(self).sqrt();
            Self::new(self.x / length, self.y / length, self.z / length)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl std::ops::Sub for Vec3Test {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self::Output {
            Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn row_major_4x4_to_column_major_test(rows: [[f32; 4]; 4]) -> [f32; 16] {
        [
            rows[0][0], rows[1][0], rows[2][0], rows[3][0], rows[0][1], rows[1][1], rows[2][1],
            rows[3][1], rows[0][2], rows[1][2], rows[2][2], rows[3][2], rows[0][3], rows[1][3],
            rows[2][3], rows[3][3],
        ]
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn align_to_test(value: u32, alignment: u32) -> u32 {
        value.div_ceil(alignment) * alignment
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn average(values: &[f64]) -> f64 {
        values.iter().sum::<f64>() / values.len().max(1) as f64
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn percentile(values: &[f64], percentile: f64) -> f64 {
        if values.is_empty() {
            return f64::NAN;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let index = ((sorted.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
        sorted[index]
    }

    #[test]
    fn pick_batch_fingerprint_tracks_pick_ids() {
        let document = Document::stress(4);
        let viewport = Rect::from_min_size(Point::new(-600, -600), 1200, 1200);
        let first = build_pick_triangles(&document, viewport);
        let mut changed = first.clone();
        changed.vertices[0].pick_id += 1;

        assert_ne!(first.gpu_fingerprint(), changed.gpu_fingerprint());
    }

    #[test]
    fn tile_cache_invalidates_only_intersecting_tiles() {
        let document = Document::stress(100);
        let index = LayoutIndex::rebuild(&document);
        let viewport = Rect::from_min_size(Point::new(-600, -600), 1200, 1200);
        let mut cache = TileCache::new(256);
        let first = cache.build_frame(&document, &index, viewport, false);

        let removed = cache.invalidate_rect(Rect::from_min_size(Point::new(0, 0), 1, 1));
        let second = cache.build_frame(&document, &index, viewport, false);

        assert!(removed > 0);
        assert!(removed < first.stats.cached_tiles);
        assert_eq!(second.stats.rebuilt_tiles, removed);
    }

    #[test]
    fn tile_cache_evicts_old_tiles_to_memory_budget() {
        let document = Document::stress(20_000);
        let index = LayoutIndex::rebuild(&document);
        let first_view = Rect::from_min_size(Point::new(-8_000, -8_000), 4_000, 4_000);
        let second_view = Rect::from_min_size(Point::new(4_000, 4_000), 4_000, 4_000);
        let mut cache = TileCache::new(512);

        let first = cache.build_frame_with_options(
            &document,
            &index,
            first_view,
            TileFrameOptions {
                include_pick: false,
                zoom: 0.5,
                memory_budget_bytes: None,
                ..Default::default()
            },
        );
        let second = cache.build_frame_with_options(
            &document,
            &index,
            second_view,
            TileFrameOptions {
                include_pick: false,
                zoom: 0.5,
                memory_budget_bytes: Some(1),
                ..Default::default()
            },
        );

        assert!(first.stats.cached_tiles > 0);
        assert!(second.stats.evicted_tiles > 0);
        assert_eq!(second.stats.memory_budget_bytes, Some(1));
        assert_eq!(second.stats.resident_tiles, second.stats.visible_tiles);
        assert!(second.stats.over_budget_bytes > 0);
    }

    #[test]
    fn pick_batch_is_limited_to_visible_tile_occurrences() {
        let document = Document::stress(5_000);
        let index = LayoutIndex::rebuild(&document);
        let viewport = Rect::from_min_size(Point::new(-500, -500), 1_000, 1_000);
        let mut cache = TileCache::new(512);

        let frame = cache.build_frame_with_options(
            &document,
            &index,
            viewport,
            TileFrameOptions {
                include_pick: true,
                zoom: 1.0,
                ..Default::default()
            },
        );
        let pick = frame.pick.unwrap();

        assert!(frame.stats.pick_shapes > 0);
        assert!(frame.stats.pick_shapes < document.shapes.len());
        assert_eq!(pick.occurrences.len(), frame.stats.pick_shapes);
        assert!(frame.stats.pick_build_ms >= 0.0);
    }

    #[test]
    fn tile_cache_uses_overview_lod_for_dense_far_zoom_tiles() {
        let document = Document::stress(1_000);
        let index = LayoutIndex::rebuild(&document);
        let viewport = Rect::from_min_size(Point::new(-6_000, -6_000), 12_000, 12_000);
        let lod = TileLodConfig {
            enabled: true,
            max_tile_screen_px: 220.0,
            min_shapes_per_tile: 8,
        };
        let mut lod_cache = TileCache::new(1_024);
        let mut precise_cache = TileCache::new(1_024);

        let lod_frame = lod_cache.build_frame_with_options(
            &document,
            &index,
            viewport,
            TileFrameOptions {
                include_pick: false,
                zoom: 0.1,
                lod,
                ..Default::default()
            },
        );
        let precise_frame = precise_cache.build_frame_with_options(
            &document,
            &index,
            viewport,
            TileFrameOptions {
                include_pick: false,
                zoom: 0.1,
                lod: TileLodConfig {
                    enabled: false,
                    ..lod
                },
                ..Default::default()
            },
        );

        assert!(lod_frame.stats.lod_tiles > 0);
        assert!(lod_frame.stats.lod_shapes > 0);
        assert!(lod_frame.render.indices.len() < precise_frame.render.indices.len());
        assert!(
            lod_frame
                .draw_ranges
                .iter()
                .any(|range| matches!(range.kind, DrawBatchRangeKind::TileOverview { .. }))
        );
    }

    #[test]
    fn tile_cache_keeps_precise_geometry_at_close_zoom() {
        let document = Document::stress(1_000);
        let index = LayoutIndex::rebuild(&document);
        let viewport = Rect::from_min_size(Point::new(-6_000, -6_000), 12_000, 12_000);
        let mut cache = TileCache::new(1_024);
        let frame = cache.build_frame_with_options(
            &document,
            &index,
            viewport,
            TileFrameOptions {
                include_pick: false,
                zoom: 1.0,
                lod: TileLodConfig {
                    enabled: true,
                    max_tile_screen_px: 220.0,
                    min_shapes_per_tile: 1,
                },
                ..Default::default()
            },
        );

        assert_eq!(frame.stats.lod_tiles, 0);
        assert_eq!(frame.stats.precise_shapes, frame.stats.visible_shapes);
        assert_eq!(frame.stats.draw_ranges, frame.stats.visible_shapes);
        assert!(
            frame
                .draw_ranges
                .iter()
                .all(|range| matches!(range.kind, DrawBatchRangeKind::Shape { .. }))
        );
    }

    #[test]
    fn tile_cache_renders_repeated_hierarchy_instances_as_distinct_occurrences() {
        let mut document = Document::new("hierarchy render");
        let metal = document.layer_by_process(ProcessLayer::Metal1).unwrap();
        let child = document.create_cell("unit");
        let shape = document
            .insert_shape_in_cell(
                child,
                metal,
                ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 100, 100)),
            )
            .unwrap();
        document
            .insert_instance_in_top(child, layout_model::Transform::translate(0, 0))
            .unwrap();
        document
            .insert_instance_in_top(child, layout_model::Transform::translate(300, 0))
            .unwrap();
        let index = LayoutIndex::rebuild_hierarchical(&document);
        let mut cache = TileCache::new(1_024);

        let frame = cache.build_frame(
            &document,
            &index,
            Rect::from_min_size(Point::new(-50, -50), 500, 200),
            true,
        );
        let pick = frame.pick.unwrap();

        assert_eq!(frame.stats.visible_shapes, 2);
        assert_eq!(frame.render.indices.len(), 12);
        assert_eq!(pick.occurrences.len(), 2);
        assert_eq!(pick.shape_for_pick_id(1), Some(shape));
        assert_ne!(pick.occurrences[0], pick.occurrences[1]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn test_wgpu_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            backend_options: wgpu::BackendOptions::from_env_or_default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })) {
                Ok(adapter) => adapter,
                Err(err) => {
                    eprintln!("skipping WGPU renderer test: no GPU adapter: {err}");
                    return None;
                }
            };
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Fabricad renderer test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })) {
            Ok(device) => Some(device),
            Err(err) => {
                eprintln!("skipping WGPU renderer test: no GPU device: {err}");
                None
            }
        }
    }
}
