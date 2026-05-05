use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;

use crate::{Awaiting, Yielding};
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

/// Maps workflow input plus current dependency into an action input.
pub trait ActionInputMapper<A: Action> {
    type In;

    fn map_input(&self, dependency: &A::Dependency, input: Self::In) -> A::In;
}

/// Maps an action completion back into dependency plus emitted workflow output.
pub trait ActionOutputMapper<A: Action> {
    type Out;

    fn map_output(&self, dependency: &mut A::Dependency, output: ActionCompletion<A>) -> Self::Out;
}

/// A primitive workflow step that adapts an [`Action`] to the
/// [`Yielding`]/[`Awaiting`] temporal protocol.
pub struct ActionStep<A, Prepare, Apply>
where
    A: Action,
    Prepare: ActionInputMapper<A>,
    Apply: ActionOutputMapper<A>,
{
    prepare: Prepare,
    apply: Apply,
    marker: PhantomData<fn() -> A>,
}

impl<A, Prepare, Apply> ActionStep<A, Prepare, Apply>
where
    A: Action,
    Prepare: ActionInputMapper<A>,
    Apply: ActionOutputMapper<A>,
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
impl<A, Prepare, Apply> Yielding for ActionStep<A, Prepare, Apply>
where
    A: Action,
    Prepare: ActionInputMapper<A>,
    Apply: ActionOutputMapper<A>,
{
    type In = (A::Dependency, <Prepare as ActionInputMapper<A>>::In);
    type Out = (A::Dependency, ActionRequest<A>);

    fn run(self, (dependency, input): Self::In) -> Self::Out {
        let action_input = self.prepare.map_input(&dependency, input);
        (dependency, ActionRequest::<A>::new(action_input))
    }
}

#[primitive(property = crate::JungleAwaiting)]
impl<A, Prepare, Apply> Awaiting for ActionStep<A, Prepare, Apply>
where
    A: Action,
    Prepare: ActionInputMapper<A>,
    Apply: ActionOutputMapper<A>,
{
    type In = (A::Dependency, ActionCompletion<A>);
    type Out = (A::Dependency, <Apply as ActionOutputMapper<A>>::Out);

    fn accept(self, (mut dependency, output): Self::In) -> Self::Out {
        let emitted = self.apply.map_output(&mut dependency, output);
        (dependency, emitted)
    }
}
