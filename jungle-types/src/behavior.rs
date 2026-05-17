use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;

use crate::{
    Animal, EffectIdentity, EffectMember, JourneyEffects, ReplaceFlow, ReplaceNode,
    ReplaceNodesWith, ReplaceStep, ReplaceWith, Running, TraverseFlow, TraverseStep, TraverseWith,
    Waiting,
};
use inception::primitive;
use typosaurus::collections::sp::Node;

/// Canonical, context-agnostic effect contract used by flow shape and wire schema.
pub trait EffectSchema {
    /// A type-level identifier for this Effect.
    type Id;

    /// The input type accepted by this effect.
    type In: Serialize + DeserializeOwned + Send + 'static;

    /// The output type produced by this effect.
    type Out: Serialize + DeserializeOwned + Send + 'static;

    /// The error type produced by this effect.
    type Err: Send + 'static;
}

/// Context-bound effect execution contract.
pub trait EffectExec<J>: EffectSchema {
    /// Process one input into one output in the provided context.
    fn effect(
        jungle: &J,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> + Send;
}

/// A typed effect request emitted by a yielding workflow phase.
pub struct EffectRequest<A: EffectSchema> {
    pub input: A::In,
    marker: PhantomData<fn() -> A>,
}

impl<A: EffectSchema> EffectRequest<A> {
    pub fn new(input: A::In) -> Self {
        Self {
            input,
            marker: PhantomData,
        }
    }

    pub fn into_input(self) -> A::In {
        self.input
    }

    pub fn effect<'a, J>(self, jungle: &'a J) -> impl Future<Output = Result<A::Out, A::Err>> + 'a
    where
        A: EffectExec<J> + 'a,
    {
        A::effect(jungle, self.input)
    }
}

/// A completed effect result consumed by an awaiting workflow phase.
pub type EffectCompletion<A> = Result<<A as EffectSchema>::Out, <A as EffectSchema>::Err>;

/// Projects a larger state into a focused mutable substate.
pub trait StateCarrier<State> {
    type View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View;
}

/// Composes two carriers into a single projection.
pub struct ComposeCarrier<Outer, Inner>(PhantomData<fn() -> (Outer, Inner)>);

impl<State, Outer, Inner> StateCarrier<State> for ComposeCarrier<Outer, Inner>
where
    Outer: StateCarrier<State>,
    <Outer as StateCarrier<State>>::View: 'static,
    Inner: StateCarrier<<Outer as StateCarrier<State>>::View>,
{
    type View = <Inner as StateCarrier<<Outer as StateCarrier<State>>::View>>::View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        let outer = <Outer as StateCarrier<State>>::view(state);
        <Inner as StateCarrier<<Outer as StateCarrier<State>>::View>>::view(outer)
    }
}

// Compatibility shim during carrier-trait migration.
pub trait Aspect<State>: StateCarrier<State> {}

impl<T, State> Aspect<State> for T where T: StateCarrier<State> {}

/// Focuses to the full state itself.
pub struct Identity;

impl<State> StateCarrier<State> for Identity {
    type View = State;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        state
    }
}

/// Single step-facing contract for adapting an [`Effect`] over an [`Aspect`]
/// of animal state.
pub trait BoundAct<T: Animal> {
    type Effect: EffectSchema;
    type Aspect: Aspect<T::State>;
    type Input;
    type Output;

    fn emit(
        view: &<<Self as BoundAct<T>>::Aspect as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In;

    fn absorb(
        view: &mut <<Self as BoundAct<T>>::Aspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output;
}

/// Late-bound action spec that can be bound to a concrete [`Animal`] at the edge.
pub trait ActionSpec {
    type Effect: EffectMember;
    type Input;
    type Output;
    type BoundAct<A: Animal>;
}

/// Re-binds an action spec authored for one scope to another scope.
pub trait ScopedActionSpec<A: Animal, ScopeState, ScopeCarrier> {
    type BoundAct: BoundAct<A>;
}

/// Animal adapter that reuses identity metadata from `A` while swapping `State`.
pub struct ScopedAnimal<A, Scope>(PhantomData<fn() -> (A, Scope)>);

impl<A, Scope> Animal for ScopedAnimal<A, Scope>
where
    A: Animal,
    Scope: Default,
{
    type Id = A::Id;
    type Generation = A::Generation;
    type State = Scope;
    type Seed = A::Seed;
    type Journey = A::Journey;
}

/// Adapts an `BoundAct` bound to `ScopedAnimal<A, ScopeState>` to run on `A` by
/// composing a parent scope carrier with the inner act's aspect.
pub struct ScopeReboundAct<A, ScopeState, ScopeCarrier, InnerAct>(
    PhantomData<fn() -> (A, ScopeState, ScopeCarrier, InnerAct)>,
);

impl<A, ScopeState, ScopeCarrier, InnerAct> BoundAct<A>
    for ScopeReboundAct<A, ScopeState, ScopeCarrier, InnerAct>
where
    A: Animal,
    ScopeState: Default + 'static,
    ScopeCarrier: Aspect<A::State, View = ScopeState>,
    InnerAct: BoundAct<ScopedAnimal<A, ScopeState>>,
{
    type Effect = <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::Effect;
    type Aspect =
        ComposeCarrier<ScopeCarrier, <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::Aspect>;
    type Input = <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::Input;
    type Output = <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::Output;

    fn emit(
        view: &<<Self as BoundAct<A>>::Aspect as StateCarrier<A::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::emit(view, input)
    }

    fn absorb(
        view: &mut <<Self as BoundAct<A>>::Aspect as StateCarrier<A::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        <InnerAct as BoundAct<ScopedAnimal<A, ScopeState>>>::absorb(view, output)
    }
}

impl<S, A, ScopeState, ScopeCarrier> ScopedActionSpec<A, ScopeState, ScopeCarrier> for S
where
    A: Animal,
    S: ActionSpec,
    ScopeState: Default + 'static,
    ScopeCarrier: Aspect<A::State, View = ScopeState>,
    <S as ActionSpec>::BoundAct<ScopedAnimal<A, ScopeState>>: BoundAct<
        ScopedAnimal<A, ScopeState>,
        Input = <S as ActionSpec>::Input,
        Output = <S as ActionSpec>::Output,
        Effect = <S as ActionSpec>::Effect,
    >,
{
    type BoundAct = ScopeReboundAct<
        A,
        ScopeState,
        ScopeCarrier,
        <S as ActionSpec>::BoundAct<ScopedAnimal<A, ScopeState>>,
    >;
}

/// Forward half of [`BoundAct`], responsible for producing an effect request input.
pub trait Emit<T: Animal> {
    type Arg;
    type Aspect: Aspect<T::State>;
    type Effect: EffectSchema;

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as EffectSchema>::In;
}

/// Backward half of [`BoundAct`], responsible for consuming an effect completion.
pub trait Absorb<T: Animal> {
    type Ret;
    type Aspect: Aspect<T::State>;
    type Effect: EffectSchema;

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret;
}

/// Emits by forwarding carry input directly as effect input.
pub struct PassthroughEmit<A, Focus, In = <A as EffectSchema>::In>(
    PhantomData<fn() -> (A, Focus, In)>,
);

impl<T, A, Focus, In> Emit<T> for PassthroughEmit<A, Focus, In>
where
    T: Animal,
    A: EffectSchema<In = In>,
    Focus: Aspect<T::State>,
{
    type Arg = In;
    type Aspect = Focus;
    type Effect = A;

    fn emit(
        _view: &<Self::Aspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as EffectSchema>::In {
        input
    }
}

/// Emits canonical unit input for effects whose input type is `()`.
pub struct UnitEmit<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, A, Focus> Emit<T> for UnitEmit<A, Focus>
where
    T: Animal,
    A: EffectSchema<In = ()>,
    Focus: Aspect<T::State>,
{
    type Arg = ();
    type Aspect = Focus;
    type Effect = A;

    fn emit(
        _view: &<Self::Aspect as StateCarrier<T::State>>::View,
        _input: Self::Arg,
    ) -> <Self::Effect as EffectSchema>::In {
    }
}

/// Type-level callable adapter used by [`EmitFn`].
pub trait EmitMapper<View, A, In>
where
    A: EffectSchema,
{
    fn emit(view: &View, input: In) -> A::In;
}

/// Emits via a type-level mapper function.
pub struct EmitFn<Focus, A, In, F>(PhantomData<fn() -> (Focus, A, In, F)>);

impl<T, Focus, A, In, F> Emit<T> for EmitFn<Focus, A, In, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: EffectSchema,
    F: EmitMapper<<Focus as StateCarrier<T::State>>::View, A, In>,
{
    type Arg = In;
    type Aspect = Focus;
    type Effect = A;

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as EffectSchema>::In {
        <F as EmitMapper<<Focus as StateCarrier<T::State>>::View, A, In>>::emit(view, input)
    }
}

/// Type-level callable adapter used by [`AbsorbFn`].
pub trait AbsorbMapper<View, A, Out>
where
    A: EffectSchema,
{
    fn absorb(view: &mut View, output: EffectCompletion<A>) -> Out;
}

/// Absorbs via a type-level mapper function.
pub struct AbsorbFn<Focus, A, Out, F>(PhantomData<fn() -> (Focus, A, Out, F)>);

impl<T, Focus, A, Out, F> Absorb<T> for AbsorbFn<Focus, A, Out, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: EffectSchema,
    F: AbsorbMapper<<Focus as StateCarrier<T::State>>::View, A, Out>,
{
    type Ret = Out;
    type Aspect = Focus;
    type Effect = A;

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret {
        <F as AbsorbMapper<<Focus as StateCarrier<T::State>>::View, A, Out>>::absorb(view, output)
    }
}

/// Combines independent [`Emit`] and [`Absorb`] implementations into [`BoundAct`].
pub struct Fuse<E, A>(PhantomData<fn() -> (E, A)>);

impl<T, E, A> BoundAct<T> for Fuse<E, A>
where
    T: Animal,
    E: Emit<T>,
    A: Absorb<T, Effect = <E as Emit<T>>::Effect, Aspect = <E as Emit<T>>::Aspect>,
{
    type Effect = <E as Emit<T>>::Effect;
    type Aspect = <E as Emit<T>>::Aspect;
    type Input = <E as Emit<T>>::Arg;
    type Output = <A as Absorb<T>>::Ret;

    fn emit(
        view: &<<Self as BoundAct<T>>::Aspect as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        <E as Emit<T>>::emit(view, input)
    }

    fn absorb(
        view: &mut <<Self as BoundAct<T>>::Aspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        <A as Absorb<T>>::absorb(view, output)
    }
}

/// Enforces a specific [`Aspect`] for an [`Emit`] implementation.
pub struct FocusedEmit<Focus, E>(PhantomData<fn() -> (Focus, E)>);

impl<T, Focus, E> Emit<T> for FocusedEmit<Focus, E>
where
    T: Animal,
    Focus: Aspect<T::State>,
    E: Emit<T, Aspect = Focus>,
{
    type Arg = <E as Emit<T>>::Arg;
    type Aspect = Focus;
    type Effect = <E as Emit<T>>::Effect;

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as EffectSchema>::In {
        <E as Emit<T>>::emit(view, input)
    }
}

/// Enforces a specific [`Aspect`] for an [`Absorb`] implementation.
pub struct FocusedAbsorb<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Absorb<T> for FocusedAbsorb<Focus, A>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Absorb<T, Aspect = Focus>,
{
    type Ret = <A as Absorb<T>>::Ret;
    type Aspect = Focus;
    type Effect = <A as Absorb<T>>::Effect;

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret {
        <A as Absorb<T>>::absorb(view, output)
    }
}

/// Alias for an [`Fuse`] step focused by a specific [`Aspect`].
pub type FocusedStep<T, Focus, E, B> =
    BoundStep<T, Fuse<FocusedEmit<Focus, E>, FocusedAbsorb<Focus, B>>>;

/// Identity-focused [`FocusedStep`].
pub type IdentityStep<T, E, B> = FocusedStep<T, Identity, E, B>;

/// An unbound step node that defers animal binding until flow finalization.
pub struct StepSpec<S>
where
    S: ActionSpec,
{
    marker: PhantomData<fn() -> S>,
}

impl<S> StepSpec<S>
where
    S: ActionSpec,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

/// Alias used by flow templates for unbound steps.
pub type UStep<S> = StepSpec<S>;

/// A primitive workflow step that adapts an [`Effect`] to the
/// [`Running`]/[`Waiting`] protocol.
pub struct BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    marker: PhantomData<fn() -> (T, A)>,
}

impl<T, A> BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[primitive(property = crate::JungleRunning)]
impl<T, A> Running for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    type In = (T::State, <A as BoundAct<T>>::Input);
    type Out = (T::State, EffectRequest<<A as BoundAct<T>>::Effect>);

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<A as BoundAct<T>>::Aspect as StateCarrier<T::State>>::view(&mut state);
        let effect_input = <A as BoundAct<T>>::emit(view, input);
        (
            state,
            EffectRequest::<<A as BoundAct<T>>::Effect>::new(effect_input),
        )
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, A> Waiting for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    type In = (T::State, EffectCompletion<<A as BoundAct<T>>::Effect>);
    type Out = (T::State, <A as BoundAct<T>>::Output);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let view = <<A as BoundAct<T>>::Aspect as StateCarrier<T::State>>::view(&mut state);
        let emitted = <A as BoundAct<T>>::absorb(view, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, A> JourneyEffects for BoundStep<T, A>
where
    T: Animal,
    <A as BoundAct<T>>::Effect: EffectMember,
    A: BoundAct<T>,
{
    type List = Node<<<A as BoundAct<T>>::Effect as EffectIdentity>::Id, <A as BoundAct<T>>::Effect>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<T, A> TraverseFlow for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    type Output = BoundStep<T, A>;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<T, A> ReplaceFlow for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
{
    type Output = BoundStep<T, A>;
}

impl<T, A, Traversal> TraverseWith<Traversal> for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
    Traversal: TraverseStep<BoundStep<T, A>>,
{
    type Output = <Traversal as TraverseStep<BoundStep<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceWith<Replacer> for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
    Replacer: ReplaceStep<BoundStep<T, A>>,
{
    type Output = <Replacer as ReplaceStep<BoundStep<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceNodesWith<Replacer> for BoundStep<T, A>
where
    T: Animal,
    A: BoundAct<T>,
    Replacer: ReplaceNode<BoundStep<T, A>>,
{
    type Output = <Replacer as ReplaceNode<BoundStep<T, A>>>::Output;
}

#[primitive(property = crate::JungleFlow)]
impl<S> JourneyEffects for StepSpec<S>
where
    S: ActionSpec,
{
    type List = Node<<<S as ActionSpec>::Effect as EffectIdentity>::Id, <S as ActionSpec>::Effect>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<S> TraverseFlow for StepSpec<S>
where
    S: ActionSpec,
{
    type Output = StepSpec<S>;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<S> ReplaceFlow for StepSpec<S>
where
    S: ActionSpec,
{
    type Output = StepSpec<S>;
}

impl<S, Traversal> TraverseWith<Traversal> for StepSpec<S>
where
    S: ActionSpec,
    Traversal: TraverseStep<StepSpec<S>>,
{
    type Output = <Traversal as TraverseStep<StepSpec<S>>>::Output;
}

impl<S, Replacer> ReplaceWith<Replacer> for StepSpec<S>
where
    S: ActionSpec,
    Replacer: ReplaceStep<StepSpec<S>>,
{
    type Output = <Replacer as ReplaceStep<StepSpec<S>>>::Output;
}

impl<S, Replacer> ReplaceNodesWith<Replacer> for StepSpec<S>
where
    S: ActionSpec,
    Replacer: ReplaceNode<StepSpec<S>>,
{
    type Output = <Replacer as ReplaceNode<StepSpec<S>>>::Output;
}

impl<T, S> TraverseStep<StepSpec<S>> for crate::BindAnimalTraversal<T, crate::RootScope>
where
    T: Animal,
    S: ActionSpec,
    <S as ActionSpec>::BoundAct<T>: BoundAct<
        T,
        Input = <S as ActionSpec>::Input,
        Output = <S as ActionSpec>::Output,
        Effect = <S as ActionSpec>::Effect,
    >,
{
    type Output = BoundStep<T, <S as ActionSpec>::BoundAct<T>>;
}

impl<T, ScopeCarrier, S> TraverseStep<StepSpec<S>> for crate::BindAnimalTraversal<T, ScopeCarrier>
where
    T: Animal,
    ScopeCarrier: crate::ScopedCarrierMarker,
    ScopeCarrier: Aspect<T::State>,
    S: ActionSpec,
    S: ScopedActionSpec<T, <ScopeCarrier as StateCarrier<T::State>>::View, ScopeCarrier>,
    <S as ScopedActionSpec<
        T,
        <ScopeCarrier as StateCarrier<T::State>>::View,
        ScopeCarrier,
    >>::BoundAct: BoundAct<
        T,
        Input = <S as ActionSpec>::Input,
        Output = <S as ActionSpec>::Output,
        Effect = <S as ActionSpec>::Effect,
    >,
{
    type Output = BoundStep<
        T,
        <S as ScopedActionSpec<
            T,
            <ScopeCarrier as StateCarrier<T::State>>::View,
            ScopeCarrier,
        >>::BoundAct,
    >;
}
