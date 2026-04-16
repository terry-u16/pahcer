use super::{io::AllResultJson, single::Objective};
use std::{collections::HashMap, num::NonZeroU64, sync::Arc};

const PERFECT_RANK_SCORE: f64 = 100.0;

/// ケースごとの比較スコアを計算する抽象インターフェース。
///
/// `seed` とそのケースで得られた `score` を受け取り、表示・集計に使う比較スコアを返す。
/// 具体的な計算内容は、相対スコア・順位スコアなど問題ごとの実装に委ねる。
pub(super) trait ComparativeScoreCalculator: Send + Sync {
    /// 指定したケースの比較スコアを計算する。
    fn calculate(&self, seed: u64, score: NonZeroU64) -> f64;
}

/// 既知のベストスコアに対する相対スコアを計算する実装。
///
/// `Objective::Max` では `score / best_score * 100`、
/// `Objective::Min` では `best_score / score * 100` を返す。
pub(super) struct RelativeScoreCalculator {
    best_scores: HashMap<u64, NonZeroU64>,
    objective: Objective,
}

impl RelativeScoreCalculator {
    /// ベストスコア表と最適化方向から相対スコア計算器を構築する。
    pub(super) fn new(best_scores: HashMap<u64, NonZeroU64>, objective: Objective) -> Self {
        Self {
            best_scores,
            objective,
        }
    }
}

impl ComparativeScoreCalculator for RelativeScoreCalculator {
    fn calculate(&self, seed: u64, score: NonZeroU64) -> f64 {
        match (self.best_scores.get(&seed).copied(), self.objective) {
            (Some(reference_score), Objective::Max) => {
                score.get() as f64 / reference_score.get() as f64 * 100.0
            }
            (Some(reference_score), Objective::Min) => {
                reference_score.get() as f64 / score.get() as f64 * 100.0
            }
            (None, _) => 100.0,
        }
    }
}

/// 相対スコア計算器を共有可能な `Arc` に包んで生成する。
pub(super) fn create_relative_score_calculator(
    best_scores: HashMap<u64, NonZeroU64>,
    objective: Objective,
) -> Arc<dyn ComparativeScoreCalculator> {
    Arc::new(RelativeScoreCalculator::new(best_scores, objective))
}

/// ローカルに保存された提出結果に対する順位スコアを計算する実装。
pub(super) struct RankScoreCalculator {
    scores_by_seed: HashMap<u64, Vec<NonZeroU64>>,
    objective: Objective,
    includes_current_submission: bool,
}

impl RankScoreCalculator {
    /// 過去の結果一覧から順位スコア計算器を構築する。
    pub(super) fn new(
        results: &[AllResultJson],
        objective: Objective,
        includes_current_submission: bool,
    ) -> Self {
        let mut scores_by_seed = HashMap::<u64, Vec<NonZeroU64>>::new();

        for result in results {
            for case in &result.cases {
                let Some(score) = NonZeroU64::new(case.score) else {
                    continue;
                };
                scores_by_seed.entry(case.seed).or_default().push(score);
            }
        }

        for scores in scores_by_seed.values_mut() {
            scores.sort_unstable();
        }

        Self {
            scores_by_seed,
            objective,
            includes_current_submission,
        }
    }
}

impl ComparativeScoreCalculator for RankScoreCalculator {
    fn calculate(&self, seed: u64, score: NonZeroU64) -> f64 {
        let scores = self.scores_by_seed.get(&seed);
        let history_count = scores.map_or(0, Vec::len);
        let n_submit = history_count + usize::from(!self.includes_current_submission);

        if n_submit == 0 {
            return PERFECT_RANK_SCORE;
        }

        let (better_count, equal_count) = scores
            .map(|scores| count_better_and_equal_scores(scores, score, self.objective))
            .unwrap_or((0, 0));

        let tie_count = if self.includes_current_submission {
            equal_count.saturating_sub(1)
        } else {
            equal_count
        };
        let rank = better_count as f64 + 0.5 * tie_count as f64;

        let score = PERFECT_RANK_SCORE * (1.0 - rank / n_submit as f64);
        score.max(0.0)
    }
}

fn count_better_and_equal_scores(
    scores: &[NonZeroU64],
    target: NonZeroU64,
    objective: Objective,
) -> (usize, usize) {
    let lower_bound = scores.partition_point(|score| score < &target);
    let upper_bound = scores.partition_point(|score| score <= &target);
    let equal_count = upper_bound - lower_bound;

    match objective {
        Objective::Max => (scores.len() - upper_bound, equal_count),
        Objective::Min => (lower_bound, equal_count),
    }
}

/// 順位スコア計算器を共有可能な `Arc` に包んで生成する。
pub(super) fn create_rank_score_calculator(
    results: &[AllResultJson],
    objective: Objective,
    includes_current_submission: bool,
) -> Arc<dyn ComparativeScoreCalculator> {
    Arc::new(RankScoreCalculator::new(
        results,
        objective,
        includes_current_submission,
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_relative_score_max() {
        let best_scores = HashMap::from([(42, NonZeroU64::new(100).unwrap())]);
        let calculator = RelativeScoreCalculator::new(best_scores, Objective::Max);

        assert_eq!(
            calculator.calculate(42, NonZeroU64::new(200).unwrap()),
            200.0
        );
    }

    #[test]
    fn test_relative_score_min() {
        let best_scores = HashMap::from([(42, NonZeroU64::new(100).unwrap())]);
        let calculator = RelativeScoreCalculator::new(best_scores, Objective::Min);

        assert_eq!(
            calculator.calculate(42, NonZeroU64::new(200).unwrap()),
            50.0
        );
    }

    #[test]
    fn test_relative_score_without_reference() {
        let best_scores = HashMap::new();
        let calculator = RelativeScoreCalculator::new(best_scores, Objective::Max);

        assert_eq!(
            calculator.calculate(42, NonZeroU64::new(200).unwrap()),
            100.0
        );
    }

    #[test]
    fn test_average_relative_score_with_failed_case() {
        let best_scores = HashMap::from([
            (0, NonZeroU64::new(100).unwrap()),
            (1, NonZeroU64::new(100).unwrap()),
            (2, NonZeroU64::new(100).unwrap()),
        ]);
        let calculator = RelativeScoreCalculator::new(best_scores, Objective::Max);
        let scores = [
            (0, NonZeroU64::new(100)),
            (1, NonZeroU64::new(200)),
            (2, None),
        ];

        let average = scores
            .into_iter()
            .map(|(seed, score)| {
                score
                    .map(|score| calculator.calculate(seed, score))
                    .unwrap_or(0.0)
            })
            .sum::<f64>()
            / scores.len() as f64;

        assert_eq!(average, 100.0);
    }

    #[test]
    fn test_rank_score_without_history() {
        let calculator = RankScoreCalculator::new(&[], Objective::Min, false);

        assert_eq!(
            calculator.calculate(42, NonZeroU64::new(200).unwrap()),
            100.0
        );
    }

    #[test]
    fn test_rank_score_for_new_submission() {
        let results = [AllResultJson {
            start_time: chrono::Local::now(),
            case_count: 2,
            total_score: 100,
            total_score_log10: 2.0,
            total_relative_score: 100.0,
            max_execution_time: 0.1,
            comment: String::new(),
            tag_name: None,
            wa_seeds: vec![],
            cases: vec![
                super::super::io::CaseResultJson {
                    seed: 0,
                    score: 100,
                    relative_score: 100.0,
                    execution_time: 0.1,
                    error_message: String::new(),
                },
                super::super::io::CaseResultJson {
                    seed: 0,
                    score: 200,
                    relative_score: 50.0,
                    execution_time: 0.1,
                    error_message: String::new(),
                },
            ],
        }];
        let calculator = RankScoreCalculator::new(&results, Objective::Min, false);

        assert_eq!(
            calculator.calculate(0, NonZeroU64::new(150).unwrap()),
            66.66666666666667
        );
    }

    #[test]
    fn test_rank_score_for_saved_submission() {
        let results = [AllResultJson {
            start_time: chrono::Local::now(),
            case_count: 3,
            total_score: 100,
            total_score_log10: 2.0,
            total_relative_score: 100.0,
            max_execution_time: 0.1,
            comment: String::new(),
            tag_name: None,
            wa_seeds: vec![],
            cases: vec![
                super::super::io::CaseResultJson {
                    seed: 0,
                    score: 100,
                    relative_score: 100.0,
                    execution_time: 0.1,
                    error_message: String::new(),
                },
                super::super::io::CaseResultJson {
                    seed: 0,
                    score: 100,
                    relative_score: 100.0,
                    execution_time: 0.1,
                    error_message: String::new(),
                },
                super::super::io::CaseResultJson {
                    seed: 0,
                    score: 200,
                    relative_score: 50.0,
                    execution_time: 0.1,
                    error_message: String::new(),
                },
            ],
        }];
        let calculator = RankScoreCalculator::new(&results, Objective::Min, true);

        assert_eq!(
            calculator.calculate(0, NonZeroU64::new(100).unwrap()),
            83.33333333333334
        );
    }
}
