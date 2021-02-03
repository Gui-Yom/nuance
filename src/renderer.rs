use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use wgpu::{
    include_spirv, Adapter, BackendBit, BlendState, Color, ColorTargetState, ColorWrite,
    CommandEncoderDescriptor, CullMode, Device, Features, FragmentState, FrontFace, Instance,
    Limits, LoadOp, MultisampleState, Operations, PipelineLayout, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    PushConstantRange, Queue, RenderPassColorAttachmentDescriptor, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderFlags, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStage, Surface, SwapChain, SwapChainDescriptor,
    TextureFormat, TextureUsage, VertexState,
};
use winit::window::Window;

pub struct Renderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface,
    format: TextureFormat,
    swapchain: SwapChain,
    pipeline_layout: PipelineLayout,
    pipeline: Option<RenderPipeline>,
    background_color: Color,
    vertex_shader: ShaderModule,
}

impl Renderer {
    pub async fn new(window: &Window, push_constants_size: u32) -> Result<Self> {
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
            .context("Can't find a suitable adapter")?;

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
                        max_push_constant_size: push_constants_size,
                    },
                },
                None,
            )
            .await?;

        // The output format
        let format = TextureFormat::Rgba8UnormSrgb;
        let window_size = window.inner_size();

        let swapchain = {
            // Here we create the swap chain, which is basically what does the job of
            // rendering our output in sync
            let sc_desc = SwapChainDescriptor {
                usage: TextureUsage::RENDER_ATTACHMENT,
                format,
                width: window_size.width,
                height: window_size.height,
                present_mode: PresentMode::Mailbox,
            };

            device.create_swap_chain(&surface, &sc_desc)
        };

        // This describes the data we'll send to our gpu with our shaders
        // This is where we'll declare textures and other stuff.
        // Simple variables are passed by push constants.
        /*
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("main bind group layout"),
            entries: &[],
        });*/

        // This describes the data coming to a pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("main compute layout"),
            bind_group_layouts: &[/*&bind_group_layout*/],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStage::FRAGMENT,
                range: 0..push_constants_size,
            }],
        });

        let vertex_shader = device.create_shader_module(&include_spirv!("shaders/screen.vert.spv"));

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            format,
            swapchain,
            pipeline_layout,
            pipeline: None,
            background_color: Color::BLACK,
            vertex_shader,
        })
    }

    pub fn new_pipeline_from_shader_source(&mut self, ps: ShaderSource) {
        // Describes the operations to execute on a render pass
        self.pipeline = Some(
            self.device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("main pipeline"),
                    layout: Some(&self.pipeline_layout),
                    vertex: VertexState {
                        module: &self.vertex_shader,
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
                        module: &self.device.create_shader_module(&ShaderModuleDescriptor {
                            label: Some("main fragment shader"),
                            source: ps,
                            flags: ShaderFlags::default(),
                        }),
                        entry_point: "main",
                        targets: &[ColorTargetState {
                            format: self.format,
                            alpha_blend: BlendState::default(),
                            color_blend: BlendState::default(),
                            write_mask: ColorWrite::ALL,
                        }],
                    }),
                }),
        );
    }

    pub fn render(&self, push_constants: &[u8]) -> Result<()> {
        // We use double buffering, so select the output texture
        let frame = self.swapchain.get_current_frame()?.output;
        // This pack a set of operations (render passes ...)
        // and send them to the gpu for completion
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            // Our render pass :
            // Clears the buffer with the background color and then run the pipeline
            let mut _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.background_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if self.pipeline.is_some() {
                _rpass.set_pipeline(self.pipeline.as_ref().unwrap());

                // Associated data
                //_rpass.set_bind_group(0, &bind_group, &[]);
                // Push constants mapped to uniform block
                _rpass.set_push_constants(ShaderStage::FRAGMENT, 0, push_constants);

                // We have no vertices, they are generated by the vertex shader in place.
                // But we act like we have 3, so the gpu calls the vertex shader 3 times.
                _rpass.draw(0..3, 0..1);
            }
        }

        // Launch !
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}
