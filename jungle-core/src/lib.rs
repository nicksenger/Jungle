mod meta;

pub trait Jungle {
    type Animals;
}

/// A worker carries out the will of the Jungle.
pub trait Worker<T: Jungle> {}
