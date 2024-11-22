use anyhow::{Context, Result};
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    num::NonZeroU64,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct TestCase {
    seed: u64,
    reference_score: Option<NonZeroU64>,
    objective: Objective,
}

impl TestCase {
    pub(crate) const fn new(
        seed: u64,
        reference_score: Option<NonZeroU64>,
        objective: Objective,
    ) -> Self {
        Self {
            seed,
            reference_score,
            objective,
        }
    }

    pub(crate) fn calc_relative_score(&self, new_score: NonZeroU64) -> f64 {
        let Some(old_score) = self.reference_score else {
            return 100.0;
        };

        match self.objective {
            Objective::Max => new_score.get() as f64 / old_score.get() as f64 * 100.0,
            Objective::Min => old_score.get() as f64 / new_score.get() as f64 * 100.0,
        }
    }

    pub(crate) fn is_best(&self, new_score: Option<NonZeroU64>) -> bool {
        let Some(new_score) = new_score else {
            return false;
        };

        let Some(old_score) = self.reference_score.map(|s| s.get()) else {
            return true;
        };

        match self.objective {
            Objective::Max => new_score.get() >= old_score,
            Objective::Min => new_score.get() <= old_score,
        }
    }

    pub(crate) const fn seed(&self) -> u64 {
        self.seed
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TestResult {
    test_case: TestCase,
    score: Result<NonZeroU64, String>,
    relative_score: Result<f64, String>,
    duration: Duration,
}

impl TestResult {
    pub(crate) fn new(
        test_case: TestCase,
        score: Result<NonZeroU64, String>,
        duration: Duration,
    ) -> Self {
        let relative_score = score.clone().map(|s| test_case.calc_relative_score(s));

        Self {
            test_case,
            score,
            relative_score,
            duration,
        }
    }

    pub(crate) const fn test_case(&self) -> &TestCase {
        &self.test_case
    }

    pub(crate) fn score(&self) -> &Result<NonZeroU64, String> {
        &self.score
    }

    /// Returns the score in log10 scale.
    pub(crate) fn score_log10(&self) -> Result<f64, &String> {
        self.score.as_ref().map(|s| (s.get() as f64).log10())
    }

    pub(crate) fn relative_score(&self) -> &Result<f64, String> {
        &self.relative_score
    }

    pub(crate) const fn duration(&self) -> Duration {
        self.duration
    }
}

/// The direction to optimize the score
#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize)]
pub(crate) enum Objective {
    /// Maximize the score
    Max,
    /// Minimize the score
    Min,
}

#[derive(Debug, Clone)]
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

    pub(crate) fn run(&self, test_case: TestCase) -> TestResult {
        let since = Instant::now();
        let result = self.run_steps(test_case.seed);
        let duration = since.elapsed();

        match result {
            Ok(outputs) => {
                let score = self.extract_score(&outputs);

                // 0点以下の場合はWrong Answerとして扱う
                let score = match score {
                    Some(score) => match NonZeroU64::new(score as u64) {
                        Some(score) => Ok(score),
                        None => Err("Wrong Answer".to_string()),
                    },
                    None => Err("Score not found".to_string()),
                };
                TestResult::new(test_case, score, duration)
            }
            Err(e) => TestResult::new(test_case, Err(format!("{:#}", e)), duration),
        }
    }

    fn run_steps(&self, seed: u64) -> Result<Vec<Vec<u8>>> {
        let mut outputs = vec![];

        for step in self.steps.iter() {
            let cmd = Self::build_cmd(step, seed)?;
            Self::run_cmd(cmd, step, seed, &mut outputs)?;
        }

        Ok(outputs)
    }

    fn build_cmd(step: &TestStep, seed: u64) -> Result<std::process::Command, anyhow::Error> {
        let mut cmd = std::process::Command::new(&step.program);
        cmd.args(step.args.iter().map(|s| Self::replace_placeholder(s, seed)));

        if let Some(dir) = &step.current_dir {
            let dir = Self::replace_placeholder(dir, seed);
            cmd.current_dir(dir);
        }

        if let Some(stdin) = &step.stdin {
            let stdin = Self::replace_placeholder(stdin, seed);
            let file = std::fs::File::open(&stdin)
                .with_context(|| format!("Failed to open input file ({})", &stdin))?;
            cmd.stdin(file);
        }

        Ok(cmd)
    }

    fn run_cmd(
        mut cmd: std::process::Command,
        step: &TestStep,
        seed: u64,
        outputs: &mut Vec<Vec<u8>>,
    ) -> Result<(), anyhow::Error> {
        let output = cmd
            .output()
            .with_context(|| format!("Failed to run. command: {:?}", cmd))?;
        anyhow::ensure!(
            output.status.success(),
            "Failed to run ({}). command: {:?}",
            output.status,
            cmd
        );

        if let Some(stdout) = &step.stdout {
            let stdout = Self::replace_placeholder(stdout, seed);
            Self::write_output(Path::new(&stdout), &output.stdout)
                .with_context(|| format!("Failed to write stdout to {stdout}"))?;
        }

        if let Some(stderr) = &step.stderr {
            let stderr = Self::replace_placeholder(stderr, seed);
            Self::write_output(Path::new(&stderr), &output.stderr)
                .with_context(|| format!("Failed to write stderr to {stderr}"))?;
        }

        outputs.push(output.stdout);
        outputs.push(output.stderr);

        Ok(())
    }

    fn create_parent_dir_all(path: impl AsRef<OsStr>) -> Result<()> {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        Ok(())
    }

    fn write_output(path: impl AsRef<OsStr>, contents: &[u8]) -> Result<()> {
        let path = Path::new(&path);
        Self::create_parent_dir_all(path)?;
        std::fs::write(&path, contents)?;

        Ok(())
    }

    fn extract_score(&self, outputs: &[Vec<u8>]) -> Option<f64> {
        outputs
            .iter()
            .map(|s| {
                let s = String::from_utf8_lossy(s);
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
    use super::*;
    use std::cell::LazyCell;

    const TEST_CASE: TestCase = TestCase::new(42, None, Objective::Max);
    const SCORE_REGEX: LazyCell<Regex> =
        LazyCell::new(|| Regex::new(r"^\s*Score\s*=\s*(?P<score>\d+)\s*$").unwrap());

    #[test]
    fn test_calc_relative_score() {
        let non_zero_100 = NonZeroU64::new(100).unwrap();
        let non_zero_200 = NonZeroU64::new(200).unwrap();

        let test_case = TestCase::new(0, Some(NonZeroU64::new(100).unwrap()), Objective::Max);
        assert_eq!(test_case.calc_relative_score(non_zero_100), 100.0);
        assert_eq!(test_case.calc_relative_score(non_zero_200), 200.0);

        let test_case = TestCase::new(0, Some(NonZeroU64::new(100).unwrap()), Objective::Min);
        assert_eq!(test_case.calc_relative_score(non_zero_100), 100.0);
        assert_eq!(test_case.calc_relative_score(non_zero_200), 50.0);
    }

    #[test]
    fn test_is_best() {
        let non_zero_50 = NonZeroU64::new(50);
        let non_zero_100 = NonZeroU64::new(100);
        let non_zero_200 = NonZeroU64::new(200);

        let test_case = TestCase::new(0, non_zero_100, Objective::Max);
        assert!(!test_case.is_best(non_zero_50));
        assert!(test_case.is_best(non_zero_100));
        assert!(test_case.is_best(non_zero_200));

        let test_case = TestCase::new(0, non_zero_100, Objective::Min);
        assert!(test_case.is_best(non_zero_50));
        assert!(test_case.is_best(non_zero_100));
        assert!(!test_case.is_best(non_zero_200));
    }

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
        let runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let result = runner.run(TEST_CASE);
        assert_eq!(result.score(), &Ok(NonZeroU64::new(1234).unwrap()));
    }

    #[test]
    fn run_test_score_zero() {
        let steps = vec![gen_teststep("echo", Some("Score = 0"))];
        let runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let result = runner.run(TEST_CASE);

        // 0点以下はWrong Answerとして扱う
        assert!(result.score.is_err());
    }

    #[test]
    fn run_test_fail() {
        let steps = vec![gen_teststep("false", None)];
        let runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let result = runner.run(TEST_CASE);
        assert!(result.score.is_err());
    }

    #[test]
    fn run_test_invalid_output() {
        let steps = vec![gen_teststep("echo", Some("invalid_output"))];
        let runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let result = runner.run(TEST_CASE);
        assert!(result.score.is_err());
    }

    fn gen_teststep(program: &str, arg: Option<&str>) -> TestStep {
        let args = arg.iter().map(|s| s.to_string()).collect();
        TestStep::new(program.to_string(), args, None, None, None, None)
    }
}
