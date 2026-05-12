//! Zoo actions organized by structural state features.

mod support;

pub mod behavioral;
pub mod digestive;
pub mod locomotion;
pub mod reproduction;
pub mod species;
pub mod temporal;

pub use behavioral::*;
pub use digestive::*;
pub use locomotion::*;
pub use reproduction::*;
pub use species::*;
pub use temporal::*;
