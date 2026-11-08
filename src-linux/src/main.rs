use std::env;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Debug)]
struct Config {
    title: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            title: "Discord Quest Completer".to_string(),
        }
    }
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().collect();
    let mut config = Config::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--title" => {
                if i + 1 < args.len() {
                    config.title = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    config
}

struct App {
    title: String,
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_visible(true)
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize::new(400.0, 200.0));

        self.window = Some(
            event_loop
                .create_window(attributes)
                .expect("Failed to create window"),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
    }
}

fn main() {
    let config = parse_args();
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        title: config.title,
        window: None,
    };

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
