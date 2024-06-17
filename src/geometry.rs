use crate::materials::MaterialsDefinition;
use crate::utils::extract::{Extractor, Property, Tag, TryFromBound};
use crate::utils::float::{f64x3, f64x3x3};
use crate::utils::io::DictLike;
use crate::utils::numpy::PyArray;
use cxx::SharedPtr;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::types::PyTuple;
use super::cxx::ffi;
use temp_dir::TempDir;

mod goupil;
mod map;
pub mod volume;

pub use map:: Map;


// ===============================================================================================
//
// Geometry interface.
//
// ===============================================================================================

#[pyclass(frozen, module="calzone")]
pub struct Geometry (SharedPtr<ffi::GeometryBorrow>);

unsafe impl Send for ffi::GeometryBorrow {}
unsafe impl Sync for ffi::GeometryBorrow {}

#[pymethods]
impl Geometry {
    #[new]
    fn new(volume: DictLike) -> PyResult<Self> {
        // XXX from GDML (manage memory by diffing G4SolidStore etc.).
        let builder = GeometryBuilder::new(volume)?;
        let geometry = builder.build()?;
        Ok(geometry)
    }

    fn __getitem__(&self, volume: &str) -> PyResult<Volume> {
        let ffi::VolumeInfo { material, solid, mother, daughters } =
            self.0.describe_volume(volume);
        if let Some(msg) = ffi::get_error().value() {
            let msg = format!("bad volume ({})", msg);
            return Err(PyIndexError::new_err(msg));
        }
        let mother = if mother.is_empty() {
            None
        } else {
            Some(mother)
        };
        let volume = Volume {
            geometry: self.0.clone(),
            name: volume.to_string(),
            material,
            solid,
            mother,
            daughters,
        };
        Ok(volume)
    }

    fn check(&self, resolution: Option<i32>) -> PyResult<()> {
        let resolution = resolution.unwrap_or(1000);
        self.0
            .check(resolution)
            .to_result()?;
        Ok(())
    }

    fn dump(&self, path: Option<&str>) -> PyResult<()> {
        let tmp = TempDir::new()?;
        let path = path.unwrap_or("geometry.gdml");
        let tmp_path = tmp
            .child("geometry.gdml")
            .display()
            .to_string();
        self.0
            .dump(tmp_path.as_str())
            .to_result()?;
        std::fs::copy(&tmp_path, path)?;
        Ok(())
    }

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
}

// ===============================================================================================
//
// Builder interface.
//
// ===============================================================================================

#[pyclass]
pub struct GeometryBuilder (GeometryDefinition);

#[pymethods]
impl GeometryBuilder {
    #[new]
    fn new(definition: DictLike) -> PyResult<Self> {
        let definition = GeometryDefinition::new(definition)?;
        let builder = Self (definition);
        Ok(builder)
    }

    fn build(&self) -> PyResult<Geometry> {
        // build materials.
        if let Some(materials) = self.0.materials.as_ref() {
            materials.build()?;
        }

        // Build volumes.
        let geometry = ffi::create_geometry(&self.0.volume);
        if geometry.is_null() {
            ffi::get_error().to_result()?;
            unreachable!()
        }
        let geometry = Geometry (geometry);
        Ok(geometry)
    }

    fn delete<'py>(
        slf: Bound<'py, GeometryBuilder>,
        volume: &str,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        if let Some((mother, name)) = volume.rsplit_once('.') {
            let mut builder = slf.borrow_mut();
            let mother = builder.find_mut(mother)?;
            let n = mother.volumes.len();
            mother.volumes.retain(|v| v.name != name);
            if mother.volumes.len() < n {
                return Ok(slf);
            }
        }
        let builder = slf.borrow();
        let msg = if builder.0.volume.name() == volume {
            format!("cannot delete top volume '{}'", volume)
        } else {
            format!("unknown '{}' volume", volume)
        };
        Err(PyValueError::new_err(format!("bad geometry operation ({})", msg)))
    }

    fn modify<'py>(
        slf: Bound<'py, GeometryBuilder>,
        volume: &str,
        name: Option<String>,
        material: Option<String>,
        position: Option<f64x3>,
        rotation: Option<f64x3x3>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let mut builder = slf.borrow_mut();
        let volume = builder.find_mut(volume)?;
        if let Some(name) = name {
            volume::Volume::check(&name)
                .map_err(|msg| {
                    let msg = format!("bad name '{}' ({})", name, msg);
                    PyValueError::new_err(msg)
                })?;
            volume.name = name;
        }
        if let Some(material) = material {
            volume.material = material;
        }
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation);
        }
        Ok(slf)
    }

    fn place<'py>(
        slf: Bound<'py, GeometryBuilder>,
        volume: DictLike,
        mother: Option<&str>,
        position: Option<f64x3>,
        rotation: Option<f64x3x3>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let GeometryDefinition { mut volume, materials } = GeometryDefinition::new(volume)?;
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation);
        }
        let mut builder = slf.borrow_mut();
        let mother = match mother {
            None => &mut builder.0.volume,
            Some(mother) => builder.find_mut(mother)?,
        };
        match mother.volumes.iter_mut().find(|v| v.name() == volume.name()) {
            None => mother.volumes.push(*volume),
            Some(v) => *v = *volume,
        }
        if let Some(materials) = materials {
            match &mut builder.0.materials {
                None => builder.0.materials = Some(materials),
                Some(m) => m.extend(materials),
            }
        }
        Ok(slf)
    }
}

impl GeometryBuilder {
    fn find_mut<'a>(&'a mut self, path: &str) -> PyResult<&'a mut volume::Volume> {
        let mut names = path.split(".");
        let volume = match names.next() {
            None => None,
            Some(name) => {
                let volume = self.0.volume.as_mut();
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
            let msg = format!("bad geometry operation (unknown '{}' volume)", path);
            PyValueError::new_err(msg)
        })
    }
}

// ===============================================================================================
//
// Geometry definition.
//
// This is a thin wrapper collecting the top volume description and some optional material
// definitions.
//
// ===============================================================================================

struct GeometryDefinition {
    volume: Box<volume::Volume>,
    materials: Option<MaterialsDefinition>,
}

impl GeometryDefinition {
    pub fn new(definition: DictLike) -> PyResult<Self> {
        let (definition, file) = definition.resolve(None)?;

        const EXTRACTOR: Extractor<1> = Extractor::new([
            Property::optional_any("materials"),
        ]);

        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let tag = Tag::new("geometry", "", file.as_deref());
        let [materials] = EXTRACTOR.extract(
            &tag, &definition, Some(&mut remainder)
        )?;

        if remainder.len() != 1 {
            let msg = format!("bad volume(s) (expected 1 top volume, found {})", remainder.len());
            return Err(PyValueError::new_err(msg));
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

#[pyclass(frozen, module="calzone")]
pub struct Volume {
    geometry: SharedPtr<ffi::GeometryBorrow>,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    material: String,
    #[pyo3(get)]
    solid: String,
    #[pyo3(get)]
    mother: Option<String>,
    daughters: Vec<String>,
}

#[pymethods]
impl Volume {
    #[getter]
    fn get_daughters<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        PyTuple::new_bound(py, &self.daughters)
    }

    #[pyo3(name = "r#box")]
    fn compute_box(
        &self,
        py: Python,
        frame: Option<&str>
    ) -> PyResult<PyObject> {
        let frame = frame.unwrap_or("");
        let bbox = self.geometry.compute_box(self.name.as_str(), frame);
        if let Some(msg) = ffi::get_error().value() {
            let msg = format!("bad box operation ({})", msg);
            return Err(PyValueError::new_err(msg));
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

    #[pyo3(name = "origin")]
    fn compute_origin(&self, frame: Option<&str>) -> PyResult<f64x3> {
        let frame = frame.unwrap_or("");
        let origin = self.geometry.compute_origin(self.name.as_str(), frame);
        if let Some(msg) = ffi::get_error().value() {
            let msg = format!("bad origin operation ({})", msg);
            return Err(PyValueError::new_err(msg));
        }
        Ok((&origin).into())
    }
}
