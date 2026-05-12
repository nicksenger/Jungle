//! Gorilla state model and lifecycle journey.

use crate::actions;
use crate::state::{
    ActivitySchedule, AgeState, CircadianWindow, DailyActivity, Finger, FingerSet, FruitFlesh,
    FruitMeal, FruitRind, Hand, Hands, LifePhase, Lobe, Nail, NervousSystem, PerceivedTimeOfDay,
    TemporalState, TimePerception, VitalReadings,
};
use jungle_sdk::types::{
    Action, ActionCompletion, Animal, AnimalMember, AnimalObservation, AnimalPerturbation, Animals,
    Condition, Conditional, Id, Identified, Identity, LoopCondition, NoopObservation,
    NoopPerturbation, Pulse, Step, While,
};
use jungle_sdk::typosaurus::num::consts::U0;
use jungle_sdk::Journey;
use jungle_sdk::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
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
    type CarryIn = ();

    fn should_continue(state: &State) -> bool {
        state.temporal.age.life_phase != LifePhase::Adult
    }
}

pub struct GorillaDaylightRemaining;
impl LoopCondition<State> for GorillaDaylightRemaining {
    type CarryIn = ();

    fn should_continue(state: &State) -> bool {
        !matches!(state.temporal.perception.current, PerceivedTimeOfDay::Night)
    }
}

pub struct GorillaIsActiveNow;
impl Condition<(State, bool)> for GorillaIsActiveNow {
    fn choose((_state, is_active): &(State, bool)) -> bool {
        *is_active
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
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.temporal.age.age_years
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
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
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        (
            state.temporal.perception.current,
            state.temporal.perception.minutes_since_transition,
        )
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let next = output.expect("gorilla perceived-time tick should succeed");
        state.temporal.perception = next;
    }
}

pub struct GorillaBirthday;
impl Pulse<Gorilla> for GorillaBirthday {
    type Action = actions::CelebrateBirthday;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        state.temporal.age = output.expect("gorilla birthday state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaBirth;
impl Pulse<Gorilla> for GorillaBirth {
    type Action = actions::Birth;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.temporal.age.clone()
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        state.temporal.age = output.expect("gorilla birth state refresh should succeed");
        state.age = u32::from(state.temporal.age.age_years);
    }
}

pub struct GorillaEvaluateActivityWindow;
impl Pulse<Gorilla> for GorillaEvaluateActivityWindow {
    type Action = actions::EvaluateActivityWindow;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = bool;

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        (
            state.temporal.schedule.activity,
            state.temporal.perception.current,
        )
    }

    fn absorb(_state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        output.expect("gorilla activity-window evaluation should succeed")
    }
}

pub struct GorillaPeelFruit;
impl Pulse<Gorilla> for GorillaPeelFruit {
    type Action = actions::PeelFruit;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        (state.meal.rind.thickness_mm, state.meal.flesh.mass_g)
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let edible = output.expect("gorilla peel-fruit should succeed");
        state.meal.flesh.mass_g = edible;
    }
}

pub struct GorillaEat;
impl Pulse<Gorilla> for GorillaEat {
    type Action = actions::Eat;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
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
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.hands.left.opposable_thumb && state.hands.right.opposable_thumb
    }

    fn absorb(_state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let _note = output.expect("gorilla tool-use should succeed");
    }
}

pub struct GorillaChestBeat;
impl Pulse<Gorilla> for GorillaChestBeat {
    type Action = actions::ChestBeat;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        (
            state.vitals.stress,
            state.hands.left.opposable_thumb && state.hands.right.opposable_thumb,
        )
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        state.vitals.stress = output.expect("gorilla chest-beat should succeed");
    }
}

pub struct GorillaRest;
impl Pulse<Gorilla> for GorillaRest {
    type Action = actions::Rest;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        state.vitals.energy
    }

    fn absorb(state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
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
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &State, _input: Self::CarryIn) -> <Self::Action as Action>::In {
        let kind = match state.temporal.perception.current {
            PerceivedTimeOfDay::Morning => "morning call",
            PerceivedTimeOfDay::Afternoon => "contact hoot",
            PerceivedTimeOfDay::Evening => "group regroup",
            PerceivedTimeOfDay::Night => "quiet rustle",
        };
        (kind.to_owned(), state.vitals.stress)
    }

    fn absorb(_state: &mut State, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let _signal = output.expect("gorilla vocalization should succeed");
    }
}

#[derive(Journey)]
pub struct GorillaFeedFlow(
    Step<Gorilla, GorillaPeelFruit>,
    Step<Gorilla, GorillaEat>,
    Step<Gorilla, GorillaRest>,
);

#[derive(Journey)]
pub struct GorillaToolSocialFlow(
    Step<Gorilla, GorillaUseTool>,
    Step<Gorilla, GorillaChestBeat>,
    Step<Gorilla, GorillaMakeSound>,
);

#[derive(Journey)]
pub struct GorillaSimpleSocialFlow(Step<Gorilla, GorillaMakeSound>, Step<Gorilla, GorillaRest>);

pub type GorillaActiveFlow = Conditional<
    GorillaIsHungry,
    GorillaFeedFlow,
    Conditional<GorillaCanUseTools, GorillaToolSocialFlow, GorillaSimpleSocialFlow>,
>;

#[derive(Journey)]
pub struct GorillaDayFlow(
    Step<Gorilla, GorillaEvaluateActivityWindow>,
    Conditional<GorillaIsActiveNow, GorillaActiveFlow, Step<Gorilla, GorillaRest>>,
    Step<Gorilla, GorillaTickPerceivedTime>,
);

#[derive(Journey)]
pub struct GorillaYearFlow(
    Step<Gorilla, GorillaBirthday>,
    While<GorillaDaylightRemaining, GorillaDayFlow>,
    Step<Gorilla, GorillaAdvanceAge>,
);

#[derive(Journey)]
pub struct GorillaJourney(
    Step<Gorilla, GorillaBirth>,
    While<GorillaStillGrowing, GorillaYearFlow>,
);

pub struct Gorilla;
impl AnimalMember for Gorilla {}

impl Animal for Gorilla {
    type Id = Id<U0>;
    type Generation = U0;
    type State = State;
    type Seed = TemporalState;
    type Journey = GorillaJourney;
}

impl AnimalObservation for Gorilla {
    type Adapter = NoopObservation;
}

impl AnimalPerturbation for Gorilla {
    type Adapter = NoopPerturbation;
}

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
