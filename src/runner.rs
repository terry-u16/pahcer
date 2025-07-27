pub(crate) mod compilie;
mod io;
mod multi;
pub(crate) mod single;

use crate::{
    git,
    settings::{Settings, SETTING_FILE_PATH},
};
use anyhow::{ensure, Context, Result};
use clap::Args;
use compilie::compile;
use rand::prelude::*;
use regex::Regex;

#[derive(Debug, Clone, Args)]
pub(crate) struct RunArgs {
    /// Shuffle the test cases
    #[clap(long = "shuffle")]
    shuffle: bool,
    /// Comment for the run
    #[clap(short = 'c', long = "comment", default_value = "")]
    comment: String,
    /// Output the result in JSON format
    #[clap(short = 'j', long = "json")]
    json: bool,
    /// Tag for the commit
    #[clap(short = 't', long = "tag", num_args = 0..=1, default_missing_value = "")]
    tag: Option<String>,
    /// Path to the setting file
    #[clap(long = "setting-file", default_value = SETTING_FILE_PATH)]
    setting_file: String,
    /// Freeze the best score
    #[clap(long = "freeze-best-scores")]
    freeze_best_scores: bool,
    /// Do not output the result file
    #[clap(long = "no-result-file")]
    no_result_file: bool,
    /// Do not compile the code
    #[clap(long = "no-compile")]
    no_compile: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ListArgs {
    /// Number of results to display
    #[clap(short = 'n', long = "number", default_value = "10")]
    number: usize,
    /// Path to the setting file
    #[clap(long = "setting-file", default_value = SETTING_FILE_PATH)]
    setting_file: String,
}

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let settings = io::load_setting_file(&args.setting_file)
        .with_context(|| format!("Failed to load the setting file {}.", &args.setting_file))?;
    let best_score_path = io::get_best_score_path(&settings.test.out_dir);
    let mut best_scores = io::load_best_scores(&best_score_path)?;

    if !args.no_compile {
        compile(&settings.test.compile_steps)?;
    }

    let tag_name = match args.tag {
        Some(tag) => {
            let tag = if tag.is_empty() { None } else { Some(tag) };
            let tag = git::commit(tag).context("Failed to tag the current changes.")?;
            println!("Tagged: {tag}");
            Some(tag)
        }
        None => None,
    };

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
        test_cases.shuffle(&mut rand::rng());
    }

    let mut runner = if args.json {
        multi::MultiCaseRunner::new_json(single_runner, test_cases, settings.test.threads)
    } else {
        multi::MultiCaseRunner::new_console(single_runner, test_cases, settings.test.threads)
    };
    let stats = runner.run()?;

    for result in stats.results.iter() {
        let Some(score) = result.score().as_ref().ok().copied() else {
            continue;
        };

        if result.test_case().is_best(Some(score)) {
            best_scores.insert(result.test_case().seed(), score);
        }
    }

    if !args.freeze_best_scores {
        io::save_best_scores(&best_score_path, best_scores)?;
    }

    if !args.no_result_file {
        let summary_file_path = io::get_summary_score_path(&settings.test.out_dir);
        io::save_summary_log(&summary_file_path, &stats, &args.comment, &tag_name)?;
        let json_file_path = io::get_json_log_path(&settings.test.out_dir, &stats);
        io::save_json_log(&json_file_path, &stats, &args.comment, &tag_name)?;
    }

    Ok(())
}

pub(crate) fn list(args: ListArgs) -> Result<()> {
    let settings = io::load_setting_file(&args.setting_file)
        .with_context(|| format!("Failed to load the setting file {}.", &args.setting_file))?;

    io::list_past_results(&settings.test.out_dir, args.number)?;

    Ok(())
}
