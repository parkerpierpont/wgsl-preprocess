use std::{
    borrow::Cow,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use runtime::event::WindowEvent;

use crate::Timer;

pub struct App {
    pub window: runtime::window::Window,
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub adapter: Rc<wgpu::Adapter>,
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub programs: Vec<Program>,
    pub last_time: Instant,
    pub frame: usize,
    pub is_focused: bool,
    pub timer: Timer,
}

impl App {
    pub fn new(
        window: runtime::window::Window,
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter: wgpu::Adapter,
        surface: wgpu::Surface,
        surface_config: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            window,
            device: Rc::new(device),
            queue: Rc::new(queue),
            adapter: Rc::new(adapter),
            surface,
            surface_config,
            programs: vec![],
            last_time: Instant::now(),
            frame: 0,
            is_focused: true,
            timer: Timer::new(),
        }
    }

    pub fn load_programs(&mut self) {
        let program_context = ProgramContext {
            adapter: Rc::downgrade(&self.adapter),
            device: Rc::downgrade(&self.device),
            format: self.surface_config.format,
            queue: Rc::downgrade(&self.queue),
        };

        self.programs.push(Program::new(program_context));
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    pub fn resize(&mut self, new_size: runtime::dpi::PhysicalSize<u32>) {
        // Zero will cause wgpu to crash
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn set_unfocused(&mut self) {
        self.is_focused = false;
        self.timer.pause();
    }

    pub fn set_focused(&mut self) {
        self.is_focused = true;
    }

    pub fn update(&mut self) {
        for program in &mut self.programs {
            program.update();
        }

        let duration = Instant::now() - self.last_time;
        if duration > Duration::from_millis(1000 / 60) {
            self.last_time = Instant::now();
            self.frame = self.frame.wrapping_add(1);
        }
    }

    #[inline]
    pub fn render(&mut self) {
        if self.programs.len() == 0 {
            self.load_programs();
        }

        let frame = self.surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            let mut bundles = vec![];
            for program in &self.programs {
                bundles.push(program.render_bundle());
            }

            render_pass.execute_bundles(bundles.into_iter());
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

pub struct ProgramContext {
    pub device: Weak<wgpu::Device>,
    pub adapter: Weak<wgpu::Adapter>,
    pub queue: Weak<wgpu::Queue>,
    pub format: wgpu::TextureFormat,
}

impl ProgramContext {
    pub fn device(&self) -> Rc<wgpu::Device> {
        self.device.upgrade().unwrap()
    }

    pub fn adapter(&self) -> Rc<wgpu::Adapter> {
        self.adapter.upgrade().unwrap()
    }

    pub fn queue(&self) -> Rc<wgpu::Queue> {
        self.queue.upgrade().unwrap()
    }
}

pub struct Program {
    ctx: ProgramContext,
    render_pipeline: wgpu::RenderPipeline,
    render_bundle: Option<wgpu::RenderBundle>,
}

impl Program {
    pub fn new(ctx: ProgramContext) -> Self {
        // Load the shaders from disk
        let shader = ctx
            .device()
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            ctx.device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[ctx.format.into()],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        let mut data = Self {
            ctx,
            render_pipeline,
            render_bundle: None,
        };

        data.update();
        data
    }

    pub fn update(&mut self) {
        let device = self.ctx.device();
        let mut render_bundle_encoder =
            device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: None,
                color_formats: &[self.ctx.format],
                depth_stencil: None,
                multiview: None,
                sample_count: 1,
                ..Default::default()
            });

        render_bundle_encoder.set_pipeline(&self.render_pipeline);
        render_bundle_encoder.draw(0..3, 0..1);

        self.render_bundle =
            Some(render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor { label: None }));
    }

    pub fn render_bundle(&self) -> &wgpu::RenderBundle {
        self.render_bundle.as_ref().unwrap()
    }
}
