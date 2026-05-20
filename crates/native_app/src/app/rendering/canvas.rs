#![allow(unused_imports)]
use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub struct LayoutCanvasResources {
    pub(crate) renderer: Option<LayoutGpuRenderer>,
    pub(crate) target_format: Option<wgpu::TextureFormat>,
    pub(crate) tile_cache: TileCache,
    pub(crate) layout_revision: u64,
    pub(crate) layout_view_revision: u64,
    pub(crate) frame: Option<TiledFrame>,
    pub(crate) frame_fingerprint: Option<BatchFingerprint>,
    pub(crate) frame_tiles: Vec<TileKey>,
    pub(crate) frame_zoom: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for LayoutCanvasResources {
    fn default() -> Self {
        Self {
            renderer: None,
            target_format: None,
            tile_cache: TileCache::default(),
            layout_revision: u64::MAX,
            layout_view_revision: u64::MAX,
            frame: None,
            frame_fingerprint: None,
            frame_tiles: Vec::new(),
            frame_zoom: 0.0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Viewport3dCanvasResources {
    pub(crate) renderer: Option<Viewport3dRenderer>,
    pub(crate) target_format: Option<wgpu::TextureFormat>,
    pub(crate) batch: Option<RenderBatch3d>,
    pub(crate) batch_fingerprint: Option<BatchFingerprint>,
    pub(crate) batch_revision: u64,
    pub(crate) batch_show_grid: bool,
    pub(crate) batch_instanced_rect_slabs: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Viewport3dCanvasResources {
    fn default() -> Self {
        Self {
            renderer: None,
            target_format: None,
            batch: None,
            batch_fingerprint: None,
            batch_revision: u64::MAX,
            batch_show_grid: false,
            batch_instanced_rect_slabs: true,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_canvas_requests(
    app: &GlassworksApp,
    renderer: &mut WgpuRenderer,
    request: &RenderFrameRequest,
    layout_canvas: &mut LayoutCanvasResources,
    viewport_3d_canvas: &mut Viewport3dCanvasResources,
) -> Result<(), String> {
    for canvas_request in request.canvas_requests() {
        let Some(size) = canvas_pixel_size(
            canvas_request.rect.width,
            canvas_request.rect.height,
            request.options.scale_factor * normalized_canvas_scale(canvas_request.transform.scale),
        ) else {
            continue;
        };
        let surface = renderer
            .get_gpu_context(&canvas_request.canvas, size)
            .map_err(|err| format!("create snapshot canvas context: {err}"))?;
        render_layout_canvas_context(
            app,
            layout_canvas,
            viewport_3d_canvas,
            canvas_request.canvas.key.as_str(),
            canvas_request.canvas.surface_key(),
            surface,
            UiSize::new(canvas_request.rect.width, canvas_request.rect.height),
        )?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_canvas_context(
    app: &GlassworksApp,
    layout_canvas: &mut LayoutCanvasResources,
    viewport_3d_canvas: &mut Viewport3dCanvasResources,
    canvas_key: &str,
    surface_key: &str,
    surface: WgpuCanvasContext<'_>,
    logical_size: UiSize,
) -> Result<(), String> {
    match canvas_key {
        "glassworks.layout.viewport.2d" => {
            render_layout_2d_canvas_with_size(app, layout_canvas, surface, logical_size)
        }
        "glassworks.layout.viewport.3d" => {
            render_layout_3d_canvas(app, viewport_3d_canvas, surface)
        }
        LAYOUT_DRC_MARKER_SNAPSHOT_CANVAS_KEY => {
            render_layout_drc_marker_snapshot_canvas(app, surface_key, surface)
        }
        _ => Ok(()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_drc_marker_snapshot_canvas(
    app: &GlassworksApp,
    image_key: &str,
    surface: WgpuCanvasContext<'_>,
) -> Result<(), String> {
    let size = surface.size();
    let Some(rgba) = layout_drc_marker_snapshot_canvas_rgba(app, image_key, size) else {
        surface.clear(ColorRgba::new(18, 24, 32, 255));
        return Ok(());
    };
    let bytes_per_row = size
        .width
        .checked_mul(4)
        .ok_or_else(|| "DRC marker snapshot canvas row overflow".to_string())?;
    surface.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: surface.texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(size.height),
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_2d_canvas(
    app: &GlassworksApp,
    resources: &mut LayoutCanvasResources,
    surface: WgpuCanvasContext<'_>,
) -> Result<(), String> {
    let size = surface.size();
    render_layout_2d_canvas_with_size(
        app,
        resources,
        surface,
        UiSize::new(size.width as f32, size.height as f32),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_2d_canvas_with_size(
    app: &GlassworksApp,
    resources: &mut LayoutCanvasResources,
    surface: WgpuCanvasContext<'_>,
    logical_size: UiSize,
) -> Result<(), String> {
    let viewport = app.layout_viewport_for_size(logical_size);
    let zoom = app.layout_zoom;
    if resources.layout_revision != app.layout_revision
        || resources.layout_view_revision != app.layout_view_revision
    {
        resources.tile_cache.clear();
        resources.layout_revision = app.layout_revision;
        resources.layout_view_revision = app.layout_view_revision;
        resources.frame = None;
        resources.frame_fingerprint = None;
        resources.frame_tiles.clear();
    }
    let fingerprint = refresh_layout_2d_frame_cache(app, resources, viewport, zoom);
    let format = surface.format();
    if resources.target_format != Some(format) {
        resources.renderer = Some(
            LayoutGpuRenderer::new(surface.device(), format)
                .ok_or_else(|| "initialize 2D layout GPU canvas renderer".to_string())?,
        );
        resources.target_format = Some(format);
    }
    let renderer = resources
        .renderer
        .as_mut()
        .ok_or_else(|| "2D layout GPU canvas renderer unavailable".to_string())?;
    let frame = resources
        .frame
        .as_ref()
        .ok_or_else(|| "2D layout frame cache unavailable".to_string())?;
    renderer.upload_with_fingerprint(
        surface.device(),
        surface.queue(),
        &frame.render,
        fingerprint,
        ViewUniforms::from_viewport(viewport),
    );

    let mut encoder = surface.create_command_encoder(Some("Glassworks 2D layout canvas encoder"));
    {
        let mut render_pass = surface
            .begin_render_pass(&mut encoder, Some(ColorRgba::new(8, 11, 14, 255)))
            .forget_lifetime();
        renderer.paint(&mut render_pass);
    }
    surface.queue().submit([encoder.finish()]);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn refresh_layout_2d_frame_cache(
    app: &GlassworksApp,
    resources: &mut LayoutCanvasResources,
    viewport: Rect,
    zoom: f32,
) -> BatchFingerprint {
    if resources.layout_revision != app.layout_revision
        || resources.layout_view_revision != app.layout_view_revision
    {
        resources.tile_cache.clear();
        resources.layout_revision = app.layout_revision;
        resources.layout_view_revision = app.layout_view_revision;
        resources.frame = None;
        resources.frame_fingerprint = None;
        resources.frame_tiles.clear();
    }
    let frame_tiles = resources.tile_cache.tile_keys_for_viewport(viewport);
    let needs_rebuild = resources.frame.is_none()
        || resources.layout_revision != app.layout_revision
        || resources.layout_view_revision != app.layout_view_revision
        || resources.frame_tiles != frame_tiles
        || (resources.frame_zoom - zoom).abs() > f32::EPSILON;
    if needs_rebuild {
        let frame = app.with_layout_display_index(|index| {
            resources.tile_cache.build_frame_with_options(
                &app.workspace().document,
                index,
                viewport,
                TileFrameOptions {
                    include_pick: false,
                    root_cell: Some(app.layout_view_top_cell),
                    zoom,
                    lod: TileLodConfig {
                        enabled: app.app_options.performance.dense_2d_lod,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
        });
        resources.frame_fingerprint = Some(frame.render.fingerprint());
        resources.frame = Some(frame);
        resources.layout_revision = app.layout_revision;
        resources.layout_view_revision = app.layout_view_revision;
        resources.frame_tiles = frame_tiles;
        resources.frame_zoom = zoom;
    }
    resources
        .frame_fingerprint
        .expect("2D frame fingerprint should be populated")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render_layout_3d_canvas(
    app: &GlassworksApp,
    resources: &mut Viewport3dCanvasResources,
    surface: WgpuCanvasContext<'_>,
) -> Result<(), String> {
    let format = surface.format();
    if resources.target_format != Some(format) {
        resources.renderer = Some(Viewport3dRenderer::new(surface.device(), format));
        resources.target_format = Some(format);
    }
    let fingerprint = refresh_layout_3d_batch_cache(app, resources);
    let size = surface.size();
    let aspect = size.width as f32 / size.height.max(1) as f32;
    let uniforms = Viewport3dUniforms::from_view_projection(view_projection_3d(
        app.camera_3d,
        aspect,
        app.camera_3d_far_plane(),
    ))
    .with_rect_camera_position([
        app.camera_3d.position.x,
        app.camera_3d.position.y,
        app.camera_3d.position.z,
    ]);
    let renderer = resources
        .renderer
        .as_mut()
        .ok_or_else(|| "3D layout GPU canvas renderer unavailable".to_string())?;
    let batch = resources
        .batch
        .as_ref()
        .ok_or_else(|| "3D layout batch cache unavailable".to_string())?;
    renderer.upload_with_fingerprint(
        surface.device(),
        surface.queue(),
        batch,
        fingerprint,
        uniforms,
    );

    let mut encoder = surface.create_command_encoder(Some("Glassworks 3D layout canvas encoder"));
    renderer.render_to_view(
        surface.device(),
        &mut encoder,
        surface.view(),
        [size.width.max(1), size.height.max(1)],
        wgpu::Color {
            r: 8.0 / 255.0,
            g: 11.0 / 255.0,
            b: 14.0 / 255.0,
            a: 1.0,
        },
    );
    surface.queue().submit([encoder.finish()]);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn refresh_layout_3d_batch_cache(
    app: &GlassworksApp,
    resources: &mut Viewport3dCanvasResources,
) -> BatchFingerprint {
    let needs_rebuild = resources.batch.is_none()
        || resources.batch_revision != app.layout_revision
        || resources.batch_show_grid != app.show_3d_grid
        || resources.batch_instanced_rect_slabs != app.app_options.performance.dense_3d_instancing;
    if needs_rebuild {
        let batch = build_layout_3d_batch_for_app(app);
        resources.batch_fingerprint = Some(batch.fingerprint());
        resources.batch = Some(batch);
        resources.batch_revision = app.layout_revision;
        resources.batch_show_grid = app.show_3d_grid;
        resources.batch_instanced_rect_slabs = app.app_options.performance.dense_3d_instancing;
    }
    resources
        .batch_fingerprint
        .expect("3D batch fingerprint should be populated")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn canvas_pixel_size(width: f32, height: f32, scale: f32) -> Option<PixelSize> {
    Some(PixelSize::new(
        pixel_extent(width, scale)?,
        pixel_extent(height, scale)?,
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn pixel_extent(value: f32, scale: f32) -> Option<u32> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let pixels = (value * normalized_canvas_scale(scale)).ceil();
    if !pixels.is_finite() || pixels <= 0.0 {
        return None;
    }
    Some(pixels.min(u32::MAX as f32) as u32)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn normalized_canvas_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fitted_layout_viewport(bounds: Option<Rect>, canvas_size: PixelSize) -> Rect {
    let bounds =
        bounds.unwrap_or_else(|| Rect::from_min_size(Point::new(-500, -500), 1_000, 1_000));
    let canvas_aspect = canvas_size.width.max(1) as f32 / canvas_size.height.max(1) as f32;
    let mut world_width = bounds.width().max(1) as f32 * 1.12;
    let mut world_height = bounds.height().max(1) as f32 * 1.12;
    if world_width / world_height < canvas_aspect {
        world_width = world_height * canvas_aspect;
    } else {
        world_height = world_width / canvas_aspect;
    }
    let center = bounds.center();
    let half_width = (world_width * 0.5).ceil().max(1.0) as i64;
    let half_height = (world_height * 0.5).ceil().max(1.0) as i64;
    Rect::new(
        Point::new(center.x - half_width, center.y - half_height),
        Point::new(center.x + half_width, center.y + half_height),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_layout_3d_batch_for_app(app: &GlassworksApp) -> RenderBatch3d {
    let bounds = app.with_layout_index(|index| index.bounds());
    build_layout_3d_batch_with_bounds_and_options(
        &app.workspace().document,
        app.show_3d_grid,
        bounds,
        app.app_options.performance.dense_3d_instancing,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_layout_3d_batch(document: &Document) -> RenderBatch3d {
    build_layout_3d_batch_with_options(document, false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_layout_3d_batch_with_options(document: &Document, show_grid: bool) -> RenderBatch3d {
    let index = LayoutIndex::rebuild_hierarchical(document);
    build_layout_3d_batch_with_bounds(document, show_grid, index.bounds())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_layout_3d_batch_with_bounds(
    document: &Document,
    show_grid: bool,
    bounds: Option<Rect>,
) -> RenderBatch3d {
    build_layout_3d_batch_with_bounds_and_options(document, show_grid, bounds, true)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_layout_3d_batch_with_bounds_and_options(
    document: &Document,
    show_grid: bool,
    bounds: Option<Rect>,
    use_instanced_rect_slabs: bool,
) -> RenderBatch3d {
    let Some(bounds) = bounds else {
        return RenderBatch3d::default();
    };
    let layer_slots = layout_layer_slots(document);
    let mut batch = RenderBatch3d::default();

    document.for_each_visible_flattened_shape_view(|_, shape| {
        if matches!(
            shape.shape.kind,
            ShapeKindView::Label { .. } | ShapeKindView::Measurement { .. }
        ) {
            return;
        }
        let rect = shape.bounds;
        if rect.width() <= 0 || rect.height() <= 0 {
            return;
        }
        let default_slot = layer_slots
            .get(&shape.shape.layer)
            .copied()
            .unwrap_or(shape.shape.layer.0 as usize);
        let (z_min, z_max) = document
            .layers
            .get(&shape.shape.layer)
            .map(|layer| layer_3d_stack_range(layer.process))
            .unwrap_or_else(|| default_layer_3d_stack_range(default_slot));
        let mut color = document.layer_color(shape.shape.layer);
        color[3] = 1.0;
        if use_instanced_rect_slabs {
            batch.rect_slabs.push(GpuRectSlabInstance {
                rect: [
                    rect.min.x as f32,
                    rect.min.y as f32,
                    rect.max.x as f32,
                    rect.max.y as f32,
                ],
                z_range: [z_min, z_max],
                color,
            });
        } else {
            add_3d_rect_slab_mesh(&mut batch, rect, z_min, z_max, color);
        }
    });

    add_3d_reference_guides(&mut batch, bounds);
    if show_grid {
        add_3d_ground_grid(&mut batch, bounds);
    }
    batch
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn layout_layer_slots(document: &Document) -> BTreeMap<LayerId, usize> {
    let mut layers = document
        .layers
        .iter()
        .filter(|(_, layer)| layer.visible)
        .map(|(id, layer)| (layer.display_order, id.0, *id))
        .collect::<Vec<_>>();
    layers.sort_unstable();
    layers
        .into_iter()
        .enumerate()
        .map(|(index, (_, _, id))| (id, index))
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_3d_rect_slab_mesh(
    batch: &mut RenderBatch3d,
    rect: Rect,
    z_min: f32,
    z_max: f32,
    color: [f32; 4],
) {
    let x0 = rect.min.x as f32;
    let y0 = rect.min.y as f32;
    let x1 = rect.max.x as f32;
    let y1 = rect.max.y as f32;
    add_3d_quad(
        batch,
        [
            [x0, y0, z_max],
            [x1, y0, z_max],
            [x1, y1, z_max],
            [x0, y1, z_max],
        ],
        [0.0, 0.0, 1.0],
        color,
    );
    add_3d_quad(
        batch,
        [
            [x0, y1, z_min],
            [x1, y1, z_min],
            [x1, y0, z_min],
            [x0, y0, z_min],
        ],
        [0.0, 0.0, -1.0],
        color,
    );
    add_3d_quad(
        batch,
        [
            [x0, y0, z_min],
            [x1, y0, z_min],
            [x1, y0, z_max],
            [x0, y0, z_max],
        ],
        [0.0, -1.0, 0.0],
        color,
    );
    add_3d_quad(
        batch,
        [
            [x1, y0, z_min],
            [x1, y1, z_min],
            [x1, y1, z_max],
            [x1, y0, z_max],
        ],
        [1.0, 0.0, 0.0],
        color,
    );
    add_3d_quad(
        batch,
        [
            [x1, y1, z_min],
            [x0, y1, z_min],
            [x0, y1, z_max],
            [x1, y1, z_max],
        ],
        [0.0, 1.0, 0.0],
        color,
    );
    add_3d_quad(
        batch,
        [
            [x0, y1, z_min],
            [x0, y0, z_min],
            [x0, y0, z_max],
            [x0, y1, z_max],
        ],
        [-1.0, 0.0, 0.0],
        color,
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_3d_quad(
    batch: &mut RenderBatch3d,
    positions: [[f32; 3]; 4],
    normal: [f32; 3],
    color: [f32; 4],
) {
    let base = batch.vertices.len() as u32;
    batch.vertices.extend(positions.map(|position| GpuVertex3d {
        position,
        normal,
        color,
    }));
    batch
        .indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_3d_reference_guides(batch: &mut RenderBatch3d, bounds: Rect) {
    let start = batch.guide_vertices.len() as u32;
    let guide_bounds = expanded_3d_guide_bounds(bounds);
    let guides = [
        (
            [guide_bounds.min.x as f32, 0.0, 0.0],
            [guide_bounds.max.x as f32, 0.0, 0.0],
            [0.35, 0.62, 0.86, 1.0],
        ),
        (
            [0.0, guide_bounds.min.y as f32, 0.0],
            [0.0, guide_bounds.max.y as f32, 0.0],
            [0.48, 0.76, 0.56, 1.0],
        ),
    ];
    for (from, to, color) in guides {
        batch.guide_vertices.push(GpuVertex3d {
            position: from,
            normal: [0.0, 0.0, 0.0],
            color,
        });
        batch.guide_vertices.push(GpuVertex3d {
            position: to,
            normal: [0.0, 0.0, 0.0],
            color,
        });
    }
    batch
        .guide_indices
        .extend([start, start + 1, start + 2, start + 3]);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_3d_ground_grid(batch: &mut RenderBatch3d, bounds: Rect) {
    let color = [0.28, 0.34, 0.40, 0.55];
    let bounds = expanded_3d_guide_bounds(bounds);
    let width = bounds.width().max(1);
    let height = bounds.height().max(1);
    let step = nice_3d_grid_step(width.max(height));
    let start_x = floor_to_step(bounds.min.x, step);
    let end_x = ceil_to_step(bounds.max.x, step);
    let start_y = floor_to_step(bounds.min.y, step);
    let end_y = ceil_to_step(bounds.max.y, step);
    let mut indices = Vec::new();
    let mut x = start_x;
    while x <= end_x {
        let base = batch.guide_vertices.len() as u32;
        batch.guide_vertices.push(GpuVertex3d {
            position: [x as f32, bounds.min.y as f32, 0.0],
            normal: [0.0, 0.0, 0.0],
            color,
        });
        batch.guide_vertices.push(GpuVertex3d {
            position: [x as f32, bounds.max.y as f32, 0.0],
            normal: [0.0, 0.0, 0.0],
            color,
        });
        indices.extend([base, base + 1]);
        x += step;
    }
    let mut y = start_y;
    while y <= end_y {
        let base = batch.guide_vertices.len() as u32;
        batch.guide_vertices.push(GpuVertex3d {
            position: [bounds.min.x as f32, y as f32, 0.0],
            normal: [0.0, 0.0, 0.0],
            color,
        });
        batch.guide_vertices.push(GpuVertex3d {
            position: [bounds.max.x as f32, y as f32, 0.0],
            normal: [0.0, 0.0, 0.0],
            color,
        });
        indices.extend([base, base + 1]);
        y += step;
    }
    batch.guide_indices.extend(indices);
}

pub(crate) fn layer_3d_stack_range(process: ProcessLayer) -> (f32, f32) {
    let (base, thickness) = match process {
        ProcessLayer::Diffusion => (0.0, 80.0),
        ProcessLayer::Oxide => (95.0, 40.0),
        ProcessLayer::Poly => (155.0, 75.0),
        ProcessLayer::Contact => (230.0, 110.0),
        ProcessLayer::Metal1 => (340.0, 90.0),
        ProcessLayer::Via1 => (430.0, 120.0),
        ProcessLayer::Metal2 => (550.0, 90.0),
        ProcessLayer::Annotation => (0.0, 0.0),
    };
    (base, base + thickness)
}

pub(crate) fn default_layer_3d_stack_range(slot: usize) -> (f32, f32) {
    let base = slot as f32 * 95.0;
    (base, base + 70.0)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn expanded_3d_guide_bounds(bounds: Rect) -> Rect {
    let margin = bounds.width().max(bounds.height()).max(1) / 10;
    bounds.expanded(margin.max(1))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn nice_3d_grid_step(span: Coord) -> Coord {
    let rough = (span / 12).max(1);
    let mut magnitude = 1;
    while magnitude <= rough / 10 {
        magnitude *= 10;
    }
    for multiplier in [1, 2, 5, 10] {
        let step = magnitude * multiplier;
        if step >= rough {
            return step;
        }
    }
    magnitude * 10
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn floor_to_step(value: Coord, step: Coord) -> Coord {
    value.div_euclid(step) * step
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ceil_to_step(value: Coord, step: Coord) -> Coord {
    value.div_euclid(step) * step + if value.rem_euclid(step) == 0 { 0 } else { step }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_offscreen_render(
    options: OffscreenRenderOptions,
) -> Result<OffscreenRenderReport, Box<dyn std::error::Error>> {
    pollster::block_on(run_offscreen_render_async(options))
}
