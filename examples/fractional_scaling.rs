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

/// Example demonstrating fractional scaling support in moxui.
/// 
/// This example shows how to properly handle fractional scale factors
/// (like 1.25x, 1.5x, 2x) that are common on high-DPI displays.
/// 
/// The approach is:
/// 1. Get the scale factor from the window
/// 2. Update the viewport with physical resolution
/// 3. Apply scale_factor when setting positions/sizes of UI elements
/// 4. Handle ScaleFactorChanged events to redraw content
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
    scale_factor: f32,
}

impl<'window> ApplicationHandler for App<'window> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = Window::default_attributes()
                .with_title("Fractional Scaling Example");
            let window = Arc::new(
                event_loop
                    .create_window(win_attr)
                    .expect("Failed to create window"),
            );
            
            self.scale_factor = window.scale_factor() as f32;
            println!("Window scale factor: {}", self.scale_factor);
            
            self.window = Some(window.clone());
            let wgpu_ctx = WgpuCtx::new(window.clone());
            self.wgpu_ctx = Some(wgpu_ctx);
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
                    wgpu_ctx.draw(self.scale_factor);
                }
            }
            // Handle window resize - update viewport with new physical size
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                let Some(ref mut wgpu_ctx) = self.wgpu_ctx else {
                    return;
                };
                
                println!("Resized to {}x{} (scale: {})", width, height, self.scale_factor);
                
                wgpu_ctx.viewport.update(&wgpu_ctx.queue, Resolution { width, height });
                wgpu_ctx.surface_config.width = width;
                wgpu_ctx.surface_config.height = height;
                wgpu_ctx.surface.configure(&wgpu_ctx.device, &wgpu_ctx.surface_config);
                wgpu_ctx.draw(self.scale_factor);
            }
            // Handle scale factor changes - important for when user moves window
            // between displays with different scale factors
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let Some(ref mut wgpu_ctx) = self.wgpu_ctx else {
                    return;
                };
                
                println!("Scale factor changed to: {}", scale_factor);
                self.scale_factor = scale_factor as f32;
                wgpu_ctx.draw(self.scale_factor);
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

        // Initialize viewport with physical resolution
        let mut viewport = Viewport::new(&device);
        viewport.update(&queue, Resolution { width, height });

        WgpuCtx {
            surface,
            surface_config,
            adapter,
            viewport,
            device,
            queue,
        }
    }

    pub fn draw(&mut self, scale_factor: f32) {
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

        // Create a test texture with some colored squares
        let width = 200;
        let height = 200;
        let mut bytes = vec![0u8; width * height * 4];

        // Create a pattern of colored squares
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                
                // Create a checkerboard pattern
                let square_size = 50;
                let is_red_square = (x / square_size + y / square_size) % 2 == 0;
                
                if is_red_square {
                    bytes[idx] = 255;     // R
                    bytes[idx + 1] = 100; // G
                    bytes[idx + 2] = 100; // B
                } else {
                    bytes[idx] = 100;     // R
                    bytes[idx + 1] = 100; // G
                    bytes[idx + 2] = 255; // B
                }
                bytes[idx + 3] = 255;     // A
            }
        }

        let mut buffer = Buffer::new(width as f32, height as f32);
        buffer.set_bytes(&bytes);

        // Position texture in logical coordinates
        // Apply scale_factor to the scale field to convert logical -> physical
        let logical_left = 50.;
        let logical_top = 50.;
        
        let texture = TextureArea {
            left: logical_left,
            top: logical_top,
            scale: scale_factor,  // Apply scale factor here
            bounds: TextureBounds {
                left: 0,
                top: 0,
                right: width as u32,
                bottom: height as u32,
            },
            buffer,
            radius: [10., 10., 10., 10.], // Rounded corners
            rotation: 0.,
            skew: [0., 0.],
            depth: 0.,
        };

        let mut texture_renderer = TextureRenderer::new(
            &self.device,
            self.surface_config.format,
            width as u32,
            self.surface_config.width,
            self.surface_config.height,
        );
        
        texture_renderer.prepare(&self.device, &self.queue, &[texture]);
        texture_renderer.render(&texture_view, &mut encoder, &self.viewport);

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

