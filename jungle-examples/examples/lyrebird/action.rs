use crate::effect::{
    BuildOptimizationPrompt, BuildOptimizationPromptInput, CompareIterationMels,
    CompareIterationMelsInput, GenerateIterationAudio, GenerateIterationAudioInput,
    GenerateIterationMels, GenerateIterationMelsInput, IterationCandidateInput,
    IterationCandidatesOutcome, LogIterationTimingEffect, LogIterationTimingInput,
    LogIterationTimingOutput, PreparePromptCandidates, PreparePromptCandidatesInput,
    PreparePromptCandidatesOutcome, RequestPromptCandidates, RequestPromptCandidatesInput,
    RequestPromptCandidatesOutcome, SearchTreeSelect, SearchTreeSubmit,
};
use crate::mcts::Submission;
use crate::{
    backoff::ExponentialBackoffInput, backoff_flow::ExponentialBackoffFlowState,
    lyrebird_prompt_request_backoff_policy, LyrebirdGeneratedCandidate, LyrebirdInstrument,
    LyrebirdInstrumentState, LyrebirdInstrumentTag, LyrebirdSeed, LyrebirdState,
    PromptInstrumentState,
};
use jungle_sdk::prelude::*;
use std::marker::PhantomData;
use std::time::Duration;
use tracing::info;

pub struct SeedState<Seed, State>(PhantomData<Seed>, PhantomData<State>);
#[jungle::action(carry = Seed)]
impl<Seed, State> Action for SeedState<Seed, State>
where
    Seed: Into<State>,
{
    type Effect = NoEffect;
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

pub type SeedLyrebirdState = SeedState<LyrebirdSeed, LyrebirdState>;

pub trait LyrebirdPromptFocus {
    fn instrument_state(&self) -> &LyrebirdInstrumentState;
    fn instrument_state_mut(&mut self) -> &mut LyrebirdInstrumentState;
}

pub trait LyrebirdPromptBackoffFocus: LyrebirdPromptFocus {
    fn prompt_backoff_state(&self)
        -> &ExponentialBackoffFlowState<LyrebirdInstrumentState, (), ()>;
    fn prompt_backoff_state_mut(
        &mut self,
    ) -> &mut ExponentialBackoffFlowState<LyrebirdInstrumentState, (), ()>;
}

impl LyrebirdPromptFocus for LyrebirdInstrumentState {
    fn instrument_state(&self) -> &LyrebirdInstrumentState {
        self
    }

    fn instrument_state_mut(&mut self) -> &mut LyrebirdInstrumentState {
        self
    }
}

impl<Marker> LyrebirdPromptFocus for PromptInstrumentState<Marker> {
    fn instrument_state(&self) -> &LyrebirdInstrumentState {
        &self.state.st
    }

    fn instrument_state_mut(&mut self) -> &mut LyrebirdInstrumentState {
        &mut self.state.st
    }
}

impl<Marker> LyrebirdPromptBackoffFocus for PromptInstrumentState<Marker> {
    fn prompt_backoff_state(
        &self,
    ) -> &ExponentialBackoffFlowState<LyrebirdInstrumentState, (), ()> {
        &self.state
    }

    fn prompt_backoff_state_mut(
        &mut self,
    ) -> &mut ExponentialBackoffFlowState<LyrebirdInstrumentState, (), ()> {
        &mut self.state
    }
}

impl<In, Out> LyrebirdPromptFocus
    for ExponentialBackoffFlowState<LyrebirdInstrumentState, In, Out>
{
    fn instrument_state(&self) -> &LyrebirdInstrumentState {
        &self.st
    }

    fn instrument_state_mut(&mut self) -> &mut LyrebirdInstrumentState {
        &mut self.st
    }
}

fn summarize_prompt_request_failures(
    responses: &[crate::effect::PromptCandidateResponse],
) -> Option<String> {
    let mut retry_reasons = Vec::new();
    for response in responses {
        if let Some(retry_reason) = response.retry_reason.as_ref() {
            if !retry_reasons
                .iter()
                .any(|existing: &String| existing == retry_reason)
            {
                retry_reasons.push(retry_reason.clone());
            }
        }
    }

    if retry_reasons.is_empty() {
        None
    } else {
        Some(retry_reasons.join("\n\n"))
    }
}

pub type LyrebirdPromptPhaseJoinOutput = ((((), ()), ((), ())), ((), ()));

pub struct FlattenLyrebirdPromptPhase<S>(PhantomData<S>);
#[jungle::action]
impl<S> Action for FlattenLyrebirdPromptPhase<S> {
    type Effect = NoEffect;
    type Input = LyrebirdPromptPhaseJoinOutput;
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) {}

    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

pub struct FlattenEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::action]
impl<T, S> Action for FlattenEither<T, S> {
    type Effect = NoEffect;
    type Input = Either<T, T>;
    type Output = T;
    type Carry = Either<T, T>;

    fn emit(_state: &S, input: Self::Input) -> ((), Either<T, T>) {
        ((), input)
    }

    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
        carry: Either<T, T>,
    ) -> Self::Output {
        match carry {
            Either::Left(value) | Either::Right(value) => value,
        }
    }
}

pub struct SetCurrentInstrument<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SetCurrentInstrument<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &LyrebirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut LyrebirdState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.current_instrument = Marker::INSTRUMENT;
        Ok(())
    }
}

pub struct InstrumentEnabled<Marker>(PhantomData<fn() -> Marker>);
impl<Marker> Predicate<(LyrebirdState, ())> for InstrumentEnabled<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    fn eval((state, _): &(LyrebirdState, ())) -> bool {
        !state.instrument_state(Marker::INSTRUMENT).disabled
    }
}

pub struct InstrumentEnabledFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
impl<Marker, Focus> Predicate<(Focus, ())> for InstrumentEnabledFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    fn eval((state, _): &(Focus, ())) -> bool {
        !state.instrument_state().disabled
    }
}

pub struct LogIterationTiming;
#[jungle::action]
impl Action for LogIterationTiming {
    type Effect = LogIterationTimingEffect;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> LogIterationTimingInput {
        LogIterationTimingInput {
            completed_iteration: state.iteration,
            completed_iteration_id: state.iteration_id.clone(),
            previous_iteration_start_time_ms: state.iteration_start_time_ms,
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let LogIterationTimingOutput {
            iteration_start_time_ms,
        } = output.map_err(Failure::from)?;
        state.iteration_start_time_ms = Some(iteration_start_time_ms);
        Ok(())
    }
}

pub struct BeginIteration;
#[jungle::action]
impl Action for BeginIteration {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &LyrebirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut LyrebirdState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.iteration = state.iteration.saturating_add(1);
        state.iteration_id = format!("{:08}", state.iteration);
        let output_root = state.output_root.clone();
        let iteration_id = state.iteration_id.clone();
        let instrument_parallelism = state.instrument_parallelism;

        for instrument in LyrebirdInstrument::ALL {
            state.instrument_state_mut(instrument).begin_iteration(
                &output_root,
                &iteration_id,
                instrument_parallelism,
            );
        }

        info!(
            iteration = state.iteration,
            iteration_id = %state.iteration_id,
            instrument_count = LyrebirdInstrument::ALL.len(),
            "starting lyrebird iteration"
        );

        Ok(())
    }
}

pub struct SkipInstrumentPromptFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for SkipInstrumentPromptFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument_state = state.instrument_state();
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            "skipping disabled lyrebird instrument prompt branch"
        );
        Ok(())
    }
}

pub struct SelectDspBranchFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for SelectDspBranchFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = SearchTreeSelect<Marker>;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        state: &mut Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument_state = state.instrument_state_mut();
        let mut branch = output.map_err(Failure::from)?;
        if branch.is_empty() {
            branch.push(instrument_state.initial_dsp_code.clone().into());
        }

        let selected_depth = branch.len().saturating_sub(1);
        let selected_score = branch.last().and_then(|node| node.score());
        instrument_state.selected_branch = branch;
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            selected_depth,
            branch_len = instrument_state.selected_branch.len(),
            selected_score = selected_score.unwrap_or_default(),
            "selected lyrebird mcts branch"
        );
        Ok(())
    }
}

pub struct BuildOptimizationPromptFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for BuildOptimizationPromptFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = BuildOptimizationPrompt;
    type Input = ();
    type Output = ();

    fn emit(state: &Focus, _input: Self::Input) -> BuildOptimizationPromptInput {
        let instrument_state = state.instrument_state();
        BuildOptimizationPromptInput {
            iteration_id: instrument_state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
            target_audio_metrics: instrument_state.target_audio_metrics,
            code_branch: instrument_state.selected_branch.clone(),
            prompt_attempt: instrument_state.prompt_attempt,
            retry_reason: instrument_state.last_retry_reason.clone(),
            system_prompt_override: instrument_state.system_prompt_override.clone(),
        }
    }

    fn absorb(
        state: &mut Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let prompt = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut();
        instrument_state.pending_prompt = Some(prompt);
        instrument_state.pending_prompt_candidates.clear();
        Ok(())
    }
}

pub struct RequestPromptCandidatesFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for RequestPromptCandidatesFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = RequestPromptCandidates;
    type Input = ();
    type Output = ();

    fn emit(state: &Focus, _input: Self::Input) -> RequestPromptCandidatesInput {
        let instrument_state = state.instrument_state();
        RequestPromptCandidatesInput {
            prompt: instrument_state
                .pending_prompt
                .clone()
                .expect("lyrebird prompt request step requires a prepared prompt"),
            iteration_id: instrument_state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            prompt_attempt: instrument_state.prompt_attempt.saturating_add(1),
            instrument_parallelism: instrument_state.instrument_parallelism,
        }
    }

    fn absorb(
        state: &mut Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument_state = state.instrument_state_mut();
        let RequestPromptCandidatesOutcome { responses } = match output {
            Ok(outcome) => outcome,
            Err(err) => {
                instrument_state.pending_prompt = None;
                instrument_state.pending_prompt_candidates.clear();
                instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
                instrument_state.last_retry_reason = Some(err.clone());
                return Err(Failure::from(err));
            }
        };
        instrument_state.pending_prompt = None;
        instrument_state.pending_prompt_candidates = responses;
        Ok(())
    }
}

pub struct BeginPromptRequestAttemptFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for BeginPromptRequestAttemptFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument_state = state.instrument_state();
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            prompt_attempt = instrument_state.prompt_attempt.saturating_add(1),
            "starting lyrebird prompt request attempt"
        );
        Ok(())
    }
}

pub struct EmitPromptRequestBackoffInputFocused<Marker, Focus>(
    PhantomData<fn() -> (Marker, Focus)>,
);
#[jungle::action]
impl<Marker, Focus> Action for EmitPromptRequestBackoffInputFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ExponentialBackoffInput<()>;

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        _state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(ExponentialBackoffInput {
            action_input: (),
            policy: lyrebird_prompt_request_backoff_policy(),
        })
    }
}

pub struct InitializePromptRequestBackoffFocused<Focus>(PhantomData<fn() -> Focus>);
#[jungle::action]
impl<Focus> Action for InitializePromptRequestBackoffFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<()>;
    type Output = ();
    type Carry = ExponentialBackoffInput<()>;

    fn emit(_state: &Focus, input: Self::Input) -> ((), ExponentialBackoffInput<()>) {
        ((), input)
    }

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<()>,
    ) -> Result<Self::Output, Failure> {
        let backoff_state = state.prompt_backoff_state_mut();
        backoff_state.attempts = 0;
        backoff_state.current_delay_ms = carry.policy.initial_delay_ms;
        backoff_state.policy = carry.policy;
        backoff_state.flow_input = Some(carry.action_input);
        backoff_state.last_result = None;
        Ok(())
    }
}

pub struct RecordPromptRequestBackoffResultFocused<Focus>(PhantomData<fn() -> Focus>);
#[jungle::action]
impl<Focus> Action for RecordPromptRequestBackoffResultFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = Result<(), Failure>;
    type Output = ();
    type Carry = Result<(), Failure>;

    fn emit(_state: &Focus, input: Self::Input) -> ((), Result<(), Failure>) {
        ((), input)
    }

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
        carry: Result<(), Failure>,
    ) -> Result<Self::Output, Failure> {
        let backoff_state = state.prompt_backoff_state_mut();
        backoff_state.attempts = backoff_state.attempts.saturating_add(1);
        backoff_state.last_result = Some(carry);
        Ok(())
    }
}

pub struct SleepForPromptRequestBackoffFocused<Focus>(PhantomData<fn() -> Focus>);
#[jungle::action]
impl<Focus> Action for SleepForPromptRequestBackoffFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone + Send + Sync + 'static,
{
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &Focus, _input: Self::Input) -> Duration {
        Duration::from_millis(state.prompt_backoff_state().current_delay_ms)
    }

    fn absorb(
        state: &mut Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        let backoff_state = state.prompt_backoff_state_mut();
        backoff_state.current_delay_ms = backoff_state
            .policy
            .next_delay_ms(backoff_state.current_delay_ms);
        Ok(())
    }
}

pub struct SkipPromptRequestBackoffSleepFocused<Focus>(PhantomData<fn() -> Focus>);
#[jungle::action]
impl<Focus> Action for SkipPromptRequestBackoffSleepFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        _state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

pub struct TakePromptRequestBackoffSuccessFocused<Focus>(PhantomData<fn() -> Focus>);
#[jungle::action]
impl<Focus> Action for TakePromptRequestBackoffSuccessFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        match state.prompt_backoff_state_mut().last_result.take() {
            Some(Ok(())) => Ok(()),
            Some(Err(err)) => Err(Failure::from(format!(
                "prompt request backoff ended with failure instead of success: {err}"
            ))),
            None => Err(Failure::from(
                "prompt request backoff is missing the terminal retry result",
            )),
        }
    }
}

pub struct PromptRequestBackoffPendingFocused<Focus>(PhantomData<fn() -> Focus>);
impl<Focus> Predicate<(&Focus, &())> for PromptRequestBackoffPendingFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus,
{
    fn eval((state, _): &(&Focus, &())) -> bool {
        match state.prompt_backoff_state().last_result.as_ref() {
            None => true,
            Some(Ok(())) => false,
            Some(Err(_)) => true,
        }
    }
}

pub struct PromptRequestBackoffShouldSleepFocused<Focus>(PhantomData<fn() -> Focus>);
impl<Focus> Predicate<(Focus, ())> for PromptRequestBackoffShouldSleepFocused<Focus>
where
    Focus: LyrebirdPromptBackoffFocus + Clone,
{
    fn eval((state, _): &(Focus, ())) -> bool {
        matches!(
            state.prompt_backoff_state().last_result.as_ref(),
            Some(Err(_))
        )
    }
}

pub struct EnsurePromptRequestSucceededFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for EnsurePromptRequestSucceededFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Focus, _input: Self::Input) {}

    fn absorb(
        state: &mut Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument_state = state.instrument_state_mut();
        if instrument_state
            .pending_prompt_candidates
            .iter()
            .any(|response| response.tool_calls.is_some())
        {
            return Ok(());
        }

        instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
        let failure_reason =
            summarize_prompt_request_failures(&instrument_state.pending_prompt_candidates)
                .unwrap_or_else(|| {
                    "prompt request returned no successful OpenAI responses".to_owned()
                });
        instrument_state.last_retry_reason = Some(failure_reason.clone());
        Err(Failure::from(failure_reason))
    }
}

pub struct PreparePromptCandidatesFocused<Marker, Focus>(PhantomData<fn() -> (Marker, Focus)>);
#[jungle::action]
impl<Marker, Focus> Action for PreparePromptCandidatesFocused<Marker, Focus>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
    Focus: LyrebirdPromptFocus + Clone + Send + Sync + 'static,
{
    type Effect = PreparePromptCandidates;
    type Input = ();
    type Output = ();

    fn emit(state: &Focus, _input: Self::Input) -> PreparePromptCandidatesInput {
        let instrument_state = state.instrument_state();
        let current_source = instrument_state
            .selected_branch
            .last()
            .map(|node| node.code.source.clone())
            .unwrap_or_default();
        PreparePromptCandidatesInput {
            iteration_id: instrument_state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            prompt_attempt: instrument_state.prompt_attempt.saturating_add(1),
            tool_name: Marker::INSTRUMENT.tool_name().to_owned(),
            current_source,
            sample_path: instrument_state.sample_path.clone(),
            spectrogram_path: instrument_state.spectrogram_path.clone(),
            instrument_parallelism: instrument_state.instrument_parallelism,
            responses: instrument_state.pending_prompt_candidates.clone(),
        }
    }

    fn absorb(
        state: &mut Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let PreparePromptCandidatesOutcome {
            candidates,
            retry_reason,
        } = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut();
        instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
        instrument_state.pending_prompt = None;
        instrument_state.pending_prompt_candidates.clear();
        instrument_state.compile_ready = false;
        instrument_state.pending_generated_patch = None;
        instrument_state.pending_generated_source = None;
        instrument_state.pending_candidates = candidates;
        instrument_state.iteration_candidates.clear();
        instrument_state.latest_generated_patch = None;
        instrument_state.latest_generated_code = None;
        instrument_state.latest_rendered_code = None;
        instrument_state.latest_generated_similarity = None;
        instrument_state.last_similarity = 0.0;

        if instrument_state.pending_candidates.is_empty() {
            instrument_state.skipped_this_iteration = true;
            instrument_state.last_retry_reason = retry_reason;
            info!(
                iteration_id = %instrument_state.iteration_id,
                instrument = Marker::INSTRUMENT.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                "lyrebird instrument produced no valid prompt candidates"
            );
        } else {
            instrument_state.skipped_this_iteration = false;
            instrument_state.last_retry_reason = None;
            info!(
                iteration_id = %instrument_state.iteration_id,
                instrument = Marker::INSTRUMENT.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                candidate_count = instrument_state.pending_candidates.len(),
                "prepared lyrebird instrument candidates"
            );
        }

        Ok(())
    }
}

pub struct GenerateIterationCandidateAudio<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for GenerateIterationCandidateAudio<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = GenerateIterationAudio;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> GenerateIterationAudioInput {
        let instrument_state = state.instrument_state(Marker::INSTRUMENT);
        GenerateIterationAudioInput {
            iteration_id: state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            dsp_source_path: instrument_state.dsp_source_path.clone(),
            original_source: instrument_state.initial_dsp_code.source.clone(),
            candidates: instrument_state
                .pending_candidates
                .iter()
                .cloned()
                .map(|candidate| IterationCandidateInput {
                    patch: candidate.patch,
                    generated_source: candidate.source,
                    sample_path: candidate.sample_path,
                    spectrogram_path: candidate.spectrogram_path,
                })
                .collect(),
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let outcome = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut(Marker::INSTRUMENT);
        instrument_state.pending_candidates.clear();
        let generated_candidate_count = apply_iteration_stage_outcome(instrument_state, outcome);
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            generated_candidate_count,
            "generated lyrebird iteration candidate audio"
        );
        Ok(())
    }
}

pub struct GenerateIterationCandidateMels<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for GenerateIterationCandidateMels<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = GenerateIterationMels;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> GenerateIterationMelsInput {
        let instrument_state = state.instrument_state(Marker::INSTRUMENT);
        GenerateIterationMelsInput {
            iteration_id: state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            candidates: instrument_state.iteration_candidates.clone(),
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let outcome = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut(Marker::INSTRUMENT);
        let generated_candidate_count = apply_iteration_stage_outcome(instrument_state, outcome);
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            generated_candidate_count,
            "generated lyrebird iteration candidate mels"
        );
        Ok(())
    }
}

pub struct CompareIterationCandidateMels<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for CompareIterationCandidateMels<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = CompareIterationMels;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> CompareIterationMelsInput {
        let instrument_state = state.instrument_state(Marker::INSTRUMENT);
        CompareIterationMelsInput {
            iteration_id: state.iteration_id.clone(),
            instrument: Marker::INSTRUMENT,
            target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
            target_audio_metrics: instrument_state.target_audio_metrics,
            candidates: instrument_state.iteration_candidates.clone(),
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let outcome = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut(Marker::INSTRUMENT);
        let rendered_candidate_count = apply_scored_candidates(instrument_state, outcome);
        info!(
            iteration_id = %instrument_state.iteration_id,
            instrument = Marker::INSTRUMENT.slug(),
            rendered_candidate_count,
            "finalized lyrebird iteration candidates"
        );
        Ok(())
    }
}

pub struct SubmitDspBranch<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SubmitDspBranch<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = SearchTreeSubmit<Marker>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &LyrebirdState,
        _input: Self::Input,
    ) -> Vec<Submission<Vec<crate::LyrebirdBranchNode>>> {
        state
            .instrument_state(Marker::INSTRUMENT)
            .iteration_candidates
            .iter()
            .cloned()
            .map(|candidate| Submission {
                score: candidate.code.score().unwrap_or_default(),
                data: vec![crate::LyrebirdBranchNode::from_generated(
                    candidate.code,
                    candidate.patch,
                )],
            })
            .collect()
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        let instrument = Marker::INSTRUMENT;
        let instrument_state = state.instrument_state(instrument);
        info!(
            iteration_id = %state.iteration_id,
            instrument = instrument.slug(),
            submitted_candidate_count = instrument_state.iteration_candidates.len(),
            best_score = instrument_state.last_similarity,
            submitted_depth = instrument_state.selected_branch.len(),
            "submitted lyrebird mcts candidates"
        );
        Ok(())
    }
}

pub struct SkipInstrumentSubmit<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SkipInstrumentSubmit<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &LyrebirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut LyrebirdState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = Marker::INSTRUMENT;
        info!(
            iteration_id = %state.iteration_id,
            instrument = instrument.slug(),
            "skipping disabled lyrebird instrument submit branch"
        );
        Ok(())
    }
}

fn best_candidate(candidates: &[LyrebirdGeneratedCandidate]) -> Option<LyrebirdGeneratedCandidate> {
    candidates.iter().cloned().max_by(|left, right| {
        let left_score = left.code.score().unwrap_or_default();
        let right_score = right.code.score().unwrap_or_default();
        left_score.total_cmp(&right_score)
    })
}

fn apply_iteration_stage_outcome(
    instrument_state: &mut LyrebirdInstrumentState,
    outcome: IterationCandidatesOutcome,
) -> usize {
    let previous_retry_reason = instrument_state.last_retry_reason.clone();
    instrument_state.iteration_candidates = outcome.candidates;
    if instrument_state.iteration_candidates.is_empty() {
        instrument_state.skipped_this_iteration = true;
        instrument_state.last_retry_reason = outcome.retry_reason.or(previous_retry_reason);
    } else {
        instrument_state.skipped_this_iteration = false;
        instrument_state.last_retry_reason = None;
    }
    instrument_state.iteration_candidates.len()
}

fn apply_scored_candidates(
    instrument_state: &mut LyrebirdInstrumentState,
    outcome: IterationCandidatesOutcome,
) -> usize {
    let previous_retry_reason = instrument_state.last_retry_reason.clone();
    instrument_state.iteration_candidates = outcome.candidates;
    instrument_state.compile_ready = false;
    instrument_state.latest_generated_patch = None;
    instrument_state.latest_generated_code = None;
    instrument_state.latest_rendered_code = None;
    instrument_state.latest_generated_similarity = None;

    let best_candidate = best_candidate(&instrument_state.iteration_candidates);
    if let Some(best_candidate) = best_candidate {
        instrument_state.compile_ready = true;
        instrument_state.skipped_this_iteration = false;
        instrument_state.last_retry_reason = None;
        instrument_state.last_similarity = best_candidate.code.score().unwrap_or_default();
        instrument_state.latest_generated_patch = Some(best_candidate.patch.clone());
        instrument_state.latest_generated_code = Some(best_candidate.code.clone());
        instrument_state.latest_rendered_code = Some(best_candidate.code.clone());
        instrument_state.latest_generated_sample_path =
            Some(best_candidate.code.sample_path.clone());
        instrument_state.latest_generated_spectrogram_path =
            Some(best_candidate.code.spectrogram_path.clone());
        instrument_state.latest_generated_similarity = best_candidate.code.score();
        let replace_best = instrument_state
            .best_similarity
            .map(|best| instrument_state.last_similarity >= best)
            .unwrap_or(true);
        if replace_best {
            instrument_state.best_generated_code = Some(best_candidate.code.clone());
            instrument_state.best_generated_sample_path =
                Some(best_candidate.code.sample_path.clone());
            instrument_state.best_generated_spectrogram_path =
                Some(best_candidate.code.spectrogram_path.clone());
            instrument_state.best_similarity = best_candidate.code.score();
        }
    } else {
        instrument_state.skipped_this_iteration = true;
        instrument_state.last_similarity = 0.0;
        instrument_state.last_retry_reason = outcome.retry_reason.or(previous_retry_reason);
    }

    instrument_state.iteration_candidates.len()
}

pub struct LyrebirdLoopForever;
impl Predicate<(&LyrebirdState, &())> for LyrebirdLoopForever {
    fn eval((_state, _): &(&LyrebirdState, &())) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {}
