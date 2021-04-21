use std::mem;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use egui::{ClippedMesh, Color32, DragValue, Frame, Style, TextureId};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use log::{debug, info};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use wgpu::PowerPreference;
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::Window;

use extractor::Param;

use crate::renderer::Renderer;
use crate::shader_loader::ShaderLoader;
use crate::types::{Globals, UVec2};
use egui_wgpu_backend::egui::FontDefinitions;

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
    /// Logical size
    ui_width: u32,
}

pub struct Nuance {
    window: Window,
    egui_platform: Platform,

    shader_loader: ShaderLoader,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,

    renderer: Renderer,

    sim_time: Instant,
    settings: Settings,
    globals: Globals,
    curr_shader_file: Option<String>,
    params: Vec<Param>,
}

impl Nuance {
    pub async fn init(window: Window, power_preference: PowerPreference) -> Result<Self> {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor();

        let ui_width = 200.0;
        let mut canvas_size = window_size;
        canvas_size.width -= (ui_width * scale_factor) as u32;

        debug!(
            "window physical size : {:?}, scale factor : {}",
            window_size, scale_factor
        );
        debug!("canvas size : {:?}", canvas_size);

        let renderer = Renderer::new(
            &window,
            power_preference,
            canvas_size.into(),
            mem::size_of::<Globals>() as u32,
        )
        .await?;

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
            sim_time: Instant::now(),
            settings: Settings {
                target_framerate: Duration::from_secs_f32(1.0 / 30.0),
                mouse_wheel_step: 0.1,
                ui_width: ui_width as u32,
            },
            globals: Globals {
                resolution: UVec2::new(canvas_size.width, canvas_size.height),
                mouse: UVec2::zero(),
                mouse_wheel: 0.0,
                ratio: (canvas_size.width) as f32 / canvas_size.height as f32,
                time: 0.0,
                frame: 0,
            },
            curr_shader_file: None,
            params: Vec::new(),
        })
    }

    /// Runs the window, will block the thread until completion
    pub async fn run(mut self, event_loop: EventLoop<Command>) -> Result<()> {
        let mut last_draw_time = Instant::now();
        //let ev_sender = event_loop.create_proxy();
        // To send user events to the event loop
        let proxy = event_loop.create_proxy();

        let app_time = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            // Run this loop indefinitely by default
            *control_flow = ControlFlow::Poll;

            if let Ok(DebouncedEvent::Write(path)) = self.watcher_rx.try_recv() {
                proxy
                    .send_event(Command::Load(path.to_str().unwrap().to_string()))
                    .unwrap();
            }

            // Let egui update with the window events
            self.egui_platform.handle_event(&event);

            match event {
                Event::UserEvent(cmd) => match cmd {
                    Command::Load(path) => {
                        info!("Loading {}", path);
                        let reload_start = Instant::now();
                        let (shader, params) = self.shader_loader.load_shader(&path).unwrap();
                        self.params = if let Some(params) = params {
                            params
                        } else {
                            Vec::new()
                        };
                        self.renderer.new_pipeline_from_shader_source(shader);
                        // Reset the running globals
                        self.globals.frame = 0;
                        self.globals.time = 0.0;
                        self.sim_time = Instant::now();
                        self.curr_shader_file = Some(path);

                        info!(
                            "Loaded and ready ! (took {} ms)",
                            reload_start.elapsed().as_millis()
                        );
                    }
                    Command::Reload => {
                        info!("Reloading !");
                        proxy
                            .send_event(Command::Load(
                                self.curr_shader_file.as_ref().unwrap().to_string(),
                            ))
                            .expect("Can't send event ?");
                    }
                    Command::Watch(path) => {
                        self.watcher
                            .watch(&path, RecursiveMode::NonRecursive)
                            .unwrap();
                    }
                    Command::Unwatch => {
                        self.watcher
                            .unwatch(self.curr_shader_file.as_ref().unwrap())
                            .unwrap();
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
                        self.sim_time = Instant::now();
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
                        let scale_factor = self.window.scale_factor();
                        if position.x > self.settings.ui_width as f64 * scale_factor {
                            self.globals.mouse = UVec2::new(
                                (position.x - self.settings.ui_width as f64 * scale_factor) as u32,
                                position.y as u32,
                            );
                        }
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
                    self.globals.time = self.sim_time.elapsed().as_secs_f32();
                }
                Event::RedrawRequested(_) => {
                    self.egui_platform
                        .update_time(app_time.elapsed().as_secs_f64());
                    let paint_jobs = self.render_gui(&proxy);
                    let window_size = self.window.inner_size();
                    self.renderer
                        .render(
                            ScreenDescriptor {
                                physical_width: window_size.width,
                                physical_height: window_size.height,
                                scale_factor: self.window.scale_factor() as f32,
                            },
                            renderer::GuiData {
                                texture: &self.egui_platform.context().texture(),
                                paint_jobs: &paint_jobs,
                            },
                            &to_glsl(&self.params),
                            bytemuck::bytes_of(&self.globals),
                        )
                        .unwrap();
                    self.globals.frame += 1;
                }
                _ => {}
            }
        });
    }

    fn render_gui(&mut self, proxy: &EventLoopProxy<Command>) -> Vec<ClippedMesh> {
        let window_size = self.window.inner_size();
        let scale_factor = self.window.scale_factor() as f32;
        self.egui_platform.begin_frame();

        let mut framerate = (1.0 / self.settings.target_framerate.as_secs_f32()).round() as u32;

        egui::SidePanel::left("params", self.settings.ui_width as f32).show(
            &self.egui_platform.context(),
            |ui| {
                ui.label(format!(
                    "resolution : {:.0}x{:.0} px",
                    self.globals.resolution.x, self.globals.resolution.y
                ));
                ui.label(format!(
                    "mouse : ({:.0}, {:.0}) px",
                    self.globals.mouse.x, self.globals.mouse.y
                ));
                ui.label(format!("mouse wheel : {:.1}", self.globals.mouse_wheel));
                ui.label(format!("time : {:.3} s", self.globals.time));
                ui.label(format!("frame : {}", self.globals.frame));

                if ui.small_button("Reset").clicked() {
                    proxy.send_event(Command::Restart).unwrap();
                }

                ui.separator();

                ui.label("Settings");

                ui.add(
                    DragValue::new(&mut framerate)
                        .prefix("framerate : ")
                        .clamp_range(4.0..=120.0)
                        .max_decimals(0)
                        .speed(0.1),
                );
                ui.add(
                    DragValue::new(&mut self.settings.mouse_wheel_step)
                        .prefix("mouse wheel inc : ")
                        .clamp_range(-100.0..=100.0)
                        .max_decimals(3)
                        .speed(0.01),
                );

                ui.separator();

                // Shader name
                if let Some(file) = self.curr_shader_file.as_ref() {
                    ui.colored_label(Color32::GREEN, file);
                } else {
                    ui.colored_label(Color32::RED, "No loaded shader");
                }

                ui.label("Params");
                for param in self.params.iter_mut() {
                    ui.add(
                        DragValue::new(&mut param.value)
                            .prefix(format!("{}: ", param.name))
                            .clamp_range(param.min..=param.max)
                            .max_decimals(3)
                            .speed(
                                param.max
                                    / (window_size.width as f32
                                        - self.settings.ui_width as f32 * scale_factor),
                            ),
                    );
                }
            },
        );
        egui::CentralPanel::default().frame(Frame::none()).show(
            &self.egui_platform.context(),
            |ui| {
                ui.image(
                    TextureId::User(0),
                    egui::Vec2::new(
                        (window_size.width as f32 - self.settings.ui_width as f32 * scale_factor)
                            / scale_factor,
                        window_size.height as f32 / scale_factor,
                    ),
                );
            },
        );

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_, paint_commands) = self.egui_platform.end_frame();

        self.settings.target_framerate = Duration::from_secs_f32(1.0 / framerate as f32);

        self.egui_platform.context().tessellate(paint_commands)
    }
}

fn to_glsl<'a>(iter: impl IntoIterator<Item = &'a Param>) -> Vec<u8> {
    // We put our values together
    let mut floats = Vec::new();
    for param in iter {
        floats.push(param.value);
    }
    // We reinterpret our floats to bytes
    // FIXME probably won't work for more complex types
    unsafe {
        let ratio = mem::size_of::<f32>() / mem::size_of::<u8>();

        let length = floats.len() * ratio;
        let capacity = floats.capacity() * ratio;
        let ptr = floats.as_mut_ptr() as *mut u8;

        // Don't run the destructor for vec32
        mem::forget(floats);

        // Construct new Vec
        Vec::from_raw_parts(ptr, length, capacity)
    }
}
