use std::{sync::Arc, time::Duration};

use welcome_audio::{PlayPriority, PlayRequest};

use crate::instrumentation::{amplitude_gain, Error, Note};

use super::VocalsArticulation;

pub(super) async fn play(
    audio: &welcome_audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<VocalsArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = synth.vocals(note).await?;

    for layer in
        welcome_audio::dsp::vocals::articulation_layers(to_dsp_articulation(note.articulation))
    {
        if layer.delay_seconds > 0.0 {
            tokio::time::sleep(Duration::from_secs_f32(layer.delay_seconds)).await;
        }
        let mut request = PlayRequest::new(Arc::clone(&pcm), 1, welcome_audio::dsp::SAMPLE_RATE);
        request.gain = gain * layer.gain_scale * amplitude_gain(&note);
        request.playback_rate = playback_rate * layer.playback_rate_scale;
        request.pan = layer.pan;
        request.priority = PlayPriority::Low;
        audio.play(request).await.map_err(|_| Error::Submission)?;
    }

    Ok(())
}

fn to_dsp_articulation(
    articulation: VocalsArticulation,
) -> welcome_audio::dsp::vocals::VocalsArticulation {
    match articulation {
        VocalsArticulation::Clean => welcome_audio::dsp::vocals::VocalsArticulation::Clean,
        VocalsArticulation::GroupHarmony => {
            welcome_audio::dsp::vocals::VocalsArticulation::GroupHarmony
        }
        VocalsArticulation::Formant(phonemes) => {
            welcome_audio::dsp::vocals::VocalsArticulation::Formant(phonemes)
        }
    }
}
