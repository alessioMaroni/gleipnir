pub mod setup;

use setup::path::ExecutablePath;

use nix::{sys::wait::waitpid,unistd::{fork, ForkResult, write}};
use libc;

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

    #[allow(unsafe_code)]
    pub fn run(&self) {
        println!("Running sandbox with binary: {}", self.path.inner.display());

        match unsafe{fork()}{
            Ok(ForkResult::Parent {child}) => {
                println!("Continuing execution in parent process, new child has pid: {}", child);
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                write(std::io::stdout(), "[Child] \n".as_bytes()).ok();
                unsafe { libc::_exit(0) };
            }
            Err(_) => println!("Fork error"),
        }
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
            .path("build/test")
            .build()?;

        sandbox.run();
        Ok(())
    }
}
