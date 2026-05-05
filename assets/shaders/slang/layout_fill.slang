module layout_fill;

struct ViewUniforms {
	float2 center;
	float2 scale;
};

ConstantBuffer<ViewUniforms> view;

struct VertexInput {
	float2 position : POSITION;
	float4 color : COLOR0;
};

struct VertexOutput {
	float4 position : SV_Position;
	float4 color : COLOR0;
};

[shader("vertex")]
VertexOutput vertex_main(VertexInput input) {
	VertexOutput output;
	float2 ndc = (input.position - view.center) * view.scale;
	output.position = float4(ndc, 0.0, 1.0);
	output.color = input.color;
	return output;
}

[shader("fragment")]
float4 fragment_main(VertexOutput input) : SV_Target {
	return input.color;
}
