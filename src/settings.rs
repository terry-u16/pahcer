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
    Go,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Settings {
    pub(crate) general: General,
    pub(crate) problem: Problem,
    pub(crate) test: Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct General {
    pub(crate) version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Problem {
    pub(crate) problem_name: String,
    pub(crate) objective: Objective,
    pub(crate) score_regex: String,
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

pub(crate) fn gen_setting_file(args: &InitArgs) -> Result<()> {
    let mut writer = BufWriter::new(std::fs::File::create_new(SETTING_FILE_PATH).context(
        "Failed to create the setting file. Ensure that ./pahcer_config.toml does not exist.",
    )?);

    let mut settings = include_str!("./settings/template.toml").to_string();
    settings.push_str("\n");

    let run_steps = get_run_step_settings(args);
    settings += run_steps;
    settings = settings.replace("{VERSION}", env!("CARGO_PKG_VERSION"));
    settings = settings.replace("{PROBLEM_NAME}", &args.problem_name);
    settings = settings.replace("{OBJECTIVE}", &format!("{}", args.objective));

    let out_dir = "./pahcer";
    writeln!(writer, "{}", settings)?;

    gen_gitignore(out_dir)?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn get_run_step_settings(args: &InitArgs) -> &str {
    match (args.langage, args.is_interactive) {
        (Lang::Rust, true) => include_str!("./settings/linux/rust_interactive.toml"),
        (Lang::Rust, false) => include_str!("./settings/linux/rust.toml"),
        (Lang::Cpp, true) => include_str!("./settings/linux/cpp_interactive.toml"),
        (Lang::Cpp, false) => include_str!("./settings/linux/cpp.toml"),
        (Lang::Python, true) => include_str!("./settings/linux/python_interactive.toml"),
        (Lang::Python, false) => include_str!("./settings/linux/python.toml"),
        (Lang::Go, true) => include_str!("./settings/linux/go_interactive.toml"),
        (Lang::Go, false) => include_str!("./settings/linux/go.toml"),
    }
}

#[cfg(target_os = "macos")]
fn get_run_step_settings(args: &InitArgs) -> &str {
    match (args.langage, args.is_interactive) {
        (Lang::Rust, true) => include_str!("./settings/macos/rust_interactive.toml"),
        (Lang::Rust, false) => include_str!("./settings/macos/rust.toml"),
        (Lang::Cpp, true) => include_str!("./settings/macos/cpp_interactive.toml"),
        (Lang::Cpp, false) => include_str!("./settings/macos/cpp.toml"),
        (Lang::Python, true) => include_str!("./settings/macos/python_interactive.toml"),
        (Lang::Python, false) => include_str!("./settings/macos/python.toml"),
        (Lang::Go, true) => include_str!("./settings/macos/go_interactive.toml"),
        (Lang::Go, false) => include_str!("./settings/macos/go.toml"),
    }
}

#[cfg(target_os = "windows")]
fn get_run_step_settings(args: &InitArgs) -> &str {
    match (args.langage, args.is_interactive) {
        (Lang::Rust, true) => include_str!("./settings/windows/rust_interactive.toml"),
        (Lang::Rust, false) => include_str!("./settings/windows/rust.toml"),
        (Lang::Cpp, true) => include_str!("./settings/windows/cpp_interactive.toml"),
        (Lang::Cpp, false) => include_str!("./settings/windows/cpp.toml"),
        (Lang::Python, true) => include_str!("./settings/windows/python_interactive.toml"),
        (Lang::Python, false) => include_str!("./settings/windows/python.toml"),
        (Lang::Go, true) => include_str!("./settings/windows/go_interactive.toml"),
        (Lang::Go, false) => include_str!("./settings/windows/go.toml"),
    }
}

fn gen_gitignore(dir: impl AsRef<OsStr>) -> Result<()> {
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir)?;

    let path = dir.join(".gitignore");

    if path.exists() {
        return Ok(());
    }

    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(writer, "*").context("Failed to write to .gitignore")?;

    Ok(())
}
