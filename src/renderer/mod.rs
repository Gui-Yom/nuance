use std::mem;
use std::num::NonZeroU32;

use anyhow::{Context, Result};
use egui::ClippedMesh;
use egui_wgpu_backend::ScreenDescriptor;
use log::{debug, error, info};
use mint::Vector2;
use puffin::ProfilerScope;
use wgpu::*;
use winit::window::Window;

use crate::renderer::shader::ShaderRenderPass;

mod shader;

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
    render_size: Vector2<u32>,

    vertex_shader: ShaderModule,
    render_tex: Texture,
    last_render_tex: Texture,
    #[allow(dead_code)]
    sampler: Sampler,
    last_render_tex_bgl: BindGroupLayout,
    last_render_tex_bg: BindGroup,

    shader_module: Option<ShaderModule>,
    shader_rpass: Option<ShaderRenderPass>,
    pub egui_rpass: egui_wgpu_backend::RenderPass,
}

impl Renderer {
    pub async fn new(
        window: &Window,
        power_preference: PowerPreference,
        render_size: Vector2<u32>,
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

        info!(
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
                        max_push_constant_size: push_constants_size,
                        ..Default::default()
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

        let render_tex_desc = TextureDescriptor {
            label: Some("shader render tex"),
            size: Extent3d {
                width: render_size.x,
                height: render_size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED | TextureUsage::COPY_SRC,
        };
        let render_tex = device.create_texture(&render_tex_desc);

        let last_render_tex_desc = TextureDescriptor {
            label: Some("shader last render tex"),
            size: Extent3d {
                width: render_size.x,
                height: render_size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        };
        let last_render_tex = device.create_texture(&last_render_tex_desc);

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("last render tex sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            ..Default::default()
        });

        let last_render_tex_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        filtering: false,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let last_render_tex_bg = device.create_bind_group(&BindGroupDescriptor {
            label: Some("last tex bind group"),
            layout: &last_render_tex_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&last_render_tex.create_view(
                        &TextureViewDescriptor {
                            label: None,
                            format: Some(format),
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: 0,
                            array_layer_count: None,
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let vertex_shader = device.create_shader_module(&include_spirv!("screen.vert.spv"));

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
            render_size,
            vertex_shader,
            render_tex,
            last_render_tex,
            sampler,
            last_render_tex_bgl,
            last_render_tex_bg,

            // Start with nothing loaded
            shader_module: None,
            shader_rpass: None,
            egui_rpass,
        })
    }

    pub fn set_shader(
        &mut self,
        shader_source: ShaderSource,
        push_constant_size: u32,
        params_buffer_size: u64,
    ) {
        let module = self.device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("nuance fragment shader"),
            source: shader_source,
            flags: ShaderFlags::default(),
        });
        self.shader_rpass = Some(ShaderRenderPass::new(
            &self.device,
            &self.vertex_shader,
            &module,
            &self.last_render_tex_bgl,
            push_constant_size,
            params_buffer_size,
            self.format,
        ));
        self.shader_module = Some(module);
    }

    pub fn render(
        &mut self,
        screen_desc: &ScreenDescriptor,
        gui: (&egui::Texture, &[ClippedMesh]),
        params_buffer: &[u8],
        push_constants: &[u8],
        should_render: bool,
    ) -> Result<()> {
        puffin::profile_function!();

        let mut _profiler_scope = ProfilerScope::new("init", puffin::short_file_name(file!()), "");

        // We use double buffering, so select the output texture
        let frame = self.swapchain.get_current_frame()?.output;
        let view_desc = TextureViewDescriptor::default();

        // This pack a set of render passes for the gpu to execute
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let render_tex_view = self.render_tex.create_view(&view_desc);

        mem::drop(_profiler_scope);

        if should_render {
            if let Some(shader_rpass) = self.shader_rpass.as_ref() {
                puffin::profile_scope!("shader render pass");
                shader_rpass.update_buffers(&self.queue, params_buffer);
                shader_rpass.execute(
                    &mut encoder,
                    &render_tex_view,
                    push_constants,
                    &self.last_render_tex_bg,
                );
            }
        }

        // Egui render pass
        {
            puffin::profile_scope!("egui render pass");

            self.egui_rpass
                .update_texture(&self.device, &self.queue, gui.0);
            self.egui_rpass
                .update_user_textures(&self.device, &self.queue);
            self.egui_rpass
                .update_buffers(&self.device, &self.queue, gui.1, &screen_desc);

            // Record all render passes.
            self.egui_rpass.execute(
                &mut encoder,
                &frame.view,
                gui.1,
                &screen_desc,
                Some(Color::BLACK),
            );
        }

        if should_render {
            // Copy our rendered texture to the last rendered
            encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &self.render_tex,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                ImageCopyTexture {
                    texture: &self.last_render_tex,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                Extent3d {
                    width: self.render_size.x,
                    height: self.render_size.y,
                    depth_or_array_layers: 1,
                },
            );
        }

        // Launch !
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn render_to_buffer(
        &self,
        render_size: Vector2<u32>,
        params_buffer: &[u8],
        push_constants: &[u8],
        consume: impl FnOnce(BufferView) -> Result<()>,
    ) -> Result<()> {
        if render_size.x % 64 != 0 {
            error!("Render size must be a multiple of 64 because reasons");
            return Err(anyhow::anyhow!("Invalid render size"));
        }

        let render_tex_desc = TextureDescriptor {
            label: Some("one time render"),
            size: Extent3d {
                width: render_size.x,
                height: render_size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::COPY_SRC,
        };
        let render_tex = self.device.create_texture(&render_tex_desc);

        let output_buffer_size = (4 * render_size.x * render_size.y) as BufferAddress;
        let output_buffer_desc = BufferDescriptor {
            size: output_buffer_size,
            usage: BufferUsage::COPY_DST | BufferUsage::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = self.device.create_buffer(&output_buffer_desc);

        let shader_rpass = ShaderRenderPass::new(
            &self.device,
            &self.vertex_shader,
            self.shader_module.as_ref().unwrap(),
            &self.last_render_tex_bgl,
            push_constants.len() as u32,
            params_buffer.len() as u64,
            self.format,
        );

        let render_tex_view = render_tex.create_view(&TextureViewDescriptor::default());

        // This pack a set of render passes for the gpu to execute
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("image render"),
            });

        shader_rpass.update_buffers(&self.queue, params_buffer);
        shader_rpass.execute(
            &mut encoder,
            &render_tex_view,
            push_constants,
            &self.last_render_tex_bg,
        );

        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &render_tex,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            ImageCopyBuffer {
                buffer: &output_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(4 * render_size.x),
                    rows_per_image: NonZeroU32::new(render_size.y),
                },
            },
            Extent3d {
                width: render_size.x,
                height: render_size.y,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = output_buffer.slice(..);
        let mapping = slice.map_async(MapMode::Read);
        self.device.poll(Maintain::Wait);
        futures_executor::block_on(mapping)?;
        let view = slice.get_mapped_range();

        consume(view)
    }
}
