use std::ops::Range;
use crate::core::material::Material;
use crate::core::mesh::Mesh;

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tangent
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // bitangent
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, bind_groups, add_texture_binds);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        for (i, group) in bind_groups.iter().enumerate() {
            self.set_bind_group(i as u32, group, &[]);
        }
        if add_texture_binds {
            self.set_bind_group(bind_groups.len() as u32, &material.bind_group, &[]);
        }
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    ) {
        self.draw_model_instanced(model, 0..1, bind_groups, add_texture_binds);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        bind_groups: Vec<&'a wgpu::BindGroup>,
        add_texture_binds: bool,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                bind_groups.clone(),
                add_texture_binds,
            );
        }
    }
}
