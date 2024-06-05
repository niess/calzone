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
    // Note that, despite trait functions all have default implementations, at least one of
    // `try_from_any` or `try_from_dict` must be overriden.

    fn try_from_any<'py>(tag: &Tag, value: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        Self: Sized
    {
        let value: Bound<PyDict> = extract(value)
            .or_else(|| tag.bad().what("properties").into())?;
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
    fn try_from_dict<'py>(tag: &Tag, value: &Bound<'py, PyDict>) -> PyResult<Self>
    where
        Self: Sized
    {
        let mut items = Vec::<T>::with_capacity(value.len());
        for (k, v) in value.iter() {
            let name: String = extract(&k)
                .or_else(|| tag.bad_type())?;
            let tag = tag.extend(&name, None);
            let item = T::try_from_any(&tag, &v)?;
            items.push(item);
        }
        Ok(items)
    }
}

/// A contextual `Tag` enclosing the type, the name and the path of the object being extracted.
pub struct Tag<'a> {
    typename: &'a str,
    name: &'a str,
    path: Cow<'a, str>,
}

impl<'a> Tag<'a> {
    pub fn bad<'b>(&'b self) -> TaggedBad<'a, 'b> {
        TaggedBad::new(self)
    }

    pub fn bad_type(&self) -> String {
        format!("bad {}", self.typename)
    }

    pub fn cast<'b: 'a>(&'b self, typename: &'a str) -> Tag<'b> {
        let path = Cow::Borrowed(self.path.as_ref());
        Self { typename, name: self.name, path }
    }

    #[allow(dead_code)] // XXX needed?
    pub fn cite(&self) -> String {
        format!("'{}' {}", self.path, self.typename)
    }

    /// Returns an empty `Tag`.
    pub fn empty() -> Self {
        const EMPTY: &'static str = "";
        Self::new(EMPTY, EMPTY)
    }

    /// Returns a new `Tag` with a path extended by `value`, and optionally a different type.
    pub fn extend(&self, value: &'a str, typename: Option<&'a str>) -> Self {
        let typename = typename.unwrap_or(self.typename);
        if self.name.is_empty() {
            Self::new(typename, value)
        } else {
            let path = format!("{}.{}", self.path(), value);
            let path = Cow::Owned(path);
            Self { typename: typename, name: value, path }
        }
    }

    /// Returns the name of this `Tag`.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns a new `Tag` initialised with `name`.
    pub fn new(typename: &'a str, name: &'a str) -> Self {
        let path = Cow::Borrowed(name);
        Self { typename, name, path }
    }

    /// Returns the path of this `Tag`.
    pub fn path<'b>(&'b self) -> &'b str {
        &self.path
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
        match value.what {
            None => match value.why {
                None => format!("bad '{}' {}", value.tag.path, value.tag.typename),
                Some(why) => format!("bad '{}' {} ({})", value.tag.path, value.tag.typename, why),
            },
            Some(what) => match value.why {
                None => format!("bad {} for '{}' {}", what, value.tag.path, value.tag.typename),
                Some(why) => format!(
                    "bad {} for '{}' {} ({})", what, value.tag.path, value.tag.typename, why,
                ),
            },
        }
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

#[allow(dead_code)] // XXX needed?
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
        tag: &Tag,
        dict: &'a Bound<'py, PyDict>
    ) -> PyResult<[PropertyValue<'py>; N]> {

        // Extract properties from (key, value).
        let mut values: [PropertyValue; N] = std::array::from_fn(|_| PropertyValue::None);
        'items: for (k, v) in dict.iter() {
            let k: String = extract(&k)
                .or_else(|| tag.bad().what("key").into())?;
            for (index, property) in self.properties.iter().enumerate() {
                if k == property.name {
                    values[index] = property.extract(tag, &v)?;
                    continue 'items;
                }
            }
            let message: String = tag.bad().why(format!(
                "unknown property '{}'",
                k
            )).into();
            return Err(PyValueError::new_err(message));
        }

        // Check for undefined properties, and apply default values.
        for index in 0..N {
            if values[index].is_none() {
                let default = &self.properties[index].default;
                if default.is_required() {
                    let message: String = tag.bad().why(format!(
                        "missing '{}' property",
                        self.properties[index].name,
                    )).into();
                    return Err(PyValueError::new_err(message));
                } else {
                    values[index] = default.into();
                }
            }
        }

        Ok(values)
    }

    pub const fn new(properties: [Property; N]) -> Self {
        Self { properties }
    }
}

#[allow(dead_code)] // XXX needed?
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
        tag: &Tag,
        value: &'a Bound<'py, PyAny>
    ) -> PyResult<PropertyValue<'py>> {
        let bad_property = || -> String {
            let what = format!("'{}'", self.name);
            tag.bad().what(&what).into()
        };
        let value = match &self.tp {
            PropertyType::Dict => {
                let value: Bound<PyDict> = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::Dict(value)
            },
            PropertyType::F64 => {
                let value: f64 = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::F64(value)
            },
            PropertyType::String => {
                let value: String = extract(value)
                    .or_else(bad_property)?;
                PropertyValue::String(value)
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
