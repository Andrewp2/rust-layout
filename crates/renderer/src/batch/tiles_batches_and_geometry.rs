#![allow(unused_imports)]
use super::*;

pub const DEFAULT_TILE_SIZE: i64 = 16_384;
pub const DEFAULT_LOD_MAX_TILE_SCREEN_PX: f32 = 220.0;
pub const DEFAULT_LOD_MIN_SHAPES_PER_TILE: usize = 512;
pub const DEFAULT_LOD_EXTREME_MAX_TILE_SCREEN_PX: f32 = 1_536.0;
pub const DEFAULT_LOD_EXTREME_MIN_SHAPES_PER_TILE: usize = 20_000;

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
    pub root_cell: Option<CellId>,
    pub zoom: f32,
    pub lod: TileLodConfig,
    pub memory_budget_bytes: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct TileLodConfig {
    pub enabled: bool,
    pub max_tile_screen_px: f32,
    pub min_shapes_per_tile: usize,
    pub extreme_max_tile_screen_px: f32,
    pub extreme_min_shapes_per_tile: usize,
}

#[derive(Clone, Debug)]
pub struct TileCache {
    pub(crate) tile_size: i64,
    pub(crate) root_cell: Option<CellId>,
    pub(crate) tiles: BTreeMap<TileKey, CachedTile>,
    pub(crate) shapes: BTreeMap<ShapeOccurrenceId, RenderBatch>,
    pub(crate) frame_counter: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct CachedTile {
    pub(crate) occurrences: Vec<ShapeOccurrenceId>,
    pub(crate) overview: Option<OverviewTile>,
    pub(crate) last_used_frame: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct OverviewTile {
    pub(crate) batch: RenderBatch,
    pub(crate) shape_count: usize,
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
            root_cell: None,
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
            extreme_max_tile_screen_px: DEFAULT_LOD_EXTREME_MAX_TILE_SCREEN_PX,
            extreme_min_shapes_per_tile: DEFAULT_LOD_EXTREME_MIN_SHAPES_PER_TILE,
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

pub(crate) fn validate_3d_vertices(
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

pub(crate) fn validate_rect_slab_instance(
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

pub(crate) fn triangle_area2_3d(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    vector_length2_3d(cross)
}

pub(crate) fn vector_length2_3d(vector: [f32; 3]) -> f32 {
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
            root_cell: None,
            tiles: BTreeMap::new(),
            shapes: BTreeMap::new(),
            frame_counter: 0,
        }
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.shapes.clear();
        self.frame_counter = 0;
        self.root_cell = None;
    }

    pub fn tile_keys_for_viewport(&self, viewport: Rect) -> Vec<TileKey> {
        tile_keys_for_rect(viewport, self.tile_size)
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
        let root_cell = options.root_cell.unwrap_or(document.top_cell);
        if self.root_cell != Some(root_cell) {
            self.tiles.clear();
            self.shapes.clear();
            self.frame_counter = 0;
            self.root_cell = Some(root_cell);
        }
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
                    build_overview_tile(document, root_cell, tile_bounds, &tile.occurrences)
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
            let Some(shape) = document.shape_view_for_occurrence_from_cell(root_cell, &id) else {
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
                let Some(shape) = document.shape_view_for_occurrence_from_cell(root_cell, &id)
                else {
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

    pub(crate) fn evict_to_budget(
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
pub(crate) struct EvictionStats {
    pub(crate) tiles: usize,
    pub(crate) shape_batches: usize,
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

pub(crate) fn should_use_overview(
    tile_size: i64,
    shape_count: usize,
    options: TileFrameOptions,
) -> bool {
    if !options.lod.enabled {
        return false;
    }
    let tile_screen_px = tile_size as f32 * options.zoom;
    (shape_count >= options.lod.min_shapes_per_tile
        && tile_screen_px <= options.lod.max_tile_screen_px)
        || (shape_count >= options.lod.extreme_min_shapes_per_tile
            && tile_screen_px <= options.lod.extreme_max_tile_screen_px)
}

pub(crate) fn tile_keys_for_rect(rect: Rect, tile_size: i64) -> Vec<TileKey> {
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

pub(crate) fn build_overview_tile(
    document: &Document,
    root_cell: CellId,
    tile_bounds: Rect,
    occurrences: &[ShapeOccurrenceId],
) -> OverviewTile {
    let mut layers: BTreeMap<LayerId, (Rect, usize)> = BTreeMap::new();
    for id in occurrences {
        let Some(shape) = document.shape_view_for_occurrence_from_cell(root_cell, id) else {
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
        let mut color = document.layer_display_color(layer);
        color[3] = color[3].min((0.16 + (layer_shape_count as f32).ln_1p() * 0.08).min(0.68));
        push_rect(&mut batch, bounds, color);
        shape_count += layer_shape_count;
    }
    OverviewTile { batch, shape_count }
}

pub(crate) fn is_geometry_shape_view(kind: ShapeKindView<'_>) -> bool {
    !matches!(
        kind,
        ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. }
    )
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LayerRenderStyle {
    pub(crate) fill_style: LayerFillStyle,
    pub(crate) line_style: LayerLineStyle,
    pub(crate) fill_color: [f32; 4],
    pub(crate) stroke_color: [f32; 4],
}

pub(crate) fn layer_render_style(document: &Document, layer: LayerId) -> LayerRenderStyle {
    let fill_color = document.layer_display_color(layer);
    let Some(layer) = document.layer(layer) else {
        return LayerRenderStyle {
            fill_style: LayerFillStyle::Solid,
            line_style: LayerLineStyle::Solid,
            fill_color,
            stroke_color: fill_color,
        };
    };
    let mut stroke_color = layer.color;
    stroke_color[3] = stroke_color[3].max(0.72);
    LayerRenderStyle {
        fill_style: layer.fill_style,
        line_style: layer.line_style,
        fill_color,
        stroke_color,
    }
}

pub(crate) fn append_shape_triangles(batch: &mut RenderBatch, document: &Document, shape: &Shape) {
    let style = layer_render_style(document, shape.layer);
    match &shape.kind {
        ShapeKind::Rectangle(rect) => append_region_triangles(batch, &rect.corners(), style),
        ShapeKind::Polygon(poly) => {
            append_region_triangles(batch, &poly.points, style);
        }
        ShapeKind::Path { points, width } => {
            append_path_triangles(batch, points, *width, style);
        }
        ShapeKind::Via { center, size, .. } => {
            let half = *size / 2;
            let rect = Rect::new(
                Point::new(center.x - half, center.y - half),
                Point::new(center.x + half, center.y + half),
            );
            append_region_triangles(batch, &rect.corners(), style);
        }
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {}
    }
}

pub(crate) fn append_shape_view_triangles(
    batch: &mut RenderBatch,
    document: &Document,
    shape: ShapeView<'_>,
    transform: Transform,
) {
    let style = layer_render_style(document, shape.layer);
    match shape.kind {
        ShapeKindView::Rectangle(rect) => {
            let corners = transformed_points(&rect.corners(), transform);
            append_region_triangles_from_f32(batch, &corners, style);
        }
        ShapeKindView::Polygon(poly) => {
            let points = transformed_points(&poly.points, transform);
            append_region_triangles_from_f32(batch, &points, style);
        }
        ShapeKindView::Path { points, width } => {
            let points = transformed_points(points, transform);
            append_path_triangles_from_f32(batch, &points, width as f32, style);
        }
        ShapeKindView::Via { center, size, .. } => {
            let center = transform.apply_point(center);
            let half = size / 2;
            let rect = Rect::new(
                Point::new(center.x - half, center.y - half),
                Point::new(center.x + half, center.y + half),
            );
            append_region_triangles(batch, &rect.corners(), style);
        }
        ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. } => {}
    }
}

pub(crate) fn append_region_triangles(
    batch: &mut RenderBatch,
    points: &[Point],
    style: LayerRenderStyle,
) {
    let points = points
        .iter()
        .map(|point| [point.x as f32, point.y as f32])
        .collect::<Vec<_>>();
    append_region_triangles_from_f32(batch, &points, style);
}

pub(crate) fn append_region_triangles_from_f32(
    batch: &mut RenderBatch,
    points: &[[f32; 2]],
    style: LayerRenderStyle,
) {
    if points.len() < 3 {
        return;
    }
    push_fan_f32(batch, points, style.fill_color);
    match style.fill_style {
        LayerFillStyle::Hatched => {
            push_region_hatching(
                batch,
                points,
                outline_width(points),
                style.stroke_color,
                false,
            );
        }
        LayerFillStyle::CrossHatched => {
            let width = outline_width(points);
            push_region_hatching(batch, points, width, style.stroke_color, false);
            push_region_hatching(batch, points, width, style.stroke_color, true);
        }
        LayerFillStyle::Stippled => {
            push_region_stipple(
                batch,
                points,
                outline_width(points),
                style.stroke_color,
                8.0,
                1.8,
            );
        }
        LayerFillStyle::DenseStippled => {
            push_region_stipple(
                batch,
                points,
                outline_width(points),
                style.stroke_color,
                5.0,
                1.4,
            );
        }
        LayerFillStyle::SparseStippled => {
            push_region_stipple(
                batch,
                points,
                outline_width(points),
                style.stroke_color,
                13.0,
                2.0,
            );
        }
        LayerFillStyle::Solid | LayerFillStyle::Outline => {}
    }
    if style.fill_style == LayerFillStyle::Outline || style.line_style != LayerLineStyle::Solid {
        push_region_outline(batch, points, style);
    }
}

pub(crate) fn append_path_triangles(
    batch: &mut RenderBatch,
    points: &[Point],
    width: i64,
    style: LayerRenderStyle,
) {
    let points = points
        .iter()
        .map(|point| [point.x as f32, point.y as f32])
        .collect::<Vec<_>>();
    append_path_triangles_from_f32(batch, &points, width as f32, style);
}

pub(crate) fn append_path_triangles_from_f32(
    batch: &mut RenderBatch,
    points: &[[f32; 2]],
    width: f32,
    style: LayerRenderStyle,
) {
    let width = width.max(1.0);
    if style.line_style == LayerLineStyle::Solid {
        for window in points.windows(2) {
            push_segment_quad(batch, window[0], window[1], width, style.fill_color);
        }
    } else {
        for window in points.windows(2) {
            push_styled_segment(
                batch,
                window[0],
                window[1],
                width,
                style.stroke_color,
                style,
            );
        }
    }
}

pub(crate) fn push_region_outline(
    batch: &mut RenderBatch,
    points: &[[f32; 2]],
    style: LayerRenderStyle,
) {
    let width = outline_width(points);
    for index in 0..points.len() {
        push_styled_segment(
            batch,
            points[index],
            points[(index + 1) % points.len()],
            width,
            style.stroke_color,
            style,
        );
    }
}

pub(crate) fn append_render_batch_with_range(
    out: &mut RenderBatch,
    batch: &RenderBatch,
) -> (usize, usize) {
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

pub(crate) fn append_pick_from_render(
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

pub(crate) fn push_rect(batch: &mut RenderBatch, rect: Rect, color: [f32; 4]) {
    push_fan(batch, &rect.corners(), color);
}

pub(crate) fn push_fan(batch: &mut RenderBatch, points: &[Point], color: [f32; 4]) {
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

pub(crate) fn transformed_points(points: &[Point], transform: Transform) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|point| {
            let point = transform.apply_point(*point);
            [point.x as f32, point.y as f32]
        })
        .collect()
}

pub(crate) fn push_fan_f32(batch: &mut RenderBatch, points: &[[f32; 2]], color: [f32; 4]) {
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
            position: *point,
            color,
        });
    }
    for index in 1..points.len().saturating_sub(1) {
        batch
            .indices
            .extend_from_slice(&[base, base + index as u32, base + index as u32 + 1]);
    }
}

pub(crate) fn outline_width(points: &[[f32; 2]]) -> f32 {
    let Some(first) = points.first() else {
        return 1.0;
    };
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first[0], first[0], first[1], first[1]);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    ((max_x - min_x).abs().min((max_y - min_y).abs()) * 0.025).clamp(1.0, 12.0)
}

pub(crate) fn push_region_hatching(
    batch: &mut RenderBatch,
    points: &[[f32; 2]],
    width: f32,
    mut color: [f32; 4],
    reverse: bool,
) {
    let Some((min_x, max_x, min_y, max_y)) = axis_aligned_rect_bounds(points) else {
        return;
    };
    let span = (max_x - min_x).abs().max((max_y - min_y).abs());
    if span <= f32::EPSILON {
        return;
    }
    color[3] = color[3].min(0.46);
    let spacing = (width * 6.0).clamp(8.0, 96.0);
    let mut offset = -span;
    while offset <= span * 2.0 {
        let (a, b) = if reverse {
            ([min_x + offset, max_y], [min_x + offset + span, min_y])
        } else {
            ([min_x + offset, min_y], [min_x + offset + span, max_y])
        };
        push_segment_quad(batch, a, b, width.max(1.0), color);
        offset += spacing;
    }
}

pub(crate) fn push_region_stipple(
    batch: &mut RenderBatch,
    points: &[[f32; 2]],
    width: f32,
    mut color: [f32; 4],
    spacing_widths: f32,
    dot_widths: f32,
) {
    let Some((min_x, max_x, min_y, max_y)) = axis_aligned_rect_bounds(points) else {
        return;
    };
    let rect_width = max_x - min_x;
    let rect_height = max_y - min_y;
    if rect_width <= f32::EPSILON || rect_height <= f32::EPSILON {
        return;
    }
    let spacing = (width * spacing_widths).clamp(8.0, 144.0);
    let columns = ((rect_width / spacing).ceil() as usize).clamp(1, 96);
    let rows = ((rect_height / spacing).ceil() as usize).clamp(1, 96);
    let step_x = rect_width / columns as f32;
    let step_y = rect_height / rows as f32;
    let dot = (width * dot_widths)
        .clamp(1.5, 10.0)
        .min(rect_width.min(rect_height) * 0.35);
    if dot <= f32::EPSILON {
        return;
    }
    let half = dot * 0.5;
    color[3] = color[3].min(0.48);
    for row in 0..rows {
        let y = min_y + step_y * (row as f32 + 0.5);
        for column in 0..columns {
            let mut x = min_x + step_x * (column as f32 + 0.5);
            if row % 2 == 1 {
                x += step_x * 0.35;
            }
            if x - half < min_x || x + half > max_x || y - half < min_y || y + half > max_y {
                continue;
            }
            push_rect_f32(batch, x - half, x + half, y - half, y + half, color);
        }
    }
}

pub(crate) fn push_rect_f32(
    batch: &mut RenderBatch,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    color: [f32; 4],
) {
    let points = [
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    push_fan_f32(batch, &points, color);
}

pub(crate) fn axis_aligned_rect_bounds(points: &[[f32; 2]]) -> Option<(f32, f32, f32, f32)> {
    if points.len() != 4 {
        return None;
    }
    let first = points[0];
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first[0], first[0], first[1], first[1]);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    let corners = [
        [min_x, min_y],
        [max_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
    ];
    points
        .iter()
        .all(|point| corners.contains(point))
        .then_some((min_x, max_x, min_y, max_y))
}

pub(crate) fn push_styled_segment(
    batch: &mut RenderBatch,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
    style: LayerRenderStyle,
) {
    match style.line_style {
        LayerLineStyle::Solid => push_segment_quad(batch, a, b, width, color),
        LayerLineStyle::Dashed => push_segment_pattern(batch, a, b, width, color, 6.0, 3.5),
        LayerLineStyle::Dotted => push_segment_pattern(batch, a, b, width, color, 1.2, 3.0),
        LayerLineStyle::DashDot => push_segment_dash_dot(batch, a, b, width, color),
    }
}

pub(crate) fn push_segment_dash_dot(
    batch: &mut RenderBatch,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let ux = dx / length;
    let uy = dy / length;
    let pattern = [
        (width * 6.0, true),
        (width * 3.0, false),
        (width * 1.2, true),
        (width * 3.0, false),
    ];
    let mut cursor = 0.0;
    let mut index = 0;
    while cursor < length {
        let (segment_length, draw) = pattern[index % pattern.len()];
        let segment_length = segment_length.max(width);
        let end = (cursor + segment_length).min(length);
        if draw && end > cursor {
            push_segment_quad(
                batch,
                [a[0] + ux * cursor, a[1] + uy * cursor],
                [a[0] + ux * end, a[1] + uy * end],
                width,
                color,
            );
        }
        cursor = end;
        index += 1;
    }
}

pub(crate) fn push_segment_pattern(
    batch: &mut RenderBatch,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
    on_widths: f32,
    off_widths: f32,
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let ux = dx / length;
    let uy = dy / length;
    let on = (width * on_widths).max(width);
    let off = (width * off_widths).max(width);
    let mut cursor = 0.0;
    while cursor < length {
        let end = (cursor + on).min(length);
        if end > cursor {
            push_segment_quad(
                batch,
                [a[0] + ux * cursor, a[1] + uy * cursor],
                [a[0] + ux * end, a[1] + uy * end],
                width,
                color,
            );
        }
        cursor += on + off;
    }
}

pub(crate) fn push_segment_quad(
    batch: &mut RenderBatch,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let half = width.max(1.0) * 0.5;
    let nx = -dy / length * half;
    let ny = dx / length * half;
    let points = [
        [a[0] + nx, a[1] + ny],
        [b[0] + nx, b[1] + ny],
        [b[0] - nx, b[1] - ny],
        [a[0] - nx, a[1] - ny],
    ];
    push_fan_f32(batch, &points, color);
}

pub(crate) fn push_pick_rect(batch: &mut PickBatch, rect: Rect, pick_id: u32) {
    push_pick_fan(batch, &rect.corners(), pick_id);
}

pub(crate) fn push_pick_fan(batch: &mut PickBatch, points: &[Point], pick_id: u32) {
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
