use crate::utils::extract::{Tag, TryFromBound};
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::cxx::ffi;
use temp_dir::TempDir;

mod goupil;
mod volume;


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
    #[pyo3(signature = (**kwargs,))]
    fn new(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        if let Some(kwargs) = kwargs {
            let volumes = Vec::<volume::Volume>::try_from_dict(&Tag::empty(), kwargs)?;
        }

        let geometry = ffi::create_geometry();
        let geometry = Self (geometry);

        Ok(geometry)
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
