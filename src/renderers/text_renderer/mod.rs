use wgpu::{MultisampleState, TextureFormat};

pub struct TextRenderer {
    pub swash_cache: glyphon::SwashCache,
    pub viewport: glyphon::Viewport,
    pub atlas: glyphon::TextAtlas,
    pub renderer: glyphon::TextRenderer,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, texture_format: TextureFormat) -> Self {
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, texture_format);
        let renderer = glyphon::TextRenderer::new(
            &mut atlas,
            device,
            MultisampleState::default(),
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
        );

        Self {
            swash_cache,
            viewport: glyphon::Viewport::new(device, &cache),
            atlas,
            renderer,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: Vec<glyphon::TextArea>,
        font_system: &mut glyphon::FontSystem,
    ) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        self.renderer.prepare_with_depth(
            device,
            queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            text,
            &mut self.swash_cache,
            |metadata| f32::from_bits(metadata as u32),
        )?;

        Ok(())
    }

    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass) -> anyhow::Result<()> {
        self.renderer
            .render(&self.atlas, &self.viewport, render_pass)?;

        Ok(())
    }
}
