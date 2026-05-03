use rand::{Rng, RngExt};

use crate::model::{DiceModifier, DiceSides, KeepDrop, Term};
use crate::parser::{ParseError, parse};

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct EvalResult {
    pub total: i64,
    pub terms: Vec<EvalTerm>,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct EvalTerm {
    pub sign: i64,
    #[serde(flatten)]
    pub kind: EvalTermKind,
    pub subtotal: i64,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EvalTermKind {
    Dice {
        count: u64,
        sides: DiceSides,
        #[serde(
            rename = "modifier",
            skip_serializing_if = "Option::is_none",
            serialize_with = "crate::model::serialize_dice_modifiers"
        )]
        modifier: Option<Vec<DiceModifier>>,
        rolls: Vec<i64>,
        /// Parallel to `rolls`; `false` means the die was dropped.
        kept: Vec<bool>,
    },
    Const {
        value: u64,
    },
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

// Sorts `indices` by roll value, marks the first `k` as `mark` (opposite of `default`).
fn keep_mask(rolls: &[i64], k: usize, descending: bool, default: bool) -> Vec<bool> {
    let n = rolls.len();
    let mut indices: Vec<usize> = (0..n).collect();
    if descending {
        indices.sort_by(|&a, &b| rolls[b].cmp(&rolls[a]));
    } else {
        indices.sort_by(|&a, &b| rolls[a].cmp(&rolls[b]));
    }
    let mut mask = vec![default; n];
    for &i in indices.iter().take(k) {
        mask[i] = !default;
    }
    mask
}

fn apply_keep_drop(keep_drop: Option<&KeepDrop>, rolls: &[i64]) -> Vec<bool> {
    let n = rolls.len();
    let Some(kd) = keep_drop else {
        return vec![true; n];
    };
    match kd {
        KeepDrop::KeepHighest(k) => keep_mask(rolls, *k as usize, true, false),
        KeepDrop::KeepLowest(k) => keep_mask(rolls, *k as usize, false, false),
        KeepDrop::DropHighest(k) => keep_mask(rolls, *k as usize, true, true),
        KeepDrop::DropLowest(k) => keep_mask(rolls, *k as usize, false, true),
    }
}

fn clamp_i64_from_u64(n: u64) -> i64 {
    n.min(i64::MAX as u64) as i64
}

fn roll_once(rng: &mut impl Rng, sides: &DiceSides) -> i64 {
    match sides {
        DiceSides::Numeric(n) => rng.random_range(1..=(*n as i64)),
        DiceSides::Fate => rng.random_range(0..=2) as i64 - 1,
    }
}

fn is_natural_minimum(roll: i64, sides: &DiceSides) -> bool {
    match sides {
        DiceSides::Numeric(_) => roll == 1,
        DiceSides::Fate => roll == -1,
    }
}

fn is_natural_maximum(roll: i64, sides: &DiceSides) -> bool {
    match sides {
        DiceSides::Numeric(n) => roll == *n as i64,
        DiceSides::Fate => roll == 1,
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
                modifier,
            } => {
                let count = *count;
                let sides = sides.clone();
                let mut rolls: Vec<i64> = Vec::with_capacity(count as usize);
                let mut kept = vec![true; count as usize];
                for _ in 0..count {
                    rolls.push(roll_once(rng, &sides));
                }
                if let Some(modifiers) = modifier {
                    for modifier in modifiers {
                        match modifier {
                            DiceModifier::Reroll => {
                                for roll in &mut rolls {
                                    while is_natural_minimum(*roll, &sides) {
                                        *roll = roll_once(rng, &sides);
                                    }
                                }
                            }
                            DiceModifier::RerollOnce => {
                                for roll in &mut rolls {
                                    if is_natural_minimum(*roll, &sides) {
                                        *roll = roll_once(rng, &sides);
                                    }
                                }
                            }
                            DiceModifier::Min(min) => {
                                let min = clamp_i64_from_u64(*min);
                                for roll in &mut rolls {
                                    *roll = (*roll).max(min);
                                }
                            }
                            DiceModifier::Max(max) => {
                                let max = clamp_i64_from_u64(*max);
                                for roll in &mut rolls {
                                    *roll = (*roll).min(max);
                                }
                            }
                            DiceModifier::KeepDrop(kd) => {
                                kept = apply_keep_drop(Some(kd), &rolls);
                            }
                            DiceModifier::Exploding => {
                                for roll in &mut rolls {
                                    let mut last = *roll;
                                    while is_natural_maximum(last, &sides) {
                                        let extra = roll_once(rng, &sides);
                                        *roll += extra;
                                        last = extra;
                                    }
                                }
                            }
                            DiceModifier::CountMatching(comp) => {
                                for (roll, k) in rolls.iter().zip(kept.iter_mut()) {
                                    if *k && !comp.test(*roll) {
                                        *k = false;
                                    }
                                }
                            }
                        }
                    }
                }
                let is_count_mode = modifier.as_ref().is_some_and(|mods| {
                    matches!(mods.last(), Some(DiceModifier::CountMatching(_)))
                });
                let sum: i64 = if is_count_mode {
                    kept.iter().filter(|&&k| k).count() as i64
                } else {
                    rolls
                        .iter()
                        .zip(kept.iter())
                        .map(|(r, k)| if *k { *r } else { 0 })
                        .sum()
                };
                (
                    EvalTermKind::Dice {
                        count,
                        sides: sides.clone(),
                        modifier: modifier.clone(),
                        rolls,
                        kept,
                    },
                    sign * sum,
                )
            }
            Term::Const(n) => (EvalTermKind::Const { value: *n }, sign * *n as i64),
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn sum_kept(rolls: &[i64], kept: &[bool]) -> i64 {
        rolls
            .iter()
            .zip(kept)
            .map(|(r, &k)| if k { *r } else { 0 })
            .sum()
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
            assert_eq!(sum_kept(rolls, kept), *rolls.iter().max().unwrap());
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_keep_lowest_picks_min() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d2kl1", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(sum_kept(rolls, kept), *rolls.iter().min().unwrap());
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_min_clamps_rolls_before_sum() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d6min3", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert!(rolls.iter().all(|&r| r >= 3));
            assert!(kept.iter().all(|&k| k));
            assert!((12..=24).contains(&r.total));
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_max_clamps_rolls_before_sum() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d6max4", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert!(rolls.iter().all(|&r| r <= 4));
            assert!(kept.iter().all(|&k| k));
            assert!((4..=16).contains(&r.total));
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_combined_modifiers_apply_in_order() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d6min3kl4", &mut rng).unwrap();
        if let EvalTermKind::Dice {
            modifier,
            rolls,
            kept,
            ..
        } = &r.terms[0].kind
        {
            assert!(matches!(modifier.as_ref().map(|mods| mods.len()), Some(2)));
            assert!(rolls.iter().all(|&r| r >= 3));
            assert_eq!(kept.iter().filter(|&&k| k).count(), 4);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_exploding_keeps_die_count_and_all_kept() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d6!", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(rolls.len(), 4);
            assert!(kept.iter().all(|&k| k));
            assert!(rolls.iter().all(|&r| r >= 1));
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_exploding_rolls_exceed_sides_when_chained() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = run("100d2!", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, .. } = &r.terms[0].kind {
            assert!(
                rolls.iter().any(|&r| r > 2),
                "expected at least one chained explosion in 100d2!"
            );
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_reroll_replaces_minimum_results() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d2r", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert!(rolls.iter().all(|&r| r == 2));
            assert!(kept.iter().all(|&k| k));
            assert_eq!(r.total, 8);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_count_successes_counts_matching_dice() {
        let mut rng = StdRng::seed_from_u64(0);
        // 8d6c>3: count dice that rolled > 3
        let r = run("8d6c>3", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(rolls.len(), 8);
            let expected: i64 = rolls.iter().filter(|&&v| v > 3).count() as i64;
            assert_eq!(r.total, expected);
            // kept matches the condition
            for (&roll, &k) in rolls.iter().zip(kept.iter()) {
                assert_eq!(k, roll > 3);
            }
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_count_successes_gte() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("8d6c>=4", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            for (&roll, &k) in rolls.iter().zip(kept.iter()) {
                assert_eq!(k, roll >= 4);
            }
            let expected: i64 = rolls.iter().filter(|&&v| v >= 4).count() as i64;
            assert_eq!(r.total, expected);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_count_failures_counts_matching_dice() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("8d6c<2", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            let expected: i64 = rolls.iter().filter(|&&v| v < 2).count() as i64;
            assert_eq!(r.total, expected);
            for (&roll, &k) in rolls.iter().zip(kept.iter()) {
                assert_eq!(k, roll < 2);
            }
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_count_successes_total_in_range() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = run("100d6c>3", &mut rng).unwrap();
        assert!((0..=100).contains(&r.total));
    }

    #[test]
    fn evaluate_count_with_keep_drop_respects_dropped_dice() {
        // dl1 drops the lowest, then c>3 counts matches among remaining 3 dice
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4d6dl1c>3", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(rolls.len(), 4);
            // at most 3 can be kept (one dropped by dl1)
            assert!(kept.iter().filter(|&&k| k).count() <= 3);
            // total is the count of kept dice
            assert_eq!(r.total, kept.iter().filter(|&&k| k).count() as i64);
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_reroll_once_rerolls_minimum_at_most_once() {
        // With a d2, ro rerolls a 1 exactly once — the result may still be 1.
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("100d2ro", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(rolls.len(), 100);
            assert!(kept.iter().all(|&k| k));
            assert!(rolls.iter().all(|&r| (1..=2).contains(&r)));
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_reroll_once_does_not_loop() {
        // d2ro with a fixed seed that would keep producing 1s — ro stops after one reroll,
        // so unlike r it cannot loop forever.
        let mut rng = StdRng::seed_from_u64(42);
        let r = run("4d2ro", &mut rng).unwrap();
        if let EvalTermKind::Dice { rolls, kept, .. } = &r.terms[0].kind {
            assert_eq!(rolls.len(), 4);
            assert!(kept.iter().all(|&k| k));
        } else {
            panic!("expected Dice term");
        }
    }

    #[test]
    fn evaluate_fate_dice_range() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("4dF", &mut rng).unwrap();
        if let EvalTermKind::Dice {
            rolls, kept, sides, ..
        } = &r.terms[0].kind
        {
            assert!(matches!(sides, DiceSides::Fate));
            assert!(rolls.iter().all(|&r| (-1..=1).contains(&r)));
            assert!(kept.iter().all(|&k| k));
            assert!((-4..=4).contains(&r.total));
        } else {
            panic!("expected Dice term");
        }
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
}
