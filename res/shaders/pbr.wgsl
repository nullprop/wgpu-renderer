#include constants.wgsl
#include globals.wgsl
#include brdf.wgsl

// Vertex shader

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    let world_normal = normalize(normal_matrix * model.normal);
    let world_tangent = normalize(normal_matrix * model.tangent);
    let world_bitangent = normalize(normal_matrix * model.bitangent);
    let tangent_matrix = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.proj * camera.view * world_position;
    out.tex_coords = model.tex_coords;
    out.tangent_position = tangent_matrix * world_position.xyz;
    out.tangent_light_position = tangent_matrix * light.position;
    out.tangent_view_position = tangent_matrix * camera.position.xyz;

    out.world_position = world_position;

    return out;
}

// Fragment shader

@group(2)@binding(0)
var t_light_depth: binding_array<texture_depth_2d>;
@group(2) @binding(1)
var s_light_depth: binding_array<sampler_comparison>;

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

fn sample_direct_light(index: i32, light_coords: vec4<f32>) -> f32 {
    if (light_coords.w <= 0.0) {
        return 0.0;
    }

    let flip_correction = vec2<f32>(0.5, -0.5);
    let proj_correction = 1.0 / light_coords.w;
    let light_local = light_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);

    return textureSampleCompareLevel(
        t_light_depth[index],
        s_light_depth[index],
        light_local,
        light_coords.z * proj_correction
    );
}

@fragment
fn fs_main(vert: VertexOutput) -> @location(0) vec4<f32> {
    // textures
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, vert.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, vert.tex_coords);
    let object_roughness_metalness: vec4<f32> = textureSample(
        t_roughness_metalness, s_roughness_metalness, vert.tex_coords);

    let albedo = object_color.xyz;
    // TODO: pass factors to shader
    let roughness = object_roughness_metalness.y * 1.0;
    let metalness = object_roughness_metalness.z * 1.0;
    
    var total_radiance: vec3<f32>;

    var in_light = 0.0;
    for (var i: i32 = 0; i < 6; i++) {
        in_light = sample_direct_light(i, light.matrices[i] * vert.world_position);
        if (in_light > 0.0) {
            break;
        }
    }

    if (in_light > 0.0) {
        // lighting vecs
        let normal_dir = object_normal.xyz * 2.0 - 1.0;
        var light_dir = normalize(vert.tangent_light_position - vert.tangent_position);
        let view_dir = normalize(vert.tangent_view_position - vert.tangent_position);
        let half_dir = normalize(view_dir + light_dir);

        // attenuation
        let light_dist = length(light.position - vert.world_position.xyz);
        let coef_a = 0.0;
        let coef_b = 1.0;
        let light_attenuation = 1.0 / (1.0 + coef_a * light_dist + coef_b * light_dist * light_dist);

        // radiance
        let radiance_strength = max(dot(normal_dir, light_dir), 0.0);
        let radiance = radiance_strength * light.color.xyz * light.color.w * light_attenuation;

        // brdf shading
        total_radiance += radiance * brdf(
            normal_dir,
            light_dir,
            view_dir,
            half_dir,
            albedo,
            roughness,
            metalness
        );
    }

    // ambient
    let ambient_strength = 0.01;
    let ambient_color = ambient_strength * albedo;

    var result = ambient_color + total_radiance;

    // tonemap
    result = result / (result + vec3(1.0));
    // gamma correction
    // TODO: seems to already be handled by wgpu?
    // result = pow(result, vec3(1.0/2.2));

    return vec4<f32>(result, object_color.a);
}
