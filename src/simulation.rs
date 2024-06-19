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
pub mod sampler;
pub mod source;

pub use physics::Physics;
pub use random::Random;
use sampler::{SamplerMode, Deposits};
pub use super::cxx::ffi;


// ===============================================================================================
//
// Simulation interface.
//
// ===============================================================================================

#[pyclass(module="calzone")]
pub struct Simulation {
    #[pyo3(get)]
    geometry: Option<Py<Geometry>>,
    #[pyo3(get)]
    physics: Py<Physics>,
    #[pyo3(get, set)]
    random: Py<Random>,
    #[pyo3(get, set)]
    sampler: Option<SamplerMode>,
}

#[pymethods]
impl Simulation {
    #[new]
    fn new<'py>(
        py: Python<'py>,
        geometry: Option<GeometryArg>,
        physics: Option<PhysicsArg>,
        random: Option<&Bound<'py, Random>>,
    ) -> PyResult<Self> {
        let geometry = geometry
            .map(|geometry| {
                let geometry: PyResult<Py<Geometry>> = geometry.try_into();
                geometry
            })
            .transpose()?;
        let physics = physics
            .map(|physics| {
                let physics: PyResult<Py<Physics>> = physics.try_into();
                physics
            })
            .unwrap_or_else(|| Py::new(py, Physics::default()))?;
        let random = random
            .map(|random| Ok(random.clone().unbind()))
            .unwrap_or_else(|| Py::new(py, Random::new(None)?))?;
        let sampler = Some(SamplerMode::Brief);
        let simulation = Self { geometry, physics, random, sampler };
        Ok(simulation)
    }

    #[setter]
    fn set_geometry(&mut self, geometry: Option<GeometryArg>) -> PyResult<()> {
        match geometry {
            None => self.geometry = None,
            Some(geometry) => {
                let geometry: Py<Geometry> = geometry.try_into()?;
                self.geometry = Some(geometry);
            },
        }
        Ok(())
    }

    #[setter]
    fn set_physics(&mut self, py: Python, physics: Option<PhysicsArg>) -> PyResult<()> {
        let physics: Py<Physics> = match physics {
            None => Py::new(py, Physics::none())?,
            Some(physics) => physics.try_into()?,
        };
        self.physics = physics;
        Ok(())
    }

    fn run(
        &self,
        py: Python,
        primaries: &PyArray<ffi::Primary>,
        verbose: Option<bool>,
    ) -> PyResult<PyObject> {
        let verbose = verbose.unwrap_or(false);

        let mut agent = RunAgent::new(py, self, primaries)?;
        let result = ffi::run_simulation(&mut agent, verbose)
            .to_result();

        // Synchronize back random stream.
        let mut random = self.random.borrow_mut(py);
        *random = agent.random.clone();

        result.and_then(|_| agent.export(py))
    }
}

#[derive(FromPyObject)]
enum GeometryArg<'py> {
    #[pyo3(transparent, annotation = "Geometry")]
    Geometry(Bound<'py, Geometry>),
    #[pyo3(transparent, annotation = "str")]
    String(Bound<'py, PyString>),
}

impl<'py> TryFrom<GeometryArg<'py>> for Py<Geometry> {
    type Error = PyErr;

    fn try_from(value: GeometryArg<'py>) -> Result<Py<Geometry>, Self::Error> {
        match value {
            GeometryArg::Geometry(geometry) => Ok(geometry.unbind()),
            GeometryArg::String(path) => {
                let py = path.py();
                let path = DictLike::String(path);
                let geometry = Geometry::new(path)?;
                Py::new(py, geometry)
            },
        }
    }
}

#[derive(FromPyObject)]
enum PhysicsArg<'py> {
    #[pyo3(transparent, annotation = "Physics")]
    Physics(Bound<'py, Physics>),
    #[pyo3(transparent, annotation = "str")]
    String(Bound<'py, PyString>),
}

impl<'py> TryFrom<PhysicsArg<'py>> for Py<Physics> {
    type Error = PyErr;

    fn try_from(value: PhysicsArg<'py>) -> Result<Py<Physics>, Self::Error> {
        match value {
            PhysicsArg::Physics(physics) => Ok(physics.unbind()),
            PhysicsArg::String(model) => {
                let py = model.py();
                let model = model.to_cow()?;
                let physics = Physics::new(Some(model.as_ref()), None, None)?;
                Py::new(py, physics)
            },
        }
    }
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
    // Iterator.
    index: usize,
    // Samples.
    deposits: Option<Deposits>,
}

impl<'a> RunAgent<'a> {
    pub fn events(&self) -> usize {
        self.primaries.len().unwrap()
    }

    fn export(self, py: Python) -> PyResult<PyObject> {
        match self.deposits {
            None => Ok(py.None()),
            Some(deposits) => deposits.export(py),
        }
    }

    pub fn geometry<'b>(&'b self) -> &'b ffi::GeometryBorrow {
        self.geometry.as_ref().unwrap()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn is_sampler(&self) -> bool {
        self.deposits.is_some()
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
        let deposits = simulation.sampler.map(|mode| Deposits::new(mode));
        let agent = RunAgent { geometry, physics, primaries, random, index, deposits };
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

    pub fn push_deposit(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        deposit: f64,
        non_ionising: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
    ) {
        if let Some(deposits) = self.deposits.as_mut() {
            deposits.push(volume, self.index, deposit, non_ionising, start, end)
        }
    }

}
