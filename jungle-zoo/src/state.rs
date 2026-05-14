//! Structural state definitions required by gorilla lifecycle and probe flows.

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VitalReadings {
    pub energy: u16,
    pub is_hungry: bool,
    pub is_sleepy: bool,
    pub stress: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NeuronDensity {
    pub neurons_per_mm3: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lobe {
    pub name: String,
    pub density: NeuronDensity,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cortex {
    pub frontal: Lobe,
    pub temporal: Lobe,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Brain {
    pub cortex: Cortex,
    pub mass_g: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NervousSystem {
    pub cranial_nerves: u8,
    pub reflex_latency_ms: u16,
    pub vitals: VitalReadings,
    pub brain: Brain,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FruitRind {
    pub thickness_mm: u8,
    pub fibrous: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FruitFlesh {
    pub sugar_brix: u8,
    pub mass_g: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FruitMeal {
    pub name: String,
    pub rind: FruitRind,
    pub flesh: FruitFlesh,
    pub has_hard_seed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Nail {
    pub length_mm: u8,
    pub curved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Finger {
    pub nail: Nail,
    pub length_mm: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FingerSet {
    pub thumb: Finger,
    pub index: Finger,
    pub middle: Finger,
    pub ring: Finger,
    pub little: Finger,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hand {
    pub fingers: FingerSet,
    pub opposable_thumb: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hands {
    pub left: Hand,
    pub right: Hand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LifePhase {
    Child,
    Adolescent,
    Adult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PerceivedTimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DailyActivity {
    Nocturnal,
    Diurnal,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgeState {
    pub age_years: u8,
    pub life_phase: LifePhase,
    pub growth_percent: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CircadianWindow {
    pub start: PerceivedTimeOfDay,
    pub end: PerceivedTimeOfDay,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivitySchedule {
    pub activity: DailyActivity,
    pub active_window: CircadianWindow,
    pub rest_window: CircadianWindow,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimePerception {
    pub current: PerceivedTimeOfDay,
    pub minutes_since_transition: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TemporalState {
    pub age: AgeState,
    pub schedule: ActivitySchedule,
    pub perception: TimePerception,
}
