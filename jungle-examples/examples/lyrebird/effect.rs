#![allow(dead_code)]

use crate::mcts::SearchTree;
use crate::tokens::{Prompt, TokenPredictor, ToolCall};
use crate::{
    DspCode, LyrebirdBranchNode, LyrebirdInstrument, LyrebirdPatch, PulseCodeParadise,
    LYREBIRD_DURATION_SECS,
};
use image::ImageReader;
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
const SAMPLER_MANIFEST_PATH: &str = "jungle-examples/examples/lyrebird/sample/Cargo.toml";
const SAMPLER_COMMAND_TIMEOUT: Duration = Duration::from_secs(180);
const SAMPLER_BINARY_PATH: &str = "./target/release/lyrebird-sample";

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
    pub instrument: LyrebirdInstrument,
    pub target_spectrogram_path: String,
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

pub struct SearchTreeSelect;
#[effect(id = 9)]
impl Effect<()> for SearchTreeSelect {
    type In = LyrebirdInstrument;
    type Out = Vec<LyrebirdBranchNode>;
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSelect requires PulseCodeParadise runtime context".to_owned()) }
    }
}

#[effect(id = 9)]
impl<J> Effect<J> for SearchTreeSelect
where
    J: SearchTree + Sync,
    <J as SearchTree>::Data: Send + 'static,
    <J as SearchTree>::Error: Send + 'static,
{
    type In = LyrebirdInstrument;
    type Out = <J as SearchTree>::Data;
    type Err = <J as SearchTree>::Error;

    fn effect(jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { <J as SearchTree>::select(jungle, input).await }
    }
}

pub struct SearchTreeSubmit;
#[effect(id = 10)]
impl Effect<()> for SearchTreeSubmit {
    type In = (LyrebirdInstrument, Vec<LyrebirdBranchNode>, f32);
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
impl<J> Effect<J> for SearchTreeSubmit
where
    J: SearchTree + Sync,
    <J as SearchTree>::Data: Send + 'static,
    <J as SearchTree>::Error: Send + 'static,
{
    type In = (LyrebirdInstrument, <J as SearchTree>::Data, f32);
    type Out = ();
    type Err = <J as SearchTree>::Error;

    fn effect(jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let (instrument, data, score) = input;
            <J as SearchTree>::submit(jungle, instrument, data, score).await
        }
    }
}

pub struct SearchTreeSkip;
#[effect(id = 14)]
impl Effect<()> for SearchTreeSkip {
    type In = LyrebirdInstrument;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async { Err("SearchTreeSkip requires PulseCodeParadise runtime context".to_owned()) }
    }
}

#[effect(id = 14)]
impl<J> Effect<J> for SearchTreeSkip
where
    J: SearchTree + Sync,
    <J as SearchTree>::Data: Send + 'static,
    <J as SearchTree>::Error: Send + 'static,
{
    type In = LyrebirdInstrument;
    type Out = ();
    type Err = <J as SearchTree>::Error;

    fn effect(jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { <J as SearchTree>::skip(jungle, input).await }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationInstrumentInput {
    pub instrument: LyrebirdInstrument,
    pub dsp_source_path: String,
    pub original_source: String,
    pub generated_source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeIterationInstrumentOutput {
    pub instrument: LyrebirdInstrument,
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
    instrument: LyrebirdInstrument,
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
        LYREBIRD_DURATION_SECS,
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
        "captured lyrebird dsp snapshot"
    );

    Ok(DspCode {
        iteration_id: iteration_id.to_owned(),
        source,
        sample_path: sample_path.display().to_string(),
        spectrogram_path: spectrogram_path.display().to_string(),
        similarity: Some(similarity),
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

async fn finalize_iteration_samples(
    input: FinalizeIterationSamplesInput,
) -> Result<FinalizeIterationSamplesOutcome, String> {
    let mut rendered = Vec::with_capacity(input.instruments.len());
    for instrument in input.instruments {
        let build_result = with_temporary_dsp_source(
            &instrument.dsp_source_path,
            &instrument.generated_source,
            &instrument.original_source,
            || async { build_sampler_release().await },
        )
        .await;
        if let Err(err) = build_result {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = instrument.instrument.slug(),
                error = %err,
                "lyrebird sample build failed; skipping instrument render"
            );
            continue;
        }

        if let Err(err) = run_sampler_binary(
            LYREBIRD_DURATION_SECS,
            &instrument.sample_path,
            &[instrument.instrument.score_spec().to_owned()],
        )
        .await
        {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = instrument.instrument.slug(),
                error = %err,
                "lyrebird sampler run failed; skipping instrument render"
            );
            continue;
        }
        if let Err(err) =
            generate_mel_spectrogram(&instrument.sample_path, &instrument.spectrogram_path)
        {
            warn!(
                iteration_id = %input.iteration_id,
                instrument = instrument.instrument.slug(),
                error = %err,
                "lyrebird spectrogram generation failed; skipping instrument render"
            );
            continue;
        }
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
        .ok_or_else(|| "lyrebird prompt requires a selected code branch".to_owned())?;
    if selected_code.mel_spectrogram_path.is_empty() {
        return Err("selected lyrebird branch is missing a spectrogram path".to_owned());
    }

    info!(
        iteration_id = %input.iteration_id,
        instrument = input.instrument.slug(),
        prompt_attempt = input.prompt_attempt.saturating_add(1),
        selected_depth = input.code_branch.len().saturating_sub(1),
        selected_similarity = selected_code.similarity().unwrap_or_default(),
        "building lyrebird optimization prompt"
    );

    let mut contents = Vec::new();
    let mut system_text = format!(
        "Produce the next small iterative patch for `{}` so the generated {} audio moves closer to the target.\n\
The patch must be valid Rust, must still compile inside `lyrebird-sample`, and must be a localized search/replace edit rather than a full-module rewrite.\n\
Iteration id: {}.\nPrompt attempt: {}.\nSelected branch depth: {}.\n\
Target score spec: {}.\nUse `{}` exactly once with `search`, `replacement`, and `note`.\n\
The `note` must briefly explain the purpose of the change and stay within 100 characters.",
        input.instrument.relative_dsp_path(),
        input.instrument.render_subject(),
        input.iteration_id,
        input.prompt_attempt.saturating_add(1),
        input.code_branch.len().saturating_sub(1),
        input.instrument.score_spec(),
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
            let score = current
                .similarity()
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "n/a".to_owned());
            patch_history.push_str(&format!(
                "\nPatch {}.\nIteration id: {}.\nNote: {}\nScore after patch: {}\n{}\n",
                index + 1,
                current.code.iteration_id,
                patch.note,
                score,
                format_search_replace_block(patch),
            ));
        }
    }
    contents.push(crate::tokens::Content::Text(patch_history));

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

async fn build_sampler_release() -> Result<(), String> {
    if run_sampler_cargo(["build", "--release"]).await? {
        return Ok(());
    }
    Err("cargo build --release for lyrebird-sample exited unsuccessfully".to_owned())
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
        "running lyrebird sampler binary"
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
        .map_err(|err| format!("failed to spawn lyrebird-sample binary: {err}"))?;

    match timeout(SAMPLER_COMMAND_TIMEOUT, child.wait()).await {
        Ok(wait_result) => {
            let status = wait_result
                .map_err(|err| format!("failed to wait for lyrebird-sample binary: {err}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "lyrebird-sample binary exited unsuccessfully: {status}"
                ))
            }
        }
        Err(_) => {
            child.kill().await.map_err(|err| {
                format!("timed out and failed to kill lyrebird-sample binary: {err}")
            })?;
            let _ = child.wait().await;
            Err("lyrebird-sample binary timed out".to_string())
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
            code_branch: vec![DspCode {
                iteration_id: "initial".to_owned(),
                source: "fn bass() {}".to_owned(),
                sample_path: "/tmp/bass.wav".to_owned(),
                spectrogram_path: "/tmp/bass.png".to_owned(),
                similarity: Some(0.5),
            }
            .into()],
            prompt_attempt: 0,
            retry_reason: None,
        })
        .unwrap();

        assert!(prompt.tools.is_empty());
    }

    #[test]
    fn optimization_prompt_uses_patch_history_and_leaf_mel_only() {
        let prompt = build_optimization_prompt(BuildOptimizationPromptInput {
            iteration_id: "00000002".to_owned(),
            instrument: LyrebirdInstrument::Bass,
            target_spectrogram_path: "/tmp/target.png".to_owned(),
            code_branch: vec![
                DspCode {
                    iteration_id: "initial".to_owned(),
                    source: "fn bass() { baseline(); }".to_owned(),
                    sample_path: "/tmp/initial.wav".to_owned(),
                    spectrogram_path: "/tmp/initial.png".to_owned(),
                    similarity: Some(0.25),
                }
                .into(),
                LyrebirdBranchNode::from_generated(
                    DspCode {
                        iteration_id: "00000001".to_owned(),
                        source: "fn bass() { mutate(); }".to_owned(),
                        sample_path: "/tmp/00000001.wav".to_owned(),
                        spectrogram_path: "/tmp/00000001.png".to_owned(),
                        similarity: Some(0.75),
                    },
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
        assert_eq!(
            system_contents[5],
            crate::tokens::Content::Image(PathBuf::from("/tmp/00000001.png"))
        );
        assert_eq!(
            system_contents[4],
            crate::tokens::Content::Text("Current mel spectrogram:".to_owned())
        );
        if let crate::tokens::Content::Text(patch_history) = &system_contents[2] {
            assert!(patch_history.contains("Patch history:"));
            assert!(patch_history.contains("No patches have been applied yet") == false);
            assert!(patch_history.contains("bass resonance tweak"));
            assert!(patch_history.contains("Score after patch: 0.750000"));
            assert!(patch_history.contains("<<<<<<< SEARCH"));
        } else {
            panic!("expected patch history text");
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
            code_branch: vec![
                DspCode {
                    iteration_id: "initial".to_owned(),
                    source: "fn bass() { baseline(); }".to_owned(),
                    sample_path: "/tmp/initial.wav".to_owned(),
                    spectrogram_path: "/tmp/initial.png".to_owned(),
                    similarity: Some(0.25),
                }
                .into(),
                DspCode {
                    iteration_id: "00000001".to_owned(),
                    source: "fn bass() { mutate(); }".to_owned(),
                    sample_path: "/tmp/00000001.wav".to_owned(),
                    spectrogram_path: "/tmp/00000001.png".to_owned(),
                    similarity: Some(0.75),
                }
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
