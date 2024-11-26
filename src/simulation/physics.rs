use crate::utils::error::{Error, variant_error};
use crate::utils::error::ErrorKind::ValueError;
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use super::ffi;


// ===============================================================================================
//
// Physics interface.
//
// ===============================================================================================

/// Geant4 physics settings.
#[derive(Default)]
#[pyclass(module="calzone")]
pub struct Physics (pub(crate) ffi::Physics);

impl Physics {
    const DEFAULT_EM_MODEL: ffi::EmPhysicsModel = ffi::EmPhysicsModel::Standard;
    const DEFAULT_HAD_MODEL: ffi::HadPhysicsModel = ffi::HadPhysicsModel::None;

    pub fn none() ->  Self {
        let mut physics = ffi::Physics::default();
        physics.em_model = ffi::EmPhysicsModel::None;
        physics.had_model = ffi::HadPhysicsModel::None;
        Self (physics)
    }
}

#[pymethods]
impl Physics {
    #[new]
    #[pyo3(signature=(em_model=None, *, default_cut=None, had_model=None))]
    pub fn new(
        em_model: Option<&str>,
        default_cut: Option<f64>,
        had_model: Option<&str>,
    ) -> PyResult<Self> {
        let mut physics = Self (ffi::Physics::default());
        if let Some(em_model) = em_model {
            physics.set_em_model(Some(em_model))?;
        }
        if let Some(had_model) = had_model {
            physics.set_had_model(Some(had_model))?;
        }
        if let Some(default_cut) = default_cut {
            physics.set_default_cut(default_cut)?;
        }
        Ok(physics)
    }

    /// Physics default cut, in cm.
    #[getter]
    fn get_default_cut(&self) -> f64 {
        self.0.default_cut
    }

    #[setter]
    fn set_default_cut(&mut self, value: f64) -> PyResult<()> {
        if value > 0.0 {
            self.0.default_cut = value;
            Ok(())
        } else {
            let why = format!("expected a positive value, found {}", value);
            let err = Error::new(ValueError).what("default_cut").why(&why);
            Err(err.into())
        }
    }

    /// Electromagnetic physics list.
    #[getter]
    fn get_em_model(&self) -> Option<&'static str> {
        let model: Option<EmPhysicsModel> = self.0.em_model.into();
        model.map(|m| m.to_str())
    }

    #[setter]
    pub fn set_em_model(&mut self, value: Option<&str>) -> PyResult<()> {
        match value {
            None => {
                self.0.em_model = ffi::EmPhysicsModel::None;
                Ok(())
            },
            Some(value) => match EmPhysicsModel::from_str(value) {
                Ok(model) => {
                    self.0.em_model = model.into();
                    Ok(())
                },
                Err(options) => Err(variant_error("bad model", value, options)),
            },
        }
    }

    /// Hadronic physics list.
    #[getter]
    fn get_had_model(&self) -> Option<&'static str> {
        let model: Option<HadPhysicsModel> = self.0.had_model.into();
        model.map(|m| m.to_str())
    }

    #[setter]
    pub fn set_had_model(&mut self, value: Option<&str>) -> PyResult<()> {
        match value {
            None => {
                self.0.had_model = ffi::HadPhysicsModel::None;
                Ok(())
            },
            Some(value) => match HadPhysicsModel::from_str(value) {
                Ok(model) => {
                    self.0.had_model = model.into();
                    Ok(())
                },
                Err(options) => Err(variant_error("bad model", value, options)),
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
enum HadPhysicsModel {
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

impl Default for ffi::Physics {
    fn default() -> Self {
        let default_cut = 0.1; // cm
        let em_model = Physics::DEFAULT_EM_MODEL;
        let had_model = Physics::DEFAULT_HAD_MODEL;
        Self { default_cut, em_model, had_model }
    }
}
