use crate::utils::numpy::{Dtype, PyArray, PyArrayFlags};
use pyo3::prelude::*;
use pyo3::pyclass::{boolean_struct::False, PyClass};
use std::pin::Pin;


// ===============================================================================================
//
// Generic export of Vec<T> data as a PyArray<T> (avoiding clone/copy).
//
// Note: The Vec<T> data are pinned to a PyClass which owns the memory wraped by the NumPy array.
// However, since PyClass does not support generics, we first define a generic Export<T> struct,
// Then, the PyClass derives AsMut<Export<T>> and AsRef<Export<T>> in order to be exportable.
//
// ===============================================================================================

#[repr(transparent)]
pub struct Export<T: Sized> (Pin<Box<[T]>>);

impl<T> Export<T>
where
    T: Copy + Dtype + Sized,
{
    pub fn export<'py, W>(py: Python<'py>, values: Vec<T>) -> PyResult<PyObject>
    where
        W: AsMut<Self> + AsRef<Self> + From<Self> + Into<PyClassInitializer<W>> +
           PyClass<Frozen = False>,
    {
        let ob: W = (Self (Box::pin([]))).into();
        let ob: Bound<'py, W> = Bound::new(py, ob)?;
        ob.borrow_mut().as_mut().0 = Box::into_pin(values.into_boxed_slice());
        let binding = ob.borrow();
        let array = PyArray::<T>::from_data(
            py,
            &binding.as_ref().0,
            ob.as_any(),
            PyArrayFlags::ReadWrite,
            None
        )?;
        Ok(array.into_any().unbind())
    }
}
