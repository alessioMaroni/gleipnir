pub mod setup;

use setup::path::ExecutablePath;
use std::path::Path;

pub struct Sandbox {
    path: ExecutablePath,
}

#[derive(Default)]
pub struct SandboxBuilder {
    path: Option<ExecutablePath>,
}

impl SandboxBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(ExecutablePath::new(path));
        self
    }

    pub fn build(self) -> Result<Sandbox, &'static str> {
        let mut exec_path = self.path.ok_or("Executable path is required")?;

        match exec_path.verify() {
            Ok(true) => Ok(Sandbox { path: exec_path }),
            Ok(false) => {
                exec_path.debug_executable_path_struct();
                Err("Provided path is not a valid executable file")
            }
            Err(_) => Err("Failed to access or read executable path"),
        }
    }
}

impl Sandbox {
    pub fn setup() -> SandboxBuilder {
        SandboxBuilder::new()
    }

    pub fn run(&self) {
        println!("Running sandbox with binary: {}", self.path.inner.display());
    }

    pub fn path(&self) -> &Path {
        &self.path.inner
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn main_test() -> Result<(), &'static str> {
        let sandbox = Sandbox::setup()
            .path("/usr/bin/python3")
            .build()?;

        sandbox.run();
        Ok(())
    }
}