use super::support::define_action;
use crate::state::Mammal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MammalDependency {
    pub use_chest_beat: bool,
}

impl Default for MammalDependency {
    fn default() -> Self {
        Self {
            use_chest_beat: true,
        }
    }
}

impl<T> From<&T> for MammalDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    MakeSound,
    id = 30,
    dependency = MammalDependency,
    in = Mammal,
    out = String,
    err = String,
    act = |_dependency, mammal| {
        let _ = mammal;
        std::future::ready(Ok("made a mammal call".to_owned()))
    }
);

define_action!(
    ChestBeat,
    id = 31,
    dependency = MammalDependency,
    in = Mammal,
    out = String,
    err = String,
    act = |dependency, mammal| {
        let _ = mammal;
        if dependency.use_chest_beat {
            std::future::ready(Ok("performed chest beat display".to_owned()))
        } else {
            std::future::ready(Err("chest beat disabled by dependency".to_owned()))
        }
    }
);

define_action!(
    Roar,
    id = 32,
    dependency = MammalDependency,
    in = Mammal,
    out = String,
    err = String,
    act = |_dependency, mammal| {
        let _ = mammal;
        std::future::ready(Ok("released a territorial roar".to_owned()))
    }
);
