use std::{
    env, fs,
    path::{Path, PathBuf},
};

const DEFAULT_BPM: f64 = 123.0;
const DEFAULT_BEATS_PER_BAR: u32 = 4;
const DEFAULT_BEAT_UNIT: u32 = 4;
const DEFAULT_EVENT_PREVIEW: usize = 12;

#[derive(Debug, Clone)]
struct ScoreEvent {
    start_tick: u32,
    duration_tick: u32,
    n_midi: u8,
}

#[derive(Debug, Clone)]
struct ScoreFile {
    name: String,
    ticks_per_quarter_note: u32,
    tempo_micros_per_quarter_note: u64,
    events: Vec<ScoreEvent>,
}

#[derive(Debug, Clone)]
struct Config {
    bpm: f64,
    beats_per_bar: u32,
    beat_unit: u32,
    preview_events: usize,
    score_filter: Option<String>,
    score_dir: PathBuf,
}

fn main() -> Result<(), String> {
    let config = Config::from_env()?;
    let mut score_files = load_score_files(&config.score_dir)?;

    if let Some(filter) = &config.score_filter {
        score_files.retain(|score| score.name == *filter);
        if score_files.is_empty() {
            return Err(format!("No score file matched --score {filter:?}"));
        }
    }

    println!(
        "Welcome score grid report ({} BPM, {}/{})",
        config.bpm, config.beats_per_bar, config.beat_unit
    );
    println!("score_dir={}", config.score_dir.display());

    for score in score_files {
        print_score_report(&score, &config)?;
    }

    Ok(())
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let mut bpm = DEFAULT_BPM;
        let mut beats_per_bar = DEFAULT_BEATS_PER_BAR;
        let mut beat_unit = DEFAULT_BEAT_UNIT;
        let mut preview_events = DEFAULT_EVENT_PREVIEW;
        let mut score_filter = None;
        let mut score_dir = default_score_dir()?;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bpm" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "Missing value for --bpm".to_string())?;
                    bpm = value
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid --bpm value: {value}"))?;
                    if !bpm.is_finite() || bpm <= 0.0 {
                        return Err("--bpm must be a positive finite number".to_string());
                    }
                }
                "--beats-per-bar" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "Missing value for --beats-per-bar".to_string())?;
                    beats_per_bar = value
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid --beats-per-bar value: {value}"))?;
                    if beats_per_bar == 0 {
                        return Err("--beats-per-bar must be > 0".to_string());
                    }
                }
                "--beat-unit" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "Missing value for --beat-unit".to_string())?;
                    beat_unit = value
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid --beat-unit value: {value}"))?;
                    if beat_unit == 0 {
                        return Err("--beat-unit must be > 0".to_string());
                    }
                }
                "--preview-events" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "Missing value for --preview-events".to_string())?;
                    preview_events = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --preview-events value: {value}"))?;
                }
                "--score" => {
                    score_filter = Some(
                        args.next()
                            .ok_or_else(|| "Missing value for --score".to_string())?,
                    );
                }
                "--score-dir" => {
                    score_dir = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "Missing value for --score-dir".to_string())?,
                    );
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    return Err(format!(
                        "Unknown arg: {arg}. Run with --help to see supported arguments."
                    ));
                }
            }
        }

        Ok(Self {
            bpm,
            beats_per_bar,
            beat_unit,
            preview_events,
            score_filter,
            score_dir,
        })
    }
}

fn print_help() {
    println!("Usage: cargo run -p jungle-examples --bin welcome_score_grid -- [options]");
    println!("Options:");
    println!("  --bpm <number>             Tempo used for wall-clock timestamps (default: 123)");
    println!("  --beats-per-bar <u32>      Time signature numerator (default: 4)");
    println!("  --beat-unit <u32>          Time signature denominator (default: 4)");
    println!("  --preview-events <usize>   Per-score event lines to print (default: 12)");
    println!("  --score <name>             Only report one score file (e.g. lead_guitar)");
    println!("  --score-dir <path>         Override score directory");
}

fn default_score_dir() -> Result<PathBuf, String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|err| format!("CARGO_MANIFEST_DIR is missing: {err}"))?;
    Ok(Path::new(&manifest_dir)
        .join("examples")
        .join("welcome")
        .join("score"))
}

fn load_score_files(score_dir: &Path) -> Result<Vec<ScoreFile>, String> {
    let entries = fs::read_dir(score_dir)
        .map_err(|err| format!("Failed reading {}: {err}", score_dir.display()))?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed reading directory entry: {err}"))?;
        let path = entry.path();
        let is_rs = path.extension().and_then(|ext| ext.to_str()) == Some("rs");
        let is_mod = path.file_stem().and_then(|stem| stem.to_str()) == Some("mod");
        if is_rs && !is_mod {
            paths.push(path);
        }
    }

    paths.sort();

    let mut score_files = Vec::with_capacity(paths.len());
    for path in paths {
        score_files.push(parse_score_file(&path)?);
    }

    Ok(score_files)
}

fn parse_score_file(path: &Path) -> Result<ScoreFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("Failed reading {}: {err}", path.display()))?;
    let mut ticks_per_quarter_note = None;
    let mut tempo_micros_per_quarter_note = None;
    let mut events = Vec::new();

    let mut start_tick: Option<u32> = None;
    let mut duration_tick: Option<u32> = None;
    let mut n_midi: Option<u8> = None;
    let mut in_score_block = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("const TICKS_PER_QUARTER_NOTE:") {
            ticks_per_quarter_note = Some(parse_number_in_line::<u32>(trimmed)?);
            continue;
        }

        if trimmed.starts_with("const TEMPO_MICROS_PER_QUARTER_NOTE:") {
            tempo_micros_per_quarter_note = Some(parse_number_in_line::<u64>(trimmed)?);
            continue;
        }

        if trimmed.starts_with("const SCORE: &[ScoreEvent] = &[") {
            in_score_block = true;
            continue;
        }

        if in_score_block && trimmed == "];" {
            in_score_block = false;
            continue;
        }

        if !in_score_block {
            continue;
        }

        if trimmed.starts_with("start_tick:") {
            start_tick = Some(parse_number_in_line::<u32>(trimmed)?);
            continue;
        }

        if trimmed.starts_with("duration_tick:") {
            duration_tick = Some(parse_number_in_line::<u32>(trimmed)?);
            continue;
        }

        if trimmed.starts_with("n_midi:") {
            n_midi = Some(parse_number_in_line::<u8>(trimmed)?);
            continue;
        }

        if trimmed == "}," {
            if let (Some(start_tick), Some(duration_tick), Some(n_midi)) =
                (start_tick.take(), duration_tick.take(), n_midi.take())
            {
                events.push(ScoreEvent {
                    start_tick,
                    duration_tick,
                    n_midi,
                });
            }
        }
    }

    let ticks_per_quarter_note = ticks_per_quarter_note
        .ok_or_else(|| format!("{} missing TICKS_PER_QUARTER_NOTE constant", path.display()))?;

    let tempo_micros_per_quarter_note = tempo_micros_per_quarter_note.ok_or_else(|| {
        format!(
            "{} missing TEMPO_MICROS_PER_QUARTER_NOTE constant",
            path.display()
        )
    })?;

    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("Invalid score filename: {}", path.display()))?
        .to_string();

    Ok(ScoreFile {
        name,
        ticks_per_quarter_note,
        tempo_micros_per_quarter_note,
        events,
    })
}

fn parse_number_in_line<T>(line: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    let value = line
        .split_whitespace()
        .last()
        .ok_or_else(|| format!("Invalid line, expected a trailing value: {line}"))?
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(',')
        .replace('_', "");

    value
        .parse::<T>()
        .map_err(|_| format!("Failed to parse numeric value from line: {line}"))
}

fn print_score_report(score: &ScoreFile, config: &Config) -> Result<(), String> {
    if score.events.is_empty() {
        println!("\n[{}] no score events", score.name);
        return Ok(());
    }

    let ticks_per_beat = ticks_per_beat(score.ticks_per_quarter_note, config.beat_unit)?;
    let ticks_per_bar = ticks_per_beat.saturating_mul(config.beats_per_bar);
    if ticks_per_bar == 0 {
        return Err("Computed ticks_per_bar is zero".to_string());
    }

    let max_end_tick = score
        .events
        .iter()
        .map(|event| event.start_tick.saturating_add(event.duration_tick))
        .max()
        .unwrap_or(0);

    let total_bars = (max_end_tick / ticks_per_bar) + 1;
    let source_bpm = 60_000_000.0 / score.tempo_micros_per_quarter_note as f64;

    println!("\n[{}]", score.name);
    println!(
        "  events={} ticks/quarter={} source_bpm={source_bpm:.3}",
        score.events.len(),
        score.ticks_per_quarter_note
    );
    println!(
        "  ticks/beat={} ticks/bar={} estimated_total_bars={}",
        ticks_per_beat, ticks_per_bar, total_bars
    );

    let preview_len = config.preview_events.min(score.events.len());
    for event in score.events.iter().take(preview_len) {
        let location = describe_location(event.start_tick, ticks_per_beat, ticks_per_bar, config);
        let value_name = note_value_name(event.duration_tick, score.ticks_per_quarter_note);
        let beats = event.duration_tick as f64 / ticks_per_beat as f64;
        let seconds = beats * 60.0 / config.bpm;

        println!(
            "  - tick={:<6} midi={:<3} dur_tick={:<4} ({:<12}, {:>5.3} beats, {:>5.3}s) @ {}",
            event.start_tick,
            event.n_midi,
            event.duration_tick,
            value_name,
            beats,
            seconds,
            location
        );
    }

    if score.events.len() > preview_len {
        println!(
            "  ... {} more events (increase with --preview-events)",
            score.events.len() - preview_len
        );
    }

    Ok(())
}

fn ticks_per_beat(ticks_per_quarter_note: u32, beat_unit: u32) -> Result<u32, String> {
    let numerator = (ticks_per_quarter_note as u64).saturating_mul(4);
    if !numerator.is_multiple_of(beat_unit as u64) {
        return Err(format!(
            "Cannot represent beat-unit 1/{beat_unit} with ticks/quarter={ticks_per_quarter_note}"
        ));
    }
    let ticks = numerator / beat_unit as u64;
    u32::try_from(ticks).map_err(|_| "ticks_per_beat overflowed u32".to_string())
}

fn describe_location(
    start_tick: u32,
    ticks_per_beat: u32,
    ticks_per_bar: u32,
    config: &Config,
) -> String {
    let bar = start_tick / ticks_per_bar + 1;
    let tick_in_bar = start_tick % ticks_per_bar;
    let beat = tick_in_bar / ticks_per_beat + 1;
    let tick_in_beat = tick_in_bar % ticks_per_beat;

    if tick_in_beat == 0 {
        return format!("bar {bar}, beat {beat}");
    }

    let (num, den) = reduced_fraction(tick_in_beat, ticks_per_beat);
    let subdivision = subdivision_name(num, den)
        .map(|name| format!(" ({name})"))
        .unwrap_or_default();

    format!(
        "bar {bar}, beat {beat} + {num}/{den} beat{subdivision} in {}/{}",
        config.beats_per_bar, config.beat_unit
    )
}

fn note_value_name(duration_tick: u32, ticks_per_quarter_note: u32) -> String {
    let whole = ticks_per_quarter_note.saturating_mul(4);
    let (num, den) = reduced_fraction(duration_tick, whole);

    if let Some(name) = exact_note_value_name(num, den) {
        return name.to_string();
    }

    format!("{num}/{den} whole")
}

fn exact_note_value_name(num: u32, den: u32) -> Option<&'static str> {
    match (num, den) {
        (1, 1) => Some("whole"),
        (1, 2) => Some("half"),
        (1, 4) => Some("quarter"),
        (1, 8) => Some("eighth"),
        (1, 16) => Some("sixteenth"),
        (1, 32) => Some("thirty-second"),
        (3, 4) => Some("dotted half"),
        (3, 8) => Some("dotted quarter"),
        (3, 16) => Some("dotted eighth"),
        (3, 32) => Some("dotted sixteenth"),
        _ => None,
    }
}

fn subdivision_name(num: u32, den: u32) -> Option<&'static str> {
    match (num, den) {
        (1, 2) => Some("offbeat eighth"),
        (1, 4) => Some("sixteenth 2"),
        (1, 3) => Some("triplet 2"),
        (3, 4) => Some("sixteenth 4"),
        (2, 3) => Some("triplet 3"),
        _ => None,
    }
}

fn reduced_fraction(numerator: u32, denominator: u32) -> (u32, u32) {
    if numerator == 0 {
        return (0, 1);
    }
    let divisor = gcd(numerator, denominator.max(1));
    (numerator / divisor, denominator / divisor.max(1))
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}
