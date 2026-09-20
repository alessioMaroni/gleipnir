//! # NAME
//! `pypnir::setup::path` - Target executable path location and ELF binary validation
//!
//! # SYNOPSIS
//! ```rust
//! use pypnir::setup::path::ExecutablePath;
//!
//! let mut exec_path = ExecutablePath::new("/usr/bin/ls");
//! if exec_path.verify().unwrap_or(false) {
//!     println!("Valid executable binary found.");
//! } else {
//!     exec_path.debug_executable_path_struct();
//! }
//! ```
//!
//! # DESCRIPTION
//! The `path` module handles validation for target binaries prior to sandbox execution.
//! It guarantees that a given file system path satisfies three critical criteria:
//!
//! 1. **Existence**: The path points to an existing entry on the target file system.
//! 2. **File Type**: The path represents a regular file (not a directory, FIFO, block device, or socket).
//! 3. **Execution Eligibility**:
//!    - **POSIX Permissions**: The file mode mask satisfies execution bit criteria (`mode & 0o111 != 0`).
//!    - **ELF Binary Format**: The first 4 bytes match the Executable and Linkable Format (ELF) magic header (`\x7FELF` or `[0x7F, 0x45, 0x4C, 0x46]`).

use std::fs::File;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, io::Error>;

/// Represents a candidate binary path and tracks its verification state.
///
/// Holds the underlying target [`PathBuf`] and status flags updated during calling [`verify`](ExecutablePath::verify).
#[derive(Debug, Clone)]
pub struct ExecutablePath {
    /// The target file system path stored as a [`PathBuf`].
    pub inner: PathBuf,
    exist: bool,
    is_file: bool,
    is_exe: bool,
}

impl ExecutablePath {
    /// Creates a new `ExecutablePath` instance from any path-like reference.
    ///
    /// Accepts types implementing `AsRef<Path>` (`&str`, `String`, `&Path`, `PathBuf`), avoiding forced path allocations by the caller.
    ///
    /// # Parameters
    /// - `path`: Path reference pointing to the target binary location.
    ///
    /// # Examples
    /// ```rust
    /// use pypnir::setup::path::ExecutablePath;
    ///
    /// let path_str = "/usr/bin/python3";
    /// let exec_path = ExecutablePath::new(path_str);
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: path.as_ref().to_path_buf(),
            exist: false,
            is_file: false,
            is_exe: false,
        }
    }

    /// Performs verification on the target path and updates internal state flags.
    ///
    /// Short-circuits execution and returns `Ok(false)` at the first failing stage:
    /// 1. Verifies existence (`exits`).
    /// 2. Verifies file type (`is_file`).
    /// 3. Validates POSIX permission mode bitmask (`0o111`) and checks the ELF header (`0x7F454C46`).
    ///
    /// # Returns
    /// - `Ok(true)` if all validation phases pass successfully.
    /// - `Ok(false)` if any check fails or if the binary is non-executable / invalid ELF format.
    /// - `Err(std::io::Error)` if a low-level I/O failure occurs during system metadata access.
    pub fn verify(&mut self) -> Result<bool> {
        let exist = self.exits()?;
        if !exist {
            self.exist = false;
            self.is_file = false;
            self.is_exe = false;
            return Ok(false);
        }

        let is_file = self.is_file()?;
        if !is_file {
            self.exist = true;
            self.is_file = false;
            self.is_exe = false;
            return Ok(false);
        }

        let is_exe = self.is_exe()?;

        self.exist = true;
        self.is_file = true;
        self.is_exe = is_exe;

        Ok(is_exe)
    }

    /// Writes diagnostic failure details to standard error (`stderr`).
    ///
    /// Inspects internal validation flags (`exist`, `is_file`, `is_exe`) and outputs
    /// specific failure reasons for troubleshooting target binaries.
    pub fn debug_executable_path_struct(&self) {
        if !self.exist {
            eprintln!("{:?} doesn't exist", self.inner);
        }

        if !self.is_file {
            eprintln!("{:?} isn't a file", self.inner);
        }

        if !self.is_exe {
            eprintln!("{:?} isn't executable", self.inner);
        }
    }

    /// Checks if the target path exists on the file system.
    fn exits(&self) -> Result<bool> {
        Ok(self.inner.exists())
    }

    /// Checks if the target path points to a regular file.
    fn is_file(&self) -> Result<bool> {
        Ok(self.inner.is_file())
    }

    /// Evaluates execution permission bits and checks the 4-byte ELF magic header.
    ///
    /// Evaluates POSIX mode mask against `0o111` (`S_IXUSR | S_IXGRP | S_IXOTH`).
    /// Reads the first 4 bytes of the binary to match against magic sequence `[0x7F, 0x45, 0x4C, 0x46]`.
    fn is_exe(&self) -> Result<bool> {
        let path = &self.inner;

        let metadata = std::fs::metadata(path)?;

        let permission = metadata.permissions();
        if permission.mode() & 0o111 == 0 {
            return Ok(false);
        }

        let mut file = File::open(path)?;
        let mut buffer = [0u8; 4];

        if file.read_exact(&mut buffer).is_err() {
            return Ok(false);
        }

        let elf_magic = [0x7F, 0x45, 0x4C, 0x46];

        Ok(buffer == elf_magic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_non_existent_path() {
        let mut exec = ExecutablePath::new("/path/that/does/not/exist/pypnir_test");
        assert_eq!(exec.verify().unwrap(), false);
    }

    #[test]
    fn test_directory_is_not_executable_file() {
        let mut exec = ExecutablePath::new("/tmp");
        assert_eq!(exec.verify().unwrap(), false);
    }

    #[test]
    fn test_file_without_execution_permissions() -> io::Result<()> {
        let temp_path = std::env::temp_dir().join("pypnir_no_exec.txt");
        let mut file = File::create(&temp_path)?;
        file.write_all(b"test file content")?;

        let mut exec = ExecutablePath::new(&temp_path);
        assert_eq!(exec.verify()?, false);

        let _ = fs::remove_file(temp_path);
        Ok(())
    }

    #[test]
    fn test_executable_file_invalid_elf_header() -> io::Result<()> {
        let temp_path = std::env::temp_dir().join("pypnir_script.sh");
        let mut file = File::create(&temp_path)?;
        file.write_all(b"#!/bin/sh\necho hello")?;

        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;

        let mut exec = ExecutablePath::new(&temp_path);
        assert_eq!(exec.verify()?, false);

        let _ = fs::remove_file(temp_path);
        Ok(())
    }

    #[test]
    fn test_valid_elf_executable() -> io::Result<()> {
        let temp_path = std::env::temp_dir().join("pypnir_valid_elf");
        let mut file = File::create(&temp_path)?;

        file.write_all(&[0x7F, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00])?;

        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;

        let mut exec = ExecutablePath::new(&temp_path);
        assert_eq!(exec.verify()?, true);

        let _ = fs::remove_file(temp_path);
        Ok(())
    }
}
