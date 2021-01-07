use log::{debug, info};
use wgpu::{BackendBit, Instance};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

mod app;
mod shader_loader;

fn create_window() -> (Window, EventLoop<()>) {
    let event_loop = EventLoop::new();
    let builder = WindowBuilder::new()
        .with_title("Shadertoy")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    (
        builder.build(&event_loop).expect("Can't create window !"),
        event_loop,
    )
}

fn main() {
    pretty_env_logger::init();
    let (window, event_loop) = create_window();

    let instance = Instance::new(BackendBit::PRIMARY);
    debug!("Found adapters :");
    instance
        .enumerate_adapters(BackendBit::PRIMARY)
        .for_each(|it| {
            debug!(
                " - {}: {:?} ({:?})",
                it.get_info().name,
                it.get_info().device_type,
                it.get_info().backend
            );
        });

    // Going async !
    futures::executor::block_on(app::run(&window, event_loop, &instance));
}
