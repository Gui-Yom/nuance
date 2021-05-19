use std::mem;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use crevice::std430::AsStd430;
use crevice::std430::Std430;
use egui::{FontDefinitions, Style};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use image::ImageFormat;
use log::{debug, error, info};
use mint::Vector2;
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use wgpu::PowerPreference;
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::gui::Gui;
use crate::renderer::Renderer;
use crate::shader::{Shader, Slider};
use crate::shader_loader::ShaderLoader;

mod gui;
pub mod preprocessor;
pub mod renderer;
pub mod shader;
pub mod shader_loader;

#[derive(Debug)]
pub enum Command {
    Load(PathBuf),
    Reload,
    Watch,
    Unwatch,
    Restart,
    Export,
    Exit,
}

/// The globals we pass to the fragment shader
#[derive(AsStd430)]
pub struct Globals {
    /// Window resolution
    pub resolution: Vector2<u32>,
    /// Mouse pos
    pub mouse: Vector2<u32>,
    /// Mouse wheel
    pub mouse_wheel: f32,
    /// Draw area width/height ratio
    pub ratio: f32,
    /// Current running time in sec
    pub time: f32,
    /// Number of frame
    pub frame: u32,
}

impl Globals {
    pub fn reset(&mut self) {
        self.frame = 0;
        self.time = 0.0;
        self.mouse_wheel = 0.0;
    }
}

pub struct Settings {
    pub target_framerate: Duration,
    pub mouse_wheel_step: f32,
}

pub struct ExportData {
    pub export_prompt: bool,
    pub size: Vector2<u32>,
    pub format: ImageFormat,
    pub path: PathBuf,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            export_prompt: false,
            size: Vector2::from([2048, 2048]),
            format: ImageFormat::Png,
            path: PathBuf::from_str("./render.png").unwrap(),
        }
    }
}

pub struct Nuance {
    /// The main window
    window: Window,
    gui: Gui,
    /// App settings
    pub settings: Settings,

    /// The current loaded shader
    pub shader: Option<Shader>,
    /// Shader compiler and transpiler
    shader_loader: ShaderLoader,
    watcher: RecommendedWatcher,
    /// Receiver for watcher events
    watcher_rx: Receiver<DebouncedEvent>,

    renderer: Renderer,

    /// The instant at which the simulation started
    sim_time: Instant,
    pub watching: bool,
    /// Parameters passed to shaders
    pub globals: Globals,

    pub export_data: ExportData,
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
            Globals::std430_size_static() as u32,
        )
        .await?;

        let (tx, rx) = std::sync::mpsc::channel();

        Ok(Self {
            window,
            gui: Gui::new(
                Platform::new(PlatformDescriptor {
                    physical_width: window_size.width,
                    physical_height: window_size.height,
                    scale_factor,
                    font_definitions: FontDefinitions::default(),
                    style: Style::default(),
                }),
                ui_width as u32,
            ),
            settings: Settings {
                target_framerate: Duration::from_secs_f32(1.0 / 30.0),
                mouse_wheel_step: 0.1,
            },
            shader: None,
            shader_loader: ShaderLoader::new(),
            watcher: watcher(tx, Duration::from_millis(200))?,
            watcher_rx: rx,
            renderer,
            sim_time: Instant::now(),
            watching: false,
            globals: Globals {
                resolution: Vector2::from([canvas_size.width, canvas_size.height]),
                mouse: Vector2::from([0, 0]),
                mouse_wheel: 0.0,
                ratio: (canvas_size.width) as f32 / canvas_size.height as f32,
                time: 0.0,
                frame: 0,
            },
            export_data: Default::default(),
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

            if let Ok(DebouncedEvent::Write(_)) = self.watcher_rx.try_recv() {
                proxy.send_event(Command::Reload).unwrap();
            }

            // Let egui update with the window events
            self.gui.handle_event(&event);

            match event {
                Event::UserEvent(cmd) => match cmd {
                    Command::Load(path) => {
                        proxy.send_event(Command::Unwatch).unwrap();
                        self.load(&path);
                    }
                    Command::Reload => {
                        info!("Reloading !");
                        self.load(self.shader.as_ref().unwrap().main.clone());
                    }
                    Command::Watch => {
                        if let Some(path) = self.shader.as_ref().map(|it| &it.main) {
                            self.watcher
                                .watch(path, RecursiveMode::NonRecursive)
                                .unwrap();
                        }
                    }
                    Command::Unwatch => {
                        if self.watching {
                            if let Some(shader) = self.shader.as_ref() {
                                self.watcher
                                    .unwatch(&shader.main)
                                    .expect("Unexpected state");
                            }
                        }
                    }
                    Command::Restart => {
                        info!("Restarting !");
                        // Reset the running globals
                        self.globals.reset();
                        self.sim_time = Instant::now();
                    }
                    Command::Export => {}
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
                        if position.x > self.gui.ui_width as f64 * scale_factor {
                            self.globals.mouse = Vector2::from([
                                (position.x - self.gui.ui_width as f64 * scale_factor) as u32,
                                position.y as u32,
                            ]);
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
                    self.gui.update_time(app_time.elapsed().as_secs_f64());
                    let window_size = self.window.inner_size();
                    let screen_desc = ScreenDescriptor {
                        physical_width: window_size.width,
                        physical_height: window_size.height,
                        scale_factor: self.window.scale_factor() as f32,
                    };
                    let paint_jobs = Gui::render(&proxy, &screen_desc, &mut self);
                    self.renderer
                        .render(
                            &screen_desc,
                            (&self.gui.texture(), &paint_jobs),
                            &self
                                .shader
                                .as_ref()
                                .map(|it| it.metadata.as_ref().map(|it| to_glsl(&it.sliders)))
                                .unwrap_or_default()
                                .unwrap_or_default(),
                            self.globals.as_std430().as_bytes(),
                        )
                        .unwrap();
                    self.globals.frame += 1;
                }
                _ => {}
            }
        });
    }

    fn load<P: AsRef<Path>>(&mut self, path: P) {
        info!("Loading {}", path.as_ref().to_str().unwrap());
        let reload_start = Instant::now();

        match self.shader_loader.load_shader(&path) {
            Ok((shader, source)) => {
                let buffer_size = if let Some(metadata) = shader.metadata.as_ref() {
                    metadata.buffer_size()
                } else {
                    0
                };

                self.renderer
                    .set_shader(source, Globals::std430_size_static() as u32, buffer_size);

                self.shader = Some(shader);
                // Reset the running globals
                self.globals.reset();
                self.sim_time = Instant::now();

                info!(
                    "Loaded and ready ! (took {} ms)",
                    reload_start.elapsed().as_millis()
                );
            }
            Err(e) => {
                error!("{}", e);
                error!("Can't load {}", path.as_ref().to_str().unwrap());
            }
        }
    }
}

fn to_glsl<'a>(iter: impl IntoIterator<Item = &'a Slider>) -> Vec<u8> {
    // We put our values together
    let mut floats = Vec::new();
    for param in iter {
        match param {
            Slider::Float { value, .. } => {
                floats.push(*value);
            }
            Slider::Vec3 { value, .. } => {
                floats.push(value.x);
                floats.push(value.y);
                floats.push(value.z);
                floats.push(0.0);
            }
            Slider::Color { value, .. } => {
                floats.push(value.x);
                floats.push(value.y);
                floats.push(value.z);
                floats.push(0.0);
            }
            _ => {}
        }
    }
    // We reinterpret our floats to bytes
    // FIXME CRITICAL, probably won't work for more complex types
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
