mod mixer;

use std::{sync::Arc, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use futures::channel::mpsc;

const COMMAND_CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("No default output audio device was found.")]
    NoOutputDevice,
    #[error("Failed to read default output config: {0}")]
    DefaultConfig(cpal::DefaultStreamConfigError),
    #[error("Unsupported output sample format: {0:?}")]
    UnsupportedSampleFormat(SampleFormat),
    #[error("Failed to build output stream: {0}")]
    BuildStream(cpal::BuildStreamError),
    #[error("Failed to start output stream: {0}")]
    StartStream(cpal::PlayStreamError),
    #[error("Audio command submission failed.")]
    Submission,
}

#[derive(Clone)]
pub struct AudioHandle {
    command_tx: mpsc::Sender<mixer::Command>,
}

impl AudioHandle {
    pub fn try_play(&self, request: PlayRequest) -> Result<(), AudioError> {
        let mut command_tx = self.command_tx.clone();
        command_tx
            .try_send(mixer::Command::Play(request))
            .map_err(|_| AudioError::Submission)
    }
}

pub struct AudioEngine {
    handle: AudioHandle,
    _stream: Stream,
}

impl AudioEngine {
    pub async fn start_default() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;
        let supported_config = device
            .default_output_config()
            .map_err(AudioError::DefaultConfig)?;
        let stream_config: cpal::StreamConfig = supported_config.config();

        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
        let handle = AudioHandle { command_tx };

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => build_stream_f32(&device, &stream_config, command_rx)?,
            SampleFormat::I16 => build_stream_i16(&device, &stream_config, command_rx)?,
            SampleFormat::U16 => build_stream_u16(&device, &stream_config, command_rx)?,
            sample_format => return Err(AudioError::UnsupportedSampleFormat(sample_format)),
        };

        stream.play().map_err(AudioError::StartStream)?;

        Ok(Self {
            handle,
            _stream: stream,
        })
    }

    pub fn handle(&self) -> AudioHandle {
        self.handle.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PlayRequest {
    pub pcm: Arc<[f32]>,
    pub source_channels: u16,
    pub source_sample_rate: u32,
    pub start_offset: Duration,
    pub gain: f32,
    pub pan: f32,
    pub playback_rate: f32,
}

impl PlayRequest {
    pub fn new(pcm: Arc<[f32]>, source_channels: u16, source_sample_rate: u32) -> Self {
        Self {
            pcm,
            source_channels,
            source_sample_rate,
            start_offset: Duration::ZERO,
            gain: 1.0,
            pan: 0.0,
            playback_rate: 1.0,
        }
    }
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut command_rx: mpsc::Receiver<mixer::Command>,
) -> Result<Stream, AudioError> {
    let mut mixer = mixer::AudioMixer::new(config.channels as usize, config.sample_rate);
    let error_callback = |err: cpal::StreamError| {
        eprintln!("audio output stream error: {err}");
    };

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                mixer.render_interleaved(data, &mut command_rx);
            },
            error_callback,
            None,
        )
        .map_err(AudioError::BuildStream)
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut command_rx: mpsc::Receiver<mixer::Command>,
) -> Result<Stream, AudioError> {
    let mut mixer = mixer::AudioMixer::new(config.channels as usize, config.sample_rate);
    let mut scratch = Vec::<f32>::new();
    let error_callback = |err: cpal::StreamError| {
        eprintln!("audio output stream error: {err}");
    };

    device
        .build_output_stream(
            config,
            move |data: &mut [i16], _| {
                scratch.resize(data.len(), 0.0);
                mixer.render_interleaved(&mut scratch, &mut command_rx);
                for (dst, mixed) in data.iter_mut().zip(scratch.iter().copied()) {
                    *dst = (mixed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                }
            },
            error_callback,
            None,
        )
        .map_err(AudioError::BuildStream)
}

fn build_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut command_rx: mpsc::Receiver<mixer::Command>,
) -> Result<Stream, AudioError> {
    let mut mixer = mixer::AudioMixer::new(config.channels as usize, config.sample_rate);
    let mut scratch = Vec::<f32>::new();
    let error_callback = |err: cpal::StreamError| {
        eprintln!("audio output stream error: {err}");
    };

    device
        .build_output_stream(
            config,
            move |data: &mut [u16], _| {
                scratch.resize(data.len(), 0.0);
                mixer.render_interleaved(&mut scratch, &mut command_rx);
                for (dst, mixed) in data.iter_mut().zip(scratch.iter().copied()) {
                    let normalized = mixed.clamp(-1.0, 1.0);
                    *dst = (((normalized + 1.0) * 0.5) * u16::MAX as f32) as u16;
                }
            },
            error_callback,
            None,
        )
        .map_err(AudioError::BuildStream)
}
