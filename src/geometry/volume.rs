use crate::utils::error::variant_error;
use crate::utils::extract::{extract, Extractor, Property, Tag, TryFromBound};
use crate::utils::float::{f64x3, f64x3x3};
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::exceptions::PyValueError;
use std::cmp::Ordering::{Equal, Greater};
use super::ffi;


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
}

impl From<&Shape> for ffi::ShapeType {
    fn from(value: &Shape) -> Self {
        match value {
            Shape::Box(_) => ffi::ShapeType::Box,
        }
    }
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="none")]
enum ShapeType {
    Box,
}


// ===============================================================================================
//
// Conversion (from a Python dict).
//
// ===============================================================================================

impl TryFromBound for Volume {
    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self> {
        // Extract base properties.
        const EXTRACTOR: Extractor<5> = Extractor::new([
            Property::required_str("material"),
            Property::optional_vec("position"),
            Property::optional_mat("rotation"),
            Property::optional_dict("volumes"),
            Property::optional_dict("overlaps"),
        ]);

        let tag = tag.cast("volume");
        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let [material, position, rotation, volumes, overlaps] = EXTRACTOR.extract(
            &tag, value, Some(&mut remainder)
        )?;

        let name = tag.name().to_string();
        let material: String = material.into();
        let position: Option<f64x3> = position.into();
        let rotation: Option<f64x3x3> = rotation.into();
        let volumes: Option<Bound<PyDict>> = volumes.into();
        let overlaps: Option<Bound<PyDict>> = overlaps.into();

        // Extract shape properties.
        let get_shape_type = |shape: &str| -> PyResult<ShapeType> {
            ShapeType::from_str(shape)
                .map_err(|options| {
                    let message: String = tag.bad().into();
                    variant_error(message.as_str(), shape, options)
                })
        };

        if remainder.len() == 0 {
            get_shape_type("None")?; // This always fails.
        }
        let (shape_name, shape) = remainder.get_index(0).unwrap();
        let shape_type = get_shape_type(shape_name)?;
        if let Some((alt_name, _)) = remainder.get_index(1) {
            let _unused = get_shape_type(alt_name)?;
            let message: String = tag.bad().why(format!(
                "multiple shape definitions ({}, {}, ?)",
                shape_name,
                alt_name,
            )).into();
            return Err(PyValueError::new_err(message));
        }
        let shape_tag = tag.extend(shape_name, None);
        let shape = match shape_type {
            ShapeType::Box => Shape::Box(ffi::BoxShape::try_from_any(&shape_tag, shape)?),
        };

        // Extract sub-volumes.
        let volumes = match volumes {
            None => Vec::<Volume>:: new(),
            Some(volumes) => {
                let volumes = Vec::<Self>::try_from_dict(&tag, &volumes)?;
                volumes
            },
        };

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

// ===============================================================================================
//
// C++ interface.
//
// ===============================================================================================

impl Volume {
    pub fn box_shape(&self) -> &ffi::BoxShape {
        match &self.shape {
            Shape::Box(shape) => &shape,
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

    pub fn volumes(&self) -> &[Volume] {
        self.volumes.as_slice()
    }
}
