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
use diceroll::stats;
use repl::{read_stdin, repl};

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run,
    Serve { port: u16 },
    Stats { samples: usize },
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
        Command::Serve { port } => {
            if let Err(e) = server::serve(rng, port) {
                eprintln!("server error: {e}");
                process::exit(1);
            }
        }
        Command::Run => dispatch_run(args.expr.as_deref(), args.json, args.no_color, rng),
        Command::Stats { samples } => dispatch_stats(args.expr.as_deref(), args.json, samples, rng),
    }
}

fn dispatch_run<R: Rng>(expr: Option<&str>, json: bool, no_color: bool, rng: &mut R) {
    use std::io::IsTerminal;
    let color =
        !json && !no_color && env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    match expr {
        Some(expr) => match run(expr, rng) {
            Ok(r) => println!("{}", r.formatted(json, color)),
            Err(e) => {
                eprintln!("parse error: {e}");
                process::exit(1);
            }
        },
        None => {
            if std::io::stdin().is_terminal() {
                if let Err(e) = repl(rng, json, color) {
                    eprintln!("repl error: {e}");
                    process::exit(1);
                }
            } else if let Err(e) = read_stdin(rng, json, color) {
                eprintln!("stdin error: {e}");
                process::exit(1);
            }
        }
    }
}

fn dispatch_stats<R: Rng>(expr: Option<&str>, json: bool, samples: usize, rng: &mut R) {
    match expr {
        Some(expr) => match stats::run_stats(expr, samples, rng) {
            Ok(stats_results) if json => match serde_json::to_string(&stats_results) {
                Ok(s) => println!("{s}"),
                Err(e) => {
                    eprintln!("formatting error: {e}")
                }
            },
            Ok(stats_results) => {
                println!("{stats_results}")
            }
            Err(e) => {
                eprintln!("parse error: {e}");
                process::exit(1);
            }
        },
        None => {
            eprintln!("expression required for stats");
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
    let mut serve_port: Option<u16> = None;
    let mut stats_samples: Option<usize> = None;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("seed") => seed = Some(parser.value()?.parse()?),
            Long("json") => json = true,
            Long("no-color") => no_color = true,
            Long("port") => serve_port = Some(parser.value()?.parse()?),
            Long("samples") => stats_samples = Some(parser.value()?.parse()?),
            Short('h') | Long("help") => {
                print_help();
                process::exit(0);
            }
            Value(v) => {
                let value = v.string()?;
                if command == Command::Run && parts.is_empty() && value == "serve" {
                    command = Command::Serve {
                        port: serve_port.take().unwrap_or(8000),
                    };
                } else if command == Command::Run && parts.is_empty() && value == "stats" {
                    command = Command::Stats {
                        samples: stats_samples.take().unwrap_or(1000),
                    };
                } else if matches!(command, Command::Serve { .. }) {
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
    let command = match command {
        Command::Run => {
            if serve_port.is_some() {
                return Err(lexopt::Error::UnexpectedArgument("--port".into()));
            }
            if stats_samples.is_some() {
                return Err(lexopt::Error::UnexpectedArgument("--samples".into()));
            }
            Command::Run
        }
        Command::Serve { port } => {
            if stats_samples.is_some() {
                return Err(lexopt::Error::UnexpectedArgument("--samples".into()));
            }
            Command::Serve {
                port: serve_port.unwrap_or(port),
            }
        }
        Command::Stats { samples } => {
            if serve_port.is_some() {
                return Err(lexopt::Error::UnexpectedArgument("--port".into()));
            }
            Command::Stats {
                samples: stats_samples.unwrap_or(samples),
            }
        }
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
Usage: diceroll [--seed N] [--json] [--no-color] [EXPR]
       diceroll serve [--port N]
       diceroll stats [--samples N] EXPR

Options:
  --seed N      seed the RNG for reproducible rolls
  --json        output structured JSON
  --no-color    disable ANSI color output (also honoured via NO_COLOR env var)
  --port N      port for `serve` (default: 8000)
  --samples N   number of samples on which to compute statistics (default: 1000)
  -h, --help    show this help

Without EXPR, runs an interactive REPL (or reads stdin line-by-line if piped).

`diceroll serve` exposes a local HTTP server on 127.0.0.1:N with GET /roll?q=... and POST /roll.

`diceroll stats` computes statistics on N rolls of the expression EXPR.

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
        assert_eq!(a.command, Command::Serve { port: 8000 });
        assert!(a.expr.is_none());
    }

    #[test]
    fn serve_subcommand_accepts_port_before() {
        let a = parse(&["--port", "8123", "serve"]);
        assert_eq!(a.command, Command::Serve { port: 8123 });
    }

    #[test]
    fn serve_subcommand_accepts_port_after() {
        let a = parse(&["serve", "--port", "8123"]);
        assert_eq!(a.command, Command::Serve { port: 8123 });
    }

    #[test]
    fn port_without_serve_errors() {
        parse_err(&["--port", "8123"]);
    }
}
