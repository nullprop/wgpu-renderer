#include globals.wgsl

struct LightVertexInput {
    @location(0) position: vec3<f32>,
};

struct LightVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: LightVertexInput,
) -> LightVertexOutput {
    let scale = 10.0;
    var out: LightVertexOutput;
    out.clip_position = camera.proj * camera.view * vec4<f32>(model.position * scale + light.position, 1.0);
    out.color = light.color.xyz;
    return out;
}

@fragment
fn fs_main(in: LightVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
