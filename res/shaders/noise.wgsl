// Noise functions from 42yeah:
// https://blog.42yeah.is/rendering/2023/02/11/clouds.html
fn rand(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.345, 67.89, 412.12))) * 42123.45) * 2.0 - 1.0;
}

fn value_noise(p: vec3<f32>) -> f32 {
    let u = floor(p);
    let v = fract(p);
    let s = smoothstep(vec3<f32>(0.0), vec3<f32>(1.0), v);

    let a = rand(u);
    let b = rand(u + vec3<f32>(1.0, 0.0, 0.0));
    let c = rand(u + vec3<f32>(0.0, 1.0, 0.0));
    let d = rand(u + vec3<f32>(1.0, 1.0, 0.0));
    let e = rand(u + vec3<f32>(0.0, 0.0, 1.0));
    let f = rand(u + vec3<f32>(1.0, 0.0, 1.0));
    let g = rand(u + vec3<f32>(0.0, 1.0, 1.0));
    let h = rand(u + vec3<f32>(1.0, 1.0, 1.0));

    return mix(mix(mix(a, b, s.x), mix(c, d, s.x), s.y),
               mix(mix(e, f, s.x), mix(g, h, s.x), s.y),
               s.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    let num_octaves = 8;
    var weight = 0.5;
    var q = p;
    var ret = 0.0;

    for (var i = 0; i < num_octaves; i++)
    {
        ret += weight * value_noise(q);
        q *= 2.0;
        weight *= 0.5;
    }

    return saturate(ret);
}