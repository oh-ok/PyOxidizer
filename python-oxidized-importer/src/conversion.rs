// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Bridge Rust and Python string types.

use {
    pyo3::{buffer::PyBuffer, prelude::*, types::*},
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
    },
};

#[cfg(target_family = "unix")]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

/// Convert a Rust Path to a pathlib.Path.
pub fn path_to_pathlib_path<'p>(py: Python<'p>, path: &Path) -> PyResult<Bound<'p, PyAny>> {
    let py_str: Bound<PyAny> = path.into_pyobject(py)?;

    let pathlib = py.import("pathlib")?;

    pathlib.getattr("Path")?.call((py_str,), None)
}

#[cfg(unix)]
pub fn pyobject_to_pathbuf<'p>(value: Bound<'p, PyAny>) -> PyResult<PathBuf> {
    let os = value.py().import("os")?;

    let encoded = os
        .getattr("fsencode")?
        .call((value,), None)?
        .extract::<Vec<u8>>()?;
    let os_str = OsStr::from_bytes(&encoded);

    Ok(PathBuf::from(os_str))
}

#[cfg(windows)]
pub fn pyobject_to_pathbuf(py: Python, value: &PyAny) -> PyResult<PathBuf> {
    let os = py.import("os")?;

    // This conversion is a bit wonky. First, the PyObject could be of various
    // types: str, bytes, or a path-like object. We normalize to a PyString
    // by round tripping through os.fsencode() and os.fsdecode(). The
    // os.fsencode() will take care of the type normalization for us and
    // os.fsdecode() gets us a PyString.
    let encoded = os.getattr("fsencode")?.call((value,), None)?;
    let normalized = os.getattr("fsdecode")?.call((encoded,), None)?;

    // We now have a Python str, which is a series of code points. The
    // ideal thing to do here would be to go to a wchar_t, then to OsStr,
    // then to PathBuf. As then the PathBuf would hold onto the native
    // wchar_t and allow lossless round-tripping. But we're lazy. So
    // we simply convert to a Rust string and feed that into PathBuf.
    // It should be close enough.
    let rust_normalized = normalized.extract::<String>()?;

    Ok(PathBuf::from(rust_normalized))
}

pub fn pyobject_to_pathbuf_optional(value: Bound<PyAny>) -> PyResult<Option<PathBuf>> {
    if value.is_none() {
        Ok(None)
    } else {
        Ok(Some(pyobject_to_pathbuf(value)?))
    }
}

/// Attempt to convert a PyObject to an owned Vec<u8>.
pub fn pyobject_to_owned_bytes(value: Bound<PyAny>) -> PyResult<Vec<u8>> {
    let buffer = PyBuffer::<u8>::get(&value)?;

    let data = unsafe {
        std::slice::from_raw_parts::<u8>(buffer.buf_ptr() as *const _, buffer.len_bytes())
    };

    Ok(data.to_owned())
}

/// Attempt to convert a PyObject to owned Vec<u8>.
///
/// Returns Ok(None) if PyObject is None.
pub fn pyobject_to_owned_bytes_optional(value: Bound<PyAny>) -> PyResult<Option<Vec<u8>>> {
    if value.is_none() {
        Ok(None)
    } else {
        Ok(Some(pyobject_to_owned_bytes(value)?))
    }
}

pub fn pyobject_optional_resources_map_to_owned_bytes(
    value: Bound<PyAny>,
) -> PyResult<Option<HashMap<String, Vec<u8>>>> {
    if value.is_none() {
        Ok(None)
    } else {
        let source = value.downcast::<PyDict>()?;
        let mut res = HashMap::with_capacity(source.len());

        for (k, v) in source.iter() {
            res.insert(k.extract::<String>()?, pyobject_to_owned_bytes(v)?);
        }

        Ok(Some(res))
    }
}

pub fn pyobject_optional_resources_map_to_pathbuf(
    value: Bound<PyAny>,
) -> PyResult<Option<HashMap<String, PathBuf>>> {
    if value.is_none() {
        Ok(None)
    } else {
        let source = value.downcast::<PyDict>()?;
        let mut res = HashMap::with_capacity(source.len());

        for (k, v) in source.iter() {
            res.insert(k.extract::<String>()?, pyobject_to_pathbuf(v)?);
        }

        Ok(Some(res))
    }
}
