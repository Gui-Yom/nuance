use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crevice::std430::AsStd430;
use crevice::std430::Std430;
use egui::{FontDefinitions, Style};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use image::{ImageBuffer, ImageFormat, Rgba};
use log::{debug, error, info};
use mint::Vector2;
use notify::{watcher, DebouncedEvent, Error, RecommendedWatcher, RecursiveMode, Watcher};
use rfd::FileDialog;
use wgpu::PowerPreference;
use winit::event::{Event, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;

use nuance::shader::{Shader, ShaderMetadata};
use nuance::shader_loader::ShaderLoader;
use nuance::Globals;

use crate::app::gui::Gui;
use crate::app::renderer::Renderer;

mod gui;
mod renderer;

pub struct Settings {
    pub target_framerate: Duration,
    pub mouse_wheel_step: f32,
}

pub struct ExportData {
    pub size: Vector2<u32>,
    pub format: ImageFormat,
    pub path: PathBuf,
}

impl Default for ExportData {
    fn default() -> Self {
        Self {
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
    watching: bool,

    renderer: Renderer,
    /// Parameters passed to shaders
    globals: Globals,

    // Time since start
    start_time: Instant,
    // Time since last draw
    last_draw: Instant,

    /// The instant at which the simulation started
    /// Reset on simulation restart and on unpause
    sim_start: Instant,
    /// Accumulated time since simulation start, to account for pauses
    /// Reset on simulation restart
    sim_duration: Duration,
    paused: bool,

    /// Export configuration
    export_data: ExportData,

    ask_load: bool,
    ask_export: bool,
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
            watching: false,
            globals: Globals {
                resolution: Vector2::from([canvas_size.width, canvas_size.height]),
                mouse: Vector2::from([0, 0]),
                mouse_wheel: 0.0,
                ratio: (canvas_size.width) as f32 / canvas_size.height as f32,
                time: 0.0,
                frame: 0,
            },
            start_time: Instant::now(),
            last_draw: Instant::now(),
            sim_start: Instant::now(),
            sim_duration: Duration::from_nanos(0),
            paused: false,
            export_data: Default::default(),
            ask_load: false,
            ask_export: false,
        })
    }

    /// Runs the window, will block the thread until completion
    pub fn run(&mut self, event: Event<'_, ()>, control_flow: &mut ControlFlow) -> Result<()> {
        // Poll the file watcher
        if let Ok(DebouncedEvent::Write(_)) = self.watcher_rx.try_recv() {
            self.reload();
        }

        match event {
            Event::WindowEvent {
                event: ref wevent,
                window_id,
            } => {
                if window_id == self.window.id() {
                    // Let egui update with the window events
                    self.gui.handle_event(&event);
                    match wevent {
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
                    }
                }
            }
            Event::MainEventsCleared => {
                // Do not poll events, wait until next frame based on target fps
                let since_last_draw = self.last_draw.elapsed();
                if since_last_draw >= self.settings.target_framerate {
                    self.window.request_redraw();
                } else {
                    // Sleep til next frame
                    *control_flow = ControlFlow::WaitUntil(
                        Instant::now() + self.settings.target_framerate - since_last_draw,
                    );
                }

                // Update shader timem
                if !self.is_paused() {
                    self.globals.time =
                        (self.sim_start.elapsed() + self.sim_duration).as_secs_f32();
                }

                if self.ask_load {
                    if let Some(path) = FileDialog::new()
                        .set_parent(&self.window)
                        .add_filter("Shaders", &["glsl", "frag", "spv"])
                        .pick_file()
                    {
                        self.unwatch();
                        self.load(&path);
                    }
                    self.ask_load = false;
                }

                if self.ask_export {
                    if let Some(path) = FileDialog::new()
                        .set_parent(&self.window)
                        .add_filter("Image", self.export_data.format.extensions_str())
                        .save_file()
                    {
                        self.export_data.path.push(&path);
                        self.export_image();
                    }
                    self.ask_export = false;
                }
            }
            Event::RedrawRequested(window_id) => {
                if window_id == self.window.id() {
                    // Tell the profiler we're running a new frame
                    puffin::GlobalProfiler::lock().new_frame();

                    // Update egui frame time from app start time
                    self.gui
                        .update_time(self.start_time.elapsed().as_secs_f64());

                    // Query window properties
                    let window_size = self.window.inner_size();
                    let screen_desc = ScreenDescriptor {
                        physical_width: window_size.width,
                        physical_height: window_size.height,
                        scale_factor: self.window.scale_factor() as f32,
                    };

                    // Generate the GUI
                    let paint_jobs = Gui::render(self, &screen_desc);

                    // Render the UI
                    self.renderer
                        .render(
                            &screen_desc,
                            (&self.gui.texture(), &paint_jobs),
                            &self
                                .shader_metadata()
                                .map(|it| it.params_buffer())
                                .unwrap_or_default(),
                            self.globals.as_std430().as_bytes(),
                            !self.is_paused(),
                        )
                        .unwrap();

                    if !self.is_paused() {
                        self.globals.frame += 1;
                        self.last_draw = Instant::now();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// This shows a file dialog to load a shader
    /// This only happens next frame
    fn ask_to_load(&mut self) {
        self.ask_load = true;
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
                self.sim_start = Instant::now();
                self.sim_duration = Duration::from_nanos(0);

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

    fn reload(&mut self) {
        info!("Reloading !");
        let path = self.shader.as_ref().unwrap().main.clone();
        self.load(&path);
    }

    /// Watch the currently loaded file
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
            match self.watcher.unwatch(&shader.main) {
                Ok(_) => {
                    info!("Not watching for changes anymore.");
                }
                Err(e) => match e {
                    Error::WatchNotFound => {
                        info!("Was not watching ?");
                    }
                    other => {
                        error!("Can't unwatch, cause : {:?}", other);
                    }
                },
            }
        }
    }

    fn reset_globals(&mut self) {
        info!("Resetting globals !");
        // Reset the running globals
        self.globals.reset();
        self.sim_start = Instant::now();
        self.sim_duration = Duration::from_nanos(0);
    }

    fn reset_params(&mut self) {
        info!("Resetting params !");
        if let Some(metadata) = self.shader_metadata_mut() {
            metadata.reset_params();
        }
    }

    fn ask_to_export(&mut self) {
        self.ask_export = true;
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
                    .shader_metadata()
                    .map(|it| it.params_buffer())
                    .unwrap_or_default(),
                globals.as_std430().as_bytes(),
                |buf| {
                    let image = ImageBuffer::<Rgba<_>, _>::from_raw(size.x, size.y, &buf[..])
                        .context("Can't create image from buffer")?;
                    image.save_with_format(path, *format)?;

                    Ok(())
                },
            )
            .unwrap();

        info!(
            "Exported image ! (took {} ms)",
            export_start.elapsed().as_millis()
        );
    }

    fn pause(&mut self) {
        self.sim_duration += self.sim_start.elapsed();
        self.paused = true;
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn resume(&mut self) {
        self.sim_start = Instant::now();
        self.paused = false;
    }

    fn shader_loaded(&self) -> bool {
        self.shader.is_some()
    }

    fn shader_metadata(&self) -> Option<&ShaderMetadata> {
        self.shader
            .as_ref()
            .map(|it| it.metadata.as_ref())
            .flatten()
    }

    fn shader_metadata_mut(&mut self) -> Option<&mut ShaderMetadata> {
        self.shader
            .as_mut()
            .map(|it| it.metadata.as_mut())
            .flatten()
    }
}
