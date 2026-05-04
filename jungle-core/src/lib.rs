mod meta;

pub trait Jungle {}

/// A worker carries out the will of the Jungle.
pub trait Worker<T: Jungle> {}
