use cgmath::prelude::*;
use wgpu::{InstanceDescriptor, Backends, TextureView, TextureViewDescriptor, StoreOp};
use std::default::Default;
use std::mem;
use std::time::Duration;

use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

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
    pub light_matrix_index: u32,
    // No DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED in WebGL
    pub use_shadowmaps: u32,
    _padding: [u32; 2],
}

pub struct State {
    pub size: winit::dpi::PhysicalSize<u32>,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    geometry_pass: RenderPass,
    fog_pass: RenderPass,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    geom_instances: Vec<Instance>,
    geom_instance_buffer: wgpu::Buffer,
    fog_instances: Vec<Instance>,
    fog_instance_buffer: wgpu::Buffer,
    depth_texture: Texture,
    geom_model: Model,
    fog_model: Model,
    light_model: Model,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_debug_pass: RenderPass,
    light_bind_group: wgpu::BindGroup,
    light_depth_bind_group: wgpu::BindGroup,
    light_depth_pass: RenderPass,
    light_depth_texture_target_views: [TextureView; SHADOW_MAP_LAYERS as usize],
    light_matrix_uniform: GlobalUniforms,
    light_matrix_buffer: wgpu::Buffer,
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Self {
        log::info!("Creating surface");
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        let instance = wgpu::Instance::new(InstanceDescriptor { backends: Backends::PRIMARY | Backends::GL, ..Default::default() });
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

        // Camera
        let camera = Camera::new(
            (-500.0, 150.0, 0.0).into(),
            0.0,
            0.0,
            55.0,
            config.width as f32 / config.height as f32,
        );

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_uniform_size = mem::size_of::<CameraUniform>() as u64;
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(camera_uniform_size),
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let camera_controller = CameraController::new(400.0, 2.0);

        let depth_texture = Texture::create_depth_texture(
            &device,
            "depth_texture",
            Some(wgpu::CompareFunction::Less),
            config.width,
            config.height,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );

        let light_depth_texture = Texture::create_depth_texture(
            &device,
            "light_depth_texture",
            Some(wgpu::CompareFunction::LessEqual),
            SHADOW_MAP_SIZE,
            SHADOW_MAP_SIZE,
            SHADOW_MAP_LAYERS,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );

        let light_depth_texture_target_views = (0..SHADOW_MAP_LAYERS)
            .map(|i| {
                light_depth_texture.texture.create_view(&TextureViewDescriptor {
                    label: Some("light_depth_texture_view"),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: i,
                    array_layer_count: Some(1),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("failed to create light depth texture views");

        let light_uniform = LightUniform::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 250000.0]);

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light UB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_matrix_uniform = GlobalUniforms::default();
        let light_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Matrix UB"),
            contents: bytemuck::cast_slice(&[light_matrix_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_uniform_size = mem::size_of::<LightUniform>() as u64;
        let light_matrix_uniform_size = mem::size_of::<GlobalUniforms>() as u64;
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // LightUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(light_uniform_size),
                        },
                        count: None,
                    },
                    // matrix index, use shadowmaps
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        // TODO: remove fragment vis once no shadowmap uniform
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(light_matrix_uniform_size),
                        },
                        count: None,
                    },
                ],
                label: Some("Light Bind Group Layout"),
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[
                // light struct
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                // matrix index
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_matrix_buffer.as_entire_binding(),
                },
            ],
            label: Some("Light Bind Group"),
        });

        let light_depth_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // depth textures
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
                label: Some("Light Depth Bind Group Layout"),
            });

        let light_depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_depth_bind_group_layout,
            entries: &[
                // depth textures
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&light_depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&light_depth_texture.sampler),
                },
            ],
            label: Some("Light Depth Bind Group"),
        });

        surface.configure(&device, &config);

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

        let fog_instances = vec![Instance {
            position: [0.0, 20.0, 0.0].into(),
            rotation: cgmath::Quaternion::one(),
            scale: [1360.0, 20.0, 600.0].into(),
        }];
        let fog_instance_data = fog_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let fog_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fog Instance Buffer"),
            contents: bytemuck::cast_slice(&fog_instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let light_depth_pass = RenderPass::new(
            &device,
            &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
            ],
            &[],
            "depth.wgsl",
            None,
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            "light depth pass",
            true,
            false,
        );

        let geometry_pass = RenderPass::new(
            &device,
            &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
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
            false,
        );

        let fog_pass = RenderPass::new(
            &device,
            &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &light_depth_bind_group_layout,
                &texture_bind_group_layout,
            ],
            &[],
            "fog.wgsl",
            Some(config.format),
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            "fog pass",
            false,
            true,
        );

        let light_debug_pass = RenderPass::new(
            &device,
            &[&camera_bind_group_layout, &light_bind_group_layout],
            &[],
            "light.wgsl",
            Some(config.format),
            Some(Texture::DEPTH_FORMAT),
            &[ModelVertex::desc()],
            "light debug pass",
            false,
            false,
        );

        Self {
            size,
            surface,
            device,
            queue,
            config,
            geometry_pass,
            fog_pass,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            geom_instances,
            geom_instance_buffer,
            fog_instances,
            fog_instance_buffer,
            depth_texture,
            geom_model,
            fog_model,
            light_model,
            light_uniform,
            light_buffer,
            light_debug_pass,
            light_bind_group,
            light_depth_bind_group,
            light_depth_pass,
            light_depth_texture_target_views,
            light_matrix_uniform,
            light_matrix_buffer,
        }
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
            self.depth_texture = Texture::create_depth_texture(
                &self.device,
                "depth_texture",
                Some(wgpu::CompareFunction::Less),
                self.config.width,
                self.config.height,
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            );
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
        self.camera_uniform.update(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the light
        self.light_uniform.position[0] = f32::sin(time.as_secs_f32() * 0.5) * 500.0;
        self.light_uniform.position[1] = 300.0 + f32::sin(time.as_secs_f32() * 0.3) * 150.0;
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
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        // render light to depth textures
        for i in 0..SHADOW_MAP_LAYERS as usize {
            self.light_matrix_uniform.light_matrix_index = i as u32;
            self.light_matrix_uniform.use_shadowmaps = if cfg!(target_arch = "wasm32") { 0u32 } else { 1u32 };
            self.queue.write_buffer(
                &self.light_matrix_buffer,
                0,
                bytemuck::cast_slice(&[self.light_matrix_uniform]),
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
                                store: StoreOp::Store,
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
                    [&self.camera_bind_group, &self.light_bind_group].into(),
                );
            }

            self.queue.submit(std::iter::once(depth_encoder.finish()));
        }

        // render geometry
        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        encoder.push_debug_group("geometry pass");
        {
            let mut geom_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: StoreOp::Store,
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
                [&self.camera_bind_group, &self.light_bind_group, &self.light_depth_bind_group].into(),
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("fog pass");
        {
            let mut fog_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fog Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: StoreOp::Store,
                    }),
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
                [&self.camera_bind_group, &self.light_bind_group, &self.light_depth_bind_group].into(),
            );
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("debug light pass");
        {
            let mut light_debug_render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Light Debug Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            light_debug_render_pass.set_pipeline(&self.light_debug_pass.pipeline);
            light_debug_render_pass.draw_light_model(
                &self.light_model,
                &self.camera_bind_group,
                &self.light_bind_group,
            );
        }
        encoder.pop_debug_group();

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}
