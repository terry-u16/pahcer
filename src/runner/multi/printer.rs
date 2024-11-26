use super::{TestResult, TestStats};
use anyhow::Result;
use colored::Colorize as _;
use num_format::{Locale, ToFormattedString as _};
use std::io::Write;

#[cfg_attr(test, mockall::automock)]
pub(super) trait Printer {
    fn print_case(&mut self, writer: &mut dyn Write, result: &TestResult) -> Result<()>;
    fn print_summary(&mut self, writer: &mut dyn Write, stats: &TestStats) -> Result<()>;
}

pub(super) struct ConsolePrinter {
    testcase_count: usize,
    completed_count: usize,
    score_width: usize,
    score_sum: u64,
    relative_score_sum: f64,
}

impl Printer for ConsolePrinter {
    fn print_case(&mut self, writer: &mut dyn Write, result: &TestResult) -> Result<()> {
        self.completed_count += 1;
        assert!(self.completed_count <= self.testcase_count);

        let score = result.score().as_ref().map(|s| s.get()).unwrap_or(0);
        let relative_score = result.relative_score().as_ref().copied().unwrap_or(0.0);
        self.score_sum += score;
        self.relative_score_sum += relative_score;

        if self.completed_count == 1 {
            self.print_header(writer)?;
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

        let record = format!(
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

        match result.score() {
            Ok(_) => writeln!(writer, "{}", record)?,
            Err(e) => {
                writeln!(writer, "{}", record.yellow().to_string())?;
                writeln!(writer, "{}", e.to_string().yellow().to_string())?;
            }
        };

        Ok(())
    }

    fn print_summary(&mut self, writer: &mut dyn Write, stats: &TestStats) -> Result<()> {
        let average_score = ((stats.score_sum as f64 / stats.results.len() as f64).round() as u64)
            .to_formatted_string(&Locale::en);
        let average_score_log10 = stats.score_sum_log10 / stats.results.len() as f64;
        let average_relative_score = stats.relative_score_sum / stats.results.len() as f64;
        let ac_count =
            stats.results.len() - stats.results.iter().filter(|r| r.score().is_err()).count();

        writeln!(writer, "Average Score          : {}", average_score)?;
        writeln!(
            writer,
            "Average Score (log10)  : {:.3}",
            average_score_log10
        )?;
        writeln!(
            writer,
            "Average Relative Score : {:.3}",
            average_relative_score
        )?;

        let ac = format!("{} / {}", ac_count, stats.results.len());
        let ac = if ac_count == stats.results.len() {
            ac.bold().green().to_string()
        } else {
            ac.bold().yellow().to_string()
        };
        writeln!(writer, "Accepted               : {}", ac)?;

        Ok(())
    }
}

impl ConsolePrinter {
    pub(super) fn new(testcase_count: usize) -> Self {
        assert!(testcase_count > 0);

        Self {
            testcase_count,
            completed_count: 0,
            score_width: 7,
            score_sum: 0,
            relative_score_sum: 0.0,
        }
    }

    fn print_header(&mut self, writer: &mut dyn Write) -> Result<()> {
        assert!(self.completed_count == 1);

        // スコア列の幅を決定する（スコアの桁数 + 余裕分3桁）
        self.score_width = self
            .score_width
            .max(self.score_sum.to_formatted_string(&Locale::en).len() + 3);

        let test_width = self.testcase_count.to_string().len() * 2 + 8;
        let score_width1 = self.score_width + 11;
        let score_width2 = self.score_width;

        writeln!(
            writer,
            "| {:^test_width$} | {:^4} | {:^score_width1$} | {:^score_width1$} | {:^9} |",
            "Progress", "Seed", "Case Score", "Average Score", "Exec."
        )?;

        writeln!(
            writer,
            "| {:^test_width$} | {:^4} | {:^score_width2$} | {:^8} | {:^score_width2$} | {:^8} | {:^9} |",
            "", "", "Score", "Relative", "Score", "Relative", "Time"
        )?;

        let test_width = test_width + 2;
        let score_width2 = score_width2 + 2;
        writeln!(
            writer,
            "|{:-^test_width$}|{:-^6}|{:-^score_width2$}|{:-^10}|{:-^score_width2$}|{:-^10}|{:-^11}|",
            "", "", "", "", "", "", ""
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::runner::{multi::TestCase, single::Objective};
    use chrono::Local;
    use std::{num::NonZero, time::Duration};

    use super::*;

    #[test]
    fn test_result_printer() {
        let mut printer = ConsolePrinter::new(3);

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

        let mut buf = Box::new(vec![]);
        printer.print_case(&mut buf, &result1).unwrap();
        printer.print_case(&mut buf, &result2).unwrap();
        printer.print_case(&mut buf, &result3).unwrap();
        printer
            .print_summary(
                &mut buf,
                &TestStats::new(vec![result1, result2, result3], Local::now()),
            )
            .unwrap();

        let expected =
            "|  Progress  | Seed |     Case Score      |    Average Score    |   Exec.   |
|            |      |  Score   | Relative |  Score   | Relative |   Time    |
|------------|------|----------|----------|----------|----------|-----------|
| case 1 / 3 | 0000 |    1,000 | 1000.000 |    1,000 | 1000.000 |  1,234 ms |
| case 2 / 3 | 0001 |      500 |  500.000 |      750 |  750.000 | 12,345 ms |
\u{1b}[33m| case 3 / 3 | 0002 |        0 |    0.000 |      500 |  500.000 |      1 ms |\u{1b}[0m
\u{1b}[33merror\u{1b}[0m
Average Score          : 500
Average Score (log10)  : 1.900
Average Relative Score : 500.000
Accepted               : \u{1b}[1;33m2 / 3\u{1b}[0m
";

        println!("[EXPECTED]");
        println!("{}", expected);

        println!();
        println!("[ACTUAL]");

        let actual = String::from_utf8(*buf).unwrap();
        println!("{}", actual);

        assert_eq!(expected, actual);
    }
}
