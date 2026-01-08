use moxui::{
    texture_renderer::{Buffer, TextureArea, TextureBounds, TextureRenderer},
    viewport::{Resolution, Viewport},
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::wayland::EventLoopBuilderExtWayland,
    window::{Window, WindowId},
};

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::builder()
        .with_wayland()
        .with_any_thread(true)
        .build()
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)
}

#[derive(Default)]
pub struct App<'window> {
    wgpu_ctx: Option<WgpuCtx<'window>>,
    window: Option<Arc<Window>>,
}

impl<'window> ApplicationHandler for App<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr =
                Window::default_attributes().with_title("moxui texture renderer example");
            let window = Arc::new(
                event_loop
                    .create_window(win_attr)
                    .expect("create window err."),
            );
            self.window = Some(window.clone());
            let wgpu_ctx = WgpuCtx::new(window.clone());

            self.wgpu_ctx = Some(wgpu_ctx);

            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(wgpu_ctx) = &mut self.wgpu_ctx {
                    wgpu_ctx.draw();
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if !event.state.is_pressed() {
                    return;
                }

                let Some(mut wgpu_ctx) = self.wgpu_ctx.take() else {
                    return;
                };

                wgpu_ctx.draw();
                self.wgpu_ctx = Some(wgpu_ctx);
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                let Some(ref mut wgpu_ctx) = self.wgpu_ctx else {
                    return;
                };

                wgpu_ctx.surface_config.width = width.max(1);
                wgpu_ctx.surface_config.height = height.max(1);
                wgpu_ctx
                    .surface
                    .configure(&wgpu_ctx.device, &wgpu_ctx.surface_config);

                wgpu_ctx
                    .viewport
                    .update(&wgpu_ctx.queue, Resolution { width, height });

                if let Some(ref mut texture_renderer) = wgpu_ctx.texture_renderer {
                    texture_renderer.resize(
                        &wgpu_ctx.device,
                        wgpu_ctx.surface_config.format,
                        width as f32,
                        height as f32,
                    );
                }

                wgpu_ctx.draw();
            }
            _ => (),
        }
    }
}

#[allow(dead_code)]
pub struct WgpuCtx<'window> {
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    viewport: Viewport,
    texture_renderer: Option<TextureRenderer>,
}

impl<'window> WgpuCtx<'window> {
    pub fn new(window: Arc<Window>) -> WgpuCtx<'window> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Failed to find suitable adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .expect("Failed to request device");

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &surface_config);

        let mut viewport = Viewport::new(&device);
        viewport.update(&queue, Resolution { width, height });

        WgpuCtx {
            surface,
            surface_config,
            adapter,
            viewport,
            device,
            queue,
            texture_renderer: None,
        }
    }

    pub fn draw(&mut self) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let width = 400;
        let height = 300;
        let mut bytes = vec![0u8; width * height * 4];

        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 4;
                let checker = ((x / 40) + (y / 40)) % 2;
                if checker == 0 {
                    bytes[i] = 255;
                    bytes[i + 1] = 0;
                    bytes[i + 2] = 0;
                    bytes[i + 3] = 255;
                } else {
                    bytes[i] = 0;
                    bytes[i + 1] = 0;
                    bytes[i + 2] = 255;
                    bytes[i + 3] = 255;
                }
            }
        }

        let mut buffer = Buffer::new(width as f32, height as f32);
        buffer.set_bytes(&bytes);

        let left = 0.0;
        let top = 0.0;

        let texture = TextureArea {
            left,
            top,
            scale: 1.0,
            bounds: TextureBounds {
                left: 0,
                top: 0,
                right: width as u32,
                bottom: height as u32,
            },
            buffer,
            radius: [0., 0., 0., 0.],
            rotation: 0.,
            skew: [0., 0.],
            depth: 0.,
        };

        let max_icon_size = width.max(height) as u32;
        if self.texture_renderer.is_none() {
            let mut texture_renderer = TextureRenderer::new(
                &self.device,
                self.surface_config.format,
                max_icon_size,
                self.surface_config.width,
                self.surface_config.height,
            );
            texture_renderer.prepare(&self.device, &self.queue, &[texture]);
            texture_renderer.render(&texture_view, &mut encoder, &self.viewport);
            self.texture_renderer = Some(texture_renderer);
        } else {
            let texture_renderer = self.texture_renderer.as_mut().unwrap();
            texture_renderer.prepare(&self.device, &self.queue, &[texture]);
            texture_renderer.render(&texture_view, &mut encoder, &self.viewport);
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}
