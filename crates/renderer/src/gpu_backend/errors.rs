#![allow(unused_imports)]
use super::*;

pub(crate) fn clamped_index_count(index_count: usize, label: &'static str) -> u32 {
    if index_count > u32::MAX as usize {
        warn!(
            index_count,
            max_index_count = u32::MAX,
            label,
            "GPU index count exceeded u32 draw range; draw count was clamped"
        );
    }
    index_count.min(u32::MAX as usize) as u32
}

pub(crate) fn next_buffer_capacity(required: usize) -> usize {
    required.max(4).next_power_of_two()
}

pub(crate) fn physical_viewport_extent(logical_points: f32, pixels_per_point: f32) -> u32 {
    let logical_points = if logical_points.is_finite() && logical_points > 0.0 {
        logical_points
    } else {
        warn!(
            logical_points,
            "viewport logical extent was non-finite or non-positive; using zero before target-size clamp"
        );
        0.0
    };
    let pixels_per_point = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        warn!(
            pixels_per_point,
            "viewport pixels-per-point was non-finite or non-positive; using 1.0"
        );
        1.0
    };
    let raw_extent = (logical_points * pixels_per_point).ceil();
    let clamped_extent = raw_extent.clamp(1.0, u32::MAX as f32);
    if clamped_extent != raw_extent {
        warn!(
            raw_extent,
            clamped_extent, "viewport physical extent outside u32 range; clamping"
        );
    }
    clamped_extent as u32
}

pub(crate) fn vertex_bytes(vertices: &[crate::GpuVertex]) -> Vec<u8> {
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

pub(crate) fn vertex_3d_bytes(vertices: &[crate::GpuVertex3d]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vertices));
    for vertex in vertices {
        for value in vertex.position {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in vertex.normal {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in vertex.color {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}

pub(crate) fn rect_slab_instance_bytes(instances: &[GpuRectSlabInstance]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * 28);
    for instance in instances {
        for value in instance.rect {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in instance.z_range {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        for value in instance.color {
            bytes.push((value.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    bytes
}

pub(crate) fn rect_slab_template_bytes() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(RECT_SLAB_TEMPLATE_VERTEX_COUNT * 16);
    append_rect_slab_template_slot(&mut bytes, 0);
    append_rect_slab_template_slot(&mut bytes, 1);
    append_rect_slab_template_slot(&mut bytes, 2);
    bytes
}

pub(crate) fn append_rect_slab_template_slot(bytes: &mut Vec<u8>, slot: u32) {
    let corners: [[f32; 3]; 6] = match slot {
        0 => [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        1 => [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        _ => [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    };
    for [x_select, y_select, z_select] in corners {
        for value in [x_select, y_select, z_select, slot as f32] {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
}

pub(crate) fn pick_vertex_bytes(vertices: &[crate::PickVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vertices));
    for vertex in vertices {
        for value in vertex.position {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        bytes.extend_from_slice(&vertex.pick_id.to_ne_bytes());
    }
    bytes
}

pub(crate) fn index_bytes(indices: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(indices));
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }
    bytes
}

pub(crate) const VIEWPORT_3D_SCENE_SHADER: &str = r#"
pub(crate) struct Viewport3dUniforms {
    pub(crate) view_projection: mat4x4<f32>,
    pub(crate) rect_camera_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport3dUniforms;

pub(crate) struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

pub(crate) struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

pub(crate) fn shade_color(color: vec4<f32>, normal: vec3<f32>) -> vec4<f32> {
    let normal_length_squared = dot(normal, normal);
    if normal_length_squared < 0.000001 {
        return color;
    }
    let key_light = vec3<f32>(-0.3495, -0.5493, 0.7590);
    let fill_light = vec3<f32>(0.7635, 0.2776, 0.5830);
    let diffuse = max(dot(normal, key_light), 0.0);
    let fill = max(dot(normal, fill_light), 0.0);
    let upward = clamp(normal.z * 0.5 + 0.5, 0.0, 1.0);
    let shade = 0.52 + diffuse * 0.26 + fill * 0.08 + upward * 0.14;
    return vec4<f32>(color.rgb * shade, color.a);
}

@vertex
pub(crate) fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = viewport.view_projection * vec4<f32>(input.position, 1.0);
    output.color = shade_color(input.color, input.normal);
    return output;
}

@fragment
pub(crate) fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

pub(crate) const VIEWPORT_3D_RECT_SLAB_SHADER: &str = r#"
pub(crate) struct Viewport3dUniforms {
    pub(crate) view_projection: mat4x4<f32>,
    pub(crate) rect_camera_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport3dUniforms;

	pub(crate) struct InstanceInput {
	    @location(0) corner: vec4<f32>,
	    @location(1) rect: vec4<f32>,
	    @location(2) z_range: vec2<f32>,
	    @location(3) color: vec4<f32>,
	};

pub(crate) struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

	@vertex
	pub(crate) fn vertex_main(input: InstanceInput) -> VertexOutput {
	    let rect_center = vec2<f32>(
	        (input.rect.x + input.rect.z) * 0.5,
	        (input.rect.y + input.rect.w) * 0.5,
	    );
	    let z_center = (input.z_range.x + input.z_range.y) * 0.5;
	    let slot = u32(input.corner.w + 0.5);
	    var x_select = input.corner.x;
	    var y_select = input.corner.y;
	    var z_select = input.corner.z;
	    var shade = 0.9040;
	    if slot == 0u {
	        if viewport.rect_camera_position.z < z_center {
	            z_select = 0.0;
	            shade = 0.3375;
	        } else {
	            z_select = 1.0;
	            shade = 0.9040;
	        }
	    } else if slot == 1u {
	        if viewport.rect_camera_position.x < rect_center.x {
	            x_select = 0.0;
	            shade = 0.4221;
	        } else {
	            x_select = 1.0;
	            shade = 0.4037;
	        }
	    } else {
	        if viewport.rect_camera_position.y < rect_center.y {
	            y_select = 0.0;
	            shade = 0.4543;
	        } else {
	            y_select = 1.0;
	            shade = 0.3796;
	        }
	    }
	    let point = vec3<f32>(
	        input.rect.x + (input.rect.z - input.rect.x) * x_select,
	        input.rect.y + (input.rect.w - input.rect.y) * y_select,
	        input.z_range.x + (input.z_range.y - input.z_range.x) * z_select,
	    );
	    var output: VertexOutput;
	    output.position = viewport.view_projection * vec4<f32>(point, 1.0);
	    output.color = vec4<f32>(input.color.rgb * shade, input.color.a);
	    return output;
	}

@fragment
pub(crate) fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

pub(crate) const VIEWPORT_3D_COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0)
var viewport_color: texture_2d<f32>;
@group(0) @binding(1)
var viewport_sampler: sampler;

pub(crate) struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
pub(crate) fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return output;
}

@fragment
pub(crate) fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(viewport_color, viewport_sampler, input.uv);
}
"#;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn create_layout_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    shader::load_shader_module(
        device,
        shader::LAYOUT_FILL_SHADER,
        shader::ShaderBackend::Wgsl,
    )
    .ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn create_pick_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    shader::load_shader_module(
        device,
        shader::LAYOUT_PICK_SHADER,
        shader::ShaderBackend::Wgsl,
    )
    .ok()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn create_layout_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    Some(shader::load_embedded_layout_shader(device))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn create_pick_shader_module(device: &wgpu::Device) -> Option<wgpu::ShaderModule> {
    Some(shader::load_embedded_pick_shader(device))
}

pub(crate) fn offscreen_viewport(width: u32, height: u32, zoom: f32, pan: [f32; 2]) -> Rect {
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

pub(crate) fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

pub(crate) fn count_non_dark_rgba_pixels(
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

pub(crate) fn gpu_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message.into()))
}
