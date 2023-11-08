#include globals.wgsl

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

@fragment
fn fs_main(vert: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.5, 0.5, 0.5, 0.1);
}
