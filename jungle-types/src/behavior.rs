use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;

use crate::{Animal, Awaiting, Yielding};
use inception::primitive;

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

    fn map_input(&self, state: &T::State, input: Self::In) -> A::In;
}

/// Maps an action completion back into updated animal state plus emitted
/// workflow output.
pub trait ActionOutputMapper<T: Animal, A: Action> {
    type Out;

    fn map_output(&self, state: &mut T::State, output: ActionCompletion<A>) -> Self::Out;
}

/// A primitive workflow step that adapts an [`Action`] to the
/// [`Yielding`]/[`Awaiting`] temporal protocol.
pub struct ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    prepare: Prepare,
    apply: Apply,
    marker: PhantomData<fn() -> (T, A)>,
}

impl<T, A, Prepare, Apply> ActionStep<T, A, Prepare, Apply>
where
    T: Animal,
    A: Action,
    Prepare: ActionInputMapper<T, A>,
    Apply: ActionOutputMapper<T, A>,
{
    pub fn new(prepare: Prepare, apply: Apply) -> Self {
        Self {
            prepare,
            apply,
            marker: PhantomData,
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

    fn run(self, (state, input): Self::In) -> Self::Out {
        let action_input = self.prepare.map_input(&state, input);
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

    fn accept(self, (mut state, output): Self::In) -> Self::Out {
        let emitted = self.apply.map_output(&mut state, output);
        (state, emitted)
    }
}
