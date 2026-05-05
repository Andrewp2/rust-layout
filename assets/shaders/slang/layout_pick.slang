module layout_pick;

struct ViewUniforms {
	float2 center;
	float2 scale;
};

ConstantBuffer<ViewUniforms> view;

struct VertexInput {
	float2 position : POSITION;
	uint pick_id : TEXCOORD0;
};

struct VertexOutput {
	float4 position : SV_Position;
	nointerpolation uint pick_id : TEXCOORD0;
};

[shader("vertex")]
VertexOutput vertex_main(VertexInput input) {
	VertexOutput output;
	float2 ndc = (input.position - view.center) * view.scale;
	output.position = float4(ndc, 0.0, 1.0);
	output.pick_id = input.pick_id;
	return output;
}

[shader("fragment")]
float4 fragment_main(VertexOutput input) : SV_Target {
	uint id = input.pick_id;
	return float4(
		float(id & 255u) / 255.0,
		float((id >> 8) & 255u) / 255.0,
		float((id >> 16) & 255u) / 255.0,
		float((id >> 24) & 255u) / 255.0
	);
}
