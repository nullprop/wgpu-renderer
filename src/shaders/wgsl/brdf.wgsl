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

fn brdf(
    normal_dir: vec3<f32>,
    light_dir: vec3<f32>,
    view_dir: vec3<f32>,
    half_dir: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    metalness: f32    
) -> vec3<f32> {
    // fresnel
    var dialect = vec3(0.04);
    dialect = mix(dialect, albedo, metalness);
    let fresnel = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), dialect);

    // distribution
    let ndf = distribution_ggx(normal_dir, half_dir, roughness);

    // geometry
    let geo = geometry_smith(normal_dir, view_dir, light_dir, roughness);

    // specular
    let nom = ndf * geo * fresnel;
    let denom = 4.0 * max(dot(normal_dir, view_dir), 0.0) * max(dot(normal_dir, light_dir), 0.0) + 0.0001;
    let specular = nom / denom;

    let k_d = (vec3(1.0) - fresnel) * (1.0 - metalness);
    let n_dot_l = max(dot(normal_dir, light_dir), 0.0);
    return (k_d * albedo / PI + specular) * n_dot_l;
}