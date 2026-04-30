use std::env;
use std::ffi::OsString;
use std::process;

use lexopt::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

mod eval;
mod parser;
mod repl;

use eval::run;
use repl::repl;

#[derive(Debug, PartialEq, Eq)]
struct Args {
    seed: Option<u64>,
    json: bool,
    expr: Option<String>,
}

fn main() {
    let args = match parse_args(env::args_os()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            process::exit(2);
        }
    };

    if let Some(n) = args.seed {
        dispatch(args.expr.as_deref(), args.json, &mut StdRng::seed_from_u64(n));
    } else {
        dispatch(args.expr.as_deref(), args.json, &mut rand::rng());
    }
}

fn dispatch<R: Rng>(expr: Option<&str>, json: bool, rng: &mut R) {
    let Some(expr) = expr else {
        if let Err(e) = repl(rng, json) {
            eprintln!("repl error: {e}");
            process::exit(1);
        }
        return;
    };
    match run(expr, rng) {
        Ok(r) => println!("{}", r.formatted(json)),
        Err(e) => {
            eprintln!("parse error: {e}");
            process::exit(1);
        }
    }
}

fn parse_args<I>(iter: I) -> Result<Args, lexopt::Error>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let mut parser = lexopt::Parser::from_iter(iter);
    let mut seed = None;
    let mut json = false;
    let mut parts: Vec<String> = Vec::new();

    while let Some(arg) = parser.next()? {
        match arg {
            Long("seed") => seed = Some(parser.value()?.parse()?),
            Long("json") => json = true,
            Short('h') | Long("help") => {
                print_help();
                process::exit(0);
            }
            Value(v) => parts.push(v.string()?),
            _ => return Err(arg.unexpected()),
        }
    }

    let expr = if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    };
    Ok(Args { seed, json, expr })
}

const HELP: &str = "\
Usage: diceroll [--seed N] [--json] [EXPR ...]

Options:
  --seed N      seed the RNG for reproducible rolls
  --json        output structured JSON
  -h, --help    show this help

Without EXPR, runs an interactive REPL.
For an expression starting with '-', use '--' (e.g. diceroll -- -1d4+10).";

fn print_help() {
    println!("{HELP}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(extra: &[&str]) -> Args {
        let mut all: Vec<&str> = vec!["prog"];
        all.extend_from_slice(extra);
        parse_args(all).unwrap()
    }

    fn parse_err(extra: &[&str]) {
        let mut all: Vec<&str> = vec!["prog"];
        all.extend_from_slice(extra);
        assert!(parse_args(all).is_err());
    }

    #[test]
    fn empty_argv() {
        assert_eq!(parse(&[]), Args { seed: None, json: false, expr: None });
    }

    #[test]
    fn seed_separated() {
        assert_eq!(
            parse(&["--seed", "42", "2d6"]),
            Args { seed: Some(42), json: false, expr: Some("2d6".into()) },
        );
    }

    #[test]
    fn seed_equals() {
        assert_eq!(
            parse(&["--seed=42", "2d6"]),
            Args { seed: Some(42), json: false, expr: Some("2d6".into()) },
        );
    }

    #[test]
    fn seed_after_expr() {
        assert_eq!(
            parse(&["2d6", "--seed", "7"]),
            Args { seed: Some(7), json: false, expr: Some("2d6".into()) },
        );
    }

    #[test]
    fn seed_missing_value_errors() {
        parse_err(&["--seed"]);
    }

    #[test]
    fn seed_invalid_value_errors() {
        parse_err(&["--seed", "foo"]);
    }

    #[test]
    fn json_flag() {
        assert_eq!(
            parse(&["--json", "2d6"]),
            Args { seed: None, json: true, expr: Some("2d6".into()) },
        );
    }

    #[test]
    fn json_after_expr() {
        assert_eq!(
            parse(&["2d6", "--json"]),
            Args { seed: None, json: true, expr: Some("2d6".into()) },
        );
    }

    #[test]
    fn multiple_positional_args_join_with_space() {
        let a = parse(&["2d20", "+", "3d6", "+", "4"]);
        assert_eq!(a.expr.as_deref(), Some("2d20 + 3d6 + 4"));
    }

    #[test]
    fn unknown_flag_errors() {
        parse_err(&["--bogus"]);
    }

    #[test]
    fn double_dash_passes_through_negative_value() {
        let a = parse(&["--", "-1d4+10"]);
        assert_eq!(a.expr.as_deref(), Some("-1d4+10"));
    }
}
