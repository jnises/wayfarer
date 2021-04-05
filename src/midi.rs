use crossbeam::channel;
use midir::{MidiInput, MidiInputConnection};
use std::convert::TryFrom;

pub enum NoteEvent {
    On { freq: f32, velocity: f32 },
    Off,
}

pub struct MidiReader {
    // we receive midi input as long as this is alive
    #[allow(dead_code)]
    connection: Option<MidiInputConnection<()>>,
    name: String,
}

impl MidiReader {
    // TODO do proper error handling
    pub fn new(
        callback: channel::Sender<NoteEvent>
    ) -> Self {
        let midi = MidiInput::new("wayfarer").unwrap();
        let ports = midi.ports();
        if let Some(port) = ports.first() {
            let name = midi.port_name(port).unwrap();
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
                name,
            }
        } else {
            MidiReader { connection: None, name: "-".to_string() }
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
