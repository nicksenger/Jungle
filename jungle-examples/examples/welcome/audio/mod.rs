mod mixer;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use futures::{channel::mpsc, SinkExt};
use tracing::{debug, error, warn};

const COMMAND_CHANNEL_CAPACITY: usize = 1024;
const ENQUEUE_WARN_THRESHOLD: Duration = Duration::from_millis(250);
const ENQUEUE_DEBUG_THRESHOLD: Duration = Duration::from_millis(50);

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
    pending_commands: Arc<AtomicUsize>,
}

impl AudioHandle {
    pub async fn play(&self, request: PlayRequest) -> Result<(), AudioError> {
        let pending_before_send = self.pending_commands.fetch_add(1, Ordering::Relaxed) + 1;
        let mut command_tx = self.command_tx.clone();
        let send_started = Instant::now();
        let send_result = command_tx
            .send(mixer::Command::Play {
                request,
                pending_commands: Arc::clone(&self.pending_commands),
            })
            .await;
        let send_elapsed = send_started.elapsed();

        if send_elapsed >= ENQUEUE_WARN_THRESHOLD {
            warn!(
                enqueue_wait_ms = send_elapsed.as_millis(),
                pending_before_send,
                current_pending = self.pending_commands.load(Ordering::Relaxed),
                "audio command enqueue is slow; queue may be backpressured"
            );
        } else if send_elapsed >= ENQUEUE_DEBUG_THRESHOLD {
            debug!(
                enqueue_wait_ms = send_elapsed.as_millis(),
                pending_before_send,
                current_pending = self.pending_commands.load(Ordering::Relaxed),
                "audio command enqueue delay observed"
            );
        }

        send_result.map_err(|err| {
            let pending_after_error = self
                .pending_commands
                .fetch_sub(1, Ordering::Relaxed)
                .saturating_sub(1);
            debug!(error = %err, "failed submitting audio command to mixer");
            debug!(
                pending_after_error,
                "decremented pending audio command count after failed enqueue"
            );
            AudioError::Submission
        })
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
        let pending_commands = Arc::new(AtomicUsize::new(0));
        let handle = AudioHandle {
            command_tx,
            pending_commands,
        };

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
        error!(error = %err, "audio output stream error");
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
        error!(error = %err, "audio output stream error");
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
        error!(error = %err, "audio output stream error");
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
