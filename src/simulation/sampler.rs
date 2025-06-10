use crate::utils::error::{variant_error, variant_explain};
use crate::utils::export::Export;
use crate::utils::namespace::Namespace;
use crate::utils::numpy::{PyArray, PyArrayMethods};
use derive_more::{AsMut, AsRef, From};
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use super::ffi;


// ===========================================================================================
//
// Sampler roles interface.
//
// ===========================================================================================

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum RoleVerb {
    Catch,
    Kill,
    Record,
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
pub enum RoleSubject {
    All,
    Deposits,
    Ingoing,
    Outgoing,
    Particles,
}

struct Role {
    verb: RoleVerb,
    subject: RoleSubject,
}

impl TryFrom<&str> for Role {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (verb, subject) = value.split_once('_')
            .ok_or_else(|| "syntax error: missing verb or subject".to_string())?;
        let verb = RoleVerb::from_str(verb)
            .map_err(|options| format!("unknown verb: {}", variant_explain(verb, options)))?;
        let subject = RoleSubject::from_str(subject)
            .map_err(|options| format!("unknown subject: {}", variant_explain(subject, options)))?;
        let role = Self { verb, subject };
        Ok(role)
    }
}

impl From<RoleVerb> for ffi::Action {
    fn from(value: RoleVerb) -> Self {
        match value {
            RoleVerb::Catch => Self::Catch,
            RoleVerb::Kill => Self::Kill,
            RoleVerb::Record => Self::Record,
        }
    }
}

impl From<ffi::Action> for &'static str {
    fn from(value: ffi::Action) -> Self {
        match value {
            ffi::Action::Catch => "catch",
            ffi::Action::Kill => "kill",
            ffi::Action::Record => "record",
            _ => unreachable!(),
        }
    }
}

impl ffi::Roles {
    pub fn any(&self) -> bool {
        self.ingoing != ffi::Action::None ||
        self.outgoing != ffi::Action::None ||
        self.deposits != ffi::Action::None
    }
}

impl Default for ffi::Roles {
    fn default() -> Self {
        let deposits = ffi::Action::None;
        let ingoing = ffi::Action::None;
        let outgoing = ffi::Action::None;
        Self { deposits, ingoing, outgoing }
    }
}

impl TryFrom<&[String]> for ffi::Roles {
    type Error = String;

    fn try_from(value: &[String]) -> Result<ffi::Roles, Self::Error> {
        let mut roles = ffi::Roles::default();
        for role in value.iter() {
            let role: Role = role.as_str().try_into()?;
            let action: ffi::Action = role.verb.into();
            match role.subject {
                RoleSubject::All => {
                    roles.ingoing = action;
                    roles.outgoing = action;
                    if action == ffi::Action::Record {
                        roles.deposits = action;
                    }
                },
                RoleSubject::Deposits => {
                    if action != ffi::Action::Record {
                        let action: &'static str = action.into();
                        return Err(format!("cannot '{}' deposits", action));
                    }
                    roles.deposits = action;
                },
                RoleSubject::Ingoing => {
                    roles.ingoing = action;
                },
                RoleSubject::Outgoing => {
                    roles.outgoing = action;
                },
                RoleSubject::Particles => {
                    roles.ingoing = action;
                    roles.outgoing = action;
                },
            }
        }
        Ok(roles)
    }
}

impl From<ffi::Roles> for Vec<String> {
    fn from(roles: ffi::Roles) -> Self {
        let mut strings = Vec::<String>::new();
        if roles.deposits != ffi::Action::None {
            let verb: &'static str = roles.deposits.into();
            strings.push(format!("{}_deposits", verb));
        }
        if roles.ingoing != ffi::Action::None {
            let verb: &'static str = roles.ingoing.into();
            strings.push(format!("{}_ingoing", verb));
        }
        if roles.outgoing != ffi::Action::None {
            let verb: &'static str = roles.outgoing.into();
            strings.push(format!("{}_outgoing", verb));
        }
        strings
    }
}


// ===============================================================================================
//
// Deposits sampler interface.
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
    values: IndexMap<*const ffi::G4VPhysicalVolume, DepositsCell>,
}

impl Deposits {
    pub fn new(mode: SamplerMode) -> Self {
        let values = IndexMap::new();
        Self { mode, values }
    }

    pub fn export(mut self, py: Python) -> PyResult<PyObject> {
        let data = PyDict::new_bound(py);
        for (volume, deposits) in self.values.drain(..) {
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
        tid: i32,
        pid: i32,
        energy: f64,
        total_deposit: f64,
        point_deposit: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
        weight: f64,
        random_index: &[u64; 2],
    ) {
        self.values.entry(volume)
            .and_modify(|e| e.push(
                event, tid, pid, energy, total_deposit, point_deposit, start, end, weight,
                random_index
            ))
            .or_insert_with(|| {
                let mut cell = DepositsCell::new(self.mode);
                cell.push(
                    event, tid, pid, energy, total_deposit, point_deposit, start, end, weight,
                    random_index
                );
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
    total: IndexMap<usize, (f64, f64, [u64; 2])>,
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
                for (i, (event, (value, weight, random_index))) in deposits.total
                    .drain(..)
                    .enumerate() {
                    let deposit = TotalDeposit { event, value, weight, random_index };
                    array.set(i, deposit)?;
                }
                array.into_any().unbind()
            },
            Self::Detailed(deposits) => {
                let line = Export::export::<LineDepositsExport>(py, deposits.line)?;
                let point = Export::export::<PointDepositsExport>(py, deposits.point)?;
                Namespace::new(py, &[
                    ("line", line),
                    ("point", point),
                ])?.unbind()
            },
        };
        Ok(deposits)
    }

    fn push(
        &mut self,
        event: usize,
        tid: i32,
        pid: i32,
        energy: f64,
        total_deposit: f64,
        point_deposit: f64,
        start: &ffi::G4ThreeVector,
        end: &ffi::G4ThreeVector,
        weight: f64,
        random_index: &[u64; 2],
    ) {
        match self {
            Self::Brief(ref mut deposits) => {
                deposits.total.entry(event)
                    .and_modify(|e| { e.0 += total_deposit })
                    .or_insert((total_deposit, weight, *random_index));
            },
            Self::Detailed(ref mut deposits) => {
                let start = ffi::to_vec(start);
                let end = ffi::to_vec(end);
                let line_deposit = total_deposit - point_deposit;
                let random_index = *random_index;
                if line_deposit > 0.0 {
                    let deposit = LineDeposit {
                        event, tid, pid, energy, start, end, value: line_deposit, weight,
                        random_index
                    };
                    deposits.line.push(deposit);
                }
                if point_deposit > 0.0 {
                    let deposit = PointDeposit {
                        event, tid, pid, energy, position: end, value: point_deposit, weight,
                        random_index
                    };
                    deposits.point.push(deposit);
                }
            },
        }
    }
}


// ===========================================================================================
//
// Deposits export formats.
//
// ===========================================================================================

#[derive(Clone, Copy)]
#[repr(C)]
pub struct LineDeposit {
    event: usize,
    tid: i32,
    pid: i32,
    energy: f64,
    value: f64,
    start: [f64; 3],
    end: [f64; 3],
    weight: f64,
    random_index: [u64; 2],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PointDeposit {
    event: usize,
    tid: i32,
    pid: i32,
    energy: f64,
    value: f64,
    position: [f64; 3],
    weight: f64,
    random_index: [u64; 2],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TotalDeposit {
    event: usize,
    value: f64,
    weight: f64,
    random_index: [u64; 2],
}

#[derive(AsMut, AsRef, From)]
#[pyclass(module="calzone")]
struct LineDepositsExport (Export<LineDeposit>);

#[derive(AsMut, AsRef, From)]
#[pyclass(module="calzone")]
struct PointDepositsExport (Export<PointDeposit>);


// ===============================================================================================
//
// Particles sampler interface.
//
// ===============================================================================================

pub struct ParticlesSampler {
    samples: IndexMap<*const ffi::G4VPhysicalVolume, ParticlesCell>,
}

impl ParticlesSampler {
    pub fn new() -> Self {
        let samples = IndexMap::new();
        Self { samples }
    }

    pub fn export(mut self, py: Python) -> PyResult<PyObject> {
        let data = PyDict::new_bound(py);
        for (volume, samples) in self.samples.drain(..) {
            let volume: &ffi::G4VPhysicalVolume = unsafe { &*volume };
            let volume = ffi::as_str(volume.GetName());
            let samples = samples.export(py)?;
            data.set_item(volume, samples)?
        }
        Ok(data.into_any().unbind())
    }

    pub fn push(
        &mut self,
        volume: *const ffi::G4VPhysicalVolume,
        event: usize,
        tid: i32,
        particle: ffi::Particle,
        weight: f64,
        random_index: &[u64; 2],
    ) {
        self.samples.entry(volume)
            .and_modify(|e| e.push(event, tid, particle, weight, random_index))
            .or_insert_with(|| {
                let mut cell = ParticlesCell::new();
                cell.push(event, tid, particle, weight, random_index);
                cell
            });
    }
}


// ===========================================================================================
//
// Particles cell implementation.
//
// ===========================================================================================

#[derive(Default)]
struct ParticlesCell {
    samples: Vec<ffi::SampledParticle>,
}

impl ParticlesCell {
    fn new() -> Self {
        Self::default()
    }

    fn export(self, py: Python) -> PyResult<PyObject> {
        let samples = Export::export::<SampledParticlesExport>(py, self.samples)?;
        Ok(samples)
    }

    fn push(
        &mut self,
        event: usize,
        tid: i32,
        state: ffi::Particle,
        weight: f64,
        random_index: &[u64; 2]
    ) {
        let random_index = *random_index;
        let sample = ffi::SampledParticle { event, tid, state, weight, random_index };
        self.samples.push(sample);
    }
}


// ===========================================================================================
//
// Sampled particles export format.
//
// ===========================================================================================

#[derive(AsMut, AsRef, From)]
#[pyclass(module="calzone")]
struct SampledParticlesExport (Export<ffi::SampledParticle>);
