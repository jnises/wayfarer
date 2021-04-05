// super simple synth
// TODO make interesting

pub struct Synth {
    sample_rate: u32,
    channels: usize,
    clock: u64,
    freq: f32,
    amplitude: f32,
}

impl Synth {
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate,
            channels,
            clock: 0,
            freq: 0f32,
            amplitude: 0f32,
        }
    }

    pub fn play(&mut self, output: &mut [f32]) {
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

    pub fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude;
    }
}
