use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Sub;

use crate::{ActionMember, Creature, FlowActions, Running, Waiting};
use inception::{primitive, Access, Field, Inception as InceptionTy, VariantHeader};
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

impl<State, Index> Aspect<State> for Lens<State, Index>
where
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
    type View = <<State as InceptionTy<crate::JungleOptic, inception::False>>::TyFields as FieldContentAt<Index>>::Content;

    fn view(state: &mut State) -> &mut Self::View {
        let mut header = VariantHeader;
        let fields =
            <State as InceptionTy<crate::JungleOptic, inception::False>>::fields_mut(state, &mut header);
        fields.at_mut()
    }
}

/// Single step-facing contract for adapting an [`Action`] over an [`Aspect`]
/// of creature state.
pub trait AspectStep<T: Creature, A: Action> {
    type Aspect: Aspect<T::State>;
    type In;
    type Out;

    fn prepare(
        view: &<<Self as AspectStep<T, A>>::Aspect as Aspect<T::State>>::View,
        input: Self::In,
    ) -> A::In;

    fn apply(
        view: &mut <<Self as AspectStep<T, A>>::Aspect as Aspect<T::State>>::View,
        output: ActionCompletion<A>,
    ) -> Self::Out;
}

/// A primitive workflow step that adapts an [`Action`] to the
/// [`Running`]/[`Waiting`] temporal protocol.
pub struct ActionStep<T, A, Step>
where
    T: Creature,
    A: Action,
    Step: AspectStep<T, A>,
{
    marker: PhantomData<fn() -> (T, A, Step)>,
}

impl<T, A, Step> ActionStep<T, A, Step>
where
    T: Creature,
    A: Action,
    Step: AspectStep<T, A>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[primitive(property = crate::JungleRunning)]
impl<T, A, Step> Running for ActionStep<T, A, Step>
where
    T: Creature,
    A: Action,
    Step: AspectStep<T, A>,
{
    type In = (T::State, <Step as AspectStep<T, A>>::In);
    type Out = (T::State, ActionRequest<A>);

    fn run((mut state, input): Self::In) -> Self::Out {
        let view = <<Step as AspectStep<T, A>>::Aspect as Aspect<T::State>>::view(&mut state);
        let action_input = <Step as AspectStep<T, A>>::prepare(view, input);
        (state, ActionRequest::<A>::new(action_input))
    }
}

#[primitive(property = crate::JungleWaiting)]
impl<T, A, Step> Waiting for ActionStep<T, A, Step>
where
    T: Creature,
    A: Action,
    Step: AspectStep<T, A>,
{
    type In = (T::State, ActionCompletion<A>);
    type Out = (T::State, <Step as AspectStep<T, A>>::Out);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let view = <<Step as AspectStep<T, A>>::Aspect as Aspect<T::State>>::view(&mut state);
        let emitted = <Step as AspectStep<T, A>>::apply(view, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlow)]
impl<T, A, Step> FlowActions for ActionStep<T, A, Step>
where
    T: Creature,
    A: Action + ActionMember,
    Step: AspectStep<T, A>,
{
    type List = Node<<A as Action>::Id, A>;
}
