use super::{
    multi::{self, TestStats},
    Settings,
};
use anyhow::{Context as _, Result};
use chrono::{DateTime, Local};
use num_format::{Locale, ToFormattedString as _};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

const BEST_SCORE_FILE: &str = "best_scores.json";
const SUMMARY_SCORE_FILE: &str = "summary.md";

pub(super) fn get_best_score_path(dir_path: impl AsRef<OsStr>) -> PathBuf {
    Path::new(&dir_path).join(Path::new(BEST_SCORE_FILE))
}

pub(super) fn load_setting_file(path: impl AsRef<OsStr>) -> Result<Settings> {
    let settings_str = std::fs::read_to_string(Path::new(&path))?;
    let settings = toml::from_str(&settings_str)?;
    Ok(settings)
}

pub(super) fn load_best_scores(path: impl AsRef<Path>) -> Result<HashMap<u64, NonZeroU64>> {
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

pub(super) fn save_best_scores(
    path: impl AsRef<Path>,
    best_scores: HashMap<u64, NonZeroU64>,
) -> Result<()> {
    let json_map: BTreeMap<String, u64> = best_scores
        .into_iter()
        .map(|(key, value)| (format!("{:04}", key), value.get()))
        .collect();

    create_parent_dir(&path)?;

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_map)?;

    Ok(())
}

pub(super) fn get_summary_score_path(dir_path: impl AsRef<OsStr>) -> PathBuf {
    Path::new(&dir_path).join(Path::new(SUMMARY_SCORE_FILE))
}

pub(super) fn save_summary_log(
    path: impl AsRef<Path>,
    stats: &multi::TestStats,
    comment: &str,
) -> Result<()> {
    let mut writer = match OpenOptions::new().write(true).append(true).open(&path) {
        Ok(file) => BufWriter::new(file),
        Err(_) => {
            create_parent_dir(&path)?;
            let mut writer = BufWriter::new(File::create(path)?);
            save_summary_header(&mut writer)?;
            writer
        }
    };

    save_summary_log_inner(&mut writer, stats, comment)?;

    Ok(())
}

fn save_summary_header(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "Time                      | Cases  | Total Score       | Average Score | Total log10 | Average log10 | Comment"
    )?;
    writeln!(
        writer,
        "--------------------------|-------:|------------------:|--------------:|------------:|--------------:|----------------------"
    )?;

    Ok(())
}

fn save_summary_log_inner(
    writer: &mut impl Write,
    stats: &multi::TestStats,
    comment: &str,
) -> Result<()> {
    let start_time = stats
        .start_time
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let case_count = stats.results.len().to_formatted_string(&Locale::en);
    let score = stats.score_sum.to_formatted_string(&Locale::en);
    let average_score = ((stats.score_sum as f64 / stats.results.len() as f64).round() as u64)
        .to_formatted_string(&Locale::en);

    let score_log10 = stats.score_sum_log10;
    let average_score_log10 = stats.score_sum_log10 as f64 / stats.results.len() as f64;

    writeln!(
        writer,
        "{} | {:>6} | {:>17} | {:>13} | {:>11.3} | {:>13.3} | {}",
        start_time, case_count, score, average_score, score_log10, average_score_log10, comment
    )?;

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct AllResultJson<'a> {
    start_time: DateTime<Local>,
    case_count: usize,
    total_score: u64,
    total_score_log10: f64,
    total_relative_score: f64,
    max_execution_time: f64,
    comment: &'a str,
    wa_seeds: Vec<u64>,
    cases: Vec<CaseResultJson>,
}

impl<'a> AllResultJson<'a> {
    fn new(stats: &TestStats, comment: &'a str) -> Self {
        let cases = stats
            .results
            .iter()
            .map(|r| {
                let score = match r.score() {
                    &Ok(score) => score.get(),
                    Err(_) => 0,
                };
                let error_message = r
                    .score()
                    .as_ref()
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_default();

                CaseResultJson::new(
                    r.test_case().seed(),
                    score,
                    *r.relative_score().as_ref().unwrap_or(&0.0),
                    r.execution_time().as_secs_f64(),
                    error_message,
                )
            })
            .collect();
        let wa_seeds = stats
            .results
            .iter()
            .filter_map(|r| r.score().as_ref().err().map(|_| r.test_case().seed()))
            .collect();
        let max_execution_time = stats
            .results
            .iter()
            .map(|r| r.execution_time().as_secs_f64())
            .fold(0.0, f64::max);

        Self {
            start_time: stats.start_time,
            case_count: stats.results.len(),
            total_score: stats.score_sum,
            total_score_log10: stats.score_sum_log10,
            total_relative_score: stats.relative_score_sum,
            max_execution_time,
            comment,
            wa_seeds,
            cases,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct CaseResultJson {
    seed: u64,
    score: u64,
    relative_score: f64,
    execution_time: f64,
    error_message: String,
}

impl CaseResultJson {
    fn new(
        seed: u64,
        score: u64,
        relative_score: f64,
        execution_time: f64,
        error_message: String,
    ) -> Self {
        Self {
            seed,
            score,
            relative_score,
            execution_time,
            error_message,
        }
    }
}

pub(super) fn get_json_log_path(dir_path: impl AsRef<OsStr>, stats: &TestStats) -> PathBuf {
    let file_name = format!("result_{}.json", stats.start_time.format("%Y%m%d_%H%M%S"));
    Path::new(&dir_path).join("json").join(file_name)
}

pub(super) fn save_json_log(
    path: impl AsRef<Path>,
    stats: &TestStats,
    comment: &str,
) -> Result<()> {
    create_parent_dir(&path)?;
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let json = AllResultJson::new(stats, comment);
    serde_json::to_writer_pretty(writer, &json)?;

    Ok(())
}

fn create_parent_dir(path: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runner::single::{Objective, TestCase, TestResult};
    use chrono::DateTime;
    use std::{num::NonZero, time::Duration};

    #[test]
    fn save_summary_log_no_file() -> Result<()> {
        let mut buf = vec![];
        let start_time = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .into();

        let stats = multi::TestStats::new(
            vec![
                TestResult::new(
                    TestCase::new(0, None, Objective::Max),
                    Ok(NonZero::new(1000).unwrap()),
                    Duration::from_millis(1000),
                ),
                TestResult::new(
                    TestCase::new(1, None, Objective::Max),
                    Ok(NonZero::new(10000).unwrap()),
                    Duration::from_millis(100),
                ),
            ],
            start_time,
        );

        save_summary_header(&mut buf)?;
        save_summary_log_inner(&mut buf, &stats, "hoge")?;

        let expected = format!(
"Time                      | Cases  | Total Score       | Average Score | Total log10 | Average log10 | Comment
--------------------------|-------:|------------------:|--------------:|------------:|--------------:|----------------------
{} |      2 |            11,000 |         5,500 |       7.000 |         3.500 | hoge
", start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));

        let actual = String::from_utf8(buf).unwrap();

        eprintln!("[Expected]");
        eprintln!("{}", expected);

        eprintln!("[Actual]");
        eprintln!("{}", actual);

        assert_eq!(actual, expected);

        Ok(())
    }
}
