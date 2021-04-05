use crossbeam::channel;
use iced::{
    executor, time, window, Application, Clipboard, Color, Column, Command, Container, Element,
    Length, Row, Settings, Space, Subscription, Text,
};
use std::time::Instant;

mod midi;
use midi::MidiReader;
mod message;
use message::Message;
mod audio;
use audio::AudioManager;
mod synth;

type MessageReceiver = Option<channel::Receiver<Message>>;
struct Wayfarer {
    message_receiver: MessageReceiver,
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

impl Application for Wayfarer {
    type Executor = executor::Default;
    type Message = GuiMessage;
    // TODO use better type for this
    type Flags = Option<channel::Receiver<Message>>;

    fn new(receiver: Self::Flags) -> (Self, Command<GuiMessage>) {
        (
            Wayfarer {
                message_receiver: receiver,
                start: Instant::now(),
                now: Instant::now(),
                midi_interface_name: "-".to_string(),
                audio_interface_name: "-".to_string(),
                status_text: String::new(),
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
                if let Some(ref receiver) = self.message_receiver {
                    for msg in receiver.try_iter() {
                        match msg {
                            Message::MidiName(s) => {
                                self.midi_interface_name = s;
                            }
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
    let (messagetx, messagerx) = channel::bounded(256);
    let (miditx, midirx) = channel::bounded(256);
    let _midi = MidiReader::new(miditx, messagetx.clone());
    let _audio = AudioManager::new(midirx, messagetx);
    Wayfarer::run(Settings {
        flags: Some(messagerx),
        antialiasing: true,
        window: window::Settings {
            size: (400, 200),
            resizable: false,
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
