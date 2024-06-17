use crate::utils::error::variant_explain;
use crate::utils::units::{self, UnitError};
use enum_variants_strings::EnumVariantsStrings;
use pyo3::prelude::*;
use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use pyo3::types::PyDict;
use regex::{Captures, CaptureMatches, Regex};
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;


pub fn load_gate_db<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
    // Load text file.
    let text = std::fs::read_to_string(path)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => {
                let path = format!("No such file or directory '{}'", path.display());
                PyFileNotFoundError::new_err(path)
            },
            _ => err.into(),
        })?;

    // Process lines.
    Processor::new(py, &text)
        .process()
        .map_err(|err| PyValueError::new_err(err.format(path)))
}

struct Processor<'a, 'py> {
    // Parsers.
    attrs_parser: AttributesParser,
    re_blank: Regex,
    re_line: Regex,
    re_section: Regex,

    // State variables.
    py: Python<'py>,
    text: &'a str,
    section: Section,

    // Parsed items.
    elements: Bound<'py, PyDict>,
    molecules: Bound<'py, PyDict>,
    mixtures: Bound<'py, PyDict>,

    // Current material (being parsed over multiple lines).
    material: Option<MaterialData<'py>>,
}

struct ProcessingError {
    line: usize,
    message: String,
}

#[derive(EnumVariantsStrings)]
#[enum_variants_strings_transform(transform="none")]
enum Section {
    Elements,
    Materials,
}

impl<'a, 'py> Processor<'a, 'py> {
    pub fn new(py: Python<'py>, text: &'a str) -> Self {
        let attrs_parser = AttributesParser::new();
        let re_blank = Regex::new(r"^[\s\r\n]*$").unwrap();
        let re_line = Regex::new(r"^\s*(\+?\w+)\s*:\s*(.*)").unwrap();
        let re_section = Regex::new(r"^\s*\[([\w_]+)\]").unwrap();

        let section = Section::Materials;

        let elements = PyDict::new_bound(py);
        let molecules = PyDict::new_bound(py);
        let mixtures = PyDict::new_bound(py);

        Self {
            attrs_parser,
            re_blank,
            re_line,
            re_section,
            py,
            text,
            section,
            elements,
            molecules,
            mixtures,
            material: None,
        }
    }

    pub fn process(&mut self) -> Result<Bound<'py, PyDict>, ProcessingError> {
        for (i, line) in self.text.lines().enumerate() {
            if let Err(msg) = self.process_line(line) {
                let lineno = i + 1;
                return Err(ProcessingError::new(lineno, msg));
            }
        }

        let db = PyDict::new_bound(self.py);
        db.set_item("elements", self.elements.as_any()).unwrap();
        db.set_item("molecules", self.molecules.as_any()).unwrap();
        db.set_item("mixtures", self.mixtures.as_any()).unwrap();

        Ok(db)
    }

    fn process_line(&mut self, line: &'a str) -> Result<(), String> {
        if let Some(_) = self.re_blank.captures(line) { // ignore blank lines.
            return Ok(())
        }

        match self.re_section.captures(line) {
            None => {
                let captures = match self.re_line.captures(line) {
                    None => return Err("unexpected statement".to_string()),
                    Some(captures) => captures,
                };
                let header = captures.get(1).unwrap().as_str();
                let body = captures.get(2).unwrap().as_str();
                match self.section {
                    Section::Elements => self.process_element(header, body)?,
                    Section::Materials => self.process_material(header, body)?,
                }
            },
            Some(captures) => {
                self.section = captures
                    .get(1)
                    .unwrap()
                    .as_str()
                    .try_into()?;
            },
        }

        Ok(())
    }

    fn process_element(&mut self, head: &str, body: &str) -> Result<(), String> {
        let element = PyDict::new_bound(self.py);
        for attr in self.attrs_parser.iter(body) {
            match attr.key {
                "S" | "Symbol" => element.set_item("symbol", attr.value).unwrap(),
                "Z" => {
                    let value: f64 = attr.value.parse()
                        .map_err(|err: ParseFloatError| wrap(&attr, err))?;
                    element.set_item("Z", value).unwrap();
                },
                "A" => {
                    let value = parse_quantity(self.py, attr.value, "g/mole")
                        .map_err(|err: QuantityError| wrap(&attr, err))?;
                    element.set_item("A", value).unwrap();
                },
                _ => return Err(format!("unknown attribute '{}'", attr.key)),
            }
        }
        self.elements.set_item(head, element).unwrap();
        Ok(())
    }

    fn process_material(&mut self, head: &str, body: &str) -> Result<(), String> {
        if head.starts_with("+") {
            match self.material.as_mut() {
                None => return Err("bad component (no associated material)".to_string()),
                Some(material) => {
                    if material.found_components >= material.expected_components {
                        return Err(format!(
                            "bad component for material '{}' (expected {}, found {} or more)",
                            material.name,
                            material.expected_components,
                            material.found_components + 1,
                        ));
                    }
                    let mut name: Option<&str> = None;
                    let mut weight: Option<Weight> = None;
                    for attr in self.attrs_parser.iter(body) {
                        match attr.key {
                            "name" => {
                                let n = match head {
                                    "+el" => {
                                        if attr.value == "auto" {
                                            material.name.as_str()
                                        } else {
                                            attr.value 
                                        }
                                    },
                                    "+mat" => attr.value,
                                    _ => return Err(format!(
                                        "bad component for material '{}' \
                                         (expected '+el' or '+mat', found '{}')",
                                        material.name,
                                        head,
                                    )),
                                };
                                name = Some(n);
                            },
                            "n" => weight = Some(Weight::Multiplicity(attr.value.parse()
                                .map_err(|err: ParseIntError| wrap(&attr, err))?)),
                            "f" | "fraction" => weight = Some(Weight::Fraction(attr.value.parse()
                                .map_err(|err: ParseFloatError| wrap(&attr, err))?)),
                            _ => return Err(format!(
                                    "unknown attribute '{}' for material '{}'",
                                    attr.key,
                                    material.name,
                                )),
                        }
                    }
                    let name = name
                        .ok_or_else(|| format!(
                            "bad component for material '{}' (missing 'name' attribute)",
                            material.name,
                        ))?;
                    let weight = weight
                        .ok_or_else(|| format!(
                            "bad component for material '{}' (missing 'n' or 'f' attribute)",
                            material.name,
                        ))?;

                    let tp = match weight {
                        Weight::Fraction(_) => MaterialType::Mixture,
                        Weight::Multiplicity(_) => MaterialType::Molecule,
                    };
                    if material.found_components > 0 {
                        if material.tp != tp {
                            let (expected, found) = match material.tp {
                                MaterialType::Molecule => ("n", "f"),
                                MaterialType::Mixture => ("f", "n"),
                                _ => unreachable!(),
                            };
                            return Err(format!(
                                "bad component for material '{}' \
                                 (expected '{}' attribute, found '{}')",
                                material.name,
                                expected,
                                found,
                            ));
                        }
                    } else {
                        material.tp = tp;
                    }

                    material.components.set_item(name, weight).unwrap();
                    material.found_components += 1;
                    if material.found_components == material.expected_components {
                        material.attributes.set_item(
                            "composition",
                            material.components.clone(),
                        ).unwrap();
                        let container = match material.tp {
                            MaterialType::Molecule => &self.molecules,
                            MaterialType::Mixture => &self.mixtures,
                            _ => unreachable!(),
                        };
                        container.set_item(
                            material.name.clone(),
                            material.attributes.clone(),
                        ).unwrap();
                    }
                },
            }
        } else {
            if let Some(material) = &self.material {
                if material.found_components < material.expected_components {
                    return Err(format!(
                        "too few component(s) for material '{}' (expected {}, found {})",
                        material.name,
                        material.expected_components,
                        material.found_components,
                    ));
                }
            }

            let mut material = MaterialData::new(self.py, head);
            for attr in self.attrs_parser.iter(body) {
                match attr.key {
                    "d" | "density" => {
                        let value = parse_quantity(self.py, attr.value, "g/cm3")
                            .map_err(|err: QuantityError| wrap(&attr, err))?;
                        material.attributes.set_item("density", value).unwrap();
                    },
                    "n" => {
                        material.expected_components = attr.value.parse()
                            .map_err(|err: ParseIntError| wrap(&attr, err))?;
                    },
                    "s" | "state" => {
                        material.attributes.set_item("state", attr.value).unwrap();
                    },
                    _ => return Err(format!(
                        "bad '{}' material (unknown attribute '{}')",
                        material.name,
                        attr.key
                    )),
                }
            }
            self.material = Some(material);
        }

        Ok(())
    }
}

enum Weight {
    Fraction(f64),
    Multiplicity(i32),
}

impl ToPyObject for Weight {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        match self {
            Self::Fraction(val) => val.to_object(py),
            Self::Multiplicity(val) => val.to_object(py),
        }
    }
}

impl ProcessingError {
    fn format(&self, path: &Path) -> String {
        format!("{}:{}: {}", path.display(), self.line, self.message)
    }

    fn new(line: usize, message: String) -> Self {
        Self { line, message }
    }
}

impl TryFrom<&str> for Section {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Section::from_str(value)
            .map_err(|options| {
                let why = variant_explain(value, options);
                format!("bad section ({})", why)
            })
    }
}


// ===============================================================================================
//
// Attributes parser (for db files).
//
// ===============================================================================================

struct AttributesParser (Regex);

impl AttributesParser {
    fn new() -> Self {
        let re = Regex::new(r"([A-Za-z]+)\s*=\s*([\w\d.\s/]+)").unwrap();
        Self (re)
    }

    fn iter<'r, 'h>(&'r self, text: &'h str) -> AttributesIterator<'r, 'h> {
        AttributesIterator (self.0.captures_iter(text))
    }
}

struct AttributesIterator<'r, 'h> (CaptureMatches<'r, 'h>);

impl<'r, 'h> Iterator for AttributesIterator<'r, 'h> {
    type Item = Attribute<'h>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|captures| Self::Item::new(&captures))
    }
}

struct Attribute<'h> {
    key: &'h str,
    value: &'h str,
}

impl<'h> Attribute<'h> {
    fn new(captures: &Captures<'h>) -> Self {
        let key = captures.get(1).unwrap().as_str();
        let value = captures.get(2).unwrap().as_str().trim();
        Self { key, value }
    }
}

fn wrap<T: fmt::Display>(attr: &Attribute, err: T) -> String {
    format!("bad '{}' attribute value ({})", attr.key, err)
}


// ===============================================================================================
//
// Materials related utilities.
//
// ===============================================================================================

struct MaterialData<'py> {
    name: String,
    expected_components: usize,
    found_components: usize,
    tp: MaterialType,
    attributes: Bound<'py, PyDict>,
    components: Bound<'py, PyDict>,
}

#[derive(PartialEq)]
enum MaterialType {
    Molecule,
    Mixture,
    Unknown,
}

impl<'py> MaterialData<'py> {
    fn new(py: Python<'py>, name: &str) -> Self {
        Self {
            name: name.to_string(),
            expected_components: 0,
            found_components: 0,
            tp: MaterialType::Unknown,
            attributes: PyDict::new_bound(py),
            components: PyDict::new_bound(py),
        }
    }
}


// ===============================================================================================
//
// Physical quantities parser.
//
// ===============================================================================================

fn parse_quantity(
    py: Python,
    value: &str,
    target: &str
) -> Result<f64, QuantityError> {
    let delimiter = Regex::new(r"\s+").unwrap();
    let elements: Vec<_> = delimiter
        .splitn(value, 2)
        .collect();
    if elements.len() < 2 {
        return Err(QuantityError::Missing);
    }
    let raw_value: f64 = match elements[0].parse() {
        Ok(value) => value,
        Err(err) => return Err(QuantityError::Parse(err)),
    };
    let unit_name = elements[1].trim();
    let unit_value = match units::convert(py, unit_name, target) {
        Ok(value) => value,
        Err(err) => return Err(QuantityError::Unit(err)),
    };
    Ok(raw_value * unit_value)
}

enum QuantityError {
    Missing,
    Parse(ParseFloatError),
    Unit(UnitError),
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "missing unit"),
            Self::Parse(err) => err.fmt(f),
            Self::Unit(err) => err.fmt(f),
        }
    }
}
