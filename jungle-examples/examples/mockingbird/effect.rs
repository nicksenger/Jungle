#![allow(dead_code)]

use crate::mcts::SearchTree;
use crate::tokens::{Prompt, TokenPredictor, ToolCall};
use crate::{
    PulseCodeParadise, MOCKINGBIRD_DSP_TOOL_NAME, MOCKINGBIRD_DURATION_SECS,
    MOCKINGBIRD_SCORE_SPEC, RELATIVE_ELECTRIC_GUITAR_DSP_PATH,
};
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
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
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

pub struct GenSample;
#[effect(id = 2)]
impl<J> Effect<J> for GenSample {
    type In = String;
    type Out = String;
    type Err = String;

    fn effect(
        _jungle: &J,
        output_path: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            run_sampler(
                MOCKINGBIRD_DURATION_SECS,
                &output_path,
                &[MOCKINGBIRD_SCORE_SPEC.to_owned()],
            )
            .await?;
            Ok(output_path)
        }
    }
}

pub struct GenSpectrogram;
#[effect(id = 3)]
impl<J> Effect<J> for GenSpectrogram {
    type In = (String, String);
    type Out = String;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let (wav_path, output_path) = _input;
            generate_mel_spectrogram(&wav_path, &output_path)?;
            Ok(output_path)
        }
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
        async move { jungle.predict(input).await.map_err(|err| err.to_string()) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildOptimizationPromptInput {
    pub iteration_id: String,
    pub generated_spectrogram_path: String,
    pub target_spectrogram_path: String,
    pub current_similarity: f32,
    pub prompt_attempt: u32,
    pub retry_reason: Option<String>,
    pub dsp_source_path: String,
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
    pub prompt_attempt: u32,
    pub dsp_source_path: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyToolCallsOutcome {
    pub compile_ok: bool,
    pub retry_reason: Option<String>,
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

pub struct WriteFile;
#[effect(id = 6)]
impl<J> Effect<J> for WriteFile {
    type In = (String, Vec<u8>);
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}

pub struct CompileSampler;
#[effect(id = 7)]
impl<J> Effect<J> for CompileSampler {
    type In = ();
    type Out = bool;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        run_sampler_cargo(["build", "--release"])
    }
}

pub struct CheckSampler;
#[effect(id = 8)]
impl<J> Effect<J> for CheckSampler {
    type In = ();
    type Out = bool;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        run_sampler_cargo(["check"])
    }
}

pub struct SearchTreeSelect<Tag>(PhantomData<fn() -> Tag>);
#[effect(id = 9)]
impl<J, Tag> Effect<J> for SearchTreeSelect<Tag>
where
    J: SearchTree<Tag> + Sync,
    <J as SearchTree<Tag>>::Data: Send + 'static,
    <J as SearchTree<Tag>>::Error: Send + 'static,
{
    type In = ();
    type Out = <J as SearchTree<Tag>>::Data;
    type Err = <J as SearchTree<Tag>>::Error;

    fn effect(jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { <J as SearchTree<Tag>>::select(jungle).await }
    }
}

pub struct SearchTreeSubmit<Tag>(PhantomData<fn() -> Tag>);
#[effect(id = 10)]
impl<J, Tag> Effect<J> for SearchTreeSubmit<Tag>
where
    J: SearchTree<Tag> + Sync,
    <J as SearchTree<Tag>>::Data: Send + 'static,
    <J as SearchTree<Tag>>::Error: Send + 'static,
{
    type In = (<J as SearchTree<Tag>>::Data, f32);
    type Out = ();
    type Err = <J as SearchTree<Tag>>::Error;

    fn effect(jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let (data, score) = input;
            <J as SearchTree<Tag>>::submit(jungle, data, score).await
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSamplerInput {
    pub duration_secs: f64,
    pub output_path: String,
    pub score_specs: Vec<String>,
}

pub struct RunSampler;
#[effect(id = 11)]
impl<J> Effect<J> for RunSampler {
    type In = RunSamplerInput;
    type Out = String;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            run_sampler(input.duration_secs, &input.output_path, &input.score_specs).await?;
            Ok(input.output_path)
        }
    }
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

fn build_optimization_prompt(input: BuildOptimizationPromptInput) -> Result<Prompt, String> {
    info!(
        iteration_id = %input.iteration_id,
        prompt_attempt = input.prompt_attempt.saturating_add(1),
        similarity = input.current_similarity,
        "building mockingbird optimization prompt"
    );
    let dsp_source = fs::read_to_string(&input.dsp_source_path)
        .map_err(|err| format!("failed to read dsp source {}: {err}", input.dsp_source_path))?;

    let mut user_text = format!(
        "Iteration id: {}.\nCurrent spectrogram similarity score: {:.6}.\nPrompt attempt: {}.\n\
Target score spec: {}\n\n\
Modify `{}` so the generated guitar spectrogram moves closer to the target.\n\
Use the `{}` tool to replace the full Rust source for that file.\n\
Keep the module compiling in the existing `mockingbird-sample` crate and preserve the file's role in the welcome audio pipeline.\n\n\
Current `{}` contents:\n```rust\n{}\n```\n\nGenerated spectrogram:",
        input.iteration_id,
        input.current_similarity,
        input.prompt_attempt.saturating_add(1),
        MOCKINGBIRD_SCORE_SPEC,
        RELATIVE_ELECTRIC_GUITAR_DSP_PATH,
        MOCKINGBIRD_DSP_TOOL_NAME,
        RELATIVE_ELECTRIC_GUITAR_DSP_PATH,
        dsp_source
    );

    if let Some(retry_reason) = input.retry_reason {
        user_text.push_str("\n\nPrevious attempt failed and the file was restored.\n");
        user_text.push_str("Failure details:\n");
        user_text.push_str(&retry_reason);
    }

    Ok(Prompt {
        messages: vec![
            crate::tokens::Message {
                role: "system".to_owned(),
                contents: vec![crate::tokens::Content::Text(format!(
                    "You are tuning a Rust guitar DSP implementation against a target mel spectrogram. \
Respond with tool calls only. Only use `{}`.",
                    MOCKINGBIRD_DSP_TOOL_NAME
                ))],
            },
            crate::tokens::Message {
                role: "user".to_owned(),
                contents: vec![
                    crate::tokens::Content::Text(user_text),
                    crate::tokens::Content::Image(PathBuf::from(
                        input.generated_spectrogram_path,
                    )),
                    crate::tokens::Content::Text("Target spectrogram:".to_owned()),
                    crate::tokens::Content::Image(PathBuf::from(input.target_spectrogram_path)),
                ],
            },
        ],
        tools: Vec::new(),
    })
}

async fn apply_tool_calls(input: ApplyToolCallsInput) -> Result<ApplyToolCallsOutcome, String> {
    info!(
        iteration_id = %input.iteration_id,
        prompt_attempt = input.prompt_attempt,
        tool_call_count = input.tool_calls.len(),
        "applying mockingbird dsp tool calls"
    );
    let original = fs::read_to_string(&input.dsp_source_path).map_err(|err| {
        format!(
            "failed to read dsp source before tool application {}: {err}",
            input.dsp_source_path
        )
    })?;

    let replacement = match extract_replacement_source(&input.tool_calls) {
        Some(source) => source,
        None => {
            warn!(
                iteration_id = %input.iteration_id,
                prompt_attempt = input.prompt_attempt,
                "no valid mockingbird dsp replacement tool call returned"
            );
            return Ok(ApplyToolCallsOutcome {
                compile_ok: false,
                retry_reason: Some(format!(
                    "no valid `{}` tool call with a `source` string was returned",
                    MOCKINGBIRD_DSP_TOOL_NAME
                )),
            });
        }
    };

    fs::write(&input.dsp_source_path, replacement).map_err(|err| {
        format!(
            "failed to write replacement dsp source {}: {err}",
            input.dsp_source_path
        )
    })?;
    debug!(
        iteration_id = %input.iteration_id,
        prompt_attempt = input.prompt_attempt,
        dsp_source_path = %input.dsp_source_path,
        "wrote candidate mockingbird dsp source"
    );

    match check_sampler_compilation().await {
        Ok(()) => {
            info!(
                iteration_id = %input.iteration_id,
                prompt_attempt = input.prompt_attempt,
                "mockingbird sample compilation succeeded"
            );
            Ok(ApplyToolCallsOutcome {
                compile_ok: true,
                retry_reason: None,
            })
        }
        Err(err) => {
            fs::write(&input.dsp_source_path, original).map_err(|restore_err| {
                format!(
                    "sampler compilation failed and restoring {} also failed: {restore_err}; original error: {err}",
                    input.dsp_source_path
                )
            })?;
            warn!(
                iteration_id = %input.iteration_id,
                prompt_attempt = input.prompt_attempt,
                error = %err,
                "mockingbird sample compilation failed; restored dsp source"
            );
            Ok(ApplyToolCallsOutcome {
                compile_ok: false,
                retry_reason: Some(err),
            })
        }
    }
}

fn extract_replacement_source(tool_calls: &[ToolCall]) -> Option<String> {
    tool_calls
        .iter()
        .rev()
        .find(|tool_call| tool_call.name == MOCKINGBIRD_DSP_TOOL_NAME)
        .and_then(|tool_call| {
            tool_call
                .arguments
                .get("source")
                .or_else(|| tool_call.arguments.get("content"))
                .or_else(|| tool_call.arguments.get("contents"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        })
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
