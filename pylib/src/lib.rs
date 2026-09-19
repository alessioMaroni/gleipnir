// TODO: Change Project Name to pypnir

use ::gleipnir::{Sandbox, SandboxBuilder};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass(name = "Sandbox")]
pub struct PySandbox {
    inner: Sandbox,
}

#[pymethods]
impl PySandbox {
    #[staticmethod]
    pub fn setup() -> PySandboxBuilder {
        PySandboxBuilder {
            inner: Sandbox::setup(),
        }
    }

    pub fn run(&self) {
        self.inner.run();
    }

    pub fn path(&self) -> String {
        self.inner.path().to_string_lossy().to_string()
    }
}

#[pyclass(name = "SandboxBuilder")]
#[derive(Default)]
pub struct PySandboxBuilder {
    inner: SandboxBuilder,
}

#[pymethods]
impl PySandboxBuilder {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: SandboxBuilder::new(),
        }
    }

    pub fn path<'a>(mut slf: PyRefMut<'a, Self>, path: String) -> PyRefMut<'a, Self> {
        let builder = std::mem::take(&mut slf.inner);
        slf.inner = builder.path(path);
        slf
    }

    pub fn build(&mut self) -> PyResult<PySandbox> {
        let builder = std::mem::take(&mut self.inner);
        match builder.build() {
            Ok(sandbox) => Ok(PySandbox { inner: sandbox }),
            Err(err) => Err(PyValueError::new_err(err)),
        }
    }
}

#[pymodule]
fn pypnir(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySandbox>()?;
    m.add_class::<PySandboxBuilder>()?;
    Ok(())
}
