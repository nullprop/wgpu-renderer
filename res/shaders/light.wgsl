fn sample_direct_light(index: i32, light_coords: vec4<f32>) -> f32 {
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
