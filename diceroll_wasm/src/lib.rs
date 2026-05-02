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
