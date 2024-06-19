use crate::utils::error::variant_error;
use crate::utils::numpy::{PyArray, PyUntypedArray};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use super::ffi;


// ===============================================================================================
//
// Sampler interface.
//
// ===============================================================================================

#[derive(Clone, Copy, EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum SamplerMode {
    Brief,
    Detailed,
}

impl<'py> FromPyObject<'py> for SamplerMode {
    fn extract(obj: &'py PyAny) -> PyResult<Self> {
        let mode: String = obj.extract()?;
        let mode = SamplerMode::from_str(mode.as_str())
            .map_err(|options| variant_error("bad sampler mode", mode.as_str(), options))?;
        Ok(mode)
    }
}

impl IntoPy<PyObject> for SamplerMode {
    fn into_py(self, py: Python) -> PyObject {
        self.to_str().into_py(py)
    }
}

pub struct Deposits {
    mode: SamplerMode,
    values: HashMap<*const ffi::G4VPhysicalVolume, DepositsCell>,
}

impl Deposits {
    pub fn new(mode: SamplerMode) -> Self {
        let values = HashMap::new();
        Self { mode, values }
    }

    pub fn export(mut self, py: Python) -> PyResult<PyObject> {
        let data = PyDict::new_bound(py);
        for (volume, deposits) in self.values.drain() {
            let volume: &ffi::G4VPhysicalVolume = unsafe { &*volume };
            let volume = ffi::as_str(volume.GetName());
            let deposits = deposits.export(py)?;
            data.set_item(volume, deposits)?
        }
        Ok(data.into_any().unbind())
    }

    pub fn push(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        event: usize,
        deposit: f64,
        non_ionising: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
    ) {
        self.values.entry(volume)
            .and_modify(|e| e.push(event, deposit, non_ionising, start, end))
            .or_insert_with(|| {
                let mut cell = DepositsCell::new(self.mode);
                cell.push(event, deposit, non_ionising, start, end);
                cell
            });
    }
}

// ===========================================================================================
//
// Deposits implementation.
//
// ===========================================================================================

enum DepositsCell {
    Brief(BriefDeposits),
    Detailed(DetailedDeposits),
}

#[derive(Default)]
struct BriefDeposits {
    total: HashMap<usize, f64>,
}

#[derive(Default)]
struct DetailedDeposits {
    line: Vec<LineDeposit>,
    point: Vec<PointDeposit>,
}

impl DepositsCell {
    fn new(mode: SamplerMode) -> Self {
        match mode {
            SamplerMode::Brief => Self::Brief(BriefDeposits::default()),
            SamplerMode::Detailed => Self::Detailed(DetailedDeposits::default()),
        }
    }

    fn export(self, py: Python) -> PyResult<PyObject> {
        let deposits = match self {
            Self::Brief(mut deposits) => {
                let array = PyArray::<TotalDeposit>::empty(py, &[deposits.total.len()])?;
                let a = unsafe { array.slice_mut()? };
                for (i, (event, value)) in deposits.total.drain().enumerate() {
                    let mut ai = a[i];
                    ai.event = event;
                    ai.value = value;
                }
                let array: &PyUntypedArray = array.into();
                array.into_py(py)
            },
            Self::Detailed(deposits) => {
                unimplemented!()
            },
        };
        Ok(deposits)
    }

    fn push(
        &mut self,
        event: usize,
        deposit: f64,
        non_ionising: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
    ) {
        match self {
            Self::Brief(ref mut deposits) => {
                deposits.total.entry(event)
                    .and_modify(|e| { *e += deposit })
                    .or_insert(deposit);
            },
            Self::Detailed(ref mut deposits) => {
                let start = ffi::to_vec(start);
                let end = ffi::to_vec(end);
                let line_deposit = deposit - non_ionising;
                if line_deposit > 0.0 {
                    let deposit = LineDeposit { event, start, end, value: line_deposit };
                    deposits.line.push(deposit);
                }
                if non_ionising > 0.0 {
                    let deposit = PointDeposit { event, position: end, value: non_ionising };
                    deposits.point.push(deposit);
                }
            },
        }
    }
}


// ===========================================================================================
//
// Export formats.
//
// ===========================================================================================

#[derive(Clone, Copy)]
#[repr(C)]
pub struct LineDeposit {
    event: usize,
    value: f64,
    start: [f64; 3],
    end: [f64; 3],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PointDeposit {
    event: usize,
    value: f64,
    position: [f64; 3],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TotalDeposit {
    event: usize,
    value: f64,
}
