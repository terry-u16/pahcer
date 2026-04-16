use super::comparative_score::ComparativeScoreCalculator;
use super::io::AllResultJson;
use crate::runner::io;
use crate::runner::single::Objective;
use crate::settings::Settings;
use anyhow::{ensure, Result};
use colored::Colorize as _;
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
pub(super) fn list_past_results(
    settings: &Settings,
    limit: Option<usize>,
    score_calculator: &dyn ComparativeScoreCalculator,
) -> Result<()> {
    // JSONファイルから結果を読み込む
    let results = io::load_result_jsons(&settings.test.out_dir, limit)?;
    ensure!(
        !results.is_empty(),
        "No results found. JSON directory does not exist or contains no result files: {}",
        io::get_json_dir_path(&settings.test.out_dir).display()
    );

    // 絶対ベストスコア
    let best_avg_absolute_score = calculate_best_avg_absolute_score(settings, &results);

    // 相対ベストスコア
    let best_avg_comparative_score =
        calculate_best_avg_comparative_score(&results, score_calculator);

    // テーブル形式で結果を表示
    print_table(
        results,
        score_calculator,
        best_avg_comparative_score,
        best_avg_absolute_score,
    );

    Ok(())
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

fn calculate_best_avg_comparative_score(
    results: &[AllResultJson],
    score_calculator: &dyn ComparativeScoreCalculator,
) -> f64 {
    let best_avg_comparative_score = results
        .iter()
        .map(|result| calculate_average_comparative_score(result, score_calculator))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(f64::NAN);

    best_avg_comparative_score
}

fn calculate_average_comparative_score(
    result: &AllResultJson,
    score_calculator: &dyn ComparativeScoreCalculator,
) -> f64 {
    if result.case_count == 0 {
        return 0.0;
    }

    let total_comparative_score = result
        .cases
        .iter()
        .filter_map(|case| {
            NonZeroU64::new(case.score).map(|score| score_calculator.calculate(case.seed, score))
        })
        .sum::<f64>();

    total_comparative_score / result.case_count as f64
}

fn print_table(
    results: Vec<AllResultJson>,
    score_calculator: &dyn ComparativeScoreCalculator,
    best_avg_comparative_score: f64,
    best_avg_absolute_score: f64,
) {
    // 結果を読み込んで表示
    let mut table_rows = vec![];

    for result in results {
        table_rows.push(convert_to_table_row(
            result,
            score_calculator,
            best_avg_absolute_score,
            best_avg_comparative_score,
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
    score_calculator: &dyn ComparativeScoreCalculator,
    best_avg_absolute_score: f64,
    best_avg_comparative_score: f64,
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
    let avg_comparative_score = calculate_average_comparative_score(&result, score_calculator);
    let avg_relative = format!("{avg_comparative_score:.3}");
    let avg_relative = if avg_comparative_score == best_avg_comparative_score {
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
