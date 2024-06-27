use crate::utils::extract::{Extractor, Property, Tag, TryFromBound};
use crate::utils::error::{Error, variant_explain};
use crate::utils::error::ErrorKind::{IndexError, ValueError};
use crate::utils::float::{f64x3, f64x3x3};
use crate::utils::io::DictLike;
use crate::utils::numpy::PyArray;
use cxx::SharedPtr;
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use super::cxx::ffi;
use temp_dir::TempDir;

mod goupil;
mod map;
pub mod materials;
pub mod volume;
pub mod tessellation;

pub use map:: Map;
pub use materials::MaterialsDefinition;


// ===============================================================================================
//
// Geometry interface.
//
// ===============================================================================================

/// A static Monte Carlo geometry.
#[pyclass(frozen, module="calzone")]
pub struct Geometry (pub(crate) SharedPtr<ffi::GeometryBorrow>);

unsafe impl Send for ffi::GeometryBorrow {}
unsafe impl Sync for ffi::GeometryBorrow {}

#[pymethods]
impl Geometry {
    #[new]
    pub fn new(volume: DictLike) -> PyResult<Self> {
        // XXX from GDML (manage memory by diffing G4SolidStore etc.).
        let builder = GeometryBuilder::new(volume)?;
        let geometry = builder.build()?;
        Ok(geometry)
    }

    fn __getitem__(&self, volume: &str) -> PyResult<Volume> {
        let ffi::VolumeInfo { material, solid, sensitive, mother, daughters } =
            self.0.describe_volume(volume);
        if let Some(msg) = ffi::get_error().value() {
            let err = Error::new(IndexError).what("volume").why(msg);
            return Err(err.into())
        }
        let mother = if mother.is_empty() {
            None
        } else {
            Some(mother)
        };
        let volume = Volume {
            geometry: self.0.clone(),
            name: volume.to_string(),
            material,
            solid,
            sensitive,
            mother,
            daughters,
        };
        Ok(volume)
    }

    /// Check the geometry by looking for overlapping volumes.
    fn check(&self, resolution: Option<i32>) -> PyResult<()> {
        let resolution = resolution.unwrap_or(1000);
        self.0
            .check(resolution)
            .to_result()?;
        Ok(())
    }

    /// Dump the geometry to a GDML file.
    fn dump(&self, path: Option<&str>) -> PyResult<()> {
        let tmp = TempDir::new()?;
        let path = path.unwrap_or("geometry.gdml");
        let tmp_path = tmp
            .child("geometry.gdml")
            .display()
            .to_string();
        self.0
            .dump(tmp_path.as_str())
            .to_result()?;
        std::fs::copy(&tmp_path, path)?;
        Ok(())
    }

    /// Export the geometry as a `goupil.ExternalGeometry` object.
    fn export<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let goupil = py.import_bound("goupil")?;
        let external_geometry = goupil.getattr("ExternalGeometry")?;
        let file = super::FILE
            .get(py)
            .unwrap();
        self.0.set_goupil();
        let args = (file,);
        external_geometry.call1(args)
    }
}

// ===============================================================================================
//
// Builder interface.
//
// ===============================================================================================

/// A Monte Carlo geometry builder.
#[pyclass(module="calzone")]
pub struct GeometryBuilder {
    definition: GeometryDefinition,
    /// Traversal algorithm for tessellated shapes.
    #[pyo3(get, set)]
    algorithm: Algorithm,
}

#[derive(Clone, Copy, Default, EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
enum Algorithm {
    #[default]
    Bvh,
    Geant4,
}

impl From<Algorithm> for ffi::TSTAlgorithm {
    fn from(value: Algorithm) -> Self {
        match value {
            Algorithm::Bvh => ffi::TSTAlgorithm::Bvh,
            Algorithm::Geant4 => ffi::TSTAlgorithm::Geant4,
        }
    }
}

impl<'py> FromPyObject<'py> for Algorithm {
    fn extract_bound(algorithm: &Bound<'py, PyAny>) -> PyResult<Self> {
        let algorithm: String = algorithm.extract()?;
        let algorithm = Algorithm::from_str(&algorithm)
            .map_err(|options| {
                let why = variant_explain(&algorithm, options);
                Error::new(ValueError).what("algorithm").why(&why).to_err()
            })?;
        Ok(algorithm)
    }
}

impl IntoPy<PyObject> for Algorithm {
    fn into_py(self, py: Python) -> PyObject {
        self.to_str().into_py(py)
    }
}

#[pymethods]
impl GeometryBuilder {
    #[new]
    fn new(definition: DictLike) -> PyResult<Self> {
        let definition = GeometryDefinition::new(definition)?;
        let algorithm = Algorithm::default();
        let builder = Self { definition, algorithm };
        Ok(builder)
    }

    /// Build the Monte Carlo `Geometry`.
    fn build(&self) -> PyResult<Geometry> {
        // build materials.
        if let Some(materials) = self.definition.materials.as_ref() {
            materials.build()?;
        }

        // Validate volumes.
        self.definition.volume.validate()?;

        // Build volumes.
        let algorithm: ffi::TSTAlgorithm = self.algorithm.into();
        let geometry = ffi::create_geometry(&self.definition.volume, &algorithm);
        if geometry.is_null() {
            ffi::get_error().to_result()?;
            unreachable!()
        }
        let geometry = Geometry (geometry);
        Ok(geometry)
    }

    /// Remove a volume from the geometry definition.
    fn delete<'py>(
        slf: Bound<'py, GeometryBuilder>,
        pathname: &str,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        if let Some((mother, name)) = pathname.rsplit_once('.') {
            let mut builder = slf.borrow_mut();
            let mother = builder.find_mut(mother)?;
            let n = mother.volumes.len();
            mother.volumes.retain(|v| v.name != name);
            if mother.volumes.len() < n {
                return Ok(slf);
            }
        }
        let builder = slf.borrow();
        let why = if builder.definition.volume.name() == pathname {
            format!("cannot delete top volume '{}'", pathname)
        } else {
            format!("unknown '{}' volume", pathname)
        };
        let err = Error::new(ValueError).what("geometry operation").why(&why);
        Err(err.into())
    }

    /// Modify the definition of a geometry volume.
    fn modify<'py>(
        slf: Bound<'py, GeometryBuilder>,
        pathname: &str,
        name: Option<String>,
        material: Option<String>,
        position: Option<f64x3>,
        rotation: Option<f64x3x3>,
        sensitive: Option<bool>,
        subtract: Option<String>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let mut builder = slf.borrow_mut();
        let volume = builder.find_mut(pathname)?;
        if let Some(name) = name {
            volume::Volume::check(&name)
                .map_err(|why| {
                    let what = format!("name '{}'", name);
                    Error::new(ValueError).what(&what).why(why).to_err()
                })?;
            volume.name = name;
        }
        if let Some(material) = material {
            volume.material = material;
        }
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation);
        }
        if let Some(sensitive) = sensitive {
            volume.sensitive = sensitive;
        }
        if let Some(subtract) = subtract {
            volume.subtract = Some(subtract);
        }
        Ok(slf)
    }

    /// (Re)place a definition of a geometry volume.
    fn place<'py>(
        slf: Bound<'py, GeometryBuilder>,
        definition: DictLike,
        mother: Option<&str>,
        position: Option<f64x3>,
        rotation: Option<f64x3x3>,
    ) -> PyResult<Bound<'py, GeometryBuilder>> {
        let GeometryDefinition { mut volume, materials } = GeometryDefinition::new(definition)?;
        if let Some(position) = position {
            volume.position = Some(position);
        }
        if let Some(rotation) = rotation {
            volume.rotation = Some(rotation);
        }
        let mut builder = slf.borrow_mut();
        let mother = match mother {
            None => &mut builder.definition.volume,
            Some(mother) => builder.find_mut(mother)?,
        };
        match mother.volumes.iter_mut().find(|v| v.name() == volume.name()) {
            None => mother.volumes.push(*volume),
            Some(v) => *v = *volume,
        }
        if let Some(materials) = materials {
            match &mut builder.definition.materials {
                None => builder.definition.materials = Some(materials),
                Some(m) => m.extend(materials),
            }
        }
        Ok(slf)
    }
}

impl GeometryBuilder {
    fn find_mut<'a>(&'a mut self, path: &str) -> PyResult<&'a mut volume::Volume> {
        let mut names = path.split(".");
        let volume = match names.next() {
            None => None,
            Some(name) => {
                let volume = self.definition.volume.as_mut();
                if name != volume.name() {
                    None
                } else {
                    let mut volume = Some(volume);
                    for name in names {
                        volume = volume.unwrap().volumes.iter_mut().find(|v| v.name() == name);
                        if volume.is_none() {
                            break
                        }
                    }
                    volume
                }
            },
        };
        volume.ok_or_else(|| {
            let why = format!("unknown '{}' volume", path);
            Error::new(ValueError).what("geometry operation").why(&why).to_err()
        })
    }
}

// ===============================================================================================
//
// Geometry definition.
//
// This is a thin wrapper collecting the top volume description and some optional material
// definitions.
//
// ===============================================================================================

struct GeometryDefinition {
    volume: Box<volume::Volume>,
    materials: Option<MaterialsDefinition>,
}

impl GeometryDefinition {
    pub fn new(definition: DictLike) -> PyResult<Self> {
        let (definition, file) = definition.resolve(None)?;

        const EXTRACTOR: Extractor<1> = Extractor::new([
            Property::optional_any("materials"),
        ]);

        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let tag = Tag::new("geometry", "", file.as_deref());
        let [materials] = EXTRACTOR.extract(
            &tag, &definition, Some(&mut remainder)
        )?;

        if remainder.len() != 1 {
            let why = format!("expected 1 top volume, found {}", remainder.len());
            let err = Error::new(ValueError).what("geometry").why(&why);
            return Err(err.into());
        }
        let (name, volume) = remainder.iter().next().unwrap();
        let tag = Tag::new("", name.as_str(), file.as_deref());
        let volume = volume::Volume::try_from_any(&tag, &volume)?;
        let volume = Box::new(volume);

        let materials: Option<Bound<PyAny>> = materials.into();
        let materials: PyResult<Option<MaterialsDefinition>> = materials
            .map(|materials| {
                let tag = Tag::new("", "", file.as_deref());
                MaterialsDefinition::try_from_any(&tag, &materials)
            })
            .transpose();
        let materials = materials?;

        let definition = Self { volume, materials };
        Ok(definition)
    }
}

// ===============================================================================================
//
// Volume proxy.
//
// ===============================================================================================

/// A volume of a Monte Carlo geometry.
#[pyclass(frozen, module="calzone")]
pub struct Volume {
    geometry: SharedPtr<ffi::GeometryBorrow>,
    /// The volume absolute pathname.
    #[pyo3(get)]
    name: String,
    /// The volume constitutive material.
    #[pyo3(get)]
    material: String,
    /// The volume shape (according to Geant4).
    #[pyo3(get)]
    solid: String,
    /// Flag indicating whether or not energy deposits are sampled.
    #[pyo3(get)]
    sensitive: bool,
    /// The mother of this volume, if any (i.e. directly containing this volume).
    #[pyo3(get)]
    mother: Option<String>,
    daughters: Vec<String>,
}

#[pymethods]
impl Volume {
    /// Daughter volume(s), if any (i.e. included insides).
    #[getter]
    fn get_daughters<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple> {
        PyTuple::new_bound(py, &self.daughters)
    }

    /// Return the volume's Axis-Aligned Bounding-Box (AABB).
    #[pyo3(name = "aabb")]
    fn compute_aabb(
        &self,
        py: Python,
        frame: Option<&str>
    ) -> PyResult<PyObject> {
        let frame = frame.unwrap_or("");
        let bbox = self.geometry.compute_box(self.name.as_str(), frame);
        if let Some(why) = ffi::get_error().value() {
            let err = Error::new(ValueError).what("box operation").why(why);
            return Err(err.into());
        }

        let result = PyArray::<f64>::empty(py, &[2, 3]).unwrap();
        result.set(0, bbox[0]).unwrap();
        result.set(1, bbox[2]).unwrap();
        result.set(2, bbox[4]).unwrap();
        result.set(3, bbox[1]).unwrap();
        result.set(4, bbox[3]).unwrap();
        result.set(5, bbox[5]).unwrap();
        result.readonly();
        Ok(result.as_any().into_py(py))
    }

    /// Return the coordinates of the volume origin.
    #[pyo3(name = "origin")]
    fn compute_origin(&self, frame: Option<&str>) -> PyResult<f64x3> {
        let frame = frame.unwrap_or("");
        let origin = self.geometry.compute_origin(self.name.as_str(), frame);
        if let Some(why) = ffi::get_error().value() {
            let err = Error::new(ValueError).what("origin operation").why(why);
            return Err(err.into());
        }
        Ok((&origin).into())
    }
}
