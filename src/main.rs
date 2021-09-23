use anyhow::Result;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
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

    TermLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Error)
            .add_filter_ignore_str("wgpu_core::device")
            .add_filter_ignore_str("wgpu_core::hub")
            .add_filter_ignore_str("wgpu_core::instance")
            .add_filter_ignore_str("wgpu_core::present")
            .add_filter_ignore_str("wgpu_hal::vulkan")
            .add_filter_ignore_str("wgpu_hal::dx12")
            .add_filter_ignore_str("naga::front")
            .add_filter_ignore_str("naga::valid")
            .build(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )?;

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
