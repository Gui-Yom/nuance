use anyhow::Result;
use env_logger::{Target, WriteStyle};
use log::{info, LevelFilter};
use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use crate::app::Nuance;

mod app;

fn main() -> Result<()> {
    let mut pref_hp = false;
    for arg in std::env::args() {
        if arg.as_str() == "-H" {
            pref_hp = true;
        }
    }

    puffin::set_scopes_on(true);

    env_logger::builder()
        .target(Target::Stdout)
        .format_timestamp(None)
        .write_style(WriteStyle::Always)
        .filter_module("wgpu_core::instance", LevelFilter::Warn)
        .filter_module("wgpu_core::device", LevelFilter::Warn)
        .filter_module("wgpu_core::present", LevelFilter::Warn)
        .filter_module("wgpu_core::hub", LevelFilter::Warn)
        .filter_module("wgpu_hal::vulkan::instance", LevelFilter::Off)
        .filter_module("wgpu_hal::vulkan::adapter", LevelFilter::Warn)
        .filter_module("wgpu_hal::dx12::instance", LevelFilter::Error)
        .filter_module("naga::front", LevelFilter::Warn)
        .filter_module("naga::valid", LevelFilter::Warn)
        .init();

    info!("Starting up !");

    let event_loop = EventLoop::new();

    // Create the window
    let builder = WindowBuilder::new()
        .with_title("Nuance")
        .with_inner_size(LogicalSize::new(1280, 720))
        .with_resizable(true)
        .with_visible(true);
    let window = builder.build(&event_loop)?;

    let mut app = futures_executor::block_on(Nuance::init(window, pref_hp))?;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => {
            app.handle_event(event, control_flow);
        }
        Event::MainEventsCleared => {
            app.update(control_flow);
        }
        Event::RedrawRequested(_) => {
            app.draw();
        }
        _ => {}
    });

    //Ok(())
}
