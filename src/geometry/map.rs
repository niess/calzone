use crate::utils::float::f64x3;
use crate::utils::numpy::{PyArray, PyUntypedArray};
use pyo3::prelude::*;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::types::PyDict;
use std::ffi::OsStr;
use std::path::Path;


// ===============================================================================================
//
// Map interface.
//
// ===============================================================================================

#[pyclass(module = "calzone")]
pub struct Map {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    crs: Option<usize>,
    #[pyo3(get)]
    nx: usize,
    #[pyo3(get)]
    x0: f64,
    #[pyo3(get)]
    x1: f64,
    #[pyo3(get)]
    ny: usize,
    #[pyo3(get)]
    y0: f64,
    #[pyo3(get)]
    y1: f64,
    #[pyo3(get)]
    z: PyObject,
}

#[pymethods]
impl Map {
    #[new]
    fn new(py: Python, filename: &str) -> PyResult<Self> {
        let path = Path::new(filename);
        match path.extension().and_then(OsStr::to_str) {
            Some("png") | Some("PNG") => Self::from_png(py, filename),
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

    fn dump(&self, py: Python, filename: String) -> PyResult<()> {
        let path = Path::new(&filename);
        match path.extension().and_then(OsStr::to_str) {
            Some("png") | Some("PNG") => self.to_png(py, &filename),
            _ => unimplemented!(),
        }
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

        let [[x0, y0], [x1, y1]] = geotiff.getattr("tif_bBox")
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
            crs: Some(crs),
            nx,
            x0,
            x1,
            ny,
            y0,
            y1,
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
        let (nx, ny) = (self.nx, self.ny);
        let kx = (self.x1 - self.x0) / ((nx - 1) as f64);
        let ky = (self.y1 - self.y0) / ((ny - 1) as f64);

        let get_x = |index: usize| -> f32 {
            let x = if index == 0 {
                self.x0
            } else if index == nx - 1 {
                self.x1
            } else {
                self.x0 + kx * (index as f64)
            };
            x as f32
        }
        ;
        let get_y = |index: usize| -> f32 {
            let y = if index == 0 {
                self.y0
            } else if index == nx - 1 {
                self.y1
            } else {
                self.y0 + ky * (index as f64)
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
        let x0 = self.x0 as f32;
        let x1 = self.x1 as f32;
        let y0 = self.y0 as f32;
        let y1 = self.y1 as f32;
        push_vertex(x0, y0, zmin);
        push_vertex(x1, y0, zmin);
        push_vertex(x1, y1, zmin);
        push_vertex(x1, y1, zmin);
        push_vertex(x0, y1, zmin);
        push_vertex(x0, y0, zmin);

        Ok(facets)
    }
}


// ===============================================================================================
//
// Png serialisation.
//
// ===============================================================================================

impl Map {
    fn from_png<'py>(py: Python, path: &str) -> PyResult<Self> {
        // Open png file and parse metadata.
        let image = py.import_bound("PIL")
            .and_then(|m| m.getattr("Image"))
            .and_then(|m| m.getattr("open"))
            .and_then(|f| f.call1((path,)))?;

        let loads = py.import_bound("json")
            .and_then(|m| m.getattr("loads"))?;
        let meta = image.getattr("info")
            .and_then(|info| info.get_item("Comment"))
            .and_then(|meta| loads.call1((meta,)))
            .and_then(|meta| meta.get_item("topography"))?;

        let x0: f64 = meta.get_item("x0")
            .and_then(|v| v.extract())?;
        let x1: f64 = meta.get_item("x1")
            .and_then(|v| v.extract())?;
        let y0: f64 = meta.get_item("y0")
            .and_then(|v| v.extract())?;
        let y1: f64 = meta.get_item("y1")
            .and_then(|v| v.extract())?;
        let zmin: f64 = meta.get_item("z0")
            .and_then(|v| v.extract())?;
        let zmax: f64 = meta.get_item("z1")
            .and_then(|v| v.extract())?;

        let crs: Option<usize> = meta.get_item("projection")
            .and_then(|projection| {
                let projection: String = projection.extract()?;
                let crs = match projection.as_str() {
                    "Lambert 93" => Some(2154),
                    "Lambert I" => Some(27571),
                    "Lambert II" => Some(27572),
                    "Lambert III" => Some(27573),
                    "Lambert IV" => Some(27574),
                    _ => None,
                };
                Ok(crs)
            })
            .or_else(|_| {
                meta.get_item("crs")
                    .and_then(|crs| {
                        let crs: usize = crs.extract()?;
                        Ok(Some(crs))
                    })
            })
            .unwrap_or(None);

        // Export pixel values.
        let size: [usize; 2] = image.getattr("size")
            .and_then(|size| size.extract())?;
        let [nx, ny] = size;

        let to_array = py.import_bound("numpy")
            .and_then(|module| module.getattr("array"))?;
        let pixels: &PyArray<u16> = image.call_method0("getdata")
            .and_then(|data| to_array.call1((data, "u2")))
            .and_then(|array| array.extract())?;
        let px = unsafe { pixels.slice()? };
        let array = PyArray::<f32>::empty(py, &[ny, nx])?;
        let z = unsafe { array.slice_mut()? };

        for (i, pi) in px.iter().enumerate() {
            z[i] = if *pi == 0 {
                zmin as f32
            } else if *pi == u16::MAX {
                zmax as f32
            } else {
                let zi = zmin + ((*pi as f64) * (zmax - zmin) / (u16::MAX as f64));
                zi.max(zmin).min(zmax) as f32
            };
        }

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
            x0,
            x1,
            ny,
            y0,
            y1,
            z: z.into(),
        };
        Ok(map)
    }

    fn to_png<'py>(&self, py: Python, path: &str) -> PyResult<()> {
        // Transform z-data.
        let z: &PyArray<f32> = self.z.extract(py)?;
        let array = PyArray::<u16>::empty(py, &z.shape())?;
        let z = unsafe { z.slice()? };
        let pixels = unsafe { array.slice_mut()? };

        let mut zmin = f32::MAX;
        let mut zmax = -f32::MAX;
        for zi in z {
            zmin = zmin.min(*zi);
            zmax = zmax.max(*zi);
        }
        for (i, zi) in z.iter().enumerate() {
            let r = (zi - zmin) / (zmax - zmin);
            pixels[i] = if r <= 0.0 {
                0
            } else if r >= 1.0 {
                u16::MAX
            } else {
                (r * (u16::MAX as f32)) as u16
            };
        }
        let array: &PyUntypedArray = &array;

        // Serialise metadata.
        let crs = self.crs
            .map(|crs| format!("\"crs\": {}, ", crs))
            .unwrap_or("".to_string());
        let meta = format!(
            "{{\"topography\": {{\
                {}\
                \"x0\": {}, \
                \"x1\": {}, \
                \"y0\": {}, \
                \"y1\": {}, \
                \"z0\": {}, \
                \"z1\": {}\
            }}}}",
            crs,
            self.x0,
            self.x1,
            self.y0,
            self.y1,
            zmin,
            zmax,
        );
        let pil = py.import_bound("PIL")?;
        let info = pil
            .getattr("PngImagePlugin")
            .and_then(|m| m.getattr("PngInfo"))
            .and_then(|c| c.call0())?;
        info.call_method1("add_text", ("Comment", meta))?;

        // Save as png file.
        let kwargs = PyDict::new_bound(py);
        kwargs.set_item("pnginfo", info)?;
        let image = pil
            .getattr("Image")
            .and_then(|m| m.getattr("fromarray"))
            .and_then(|f| f.call1((array,)))?;
        image.call_method("save", (path,), Some(&kwargs))?;

        Ok(())
    }
}
