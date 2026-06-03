use crate::effect::{
    OptimizeInstrument, OptimizeInstrumentInput, SearchTreeSelect, SearchTreeSubmit,
};
use crate::mcts::Submission;
use crate::{LyrebirdInstrument, LyrebirdSeed, LyrebirdState};
use jungle_sdk::prelude::*;
use std::marker::PhantomData;
use tracing::info;

pub trait InstrumentMarker {
    const INSTRUMENT: LyrebirdInstrument;
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

pub struct SetCurrentInstrument<Marker>(PhantomData<fn() -> Marker>);
#[jungle::action]
impl<Marker> Action for SetCurrentInstrument<Marker>
where
    Marker: InstrumentMarker,
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

pub struct SelectDspBranch;
#[jungle::action]
impl Action for SelectDspBranch {
    type Effect = SearchTreeSelect;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> LyrebirdInstrument {
        state.current_instrument
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = state.current_instrument;
        let iteration_id = state.iteration_id.clone();
        let instrument_state = state.current_state_mut();
        let mut branch = output.map_err(Failure::from)?;
        if branch.is_empty() {
            branch.push(instrument_state.initial_dsp_code.clone().into());
        }

        let selected_depth = branch.len().saturating_sub(1);
        let selected_similarity = branch.last().and_then(|node| node.similarity());
        instrument_state.selected_branch = branch;
        info!(
            iteration_id = %iteration_id,
            instrument = instrument.slug(),
            selected_depth,
            branch_len = instrument_state.selected_branch.len(),
            selected_similarity = selected_similarity.unwrap_or_default(),
            "selected lyrebird mcts branch"
        );
        Ok(())
    }
}

pub struct OptimizeSelectedInstrument;
#[jungle::action]
impl Action for OptimizeSelectedInstrument {
    type Effect = OptimizeInstrument;
    type Input = ();
    type Output = ();

    fn emit(state: &LyrebirdState, _input: Self::Input) -> OptimizeInstrumentInput {
        let instrument_state = state.current_state();
        OptimizeInstrumentInput {
            iteration_id: state.iteration_id.clone(),
            instrument: state.current_instrument,
            target_spectrogram_path: instrument_state.target_spectrogram_path.clone(),
            code_branch: instrument_state.selected_branch.clone(),
            prompt_attempt: instrument_state.prompt_attempt,
            retry_reason: instrument_state.last_retry_reason.clone(),
            dsp_source_path: instrument_state.dsp_source_path.clone(),
            original_source: instrument_state.initial_dsp_code.source.clone(),
            sample_path: instrument_state.sample_path.clone(),
            spectrogram_path: instrument_state.spectrogram_path.clone(),
        }
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let instrument = state.current_instrument;
        let iteration_id = state.iteration_id.clone();
        let outcome = output.map_err(Failure::from)?;
        let instrument_state = state.current_state_mut();
        instrument_state.prompt_attempt = instrument_state.prompt_attempt.saturating_add(1);
        instrument_state.compile_ready = false;
        instrument_state.pending_generated_patch = None;
        instrument_state.pending_generated_source = None;
        instrument_state.iteration_candidates = outcome.candidates;

        let best_candidate = instrument_state
            .iteration_candidates
            .iter()
            .cloned()
            .max_by(|left, right| {
                let left_similarity = left.code.similarity.unwrap_or_default();
                let right_similarity = right.code.similarity.unwrap_or_default();
                left_similarity.total_cmp(&right_similarity)
            });

        if let Some(best_candidate) = best_candidate {
            instrument_state.skipped_this_iteration = false;
            instrument_state.last_retry_reason = None;
            instrument_state.last_similarity = best_candidate.code.similarity.unwrap_or_default();
            instrument_state.latest_generated_patch = Some(best_candidate.patch.clone());
            instrument_state.latest_generated_code = Some(best_candidate.code.clone());
            instrument_state.latest_rendered_code = Some(best_candidate.code.clone());
            instrument_state.latest_generated_sample_path =
                Some(best_candidate.code.sample_path.clone());
            instrument_state.latest_generated_spectrogram_path =
                Some(best_candidate.code.spectrogram_path.clone());
            instrument_state.latest_generated_similarity = best_candidate.code.similarity;
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
                instrument_state.best_similarity = best_candidate.code.similarity;
            }
            info!(
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                candidate_count = instrument_state.iteration_candidates.len(),
                best_similarity = instrument_state.last_similarity,
                "optimized lyrebird instrument candidates"
            );
        } else {
            instrument_state.skipped_this_iteration = true;
            instrument_state.last_retry_reason = outcome.retry_reason;
            instrument_state.latest_generated_patch = None;
            instrument_state.latest_generated_code = None;
            instrument_state.last_similarity = 0.0;
            info!(
                iteration_id = %iteration_id,
                instrument = instrument.slug(),
                prompt_attempt = instrument_state.prompt_attempt,
                "lyrebird instrument produced no valid candidates this iteration"
            );
        }

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
        state: &LyrebirdState,
        _input: Self::Input,
    ) -> (
        LyrebirdInstrument,
        Vec<Submission<Vec<crate::LyrebirdBranchNode>>>,
    ) {
        let instrument_state = state.current_state();
        let submissions = instrument_state
            .iteration_candidates
            .iter()
            .cloned()
            .map(|candidate| Submission {
                score: candidate.code.similarity.unwrap_or_default(),
                data: vec![crate::LyrebirdBranchNode::from_generated(
                    candidate.code,
                    candidate.patch,
                )],
            })
            .collect();
        (state.current_instrument, submissions)
    }

    fn absorb(
        state: &mut LyrebirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        let instrument_state = state.current_state();
        info!(
            iteration_id = %state.iteration_id,
            instrument = state.current_instrument.slug(),
            submitted_candidate_count = instrument_state.iteration_candidates.len(),
            best_similarity = instrument_state.last_similarity,
            submitted_depth = instrument_state.selected_branch.len(),
            "submitted lyrebird mcts candidates"
        );
        Ok(())
    }
}

pub struct LyrebirdLoopForever;
impl Predicate<(&LyrebirdState, &())> for LyrebirdLoopForever {
    fn eval((_state, _): &(&LyrebirdState, &())) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {}
