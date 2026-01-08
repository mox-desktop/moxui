use moxui::shape_renderer::{ShapeInstance, ShapeRenderer};
use moxui::viewport::{Resolution, Viewport};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::window::{Window, WindowId};

fn create_depth_buffer(
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, view)
}

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

                let width = width.max(1);
                let height = height.max(1);

                wgpu_ctx.surface_config.width = width;
                wgpu_ctx.surface_config.height = height;
                wgpu_ctx
                    .surface
                    .configure(&wgpu_ctx.device, &wgpu_ctx.surface_config);

                wgpu_ctx
                    .viewport
                    .update(&wgpu_ctx.queue, Resolution { width, height });

                // Recreate depth buffer with new size
                let (depth_texture, depth_view) =
                    create_depth_buffer(&wgpu_ctx.device, width, height);
                wgpu_ctx.depth_texture = depth_texture;
                wgpu_ctx.depth_view = depth_view;

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
    shape_renderer: ShapeRenderer,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
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

        let texture_renderer = ShapeRenderer::new(&device, surface_config.format);

        // Create depth buffer
        let (depth_texture, depth_view) = create_depth_buffer(&device, width, height);

        WgpuCtx {
            surface,
            surface_config,
            adapter,
            viewport,
            device,
            queue,
            shape_renderer: texture_renderer,
            depth_texture,
            depth_view,
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

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("standard_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let shapes = vec![
            ShapeInstance {
                rect_pos: [0., 0.],
                rect_size: [400., 300.],
                scale: 1.0,
                rect_color: [1., 1., 0., 1.],
                border_radius: [0., 0., 0., 0.],
                border_size: [1., 1., 1., 1.],
                border_color: [1., 0., 1., 1.],
                depth: 0.,
            },
            ShapeInstance {
                rect_pos: [450., 0.],
                rect_size: [400., 300.],
                scale: 1.0,
                rect_color: [1., 0., 0., 1.],
                border_radius: [50., 50., 50., 50.],
                border_size: [1., 1., 1., 1.],
                border_color: [1., 0., 1., 1.],
                depth: 0.1,
            },
            ShapeInstance {
                rect_pos: [550., 50.],
                rect_size: [100., 100.],
                scale: 1.0,
                rect_color: [0.5, 0., 1., 1.],
                border_radius: [0., 0., 0., 0.],
                border_size: [1., 1., 1., 1.],
                border_color: [1., 0., 1., 1.],
                depth: 0.,
            },
            ShapeInstance {
                rect_pos: [50., 400.],
                rect_size: [500., 500.],
                scale: 1.0,
                rect_color: [0., 1., 1., 1.],
                border_radius: [50., 50., 0., 0.],
                border_size: [1., 1., 1., 1.],
                border_color: [1., 0., 1., 1.],
                depth: 0.,
            },
            ShapeInstance {
                rect_pos: [300., 400.],
                rect_size: [500., 500.],
                scale: 1.0,
                rect_color: [1., 0., 0., 1.],
                border_radius: [50., 50., 0., 0.],
                border_size: [1., 1., 1., 1.],
                border_color: [1., 0., 1., 1.],
                depth: 0.,
            },
        ];

        self.shape_renderer
            .prepare(&self.device, &self.queue, &shapes);
        self.shape_renderer.render(&mut render_pass, &self.viewport);

        drop(render_pass);

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}
