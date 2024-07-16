use crate::geometry::Volume;
use crate::utils::error::Error;
use crate::utils::error::ErrorKind::ValueError;
use crate::utils::numpy::{PyArray, ShapeArg};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySlice};
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
    #[pyo3(get)]
    particles: PyObject, // XXX make private?
    #[pyo3(get, set)]
    random: Py<Random>,
    #[pyo3(get)]
    weights: PyObject, // XXX weighted particles instead?
}

#[pymethods]
impl ParticlesGenerator {
    #[new]
    #[pyo3(signature=(shape, random=None, **kwargs))]
    fn new<'py>(
        py: Python<'py>,
        shape: ShapeArg,
        random: Option<Bound<'py, Random>>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<Self> {
        let particles = particles(py, shape.clone(), kwargs)?;
        let shape: Vec<usize> = shape.into();
        let weights: &PyAny = PyArray::<f64>::zeros(py, &shape)?;
        weights.set_item(PySlice::full_bound(py), 1.0_f64)?;
        let weights: PyObject = weights.into();
        let random = match random {
            None => Py::new(py, Random::new(None)?)?,
            Some(random) => random.unbind(),
        };
        let generator = Self { particles, weights, random };
        Ok(generator)
    }

    fn inside<'py>(
        slf: Bound<'py, Self>,
        volume: &Volume,
        daughters: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let daughters = daughters.unwrap_or(false);
        Ok(slf)
    }

    fn spectral_lines<'py>(
        slf: Bound<'py, Self>,
        energies: Vec<f64>, // XXX Optional / spectrum case.
        intensities: Option<Vec<f64>>,
    ) -> PyResult<Bound<'py, Self>> {
        // XXX Check for any other energy method already called?
        let spectrum = match intensities {
            None => {
                let spectrum: Vec<_> = energies.iter()
                    .map(|energy| (*energy, 1.0))
                    .collect();
                spectrum
            }
            Some(intensities) => {
                if intensities.len() != energies.len() {
                    let why = format!(
                        "expected a length {} sequence, found a length {} sequence",
                        energies.len(),
                        intensities.len(),
                    );
                    let err = Error::new(ValueError)
                        .what("intensities")
                        .why(&why);
                    return Err(err.to_err());
                }
                let spectrum: Vec<_> = energies.iter().zip(intensities.iter())
                    .filter_map(|(energy, intensity)| if *intensity > 0.0 {
                        Some((*energy, *intensity))
                    } else {
                        None
                    })
                    .collect();
                spectrum
            },
        };
        let total_intensity: f64 = spectrum.iter()
            .map(|(_, intensity)| intensity)
            .sum();
        if total_intensity <= 0.0 {
            let err = Error::new(ValueError)
                .what("intensities")
                .why("no positive value");
            return Err(err.to_err());
        }

        let py = slf.py();
        let generator = slf.borrow();
        let particles_energy: &PyArray<f64> = generator
            .particles
            .bind(py)
            .get_item("energy")?
            .extract()?;
        let weights: &PyArray<f64> = generator
            .weights
            .bind(py)
            .extract()?;
        let mut random = generator.random.bind(py).borrow_mut();
        for i in 0..particles_energy.len()? {
            let energy = {
                let target = random.open01() * total_intensity;
                let mut acc = 0.0;
                let mut j = 0_usize;
                loop {
                    let (energy, intensity) = spectrum[j];
                    acc += intensity;
                    if (acc >= target) || (j == spectrum.len() - 1) {
                        let weight = weights.get(i)? * (total_intensity / intensity);
                        weights.set(i, weight)?;
                        break energy
                    } else {
                        j += 1;
                    }
                }
            };
            particles_energy.set(i, energy)?;
        }
        Ok(slf)
    }
}
