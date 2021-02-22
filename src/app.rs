use std::mem;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::renderer::Renderer;
use crate::shader_loader::ShaderLoader;
use crate::types::{Globals, UVec2};

#[derive(Debug)]
pub enum Command {
    Load(String),
    Reload,
    Watch(String),
    Unwatch,
    TargetFps(i16),
    Restart,
    Exit,
}

pub struct App {
    window: Window,

    shader_loader: ShaderLoader,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,

    renderer: Renderer,

    started: Instant,
    target_framerate: Duration,
    globals: Globals,
}

impl App {
    pub async fn init(window: Window) -> Result<Self> {
        let window_size = window.inner_size();
        let renderer = Renderer::new(&window, mem::size_of::<Globals>() as u32).await?;
        let (tx, rx) = std::sync::mpsc::channel();
        Ok(Self {
            window,
            shader_loader: ShaderLoader::new(),
            watcher: watcher(tx, Duration::from_millis(200))?,
            watcher_rx: rx,
            renderer,
            started: Instant::now(),
            target_framerate: Duration::from_secs_f32(1.0 / 30.0),
            globals: Globals {
                resolution: UVec2::new(window_size.width, window_size.height),
                mouse: UVec2::zero(),
                ratio: window_size.width as f32 / window_size.height as f32,
                time: 0.0,
                frame: 0,
            },
        })
    }

    /// Runs the window, will block the thread until completion
    pub async fn run(mut self, event_loop: EventLoop<Command>) -> Result<()> {
        let mut last_draw_time = Instant::now();
        //let ev_sender = event_loop.create_proxy();
        let mut curr_shader_file = None;
        // To send user events to the event loop
        let proxy = event_loop.create_proxy();

        event_loop.run(move |event, _, control_flow| {
            // Run this loop indefinitely
            *control_flow = ControlFlow::Poll;

            match self.watcher_rx.try_recv() {
                Ok(DebouncedEvent::Write(path)) => {
                    proxy
                        .send_event(Command::Load(path.to_str().unwrap().to_string()))
                        .unwrap();
                }
                _ => {}
            }

            match event {
                Event::UserEvent(cmd) => match cmd {
                    Command::Load(path) => {
                        info!("Reloading !");
                        let reload_start = Instant::now();
                        self.renderer.new_pipeline_from_shader_source(
                            self.shader_loader.load_shader(&path).unwrap(),
                        );
                        // Reset the running globals
                        self.globals.frame = 0;
                        self.globals.time = 0.0;
                        self.started = Instant::now();
                        curr_shader_file = Some(path);

                        info!(
                            "Reloaded ! (took {} ms)",
                            reload_start.elapsed().as_millis()
                        );
                    }
                    Command::Reload => {
                        proxy.send_event(Command::Load(
                            curr_shader_file.as_ref().unwrap().to_string(),
                        ));
                    }
                    Command::Watch(path) => {
                        curr_shader_file = Some(path);
                        self.watcher
                            .watch(
                                curr_shader_file.as_ref().unwrap(),
                                RecursiveMode::NonRecursive,
                            )
                            .unwrap();
                    }
                    Command::Unwatch => {
                        self.watcher
                            .unwatch(curr_shader_file.as_ref().unwrap())
                            .unwrap();
                        curr_shader_file = None;
                    }
                    Command::TargetFps(new_fps) => {
                        self.target_framerate = Duration::from_secs_f32(1.0 / new_fps as f32)
                    }
                    Command::Restart => {
                        info!("Restarting !");
                        // Reset the running globals
                        self.globals.frame = 0;
                        self.globals.time = 0.0;
                        self.started = Instant::now();
                    }
                    Command::Exit => {
                        *control_flow = ControlFlow::Exit;
                    }
                },
                Event::WindowEvent {
                    event:
                        WindowEvent::CursorMoved {
                            device_id: _,
                            position,
                            ..
                        },
                    ..
                } => {
                    let size = self.window.inner_size();
                    self.globals.mouse = UVec2::new(
                        position.x.clamp(0.0, size.width as f64) as u32,
                        position.y.clamp(0.0, size.height as f64) as u32,
                    );
                }
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
                }
                Event::RedrawRequested(_) => {
                    self.renderer
                        .render(bytemuck::bytes_of(&self.globals))
                        .unwrap();
                    self.globals.frame += 1;
                }

                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
        Ok(())
    }
}
