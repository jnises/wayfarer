use crossbeam::channel;
use iced::{
    executor, time, window, Application, Clipboard, Color, Column, Command, Container, Element,
    Length, Row, Settings, Space, Subscription, Text,
};
use std::time::Instant;

mod midi;
use midi::MidiReader;
mod audio;
use audio::{AudioManager, Message};
mod synth;
use synth::Synth;

type MessageReceiver = Option<channel::Receiver<Message>>;
struct Wayfarer {
    audio_messages: MessageReceiver,
    start: Instant,
    now: Instant,
    midi_interface_name: String,
    audio_interface_name: String,
    status_text: String,
}

#[derive(Debug, Clone, Copy)]
enum GuiMessage {
    Tick,
}

struct Flags {
    audio_messages: MessageReceiver,
    midi_name: String,
    initial_status: String,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            audio_messages: None,
            midi_name: "-".to_string(),
            initial_status: String::new(),
        }
    }
}

impl Application for Wayfarer {
    type Executor = executor::Default;
    type Message = GuiMessage;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<GuiMessage>) {
        (
            Wayfarer {
                audio_messages: flags.audio_messages,
                start: Instant::now(),
                now: Instant::now(),
                midi_interface_name: flags.midi_name,
                audio_interface_name: "-".to_string(),
                status_text: flags.initial_status,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Wayfärer")
    }

    fn update(
        &mut self,
        gui_message: GuiMessage,
        _clipboard: &mut Clipboard,
    ) -> Command<GuiMessage> {
        match gui_message {
            GuiMessage::Tick => {
                self.now = Instant::now();
                // can't seem to figure out the subscription feature.. so we just pump the channel here
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
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<GuiMessage> {
        // TODO subscribe on channel
        time::every(std::time::Duration::from_millis(100)).map(|_| GuiMessage::Tick)
    }

    fn view(&mut self) -> Element<GuiMessage> {
        let t = self.now - self.start;
        let color = palette::Srgb::from(palette::Lch::new(50.0, 50.0, 10.0 * t.as_secs_f32()));
        let color = Color::from_rgb(color.red, color.green, color.blue);
        Container::new(
            Column::new()
                .push(Text::new("Wayfärer").size(50).color(color))
                .push(Space::with_height(Length::Units(10)))
                .push(
                    Row::new()
                        .push(Text::new("midi: "))
                        .push(Text::new(self.midi_interface_name.clone())),
                )
                .push(
                    Row::new()
                        .push(Text::new("audio: "))
                        .push(Text::new(self.audio_interface_name.clone())),
                )
                .push(Space::with_height(Length::Units(10)))
                .push(Text::new(self.status_text.clone())),
        )
        .padding(20)
        .into()
    }
}

fn main() -> iced::Result {
    let (audio_messages_tx, audio_messages_rx) = channel::bounded(256);
    let (miditx, midirx) = channel::bounded(256);
    // keeping _midi around so that we keep receiving midi events
    let (_midi, midi_name, initial_status) = match MidiReader::new(miditx) {
        Ok(midi) => {
            let name = midi.get_name().to_string();
            (Some(midi), name, String::new())
        },
        Err(e) => (None, "-".to_string(), format!("error initializaing midi: {}", e)),
    };
    let synth = Synth::new(midirx);
    let _audio = AudioManager::new(audio_messages_tx, synth);
    Wayfarer::run(Settings {
        flags: Flags {
            audio_messages: Some(audio_messages_rx),
            midi_name,
            initial_status,
        },
        antialiasing: true,
        window: window::Settings {
            size: (400, 200),
            resizable: false,
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
