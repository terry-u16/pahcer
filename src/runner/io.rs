use crate::util::format_float_with_commas;

use super::{
    multi::{self, TestStats},
    Settings,
};
use anyhow::{Context as _, Result};
use chrono::{DateTime, Local};
use num_format::{Locale, ToFormattedString as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    num::{NonZeroU64, NonZeroUsize},
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
        .map(|(key, value)| (format!("{key:04}"), value.get()))
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
    tag_name: &Option<String>,
) -> Result<()> {
    let comment = match tag_name {
        Some(tag_name) => format!("({tag_name}) {comment}"),
        None => comment.to_string(),
    };

    let mut writer = match OpenOptions::new().append(true).open(&path) {
        Ok(file) => BufWriter::new(file),
        Err(_) => {
            create_parent_dir(&path)?;
            let mut writer = BufWriter::new(File::create(path)?);
            save_summary_header(&mut writer)?;
            writer
        }
    };

    save_summary_log_inner(&mut writer, stats, &comment)?;

    Ok(())
}

fn save_summary_header(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "Time                      | Cases | Total Score      | Avg. Score       | Total log10  | Avg. log10  | Comment"
    )?;
    writeln!(
        writer,
        "--------------------------|------:|-----------------:|-----------------:|-------------:|------------:|----------------------"
    )?;

    Ok(())
}

fn save_summary_log_inner(
    writer: &mut impl Write,
    stats: &multi::TestStats,
    comment: &str,
) -> Result<()> {
    let nonzero2 = NonZeroUsize::new(2).unwrap();
    let nonzero5 = NonZeroUsize::new(5).unwrap();

    let start_time = stats
        .start_time
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let case_count = stats.results.len().to_formatted_string(&Locale::en);
    let score = stats.score_sum.to_formatted_string(&Locale::en);
    let average_score = format_float_with_commas(
        stats.score_sum as f64 / stats.results.len() as f64,
        nonzero2,
    );

    let score_log10 = format_float_with_commas(stats.score_sum_log10, nonzero5);
    let average_score_log10 =
        format_float_with_commas(stats.score_sum_log10 / stats.results.len() as f64, nonzero5);

    writeln!(
        writer,
        "{start_time} | {case_count:>5} | {score:>16} | {average_score:>16} | {score_log10:>12} | {average_score_log10:>11} | {comment}"
    )?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AllResultJson {
    pub(super) start_time: DateTime<Local>,
    pub(super) case_count: usize,
    pub(super) total_score: u64,
    pub(super) total_score_log10: f64,
    pub(super) total_relative_score: f64,
    pub(super) max_execution_time: f64,
    pub(super) comment: String,
    pub(super) tag_name: Option<String>,
    pub(super) wa_seeds: Vec<u64>,
    pub(super) cases: Vec<CaseResultJson>,
}

impl AllResultJson {
    fn new(stats: &TestStats, comment: &str, tag_name: &Option<String>) -> Self {
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
            comment: comment.to_string(),
            wa_seeds,
            cases,
            tag_name: tag_name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CaseResultJson {
    pub(super) seed: u64,
    pub(super) score: u64,
    pub(super) relative_score: f64,
    pub(super) execution_time: f64,
    pub(super) error_message: String,
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

pub(super) fn get_json_dir_path(dir_path: impl AsRef<OsStr>) -> PathBuf {
    Path::new(&dir_path).join("json")
}

pub(super) fn get_json_log_path(dir_path: impl AsRef<OsStr>, stats: &TestStats) -> PathBuf {
    let file_name = format!("result_{}.json", stats.start_time.format("%Y%m%d_%H%M%S"));
    get_json_dir_path(dir_path).join(file_name)
}

pub(super) fn save_json_log(
    path: impl AsRef<Path>,
    stats: &TestStats,
    comment: &str,
    tag_name: &Option<String>,
) -> Result<()> {
    create_parent_dir(&path)?;
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let json = AllResultJson::new(stats, comment, tag_name);
    serde_json::to_writer_pretty(writer, &json)?;

    Ok(())
}

fn create_parent_dir(path: impl AsRef<Path>) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

pub(super) fn load_result_json(path: &Path) -> Result<AllResultJson> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let result: AllResultJson = serde_json::from_reader(reader)?;
    Ok(result)
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
"Time                      | Cases | Total Score      | Avg. Score       | Total log10  | Avg. log10  | Comment
--------------------------|------:|-----------------:|-----------------:|-------------:|------------:|----------------------
{} |     2 |           11,000 |         5,500.00 |      7.00000 |     3.50000 | hoge
", start_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));

        let actual = String::from_utf8(buf).unwrap();

        eprintln!("[Expected]");
        eprintln!("{expected}");

        eprintln!("[Actual]");
        eprintln!("{actual}");

        assert_eq!(actual, expected);

        Ok(())
    }
}
