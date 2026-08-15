use std::fmt::Write;

use crate::eval::{EvalResult, EvalTerm, EvalTermKind};
use crate::model::DiceSides;

const ANSI_RED: &str = "\x1b[31m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_RESET: &str = "\x1b[0m";

/// How a [`Span`] is emphasised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpanStyle {
    /// Ordinary text: notation, operators, brackets, kept/dropped markers.
    Plain,
    /// A natural 1 on a numeric die.
    #[serde(rename = "nat-1")]
    Nat1,
    /// The highest face of a numeric die.
    NatMax,
    /// The grand total at the end of a line.
    Total,
}

/// A run of output text carrying a single style.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

/// Accumulates spans, coalescing consecutive plain text so a line arrives as a
/// handful of spans rather than one per token.
#[derive(Debug, Default)]
struct SpanBuilder {
    spans: Vec<Span>,
}

impl SpanBuilder {
    fn plain(&mut self, text: &str) {
        match self.spans.last_mut() {
            Some(last) if last.style == SpanStyle::Plain => last.text.push_str(text),
            _ => self.spans.push(Span {
                text: text.to_owned(),
                style: SpanStyle::Plain,
            }),
        }
    }

    fn styled(&mut self, text: &str, style: SpanStyle) {
        if style == SpanStyle::Plain {
            self.plain(text);
        } else {
            self.spans.push(Span {
                text: text.to_owned(),
                style,
            });
        }
    }
}

// Lets the traversal below build plain runs with `write!`.
impl Write for SpanBuilder {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.plain(s);
        Ok(())
    }
}

fn roll_style(roll: i64, sides: &DiceSides) -> SpanStyle {
    match sides {
        // Fate dice have no natural 1 or critical, so they stay plain
        DiceSides::Numeric(_) if roll == 1 => SpanStyle::Nat1,
        DiceSides::Numeric(n) if roll == *n as i64 => SpanStyle::NatMax,
        _ => SpanStyle::Plain,
    }
}

fn push_terms(out: &mut SpanBuilder, terms: &[EvalTerm]) {
    for (idx, term) in terms.iter().enumerate() {
        out.plain(if term.sign < 0 {
            " - "
        } else if idx == 0 {
            ""
        } else {
            " + "
        });
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
                out.plain("[");
                for (i, (r, &k)) in rolls.iter().zip(kept.iter()).enumerate() {
                    if i > 0 {
                        out.plain(",");
                    }
                    if !k {
                        out.plain("{");
                    }
                    out.styled(&r.to_string(), roll_style(*r, sides));
                    if !k {
                        out.plain("}");
                    }
                }
                out.plain("]");
            }
            EvalTermKind::Const { value: n } => {
                let _ = write!(out, "{n}");
            }
            EvalTermKind::Group {
                terms: inner,
                multiplier,
            } => {
                out.plain("(");
                push_terms(out, inner);
                out.plain(")");
                if *multiplier != 1 {
                    let _ = write!(out, " * {multiplier}");
                }
            }
        }
    }
}

/// Renders spans back to a string. When `color` is true, natural 1s are red and
/// max rolls are green (ANSI); every other style renders as bare text.
pub fn render(spans: &[Span], color: bool) -> String {
    let mut out = String::new();
    for span in spans {
        match span.style {
            SpanStyle::Nat1 if color => {
                let _ = write!(out, "{ANSI_RED}{}{ANSI_RESET}", span.text);
            }
            SpanStyle::NatMax if color => {
                let _ = write!(out, "{ANSI_GREEN}{}{ANSI_RESET}", span.text);
            }
            _ => out.push_str(&span.text),
        }
    }
    out
}

impl EvalResult {
    /// The breakdown as styled spans.
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    /// use diceroll::SpanStyle;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let spans = diceroll::run("3+4", &mut rng).unwrap().spans();
    /// assert_eq!(spans.len(), 1);
    /// assert_eq!(spans[0].text, "3 + 4");
    /// assert_eq!(spans[0].style, SpanStyle::Plain);
    /// ```
    pub fn spans(&self) -> Vec<Span> {
        let mut out = SpanBuilder::default();
        push_terms(&mut out, &self.terms);
        out.spans
    }

    /// The full line, `<breakdown> = <total>`, as styled spans.
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    /// use diceroll::SpanStyle;
    ///
    /// let mut rng = StdRng::seed_from_u64(0);
    /// let spans = diceroll::run("3+4", &mut rng).unwrap().line_spans();
    /// let last = spans.last().unwrap();
    /// assert_eq!((last.text.as_str(), last.style), ("7", SpanStyle::Total));
    /// ```
    pub fn line_spans(&self) -> Vec<Span> {
        let mut out = SpanBuilder::default();
        push_terms(&mut out, &self.terms);
        out.plain(" = ");
        out.styled(&self.total.to_string(), SpanStyle::Total);
        out.spans
    }

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
        render(&self.spans(), color)
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
            render(&self.line_spans(), color)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::{Span, SpanStyle, render};
    use crate::eval::run;

    fn styled(spans: &[Span], style: SpanStyle) -> Vec<&str> {
        spans
            .iter()
            .filter(|s| s.style == style)
            .map(|s| s.text.as_str())
            .collect()
    }

    #[test]
    fn spans_coalesce_consecutive_plain_text() {
        let mut rng = StdRng::seed_from_u64(0);
        let spans = run("3+4-1", &mut rng).unwrap().spans();
        assert_eq!(spans.len(), 1, "got: {spans:?}");
        assert_eq!(spans[0].text, "3 + 4 - 1");
    }

    #[test]
    fn spans_mark_nat_1_and_nat_max() {
        // Every face of a d2 is either the natural 1 or the max, so 20 rolls
        // are all styled and both styles are certain to show up
        let mut rng = StdRng::seed_from_u64(3);
        let spans = run("20d2", &mut rng).unwrap().spans();
        let marked = spans.iter().filter(|s| s.style != SpanStyle::Plain).count();
        assert_eq!(marked, 20, "got: {spans:?}");
        assert!(styled(&spans, SpanStyle::Nat1).iter().all(|t| *t == "1"));
        assert!(styled(&spans, SpanStyle::NatMax).iter().all(|t| *t == "2"));
        assert!(
            !styled(&spans, SpanStyle::Nat1).is_empty(),
            "got: {spans:?}"
        );
        assert!(
            !styled(&spans, SpanStyle::NatMax).is_empty(),
            "got: {spans:?}"
        );
    }

    #[test]
    fn spans_leave_middling_rolls_plain() {
        // Whatever the seed, only the extremes of a d6 may carry a style
        let mut rng = StdRng::seed_from_u64(1);
        let spans = run("30d6", &mut rng).unwrap().spans();
        assert!(styled(&spans, SpanStyle::Nat1).iter().all(|t| *t == "1"));
        assert!(styled(&spans, SpanStyle::NatMax).iter().all(|t| *t == "6"));
    }

    #[test]
    fn spans_leave_fate_dice_plain() {
        let mut rng = StdRng::seed_from_u64(1);
        let spans = run("4dF", &mut rng).unwrap().spans();
        assert!(
            spans.iter().all(|s| s.style == SpanStyle::Plain),
            "got: {spans:?}"
        );
    }

    #[test]
    fn spans_keep_dropped_dice_braces_plain() {
        let mut rng = StdRng::seed_from_u64(1);
        let spans = run("4d6dl1", &mut rng).unwrap().spans();
        let plain: String = spans
            .iter()
            .filter(|s| s.style == SpanStyle::Plain)
            .map(|s| s.text.as_str())
            .collect();
        assert!(plain.contains('{') && plain.contains('}'), "got: {spans:?}");
    }

    #[test]
    fn line_spans_end_with_the_total() {
        let mut rng = StdRng::seed_from_u64(0);
        let r = run("3+4-1", &mut rng).unwrap();
        let spans = r.line_spans();
        assert_eq!(styled(&spans, SpanStyle::Total), [r.total.to_string()]);
        assert_eq!(render(&spans, false), "3 + 4 - 1 = 6");
    }

    #[test]
    fn rendering_spans_agrees_with_the_string_output() {
        for expr in ["3+4-1", "4d6dl1", "3d2", "4dF", "(2d6+3)*2", "8d6c>3"] {
            let mut rng = StdRng::seed_from_u64(11);
            let r = run(expr, &mut rng).unwrap();
            assert_eq!(render(&r.spans(), false), r.display(false), "{expr}");
            assert_eq!(render(&r.spans(), true), r.display(true), "{expr}");
            assert_eq!(
                render(&r.line_spans(), false),
                r.formatted(false, false),
                "{expr}"
            );
        }
    }

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
    fn display_exploding_shows_bang_modifier() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = run("4d6!", &mut rng).unwrap();
        let d = r.display(false);
        assert!(d.starts_with("4d6!["), "got: {d}");
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
