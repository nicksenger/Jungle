mod behavior;
mod error;
mod executor;
mod journey;
mod meta;
mod no_effect;
mod sealed {
    pub trait Sealed {}
}
mod sleep;
mod transport;
mod view;
pub use behavior::{Absorb, Emit, Fuse};
pub use behavior::{
    AbsorbFn, AbsorbMapper, EmitFn, EmitMapper, FocusedStep, IdentityStep, PassthroughEmit,
    UnitEmit,
};
pub use behavior::{
    Action, Aspect, BoundAction, BoundFlowStep, Effect, EffectCompletion, EffectRequest,
    EffectSchema, Identity, ScopeReboundAction, ScopedAction, ScopedAnimal, StateCarrier, Step,
};
pub use behavior::{FocusedAbsorb, FocusedEmit};
pub use error::{Error, Failure};
pub use executor::{
    ArgputForState, BuildFlow, BuildFlowWithContext, ContextExecutor, ContextualTypedErasedStep,
    DynFlow, ErasedStep, ExecutableEffectRequest, Executor, ExecutorError, ExecutorFlow,
    JungleDynFlow, JungleDynFlowContext, ManualExecutor, TypedErasedStep,
};
use inception::*;
pub use journey::Journey;
pub use meta::Id;
pub use meta::{
    AllFrom, AnimalEffectCompatible, AnimalEffectMembers, AnimalIdValue, AnimalMember, AnimalSet,
    AnimalStates, AnimalStatesCompatible, AnimalVersion, AnimalVersionIdentitiesUnique,
    AnimalVersions, EffectIdentity, EffectMember, EffectSet, Generations, GenerationsForAnimals,
    HighestGeneration, HighestGenerationForAnimals, IdValue, StripAnimalHeaders,
    StripEffectHeaders, WithEffectFor,
};
pub use no_effect::NoEffect;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
pub use sleep::{Sleep, SleepError, SleepStep};
use std::marker::PhantomData;
pub use transport::{
    BackendError, JourneyEvent, JourneyRecord, JourneyReplayPage, JourneyStatus,
    JourneyUpdateEvent, NodeLifecycle, NodeLifecyclePhase, RunnerOut, RunnerUpdateOut, WireIn,
    WireOut, Work,
};
pub use transport::{ClaimedPerturbable, OwnerWake, SupportedAnimal};
use typosaurus::collections::list::{self, List as TList};
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U0;
use typosaurus::num::{Bit, UInt, UTerm, Unsigned};
pub use view::{BuildJourneyAst, JourneyAst, JourneyAstSource, JungleJourneyAst};

/// A tagged union over two possible outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Default for Either<L, R>
where
    L: Default,
{
    fn default() -> Self {
        Self::Left(L::default())
    }
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

/// Unified predicate contract for control-flow decisions.
pub trait Predicate<Input> {
    fn eval(input: &Input) -> bool;
}

/// Adapts a [`Predicate`] defined over a focused view to run against a larger root state.
///
/// This is useful for scoped flow templates where branch predicates are authored against
/// the focused scope type, but runtime evaluation happens on the full animal state.
pub struct FocusedCondition<P, View>(PhantomData<fn() -> (P, View)>);

impl<State, View, Arg, P> Predicate<(State, Arg)> for FocusedCondition<P, View>
where
    State: ViewProject<View> + Clone,
    View: Clone,
    Arg: Clone,
    P: Predicate<(View, Arg)>,
{
    fn eval((state, arg): &(State, Arg)) -> bool {
        let mut projected_state = state.clone();
        let view = <State as ViewProject<View>>::project_view(&mut projected_state).clone();
        <P as Predicate<(View, Arg)>>::eval(&(view, arg.clone()))
    }
}

/// Adapts a borrowed-input loop [`Predicate`] over a focused view to run against a larger root state.
pub struct FocusedLoopCondition<C, View>(PhantomData<fn() -> (C, View)>);

impl<'a, State, View, Arg, C> Predicate<(&'a State, &'a Arg)> for FocusedLoopCondition<C, View>
where
    State: ViewProject<View> + Clone,
    C: for<'b> Predicate<(&'b View, &'b Arg)>,
{
    fn eval((state, arg): &(&'a State, &'a Arg)) -> bool {
        let mut projected_state = (*state).clone();
        let view = <State as ViewProject<View>>::project_view(&mut projected_state);
        <C as Predicate<(&View, &Arg)>>::eval(&(view, arg))
    }
}

/// Property used to opt-in a state type to field-index lenses.
pub struct JungleOptic;
impl Property for JungleOptic {}

/// Marker trait proving a type has opted into [`JungleOptic`].
pub trait Optic: Inception<JungleOptic, False> {}
impl<T> Optic for T where T: Inception<JungleOptic, False> {}

/// Direct projection contract from a scope state to a requested view type.
pub trait ViewProject<View> {
    fn project_view(state: &mut Self) -> &mut View;
}

/// Carrier that projects by target type via [`ViewProject`].
pub struct ViewCarrier<View>(PhantomData<fn() -> View>);

impl<State, View> StateCarrier<State> for ViewCarrier<View>
where
    State: ViewProject<View>,
{
    type Focus = View;

    fn focus(state: &mut State) -> &mut Self::Focus {
        <State as ViewProject<View>>::project_view(state)
    }
}

/// Index-based field projection contract used by [`Lens`].
pub trait LensIndex<Index> {
    type View;

    fn lens_index(state: &mut Self) -> &mut Self::View;
}

/// Recursive path projection over nested optic fields.
pub trait LensPath<Path> {
    type View;

    fn lens_path(state: &mut Self) -> &mut Self::View;
}

impl<State> LensPath<list::List<()>> for State {
    type View = State;

    fn lens_path(state: &mut Self) -> &mut Self::View {
        state
    }
}

/// Marker for numeric lens indexes (`U0`, `U1`, `U2`, ...).
pub trait LensNumber: Unsigned {}

impl LensNumber for UTerm {}
impl<U, B> LensNumber for UInt<U, B>
where
    U: Unsigned,
    B: Bit,
{
}

impl<State, Index> LensPath<Index> for State
where
    Index: LensNumber,
    State: LensIndex<Index>,
{
    type View = <State as LensIndex<Index>>::View;

    fn lens_path(state: &mut Self) -> &mut Self::View {
        <State as LensIndex<Index>>::lens_index(state)
    }
}

impl<State, Head, Tail> LensPath<list::List<(Head, Tail)>> for State
where
    Head: LensNumber,
    State: LensIndex<Head>,
    <State as LensIndex<Head>>::View: LensPath<Tail>,
{
    type View = <<State as LensIndex<Head>>::View as LensPath<Tail>>::View;

    fn lens_path(state: &mut Self) -> &mut Self::View {
        let inner = <State as LensIndex<Head>>::lens_index(state);
        <<State as LensIndex<Head>>::View as LensPath<Tail>>::lens_path(inner)
    }
}

/// Generic state carrier that projects by a type-level index or index path.
pub struct Lens<S, P>(PhantomData<S>, PhantomData<P>);

impl<State, Path> StateCarrier<State> for Lens<State, Path>
where
    State: LensPath<Path>,
{
    type Focus = <State as LensPath<Path>>::View;

    fn focus(state: &mut State) -> &mut Self::Focus {
        <State as LensPath<Path>>::lens_path(state)
    }
}

/// A flow combinator that chooses either `L` or `R` at runtime.
pub struct Conditional<P, L, R, M = NoMetadata>(PhantomData<fn() -> (P, L, R, M)>);

/// A flow combinator that repeatedly executes `F` while `C` is true.
pub struct While<C, F, M = NoMetadata>(PhantomData<fn() -> (C, F, M)>);

/// A flow combinator that runs two activities and resolves to whichever completes first.
pub struct Select<L, R, M = NoMetadata>(PhantomData<fn() -> (L, R, M)>);

/// A flow combinator that runs two activities and resolves when both complete.
pub struct Join<L, R, M = NoMetadata>(PhantomData<fn() -> (L, R, M)>);

/// A flow combinator that catches inner action failures and emits them as data.
pub struct Attempt<F, M = NoMetadata>(PhantomData<fn() -> (F, M)>);

/// Type-level metadata marker for flow nodes.
pub trait NodeMetadata {
    const METADATA: &'static str = "";
}

/// Empty metadata marker used by default when no metadata is provided.
pub struct NoMetadata;

impl NodeMetadata for NoMetadata {}

/// A no-op boundary wrapper used for organization and metadata anchoring.
pub struct Transparent<M, F>(PhantomData<fn() -> (M, F)>);

/// Scope wrapper used by late-bound templates to rebind subflow state.
pub struct Scoped<View, F>(PhantomData<fn() -> (View, F)>);

/// Bound flow wrapper that preserves a focused carrier boundary at runtime.
#[doc(hidden)]
pub struct FocusedBoundFlow<Carrier, F>(PhantomData<fn() -> (Carrier, F)>);

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

    /// The fundamental behavior template of this Animal.
    ///
    /// Framework/runtime sites bind this to the concrete animal via [`BindAnimal`]
    /// before execution.
    type Flow;
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
pub trait Observable: Animal + Sized {
    type Observation: ObservationBridge<Self>;
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
pub trait Perturbable: Animal + Sized {
    type Perturbation: PerturbationBridge<Self>;
}

#[inception(property = AnimalIdent, types, no_blanket)]
pub trait AnimalIdentified {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as AnimalIdentified>::Id, <Tail as AnimalIdentified>::Id)>,
        merge_variant = TList<(<Head as AnimalIdentified>::Id, <Tail as AnimalIdentified>::Id)>,
        join = TList<(U0, <Fields as AnimalIdentified>::Id)>
    )]
    type Id;
}

#[inception(property = EffectIdent, types, no_blanket)]
pub trait EffectIdentified {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as EffectIdentified>::Id, <Tail as EffectIdentified>::Id)>,
        merge_variant = TList<(<Head as EffectIdentified>::Id, <Tail as EffectIdentified>::Id)>,
        join = TList<(U0, <Fields as EffectIdentified>::Id)>
    )]
    type Id;
}

/// Any collection of [`Animal`]s with a flat type-level list of members.
#[inception(property = JungleAnimals, types, no_blanket)]
pub trait Animals {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        merge_variant = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        join = TList<(Node<<Self as AnimalIdentified>::Id, ()>, <Fields as Animals>::List)> where { Self: AnimalIdentified }
    )]
    type List;
}

/// Any collection of [`Effect`]s with a flat type-level list of members.
#[inception(property = JungleEffects, types, no_blanket)]
pub trait Effects {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Effects>::List, <Tail as Effects>::List)>,
        merge_variant = TList<(<Head as Effects>::List, <Tail as Effects>::List)>,
        join = TList<(Node<<Self as EffectIdentified>::Id, ()>, <Fields as Effects>::List)> where { Self: EffectIdentified }
    )]
    type List;
}

#[primitive(property = AnimalIdent)]
impl<T> Compat<T> for AnimalIdent
where
    T: Animal,
    T::Id: IdValue,
{
    type Out = True;
}

#[doc(hidden)]
pub trait AnimalIdentDispatch<P, T>: sealed::Sealed {
    type Id;
}

impl<T> AnimalIdentDispatch<True, T> for ()
where
    T: Animal,
    T::Id: IdValue,
{
    type Id = <T::Id as IdValue>::Value;
}

impl<T> AnimalIdentDispatch<False, T> for ()
where
    T: Inception<AnimalIdent> + Meta,
    __inception_animal_identified::Wrap<<T as Inception<AnimalIdent>>::TyFields>:
        __inception_animal_identified::__InceptionInduceId<False>,
{
    type Id = <__inception_animal_identified::Wrap<
        <T as Inception<AnimalIdent>>::TyFields,
    > as __inception_animal_identified::__InceptionInduceId<False>>::Ret;
}

impl<T> AnimalIdentified for T
where
    T: IsPrimitive<AnimalIdent>,
    (): AnimalIdentDispatch<<T as IsPrimitive<AnimalIdent>>::Is, T>,
{
    type Id = <() as AnimalIdentDispatch<<T as IsPrimitive<AnimalIdent>>::Is, T>>::Id;
}

#[primitive(property = JungleAnimals)]
impl<T> Compat<T> for JungleAnimals
where
    T: Animal,
{
    type Out = True;
}

#[doc(hidden)]
pub trait AnimalsDispatch<P, T>: sealed::Sealed {
    type List;
}

impl<T> AnimalsDispatch<True, T> for ()
where
    T: Animal + AnimalIdentified,
{
    type List = Node<<T as AnimalIdentified>::Id, T>;
}

impl<T> AnimalsDispatch<False, T> for ()
where
    T: Inception<JungleAnimals> + Meta,
    __inception_animals::Wrap<<T as Inception<JungleAnimals>>::TyFields>:
        __inception_animals::__InceptionInduceList<False>,
{
    type List = <__inception_animals::Wrap<
        <T as Inception<JungleAnimals>>::TyFields,
    > as __inception_animals::__InceptionInduceList<False>>::Ret;
}

impl<T> Animals for T
where
    T: IsPrimitive<JungleAnimals>,
    (): AnimalsDispatch<<T as IsPrimitive<JungleAnimals>>::Is, T>,
{
    type List = <() as AnimalsDispatch<<T as IsPrimitive<JungleAnimals>>::Is, T>>::List;
}

#[primitive(property = EffectIdent)]
impl<T> Compat<T> for EffectIdent
where
    T: EffectSchema,
    T::Id: IdValue,
{
    type Out = True;
}

#[doc(hidden)]
pub trait EffectIdentDispatch<P, T>: sealed::Sealed {
    type Id;
}

impl<T> EffectIdentDispatch<True, T> for ()
where
    T: EffectSchema,
    T::Id: IdValue,
{
    type Id = <T::Id as IdValue>::Value;
}

impl<T> EffectIdentDispatch<False, T> for ()
where
    T: Inception<EffectIdent> + Meta,
    __inception_effect_identified::Wrap<<T as Inception<EffectIdent>>::TyFields>:
        __inception_effect_identified::__InceptionInduceId<False>,
{
    type Id = <__inception_effect_identified::Wrap<
        <T as Inception<EffectIdent>>::TyFields,
    > as __inception_effect_identified::__InceptionInduceId<False>>::Ret;
}

impl<T> EffectIdentified for T
where
    T: IsPrimitive<EffectIdent>,
    (): EffectIdentDispatch<<T as IsPrimitive<EffectIdent>>::Is, T>,
{
    type Id = <() as EffectIdentDispatch<<T as IsPrimitive<EffectIdent>>::Is, T>>::Id;
}

#[primitive(property = JungleEffects)]
impl<T> Compat<T> for JungleEffects
where
    T: EffectSchema,
{
    type Out = True;
}

#[doc(hidden)]
pub trait EffectsDispatch<P, T>: sealed::Sealed {
    type List;
}

impl<T> EffectsDispatch<True, T> for ()
where
    T: EffectSchema + EffectIdentified,
{
    type List = Node<<T as EffectIdentified>::Id, T>;
}

impl<T> EffectsDispatch<False, T> for ()
where
    T: Inception<JungleEffects> + Meta,
    __inception_effects::Wrap<<T as Inception<JungleEffects>>::TyFields>:
        __inception_effects::__InceptionInduceList<False>,
{
    type List = <__inception_effects::Wrap<
        <T as Inception<JungleEffects>>::TyFields,
    > as __inception_effects::__InceptionInduceList<False>>::Ret;
}

impl<T> Effects for T
where
    T: IsPrimitive<JungleEffects>,
    (): EffectsDispatch<<T as IsPrimitive<JungleEffects>>::Is, T>,
{
    type List = <() as EffectsDispatch<<T as IsPrimitive<JungleEffects>>::Is, T>>::List;
}

/// A collection of [`Effect`]s extractable from an executable workflow.
#[inception(property = JungleFlow, types)]
pub trait JourneyEffects {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as JourneyEffects>::List, <Tail as JourneyEffects>::List)>,
        merge_variant = TList<(<Head as JourneyEffects>::List, <Tail as JourneyEffects>::List)>,
        join = TList<(Node<U0, ()>, <Fields as JourneyEffects>::List)>
    )]
    type List;
}

// Late-bound `BindAnimal` outputs are list-shaped flows (`TList`), and providing
// direct impls avoids pushing these through inception's reflective field path.
impl JourneyEffects for list::Empty {
    type List = list::Empty;
}

impl<Head, Tail> JourneyEffects for TList<(Head, Tail)>
where
    Head: JourneyEffects,
    Tail: JourneyEffects,
{
    type List = TList<(
        <Head as JourneyEffects>::List,
        <Tail as JourneyEffects>::List,
    )>;
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

/// Binds an unbound/template flow to a concrete [`Animal`].
pub trait BindAnimal<A: Animal> {
    type Bound;
}

/// Per-template scope declaration used by late-bound `BindAnimal`.
pub trait FlowScope {
    type View;
}

/// Default marker: bind template against root animal state.
pub struct RootFlowScope;
pub struct FlowView<View>(PhantomData<fn() -> View>);

/// Internal helper selecting bind traversal from [`FlowScope`].
pub trait BindWithFlowScope<A: Animal, ScopeView> {
    type Bound;
}

/// Dedicated binding recursion used by [`BoundAnimalJourney`] hot path.
pub trait BindFlow<A: Animal, Scope> {
    type Out;
}

/// Convenience alias for binding a flow/template to a concrete animal.
pub type BoundFlow<F, A> = <F as BindAnimal<A>>::Bound;
/// Marker for animals whose journey template can be bound to themselves.
pub trait BoundAnimal: Animal {
    type BoundJourney;
}

impl<A> BoundAnimal for A
where
    A: Animal,
    A::Flow: BindAnimal<A>,
{
    type BoundJourney = BoundFlow<A::Flow, A>;
}

/// Convenience alias for an [`Animal`]'s bound journey.
pub type BoundAnimalJourney<A> = <A as BoundAnimal>::BoundJourney;

/// Traversal that binds `Step<S>` nodes to concrete `BoundFlowStep<A, _>` nodes
/// within a current scope carrier.
pub struct RootScope;
pub struct BindAnimalTraversal<A, Scope = RootScope>(PhantomData<fn() -> (A, Scope)>);

impl<State> StateCarrier<State> for RootScope {
    type Focus = State;

    fn focus(state: &mut State) -> &mut Self::Focus {
        state
    }
}

pub(crate) trait ScopedCarrierMarker {}

impl<View> ScopedCarrierMarker for ViewCarrier<View> {}

impl<Outer, Inner> ScopedCarrierMarker for behavior::ComposeCarrier<Outer, Inner>
where
    Outer: ScopedCarrierMarker,
    Inner: ScopedCarrierMarker,
{
}

impl<A, Scope> BindFlow<A, Scope> for list::Empty
where
    A: Animal,
{
    type Out = list::Empty;
}

impl<A, Scope, Head, Tail> BindFlow<A, Scope> for TList<(Head, Tail)>
where
    A: Animal,
    Head: BindFlow<A, Scope>,
    Tail: BindFlow<A, Scope>,
{
    type Out = TList<(
        <Head as BindFlow<A, Scope>>::Out,
        <Tail as BindFlow<A, Scope>>::Out,
    )>;
}

impl<A, Scope, T, B> BindFlow<A, Scope> for BoundFlowStep<T, B>
where
    A: Animal,
    T: Animal,
    B: BoundAction<T>,
{
    type Out = BoundFlowStep<T, B>;
}

impl<T, S> BindFlow<T, RootScope> for Step<S>
where
    T: Animal,
    S: Action,
    <S as Action>::Bind<T>: BoundAction<
        T,
        Input = <S as Action>::Input,
        Output = <S as Action>::Output,
        Effect = <S as Action>::Effect,
    >,
{
    type Out = BoundFlowStep<T, <S as Action>::Bind<T>>;
}

impl<T, ScopeCarrier, S> BindFlow<T, ScopeCarrier> for Step<S>
where
    T: Animal,
    ScopeCarrier: ScopedCarrierMarker,
    ScopeCarrier: Aspect<T::State>,
    S: Action,
    S: ScopedAction<T, <ScopeCarrier as StateCarrier<T::State>>::Focus, ScopeCarrier>,
    <S as ScopedAction<T, <ScopeCarrier as StateCarrier<T::State>>::Focus, ScopeCarrier>>::BoundAction:
        BoundAction<
            T,
            Input = <S as Action>::Input,
            Output = <S as Action>::Output,
            Effect = <S as Action>::Effect,
        >,
{
    type Out = BoundFlowStep<
        T,
        <S as ScopedAction<
            T,
            <ScopeCarrier as StateCarrier<T::State>>::Focus,
            ScopeCarrier,
        >>::BoundAction,
    >;
}

impl<A, Scope, P, L, R, M> BindFlow<A, Scope> for Conditional<P, L, R, M>
where
    A: Animal,
    L: BindFlow<A, Scope>,
    R: BindFlow<A, Scope>,
{
    type Out = Conditional<P, <L as BindFlow<A, Scope>>::Out, <R as BindFlow<A, Scope>>::Out, M>;
}

impl<A, Scope, C, F, M> BindFlow<A, Scope> for While<C, F, M>
where
    A: Animal,
    F: BindFlow<A, Scope>,
{
    type Out = While<C, <F as BindFlow<A, Scope>>::Out, M>;
}

impl<A, Scope, M, F> BindFlow<A, Scope> for Transparent<M, F>
where
    A: Animal,
    F: BindFlow<A, Scope>,
{
    type Out = Transparent<M, <F as BindFlow<A, Scope>>::Out>;
}

impl<A, Scope, L, R, M> BindFlow<A, Scope> for Select<L, R, M>
where
    A: Animal,
    L: BindFlow<A, Scope>,
    R: BindFlow<A, Scope>,
{
    type Out = Select<<L as BindFlow<A, Scope>>::Out, <R as BindFlow<A, Scope>>::Out, M>;
}

impl<A, Scope, L, R, M> BindFlow<A, Scope> for Join<L, R, M>
where
    A: Animal,
    L: BindFlow<A, Scope>,
    R: BindFlow<A, Scope>,
{
    type Out = Join<<L as BindFlow<A, Scope>>::Out, <R as BindFlow<A, Scope>>::Out, M>;
}

impl<A, Scope, F, M> BindFlow<A, Scope> for Attempt<F, M>
where
    A: Animal,
    F: BindFlow<A, Scope>,
{
    type Out = Attempt<<F as BindFlow<A, Scope>>::Out, M>;
}

impl<A, View, F> BindFlow<A, RootScope> for Scoped<View, F>
where
    A: Animal,
    View: 'static,
    F: BindFlow<A, ViewCarrier<View>>,
{
    type Out = FocusedBoundFlow<ViewCarrier<View>, <F as BindFlow<A, ViewCarrier<View>>>::Out>;
}

impl<A, ScopeCarrier, View, F> BindFlow<A, ScopeCarrier> for Scoped<View, F>
where
    A: Animal,
    ScopeCarrier: ScopedCarrierMarker,
    ScopeCarrier: Aspect<A::State>,
    View: 'static,
    F: BindFlow<A, behavior::ComposeCarrier<ScopeCarrier, ViewCarrier<View>>>,
{
    type Out = FocusedBoundFlow<
        behavior::ComposeCarrier<ScopeCarrier, ViewCarrier<View>>,
        <F as BindFlow<A, behavior::ComposeCarrier<ScopeCarrier, ViewCarrier<View>>>>::Out,
    >;
}

impl<F, A> BindWithFlowScope<A, RootFlowScope> for F
where
    A: Animal,
    F: BindFlow<A, RootScope>,
{
    type Bound = <F as BindFlow<A, RootScope>>::Out;
}

impl<F, A, View> BindWithFlowScope<A, FlowView<View>> for F
where
    A: Animal,
    View: 'static,
    F: BindFlow<A, RootScope>,
{
    type Bound = <F as BindFlow<A, RootScope>>::Out;
}

/// Directional helper that rewrites `BoundFlowStep<Animal, Left>` to `BoundFlowStep<Animal, Right>`.
pub struct SwapLR<Left, Right>(PhantomData<fn() -> (Left, Right)>);

/// Directional helper that rewrites `BoundFlowStep<Animal, Right>` to `BoundFlowStep<Animal, Left>`.
pub struct SwapRL<Left, Right>(PhantomData<fn() -> (Left, Right)>);

/// Directional helper alias for node replacement from `Left` to `Right`.
pub type SwapNodeLR<Left, Right> = SwapLR<Left, Right>;

/// Directional helper alias for node replacement from `Right` to `Left`.
pub type SwapNodeRL<Left, Right> = SwapRL<Left, Right>;

impl<A, Left, Right> ReplaceStep<BoundFlowStep<A, Left>> for SwapLR<Left, Right>
where
    A: Animal,
    Left: BoundAction<A>,
    Right: BoundAction<A>,
{
    type Output = BoundFlowStep<A, Right>;
}

impl<A, Left, Right> ReplaceStep<BoundFlowStep<A, Right>> for SwapRL<Left, Right>
where
    A: Animal,
    Left: BoundAction<A>,
    Right: BoundAction<A>,
{
    type Output = BoundFlowStep<A, Left>;
}

impl<Left, Right> ReplaceStep<Step<Left>> for SwapLR<Left, Right>
where
    Left: Action,
    Right: Action<
        Input = <Left as Action>::Input,
        Output = <Left as Action>::Output,
        Effect = <Left as Action>::Effect,
    >,
{
    type Output = Step<Right>;
}

impl<Left, Right> ReplaceStep<Step<Right>> for SwapRL<Left, Right>
where
    Left: Action,
    Right: Action<
        Input = <Left as Action>::Input,
        Output = <Left as Action>::Output,
        Effect = <Left as Action>::Effect,
    >,
{
    type Output = Step<Left>;
}

impl<Left, Right> ReplaceNode<Left> for SwapLR<Left, Right> {
    type Output = Right;
}

impl<Left, Right> ReplaceNode<Right> for SwapRL<Left, Right> {
    type Output = Left;
}

/// Inception property that normalizes/walks a flow's type structure.
///
/// This is an internal structural traversal used by derive/inception output.
/// Public [`TraverseFlow`] adapts this shape based on [`FlowScope`].
#[inception(property = JungleTraverseFlow, types)]
pub trait TraverseFlowShape {
    #[induce(
        base = list::Empty,
        merge = TList<(
            <Head as TraverseFlow>::Output,
            <Tail as TraverseFlowShape>::Output
        )> where { Head: TraverseFlow },
        merge_variant = TList<(
            <Head as TraverseFlow>::Output,
            <Tail as TraverseFlowShape>::Output
        )> where { Head: TraverseFlow },
        join = <Fields as TraverseFlowShape>::Output
    )]
    type Output;
}

/// Public flow traversal output used by bind/traverse operations.
pub trait TraverseFlow {
    type Output;
}

/// Internal helper that routes [`TraverseFlow`] by declared [`FlowScope`].
pub trait TraverseFlowWithScope<ScopeView> {
    type Output;
}

impl<F> TraverseFlow for F
where
    F: FlowScope + TraverseFlowWithScope<<F as FlowScope>::View>,
{
    type Output = <F as TraverseFlowWithScope<<F as FlowScope>::View>>::Output;
}

impl<F> TraverseFlowWithScope<RootFlowScope> for F
where
    F: TraverseFlowShape,
{
    type Output = <F as TraverseFlowShape>::Output;
}

impl<F, View> TraverseFlowWithScope<FlowView<View>> for F
where
    F: TraverseFlowShape,
    <F as TraverseFlowShape>::Output: ScopedFieldListNormalize,
{
    type Output =
        Scoped<View, <<F as TraverseFlowShape>::Output as ScopedFieldListNormalize>::Output>;
}

impl TraverseFlow for VariantHeader {
    type Output = list::Empty;
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

/// Normalizes a flow fragment into a list-shaped representation suitable for
/// focused-field concatenation.
pub trait ScopedFieldListNormalize: sealed::Sealed {
    type Output;
}

/// Concatenates two list-shaped flow fragments.
pub trait FlowListConcat<Rhs>: sealed::Sealed {
    type Output;
}

impl<Rhs> FlowListConcat<Rhs> for list::Empty {
    type Output = Rhs;
}

impl<Head, Tail, Rhs> FlowListConcat<Rhs> for TList<(Head, Tail)>
where
    Tail: FlowListConcat<Rhs>,
{
    type Output = TList<(Head, <Tail as FlowListConcat<Rhs>>::Output)>;
}

impl ScopedFieldListNormalize for list::Empty {
    type Output = list::Empty;
}

impl<Head, Tail> ScopedFieldListNormalize for TList<(Head, Tail)>
where
    Head: ScopedFieldListNormalize,
    Tail: ScopedFieldListNormalize,
    <Head as ScopedFieldListNormalize>::Output:
        FlowListConcat<<Tail as ScopedFieldListNormalize>::Output>,
{
    type Output = <<Head as ScopedFieldListNormalize>::Output as FlowListConcat<
        <Tail as ScopedFieldListNormalize>::Output,
    >>::Output;
}

impl<T, A> ScopedFieldListNormalize for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type Output = TList<(BoundFlowStep<T, A>, list::Empty)>;
}

impl<S> ScopedFieldListNormalize for Step<S>
where
    S: Action,
{
    type Output = TList<(Step<S>, list::Empty)>;
}

impl<P, L, R, M> ScopedFieldListNormalize for Conditional<P, L, R, M> {
    type Output = TList<(Conditional<P, L, R, M>, list::Empty)>;
}

impl<C, F, M> ScopedFieldListNormalize for While<C, F, M> {
    type Output = TList<(While<C, F, M>, list::Empty)>;
}

impl<L, R, M> ScopedFieldListNormalize for Select<L, R, M> {
    type Output = TList<(Select<L, R, M>, list::Empty)>;
}

impl<L, R, M> ScopedFieldListNormalize for Join<L, R, M> {
    type Output = TList<(Join<L, R, M>, list::Empty)>;
}

impl<F, M> ScopedFieldListNormalize for Attempt<F, M> {
    type Output = TList<(Attempt<F, M>, list::Empty)>;
}

impl<M, F> ScopedFieldListNormalize for Transparent<M, F> {
    type Output = TList<(Transparent<M, F>, list::Empty)>;
}

impl<View, F> ScopedFieldListNormalize for Scoped<View, F> {
    type Output = TList<(Scoped<View, F>, list::Empty)>;
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

macro_rules! flow_list_chain {
    ($h:ty) => {
        TList<($h, list::Empty)>
    };
    ($h:ty, $($rest:ty),+) => {
        TList<($h, flow_list_chain!($($rest),+))>
    };
}
macro_rules! flow_list_chain_tail {
    ($h:ty ; $tail:ty) => {
        TList<($h, $tail)>
    };
    ($h:ty, $($rest:ty),+ ; $tail:ty) => {
        TList<($h, flow_list_chain_tail!($($rest),+ ; $tail))>
    };
}

macro_rules! traverse_with_len_impl {
    ($h0:ident) => {
        impl<$h0, Traversal> TraverseWith<Traversal> for flow_list_chain!($h0)
        where
            $h0: TraverseWith<Traversal>,
        {
            type Output = flow_list_chain!(<$h0 as TraverseWith<Traversal>>::Output);
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+ Traversal> TraverseWith<Traversal> for flow_list_chain!($h0, $($rest),+)
        where
            $h0: TraverseWith<Traversal>,
            flow_list_chain!($($rest),+): TraverseWith<Traversal>,
        {
            type Output = flow_list_chain_tail!(
                <$h0 as TraverseWith<Traversal>>::Output ;
                <flow_list_chain!($($rest),+) as TraverseWith<Traversal>>::Output
            );
        }
    };
}
traverse_with_len_impl!(H0);
traverse_with_len_impl!(H0; H1);
traverse_with_len_impl!(H0; H1, H2);
traverse_with_len_impl!(H0; H1, H2, H3);
traverse_with_len_impl!(H0; H1, H2, H3, H4);
traverse_with_len_impl!(H0; H1, H2, H3, H4, H5);
traverse_with_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail, Traversal> TraverseWith<Traversal> for flow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail)
where
    H0: TraverseWith<Traversal>,
    H1: TraverseWith<Traversal>,
    H2: TraverseWith<Traversal>,
    H3: TraverseWith<Traversal>,
    H4: TraverseWith<Traversal>,
    H5: TraverseWith<Traversal>,
    H6: TraverseWith<Traversal>,
    H7: TraverseWith<Traversal>,
    Tail: TraverseWith<Traversal>,
{
    type Output = flow_list_chain_tail!(
        <H0 as TraverseWith<Traversal>>::Output,
        <H1 as TraverseWith<Traversal>>::Output,
        <H2 as TraverseWith<Traversal>>::Output,
        <H3 as TraverseWith<Traversal>>::Output,
        <H4 as TraverseWith<Traversal>>::Output,
        <H5 as TraverseWith<Traversal>>::Output,
        <H6 as TraverseWith<Traversal>>::Output,
        <H7 as TraverseWith<Traversal>>::Output ;
        <Tail as TraverseWith<Traversal>>::Output
    );
}

macro_rules! replace_with_len_impl {
    ($h0:ident) => {
        impl<$h0, Replacer> ReplaceWith<Replacer> for flow_list_chain!($h0)
        where
            $h0: ReplaceWith<Replacer>,
        {
            type Output = flow_list_chain!(<$h0 as ReplaceWith<Replacer>>::Output);
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+ Replacer> ReplaceWith<Replacer> for flow_list_chain!($h0, $($rest),+)
        where
            $h0: ReplaceWith<Replacer>,
            flow_list_chain!($($rest),+): ReplaceWith<Replacer>,
        {
            type Output = flow_list_chain_tail!(
                <$h0 as ReplaceWith<Replacer>>::Output ;
                <flow_list_chain!($($rest),+) as ReplaceWith<Replacer>>::Output
            );
        }
    };
}
replace_with_len_impl!(H0);
replace_with_len_impl!(H0; H1);
replace_with_len_impl!(H0; H1, H2);
replace_with_len_impl!(H0; H1, H2, H3);
replace_with_len_impl!(H0; H1, H2, H3, H4);
replace_with_len_impl!(H0; H1, H2, H3, H4, H5);
replace_with_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail, Replacer> ReplaceWith<Replacer> for flow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail)
where
    H0: ReplaceWith<Replacer>,
    H1: ReplaceWith<Replacer>,
    H2: ReplaceWith<Replacer>,
    H3: ReplaceWith<Replacer>,
    H4: ReplaceWith<Replacer>,
    H5: ReplaceWith<Replacer>,
    H6: ReplaceWith<Replacer>,
    H7: ReplaceWith<Replacer>,
    Tail: ReplaceWith<Replacer>,
{
    type Output = flow_list_chain_tail!(
        <H0 as ReplaceWith<Replacer>>::Output,
        <H1 as ReplaceWith<Replacer>>::Output,
        <H2 as ReplaceWith<Replacer>>::Output,
        <H3 as ReplaceWith<Replacer>>::Output,
        <H4 as ReplaceWith<Replacer>>::Output,
        <H5 as ReplaceWith<Replacer>>::Output,
        <H6 as ReplaceWith<Replacer>>::Output,
        <H7 as ReplaceWith<Replacer>>::Output ;
        <Tail as ReplaceWith<Replacer>>::Output
    );
}

macro_rules! replace_nodes_with_len_impl {
    ($h0:ident) => {
        impl<$h0, Replacer> ReplaceNodesWith<Replacer> for flow_list_chain!($h0)
        where
            $h0: ReplaceNodesWith<Replacer>,
        {
            type Output = flow_list_chain!(<$h0 as ReplaceNodesWith<Replacer>>::Output);
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+ Replacer> ReplaceNodesWith<Replacer> for flow_list_chain!($h0, $($rest),+)
        where
            $h0: ReplaceNodesWith<Replacer>,
            flow_list_chain!($($rest),+): ReplaceNodesWith<Replacer>,
        {
            type Output = flow_list_chain_tail!(
                <$h0 as ReplaceNodesWith<Replacer>>::Output ;
                <flow_list_chain!($($rest),+) as ReplaceNodesWith<Replacer>>::Output
            );
        }
    };
}
replace_nodes_with_len_impl!(H0);
replace_nodes_with_len_impl!(H0; H1);
replace_nodes_with_len_impl!(H0; H1, H2);
replace_nodes_with_len_impl!(H0; H1, H2, H3);
replace_nodes_with_len_impl!(H0; H1, H2, H3, H4);
replace_nodes_with_len_impl!(H0; H1, H2, H3, H4, H5);
replace_nodes_with_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail, Replacer> ReplaceNodesWith<Replacer> for flow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail)
where
    H0: ReplaceNodesWith<Replacer>,
    H1: ReplaceNodesWith<Replacer>,
    H2: ReplaceNodesWith<Replacer>,
    H3: ReplaceNodesWith<Replacer>,
    H4: ReplaceNodesWith<Replacer>,
    H5: ReplaceNodesWith<Replacer>,
    H6: ReplaceNodesWith<Replacer>,
    H7: ReplaceNodesWith<Replacer>,
    Tail: ReplaceNodesWith<Replacer>,
{
    type Output = flow_list_chain_tail!(
        <H0 as ReplaceNodesWith<Replacer>>::Output,
        <H1 as ReplaceNodesWith<Replacer>>::Output,
        <H2 as ReplaceNodesWith<Replacer>>::Output,
        <H3 as ReplaceNodesWith<Replacer>>::Output,
        <H4 as ReplaceNodesWith<Replacer>>::Output,
        <H5 as ReplaceNodesWith<Replacer>>::Output,
        <H6 as ReplaceNodesWith<Replacer>>::Output,
        <H7 as ReplaceNodesWith<Replacer>>::Output ;
        <Tail as ReplaceNodesWith<Replacer>>::Output
    );
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
    P: Predicate<L::In>,
{
    type In = L::In;
    type Out = Either<L::Out, R::Out>;

    fn run(input: Self::In) -> Self::Out {
        if <P as Predicate<L::In>>::eval(&input) {
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
impl<P, L, R, M> JourneyEffects for Conditional<P, L, R, M>
where
    L: JourneyEffects,
    R: JourneyEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<P, L, R, M> TraverseFlowShape for Conditional<P, L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Conditional<P, <L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

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
impl<C, F, M> JourneyEffects for While<C, F, M>
where
    F: JourneyEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<C, F, M> TraverseFlowShape for While<C, F, M>
where
    F: TraverseFlow,
{
    type Output = While<C, <F as TraverseFlow>::Output, M>;
}

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
impl<M, F> Running for Attempt<F, M>
where
    F: Running,
{
    type In = F::In;
    type Out = Result<F::Out, Failure>;

    fn run(input: Self::In) -> Self::Out {
        Ok(<F as Running>::run(input))
    }
}

#[primitive(property = JungleWaiting)]
impl<M, F> Waiting for Attempt<F, M>
where
    F: Waiting,
{
    type In = F::In;
    type Out = Result<F::Out, Failure>;

    fn accept(input: Self::In) -> Self::Out {
        Ok(<F as Waiting>::accept(input))
    }
}

#[primitive(property = JungleFlow)]
impl<M, F> JourneyEffects for Attempt<F, M>
where
    F: JourneyEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<M, F> TraverseFlowShape for Attempt<F, M>
where
    F: TraverseFlow,
{
    type Output = Attempt<<F as TraverseFlow>::Output, M>;
}

impl<M, F> TraverseFlow for Attempt<F, M>
where
    F: TraverseFlow,
{
    type Output = Attempt<<F as TraverseFlow>::Output, M>;
}

impl<M, F, Traversal> TraverseWith<Traversal> for Attempt<F, M>
where
    F: TraverseWith<Traversal>,
{
    type Output = Attempt<<F as TraverseWith<Traversal>>::Output, M>;
}

#[primitive(property = JungleReplaceFlow)]
impl<M, F> ReplaceFlow for Attempt<F, M>
where
    F: ReplaceFlow,
{
    type Output = Attempt<<F as ReplaceFlow>::Output, M>;
}

impl<M, F, Replacer> ReplaceWith<Replacer> for Attempt<F, M>
where
    F: ReplaceWith<Replacer>,
{
    type Output = Attempt<<F as ReplaceWith<Replacer>>::Output, M>;
}

impl<M, F, Replacer> ReplaceNodesWith<Replacer> for Attempt<F, M>
where
    F: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<Attempt<<F as ReplaceNodesWith<Replacer>>::Output, M>>,
{
    type Output =
        <Replacer as ReplaceNode<Attempt<<F as ReplaceNodesWith<Replacer>>::Output, M>>>::Output;
}

#[primitive(property = JungleRunning)]
impl<View, F> Running for Scoped<View, F>
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
impl<View, F> Waiting for Scoped<View, F>
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
impl<View, F> JourneyEffects for Scoped<View, F>
where
    F: JourneyEffects,
{
    type List = F::List;
}

#[primitive(property = JungleFlow)]
impl<Carrier, F> JourneyEffects for FocusedBoundFlow<Carrier, F>
where
    F: JourneyEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<View, F> TraverseFlowShape for Scoped<View, F>
where
    F: TraverseFlow,
{
    type Output = Scoped<View, <F as TraverseFlow>::Output>;
}

impl<View, F> TraverseFlow for Scoped<View, F>
where
    F: TraverseFlow,
{
    type Output = Scoped<View, <F as TraverseFlow>::Output>;
}

#[primitive(property = JungleReplaceFlow)]
impl<View, F> ReplaceFlow for Scoped<View, F>
where
    F: ReplaceFlow,
{
    type Output = Scoped<View, <F as ReplaceFlow>::Output>;
}

impl<View, F, Replacer> ReplaceWith<Replacer> for Scoped<View, F>
where
    F: ReplaceWith<Replacer>,
{
    type Output = Scoped<View, <F as ReplaceWith<Replacer>>::Output>;
}

impl<View, F, Replacer> ReplaceNodesWith<Replacer> for Scoped<View, F>
where
    F: ReplaceNodesWith<Replacer>,
    Replacer: ReplaceNode<Scoped<View, <F as ReplaceNodesWith<Replacer>>::Output>>,
{
    type Output =
        <Replacer as ReplaceNode<Scoped<View, <F as ReplaceNodesWith<Replacer>>::Output>>>::Output;
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
impl<M, F> JourneyEffects for Transparent<M, F>
where
    F: JourneyEffects,
{
    type List = F::List;
}

#[primitive(property = JungleTraverseFlow)]
impl<M, F> TraverseFlowShape for Transparent<M, F>
where
    F: TraverseFlow,
{
    type Output = Transparent<M, <F as TraverseFlow>::Output>;
}

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

impl<T, A> NodeMetadata for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
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

impl<F, M> NodeMetadata for Attempt<F, M>
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

impl<View, F> NodeMetadata for Scoped<View, F> {}

impl<Carrier, F> NodeMetadata for FocusedBoundFlow<Carrier, F> {}

impl<S> NodeMetadata for Step<S> where S: Action {}

impl<A, Scope, T, B> TraverseStep<BoundFlowStep<T, B>> for BindAnimalTraversal<A, Scope>
where
    A: Animal,
    Scope: Aspect<A::State>,
    T: Animal,
    B: BoundAction<T>,
{
    type Output = BoundFlowStep<T, B>;
}

impl<A, View, F> TraverseWith<BindAnimalTraversal<A, RootScope>> for Scoped<View, F>
where
    A: Animal,
    View: 'static,
    F: TraverseWith<BindAnimalTraversal<A, ViewCarrier<View>>>,
{
    type Output = <F as TraverseWith<BindAnimalTraversal<A, ViewCarrier<View>>>>::Output;
}

impl<A, ScopeCarrier, View, F> TraverseWith<BindAnimalTraversal<A, ScopeCarrier>>
    for Scoped<View, F>
where
    A: Animal,
    ScopeCarrier: ScopedCarrierMarker,
    ScopeCarrier: Aspect<A::State>,
    View: 'static,
    F: TraverseWith<
        BindAnimalTraversal<A, behavior::ComposeCarrier<ScopeCarrier, ViewCarrier<View>>>,
    >,
{
    type Output = <F as TraverseWith<
        BindAnimalTraversal<A, behavior::ComposeCarrier<ScopeCarrier, ViewCarrier<View>>>,
    >>::Output;
}

impl<A, F> BindAnimal<A> for F
where
    A: Animal,
    F: FlowScope + BindWithFlowScope<A, <F as FlowScope>::View>,
{
    type Bound = <F as BindWithFlowScope<A, <F as FlowScope>::View>>::Bound;
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
impl<L, R, M> JourneyEffects for Select<L, R, M>
where
    L: JourneyEffects,
    R: JourneyEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<L, R, M> TraverseFlowShape for Select<L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Select<<L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

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
    R: Running,
{
    type In = (L::In, R::In);
    type Out = (L::Out, R::Out);

    fn run((left, right): Self::In) -> Self::Out {
        let _ = (left, right);
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
impl<L, R, M> JourneyEffects for Join<L, R, M>
where
    L: JourneyEffects,
    R: JourneyEffects,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleTraverseFlow)]
impl<L, R, M> TraverseFlowShape for Join<L, R, M>
where
    L: TraverseFlow,
    R: TraverseFlow,
{
    type Output = Join<<L as TraverseFlow>::Output, <R as TraverseFlow>::Output, M>;
}

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

impl sealed::Sealed for list::Empty {}

impl<Head, Tail> sealed::Sealed for TList<(Head, Tail)> {}

impl<T, A> sealed::Sealed for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
}

impl<S> sealed::Sealed for Step<S> where S: Action {}

impl<P, L, R, M> sealed::Sealed for Conditional<P, L, R, M> {}

impl<C, F, M> sealed::Sealed for While<C, F, M> {}

impl<L, R, M> sealed::Sealed for Select<L, R, M> {}

impl<L, R, M> sealed::Sealed for Join<L, R, M> {}

impl<F, M> sealed::Sealed for Attempt<F, M> {}

impl<M, F> sealed::Sealed for Transparent<M, F> {}

impl<View, F> sealed::Sealed for Scoped<View, F> {}

impl<Carrier, F> sealed::Sealed for FocusedBoundFlow<Carrier, F> {}

impl sealed::Sealed for () {}

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
