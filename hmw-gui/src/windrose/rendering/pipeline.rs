use iced::{
    Rectangle,
    wgpu::{self},
};

use super::{
    sector::RoseSector,
    uniforms::{GridlineUniformParams, GridlineUniforms, Uniforms},
};

pub struct UpdateUniformsParams {
    pub subdivisions_per_sector: u32,
    pub scale: glam::Vec3,
    pub translate: glam::Vec3,
    pub highlight_segment: u32,
    pub gridline_params: GridlineUniformParams,
    pub scaling_factor: f32,
}

#[derive(Debug)]
pub struct Pipeline {
    gridline_pipeline: GridlinePipeline,
    pipeline: wgpu::RenderPipeline,
    rose_buffer: wgpu::Buffer,
    uniforms_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Number of vertices in each sector.
    vertices: u32,
    /// Number of sectors.
    instances: u32,
    /// The max number of instances. Aka max number of sectors. aka size of the buffer.
    max_instances: u32,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, max_sectors: u32) -> Self {
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("windrose uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let rose_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("windrose sector buffer"),
            size: std::mem::size_of::<RoseSector>() as u64 * max_sectors as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("windrose bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("windrose bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rose_buffer.as_entire_binding(),
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("windrose pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/rose.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("windrose pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            rose_buffer,
            uniforms_buffer,
            bind_group,
            vertices: 0,
            instances: 0,
            max_instances: max_sectors,
            gridline_pipeline: GridlinePipeline::new(device, format),
        }
    }

    pub fn update_uniforms(&mut self, queue: &wgpu::Queue, params: UpdateUniformsParams) {
        let uniforms = Uniforms::new(
            params.scale * params.scaling_factor,
            glam::Quat::IDENTITY,
            params.translate,
            params.subdivisions_per_sector,
            params.highlight_segment,
        );
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
        self.vertices = params.subdivisions_per_sector * 6;
        self.gridline_pipeline.update_uniforms(
            queue,
            params.scale,
            params.translate,
            params.gridline_params,
        );
    }

    pub fn update_sectors(
        &mut self,
        queue: &wgpu::Queue,
        sectors: &[RoseSector],
    ) -> Result<(), &'static str> {
        if sectors.len() > self.max_instances as usize {
            return Err("exceeded max number of sectors");
        }
        self.instances = sectors.len() as u32;

        queue.write_buffer(&self.rose_buffer, 0, bytemuck::cast_slice(sectors));

        Ok(())
    }

    pub fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("windrose.pipeline.pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);
        pass.draw(0..self.vertices, 0..self.instances);
        drop(pass);

        self.gridline_pipeline.render(target, encoder, viewport);
    }
}

#[derive(Debug)]
struct GridlinePipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: u32,
    instances: u32,
}

impl GridlinePipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("windrose uniform buffer"),
            size: std::mem::size_of::<GridlineUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("windrose bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("windrose bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/gridlines.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("windrose pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("windrose pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniforms_buffer,
            bind_group,
            vertices: 0,
            instances: 0,
        }
    }

    fn update_uniforms(
        &mut self,
        queue: &wgpu::Queue,
        scale: glam::Vec3,
        translate: glam::Vec3,
        params: GridlineUniformParams,
    ) {
        self.vertices = params.segments_per_gridline * 6;
        self.instances = params.gridlines;
        let uniforms = GridlineUniforms::new(scale, glam::Quat::IDENTITY, translate, params);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("windrose.pipeline.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_scissor_rect(viewport.x, viewport.y, viewport.width, viewport.height);

            pass.draw(0..self.vertices, 0..self.instances);
        }
    }
}
