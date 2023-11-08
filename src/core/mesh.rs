use wgpu::util::DeviceExt;
use crate::core::model::ModelVertex;

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Mesh {
    pub fn from_gltf(
        device: &wgpu::Device,
        buffers: &[gltf::buffer::Data],
        mesh: &gltf::Mesh,
        name: &str) -> Vec<Mesh> {
        let mut meshes = Vec::new();

        let primitives = mesh.primitives();
        primitives.for_each(|primitive| {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
            let material_index = primitive.material().index().unwrap_or(0);

            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            if let Some(vertex_attribute) = reader.read_positions() {
                vertex_attribute.for_each(|vertex| {
                    // dbg!(vertex);
                    vertices.push(ModelVertex {
                        position: vertex,
                        ..Default::default()
                    })
                });
            } else {
                panic!();
            }

            if let Some(normal_attribute) = reader.read_normals() {
                let mut normal_index = 0;
                normal_attribute.for_each(|normal| {
                    // dbg!(normal);
                    vertices[normal_index].normal = normal;
                    normal_index += 1;
                });
            } else {
                panic!();
            }

            if let Some(tangent_attribute) = reader.read_tangents() {
                // println!("gltf: loading tangents from file");
                let mut tangent_index = 0;
                tangent_attribute.for_each(|tangent| {
                    // dbg!(tangent);
                    vertices[tangent_index].tangent = [
                        tangent[0] * tangent[3],
                        tangent[1] * tangent[3],
                        tangent[2] * tangent[3],
                    ];
                    vertices[tangent_index].bitangent =
                        cgmath::Vector3::from(vertices[tangent_index].normal)
                            .cross(cgmath::Vector3::from(vertices[tangent_index].tangent))
                            .into();
                    tangent_index += 1;
                });
            } else {
                println!("gltf: no tangents in file, calculating from tris");
                Mesh::calc_tangents(&indices, &mut vertices);
            }

            if let Some(tex_coord_attribute) = reader.read_tex_coords(0).map(|v| v.into_f32()) {
                let mut tex_coord_index = 0;
                tex_coord_attribute.for_each(|tex_coord| {
                    // dbg!(tex_coord);
                    vertices[tex_coord_index].tex_coords = tex_coord;
                    tex_coord_index += 1;
                });
            } else {
                panic!();
            }

            if let Some(indices_raw) = reader.read_indices() {
                // dbg!(indices_raw);
                indices.append(&mut indices_raw.into_u32().collect::<Vec<u32>>());
            } else {
                panic!();
            }
            // dbg!(indices);

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", name)),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            meshes.push(Mesh {
                name: name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: indices.len() as u32,
                material: material_index,
            });
        });

        meshes
    }

    pub fn calc_tangents(indices: &[u32], vertices: &mut Vec<ModelVertex>) {
        // tangents and bitangents from triangles
        let mut triangles_included = vec![0; vertices.len()];
        for chunk in indices.chunks(3) {
            let v0 = vertices[chunk[0] as usize];
            let v1 = vertices[chunk[1] as usize];
            let v2 = vertices[chunk[2] as usize];

            let pos0: cgmath::Vector3<f32> = v0.position.into();
            let pos1: cgmath::Vector3<f32> = v1.position.into();
            let pos2: cgmath::Vector3<f32> = v2.position.into();

            let uv0: cgmath::Vector2<f32> = v0.tex_coords.into();
            let uv1: cgmath::Vector2<f32> = v1.tex_coords.into();
            let uv2: cgmath::Vector2<f32> = v2.tex_coords.into();

            let delta_pos1 = pos1 - pos0;
            let delta_pos2 = pos2 - pos0;

            let delta_uv1 = uv1 - uv0;
            let delta_uv2 = uv2 - uv0;

            let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
            let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
            let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

            for i in chunk.iter().take(3) {
                let sz = *i as usize;
                vertices[sz].tangent =
                    (tangent + cgmath::Vector3::from(vertices[sz].tangent)).into();
                vertices[sz].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[sz].bitangent)).into();
                triangles_included[sz] += 1;
            }
        }

        // Average the tangents/bitangents
        for (i, n) in triangles_included.into_iter().enumerate() {
            let denom = 1.0 / n as f32;
            let v = &mut vertices[i];
            v.tangent = (cgmath::Vector3::from(v.tangent) * denom).into();
            v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom).into();
        }
    }

    /*
    pub fn cube(device: &wgpu::Device, size: [f32; 3], name: &str, material_index: usize) -> Mesh {
        #[rustfmt::skip]
        let mut vertices = vec![
            // front
            ModelVertex { position: [-size[0], -size[1], -size[2]], tex_coords: [0.0, 0.0], normal: [-size[0], -size[1], -size[2]], ..Default::default() },
            ModelVertex { position: [ size[0], -size[1], -size[2]], tex_coords: [0.0, 0.0], normal: [ size[0], -size[1], -size[2]], ..Default::default() },
            ModelVertex { position: [ size[0],  size[1], -size[2]], tex_coords: [0.0, 0.0], normal: [ size[0],  size[1], -size[2]], ..Default::default() },
            ModelVertex { position: [-size[0],  size[1], -size[2]], tex_coords: [0.0, 0.0], normal: [-size[0],  size[1], -size[2]], ..Default::default() },

            // back
            ModelVertex { position: [-size[0], -size[1],  size[2]], tex_coords: [0.0, 0.0], normal: [-size[0], -size[1],  size[2]], ..Default::default() },
            ModelVertex { position: [ size[0], -size[1],  size[2]], tex_coords: [0.0, 0.0], normal: [ size[0], -size[1],  size[2]], ..Default::default() },
            ModelVertex { position: [ size[0],  size[1],  size[2]], tex_coords: [0.0, 0.0], normal: [ size[0],  size[1],  size[2]], ..Default::default() },
            ModelVertex { position: [-size[0],  size[1],  size[2]], tex_coords: [0.0, 0.0], normal: [-size[0],  size[1],  size[2]], ..Default::default() },
        ];
        #[rustfmt::skip]
        let indices = vec![
            // front
            0, 1, 2,
            2, 3, 0,
            // back
            4, 6, 5,
            4, 7, 6,
            // left
            0, 3, 4,
            3, 7, 4,
            // right
            1, 2, 5,
            2, 6, 5,
            // top
            2, 7, 3,
            2, 6, 7,
            // bottom
            1, 4, 0,
            1, 5, 4
        ];
        Mesh::calc_tangents(&indices, &mut vertices);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh {
            name: name.to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material: material_index,
        }
    }
    */
}