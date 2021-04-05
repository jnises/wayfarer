use crossbeam::channel;
use wmidi::MidiMessage;

// super simple synth
// TODO make interesting

type MidiChannel = channel::Receiver<MidiMessage<'static>>;

pub struct Synth {
    clock: u64,
    midi_events: MidiChannel,
    freq: f32,
    velocity: f32,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            clock: 0,
            midi_events,
            freq: 440f32,
            velocity: 0f32,
        }
    }
}

pub trait SynthPlayer {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]);
}

impl SynthPlayer for Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]) {
        for message in self.midi_events.try_iter() {
            match message {
                wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                    let norm_vel = (u8::from(velocity) - u8::from(wmidi::U7::MIN)) as f32
                        / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32;
                    self.freq = note.to_freq_f32();
                    self.velocity = norm_vel;
                }
                wmidi::MidiMessage::NoteOff(_, _note, _) => {
                    self.velocity = 0f32;
                }
                _ => {}
            }
        }
        for frame in output.chunks_mut(channels) {
            // TODO mod clock before casting
            let value = self.velocity
                * (self.clock as f32 / sample_rate as f32
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

#[cfg(test)]
mod test {
    use crossbeam::channel;
    use super::{Synth, SynthPlayer};

    #[test]
    fn silence() {
        let (_tx, rx) = channel::bounded(1);
        let mut synth = Synth::new(rx);
        let mut data = [0f32; 512];
        synth.play(48000, 2, &mut data);
        assert_eq!([0f32; 512], data);
    }
}
