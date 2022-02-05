pub struct GlobalGPU {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,
}

impl GlobalGPU {
    pub fn new(window: &winit::window::Window) -> Self {
        let inner_size = window.inner_size();
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::Backends::all());
            let surface = unsafe { instance.create_surface(window) };

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..wgpu::RequestAdapterOptions::default()
                })
                .await
                .unwrap();

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Main window"),
                        ..wgpu::DeviceDescriptor::default()
                    },
                    None,
                )
                .await
                .unwrap();

            // This specifies how we want the surface to create it's underlying
            // `SurfaceTexture`s.
            let surface_config = wgpu::SurfaceConfiguration {
                /// How the `SurfaceTexture`s will be used. (This specifies
                /// that it will be used on a screen).
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                /// How the surface textures will be stored on the GPU.
                /// Different displays use different formats. We use
                /// surface.get_preferred_format(&adapter) to figure
                /// our the best format to use depending on the display
                /// used.
                format: surface.get_preferred_format(&adapter).unwrap(),
                /// The width and height in pixels of the SurfaceTexture.
                /// This should usually be the width and height of the window.
                ///
                /// If this is set to 0, the app will crash.
                width: inner_size.width,
                height: inner_size.height,
                /// This tells wgpu how to sync the surface with the display. This
                /// will cap the display rate at the display's framerate
                /// (essentially vsync).
                ///
                /// It's by far the best mode to use on Mobile. There are other
                /// options you can use as well.
                present_mode: wgpu::PresentMode::Fifo,
            };

            surface.configure(&device, &surface_config);

            Self {
                device,
                adapter,
                queue,
                surface,
                surface_config,
            }
        })
    }
}
