use std::time::Duration;

use cgmath::num_traits::clamp;
use cgmath::SquareMatrix;
use winit::{dpi::PhysicalPosition, event::*};
use winit::keyboard::{PhysicalKey, KeyCode};

pub const NEAR_PLANE: f32 = 1.0;
pub const FAR_PLANE: f32 = 3000.0;

pub struct Camera {
    pub position: cgmath::Point3<f32>,
    pub pitch: f32,
    pub yaw: f32,
    pub projection: PerspectiveProjection,
}

pub struct PerspectiveProjection {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl PerspectiveProjection {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn get_matrix(&self) -> cgmath::Matrix4<f32> {
        cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar)
    }
}

impl Camera {
    pub fn new(
        position: cgmath::Point3<f32>,
        pitch: f32,
        yaw: f32,
        fovy: f32,
        aspect: f32,
    ) -> Self {
        Self {
            position,
            pitch,
            yaw,
            projection: PerspectiveProjection {
                aspect,
                fovy,
                znear: NEAR_PLANE,
                zfar: FAR_PLANE,
            },
        }
    }

    pub fn get_view_matrix(&self) -> cgmath::Matrix4<f32> {
        let (_right, up, forward) = self.get_vecs();
        cgmath::Matrix4::look_to_rh(self.position, forward, up)
    }

    pub fn get_vecs(
        &self,
    ) -> (
        cgmath::Vector3<f32>,
        cgmath::Vector3<f32>,
        cgmath::Vector3<f32>,
    ) {
        use cgmath::InnerSpace;
        let (yaw_sin, yaw_cos) = cgmath::Rad::from(cgmath::Deg(self.yaw)).0.sin_cos();
        let (pitch_sin, pitch_cos) = cgmath::Rad::from(cgmath::Deg(self.pitch)).0.sin_cos();
        let forward =
            cgmath::Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        let right = cgmath::Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        let up = right.cross(forward);
        (right, up, forward)
    }

    pub fn update(&mut self, dt: Duration, controller: &CameraController) {
        let dt = dt.as_secs_f32();
        self.pitch = clamp(
            self.pitch - controller.deltay * controller.sensitivity * 0.022,
            -89.0,
            89.0,
        );
        self.yaw += controller.deltax * controller.sensitivity * 0.022;
        self.yaw %= 360.0;
        if self.yaw < 0.0 {
            self.yaw += 360.0;
        }

        let (right, up, forward) = self.get_vecs();
        self.position +=
            forward * (controller.move_forward - controller.move_backward) * controller.speed * dt;
        self.position +=
            right * (controller.move_right - controller.move_left) * controller.speed * dt;
        self.position += up * (controller.move_up - controller.move_down) * controller.speed * dt;
        // println!(
        //     "camera pos ({}, {}, {})",
        //     self.position.x, self.position.y, self.position.z
        // );
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub position: [f32; 4],
    pub planes: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view: cgmath::Matrix4::identity().into(),
            proj: cgmath::Matrix4::identity().into(),
            inv_view_proj: cgmath::Matrix4::identity().into(),
            position: [0.0; 4],
            planes: [NEAR_PLANE, FAR_PLANE, 1.0, 1.0],
        }
    }

    pub fn update(&mut self, camera: &Camera, config: &wgpu::SurfaceConfiguration) {
        let view = camera.get_view_matrix();
        let proj = camera.projection.get_matrix();
        self.view = view.into();
        self.proj = proj.into();
        let inv_view_proj = (proj * view).invert().unwrap();
        self.inv_view_proj = inv_view_proj.into();
        self.position = camera.position.to_homogeneous().into();
        self.planes = [NEAR_PLANE, FAR_PLANE, config.width as f32, config.height as f32];
    }
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub move_forward: f32,
    pub move_backward: f32,
    pub move_left: f32,
    pub move_right: f32,
    pub move_up: f32,
    pub move_down: f32,
    pub deltax: f32,
    pub deltay: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            move_forward: 0.0,
            move_backward: 0.0,
            move_left: 0.0,
            move_right: 0.0,
            move_up: 0.0,
            move_down: 0.0,
            deltax: 0.0,
            deltay: 0.0,
        }
    }

    pub fn reset(&mut self, held_keys: bool) {
        if held_keys {
            self.move_forward = 0.0;
            self.move_backward = 0.0;
            self.move_left = 0.0;
            self.move_right = 0.0;
            self.move_up = 0.0;
            self.move_down = 0.0;
        }
        self.deltax = 0.0;
        self.deltay = 0.0;
    }

    pub fn process_events(
        &mut self,
        window_event: Option<&WindowEvent>,
        device_event: Option<&DeviceEvent>,
    ) -> bool {
        let handled = match window_event {
            None => false,
            Some(event) => match event {
                WindowEvent::KeyboardInput {
                    event: key_event,
                    ..
                } => {
                    let amount = if key_event.state == ElementState::Pressed { 1.0 } else { 0.0 };
                    match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                            self.move_forward = amount;
                            true
                        }
                        PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            self.move_left = amount;
                            true
                        }
                        PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                            self.move_backward = amount;
                            true
                        }
                        PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                            self.move_right = amount;
                            true
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.move_up = amount;
                            true
                        }
                        PhysicalKey::Code(KeyCode::ControlLeft) => {
                            self.move_down = amount;
                            true
                        }
                        _ => false,
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    MouseScrollDelta::LineDelta(_, scroll) => {
                        if *scroll > 0.0 {
                            self.speed *= 2.0;
                        } else {
                            self.speed /= 2.0;
                        }
                        true
                    }
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        if *scroll > 0.0 {
                            self.speed *= 2.0;
                        } else {
                            self.speed /= 2.0;
                        }
                        true
                    }
                },
                _ => false,
            },
        };

        if handled {
            return true;
        }

        match device_event {
            Some(DeviceEvent::MouseMotion { delta }) => {
                self.deltax += delta.0 as f32;
                self.deltay += delta.1 as f32;
                true
            }
            _ => false,
        }
    }
}
