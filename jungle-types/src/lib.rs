mod behavior;
mod error;
mod executor;
mod instinct;
mod meta;
mod transport;
pub use behavior::{
    Action, ActionCompletion, ActionRequest, Aspect, Identity, Impulse, Lens, Task,
};
pub use error::Error;
pub use executor::{
    BuildFlow, BuildFlowWithContext, ContextExecutor, ContextualTypedErasedStep, DynFlow,
    ErasedStep, ExecutableActionRequest, Executor, ExecutorError, ExecutorFlow, JungleDynFlow,
    ManualExecutor, TypedErasedStep,
};
use inception::*;
pub use instinct::Journey;
pub use meta::Id;
pub use meta::{
    ActionMember, ActionSet, AllFrom, AnimaActionDependencies,
    AnimaActionDependenciesCompatible, AnimaActionSet, AnimaMember, AnimaSet,
    AnimaStates, AnimaStatesCompatible, StripActionHeaders, StripAnimaHeaders,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
pub use transport::{BackendError, FlowStatus, RunnerOut, WireIn, WireOut, Work};
use typosaurus::collections::list::{self, List as TList};
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U0;

/// A tagged union over two possible outputs.
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

/// Predicate used by [`Conditional`] to choose a branch.
pub trait Condition<In> {
    fn choose(input: &In) -> bool;
}

/// Extracts state from flow inputs shaped as `(State, Input)`.
pub trait StatefulInput {
    type State;
    fn state_ref(&self) -> &Self::State;
}

impl<State, In> StatefulInput for (State, In) {
    type State = State;

    fn state_ref(&self) -> &Self::State {
        &self.0
    }
}

/// Predicate used by [`While`] to decide whether another iteration runs.
pub trait LoopCondition<State> {
    fn should_continue(state: &State) -> bool;
}

/// Property used to opt-in a state type to field-index lenses.
pub struct JungleOptic;
impl Property for JungleOptic {}

/// Marker trait proving a type has opted into [`JungleOptic`].
pub trait Optic: Inception<JungleOptic, False> {}
impl<T> Optic for T where T: Inception<JungleOptic, False> {}

/// A flow combinator that chooses either `L` or `R` at runtime.
pub struct Conditional<P, L, R>(PhantomData<fn() -> (P, L, R)>);

/// A flow combinator that repeatedly executes `F` while `C` is true.
pub struct While<C, F>(PhantomData<fn() -> (C, F)>);

/// A collection of `Animae` which act together as a system.
pub trait Ecosystem {
    type Animae;
}

/// A living anima within the Jungle ecosystem.
pub trait Anima {
    /// A type-level identifier for this Anima.
    type Id;

    /// The state of this `Anima` at any given time.
    type State;

    /// Serializable seed used to initialize this anima's state.
    type Seed: Serialize + DeserializeOwned + Into<Self::State>;

    /// The fundamental behavior of this Anima.
    type Journey;
}

#[inception(property = Ident, types)]
pub trait Identified {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        merge_variant = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        join = TList<(U0, <Fields as Identified>::Id)>
    )]
    type Id;
}

/// Any collection of [`Anima`]s with a flat type-level list of members.
#[inception(property = JungleAnimae, types)]
pub trait Animae {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Animae>::List, <Tail as Animae>::List)>,
        merge_variant = TList<(<Head as Animae>::List, <Tail as Animae>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Animae>::List)> where { Self: Identified }
    )]
    type List;
}

/// Any collection of [`Action`]s with a flat type-level list of members.
#[inception(property = JungleActions, types)]
pub trait Actions {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Actions>::List, <Tail as Actions>::List)>,
        merge_variant = TList<(<Head as Actions>::List, <Tail as Actions>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Actions>::List)> where { Self: Identified }
    )]
    type List;
}

/// A collection of [`Action`]s extractable from an executable workflow.
#[inception(property = JungleFlow, types)]
pub trait FlowActions {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as FlowActions>::List, <Tail as FlowActions>::List)>,
        merge_variant = TList<(<Head as FlowActions>::List, <Tail as FlowActions>::List)>,
        join = TList<(Node<U0, ()>, <Fields as FlowActions>::List)>
    )]
    type List;
}

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
impl<P, L, R> Running for Conditional<P, L, R>
where
    L: Running,
    R: Running<In = L::In>,
    P: Condition<L::In>,
{
    type In = L::In;
    type Out = Either<L::Out, R::Out>;

    fn run(input: Self::In) -> Self::Out {
        if <P as Condition<L::In>>::choose(&input) {
            Either::Left(<L as Running>::run(input))
        } else {
            Either::Right(<R as Running>::run(input))
        }
    }
}

#[primitive(property = JungleWaiting)]
impl<P, L, R> Waiting for Conditional<P, L, R>
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
impl<P, L, R> FlowActions for Conditional<P, L, R>
where
    L: FlowActions,
    R: FlowActions,
{
    type List = TList<(L::List, R::List)>;
}

#[primitive(property = JungleRunning)]
impl<C, F> Running for While<C, F>
where
    F: Running,
    F::In: StatefulInput,
    C: LoopCondition<<F::In as StatefulInput>::State>,
{
    type In = F::In;
    type Out = Option<F::Out>;

    fn run(input: Self::In) -> Self::Out {
        if <C as LoopCondition<<F::In as StatefulInput>::State>>::should_continue(input.state_ref())
        {
            Some(<F as Running>::run(input))
        } else {
            None
        }
    }
}

#[primitive(property = JungleWaiting)]
impl<C, F> Waiting for While<C, F>
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
impl<C, F> FlowActions for While<C, F>
where
    F: FlowActions,
{
    type List = F::List;
}

/// An organism that hosts symbionts.
pub trait Host {
    /// Organisms that live in close association with this Host.
    type Symbionts;
}

/// A trait that transforms a stream of inputs into a stream of outputs.
pub trait Evoke {
    /// The input type accepted by this evoke.
    type In;

    /// The output type produced by this evoke.
    type Out;

    /// Process a stream of inputs, yielding a stream of outputs.
    fn evoke(
        self,
        input: impl futures::Stream<Item = Self::In>,
    ) -> impl futures::Stream<Item = Self::Out>;
}
