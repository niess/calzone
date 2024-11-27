use crate::geometry::{Geometry, Volume};
use crate::utils::error::{ctrlc_catched, Error, variant_explain};
use crate::utils::error::ErrorKind::{KeyboardInterrupt, KeyError, NotImplementedError, TypeError,
                                     ValueError};
use crate::utils::float::f64x3;
use crate::utils::numpy::{Dtype, PyArray, PyArrayMethods, ShapeArg};
use cxx::{SharedPtr, UniquePtr};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyType};
use pyo3::exceptions::PyKeyError;
use super::ffi;
use super::random::{Random, RandomContext};


const DEFAULT_PID: i32 = 22; // A photon.


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
    let array = PyArray::<ffi::Particle>::zeros(py, &shape)?;
    let array = array.into_any();
    let mut has_direction = false;
    let mut has_energy = false;
    let mut has_pid = false;
    if let Some(kwargs) = kwargs {
        for (key, value) in kwargs.iter() {
            let key: String = key.extract()?;
            match key.as_str() {
                "direction" => { has_direction = true; },
                "energy" => { has_energy = true; },
                "pid" => {
                    has_pid = true;
                    if let Ok(particle) = value.extract::<ParticleName>() {
                        array.set_item(&key, i32::try_from(particle)?)?;
                        continue;
                    }
                },
                _ => {},
            }
            array.set_item(&key, value)
                .map_err(|err| {
                    let why = err.value(py).to_string();
                    Error::new(ValueError)
                        .what(&key)
                        .why(&why)
                        .to_err()
                })?;
        }
    }
    if !has_direction {
        array.set_item("direction", (0.0, 0.0, 1.0))?;
    }
    if !has_energy {
        array.set_item("energy", 1.0)?;
    }
    if !has_pid {
        array.set_item("pid", DEFAULT_PID)?;
    }
    Ok(array.unbind())
}


// ===============================================================================================
//
// Primaries iterator.
//
// ===============================================================================================

pub struct ParticlesIterator<'a> {
    energy: Property<'a, f64>,
    position: &'a PyArray<f64>,
    direction: &'a PyArray<f64>,
    pid: Option<Property<'a, i32>>,
    weight: Option<Property<'a, f64>>,
    size: usize,
    index: usize,
}

impl<'a> ParticlesIterator<'a> {
    pub fn new<'py: 'a>(elements: &'a Bound<'py, PyAny>) -> PyResult<Self> {
        let energy = Property::new(elements, "energy")?;
        let position = extract(elements, "position")?;
        let direction = extract(elements, "direction")?;
        let pid = Property::maybe_new(elements, "pid")?;
        let weight = Property::maybe_new(elements, "weight")?;
        if *position.shape().last().unwrap_or(&0) != 3 {
            let why = format!("expected a shape '[..,3]' array, found '{:?}'", position.shape());
            let err = Error::new(ValueError)
                .what("particles position")
                .why(&why)
                .to_err();
            return Err(err);
        }
        if *direction.shape().last().unwrap_or(&0) != 3 {
            let why = format!("expected a shape '[..,3]' array, found '{:?}'", direction.shape());
            let err = Error::new(ValueError)
                .what("particles direction")
                .why(&why)
                .to_err();
            return Err(err);
        }
        let size = energy.size();
        let others = [
            position.size() / 3,
            direction.size() / 3,
            pid.map(|a| a.size()).unwrap_or(size),
            weight.map(|a| a.size()).unwrap_or(size),
        ];
        if others.iter().any(|x| *x != size) {
            let err = Error::new(ValueError)
                .what("particles")
                .why("differing arrays sizes")
                .to_err();
            return Err(err);
        }
        let index = 0;
        let iter = Self {
            energy, position, direction, pid, weight, size, index
        };
        Ok(iter)
    }

    fn get(&self, index: usize) -> PyResult<(ffi::Particle, f64)> {
        let pid = match self.pid {
            None => DEFAULT_PID,
            Some(pid) => pid.get(index)?,
        };
        let particle = ffi::Particle {
            pid,
            energy: self.energy.get(index)?,
            position: [
                self.position.get(3 * index)?,
                self.position.get(3 * index + 1)?,
                self.position.get(3 * index + 2)?,
            ],
            direction: [
                self.direction.get(3 * index)?,
                self.direction.get(3 * index + 1)?,
                self.direction.get(3 * index + 2)?,
            ],
        };
        let weight = match self.weight {
            None => 1.0,
            Some(weight) => weight.get(index)?,
        };
        Ok((particle, weight))
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

fn extract<'a, 'py, T>(elements: &'a Bound<'py, PyAny>, key: &str) -> PyResult<&'a PyArray<T>>
where
    'py: 'a,
    T: Dtype,
{
    let py = elements.py();
    elements.get_item(key)
        .map_err(|err| {
            Error::new(KeyError)
                .what("particles")
                .why(&err.value_bound(py).to_string()).to_err()
        })?
        .extract()
        .map_err(|err| {
            Error::new(TypeError)
                .what(key)
                .why(&err.value_bound(py).to_string()).to_err()
        })
}

impl<'a> Iterator for ParticlesIterator<'a> {
    type Item = PyResult<(ffi::Particle, f64)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.size {
            let item = Some(self.get(self.index));
            self.index += 1;
            item
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum Property<'a, T>
where
    T: Copy + Dtype
{
    Array(&'a PyArray<T>),
    Scalar(T),
}

impl <'a, T> Property<'a, T>
where
    T: Copy + Dtype + FromPyObject<'a>
{
    fn get(&self, index: usize) -> PyResult<T> {
        match self {
            Self::Array(array) => array.get(index),
            Self::Scalar(value) => Ok(*value),
        }
    }

    fn maybe_new<'py: 'a>(elements: &'a Bound<'py, PyAny>, key: &str) -> PyResult<Option<Self>> {
        let py = elements.py();
        match Self::new(elements, key) {
            Ok(property) => Ok(Some(property)),
            Err(err) => if err.get_type_bound(py).is(&PyType::new_bound::<PyKeyError>(py)) {
                Ok(None)
            } else {
                Err(err)
            },
        }
    }

    fn new<'py: 'a>(elements: &'a Bound<'py, PyAny>, key: &str) -> PyResult<Self> {
        let py = elements.py();
        let value = elements.get_item(key)
            .map_err(|err| {
                Error::new(KeyError)
                    .what("particles")
                    .why(&err.value_bound(py).to_string()).to_err()
            })?;
        let maybe_scalar: PyResult<T> = value.extract();
        match maybe_scalar {
            Ok(value) => Ok(Self::Scalar(value)),
            Err(_) => {
                let array = value
                    .extract()
                    .map_err(|err| {
                        Error::new(TypeError)
                            .what(key)
                            .why(&err.value_bound(py).to_string()).to_err()
                    })?;
                Ok(Self::Array(array))
            },
        }
    }

    fn size(&self) -> usize {
        match self {
            Self::Array(array) => array.size(),
            Self::Scalar(_) => 1,
        }
    }
}


// ===============================================================================================
//
// Generator interface.
//
// ===============================================================================================

#[pyclass(module="calzone")]
pub struct ParticlesGenerator {
    random: Py<Random>,
    geometry: Option<SharedPtr<ffi::GeometryBorrow>>,
    // Configuration.
    direction: Direction,
    energy: Energy,
    pid: Option<i32>,
    position: Position,
    // Weight flags.
    weight: bool,
    weight_direction: Option<bool>,
    weight_energy: Option<bool>,
    weight_position: Option<bool>,
}

#[derive(Default)]
enum Direction {
    #[default]
    None,
    Point([f64; 3]),
    SolidAngle { phi: [f64; 2], cos_theta: [f64; 2] },
}

#[derive(Default)]
enum Energy {
    #[default]
    None,
    Point(f64),
    PowerLaw { energy_min: f64, energy_max: f64, exponent: f64 },
    Spectrum { lines: Vec<EmissionLine>, total_intensity: f64 },
}

struct EmissionLine {
    energy: f64,
    intensity: f64,
}

#[derive(Default)]
enum Position {
    Inside { volume: Volume, include_daughters: bool },
    #[default]
    None,
    Onto { volume: Volume, direction: Option<DirectionArg> },
    Point([f64; 3]),
}

#[pymethods]
impl ParticlesGenerator {
    #[new]
    #[pyo3(signature=(*, geometry=None, random=None, weight=None))]
    pub fn new<'py>(
        py: Python<'py>,
        geometry: Option<&Bound<'py, Geometry>>,
        random: Option<Bound<'py, Random>>,
        weight: Option<bool>,
    ) -> PyResult<Self> {
        let weight = weight.unwrap_or(false);
        let random = match random {
            None => Py::new(py, Random::new(None, None)?)?,
            Some(random) => random.unbind(),
        };
        let geometry = geometry.map(|geometry| geometry.borrow().0.clone());
        let generator = Self {
            random,
            geometry,
            direction: Direction::default(),
            energy: Energy::default(),
            pid: None,
            position: Position::default(),
            weight,
            weight_direction: None,
            weight_energy: None,
            weight_position: None,
        };
        Ok(generator)
    }

    /// Fix the Monte Carlo particles direction.
    #[pyo3(signature=(value, /))]
    fn direction<'py>(
        slf: Bound<'py, Self>,
        value: [f64; 3],
    ) -> PyResult<Bound<'py, Self>> {
        let mut generator = slf.borrow_mut();
        generator.direction = Direction::Point(value);
        generator.weight_direction = Some(false);
        Ok(slf)
    }

    /// Fix the Monte Carlo particles kinetic energy.
    #[pyo3(signature=(value, /))]
    fn energy<'py>(
        slf: Bound<'py, Self>,
        value: f64,
    ) -> PyResult<Bound<'py, Self>> {
        let mut generator = slf.borrow_mut();
        generator.energy = Energy::Point(value);
        generator.weight_energy = Some(false);
        Ok(slf)
    }

    /// Generate Monte Carlo particles according to the current settings.
    #[pyo3(signature=(shape=None, /))]
    fn generate<'py>(&self, py: Python<'py>, shape: Option<ShapeArg>) -> PyResult<PyObject> {
        // Check configuration.
        if let Position::Onto { direction, .. } = self.position {
            if let Some(direction) = direction {
                match &self.direction {
                    Direction::None => (),
                    _ => {
                        let why = format!(
                            "'{}' conflicts with 'on/{}'",
                            self.direction.display(),
                            direction.to_str(),
                        );
                        let err = Error::new(ValueError)
                            .what("configuration")
                            .why(&why);
                        return Err(err.to_err())
                    },
                }
            }
        }

        // Create particles container.
        let shape: Vec<usize> = match shape {
            Some(shape) => shape.into(),
            None => Vec::new(),
        };
        let array = PyArray::<ffi::SampledParticle>::zeros(py, &shape)?;
        let particles = unsafe { array.slice_mut()? };
        let any_weight = {
            let direction = self.weight_direction.unwrap_or(self.weight);
            let energy = self.weight_energy.unwrap_or(self.weight);
            let position = self.weight_position.unwrap_or(self.weight);
            direction || energy || position
        };

        // Prepare specific generators.
        let (mut inside, onto) = match &self.position {
            Position::Inside { volume, include_daughters } => {
                let inside = InsideGenerator::new(volume, *include_daughters);
                (Some(inside), None)
            },
            Position::Onto { volume, direction } => {
                let weight = self.weight_position.unwrap_or(self.weight);
                let onto = OntoGenerator::new(volume, *direction, weight);
                (None, Some(onto))
            },
            _ => (None, None),
        };

        // Bind PRNG.
        let mut binding = self.random.bind(py).borrow_mut();
        let mut random = RandomContext::new(&mut binding);

        // Loop over events.
        for (event, primary) in particles.iter_mut().enumerate() {
            primary.event = event;
            primary.tid = 1;
            primary.random_index = random.index();
            let particle = &mut primary.state;

            if (event % 1000) == 0 && ctrlc_catched() {
                return Err(Error::new(KeyboardInterrupt).to_err())
            }
            particle.pid = match self.pid {
                None => DEFAULT_PID,
                Some(pid) => pid,
            };

            let mut weight = 1.0;

            let direction = match self.position {
                Position::Inside { .. } => {
                    particle.position = inside.as_mut().unwrap().generate(random.get())?;
                    false
                },
                Position::None => false,
                Position::Point(position) => {
                    particle.position = position;
                    false
                },
                Position::Onto { .. } => onto.as_ref().unwrap().generate(
                    &mut random,
                    particle,
                    &mut weight,
                ),
            };

            if !direction {
                self.generate_direction(random.get(), particle, &mut weight);
            }
            self.generate_energy(random.get(), particle, &mut weight);

            primary.weight = if any_weight {
                 weight
            } else {
                1.0
            };
        }

        // Apply volume weight, if needed.
        if any_weight {
            if self.weight_position.unwrap_or(self.weight) {
                if let Position::Inside { volume, include_daughters } = &self.position {
                    let has_volume = if *include_daughters {
                        volume.properties.has_cubic_volume
                    } else {
                        volume.properties.has_exclusive_volume
                    };
                    let cubic_volume = match has_volume {
                        true => volume.volume.compute_volume(*include_daughters),
                        false => inside.as_ref().unwrap().compute_volume(),
                    };
                    for primary in particles.iter_mut() {
                        primary.weight *= cubic_volume;
                    }
                }
            }
        }

        // Return result.
        Ok(array.into_any().unbind())
    }

    /// Set particles positions to be distributed inside a volume.
    #[pyo3(signature=(volume, /, *, include_daughters=None, weight=None))]
    #[pyo3(text_signature="(volume, /, *, include_daughters=False, weight=None)")]
    fn inside<'py>(
        slf: Bound<'py, Self>,
        volume: VolumeArg,
        include_daughters: Option<bool>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let include_daughters = include_daughters.unwrap_or(false);
        let mut generator = slf.borrow_mut();
        let volume = volume.resolve(generator.geometry.as_ref())?;
        generator.weight_position = weight;
        generator.position = Position::Inside { volume, include_daughters };
        Ok(slf)
    }

    /// Set particles positions to be distributed on a volume surface.
    #[pyo3(signature=(volume, /, direction=None, *, weight=None))]
    fn on<'py>(
        slf: Bound<'py, Self>,
        volume: VolumeArg,
        direction: Option<String>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let direction = direction
            .map(|direction| DirectionArg::from_str(direction.as_str())
                .map_err(|options| {
                    let why = variant_explain(direction.as_str(), options);
                    Error::new(ValueError).what("direction").why(&why).to_err()
                })
            )
            .transpose()?;
        let mut generator = slf.borrow_mut();
        let volume = volume.resolve(generator.geometry.as_ref())?;
        let weight_position = generator.weight_position.unwrap_or(generator.weight);
        if !volume.properties.has_surface_generation ||
            (weight_position && !volume.properties.has_surface_area) {
            let why = format!("not implemented for '{}'", volume.solid);
            let err = Error::new(NotImplementedError)
                .what("'on' operation")
                .why(&why);
            return Err(err.to_err());
        }
        generator.weight_position = weight;
        generator.position = Position::Onto { volume, direction };
        Ok(slf)
    }

    /// Fix the Monte Carlo particles type.
    #[pyo3(signature=(value, /))]
    fn pid<'py>(
        slf: Bound<'py, Self>,
        value: PidArg,
    ) -> PyResult<Bound<'py, Self>> {
        let mut generator = slf.borrow_mut();
        generator.pid = Some(value.try_into()?);
        Ok(slf)
    }

    /// Fix the Monte Carlo particles position.
    #[pyo3(signature=(value, /))]
    fn position<'py>(
        slf: Bound<'py, Self>,
        value: [f64; 3],
    ) -> PyResult<Bound<'py, Self>> {
        let mut generator = slf.borrow_mut();
        generator.position = Position::Point(value);
        generator.weight_position = Some(false);
        Ok(slf)
    }

    /// Set particles kinetic energy to follow a power-law.
    #[pyo3(signature=(energy_min, energy_max, /, *, exponent=None, weight=None))]
    fn powerlaw<'py>(
        slf: Bound<'py, Self>,
        energy_min: f64,
        energy_max: f64,
        exponent: Option<f64>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        if energy_min >= energy_max || energy_min <= 0.0 {
            let why = "expected energy_max > energy_min > 0.0";
            let err = Error::new(ValueError).what("powerlaw").why(why);
            return Err(err.to_err());
        }
        let exponent = exponent.unwrap_or(-1.0);
        let mut generator = slf.borrow_mut();
        generator.weight_energy = weight;
        generator.energy = Energy::PowerLaw { energy_min, energy_max, exponent };
        Ok(slf)
    }

    /// Set particles direction to be distributed over a solid-angle.
    #[pyo3(signature=(theta=None, phi=None, *, weight=None))]
    fn solid_angle<'py>(
        slf: Bound<'py, Self>,
        theta: Option<[f64; 2]>,
        phi: Option<[f64; 2]>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let ascending = |[a, b]: [f64; 2]| if a <= b { [ a, b ] } else { [ b, a ] };
        let theta = theta.map(ascending);
        let phi = phi.map(ascending);
        let cos_theta = match theta {
            None => [-1.0, 1.0],
            Some([th0, th1]) => {
                check_angle(th0, 0.0, 180.0, "theta")?;
                check_angle(th1, 0.0, 180.0, "theta")?;
                let th0 = th0 * Self::RAD;
                let th1 = th1 * Self::RAD;
                [th1.cos(), th0.cos()]
            },
        };
        let phi = match phi {
            None => [-180.0, 180.0],
            Some([ph0, ph1]) => {
                check_angle(ph0, -180.0, 180.0, "phi")?;
                check_angle(ph1, -180.0, 180.0, "phi")?;
                [ph0, ph1]
            },
        };

        let mut generator = slf.borrow_mut();
        generator.weight_direction = weight;
        generator.direction = Direction::SolidAngle { phi, cos_theta };
        Ok(slf)
    }

    /// Set particles kinetic energy to be distributed according to spectral lines.
    #[pyo3(signature=(data, /, *, weight=None))]
    fn spectrum<'py>(
        slf: Bound<'py, Self>,
        data: Vec<[f64; 2]>,
        weight: Option<bool>,
    ) -> PyResult<Bound<'py, Self>> {
        let lines: Vec<EmissionLine> = data.iter()
            .filter_map(|[energy, intensity]| if *intensity > 0.0 {
                Some(EmissionLine { energy: *energy, intensity: *intensity })
            } else {
                None
            })
            .collect();
        let total_intensity: f64 = lines.iter()
            .map(|line| line.intensity)
            .sum();
        if total_intensity <= 0.0 {
            let err = Error::new(ValueError)
                .what("data")
                .why("no positive intensity");
            return Err(err.to_err());
        }

        let mut generator = slf.borrow_mut();
        generator.weight_energy = weight;
        generator.energy = Energy::Spectrum { lines, total_intensity };
        Ok(slf)
    }
}

fn check_angle(value: f64, min: f64, max: f64, what: &str) -> PyResult<()> {
    if (value < min) || (value > max) {
        let why = format!(
            "expected a value in [{}, {}], found {}",
            min,
            max,
            value,
        );
        let err = Error::new(ValueError)
            .what(what)
            .why(&why);
        Err(err.to_err())
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
enum DirectionArg {
    Ingoing,
    Outgoing,
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
    ) -> PyResult<Volume> {
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
                Volume::new(geometry, &path, true)?
            },
            Self::Volume(volume) => volume.get().clone(),
        };
        Ok(volume)
    }
}

impl ParticlesGenerator {
    const RAD: f64 = std::f64::consts::PI / 180.0;

    fn generate_direction(
        &self,
        random: &mut Random,
        particle: &mut ffi::Particle,
        weight: &mut f64,
    ) {
        let (direction, w) = self.direction.generate(random);
        particle.direction = direction;
        if self.weight_direction.unwrap_or(self.weight) {
            *weight *= w;
        }
    }

    fn generate_energy(
        &self,
        random: &mut Random,
        particle: &mut ffi::Particle,
        weight: &mut f64,
    ) {
        let (energy, w) = self.energy.generate(random);
        particle.energy = energy;
        if self.weight_energy.unwrap_or(self.weight) {
            *weight *= w;
        }
    }
}

impl Direction {
    const RAD: f64 = std::f64::consts::PI / 180.0;

    fn display(&self) -> &'static str {
        match self {
            Self::None => unreachable!(),
            Self::Point(_) => "direction",
            Self::SolidAngle { .. } => "solid_angle",
        }
    }

    fn generate(&self, random: &mut Random) -> ([f64; 3], f64) {
        match self {
            Self::Point(direction) => (*direction, 1.0),
            _ => self.generate_solid_angle(random),
        }
    }

    fn generate_solid_angle(&self, random: &mut Random) -> ([f64; 3], f64) {
        let (phi, cos_theta) = match self {
            Direction::SolidAngle { phi, cos_theta, .. } => (*phi, *cos_theta),
            _ => ([-180.0, 180.0], [-1.0, 1.0]),
        };
        let [ph0, ph1] = phi;
        let [cos_th0, cos_th1] = cos_theta;
        let cos_theta = random.uniform(cos_th0, cos_th1);
        let phi = random.uniform(ph0, ph1) * Self::RAD;
        let solid_angle = (cos_th1 - cos_th0).abs() * (ph1 - ph0).abs() * Self::RAD;
        let sin_theta = (1.0 - cos_theta * cos_theta)
            .max(0.0)
            .sqrt();
        let direction = [
            sin_theta * phi.cos(),
            sin_theta * phi.sin(),
            cos_theta,
        ];
        (direction, solid_angle)
    }
}

impl Energy {
    fn generate(&self, random: &mut Random) -> (f64, f64) {
        match self {
            Self::None => (1E+00, 1.0),
            Self::Point(value) => (*value, 1.0),
            Self::PowerLaw { .. } => self.generate_powerlaw(random),
            Self::Spectrum { .. } => self.generate_spectrum(random),
        }
    }

    fn generate_powerlaw(&self, random: &mut Random) -> (f64, f64) {
        let Energy::PowerLaw { energy_min, energy_max, exponent } = *self else { unreachable!() };
        let (energy, weight) = if exponent == -1.0 {
            let lne = (energy_max / energy_min).ln();
            let energy = energy_min * (random.open01() * lne).exp();
            (energy, energy * lne)
        } else if exponent == 0.0 {
            let de = energy_max - energy_min;
            let energy = de * random.open01() + energy_min;
            (energy, de)
        } else {
            let a = exponent + 1.0;
            let b = energy_min.powf(a);
            let de = energy_max.powf(a) - b;
            let energy = (de * random.open01() + b).powf(1.0 / a);
            let weight = de / (a * energy.powf(exponent));
            (energy, weight)
        };
        let energy = energy.clamp(energy_min, energy_max);
        (energy, weight)
    }

    fn generate_spectrum(&self, random: &mut Random) -> (f64, f64) {
        let Energy::Spectrum { lines, total_intensity } = self else { unreachable!() };
        let target = random.open01() * total_intensity;
        let mut acc = 0.0;
        let mut j = 0_usize;
        loop {
            let EmissionLine { energy, intensity } = lines[j];
            acc += intensity;
            if (acc >= target) || (j == lines.len() - 1) {
                let weight = total_intensity / intensity;
                return (energy, weight);
            } else {
                j += 1;
            }
        }
    }
}

struct InsideGenerator<'a> {
    volume: &'a Volume,
    include_daughters: bool,
    transform: UniquePtr<ffi::G4AffineTransform>,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    zmin: f64,
    zmax: f64,
    n: usize,
    trials: usize,
}

impl <'a> InsideGenerator<'a> {
    fn new(volume: &'a Volume, include_daughters: bool) -> Self {
        let [xmin, xmax, ymin, ymax, zmin, zmax] = volume.volume.compute_box("");
        let transform = volume.volume.compute_transform("");
        let n = 0;
        let trials = 0;
        Self {
            volume, include_daughters, transform, xmin, xmax, ymin, ymax, zmin, zmax, n, trials
        }
    }

    fn compute_volume(&self) -> f64 {
        let p = (self.n as f64) / (self.trials as f64);
        (self.xmax - self.xmin) * (self.ymax - self.ymin) * (self.zmax - self.zmin) * p
    }

    fn generate(&mut self, random: &mut Random) -> PyResult<[f64; 3]>  {
        self.n += 1;
        loop {
            self.trials += 1;
            let r = [
                random.uniform(self.xmin, self.xmax),
                random.uniform(self.ymin, self.ymax),
                random.uniform(self.zmin, self.zmax),
            ];
            if self.volume.volume.inside(
                &r,
                &self.transform,
                self.include_daughters
            ) == ffi::EInside::kInside {
                return Ok(r);
            } else if ((self.trials % 1000) == 0) && ctrlc_catched() {
                return Err(Error::new(KeyboardInterrupt).to_err());
            }
        }
    }
}

struct OntoGenerator<'a> {
    volume: &'a Volume,
    transform: UniquePtr<ffi::G4AffineTransform>,
    direction: Option<DirectionArg>,
    weight: Option<f64>,
}

impl <'a> OntoGenerator<'a> {
    fn new(volume: &'a Volume, direction: Option<DirectionArg>, weight: bool) -> Self {
        let transform = volume.volume.compute_transform("");
        let weight = if weight {
            let surface = volume.volume.compute_surface();
            let solid_angle = if direction.is_some() {
                std::f64::consts::PI
            } else {
                1.0
            };
            let weight = surface * solid_angle;
            Some(weight)
        } else {
            None
        };
        Self { volume, transform, direction, weight }
    }

    fn generate(
        &self,
        random: &mut RandomContext,
        particle: &mut ffi::Particle,
        weight: &mut f64,
    ) -> bool {
        let data = self.volume.volume.generate_onto(
            random,
            &self.transform,
            self.direction.is_some()
        );
        for j in 0..3 {
            particle.position[j] = data[j];
        }
        if let Some(direction) = self.direction.as_ref() {
            let mut direction = match direction {
                DirectionArg::Ingoing => f64x3::new(-data[3], -data[4], -data[5]),
                DirectionArg::Outgoing => f64x3::new(data[3], data[4], data[5]),
            };
            let cos_theta = random.next_open01().sqrt();
            let phi = 2.0 * std::f64::consts::PI * random.next_open01();
            direction.rotate(cos_theta, phi);
            particle.direction = direction.into();

            if let Some(w) = self.weight {
                *weight *= w;
            }
        }

        self.direction.is_some()
    }
}


// ===============================================================================================
//
// Named particles.
//
// ===============================================================================================

#[derive(FromPyObject)]
struct ParticleName (String);

impl TryFrom<ParticleName> for i32 {
    type Error = PyErr;

    fn try_from(value: ParticleName) -> PyResult<i32> {
        let pid = match value.0.as_str() {
            "e-" => 11,
            "e+" => -11,
            "mu-" => 13,
            "mu+" => -13,
            "tau-" => 15,
            "tau+" => -15,
            "gamma" => 22,
            "p" => 2212,
            "n" => 2112,
            _ => {
                let why = format!("unknown particle '{}'", value.0);
                let err = Error::new(ValueError)
                    .what("pid")
                    .why(&why);
                return Err(err.to_err());
            },
        };
        Ok(pid)
    }
}

#[derive(FromPyObject)]
enum PidArg {
    Name(ParticleName),
    Number(i32),
}

impl TryFrom<PidArg> for i32 {
    type Error = PyErr;

    fn try_from(value: PidArg) -> PyResult<i32> {
        match value {
            PidArg::Name(name) => name.try_into(),
            PidArg::Number(pid) => Ok(pid),
        }
    }
}
