use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
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
    active: Vec<Voice>,
}

impl AudioMixer {
    pub(crate) fn new(output_channels: usize, output_sample_rate: u32) -> Self {
        Self {
            output_channels: output_channels.max(1),
            output_sample_rate,
            active: Vec::new(),
        }
    }

    pub(crate) fn render_interleaved(
        &mut self,
        output: &mut [f32],
        critical_command_rx: &mut Receiver<Command>,
        standard_command_rx: &mut Receiver<Command>,
    ) {
        output.fill(0.0);
        self.drain_commands(critical_command_rx);
        self.drain_commands(standard_command_rx);

        let frame_count = output.len() / self.output_channels;
        for frame_index in 0..frame_count {
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
                    if let Some(voice) = Voice::from_request(request, self.output_sample_rate) {
                        self.active.push(voice);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Closed) => break,
            }
        }
    }
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
    fn from_request(request: PlayRequest, output_sample_rate: u32) -> Option<Self> {
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

        Some(Voice {
            pcm: request.pcm,
            source_channels,
            source_frames,
            frame_cursor: 0.0,
            frame_step,
            left_gain,
            right_gain,
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
        let (mut critical_tx, mut critical_rx) = mpsc::channel(8);
        let (_standard_tx, mut standard_rx) = mpsc::channel(8);
        let request = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 48_000,
            gain: 1.0,
            pan: 0.0,
            playback_rate: 1.0,
            priority: super::PlayPriority::Normal,
        };
        critical_tx
            .try_send(play_command(request))
            .expect("command send should succeed");

        let mut output = vec![0.0_f32; 6];
        mixer.render_interleaved(&mut output, &mut critical_rx, &mut standard_rx);

        assert!(output[0] > 0.0);
        assert_eq!(output[0], output[1]);
        assert_eq!(output[2], output[3]);
    }

    #[test]
    fn overlaps_multiple_notes_in_same_render_window() {
        let mut mixer = AudioMixer::new(2, 10);
        let (mut critical_tx, mut critical_rx) = mpsc::channel(8);
        let (_standard_tx, mut standard_rx) = mpsc::channel(8);
        let first = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 10,
            gain: 0.4,
            pan: 0.0,
            playback_rate: 1.0,
            priority: super::PlayPriority::Normal,
        };
        let second = first.clone();

        critical_tx
            .try_send(play_command(first))
            .expect("first command send should succeed");
        critical_tx
            .try_send(play_command(second))
            .expect("second command send should succeed");

        let mut output = vec![0.0_f32; 10];
        mixer.render_interleaved(&mut output, &mut critical_rx, &mut standard_rx);

        // Both notes start immediately and overlap in frame 0.
        assert!(output[0] > 0.0);
        assert_eq!(output[0], output[1]);
        assert!(output[0] > 0.3);
    }

    #[test]
    fn overlaps_same_note_when_submitted_back_to_back() {
        let mut mixer = AudioMixer::new(2, 10);
        let (mut critical_tx, mut critical_rx) = mpsc::channel(8);
        let (_standard_tx, mut standard_rx) = mpsc::channel(8);
        let request = PlayRequest {
            pcm: Arc::from([1.0_f32, 1.0, 1.0]),
            source_channels: 1,
            source_sample_rate: 10,
            gain: 0.35,
            pan: 0.0,
            playback_rate: 1.0,
            priority: super::PlayPriority::Normal,
        };

        critical_tx
            .try_send(play_command(request.clone()))
            .expect("first command send should succeed");
        critical_tx
            .try_send(play_command(request))
            .expect("second command send should succeed");

        let mut output = vec![0.0_f32; 6];
        mixer.render_interleaved(&mut output, &mut critical_rx, &mut standard_rx);

        // Both notes start at the same frame and should both be present.
        let first_frame_left = output[0];
        assert!(first_frame_left > 0.0);
        assert_eq!(first_frame_left, output[1]);
        // One note at this gain is about 0.247 on each channel; overlap should be higher.
        assert!(first_frame_left > 0.3);
    }
}
