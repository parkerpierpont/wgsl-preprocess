use crate::App;
use runtime::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use runtime::event_loop::ControlFlow;
use runtime::EventHandler;

impl EventHandler for App {
    #[inline]
    fn handle_event(
        &mut self,
        event: &runtime::event::Event<()>,
        _target: &runtime::event_loop::EventLoopWindowTarget<()>,
        control_flow: &mut runtime::event_loop::ControlFlow,
    ) {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == &self.window.id() => {
                if !self.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            self.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so w have to dereference it twice
                            self.resize(**new_inner_size);
                        }
                        WindowEvent::Focused(is_focused) => {
                            if !is_focused {
                                self.set_unfocused();
                            } else {
                                self.set_focused();
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                self.update();
                self.render();
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it
                if self.is_focused {
                    self.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
