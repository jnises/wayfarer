use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, SampleFormat, SampleRate,
};
use crossbeam::channel;
use iced::{
    executor, time, window, Application, Clipboard, Color, Column, Command, Container, Element,
    Length, Row, Settings, Space, Subscription, Text,
};
use midir::{MidiInput, MidiInputConnection};
use std::{
    convert::TryFrom,
    thread::{self, JoinHandle},
    time::Instant,
};

enum NoteEvent {
    On { freq: f32, velocity: f32 },
    Off,
}

struct MidiReader {
    // we receive midi input as long as this is alive
    #[allow(dead_code)]
    connection: Option<MidiInputConnection<()>>,
}

impl MidiReader {
    fn new(callback: channel::Sender<NoteEvent>, message_sender: channel::Sender<Message>) -> Self {
        let midi = MidiInput::new("wayfarer").unwrap();
        let ports = midi.ports();
        if let Some(port) = ports.first() {
            let name = midi.port_name(port).unwrap();
            message_sender
                .send(Message::MidiName(name.clone()))
                .unwrap();
            let connection = midi
                .connect(
                    port,
                    &name,
                    move |_time_ms, message, _| {
                        let message = wmidi::MidiMessage::try_from(message).unwrap();
                        match message {
                            wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                                let norm_vel = (u8::from(velocity) - u8::from(wmidi::U7::MIN))
                                    as f32
                                    / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32;
                                callback
                                    .try_send(NoteEvent::On {
                                        freq: note.to_freq_f32(),
                                        velocity: norm_vel,
                                    })
                                    .unwrap();
                            }
                            wmidi::MidiMessage::NoteOff(_, _note, _) => {
                                callback.try_send(NoteEvent::Off).unwrap();
                            }
                            _ => {}
                        }
                    },
                    (),
                )
                .unwrap();
            MidiReader {
                connection: Some(connection),
            }
        } else {
            MidiReader { connection: None }
        }
    }
}

struct Synth {
    sample_rate: u32,
    channels: usize,
    clock: u64,
    freq: f32,
    amplitude: f32,
}

impl Synth {
    fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate,
            channels,
            clock: 0,
            freq: 0f32,
            amplitude: 0f32,
        }
    }

    fn play(&mut self, output: &mut [f32]) {
        for frame in output.chunks_mut(self.channels) {
            let value = self.amplitude
                * (self.clock as f32 / self.sample_rate as f32
                    * self.freq
                    * 2f32
                    * std::f32::consts::PI)
                    .sin();
            self.clock += 1;
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

struct AudioManager {
    handle: Option<JoinHandle<()>>,
    shutdown: channel::Sender<()>,
}

impl AudioManager {
    fn new(
        midi_events: channel::Receiver<NoteEvent>,
        message_sender: channel::Sender<Message>,
    ) -> Self {
        let (tx, rx) = channel::bounded(1);
        let handle = thread::spawn(move || {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .expect("no default output device found");
            let supported_config = device
                .supported_output_configs()
                .unwrap()
                .filter(|config| {
                    config.sample_format() == SampleFormat::F32 && config.channels() == 2
                })
                .next()
                .unwrap();
            let min_rate = supported_config.min_sample_rate();
            let max_rate = supported_config.max_sample_rate();
            let config = supported_config
                .with_sample_rate(SampleRate(48000).clamp(min_rate, max_rate))
                .config();
            let mut synth = Synth::new(config.sample_rate.0, 2);
            let message_sender_clone = message_sender.clone();
            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        while let Ok(event) = midi_events.try_recv() {
                            match event {
                                NoteEvent::On { freq, velocity } => {
                                    synth.freq = freq;
                                    synth.amplitude = velocity;
                                }
                                NoteEvent::Off => {
                                    synth.amplitude = 0f32;
                                }
                            }
                        }
                        synth.play(data);
                    },
                    move|error| {
                        message_sender_clone.send(Message::Status(format!("error: {:?}", error))).unwrap();
                    },
                )
                .unwrap();
            message_sender
                .send(Message::AudioName(device.name().unwrap()))
                .unwrap();
            stream.play().unwrap();
            rx.recv().unwrap();
        });
        Self {
            handle: Some(handle),
            shutdown: tx,
        }
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.shutdown.send(()).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}

enum Message {
    MidiName(String),
    AudioName(String),
    Status(String),
}

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
