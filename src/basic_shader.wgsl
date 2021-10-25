// Vertex shader

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn main(
    model: VertexInput,
) -> VertexOutput {
    
    var out: VertexOutput;

    out.uv = model.uv;
    out.clip_position = vec4<f32>(model.position, 1.0);

    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var tex_diffuse: texture_2d<f32>; // uniform var

[[group(0), binding(1)]]
var sampler_diffuse: sampler; // uniform var

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(tex_diffuse, sampler_diffuse, in.uv);
}
