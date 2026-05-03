use serde::de::DeserializeOwned;
use serde::Serialize;

/// The innate behavior definition for an Animal.
pub trait Instinct {
    /// The actions available to this instinct.
    type Actions;
}

/// A behavior that transforms a single input into a single output.
pub trait Action {
    /// The input type accepted by this action.
    type In: Serialize + DeserializeOwned;

    /// The output type produced by this action.
    type Out: Serialize + DeserializeOwned;

    /// The error type produced by this action.
    type Err;

    /// Process one input into one output.
    async fn act(input: Self::In) -> Result<Self::Out, Self::Err>;
}
