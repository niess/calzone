use crate::materials::gate::load_gate_db;
use pyo3::prelude::*;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::types::{PyDict, PyString};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};


// ===============================================================================================
//
// Dict config loader.
//
// ===============================================================================================

pub fn load_toml<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
    let content = std::fs::read_to_string(path)?;
    let toml = py.import_bound("tomllib")
        .or_else(|_| py.import_bound("tomli"))?;
    let loads = toml.getattr("loads")?;
    let content = loads.call1((content,))?;
    let dict: Bound<PyDict> = content.extract()?;
    Ok(dict)
}


// ===============================================================================================
//
// Generic dict argument.
//
// ===============================================================================================

#[derive(FromPyObject)]
pub enum DictLike<'py> {
    #[pyo3(transparent, annotation = "dict")]
    Dict(Bound<'py, PyDict>),
    #[pyo3(transparent, annotation = "str")]
    String(Bound<'py, PyString>),
}

impl<'py> DictLike<'py> {
    pub fn resolve<'a>(&'a self) -> PyResult<(Cow<'a, Bound<'py, PyDict>>, Option<PathBuf>)> {
        let result = match &self {
            Self::Dict(dict) => (Cow::Borrowed(dict), None),
            Self::String(path) => {
                let py = path.py();
                let path = path.to_cow()?;
                let path = Path::new(path.deref());
                let dict = match path.extension().and_then(OsStr::to_str) {
                    Some("db") => load_gate_db(py, &path),
                    Some("toml") => load_toml(py, &path),
                    _ => Err(PyNotImplementedError::new_err("")),
                }?;
                (Cow::Owned(dict), Some(path.to_path_buf()))
            },
        };
        Ok(result)
    }
}


// ===============================================================================================
//
// Data loaders.
//
// ===============================================================================================

pub fn load_stl<'py>(path: &Path) -> Result<Vec<f32>, String> {
    let bad_format = || format!("{}: bad STL format)", path.display());

    let bytes = std::fs::read(path)
        .map_err(|_| format!("could not read '{}'", path.display()))?;
    let data = bytes.get(80..84)
        .ok_or_else(bad_format)?;
    let facets: usize = u32::from_le_bytes(data.try_into().unwrap()).try_into().unwrap();
    let mut values = Vec::<f32>::with_capacity(9 * facets);
    for i in 0..facets {
        let start: usize = (84 + 50 * i).try_into().unwrap();
        let data = bytes.get(start..(start + 50))
            .ok_or_else(bad_format)?;
        for j in 0..3 {
            let start = 12 * (j + 1);
            for k in 0..3 {
                let start = start + 4 * k;
                let data = &data[start..(start + 4)];
                let v = f32::from_le_bytes(data.try_into().unwrap());
                values.push(v);
            }
        }
    }
    Ok(values)
}
