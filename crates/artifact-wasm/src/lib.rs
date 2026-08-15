//! Browser bindings for pure decision artifact review functions.

use artifact_core::{review_source, semantic_diff};
use wasm_bindgen::prelude::*;

/// Validate and compile source into the same review snapshot used natively.
///
/// # Errors
/// Returns a JavaScript serialization error if the snapshot cannot cross the WASM boundary.
#[wasm_bindgen]
pub fn validate_artifact(source: &str) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&review_source(source)).map_err(|error| error.to_string().into())
}

/// Semantically compare two valid source artifacts.
///
/// # Errors
/// Returns a JavaScript string error when either source is invalid or serialization fails.
#[wasm_bindgen]
pub fn diff_artifacts(base: &str, current: &str) -> Result<JsValue, JsValue> {
    let base = review_source(base);
    let current = review_source(current);
    let base = base
        .compiled
        .ok_or_else(|| JsValue::from_str("base artifact is invalid"))?;
    let current = current
        .compiled
        .ok_or_else(|| JsValue::from_str("current artifact is invalid"))?;
    serde_wasm_bindgen::to_value(&semantic_diff(&base, &current))
        .map_err(|error| error.to_string().into())
}
