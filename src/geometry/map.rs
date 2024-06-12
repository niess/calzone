use crate::utils::float::f64x3;
use crate::utils::numpy::{PyArray, PyUntypedArray};
use pyo3::prelude::*;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use std::ffi::OsStr;
use std::path::Path;


#[pyclass(module = "calzone")]
pub struct Map {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    crs: usize,
    #[pyo3(get)]
    nx: usize,
    #[pyo3(get)]
    xmin: f64,
    #[pyo3(get)]
    xmax: f64,
    #[pyo3(get)]
    ny: usize,
    #[pyo3(get)]
    ymin: f64,
    #[pyo3(get)]
    ymax: f64,
    #[pyo3(get)]
    z: PyObject,
}

#[pymethods]
impl Map {
    #[new]
    fn new(py: Python, filename: &str) -> PyResult<Self> {
        let path = Path::new(filename);
        match path.extension().and_then(OsStr::to_str) {
            Some("tif") | Some("TIF") => Self::from_geotiff(py, filename),
            Some(other) => {
                let message = format!("bad map (unimplemented '{}' format)", other);
                Err(PyNotImplementedError::new_err(message))
            },
            None => {
                Err(PyValueError::new_err("bad map (missing format)"))
            },
        }
    }

    fn export(&self, path: Option<String>) {
        unimplemented!(); // XXX Tessellation, or serialisation?
    }
}

// Private interface.
impl Map {
    fn from_geotiff<'py>(py: Python, path: &str) -> PyResult<Self> {
        // Open geotiff file and get metadata.
        let geotiff = py.import_bound("geotiff")
            .and_then(|module| module.getattr("GeoTiff"))
            .and_then(|constr| constr.call1((path,)))?;

        let crs = geotiff.getattr("crs_code")
            .and_then(|crs| {
                let crs: PyResult<usize> = crs.extract();
                crs
            })?;

        let [[xmin, ymax], [xmax, ymin]] = geotiff.getattr("tif_bBox")
            .and_then(|bbox| {
                let bbox: PyResult<[[f64; 2]; 2]> = bbox.extract();
                bbox
            })?;

        // Extract data to a numpy array.
        let to_array = py.import_bound("numpy")
            .and_then(|module| module.getattr("asarray"))?;

        let array = geotiff.getattr("read")
            .and_then(|readfn| readfn.call0())
            .and_then(|arrayz| to_array.call1((arrayz, "f4")))
            .and_then(|arrayf| {
                let arrayf: PyResult<&PyArray<f32>> = arrayf.extract();
                arrayf
            })?;

        let shape: [usize; 2] = array.shape()
            .try_into()
            .map_err(|shape: Vec<usize>| {
                let message = format!(
                    "bad shape (expected a size 2 array, found a size {} array)",
                    shape.len(),
                );
                PyValueError::new_err(message)
            })?;
        let [ny, nx] = shape;

        let path = Path::new(path);
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");

        let z: &PyUntypedArray = array.into();
        let map = Self {
            name: name.to_string(),
            crs,
            nx,
            xmin,
            xmax,
            ny,
            ymin,
            ymax,
            z: z.into(),
        };
        Ok(map)
    }

    fn tessellate(
        &self,
        py: Python,
        scale: Option<f64>,
        origin: Option<f64x3>
    ) -> PyResult<Vec<f32>> {
        let z: &PyArray<f32> = self.z.extract(py)?;
        let z = unsafe { z.slice()? };

        // Helpers for accessing data.
        let (xmin, xmax, nx) = (self.xmin, self.xmax, self.nx);
        let (ymin, ymax, ny) = (self.ymin, self.ymax, self.ny);

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

        let size = 18 * (nx * ny + nx + ny - 2);
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
}
