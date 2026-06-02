use crate::effect::{
    ApplyToolCalls, ApplyToolCallsInput, ApplyToolCallsOutcome, BuildOptimizationPrompt,
    BuildOptimizationPromptInput, CompareSpectrograms, FinalizeIterationSamples,
    FinalizeIterationSamplesInput, FinalizeIterationSamplesOutcome, PromptModel, SearchTreeSelect,
    SearchTreeSubmit,
};
use crate::{DspCode, MockingBirdInstrument, MockingBirdSeed, MockingBirdState};
use jungle_sdk::prelude::*;
use std::marker::PhantomData;
use tracing::{debug, info, warn};

pub trait InstrumentMarker {
    const INSTRUMENT: MockingBirdInstrument;
}

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

fn selected_leaf(state: &MockingBirdState) -> DspCode {
    let instrument_state = state.current_state();
    instrument_state
        .selected_branch
        .last()
        .cloned()
        .unwrap_or_else(|| instrument_state.initial_dsp_code.clone())
}

pub struct SetCurrentInstrument<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SetCurrentInstrument<Marker>
where
    Marker: InstrumentMarker,
{
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &MockingBirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut MockingBirdState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.current_instrument = Marker::INSTRUMENT;
        Ok(())
    }
}

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

        for instrument_state in &mut state.instruments {
            instrument_state.begin_iteration(&state.output_root, &state.iteration_id);
        }

        info!(
            iteration = state.iteration,
            iteration_id = %state.iteration_id,
            instrument_count = state.instruments.len(),
            "starting mockingbird iteration"
        );

        Ok(())
    }
}

pub struct SelectDspBranch;
#[jungle::action]
impl Action for SelectDspBranch {
    type Effect = SearchTreeSelect;
    type Input = ();
    type Output = ();

    fn emit(state: &MockingBirdState, _input: Self::Input) -> MockingBirdInstrument {
        state.current_instrument
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = state.current_instrument;
        let iteration_id = state.iteration_id.clone();
        let instrument_state = state.current_state_mut();
        let mut branch = output.map_err(Failure::from)?;
        if branch.is_empty() {
            branch.push(instrument_state.initial_dsp_code.clone());
        }

        let selected_depth = branch.len().saturating_sub(1);
        let selected_similarity = branch.last().and_then(|code| code.similarity);
        instrument_state.selected_branch = branch;
        info!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            selected_depth,
            branch_len = instrument_state.selected_branch.len(),
            selected_similarity = selected_similarity.unwrap_or_default(),
            "selected mockingbird mcts branch"
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
        let instrument_state = state.current_state();
        (
            instrument_state.spectrogram_path.clone(),
            instrument_state.target_spectrogram_path.clone(),
        )
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = state.current_instrument;
        let iteration_id = state.iteration_id.clone();
        let instrument_state = state.current_state_mut();
        instrument_state.last_similarity = output.map_err(Failure::from)?;
        instrument_state.latest_generated_similarity = Some(instrument_state.last_similarity);
        if let Some(code) = instrument_state.latest_generated_code.as_mut() {
            code.sample_path = instrument_state.sample_path.clone();
            code.spectrogram_path = instrument_state.spectrogram_path.clone();
            code.similarity = Some(instrument_state.last_similarity);
        }
        let replace_best = instrument_state
            .best_similarity
            .map(|best| instrument_state.last_similarity >= best)
            .unwrap_or(true);
        if replace_best {
            instrument_state.best_similarity = Some(instrument_state.last_similarity);
            instrument_state.best_generated_sample_path =
                Some(instrument_state.sample_path.clone());
            instrument_state.best_generated_spectrogram_path =
                Some(instrument_state.spectrogram_path.clone());
        }
        info!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            similarity = instrument_state.last_similarity,
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
        let instrument_state = state.current_state();
        BuildOptimizationPromptInput {
            iteration_id: state.iteration_id.clone(),
            instrument: state.current_instrument,
            code_branch: instrument_state.selected_branch.clone(),
            target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
            prompt_attempt: instrument_state.prompt_attempt,
            retry_reason: instrument_state.last_retry_reason.clone(),
        }
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let prompt = output.map_err(Failure::from)?;
        let instrument_state = state.current_state();
        debug!(
            iteration_id = %state.iteration_id,
            instrument = state.current_instrument.slug(),
            prompt_attempt = instrument_state.prompt_attempt.saturating_add(1),
            selected_depth = instrument_state.selected_branch.len().saturating_sub(1),
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
        let instrument_state = state.current_state();
        info!(
            iteration_id = %state.iteration_id,
            instrument = state.current_instrument.slug(),
            prompt_attempt = instrument_state.prompt_attempt.saturating_add(1),
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
        let instrument_state = state.current_state();
        ApplyToolCallsInput {
            iteration_id: state.iteration_id.clone(),
            instrument: state.current_instrument,
            prompt_attempt: instrument_state.prompt_attempt.saturating_add(1),
            tool_name: state.current_instrument.tool_name().to_owned(),
            dsp_source_path: instrument_state.dsp_source_path.clone(),
            base_source: selected_leaf(state).source,
            tool_calls: input,
        }
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = state.current_instrument;
        let iteration_id = state.iteration_id.clone();
        let generated_iteration_id = iteration_id.clone();
        let ApplyToolCallsOutcome {
            compile_ok,
            retry_reason,
            generated_source,
        } = output.map_err(Failure::from)?;

        let instrument_state = state.current_state_mut();
        instrument_state.compile_ready = compile_ok;
        if compile_ok {
            instrument_state.last_retry_reason = None;
            instrument_state.latest_generated_code = generated_source.map(|source| DspCode {
                iteration_id: generated_iteration_id.clone(),
                source,
                sample_path: instrument_state.sample_path.clone(),
                spectrogram_path: instrument_state.spectrogram_path.clone(),
                similarity: None,
            });
            info!(
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt.saturating_add(1),
                "mockingbird dsp patch compiled successfully"
            );
        } else {
            instrument_state.latest_generated_code = None;
            instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
            instrument_state.last_retry_reason = retry_reason;
            warn!(
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                "mockingbird dsp patch failed compilation; retrying"
            );
        }

        Ok(())
    }
}

pub struct FinalizeIterationRender;
#[jungle::action]
impl Action for FinalizeIterationRender {
    type Effect = FinalizeIterationSamples;
    type Input = ();
    type Output = ();

    fn emit(state: &MockingBirdState, _input: Self::Input) -> FinalizeIterationSamplesInput {
        FinalizeIterationSamplesInput {
            iteration_id: state.iteration_id.clone(),
            instruments: state
                .instruments
                .iter()
                .map(
                    |instrument_state| crate::effect::FinalizeIterationInstrumentInput {
                        instrument: instrument_state.instrument,
                        sample_path: instrument_state.sample_path.clone(),
                        spectrogram_path: instrument_state.spectrogram_path.clone(),
                    },
                )
                .collect(),
        }
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let FinalizeIterationSamplesOutcome { rendered } = output.map_err(Failure::from)?;
        for instrument_output in rendered {
            let instrument_state = state.instrument_state_mut(instrument_output.instrument);
            instrument_state.latest_generated_sample_path =
                Some(instrument_output.sample_path.clone());
            instrument_state.latest_generated_spectrogram_path =
                Some(instrument_output.spectrogram_path.clone());
            if let Some(code) = instrument_state.latest_generated_code.as_mut() {
                code.sample_path = instrument_output.sample_path.clone();
                code.spectrogram_path = instrument_output.spectrogram_path.clone();
            }
        }
        info!(
            iteration_id = %state.iteration_id,
            rendered_instrument_count = state.instruments.len(),
            "rendered mockingbird iteration samples"
        );
        Ok(())
    }
}

pub struct SubmitDspBranch;
#[jungle::action]
impl Action for SubmitDspBranch {
    type Effect = SearchTreeSubmit;
    type Input = ();
    type Output = ();

    fn emit(
        state: &MockingBirdState,
        _input: Self::Input,
    ) -> (MockingBirdInstrument, Vec<DspCode>, f32) {
        let instrument_state = state.current_state();
        (
            state.current_instrument,
            instrument_state
                .latest_generated_code
                .clone()
                .map(|code| vec![code])
                .unwrap_or_default(),
            instrument_state.last_similarity,
        )
    }

    fn absorb(
        state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        let instrument_state = state.current_state();
        info!(
            iteration_id = %state.iteration_id,
            instrument = state.current_instrument.slug(),
            similarity = instrument_state.last_similarity,
            submitted_depth = instrument_state.selected_branch.len(),
            "submitted mockingbird mcts candidate"
        );
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
        !state.current_state().compile_ready
    }
}
