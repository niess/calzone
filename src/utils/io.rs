use crate::materials::gate::load_gate_db;
use pyo3::prelude::*;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::types::{PyDict, PyString};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::Path;


pub fn load_toml<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
    let content = std::fs::read_to_string(path)?;
    let toml = py.import_bound("tomllib")
        .or_else(|_| py.import_bound("tomli"))?;
    let loads = toml.getattr("loads")?;
    let content = loads.call1((content,))?;
    let dict: Bound<PyDict> = content.extract()?;
    Ok(dict)
}

#[derive(FromPyObject)]
pub enum DictLike<'py> {
    #[pyo3(transparent, annotation = "dict")]
    Dict(Bound<'py, PyDict>),
    #[pyo3(transparent, annotation = "str")]
    String(Bound<'py, PyString>),
}

impl<'py> DictLike<'py> {
    pub fn to_dict<'a>(&'a self) -> PyResult<Cow<'a, Bound<'py, PyDict>>> {
        let dict = match &self {
            Self::Dict(dict) => Cow::Borrowed(dict),
            Self::String(path) => {
                let py = path.py();
                let path = path.to_cow()?;
                let path = Path::new(path.deref());
                let dict = match path.extension().and_then(OsStr::to_str) {
                    Some("db") => load_gate_db(py, &path),
                    Some("toml") => load_toml(py, &path),
                    _ => Err(PyNotImplementedError::new_err("")),
                }?;
                Cow::Owned(dict)
            },
        };
        Ok(dict)
    }
}
