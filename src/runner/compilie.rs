use anyhow::Result;

#[derive(Debug, Clone)]
pub(crate) struct CompileStep {
    program: String,
    args: Vec<String>,
    current_dir: Option<String>,
}

impl CompileStep {
    pub(crate) const fn new(
        program: String,
        args: Vec<String>,
        current_dir: Option<String>,
    ) -> Self {
        Self {
            program,
            args,
            current_dir,
        }
    }
}

pub(super) fn compile(steps: &[CompileStep]) -> Result<()> {
    for step in steps {
        let mut cmd = std::process::Command::new(&step.program);
        cmd.args(&step.args);

        if let Some(ref dir) = step.current_dir {
            cmd.current_dir(dir);
        }

        let status = cmd.status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to compile"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compile_success() {
        let steps = vec![CompileStep::new("true".to_string(), vec![], None)];
        assert!(compile(&steps).is_ok());
    }

    #[test]
    fn test_compile_fail() {
        let steps = vec![CompileStep::new("false".to_string(), vec![], None)];
        assert!(compile(&steps).is_err());
    }
}
