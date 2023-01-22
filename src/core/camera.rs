use std::time::Duration;

use cgmath::num_traits::clamp;
use winit::{dpi::PhysicalPosition, event::*};

pub struct Camera {
    pub position: cgmath::Point3<f32>,
    pub pitch: f32,
    pub yaw: f32,
    pub projection: Projection,
}

pub struct Projection {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn get_matrix(&self) -> cgmath::Matrix4<f32> {
        return cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
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
            position: position,
            pitch: pitch,
            yaw: yaw,
            projection: Projection {
                aspect: aspect,
                fovy: fovy,
                znear: 0.1,
                zfar: 3000.0,
            },
        }
    }

    pub fn get_view_matrix(&self) -> cgmath::Matrix4<f32> {
        let (_right, up, forward) = self.get_vecs();
        return cgmath::Matrix4::look_to_rh(self.position, forward, up);
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
        return (right, up, forward);
    }

    pub fn update(&mut self, dt: Duration, controller: &CameraController) {
        let dt = dt.as_secs_f32();
        self.pitch = clamp(
            self.pitch - controller.deltay * controller.sensitivity * 0.022,
            -89.0,
            89.0,
        );
        self.yaw += controller.deltax * controller.sensitivity * 0.022;
        self.yaw = self.yaw % 360.0;
        if self.yaw < 0.0 {
            self.yaw = 360.0 + self.yaw;
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
    pub position: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view: cgmath::Matrix4::identity().into(),
            proj: cgmath::Matrix4::identity().into(),
            position: [0.0; 4],
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view = camera.get_view_matrix().into();
        self.proj = camera.projection.get_matrix().into();
        self.position = camera.position.to_homogeneous().into();
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
        let mut handled = match window_event {
            None => false,
            Some(event) => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => {
                    let is_pressed = *state == ElementState::Pressed;
                    let amount = if is_pressed { 1.0 } else { 0.0 };
                    match keycode {
                        VirtualKeyCode::W | VirtualKeyCode::Up => {
                            self.move_forward = amount;
                            return true;
                        }
                        VirtualKeyCode::A | VirtualKeyCode::Left => {
                            self.move_left = amount;
                            return true;
                        }
                        VirtualKeyCode::S | VirtualKeyCode::Down => {
                            self.move_backward = amount;
                            return true;
                        }
                        VirtualKeyCode::D | VirtualKeyCode::Right => {
                            self.move_right = amount;
                            return true;
                        }
                        VirtualKeyCode::Space => {
                            self.move_up = amount;
                            return true;
                        }
                        VirtualKeyCode::LControl => {
                            self.move_down = amount;
                            return true;
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
                        return true;
                    }
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                        if *scroll > 0.0 {
                            self.speed *= 2.0;
                        } else {
                            self.speed /= 2.0;
                        }
                        return true;
                    }
                },
                _ => false,
            },
        };

        if handled {
            return true;
        }

        handled = match device_event {
            None => false,
            Some(event) => match event {
                DeviceEvent::MouseMotion { delta } => {
                    self.deltax += delta.0 as f32;
                    self.deltay += delta.1 as f32;
                    return true;
                }
                _ => false,
            },
        };

        return handled;
    }
}
