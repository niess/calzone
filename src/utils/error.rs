use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::{
    PyException, PyFileNotFoundError, PyKeyboardInterrupt, PyMemoryError, PyValueError
};
use pyo3::ffi::PyErr_CheckSignals;
use super::ffi;


pub fn initialise() {
    ffi::initialise_errors();
}

create_exception!(calzone, Geant4Exception, PyException);

impl ffi::Error {
    pub fn to_result(&self) -> PyResult<()> {
        match self.tp {
            ffi::ErrorType::None => Ok(()),
            ffi::ErrorType::FileNotFoundError => {
                Err(PyFileNotFoundError::new_err(self.message.clone()))
            },
            ffi::ErrorType::Geant4Exception => {
                Err(Geant4Exception::new_err(self.message.clone()))
            },
            ffi::ErrorType::KeyboardInterrupt => {
                Err(PyKeyboardInterrupt::new_err("Ctrl+C catched"))
            },
            ffi::ErrorType::MemoryError => {
                Err(PyMemoryError::new_err("could not allocate memory"))
            },
            ffi::ErrorType::ValueError => {
                Err(PyValueError::new_err(self.message.clone()))
            },
            _ => unreachable!(),
        }
    }

    pub fn value(&self) -> Option<&str> {
        match self.tp {
            ffi::ErrorType::None => None,
            ffi::ErrorType::FileNotFoundError => Some(self.message.as_str()),
            ffi::ErrorType::Geant4Exception => Some(self.message.as_str()),
            ffi::ErrorType::KeyboardInterrupt => Some("Ctrl+C catched"),
            ffi::ErrorType::MemoryError => Some("could not allocate memory"),
            ffi::ErrorType::ValueError => Some(self.message.as_str()),
            _ => unreachable!(),
        }
    }
}

pub fn ctrlc_catched() -> bool {
    if unsafe { PyErr_CheckSignals() } == -1 { true } else {false}
}

pub fn variant_error(header: &str, value: &str, options: &[&str]) -> PyErr {
    let explain = variant_explain(value, options);
    let message = format!(
        "{} ({})",
        header,
        explain,
    );
    PyValueError::new_err(message)
}

pub fn variant_explain(value: &str, options: &[&str]) -> String {
    let n = options.len();
    let options = match n {
        0 => unimplemented!(),
        1 => format!("'{}'", options[0]),
        2 => format!("'{}' or '{}'", options[0], options[1]),
        _ => {
            let options: Vec<_> = options
                .iter()
                .map(|e| format!("'{}'", e))
                .collect();
            format!(
                "{} or {}",
                options[0..(n - 1)].join(", "),
                options[n - 1],
            )
        },
    };
    format!(
        "expected one of {}, found '{}'",
        options,
        value
    )
}
