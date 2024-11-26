mod printer;

use super::single::{SingleCaseRunner, TestCase, TestResult};
use anyhow::Result;
use chrono::{DateTime, Local};
use printer::Printer;
use std::sync::{mpsc, Arc};
use threadpool::ThreadPool;

/// The runner for multiple cases.
pub(super) struct MultiCaseRunner {
    single_runner: SingleCaseRunner,
    test_cases: Vec<TestCase>,
    threads: usize,
    printer: Box<dyn Printer>,
}

impl MultiCaseRunner {
    pub(super) fn new_console(
        single_runner: SingleCaseRunner,
        test_cases: Vec<TestCase>,
        threads: usize,
    ) -> Self {
        let printer = Box::new(printer::ConsolePrinter::new(test_cases.len()));
        Self::new(single_runner, test_cases, threads, printer)
    }

    pub(super) fn new_json(
        single_runner: SingleCaseRunner,
        test_cases: Vec<TestCase>,
        threads: usize,
    ) -> Self {
        let printer = Box::new(printer::JsonPrinter::new());
        Self::new(single_runner, test_cases, threads, printer)
    }

    fn new(
        single_runner: SingleCaseRunner,
        test_cases: Vec<TestCase>,
        threads: usize,
        printer: Box<dyn Printer>,
    ) -> Self {
        Self {
            single_runner,
            test_cases,
            threads,
            printer,
        }
    }

    pub(super) fn run(&mut self) -> Result<TestStats> {
        let (rx, start_time) = self.start_tests();
        self.collect_results(rx, start_time)
    }

    fn start_tests(&mut self) -> (mpsc::Receiver<TestResult>, DateTime<Local>) {
        let start_time = Local::now();
        let thread_cnt = match self.threads {
            0 => num_cpus::get_physical(),
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

        (rx, start_time)
    }

    fn collect_results(
        &mut self,
        rx: mpsc::Receiver<TestResult>,
        start_time: DateTime<Local>,
    ) -> Result<TestStats> {
        let mut results = Vec::with_capacity(self.test_cases.len());
        let mut stdio = std::io::stdout();

        for result in rx {
            self.printer.print_case(&mut stdio, &result)?;
            results.push(result);
        }

        results.sort_unstable_by_key(|r| r.test_case().seed());

        let stats = TestStats::new(results, start_time);

        self.printer.print_summary(&mut stdio, &stats)?;

        Ok(stats)
    }
}

#[derive(Debug, Clone)]
pub(super) struct TestStats {
    pub(super) results: Vec<TestResult>,
    pub(super) score_sum: u64,
    pub(super) score_sum_log10: f64,
    pub(super) relative_score_sum: f64,
    pub(super) start_time: DateTime<Local>,
}

impl TestStats {
    pub(crate) fn new(results: Vec<TestResult>, start_time: DateTime<Local>) -> Self {
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
            start_time,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runner::single::{Objective, TestStep};
    use printer::MockPrinter;
    use regex::Regex;
    use std::{cell::LazyCell, num::NonZero};

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
            true,
        )];
        let single_runner = SingleCaseRunner::new(steps, SCORE_REGEX.clone());
        let test_cases = vec![
            TestCase::new(0, NonZero::new(100), Objective::Max),
            TestCase::new(1, NonZero::new(200), Objective::Max),
            TestCase::new(2, NonZero::new(50), Objective::Max),
            TestCase::new(3, None, Objective::Max),
        ];

        let mut printer = MockPrinter::new();
        printer
            .expect_print_case()
            .times(4)
            .returning(|_, _| Ok(()));
        printer
            .expect_print_summary()
            .times(1)
            .returning(|_, _| Ok(()));
        let mut runner = MultiCaseRunner::new(single_runner, test_cases, 0, Box::new(printer));

        let stats = runner.run().unwrap();

        assert_eq!(stats.results.len(), 4);
        assert_eq!(stats.score_sum, 400);
        assert_eq!(stats.score_sum_log10, 8.0);
        assert_eq!(stats.relative_score_sum, 450.0);
    }
}
