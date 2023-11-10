use std::ops::Range;

use super::{
    camera::{FAR_PLANE, NEAR_PLANE},
    model::{Model},
    mesh::{Mesh},
};

use cgmath::{Matrix4, Vector3};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    _padding: u32,
    pub color: [f32; 4],
    pub matrices: [[[f32; 4]; 4]; 6],
}

impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        let mut s = Self {
            position,
            _padding: 0,
            color,
            ..Default::default()
        };
        s.update_matrices();
        s
    }

    pub fn update_matrices(&mut self) {
        let proj = cgmath::perspective(cgmath::Deg(90.0), 1.0, NEAR_PLANE, FAR_PLANE);
        self.matrices = [
            (proj * Matrix4::look_to_rh(self.position.into(), Vector3::unit_x(), Vector3::unit_y())).into(), // forward
            (proj * Matrix4::look_to_rh(self.position.into(), -Vector3::unit_x(), Vector3::unit_y())).into(), // back
            (proj * Matrix4::look_to_rh(self.position.into(), Vector3::unit_y(), Vector3::unit_x())).into(), // up
            (proj * Matrix4::look_to_rh(self.position.into(), -Vector3::unit_y(), -Vector3::unit_x())).into(), // down
            (proj * Matrix4::look_to_rh(self.position.into(), Vector3::unit_z(), Vector3::unit_y())).into(), // right
            (proj * Matrix4::look_to_rh(self.position.into(), -Vector3::unit_z(), Vector3::unit_y())).into(), // left
        ];
    }
}

pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        global_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        global_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        global_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        global_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
    where
        'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b Mesh,
        global_bind_group: &'b wgpu::BindGroup
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, global_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        global_bind_group: &'b wgpu::BindGroup
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, global_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b Model,
        global_bind_group: &'b wgpu::BindGroup
    ) {
        self.draw_light_model_instanced(model, 0..1, global_bind_group);
    }

    fn draw_light_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        global_bind_group: &'b wgpu::BindGroup
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(
                mesh,
                instances.clone(),
                global_bind_group
            );
        }
    }
}
