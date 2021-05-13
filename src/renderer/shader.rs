use egui_wgpu_backend::ScreenDescriptor;
use log::debug;
use wgpu::{
    include_spirv, Adapter, BackendBit, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsage, Color, ColorTargetState,
    ColorWrite, CommandEncoder, CommandEncoderDescriptor, Device, Extent3d, Features,
    FragmentState, FrontFace, Instance, Limits, LoadOp, MultisampleState, Operations,
    PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode,
    PrimitiveState, PrimitiveTopology, PushConstantRange, Queue, RenderBundle,
    RenderBundleDescriptor, RenderBundleEncoderDescriptor, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderFlags, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStage, Surface,
    SwapChain, SwapChainDescriptor, Texture, TextureDescriptor, TextureFormat, TextureUsage,
    TextureView, TextureViewDescriptor, VertexState,
};

pub(crate) struct ShaderRenderPass {
    params_bind_group: BindGroup,
    params_buffer: Buffer,
    pipeline: RenderPipeline,
}

impl ShaderRenderPass {
    pub(crate) fn new(
        device: &Device,
        vertex_shader: &ShaderModule,
        shader_source: &ShaderModule,
        push_constants_size: u32,
        params_buffer_size: u64,
        format: TextureFormat,
    ) -> Self {
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
            size: params_buffer_size,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("main bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("nuance shader pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStage::FRAGMENT,
                range: 0..push_constants_size,
            }],
        });

        // Describes the operations to execute on a render pass
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("nuance shader pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: vertex_shader,
                entry_point: "main",
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                clamp_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: shader_source,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format,
                    write_mask: ColorWrite::ALL,
                    blend: None,
                }],
            }),
        });

        Self {
            params_bind_group,
            params_buffer,
            pipeline,
        }
    }

    pub(crate) fn update_buffers(&self, queue: &Queue, params_buffer: &[u8]) {
        // Update the params buffer on the gpu side
        queue.write_buffer(&self.params_buffer, 0, params_buffer);
    }

    pub(crate) fn execute(
        &self,
        encoder: &mut CommandEncoder,
        output_tex: &TextureView,
        push_constants: &[u8],
    ) {
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("main render pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: output_tex,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_bind_group(0, &self.params_bind_group, &[]);
        rpass.set_pipeline(&self.pipeline);
        // Push constants mapped to uniform block
        rpass.set_push_constants(ShaderStage::FRAGMENT, 0, push_constants);
        // We have no vertices, they are generated by the vertex shader in place.
        // But we act like we have 3, so the gpu calls the vertex shader 3 times.
        rpass.draw(0..3, 0..1);
    }
}
