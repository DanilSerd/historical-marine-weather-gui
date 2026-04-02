#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct EarthMapUniforms {
    pub mvp: glam::Mat4,
}

#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct EarthLatticeUniforms {
    pub vp: glam::Mat4,
    pub highlight_color: glam::Vec4,
}
