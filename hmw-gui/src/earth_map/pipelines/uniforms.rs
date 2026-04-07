#[derive(Debug, Clone, Copy, Default, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct EarthMapUniforms {
    pub mvp: glam::Mat4,
    pub dark_mode: u32,
    pub _padding: [u32; 3],
}

#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct EarthLatticeUniforms {
    pub vp: glam::Mat4,
    pub highlight_color: glam::Vec4,
}
