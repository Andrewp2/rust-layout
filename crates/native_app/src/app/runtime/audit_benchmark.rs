#![allow(unused_imports)]
use super::*;

pub fn run_operad_audit(options: StartupOptions) -> Result<OperadAuditReport, String> {
    run_operad_audit_scaled(options, UiScale::new(1.0))
}

pub fn run_operad_audit_scaled(
    options: StartupOptions,
    ui_scale: UiScale,
) -> Result<OperadAuditReport, String> {
    GlassworksApp::new_with_options(options)
        .audit_operad_document_scaled(UiSize::new(1440.0, 920.0), ui_scale)
}

pub fn render_operad_snapshot(
    options: StartupOptions,
    width: u32,
    height: u32,
) -> Result<OperadSnapshotReport, String> {
    GlassworksApp::new_with_options(options).render_operad_snapshot(width, height)
}

pub fn render_operad_snapshot_scaled(
    options: StartupOptions,
    width: u32,
    height: u32,
    ui_scale: UiScale,
) -> Result<OperadSnapshotReport, String> {
    GlassworksApp::new_with_options(options).render_operad_snapshot_scaled(width, height, ui_scale)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_3d_benchmark_scaled(
    options: StartupOptions,
    ui_scale: UiScale,
) -> Result<Benchmark3dReport, String> {
    let benchmark = options.benchmark_3d.unwrap_or_default();
    let frames = benchmark.frames.max(1);
    let warmup_frames = benchmark.warmup_frames;
    let count = options.stress_count.unwrap_or(10_000);
    let app_init_started = std::time::Instant::now();
    let mut app = GlassworksApp::new_with_options(options);
    app.set_active_view(StartupView::Layout3d);
    let app_init_ms = app_init_started.elapsed().as_secs_f64() * 1000.0;

    let viewport = UiSize::new(
        benchmark.width.max(1) as f32,
        benchmark.height.max(1) as f32,
    );
    let ui_started = std::time::Instant::now();
    let document = app.build_operad_document_scaled(viewport, ui_scale)?;
    let ui_paint_items = document.paint_list().items.len();
    let ui_build_ms = ui_started.elapsed().as_secs_f64() * 1000.0;

    let batch_started = std::time::Instant::now();
    let batch = build_layout_3d_batch_for_app(&app);
    let fingerprint = batch.fingerprint();
    let validation = batch
        .validate_geometry()
        .map_err(|err| format!("3D benchmark batch validation failed: {err}"))?;
    let batch_build_ms = batch_started.elapsed().as_secs_f64() * 1000.0;
    let batch_bytes = batch.estimate_bytes();

    let (device, queue, adapter) = benchmark_wgpu_device()?;
    let mut renderer = Viewport3dRenderer::new(&device, VIEWPORT_3D_COLOR_FORMAT);
    let target_size = [
        viewport.width.round() as u32,
        viewport.height.round() as u32,
    ];
    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Glassworks 3D benchmark output texture"),
        size: wgpu::Extent3d {
            width: target_size[0].max(1),
            height: target_size[1].max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: VIEWPORT_3D_COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let output_view = output_texture.create_view(&Default::default());
    let first_upload_started = std::time::Instant::now();
    let uniforms = benchmark_3d_uniforms(&app, target_size);
    let first_upload =
        renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, uniforms);
    let first_upload_ms = first_upload_started.elapsed().as_secs_f64() * 1000.0;

    let total_frames = warmup_frames + frames;
    let mut uniform_samples = Vec::with_capacity(frames);
    let mut upload_samples = Vec::with_capacity(frames);
    let mut encode_samples = Vec::with_capacity(frames);
    let mut submit_wait_samples = Vec::with_capacity(frames);
    for frame in 0..total_frames {
        let uniform_started = std::time::Instant::now();
        let uniforms = benchmark_3d_uniforms(&app, target_size);
        let uniform_ms = uniform_started.elapsed().as_secs_f64() * 1000.0;

        let upload_started = std::time::Instant::now();
        renderer.upload_with_fingerprint(&device, &queue, &batch, fingerprint, uniforms);
        let upload_ms = upload_started.elapsed().as_secs_f64() * 1000.0;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Glassworks 3D benchmark encoder"),
        });
        let encode_started = std::time::Instant::now();
        renderer.render_to_view(
            &device,
            &mut encoder,
            &output_view,
            target_size,
            wgpu::Color {
                r: 8.0 / 255.0,
                g: 11.0 / 255.0,
                b: 14.0 / 255.0,
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
            .map_err(|err| format!("3D benchmark GPU submission failed: {err}"))?;
        let submit_wait_ms = submit_started.elapsed().as_secs_f64() * 1000.0;

        if frame >= warmup_frames {
            uniform_samples.push(uniform_ms);
            upload_samples.push(upload_ms);
            encode_samples.push(encode_ms);
            submit_wait_samples.push(submit_wait_ms);
        }
    }

    Ok(Benchmark3dReport {
        count,
        frames,
        warmup_frames,
        viewport,
        adapter,
        app_init_ms,
        ui_build_ms,
        ui_paint_items,
        batch_build_ms,
        batch_bytes,
        mesh_triangles: validation.mesh_triangles,
        rect_slabs: validation.rect_slabs,
        guide_segments: validation.guide_segments,
        first_upload_ms,
        first_upload_bytes: first_upload.bytes_uploaded,
        uniform_avg_ms: average_f64(&uniform_samples),
        cached_upload_avg_ms: average_f64(&upload_samples),
        encode_avg_ms: average_f64(&encode_samples),
        submit_wait_avg_ms: average_f64(&submit_wait_samples),
        submit_wait_p50_ms: percentile_f64(&submit_wait_samples, 0.50),
        submit_wait_p95_ms: percentile_f64(&submit_wait_samples, 0.95),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn benchmark_3d_uniforms(
    app: &GlassworksApp,
    target_size: [u32; 2],
) -> Viewport3dUniforms {
    let aspect = target_size[0] as f32 / target_size[1].max(1) as f32;
    Viewport3dUniforms::from_view_projection(view_projection_3d(
        app.camera_3d,
        aspect,
        app.camera_3d_far_plane(),
    ))
    .with_rect_camera_position([
        app.camera_3d.position.x,
        app.camera_3d.position.y,
        app.camera_3d.position.z,
    ])
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn benchmark_wgpu_device() -> Result<(wgpu::Device, wgpu::Queue, String), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|err| format!("3D benchmark could not find a GPU adapter: {err}"))?;
    let info = adapter.get_info();
    let adapter_label = format!("{} {:?}", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Glassworks 3D benchmark device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .map_err(|err| format!("3D benchmark could not create a GPU device: {err}"))?;
    Ok((device, queue, adapter_label))
}

pub(crate) fn average_f64(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

pub(crate) fn percentile_f64(samples: &[f64], percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let index = ((sorted.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}
