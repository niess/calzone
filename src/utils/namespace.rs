use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};


pub struct Namespace;

impl Namespace {
    pub fn new<'py, T>(
        py: Python<'py>,
        kwargs: &[(&str, T)]
    ) -> PyResult<Bound<'py, PyAny>>
    where
        T: ToPyObject,
    {
        let kwargs = PyTuple::new_bound(py, kwargs);
        let kwargs = PyDict::from_sequence_bound(kwargs.as_any())?;
        let namespace = py.import_bound("types")
            .and_then(|x| x.getattr("SimpleNamespace"))
            .and_then(|x| x.call((), Some(&kwargs)))?;
        Ok(namespace)
    }
}
