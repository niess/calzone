use crate::geometry::Geometry;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::io::DictLike;
use crate::utils::numpy::PyArray;
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::types::PyString;

mod physics;
mod random;
pub mod source;

pub use physics::Physics;
pub use random::Random;
pub use super::cxx::ffi;


// ===============================================================================================
//
// Simulation interface.
//
// ===============================================================================================

#[pyclass(module="calzone")]
pub struct Simulation {
    #[pyo3(get, set)]
    geometry: Option<Py<Geometry>>,
    #[pyo3(get, set)]
    physics: Py<Physics>,
    #[pyo3(get, set)]
    random: Py<Random>,
}

#[pymethods]
impl Simulation {
    #[new]
    fn new<'py>(
        py: Python<'py>,
        geometry: Option<GeometryArg>,
        physics: Option<&Bound<'py, Physics>>,
        random: Option<&Bound<'py, Random>>,
    ) -> PyResult<Self> {
        let geometry = geometry
            .map(|geometry| match geometry {
                GeometryArg::Geometry(geometry) => Ok(geometry.unbind()),
                GeometryArg::String(path) => {
                    let path = DictLike::String(path);
                    let geometry = Geometry::new(path)?;
                    Py::new(py, geometry)
                },
            })
            .transpose()?;
        let physics = physics
            .map(|physics| Ok(physics.clone().unbind()))
            .unwrap_or_else(|| Py::new(py, Physics::new()))?;
        let random = random
            .map(|random| Ok(random.clone().unbind()))
            .unwrap_or_else(|| Py::new(py, Random::new(None)?))?;
        let simulation = Self { geometry, physics, random };
        Ok(simulation)
    }

    fn run(
        &self,
        py: Python,
        primaries: &PyArray<ffi::Primary>,
        verbose: Option<bool>,
    ) -> PyResult<()> {
        let verbose = verbose.unwrap_or(false);

        let mut agent = RunAgent::new(py, self, primaries)?;
        let result = ffi::run_simulation(&mut agent, verbose)
            .to_result();

        // Synchronize back random stream.
        let mut random = self.random.borrow_mut(py);
        *random = agent.random.clone();

        result
    }
}

#[derive(FromPyObject)]
enum GeometryArg<'py> {
    #[pyo3(transparent, annotation = "Geometry")]
    Geometry(Bound<'py, Geometry>),
    #[pyo3(transparent, annotation = "str")]
    String(Bound<'py, PyString>),
}

#[pyfunction]
pub fn drop_simulation() {
    ffi::drop_simulation();
}

// ===============================================================================================
//
// Run agent (C++ interface).
//
// ===============================================================================================

pub struct RunAgent<'a> {
    geometry: SharedPtr<ffi::GeometryBorrow>,
    physics: ffi::Physics,
    primaries: &'a PyArray<ffi::Primary>,
    random: Random,
    index: usize,
}

impl<'a> RunAgent<'a> {
    pub fn events(&self) -> usize {
        self.primaries.len().unwrap()
    }

    pub fn geometry<'b>(&'b self) -> &'b ffi::GeometryBorrow {
        self.geometry.as_ref().unwrap()
    }

    fn new(
        py: Python,
        simulation: &Simulation,
        primaries: &'a PyArray<ffi::Primary>,
    ) -> PyResult<RunAgent<'a>> {
        let geometry = simulation.geometry
            .as_ref()
            .ok_or_else(|| Error::new(ValueError).what("geometry").why("undefined").to_err())?;
        let geometry = geometry.get().0.clone();
        let physics = simulation.physics.bind(py).borrow().0;
        let random = simulation.random.bind(py).borrow().clone();
        let index = 0;
        let agent = RunAgent { geometry, physics, primaries, random, index };
        Ok(agent)
    }

    pub fn next_open01(&mut self) -> f64 {
        self.random.open01()
    }

    pub fn next_primary<'b>(&'b mut self) -> &'b ffi::Primary {
        let primary = self.primaries.data(self.index).unwrap();
        self.index += 1;
        unsafe { (primary as *const ffi::Primary).as_ref().unwrap() }
    }

    pub fn physics<'b>(&'b self) -> &'b ffi::Physics {
        &self.physics
    }

    pub fn prng_name(&self) -> &'static str {
        "Pcg64Mcg"
    }
}
