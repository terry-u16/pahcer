use super::io::{load_result_json, AllResultJson};
use crate::runner::io;
use crate::runner::single::Objective;
use crate::settings::Settings;
use anyhow::{ensure, Result};
use colored::Colorize as _;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU64;
use tabled::{
    settings::{object::Columns, Alignment, Style},
    Table, Tabled,
};

#[derive(Tabled)]
struct ResultTableRow {
    #[tabled(rename = "Time")]
    time: String,
    #[tabled(rename = "AC/All")]
    ac_total: String,
    #[tabled(rename = "Avg Score")]
    avg_score: String,
    #[tabled(rename = "Avg Rel.")]
    avg_relative: String,
    #[tabled(rename = "Max Time")]
    max_time: String,
    #[tabled(rename = "Tag")]
    tag: String,
    #[tabled(rename = "Comment")]
    comment: String,
}

/// 過去のテスト結果をリスト表示する関数
pub(super) fn list_past_results(settings: &Settings, limit: Option<usize>) -> Result<()> {
    // JSONファイルから結果を読み込む
    let results = load_results(settings, limit)?;

    // 絶対ベストスコア
    let best_avg_absolute_score = calculate_best_avg_absolute_score(settings, &results);

    // 相対ベストスコア
    let best_scores = load_best_scores(settings);
    let best_avg_relative_score =
        calculate_best_avg_relative_score(settings, &results, &best_scores);

    // テーブル形式で結果を表示
    print_table(
        settings,
        results,
        best_avg_absolute_score,
        best_scores,
        best_avg_relative_score,
    );

    Ok(())
}

fn load_results(settings: &Settings, limit: Option<usize>) -> Result<Vec<AllResultJson>> {
    let json_dir = io::get_json_dir_path(&settings.test.out_dir);

    ensure!(
        json_dir.exists(),
        "No results found. JSON directory does not exist: {}",
        json_dir.display()
    );

    let mut json_files = vec![];

    for entry in fs::read_dir(&json_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with("result_") && file_name.ends_with(".json") {
                json_files.push(path);
            }
        }
    }

    // ファイル名でソート（新しい順）
    json_files.sort_by(|a, b| {
        let name_a = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name_b = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name_b.cmp(name_a)
    });

    // 制限数まで読み込み（Noneの場合は制限なし）
    if let Some(limit_value) = limit {
        json_files.truncate(limit_value);
    }

    // ファイルを読み込み
    let results = json_files
        .iter()
        .filter_map(|file| match load_result_json(file) {
            Ok(result) => Some(result),
            Err(e) => {
                eprintln!("Failed to load JSON file {}: {}", file.display(), e);
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(results)
}

fn calculate_best_avg_absolute_score(settings: &Settings, results: &[AllResultJson]) -> f64 {
    let best_avg_absolute_score = results
        .iter()
        .map(|result| {
            if result.case_count > 0 {
                result.total_score as f64 / result.case_count as f64
            } else {
                0.0
            }
        })
        .max_by(|a, b| match settings.problem.objective {
            Objective::Max => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            Objective::Min => b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal),
        })
        .unwrap_or(f64::NAN);

    best_avg_absolute_score
}

fn load_best_scores(settings: &Settings) -> HashMap<u64, NonZeroU64> {
    let best_score_path = io::get_best_score_path(&settings.test.out_dir);
    io::load_best_scores(&best_score_path).unwrap_or_else(|_| std::collections::HashMap::new())
}

fn calculate_best_avg_relative_score(
    settings: &Settings,
    results: &[AllResultJson],
    best_scores: &HashMap<u64, NonZeroU64>,
) -> f64 {
    let best_avg_relative_score = results
        .iter()
        .map(|result| calc_average_relative_score(result, best_scores, settings.problem.objective))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(f64::NAN);

    best_avg_relative_score
}

fn calc_average_relative_score(
    result: &AllResultJson,
    best_scores: &HashMap<u64, NonZeroU64>,
    objective: Objective,
) -> f64 {
    if result.case_count == 0 {
        return 0.0;
    }

    let mut total_relative_score = 0.0;

    for case in &result.cases {
        if case.score == 0 {
            continue; // スコアが0のケースは無視
        }

        let relative_score = match (best_scores.get(&case.seed).copied(), objective) {
            (Some(best), Objective::Max) => case.score as f64 / best.get() as f64 * 100.0,
            (Some(best), Objective::Min) => best.get() as f64 / case.score as f64 * 100.0,
            (None, _) => 100.0,
        };

        total_relative_score += relative_score;
    }

    total_relative_score / result.case_count as f64
}

fn print_table(
    settings: &Settings,
    results: Vec<AllResultJson>,
    best_avg_absolute_score: f64,
    best_scores: HashMap<u64, NonZeroU64>,
    best_avg_relative_score: f64,
) {
    // 結果を読み込んで表示
    let mut table_rows = vec![];

    for result in results {
        table_rows.push(convert_to_table_row(
            result,
            &best_scores,
            settings.problem.objective,
            best_avg_absolute_score,
            best_avg_relative_score,
        ));
    }

    // tabledを使ってテーブルを表示
    let mut table = Table::new(table_rows);
    table.with(Style::markdown());
    table.modify(Columns::new(1..=4), Alignment::right());
    println!("{table}");
}

fn convert_to_table_row(
    result: AllResultJson,
    best_scores: &HashMap<u64, NonZeroU64>,
    objective: Objective,
    best_avg_absolute_score: f64,
    best_avg_relative_score: f64,
) -> ResultTableRow {
    let time_str = result.start_time.format("%m/%d %H:%M:%S").to_string();
    let ac_count = result.case_count - result.wa_seeds.len();
    let ac_total = format!("{}/{}", ac_count, result.case_count);
    let ac_total = if result.wa_seeds.is_empty() {
        ac_total.green()
    } else {
        ac_total.yellow()
    }
    .to_string();
    let avg_score_f64 = if result.case_count > 0 {
        result.total_score as f64 / result.case_count as f64
    } else {
        0.0
    };
    let avg_score = format!("{avg_score_f64:.2}");
    let avg_score = if avg_score_f64 == best_avg_absolute_score {
        avg_score.bold().green().to_string()
    } else {
        avg_score
    };
    let avg_relative_f64 = calc_average_relative_score(&result, best_scores, objective);
    let avg_relative = format!("{avg_relative_f64:.3}");
    let avg_relative = if avg_relative_f64 == best_avg_relative_score {
        avg_relative.bold().green().to_string()
    } else {
        avg_relative
    };

    let max_time = format!("{:.0} ms", result.max_execution_time * 1e3);
    let tag_display = result
        .tag_name
        .as_deref()
        .unwrap_or("-")
        .replace("pahcer/", "");

    ResultTableRow {
        time: time_str,
        ac_total,
        avg_score,
        avg_relative,
        max_time,
        tag: tag_display,
        comment: result.comment,
    }
}
