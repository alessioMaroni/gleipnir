//! ```rust
//! use gleipnir::setup::path::ExecutablePath;
//! ```
//! 
//! Manage "executable file's" path locating and verification
//! - Check if the path exist
//! - If is a file
//!     - If it's executable

use std::fs::File;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, io::Error>;

#[derive(Debug, Clone)]
pub struct ExecutablePath {
    pub inner: PathBuf,
    exist: bool,
    is_file: bool,
    is_exe: bool,
}

impl ExecutablePath {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: path.as_ref().to_path_buf(),
            exist: false,
            is_file: false,
            is_exe:false
        }
    }

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

    pub fn debug_executable_path_struct(&self) {
        if !self.exist == true{
            eprintln!("{:?} dosen't exist", self.inner);
        }
        
        if !self.is_file == true {
            eprintln!("{:?} isn't a file", self.inner);
        }

        if !self.is_exe == true {
            eprintln!("{:?} isn't executable", self.inner);
        }
    }

    // TODO: Consider to remove this function,
    // function is_file alredy check is exist
    fn exits(&self) -> Result<bool> {
        Ok(self.inner.exists())
    }

    fn is_file(&self) -> Result<bool> {
        Ok(self.inner.is_file())
    }

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
        let mut exec = ExecutablePath::new("/path/that/does/not/exist/gleipnir_test");
        assert_eq!(exec.verify().unwrap(), false);
    }

    #[test]
    fn test_directory_is_not_executable_file() {
        let mut exec = ExecutablePath::new("/tmp");
        assert_eq!(exec.verify().unwrap(), false);
    }

    #[test]
    fn test_file_without_execution_permissions() -> io::Result<()> {
        let temp_path = std::env::temp_dir().join("gleipnir_no_exec.txt");
        let mut file = File::create(&temp_path)?;
        file.write_all(b"test file content")?;

        let mut exec = ExecutablePath::new(&temp_path);
        assert_eq!(exec.verify()?, false);

        let _ = fs::remove_file(temp_path);
        Ok(())
    }

    #[test]
    fn test_executable_file_invalid_elf_header() -> io::Result<()> {
        let temp_path = std::env::temp_dir().join("gleipnir_script.sh");
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
        let temp_path = std::env::temp_dir().join("gleipnir_valid_elf");
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