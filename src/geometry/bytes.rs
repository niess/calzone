use super::ffi;
use serde::Serialize;
use std::collections::HashMap;


#[derive(Serialize)]
pub struct GeometryInfo {
    pub volumes: VolumeInfo,
    pub materials: HashMap<String, MaterialInfo>,
}

#[derive(Serialize)]
pub struct VolumeInfo {
    pub name: String,
    pub solid: SolidInfo,
    pub material: String,
    pub transform: ffi::TransformInfo,
    pub daughters: Vec<VolumeInfo>,
}

#[derive(Serialize)]
pub enum SolidInfo {
    Box(ffi::BoxInfo),
    Mesh(Vec<f32>),
    Orb(ffi::OrbInfo),
    Sphere(ffi::SphereInfo),
    Tubs(ffi::TubsInfo),
}

#[derive(Serialize)]
pub struct MaterialInfo {
    pub density: f64,
    pub state: String,
    pub composition: Vec<(String, f64)>,
}
