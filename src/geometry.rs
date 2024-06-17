use crate::utils::extract::{extract, Tag, TryFromBound};
use crate::utils::float::f64x3;
use crate::utils::io::DictLike;
use crate::utils::numpy::PyArray;
use cxx::SharedPtr;
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
    fn new(arg: DictLike) -> PyResult<Self> {
        // XXX materials (dedicated Geometry entry?)
        // XXX from GDML (manage memory by diffing G4SolidStore etc.).
        // XXX Displaced solids to tessellated DEMs?
        let (dict, file) = arg.resolve()?;
        if dict.len() != 1 {
            let msg = format!("bad geometry (expected 1 top volume, found {})", dict.len());
            return Err(PyValueError::new_err(msg));
        }
        let (name, definition) = dict.iter().next().unwrap();
        let name: String = extract(&name)
            .or("bad geometry")?;
        let file = file.as_ref().map(|f| f.as_path());
        let tag = Tag::new("", name.as_str(), file);
        let volume = volume::Volume::try_from_any(&tag, &definition)?;
        let geometry = ffi::create_geometry(Box::new(volume));
        if geometry.is_null() {
            ffi::get_error().to_result()?;
            unreachable!()
        }
        let geometry = Self (geometry);
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
