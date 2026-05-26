use std::{f32::consts::TAU, sync::Arc, time::Duration};

use tracing::warn;

use crate::audio::{PlayPriority, PlayRequest};
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, SAMPLE_RATE},
    Error, Expression, Note,
};

use super::VocalsArticulation;

pub(super) fn text_to_phonemes(text: &str) -> String {
    let mut tokens = Vec::new();
    let mut chars = text
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .peekable();

    while let Some(ch) = chars.next() {
        let next = chars.peek().copied();
        let token = match (ch, next) {
            ('C', Some('H')) => {
                chars.next();
                "CH"
            }
            ('S', Some('H')) => {
                chars.next();
                "SH"
            }
            ('T', Some('H')) => {
                chars.next();
                "TH"
            }
            ('P', Some('H')) => {
                chars.next();
                "F*"
            }
            ('W', Some('H')) => {
                chars.next();
                "WH"
            }
            ('N', Some('G')) => {
                chars.next();
                "NX"
            }
            ('Q', Some('U')) => {
                chars.next();
                "K*"
            }
            ('A', _) => "AE",
            ('E', _) => "EH",
            ('I', _) => "IH",
            ('O', _) => "OW",
            ('U', _) => "UH",
            ('Y', _) => "IY",
            ('B', _) => "B*",
            ('C', _) => "K*",
            ('D', _) => "D*",
            ('F', _) => "F*",
            ('G', _) => "G*",
            ('H', _) => "/H",
            ('J', _) => "J*",
            ('K', _) => "K*",
            ('L', _) => "L*",
            ('M', _) => "M*",
            ('N', _) => "N*",
            ('P', _) => "P*",
            ('R', _) => "R*",
            ('S', _) => "S*",
            ('T', _) => "T*",
            ('V', _) => "V*",
            ('W', _) => "W*",
            ('X', _) => "K*",
            ('Z', _) => "Z*",
            _ => continue,
        };
        tokens.push(token);
    }

    tokens.join(" ")
}

pub(super) fn parse_phonemes(text: &str) -> Result<Vec<inline_rustsam::Phoneme>, inline_rustsam::ParseError> {
    inline_rustsam::parse_phonemes(text)
}

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<VocalsArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = synth.vocals(note).await?;

    for layer in articulation_layers(note.articulation) {
        if layer.delay_seconds > 0.0 {
            tokio::time::sleep(Duration::from_secs_f32(layer.delay_seconds)).await;
        }
        let mut request = PlayRequest::new(Arc::clone(&pcm), 1, SAMPLE_RATE);
        request.gain = gain * layer.gain_scale * amplitude_gain(&note);
        request.playback_rate = playback_rate * layer.playback_rate_scale;
        request.pan = layer.pan;
        request.priority = PlayPriority::Low;
        audio.play(request).await.map_err(|_| Error::Submission)?;
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct PlaybackLayer {
    pan: f32,
    gain_scale: f32,
    playback_rate_scale: f32,
    delay_seconds: f32,
}

fn articulation_layers(articulation: VocalsArticulation) -> &'static [PlaybackLayer] {
    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => &[
            PlaybackLayer {
                pan: 0.02,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.17,
                gain_scale: 0.26,
                playback_rate_scale: 0.998,
                delay_seconds: 0.008,
            },
            PlaybackLayer {
                pan: 0.21,
                gain_scale: 0.21,
                playback_rate_scale: 1.004,
                delay_seconds: 0.015,
            },
        ],
        VocalsArticulation::GroupHarmony => &[
            PlaybackLayer {
                pan: -0.42,
                gain_scale: 0.7,
                playback_rate_scale: 0.992,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 0.9,
                playback_rate_scale: 1.0,
                delay_seconds: 0.011,
            },
            PlaybackLayer {
                pan: 0.38,
                gain_scale: 0.68,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
    }
}

pub(in crate::instrumentation) fn synthesize_vocals(
    note: &Note<VocalsArticulation>,
) -> (Arc<[f32]>, f32, f32) {
    if let VocalsArticulation::Formant(phonemes) = note.articulation {
        if let Some(formant) = synthesize_formant_vocals(note, phonemes) {
            return formant;
        }
    }

    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(85.0, 1_300.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, base_hz, phase, t, expression);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn synthesize_formant_vocals(
    note: &Note<VocalsArticulation>,
    phonemes: [Option<super::Phoneme>; 12],
) -> Option<(Arc<[f32]>, f32, f32)> {
    const FORMANT_TARGET_RMS: f32 = 0.22;
    const FORMANT_TARGET_PEAK: f32 = 0.92;
    const FORMANT_MAX_MAKEUP_GAIN: f32 = 4.0;

    let phonemes: Vec<inline_rustsam::Phoneme> = phonemes
        .into_iter()
        .flatten()
        .map(|phoneme| inline_rustsam::Phoneme {
            length: phoneme.length,
            index: phoneme.index,
            stress: phoneme.stress,
        })
        .collect();

    if phonemes.is_empty() {
        return None;
    }

    let duration = articulation_duration(note.duration, note.articulation);
    let voice = inline_rustsam::VoiceParams::default();
    let rendered = inline_rustsam::render_vocal_note(
        inline_rustsam::VocalNote {
            midi_note: note.n_midi,
            phonemes,
            duration,
        },
        SAMPLE_RATE,
        voice,
    );

    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(err) => {
            warn!(error = %err, "failed to render rustsam formant vocals; using procedural fallback");
            return None;
        }
    };

    let mut normalized: Vec<f32> = rendered
        .into_iter()
        .map(|sample| sample as f32 / 127.5 - 1.0)
        .collect();

    let peak_abs = normalized.iter().fold(0.0_f32, |acc, sample| {
        if sample.abs() > acc {
            sample.abs()
        } else {
            acc
        }
    });
    let rms = if normalized.is_empty() {
        0.0
    } else {
        (normalized.iter().map(|sample| sample * sample).sum::<f32>() / normalized.len() as f32)
            .sqrt()
    };

    if peak_abs > 1.0e-4 && rms > 1.0e-4 {
        let rms_gain = FORMANT_TARGET_RMS / rms;
        let peak_gain = FORMANT_TARGET_PEAK / peak_abs;
        let makeup_gain = rms_gain.min(peak_gain).clamp(1.0, FORMANT_MAX_MAKEUP_GAIN);
        if makeup_gain > 1.0 {
            for sample in &mut normalized {
                *sample = (*sample * makeup_gain).clamp(-1.0, 1.0);
            }
        }
    }

    let velocity = note.velocity.clamp(0.0, 1.0);
    let pcm: Vec<f32> = normalized
        .into_iter()
        .map(|sample| sample * velocity)
        .collect();
    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    Some((Arc::from(pcm), gain, playback_rate))
}

fn articulation_duration(base: Duration, articulation: VocalsArticulation) -> Duration {
    let scale = match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => 1.05,
        VocalsArticulation::GroupHarmony => 1.0,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: VocalsArticulation) -> (f32, f32) {
    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => (0.83, 1.0),
        VocalsArticulation::GroupHarmony => (0.78, 1.0),
    }
}

fn articulation_sample(
    articulation: VocalsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let vibrato = (TAU * 5.7 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.02;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.15;

    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => {
            let f = base_hz * (1.0 + bend + vibrato);
            reed_formant(f, t, phase)
        }
        VocalsArticulation::GroupHarmony => {
            let f = base_hz.clamp(90.0, 880.0);
            let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
            unison + sine(f * 2.0, t) * 0.15 + hash_noise(t * 4_500.0) * 0.08
        }
    }
}

fn reed_formant(frequency_hz: f32, t: f32, phase: f32) -> f32 {
    let mouthpiece = smoothstep((phase / 0.12).clamp(0.0, 1.0));
    let body = sine(frequency_hz, t) * 0.62 + saw(frequency_hz, t) * 0.17;
    let warmth = sine(frequency_hz * 2.0, t) * 0.18 + sine(frequency_hz * 3.0, t) * 0.08;
    let breath = hash_noise(t * 6_100.0) * (0.03 + 0.03 * (1.0 - mouthpiece));
    (body + warmth + breath).tanh()
}

fn articulation_envelope(articulation: VocalsArticulation, phase: f32) -> f32 {
    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => {
            let attack = 0.03;
            let body = 0.9;
            let release_start = 0.8;
            let release = 0.18;
            let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
            let release_phase = ((phase - release_start) / release).clamp(0.0, 1.0);
            let release_env = 1.0 - smoothstep(release_phase);
            (attack_env * body * release_env).clamp(0.0, 1.0)
        }
        VocalsArticulation::GroupHarmony => {
            let attack = 0.04;
            let decay = 0.45;
            let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
            let decay_env = (-phase * decay * 4.0).exp();
            (attack_env * decay_env).clamp(0.0, 1.0)
        }
    }
}

mod inline_rustsam {
use std::cmp::Ordering;
use std::time::Duration;

#[derive(Debug)]
pub enum ParseError {
    // TODO: Cases
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Parse error")
    }
}

#[derive(Debug)]
pub struct ParseResult {
    pub phonemes: Vec<Phoneme>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phoneme {
    pub length: u8,
    pub index: usize,
    pub stress: u8,
}

impl Phoneme {
    fn has_flag(&self, flag: u16) -> bool {
        PHONEME_FLAGS[self.index] & flag != 0
    }
}

impl ParseResult {
    fn new() -> Self {
        Self {
            phonemes: Vec::new(),
        }
    }
}

const STRESS_TABLE: &[char] = &['*', '1', '2', '3', '4', '5', '6', '7', '8'];

const PHONEME_NAME_TABLE: &[(char, char)] = &[
    (' ', '*'), // 00
    ('.', '*'), // 01
    ('?', '*'), // 02
    (',', '*'), // 03
    ('-', '*'), // 04
    ('I', 'Y'), // 05
    ('I', 'H'), // 06
    ('E', 'H'), // 07
    ('A', 'E'), // 08
    ('A', 'A'), // 09
    ('A', 'H'), // 10
    ('A', 'O'), // 11
    ('U', 'H'), // 12
    ('A', 'X'), // 13
    ('I', 'X'), // 14
    ('E', 'R'), // 15
    ('U', 'X'), // 16
    ('O', 'H'), // 17
    ('R', 'X'), // 18
    ('L', 'X'), // 19
    ('W', 'X'), // 20
    ('Y', 'X'), // 21
    ('W', 'H'), // 22
    ('R', '*'), // 23
    ('L', '*'), // 24
    ('W', '*'), // 25
    ('Y', '*'), // 26
    ('M', '*'), // 27
    ('N', '*'), // 28
    ('N', 'X'), // 29
    ('D', 'X'), // 30
    ('Q', '*'), // 31
    ('S', '*'), // 32
    ('S', 'H'), // 33
    ('F', '*'), // 34
    ('T', 'H'), // 35
    ('/', 'H'), // 36
    ('/', 'X'), // 37
    ('Z', '*'), // 38
    ('Z', 'H'), // 39
    ('V', '*'), // 40
    ('D', 'H'), // 41
    ('C', 'H'), // 42
    ('*', '*'), // 43
    ('J', '*'), // 44
    ('*', '*'), // 45
    ('*', '*'), // 46
    ('*', '*'), // 47
    ('E', 'Y'), // 48
    ('A', 'Y'), // 49
    ('O', 'Y'), // 50
    ('A', 'W'), // 51
    ('O', 'W'), // 52
    ('U', 'W'), // 53
    ('B', '*'), // 54
    ('*', '*'), // 55
    ('*', '*'), // 56
    ('D', '*'), // 57
    ('*', '*'), // 58
    ('*', '*'), // 59
    ('G', '*'), // 60
    ('*', '*'), // 61
    ('*', '*'), // 62
    ('G', 'X'), // 63
    ('*', '*'), // 64
    ('*', '*'), // 65
    ('P', '*'), // 66
    ('*', '*'), // 67
    ('*', '*'), // 68
    ('T', '*'), // 69
    ('*', '*'), // 70
    ('*', '*'), // 71
    ('K', '*'), // 72
    ('*', '*'), // 73
    ('*', '*'), // 74
    ('K', 'X'), // 75
    ('*', '*'), // 76
    ('*', '*'), // 77
    ('U', 'L'), // 78
    ('U', 'M'), // 79
    ('U', 'N'), // 80
];

const PHONEME_LENGTH_TABLE: &[(u8, u8)] = &[
    (0x00, 0x00), // ' *' 00
    (0x12, 0x12), // '.*' 01
    (0x12, 0x12), // '?*' 02
    (0x12, 0x12), // ',*' 03
    (0x08, 0x08), // '-*' 04
    (0x08, 0x0B), // 'IY' 05
    (0x08, 0x09), // 'IH' 06
    (0x08, 0x0B), // 'EH' 07
    (0x08, 0x0E), // 'AE' 08
    (0x0B, 0x0F), // 'AA' 09
    (0x06, 0x0B), // 'AH' 10
    (0x0C, 0x10), // 'AO' 11
    (0x0A, 0x0C), // 'UH' 12
    (0x05, 0x06), // 'AX' 13
    (0x05, 0x06), // 'IX' 14
    (0x0B, 0x0E), // 'ER' 15
    (0x0A, 0x0C), // 'UX' 16
    (0x0A, 0x0E), // 'OH' 17
    (0x0A, 0x0C), // 'RX' 18
    (0x09, 0x0B), // 'LX' 19
    (0x08, 0x08), // 'WX' 20
    (0x07, 0x08), // 'YX' 21
    (0x09, 0x0B), // 'WH' 22
    (0x07, 0x0A), // 'R*' 23
    (0x06, 0x09), // 'L*' 24
    (0x08, 0x08), // 'W*' 25
    (0x06, 0x08), // 'Y*' 26
    (0x07, 0x08), // 'M*' 27
    (0x07, 0x08), // 'N*' 28
    (0x07, 0x08), // 'NX' 29
    (0x02, 0x03), // 'DX' 30
    (0x05, 0x05), // 'Q*' 31
    (0x02, 0x02), // 'S*' 32
    (0x02, 0x02), // 'SH' 33
    (0x02, 0x02), // 'F*' 34
    (0x02, 0x02), // 'TH' 35
    (0x02, 0x02), // '/H' 36
    (0x02, 0x02), // '/X' 37
    (0x06, 0x06), // 'Z*' 38
    (0x06, 0x06), // 'ZH' 39
    (0x07, 0x08), // 'V*' 40
    (0x06, 0x06), // 'DH' 41
    (0x06, 0x06), // 'CH' 42
    (0x02, 0x02), // '**' 43
    (0x08, 0x09), // 'J*' 44
    (0x03, 0x04), // '**' 45
    (0x01, 0x02), // '**' 46
    (0x1E, 0x01), // '**' 47
    (0x0D, 0x0E), // 'EY' 48
    (0x0C, 0x0F), // 'AY' 49
    (0x0C, 0x0F), // 'OY' 50
    (0x0C, 0x0F), // 'AW' 51
    (0x0E, 0x0E), // 'OW' 52
    (0x09, 0x0E), // 'UW' 53
    (0x06, 0x08), // 'B*' 54
    (0x01, 0x02), // '**' 55
    (0x02, 0x02), // '**' 56
    (0x05, 0x07), // 'D*' 57
    (0x01, 0x02), // '**' 58
    (0x01, 0x01), // '**' 59
    (0x06, 0x07), // 'G*' 60
    (0x01, 0x02), // '**' 61
    (0x02, 0x02), // '**' 62
    (0x06, 0x07), // 'GX' 63
    (0x01, 0x02), // '**' 64
    (0x02, 0x02), // '**' 65
    (0x08, 0x08), // 'P*' 66
    (0x02, 0x02), // '**' 67
    (0x02, 0x02), // '**' 68
    (0x04, 0x06), // 'T*' 69
    (0x02, 0x02), // '**' 70
    (0x02, 0x02), // '**' 71
    (0x06, 0x07), // 'K*' 72
    (0x01, 0x02), // '**' 73
    (0x04, 0x04), // '**' 74
    (0x06, 0x07), // 'KX' 75
    (0x01, 0x01), // '**' 76
    (0x04, 0x04), // '**' 77
    (0xC7, 0x05), // 'UL' 78
    (0xFF, 0x05), // 'UM' 79
                  // FIXME: Phoneme 80 (UN) is missing
];

mod flag {
    // Unused constants
    pub const _OX8000: u16 = 0x8000; // Unknown: ' *', '.*', '?*', ',*', '-*'
    pub const _OX4000: u16 = 0x4000; // Unknown: '.*', '?*', ',*', '-*', 'Q*'

    // Consonant articulations
    pub const FRICATIVE: u16 = 0x2000;
    pub const LIQUID: u16 = 0x1000;
    pub const NASAL: u16 = 0x0800;
    pub const ALVEOLAR: u16 = 0x0400;

    // 0x0200 is unused
    pub const _OX0200: u16 = 0x0200;

    pub const PUNCTUATION: u16 = 0x0100;
    pub const VOWEL: u16 = 0x0080;
    pub const CONSONANT: u16 = 0x0040; // Note that UM and UN are marked as both vowels and consonants

    pub const DIPHTHONG_YX: u16 = 0x0020; // Diphthong ending with YX, front vowels?
    pub const DIPHTHONG: u16 = 0x0010;

    // Unknown:
    // 'M*', 'N*', 'NX', 'DX', 'Q*', 'CH', 'J*', 'B*', '**', '**', 'D*',
    // '**', '**', 'G*', '**', '**', 'GX', '**', '**', 'P*', '**', '**',
    // 'T*', '**', '**', 'K*', '**', '**', 'KX', '**', '**'
    pub const OX0008: u16 = 0x0008;

    pub const VOICED: u16 = 0x0004; // Applied to vowels and consonants

    // Plosives
    pub const PLOSIVE: u16 = 0x0002; // Both voiced and unvoiced
    pub const UNVOICED_PLOSIVE: u16 = 0x0001;
}

const PHONEME_FLAGS: &[u16] = &[
    0x8000, // ' *' 00
    0xc100, // '.*' 01
    0xc100, // '?*' 02
    0xc100, // ',*' 03
    0xc100, // '-*' 04
    0x00a4, // 'IY' 05
    0x00a4, // 'IH' 06
    0x00a4, // 'EH' 07
    0x00a4, // 'AE' 08
    0x00a4, // 'AA' 09
    0x00a4, // 'AH' 10
    0x0084, // 'AO' 11
    0x0084, // 'UH' 12
    0x00a4, // 'AX' 13
    0x00a4, // 'IX' 14
    0x0084, // 'ER' 15
    0x0084, // 'UX' 16
    0x0084, // 'OH' 17
    0x0084, // 'RX' 18
    0x0084, // 'LX' 19
    0x0084, // 'WX' 20
    0x0084, // 'YX' 21
    0x0044, // 'WH' 22
    0x1044, // 'R*' 23
    0x1044, // 'L*' 24
    0x1044, // 'W*' 25
    0x1044, // 'Y*' 26
    0x084c, // 'M*' 27
    0x0c4c, // 'N*' 28
    0x084c, // 'NX' 29
    0x0448, // 'DX' 30
    0x404c, // 'Q*' 31
    0x2440, // 'S*' 32
    0x2040, // 'SH' 33
    0x2040, // 'F*' 34
    0x2440, // 'TH' 35
    0x0040, // '/H' 36
    0x0040, // '/X' 37
    0x2444, // 'Z*' 38
    0x2044, // 'ZH' 39
    0x2044, // 'V*' 40
    0x2444, // 'DH' 41
    0x2048, // 'CH' 42
    0x2040, // '**' 43
    0x004c, // 'J*' 44
    0x2044, // '**' 45
    0x0000, // '**' 46
    0x0000, // '**' 47
    0x00b4, // 'EY' 48
    0x00b4, // 'AY' 49
    0x00b4, // 'OY' 50
    0x0094, // 'AW' 51
    0x0094, // 'OW' 52
    0x0094, // 'UW' 53
    0x004e, // 'B*' 54
    0x004e, // '**' 55
    0x004e, // '**' 56
    0x044e, // 'D*' 57
    0x044e, // '**' 58
    0x044e, // '**' 59
    0x004e, // 'G*' 60
    0x004e, // '**' 61
    0x004e, // '**' 62
    0x004e, // 'GX' 63
    0x004e, // '**' 64
    0x004e, // '**' 65
    0x004b, // 'P*' 66
    0x004b, // '**' 67
    0x004b, // '**' 68
    0x044b, // 'T*' 69
    0x044b, // '**' 70
    0x044b, // '**' 71
    0x004b, // 'K*' 72
    0x004b, // '**' 73
    0x004b, // '**' 74
    0x004b, // 'KX' 75
    0x004b, // '**' 76
    0x004b, // '**' 77
    0x0080, // 'UL' 78
    0x00c1, // 'UM' 79
    0x00c1, // 'UN' 80
];

/// Match both characters, but not with wildcards.
fn full_match(sign1: char, sign2: char) -> Option<usize> {
    // TODO: Investigate if sign2 is ever an asterisk
    PHONEME_NAME_TABLE
        .iter()
        .position(|(first, second)| *second != '*' && *first == sign1 && *second == sign2)
}

/// Match character plus a wildcard.
fn wildcard_match(sign1: char) -> Option<usize> {
    PHONEME_NAME_TABLE
        .iter()
        .position(|(first, second)| *first == sign1 && *second == '*')
}

// TODO: Emit Result instead of panicking
fn parser1(text: &str) -> ParseResult {
    let mut result = ParseResult::new();
    let mut iter = text.chars().peekable();

    while let Some(sign1) = iter.next() {
        if let Some(sign2) = iter.peek() {
            if let Some(phoneme_index) = full_match(sign1, *sign2) {
                // Matched both characters (no wildcards)

                // Skip the second character of the input as we've matched it
                iter.next();

                // add_phoneme
                result.phonemes.push(Phoneme {
                    index: phoneme_index,
                    length: 0,
                    stress: 0,
                });

                continue;
            }
        }

        if let Some(phoneme_index) = wildcard_match(sign1) {
            // Matched just the first character (with second character matching '*'

            // add_phoneme
            result.phonemes.push(Phoneme {
                index: phoneme_index,
                length: 0,
                stress: 0,
            });

            continue;
        }

        // Note: the first index ("*") is not matched in the original implementation. The original
        // implementation searches backwards, but this does not make sense on a modern CPU.
        // TODO: Can be replaced with ascii math instead of iteration?
        if let Some(index) = STRESS_TABLE[1..]
            .iter()
            .position(|candidate| *candidate == sign1)
        {
            // add_stress
            // Compensate for the skipped "*" in the iterator index
            let index = index + 1;

            // FIXME: This can never happen here?
            //if index & 128 != 0 {
            //throw new Error('Got the flag 0x80, see CopyStress() and SetPhonemeLength() comments!');
            //}

            // Set stress for prior phoneme
            result
                .phonemes
                .last_mut()
                .expect("Tried adding stress without adding a phoneme first")
                .stress = index as u8;
        } else {
            panic!("Could not parse character {:?}", sign1);
        }
    }

    result
}

pub const PHONEME_PAUSE: usize = 0;
pub const PHONEME_PERIOD: usize = 1;
pub const PHONEME_QUESTION_MARK: usize = 2;
pub const PHONEME_AX: usize = 13;
pub const PHONEME_UX: usize = 16;
pub const PHONEME_RX: usize = 18;
pub const PHONEME_LX: usize = 19;
pub const PHONEME_WX: usize = 20;
pub const PHONEME_YX: usize = 21;
pub const PHONEME_R_STAR: usize = 23;
pub const PHONEME_L_STAR: usize = 24;
pub const PHONEME_M_STAR: usize = 27;
pub const PHONEME_N_STAR: usize = 28;
pub const PHONEME_DX: usize = 30;
pub const PHONEME_Q_STAR: usize = 31;
pub const PHONEME_S_STAR: usize = 32;
pub const PHONEME_SLASH_H: usize = 36;
pub const PHONEME_SLASH_X: usize = 37;
pub const PHONEME_Z_STAR: usize = 38;
pub const PHONEME_CH: usize = 42;
pub const PHONEME_STAR_STAR_43: usize = 43;
pub const PHONEME_J_STAR: usize = 44;
pub const PHONEME_STAR_STAR_45: usize = 45;
pub const PHONEME_UW: usize = 53;
pub const PHONEME_D_STAR: usize = 57;
pub const PHONEME_G_STAR: usize = 60;
pub const PHONEME_GX: usize = 63;
pub const PHONEME_T_STAR: usize = 69;
pub const PHONEME_K_STAR: usize = 72;
pub const PHONEME_KX: usize = 75;
pub const PHONEME_UL: usize = 78;
pub const PHONEME_UM: usize = 79;
pub const PHONEME_UN: usize = 80;

fn handle_uw_ch_j(phonemes: &mut Vec<Phoneme>, position: usize) {
    let phoneme = &phonemes[position];

    match phoneme.index {
        // 'UW' Example: NEW, DEW, SUE, ZOO, THOO, TOO
        // Check for UW with alveolar flag set on previous phoneme
        PHONEME_UW
            if position
                .checked_sub(1)
                .and_then(|prev| phonemes.get(prev))
                .map_or(false, |prev| prev.has_flag(flag::ALVEOLAR)) =>
        {
            phonemes[position].index = PHONEME_UX;
        }

        // 'CH' Example: CHEW
        PHONEME_CH => {
            phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: PHONEME_STAR_STAR_43,
                    stress: phoneme.stress,
                },
            );
        }

        // 'J*' Example: JAY
        PHONEME_J_STAR => {
            phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: PHONEME_STAR_STAR_45,
                    stress: phoneme.stress,
                },
            );
        }

        _ => (),
    }
}

fn parser2(result: &mut ParseResult) -> Result<(), ParseError> {
    let mut position: isize = -1;

    loop {
        position += 1;
        let position = position as usize;

        if position >= result.phonemes.len() {
            break;
        }

        // Is phoneme pause?
        if result.phonemes[position].index == PHONEME_PAUSE {
            continue;
        }

        if result.phonemes[position].has_flag(flag::DIPHTHONG) {
            // <DIPHTHONG ENDING WITH WX> -> <DIPHTHONG ENDING WITH WX> WX
            // <DIPHTHONG NOT ENDING WITH WX> -> <DIPHTHONG NOT ENDING WITH WX> YX
            // Example: OIL, COW
            // If ends with IY, use YX, else use WX
            // Insert at WX or YX following, copying the stress
            // 'WX' = 20 'YX' = 21
            result.phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: if result.phonemes[position].has_flag(flag::DIPHTHONG_YX) {
                        PHONEME_YX
                    } else {
                        PHONEME_WX
                    },
                    stress: result.phonemes[position].stress,
                },
            );

            handle_uw_ch_j(&mut result.phonemes, position);
            continue;
        }

        if result.phonemes[position].index == PHONEME_UL {
            // 'UL' => 'AX' 'L*'
            // Example: MEDDLE
            result.phonemes[position].index = PHONEME_AX;
            result.phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: PHONEME_L_STAR,
                    stress: result.phonemes[position].stress,
                },
            );

            continue;
        }

        if result.phonemes[position].index == PHONEME_UM {
            // 'UM' => 'AX' 'M*'
            // Example: ASTRONOMY
            result.phonemes[position].index = PHONEME_AX;
            result.phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: PHONEME_M_STAR,
                    stress: result.phonemes[position].stress,
                },
            );

            continue;
        }

        if result.phonemes[position].index == PHONEME_UN {
            // 'UN' => 'AX' 'N*'
            result.phonemes[position].index = PHONEME_AX;
            result.phonemes.insert(
                position + 1,
                Phoneme {
                    length: 0,
                    index: PHONEME_N_STAR,
                    stress: result.phonemes[position].stress,
                },
            );

            continue;
        }

        if result.phonemes[position].has_flag(flag::VOWEL) && result.phonemes[position].stress != 0
        {
            // Example: FUNCTION
            // RULE:
            //       <STRESSED VOWEL> <SILENCE> <STRESSED VOWEL> -> <STRESSED VOWEL> <SILENCE> Q <VOWEL>
            // EXAMPLE: AWAY EIGHT
            if result
                .phonemes
                .get(position + 1)
                .map_or(false, |phoneme| phoneme.index == PHONEME_PAUSE)
            {
                // If following phoneme is a pause, get next
                if let Some(phoneme) = result.phonemes.get(position + 2) {
                    if phoneme.has_flag(flag::VOWEL) && phoneme.stress != 0 {
                        // Insert glottal stop between two stressed vowels with space between them
                        result.phonemes.insert(
                            position + 2,
                            Phoneme {
                                length: 0,
                                index: PHONEME_Q_STAR,
                                stress: 0,
                            },
                        );
                    }
                }
            }

            continue;
        }

        ////let priorPhoneme = (pos === 0) ? null : getPhoneme(pos - 1);
        let prior_phoneme = if position > 0 {
            result.phonemes.get(position - 1)
        } else {
            None
        };

        if result.phonemes[position].index == PHONEME_R_STAR {
            if let Some(prior_phoneme) = prior_phoneme {
                // position - 1 is guaranteed to be valid inside this block
                // Rules for phonemes before R
                match prior_phoneme.index {
                    // Example: TRACK
                    // T* R* -> CH R*
                    PHONEME_T_STAR => {
                        result.phonemes[position - 1].index = PHONEME_CH;
                    }

                    // Example: DRY
                    // D* R* -> J* R*
                    PHONEME_D_STAR => {
                        result.phonemes[position - 1].index = PHONEME_J_STAR;
                    }

                    // Example: ART
                    // <VOWEL> R* -> <VOWEL> RX
                    _ => {
                        if prior_phoneme.has_flag(flag::VOWEL) {
                            result.phonemes[position].index = PHONEME_RX;
                        }
                    }
                }
            }

            continue;
        }

        // 'L*'
        if result.phonemes[position].index == PHONEME_L_STAR
            && prior_phoneme.map_or(false, |phoneme| phoneme.has_flag(flag::VOWEL))
        {
            // Example: ALL
            // <VOWEL> L* -> <VOWEL> LX
            result.phonemes[position].index = PHONEME_LX;
            continue;
        }

        // 'G*' 'S*'
        if result.phonemes[position].index == PHONEME_S_STAR
            && prior_phoneme.map_or(false, |phoneme| phoneme.index == PHONEME_G_STAR)
        {
            // G S -> G Z
            // Can't get to fire -
            //       1. The G -> GX rule intervenes
            //       2. Reciter already replaces GS -> GZ
            result.phonemes[position].index = PHONEME_Z_STAR;
            continue;
        }

        // 'G*'
        if result.phonemes[position].index == PHONEME_G_STAR {
            // G <VOWEL OR DIPHTHONG NOT ENDING WITH IY> -> GX <VOWEL OR DIPHTHONG NOT ENDING WITH IY>
            // Example: GO
            if let Some(phoneme) = result.phonemes.get(position + 1) {
                // If diphthong ending with YX, move continue processing next phoneme
                if !phoneme.has_flag(flag::DIPHTHONG_YX) {
                    // replace G with GX and continue processing next phoneme
                    // G <VOWEL OR DIPHTHONG NOT ENDING WITH IY> -> GX <VOWEL OR DIPHTHONG NOT ENDING WITH IY>
                    result.phonemes[position].index = PHONEME_GX;
                }
            }

            continue;
        }

        // 'K*'
        if result.phonemes[position].index == PHONEME_K_STAR {
            // K <VOWEL OR DIPHTHONG NOT ENDING WITH IY> -> KX <VOWEL OR DIPHTHONG NOT ENDING WITH IY>
            // Example: COW
            // If at end, replace current phoneme with KX
            // Note: also applies when next phoneme is not DIPHTHONG_YX
            if result
                .phonemes
                .get(position + 1)
                .map_or(true, |phoneme| !phoneme.has_flag(flag::DIPHTHONG_YX))
            {
                // VOWELS AND DIPHTHONGS ENDING WITH IY SOUND flag set?
                result.phonemes[position].index = PHONEME_KX;

                // TODO: Figure out what the impact of this change is and if it can ever match
                // any rules below
                // TODO: Can be removed safely after switching to using array indices?
                //phoneme = PHONEME_KX;
            }
        }

        // Replace with softer version?
        if result.phonemes[position].has_flag(flag::UNVOICED_PLOSIVE)
            && position
                .checked_sub(1)
                .and_then(|prev| result.phonemes.get(prev))
                .map_or(false, |phoneme| phoneme.index == PHONEME_S_STAR)
        {
            // 'S*'
            // RULE:
            //   'S*' 'P*' -> 'S*' 'B*'
            //   'S*' 'T*' -> 'S*' 'D*'
            //   'S*' 'K*' -> 'S*' 'G*'
            //   'S*' 'KX' -> 'S*' 'GX'
            //   'S*' 'UM' -> 'S*' '**'
            //   'S*' 'UN' -> 'S*' '**'
            // Examples: SPY, STY, SKY, SCOWL
            result.phonemes[position].index -= 12;
        } else if !result.phonemes[position].has_flag(flag::UNVOICED_PLOSIVE) {
            handle_uw_ch_j(&mut result.phonemes, position);
        }

        // 'T*', 'D*'
        if result.phonemes[position].index == PHONEME_T_STAR
            || result.phonemes[position].index == PHONEME_D_STAR
        {
            // RULE: Soften T following vowel
            // NOTE: This rule fails for cases such as "ODD"
            //       <UNSTRESSED VOWEL> T <PAUSE> -> <UNSTRESSED VOWEL> DX <PAUSE>
            //       <UNSTRESSED VOWEL> D <PAUSE>  -> <UNSTRESSED VOWEL> DX <PAUSE>
            // Example: PARTY, TARDY
            if let Some(prior_phoneme) = position
                .checked_sub(1)
                .and_then(|prev| result.phonemes.get(prev))
            {
                if prior_phoneme.has_flag(flag::VOWEL) {
                    let mut phoneme = result.phonemes.get(position + 1);
                    let next_phoneme = phoneme;

                    if next_phoneme.is_some() && next_phoneme.unwrap().index == PHONEME_PAUSE {
                        phoneme = result.phonemes.get(position + 2);
                    }

                    if let Some(phoneme) = phoneme {
                        if phoneme.has_flag(flag::VOWEL)
                            && next_phoneme.map_or(false, |phoneme| phoneme.stress == 0)
                        {
                            // Soften T or D following vowel or ER and preceding a pause -> DX
                            result.phonemes[position].index = PHONEME_DX;
                        }
                    }
                }
            }

            continue;
        }
    }

    Ok(())
}

fn copy_stress(phonemes: &mut [Phoneme]) {
    let mut iter = phonemes.iter_mut().peekable();

    while let Some(phoneme) = iter.next() {
        // if CONSONANT_FLAG set, skip - only vowels get stress
        if !phoneme.has_flag(flag::CONSONANT) {
            continue;
        }

        if let Some(next_phoneme) = iter.peek() {
            // if the following phoneme is the end, or a vowel, skip
            if next_phoneme.has_flag(flag::VOWEL) {
                // get the stress value at the next position
                let stress = next_phoneme.stress;

                // TODO: Why the check for <0x80? Value never seems to be set that high
                if stress != 0 && stress < 0x80 {
                    // if next phoneme is stressed, and a VOWEL OR ER
                    // copy stress from next phoneme to this one
                    phoneme.stress = stress + 1;
                }
            }
        }
    }
}

fn set_phoneme_length(phonemes: &mut [Phoneme]) {
    for phoneme in phonemes.iter_mut() {
        let stress = phoneme.stress;

        if stress == 0 || stress > 0x7F {
            phoneme.length = PHONEME_LENGTH_TABLE[phoneme.index].0;
        } else {
            phoneme.length = PHONEME_LENGTH_TABLE[phoneme.index].1;
        }
    }
}

fn adjust_lengths(phonemes: &mut Vec<Phoneme>) {
    // LENGTHEN VOWELS PRECEDING PUNCTUATION
    //
    // Search for punctuation. If found, back up to the first vowel, then
    // process all phonemes between there and up to (but not including) the punctuation.
    // If any phoneme is found that is a either a fricative or voiced, the duration is
    // increased by (length * 1.5) + 1

    for position in 0..phonemes.len() {
        // not punctuation?
        if !phonemes[position].has_flag(flag::PUNCTUATION) {
            continue;
        }

        // Back up while not a vowel
        let mut vowel_position = position;
        while vowel_position > 0 && !phonemes[vowel_position - 1].has_flag(flag::VOWEL) {
            vowel_position -= 1;
        }

        // Vowel position now points to the last non-vowel. Decrement again to make it point to the
        // vowel itself.
        vowel_position = vowel_position.saturating_sub(1);

        // Now handle everything between the vowel up to the punctuation
        for phoneme in &mut phonemes[vowel_position..position] {
            // test for not fricative/unvoiced or not voiced
            if !phoneme.has_flag(flag::FRICATIVE) || phoneme.has_flag(flag::VOICED) {
                // change phoneme length to (length * 1.5) + 1
                phoneme.length += (phoneme.length >> 1) + 1;
            }
        }
    }

    // Similar to the above routine, but shorten vowels under some circumstances
    // Loop through all phonemes
    for loop_position in 0..phonemes.len() {
        let mut position = loop_position;

        // vowel?
        if phonemes[loop_position].has_flag(flag::VOWEL) {
            // get next phoneme
            position += 1;

            // The reference implementation does not check for bounds here, causing the phoneme to
            // be null. This will fail all has_flag checks, effectively marking the position as a
            // vowel and doing nothing (because the phoneme index checks fail).
            if position >= phonemes.len() {
                continue;
            }

            let vowel_phoneme_position = Some(position);

            // not a consonant
            if !phonemes[position].has_flag(flag::CONSONANT) {
                // 'RX' or 'LX'?
                if phonemes[position].index == PHONEME_RX || phonemes[position].index == PHONEME_LX
                {
                    position += 1;

                    if phonemes
                        .get(position)
                        .map(|phoneme| phoneme.has_flag(flag::CONSONANT))
                        .unwrap_or(false)
                    {
                        // followed by consonant?

                        // decrease length of vowel by 1 frame
                        phonemes[loop_position].length -= 1;
                    }
                }

                continue;
            }

            // Got here if not <VOWEL>
            // FIXME: the case when phoneme === END is taken over by !phonemeHasFlag(phoneme, FLAG_CONSONANT)
            let flags = vowel_phoneme_position
                .map_or(flag::CONSONANT | flag::UNVOICED_PLOSIVE, |position| {
                    PHONEME_FLAGS[phonemes[position].index]
                });

            // Unvoiced
            if flags & flag::VOICED == 0 {
                // *, .*, ?*, ,*, -*, DX, S*, SH, F*, TH, /H, /X, CH, P*, T*, K*, KX

                // unvoiced plosive
                if flags & flag::UNVOICED_PLOSIVE != 0 {
                    // RULE: <VOWEL> <UNVOICED PLOSIVE>
                    // <VOWEL> <P*, T*, K*, KX>
                    phonemes[loop_position].length -= phonemes[loop_position].length >> 3;
                }

                continue;
            }

            // RULE: <VOWEL> <VOWEL or VOICED CONSONANT>
            // <VOWEL> <IY, IH, EH, AE, AA, AH, AO, UH, AX, IX, ER, UX, OH, RX, LX, WX, YX, WH, R*, L*, W*,
            //          Y*, M*, N*, NX, Q*, Z*, ZH, V*, DH, J*, EY, AY, OY, AW, OW, UW, B*, D*, G*, GX>
            // increase length
            phonemes[loop_position].length += (phonemes[loop_position].length >> 2) + 1;

            continue;
        }

        //  *, .*, ?*, ,*, -*, WH, R*, L*, W*, Y*, M*, N*, NX, DX, Q*, S*, SH, F*,
        // TH, /H, /X, Z*, ZH, V*, DH, CH, J*, B*, D*, G*, GX, P*, T*, K*, KX

        // nasal?
        if phonemes[loop_position].has_flag(flag::NASAL) {
            // RULE: <NASAL> <STOP CONSONANT>
            //       Set punctuation length to 6
            //       Set stop consonant length to 5
            // M*, N*, NX,

            // is next phoneme a stop consonant?
            if let Some(phoneme) = phonemes.get_mut(loop_position + 1) {
                if phoneme.has_flag(flag::PLOSIVE) {
                    // B*, D*, G*, GX, P*, T*, K*, KX
                    phoneme.length = 6;
                    phonemes[loop_position].length = 5;
                }
            }

            continue;
        }

        //  *, .*, ?*, ,*, -*, WH, R*, L*, W*, Y*, DX, Q*, S*, SH, F*, TH,
        // /H, /X, Z*, ZH, V*, DH, CH, J*, B*, D*, G*, GX, P*, T*, K*, KX

        // stop consonant?
        if phonemes[loop_position].has_flag(flag::PLOSIVE) {
            // B*, D*, G*, GX

            // RULE: <STOP CONSONANT> {optional silence} <STOP CONSONANT>
            //       Shorten both to (length/2 + 1)

            // Move past silence
            let mut position = loop_position + 1;
            while position < phonemes.len() && phonemes[position].index == PHONEME_PAUSE {
                position += 1;
            }

            // if another stop consonant, process.
            if let Some(phoneme) = phonemes.get_mut(position) {
                if phoneme.has_flag(flag::PLOSIVE) {
                    // RULE: <STOP CONSONANT> {optional silence} <STOP CONSONANT>
                    phoneme.length = (phoneme.length >> 1) + 1;
                    phonemes[loop_position].length = (phonemes[loop_position].length >> 1) + 1;
                }
            }

            continue;
        }

        //  *, .*, ?*, ,*, -*, WH, R*, L*, W*, Y*, DX, Q*, S*, SH, F*, TH,
        // /H, /X, Z*, ZH, V*, DH, CH, J*

        // liquid consonant following a plosive
        if loop_position > 0
            && phonemes[loop_position].has_flag(flag::LIQUID)
            && phonemes[loop_position - 1].has_flag(flag::PLOSIVE)
        {
            // R*, L*, W*, Y*
            // RULE: <STOP CONSONANT> <LIQUID>
            //       Decrease <LIQUID> by 2
            // prior phoneme is a stop consonant
            // decrease the phoneme length by 2 frames
            phonemes[loop_position].length -= 2;
        }
    }
}

fn prolong_plosives(phonemes: &mut Vec<Phoneme>) {
    let mut position = 0;

    while position < phonemes.len() {
        // Not a stop consonant, move to next one.
        if !phonemes[position].has_flag(flag::PLOSIVE) {
            position += 1;
            continue;
        }

        // If plosive, move to next non-empty phoneme and validate the flags.
        if phonemes[position].has_flag(flag::UNVOICED_PLOSIVE) {
            let mut next_non_empty = position + 1;
            while phonemes
                .get(next_non_empty)
                .map_or(false, |phoneme| phoneme.index == PHONEME_PAUSE)
            {
                next_non_empty += 1;
            }

            // If not END and either flag 0x0008 or '/H' or '/X'
            if let Some(phoneme) = phonemes.get(next_non_empty) {
                if phoneme.has_flag(flag::OX0008)
                    || phoneme.index == PHONEME_SLASH_H
                    || phoneme.index == PHONEME_SLASH_X
                {
                    position += 1;
                    continue;
                }
            }
        }

        phonemes.insert(
            position + 1,
            Phoneme {
                index: phonemes[position].index + 1,
                stress: phonemes[position].stress,
                length: PHONEME_LENGTH_TABLE[phonemes[position].index + 1].0,
            },
        );

        phonemes.insert(
            position + 2,
            Phoneme {
                index: phonemes[position].index + 2,
                stress: phonemes[position].stress,
                length: PHONEME_LENGTH_TABLE[phonemes[position].index + 2].0,
            },
        );

        position += 3;
    }
}

pub fn parse_phonemes(text: &str) -> Result<Vec<Phoneme>, ParseError> {
    // TODO: Find a better name for this

    // Parser1
    let mut result = parser1(text);

    // Parser2
    parser2(&mut result)?;

    // CopyStress
    copy_stress(&mut result.phonemes);

    // SetPhonemeLength
    set_phoneme_length(&mut result.phonemes);

    // AdjustLengths
    adjust_lengths(&mut result.phonemes);

    // ProlongPlosiveStopConsonantsCode41240
    prolong_plosives(&mut result.phonemes);

    // Filter pauses
    result
        .phonemes
        .retain(|phoneme| phoneme.index != PHONEME_PAUSE);

    Ok(result.phonemes)
}

// Frequency data for each of the three formant waveforms
const FREQUENCY_DATA: (&[u8], &[u8], &[u8]) = (
    &[
        0x00, // ' *' 00
        0x13, // '.*' 01
        0x13, // '?*' 02
        0x13, // ',*' 03
        0x13, // '-*' 04
        0x0A, // 'IY' 05
        0x0E, // 'IH' 06
        0x13, // 'EH' 07
        0x18, // 'AE' 08
        0x1B, // 'AA' 09
        0x17, // 'AH' 10
        0x15, // 'AO' 11
        0x10, // 'UH' 12
        0x14, // 'AX' 13
        0x0E, // 'IX' 14
        0x12, // 'ER' 15
        0x0E, // 'UX' 16
        0x12, // 'OH' 17
        0x12, // 'RX' 18
        0x10, // 'LX' 19
        0x0D, // 'WX' 20
        0x0F, // 'YX' 21
        0x0B, // 'WH' 22
        0x12, // 'R*' 23
        0x0E, // 'L*' 24
        0x0B, // 'W*' 25
        0x09, // 'Y*' 26
        0x06, // 'M*' 27
        0x06, // 'N*' 28
        0x06, // 'NX' 29
        0x06, // 'DX' 30
        0x11, // 'Q*' 31
        0x06, // 'S*' 32
        0x06, // 'SH' 33
        0x06, // 'F*' 34
        0x06, // 'TH' 35
        0x0E, // '/H' 36
        0x10, // '/X' 37
        0x09, // 'Z*' 38
        0x0A, // 'ZH' 39
        0x08, // 'V*' 40
        0x0A, // 'DH' 41
        0x06, // 'CH' 42
        0x06, // '**' 43
        0x06, // 'J*' 44
        0x05, // '**' 45
        0x06, // '**' 46
        0x00, // '**' 47
        0x13, // 'EY' 48
        0x1B, // 'AY' 49
        0x15, // 'OY' 50
        0x1B, // 'AW' 51
        0x12, // 'OW' 52
        0x0D, // 'UW' 53
        0x06, // 'B*' 54
        0x06, // '**' 55
        0x06, // '**' 56
        0x06, // 'D*' 57
        0x06, // '**' 58
        0x06, // '**' 59
        0x06, // 'G*' 60
        0x06, // '**' 61
        0x06, // '**' 62
        0x06, // 'GX' 63
        0x06, // '**' 64
        0x06, // '**' 65
        0x06, // 'P*' 66
        0x06, // '**' 67
        0x06, // '**' 68
        0x06, // 'T*' 69
        0x06, // '**' 70
        0x06, // '**' 71
        0x06, // 'K*' 72
        0x0A, // '**' 73
        0x0A, // '**' 74
        0x06, // 'KX' 75
        0x06, // '**' 76
        0x06, // '**' 77
        0x2C, // 'UL' 78
        0x13, // 'UM' 79
    ],
    &[
        0x00, // ' *' 00
        0x43, // '.*' 01
        0x43, // '?*' 02
        0x43, // ',*' 03
        0x43, // '-*' 04
        0x54, // 'IY' 05
        0x49, // 'IH' 06
        0x43, // 'EH' 07
        0x3F, // 'AE' 08
        0x28, // 'AA' 09
        0x2C, // 'AH' 10
        0x1F, // 'AO' 11
        0x25, // 'UH' 12
        0x2D, // 'AX' 13
        0x49, // 'IX' 14
        0x31, // 'ER' 15
        0x24, // 'UX' 16
        0x1E, // 'OH' 17
        0x33, // 'RX' 18
        0x25, // 'LX' 19
        0x1D, // 'WX' 20
        0x45, // 'YX' 21
        0x18, // 'WH' 22
        0x32, // 'R*' 23
        0x1E, // 'L*' 24
        0x18, // 'W*' 25
        0x53, // 'Y*' 26
        0x2E, // 'M*' 27
        0x36, // 'N*' 28
        0x56, // 'NX' 29
        0x36, // 'DX' 30
        0x43, // 'Q*' 31
        0x49, // 'S*' 32
        0x4F, // 'SH' 33
        0x1A, // 'F*' 34
        0x42, // 'TH' 35
        0x49, // '/H' 36
        0x25, // '/X' 37
        0x33, // 'Z*' 38
        0x42, // 'ZH' 39
        0x28, // 'V*' 40
        0x2F, // 'DH' 41
        0x4F, // 'CH' 42
        0x4F, // '**' 43
        0x42, // 'J*' 44
        0x4F, // '**' 45
        0x6E, // '**' 46
        0x00, // '**' 47
        0x48, // 'EY' 48
        0x27, // 'AY' 49
        0x1F, // 'OY' 50
        0x2B, // 'AW' 51
        0x1E, // 'OW' 52
        0x22, // 'UW' 53
        0x1A, // 'B*' 54
        0x1A, // '**' 55
        0x1A, // '**' 56
        0x42, // 'D*' 57
        0x42, // '**' 58
        0x42, // '**' 59
        0x6E, // 'G*' 60
        0x6E, // '**' 61
        0x6E, // '**' 62
        0x54, // 'GX' 63
        0x54, // '**' 64
        0x54, // '**' 65
        0x1A, // 'P*' 66
        0x1A, // '**' 67
        0x1A, // '**' 68
        0x42, // 'T*' 69
        0x42, // '**' 70
        0x42, // '**' 71
        0x6D, // 'K*' 72
        0x56, // '**' 73
        0x6D, // '**' 74
        0x54, // 'KX' 75
        0x54, // '**' 76
        0x54, // '**' 77
        0x7F, // 'UL' 78
        0x7F, // 'UM' 79
    ],
    &[
        0x00, // ' *' 00
        0x5B, // '.*' 01
        0x5B, // '?*' 02
        0x5B, // ',*' 03
        0x5B, // '-*' 04
        0x6E, // 'IY' 05
        0x5D, // 'IH' 06
        0x5B, // 'EH' 07
        0x58, // 'AE' 08
        0x59, // 'AA' 09
        0x57, // 'AH' 10
        0x58, // 'AO' 11
        0x52, // 'UH' 12
        0x59, // 'AX' 13
        0x5D, // 'IX' 14
        0x3E, // 'ER' 15
        0x52, // 'UX' 16
        0x58, // 'OH' 17
        0x3E, // 'RX' 18
        0x6E, // 'LX' 19
        0x50, // 'WX' 20
        0x5D, // 'YX' 21
        0x5A, // 'WH' 22
        0x3C, // 'R*' 23
        0x6E, // 'L*' 24
        0x5A, // 'W*' 25
        0x6E, // 'Y*' 26
        0x51, // 'M*' 27
        0x79, // 'N*' 28
        0x65, // 'NX' 29
        0x79, // 'DX' 30
        0x5B, // 'Q*' 31
        0x63, // 'S*' 32
        0x6A, // 'SH' 33
        0x51, // 'F*' 34
        0x79, // 'TH' 35
        0x5D, // '/H' 36
        0x52, // '/X' 37
        0x5D, // 'Z*' 38
        0x67, // 'ZH' 39
        0x4C, // 'V*' 40
        0x5D, // 'DH' 41
        0x65, // 'CH' 42
        0x65, // '**' 43
        0x79, // 'J*' 44
        0x65, // '**' 45
        0x79, // '**' 46
        0x00, // '**' 47
        0x5A, // 'EY' 48
        0x58, // 'AY' 49
        0x58, // 'OY' 50
        0x58, // 'AW' 51
        0x58, // 'OW' 52
        0x52, // 'UW' 53
        0x51, // 'B*' 54
        0x51, // '**' 55
        0x51, // '**' 56
        0x79, // 'D*' 57
        0x79, // '**' 58
        0x79, // '**' 59
        0x70, // 'G*' 60
        0x6E, // '**' 61
        0x6E, // '**' 62
        0x5E, // 'GX' 63
        0x5E, // '**' 64
        0x5E, // '**' 65
        0x51, // 'P*' 66
        0x51, // '**' 67
        0x51, // '**' 68
        0x79, // 'T*' 69
        0x79, // '**' 70
        0x79, // '**' 71
        0x65, // 'K*' 72
        0x65, // '**' 73
        0x70, // '**' 74
        0x5E, // 'KX' 75
        0x5E, // '**' 76
        0x5E, // '**' 77
        0x08, // 'UL' 78
        0x01, // 'UM' 79
    ],
);

const AMPLITUDE_DATA: &[(u8, u8, u8)] = &[
    (0x00, 0x00, 0x00), // ' *' 00
    (0x00, 0x00, 0x00), // '.*' 01
    (0x00, 0x00, 0x00), // '?*' 02
    (0x00, 0x00, 0x00), // ',*' 03
    (0x00, 0x00, 0x00), // '-*' 04
    (0x0D, 0x0A, 0x08), // 'IY' 05
    (0x0D, 0x0B, 0x07), // 'IH' 06
    (0x0E, 0x0D, 0x08), // 'EH' 07
    (0x0F, 0x0E, 0x08), // 'AE' 08
    (0x0F, 0x0D, 0x01), // 'AA' 09
    (0x0F, 0x0C, 0x01), // 'AH' 10
    (0x0F, 0x0C, 0x00), // 'AO' 11
    (0x0F, 0x0B, 0x01), // 'UH' 12
    (0x0C, 0x09, 0x00), // 'AX' 13
    (0x0D, 0x0B, 0x07), // 'IX' 14
    (0x0C, 0x0B, 0x05), // 'ER' 15
    (0x0F, 0x0C, 0x01), // 'UX' 16
    (0x0F, 0x0C, 0x00), // 'OH' 17
    (0x0D, 0x0C, 0x06), // 'RX' 18
    (0x0D, 0x08, 0x01), // 'LX' 19
    (0x0D, 0x08, 0x00), // 'WX' 20
    (0x0E, 0x0C, 0x07), // 'YX' 21
    (0x0D, 0x08, 0x00), // 'WH' 22
    (0x0C, 0x0A, 0x05), // 'R*' 23
    (0x0D, 0x08, 0x01), // 'L*' 24
    (0x0D, 0x08, 0x00), // 'W*' 25
    (0x0D, 0x0A, 0x08), // 'Y*' 26
    (0x0C, 0x03, 0x00), // 'M*' 27
    (0x09, 0x09, 0x00), // 'N*' 28
    (0x09, 0x06, 0x03), // 'NX' 29
    (0x00, 0x00, 0x00), // 'DX' 30
    (0x00, 0x00, 0x00), // 'Q*' 31
    (0x00, 0x00, 0x00), // 'S*' 32
    (0x00, 0x00, 0x00), // 'SH' 33
    (0x00, 0x00, 0x00), // 'F*' 34
    (0x00, 0x00, 0x00), // 'TH' 35
    (0x00, 0x00, 0x00), // '/H' 36
    (0x00, 0x00, 0x00), // '/X' 37
    (0x0B, 0x03, 0x00), // 'Z*' 38
    (0x0B, 0x05, 0x01), // 'ZH' 39
    (0x0B, 0x03, 0x00), // 'V*' 40
    (0x0B, 0x04, 0x00), // 'DH' 41
    (0x00, 0x00, 0x00), // 'CH' 42
    (0x00, 0x00, 0x00), // '**' 43
    (0x01, 0x00, 0x00), // 'J*' 44
    (0x0B, 0x05, 0x01), // '**' 45
    (0x00, 0x0A, 0x0E), // '**' 46
    (0x02, 0x02, 0x01), // '**' 47
    (0x0E, 0x0E, 0x09), // 'EY' 48
    (0x0F, 0x0D, 0x01), // 'AY' 49
    (0x0F, 0x0C, 0x00), // 'OY' 50
    (0x0F, 0x0D, 0x01), // 'AW' 51
    (0x0F, 0x0C, 0x00), // 'OW' 52
    (0x0D, 0x08, 0x00), // 'UW' 53
    (0x02, 0x00, 0x00), // 'B*' 54
    (0x04, 0x01, 0x00), // '**' 55
    (0x00, 0x00, 0x00), // '**' 56
    (0x02, 0x00, 0x00), // 'D*' 57
    (0x04, 0x01, 0x00), // '**' 58
    (0x00, 0x00, 0x00), // '**' 59
    (0x01, 0x00, 0x00), // 'G*' 60
    (0x04, 0x01, 0x00), // '**' 61
    (0x00, 0x00, 0x00), // '**' 62
    (0x01, 0x00, 0x00), // 'GX' 63
    (0x04, 0x01, 0x00), // '**' 64
    (0x00, 0x00, 0x00), // '**' 65
    (0x00, 0x00, 0x00), // 'P*' 66
    (0x00, 0x00, 0x00), // '**' 67
    (0x00, 0x00, 0x00), // '**' 68
    (0x00, 0x00, 0x00), // 'T*' 69
    (0x00, 0x00, 0x00), // '**' 70
    (0x00, 0x00, 0x00), // '**' 71
    (0x00, 0x00, 0x00), // 'K*' 72
    (0x0C, 0x0A, 0x07), // '**' 73
    (0x00, 0x00, 0x00), // '**' 74
    (0x00, 0x00, 0x00), // 'KX' 75
    (0x00, 0x0A, 0x05), // '**' 76
    (0x00, 0x00, 0x00), // '**' 77
    (0x0F, 0x00, 0x13), // 'UL' 78
    (0x0F, 0x00, 0x10), // 'UM' 79
];

const SAMPLED_CONSONANT_FLAGS: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xF1, 0xE2, 0xD3, 0xBB, 0x7C, 0x95, 0x01, 0x02, 0x03, 0x03, 0x00, 0x72, 0x00, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x1B, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Eq, PartialEq)]
struct FrequencyData {
    f1: Vec<u8>,
    f2: Vec<u8>,
    f3: Vec<u8>,
}

fn set_mouth_and_throat(mouth: u8, throat: u8) -> FrequencyData {
    // TODO: Convert to constructor?

    fn trans(factor: u8, frequency: u8) -> u8 {
        // Compute (((factor * frequency) / 256) % 256) * 2
        // Note: this assumes all of the frequencies are 7 bit values (to prevent overflowing).
        ((((factor as u16 * frequency as u16) >> 8) & 0xff) << 1) as u8
    }

    let mut frequency_data = FrequencyData {
        f1: FREQUENCY_DATA.0.into(),
        f2: FREQUENCY_DATA.1.into(),
        f3: FREQUENCY_DATA.2.into(),
    };

    // recalculate formant frequencies 5..29 for the mouth (F1) and throat (F2)
    for index in 5..30 {
        // recalculate f1 (mouth formant)
        frequency_data.f1[index] = trans(mouth, frequency_data.f1[index]);

        // recalculate f2 (throat formant)
        frequency_data.f2[index] = trans(throat, frequency_data.f2[index]);
    }

    // recalculate formant frequencies 48..53
    for index in 48..54 {
        // recalculate f1 (mouth formant)
        frequency_data.f1[index] = trans(mouth, frequency_data.f1[index]);

        // recalculate f2 (throat formant)
        frequency_data.f2[index] = trans(throat, frequency_data.f2[index]);
    }

    frequency_data
}

#[derive(Debug, Eq, PartialEq)]
struct Frame {
    pitch: u8,

    // Frequencies
    f1: u8,
    f2: u8,
    f3: u8,

    // Amplitudes
    a1: u8,
    a2: u8,
    a3: u8,

    sampled_consonant_flag: u8,
}

impl Frame {
    fn new() -> Self {
        Self {
            pitch: 0,
            f1: 0,
            f2: 0,
            f3: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            sampled_consonant_flag: 0,
        }
    }
}

const RISING_INFLECTION: u8 = 255;
//const FALLING_INFLECTION: u8 = 1;

enum Inflection {
    Rising,
    Falling,
}

const STRESS_PITCH_TABLE: &[u8] = &[0x00, 0xE0, 0xE6, 0xEC, 0xF3, 0xF9, 0x00, 0x06, 0x0C, 0x06];

/// Apply rising or falling inflection to the last 30 frames of the frame vec.
fn add_inflection(inflection: Inflection, frames: &mut [Frame]) {
    // store the location of the punctuation
    let end = frames.len();

    let mut position = end.saturating_sub(30);

    let mut a;

    // FIXME: Explain this fix better, it's not obvious
    // ML : A =, fixes a problem with invalid pitch with '.'
    loop {
        // TODO: Bounds checking
        a = frames[position].pitch;
        if a != 127 {
            break;
        }

        position += 1;
    }

    while position < end {
        // Add the inflection direction
        match inflection {
            Inflection::Falling => a += 1,
            Inflection::Rising => a -= 1,
        }

        // Set the inflection
        frames[position].pitch = a;

        // Advance position to the next non-255 value, stopping at the end
        loop {
            position += 1;

            // TODO: Investigate why the un-equals check is here, possible bug?
            if position == end || frames[position].pitch != RISING_INFLECTION {
                break;
            }
        }
    }
}

// TODO: Figure out pitch range
fn create_frames(pitch: u8, phonemes: &[Phoneme], frequency_data: &FrequencyData) -> Vec<Frame> {
    let mut frames = Vec::new();

    for phoneme in phonemes {
        if phoneme.index == PHONEME_PERIOD {
            add_inflection(Inflection::Falling, &mut frames);
        } else if phoneme.index == PHONEME_QUESTION_MARK {
            add_inflection(Inflection::Rising, &mut frames);
        }

        // get the stress amount (more stress = higher pitch)
        let phase1 = STRESS_PITCH_TABLE[phoneme.stress as usize];

        // get number of frames to write
        // copy from the source to the frames list
        frames.extend((0..phoneme.length).map(|_| Frame {
            pitch: (pitch.wrapping_add(phase1)),

            f1: frequency_data.f1[phoneme.index],
            f2: frequency_data.f2[phoneme.index],
            f3: frequency_data.f3[phoneme.index],

            a1: AMPLITUDE_DATA[phoneme.index].0,
            a2: AMPLITUDE_DATA[phoneme.index].1,
            a3: AMPLITUDE_DATA[phoneme.index].2,

            sampled_consonant_flag: SAMPLED_CONSONANT_FLAGS[phoneme.index],
        }));
    }

    frames
}

const BLEND_RANK: &[u8] = &[
    0x00, 0x1F, 0x1F, 0x1F, 0x1F, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x05, 0x05,
    0x02, 0x0A, 0x02, 0x08, 0x05, 0x05, 0x0B, 0x0A, 0x09, 0x08, 0x08, 0xA0, 0x08, 0x08, 0x17, 0x1F,
    0x12, 0x12, 0x12, 0x12, 0x1E, 0x1E, 0x14, 0x14, 0x14, 0x14, 0x17, 0x17, 0x1A, 0x1A, 0x1D, 0x1D,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x1A, 0x1D, 0x1B, 0x1A, 0x1D, 0x1B, 0x1A, 0x1D, 0x1B, 0x1A,
    0x1D, 0x1B, 0x17, 0x1D, 0x17, 0x17, 0x1D, 0x17, 0x17, 0x1D, 0x17, 0x17, 0x1D, 0x17, 0x17, 0x17,
];

const OUT_BLEND_LENGTH: &[u8] = &[
    0x00, 0x02, 0x02, 0x02, 0x02, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x04, 0x03, 0x02, 0x04, 0x04, 0x02, 0x02, 0x02, 0x02, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x05,
    0x05, 0x05, 0x05, 0x05, 0x04, 0x04, 0x02, 0x00, 0x01, 0x02, 0x00, 0x01, 0x02, 0x00, 0x01, 0x02,
    0x00, 0x01, 0x02, 0x00, 0x02, 0x02, 0x00, 0x01, 0x03, 0x00, 0x02, 0x03, 0x00, 0x02, 0xA0, 0xA0,
];

const IN_BLEND_LENGTH: &[u8] = &[
    0x00, 0x02, 0x02, 0x02, 0x02, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x04, 0x03, 0x03, 0x04, 0x04, 0x03, 0x03, 0x03, 0x03, 0x03, 0x01, 0x02, 0x03, 0x02, 0x01,
    0x03, 0x03, 0x03, 0x03, 0x01, 0x01, 0x03, 0x03, 0x03, 0x02, 0x02, 0x03, 0x02, 0x03, 0x00, 0x00,
    0x05, 0x05, 0x05, 0x05, 0x04, 0x04, 0x02, 0x00, 0x02, 0x02, 0x00, 0x03, 0x02, 0x00, 0x04, 0x02,
    0x00, 0x03, 0x02, 0x00, 0x02, 0x02, 0x00, 0x02, 0x03, 0x00, 0x03, 0x03, 0x00, 0x03, 0xB0, 0xA0,
];

/*
fn interpolate_buffer(buffer: &mut [u8], change: Option<i8>) {
    let width = buffer.len() - 1;

    let change = change.unwrap_or_else(|| buffer[width] as i8 - buffer[0] as i8);

    let sign = change < 0;
    let remainder = change.unsigned_abs() % width as u8;
    let div = change.checked_div(width as i8).unwrap_or(0);

    //println!("sign remainder div change width {} {} {} {} {}", sign, remainder, div, change, width);

    let mut error = 0;

    for position in 1..width {
        // Ensure value is 7 bits so i8 can be used
        assert!(buffer[position - 1] < 128);

        let mut value = buffer[position - 1] as i8 + div;

        error += remainder;

        if error >= width as u8 {
            // accumulated a whole integer error, so adjust output
            error -= width as u8;

            if sign {
                value -= 1;
            } else if value != 0 {
                // if value is 0, we always leave it alone
                value += 1;
            }
        }

        buffer[position] = value as u8;
    }

    //println!("iterated {} times, len = {}", width - 1, buffer.len());
}
*/

fn interpolate<F>(width: u8, mut table: F, frame: isize, change: i8)
where
    F: FnMut(usize, Option<u8>) -> u8,
{
    let sign = change < 0;
    let remainder = change.unsigned_abs() % width;
    let div = change.checked_div(width as i8).unwrap_or(0);

    let mut error = 0;

    for position in (frame + 1)..(frame + width as isize) {
        // The reference implementation has a bug where the starting frame can sometimes be a
        // negative index. In JavaScript such an array lookup will result in a NaN value, causing
        // the interpolator to write a NaN value, and creating NaN feedback for the rest of the
        // interpolation sequence.
        if position < 1 {
            table(position as usize, Some(0));
            continue;
        }

        let value = table(position as usize - 1, None);

        // Ensure value is 7 bits so i8 can be used safely
        //assert!(value < 128);

        let mut value = value as i16 + div as i16;

        error += remainder;
        if error >= width {
            // Accumulated a whole integer error, so adjust output
            error -= width;

            if sign {
                value -= 1;
            } else if value != 0 {
                // If value is zero it should always be left alone
                value += 1;
            }
        }

        table(position as usize, Some(value as u8));
    }
}

fn create_transitions(frames: &mut Vec<Frame>, phonemes: &[Phoneme]) -> usize {
    let mut boundary: usize = 0;

    for position in 0..(phonemes.len() - 1) {
        let phoneme = phonemes[position].index;
        let next_phoneme = phonemes[position + 1].index;

        // get the ranking of each phoneme
        let next_rank = BLEND_RANK[next_phoneme];
        let rank = BLEND_RANK[phoneme];

        // compare the rank - lower rank value is stronger
        let (out_blend_frames, in_blend_frames) = match rank.cmp(&next_rank) {
            // Same rank, so use out blend lengths from each phoneme
            Ordering::Equal => (OUT_BLEND_LENGTH[phoneme], OUT_BLEND_LENGTH[next_phoneme]),

            // Next phoneme is stronger, so use its blend lengths
            Ordering::Less => (
                IN_BLEND_LENGTH[next_phoneme],
                OUT_BLEND_LENGTH[next_phoneme],
            ),

            // Current phoneme is stronger, so use its blend lengths. Note: the out/in are swapped
            Ordering::Greater => (OUT_BLEND_LENGTH[phoneme], IN_BLEND_LENGTH[phoneme]),
        };

        boundary = boundary.wrapping_add(phonemes[position].length as usize);

        let trans_end = boundary + in_blend_frames as usize;
        let trans_start = boundary as isize - out_blend_frames as isize;
        let trans_length = out_blend_frames + in_blend_frames; // total transition

        // TODO: What does the & 128 do? Check for positive numbers?
        if (trans_length.wrapping_sub(2)) & 128 == 0 {
            // unlike the other values, the pitches[] interpolates from
            // the middle of the current phoneme to the middle of the
            // next phoneme

            // half the width of the current and next phoneme
            let cur_width = phonemes[position].length >> 1;
            let next_width = phonemes[position + 1].length >> 1;

            // Interpolate the pitch
            // TODO: The start position for pitch doesn't seem correct, needs verification
            let pitch = (frames[boundary + next_width as usize].pitch as i16
                - frames[boundary - cur_width as usize].pitch as i16) as i8;

            interpolate(
                cur_width + next_width,
                |index, value| {
                    if let Some(value) = value {
                        // The reference implementation has a bug where it tries to interpolate off
                        // of the end of the frame list. This creates new frames that shouldn't
                        // actually exist. To prevent any indexing panics a new frame is added when
                        // writing off of the end. The new frame is populated with zeroes to mimic
                        // the behavior of "undefined" values in JavaScript in a safe way.
                        if index == frames.len() {
                            frames.push(Frame::new());
                        }

                        frames[index].pitch = value
                    }

                    // TODO: This check is only necessary for debugging
                    if index == frames.len() {
                        0
                    } else {
                        frames[index].pitch
                    }
                },
                trans_start,
                pitch,
            );

            //let mut buffer = frames[range.clone()].iter().map(|frame| frame.pitch).collect::<Vec<_>>();
            //interpolate_buffer(&mut buffer, Some(pitch));
            //frames[range.clone()].iter_mut().zip(buffer.into_iter()).for_each(|(frame, value)| frame.pitch = value);

            // Interpolate the other values
            //let range = trans_start as usize..(trans_start as usize + trans_length as usize + 1);

            // The reference implementation has a bug here where it on some occasions tries to
            // interpolate one frame beyond the end of the frame list. This causes the change delta
            // to become NaN, causing the interpolator to leave the frames untouched (this might
            // not be the desired result). To prevent any panics the interpolation is skipped when
            // trans_end goes beyond the frame list.
            if trans_end >= frames.len() {
                continue;
            }

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].f1 as i8 - frame.f1 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        // The reference implementation has a bug where it tries to interpolate off
                        // of the end of the frame list. This creates new frames that shouldn't
                        // actually exist. To prevent any indexing panics a new frame is added when
                        // writing off of the end. The new frame is populated with zeroes to mimic
                        // the behavior of "undefined" values in JavaScript in a safe way.
                        //if index == frames.len() {
                        //frames.push(Frame::new());
                        //}

                        frames[index].f1 = value
                    }

                    frames[index].f1
                },
                trans_start,
                change,
            );

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].f2 as i8 - frame.f2 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        frames[index].f2 = value
                    }

                    frames[index].f2
                },
                trans_start,
                change,
            );

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].f3 as i8 - frame.f3 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        frames[index].f3 = value
                    }

                    frames[index].f3
                },
                trans_start,
                change,
            );

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].a1 as i8 - frame.a1 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        frames[index].a1 = value
                    }

                    frames[index].a1
                },
                trans_start,
                change,
            );

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].a2 as i8 - frame.a2 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        frames[index].a2 = value
                    }

                    frames[index].a2
                },
                trans_start,
                change,
            );

            let change = frames
                .get(trans_start as usize)
                .map(|frame| frames[trans_end].a3 as i8 - frame.a3 as i8)
                .unwrap_or(0);
            interpolate(
                trans_length,
                |index, value| {
                    if let Some(value) = value {
                        frames[index].a3 = value
                    }

                    frames[index].a3
                },
                trans_start,
                change,
            );
        }
    }

    // add the length of last phoneme
    boundary + phonemes[phonemes.len() - 1].length as usize
}

const AMPLITUDE_RESCALE_TABLE: &[u8] = &[
    0x00, 0x01, 0x02, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0B, 0x0D, 0x0F,
];

struct PreparedFrames {
    // TODO: How does this relate to frames.len()?
    frame_count: usize,
    frames: Vec<Frame>,
}

fn prepare_frames(
    phonemes: &[Phoneme],
    pitch: u8,
    mouth: u8,
    throat: u8,
    sing_mode: bool,
) -> PreparedFrames {
    let frequency_data = set_mouth_and_throat(mouth, throat);
    let mut frames = create_frames(pitch, phonemes, &frequency_data);
    let t = create_transitions(&mut frames, phonemes);

    if !sing_mode {
        // Assing pitch contour
        // subtract half the frequency of the formant 1.
        // this adds variety to the voice
        for frame in frames.iter_mut() {
            frame.pitch = frame.pitch.saturating_sub(frame.f1 >> 1);
        }
    }

    // Rescale volume from decibels to the linear scale.
    for frame in frames.iter_mut() {
        frame.a1 = AMPLITUDE_RESCALE_TABLE[frame.a1 as usize];
        frame.a2 = AMPLITUDE_RESCALE_TABLE[frame.a2 as usize];
        frame.a3 = AMPLITUDE_RESCALE_TABLE[frame.a3 as usize];
    }

    PreparedFrames {
        frame_count: t,
        frames,
    }
}

// Timetable for more accurate C64 simulation
const TIMETABLE: &[[u8; 5]] = &[
    [162, 167, 167, 127, 128], // formants synth
    [226, 60, 60, 0, 0],       // unvoiced sample 0
    [225, 60, 59, 0, 0],       // unvoiced sample 1
    [200, 0, 0, 54, 55],       // voiced sample 0
    [199, 0, 0, 54, 54],       // voiced sample 1
];

struct OutputBuffer {
    buffer: Vec<u8>,
    position: usize,
    old_timetable_index: usize,
}

impl OutputBuffer {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0; size],
            position: 0,
            old_timetable_index: 0,
        }
    }

    fn ary(&mut self, index: usize, array: [u8; 5]) {
        // TODO: index seems to be 0..=2, needs to be verified more on longer sentences
        self.position += TIMETABLE[self.old_timetable_index][index] as usize;

        self.old_timetable_index = index;

        // Write a little bit in advance
        for (index, sample) in array.into_iter().enumerate() {
            self.buffer[self.position / 50 + index] = sample;
        }
    }

    fn get(&self) -> &[u8] {
        &self.buffer[..(self.position / 50)]
    }

    fn write(&mut self, index: usize, a: u8) {
        // Scale by 16 and write 5 times
        // Note: renderer passes in values that are > 16, these are overflowing
        let scaled = (a & 15) * 16;

        self.ary(index, [scaled, scaled, scaled, scaled, scaled]);
    }
}

// Sampled data for consonants, consisting of five 256-byte sections
const SAMPLE_TABLE: &[u8] = &[
    //00  T', S, Z  (coronal)
    0x38, 0x84, 0x6B, 0x19, 0xC6, 0x63, 0x18, 0x86, 0x73, 0x98, 0xC6, 0xB1, 0x1C, 0xCA, 0x31, 0x8C,
    0xC7, 0x31, 0x88, 0xC2, 0x30, 0x98, 0x46, 0x31, 0x18, 0xC6, 0x35, 0x0C, 0xCA, 0x31, 0x0C, 0xC6,
    //20
    0x21, 0x10, 0x24, 0x69, 0x12, 0xC2, 0x31, 0x14, 0xC4, 0x71, 0x08, 0x4A, 0x22, 0x49, 0xAB, 0x6A,
    0xA8, 0xAC, 0x49, 0x51, 0x32, 0xD5, 0x52, 0x88, 0x93, 0x6C, 0x94, 0x22, 0x15, 0x54, 0xD2, 0x25,
    //40
    0x96, 0xD4, 0x50, 0xA5, 0x46, 0x21, 0x08, 0x85, 0x6B, 0x18, 0xC4, 0x63, 0x10, 0xCE, 0x6B, 0x18,
    0x8C, 0x71, 0x19, 0x8C, 0x63, 0x35, 0x0C, 0xC6, 0x33, 0x99, 0xCC, 0x6C, 0xB5, 0x4E, 0xA2, 0x99,
    //60
    0x46, 0x21, 0x28, 0x82, 0x95, 0x2E, 0xE3, 0x30, 0x9C, 0xC5, 0x30, 0x9C, 0xA2, 0xB1, 0x9C, 0x67,
    0x31, 0x88, 0x66, 0x59, 0x2C, 0x53, 0x18, 0x84, 0x67, 0x50, 0xCA, 0xE3, 0x0A, 0xAC, 0xAB, 0x30,
    //80
    0xAC, 0x62, 0x30, 0x8C, 0x63, 0x10, 0x94, 0x62, 0xB1, 0x8C, 0x82, 0x28, 0x96, 0x33, 0x98, 0xD6,
    0xB5, 0x4C, 0x62, 0x29, 0xA5, 0x4A, 0xB5, 0x9C, 0xC6, 0x31, 0x14, 0xD6, 0x38, 0x9C, 0x4B, 0xB4,
    //A0
    0x86, 0x65, 0x18, 0xAE, 0x67, 0x1C, 0xA6, 0x63, 0x19, 0x96, 0x23, 0x19, 0x84, 0x13, 0x08, 0xA6,
    0x52, 0xAC, 0xCA, 0x22, 0x89, 0x6E, 0xAB, 0x19, 0x8C, 0x62, 0x34, 0xC4, 0x62, 0x19, 0x86, 0x63,
    //C0
    0x18, 0xC4, 0x23, 0x58, 0xD6, 0xA3, 0x50, 0x42, 0x54, 0x4A, 0xAD, 0x4A, 0x25, 0x11, 0x6B, 0x64,
    0x89, 0x4A, 0x63, 0x39, 0x8A, 0x23, 0x31, 0x2A, 0xEA, 0xA2, 0xA9, 0x44, 0xC5, 0x12, 0xCD, 0x42,
    //E0
    0x34, 0x8C, 0x62, 0x18, 0x8C, 0x63, 0x11, 0x48, 0x66, 0x31, 0x9D, 0x44, 0x33, 0x1D, 0x46, 0x31,
    0x9C, 0xC6, 0xB1, 0x0C, 0xCD, 0x32, 0x88, 0xC4, 0x73, 0x18, 0x86, 0x73, 0x08, 0xD6, 0x63, 0x58,
    //100 CH', J', SH, ZH  (palato-alveolar)
    0x07, 0x81, 0xE0, 0xF0, 0x3C, 0x07, 0x87, 0x90, 0x3C, 0x7C, 0x0F, 0xC7, 0xC0, 0xC0, 0xF0, 0x7C,
    0x1E, 0x07, 0x80, 0x80, 0x00, 0x1C, 0x78, 0x70, 0xF1, 0xC7, 0x1F, 0xC0, 0x0C, 0xFE, 0x1C, 0x1F,
    //120
    0x1F, 0x0E, 0x0A, 0x7A, 0xC0, 0x71, 0xF2, 0x83, 0x8F, 0x03, 0x0F, 0x0F, 0x0C, 0x00, 0x79, 0xF8,
    0x61, 0xE0, 0x43, 0x0F, 0x83, 0xE7, 0x18, 0xF9, 0xC1, 0x13, 0xDA, 0xE9, 0x63, 0x8F, 0x0F, 0x83,
    //140
    0x83, 0x87, 0xC3, 0x1F, 0x3C, 0x70, 0xF0, 0xE1, 0xE1, 0xE3, 0x87, 0xB8, 0x71, 0x0E, 0x20, 0xE3,
    0x8D, 0x48, 0x78, 0x1C, 0x93, 0x87, 0x30, 0xE1, 0xC1, 0xC1, 0xE4, 0x78, 0x21, 0x83, 0x83, 0xC3,
    //160
    0x87, 0x06, 0x39, 0xE5, 0xC3, 0x87, 0x07, 0x0E, 0x1C, 0x1C, 0x70, 0xF4, 0x71, 0x9C, 0x60, 0x36,
    0x32, 0xC3, 0x1E, 0x3C, 0xF3, 0x8F, 0x0E, 0x3C, 0x70, 0xE3, 0xC7, 0x8F, 0x0F, 0x0F, 0x0E, 0x3C,
    //180
    0x78, 0xF0, 0xE3, 0x87, 0x06, 0xF0, 0xE3, 0x07, 0xC1, 0x99, 0x87, 0x0F, 0x18, 0x78, 0x70, 0x70,
    0xFC, 0xF3, 0x10, 0xB1, 0x8C, 0x8C, 0x31, 0x7C, 0x70, 0xE1, 0x86, 0x3C, 0x64, 0x6C, 0xB0, 0xE1,
    //1A0
    0xE3, 0x0F, 0x23, 0x8F, 0x0F, 0x1E, 0x3E, 0x38, 0x3C, 0x38, 0x7B, 0x8F, 0x07, 0x0E, 0x3C, 0xF4,
    0x17, 0x1E, 0x3C, 0x78, 0xF2, 0x9E, 0x72, 0x49, 0xE3, 0x25, 0x36, 0x38, 0x58, 0x39, 0xE2, 0xDE,
    //1C0
    0x3C, 0x78, 0x78, 0xE1, 0xC7, 0x61, 0xE1, 0xE1, 0xB0, 0xF0, 0xF0, 0xC3, 0xC7, 0x0E, 0x38, 0xC0,
    0xF0, 0xCE, 0x73, 0x73, 0x18, 0x34, 0xB0, 0xE1, 0xC7, 0x8E, 0x1C, 0x3C, 0xF8, 0x38, 0xF0, 0xE1,
    //1E0
    0xC1, 0x8B, 0x86, 0x8F, 0x1C, 0x78, 0x70, 0xF0, 0x78, 0xAC, 0xB1, 0x8F, 0x39, 0x31, 0xDB, 0x38,
    0x61, 0xC3, 0x0E, 0x0E, 0x38, 0x78, 0x73, 0x17, 0x1E, 0x39, 0x1E, 0x38, 0x64, 0xE1, 0xF1, 0xC1,
    //200 P', F, V, TH, DH  ([labio]dental)
    0x4E, 0x0F, 0x40, 0xA2, 0x02, 0xC5, 0x8F, 0x81, 0xA1, 0xFC, 0x12, 0x08, 0x64, 0xE0, 0x3C, 0x22,
    0xE0, 0x45, 0x07, 0x8E, 0x0C, 0x32, 0x90, 0xF0, 0x1F, 0x20, 0x49, 0xE0, 0xF8, 0x0C, 0x60, 0xF0,
    //220
    0x17, 0x1A, 0x41, 0xAA, 0xA4, 0xD0, 0x8D, 0x12, 0x82, 0x1E, 0x1E, 0x03, 0xF8, 0x3E, 0x03, 0x0C,
    0x73, 0x80, 0x70, 0x44, 0x26, 0x03, 0x24, 0xE1, 0x3E, 0x04, 0x4E, 0x04, 0x1C, 0xC1, 0x09, 0xCC,
    //240
    0x9E, 0x90, 0x21, 0x07, 0x90, 0x43, 0x64, 0xC0, 0x0F, 0xC6, 0x90, 0x9C, 0xC1, 0x5B, 0x03, 0xE2,
    0x1D, 0x81, 0xE0, 0x5E, 0x1D, 0x03, 0x84, 0xB8, 0x2C, 0x0F, 0x80, 0xB1, 0x83, 0xE0, 0x30, 0x41,
    //260
    0x1E, 0x43, 0x89, 0x83, 0x50, 0xFC, 0x24, 0x2E, 0x13, 0x83, 0xF1, 0x7C, 0x4C, 0x2C, 0xC9, 0x0D,
    0x83, 0xB0, 0xB5, 0x82, 0xE4, 0xE8, 0x06, 0x9C, 0x07, 0xA0, 0x99, 0x1D, 0x07, 0x3E, 0x82, 0x8F,
    //280
    0x70, 0x30, 0x74, 0x40, 0xCA, 0x10, 0xE4, 0xE8, 0x0F, 0x92, 0x14, 0x3F, 0x06, 0xF8, 0x84, 0x88,
    0x43, 0x81, 0x0A, 0x34, 0x39, 0x41, 0xC6, 0xE3, 0x1C, 0x47, 0x03, 0xB0, 0xB8, 0x13, 0x0A, 0xC2,
    //2A0
    0x64, 0xF8, 0x18, 0xF9, 0x60, 0xB3, 0xC0, 0x65, 0x20, 0x60, 0xA6, 0x8C, 0xC3, 0x81, 0x20, 0x30,
    0x26, 0x1E, 0x1C, 0x38, 0xD3, 0x01, 0xB0, 0x26, 0x40, 0xF4, 0x0B, 0xC3, 0x42, 0x1F, 0x85, 0x32,
    //2C0
    0x26, 0x60, 0x40, 0xC9, 0xCB, 0x01, 0xEC, 0x11, 0x28, 0x40, 0xFA, 0x04, 0x34, 0xE0, 0x70, 0x4C,
    0x8C, 0x1D, 0x07, 0x69, 0x03, 0x16, 0xC8, 0x04, 0x23, 0xE8, 0xC6, 0x9A, 0x0B, 0x1A, 0x03, 0xE0,
    //2E0
    0x76, 0x06, 0x05, 0xCF, 0x1E, 0xBC, 0x58, 0x31, 0x71, 0x66, 0x00, 0xF8, 0x3F, 0x04, 0xFC, 0x0C,
    0x74, 0x27, 0x8A, 0x80, 0x71, 0xC2, 0x3A, 0x26, 0x06, 0xC0, 0x1F, 0x05, 0x0F, 0x98, 0x40, 0xAE,
    //300 /H  (palatal)
    0x01, 0x7F, 0xC0, 0x07, 0xFF, 0x00, 0x0E, 0xFE, 0x00, 0x03, 0xDF, 0x80, 0x03, 0xEF, 0x80, 0x1B,
    0xF1, 0xC2, 0x00, 0xE7, 0xE0, 0x18, 0xFC, 0xE0, 0x21, 0xFC, 0x80, 0x3C, 0xFC, 0x40, 0x0E, 0x7E,
    //320
    0x00, 0x3F, 0x3E, 0x00, 0x0F, 0xFE, 0x00, 0x1F, 0xFF, 0x00, 0x3E, 0xF0, 0x07, 0xFC, 0x00, 0x7E,
    0x10, 0x3F, 0xFF, 0x00, 0x3F, 0x38, 0x0E, 0x7C, 0x01, 0x87, 0x0C, 0xFC, 0xC7, 0x00, 0x3E, 0x04,
    //340
    0x0F, 0x3E, 0x1F, 0x0F, 0x0F, 0x1F, 0x0F, 0x02, 0x83, 0x87, 0xCF, 0x03, 0x87, 0x0F, 0x3F, 0xC0,
    0x07, 0x9E, 0x60, 0x3F, 0xC0, 0x03, 0xFE, 0x00, 0x3F, 0xE0, 0x77, 0xE1, 0xC0, 0xFE, 0xE0, 0xC3,
    //360
    0xE0, 0x01, 0xDF, 0xF8, 0x03, 0x07, 0x00, 0x7E, 0x70, 0x00, 0x7C, 0x38, 0x18, 0xFE, 0x0C, 0x1E,
    0x78, 0x1C, 0x7C, 0x3E, 0x0E, 0x1F, 0x1E, 0x1E, 0x3E, 0x00, 0x7F, 0x83, 0x07, 0xDB, 0x87, 0x83,
    //380
    0x07, 0xC7, 0x07, 0x10, 0x71, 0xFF, 0x00, 0x3F, 0xE2, 0x01, 0xE0, 0xC1, 0xC3, 0xE1, 0x00, 0x7F,
    0xC0, 0x05, 0xF0, 0x20, 0xF8, 0xF0, 0x70, 0xFE, 0x78, 0x79, 0xF8, 0x02, 0x3F, 0x0C, 0x8F, 0x03,
    //3a0
    0x0F, 0x9F, 0xE0, 0xC1, 0xC7, 0x87, 0x03, 0xC3, 0xC3, 0xB0, 0xE1, 0xE1, 0xC1, 0xE3, 0xE0, 0x71,
    0xF0, 0x00, 0xFC, 0x70, 0x7C, 0x0C, 0x3E, 0x38, 0x0E, 0x1C, 0x70, 0xC3, 0xC7, 0x03, 0x81, 0xC1,
    //3c0
    0xC7, 0xE7, 0x00, 0x0F, 0xC7, 0x87, 0x19, 0x09, 0xEF, 0xC4, 0x33, 0xE0, 0xC1, 0xFC, 0xF8, 0x70,
    0xF0, 0x78, 0xF8, 0xF0, 0x61, 0xC7, 0x00, 0x1F, 0xF8, 0x01, 0x7C, 0xF8, 0xF0, 0x78, 0x70, 0x3C,
    //3e0
    0x7C, 0xCE, 0x0E, 0x21, 0x83, 0xCF, 0x08, 0x07, 0x8F, 0x08, 0xC1, 0x87, 0x8F, 0x80, 0xC7, 0xE3,
    0x00, 0x07, 0xF8, 0xE0, 0xEF, 0x00, 0x39, 0xF7, 0x80, 0x0E, 0xF8, 0xE1, 0xE3, 0xF8, 0x21, 0x9F,
    //400 /X  (glottal)
    0xC0, 0xFF, 0x03, 0xF8, 0x07, 0xC0, 0x1F, 0xF8, 0xC4, 0x04, 0xFC, 0xC4, 0xC1, 0xBC, 0x87, 0xF0,
    0x0F, 0xC0, 0x7F, 0x05, 0xE0, 0x25, 0xEC, 0xC0, 0x3E, 0x84, 0x47, 0xF0, 0x8E, 0x03, 0xF8, 0x03,
    //420
    0xFB, 0xC0, 0x19, 0xF8, 0x07, 0x9C, 0x0C, 0x17, 0xF8, 0x07, 0xE0, 0x1F, 0xA1, 0xFC, 0x0F, 0xFC,
    0x01, 0xF0, 0x3F, 0x00, 0xFE, 0x03, 0xF0, 0x1F, 0x00, 0xFD, 0x00, 0xFF, 0x88, 0x0D, 0xF9, 0x01,
    //440
    0xFF, 0x00, 0x70, 0x07, 0xC0, 0x3E, 0x42, 0xF3, 0x0D, 0xC4, 0x7F, 0x80, 0xFC, 0x07, 0xF0, 0x5E,
    0xC0, 0x3F, 0x00, 0x78, 0x3F, 0x81, 0xFF, 0x01, 0xF8, 0x01, 0xC3, 0xE8, 0x0C, 0xE4, 0x64, 0x8F,
    //460
    0xE4, 0x0F, 0xF0, 0x07, 0xF0, 0xC2, 0x1F, 0x00, 0x7F, 0xC0, 0x6F, 0x80, 0x7E, 0x03, 0xF8, 0x07,
    0xF0, 0x3F, 0xC0, 0x78, 0x0F, 0x82, 0x07, 0xFE, 0x22, 0x77, 0x70, 0x02, 0x76, 0x03, 0xFE, 0x00,
    //480
    0xFE, 0x67, 0x00, 0x7C, 0xC7, 0xF1, 0x8E, 0xC6, 0x3B, 0xE0, 0x3F, 0x84, 0xF3, 0x19, 0xD8, 0x03,
    0x99, 0xFC, 0x09, 0xB8, 0x0F, 0xF8, 0x00, 0x9D, 0x24, 0x61, 0xF9, 0x0D, 0x00, 0xFD, 0x03, 0xF0,
    //4a0
    0x1F, 0x90, 0x3F, 0x01, 0xF8, 0x1F, 0xD0, 0x0F, 0xF8, 0x37, 0x01, 0xF8, 0x07, 0xF0, 0x0F, 0xC0,
    0x3F, 0x00, 0xFE, 0x03, 0xF8, 0x0F, 0xC0, 0x3F, 0x00, 0xFA, 0x03, 0xF0, 0x0F, 0x80, 0xFF, 0x01,
    //4c0
    0xB8, 0x07, 0xF0, 0x01, 0xFC, 0x01, 0xBC, 0x80, 0x13, 0x1E, 0x00, 0x7F, 0xE1, 0x40, 0x7F, 0xA0,
    0x7F, 0xB0, 0x00, 0x3F, 0xC0, 0x1F, 0xC0, 0x38, 0x0F, 0xF0, 0x1F, 0x80, 0xFF, 0x01, 0xFC, 0x03,
    //4e0
    0xF1, 0x7E, 0x01, 0xFE, 0x01, 0xF0, 0xFF, 0x00, 0x7F, 0xC0, 0x1D, 0x07, 0xF0, 0x0F, 0xC0, 0x7E,
    0x06, 0xE0, 0x07, 0xE0, 0x0F, 0xF8, 0x06, 0xC1, 0xFE, 0x01, 0xFC, 0x03, 0xE0, 0x0F, 0x00, 0xFC,
];

const SAMPLED_CONSONANT_VALUES_ZERO: &[u8] = &[0x18, 0x1A, 0x17, 0x17, 0x17];

fn render_sample_inner(
    output: &mut OutputBuffer,
    sample_page: u16,
    off: u8,
    index1: u8,
    value1: u8,
    index0: u8,
    value0: u8,
) {
    let mut bit = 8;
    let mut sample = SAMPLE_TABLE[sample_page as usize + off as usize];

    loop {
        if sample & 128 != 0 {
            output.write(index1 as usize, value1);
        } else {
            output.write(index0 as usize, value0);
        }

        sample <<= 1;

        bit -= 1;
        if bit == 0 {
            break;
        }
    }
}

fn render_sample(
    output: &mut OutputBuffer,
    last_sample_offset: usize,
    consonant_flag: u8,
    pitch: u8,
) -> usize {
    // mask low three bits and subtract 1 get value to
    // convert 0 bits on unvoiced samples.
    let kind = (consonant_flag & 7) - 1;

    // determine which value to use from table { 0x18, 0x1A, 0x17, 0x17, 0x17 }
    // T', S, Z               0          0x18   coronal
    // CH', J', SH, ZH        1          0x1A   palato-alveolar
    // P', F, V, TH, DH       2          0x17   [labio]dental
    // /H                     3          0x17   palatal
    // /X                     4          0x17   glottal

    let sample_page: u16 = kind as u16 * 256; // unsigned short
    let mut off = consonant_flag & 248; // unsigned char

    if off == 0 {
        // voiced phoneme: Z*, ZH, V*, DH
        let mut phase1 = (pitch >> 4) ^ 255; // unsigned char

        off = (last_sample_offset & 0xFF) as u8; // unsigned char

        loop {
            render_sample_inner(output, sample_page, off, 3, 26, 4, 6);
            off = off.wrapping_add(1);

            let (new_phase1, overflowed) = phase1.overflowing_add(1);
            phase1 = new_phase1;
            if overflowed {
                break;
            }
        }

        return off as usize;
    }

    // unvoiced
    off ^= 255; // unsigned char

    let value0 = SAMPLED_CONSONANT_VALUES_ZERO[kind as usize]; // unsigned char

    loop {
        render_sample_inner(output, sample_page, off, 2, 5, 1, value0);

        off = off.wrapping_add(1);
        if off == 0 {
            break;
        }
    }

    last_sample_offset
}

fn sinus(x: u8) -> i8 {
    ((2.0 * std::f32::consts::PI * (x as f32 / 256.0)).sin() * 127.0) as i8
}

fn process_frames(output: &mut OutputBuffer, speed: u8, prepared_frames: &PreparedFrames) {
    let mut frame_count = prepared_frames.frame_count;
    let frames = &prepared_frames.frames;

    let mut speed_counter = speed;
    let mut phase1 = 0;
    let mut phase2 = 0;
    let mut phase3 = 0;
    let mut last_sample_offset = 0;
    let mut pos = 0;

    // These two variables are not supposed to underflow, however due to a bug in the reference
    // implementation glottal_pulse can be set to NaN, which will lock it to that value.
    let mut glottal_pulse = frames[0].pitch as isize;
    let mut mem38 = (glottal_pulse * 3) / 4;

    while frame_count > 0 {
        let flags = frames[pos].sampled_consonant_flag;

        // unvoiced sampled phoneme?
        if flags & 248 != 0 {
            last_sample_offset =
                render_sample(output, last_sample_offset, flags, frames[pos & 0xff].pitch);

            // skip ahead two in the phoneme buffer
            pos += 2;
            frame_count -= 2;
            speed_counter = speed;
        } else {
            {
                // Rectangle wave consisting of:
                //   0-128 = 0x90
                // 128-255 = 0x70

                // simulate the glottal pulse and formants
                let mut ary = [0_u8; 5];

                // TODO: Check if u16 is sufficient for these values
                let mut /* unsigned int */ p1: u32 = phase1 * 256; // Fixed point integers because we need to divide later on
                let mut /* unsigned int */ p2: u32 = phase2 * 256;
                let mut /* unsigned int */ p3: u32 = phase3 * 256;

                for sample in ary.iter_mut() {
                    // Sine oscillators
                    let /* signed char */ sp1 = sinus(((p1 >> 8) & 0xff) as u8);
                    let /* signed char */ sp2 = sinus(((p2 >> 8) & 0xff) as u8);

                    // Square oscillator
                    let /* signed char */ rp3: i8 = if 0xff & (p3 >> 8) < 129 {
                        -0x70
                    } else {
                        0x70
                    };

                    let /* signed int */ sin1: i32 = sp1 as i32 * (/* (unsigned char) */ frames[pos].a1 & 0x0F) as i32;
                    let /* signed int */ sin2: i32 = sp2 as i32 * (/* (unsigned char) */ frames[pos].a2 & 0x0F) as i32;
                    let /* signed int */ rect: i32 = rp3 as i32 * (/* (unsigned char) */ frames[pos].a3 & 0x0F) as i32;

                    // Sum the oscillators and convert to unsigned 8 bit audio
                    let mix = (sin1 + sin2 + rect + 4096) / 32;

                    *sample = mix as u8;

                    p1 += frames[pos].f1 as u32 * 256 / 4; // Compromise, this becomes a shift and works well
                    p2 += frames[pos].f2 as u32 * 256 / 4;
                    p3 += frames[pos].f3 as u32 * 256 / 4;
                }

                output.ary(0, ary);
            }

            speed_counter -= 1;

            if speed_counter == 0 {
                pos += 1; //go to next amplitude

                // decrement the frame count
                frame_count -= 1;

                if frame_count == 0 {
                    return;
                }

                speed_counter = speed;
            }

            glottal_pulse -= 1;

            if glottal_pulse != 0 {
                // not finished with a glottal pulse

                mem38 -= 1;

                // within the first 75% of the glottal pulse?
                // is the count non-zero and the sampled flag is zero?
                if mem38 != 0 || flags == 0 {
                    // update the phase of the formants
                    // TODO: we should have a switch to disable this, it causes a pretty nice voice without the masking!
                    phase1 += frames[pos].f1 as u32; // & 0xFF;
                    phase2 += frames[pos].f2 as u32; // & 0xFF;
                    phase3 += frames[pos].f3 as u32; // & 0xFF;

                    continue;
                }

                // voiced sampled phonemes interleave the sample with the
                // glottal pulse. The sample flag is non-zero, so render
                // the sample for the phoneme.
                last_sample_offset =
                    render_sample(output, last_sample_offset, flags, frames[pos & 0xFF].pitch);
            }
        }

        // The reference implementation has a bug and tries to read beyond the end of the frame
        // list. In JavaScript this returns undefined, but in rust this results in a panic.
        if frame_count == 0 {
            break;
        }

        glottal_pulse = frames[pos].pitch as isize;
        if glottal_pulse > 0 {
            mem38 = (glottal_pulse * 3) / 4;
        }

        // reset the formant wave generators to keep them in
        // sync with the glottal pulse
        phase1 = 0;
        phase2 = 0;
        phase3 = 0;
    }
}

pub fn render(
    phonemes: &[Phoneme],
    pitch: u8,
    mouth: u8,
    throat: u8,
    speed: u8,
    sing_mode: bool,
) -> Vec<u8> {
    let prepared_frames = prepare_frames(phonemes, pitch, mouth, throat, sing_mode);

    // Create output buffer
    let mut output = OutputBuffer::new(
        (176.4_f32 * // 22050 / 125
            phonemes.iter().fold(0, |length, phoneme| length + phoneme.length as usize) as f32 * // Combined phoneme length in frames.
            speed as f32)
            .ceil() as usize,
    );

    process_frames(&mut output, speed, &prepared_frames);

    output.get().to_vec()
}

const SAM_SAMPLE_RATE: u32 = 22_050;

pub struct VocalNote {
    pub midi_note: u8,
    pub phonemes: Vec<Phoneme>,
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoiceParams {
    pub mouth: u8,
    pub throat: u8,
    pub speed: u8,
    pub base_midi_note: u8,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            mouth: 128,
            throat: 128,
            speed: 72,
            base_midi_note: 60,
        }
    }
}

#[derive(Debug)]
pub enum VocalError {
    InvalidSampleRate(u32),
    InvalidDuration(Duration),
}

impl std::fmt::Display for VocalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VocalError::InvalidSampleRate(rate) => {
                write!(f, "Sample rate must be greater than 0 (got {rate})")
            }
            VocalError::InvalidDuration(duration) => {
                write!(f, "Duration must be greater than 0 (got {duration:?})")
            }
        }
    }
}

impl std::error::Error for VocalError {}

pub fn render_vocal_note(
    note: VocalNote,
    sample_rate: u32,
    voice: VoiceParams,
) -> Result<Vec<u8>, VocalError> {
    if sample_rate == 0 {
        return Err(VocalError::InvalidSampleRate(sample_rate));
    }

    if note.duration.is_zero() {
        return Err(VocalError::InvalidDuration(note.duration));
    }

    let post_shift_semitones = note.midi_note as f32 - voice.base_midi_note as f32;
    let sam_pitch = midi_note_to_sam_pitch(voice.base_midi_note);
    let rendered = render(
        &note.phonemes,
        sam_pitch,
        voice.mouth,
        voice.throat,
        voice.speed,
        true,
    );

    let post_shifted = if post_shift_semitones != 0.0 {
        pitch_shift_with_formant_compensation(&rendered, post_shift_semitones)
    } else {
        rendered
    };

    let resampled = if sample_rate == SAM_SAMPLE_RATE {
        post_shifted
    } else {
        resample_nearest(&post_shifted, SAM_SAMPLE_RATE, sample_rate)
    };

    let mut fitted = fit_to_duration(resampled, note.duration, sample_rate);
    apply_enhancements(&mut fitted, &note.phonemes, note.duration, sample_rate);

    Ok(fitted)
}

fn midi_note_to_sam_pitch(midi_note: u8) -> u8 {
    let pitch = 100_i16 + (midi_note as i16 - 60_i16) * 2_i16;
    pitch.clamp(0, u8::MAX as i16) as u8
}

fn resample_nearest(input: &[u8], source_rate: u32, target_rate: u32) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let target_len =
        ((input.len() as u128) * (target_rate as u128) / (source_rate as u128)) as usize;
    if target_len == 0 {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(target_len);
    for out_index in 0..target_len {
        let source_index =
            ((out_index as u128) * (source_rate as u128) / (target_rate as u128)) as usize;
        let clamped_source_index = source_index.min(input.len() - 1);
        output.push(input[clamped_source_index]);
    }

    output
}

fn pitch_shift(input: &[u8], semitones: f32) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let ratio = 2.0_f64.powf(semitones as f64 / 12.0);
    let output_len = ((input.len() as f64) / ratio).round().max(1.0) as usize;

    let mut output = Vec::with_capacity(output_len);
    for out_index in 0..output_len {
        let source_index = ((out_index as f64) * ratio).round() as usize;
        let clamped_source_index = source_index.min(input.len() - 1);
        output.push(input[clamped_source_index]);
    }

    output
}

fn pitch_shift_with_formant_compensation(input: &[u8], semitones: f32) -> Vec<u8> {
    let shifted = pitch_shift(input, semitones);
    if shifted.is_empty() {
        return shifted;
    }

    let reference = stretch_to_len(input, shifted.len());
    let shifted_f32 = to_f32(&shifted);
    let reference_f32 = to_f32(&reference);

    let shifted_low = low_pass_clone(&shifted_f32, 0.18);
    let reference_low = low_pass_clone(&reference_f32, 0.18);

    let mut mixed = Vec::with_capacity(shifted.len());
    for index in 0..shifted.len() {
        let shifted_high = shifted_f32[index] - shifted_low[index];
        let sample = reference_low[index] * 0.75 + shifted_high * 0.85;
        mixed.push(sample.clamp(-1.0, 1.0));
    }

    let mut output = vec![128_u8; shifted.len()];
    from_f32(&mixed, &mut output);
    output
}

fn fit_to_duration(mut rendered: Vec<u8>, duration: Duration, sample_rate: u32) -> Vec<u8> {
    let target_len = (duration.as_secs_f64() * sample_rate as f64).round() as usize;

    if rendered.len() > target_len {
        rendered.truncate(target_len);
    } else if rendered.len() < target_len {
        rendered.resize(target_len, 128);
    }

    rendered
}

fn stretch_to_len(input: &[u8], target_len: usize) -> Vec<u8> {
    if input.is_empty() || target_len == 0 {
        return Vec::new();
    }

    let normalized = to_f32(input);
    let mut stretched = Vec::with_capacity(target_len);
    let ratio = input.len() as f64 / target_len as f64;
    for out_index in 0..target_len {
        let source_position = out_index as f64 * ratio;
        stretched.push(sample_linear(&normalized, source_position));
    }

    let mut output = vec![128_u8; target_len];
    from_f32(&stretched, &mut output);
    output
}

fn apply_enhancements(
    samples: &mut [u8],
    phonemes: &[Phoneme],
    duration: Duration,
    sample_rate: u32,
) {
    if samples.is_empty() || phonemes.is_empty() {
        return;
    }

    let mut normalized = to_f32(samples);
    let segment_map = build_segment_map(phonemes, samples.len());

    normalized = apply_pitch_modulation(&normalized, 0.0035, 0.0009, sample_rate);
    apply_amplitude_micro_jitter(&mut normalized, 0.02);
    apply_consonant_emphasis(&mut normalized, &segment_map, 1.18, 0.9);
    apply_aspiration_noise(&mut normalized, &segment_map, 0.035);
    smooth_boundaries(&mut normalized, &segment_map, sample_rate);
    apply_attack_release(&mut normalized, duration);
    soft_clip(&mut normalized, 1.4);
    one_pole_low_pass(&mut normalized, 0.22);

    from_f32(&normalized, samples);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SegmentClass {
    VowelLike,
    ConsonantLike,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    start: usize,
    end: usize,
    class: SegmentClass,
}

fn build_segment_map(phonemes: &[Phoneme], sample_len: usize) -> Vec<Segment> {
    let mut segments = Vec::new();
    let total_units = phonemes
        .iter()
        .fold(0_usize, |acc, phoneme| acc + phoneme.length as usize)
        .max(1);

    let mut unit_cursor = 0_usize;
    for phoneme in phonemes {
        let start = unit_cursor * sample_len / total_units;
        unit_cursor += phoneme.length as usize;
        let mut end = unit_cursor * sample_len / total_units;
        if end <= start {
            end = (start + 1).min(sample_len);
        }

        segments.push(Segment {
            start,
            end,
            class: if is_vowel_like(phoneme.index) {
                SegmentClass::VowelLike
            } else {
                SegmentClass::ConsonantLike
            },
        });
    }

    if let Some(last) = segments.last_mut() {
        last.end = sample_len;
    }

    segments
}

fn is_vowel_like(index: usize) -> bool {
    matches!(index, 5..=22 | 48..=53)
}

fn to_f32(input: &[u8]) -> Vec<f32> {
    input
        .iter()
        .map(|sample| *sample as f32 / 127.5 - 1.0)
        .collect()
}

fn from_f32(input: &[f32], output: &mut [u8]) {
    for (sample, out) in input.iter().zip(output.iter_mut()) {
        let value = ((*sample).clamp(-1.0, 1.0) * 127.5 + 127.5).round();
        *out = value.clamp(0.0, 255.0) as u8;
    }
}

fn sample_linear(input: &[f32], position: f64) -> f32 {
    let floor = position.floor().max(0.0) as usize;
    let ceil = (floor + 1).min(input.len() - 1);
    let frac = (position - floor as f64) as f32;
    input[floor] * (1.0 - frac) + input[ceil] * frac
}

fn apply_pitch_modulation(
    input: &[f32],
    drift_depth: f64,
    jitter_depth: f64,
    sample_rate: u32,
) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(input.len());
    let mut source_position = 0.0_f64;
    let drift_hz = 4.8_f64;

    for index in 0..input.len() {
        let t = index as f64 / sample_rate as f64;
        let drift = (2.0 * std::f64::consts::PI * drift_hz * t).sin() * drift_depth;
        let jitter = hash_noise(index as u64).mul_add(2.0, -1.0) * jitter_depth;
        let ratio = (1.0 + drift + jitter).clamp(0.97, 1.03);
        source_position += ratio;

        if source_position >= (input.len() - 1) as f64 {
            source_position = (input.len() - 1) as f64;
        }

        output.push(sample_linear(input, source_position));
    }

    output
}

fn apply_amplitude_micro_jitter(samples: &mut [f32], depth: f32) {
    for (index, sample) in samples.iter_mut().enumerate() {
        let jitter = (hash_noise(index as u64 + 17) * 2.0 - 1.0) as f32 * depth;
        *sample *= 1.0 + jitter;
    }
}

fn apply_consonant_emphasis(
    samples: &mut [f32],
    segments: &[Segment],
    consonant_gain: f32,
    vowel_gain: f32,
) {
    if segments.is_empty() {
        return;
    }

    let mut emphasized = vec![0.0_f32; samples.len()];
    let mut prev = 0.0_f32;
    for (idx, sample) in samples.iter().enumerate() {
        let high = *sample - prev * 0.96;
        emphasized[idx] = high;
        prev = *sample;
    }

    for segment in segments {
        if segment.start >= segment.end || segment.start >= samples.len() {
            continue;
        }
        let end = segment.end.min(samples.len());
        let gain = match segment.class {
            SegmentClass::ConsonantLike => consonant_gain,
            SegmentClass::VowelLike => vowel_gain,
        };
        for index in segment.start..end {
            samples[index] = samples[index] * gain + emphasized[index] * 0.28;
        }
    }
}

fn apply_aspiration_noise(samples: &mut [f32], segments: &[Segment], amount: f32) {
    for (segment_index, segment) in segments.iter().enumerate() {
        if segment.start >= segment.end || segment.start >= samples.len() {
            continue;
        }

        let end = segment.end.min(samples.len());
        let consonant_weight = if segment.class == SegmentClass::ConsonantLike {
            1.0
        } else {
            0.25
        };

        for index in segment.start..end {
            let position = (index - segment.start) as f32 / ((end - segment.start).max(1) as f32);
            let edge = (1.0 - (position - 0.5).abs() * 2.0).powf(0.6);
            let transition = if segment_index > 0 && segment_index + 1 < segments.len() {
                0.5
            } else {
                0.3
            };
            let noise = (hash_noise(
                (index as u64)
                    .wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223),
            ) * 2.0
                - 1.0) as f32;
            samples[index] += noise * amount * consonant_weight * (edge * 0.65 + transition * 0.35);
        }
    }
}

fn smooth_boundaries(samples: &mut [f32], segments: &[Segment], sample_rate: u32) {
    if segments.len() < 2 {
        return;
    }

    let radius = ((sample_rate as f32 * 0.0015).round() as usize).max(2);
    for boundary in segments.iter().skip(1).map(|seg| seg.start) {
        let start = boundary.saturating_sub(radius);
        let end = (boundary + radius).min(samples.len());
        if end <= start + 1 {
            continue;
        }

        let left = samples[start];
        let right = samples[end - 1];
        for (offset, index) in (start..end).enumerate() {
            let t = offset as f32 / (end - start - 1) as f32;
            let smooth = t * t * (3.0 - 2.0 * t);
            let cross = left * (1.0 - smooth) + right * smooth;
            samples[index] = samples[index] * 0.6 + cross * 0.4;
        }
    }
}

fn apply_attack_release(samples: &mut [f32], duration: Duration) {
    if samples.len() < 4 {
        return;
    }

    let sample_len = samples.len();
    let total_secs = duration.as_secs_f64().max(0.001);
    let attack_secs = total_secs.min(0.02);
    let release_secs = total_secs.min(0.03);
    let attack = ((sample_len as f64 * attack_secs / total_secs).round() as usize).max(1);
    let release = ((sample_len as f64 * release_secs / total_secs).round() as usize).max(1);

    for (index, sample) in samples.iter_mut().enumerate() {
        let mut gain = 1.0_f32;
        if index < attack {
            gain *= index as f32 / attack as f32;
        }
        if index + release >= sample_len {
            let tail_pos = (sample_len - index) as f32 / release as f32;
            gain *= tail_pos.clamp(0.0, 1.0);
        }
        *sample *= gain;
    }
}

fn soft_clip(samples: &mut [f32], drive: f32) {
    for sample in samples.iter_mut() {
        let x = *sample * drive;
        *sample = x / (1.0 + x.abs());
    }
}

fn one_pole_low_pass(samples: &mut [f32], alpha: f32) {
    if samples.is_empty() {
        return;
    }

    let mut state = samples[0];
    for sample in samples.iter_mut() {
        state += alpha * (*sample - state);
        *sample = state;
    }
}

fn low_pass_clone(samples: &[f32], alpha: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(samples.len());
    let mut state = samples[0];
    for sample in samples {
        state += alpha * (*sample - state);
        output.push(state);
    }
    output
}

fn hash_noise(value: u64) -> f64 {
    let mut x = value ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x as f64) / (u64::MAX as f64)
}

}
