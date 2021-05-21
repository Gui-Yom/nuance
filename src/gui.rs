use std::mem;
use std::sync::Arc;
use std::time::Duration;

use egui::special_emojis::GITHUB;
use egui::{ClippedMesh, Color32, CtxRef, DragValue, Frame, Id, Texture, TextureId, Ui};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::Platform;
use image::ImageFormat;
use log::debug;
use rfd::FileDialog;
use winit::event::Event;
use winit::event_loop::EventLoopProxy;

use crate::shader::Slider;
use crate::{Command, Nuance};

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
        proxy: &EventLoopProxy<Command>,
        window: &ScreenDescriptor,
        app: &mut Nuance,
    ) -> Vec<ClippedMesh> {
        app.gui.egui_platform.begin_frame();

        let mut framerate = (1.0 / app.settings.target_framerate.as_secs_f32()).round() as u32;

        egui::SidePanel::left("params", app.gui.ui_width as f32).show(&app.gui.context(), |ui| {
            ui.label(format!(
                "resolution : {:.0}x{:.0} px",
                app.globals.resolution.x, app.globals.resolution.y
            ));
            ui.label(format!(
                "mouse : ({:.0}, {:.0}) px",
                app.globals.mouse.x, app.globals.mouse.y
            ));
            ui.label(format!("mouse wheel : {:.1}", app.globals.mouse_wheel));
            ui.label(format!("time : {:.3} s", app.globals.time));
            ui.label(format!("frame : {}", app.globals.frame));

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
                DragValue::new(&mut app.settings.mouse_wheel_step)
                    .prefix("mouse wheel inc : ")
                    .clamp_range(-100.0..=100.0)
                    .max_decimals(3)
                    .speed(0.01),
            );

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    app.export_data.export_prompt = true;
                }
                if ui.button("Load").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Shaders", &["glsl", "frag", "spv"])
                        .pick_file()
                    {
                        proxy.send_event(Command::Load(path)).unwrap();
                    }
                }
                if app.shader.is_some() && ui.checkbox(&mut app.watching, "watch").changed() {
                    if app.watching {
                        proxy.send_event(Command::Watch).unwrap();
                    } else {
                        proxy.send_event(Command::Unwatch).unwrap();
                    }
                }
            });

            // Shader name
            if let Some(file) = app.shader.as_ref() {
                ui.colored_label(Color32::GREEN, file.main.to_str().unwrap());
            } else {
                ui.colored_label(Color32::RED, "No shader");
            }

            if let Some(Some(metadata)) = app.shader.as_mut().map(|it| it.metadata.as_mut()) {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Params");
                    if ui.button("Reset").clicked() {
                        metadata.reset_params();
                    }
                });
                let sliders = &mut metadata.sliders;
                egui::Grid::new("params grid")
                    .striped(true)
                    //.max_col_width(self.ui_width as f32 - 20.0)
                    .show(ui, |ui| {
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
        });
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(&app.gui.context(), |ui| {
                ui.image(
                    TextureId::User(0),
                    egui::Vec2::new(
                        window.physical_width as f32 / window.scale_factor
                            - app.gui.ui_width as f32,
                        window.physical_height as f32 / window.scale_factor,
                    ),
                );
            });

        let path_ref = &mut app.export_data.path;
        let format_ref = &mut app.export_data.format;
        let size_x_ref = &mut app.export_data.size.x;
        let size_y_ref = &mut app.export_data.size.y;
        egui::Window::new("Export image")
            .id(Id::new("export image window"))
            .open(&mut app.export_data.export_prompt)
            .collapsible(false)
            .resizable(false)
            .scroll(false)
            .show(&app.gui.context(), |ui| {
                // File path
                ui.horizontal(|ui| {
                    ui.label(path_ref.to_str().unwrap());
                    if ui.button("...").clicked() {
                        if let Some(path) = FileDialog::new()
                            //.add_filter("Image", &[&format_ref.to_string()])
                            .save_file()
                        {
                            path_ref.push(&path);
                        }
                    }
                });

                if let Ok(format) = ImageFormat::from_path(path_ref) {
                    if format.can_write() {
                        *format_ref = format;
                    } else {
                        ui.colored_label(Color32::RED, "× Unsupported image format");
                    }
                } else {
                    ui.colored_label(Color32::RED, "× Unknown image format");
                }
                ui.horizontal(|ui| {
                    ui.label("Size :");
                    ui.add(DragValue::new(size_x_ref).suffix("px"));
                    ui.label("x");
                    ui.add(DragValue::new(size_y_ref).suffix("px"));
                });

                if ui.button("export").clicked() {
                    proxy.send_event(Command::Export).unwrap();
                }
            });

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_, paint_commands) = app.gui.egui_platform.end_frame();

        app.settings.target_framerate = Duration::from_secs_f32(1.0 / framerate as f32);

        app.gui.context().tessellate(paint_commands)
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
                ..
            } => {
                ui.label(name.as_str());
                ui.add(
                    DragValue::new(value)
                        .clamp_range(*min..=*max)
                        .speed((*max - *min) / ui.available_width())
                        .fixed_decimals(3),
                );
            }
            Slider::Vec2 { name, value, .. } => {
                ui.label(name.as_str());
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.columns(2, |columns| {
                    columns[0].add(DragValue::new(&mut value.x).speed(0.005).fixed_decimals(3));
                    columns[1].add(DragValue::new(&mut value.y).speed(0.005).fixed_decimals(3));
                });
            }
            Slider::Vec3 { name, value, .. } => {
                ui.label(name.as_str());
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.columns(3, |columns| {
                    columns[0].add(DragValue::new(&mut value.x).speed(0.005).fixed_decimals(3));
                    columns[1].add(DragValue::new(&mut value.y).speed(0.005).fixed_decimals(3));
                    columns[2].add(DragValue::new(&mut value.z).speed(0.005).fixed_decimals(3));
                });
            }
            Slider::Color { name, value, .. } => {
                ui.label(name.as_str());
                // I feel bad for doing this BUT mint only implements AsRef but not AsMut,
                // so this right here is the same implementation as AsRef but mutable
                let ref_mut = unsafe { mem::transmute(value) };
                ui.color_edit_button_rgb(ref_mut);
            }
        }
    }
}
