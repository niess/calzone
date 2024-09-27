use super::ffi;
use serde::Serialize;


#[derive(Serialize)]
pub struct VolumeInfo {
    pub solid: SolidInfo,
    pub material: String,
    pub transform: ffi::TransformInfo,
    pub daughters: Vec<VolumeInfo>,
}

#[derive(Serialize)]
pub enum SolidInfo {
    Box(ffi::BoxInfo),
    Orb(ffi::OrbInfo),
    Sphere(ffi::SphereInfo),
    Tessellation(Vec<f32>),
    Tubs(ffi::TubsInfo),
}
