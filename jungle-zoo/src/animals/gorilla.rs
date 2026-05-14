//! Gorilla state model and lifecycle journey.

use crate::actions;
use crate::state::{
    ActivitySchedule, AgeState, CircadianWindow, DailyActivity, Finger, FingerSet, FruitFlesh,
    FruitMeal, FruitRind, Hand, Hands, LifePhase, Lobe, Nail, NervousSystem, PerceivedTimeOfDay,
    TemporalState, TimePerception, VitalReadings,
};
use jungle_sdk::types::{
    Action, ActionCompletion, Animal, AnimalMember, AnimalObservation, AnimalPerturbation, Animals,
    Condition, Conditional, Id, Identified, Identity, LoopCondition, NodeMetadata, NoopObservation,
    NoopPerturbation, Pulse, StatePick, StatePickMapper, Step, Transparent, While,
};
use jungle_sdk::typosaurus::num::consts::U0;
use jungle_sdk::Optic;

const GORILLA_DAY_LOOPS_PER_YEAR: u16 = 365;

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

//#[cfg(test)]
//mod compile_checks {
//    use super::*;
//    use jungle_sdk::types::{BuildFlowWithContext, DynFlow, FlowActions, Running, Waiting};
//    use std::sync::Arc;
//
//    #[allow(dead_code)]
//    fn assert_running<F: Running<In = (State, ())>>() {}
//
//    #[allow(dead_code)]
//    fn assert_waiting<F: Waiting>() {}
//
//    #[allow(dead_code)]
//    fn assert_flow_actions<F: FlowActions>() {}
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
//        assert_running::<Step<Gorilla, GorillaBirth>>();
//        assert_running::<GorillaFeedFlow>();
//        assert_running::<GorillaToolSocialFlow>();
//        assert_running::<GorillaSimpleSocialFlow>();
//        assert_running::<GorillaActiveFlow>();
//        assert_running::<GorillaDayFlow>();
//        assert_running::<GorillaYearFlow>();
//        assert_running::<GorillaJourney>();
//
//        assert_waiting::<Step<Gorilla, GorillaBirth>>();
//        assert_flow_actions::<Step<Gorilla, GorillaBirth>>();
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
impl Pulse<Gorilla> for GorillaAdvanceAge {
    type Action = actions::AdvanceAge;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.temporal.age.age_years
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let next = output.expect("gorilla age advancement should succeed");
        state.temporal.age = next;
        state.age = u32::from(state.temporal.age.age_years);

        // Each annual cycle begins a new day schedule from morning.
        state.temporal.perception.current = PerceivedTimeOfDay::Morning;
        state.temporal.perception.minutes_since_transition = 0;
    }
}

pub struct GorillaTickPerceivedTime;
impl Pulse<Gorilla> for GorillaTickPerceivedTime {
    type Action = actions::TickPerceivedTime;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        let segment_minutes = if state.temporal.perception.minutes_since_transition % 2 == 0 {
            0
        } else {
            360
        };
        (state.temporal.perception.current, segment_minutes)
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
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
impl Pulse<Gorilla> for GorillaBirthday {
    type Action = actions::CelebrateBirthday;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        state.temporal.age = output.expect("gorilla birthday state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaBirth;
impl Pulse<Gorilla> for GorillaBirth {
    type Action = actions::Birth;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        state.temporal.age = output.expect("gorilla birth state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaEvaluateActivityWindowPick;
impl StatePickMapper<State, actions::EvaluateActivityWindow, (), (), U0>
    for GorillaEvaluateActivityWindowPick
{
    fn emit(state: &State, _input: ()) -> <actions::EvaluateActivityWindow as Action>::In {
        (
            state.temporal.schedule.activity,
            state.temporal.perception.current,
        )
    }

    fn absorb(_state: &mut State, output: ActionCompletion<actions::EvaluateActivityWindow>) {
        let _ = output.expect("gorilla activity-window evaluation should succeed");
    }
}
pub type GorillaEvaluateActivityWindow =
    StatePick<actions::EvaluateActivityWindow, U0, GorillaEvaluateActivityWindowPick>;

pub struct GorillaPeelFruitPick;
impl StatePickMapper<State, actions::PeelFruit, (), (), U0> for GorillaPeelFruitPick {
    fn emit(state: &State, _input: ()) -> <actions::PeelFruit as Action>::In {
        (state.meal.rind.thickness_mm, state.meal.flesh.mass_g)
    }

    fn absorb(state: &mut State, output: ActionCompletion<actions::PeelFruit>) {
        let edible = output.expect("gorilla peel-fruit should succeed");
        state.meal.flesh.mass_g = edible;
    }
}
pub type GorillaPeelFruit = StatePick<actions::PeelFruit, U0, GorillaPeelFruitPick>;

pub struct GorillaEat;
impl Pulse<Gorilla> for GorillaEat {
    type Action = actions::Eat;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let energy = output.expect("gorilla eat should succeed");
        state.vitals.energy = energy;
        state.vitals.is_hungry = energy < 30;
        state.vitals.is_sleepy = energy < 25;
    }
}

pub struct GorillaUseTool;
impl Pulse<Gorilla> for GorillaUseTool {
    type Action = actions::UseTool;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.hands.left.opposable_thumb && state.hands.right.opposable_thumb
    }

    fn absorb(_state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let _note = output.expect("gorilla tool-use should succeed");
    }
}

pub struct GorillaChestBeat;
impl Pulse<Gorilla> for GorillaChestBeat {
    type Action = actions::ChestBeat;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        (
            state.vitals.stress,
            state.hands.left.opposable_thumb && state.hands.right.opposable_thumb,
        )
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        state.vitals.stress = output.expect("gorilla chest-beat should succeed");
    }
}

pub struct GorillaRest;
impl Pulse<Gorilla> for GorillaRest {
    type Action = actions::Rest;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let energy = output.expect("gorilla rest should succeed");
        state.vitals.energy = energy;
        state.vitals.is_hungry = energy < 30;
        state.vitals.is_sleepy = energy < 25;
    }
}

pub struct GorillaMakeSound;
impl Pulse<Gorilla> for GorillaMakeSound {
    type Action = actions::MakeSound;
    type Aspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(state: &State, _input: Self::Arg) -> <Self::Action as Action>::In {
        let kind = match state.temporal.perception.current {
            PerceivedTimeOfDay::Morning => "morning call",
            PerceivedTimeOfDay::Afternoon => "contact hoot",
            PerceivedTimeOfDay::Evening => "group regroup",
            PerceivedTimeOfDay::Night => "quiet rustle",
        };
        (kind.to_owned(), state.vitals.stress)
    }

    fn absorb(_state: &mut State, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let _signal = output.expect("gorilla vocalization should succeed");
    }
}

#[derive(jungle_sdk::Journey)]
pub struct GorillaFeedFlow(
    Step<Gorilla, GorillaPeelFruit>,
    Step<Gorilla, GorillaEat>,
    Step<Gorilla, GorillaRest>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaToolSocialFlow(
    Step<Gorilla, GorillaUseTool>,
    Step<Gorilla, GorillaChestBeat>,
    Step<Gorilla, GorillaMakeSound>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaSimpleSocialFlow(Step<Gorilla, GorillaMakeSound>, Step<Gorilla, GorillaRest>);

pub type GorillaActiveFlow = Conditional<
    GorillaIsHungry,
    GorillaFeedFlow,
    Conditional<GorillaCanUseTools, GorillaToolSocialFlow, GorillaSimpleSocialFlow>,
>;

#[derive(jungle_sdk::Journey)]
pub struct GorillaDayFlow(
    Step<Gorilla, GorillaEvaluateActivityWindow>,
    Conditional<GorillaIsActiveNow, GorillaActiveFlow, Step<Gorilla, GorillaRest>>,
    Step<Gorilla, GorillaTickPerceivedTime>,
);

#[derive(jungle_sdk::Journey)]
pub struct GorillaYearFlow(
    Step<Gorilla, GorillaBirthday>,
    While<GorillaDaylightRemaining, GorillaDayFlow>,
    Step<Gorilla, GorillaAdvanceAge>,
);

pub struct GorillaLifecycleMetadata;
impl NodeMetadata for GorillaLifecycleMetadata {
    const METADATA: &'static str = "section:gorilla/lifecycle";
}

type GorillaLifecycleFlow = Transparent<GorillaLifecycleMetadata, GorillaYearFlow>;

#[derive(jungle_sdk::Journey)]
pub struct GorillaJourney(
    Step<Gorilla, GorillaBirth>,
    While<GorillaStillGrowing, GorillaLifecycleFlow>,
);

#[derive(jungle_sdk::Journey)]
pub struct ProbeDayFlow(
    Step<Gorilla, GorillaEvaluateActivityWindow>,
    Conditional<GorillaIsActiveNow, ProbeActiveFlow, Step<Gorilla, GorillaRest>>,
    Step<Gorilla, GorillaTickPerceivedTime>,
);

#[derive(jungle_sdk::Journey)]
pub struct ProbeActiveFlow(Step<Gorilla, ProbeStep>, Step<Gorilla, ProbeStep>);

#[derive(jungle_sdk::Journey)]
pub struct ProbeYearFlow(
    Step<Gorilla, GorillaBirthday>,
    While<GorillaDaylightRemaining, ProbeDayFlow>,
    Step<Gorilla, GorillaAdvanceAge>,
);

pub struct Gorilla;
impl AnimalMember for Gorilla {}

pub struct ProbeStep;
impl jungle_sdk::types::Pulse<Gorilla> for ProbeStep {
    type Action = crate::probe::ProbeAction;
    type Aspect = jungle_sdk::types::Identity;
    type Arg = ();
    type Ret = ();

    fn emit(
        _state: &<Gorilla as Animal>::State,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_sdk::types::Action>::In {
    }

    fn absorb(
        _state: &mut <Gorilla as Animal>::State,
        _output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::Ret {
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(
    Step<Gorilla, GorillaBirth>,
    While<GorillaStillGrowing, ProbeYearFlow>,
);
impl Animal for Gorilla {
    type Id = Id<U0>;
    type Generation = U0;
    type State = State;
    type Seed = TemporalState;
    type Journey = GorillaJourney;
}

impl AnimalObservation for Gorilla {
    type Bridge = NoopObservation;
}

impl AnimalPerturbation for Gorilla {
    type Bridge = NoopPerturbation;
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
//    assert_step::<Step<Gorilla, GorillaBirth>>();
//    assert_step::<Step<Gorilla, GorillaEvaluateActivityWindow>>();
//    assert_step::<Step<Gorilla, GorillaTickPerceivedTime>>();
//    assert_step::<Step<Gorilla, GorillaBirthday>>();
//    assert_step::<Step<Gorilla, GorillaAdvanceAge>>();
//    assert_step::<Step<Gorilla, GorillaPeelFruit>>();
//    assert_step::<Step<Gorilla, GorillaEat>>();
//    assert_step::<Step<Gorilla, GorillaUseTool>>();
//    assert_step::<Step<Gorilla, GorillaChestBeat>>();
//    assert_step::<Step<Gorilla, GorillaRest>>();
//    assert_step::<Step<Gorilla, GorillaMakeSound>>();
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
