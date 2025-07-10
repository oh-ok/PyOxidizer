// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use {
    crate::importer::ImporterState,
    pyo3::{exceptions::PyFileNotFoundError, prelude::*, types::*, IntoPyObjectExt},
    std::sync::Arc,
};

/// Implements in-memory reading of resource data.
///
/// Implements importlib.abc.ResourceReader.
#[pyclass(module = "oxidized_importer")]
pub(crate) struct OxidizedResourceReader {
    state: Arc<ImporterState>,
    package: String,
}

impl OxidizedResourceReader {
    pub(crate) fn new(state: Arc<ImporterState>, package: String) -> Self {
        Self { state, package }
    }
}

#[pymethods]
impl OxidizedResourceReader {
    /// Returns an opened, file-like object for binary reading of the resource.
    ///
    /// If the resource cannot be found, FileNotFoundError is raised.
    fn open_resource<'p>(&self, py: Python<'p>, resource: &str) -> PyResult<Bound<'p, PyAny>> {
        if let Some(file) = self.state.get_resources_state().get_package_resource_file(
            py,
            &self.package,
            resource,
        )? {
            Ok(file)
        } else {
            Err(PyFileNotFoundError::new_err("resource not found"))
        }
    }

    /// Returns the file system path to the resource.
    ///
    /// If the resource does not concretely exist on the file system, raise
    /// FileNotFoundError.
    #[allow(unused)]
    fn resource_path(&self, resource: Py<PyAny>) -> PyResult<()> {
        Err(PyFileNotFoundError::new_err(
            "in-memory resources do not have filesystem paths",
        ))
    }

    /// Returns True if the named name is considered a resource. FileNotFoundError
    /// is raised if name does not exist.
    fn is_resource(&self, name: &str) -> PyResult<bool> {
        if self
            .state
            .get_resources_state()
            .is_package_resource(&self.package, name)
        {
            Ok(true)
        } else {
            Err(PyFileNotFoundError::new_err("resource not found"))
        }
    }

    /// Returns an iterable of strings over the contents of the package.
    ///
    /// Do note that it is not required that all names returned by the iterator be actual resources,
    /// e.g. it is acceptable to return names for which is_resource() would be false.
    ///
    /// Allowing non-resource names to be returned is to allow for situations where how a package
    /// and its resources are stored are known a priori and the non-resource names would be useful.
    /// For instance, returning subdirectory names is allowed so that when it is known that the
    /// package and resources are stored on the file system then those subdirectory names can be
    /// used directly.
    fn contents<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        self.state
            .get_resources_state()
            .package_resource_names(py, &self.package)
    }

    /// Returns an object implementing importlib.resources.Traversable.
    fn files<'p>(self_: Py<Self>, py: Python<'p>) -> PyResult<Py<OxidizedResourceRoot>> {
        OxidizedResourceRoot::new(self_).and_then(|p| Py::new(py, p))
    }
}

/// Implements the importlib.resources.Traversable interface for a resource.
///
/// [OxidizedResourceRoot] and [OxidizedResourcePath] allow for object oriented access to
/// package resources. This is essentially a simple wrapper around an [OxidizedResourceReader].
#[pyclass(module = "oxidized_importer")]
struct OxidizedResourceRoot {
    reader: Py<OxidizedResourceReader>,
}

impl OxidizedResourceRoot {
    pub(crate) fn new(reader: Py<OxidizedResourceReader>) -> PyResult<Self> {
        Ok(Self { reader })
    }
}

#[pymethods]
impl OxidizedResourceRoot {
    fn iterdir<'p>(&self, py: Python<'p>) -> PyResult<Vec<OxidizedResourcePath>> {
        let reader = self.reader.borrow(py);
        let result = reader
            .state
            .get_resources_state()
            .package_resources_list_directory(&reader.package, "")
            .iter()
            .map(|p| self.__truediv__(py, p))
            .collect();

        result
    }

    fn is_dir(&self) -> PyResult<bool> {
        Ok(true)
    }
    fn is_file(&self) -> PyResult<bool> {
        Ok(false)
    }

    #[pyo3(signature = (*others))]
    fn joinpath<'p>(
        &self,
        py: Python<'p>,
        others: Bound<'p, PyTuple>,
    ) -> PyResult<OxidizedResourcePath> {
        let others = others.extract::<Vec<String>>()?;
        OxidizedResourcePath::new(self.reader.clone_ref(py), others.join("/"))
    }

    fn __truediv__<'p>(&self, py: Python, other: &str) -> PyResult<OxidizedResourcePath> {
        let reader = self.reader.clone_ref(py);
        OxidizedResourcePath::new(reader, other.to_string())
    }

    #[pyo3(signature = (*_args, **_kwargs))]
    fn open(&self, _args: Py<PyAny>, _kwargs: Option<Py<PyAny>>) -> PyErr {
        PyFileNotFoundError::new_err("resource not found")
    }

    fn read_bytes(&self) -> PyErr {
        PyFileNotFoundError::new_err("resource not found")
    }

    #[pyo3(signature = (*_args, **_kwargs))]
    fn read_text(&self, _args: Py<PyAny>, _kwargs: Option<Py<PyAny>>) -> PyErr {
        PyFileNotFoundError::new_err("resource not found")
    }

    fn __eq__<'p>(&self, py: Python<'p>, other: Bound<'p, PyAny>) -> PyResult<Bound<'p, PyAny>> {
        match other.downcast_into::<Self>() {
            Err(_) => Ok(PyNotImplemented::get(py).to_owned().into_any()),
            Ok(other) => (other.borrow().reader.borrow(py).package
                == self.reader.borrow(py).package)
                .into_bound_py_any(py),
        }
    }

    #[getter]
    fn name<'p>(&self, py: Python<'p>) -> String {
        self.reader.borrow(py).package.clone()
    }
}

#[pyclass(module = "oxidized_importer")]
struct OxidizedResourcePath {
    reader: Py<OxidizedResourceReader>,
    path: String,
}

impl OxidizedResourcePath {
    pub(crate) fn new(reader: Py<OxidizedResourceReader>, path: String) -> PyResult<Self> {
        Ok(Self { reader, path })
    }
}

#[pymethods]
impl OxidizedResourcePath {
    fn iterdir<'p>(&self, py: Python<'p>) -> PyResult<Vec<Self>> {
        let reader = self.reader.borrow(py);
        let result = reader
            .state
            .get_resources_state()
            .package_resources_list_directory(&reader.package, &self.path)
            .iter()
            .map(|p| self.__truediv__(py, p))
            .collect();

        result
    }

    fn is_dir(&self, py: Python) -> PyResult<bool> {
        let reader = self.reader.borrow(py);
        Ok(reader
            .state
            .get_resources_state()
            .is_package_resource_directory(&reader.package, &self.path))
    }

    fn is_file(&self, py: Python) -> PyResult<bool> {
        let reader = self.reader.borrow(py);
        let state = reader.state.get_resources_state();
        Ok(state.is_package_resource(&reader.package, &self.path)
            && !state.is_package_resource_directory(&reader.package, &self.path))
    }

    #[pyo3(signature = (*others))]
    fn joinpath<'p>(&self, others: Bound<'p, PyTuple>) -> PyResult<OxidizedResourcePath> {
        let mut parts = Vec::with_capacity(others.len() + 1);

        parts.push(self.path.clone());

        for part in &others {
            parts.push(part.extract()?);
        }

        Self::new(self.reader.clone_ref(others.py()), parts.join("/"))
    }

    fn __truediv__<'p>(&self, py: Python, other: &str) -> PyResult<Self> {
        let reader = self.reader.clone_ref(py);
        Self::new(reader, [&self.path, other].join("/"))
    }

    #[pyo3(signature = (mode=None, *args, **kwargs))]
    fn open<'p>(
        &self,
        py: Python<'p>,
        mode: Option<&str>,
        args: Bound<'p, PyTuple>,
        kwargs: Option<Bound<'p, PyDict>>,
    ) -> PyResult<Bound<'p, PyAny>> {
        let reader = self.reader.borrow(py);
        let file = reader.open_resource(py, &self.path)?;

        let mode = mode.unwrap_or("rb");

        if "r" == mode {
            let io = py.import("io")?;
            let cls = io.getattr("TextIOWrapper")?;
            let _args = PyTuple::new(py, [file])?;
            let _args = _args.add(args)?;
            let _args = _args.downcast_into::<PyTuple>()?;
            return cls.call(_args, kwargs.as_ref());
        }

        Ok(file)
    }

    fn read_bytes<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let file = self.open(py, Some("rb"), PyTuple::empty(py), None)?;
        let result = file.call_method0("read");
        file.call_method0("close")?;

        result
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn read_text<'p>(
        &self,
        py: Python<'p>,
        args: Bound<'p, PyTuple>,
        kwargs: Option<Bound<'p, PyDict>>,
    ) -> PyResult<Bound<'p, PyAny>> {
        let file = self.open(py, Some("r"), args, kwargs)?;
        let result = file.call_method0("read");
        file.call_method0("close")?;

        result
    }

    fn __eq__<'p>(&self, py: Python<'p>, other: Bound<'p, PyAny>) -> PyResult<Bound<'p, PyAny>> {
        match other.downcast_into::<Self>() {
            Err(_) => Ok(PyNotImplemented::get(py).to_owned().into_any()),
            Ok(other) => {
                let other = other.borrow();
                let my_reader = self.reader.borrow(py);
                let their_reader = other.reader.borrow(py);
                ((my_reader.package == their_reader.package) && (self.path == other.path))
                    .into_bound_py_any(py)
            }
        }
    }

    #[getter]
    fn name(&self) -> &str {
        match self.path.rsplit_once("/") {
            Some((_, name)) => name,
            None => &self.path,
        }
    }
}
