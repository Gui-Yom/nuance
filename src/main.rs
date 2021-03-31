use anyhow::Result;
use log::{debug, info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use wgpu::PowerPreference;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use winit::{dpi::LogicalSize, event_loop};

use nuance::{Command, Nuance};

fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .add_filter_ignore_str("naga::front::spv")
            .build(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )?;

    info!("Starting up !");

    let event_loop = EventLoop::with_user_event();

    // Create the window
    let builder = WindowBuilder::new()
        .with_title("Shadertoy")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    let window = builder.build(&event_loop)?;

    // GPU power preference
    let args: Vec<String> = std::env::args().collect();
    let power_preference = if args.contains(&"-H".to_string()) {
        PowerPreference::HighPerformance
    } else {
        PowerPreference::LowPower
    };

    event_loop
        .create_proxy()
        .send_event(Command::Load("shaders/sliders.frag".to_string()))?;

    // Going async !
    let app = futures_executor::block_on(Nuance::init(window, power_preference))?;
    futures_executor::block_on(app.run(event_loop))?;

    Ok(())
}
