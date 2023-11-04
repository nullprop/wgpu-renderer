use wgpu::util::DeviceExt;
use rust_embed::RustEmbed;

use crate::core::model::{Material, Mesh, Model, ModelVertex};
use crate::core::texture::Texture;

#[derive(RustEmbed)]
#[folder = "res"]
struct Asset;

pub fn load_string(file_name: &str) -> String {
    let binary = Asset::get(file_name).unwrap();
    std::str::from_utf8(binary.data.as_ref()).unwrap().to_owned()
}

pub async fn load_model_gltf(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<Model> {
    let mut materials = Vec::new();
    let mut meshes = Vec::new();

    println!("gltf: Loading file {}", file_name);
    let binary = Asset::get(file_name).unwrap();
    let (document, buffers, mut images) = gltf::import_slice(binary.data.as_ref())?;

    println!("gltf: Loading meshes");
    for mesh in document.meshes() {
        let primitives = mesh.primitives();
        primitives.for_each(|primitive| {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

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
                // println!("gltf: no tangents in file, calculating from tris");
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

                    for i in 0..3 {
                        let sz = chunk[i] as usize;
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
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            meshes.push(Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: indices.len() as u32,
                material: primitive.material().index().unwrap_or(0),
            });
        });
    }

    println!("gltf: Loading materials");
    for material in document.materials() {
        let pbr = material.pbr_metallic_roughness();

        // diffuse
        let diffuse_index = pbr
            .base_color_texture()
            .map(|tex| {
                // println!("gltf: get diffuse tex");
                tex.texture().source().index()
            })
            .unwrap_or(0); // TODO default tex

        let diffuse_data = &mut images[diffuse_index];

        if diffuse_data.format == gltf::image::Format::R8G8B8
            || diffuse_data.format == gltf::image::Format::R16G16B16
        {
            diffuse_data.pixels =
                gltf_pixels_to_wgpu(diffuse_data.pixels.clone(), diffuse_data.format);
        }

        let diffuse_texture = Texture::from_pixels(
            device,
            queue,
            &diffuse_data.pixels,
            (diffuse_data.width, diffuse_data.height),
            gltf_image_format_stride(diffuse_data.format),
            gltf_image_format_to_wgpu(diffuse_data.format, true),
            Some(file_name),
        )
        .unwrap();

        // normal
        let normal_index = material
            .normal_texture()
            .map(|tex| {
                // println!("gltf: get normal tex");
                tex.texture().source().index()
            })
            .unwrap_or(0); // TODO default tex

        let normal_data = &mut images[normal_index];

        if normal_data.format == gltf::image::Format::R8G8B8
            || normal_data.format == gltf::image::Format::R16G16B16
        {
            normal_data.pixels =
                gltf_pixels_to_wgpu(normal_data.pixels.clone(), normal_data.format);
        }

        let normal_texture = Texture::from_pixels(
            device,
            queue,
            &normal_data.pixels,
            (normal_data.width, normal_data.height),
            gltf_image_format_stride(normal_data.format),
            gltf_image_format_to_wgpu(normal_data.format, false),
            Some(file_name),
        )
        .unwrap();

        // roughness-metalness
        let rm_index = pbr
            .metallic_roughness_texture()
            .map(|tex| {
                // println!("gltf: get roughness metalness tex");
                tex.texture().source().index()
            })
            .unwrap_or(0); // TODO default tex

        let rm_data = &mut images[rm_index];
        // dbg!(rm_data.format);

        if rm_data.format == gltf::image::Format::R8G8B8
            || rm_data.format == gltf::image::Format::R16G16B16
        {
            rm_data.pixels =
                gltf_pixels_to_wgpu(rm_data.pixels.clone(), rm_data.format);
        }

        let rm_texture = Texture::from_pixels(
            device,
            queue,
            &rm_data.pixels,
            (rm_data.width, rm_data.height),
            gltf_image_format_stride(rm_data.format),
            gltf_image_format_to_wgpu(rm_data.format, false),
            Some(file_name),
        )
        .unwrap();

        materials.push(Material::new(
            device,
            material.name().unwrap_or("Default Material"),
            diffuse_texture,
            normal_texture,
            rm_texture,
            pbr.metallic_factor(),
            pbr.roughness_factor(),
            layout,
        ));
    }

    println!("gltf: load done!");

    Ok(Model { meshes, materials })
}

fn gltf_image_format_to_wgpu(format: gltf::image::Format, srgb: bool) -> wgpu::TextureFormat {
    if srgb {
        return match format {
            gltf::image::Format::R8 => panic!(),
            gltf::image::Format::R8G8 => panic!(),
            gltf::image::Format::R8G8B8 => wgpu::TextureFormat::Rgba8UnormSrgb, // converted
            gltf::image::Format::R8G8B8A8 => wgpu::TextureFormat::Rgba8UnormSrgb,
            gltf::image::Format::R16 => panic!(),
            gltf::image::Format::R16G16 => panic!(),
            gltf::image::Format::R16G16B16 => panic!(), // converted
            gltf::image::Format::R16G16B16A16 => panic!(),
            gltf::image::Format::R32G32B32FLOAT => panic!(),
            gltf::image::Format::R32G32B32A32FLOAT => panic!(),
        };
    }

    match format {
        gltf::image::Format::R8 => wgpu::TextureFormat::R8Unorm,
        gltf::image::Format::R8G8 => wgpu::TextureFormat::Rg8Unorm,
        gltf::image::Format::R8G8B8 => wgpu::TextureFormat::Rgba8Unorm, // converted
        gltf::image::Format::R8G8B8A8 => wgpu::TextureFormat::Rgba8Unorm,
        gltf::image::Format::R16 => wgpu::TextureFormat::R16Unorm,
        gltf::image::Format::R16G16 => wgpu::TextureFormat::Rg16Unorm,
        gltf::image::Format::R16G16B16 => wgpu::TextureFormat::Rgba16Unorm, // converted
        gltf::image::Format::R16G16B16A16 => wgpu::TextureFormat::Rgba16Unorm,
        gltf::image::Format::R32G32B32FLOAT => wgpu::TextureFormat::Rgba32Float   ,
        gltf::image::Format::R32G32B32A32FLOAT => wgpu::TextureFormat::Rgba32Float,
    }
}

fn gltf_image_format_stride(format: gltf::image::Format) -> u32 {
    match format {
        gltf::image::Format::R8 => 1,
        gltf::image::Format::R8G8 => 2,
        gltf::image::Format::R8G8B8 => 4, // converted
        gltf::image::Format::R8G8B8A8 => 4,
        gltf::image::Format::R16 => 2,
        gltf::image::Format::R16G16 => 4,
        gltf::image::Format::R16G16B16 => 8, // converted
        gltf::image::Format::R16G16B16A16 => 8,
        gltf::image::Format::R32G32B32FLOAT => 12,
        gltf::image::Format::R32G32B32A32FLOAT => 16,
    }
}

// Add alpha if needed
fn gltf_pixels_to_wgpu(mut bytes: Vec<u8>, format: gltf::image::Format) -> Vec<u8> {
    if format == gltf::image::Format::R8G8B8 {
        let pixels = bytes.len() / 3;
        bytes.reserve_exact(pixels);
        bytes = bytes
            .chunks_exact(3)
            .flat_map(|s| [s[0], s[1], s[2], 255])
            .collect();
    } else if format == gltf::image::Format::R16G16B16 {
        let pixels = bytes.len() / 6;
        bytes.reserve_exact(pixels);
        bytes = bytes
            .chunks_exact(6)
            .flat_map(|s| [s[0], s[1], s[2], s[3], s[4], s[5], 255, 255])
            .collect();
    }

    bytes
}
