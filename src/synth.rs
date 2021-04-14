use std::f32::consts::PI;

use crossbeam::channel;
use wmidi::MidiMessage;

// super simple synth
// TODO make interesting

type MidiChannel = channel::Receiver<MidiMessage<'static>>;

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
    velocity: wmidi::U7,
    time: u64,
}

#[derive(Clone)]
pub struct Synth {
    clock: u64,
    midi_events: MidiChannel,

    note_event: Option<NoteEvent>,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            clock: 0,
            midi_events,
            note_event: None,
        }
    }
}

pub trait SynthPlayer {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]);
}

impl SynthPlayer for Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]) {
        // pump midi messages
        for message in self.midi_events.try_iter() {
            match message {
                wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                    self.note_event = Some(NoteEvent {
                        note,
                        velocity,
                        time: self.clock,
                    });
                }
                wmidi::MidiMessage::NoteOff(_, note, _) => {
                    if let Some(NoteEvent {
                        note: held_note, ..
                    }) = self.note_event
                    {
                        if note == held_note {
                            self.note_event = None;
                        }
                    }
                }
                _ => {}
            }
        }

        // produce sound
        if let Some(NoteEvent {
            note,
            velocity,
            time: note_start,
        }) = self.note_event
        {
            for frame in output.chunks_exact_mut(channels) {
                let time = (self.clock - note_start) as f32 / sample_rate as f32;
                let norm_vel = (u8::from(velocity) - u8::from(wmidi::U7::MIN)) as f32
                    / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32;
                let freq = note.to_freq_f32();
                let value = norm_vel * (time * freq * 2f32 * PI).sin();
                for sample in frame.iter_mut() {
                    *sample = value;
                }
                self.clock += 1;
            }
        } else {
            output.fill(0f32);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Synth, SynthPlayer};
    use crossbeam::channel;

    #[test]
    fn silence() {
        let (_tx, rx) = channel::bounded(1);
        let mut synth = Synth::new(rx);
        let mut data = [0f32; 512];
        synth.play(48000, 2, &mut data);
        assert_eq!([0f32; 512], data);
    }
}
