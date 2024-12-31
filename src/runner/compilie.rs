use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompileStep {
    program: String,
    args: Vec<String>,
    current_dir: Option<String>,
    os_regex: Option<String>,
}

impl CompileStep {
    pub(crate) const fn new(
        program: String,
        args: Vec<String>,
        current_dir: Option<String>,
        os_regex: Option<String>,
    ) -> Self {
        Self {
            program,
            args,
            current_dir,
            os_regex,
        }
    }
}

pub(super) fn compile(steps: &[CompileStep]) -> Result<()> {
    for step in steps {
        if let Some(os_regex) = &step.os_regex {
            // os_regex が指定された場合は現在実行中の OS がその正規表現にマッチしたときだけ処理を行う
            // 本当は正規表現オブジェクト使いまわすために Option<Regex> にしたいが TestStep を Serialize するために Option<String> のままにしている
            match Regex::new(os_regex) {
                Ok(re) => {
                    if !re.is_match(std::env::consts::OS) {
                        // 正規表現がマッチしなかった場合はその teststep はスキップする
                        continue;
                    }
                }
                Err(error) => {
                    // エラー終了にすべき？
                    eprintln!("Invalid regex found in test step.\n{}", error);
                    continue;
                }
            }
        }
        let mut cmd = std::process::Command::new(&step.program);
        cmd.args(&step.args);

        if let Some(ref dir) = step.current_dir {
            cmd.current_dir(dir);
        }

        let status = cmd
            .status()
            .with_context(|| format!("Failed to compile. command: {:?}", cmd))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Failed to compile. command: {:?}, status: {}",
                cmd,
                status
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compile_success() {
        let steps = vec![CompileStep::new("true".to_string(), vec![], None, None)];
        assert!(compile(&steps).is_ok());
    }

    #[test]
    fn test_compile_fail() {
        let steps = vec![CompileStep::new("false".to_string(), vec![], None, None)];
        assert!(compile(&steps).is_err());
    }
    #[test]
    fn test_compile_os_specific_success() {
        // 成功ケースのテストのため、テストが実行される OS 用の正規表現を作る
        let os_regex = std::env::consts::OS;
        let steps = vec![CompileStep::new("true".to_string(), vec![], None, Some(os_regex.to_string()))];
        assert!(compile(&steps).is_ok());
    }

    #[test]
    fn test_compile_os_specific_fail() {
        // 失敗ケースのテストのため、テストが実行される OS 以外の正規表現を作る
        let os_regex = if std::env::consts::OS == "linux" { "macos" } else { "linux" };
        let steps = vec![
            CompileStep::new("true".to_string(), vec![], None, None),
            CompileStep::new("false".to_string(), vec![], None, Some(os_regex.to_string()))
        ];
        assert!(compile(&steps).is_ok());
    }
}
