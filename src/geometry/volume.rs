use crate::utils::error::variant_error;
use crate::utils::extract::{extract, Extractor, Property, PropertyValue, Tag, TryFromBound};
use crate::utils::float::{f64x3, f64x3x3};
use crate::utils::io::load_stl;
use crate::utils::units::convert;
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use std::borrow::Cow;
use std::cmp::Ordering::{Equal, Greater};
use std::ffi::OsStr;
use std::path::Path;
use super::ffi;
use super::map::Map;


// ===============================================================================================
//
// Geometry volume.
//
// ===============================================================================================

pub struct Volume {
    name: String,
    material: String,
    shape: Shape,
    position: Option<f64x3>,
    rotation: Option<f64x3x3>,
    volumes: Vec<Volume>,
    overlaps: Vec<[String; 2]>,
}

pub enum Shape {
    Box(ffi::BoxShape),
    Cylinder(ffi::CylinderShape),
    Sphere(ffi::SphereShape),
    Tessellation(ffi::TessellatedShape),
}

impl From<&Shape> for ffi::ShapeType {
    fn from(value: &Shape) -> Self {
        match value {
            Shape::Box(_) => ffi::ShapeType::Box,
            Shape::Cylinder(_) => ffi::ShapeType::Cylinder,
            Shape::Sphere(_) => ffi::ShapeType::Sphere,
            Shape::Tessellation(_) => ffi::ShapeType::Tessellation,
        }
    }
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="lower_case")]
enum ShapeType {
    Box,
    Cylinder,
    Sphere,
    Tessellation,
}


// ===============================================================================================
//
// Conversion (from a Python dict).
//
// ===============================================================================================

impl TryFromBound for Volume {
    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        // Check volume name.
        match tag.name().chars().next() {
            None => {
                let message = format!("{}bad material (empty name)", tag.file_prefix());
                return Err(PyValueError::new_err(message));
            },
            Some(c) => if !c.is_uppercase() {
                let message: String = tag.bad().what("name")
                    .why("should be capitalized".to_string()).into();
                return Err(PyValueError::new_err(message));
            },
        }

        // Extract base properties.
        const EXTRACTOR: Extractor<4> = Extractor::new([
            Property::required_str("material"),
            Property::optional_vec("position"),
            Property::optional_mat("rotation"),
            Property::optional_dict("overlaps"),

        ]);

        let tag = tag.cast("volume");
        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let [material, position, rotation, overlaps] = EXTRACTOR.extract(
            &tag, value, Some(&mut remainder)
        )?;

        let name = tag.name().to_string();
        let material: String = material.into();
        let position: Option<f64x3> = position.into();
        let rotation: Option<f64x3x3> = rotation.into();
        let overlaps: Option<Bound<PyDict>> = overlaps.into();

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
                .map_err(|options| {
                    let message: String = tag.bad().into();
                    variant_error(message.as_str(), shape, options)
                })
        };

        if shapes.len() == 0 {
            get_shape_type("None")?; // This always fails.
        }
        let (shape_name, shape) = shapes.get(0).unwrap();
        let shape_type = get_shape_type(shape_name)?;
        if let Some((alt_name, _)) = shapes.get(1) {
            let _unused = get_shape_type(alt_name)?;
            let message: String = tag.bad().why(format!(
                "multiple shape definitions ({}, {}, ...)",
                shape_name,
                alt_name,
            )).into();
            return Err(PyValueError::new_err(message));
        }
        let shape_tag = tag.extend(shape_name, Some("shape"), None);
        let shape = match shape_type {
            ShapeType::Box => Shape::Box(ffi::BoxShape::try_from_any(&shape_tag, shape)?),
            ShapeType::Cylinder => Shape::Cylinder(
                ffi::CylinderShape::try_from_any(&shape_tag, shape)?),
            ShapeType::Sphere => Shape::Sphere(
                ffi::SphereShape::try_from_any(&shape_tag, shape)?),
            ShapeType::Tessellation => Shape::Tessellation(
                ffi::TessellatedShape::try_from_any(&shape_tag, shape)?),
        };

        // Extract sub-volumes.
        let volumes: PyResult<Vec<Volume>> = volumes
            .iter()
            .map(|(k, v)| {
                let tag = tag.extend(&k, None, None);
                Self::try_from_any(&tag, &v)
            })
            .collect();
        let volumes = volumes?;

        // Extract overlaps.
        let overlaps = match overlaps {
            None => Vec::<[String; 2]>::new(),
            Some(overlaps) => {
                // Extract and flatten overlaps.
                let mut o = Vec::<[String; 2]>::new();

                let find = |name: &String| -> PyResult<()> {
                    // Check that the overlaping volume is defined.
                    match volumes.iter().find(|v| &v.name == name) {
                        None => {
                            let message: String = tag.bad().what("overlap").why(format!(
                                "undefined '{}' volume",
                                name,
                            )).into();
                            Err(PyValueError::new_err(message))
                        }
                        Some(v) => {
                            // Check that the volume is not displaced.
                            if v.position.is_some() || v.rotation.is_some() {
                                let message: String = tag.bad().what("overlap").why(format!(
                                    "displaced '{}' volume",
                                    name,
                                )).into();
                                Err(PyNotImplementedError::new_err(message))
                            } else {
                                Ok(())
                            }
                        },
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

                for (left, right) in overlaps.iter() {
                    let left: String = extract(&left)
                        .or_else(|| tag.bad().what("left overlap").into())?;
                    let result: PyResult<Vec<String>> = right.extract();
                    match result {
                        Err(_) => {
                            let right: String = extract(&right)
                                .expect("a string or a sequence of strings")
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
                o
            },
        };

        let volume = Self { name, material, shape, position, rotation, volumes, overlaps };
        Ok(volume)
    }
}

impl TryFromBound for ffi::BoxShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let size: PyResult<f64x3> = value.extract();
        let size: f64x3 = match size {
            Err(_) => {
                const EXTRACTOR: Extractor<1> = Extractor::new([
                    Property::required_vec("size"),
                ]);

                let tag = tag.cast("Box");
                let [size] = EXTRACTOR.extract_any(&tag, value, None)?;
                size.into()
            },
            Ok(size) => size,
        };
        let shape = Self { size: size.into() };
        Ok(shape)
    }
}

impl TryFromBound for ffi::CylinderShape {
    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        const EXTRACTOR: Extractor<3> = Extractor::new([
            Property::required_f64("length"),
            Property::required_f64("radius"),
            Property::new_f64("thickness", 0.0),
        ]);

        let tag = tag.cast("Cylinder");
        let [length, radius, thickness] = EXTRACTOR.extract_any(&tag, value, None)?;
        let shape = Self {
            length: length.into(),
            radius: radius.into(),
            thickness: thickness.into(),
        };
        Ok(shape)
    }
}

impl TryFromBound for ffi::SphereShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let radius: PyResult<f64> = value.extract();
        let radius: f64 = match radius {
            Err(_) => {
                const EXTRACTOR: Extractor<1> = Extractor::new([
                    Property::required_f64("radius"),
                ]);

                let tag = tag.cast("Sphere");
                let [size] = EXTRACTOR.extract_any(&tag, value, None)?;
                size.into()
            },
            Ok(radius) => radius,
        };
        let shape = Self { radius: radius.into() };
        Ok(shape)
    }
}

impl TryFromBound for ffi::TessellatedShape {
    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self> {
        let mut scale: f64 = 1.0;
        let mut origin: Option<f64x3> = None;
        let mut min_depth: Option<f64> = None;
        let path: PyResult<String> = value.extract();
        let path: String = match path {
            Err(_) => {
                const EXTRACTOR: Extractor<4> = Extractor::new([
                    Property::required_str("path"),
                    Property::optional_str("units"),
                    Property::optional_vec("origin"),
                    Property::optional_f64("min_depth"),
                ]);

                let tag = tag.cast("tessellation");
                let [path, units, center, depth] = EXTRACTOR.extract_any(&tag, value, None)?;
                if let PropertyValue::String(units) = units {
                    scale = convert(value.py(), units.as_str(), "cm")
                        .map_err(|e| {
                            let msg: String = tag.bad().what("units").why(format!("{}", e)).into();
                            PyValueError::new_err(msg)
                        })?;
                }
                origin = center.into();
                min_depth = depth.into();
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
                if min_depth.is_some() {
                    let msg: String = tag.bad()
                        .what("min_depth")
                        .why("invalid option for STL format".to_string())
                        .into();
                        return Err(PyValueError::new_err(msg));
                } else if origin.is_some() {
                    let msg: String = tag.bad()
                        .what("origin")
                        .why("invalid option for STL format".to_string())
                        .into();
                        return Err(PyValueError::new_err(msg));
                } else {
                    load_stl(&path)
                }
            },
            Some("png") | Some("tif") => {
                let py = value.py();
                let map = Map::from_file(py, &path)?;
                let facets = map.tessellate(py, origin, min_depth)?;
                Ok(facets)
            },
            _ => return Err(PyNotImplementedError::new_err("")),
        }.map_err(|msg| {
            let msg: String = tag.bad().why(msg).into();
            PyValueError::new_err(msg)
        })?;

        let scale = scale as f32;
        for value in &mut facets.iter_mut() {
            *value *= scale;
        }
        let shape = Self { facets };
        Ok(shape)
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

    pub fn is_rotated(&self) -> bool {
        return self.rotation.is_some()
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

    pub fn rotation(&self) -> &[[f64; 3]] {
        match self.rotation.as_ref() {
            Some(rotation) => rotation.as_ref(),
            None => unreachable!(),
        }
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
