mod mixer;
pub mod vocals;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use futures::channel::mpsc;
use tracing::{debug, error, trace, warn};

const COMMAND_CHANNEL_CAPACITY_CRITICAL: usize = 512;
const COMMAND_CHANNEL_CAPACITY_STANDARD: usize = 1024;
const DROP_LOG_INTERVAL: usize = 128;
const AUDIO_ENQUEUE_LOG_INTERVAL: usize = 512;
const AUDIO_ENQUEUE_SLOW_WARN_THRESHOLD: Duration = Duration::from_millis(5);

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
    critical_command_tx: mpsc::Sender<mixer::Command>,
    standard_command_tx: mpsc::Sender<mixer::Command>,
    pending_commands: Arc<AtomicUsize>,
    dropped_commands: Arc<AtomicUsize>,
    enqueue_attempts: Arc<AtomicUsize>,
    enqueue_successes: Arc<AtomicUsize>,
    max_pending_commands: Arc<AtomicUsize>,
    playback_delay: Duration,
}

impl AudioHandle {
    pub async fn play(&self, request: PlayRequest) -> Result<(), AudioError> {
        let enqueue_started_at = Instant::now();
        let mut request = request;
        request.start_delay = self.playback_delay;
        let priority = request.priority;
        let enqueue_attempt = self.enqueue_attempts.fetch_add(1, Ordering::Relaxed) + 1;
        let pending_before_send = self.pending_commands.fetch_add(1, Ordering::Relaxed) + 1;
        update_max_pending(&self.max_pending_commands, pending_before_send);
        let mut command_tx = match priority {
            PlayPriority::Critical => self.critical_command_tx.clone(),
            PlayPriority::Normal | PlayPriority::Low => self.standard_command_tx.clone(),
        };
        let command = mixer::Command::Play {
            request,
            pending_commands: Arc::clone(&self.pending_commands),
        };
        match command_tx.try_send(command) {
            Ok(()) => {
                let enqueue_elapsed = enqueue_started_at.elapsed();
                let success_count = self.enqueue_successes.fetch_add(1, Ordering::Relaxed) + 1;
                if enqueue_elapsed > AUDIO_ENQUEUE_SLOW_WARN_THRESHOLD {
                    warn!(
                        priority = ?priority,
                        enqueue_elapsed_ms = enqueue_elapsed.as_millis(),
                        pending_before_send,
                        max_pending_commands = self.max_pending_commands.load(Ordering::Relaxed),
                        "slow audio enqueue observed"
                    );
                } else if success_count % AUDIO_ENQUEUE_LOG_INTERVAL == 0 {
                    debug!(
                        priority = ?priority,
                        enqueue_attempt,
                        success_count,
                        pending_before_send,
                        max_pending_commands = self.max_pending_commands.load(Ordering::Relaxed),
                        dropped_commands = self.dropped_commands.load(Ordering::Relaxed),
                        "audio enqueue heartbeat"
                    );
                } else {
                    trace!(
                        priority = ?priority,
                        enqueue_elapsed_us = enqueue_elapsed.as_micros(),
                        "audio enqueue complete"
                    );
                }
                Ok(())
            }
            Err(err) if err.is_full() => {
                let pending_after_drop = self
                    .pending_commands
                    .fetch_sub(1, Ordering::Relaxed)
                    .saturating_sub(1);
                let dropped_total = self.dropped_commands.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped_total % DROP_LOG_INTERVAL == 0 {
                    warn!(
                        dropped_total,
                        enqueue_attempt,
                        pending_before_send,
                        pending_after_drop,
                        max_pending_commands = self.max_pending_commands.load(Ordering::Relaxed),
                        priority = ?priority,
                        "audio command queue full; dropping queued note"
                    );
                } else {
                    debug!(
                        dropped_total,
                        enqueue_attempt,
                        pending_before_send,
                        pending_after_drop,
                        max_pending_commands = self.max_pending_commands.load(Ordering::Relaxed),
                        priority = ?priority,
                        "audio command dropped due to queue pressure"
                    );
                }
                Ok(())
            }
            Err(err) => {
                let pending_after_error = self
                    .pending_commands
                    .fetch_sub(1, Ordering::Relaxed)
                    .saturating_sub(1);
                debug!(error = %err, "failed submitting audio command to mixer");
                debug!(
                    enqueue_attempt,
                    pending_after_error,
                    "decremented pending audio command count after failed enqueue"
                );
                Err(AudioError::Submission)
            }
        }
    }

    pub fn with_playback_delay(mut self, playback_delay: Duration) -> Self {
        self.playback_delay = playback_delay;
        self
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

        let (critical_command_tx, critical_command_rx) =
            mpsc::channel(COMMAND_CHANNEL_CAPACITY_CRITICAL);
        let (standard_command_tx, standard_command_rx) =
            mpsc::channel(COMMAND_CHANNEL_CAPACITY_STANDARD);
        let pending_commands = Arc::new(AtomicUsize::new(0));
        let dropped_commands = Arc::new(AtomicUsize::new(0));
        let handle = AudioHandle {
            critical_command_tx,
            standard_command_tx,
            pending_commands,
            dropped_commands,
            enqueue_attempts: Arc::new(AtomicUsize::new(0)),
            enqueue_successes: Arc::new(AtomicUsize::new(0)),
            max_pending_commands: Arc::new(AtomicUsize::new(0)),
            playback_delay: Duration::ZERO,
        };

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => build_stream_f32(
                &device,
                &stream_config,
                critical_command_rx,
                standard_command_rx,
            )?,
            SampleFormat::I16 => build_stream_i16(
                &device,
                &stream_config,
                critical_command_rx,
                standard_command_rx,
            )?,
            SampleFormat::U16 => build_stream_u16(
                &device,
                &stream_config,
                critical_command_rx,
                standard_command_rx,
            )?,
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

pub struct StubAudioKeepAlive {
    _critical_command_rx: mpsc::Receiver<mixer::Command>,
    _standard_command_rx: mpsc::Receiver<mixer::Command>,
}

impl AudioHandle {
    pub fn stub() -> (Self, StubAudioKeepAlive) {
        let (critical_command_tx, critical_command_rx) =
            mpsc::channel(COMMAND_CHANNEL_CAPACITY_CRITICAL);
        let (standard_command_tx, standard_command_rx) =
            mpsc::channel(COMMAND_CHANNEL_CAPACITY_STANDARD);
        (
            Self {
                critical_command_tx,
                standard_command_tx,
                pending_commands: Arc::new(AtomicUsize::new(0)),
                dropped_commands: Arc::new(AtomicUsize::new(0)),
                enqueue_attempts: Arc::new(AtomicUsize::new(0)),
                enqueue_successes: Arc::new(AtomicUsize::new(0)),
                max_pending_commands: Arc::new(AtomicUsize::new(0)),
                playback_delay: Duration::ZERO,
            },
            StubAudioKeepAlive {
                _critical_command_rx: critical_command_rx,
                _standard_command_rx: standard_command_rx,
            },
        )
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
    pub start_delay: Duration,
    pub priority: PlayPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayPriority {
    Critical,
    Normal,
    Low,
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
            start_delay: Duration::ZERO,
            priority: PlayPriority::Normal,
        }
    }
}

fn update_max_pending(max_pending_commands: &AtomicUsize, pending: usize) {
    let mut current = max_pending_commands.load(Ordering::Relaxed);
    while pending > current {
        match max_pending_commands.compare_exchange_weak(
            current,
            pending,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(updated) => current = updated,
        }
    }
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut critical_command_rx: mpsc::Receiver<mixer::Command>,
    mut standard_command_rx: mpsc::Receiver<mixer::Command>,
) -> Result<Stream, AudioError> {
    let mut mixer = mixer::AudioMixer::new(config.channels as usize, config.sample_rate);
    let error_callback = |err: cpal::StreamError| {
        error!(error = %err, "audio output stream error");
    };

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                mixer.render_interleaved(data, &mut critical_command_rx, &mut standard_command_rx);
            },
            error_callback,
            None,
        )
        .map_err(AudioError::BuildStream)
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut critical_command_rx: mpsc::Receiver<mixer::Command>,
    mut standard_command_rx: mpsc::Receiver<mixer::Command>,
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
                mixer.render_interleaved(
                    &mut scratch,
                    &mut critical_command_rx,
                    &mut standard_command_rx,
                );
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
    mut critical_command_rx: mpsc::Receiver<mixer::Command>,
    mut standard_command_rx: mpsc::Receiver<mixer::Command>,
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
                mixer.render_interleaved(
                    &mut scratch,
                    &mut critical_command_rx,
                    &mut standard_command_rx,
                );
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
