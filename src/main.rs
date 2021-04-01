use iced::{
    executor, time, window, Application, Clipboard, Color, Command, Container, Element, Length,
    Settings, Subscription, Text,
};
use std::time::Instant;

pub fn main() -> iced::Result {
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
        time::every(std::time::Duration::from_millis(16)).map(|instant| Message::Tick(instant))
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
