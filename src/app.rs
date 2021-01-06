use log::{debug, info};
use wgpu::{
    BindGroupLayoutDescriptor, Color, CommandEncoderDescriptor, CullMode,
    FrontFace, include_spirv, IndexFormat, Instance, LoadOp,
    Operations, PipelineLayoutDescriptor, PowerPreference,
    PresentMode, PrimitiveTopology, ProgrammableStageDescriptor,
    RasterizationStateDescriptor, RenderPassColorAttachmentDescriptor,
    RenderPassDescriptor, RenderPipelineDescriptor,
    RequestAdapterOptions, SwapChainDescriptor, TextureFormat,
    TextureUsage, VertexStateDescriptor,
};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

pub(crate) async fn run(window: &Window, event_loop: EventLoop<()>, instance: &Instance) {

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
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let size = window.inner_size();

    // The output format
    let format = TextureFormat::Rgba8UnormSrgb;

    // Here we create the swap chain, which is basically what does the job of
    // rendering our output in sync
    let sc_desc = SwapChainDescriptor {
        usage: TextureUsage::OUTPUT_ATTACHMENT,
        format,
        width: size.width,
        height: size.height,
        present_mode: PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // This is totally unused atm
    // This describes the data we'll send to our gpu with our shaders
    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("main bind group layout"),
        entries: &[/*BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStage::FRAGMENT,
            ty: BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: None,
            },
            count: None,
        }*/],
    });

    // Currently unused
    // We will pass data to the shader with this uniform buffer object
    /*
    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: &[0],
        usage: BufferUsage::UNIFORM,
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("main bind group"),
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(uniform_buffer.slice(..)),
        }],
    });
    */

    // This describes the data coming to a pipeline
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("main compute layout"),
        bind_group_layouts: &[/*&bind_group_layout*/],
        push_constant_ranges: &[],
    });

    // The vertex shader (place triangles for rasterization)
    let vertex_shader = &device.create_shader_module(include_spirv!("shaders/screen.vert.spv"));
    // The fragment shader (colorize our triangles)
    let fragment_shader = &device.create_shader_module(include_spirv!("shaders/red.frag.spv"));

    // Describes the operations to execute on a render pass
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("main render"),
        layout: Some(&pipeline_layout),
        // First, place our points and triangles
        vertex_stage: ProgrammableStageDescriptor { module: vertex_shader, entry_point: "main" },
        // Draw a color on them
        fragment_stage: Some(ProgrammableStageDescriptor { module: fragment_shader, entry_point: "main" }),
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
    });

    // The background color
    let background_color = Color::BLACK;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::RedrawRequested(_) => {
                // We use double buffering, so select the output texture
                let frame = swap_chain
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
                    //_rpass.set_bind_group(0, &bind_group, &[0]);
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
    });
}