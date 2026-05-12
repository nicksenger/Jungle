//! Structural state definitions for zoo animals.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimalCore {
    pub species: String,
    pub age_years: u8,
    pub mass_kg: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VitalReadings {
    pub energy: u16,
    pub is_hungry: bool,
    pub is_sleepy: bool,
    pub stress: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeuronDensity {
    pub neurons_per_mm3: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lobe {
    pub name: String,
    pub density: NeuronDensity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cortex {
    pub frontal: Lobe,
    pub temporal: Lobe,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brain {
    pub cortex: Cortex,
    pub mass_g: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NervousSystem {
    pub cranial_nerves: u8,
    pub reflex_latency_ms: u16,
    pub vitals: VitalReadings,
    pub brain: Brain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellLayer {
    pub thickness_microns: u16,
    pub regeneration_rate: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mucosa {
    pub epithelium: CellLayer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StomachLining {
    pub mucosa: Mucosa,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stomach {
    pub chamber_count: u8,
    pub lining: StomachLining,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntestineSegment {
    pub name: String,
    pub length_cm: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intestines {
    pub small: IntestineSegment,
    pub large: IntestineSegment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DigestiveSystem {
    pub stomach: Stomach,
    pub intestines: Intestines,
    pub has_fermentation_chamber: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoneDensity {
    pub grams_per_cm3: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimbBone {
    pub name: String,
    pub length_cm: u16,
    pub density: BoneDensity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limb {
    pub upper: LimbBone,
    pub lower: LimbBone,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skeleton {
    pub vertebrae: u16,
    pub rib_pairs: u8,
    pub forelimb: Limb,
    pub hindlimb: Limb,
    pub has_tail: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChestCavity {
    pub lung_capacity_liters: u16,
    pub heart_volume_ml: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Torso {
    pub length_cm: u16,
    pub girth_cm: u16,
    pub chest_cavity: ChestCavity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ears {
    pub count: u8,
    pub span_cm: u16,
    pub can_rotate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dermis {
    pub thickness_mm: u8,
    pub melanin: u8,
    pub waterproof: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleBed {
    pub blood_supply_rating: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scale {
    pub width_mm: u8,
    pub bed: ScaleBed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scales {
    pub dorsal: Scale,
    pub ventral: Scale,
    pub has_osteoderms: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MuscleGroup {
    pub name: String,
    pub strength_rating: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tongue {
    pub length_cm: u16,
    pub prehensile: bool,
    pub muscle_group: MuscleGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nail {
    pub length_mm: u8,
    pub curved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finger {
    pub nail: Nail,
    pub length_mm: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerSet {
    pub thumb: Finger,
    pub index: Finger,
    pub middle: Finger,
    pub ring: Finger,
    pub little: Finger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hand {
    pub fingers: FingerSet,
    pub opposable_thumb: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hands {
    pub left: Hand,
    pub right: Hand,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trunk {
    pub length_cm: u16,
    pub diameter_cm: u8,
    pub finger_count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tusks {
    pub count: u8,
    pub max_length_cm: u16,
    pub is_ivory: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Horns {
    pub count: u8,
    pub max_length_cm: u16,
    pub keratinized: bool,
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
