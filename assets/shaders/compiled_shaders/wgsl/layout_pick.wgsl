struct ViewUniforms_std140_0
{
    @align(16) center_0 : vec2<f32>,
    @align(8) scale_0 : vec2<f32>,
};

@binding(0) @group(0) var<uniform> view_0 : ViewUniforms_std140_0;
struct VertexOutput_0
{
    @builtin(position) position_0 : vec4<f32>,
    @interpolate(flat) @location(0) pick_id_0 : u32,
};

struct vertexInput_0
{
    @location(0) position_1 : vec2<f32>,
    @location(1) pick_id_1 : u32,
};

@vertex
fn vertex_main( _S1 : vertexInput_0) -> VertexOutput_0
{
    var output_0 : VertexOutput_0;
    output_0.position_0 = vec4<f32>((_S1.position_1 - view_0.center_0) * view_0.scale_0, 0.0f, 1.0f);
    output_0.pick_id_0 = _S1.pick_id_1;
    return output_0;
}

struct pixelOutput_0
{
    @location(0) output_1 : vec4<f32>,
};

struct pixelInput_0
{
    @interpolate(flat) @location(0) pick_id_2 : u32,
};

@fragment
fn fragment_main( _S2 : pixelInput_0, @builtin(position) position_2 : vec4<f32>) -> pixelOutput_0
{
    var _S3 : pixelOutput_0 = pixelOutput_0( vec4<f32>(f32(((_S2.pick_id_2) & (u32(255)))) / 255.0f, f32(((((_S2.pick_id_2) >> (u32(8)))) & (u32(255)))) / 255.0f, f32(((((_S2.pick_id_2) >> (u32(16)))) & (u32(255)))) / 255.0f, f32(((((_S2.pick_id_2) >> (u32(24)))) & (u32(255)))) / 255.0f) );
    return _S3;
}

