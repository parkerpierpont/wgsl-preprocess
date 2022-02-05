mod event_handler;
mod program;
mod timer;
use timer::*;

pub use program::App;
use runtime::Application;

fn main() {
    Application::run(App::new);
}
