use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::mem::transmute;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use super::numpy::{PyArray, PyArrayMethods};


// ===============================================================================================
//
// 3-vector type (for conversions, mostly).
//
// ===============================================================================================

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Default, FromPyObject, PartialEq, Deserialize, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct f64x3 ([f64; 3]);

impl f64x3 {
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y() * rhs.z() - self.z() * rhs.y(),
            self.z() * rhs.x() - self.x() * rhs.z(),
            self.x() * rhs.y() - self.y() * rhs.x(),
        )
    }

    pub fn dot(self, rhs: Self) -> f64 {
        self.x() * rhs.x() + self.y() * rhs.y() + self.z() * rhs.z()
    }

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self ([x, y, z])
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn norm2(self) -> f64 {
        self.x() * self.x() + self.y() * self.y() + self.z() * self.z()
    }

    pub fn normalise(&mut self) {
        *self /= self.norm();
    }

    pub fn rotate(&mut self, cos_theta: f64, phi: f64) {
        // Compute (and check) the sine.
        let sin_theta = {
            let stsq = 1.0 - cos_theta * cos_theta;
            if stsq < 0.0 { return }
            stsq.sqrt()
        };

        // Get norm and unit vector.
        let norm = self.norm();
        let direction = *self / norm;

        // Generate co-vectors for the local basis.
        let e: Self = {
            let a0 = direction.x().abs();
            let a1 = direction.y().abs();
            let a2 = direction.z().abs();

            if a0 < a1 {
                if a0 < a2 {
                    Self::new(1.0, 0.0, 0.0)
                } else {
                    Self::new(0.0, 0.0, 1.0)
                }
            } else {
                if a1 < a2 {
                    Self::new(0.0, 1.0, 0.0)
                } else {
                    Self::new(0.0, 0.0, 1.0)
                }
            }
        };

        let mut u0 = direction.cross(e);
        u0.normalise();
        let u1 = u0.cross(direction);

        // Apply the rotation.
        *self = (norm * cos_theta) * direction +
                (norm * sin_theta) * (phi.cos() * u0 + phi.sin() * u1);
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

macro_rules! impl_binary_operator {
    ($trait:ident, $func:ident, $op:tt) => {
        impl $trait for f64x3 {
            type Output = Self;
            fn $func (self, rhs: Self) -> Self {
                Self::new(self.x() $op rhs.x(), self.y() $op rhs.y(), self.z() $op rhs.z())
            }
        }
        impl $trait<f64x3> for f64 {
            type Output = f64x3;
            fn $func (self, rhs: f64x3) -> f64x3 {
                f64x3::new(self $op rhs.x(), self $op rhs.y(), self $op rhs.z())
            }
        }
        impl $trait<f64> for f64x3 {
            type Output = Self;
            fn $func (self, rhs: f64) -> Self {
                Self::new(self.x() $op rhs, self.y() $op rhs, self.z() $op rhs)
            }
        }
    }
}

impl_binary_operator!(Add, add, +);
impl_binary_operator!(Mul, mul, *);
impl_binary_operator!(Sub, sub, -);

impl Div<f64> for f64x3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        let tmp = 1.0 / rhs;
        Self::new(self.x() * tmp, self.y() * tmp, self.z() * tmp)
    }
}

macro_rules! impl_assign_operator {
    ($trait:ident, $func:ident, $op:tt) => {
        impl $trait for f64x3 {
            fn $func (&mut self, rhs: Self) {
                *self = Self::new(
                    self.x() $op rhs.x(), self.y() $op rhs.y(), self.z() $op rhs.z()
                );
            }
        }
        impl $trait<f64> for f64x3 {
            fn $func (&mut self, rhs: f64) {
                *self = Self::new(self.x() $op rhs, self.y() $op rhs, self.z() $op rhs);
            }
        }
    }
}

impl_assign_operator!(AddAssign, add_assign, +);
impl_assign_operator!(MulAssign, mul_assign, *);
impl_assign_operator!(SubAssign, sub_assign, -);

impl DivAssign<f64> for f64x3 {
    fn div_assign(&mut self, rhs: f64) {
        let tmp = 1.0 / rhs;
        *self = Self::new(self.x() * tmp, self.y() * tmp, self.z() * tmp);
    }
}

impl Neg for f64x3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x(), -self.y(), -self.z())
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

impl IntoPy<PyObject> for f64x3 {
    fn into_py(self, py: Python) -> PyObject {
        let result = PyArray::<f64>::empty(py, &[3]).unwrap();
        result.set(0, self.0[0]).unwrap();
        result.set(1, self.0[1]).unwrap();
        result.set(2, self.0[2]).unwrap();
        result.readonly();
        result
            .as_any()
            .into_py(py)
    }
}

// ===============================================================================================
//
// 3x3-matrix type (for conversions, mostly).
//
// ===============================================================================================
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Default, FromPyObject, PartialEq, Deserialize, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
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

impl From<[[f64; 3]; 3]> for f64x3x3 {
    fn from(value: [[f64; 3]; 3]) -> Self {
        Self(value)
    }
}
