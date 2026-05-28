use welcome_audio::{PlayPriority, PlayRequest};

use crate::instrumentation::{amplitude_gain, Error, Note};

use super::SnareDrumArticulation;

pub(super) async fn play(
    audio: &welcome_audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<SnareDrumArticulation>,
) -> Result<(), Error> {
    let (pcm, mut gain, mut playback_rate) = synth.snare_drum(note).await?;

    let velocity = note.velocity.clamp(0.0, 1.0);
    gain *= 0.88 + velocity * 0.52;
    playback_rate *= 0.98 + velocity * 0.06;

    let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.08 + (velocity - 0.5) * 0.06;
    request.priority = PlayPriority::Critical;
    audio.play(request).await.map_err(|_| Error::Submission)
}
