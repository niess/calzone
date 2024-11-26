use bvh::aabb::{Aabb, Bounded};
use bvh::bounding_hierarchy::BHShape;
use bvh::bvh::{Bvh, BvhNode};
use bvh::ray::Ray;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{MemoryError, ValueError};
use crate::utils::extract::{Extractor, Tag, TryFromBound};
use crate::utils::float::f64x3;
use crate::utils::namespace::Namespace;
use crate::utils::io::{DictLike, load_mesh};
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use nalgebra::{Point3, Vector3};
use ordered_float::OrderedFloat;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use super::{ffi, map::Map, volume::MeshShape};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, RwLock};
use super::Algorithm;


// ===============================================================================================
//
// Mesh interface.
//
// ===============================================================================================

static MESHES: LazyLock<RwLock<HashMap<MeshDefinition, MeshHandle>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static TESSELLATED_SOLIDS: LazyLock<RwLock<HashMap<MeshDefinition, TessellatedSolidHandle>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn collect_meshes() {
    MESHES
        .write()
        .unwrap()
        .retain(|_, v| Arc::strong_count(&v.facets) > 1);
    TESSELLATED_SOLIDS
        .write()
        .unwrap()
        .retain(|_, v| Arc::strong_count(&v.solid) > 1);
}

#[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MeshDefinition {
    path: PathBuf,
    scale: OrderedFloat<f64>,
    map: Option<MapParameters>,
}

#[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MapParameters {
    padding: Option<OrderedFloat<f64>>,
    origin: Option<[OrderedFloat<f64>; 3]>,
    regular: bool,
}

impl MeshDefinition {
    pub fn new(path: PathBuf, scale: f64, map: Option<MapParameters>) -> Self {
        let scale = OrderedFloat(scale);
        Self { path, scale, map }
    }

    pub fn build(
        &self,
        py: Python,
        algorithm: Option<ffi::TSTAlgorithm>
    ) -> PyResult<ffi::TSTAlgorithm> {
        let algorithm = algorithm
            .unwrap_or_else(|| if self.map.is_none() {
                ffi::TSTAlgorithm::Voxels
            } else {
                ffi::TSTAlgorithm::Bvh
            });
        match algorithm {
            ffi::TSTAlgorithm::Bvh => MeshHandle::build(py, self)?,
            ffi::TSTAlgorithm::Voxels => TessellatedSolidHandle::build(py, self)?,
            _ => unreachable!(),
        }
        Ok(algorithm)
    }

    pub fn get_mesh(&self) -> Box<MeshHandle> {
        let mesh = MESHES
            .read()
            .unwrap()
            .get(self)
            .unwrap()
            .clone();
        Box::new(mesh)
    }

    pub fn get_tessellated_solid(&self) -> Box<TessellatedSolidHandle> {
        let solid = TESSELLATED_SOLIDS
            .read()
            .unwrap()
            .get(self)
            .unwrap()
            .clone();
        Box::new(solid)
    }

    fn load_facets(&self, py: Python) -> PyResult<Vec<f32>> {
        let mut facets = match self.map.as_ref() {
            Some(params) => {
                let map = Map::from_file(py, self.path.as_path())?;
                let origin = params.origin.map(|origin| {
                    let origin: [f64; 3] = std::array::from_fn(|i| origin[i].into());
                    (&origin).into()
                });
                let padding = params.padding.map(|padding| padding.into());
                map.build_mesh(py, params.regular, origin, padding)?
            },
            None => load_mesh(self.path.as_path())
                .map_err(|msg| Error::new(ValueError).why(&msg).to_err())?,
        };

        let scale = f64::from(self.scale) as f32;
        for value in &mut facets.iter_mut() {
            *value *= scale;
        }

        Ok(facets)
    }
}

impl MapParameters {
    pub fn new(padding: Option<f64>, origin: Option<f64x3>, regular: bool) -> Self {
        let padding = padding.map(|padding| OrderedFloat(padding));
        let origin = origin.map(|origin| {
            let origin: [f64; 3] = origin.into();
            let origin: [OrderedFloat<f64>; 3] = std::array::from_fn(|i| OrderedFloat(origin[i]));
            origin
        });
        Self { padding, origin, regular }
    }
}

#[derive(Clone)]
pub struct MeshHandle {
    facets: Arc<SortedFacets>,
}

impl MeshHandle {
    fn build(
        py: Python,
        definition: &MeshDefinition,
    ) -> PyResult<()> {
        if !MESHES
            .read()
            .unwrap()
            .contains_key(&definition) {

            let facets = definition.load_facets(py)?;
            let facets = Arc::new(SortedFacets::new(facets));
            let mesh = Self { facets };

            MESHES
                .write()
                .unwrap()
                .insert(definition.clone(), mesh);
        }

        Ok(())
    }
}

// C++ interface.
impl MeshHandle {
    pub fn area(&self) -> f64 {
        self.facets.area
    }

    pub fn distance_to_in(
        &self,
        point: &ffi::G4ThreeVector,
        direction: &ffi::G4ThreeVector
    ) -> f64 {
        let point = Point3::new(point.x(), point.y(), point.z());
        let direction = Vector3::new(direction.x(), direction.y(), direction.z());
        let ray = Ray::new(point, direction);
        let (_, distance) = self.intersect(&ray, Side::Front);
        distance
    }

    pub fn distance_to_out(
        &self,
        point: &ffi::G4ThreeVector,
        direction: &ffi::G4ThreeVector,
        index: &mut i64,
    ) -> f64 {
        *index = -1;
        let point = Point3::new(point.x(), point.y(), point.z());
        let direction = Vector3::new(direction.x(), direction.y(), direction.z());
        let ray = Ray::new(point, direction);
        let (hits, distance) = self.intersect(&ray, Side::Back);
        if hits == 0 {
            0.0 // This should not happen, up to numeric uncertainties. In this case, Geant4
                // seems to return 0.
        } else {
            distance
        }
    }

    pub fn envelope(&self) -> [[f64; 3]; 2] {
        let envelope = &self.facets.envelope;
        [
            [envelope.min[0], envelope.min[1], envelope.min[2]],
            [envelope.max[0], envelope.max[1], envelope.max[2]],
        ]
    }

    pub fn inside(&self, point: &ffi::G4ThreeVector, delta: f64) -> ffi::EInside {
        // First, let us check if the point lies on the surface (according to Geant4).
        struct Match {
            distance: f64,
        }

        impl Match {
            fn inspect(
                &mut self,
                sorted_facets: &SortedFacets,
                node_index: usize,
                point: &Point3<f64>,
                delta: f64,
            ) {
                match &sorted_facets.tree.nodes[node_index] {
                    BvhNode::Leaf{shape_index, ..} => {
                        let facet = &sorted_facets.facets[*shape_index];
                        let d = facet.distance(point);
                        if d < self.distance {
                            self.distance = d;
                        }
                    },
                    BvhNode::Node{child_l_index, child_l_aabb,
                                  child_r_index, child_r_aabb, ..} => {
                        if child_l_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_l_index, point, delta)
                        }
                        if child_r_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_r_index, point, delta)
                        }
                    },
                }
            }
        }

        let point = Point3::new(point.x(), point.y(), point.z());
        let mut closest = Match { distance: f64::INFINITY };
        closest.inspect(&self.facets, 0, &point, delta);
        if closest.distance <= delta {
            return ffi::EInside::kSurface;
        }

        // Otherwise, let us check if the point actually lies outside of the bounding box.
        if !self.facets.envelope.approx_contains_eps(&point, delta) {
            return ffi::EInside::kOutside;
        }

        // Finally, let us count the number of intersections with the bounding surface. An odd
        // value implies an inner point.
        let direction = Vector3::new(0.0, 0.0, 1.0);
        let ray = Ray::new(point, direction);
        let (hits, _) = self.intersect(&ray, Side::Both);
        if (hits % 2) == 1 { ffi::EInside::kInside } else { ffi::EInside::kOutside }
    }

    fn intersect(&self, ray: &Ray<f64, 3>, side: Side ) -> (usize, f64) {
        struct Match<'a> {
            tree: &'a Bvh<f64, 3>,
            facets: &'a [TriangularFacet],
            side: Side,
            intersections: usize,
            distance: f64,
        }

        impl<'a> Match<'a> {
            fn inspect(&mut self, ray: &Ray<f64, 3>, index: usize) {
                match self.tree.nodes[index] {
                    BvhNode::Node {
                        ref child_l_aabb,
                        child_l_index,
                        ref child_r_aabb,
                        child_r_index,
                        ..
                    } => {
                        if ray_intersects_aabb(ray, child_l_aabb) {
                            self.inspect(ray, child_l_index);
                        }
                        if ray_intersects_aabb(ray, child_r_aabb) {
                            self.inspect(ray, child_r_index);
                        }
                    }
                    BvhNode::Leaf { shape_index, .. } => {
                        let facet = &self.facets[shape_index];
                        if let Some(distance) = facet.intersect(ray, self.side) {
                            self.intersections += 1;
                            if distance < self.distance {
                                self.distance = distance;
                            }
                        }
                    }
                }
            }
        }

        let mut matches = Match {
            side,
            tree: &self.facets.tree,
            facets: self.facets.facets.as_slice(),
            intersections: 0,
            distance: f64::INFINITY,
        };
        matches.inspect(ray, 0);
        (matches.intersections, matches.distance)
    }

    pub fn normal(&self, index: usize) -> [f64; 3] {
        self.facets.facets[index].normal.into()
    }

    pub fn surface_normal(&self, point: &ffi::G4ThreeVector, delta: f64) -> [f64; 3] {
        struct Match {
            normal: [f64; 3],
            distance: f64,
        }

        impl Match {
            fn inspect(
                &mut self,
                sorted_facets: &SortedFacets,
                node_index: usize,
                point: &Point3<f64>,
                delta: f64,
            ) {
                match &sorted_facets.tree.nodes[node_index] {
                    BvhNode::Leaf{shape_index, ..} => {
                        let facet = &sorted_facets.facets[*shape_index];
                        let d = facet.distance(point);
                        if d < self.distance {
                            self.normal = facet.normal.into();
                            self.distance = d;
                        }
                    },
                    BvhNode::Node{child_l_index, child_l_aabb,
                                  child_r_index, child_r_aabb, ..} => {
                        if child_l_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_l_index, point, delta)
                        }
                        if child_r_aabb.approx_contains_eps(&point, delta) {
                            self.inspect(sorted_facets, *child_r_index, point, delta)
                        }
                    },
                }
            }
        }

        let point = Point3::new(point.x(), point.y(), point.z());
        let mut closest = Match { normal: [0.0; 3], distance: f64::INFINITY };
        closest.inspect(&self.facets, 0, &point, delta);
        closest.normal
    }

    pub fn surface_point(&self, index: f64, u: f64, v: f64) -> [f64; 3] {
        let target = index * self.facets.area;
        let mut area = 0.0;
        let mut index = self.facets.facets.len() - 1;
        for (i, facet) in self.facets.facets.iter().enumerate() {
            area += facet.area;
            if target <= area {
                index = i;
                break;
            }
        }
        let facet = &self.facets.facets[index];
        let (u, v) = if u + v <= 1.0 { (u, v) } else { (1.0 - u, 1.0 - v) };
        let dr: Vector3<f64> = (u * (facet.v1 - facet.v0) + v * (facet.v2 - facet.v0)).into();
        let r = facet.v0 + dr;
        r.into()
    }
}

impl From<&MeshHandle> for Vec<f32> {
    fn from(value: &MeshHandle) -> Self {
        let mut data = Vec::<f32>::with_capacity(9 * value.facets.facets.len());
        for facet in value.facets.facets.iter() {
            data.push(facet.v0[0] as f32);
            data.push(facet.v0[1] as f32);
            data.push(facet.v0[2] as f32);
            data.push(facet.v1[0] as f32);
            data.push(facet.v1[1] as f32);
            data.push(facet.v1[2] as f32);
            data.push(facet.v2[0] as f32);
            data.push(facet.v2[1] as f32);
            data.push(facet.v2[2] as f32);
        }
        data
    }
}

#[derive(Clone)]
pub struct TessellatedSolidHandle {
    solid: Arc<*mut ffi::G4TessellatedSolid>,
}

unsafe impl Send for TessellatedSolidHandle {}
unsafe impl Sync for TessellatedSolidHandle {}

impl TessellatedSolidHandle {
    fn build(
        py: Python,
        definition: &MeshDefinition,
    ) -> PyResult<()> {
        if !TESSELLATED_SOLIDS
            .read()
            .unwrap()
            .contains_key(&definition) {

            let facets = definition.load_facets(py)?;
            let solid = ffi::create_tessellated_solid(facets);
            let result = ffi::get_error();
            match result.tp {
                ffi::ErrorType::None => (),
                ffi::ErrorType::MemoryError => {
                    let why = format!("{}", definition.path.display());
                    let err = Error::new(MemoryError).what("mesh").why(&why);
                    return Err(err.into());
                },
                _ => {
                    let why = format!("{}: {}", definition.path.display(), result.message);
                    let err = Error::new(ValueError).what("mesh").why(&why);
                    return Err(err.into());
                }
            }
            let solid = Arc::new(solid);
            let solid = Self { solid };

            TESSELLATED_SOLIDS
                .write()
                .unwrap()
                .insert(definition.clone(), solid);
        }

        Ok(())
    }

    pub fn ptr(&self) -> *mut ffi::G4TessellatedSolid {
        *self.solid
    }
}

impl From<&TessellatedSolidHandle> for Vec<f32> {
    fn from(value: &TessellatedSolidHandle) -> Self {
        let mut data = Vec::<f32>::new();
        ffi::get_facets(value, &mut data);
        data
    }
}


// ===============================================================================================
//
// Mesh implementation.
//
// ===============================================================================================

pub struct SortedFacets {
    envelope: Aabb<f64, 3>,
    facets: Vec<TriangularFacet>,
    tree: Bvh::<f64, 3>,
    area: f64,
}

#[derive(Debug)]
struct TriangularFacet {
    v0: Point3<f64>,
    v1: Point3<f64>,
    v2: Point3<f64>,
    normal: Vector3<f64>,
    area: f64,
    index: usize,
}

#[derive(Clone, Copy)]
enum Side {
    Back,
    Both,
    Front,
}

impl TriangularFacet {
    fn closest(&self, p: &Point3<f64>) -> Point3<f64> {
        // Located closest point inside triangle.
        // Ref: https://stackoverflow.com/a/74395029

        let a = &self.v0;
        let b = &self.v1;
        let c = &self.v2;

        let ab = b - a;
        let ac = c - a;
        let ap = p - a;

        let d1 = ab.dot(&ap);
        let d2 = ac.dot(&ap);
        if (d1 <= 0.0) && (d2 <= 0.0) {
            return *a; //#1
        }

        let bp = p - b;
        let d3 = ab.dot(&bp);
        let d4 = ac.dot(&bp);
        if (d3 >= 0.0) && (d4 <= d3) {
            return *b; //#2
        }

        let cp = p - c;
        let d5 = ab.dot(&cp);
        let d6 = ac.dot(&cp);
        if (d6 >= 0.0) && (d5 <= d6) {
            return *c; //#3
        }

        let vc = d1 * d4 - d3 * d2;
        if (vc <= 0.0) && (d1 >= 0.0) && (d3 <= 0.0) {
            let v = d1 / (d1 - d3);
            return a + v * ab; //#4
        }

        let vb = d5 * d2 - d1 * d6;
        if (vb <= 0.0) && (d2 >= 0.0) && (d6 <= 0.0) {
            let v = d2 / (d2 - d6);
            return a + v * ac; //#5
        }

        let va = d3 * d6 - d5 * d4;
        if (va <= 0.0) && ((d4 - d3) >= 0.0) && ((d5 - d6) >= 0.0) {
            let v = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            return b + v * (c - b); //#6
        }

        let denom = 1.0 / (va + vb + vc);
        let v = vb * denom;
        let w = vc * denom;
        return a + v * ab + w * ac; //#0
    }

    fn distance(&self, p: &Point3<f64>) -> f64 {
        let c = self.closest(p);
        (p - c).norm()
    }

    fn intersect(&self, ray: &Ray<f64, 3>, side: Side) -> Option<f64> {
        // From bvh crate, modified in order to manage back-culling.
        let a_to_b = self.v1 - self.v0;
        let a_to_c = self.v2 - self.v0;
        let u_vec = ray.direction.cross(&a_to_c);
        let det = a_to_b.dot(&u_vec);

        let hit = match side {
            Side::Back => det < -f64::EPSILON,
            Side::Both => det.abs() >= f64::EPSILON,
            Side::Front => det > f64::EPSILON,
        };
        if !hit {
            return None
        }

        let inv_det = 1.0 / det;
        let a_to_origin = ray.origin - self.v0;
        let u = a_to_origin.dot(&u_vec) * inv_det;

        if !(0.0..=1.0).contains(&u) {
            return None
        }

        let v_vec = a_to_origin.cross(&a_to_b);
        let v = ray.direction.dot(&v_vec) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None
        }

        let distance = a_to_c.dot(&v_vec) * inv_det;
        if distance > f64::EPSILON {
            Some(distance)
        } else {
            None
        }
    }

    fn new(v0: Point3<f64>, v1: Point3<f64>, v2: Point3<f64>) -> Self {
        let u = v1 - v0;
        let v = v2 - v0;
        let mut normal = u.cross(&v);
        let norm = normal.norm();
        normal /= norm;
        let area = 0.5 * norm;
        let index = 0;
        Self { v0, v1, v2, normal, area, index }
    }
}

impl Bounded<f64, 3> for TriangularFacet {
    fn aabb(&self) -> Aabb<f64, 3> {
        let mut aabb = Aabb::empty();
        aabb.grow_mut(&self.v0);
        aabb.grow_mut(&self.v1);
        aabb.grow_mut(&self.v2);
        aabb
    }
}

impl BHShape<f64, 3> for TriangularFacet {
    fn set_bh_node_index(&mut self, index: usize) {
        self.index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.index
    }
}

impl SortedFacets {
    fn new(data: Vec<f32>) -> Self {
        let mut envelope = Aabb::empty();
        let mut facets = Vec::<TriangularFacet>::with_capacity(data.len() / 9);
        let mut area = 0.0;
        for facet in data.chunks(9) {
            let [x0, y0, z0, x1, y1, z1, x2, y2, z2] = facet else { unreachable!() };
            const CM: f64 = 10.0;
            let v0 = Point3::<f64>::new(*x0 as f64 * CM, *y0 as f64 * CM, *z0 as f64 * CM);
            let v1 = Point3::<f64>::new(*x1 as f64 * CM, *y1 as f64 * CM, *z1 as f64 * CM);
            let v2 = Point3::<f64>::new(*x2 as f64 * CM, *y2 as f64 * CM, *z2 as f64 * CM);
            let facet = TriangularFacet::new(v0, v1, v2);
            area += facet.area;
            facets.push(facet);
            envelope.grow_mut(&v0);
            envelope.grow_mut(&v1);
            envelope.grow_mut(&v2);
        }
        let tree = Bvh::build(&mut facets);
        Self { envelope, facets, tree, area }
    }
}

fn ray_intersects_aabb(ray: &Ray<f64, 3>, aabb: &Aabb<f64, 3>) -> bool {
    let lbr = (aabb[0].coords - ray.origin.coords).component_mul(&ray.inv_direction);
    let rtr = (aabb[1].coords - ray.origin.coords).component_mul(&ray.inv_direction);

    let (inf, sup) = lbr.inf_sup(&rtr);

    let tmin = inf.max();
    let tmin = if tmin > 0.0 { tmin } else { 0.0 };
    let tmax = sup.min();

    tmax >= tmin
}


// ===============================================================================================
//
// Named meshes.
//
// ===============================================================================================

static NAMED_MESHES: LazyLock<RwLock<HashMap<String, MeshShape>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct NamedMesh;

impl NamedMesh {
    fn check(name: &str) -> Result<(), &'static str> {
        for c in name.chars() {
            if !c.is_alphanumeric() {
                return Err("expected an alphanumeric string");
            }
        }
        match name.chars().next() {
            None => {
                return Err("empty string");
            },
            Some(c) => if !c.is_uppercase() {
                return Err("should be capitalised");
            },
        }
        Ok(())
    }

    pub fn define(name: String, shape: MeshShape) -> Result<(), String> {
        if let Some(other) = NAMED_MESHES.read().unwrap().get(&name) {
            if (other.definition != shape.definition) || (other.algorithm != shape.algorithm) {
                let msg = format!("redefinition of '{}'", name);
                return Err(msg)
            } else {
                return Ok(())
            }
        }

        NAMED_MESHES.write().unwrap().insert(name, shape);
        Ok(())
    }

    pub fn describe(name: &str) -> Option<MeshShape> {
        NAMED_MESHES
            .read()
            .unwrap()
            .get(name)
            .map(|shape| shape.clone())
    }
}

impl TryFromBound for NamedMesh {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        // Extract meshes.
        const EXTRACTOR: Extractor<0> = Extractor::new([]);
        let tag = tag.cast("meshes");
        let mut meshes = IndexMap::<String, Bound<PyAny>>::new();
        let [] = EXTRACTOR.extract(&tag, value, Some(&mut meshes))?;

        for (name, properties) in meshes.iter() {
            Self::check(name)
                .map_err(|why| tag.bad().what("name").why(why.to_string()).to_err(ValueError))?;
            let tag = tag.extend(name, Some("mesh"), None);
            let shape = MeshShape::try_from_any(&tag, properties)?;
            Self::define(name.to_string(), shape)
                .map_err(|why| tag.bad().why(why.to_string()).to_err(ValueError))?;
        }

        Ok(Self)
    }
}

impl ToPyObject for MeshShape {
    fn to_object(&self, py: Python) -> PyObject {
        let mut references = 0;
        if self.algorithm.map(|algorithm| algorithm == Algorithm::Bvh).unwrap_or(true) {
            if let Some(mesh) = MESHES.read().unwrap().get(&self.definition) {
                references += Arc::strong_count(&mesh.facets) - 1;
            }
        }
        if self.algorithm.map(|algorithm| algorithm == Algorithm::Voxels).unwrap_or(true) {
            if let Some(solid) = TESSELLATED_SOLIDS.read().unwrap().get(&self.definition) {
                references += Arc::strong_count(&solid.solid) - 1;
            }
        }
        let references = references.to_object(py);

        let algorithm = self.algorithm
            .map(|algorithm| algorithm.to_str())
            .to_object(py);

        match &self.definition.map {
            Some(map) => Namespace::new(py, &[
                ("path", self.definition.path.to_object(py)),
                ("scale", self.definition.scale.to_object(py)),
                ("padding", map.padding.map(|padding| padding.into_inner()).to_object(py)),
                ("origin", map.origin.map(|origin| {
                    let origin: [f64; 3] = std::array::from_fn(|i| origin[i].into_inner());
                    origin
                }).to_object(py)),
                ("regular", map.regular.to_object(py)),
                ("algorithm", algorithm),
                ("references", references),
            ]).unwrap().unbind(),
            None => Namespace::new(py, &[
                ("path", self.definition.path.to_object(py)),
                ("scale", self.definition.scale.to_object(py)),
                ("algorithm", algorithm),
                ("references", references),
            ]).unwrap().unbind(),
        }
    }
}
