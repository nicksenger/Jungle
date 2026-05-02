/// Behavior traits for Jungle entities.
///
/// These traits define the contract for actions an entity can take
/// and the impulses that drive those actions.

/// Niche for an ecological role
pub trait Niche {}

/// A trait for types that define the input/output contract of an action.
pub trait Impulse {
    type Input: serde::Serialize + serde::de::DeserializeOwned;
    type Output: serde::Serialize + serde::de::DeserializeOwned;
}

/// A trait for actions, which are a kind of impulse scoped to a niche.
pub trait Action: Impulse {
    type Niche: Niche;
}
