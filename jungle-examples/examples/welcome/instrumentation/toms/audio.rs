use welcome_audio::{PlayPriority, PlayRequest};

use crate::instrumentation::{amplitude_gain, Error, Note};

use super::TomsArticulation;

pub(super) async fn play(
    audio: &welcome_audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<TomsArticulation>,
) -> Result<(), Error> {
    let (pcm, mut gain, mut playback_rate) = synth.toms(note).await?;

    let velocity = note.velocity.clamp(0.0, 1.0);
    gain *= 0.86 + velocity * 0.42;
    playback_rate *= 0.985 + velocity * 0.045;

    let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = -0.14 + (velocity - 0.5) * 0.08;
    request.priority = PlayPriority::Normal;
    audio.play(request).await.map_err(|_| Error::Submission)
}
