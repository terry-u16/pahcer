use anyhow::Result;
use regex::Regex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct TestStep {
    program: String,
    args: Vec<String>,
    current_dir: Option<String>,
    stdin: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

impl TestStep {
    pub(crate) const fn new(
        program: String,
        args: Vec<String>,
        current_dir: Option<String>,
        stdin: Option<String>,
        stdout: Option<String>,
        stderr: Option<String>,
    ) -> Self {
        Self {
            program,
            args,
            current_dir,
            stdin,
            stdout,
            stderr,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TestResult {
    seed: u64,
    score: Result<f64, String>,
    duration: Duration,
}

impl TestResult {
    pub(crate) const fn new(seed: u64, score: Result<f64, String>, duration: Duration) -> Self {
        Self {
            seed,
            score,
            duration,
        }
    }

    pub(crate) const fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) const fn score(&self) -> &Result<f64, String> {
        &self.score
    }

    pub(crate) const fn duration(&self) -> Duration {
        self.duration
    }
}

pub(crate) struct SingleCaseRunner {
    steps: Vec<TestStep>,
    score_pattern: Regex,
}

impl SingleCaseRunner {
    pub(crate) const fn new(steps: Vec<TestStep>, score_pattern: Regex) -> Self {
        Self {
            steps,
            score_pattern,
        }
    }

    pub(crate) fn run(&self, seed: u64) -> TestResult {
        let since = Instant::now();
        let result = self.run_steps(seed);
        let duration = since.elapsed();

        match result {
            Ok(outputs) => {
                let score = self.extract_score(&outputs);

                // 0点以下の場合はWrong Answerとして扱う
                let score = match score {
                    Some(score) => {
                        if score >= 0.0 {
                            Ok(score)
                        } else {
                            Err("Wrong Answer".to_string())
                        }
                    }
                    None => Err("Score not found".to_string()),
                };
                TestResult::new(seed, score, duration)
            }
            Err(e) => TestResult::new(seed, Err(e.to_string()), duration),
        }
    }

    fn run_steps(&self, seed: u64) -> Result<Vec<Vec<u8>>> {
        let mut outputs = vec![];

        for step in self.steps.iter() {
            // Set up the command
            let mut cmd = std::process::Command::new(&step.program);
            cmd.args(step.args.iter().map(|s| Self::replace_placeholder(s, seed)));

            if let Some(dir) = &step.current_dir {
                let dir = Self::replace_placeholder(dir, seed);
                cmd.current_dir(dir);
            }

            if let Some(stdin) = &step.stdin {
                let stdin = Self::replace_placeholder(stdin, seed);
                cmd.stdin(std::fs::File::open(stdin)?);
            }

            // Run the command
            let output = cmd.output()?;

            // Check the result
            anyhow::ensure!(output.status.success(), "Failed to run: {}", output.status);

            // Write the output
            if let Some(stdout) = &step.stdout {
                let stdout = Self::replace_placeholder(stdout, seed);
                std::fs::write(stdout, &output.stdout)?;
            }

            if let Some(stderr) = &step.stderr {
                let stderr = Self::replace_placeholder(stderr, seed);
                std::fs::write(stderr, &output.stderr)?;
            }

            // Save the output
            outputs.push(output.stdout);
            outputs.push(output.stderr);
        }

        Ok(outputs)
    }

    fn extract_score(&self, outputs: &[Vec<u8>]) -> Option<f64> {
        outputs
            .iter()
            .map(|s| {
                let s = String::from_utf8_lossy(s);
                eprintln!("{}", s);
                self.score_pattern
                    .captures_iter(&s)
                    .filter_map(|m| m.name("score").map(|s| s.as_str().parse().ok()).flatten())
                    .last()
            })
            .flatten()
            .last()
    }

    fn replace_placeholder(s: &str, seed: u64) -> String {
        s.replace("{SEED}", &seed.to_string())
            .replace("{SEED04}", &format!("{:04}", seed))
    }
}

#[cfg(test)]
mod test {
    use std::cell::LazyCell;

    use super::*;

    const REGEX: LazyCell<Regex> =
        LazyCell::new(|| Regex::new(r"^\s*Score\s*=\s*(?P<score>\d+)\s*$").unwrap());

    #[test]
    fn test_replace_placeholder() {
        assert_eq!(SingleCaseRunner::replace_placeholder("foo", 42), "foo");
        assert_eq!(SingleCaseRunner::replace_placeholder("{SEED}", 42), "42");
        assert_eq!(
            SingleCaseRunner::replace_placeholder("{SEED04}", 42),
            "0042"
        );
    }

    #[test]
    fn run_test_ok() {
        let steps = vec![gen_teststep("echo", Some("Score = 1234"))];
        let runner = SingleCaseRunner::new(steps, REGEX.clone());
        let result = runner.run(42);
        assert_eq!(result.score, Ok(1234.0));
    }

    #[test]
    fn run_test_score_zero() {
        let steps = vec![gen_teststep("echo", Some("Score = 0"))];
        let runner = SingleCaseRunner::new(steps, REGEX.clone());
        let result = runner.run(42);

        // 0点以下はWrong Answerとして扱う
        assert!(result.score.is_err());
    }

    #[test]
    fn run_test_fail() {
        let steps = vec![gen_teststep("false", None)];
        let runner = SingleCaseRunner::new(steps, REGEX.clone());
        let result = runner.run(42);
        assert!(result.score.is_err());
    }

    #[test]
    fn run_test_invalid_output() {
        let steps = vec![gen_teststep("echo", Some("invalid_output"))];
        let runner = SingleCaseRunner::new(steps, REGEX.clone());
        let result = runner.run(42);
        assert!(result.score.is_err());
    }

    fn gen_teststep(program: &str, arg: Option<&str>) -> TestStep {
        let args = arg.iter().map(|s| s.to_string()).collect();
        TestStep::new(program.to_string(), args, None, None, None, None)
    }
}
