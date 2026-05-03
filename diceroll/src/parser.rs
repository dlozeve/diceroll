use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, digit1, multispace0, one_of},
    combinator::opt,
    multi::many0,
};

use crate::model::{DiceModifier, DiceSides, KeepDrop, Term};

/// Typed parse errors returned by [`parse`].
///
/// # Examples
///
/// ```
/// use diceroll::parser::{parse, ParseError};
///
/// let err = parse("0d6").unwrap_err();
/// assert!(matches!(err, ParseError::ZeroDice));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("empty expression")]
    Empty,
    #[error("unexpected input: '{token}'")]
    Unexpected { token: String },
    #[error("unexpected trailing input: '{rest}'")]
    UnexpectedTrailing { rest: String },
    #[error("missing '+' or '-' between terms")]
    MissingOperator,
    #[error("must roll at least 1 die")]
    ZeroDice,
    #[error("dice must have at least 2 sides")]
    TooFewSides,
    #[error("dice count exceeds maximum of {max}")]
    DiceCountExceeded { count: u64, max: u64 },
    #[error("modifier {modifier} exceeds dice count {count}")]
    ModifierExceedsDiceCount { count: u64, modifier: u64 },
    #[error("invalid number: '{0}'")]
    InvalidNumber(String),
}

const MAX_DICE_COUNT: u64 = 1_000_000;

type KeepDropCtor = fn(u64) -> KeepDrop;

enum RawAtom<'a> {
    Dice {
        count_str: &'a str,
        sides: RawDiceSides<'a>,
        modifiers: Vec<RawDiceModifier<'a>>,
    },
    Const(&'a str),
    Group {
        inner: &'a str,
        multiplier_str: Option<&'a str>,
    },
}

enum RawDiceModifier<'a> {
    KeepDrop(KeepDropCtor, &'a str),
    Min(&'a str),
    Max(&'a str),
    Reroll,
    RerollOnce,
    Exploding,
}

enum RawDiceSides<'a> {
    Percent,
    Fate,
    Numeric(&'a str),
}

fn sign_to_i64(c: char) -> i64 {
    if c == '-' { -1 } else { 1 }
}

fn parse_sign(input: &str) -> IResult<&str, char> {
    one_of("+-").parse(input)
}

fn parse_dice_modifier(input: &str) -> IResult<&str, RawDiceModifier<'_>> {
    let parse_keep_drop = alt((
        tag("kh").map(|_| KeepDrop::KeepHighest as KeepDropCtor),
        tag("kl").map(|_| KeepDrop::KeepLowest as KeepDropCtor),
        tag("dh").map(|_| KeepDrop::DropHighest as KeepDropCtor),
        tag("dl").map(|_| KeepDrop::DropLowest as KeepDropCtor),
    ));

    let parse_keep_drop = parse_keep_drop
        .and(digit1)
        .map(|(ctor, count_str)| RawDiceModifier::KeepDrop(ctor, count_str));
    let parse_min = tag("min")
        .and(digit1)
        .map(|(_, count_str)| RawDiceModifier::Min(count_str));
    let parse_max = tag("max")
        .and(digit1)
        .map(|(_, count_str)| RawDiceModifier::Max(count_str));
    let parse_reroll_once = tag("ro").map(|_| RawDiceModifier::RerollOnce);
    let parse_reroll = tag("r").map(|_| RawDiceModifier::Reroll);
    let parse_exploding = tag("!").map(|_| RawDiceModifier::Exploding);

    alt((
        parse_keep_drop,
        parse_min,
        parse_max,
        parse_reroll_once,
        parse_reroll,
        parse_exploding,
    ))
    .parse(input)
}

fn parse_dice(input: &str) -> IResult<&str, RawAtom<'_>> {
    let (input, count_str) = digit0(input)?;
    let (input, _) = one_of("dD").parse(input)?;
    let (input, sides) = alt((
        tag("%").map(|_| RawDiceSides::Percent),
        tag("F").map(|_| RawDiceSides::Fate),
        tag("f").map(|_| RawDiceSides::Fate),
        digit1.map(RawDiceSides::Numeric),
    ))
    .parse(input)?;
    let (input, modifiers) = many0(parse_dice_modifier).parse(input)?;
    Ok((
        input,
        RawAtom::Dice {
            count_str,
            sides,
            modifiers,
        },
    ))
}

fn parse_constant(input: &str) -> IResult<&str, RawAtom<'_>> {
    let (input, num_str) = digit1(input)?;
    Ok((input, RawAtom::Const(num_str)))
}

fn parse_multiplier_str(input: &str) -> IResult<&str, &str> {
    let (input, _) = multispace0(input)?;
    let (input, _) = one_of("*").parse(input)?;
    let (input, _) = multispace0(input)?;
    digit1(input)
}

fn parse_group_body(input: &str) -> IResult<&str, &str> {
    let after_open = input.strip_prefix('(').ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Char))
    })?;

    // Find the matching ')' by tracking nesting depth.
    let mut depth = 1usize;
    let mut close_pos = None;
    for (i, c) in after_open.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_pos = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_pos = close_pos.ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Char))
    })?;

    Ok((&after_open[close_pos + 1..], &after_open[..close_pos]))
}

/// Parses `'(' expr ')' ('*' N)?` — suffix multiplier form.
fn parse_group_suffix(input: &str) -> IResult<&str, RawAtom<'_>> {
    let (after_close, inner) = parse_group_body(input)?;
    let (remaining, multiplier_str) = opt(parse_multiplier_str).parse(after_close)?;
    Ok((
        remaining,
        RawAtom::Group {
            inner,
            multiplier_str,
        },
    ))
}

/// Parses `N '*' '(' expr ')'` — prefix multiplier form.
fn parse_group_prefix(input: &str) -> IResult<&str, RawAtom<'_>> {
    let (input, multiplier_str) = digit1(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = one_of("*").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (remaining, inner) = parse_group_body(input)?;
    Ok((
        remaining,
        RawAtom::Group {
            inner,
            multiplier_str: Some(multiplier_str),
        },
    ))
}

fn parse_atom(input: &str) -> IResult<&str, RawAtom<'_>> {
    alt((
        parse_group_prefix,
        parse_group_suffix,
        parse_dice,
        parse_constant,
    ))
    .parse(input)
}

fn parse_first_term(input: &str) -> IResult<&str, (Option<char>, RawAtom<'_>)> {
    let (input, sign) = opt(parse_sign).parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, atom) = parse_atom(input)?;
    Ok((input, (sign, atom)))
}

fn parse_subsequent_term(input: &str) -> IResult<&str, (char, RawAtom<'_>)> {
    let (input, _) = multispace0(input)?;
    let (input, sign) = parse_sign(input)?;
    let (input, _) = multispace0(input)?;
    let (input, atom) = parse_atom(input)?;
    Ok((input, (sign, atom)))
}

fn validate_atom(sign: i64, raw: RawAtom<'_>) -> Result<(i64, Term), ParseError> {
    match raw {
        RawAtom::Dice {
            count_str,
            sides,
            modifiers,
        } => {
            let count: u64 = if count_str.is_empty() {
                1
            } else {
                count_str
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(count_str.to_owned()))?
            };
            if count == 0 {
                return Err(ParseError::ZeroDice);
            }
            if count > MAX_DICE_COUNT {
                return Err(ParseError::DiceCountExceeded {
                    count,
                    max: MAX_DICE_COUNT,
                });
            }
            let sides = match sides {
                RawDiceSides::Percent => DiceSides::Numeric(100),
                RawDiceSides::Fate => DiceSides::Fate,
                RawDiceSides::Numeric(s) => {
                    let n: u64 = s
                        .parse()
                        .map_err(|_| ParseError::InvalidNumber(s.to_owned()))?;
                    if n < 2 {
                        return Err(ParseError::TooFewSides);
                    }
                    DiceSides::Numeric(n)
                }
            };
            let mut parsed_modifiers = Vec::with_capacity(modifiers.len());
            for modifier in modifiers {
                let parsed = match modifier {
                    RawDiceModifier::KeepDrop(ctor, s) => {
                        let n: u64 = s
                            .parse()
                            .map_err(|_| ParseError::InvalidNumber(s.to_owned()))?;
                        if n > count {
                            return Err(ParseError::ModifierExceedsDiceCount {
                                count,
                                modifier: n,
                            });
                        }
                        DiceModifier::KeepDrop(ctor(n))
                    }
                    RawDiceModifier::Min(s) => {
                        let n: u64 = s
                            .parse()
                            .map_err(|_| ParseError::InvalidNumber(s.to_owned()))?;
                        DiceModifier::Min(n)
                    }
                    RawDiceModifier::Max(s) => {
                        let n: u64 = s
                            .parse()
                            .map_err(|_| ParseError::InvalidNumber(s.to_owned()))?;
                        DiceModifier::Max(n)
                    }
                    RawDiceModifier::Reroll => DiceModifier::Reroll,
                    RawDiceModifier::RerollOnce => DiceModifier::RerollOnce,
                    RawDiceModifier::Exploding => DiceModifier::Exploding,
                };
                parsed_modifiers.push(parsed);
            }
            let modifier = if parsed_modifiers.is_empty() {
                None
            } else {
                Some(parsed_modifiers)
            };
            Ok((
                sign,
                Term::Dice {
                    count,
                    sides,
                    modifier,
                },
            ))
        }
        RawAtom::Const(num_str) => {
            let n: u64 = num_str
                .parse()
                .map_err(|_| ParseError::InvalidNumber(num_str.to_owned()))?;
            Ok((sign, Term::Const(n)))
        }
        RawAtom::Group {
            inner,
            multiplier_str,
        } => {
            let multiplier: u64 = match multiplier_str {
                Some(s) => s
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber(s.to_owned()))?,
                None => 1,
            };
            let inner_trimmed = inner.trim();
            if inner_trimmed.is_empty() {
                return Err(ParseError::Empty);
            }
            let terms = parse_nonempty(inner_trimmed)?;
            Ok((sign, Term::Group { terms, multiplier }))
        }
    }
}

fn parse_nonempty(trimmed: &str) -> Result<Vec<(i64, Term)>, ParseError> {
    let (remaining, (sign_ch, first_raw)) =
        parse_first_term(trimmed).map_err(|_| ParseError::Unexpected {
            token: trimmed.to_owned(),
        })?;

    let (remaining, rest_raw) = many0(parse_subsequent_term)
        .parse(remaining)
        .unwrap_or_else(|_| unreachable!("many0 never fails"));

    let trailing = remaining.trim();
    if !trailing.is_empty() {
        if trailing.starts_with(|c: char| c.is_ascii_digit() || c == 'd' || c == 'D' || c == '(') {
            return Err(ParseError::MissingOperator);
        }
        return Err(ParseError::UnexpectedTrailing {
            rest: trailing.to_owned(),
        });
    }

    let mut terms = Vec::with_capacity(1 + rest_raw.len());
    terms.push(validate_atom(sign_ch.map_or(1, sign_to_i64), first_raw)?);
    for (sign_ch, raw) in rest_raw {
        terms.push(validate_atom(sign_to_i64(sign_ch), raw)?);
    }

    Ok(terms)
}

/// Parses a dice expression into a list of `(sign, Term)` pairs.
///
/// Signs are `1` or `-1`. Whitespace is ignored. The leading sign of the
/// first term is always explicit in the result (`+` → `1`, `-` → `-1`,
/// absent → `1`).
///
/// Parenthesised sub-expressions may be scaled with a `*N` suffix.
/// Nesting is supported.
///
/// # Errors
///
/// Returns a [`ParseError`] if the expression is empty, contains unknown
/// characters, or violates constraints (zero dice, fewer than 2 sides, etc.).
///
/// # Examples
///
/// ```
/// use diceroll::parser::parse;
/// use diceroll::{DiceSides, Term};
///
/// let terms = parse("2d6+3").unwrap();
/// assert_eq!(terms, vec![
///     (1,  Term::Dice { count: 2, sides: DiceSides::Numeric(6), modifier: None }),
///     (1,  Term::Const(3)),
/// ]);
///
/// let terms = parse("4d6 - 1").unwrap();
/// assert_eq!(terms[1], (-1, Term::Const(1)));
///
/// let terms = parse("(2d6+3)*2").unwrap();
/// assert!(matches!(&terms[0], (1, Term::Group { multiplier: 2, .. })));
///
/// assert!(parse("").is_err());
/// assert!(parse("0d6").is_err());
/// ```
pub fn parse(input: &str) -> Result<Vec<(i64, Term)>, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    parse_nonempty(trimmed)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn dice(count: u64, sides: u64) -> Term {
        Term::Dice {
            count,
            sides: DiceSides::Numeric(sides),
            modifier: None,
        }
    }

    fn fate_dice(count: u64) -> Term {
        Term::Dice {
            count,
            sides: DiceSides::Fate,
            modifier: None,
        }
    }

    fn dice_kd(count: u64, sides: u64, keep_drop: KeepDrop) -> Term {
        Term::Dice {
            count,
            sides: DiceSides::Numeric(sides),
            modifier: Some(vec![DiceModifier::KeepDrop(keep_drop)]),
        }
    }

    fn dice_mod(count: u64, sides: u64, modifier: DiceModifier) -> Term {
        Term::Dice {
            count,
            sides: DiceSides::Numeric(sides),
            modifier: Some(vec![modifier]),
        }
    }

    fn dice_mods(count: u64, sides: u64, modifiers: Vec<DiceModifier>) -> Term {
        Term::Dice {
            count,
            sides: DiceSides::Numeric(sides),
            modifier: Some(modifiers),
        }
    }

    fn group(terms: Vec<(i64, Term)>, multiplier: u64) -> Term {
        Term::Group { terms, multiplier }
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
    fn parse_percent_sides() {
        assert_eq!(parse("d%").unwrap(), vec![(1, dice(1, 100))]);
        assert_eq!(parse("3d%").unwrap(), vec![(1, dice(3, 100))]);
        assert_eq!(parse("D%").unwrap(), vec![(1, dice(1, 100))]);
    }

    #[test]
    fn parse_fate_dice() {
        assert_eq!(parse("dF").unwrap(), vec![(1, fate_dice(1))]);
        assert_eq!(parse("4df").unwrap(), vec![(1, fate_dice(4))]);
    }

    #[test]
    fn parse_percent_with_modifier() {
        assert_eq!(
            parse("d%kh1").unwrap(),
            vec![(1, dice_kd(1, 100, KeepDrop::KeepHighest(1)))],
        );
    }

    #[test]
    fn parse_min_modifier() {
        assert_eq!(
            parse("4d6min3").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::Min(3)))],
        );
    }

    #[test]
    fn parse_max_modifier() {
        assert_eq!(
            parse("4d6max4").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::Max(4)))],
        );
    }

    #[test]
    fn parse_reroll_modifier() {
        assert_eq!(
            parse("4d6r").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::Reroll))],
        );
    }

    #[test]
    fn parse_reroll_once_modifier() {
        assert_eq!(
            parse("4d6ro").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::RerollOnce))],
        );
    }

    #[test]
    fn parse_reroll_is_not_reroll_once() {
        assert_eq!(
            parse("4d6r").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::Reroll))],
        );
        assert_ne!(
            parse("4d6r").unwrap(),
            parse("4d6ro").unwrap(),
        );
    }

    #[test]
    fn parse_exploding_modifier() {
        assert_eq!(
            parse("4d6!").unwrap(),
            vec![(1, dice_mod(4, 6, DiceModifier::Exploding))],
        );
        assert_eq!(
            parse("d20!").unwrap(),
            vec![(1, dice_mod(1, 20, DiceModifier::Exploding))],
        );
    }

    #[test]
    fn parse_combined_modifiers() {
        assert_eq!(
            parse("4d6rmin3kl4").unwrap(),
            vec![(
                1,
                dice_mods(
                    4,
                    6,
                    vec![
                        DiceModifier::Reroll,
                        DiceModifier::Min(3),
                        DiceModifier::KeepDrop(KeepDrop::KeepLowest(4))
                    ]
                )
            )],
        );
    }

    #[test]
    fn parse_keep_highest() {
        assert_eq!(
            parse("8d4kh3").unwrap(),
            vec![(1, dice_kd(8, 4, KeepDrop::KeepHighest(3)))],
        );
    }

    #[test]
    fn parse_keep_lowest() {
        assert_eq!(
            parse("4d6kl2").unwrap(),
            vec![(1, dice_kd(4, 6, KeepDrop::KeepLowest(2)))],
        );
    }

    #[test]
    fn parse_drop_highest() {
        assert_eq!(
            parse("4d6dh1").unwrap(),
            vec![(1, dice_kd(4, 6, KeepDrop::DropHighest(1)))],
        );
    }

    #[test]
    fn parse_drop_lowest() {
        assert_eq!(
            parse("4d6dl1").unwrap(),
            vec![(1, dice_kd(4, 6, KeepDrop::DropLowest(1)))],
        );
    }

    #[test]
    fn parse_modifier_equals_count_is_ok() {
        assert!(parse("4d6kh4").is_ok());
        assert!(parse("4d6dl4").is_ok());
    }

    #[test]
    fn parse_group_with_multiplier() {
        assert_eq!(
            parse("(2d6+3)*2").unwrap(),
            vec![(1, group(vec![(1, dice(2, 6)), (1, Term::Const(3))], 2))],
        );
    }

    #[test]
    fn parse_group_no_multiplier() {
        assert_eq!(
            parse("(d6)").unwrap(),
            vec![(1, group(vec![(1, dice(1, 6))], 1))],
        );
    }

    #[test]
    fn parse_full_expression_with_group() {
        let terms = parse("d20 + (2d6+3)*2 + 5").unwrap();
        assert_eq!(terms.len(), 3);
        assert_eq!(terms[0], (1, dice(1, 20)));
        assert_eq!(
            terms[1],
            (1, group(vec![(1, dice(2, 6)), (1, Term::Const(3))], 2))
        );
        assert_eq!(terms[2], (1, Term::Const(5)));
    }

    #[test]
    fn parse_group_with_whitespace() {
        assert_eq!(
            parse("( 2d6 + 3 ) * 2").unwrap(),
            parse("(2d6+3)*2").unwrap(),
        );
    }

    #[test]
    fn parse_group_prefix_multiplier() {
        assert_eq!(parse("2*(2d6+3)").unwrap(), parse("(2d6+3)*2").unwrap());
    }

    #[test]
    fn parse_group_prefix_with_whitespace() {
        assert_eq!(parse("2 * (2d6+3)").unwrap(), parse("(2d6+3)*2").unwrap());
    }

    #[test]
    fn parse_rejects_both_prefix_and_suffix_multiplier() {
        assert!(parse("2*(2d6+3)*3").is_err());
    }

    #[test]
    fn parse_group_negative_sign() {
        assert_eq!(
            parse("-(2d6+3)*2").unwrap(),
            vec![(-1, group(vec![(1, dice(2, 6)), (1, Term::Const(3))], 2))],
        );
    }

    #[test]
    fn parse_nested_groups() {
        let terms = parse("((d6+1)*2+3)*4").unwrap();
        assert_eq!(terms.len(), 1);
        let (sign, outer) = &terms[0];
        assert_eq!(*sign, 1);
        if let Term::Group {
            terms: outer_inner,
            multiplier: 4,
        } = outer
        {
            assert_eq!(outer_inner.len(), 2);
            assert_eq!(
                outer_inner[0],
                (1, group(vec![(1, dice(1, 6)), (1, Term::Const(1))], 2))
            );
            assert_eq!(outer_inner[1], (1, Term::Const(3)));
        } else {
            panic!("expected outer Group with multiplier 4");
        }
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

    #[test]
    fn parse_rejects_unmatched_paren() {
        assert!(parse("(2d6+3").is_err());
    }

    #[test]
    fn parse_rejects_empty_group() {
        assert!(parse("()*2").is_err());
        assert!(parse("()").is_err());
    }

    #[test]
    fn parse_rejects_missing_operator_with_group() {
        assert!(parse("2d6 (d4)").is_err());
        assert!(parse("(2d6+3) (d4+1)").is_err());
    }

    // Typed variant tests — pin the exact error for each failure mode.

    #[test]
    fn error_empty() {
        assert_eq!(parse(""), Err(ParseError::Empty));
        assert_eq!(parse("   "), Err(ParseError::Empty));
    }

    #[test]
    fn error_unexpected() {
        assert!(matches!(parse("foo"), Err(ParseError::Unexpected { .. })));
    }

    #[test]
    fn error_unexpected_trailing() {
        assert!(matches!(
            parse("2d6+foo"),
            Err(ParseError::UnexpectedTrailing { .. })
        ));
    }

    #[test]
    fn error_missing_operator() {
        assert_eq!(parse("2d6 3d6"), Err(ParseError::MissingOperator));
    }

    #[test]
    fn error_zero_dice() {
        assert_eq!(parse("0d6"), Err(ParseError::ZeroDice));
    }

    #[test]
    fn error_too_few_sides() {
        assert_eq!(parse("2d1"), Err(ParseError::TooFewSides));
        assert_eq!(parse("2d0"), Err(ParseError::TooFewSides));
    }

    #[test]
    fn error_dice_count_exceeded() {
        let expr = format!("{}d6", MAX_DICE_COUNT + 1);
        assert!(matches!(
            parse(&expr),
            Err(ParseError::DiceCountExceeded {
                count: _,
                max: MAX_DICE_COUNT
            })
        ));
    }

    #[test]
    fn error_modifier_exceeds_count() {
        assert!(matches!(
            parse("4d6kh5"),
            Err(ParseError::ModifierExceedsDiceCount {
                count: 4,
                modifier: 5
            })
        ));
        assert!(matches!(
            parse("4d6dl5"),
            Err(ParseError::ModifierExceedsDiceCount {
                count: 4,
                modifier: 5
            })
        ));
    }

    #[test]
    fn error_invalid_number() {
        let big = "9".repeat(30);
        assert!(matches!(
            parse(&format!("{big}d6")),
            Err(ParseError::InvalidNumber(_))
        ));
    }

    #[test]
    fn display_messages_match_original() {
        assert_eq!(ParseError::Empty.to_string(), "empty expression");
        assert_eq!(
            ParseError::Unexpected {
                token: "foo".into()
            }
            .to_string(),
            "unexpected input: 'foo'"
        );
        assert_eq!(
            ParseError::UnexpectedTrailing { rest: "bar".into() }.to_string(),
            "unexpected trailing input: 'bar'"
        );
        assert_eq!(
            ParseError::MissingOperator.to_string(),
            "missing '+' or '-' between terms"
        );
        assert_eq!(ParseError::ZeroDice.to_string(), "must roll at least 1 die");
        assert_eq!(
            ParseError::TooFewSides.to_string(),
            "dice must have at least 2 sides"
        );
        assert_eq!(
            ParseError::DiceCountExceeded {
                count: 2,
                max: 1_000_000
            }
            .to_string(),
            "dice count exceeds maximum of 1000000"
        );
        assert_eq!(
            ParseError::ModifierExceedsDiceCount {
                count: 4,
                modifier: 5
            }
            .to_string(),
            "modifier 5 exceeds dice count 4"
        );
        assert_eq!(
            ParseError::InvalidNumber("xyz".into()).to_string(),
            "invalid number: 'xyz'"
        );
    }
}
