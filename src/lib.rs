#![warn(clippy::all, rust_2018_idioms)]

mod audio;
mod keyboard;
mod midi;
mod synth;

mod app;
pub use app::Wayfarer;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    let app = Wayfarer::default();
    eframe::start_web(canvas_id, Box::new(app))
}
