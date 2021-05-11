use anyhow::Result;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use wgpu::PowerPreference;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use nuance::{Command, Nuance};

fn main() -> Result<()> {
    let mut power_preference = PowerPreference::LowPower;
    let mut shader = None;
    for arg in std::env::args() {
        match arg.as_str() {
            "-H" => {
                power_preference = PowerPreference::HighPerformance;
            }
            _ => {
                shader = Some(arg);
            }
        }
    }

    TermLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Error)
            .add_filter_ignore_str("naga::front::spv")
            .add_filter_ignore_str("naga::valid::interface")
            .add_filter_ignore_str("wgpu_core::instance")
            .add_filter_ignore_str("wgpu_core::device")
            .add_filter_ignore_str("wgpu_core::swap_chain")
            .add_filter_ignore_str("wgpu_core::command")
            .add_filter_ignore_str("wgpu_core::hub")
            .add_filter_ignore_str("gfx_backend_vulkan")
            .build(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )?;

    info!("Starting up !");

    let event_loop = EventLoop::with_user_event();

    // Create the window
    let builder = WindowBuilder::new()
        .with_title("Nuance")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    let window = builder.build(&event_loop)?;

    if let Some(shader) = shader {
        event_loop
            .create_proxy()
            .send_event(Command::Load(shader))?;
    }

    // Going async !
    let app = futures_executor::block_on(Nuance::init(window, power_preference))?;
    futures_executor::block_on(app.run(event_loop))?;

    Ok(())
}
