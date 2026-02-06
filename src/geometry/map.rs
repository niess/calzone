use crate::utils::error::Error;
use crate::utils::error::ErrorKind::{IOError, NotImplementedError, TypeError, ValueError};
use crate::utils::extract::{Extractor, Property, Tag};
use crate::utils::float::f64x3;
use crate::utils::io::{dump_stl, PathString};
use crate::utils::numpy::{PyArray, PyArrayMethods, PyUntypedArray};
use geotiff::{GeoTiff, RasterType};
use geo_types::geometry::Coord;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;


// ===============================================================================================
//
// Map interface.
//
// ===============================================================================================

/// A structured topography map spanning a x-y grid.
#[pyclass(module = "calzone")]
pub struct Map {
    /// Coordinates Reference System (CRS).
    #[pyo3(get)]
    crs: Option<usize>,
    /// Number of nodes along the x-axis (i.e. columns).
    #[pyo3(get)]
    nx: usize,
    /// Left x-coordinate (index-wise).
    #[pyo3(get)]
    x0: f64,
    /// Right x-coordinate (index-wise).
    #[pyo3(get)]
    x1: f64,
    /// Number of nodes along the y-axis (i.e. rows).
    #[pyo3(get)]
    ny: usize,
    #[pyo3(get)]
    /// Lower y-coordinate (index-wise).
    y0: f64,
    #[pyo3(get)]
    /// Upper y-coordinate (index-wise).
    y1: f64,
    #[pyo3(get)]
    /// Topography elevation values at map nodes.
    z: PyObject,
}

#[pymethods]
impl Map {
    #[new]
    fn new(
        map: MapLike,
    ) -> PyResult<Self> {
        let py = map.py();
        match map {
            MapLike::String(filename) => {
                let filename = filename.to_string();
                let path = Path::new(&filename);
                Self::from_file(py, &path)
            },
            MapLike::Any(any) => {
                let geotiff = py.import_bound("geotiff")
                    .and_then(|module| module.getattr("GeoTiff"))?;
                if any.is_instance(&geotiff)? {
                    Self::from_geotiff_object(&any)
                } else {
                    let why = format!(
                        "unimplemented conversion from '{}'",
                        any.get_type(),
                    );
                    let err = Error::new(NotImplementedError).what("map type").why(&why);
                    Err(err.into())
                }
            },
        }
    }

    /// Create a new topography map from a 2D-numpy ndarray.
    #[staticmethod]
    fn from_array(
        z: &Bound<PyUntypedArray>,
        xlim: [f64; 2],
        ylim: [f64; 2],
        crs: Option<usize>,
    ) -> PyResult<Self> {
        let py = z.py();
        let z: &PyUntypedArray = z.extract()?;

        let [ny, nx] = {
            let shape = z.shape();
            if shape.len() != 2 {
                let why = format!(
                    "expected 2 dimensions, found {}",
                    shape.len(),
                );
                let err = Error::new(ValueError).what("map shape").why(&why);
                return Err(err.into());
            }
            [shape[0], shape[1]]
        };
        let [x0, x1] = xlim;
        let [y0, y1] = ylim;

        let z = py.import_bound("numpy")
            .and_then(|m| m.getattr("asarray"))
            .and_then(|f| f.call1((z, "f4")))?;

        let map = Self {
            nx,
            x0,
            x1,
            ny,
            y0,
            y1,
            crs,
            z: z.into(),
        };
        Ok(map)
    }

    /// Dump the map to a file.
    #[pyo3(signature = (filename, **kwargs,))]
    fn dump<'py>(
        &self,
        filename: PathString,
        kwargs: Option<&Bound<'py, PyDict>>
    ) -> PyResult<()> {
        let py = filename.0.py();
        let filename = filename.to_string();
        let path = Path::new(&filename);
        match path.extension().and_then(OsStr::to_str) {
            Some("asc") | Some("ASC") => {
                let mut nodata: Option<f32> = None;
                let mut precision: Option<usize> = None;
                if let Some(kwargs) = kwargs {
                    const EXTRACTOR: Extractor<2> = Extractor::new([
                        Property::optional_f64("nodata"),
                        Property::optional_u32("precision"),
                    ]);
                    let tag = Tag::new("dump", "", None);
                    let [nodata_value, prec] = EXTRACTOR.extract_any(&tag, kwargs, None)?;
                    let nodata_value: Option<f64> = nodata_value.into();
                    nodata = nodata_value.map(|value| value as f32);
                    let prec: Option<u32> = prec.into();
                    precision = prec.map(|value| value as usize);
                }
                self.to_ascii(py, &filename, nodata, precision)
            },
            Some("png") | Some("PNG") => {
                if let Some(kwargs) = kwargs {
                    const EXTRACTOR: Extractor<0> = Extractor::new([]); // No arguments.
                    let tag = Tag::new("dump", "", None);
                    let _unused = EXTRACTOR.extract_any(&tag, kwargs, None)?;
                }
                self.to_png(py, &filename)
            },
            Some("stl") | Some("STL") => {
                let mut origin: Option<f64x3> = None;
                let mut extra_depth: Option<f64> = None;
                let mut regular: Option<bool> = None;
                if let Some(kwargs) = kwargs {
                    const EXTRACTOR: Extractor<3> = Extractor::new([
                        Property::optional_vec("origin"),
                        Property::optional_f64("padding"),
                        Property::optional_bool("regular"),
                    ]);
                    let tag = Tag::new("dump", "", None);
                    let [center, depth, reg] = EXTRACTOR.extract_any(&tag, kwargs, None)?;
                    origin = center.into();
                    extra_depth = depth.into();
                    regular = reg.into();
                }
                let regular = regular.unwrap_or(false);
                let facets = self.build_mesh(py, regular, origin, extra_depth)?;
                dump_stl(&facets, &path)
            },
            Some(other) => {
                let why = format!(
                    "unimplemented conversion to '{}'",
                    other,
                );
                let err = Error::new(NotImplementedError).what("dump format").why(&why);
                Err(err.into())
            },
            None => {
                let err = Error::new(ValueError).what("dump format").why("missing file extension");
                Err(err.into())
            },
        }
    }
}

#[derive(FromPyObject)]
pub enum MapLike<'py> {
    String(PathString<'py>),
    Any(Bound<'py, PyAny>),
}

impl<'py> MapLike<'py> {
    fn py(&self) -> Python<'py> {
        match self {
            Self::String(s) => s.0.py(),
            Self::Any(a) => a.py(),
        }
    }
}

impl Map {
    const DEFAULT_MIN_DEPTH: f64 = 100.0; // in map units.

    pub fn from_file(py: Python, path: &Path) -> PyResult<Self> {
        let filename = path.to_str().unwrap();
        match path.extension().and_then(OsStr::to_str) {
            Some("asc") | Some("ASC") => Self::from_ascii(py, filename),
            Some("png") | Some("PNG") => Self::from_png(py, filename),
            Some("tif") | Some("TIF") => Self::from_geotiff_file(py, filename),
            Some(other) => {
                let why = format!("unimplemented '{}' format", other);
                let err = Error::new(NotImplementedError).what("map").why(&why);
                Err(err.into())
            },
            None => {
                let err = Error::new(ValueError).what("map").why("missing format");
                Err(err.into())
            },
        }
    }

    pub fn build_mesh(
        &self,
        py: Python,
        regular: bool,
        origin: Option<f64x3>,
        extra_depth: Option<f64>,
    ) -> PyResult<Vec<f32>> {
        // Unpack or set the origin.
        let (xc, yc, zc) = origin
            .map(|origin| {
                let xc = origin.x() as f32;
                let yc = origin.y() as f32;
                let zc = origin.z() as f32;
                (xc, yc, zc)
            })
            .unwrap_or_else(|| (0.0, 0.0, 0.0));

        // Set bottom z.
        let z: &PyArray<f32> = self.z.extract(py)?;
        let z = unsafe { z.slice()? };

        let zbot = {
            let mut zmin = f32::MAX;
            for zi in z {
                zmin = zmin.min(*zi);
            }
            let extra_depth = extra_depth.unwrap_or(Self::DEFAULT_MIN_DEPTH);
            zmin - (extra_depth as f32)
        };

        // Helpers for manipulating data.
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
        };

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

        let size = if regular { 36 * (nx * ny - 1) } else { 18 * ((nx + 1) * (ny + 1) - 3) };
        let mut facets = Vec::<f32>::with_capacity(size);

        struct Vertex (f32, f32, f32);

        let vertex = |x: f32, y: f32, z: f32| -> Vertex {
            Vertex (x - xc, y - yc, z - zc)
        };

        #[derive(Clone, Copy)]
        enum Orientation {
            Left,
            Right,
        }

        let mut push = |
            orientation: Orientation,
            v0: Vertex,
            v1: Vertex,
            v2: Vertex,
            v3: Vertex
        | {
            match orientation {
                Orientation::Left => {
                    facets.extend_from_slice(&[
                        v0.0, v0.1, v0.2,
                        v1.0, v1.1, v1.2,
                        v2.0, v2.1, v2.2,
                        v2.0, v2.1, v2.2,
                        v3.0, v3.1, v3.2,
                        v0.0, v0.1, v0.2,
                    ]);
                },
                Orientation::Right => {
                    facets.extend_from_slice(&[
                        v0.0, v0.1, v0.2,
                        v3.0, v3.1, v3.2,
                        v2.0, v2.1, v2.2,
                        v2.0, v2.1, v2.2,
                        v1.0, v1.1, v1.2,
                        v0.0, v0.1, v0.2,
                    ]);
                },
            }
        };

        let (left, right) = {
            let sgn = (self.x1 - self.x0) * (self.y1 - self.y0);
            if sgn > 0.0 {
                (Orientation::Left, Orientation::Right)
            } else if sgn < 0.0 {
                (Orientation::Right, Orientation::Left)
            } else {
                return Ok(facets)
            }
        };

        // Tessellate the topography surface.
        let mut y0 = get_y(0);
        for i in 0..(ny - 1) {
            let y1 = get_y(i + 1);
            let mut x0 = get_x(0);
            let mut z00 = get_z(i, 0);
            let mut z10 = get_z(i + 1, 0);
            for j in 0..(nx - 1) {
                let x1 = get_x(j + 1);
                let z01 = get_z(i, j + 1);
                let z11 = get_z(i + 1, j + 1);
                push(
                    left,
                    vertex(x0, y0, z00),
                    vertex(x1, y0, z01),
                    vertex(x1, y1, z11),
                    vertex(x0, y1, z10),
                );
                x0 = x1;
                z00 = z01;
                z10 = z11;
            }
            y0 = y1;
        }

        // Tessellate the side faces.
        for i in [0, ny - 1] {
            let orientation = match i {
                0 => right,
                _ => left,
            };
            let mut x0 = get_x(0);
            let mut z0 = get_z(i, 0);
            let y = get_y(i);
            for j in 0..(nx - 1) {
                let x1 = get_x(j + 1);
                let z1 = get_z(i, j + 1);
                push(
                    orientation,
                    vertex(x0, y, z0),
                    vertex(x1, y, z1),
                    vertex(x1, y, zbot),
                    vertex(x0, y, zbot),
                );
                x0 = x1;
                z0 = z1;
            }
        }

        for j in [0, nx - 1] {
            let orientation = match j {
                0 => left,
                _ => right,
            };
            let mut y0 = get_y(0);
            let mut z0 = get_z(0, j);
            let x = get_x(j);
            for i in 0..(ny - 1) {
                let y1 = get_y(i + 1);
                let z1 = get_z(i + 1, j);
                push(
                    orientation,
                    vertex(x, y0, z0),
                    vertex(x, y1, z1),
                    vertex(x, y1, zbot),
                    vertex(x, y0, zbot),
                );
                y0 = y1;
                z0 = z1;
            }
        }

        // Tessellate the bottom face.
        if regular {
            // We reproduce the top grid, despite the bottom surface being flat, otherwise Geant4
            // does not recognise the mesh as being closed.
            let mut y0 = get_y(0);
            for i in 0..(ny - 1) {
                let y1 = get_y(i + 1);
                let mut x0 = get_x(0);
                for j in 0..(nx - 1) {
                    let x1 = get_x(j + 1);
                    push(
                        right,
                        vertex(x0, y0, zbot),
                        vertex(x1, y0, zbot),
                        vertex(x1, y1, zbot),
                        vertex(x0, y1, zbot),
                    );
                    x0 = x1;
                }
                y0 = y1;
            }
        } else {
            let x0 = get_x(0);
            let x1 = get_x(nx - 1);
            let y0 = get_y(0);
            let y1 = get_y(ny - 1);
            push(
                right,
                vertex(x0, y0, zbot),
                vertex(x1, y0, zbot),
                vertex(x1, y1, zbot),
                vertex(x0, y1, zbot),
            );
        }

        Ok(facets)
    }
}


// ===============================================================================================
//
// Geotiff serialisation.
//
// ===============================================================================================

impl Map {
    fn from_geotiff_file<'py>(py: Python, path: &str) -> PyResult<Self> {
        let geotiff = GeoTiff::read(File::open(path)?)
            .map_err(|err| Error::new(IOError)
                .what("GeoTIFF file")
                .why(&err.to_string())
                .to_err()
            )?;
        let crs = geotiff.geo_key_directory.projected_type.map(|crs| crs as usize);
        let nx = geotiff.raster_width;
        let ny = geotiff.raster_height;
        let ([x0, x1], [y0, y1]) = {
            let extent = geotiff.model_extent();
            let min = extent.min();
            let max = extent.max();
            let x = [ min.x, max.x ];
            let y = [ min.y, max.y ];
            (x, y)
        };

        let raster_type = match geotiff.geo_key_directory.raster_type
            .unwrap_or(RasterType::Undefined) {
            RasterType::RasterPixelIsArea => RasterType::RasterPixelIsArea,
            RasterType::RasterPixelIsPoint => RasterType::RasterPixelIsPoint,
            _ => if geotiff
                .get_value_at::<f64>(&Coord { x: x0, y: y0 }, 0)
                .or_else(|| geotiff.get_value_at::<f64>(&Coord { x: x1, y: y0 }, 0))
                .or_else(|| geotiff.get_value_at::<f64>(&Coord { x: x0, y: y1 }, 0))
                .or_else(|| geotiff.get_value_at::<f64>(&Coord { x: x1, y: y1 }, 0))
                .is_none() {
                RasterType::RasterPixelIsPoint
            } else {
                RasterType::RasterPixelIsArea
            },
        };
        let (x0, x1, dx, y0, y1, dy) = match raster_type {
            RasterType::RasterPixelIsArea => {
                let dx = (x1 - x0) / (nx as f64);
                let dy = (y1 - y0) / (ny as f64);
                (x0 + 0.5 * dx, x1 - 0.5 * dx, dx, y0 + 0.5 * dy, y1 - 0.5 * dy, dy)
            },
            RasterType::RasterPixelIsPoint => {
                let dx = (x1 - x0) / ((nx - 1) as f64);
                let dy = (y1 - y0) / ((ny - 1) as f64);
                (x0, x1, dx, y0, y1, dy)
            },
            _ => unreachable!(),
        };

        let array = PyArray::<f32>::empty(py, &[ny, nx])?;
        let size = nx * ny;
        if size > 0 {
            let z = unsafe { array.slice_mut()? };
            for iy in 0..ny {
                let y = if iy == ny - 1 { y1 } else { y0 + iy as f64 * dy };
                for ix in 0..nx {
                    let x = if ix == nx - 1 { x1 } else { x0 + ix as f64 * dx };
                    z[iy * nx + ix] = geotiff.get_value_at(&Coord { x, y }, 0)
                        .unwrap_or_else(|| 0.0);
                }
            }
        }
        let z = array.into_any().unbind();

        Ok(Self { crs, nx, ny, x0, x1, y0, y1, z })
    }

    fn from_geotiff_object<'py>(geotiff: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Get metadata.
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
        let py = geotiff.py();
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
                let why = format!(
                    "expected a size 2 array, found a size {} array",
                    shape.len(),
                );
                Error::new(ValueError).what("shape").why(&why).to_err()
            })?;
        let [ny, nx] = shape;

        let map = Self {
            crs: Some(crs),
            nx,
            x0,
            x1,
            ny,
            y0,
            y1,
            z: array.as_any().into(),
        };
        Ok(map)
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
        let image = py.import_bound("PIL.Image")
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

        let map = Self {
            crs,
            nx,
            x0,
            x1,
            ny,
            y0,
            y1,
            z: array.into_any().unbind(),
        };
        Ok(map)
    }

    fn to_png(&self, py: Python, path: &str) -> PyResult<()> {
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
        let info = py.import_bound("PIL.PngImagePlugin")?
            .getattr("PngInfo")
            .and_then(|c| c.call0())?;
        info.call_method1("add_text", ("Comment", meta))?;

        // Save as png file.
        let kwargs = PyDict::new_bound(py);
        kwargs.set_item("pnginfo", info)?;
        let image = py.import_bound("PIL.Image")
            .and_then(|m| m.getattr("fromarray"))
            .and_then(|f| f.call1((array.as_any(),)))?;
        image.call_method("save", (path,), Some(&kwargs))?;

        Ok(())
    }
}


// ===============================================================================================
//
// ASCII Grid serialisation.
//
// ===============================================================================================

impl Map {
    fn from_ascii<'py>(py: Python, path: &str) -> PyResult<Self> {
        #[derive(Clone, Copy)]
        enum MetaType {
            Nx,
            Ny,
            X0,
            Y0,
            Delta,
            NoData,
        }

        enum MetaValue {
            NRows(usize),
            NCols(usize),
            XLLCenter(f64),
            XLLCorner(f64),
            YLLCenter(f64),
            YLLCorner(f64),
            CellSize(f64),
            NoData(Result<f32, String>),
        }

        impl MetaValue {
            fn into_f64(self) -> f64 {
                match self {
                    Self::CellSize(value) => value,
                    _ => unreachable!(),
                }
            }

            fn into_usize(self) -> usize {
                match self {
                    Self::NRows(value) => value,
                    Self::NCols(value) => value,
                    _ => unreachable!(),
                }
            }
        }

        const WHAT: &str = "ASCII Grid file";

        impl MetaType {
            const fn lineno(&self) -> usize {
                match self {
                    Self::Nx => 1,
                    Self::Ny => 2,
                    Self::X0 => 3,
                    Self::Y0 => 4,
                    Self::Delta => 5,
                    Self::NoData => 6,
                }
            }

            fn parse(
                &self,
                lines: &mut impl Iterator<Item=Result<String, std::io::Error>>,
                path: &str,
            ) -> PyResult<MetaValue> {
                let badname = |expected: &str, found: &str| -> PyErr {
                    let why = format!(
                        "{}:{}: expected {}, found {}", path, self.lineno(), expected, found
                    );
                    Error::new(TypeError).what(WHAT).why(&why).to_err()
                };

                let badvalue = |err: String, value: &str| -> PyErr {
                    let why = format!("{}:{}: {}: {}", path, self.lineno(), err, value);
                    Error::new(TypeError).what(WHAT).why(&why).to_err()
                };

                let eol = || -> PyErr {
                    let why = format!("{}:{}: unexpected end-of-line", path, self.lineno());
                    Error::new(TypeError).what(WHAT).why(&why).to_err()
                };

                let Some(line) = lines.next() else { return Err(eol()) };
                let Ok(line) = line else { return Err(eol()) };
                let mut tokens = line.split_ascii_whitespace();
                let Some(name) = tokens.next() else { return Err(eol()) };
                let name = name.to_uppercase();
                let Some(value) = tokens.next() else { return Err(eol()) };
                if let Some(token) = tokens.next() {
                    let why = format!("{}:{}: unexpected token: {}", path, self.lineno(), token);
                    return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
                }
                let value = match self {
                    Self::Nx => {
                        if name.as_str() != "NCOLS" { return Err(badname("ncols", name.as_str())) }
                        match value.parse::<usize>() {
                            Ok(value) => MetaValue::NCols(value),
                            Err(err) => { return Err(badvalue(err.to_string(), value)) },
                        }
                    },
                    Self::Ny => {
                        if name.as_str() != "NROWS" { return Err(badname("nrows", name.as_str())) }
                        match value.parse::<usize>() {
                            Ok(value) => MetaValue::NRows(value),
                            Err(err) => { return Err(badvalue(err.to_string(), value)) },
                        }
                    },
                    Self::X0 => {
                        if name.as_str() == "XLLCENTER" {
                            match value.parse::<f64>() {
                                Ok(value) => MetaValue::XLLCenter(value),
                                Err(err) => { return Err(badvalue(err.to_string(), value)) },
                            }
                        } else if name.as_str() == "XLLCORNER" {
                            match value.parse::<f64>() {
                                Ok(value) => MetaValue::XLLCorner(value),
                                Err(err) => { return Err(badvalue(err.to_string(), value)) },
                            }
                        } else {
                            return Err(badname("xllcenter or xllcorner", name.as_str()))
                        }
                    },
                    Self::Y0 => {
                        if name.as_str() == "YLLCENTER" {
                            match value.parse::<f64>() {
                                Ok(value) => MetaValue::YLLCenter(value),
                                Err(err) => { return Err(badvalue(err.to_string(), value)) },
                            }
                        } else if name.as_str() == "YLLCORNER" {
                            match value.parse::<f64>() {
                                Ok(value) => MetaValue::YLLCorner(value),
                                Err(err) => { return Err(badvalue(err.to_string(), value)) },
                            }
                        } else {
                            return Err(badname("yllcenter or yllcorner", name.as_str()))
                        }
                    },
                    Self::Delta => {
                        if name.as_str() != "CELLSIZE" {
                            return Err(badname("cellsize", name.as_str()))
                        }
                        match value.parse::<f64>() {
                            Ok(value) => MetaValue::CellSize(value),
                            Err(err) => { return Err(badvalue(err.to_string(), value)) },
                        }
                    },
                    Self::NoData => {
                        if name.as_str() == "NODATA_VALUE" {
                            match value.parse::<f32>() {
                                Ok(value) => MetaValue::NoData(Ok(value)),
                                Err(err) => { return Err(badvalue(err.to_string(), value)) },
                            }
                        } else {
                            MetaValue::NoData(Err(line))
                        }
                    },
                };
                Ok(value)
            }
        }

        let file = File::open(path)?;
        let mut lines = BufReader::new(file).lines();

        let nx = MetaType::Nx.parse(&mut lines, path)?.into_usize();
        let ny = MetaType::Ny.parse(&mut lines, path)?.into_usize();
        let x0 = MetaType::X0.parse(&mut lines, path)?;
        let y0 = MetaType::Y0.parse(&mut lines, path)?;
        let dx = MetaType::Delta.parse(&mut lines, path)?.into_f64();
        let dy = dx;
        let x0 = match x0 {
            MetaValue::XLLCorner(x0) => x0 + 0.5 * dx,
            MetaValue::XLLCenter(x0) => x0,
            _ => unreachable!(),
        };
        let y0 = match y0 {
            MetaValue::YLLCorner(y0) => y0 + 0.5 * dy,
            MetaValue::YLLCenter(y0) => y0,
            _ => unreachable!(),
        };
        let nodata = MetaType::NoData.parse(&mut lines, path)?;

        let (mut lineno, mut line, nodata) = match nodata {
            MetaValue::NoData(value) => match value {
                Ok(value) => (MetaType::NoData.lineno() + 1, lines.next(), value),
                Err(line) => (MetaType::NoData.lineno(), Some(Ok(line)), -f32::INFINITY),
            },
            _ => unreachable!()
        };
        let array = PyArray::<f32>::empty(py, &[ny, nx])?;
        let size = nx * ny;
        if size > 0 {
            let z = unsafe { array.slice_mut()? };
            let mut index: usize = 0;
            'outer: loop {
                let Some(content) = line else { break };
                let Ok(content) = content else {
                    let why = format!("{}:{}: unexpected end-of-line", path, lineno);
                    return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
                };
                for token in content.split_ascii_whitespace() {
                    if index >= size { break 'outer }
                    let row = ny - 1 - (index / nx);
                    let col = index % nx;
                    z[row * nx + col] = token.parse::<f32>()
                        .map(|zi| if zi == nodata { f32::NAN } else { zi })
                        .map_err(|err| {
                            let why = format!("{}:{}: {}: {}", path, lineno, err, token);
                            Error::new(TypeError).what(WHAT).why(&why).to_err()
                        })?;
                    index += 1;
                }
                lineno += 1;
                line = lines.next();
            }
            if index < size {
                let why = format!("{}: expected {} z-values, found {}", path, size, index);
                return Err(Error::new(TypeError).what(WHAT).why(&why).to_err())
            }
        }

        let map = Self {
            crs: None,
            nx,
            x0,
            x1: x0 + dx * ((nx - 1) as f64),
            ny,
            y0,
            y1: y0 + dy * ((ny - 1) as f64),
            z: array.into_any().unbind(),
        };
        Ok(map)
    }

    fn to_ascii(
        &self,
        py: Python,
        path: &str,
        nodata: Option<f32>,
        precision: Option<usize>,
    ) -> PyResult<()> {
        let dx = if self.nx > 1 { (self.x1 - self.x0) / ((self.nx - 1) as f64) } else { 0.0 };
        let dy = if self.ny > 1 { (self.y1 - self.y0) / ((self.ny - 1) as f64) } else { 0.0 };
        if (dx.abs() - dy.abs()).abs() > f64::EPSILON {
            return Err(Error::new(TypeError).what("cells").why("not squared").to_err())
        }
        enum Order {
            Decreasing,
            Increasing,
        }
        let xorder = if dx < 0.0 { Order::Decreasing } else { Order::Increasing };
        let yorder = if dy < 0.0 { Order::Decreasing } else { Order::Increasing };

        let file = File::create(path)?;
        let mut stream = BufWriter::new(file);
        write!(stream,
            "\
            ncols {}\n\
            nrows {}\n\
            xllcenter {}\n\
            yllcenter {}\n\
            cellsize {}\n",
            self.nx,
            self.ny,
            if dx > 0.0 { self.x0 } else { self.x1 },
            if dy > 0.0 { self.y0 } else { self.y1 },
            dx.abs(),
        )?;
        if let Some(nodata) = nodata {
            match precision {
                Some(precision) => write!(
                    stream, "nodata_value {:.prec$}\n", nodata, prec=precision
                )?,
                None => write!(stream, "nodata_value {}\n", nodata)?,
            }
        }

        let z: &PyArray<f32> = self.z.extract(py)?;
        let z = unsafe { z.slice()? };
        for i in 0..self.ny {
            let i = match yorder {
                Order::Increasing => self.ny - 1 - i,
                Order::Decreasing => i,
            };
            for j in 0..self.nx {
                let jj = match xorder {
                    Order::Increasing => j,
                    Order::Decreasing => self.nx - 1 - j,
                };
                let mut zij = z[i * self.nx + jj];
                if let Some(nodata) = nodata {
                    if zij.is_nan() { zij = nodata }
                }
                match precision {
                    Some(precision) => write!(stream, "{:.prec$}", zij, prec=precision)?,
                    None => write!(stream, "{}", zij)?,
                }
                let sep = if j < self.nx - 1 { ' ' } else { '\n' };
                write!(stream, "{}", sep)?
            }
        }

        Ok(())
    }
}
