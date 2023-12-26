use cgmath::prelude::*;
use std::default::Default;
use std::mem;
use std::time::Duration;

use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};
use crate::core::material::MaterialUniform;

use super::camera::{Camera, CameraController, CameraUniform};
use super::instance::{Instance, InstanceRaw};
use super::light::{DrawLight, LightUniform};
use super::model::{DrawModel, Model, ModelVertex, Vertex};
use super::pass::RenderPass;
use super::resources;
use super::texture::Texture;

const SHADOW_MAP_SIZE: u32 = 2048;
const SHADOW_MAP_LAYERS: u32 = 6;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobalUniforms {
    pub time: f32,
    pub light_matrix_index: u32,
    pub use_shadowmaps: u32,
    pub _padding: u32,
}

pub struct State {
    pub size: winit::dpi::PhysicalSize<u32>,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    geometry_pass: RenderPass,
    #[cfg(not(target_arch = "wasm32"))]
    fog_pass: RenderPass,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    global_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    geom_instances: Vec<Instance>,
    geom_instance_buffer: wgpu::Buffer,
    #[cfg(not(target_arch = "wasm32"))]
    fog_instances: Vec<Instance>,
    #[cfg(not(target_arch = "wasm32"))]
    fog_instance_buffer: wgpu::Buffer,
    geometry_depth_texture: Texture,
    geom_model: Model,
    #[cfg(not(target_arch = "wasm32"))]
    fog_model: Model,
    light_model: Model,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_debug_pass: RenderPass,
    light_depth_bind_group: wgpu::BindGroup,
    geometry_depth_bind_group: wgpu::BindGroup,
    geometry_depth_bind_group_layout: wgpu::BindGroupLayout,
    light_depth_pass: RenderPass,
    light_depth_texture_target_views: [wgpu::TextureView; SHADOW_MAP_LAYERS as usize],
    global_uniforms: GlobalUniforms,
    global_uniforms_buffer: wgpu::Buffer,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        log::info!("Creating surface");
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::PRIMARY | wgpu::Backends::GL, ..Default::default() });
        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("failed to get adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
                    limits: if cfg!(target_arch = "wasm32") {
                        // TODO: remove once webgpu?
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .expect("failed to get device");

        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![caps.formats[0]],
        };

        surface.configure(&device, &config);

        let camera = Camera::new(
            (-500.0, 150.0, 0.0).into(),
            0.0,
            0.0,
            55.0,
            config.width as f32 / config.height as f32,
        );

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera, &config);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_uniform_size = mem::size_of::<CameraUniform>() as u64;

        let light_uniform = LightUniform::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 250000.0]);
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light UB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_uniform_size = mem::size_of::<LightUniform>() as u64;

        let global_uniforms = GlobalUniforms::default();
        let global_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Matrix UB"),
            contents: bytemuck::cast_slice(&[global_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let global_uniform_size = mem::size_of::<GlobalUniforms>() as u64;

        let global_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // CameraUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(camera_uniform_size),
                        },
                        count: None,
                    },
                    // LightUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(light_uniform_size),
                        },
                        count: None,
                    },
                    // global_uniforms
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(global_uniform_size),
                        },
                        count: None,
                    },
                ],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &global_bind_group_layout,
            entries: &[
                // CameraUniform
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                // light struct
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                // global_uniforms
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: global_uniforms_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let camera_controller = CameraController::new(400.0, 2.0);

        let geometry_depth_texture = State::create_geometry_depth_texture(&device, &config);

        let light_depth_texture = Texture::create_depth_texture(
            &device,
            "light_depth_texture",
            Some(wgpu::CompareFunction::LessEqual),
            SHADOW_MAP_SIZE,
            SHADOW_MAP_SIZE,
            SHADOW_MAP_LAYERS,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            true,
        );

        let light_depth_texture_target_views = (0..SHADOW_MAP_LAYERS)
            .map(|i| {
                light_depth_texture.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("light_depth_texture_view"),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: i,
                    array_layer_count: Some(1),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("failed to create light depth texture views");

        let light_depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // light cubemap
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
                label: Some("Light Bind Group Layout"),
            });

        let light_depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_depth_bind_group_layout,
            entries: &[
                // light cubemap
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&light_depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&light_depth_texture.sampler),
                },
            ],
            label: Some("Light Bind Group"),
        });

        let geometry_depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // geometry depth
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Depth Bind Group Layout"),
            });

        let geometry_depth_bind_group = State::create_geometry_depth_bind_group(&device, &geometry_depth_bind_group_layout, &geometry_depth_texture);

        let material_uniform_size = mem::size_of::<MaterialUniform>() as u64;
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // diffuse
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // normal
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // metallic + roughness
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // material uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(material_uniform_size),
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let geom_model = resources::load_model_gltf(
            "models/Sponza.glb",
            &device,
            &queue,
            &texture_bind_group_layout,
        )
            .await
            .unwrap();

        #[cfg(not(target_arch = "wasm32"))]
        let fog_model = resources::load_model_gltf(
            "models/Cube.glb",
            &device,
            &queue,
            &texture_bind_group_layout,
        )
            .await
            .unwrap();

        let light_model = resources::load_model_gltf(
            "models/Cube.glb",
            &device,
            &queue,
            &texture_bind_group_layout,
        )
            .await
            .unwrap();

        let geom_instances = vec![Instance {
            // this sponza model isn't quite centered
            position: [60.0, 0.0, 35.0].into(),
            rotation: cgmath::Quaternion::one(),
            scale: [1.0, 1.0, 1.0].into(),
        }];
        let geom_instance_data = geom_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let geom_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Geometry Instance Buffer"),
            contents: bytemuck::cast_slice(&geom_instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        #[cfg(not(target_arch = "wasm32"))]
        let fog_instances = vec![Instance {
            position: [0.0, 30.0, 0.0].into(),
            rotation: cgmath::Quaternion::one(),
            scale: [1360.0, 30.0, 600.0].into(),
        }];
        #[cfg(not(target_arch = "wasm32"))]
        let fog_instance_data = fog_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        #[cfg(not(target_arch = "wasm32"))]
        let fog_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fog Instance Buffer"),
            contents: bytemuck::cast_slice(&fog_instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let light_depth_pass = RenderPass::new(
            &device,
            &[
                &global_bind_group_layout,
            ],
            &[],
            "depth.wgsl",
            None,
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            "light depth pass",
            true,
            false,
            true,
            Some(wgpu::Face::Back),
        );

        let geometry_pass = RenderPass::new(
            &device,
            &[
                &global_bind_group_layout,
                &light_depth_bind_group_layout,
                &texture_bind_group_layout,
            ],
            &[],
            "pbr.wgsl",
            Some(config.format),
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            "geometry pass",
            false,
            true,
            true,
            Some(wgpu::Face::Back),
        );

        let light_debug_pass = RenderPass::new(
            &device,
            &[&global_bind_group_layout],
            &[],
            "light_debug.wgsl",
            Some(config.format),
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc()],
            "light debug pass",
            false,
            false,
            true,
            Some(wgpu::Face::Back),
        );

        #[cfg(not(target_arch = "wasm32"))]
        let fog_pass = RenderPass::new(
            &device,
            &[
                &global_bind_group_layout,
                &light_depth_bind_group_layout,
                &geometry_depth_bind_group_layout,
            ],
            &[],
            "fog.wgsl",
            Some(config.format),
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            "fog pass",
            false,
            true,
            false,
            Some(wgpu::Face::Back),
        );

        Self {
            size,
            surface,
            device,
            queue,
            config,
            geometry_pass,
            #[cfg(not(target_arch = "wasm32"))]
            fog_pass,
            camera,
            camera_uniform,
            camera_buffer,
            global_bind_group: camera_bind_group,
            camera_controller,
            geom_instances,
            geom_instance_buffer,
            #[cfg(not(target_arch = "wasm32"))]
            fog_instances,
            #[cfg(not(target_arch = "wasm32"))]
            fog_instance_buffer,
            geometry_depth_texture,
            geom_model,
            #[cfg(not(target_arch = "wasm32"))]
            fog_model,
            light_model,
            light_uniform,
            light_buffer,
            light_debug_pass,
            light_depth_bind_group,
            geometry_depth_bind_group,
            geometry_depth_bind_group_layout,
            light_depth_pass,
            light_depth_texture_target_views,
            global_uniforms,
            global_uniforms_buffer,
        }
    }

    pub fn create_geometry_depth_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, geometry_depth_texture: &Texture) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                // geometry depth
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&geometry_depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&geometry_depth_texture.sampler),
                },
            ],
            label: Some("Depth Bind Group"),
        })
    }

    fn create_geometry_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Texture {
        Texture::create_depth_texture(
            device,
            "geometry_depth_texture",
            None,
            config.width,
            config.height,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            true,
        )
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera
                .projection
                .resize(new_size.width, new_size.height);
            self.geometry_depth_texture = State::create_geometry_depth_texture(&self.device, &self.config);
            self.geometry_depth_bind_group = State::create_geometry_depth_bind_group(&self.device, &self.geometry_depth_bind_group_layout, &self.geometry_depth_texture);
        }
    }

    pub fn input(
        &mut self,
        window_event: Option<&WindowEvent>,
        device_event: Option<&DeviceEvent>,
    ) -> bool {
        self.camera_controller
            .process_events(window_event, device_event)
    }

    pub fn update(&mut self, dt: Duration, time: Duration) {
        // Update camera
        self.camera.update(dt, &self.camera_controller);
        self.camera_controller.reset(false);
        self.camera_uniform.update(&self.camera, &self.config);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the light
        self.light_uniform.position[0] = f32::sin(time.as_secs_f32() * 0.5) * 500.0;
        self.light_uniform.position[1] = 250.0 + f32::sin(time.as_secs_f32() * 0.3) * 200.0;
        self.light_uniform.position[2] = f32::sin(time.as_secs_f32() * 0.8) * 100.0;
        self.light_uniform.update_matrices();

        self.light_uniform.color[0] = f32::abs(f32::sin(time.as_secs_f32() * 1.0));
        self.light_uniform.color[1] = f32::abs(f32::sin(time.as_secs_f32() * 0.6));
        self.light_uniform.color[2] = f32::abs(f32::sin(time.as_secs_f32() * 0.4));

        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );

        // Global uniforms
        self.global_uniforms.time = time.as_secs_f32();
        self.global_uniforms.use_shadowmaps = if cfg!(target_arch = "wasm32") { 0u32 } else { 1u32 };
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        // render light to depth textures
        for i in 0..SHADOW_MAP_LAYERS as usize {
            self.global_uniforms.light_matrix_index = i as u32;
            self.queue.write_buffer(
                &self.global_uniforms_buffer,
                0,
                bytemuck::cast_slice(&[self.global_uniforms]),
            );

            let mut depth_encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Depth Encoder"),
                });

            {
                let mut light_depth_render_pass =
                    depth_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Light Depth Render Pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.light_depth_texture_target_views[i],
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                light_depth_render_pass.set_vertex_buffer(1, self.geom_instance_buffer.slice(..));
                light_depth_render_pass.set_pipeline(&self.light_depth_pass.pipeline);
                light_depth_render_pass.draw_model_instanced(
                    &self.geom_model,
                    0..self.geom_instances.len() as u32,
                    [&self.global_bind_group].into(),
                    false,
                );
            }

            self.queue.submit(std::iter::once(depth_encoder.finish()));
        }

        // render geometry
        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut geometry_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        geometry_encoder.push_debug_group("geometry pass");
        {
            let mut geom_render_pass = geometry_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Geometry Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.geometry_depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            geom_render_pass.set_vertex_buffer(1, self.geom_instance_buffer.slice(..));
            geom_render_pass.set_pipeline(&self.geometry_pass.pipeline);
            geom_render_pass.draw_model_instanced(
                &self.geom_model,
                0..self.geom_instances.len() as u32,
                [&self.global_bind_group, &self.light_depth_bind_group].into(),
                true,
            );
        }
        geometry_encoder.pop_debug_group();

        geometry_encoder.push_debug_group("debug light pass");
        {
            let mut light_debug_render_pass =
                geometry_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Light Debug Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.geometry_depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            light_debug_render_pass.set_pipeline(&self.light_debug_pass.pipeline);
            light_debug_render_pass.draw_light_model(
                &self.light_model,
                &self.global_bind_group,
            );
        }
        geometry_encoder.pop_debug_group();

        self.queue.submit(std::iter::once(geometry_encoder.finish()));

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut fog_encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Fog Encoder"),
                });

            fog_encoder.push_debug_group("fog pass");
            {
                let mut fog_render_pass = fog_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Fog Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.geometry_depth_texture.view,
                        depth_ops: None,
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                fog_render_pass.set_vertex_buffer(1, self.fog_instance_buffer.slice(..));
                fog_render_pass.set_pipeline(&self.fog_pass.pipeline);
                fog_render_pass.draw_model_instanced(
                    &self.fog_model,
                    0..self.fog_instances.len() as u32,
                    [&self.global_bind_group, &self.light_depth_bind_group, &self.geometry_depth_bind_group].into(),
                    false,
                );
            }
            fog_encoder.pop_debug_group();

            self.queue.submit(std::iter::once(fog_encoder.finish()));
        }

        surface_texture.present();

        Ok(())
    }
}
