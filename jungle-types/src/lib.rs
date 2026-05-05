mod behavior;
mod error;
mod instinct;
mod meta;
pub use behavior::Action;
pub use error::Error;
pub use instinct::Instinct;
use inception::*;
pub use meta::Id;
pub use meta::{
    ActionMember, ActionSet, AllFrom, AnimalActionSet, AnimalMember, AnimalSet, AnimalStates,
    AnimalStatesCompatible, StripActionHeaders, StripAnimalHeaders,
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
#[inception(property = JungleYielding, signature(input = In, output = Out))]
pub trait Yielding {
    /// Input used to start/resume this yielding phase.
    type In;

    /// A typed transition frame, typically
    /// `Yielded<Output, AwaitingTail<NextAwaitingState>>`.
    type Out;

    /// Run until this phase yields output and transitions to an awaiting phase.
    fn run(self, input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }

    fn merge<H, R>(l: H, r: R, input: Self::In) -> Yielded<<H as Yielding>::Out, AwaitingTail<R>>
    where
        H: Yielding<In = Self::In>,
    {
        Yielded {
            output: <H as Yielding>::run(l.access(), input),
            awaiting: AwaitingTail(r),
        }
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, input: Self::In) -> Self::In {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, input: Self::In) -> <F as Yielding>::Out
    where
        F: Yielding<In = Self::In>,
    {
        <F as Yielding>::run(fields, input)
    }
}

/// A phase that awaits an external input, then transitions back to a yielding
/// phase.
#[inception(property = JungleAwaiting, signature(input = In, output = Out))]
pub trait Awaiting {
    /// External input expected at this await point.
    type In;

    /// A typed transition frame, typically
    /// `Awaited<Output, YieldingTail<NextYieldingState>>`.
    type Out;

    /// Accept awaited input and transition to the next yielding phase.
    fn accept(self, input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }

    fn merge<H, R>(l: H, r: R, input: Self::In) -> Awaited<<H as Awaiting>::Out, YieldingTail<R>>
    where
        H: Awaiting<In = Self::In>,
    {
        Awaited {
            output: <H as Awaiting>::accept(l.access(), input),
            yielding: YieldingTail(r),
        }
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, input: Self::In) -> Self::In {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, input: Self::In) -> <F as Awaiting>::Out
    where
        F: Awaiting<In = Self::In>,
    {
        <F as Awaiting>::accept(fields, input)
    }
}

impl<T> Awaiting for AwaitingTail<T>
where
    T: Awaiting,
{
    type In = <T as Awaiting>::In;
    type Out = <T as Awaiting>::Out;

    fn accept(self, input: Self::In) -> Self::Out {
        <T as Awaiting>::accept(self.0, input)
    }
}

impl<T> Yielding for YieldingTail<T>
where
    T: Yielding,
{
    type In = <T as Yielding>::In;
    type Out = <T as Yielding>::Out;

    fn run(self, input: Self::In) -> Self::Out {
        <T as Yielding>::run(self.0, input)
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
