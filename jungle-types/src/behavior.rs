use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;

use crate::{
    Animal, EffectIdentity, EffectMember, Failure, JourneyEffects, ReplaceFlow, ReplaceNode,
    ReplaceNodesWith, ReplaceStep, ReplaceWith, Running, TraverseFlow, TraverseStep, TraverseWith,
    Waiting,
};
use inception::primitive;
use typosaurus::collections::sp::Node;

/// Canonical effect contract used by flow shape and wire schema.
pub trait EffectSchema<J = ()> {
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
pub trait Effect<J>: EffectSchema<J> {
    /// Process one input into one output in the provided context.
    fn effect(
        jungle: &J,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> + Send;
}

/// A typed effect request emitted by a yielding workflow phase.
pub struct EffectRequest<A: EffectSchema<J>, J = ()> {
    pub input: <A as EffectSchema<J>>::In,
    marker: PhantomData<fn() -> A>,
}

impl<A: EffectSchema<J>, J> EffectRequest<A, J> {
    pub fn new(input: <A as EffectSchema<J>>::In) -> Self {
        Self {
            input,
            marker: PhantomData,
        }
    }

    pub fn into_input(self) -> <A as EffectSchema<J>>::In {
        self.input
    }

    pub fn effect<'a>(
        self,
        jungle: &'a J,
    ) -> impl Future<Output = Result<<A as EffectSchema<J>>::Out, <A as EffectSchema<J>>::Err>> + 'a
    where
        A: Effect<J> + 'a,
    {
        A::effect(jungle, self.input)
    }
}

/// A completed effect result consumed by an awaiting workflow phase.
pub type EffectCompletion<A, J = ()> =
    Result<<A as EffectSchema<J>>::Out, <A as EffectSchema<J>>::Err>;

/// Projects a larger state into a focused mutable substate.
pub trait StateCarrier<State> {
    type Focus;

    fn focus(state: &mut State) -> &mut Self::Focus;
}

/// Composes two carriers into a single projection.
pub struct ComposeCarrier<Outer, Inner>(PhantomData<fn() -> (Outer, Inner)>);

impl<State, Outer, Inner> StateCarrier<State> for ComposeCarrier<Outer, Inner>
where
    Outer: StateCarrier<State>,
    <Outer as StateCarrier<State>>::Focus: 'static,
    Inner: StateCarrier<<Outer as StateCarrier<State>>::Focus>,
{
    type Focus = <Inner as StateCarrier<<Outer as StateCarrier<State>>::Focus>>::Focus;

    fn focus(state: &mut State) -> &mut Self::Focus {
        let outer = <Outer as StateCarrier<State>>::focus(state);
        <Inner as StateCarrier<<Outer as StateCarrier<State>>::Focus>>::focus(outer)
    }
}

// Compatibility shim during carrier-trait migration.
pub trait Aspect<State>: StateCarrier<State> {}

impl<T, State> Aspect<State> for T where T: StateCarrier<State> {}

/// Focuses to the full state itself.
pub struct Identity;

impl<State> StateCarrier<State> for Identity {
    type Focus = State;

    fn focus(state: &mut State) -> &mut Self::Focus {
        state
    }
}

/// Single step-facing contract for adapting an [`Effect`] over an [`Aspect`]
/// of animal state.
pub trait BoundAction<T: Animal> {
    type Effect: EffectSchema;
    type Aspect: Aspect<T::State>;
    type Input;
    type Output;
    type Carry;

    fn emit(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In;

    fn absorb(
        view: &mut <<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure>;

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry);

    fn absorb_with_carry(
        view: &mut <<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        _carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Self::absorb(view, output)
    }
}

/// Late-bound action spec that can be bound to a concrete [`Animal`] at the edge.
pub trait Action {
    type Effect: EffectMember;
    type Input;
    type Output;
    type Carry;
    type Bind<A: Animal>;
}

/// Re-binds an action spec authored for one scope to another scope.
pub trait ScopedAction<A: Animal, ScopeState, ScopeCarrier> {
    type BoundAction: BoundAction<A>;
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
    type Flow = A::Flow;
}

/// Adapts an `BoundAction` bound to `ScopedAnimal<A, ScopeState>` to run on `A` by
/// composing a parent scope carrier with the inner act's aspect.
pub struct ScopeReboundAction<A, ScopeState, ScopeCarrier, InnerAct>(
    PhantomData<fn() -> (A, ScopeState, ScopeCarrier, InnerAct)>,
);

impl<A, ScopeState, ScopeCarrier, InnerAct> BoundAction<A>
    for ScopeReboundAction<A, ScopeState, ScopeCarrier, InnerAct>
where
    A: Animal,
    ScopeState: Default + 'static,
    ScopeCarrier: Aspect<A::State, Focus = ScopeState>,
    InnerAct: BoundAction<ScopedAnimal<A, ScopeState>>,
{
    type Effect = <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::Effect;
    type Aspect = ComposeCarrier<
        ScopeCarrier,
        <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::Aspect,
    >;
    type Input = <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::Input;
    type Output = <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::Output;
    type Carry = <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::Carry;

    fn emit(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::emit(view, input)
    }

    fn absorb(
        view: &mut <<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::absorb(view, output)
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::emit_with_carry(view, input)
    }

    fn absorb_with_carry(
        view: &mut <<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        <InnerAct as BoundAction<ScopedAnimal<A, ScopeState>>>::absorb_with_carry(
            view, output, carry,
        )
    }
}

impl<S, A, ScopeState, ScopeCarrier> ScopedAction<A, ScopeState, ScopeCarrier> for S
where
    A: Animal,
    S: Action,
    ScopeState: Default + 'static,
    ScopeCarrier: Aspect<A::State, Focus = ScopeState>,
    <S as Action>::Bind<ScopedAnimal<A, ScopeState>>: BoundAction<
        ScopedAnimal<A, ScopeState>,
        Input = <S as Action>::Input,
        Output = <S as Action>::Output,
        Effect = <S as Action>::Effect,
        Carry = <S as Action>::Carry,
    >,
{
    type BoundAction = ScopeReboundAction<
        A,
        ScopeState,
        ScopeCarrier,
        <S as Action>::Bind<ScopedAnimal<A, ScopeState>>,
    >;
}

/// Forward half of [`BoundAction`], responsible for producing an effect request input.
pub trait Emit<T: Animal> {
    type Arg;
    type Aspect: Aspect<T::State>;
    type Effect: EffectSchema;
    type Carry;

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Arg,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry);
}

/// Backward half of [`BoundAction`], responsible for consuming an effect completion.
pub trait Absorb<T: Animal> {
    type Ret;
    type Aspect: Aspect<T::State>;
    type Effect: EffectSchema;
    type Carry;

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Ret, Failure>;
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
    type Carry = ();

    fn emit(
        _view: &<Self::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Arg,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (input, ())
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
    type Carry = ();

    fn emit(
        _view: &<Self::Aspect as StateCarrier<T::State>>::Focus,
        _input: Self::Arg,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), ())
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
    F: EmitMapper<<Focus as StateCarrier<T::State>>::Focus, A, In>,
{
    type Arg = In;
    type Aspect = Focus;
    type Effect = A;
    type Carry = ();

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Arg,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (
            <F as EmitMapper<<Focus as StateCarrier<T::State>>::Focus, A, In>>::emit(view, input),
            (),
        )
    }
}

/// Type-level callable adapter used by [`AbsorbFn`].
pub trait AbsorbMapper<View, A, Out>
where
    A: EffectSchema,
{
    fn absorb(view: &mut View, output: EffectCompletion<A>) -> Result<Out, Failure>;
}

/// Absorbs via a type-level mapper function.
pub struct AbsorbFn<Focus, A, Out, F>(PhantomData<fn() -> (Focus, A, Out, F)>);

impl<T, Focus, A, Out, F> Absorb<T> for AbsorbFn<Focus, A, Out, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: EffectSchema,
    F: AbsorbMapper<<Focus as StateCarrier<T::State>>::Focus, A, Out>,
{
    type Ret = Out;
    type Aspect = Focus;
    type Effect = A;
    type Carry = ();

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        _carry: Self::Carry,
    ) -> Result<Self::Ret, Failure> {
        <F as AbsorbMapper<<Focus as StateCarrier<T::State>>::Focus, A, Out>>::absorb(view, output)
    }
}

/// Combines independent [`Emit`] and [`Absorb`] implementations into [`BoundAction`].
pub struct Fuse<E, A>(PhantomData<fn() -> (E, A)>);

impl<T, E, A> BoundAction<T> for Fuse<E, A>
where
    T: Animal,
    E: Emit<T>,
    A: Absorb<
        T,
        Effect = <E as Emit<T>>::Effect,
        Aspect = <E as Emit<T>>::Aspect,
        Carry = <E as Emit<T>>::Carry,
    >,
{
    type Effect = <E as Emit<T>>::Effect;
    type Aspect = <E as Emit<T>>::Aspect;
    type Input = <E as Emit<T>>::Arg;
    type Output = <A as Absorb<T>>::Ret;
    type Carry = <E as Emit<T>>::Carry;

    fn emit(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        <E as Emit<T>>::emit(view, input).0
    }

    fn absorb(
        _view: &mut <<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        panic!("`absorb` is unavailable for carry-enabled fused acts; use `absorb_with_carry`.")
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        <E as Emit<T>>::emit(view, input)
    }

    fn absorb_with_carry(
        view: &mut <<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        <A as Absorb<T>>::absorb(view, output, carry)
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
    type Carry = <E as Emit<T>>::Carry;

    fn emit(
        view: &<Self::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Arg,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
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
    type Carry = <A as Absorb<T>>::Carry;

    fn absorb(
        view: &mut <Self::Aspect as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Ret, Failure> {
        <A as Absorb<T>>::absorb(view, output, carry)
    }
}

/// Alias for an [`Fuse`] step focused by a specific [`Aspect`].
pub type FocusedStep<T, Focus, E, B> =
    BoundFlowStep<T, Fuse<FocusedEmit<Focus, E>, FocusedAbsorb<Focus, B>>>;

/// Identity-focused [`FocusedStep`].
pub type IdentityStep<T, E, B> = FocusedStep<T, Identity, E, B>;

/// An unbound step node that defers animal binding until flow finalization.
pub struct Step<S>
where
    S: Action,
{
    marker: PhantomData<fn() -> S>,
}

impl<S> Step<S>
where
    S: Action,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<S> Default for Step<S>
where
    S: Action,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A primitive workflow step that adapts an [`Effect`] to the
/// [`Running`]/[`Waiting`] protocol.
pub struct BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    marker: PhantomData<fn() -> (T, A)>,
}

impl<T, A> BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T, A> Default for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[primitive(property = crate::JungleRunning)]
impl<T, A> Running for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type In = (T::State, <A as BoundAction<T>>::Input);
    type Out = (
        T::State,
        (
            EffectRequest<<A as BoundAction<T>>::Effect>,
            <A as BoundAction<T>>::Carry,
        ),
    );

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<A as BoundAction<T>>::Aspect as StateCarrier<T::State>>::focus(&mut state);
        let (effect_input, carry) = <A as BoundAction<T>>::emit_with_carry(view, input);
        (
            state,
            (
                EffectRequest::<<A as BoundAction<T>>::Effect>::new(effect_input),
                carry,
            ),
        )
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, A> Waiting for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type In = (
        T::State,
        EffectCompletion<<A as BoundAction<T>>::Effect>,
        <A as BoundAction<T>>::Carry,
    );
    type Out = (T::State, <A as BoundAction<T>>::Output);

    fn accept((mut state, output, carry): Self::In) -> Self::Out {
        let view = <<A as BoundAction<T>>::Aspect as StateCarrier<T::State>>::focus(&mut state);
        let emitted = <A as BoundAction<T>>::absorb_with_carry(view, output, carry)
            .expect("absorb failures must be handled by the executor");
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, A> JourneyEffects for BoundFlowStep<T, A>
where
    T: Animal,
    <A as BoundAction<T>>::Effect: EffectMember,
    A: BoundAction<T>,
{
    type List =
        Node<<<A as BoundAction<T>>::Effect as EffectIdentity>::Id, <A as BoundAction<T>>::Effect>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<T, A> crate::TraverseFlowShape for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type Output = BoundFlowStep<T, A>;
}

impl<T, A> TraverseFlow for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type Output = <Self as crate::TraverseFlowShape>::Output;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<T, A> ReplaceFlow for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type Output = BoundFlowStep<T, A>;
}

impl<T, A, Traversal> TraverseWith<Traversal> for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
    Traversal: TraverseStep<BoundFlowStep<T, A>>,
{
    type Output = <Traversal as TraverseStep<BoundFlowStep<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceWith<Replacer> for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
    Replacer: ReplaceStep<BoundFlowStep<T, A>>,
{
    type Output = <Replacer as ReplaceStep<BoundFlowStep<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceNodesWith<Replacer> for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
    Replacer: ReplaceNode<BoundFlowStep<T, A>>,
{
    type Output = <Replacer as ReplaceNode<BoundFlowStep<T, A>>>::Output;
}

#[primitive(property = crate::JungleFlow)]
impl<S> JourneyEffects for Step<S>
where
    S: Action,
{
    type List = Node<<<S as Action>::Effect as EffectIdentity>::Id, <S as Action>::Effect>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<S> crate::TraverseFlowShape for Step<S>
where
    S: Action,
{
    type Output = Step<S>;
}

impl<S> TraverseFlow for Step<S>
where
    S: Action,
{
    type Output = <Self as crate::TraverseFlowShape>::Output;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<S> ReplaceFlow for Step<S>
where
    S: Action,
{
    type Output = Step<S>;
}

impl<S, Traversal> TraverseWith<Traversal> for Step<S>
where
    S: Action,
    Traversal: TraverseStep<Step<S>>,
{
    type Output = <Traversal as TraverseStep<Step<S>>>::Output;
}

impl<S, Replacer> ReplaceWith<Replacer> for Step<S>
where
    S: Action,
    Replacer: ReplaceStep<Step<S>>,
{
    type Output = <Replacer as ReplaceStep<Step<S>>>::Output;
}

impl<S, Replacer> ReplaceNodesWith<Replacer> for Step<S>
where
    S: Action,
    Replacer: ReplaceNode<Step<S>>,
{
    type Output = <Replacer as ReplaceNode<Step<S>>>::Output;
}

impl<T, S> TraverseStep<Step<S>> for crate::BindAnimalTraversal<T, crate::RootScope>
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
    type Output = BoundFlowStep<T, <S as Action>::Bind<T>>;
}

impl<T, ScopeCarrier, S> TraverseStep<Step<S>> for crate::BindAnimalTraversal<T, ScopeCarrier>
where
    T: Animal,
    ScopeCarrier: crate::ScopedCarrierMarker,
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
    type Output = BoundFlowStep<
        T,
        <S as ScopedAction<
            T,
            <ScopeCarrier as StateCarrier<T::State>>::Focus,
            ScopeCarrier,
        >>::BoundAction,
    >;
}
