use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Sub;

use crate::{
    ActionMember, Animal, FlowActions, ReplaceFlow, ReplaceNode, ReplaceNodesWith, ReplaceStep,
    ReplaceWith, Running, TraverseFlow, TraverseStep, TraverseWith, Waiting,
};
use inception::{primitive, Access, Field, Inception as InceptionTy, VariantHeader};
use typosaurus::collections::list;
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::{U0, U1};
use typosaurus::num::{Bit, UInt, Unsigned};

/// A behavior that transforms a single input into a single output.
pub trait Action {
    /// A type-level identifier for this Action.
    type Id;

    /// The shared dependency consumed by this action.
    type Dependency;

    /// The input type accepted by this action.
    type In: Serialize + DeserializeOwned;

    /// The output type produced by this action.
    type Out: Serialize + DeserializeOwned;

    /// The error type produced by this action.
    type Err;

    /// Process one input into one output.
    fn act(
        dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>>;
}

/// A typed action request emitted by a yielding workflow phase.
pub struct ActionRequest<A: Action> {
    pub input: A::In,
    marker: PhantomData<fn() -> A>,
}

impl<A: Action> ActionRequest<A> {
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

/// A completed action result consumed by an awaiting workflow phase.
pub type ActionCompletion<A> = Result<<A as Action>::Out, <A as Action>::Err>;

/// Projects a larger state into a focused mutable substate.
pub trait Aspect<State> {
    type View;

    fn view(state: &mut State) -> &mut Self::View;
}

/// Focuses to the full state itself.
pub struct Identity;

impl<State> Aspect<State> for Identity {
    type View = State;

    fn view(state: &mut State) -> &mut Self::View {
        state
    }
}

/// Focuses to a field on a state type by its type-level field index.
pub struct Lens<State, Index>(PhantomData<fn() -> (State, Index)>);

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
pub trait LensPath<State, Index> {
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

impl<State, Index> LensPath<State, Index> for ()
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

impl<State, Head, Next, Tail> LensPath<State, list::List<(Head, list::List<(Next, Tail)>)>> for ()
where
    (): LensPath<State, Head>,
    <() as LensPath<State, Head>>::View: 'static,
    (): LensPath<<() as LensPath<State, Head>>::View, list::List<(Next, Tail)>>,
{
    type View =
        <() as LensPath<<() as LensPath<State, Head>>::View, list::List<(Next, Tail)>>>::View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        let head = <() as LensPath<State, Head>>::view(state);
        <() as LensPath<<() as LensPath<State, Head>>::View, list::List<(Next, Tail)>>>::view(head)
    }
}

impl<State, Head> LensPath<State, list::List<(Head, list::Empty)>> for ()
where
    (): LensPath<State, Head>,
{
    type View = <() as LensPath<State, Head>>::View;

    fn view<'a>(state: &'a mut State) -> &'a mut Self::View {
        <() as LensPath<State, Head>>::view(state)
    }
}

impl<State, Index> Aspect<State> for Lens<State, Index>
where
    (): LensPath<State, Index>,
{
    type View = <() as LensPath<State, Index>>::View;

    fn view(state: &mut State) -> &mut Self::View {
        <() as LensPath<State, Index>>::view(state)
    }
}

/// Single step-facing contract for adapting an [`Action`] over an [`Aspect`]
/// of animal state.
pub trait Act<T: Animal> {
    type Action: Action;
    type Aspect: Aspect<T::State>;
    type In;
    type Out;

    fn emit(
        view: &<<Self as Act<T>>::Aspect as Aspect<T::State>>::View,
        input: Self::In,
    ) -> <Self::Action as Action>::In;

    fn absorb(
        view: &mut <<Self as Act<T>>::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Out;
}

/// Forward half of [`Act`], responsible for producing an action request input.
pub trait Emit<T: Animal> {
    type CarryIn;
    type Aspect: Aspect<T::State>;
    type Action: Action;

    fn emit(
        view: &<Self::Aspect as Aspect<T::State>>::View,
        input: Self::CarryIn,
    ) -> <Self::Action as Action>::In;
}

/// Backward half of [`Act`], responsible for consuming an action completion.
pub trait Absorb<T: Animal> {
    type CarryOut;
    type Aspect: Aspect<T::State>;
    type Action: Action;

    fn absorb(
        view: &mut <Self::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::CarryOut;
}

/// Emits by forwarding carry input directly as action input.
pub struct PassthroughEmit<A, Focus, In = <A as Action>::In>(PhantomData<fn() -> (A, Focus, In)>);

impl<T, A, Focus, In> Emit<T> for PassthroughEmit<A, Focus, In>
where
    T: Animal,
    A: Action<In = In>,
    Focus: Aspect<T::State>,
{
    type CarryIn = In;
    type Aspect = Focus;
    type Action = A;

    fn emit(
        _view: &<Self::Aspect as Aspect<T::State>>::View,
        input: Self::CarryIn,
    ) -> <Self::Action as Action>::In {
        input
    }
}

/// Emits canonical unit input for actions whose input type is `()`.
pub struct UnitEmit<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, A, Focus> Emit<T> for UnitEmit<A, Focus>
where
    T: Animal,
    A: Action<In = ()>,
    Focus: Aspect<T::State>,
{
    type CarryIn = ();
    type Aspect = Focus;
    type Action = A;

    fn emit(
        _view: &<Self::Aspect as Aspect<T::State>>::View,
        _input: Self::CarryIn,
    ) -> <Self::Action as Action>::In {
    }
}

/// Type-level callable adapter used by [`EmitFn`].
pub trait EmitMapper<View, A, In>
where
    A: Action,
{
    fn emit(view: &View, input: In) -> A::In;
}

/// Emits via a type-level mapper function.
pub struct EmitFn<Focus, A, In, F>(PhantomData<fn() -> (Focus, A, In, F)>);

impl<T, Focus, A, In, F> Emit<T> for EmitFn<Focus, A, In, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Action,
    F: EmitMapper<<Focus as Aspect<T::State>>::View, A, In>,
{
    type CarryIn = In;
    type Aspect = Focus;
    type Action = A;

    fn emit(
        view: &<Self::Aspect as Aspect<T::State>>::View,
        input: Self::CarryIn,
    ) -> <Self::Action as Action>::In {
        <F as EmitMapper<<Focus as Aspect<T::State>>::View, A, In>>::emit(view, input)
    }
}

/// Type-level callable adapter used by [`AbsorbFn`].
pub trait AbsorbMapper<View, A, Out>
where
    A: Action,
{
    fn absorb(view: &mut View, output: ActionCompletion<A>) -> Out;
}

/// Absorbs via a type-level mapper function.
pub struct AbsorbFn<Focus, A, Out, F>(PhantomData<fn() -> (Focus, A, Out, F)>);

impl<T, Focus, A, Out, F> Absorb<T> for AbsorbFn<Focus, A, Out, F>
where
    T: Animal,
    Focus: Aspect<T::State>,
    A: Action,
    F: AbsorbMapper<<Focus as Aspect<T::State>>::View, A, Out>,
{
    type CarryOut = Out;
    type Aspect = Focus;
    type Action = A;

    fn absorb(
        view: &mut <Self::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
        <F as AbsorbMapper<<Focus as Aspect<T::State>>::View, A, Out>>::absorb(view, output)
    }
}

/// Combines independent [`Emit`] and [`Absorb`] implementations into [`Act`].
pub struct Adapt<E, A>(PhantomData<fn() -> (E, A)>);

impl<T, E, A> Act<T> for Adapt<E, A>
where
    T: Animal,
    E: Emit<T>,
    A: Absorb<T, Action = <E as Emit<T>>::Action, Aspect = <E as Emit<T>>::Aspect>,
{
    type Action = <E as Emit<T>>::Action;
    type Aspect = <E as Emit<T>>::Aspect;
    type In = <E as Emit<T>>::CarryIn;
    type Out = <A as Absorb<T>>::CarryOut;

    fn emit(
        view: &<<Self as Act<T>>::Aspect as Aspect<T::State>>::View,
        input: Self::In,
    ) -> <Self::Action as Action>::In {
        <E as Emit<T>>::emit(view, input)
    }

    fn absorb(
        view: &mut <<Self as Act<T>>::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Out {
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
    type CarryIn = <E as Emit<T>>::CarryIn;
    type Aspect = Focus;
    type Action = <E as Emit<T>>::Action;

    fn emit(
        view: &<Self::Aspect as Aspect<T::State>>::View,
        input: Self::CarryIn,
    ) -> <Self::Action as Action>::In {
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
    type CarryOut = <A as Absorb<T>>::CarryOut;
    type Aspect = Focus;
    type Action = <A as Absorb<T>>::Action;

    fn absorb(
        view: &mut <Self::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
        <A as Absorb<T>>::absorb(view, output)
    }
}

/// Alias for an [`Adapt`] step focused by a specific [`Aspect`].
pub type FocusedStep<T, Focus, E, B> =
    Step<T, Adapt<FocusedEmit<Focus, E>, FocusedAbsorb<Focus, B>>>;

/// Identity-focused [`FocusedStep`].
pub type IdentityStep<T, E, B> = FocusedStep<T, Identity, E, B>;

/// A primitive workflow step that adapts an [`Action`] to the
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
    type In = (T::State, <A as Act<T>>::In);
    type Out = (T::State, ActionRequest<<A as Act<T>>::Action>);

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<A as Act<T>>::Aspect as Aspect<T::State>>::view(&mut state);
        let action_input = <A as Act<T>>::emit(view, input);
        (
            state,
            ActionRequest::<<A as Act<T>>::Action>::new(action_input),
        )
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, A> Waiting for Step<T, A>
where
    T: Animal,
    A: Act<T>,
{
    type In = (T::State, ActionCompletion<<A as Act<T>>::Action>);
    type Out = (T::State, <A as Act<T>>::Out);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let view = <<A as Act<T>>::Aspect as Aspect<T::State>>::view(&mut state);
        let emitted = <A as Act<T>>::absorb(view, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, A> FlowActions for Step<T, A>
where
    T: Animal,
    <A as Act<T>>::Action: ActionMember,
    A: Act<T>,
{
    type List = Node<<<A as Act<T>>::Action as Action>::Id, <A as Act<T>>::Action>;
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
