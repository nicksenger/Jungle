use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;

use crate::{ActionMember, Animal, Awaiting, FlowActions, Yielding};
use inception::primitive;
use typosaurus::collections::sp::Node;

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

/// Maps workflow input plus current animal state into an action input.
pub trait ActionInputMapper<T: Animal, A: Action> {
    type In;

    fn map_input(state: &T::State, input: Self::In) -> A::In;
}

/// Maps an action completion back into updated animal state plus emitted
/// workflow output.
pub trait ActionOutputMapper<T: Animal, A: Action> {
    type Out;

    fn map_output(state: &mut T::State, output: ActionCompletion<A>) -> Self::Out;
}

/// Unified mapper that adapts workflow input/state to action input and maps
/// action completion back into workflow output.
pub trait ActionMapper<T: Animal, A: Action> {
    type In;
    type Out;

    fn map_input(state: &T::State, input: Self::In) -> A::In;

    fn map_output(state: &mut T::State, output: ActionCompletion<A>) -> Self::Out;
}

/// Projects a larger state into a focused mutable substate.
pub trait Aspect<State> {
    type View;

    fn view(state: &State) -> &Self::View;

    fn view_mut(state: &mut State) -> &mut Self::View;
}

/// Maps action transitions against a focused state view.
pub trait FocusedActionMapper<View, A: Action> {
    type In;
    type Out;

    fn map_input(view: &View, input: Self::In) -> A::In;

    fn map_output(view: &mut View, output: ActionCompletion<A>) -> Self::Out;
}

/// Adapts a focused mapper through an [`Aspect`] so it can power a full-state
/// [`ActionMapper`].
pub struct AspectMapper<Focus, Mapper>(PhantomData<fn() -> (Focus, Mapper)>);

impl<T, A, Focus, Mapper> ActionMapper<T, A> for AspectMapper<Focus, Mapper>
where
    T: Animal,
    A: Action,
    Focus: Aspect<T::State>,
    Mapper: FocusedActionMapper<<Focus as Aspect<T::State>>::View, A>,
{
    type In = <Mapper as FocusedActionMapper<<Focus as Aspect<T::State>>::View, A>>::In;
    type Out = <Mapper as FocusedActionMapper<<Focus as Aspect<T::State>>::View, A>>::Out;

    fn map_input(state: &T::State, input: Self::In) -> A::In {
        let view = <Focus as Aspect<T::State>>::view(state);
        <Mapper as FocusedActionMapper<<Focus as Aspect<T::State>>::View, A>>::map_input(
            view, input,
        )
    }

    fn map_output(state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
        let view = <Focus as Aspect<T::State>>::view_mut(state);
        <Mapper as FocusedActionMapper<<Focus as Aspect<T::State>>::View, A>>::map_output(
            view, output,
        )
    }
}

pub struct MapperInput<M>(PhantomData<fn() -> M>);

impl<T, A, M> ActionInputMapper<T, A> for MapperInput<M>
where
    T: Animal,
    A: Action,
    M: ActionMapper<T, A>,
{
    type In = <M as ActionMapper<T, A>>::In;

    fn map_input(state: &T::State, input: Self::In) -> A::In {
        <M as ActionMapper<T, A>>::map_input(state, input)
    }
}

pub struct MapperOutput<M>(PhantomData<fn() -> M>);

impl<T, A, M> ActionOutputMapper<T, A> for MapperOutput<M>
where
    T: Animal,
    A: Action,
    M: ActionMapper<T, A>,
{
    type Out = <M as ActionMapper<T, A>>::Out;

    fn map_output(state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
        <M as ActionMapper<T, A>>::map_output(state, output)
    }
}

pub type ActionMapperStep<T, A, Mapper> =
    ActionStep<T, A, MapperInput<Mapper>, MapperOutput<Mapper>>;

/// Action step backed by a focused mapper over an [`Aspect`] of state.
pub type AspectActionStep<T, A, Focus, Mapper> =
    ActionMapperStep<T, A, AspectMapper<Focus, Mapper>>;

/// A primitive workflow step that adapts an [`Action`] to the
/// [`Yielding`]/[`Awaiting`] temporal protocol.
pub struct ActionStep<T, A, Prepare, Apply = Prepare>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    marker: PhantomData<fn() -> (T, A)>,
    mapper_marker: PhantomData<fn() -> (Prepare, Apply)>,
}

impl<T, A, Prepare, Apply> ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    pub fn new() -> Self {
        Self {
            marker: PhantomData,
            mapper_marker: PhantomData,
        }
    }
}

#[primitive(property = crate::JungleYielding)]
impl<T, A, Prepare, Apply> Yielding for ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    type In = (T::State, <Prepare as ActionInputMapper<T, A>>::In);
    type Out = (T::State, ActionRequest<A>);

    fn run((state, input): Self::In) -> Self::Out {
        let action_input = <Prepare as ActionInputMapper<T, A>>::map_input(&state, input);
        (state, ActionRequest::<A>::new(action_input))
    }
}

#[primitive(property = crate::JungleAwaiting)]
impl<T, A, Prepare, Apply> Awaiting for ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    type In = (T::State, ActionCompletion<A>);
    type Out = (T::State, <Apply as ActionOutputMapper<T, A>>::Out);

    fn accept((mut state, output): Self::In) -> Self::Out {
        let emitted = <Apply as ActionOutputMapper<T, A>>::map_output(&mut state, output);
        (state, emitted)
    }
}

#[primitive(property = crate::JungleFlowActions)]
impl<T, A, Prepare, Apply> FlowActions for ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action + ActionMember,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    type List = Node<<A as Action>::Id, A>;
}
