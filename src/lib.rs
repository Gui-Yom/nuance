use std::mem;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use egui::{ClippedMesh, DragValue, FontDefinitions, Frame, Style, TextureId};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use log::{debug, info};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use wgpu::PowerPreference;
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use extractor::Param;

use crate::renderer::Renderer;
use crate::shader_loader::ShaderLoader;
use crate::types::{Globals, UVec2};

pub mod extractor;
pub mod renderer;
pub mod shader_loader;
pub mod types;

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

struct Settings {
    target_framerate: Duration,
    mouse_wheel_step: f32,
}

pub struct Nuance {
    window: Window,
    egui_platform: Platform,

    shader_loader: ShaderLoader,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,

    renderer: Renderer,

    started: Instant,
    settings: Settings,
    globals: Globals,
    params: Vec<Param>,
}

impl Nuance {
    pub async fn init(window: Window, power_preference: PowerPreference) -> Result<Self> {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor();
        debug!(
            "window physical size : {:?}, scale factor : {}",
            window_size, scale_factor
        );
        let renderer =
            Renderer::new(&window, power_preference, mem::size_of::<Globals>() as u32).await?;
        let (tx, rx) = std::sync::mpsc::channel();
        Ok(Self {
            window,
            egui_platform: Platform::new(PlatformDescriptor {
                physical_width: window_size.width,
                physical_height: window_size.height,
                scale_factor,
                font_definitions: FontDefinitions::default(),
                style: Style::default(),
            }),
            shader_loader: ShaderLoader::new(),
            watcher: watcher(tx, Duration::from_millis(200))?,
            watcher_rx: rx,
            renderer,
            started: Instant::now(),
            settings: Settings {
                target_framerate: Duration::from_secs_f32(1.0 / 30.0),
                mouse_wheel_step: 0.1,
            },
            globals: Globals {
                resolution: UVec2::new(window_size.width - 200, window_size.height),
                mouse: UVec2::zero(),
                mouse_wheel: 0.0,
                ratio: (window_size.width - 200) as f32 / window_size.height as f32,
                time: 0.0,
                frame: 0,
            },
            params: Vec::new(),
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
            // Run this loop indefinitely by default
            *control_flow = ControlFlow::Poll;

            if let Ok(DebouncedEvent::Write(path)) = self.watcher_rx.try_recv() {
                proxy
                    .send_event(Command::Load(path.to_str().unwrap().to_string()))
                    .unwrap();
            }

            self.egui_platform.handle_event(&event);

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
                        proxy
                            .send_event(Command::Load(
                                curr_shader_file.as_ref().unwrap().to_string(),
                            ))
                            .expect("Can't send event ?");
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
                        self.settings.target_framerate =
                            Duration::from_secs_f32(1.0 / new_fps as f32)
                    }
                    Command::Restart => {
                        info!("Restarting !");
                        // Reset the running globals
                        self.globals.frame = 0;
                        self.globals.time = 0.0;
                        self.globals.mouse_wheel = 0.0;
                        self.started = Instant::now();
                    }
                    Command::Exit => {
                        *control_flow = ControlFlow::Exit;
                    }
                },
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CursorMoved {
                        device_id: _device_id,
                        position,
                        ..
                    } => {
                        let size = self.window.inner_size();
                        self.globals.mouse = UVec2::new(
                            position.x.clamp(0.0, size.width as f64) as u32,
                            position.y.clamp(0.0, size.height as f64) as u32,
                        );
                    }
                    WindowEvent::MouseWheel {
                        device_id: _device_id,
                        delta,
                        ..
                    } => match delta {
                        MouseScrollDelta::LineDelta(_, value) => {
                            self.globals.mouse_wheel += value * self.settings.mouse_wheel_step;
                        }
                        MouseScrollDelta::PixelDelta(pos) => {
                            info!("{:?}", pos);
                        }
                    },
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    let frame_time = last_draw_time.elapsed();
                    if frame_time >= self.settings.target_framerate {
                        self.window.request_redraw();
                        last_draw_time = Instant::now();
                    } else {
                        // Sleep til next frame
                        *control_flow = ControlFlow::WaitUntil(
                            Instant::now() + self.settings.target_framerate - frame_time,
                        );
                    }
                    self.globals.time = self.started.elapsed().as_secs_f32();
                }
                Event::RedrawRequested(_) => {
                    self.egui_platform
                        .update_time(self.started.elapsed().as_secs_f64());
                    let paint_jobs = self.render_gui();
                    let window_size = self.window.inner_size();
                    self.renderer
                        .render(
                            ScreenDescriptor {
                                physical_width: window_size.width,
                                physical_height: window_size.height,
                                scale_factor: self.window.scale_factor() as f32,
                            },
                            renderer::GUIData {
                                texture: &self.egui_platform.context().texture(),
                                paint_jobs: &paint_jobs,
                            },
                            bytemuck::bytes_of(&self.globals),
                        )
                        .unwrap();
                    self.globals.frame += 1;
                }
                _ => {}
            }
        });
    }

    fn render_gui(&mut self) -> Vec<ClippedMesh> {
        let window_size = self.window.inner_size();
        self.egui_platform.begin_frame();

        egui::SidePanel::left("params", 200.0).show(&self.egui_platform.context(), |ui| {
            if ui.button("Hello !").clicked() {
                println!("Clicked !");
            }
            ui.separator();

            for param in self.params.iter_mut() {
                ui.add(
                    DragValue::f32(&mut param.value)
                        .prefix(format!("{}: ", param.name))
                        .clamp_range(param.min..=param.max)
                        .max_decimals(3)
                        .speed(param.max / (window_size.width - 200) as f32),
                );
            }
        });
        egui::CentralPanel::default().frame(Frame::none()).show(
            &self.egui_platform.context(),
            |ui| {
                ui.image(
                    TextureId::User(0),
                    egui::vec2(
                        (window_size.width - 200) as f32 / 1.25,
                        window_size.height as f32 / 1.25,
                    ),
                );
            },
        );

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_, paint_commands) = self.egui_platform.end_frame();
        self.egui_platform.context().tessellate(paint_commands)
    }
}
