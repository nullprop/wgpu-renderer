use wgpu::{
    BindGroupLayout, Device, PushConstantRange, RenderPipeline, TextureFormat, VertexBufferLayout,
};

use crate::shaders::preprocessor::preprocess_wgsl;

pub struct RenderPass {
    pub pipeline: RenderPipeline,
}

impl RenderPass {
    pub fn new(
        device: &Device,
        bind_group_layouts: &[&BindGroupLayout],
        push_constant_ranges: &[PushConstantRange],
        shader_name: &str,
        color_format: Option<TextureFormat>,
        depth_format: Option<TextureFormat>,
        vertex_layouts: &[VertexBufferLayout],
        label: &str,
        is_shadow: bool,
        has_transparency: bool,
        write_depth: bool,
        cull_mode: Option<wgpu::Face>,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some((label.to_owned() + " pipeline Layout").as_str()),
            bind_group_layouts,
            push_constant_ranges,
        });
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some(shader_name),
            source: preprocess_wgsl(shader_name),
        };
        let pipeline = Self::create_render_pipeline(
            device,
            &layout,
            color_format,
            depth_format,
            vertex_layouts,
            shader,
            label,
            is_shadow,
            has_transparency,
            write_depth,
            cull_mode,
        );

        Self { pipeline }
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: Option<wgpu::TextureFormat>,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: wgpu::ShaderModuleDescriptor,
        label: &str,
        is_shadow: bool,
        has_transparency: bool,
        write_depth: bool,
        cull_mode: Option<wgpu::Face>,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(shader);

        let blend_comp = if has_transparency {
            wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            }
        } else {
            wgpu::BlendComponent::REPLACE
        };

        let fragment_targets = &[Some(wgpu::ColorTargetState {
            format: color_format.unwrap_or(wgpu::TextureFormat::Bgra8Unorm),
            blend: Some(wgpu::BlendState {
                alpha: blend_comp,
                color: blend_comp,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let fragment = match color_format {
            Some(..) => Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: fragment_targets,
            }),
            None => None,
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some((label.to_owned() + " pipeline Layout").as_str()),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_layouts,
            },
            fragment,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: write_depth,
                depth_compare: if is_shadow { wgpu::CompareFunction::LessEqual } else { wgpu::CompareFunction::Less },
                stencil: wgpu::StencilState::default(),
                bias: if is_shadow {
                    wgpu::DepthBiasState {
                        constant: 2, // bilinear
                        slope_scale: 2.0,
                        clamp: 0.0,
                    }
                } else { wgpu::DepthBiasState::default() },
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }
}
