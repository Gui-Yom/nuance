use std::mem;
use std::sync::Arc;
use std::time::Duration;

use egui::special_emojis::GITHUB;
use egui::{ClippedMesh, Color32, CtxRef, DragValue, Frame, Texture, TextureId, Ui};
use egui_winit_platform::Platform;
use rfd::FileDialog;
use winit::event::Event;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::shader::{Shader, Slider};
use crate::types::Globals;
use crate::{Command, Settings};

pub struct Gui {
    /// Egui subsystem
    pub egui_platform: Platform,
    /// Logical size
    pub ui_width: u32,
}

impl Gui {
    pub fn new(egui_platform: Platform, ui_width: u32) -> Self {
        Self {
            egui_platform,
            ui_width,
        }
    }

    pub fn handle_event(&mut self, event: &Event<Command>) {
        self.egui_platform.handle_event(event);
    }

    pub fn update_time(&mut self, time: f64) {
        self.egui_platform.update_time(time);
    }

    pub fn render(
        &mut self,
        window: &Window,
        shader: Option<&mut Shader>,
        watching: &mut bool,
        settings: &mut Settings,
        globals: &Globals,
        proxy: &EventLoopProxy<Command>,
    ) -> Vec<ClippedMesh> {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        self.egui_platform.begin_frame();

        let mut framerate = (1.0 / settings.target_framerate.as_secs_f32()).round() as u32;

        egui::SidePanel::left("params", self.ui_width as f32).show(
            &self.egui_platform.context(),
            |ui| {
                ui.label(format!(
                    "resolution : {:.0}x{:.0} px",
                    globals.resolution.x, globals.resolution.y
                ));
                ui.label(format!(
                    "mouse : ({:.0}, {:.0}) px",
                    globals.mouse.x, globals.mouse.y
                ));
                ui.label(format!("mouse wheel : {:.1}", globals.mouse_wheel));
                ui.label(format!("time : {:.3} s", globals.time));
                ui.label(format!("frame : {}", globals.frame));

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
                    DragValue::new(&mut settings.mouse_wheel_step)
                        .prefix("mouse wheel inc : ")
                        .clamp_range(-100.0..=100.0)
                        .max_decimals(3)
                        .speed(0.01),
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("Shaders", &["glsl", "frag", "spv"])
                            .pick_file()
                        {
                            proxy.send_event(Command::Load(path)).unwrap();
                        }
                    }
                    if shader.is_some() && ui.checkbox(watching, "watch").changed() {
                        if *watching {
                            proxy.send_event(Command::Watch).unwrap();
                        } else {
                            proxy.send_event(Command::Unwatch).unwrap();
                        }
                    }
                });

                // Shader name
                if let Some(file) = shader.as_ref() {
                    ui.colored_label(Color32::GREEN, file.main.to_str().unwrap());
                } else {
                    ui.colored_label(Color32::RED, "No shader");
                }

                if let Some(Some(sliders)) =
                    shader.map(|it| it.metadata.as_mut().map(|it| &mut it.sliders))
                {
                    ui.separator();
                    ui.label("Params");
                    egui::Grid::new("params grid").striped(true).show(ui, |ui| {
                        for slider in sliders {
                            slider.draw(ui);
                            ui.end_row();
                        }
                    });
                }

                ui.add_space(ui.available_size().y - 2.0 * ui.spacing().item_spacing.y - 30.0);
                ui.vertical_centered(|ui| {
                    ui.hyperlink_to(
                        format!("{} Manual", GITHUB),
                        "https://github.com/Gui-Yom/nuance/blob/master/MANUAL.md",
                    );
                    ui.hyperlink_to(
                        format!("{} source code", GITHUB),
                        "https://github.com/Gui-Yom/nuance",
                    );
                });
            },
        );
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(&self.context(), |ui| {
                ui.image(
                    TextureId::User(0),
                    egui::Vec2::new(
                        (window_size.width as f32 - self.ui_width as f32 * scale_factor)
                            / scale_factor,
                        window_size.height as f32 / scale_factor,
                    ),
                );
            });

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_, paint_commands) = self.egui_platform.end_frame();

        settings.target_framerate = Duration::from_secs_f32(1.0 / framerate as f32);

        self.context().tessellate(paint_commands)
    }

    pub fn context(&self) -> CtxRef {
        self.egui_platform.context()
    }

    pub fn texture(&self) -> Arc<Texture> {
        self.egui_platform.context().texture()
    }
}

impl Slider {
    pub fn draw(&mut self, ui: &mut Ui) {
        match self {
            Slider::Float {
                name,
                min,
                max,
                value,
            } => {
                ui.label(name.as_str());
                ui.add(
                    DragValue::new(value)
                        .clamp_range(*min..=*max)
                        .max_decimals(3),
                );
            }
            Slider::Color { name, value } => {
                ui.label(name.as_str());
                // I feel bad for using unsafe BUT mint implements AsRef but not AsMut,
                // so this right here is the same implementation as AsRef but mutable
                let ref_mut = unsafe { mem::transmute(value) };
                ui.color_edit_button_rgb(ref_mut);
            }
            _ => {}
        }
    }
}
