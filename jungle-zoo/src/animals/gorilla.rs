//! Gorilla state model and lifecycle journey.

use crate::effects;
use crate::state::{
    ActivitySchedule, AgeState, CircadianWindow, DailyActivity, Finger, FingerSet, FruitFlesh,
    FruitMeal, FruitRind, Hand, Hands, LifePhase, Lobe, Nail, NervousSystem, PerceivedTimeOfDay,
    TemporalState, TimePerception, VitalReadings,
};
use jungle_sdk::types::{
    BoundAct, Animal, Animals, Condition, Conditional, EffectCompletion, EffectSchema, Id, Identified,
    Identity, LoopCondition, NodeMetadata, NoopObservation, NoopPerturbation, Observable,
    Perturbable, BoundStep, Transparent, While,
};
use jungle_sdk::typosaurus::num::consts::U0;
use jungle_sdk::Optic;
use serde::{Deserialize, Serialize};

const GORILLA_DAY_LOOPS_PER_YEAR: u16 = 4;

#[derive(Optic, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub meal: FruitMeal,
    pub hands: Hands,
    pub nervous_system: NervousSystem,
    pub temporal: TemporalState,
}

impl From<TemporalState> for State {
    fn from(temporal: TemporalState) -> Self {
        Self {
            age: u32::from(temporal.age.age_years),
            vitals: VitalReadings {
                energy: 42,
                is_hungry: false,
                is_sleepy: false,
                stress: 20,
            },
            meal: FruitMeal {
                name: "fig".to_owned(),
                rind: FruitRind {
                    thickness_mm: 1,
                    fibrous: false,
                },
                flesh: FruitFlesh {
                    sugar_brix: 16,
                    mass_g: 140,
                },
                has_hard_seed: true,
            },
            hands: default_hands(),
            nervous_system: default_nervous_system(),
            temporal,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::from(default_temporal_seed())
    }
}

impl From<TemporalState> for () {
    fn from(_value: TemporalState) -> Self {}
}

//#[cfg(test)]
//mod compile_checks {
//    use super::*;
//    use jungle_sdk::types::{BuildFlowWithContext, DynFlow, JourneyEffects, Running, Waiting};
//    use std::sync::Arc;
//
//    #[allow(dead_code)]
//    fn assert_running<F: Running<In = (State, ())>>() {}
//
//    #[allow(dead_code)]
//    fn assert_waiting<F: Waiting>() {}
//
//    #[allow(dead_code)]
//    fn assert_flow_effects<F: JourneyEffects>() {}
//
//    #[allow(dead_code)]
//    fn assert_context_flow<F>()
//    where
//        F: BuildFlowWithContext<
//            (Arc<crate::Zoo>, DynFlow<State>),
//            Output = (Arc<crate::Zoo>, DynFlow<State>),
//        >,
//    {
//    }
//
//    #[test]
//    fn probe_flow_compile_contracts_hold() {
//        assert_running::<BoundStep<Gorilla, GorillaBirth>>();
//        assert_running::<GorillaFeedFlow>();
//        assert_running::<GorillaToolSocialFlow>();
//        assert_running::<GorillaSimpleSocialFlow>();
//        assert_running::<GorillaActiveFlow>();
//        assert_running::<GorillaDayFlow>();
//        assert_running::<GorillaYearFlow>();
//        assert_running::<GorillaJourney>();
//
//        assert_waiting::<BoundStep<Gorilla, GorillaBirth>>();
//        assert_flow_effects::<BoundStep<Gorilla, GorillaBirth>>();
//
//        assert_context_flow::<GorillaFeedFlow>();
//        assert_context_flow::<GorillaToolSocialFlow>();
//        assert_context_flow::<GorillaSimpleSocialFlow>();
//        assert_context_flow::<GorillaActiveFlow>();
//        assert_context_flow::<GorillaDayFlow>();
//        assert_context_flow::<GorillaYearFlow>();
//        assert_context_flow::<GorillaJourney>();
//    }
//}

fn default_hands() -> Hands {
    fn finger(length_mm: u16, nail_len: u8) -> Finger {
        Finger {
            nail: Nail {
                length_mm: nail_len,
                curved: true,
            },
            length_mm,
        }
    }

    let fingers = FingerSet {
        thumb: finger(44, 7),
        index: finger(58, 6),
        middle: finger(64, 6),
        ring: finger(60, 6),
        little: finger(52, 5),
    };

    Hands {
        left: Hand {
            fingers: fingers.clone(),
            opposable_thumb: true,
        },
        right: Hand {
            fingers,
            opposable_thumb: true,
        },
    }
}

fn default_nervous_system() -> NervousSystem {
    use crate::state::{Brain, Cortex, NeuronDensity};

    NervousSystem {
        cranial_nerves: 12,
        reflex_latency_ms: 70,
        vitals: VitalReadings {
            energy: 42,
            is_hungry: false,
            is_sleepy: false,
            stress: 20,
        },
        brain: Brain {
            cortex: Cortex {
                frontal: Lobe {
                    name: "frontal".to_owned(),
                    density: NeuronDensity {
                        neurons_per_mm3: 95_000,
                    },
                },
                temporal: Lobe {
                    name: "temporal".to_owned(),
                    density: NeuronDensity {
                        neurons_per_mm3: 91_000,
                    },
                },
            },
            mass_g: 420,
        },
    }
}

pub struct GorillaStillGrowing;
impl LoopCondition<State> for GorillaStillGrowing {
    type Arg = ();

    fn should_continue(state: &State) -> bool {
        state.temporal.age.life_phase != LifePhase::Adult
    }
}

pub struct GorillaDaylightRemaining;
impl LoopCondition<State> for GorillaDaylightRemaining {
    type Arg = ();

    fn should_continue(state: &State) -> bool {
        state.temporal.perception.minutes_since_transition < GORILLA_DAY_LOOPS_PER_YEAR
    }
}

pub struct GorillaIsActiveNow;
impl Condition<(State, ())> for GorillaIsActiveNow {
    fn choose((state, _): &(State, ())) -> bool {
        match state.temporal.schedule.activity {
            DailyActivity::Diurnal => {
                matches!(
                    state.temporal.perception.current,
                    PerceivedTimeOfDay::Morning | PerceivedTimeOfDay::Afternoon
                )
            }
            DailyActivity::Nocturnal => {
                matches!(
                    state.temporal.perception.current,
                    PerceivedTimeOfDay::Evening | PerceivedTimeOfDay::Night
                )
            }
        }
    }
}

pub struct GorillaIsHungry;
impl Condition<(State, ())> for GorillaIsHungry {
    fn choose((state, _): &(State, ())) -> bool {
        state.vitals.is_hungry || state.vitals.energy < 36
    }
}

pub struct GorillaCanUseTools;
impl Condition<(State, ())> for GorillaCanUseTools {
    fn choose((state, _): &(State, ())) -> bool {
        state.hands.left.opposable_thumb
            && state.hands.right.opposable_thumb
            && state.temporal.age.life_phase != LifePhase::Child
    }
}

pub struct GorillaAdvanceAge;
impl BoundAct<Gorilla> for GorillaAdvanceAge {
    type Effect = effects::AdvanceAge;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.temporal.age.age_years
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let next = output.expect("gorilla age advancement should succeed");
        state.temporal.age = next;
        state.age = u32::from(state.temporal.age.age_years);

        // Each annual cycle begins a new day schedule from morning.
        state.temporal.perception.current = PerceivedTimeOfDay::Morning;
        state.temporal.perception.minutes_since_transition = 0;
    }
}

pub struct GorillaTickPerceivedTime;
impl BoundAct<Gorilla> for GorillaTickPerceivedTime {
    type Effect = effects::TickPerceivedTime;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        let segment_minutes = if state.temporal.perception.minutes_since_transition % 2 == 0 {
            0
        } else {
            360
        };
        (state.temporal.perception.current, segment_minutes)
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let next = output.expect("gorilla perceived-time tick should succeed");
        // Track day-loop progress as "iterations elapsed this year" and keep
        // time-of-day cycling for activity-window branch decisions.
        let day_iteration = state
            .temporal
            .perception
            .minutes_since_transition
            .saturating_add(1);
        state.temporal.perception.current = next.current;
        state.temporal.perception.minutes_since_transition = day_iteration;
    }
}

pub struct GorillaBirthday;
impl BoundAct<Gorilla> for GorillaBirthday {
    type Effect = effects::CelebrateBirthday;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.temporal.age = output.expect("gorilla birthday state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaBirth;
impl BoundAct<Gorilla> for GorillaBirth {
    type Effect = effects::Birth;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.temporal.age = output.expect("gorilla birth state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaEvaluateActivityWindow;
impl BoundAct<Gorilla> for GorillaEvaluateActivityWindow {
    type Effect = effects::EvaluateActivityWindow;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            state.temporal.schedule.activity,
            state.temporal.perception.current,
        )
    }

    fn absorb(_state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let _ = output.expect("gorilla activity-window evaluation should succeed");
    }
}

pub struct GorillaPeelFruit;
impl BoundAct<Gorilla> for GorillaPeelFruit {
    type Effect = effects::PeelFruit;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (state.meal.rind.thickness_mm, state.meal.flesh.mass_g)
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let edible = output.expect("gorilla peel-fruit should succeed");
        state.meal.flesh.mass_g = edible;
    }
}

pub struct GorillaEat;
impl BoundAct<Gorilla> for GorillaEat {
    type Effect = effects::Eat;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let energy = output.expect("gorilla eat should succeed");
        state.vitals.energy = energy;
        state.vitals.is_hungry = energy < 30;
        state.vitals.is_sleepy = energy < 25;
    }
}

pub struct GorillaUseTool;
impl BoundAct<Gorilla> for GorillaUseTool {
    type Effect = effects::UseTool;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.hands.left.opposable_thumb && state.hands.right.opposable_thumb
    }

    fn absorb(_state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let _note = output.expect("gorilla tool-use should succeed");
    }
}

pub struct GorillaChestBeat;
impl BoundAct<Gorilla> for GorillaChestBeat {
    type Effect = effects::ChestBeat;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            state.vitals.stress,
            state.hands.left.opposable_thumb && state.hands.right.opposable_thumb,
        )
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.vitals.stress = output.expect("gorilla chest-beat should succeed");
    }
}

pub struct GorillaRest;
impl BoundAct<Gorilla> for GorillaRest {
    type Effect = effects::Rest;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let energy = output.expect("gorilla rest should succeed");
        state.vitals.energy = energy;
        state.vitals.is_hungry = energy < 30;
        state.vitals.is_sleepy = energy < 25;
    }
}

pub struct GorillaMakeSound;
impl BoundAct<Gorilla> for GorillaMakeSound {
    type Effect = effects::MakeSound;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &State, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        let kind = match state.temporal.perception.current {
            PerceivedTimeOfDay::Morning => "morning call",
            PerceivedTimeOfDay::Afternoon => "contact hoot",
            PerceivedTimeOfDay::Evening => "group regroup",
            PerceivedTimeOfDay::Night => "quiet rustle",
        };
        (kind.to_owned(), state.vitals.stress)
    }

    fn absorb(_state: &mut State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let _signal = output.expect("gorilla vocalization should succeed");
    }
}

#[derive(jungle_sdk::Journey)]
pub struct GorillaFeedFlow(
    BoundStep<Gorilla, GorillaPeelFruit>,
    BoundStep<Gorilla, GorillaEat>,
    BoundStep<Gorilla, GorillaRest>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaToolSocialFlow(
    BoundStep<Gorilla, GorillaUseTool>,
    BoundStep<Gorilla, GorillaChestBeat>,
    BoundStep<Gorilla, GorillaMakeSound>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaSimpleSocialFlow(BoundStep<Gorilla, GorillaMakeSound>, BoundStep<Gorilla, GorillaRest>);

pub type GorillaActiveFlow = Conditional<
    GorillaIsHungry,
    GorillaFeedFlow,
    Conditional<GorillaCanUseTools, GorillaToolSocialFlow, GorillaSimpleSocialFlow>,
>;

#[derive(jungle_sdk::Journey)]
pub struct GorillaDayFlow(
    BoundStep<Gorilla, GorillaEvaluateActivityWindow>,
    Conditional<GorillaIsActiveNow, GorillaActiveFlow, BoundStep<Gorilla, GorillaRest>>,
    BoundStep<Gorilla, GorillaTickPerceivedTime>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaYearFlow(
    BoundStep<Gorilla, GorillaBirthday>,
    While<GorillaDaylightRemaining, GorillaDayFlow>,
    BoundStep<Gorilla, GorillaAdvanceAge>,
);

pub struct GorillaLifecycleMetadata;
impl NodeMetadata for GorillaLifecycleMetadata {
    const METADATA: &'static str = "section:gorilla/lifecycle";
}

#[derive(jungle_sdk::Journey)]
pub struct GorillaLifecycleFlow(
    BoundStep<Gorilla, GorillaRest>,
    Transparent<GorillaLifecycleMetadata, GorillaYearFlow>,
    While<GorillaDaylightRemaining, GorillaDayFlow>,
    BoundStep<Gorilla, GorillaRest>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaJourney(
    BoundStep<Gorilla, GorillaBirth>,
    While<GorillaStillGrowing, GorillaLifecycleFlow>,
    BoundStep<Gorilla, GorillaRest>,
);

#[derive(jungle_sdk::Journey)]
pub struct ProbeDayFlow(
    BoundStep<Gorilla, GorillaEvaluateActivityWindow>,
    Conditional<GorillaIsActiveNow, ProbeActiveFlow, BoundStep<Gorilla, GorillaRest>>,
    BoundStep<Gorilla, GorillaTickPerceivedTime>,
);

#[derive(jungle_sdk::Journey)]
pub struct ProbeActiveFlow(BoundStep<Gorilla, ProbeStep>, BoundStep<Gorilla, ProbeStep>);

#[derive(jungle_sdk::Journey)]
pub struct ProbeYearFlow(
    BoundStep<Gorilla, GorillaBirthday>,
    While<GorillaDaylightRemaining, ProbeDayFlow>,
    BoundStep<Gorilla, GorillaAdvanceAge>,
);

pub struct Gorilla;

pub struct ProbeStep;
impl jungle_sdk::types::BoundAct<Gorilla> for ProbeStep {
    type Effect = crate::probe::ProbeEffect;
    type Aspect = jungle_sdk::types::Identity;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &<Gorilla as Animal>::State,
        _input: Self::Input,
    ) -> <Self::Effect as jungle_sdk::types::EffectSchema>::In {
    }

    fn absorb(
        _state: &mut <Gorilla as Animal>::State,
        _output: jungle_sdk::types::EffectCompletion<Self::Effect>,
    ) -> Self::Output {
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(
    BoundStep<Gorilla, GorillaBirth>,
    While<GorillaStillGrowing, ProbeYearFlow>,
);
impl Animal for Gorilla {
    type Id = Id<U0>;
    type Generation = U0;
    type State = State;
    type Seed = TemporalState;
    type Journey = GorillaJourney;
}

impl Observable for Gorilla {
    type Observation = NoopObservation;
}

impl Perturbable for Gorilla {
    type Perturbation = NoopPerturbation;
}

//#[allow(dead_code)]
//fn _assert_gorilla_context_buildflow_bounds() {
//    fn assert_running<R>()
//    where
//        R: jungle_sdk::types::Running<In = (State, ())>,
//    {
//    }
//
//    fn assert_step<S>()
//    where
//        S: jungle_sdk::types::BuildFlowWithContext<
//            (Arc<crate::Zoo>, jungle_sdk::types::DynFlow<State>),
//            Output = (Arc<crate::Zoo>, jungle_sdk::types::DynFlow<State>),
//        >,
//    {
//    }
//
//    assert_step::<BoundStep<Gorilla, GorillaBirth>>();
//    assert_step::<BoundStep<Gorilla, GorillaEvaluateActivityWindow>>();
//    assert_step::<BoundStep<Gorilla, GorillaTickPerceivedTime>>();
//    assert_step::<BoundStep<Gorilla, GorillaBirthday>>();
//    assert_step::<BoundStep<Gorilla, GorillaAdvanceAge>>();
//    assert_step::<BoundStep<Gorilla, GorillaPeelFruit>>();
//    assert_step::<BoundStep<Gorilla, GorillaEat>>();
//    assert_step::<BoundStep<Gorilla, GorillaUseTool>>();
//    assert_step::<BoundStep<Gorilla, GorillaChestBeat>>();
//    assert_step::<BoundStep<Gorilla, GorillaRest>>();
//    assert_step::<BoundStep<Gorilla, GorillaMakeSound>>();
//
//    fn assert_flow<F>()
//    where
//        F: jungle_sdk::types::BuildFlowWithContext<
//            (Arc<crate::Zoo>, jungle_sdk::types::DynFlow<State>),
//            Output = (Arc<crate::Zoo>, jungle_sdk::types::DynFlow<State>),
//        >,
//    {
//    }
//
//    assert_flow::<GorillaFeedFlow>();
//    assert_flow::<GorillaToolSocialFlow>();
//    assert_flow::<GorillaSimpleSocialFlow>();
//    assert_flow::<GorillaActiveFlow>();
//    assert_flow::<GorillaDayFlow>();
//    assert_flow::<GorillaYearFlow>();
//    assert_flow::<GorillaJourney>();
//
//    assert_running::<GorillaFeedFlow>();
//    assert_running::<GorillaToolSocialFlow>();
//    assert_running::<GorillaSimpleSocialFlow>();
//    assert_running::<GorillaActiveFlow>();
//    assert_running::<GorillaDayFlow>();
//    assert_running::<GorillaYearFlow>();
//    assert_running::<GorillaJourney>();
//}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl Animals for Gorilla {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U0, Gorilla>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl Identified for Gorilla {
    type Id = U0;
}

#[allow(dead_code)]
pub fn default_temporal_seed() -> TemporalState {
    TemporalState {
        age: AgeState {
            age_years: 1,
            life_phase: LifePhase::Child,
            growth_percent: 12,
        },
        schedule: ActivitySchedule {
            activity: DailyActivity::Diurnal,
            active_window: CircadianWindow {
                start: PerceivedTimeOfDay::Morning,
                end: PerceivedTimeOfDay::Afternoon,
            },
            rest_window: CircadianWindow {
                start: PerceivedTimeOfDay::Evening,
                end: PerceivedTimeOfDay::Night,
            },
        },
        perception: TimePerception {
            current: PerceivedTimeOfDay::Morning,
            minutes_since_transition: 0,
        },
    }
}
