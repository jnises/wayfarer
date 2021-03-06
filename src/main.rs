#![warn(clippy::all, rust_2018_idioms)]

mod audio;
mod keyboard;
mod midi;
mod periodic_updater;
mod synth;
mod timer;

mod app;
use app::Wayfarer;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use eframe::{egui::Vec2, epi};

    env_logger::init();
    let app = Box::new(Wayfarer::new());
    eframe::run_native(
        app,
        epi::NativeOptions {
            // has to be disabled to work with cpal
            drag_and_drop_support: false,
            initial_window_size: Some(Vec2 {
                x: 400f32,
                y: 400f32,
            }),
            ..Default::default()
        },
    );
}
