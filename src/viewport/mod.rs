/// Viewport resolution.
///
/// Represents the physical dimensions of the viewport in pixels.
///
/// # Example
///
/// ```ignore
/// use moxui::viewport::Resolution;
///
/// let size = window.inner_size();
/// let resolution = Resolution {
///     width: size.width,
///     height: size.height,
/// };
/// ```
#[derive(PartialEq, Eq, Clone)]
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    resolution: [u32; 2],
    _pad: [u32; 2],
}

/// Viewport manager.
///
/// The viewport manages the screen resolution and provides it to shaders
/// for coordinate transformation.
///
/// # Example
///
/// ```ignore
/// use moxui::viewport::{Viewport, Resolution};
///
/// let mut viewport = Viewport::new(&device);
/// viewport.update(&queue, Resolution {
///     width: 1920,
///     height: 1080,
/// });
/// ```
pub struct Viewport {
    params: Params,
    buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl Viewport {
    /// Creates a new viewport with default settings.
    ///
    /// The viewport is initialized with zero dimensions.
    /// Call [`update`](Self::update) to set the actual resolution.
    pub fn new(device: &wgpu::Device) -> Self {
        let params = Params {
            resolution: [0, 0],
            _pad: [0, 0],
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("texture renderer params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
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
                label: Some("uniform_bind_group_layout"),
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniforms_bind_group"),
        });

        Self {
            params,
            buffer,
            bind_group,
        }
    }

    /// Updates the viewport resolution.
    ///
    /// Call this method whenever the window is resized.
    ///
    /// # Example
    ///
    /// ```ignore
    /// WindowEvent::Resized(PhysicalSize { width, height }) => {
    ///     viewport.update(&queue, Resolution { width, height });
    /// }
    /// ```
    pub fn update(&mut self, queue: &wgpu::Queue, resolution: Resolution) {
        if self.params.resolution != [resolution.width, resolution.height] {
            self.params.resolution = [resolution.width, resolution.height];
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.params]));
        }
    }

    /// Returns the current viewport resolution.
    pub fn resolution(&self) -> Resolution {
        Resolution {
            width: self.params.resolution[0],
            height: self.params.resolution[1],
        }
    }
}
