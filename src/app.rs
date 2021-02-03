use core::mem;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use hotwatch::notify::DebouncedEvent;
use hotwatch::Hotwatch;
use log::{debug, error, info, warn};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::renderer::Renderer;
use crate::shader_loader::ShaderLoader;

/// The globals we pass to the fragment shader
/// aligned to 32bit words
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    /// Draw area width
    width: u32,
    /// Draw area height
    height: u32,
    /// Draw area width/height ratio
    ratio: f32,
    /// Current running time in sec
    time: f32,
    /// Time since the last frame in sec
    time_delta: f32,
    /// Number of frame
    frame: u32,
}

#[derive(Debug)]
pub enum Command {
    Load(String),
    WatchEvent(DebouncedEvent),
    Close,
}

pub struct App {
    window: Window,

    shader_loader: ShaderLoader,
    watcher: Option<Hotwatch>,

    renderer: Renderer,

    started: Instant,
    target_framerate: Duration,
    globals: Globals,
}

impl App {
    pub async fn init(window: Window) -> Result<Self> {
        let window_size = window.inner_size();
        let renderer = Renderer::new(&window, mem::size_of::<Globals>() as u32).await?;
        Ok(Self {
            window,
            shader_loader: ShaderLoader::new(),
            watcher: None,
            renderer,
            started: Instant::now(),
            target_framerate: Duration::from_secs_f32(1.0 / 30.0),
            globals: Globals {
                width: window_size.width,
                height: window_size.height,
                ratio: window_size.width as f32 / window_size.height as f32,
                time: 0.0,
                time_delta: 0.0,
                frame: 0,
            },
        })
    }

    /// Runs the window, will block the thread until completion
    pub async fn run(mut self, event_loop: EventLoop<Command>) -> Result<()> {
        let mut last_draw_time = Instant::now();

        /*
        let event_loop_proxy = self.event_loop.create_proxy();
        // This value was found by fiddling a bit, the shorter, the more dangerous it is,
        // because we could receive some events twice.
        self.watcher = Some(Hotwatch::new_with_custom_delay(Duration::from_millis(400))?);
        self.watcher.watch(shader_file, move |e| {
            event_loop_proxy.send_event(Command::WatchEvent(e)).unwrap()
        })?;

         */

        event_loop.run(move |event, _, control_flow| {
            // Run this loop indefinitely
            *control_flow = ControlFlow::Poll;

            match event {
                Event::UserEvent(cmd) => match cmd {
                    Command::Load(_) => {}
                    Command::Close => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Command::WatchEvent(DebouncedEvent::Write(path)) => {
                        info!("Reloading !");
                        let reload_start = Instant::now();
                        self.renderer.new_pipeline_from_shader_source(
                            self.shader_loader.load_shader(path).unwrap(),
                        );
                        // Reset the running globals
                        self.globals.frame = 0;
                        self.globals.time = 0.0;
                        self.globals.time_delta = 0.0;

                        info!(
                            "Reloaded ! (took {} ms)",
                            reload_start.elapsed().as_millis()
                        );
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    let frame_time = last_draw_time.elapsed();
                    if frame_time >= self.target_framerate {
                        self.window.request_redraw();
                        last_draw_time = Instant::now();
                    } else {
                        // Sleep til next frame
                        *control_flow = ControlFlow::WaitUntil(
                            Instant::now() + self.target_framerate - frame_time,
                        );
                    }
                    self.globals.time = self.started.elapsed().as_secs_f32();
                    self.globals.time_delta = frame_time.as_secs_f32();
                }
                Event::RedrawRequested(_) => {
                    self.renderer
                        .render(bytemuck::cast_slice(&[self.globals]))
                        .unwrap();
                    self.globals.frame += 1;
                }

                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    warn!("Close the app with Esc or \"exit\" in the terminal.");
                }
                _ => {}
            }
        });
        Ok(())
    }
}
