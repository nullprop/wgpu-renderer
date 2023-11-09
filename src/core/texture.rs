use anyhow::*;

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        label: &str,
        compare: Option<wgpu::CompareFunction>,
        width: u32,
        height: u32,
        layers: u32,
        usage: wgpu::TextureUsages,
        filter: bool,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{:?}_view", label)),
            format: Some(Self::DEPTH_FORMAT),
            dimension: if layers > 1 { Some(wgpu::TextureViewDimension::D2Array) } else { Some(wgpu::TextureViewDimension::D2) },
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: if layers > 1 { Some(layers) } else { None },
        });
        let sampler = Texture::create_sampler(device, compare, filter);

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn create_sampler(device: &wgpu::Device, compare: Option<wgpu::CompareFunction>, filter: bool) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: if filter { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest },
            min_filter: if filter { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest },
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        })
    }

    pub fn from_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: &[u8],
        dimensions: (u32, u32),
        stride: u32,
        format: wgpu::TextureFormat,
        label: Option<&str>,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // dbg!(stride);
        // dbg!(dimensions);
        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(stride * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
