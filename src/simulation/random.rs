use crate::utils::numpy::{PyArray, ShapeArg};
use getrandom::getrandom;
use pyo3::prelude::*;
use pyo3::exceptions::PySystemError;
use rand::Rng;
use rand::distributions::Open01;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use std::pin::Pin;
use super::ffi;


// ===============================================================================================
//
// Generator interface.
//
// ===============================================================================================

/// A Pseudo-Random Numbers Generator (PRNG).
#[derive(Clone)]
#[pyclass(module = "calzone")]
pub struct Random {
    rng: Pcg64Mcg,
    /// Prng stream index.
    #[pyo3(get)]
    index: u128,
    /// Prng initial seed.
    #[pyo3(get)]
    seed: u128,
}

#[pymethods]
impl Random {
    #[new]
    pub fn new(seed: Option<u128>) -> PyResult<Self> {
        let rng = Pcg64Mcg::new(0xCAFEF00DD15EA5E5);
        let mut random = Self { rng, seed: 0, index: 0 };
        random.initialise(seed)?;
        Ok(random)
    }

    #[setter]
    fn set_index(&mut self, index: Option<u128>) -> PyResult<()> {
        match index {
            None => self.initialise(Some(self.seed))?,
            Some(index) => {
                let delta: u128 = index.wrapping_sub(self.index);
                self.rng.advance(delta);
                self.index = index;
            },
        }
        Ok(())
    }

    #[setter]
    fn set_seed(&mut self, seed: Option<u128>) -> PyResult<()> {
        self.initialise(seed)
    }

    /// Generate pseudo-random number(s) uniformly distributed over (0,1).
    fn uniform01(
        &mut self,
        py: Python,
        shape: Option<ShapeArg>,
    ) -> PyResult<PyObject> {
        match shape {
            None => {
                let value = self.open01();
                Ok(value.into_py(py))
            },
            Some(shape) => {
                let shape: Vec<usize> = shape.into();
                let n = shape.iter().product();
                let iter = (0..n).map(|_| self.open01());
                let array: &PyAny = PyArray::<f64>::from_iter(py, &shape, iter)?;
                Ok(array.into())
            },
        }
    }
}

impl Random {
    fn initialise(&mut self, seed: Option<u128>) -> PyResult<()> {
        match seed {
            None => {
                let mut seed = [0_u8; 16];
                getrandom(&mut seed)
                    .map_err(|_| PySystemError::new_err("could not seed random engine"))?;
                self.rng = Pcg64Mcg::from_seed(seed);
                self.seed = u128::from_ne_bytes(seed);
            },
            Some(seed) => {
                self.seed = seed;
                let seed = u128::to_ne_bytes(seed);
                self.rng = Pcg64Mcg::from_seed(seed);
            },
        }
        self.index = 0;
        Ok(())
    }

    pub(super) fn open01(&mut self) -> f64 {
        self.index += 1;
        self.rng.sample::<f64, Open01>(Open01)
    }
}


// ===============================================================================================
//
// Random context.
//
// ===============================================================================================

pub struct RandomContext<'a> (&'a mut Random);

impl<'a> RandomContext<'a> {
    pub fn next_open01(&mut self) -> f64 {
        self.0.open01()
    }

    pub fn new(prng: &'a mut Random) -> Pin<Box<Self>> {
        let mut context = Box::pin(Self (prng)); // Pin memory location.
        ffi::set_random_context(&mut context);
        context
    }

    pub fn prng_name(&self) -> &'static str {
        "Pcg64Mcg"
    }
}

impl<'a> Drop for RandomContext<'a> {
    fn drop(&mut self) {
        ffi::release_random_context();
    }
}
