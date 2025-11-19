use lunaris_api::{
    export_plugin,
    prelude::*,
    plugin::{Plugin, PluginContext, PluginReport, RenderJob, RenderTask, Renderer},
    render::{device, queue, RawImage},
};

/// A simple renderer that draws a single, hard-coded triangle to a texture.
pub struct TriangleRenderer {
    render_pipeline: wgpu::RenderPipeline,
}

impl Plugin for TriangleRenderer {
    fn new() -> Self {
        // This plugin is initialized within the world thread, after the global
        // wgpu device has been set up.
        let device = device();

        let shader_code = r#"
            @vertex
            fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(in_vertex_index) - 1) * 0.5;
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                return vec4<f32>(x, y, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(0.2, 0.3, 0.9, 1.0); // A nice blue color
            }
        "#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor::default());

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { render_pipeline }
    }

    fn name(&self) -> &'static str {
        "TriangleRenderer"
    }

    // Other Plugin methods are empty
    fn init(&self, _ctx: PluginContext<'_>) -> Result<()> { Ok(()) }
    fn add_schedule(&self, _schedule: &mut lunaris_api::plugin::Schedule) -> Result<()> {
        Ok(())
    }
    fn update_world(&mut self, _ctx: PluginContext<'_>) -> Result<()> { Ok(()) }
    fn report(&self, _ctx: PluginContext<'_>) -> PluginReport { PluginReport::Operational }
    fn shutdown(&mut self, _ctx: PluginContext<'_>) {}
    fn reset(&mut self, _ctx: PluginContext<'_>) {}
}

impl Renderer for TriangleRenderer {
    fn schedule_render(&self, job: RenderJob) -> Result<RenderTask> {
        let pipeline = self.render_pipeline.clone();

        let render_future = async move {
            let device = device();
            let queue = queue();
            let width = 128;
            let height = 128;

            // 1. Create a texture to render to
            let texture_desc = wgpu::TextureDescriptor {
                label: Some("render_target"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            };
            let texture = device.create_texture(&texture_desc);
            let texture_view = texture.create_view(&Default::default());

            // 2. Create a command encoder and render pass
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Triangle Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                render_pass.set_pipeline(&pipeline);
                render_pass.draw(0..3, 0..1);
            }

            // 3. Copy texture to buffer to read it back to the CPU
            let buffer_size = (width * height * 4) as u64;
            let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("render_output_buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &output_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                },
                texture.size(),
            );

            queue.submit(Some(encoder.finish()));

            // 4. Map the buffer and create the RawImage  
            let buffer_slice = output_buffer.slice(..);
            let (tx, rx) = futures::channel::oneshot::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
            // Wait for the async operation
            rx.await.unwrap().unwrap();
            let data = buffer_slice.get_mapped_range().to_vec();
            drop(buffer_slice); // unmap

            RawImage::from_rgba8(width, height, data)
        };

        Ok(Box::pin(render_future))
    }
}

export_plugin!(TriangleRenderer, id: "lunaris.core.triangle_renderer", [Renderer]);
