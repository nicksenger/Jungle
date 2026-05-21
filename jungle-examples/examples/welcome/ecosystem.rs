use jungle_sdk::prelude::*;

use crate::{
    animals::{
        Bass as BassAnimal, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist, WelcomeAnimals,
    },
    audio::{AudioEngine, AudioHandle},
    instrumentation::{
        Bass, BassArticulation, Cymbal, CymbalArticulation, ElectricGuitar,
        ElectricGuitarArticulation, Error as InstrumentError, HiHat, HiHatArticulation, Instrument,
        KickDrum, KickDrumArticulation, Note, SnareDrum, SnareDrumArticulation, Toms,
        TomsArticulation, Vocals, VocalsArticulation,
    },
    metronome::{Metronome, MetronomeSync},
    score::ScheduledNote,
    PlaybackClock,
};

pub struct WelcomeEcosystem {
    rhythm_guitar: ElectricGuitar,
    bpm: f32,
    playback_clock: PlaybackClock,
}

impl WelcomeEcosystem {
    pub fn new(audio_handle: AudioHandle, bpm: f32, playback_clock: PlaybackClock) -> Self {
        Self {
            rhythm_guitar: ElectricGuitar::new(audio_handle),
            bpm,
            playback_clock,
        }
    }

    pub fn rhythm_guitar(&self) -> &ElectricGuitar {
        &self.rhythm_guitar
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    pub fn playback_clock(&self) -> &PlaybackClock {
        &self.playback_clock
    }
}

impl Ecosystem for WelcomeEcosystem {
    const NAME: &'static str = "welcome";
    type Animals = WelcomeAnimals;
}
