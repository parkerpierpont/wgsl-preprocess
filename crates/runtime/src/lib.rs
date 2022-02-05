mod setup;
mod shader;
pub struct Application;
pub use winit::*;

pub trait EventHandler {
    fn handle_event(
        &mut self,
        event: &winit::event::Event<()>,
        target: &winit::event_loop::EventLoopWindowTarget<()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    );
}

impl Application {
    pub fn run<T: EventHandler + 'static>(
        app_constructor: impl FnOnce(
            winit::window::Window,
            wgpu::Device,
            wgpu::Queue,
            wgpu::Adapter,
            wgpu::Surface,
            wgpu::SurfaceConfiguration,
        ) -> T,
    ) {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
                1068.0, 800.0,
            )))
            .with_title("Winit Playground")
            .with_visible(true)
            .build(&event_loop)
            .unwrap();

        let setup::GlobalGPU {
            device,
            queue,
            adapter,
            surface,
            surface_config,
        } = setup::GlobalGPU::new(&window);

        let mut application =
            app_constructor(window, device, queue, adapter, surface, surface_config);
        // app.setup(window, device, queue, surface, surface_config);

        event_loop.run(move |x, y, z| {
            application.handle_event(&x, y, z);
        })
    }
}
