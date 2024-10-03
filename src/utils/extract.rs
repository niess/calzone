use indexmap::IndexMap;
use nalgebra::{Rotation3, Unit, Vector3};
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyDict;
use std::borrow::Cow;
use std::path::Path;
use super::error::{Error, ErrorKind};
use super::float::{f64x3, f64x3x3};
use super::io::DictLike;


// ===============================================================================================
//
// Generic extraction from a Python bound object.
//
// ===============================================================================================

pub trait TryFromBound {
    // Note that, despite trait functions all have default implementations, at least one of
    // `try_from_any` or `try_from_dict` must be overriden.

    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        Self: Sized
    {
        let value: DictLike = extract(value)
            .or_else(|| tag.bad().what("properties").into())?;
        Self::try_from_dict(tag, &value)
    }

    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self>
    where
        Self: Sized
    {
        let (value, tag) = tag.resolve(value)?;
        Self::try_from_any(&tag, value.as_any())
    }
}

impl<T> TryFromBound for Vec<T>
where
    T: TryFromBound + Sized,
{
    fn try_from_dict<'py>(tag: &Tag, value: &DictLike<'py>) -> PyResult<Self>
    where
        Self: Sized
    {
        let (value, tag) = tag.resolve(value)?;
        let mut items = Vec::<T>::with_capacity(value.len());
        for (k, v) in value.iter() {
            let name: String = extract(&k)
                .or_else(|| tag.bad_type())?;
            let tag = tag.extend(&name, None, None);
            let item = T::try_from_any(&tag, &v)?;
            items.push(item);
        }
        Ok(items)
    }
}

/// A contextual `Tag` enclosing the type, the name and the path of the object being extracted.
#[derive(Clone)]
pub struct Tag<'a> {
    typename: &'a str,
    name: &'a str,
    path: Cow<'a, str>,
    file: Option<Cow<'a, Path>>,
}

impl<'a> Tag<'a> {
    pub fn bad<'b>(&'b self) -> TaggedBad<'a, 'b> {
        TaggedBad::new(self)
    }

    pub fn bad_type(&self) -> String {
        let prefix = match &self.file {
            None => Cow::Borrowed(""),
            Some(file) => Cow::Owned(format!("{}: ", file.display())),
        };
        format!("{}bad {}", prefix, self.typename)
    }

    pub fn cast<'b: 'a>(&'b self, typename: &'a str) -> Tag<'b> {
        let path = Cow::Borrowed(self.path.as_ref());
        Self { typename, name: self.name, path, file: self.file.clone() }
    }

    /// Returns a new `Tag` with a path extended by `value`, and optionally a different type or
    /// file.
    pub fn extend<'b: 'a>(
        &'b self,
        value: &'a str,
        typename: Option<&'a str>,
        file: Option<&'a Path>
    ) -> Self {
        let typename = typename.unwrap_or(self.typename);
        let file = file.or(self.file.as_deref());
        if self.name.is_empty() {
            Self::new(typename, value, file)
        } else {
            let path = format!("{}.{}", self.path(), value);
            let path = Cow::Owned(path);
            let file = file.as_ref().map(|file| Cow::Borrowed(*file));
            Self { typename, name: value, path, file }
        }
    }

    /// Returns the file of this `Tag`.
    pub fn file<'b: 'a>(&'b self) -> Option<&'a Path> {
        self.file.as_deref()
    }

    /// Returns the qualified name of this `Tag`.
    pub fn qualified_name(&self) -> String {
        if self.path.is_empty() {
            self.typename.to_string()
        } else {
            format!("'{}' {}", self.path, self.typename)
        }
    }

    /// Returns the name of this `Tag`.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns a new `Tag` initialised with `name`.
    pub fn new(typename: &'a str, name: &'a str, file: Option<&'a Path>) -> Self {
        let path = Cow::Borrowed(name);
        let file = file.as_ref().map(|file| Cow::Borrowed(*file));
        Self { typename, name, path, file }
    }

    /// Returns the path of this `Tag`.
    pub fn path<'b>(&'b self) -> &'b str {
        &self.path
    }

    pub fn resolve<'b: 'a + 'c, 'c, 'py>(&'b self, dict: &'c DictLike<'py>) -> PyResult<(Cow<'c, Bound<'py, PyDict>>, Self)> {
        let py = dict.py();
        let (dict, file) = dict.resolve(self.file())
            .map_err(|err|
                self.bad().why(format!("{}", err.value_bound(py))).to_err(ErrorKind::ValueError)
            )?;
        let tag = if file.is_some() {
            let file = Some(Cow::Owned(file.unwrap()));
            Self { typename: self.typename, name: self.name, path: self.path.clone(), file }
        } else {
            self.clone()
        };
        Ok((dict, tag))
    }
}

pub struct TaggedBad<'a, 'b> {
    tag: &'b Tag<'a>,
    what: Option<&'b str>,
    why: Option<String>,
}

impl<'a, 'b> TaggedBad<'a, 'b> {
    fn new(tag: &'b Tag<'a>) -> Self {
        Self { tag, what: None, why: None }
    }

    pub fn to_err(&self, kind: ErrorKind) -> PyErr {
        let tag = self.tag.qualified_name();
        let prefix = self.tag.file.as_ref().map(|file| file.to_string_lossy());
        Error::new(kind)
            .maybe_where(prefix.as_deref())
            .who(tag.as_str())
            .maybe_what(self.what)
            .maybe_why(self.why.as_deref())
            .into()
    }

    pub fn what(mut self, what: &'b str) -> Self {
        self.what = Some(what);
        self
    }

    pub fn why(mut self, why: String) -> Self {
        self.why = Some(why);
        self
    }
}

impl<'a, 'b> From<TaggedBad<'a, 'b>> for String {
    fn from(value: TaggedBad<'a, 'b>) -> Self {
        let tag = value.tag.qualified_name();
        let prefix = value.tag.file.as_ref().map(|file| file.to_string_lossy());
        Error::default()
            .maybe_where(prefix.as_deref())
            .who(tag.as_str())
            .maybe_what(value.what)
            .maybe_why(value.why.as_deref())
            .into()
    }
}


// ===============================================================================================
//
// Procedural properties extractor (from a Python dict).
//
// ===============================================================================================

pub struct Extractor<const N: usize> {
    properties: [Property; N],
}

pub struct Property {
    name: &'static str,
    tp: PropertyType,
    default: PropertyDefault,
}

#[allow(dead_code)]
enum PropertyDefault {
    Bool(bool),
    F64(f64),
    F64x3(f64x3),
    F64x3x3(f64x3x3),
    Interval([f64; 2]),
    Optional,
    Padding(Padding),
    Required,
    String(&'static str),
    U32(u32),
}

enum PropertyType {
    Any,
    Bool,
    Dict,
    F64,
    F64x3,
    F64x3x3,
    Interval,
    Padding,
    String,
    Strings,
    U32,
}

pub enum PropertyValue<'py> {
    Any(Bound<'py, PyAny>),
    Bool(bool),
    Dict(DictLike<'py>),
    F64(f64),
    F64x3(f64x3),
    F64x3x3(f64x3x3),
    Interval([f64; 2]),
    None,
    Padding(Padding),
    String(String),
    Strings(Vec<String>),
    U32(u32),
}

impl<const N: usize> Extractor<N> {
    pub fn extract<'a, 'py>(
        &self,
        tag: &Tag,
        dict: &'a DictLike<'py>,
        mut remainder: Option<&mut IndexMap<String, Bound<'py, PyAny>>>,
    ) -> PyResult<[PropertyValue<'py>; N]> {
        // Resolve dict object.
        let (dict, tag) = tag.resolve(dict)?;

        // Extract properties from (key, value).
        let mut values: [PropertyValue; N] = std::array::from_fn(|_| PropertyValue::None);
        'items: for (k, v) in dict.iter() {
            let k: String = extract(&k)
                .or_else(|| tag.bad().what("key").into())?;
            for (index, property) in self.properties.iter().enumerate() {
                if k == property.name {
                    values[index] = property.extract(&tag, &v)?;
                    continue 'items;
                }
            }
            match remainder.as_mut() {
                None => {
                    let err = tag.bad().why(format!(
                        "unknown property '{}'",
                        k
                    )).to_err(ErrorKind::TypeError);
                    return Err(err);
                },
                Some(remainder) => {
                    let _unused = remainder.insert(k, v);
                },
            }
        }

        // Check for undefined properties, and apply default values.
        for index in 0..N {
            if values[index].is_none() {
                let default = &self.properties[index].default;
                if default.is_required() {
                    let err = tag.bad().why(format!(
                        "missing '{}' property",
                        self.properties[index].name,
                    )).to_err(ErrorKind::TypeError);
                    return Err(err);
                } else {
                    values[index] = default.into();
                }
            }
        }

        Ok(values)
    }

    pub fn extract_any<'a, 'py>(
        &self,
        tag: &Tag,
        any: &'a Bound<'py, PyAny>,
        remainder: Option<&mut IndexMap<String, Bound<'py, PyAny>>>,
    ) -> PyResult<[PropertyValue<'py>; N]> {
        let dict: DictLike = extract(any)
            .or_else(|| tag.bad().what("properties").into())?;
        self.extract(tag, &dict, remainder)
    }

    pub const fn new(properties: [Property; N]) -> Self {
        Self { properties }
    }
}

#[allow(dead_code)]
impl Property {
    #[inline]
    const fn new(name: &'static str, tp: PropertyType, default: PropertyDefault) -> Self {
        Self { name, tp, default }
    }

    // Defaulted constructors.
    pub const fn new_bool(name: &'static str, default: bool) -> Self {
        let tp = PropertyType::Bool;
        let default = PropertyDefault::Bool(default);
        Self::new(name, tp, default)
    }

    pub const fn new_f64(name: &'static str, default: f64) -> Self {
        let tp = PropertyType::F64;
        let default = PropertyDefault::F64(default);
        Self::new(name, tp, default)
    }

    pub const fn new_interval(name: &'static str, default: [f64; 2]) -> Self {
        let tp = PropertyType::Interval;
        let default = PropertyDefault::Interval(default);
        Self::new(name, tp, default)
    }

    pub const fn new_mat(name: &'static str, default: f64x3x3) -> Self {
        let tp = PropertyType::F64x3x3;
        let default = PropertyDefault::F64x3x3(default);
        Self::new(name, tp, default)
    }

    pub const fn new_str(name: &'static str, default: &'static str) -> Self {
        let tp = PropertyType::String;
        let default = PropertyDefault::String(default);
        Self::new(name, tp, default)
    }

    pub const fn new_u32(name: &'static str, default: u32) -> Self {
        let tp = PropertyType::U32;
        let default = PropertyDefault::U32(default);
        Self::new(name, tp, default)
    }

    pub const fn new_vec(name: &'static str, default: f64x3) -> Self {
        let tp = PropertyType::F64x3;
        let default = PropertyDefault::F64x3(default);
        Self::new(name, tp, default)
    }

    // Optional constructors.
    pub const fn optional_any(name: &'static str) -> Self {
        let tp = PropertyType::Any;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_bool(name: &'static str) -> Self {
        let tp = PropertyType::Bool;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_dict(name: &'static str) -> Self {
        let tp = PropertyType::Dict;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_f64(name: &'static str) -> Self {
        let tp = PropertyType::F64;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_mat(name: &'static str) -> Self {
        let tp = PropertyType::F64x3x3;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_padding(name: &'static str) -> Self {
        let tp = PropertyType::Padding;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_str(name: &'static str) -> Self {
        let tp = PropertyType::String;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_strs(name: &'static str) -> Self {
        let tp = PropertyType::Strings;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_u32(name: &'static str) -> Self {
        let tp = PropertyType::U32;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_vec(name: &'static str) -> Self {
        let tp = PropertyType::F64x3;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    // Required constructors.
    pub const fn required_dict(name: &'static str) -> Self {
        let tp = PropertyType::Dict;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_f64(name: &'static str) -> Self {
        let tp = PropertyType::F64;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_mat(name: &'static str) -> Self {
        let tp = PropertyType::F64x3x3;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_str(name: &'static str) -> Self {
        let tp = PropertyType::String;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_strs(name: &'static str) -> Self {
        let tp = PropertyType::Strings;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_u32(name: &'static str) -> Self {
        let tp = PropertyType::U32;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_vec(name: &'static str) -> Self {
        let tp = PropertyType::F64x3;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub fn extract<'a, 'py>(
        &self,
        tag: &Tag,
        value: &'a Bound<'py, PyAny>
    ) -> PyResult<PropertyValue<'py>> {
        let bad_property = || -> String {
            let what = format!("'{}'", self.name);
            tag.bad().what(&what).into()
        };
        let value = match &self.tp {
            PropertyType::Any => {
                let value: Bound<PyAny> = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Any(value)
            },
            PropertyType::Bool => {
                let value: bool = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Bool(value)
            },
            PropertyType::Dict => {
                let value: DictLike = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Dict(value)
            },
            PropertyType::F64 => {
                let value: f64 = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::F64(value)
            },
            PropertyType::F64x3 => {
                let value: Vector = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::F64x3(value.into_vec())
            },
            PropertyType::F64x3x3 => {
                let value: Rotation = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::F64x3x3(value.into_mat())
            },
            PropertyType::Padding => {
                let value: Padding = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Padding(value)
            },
            PropertyType::Interval => {
                let value: [f64; 2] = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Interval(value)
            },
            PropertyType::String => {
                let value: String = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::String(value)
            },
            PropertyType::Strings => {
                let value: Strings = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Strings(value.into_vec())
            },
            PropertyType::U32 => {
                let value: u32 = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::U32(value)
            },
        };
        Ok(value)
    }
}

#[derive(FromPyObject)]
pub enum Vector {
    #[pyo3(transparent, annotation = "float")]
    Float(f64),
    #[pyo3(transparent, annotation = "[float;3]")]
    Vec(f64x3),
}

impl Vector {
    pub fn into_vec(self) -> f64x3 {
        match self {
            Self::Float(f) => f64x3::splat(f),
            Self::Vec(v) => v,
        }
    }
}

#[derive(FromPyObject)]
pub enum Rotation {
    #[pyo3(transparent, annotation = "[[float;3];3]")]
    Matrix(f64x3x3),
    #[pyo3(transparent, annotation = "[float;3]")]
    Vector(f64x3),
}

impl Rotation {
    pub fn into_mat(self) -> f64x3x3 {
        match self {
            Self::Matrix(m) => m,
            Self::Vector(v) => {
                let v = Vector3::<f64>::from_iterator(v.as_ref().iter().map(|v| *v));
                let angle = v.norm().to_radians();
                let axis = Unit::new_normalize(v);
                let rotation = Rotation3::from_axis_angle(&axis, angle);
                let rotation: [[f64; 3]; 3] = rotation.into_inner().into();
                rotation.into()
            },
        }
    }
}

#[derive(Clone, Copy, FromPyObject)]
pub enum Padding {
    #[pyo3(transparent, annotation = "[float;6]")]
    Float6([f64; 6]),
    #[pyo3(transparent, annotation = "[float;3]")]
    Float3([f64; 3]),
    #[pyo3(transparent, annotation = "float")]
    Float(f64),
}

impl Padding {
    pub fn into_array(self) -> [f64; 6] {
        match self {
            Self::Float(f) => [f; 6],
            Self::Float3(a) => [a[0], a[0], a[1], a[1], a[2], a[2]],
            Self::Float6(a) => a,
        }
    }
}

#[derive(FromPyObject)]
pub enum Strings {
    #[pyo3(transparent, annotation = "str")]
    Scalar(String),
    #[pyo3(transparent, annotation = "[str]")]
    Vec(Vec<String>),
}

impl Strings {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Scalar(s) => vec![s],
            Self::Vec(v) => v,
        }
    }
}

impl PropertyDefault {
    pub fn is_required(&self) -> bool {
        match self {
            Self::Required => true,
            _ => false,
        }
    }
}

impl<'py> PropertyValue<'py> {
    pub fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }
}

impl<'py> From<&PropertyDefault> for PropertyValue<'py> {
    fn from(value: &PropertyDefault) -> Self {
        match value {
            PropertyDefault::Bool(value) => Self::Bool(*value),
            PropertyDefault::F64(value) => Self::F64(*value),
            PropertyDefault::F64x3(value) => Self::F64x3(*value),
            PropertyDefault::F64x3x3(value) => Self::F64x3x3(*value),
            PropertyDefault::Interval(value) => Self::Interval(*value),
            PropertyDefault::Optional => Self::None,
            PropertyDefault::Padding(value) => Self::Padding(*value),
            PropertyDefault::String(value) => Self::String(value.to_string()),
            PropertyDefault::U32(value) => Self::U32(*value),
            _ => unreachable!()
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Bound<'py, PyAny> {
    fn from(value: PropertyValue<'py>) -> Bound<'py, PyAny> {
        match value {
            PropertyValue::Any(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<Bound<'py, PyAny>> {
    fn from(value: PropertyValue<'py>) -> Option<Bound<'py, PyAny>> {
        match value {
            PropertyValue::Any(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for bool {
    fn from(value: PropertyValue<'py>) -> bool {
        match value {
            PropertyValue::Bool(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<bool> {
    fn from(value: PropertyValue<'py>) -> Option<bool> {
        match value {
            PropertyValue::Bool(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for DictLike<'py> {
    fn from(value: PropertyValue<'py>) -> DictLike<'py> {
        match value {
            PropertyValue::Dict(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<DictLike<'py>> {
    fn from(value: PropertyValue<'py>) -> Option<DictLike<'py>> {
        match value {
            PropertyValue::Dict(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for f64 {
    fn from(value: PropertyValue<'py>) -> f64 {
        match value {
            PropertyValue::F64(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<f64> {
    fn from(value: PropertyValue<'py>) -> Option<f64> {
        match value {
            PropertyValue::F64(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for f64x3 {
    fn from(value: PropertyValue<'py>) -> f64x3 {
        match value {
            PropertyValue::F64x3(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<f64x3> {
    fn from(value: PropertyValue<'py>) -> Option<f64x3> {
        match value {
            PropertyValue::F64x3(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for f64x3x3 {
    fn from(value: PropertyValue<'py>) -> f64x3x3 {
        match value {
            PropertyValue::F64x3x3(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<f64x3x3> {
    fn from(value: PropertyValue<'py>) -> Option<f64x3x3> {
        match value {
            PropertyValue::F64x3x3(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for [f64; 2] {
    fn from(value: PropertyValue<'py>) -> [f64; 2] {
        match value {
            PropertyValue::Interval(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<[f64; 2]> {
    fn from(value: PropertyValue<'py>) -> Option<[f64; 2]> {
        match value {
            PropertyValue::Interval(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Padding {
    fn from(value: PropertyValue<'py>) -> Padding {
        match value {
            PropertyValue::Padding(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<Padding> {
    fn from(value: PropertyValue<'py>) -> Option<Padding> {
        match value {
            PropertyValue::Padding(value) => Some(value),
            PropertyValue::None => None,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for String {
    fn from(value: PropertyValue<'py>) -> String {
        match value {
            PropertyValue::String(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<String> {
    fn from(value: PropertyValue<'py>) -> Option<String> {
        match value {
            PropertyValue::None => None,
            PropertyValue::String(value) => Some(value),
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Vec<String> {
    fn from(value: PropertyValue<'py>) -> Vec<String> {
        match value {
            PropertyValue::None => Vec::new(),
            PropertyValue::Strings(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for u32 {
    fn from(value: PropertyValue<'py>) -> u32 {
        match value {
            PropertyValue::U32(value) => value,
            _ => unreachable!(),
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Option<u32> {
    fn from(value: PropertyValue<'py>) -> Option<u32> {
        match value {
            PropertyValue::None => None,
            PropertyValue::U32(value) => Some(value),
            _ => unreachable!(),
        }
    }
}


// ===============================================================================================
//
// Extract from a Python object (with a formatted error)
//
// ===============================================================================================

pub fn extract<'a, 'py, T>(
    ob: &'a Bound<'py, PyAny>
) -> ExtractResult<'a, 'py, T>
where
    T: FromPyObject<'py>,
{
    let result = T::extract_bound(ob)
        .map_err(|_| ob);
    ExtractResult { result, expected: None }
}

pub struct ExtractResult<'a, 'py, T> {
    result: Result<T, &'a Bound<'py, PyAny>>,
    expected: Option<&'static str>,
}

impl<'a, 'py, T> ExtractResult<'a, 'py, T>
where
    T: FromPyObject<'py> + TypeName,
{
    pub fn expect(mut self, typename: &'static str) -> Self {
        if let Err(_) = self.result.as_ref() {
            self.expected = Some(typename);
        }
        self
    }

    pub fn or(self, message: &str) -> PyResult<T> {
        let expected = self.expected
            .unwrap_or_else(T::type_name);
        let value: T = self.result.map_err(|ob| {
            let message = format!(
                "{} (expected {}, found '{}')",
                message,
                expected,
                ob,
            );
            PyValueError::new_err(message)
        })?;
        Ok(value)
    }

    pub fn or_else<M>(self, message: M) -> PyResult<T>
    where
        M: FnOnce() -> String,
    {
        let message = message();
        self.or(&message)
    }
}

pub trait TypeName {
    fn type_name() -> &'static str;
}

impl <'py, T> TypeName for Bound<'py, T>
where
    T: TypeName
{
    fn type_name() -> &'static str {
        T::type_name()
    }
}

impl <'a, T> TypeName for &'a T
where
    T: TypeName
{
    fn type_name() -> &'static str {
        T::type_name()
    }
}

impl TypeName for bool {
    fn type_name() -> &'static str { "a 'bool'" }
}

impl TypeName for f64 {
    fn type_name() -> &'static str { "a 'float'" }
}

impl TypeName for [f64; 2] {
    fn type_name() -> &'static str { "an 'interval'" }
}

impl TypeName for Vector {
    fn type_name() -> &'static str { "a (vector of) 'float'" }
}

impl TypeName for Padding {
    fn type_name() -> &'static str { "an (array of) 'float'" }
}

impl TypeName for Rotation {
    fn type_name() -> &'static str { "a 'rotation'" }
}

impl TypeName for PyAny {
    fn type_name() -> &'static str { "an 'object'" }
}

impl<'py> TypeName for DictLike<'py> {
    fn type_name() -> &'static str { "a 'dict' or a 'str'" }
}

impl TypeName for String {
    fn type_name() -> &'static str { "a 'str'" }
}

impl TypeName for Strings {
    fn type_name() -> &'static str { "a (sequence of) 'str'" }
}

impl TypeName for u32 {
    fn type_name() -> &'static str { "an 'int'" }
}
