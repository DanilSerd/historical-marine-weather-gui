use iced::wgpu;

#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SpheroidVertex {
    pub position: glam::Vec3,
    pub normal: glam::Vec3,
}

/// A spheroid based on [`hexasphere::shapes::CubeSphere`].
pub struct Spheroid {
    hexasphere: hexasphere::shapes::IcoSphere<SpheroidVertex>,
    texture_data: SpheroidTextureData,
}

impl Spheroid {
    /// Create a new spheroid. In left handed coordinate system. The flattening is in the y direction WGS84.
    ///
    /// # Arguments
    ///
    /// * `subdivisions` - The number of subdivisions of the spheroid.
    /// * `texture_data` - The texture data of the spheroid.
    pub fn new(subdivisions: usize, texture_data: SpheroidTextureData) -> Self {
        let flattening = hmw_geo::WGS84_FLATTENING as f32;
        let position_scale = glam::Vec3::new(1., 1. - flattening, 1.);
        let normal_scale = glam::Vec3::new(1., 1. / position_scale.y, 1.);
        let hexasphere = hexasphere::shapes::IcoSphere::new(subdivisions, |v| SpheroidVertex {
            position: glam::Vec3::from(v) * position_scale,
            normal: (glam::Vec3::from(v) * normal_scale).normalize(),
        });
        Self {
            hexasphere,
            texture_data,
        }
    }

    pub fn vertices(&self) -> &[SpheroidVertex] {
        self.hexasphere.raw_data()
    }

    pub fn triangle_indices(&self) -> Vec<u32> {
        self.hexasphere.get_all_indices()
    }

    pub fn texture_data(&self) -> &SpheroidTextureData {
        &self.texture_data
    }
}

impl std::fmt::Debug for Spheroid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Spheroid vertices: {}, subdivisions: {}",
            self.hexasphere.raw_points().len(),
            self.hexasphere.subdivisions()
        )
    }
}

pub struct SpheroidTextureData {
    reader: ktx2::Reader<Vec<u8>>,
}

impl SpheroidTextureData {
    pub fn load(bytes: &[u8]) -> Result<Self, &'static str> {
        let reader =
            ktx2::Reader::new(bytes.to_vec()).map_err(|_| "Failed to create ktx2 reader")?;
        Ok(Self { reader })
    }

    pub fn data(&self) -> Vec<u8> {
        let mut image = Vec::with_capacity(self.reader.data().len());
        for level in self.reader.levels() {
            image.extend_from_slice(level.data);
        }
        image
    }

    pub fn order() -> wgpu::util::TextureDataOrder {
        wgpu::util::TextureDataOrder::MipMajor
    }
}

impl From<&SpheroidTextureData> for wgpu::TextureDescriptor<'_> {
    fn from(value: &SpheroidTextureData) -> Self {
        let header = value.reader.header();
        let format = match header.format {
            Some(ktx2::Format::R8G8B8A8_SRGB) => wgpu::TextureFormat::Rgba8UnormSrgb,
            Some(f) => panic!("Unsupported texture format: {:?}", f),
            None => panic!("No texture format found"),
        };
        wgpu::TextureDescriptor {
            label: Some("map cube texture"),
            size: wgpu::Extent3d {
                width: header.pixel_width,
                height: header.pixel_height,
                depth_or_array_layers: header.face_count,
            },
            mip_level_count: header.level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }
    }
}

pub fn point_project_from_unit_sphere_to_spheroid_left_handed(
    point: glam::Vec3,
    spheroid_rotation: glam::Quat,
) -> Option<glam::Vec3> {
    let flattening = hmw_geo::WGS84_FLATTENING;
    let minor_axis = 1. - flattening;
    let inverse_rotation = spheroid_rotation.inverse().as_dquat();

    let point = inverse_rotation * point.as_dvec3();
    let ray = inverse_rotation * glam::dvec3(0., 0., 1.);

    let a = ray.x.powf(2.) + ray.z.powf(2.) + (ray.y / minor_axis).powf(2.);
    let b = 2. * (point.x * ray.x + point.z * ray.z + point.y * ray.y / minor_axis.powf(2.));
    let c = point.x.powf(2.) + point.z.powf(2.) + (point.y / minor_axis).powf(2.) - 1.;

    let discriminant = b.powf(2.) - 4. * a * c;
    if discriminant < 0. {
        return None;
    }

    let q = -0.5 * (b + b.signum() * discriminant.sqrt());
    let t = (q / a, c / q);

    let t = if t.0 < t.1 {
        if t.0 < 0. { t.1 } else { t.0 }
    } else {
        t.1
    };

    Some((point + t * ray).as_vec3())
}

pub fn point_project_from_circle_to_unit_sphere_left_handed(
    point: glam::Vec2,
    radius: f32,
) -> Option<glam::Vec3> {
    let p = point / radius;
    let x_sqr = p.x.powf(2.);
    let y_sqr = p.y.powf(2.);

    if x_sqr + y_sqr >= 1. {
        return None;
    }

    Some(glam::vec3(p.x, p.y, -(1. - (x_sqr + y_sqr)).sqrt()).normalize())
}

pub fn geo_point_to_unit_spheroid_left_handed(point: hmw_geo::geo::Point) -> glam::Vec3 {
    let ecef = hmw_geo::ECEFPoint::from(point);
    glam::Vec3::new(
        (ecef.0[1] / hmw_geo::WGS84_MAJOR_AXIS) as f32,
        (ecef.0[2] / hmw_geo::WGS84_MAJOR_AXIS) as f32,
        (-ecef.0[0] / hmw_geo::WGS84_MAJOR_AXIS) as f32,
    )
}

pub fn left_handed_unit_spheroid_point_to_geo(point: glam::Vec3) -> hmw_geo::geo::Point {
    let p = [
        -point.z as f64 * hmw_geo::WGS84_MAJOR_AXIS,
        point.x as f64 * hmw_geo::WGS84_MAJOR_AXIS,
        point.y as f64 * hmw_geo::WGS84_MAJOR_AXIS,
    ];
    hmw_geo::ECEFPoint(p).into()
}
