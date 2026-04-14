use super::single::Objective;
use std::{collections::HashMap, num::NonZeroU64, sync::Arc};

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

/// 相対スコア計算器を共有可能な `Arc` に包んで生成する。
pub(super) fn create_relative_score_calculator(
    best_scores: HashMap<u64, NonZeroU64>,
    objective: Objective,
) -> Arc<dyn ComparativeScoreCalculator> {
    Arc::new(RelativeScoreCalculator::new(best_scores, objective))
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
}
