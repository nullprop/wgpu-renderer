#include globals.wgsl

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> @builtin(position) vec4<f32> {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    return light.matrices[light_matrix_index] * (world_position - vec4<f32>(light.position, 0.0));
}
