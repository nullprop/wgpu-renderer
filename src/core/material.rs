use wgpu::util::DeviceExt;
use crate::core::texture::Texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub normal_texture: Texture,
    pub metallic_roughness_texture: Texture,
    pub material_uniform: MaterialUniform,
    pub bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    // metallic, roughness, none, none
    pub factors: [f32; 4],
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: Texture,
        normal_texture: Texture,
        metallic_roughness_texture: Texture,
        metallic_factor: f32,
        roughness_factor: f32,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let material_uniform = MaterialUniform {
            factors: [metallic_factor, roughness_factor, 0.0, 0.0]
        };
        let material_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Uniform UB"),
            contents: bytemuck::cast_slice(&[material_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                // diffuse
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                // normal
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
                // metallic roughness
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&metallic_roughness_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&metallic_roughness_texture.sampler),
                },
                // uniform
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: material_uniform_buffer.as_entire_binding(),
                }
            ],
            label: None,
        });

        Self {
            name: String::from(name),
            diffuse_texture,
            normal_texture,
            metallic_roughness_texture,
            material_uniform,
            bind_group,
        }
    }
}