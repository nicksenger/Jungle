use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::channel::mpsc::{Receiver, TryRecvError};

use super::PlayRequest;

pub(crate) enum Command {
    Play {
        request: PlayRequest,
        pending_commands: Arc<AtomicUsize>,
    },
}

pub(crate) struct AudioMixer {
    output_channels: usize,
    output_sample_rate: u32,
    frame_clock: u64,
    pending: Vec<PendingVoice>,
    pending_needs_sort: bool,
    active: Vec<Voice>,
}

impl AudioMixer {
    pub(crate) fn new(output_channels: usize, output_sample_rate: u32) -> Self {
        Self {
            output_channels: output_channels.max(1),
            output_sample_rate,
            frame_clock: 0,
            pending: Vec::new(),
            pending_needs_sort: false,
            active: Vec::new(),
        }
    }

    pub(crate) fn render_interleaved(
        &mut self,
        output: &mut [f32],
        command_rx: &mut Receiver<Command>,
    ) {
        output.fill(0.0);
        self.drain_commands(command_rx);
        self.sort_pending_if_needed();

        let frame_count = output.len() / self.output_channels;
        for frame_index in 0..frame_count {
            self.activate_pending_voices();

            let mut mixed_l = 0.0;
            let mut mixed_r = 0.0;
            let mut i = 0;
            while i < self.active.len() {
                if let Some((voice_l, voice_r)) = self.active[i].next_sample() {
                    mixed_l += voice_l;
                    mixed_r += voice_r;
                    i += 1;
                } else {
                    self.active.swap_remove(i);
                }
            }

            self.write_frame(output, frame_index, mixed_l, mixed_r);
            self.frame_clock = self.frame_clock.saturating_add(1);
        }
    }

    fn write_frame(&self, output: &mut [f32], frame_index: usize, left: f32, right: f32) {
        let base = frame_index * self.output_channels;
        if self.output_channels == 1 {
            output[base] = 0.5 * (left + right);
            return;
        }

        output[base] = clamp_unit(left);
        output[base + 1] = clamp_unit(right);

        for channel in 2..self.output_channels {
            output[base + channel] = 0.0;
        }
    }

    fn drain_commands(&mut self, command_rx: &mut Receiver<Command>) {
        loop {
            match command_rx.try_recv() {
                Ok(Command::Play {
                    request,
                    pending_commands,
                }) => {
                    let _ = pending_commands.fetch_sub(1, Ordering::Relaxed);
                    if let Some(voice) =
                        Voice::from_request(request, self.output_sample_rate, self.frame_clock)
                    {
                        self.pending.push(voice);
                        self.pending_needs_sort = true;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Closed) => break,
            }
        }
    }

    fn sort_pending_if_needed(&mut self) {
        if !self.pending_needs_sort {
            return;
        }
        // Keep earliest start frame at the end so activation is a cheap pop.
        self.pending
            .sort_unstable_by(|a, b| b.start_frame.cmp(&a.start_frame));
        self.pending_needs_sort = false;
    }

    fn activate_pending_voices(&mut self) {
        let now = self.frame_clock;
        while self
            .pending
            .last()
            .is_some_and(|pending| pending.start_frame <= now)
        {
            if let Some(pending_voice) = self.pending.pop() {
                self.active.push(pending_voice.voice);
            }
        }
    }
}

struct PendingVoice {
    start_frame: u64,
    voice: Voice,
}

struct Voice {
    pcm: Arc<[f32]>,
    source_channels: usize,
    source_frames: usize,
    frame_cursor: f64,
    frame_step: f64,
    left_gain: f32,
    right_gain: f32,
}

impl Voice {
    fn from_request(
        request: PlayRequest,
        output_sample_rate: u32,
        now_frame: u64,
    ) -> Option<PendingVoice> {
        if request.pcm.is_empty() {
            return None;
        }
        if request.source_channels == 0 || request.source_channels > 2 {
            return None;
        }
        if request.source_sample_rate == 0 || output_sample_rate == 0 {
            return None;
        }
        if !request.gain.is_finite()
            || !request.pan.is_finite()
            || !request.playback_rate.is_finite()
        {
            return None;
        }
        if request.playback_rate <= 0.0 {
            return None;
        }

        let source_channels = request.source_channels as usize;
        let source_frames = request.pcm.len() / source_channels;
        if source_frames == 0 {
            return None;
        }

        let step_ratio = request.source_sample_rate as f64 / output_sample_rate as f64;
        let frame_step = step_ratio * request.playback_rate as f64;
        if !frame_step.is_finite() || frame_step <= 0.0 {
            return None;
        }

        let pan = request.pan.clamp(-1.0, 1.0);
        let left_pan = ((1.0 - pan) * 0.5).sqrt();
        let right_pan = ((1.0 + pan) * 0.5).sqrt();
        let left_gain = request.gain * left_pan;
        let right_gain = request.gain * right_pan;

        let start_frame =
            now_frame.saturating_add(duration_to_frames(request.start_offset, output_sample_rate));

        Some(PendingVoice {
            start_frame,
            voice: Voice {
                pcm: request.pcm,
                source_channels,
                source_frames,
                frame_cursor: 0.0,
                frame_step,
                left_gain,
                right_gain,
            },
        })
    }

    fn next_sample(&mut self) -> Option<(f32, f32)> {
        if self.frame_cursor >= self.source_frames as f64 {
            return None;
        }

        let frame_index = self.frame_cursor.floor() as usize;
        let base = frame_index * self.source_channels;

        let (source_l, source_r) = if self.source_channels == 1 {
            let mono = *self.pcm.get(base)?;
            (mono, mono)
        } else {
            let left = *self.pcm.get(base)?;
            let right = *self.pcm.get(base + 1)?;
            (left, right)
        };

        self.frame_cursor += self.frame_step;
        Some((source_l * self.left_gain, source_r * self.right_gain))
    }
}

fn duration_to_frames(duration: Duration, sample_rate: u32) -> u64 {
    duration.as_secs().saturating_mul(sample_rate as u64)
        + ((duration.subsec_nanos() as u64).saturating_mul(sample_rate as u64) / 1_000_000_000)
}

fn clamp_unit(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;
    use std::sync::atomic::AtomicUsize;

    fn play_command(request: PlayRequest) -> Command {
        Command::Play {
            request,
            pending_commands: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[test]
    fn mixes_mono_voice_into_stereo_output() {
        let mut mixer = AudioMixer::new(2, 48_000);
        let (mut tx, mut rx) = mpsc::channel(8);
        let request = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 48_000,
            start_offset: Duration::ZERO,
            gain: 1.0,
            pan: 0.0,
            playback_rate: 1.0,
        };
        tx.try_send(play_command(request))
            .expect("command send should succeed");

        let mut output = vec![0.0_f32; 6];
        mixer.render_interleaved(&mut output, &mut rx);

        assert!(output[0] > 0.0);
        assert_eq!(output[0], output[1]);
        assert_eq!(output[2], output[3]);
    }

    #[test]
    fn respects_start_offset() {
        let mut mixer = AudioMixer::new(2, 10);
        let (mut tx, mut rx) = mpsc::channel(8);
        let request = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0]),
            source_channels: 1,
            source_sample_rate: 10,
            start_offset: Duration::from_millis(200),
            gain: 1.0,
            pan: 0.0,
            playback_rate: 1.0,
        };
        tx.try_send(play_command(request))
            .expect("command send should succeed");

        let mut output = vec![0.0_f32; 8];
        mixer.render_interleaved(&mut output, &mut rx);

        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert!(output[4] > 0.0);
        assert!(output[5] > 0.0);
    }

    #[test]
    fn overlaps_multiple_notes_in_same_render_window() {
        let mut mixer = AudioMixer::new(2, 10);
        let (mut tx, mut rx) = mpsc::channel(8);
        let first = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 10,
            start_offset: Duration::ZERO,
            gain: 0.4,
            pan: 0.0,
            playback_rate: 1.0,
        };
        let second = PlayRequest {
            start_offset: Duration::from_millis(100),
            ..first.clone()
        };

        tx.try_send(play_command(first))
            .expect("first command send should succeed");
        tx.try_send(play_command(second))
            .expect("second command send should succeed");

        let mut output = vec![0.0_f32; 10];
        mixer.render_interleaved(&mut output, &mut rx);

        // Frame 0: only note 1. Frame 1: note 1 + note 2 overlap.
        assert!(output[0] > 0.0);
        assert!(output[2] > output[0]);
        assert_eq!(output[2], output[3]);
    }

    #[test]
    fn overlaps_same_note_when_submitted_back_to_back() {
        let mut mixer = AudioMixer::new(2, 10);
        let (mut tx, mut rx) = mpsc::channel(8);
        let request = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 10,
            start_offset: Duration::ZERO,
            gain: 0.35,
            pan: 0.0,
            playback_rate: 1.0,
        };

        tx.try_send(play_command(request.clone()))
            .expect("first command send should succeed");
        tx.try_send(play_command(request))
            .expect("second command send should succeed");

        let mut output = vec![0.0_f32; 6];
        mixer.render_interleaved(&mut output, &mut rx);

        // Both notes start at the same frame and should both be present.
        let first_frame_left = output[0];
        assert!(first_frame_left > 0.0);
        assert_eq!(first_frame_left, output[1]);
        // One note at this gain is about 0.247 on each channel; overlap should be higher.
        assert!(first_frame_left > 0.3);
    }
}
