use crate::geometry::Geometry;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::io::{DictLike, PathString};
use crate::utils::numpy::PyArray;
use crate::utils::tuple::NamedTuple;
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::types::PyString;
use std::ffi::c_char;

mod physics;
mod random;
pub mod sampler;
pub mod source;
pub mod tracker;

pub use physics::Physics;
pub use random::Random;
use sampler::{Deposits, ParticlesSampler, SamplerMode};
use tracker::Tracker;
pub use super::cxx::ffi;


// ===============================================================================================
//
// Simulation interface.
//
// ===============================================================================================

/// Interface to a Geant4 simulation.
#[pyclass(module="calzone")]
pub struct Simulation {
    /// The Monte Carlo `Geometry`.
    #[pyo3(get)]
    geometry: Option<Py<Geometry>>,
    /// Monte Carlo `Physics` settings.
    #[pyo3(get)]
    physics: Py<Physics>,
    /// Monte Carlo pseudo-random stream.
    #[pyo3(get, set)]
    random: Py<Random>,
    /// Sampling mode for energy deposits.
    #[pyo3(get, set)]
    sample_deposits: Option<SamplerMode>,
    /// Flag controlling the sampling of Monte Carlo particles.
    #[pyo3(get, set)]
    sample_particles: bool,
    /// Flag controlling the production of secondary Monte Carlo particles.
    #[pyo3(get, set)]
    secondaries: bool,
    /// Flag controlling the recording of Monte Carlo tracks.
    #[pyo3(get, set)]
    tracking: bool,
}

#[pymethods]
impl Simulation {
    #[new]
    fn new<'py>(
        py: Python<'py>,
        geometry: Option<GeometryArg>,
        physics: Option<PhysicsArg>,
        random: Option<&Bound<'py, Random>>,
        sample_deposits: Option<SamplerMode>,
        sample_particles: Option<bool>,
        secondaries: Option<bool>,
        tracking: Option<bool>,
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
        let sample_deposits = sample_deposits.or_else(|| Some(SamplerMode::Brief));
        let sample_particles = sample_particles.unwrap_or(false);
        let secondaries = secondaries.unwrap_or(true);
        let tracking = tracking.unwrap_or(false);
        let simulation = Self {
            geometry,
            physics,
            random,
            sample_deposits,
            sample_particles,
            secondaries,
            tracking
        };
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

    /// Run a Geant4 Monte Carlo simulation.
    #[pyo3(signature = (particles, /, verbose=false), text_signature="(particles, /)")]
    fn run(
        &self,
        py: Python,
        particles: Primaries<'_>,
        verbose: Option<bool>, // Hidden argument.
    ) -> PyResult<PyObject> {
        let verbose = verbose.unwrap_or(false);

        let mut agent = RunAgent::new(py, self, particles)?;
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
    String(PathString<'py>),
}

impl<'py> TryFrom<GeometryArg<'py>> for Py<Geometry> {
    type Error = PyErr;

    fn try_from(value: GeometryArg<'py>) -> Result<Py<Geometry>, Self::Error> {
        match value {
            GeometryArg::Geometry(geometry) => Ok(geometry.unbind()),
            GeometryArg::String(path) => {
                let py = path.0.py();
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
    primaries: Primaries<'a>,
    random: Random,
    // Iterator.
    index: usize,
    // Energy deposits.
    deposits: Option<Deposits>,
    // Sampled particles.
    particles: Option<ParticlesSampler>,
    // tracks.
    tracker: Option<Tracker>,
    // secondaries.
    secondaries: bool,
}

impl<'a> RunAgent<'a> {
    pub fn events(&self) -> usize {
        self.primaries.len()
    }

    fn export(self, py: Python) -> PyResult<PyObject> {
        let deposits = self.deposits.map(|deposits| deposits.export(py)).transpose()?;
        let particles = self.particles.map(|particles| particles.export(py)).transpose()?;
        let tracker = self.tracker.map(|tracker| tracker.export(py)).transpose()?;

        let result = match deposits {
            Some(deposits) => match particles {
                Some(particles) => match tracker {
                    Some((tracks, vertices)) => {
                        static RESULT: NamedTuple<4> = NamedTuple::new(
                            "Result", ["deposits", "particles", "tracks", "vertices"]);
                        RESULT.instance(py, (deposits, particles, tracks, vertices))?.unbind()
                    },
                    None => {
                        static RESULT: NamedTuple<2> = NamedTuple::new(
                            "Result", ["deposits", "particles"]);
                        RESULT.instance(py, (deposits, particles))?.unbind()
                    },
                },
                None => match tracker {
                    Some((tracks, vertices)) => {
                        static RESULT: NamedTuple<3> = NamedTuple::new(
                            "Result", ["deposits", "tracks", "vertices"]);
                        RESULT.instance(py, (deposits, tracks, vertices))?.unbind()
                    },
                    None => deposits,
                },
            },
            None => match particles {
                Some(particles) => match tracker {
                    Some((tracks, vertices)) => {
                        static RESULT: NamedTuple<3> = NamedTuple::new(
                            "Result", ["particles", "tracks", "vertices"]);
                        RESULT.instance(py, (particles, tracks, vertices))?.unbind()
                    },
                    None => particles,
                },
                None => match tracker {
                    Some((tracks, vertices)) => {
                        static RESULT: NamedTuple<2> = NamedTuple::new(
                            "Result", ["tracks", "vertices"]);
                        RESULT.instance(py, (tracks, vertices))?.unbind()
                    },
                    None => py.None(),
                },
            },
        };
        Ok(result)
    }

    pub fn geometry<'b>(&'b self) -> &'b ffi::GeometryBorrow {
        self.geometry.as_ref().unwrap()
    }

    pub fn is_deposits(&self) -> bool {
        self.deposits.is_some()
    }

    pub fn is_particles(&self) -> bool {
        self.particles.is_some()
    }

    pub fn is_secondaries(&self) -> bool {
        self.secondaries
    }

    pub fn is_tracker(&self) -> bool {
        self.tracker.is_some()
    }

    fn new(
        py: Python,
        simulation: &Simulation,
        primaries: Primaries<'a>,
    ) -> PyResult<RunAgent<'a>> {
        let geometry = simulation.geometry
            .as_ref()
            .ok_or_else(|| Error::new(ValueError).what("geometry").why("undefined").to_err())?;
        let geometry = geometry.get().0.clone();
        let physics = simulation.physics.bind(py).borrow().0;
        let random = simulation.random.bind(py).borrow().clone();
        let index = 0;
        let deposits = simulation.sample_deposits.map(|mode| Deposits::new(mode));
        let particles = if simulation.sample_particles {
            Some(ParticlesSampler::new())
        } else {
            None
        };
        let tracker = if simulation.tracking { Some(Tracker::new()) } else { None };
        let secondaries = simulation.secondaries;
        let agent = RunAgent {
            geometry, physics, primaries, random, index, deposits, particles, tracker, secondaries
        };
        Ok(agent)
    }

    pub fn next_open01(&mut self) -> f64 {
        self.random.open01()
    }

    pub fn next_primary(&mut self) -> ffi::Particle {
        let primary = self.primaries.data(self.index).unwrap();
        self.index += 1;
        match self.primaries {
            Primaries::Calzone(_) => {
                let primary = unsafe { (primary as *const ffi::Particle).as_ref().unwrap() };
                primary.clone()
            },
            Primaries::Goupil(_) => {
                let state = unsafe { (primary as *const ffi::GoupilState).as_ref().unwrap() };
                ffi::Particle {
                    pid: 22,
                    energy: state.energy,
                    position: state.position,
                    direction: state.direction,
                }
            },
        }
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

    pub fn push_particle(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        particle: ffi::Particle,
    ) {
        if let Some(particles) = self.particles.as_mut() {
            particles.push(volume, self.index, particle)
        }
    }

    pub fn push_track(&mut self, mut track: ffi::Track) {
        if let Some(tracker) = self.tracker.as_mut() {
            track.event = self.index;
            tracker.push_track(track)
        }
    }

    pub fn push_vertex(&mut self, mut vertex: ffi::Vertex) {
        if let Some(tracker) = self.tracker.as_mut() {
            vertex.event = self.index;
            tracker.push_vertex(vertex)
        }
    }
}

#[derive(FromPyObject)]
enum Primaries<'a> {
    Calzone(&'a PyArray<ffi::Particle>),
    Goupil(&'a PyArray<ffi::GoupilState>),
}

impl<'a> Primaries<'a> {
    fn data(&self, index: usize) -> PyResult<*mut c_char> {
        match self {
            Self::Calzone(v) => v.data(index),
            Self::Goupil(v) => v.data(index),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Calzone(v) => v.len().unwrap(),
            Self::Goupil(v) => v.len().unwrap(),
        }
    }
}
