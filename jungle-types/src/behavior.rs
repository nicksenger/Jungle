use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Sub;

use crate::{
    EffectMember, Animal, FlowEffects, ReplaceFlow, ReplaceNode, ReplaceNodesWith, ReplaceStep,
    ReplaceWith, Running, TraverseFlow, TraverseStep, TraverseWith, Waiting,
};
use inception::{primitive, Access, Field, Inception as InceptionTy, VariantHeader};
use typosaurus::collections::list;
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::{U0, U1};
use typosaurus::num::{Bit, UInt, Unsigned};

/// A behavior that transforms a single input into a single output.
pub trait Effect {
    /// A type-level identifier for this Effect.
    type Id;

    /// The shared dependency consumed by this effect.
    type Dependency: Send + Sync + 'static;

    /// The input type accepted by this effect.
    type In: Serialize + DeserializeOwned + Send + 'static;

    /// The output type produced by this effect.
    type Out: Serialize + DeserializeOwned + Send + 'static;

    /// The error type produced by this effect.
    type Err: Send + 'static;

    /// Process one input into one output.
    fn act(
        dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> + Send;
}

/// A typed effect request emitted by a yielding workflow phase.
pub struct EffectRequest<A: Effect> {
    pub input: A::In,
    marker: PhantomData<fn() -> A>,
}

impl<A: Effect> EffectRequest<A> {
    pub fn new(input: A::In) -> Self {
        Self {
            input,
            marker: PhantomData,
        }
    }

    pub fn into_input(self) -> A::In {
        self.input
    }

    pub fn act<'a>(
        self,
        dependency: &'a A::Dependency,
    ) -> impl Future<Output = Result<A::Out, A::Err>> + 'a
    where
        A: 'a,
    {
        A::act(dependency, self.input)
    }
}

/// A completed effect result consumed by an awaiting workflow phase.
pub type EffectCompletion<A> = Result<<A as Effect>::Out, <A as Effect>::Err>;

/// Projects a larger state into a focused mutable substate.
pub trait StateCarrier<State> {
    type View;

    fn view(state: &mut State) -> &mut Self::View;
}

// Compatibility shim during carrier-trait migration.
pub trait Aspect<State>: StateCarrier<State> {}

impl<T, State> Aspect<State> for T where T: StateCarrier<State> {}

/// Focuses to the full state itself.
pub struct Identity;

impl<State> StateCarrier<State> for Identity {
    type View = State;

    fn view(state: &mut State) -> &mut Self::View {
        state
    }
}

/// Focuses to a field on a state type by its type-level field index.
pub struct StateLens<State, Index>(PhantomData<fn() -> (State, Index)>);

trait FieldAtMut<'a, Index, View> {
    fn at_mut(self) -> &'a mut View;
}

impl<'a, Head, Tail, View> FieldAtMut<'a, U0, View> for inception::List<(Head, Tail)>
where
    View: 'a,
    Head: Access<Out = &'a mut View>,
{
    fn at_mut(self) -> &'a mut View {
        self.0 .0.access()
    }
}

impl<'a, Head, Tail, U, B, View> FieldAtMut<'a, UInt<U, B>, View> for inception::List<(Head, Tail)>
where
    U: Unsigned,
    B: Bit,
    UInt<U, B>: Sub<U1>,
    Tail: FieldAtMut<'a, <UInt<U, B> as Sub<U1>>::Output, View>,
{
    fn at_mut(self) -> &'a mut View {
        self.0 .1.at_mut()
    }
}

#[doc(hidden)]
pub trait FieldContentAt<Index> {
    type Content;
}

impl<Head, Tail> FieldContentAt<U0> for inception::List<(Head, Tail)>
where
    Head: Field,
{
    type Content = <Head as Field>::Content;
}

impl<Head, Tail, U, B> FieldContentAt<UInt<U, B>> for inception::List<(Head, Tail)>
where
    U: Unsigned,
    B: Bit,
    UInt<U, B>: Sub<U1>,
    Tail: FieldContentAt<<UInt<U, B> as Sub<U1>>::Output>,
{
    type Content = <Tail as FieldContentAt<<UInt<U, B> as Sub<U1>>::Output>>::Content;
}

#[doc(hidden)]
pub trait StateLensPath<State, Index> {
    type View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View;
}

trait ScalarIndex {}

impl ScalarIndex for U0 {}

impl<U, B> ScalarIndex for UInt<U, B>
where
    U: Unsigned,
    B: Bit,
{
}

impl<State, Index> StateLensPath<State, Index> for ()
where
    Index: ScalarIndex,
    State: crate::Optic
        + InceptionTy<crate::JungleOptic, inception::False>
        + inception::DataType<Ty = inception::StructTy<inception::True>>,
    <State as InceptionTy<crate::JungleOptic, inception::False>>::TyFields: FieldContentAt<Index>,
    for<'a> <State as InceptionTy<crate::JungleOptic, inception::False>>::MutFields<'a>: FieldAtMut<
        'a,
        Index,
        <<State as InceptionTy<crate::JungleOptic, inception::False>>::TyFields as FieldContentAt<Index>>::Content,
    >,
{
    type View =
        <<State as InceptionTy<crate::JungleOptic, inception::False>>::TyFields as FieldContentAt<Index>>::Content;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        let mut header = VariantHeader;
        let fields =
            <State as InceptionTy<crate::JungleOptic, inception::False>>::fields_mut(state, &mut header);
        fields.at_mut()
    }
}

impl<State, Head, Next, Tail> StateLensPath<State, list::List<(Head, list::List<(Next, Tail)>)>>
    for ()
where
    (): StateLensPath<State, Head>,
    <() as StateLensPath<State, Head>>::View: 'static,
    (): StateLensPath<<() as StateLensPath<State, Head>>::View, list::List<(Next, Tail)>>,
{
    type View = <() as StateLensPath<
        <() as StateLensPath<State, Head>>::View,
        list::List<(Next, Tail)>,
    >>::View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        let head = <() as StateLensPath<State, Head>>::view(state);
        <() as StateLensPath<<() as StateLensPath<State, Head>>::View, list::List<(Next, Tail)>>>::view(head)
    }
}

impl<State, Head> StateLensPath<State, list::List<(Head, list::Empty)>> for ()
where
    (): StateLensPath<State, Head>,
{
    type View = <() as StateLensPath<State, Head>>::View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        <() as StateLensPath<State, Head>>::view(state)
    }
}

impl<State, Index> StateCarrier<State> for StateLens<State, Index>
where
    (): StateLensPath<State, Index>,
{
    type View = <() as StateLensPath<State, Index>>::View;

    fn view(state: &mut State) -> &mut Self::View {
        <() as StateLensPath<State, Index>>::view(state)
    }
}

/// Single step-facing contract for adapting an [`Effect`] over an [`Aspect`]
/// of animal state.
pub trait Act<T: Animal> {
    type Effect: Effect;
    type StateAspect: Aspect<T::State>;
    type Input;
    type Output;

    fn emit(
        view: &<<Self as Act<T>>::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as Effect>::In;

    fn absorb(
        view: &mut <<Self as Act<T>>::StateAspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output;
}

/// Forward half of [`Act`], responsible for producing an effect request input.
pub trait Emit<T: Animal> {
    type Arg;
    type StateAspect: Aspect<T::State>;
    type Effect: Effect;

    fn emit(
        view: &<Self::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as Effect>::In;
}

/// Backward half of [`Act`], responsible for consuming an effect completion.
pub trait Absorb<T: Animal> {
    type Ret;
    type StateAspect: Aspect<T::State>;
    type Effect: Effect;

    fn absorb(
        view: &mut <Self::StateAspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret;
}

/// Emits by forwarding carry input directly as effect input.
pub struct PassthroughEmit<A, Focus, In = <A as Effect>::In>(PhantomData<fn() -> (A, Focus, In)>);

impl<T, A, Focus, In> Emit<T> for PassthroughEmit<A, Focus, In>
where
    T: Animal,
    A: Effect<In = In>,
    Focus: Aspect<T::State>,
{
    type Arg = In;
    type StateAspect = Focus;
    type Effect = A;

    fn emit(
        _view: &<Self::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as Effect>::In {
        input
    }
}

/// Emits canonical unit input for effects whose input type is `()`.
pub struct UnitEmit<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, A, Focus> Emit<T> for UnitEmit<A, Focus>
where
    T: Animal,
    A: Effect<In = ()>,
    Focus: Aspect<T::State>,
{
    type Arg = ();
    type StateAspect = Focus;
    type Effect = A;

    fn emit(
        _view: &<Self::StateAspect as StateCarrier<T::State>>::View,
        _input: Self::Arg,
    ) -> <Self::Effect as Effect>::In {
    }
}

/// Type-level callable adapter used by [`EmitFn`].
pub trait EmitMapper<View, A, In>
where
    A: Effect,
{
    fn emit(view: &View, input: In) -> A::In;
}

/// Emits via a type-level mapper function.
pub struct EmitFn<Focus, A, In, F>(PhantomData<fn() -> (Focus, A, In, F)>);

impl<T, Focus, A, In, F> Emit<T> for EmitFn<Focus, A, In, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Effect,
    F: EmitMapper<<Focus as StateCarrier<T::State>>::View, A, In>,
{
    type Arg = In;
    type StateAspect = Focus;
    type Effect = A;

    fn emit(
        view: &<Self::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as Effect>::In {
        <F as EmitMapper<<Focus as StateCarrier<T::State>>::View, A, In>>::emit(view, input)
    }
}

/// Type-level callable adapter used by [`AbsorbFn`].
pub trait AbsorbMapper<View, A, Out>
where
    A: Effect,
{
    fn absorb(view: &mut View, output: EffectCompletion<A>) -> Out;
}

/// Absorbs via a type-level mapper function.
pub struct AbsorbFn<Focus, A, Out, F>(PhantomData<fn() -> (Focus, A, Out, F)>);

impl<T, Focus, A, Out, F> Absorb<T> for AbsorbFn<Focus, A, Out, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Effect,
    F: AbsorbMapper<<Focus as StateCarrier<T::State>>::View, A, Out>,
{
    type Ret = Out;
    type StateAspect = Focus;
    type Effect = A;

    fn absorb(
        view: &mut <Self::StateAspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret {
        <F as AbsorbMapper<<Focus as StateCarrier<T::State>>::View, A, Out>>::absorb(view, output)
    }
}

/// Combines independent [`Emit`] and [`Absorb`] implementations into [`Act`].
pub struct Fuse<E, A>(PhantomData<fn() -> (E, A)>);

impl<T, E, A> Act<T> for Fuse<E, A>
where
    T: Animal,
    E: Emit<T>,
    A: Absorb<T, Effect = <E as Emit<T>>::Effect, StateAspect = <E as Emit<T>>::StateAspect>,
{
    type Effect = <E as Emit<T>>::Effect;
    type StateAspect = <E as Emit<T>>::StateAspect;
    type Input = <E as Emit<T>>::Arg;
    type Output = <A as Absorb<T>>::Ret;

    fn emit(
        view: &<<Self as Act<T>>::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as Effect>::In {
        <E as Emit<T>>::emit(view, input)
    }

    fn absorb(
        view: &mut <<Self as Act<T>>::StateAspect as StateCarrier<T::State>>::View,
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
    E: Emit<T, StateAspect = Focus>,
{
    type Arg = <E as Emit<T>>::Arg;
    type StateAspect = Focus;
    type Effect = <E as Emit<T>>::Effect;

    fn emit(
        view: &<Self::StateAspect as StateCarrier<T::State>>::View,
        input: Self::Arg,
    ) -> <Self::Effect as Effect>::In {
        <E as Emit<T>>::emit(view, input)
    }
}

/// Enforces a specific [`Aspect`] for an [`Absorb`] implementation.
pub struct FocusedAbsorb<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Absorb<T> for FocusedAbsorb<Focus, A>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Absorb<T, StateAspect = Focus>,
{
    type Ret = <A as Absorb<T>>::Ret;
    type StateAspect = Focus;
    type Effect = <A as Absorb<T>>::Effect;

    fn absorb(
        view: &mut <Self::StateAspect as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Ret {
        <A as Absorb<T>>::absorb(view, output)
    }
}

/// Alias for an [`Fuse`] step focused by a specific [`Aspect`].
pub type FocusedStep<T, Focus, E, B> =
    Step<T, Fuse<FocusedEmit<Focus, E>, FocusedAbsorb<Focus, B>>>;

/// Identity-focused [`FocusedStep`].
pub type IdentityStep<T, E, B> = FocusedStep<T, Identity, E, B>;

/// A primitive workflow step that adapts an [`Effect`] to the
/// [`Running`]/[`Waiting`] protocol.
pub struct Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    marker: PhantomData<fn() -> (T, A)>,
}

impl<T, A> Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[primitive(property = crate::JungleRunning)]
impl<T, A> Running for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    type In = (T::State, <A as Act<T>>::Input);
    type Out = (T::State, EffectRequest<<A as Act<T>>::Effect>);

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<A as Act<T>>::StateAspect as StateCarrier<T::State>>::view(&mut state);
        let effect_input = <A as Act<T>>::emit(view, input);
        (
            state,
            EffectRequest::<<A as Act<T>>::Effect>::new(effect_input),
        )
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, A> Waiting for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    type In = (T::State, EffectCompletion<<A as Act<T>>::Effect>);
    type Out = (T::State, <A as Act<T>>::Output);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let view = <<A as Act<T>>::StateAspect as StateCarrier<T::State>>::view(&mut state);
        let emitted = <A as Act<T>>::absorb(view, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, A> FlowEffects for Step<T, A>
where
    T: Animal,
    <A as Act<T>>::Effect: EffectMember,
    A: Act<T>,
{
    type List = Node<<<A as Act<T>>::Effect as Effect>::Id, <A as Act<T>>::Effect>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<T, A> TraverseFlow for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    type Output = Step<T, A>;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<T, A> ReplaceFlow for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    type Output = Step<T, A>;
}

impl<T, A, Traversal> TraverseWith<Traversal> for Step<T, A>
where
    T: Animal,
    A: Act<T>,
    Traversal: TraverseStep<Step<T, A>>,
{
    type Output = <Traversal as TraverseStep<Step<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceWith<Replacer> for Step<T, A>
where
    T: Animal,
    A: Act<T>,
    Replacer: ReplaceStep<Step<T, A>>,
{
    type Output = <Replacer as ReplaceStep<Step<T, A>>>::Output;
}

impl<T, A, Replacer> ReplaceNodesWith<Replacer> for Step<T, A>
where
    T: Animal,
    A: Act<T>,
    Replacer: ReplaceNode<Step<T, A>>,
{
    type Output = <Replacer as ReplaceNode<Step<T, A>>>::Output;
}
