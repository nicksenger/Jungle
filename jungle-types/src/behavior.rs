use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Sub;

use crate::{
    ActionMember, Anima, FlowActions, ReplaceFlow, ReplaceStep, ReplaceWith, Running, TraverseFlow,
    TraverseStep, TraverseWith, Waiting,
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
/// of anima state.
pub trait Reflex<T: Anima> {
    type Action: Action;
    type Aspect: Aspect<T::State>;
    type In;
    type Out;

    fn prepare(
        view: &<<Self as Reflex<T>>::Aspect as Aspect<T::State>>::View,
        input: Self::In,
    ) -> <Self::Action as Action>::In;

    fn process(
        view: &mut <<Self as Reflex<T>>::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Out;
}

/// A primitive workflow step that adapts an [`Action`] to the
/// [`Running`]/[`Waiting`] protocol.
pub struct Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    marker: PhantomData<fn() -> (T, R)>,
}

impl<T, R> Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[primitive(property = crate::JungleRunning)]
impl<T, R> Running for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    type In = (T::State, <R as Reflex<T>>::In);
    type Out = (T::State, ActionRequest<<R as Reflex<T>>::Action>);

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<R as Reflex<T>>::Aspect as Aspect<T::State>>::view(&mut state);
        let action_input = <R as Reflex<T>>::prepare(view, input);
        (
            state,
            ActionRequest::<<R as Reflex<T>>::Action>::new(action_input),
        )
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, R> Waiting for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    type In = (T::State, ActionCompletion<<R as Reflex<T>>::Action>);
    type Out = (T::State, <R as Reflex<T>>::Out);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let view = <<R as Reflex<T>>::Aspect as Aspect<T::State>>::view(&mut state);
        let emitted = <R as Reflex<T>>::process(view, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, R> FlowActions for Impulse<T, R>
where
    T: Anima,
    <R as Reflex<T>>::Action: ActionMember,
    R: Reflex<T>,
{
    type List = Node<<<R as Reflex<T>>::Action as Action>::Id, <R as Reflex<T>>::Action>;
}

#[primitive(property = crate::JungleTraverseFlow)]
impl<T, R> TraverseFlow for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    type Output = Impulse<T, R>;
}

#[primitive(property = crate::JungleReplaceFlow)]
impl<T, R> ReplaceFlow for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
{
    type Output = Impulse<T, R>;
}

impl<T, R, Traversal> TraverseWith<Traversal> for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
    Traversal: TraverseStep<Impulse<T, R>>,
{
    type Output = <Traversal as TraverseStep<Impulse<T, R>>>::Output;
}

impl<T, R, Replacer> ReplaceWith<Replacer> for Impulse<T, R>
where
    T: Anima,
    R: Reflex<T>,
    Replacer: ReplaceStep<Impulse<T, R>>,
{
    type Output = <Replacer as ReplaceStep<Impulse<T, R>>>::Output;
}
