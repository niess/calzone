#![allow(deprecated)]

use crate::utils::extract::{Extractor, Rotation, Strings, Property, Tag, TryFromBound};
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{Exception, IndexError, NotImplementedError, TypeError,
    ValueError};
use crate::utils::float::f64x3;
use crate::utils::io::DictLike;
use crate::utils::namespace::Namespace;
use crate::utils::numpy::{PyArray, PyArrayMethods};
use cxx::SharedPtr;
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyTuple};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use super::cxx::ffi;
use std::collections::HashMap;
use std::path::Path;

mod bytes;
mod goupil;
mod map;
pub mod materials;
pub mod mesh;
pub mod volume;

pub use map::Map;
pub use materials::MaterialsDefinition;
pub use volume::Algorithm;


// ===============================================================================================
//
// Geometry interface.
//
// ===============================================================================================

/// A static Monte Carlo geometry.
#[pyclass(frozen, module="calzone")]
pub struct Geometry (pub(crate) SharedPtr<ffi::GeometryBorrow>);

unsafe impl Send for ffi::GeometryBorrow {}
unsafe impl Sync for ffi::GeometryBorrow {}

#[pymethods]
impl Geometry {
    #[new]
    pub fn new(volume: DictLike) -> PyResult<Self> {
        let py = volume.py();
        let mut builder = GeometryBuilder::new(Some(volume), None)?;
        let geometry = builder.build(py)?;
        Ok(geometry)
    }

    /// The geometry root volume.
    #[getter]
    fn get_root(&self) -> PyResult<Volume> {
        Volume::new(&self.0, "__root__", true)
    }

    fn __getitem__(&self, path: &str) -> PyResult<Volume> {
        Volume::new(&self.0, path, true)
    }

    /// Check the geometry by looking for overlapping volumes.
    fn check(&self, resolution: Option<i32>) -> PyResult<()> {
        let resolution = resolution.unwrap_or(1000);
        self.0
            .check(resolution)
            .to_result()?;
        Ok(())
    }

    /// Display the geometry.
    #[pyo3(signature=(data=None,/))]
    fn display<'py>(
        &self,
        py: Python<'py>,
        data: Option<&Bound<'py, PyAny>>
    ) -> PyResult<()> {
        let root = self.get_root()?;
        let root = Bound::new(py, root)?;
        Volume::display(&root, data)
    }

    /// Export the geometry as a `goupil.ExternalGeometry` object.
    fn export<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let goupil = py.import_bound("goupil")?;
        let external_geometry = goupil.getattr("ExternalGeometry")?;
        let file = super::FILE
            .get(py)
            .unwrap();
        self.0.set_goupil();
        let args = (file,);
        external_geometry.call1(args)
    }

    /// Find a geometry volume matching the given stem.
    fn find(&self, stem: &str) -> PyResult<Volume> {
        Volume::new(&self.0, stem, false)
    }
}

// ===============================================================================================
//
// Builder interface.
//
// ===============================================================================================

/// A Monte Carlo geometry builder.
#[pyclass(module="calzone")]
#[derive(Deserialize, Serialize)]
pub struct GeometryBuilder {
    definition: GeometryDefinition,
    /// Meshes traversal algorithm.
    #[pyo3(get, set)]
    algorithm: Option<Algorithm>,
}

#[pymethods]
impl GeometryBuilder {
    #[new]
    #[pyo3(signature=(definition, /, *, algorithm=None))]
    fn new(definition: Option<DictLike>, algorithm: Option<Algorithm>) -> PyResult<Self> {
        let definition = match definition {
            Some(definition) => GeometryDefinition::new(definition, None)?,
            None => GeometryDefinition::default(),
        };
        let builder = Self { definition, algorithm };
        Ok(builder)
    }

    /// Build the Monte Carlo `Geometry`.
    fn build(&mut self, py: Python) -> PyResult<Geometry> {
        // Validate volumes.
        self.definition.volume.validate()?;

        // Build meshes.
        self.definition.volume.build_meshes(py, self.algorithm)?;

        // Build materials.
        self.definition.materials = MaterialsDefinition::drain(
            self.definition.materials.take(),
            &mut self.definition.volume,
        );
        if let Some(materials) = self.definition.materials.as_ref() {
            materials.build()?;
        }

        // Build volumes.
        let geometry = ffi::create_geometry(&self.definition.volume);
        if geometry.is_null() {
            ffi::get_error().to_result()?;
            unreachable!()
        }
        let geometry = Geometry (geometry);
        Ok(geometry)
    }

    /// Remove a volume from the geometry definition.
    fn delete<'py>(
        slf: Bound<'py, GeometryBuilder>,
        pathname: &str,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        if let Some((mother, name)) = pathname.rsplit_once('.') {
            let mut builder = slf.borrow_mut();
            let mother = builder.find_mut(mother)?;
            let n = mother.volumes.len();
            mother.volumes.retain(|v| v.name != name);
            if mother.volumes.len() < n {
                return Ok(slf);
            }
        }
        let builder = slf.borrow();
        let why = if builder.definition.volume.name() == pathname {
            format!("cannot delete root volume '{}'", pathname)
        } else {
            format!("unknown '{}' volume", pathname)
        };
        let err = Error::new(ValueError).what("geometry operation").why(&why);
        Err(err.into())
    }

    /// Modify the definition of a geometry volume.
    fn modify<'py>(
        slf: Bound<'py, GeometryBuilder>,
        pathname: &str,
        material: Option<String>,
        overlaps: Option<DictLike<'py>>,
        position: Option<f64x3>,
        role: Option<Strings>,
        rotation: Option<Rotation>,
        shape: Option<DictLike<'py>>,
        subtract: Option<Strings>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let mut builder = slf.borrow_mut();
        let volume = builder.find_mut(pathname)?;
        if let Some(material) = material {
            volume.material = material;
        }
        if let Some(overlaps) = overlaps {
            let tag = Tag::new("", "overlaps", None);
            volume.overlaps = volume::Volume::flatten_overlaps(
                &tag,
                &overlaps,
                volume.volumes.as_slice()
            )?;
        }
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation.into_mat());
        }
        if let Some(role) = role {
            volume.roles = role.into_vec().as_slice().try_into()
                .map_err(|why: String| {
                    Error::new(ValueError).what("role").why(&why).to_err()
                })?;
        }
        if let Some(shape) = shape {
            let tag = Tag::new("", "shape", None);
            volume.shape = volume::Shape::try_from_dict(&tag, &shape)?;
        }
        if let Some(subtract) = subtract {
            volume.subtract = subtract.into_vec();
        }
        Ok(slf)
    }

    /// Relocate a volume within the geometry definition.
    fn r#move<'py>(
        slf: Bound<'py, GeometryBuilder>,
        source: &str,
        destination: &str,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let (src_mother, src_name) = source.rsplit_once('.')
            .ok_or_else(|| {
                let why = format!("cannot relocate root volume '{}'", source);
                let err: PyErr = Error::new(ValueError)
                    .what("geometry operation").why(&why).into();
                err
            })?;
        let (dst_mother, dst_name) = destination.rsplit_once('.')
            .ok_or_else(|| {
                let why = format!("cannot relocate as root volume '{}'", source);
                let err: PyErr = Error::new(ValueError)
                    .what("geometry operation").why(&why).into();
                err
            })?;
        if dst_name != src_name {
            volume::Volume::check(&dst_name)
                .map_err(|why| {
                    let what = format!("name '{}'", dst_name);
                    Error::new(ValueError).what(&what).why(why).to_err()
                })?;
        }
        let mut builder = slf.borrow_mut();
        if dst_mother == src_mother {
            let volume = builder.find_mut(source)?;
            if volume.name != dst_name {
                volume.name = dst_name.to_string();
            }
            return Ok(slf);
        }
        if !builder.contains(dst_mother) {
            let why = format!("unknown '{}' volume", dst_mother);
            let err: PyErr = Error::new(ValueError)
                .what("geometry operation").why(&why).into();
            return Err(err);
        }
        let mut volume = {
            let src_mother = builder.find_mut(src_mother)?;
            let mut volume = None;
            for i in 0..src_mother.volumes.len() {
                let v = &src_mother.volumes[i];
                if v.name == src_name {
                    volume = Some(src_mother.volumes.remove(i));
                    break;
                }
            }
            let volume = volume
                .ok_or_else(|| {
                    let why = format!("unknown '{}' volume", source);
                    let err: PyErr = Error::new(ValueError)
                        .what("geometry operation").why(&why).into();
                    err
                })?;
            volume
        };

        let dst_mother = builder.find_mut(dst_mother)?;
        for v in dst_mother.volumes.iter_mut() {
            if v.name == dst_name {
                *v = volume;
                return Ok(slf)
            }
        }
        if volume.name != dst_name {
            volume.name = dst_name.to_string();
        }
        dst_mother.volumes.push(volume);
        Ok(slf)
    }

    /// (Re)place a definition of a geometry volume.
    fn place<'py>(
        slf: Bound<'py, GeometryBuilder>,
        definition: DictLike,
        mother: Option<&str>,
        position: Option<f64x3>,
        rotation: Option<Rotation>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let GeometryDefinition { mut volume, materials } =
            GeometryDefinition::new(definition, None)?;
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation.into_mat());
        }
        let mut builder = slf.borrow_mut();
        let mother = match mother {
            None => &mut builder.definition.volume,
            Some(mother) => builder.find_mut(mother)?,
        };
        match mother.volumes.iter_mut().find(|v| v.name() == volume.name()) {
            None => mother.volumes.push(*volume),
            Some(v) => *v = *volume,
        }
        if let Some(materials) = materials {
            match &mut builder.definition.materials {
                None => builder.definition.materials = Some(materials),
                Some(m) => m.extend(materials),
            }
        }
        Ok(slf)
    }

    fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let mut buffer = Vec::new();
        let mut serializer = Serializer::new(&mut buffer);
        self.serialize(&mut serializer)
            .map_err(|err| {
                let why = format!("{}", err);
                Error::new(Exception)
                    .what("serialisation")
                    .why(&why)
                    .to_err()
            })?;
        Ok(PyBytes::new_bound(py, &buffer))
    }

    fn __setstate__(&mut self, state: &Bound<PyBytes>) -> PyResult<()> {
        let mut deserializer = Deserializer::new(state.as_bytes());
        *self = Deserialize::deserialize(&mut deserializer)
            .map_err(|err| {
                let why = format!("{}", err);
                Error::new(Exception)
                    .what("serialisation")
                    .why(&why)
                    .to_err()
            })?;
        Ok(())
    }
}

impl GeometryBuilder {
    fn contains(&self, path: &str) -> bool {
        let mut names = path.split(".");
        let volume = match names.next() {
            None => None,
            Some(name) => {
                let volume = self.definition.volume.as_ref();
                if name != volume.name() {
                    None
                } else {
                    let mut volume = Some(volume);
                    for name in names {
                        volume = volume.unwrap().volumes.iter().find(|v| v.name() == name);
                        if volume.is_none() {
                            break
                        }
                    }
                    volume
                }
            },
        };
        volume.is_some()
    }

    fn find_mut<'a>(&'a mut self, path: &str) -> PyResult<&'a mut volume::Volume> {
        let mut names = path.split(".");
        let volume = match names.next() {
            None => None,
            Some(name) => {
                let volume = self.definition.volume.as_mut();
                if name != volume.name() {
                    None
                } else {
                    let mut volume = Some(volume);
                    for name in names {
                        volume = volume.unwrap().volumes.iter_mut().find(|v| v.name() == name);
                        if volume.is_none() {
                            break
                        }
                    }
                    volume
                }
            },
        };
        volume.ok_or_else(|| {
            let why = format!("unknown '{}' volume", path);
            Error::new(ValueError).what("geometry operation").why(&why).to_err()
        })
    }
}

// ===============================================================================================
//
// Geometry definition.
//
// This is a thin wrapper collecting the root volume description and some optional material
// definitions.
//
// ===============================================================================================

#[derive(Default, Deserialize, Serialize)]
struct GeometryDefinition {
    volume: Box<volume::Volume>,
    materials: Option<MaterialsDefinition>,
}

impl GeometryDefinition {
    pub fn new(definition: DictLike, file: Option<&Path>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<2> = Extractor::new([
            Property::optional_any("materials"),
            Property::optional_any("meshes"),
        ]);

        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let tag = Tag::new("geometry", "", file);
        let [materials, meshes] = EXTRACTOR.extract(
            &tag, &definition, Some(&mut remainder)
        )?;

        let (_, new_file) = definition.resolve(file)?;
        if let Some(ref new_file) = new_file {
            if let Some(file) = file {
                if new_file == file {
                    let why = format!("recursion: '{}'", file.display());
                    let err = Error::new(ValueError).what("geometry").why(&why);
                    return Err(err.into());
                }
            }
        }
        let file = new_file;

        if remainder.len() != 1 {
            let why = format!("expected 1 root volume, found {}", remainder.len());
            let err = Error::new(ValueError).what("geometry").why(&why);
            return Err(err.into());
        }

        let meshes: Option<Bound<PyAny>> = meshes.into();
        if let Some(meshes) = meshes {
            let tag = Tag::new("", "", file.as_deref());
            mesh::NamedMesh::try_from_any(&tag, &meshes)?;
        }

        let (name, volume) = remainder.iter().next().unwrap();
        let tag = Tag::new("", name.as_str(), file.as_deref());
        let volume = volume::Volume::try_from_any(&tag, &volume)?;
        let volume = Box::new(volume);

        let materials: Option<Bound<PyAny>> = materials.into();
        let materials: PyResult<Option<MaterialsDefinition>> = materials
            .map(|materials| {
                let tag = Tag::new("", "", file.as_deref());
                MaterialsDefinition::try_from_any(&tag, &materials)
            })
            .transpose();
        let materials = materials?;

        let definition = Self { volume, materials };
        Ok(definition)
    }
}

// ===============================================================================================
//
// Volume proxy.
//
// ===============================================================================================

/// A volume of a Monte Carlo geometry.
#[pyclass(frozen, module="calzone")]
#[derive(Clone)]
pub struct Volume {
    pub(crate) volume: SharedPtr<ffi::VolumeBorrow>,
    /// The volume absolute pathname.
    #[pyo3(get)]
    path: String,
    /// The volume constitutive material.
    #[pyo3(get)]
    material: String,
    /// The volume shape (according to Geant4).
    #[pyo3(get)]
    pub(crate) solid: String,
    /// The mother of this volume, if any (i.e. directly containing this volume).
    #[pyo3(get)]
    mother: Option<String>,
    daughters: Vec<String>,

    pub(crate) properties: SolidProperties,
    geometry: SharedPtr<ffi::GeometryBorrow>,
}

#[derive(Clone)]
pub struct SolidProperties {
    pub has_cubic_volume: bool,
    pub has_exclusive_volume: bool,
    pub has_surface_area: bool,
    pub has_surface_generation: bool,
}

unsafe impl Send for ffi::VolumeBorrow {}
unsafe impl Sync for ffi::VolumeBorrow {}

#[pymethods]
impl Volume {
    fn __eq__(&self, other: &Self) -> bool {
        self.volume.eq(&other.volume)
    }

    /// Daughter volume(s), if any (i.e. included insides).
    #[getter]
    fn get_daughters<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        PyTuple::new_bound(py, &self.daughters)
    }

    /// The volume name.
    #[getter]
    fn get_name<'py>(&self) -> &str {
        match self.path.rsplit_once('.') {
            None => self.path.as_str(),
            Some((_, name)) => name,
        }
    }

    /// The volume surface area, in cm\ :sup:`2`.
    #[getter]
    fn get_surface_area<'py>(&self) -> PyResult<f64> {
        if self.properties.has_surface_area {
            Ok(self.volume.compute_surface())
        } else {
            let why = format!("not implemented for '{}'", self.solid);
            let err = Error::new(ValueError)
                .what("'surface' attribute")
                .why(&why);
            Err(err.into())
        }
    }

    /// The volume role(s), if any.
    #[getter]
    fn get_role(&self, py: Python) -> PyResult<PyObject> {
        let roles = self.volume.get_roles();
        let roles: Vec<String> = roles.into();
        let role = match roles.len() {
            0 => py.None(),
            1 => (&roles[0]).into_py(py),
            _ => PyTuple::new_bound(py, roles.iter()).into_any().unbind(),
        };
        Ok(role)
    }

    #[setter]
    fn set_role(&self, role: Option<Strings>) -> PyResult<()> {
        let roles = role.map(|role| role.into_vec()).unwrap_or(Vec::new());
        if roles.is_empty() {
            self.volume.clear_roles()
        } else {
            let roles: ffi::Roles = roles.as_slice().try_into()
                .map_err(|why: String| {
                    Error::new(ValueError).what("role").why(&why).to_err()
                })?;
            self.volume.set_roles(roles)
        };
        Ok(())
    }

    /// Return the volume's Axis-Aligned Bounding-Box (AABB).
    #[pyo3(name = "aabb")]
    fn compute_aabb(
        &self,
        py: Python,
        frame: Option<&str>
    ) -> PyResult<PyObject> {
        let frame = frame.unwrap_or("");
        let bbox = self.volume.compute_box(frame);
        if let Some(why) = ffi::get_error().value() {
            let err = Error::new(ValueError).what("box operation").why(why);
            return Err(err.into());
        }

        let result = PyArray::<f64>::empty(py, &[2, 3]).unwrap();
        result.set(0, bbox[0]).unwrap();
        result.set(1, bbox[2]).unwrap();
        result.set(2, bbox[4]).unwrap();
        result.set(3, bbox[1]).unwrap();
        result.set(4, bbox[3]).unwrap();
        result.set(5, bbox[5]).unwrap();
        result.readonly();
        Ok(result.as_any().into_py(py))
    }

    /// Return the coordinates of the volume origin.
    #[pyo3(name = "origin")]
    fn compute_origin(&self, frame: Option<&str>) -> PyResult<f64x3> {
        let frame = frame.unwrap_or("");
        let origin = self.volume.compute_origin(frame);
        if let Some(why) = ffi::get_error().value() {
            let err = Error::new(ValueError).what("origin operation").why(why);
            return Err(err.into());
        }
        Ok((&origin).into())
    }

    /// Return the cubic volume of this volume.
    #[pyo3(name = "volume")]
    fn compute_volume(&self, include_daughters: Option<bool>) -> PyResult<f64> {
        if !self.properties.has_cubic_volume {
            let why = format!("not implemented for '{}'", self.solid);
            let err = Error::new(NotImplementedError).what("volume operation").why(&why);
            return Err(err.into());
        }

        let include_daughters = include_daughters.unwrap_or(false);
        if !include_daughters && !self.properties.has_exclusive_volume {
            let why = "not implemented for daughter volume(s)";
            let err = Error::new(NotImplementedError).what("volume operation").why(why);
            return Err(err.into());
        }

        let volume = self.volume.compute_volume(include_daughters);
        Ok(volume)
    }

    /// Display the volume.
    #[pyo3(signature=(data=None,/))]
    fn display<'py>(
        slf: &Bound<'py, Self>,
        data: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<()> {
        let py = slf.py();
        let display_func = py.import_bound("calzone_display")
            .and_then(|m| m.getattr("display"))?;
        let args = (slf,);
        match data {
            Some(data) => {
                let kwargs = [("data", data),].to_object(py);
                let kwargs = PyDict::from_sequence_bound(&kwargs.bind(py))?;
                display_func.call(args, Some(&kwargs))?;
            },
            None => {
                display_func.call1(args)?;
            },
        }
        Ok(())
    }

    /// Compute point(s) local coordinates.
    #[pyo3(signature=(points,/))]
    fn local_coordinates<'py>(
        &self,
        py: Python,
        points: &PyArray<f64>,
    ) -> PyResult<PyObject> {
        let shape = points.shape();
        let dim = shape.last().unwrap_or(&0);
        if dim != &3 {
            let why = format!("expected a 3d-array, found a {}d-array", dim);
            let err = Error::new(TypeError).what("points").why(&why);
            return Err(err.to_err())
        }

        let transform = self.volume.compute_transform("");
        let result = if shape.len() == 1 {
            let point = [
                points.get(0)?,
                points.get(1)?,
                points.get(2)?,
            ];
            let v = self.volume.local_coordinates(&point, &transform);
            v.into_py(py)
        } else {
            let result = PyArray::<f64>::empty(py, &shape)?;
            for i in 0..(result.size() / 3) {
                let point = [
                    points.get(3 * i + 0)?,
                    points.get(3 * i + 1)?,
                    points.get(3 * i + 2)?,
                ];
                let v = self.volume.local_coordinates(&point, &transform);
                result.set(3 * i + 0, v[0])?;
                result.set(3 * i + 1, v[1])?;
                result.set(3 * i + 2, v[2])?;
            }
            result.into_any().unbind()
        };

        Ok(result)
    }

    /// Return the side of elements w.r.t. this volume.
    #[pyo3(signature=(elements, /, *, include_daughters=None))]
    fn side<'py>(
        &self,
        elements: &Bound<'py, PyAny>,
        include_daughters: Option<bool>
    ) -> PyResult<PyObject> {
        let py = elements.py();
        let points: &PyArray<f64> = elements.get_item("position")
            .unwrap_or(elements.clone())
            .extract()
            .map_err(|_| {
                let why = format!("expected 'particles' or 'positions', found '{:?}'", elements);
                Error::new(TypeError).what("elements").why(&why).to_err()
            })?;
        let mut shape = points.shape();
        let dim = shape.pop().unwrap_or(0);
        if dim != 3 {
            let why = format!("expected 3-d elements, found {}-d values", dim);
            let err = Error::new(TypeError).what("elements").why(&why);
            return Err(err.to_err());
        }
        let include_daughters = include_daughters.unwrap_or(false);
        let transform = self.volume.compute_transform("");
        let result: PyObject = if shape.len() == 0 {
            let point = [
                points.get(0)?,
                points.get(1)?,
                points.get(2)?,
            ];
            let v: i32 = self.volume.inside(&point, &transform, include_daughters).into();
            v.into_py(py)
        } else {
            let result = PyArray::<i32>::empty(py, &shape)?;
            for i in 0..result.size() {
                let point = [
                    points.get(3 * i + 0)?,
                    points.get(3 * i + 1)?,
                    points.get(3 * i + 2)?,
                ];
                let v: i32 = self.volume.inside(&point, &transform, include_daughters).into();
                result.set(i, v)?;
            }
            result.into_any().unbind()
        };
        Ok(result)
    }

    /// Return a binary representation of the volume.
    #[pyo3(signature=(*, include_daughters=None))]
    #[pyo3(text_signature="(*, include_daughters=True)")]
    fn to_bytes<'py>(
        &self,
        py: Python<'py>,
        include_daughters: Option<bool>
    ) -> PyResult<Bound<'py, PyBytes>> {
        fn get_info(
            name: &str,
            solid: &str,
            material: String,
            volume: &ffi::VolumeBorrow
        ) -> bytes::VolumeInfo {
            let solid = match solid {
                "G4Box" => bytes::SolidInfo::Box(volume.describe_box()),
                "G4Orb" => bytes::SolidInfo::Orb(volume.describe_orb()),
                "G4Sphere" => bytes::SolidInfo::Sphere(volume.describe_sphere()),
                "G4TessellatedSolid" => {
                    let data: Vec<f32> = volume.describe_tessellated_solid().as_ref().into();
                    bytes::SolidInfo::Mesh(data)
                },
                "G4Tubs" => bytes::SolidInfo::Tubs(volume.describe_tubs()),
                "Mesh" => {
                    let data: Vec<f32> = volume.describe_mesh().as_ref().into();
                    bytes::SolidInfo::Mesh(data)
                },
                _ => unreachable!("unexpected solid '{}'", solid),
            };
            let transform = volume.describe_transform();
            bytes::VolumeInfo {
                name: name.to_string(),
                solid,
                material,
                transform,
                daughters: Vec::new()
            }
        }

        let mut volumes = get_info(
            self.get_name(),
            self.solid.as_str(),
            self.material.clone(),
            &self.volume
        );

        if include_daughters.unwrap_or(true) {
            fn add(
                volume: &mut bytes::VolumeInfo,
                daughters: &[String],
                geometry: &ffi::GeometryBorrow
            ) { // recursively.
                for daughter in daughters {
                    let daughter_volume = geometry.borrow_volume(daughter);
                    let ffi::VolumeInfo { path, material, solid, mut daughters, .. } =
                        daughter_volume.describe();
                    let name = match path.rsplit_once('.') {
                        None => path.as_str(),
                        Some((_, name)) => name,
                    };
                    let daughters: Vec<_> = daughters.drain(..)
                        .map(|ffi::DaughterInfo { path, .. }| path)
                        .collect();
                    let mut daughter_volume = get_info(
                        name,
                        solid.as_str(),
                        material,
                        &daughter_volume
                    );
                    add(&mut daughter_volume, &daughters, geometry);
                    volume.daughters.push(daughter_volume);
                }
            }
            add(&mut volumes, &self.daughters, &self.geometry);
        }

        type Materials = HashMap::<String, bytes::MaterialInfo>;

        fn get_material(
            volume: &bytes::VolumeInfo,
            materials: &mut Materials,
            geometry: &ffi::GeometryBorrow,
        ) {
            materials
                .entry(volume.material.clone())
                .or_insert_with(|| {
                    let mut mixture = ffi::describe_material(volume.material.as_str());
                    let state = materials::State::try_from(mixture.properties.state)
                        .map_or_else(
                            |err| err.to_owned(),
                            |state| state.to_str().to_owned(),
                        );
                    let composition: Vec<_> = mixture.components.drain(..)
                        .map(|component| (component.name, component.weight))
                        .collect();
                    bytes::MaterialInfo {
                        density: mixture.properties.density,
                        state: state.to_string(),
                        composition,
                    }
                });
            for daughter in volume.daughters.iter() {
                get_material(daughter, materials, geometry);
            }
        }

        let mut materials = Materials::new();
        get_material(&volumes, &mut materials, &self.geometry);

        let geometry = bytes::GeometryInfo { volumes, materials };

        let mut buffer = Vec::new();
        let mut serializer = Serializer::new(&mut buffer);
        geometry.serialize(&mut serializer)
            .map_err(|err| {
                let why = format!("{}", err);
                Error::new(Exception)
                    .what("serialisation")
                    .why(&why)
                    .to_err()
            })?;
        Ok(PyBytes::new_bound(py, &buffer))
    }
}

impl From<ffi::EInside> for i32 {
    fn from(value: ffi::EInside) -> Self {
        match value {
            ffi::EInside::kInside => 1,
            ffi::EInside::kSurface => 0,
            ffi::EInside::kOutside => -1,
            _ => 0,
        }
    }
}

impl Volume {
    pub fn new(
        geometry: &SharedPtr<ffi::GeometryBorrow>,
        name: &str,
        exact: bool
    ) -> PyResult<Self> {
        let volume = match exact {
            true => geometry.borrow_volume(name),
            false => geometry.find_volume(name),
        };
        if let Some(msg) = ffi::get_error().value() {
            let err = Error::new(IndexError).what("volume").why(msg);
            return Err(err.into())
        }
        let ffi::VolumeInfo { path, material, solid, mother, mut daughters } =
            volume.describe();
        let mother = if mother.is_empty() {
            None
        } else {
            Some(mother)
        };

        let get_properties = |solid: &str| -> SolidProperties {
            match solid {
                "G4Box" | "G4DisplacedSolid" | "G4Orb" | "G4Sphere" | "G4Tubs" => {
                    SolidProperties::everything()
                },
                "G4TessellatedSolid" | "Mesh" => {
                    SolidProperties {
                        has_cubic_volume: false,
                        has_exclusive_volume: false,
                        has_surface_area: true,
                        has_surface_generation: true,
                    }
                },
                _ => SolidProperties::nothing(),
            }
        };

        let mut properties = get_properties(solid.as_str());
        properties.has_exclusive_volume = properties.has_cubic_volume && daughters.iter()
            .map(|ffi::DaughterInfo { solid, .. }| {
                let properties = get_properties(solid.as_str());
                properties.has_cubic_volume
            })
            .fold(true, |acc, v| acc && v);
        let daughters: Vec<_> = daughters.drain(..)
            .map(|ffi::DaughterInfo { path, .. }| path)
            .collect();

        let volume = Volume {
            volume,
            path,
            material,
            solid,
            mother,
            daughters,
            properties,
            geometry: geometry.clone(),
        };
        Ok(volume)
    }
}

impl SolidProperties {
    pub const fn everything() -> Self {
        Self {
            has_cubic_volume: true,
            has_exclusive_volume: true,
            has_surface_area: true,
            has_surface_generation: true,
        }
    }

    pub const fn nothing() -> Self {
        Self {
            has_cubic_volume: false,
            has_exclusive_volume: false,
            has_surface_area: false,
            has_surface_generation: false,
        }
    }
}


// ===============================================================================================
//
// Define & describe interface.
//
// ===============================================================================================

/// Define materials and meshes.
#[pyfunction]
#[pyo3(signature=(*, materials=None, meshes=None))]
pub fn define(materials: Option<DictLike>, meshes: Option<DictLike>) -> PyResult<()> {
    if let Some(materials) = materials {
        let tag = Tag::new("", "materials", None);
        let materials = MaterialsDefinition::try_from_dict(&tag, &materials)?;
        materials.build()?;
    }
    if let Some(meshes) = meshes {
        let tag = Tag::new("", "meshes", None);
        mesh::NamedMesh::try_from_dict(&tag, &meshes)?;
    }
    Ok(())
}

/// Describe a material or a mesh.
#[pyfunction]
#[pyo3(signature=(*, material=None, mesh=None))]
pub fn describe(py: Python, material: Option<&str>, mesh: Option<&str>) -> PyResult<PyObject> {
    let material = material
        .map(|material| ffi::describe_material(material).to_object(py));
    let mesh = mesh
        .map(|mesh| match mesh::NamedMesh::describe(mesh) {
            Some(mesh) => mesh.to_object(py),
            None => py.None(),
        });

    let result = match material {
        Some(material) => match mesh {
            Some(mesh) => Namespace::new(py, &[
                ("material", material),
                ("mesh", mesh),
            ])?.unbind(),
            None => material,
        },
        None => match mesh {
            Some(mesh) => mesh,
            None => py.None(),
        }
    };
    Ok(result)
}
