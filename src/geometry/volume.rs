use crate::utils::error::ErrorKind::{NotImplementedError, ValueError};
use crate::utils::error::variant_error;
use crate::utils::extract::{extract, Extractor, Vector, Property, PropertyValue, Tag,
                            TryFromBound};
use crate::utils::float::{f64x3, f64x3x3};
use crate::utils::io::{DictLike, load_stl};
use crate::utils::units::convert;
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::exceptions::{PyNotImplementedError};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering::{Equal, Greater};
use std::ffi::OsStr;
use std::path::Path;
use super::{ffi, MaterialsDefinition};
use super::map::Map;


// ===============================================================================================
//
// Geometry volume.
//
// ===============================================================================================

#[derive(Default, Deserialize, Serialize)]
pub struct Volume {
    pub(super) name: String,
    pub(super) material: String,
    pub(super) shape: Shape,
    pub(super) position: Option<f64x3>,
    pub(super) rotation: Option<f64x3x3>,
    pub(super) volumes: Vec<Volume>,
    pub(super) overlaps: Vec<[String; 2]>,
    pub(super) roles: ffi::Roles,
    pub(super) subtract: Vec<String>,
    pub(super) materials: Option<MaterialsDefinition>,
}

#[derive(Deserialize, Serialize)]
pub enum Shape {
    Box(ffi::BoxShape),
    Cylinder(ffi::CylinderShape),
    Envelope(ffi::EnvelopeShape),
    Sphere(ffi::SphereShape),
    Tessellation(ffi::TessellatedShape),
}

impl From<&Shape> for ffi::ShapeType {
    fn from(value: &Shape) -> Self {
        match value {
            Shape::Box(_) => ffi::ShapeType::Box,
            Shape::Cylinder(_) => ffi::ShapeType::Cylinder,
            Shape::Envelope(_) => ffi::ShapeType::Envelope,
            Shape::Sphere(_) => ffi::ShapeType::Sphere,
            Shape::Tessellation(_) => ffi::ShapeType::Tessellation,
        }
    }
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
pub(super) enum ShapeType {
    Box,
    Cylinder,
    Envelope,
    Sphere,
    Tessellation,
}

pub struct Include {
    path: String,
    position: Option<f64x3>,
    rotation: Option<f64x3x3>,
    name: Option<String>,
    subtract: Vec<String>,
}

impl Volume {
    pub(super) fn check(name: &str) -> Result<(), &'static str> {
        for c in name.chars() {
            if !c.is_alphanumeric() {
                return Err("expected an alphanumeric string");
            }
        }
        match name.chars().next() {
            None => {
                return Err("empty string");
            },
            Some(c) => if !c.is_uppercase() {
                return Err("should be capitalised");
            },
        }
        Ok(())
    }

    pub(super) fn flatten_overlaps<'py>(
        tag: &Tag,
        overlaps: &DictLike<'py>,
        volumes: &[Self],
    ) -> PyResult<Vec<[String; 2]>> {
        // Extract and flatten overlaps.
        let mut o = Vec::<[String; 2]>::new();

        let find = |name: &String| -> PyResult<()> {
            // Check that the overlaping volume is defined.
            match volumes.iter().find(|v| &v.name == name) {
                None => {
                    let err = tag.bad().what("overlap").why(format!(
                        "undefined '{}' volume",
                        name,
                    )).to_err(ValueError);
                    Err(err)
                }
                Some(_) => Ok(()),
            }
        };

        let mut push = |left: String, right: String| -> PyResult<()> {
            // Order and push an overlap pair.
            find(&left)?;
            find(&right)?;
            match left.cmp(&right) {
                Greater => o.push([right, left]),
                _ => o.push([left, right]),
            }
            Ok(())
        };

        let (overlaps, tag) = tag.resolve(&overlaps)?;
        for (left, right) in overlaps.iter() {
            let left: String = extract(&left)
                .or_else(|| tag.bad().what("left overlap").into())?;
            let result: PyResult<Vec<String>> = right.extract();
            match result {
                Err(_) => {
                    let right: String = extract(&right)
                        .expect("a (sequence of) 'str'")
                        .or_else(|| tag.bad().what("right overlap").into())?;
                    push(left, right)?;
                },
                Ok(rights) => {
                    for right in rights {
                        push(left.clone(), right)?;
                    }
                },
            }
        }

        // Sort overlaps.
        o.sort_by(|a, b| match a[0].cmp(&b[0]) {
            Equal => a[1].cmp(&b[1]),
            other => other,
        });
        o.dedup(); // remove duplicates.
        Ok(o)
    }

    pub(super) fn validate(&self) -> PyResult<()> {
        fn inspect(tag: &Tag, volume: &Volume) -> PyResult<()> {
            let daughters: Vec<_> = volume.volumes.iter()
                .map(|v| (v.name(), !v.subtract.is_empty()))
                .collect();
            for v in volume.volumes.iter() {
                let vtag = tag.extend(v.name.as_ref(), None, None);
                for subtract in v.subtract.iter() {
                    if subtract == v.name() {
                        let why = format!("cannot subtract self ('{}.{}')", tag.path(), subtract);
                        return Err(vtag.bad().what("subtract").why(why).to_err(ValueError))
                    }

                    match daughters.iter().find(|v| v.0 == subtract) {
                        None => {
                            let why = format!("unknown volume '{}.{}'", tag.path(), subtract);
                            return Err(vtag.bad().what("subtract").why(why).to_err(ValueError))
                        },
                        Some((_, subtracted)) => if *subtracted {
                            let why = format!(
                                "cannot subtract a subtracted volume ('{}.{}')",
                                tag.path(),
                                subtract
                            );
                            return Err(vtag.bad().what("subtract").why(why)
                                .to_err(NotImplementedError))
                        } else {
                            // Check overlaps.
                            let mut is_vol = false;
                            let mut is_sub = false;
                            for [left, right] in volume.overlaps.iter() {
                                if left == volume.name() || right == volume.name() {
                                    is_vol = true;
                                }
                                if left == subtract || right == subtract {
                                    is_sub = true;
                                }
                                if is_vol && is_sub {
                                    let why = format!(
                                        "cannot subtract overlaping volumes ('{}.{}', '{}'.'{}')",
                                        tag.path(),
                                        volume.name(),
                                        tag.path(),
                                        subtract,
                                    );
                                    return Err(vtag.bad().what("subtract").why(why)
                                        .to_err(NotImplementedError))
                                }
                            }
                        },
                    }
                }
                inspect(&vtag, v)?;
            }
            Ok(())
        }

        let tag = Tag::new("volume", self.name.as_ref(), None);
        if !self.subtract.is_empty() {
            let why = format!("unknown volume '{}'", self.subtract[0]);
            return Err(tag.bad().what("subtract").why(why).to_err(ValueError))
        }
        inspect(&tag, self)
    }
}

impl Shape {
    fn new<'py>(
        tag: &Tag,
        shape_type: ShapeType,
        properties: &Bound<'py, PyAny>
    ) -> PyResult<Shape> {
        let tag = tag.extend(shape_type.to_str(), Some("shape"), None);
        let shape = match shape_type {
            ShapeType::Box => Shape::Box(ffi::BoxShape::try_from_any(&tag, properties)?),
            ShapeType::Cylinder => Shape::Cylinder(
                ffi::CylinderShape::try_from_any(&tag, properties)?),
            ShapeType::Envelope => Shape::Envelope(
                ffi::EnvelopeShape::try_from_any(&tag, properties)?),
            ShapeType::Sphere => Shape::Sphere(
                ffi::SphereShape::try_from_any(&tag, properties)?),
            ShapeType::Tessellation => Shape::Tessellation(
                ffi::TessellatedShape::try_from_any(&tag, properties)?),
        };
        Ok(shape)
    }
}

impl Default for Shape {
    fn default() -> Self {
        Self::Envelope(ffi::EnvelopeShape {
            safety: ffi::EnvelopeShape::DEFAULT_SAFETY,
            shape: ffi::EnvelopeShape::DEFAULT_SHAPE,
        })
    }
}

impl ffi::EnvelopeShape {
    const DEFAULT_SAFETY: f64 = 0.01;
    const DEFAULT_SHAPE: ffi::ShapeType = ffi::ShapeType::Box;
    const DEFAULT_SHAPE_NAME: &'static str = "box";
}

// ===============================================================================================
//
// Conversion (from a Python dict).
//
// ===============================================================================================

impl TryFromBound for Volume {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        // Check volume name.
        Self::check(tag.name())
            .map_err(|why| tag.bad().what("name").why(why.to_string()).to_err(ValueError))?;

        // Extract base properties.
        const EXTRACTOR: Extractor<8> = Extractor::new([
            Property::required_str("material"),
            Property::optional_strs("role"),
            Property::optional_vec("position"),
            Property::optional_mat("rotation"),
            Property::optional_dict("overlaps"),
            Property::optional_strs("subtract"),
            Property::optional_any("materials"),
            Property::optional_any("include"), // XXX Document this option.
        ]);

        let py = value.py();
        let tag = tag.cast("volume");
        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let [material, role, position, rotation, overlaps, subtract, materials, include] =
            EXTRACTOR.extract(&tag, value, Some(&mut remainder))?;

        let name = tag.name().to_string();
        let material: String = material.into();
        let role: Vec<String> = role.into();
        let position: Option<f64x3> = position.into();
        let rotation: Option<f64x3x3> = rotation.into();
        let overlaps: Option<DictLike> = overlaps.into();
        let subtract: Vec<String> = subtract.into();
        let include: Option<Bound<PyAny>> = include.into();

        // Parse role(s).
        let (_, tag) = tag.resolve(value)?;
        let roles: ffi::Roles = role.as_slice().try_into()
            .map_err(|why| {
                tag.bad().what("role").why(why).to_err(ValueError)
            })?;

        // Split shape(s) and volumes from remainder.
        let (volumes, shapes) = {
            let mut volumes = Vec::<(String, Bound<PyAny>)>::new();
            let mut shapes = Vec::<(String, Bound<PyAny>)>::new();
            for (k, v) in remainder.drain(..) {
                if k.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    volumes.push((k, v))
                } else {
                    shapes.push((k, v))
                }
            }
            (volumes, shapes)
        };

        // Extract shape properties.
        let get_shape_type = |shape: &str| -> PyResult<ShapeType> {
            ShapeType::from_str(shape)
                .map_err(|_| {
                    let why = format!("unknown property or shape '{}'", shape);
                    tag.bad().why(why).to_err(ValueError)
                })
        };

        let shape = if shapes.len() == 0 {
            Shape::default()
        } else {
            let (shape_name, shape) = shapes.get(0).unwrap();
            let shape_type = get_shape_type(shape_name)?;
            if let Some((alt_name, _)) = shapes.get(1) {
                let _unused = get_shape_type(alt_name)?;
                let err = tag.bad().why(format!(
                    "multiple shape definitions ({}, {}, ..)",
                    shape_name,
                    alt_name,
                )).to_err(ValueError);
                return Err(err);
            }
            Shape::new(&tag, shape_type, shape)?
        };

        // Extract sub-volumes.
        let volumes: PyResult<Vec<Volume>> = volumes
            .iter()
            .map(|(k, v)| {
                let tag = tag.extend(&k, None, None);
                Self::try_from_any(&tag, &v)
            })
            .collect();
        let mut volumes = volumes?;

        // Extract includes.
        if let Some(include) = include {
            let includes: PyResult<Vec<Bound<PyAny>>> = FromPyObject::extract_bound(&include);
            let includes = includes.unwrap_or_else(|_| vec![include]);
            for include in includes {
                let include = {
                    let path: PyResult<String> = FromPyObject::extract_bound(&include);
                    match path {
                        Ok(path) => Include::new(path),
                        Err(_) => {
                            let include: Include = TryFromBound::try_from_any(&tag, &include)?;
                            include
                        },
                    }
                };
                let definition = super::GeometryDefinition::new(
                    DictLike::from_str(py, include.path.as_str()), tag.file(),
                )?;
                let mut sub = *definition.volume;
                if let Some(name) = include.name {
                    sub.name = name
                }
                sub.position = include.position.or(sub.position);
                sub.rotation = include.rotation.or(sub.rotation);
                sub.subtract = include.subtract;

                for v in volumes.iter() {
                    if v.name == sub.name {
                        let err = tag.bad().what("include").why(format!(
                            "multiple definitions of volume '{}'", sub.name,
                        )).to_err(ValueError);
                        return Err(err);
                    }
                }
                volumes.push(sub);
            }
        }

        // Extract overlaps.
        let overlaps = match overlaps {
            None => Vec::<[String; 2]>::new(),
            Some(overlaps) => Self::flatten_overlaps(&tag, &overlaps, volumes.as_slice())?,
        };

        // Extract materials.
        let materials: Option<Bound<PyAny>> = materials.into();
        let materials: PyResult<Option<MaterialsDefinition>> = materials
            .map(|materials| MaterialsDefinition::try_from_any(&tag, &materials))
            .transpose();
        let materials = materials?;

        let volume = Self {
            name, material, roles, shape, position, rotation, volumes, overlaps, subtract,
            materials
        };

        Ok(volume)
    }
}

impl TryFromBound for Shape {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        let (value, tag) = tag.resolve(value)?;

        let get_shape_type = |shape: &str| -> PyResult<ShapeType> {
            ShapeType::from_str(shape)
                .map_err(|_| {
                    let why = format!("unknown shape '{}'", shape);
                    tag.bad().why(why).to_err(ValueError)
                })
        };

        if value.len() == 0 {
            get_shape_type("None")?; // This always fails.
        }

        let mut items = value.iter();
        let (shape_name, shape) = items.next().unwrap();
        let shape_name: String = extract(&shape_name)
                .or_else(|| tag.bad_type())?;
        let shape_type = get_shape_type(shape_name.as_str())?;
        let shape = Shape::new(&tag, shape_type, &shape)?;
        if let Some((alt_name, _)) = items.next() {
            let alt_name: String = extract(&alt_name)
                    .or_else(|| tag.bad_type())?;
            let _unused = get_shape_type(alt_name.as_str())?;
            let err = tag.bad().why(format!(
                "multiple shape definitions ({}, {}, ..)",
                shape_name,
                alt_name,
            )).to_err(ValueError);
            return Err(err);
        }
        Ok(shape)
    }
}

impl TryFromBound for ffi::BoxShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let size: PyResult<Vector> = value.extract();
        let size: f64x3 = match size {
            Err(_) => {
                const EXTRACTOR: Extractor<1> = Extractor::new([
                    Property::required_vec("size"),
                ]);

                let tag = tag.cast("Box");
                let [size] = EXTRACTOR.extract_any(&tag, value, None)?;
                size.into()
            },
            Ok(size) => size.into_vec(),
        };
        let shape = Self { size: size.into() };
        Ok(shape)
    }
}

impl TryFromBound for ffi::CylinderShape {
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<4> = Extractor::new([
            Property::required_f64("length"),
            Property::required_f64("radius"),
            Property::new_f64("thickness", 0.0),
            Property::new_interval("section", [0.0, 360.0]),
        ]);

        let tag = tag.cast("Cylinder");
        let [length, radius, thickness, section] = EXTRACTOR.extract(&tag, value, None)?;
        let shape = Self {
            length: length.into(),
            radius: radius.into(),
            thickness: thickness.into(),
            section: section.into(),
        };
        Ok(shape)
    }
}

impl TryFromBound for ffi::EnvelopeShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let mut safety: Option<f64> = None;
        let shape: PyResult<String> = value.extract();
        let shape: String = match shape {
            Err(_) => {
                let sfty: PyResult<f64> = value.extract();
                match sfty {
                    Err(_) => {
                        const EXTRACTOR: Extractor<2> = Extractor::new([
                            Property::new_str("shape", ffi::EnvelopeShape::DEFAULT_SHAPE_NAME),
                            Property::optional_f64("safety"),
                        ]);

                        let tag = tag.cast("Envelope");
                        let [shape, sfty] = EXTRACTOR.extract_any(&tag, value, None)?;
                        safety = sfty.into();
                        shape.into()
                    }
                    Ok(sfty) => {
                        safety = Some(sfty);
                        Self::DEFAULT_SHAPE_NAME.to_string()
                    },
                }
            },
            Ok(shape) => shape,
        };
        let shape: ffi::ShapeType = ShapeType::from_str(shape.as_str())
            .and_then(|shape| match shape {
                ShapeType::Box => Ok(ffi::ShapeType::Box),
                ShapeType::Cylinder => Ok(ffi::ShapeType::Cylinder),
                ShapeType::Sphere => Ok(ffi::ShapeType::Sphere),
                _ => Err(&[]),
            })
            .map_err(|_| {
                let message: String = tag.bad().what("shape").into();
                let options = ["box", "cylinder", "sphere"];
                variant_error(message.as_str(), shape.as_str(), &options)
            })?;
        let safety = safety.unwrap_or(Self::DEFAULT_SAFETY);
        let envelope = Self { shape, safety };
        Ok(envelope)
    }
}

impl TryFromBound for ffi::SphereShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let mut thickness: Option<f64> = None;
        let mut azimuth_section = [0.0 , 360.0];
        let mut zenith_section = [0.0 , 180.0];
        let radius: PyResult<f64> = value.extract();
        let radius: f64 = match radius {
            Err(_) => {
                const EXTRACTOR: Extractor<4> = Extractor::new([
                    Property::required_f64("radius"),
                    Property::optional_f64("thickness"),
                    Property::new_interval("azimuth_section", [0.0, 360.0]),
                    Property::new_interval("zenith_section", [0.0, 180.0]),
                ]);

                let tag = tag.cast("Sphere");
                let [r, e, az, ze] = EXTRACTOR.extract_any(&tag, value, None)?;
                thickness = e.into();
                azimuth_section = az.into();
                zenith_section = ze.into();
                r.into()
            },
            Ok(radius) => radius,
        };
        let thickness = thickness.unwrap_or(0.0);
        let shape = Self { radius, thickness, azimuth_section, zenith_section };
        Ok(shape)
    }
}

impl TryFromBound for ffi::TessellatedShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let mut scale: f64 = 1.0;
        let mut origin: Option<f64x3> = None;
        let mut extra_depth: Option<f64> = None;
        let mut regular: Option<bool> = None;
        let path: PyResult<String> = value.extract();
        let path: String = match path {
            Err(_) => {
                const EXTRACTOR: Extractor<5> = Extractor::new([
                    Property::required_str("path"),
                    Property::optional_str("units"),
                    Property::optional_vec("origin"),
                    Property::optional_f64("extra_depth"),
                    Property::optional_bool("regular"),
                ]);

                let tag = tag.cast("tessellation");
                let [path, units, center, depth, reg] = EXTRACTOR.extract_any(&tag, value, None)?;
                if let PropertyValue::String(units) = units {
                    scale = convert(value.py(), units.as_str(), "cm")
                        .map_err(|e|
                            tag.bad().what("units").why(format!("{}", e)).to_err(ValueError)
                        )?;
                }
                origin = center.into();
                extra_depth = depth.into();
                regular = reg.into();
                path.into()
            },
            Ok(path) => path,
        };

        let path = match tag.file() {
            None => Cow::Borrowed(Path::new(&path)),
            Some(file) => {
                let mut file = file.to_path_buf();
                if file.pop() {
                    file.push(path);
                    Cow::Owned(file)
                } else {
                    Cow::Borrowed(Path::new(&path))
                }
            },
        };
        let mut facets = match path.extension().and_then(OsStr::to_str) {
            Some("stl") => {
                if extra_depth.is_some() {
                    let err = tag.bad()
                        .what("extra_depth")
                        .why("invalid option for STL format".to_string())
                        .to_err(ValueError);
                        return Err(err);
                } else if origin.is_some() {
                    let err = tag.bad()
                        .what("origin")
                        .why("invalid option for STL format".to_string())
                        .to_err(ValueError);
                        return Err(err);
                } else if regular.is_some() {
                    let err = tag.bad()
                        .what("regular")
                        .why("invalid option for STL format".to_string())
                        .to_err(ValueError);
                        return Err(err);
                } else {
                    load_stl(&path)
                }
            },
            Some("png") | Some("tif") => {
                let py = value.py();
                let map = Map::from_file(py, &path)?;
                let regular = regular.unwrap_or(false);
                let facets = map.tessellate(py, regular, origin, extra_depth)?;
                Ok(facets)
            },
            _ => return Err(PyNotImplementedError::new_err("")),
        }.map_err(|msg| tag.bad().why(msg).to_err(ValueError))?;

        let scale = scale as f32;
        for value in &mut facets.iter_mut() {
            *value *= scale;
        }
        let shape = Self { facets };
        Ok(shape)
    }
}

impl TryFromBound for Include {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Extract base properties.
        const EXTRACTOR: Extractor<5> = Extractor::new([
            Property::required_str("path"),
            Property::optional_vec("position"),
            Property::optional_mat("rotation"),
            Property::optional_str("name"),
            Property::optional_strs("subtract"),
        ]);

        let tag = tag.cast("include");
        let [path, position, rotation, name, subtract] = EXTRACTOR.extract_any(&tag, value, None)?;

        let path: String = path.into();
        let position: Option<f64x3> = position.into();
        let rotation: Option<f64x3x3> = rotation.into();
        let name: Option<String> = name.into();
        let subtract: Vec<String> = subtract.into();

        let include = Self { path, position, rotation, name, subtract };
        Ok(include)
    }
}

impl Include {
    fn new(path: String) -> Self {
        Self { path, position: None, rotation: None, name: None, subtract: Vec::new() }
    }
}

// ===============================================================================================
//
// C++ interface.
//
// ===============================================================================================

impl Volume {
    pub fn box_shape(&self) -> &ffi::BoxShape {
        match &self.shape {
            Shape::Box(shape) => &shape,
            _ => unreachable!(),
        }
    }

    pub fn cylinder_shape(&self) -> &ffi::CylinderShape {
        match &self.shape {
            Shape::Cylinder(shape) => &shape,
            _ => unreachable!(),
        }
    }

    pub fn envelope_shape(&self) -> &ffi::EnvelopeShape {
        match &self.shape {
            Shape::Envelope(shape) => &shape,
            _ => unreachable!(),
        }
    }

    pub fn is_rotated(&self) -> bool {
        return self.rotation.is_some()
    }

    pub fn is_translated(&self) -> bool {
        return self.position.is_some()
    }

    pub fn material(&self) -> &String {
        &self.material
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn overlaps(&self) -> &[[String; 2]] {
        self.overlaps.as_slice()
    }

    pub fn position(&self) -> [f64; 3] {
        match self.position {
            None => f64x3::zero().into(),
            Some(p) => p.into(),
        }
    }

    pub fn roles(&self) -> ffi::Roles {
        self.roles
    }

    pub fn rotation(&self) -> &[[f64; 3]] {
        match self.rotation.as_ref() {
            Some(rotation) => rotation.as_ref(),
            None => unreachable!(),
        }
    }

    pub fn sensitive(&self) -> bool {
        self.roles.any()
    }

    pub fn shape(&self) -> ffi::ShapeType {
        (&self.shape).into()
    }

    pub fn sphere_shape(&self) -> &ffi::SphereShape {
        match &self.shape {
            Shape::Sphere(shape) => &shape,
            _ => unreachable!(),
        }
    }

    pub fn subtract(&self) -> &[String] {
        self.subtract.as_slice()
    }

    pub fn tessellated_shape(&self) -> &ffi::TessellatedShape {
        match &self.shape {
            Shape::Tessellation(shape) => &shape,
            _ => unreachable!(),
        }
    }

    pub fn volumes(&self) -> &[Volume] {
        self.volumes.as_slice()
    }
}
