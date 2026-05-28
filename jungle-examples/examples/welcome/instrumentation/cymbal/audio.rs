use welcome_audio::{PlayPriority, PlayRequest};

use crate::instrumentation::{amplitude_gain, Error, Note};

use super::CymbalArticulation;

pub(super) async fn play(
    audio: &welcome_audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<CymbalArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = synth.cymbal(note).await?;

    let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.25;
    request.priority = PlayPriority::Normal;
    audio.play(request).await.map_err(|_| Error::Submission)
}
