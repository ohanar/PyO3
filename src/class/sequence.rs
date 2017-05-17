// Copyright (c) 2017-present PyO3 Project and Contributors

//! Python Sequence Interface
//! Trait and support implementation for implementing sequence

use std::os::raw::c_int;

use ffi;
use err::PyResult;
use python::{Python, PythonObject, PyDrop};
use objects::PyObject;
use py_class::slots::{LenResultConverter, BoolConverter, UnitCallbackConverter};
use function::PyObjectCallbackConverter;
use conversion::{ToPyObject, FromPyObject};

/// Sequence interface
pub trait PySequenceProtocol: PythonObject {
    fn __len__(&self, py: Python) -> Self::Result
        where Self: PySequenceLenProtocol
    { unimplemented!() }

    fn __getitem__(&self, py: Python, key: isize) -> Self::Result
        where Self: PySequenceGetItemProtocol
    { unimplemented!() }

    fn __setitem__(&self, py: Python, key: isize, value: Option<Self::Value>) -> Self::Result
        where Self: PySequenceSetItemProtocol
    { unimplemented!() }
    
    fn __contains__(&self, py: Python, value: Self::Value) -> Self::Result
        where Self: PySequenceContainsProtocol
    { unimplemented!() }

    fn __concat__(&self, py: Python, other: Self::Other) -> Self::Result
        where Self: PySequenceConcatProtocol
    { unimplemented!() }

    fn __repeat__(&self, py: Python, count: isize) -> Self::Result
        where Self: PySequenceRepeatProtocol
    { unimplemented!() }

    fn __inplace_concat__(&self, py: Python, other: Self::Other) -> Self::Result
        where Self: PySequenceInplaceConcatProtocol
    { unimplemented!() }

    fn __inplace_repeat__(&self, py: Python, count: isize) -> Self::Result
        where Self: PySequenceInplaceRepeatProtocol
    { unimplemented!() }
}

// The following are a bunch of marker traits used to detect
// the existance of a slotted method.

pub trait PySequenceLenProtocol: PySequenceProtocol {
    type Result: Into<PyResult<usize>>;
}

pub trait PySequenceGetItemProtocol: PySequenceProtocol {
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PySequenceSetItemProtocol: PySequenceProtocol {
    type Value: for<'a> FromPyObject<'a>;
    type Result: Into<PyResult<()>>;
}

pub trait PySequenceContainsProtocol: PySequenceProtocol {
    type Value: for<'a> FromPyObject<'a>;
    type Result: Into<PyResult<bool>>;
}

pub trait PySequenceConcatProtocol: PySequenceProtocol {
    type Other: for<'a> FromPyObject<'a>;
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PySequenceRepeatProtocol: PySequenceProtocol {
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PySequenceInplaceConcatProtocol: PySequenceProtocol {
    type Other: for<'a> FromPyObject<'a>;
    type Result: Into<PyResult<Self>>;
}

pub trait PySequenceInplaceRepeatProtocol: PySequenceProtocol {
    type Result: Into<PyResult<Self>>;
}

#[doc(hidden)]
pub trait PySequenceProtocolImpl {
    fn tp_as_sequence() -> Option<ffi::PySequenceMethods>;
}

impl<T> PySequenceProtocolImpl for T {
    #[inline]
    default fn tp_as_sequence() -> Option<ffi::PySequenceMethods> {
        None
    }
}

impl<T> PySequenceProtocolImpl for T where T: PySequenceProtocol {
    #[inline]
    fn tp_as_sequence() -> Option<ffi::PySequenceMethods> {
        Some(ffi::PySequenceMethods {
            sq_length: Self::sq_length(),
            sq_concat: Self::sq_concat(),
            sq_repeat: Self::sq_repeat(),
            sq_item: Self::sq_item(),
            was_sq_slice: 0 as *mut _,
            sq_ass_item: Self::sq_ass_item(),
            was_sq_ass_slice: 0 as *mut _,
            sq_contains: Self::sq_contains(),
            sq_inplace_concat: Self::sq_inplace_concat(),
            sq_inplace_repeat: Self::sq_inplace_repeat(),
        })
    }
}

trait PySequenceLenProtocolImpl {
    fn sq_length() -> Option<ffi::lenfunc>;
}

impl<T> PySequenceLenProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_length() -> Option<ffi::lenfunc> {
        None
    }
}

impl<T> PySequenceLenProtocolImpl for T
    where T: PySequenceLenProtocol
{
    #[inline]
    fn sq_length() -> Option<ffi::lenfunc> {
        py_len_func!(PySequenceLenProtocol, T::__len__, LenResultConverter)
    }
}

trait PySequenceGetItemProtocolImpl {
    fn sq_item() -> Option<ffi::ssizeargfunc>;
}

impl<T> PySequenceGetItemProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_item() -> Option<ffi::ssizeargfunc> {
        None
    }
}

impl<T> PySequenceGetItemProtocolImpl for T
    where T: PySequenceGetItemProtocol
{
    #[inline]
    fn sq_item() -> Option<ffi::ssizeargfunc> {
        py_ssizearg_func!(PySequenceGetItemProtocol, T::__getitem__, PyObjectCallbackConverter)
    }
}

trait PySequenceSetItemProtocolImpl {
    fn sq_ass_item() -> Option<ffi::ssizeobjargproc>;
}

impl<T> PySequenceSetItemProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_ass_item() -> Option<ffi::ssizeobjargproc> {
        None
    }
}

impl<T> PySequenceSetItemProtocolImpl for T
    where T: PySequenceSetItemProtocol
{
    #[inline]
    fn sq_ass_item() -> Option<ffi::ssizeobjargproc> {
         unsafe extern "C" fn wrap<T>(
             slf: *mut ffi::PyObject,
             key: ffi::Py_ssize_t,
             value: *mut ffi::PyObject,
         ) -> c_int
            where T: PySequenceSetItemProtocol
        {
            const LOCATION: &'static str = "foo.__setitem__()";
            ::_detail::handle_callback(LOCATION, UnitCallbackConverter, |py| {
                let slf = PyObject::from_borrowed_ptr(py, slf).unchecked_cast_into::<T>();
                
                let ret = if value.is_null() {
                    slf.__setitem__(py, key as isize, None).into()
                } else {
                    let value = PyObject::from_borrowed_ptr(py, value);
                    let ret = match value.extract(py) {
                        Ok(value) => slf.__setitem__(py, key as isize, Some(value)).into(),
                        Err(e) => Err(e),
                    };
                    PyDrop::release_ref(value, py);
                    ret
                };

                PyDrop::release_ref(slf, py);
                ret
            })
        }
        Some(wrap::<T>)
    }
}

trait PySequenceContainsProtocolImpl {
    fn sq_contains() -> Option<ffi::objobjproc>;
}

impl<T> PySequenceContainsProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_contains() -> Option<ffi::objobjproc> {
        None
    }
}

impl<T> PySequenceContainsProtocolImpl for T
    where T: PySequenceContainsProtocol
{
    #[inline]
    fn sq_contains() -> Option<ffi::objobjproc> {
        py_objobj_proc!(PySequenceContainsProtocol, T::__contains__, BoolConverter)
    }
}

trait PySequenceConcatProtocolImpl {
    fn sq_concat() -> Option<ffi::binaryfunc>;
}

impl<T> PySequenceConcatProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_concat() -> Option<ffi::binaryfunc> {
        None
    }
}

impl<T> PySequenceConcatProtocolImpl for T
    where T: PySequenceConcatProtocol
{
    #[inline]
    fn sq_concat() -> Option<ffi::binaryfunc> {
        py_binary_func!(PySequenceConcatProtocol, T::__concat__, PyObjectCallbackConverter)
    }
}

trait PySequenceRepeatProtocolImpl {
    fn sq_repeat() -> Option<ffi::ssizeargfunc>;
}

impl<T> PySequenceRepeatProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_repeat() -> Option<ffi::ssizeargfunc> {
        None
    }
}

impl<T> PySequenceRepeatProtocolImpl for T
    where T: PySequenceRepeatProtocol
{
    #[inline]
    fn sq_repeat() -> Option<ffi::ssizeargfunc> {
        py_ssizearg_func!(PySequenceRepeatProtocol, T::__repeat__, PyObjectCallbackConverter)
    }
}

trait PySequenceInplaceConcatProtocolImpl {
    fn sq_inplace_concat() -> Option<ffi::binaryfunc>;
}

impl<T> PySequenceInplaceConcatProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_inplace_concat() -> Option<ffi::binaryfunc> {
        None
    }
}

impl<T> PySequenceInplaceConcatProtocolImpl for T
    where T: PySequenceInplaceConcatProtocol
{
    #[inline]
    fn sq_inplace_concat() -> Option<ffi::binaryfunc> {
        py_binary_func!(PySequenceInplaceConcatProtocol, T::__inplace_concat__, PyObjectCallbackConverter)
    }
}

trait PySequenceInplaceRepeatProtocolImpl {
    fn sq_inplace_repeat() -> Option<ffi::ssizeargfunc>;
}

impl<T> PySequenceInplaceRepeatProtocolImpl for T
    where T: PySequenceProtocol
{
    #[inline]
    default fn sq_inplace_repeat() -> Option<ffi::ssizeargfunc> {
        None
    }
}

impl<T> PySequenceInplaceRepeatProtocolImpl for T
    where T: PySequenceInplaceRepeatProtocol
{
    #[inline]
    fn sq_inplace_repeat() -> Option<ffi::ssizeargfunc> {
        py_ssizearg_func!(PySequenceInplaceRepeatProtocol, T::__inplace_repeat__, PyObjectCallbackConverter)
    }
}