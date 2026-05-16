mod behavior;
mod error;
mod executor;
mod journey;
mod meta;
mod sleep;
mod transport;
mod view;
pub use behavior::{Absorb, Emit, Fuse};
pub use behavior::{
    AbsorbFn, AbsorbMapper, EmitFn, EmitMapper, FocusedStep, IdentityStep, PassthroughEmit,
    UnitEmit,
};
pub use behavior::{
    Act, Aspect, Effect, EffectCompletion, EffectRequest, Identity, StateCarrier, StateLens, Step,
};
pub use behavior::{FocusedAbsorb, FocusedEmit};
pub use error::Error;
pub use executor::{
    ArgputForState, BuildFlow, BuildFlowWithContext, ContextExecutor, ContextualTypedErasedStep,
    DynFlow, ErasedStep, ExecutableEffectRequest, Executor, ExecutorError, ExecutorFlow,
    JungleDynFlow, JungleDynFlowContext, ManualExecutor, TypedErasedStep,
};
use inception::*;
pub use journey::Journey;
pub use meta::Id;
pub use meta::{
    AllFrom, AnimalEffectMembers, AnimalEffectSet, AnimalEffectsCompatible, AnimalIdValue,
    AnimalMember, AnimalSet, AnimalStates, AnimalStatesCompatible, AnimalVersion,
    AnimalVersionIdentitiesUnique, AnimalVersions, EffectMember, EffectSet, Generations,
    GenerationsForAnimals, HighestGeneration, HighestGenerationForAnimals, StripAnimalHeaders,
    StripEffectHeaders, WithEffectFor,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
pub use sleep::{Sleep, SleepError, SleepStep};
use std::marker::PhantomData;
pub use transport::{
    BackendError, JourneyEvent, JourneyStatus, JourneyUpdateEvent, RunnerOut, RunnerUpdateOut,
    WireIn, WireOut, Work,
};
pub use transport::{ClaimedAnimalPerturbation, OwnerWake, SupportedAnimal};
use typosaurus::collections::list::{self, List as TList};
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U0;
pub use view::{BuildJourneyAst, JourneyAst, JourneyAstSource, JungleJourneyAst};

/// A tagged union over two possible outputs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

/// Input adapter used by [`Conditional`] to pick a branch and forward carry input.
pub trait ConditionInput {
    fn choose_left(&self) -> bool;
}

impl<In> ConditionInput for (bool, In) {
    fn choose_left(&self) -> bool {
        self.0
    }
}

impl<In> ConditionInput for (Vec<bool>, In) {
    fn choose_left(&self) -> bool {
        self.0.last().copied().unwrap_or(false)
    }
}

/// Input adapter used by [`While`] to decide loop continuation and forward carry input.
pub trait LoopInput {
    type Arg;

    fn into_loop(self) -> (bool, Self::Arg);
}

impl<In> LoopInput for (bool, In) {
    type Arg = In;

    fn into_loop(self) -> (bool, Self::Arg) {
        self
    }
}

/// Legacy predicate hook for [`Conditional`], retained as a marker for type-level flow shape.
pub trait Condition<In> {
    fn choose(input: &In) -> bool;
}

/// Legacy predicate hook for [`While`], retained as a marker for type-level flow shape.
pub trait LoopCondition<State> {
    type Arg;

    fn should_continue(state: &State) -> bool;
}

/// Property used to opt-in a state type to field-index lenses.
pub struct JungleOptic;
impl Property for JungleOptic {}

/// Marker trait proving a type has opted into [`JungleOptic`].
pub trait Optic: Inception<JungleOptic, False> {}
impl<T> Optic for T where T: Inception<JungleOptic, False> {}

/// A flow combinator that chooses either `L` or `R` at runtime.
pub struct Conditional<P, L, R, M = NoMetadata>(PhantomData<fn() -> (P, L, R, M)>);

/// A flow combinator that repeatedly executes `F` while `C` is true.
pub struct While<C, F, M = NoMetadata>(PhantomData<fn() -> (C, F, M)>);

/// A flow combinator that runs two activities and resolves to whichever completes first.
pub struct Select<L, R, M = NoMetadata>(PhantomData<fn() -> (L, R, M)>);

/// A flow combinator that runs two activities and resolves when both complete.
pub struct Join<L, R, M = NoMetadata>(PhantomData<fn() -> (L, R, M)>);

/// Type-level metadata marker for flow nodes.
pub trait NodeMetadata {
    const METADATA: &'static str = "";
}

/// Empty metadata marker used by default when no metadata is provided.
pub struct NoMetadata;

impl NodeMetadata for NoMetadata {}

/// A no-op boundary wrapper used for organization and metadata anchoring.
pub struct Transparent<M, F>(PhantomData<fn() -> (M, F)>);

/// A collection of `Animals` which act together as a system.
pub trait Ecosystem {
    const NAME: &'static str;
    type Animals;
}

/// A living animal within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// A type-level generation for this Animal.
    ///
    /// New journey starts should target the latest generation for a given `Id`,
    /// while in-flight journeys continue to resume on their original generation.
    type Generation;

    /// The state of this `Animal` at any given time.
    type State: Default;

    /// Serializable seed used to initialize the first step input of this animal's journey.
    type Seed: Serialize + DeserializeOwned;

    /// The fundamental behavior of this Animal.
    type Journey;
}

/// Bridge invoked by executors/runners to optionally snapshot appearance bytes.
pub trait ObservationBridge<A: Animal> {
    fn snapshot(state: &A::State) -> Result<Option<Vec<u8>>, ExecutorError>;
}

/// Default no-op observation bridge for animals without appearance snapshots.
pub struct NoopObservation;

impl<A> ObservationBridge<A> for NoopObservation
where
    A: Animal,
{
    fn snapshot(_state: &A::State) -> Result<Option<Vec<u8>>, ExecutorError> {
        Ok(None)
    }
}

/// Observation bridge that maps [`Observe`] into serialized snapshot bytes.
pub struct ObserveObservation;

impl<A> ObservationBridge<A> for ObserveObservation
where
    A: Observe,
    A::Appearance: Serialize,
{
    fn snapshot(state: &A::State) -> Result<Option<Vec<u8>>, ExecutorError> {
        let appearance = <A as Observe>::observe(state);
        let bytes = postcard::to_allocvec(&appearance)
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))?;
        Ok(Some(bytes))
    }
}

/// Per-animal binding that selects how appearance snapshots are produced.
pub trait AnimalObservation: Animal + Sized {
    type Bridge: ObservationBridge<Self>;
}

/// Bridge invoked by executors/runners to optionally apply perturbation payloads.
pub trait PerturbationBridge<A: Animal> {
    fn enabled() -> bool {
        true
    }

    fn apply(state: &mut A::State, payload: &[u8]) -> Result<bool, ExecutorError>;
}

/// Default no-op perturbation bridge for animals without perturb handlers.
pub struct NoopPerturbation;

impl<A> PerturbationBridge<A> for NoopPerturbation
where
    A: Animal,
{
    fn enabled() -> bool {
        false
    }

    fn apply(_state: &mut A::State, _payload: &[u8]) -> Result<bool, ExecutorError> {
        Ok(false)
    }
}

/// Perturbation bridge that maps [`Perturb`] from serialized stimuli.
pub struct TraitPerturbation;

impl<A> PerturbationBridge<A> for TraitPerturbation
where
    A: Perturb,
    A::Stimulus: DeserializeOwned,
{
    fn apply(state: &mut A::State, payload: &[u8]) -> Result<bool, ExecutorError> {
        let stimulus: A::Stimulus = postcard::from_bytes(payload)
            .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
        <A as Perturb>::perturb(state, stimulus);
        Ok(true)
    }
}

/// Per-animal binding that selects how perturbation payloads are applied.
pub trait AnimalPerturbation: Animal + Sized {
    type Bridge: PerturbationBridge<Self>;
}

#[inception(property = Ident, types)]
pub trait Identified {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        merge_variant = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        join = TList<(U0, <Fields as Identified>::Id)>
    )]
    type Id;
}

/// Any collection of [`Animal`]s with a flat type-level list of members.
#[inception(property = JungleAnimals, types)]
pub trait Animals {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        merge_variant = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Animals>::List)> where { Self: Identified }
    )]
    type List;
}

/// Any collection of [`Effect`]s with a flat type-level list of members.
#[inception(property = JungleEffects, types)]
pub trait Effects {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Effects>::List, <Tail as Effects>::List)>,
        merge_variant = TList<(<Head as Effects>::List, <Tail as Effects>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Effects>::List)> where { Self: Identified }
    )]
    type List;
}

/// A collection of [`Effect`]s extractable from an executable workflow.
#[inception(property = JungleFlow, types)]
pub trait FlowEffects {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as FlowEffects>::List, <Tail as FlowEffects>::List)>,
        merge_variant = TList<(<Head as FlowEffects>::List, <Tail as FlowEffects>::List)>,
        join = TList<(Node<U0, ()>, <Fields as FlowEffects>::List)>
    )]
    type List;
}

/// Leaf-level hook used by [`TraverseWith`] at `Step` nodes.
pub trait TraverseStep<Step> {
    type Output;
}

/// Leaf-level hook used by [`ReplaceWith`] at `Step` nodes.
pub trait ReplaceStep<Step> {
    type Output;
}

/// Node-level hook used by [`ReplaceNodesWith`] for section/subtree replacement.
pub trait ReplaceNode<Node> {
    type Output;
}

/// Directional helper that rewrites `Step<Animal, Left>` to `Step<Animal, Right>`.
pub struct SwapLR<Left, Right>(PhantomData<fn() -> (Left, Right)>);

/// Directional helper that rewrites `Step<Animal, Right>` to `Step<Animal, Left>`.
pub struct SwapRL<Left, Right>(PhantomData<fn() -> (Left, Right)>);

/// Directional helper alias for node replacement from `Left` to `Right`.
pub type SwapNodeLR<Left, Right> = SwapLR<Left, Right>;

/// Directional helper alias for node replacement from `Right` to `Left`.
pub type SwapNodeRL<Left, Right> = SwapRL<Left, Right>;

impl<A, Left, Right> ReplaceStep<Step<A, Left>> for SwapLR<Left, Right>
where
    A: Animal,
    Left: Act<A>,
    Right: Act<A>,
{
    type Output = Step<A, Right>;
}

impl<A, Left, Right> ReplaceStep<Step<A, Right>> for SwapRL<Left, Right>
where
    A: Animal,
    Left: Act<A>,
    Right: Act<A>,
{
    type Output = Step<A, Left>;
}

impl<Left, Right> ReplaceNode<Left> for SwapLR<Left, Right> {
    type Output = Right;
}

impl<Left, Right> ReplaceNode<Right> for SwapRL<Left, Right> {
    type Output = Left;
}

/// Inception property that normalizes/walks a flow's type structure.
#[inception(property = JungleTraverseFlow, types)]
pub trait TraverseFlow {
    #[induce(
        base = list::Empty,
        merge = TList<(
            <Head as TraverseFlow>::Output,
            <Tail as TraverseFlow>::Output
        )>,
        merge_variant = TList<(
            <Head as TraverseFlow>::Output,
            <Tail as TraverseFlow>::Output
        )>,
        join = <Fields as TraverseFlow>::Output
    )]
    type Output;
}

/// Inception property that normalizes/walks a flow's type structure.
#[inception(property = JungleReplaceFlow, types)]
pub trait ReplaceFlow {
    #[induce(
        base = list::Empty,
        merge = TList<(
            <Head as ReplaceFlow>::Output,
            <Tail as ReplaceFlow>::Output
        )>,
        merge_variant = TList<(
            <Head as ReplaceFlow>::Output,
            <Tail as ReplaceFlow>::Output
        )>,
        join = <Fields as ReplaceFlow>::Output
    )]
    type Output;
}

/// Applies a traversal operator across a normalized flow type.
pub trait TraverseWith<Traversal> {
    type Output;
}

/// Applies a replacement operator across a normalized flow type.
pub trait ReplaceWith<Replacer> {
    type Output;
}

/// Applies a node-level replacer across a normalized flow type.
pub trait ReplaceNodesWith<Replacer> {
    type Output;
}

impl<Traversal> TraverseWith<Traversal> for list::Empty {
    type Output = list::Empty;
}

impl<Replacer> ReplaceWith<Replacer> for list::Empty {
    type Output = list::Empty;
}

impl<Replacer> ReplaceNodesWith<Replacer> for list::Empty {
    type Output = list::Empty;
}

impl<Head, Tail, Traversal> TraverseWith<Traversal> for TList<(Head, Tail)>
where
    Head: TraverseWith<Traversal>,
    Tail: TraverseWith<Traversal>,
{
    type Output = TList<(
        <Head as TraverseWith<Traversal>>::Output,
        <Tail as TraverseWith<Traversal>>::Output,
    )>;
}

impl<Head, Tail, Replacer> ReplaceWith<Replacer> for TList<(Head, Tail)>
where
    Head: ReplaceWith<Replacer>,
    Tail: ReplaceWith<Replacer>,
{
    type Output = TList<(
        <Head as ReplaceWith<Replacer>>::Output,
        <Tail as ReplaceWith<Replacer>>::Output,
    )>;
}

impl<Head, Tail, Replacer> ReplaceNodesWith<Replacer> for TList<(Head, Tail)>
where
    Head: ReplaceNodesWith<Replacer>,
    Tail: ReplaceNodesWith<Replacer>,
{
    type Output = TList<(
        <Head as ReplaceNodesWith<Replacer>>::Output,
        <Tail as ReplaceNodesWith<Replacer>>::Output,
    )>;
}

pub type Traversed<Flow, Traversal> =
    <<Flow as TraverseFlow>::Output as TraverseWith<Traversal>>::Output;
pub type Replace<Flow, Replacer> = <<Flow as ReplaceFlow>::Output as ReplaceWith<Replacer>>::Output;
pub type ReplaceNodes<Flow, Replacer> =
    <Replacer as ReplaceNode<<Flow as ReplaceFlow>::Output>>::Output;

/// Output produced by a yielding phase.
pub struct Yielded<Y, A> {
    pub output: Y,
    pub awaiting: A,
}

/// Output produced by an awaiting phase.
pub struct Awaited<A, Y> {
    pub output: A,
    pub yielding: Y,
}

/// Marks a composed tail as the next awaiting continuation.
pub struct AwaitingTail<T>(pub T);

/// Marks a composed tail as the next yielding continuation.
pub struct YieldingTail<T>(pub T);

impl<T> AwaitingTail<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> YieldingTail<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// A phase that runs until it emits an output, then transitions to an
/// awaiting phase that expects the next external input.
#[inception(property = JungleRunning, signature(input = In, output = Out))]
pub trait Running {
    /// Input used to start/resume this yielding phase.
    type In;

    /// A typed transition frame, typically
    /// `Yielded<Output, AwaitingTail<NextAwaitingState>>`.
    type Out;

    /// Run until this phase yields output and transitions to an awaiting phase.
    fn run(input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }

    fn merge<H, R>(_l: H, r: R, input: Self::In) -> Yielded<<H as Running>::Out, AwaitingTail<R>>
    where
        H: Running<In = Self::In>,
    {
        let _ = _l;
        Yielded {
            output: <H as Running>::run(input),
            awaiting: AwaitingTail(r),
        }
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, input: Self::In) -> Self::In {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, input: Self::In) -> <F as Running>::Out
    where
        F: Running<In = Self::In>,
    {
        let _ = fields;
        <F as Running>::run(input)
    }
}

impl<T> __inception_running::FieldsInput<__inception_running::Wrap<T>> for ()
where
    T: Fields,
    <T as Fields>::Head: Field,
    <<T as Fields>::Head as Field>::Content: Running,
{
    type In = <<<T as Fields>::Head as Field>::Content as Running>::In;
}

impl<T> Running for __inception_running::Wrap<T>
where
    (): __inception_running::FieldsInput<__inception_running::Wrap<T>>,
    __inception_running::Wrap<T>: IsPrimitive<JungleRunning, Is = False>,
    __inception_running::Wrap<T>: __inception_running::Inductive<
        False,
        <() as __inception_running::FieldsInput<__inception_running::Wrap<T>>>::In,
    >,
{
    type In = <() as __inception_running::FieldsInput<__inception_running::Wrap<T>>>::In;
    type Out = <__inception_running::Wrap<T> as __inception_running::Inductive<
        False,
        <() as __inception_running::FieldsInput<__inception_running::Wrap<T>>>::In,
    >>::Ret;

    fn run(input: Self::In) -> Self::Out {
        <Self as __inception_running::Inductive<False, Self::In>>::run(input)
    }
}

/// A phase that awaits an external input, then transitions back to a yielding
/// phase.
#[inception(property = JungleWaiting, signature(input = In, output = Out))]
pub trait Waiting {
    /// External input expected at this await point.
    type In;

    /// A typed transition frame, typically
    /// `Awaited<Output, YieldingTail<NextYieldingState>>`.
    type Out;

    /// Accept awaited input and transition to the next yielding phase.
    fn accept(input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }

    fn merge<H, R>(_l: H, r: R, input: Self::In) -> Awaited<<H as Waiting>::Out, YieldingTail<R>>
    where
        H: Waiting<In = Self::In>,
    {
        let _ = _l;
        Awaited {
            output: <H as Waiting>::accept(input),
            yielding: YieldingTail(r),
        }
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, input: Self::In) -> Self::In {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, input: Self::In) -> <F as Waiting>::Out
    where
        F: Waiting<In = Self::In>,
    {
        let _ = fields;
        <F as Waiting>::accept(input)
    }
}

impl<T> Waiting for AwaitingTail<T>
where
    T: Waiting,
{
    type In = <T as Waiting>::In;
    type Out = <T as Waiting>::Out;

    fn accept(input: Self::In) -> Self::Out {
        <T as Waiting>::accept(input)
    }
}

impl<T> Running for YieldingTail<T>
where
    T: Running,
{
    type In = <T as Running>::In;
    type Out = <T as Running>::Out;

    fn run(input: Self::In) -> Self::Out {
        <T as Running>::run(input)
    }
}

#[primitive(property = JungleRunning)]
impl<P, L, R, M> Running for Conditional<P, L, R, M>
where
    L: Running,
    R: Running<In = L::In>,
    P: Condition<L::In>,
{
    type In = L::In;
    type Out = Either<L::Out, R::Out>;

    fn run(input: Self::In) -> Self::Out {
        if <P as Condition<L::In>>::choose(&input) {
            Either::Left(<L as Running>::run(input))
        } else {
            Either::Right(<R as Running>::run(input))
        }
    }
}

#[primitive(property = JungleWaiting)]
impl<P, L, R, M> Waiting for Conditional<P, L, R, M>
where
    L: Waiting,
    R: Waiting,
{
    type In = Either<L::In, R::In>;
    type Out = Either<L::Out, R::Out>;

    fn accept(input: Self::In) -> Self::Out {
        match input {
            Either::Left(input) => Either::Left(<L as Waiting>::accept(input)),
            Either::Right(input) => Either::Right(<R as Waiting>::accept(input)),
        }
    }
}

#[primitive(property = JungleFlow)]
impl<P, L, R, M> FlowEffects for Conditional<P, L, R, M>
where
    L: FlowEffects,
    R: FlowEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<P, L, R, M> TraverseFlow for Conditional<P, L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Conditional<P, <L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

impl<P, L, R, M, Traversal> TraverseWith<Traversal> for Conditional<P, L, R, M>
where
    L: TraverseWith<Traversal>,
    R: TraverseWith<Traversal>,
{
    type Output = Conditional<
        P,
        <L as TraverseWith<Traversal>>::Output,
        <R as TraverseWith<Traversal>>::Output,
        M,
    >;
}

#[primitive(property = JungleReplaceFlow)]
impl<P, L, R, M> ReplaceFlow for Conditional<P, L, R, M>
where
    L: ReplaceFlow,
    R: ReplaceFlow,
{
    type Output = Conditional<P, <L as ReplaceFlow>::Output, <R as ReplaceFlow>::Output, M>;
}

impl<P, L, R, M, Replacer> ReplaceWith<Replacer> for Conditional<P, L, R, M>
where
    L: ReplaceWith<Replacer>,
    R: ReplaceWith<Replacer>,
{
    type Output = Conditional<
        P,
        <L as ReplaceWith<Replacer>>::Output,
        <R as ReplaceWith<Replacer>>::Output,
        M,
    >;
}

impl<P, L, R, M, Replacer> ReplaceNodesWith<Replacer> for Conditional<P, L, R, M>
where
    L: ReplaceNodesWith<Replacer>,
    R: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<
        Conditional<
            P,
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >,
{
    type Output = <Replacer as ReplaceNode<
        Conditional<
            P,
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >>::Output;
}

#[primitive(property = JungleRunning)]
impl<C, F, M> Running for While<C, F, M>
where
    F: Running,
{
    type In = (bool, F::In);
    type Out = Option<F::Out>;

    fn run(input: Self::In) -> Self::Out {
        let (should_continue, input) = input;
        if should_continue {
            Some(<F as Running>::run(input))
        } else {
            None
        }
    }
}

#[primitive(property = JungleWaiting)]
impl<C, F, M> Waiting for While<C, F, M>
where
    F: Waiting,
{
    type In = Option<F::In>;
    type Out = Option<F::Out>;

    fn accept(input: Self::In) -> Self::Out {
        input.map(<F as Waiting>::accept)
    }
}

#[primitive(property = JungleFlow)]
impl<C, F, M> FlowEffects for While<C, F, M>
where
    F: FlowEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<C, F, M> TraverseFlow for While<C, F, M>
where
    F: TraverseFlow,
{
    type Output = While<C, <F as TraverseFlow>::Output, M>;
}

impl<C, F, M, Traversal> TraverseWith<Traversal> for While<C, F, M>
where
    F: TraverseWith<Traversal>,
{
    type Output = While<C, <F as TraverseWith<Traversal>>::Output, M>;
}

#[primitive(property = JungleReplaceFlow)]
impl<C, F, M> ReplaceFlow for While<C, F, M>
where
    F: ReplaceFlow,
{
    type Output = While<C, <F as ReplaceFlow>::Output, M>;
}

impl<C, F, M, Replacer> ReplaceWith<Replacer> for While<C, F, M>
where
    F: ReplaceWith<Replacer>,
{
    type Output = While<C, <F as ReplaceWith<Replacer>>::Output, M>;
}

impl<C, F, M, Replacer> ReplaceNodesWith<Replacer> for While<C, F, M>
where
    F: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<While<C, <F as ReplaceNodesWith<Replacer>>::Output, M>>,
{
    type Output =
        <Replacer as ReplaceNode<While<C, <F as ReplaceNodesWith<Replacer>>::Output, M>>>::Output;
}

#[primitive(property = JungleRunning)]
impl<M, F> Running for Transparent<M, F>
where
    F: Running,
{
    type In = F::In;
    type Out = F::Out;

    fn run(input: Self::In) -> Self::Out {
        <F as Running>::run(input)
    }
}

#[primitive(property = JungleWaiting)]
impl<M, F> Waiting for Transparent<M, F>
where
    F: Waiting,
{
    type In = F::In;
    type Out = F::Out;

    fn accept(input: Self::In) -> Self::Out {
        <F as Waiting>::accept(input)
    }
}

#[primitive(property = JungleFlow)]
impl<M, F> FlowEffects for Transparent<M, F>
where
    F: FlowEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<M, F> TraverseFlow for Transparent<M, F>
where
    F: TraverseFlow,
{
    type Output = Transparent<M, <F as TraverseFlow>::Output>;
}

impl<M, F, Traversal> TraverseWith<Traversal> for Transparent<M, F>
where
    F: TraverseWith<Traversal>,
{
    type Output = Transparent<M, <F as TraverseWith<Traversal>>::Output>;
}

#[primitive(property = JungleReplaceFlow)]
impl<M, F> ReplaceFlow for Transparent<M, F>
where
    F: ReplaceFlow,
{
    type Output = Transparent<M, <F as ReplaceFlow>::Output>;
}

impl<M, F, Replacer> ReplaceWith<Replacer> for Transparent<M, F>
where
    F: ReplaceWith<Replacer>,
{
    type Output = Transparent<M, <F as ReplaceWith<Replacer>>::Output>;
}

impl<M, F, Replacer> ReplaceNodesWith<Replacer> for Transparent<M, F>
where
    F: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<Transparent<M, <F as ReplaceNodesWith<Replacer>>::Output>>,
{
    type Output = <Replacer as ReplaceNode<
        Transparent<M, <F as ReplaceNodesWith<Replacer>>::Output>,
    >>::Output;
}

impl<T, A> NodeMetadata for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
}

impl<P, L, R, M> NodeMetadata for Conditional<P, L, R, M>
where
    M: NodeMetadata,
{
    const METADATA: &'static str = M::METADATA;
}
impl<C, F, M> NodeMetadata for While<C, F, M>
where
    M: NodeMetadata,
{
    const METADATA: &'static str = M::METADATA;
}
impl<L, R, M> NodeMetadata for Select<L, R, M>
where
    M: NodeMetadata,
{
    const METADATA: &'static str = M::METADATA;
}
impl<L, R, M> NodeMetadata for Join<L, R, M>
where
    M: NodeMetadata,
{
    const METADATA: &'static str = M::METADATA;
}

impl<M, F> NodeMetadata for Transparent<M, F>
where
    M: NodeMetadata,
{
    const METADATA: &'static str = M::METADATA;
}

#[primitive(property = JungleRunning)]
impl<L, R, M> Running for Select<L, R, M>
where
    L: Running,
    R: Running<In = L::In>,
{
    type In = L::In;
    type Out = Either<L::Out, R::Out>;

    fn run(input: Self::In) -> Self::Out {
        let _ = input;
        panic!("Select::run is executed by dynamic flow runtime");
    }
}

#[primitive(property = JungleWaiting)]
impl<L, R, M> Waiting for Select<L, R, M>
where
    L: Waiting,
    R: Waiting,
{
    type In = Either<L::In, R::In>;
    type Out = Either<L::Out, R::Out>;

    fn accept(input: Self::In) -> Self::Out {
        match input {
            Either::Left(input) => Either::Left(<L as Waiting>::accept(input)),
            Either::Right(input) => Either::Right(<R as Waiting>::accept(input)),
        }
    }
}

#[primitive(property = JungleFlow)]
impl<L, R, M> FlowEffects for Select<L, R, M>
where
    L: FlowEffects,
    R: FlowEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<L, R, M> TraverseFlow for Select<L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Select<<L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

impl<L, R, M, Traversal> TraverseWith<Traversal> for Select<L, R, M>
where
    L: TraverseWith<Traversal>,
    R: TraverseWith<Traversal>,
{
    type Output =
        Select<<L as TraverseWith<Traversal>>::Output, <R as TraverseWith<Traversal>>::Output, M>;
}

#[primitive(property = JungleReplaceFlow)]
impl<L, R, M> ReplaceFlow for Select<L, R, M>
where
    L: ReplaceFlow,
    R: ReplaceFlow,
{
    type Output = Select<<L as ReplaceFlow>::Output, <R as ReplaceFlow>::Output, M>;
}

impl<L, R, M, Replacer> ReplaceWith<Replacer> for Select<L, R, M>
where
    L: ReplaceWith<Replacer>,
    R: ReplaceWith<Replacer>,
{
    type Output =
        Select<<L as ReplaceWith<Replacer>>::Output, <R as ReplaceWith<Replacer>>::Output, M>;
}

impl<L, R, M, Replacer> ReplaceNodesWith<Replacer> for Select<L, R, M>
where
    L: ReplaceNodesWith<Replacer>,
    R: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<
        Select<
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >,
{
    type Output = <Replacer as ReplaceNode<
        Select<
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >>::Output;
}

#[primitive(property = JungleRunning)]
impl<L, R, M> Running for Join<L, R, M>
where
    L: Running,
    R: Running<In = L::In>,
{
    type In = L::In;
    type Out = (L::Out, R::Out);

    fn run(input: Self::In) -> Self::Out {
        let _ = input;
        panic!("Join::run is executed by dynamic flow runtime");
    }
}

#[primitive(property = JungleWaiting)]
impl<L, R, M> Waiting for Join<L, R, M>
where
    L: Waiting,
    R: Waiting,
{
    type In = (L::In, R::In);
    type Out = (L::Out, R::Out);

    fn accept((left, right): Self::In) -> Self::Out {
        (<L as Waiting>::accept(left), <R as Waiting>::accept(right))
    }
}

#[primitive(property = JungleFlow)]
impl<L, R, M> FlowEffects for Join<L, R, M>
where
    L: FlowEffects,
    R: FlowEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<L, R, M> TraverseFlow for Join<L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Join<<L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

impl<L, R, M, Traversal> TraverseWith<Traversal> for Join<L, R, M>
where
    L: TraverseWith<Traversal>,
    R: TraverseWith<Traversal>,
{
    type Output =
        Join<<L as TraverseWith<Traversal>>::Output, <R as TraverseWith<Traversal>>::Output, M>;
}

#[primitive(property = JungleReplaceFlow)]
impl<L, R, M> ReplaceFlow for Join<L, R, M>
where
    L: ReplaceFlow,
    R: ReplaceFlow,
{
    type Output = Join<<L as ReplaceFlow>::Output, <R as ReplaceFlow>::Output, M>;
}

impl<L, R, M, Replacer> ReplaceWith<Replacer> for Join<L, R, M>
where
    L: ReplaceWith<Replacer>,
    R: ReplaceWith<Replacer>,
{
    type Output =
        Join<<L as ReplaceWith<Replacer>>::Output, <R as ReplaceWith<Replacer>>::Output, M>;
}

impl<L, R, M, Replacer> ReplaceNodesWith<Replacer> for Join<L, R, M>
where
    L: ReplaceNodesWith<Replacer>,
    R: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<
        Join<
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >,
{
    type Output = <Replacer as ReplaceNode<
        Join<
            <L as ReplaceNodesWith<Replacer>>::Output,
            <R as ReplaceNodesWith<Replacer>>::Output,
            M,
        >,
    >>::Output;
}

/// A read-only view over an [`Animal`]'s current state.
pub trait Observe: Animal {
    /// The rendered appearance exposed by an observation.
    type Appearance;

    /// Observe the animal state and derive its outward appearance.
    fn observe(state: &Self::State) -> Self::Appearance;
}

/// A write path that perturbs an [`Animal`]'s state with an external stimulus.
pub trait Perturb: Animal {
    /// Input that drives a state transition.
    type Stimulus;

    /// Apply a stimulus to the current state.
    fn perturb(state: &mut Self::State, stimulus: Self::Stimulus);
}
