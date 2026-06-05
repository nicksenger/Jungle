#![allow(dead_code)]

use crate::mcts::{SearchTree, Submission};
use crate::tokens::{Prompt, TokenPredictor, ToolCall};
use crate::{
    aggregate_sample_score, DspCode, LyrebirdAudioMetricErrors, LyrebirdAudioMetrics,
    LyrebirdBranchNode, LyrebirdGeneratedCandidate, LyrebirdInstrument, LyrebirdInstrumentTag,
    LyrebirdPatch, LyrebirdPreparedCandidate, PulseCodePurgatory, LYREBIRD_DURATION_SECS,
};
use futures::future::join_all;
use image::ImageReader;
use jungle_sdk::effect;
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use spectrs::io::audio::read_audio_file_mono;
use spectrs::io::image::{save_spectrogram_image, Colormap};
use spectrs::spectrogram::mel::{par_convert_to_mel, MelScale};
use spectrs::spectrogram::stft::{par_compute_spectrogram, SpectrogramType};
use std::f32::consts::PI;
use std::fs;
use std::future::{ready, Future};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, info, warn};
use zensim::{RgbSlice, Zensim, ZensimProfile};

fn stub_ok<T>(value: T) -> impl Future<Output = Result<T, String>> {
    ready(Ok(value))
}

const MEL_N_FFT: usize = 2048;
const MEL_HOP_LENGTH: usize = 512;
const MEL_WIN_LENGTH: usize = 2048;
const MEL_N_MELS: usize = 256;
const MEL_F_MIN_HZ: f32 = 20.0;
const MEL_F_MAX_HZ: f32 = 16_000.0;
const ANALYSIS_FRAME_SIZE: usize = 2048;
const ANALYSIS_HOP_SIZE: usize = 512;
const SPECTRAL_ROLLOFF_FRACTION: f32 = 0.85;
const SAMPLER_MANIFEST_PATH: &str = "jungle-examples/examples/lyrebird/sample/Cargo.toml";
const SAMPLER_COMMAND_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Clone, Copy, Debug)]
struct ScoredAudioSample {
    mel_similarity: f32,
    score: f32,
    audio_metrics: LyrebirdAudioMetrics,
    audio_metric_errors: LyrebirdAudioMetricErrors,
}

pub struct CreateSessionDB;
#[effect(id = 1)]
impl<J> Effect<J> for CreateSessionDB {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}

pub struct CompareSpectrograms;
#[effect(id = 4)]
impl<J> Effect<J> for CompareSpectrograms {
    type In = (String, String);
    type Out = f32;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let (left_path, right_path) = input;
            compare_spectrograms(&left_path, &right_path)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIterationTimingInput {
    pub completed_iteration: u64,
    pub completed_iteration_id: String,
    pub previous_iteration_start_time_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogIterationTimingOutput {
    pub iteration_start_time_ms: u64,
}

pub struct LogIterationTimingEffect;
#[effect(id = 17)]
impl<J> Effect<J> for LogIterationTimingEffect {
    type In = LogIterationTimingInput;
    type Out = LogIterationTimingOutput;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { log_iteration_timing(input) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptModelInput {
    pub prompt: Prompt,
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub prompt_attempt: u32,
    pub candidate_index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildOptimizationPromptInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub target_spectrogram_path: String,
    pub target_audio_metrics: LyrebirdAudioMetrics,
    pub code_branch: Vec<LyrebirdBranchNode>,
    pub prompt_attempt: u32,
    pub retry_reason: Option<String>,
}

pub struct BuildOptimizationPrompt;
#[effect(id = 12)]
impl<J> Effect<J> for BuildOptimizationPrompt {
    type In = BuildOptimizationPromptInput;
    type Out = Prompt;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { build_optimization_prompt(input) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareToolCallsInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub prompt_attempt: u32,
    pub tool_name: String,
    pub current_source: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepareToolCallsOutcome {
    pub retry_reason: Option<String>,
    pub generated_patch: Option<LyrebirdPatch>,
    pub generated_source: Option<String>,
}

pub struct PrepareToolCalls;
#[effect(id = 13)]
impl<J> Effect<J> for PrepareToolCalls {
    type In = PrepareToolCallsInput;
    type Out = PrepareToolCallsOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { prepare_tool_calls(input).await }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptCandidateResponse {
    pub candidate_index: usize,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub retry_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestPromptCandidatesInput {
    pub prompt: Prompt,
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub prompt_attempt: u32,
    pub instrument_parallelism: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestPromptCandidatesOutcome {
    pub responses: Vec<PromptCandidateResponse>,
}

pub struct RequestPromptCandidates;
#[effect(id = 16)]
impl Effect<()> for RequestPromptCandidates {
    type In = RequestPromptCandidatesInput;
    type Out = RequestPromptCandidatesOutcome;
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async {
            Err("RequestPromptCandidates requires PulseCodePurgatory runtime context".to_owned())
        }
    }
}

#[effect(id = 16)]
impl Effect<PulseCodePurgatory> for RequestPromptCandidates {
    type In = RequestPromptCandidatesInput;
    type Out = RequestPromptCandidatesOutcome;
    type Err = String;

    fn effect(
        jungle: &PulseCodePurgatory,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { request_prompt_candidates(jungle, input).await }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparePromptCandidatesInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub prompt_attempt: u32,
    pub tool_name: String,
    pub current_source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub instrument_parallelism: usize,
    pub responses: Vec<PromptCandidateResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreparePromptCandidatesOutcome {
    pub candidates: Vec<LyrebirdPreparedCandidate>,
    pub retry_reason: Option<String>,
}

pub struct PreparePromptCandidates;
#[effect(id = 18)]
impl<J> Effect<J> for PreparePromptCandidates {
    type In = PreparePromptCandidatesInput;
    type Out = PreparePromptCandidatesOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { prepare_prompt_candidates(input).await }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilePreparedPatchInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub prompt_attempt: u32,
    pub dsp_source_path: String,
    pub original_source: String,
    pub generated_source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilePreparedPatchOutcome {
    pub compile_ok: bool,
    pub retry_reason: Option<String>,
}

pub struct CompilePreparedPatch;
#[effect(id = 15)]
impl<J> Effect<J> for CompilePreparedPatch {
    type In = CompilePreparedPatchInput;
    type Out = CompilePreparedPatchOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { compile_prepared_patch(input).await }
    }
}

pub struct SearchTreeSelect<Marker>(std::marker::PhantomData<fn() -> Marker>);
#[effect(id = 9)]
impl<Marker> Effect<()> for SearchTreeSelect<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type In = ();
    type Out = Vec<LyrebirdBranchNode>;
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSelect requires PulseCodePurgatory runtime context".to_owned()) }
    }
}

#[effect(id = 9)]
impl<Marker> Effect<PulseCodePurgatory> for SearchTreeSelect<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    <PulseCodePurgatory as SearchTree<Marker>>::Data: Send + 'static,
    <PulseCodePurgatory as SearchTree<Marker>>::Error: Send + 'static,
{
    type In = ();
    type Out = <PulseCodePurgatory as SearchTree<Marker>>::Data;
    type Err = <PulseCodePurgatory as SearchTree<Marker>>::Error;

    fn effect(
        jungle: &PulseCodePurgatory,
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { <PulseCodePurgatory as SearchTree<Marker>>::select(jungle).await }
    }
}

pub struct SearchTreeSubmit<Marker>(std::marker::PhantomData<fn() -> Marker>);
#[effect(id = 10)]
impl<Marker> Effect<()> for SearchTreeSubmit<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type In = Vec<Submission<Vec<LyrebirdBranchNode>>>;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSubmit requires PulseCodePurgatory runtime context".to_owned()) }
    }
}

#[effect(id = 10)]
impl<Marker> Effect<PulseCodePurgatory> for SearchTreeSubmit<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    <PulseCodePurgatory as SearchTree<Marker>>::Data: Send + 'static,
    <PulseCodePurgatory as SearchTree<Marker>>::Error: Send + 'static,
{
    type In = Vec<Submission<<PulseCodePurgatory as SearchTree<Marker>>::Data>>;
    type Out = ();
    type Err = <PulseCodePurgatory as SearchTree<Marker>>::Error;

    fn effect(
        jungle: &PulseCodePurgatory,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { <PulseCodePurgatory as SearchTree<Marker>>::submit(jungle, input).await }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterationCandidateInput {
    pub patch: LyrebirdPatch,
    pub generated_source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateIterationAudioInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub dsp_source_path: String,
    pub original_source: String,
    pub candidates: Vec<IterationCandidateInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterationCandidatesOutcome {
    pub candidates: Vec<LyrebirdGeneratedCandidate>,
    pub retry_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateIterationMelsInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub candidates: Vec<LyrebirdGeneratedCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompareIterationMelsInput {
    pub iteration_id: String,
    pub instrument: LyrebirdInstrument,
    pub target_spectrogram_path: String,
    pub target_audio_metrics: LyrebirdAudioMetrics,
    pub candidates: Vec<LyrebirdGeneratedCandidate>,
}

pub struct GenerateIterationAudio;
#[effect(id = 11)]
impl<J> Effect<J> for GenerateIterationAudio {
    type In = GenerateIterationAudioInput;
    type Out = IterationCandidatesOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { generate_iteration_audio(input).await }
    }
}

pub struct GenerateIterationMels;
#[effect(id = 19)]
impl<J> Effect<J> for GenerateIterationMels {
    type In = GenerateIterationMelsInput;
    type Out = IterationCandidatesOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { generate_iteration_mels(input).await }
    }
}

pub struct CompareIterationMels;
#[effect(id = 20)]
impl<J> Effect<J> for CompareIterationMels {
    type In = CompareIterationMelsInput;
    type Out = IterationCandidatesOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { compare_iteration_mels(input).await }
    }
}

pub async fn capture_current_dsp_code_snapshot(
    iteration_id: &str,
    output_root: &Path,
    instrument: LyrebirdInstrument,
    target_spectrogram_path: &Path,
    target_audio_metrics: &LyrebirdAudioMetrics,
    dsp_source_path: &Path,
) -> Result<DspCode, String> {
    let source = fs::read_to_string(dsp_source_path).map_err(|err| {
        format!(
            "failed to read dsp source {}: {err}",
            dsp_source_path.display()
        )
    })?;
    let iteration_dir = output_root.join(iteration_id);
    let sample_path = iteration_dir.join(format!("{}.wav", instrument.output_stem()));
    let spectrogram_path = iteration_dir.join(format!("{}.png", instrument.output_stem()));

    run_sampler(
        LYREBIRD_DURATION_SECS,
        &sample_path.display().to_string(),
        &instrument_score_specs(instrument),
    )
    .await?;
    generate_mel_spectrogram(
        &sample_path.display().to_string(),
        &spectrogram_path.display().to_string(),
    )?;
    let scored_sample = score_rendered_sample(
        &sample_path.display().to_string(),
        &spectrogram_path.display().to_string(),
        &target_spectrogram_path.display().to_string(),
        *target_audio_metrics,
    )?;
    info!(
        iteration_id,
        instrument = instrument.slug(),
        score = scored_sample.score,
        mel_similarity = scored_sample.mel_similarity,
        sample_path = %sample_path.display(),
        spectrogram_path = %spectrogram_path.display(),
        "captured lyrebird dsp snapshot"
    );

    Ok(DspCode {
        iteration_id: iteration_id.to_owned(),
        source,
        sample_path: sample_path.display().to_string(),
        spectrogram_path: spectrogram_path.display().to_string(),
        mel_similarity: Some(scored_sample.mel_similarity),
        score: Some(scored_sample.score),
        audio_metrics: Some(scored_sample.audio_metrics),
        audio_metric_errors: Some(scored_sample.audio_metric_errors),
    })
}

pub(crate) fn generate_mel_spectrogram(wav_path: &str, output_path: &str) -> Result<(), String> {
    let input = Path::new(wav_path);
    let output = Path::new(output_path);
    ensure_parent_dir(output)?;

    let (audio, sample_rate) =
        read_audio_file_mono(input).map_err(|err| format!("failed to read wav file: {err}"))?;

    let spectrogram = par_compute_spectrogram(
        &audio,
        MEL_N_FFT,
        MEL_HOP_LENGTH,
        MEL_WIN_LENGTH,
        true,
        SpectrogramType::Power,
    );
    let mel = par_convert_to_mel(
        &spectrogram,
        sample_rate,
        MEL_N_FFT,
        MEL_N_MELS,
        Some(MEL_F_MIN_HZ),
        Some(MEL_F_MAX_HZ),
        MelScale::Slaney,
    );

    save_spectrogram_image(&mel, output.to_path_buf(), Colormap::Viridis)
        .map_err(|err| format!("failed to write mel spectrogram: {err}"))?;

    Ok(())
}

pub(crate) fn analyze_audio_file(path: &str) -> Result<LyrebirdAudioMetrics, String> {
    let (audio, sample_rate) = read_audio_file_mono(Path::new(path))
        .map_err(|err| format!("failed to read wav file: {err}"))?;
    Ok(analyze_audio_signal(&audio, sample_rate))
}

fn analyze_audio_signal(audio: &[f32], sample_rate: u32) -> LyrebirdAudioMetrics {
    if audio.is_empty() {
        return LyrebirdAudioMetrics::default();
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(ANALYSIS_FRAME_SIZE);
    let window = hann_window(ANALYSIS_FRAME_SIZE);
    let nyquist_hz = sample_rate as f32 / 2.0;
    let bin_hz = sample_rate as f32 / ANALYSIS_FRAME_SIZE as f32;

    let mut zero_crossing_rate_sum = 0.0;
    let mut crest_factor_sum = 0.0;
    let mut spectral_centroid_sum = 0.0;
    let mut spectral_flatness_sum = 0.0;
    let mut spectral_rolloff_sum = 0.0;
    let mut frame_count = 0usize;

    for frame_start in analysis_frame_starts(audio.len()) {
        let mut samples = vec![0.0f32; ANALYSIS_FRAME_SIZE];
        for (offset, value) in samples.iter_mut().enumerate() {
            if let Some(sample) = audio.get(frame_start + offset) {
                *value = *sample;
            }
        }

        zero_crossing_rate_sum += zero_crossing_rate(&samples);
        crest_factor_sum += crest_factor(&samples);

        let mut spectrum = samples
            .iter()
            .zip(window.iter())
            .map(|(sample, weight)| Complex::new(sample * weight, 0.0))
            .collect::<Vec<_>>();
        fft.process(&mut spectrum);

        let (magnitudes, powers) = spectral_bins(&spectrum);
        spectral_centroid_sum += spectral_centroid_hz(&magnitudes, bin_hz);
        spectral_flatness_sum += spectral_flatness(&powers);
        spectral_rolloff_sum += spectral_rolloff_hz(&powers, bin_hz, nyquist_hz);
        frame_count = frame_count.saturating_add(1);
    }

    let frame_count = frame_count.max(1) as f32;
    LyrebirdAudioMetrics {
        zero_crossing_rate: zero_crossing_rate_sum / frame_count,
        crest_factor: crest_factor_sum / frame_count,
        spectral_centroid: spectral_centroid_sum / frame_count,
        spectral_flatness: spectral_flatness_sum / frame_count,
        spectral_rolloff: spectral_rolloff_sum / frame_count,
    }
}

fn analysis_frame_starts(sample_len: usize) -> Vec<usize> {
    if sample_len <= ANALYSIS_FRAME_SIZE {
        return vec![0];
    }

    let mut starts = Vec::new();
    let mut start = 0usize;
    while start < sample_len {
        starts.push(start);
        start = start.saturating_add(ANALYSIS_HOP_SIZE);
    }
    starts
}

fn hann_window(frame_size: usize) -> Vec<f32> {
    if frame_size <= 1 {
        return vec![1.0; frame_size.max(1)];
    }

    (0..frame_size)
        .map(|index| 0.5 - 0.5 * ((2.0 * PI * index as f32) / frame_size as f32).cos())
        .collect()
}

fn zero_crossing_rate(frame: &[f32]) -> f32 {
    if frame.len() <= 1 {
        return 0.0;
    }

    let mut previous_sign = sample_sign(frame[0]);
    let mut crossings = 0usize;
    for &sample in &frame[1..] {
        let sign = sample_sign(sample);
        if sign == 0 {
            continue;
        }
        if previous_sign != 0 && previous_sign != sign {
            crossings = crossings.saturating_add(1);
        }
        previous_sign = sign;
    }

    crossings as f32 / (frame.len().saturating_sub(1).max(1) as f32)
}

fn sample_sign(sample: f32) -> i8 {
    if sample > 0.0 {
        1
    } else if sample < 0.0 {
        -1
    } else {
        0
    }
}

fn crest_factor(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }

    let peak = frame.iter().map(|sample| sample.abs()).fold(0.0, f32::max);
    let rms = (frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len() as f32).sqrt();
    if rms <= f32::EPSILON {
        0.0
    } else {
        peak / rms
    }
}

fn spectral_bins(spectrum: &[Complex<f32>]) -> (Vec<f32>, Vec<f32>) {
    let usable_len = (spectrum.len() / 2).saturating_add(1);
    let mut magnitudes = Vec::with_capacity(usable_len);
    let mut powers = Vec::with_capacity(usable_len);
    for bin in spectrum.iter().take(usable_len) {
        let magnitude = bin.norm();
        magnitudes.push(magnitude);
        powers.push((magnitude * magnitude).max(1e-12));
    }
    (magnitudes, powers)
}

fn spectral_centroid_hz(magnitudes: &[f32], bin_hz: f32) -> f32 {
    let total_magnitude = magnitudes.iter().sum::<f32>();
    if total_magnitude <= f32::EPSILON {
        return 0.0;
    }

    magnitudes
        .iter()
        .enumerate()
        .map(|(index, magnitude)| index as f32 * bin_hz * magnitude)
        .sum::<f32>()
        / total_magnitude
}

fn spectral_flatness(powers: &[f32]) -> f32 {
    if powers.is_empty() {
        return 0.0;
    }

    let geometric_mean =
        (powers.iter().map(|power| power.ln()).sum::<f32>() / powers.len() as f32).exp();
    let arithmetic_mean = powers.iter().sum::<f32>() / powers.len() as f32;
    if arithmetic_mean <= f32::EPSILON {
        0.0
    } else {
        (geometric_mean / arithmetic_mean).clamp(0.0, 1.0)
    }
}

fn spectral_rolloff_hz(powers: &[f32], bin_hz: f32, nyquist_hz: f32) -> f32 {
    let total_power = powers.iter().sum::<f32>();
    if total_power <= f32::EPSILON {
        return 0.0;
    }

    let threshold = total_power * SPECTRAL_ROLLOFF_FRACTION;
    let mut cumulative_power = 0.0;
    for (index, power) in powers.iter().enumerate() {
        cumulative_power += power;
        if cumulative_power >= threshold {
            return (index as f32 * bin_hz).min(nyquist_hz);
        }
    }

    nyquist_hz
}

fn score_rendered_sample(
    sample_path: &str,
    spectrogram_path: &str,
    target_spectrogram_path: &str,
    target_audio_metrics: LyrebirdAudioMetrics,
) -> Result<ScoredAudioSample, String> {
    let mel_similarity = compare_spectrograms(spectrogram_path, target_spectrogram_path)?;
    let audio_metrics = analyze_audio_file(sample_path)?;
    let audio_metric_errors = audio_metrics.relative_errors(target_audio_metrics);
    let score = aggregate_sample_score(mel_similarity, audio_metric_errors);

    Ok(ScoredAudioSample {
        mel_similarity,
        score,
        audio_metrics,
        audio_metric_errors,
    })
}

fn compare_spectrograms(left_path: &str, right_path: &str) -> Result<f32, String> {
    let left = ImageReader::open(left_path)
        .map_err(|err| format!("failed to open spectrogram image {}: {err}", left_path))?
        .decode()
        .map_err(|err| format!("failed to decode spectrogram image {}: {err}", left_path))?
        .into_rgb8();
    let right = ImageReader::open(right_path)
        .map_err(|err| format!("failed to open spectrogram image {}: {err}", right_path))?
        .decode()
        .map_err(|err| format!("failed to decode spectrogram image {}: {err}", right_path))?
        .into_rgb8();
    let (left, right) = if left.dimensions() == right.dimensions() {
        (left, right)
    } else {
        let width = left.width().min(right.width());
        let height = left.height().min(right.height());
        debug!(
            left_path,
            right_path,
            left_width = left.width(),
            left_height = left.height(),
            right_width = right.width(),
            right_height = right.height(),
            crop_width = width,
            crop_height = height,
            "cropping spectrograms to shared dimensions before comparison"
        );
        (
            image::imageops::crop_imm(&left, 0, 0, width, height).to_image(),
            image::imageops::crop_imm(&right, 0, 0, width, height).to_image(),
        )
    };

    let width = left.width() as usize;
    let height = left.height() as usize;
    let left_pixels: Vec<[u8; 3]> = left.pixels().map(|pixel| pixel.0).collect();
    let right_pixels: Vec<[u8; 3]> = right.pixels().map(|pixel| pixel.0).collect();
    let zensim = Zensim::new(ZensimProfile::latest());
    let similarity = zensim
        .compute(
            &RgbSlice::new(&left_pixels, width, height),
            &RgbSlice::new(&right_pixels, width, height),
        )
        .map_err(|err| format!("failed to compare spectrograms: {err}"))?;

    Ok(normalize_zensim(similarity.score() as f32))
}

fn normalize_zensim(raw_score: f32) -> f32 {
    if raw_score >= 100.0 {
        return 1.0;
    }

    let k = 0.05;
    let x0 = 30.0;

    1.0 / (1.0 + (-(raw_score - x0) * k).exp())
}

async fn generate_iteration_audio(
    input: GenerateIterationAudioInput,
) -> Result<IterationCandidatesOutcome, String> {
    let mut candidates = Vec::new();
    let mut retry_reasons = Vec::new();

    for candidate in input.candidates {
        let compiled = compile_prepared_patch(CompilePreparedPatchInput {
            iteration_id: input.iteration_id.clone(),
            instrument: input.instrument,
            prompt_attempt: 0,
            dsp_source_path: input.dsp_source_path.clone(),
            original_source: input.original_source.clone(),
            generated_source: Some(candidate.generated_source.clone()),
        })
        .await?;
        if !compiled.compile_ok {
            if let Some(retry_reason) = compiled.retry_reason {
                retry_reasons.push(retry_reason);
            }
            continue;
        }

        match generate_candidate_audio(
            &input.iteration_id,
            input.instrument,
            &input.dsp_source_path,
            &input.original_source,
            &candidate.generated_source,
            &candidate.sample_path,
        )
        .await
        {
            Ok(()) => candidates.push(LyrebirdGeneratedCandidate {
                patch: candidate.patch,
                code: DspCode {
                    iteration_id: input.iteration_id.clone(),
                    source: candidate.generated_source,
                    sample_path: candidate.sample_path,
                    spectrogram_path: candidate.spectrogram_path,
                    mel_similarity: None,
                    score: None,
                    audio_metrics: None,
                    audio_metric_errors: None,
                },
            }),
            Err(err) => retry_reasons.push(err),
        }
    }

    Ok(IterationCandidatesOutcome {
        retry_reason: if candidates.is_empty() {
            summarize_retry_reasons(&retry_reasons)
        } else {
            None
        },
        candidates,
    })
}

async fn generate_iteration_mels(
    input: GenerateIterationMelsInput,
) -> Result<IterationCandidatesOutcome, String> {
    let mut candidates = Vec::new();
    let mut retry_reasons = Vec::new();

    for candidate in input.candidates {
        match generate_candidate_mel(
            &input.iteration_id,
            input.instrument,
            &candidate.code.sample_path,
            &candidate.code.spectrogram_path,
        ) {
            Ok(()) => candidates.push(candidate),
            Err(err) => retry_reasons.push(err),
        }
    }

    Ok(IterationCandidatesOutcome {
        retry_reason: if candidates.is_empty() {
            summarize_retry_reasons(&retry_reasons)
        } else {
            None
        },
        candidates,
    })
}

async fn compare_iteration_mels(
    input: CompareIterationMelsInput,
) -> Result<IterationCandidatesOutcome, String> {
    let mut candidates = Vec::new();
    let mut retry_reasons = Vec::new();

    for candidate in input.candidates {
        match compare_candidate_mel(
            &input.iteration_id,
            input.instrument,
            &candidate.code.sample_path,
            &candidate.code.spectrogram_path,
            &input.target_spectrogram_path,
            input.target_audio_metrics,
        ) {
            Ok(similarity) => candidates.push(LyrebirdGeneratedCandidate {
                patch: candidate.patch,
                code: DspCode {
                    mel_similarity: Some(similarity.mel_similarity),
                    score: Some(similarity.score),
                    audio_metrics: Some(similarity.audio_metrics),
                    audio_metric_errors: Some(similarity.audio_metric_errors),
                    ..candidate.code
                },
            }),
            Err(err) => retry_reasons.push(err),
        }
    }

    Ok(IterationCandidatesOutcome {
        retry_reason: if candidates.is_empty() {
            summarize_retry_reasons(&retry_reasons)
        } else {
            None
        },
        candidates,
    })
}

async fn request_prompt_candidates(
    jungle: &PulseCodePurgatory,
    input: RequestPromptCandidatesInput,
) -> Result<RequestPromptCandidatesOutcome, String> {
    let prompt_results = join_all((0..input.instrument_parallelism).map(|candidate_index| {
        request_prompt_model_candidate(
            jungle,
            PromptModelInput {
                prompt: input.prompt.clone(),
                iteration_id: input.iteration_id.clone(),
                instrument: input.instrument,
                prompt_attempt: input.prompt_attempt,
                candidate_index,
            },
        )
    }))
    .await;

    Ok(RequestPromptCandidatesOutcome {
        responses: prompt_results
            .into_iter()
            .enumerate()
            .map(|(candidate_index, result)| match result {
                Ok(tool_calls) => PromptCandidateResponse {
                    candidate_index,
                    tool_calls: Some(tool_calls),
                    retry_reason: None,
                },
                Err(err) => PromptCandidateResponse {
                    candidate_index,
                    tool_calls: None,
                    retry_reason: Some(err),
                },
            })
            .collect(),
    })
}

async fn request_prompt_model_candidate(
    jungle: &PulseCodePurgatory,
    input: PromptModelInput,
) -> Result<Vec<ToolCall>, String> {
    if input.candidate_index == 0 {
        debug!(
            iteration_id = %input.iteration_id,
            instrument = input.instrument.slug(),
            prompt_attempt = input.prompt_attempt,
            prompt = %render_prompt_for_log(&input.prompt),
            "sending lyrebird prompt model request"
        );
    }
    let prompt_started_at = Instant::now();
    let response = jungle
        .predict(input.prompt, Some(input.instrument))
        .await
        .map_err(|err| err.to_string());
    let prompt_elapsed_ms = prompt_started_at.elapsed().as_millis();
    match &response {
        Ok(tool_calls) => info!(
            iteration_id = %input.iteration_id,
            instrument = input.instrument.slug(),
            prompt_attempt = input.prompt_attempt,
            candidate_index = input.candidate_index,
            prompt_elapsed_ms,
            tool_call_count = tool_calls.len(),
            "received prompt model response"
        ),
        Err(error) => info!(
            iteration_id = %input.iteration_id,
            instrument = input.instrument.slug(),
            prompt_attempt = input.prompt_attempt,
            candidate_index = input.candidate_index,
            prompt_elapsed_ms,
            error,
            "prompt model request failed"
        ),
    }
    response
}

async fn prepare_prompt_candidates(
    input: PreparePromptCandidatesInput,
) -> Result<PreparePromptCandidatesOutcome, String> {
    let mut candidates = Vec::new();
    let mut retry_reasons = Vec::new();

    for response in input.responses {
        let Some(tool_calls) = response.tool_calls else {
            if let Some(retry_reason) = response.retry_reason {
                retry_reasons.push(retry_reason);
            }
            continue;
        };

        let prepared = prepare_tool_calls(PrepareToolCallsInput {
            iteration_id: input.iteration_id.clone(),
            instrument: input.instrument,
            prompt_attempt: input.prompt_attempt,
            tool_name: input.tool_name.clone(),
            current_source: input.current_source.clone(),
            tool_calls,
        })
        .await?;

        let (generated_patch, generated_source) =
            match (prepared.generated_patch, prepared.generated_source) {
                (Some(generated_patch), Some(generated_source)) => {
                    (generated_patch, generated_source)
                }
                _ => {
                    if let Some(retry_reason) = prepared.retry_reason {
                        retry_reasons.push(retry_reason);
                    }
                    continue;
                }
            };

        let (sample_path, spectrogram_path) = candidate_output_paths(
            &input.sample_path,
            &input.spectrogram_path,
            input.instrument_parallelism,
            response.candidate_index,
        );
        candidates.push(LyrebirdPreparedCandidate {
            patch: generated_patch,
            source: generated_source,
            sample_path,
            spectrogram_path,
        });
    }

    Ok(PreparePromptCandidatesOutcome {
        retry_reason: if candidates.is_empty() {
            summarize_retry_reasons(&retry_reasons)
        } else {
            None
        },
        candidates,
    })
}

fn log_iteration_timing(
    input: LogIterationTimingInput,
) -> Result<LogIterationTimingOutput, String> {
    let iteration_start_time_ms = current_time_ms()?;
    if let Some(iteration_elapsed_ms) = iteration_elapsed_ms(
        iteration_start_time_ms,
        input.previous_iteration_start_time_ms,
    ) {
        info!(
            iteration = input.completed_iteration,
            iteration_id = %input.completed_iteration_id,
            iteration_elapsed_ms,
            "completed lyrebird iteration"
        );
    }
    Ok(LogIterationTimingOutput {
        iteration_start_time_ms,
    })
}

fn current_time_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock before unix epoch: {err}"))?;
    let millis = elapsed.as_millis();
    u64::try_from(millis).map_err(|_| format!("system time overflowed u64 milliseconds: {millis}"))
}

fn iteration_elapsed_ms(now_ms: u64, previous_iteration_start_time_ms: Option<u64>) -> Option<u64> {
    previous_iteration_start_time_ms
        .map(|previous_start_ms| now_ms.saturating_sub(previous_start_ms))
}

fn render_prompt_for_log(prompt: &Prompt) -> String {
    serde_json::to_string_pretty(prompt).unwrap_or_else(|_| format!("{prompt:?}"))
}

async fn generate_candidate_audio(
    iteration_id: &str,
    instrument: LyrebirdInstrument,
    dsp_source_path: &str,
    original_source: &str,
    generated_source: &str,
    sample_path: &str,
) -> Result<(), String> {
    with_temporary_dsp_source(
        dsp_source_path,
        generated_source,
        original_source,
        || async {
            run_sampler(
                LYREBIRD_DURATION_SECS,
                sample_path,
                &instrument_score_specs(instrument),
            )
            .await
        },
    )
    .await
    .map_err(|err| {
        warn!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            error = %err,
            "lyrebird sample build failed; skipping candidate render"
        );
        err
    })?;

    info!(
        iteration_id = %iteration_id,
        instrument = instrument.slug(),
        sample_path,
        "generated lyrebird candidate audio"
    );
    Ok(())
}

fn generate_candidate_mel(
    iteration_id: &str,
    instrument: LyrebirdInstrument,
    sample_path: &str,
    spectrogram_path: &str,
) -> Result<(), String> {
    generate_mel_spectrogram(sample_path, spectrogram_path).map_err(|err| {
        warn!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            error = %err,
            "lyrebird spectrogram generation failed; skipping candidate render"
        );
        err
    })?;

    info!(
        iteration_id = %iteration_id,
        instrument = instrument.slug(),
        sample_path,
        spectrogram_path,
        "generated lyrebird candidate mel"
    );
    Ok(())
}

fn compare_candidate_mel(
    iteration_id: &str,
    instrument: LyrebirdInstrument,
    sample_path: &str,
    spectrogram_path: &str,
    target_spectrogram_path: &str,
    target_audio_metrics: LyrebirdAudioMetrics,
) -> Result<ScoredAudioSample, String> {
    let scored_sample = score_rendered_sample(
        sample_path,
        spectrogram_path,
        target_spectrogram_path,
        target_audio_metrics,
    )?;
    info!(
        iteration_id = %iteration_id,
        instrument = instrument.slug(),
        score = scored_sample.score,
        mel_similarity = scored_sample.mel_similarity,
        sample_path,
        spectrogram_path,
        "rendered and scored lyrebird candidate"
    );
    Ok(scored_sample)
}

fn candidate_output_paths(
    base_sample_path: &str,
    base_spectrogram_path: &str,
    instrument_parallelism: usize,
    candidate_index: usize,
) -> (String, String) {
    if instrument_parallelism <= 1 {
        return (
            base_sample_path.to_owned(),
            base_spectrogram_path.to_owned(),
        );
    }

    (
        append_candidate_suffix(base_sample_path, candidate_index),
        append_candidate_suffix(base_spectrogram_path, candidate_index),
    )
}

fn append_candidate_suffix(path: &str, candidate_index: usize) -> String {
    let candidate_number = candidate_index.saturating_add(1);
    let path = Path::new(path);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("candidate");
    let suffix = format!("{stem}-p{candidate_number}");
    let file_name = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => format!("{suffix}.{extension}"),
        None => suffix,
    };
    path.with_file_name(file_name).display().to_string()
}

fn instrument_score_specs(instrument: LyrebirdInstrument) -> Vec<String> {
    instrument
        .score_specs()
        .iter()
        .map(|score_spec| (*score_spec).to_owned())
        .collect()
}

fn format_score_specs(instrument: LyrebirdInstrument) -> String {
    instrument
        .score_specs()
        .iter()
        .map(|score_spec| format!("- {score_spec}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_retry_reasons(retry_reasons: &[String]) -> Option<String> {
    if retry_reasons.is_empty() {
        return None;
    }

    let mut unique_reasons = Vec::new();
    for retry_reason in retry_reasons {
        if !unique_reasons
            .iter()
            .any(|existing| existing == retry_reason)
        {
            unique_reasons.push(retry_reason.clone());
        }
    }

    Some(truncate_retry_reason(&unique_reasons.join("\n\n")))
}

fn build_optimization_prompt(input: BuildOptimizationPromptInput) -> Result<Prompt, String> {
    let selected_code = input
        .code_branch
        .last()
        .cloned()
        .ok_or_else(|| "lyrebird prompt requires a selected code branch".to_owned())?;
    if selected_code.mel_spectrogram_path.is_empty() {
        return Err("selected lyrebird branch is missing a spectrogram path".to_owned());
    }

    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt.saturating_add(1),
        selected_depth = input.code_branch.len().saturating_sub(1),
        selected_score = selected_code.score().unwrap_or_default(),
        selected_mel_similarity = selected_code.mel_similarity().unwrap_or_default(),
        "building lyrebird optimization prompt"
    );

    let target_score_specs = format_score_specs(input.instrument);
    let mut contents = Vec::new();
    let mut system_text = format!(
        "You are an experienced software engineer and an expert in digital signal processing.\n\
Produce the next iterative patch for `{}` so the generated {} audio moves closer to the target.\n\
The patch must be valid Rust, must still compile inside `lyrebird-sample`, and must be a localized search/replace edit rather than a full-module rewrite.\n\
Iteration id: {}.\nPrompt attempt: {}.\nSelected branch depth: {}.\n\
Target score spec(s):\n{}.\nUse `{}` exactly once with `search`, `replacement`, and `note`.\n\
The `note` must briefly explain the purpose of the change and stay within 100 characters.",
        input.instrument.relative_dsp_path(),
        input.instrument.render_subject(),
        input.iteration_id,
        input.prompt_attempt.saturating_add(1),
        input.code_branch.len().saturating_sub(1),
        target_score_specs,
        input.instrument.tool_name()
    );
    if let Some(retry_reason) = input.retry_reason {
        system_text.push_str(
            "\n\nPrevious attempt failed compilation and the original DSP source was restored.\nFailure details:\n",
        );
        system_text.push_str(&retry_reason);
    }
    contents.push(crate::tokens::Content::Text(system_text));

    let initial_code = &input.code_branch[0].code;
    contents.push(crate::tokens::Content::Text(format!(
        "Initial code:\n```rust\n{}\n```",
        initial_code.source
    )));
    contents.push(crate::tokens::Content::Text(format!(
        "Target sample metrics:\n{}",
        format_target_audio_metrics(input.target_audio_metrics)
    )));

    let mut patch_history = String::from("Patch history:\n");
    if input.code_branch.len() == 1 {
        patch_history.push_str("No patches have been applied yet.");
    } else {
        for (index, window) in input.code_branch.windows(2).enumerate() {
            let current = &window[1];
            let patch = current.patch.as_ref().ok_or_else(|| {
                format!(
                    "lyrebird branch node {} is missing required patch metadata",
                    current.code.iteration_id
                )
            })?;
            patch_history.push_str(&format!(
                "\nPatch {}.\nIteration id: {}.\nNote: {}\n{}\n{}\n",
                index + 1,
                current.code.iteration_id,
                patch.note,
                format_code_analysis("After patch", &current.code),
                format_search_replace_block(patch),
            ));
        }
    }
    contents.push(crate::tokens::Content::Text(patch_history));
    contents.push(crate::tokens::Content::Text(format!(
        "Current sample analysis:\n{}",
        format_code_analysis("Current sample", &selected_code.code)
    )));

    contents.push(crate::tokens::Content::Text(format!(
        "Current code:\n```rust\n{}\n```",
        selected_code.code.source
    )));
    contents.push(crate::tokens::Content::Text(
        "Current mel spectrogram:".to_owned(),
    ));
    contents.push(crate::tokens::Content::Image(PathBuf::from(
        &selected_code.mel_spectrogram_path,
    )));
    contents.push(crate::tokens::Content::Text(format!(
        "Return exactly one `{}` tool call.",
        input.instrument.tool_name(),
    )));

    Ok(Prompt {
        messages: vec![
            crate::tokens::Message {
                role: "system".to_owned(),
                contents,
            },
            crate::tokens::Message {
                role: "user".to_owned(),
                contents: vec![
                    crate::tokens::Content::Text(format!(
                        "This {} still sounds way different from the target! Produce the next patch to close the gap. Here's the mel spectrogram of the target for reference:",
                        input.instrument.display_name()
                    )),
                    crate::tokens::Content::Image(PathBuf::from(
                        &input.target_spectrogram_path,
                    )),
                ],
            },
        ],
        // Keep tool definitions out of the prompt payload because `Tool.parameters`
        // uses `serde_json::Value`, which the worker's postcard transport cannot
        // deserialize. The runtime registers the replace tools statically instead.
        tools: Vec::new(),
    })
}

async fn prepare_tool_calls(
    input: PrepareToolCallsInput,
) -> Result<PrepareToolCallsOutcome, String> {
    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt,
        tool_call_count = input.tool_calls.len(),
        "preparing lyrebird dsp tool calls"
    );

    let replacement = match extract_replacement_source(&input.tool_calls, &input.tool_name) {
        Ok(Some(patch)) => patch,
        Err(err) => {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                error = %err,
                "lyrebird tool call arguments were malformed"
            );
            return Ok(PrepareToolCallsOutcome {
                retry_reason: Some(err),
                generated_patch: None,
                generated_source: None,
            });
        }
        Ok(None) => {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                "no valid lyrebird dsp replacement tool call returned"
            );
            return Ok(PrepareToolCallsOutcome {
                retry_reason: Some(format!(
                    "no valid `{}` tool call with `search`, `replacement`, and `note` strings was returned",
                    input.tool_name
                )),
                generated_patch: None,
                generated_source: None,
            });
        }
    };

    let generated_source = match apply_search_replace_patch(&input.current_source, &replacement) {
        Ok(source) => source,
        Err(err) => {
            return Ok(PrepareToolCallsOutcome {
                retry_reason: Some(err),
                generated_patch: None,
                generated_source: None,
            });
        }
    };

    debug!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt,
        "prepared candidate lyrebird dsp source"
    );

    Ok(PrepareToolCallsOutcome {
        retry_reason: None,
        generated_patch: Some(replacement),
        generated_source: Some(generated_source),
    })
}

async fn compile_prepared_patch(
    input: CompilePreparedPatchInput,
) -> Result<CompilePreparedPatchOutcome, String> {
    let Some(generated_source) = input.generated_source.as_deref() else {
        return Ok(CompilePreparedPatchOutcome {
            compile_ok: false,
            retry_reason: None,
        });
    };

    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt,
        "checking lyrebird dsp patch compilation"
    );

    match with_temporary_dsp_source(
        &input.dsp_source_path,
        generated_source,
        &input.original_source,
        || async { check_sampler_compilation().await },
    )
    .await
    {
        Ok(()) => Ok(CompilePreparedPatchOutcome {
            compile_ok: true,
            retry_reason: None,
        }),
        Err(err) => {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                error = %err,
                "lyrebird sample compilation failed; restored original dsp source"
            );
            Ok(CompilePreparedPatchOutcome {
                compile_ok: false,
                retry_reason: Some(err),
            })
        }
    }
}

fn extract_replacement_source(
    tool_calls: &[ToolCall],
    expected_tool_name: &str,
) -> Result<Option<LyrebirdPatch>, String> {
    let Some(tool_call) = tool_calls
        .iter()
        .rev()
        .find(|tool_call| tool_call.name == expected_tool_name)
    else {
        return Ok(None);
    };

    let arguments = tool_call.arguments_json_value().map_err(|err| {
        format!(
            "malformed `{}` tool call arguments: {}",
            expected_tool_name,
            truncate_retry_reason(&err.to_string())
        )
    })?;

    let search = arguments
        .get("search")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let replacement = arguments
        .get("replacement")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let note = arguments
        .get("note")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);

    match (search, replacement, note) {
        (Some(search), Some(replacement), Some(note)) => {
            validate_patch_note(&note)?;
            if search.is_empty() {
                return Err(format!(
                    "`{expected_tool_name}` tool call `search` must not be empty"
                ));
            }
            Ok(Some(LyrebirdPatch {
                search,
                replacement,
                note,
            }))
        }
        _ => Ok(None),
    }
}

fn validate_patch_note(note: &str) -> Result<(), String> {
    let trimmed = note.trim();
    if trimmed.is_empty() {
        return Err("tool call `note` must not be empty".to_owned());
    }
    if trimmed.chars().count() > 100 {
        return Err("tool call `note` must be at most 100 characters".to_owned());
    }
    Ok(())
}

fn apply_search_replace_patch(
    current_source: &str,
    patch: &LyrebirdPatch,
) -> Result<String, String> {
    let matches = current_source
        .match_indices(&patch.search)
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err("tool call `search` did not match the current code".to_owned()),
        1 => {
            let (start, matched) = matches[0];
            let end = start + matched.len();
            let mut updated =
                String::with_capacity(current_source.len() - matched.len() + patch.replacement.len());
            updated.push_str(&current_source[..start]);
            updated.push_str(&patch.replacement);
            updated.push_str(&current_source[end..]);
            Ok(updated)
        }
        count => Err(format!(
            "tool call `search` matched {count} locations in the current code; make it more specific"
        )),
    }
}

fn format_search_replace_block(patch: &LyrebirdPatch) -> String {
    format!(
        "```text\n<<<<<<< SEARCH\n{}\n=======\n{}\n>>>>>>> REPLACE\n```",
        patch.search, patch.replacement
    )
}

fn format_target_audio_metrics(metrics: LyrebirdAudioMetrics) -> String {
    [
        format_metric_line("zero-crossing rate", metrics.zero_crossing_rate, None),
        format_metric_line("crest factor", metrics.crest_factor, None),
        format_metric_line("spectral centroid", metrics.spectral_centroid, Some("Hz")),
        format_metric_line("spectral flatness", metrics.spectral_flatness, None),
        format_metric_line("spectral roll-off", metrics.spectral_rolloff, Some("Hz")),
    ]
    .join("\n")
}

fn format_code_analysis(label: &str, code: &DspCode) -> String {
    let mut lines = vec![
        format!("{label} score: {}", format_optional_scalar(code.score())),
        format!(
            "{label} mel similarity: {}",
            format_optional_scalar(code.mel_similarity())
        ),
    ];

    if let Some(metrics) = code.audio_metrics {
        let errors = code.audio_metric_errors;
        lines.push(format_metric_with_error(
            "zero-crossing rate",
            metrics.zero_crossing_rate,
            errors.map(|value| value.zero_crossing_rate),
            None,
        ));
        lines.push(format_metric_with_error(
            "crest factor",
            metrics.crest_factor,
            errors.map(|value| value.crest_factor),
            None,
        ));
        lines.push(format_metric_with_error(
            "spectral centroid",
            metrics.spectral_centroid,
            errors.map(|value| value.spectral_centroid),
            Some("Hz"),
        ));
        lines.push(format_metric_with_error(
            "spectral flatness",
            metrics.spectral_flatness,
            errors.map(|value| value.spectral_flatness),
            None,
        ));
        lines.push(format_metric_with_error(
            "spectral roll-off",
            metrics.spectral_rolloff,
            errors.map(|value| value.spectral_rolloff),
            Some("Hz"),
        ));
    }

    lines.join("\n")
}

fn format_metric_with_error(
    name: &str,
    value: f32,
    relative_error: Option<f32>,
    unit: Option<&str>,
) -> String {
    match relative_error {
        Some(relative_error) => match unit {
            Some(unit) => format!(
                "{name}: {} {unit} (relative error {})",
                format_scalar(value),
                format_scalar(relative_error),
            ),
            None => format!(
                "{name}: {} (relative error {})",
                format_scalar(value),
                format_scalar(relative_error),
            ),
        },
        None => format_metric_line(name, value, unit),
    }
}

fn format_metric_line(name: &str, value: f32, unit: Option<&str>) -> String {
    match unit {
        Some(unit) => format!("{name}: {} {unit}", format_scalar(value)),
        None => format!("{name}: {}", format_scalar(value)),
    }
}

fn format_optional_scalar(value: Option<f32>) -> String {
    value.map(format_scalar).unwrap_or_else(|| "n/a".to_owned())
}

fn format_scalar(value: f32) -> String {
    format!("{value:.6}")
}

async fn check_sampler_compilation() -> Result<(), String> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(SAMPLER_MANIFEST_PATH)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|err| format!("failed to spawn cargo check for lyrebird-sample: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };

    Err(format!(
        "lyrebird-sample compilation failed:\n{}",
        truncate_retry_reason(details)
    ))
}

async fn with_temporary_dsp_source<T, F, Fut>(
    dsp_source_path: &str,
    generated_source: &str,
    original_source: &str,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    fs::write(dsp_source_path, generated_source).map_err(|err| {
        format!(
            "failed to write replacement dsp source {}: {err}",
            dsp_source_path
        )
    })?;

    let operation_result = operation().await;
    let restore_result = fs::write(dsp_source_path, original_source).map_err(|err| {
        format!(
            "failed to restore original dsp source {}: {err}",
            dsp_source_path
        )
    });

    match (operation_result, restore_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(err), Ok(())) => Err(err),
        (Ok(_), Err(restore_err)) => Err(restore_err),
        (Err(err), Err(restore_err)) => Err(format!(
            "{err}\nrestoring the original dsp source also failed: {restore_err}"
        )),
    }
}

async fn run_sampler(
    duration_secs: f64,
    output_path: &str,
    score_specs: &[String],
) -> Result<(), String> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err("duration seconds must be a finite value > 0".to_string());
    }
    if score_specs.is_empty() {
        return Err("run sampler requires at least one score spec".to_string());
    }

    ensure_parent_dir(Path::new(output_path))?;
    debug!(
        output_path,
        duration_secs,
        score_spec_count = score_specs.len(),
        "running lyrebird sampler"
    );

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("--manifest-path")
        .arg(SAMPLER_MANIFEST_PATH)
        .arg("--")
        .arg("--duration-secs")
        .arg(duration_secs.to_string())
        .arg("--output-path")
        .arg(output_path)
        .args(score_specs)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to spawn cargo sampler run: {err}"))?;

    match timeout(SAMPLER_COMMAND_TIMEOUT, child.wait()).await {
        Ok(wait_result) => {
            let status = wait_result
                .map_err(|err| format!("failed to wait for cargo sampler run: {err}"))?;
            if status.success() {
                debug!(output_path, "lyrebird sampler run finished successfully");
                Ok(())
            } else {
                Err(format!("cargo sampler run exited unsuccessfully: {status}"))
            }
        }
        Err(_) => {
            child
                .kill()
                .await
                .map_err(|err| format!("timed out and failed to kill cargo sampler run: {err}"))?;
            let _ = child.wait().await;
            Err("cargo sampler run timed out".to_string())
        }
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create directory {}: {err}", parent.display()))?;
    }
    Ok(())
}

fn truncate_retry_reason(text: &str) -> String {
    const MAX_LEN: usize = 4_000;
    if text.len() <= MAX_LEN {
        return text.to_owned();
    }
    format!("{}...", &text[..MAX_LEN])
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use uuid::Uuid;

    fn temp_png_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-lyrebird-tests")
            .join(format!("{name}-{}.png", Uuid::new_v4()))
    }

    fn write_rgb_image(path: &Path, width: u32, height: u32, value: [u8; 3]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let image = RgbImage::from_fn(width, height, |_x, _y| Rgb(value));
        image.save(path).unwrap();
    }

    fn temp_text_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-lyrebird-tests")
            .join(format!("{name}-{}.rs", Uuid::new_v4()))
    }

    fn target_metrics() -> LyrebirdAudioMetrics {
        LyrebirdAudioMetrics {
            zero_crossing_rate: 0.125,
            crest_factor: 2.5,
            spectral_centroid: 1_250.0,
            spectral_flatness: 0.2,
            spectral_rolloff: 3_200.0,
        }
    }

    fn score_code(
        iteration_id: &str,
        source: &str,
        sample_path: &str,
        spectrogram_path: &str,
        mel_similarity: f32,
        score: f32,
    ) -> DspCode {
        DspCode {
            iteration_id: iteration_id.to_owned(),
            source: source.to_owned(),
            sample_path: sample_path.to_owned(),
            spectrogram_path: spectrogram_path.to_owned(),
            mel_similarity: Some(mel_similarity),
            score: Some(score),
            audio_metrics: Some(LyrebirdAudioMetrics {
                zero_crossing_rate: 0.15,
                crest_factor: 2.2,
                spectral_centroid: 1_100.0,
                spectral_flatness: 0.18,
                spectral_rolloff: 3_050.0,
            }),
            audio_metric_errors: Some(LyrebirdAudioMetricErrors {
                zero_crossing_rate: 0.10,
                crest_factor: 0.12,
                spectral_centroid: 0.08,
                spectral_flatness: 0.10,
                spectral_rolloff: 0.05,
            }),
        }
    }

    #[test]
    fn compare_spectrograms_crops_extra_width_from_the_right() {
        let left_path = temp_png_path("left");
        let right_path = temp_png_path("right");
        if let Some(parent) = left_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let left = RgbImage::from_fn(16, 8, |x, _y| {
            if x < 8 {
                Rgb([0, 0, 0])
            } else {
                Rgb([255, 255, 255])
            }
        });
        left.save(&left_path).unwrap();
        write_rgb_image(&right_path, 8, 8, [0, 0, 0]);

        let similarity = compare_spectrograms(
            &left_path.display().to_string(),
            &right_path.display().to_string(),
        )
        .unwrap();

        assert!(similarity > 0.99);
    }

    #[test]
    fn compare_spectrograms_crops_target_when_it_is_wider() {
        let left_path = temp_png_path("left-narrow");
        let right_path = temp_png_path("right-wide");
        write_rgb_image(&left_path, 8, 8, [0, 0, 0]);
        if let Some(parent) = right_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let right = RgbImage::from_fn(16, 8, |x, _y| {
            if x < 8 {
                Rgb([0, 0, 0])
            } else {
                Rgb([255, 255, 255])
            }
        });
        right.save(&right_path).unwrap();

        let similarity = compare_spectrograms(
            &left_path.display().to_string(),
            &right_path.display().to_string(),
        )
        .unwrap();

        assert!(similarity > 0.99);
    }

    #[test]
    fn normalize_zensim_caps_perfect_scores_at_one() {
        assert_eq!(normalize_zensim(100.0), 1.0);
        assert_eq!(normalize_zensim(120.0), 1.0);
    }

    #[test]
    fn normalize_zensim_soft_clamps_bad_scores_without_going_negative() {
        let very_bad = normalize_zensim(-100.0);
        let poor = normalize_zensim(0.0);
        let midpoint = normalize_zensim(30.0);
        let strong = normalize_zensim(80.0);

        assert!((0.0..1.0).contains(&very_bad));
        assert!((very_bad..1.0).contains(&poor));
        assert!((0.49..0.51).contains(&midpoint));
        assert!((midpoint..1.0).contains(&strong));
    }

    #[test]
    fn extract_replacement_source_uses_expected_tool_name() {
        let tool_calls = vec![
            ToolCall {
                id: None,
                name: "replace_vocals_formant_dsp".to_owned(),
                arguments:
                    "{\"search\":\"vocals\",\"replacement\":\"vox\",\"note\":\"voice tweak\"}"
                        .to_owned(),
            },
            ToolCall {
                id: None,
                name: "replace_rhythm_guitar_dsp".to_owned(),
                arguments:
                    "{\"search\":\"rhythm\",\"replacement\":\"chug\",\"note\":\"tighten attack\"}"
                        .to_owned(),
            },
        ];

        assert_eq!(
            extract_replacement_source(&tool_calls, "replace_rhythm_guitar_dsp").unwrap(),
            Some(LyrebirdPatch {
                search: "rhythm".to_owned(),
                replacement: "chug".to_owned(),
                note: "tighten attack".to_owned(),
            })
        );
        assert_eq!(
            extract_replacement_source(&tool_calls, "replace_backup_vocals_dsp").unwrap(),
            None
        );
    }

    #[test]
    fn extract_replacement_source_reports_malformed_expected_tool_arguments() {
        let tool_calls = vec![ToolCall {
            id: None,
            name: "replace_rhythm_guitar_dsp".to_owned(),
            arguments: "{\"search\":\"unterminated".to_owned(),
        }];

        let err = extract_replacement_source(&tool_calls, "replace_rhythm_guitar_dsp").unwrap_err();

        assert!(err.contains("malformed `replace_rhythm_guitar_dsp` tool call arguments"));
        assert!(err.contains("EOF while parsing a string"));
    }

    #[test]
    fn optimization_prompt_keeps_inline_tools_empty_for_postcard_transport() {
        let prompt = build_optimization_prompt(BuildOptimizationPromptInput {
            iteration_id: "00000001".to_owned(),
            instrument: LyrebirdInstrument::Bass,
            target_spectrogram_path: "/tmp/target.png".to_owned(),
            target_audio_metrics: target_metrics(),
            code_branch: vec![score_code(
                "initial",
                "fn bass() {}",
                "/tmp/bass.wav",
                "/tmp/bass.png",
                0.5,
                0.55,
            )
            .into()],
            prompt_attempt: 0,
            retry_reason: None,
        })
        .unwrap();

        assert!(prompt.tools.is_empty());
    }

    #[test]
    fn iteration_elapsed_ms_uses_previous_start_time_when_present() {
        assert_eq!(iteration_elapsed_ms(1_250, Some(1_000)), Some(250));
        assert_eq!(iteration_elapsed_ms(1_000, None), None);
        assert_eq!(iteration_elapsed_ms(900, Some(1_000)), Some(0));
    }

    #[test]
    fn optimization_prompt_uses_patch_history_and_leaf_mel_only() {
        let prompt = build_optimization_prompt(BuildOptimizationPromptInput {
            iteration_id: "00000002".to_owned(),
            instrument: LyrebirdInstrument::Bass,
            target_spectrogram_path: "/tmp/target.png".to_owned(),
            target_audio_metrics: target_metrics(),
            code_branch: vec![
                score_code(
                    "initial",
                    "fn bass() { baseline(); }",
                    "/tmp/initial.wav",
                    "/tmp/initial.png",
                    0.25,
                    0.30,
                )
                .into(),
                LyrebirdBranchNode::from_generated(
                    score_code(
                        "00000001",
                        "fn bass() { mutate(); }",
                        "/tmp/00000001.wav",
                        "/tmp/00000001.png",
                        0.75,
                        0.82,
                    ),
                    LyrebirdPatch {
                        search: "baseline();".to_owned(),
                        replacement: "mutate();".to_owned(),
                        note: "bass resonance tweak".to_owned(),
                    },
                ),
            ],
            prompt_attempt: 1,
            retry_reason: None,
        })
        .unwrap();

        let system_contents = &prompt.messages[0].contents;
        assert_eq!(
            system_contents[1],
            crate::tokens::Content::Text(
                "Initial code:\n```rust\nfn bass() { baseline(); }\n```".to_owned()
            )
        );
        if let crate::tokens::Content::Text(target_metrics_text) = &system_contents[2] {
            assert!(target_metrics_text.contains("Target sample metrics:"));
            assert!(target_metrics_text.contains("zero-crossing rate: 0.125000"));
            assert!(target_metrics_text.contains("spectral roll-off: 3200.000000 Hz"));
        } else {
            panic!("expected target metrics text");
        }
        assert!(system_contents.contains(&crate::tokens::Content::Text(
            "Current mel spectrogram:".to_owned()
        )));
        assert!(
            system_contents.contains(&crate::tokens::Content::Image(PathBuf::from(
                "/tmp/00000001.png"
            )))
        );
        if let crate::tokens::Content::Text(patch_history) = &system_contents[3] {
            assert!(patch_history.contains("Patch history:"));
            assert!(patch_history.contains("No patches have been applied yet") == false);
            assert!(patch_history.contains("bass resonance tweak"));
            assert!(patch_history.contains("After patch score: 0.820000"));
            assert!(patch_history.contains("After patch mel similarity: 0.750000"));
            assert!(
                patch_history.contains("zero-crossing rate: 0.150000 (relative error 0.100000)")
            );
            assert!(patch_history.contains("<<<<<<< SEARCH"));
        } else {
            panic!("expected patch history text");
        }
        if let crate::tokens::Content::Text(current_analysis) = &system_contents[4] {
            assert!(current_analysis.contains("Current sample analysis:"));
            assert!(current_analysis.contains("Current sample score: 0.820000"));
            assert!(current_analysis.contains("Current sample mel similarity: 0.750000"));
        } else {
            panic!("expected current analysis text");
        }
        assert_eq!(
            prompt.messages[1].contents,
            vec![
                crate::tokens::Content::Text(
                    "This Bass still sounds way different from the target! Produce the next patch to close the gap. Here's the mel spectrogram of the target for reference:".to_owned()
                ),
                crate::tokens::Content::Image(PathBuf::from("/tmp/target.png"))
            ]
        );
    }

    #[test]
    fn optimization_prompt_rejects_non_initial_nodes_without_patch_metadata() {
        let err = build_optimization_prompt(BuildOptimizationPromptInput {
            iteration_id: "00000002".to_owned(),
            instrument: LyrebirdInstrument::Bass,
            target_spectrogram_path: "/tmp/target.png".to_owned(),
            target_audio_metrics: target_metrics(),
            code_branch: vec![
                score_code(
                    "initial",
                    "fn bass() { baseline(); }",
                    "/tmp/initial.wav",
                    "/tmp/initial.png",
                    0.25,
                    0.30,
                )
                .into(),
                score_code(
                    "00000001",
                    "fn bass() { mutate(); }",
                    "/tmp/00000001.wav",
                    "/tmp/00000001.png",
                    0.75,
                    0.82,
                )
                .into(),
            ],
            prompt_attempt: 1,
            retry_reason: None,
        })
        .unwrap_err();

        assert!(err.contains("missing required patch metadata"));
    }

    #[test]
    fn apply_search_replace_patch_requires_exactly_one_match() {
        let patch = LyrebirdPatch {
            search: "foo".to_owned(),
            replacement: "bar".to_owned(),
            note: "swap token".to_owned(),
        };

        assert_eq!(
            apply_search_replace_patch("fn foo() {}", &patch).unwrap(),
            "fn bar() {}"
        );
        assert!(apply_search_replace_patch("fn baz() {}", &patch)
            .unwrap_err()
            .contains("did not match"));
        assert!(apply_search_replace_patch("foo foo", &patch)
            .unwrap_err()
            .contains("matched 2 locations"));
    }

    #[tokio::test]
    async fn temporary_dsp_source_restores_original_after_success() {
        let path = temp_text_path("restore-success");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, "original").unwrap();
        let path_string = path.display().to_string();

        let result = with_temporary_dsp_source(&path_string, "generated", "original", || async {
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "generated");
            Ok::<_, String>("ok")
        })
        .await
        .unwrap();

        assert_eq!(result, "ok");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    }

    #[tokio::test]
    async fn temporary_dsp_source_restores_original_after_failure() {
        let path = temp_text_path("restore-failure");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, "original").unwrap();
        let path_string = path.display().to_string();

        let err = with_temporary_dsp_source(&path_string, "generated", "original", || async {
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "generated");
            Err::<(), _>("boom".to_owned())
        })
        .await
        .unwrap_err();

        assert!(err.contains("boom"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    }
}
