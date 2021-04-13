#![warn(clippy::all, rust_2018_idioms)]
use std::sync::{Arc, Mutex};

use crossbeam::channel;
use eframe::{
    egui::{self, Vec2},
    epi::{self, App},
};
mod keyboard;
use keyboard::OnScreenKeyboard;
mod midi;
use midi::MidiReader;
mod audio;
use audio::AudioManager;
mod synth;
use synth::Synth;

const NAME: &'static str = "Wayfärer";

struct Wayfarer {
    audio: AudioManager<Synth>,
    midi: Option<MidiReader>,
    status_text: Arc<Mutex<String>>,
    keyboard: OnScreenKeyboard,
}

impl Wayfarer {
    fn new() -> Self {
        let (midi_tx, midi_rx) = channel::bounded(256);
        let (midi, initial_status) = match MidiReader::new(midi_tx.clone()) {
            Ok(midi) => (Some(midi), String::new()),
            Err(e) => (None, format!("error initializaing midi: {}", e)),
        };
        let synth = Synth::new(midi_rx);
        let status_text = Arc::new(Mutex::new(initial_status));
        let status_clone = status_text.clone();
        let audio = AudioManager::new(synth, move |e| {
            *status_clone.lock().unwrap() = e;
        });
        Self {
            audio,
            midi,
            status_text,
            keyboard: OnScreenKeyboard::new(midi_tx),
        }
    }
}

impl App for Wayfarer {
    fn name(&self) -> &str {
        NAME
    }

    fn initial_window_size(&self) -> Option<Vec2> {
        Some(Vec2 {
            x: 400f32,
            y: 300f32,
        })
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(NAME);
            ui.horizontal(|ui| {
                ui.label("midi: ");
                ui.label(
                    self.midi
                        .as_ref()
                        .map(|midi| midi.get_name())
                        .unwrap_or("-"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("audio: ");
                ui.label(&self.audio.get_name().unwrap_or("_".to_string()));
            });
            ui.label(&*self.status_text.lock().unwrap());
            // put onscreen keyboard at bottom of window
            let height = ui.available_size().y;
            ui.add_space(height - 20f32);
            self.keyboard.show(ui);
        });
    }
}

fn main() {
    let app = Box::new(Wayfarer::new());
    eframe::run_native(app);
}
