use crate::utils::export::Export;
use derive_more::{AsMut, AsRef, From};
use pyo3::prelude::*;
use super::ffi;


// ===============================================================================================
//
// Tracker interface.
//
// ===============================================================================================

#[derive(Default)]
pub struct Tracker {
    tracks: Vec<ffi::Track>,
    vertices: Vec<ffi::Vertex>,
}

impl Tracker {
    pub fn export(self, py: Python) -> PyResult<(PyObject, PyObject)> {
        let tracks = Export::export::<TracksExport>(py, self.tracks)?;
        let vertices = Export::export::<VerticesExport>(py, self.vertices)?;
        Ok((tracks, vertices))
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_track(&mut self, track: ffi::Track) {
        self.tracks.push(track)
    }

    pub fn push_vertex(&mut self, vertex: ffi::Vertex) {
        self.vertices.push(vertex)
    }
}

#[derive(AsMut, AsRef, From)]
#[pyclass(module="calzone")]
struct TracksExport (Export<ffi::Track>);

#[derive(AsMut, AsRef, From)]
#[pyclass(module="calzone")]
struct VerticesExport (Export<ffi::Vertex>);
