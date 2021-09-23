use std::mem;
use std::sync::Arc;
use std::time::Duration;

use egui::special_emojis::GITHUB;
use egui::{ClippedMesh, Color32, CtxRef, DragValue, Frame, Id, Texture, TextureId, Ui};
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::Platform;
use image::ImageFormat;
use winit::event::Event;

use nuance::Slider;

use crate::app::Nuance;

pub struct Gui {
    /// Egui subsystem
    pub egui_platform: Platform,
    /// Logical size
    pub ui_width: u32,
    /// true if the profiling window should be open
    pub profiling_window: bool,
    export_window: bool,
}

impl Gui {
    pub fn new(egui_platform: Platform, ui_width: u32) -> Self {
        Self {
            egui_platform,
            ui_width,
            profiling_window: false,
            export_window: false,
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.egui_platform.handle_event(event);
    }

    pub fn update_time(&mut self, time: f64) {
        self.egui_platform.update_time(time);
    }

    pub fn render(app: &mut Nuance, window: &ScreenDescriptor) -> Vec<ClippedMesh> {
        // Profiler
        puffin::profile_scope!("create gui");

        app.gui.context().set_pixels_per_point(window.scale_factor);

        app.gui.egui_platform.begin_frame();

        let mut framerate = (1.0 / app.settings.target_framerate.as_secs_f32()).round() as u32;
        //app.gui.ui_width as f32
        let side_panel = egui::SidePanel::left("params").show(&app.gui.context(), |ui| {
            ui.label(format!(
                "resolution : {:.0}x{:.0} px",
                app.globals.resolution.x, app.globals.resolution.y
            ))
            .on_hover_text("The resolution of the GPU output (on the right)");
            ui.label(format!(
                "mouse : ({:.0}, {:.0}) px",
                app.globals.mouse.x, app.globals.mouse.y
            ))
            .on_hover_text("The position of the mouse pointer as sent to the shader");
            ui.label(format!("mouse wheel : {:.1}", app.globals.mouse_wheel))
                .on_hover_text("The current value of the mouse wheel global");
            ui.label(format!("time : {:.3} s", app.globals.time))
                .on_hover_text("Time elapsed since the start of the shader execution");
            ui.label(format!("frame : {}", app.globals.frame))
                .on_hover_text("Number of frames rendered since the start of the shader execution");

            if ui
                .small_button("Reset")
                .on_hover_text("Reset the shader globals")
                .clicked()
            {
                app.reset_globals();
            }

            ui.separator();

            ui.label("Settings");

            ui.add(
                DragValue::new(&mut framerate)
                    .prefix("framerate : ")
                    .clamp_range(4.0..=120.0)
                    .max_decimals(0)
                    .speed(0.1),
            )
            .on_hover_text("This is the framerate limit of the whole application.");
            ui.add(
                DragValue::new(&mut app.settings.mouse_wheel_step)
                    .prefix("mouse wheel inc : ")
                    .clamp_range(-100.0..=100.0)
                    .max_decimals(3)
                    .speed(0.01),
            )
            .on_hover_text("The rate of change of the mouse wheel global");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Load").on_hover_text("Load a new shader").clicked() {
                    app.ask_to_load();
                }
                if app.shader_loaded() {
                    if ui.button("Reload").on_hover_text("Reload this shader").clicked() {
                        app.reload_shader();
                    }
                    if ui.checkbox(&mut app.watching, "watch").on_hover_text("Watch for changes (on the filesystem) and reload the shader when necessary").changed() {
                        if app.watching {
                            app.watch();
                        } else {
                            app.unwatch();
                        }
                    }
                    if ui.button("Export").on_hover_text("Opens a window to export an image").clicked() {
                        app.gui.export_window = true;
                    }
                }
            });

            // Shader name
            if let Some(shader) = app.shader.as_ref() {
                ui.colored_label(Color32::GREEN, shader.main.to_str().unwrap());
            } else {
                ui.colored_label(Color32::RED, "No shader");
            }

            if app.shader_loaded() && ui.selectable_label(app.is_paused(), "Pause").on_hover_text("Pause the current shader execution").clicked() {
                if app.is_paused() {
                    app.resume();
                } else {
                    app.pause();
                }
            }

            let mut should_reset_params = false;
            if let Some(metadata) = app.shader_metadata_mut() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Params").on_hover_text("Params are special values you can declare in your shader and tweak in this panel");
                    if ui.button("Reset").on_hover_text("Reset all params to their default values").clicked() {
                        should_reset_params = true;
                    }
                });
                let sliders = &mut metadata.sliders;
                egui::Grid::new("params grid")
                    .striped(true)
                    //.max_col_width(self.ui_width as f32 - 20.0)
                    .show(ui, |ui| {
                        for slider in sliders {
                            draw_slider(slider, ui);
                            ui.end_row();
                        }
                    });
            }

            if should_reset_params {
                app.reset_params();
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
        }).response;

        // Update the size of the side panel
        // We want to resize the canvas it changes
        app.gui.ui_width = side_panel.rect.max.x.round() as u32;

        //log::info!("{:?}", app.gui.ui_width);
        //log::info!("{:?}", app.gui.context().used_size());

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(&app.gui.context(), |ui| {
                ui.image(
                    TextureId::User(0),
                    egui::Vec2::new(
                        window.physical_width as f32 / window.scale_factor - side_panel.rect.max.x,
                        window.physical_height as f32 / window.scale_factor,
                    ),
                );
            });

        let mut should_ask_export = false;

        let format_ref = &mut app.export_data.format;
        let size_x_ref = &mut app.export_data.size.x;
        let size_y_ref = &mut app.export_data.size.y;
        egui::Window::new("Export image")
            .id(Id::new("export image window"))
            .open(&mut app.gui.export_window)
            .collapsible(false)
            .resizable(false)
            .scroll(false)
            .show(&app.gui.egui_platform.context(), |ui| {
                egui::ComboBox::from_label("format")
                    .selected_text(format_ref.extensions_str()[0])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(format_ref, ImageFormat::Png, "PNG");
                        ui.selectable_value(format_ref, ImageFormat::Bmp, "BMP");
                        ui.selectable_value(format_ref, ImageFormat::Gif, "GIF");
                        ui.selectable_value(format_ref, ImageFormat::Jpeg, "JPEG");
                    });

                ui.horizontal(|ui| {
                    ui.label("Size :");
                    ui.add(DragValue::new(size_x_ref).suffix("px"));
                    ui.label("x");
                    ui.add(DragValue::new(size_y_ref).suffix("px"));
                });
                if *size_x_ref % 64 != 0 {
                    ui.colored_label(Color32::RED, "× Image width must be a multiple of 64");
                }

                if ui.button("export").clicked() {
                    should_ask_export = true;
                }
            });

        if should_ask_export {
            app.ask_to_export();
        }

        if app.gui.profiling_window {
            app.gui.profiling_window = puffin_egui::profiler_window(&app.gui.context());
        }

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_, paint_commands) = app.gui.egui_platform.end_frame(Some(&app.window));

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

fn draw_slider(slider: &mut Slider, ui: &mut Ui) {
    match slider {
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
                    .max_decimals(3),
            );
        }
        Slider::Uint {
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
                    .speed((*max - *min) as f32 / ui.available_width())
                    .max_decimals(3),
            );
        }
        Slider::Vec2 { name, value, .. } => {
            ui.label(name.as_str());
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.columns(2, |columns| {
                columns[0].add(DragValue::new(&mut value.x).speed(0.01).max_decimals(3));
                columns[1].add(DragValue::new(&mut value.y).speed(0.01).max_decimals(3));
            });
        }
        Slider::Vec3 { name, value, .. } => {
            ui.label(name.as_str());
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.columns(3, |columns| {
                columns[0].add(DragValue::new(&mut value.x).speed(0.01).max_decimals(3));
                columns[1].add(DragValue::new(&mut value.y).speed(0.01).max_decimals(3));
                columns[2].add(DragValue::new(&mut value.z).speed(0.01).max_decimals(3));
            });
        }
        Slider::Color { name, value, .. } => {
            ui.label(name.as_str());
            // I feel bad for doing this BUT mint only implements AsRef but not AsMut,
            // so this right here is the same implementation as AsRef but mutable
            let ref_mut = unsafe { mem::transmute(value) };
            ui.color_edit_button_rgb(ref_mut);
        }
        Slider::Bool { name, value, .. } => {
            ui.label(name.as_str());
            let mut val = *value != 0;
            if ui.checkbox(&mut val, "").changed() {
                *value = if val { 1 } else { 0 };
            }
        }
    }
}
