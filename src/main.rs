#![warn(clippy::all, rust_2018_idioms)]
use std::{collections::HashSet, convert::TryFrom};

use crossbeam::channel;
use eframe::{
    egui::{self, Ui, Vec2},
    epi::{self, App},
};

mod midi;
use midi::MidiReader;
mod audio;
use audio::{AudioManager, Message};
mod synth;
use synth::Synth;
use wmidi::MidiMessage;

const NAME: &'static str = "Wayfärer";

type MidiSender = channel::Sender<MidiMessage<'static>>;

fn is_key_black(note: wmidi::Note) -> bool {
    [
        false, true, false, true, false, false, true, false, true, false, true, false,
    ][(u8::from(note) % 12) as usize]
}

type MessageReceiver = channel::Receiver<Message>;
struct Wayfarer {
    audio_messages: Option<MessageReceiver>,
    midi_interface_name: String,
    audio_interface_name: String,
    status_text: String,
    midi_tx: MidiSender,
    keyboard_pressed: HashSet<egui::Id>,
}

struct WayfarerArgs {
    audio_messages: MessageReceiver,
    midi_name: String,
    initial_status: String,
    midi_tx: MidiSender,
}

impl Wayfarer {
    fn new(args: WayfarerArgs) -> Self {
        Wayfarer {
            audio_messages: Some(args.audio_messages),
            midi_interface_name: args.midi_name,
            audio_interface_name: "-".to_string(),
            status_text: args.initial_status,
            midi_tx: args.midi_tx,
            keyboard_pressed: HashSet::new(),
        }
    }

    // need to keep state in self since egui doesn't seem to have support for custom state currently
    fn on_screen_keyboard(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
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
                        let _ = self.midi_tx.try_send(MidiMessage::NoteOn(
                            wmidi::Channel::Ch1,
                            note,
                            wmidi::Velocity::from_u8_lossy(127),
                        ));
                    }
                } else {
                    if self.keyboard_pressed.remove(&r.id) {
                        let _ = self.midi_tx.try_send(MidiMessage::NoteOff(
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

    fn is_resizable(&self) -> bool {
        //false
        true
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        if let Some(ref receiver) = self.audio_messages {
            for msg in receiver.try_iter() {
                match msg {
                    Message::AudioName(s) => {
                        self.audio_interface_name = s;
                    }
                    Message::Status(s) => {
                        self.status_text = s;
                    }
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(NAME);
            ui.horizontal(|ui| {
                ui.label("midi: ");
                ui.label(&self.midi_interface_name);
            });
            ui.horizontal(|ui| {
                ui.label("audio: ");
                ui.label(&self.audio_interface_name);
            });
            ui.label(&self.status_text);
            self.on_screen_keyboard(ui);
        });
    }
}

fn main() {
    let (audio_messages_tx, audio_messages_rx) = channel::bounded(256);
    let (miditx, midirx) = channel::bounded(256);
    // keeping _midi around so that we keep receiving midi events
    let (_midi, midi_name, initial_status) = match MidiReader::new(miditx.clone()) {
        Ok(midi) => {
            let name = midi.get_name().to_string();
            (Some(midi), name, String::new())
        }
        Err(e) => (
            None,
            "-".to_string(),
            format!("error initializaing midi: {}", e),
        ),
    };
    let synth = Synth::new(midirx);
    let _audio = AudioManager::new(audio_messages_tx, synth);
    let app = Box::new(Wayfarer::new(WayfarerArgs {
        audio_messages: audio_messages_rx,
        midi_name,
        initial_status,
        midi_tx: miditx,
    }));
    eframe::run_native(app);
}
