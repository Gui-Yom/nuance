use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsage, Color, ColorTargetState, ColorWrite, CommandEncoder, Device,
    FragmentState, FrontFace, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, ShaderStage, TextureFormat, TextureView, VertexState,
};

pub(crate) struct ShaderRenderPass {
    params_bind_group: Option<BindGroup>,
    params_buffer: Option<Buffer>,
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
        let bind_group_layout;
        let params_buffer;
        let params_bind_group;
        if params_buffer_size > 0 {
            bind_group_layout = Some(device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
            }));

            params_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("params ubo"),
                size: params_buffer_size,
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                mapped_at_creation: false,
            }));

            params_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
                label: Some("main bind group"),
                layout: bind_group_layout.as_ref().unwrap(),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: params_buffer.as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                }],
            }));
        } else {
            bind_group_layout = None;
            params_buffer = None;
            params_bind_group = None;
        }

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("nuance shader pipeline layout"),
            bind_group_layouts: &if let Some(layout) = &bind_group_layout {
                vec![layout]
            } else {
                vec![]
            },
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
        if let Some(buffer) = &self.params_buffer {
            // Update the params buffer on the gpu side
            queue.write_buffer(buffer, 0, params_buffer);
        }
    }

    pub(crate) fn execute(
        &self,
        encoder: &mut CommandEncoder,
        output_tex: &TextureView,
        push_constants: &[u8],
    ) {
        puffin::profile_scope!("record shader pass");

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
        if let Some(bind_group) = &self.params_bind_group {
            rpass.set_bind_group(0, bind_group, &[]);
        }
        rpass.set_pipeline(&self.pipeline);
        // Push constants mapped to uniform block
        rpass.set_push_constants(ShaderStage::FRAGMENT, 0, push_constants);
        // We have no vertices, they are generated by the vertex shader in place.
        // But we act like we have 3, so the gpu calls the vertex shader 3 times.
        rpass.draw(0..3, 0..1);
    }
}
