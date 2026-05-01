#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::env;
use std::ffi::OsString;
use std::process;

use lexopt::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

mod repl;
mod server;

use diceroll::run;
use repl::{read_stdin, repl};

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run,
    Serve,
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    seed: Option<u64>,
    json: bool,
    no_color: bool,
    expr: Option<String>,
    command: Command,
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
        run_mode(args, &mut StdRng::seed_from_u64(n));
    } else {
        run_mode(args, &mut rand::rng());
    }
}

fn run_mode<R: Rng>(args: Args, rng: &mut R) {
    match args.command {
        Command::Serve => {
            if let Err(e) = server::serve(rng) {
                eprintln!("server error: {e}");
                process::exit(1);
            }
        }
        Command::Run => dispatch(args.expr.as_deref(), args.json, args.no_color, rng),
    }
}

fn dispatch<R: Rng>(expr: Option<&str>, json: bool, no_color: bool, rng: &mut R) {
    use std::io::IsTerminal;
    let color =
        !json && !no_color && env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    let Some(expr) = expr else {
        if std::io::stdin().is_terminal() {
            if let Err(e) = repl(rng, json, color) {
                eprintln!("repl error: {e}");
                process::exit(1);
            }
        } else if let Err(e) = read_stdin(rng, json, color) {
            eprintln!("stdin error: {e}");
            process::exit(1);
        }
        return;
    };
    match run(expr, rng) {
        Ok(r) => println!("{}", r.formatted(json, color)),
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
    let mut no_color = false;
    let mut parts: Vec<String> = Vec::new();
    let mut command = Command::Run;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("seed") => seed = Some(parser.value()?.parse()?),
            Long("json") => json = true,
            Long("no-color") => no_color = true,
            Short('h') | Long("help") => {
                print_help();
                process::exit(0);
            }
            Value(v) => {
                let value = v.string()?;
                if command == Command::Run && parts.is_empty() && value == "serve" {
                    command = Command::Serve;
                } else if command == Command::Serve {
                    return Err(lexopt::Error::UnexpectedArgument(value.into()));
                } else {
                    parts.push(value);
                }
            }
            _ => return Err(arg.unexpected()),
        }
    }

    let expr = if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    };
    Ok(Args {
        seed,
        json,
        no_color,
        expr,
        command,
    })
}

const HELP: &str = "\
Usage: diceroll [--seed N] [--json] [--no-color] [EXPR ...]
       diceroll serve

Options:
  --seed N      seed the RNG for reproducible rolls
  --json        output structured JSON
  --no-color    disable ANSI color output (also honoured via NO_COLOR env var)
  -h, --help    show this help

Without EXPR, runs an interactive REPL (or reads stdin line-by-line if piped).
`diceroll serve` exposes a local HTTP server on 127.0.0.1:8000 with GET /roll?q=...
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
        assert_eq!(
            parse(&[]),
            Args {
                seed: None,
                json: false,
                no_color: false,
                expr: None,
                command: Command::Run
            }
        );
    }

    #[test]
    fn seed_separated() {
        assert_eq!(
            parse(&["--seed", "42", "2d6"]),
            Args {
                seed: Some(42),
                json: false,
                no_color: false,
                expr: Some("2d6".into()),
                command: Command::Run
            },
        );
    }

    #[test]
    fn seed_equals() {
        assert_eq!(
            parse(&["--seed=42", "2d6"]),
            Args {
                seed: Some(42),
                json: false,
                no_color: false,
                expr: Some("2d6".into()),
                command: Command::Run
            },
        );
    }

    #[test]
    fn seed_after_expr() {
        assert_eq!(
            parse(&["2d6", "--seed", "7"]),
            Args {
                seed: Some(7),
                json: false,
                no_color: false,
                expr: Some("2d6".into()),
                command: Command::Run
            },
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
            Args {
                seed: None,
                json: true,
                no_color: false,
                expr: Some("2d6".into()),
                command: Command::Run
            },
        );
    }

    #[test]
    fn json_after_expr() {
        assert_eq!(
            parse(&["2d6", "--json"]),
            Args {
                seed: None,
                json: true,
                no_color: false,
                expr: Some("2d6".into()),
                command: Command::Run
            },
        );
    }

    #[test]
    fn multiple_positional_args_join_with_space() {
        let a = parse(&["2d20", "+", "3d6", "+", "4"]);
        assert_eq!(a.expr.as_deref(), Some("2d20 + 3d6 + 4"));
    }

    #[test]
    fn no_color_flag() {
        assert_eq!(
            parse(&["--no-color", "2d6"]),
            Args {
                seed: None,
                json: false,
                no_color: true,
                expr: Some("2d6".into()),
                command: Command::Run
            },
        );
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

    #[test]
    fn serve_subcommand_sets_command() {
        let a = parse(&["serve"]);
        assert_eq!(a.command, Command::Serve);
        assert!(a.expr.is_none());
    }
}
