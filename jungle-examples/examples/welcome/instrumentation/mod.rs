mod bass;
mod electric_guitar;
mod vocals;

#[allow(unused)]
pub use bass::Thump;
#[allow(unused_imports)]
pub use electric_guitar::{Pick, Pluck, Strum};
#[allow(unused)]
pub use vocals::Generate;

#[allow(unused_imports)]
pub use welcome_audio::instrumentation::{
    phonemes_from_text, Bass, BassArticulation, Cymbal, CymbalArticulation, ElectricGuitar,
    ElectricGuitarArticulation, Error, HiHat, HiHatArticulation, Instrument, KickDrum,
    KickDrumArticulation, Lyrics, Note, SnareDrum, SnareDrumArticulation, SynthHandle, Toms,
    TomsArticulation, Vocals, VocalsArticulation,
};
