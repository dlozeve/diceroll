#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiceModifier {
    KeepDrop(KeepDrop),
    Min(u64),
    Max(u64),
    Reroll,
    RerollOnce,
    Exploding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepDrop {
    KeepHighest(u64),
    KeepLowest(u64),
    DropHighest(u64),
    DropLowest(u64),
}

impl std::fmt::Display for KeepDrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeepDrop::KeepHighest(n) => write!(f, "kh{n}"),
            KeepDrop::KeepLowest(n) => write!(f, "kl{n}"),
            KeepDrop::DropHighest(n) => write!(f, "dh{n}"),
            KeepDrop::DropLowest(n) => write!(f, "dl{n}"),
        }
    }
}

impl std::fmt::Display for DiceModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiceModifier::KeepDrop(kd) => write!(f, "{kd}"),
            DiceModifier::Min(n) => write!(f, "min{n}"),
            DiceModifier::Max(n) => write!(f, "max{n}"),
            DiceModifier::Reroll => write!(f, "r"),
            DiceModifier::RerollOnce => write!(f, "ro"),
            DiceModifier::Exploding => write!(f, "!"),
        }
    }
}

impl serde::Serialize for KeepDrop {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl serde::Serialize for DiceModifier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiceSides {
    Numeric(u64),
    Fate,
}

impl std::fmt::Display for DiceSides {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiceSides::Numeric(n) => write!(f, "{n}"),
            DiceSides::Fate => write!(f, "F"),
        }
    }
}

impl serde::Serialize for DiceSides {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            DiceSides::Numeric(n) => serializer.serialize_u64(*n),
            DiceSides::Fate => serializer.serialize_str("F"),
        }
    }
}

pub(crate) fn serialize_dice_modifiers<S>(
    modifiers: &Option<Vec<DiceModifier>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match modifiers {
        None => serializer.serialize_none(),
        Some(modifiers) if modifiers.len() == 1 => serializer.collect_str(&modifiers[0]),
        Some(modifiers) => serde::Serialize::serialize(modifiers, serializer),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Term {
    Dice {
        count: u64,
        sides: DiceSides,
        modifier: Option<Vec<DiceModifier>>,
    },
    Const(u64),
    /// A parenthesised sub-expression with an optional integer multiplier.
    /// `(2d6+3)*2` produces `Group { terms: [...], multiplier: 2 }`.
    Group {
        terms: Vec<(i64, Term)>,
        multiplier: u64,
    },
}
