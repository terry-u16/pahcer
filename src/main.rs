pub(crate) mod runner;
pub(crate) mod settings;
pub(crate) mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Debug, Clone, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Initialize the project
    Init(settings::InitArgs),
    /// Run tests
    Run(runner::RunArgs),
}

fn main() {
    let args = Cli::parse();

    if let Err(e) = run_command(args) {
        eprintln!("{}", format!("Error: {:?}", e).yellow().bold());
        std::process::exit(1);
    }
}

fn run_command(args: Cli) -> Result<(), anyhow::Error> {
    match args.command {
        Command::Init(args) => {
            settings::gen_setting_file(&args)?;
        }
        Command::Run(args) => {
            runner::run(args)?;
        }
    };
    Ok(())
}
