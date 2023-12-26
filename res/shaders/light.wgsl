fn sample_direct_light_index(index: i32, light_coords: vec4<f32>) -> f32 {
    if (light_coords.w <= 0.0) {
        return 0.0;
    }

    let flip_correction = vec2<f32>(0.5, -0.5);
    let proj_correction = 1.0 / light_coords.w;
    let light_local = light_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);
    let bias = 0.000001;
    let reference_depth = light_coords.z * proj_correction - bias;

    var total_sample = 0.0;
    for (var x: i32 = -SHADOW_SAMPLES; x < SHADOW_SAMPLES; x++) {
        for (var y: i32 = -SHADOW_SAMPLES; y < SHADOW_SAMPLES; y++) {
            let texelSize = vec2<f32>(textureDimensions(t_light_depth));
            let offset = vec2<f32>(f32(x), f32(y)) / texelSize.xy;
            let s = textureSampleCompare(
                t_light_depth,
                s_light_depth,
                light_local + offset,
                index,
                reference_depth
            );
            total_sample += s * INV_SHADOW_SAMPLES;
        }
    }

    return total_sample;
}

fn sample_direct_light(world_position: vec4<f32>) -> f32 {
    var in_light = 0.0;
    if (global_uniforms.use_shadowmaps > 0u) {
        for (var i: i32 = 0; i < 6; i++) {
            let light_coords = light.matrices[i] * world_position;
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

            in_light = sample_direct_light_index(i, light_coords);
            // TODO should break even if 0 since we're inside frustum.
            // See if causes issues with bias overlap between directions.
            if (in_light > 0.0) {
                break;
            }
        }
    } else {
        in_light = 1.0;
    }
    return in_light;
}

fn sample_ambient_light(light: vec4<f32>, light_dist: f32, surface_light_dot: f32) -> vec3<f32> {
    // base ambient
    var ambient = vec3(0.01);

    // lower attenuation to reduce light bleed
    let diff_coef_a = -0.75;
    let diff_coef_b = 0.25;
    let diff_light_attenuation = 1.0 / (1.0 + diff_coef_a * light_dist + diff_coef_b * light_dist * light_dist);
    let diff_direct_light = light.rgb * light.a * diff_light_attenuation;

    // very rough bounce light estimation
    let diffuse_mult = max(surface_light_dot, 0.0);
    ambient += diffuse_mult * 0.03 * diff_direct_light;

    return ambient;
}
