use anyhow::{anyhow, Result};
use crossbeam::channel;
use midir::{MidiInput, MidiInputConnection};
use std::convert::TryFrom;

// TODO send midi note instead of freq?
pub enum NoteEvent {
    On { freq: f32, velocity: f32 },
    // TODO include midi note in off so we can handle multiple keys at once
    Off,
}

pub struct MidiReader {
    // we receive midi input as long as this is alive
    #[allow(dead_code)]
    connection: Option<MidiInputConnection<()>>,
    name: String,
}

impl MidiReader {
    pub fn new(callback: channel::Sender<NoteEvent>) -> Result<Self> {
        let midi = MidiInput::new("wayfarer")?;
        let ports = midi.ports();
        Ok(if let Some(port) = ports.first() {
            let name = midi.port_name(port)?;
            let connection = midi
                .connect(
                    port,
                    &name,
                    move |_time_ms, message, _| {
                        // will panic here on bad midi message.
                        // TODO better error handling?
                        let message =
                            wmidi::MidiMessage::try_from(message).expect("bad midi message");
                        match message {
                            wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                                let norm_vel = (u8::from(velocity) - u8::from(wmidi::U7::MIN))
                                    as f32
                                    / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32;
                                // TODO just ignore failing to send the message rather than panicing?
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
                .map_err(|e| anyhow!("{}", e))?;
            MidiReader {
                connection: Some(connection),
                name,
            }
        } else {
            MidiReader {
                connection: None,
                name: "-".to_string(),
            }
        })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
