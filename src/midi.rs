use anyhow::{anyhow, Result};
use crossbeam::channel;
use log::error;
use midir::{MidiInput, MidiInputConnection};
use std::convert::TryFrom;
use wmidi::MidiMessage;

pub struct MidiReader {
    // we receive midi input as long as this is alive
    #[allow(dead_code)]
    connection: Option<MidiInputConnection<()>>,
    name: String,
}

impl MidiReader {
    pub fn new(midi_events: channel::Sender<MidiMessage<'static>>) -> Result<Self> {
        let midi = MidiInput::new("wayfarer")?;
        let ports = midi.ports();
        Ok(if let Some(port) = ports.first() {
            let name = midi.port_name(port)?;
            let connection = midi
                .connect(
                    port,
                    &name,
                    move |_time_ms, message, _| match wmidi::MidiMessage::try_from(message) {
                        Ok(message) => {
                            if let Err(e) = midi_events.try_send(message.to_owned()) {
                                error!("error sending midi event {}", e);
                            }
                        }
                        Err(e) => {
                            error!("error parsingi midi event {}", e);
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
