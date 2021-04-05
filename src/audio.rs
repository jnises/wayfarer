use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, SampleFormat, SampleRate,
};

use std::thread::{self, JoinHandle};

use crossbeam::channel;

use crate::message::Message;
use crate::midi::NoteEvent;
use crate::synth::Synth;

pub struct AudioManager {
    handle: Option<JoinHandle<()>>,
    shutdown: channel::Sender<()>,
}

impl AudioManager {
    pub fn new(
        midi_events: channel::Receiver<NoteEvent>,
        message_sender: channel::Sender<Message>,
    ) -> Self {
        let (tx, rx) = channel::bounded(1);
        // run this in a thread since it causes errors if run before the gui on a thread
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
                                    synth.set_freq(freq);
                                    synth.set_amplitude(velocity);
                                }
                                NoteEvent::Off => {
                                    synth.set_amplitude(0f32);
                                }
                            }
                        }
                        synth.play(data);
                    },
                    move |error| {
                        message_sender_clone
                            .send(Message::Status(format!("error: {:?}", error)))
                            .unwrap();
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
