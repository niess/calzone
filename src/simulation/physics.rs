use crate::utils::error::{Error, variant_error};
use crate::utils::error::ErrorKind::{TypeError, ValueError};
use crate::utils::numpy::{PyArray, PyArrayMethods};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use std::collections::HashMap;
use super::ffi;
use super::source::PidArg;


// ===============================================================================================
//
// Physics interface.
//
// ===============================================================================================

/// Interface to Geant4 physics settings.
#[derive(Default)]
#[pyclass(module="calzone")]
pub struct Physics;

#[pymethods]
impl Physics {
    #[new]
    #[pyo3(signature=(em_model=None, *, default_cut=None, had_model=None))]
    pub fn new(
        py: Python,
        em_model: Option<&str>,
        default_cut: Option<f64>,
        had_model: Option<&str>,
    ) -> PyResult<Self> {
        let mut physics = Self;
        if let Some(em_model) = em_model {
            physics.set_em_model(py, Some(em_model))?;
        }
        if let Some(had_model) = had_model {
            physics.set_had_model(py, Some(had_model))?;
        }
        if let Some(default_cut) = default_cut {
            physics.set_default_cut(py, default_cut)?;
        }
        Ok(physics)
    }

    /// Default value for production cuts, in cm.
    #[getter]
    fn get_default_cut(&self, _py: Python /* Lock */) -> f64 {
        ffi::get_default_cut()
    }

    #[setter]
    fn set_default_cut(&mut self, _py: Python /* Lock */, value: f64) -> PyResult<()> {
        if value > 0.0 {
            ffi::set_default_cut(value);
            Ok(())
        } else {
            let why = format!("expected a positive value, found {}", value);
            let err = Error::new(ValueError).what("default_cut").why(&why);
            Err(err.into())
        }
    }

    /// Simulation of Auger electrons.
    #[getter]
    fn get_em_auger(&self, _py: Python /* Lock */) -> bool {
        ffi::get_em_auger()
    }

    #[setter]
    pub fn set_em_auger(&mut self, _py: Python /* Lock */, value: bool) {
        ffi::set_em_auger(value);
        if value {
            ffi::set_em_fluo(true);
        }
    }

    /// Simulation of X-fluorescence.
    #[getter]
    fn get_em_fluo(&self, _py: Python /* Lock */) -> bool {
        ffi::get_em_fluo()
    }

    #[setter]
    pub fn set_em_fluo(&mut self, _py: Python /* Lock */, value: bool) {
        ffi::set_em_fluo(value);
        if !value {
            ffi::set_em_auger(false);
            ffi::set_em_pixe(false);
        }
    }

    /// Electromagnetic physics list.
    #[getter]
    fn get_em_model(&self, _py: Python /* Lock */) -> Option<&'static str> {
        let model: Option<EmPhysicsModel> = ffi::get_em_model().into();
        model.map(|m| m.to_str())
    }

    #[setter]
    pub fn set_em_model(&mut self, _py: Python /* Lock */, value: Option<&str>) -> PyResult<()> {
        Self::check_state("em_model")?;
        let model: ffi::EmPhysicsModel = match value {
            None => ffi::EmPhysicsModel::None,
            Some(value) => match EmPhysicsModel::from_str(value) {
                Ok(model) => model.into(),
                Err(options) => {
                    return Err(variant_error("bad model", value, options))
                },
            },
        };
        ffi::set_em_model(model);
        match model {
            ffi::EmPhysicsModel::Livermore | ffi::EmPhysicsModel::Penelope => {
                ffi::set_em_auger(false);
                ffi::set_em_fluo(true);
                ffi::set_em_pixe(false);
            },
            ffi::EmPhysicsModel::Option4 => {
                ffi::set_em_auger(true);
                ffi::set_em_fluo(true);
                ffi::set_em_pixe(false);
            },
            _ => {
                ffi::set_em_auger(false);
                ffi::set_em_fluo(false);
                ffi::set_em_pixe(false);
            },
        }
        Ok(())
    }

    /// Simulation of Particle-Induced X-Emission.
    #[getter]
    fn get_em_pixe(&self, _py: Python /* Lock */) -> bool {
        ffi::get_em_pixe()
    }

    #[setter]
    pub fn set_em_pixe(&mut self, _py: Python /* Lock */, value: bool) {
        ffi::set_em_pixe(value);
        if value {
            ffi::set_em_fluo(true);
        }
    }

    /// Hadronic physics list.
    #[getter]
    fn get_had_model(&self, _py: Python /* Lock */) -> Option<&'static str> {
        let model: Option<HadPhysicsModel> = ffi::get_had_model().into();
        model.map(|m| m.to_str())
    }

    #[setter]
    pub fn set_had_model(&mut self, _py: Python /* Lock */, value: Option<&str>) -> PyResult<()> {
        Self::check_state("had_model")?;
        let model: ffi::HadPhysicsModel = match value {
            None => ffi::HadPhysicsModel::None,
            Some(value) => match HadPhysicsModel::from_str(value) {
                Ok(model) => model.into(),
                Err(options) => {
                    return Err(variant_error("bad model", value, options))
                },
            },
        };
        ffi::set_had_model(model);
        Ok(())
    }

    /// Macroscopic cross-section of a process, in 1/cm.
    #[pyo3(signature=(*, pid, material, energy, process, cut=None))]
    pub fn cross_section<'py>(
        &self,
        py: Python<'py>,
        pid: PidArg,
        material: String,
        energy: FloatArg<'py>,
        process: &str,
        cut: Option<CutArg>,
    ) -> PyResult<Py<PyAny>> {
        let pid: i32 = pid.try_into()?;
        let (range_cut, energy_cut) = cut.unwrap_or(CutArg::FALSE).unpack(0.0);
        let mut calc = ffi::create_physics_calculator(pid, material, process);
        ffi::get_error().take_result()?;
        let result = match energy {
            FloatArg::Scalar(energy) => match energy_cut {
                FloatArg::Scalar(energy_cut) => {
                    let s = calc.as_mut().unwrap().cross_section(energy, range_cut, energy_cut);
                    s.to_object(py)
                },
                FloatArg::Array(energy_cut) => {
                    let result = PyArray::<f64>::empty(py, &energy_cut.shape())?;
                    for i in 0..energy_cut.size() {
                        let ci = energy_cut.get(i)?;
                        let si = calc.as_mut().unwrap().cross_section(energy, range_cut, ci);
                        result.set(i, si)?;
                    }
                    result.into_any().unbind()
                },
            },
            FloatArg::Array(energy) => {
                if let FloatArg::Array(a) = energy_cut {
                    if a.size() != energy.size() {
                        let why = format!(
                            "expected a size {} array, found size {}",
                            energy.size(),
                            a.size(),
                        );
                        let err = Error::new(ValueError).what("cut").why(&why);
                        return Err(err.into())
                    }
                }
                let result = PyArray::<f64>::empty(py, &energy.shape())?;
                for i in 0..energy.size() {
                    let ei = energy.get(i)?;
                    let ci = match energy_cut {
                        FloatArg::Scalar(s) => s,
                        FloatArg::Array(a) => a.get(i)?,
                    };
                    let si = calc.as_mut().unwrap().cross_section(ei, range_cut, ci);
                    result.set(i, si)?;
                }
                result.into_any().unbind()
            },
        };
        Ok(result)
    }

    /// Range (cm) to energy (MeV) cut conversion.
    #[pyo3(signature=(*, pid, material, range_cut=None))]
    pub fn energy_cut<'py>(
        &self,
        py: Python<'py>,
        pid: PidArg,
        material: String,
        range_cut: Option<FloatArg<'py>>,
    ) -> PyResult<Py<PyAny>> {
        let pid: i32 = pid.try_into()?;
        let mut calc = ffi::create_physics_calculator(pid, material, "");
        ffi::get_error().take_result()?;
        if !calc.production_cuts() {
            let why = format!(
                "no production cuts for pid={}",
                pid,
            );
            let err = Error::new(ValueError).what("pid").why(&why);
            return Err(err.into())
        }

        let result = match range_cut {
            Some(range_cut) => match range_cut {
                FloatArg::Scalar(range_cut) => {
                    let e = calc.as_mut().unwrap().energy_cut(false, range_cut);
                    e.to_object(py)
                },
                FloatArg::Array(range_cut) => {
                    let result = PyArray::<f64>::empty(py, &range_cut.shape())?;
                    for i in 0..range_cut.size() {
                        let ri = range_cut.get(i)?;
                        let ei = calc.as_mut().unwrap().energy_cut(false, ri);
                        result.set(i, ei)?;
                    }
                    result.into_any().unbind()
                },
            },
            None => {
                let e = calc.as_mut().unwrap().energy_cut(true, 0.0);
                e.to_object(py)
            },
        };
        Ok(result)
    }

    /// Active particles for the physics.
    #[pyo3(signature=())]
    pub fn particles<'py>(&self, _py: Python<'py>) -> PyResult<HashMap<String, i32>> {
        let mut particles = Vec::new();
        ffi::get_active_particles(&mut particles);
        ffi::get_error().take_result()?;
        let particles: HashMap<String, i32> = particles
            .into_iter()
            .map(|e| (e.name, e.pid))
            .collect();
        Ok(particles)
    }

    // XXX Use nicknames for processes, as in tracking.
    /// Active processes for a particle.
    #[pyo3(signature=(pid))]
    pub fn processes<'py>(&self, _py: Python<'py>, pid: PidArg) -> PyResult<Vec<String>> {
        let pid: i32 = pid.try_into()?;
        let mut processes = Vec::new();
        ffi::get_active_processes(pid, &mut processes);
        ffi::get_error().take_result()?;
        Ok(processes)
    }

    /// Range of a particle, in cm.
    #[pyo3(signature=(*, pid, material, energy, cut=None))]
    pub fn range<'py>(
        &self,
        py: Python<'py>,
        pid: PidArg,
        material: String,
        energy: FloatArg<'py>,
        cut: Option<bool>,
    ) -> PyResult<Py<PyAny>> {
        let pid: i32 = pid.try_into()?;
        let cut = cut.unwrap_or(false);
        let mut calc = ffi::create_physics_calculator(pid, material, "");
        ffi::get_error().take_result()?;
        let result = match energy {
            FloatArg::Scalar(energy) => {
                let r = calc.as_mut().unwrap().range(energy, cut);
                r.to_object(py)
            },
            FloatArg::Array(energy) => {
                let result = PyArray::<f64>::empty(py, &energy.shape())?;
                for i in 0..energy.size() {
                    let ei = energy.get(i)?;
                    let ri = calc.as_mut().unwrap().range(ei, cut);
                    result.set(i, ri)?;
                }
                result.into_any().unbind()
            },
        };
        Ok(result)
    }

    /// Stopping-power of a material, in MeV / cm.
    #[pyo3(signature=(*, pid, material, energy, process=None, cut=None))]
    pub fn stopping_power<'py>(
        &self,
        py: Python<'py>,
        pid: PidArg,
        material: String,
        energy: FloatArg<'py>,
        process: Option<&str>,
        cut: Option<CutArg>,
    ) -> PyResult<Py<PyAny>> {
        let pid: i32 = pid.try_into()?;
        let (process, range_cut, energy_cut) = match process {
            Some(process) => {
                let energy_cut = match cut {
                    Some(cut) => match cut {
                        CutArg::Flag(_) => {
                            let why = "expected a scalar or an array, found a boolean";
                            let err = Error::new(TypeError).what("cut").why(why);
                            return Err(err.into())
                        },
                        CutArg::Scalar(s) => FloatArg::Scalar(s),
                        CutArg::Array(a) => FloatArg::Array(a),
                    },
                    None => FloatArg::Scalar(f64::MAX),
                };
                (process, false, energy_cut)
            },
            None => {
                let range_cut = match cut {
                    Some(cut) => match cut {
                        CutArg::Flag(b) => b,
                        _ => {
                            let found = match cut {
                                CutArg::Scalar(_) => "a scalar",
                                CutArg::Array(_) => "an array",
                                _ => unreachable!(),
                            };
                            let why = format!("expected a boolean, found {}", found);
                            let err = Error::new(TypeError).what("cut").why(&why);
                            return Err(err.into())
                        },
                    },
                    None => false,
                };
                ("", range_cut, FloatArg::Scalar(0.0))
            },
        };
        let mut calc = ffi::create_physics_calculator(pid, material, process);
        ffi::get_error().take_result()?;
        let result = match energy {
            FloatArg::Scalar(energy) => match energy_cut {
                FloatArg::Scalar(energy_cut) => {
                    let s = calc.as_mut().unwrap().stopping_power(energy, range_cut, energy_cut);
                    s.to_object(py)
                },
                FloatArg::Array(energy_cut) => {
                    let result = PyArray::<f64>::empty(py, &energy_cut.shape())?;
                    for i in 0..energy_cut.size() {
                        let ci = energy_cut.get(i)?;
                        let si = calc.as_mut().unwrap().stopping_power(energy, range_cut, ci);
                        result.set(i, si)?;
                    }
                    result.into_any().unbind()
                },
            },
            FloatArg::Array(energy) => {
                if let FloatArg::Array(a) = energy_cut {
                    if a.size() != energy.size() {
                        let why = format!(
                            "expected a size {} array, found size {}",
                            energy.size(),
                            a.size(),
                        );
                        let err = Error::new(ValueError).what("cut").why(&why);
                        return Err(err.into())
                    }
                }
                let result = PyArray::<f64>::empty(py, &energy.shape())?;
                for i in 0..energy.size() {
                    let ei = energy.get(i)?;
                    let ci = match energy_cut {
                        FloatArg::Scalar(s) => s,
                        FloatArg::Array(a) => a.get(i)?,
                    };
                    let si = calc.as_mut().unwrap().stopping_power(ei, range_cut, ci);
                    result.set(i, si)?;
                }
                result.into_any().unbind()
            },
        };
        Ok(result)
    }
}

impl Physics {
    fn check_state(model: &str) -> PyResult<()> {
        if ffi::simulation_state() == ffi::G4ApplicationState::G4State_PreInit {
            Ok(())
        } else {
            let why = "cannot modify physics model after Geant4 init";
            let err = Error::new(TypeError).what(model).why(why);
            Err(err.into())
        }
    }
}

#[derive(FromPyObject)]
pub enum CutArg<'py> {
    Flag(bool),
    Scalar(f64),
    Array(&'py PyArray<f64>),
}

#[derive(FromPyObject)]
pub enum FloatArg<'py> {
    Scalar(f64),
    Array(&'py PyArray<f64>),
}

impl<'py> CutArg<'py> {
    const FALSE: Self = Self::Flag(false);

    fn unpack(self, default: f64) -> (bool, FloatArg<'py>) {
        match self {
            Self::Flag(b) => if b {
                (true, FloatArg::Scalar(0.0))
            } else {
                (false, FloatArg::Scalar(default))
            },
            Self::Scalar(s) => {
                (false, FloatArg::Scalar(s))
            },
            Self::Array(a) => {
                (false, FloatArg::Array(a))
            },
        }
    }
}


// ===============================================================================================
//
// Conversion utilities.
//
// ===============================================================================================

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
enum EmPhysicsModel {
    Dna,
    Livermore,
    Option1,
    Option2,
    Option3,
    Option4,
    Penelope,
    Standard,
}

impl From<ffi::EmPhysicsModel> for Option<EmPhysicsModel> {
    fn from(value: ffi::EmPhysicsModel) -> Self {
        match value {
            ffi::EmPhysicsModel::Dna => Some(EmPhysicsModel::Dna),
            ffi::EmPhysicsModel::Livermore => Some(EmPhysicsModel::Livermore),
            ffi::EmPhysicsModel::None => None,
            ffi::EmPhysicsModel::Option1 => Some(EmPhysicsModel::Option1),
            ffi::EmPhysicsModel::Option2 => Some(EmPhysicsModel::Option2),
            ffi::EmPhysicsModel::Option3 => Some(EmPhysicsModel::Option3),
            ffi::EmPhysicsModel::Option4 => Some(EmPhysicsModel::Option4),
            ffi::EmPhysicsModel::Penelope => Some(EmPhysicsModel::Penelope),
            ffi::EmPhysicsModel::Standard => Some(EmPhysicsModel::Standard),
            _ => unreachable!(),
        }
    }
}

impl From<EmPhysicsModel> for ffi::EmPhysicsModel {
    fn from(value: EmPhysicsModel) -> Self {
        match value {
            EmPhysicsModel::Dna => Self::Dna,
            EmPhysicsModel::Livermore => Self::Livermore,
            EmPhysicsModel::Option1 => Self::Option1,
            EmPhysicsModel::Option2 => Self::Option2,
            EmPhysicsModel::Option3 => Self::Option3,
            EmPhysicsModel::Option4 => Self::Option4,
            EmPhysicsModel::Penelope => Self::Penelope,
            EmPhysicsModel::Standard => Self::Standard,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="upper_case")]
pub enum HadPhysicsModel {
    FTFP_BERT,
    FTFP_BERT_HP,
    QGSP_BERT,
    QGSP_BERT_HP,
    QGSP_BIC,
    QGSP_BIC_HP,
}

impl From<ffi::HadPhysicsModel> for Option<HadPhysicsModel> {
    fn from(value: ffi::HadPhysicsModel) -> Self {
        match value {
            ffi::HadPhysicsModel::FTFP_BERT => Some(HadPhysicsModel::FTFP_BERT),
            ffi::HadPhysicsModel::FTFP_BERT_HP => Some(HadPhysicsModel::FTFP_BERT_HP),
            ffi::HadPhysicsModel::QGSP_BERT => Some(HadPhysicsModel::QGSP_BERT),
            ffi::HadPhysicsModel::QGSP_BERT_HP => Some(HadPhysicsModel::QGSP_BERT_HP),
            ffi::HadPhysicsModel::QGSP_BIC => Some(HadPhysicsModel::QGSP_BIC),
            ffi::HadPhysicsModel::QGSP_BIC_HP => Some(HadPhysicsModel::QGSP_BIC_HP),
            ffi::HadPhysicsModel::None => None,
            _ => unreachable!(),
        }
    }
}

impl From<HadPhysicsModel> for ffi::HadPhysicsModel {
    fn from(value: HadPhysicsModel) -> Self {
        match value {
            HadPhysicsModel::FTFP_BERT => Self::FTFP_BERT,
            HadPhysicsModel::FTFP_BERT_HP => Self::FTFP_BERT_HP,
            HadPhysicsModel::QGSP_BERT => Self::QGSP_BERT,
            HadPhysicsModel::QGSP_BERT_HP => Self::QGSP_BERT_HP,
            HadPhysicsModel::QGSP_BIC => Self::QGSP_BIC,
            HadPhysicsModel::QGSP_BIC_HP => Self::QGSP_BIC_HP,
        }
    }
}
