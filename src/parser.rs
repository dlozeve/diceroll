use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum Term {
    Dice { count: u64, sides: u64 },
    Const(u64),
}

const MAX_DICE_COUNT: u64 = 1_000_000;

static TERM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?<sign>[+-]?)(?:(?<count>\d*)[dD](?<sides>\d+)|(?<num>\d+))").unwrap()
});

/// Parses a dice expression into a list of `(sign, Term)` pairs.
///
/// Signs are `1` or `-1`. Whitespace is ignored. The leading sign of the
/// first term is always explicit in the result (`+` → `1`, `-` → `-1`,
/// absent → `1`).
///
/// # Errors
///
/// Returns an error string if the expression is empty, contains unknown
/// characters, or violates constraints (zero dice, fewer than 2 sides, etc.).
///
/// # Examples
///
/// ```
/// use diceroll::parser::{parse, Term};
///
/// let terms = parse("2d6+3").unwrap();
/// assert_eq!(terms, vec![
///     (1,  Term::Dice { count: 2, sides: 6 }),
///     (1,  Term::Const(3)),
/// ]);
///
/// let terms = parse("4d6 - 1").unwrap();
/// assert_eq!(terms[1], (-1, Term::Const(1)));
///
/// assert!(parse("").is_err());
/// assert!(parse("0d6").is_err());
/// ```
pub fn parse(input: &str) -> Result<Vec<(i64, Term)>, String> {
    let s: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return Err("empty expression".into());
    }

    let mut terms = Vec::new();
    let mut pos = 0;

    for caps in TERM_RE.captures_iter(&s) {
        let m = caps.get(0).unwrap();
        if m.start() != pos {
            return Err(format!("unexpected input: '{}'", &s[pos..m.start()]));
        }
        let sign_str = caps.name("sign").unwrap().as_str();
        if pos > 0 && sign_str.is_empty() {
            return Err("missing '+' or '-' between terms".into());
        }
        let sign: i64 = if sign_str == "-" { -1 } else { 1 };

        let term = if let Some(sides_m) = caps.name("sides") {
            let count_str = caps.name("count").unwrap().as_str();
            let count: u64 = if count_str.is_empty() {
                1
            } else {
                count_str
                    .parse()
                    .map_err(|_| format!("invalid dice count: '{count_str}'"))?
            };
            let sides: u64 = sides_m
                .as_str()
                .parse()
                .map_err(|_| format!("invalid dice sides: '{}'", sides_m.as_str()))?;
            if count == 0 {
                return Err("must roll at least 1 die".into());
            }
            if count > MAX_DICE_COUNT {
                return Err(format!("dice count exceeds maximum of {MAX_DICE_COUNT}"));
            }
            if sides < 2 {
                return Err("dice must have at least 2 sides".into());
            }
            Term::Dice { count, sides }
        } else {
            let num_str = caps.name("num").unwrap().as_str();
            Term::Const(num_str.parse().map_err(|_| format!("invalid number: '{num_str}'"))?)
        };

        terms.push((sign, term));
        pos = m.end();
    }

    if pos != s.len() {
        return Err(format!("unexpected trailing input: '{}'", &s[pos..]));
    }
    if terms.is_empty() {
        return Err("no terms found".into());
    }

    Ok(terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dice(count: u64, sides: u64) -> Term {
        Term::Dice { count, sides }
    }

    #[test]
    fn parse_single_die_with_implicit_count() {
        assert_eq!(parse("d20").unwrap(), vec![(1, dice(1, 20))]);
        assert_eq!(parse("1d20").unwrap(), vec![(1, dice(1, 20))]);
    }

    #[test]
    fn parse_full_expression() {
        assert_eq!(
            parse("2d20+3d6+4").unwrap(),
            vec![(1, dice(2, 20)), (1, dice(3, 6)), (1, Term::Const(4))],
        );
    }

    #[test]
    fn parse_subtraction() {
        assert_eq!(
            parse("4d6-1").unwrap(),
            vec![(1, dice(4, 6)), (-1, Term::Const(1))],
        );
    }

    #[test]
    fn parse_leading_sign() {
        assert_eq!(
            parse("-1d4+10").unwrap(),
            vec![(-1, dice(1, 4)), (1, Term::Const(10))],
        );
        assert_eq!(parse("+5").unwrap(), vec![(1, Term::Const(5))]);
    }

    #[test]
    fn parse_uppercase_d() {
        assert_eq!(parse("2D6+1").unwrap(), parse("2d6+1").unwrap());
    }

    #[test]
    fn parse_ignores_whitespace() {
        assert_eq!(
            parse("  2d20 + 3d6 + 4 ").unwrap(),
            parse("2d20+3d6+4").unwrap(),
        );
    }

    #[test]
    fn parse_constants_only() {
        assert_eq!(
            parse("3+4-1").unwrap(),
            vec![
                (1, Term::Const(3)),
                (1, Term::Const(4)),
                (-1, Term::Const(1)),
            ],
        );
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse("foo").is_err());
        assert!(parse("2d6+foo").is_err());
    }

    #[test]
    fn parse_rejects_trailing_operator() {
        assert!(parse("2d6+").is_err());
        assert!(parse("2d6-").is_err());
    }

    #[test]
    fn parse_rejects_zero_dice() {
        assert!(parse("0d6").is_err());
    }

    #[test]
    fn parse_rejects_one_sided_dice() {
        assert!(parse("2d1").is_err());
        assert!(parse("2d0").is_err());
    }

    #[test]
    fn parse_rejects_missing_operator() {
        // "2d6 3d6" → after stripping whitespace becomes "2d63d6";
        // first match consumes "2d63", second has empty sign
        assert!(parse("2d6 3d6").is_err());
    }

    #[test]
    fn parse_rejects_partial_dice() {
        assert!(parse("2d6+5d").is_err());
    }

    #[test]
    fn parse_rejects_excessive_dice_count() {
        assert!(parse(&format!("{}d6", MAX_DICE_COUNT + 1)).is_err());
        assert!(parse(&format!("{}d6", MAX_DICE_COUNT)).is_ok());
    }
}
