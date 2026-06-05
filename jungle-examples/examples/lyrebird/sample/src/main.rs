use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use tempfile::Builder;
use welcome_audio::dsp::SAMPLE_RATE as SOURCE_SAMPLE_RATE;
use welcome_audio::instrumentation::{
    phonemes_from_text, BassArticulation, CymbalArticulation, ElectricGuitarArticulation,
    HiHatArticulation, KickDrumArticulation, Note, SnareDrumArticulation, SynthHandle,
    TomsArticulation, VocalsArticulation,
};

const DEFAULT_BPM: f64 = 123.0;
const OUTPUT_SAMPLE_RATE: u32 = 44_100;
const TICKS_PER_BEAT: f64 = 384.0;

#[derive(Debug, Parser)]
#[command(name = "lyrebird-sample")]
struct Cli {
    /// Output WAV duration in seconds.
    #[arg(long = "duration-secs", value_parser = parse_duration_secs)]
    duration_secs: f64,
    /// Tempo in beats per minute.
    #[arg(long = "bpm", value_parser = parse_bpm, default_value_t = DEFAULT_BPM)]
    bpm: f64,
    /// Optional explicit output path for the rendered WAV.
    #[arg(long = "output-path")]
    output_path: Option<PathBuf>,
    /// One or more score specs, e.g. `electric-guitar(sustained):[0,60,192],[384,64,96]`,
    /// `vocals(formant):[0,60,192,"jungle"]`, or
    /// `drums:cymbal:[150,57,192];hihat:[1878,46,192];kick-drum:[150,36,192];snare-drum:[1350,38,192];toms:[150,36,192]`.
    #[arg(required = true)]
    specs: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("invalid score spec `{spec}`: {reason}")]
    InvalidSpec { spec: String, reason: String },
    #[error("invalid note tuple `{tuple}` in `{spec}`: {reason}")]
    InvalidTuple {
        spec: String,
        tuple: String,
        reason: String,
    },
    #[error("unsupported instrument `{instrument}`")]
    UnsupportedInstrument { instrument: String },
    #[error("unsupported articulation `{articulation}` for instrument `{instrument}`")]
    UnsupportedArticulation {
        instrument: String,
        articulation: String,
    },
    #[error("output file io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed writing wav file: {0}")]
    Wav(#[from] hound::Error),
    #[error("synth failure for instrument `{0}`")]
    Synth(String),
}

#[derive(Debug, Clone)]
struct ParsedSpec {
    instrument: String,
    articulation: Option<String>,
    events: Vec<NoteEvent>,
}

#[derive(Debug, Clone)]
struct NoteEvent {
    start_ticks: u64,
    midi: u8,
    duration_ticks: u64,
    synthesis_word: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct SynthEvent {
    start_frame: usize,
    pan: f32,
    gain: f32,
    playback_rate: f32,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let mut specs = Vec::new();
    for spec in &cli.specs {
        specs.extend(parse_specs(spec)?);
    }

    let total_frames = duration_to_frames(cli.duration_secs);
    let mut left = vec![0.0f32; total_frames];
    let mut right = vec![0.0f32; total_frames];
    let synth = SynthHandle::new();
    let tick_seconds = tick_duration_seconds(cli.bpm);

    for spec in specs {
        let ParsedSpec {
            instrument,
            articulation,
            events,
        } = spec;

        for event in events {
            let synth_event = SynthEvent {
                start_frame: seconds_to_frame(ticks_to_seconds(event.start_ticks, tick_seconds)),
                pan: 0.0,
                gain: 1.0,
                playback_rate: 1.0,
            };

            let note_duration =
                Duration::from_secs_f64(ticks_to_seconds(event.duration_ticks, tick_seconds));
            match normalized_token(&instrument).as_str() {
                "electric-guitar" => {
                    let articulation =
                        match articulation.as_deref().map(normalized_token).as_deref() {
                            None | Some("rhythm-sustained") => {
                                ElectricGuitarArticulation::RhythmSustained
                            }
                            Some("sustained") => ElectricGuitarArticulation::Sustained,
                            Some(other) => {
                                return Err(CliError::UnsupportedArticulation {
                                    instrument: instrument.clone(),
                                    articulation: other.to_string(),
                                });
                            }
                        };
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate, pan) = synth
                        .electric_guitar(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "bass" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "picked",
                        BassArticulation::Picked,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .bass(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.0,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "vocals" => {
                    let articulation =
                        parse_vocals_articulation(&instrument, articulation.as_deref(), &event)?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .vocals(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.0,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "hihat" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "closed-tip",
                        HiHatArticulation::ClosedTip,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .hihat(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.2,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "kick-drum" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "standard-hit",
                        KickDrumArticulation::StandardHit,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .kick_drum(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.0,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "snare-drum" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "rimshot",
                        SnareDrumArticulation::Rimshot,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .snare_drum(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.08,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "toms" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "standard-hit",
                        TomsArticulation::StandardHit,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .toms(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: -0.14,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                "cymbal" => {
                    let articulation = parse_single_articulation(
                        &instrument,
                        articulation.as_deref(),
                        "standard-crash",
                        CymbalArticulation::StandardCrash,
                    )?;
                    let note = base_note(event.midi, note_duration, articulation);
                    let (pcm, gain, playback_rate) = synth
                        .cymbal(note)
                        .await
                        .map_err(|_| CliError::Synth(instrument.clone()))?;
                    mix_pcm_stereo(
                        &mut left,
                        &mut right,
                        &pcm,
                        SynthEvent {
                            pan: 0.25,
                            gain,
                            playback_rate,
                            ..synth_event
                        },
                    );
                }
                _ => {
                    return Err(CliError::UnsupportedInstrument {
                        instrument: instrument.clone(),
                    });
                }
            }
        }
    }

    let path = write_wav(&left, &right, cli.output_path.as_deref())?;
    println!("{}", path.display());
    Ok(())
}

fn parse_duration_secs(value: &str) -> Result<f64, String> {
    let duration_secs = value
        .parse::<f64>()
        .map_err(|_| format!("invalid duration seconds argument: {value}"))?;
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err("duration seconds must be a finite value > 0".to_string());
    }
    Ok(duration_secs)
}

fn parse_bpm(value: &str) -> Result<f64, String> {
    let bpm = value
        .parse::<f64>()
        .map_err(|_| format!("invalid bpm argument: {value}"))?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return Err("bpm must be a finite value > 0".to_string());
    }
    Ok(bpm)
}

fn parse_specs(spec: &str) -> Result<Vec<ParsedSpec>, CliError> {
    let (head, tail) = spec.split_once(':').ok_or_else(|| CliError::InvalidSpec {
        spec: spec.to_string(),
        reason: "missing ':' separator".to_string(),
    })?;

    let instrument = head
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(head)
        .trim();
    if normalized_token(instrument) != "drums" {
        return Ok(vec![parse_spec(spec)?]);
    }

    if head.contains('(') {
        return Err(CliError::InvalidSpec {
            spec: spec.to_string(),
            reason: "drums does not support a top-level articulation".to_string(),
        });
    }
    let delimiter = if tail.contains(';') {
        ';'
    } else if tail.contains('|') {
        '|'
    } else {
        return Err(CliError::InvalidSpec {
            spec: spec.to_string(),
            reason: "drums requires sub-specs delimited by `;` or `|`".to_string(),
        });
    };

    tail.split(delimiter)
        .map(str::trim)
        .filter(|sub_spec| !sub_spec.is_empty())
        .map(parse_spec)
        .collect()
}

fn parse_spec(spec: &str) -> Result<ParsedSpec, CliError> {
    let (head, tail) = spec.split_once(':').ok_or_else(|| CliError::InvalidSpec {
        spec: spec.to_string(),
        reason: "missing ':' separator".to_string(),
    })?;

    let (instrument, articulation) = if let Some((name, rest)) = head.split_once('(') {
        if !rest.ends_with(')') {
            return Err(CliError::InvalidSpec {
                spec: spec.to_string(),
                reason: "missing closing ')' in articulation".to_string(),
            });
        }
        let articulation = &rest[..rest.len().saturating_sub(1)];
        if articulation.is_empty() {
            return Err(CliError::InvalidSpec {
                spec: spec.to_string(),
                reason: "articulation cannot be empty".to_string(),
            });
        }
        (
            name.trim().to_string(),
            Some(articulation.trim().to_string()),
        )
    } else {
        (head.trim().to_string(), None)
    };

    if instrument.is_empty() {
        return Err(CliError::InvalidSpec {
            spec: spec.to_string(),
            reason: "instrument cannot be empty".to_string(),
        });
    }

    let tuples = extract_tuples(tail, spec)?;
    if tuples.is_empty() {
        return Err(CliError::InvalidSpec {
            spec: spec.to_string(),
            reason: "at least one note tuple is required".to_string(),
        });
    }

    let requires_synthesis_word = normalized_token(&instrument) == "vocals"
        && articulation.as_deref().map(normalized_token).as_deref() == Some("formant");

    let mut events = Vec::with_capacity(tuples.len());
    for tuple in tuples {
        events.push(parse_tuple(spec, tuple, requires_synthesis_word)?);
    }

    Ok(ParsedSpec {
        instrument,
        articulation,
        events,
    })
}

fn extract_tuples<'a>(tail: &'a str, spec: &str) -> Result<Vec<&'a str>, CliError> {
    let mut tuples = Vec::new();
    let mut remainder = tail.trim();
    while !remainder.is_empty() {
        if !remainder.starts_with('[') {
            return Err(CliError::InvalidSpec {
                spec: spec.to_string(),
                reason:
                    "note list must contain bracketed tuples like [start_ticks,midi,duration_ticks]"
                        .to_string(),
            });
        }
        let close_idx = remainder.find(']').ok_or_else(|| CliError::InvalidSpec {
            spec: spec.to_string(),
            reason: "missing closing ']' in note tuple".to_string(),
        })?;
        tuples.push(&remainder[..=close_idx]);
        remainder = remainder[close_idx + 1..].trim();
        if remainder.is_empty() {
            break;
        }
        if !remainder.starts_with(',') {
            return Err(CliError::InvalidSpec {
                spec: spec.to_string(),
                reason: "tuples must be comma-delimited".to_string(),
            });
        }
        remainder = remainder[1..].trim();
    }
    Ok(tuples)
}

fn parse_tuple(
    spec: &str,
    tuple: &str,
    requires_synthesis_word: bool,
) -> Result<NoteEvent, CliError> {
    let inner = tuple
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "tuple must be wrapped in []".to_string(),
        })?;

    let parts: Vec<_> = inner.split(',').map(str::trim).collect();
    let expected_len = if requires_synthesis_word { 4 } else { 3 };
    if parts.len() != expected_len {
        return Err(CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: format!("tuple must contain exactly {expected_len} fields"),
        });
    }

    let start = parts[0];
    let midi = parts[1];
    let duration = parts[2];

    let start_ticks = parse_nonnegative_ticks(start).map_err(|reason| CliError::InvalidTuple {
        spec: spec.to_string(),
        tuple: tuple.to_string(),
        reason,
    })?;
    let duration_ticks =
        parse_nonnegative_ticks(duration).map_err(|reason| CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason,
        })?;
    if duration_ticks == 0 {
        return Err(CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "duration ticks must be > 0".to_string(),
        });
    }

    let midi = midi
        .parse::<u16>()
        .map_err(|_| CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "midi must be an integer between 0 and 255".to_string(),
        })
        .and_then(|n| {
            u8::try_from(n).map_err(|_| CliError::InvalidTuple {
                spec: spec.to_string(),
                tuple: tuple.to_string(),
                reason: "midi must be an integer between 0 and 255".to_string(),
            })
        })?;

    let synthesis_word = if requires_synthesis_word {
        Some(parse_synthesis_word(spec, tuple, parts[3])?)
    } else {
        None
    };

    Ok(NoteEvent {
        start_ticks,
        midi,
        duration_ticks,
        synthesis_word,
    })
}

fn parse_synthesis_word(spec: &str, tuple: &str, value: &str) -> Result<String, CliError> {
    let quoted = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .ok_or_else(|| CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "formant synthesis word must be a quoted string".to_string(),
        })?;

    let synthesis_word = quoted.trim();
    if synthesis_word.is_empty() {
        return Err(CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "formant synthesis word cannot be empty".to_string(),
        });
    }

    Ok(synthesis_word.to_string())
}

fn parse_nonnegative_ticks(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid tick value `{value}`; expected integer >= 0"))
}

fn tick_duration_seconds(bpm: f64) -> f64 {
    60.0 / (bpm * TICKS_PER_BEAT)
}

fn ticks_to_seconds(ticks: u64, tick_seconds: f64) -> f64 {
    (ticks as f64) * tick_seconds
}

fn normalized_token(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .replace(' ', "-")
}

fn parse_single_articulation<T: Copy>(
    instrument: &str,
    articulation: Option<&str>,
    expected: &'static str,
    output: T,
) -> Result<T, CliError> {
    if let Some(articulation) = articulation {
        let normalized = normalized_token(articulation);
        if normalized != expected {
            return Err(CliError::UnsupportedArticulation {
                instrument: instrument.to_string(),
                articulation: articulation.to_string(),
            });
        }
    }
    Ok(output)
}

fn parse_vocals_articulation(
    instrument: &str,
    articulation: Option<&str>,
    event: &NoteEvent,
) -> Result<VocalsArticulation, CliError> {
    match articulation.map(normalized_token).as_deref() {
        None | Some("group-harmony") => Ok(VocalsArticulation::GroupHarmony),
        Some("formant") => {
            let synthesis_word =
                event
                    .synthesis_word
                    .as_deref()
                    .ok_or_else(|| CliError::InvalidSpec {
                        spec: format!(
                            "vocals(formant):[{},{},{}]",
                            event.start_ticks, event.midi, event.duration_ticks
                        ),
                        reason: "formant vocals require a synthesis word in each note tuple"
                            .to_string(),
                    })?;
            Ok(VocalsArticulation::Formant(phonemes_from_text(
                synthesis_word,
            )))
        }
        Some(other) => Err(CliError::UnsupportedArticulation {
            instrument: instrument.to_string(),
            articulation: other.to_string(),
        }),
    }
}

fn base_note<A>(midi: u8, duration: Duration, articulation: A) -> Note<A> {
    Note {
        n_midi: midi,
        amplitude_multiplier: 0.5,
        pan: 0.5,
        duration,
        velocity: 1.0,
        expression: None,
        articulation,
    }
}

fn duration_to_frames(duration_secs: f64) -> usize {
    (duration_secs * OUTPUT_SAMPLE_RATE as f64).round() as usize
}

fn seconds_to_frame(seconds: f64) -> usize {
    (seconds * OUTPUT_SAMPLE_RATE as f64).round() as usize
}

fn mix_pcm_stereo(left: &mut [f32], right: &mut [f32], pcm: &[f32], event: SynthEvent) {
    if event.start_frame >= left.len() {
        return;
    }
    if !event.playback_rate.is_finite() || event.playback_rate <= 0.0 {
        return;
    }

    let pan = event.pan.clamp(-1.0, 1.0);
    let left_gain = (1.0 - pan) * 0.5;
    let right_gain = (1.0 + pan) * 0.5;

    let available = left.len() - event.start_frame;
    for i in 0..available {
        let src_pos = (i as f32) * event.playback_rate * SOURCE_SAMPLE_RATE as f32
            / OUTPUT_SAMPLE_RATE as f32;
        let src_idx = src_pos.floor() as usize;
        if src_idx >= pcm.len() {
            break;
        }

        let frac = src_pos - src_idx as f32;
        let current = pcm[src_idx];
        let next = if src_idx + 1 < pcm.len() {
            pcm[src_idx + 1]
        } else {
            current
        };
        let sample = (current + (next - current) * frac) * event.gain;
        let dst = event.start_frame + i;
        left[dst] += sample * left_gain;
        right[dst] += sample * right_gain;
    }
}

fn write_wav(left: &[f32], right: &[f32], output_path: Option<&Path>) -> Result<PathBuf, CliError> {
    let path = match output_path {
        Some(path) => path.to_path_buf(),
        None => {
            let file = Builder::new()
                .prefix("lyrebird-sample-")
                .suffix(".wav")
                .tempfile()?;
            let (_, path) = file.keep().map_err(|err| err.error)?;
            path
        }
    };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: OUTPUT_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec)?;
    for (&l, &r) in left.iter().zip(right.iter()) {
        writer.write_sample(float_to_i16(l))?;
        writer.write_sample(float_to_i16(r))?;
    }
    writer.finalize()?;
    Ok(path)
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    const LYREBIRD_INTRO_SCORE_SPEC: &str = "electric-guitar(rhythm-sustained):[350,58,96],[350,58,96],[446,58,96],[542,58,96],[542,58,96],[638,56,96],[638,56,96],[734,56,96],[830,56,96],[830,56,96],[926,53,96],[926,53,96],[1022,53,96],[1118,53,96],[1118,53,96],[1214,51,96],[1214,51,96],[1310,51,96],[1406,51,96],[1406,51,96],[1502,49,96],[1502,49,96],[1598,49,96],[1694,46,96],[1694,49,96],[1694,46,96],[1790,49,96],[1790,46,96],[1886,58,96],[1886,58,96],[1982,58,96],[2078,58,96],[2078,58,96],[2174,56,96],[2174,56,96],[2270,56,96],[2366,56,96],[2366,56,96],[2462,53,96],[2462,53,96],[2558,53,96],[2654,53,96],[2654,53,96],[2750,51,96],[2750,51,96],[2846,51,96]";
    const LYREBIRD_VOCALS_SCORE_SPEC: &str = "vocals(formant):[250,66,96,'wel'],[346,68,288,'come'],[634,68,96,'to'],[730,66,96,'the'],[826,71,384,'jun'],[1210,68,192,'gol'],[1786,66,96,'weve'],[1882,68,288,'got'],[2170,68,96,'fun'],[2266,66,192,'and'],[2458,68,288,'games']";
    const LYREBIRD_BACKUP_VOCALS_SCORE_SPEC: &str = "vocals(group-harmony):[150,71,384],[534,70,384],[918,68,384],[1302,66,384],[1686,73,384],[2070,72,384],[2454,70,384],[2838,68,384]";
    const LYREBIRD_GUITAR_SOLO_SCORE_SPEC: &str = "electric-guitar(sustained):[240,60,192],[432,72,128],[560,75,129],[689,82,896],[1585,82,128],[1713,81,129],[1842,80,704],[2546,78,96],[2642,79,96],[2738,73,672],[3410,73,224]";
    const LYREBIRD_BASS_SCORE_SPEC: &str = "bass:[150,32,192],[342,32,192],[534,30,192],[726,27,96],[822,32,192],[1014,27,96],[1110,30,192],[1302,29,192],[1494,27,192],[1686,32,192],[1878,32,192],[2070,30,192],[2262,27,96],[2358,32,192],[2550,27,96],[2646,42,96],[2838,42,96],[3030,42,96]";
    const LYREBIRD_DRUMS_KICK_DRUM_SCORE_SPEC: &str = "kick-drum:[150,36,192],[438,36,192],[726,36,192],[1110,36,192],[1686,36,48],[1686,36,192],[2454,36,192],[3030,36,192],[3222,36,192]";
    const LYREBIRD_DRUMS_TOMS_SCORE_SPEC: &str = "toms:[150,36,192],[438,36,192],[726,36,192],[1110,36,192],[1686,36,48],[1686,36,192],[2454,36,192],[3030,36,192],[3222,36,192]";

    #[test]
    fn parse_formant_vocals_spec_requires_synthesis_word() {
        let err = parse_spec("vocals(formant):[0,60,192]").unwrap_err();
        assert!(matches!(err, CliError::InvalidTuple { .. }));
        assert!(err.to_string().contains("exactly 4 fields"));
    }

    #[test]
    fn parse_formant_vocals_spec_captures_synthesis_word() {
        let parsed = parse_spec("vocals(formant):[0,60,192,\"jungle\"]").unwrap();

        assert_eq!(parsed.instrument, "vocals");
        assert_eq!(parsed.articulation.as_deref(), Some("formant"));
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].synthesis_word.as_deref(), Some("jungle"));
    }

    #[test]
    fn parse_non_formant_vocals_spec_rejects_extra_tuple_field() {
        let err = parse_spec("vocals(group-harmony):[0,60,192,\"jungle\"]").unwrap_err();
        assert!(matches!(err, CliError::InvalidTuple { .. }));
        assert!(err.to_string().contains("exactly 3 fields"));
    }

    #[test]
    fn parse_vocals_articulation_defaults_to_group_harmony() {
        let articulation = parse_vocals_articulation(
            "vocals",
            None,
            &NoteEvent {
                start_ticks: 0,
                midi: 60,
                duration_ticks: 192,
                synthesis_word: None,
            },
        )
        .unwrap();

        assert!(matches!(articulation, VocalsArticulation::GroupHarmony));
    }

    #[test]
    fn parse_vocals_articulation_rejects_clean() {
        let err = parse_vocals_articulation(
            "vocals",
            Some("clean"),
            &NoteEvent {
                start_ticks: 0,
                midi: 60,
                duration_ticks: 192,
                synthesis_word: None,
            },
        )
        .unwrap_err();

        assert!(matches!(err, CliError::UnsupportedArticulation { .. }));
        assert!(err.to_string().contains("unsupported articulation `clean`"));
    }

    #[test]
    fn parse_lyrebird_intro_score_spec_uses_rhythm_sustained_guitar() {
        let parsed = parse_spec(LYREBIRD_INTRO_SCORE_SPEC).unwrap();

        assert_eq!(parsed.instrument, "electric-guitar");
        assert_eq!(parsed.articulation.as_deref(), Some("rhythm-sustained"));
        assert_eq!(parsed.events.len(), 46);
    }

    #[test]
    fn parse_remaining_lyrebird_score_specs() {
        for spec in [
            LYREBIRD_VOCALS_SCORE_SPEC,
            LYREBIRD_BACKUP_VOCALS_SCORE_SPEC,
            LYREBIRD_GUITAR_SOLO_SCORE_SPEC,
            LYREBIRD_BASS_SCORE_SPEC,
            LYREBIRD_DRUMS_KICK_DRUM_SCORE_SPEC,
            LYREBIRD_DRUMS_TOMS_SCORE_SPEC,
        ] {
            let parsed = parse_spec(spec).unwrap();
            assert!(!parsed.instrument.is_empty());
            assert!(!parsed.events.is_empty());
        }
    }

    #[test]
    fn parse_drums_composite_spec_expands_to_component_specs() {
        let parsed = parse_specs(
            "drums:cymbal:[150,57,192];hihat:[1878,46,192];kick-drum:[150,36,192];snare-drum:[1350,38,192];toms:[150,36,192]",
        )
        .unwrap();

        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0].instrument, "cymbal");
        assert_eq!(parsed[1].instrument, "hihat");
        assert_eq!(parsed[2].instrument, "kick-drum");
        assert_eq!(parsed[3].instrument, "snare-drum");
        assert_eq!(parsed[4].instrument, "toms");
    }

    #[test]
    fn write_wav_uses_44100_output_rate() {
        let output = Builder::new()
            .prefix("lyrebird-sample-test-")
            .suffix(".wav")
            .tempfile()
            .unwrap();
        let path = output.path().to_path_buf();

        write_wav(&[0.0], &[0.0], Some(&path)).unwrap();

        let reader = hound::WavReader::open(path).unwrap();
        assert_eq!(reader.spec().sample_rate, OUTPUT_SAMPLE_RATE);
    }
}
