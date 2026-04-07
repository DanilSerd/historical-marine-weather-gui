use core::f32;

/// Rose sector. Each sector (annular sector) is placed on a unit circle.
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct RoseSector {
    /// Color of the rose sector.
    pub color: glam::Vec4,
    /// The inner radius fraction. Distance from unit circle origin for the smaller arc.
    pub inner: f32,
    /// The outer radius fraction. Distance from unit circle origin for larger arc.
    pub outer: f32,
    /// Where the sector starts around the circle.
    pub sweep_start_angle: f32,
    /// Where the sector end around the circle.
    pub sweep_end_angle: f32,
}

impl RoseSector {
    pub fn new(
        color: iced::Color,
        inner: f32,
        outer: f32,
        sweep_start_angle: f32,
        sweep_end_angle: f32,
    ) -> Result<Self, &'static str> {
        (0.0..1.0)
            .contains(&inner)
            .then_some(())
            .ok_or("inner must be: 0 >= inner < 1")?;
        (outer > 0. && outer <= 1.)
            .then_some(())
            .ok_or("outer must be: 0 > outer <= 1")?;
        (outer > inner)
            .then_some(())
            .ok_or("outer must be greater than inner")?;
        (sweep_start_angle >= 0. && sweep_end_angle > 0.)
            .then_some(())
            .ok_or("sweep start and end angle must be positive.")?;
        (sweep_start_angle < sweep_end_angle)
            .then_some(())
            .ok_or("sweep start must be less than end")?;

        Ok(Self {
            color: color.into_linear().into(),
            inner,
            outer,
            sweep_start_angle,
            sweep_end_angle,
        })
    }
}
