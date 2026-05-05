/// The innate behavior of an `Animal`.
pub trait Instinct {
    /// The `Action`s taken by `Animal`s with this `Instinct`.
    type Actions;
}
