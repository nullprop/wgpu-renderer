// Vertex shader

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    position: vec4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>,
    color: vec4<f32>,
}
@group(2) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
    @location(4) world_position: vec3<f32>,
}

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

    out.world_position = world_position.xyz;

    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@group(0)@binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // textures
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    
    // lighting vecs
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    var light_dir = normalize(in.tangent_light_position - in.tangent_position);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);

    // attenuation
    let light_dist = length(light.position - in.world_position);
    let coef_a = 0.0;
    let coef_b = 1.25;
    let light_attenuation = 1.0 / (1.0 + coef_a * light_dist + coef_b * light_dist * light_dist);

    // diffuse
    let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
    let diffuse_color = diffuse_strength * light.color.xyz * light.color.w * light_attenuation;

    // specular
    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color.xyz * light.color.w * light_attenuation;

    // ambient
    let ambient_light_color = vec3(1.0);
    let ambient_strength = 0.025;
    let ambient_color = ambient_light_color * ambient_strength;

    let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;

    return vec4<f32>(result, object_color.a);
}
