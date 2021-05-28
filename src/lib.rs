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
use image::{ImageBuffer, ImageFormat, Rgba};
use log::{debug, error, info};
use mint::Vector2;
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use rfd::FileDialog;
use wgpu::PowerPreference;
use winit::event::{Event, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::gui::Gui;
use crate::renderer::Renderer;
use crate::shader::Shader;
use crate::shader_loader::ShaderLoader;

mod gui;
pub mod preprocessor;
pub mod renderer;
pub mod shader;
pub mod shader_loader;

#[derive(Debug)]
pub enum Command {
    /// Open a pick file dialog and load a shader
    Load,
    /// Reload the shader from disk
    Reload,
    /// Watch the shader for fs changes
    Watch,
    /// Unwatch the shader
    Unwatch,
    /// Reset the globals to their default
    ResetGlobals,
    /// Reset the shader params to their default
    ResetParams,
    /// Export a render of the current shader
    ExportImage,
    /// Terminate the application
    Exit,
}

/// The globals we pass to the fragment shader
#[derive(AsStd430, Clone)]
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
    //pub valid_format: bool,
    pub path: PathBuf,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
            export_prompt: false,
            size: Vector2::from([2048, 2048]),
            format: ImageFormat::Png,
            path: PathBuf::from_str("render.png").unwrap(),
        }
    }
}

pub struct Nuance {
    /// The main window
    window: Window,
    gui: Gui,
    /// App settings
    settings: Settings,

    /// The current loaded shader
    shader: Option<Shader>,
    /// Shader compiler and transpiler
    shader_loader: ShaderLoader,
    watcher: RecommendedWatcher,
    /// Receiver for watcher events
    watcher_rx: Receiver<DebouncedEvent>,

    renderer: Renderer,

    /// The instant at which the simulation started
    /// Reset on simulation restart
    sim_time: Instant,
    watching: bool,
    /// Parameters passed to shaders
    globals: Globals,

    export_data: ExportData,
}

impl Nuance {
    pub async fn init(window: Window, power_preference: PowerPreference) -> Result<Self> {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor();

        let ui_width = 280.0;
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
                target_framerate: Duration::from_secs_f32(1.0 / 60.0),
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
    pub fn run(mut self, event_loop: EventLoop<Command>) -> Result<()> {
        let mut last_draw = Instant::now();
        //let ev_sender = event_loop.create_proxy();
        // To send user events to the event loop
        let proxy = event_loop.create_proxy();

        // Time since start
        let start_time = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            if let Ok(DebouncedEvent::Write(_)) = self.watcher_rx.try_recv() {
                proxy.send_event(Command::Reload).unwrap();
            }

            // Let egui update with the window events
            self.gui.handle_event(&event);

            match event {
                // Commands allow running code on the next loop
                // This is needed because the UI code triggers some functionalities
                // we can't execute immediately
                Event::UserEvent(cmd) => match cmd {
                    Command::Load => {
                        if let Some(path) = FileDialog::new()
                            .set_parent(&self.window)
                            .add_filter("Shaders", &["glsl", "frag", "spv"])
                            .pick_file()
                        {
                            self.unwatch();
                            self.load(&path);
                        }
                    }
                    Command::Reload => {
                        info!("Reloading !");
                        self.load(self.shader.as_ref().unwrap().main.clone());
                    }
                    Command::Watch => {
                        self.watch();
                    }
                    Command::Unwatch => {
                        self.unwatch();
                    }
                    Command::ResetGlobals => {
                        info!("Resetting globals !");
                        // Reset the running globals
                        self.globals.reset();
                        self.sim_time = Instant::now();
                    }
                    Command::ResetParams => {
                        info!("Resetting params !");
                        if let Some(Some(metadata)) =
                            self.shader.as_mut().map(|it| it.metadata.as_mut())
                        {
                            metadata.reset_params();
                        }
                    }
                    Command::ExportImage => {
                        self.export_image();
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
                    WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                        Some(VirtualKeyCode::F1) => {
                            self.gui.profiling_window = true;
                        }
                        _ => {}
                    },
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    // Do not poll events, wait until next frame based on target fps
                    let since_last_draw = last_draw.elapsed();
                    if since_last_draw >= self.settings.target_framerate {
                        self.window.request_redraw();
                    } else {
                        // Sleep til next frame
                        *control_flow = ControlFlow::WaitUntil(
                            Instant::now() + self.settings.target_framerate - since_last_draw,
                        );
                    }

                    // Update shader time based on current restart time
                    self.globals.time = self.sim_time.elapsed().as_secs_f32();
                }
                Event::RedrawRequested(_) => {
                    // Tell the profiler we're running a new frame
                    puffin::GlobalProfiler::lock().new_frame();

                    // Update egui frame time from app start time
                    self.gui.update_time(start_time.elapsed().as_secs_f64());

                    // Query window properties
                    let window_size = self.window.inner_size();
                    let screen_desc = ScreenDescriptor {
                        physical_width: window_size.width,
                        physical_height: window_size.height,
                        scale_factor: self.window.scale_factor() as f32,
                    };

                    // Generate the GUI
                    let paint_jobs = Gui::render(&proxy, &screen_desc, &mut self);

                    // Render the UI
                    self.renderer
                        .render(
                            &screen_desc,
                            (&self.gui.texture(), &paint_jobs),
                            &self
                                .shader
                                .as_ref()
                                .map(|it| it.metadata.as_ref().map(|it| it.params_buffer()))
                                .unwrap_or_default()
                                .unwrap_or_default(),
                            self.globals.as_std430().as_bytes(),
                        )
                        .unwrap();

                    self.globals.frame += 1;
                    last_draw = Instant::now();
                }
                _ => {}
            }
        });
    }

    /// Immediate load
    fn load<P: AsRef<Path>>(&mut self, path: P) {
        info!("Loading {}", path.as_ref().to_str().unwrap());
        let reload_start = Instant::now();

        match self.shader_loader.load_shader(&path) {
            Ok((shader, source)) => {
                let buffer_size = if let Some(metadata) = shader.metadata.as_ref() {
                    metadata.params_buffer_size()
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

    /// Immediate watch
    fn watch(&mut self) {
        // TODO should watch for every file that is part of compilation
        if let Some(path) = self.shader.as_ref().map(|it| &it.main) {
            self.watcher
                .watch(path, RecursiveMode::NonRecursive)
                .unwrap();
            info!("Watching loaded shader for changes.");
        }
    }

    /// Immediate unwatch
    fn unwatch(&mut self) {
        // TODO should unwatch for every file that is part of compilation
        if let Some(shader) = self.shader.as_ref() {
            self.watcher
                .unwatch(&shader.main)
                .expect("Unexpected state");
            info!("Not watching for changes anymore.");
        }
    }

    fn export_image(&self) {
        let export_start = Instant::now();

        let ExportData {
            size, path, format, ..
        } = &self.export_data;

        let mut globals = self.globals.clone();
        globals.resolution = *size;
        globals.ratio = globals.resolution.x as f32 / globals.resolution.y as f32;

        self.renderer
            .render_to_buffer(
                *size,
                &self
                    .shader
                    .as_ref()
                    .map(|it| it.metadata.as_ref().map(|it| it.params_buffer()))
                    .unwrap_or_default()
                    .unwrap_or_default(),
                globals.as_std430().as_bytes(),
                |buf| {
                    let image =
                        ImageBuffer::<Rgba<u8>, _>::from_raw(size.x, size.y, &buf[..]).unwrap();
                    image.save_with_format(path, *format).unwrap();
                },
            )
            .unwrap();

        info!(
            "Exported image ! (took {} ms)",
            export_start.elapsed().as_millis()
        );
    }
}
