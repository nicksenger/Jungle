use jungle_sdk::types::{
    Act, ActionSpec, BindAnimal, Effect, EffectCompletion, Identity, ManualExecutor, Step, UStep,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::{U0, U40, U41, U42, U43, U44};

struct TemplateAddEffect;
impl<J> Effect<J> for TemplateAddEffect {
    type Id = jungle_sdk::types::Id<U40>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}

struct TemplateCommitEffect;
impl<J> Effect<J> for TemplateCommitEffect {
    type Id = jungle_sdk::types::Id<U41>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

struct AddOneSpec;
struct CommitSpec;

impl ActionSpec for AddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: jungle_sdk::types::Animal> = GenericAddOne<A>;
}

impl ActionSpec for CommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: jungle_sdk::types::Animal> = GenericCommit<A>;
}

#[derive(jungle_sdk::FlowTemplate)]
struct TemplateFlow(UStep<AddOneSpec>, UStep<CommitSpec>);

#[derive(jungle_sdk::Journey)]
struct CounterJourney(
    Step<CounterAnimal, CounterAddOne>,
    Step<CounterAnimal, CounterCommit>,
);

#[derive(jungle_sdk::Journey)]
struct LedgerJourney(
    Step<LedgerAnimal, LedgerAddOne>,
    Step<LedgerAnimal, LedgerCommit>,
);

struct CounterAnimal;
impl jungle_sdk::types::Animal for CounterAnimal {
    type Id = jungle_sdk::types::Id<U42>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = CounterJourney;
}

impl jungle_sdk::types::Observable for CounterAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for CounterAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for CounterAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U42, CounterAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for CounterAnimal {
    type Id = U42;
}

struct LedgerAnimal;
impl jungle_sdk::types::Animal for LedgerAnimal {
    type Id = jungle_sdk::types::Id<U43>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = LedgerJourney;
}

impl jungle_sdk::types::Observable for LedgerAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for LedgerAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for LedgerAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U43, LedgerAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for LedgerAnimal {
    type Id = U43;
}

struct CounterAddOne;
impl Act<CounterAnimal> for CounterAddOne {
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 1
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("counter add step should succeed");
        *state = value;
        value
    }
}

struct LedgerAddOne;
impl Act<LedgerAnimal> for LedgerAddOne {
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 10
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("ledger add step should succeed");
        *state = value;
        value
    }
}

struct CounterCommit;
impl Act<CounterAnimal> for CounterCommit {
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("counter commit step should succeed");
        *state = value;
        value
    }
}

struct LedgerCommit;
impl Act<LedgerAnimal> for LedgerCommit {
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state - input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("ledger commit step should succeed");
        *state = value;
        value
    }
}

struct GenericAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericAddOne<A>
where
    A: jungle_sdk::types::Animal<State = i32>,
{
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 1
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("generic add step should succeed");
        *state = value;
        value
    }
}

struct GenericCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericCommit<A>
where
    A: jungle_sdk::types::Animal<State = i32>,
{
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("generic commit step should succeed");
        *state = value;
        value
    }
}

#[test]
fn template_binding_executes_with_animal_specific_actions() {
    let mut counter = ManualExecutor::<CounterAnimal>::new(0);
    let counter_request_1: i32 = counter
        .next_request_typed::<_, i32>(3)
        .expect("counter first request");
    assert_eq!(counter_request_1, 4);
    let counter_emitted_1: i32 = counter
        .complete_typed::<i32, (), i32>(Ok(5))
        .expect("counter first completion");
    assert_eq!(counter_emitted_1, 5);

    let counter_request_2: i32 = counter
        .next_request_typed::<_, i32>(2)
        .expect("counter second request");
    assert_eq!(counter_request_2, 7);
    let counter_emitted_2: i32 = counter
        .complete_typed::<i32, (), i32>(Ok(7))
        .expect("counter second completion");
    assert_eq!(counter_emitted_2, 7);
    assert_eq!(counter.into_state(), 7);

    let mut ledger = ManualExecutor::<LedgerAnimal>::new(0);
    let ledger_request_1: i32 = ledger
        .next_request_typed::<_, i32>(3)
        .expect("ledger first request");
    assert_eq!(ledger_request_1, 13);
    let ledger_emitted_1: i32 = ledger
        .complete_typed::<i32, (), i32>(Ok(20))
        .expect("ledger first completion");
    assert_eq!(ledger_emitted_1, 20);

    let ledger_request_2: i32 = ledger
        .next_request_typed::<_, i32>(2)
        .expect("ledger second request");
    assert_eq!(ledger_request_2, 18);
    let ledger_emitted_2: i32 = ledger
        .complete_typed::<i32, (), i32>(Ok(18))
        .expect("ledger second completion");
    assert_eq!(ledger_emitted_2, 18);
    assert_eq!(ledger.into_state(), 18);
}

#[test]
fn template_binding_preserves_step_shape_after_binding() {
    type CounterBound = <TemplateFlow as BindAnimal<CounterAnimal>>::Bound;
    type LedgerBound = <TemplateFlow as BindAnimal<LedgerAnimal>>::Bound;
    type ExpectedCounter = jungle_sdk::typosaurus::collections::list::List<(
        Step<CounterAnimal, GenericAddOne<CounterAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            Step<CounterAnimal, GenericCommit<CounterAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;
    type ExpectedLedger = jungle_sdk::typosaurus::collections::list::List<(
        Step<LedgerAnimal, GenericAddOne<LedgerAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            Step<LedgerAnimal, GenericCommit<LedgerAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;

    let _counter_step_1: Step<CounterAnimal, CounterAddOne> = Step::new();
    let _counter_step_2: Step<CounterAnimal, CounterCommit> = Step::new();
    let _ledger_step_1: Step<LedgerAnimal, LedgerAddOne> = Step::new();
    let _ledger_step_2: Step<LedgerAnimal, LedgerCommit> = Step::new();

    assert_type_eq!(CounterBound, ExpectedCounter);
    assert_type_eq!(LedgerBound, ExpectedLedger);
}

struct BoundTemplateAnimal;
impl jungle_sdk::types::Animal for BoundTemplateAnimal {
    type Id = jungle_sdk::types::Id<U44>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<BoundTemplateAnimal>>::Bound;
}

#[test]
fn template_binding_bound_journey_is_executor_ready() {
    let mut executor = ManualExecutor::<BoundTemplateAnimal>::new(0);

    let req_1: i32 = executor
        .next_request_typed::<_, i32>(3)
        .expect("first bound request");
    assert_eq!(req_1, 4);
    let out_1: i32 = executor
        .complete_typed::<i32, (), i32>(Ok(6))
        .expect("first bound completion");
    assert_eq!(out_1, 6);

    let req_2: i32 = executor
        .next_request_typed::<_, i32>(2)
        .expect("second bound request");
    assert_eq!(req_2, 8);
    let out_2: i32 = executor
        .complete_typed::<i32, (), i32>(Ok(8))
        .expect("second bound completion");
    assert_eq!(out_2, 8);
    assert_eq!(executor.into_state(), 8);
}
