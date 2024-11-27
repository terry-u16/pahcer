use crate::runner::{
    compilie::CompileStep,
    single::{Objective, TestStep},
};
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Write as _},
    path::Path,
};

pub(crate) const SETTING_FILE_PATH: &str = "pahcer_config.toml";

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
    pub(crate) general: General,
    pub(crate) problem: Problem,
    pub(crate) test: Test,
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
    pub(crate) version: String,
}

impl General {
    pub(crate) fn new(version: String) -> Self {
        Self { version }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Problem {
    pub(crate) problem_name: String,
    pub(crate) objective: Objective,
    pub(crate) score_regex: String,
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
    pub(crate) start_seed: u64,
    pub(crate) end_seed: u64,
    pub(crate) threads: usize,
    pub(crate) out_dir: String,
    pub(crate) compile_steps: Vec<CompileStep>,
    pub(crate) test_steps: Vec<TestStep>,
}

impl Test {
    pub(crate) fn new(
        start_seed: u64,
        end_seed: u64,
        threads: usize,
        out_dir: String,
        compile_steps: Vec<CompileStep>,
        test_steps: Vec<TestStep>,
    ) -> Self {
        Self {
            start_seed,
            end_seed,
            threads,
            out_dir,
            compile_steps,
            test_steps,
        }
    }
}

pub(crate) fn gen_setting_file(args: &InitArgs) -> Result<()> {
    let mut writer = BufWriter::new(std::fs::File::create_new(SETTING_FILE_PATH).context(
        "Failed to create the setting file. Ensure that ./pahcer_config.toml does not exist.",
    )?);

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
        r"(?m)^\s*Score\s*=\s*(?P<score>\d+)\s*$".to_string(),
    );

    let compile_steps = lang.compile_command();
    let test_steps = gen_run_steps(lang, args.is_interactive);

    let out_dir = "./pahcer";
    let test = Test::new(0, 100, 0, out_dir.to_string(), compile_steps, test_steps);

    let setting = Settings::new(general, problem, test);

    let setting_str = toml::to_string_pretty(&setting)?;
    writeln!(writer, "{}", setting_str)?;

    gen_gitignore(out_dir)?;

    Ok(())
}

fn gen_run_steps(lang: Box<dyn Language>, is_interactive: bool) -> Vec<TestStep> {
    let (test_command, test_args) = lang.test_command(is_interactive);

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
            Some("./tools".to_string()),
            Some("./tools/in/{SEED04}.txt".to_string()),
            Some("./tools/out/{SEED04}.txt".to_string()),
            Some("./tools/err/{SEED04}.txt".to_string()),
            true,
        )]
    } else {
        vec![
            TestStep::new(
                test_command,
                test_args,
                None,
                Some("./tools/in/{SEED04}.txt".to_string()),
                Some("./tools/out/{SEED04}.txt".to_string()),
                Some("./tools/err/{SEED04}.txt".to_string()),
                true,
            ),
            TestStep::new(
                "cargo".to_string(),
                vec![
                    "run".to_string(),
                    "--bin".to_string(),
                    "vis".to_string(),
                    "--release".to_string(),
                    "./in/{SEED04}.txt".to_string(),
                    "./out/{SEED04}.txt".to_string(),
                ],
                Some("./tools".to_string()),
                None,
                None,
                None,
                false,
            ),
        ]
    }
}

fn gen_gitignore(dir: impl AsRef<OsStr>) -> Result<()> {
    let dir = Path::new(&dir);
    std::fs::create_dir_all(&dir)?;

    let path = dir.join(".gitignore");

    if path.exists() {
        return Ok(());
    }

    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(writer, "*").context("Failed to write to .gitignore")?;

    Ok(())
}

trait Language {
    fn compile_command(&self) -> Vec<CompileStep>;
    fn test_command(&self, is_interactive: bool) -> (String, Vec<String>);
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
                "rm".to_string(),
                vec![format!("./{}", self.problem_name), "-f".to_string()],
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

    fn test_command(&self, is_interactive: bool) -> (String, Vec<String>) {
        if is_interactive {
            (format!("../{}", self.problem_name), vec![])
        } else {
            (format!("./{}", self.problem_name), vec![])
        }
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

    fn test_command(&self, is_interactive: bool) -> (String, Vec<String>) {
        if is_interactive {
            return ("../a.out".to_string(), vec![]);
        } else {
            return ("./a.out".to_string(), vec![]);
        }
    }
}

struct Python;

impl Language for Python {
    fn compile_command(&self) -> Vec<CompileStep> {
        vec![]
    }

    fn test_command(&self, is_interactive: bool) -> (String, Vec<String>) {
        if is_interactive {
            ("python".to_string(), vec!["../main.py".to_string()])
        } else {
            ("python".to_string(), vec!["./main.py".to_string()])
        }
    }
}
