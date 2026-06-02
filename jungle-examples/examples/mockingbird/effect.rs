#![allow(dead_code)]

use crate::tokens::{Prompt, TokenPredictor, ToolCall};
use crate::{DspCode, MockingBirdInstrument, PulseCodeParadise, MOCKINGBIRD_DURATION_SECS};
use image::ImageReader;
use image_compare::Algorithm;
use jungle_sdk::effect;
use serde::{Deserialize, Serialize};
use spectrs::io::audio::read_audio_file_mono;
use spectrs::io::image::{save_spectrogram_image, Colormap};
use spectrs::spectrogram::mel::{par_convert_to_mel, MelScale};
use spectrs::spectrogram::stft::{par_compute_spectrogram, SpectrogramType};
use std::fs;
use std::future::{ready, Future};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration, Instant};
use tracing::{debug, info, warn};

fn stub_ok<T>(value: T) -> impl Future<Output = Result<T, String>> {
    ready(Ok(value))
}

const MEL_N_FFT: usize = 2048;
const MEL_HOP_LENGTH: usize = 512;
const MEL_WIN_LENGTH: usize = 2048;
const MEL_N_MELS: usize = 256;
const MEL_F_MIN_HZ: f32 = 20.0;
const MEL_F_MAX_HZ: f32 = 16_000.0;
const SAMPLER_MANIFEST_PATH: &str = "jungle-examples/examples/mockingbird/sample/Cargo.toml";
const SAMPLER_COMMAND_TIMEOUT: Duration = Duration::from_secs(180);
const SAMPLER_BINARY_PATH: &str = "./target/release/mockingbird-sample";

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

pub struct PromptModel;
#[effect(id = 5)]
impl Effect<()> for PromptModel {
    type In = Prompt;
    type Out = Vec<ToolCall>;
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("PromptModel requires PulseCodeParadise runtime context".to_owned()) }
    }
}

#[effect(id = 5)]
impl Effect<PulseCodeParadise> for PromptModel {
    type In = Prompt;
    type Out = Vec<ToolCall>;
    type Err = String;

    fn effect(
        jungle: &PulseCodeParadise,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let prompt_started_at = Instant::now();
            let response = jungle.predict(input).await.map_err(|err| err.to_string());
            let prompt_elapsed_ms = prompt_started_at.elapsed().as_millis();
            match &response {
                Ok(tool_calls) => info!(
                    prompt_elapsed_ms,
                    tool_call_count = tool_calls.len(),
                    "received prompt model response"
                ),
                Err(error) => info!(prompt_elapsed_ms, error, "prompt model request failed"),
            }
            response
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildOptimizationPromptInput {
    pub iteration_id: String,
    pub instrument: MockingBirdInstrument,
    pub code_branch: Vec<DspCode>,
    pub target_spectrogram_path: String,
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
pub struct ApplyToolCallsInput {
    pub iteration_id: String,
    pub instrument: MockingBirdInstrument,
    pub prompt_attempt: u32,
    pub tool_name: String,
    pub dsp_source_path: String,
    pub base_source: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyToolCallsOutcome {
    pub compile_ok: bool,
    pub retry_reason: Option<String>,
    pub generated_source: Option<String>,
}

pub struct ApplyToolCalls;
#[effect(id = 13)]
impl<J> Effect<J> for ApplyToolCalls {
    type In = ApplyToolCallsInput;
    type Out = ApplyToolCallsOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { apply_tool_calls(input).await }
    }
}

pub struct SearchTreeSelect;
#[effect(id = 9)]
impl Effect<()> for SearchTreeSelect {
    type In = MockingBirdInstrument;
    type Out = Vec<DspCode>;
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSelect requires PulseCodeParadise runtime context".to_owned()) }
    }
}

#[effect(id = 9)]
impl Effect<PulseCodeParadise> for SearchTreeSelect {
    type In = MockingBirdInstrument;
    type Out = Vec<DspCode>;
    type Err = String;

    fn effect(
        jungle: &PulseCodeParadise,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            jungle
                .select_mockingbird_branch(input)
                .map_err(|err| err.to_string())
        }
    }
}

pub struct SearchTreeSubmit;
#[effect(id = 10)]
impl Effect<()> for SearchTreeSubmit {
    type In = (MockingBirdInstrument, Vec<DspCode>, f32);
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSubmit requires PulseCodeParadise runtime context".to_owned()) }
    }
}

#[effect(id = 10)]
impl Effect<PulseCodeParadise> for SearchTreeSubmit {
    type In = (MockingBirdInstrument, Vec<DspCode>, f32);
    type Out = ();
    type Err = String;

    fn effect(
        jungle: &PulseCodeParadise,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let (instrument, data, score) = input;
            jungle
                .submit_mockingbird_branch(instrument, data, score)
                .map_err(|err| err.to_string())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationInstrumentInput {
    pub instrument: MockingBirdInstrument,
    pub sample_path: String,
    pub spectrogram_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationInstrumentOutput {
    pub instrument: MockingBirdInstrument,
    pub sample_path: String,
    pub spectrogram_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationSamplesInput {
    pub iteration_id: String,
    pub instruments: Vec<FinalizeIterationInstrumentInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationSamplesOutcome {
    pub rendered: Vec<FinalizeIterationInstrumentOutput>,
}

pub struct FinalizeIterationSamples;
#[effect(id = 11)]
impl<J> Effect<J> for FinalizeIterationSamples {
    type In = FinalizeIterationSamplesInput;
    type Out = FinalizeIterationSamplesOutcome;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { finalize_iteration_samples(input).await }
    }
}

pub async fn capture_current_dsp_code_snapshot(
    iteration_id: &str,
    output_root: &Path,
    instrument: MockingBirdInstrument,
    target_spectrogram_path: &Path,
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
        MOCKINGBIRD_DURATION_SECS,
        &sample_path.display().to_string(),
        &[instrument.score_spec().to_owned()],
    )
    .await?;
    generate_mel_spectrogram(
        &sample_path.display().to_string(),
        &spectrogram_path.display().to_string(),
    )?;
    let similarity = compare_spectrograms(
        &spectrogram_path.display().to_string(),
        &target_spectrogram_path.display().to_string(),
    )?;
    info!(
        iteration_id,
        instrument = instrument.slug(),
        similarity,
        sample_path = %sample_path.display(),
        spectrogram_path = %spectrogram_path.display(),
        "captured mockingbird dsp snapshot"
    );

    Ok(DspCode {
        iteration_id: iteration_id.to_owned(),
        source,
        sample_path: sample_path.display().to_string(),
        spectrogram_path: spectrogram_path.display().to_string(),
        similarity: Some(similarity),
    })
}

fn generate_mel_spectrogram(wav_path: &str, output_path: &str) -> Result<(), String> {
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

fn compare_spectrograms(left_path: &str, right_path: &str) -> Result<f32, String> {
    let left = ImageReader::open(left_path)
        .map_err(|err| format!("failed to open spectrogram image {}: {err}", left_path))?
        .decode()
        .map_err(|err| format!("failed to decode spectrogram image {}: {err}", left_path))?
        .into_luma8();
    let right = ImageReader::open(right_path)
        .map_err(|err| format!("failed to open spectrogram image {}: {err}", right_path))?
        .decode()
        .map_err(|err| format!("failed to decode spectrogram image {}: {err}", right_path))?
        .into_luma8();
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

    let similarity =
        image_compare::gray_similarity_structure(&Algorithm::MSSIMSimple, &left, &right)
            .map_err(|err| format!("failed to compare spectrograms: {err}"))?;

    Ok(similarity.score as f32)
}

async fn finalize_iteration_samples(
    input: FinalizeIterationSamplesInput,
) -> Result<FinalizeIterationSamplesOutcome, String> {
    build_sampler_release().await?;

    let mut rendered = Vec::with_capacity(input.instruments.len());
    for instrument in input.instruments {
        run_sampler_binary(
            MOCKINGBIRD_DURATION_SECS,
            &instrument.sample_path,
            &[instrument.instrument.score_spec().to_owned()],
        )
        .await?;
        generate_mel_spectrogram(&instrument.sample_path, &instrument.spectrogram_path)?;
        rendered.push(FinalizeIterationInstrumentOutput {
            instrument: instrument.instrument,
            sample_path: instrument.sample_path,
            spectrogram_path: instrument.spectrogram_path,
        });
    }

    Ok(FinalizeIterationSamplesOutcome { rendered })
}

fn build_optimization_prompt(input: BuildOptimizationPromptInput) -> Result<Prompt, String> {
    let selected_code = input
        .code_branch
        .last()
        .cloned()
        .ok_or_else(|| "mockingbird prompt requires a selected code branch".to_owned())?;
    if selected_code.spectrogram_path.is_empty() {
        return Err("selected mockingbird branch is missing a spectrogram path".to_owned());
    }

    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt.saturating_add(1),
        selected_depth = input.code_branch.len().saturating_sub(1),
        selected_similarity = selected_code.similarity.unwrap_or_default(),
        "building mockingbird optimization prompt"
    );

    let mut contents = Vec::new();
    let mut task_description = format!(
        "Task: optimize `{}` so the generated {} mel spectrogram moves closer to the target.\n\
Iteration id: {}.\nPrompt attempt: {}.\nSelected branch depth: {}.\n\
Target score spec: {}.\nUse the `{}` tool to replace the full Rust source for that file.\n\
Keep the module compiling in the existing `mockingbird-sample` crate and preserve the file's role in the welcome audio pipeline.",
        input.instrument.relative_dsp_path(),
        input.instrument.render_subject(),
        input.iteration_id,
        input.prompt_attempt.saturating_add(1),
        input.code_branch.len().saturating_sub(1),
        input.instrument.score_spec(),
        input.instrument.tool_name()
    );
    if let Some(retry_reason) = input.retry_reason {
        task_description.push_str(
            "\n\nPrevious attempt failed compilation and the selected branch source was restored.\nFailure details:\n",
        );
        task_description.push_str(&retry_reason);
    }
    contents.push(crate::tokens::Content::Text(task_description));

    for (index, code) in input.code_branch.iter().enumerate() {
        let heading = if index == 0 {
            "Initial baseline code"
        } else {
            "Branch code"
        };
        let similarity = code
            .similarity
            .map(|score| format!("{score:.6}"))
            .unwrap_or_else(|| "n/a".to_owned());
        contents.push(crate::tokens::Content::Text(format!(
            "{heading} {index}.\nIteration id: {}.\nSimilarity: {}.\n```rust\n{}\n```",
            code.iteration_id, similarity, code.source
        )));
    }

    contents.push(crate::tokens::Content::Text(
        "Selected node spectrogram:".to_owned(),
    ));
    contents.push(crate::tokens::Content::Image(PathBuf::from(
        &selected_code.spectrogram_path,
    )));
    contents.push(crate::tokens::Content::Text(
        "Target spectrogram:".to_owned(),
    ));
    contents.push(crate::tokens::Content::Image(PathBuf::from(
        input.target_spectrogram_path,
    )));
    contents.push(crate::tokens::Content::Text(format!(
        "Return replacement code by calling `{}` exactly once with the full Rust source for `{}`.",
        input.instrument.tool_name(),
        input.instrument.relative_dsp_path()
    )));

    Ok(Prompt {
        messages: vec![
            crate::tokens::Message {
                role: "system".to_owned(),
                contents: vec![crate::tokens::Content::Text(format!(
                    "You are tuning a Rust DSP implementation against a target mel spectrogram. \
Respond with tool calls only. Only use `{}`.",
                    input.instrument.tool_name()
                ))],
            },
            crate::tokens::Message {
                role: "user".to_owned(),
                contents,
            },
        ],
        // Keep tool definitions out of the prompt payload because `Tool.parameters`
        // uses `serde_json::Value`, which the worker's postcard transport cannot
        // deserialize. The runtime registers the replace tools statically instead.
        tools: Vec::new(),
    })
}

async fn apply_tool_calls(input: ApplyToolCallsInput) -> Result<ApplyToolCallsOutcome, String> {
    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt,
        tool_call_count = input.tool_calls.len(),
        "applying mockingbird dsp tool calls"
    );

    let replacement = match extract_replacement_source(&input.tool_calls, &input.tool_name) {
        Some(source) => source,
        None => {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                "no valid mockingbird dsp replacement tool call returned"
            );
            return Ok(ApplyToolCallsOutcome {
                compile_ok: false,
                retry_reason: Some(format!(
                    "no valid `{}` tool call with a `source` string was returned",
                    input.tool_name
                )),
                generated_source: None,
            });
        }
    };

    fs::write(&input.dsp_source_path, &replacement).map_err(|err| {
        format!(
            "failed to write replacement dsp source {}: {err}",
            input.dsp_source_path
        )
    })?;
    debug!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt,
        dsp_source_path = %input.dsp_source_path,
        "wrote candidate mockingbird dsp source"
    );

    match check_sampler_compilation().await {
        Ok(()) => {
            info!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                "mockingbird sample compilation succeeded"
            );
            Ok(ApplyToolCallsOutcome {
                compile_ok: true,
                retry_reason: None,
                generated_source: Some(replacement),
            })
        }
        Err(err) => {
            fs::write(&input.dsp_source_path, &input.base_source).map_err(|restore_err| {
                format!(
                    "sampler compilation failed and restoring {} also failed: {restore_err}; original error: {err}",
                    input.dsp_source_path
                )
            })?;
            warn!(
                iteration_id = %input.iteration_id,
                instrument = input.instrument.slug(),
                prompt_attempt = input.prompt_attempt,
                error = %err,
                "mockingbird sample compilation failed; restored dsp source"
            );
            Ok(ApplyToolCallsOutcome {
                compile_ok: false,
                retry_reason: Some(err),
                generated_source: None,
            })
        }
    }
}

fn extract_replacement_source(tool_calls: &[ToolCall], expected_tool_name: &str) -> Option<String> {
    tool_calls
        .iter()
        .rev()
        .find(|tool_call| tool_call.name == expected_tool_name)
        .and_then(|tool_call| tool_call.arguments_json_value().ok())
        .and_then(|arguments| {
            arguments
                .get("source")
                .or_else(|| arguments.get("content"))
                .or_else(|| arguments.get("contents"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        })
}

async fn build_sampler_release() -> Result<(), String> {
    if run_sampler_cargo(["build", "--release"]).await? {
        return Ok(());
    }
    Err("cargo build --release for mockingbird-sample exited unsuccessfully".to_owned())
}

async fn check_sampler_compilation() -> Result<(), String> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(SAMPLER_MANIFEST_PATH)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|err| format!("failed to spawn cargo check for mockingbird-sample: {err}"))?;

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
        "mockingbird-sample compilation failed:\n{}",
        truncate_retry_reason(details)
    ))
}

async fn run_sampler_cargo<const N: usize>(args: [&str; N]) -> Result<bool, String> {
    let mut child = Command::new("cargo")
        .args(args)
        .arg("--manifest-path")
        .arg(SAMPLER_MANIFEST_PATH)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to spawn cargo {}: {err}", args.join(" ")))?;

    match timeout(SAMPLER_COMMAND_TIMEOUT, child.wait()).await {
        Ok(wait_result) => {
            let status = wait_result
                .map_err(|err| format!("failed to wait for cargo {}: {err}", args.join(" ")))?;
            Ok(status.success())
        }
        Err(_) => {
            child.kill().await.map_err(|err| {
                format!(
                    "timed out and failed to kill cargo {}: {err}",
                    args.join(" ")
                )
            })?;
            let _ = child.wait().await;
            Ok(false)
        }
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
        "running mockingbird sampler"
    );

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("--release")
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
                debug!(output_path, "mockingbird sampler run finished successfully");
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

async fn run_sampler_binary(
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
        "running mockingbird sampler binary"
    );

    let mut child = Command::new(SAMPLER_BINARY_PATH)
        .arg("--duration-secs")
        .arg(duration_secs.to_string())
        .arg("--output-path")
        .arg(output_path)
        .args(score_specs)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to spawn mockingbird-sample binary: {err}"))?;

    match timeout(SAMPLER_COMMAND_TIMEOUT, child.wait()).await {
        Ok(wait_result) => {
            let status = wait_result
                .map_err(|err| format!("failed to wait for mockingbird-sample binary: {err}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "mockingbird-sample binary exited unsuccessfully: {status}"
                ))
            }
        }
        Err(_) => {
            child.kill().await.map_err(|err| {
                format!("timed out and failed to kill mockingbird-sample binary: {err}")
            })?;
            let _ = child.wait().await;
            Err("mockingbird-sample binary timed out".to_string())
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
    use image::{GrayImage, Luma};
    use uuid::Uuid;

    fn temp_png_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-mockingbird-tests")
            .join(format!("{name}-{}.png", Uuid::new_v4()))
    }

    fn write_gray_image(path: &Path, width: u32, height: u32, value: u8) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let image = GrayImage::from_fn(width, height, |_x, _y| Luma([value]));
        image.save(path).unwrap();
    }

    #[test]
    fn compare_spectrograms_crops_extra_width_from_the_right() {
        let left_path = temp_png_path("left");
        let right_path = temp_png_path("right");
        if let Some(parent) = left_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let left = GrayImage::from_fn(4, 1, |x, _y| if x < 2 { Luma([0]) } else { Luma([255]) });
        left.save(&left_path).unwrap();
        write_gray_image(&right_path, 2, 1, 0);

        let similarity = compare_spectrograms(
            &left_path.display().to_string(),
            &right_path.display().to_string(),
        )
        .unwrap();

        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn compare_spectrograms_crops_target_when_it_is_wider() {
        let left_path = temp_png_path("left-narrow");
        let right_path = temp_png_path("right-wide");
        write_gray_image(&left_path, 2, 1, 0);
        if let Some(parent) = right_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let right = GrayImage::from_fn(4, 1, |x, _y| if x < 2 { Luma([0]) } else { Luma([255]) });
        right.save(&right_path).unwrap();

        let similarity = compare_spectrograms(
            &left_path.display().to_string(),
            &right_path.display().to_string(),
        )
        .unwrap();

        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn extract_replacement_source_uses_expected_tool_name() {
        let tool_calls = vec![
            ToolCall {
                id: None,
                name: "replace_vocals_formant_dsp".to_owned(),
                arguments: "{\"source\":\"vocals\"}".to_owned(),
            },
            ToolCall {
                id: None,
                name: "replace_rhythm_guitar_dsp".to_owned(),
                arguments: "{\"source\":\"rhythm\"}".to_owned(),
            },
        ];

        assert_eq!(
            extract_replacement_source(&tool_calls, "replace_rhythm_guitar_dsp"),
            Some("rhythm".to_owned())
        );
        assert_eq!(
            extract_replacement_source(&tool_calls, "replace_backup_vocals_dsp"),
            None
        );
    }

    #[test]
    fn optimization_prompt_keeps_inline_tools_empty_for_postcard_transport() {
        let prompt = build_optimization_prompt(BuildOptimizationPromptInput {
            iteration_id: "00000001".to_owned(),
            instrument: MockingBirdInstrument::Bass,
            code_branch: vec![DspCode {
                iteration_id: "initial".to_owned(),
                source: "fn bass() {}".to_owned(),
                sample_path: "/tmp/bass.wav".to_owned(),
                spectrogram_path: "/tmp/bass.png".to_owned(),
                similarity: Some(0.5),
            }],
            target_spectrogram_path: "/tmp/target.png".to_owned(),
            prompt_attempt: 0,
            retry_reason: None,
        })
        .unwrap();

        assert!(prompt.tools.is_empty());
    }
}
