#![allow(deprecated)]

use crate::geometry::Geometry;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::namespace::Namespace;
use crate::utils::numpy::{PyArray, PyArrayMethods};
use crate::utils::io::DictLike;
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::types::PyString;
use std::pin::Pin;

mod physics;
mod random;
pub mod sampler;
pub mod source;
pub mod tracker;

pub use physics::Physics;
pub use random::{Random, RandomContext};
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
    /// Flag controlling the sampling of particles at volume boundaries.
    #[pyo3(get, set)]
    sample_particles: bool,
    /// Flag controlling the production of secondary particles.
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
            .unwrap_or_else(|| Py::new(py, Random::new(None, None)?))?;
        let sample_deposits = sample_deposits.or_else(|| Some(SamplerMode::Brief));
        let sample_particles = sample_particles.unwrap_or(true);
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

    /// Create a Monte Carlo particles generator.
    fn particles(
        &self,
        py: Python,
        weight: Option<bool>,
    ) -> PyResult<source::ParticlesGenerator> {
        let geometry = self.geometry
            .as_ref()
            .map(|geometry| geometry.bind(py));
        let random = Some(self.random.bind(py).clone());
        source::ParticlesGenerator::new(py, geometry, random, weight)
    }

    /// Run a Geant4 Monte Carlo simulation.
    #[pyo3(signature = (particles, /, *, random_indices=None, verbose=false))]
    #[pyo3(text_signature = "(particles, /, *, random_indices=None)")]
    fn run<'py>(
        &self,
        particles: &Bound<'py, PyAny>,
        random_indices: Option<&PyArray<u64>>,
        verbose: Option<bool>, // Hidden argument.
    ) -> PyResult<PyObject> {
        let py = particles.py();
        let verbose = verbose.unwrap_or(false);
        let particles = source::ParticlesIterator::new(particles)?;
        let mut agent = RunAgent::new(py, self, particles, random_indices)?;
        let mut binding = self.random.bind(py).borrow_mut();
        let mut random = RandomContext::new(&mut binding);
        let result = ffi::run_simulation(&mut agent, &mut random, verbose)
            .to_result();

        let agent = Pin::into_inner(agent);
        result.and_then(|_| agent.export(py))
    }
}

#[derive(FromPyObject)]
enum GeometryArg<'py> {
    #[pyo3(transparent, annotation = "Geometry")]
    Geometry(Bound<'py, Geometry>),
    #[pyo3(transparent, annotation = "dict|str")]
    Data(DictLike<'py>),
}

impl<'py> TryFrom<GeometryArg<'py>> for Py<Geometry> {
    type Error = PyErr;

    fn try_from(value: GeometryArg<'py>) -> Result<Py<Geometry>, Self::Error> {
        match value {
            GeometryArg::Geometry(geometry) => Ok(geometry.unbind()),
            GeometryArg::Data(data) => {
                let py = data.py();
                let geometry = Geometry::new(data)?;
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
    primaries: source::ParticlesIterator<'a>,
    indices: Option<&'a PyArray<u64>>,
    // Iterator.
    index: usize,
    random_index: [u64; 2],
    weight: f64,
    // Energy deposits.
    deposits: Option<Deposits>,
    // Sampled particles.
    particles: Option<ParticlesSampler>,
    // tracks.
    tracker: Option<Tracker>,
    tracker_index: Vec<[u64; 2]>,
    // secondaries.
    secondaries: bool,
}

impl<'a> RunAgent<'a> {
    pub fn events(&self) -> usize {
        self.primaries.size()
    }

    fn export(mut self, py: Python) -> PyResult<PyObject> {
        let deposits = self.deposits.map(|deposits| deposits.export(py)).transpose()?;
        let particles = self.particles.map(|particles| particles.export(py)).transpose()?;
        let tracker = self.tracker.map(|tracker| tracker.export(py)).transpose()?;
        let random_index = if tracker.is_some() {
            let array = PyArray::<u64>::empty(py, &[self.index, 2])?;
            let random_index = unsafe { array.slice_mut()? };
            for (i, index) in self.tracker_index.drain(..).enumerate() {
                random_index[2 * i] = index[0];
                random_index[2 * i + 1] = index[1];
            }
            Some(array.into_any().unbind())
        } else {
            None
        };

        let result = match deposits {
            Some(deposits) => match particles {
                Some(particles) => match tracker {
                    Some((tracks, vertices)) => Namespace::new(py, &[
                            ("deposits", deposits),
                            ("particles", particles),
                            ("random_index", random_index.unwrap()),
                            ("tracks", tracks),
                            ("vertices", vertices),
                        ])?.unbind(),
                    None => Namespace::new(py, &[
                            ("deposits", deposits),
                            ("particles", particles),
                        ])?.unbind(),
                },
                None => match tracker {
                    Some((tracks, vertices)) => Namespace::new(py, &[
                            ("deposits", deposits),
                            ("random_index", random_index.unwrap()),
                            ("tracks", tracks),
                            ("vertices", vertices),
                        ])?.unbind(),
                    None => deposits,
                },
            },
            None => match particles {
                Some(particles) => match tracker {
                    Some((tracks, vertices)) => Namespace::new(py, &[
                            ("particles", particles),
                            ("random_index", random_index.unwrap()),
                            ("tracks", tracks),
                            ("vertices", vertices),
                        ])?.unbind(),
                    None => particles,
                },
                None => match tracker {
                    Some((tracks, vertices)) => Namespace::new(py, &[
                            ("random_index", random_index.unwrap()),
                            ("tracks", tracks),
                            ("vertices", vertices),
                        ])?.unbind(),
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

    pub fn is_random_indices(&self) -> bool {
        self.indices.is_some()
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
        primaries: source::ParticlesIterator<'a>,
        indices: Option<&'a PyArray<u64>>,
    ) -> PyResult<Pin<Box<RunAgent<'a>>>> {
        if let Some(indices) = indices {
            if indices.size() != 2 * primaries.size() {
                let why = format!(
                    "expected a size {} array, found a size {} array",
                    2 * primaries.size(),
                    indices.size(),
                );
                let err = Error::new(ValueError)
                    .what("random_indices")
                    .why(&why);
                return Err(err.to_err())
            }
        }
        let geometry = simulation.geometry
            .as_ref()
            .ok_or_else(|| Error::new(ValueError).what("geometry").why("undefined").to_err())?;
        let geometry = geometry.get().0.clone();
        let physics = simulation.physics.bind(py).borrow().0;
        let index = 0;
        let random_index = [0, 0];
        let weight = 0.0;
        let deposits = simulation.sample_deposits.map(|mode| Deposits::new(mode));
        let particles = if simulation.sample_particles {
            Some(ParticlesSampler::new())
        } else {
            None
        };
        let tracker = if simulation.tracking { Some(Tracker::new()) } else { None };
        let tracker_index = Vec::new();
        let secondaries = simulation.secondaries;
        let agent = RunAgent {
            geometry, physics, primaries, indices, index, random_index, weight, deposits,
            particles, tracker, tracker_index, secondaries
        };
        Ok(Box::pin(agent))
    }

    pub fn next_primary(&mut self, random_index: &[u64; 2]) -> ffi::Particle {
        if self.tracker.is_some() {
            self.tracker_index.push(*random_index);
        }

        self.index += 1;
        self.random_index = *random_index;
        let (particle, weight) = self.primaries.next().unwrap().unwrap();
        self.weight = weight;
        particle
    }

    pub fn next_random_index(&self) -> [u64; 2] {
        let Some(indices) = self.indices else { unreachable!() };
        [
            indices.get(2 * self.index).unwrap(),
            indices.get(2 * self.index + 1).unwrap(),
        ]
    }

    pub fn physics<'b>(&'b self) -> &'b ffi::Physics {
        &self.physics
    }

    pub fn push_deposit(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        tid: i32,
        pid: i32,
        energy: f64,
        total_deposit: f64,
        point_deposit: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
    ) {
        if let Some(deposits) = self.deposits.as_mut() {
            deposits.push(
                volume, self.index - 1, tid, pid, energy, total_deposit, point_deposit, start, end,
                self.weight, &self.random_index
            )
        }
    }

    pub fn push_particle(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        tid: i32,
        particle: ffi::Particle,
    ) {
        if let Some(particles) = self.particles.as_mut() {
            particles.push(volume, self.index - 1, tid, particle, self.weight, &self.random_index)
        }
    }

    pub fn push_track(&mut self, mut track: ffi::Track) {
        if let Some(tracker) = self.tracker.as_mut() {
            track.event = self.index - 1;
            tracker.push_track(track)
        }
    }

    pub fn push_vertex(&mut self, mut vertex: ffi::Vertex) {
        if let Some(tracker) = self.tracker.as_mut() {
            vertex.event = self.index - 1;
            tracker.push_vertex(vertex)
        }
    }
}
