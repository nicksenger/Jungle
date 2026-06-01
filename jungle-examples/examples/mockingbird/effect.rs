#![allow(dead_code)]

use jungle_sdk::effect;
use std::future::{ready, Future};
use uuid::Uuid;

fn stub_ok<T>(value: T) -> impl Future<Output = Result<T, String>> {
    ready(Ok(value))
}

pub struct GenUuid;
#[effect(id = 0)]
impl<J> Effect<J> for GenUuid {
    type In = ();
    type Out = Uuid;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(Uuid::new_v4())
    }
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
    type In = Vec<u8>;
    type Out = Vec<u8>;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(Vec::new())
    }
}

pub struct CompareSpectrograms;
#[effect(id = 4)]
impl<J> Effect<J> for CompareSpectrograms {
    type In = (Vec<u8>, Vec<u8>);
    type Out = f32;
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(0.0)
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

pub struct SubmitResult;
#[effect(id = 8)]
impl<J> Effect<J> for SubmitResult {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}

pub struct NextMCTS;
#[effect(id = 9)]
impl<J> Effect<J> for NextMCTS {
    type In = ();
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, _input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        stub_ok(())
    }
}
