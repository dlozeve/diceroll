use std::fmt::Write;

use rand::Rng;

use crate::parser::{KeepDrop, ParseError, Term, parse};

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
        keep_drop: Option<KeepDrop>,
        rolls: Vec<u64>,
        /// Parallel to `rolls`; `false` means the die was dropped.
        kept: Vec<bool>,
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

/// Returns a `kept` mask parallel to `rolls`: `false` = dropped.
fn apply_keep_drop(keep_drop: Option<&KeepDrop>, rolls: &[u64]) -> Vec<bool> {
    let n = rolls.len();
    let Some(kd) = keep_drop else {
        return vec![true; n];
    };
    let mut indices: Vec<usize> = (0..n).collect();
    match kd {
        KeepDrop::KeepHighest(k) => {
            indices.sort_by(|&a, &b| rolls[b].cmp(&rolls[a]));
            let mut kept = vec![false; n];
            for &i in indices.iter().take(*k as usize) {
                kept[i] = true;
            }
            kept
        }
        KeepDrop::KeepLowest(k) => {
            indices.sort_by(|&a, &b| rolls[a].cmp(&rolls[b]));
            let mut kept = vec![false; n];
            for &i in indices.iter().take(*k as usize) {
                kept[i] = true;
            }
            kept
        }
        KeepDrop::DropHighest(k) => {
            indices.sort_by(|&a, &b| rolls[b].cmp(&rolls[a]));
            let mut kept = vec![true; n];
            for &i in indices.iter().take(*k as usize) {
                kept[i] = false;
            }
            kept
        }
        KeepDrop::DropLowest(k) => {
            indices.sort_by(|&a, &b| rolls[a].cmp(&rolls[b]));
            let mut kept = vec![true; n];
            for &i in indices.iter().take(*k as usize) {
                kept[i] = false;
            }
            kept
        }
    }
}

pub fn evaluate(terms: &[(i64, Term)], rng: &mut impl Rng) -> EvalResult {
    let mut total: i64 = 0;
    let mut out_terms: Vec<EvalTerm> = Vec::with_capacity(terms.len());

    for (sign, term) in terms {
        let sign = *sign;
        let (kind, subtotal) = match term {
            Term::Dice {
                count,
                sides,
                keep_drop,
            } => {
                let count = *count;
                let sides = *sides;
                let mut rolls: Vec<u64> = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    rolls.push(rng.random_range(1..=sides));
                }
                let kept = apply_keep_drop(keep_drop.as_ref(), &rolls);
                let sum: u64 = rolls
                    .iter()
                    .zip(kept.iter())
                    .map(|(r, k)| if *k { *r } else { 0 })
                    .sum();
                (
                    EvalTermKind::Dice {
                        count,
                        sides,
                        keep_drop: keep_drop.clone(),
                        rolls,
                        kept,
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
                keep_drop,
                rolls,
                kept,
            } => {
                let _ = write!(out, "{count}d{sides}");
                match keep_drop {
                    Some(KeepDrop::KeepHighest(n)) => {
                        let _ = write!(out, "kh{n}");
                    }
                    Some(KeepDrop::KeepLowest(n)) => {
                        let _ = write!(out, "kl{n}");
                    }
                    Some(KeepDrop::DropHighest(n)) => {
                        let _ = write!(out, "dh{n}");
                    }
                    Some(KeepDrop::DropLowest(n)) => {
                        let _ = write!(out, "dl{n}");
                    }
                    None => {}
                }
                out.push('[');
                for (i, (r, &k)) in rolls.iter().zip(kept.iter()).enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    if k {
                        let _ = write!(out, "{r}");
                    } else {
                        let _ = write!(out, "{{{r}}}");
                    }
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
            keep_drop,
            rolls,
            kept,
        } => {
            let _ = write!(
                out,
                "{{\"kind\":\"dice\",\"sign\":{},\"count\":{count},\"sides\":{sides},",
                term.sign
            );
            if let Some(kd) = keep_drop {
                out.push_str("\"modifier\":\"");
                match kd {
                    KeepDrop::KeepHighest(n) => {
                        let _ = write!(out, "kh{n}");
                    }
                    KeepDrop::KeepLowest(n) => {
                        let _ = write!(out, "kl{n}");
                    }
                    KeepDrop::DropHighest(n) => {
                        let _ = write!(out, "dh{n}");
                    }
                    KeepDrop::DropLowest(n) => {
                        let _ = write!(out, "dl{n}");
                    }
                }
                out.push_str("\",");
            }
            out.push_str("\"rolls\":[");
            for (j, r) in rolls.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{r}");
            }
            out.push_str("],\"kept\":[");
            for (j, &k) in kept.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str(if k { "true" } else { "false" });
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
    /// Dropped dice are shown in curly braces, e.g. `4d6dl1[5,4,3,{1}]`.
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
    fn evaluate_drop_lowest_total_within_bounds() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6dl1", &mut rng).unwrap();
        // drop 1 lowest → sum of best 3d6 in [3, 18]
        assert!((3..=18).contains(&r.total));
    }

    #[test]
    fn evaluate_drop_lowest_keeps_correct_count() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6dl1", &mut rng).unwrap();
        if let EvalTermKind::Dice { kept, .. } = &r.terms[0].kind {
            assert_eq!(kept.iter().filter(|&&k| k).count(), 3);
            assert_eq!(kept.iter().filter(|&&k| !k).count(), 1);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_keep_highest_picks_max() {
        // with only 2 sides, we can verify the kept die is the max
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d2kh1", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            let max_roll = *rolls.iter().max().unwrap();
            let kept_roll: u64 = rolls
                .iter()
                .zip(kept.iter())
                .map(|(r, k)| if *k { *r } else { 0 })
                .sum();
            assert_eq!(kept_roll, max_roll);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_keep_lowest_picks_min() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d2kl1", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            let min_roll = *rolls.iter().min().unwrap();
            let kept_roll: u64 = rolls
                .iter()
                .zip(kept.iter())
                .map(|(r, k)| if *k { *r } else { 0 })
                .sum();
            assert_eq!(kept_roll, min_roll);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_drop_lowest_display_shows_modifier_and_braces() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6dl1", &mut rng).unwrap();
        let d = r.display();
        assert!(d.starts_with("4d6dl1["), "got: {d}");
        assert!(d.contains('{'), "dropped die not in braces: {d}");
        assert!(d.contains('}'), "dropped die not in braces: {d}");
    }

    #[test]
    fn evaluate_no_modifier_display_unchanged() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6", &mut rng).unwrap();
        let d = r.display();
        assert!(d.starts_with("4d6["), "got: {d}");
        assert!(!d.contains('{'));
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
    fn subtotals_sum_to_total_with_keep_drop() {
        let mut rng = StdRng::seed_from_u64(77);
        let r = run("4d6dl1+2d8kh1", &mut rng).unwrap();
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
        assert!(json.contains(r#""kept":["#));
        assert!(json.contains(r#""kind":"const""#));
        assert!(json.contains(r#""value":3"#));
        assert!(json.ends_with("]}"));
    }

    #[test]
    fn json_output_for_dice_with_modifier_has_modifier_field() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("4d6dl1", &mut rng).unwrap();
        let json = r.json();
        assert!(json.contains(r#""modifier":"dl1""#));
        assert!(json.contains(r#""kept":["#));
        // one false in the kept array
        assert!(json.contains("false"));
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
