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
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some((label.to_owned() + " pipeline Layout").as_str()),
            bind_group_layouts: bind_group_layouts,
            push_constant_ranges: push_constant_ranges,
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
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(shader);

        let fragment_targets = &[Some(wgpu::ColorTargetState {
            format: color_format.unwrap_or(wgpu::TextureFormat::Bgra8Unorm),
            blend: Some(wgpu::BlendState {
                alpha: wgpu::BlendComponent::REPLACE,
                color: wgpu::BlendComponent::REPLACE,
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
            fragment: fragment,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
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
