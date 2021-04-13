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
    forced_buffer_size: Option<u32>,
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
            forced_buffer_size: None,
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
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("midi:");
                    ui.label(
                        self.midi
                            .as_ref()
                            .map(|midi| midi.get_name())
                            .unwrap_or("-"),
                    );
                });
            });

            ui.group(|ui| {
                ui.horizontal(|ui| {
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
                });
                let buffer_range = self.audio.get_buffer_size_range();
                ui.horizontal(|ui| {
                    ui.label("buffer size:");
                    ui.group(|ui| {
                        if buffer_range.is_none() {
                            ui.set_enabled(false);
                            self.forced_buffer_size = None;
                        }
                        let mut forced = self.forced_buffer_size.is_some();
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut forced, "force");
                            ui.set_enabled(forced);
                            let mut size = match self.forced_buffer_size {
                                Some(size) => size,
                                None => self.audio.get_buffer_size().unwrap_or(0),
                            };
                            let range = match buffer_range {
                                Some((min, max)) => min..=max,
                                None => 0..=1,
                            };
                            ui.add(egui::Slider::new(&mut size, range));
                            if forced {
                                self.forced_buffer_size = Some(size);
                            } else {
                                self.forced_buffer_size = None;
                            }
                            self.audio.set_forced_buffer_size(self.forced_buffer_size);
                        });
                    });
                });

                ui.label(&*self.status_text.lock().unwrap());
            });
            // put onscreen keyboard at bottom of window
            let height = ui.available_size().y;
            ui.add_space(height - 20f32);
            self.keyboard.show(ui);
        });
    }
}

fn main() {
    env_logger::init();
    let app = Box::new(Wayfarer::new());
    eframe::run_native(app);
}
