pub mod gpu;
pub mod shader;

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    hash::{Hash, Hasher},
};

use geometry_core::{Point, Rect};
use layout_model::{
    Document, InstanceId, LayerId, LayoutIndex, Shape, ShapeId, ShapeKind, ShapeOccurrenceId,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StressLayout {
    pub count: usize,
    pub columns: i64,
    pub rows: i64,
    pub pitch: i64,
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
    }

    pub fn fingerprint(&self) -> BatchFingerprint {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.vertices.len().hash(&mut hasher);
        self.indices.len().hash(&mut hasher);
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
            vertex_count: self.vertices.len() + self.guide_vertices.len(),
            index_count: self.indices.len() + self.guide_indices.len(),
            hash: hasher.finish(),
        }
    }
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
        Self {
            tile_size: tile_size.max(1),
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
        self.frame_counter = self.frame_counter.wrapping_add(1).max(1);
        let frame_id = self.frame_counter;
        let visible_tile_keys = keys.iter().copied().collect::<BTreeSet<_>>();
        let mut stats = TileFrameStats {
            visible_tiles: keys.len(),
            cached_tiles: self.tiles.len(),
            memory_budget_bytes: options.memory_budget_bytes,
            ..Default::default()
        };
        let occurrence_shapes = occurrence_shape_map(document);
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
                    build_overview_tile(
                        document,
                        occurrence_shapes.as_ref(),
                        tile_bounds,
                        &tile.occurrences,
                    )
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
            let Some(shape) = shape_for_occurrence(document, occurrence_shapes.as_ref(), &id)
            else {
                continue;
            };
            if !document
                .layer(shape.layer)
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
                    entry.insert(build_shape_triangles(document, shape.as_ref()))
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
                let Some(shape) = shape_for_occurrence(document, occurrence_shapes.as_ref(), &id)
                else {
                    continue;
                };
                if !document
                    .layer(shape.layer)
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
                        entry.insert(build_shape_triangles(document, shape.as_ref()))
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

impl StressLayout {
    pub fn new(count: usize) -> Self {
        let columns = (count as f64).sqrt().ceil().max(1.0) as i64;
        let rows = ((count as i64) + columns - 1) / columns;
        Self {
            count,
            columns,
            rows,
            pitch: 240,
        }
    }

    pub fn bounds(self) -> Rect {
        let half_width = self.columns * self.pitch / 2;
        let half_height = self.rows * self.pitch / 2;
        Rect::new(
            Point::new(-half_width - 80, -half_height - 80),
            Point::new(half_width + 80, half_height + 80),
        )
    }
}

pub fn build_stress_frame(
    layout: StressLayout,
    viewport: Rect,
    options: TileFrameOptions,
) -> TiledFrame {
    let mut frame = TiledFrame::default();
    frame.stats.memory_budget_bytes = options.memory_budget_bytes;
    if layout.count == 0 {
        return frame;
    }

    let keys = tile_keys_for_rect(viewport, DEFAULT_TILE_SIZE);
    frame.stats.visible_tiles = keys.len();
    let use_overview = options.lod.enabled
        && (DEFAULT_TILE_SIZE as f32 * options.zoom) <= options.lod.max_tile_screen_px;
    if use_overview {
        for key in keys {
            let tile = key.bounds(DEFAULT_TILE_SIZE);
            let overview = build_stress_overview_tile(layout, tile);
            if overview.shape_count == 0 {
                continue;
            }
            frame.stats.lod_tiles += 1;
            frame.stats.lod_shapes += overview.shape_count;
            frame.stats.visible_shapes += overview.shape_count;
            append_render_batch_with_range(&mut frame.render, &overview.batch);
        }
    } else {
        append_stress_rectangles(layout, viewport, &mut frame.render, &mut frame.stats);
        frame.stats.precise_shapes = frame.stats.visible_shapes;
    }

    frame.stats.draw_ranges = if frame.render.indices.is_empty() {
        0
    } else {
        1
    };
    frame.stats.cache_bytes = frame.render.estimate_bytes();
    if let Some(budget) = options.memory_budget_bytes {
        frame.stats.over_budget_bytes = frame.stats.cache_bytes.saturating_sub(budget);
    }
    frame
}

#[derive(Clone, Copy, Debug, Default)]
struct EvictionStats {
    tiles: usize,
    shape_batches: usize,
}

pub fn build_layout_triangles(document: &Document, viewport: Rect) -> RenderBatch {
    let mut batch = RenderBatch::default();
    for shape in document.visible_flattened_shapes() {
        if !shape.bounds.intersects(viewport) {
            continue;
        }
        let shape = shape.transformed_shape();
        append_shape_triangles(&mut batch, document, &shape);
    }
    batch
}

pub fn build_pick_triangles(document: &Document, viewport: Rect) -> PickBatch {
    let mut batch = PickBatch::default();
    for shape in document.visible_flattened_shapes() {
        if !shape.bounds.intersects(viewport) {
            continue;
        }
        let pick_id = (batch.occurrences.len() + 1).min(u32::MAX as usize) as u32;
        batch.occurrences.push(shape.id.clone());
        let transformed = shape.transformed_shape();
        match &transformed.kind {
            ShapeKind::Rectangle(rect) => push_pick_rect(&mut batch, *rect, pick_id),
            ShapeKind::Polygon(poly) => {
                if poly.points.len() >= 3 {
                    push_pick_fan(&mut batch, &poly.points, pick_id);
                }
            }
            ShapeKind::Path { points, width } => {
                for window in points.windows(2) {
                    push_pick_segment_as_rect(&mut batch, window[0], window[1], *width, pick_id);
                }
            }
            ShapeKind::Via { center, size, .. } => {
                let half = *size / 2;
                push_pick_rect(
                    &mut batch,
                    Rect::new(
                        Point::new(center.x - half, center.y - half),
                        Point::new(center.x + half, center.y + half),
                    ),
                    pick_id,
                );
            }
            ShapeKind::Label { .. } | ShapeKind::Measurement { .. } => {}
        }
    }
    batch
}

pub fn build_shape_triangles(document: &Document, shape: &Shape) -> RenderBatch {
    let mut batch = RenderBatch::default();
    append_shape_triangles(&mut batch, document, shape);
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
    occurrence_shapes: Option<&BTreeMap<ShapeOccurrenceId, Shape>>,
    tile_bounds: Rect,
    occurrences: &[ShapeOccurrenceId],
) -> OverviewTile {
    let mut layers: BTreeMap<LayerId, (Rect, usize)> = BTreeMap::new();
    for id in occurrences {
        let Some(shape) = shape_for_occurrence(document, occurrence_shapes, id) else {
            continue;
        };
        if !document
            .layer(shape.layer)
            .is_some_and(|layer| layer.visible)
            || !is_geometry_shape(&shape.kind)
        {
            continue;
        }
        let Some(bounds) = shape.kind.bounds().intersection(tile_bounds) else {
            continue;
        };
        layers
            .entry(shape.layer)
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

fn build_stress_overview_tile(layout: StressLayout, tile_bounds: Rect) -> OverviewTile {
    let mut layers: BTreeMap<usize, (Rect, usize)> = BTreeMap::new();
    for (index, rect) in stress_rects_in_viewport(layout, tile_bounds) {
        let Some(bounds) = rect.intersection(tile_bounds) else {
            continue;
        };
        let layer = index % STRESS_LAYER_COLORS.len();
        layers
            .entry(layer)
            .and_modify(|(union, count)| {
                *union = union.union(bounds);
                *count += 1;
            })
            .or_insert((bounds, 1));
    }

    let mut batch = RenderBatch::default();
    let mut shape_count = 0;
    for (layer, (bounds, layer_shape_count)) in layers {
        let mut color = STRESS_LAYER_COLORS[layer];
        color[3] = (0.16 + (layer_shape_count as f32).ln_1p() * 0.08).min(0.68);
        push_rect(&mut batch, bounds, color);
        shape_count += layer_shape_count;
    }
    OverviewTile { batch, shape_count }
}

fn append_stress_rectangles(
    layout: StressLayout,
    viewport: Rect,
    batch: &mut RenderBatch,
    stats: &mut TileFrameStats,
) {
    for (index, rect) in stress_rects_in_viewport(layout, viewport) {
        push_rect(
            batch,
            rect,
            STRESS_LAYER_COLORS[index % STRESS_LAYER_COLORS.len()],
        );
        stats.visible_shapes += 1;
    }
}

fn stress_rects_in_viewport(
    layout: StressLayout,
    viewport: Rect,
) -> impl Iterator<Item = (usize, Rect)> {
    let margin = 96;
    let min_col = stress_axis_index(viewport.min.x - margin, layout.columns, layout.pitch);
    let max_col = stress_axis_index(viewport.max.x + margin, layout.columns, layout.pitch);
    let min_row = stress_axis_index(viewport.min.y - margin, layout.rows, layout.pitch);
    let max_row = stress_axis_index(viewport.max.y + margin, layout.rows, layout.pitch);
    let columns = layout.columns;
    let pitch = layout.pitch;
    let x_offset = columns * pitch / 2;
    let y_offset = layout.rows * pitch / 2;
    (min_row..=max_row).flat_map(move |row| {
        (min_col..=max_col).filter_map(move |col| {
            let index = (row * columns + col) as usize;
            if index >= layout.count {
                return None;
            }
            let x = col * pitch - x_offset;
            let y = row * pitch - y_offset;
            let width = 50 + ((index % 7) as i64) * 10;
            let height = 40 + ((index % 5) as i64) * 12;
            Some((index, Rect::from_min_size(Point::new(x, y), width, height)))
        })
    })
}

fn stress_axis_index(world: i64, limit: i64, pitch: i64) -> i64 {
    let offset = limit * pitch / 2;
    ((world + offset).div_euclid(pitch)).clamp(0, limit.saturating_sub(1))
}

const STRESS_LAYER_COLORS: [[f32; 4]; 5] = [
    [0.38, 0.74, 0.50, 0.78],
    [0.83, 0.58, 0.24, 0.78],
    [0.36, 0.64, 0.94, 0.78],
    [0.63, 0.48, 0.88, 0.78],
    [0.62, 0.70, 0.76, 0.58],
];

fn occurrence_shape_map(document: &Document) -> Option<BTreeMap<ShapeOccurrenceId, Shape>> {
    document.has_hierarchy_instances().then(|| {
        document
            .visible_flattened_shapes()
            .into_iter()
            .map(|shape| (shape.id.clone(), shape.transformed_shape()))
            .collect()
    })
}

fn shape_for_occurrence<'a>(
    document: &'a Document,
    occurrence_shapes: Option<&'a BTreeMap<ShapeOccurrenceId, Shape>>,
    occurrence: &ShapeOccurrenceId,
) -> Option<Cow<'a, Shape>> {
    if let Some(shapes) = occurrence_shapes {
        return shapes.get(occurrence).map(Cow::Borrowed);
    }
    document
        .shapes
        .get(&occurrence.source_shape_id())
        .map(Cow::Borrowed)
}

fn is_geometry_shape(kind: &ShapeKind) -> bool {
    !matches!(
        kind,
        ShapeKind::Label { .. } | ShapeKind::Measurement { .. }
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

fn append_render_batch_with_range(out: &mut RenderBatch, batch: &RenderBatch) -> (usize, usize) {
    let index_start = out.indices.len();
    let base = out.vertices.len().min(u32::MAX as usize) as u32;
    out.vertices.extend_from_slice(&batch.vertices);
    out.indices
        .extend(batch.indices.iter().map(|index| base + *index));
    (index_start, out.indices.len() - index_start)
}

fn append_pick_from_render(
    out: &mut PickBatch,
    occurrence: ShapeOccurrenceId,
    batch: &RenderBatch,
) {
    let pick_id = (out.occurrences.len() + 1).min(u32::MAX as usize) as u32;
    let base = out.vertices.len().min(u32::MAX as usize) as u32;
    out.occurrences.push(occurrence);
    out.vertices
        .extend(batch.vertices.iter().map(|vertex| PickVertex {
            position: vertex.position,
            pick_id,
        }));
    out.indices
        .extend(batch.indices.iter().map(|index| base + *index));
}

fn push_rect(batch: &mut RenderBatch, rect: Rect, color: [f32; 4]) {
    push_fan(batch, &rect.corners(), color);
}

fn push_fan(batch: &mut RenderBatch, points: &[Point], color: [f32; 4]) {
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

        assert_ne!(first.fingerprint(), changed.fingerprint());
        assert_ne!(first.fingerprint(), changed_guide.fingerprint());
        assert_ne!(first.fingerprint(), changed_normal.fingerprint());
        assert_eq!(
            first.estimate_bytes(),
            5 * std::mem::size_of::<GpuVertex3d>() + 5 * std::mem::size_of::<u32>()
        );
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
        let document = Document::stress(2_000);
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
    fn procedural_stress_frame_limits_one_million_to_viewport() {
        let layout = StressLayout::new(1_000_000);
        let viewport = Rect::from_min_size(Point::new(-9_600, -6_133), 19_200, 12_266);
        let frame = build_stress_frame(
            layout,
            viewport,
            TileFrameOptions {
                zoom: 0.075,
                ..Default::default()
            },
        );

        assert!(frame.stats.visible_shapes < 8_000);
        assert_eq!(frame.stats.precise_shapes, frame.stats.visible_shapes);
        assert_eq!(frame.render.vertices.len(), frame.stats.visible_shapes * 4);
        assert_eq!(frame.render.indices.len(), frame.stats.visible_shapes * 6);
    }

    #[test]
    fn procedural_stress_frame_uses_lod_when_zoomed_out() {
        let layout = StressLayout::new(1_000_000);
        let viewport = layout.bounds();
        let frame = build_stress_frame(
            layout,
            viewport,
            TileFrameOptions {
                zoom: 0.008,
                ..Default::default()
            },
        );

        assert!(frame.stats.visible_shapes >= 1_000_000);
        assert!(frame.stats.lod_tiles > 0);
        assert_eq!(frame.stats.precise_shapes, 0);
        assert!(frame.render.vertices.len() < 6_000);
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
}
