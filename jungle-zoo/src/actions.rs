//! Zoo actions organized by shared state categories.

mod support;

pub mod climber;
pub mod combatant;
pub mod diet;
pub mod mammal;
pub mod metabolism;
pub mod reptile;
pub mod species;
pub mod swimmer;

pub use climber::*;
pub use combatant::*;
pub use diet::*;
pub use mammal::*;
pub use metabolism::*;
pub use reptile::*;
pub use species::*;
pub use swimmer::*;
