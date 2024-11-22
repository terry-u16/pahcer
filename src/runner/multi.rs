use super::single::{SingleCaseRunner, TestCase, TestResult};
use colored::Colorize;
use num_format::{Locale, ToFormattedString};
use std::sync::{mpsc, Arc};
use threadpool::ThreadPool;

/// The runner for multiple cases.
#[derive(Debug, Clone)]
pub(crate) struct MultiCaseRunner {
    single_runner: SingleCaseRunner,
    test_cases: Vec<TestCase>,
    threads: usize,
}

impl MultiCaseRunner {
    pub(crate) fn new(
        single_runner: SingleCaseRunner,
        test_cases: Vec<TestCase>,
        threads: usize,
    ) -> Self {
        Self {
            single_runner,
            test_cases,
            threads,
        }
    }

    pub(super) fn run(&mut self) -> TestStats {
        let rx = self.start_tests();
        self.collect_results(rx)
    }

    fn start_tests(&mut self) -> mpsc::Receiver<TestResult> {
        let thread_cnt = match self.threads {
            0 => num_cpus::get(),
            n => n,
        };

        let threadpool = ThreadPool::new(thread_cnt);
        let (tx, rx) = mpsc::channel();
        let single_runner = Arc::new(self.single_runner.clone());

        // 送信側
        for &test_case in self.test_cases.iter() {
            let tx = tx.clone();
            let runner = single_runner.clone();
            threadpool.execute(move || {
                let result = runner.run(test_case);
                tx.send(result).expect("Failed to send result");
            });
        }

        rx
    }

    fn collect_results(&mut self, rx: mpsc::Receiver<TestResult>) -> TestStats {
        let mut results = Vec::with_capacity(self.test_cases.len());
        let mut printer = ResultPrinter::new(self.test_cases.len());

        for result in rx {
            for row in printer.gen_record(&result) {
                println!("{}", row);
            }

            results.push(result);
        }

        results.sort_unstable_by_key(|r| r.test_case().seed());

        let stats = TestStats::new(results);

        for row in printer.gen_stats_footer(&stats) {
            println!("{}", row);
        }

        stats
    }
}

struct ResultPrinter {
    testcase_count: usize,
    completed_count: usize,
    score_width: usize,
    score_sum: u64,
    relative_score_sum: f64,
}

impl ResultPrinter {
    fn new(testcase_count: usize) -> Self {
        assert!(testcase_count > 0);

        Self {
            testcase_count,
            completed_count: 0,
            score_width: 7,
            score_sum: 0,
            relative_score_sum: 0.0,
        }
    }

    /// ヘッダーを生成する
    ///
    /// # Caution
    ///
    /// 1つ目のレコード生成時に呼び出すこと
    fn gen_header(&mut self) -> Vec<String> {
        assert!(self.completed_count == 1);

        // スコア列の幅を決定する（スコアの桁数 + 余裕分3桁）
        self.score_width = self
            .score_width
            .max(self.score_sum.to_formatted_string(&Locale::en).len() + 3);

        let test_width = self.testcase_count.to_string().len() * 2 + 8;
        let score_width1 = self.score_width + 11;
        let score_width2 = self.score_width;
        let mut rows = vec![];

        rows.push(format!(
            "| {:^test_width$} | {:^4} | {:^score_width1$} | {:^score_width1$} | {:^9} |",
            "Progress", "Seed", "Case Score", "Average Score", "Exec."
        ));
        rows.push(format!(
            "| {:^test_width$} | {:^4} | {:^score_width2$} | {:^8} | {:^score_width2$} | {:^8} | {:^9} |",
            "", "", "Score", "Relative", "Score", "Relative", "Time"
        ));

        let test_width = test_width + 2;
        let score_width2 = score_width2 + 2;
        rows.push(format!(
            "|{:-^test_width$}|{:-^6}|{:-^score_width2$}|{:-^10}|{:-^score_width2$}|{:-^10}|{:-^11}|",
            "", "", "", "", "", "", ""
        ));

        rows
    }

    fn gen_record(&mut self, result: &TestResult) -> Vec<String> {
        self.completed_count += 1;
        assert!(self.completed_count <= self.testcase_count);

        let score = result.score().as_ref().map(|s| s.get()).unwrap_or(0);
        let relative_score = result.relative_score().as_ref().copied().unwrap_or(0.0);
        self.score_sum += score;
        self.relative_score_sum += relative_score;

        let mut rows = vec![];

        if self.completed_count == 1 {
            rows.extend(self.gen_header());
        }

        let digit = self.testcase_count.to_string().len();

        let score = score.to_formatted_string(&Locale::en);
        let average_score = ((self.score_sum as f64 / self.completed_count as f64).round() as u64)
            .to_formatted_string(&Locale::en);
        let duration = result
            .duration()
            .as_millis()
            .to_formatted_string(&Locale::en);
        let average_relative_score = self.relative_score_sum / self.completed_count as f64;
        self.score_width = self.score_width.max(score.len());
        let score_width = self.score_width;

        let mut row = format!(
            "| case {:digit$} / {:digit$} | {:04} | {:>score_width$} | {:8.3} | {:>score_width$} | {:8.3} | {:>6} ms |",
            self.completed_count,
            self.testcase_count,
            result.test_case().seed(),
            score,
            relative_score,
            average_score,
            average_relative_score,
            duration,
        );

        if let Err(e) = result.score() {
            row.push_str(&format!("\n{}", e));
            row = row.yellow().to_string();
        }

        rows.push(row);

        rows
    }

    fn gen_stats_footer(&self, stats: &TestStats) -> Vec<String> {
        let mut rows = vec![];

        let average_score = ((stats.score_sum as f64 / stats.results.len() as f64).round() as u64)
            .to_formatted_string(&Locale::en);
        let average_score_log10 = stats.score_sum_log10 / stats.results.len() as f64;
        let average_relative_score = stats.relative_score_sum / stats.results.len() as f64;
        let ac_count =
            stats.results.len() - stats.results.iter().filter(|r| r.score().is_err()).count();

        rows.push(format!("Average Score          : {}", average_score));
        rows.push(format!(
            "Average Score (log10)  : {:.3}",
            average_score_log10
        ));
        rows.push(format!(
            "Average Relative Score : {:.3}",
            average_relative_score
        ));

        let ac = format!("{} / {}", ac_count, stats.results.len());
        let ac = if ac_count == stats.results.len() {
            ac.bold().green().to_string()
        } else {
            ac.bold().yellow().to_string()
        };
        rows.push(format!("Accepted               : {}", ac));

        rows
    }
}

#[derive(Debug, Clone)]
pub(super) struct TestStats {
    pub(super) results: Vec<TestResult>,
    pub(super) score_sum: u64,
    pub(super) score_sum_log10: f64,
    pub(super) relative_score_sum: f64,
}

impl TestStats {
    pub(crate) fn new(results: Vec<TestResult>) -> Self {
        let score_sum = results
            .iter()
            .filter_map(|r| r.score().as_ref().ok().map(|s| s.get()))
            .sum();
        let score_sum_log10 = results
            .iter()
            .filter_map(|r| r.score_log10().ok())
            .sum::<f64>()
            .max(0.0);
        let relative_score_sum = results
            .iter()
            .filter_map(|r| r.relative_score().as_ref().ok())
            .sum::<f64>()
            .max(0.0);

        Self {
            results,
            score_sum,
            score_sum_log10,
            relative_score_sum,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runner::single::{Objective, TestStep};
    use regex::Regex;
    use std::{cell::LazyCell, num::NonZero, time::Duration};

    const SCORE_REGEX: LazyCell<Regex> =
        LazyCell::new(|| Regex::new(r"^\s*Score\s*=\s*(?P<score>\d+)\s*$").unwrap());

    #[test]
    fn test_multi_case_runner() {
        let steps = vec![TestStep::new(
            "echo".to_string(),
            vec!["Score = 100".to_string()],
            None,
            None,
            None,
            None,
        )];
        let single_runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let test_cases = vec![
            TestCase::new(0, NonZero::new(100), Objective::Max),
            TestCase::new(1, NonZero::new(200), Objective::Max),
            TestCase::new(2, NonZero::new(50), Objective::Max),
            TestCase::new(3, None, Objective::Max),
        ];
        let mut runner = MultiCaseRunner {
            single_runner,
            test_cases,
            threads: 0,
        };

        let stats = runner.run();

        assert_eq!(stats.results.len(), 4);
        assert_eq!(stats.score_sum, 400);
        assert_eq!(stats.score_sum_log10, 8.0);
        assert_eq!(stats.relative_score_sum, 450.0);
    }

    #[test]
    fn test_result_printer() {
        let mut printer = ResultPrinter::new(3);

        let result1 = TestResult::new(
            TestCase::new(0, NonZero::new(100), Objective::Max),
            Ok(NonZero::new(1000).unwrap()),
            Duration::from_millis(1234),
        );

        let result2 = TestResult::new(
            TestCase::new(1, NonZero::new(100), Objective::Max),
            Ok(NonZero::new(500).unwrap()),
            Duration::from_millis(12345),
        );

        let result3 = TestResult::new(
            TestCase::new(2, NonZero::new(100), Objective::Max),
            Err("error".to_string()),
            Duration::from_millis(1),
        );

        let mut rows = vec![];
        rows.extend(printer.gen_record(&result1));
        rows.extend(printer.gen_record(&result2));
        rows.extend(printer.gen_record(&result3));
        rows.extend(printer.gen_stats_footer(&TestStats::new(vec![result1, result2, result3])));

        let expected = vec![
            "|  Progress  | Seed |     Case Score      |    Average Score    |   Exec.   |",
            "|            |      |  Score   | Relative |  Score   | Relative |   Time    |",
            "|------------|------|----------|----------|----------|----------|-----------|",
            "| case 1 / 3 | 0000 |    1,000 | 1000.000 |    1,000 | 1000.000 |  1,234 ms |",
            "| case 2 / 3 | 0001 |      500 |  500.000 |      750 |  750.000 | 12,345 ms |",
            "\u{1b}[33m| case 3 / 3 | 0002 |        0 |    0.000 |      500 |  500.000 |      1 ms |\nerror\u{1b}[0m",
            "Average Score          : 500",
            "Average Score (log10)  : 1.900",
            "Average Relative Score : 500.000",
            "Accepted               : \u{1b}[1;33m2 / 3\u{1b}[0m",
        ];

        println!("[EXPECTED]");

        for row in &expected {
            println!("{}", row);
        }

        println!();
        println!("[ACTUAL]");

        for row in &rows {
            println!("{}", row);
        }

        assert_eq!(expected, rows);
    }
}
