pub(crate) mod compilie;
pub(crate) mod multi;
pub(crate) mod single;

use crate::settings::{Settings, SETTING_FILE_PATH};
use anyhow::{ensure, Context, Result};
use clap::Args;
use compilie::compile;
use rand::prelude::*;
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufReader, BufWriter},
    num::NonZeroU64,
    path::Path,
};

const BEST_SCORE_FILE: &str = "best_scores.json";

#[derive(Debug, Clone, Args)]
pub(crate) struct RunArgs {
    /// Shuffle the test cases
    #[clap(short = 's', long = "shuffle")]
    shuffle: bool,
}

pub(crate) fn run(args: RunArgs) -> Result<()> {
    let settings = load_setting_file()?;
    let best_score_path = Path::new(&settings.test.out_dir).join(Path::new(BEST_SCORE_FILE));
    let mut best_scores = load_best_scores(&best_score_path)?;
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

    // TODO: reference_scoreの読み込み
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

    save_best_scores(&best_score_path, best_scores)?;

    Ok(())
}

fn load_setting_file() -> Result<Settings> {
    let settings_str = std::fs::read_to_string(SETTING_FILE_PATH)?;
    let settings = toml::from_str(&settings_str)?;
    Ok(settings)
}

fn load_best_scores(path: impl AsRef<Path>) -> Result<HashMap<u64, NonZeroU64>> {
    let Ok(file) = File::open(&path) else {
        return Ok(HashMap::new());
    };
    let reader = BufReader::new(file);
    let temp_map: HashMap<String, u64> =
        serde_json::from_reader(reader).context("Failed to parse json")?;

    let map = temp_map
        .into_iter()
        .flat_map(|(key, value)| {
            let key = key.parse::<u64>().ok();
            let value = NonZeroU64::new(value);
            match (key, value) {
                (Some(key), Some(value)) => Some((key, value)),
                (_, _) => None,
            }
        })
        .collect();

    Ok(map)
}

fn save_best_scores(path: impl AsRef<Path>, best_scores: HashMap<u64, NonZeroU64>) -> Result<()> {
    let json_map: BTreeMap<String, u64> = best_scores
        .into_iter()
        .map(|(key, value)| (format!("{:04}", key), value.get()))
        .collect();

    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_map)?;

    Ok(())
}
