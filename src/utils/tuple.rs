use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::sync::GILOnceCell;


pub struct NamedTuple<const N: usize> {
    name: &'static str,
    fields: [&'static str; N],
    object: GILOnceCell<PyObject>,
}

impl<const N: usize> NamedTuple<N> {
    pub const fn new(name: &'static str, fields: [&'static str; N]) -> Self {
        let object = GILOnceCell::new();
        Self { name, fields, object }
    }

    pub fn instance<'py>(
        &self,
        py: Python<'py>,
        args: impl IntoPy<Py<PyTuple>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let tuple = self.object.get_or_try_init(py, || py.import_bound("collections")
            .and_then(|m| m.getattr("namedtuple"))
            .and_then(|m| m.call1((self.name, self.fields)))
            .map(|m| m.unbind())
        )?.bind(py);
        let tuple = tuple.call1(args)?;
        Ok(tuple)
    }
}
