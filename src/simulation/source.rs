use crate::utils::numpy::{PyArray, ShapeArg};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::ffi;


// ===============================================================================================
//
// Source interface.
//
// ===============================================================================================

#[pyfunction]
#[pyo3(signature=(shape=None, **kwargs))]
pub fn primaries(
    py: Python,
    shape: Option<ShapeArg>,
    kwargs: Option<&Bound<PyDict>>
) -> PyResult<PyObject> {
    let shape: Vec<usize> = match shape {
        None => vec![0],
        Some(shape) => shape.into(),
    };
    let array: &PyAny = PyArray::<ffi::Primary>::zeros(py, &shape)?;
    let mut has_direction = false;
    let mut has_energy = false;
    let mut has_pid = false;
    if let Some(kwargs) = kwargs {
        for (key, value) in kwargs.iter() {
            {
                let key: String = key.extract()?;
                match key.as_str() {
                    "direction" => { has_direction = true; },
                    "energy" => { has_energy = true; },
                    "pid" => { has_pid = true; },
                    _ => {},
                }
            }
            array.set_item(key, value)?;
        }
    }
    if !has_direction {
        array.set_item("direction", (0.0, 0.0, 1.0))?;
    }
    if !has_energy {
        array.set_item("energy", 1.0)?;
    }
    if !has_pid {
        array.set_item("pid", 22)?;
    }
    Ok(array.into())
}
