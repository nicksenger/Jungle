use std::time::Duration;

use super::parser;
use super::reciter;
use super::renderer;

const SAM_SAMPLE_RATE: u32 = 22_050;

pub enum LyricInput<'a> {
    Text(&'a str),
    PhonemeText(&'a str),
    Phonemes(Vec<parser::Phoneme>),
}

pub struct VocalNote<'a> {
    pub midi_note: u8,
    pub lyric: LyricInput<'a>,
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoiceParams {
    pub mouth: u8,
    pub throat: u8,
    pub speed: u8,
    pub sing_mode: bool,
    pub pitch_mode: PitchMode,
    pub base_midi_note: u8,
    pub hybrid_sam_range: i8,
    pub enhancement_profile: EnhancementProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PitchMode {
    Sam,
    Post,
    Hybrid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnhancementProfile {
    Legacy,
    Improved,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            mouth: 128,
            throat: 128,
            speed: 72,
            sing_mode: true,
            pitch_mode: PitchMode::Post,
            base_midi_note: 60,
            hybrid_sam_range: 4,
            enhancement_profile: EnhancementProfile::Improved,
        }
    }
}

#[derive(Debug)]
pub enum VocalError {
    Reciter(reciter::ReciterError),
    Parse(parser::ParseError),
    InvalidSampleRate(u32),
    InvalidDuration(Duration),
}

impl std::fmt::Display for VocalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VocalError::Reciter(err) => write!(f, "Reciter error: {err}"),
            VocalError::Parse(err) => write!(f, "Parse error: {err}"),
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
    note: VocalNote<'_>,
    sample_rate: u32,
    voice: VoiceParams,
) -> Result<Vec<u8>, VocalError> {
    if sample_rate == 0 {
        return Err(VocalError::InvalidSampleRate(sample_rate));
    }

    if note.duration.is_zero() {
        return Err(VocalError::InvalidDuration(note.duration));
    }

    let parsed = match note.lyric {
        LyricInput::Text(text) => {
            let phrase = reciter::text_to_phonemes(text).map_err(VocalError::Reciter)?;
            parser::parse_phonemes(&phrase).map_err(VocalError::Parse)?
        }
        LyricInput::PhonemeText(phoneme_text) => {
            parser::parse_phonemes(phoneme_text).map_err(VocalError::Parse)?
        }
        LyricInput::Phonemes(phonemes) => phonemes,
    };

    let (sam_midi_note, post_shift_semitones) = match voice.pitch_mode {
        PitchMode::Sam => (note.midi_note, 0.0),
        PitchMode::Post => (
            voice.base_midi_note,
            note.midi_note as f32 - voice.base_midi_note as f32,
        ),
        PitchMode::Hybrid => {
            let delta = note.midi_note as i16 - voice.base_midi_note as i16;
            let sam_delta = delta.clamp(
                -(voice.hybrid_sam_range as i16),
                voice.hybrid_sam_range as i16,
            );
            let sam_note = (voice.base_midi_note as i16 + sam_delta).clamp(0, 127) as u8;
            let post_delta = (delta - sam_delta) as f32;
            (sam_note, post_delta)
        }
    };

    let sam_pitch = midi_note_to_sam_pitch(sam_midi_note);
    let rendered = renderer::render(
        &parsed,
        sam_pitch,
        voice.mouth,
        voice.throat,
        voice.speed,
        voice.sing_mode,
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
    if voice.enhancement_profile == EnhancementProfile::Improved {
        apply_enhancements(&mut fitted, &parsed, note.duration, sample_rate);
    }

    Ok(fitted)
}

fn midi_note_to_sam_pitch(midi_note: u8) -> u8 {
    // Anchor MIDI note 60 (C4) around SAM pitch 100 and clamp to u8 range.
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

    // Keep the low-band spectral envelope from the unshifted signal and
    // combine it with the high-band detail from the pitch-shifted signal.
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
    phonemes: &[parser::Phoneme],
    duration: Duration,
    sample_rate: u32,
) {
    if samples.is_empty() || phonemes.is_empty() {
        return;
    }

    let mut normalized = to_f32(samples);
    let segment_map = build_segment_map(phonemes, samples.len());

    // Naturalness: subtle pitch drift/jitter and amplitude micro-variation.
    normalized = apply_pitch_modulation(&normalized, 0.0035, 0.0009, sample_rate);
    apply_amplitude_micro_jitter(&mut normalized, 0.02);

    // Intelligibility: emphasize consonants and reduce low-mid masking.
    apply_consonant_emphasis(&mut normalized, &segment_map, 1.18, 0.9);

    // Naturalness: add aspiration noise in unvoiced and transition regions.
    apply_aspiration_noise(&mut normalized, &segment_map, 0.035);

    // Naturalness + smoothness: soften hard boundaries between phonemes.
    smooth_boundaries(&mut normalized, &segment_map, sample_rate);

    // Naturalness: gentle envelope to avoid hard note on/offs.
    apply_attack_release(&mut normalized, duration);

    // Gentle saturation and low-pass for less metallic timbre.
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

fn build_segment_map(phonemes: &[parser::Phoneme], sample_len: usize) -> Vec<Segment> {
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
    matches!(
        index,
        5..=22 | 48..=53 // vowel and diphthong-heavy region
    )
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{parser, reciter};
    use super::{
        render_vocal_note, EnhancementProfile, LyricInput, PitchMode, VocalNote, VoiceParams,
    };

    #[test]
    fn defaults_pin_high_quality_path() {
        let defaults = VoiceParams::default();
        assert!(defaults.sing_mode);
        assert_eq!(defaults.pitch_mode, PitchMode::Post);
        assert_eq!(defaults.enhancement_profile, EnhancementProfile::Improved);
    }

    #[test]
    fn text_and_phoneme_inputs_are_output_identical() {
        let text = "welcome to the jungle";
        let duration = Duration::from_millis(900);
        let note = 68;
        let sample_rate = 22_050;
        let voice = VoiceParams::default();

        let rendered_text = render_vocal_note(
            VocalNote {
                midi_note: note,
                lyric: LyricInput::Text(text),
                duration,
            },
            sample_rate,
            voice,
        )
        .expect("text input should render");

        let recited =
            reciter::text_to_phonemes(text).expect("reciter should generate phoneme text");
        let rendered_phoneme_text = render_vocal_note(
            VocalNote {
                midi_note: note,
                lyric: LyricInput::PhonemeText(&recited),
                duration,
            },
            sample_rate,
            voice,
        )
        .expect("phoneme text input should render");
        assert_eq!(rendered_text, rendered_phoneme_text);

        let parsed = parser::parse_phonemes(&recited).expect("parser should parse reciter output");
        let rendered_phonemes = render_vocal_note(
            VocalNote {
                midi_note: note,
                lyric: LyricInput::Phonemes(parsed),
                duration,
            },
            sample_rate,
            voice,
        )
        .expect("phoneme vector input should render");
        assert_eq!(rendered_text, rendered_phonemes);
    }

    #[test]
    fn post_shifted_output_stays_centered_and_energetic() {
        let rendered = render_vocal_note(
            VocalNote {
                midi_note: 72,
                lyric: LyricInput::Text("welcome"),
                duration: Duration::from_millis(700),
            },
            48_000,
            VoiceParams::default(),
        )
        .expect("post-shifted vocal should render");

        let normalized: Vec<f32> = rendered
            .iter()
            .map(|sample| *sample as f32 / 127.5 - 1.0)
            .collect();
        let mean = normalized.iter().sum::<f32>() / normalized.len() as f32;
        let rms = (normalized.iter().map(|sample| sample * sample).sum::<f32>()
            / normalized.len() as f32)
            .sqrt();
        let peak = normalized
            .iter()
            .fold(0.0_f32, |acc, sample| acc.max(sample.abs()));

        assert!(mean.abs() < 0.12, "dc offset too large: {mean}");
        assert!(rms > 0.05, "rendered vocal lost too much energy: {rms}");
        assert!(peak > 0.2, "rendered vocal peak too small: {peak}");
    }
}
