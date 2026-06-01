use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use tempfile::Builder;
use welcome_audio::dsp::SAMPLE_RATE;
use welcome_audio::instrumentation::{
    BassArticulation, CymbalArticulation, ElectricGuitarArticulation, HiHatArticulation,
    KickDrumArticulation, Note, SnareDrumArticulation, SynthHandle, TomsArticulation,
    VocalsArticulation,
};

const DEFAULT_BPM: f64 = 123.0;
const TICKS_PER_BEAT: f64 = 384.0;

#[derive(Debug, Parser)]
#[command(name = "mockingbird-sample")]
struct Cli {
    /// Output WAV duration in seconds.
    #[arg(long = "duration-secs", value_parser = parse_duration_secs)]
    duration_secs: f64,
    /// Tempo in beats per minute.
    #[arg(long = "bpm", value_parser = parse_bpm, default_value_t = DEFAULT_BPM)]
    bpm: f64,
    /// One or more score specs, e.g. `electric-guitar(sustained):[0,60,192],[384,64,96]`.
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

#[derive(Debug, Clone, Copy)]
struct NoteEvent {
    start_ticks: u64,
    midi: u8,
    duration_ticks: u64,
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
    let specs = cli
        .specs
        .iter()
        .map(|spec| parse_spec(spec))
        .collect::<Result<Vec<_>, _>>()?;

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
                        match articulation.as_deref().map(normalized_token).as_deref() {
                            None | Some("clean") => VocalsArticulation::Clean,
                            Some("group-harmony") => VocalsArticulation::GroupHarmony,
                            Some(other) => {
                                return Err(CliError::UnsupportedArticulation {
                                    instrument: instrument.clone(),
                                    articulation: other.to_string(),
                                });
                            }
                        };
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

    let path = write_wav_to_temp(&left, &right)?;
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

    let mut events = Vec::with_capacity(tuples.len());
    for tuple in tuples {
        events.push(parse_tuple(spec, tuple)?);
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

fn parse_tuple(spec: &str, tuple: &str) -> Result<NoteEvent, CliError> {
    let inner = tuple
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .ok_or_else(|| CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "tuple must be wrapped in []".to_string(),
        })?;

    let mut parts = inner.split(',').map(str::trim);
    let start = parts.next().ok_or_else(|| CliError::InvalidTuple {
        spec: spec.to_string(),
        tuple: tuple.to_string(),
        reason: "missing start ticks".to_string(),
    })?;
    let midi = parts.next().ok_or_else(|| CliError::InvalidTuple {
        spec: spec.to_string(),
        tuple: tuple.to_string(),
        reason: "missing midi value".to_string(),
    })?;
    let duration = parts.next().ok_or_else(|| CliError::InvalidTuple {
        spec: spec.to_string(),
        tuple: tuple.to_string(),
        reason: "missing duration ticks".to_string(),
    })?;

    if parts.next().is_some() {
        return Err(CliError::InvalidTuple {
            spec: spec.to_string(),
            tuple: tuple.to_string(),
            reason: "tuple must contain exactly 3 fields".to_string(),
        });
    }

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

    Ok(NoteEvent {
        start_ticks,
        midi,
        duration_ticks,
    })
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
    (duration_secs * SAMPLE_RATE as f64).round() as usize
}

fn seconds_to_frame(seconds: f64) -> usize {
    (seconds * SAMPLE_RATE as f64).round() as usize
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
        let src_pos = (i as f32) * event.playback_rate;
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

fn write_wav_to_temp(left: &[f32], right: &[f32]) -> Result<PathBuf, CliError> {
    let file = Builder::new()
        .prefix("mockingbird-sample-")
        .suffix(".wav")
        .tempfile()?;
    let (_, path) = file.keep().map_err(|err| err.error)?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
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
