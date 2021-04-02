use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, SampleFormat, SampleRate,
};
use iced::{
    executor, time, window, Application, Clipboard, Color, Command, Container, Element, Length,
    Settings, Subscription, Text,
};
use std::{sync::mpsc::{Sender, channel}, thread::{self, JoinHandle}, time::Instant};

struct MidiReader {
    
}

struct Synth {
    sample_rate: u32,
    channels: usize,
    clock: u64,
}

impl Synth {
    fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate,
            channels,
            clock: 0,
        }
    }

    fn play(&mut self, output: &mut [f32]) {
        for frame in output.chunks_mut(self.channels) {
            let value = (self.clock as f32 / self.sample_rate as f32 * 440f32).sin();
            self.clock += 1;
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }
}

struct AudioManager {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
    #[allow(dead_code)]
    shutdown: Sender<()>,
}

impl AudioManager {
    fn start() -> Self {
        let (tx, rx) = channel();
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
            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        synth.play(data);
                    },
                    |error| {
                        eprintln!("error: {:?}", error);
                    },
                )
                .unwrap();
            stream.play().unwrap();
            rx.recv().unwrap();
        });
        Self {
            handle,
            shutdown: tx,
        }
    }
}

fn main() -> iced::Result {
    let _audio = AudioManager::start();
    Wayfarer::run(Settings {
        antialiasing: true,
        window: window::Settings {
            size: (400, 200),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}

struct GuiState {
    start: Instant,
    now: Instant,
}

struct Wayfarer {
    state: GuiState,
}

impl Default for GuiState {
    fn default() -> Self {
        GuiState {
            start: Instant::now(),
            now: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick(Instant),
}

impl Application for Wayfarer {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Wayfarer {
                state: GuiState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Wayfärer")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match message {
            Message::Tick(instant) => {
                self.state.now = instant;
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
    }

    fn view(&mut self) -> Element<Message> {
        let t = self.state.now - self.state.start;
        let color = palette::Srgb::from(palette::Lch::new(50.0, 50.0, 10.0 * t.as_secs_f32()));
        let color = Color::from_rgb(color.red, color.green, color.blue);
        let text = Text::new("Wayfärer").size(50).color(color);
        Container::new(text)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x()
            .center_y()
            .into()
    }
}
