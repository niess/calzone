use crate::utils::error::variant_error;
use crate::utils::extract::{Extractor, Property, Tag, TryFromBound};
use crate::utils::float::{f64x3, f64x3x3};
use enum_variants_strings::EnumVariantsStrings;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::exceptions::PyValueError;
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
}

pub enum Shape {
    Box(ffi::BoxShape),
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
        const EXTRACTOR: Extractor<4> = Extractor::new([
            Property::required_str("material"),
            Property::optional_vec("position"),
            Property::optional_mat("rotation"),
            Property::optional_dict("volumes"),
        ]);

        let tag = tag.cast("volume");
        let mut remainder = IndexMap::<String, Bound<PyAny>>::new();
        let [material, position, rotation, volumes] = EXTRACTOR.extract(
            &tag, value, Some(&mut remainder)
        )?;

        let name = tag.name().to_string();
        let material: String = material.into();
        let position: Option<f64x3> = position.into();
        let rotation: Option<f64x3x3> = rotation.into();
        let volumes: Option<Bound<PyDict>> = volumes.into();

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

        let volume = Self { name, material, shape, position, rotation, volumes };
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
