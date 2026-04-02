use hmw_geo::{Lattice, geo::CoordsIter};
use iced::wgpu::{self, vertex_attr_array};

use super::spheroid::geo_point_to_unit_spheroid_left_handed;

const LATTICE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    vertex_attr_array![0 => Float32x3, 1 => Float32];

#[derive(Debug)]
pub struct LatticeMapDataBuffers {
    vertex_buffer: wgpu::Buffer,
    vertices: Box<[Box<[glam::Vec3]>]>,
    current_vertices_to_display: usize,
    vertex_buffers_layout: Vec<wgpu::VertexBufferLayout<'static>>,
}

impl LatticeMapDataBuffers {
    pub fn new(lattice: &Lattice, device: &wgpu::Device) -> Self {
        let size_of_vertex = std::mem::size_of::<LatticeCellVertex>() as u64;
        let vertices = from_lattice_to_vertices(lattice);
        let vertices_len: usize = vertices.iter().map(|v| v.len()).sum();
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("earth lattice vertices buffer"),
            size: size_of_vertex * vertices_len as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_buffers_layout = vec![wgpu::VertexBufferLayout {
            array_stride: size_of_vertex,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &LATTICE_VERTEX_ATTRIBUTES,
        }];
        Self {
            vertex_buffer,
            vertices,
            current_vertices_to_display: 0,
            vertex_buffers_layout,
        }
    }

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        selected_cells: &[usize],
        hilight_cells: &[usize],
    ) {
        let size_of_vertex = std::mem::size_of::<LatticeCellVertex>();
        let size: u64 = selected_cells
            .iter()
            .chain(hilight_cells.iter())
            .map(|ci| (size_of_vertex * self.vertices[*ci].len()) as u64)
            .sum();

        if size == 0 {
            self.current_vertices_to_display = 0;
            return;
        }

        let mut buffer = queue
            .write_buffer_with(&self.vertex_buffer, 0, size.try_into().unwrap())
            .expect("size correct");
        let buffer = buffer.as_mut();

        let sc = selected_cells.iter().flat_map(|ci| {
            self.vertices[*ci]
                .iter()
                .map(|v| LatticeCellVertex::selected_cell(*v))
        });
        let hc = hilight_cells.iter().flat_map(|ci| {
            self.vertices[*ci]
                .iter()
                .map(|v| LatticeCellVertex::higlight_cell(*v))
        });

        sc.chain(hc).enumerate().for_each(|(i, v)| {
            let bytes = bytemuck::bytes_of(&v);
            buffer[i * size_of_vertex..(i + 1) * size_of_vertex].copy_from_slice(bytes);
        });
        self.current_vertices_to_display = size as usize / size_of_vertex;
    }

    pub fn add_to_pass<'s, 'p>(&'s self, pass: &mut wgpu::RenderPass<'p>)
    where
        's: 'p,
    {
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    }

    pub fn vertices_to_draw(&self) -> u32 {
        self.current_vertices_to_display as u32
    }

    pub fn vertex_buffers_layout(&self) -> &[wgpu::VertexBufferLayout<'_>] {
        self.vertex_buffers_layout.as_slice()
    }
}

#[derive(Debug, Default, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct LatticeCellVertex {
    position: glam::Vec3,
    color_intensity: f32,
}

impl LatticeCellVertex {
    fn higlight_cell(position: glam::Vec3) -> Self {
        Self {
            position,
            color_intensity: 0.3,
        }
    }

    fn selected_cell(position: glam::Vec3) -> Self {
        Self {
            position,
            color_intensity: 1.0,
        }
    }
}

fn from_lattice_to_vertices(lattice: &Lattice) -> Box<[Box<[glam::Vec3]>]> {
    lattice
        .iter_ordered()
        .into_iter()
        .map(|(e, _)| e.triangulate())
        .map(|triangles| {
            triangles
                .into_iter()
                .flat_map(|t| {
                    t.coords_iter()
                        .map(|c| geo_point_to_unit_spheroid_left_handed(c.into()))
                })
                .collect()
        })
        .collect()
}
