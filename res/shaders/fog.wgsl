#include constants.wgsl
#include globals.wgsl
#include light.wgsl
#include noise.wgsl

struct FogVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
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
    out.world_position = world_position;
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
        let noise = fog_noise(origin + direction * depth);
        depth += FOG_MAX_DIST / f32(FOG_MAX_STEPS);
        let blend = min(depth / FOG_BLEND_DIST, 1.0);
        let contribution = FOG_DENSITY / f32(FOG_MAX_STEPS);
        density += blend * noise * contribution;
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

fn scene_depth(clip_position: vec4<f32>) -> f32 {
    if (clip_position.w <= 0.0) {
        return 0.0;
    }

    let ndc = clip_position.xy / clip_position.w;
    let uv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    let depth = textureSample(t_geometry_depth, s_geometry_depth, uv);

    // convert to linear [near, far] range
    let z_near = camera.planes.x;
    let z_far = camera.planes.y;
    return z_near * z_far / (z_far + depth * (z_near - z_far));
}

@fragment
fn fs_main(vert: FogVertexOutput) -> @location(0) vec4<f32> {
    let cam_to_volume = vert.world_position.xyz - camera.position.xyz;
    let distance_to_volume = length(cam_to_volume);
    let direction = cam_to_volume / distance_to_volume;
    // FIXME: t_geometry_depth is 0
//    let geometry_depth = scene_depth(vert.clip_position) - distance_to_volume;
//    if (geometry_depth <= 0.0)
//    {
//        return vec4<f32>(0.0);
//    }
    let geometry_depth = 3000.0;
    let density = ray_march(vert.world_position.xyz, direction, geometry_depth);

    var in_light = 0.0;
    if (global_uniforms.use_shadowmaps > 0u) {
        for (var i: i32 = 0; i < 6; i++) {
            let light_coords = light.matrices[i] * vert.world_position;
            let light_dir = normalize(light_coords.xyz);
            let bias = 0.01;
            // z can never be smaller than this inside 90 degree frustum
            if (light_dir.z < INV_SQRT_3 - bias) {
                continue;
            }
            // x and y can never be larger than this inside frustum
            if (abs(light_dir.y) > INV_SQRT_2 + bias) {
                continue;
            }
            if (abs(light_dir.x) > INV_SQRT_2 + bias) {
                continue;
            }

            in_light = sample_direct_light(i, light_coords);
            // TODO should break even if 0 since we're inside frustum.
            // See if causes issues with bias overlap between directions.
            if (in_light > 0.0) {
                break;
            }
        }
    } else {
        in_light = 1.0;
    }

    var color = vec3<f32>(0.5, 0.5, 0.5);
    let ambient_strength = 0.02;
    let ambient_color = color * ambient_strength;

    var radiance = vec3<f32>(0.0);
    if (in_light > 0.0) {
        // attenuation
        let light_dist = length(light.position - vert.world_position.xyz);
        let coef_a = 0.0;
        let coef_b = 1.0;
        let light_attenuation = 1.0 / (1.0 + coef_a * light_dist + coef_b * light_dist * light_dist);

        radiance = light.color.rgb * light.color.a * light_attenuation * in_light;
    }

    var result = ambient_color + radiance;

    // tonemap
    result = result / (result + vec3(1.0));

    return vec4(result, density * FOG_ALPHA);
}
