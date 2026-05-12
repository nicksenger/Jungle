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
pub struct NervousSystem {
    pub cranial_nerves: u8,
    pub reflex_latency_ms: u16,
    pub vitals: VitalReadings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DigestiveSystem {
    pub stomach_count: u8,
    pub intestine_length_cm: u32,
    pub has_fermentation_chamber: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skeleton {
    pub vertebrae: u16,
    pub rib_pairs: u8,
    pub limb_count: u8,
    pub has_tail: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Torso {
    pub length_cm: u16,
    pub girth_cm: u16,
    pub lung_capacity_liters: u16,
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
pub struct Scales {
    pub rows: u16,
    pub hardness: u8,
    pub has_osteoderms: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tongue {
    pub length_cm: u16,
    pub prehensile: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hands {
    pub hand_count: u8,
    pub digits_per_hand: u8,
    pub opposable_thumb: bool,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LionState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub ears: Ears,
    pub dermis: Dermis,
    pub tongue: Tongue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElephantState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub ears: Ears,
    pub dermis: Dermis,
    pub trunk: Trunk,
    pub tusks: Tusks,
    pub tongue: Tongue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhinoState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub ears: Ears,
    pub dermis: Dermis,
    pub horns: Horns,
    pub tongue: Tongue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrocodileState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub scales: Scales,
    pub tongue: Tongue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GiraffeState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub ears: Ears,
    pub dermis: Dermis,
    pub tongue: Tongue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GorillaState {
    pub core: AnimalCore,
    pub nervous_system: NervousSystem,
    pub digestive_system: DigestiveSystem,
    pub skeleton: Skeleton,
    pub torso: Torso,
    pub ears: Ears,
    pub dermis: Dermis,
    pub hands: Hands,
    pub tongue: Tongue,
}
