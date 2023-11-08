use rust_embed::RustEmbed;

use crate::core::model::{Model};
use crate::core::mesh::Mesh;
use crate::core::material::Material;
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
        meshes.extend(Mesh::from_gltf(device, &buffers, &mesh, file_name));
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
        gltf::image::Format::R32G32B32FLOAT => wgpu::TextureFormat::Rgba32Float,
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
