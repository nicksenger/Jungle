use crate::effect::{
    FinalizeIterationSamples, FinalizeIterationSamplesInput, FinalizeIterationSamplesOutcome,
    LogIterationTimingEffect, LogIterationTimingInput, LogIterationTimingOutput,
    OptimizeInstrument, OptimizeInstrumentInput, OptimizeInstrumentOutcome, SearchTreeSelect,
    SearchTreeSubmit,
};
use crate::mcts::Submission;
use crate::{
    LyrebirdGeneratedCandidate, LyrebirdInstrument, LyrebirdInstrumentTag, LyrebirdSeed,
    LyrebirdState,
};
use jungle_sdk::prelude::*;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use tracing::info;

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

pub type SeedLyrebirdState = SeedState<LyrebirdSeed, LyrebirdState>;

pub struct FlattenJoinedUnit<S>(PhantomData<S>);
#[jungle::action]
impl<S> Action for FlattenJoinedUnit<S> {
    type Effect = Noop;
    type Input = ((), ());
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
    type Effect = Noop;
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
    type Effect = Noop;
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
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &LyrebirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut LyrebirdState,
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
            "starting lyrebird iteration"
        );

        Ok(())
    }
}

pub struct SkipInstrumentPrompt<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SkipInstrumentPrompt<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = Noop;
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
            "skipping disabled lyrebird instrument prompt branch"
        );
        Ok(())
    }
}

pub struct SelectDspBranch<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SelectDspBranch<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = SearchTreeSelect<Marker>;
    type Input = ();
    type Output = ();

    fn emit(_state: &LyrebirdState, _input: Self::Input) {}

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = Marker::INSTRUMENT;
        let iteration_id = state.iteration_id.clone();
        let instrument_state = state.instrument_state_mut(instrument);
        let mut branch = output.map_err(Failure::from)?;
        if branch.is_empty() {
            branch.push(instrument_state.initial_dsp_code.clone().into());
        }

        let selected_depth = branch.len().saturating_sub(1);
        let selected_score = branch.last().and_then(|node| node.score());
        instrument_state.selected_branch = branch;
        info!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            selected_depth,
            branch_len = instrument_state.selected_branch.len(),
            selected_score = selected_score.unwrap_or_default(),
            "selected lyrebird mcts branch"
        );
        Ok(())
    }
}

pub struct OptimizeSelectedInstrument<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for OptimizeSelectedInstrument<Marker>
where
    Marker: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Effect = OptimizeInstrument;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> OptimizeInstrumentInput {
        let instrument = Marker::INSTRUMENT;
        let instrument_state = state.instrument_state(instrument);
        OptimizeInstrumentInput {
            iteration_id: state.iteration_id.clone(),
            instrument,
            target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
            target_audio_metrics: instrument_state.target_audio_metrics,
            code_branch: instrument_state.selected_branch.clone(),
            prompt_attempt: instrument_state.prompt_attempt,
            retry_reason: instrument_state.last_retry_reason.clone(),
            sample_path: instrument_state.sample_path.clone(),
            spectrogram_path: instrument_state.spectrogram_path.clone(),
            instrument_parallelism: state.instrument_parallelism,
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = Marker::INSTRUMENT;
        let iteration_id = state.iteration_id.clone();
        let OptimizeInstrumentOutcome {
            candidates,
            retry_reason,
        } = output.map_err(Failure::from)?;
        let instrument_state = state.instrument_state_mut(instrument);
        instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
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
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                "lyrebird instrument produced no valid prompt candidates"
            );
        } else {
            instrument_state.skipped_this_iteration = false;
            instrument_state.last_retry_reason = None;
            info!(
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                candidate_count = instrument_state.pending_candidates.len(),
                "prepared lyrebird instrument candidates"
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

    fn emit(state: &LyrebirdState, _input: Self::Input) -> FinalizeIterationSamplesInput {
        FinalizeIterationSamplesInput {
            iteration_id: state.iteration_id.clone(),
            instruments: state
                .instruments
                .iter()
                .filter(|instrument_state| !instrument_state.pending_candidates.is_empty())
                .map(
                    |instrument_state| crate::effect::FinalizeIterationInstrumentInput {
                        instrument: instrument_state.instrument,
                        dsp_source_path: instrument_state.dsp_source_path.clone(),
                        original_source: instrument_state.initial_dsp_code.source.clone(),
                        target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
                        target_audio_metrics: instrument_state.target_audio_metrics,
                        candidates: instrument_state
                            .pending_candidates
                            .iter()
                            .cloned()
                            .map(|candidate| crate::effect::FinalizeIterationCandidateInput {
                                patch: candidate.patch,
                                generated_source: candidate.source,
                                sample_path: candidate.sample_path,
                                spectrogram_path: candidate.spectrogram_path,
                            })
                            .collect(),
                    },
                )
                .collect(),
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let FinalizeIterationSamplesOutcome { rendered } = output.map_err(Failure::from)?;
        let rendered_by_instrument = rendered
            .into_iter()
            .map(|instrument_output| (instrument_output.instrument, instrument_output))
            .collect::<BTreeMap<_, _>>();

        let mut rendered_candidate_count = 0usize;
        for instrument in LyrebirdInstrument::ALL {
            let instrument_state = state.instrument_state_mut(instrument);
            let previous_retry_reason = instrument_state.last_retry_reason.clone();
            instrument_state.pending_candidates.clear();
            instrument_state.iteration_candidates.clear();
            instrument_state.compile_ready = false;
            instrument_state.latest_generated_patch = None;
            instrument_state.latest_generated_code = None;
            instrument_state.latest_rendered_code = None;
            instrument_state.latest_generated_similarity = None;

            let Some(instrument_output) = rendered_by_instrument.get(&instrument) else {
                instrument_state.last_retry_reason = previous_retry_reason;
                continue;
            };

            rendered_candidate_count += instrument_output.candidates.len();
            instrument_state.iteration_candidates = instrument_output.candidates.clone();

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
                instrument_state.last_retry_reason = instrument_output.retry_reason.clone();
            }
        }

        info!(
            iteration_id = %state.iteration_id,
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
    type Effect = Noop;
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

pub struct LyrebirdLoopForever;
impl Predicate<(&LyrebirdState, &())> for LyrebirdLoopForever {
    fn eval((_state, _): &(&LyrebirdState, &())) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {}
