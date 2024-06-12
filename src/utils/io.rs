use crate::materials::gate::load_gate_db;
use pyo3::prelude::*;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::types::{PyDict, PyString};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use super::numpy::PyArray;


// ===============================================================================================
//
// Dict config loader.
//
// ===============================================================================================

pub fn load_toml<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyDict>> {
    let content = std::fs::read_to_string(path)?;
    let toml = py.import_bound("tomllib")
        .or_else(|_| py.import_bound("tomli"))?;
    let loads = toml.getattr("loads")?;
    let content = loads.call1((content,))?;
    let dict: Bound<PyDict> = content.extract()?;
    Ok(dict)
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
    String(Bound<'py, PyString>),
}

impl<'py> DictLike<'py> {
    pub fn resolve<'a>(&'a self) -> PyResult<(Cow<'a, Bound<'py, PyDict>>, Option<PathBuf>)> {
        let result = match &self {
            Self::Dict(dict) => (Cow::Borrowed(dict), None),
            Self::String(path) => {
                let py = path.py();
                let path = path.to_cow()?;
                let path = Path::new(path.deref());
                let dict = match path.extension().and_then(OsStr::to_str) {
                    Some("db") => load_gate_db(py, &path),
                    Some("toml") => load_toml(py, &path),
                    _ => Err(PyNotImplementedError::new_err("")),
                }?;
                (Cow::Owned(dict), Some(path.to_path_buf()))
            },
        };
        Ok(result)
    }
}


// ===============================================================================================
//
// Data loaders.
//
// ===============================================================================================

pub fn load_geotiff<'py>(py: Python, path: &Path) -> Result<Vec<f32>, String> {
    // Open geotiff file and get metadata.
    let geotiff = py.import_bound("geotiff")
        .and_then(|module| module.getattr("GeoTiff"))
        .and_then(|constr| constr.call1((path,)))
        .map_err(|err| format!("{}", err))?;

    let crs = geotiff.getattr("crs_code")
        .and_then(|crs| {
            let crs: PyResult<usize> = crs.extract();
            crs
        })
        .map_err(|err| format!("{}", err))?;

    let [[xmin, ymax], [xmax, ymin]] = geotiff.getattr("tif_bBox")
        .and_then(|bbox| {
            let bbox: PyResult<[[f64; 2]; 2]> = bbox.extract();
            bbox
        })
        .map_err(|err| format!("{}", err))?;

    // Extract data to a numpy array.
    let to_array = py.import_bound("numpy")
        .and_then(|module| module.getattr("array"))
        .map_err(|err| format!("{}", err))?;

    let array = geotiff.getattr("read")
        .and_then(|readfn| readfn.call0())
        .and_then(|arrayz| to_array.call1((arrayz,)))
        .and_then(|arrayf| {
            let arrayf: PyResult<&PyArray<f32>> = arrayf.extract();
            arrayf
        })
        .map_err(|err| format!("{}", err))?;

    let shape: [usize; 2] = array.shape()
        .try_into()
        .map_err(|shape: Vec<usize>| format!(
            "bad shape (expected a size 2 array, found a size {} array)",
            shape.len(),
        ))?;
    let [nx, ny] = shape;

    let z = unsafe { array.slice() }
        .map_err(|err| format!("{}", err))?;

    // Helpers for accessing data.
    let kx = (xmax - xmin) / ((nx - 1) as f64);
    let ky = (ymax - ymin) / ((ny - 1) as f64);

    let get_x = |index: usize| -> f32 {
        let x = if index == 0 {
            xmin
        } else if index == nx - 1 {
            xmax
        } else {
            xmin + kx * (index as f64)
        };
        x as f32
    }
    ;
    let get_y = |index: usize| -> f32 {
        let y = if index == 0 {
            ymax
        } else if index == nx - 1 {
            ymin
        } else {
            ymax - ky * (index as f64)
        };
        y as f32
    };

    let get_z = |i: usize, j: usize| -> f32 {
        z[i * nx + j]
    };

    let size = nx * ny + nx + ny - 2;
    let mut facets = Vec::<f32>::with_capacity(size);

    let mut push_vertex = |x, y, z| {
        facets.push(x);
        facets.push(y);
        facets.push(z);
    };

    // Tessellate the topography surface.
    let mut zmin = f32::MAX;
    let mut y0 = get_x(0);
    for i in 0..(ny - 1) {
        let y1 = get_y(i + 1);
        let mut x0 = get_x(0);
        let mut z00 = get_z(i, 0);
        zmin = zmin.min(z00);
        let mut z10 = get_z(i + 1, 0);
        zmin = zmin.min(z10);
        for j in 0..(nx - 1) {
            let x1 = get_x(j + 1);
            let z01 = get_z(i, j + 1);
            zmin = zmin.min(z01);
            let z11 = get_z(i + 1, j + 1);
            zmin = zmin.min(z11);

            push_vertex(x0, y0, z00);
            push_vertex(x1, y0, z01);
            push_vertex(x1, y1, z11);
            push_vertex(x1, y1, z11);
            push_vertex(x0, y1, z10);
            push_vertex(x0, y0, z00);

            x0 = x1;
            z00 = z01;
            z10 = z11;
        }
        y0 = y1;
    }

    // Tessellate topography sides.
    for i in [0, ny - 1] {
        let mut x0 = get_x(0);
        let mut z0 = get_z(i, 0);
        let y = get_y(i);
        for j in 0..(nx - 1) {
            let x1 = get_x(j + 1);
            let z1 = get_z(i, j + 1);

            push_vertex(x0, y, z0);
            push_vertex(x1, y, z1);
            push_vertex(x1, y, zmin);
            push_vertex(x1, y, zmin);
            push_vertex(x0, y, zmin);
            push_vertex(x0, y, z0);

            x0 = x1;
            z0 = z1;
        }
    }

    for j in [0, nx - 1] {
        let mut y0 = get_y(0);
        let mut z0 = get_z(0, j);
        let x = get_x(j);
        for i in 0..(ny - 1) {
            let y1 = get_y(i + 1);
            let z1 = get_z(i + 1, j);

            push_vertex(x, y0, z0);
            push_vertex(x, y1, z1);
            push_vertex(x, y1, zmin);
            push_vertex(x, y1, zmin);
            push_vertex(x, y0, zmin);
            push_vertex(x, y0, z0);

            y0 = y1;
            z0 = z1;
        }
    }

    // Tessellate bottom.
    let xmin = xmin as f32;
    let xmax = xmax as f32;
    let ymin = ymin as f32;
    let ymax = ymax as f32;
    push_vertex(xmin, ymin, zmin);
    push_vertex(xmax, ymin, zmin);
    push_vertex(xmax, ymax, zmin);
    push_vertex(xmax, ymax, zmin);
    push_vertex(xmin, ymax, zmin);
    push_vertex(xmin, ymin, zmin);

    Ok(facets)
}

pub fn load_stl<'py>(path: &Path) -> Result<Vec<f32>, String> {
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
