mod behavior;
mod error;
mod instinct;
mod meta;
mod test_executor;
pub use behavior::{
    Action, ActionCompletion, ActionRequest, ActionStep, Aspect, AspectStep, Whole,
};
pub use error::Error;
use inception::*;
pub use instinct::Instinct;
pub use meta::Id;
pub use meta::{
    ActionMember, ActionSet, AllFrom, AnimalActionSet, AnimalMember, AnimalSet, AnimalStates,
    AnimalStatesCompatible, StripActionHeaders, StripAnimalHeaders,
};
pub use test_executor::{
    BuildTestFlow, DynFlow, ErasedStep, JungleTestFlow, TestExecutor, TestExecutorError, TestFlow,
    TypedErasedStep,
};
use typosaurus::collections::list::{self, List as TList};
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U0;

/// A collection of `Animals` which act together as a system.
pub trait Ecosystem {
    type Animals;
}

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// The state of this `Animal` at any given time.
    type State;

    /// The fundamental behavior of this Animal.
    type Instinct;
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

/// Any collection of [`Animal`]s with a flat type-level list of members.
#[inception(property = JungleAnimals, types)]
pub trait Animals {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        merge_variant = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Animals>::List)> where { Self: Identified }
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
