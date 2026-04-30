use std::fmt::Write;

use rand::Rng;

use crate::parser::{Term, parse};

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
pub fn run(expr: &str, rng: &mut impl Rng) -> Result<EvalResult, String> {
    let terms = parse(expr)?;
    Ok(evaluate(&terms, rng))
}

pub fn evaluate(terms: &[(i64, Term)], rng: &mut impl Rng) -> EvalResult {
    let mut total: i64 = 0;
    let mut out_terms: Vec<EvalTerm> = Vec::with_capacity(terms.len());

    for (sign, term) in terms {
        let sign = *sign;
        let (kind, subtotal) = match *term {
            Term::Dice { count, sides } => {
                let mut sum: u64 = 0;
                let mut rolls: Vec<u64> = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let r = rng.random_range(1..=sides);
                    sum += r;
                    rolls.push(r);
                }
                (
                    EvalTermKind::Dice { count, sides, rolls },
                    sign * sum as i64,
                )
            }
            Term::Const(n) => (EvalTermKind::Const(n), sign * n as i64),
        };
        total += subtotal;
        out_terms.push(EvalTerm { sign, kind, subtotal });
    }

    EvalResult { total, terms: out_terms }
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
        let mut out = String::new();
        for (idx, term) in self.terms.iter().enumerate() {
            let op = if term.sign < 0 {
                " - "
            } else if idx == 0 {
                ""
            } else {
                " + "
            };
            out.push_str(op);
            match &term.kind {
                EvalTermKind::Dice { count, sides, rolls } => {
                    write!(out, "{count}d{sides}[").unwrap();
                    for (i, r) in rolls.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        write!(out, "{r}").unwrap();
                    }
                    out.push(']');
                }
                EvalTermKind::Const(n) => {
                    write!(out, "{n}").unwrap();
                }
            }
        }
        out
    }

    pub fn json(&self) -> String {
        let mut out = String::new();
        write!(out, "{{\"total\":{},\"terms\":[", self.total).unwrap();
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            match &term.kind {
                EvalTermKind::Dice { count, sides, rolls } => {
                    write!(
                        out,
                        "{{\"kind\":\"dice\",\"sign\":{},\"count\":{count},\"sides\":{sides},\"rolls\":[",
                        term.sign
                    )
                    .unwrap();
                    for (j, r) in rolls.iter().enumerate() {
                        if j > 0 {
                            out.push(',');
                        }
                        write!(out, "{r}").unwrap();
                    }
                    write!(out, "],\"subtotal\":{}}}", term.subtotal).unwrap();
                }
                EvalTermKind::Const(n) => {
                    write!(
                        out,
                        "{{\"kind\":\"const\",\"sign\":{},\"value\":{n},\"subtotal\":{}}}",
                        term.sign, term.subtotal,
                    )
                    .unwrap();
                }
            }
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
        assert_eq!(run("2d20+3d6+4", &mut a).unwrap(), run("2d20+3d6+4", &mut b).unwrap());
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
    fn subtotals_sum_to_total() {
        let mut rng = StdRng::seed_from_u64(31);
        let r = run("4d6-1d4+2", &mut rng).unwrap();
        let computed: i64 = r.terms.iter().map(|t| t.subtotal).sum();
        assert_eq!(computed, r.total);
    }
}
