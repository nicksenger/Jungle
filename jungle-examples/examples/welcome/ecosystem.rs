use jungle_sdk::prelude::*;

use crate::{
    animals::WelcomeAnimals,
    instrumentation::{
        Bass, Cymbal, ElectricGuitar, HiHat, KickDrum, SnareDrum, SynthHandle, Toms, Vocals,
    },
    metronome::Metronome,
};
use welcome_audio::AudioHandle;

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
    animal_volumes: AnimalVolumes,
}

#[derive(Debug, Clone, Copy)]
pub struct AnimalVolumes {
    lead_vocalist: f32,
    lead_guitarist: f32,
    rhythm_guitarist: f32,
    bassist: f32,
    drummer: f32,
}

impl Default for AnimalVolumes {
    fn default() -> Self {
        Self {
            lead_vocalist: 0.5,
            lead_guitarist: 0.5,
            rhythm_guitarist: 0.5,
            bassist: 0.5,
            drummer: 0.5,
        }
    }
}

impl AnimalVolumes {
    const LEAD_VOCALIST_LANE_ID: u32 = 0;
    const LEAD_GUITARIST_LANE_ID: u32 = 1;
    const RHYTHM_GUITARIST_LANE_ID: u32 = 2;
    const BASSIST_LANE_ID: u32 = 3;
    const DRUMMER_LANE_ID: u32 = 4;

    pub fn with_lead_vocalist(mut self, volume: f32) -> Self {
        self.lead_vocalist = volume;
        self
    }

    pub fn with_lead_guitarist(mut self, volume: f32) -> Self {
        self.lead_guitarist = volume;
        self
    }

    pub fn with_rhythm_guitarist(mut self, volume: f32) -> Self {
        self.rhythm_guitarist = volume;
        self
    }

    pub fn with_bassist(mut self, volume: f32) -> Self {
        self.bassist = volume;
        self
    }

    pub fn with_drummer(mut self, volume: f32) -> Self {
        self.drummer = volume;
        self
    }

    pub fn for_lane(self, lane_id: u32) -> f32 {
        match lane_id {
            Self::LEAD_VOCALIST_LANE_ID => self.lead_vocalist,
            Self::LEAD_GUITARIST_LANE_ID => self.lead_guitarist,
            Self::RHYTHM_GUITARIST_LANE_ID => self.rhythm_guitarist,
            Self::BASSIST_LANE_ID => self.bassist,
            Self::DRUMMER_LANE_ID => self.drummer,
            _ => 0.5,
        }
    }
}

impl TheJungle {
    pub fn new(audio_handle: AudioHandle, bpm: f32) -> Self {
        let synth_handle = SynthHandle::new();
        let metronome = Metronome::spawn(bpm);
        Self::new_with_metronome_and_synth_and_volumes(
            audio_handle,
            synth_handle,
            bpm,
            metronome,
            AnimalVolumes::default(),
        )
    }

    pub fn new_with_metronome(audio_handle: AudioHandle, bpm: f32, metronome: Metronome) -> Self {
        let synth_handle = SynthHandle::new();
        Self::new_with_metronome_and_synth_and_volumes(
            audio_handle,
            synth_handle,
            bpm,
            metronome,
            AnimalVolumes::default(),
        )
    }

    pub fn new_with_metronome_and_synth(
        audio_handle: AudioHandle,
        synth_handle: SynthHandle,
        bpm: f32,
        metronome: Metronome,
    ) -> Self {
        Self::new_with_metronome_and_synth_and_volumes(
            audio_handle,
            synth_handle,
            bpm,
            metronome,
            AnimalVolumes::default(),
        )
    }

    pub fn new_with_metronome_and_synth_and_volumes(
        audio_handle: AudioHandle,
        synth_handle: SynthHandle,
        bpm: f32,
        metronome: Metronome,
        animal_volumes: AnimalVolumes,
    ) -> Self {
        Self {
            rhythm_guitar: ElectricGuitar::new(audio_handle.clone(), synth_handle.clone()),
            bass: Bass::new(audio_handle.clone(), synth_handle.clone()),
            vocals: Vocals::new(audio_handle.clone(), synth_handle.clone()),
            hihat: HiHat::new(audio_handle.clone(), synth_handle.clone()),
            kick_drum: KickDrum::new(audio_handle.clone(), synth_handle.clone()),
            snare_drum: SnareDrum::new(audio_handle.clone(), synth_handle.clone()),
            toms: Toms::new(audio_handle.clone(), synth_handle.clone()),
            cymbal: Cymbal::new(audio_handle, synth_handle),
            bpm,
            metronome,
            animal_volumes,
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

    pub fn animal_volume_for_lane(&self, lane_id: u32) -> f32 {
        self.animal_volumes.for_lane(lane_id)
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
