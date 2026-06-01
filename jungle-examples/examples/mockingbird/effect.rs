#![allow(dead_code)]

use crate::tokens::{Prompt, TokenPredictor, ToolCall};
use image::ImageReader;
use image_compare::Algorithm;
use jungle_sdk::effect;
use serde::{Deserialize, Serialize};
use spectrs::io::audio::read_audio_file_mono;
use spectrs::io::image::{save_spectrogram_image, Colormap};
use spectrs::spectrogram::mel::{par_convert_to_mel, MelScale};
use spectrs::spectrogram::stft::{par_compute_spectrogram, SpectrogramType};
use std::future::{ready, Future};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

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
    type Out = Vec<u8>;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(Vec::new())
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
impl<J> Effect<J> for PromptModel
where
    J: TokenPredictor + Sync,
    J::Error: ToString + Send + 'static,
{
    type In = Prompt;
    type Out = Vec<ToolCall>;
    type Err = String;

    fn effect(jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { jungle.predict(input).await.map_err(|err| err.to_string()) }
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

pub struct InsertNode;
#[effect(id = 10)]
impl<J> Effect<J> for InsertNode {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}

pub struct SearchTreeMove;
#[effect(id = 9)]
impl<J> Effect<J> for SearchTreeMove {
    type In = ();
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
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

    let similarity =
        image_compare::gray_similarity_structure(&Algorithm::MSSIMSimple, &left, &right)
            .map_err(|err| format!("failed to compare spectrograms: {err}"))?;

    Ok(similarity.score as f32)
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

    let mut command = Command::new(SAMPLER_BINARY_PATH);
    command
        .arg("--duration-secs")
        .arg(duration_secs.to_string())
        .arg("--output-path")
        .arg(output_path)
        .args(score_specs)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = command.spawn().map_err(|err| {
        format!(
            "failed to spawn sampler binary {}: {err}",
            SAMPLER_BINARY_PATH
        )
    })?;

    match timeout(SAMPLER_COMMAND_TIMEOUT, child.wait()).await {
        Ok(wait_result) => {
            let status =
                wait_result.map_err(|err| format!("failed to wait for sampler binary: {err}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("sampler binary exited unsuccessfully: {status}"))
            }
        }
        Err(_) => {
            child
                .kill()
                .await
                .map_err(|err| format!("timed out and failed to kill sampler binary: {err}"))?;
            let _ = child.wait().await;
            Err("sampler binary timed out".to_string())
        }
    }
}
