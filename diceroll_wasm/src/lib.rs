use rand::{SeedableRng, rngs::StdRng};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// `serialize_maps_as_objects(true)` makes Rust structs land as plain JS objects
// instead of `Map`s, so callers can use `value.field` directly.
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer).map_err(Into::into)
}

fn seed_to_u64(seed: &str) -> Result<u64, JsError> {
    let seed = seed.trim();
    let seed = seed
        .strip_prefix("0x")
        .or_else(|| seed.strip_prefix("0X"))
        .unwrap_or(seed);

    u64::from_str_radix(seed, 16)
        .or_else(|_| seed.parse::<u64>())
        .map_err(|_| JsError::new("invalid session seed"))
}

fn seeded_rng(seed: &str) -> Result<StdRng, JsError> {
    Ok(StdRng::seed_from_u64(seed_to_u64(seed)?))
}

/// Roll an expression. Returns the formatted line, e.g. `"2d6[3,4] + 3 = 10"`.
#[wasm_bindgen]
pub fn roll(expr: &str) -> Result<String, JsError> {
    let mut rng = rand::rng();
    let result = diceroll::run(expr, &mut rng).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(result.formatted(false, false))
}

/// Roll an expression. Returns a JS object matching the JSON HTTP response shape.
#[wasm_bindgen(js_name = rollJson)]
pub fn roll_json(expr: &str) -> Result<JsValue, JsError> {
    let mut rng = rand::rng();
    let result = diceroll::run(expr, &mut rng).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&result)
}

/// Compute statistics over `samples` rolls of `expr`. Returns the formatted text block.
#[wasm_bindgen]
pub fn stats(expr: &str, samples: usize) -> Result<String, JsError> {
    let mut rng = rand::rng();
    let s = diceroll::stats::run_stats(expr, samples, &mut rng)
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(s.to_string())
}

#[wasm_bindgen(js_name = statsJson)]
pub fn stats_json(expr: &str, samples: usize) -> Result<JsValue, JsError> {
    let mut rng = rand::rng();
    let s = diceroll::stats::run_stats(expr, samples, &mut rng)
        .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&s)
}

#[wasm_bindgen]
pub struct Session {
    rng: StdRng,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: &str) -> Result<Session, JsError> {
        Ok(Self {
            rng: seeded_rng(seed)?,
        })
    }

    /// Roll an expression using this session's RNG stream.
    #[wasm_bindgen(js_name = rollJson)]
    pub fn roll_json(&mut self, expr: &str) -> Result<JsValue, JsError> {
        let result =
            diceroll::run(expr, &mut self.rng).map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&result)
    }

    /// Compute statistics using this session's RNG stream.
    #[wasm_bindgen]
    pub fn stats(&mut self, expr: &str, samples: usize) -> Result<String, JsError> {
        let s = diceroll::stats::run_stats(expr, samples, &mut self.rng)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(s.to_string())
    }

    #[wasm_bindgen(js_name = statsJson)]
    pub fn stats_json(&mut self, expr: &str, samples: usize) -> Result<JsValue, JsError> {
        let s = diceroll::stats::run_stats(expr, samples, &mut self.rng)
            .map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn session_replays_the_same_sequence_for_the_same_seed() {
        let commands = ["d20", "2d6+3", "stats 2d6"];

        fn replay(seed: &str, commands: &[&str]) -> Vec<String> {
            let mut rng = seeded_rng(seed).unwrap();
            commands
                .iter()
                .map(|line| {
                    if let Some(expr) = line.strip_prefix("stats ") {
                        diceroll::stats::run_stats(expr, 8, &mut rng)
                            .unwrap()
                            .to_string()
                    } else {
                        diceroll::run(line, &mut rng)
                            .unwrap()
                            .formatted(false, false)
                    }
                })
                .collect()
        }

        assert_eq!(
            replay("0x1234abcd", &commands),
            replay("0x1234abcd", &commands)
        );
    }

    #[test]
    fn seeded_rng_advances_across_draws() {
        let mut rng = seeded_rng("0x1234abcd").unwrap();

        let first = rng.next_u64();
        let second = rng.next_u64();

        assert_ne!(first, second);
    }
}
