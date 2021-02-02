use core::mem;
use std::time::{Duration, Instant};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use hotwatch::Hotwatch;
use log::{debug, error, info, warn};
use wgpu::{
    include_spirv, Adapter, BackendBit, BlendState, Color, ColorTargetState, ColorWrite,
    CommandEncoderDescriptor, CullMode, Device, Features, FragmentState, FrontFace, Instance,
    Limits, LoadOp, MultisampleState, Operations, PipelineLayout, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    PushConstantRange, Queue, RenderPassColorAttachmentDescriptor, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderFlags, ShaderModule,
    ShaderModuleDescriptor, ShaderStage, Surface, SwapChain, SwapChainDescriptor, TextureFormat,
    TextureUsage, VertexState,
};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::shader_loader::ShaderLoader;
use std::sync::mpsc::channel;

/// The globals we pass to the fragment shader
/// aligned to 32bit words
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    /// Draw area width
    width: u32,
    /// Draw area height
    height: u32,
    /// Draw area width/height ratio
    ratio: f32,
    /// Current running time in sec
    time: f32,
    /// Time since the last frame in sec
    time_delta: f32,
    /// Number of frame
    frame: u32,
}

#[derive(Default, Debug)]
struct Renderer {
    // TODO
}

#[derive(Debug)]
pub enum Command {
    Load(String),
    Close,
}

pub(crate) async fn run(window: Window, event_loop: EventLoop<Command>) -> Result<()> {
    let (_instance, _adapter, device, queue, surface) = init_wgpu(&window).await;

    let window_size = window.inner_size();

    // The output format
    let format = TextureFormat::Rgba8UnormSrgb;

    let swapchain = create_swapchain(&device, &surface, format, window_size.into());

    // This describes the data we'll send to our gpu with our shaders
    // This is where we'll declare textures and other stuff.
    // Simple variables are passed by push constants.
    /*
    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("main bind group layout"),
        entries: &[],
    });*/

    let shader_file = "shaders/time.frag";

    let mut shader_loader = ShaderLoader::new();
    // The vertex shader (place triangles for rasterization)
    // It is included at compilation because it won't ever change
    let vertex_shader = device.create_shader_module(&include_spirv!("shaders/screen.vert.spv"));
    // The fragment shader (colorize our triangles)
    let fragment_shader = &device.create_shader_module(&ShaderModuleDescriptor {
        label: Some("main fragment shader"),
        source: shader_loader.load_shader(shader_file)?,
        flags: ShaderFlags::default(),
    });

    // This describes the data coming to a pipeline
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("main compute layout"),
        bind_group_layouts: &[/*&bind_group_layout*/],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStage::FRAGMENT,
            range: 0..mem::size_of::<Globals>() as u32,
        }],
    });

    let mut pipeline = create_pipeline(
        &device,
        &pipeline_layout,
        &vertex_shader,
        fragment_shader,
        format,
    );

    let (tx, watcher_rx) = channel();

    // This value was found by fiddling a bit, the shorter, the more dangerous it is,
    // because we could receive some events twice.
    let mut watcher = Hotwatch::new_with_custom_delay(Duration::from_millis(400))
        .expect("Failed to initialize hotwatch");
    watcher
        .watch(shader_file, move |e| tx.send(e).unwrap())
        .unwrap();

    // The background color
    let background_color = Color::BLACK;

    let mut globals = Globals {
        width: window_size.width,
        height: window_size.height,
        ratio: window_size.width as f32 / window_size.height as f32,
        time: 0.0,
        time_delta: 0.0,
        frame: 0,
    };
    let started = Instant::now();
    let mut last_draw_time = Instant::now();
    let target_framerate = Duration::from_secs_f32(1.0 / 30.0);

    event_loop.run(move |event, _, control_flow| {
        // Run this loop indefinitely
        *control_flow = ControlFlow::Poll;

        if let Some(hotwatch::Event::Write(_path)) = watcher_rx.try_recv().ok() {
            info!("Reloading !");
            let reload_start = Instant::now();
            pipeline = create_pipeline(
                &device,
                &pipeline_layout,
                &vertex_shader,
                &device.create_shader_module(&ShaderModuleDescriptor {
                    label: Some("main fragment shader"),
                    source: shader_loader.load_shader(shader_file).unwrap(),
                    flags: ShaderFlags::default(),
                }),
                format,
            );
            // Reset the running globals
            globals.frame = 0;
            globals.time = 0.0;
            globals.time_delta = 0.0;

            info!(
                "Reloaded ! (took {} ms)",
                reload_start.elapsed().as_millis()
            );
        }

        match event {
            Event::UserEvent(cmd) => match cmd {
                Command::Load(_) => {}
                Command::Close => {
                    *control_flow = ControlFlow::Exit;
                }
            },
            Event::MainEventsCleared => {
                let frame_time = last_draw_time.elapsed();
                if frame_time >= target_framerate {
                    window.request_redraw();
                    last_draw_time = Instant::now();
                } else {
                    // Sleep til next frame
                    *control_flow =
                        ControlFlow::WaitUntil(Instant::now() + target_framerate - frame_time);
                }
                globals.time = started.elapsed().as_secs_f32();
                globals.time_delta = frame_time.as_secs_f32();
            }
            Event::RedrawRequested(_) => {
                // We use double buffering, so select the output texture
                let frame = swapchain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                // This pack a set of operations (render passes ...)
                // and send them to the gpu for completion
                let mut encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                {
                    // Our render pass :
                    // Clears the buffer with the background color and then run the pipeline
                    let mut _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("main render pass"),
                        color_attachments: &[RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(background_color),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                    _rpass.set_pipeline(&pipeline);

                    // Associated data
                    //_rpass.set_bind_group(0, &bind_group, &[]);
                    // Push constants mapped to uniform block
                    _rpass.set_push_constants(
                        ShaderStage::FRAGMENT,
                        0,
                        bytemuck::cast_slice(&[globals]),
                    );

                    // We have no vertices, they are generated by the vertex shader in place.
                    // But we act like we have 3, so the gpu calls the vertex shader 3 times.
                    _rpass.draw(0..3, 0..1);
                }
                globals.frame += 1;

                // Launch !
                queue.submit(Some(encoder.finish()));
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                warn!("Close the app with Esc or \"exit\" in the terminal.");
            }
            _ => {}
        }
    });
}

async fn init_wgpu(window: &Window) -> (Instance, Adapter, Device, Queue, Surface) {
    let instance = Instance::new(BackendBit::PRIMARY);
    debug!("Found adapters :");
    instance
        .enumerate_adapters(BackendBit::PRIMARY)
        .for_each(|it| {
            debug!(
                " - {}: {:?} ({:?})",
                it.get_info().name,
                it.get_info().device_type,
                it.get_info().backend
            );
        });

    // The surface describes where we'll draw our output
    let surface = unsafe { instance.create_surface(window) };

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            // Use an integrated gpu if possible
            power_preference: PowerPreference::LowPower,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Can't find a low power adapter !");

    info!(
        "picked adapter : {}: {:?} ({:?})",
        adapter.get_info().name,
        adapter.get_info().device_type,
        adapter.get_info().backend
    );

    // A device is an open connection to a gpu
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("I want a device"),
                features: Features::PUSH_CONSTANTS,
                limits: Limits {
                    max_bind_groups: 1,
                    max_dynamic_uniform_buffers_per_pipeline_layout: 0,
                    max_dynamic_storage_buffers_per_pipeline_layout: 0,
                    max_sampled_textures_per_shader_stage: 0,
                    max_samplers_per_shader_stage: 0,
                    max_storage_buffers_per_shader_stage: 0,
                    max_storage_textures_per_shader_stage: 0,
                    max_uniform_buffers_per_shader_stage: 1,
                    max_uniform_buffer_binding_size: 0,
                    max_push_constant_size: mem::size_of::<Globals>() as u32,
                },
            },
            None,
        )
        .await
        .expect("Failed to create device");

    (instance, adapter, device, queue, surface)
}

fn create_swapchain(
    device: &Device,
    surface: &Surface,
    format: TextureFormat,
    (width, height): (u32, u32),
) -> SwapChain {
    // Here we create the swap chain, which is basically what does the job of
    // rendering our output in sync
    let sc_desc = SwapChainDescriptor {
        usage: TextureUsage::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: PresentMode::Mailbox,
    };

    device.create_swap_chain(&surface, &sc_desc)
}

fn create_pipeline(
    device: &Device,
    pipeline_layout: &PipelineLayout,
    vs: &ShaderModule,
    ps: &ShaderModule,
    format: TextureFormat,
) -> RenderPipeline {
    // Describes the operations to execute on a render pass
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("main pipeline"),
        layout: Some(pipeline_layout),
        vertex: VertexState {
            module: vs,
            entry_point: "main",
            buffers: &[],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            polygon_mode: PolygonMode::Fill,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(FragmentState {
            module: ps,
            entry_point: "main",
            targets: &[ColorTargetState {
                format,
                alpha_blend: BlendState::default(),
                color_blend: BlendState::default(),
                write_mask: ColorWrite::ALL,
            }],
        }),
    })
}
