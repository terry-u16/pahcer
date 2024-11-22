use super::{Settings, SETTING_FILE_PATH};
use anyhow::{Context as _, Result};
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    fs::File,
    io::{BufReader, BufWriter},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

const BEST_SCORE_FILE: &str = "best_scores.json";

pub(super) fn get_best_score_path(dir_path: impl AsRef<OsStr>) -> PathBuf {
    Path::new(&dir_path).join(Path::new(BEST_SCORE_FILE))
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

    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &json_map)?;

    Ok(())
}
