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

@group(1) @binding(0)
var t_light_depth: texture_depth_2d_array;
@group(1) @binding(1)
var s_light_depth: sampler_comparison;

@group(2) @binding(0)
var t_geometry_depth: texture_depth_2d;
@group(2) @binding(1)
var s_geometry_depth: sampler;

fn fog_noise(pos: vec3<f32>) -> f32 {
    var p1 = pos * 0.01;
    p1.x += global_uniforms.time * 0.2;
    p1.y += global_uniforms.time * 0.2;
    p1.z += sin(global_uniforms.time * 0.1) * 0.5;
    let noise1 = fbm(p1);

    var p2 = pos * 0.05;
    p2.x += global_uniforms.time * 0.2;
    p2.y += global_uniforms.time * 0.2;
    p2.z += sin(global_uniforms.time * 0.1) * 0.5;
    let noise2 = fbm(p2);

    return 0.8 * noise1 + 0.2 * noise2;
}

fn ray_march(origin: vec3<f32>, direction: vec3<f32>, max_depth: f32, max_steps: i32, step_size: f32, fog_density: f32) -> vec2<f32> {
    var density = 0.0;
    var depth = 0.0;
    for (var i = 0; i < max_steps; i++)
    {
        depth += step_size;
        if (depth >= max_depth)
        {
            break;
        }
        let noise = fog_noise(origin + direction * depth);
        let blend = min(f32(i + 1) / f32(FOG_BLEND_STEPS), 1.0);
        let contribution = fog_density / f32(max_steps);
        density += blend * noise * contribution;
        if (density >= 1.0)
        {
            density = 1.0;
            break;
        }
    }

    return vec2(density, depth);
}

fn ray_march_fog(origin: vec3<f32>, direction: vec3<f32>, scene_depth: f32) -> vec3<f32> {
    // march into the fog volume
    let fog_march = ray_march(origin, direction, scene_depth, FOG_MAX_STEPS, FOG_STEP_SIZE, FOG_DENSITY);
    let fog_density = fog_march.x;
    let fog_depth = fog_march.y;
    let fog_end_position = origin + direction * fog_depth;

    // march from fog volume to the light
    let fog_to_light = light.position - fog_end_position;
    let max_light_dist = length(fog_to_light);
    let light_direction = fog_to_light / max_light_dist;
    let light_march = ray_march(fog_end_position, light_direction, max_light_dist, FOG_LIGHT_MAX_STEPS, FOG_LIGHT_STEP_SIZE, FOG_LIGHT_DENSITY);
    let occlusion = light_march.x;

    return vec3<f32>(fog_density, fog_depth, occlusion);
}

fn depth_to_linear(depth: f32) -> f32 {
    // convert to linear [near, far] range
    let z_near = camera.planes.x;
    let z_far = camera.planes.y;
    return z_near * z_far / (z_far + depth * (z_near - z_far));
}

@fragment
fn fs_main(vert: FogVertexOutput) -> @location(0) vec4<f32> {
    let origin = vert.world_position.xyz;
    let direction = normalize(origin - camera.position.xyz);
    let volume_depth = depth_to_linear(vert.clip_position.z);
    let uv = vert.clip_position.xy / camera.planes.zw;
    let geometry_depth = depth_to_linear(textureSample(t_geometry_depth, s_geometry_depth, uv));
    let max_fog_depth = geometry_depth - volume_depth;
    if (max_fog_depth <= 0.0)
    {
        return vec4<f32>(0.0);
    }

    let march_result = ray_march_fog(origin, direction, max_fog_depth);
    let fog_density = march_result.x;
    let fog_depth = march_result.y;
    let occlusion = march_result.z;

    var base_color = vec3<f32>(mix(0.5, 0.1, fog_density));
    let ambient_strength = 0.05;
    let ambient_color = base_color * ambient_strength;

    var radiance = vec3<f32>(0.0);
    let fog_position = vert.world_position.xyz + direction * fog_depth;
    let in_light = sample_direct_light(vec4<f32>(fog_position, 1.0));
    if (in_light > 0.0) {
        // attenuation
        let light_dist = length(light.position - fog_position);
        let coef_a = 0.0;
        let coef_b = 1.0;
        let light_attenuation = 1.0 / (1.0 + coef_a * light_dist + coef_b * light_dist * light_dist);

        radiance = light.color.rgb * light.color.a * light_attenuation * in_light * (1.0 - occlusion);
    }

    var result = ambient_color + radiance;

    // tonemap
    result = result / (result + vec3(1.0));

    return vec4(result, fog_density * FOG_ALPHA);
}
