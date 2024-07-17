use crate::geometry::{Geometry, Volume};
use crate::utils::error::{ctrlc_catched, Error};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, TypeError, ValueError};
use crate::utils::numpy::{PyArray, ShapeArg};
use crate::utils::tuple::NamedTuple;
use cxx::SharedPtr;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySlice, PyString};
use std::borrow::Cow;
use super::ffi;
use super::random::Random;


// ===============================================================================================
//
// Array interface.
//
// ===============================================================================================

/// Create an array of Monte Carlo particles.
#[pyfunction]
#[pyo3(signature=(shape, **kwargs))]
pub fn particles(
    py: Python,
    shape: ShapeArg,
    kwargs: Option<&Bound<PyDict>>
) -> PyResult<PyObject> {
    let shape: Vec<usize> = shape.into();
    let array: &PyAny = PyArray::<ffi::Particle>::zeros(py, &shape)?;
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


// ===============================================================================================
//
// Generator interface.
//
// ===============================================================================================

#[pyclass(module="calzone")]
pub struct ParticlesGenerator {
    particles: PyObject,
    random: Py<Random>,
    geometry: Option<SharedPtr<ffi::GeometryBorrow>>,
    weights: Option<PyObject>,
    // Status flags.
    is_pid: bool,
    is_energy: bool,
    is_position: bool,
    is_direction: bool,
}

// XXX Solid angle generator.
// XXX Surface generator / cos(theta).
// XXX Power law generator.
// XXX const setters.

#[pymethods]
impl ParticlesGenerator {
    #[new]
    pub fn new<'py>(
        py: Python<'py>,
        shape: ShapeArg,
        geometry: Option<&Bound<'py, Geometry>>,
        random: Option<Bound<'py, Random>>,
        weight: Option<bool>,
    ) -> PyResult<Self> {
        let shape: Vec<usize> = shape.into();
        let particles = {
            let particles: &PyAny = PyArray::<ffi::Particle>::zeros(py, &shape)?;
            let particles: PyObject = particles.into();
            particles
        };
        let weights = match weight.unwrap_or(false) {
            false => None,
            true => Some(Self::new_weights(py, &shape)?),
        };
        let random = match random {
            None => Py::new(py, Random::new(None)?)?,
            Some(random) => random.unbind(),
        };
        let geometry = geometry.map(|geometry| geometry.borrow().0.clone());
        let generator = Self {
            particles, random, geometry, weights,
            is_pid: false,
            is_energy: false,
            is_position: false,
            is_direction: false,
        };
        Ok(generator)
    }

    fn generate<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if !self.is_pid {
            self.particles.bind(py).set_item("pid", 22)?;
            self.is_pid = true;
        }

        if !self.is_energy {
            self.particles.bind(py).set_item("energy", 1.0)?;
            self.is_energy = true;
        }

        if !self.is_position {
            // Positions are already initialised to 0.
            self.is_position = true;
        }

        if !self.is_direction {
            self.generate_solid_angle(py, None, None, None)?;
        }

        match self.weights.as_ref() {
            None => Ok(self.particles.bind(py).clone().into_any()),
            Some(weights) => {
                static RESULT: NamedTuple<2> = NamedTuple::new(
                    "Result", ["particles", "weights"]);
                RESULT.instance(py, (&self.particles, weights))
            },
        }
    }

    fn inside<'py>(
        slf: Bound<'py, Self>,
        volume: VolumeArg,
        include_daughters: Option<bool>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let py = slf.py();
        let is_weights = {
            let generator = slf.borrow();
            if generator.is_position {
                let err = Error::new(ValueError)
                    .what("inside")
                    .why("position already defined");
                return Err(err.to_err())
            }
            generator.weights.is_some()
        };

        let volume = volume.resolve(slf.borrow().geometry.as_ref())?;

        let weight = weight.unwrap_or(is_weights);
        if weight && !is_weights {
            let mut generator = slf.borrow_mut();
            generator.initialise_weights(py)?;
        }

        let include_daughters = include_daughters.unwrap_or(false);
        let [xmin, xmax, ymin, ymax, zmin, zmax] = volume.volume.compute_box("");
        let transform = volume.volume.compute_transform("");
        let generator = slf.borrow();
        let positions: &PyArray<f64> = generator
            .particles
            .bind(py)
            .get_item("position")?
            .extract()?;
        let weights: Option<&PyArray<f64>> = if weight {
            generator
                .weights
                .as_ref()
                .map(|weights| weights.bind(py).extract())
                .transpose()?
        } else {
            None
        };
        let mut random = generator.random.bind(py).borrow_mut();
        let n = positions.size() / 3;
        let mut trials = 0;
        let mut i = 0_usize;
        while i < n {
            let r = [
                (xmax - xmin) * random.open01() + xmin,
                (ymax - ymin) * random.open01() + ymin,
                (zmax - zmin) * random.open01() + zmin,
            ];
            if volume.volume.inside(&r, &transform, include_daughters) == ffi::EInside::kInside {
                for j in 0..3 {
                    positions.set(3 * i + j, r[j])?;
                }
                i += 1;
            }
            trials += 1;
            if ((trials % 1000) == 0) && ctrlc_catched() {
                return Err(Error::new(KeyboardInterrupt).to_err())
            }
        }
        if let Some(weights) = weights {
            // XXX Use exact volume, when available.
            let p = (n as f64) / (trials as f64);
            let volume = (xmax - xmin) * (ymax - ymin) * (zmax - zmin) * p;
            for i in 0..n {
                let weight = weights.get(i)? * volume;
                weights.set(i, weight)?;
            }
        }

        drop(generator);
        let mut generator = slf.borrow_mut();
        generator.is_position = true;

        Ok(slf)
    }

    fn solid_angle<'py>(
        slf: Bound<'py, Self>,
        theta: Option<[f64; 2]>,
        phi: Option<[f64; 2]>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let py = slf.py();
        let mut generator = slf.borrow_mut();
        generator.generate_solid_angle(py, theta, phi, weight)?;
        Ok(slf)
    }

    fn spectrum<'py>(
        slf: Bound<'py, Self>,
        data: Vec<[f64; 2]>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let py = slf.py();
        let is_weights = {
            let generator = slf.borrow();
            if generator.is_energy {
                let err = Error::new(ValueError)
                    .what("spectrum")
                    .why("energy already defined");
                return Err(err.to_err())
            }
            generator.weights.is_some()
        };

        let weight = weight.unwrap_or(is_weights);
        if weight && !is_weights {
            let mut generator = slf.borrow_mut();
            generator.initialise_weights(py)?;
        }

        struct EmissionLine {
            energy: f64,
            intensity: f64,
        }

        let spectrum: Vec<EmissionLine> = data.iter()
            .filter_map(|[energy, intensity]| if *intensity > 0.0 {
                Some(EmissionLine { energy: *energy, intensity: *intensity })
            } else {
                None
            })
            .collect();
        let total_intensity: f64 = spectrum.iter()
            .map(|line| line.intensity)
            .sum();
        if total_intensity <= 0.0 {
            let err = Error::new(ValueError)
                .what("data")
                .why("no positive intensity");
            return Err(err.to_err());
        }

        let generator = slf.borrow();
        let particles_energy: &PyArray<f64> = generator
            .particles
            .bind(py)
            .get_item("energy")?
            .extract()?;
        let weights: Option<&PyArray<f64>> = if weight {
            generator
                .weights
                .as_ref()
                .map(|weights| weights.bind(py).extract())
                .transpose()?
        } else {
            None
        };
        let mut random = generator.random.bind(py).borrow_mut();
        for i in 0..particles_energy.size() {
            let energy = {
                let target = random.open01() * total_intensity;
                let mut acc = 0.0;
                let mut j = 0_usize;
                loop {
                    let EmissionLine { energy, intensity } = spectrum[j];
                    acc += intensity;
                    if (acc >= target) || (j == spectrum.len() - 1) {
                        if let Some(weights) = weights {
                            let weight = weights.get(i)? * (total_intensity / intensity);
                            weights.set(i, weight)?;
                        }
                        break energy
                    } else {
                        j += 1;
                    }
                }
            };
            particles_energy.set(i, energy)?;
        }

        drop(generator);
        let mut generator = slf.borrow_mut();
        generator.is_energy = true;

        Ok(slf)
    }
}

#[derive(FromPyObject)]
pub enum VolumeArg<'py> {
    #[pyo3(transparent, annotation = "str")]
    Path(Bound<'py, PyString>),
    #[pyo3(transparent, annotation = "Volume")]
    Volume(Bound<'py, Volume>),
}

impl<'py> VolumeArg<'py> {
    fn resolve(
        &self,
        geometry: Option<&SharedPtr<ffi::GeometryBorrow>>
    ) -> PyResult<Cow<'_, Volume>> {
        let volume = match self {
            Self::Path(path) => {
                let path = path.to_cow()?;
                let geometry = geometry
                    .ok_or_else(|| {
                        let err = Error::new(TypeError)
                            .what("volume")
                            .why("expected a 'Volume', found a 'str'");
                        err.to_err()
                    })?;
                let volume = Volume::new(geometry, &path)?;
                Cow::Owned(volume)
            },
            Self::Volume(volume) => Cow::Borrowed(volume.get()),
        };
        Ok(volume)
    }
}

impl ParticlesGenerator {
    fn generate_solid_angle(
        &mut self,
        py: Python,
        theta: Option<[f64; 2]>,
        phi: Option<[f64; 2]>,
        weight: Option<bool>,
    ) -> PyResult<()> {
        let is_weights = {
            if self.is_direction {
                let err = Error::new(ValueError)
                    .what("solid_angle")
                    .why("direction already defined");
                return Err(err.to_err())
            }
            self.weights.is_some()
        };

        let weight = weight.unwrap_or(is_weights);
        if weight && !is_weights {
            self.initialise_weights(py)?;
        }

        let (cos_theta0, cos_theta1) = match theta {
            None => (-1.0, 1.0),
            Some([theta0, theta1]) => {
                let theta0 = ((theta0 % 180.0) / 180.0) * std::f64::consts::PI;
                let theta1 = ((theta1 % 180.0) / 180.0) * std::f64::consts::PI;
                (theta0.cos(), theta1.cos())
            },
        };

        const TWO_PI: f64 = 2.0 * std::f64::consts::PI;
        let (phi0, phi1) = match phi {
            None => (0.0, TWO_PI),
            Some([phi0, phi1]) => {
                let phi0 = ((phi0 % 360.0) / 360.0) * TWO_PI;
                let phi1 = ((phi1 % 360.0) / 360.0) * TWO_PI;
                (phi0, phi1)
            },
        };

        let directions: &PyArray<f64> = self
            .particles
            .bind(py)
            .get_item("direction")?
            .extract()?;
        let weights: Option<&PyArray<f64>> = if weight {
            self
                .weights
                .as_ref()
                .map(|weights| weights.bind(py).extract())
                .transpose()?
        } else {
            None
        };
        let mut random = self.random.bind(py).borrow_mut();
        let n = directions.size() / 3;
        for i in 0..n {
            let cos_theta = (cos_theta1 - cos_theta0) * random.open01() + cos_theta0;
            let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
            let phi = (phi1 - phi0) * random.open01() + phi0;
            let u = [
                sin_theta * phi.cos(),
                sin_theta * phi.sin(),
                cos_theta,
            ];
            for j in 0..3 {
                directions.set(3 * i + j, u[j])?;
            }
        }
        if let Some(weights) = weights {
            let solid_angle = (cos_theta1 - cos_theta0).abs() * (phi1 - phi0).abs();
            for i in 0..n {
                let weight = weights.get(i)? * solid_angle;
                weights.set(i, weight)?;
            }
        }

        self.is_direction = true;
        Ok(())
    }

    fn initialise_weights(&mut self, py: Python) -> PyResult<()> {
        let particles: &PyArray<ffi::Particle> = self.particles.bind(py).extract()?;
        self.weights = Some(Self::new_weights(py, &particles.shape())?);
        Ok(())
    }

    fn new_weights(py: Python, shape: &[usize]) -> PyResult<PyObject> {
        let weights: &PyAny = PyArray::<f64>::zeros(py, shape)?;
        weights.set_item(PySlice::full_bound(py), 1.0_f64)?;
        Ok(weights.into())
    }
}
