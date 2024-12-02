use process_path::get_dylib_path;
use pyo3::prelude::*;
use pyo3::exceptions::PySystemError;
use pyo3::sync::GILOnceCell;
use std::env;

mod cxx;
mod geometry;
mod simulation;
mod utils;


static FILE: GILOnceCell<String> = GILOnceCell::new();

// Fetch the Geant4 version (exported by the build script).
include!(concat!(env!("OUT_DIR"), "/geant4_version.rs"));


/// CALorimeter ZONE (CalZone)
#[pymodule]
fn calzone(module: &Bound<PyModule>) -> PyResult<()> {

    // Set __file__.
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

    // Set data path.
    const DATA_KEY: &str = "GEANT4_DATA_DIR";
    if let Err(_) = env::var(DATA_KEY) {
        env::set_var(DATA_KEY, utils::data::default_path());
    }

    // Initialise interfaces.
    utils::error::initialise();
    utils::numpy::initialise(py)?;
    utils::units::initialise(py);

    // Register class object(s).
    module.add_class::<geometry::Geometry>()?;
    module.add_class::<geometry::GeometryBuilder>()?;
    module.add_class::<geometry::Map>()?;
    module.add_class::<simulation::Physics>()?;
    module.add_class::<simulation::Random>()?;
    module.add_class::<simulation::Simulation>()?;
    module.add_class::<simulation::source::ParticlesGenerator>()?;
    module.add_class::<geometry::Volume>()?;

    // Register exception(s).
    module.add("Geant4Exception", py.get_type_bound::<utils::error::Geant4Exception>())?;

    // Register function(s).
    module.add_function(wrap_pyfunction!(utils::data::download, module)?)?;
    module.add_function(wrap_pyfunction!(geometry::define, module)?)?;
    module.add_function(wrap_pyfunction!(geometry::describe, module)?)?;
    module.add_function(wrap_pyfunction!(simulation::source::particles, module)?)?;

    // Register constant(s).
    module.add("GEANT4_VERSION", GEANT4_VERSION)?;

    // Register Geant4 finalisation.
    let dropper = wrap_pyfunction!(simulation::drop_simulation, module)?;
    py.import_bound("atexit")?
      .call_method1("register", (dropper,))?;

    Ok(())
}
