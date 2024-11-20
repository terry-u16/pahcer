use crate::runner::{
    compilie::CompileStep,
    single::{Direction, TestStep},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Settings {
    general: General,
    problem: Problem,
    test: Test,
}

impl Settings {
    pub(crate) fn new(general: General, problem: Problem, test: Test) -> Self {
        Self {
            general,
            problem,
            test,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct General {
    version: String,
}

impl General {
    pub(crate) fn new(version: String) -> Self {
        Self { version }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Problem {
    problem_name: String,
    score_direction: Direction,
    score_regex: String,
}

impl Problem {
    pub(crate) fn new(
        problem_name: String,
        score_direction: Direction,
        score_regex: String,
    ) -> Self {
        Self {
            problem_name,
            score_direction,
            score_regex,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Test {
    start_seed: u64,
    end_seed: u64,
    threads: usize,
    compile_steps: Vec<CompileStep>,
    test_steps: Vec<TestStep>,
}

impl Test {
    pub(crate) fn new(
        start_seed: u64,
        end_seed: u64,
        threads: usize,
        compile_steps: Vec<CompileStep>,
        test_steps: Vec<TestStep>,
    ) -> Self {
        Self {
            start_seed,
            end_seed,
            threads,
            compile_steps,
            test_steps,
        }
    }
}

pub(crate) fn gen_setting_file() {
    let version = "0.1.0".to_string();
    let general = General::new(version);

    let problem_name = "ahc001".to_string();
    let problem = Problem::new(
        problem_name,
        Direction::Maximize,
        r"^\s*Score\s*=\s*(?P<score>\d+)\s*$".to_string(),
    );

    let compile_steps = vec![
        CompileStep::new(
            "cargo".to_string(),
            vec!["build".to_string(), "--release".to_string()],
            None,
        ),
        CompileStep::new(
            "mv".to_string(),
            vec![
                "../target/release/ahc001".to_string(),
                "./ahc001".to_string(),
            ],
            None,
        ),
    ];
    let test_steps = vec![
        TestStep::new(
            "./ahc001".to_string(),
            vec![],
            None,
            Some("./testcase/in/{SEED04}.txt".to_string()),
            Some("./testcase/out/{SEED04}.txt".to_string()),
            Some("./testcase/err/{SEED04}.txt".to_string()),
        ),
        TestStep::new(
            "./vis".to_string(),
            vec![
                "./testcase/in/{SEED04}.txt".to_string(),
                "./testcase/out/{SEED04}.txt".to_string(),
            ],
            None,
            None,
            None,
            None,
        ),
    ];

    let test = Test::new(0, 100, 0, compile_steps, test_steps);

    let setting = Settings::new(general, problem, test);

    let setting_str = toml::to_string_pretty(&setting).unwrap();
    std::fs::write("pahcer_config.toml", setting_str).unwrap();
}

pub(crate) fn load_setting_file() -> Result<Settings> {
    let setting_str = std::fs::read_to_string("pahcer_config.toml")?;
    let setting = toml::from_str(&setting_str)?;
    Ok(setting)
}
