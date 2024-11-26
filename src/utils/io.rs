use crate::geometry::materials::gate::load_gate_db;
use obj::{load_obj as parse_obj, Obj};
use pyo3::prelude::*;
use pyo3::exceptions::{PyFileNotFoundError, PyNotImplementedError};
use pyo3::sync::GILOnceCell;
use pyo3::types::{PyDict, PyString};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};


// ===============================================================================================
//
// Dict loaders.
//
// ===============================================================================================

trait ConfigFormat {
    const LOADER: &'static str = "loads";

    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>>;

    fn load_dict<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => {
                    let path = format!("No such file or directory '{}'", path.display());
                    PyFileNotFoundError::new_err(path)
                },
                _ => err.into(),
            })?;
        let module = Self::import_module(py)?;
        let loads = module.getattr(Self::LOADER)?;
        let content = loads.call1((content,))?;
        let dict: Bound<PyDict> = content.extract()?;
        Ok(dict)
    }
}

struct Json;

impl ConfigFormat for Json {
    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        py.import_bound("json")
    }
}

struct Toml;

impl ConfigFormat for Toml {
    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        py.import_bound("tomllib")
            .or_else(|_| py.import_bound("tomli"))
    }
}

struct Yaml;

impl ConfigFormat for Yaml {
    const LOADER: &'static str = "safe_load";

    fn import_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
        py.import_bound("yaml")
    }
}

// ===============================================================================================
//
// Generic dict argument.
//
// ===============================================================================================

#[derive(FromPyObject)]
pub enum DictLike<'py> {
    #[pyo3(transparent, annotation = "dict")]
    Dict(Bound<'py, PyDict>),
    #[pyo3(transparent, annotation = "str")]
    String(PathString<'py>),
}

impl<'py> DictLike<'py> {
    pub fn resolve<'a>(
        &'a self,
        file: Option<&Path>
    ) -> PyResult<(Cow<'a, Bound<'py, PyDict>>, Option<PathBuf>)> {
        let result = match &self {
            Self::Dict(dict) => (Cow::Borrowed(dict), None),
            Self::String(path) => {
                let py = path.0.py();
                let path = path.0.to_cow()?;
                let path = Path::new(path.deref());
                let path = match file {
                    None => Cow::Borrowed(path),
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
                let dict = match path.extension().and_then(OsStr::to_str) {
                    Some("db") => load_gate_db(py, &path),
                    Some("json") => Json::load_dict(py, &path),
                    Some("toml") => Toml::load_dict(py, &path),
                    Some("yml") | Some("yaml") => Yaml::load_dict(py, &path),
                    _ => Err(PyNotImplementedError::new_err("")),
                }?;
                (Cow::Owned(dict), Some(path.to_path_buf()))
            },
        };
        Ok(result)
    }

    pub fn py(&self) -> Python<'py> {
        match self {
            Self::Dict(dict) => dict.py(),
            Self::String(path) => path.0.py(),
        }
    }

    pub fn from_str(py: Python<'py>, value: &str) -> Self {
        let value = PyString::new_bound(py, value);
        Self::String(PathString(value))
    }
}


// ===============================================================================================
//
// Pathlib.Path wrapper.
//
// ===============================================================================================

pub struct PathString<'py> (pub Bound<'py, PyString>);

impl<'py> FromPyObject<'py> for PathString<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        static TYPE: GILOnceCell<PyObject> = GILOnceCell::new();
        let py = ob.py();
        let tp = TYPE.get_or_try_init(py, || py.import_bound("pathlib")
            .and_then(|m| m.getattr("Path"))
            .map(|m| m.unbind())
        )?.bind(py);
        if ob.is_instance(tp)? {
            let path = ob.str()?;
            Ok(Self(path))
        } else {
            let path: Bound<PyString> = ob.extract()?;
            Ok(Self(path))
        }
    }
}

impl<'py> ToString for PathString<'py> {
    fn to_string(&self) -> String {
        self.0.to_string_lossy().to_string()
    }
}


// ===============================================================================================
//
// Generic mesh loader.
//
// ===============================================================================================

pub fn load_mesh(path: &Path) -> Result<Vec<f32>, String> {
    match path.extension().and_then(OsStr::to_str) {
        Some("obj") => load_obj(path),
        Some("stl") => load_stl(path),
        Some(other) => Err(format!("{}: bad '{}' format", path.display(), other)),
        None => Err(format!("{}: missing format", path.display())),
    }
}


// ===============================================================================================
//
// Stl loaders & writers.
//
// ===============================================================================================

pub fn dump_stl(facets: &[f32], path: &Path) -> PyResult<()> {
    let file = File::create(path)?;
    let mut buf = BufWriter::new(file);
    let header = [0_u8; 80];
    buf.write(&header)?;
    let size = facets.len() / 9;
    buf.write(&(size as u32).to_le_bytes())?;
    let normal = [0.0_f32; 3];
    let control: u16 = 0;
    for i in 0..size {
        for j in 0..3 {
            buf.write(&normal[j].to_le_bytes())?;
        }
        for j in 0..9 {
            buf.write(&facets[9 * i + j].to_le_bytes())?;
        }
        buf.write(&control.to_le_bytes())?;
    }
    Ok(())
}

fn load_stl(path: &Path) -> Result<Vec<f32>, String> {
    let bad_format = || format!("{}: bad STL format)", path.display());

    let bytes = std::fs::read(path)
        .map_err(|_| format!("could not read '{}'", path.display()))?;
    let data = bytes.get(80..84)
        .ok_or_else(bad_format)?;
    let facets: usize = u32::from_le_bytes(data.try_into().unwrap()).try_into().unwrap();
    let mut values = Vec::<f32>::with_capacity(9 * facets);
    for i in 0..facets {
        let start: usize = (84 + 50 * i).try_into().unwrap();
        let data = bytes.get(start..(start + 50))
            .ok_or_else(bad_format)?;
        for j in 0..3 {
            let start = 12 * (j + 1);
            for k in 0..3 {
                let start = start + 4 * k;
                let data = &data[start..(start + 4)];
                let v = f32::from_le_bytes(data.try_into().unwrap());
                values.push(v);
            }
        }
    }
    Ok(values)
}


// ===============================================================================================
//
// Obj loader.
//
// ===============================================================================================

fn load_obj(path: &Path) -> Result<Vec<f32>, String> {
    let input = File::open(path)
        .and_then(|file| Ok(BufReader::new(file)))
        .map_err(|_| format!("could not read '{}'", path.display()))?;
    let model: Obj = parse_obj(input)
        .map_err(|msg| format!("{}: {}", msg, path.display()))?;

    let mut data: Vec<f32> = Vec::with_capacity(3 * model.indices.len());
    for index in model.indices {
        let r = &model.vertices[index as usize].position;
        for j in 0..3 {
            data.push(r[j]);
        }
    }
    Ok(data)
}
