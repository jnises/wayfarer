#![warn(clippy::all, rust_2018_idioms)]
use std::sync::{Arc, Mutex};

use cpal::traits::DeviceTrait;
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
    forced_buffer_size: String,
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
            forced_buffer_size: String::new(),
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
            egui::Grid::new("settings grid").show(ui, |ui| {
                ui.label("midi:");
                ui.label(
                    self.midi
                        .as_ref()
                        .map(|midi| midi.get_name())
                        .unwrap_or("-"),
                );
                ui.end_row();

                ui.label("audio:");
                let mut selected = self.audio.get_name().unwrap_or("-".to_string());
                egui::ComboBox::from_id_source("audio combo box")
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        // TODO cache this to not poll too often
                        for device in self.audio.get_devices() {
                            if let Ok(name) = device.name() {
                                ui.selectable_value(&mut selected, name.clone(), name);
                            }
                        }
                    });
                if Some(&selected) != self.audio.get_name().as_ref() {
                    if let Some(device) = self.audio.get_devices().into_iter().find(|d| {
                        if let Ok(name) = d.name() {
                            name == selected
                        } else {
                            false
                        }
                    }) {
                        self.audio.set_device(device);
                    }
                }
                ui.end_row();

                let buffer_range = self.audio.get_buffer_size_range();
                ui.label("min buffer size:");
                ui.label(
                    buffer_range
                        .map(|t| t.0.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                );
                ui.end_row();

                ui.label("max buffer size:");
                ui.label(
                    buffer_range
                        .map(|t| t.1.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                );
                ui.end_row();

                ui.label("force buffer size:");
                if ui
                    .text_edit_singleline(&mut self.forced_buffer_size)
                    .lost_focus()
                {
                    self.audio
                        .set_forced_buffer_size(self.forced_buffer_size.parse().ok());
                }
                ui.end_row();

                ui.label("actual buffer size:");
                ui.label(
                    self.audio
                        .get_buffer_size()
                        .map(|b| b.to_string())
                        .unwrap_or("-".to_string()),
                );
                ui.end_row();

                ui.label(&*self.status_text.lock().unwrap());
                ui.end_row();
            });
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
