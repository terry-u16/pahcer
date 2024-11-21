pub(crate) mod runner;
pub(crate) mod settings;

use anyhow::Result;
use clap::{Parser, Subcommand};
use settings::gen_setting_file;

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
    Run,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    dbg!(&args);

    match args.command {
        Command::Init(args) => {
            gen_setting_file(&args);
        }
        Command::Run => {
            unimplemented!();
        }
    }

    let settings = settings::load_setting_file()?;
    dbg!(settings);
    Ok(())
}
