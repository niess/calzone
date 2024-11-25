use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::{
    PyException, PyFileNotFoundError, PyIndexError, PyIOError, PyKeyboardInterrupt, PyKeyError,
    PyMemoryError, PyNotImplementedError, PyTypeError, PyValueError
};
use pyo3::ffi::PyErr_CheckSignals;
use super::ffi;


pub fn initialise() {
    ffi::initialise_errors();
}


// ===============================================================================================
//
// Normalised errors.
//
// ===============================================================================================

#[derive(Default)]
pub struct Error<'a> {
    pub kind: Option<ErrorKind>,
    pub who: Option<&'a str>,
    pub what: Option<&'a str>,
    pub why: Option<&'a str>,
    pub r#where: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub enum ErrorKind {
    Exception,
    FileNotFoundError,
    Geant4Exception,
    IndexError,
    IOError,
    KeyboardInterrupt,
    KeyError,
    MemoryError,
    NotImplementedError,
    TypeError,
    ValueError,
}

impl<'a> Error<'a> {
    pub fn maybe_what(mut self, what: Option<&'a str>) -> Self {
        self.what = what;
        self
    }

    pub fn maybe_where(mut self, r#where: Option<&'a str>) -> Self {
        self.r#where = r#where;
        self
    }

    pub fn maybe_why(mut self, why: Option<&'a str>) -> Self {
        self.why = why;
        self
    }

    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind: Some(kind),
            who: None,
            what: None,
            why: None,
            r#where: None,
        }
    }

    pub fn to_err(&self) -> PyErr {
        self.into()
    }

    pub fn to_string(&self) -> String {
        self.into()
    }

    pub fn what(mut self, what: &'a str) -> Self {
        self.what = Some(what);
        self
    }

    pub fn r#where(mut self, r#where: &'a str) -> Self {
        self.r#where = Some(r#where);
        self
    }

    pub fn who(mut self, who: &'a str) -> Self {
        self.who = Some(who);
        self
    }

    pub fn why(mut self, why: &'a str) -> Self {
        self.why = Some(why);
        self
    }
}

impl<'a> From<&Error<'a>> for String {
    fn from(value: &Error<'a>) -> Self {
        let Error {who, what, why, r#where, ..} = value;
        let msg = match who {
            None => match what {
                None => match why {
                    None => "something bad happened".to_string(),
                    Some(why) => why.to_string(),
                }
                Some(what) => match why {
                    None => format!("bad {what}"),
                    Some(why) => format!("bad {what} ({why})"),
                },
            }
            Some(who) => match what {
                None => match why {
                    None => format!("bad {who}"),
                    Some(why) => format!("bad {who} ({why})"),
                },
                Some(what) => match why {
                    None => format!("bad {what} for {who}"),
                    Some(why) => format!("bad {what} for {who} ({why})"),
                },
            },
        };
        match r#where {
            None => msg,
            Some(r#where) => format!("{where}: {msg}"),
        }
    }
}

impl<'a> From<Error<'a>> for String {
    fn from(value: Error<'a>) -> Self {
        (&value).into()
    }
}

impl<'a> From<&Error<'a>> for PyErr {
    fn from(value: &Error<'a>) -> Self {
        let msg: String = value.into();
        let kind = value.kind
            .unwrap_or(ErrorKind::Exception);
        match kind {
            ErrorKind::Exception => PyErr::new::<PyException, _>(msg),
            ErrorKind::FileNotFoundError => PyErr::new::<PyFileNotFoundError, _>(msg),
            ErrorKind::Geant4Exception => PyErr::new::<Geant4Exception, _>(msg),
            ErrorKind::IndexError => PyErr::new::<PyIndexError, _>(msg),
            ErrorKind::IOError => PyErr::new::<PyIOError, _>(msg),
            ErrorKind::KeyboardInterrupt => PyErr::new::<PyKeyboardInterrupt, _>(msg),
            ErrorKind::KeyError => PyErr::new::<PyKeyError, _>(msg),
            ErrorKind::MemoryError => PyErr::new::<PyMemoryError, _>(msg),
            ErrorKind::NotImplementedError => PyErr::new::<PyNotImplementedError, _>(msg),
            ErrorKind::TypeError => PyErr::new::<PyTypeError, _>(msg),
            ErrorKind::ValueError => PyErr::new::<PyValueError, _>(msg),
        }
    }
}

impl<'a> From<Error<'a>> for PyErr {
    fn from(value: Error<'a>) -> Self {
        (&value).into()
    }
}


// ===============================================================================================
//
// C++ interface.
//
// ===============================================================================================

create_exception!(calzone, Geant4Exception, PyException, "A Geant4 exception.");

impl ffi::Error {
    const KEYBOARD_INTERUPT: &'static str = "Ctrl+C catched";
    const MEMORY_ERROR: &'static str = "could not allocate memory";

    pub fn to_result(&self) -> PyResult<()> {
        match self.tp {
            ffi::ErrorType::None => Ok(()),
            ffi::ErrorType::FileNotFoundError => {
                Err(PyFileNotFoundError::new_err(self.message.clone()))
            },
            ffi::ErrorType::Geant4Exception => {
                Err(Geant4Exception::new_err(self.message.clone()))
            },
            ffi::ErrorType::IndexError => {
                Err(PyIndexError::new_err(self.message.clone()))
            },
            ffi::ErrorType::IOError => {
                Err(PyIOError::new_err(self.message.clone()))
            },
            ffi::ErrorType::KeyboardInterrupt => {
                Err(PyKeyboardInterrupt::new_err(Self::KEYBOARD_INTERUPT))
            },
            ffi::ErrorType::MemoryError => {
                Err(PyMemoryError::new_err(Self::MEMORY_ERROR))
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
            ffi::ErrorType::IndexError => Some(self.message.as_str()),
            ffi::ErrorType::IOError => Some(self.message.as_str()),
            ffi::ErrorType::KeyboardInterrupt => Some(Self::KEYBOARD_INTERUPT),
            ffi::ErrorType::MemoryError => Some(Self::MEMORY_ERROR),
            ffi::ErrorType::ValueError => Some(self.message.as_str()),
            _ => unreachable!(),
        }
    }
}

pub fn ctrlc_catched() -> bool {
    if unsafe { PyErr_CheckSignals() } == -1 { true } else {false}
}

// ===============================================================================================
//
// Variants explainers.
//
// ===============================================================================================

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
