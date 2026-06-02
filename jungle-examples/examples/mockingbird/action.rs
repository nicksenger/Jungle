use crate::effect::{
    ApplyToolCalls, ApplyToolCallsInput, ApplyToolCallsOutcome, BuildOptimizationPrompt,
    BuildOptimizationPromptInput, CompareSpectrograms, GenSample, GenSpectrogram, PromptModel,
};
use crate::{MockingBirdSeed, MockingBirdState};
use jungle_sdk::prelude::*;
use std::marker::PhantomData;
use std::path::PathBuf;
use tracing::{debug, info, warn};

pub struct SeedState<Seed, State>(PhantomData<Seed>, PhantomData<State>);
#[jungle::action(carry = Seed)]
impl<Seed, State> Action for SeedState<Seed, State>
where
    Seed: Into<State>,
{
    type Effect = Noop;
    type Input = Seed;
    type Output = ();

    fn emit(_state: &State, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Seed) {
        ((), input)
    }

    fn absorb(
        state: &mut State,
        _output: EffectCompletion<Self::Effect>,
        seed: Seed,
    ) -> Result<Self::Output, Failure> {
        *state = seed.into();
        Ok(())
    }
}

pub type SeedMockingBirdState = SeedState<MockingBirdSeed, MockingBirdState>;

pub struct BeginIteration;
#[jungle::action]
impl Action for BeginIteration {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &MockingBirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut MockingBirdState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.iteration = state.iteration.saturating_add(1);
        state.iteration_id = format!("{:08}", state.iteration);

        let iteration_dir = PathBuf::from(&state.output_root).join(&state.iteration_id);
        state.sample_path = iteration_dir
            .join("distortion_guitar.wav")
            .display()
            .to_string();
        state.spectrogram_path = iteration_dir
            .join("distortion_guitar.png")
            .display()
            .to_string();
        state.last_similarity = 0.0;
        state.compile_ready = false;
        state.prompt_attempt = 0;
        state.last_retry_reason = None;
        info!(
            iteration = state.iteration,
            iteration_id = %state.iteration_id,
            sample_path = %state.sample_path,
            spectrogram_path = %state.spectrogram_path,
            "starting mockingbird iteration"
        );

        Ok(())
    }
}

pub struct RenderSample;
#[jungle::action]
impl Action for RenderSample {
    type Effect = GenSample;
    type Input = ();
    type Output = ();

    fn emit(state: &MockingBirdState, _input: Self::Input) -> String {
        state.sample_path.clone()
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        info!(
            iteration_id = %state.iteration_id,
            sample_path = %state.sample_path,
            "rendered mockingbird sample"
        );
        Ok(())
    }
}

pub struct RenderSpectrogram;
#[jungle::action]
impl Action for RenderSpectrogram {
    type Effect = GenSpectrogram;
    type Input = ();
    type Output = ();

    fn emit(state: &MockingBirdState, _input: Self::Input) -> (String, String) {
        (state.sample_path.clone(), state.spectrogram_path.clone())
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        info!(
            iteration_id = %state.iteration_id,
            spectrogram_path = %state.spectrogram_path,
            "rendered mockingbird spectrogram"
        );
        Ok(())
    }
}

pub struct ScoreSpectrogram;
#[jungle::action]
impl Action for ScoreSpectrogram {
    type Effect = CompareSpectrograms;
    type Input = ();
    type Output = ();

    fn emit(state: &MockingBirdState, _input: Self::Input) -> (String, String) {
        (
            state.spectrogram_path.clone(),
            state.target_spectrogram_path.clone(),
        )
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.last_similarity = output.map_err(Failure::from)?;
        state.compile_ready = false;
        state.prompt_attempt = 0;
        state.last_retry_reason = None;
        info!(
            iteration_id = %state.iteration_id,
            similarity = state.last_similarity,
            "compared mockingbird spectrograms"
        );
        Ok(())
    }
}

pub struct BuildPrompt;
#[jungle::action]
impl Action for BuildPrompt {
    type Effect = BuildOptimizationPrompt;
    type Input = ();
    type Output = crate::tokens::Prompt;

    fn emit(state: &MockingBirdState, _input: Self::Input) -> BuildOptimizationPromptInput {
        BuildOptimizationPromptInput {
            iteration_id: state.iteration_id.clone(),
            generated_spectrogram_path: state.spectrogram_path.clone(),
            target_spectrogram_path: state.target_spectrogram_path.clone(),
            current_similarity: state.last_similarity,
            prompt_attempt: state.prompt_attempt,
            retry_reason: state.last_retry_reason.clone(),
            dsp_source_path: state.dsp_source_path.clone(),
        }
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let prompt = output.map_err(Failure::from)?;
        debug!(
            iteration_id = %state.iteration_id,
            prompt_attempt = state.prompt_attempt.saturating_add(1),
            similarity = state.last_similarity,
            "built mockingbird optimization prompt"
        );
        Ok(prompt)
    }
}

pub struct RequestDspPatch;
#[jungle::action]
impl Action for RequestDspPatch {
    type Effect = PromptModel;
    type Input = crate::tokens::Prompt;
    type Output = Vec<crate::tokens::ToolCall>;

    fn emit(_state: &MockingBirdState, input: Self::Input) -> crate::tokens::Prompt {
        input
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let tool_calls = output.map_err(Failure::from)?;
        info!(
            iteration_id = %state.iteration_id,
            prompt_attempt = state.prompt_attempt.saturating_add(1),
            tool_call_count = tool_calls.len(),
            "received mockingbird tool calls"
        );
        Ok(tool_calls)
    }
}

pub struct ApplyDspPatch;
#[jungle::action]
impl Action for ApplyDspPatch {
    type Effect = ApplyToolCalls;
    type Input = Vec<crate::tokens::ToolCall>;
    type Output = ();

    fn emit(state: &MockingBirdState, input: Self::Input) -> ApplyToolCallsInput {
        ApplyToolCallsInput {
            iteration_id: state.iteration_id.clone(),
            prompt_attempt: state.prompt_attempt.saturating_add(1),
            dsp_source_path: state.dsp_source_path.clone(),
            tool_calls: input,
        }
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let ApplyToolCallsOutcome {
            compile_ok,
            retry_reason,
        } = output.map_err(Failure::from)?;

        state.compile_ready = compile_ok;
        if compile_ok {
            state.last_retry_reason = None;
            info!(
                iteration_id = %state.iteration_id,
                similarity = state.last_similarity,
                prompt_attempt = state.prompt_attempt.saturating_add(1),
                "mockingbird dsp patch compiled successfully"
            );
        } else {
            state.prompt_attempt = state.prompt_attempt.saturating_add(1);
            warn!(
                iteration_id = %state.iteration_id,
                similarity = state.last_similarity,
                prompt_attempt = state.prompt_attempt,
                "mockingbird dsp patch failed compilation; retrying"
            );
            state.last_retry_reason = retry_reason;
        }

        Ok(())
    }
}

pub struct MockingBirdLoopForever;
impl Predicate<(&MockingBirdState, &())> for MockingBirdLoopForever {
    fn eval((_state, _): &(&MockingBirdState, &())) -> bool {
        true
    }
}

pub struct MockingBirdCompilePending;
impl Predicate<(&MockingBirdState, &())> for MockingBirdCompilePending {
    fn eval((state, _): &(&MockingBirdState, &())) -> bool {
        !state.compile_ready
    }
}
