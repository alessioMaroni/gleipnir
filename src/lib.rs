//! Pypnir Sandbox Library

// Setup module.
// Contains the initialization functions for the Sandbox:
//     - Path: Logic to load the file in the Sandbox,
//             verify that it exists, and that it is executable.
pub mod setup;

use setup::path::ExecutablePath;

use nix::{sys::wait::waitpid, unistd::{fork, ForkResult, write}};
use libc;

use std::path::Path;
use std::process::Command;

/// Represents the isolation environment (sandbox) for executing a target binary.
///
/// `Sandbox` is the primary struct of the library. It manages executable file
/// verification and controls its lifecycle within an isolated process.
///
/// Instantiating this struct must be done via [`SandboxBuilder`],
/// which can be initialized using [`Sandbox::setup`].
///
/// # Examples
///
/// ```rust,no_run
/// use pypnir::Sandbox;
///
/// fn main() -> Result<(), &'static str> {
///     let sandbox = Sandbox::setup()
///         .path("path/of/the/executable")
///         .build()?;
///
///     sandbox.run();
///     Ok(())
/// }
/// ```
pub struct Sandbox {
    /// Path to the verified executable ready for execution.
    path: ExecutablePath,
}

/// A builder for constructing and configuring a [`Sandbox`].
///
/// `SandboxBuilder` provides a fluent interface to specify execution parameters—such
/// as the target executable path—and validates them before instantiating the sandbox.
#[derive(Default)]
pub struct SandboxBuilder {
    /// Optional path to the target executable, validated during [`build`](Self::build).
    path: Option<ExecutablePath>,
}

impl SandboxBuilder {
    /// Initializes the `SandboxBuilder` struct.
    pub fn new() -> Self {
        // Return default instance
        Self::default()
    }

    /// Sets the target executable path for the sandbox environment.
    ///
    /// Wraps the provided path into an [`ExecutablePath`] instance and stores it
    /// within the builder for later validation during [`build`](Self::build).
    ///
    /// # Parameters
    ///
    /// * `path` - Any path-like type implementing [`AsRef<Path>`] (e.g., `&str`, `String`, `PathBuf`).
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        // Initialize 'ExecutablePath' struct, setting its parameters to false:
        // ```rust,no_run
        // pub fn new(path: impl AsRef<Path>) -> Self {
        //     Self {
        //         inner: path.as_ref().to_path_buf(),
        //         exist: false,
        //         is_file: false,
        //         is_exe: false,
        //     }
        // }
        // ```
        self.path = Some(ExecutablePath::new(path));

        // Return self
        self
    }

    /// Consumes the builder, validates the configured executable, and constructs a [`Sandbox`].
    ///
    /// This method verifies that the target path exists, is accessible, and passes
    /// valid binary format (ELF) checks before instantiating the sandbox.
    ///
    /// # Errors
    ///
    /// Returns an `Err(&'static str)` if:
    /// - No executable path was set prior to calling `build`.
    /// - The file at the specified path does not exist or cannot be accessed.
    /// - The target file fails executable verification (e.g., invalid ELF header).
    pub fn build(self) -> Result<Sandbox, &'static str> {
        let mut exec_path = self.path.ok_or("Executable path is required")?;

        // Verify the path
        match exec_path.verify() {
            Ok(true) => Ok(Sandbox { path: exec_path }),

            // Only if exec_path.verify() returns false,
            // we execute debug printing to show why the function returned false
            Ok(false) => {
                exec_path.debug_executable_path_struct();
                Err("Provided path is not a valid executable file")
            }
            Err(_) => Err("Failed to access or read executable path"),
        }
    }
}

impl Sandbox {
    /// Creates a new [`SandboxBuilder`] to configure and construct a [`Sandbox`].
    ///
    /// This serves as the primary entry point for setting up a sandbox instance.
    pub fn setup() -> SandboxBuilder {
        SandboxBuilder::new()
    }

    /// Executes the target binary within an isolated environment using a double-fork mechanism.
    ///
    /// This method manages the lifecycle of the sandbox execution. It spawns an intermediate
    /// child process which in turn spawns a secondary child process to execute the target binary.
    /// The double-fork pattern ensures process isolation and prevents zombie processes.
    ///
    /// # Execution Flow
    ///
    /// 1. **First Fork (`Child 1`)**: Spawns an intermediate monitoring process. The parent waits for `Child 1` to exit.
    /// 2. **Second Fork (`Child 2`)**: Spawns the execution process that executes the target binary via [`Command`].
    /// 3. **Binary Execution**: `Child 2` runs the specified binary, waits for completion, and outputs the exit status code.
    /// 4. **Cleanup**: Both child processes bypass standard Rust destructors and buffer flushes by calling [`libc::_exit`] directly.
    ///
    /// # Side Effects
    ///
    /// - Spawns sub-processes in the operating system.
    /// - Performs raw writes to `stdout` to ensure immediate log output across process boundaries.
    /// - Terminates child process branches immediately via `libc::_exit`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pypnir::Sandbox;
    ///
    /// let sandbox = Sandbox::setup()
    ///     .path("build/test")
    ///     .build()?;
    ///
    /// sandbox.run();
    /// # Ok::<(), &'static str>(())
    /// ```
    // Attribute allowing unsafe code for this function
    #[allow(unsafe_code)]
    pub fn run(&self) {
        println!("Running sandbox with binary: {}", self.path.inner.display());

        // First fork
        match unsafe { fork() } {
            // First parent
            Ok(ForkResult::Parent { child }) => {
                println!("Continuing execution in parent process, new child has pid: {}", child);
                waitpid(child, None).unwrap();
            }
            // First child
            Ok(ForkResult::Child) => {
                write(std::io::stdout(), "[Child 1] \n".as_bytes()).ok();

                // Second fork
                match unsafe { fork() } {
                    // Second parent
                    Ok(ForkResult::Parent { child }) => {
                        println!("Continuing execution in parent process, second child has pid: {}", child);
                        waitpid(child, None).unwrap();
                    }
                    // Second child
                    Ok(ForkResult::Child) => {
                        write(std::io::stdout(), "[Child 2] \n".as_bytes()).ok();

                        // Convert the target executable `PathBuf` (`self.path.inner`) into a valid UTF-8 string slice.
                        // Since paths on Linux/POSIX are OS-native byte sequences (`OsStr`) and not guaranteed
                        // to be UTF-8, `to_str()` returns an `Option<&str>`.
                        let prompt = match self.path.inner.to_str() {
                            // If the path is valid UTF-8, format it as a relative execution path (e.g., "./build/test")
                            Some(prompt_str) => format!("./{}", prompt_str),
                            // Fallback handling if the path contains non-UTF-8 sequence bytes
                            None => {
                                write(std::io::stdout(), "File path isn't in UTF-8 format\n".as_bytes()).ok();
                                String::from("Path Error")
                            }
                        };

                        let exit_code = Command::new(prompt)
                            .output()
                            .expect("Failed to run command");

                        // Print the program's exit code
                        {
                            let output_msg = format!("Program exit code: [{}]\n", exit_code.status);
                            write(std::io::stdout(), output_msg.as_bytes()).ok();
                        }

                        unsafe { libc::_exit(0) };
                    }
                    Err(_) => println!("Fork error in second child"),
                }

                unsafe { libc::_exit(0) };
            }
            Err(_) => println!("Fork error in main process"),
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
