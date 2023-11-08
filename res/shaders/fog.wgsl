#include globals.wgsl
#include constants.wgsl
#include noise.wgsl

struct FogVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) light_world_position: vec3<f32>,
}

// Vertex shader

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> FogVertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: FogVertexOutput;
    out.clip_position = camera.proj * camera.view * world_position;
    out.world_position = world_position.xyz / world_position.w;
    out.light_world_position = light.position;

    return out;
}

// Fragment shader

@group(2)@binding(0)
var t_light_depth: texture_depth_2d_array;
@group(2) @binding(1)
var s_light_depth: sampler_comparison;

@group(2)@binding(2)
var t_geometry_depth: texture_depth_2d;
@group(2) @binding(3)
var s_geometry_depth: sampler;

@group(3) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(3)@binding(1)
var s_diffuse: sampler;

@group(3)@binding(2)
var t_normal: texture_2d<f32>;
@group(3) @binding(3)
var s_normal: sampler;

@group(3)@binding(4)
var t_roughness_metalness: texture_2d<f32>;
@group(3) @binding(5)
var s_roughness_metalness: sampler;

fn fog_noise(pos: vec3<f32>) -> f32 {
    var p = pos * FOG_SCALE;
    p.x += global_uniforms.time * 0.01;
    p.y += global_uniforms.time * 0.1;
    p.z += sin(global_uniforms.time * 0.1) * 0.1;
    return fbm(p);
}

fn ray_march(origin: vec3<f32>, direction: vec3<f32>, scene_depth: f32) -> f32 {
    var density = 0.0;
    var depth = 0.0;
    for (var i = 0; i < FOG_MAX_STEPS; i++)
    {
        depth += FOG_MAX_DIST / f32(FOG_MAX_STEPS);
        let p = origin + direction * depth;
        density += fog_noise(p) * FOG_DENSITY / f32(FOG_MAX_STEPS);
        if (density >= 1.0)
        {
            density = 1.0;
            break;
        }
        if (depth >= scene_depth)
        {
            break;
        }
    }
    return density;
}

// FIXME: always 0???
fn scene_depth(clip_position: vec4<f32>) -> f32 {
    if (clip_position.w <= 0.0) {
        return 0.0;
    }

    let ndc = clip_position.xy / clip_position.w;
    let uv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return textureSample(t_geometry_depth, s_geometry_depth, uv);
}

@fragment
fn fs_main(vert: FogVertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.5, 0.5, 0.5, 1.0);

    let cam_to_volume = vert.world_position.xyz - camera.position.xyz;
    let distance_to_volume = length(cam_to_volume);
    let direction = cam_to_volume / distance_to_volume;
    // TODO: pass near and far plane in uniforms
    let geometry_depth = scene_depth(vert.clip_position) * (3000.0 - 1.0) + 1.0 - distance_to_volume;
    if (geometry_depth <= 0.0)
    {
        return vec4<f32>(0.0);
    }
    let density = ray_march(vert.world_position.xyz, direction, geometry_depth);
    color.a *= density;

    return color;
}
