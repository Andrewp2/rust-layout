use std::error::Error;

use crate::{PickBatch, RenderBatch, shader};
use geometry_core::{Coord, Point, Rect};
use layout_model::{Document, LayoutIndex, ShapeOccurrenceId};

#[derive(Clone, Copy, Debug)]
pub struct ViewUniforms {
    center: [f32; 2],
    scale: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct GpuPickRequest {
    pub serial: u64,
    pub pointer_screen: [f32; 2],
    pub pointer_local: [f32; 2],
    pub canvas_size: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GpuUploadStats {
    pub last_upload_bytes: usize,
    pub resident_bytes: usize,
    pub layout_uploads: u64,
    pub layout_skips: u64,
    pub pick_uploads: u64,
    pub pick_skips: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BufferUploadResult {
    pub uploaded: bool,
    pub skipped: bool,
    pub bytes_uploaded: usize,
}

#[derive(Clone, Debug)]
pub struct OffscreenRenderRequest {
    pub scene: String,
    pub document: Document,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub pan: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct OffscreenRenderReport {
    pub scene: String,
    pub backend: String,
    pub adapter: String,
    pub width: u32,
    pub height: u32,
    pub visible_shapes: usize,
    pub visible_tiles: usize,
    pub vertices: usize,
    pub indices: usize,
    pub non_dark_pixels: usize,
}

pub struct LayoutGpuRenderer {
    pipeline: wgpu::RenderPipeline,
    pick_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pick_vertex_buffer: wgpu::Buffer,
    pick_index_buffer: wgpu::Buffer,
    pick_texture: Option<wgpu::Texture>,
    pick_texture_size: [u32; 2],
    vertex_capacity: usize,
    index_capacity: usize,
    pick_vertex_capacity: usize,
    pick_index_capacity: usize,
    render_fingerprint: Option<crate::BatchFingerprint>,
    pick_fingerprint: Option<crate::BatchFingerprint>,
    index_count: u32,
    pick_index_count: u32,
}

impl ViewUniforms {
    pub fn from_viewport(viewport: Rect) -> Self {
        let center = viewport.center();
        Self {
            center: [center.x as f32, center.y as f32],
            scale: [
                2.0 / (viewport.width().max(1) as f32),
                2.0 / (viewport.height().max(1) as f32),
            ],
        }
    }

    fn as_bytes(self) -> [u8; 16] {
        let values = [self.center[0], self.center[1], self.scale[0], self.scale[1]];
        let mut bytes = [0u8; 16];
        for (index, value) in values.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
        }
        bytes
    }
}

impl GpuPickRequest {
    pub fn new(
        serial: u64,
        pointer_screen: [f32; 2],
        pointer_local: [f32; 2],
        canvas_size: [f32; 2],
    ) -> Self {
        Self {
            serial,
            pointer_screen,
            pointer_local,
            canvas_size,
        }
    }
}

impl OffscreenRenderReport {
    pub fn summary(&self) -> String {
        format!(
            "offscreen render ok: scene={} size={}x{} backend={} adapter=\"{}\" visible_shapes={} visible_tiles={} vertices={} indices={} non_dark_pixels={}",
            self.scene,
            self.width,
            self.height,
            self.backend,
            self.adapter,
            self.visible_shapes,
            self.visible_tiles,
            self.vertices,
            self.indices,
            self.non_dark_pixels
        )
    }
}

impl LayoutGpuRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Option<Self> {
        let shader_module = create_layout_shader_module(device)?;
        let pick_shader_module = create_pick_shader_module(device)?;
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fabricad layout uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fabricad layout bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fabricad layout bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fabricad layout pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let targets = [Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let vertex_attributes = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 8,
                shader_location: 1,
            },
        ];
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attributes,
        }];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fabricad layout pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fragment_main"),
                targets: &targets,
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let pick_targets = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pick_vertex_attributes = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32,
                offset: 8,
                shader_location: 1,
            },
        ];
        let pick_vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::PickVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &pick_vertex_attributes,
        }];
        let pick_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fabricad pick pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &pick_shader_module,
                entry_point: Some("vertex_main"),
                buffers: &pick_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &pick_shader_module,
                entry_point: Some("fragment_main"),
                targets: &pick_targets,
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(Self {
            pipeline,
            pick_pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer: create_upload_buffer(
                device,
                "Fabricad layout vertex buffer",
                4,
                wgpu::BufferUsages::VERTEX,
            ),
            index_buffer: create_upload_buffer(
                device,
                "Fabricad layout index buffer",
                4,
                wgpu::BufferUsages::INDEX,
            ),
            pick_vertex_buffer: create_upload_buffer(
                device,
                "Fabricad pick vertex buffer",
                4,
                wgpu::BufferUsages::VERTEX,
            ),
            pick_index_buffer: create_upload_buffer(
                device,
                "Fabricad pick index buffer",
                4,
                wgpu::BufferUsages::INDEX,
            ),
            pick_texture: None,
            pick_texture_size: [0, 0],
            vertex_capacity: 4,
            index_capacity: 4,
            pick_vertex_capacity: 4,
            pick_index_capacity: 4,
            render_fingerprint: None,
            pick_fingerprint: None,
            index_count: 0,
            pick_index_count: 0,
        })
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &RenderBatch,
        uniforms: ViewUniforms,
    ) -> BufferUploadResult {
        queue.write_buffer(&self.uniform_buffer, 0, &uniforms.as_bytes());
        let fingerprint = batch.fingerprint();
        let changed = self.render_fingerprint != Some(fingerprint);
        let mut bytes_uploaded = 0;
        if changed {
            let vertex_bytes = vertex_bytes(&batch.vertices);
            let index_bytes = index_bytes(&batch.indices);
            self.ensure_vertex_capacity(device, vertex_bytes.len());
            self.ensure_index_capacity(device, index_bytes.len());
            if !vertex_bytes.is_empty() {
                queue.write_buffer(&self.vertex_buffer, 0, &vertex_bytes);
                bytes_uploaded += vertex_bytes.len();
            }
            if !index_bytes.is_empty() {
                queue.write_buffer(&self.index_buffer, 0, &index_bytes);
                bytes_uploaded += index_bytes.len();
            }
            self.render_fingerprint = Some(fingerprint);
        }
        self.index_count = batch.indices.len().min(u32::MAX as usize) as u32;
        BufferUploadResult {
            uploaded: bytes_uploaded > 0,
            skipped: !changed,
            bytes_uploaded,
        }
    }

    pub fn upload_pick(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &PickBatch,
        uniforms: ViewUniforms,
    ) -> BufferUploadResult {
        queue.write_buffer(&self.uniform_buffer, 0, &uniforms.as_bytes());
        let fingerprint = batch.gpu_fingerprint();
        let changed = self.pick_fingerprint != Some(fingerprint);
        let mut bytes_uploaded = 0;
        if changed {
            let vertex_bytes = pick_vertex_bytes(&batch.vertices);
            let index_bytes = index_bytes(&batch.indices);
            self.ensure_pick_vertex_capacity(device, vertex_bytes.len());
            self.ensure_pick_index_capacity(device, index_bytes.len());
            if !vertex_bytes.is_empty() {
                queue.write_buffer(&self.pick_vertex_buffer, 0, &vertex_bytes);
                bytes_uploaded += vertex_bytes.len();
            }
            if !index_bytes.is_empty() {
                queue.write_buffer(&self.pick_index_buffer, 0, &index_bytes);
                bytes_uploaded += index_bytes.len();
            }
            self.pick_fingerprint = Some(fingerprint);
        }
        self.pick_index_count = batch.indices.len().min(u32::MAX as usize) as u32;
        BufferUploadResult {
            uploaded: bytes_uploaded > 0,
            skipped: !changed,
            bytes_uploaded,
        }
    }

    pub fn resident_bytes(&self) -> usize {
        self.vertex_capacity
            + self.index_capacity
            + self.pick_vertex_capacity
            + self.pick_index_capacity
            + 16
    }

    pub fn prepare_pick_readback<F>(
        &mut self,
        device: &wgpu::Device,
        pixels_per_point: f32,
        batch: &PickBatch,
        request: GpuPickRequest,
        publish: F,
    ) -> Option<wgpu::CommandBuffer>
    where
        F: FnOnce(GpuPickRequest, Option<ShapeOccurrenceId>) + Send + 'static,
    {
        if self.pick_index_count == 0 {
            publish(request, None);
            return None;
        }

        let width = (request.canvas_size[0] * pixels_per_point).ceil().max(1.0) as u32;
        let height = (request.canvas_size[1] * pixels_per_point).ceil().max(1.0) as u32;
        if request.pointer_local[0] < 0.0
            || request.pointer_local[1] < 0.0
            || request.pointer_local[0] >= request.canvas_size[0]
            || request.pointer_local[1] >= request.canvas_size[1]
        {
            publish(request, None);
            return None;
        }
        let pixel_x = ((request.pointer_local[0] * pixels_per_point).floor() as u32)
            .min(width.saturating_sub(1));
        let pixel_y = ((request.pointer_local[1] * pixels_per_point).floor() as u32)
            .min(height.saturating_sub(1));

        self.ensure_pick_texture(device, width, height);
        let texture = self.pick_texture.as_ref()?;
        let texture_view = texture.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fabricad pick readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Fabricad pick encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fabricad pick pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
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
            });
            render_pass.set_pipeline(&self.pick_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.pick_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.pick_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.pick_index_count, 0, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: pixel_x,
                    y: pixel_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let occurrences = batch.occurrences.clone();
        let readback_for_callback = readback.clone();
        encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
            let shape_id = if result.is_ok() {
                let view = readback_for_callback.slice(..).get_mapped_range();
                let pick_id = u32::from_le_bytes([view[0], view[1], view[2], view[3]]);
                drop(view);
                readback_for_callback.unmap();
                pick_id
                    .checked_sub(1)
                    .and_then(|index| occurrences.get(index as usize).cloned())
            } else {
                None
            };
            publish(request, shape_id);
        });

        Some(encoder.finish())
    }

    pub fn paint(&self, render_pass: &mut wgpu::RenderPass<'static>) {
        if self.index_count == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    fn ensure_vertex_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.vertex_capacity {
            return;
        }
        self.vertex_capacity = next_buffer_capacity(required);
        self.vertex_buffer = create_upload_buffer(
            device,
            "Fabricad layout vertex buffer",
            self.vertex_capacity,
            wgpu::BufferUsages::VERTEX,
        );
    }

    fn ensure_index_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.index_capacity {
            return;
        }
        self.index_capacity = next_buffer_capacity(required);
        self.index_buffer = create_upload_buffer(
            device,
            "Fabricad layout index buffer",
            self.index_capacity,
            wgpu::BufferUsages::INDEX,
        );
    }

    fn ensure_pick_vertex_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.pick_vertex_capacity {
            return;
        }
        self.pick_vertex_capacity = next_buffer_capacity(required);
        self.pick_vertex_buffer = create_upload_buffer(
            device,
            "Fabricad pick vertex buffer",
            self.pick_vertex_capacity,
            wgpu::BufferUsages::VERTEX,
        );
    }

    fn ensure_pick_index_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.pick_index_capacity {
            return;
        }
        self.pick_index_capacity = next_buffer_capacity(required);
        self.pick_index_buffer = create_upload_buffer(
            device,
            "Fabricad pick index buffer",
            self.pick_index_capacity,
            wgpu::BufferUsages::INDEX,
        );
    }

    fn ensure_pick_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.pick_texture_size == [width, height] && self.pick_texture.is_some() {
            return;
        }
        self.pick_texture_size = [width, height];
        self.pick_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Fabricad pick texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }));
    }
}

pub async fn render_document_offscreen(
    request: OffscreenRenderRequest,
) -> Result<OffscreenRenderReport, Box<dyn Error>> {
    let width = request.width.max(1);
    let height = request.height.max(1);
    let zoom = request.zoom.clamp(0.001, 32.0);
    let index = build_layout_index(&request.document);
    let viewport = offscreen_viewport(width, height, zoom, request.pan);
    let mut tile_cache = crate::TileCache::default();
    let frame = tile_cache.build_frame_with_options(
        &request.document,
        &index,
        viewport,
        crate::TileFrameOptions {
            include_pick: false,
            zoom,
            ..Default::default()
        },
    );
    if frame.render.indices.is_empty() {
        return Err(gpu_error(format!(
            "offscreen scene {} produced no render indices",
            request.scene
        )));
    }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::PRIMARY),
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await?;
    let adapter_info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Fabricad offscreen device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut resources = LayoutGpuRenderer::new(&device, format)
        .ok_or_else(|| gpu_error("failed to create offscreen layout GPU resources"))?;
    resources.upload(
        &device,
        &queue,
        &frame.render,
        ViewUniforms::from_viewport(viewport),
    );

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Fabricad offscreen texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&Default::default());
    let unpadded_bytes_per_row = width * 4;
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback_size = padded_bytes_per_row as u64 * height as u64;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Fabricad offscreen readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Fabricad offscreen encoder"),
    });
    {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fabricad offscreen render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 13.0 / 255.0,
                            g: 16.0 / 255.0,
                            b: 18.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();
        resources.paint(&mut render_pass);
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
                bytes_per_row: Some(padded_bytes_per_row),
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
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(std::time::Duration::from_secs(30)),
    })?;

    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result.map_err(|err| err.to_string()));
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(30)),
    })?;
    rx.recv_timeout(std::time::Duration::from_secs(30))?
        .map_err(gpu_error)?;

    let non_dark_pixels = count_non_dark_rgba_pixels(
        &readback.slice(..).get_mapped_range(),
        width,
        height,
        unpadded_bytes_per_row,
        padded_bytes_per_row,
    );
    readback.unmap();
    if non_dark_pixels == 0 {
        return Err(gpu_error("offscreen render completed but output was blank"));
    }

    Ok(OffscreenRenderReport {
        scene: request.scene,
        backend: format!("{:?}", adapter_info.backend),
        adapter: adapter_info.name,
        width,
        height,
        visible_shapes: frame.stats.visible_shapes,
        visible_tiles: frame.stats.visible_tiles,
        vertices: frame.render.vertices.len(),
        indices: frame.render.indices.len(),
        non_dark_pixels,
    })
}

fn build_layout_index(document: &Document) -> LayoutIndex {
    if document.has_hierarchy_instances() {
        LayoutIndex::rebuild_hierarchical(document)
    } else {
        LayoutIndex::rebuild(document)
    }
}

fn create_upload_buffer(
    device: &wgpu::Device,
    label: &'static str,
    size: usize,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4) as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn next_buffer_capacity(required: usize) -> usize {
    required.max(4).next_power_of_two()
}

fn vertex_bytes(vertices: &[crate::GpuVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vertices));
    for vertex in vertices {
        for value in vertex.position {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in vertex.color {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

fn pick_vertex_bytes(vertices: &[crate::PickVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vertices));
    for vertex in vertices {
        for value in vertex.position {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        bytes.extend_from_slice(&vertex.pick_id.to_ne_bytes());
    }
    bytes
}

fn index_bytes(indices: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(indices));
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }
    bytes
}

#[cfg(not(target_arch = "wasm32"))]
fn create_layout_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    shader::load_shader_module(
        device,
        shader::LAYOUT_FILL_SHADER,
        shader::ShaderBackend::Wgsl,
    )
    .ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn create_pick_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    shader::load_shader_module(
        device,
        shader::LAYOUT_PICK_SHADER,
        shader::ShaderBackend::Wgsl,
    )
    .ok()
}

#[cfg(target_arch = "wasm32")]
fn create_layout_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    Some(shader::load_embedded_layout_shader(device))
}

#[cfg(target_arch = "wasm32")]
fn create_pick_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    Some(shader::load_embedded_pick_shader(device))
}

fn offscreen_viewport(width: u32, height: u32, zoom: f32, pan: [f32; 2]) -> Rect {
    let zoom = zoom.max(0.001);
    let center = Point::new(
        (-pan[0] / zoom).round() as Coord,
        (pan[1] / zoom).round() as Coord,
    );
    let half_width = (width as f32 / (2.0 * zoom)).ceil() as Coord;
    let half_height = (height as f32 / (2.0 * zoom)).ceil() as Coord;
    Rect::new(
        Point::new(center.x - half_width, center.y - half_height),
        Point::new(center.x + half_width, center.y + half_height),
    )
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn count_non_dark_rgba_pixels(
    bytes: &[u8],
    width: u32,
    height: u32,
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
) -> usize {
    let mut count = 0;
    for y in 0..height as usize {
        let row_start = y * padded_bytes_per_row as usize;
        let row_end = row_start + unpadded_bytes_per_row as usize;
        let row = &bytes[row_start..row_end];
        for pixel in row.chunks_exact(4).take(width as usize) {
            if pixel[0].max(pixel[1]).max(pixel[2]) > 45 {
                count += 1;
            }
        }
    }
    count
}

fn gpu_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message.into()))
}
