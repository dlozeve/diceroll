use std::fmt::Write;

use rand::Rng;

use crate::parser::{ParseError, Term, parse};

#[derive(Debug, PartialEq, Eq)]
pub struct EvalResult {
    pub total: i64,
    pub terms: Vec<EvalTerm>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EvalTerm {
    pub sign: i64,
    pub kind: EvalTermKind,
    pub subtotal: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvalTermKind {
    Dice {
        count: u64,
        sides: u64,
        rolls: Vec<u64>,
    },
    Const(u64),
    Group {
        terms: Vec<EvalTerm>,
        multiplier: u64,
    },
}

/// Parses and evaluates a dice expression, returning a full breakdown.
///
/// # Errors
///
/// Propagates any parse error from [`crate::parser::parse`].
///
/// # Examples
///
/// ```
/// use rand::SeedableRng;
/// use rand::rngs::StdRng;
///
/// let mut rng = StdRng::seed_from_u64(0);
/// let result = diceroll::run("2d6+3", &mut rng).unwrap();
/// assert!(result.total >= 5 && result.total <= 15);
/// assert_eq!(result.terms.len(), 2);
/// ```
pub fn run(expr: &str, rng: &mut impl Rng) -> Result<EvalResult, ParseError> {
    let terms = parse(expr)?;
    Ok(evaluate(&terms, rng))
}

pub fn evaluate(terms: &[(i64, Term)], rng: &mut impl Rng) -> EvalResult {
    let mut total: i64 = 0;
    let mut out_terms: Vec<EvalTerm> = Vec::with_capacity(terms.len());

    for (sign, term) in terms {
        let sign = *sign;
        let (kind, subtotal) = match term {
            Term::Dice { count, sides } => {
                let count = *count;
                let sides = *sides;
                let mut sum: u64 = 0;
                let mut rolls: Vec<u64> = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let r = rng.random_range(1..=sides);
                    sum += r;
                    rolls.push(r);
                }
                (
                    EvalTermKind::Dice {
                        count,
                        sides,
                        rolls,
                    },
                    sign * sum as i64,
                )
            }
            Term::Const(n) => (EvalTermKind::Const(*n), sign * *n as i64),
            Term::Group {
                terms: inner_terms,
                multiplier,
            } => {
                let inner = evaluate(inner_terms, rng);
                let subtotal = sign * (*multiplier as i64) * inner.total;
                (
                    EvalTermKind::Group {
                        terms: inner.terms,
                        multiplier: *multiplier,
                    },
                    subtotal,
                )
            }
        };
        total += subtotal;
        out_terms.push(EvalTerm {
            sign,
            kind,
            subtotal,
        });
    }

    EvalResult {
        total,
        terms: out_terms,
    }
}

fn format_terms(terms: &[EvalTerm]) -> String {
    let mut out = String::new();
    for (idx, term) in terms.iter().enumerate() {
        let op = if term.sign < 0 {
            " - "
        } else if idx == 0 {
            ""
        } else {
            " + "
        };
        out.push_str(op);
        match &term.kind {
            EvalTermKind::Dice {
                count,
                sides,
                rolls,
            } => {
                let _ = write!(out, "{count}d{sides}[");
                for (i, r) in rolls.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "{r}");
                }
                out.push(']');
            }
            EvalTermKind::Const(n) => {
                let _ = write!(out, "{n}");
            }
            EvalTermKind::Group {
                terms: inner,
                multiplier,
            } => {
                out.push('(');
                out.push_str(&format_terms(inner));
                out.push(')');
                if *multiplier != 1 {
                    let _ = write!(out, " * {multiplier}");
                }
            }
        }
    }
    out
}

fn write_term_json(out: &mut String, term: &EvalTerm) {
    match &term.kind {
        EvalTermKind::Dice {
            count,
            sides,
            rolls,
        } => {
            let _ = write!(
                out,
                "{{\"kind\":\"dice\",\"sign\":{},\"count\":{count},\"sides\":{sides},\"rolls\":[",
                term.sign
            );
            for (j, r) in rolls.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{r}");
            }
            let _ = write!(out, "],\"subtotal\":{}}}", term.subtotal);
        }
        EvalTermKind::Const(n) => {
            let _ = write!(
                out,
                "{{\"kind\":\"const\",\"sign\":{},\"value\":{n},\"subtotal\":{}}}",
                term.sign, term.subtotal,
            );
        }
        EvalTermKind::Group {
            terms: inner_terms,
            multiplier,
        } => {
            let _ = write!(
                out,
                "{{\"kind\":\"group\",\"sign\":{},\"multiplier\":{multiplier},\"subtotal\":{},\"terms\":[",
                term.sign, term.subtotal
            );
            for (j, t) in inner_terms.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                write_term_json(out, t);
            }
            out.push_str("]}");
        }
    }
}

impl EvalResult {
    /// Returns a human-readable breakdown: each term with its rolls, then the total.
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let result = diceroll::run("3+4-1", &mut rng).unwrap();
    /// assert_eq!(result.display(), "3 + 4 - 1");
    /// ```
    pub fn display(&self) -> String {
        format_terms(&self.terms)
    }

    pub fn json(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "{{\"total\":{},\"terms\":[", self.total);
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_term_json(&mut out, term);
        }
        out.push_str("]}");
        out
    }

    /// Returns the plain display string (`"<breakdown> = <total>"`) or JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let result = diceroll::run("3+4-1", &mut rng).unwrap();
    /// assert_eq!(result.formatted(false), "3 + 4 - 1 = 6");
    /// let json = result.formatted(true);
    /// assert!(json.starts_with(r#"{"total":6"#));
    /// ```
    pub fn formatted(&self, json: bool) -> String {
        if json {
            self.json()
        } else {
            format!("{} = {}", self.display(), self.total)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn evaluate_constants_have_no_randomness() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("3+4-1", &mut rng).unwrap();
        assert_eq!(r.display(), "3 + 4 - 1");
        assert_eq!(r.total, 6);
    }

    #[test]
    fn evaluate_rolls_within_bounds() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = run("100d6", &mut rng).unwrap();
        assert!((100..=600).contains(&r.total));
    }

    #[test]
    fn evaluate_is_deterministic_for_seed() {
        let mut a = StdRng::seed_from_u64(123);
        let mut b = StdRng::seed_from_u64(123);
        assert_eq!(
            run("2d20+3d6+4", &mut a).unwrap(),
            run("2d20+3d6+4", &mut b).unwrap()
        );
    }

    #[test]
    fn evaluate_subtraction_total() {
        // d2 always rolls in [1, 2]; total of 1d2-3d2 lies in [1-6, 2-3] = [-5, -1]
        let mut rng = StdRng::seed_from_u64(99);
        let r = run("1d2-3d2", &mut rng).unwrap();
        assert!((-5..=-1).contains(&r.total));
    }

    #[test]
    fn evaluate_output_lists_each_roll() {
        let mut rng = StdRng::seed_from_u64(1);
        let out = run("3d6+2", &mut rng).unwrap().display();
        let bracket = &out[out.find('[').unwrap()..=out.find(']').unwrap()];
        assert_eq!(bracket.matches(',').count(), 2);
        assert!(out.starts_with("3d6["));
        assert!(out.contains("] + 2"));
    }

    #[test]
    fn evaluate_group_total_within_bounds() {
        let mut rng = StdRng::seed_from_u64(5);
        let r = run("(2d6+3)*2", &mut rng).unwrap();
        // 2d6 in [2,12], +3 → [5,15], *2 → [10,30]
        assert!((10..=30).contains(&r.total));
        assert_eq!(r.terms.len(), 1);
    }

    #[test]
    fn evaluate_group_display() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("(2d6+3)*2", &mut rng).unwrap();
        let d = r.display();
        assert!(d.starts_with('('));
        assert!(d.contains("2d6["));
        assert!(d.contains("] + 3)"));
        assert!(d.ends_with(" * 2"));
    }

    #[test]
    fn evaluate_group_no_multiplier_display() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("(d6)", &mut rng).unwrap();
        let d = r.display();
        assert!(d.starts_with('('));
        assert!(d.ends_with(')'));
        assert!(!d.contains('*'));
    }

    #[test]
    fn evaluate_full_expression_with_group() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("d20 + (2d6+3)*2 + 5", &mut rng).unwrap();
        assert_eq!(r.terms.len(), 3);
        // total = d20 + (2d6+3)*2 + 5; d20 in [1,20], group in [10,30]
        assert!((16..=55).contains(&r.total));
    }

    #[test]
    fn evaluate_nested_groups() {
        let mut rng = StdRng::seed_from_u64(3);
        let r = run("((d6+1)*2+3)*4", &mut rng).unwrap();
        // d6 in [1,6], +1 → [2,7], *2 → [4,14], +3 → [7,17], *4 → [28,68]
        assert!((28..=68).contains(&r.total));
    }

    #[test]
    fn subtotals_sum_to_total() {
        let mut rng = StdRng::seed_from_u64(31);
        let r = run("4d6-1d4+2", &mut rng).unwrap();
        let computed: i64 = r.terms.iter().map(|t| t.subtotal).sum();
        assert_eq!(computed, r.total);
    }

    #[test]
    fn subtotals_sum_to_total_with_group() {
        let mut rng = StdRng::seed_from_u64(55);
        let r = run("d20 + (2d6+3)*2 + 5", &mut rng).unwrap();
        let computed: i64 = r.terms.iter().map(|t| t.subtotal).sum();
        assert_eq!(computed, r.total);
    }

    #[test]
    fn json_output_for_constants_is_exact() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("3+4-1", &mut rng).unwrap();
        assert_eq!(
            r.json(),
            r#"{"total":6,"terms":[{"kind":"const","sign":1,"value":3,"subtotal":3},{"kind":"const","sign":1,"value":4,"subtotal":4},{"kind":"const","sign":-1,"value":1,"subtotal":-1}]}"#
        );
    }

    #[test]
    fn json_output_for_dice_has_expected_shape() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("2d6+3", &mut rng).unwrap();
        let json = r.json();
        assert!(json.starts_with(&format!("{{\"total\":{},", r.total)));
        assert!(json.contains(r#""kind":"dice""#));
        assert!(json.contains(r#""count":2"#));
        assert!(json.contains(r#""sides":6"#));
        assert!(json.contains(r#""rolls":["#));
        assert!(json.contains(r#""kind":"const""#));
        assert!(json.contains(r#""value":3"#));
        assert!(json.ends_with("]}"));
    }

    #[test]
    fn json_output_for_group_has_expected_shape() {
        let mut rng = StdRng::seed_from_u64(9);
        let r = run("(2d6+3)*2", &mut rng).unwrap();
        let json = r.json();
        assert!(json.contains(r#""kind":"group""#));
        assert!(json.contains(r#""multiplier":2"#));
        assert!(json.contains(r#""kind":"dice""#));
        assert!(json.contains(r#""kind":"const""#));
        assert!(json.ends_with("]}"));
    }
}
