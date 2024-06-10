use crate::utils::extract::{extract, Tag, TryFromBound};
use crate::utils::io::DictLike;
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use super::cxx::ffi;
use temp_dir::TempDir;

mod goupil;
pub mod volume;


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
        let dict = arg.to_dict()?;
        if dict.len() != 1 {
            let msg = format!("bad geometry (expected one top volume, found {})", dict.len());
            return Err(PyValueError::new_err(msg));
        }
        let (name, definition) = dict.iter().next().unwrap();
        let name: String = extract(&name)
            .or("bad geometry")?;
        let volume = volume::Volume::try_from_any(&Tag::new("", name.as_str()), &definition)?;
        let geometry = ffi::create_geometry(Box::new(volume));
        if geometry.is_null() {
            ffi::get_error().to_result()?;
            unreachable!()
        }
        let geometry = Self (geometry);
        Ok(geometry)
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
