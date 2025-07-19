use crate::{
    buffers::{self, DataDescription, GpuBuffer, instance},
    texture_renderer::viewport,
};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShapeInstance {
    pub rect_pos: [f32; 2],
    pub rect_size: [f32; 2],
    pub rect_color: [f32; 4],
    pub border_radius: [f32; 4],
    pub border_size: [f32; 4],
    pub border_color: [f32; 4],
    pub scale: f32,
    pub depth: f32,
}

impl DataDescription for ShapeInstance {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Instance;

    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32,
        8 => Float32,
    ];
}

impl buffers::instance::Instance for ShapeInstance {}

pub struct ShapeRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: buffers::VertexBuffer,
    index_buffer: buffers::IndexBuffer,
    instance_buffer: instance::InstanceBuffer<ShapeInstance>,
    viewport_bind_group: Option<wgpu::BindGroup>,
}

impl ShapeRenderer {
    pub fn new(device: &wgpu::Device, texture_format: wgpu::TextureFormat) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[buffers::Vertex::desc(), ShapeInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multiview: None,
            cache: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let index_buffer = buffers::IndexBuffer::new(device, &[0, 1, 3, 1, 2, 3]);

        let vertex_buffer = buffers::VertexBuffer::new(
            device,
            &[
                buffers::Vertex {
                    position: [0.0, 1.0],
                },
                buffers::Vertex {
                    position: [1.0, 1.0],
                },
                buffers::Vertex {
                    position: [1.0, 0.0],
                },
                buffers::Vertex {
                    position: [0.0, 0.0],
                },
            ],
        );

        let instance_buffer = instance::InstanceBuffer::new(device, &[]);

        Self {
            instance_buffer,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            viewport_bind_group: None,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        viewport: &viewport::Viewport,
        instances: &[ShapeInstance],
    ) {
        if instances.is_empty() {
            return;
        }

        if self.instance_buffer.size() < std::mem::size_of_val(instances) as u32 {
            self.instance_buffer = instance::InstanceBuffer::with_size(
                device,
                std::mem::size_of_val(instances) as u64,
            );
        }

        self.viewport_bind_group = Some(viewport.bind_group.clone());

        self.instance_buffer.write(queue, instances);
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        let Some(viewport_bind_group) = self.viewport_bind_group.as_ref().take() else {
            return;
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, viewport_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(
            0..self.index_buffer.size(),
            0,
            0..self.instance_buffer.size(),
        );
    }
}
