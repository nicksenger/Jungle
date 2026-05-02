/// A vessel is a container for an entity within the Jungle ecosystem.
pub trait Vessel {
    /// The kind of entity this vessel holds.
    type Entity;
}
