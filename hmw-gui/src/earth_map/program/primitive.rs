use std::sync::Arc;

use hmw_geo::Lattice;
use iced::{wgpu, widget::shader};

use crate::earth_map::pipelines::lattice_data::LatticeMapDataBuffers;
use crate::earth_map::pipelines::lattice_pepiline::LatticePipeline;
use crate::earth_map::pipelines::spheroid::Spheroid;
use crate::earth_map::pipelines::uniforms::EarthLatticeUniforms;

use super::super::pipelines::{
    data::EarthMapDataBuffers, pipeline::EarthPipeline, uniforms::EarthMapUniforms,
};
use super::types::{CellSelection, EarthMapColors};

#[derive(Debug, Clone)]
pub struct EarthMapPrimitive {
    pub spheroid: Arc<Spheroid>,
    pub lattice: Arc<Lattice>,
    pub scale: f32,
    pub rotation: glam::Quat,
    pub colors: EarthMapColors,
    pub cell_selection: Arc<CellSelection>,
}

#[derive(Debug, Default)]
pub struct EarthMapPrimitivePipeline {
    format: Option<wgpu::TextureFormat>,
    map: Option<EarthPipeline>,
    lattice: Option<LatticePipeline>,
    cell_selection: Arc<CellSelection>,
}

impl shader::Pipeline for EarthMapPrimitivePipeline {
    fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            format: Some(format),
            ..Self::default()
        }
    }
}

impl shader::Primitive for EarthMapPrimitive {
    type Pipeline = EarthMapPrimitivePipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        let format = pipeline
            .format
            .expect("shader pipeline format should be initialized");

        if pipeline.map.is_none() {
            let buffers = EarthMapDataBuffers::new(&self.spheroid, device, queue);
            pipeline.map = Some(EarthPipeline::new(format, device, buffers));
        }

        if pipeline.lattice.is_none() {
            let lattice_buffer = LatticeMapDataBuffers::new(&self.lattice, device);
            pipeline.lattice = Some(LatticePipeline::new(device, format, lattice_buffer));
            pipeline.cell_selection = Arc::default();
        }

        let map_pipeline = pipeline
            .map
            .as_mut()
            .expect("earth map pipeline should be initialized");

        let depth_texture_size = wgpu::Extent3d {
            width: viewport.physical_width(),
            height: viewport.physical_height(),
            depth_or_array_layers: 1,
        };

        let projection = glam::Mat4::orthographic_lh(
            0.,
            viewport.logical_size().width,
            -viewport.logical_size().height,
            0.,
            0.,
            self.scale * 2.,
        );

        let view = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(self.scale),
            self.rotation,
            glam::vec3(bounds.center_x(), -bounds.center_y(), self.scale * 2.),
        );

        let vp = projection * view;
        map_pipeline.prepare_depth_texture(device, depth_texture_size);
        map_pipeline.update_uniforms(queue, &EarthMapUniforms { mvp: vp });

        let lattice_pipeline = pipeline
            .lattice
            .as_mut()
            .expect("lattice pipeline should be initialized");
        lattice_pipeline.prepare_uniforms(
            queue,
            &EarthLatticeUniforms {
                vp,
                highlight_color: self.colors.lattice_highlight,
            },
        );

        if pipeline.cell_selection != self.cell_selection {
            lattice_pipeline.prepare_cells(
                queue,
                &self.cell_selection.selected_cells,
                &self.cell_selection.highlight_cells,
            );
            pipeline.cell_selection = self.cell_selection.clone();
        }
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        pipeline
            .map
            .as_ref()
            .expect("earth map pipeline should be initialized")
            .render(target, encoder, clip_bounds);

        pipeline
            .lattice
            .as_ref()
            .expect("lattice pipeline should be initialized")
            .render(target, encoder, clip_bounds);
    }
}
