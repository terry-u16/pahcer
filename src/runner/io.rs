use super::{multi, Settings, SETTING_FILE_PATH};
use anyhow::{Context as _, Result};
use num_format::{Locale, ToFormattedString as _};
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

pub(super) fn get_summary_score_path(dir_path: impl AsRef<OsStr>) -> PathBuf {
    Path::new(&dir_path).join(Path::new(SUMMARY_SCORE_FILE))
}

pub(super) fn load_setting_file() -> Result<Settings> {
    let settings_str = std::fs::read_to_string(SETTING_FILE_PATH)?;
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

pub(super) fn create_parent_dir(path: impl AsRef<Path>) -> Result<()> {
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
