pub struct Sandbox {
    path: String,
}

#[derive(Default)]
pub struct SandboxBuilder {
    path: Option<String>,
}

impl SandboxBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn build(self) -> Result<Sandbox, &'static str> {
        let path = self.path.ok_or("Executable path is required")?;
        Ok(Sandbox { path })
    }
}

impl Sandbox {
    pub fn setup() -> SandboxBuilder {
        SandboxBuilder::new()
    }

    pub fn run(&self) {
        println!("Running sandbox with binary: {}", self.path);
    }

    pub fn path(&self) -> &str {
        &self.path
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