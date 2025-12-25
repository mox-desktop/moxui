mod blur;

use crate::buffers::{self, DataDescription, GpuBuffer};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TextureInstance {
    pub filters1: [f32; 4],       // [opacity, brightness, contrast, saturation]
    pub filters2: [f32; 4],       // [hue_rotate, sepia, invert, grayscale]
    pub rotation_depth: [f32; 2], // [rotation, depth]
    pub scale: [f32; 2],
    pub skew: [f32; 2],
    pub rect: [f32; 4],
    pub radius: [f32; 4],
    pub texture_bounds: [f32; 4],
    pub shadow: [f32; 3],
}

impl DataDescription for TextureInstance {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Instance;

    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x2,
        4 => Float32x2,
        5 => Float32x2,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x3,
    ];
}

impl buffers::instance::Instance for TextureInstance {}

#[derive(Debug, Clone, Copy)]
pub struct Filters {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue_rotate: f32,
    pub sepia: f32,
    pub invert: f32,
    pub grayscale: f32,
    pub opacity: f32,
    pub blur: u32,
    pub blur_color: [f32; 4],
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue_rotate: 0.0,
            sepia: 0.0,
            invert: 0.0,
            grayscale: 0.0,
            blur: 0,
            blur_color: [0., 0., 0., 0.],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transforms {
    pub rotate: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub skew_x: f32,
    pub skew_y: f32,
    pub translate: [f32; 2],
}

impl Default for Transforms {
    fn default() -> Self {
        Self {
            rotate: 0.,
            scale_x: 1.,
            scale_y: 1.,
            skew_x: 0.,
            skew_y: 0.,
            translate: [0., 0.],
        }
    }
}

pub struct Buffer<'a> {
    width: f32,
    height: f32,
    skew: [f32; 2],
    bytes: &'a [u8],
    filters: Filters,
    scale: [f32; 2],
}

impl<'a> Default for Buffer<'a> {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            skew: [0.0, 0.0],
            bytes: &[],
            filters: Filters::default(),
            scale: [1.0, 1.0],
        }
    }
}

impl<'a> Buffer<'a> {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    pub fn set_bytes(&mut self, bytes: &'a [u8]) {
        self.bytes = bytes;
    }

    pub fn set_size(&mut self, width_opt: Option<f32>, height_opt: Option<f32>) {
        if let Some(width) = width_opt {
            self.width = width;
        }

        if let Some(height) = height_opt {
            self.height = height;
        }
    }

    pub fn set_skew(&mut self, skew_x: f32, skew_y: f32) {
        self.skew = [skew_x, skew_y];
    }

    pub fn set_opacity(&mut self, val: f32) {
        self.filters.opacity = val;
    }

    pub fn set_brightness(&mut self, val: f32) {
        self.filters.brightness = val;
    }

    pub fn set_contrast(&mut self, val: f32) {
        self.filters.contrast = val;
    }

    pub fn set_saturation(&mut self, val: f32) {
        self.filters.saturation = val;
    }

    pub fn set_hue_rotate(&mut self, deg: f32) {
        self.filters.hue_rotate = deg;
    }

    pub fn set_sepia(&mut self, val: f32) {
        self.filters.sepia = val;
    }

    pub fn set_invert(&mut self, val: f32) {
        self.filters.invert = val;
    }

    pub fn set_grayscale(&mut self, val: f32) {
        self.filters.grayscale = val;
    }

    pub fn set_blur(&mut self, val: u32) {
        self.filters.blur = val;
    }

    pub fn set_blur_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.filters.blur_color = [r, g, b, a];
    }

    pub fn set_scale(&mut self, scale_x: f32, scale_y: f32) {
        self.scale = [scale_x, scale_y];
    }
}

pub struct TextureRenderer {
    blur: blur::BlurRenderer,
    render_pipeline: wgpu::RenderPipeline,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    vertex_buffer: buffers::VertexBuffer,
    index_buffer: buffers::IndexBuffer,
    instance_buffer: buffers::instance::InstanceBuffer<TextureInstance>,
    height: f32,
    max_texture_width: u32,
    max_texture_height: u32,
    prepared_instances: usize,
}

pub struct TextureArea<'a> {
    pub left: f32,
    pub top: f32,
    pub scale: f32,
    pub rotation: f32,
    pub bounds: TextureBounds,
    pub skew: [f32; 2],
    pub radius: [f32; 4],
    pub buffer: Buffer<'a>,
    pub depth: f32,
}

#[derive(Clone)]
pub struct TextureBounds {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl TextureBounds {
    pub fn width(&self) -> u32 {
        self.right - self.left
    }

    pub fn height(&self) -> u32 {
        self.bottom - self.top
    }
}

// Helper function for simple texture rendering (like moxnotify)
impl<'a> TextureArea<'a> {
    pub fn simple(
        data: &'a [u8],
        left: f32,
        top: f32,
        width: f32,
        height: f32,
        bounds: TextureBounds,
        radius: [f32; 4],
        _border_size: [f32; 4],
        depth: f32,
    ) -> Self {
        let mut buffer = Buffer::new(width, height);
        buffer.set_bytes(data);

        Self {
            left,
            top,
            scale: 1.0,
            rotation: 0.0,
            bounds,
            skew: [0.0, 0.0],
            radius,
            buffer,
            depth,
        }
    }
}

impl TextureRenderer {
    pub fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        max_icon_size: u32,
        width: u32,
        height: u32,
    ) -> Self {
        Self::with_layers(device, texture_format, max_icon_size, width, height, 256)
    }

    pub fn with_layers(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        max_icon_size: u32,
        width: u32,
        height: u32,
        max_textures: u32,
    ) -> Self {
        Self::with_texture_dimensions(
            device,
            texture_format,
            max_icon_size,
            max_icon_size,
            width,
            height,
            max_textures,
        )
    }

    pub fn with_texture_dimensions(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        texture_width: u32,
        texture_height: u32,
        width: u32,
        height: u32,
        max_textures: u32,
    ) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
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
                ],
                label: Some("texture_bind_group_layout"),
            });

        let viewport_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Viewport Bind Group Layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("texture_render_pipeline_layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &viewport_bind_group_layout],
                immediate_size: 0,
            });

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[buffers::Vertex::desc(), TextureInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        let texture_size = wgpu::Extent3d {
            width: texture_width,
            height: texture_height,
            depth_or_array_layers: max_textures,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture_renderer_texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            base_array_layer: 0,
            array_layer_count: Some(max_textures),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture_renderer_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let vertex_buffer = buffers::VertexBuffer::new(
            device,
            &[
                buffers::Vertex {
                    position: [0.0, 0.0],
                },
                buffers::Vertex {
                    position: [1.0, 0.0],
                },
                buffers::Vertex {
                    position: [0.0, 1.0],
                },
                buffers::Vertex {
                    position: [1.0, 1.0],
                },
            ],
        );

        let index_buffer = buffers::IndexBuffer::new(device, &[0, 1, 2, 3]);

        let instance_buffer = buffers::instance::InstanceBuffer::new(device, &[]);

        Self {
            prepared_instances: 0,
            max_texture_width: texture_width,
            max_texture_height: texture_height,
            instance_buffer,
            render_pipeline,
            texture,
            index_buffer,
            vertex_buffer,
            bind_group,
            blur: blur::BlurRenderer::new(device, texture_format, width, height),
            height: 0.,
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        width: f32,
        height: f32,
    ) {
        self.height = height;
        // Resize blur textures to match new surface size
        self.blur
            .resize(device, width as u32, height as u32, texture_format);
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &[TextureArea],
    ) {
        self.prepared_instances = textures.len();

        if textures.is_empty() {
            return;
        }

        let mut instances = Vec::new();

        textures.iter().enumerate().for_each(|(i, texture)| {
            instances.push(TextureInstance {
                filters1: [
                    texture.buffer.filters.opacity,
                    texture.buffer.filters.brightness,
                    texture.buffer.filters.contrast,
                    texture.buffer.filters.saturation,
                ],
                filters2: [
                    texture.buffer.filters.hue_rotate,
                    texture.buffer.filters.sepia,
                    texture.buffer.filters.invert,
                    texture.buffer.filters.grayscale,
                ],
                rotation_depth: [texture.rotation, texture.depth],
                scale: texture.buffer.scale,
                skew: texture.skew,
                rect: [
                    texture.left,
                    texture.top,
                    texture.buffer.width,
                    texture.buffer.height,
                ],
                radius: texture.radius,
                texture_bounds: [
                    texture.bounds.left as f32,
                    texture.bounds.top as f32,
                    texture.bounds.right as f32,
                    texture.bounds.bottom as f32,
                ],
                shadow: [0., 0., 0.],
            });

            // Calculate actual texture dimensions and bytes_per_row
            let tex_width = (texture.buffer.width as u32).min(self.max_texture_width);
            let tex_height = (texture.buffer.height as u32).min(self.max_texture_height);

            // bytes_per_row must be aligned to 256 bytes for wgpu
            let unpadded_bytes_per_row = 4 * tex_width;
            let bytes_per_row = unpadded_bytes_per_row.div_ceil(256) * 256;

            // Check if we need to pad the data
            if bytes_per_row != unpadded_bytes_per_row {
                // Need to pad each row to meet alignment requirement
                let mut padded_data = Vec::with_capacity((bytes_per_row * tex_height) as usize);
                for y in 0..tex_height {
                    let row_start = (y * unpadded_bytes_per_row) as usize;
                    let row_end = row_start + unpadded_bytes_per_row as usize;

                    // Copy the actual row data
                    if row_end <= texture.buffer.bytes.len() {
                        padded_data.extend_from_slice(&texture.buffer.bytes[row_start..row_end]);
                        // Add padding to reach bytes_per_row alignment
                        padded_data.resize(
                            padded_data.len() + (bytes_per_row - unpadded_bytes_per_row) as usize,
                            0,
                        );
                    }
                }

                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: i as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &padded_data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width: tex_width,
                        height: tex_height,
                        depth_or_array_layers: 1,
                    },
                );
            } else {
                // No padding needed, use data as-is
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: i as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    texture.buffer.bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width: tex_width,
                        height: tex_height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        });

        let instance_buffer_size = std::mem::size_of::<TextureInstance>() * instances.len();

        if self.instance_buffer.size() < instance_buffer_size as u32 {
            self.instance_buffer =
                buffers::instance::InstanceBuffer::with_size(device, instance_buffer_size as u64);
        }

        self.instance_buffer.write(queue, &instances);

        self.blur.prepare(device, queue, textures);
    }

    pub fn render(
        &self,
        texture_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: &crate::viewport::Viewport,
    ) {
        if self.prepared_instances == 0 {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("standard_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.blur.intermediate_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &viewport.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(
            0..self.index_buffer.size(),
            0,
            0..self.prepared_instances as u32,
        );

        drop(render_pass);

        self.blur.render(
            texture_view,
            encoder,
            viewport,
            &self.vertex_buffer,
            &self.index_buffer,
        );
    }
}

pub fn create_depth_buffer(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("DepthBuffer"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, view)
}
