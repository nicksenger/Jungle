use jungle_sdk::prelude::*;

use crate::{
    animals::WelcomeAnimals, audio::AudioHandle, instrumentation::ElectricGuitar,
    metronome::Metronome,
};

pub struct WelcomeEcosystem {
    rhythm_guitar: ElectricGuitar,
    bpm: f32,
    metronome: Metronome,
}

impl WelcomeEcosystem {
    pub fn new(audio_handle: AudioHandle, bpm: f32) -> Self {
        Self {
            rhythm_guitar: ElectricGuitar::new(audio_handle),
            bpm,
            metronome: Metronome::spawn(bpm),
        }
    }

    pub fn rhythm_guitar(&self) -> &ElectricGuitar {
        &self.rhythm_guitar
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    pub fn metronome(&self) -> &Metronome {
        &self.metronome
    }
}

impl Ecosystem for WelcomeEcosystem {
    const NAME: &'static str = "welcome";
    type Animals = WelcomeAnimals;
}
