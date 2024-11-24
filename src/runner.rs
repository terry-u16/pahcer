pub(crate) mod compilie;
mod io;
mod multi;
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
    /// Comment for the run
    #[clap(short = 'c', long = "comment", default_value = "")]
    comment: String,
}

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let settings = io::load_setting_file()?;
    let best_score_path = io::get_best_score_path(&settings.test.out_dir);
    let mut best_scores = io::load_best_scores(&best_score_path)?;
    compile(&settings.test.compile_steps)?;

    let single_runner = single::SingleCaseRunner::new(
        settings.test.test_steps.clone(),
        Regex::new(&settings.problem.score_regex)?,
    );

    let seed_range = settings.test.start_seed..settings.test.end_seed;
    ensure!(
        !seed_range.is_empty(),
        "Seed range [{}, {}) is empty. Ensure that start_seed < end_seed (note that end_seed is exclusive).",
        seed_range.start,
        seed_range.end
    );

    let mut test_cases = seed_range
        .map(|seed| {
            single::TestCase::new(
                seed,
                best_scores.get(&seed).copied(),
                settings.problem.objective,
            )
        })
        .collect::<Vec<_>>();

    if args.shuffle {
        test_cases.shuffle(&mut rand::thread_rng());
    }

    let mut runner = multi::MultiCaseRunner::new(single_runner, test_cases, settings.test.threads);
    let stats = runner.run();

    for result in stats.results.iter() {
        let Some(score) = result.score().as_ref().ok().copied() else {
            continue;
        };

        if result.test_case().is_best(Some(score)) {
            best_scores.insert(result.test_case().seed(), score);
        }
    }

    let summary_file_path = io::get_summary_score_path(&settings.test.out_dir);
    io::save_summary_log(&summary_file_path, &stats, &args.comment)?;
    io::save_best_scores(&best_score_path, best_scores)?;

    Ok(())
}
