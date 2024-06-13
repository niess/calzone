use pyo3::prelude::*;
use std::mem::transmute;


// ===============================================================================================
//
// 3-vector type (for conversions, mostly).
//
// ===============================================================================================

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Default, FromPyObject, PartialEq)]
#[repr(transparent)]
pub struct f64x3 ([f64; 3]);

impl f64x3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self ([x, y, z])
    }

    pub const fn splat(v: f64) -> Self {
        Self([v, v, v])
    }

    pub const fn zero() -> Self {
        Self([0.0, 0.0, 0.0])
    }

    #[inline]
    pub fn x(&self) -> f64 {
        self.0[0]
    }

    #[inline]
    pub fn y(&self) -> f64 {
        self.0[1]
    }

    #[inline]
    pub fn z(&self) -> f64 {
        self.0[2]
    }
}


impl AsRef<[f64]> for f64x3 {
    fn as_ref(&self) -> &[f64] {
        &self.0
    }
}

impl From<f64x3> for [f64; 3] {
    fn from(value: f64x3) -> Self {
        value.0
    }
}

impl From<&[f64; 3]> for f64x3 {
    fn from(value: &[f64; 3]) -> Self {
        Self (value.clone())
    }
}


// ===============================================================================================
//
// 3x3-matrix type (for conversions, mostly).
//
// ===============================================================================================
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Default, FromPyObject, PartialEq)]
#[repr(transparent)]
pub struct f64x3x3 ([[f64; 3]; 3]);

impl f64x3x3 {
    pub const fn new() -> Self {
        Self ([
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ])
    }

    pub const fn eye() -> Self {
        Self ([
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ])
    }

    pub fn as_flat(&self) -> &[f64] {
        let slice: &[f64; 9] = unsafe { transmute(&self.0) };
        slice
    }
}

impl AsRef<[[f64; 3]]> for f64x3x3 {
    fn as_ref(&self) -> &[[f64; 3]] {
        &self.0
    }
}

impl From<f64x3x3> for [[f64; 3]; 3] {
    fn from(value: f64x3x3) -> Self {
        value.0
    }
}
