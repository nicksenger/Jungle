use jungle_macros::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Metabolism {
    pub energy: u64,
    pub is_hungry: bool,
    pub is_sleepy: bool,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct Base {
    pub species: String,
    pub age: u32,
    pub weight: f32,
    pub metabolism: Metabolism,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct Herbivore {
    pub base: Base,
    pub favorite_plant: String,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct Carnivore {
    pub base: Base,
    pub favorite_meat: String,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct Omnivore {
    pub base: Base,
    pub favorite_plant: String,
    pub favorite_meat: String,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Mammal {
    pub fur_color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalePattern {
    Cycloid,
    DiamondLattice,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Reptile {
    pub scale_pattern: ScalePattern,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Swimmer {
    pub has_fins: bool,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Climber {
    pub is_ape: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weapon {
    Teeth,
    Claws,
    Tools,
    Mass,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct Combatant {
    pub weapons: Weapon,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct LionState {
    pub carnivore: Carnivore,
    pub mammal: Mammal,
    pub combatant: Combatant,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct HippoState {
    pub herbivore: Herbivore,
    pub mammal: Mammal,
    pub swimmer: Swimmer,
    pub combatant: Combatant,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct RhinoState {
    pub herbivore: Herbivore,
    pub mammal: Mammal,
    pub combatant: Combatant,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct CrocodileState {
    pub carnivore: Carnivore,
    pub reptile: Reptile,
    pub swimmer: Swimmer,
    pub combatant: Combatant,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct GiraffeState {
    pub herbivore: Herbivore,
    pub mammal: Mammal,
    pub combatant: Combatant,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct GorillaState {
    pub omnivore: Omnivore,
    pub mammal: Mammal,
    pub climber: Climber,
    pub combatant: Combatant,
}
