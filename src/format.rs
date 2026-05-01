use std::fmt::Write;

use crate::eval::{EvalResult, EvalTerm, EvalTermKind};
use crate::parser::DiceSides;

const ANSI_RED: &str = "\x1b[31m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_RESET: &str = "\x1b[0m";

fn format_roll(r: i64, sides: &DiceSides, color: bool) -> String {
    match sides {
        DiceSides::Numeric(n) if color && r == 1 => format!("{ANSI_RED}{r}{ANSI_RESET}"),
        DiceSides::Numeric(n) if color && r == *n as i64 => {
            format!("{ANSI_GREEN}{r}{ANSI_RESET}")
        }
        _ => format!("{r}"),
    }
}

fn format_terms(terms: &[EvalTerm], color: bool) -> String {
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
                modifier,
                rolls,
                kept,
            } => {
                let _ = write!(out, "{count}d{sides}");
                if let Some(modifiers) = modifier {
                    for modifier in modifiers {
                        let _ = write!(out, "{modifier}");
                    }
                }
                out.push('[');
                for (i, (r, &k)) in rolls.iter().zip(kept.iter()).enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let formatted = format_roll(*r, sides, color);
                    if k {
                        out.push_str(&formatted);
                    } else {
                        let _ = write!(out, "{{{formatted}}}");
                    }
                }
                out.push(']');
            }
            EvalTermKind::Const { value: n } => {
                let _ = write!(out, "{n}");
            }
            EvalTermKind::Group {
                terms: inner,
                multiplier,
            } => {
                out.push('(');
                out.push_str(&format_terms(inner, color));
                out.push(')');
                if *multiplier != 1 {
                    let _ = write!(out, " * {multiplier}");
                }
            }
        }
    }
    out
}

impl EvalResult {
    /// Returns a human-readable breakdown: each term with its rolls, then the total.
    /// Dropped dice are shown in curly braces, e.g. `4d6dl1[5,4,3,{1}]`.
    /// When `color` is true, nat-1 rolls are red and nat-max rolls are green (ANSI).
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let result = diceroll::run("3+4-1", &mut rng).unwrap();
    /// assert_eq!(result.display(false), "3 + 4 - 1");
    /// ```
    pub fn display(&self, color: bool) -> String {
        format_terms(&self.terms, color)
    }

    pub fn json(&self) -> String {
        #[allow(clippy::expect_used)]
        serde_json::to_string(self).expect("infallible: no floats, no non-string map keys")
    }

    /// Returns the plain display string (`"<breakdown> = <total>"`) or JSON.
    /// When `color` is true (ignored for JSON), nat-1 rolls are red and nat-max rolls are green.
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let result = diceroll::run("3+4-1", &mut rng).unwrap();
    /// assert_eq!(result.formatted(false, false), "3 + 4 - 1 = 6");
    /// let json = result.formatted(true, false);
    /// assert!(json.starts_with(r#"{"total":6"#));
    /// ```
    pub fn formatted(&self, json: bool, color: bool) -> String {
        if json {
            self.json()
        } else {
            format!("{} = {}", self.display(color), self.total)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::eval::run;

    #[test]
    fn display_constants() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("3+4-1", &mut rng).unwrap();
        assert_eq!(r.display(false), "3 + 4 - 1");
        assert_eq!(r.total, 6);
    }

    #[test]
    fn display_lists_each_roll() {
        let mut rng = StdRng::seed_from_u64(1);
        let out = run("3d6+2", &mut rng).unwrap().display(false);
        let bracket = &out[out.find('[').unwrap()..=out.find(']').unwrap()];
        assert_eq!(bracket.matches(',').count(), 2);
        assert!(out.starts_with("3d6["));
        assert!(out.contains("] + 2"));
    }

    #[test]
    fn display_drop_lowest_shows_modifier_and_braces() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6dl1", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6dl1["), "got: {d}");
        assert!(d.contains('{'), "dropped die not in braces: {d}");
        assert!(d.contains('}'), "dropped die not in braces: {d}");
    }

    #[test]
    fn display_no_modifier_no_braces() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6["), "got: {d}");
        assert!(!d.contains('{'));
    }

    #[test]
    fn display_min_shows_modifier() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6min3", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6min3["), "got: {d}");
        assert!(!d.contains('{'));
    }

    #[test]
    fn display_max_shows_modifier() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6max4", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6max4["), "got: {d}");
        assert!(!d.contains('{'));
    }

    #[test]
    fn display_reroll_shows_modifier() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6r", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6r["), "got: {d}");
    }

    #[test]
    fn display_fate_dice_uses_d_f_notation() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4dF", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4dF["), "got: {d}");
        assert!(d.contains("-1") || d.contains("0") || d.contains("1"));
    }

    #[test]
    fn display_combined_modifiers_concatenates_them() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6rmin3kl4", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6rmin3kl4["), "got: {d}");
    }

    #[test]
    fn display_group() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("(2d6+3)*2", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with('('));
        assert!(d.contains("2d6["));
        assert!(d.contains("] + 3)"));
        assert!(d.ends_with(" * 2"));
    }

    #[test]
    fn display_group_no_multiplier() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("(d6)", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with('('));
        assert!(d.ends_with(')'));
        assert!(!d.contains('*'));
    }

    #[test]
    fn color_nat1_and_max_are_wrapped() {
        // d1 always rolls 1, which is also the max — use d2 to distinguish
        // With seed 0 on 1d2, verify coloring is applied to the single roll
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("1d2", &mut rng).unwrap();
        let colored = r.display(true);
        let plain = r.display(false);
        // colored output must contain an ANSI escape; plain must not
        assert!(colored.contains("\x1b["), "expected ANSI codes: {colored}");
        assert!(
            !plain.contains("\x1b["),
            "unexpected ANSI in plain: {plain}"
        );
    }

    #[test]
    fn color_false_matches_plain() {
        let mut rng_a = StdRng::seed_from_u64(5);
        let mut rng_b = StdRng::seed_from_u64(5);
        let a = run("4d6dl1", &mut rng_a).unwrap().display(false);
        let b = run("4d6dl1", &mut rng_b).unwrap().display(false);
        assert_eq!(a, b);
    }

    #[test]
    fn json_output_for_constants_is_exact() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("3+4-1", &mut rng).unwrap();
        assert_eq!(
            r.json(),
            r#"{"total":6,"terms":[{"sign":1,"kind":"const","value":3,"subtotal":3},{"sign":1,"kind":"const","value":4,"subtotal":4},{"sign":-1,"kind":"const","value":1,"subtotal":-1}]}"#
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
    fn json_output_for_dice_with_min_modifier_has_modifier_field() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("4d6min3", &mut rng).unwrap();
        let json = r.json();
        assert!(json.contains(r#""modifier":"min3""#));
        assert!(json.contains(r#""rolls":["#));
    }

    #[test]
    fn json_output_for_dice_with_combined_modifiers_is_array() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("4d6rmin3kl4", &mut rng).unwrap();
        let json = r.json();
        assert!(
            json.contains(r#""modifier":["r","min3","kl4"]"#),
            "got: {json}"
        );
    }

    #[test]
    fn json_output_for_fate_dice_has_fate_sides() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = run("4dF", &mut rng).unwrap();
        let json = r.json();
        assert!(json.contains(r#""sides":"F""#), "got: {json}");
        assert!(json.contains(r#""rolls":["#));
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
