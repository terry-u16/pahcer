use crate::runner::{
    compilie::CompileStep,
    single::{Objective, TestStep},
};
use anyhow::Result;
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Args)]
pub(crate) struct InitArgs {
    /// Name of the problem
    #[clap(short = 'p', long = "problem")]
    problem_name: String,

    /// Objective of the problem
    #[clap(short = 'o', long = "objective")]
    objective: Objective,

    /// Language of your code
    #[clap(short = 'l', long = "lang")]
    langage: Lang,

    /// Interactive problem or not
    #[clap(short = 'i', long = "interactive")]
    is_interactive: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Lang {
    Rust,
    Cpp,
    Python,
}

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
    objective: Objective,
    score_regex: String,
}

impl Problem {
    pub(crate) fn new(problem_name: String, objective: Objective, score_regex: String) -> Self {
        Self {
            problem_name,
            objective,
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

pub(crate) fn gen_setting_file(args: &InitArgs) {
    let version = "0.1.0".to_string();
    let general = General::new(version);

    let lang: Box<dyn Language> = match args.langage {
        Lang::Rust => Box::new(Rust::new(args.problem_name.clone())),
        Lang::Cpp => Box::new(Cpp),
        Lang::Python => Box::new(Python),
    };

    let problem_name = args.problem_name.clone();
    let problem = Problem::new(
        problem_name,
        args.objective,
        r"^\s*Score\s*=\s*(?P<score>\d+)\s*$".to_string(),
    );

    let compile_steps = lang.compile_command();
    let test_steps = gen_run_steps(lang, args.is_interactive);

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

fn gen_run_steps(lang: Box<dyn Language>, is_interactive: bool) -> Vec<TestStep> {
    let (test_command, test_args) = lang.test_command();

    if is_interactive {
        let mut args = vec![
            "run".to_string(),
            "--bin".to_string(),
            "tester".to_string(),
            "--release".to_string(),
        ];
        args.push(test_command);
        args.extend(test_args);

        vec![TestStep::new(
            "cargo".to_string(),
            args,
            None,
            Some("./testcase/in/{SEED04}.txt".to_string()),
            Some("./testcase/out/{SEED04}.txt".to_string()),
            Some("./testcase/err/{SEED04}.txt".to_string()),
        )]
    } else {
        vec![
            TestStep::new(
                test_command,
                test_args,
                None,
                Some("./testcase/in/{SEED04}.txt".to_string()),
                Some("./testcase/out/{SEED04}.txt".to_string()),
                Some("./testcase/err/{SEED04}.txt".to_string()),
            ),
            TestStep::new(
                "cargo".to_string(),
                vec![
                    "run".to_string(),
                    "--bin".to_string(),
                    "vis".to_string(),
                    "--release".to_string(),
                    "../testcase/in/{SEED04}.txt".to_string(),
                    "../testcase/out/{SEED04}.txt".to_string(),
                ],
                Some("./tools/".to_string()),
                None,
                None,
                None,
            ),
        ]
    }
}

trait Language {
    fn compile_command(&self) -> Vec<CompileStep>;
    fn test_command(&self) -> (String, Vec<String>);
}

struct Rust {
    problem_name: String,
}

impl Rust {
    fn new(problem_name: String) -> Self {
        Self { problem_name }
    }
}

impl Language for Rust {
    fn compile_command(&self) -> Vec<CompileStep> {
        vec![
            CompileStep::new(
                "cargo".to_string(),
                vec!["build".to_string(), "--release".to_string()],
                None,
            ),
            CompileStep::new(
                "mv".to_string(),
                vec![
                    format!("./target/release/{}", self.problem_name),
                    format!("./{}", self.problem_name),
                ],
                None,
            ),
        ]
    }

    fn test_command(&self) -> (String, Vec<String>) {
        (format!("./{}", self.problem_name), vec![])
    }
}

struct Cpp;

impl Language for Cpp {
    fn compile_command(&self) -> Vec<CompileStep> {
        vec![CompileStep::new(
            "g++".to_string(),
            vec![
                "-std=c++20".to_string(),
                "-O2".to_string(),
                "main.cpp".to_string(),
            ],
            None,
        )]
    }

    fn test_command(&self) -> (String, Vec<String>) {
        ("./a.out".to_string(), vec![])
    }
}

struct Python;

impl Language for Python {
    fn compile_command(&self) -> Vec<CompileStep> {
        vec![]
    }

    fn test_command(&self) -> (String, Vec<String>) {
        ("python".to_string(), vec!["main.py".to_string()])
    }
}
