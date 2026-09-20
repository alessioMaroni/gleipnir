//! Python bindings for the `pypnir` sandbox library using PyO3.
//!
//! Provides Python classes `Sandbox` and `SandboxBuilder` wrapping the native
//! Rust sandbox implementation for process isolation.

use ::pypnir::{Sandbox, SandboxBuilder};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Python wrapper around the native Rust [`Sandbox`] struct.
///
/// Exposes process isolation functionality to Python code as `pypnir.Sandbox`.
#[pyclass(name = "Sandbox")]
pub struct PySandbox {
    inner: Sandbox,
}

#[pymethods]
impl PySandbox {
    /// Initializes a new [`PySandboxBuilder`] to configure and construct a sandbox.
    ///
    /// Exposed to Python as a static method `Sandbox.setup()`.
    #[staticmethod]
    pub fn setup() -> PySandboxBuilder {
        PySandboxBuilder {
            inner: Sandbox::setup(),
        }
    }

    /// Executes the target binary within the isolated sandbox environment.
    ///
    /// Exposed to Python as `Sandbox.run()`.
    pub fn run(&self) {
        self.inner.run();
    }

    /// Returns the target executable path as a string.
    ///
    /// Exposed to Python as `Sandbox.path()`.
    pub fn path(&self) -> String {
        self.inner.path().to_string_lossy().to_string()
    }
}

/// Python wrapper around the native Rust [`SandboxBuilder`] struct.
///
/// Provides a fluent builder interface exposed to Python as `pypnir.SandboxBuilder`.
#[pyclass(name = "SandboxBuilder")]
#[derive(Default)]
pub struct PySandboxBuilder {
    inner: SandboxBuilder,
}

#[pymethods]
impl PySandboxBuilder {
    /// Creates a new `SandboxBuilder` instance.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: SandboxBuilder::new(),
        }
    }

    /// Sets the target executable path for the sandbox environment.
    ///
    /// Accepts a string path and returns the updated builder instance for method chaining.
    pub fn path<'a>(mut slf: PyRefMut<'a, Self>, path: String) -> PyRefMut<'a, Self> {
        let builder = std::mem::take(&mut slf.inner);
        slf.inner = builder.path(path);
        slf
    }

    /// Validates the target executable and constructs a [`PySandbox`] instance.
    ///
    /// # Errors
    ///
    /// Raises a Python `ValueError` if the executable path is missing or invalid.
    pub fn build(&mut self) -> PyResult<PySandbox> {
        let builder = std::mem::take(&mut self.inner);
        match builder.build() {
            Ok(sandbox) => Ok(PySandbox { inner: sandbox }),
            Err(err) => Err(PyValueError::new_err(err)),
        }
    }
}

/// Initializes the native `pypnir` Python module and registers exposed classes.
#[pymodule]
fn pypnir(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySandbox>()?;
    m.add_class::<PySandboxBuilder>()?;
    Ok(())
}
