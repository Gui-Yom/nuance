use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod app;
mod shader_loader;

fn main() {
    // Setup the logger
    pretty_env_logger::init();

    // Create the window
    let event_loop = EventLoop::new();
    let builder = WindowBuilder::new()
        .with_title("Shadertoy")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    let window = builder.build(&event_loop).expect("Can't create window !");

    // Going async !
    futures_executor::block_on(app::run(window, event_loop));
}
