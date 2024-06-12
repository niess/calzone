use process_path::get_dylib_path;

use pyo3::prelude::*;
use pyo3::exceptions::PySystemError;
use pyo3::sync::GILOnceCell;

mod cxx;
mod geometry;
mod materials;
mod utils;


static FILE: GILOnceCell<String> = GILOnceCell::new();


use std::path::Path;
use pyo3::exceptions::PyValueError;

#[pyfunction]
pub fn load_geotiff(py: Python, path: &str) -> PyResult<()> {
    let path = Path::new(path);
    utils::io::load_geotiff(py, path)
        .map_err(|err| PyValueError::new_err(err))?;
    Ok(())
}


#[pymodule]
fn calzone(module: &Bound<PyModule>) -> PyResult<()> {

    // Set module path.
    let py = module.py();
    {
        let filename = match get_dylib_path() {
            Some(path) => path
                            .to_string_lossy()
                            .to_string(),
            None => return Err(PySystemError::new_err("could not resolve module path")),
        };
        FILE
            .set(py, filename)
            .unwrap();
    }

    // Initialise interfaces.
    utils::error::initialise();
    utils::numpy::initialise(py)?;
    utils::units::initialise(py);

    // Register class object(s).
    module.add_class::<geometry::Geometry>()?;

    // Register function(s).
    module.add_function(wrap_pyfunction!(materials::elements, module)?)?;
    module.add_function(wrap_pyfunction!(materials::load, module)?)?;
    module.add_function(wrap_pyfunction!(materials::molecules, module)?)?;
    module.add_function(wrap_pyfunction!(materials::mixtures, module)?)?;

    module.add_function(wrap_pyfunction!(load_geotiff, module)?)?; // XXX debug.

    Ok(())
}
