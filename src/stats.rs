use std::fmt;

use rand::Rng;

use crate::{
    evaluate,
    parser::{ParseError, parse},
};

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct StatsResult {
    pub samples: usize,
    pub min: i64,
    pub max: i64,
    pub mean: f64,
    pub std_dev: f64,
}

impl fmt::Display for StatsResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "samples = {}", self.samples)?;
        writeln!(f, "min     = {}", self.min)?;
        writeln!(f, "max     = {}", self.max)?;
        writeln!(f, "mean    = {:.2}", self.mean)?;
        write!(f, "std_dev = {:.2}", self.std_dev)?;
        Ok(())
    }
}

/// Computes statistics on several rolls of the same expression.
///
/// # Errors
///
/// Propagates any parse error from [`crate::parser::parse`]
///
/// # Examples
///
/// ```
///
/// ```
pub fn run_stats(
    expr: &str,
    samples: usize,
    rng: &mut impl Rng,
) -> Result<StatsResult, ParseError> {
    let terms = parse(expr)?;
    let results = (0..samples).map(|_| evaluate(&terms, rng).total);

    let mut count = 0f64;
    let mut mean = 0f64;
    let mut m2 = 0f64;
    let mut min = i64::MAX;
    let mut max = i64::MIN;

    for r in results {
        let x = r as f64;

        count += 1.0;
        let delta = x - mean;
        mean += delta / count;
        m2 += delta * (x - mean);

        if r < min {
            min = r
        };
        if r > max {
            max = r
        };
    }

    let variance = m2 / count;
    let std_dev = variance.sqrt();

    Ok(StatsResult {
        samples,
        min,
        max,
        mean,
        std_dev,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn display_formats_all_fields() {
        let stats = StatsResult {
            samples: 12,
            min: 2,
            max: 11,
            mean: 6.25,
            std_dev: 2.75,
        };

        assert_eq!(
            stats.to_string(),
            "samples = 12\nmin     = 2\nmax     = 11\nmean    = 6.25\nstd_dev = 2.75"
        );
    }

    #[test]
    fn run_stats_reports_constant_expression() {
        let mut rng = StdRng::seed_from_u64(7);
        let stats = run_stats("2", 5, &mut rng).unwrap();

        assert_eq!(
            stats,
            StatsResult {
                samples: 5,
                min: 2,
                max: 2,
                mean: 2.0,
                std_dev: 0.0,
            }
        );
    }

    #[test]
    fn run_stats_propagates_parse_errors() {
        let mut rng = StdRng::seed_from_u64(7);
        let err = run_stats("foo", 5, &mut rng).unwrap_err();

        assert_eq!(err.to_string(), "unexpected input: 'foo'");
    }
}
