#![warn(clippy::all, rust_2018_idioms)]
use crossbeam::channel;
use eframe::{egui::{self, Vec2}, epi::{self, App}};

mod midi;
use midi::MidiReader;
mod audio;
use audio::{AudioManager, Message};
mod synth;
use synth::Synth;

const NAME: &'static str = "Wayfärer";

type MessageReceiver = channel::Receiver<Message>;
struct Wayfarer {
    audio_messages: Option<MessageReceiver>,
    midi_interface_name: String,
    audio_interface_name: String,
    status_text: String,
}

struct WayfarerArgs {
    audio_messages: MessageReceiver,
    midi_name: String,
    initial_status: String,
}

impl Wayfarer {
    fn new(args: WayfarerArgs) -> Self {
        Wayfarer {
            audio_messages: Some(args.audio_messages),
            midi_interface_name: args.midi_name,
            audio_interface_name: "-".to_string(),
            status_text: args.initial_status,
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

    fn is_resizable(&self) -> bool {
        false
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
        });
    }
}

fn main() {
    let (audio_messages_tx, audio_messages_rx) = channel::bounded(256);
    let (miditx, midirx) = channel::bounded(256);
    // keeping _midi around so that we keep receiving midi events
    let (_midi, midi_name, initial_status) = match MidiReader::new(miditx) {
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
    }));
    eframe::run_native(app);
}
