//! A dice-expression parser and evaluator.
//!
//! # Quick start
//!
//! ```
//! use rand::SeedableRng;
//! use rand::rngs::StdRng;
//!
//! let mut rng = StdRng::seed_from_u64(0);
//! let result = diceroll::run("2d6+3", &mut rng).unwrap();
//! assert!(result.total >= 5 && result.total <= 15);
//! println!("{}", result.formatted(false, false)); // e.g. "2d6[3,4] + 3 = 10"
//! ```

pub mod eval;
pub mod format;
pub mod parser;

pub use eval::{EvalResult, EvalTerm, EvalTermKind, evaluate, run};
pub use parser::{DiceModifier, KeepDrop, ParseError, Term, parse};
