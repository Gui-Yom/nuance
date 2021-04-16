use anyhow::{Context, Result};
use egui::ClippedMesh;
use egui_wgpu_backend::ScreenDescriptor;
use log::debug;
use wgpu::{
    include_spirv, Adapter, BackendBit, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsage, Color, ColorTargetState, ColorWrite,
    CommandEncoderDescriptor, CullMode, Device, Extent3d, Features, FragmentState, FrontFace,
    Instance, Limits, LoadOp, MultisampleState, Operations, PipelineLayout,
    PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode, PrimitiveState,
    PrimitiveTopology, PushConstantRange, Queue, RenderPassColorAttachmentDescriptor,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderFlags, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStage, Surface,
    SwapChain, SwapChainDescriptor, Texture, TextureDescriptor, TextureFormat, TextureUsage,
    TextureViewDescriptor, VertexState,
};
use winit::window::Window;

pub struct GuiData<'a> {
    pub texture: &'a egui::Texture,
    pub paint_jobs: &'a [ClippedMesh],
}

pub struct Renderer {
    #[allow(dead_code)]
    instance: Instance,
    #[allow(dead_code)]
    adapter: Adapter,
    device: Device,

    queue: Queue,
    #[allow(dead_code)]
    surface: Surface,
    format: TextureFormat,
    swapchain: SwapChain,

    pipeline_layout: PipelineLayout,
    pipeline: Option<RenderPipeline>,
    bind_group: BindGroup,
    params_buffer: Buffer,
    background_color: Color,
    vertex_shader: ShaderModule,
    render_tex: Texture,

    pub egui_rpass: egui_wgpu_backend::RenderPass,
}

impl Renderer {
    pub async fn new(
        window: &Window,
        power_preference: PowerPreference,
        render_size: (u32, u32),
        push_constants_size: u32,
    ) -> Result<Self> {
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
                power_preference,
                compatible_surface: Some(&surface),
            })
            .await
            .context("Can't find a suitable adapter")?;

        debug!(
            "picked : {}: {:?} ({:?})",
            adapter.get_info().name,
            adapter.get_info().device_type,
            adapter.get_info().backend
        );

        // A device is an open connection to a gpu
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device_request"),
                    features: Features::PUSH_CONSTANTS,
                    limits: Limits {
                        max_bind_groups: 2,
                        max_dynamic_uniform_buffers_per_pipeline_layout: 0,
                        max_dynamic_storage_buffers_per_pipeline_layout: 0,
                        max_sampled_textures_per_shader_stage: 1,
                        max_samplers_per_shader_stage: 1,
                        max_storage_buffers_per_shader_stage: 0,
                        max_storage_textures_per_shader_stage: 0,
                        max_uniform_buffers_per_shader_stage: 2,
                        max_uniform_buffer_binding_size: 16384,
                        max_push_constant_size: push_constants_size,
                    },
                },
                None,
            )
            .await?;

        // The output format
        let format = if power_preference == PowerPreference::LowPower {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Bgra8UnormSrgb
        };
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

        let render_tex_desc = TextureDescriptor {
            label: Some("yay"),
            size: Extent3d {
                width: render_size.0,
                height: render_size.1,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
        };
        let render_tex = device.create_texture(&render_tex_desc);

        // This describes the data we'll send to our gpu with our shaders
        // This is where we'll declare textures and other stuff.
        // Simple variables are passed by push constants.

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("main bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("params ubo"),
            size: 12,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("main bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                },
            }],
        });

        // This describes the data coming to a pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("main compute layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStage::FRAGMENT,
                range: 0..push_constants_size,
            }],
        });

        let vertex_shader = device.create_shader_module(&include_spirv!("shaders/screen.vert.spv"));

        // The egui renderer in its own render pass
        let mut egui_rpass = egui_wgpu_backend::RenderPass::new(&device, format);
        // egui will need our render texture
        egui_rpass.egui_texture_from_wgpu_texture(&device, &render_tex);

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
            bind_group,
            params_buffer,
            background_color: Color::BLACK,
            vertex_shader,
            render_tex,
            egui_rpass,
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

    pub fn render(
        &mut self,
        screen_desc: ScreenDescriptor,
        gui: GuiData,
        params_buffer: &[u8],
        push_constants: &[u8],
    ) -> Result<()> {
        // We use double buffering, so select the output texture
        let frame = self.swapchain.get_current_frame()?.output;
        let view_desc = TextureViewDescriptor::default();

        // Update the params buffer on the gpu side
        self.queue
            .write_buffer(&self.params_buffer, 0, params_buffer);

        // This pack a set of render passes for the gpu to execute
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let render_tex_view = self.render_tex.create_view(&view_desc);
            // Our render pass :
            // Clears the buffer with the background color and then run the pipeline
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &render_tex_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.background_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            if self.pipeline.is_some() {
                // Bind buffers to shaders
                rpass.set_bind_group(0, &self.bind_group, &[]);

                rpass.set_pipeline(self.pipeline.as_ref().unwrap());

                // Associated data
                //_rpass.set_bind_group(0, &bind_group, &[]);
                // Push constants mapped to uniform block
                rpass.set_push_constants(ShaderStage::FRAGMENT, 0, push_constants);

                // We have no vertices, they are generated by the vertex shader in place.
                // But we act like we have 3, so the gpu calls the vertex shader 3 times.
                rpass.draw(0..3, 0..1);
            }
        }

        // Upload all resources for the GPU.
        self.egui_rpass
            .update_texture(&self.device, &self.queue, gui.texture);
        self.egui_rpass
            .update_user_textures(&self.device, &self.queue);
        self.egui_rpass
            .update_buffers(&self.device, &self.queue, gui.paint_jobs, &screen_desc);

        // Record all render passes.
        self.egui_rpass.execute(
            &mut encoder,
            &frame.view,
            gui.paint_jobs,
            &screen_desc,
            Some(Color::BLACK),
        );

        // Launch !
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}
