use core::mem;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use hotwatch::notify::DebouncedEvent;
use hotwatch::Hotwatch;
use log::{debug, error, info};
use wgpu::{
    include_spirv, Adapter, BackendBit, BindGroupLayout, BindGroupLayoutDescriptor, Color,
    CommandEncoderDescriptor, CullMode, Device, Features, FrontFace, IndexFormat, Instance, Limits,
    LoadOp, Operations, PipelineLayout, PipelineLayoutDescriptor, PowerPreference, PresentMode,
    PrimitiveTopology, ProgrammableStageDescriptor, PushConstantRange, Queue,
    RasterizationStateDescriptor, RenderPassColorAttachmentDescriptor, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModule, ShaderStage,
    Surface, SwapChain, SwapChainDescriptor, TextureFormat, TextureUsage, VertexStateDescriptor,
};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::shader_loader::ShaderLoader;

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
}

pub(crate) async fn run(window: Window, event_loop: EventLoop<()>) {
    let (_instance, _adapter, device, queue, surface) = init_wgpu(&window).await;

    let window_size = window.inner_size();

    // The output format
    let format = TextureFormat::Rgba8UnormSrgb;

    let mut swapchain = create_swapchain(&device, &surface, format, window_size.into());

    // This describes the data we'll send to our gpu with our shaders
    // This is where we'll declare textures and other stuff.
    // Simple variables are passed by push constants.
    /*
    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("main bind group layout"),
        entries: &[],
    });*/

    let shader_file = "shaders/purple.frag";

    let mut shader_loader = ShaderLoader::new();
    // The vertex shader (place triangles for rasterization)
    // It is included at compilation because it won't ever change
    let vertex_shader = device.create_shader_module(include_spirv!("shaders/screen.vert.spv"));
    // The fragment shader (colorize our triangles)
    let fragment_shader = &device.create_shader_module(
        shader_loader
            .load_shader(shader_file)
            .expect("Can't load shader"),
    );

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

    let mut watcher = Hotwatch::new().expect("Failed to initialize hotwatch");
    watcher
        .watch("shaders/purple.frag", move |e| tx.send(e).unwrap())
        .unwrap();

    // The background color
    let background_color = Color::BLACK;

    let mut globals = Globals {
        width: window_size.width,
        height: window_size.height,
        ratio: window_size.width as f32 / window_size.height as f32,
        time: 0.0,
        time_delta: 0.0,
    };
    let started = Instant::now();
    let mut last_draw_time = Instant::now();
    let target_framerate = Duration::from_secs_f32(1.0 / 30.0);

    event_loop.run(move |event, _, control_flow| {
        // Run this loop indefinitely
        *control_flow = ControlFlow::Poll;
        match event {
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

                // Launch !
                queue.submit(Some(encoder.finish()));
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }

        let event = watcher_rx.try_recv().ok();
        if let Some(hotwatch::Event::Write(_path)) = event {
            pipeline = create_pipeline(
                &device,
                &pipeline_layout,
                &vertex_shader,
                &device.create_shader_module(
                    shader_loader
                        .load_shader(shader_file)
                        .expect("Can't load shader"),
                ),
                format,
            );
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
                shader_validation: true,
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
        usage: TextureUsage::OUTPUT_ATTACHMENT,
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
        label: Some("main render"),
        layout: Some(&pipeline_layout),
        // First, place our points and triangles
        vertex_stage: ProgrammableStageDescriptor {
            module: vs,
            entry_point: "main",
        },
        // Draw a color on them
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: ps,
            entry_point: "main",
        }),
        // Describes the rasterization stage
        rasterization_state: Some(RasterizationStateDescriptor {
            // The orientation of our triangles
            front_face: FrontFace::Ccw,
            // The culling mode (wether the triangles have a front side or not)
            // as we only paint the front side usually
            // Here we don't care
            cull_mode: CullMode::None,
            clamp_depth: false,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        // How the gpu should interpret our vertex buffer
        // In our case, it's just a single triangle
        primitive_topology: PrimitiveTopology::TriangleList,
        color_states: &[format.into()],
        depth_stencil_state: None,
        // Describe our vertex buffers
        // In our case, we don't have anu since they are generated by the vertex shader
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        // 1 sample per pixel
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}
