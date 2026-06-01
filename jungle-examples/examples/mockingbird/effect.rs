#![allow(dead_code)]

use image::ImageReader;
use image_compare::Algorithm;
use jungle_sdk::effect;
use spectrs::io::audio::read_audio_file_mono;
use spectrs::io::image::{save_spectrogram_image, Colormap};
use spectrs::spectrogram::mel::{par_convert_to_mel, MelScale};
use spectrs::spectrogram::stft::{par_compute_spectrogram, SpectrogramType};
use std::future::{ready, Future};
use std::path::Path;

fn stub_ok<T>(value: T) -> impl Future<Output = Result<T, String>> {
    ready(Ok(value))
}

const MEL_N_FFT: usize = 2048;
const MEL_HOP_LENGTH: usize = 512;
const MEL_WIN_LENGTH: usize = 2048;
const MEL_N_MELS: usize = 128;

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
impl<J> Effect<J> for PromptModel {
    type In = String;
    type Out = String;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(String::new())
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
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}

pub struct CheckSampler;
#[effect(id = 8)]
impl<J> Effect<J> for CheckSampler {
    type In = String;
    type Out = bool;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(false)
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
        None,
        None,
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
