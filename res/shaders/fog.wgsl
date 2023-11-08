#include globals.wgsl
#include constants.wgsl
#include noise.wgsl

struct FogVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) light_world_position: vec3<f32>,
}

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

@fragment
fn fs_main(vert: FogVertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.5, 0.5, 0.5, 1.0);

    let direction = normalize(vert.world_position.xyz - camera.position.xyz);
    let scene_depth = FOG_MAX_DIST; // TODO: sample geometry pass depth buffer
    let density = ray_march(vert.world_position.xyz, direction, scene_depth);
    color.a *= density;

    return color;
}
