#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    /// trnslates, scales and rotates the wind rose for rendering.
    view: glam::Mat4,
    /// Number of triangle per section of the wind rose.
    subdivisions_per_sector: u32,
    highlight_segment: u32,
    _padding: [u8; 8],
}

impl Uniforms {
    pub fn new(
        scale: glam::Vec3,
        rotation: glam::Quat,
        translation: glam::Vec3,
        subdivisions_per_sector: u32,
        highlight_segment: u32,
    ) -> Self {
        let view = glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);
        Self {
            view,
            subdivisions_per_sector,
            highlight_segment,
            _padding: [0; 8],
        }
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GridlineUniforms {
    view: glam::Mat4,
    segments_per_gridline: u32,
    gridlines: u32,
    gridline_thickness: f32,
    _padding: [u8; 4],
    gridline_color: glam::Vec4,
}

pub struct GridlineUniformParams {
    pub segments_per_gridline: u32,
    pub gridlines: u32,
    pub gridline_thickness: f32,
    pub gridline_color: glam::Vec4,
    pub scaling_factor: f32,
}

impl GridlineUniforms {
    pub fn new(
        scale: glam::Vec3,
        rotation: glam::Quat,
        translation: glam::Vec3,
        params: GridlineUniformParams,
    ) -> Self {
        let view = glam::Mat4::from_scale_rotation_translation(
            scale * params.scaling_factor,
            rotation,
            translation,
        );
        Self {
            view,
            segments_per_gridline: params.segments_per_gridline,
            gridlines: params.gridlines,
            gridline_thickness: params.gridline_thickness,
            _padding: [0; 4],
            gridline_color: params.gridline_color,
        }
    }
}
