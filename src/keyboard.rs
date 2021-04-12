use std::{collections::HashSet, convert::TryFrom};

use crossbeam::channel;
use eframe::egui;
use wmidi::MidiMessage;

fn is_key_black(note: wmidi::Note) -> bool {
    [
        false, true, false, true, false, false, true, false, true, false, true, false,
    ][(u8::from(note) % 12) as usize]
}

pub struct OnScreenKeyboard {
    keyboard_pressed: HashSet<egui::Id>,
}

impl OnScreenKeyboard {
    pub fn new() -> Self {
        Self {
            keyboard_pressed: HashSet::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, midi_tx: &mut channel::Sender<MidiMessage<'static>>) {
        ui.horizontal(|ui| {
            // start from middle c                        
            for note_num in 60.. {
                if ui.available_width() <= 0f32 {
                    break;
                }
                let note = match wmidi::Note::try_from(note_num) {
                    Ok(note) => note,
                    Err(_) => break,
                };
                let b = egui::Button::new(" ").fill(Some(if is_key_black(note) {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                }));
                let r = ui.add(b);
                // egui doesn't seem to have any convenient "pressed" or "released" event
                if r.is_pointer_button_down_on() {
                    if !self.keyboard_pressed.insert(r.id) {
                        let _ = midi_tx.try_send(MidiMessage::NoteOn(
                            wmidi::Channel::Ch1,
                            note,
                            wmidi::Velocity::from_u8_lossy(127),
                        ));
                    }
                } else {
                    if self.keyboard_pressed.remove(&r.id) {
                        let _ = midi_tx.try_send(MidiMessage::NoteOff(
                            wmidi::Channel::Ch1,
                            note,
                            wmidi::Velocity::from_u8_lossy(0),
                        ));
                    }
                }
            }
        });
    }
}
