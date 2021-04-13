use std::sync::Arc;

use crate::synth::SynthPlayer;
use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, SampleFormat, Stream,
};

pub struct AudioManager<T> {
    device: Option<Device>,
    stream: Option<Stream>,
    error_callback: Arc<Box<dyn Fn(String) -> () + Send + Sync>>,
    synth: T,
}

impl<T> AudioManager<T>
where
    T: SynthPlayer + Clone + Send + 'static,
{
    pub fn new<U>(synth: T, error_callback: U) -> Self
    where
        U: Fn(String) -> () + Send + Sync + 'static,
    {
        let mut s = Self {
            device: None,
            stream: None,
            error_callback: Arc::new(Box::new(error_callback)),
            synth,
        };
        s.setup();
        s
    }

    pub fn get_devices(&self) -> Vec<Device> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.collect(),
            Err(_) => vec![],
        }
    }

    pub fn set_device(&mut self, device: Device) {
        if self.device.as_ref().and_then(|d| d.name().ok()) != device.name().ok() {
            self.stream = None;
            self.device = Some(device);
            self.setup();
        }
    }

    fn setup(&mut self) {
        self.stream = None;
        if self.device.is_none() {
            let host = cpal::default_host();
            self.device = host.default_output_device();
        }
        if let Some(ref device) = self.device {
            // emulate try block
            match (|| -> Result<_> {
                let supported_config = device
                    .supported_output_configs()?
                    .filter(|config| {
                        // only stereo configs
                        config.sample_format() == SampleFormat::F32 && config.channels() == 2
                    })
                    // just pick the first valid config
                    .next()
                    .ok_or_else(|| anyhow!("no valid output audio config found"))?;
                let sample_rate = device.default_output_config()?.sample_rate().clamp(
                    supported_config.min_sample_rate(),
                    supported_config.max_sample_rate(),
                );
                let config = supported_config
                    .with_sample_rate(sample_rate)
                    // TODO make buffer size configurable
                    .config();
                let sample_rate = config.sample_rate.0;
                let channels = config.channels.into();
                let mut synth = self.synth.clone();
                let error_callback = self.error_callback.clone();
                let stream = device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        synth.play(sample_rate, channels, data);
                    },
                    move |error| {
                        error_callback(format!("error: {:?}", error));
                    },
                )?;
                stream.play()?;
                Ok(stream)
            })() {
                Ok(stream) => {
                    self.stream = Some(stream);
                }
                Err(e) => {
                    (self.error_callback)(format!("error: {:?}", e));
                }
            }
        }
    }

    pub fn get_name(&self) -> Option<String> {
        self.device.as_ref().and_then(|d| d.name().ok())
    }
}
