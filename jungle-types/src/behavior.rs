use serde::de::DeserializeOwned;
use serde::Serialize;

/// The innate behavior definition for an Animal.
pub trait Instinct {
    /// The actions available to this instinct.
    type Actions;
}

/// A behavior that transforms a single input into a single output.
pub trait Action {
    /// A type-level identifier for this Action.
    type Id;

    /// The shared state consumed by this action.
    type State;

    /// The input type accepted by this action.
    type In: Serialize + DeserializeOwned;

    /// The output type produced by this action.
    type Out: Serialize + DeserializeOwned;

    /// The error type produced by this action.
    type Err;

    /// Process one input into one output.
    async fn act(state: &Self::State, input: Self::In) -> Result<Self::Out, Self::Err>;
}
