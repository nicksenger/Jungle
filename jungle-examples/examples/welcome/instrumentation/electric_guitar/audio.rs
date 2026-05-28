use welcome_audio::{AudioHandle, PlayPriority, PlayRequest};

use crate::instrumentation::{amplitude_gain, Error, Note};

use super::ElectricGuitarArticulation;

pub(super) async fn play(
    audio: &AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<ElectricGuitarArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate, pan) = synth.electric_guitar(note).await?;

    let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = pan;
    request.priority = PlayPriority::Low;

    audio.play(request).await.map_err(|_| Error::Submission)
}
