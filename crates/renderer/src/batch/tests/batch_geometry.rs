#![allow(unused_imports)]
use super::*;
use crate::*;
use layout_model::{LayerFillStyle, LayerLineStyle, ProcessLayer, ShapeKind};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) static WGPU_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wgpu_test_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
pub(crate) fn tile_cache_deduplicates_shapes_spanning_tiles() {
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
pub(crate) fn tile_cache_reuses_tiles_and_shape_geometry() {
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
pub(crate) fn render_batch_fingerprint_tracks_gpu_buffer_contents() {
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
pub(crate) fn layer_display_styles_change_2d_geometry_batches() {
    let mut document = Document::new("display styles");
    let metal = document.layer_by_process(ProcessLayer::Metal1).unwrap();
    let shape_id = document.insert_shape(
        metal,
        ShapeKind::Rectangle(Rect::from_min_size(Point::new(0, 0), 120, 80)),
    );

    let shape = document.shapes.get(&shape_id).unwrap();
    let solid = build_shape_triangles(&document, &shape);
    assert_eq!(solid.indices.len(), 6);

    document.layers.get_mut(&metal).unwrap().line_style = LayerLineStyle::Dashed;
    let shape = document.shapes.get(&shape_id).unwrap();
    let dashed = build_shape_triangles(&document, &shape);
    assert!(dashed.indices.len() > solid.indices.len());
    assert_ne!(dashed.fingerprint(), solid.fingerprint());

    document.layers.get_mut(&metal).unwrap().line_style = LayerLineStyle::Dotted;
    let shape = document.shapes.get(&shape_id).unwrap();
    let dotted = build_shape_triangles(&document, &shape);
    assert!(dotted.indices.len() > solid.indices.len());
    assert_ne!(dotted.fingerprint(), dashed.fingerprint());

    document.layers.get_mut(&metal).unwrap().line_style = LayerLineStyle::DashDot;
    let shape = document.shapes.get(&shape_id).unwrap();
    let dash_dot = build_shape_triangles(&document, &shape);
    assert!(dash_dot.indices.len() > solid.indices.len());
    assert_ne!(dash_dot.fingerprint(), dotted.fingerprint());

    let layer = document.layers.get_mut(&metal).unwrap();
    layer.fill_style = LayerFillStyle::Hatched;
    layer.line_style = LayerLineStyle::Solid;
    let shape = document.shapes.get(&shape_id).unwrap();
    let hatched = build_shape_triangles(&document, &shape);
    assert!(hatched.indices.len() > solid.indices.len());
    assert_ne!(hatched.fingerprint(), solid.fingerprint());

    document.layers.get_mut(&metal).unwrap().fill_style = LayerFillStyle::CrossHatched;
    let shape = document.shapes.get(&shape_id).unwrap();
    let cross_hatched = build_shape_triangles(&document, &shape);
    assert!(cross_hatched.indices.len() > hatched.indices.len());
    assert_ne!(cross_hatched.fingerprint(), hatched.fingerprint());

    document.layers.get_mut(&metal).unwrap().fill_style = LayerFillStyle::Stippled;
    let shape = document.shapes.get(&shape_id).unwrap();
    let stippled = build_shape_triangles(&document, &shape);
    assert!(stippled.indices.len() > solid.indices.len());
    assert_ne!(stippled.fingerprint(), cross_hatched.fingerprint());

    document.layers.get_mut(&metal).unwrap().fill_style = LayerFillStyle::DenseStippled;
    let shape = document.shapes.get(&shape_id).unwrap();
    let dense_stippled = build_shape_triangles(&document, &shape);
    assert!(dense_stippled.indices.len() > stippled.indices.len());
    assert_ne!(dense_stippled.fingerprint(), stippled.fingerprint());

    document.layers.get_mut(&metal).unwrap().fill_style = LayerFillStyle::SparseStippled;
    let shape = document.shapes.get(&shape_id).unwrap();
    let sparse_stippled = build_shape_triangles(&document, &shape);
    assert!(sparse_stippled.indices.len() > solid.indices.len());
    assert_ne!(sparse_stippled.fingerprint(), dense_stippled.fingerprint());
}

#[test]
pub(crate) fn render_batch_3d_fingerprint_tracks_depth_geometry() {
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
pub(crate) fn render_batch_3d_validation_accepts_valid_mesh_slabs_and_guides() {
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
pub(crate) fn render_batch_3d_validation_rejects_invalid_geometry() {
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
pub(crate) fn viewport_3d_renderer_initializes_gpu_pipelines() {
    let _wgpu_guard = wgpu_test_guard();
    let Some((device, _queue)) = test_wgpu_device() else {
        return;
    };

    let _renderer = gpu::Viewport3dRenderer::new(&device, gpu::VIEWPORT_3D_COLOR_FORMAT);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn viewport_3d_renderer_draws_ten_k_instanced_slabs() {
    let _wgpu_guard = wgpu_test_guard();
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
        label: Some("Glassworks 10k 3D renderer test composite target"),
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
        let (_encode_ms, submit_wait_ms) =
            submit_3d_test_scene_only(&device, &queue, &mut renderer, [TEST_WIDTH, TEST_HEIGHT]);
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
pub(crate) fn viewport_3d_renderer_screenshot_tracks_camera_facing_slab_side() {
    let _wgpu_guard = wgpu_test_guard();
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
    let front_uniforms = gpu::Viewport3dUniforms::from_view_projection(view_projection_3d_test(
        [0.0, -4.0, 0.5],
        [0.0, 0.0, 0.5],
        1.0,
    ))
    .with_rect_camera_position([0.0, -4.0, 0.5]);
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
    .with_rect_camera_position([0.0, 4.0, 0.5]);
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
pub(crate) fn viewport_3d_renderer_screenshot_keeps_mesh_above_rect_slabs() {
    let _wgpu_guard = wgpu_test_guard();
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
pub(crate) fn submit_3d_test_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut gpu::Viewport3dRenderer,
    composite_view: &wgpu::TextureView,
    target_size: [u32; 2],
) -> (f64, f64) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Glassworks 10k 3D renderer test encoder"),
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
                label: Some("Glassworks 10k 3D renderer test composite pass"),
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
                multiview_mask: None,
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
pub(crate) fn submit_3d_test_scene_only(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut gpu::Viewport3dRenderer,
    target_size: [u32; 2],
) -> (f64, f64) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Glassworks 10k 3D renderer scene-only test encoder"),
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
pub(crate) fn submit_3d_test_composite_only(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &gpu::Viewport3dRenderer,
    composite_view: &wgpu::TextureView,
) -> (f64, f64) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Glassworks 10k 3D renderer composite-only test encoder"),
    });
    let encode_started = std::time::Instant::now();
    {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Glassworks 10k 3D renderer composite-only test pass"),
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
                multiview_mask: None,
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
pub(crate) struct TestPixels {
    pub(crate) bytes_per_row: u32,
    pub(crate) data: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
impl TestPixels {
    pub(crate) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
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
pub(crate) fn render_3d_screenshot_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut gpu::Viewport3dRenderer,
    target_size: [u32; 2],
) -> TestPixels {
    let width = target_size[0];
    let height = target_size[1];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Glassworks 3D screenshot test target"),
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
        label: Some("Glassworks 3D screenshot test readback"),
        size: bytes_per_row as u64 * height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Glassworks 3D screenshot test encoder"),
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
                label: Some("Glassworks 3D screenshot test composite pass"),
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
                multiview_mask: None,
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
pub(crate) fn view_projection_3d_test(
    position: [f32; 3],
    target: [f32; 3],
    aspect: f32,
) -> [f32; 16] {
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
pub(crate) struct Vec3Test {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Vec3Test {
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub(crate) fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub(crate) fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub(crate) fn normalized(self) -> Self {
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
pub(crate) fn row_major_4x4_to_column_major_test(rows: [[f32; 4]; 4]) -> [f32; 16] {
    [
        rows[0][0], rows[1][0], rows[2][0], rows[3][0], rows[0][1], rows[1][1], rows[2][1],
        rows[3][1], rows[0][2], rows[1][2], rows[2][2], rows[3][2], rows[0][3], rows[1][3],
        rows[2][3], rows[3][3],
    ]
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn align_to_test(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn average(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((sorted.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}

#[test]
pub(crate) fn pick_batch_fingerprint_tracks_pick_ids() {
    let document = Document::stress(4);
    let viewport = Rect::from_min_size(Point::new(-600, -600), 1200, 1200);
    let first = build_pick_triangles(&document, viewport);
    let mut changed = first.clone();
    changed.vertices[0].pick_id += 1;

    assert_ne!(first.gpu_fingerprint(), changed.gpu_fingerprint());
}

#[test]
pub(crate) fn tile_cache_invalidates_only_intersecting_tiles() {
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
pub(crate) fn tile_cache_evicts_old_tiles_to_memory_budget() {
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
pub(crate) fn pick_batch_is_limited_to_visible_tile_occurrences() {
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
pub(crate) fn tile_cache_uses_overview_lod_for_dense_far_zoom_tiles() {
    let document = Document::stress(1_000);
    let index = LayoutIndex::rebuild(&document);
    let viewport = Rect::from_min_size(Point::new(-6_000, -6_000), 12_000, 12_000);
    let lod = TileLodConfig {
        enabled: true,
        max_tile_screen_px: 220.0,
        min_shapes_per_tile: 8,
        ..Default::default()
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
pub(crate) fn tile_cache_uses_overview_lod_for_extremely_dense_tiles() {
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
                max_tile_screen_px: 0.0,
                min_shapes_per_tile: usize::MAX,
                extreme_max_tile_screen_px: 2_048.0,
                extreme_min_shapes_per_tile: 8,
            },
            ..Default::default()
        },
    );

    assert!(frame.stats.lod_tiles > 0);
    assert!(
        frame
            .draw_ranges
            .iter()
            .any(|range| matches!(range.kind, DrawBatchRangeKind::TileOverview { .. }))
    );
}

#[test]
pub(crate) fn tile_cache_keeps_precise_geometry_at_close_zoom() {
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
                ..Default::default()
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
pub(crate) fn tile_cache_renders_repeated_hierarchy_instances_as_distinct_occurrences() {
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
pub(crate) fn test_wgpu_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    });
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
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
        label: Some("Glassworks renderer test device"),
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
