use std::sync::Arc;

use iced::{Rectangle, advanced::mouse, wgpu, widget::shader};

use super::{
    pipeline::{Pipeline, UpdateUniformsParams},
    sector::RoseSector,
    uniforms::GridlineUniformParams,
};

pub const DEFAULT_GRIDLINE_COLOR: glam::Vec4 = glam::Vec4::new(0.3, 0.3, 0.3, 0.6);
const DEFAULT_MAX_ROSE_SECTORS: u32 = 1000;
const DEFAULT_SUBDIVISIONS_PER_SECTOR: u32 = 200;
const DEFAULT_GRIDLINE_PARAMS: GridlineUniformParams = GridlineUniformParams {
    segments_per_gridline: 100,
    gridlines: 5,
    gridline_thickness: 0.005,
    gridline_color: DEFAULT_GRIDLINE_COLOR,
    scaling_factor: 1.,
};

#[derive(Debug, Clone)]
pub struct WindRoseProgram {
    instance: usize,
    sectors: Arc<Box<[RoseSector]>>,
    gridlines: u32,
    highlight_segment: Option<u32>,
    scaling_factor: f32,
    apply_scaling_factor_to_gridlines: bool,
}

impl WindRoseProgram {
    pub fn new(
        instance: usize,
        sectors: Arc<Box<[RoseSector]>>,
        gridlines: u32,
        highlight_segment: Option<u32>,
        scaling_factor: f32,
        apply_scaling_factor_to_gridlines: bool,
    ) -> Self {
        Self {
            instance,
            sectors,
            gridlines,
            highlight_segment,
            scaling_factor,
            apply_scaling_factor_to_gridlines,
        }
    }
}

impl<Message> shader::Program<Message> for WindRoseProgram {
    type State = ();
    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        Primitive::new(
            self.instance,
            self.sectors.clone(),
            self.gridlines,
            self.highlight_segment
                .unwrap_or(DEFAULT_MAX_ROSE_SECTORS + 1),
            self.scaling_factor,
            self.apply_scaling_factor_to_gridlines,
        )
    }
}

#[derive(Debug)]
pub struct Primitive {
    instance: usize,
    sectors: Arc<Box<[RoseSector]>>,
    gridlines: u32,
    highlight_segment: u32,
    scaling_factor: f32,
    apply_scaling_factor_to_gridlines: bool,
}

#[derive(Debug, Default)]
pub struct PipelineCollection {
    format: Option<wgpu::TextureFormat>,
    pipelines: Vec<Pipeline>,
}

impl shader::Pipeline for PipelineCollection {
    fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            format: Some(format),
            pipelines: Vec::new(),
        }
    }
}

impl Primitive {
    pub fn new(
        instance: usize,
        sectors: Arc<Box<[RoseSector]>>,
        gridlines: u32,
        highlight_segment: u32,
        scaling_factor: f32,
        apply_scaling_factor_to_gridlines: bool,
    ) -> Self {
        Self {
            instance,
            sectors,
            gridlines,
            highlight_segment,
            scaling_factor,
            apply_scaling_factor_to_gridlines,
        }
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = PipelineCollection;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let format = pipeline
            .format
            .expect("shader pipeline format should be initialized");

        while pipeline.pipelines.len() <= self.instance {
            pipeline
                .pipelines
                .push(Pipeline::new(device, format, DEFAULT_MAX_ROSE_SECTORS));
        }

        let pipeline = &mut pipeline.pipelines[self.instance];

        pipeline
            .update_sectors(queue, &self.sectors)
            .expect("updating sectors is fine");

        let viewport_size: glam::Vec2 = <[f32; 2]>::from(viewport.logical_size()).into();
        let bounds_vec: glam::Vec2 = <[f32; 2]>::from(bounds.size()).into();
        let scale = bounds_vec / viewport_size;

        let bounds_center: glam::Vec2 = <[f32; 2]>::from(bounds.center()).into();
        let translate = glam::vec2(
            (bounds_center.x - viewport_size.x * 0.5) / (viewport_size.x * 0.5),
            (viewport_size.y * 0.5 - bounds_center.y) / (viewport_size.y * 0.5),
        );

        pipeline.update_uniforms(
            queue,
            UpdateUniformsParams {
                subdivisions_per_sector: DEFAULT_SUBDIVISIONS_PER_SECTOR,
                scale: glam::Vec3::new(scale.x, scale.y, 0.),
                translate: glam::Vec3::new(translate.x, translate.y, 0.),
                highlight_segment: self.highlight_segment,
                gridline_params: GridlineUniformParams {
                    gridlines: self.gridlines,
                    scaling_factor: if self.apply_scaling_factor_to_gridlines {
                        self.scaling_factor
                    } else {
                        1.
                    },
                    ..DEFAULT_GRIDLINE_PARAMS
                },
                scaling_factor: self.scaling_factor,
            },
        );
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        pipeline.pipelines[self.instance].render(target, encoder, *clip_bounds);
    }
}
