pub(crate) mod compilie;
pub(crate) mod multi;
pub(crate) mod single;

use crate::settings::{Settings, SETTING_FILE_PATH};
use anyhow::{ensure, Result};
use clap::Args;
use compilie::compile;
use rand::prelude::*;
use regex::Regex;

#[derive(Debug, Clone, Args)]
pub(crate) struct RunArgs {
    /// Shuffle the test cases
    #[clap(short = 's', long = "shuffle")]
    shuffle: bool,
}

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let settings = load_setting_file()?;
    compile(&settings.test.compile_steps)?;

    let single_runner = single::SingleCaseRunner::new(
        settings.test.test_steps.clone(),
        Regex::new(&settings.problem.score_regex)?,
    );

    let seed_range = settings.test.start_seed..settings.test.end_seed;
    ensure!(!seed_range.is_empty(), "Seed range is empty");

    // TODO: reference_scoreの読み込み
    let mut test_cases = seed_range
        .map(|seed| single::TestCase::new(seed, None, settings.problem.objective))
        .collect::<Vec<_>>();

    if args.shuffle {
        test_cases.shuffle(&mut rand::thread_rng());
    }

    let mut runner = multi::MultiCaseRunner::new(single_runner, test_cases, settings.test.threads);
    runner.run();

    Ok(())
}

fn load_setting_file() -> Result<Settings> {
    let settings_str = std::fs::read_to_string(SETTING_FILE_PATH)?;
    let settings = toml::from_str(&settings_str)?;
    Ok(settings)
}
