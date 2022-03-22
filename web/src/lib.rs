#![allow(clippy::unused_unit)]
use core::recipes::recipes_by_level;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn recipes(level: u32) -> JsValue {
    let recipes = recipes_by_level(level);
    JsValue::from_serde(recipes).unwrap()
}
