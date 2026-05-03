/// Behavior traits for Jungle entities.
///
/// These traits define the contract for actions an entity can take
/// and the instincts that drive those actions.

/// A trait for actions that define the input/output contract of an action.
pub trait Action {
    type Input: serde::Serialize + serde::de::DeserializeOwned;
    type Output: serde::Serialize + serde::de::DeserializeOwned;
}

/// A trait for instincts.
pub trait Instinct {
}
