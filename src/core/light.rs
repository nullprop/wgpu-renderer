use std::ops::Range;

use super::model::{Mesh, Model};

use cgmath::{Matrix4, Vector3};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    _padding: u32,
    pub color: [f32; 4],
    pub matrices: [[[f32; 4]; 4]; 6],
    pub active_matrix: u32,
    _padding2: [u32; 3],
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        let proj = cgmath::perspective(cgmath::Deg(90.0), 1.0, 0.1, 1000.0);
        #[rustfmt::skip]
        let matrices: [[[f32; 4]; 4]; 6] = [
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new( 1.0, 0.0, 0.0), Vector3::new(0.0,-1.0, 0.0))).into(),
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.0,-1.0, 0.0))).into(),
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new( 0.0, 1.0, 0.0), Vector3::new(0.0, 0.0, 1.0))).into(),
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new( 0.0,-1.0, 0.0), Vector3::new(0.0, 0.0,-1.0))).into(),
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new( 0.0, 0.0, 1.0), Vector3::new(0.0,-1.0, 0.0))).into(),
            (proj * Matrix4::look_to_rh(position.into(), Vector3::new( 0.0, 0.0,-1.0), Vector3::new(0.0,-1.0, 0.0))).into(),
        ];

        Self {
            position,
            _padding: 0,
            color,
            matrices: matrices,
            active_matrix: 0,
            _padding2: [0, 0, 0]
        }
    }
}

pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(
                mesh,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}
