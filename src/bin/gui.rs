use std::time::Instant;
use std::{fs, ops::Deref};
use std::{
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use chrono::Timelike;
use egui::{DragValue, FontDefinitions};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use glsl_lang::parse::{Parsable, ParseOptions};
use glsl_lang::{
    ast::{
        Block, DeclarationData, Expr, ExternalDeclarationData, Identifier, IdentifierData,
        LayoutQualifier, LayoutQualifierSpec, SmolStr, StructFieldSpecifier, TranslationUnit,
        TypeQualifierSpec,
    },
    transpiler::glsl::{self, FormattingState},
};
use log::debug;
use winit::event_loop::ControlFlow;
use winit::{event::Event, event_loop::EventLoopProxy};

const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

enum InternalEvent {
    RequestRedraw,
}
/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct ExampleRepaintSignal(Mutex<EventLoopProxy<InternalEvent>>);

impl epi::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0
            .lock()
            .unwrap()
            .send_event(InternalEvent::RequestRedraw)
            .ok();
    }
}

struct Slider {
    name: String,
    range: RangeInclusive<f32>,
    value: f32,
}

/// Extract sliders from a pseudo-GLSL source
/// returns the sliders and the transpiled source if necessary
fn extract_sliders_from_glsl(source: &str) -> Option<(Vec<Slider>, String)> {
    // The AST
    let (mut ast, _ctx) = TranslationUnit::parse_with_options(
        source,
        &ParseOptions {
            target_vulkan: true,
            source_id: 0,
            allow_rs_ident: false,
        }
        .build(),
    )
    .expect("Invalid GLSL source.");

    let block = extract_params_block(&mut ast);
    if block.is_none() {
        debug!("Can't find params block.");
        return None;
    }
    let block = block.unwrap();

    convert_block(block);

    let mut temp = Vec::new();
    for field in block.fields.iter_mut() {
        temp.push(create_slider_from_field(field));
        // FIXME field might not need transpiling if we allow default sliders
        convert_field(field);
    }
    let mut transpiled = String::new();
    glsl_lang::transpiler::glsl::show_translation_unit(
        &mut transpiled,
        &ast,
        FormattingState::default(),
    )
    .expect("Can't transpile ast");
    Some((temp, transpiled))
}

fn extract_params_block(ast: &mut TranslationUnit) -> Option<&mut Block> {
    ast.0.iter_mut().find_map(|node| {
        if let ExternalDeclarationData::Declaration(ref mut node) = node.content {
            if let DeclarationData::Block(ref mut block) = node.content {
                if let Some(TypeQualifierSpec::Layout(layout)) = block.qualifier.qualifiers.first()
                {
                    if let Some(LayoutQualifierSpec::Identifier(id, _)) = layout.ids.first() {
                        if id.content.0 == "params" {
                            return Some(block);
                        }
                    }
                }
            }
        }
        None
    })
}

fn create_slider_from_field(field: &StructFieldSpecifier) -> Slider {
    let name = field
        .identifiers
        .first()
        .unwrap()
        .ident
        .content
        .0
        .to_string();

    // TODO different sliders and params based on field type

    let range = match field
        .qualifier
        .as_ref()
        .unwrap()
        .qualifiers
        .first()
        .unwrap()
    {
        TypeQualifierSpec::Layout(LayoutQualifier { ids }) => {
            let mut min: f32 = 0.0;
            let mut max: f32 = 0.0;
            for qualifier in ids.iter() {
                if let LayoutQualifierSpec::Identifier(id, param) = qualifier {
                    if id.0 == "min" {
                        if let Expr::IntConst(value) = param.as_ref().unwrap().deref() {
                            min = *value as f32;
                        }
                    }
                    if id.0 == "max" {
                        if let Expr::IntConst(value) = param.as_ref().unwrap().deref() {
                            max = *value as f32;
                        }
                    }
                }
            }
            min..=max
        }
        _ => 0.0..=100.0,
    };
    Slider {
        name,
        range,
        value: 0.0,
    }
}

/// Replace the layout(params) with a predefined layout(set=?, binding=?)
fn convert_block(block: &mut Block) {
    block.qualifier.qualifiers[0] = TypeQualifierSpec::Layout(LayoutQualifier {
        ids: vec![
            LayoutQualifierSpec::Identifier(
                Identifier {
                    content: IdentifierData(SmolStr::new("set")),
                    span: None,
                },
                Some(Box::new(Expr::IntConst(0))),
            ),
            LayoutQualifierSpec::Identifier(
                Identifier {
                    content: IdentifierData(SmolStr::new("binding")),
                    span: None,
                },
                Some(Box::new(Expr::IntConst(0))),
            ),
        ],
    });
}

/// Replace the layout(min=?, max=?) with nothing
fn convert_field(field: &mut StructFieldSpecifier) {
    field.qualifier = None;
}

fn main() {
    let (mut sliders, source) = {
        let source = fs::read_to_string("shaders/purple.frag").unwrap();
        // TODO preprocess source before ?

        let option = extract_sliders_from_glsl(&source);
        option.unwrap_or((Vec::new(), source))
    };
    println!("Source : \n{}", source);

    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("Nuance")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 400,
            height: 300,
        })
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };

    let adapter =
        futures_executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
        }))
        .unwrap();

    let (mut device, mut queue) = futures_executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: OUTPUT_FORMAT,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let repaint_signal = Arc::new(ExampleRepaintSignal(Mutex::new(event_loop.create_proxy())));

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&device, OUTPUT_FORMAT);

    let start_time = Instant::now();
    let mut previous_frame_time = None;
    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            Event::RedrawRequested(_) => {
                platform.update_time(start_time.elapsed().as_secs_f64());

                let output_frame = match swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };

                // Begin to draw the UI frame.
                let egui_start = Instant::now();
                platform.begin_frame();
                let mut app_output = epi::backend::AppOutput::default();

                let mut frame = epi::backend::FrameBuilder {
                    info: epi::IntegrationInfo {
                        web_info: None,
                        cpu_usage: previous_frame_time,
                        seconds_since_midnight: Some(seconds_since_midnight()),
                        native_pixels_per_point: Some(window.scale_factor() as _),
                    },
                    tex_allocator: &mut egui_rpass,
                    output: &mut app_output,
                    repaint_signal: repaint_signal.clone(),
                }
                .build();

                egui::TopPanel::top("top_panel").show(&platform.context(), |ui| {
                    if ui.button("Hello !").clicked() {
                        println!("Clicked !");
                    }
                    ui.separator();

                    for slider in sliders.iter_mut() {
                        ui.add(
                            DragValue::f32(&mut slider.value)
                                .prefix(format!("{}: ", slider.name))
                                .clamp_range(slider.range.clone())
                                .fixed_decimals(2)
                                .speed(0.10),
                        );
                    }
                });

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let (_output, paint_commands) = platform.end_frame();
                let paint_jobs = platform.context().tessellate(paint_commands);

                let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                previous_frame_time = Some(frame_time);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    physical_width: sc_desc.width,
                    physical_height: sc_desc.height,
                    scale_factor: window.scale_factor() as f32,
                };
                egui_rpass.update_texture(&device, &queue, &platform.context().texture());
                egui_rpass.update_user_textures(&device, &queue);
                egui_rpass.update_buffers(&mut device, &mut queue, &paint_jobs, &screen_descriptor);

                // Record all render passes.
                egui_rpass.execute(
                    &mut encoder,
                    &output_frame.output.view,
                    &paint_jobs,
                    &screen_descriptor,
                    Some(wgpu::Color::BLACK),
                );

                // Submit the commands.
                queue.submit(Some(encoder.finish()));
                *control_flow = ControlFlow::Poll;
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::UserEvent(InternalEvent::RequestRedraw) => {
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    sc_desc.width = size.width;
                    sc_desc.height = size.height;
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            _ => (),
        }
    });
}

/// Time of day as seconds since midnight. Used for clock in demo app.
pub fn seconds_since_midnight() -> f64 {
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
