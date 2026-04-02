use std::ops::Range;

use iced::wgpu::{self, util::DeviceExt};

use super::spheroid::{Spheroid, SpheroidTextureData, SpheroidVertex};

#[derive(Debug)]
pub struct EarthMapDataBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_buffers_layout: Vec<wgpu::VertexBufferLayout<'static>>,
    map_cube_texture: wgpu::Texture,
    sampler: wgpu::Sampler,
}

impl EarthMapDataBuffers {
    pub fn new(spheroid: &Spheroid, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let vertices = spheroid.vertices();
        let indices = spheroid.triangle_indices();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("earth map vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("earth maps index buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });

        let vertex_buffers_layout = vec![wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpheroidVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        }];

        let texture_data = spheroid.texture_data();

        let map_cube_texture = device.create_texture_with_data(
            queue,
            &texture_data.into(),
            SpheroidTextureData::order(),
            texture_data.data().as_slice(),
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            vertex_buffer,
            index_buffer,
            vertex_buffers_layout,
            map_cube_texture,
            sampler,
        }
    }

    pub fn vertex_buffers_layout(&self) -> &[wgpu::VertexBufferLayout<'_>] {
        self.vertex_buffers_layout.as_slice()
    }

    pub fn add_to_pass<'s, 'p>(&'s self, pass: &mut wgpu::RenderPass<'p>)
    where
        's: 'p,
    {
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    }

    pub fn indices_range(&self) -> Range<u32> {
        0..(self.index_buffer.size() / 4) as u32
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn texture_view(&self) -> wgpu::TextureView {
        self.map_cube_texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: None,
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..wgpu::TextureViewDescriptor::default()
            })
    }
}
