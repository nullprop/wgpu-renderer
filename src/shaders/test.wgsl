let PI = 3.14159;

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

// normal distribution function (Trowbridge-Reitz GGX)

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, a: f32) -> f32 {
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;

    var denom = (n_dot_h2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return a2 / denom;
}

// geometry function (Smith's Schlick-GGX)

fn geometry_schlick_ggx(nom: f32, k: f32) -> f32 {
    let denom = nom * (1.0 - k) + k;
    return nom / denom;
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, k: f32) -> f32 {
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);
    let ggx1 = geometry_schlick_ggx(n_dot_v, k);
    let ggx2 = geometry_schlick_ggx(n_dot_l, k);
    return ggx1 * ggx2;
}

// fresnel function (Fresnel-Schlick approximation)

fn fresnel_schlick(cos_theta: f32, f: vec3<f32>) -> vec3<f32> {
    return f + (1.0 - f) * pow(1.0 - cos_theta, 5.0);
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;

@group(0)@binding(2)
var t_normal: texture_2d<f32>;
@group(0) @binding(3)
var s_normal: sampler;

@group(0)@binding(4)
var t_metallic_roughness: texture_2d<f32>;
@group(0) @binding(5)
var s_metallic_roughness: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // textures
    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
    let object_metallic_roughness: vec4<f32> = textureSample(
        t_metallic_roughness, s_metallic_roughness, in.tex_coords);
    // TODO: AO

    let albedo = object_color.xyz;
    // TODO: pass factors to shader
    let roughness = object_metallic_roughness.y * 1.0;
    let metallic = object_metallic_roughness.z * 1.0;
    
    // lighting vecs
    let tangent_normal = object_normal.xyz * 2.0 - 1.0;
    var light_dir = normalize(in.tangent_light_position - in.tangent_position);
    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
    let half_dir = normalize(view_dir + light_dir);

    // attenuation
    let light_dist = length(light.position - in.world_position);
    let coef_a = 0.0;
    let coef_b = 1.0;
    let light_attenuation = 1.0 / (1.0 + coef_a * light_dist + coef_b * light_dist * light_dist);

    // radiance
    let radiance_strength = max(dot(tangent_normal, light_dir), 0.0);
    let radiance = radiance_strength * light.color.xyz * light.color.w * light_attenuation;

    // fresnel
    var f = vec3(0.04);
    f = mix(f, albedo, metallic);
    let fresnel = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), f);

    // distribution
    let ndf = distribution_ggx(tangent_normal, half_dir, roughness);

    // geometry
    let geo = geometry_smith(tangent_normal, view_dir, light_dir, roughness);

    // brdf
    let nom = ndf * geo * fresnel;
    let denom = 4.0 * max(dot(tangent_normal, view_dir), 0.0) * max(dot(tangent_normal, light_dir), 0.0) + 0.0001;
    let specular = nom / denom;

    let k_d = (vec3(1.0) - fresnel) * (1.0 - metallic);
    let n_dot_l = max(dot(tangent_normal, light_dir), 0.0);
    let total_radiance = (k_d * albedo / PI + specular) * radiance * n_dot_l;

    // ambient
    let ambient_light_color = vec3(1.0);
    let ambient_strength = 0.025;
    let ambient_color = ambient_light_color * ambient_strength;

    var result = ambient_color + total_radiance;

    // tonemap
    result = result / (result + vec3(1.0));
    //result = pow(result, vec3(1.0/2.2));

    return vec4<f32>(result, object_color.a);
}
