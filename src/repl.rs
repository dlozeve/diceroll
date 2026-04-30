use std::io::{self, BufRead, Write};

use rand::Rng;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::eval::run;

fn handle_line(
    line: &str,
    rng: &mut impl Rng,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> io::Result<bool> {
    let expr = line.trim();
    if expr.is_empty() {
        return Ok(false);
    }
    match run(expr, rng) {
        Ok(r) => writeln!(out, "{}", r.formatted(json))?,
        Err(e) => writeln!(err, "parse error: {e}")?,
    }
    Ok(true)
}

pub fn read_stdin(rng: &mut impl Rng, json: bool) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    for line in stdin.lock().lines() {
        handle_line(&line?, rng, json, &mut stdout, &mut stderr)?;
    }
    Ok(())
}

pub fn repl(rng: &mut impl Rng, json: bool) -> rustyline::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    loop {
        match rl.readline(">>> ") {
            Ok(line) => {
                if handle_line(&line, rng, json, &mut stdout, &mut stderr)? {
                    rl.add_history_entry(line.trim())?;
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn handle_line_writes_plain_to_out() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let added = handle_line("2d6+1", &mut rng, false, &mut out, &mut err).unwrap();

        let mut expected_rng = StdRng::seed_from_u64(7);
        let r = run("2d6+1", &mut expected_rng).unwrap();

        assert!(added);
        assert_eq!(String::from_utf8(out).unwrap(), format!("{}\n", r.formatted(false)));
        assert!(err.is_empty());
    }

    #[test]
    fn handle_line_writes_json_when_flag_set() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        handle_line("2d6+1", &mut rng, true, &mut out, &mut err).unwrap();

        let mut expected_rng = StdRng::seed_from_u64(7);
        let r = run("2d6+1", &mut expected_rng).unwrap();

        assert_eq!(String::from_utf8(out).unwrap(), format!("{}\n", r.formatted(true)));
        assert!(err.is_empty());
    }

    #[test]
    fn handle_line_routes_parse_error_to_err() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let added = handle_line("foo", &mut rng, false, &mut out, &mut err).unwrap();

        assert!(added);
        assert!(out.is_empty());
        let err_s = String::from_utf8(err).unwrap();
        assert!(err_s.starts_with("parse error:"));
        assert!(err_s.ends_with('\n'));
    }

    #[test]
    fn handle_line_empty_returns_false_and_writes_nothing() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let added = handle_line("", &mut rng, false, &mut out, &mut err).unwrap();

        assert!(!added);
        assert!(out.is_empty());
        assert!(err.is_empty());
    }

    #[test]
    fn handle_line_whitespace_only_returns_false() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let added = handle_line("  \t  \n", &mut rng, false, &mut out, &mut err).unwrap();

        assert!(!added);
        assert!(out.is_empty());
        assert!(err.is_empty());
    }

    #[test]
    fn handle_line_trims_input_before_evaluating() {
        let mut rng_a = StdRng::seed_from_u64(11);
        let mut out_a: Vec<u8> = Vec::new();
        let mut err_a: Vec<u8> = Vec::new();
        handle_line("  2d6+1  \n", &mut rng_a, false, &mut out_a, &mut err_a).unwrap();

        let mut rng_b = StdRng::seed_from_u64(11);
        let mut out_b: Vec<u8> = Vec::new();
        let mut err_b: Vec<u8> = Vec::new();
        handle_line("2d6+1", &mut rng_b, false, &mut out_b, &mut err_b).unwrap();

        assert_eq!(out_a, out_b);
        assert!(err_a.is_empty());
        assert!(err_b.is_empty());
    }
}
