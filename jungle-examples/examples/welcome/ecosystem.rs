use jungle_sdk::prelude::*;

use crate::{
    animals::WelcomeAnimals,
    audio::AudioHandle,
    instrumentation::{Bass, Cymbal, ElectricGuitar, HiHat, KickDrum, SnareDrum, Toms, Vocals},
    metronome::Metronome,
};

pub struct TheJungle {
    rhythm_guitar: ElectricGuitar,
    bass: Bass,
    vocals: Vocals,
    hihat: HiHat,
    kick_drum: KickDrum,
    snare_drum: SnareDrum,
    toms: Toms,
    cymbal: Cymbal,
    bpm: f32,
    metronome: Metronome,
}

impl TheJungle {
    pub fn new(audio_handle: AudioHandle, bpm: f32) -> Self {
        Self {
            rhythm_guitar: ElectricGuitar::new(audio_handle.clone()),
            bass: Bass::new(audio_handle.clone()),
            vocals: Vocals::new(audio_handle.clone()),
            hihat: HiHat::new(audio_handle.clone()),
            kick_drum: KickDrum::new(audio_handle.clone()),
            snare_drum: SnareDrum::new(audio_handle.clone()),
            toms: Toms::new(audio_handle.clone()),
            cymbal: Cymbal::new(audio_handle),
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

    pub fn bass(&self) -> &Bass {
        &self.bass
    }

    pub fn vocals(&self) -> &Vocals {
        &self.vocals
    }

    pub fn hihat(&self) -> &HiHat {
        &self.hihat
    }

    pub fn kick_drum(&self) -> &KickDrum {
        &self.kick_drum
    }

    pub fn snare_drum(&self) -> &SnareDrum {
        &self.snare_drum
    }

    pub fn cymbal(&self) -> &Cymbal {
        &self.cymbal
    }

    pub fn toms(&self) -> &Toms {
        &self.toms
    }

    pub fn metronome(&self) -> &Metronome {
        &self.metronome
    }
}

impl Ecosystem for TheJungle {
    const NAME: &'static str = "welcome";
    type Animals = WelcomeAnimals;
}

impl<'a> From<&'a TheJungle> for &'a ElectricGuitar {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.rhythm_guitar()
    }
}

impl<'a> From<&'a TheJungle> for &'a Bass {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.bass()
    }
}

impl<'a> From<&'a TheJungle> for &'a Vocals {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.vocals()
    }
}

impl<'a> From<&'a TheJungle> for &'a HiHat {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.hihat()
    }
}

impl<'a> From<&'a TheJungle> for &'a KickDrum {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.kick_drum()
    }
}

impl<'a> From<&'a TheJungle> for &'a SnareDrum {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.snare_drum()
    }
}

impl<'a> From<&'a TheJungle> for &'a Cymbal {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.cymbal()
    }
}

impl<'a> From<&'a TheJungle> for &'a Toms {
    fn from(ecosystem: &'a TheJungle) -> Self {
        ecosystem.toms()
    }
}
