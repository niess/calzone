use indexmap::IndexMap; // XXX needed?
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyDict;
use std::borrow::Cow;


// ===============================================================================================
//
// Generic extraction from a Python bound object.
//
// ===============================================================================================

pub trait TryFromBound {
    const TYPE_NAME: &'static str;

    // Note that, despite trait functions all have default implementations, at least one of
    // `try_from_any` or `try_from_dict` must be overriden.

    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        Self: Sized
    {
        let value: Bound<PyDict> = extract(value)
            .or_else(|| format!(
                "bad properties for {} '{}'",
                <Self as TryFromBound>::TYPE_NAME,
                tag.path(),
            ))?;
        Self::try_from_dict(tag, &value)
    }

    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self>
    where
        Self: Sized
    {
        Self::try_from_any(tag, value.as_any())
    }
}

impl<T> TryFromBound for Vec<T>
where
    T: TryFromBound + Sized,
{
    const TYPE_NAME: &'static str = T::TYPE_NAME;

    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self>
    where
        Self: Sized
    {
        let mut items = Vec::<T>::with_capacity(value.len());
        for (k, v) in value.iter() {
            let name: String = extract(&k)
                .or_else(|| format!(
                    "bad {}",
                    <T as TryFromBound>::TYPE_NAME,
                ))?;
            let tag = tag.extend(&name);
            let item = T::try_from_any(&tag, &v)?;
            items.push(item);
        }
        Ok(items)
    }
}

/// A contextual `Tag` enclosing the name and the path of the object being extracted.
pub struct Tag<'a> {
    name: &'a str,
    path: Cow<'a, str>,
}

impl<'a> Tag<'a> {
    /// Returns an empty `Tag`.
    pub fn empty() -> Self {
        const EMPTY: &'static str = "";
        Self::new(EMPTY)
    }

    /// Returns a new `Tag` with a path extended by `value`.
    pub fn extend(&self, value: &'a str) -> Self {
        if self.name.is_empty() {
            Self::new(value)
        } else {
            let path = format!("{}.{}", self.path(), value);
            let path = Cow::Owned(path);
            Self { name: value, path }
        }
    }

    /// Returns the name of this `Tag`.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns a new `Tag` initialised with `name`.
    pub fn new(name: &'a str) -> Self {
        let path = Cow::Borrowed(name);
        Self { name, path }
    }

    /// Returns the path of this `Tag`.
    pub fn path<'b>(&'b self) -> &'b str {
        &self.path
    }
}


// ===============================================================================================
//
// Procedural properties extractor (from a Python dict).
//
// ===============================================================================================

pub struct Extractor<const N: usize> {
    context: &'static str,
    properties: [Property; N],
}

pub struct Property {
    name: &'static str,
    tp: PropertyType,
    default: PropertyDefault,
}

enum PropertyDefault {
    F64(f64),
    Optional,
    Required,
    String(&'static str),
    U32(u32),
}

enum PropertyType {
    Dict,
    F64,
    String,
    U32,
}

pub enum PropertyValue<'py> {
    Dict(Bound<'py, PyDict>),
    F64(f64),
    None,
    String(String),
    U32(u32),
}

impl<const N: usize> Extractor<N> {
    pub fn extract<'a, 'py>(
        &self,
        name: &str,
        dict: &'a Bound<'py, PyDict>
    ) -> PyResult<[PropertyValue<'py>; N]> {

        // Extract properties from (key, value).
        let context = format!("{} '{}'", self.context, name);
        let mut values: [PropertyValue; N] = std::array::from_fn(|_| PropertyValue::None);
        'items: for (k, v) in dict.iter() {
            let k: String = extract(&k)
                .or_else(|| format!("bad key for {}", context))?;
            for (index, property) in self.properties.iter().enumerate() {
                if k == property.name {
                    values[index] = property.extract(context.as_str(), &v)?;
                    continue 'items;
                }
            }
            let message = format!(
                "bad {} (unknown property '{}')",
                context,
                k,
            );
            return Err(PyValueError::new_err(message));
        }

        // Check for undefined properties, and apply default values.
        for index in 0..N {
            if values[index].is_none() {
                let default = &self.properties[index].default;
                if default.is_required() {
                    let message = format!(
                        "bad {} (missing '{}' property)",
                        context,
                        self.properties[index].name,
                    );
                    return Err(PyValueError::new_err(message));
                } else {
                    values[index] = default.into();
                }
            }
        }

        Ok(values)
    }

    pub const fn new(context: &'static str, properties: [Property; N]) -> Self {
        Self { context, properties }
    }
}

impl Property {
    #[inline]
    const fn new(name: &'static str, tp: PropertyType, default: PropertyDefault) -> Self {
        Self { name, tp, default }
    }

    // Defaulted constructors.
    pub const fn new_f64(name: &'static str, default: f64) -> Self {
        let tp = PropertyType::F64;
        let default = PropertyDefault::F64(default);
        Self::new(name, tp, default)
    }

    pub const fn new_str(name: &'static str, default: &'static str) -> Self {
        let tp = PropertyType::F64;
        let default = PropertyDefault::String(default);
        Self::new(name, tp, default)
    }

    pub const fn new_u32(name: &'static str, default: u32) -> Self {
        let tp = PropertyType::U32;
        let default = PropertyDefault::U32(default);
        Self::new(name, tp, default)
    }

    // Optional constructors.
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

    pub const fn optional_str(name: &'static str) -> Self {
        let tp = PropertyType::String;
        let default = PropertyDefault::Optional;
        Self::new(name, tp, default)
    }

    pub const fn optional_u32(name: &'static str) -> Self {
        let tp = PropertyType::U32;
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

    pub const fn required_str(name: &'static str) -> Self {
        let tp = PropertyType::String;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub const fn required_u32(name: &'static str) -> Self {
        let tp = PropertyType::U32;
        let default = PropertyDefault::Required;
        Self::new(name, tp, default)
    }

    pub fn extract<'a, 'py>(
        &self,
        context: &str,
        value: &'a Bound<'py, PyAny>
    ) -> PyResult<PropertyValue<'py>> {
        let value = match &self.tp {
            PropertyType::Dict => {
                let value: Bound<PyDict> = extract(value)
                    .or_else(|| format!(
                        "bad '{}' for {}", self.name, context,
                    ))?;
                PropertyValue::Dict(value)
            },
            PropertyType::F64 => {
                let value: f64 = extract(value)
                    .or_else(|| format!(
                        "bad '{}' for {}", self.name, context,
                    ))?;
                PropertyValue::F64(value)
            },
            PropertyType::String => {
                let value: String = extract(value)
                    .or_else(|| format!(
                        "bad '{}' for {}", self.name, context,
                    ))?;
                PropertyValue::String(value)
            },
            PropertyType::U32 => {
                let value: u32 = extract(value)
                    .or_else(|| format!(
                        "bad '{}' for {}", self.name, context,
                    ))?;
                PropertyValue::U32(value)
            },
        };
        Ok(value)
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
            PropertyDefault::F64(value) => Self::F64(*value),
            PropertyDefault::Optional => Self::None,
            PropertyDefault::String(value) => Self::String(value.to_string()),
            PropertyDefault::U32(value) => Self::U32(*value),
            _ => unreachable!()
        }
    }
}

impl<'py> From<PropertyValue<'py>> for Bound<'py, PyDict> {
    fn from(value: PropertyValue<'py>) -> Bound<'py, PyDict> {
        match value {
            PropertyValue::Dict(value) => value,
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

impl<'py> From<PropertyValue<'py>> for String {
    fn from(value: PropertyValue<'py>) -> String {
        match value {
            PropertyValue::String(value) => value,
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


// ===============================================================================================
//
// Extract from a Python object (with a formatted error)
//
// ===============================================================================================

fn extract<'a, 'py, T>(
    ob: &'a Bound<'py, PyAny>
) -> ExtractResult<'a, 'py, T>
where
    T: FromPyObject<'py>,
{
    let result = T::extract_bound(ob)
        .map_err(|_| ob);
    ExtractResult (result)
}

struct ExtractResult<'a, 'py, T> (Result<T, &'a Bound<'py, PyAny>>);

impl<'a, 'py, T> ExtractResult<'a, 'py, T>
where
    T: FromPyObject<'py> + TypeName,
{
    pub fn or(self, message: &str) -> PyResult<T> {
        let value: T = self.0.map_err(|ob| {
            let message = format!(
                "{} (expected {}, found {})",
                message,
                T::type_name(),
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

trait TypeName {
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

// XXX 'a' or 'an'.

impl TypeName for PyDict {
    fn type_name() -> &'static str { "a dict" }
}

impl TypeName for String {
    fn type_name() -> &'static str { "a string" }
}

impl TypeName for f64 {
    fn type_name() -> &'static str { "a float" }
}

impl TypeName for u32 {
    fn type_name() -> &'static str { "an int" }
}
