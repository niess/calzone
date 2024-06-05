use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use std::collections::HashMap;
use std::fmt;
use super::ffi;


static UNITS: GILOnceCell<HashMap<String, f64>> = GILOnceCell::new();

pub fn initialise(py: Python) {
    let units = {
        let mut units = Vec::<ffi::UnitDefinition>::new();
        ffi::export_units(&mut units);
        units
    };

    let mut map = HashMap::<String, f64>::new();
    for unit in units {
        let the_same = unit.symbol == unit.name;
        let _unused = map.insert(unit.name, unit.value);
        if !the_same {
            let _unused = map.insert(unit.symbol, unit.value);
        }
    }

    let _unused = UNITS.set(py, map);
}

pub fn convert(py: Python, value: &str, target: &str) -> Result<f64, UnitError> {
    if value == target {
        Ok(1.0)
    } else {
        let units = UNITS.get(py).unwrap();
        let target = units.get(target).unwrap();
        let value = match units.get(value) {
            None => {
                return Err(UnitError::new(value));
            },
            Some(value) => value,
        };
        Ok(value / target)
    }
}

#[derive(Clone, Debug)]
pub struct UnitError (String);

impl UnitError {
    pub fn new(name: &str) -> Self {
        Self (name.to_string())
    }
}

impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unknown unit '{}'", &self.0)
    }
}
