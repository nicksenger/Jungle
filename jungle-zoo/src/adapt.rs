//! Action adapters that connect structural state to action inputs/outputs.

use crate::actions;
use crate::state::{
    ActivitySchedule, AgeState, Ears, FruitMeal, Hands, Horns, LeafCrop, PerceivedTimeOfDay,
    PreyCut, Scales, Skeleton, TimePerception, Torso, VitalReadings,
};
use jungle_types::{
    AbsorbFn, AbsorbMapper, Action, ActionCompletion, Animal, EmitFn, EmitMapper, Fuse, Identity,
    Step,
};

pub struct EmitVitalEnergy;
impl<A> EmitMapper<VitalReadings, A, ()> for EmitVitalEnergy
where
    A: Action<In = u16>,
{
    fn emit(view: &VitalReadings, _input: ()) -> A::In {
        view.energy
    }
}

pub struct AbsorbVitalEnergy;
impl<A> AbsorbMapper<VitalReadings, A, ()> for AbsorbVitalEnergy
where
    A: Action<Out = u16>,
{
    fn absorb(view: &mut VitalReadings, output: ActionCompletion<A>) {
        let energy = match output {
            Ok(value) => value,
            Err(_) => return,
        };
        view.energy = energy;
        view.is_hungry = energy < 30;
        view.is_sleepy = energy < 25;
    }
}

pub type VitalEnergyStep<T, A> = Step<
    T,
    Fuse<EmitFn<Identity, A, (), EmitVitalEnergy>, AbsorbFn<Identity, A, (), AbsorbVitalEnergy>>,
>;

pub struct EmitVitalStress;
impl<A> EmitMapper<VitalReadings, A, ()> for EmitVitalStress
where
    A: Action<In = u8>,
{
    fn emit(view: &VitalReadings, _input: ()) -> A::In {
        view.stress
    }
}

pub struct AbsorbVitalStress;
impl<A> AbsorbMapper<VitalReadings, A, ()> for AbsorbVitalStress
where
    A: Action<Out = u8>,
{
    fn absorb(view: &mut VitalReadings, output: ActionCompletion<A>) {
        if let Ok(stress) = output {
            view.stress = stress;
        }
    }
}

pub type VitalStressStep<T, A> = Step<
    T,
    Fuse<EmitFn<Identity, A, (), EmitVitalStress>, AbsorbFn<Identity, A, (), AbsorbVitalStress>>,
>;

pub struct EmitRelax;
impl EmitMapper<VitalReadings, actions::Relax, ()> for EmitRelax {
    fn emit(view: &VitalReadings, _input: ()) -> <actions::Relax as Action>::In {
        (view.energy, view.stress)
    }
}

pub struct AbsorbRelax;
impl AbsorbMapper<VitalReadings, actions::Relax, ()> for AbsorbRelax {
    fn absorb(view: &mut VitalReadings, output: ActionCompletion<actions::Relax>) {
        if let Ok((energy, stress)) = output {
            view.energy = energy;
            view.stress = stress;
            view.is_sleepy = energy < 25;
        }
    }
}

pub type RelaxStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Relax, (), EmitRelax>,
        AbsorbFn<Identity, actions::Relax, (), AbsorbRelax>,
    >,
>;

pub struct EmitMakeSound;
impl EmitMapper<VitalReadings, actions::MakeSound, String> for EmitMakeSound {
    fn emit(view: &VitalReadings, kind: String) -> <actions::MakeSound as Action>::In {
        (kind, view.stress)
    }
}

pub struct AbsorbMakeSound;
impl AbsorbMapper<VitalReadings, actions::MakeSound, String> for AbsorbMakeSound {
    fn absorb(_view: &mut VitalReadings, output: ActionCompletion<actions::MakeSound>) -> String {
        output.expect("make sound should succeed")
    }
}

pub type MakeSoundStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::MakeSound, String, EmitMakeSound>,
        AbsorbFn<Identity, actions::MakeSound, String, AbsorbMakeSound>,
    >,
>;

pub struct EmitUseTool;
impl EmitMapper<Hands, actions::UseTool, ()> for EmitUseTool {
    fn emit(view: &Hands, _input: ()) -> <actions::UseTool as Action>::In {
        view.left.opposable_thumb && view.right.opposable_thumb
    }
}

pub struct AbsorbUseTool;
impl AbsorbMapper<Hands, actions::UseTool, String> for AbsorbUseTool {
    fn absorb(_view: &mut Hands, output: ActionCompletion<actions::UseTool>) -> String {
        output.expect("tool-use should succeed")
    }
}

pub type UseToolStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::UseTool, (), EmitUseTool>,
        AbsorbFn<Identity, actions::UseTool, String, AbsorbUseTool>,
    >,
>;

pub struct EmitStripLeaves;
impl EmitMapper<LeafCrop, actions::StripLeaves, ()> for EmitStripLeaves {
    fn emit(view: &LeafCrop, _input: ()) -> <actions::StripLeaves as Action>::In {
        (view.fiber.toughness, view.mass_g)
    }
}

pub struct AbsorbStripLeaves;
impl AbsorbMapper<LeafCrop, actions::StripLeaves, u16> for AbsorbStripLeaves {
    fn absorb(view: &mut LeafCrop, output: ActionCompletion<actions::StripLeaves>) -> u16 {
        let edible = output.expect("strip-leaves should succeed");
        view.mass_g = edible;
        edible
    }
}

pub type StripLeavesStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::StripLeaves, (), EmitStripLeaves>,
        AbsorbFn<Identity, actions::StripLeaves, u16, AbsorbStripLeaves>,
    >,
>;

pub struct EmitPeelFruit;
impl EmitMapper<FruitMeal, actions::PeelFruit, ()> for EmitPeelFruit {
    fn emit(view: &FruitMeal, _input: ()) -> <actions::PeelFruit as Action>::In {
        (view.rind.thickness_mm, view.flesh.mass_g)
    }
}

pub struct AbsorbPeelFruit;
impl AbsorbMapper<FruitMeal, actions::PeelFruit, u16> for AbsorbPeelFruit {
    fn absorb(view: &mut FruitMeal, output: ActionCompletion<actions::PeelFruit>) -> u16 {
        let edible = output.expect("peel-fruit should succeed");
        view.flesh.mass_g = edible;
        edible
    }
}

pub type PeelFruitStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::PeelFruit, (), EmitPeelFruit>,
        AbsorbFn<Identity, actions::PeelFruit, u16, AbsorbPeelFruit>,
    >,
>;

pub struct EmitCrackShell;
impl EmitMapper<PreyCut, actions::CrackShell, u8> for EmitCrackShell {
    fn emit(view: &PreyCut, bite_strength: u8) -> <actions::CrackShell as Action>::In {
        (view.has_shell, bite_strength)
    }
}

pub struct AbsorbCrackShell;
impl AbsorbMapper<PreyCut, actions::CrackShell, bool> for AbsorbCrackShell {
    fn absorb(view: &mut PreyCut, output: ActionCompletion<actions::CrackShell>) -> bool {
        let cracked = output.expect("crack-shell should succeed");
        if cracked {
            view.has_shell = false;
        }
        cracked
    }
}

pub type CrackShellStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::CrackShell, u8, EmitCrackShell>,
        AbsorbFn<Identity, actions::CrackShell, bool, AbsorbCrackShell>,
    >,
>;

pub struct EmitTearMeat;
impl EmitMapper<PreyCut, actions::TearMeat, ()> for EmitTearMeat {
    fn emit(view: &PreyCut, _input: ()) -> <actions::TearMeat as Action>::In {
        (view.muscle.mass_g, view.hide.thickness_mm)
    }
}

pub struct AbsorbTearMeat;
impl AbsorbMapper<PreyCut, actions::TearMeat, u16> for AbsorbTearMeat {
    fn absorb(view: &mut PreyCut, output: ActionCompletion<actions::TearMeat>) -> u16 {
        let exposed = output.expect("tear-meat should succeed");
        view.muscle.mass_g = exposed;
        exposed
    }
}

pub type TearMeatStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::TearMeat, (), EmitTearMeat>,
        AbsorbFn<Identity, actions::TearMeat, u16, AbsorbTearMeat>,
    >,
>;

pub struct EmitSocialize;
impl EmitMapper<Ears, actions::Socialize, u8> for EmitSocialize {
    fn emit(view: &Ears, stress: u8) -> <actions::Socialize as Action>::In {
        (stress, view.can_rotate)
    }
}

pub struct AbsorbSocialize;
impl AbsorbMapper<Ears, actions::Socialize, String> for AbsorbSocialize {
    fn absorb(_view: &mut Ears, output: ActionCompletion<actions::Socialize>) -> String {
        output.expect("socialize should succeed")
    }
}

pub type SocializeStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Socialize, u8, EmitSocialize>,
        AbsorbFn<Identity, actions::Socialize, String, AbsorbSocialize>,
    >,
>;

pub struct EmitSwim;
impl EmitMapper<Torso, actions::Swim, ()> for EmitSwim {
    fn emit(view: &Torso, _input: ()) -> <actions::Swim as Action>::In {
        view.chest_cavity.lung_capacity_liters
    }
}

pub struct AbsorbSwim;
impl AbsorbMapper<Torso, actions::Swim, u16> for AbsorbSwim {
    fn absorb(view: &mut Torso, output: ActionCompletion<actions::Swim>) -> u16 {
        let swim_score = output.expect("swim should succeed");
        view.chest_cavity.lung_capacity_liters = swim_score;
        swim_score
    }
}

pub type SwimStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Swim, (), EmitSwim>,
        AbsorbFn<Identity, actions::Swim, u16, AbsorbSwim>,
    >,
>;

pub struct EmitWalk;
impl EmitMapper<Skeleton, actions::Walk, ()> for EmitWalk {
    fn emit(view: &Skeleton, _input: ()) -> <actions::Walk as Action>::In {
        (view.forelimb.upper.length_cm, view.hindlimb.upper.length_cm)
    }
}

pub struct AbsorbWalk;
impl AbsorbMapper<Skeleton, actions::Walk, u16> for AbsorbWalk {
    fn absorb(_view: &mut Skeleton, output: ActionCompletion<actions::Walk>) -> u16 {
        output.expect("walk should succeed")
    }
}

pub type WalkStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Walk, (), EmitWalk>,
        AbsorbFn<Identity, actions::Walk, u16, AbsorbWalk>,
    >,
>;

pub struct EmitRun;
impl EmitMapper<Torso, actions::Run, u8> for EmitRun {
    fn emit(view: &Torso, stress: u8) -> <actions::Run as Action>::In {
        (view.chest_cavity.lung_capacity_liters, stress)
    }
}

pub struct AbsorbRun;
impl AbsorbMapper<Torso, actions::Run, u16> for AbsorbRun {
    fn absorb(_view: &mut Torso, output: ActionCompletion<actions::Run>) -> u16 {
        output.expect("run should succeed")
    }
}

pub type RunStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Run, u8, EmitRun>,
        AbsorbFn<Identity, actions::Run, u16, AbsorbRun>,
    >,
>;

pub struct EmitCharge;
impl EmitMapper<Horns, actions::Charge, u8> for EmitCharge {
    fn emit(view: &Horns, stress: u8) -> <actions::Charge as Action>::In {
        (view.max_length_cm, stress)
    }
}

pub struct AbsorbCharge;
impl AbsorbMapper<Horns, actions::Charge, u16> for AbsorbCharge {
    fn absorb(_view: &mut Horns, output: ActionCompletion<actions::Charge>) -> u16 {
        output.expect("charge should succeed")
    }
}

pub type ChargeStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::Charge, u8, EmitCharge>,
        AbsorbFn<Identity, actions::Charge, u16, AbsorbCharge>,
    >,
>;

pub struct EmitLayEggs;
impl EmitMapper<Scales, actions::LayEggs, ()> for EmitLayEggs {
    fn emit(view: &Scales, _input: ()) -> <actions::LayEggs as Action>::In {
        view.has_osteoderms
    }
}

pub struct AbsorbLayEggs;
impl AbsorbMapper<Scales, actions::LayEggs, u8> for AbsorbLayEggs {
    fn absorb(_view: &mut Scales, output: ActionCompletion<actions::LayEggs>) -> u8 {
        output.expect("lay-eggs should succeed")
    }
}

pub type LayEggsStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::LayEggs, (), EmitLayEggs>,
        AbsorbFn<Identity, actions::LayEggs, u8, AbsorbLayEggs>,
    >,
>;

pub struct EmitDeathRoll;
impl EmitMapper<Skeleton, actions::CrocodileDeathRoll, u8> for EmitDeathRoll {
    fn emit(view: &Skeleton, stress: u8) -> <actions::CrocodileDeathRoll as Action>::In {
        (view.has_tail, stress)
    }
}

pub struct AbsorbCrocDeathRoll;
impl AbsorbMapper<Skeleton, actions::CrocodileDeathRoll, String> for AbsorbCrocDeathRoll {
    fn absorb(
        _view: &mut Skeleton,
        output: ActionCompletion<actions::CrocodileDeathRoll>,
    ) -> String {
        output.expect("croc death roll should succeed")
    }
}

pub type CrocodileDeathRollStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::CrocodileDeathRoll, u8, EmitDeathRoll>,
        AbsorbFn<Identity, actions::CrocodileDeathRoll, String, AbsorbCrocDeathRoll>,
    >,
>;

pub struct EmitRoar;
impl EmitMapper<Torso, actions::LionRoar, u8> for EmitRoar {
    fn emit(view: &Torso, stress: u8) -> <actions::LionRoar as Action>::In {
        (view.chest_cavity.lung_capacity_liters, stress)
    }
}

pub struct AbsorbLionRoar;
impl AbsorbMapper<Torso, actions::LionRoar, String> for AbsorbLionRoar {
    fn absorb(_view: &mut Torso, output: ActionCompletion<actions::LionRoar>) -> String {
        output.expect("lion roar should succeed")
    }
}

pub type LionRoarStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::LionRoar, u8, EmitRoar>,
        AbsorbFn<Identity, actions::LionRoar, String, AbsorbLionRoar>,
    >,
>;

pub struct EmitAdvanceAge;
impl EmitMapper<AgeState, actions::AdvanceAge, ()> for EmitAdvanceAge {
    fn emit(view: &AgeState, _input: ()) -> <actions::AdvanceAge as Action>::In {
        view.age_years
    }
}

pub struct AbsorbAdvanceAge;
impl AbsorbMapper<AgeState, actions::AdvanceAge, ()> for AbsorbAdvanceAge {
    fn absorb(view: &mut AgeState, output: ActionCompletion<actions::AdvanceAge>) {
        if let Ok(next) = output {
            *view = next;
        }
    }
}

pub type AdvanceAgeStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::AdvanceAge, (), EmitAdvanceAge>,
        AbsorbFn<Identity, actions::AdvanceAge, (), AbsorbAdvanceAge>,
    >,
>;

pub struct EmitTickPerceivedTime;
impl EmitMapper<TimePerception, actions::TickPerceivedTime, ()> for EmitTickPerceivedTime {
    fn emit(view: &TimePerception, _input: ()) -> <actions::TickPerceivedTime as Action>::In {
        (view.current, view.minutes_since_transition)
    }
}

pub struct AbsorbTickPerceivedTime;
impl AbsorbMapper<TimePerception, actions::TickPerceivedTime, u16> for AbsorbTickPerceivedTime {
    fn absorb(
        view: &mut TimePerception,
        output: ActionCompletion<actions::TickPerceivedTime>,
    ) -> u16 {
        let next = output.expect("tick-perceived-time should succeed");
        *view = next;
        view.minutes_since_transition
    }
}

pub type TickPerceivedTimeStep<T> = Step<
    T,
    Fuse<
        EmitFn<Identity, actions::TickPerceivedTime, (), EmitTickPerceivedTime>,
        AbsorbFn<Identity, actions::TickPerceivedTime, u16, AbsorbTickPerceivedTime>,
    >,
>;

pub struct EmitEvaluateActivityWindow;
impl EmitMapper<ActivitySchedule, actions::EvaluateActivityWindow, PerceivedTimeOfDay>
    for EmitEvaluateActivityWindow
{
    fn emit(
        view: &ActivitySchedule,
        input: PerceivedTimeOfDay,
    ) -> <actions::EvaluateActivityWindow as Action>::In {
        (view.activity, input)
    }
}

pub struct AbsorbEvaluateActivityWindow;
impl AbsorbMapper<ActivitySchedule, actions::EvaluateActivityWindow, bool>
    for AbsorbEvaluateActivityWindow
{
    fn absorb(
        _view: &mut ActivitySchedule,
        output: ActionCompletion<actions::EvaluateActivityWindow>,
    ) -> bool {
        output.expect("evaluate-activity-window should succeed")
    }
}

pub type EvaluateActivityWindowStep<T> = Step<
    T,
    Fuse<
        EmitFn<
            Identity,
            actions::EvaluateActivityWindow,
            PerceivedTimeOfDay,
            EmitEvaluateActivityWindow,
        >,
        AbsorbFn<Identity, actions::EvaluateActivityWindow, bool, AbsorbEvaluateActivityWindow>,
    >,
>;

pub trait VitalsAnimal: Animal<State = VitalReadings> {}
impl<T> VitalsAnimal for T where T: Animal<State = VitalReadings> {}

pub trait HandsAnimal: Animal<State = Hands> {}
impl<T> HandsAnimal for T where T: Animal<State = Hands> {}

pub trait EarsAnimal: Animal<State = Ears> {}
impl<T> EarsAnimal for T where T: Animal<State = Ears> {}

pub trait TorsoAnimal: Animal<State = Torso> {}
impl<T> TorsoAnimal for T where T: Animal<State = Torso> {}

pub trait ScalesAnimal: Animal<State = Scales> {}
impl<T> ScalesAnimal for T where T: Animal<State = Scales> {}

pub trait SkeletonAnimal: Animal<State = Skeleton> {}
impl<T> SkeletonAnimal for T where T: Animal<State = Skeleton> {}

pub trait HornsAnimal: Animal<State = Horns> {}
impl<T> HornsAnimal for T where T: Animal<State = Horns> {}

pub trait LeafCropAnimal: Animal<State = LeafCrop> {}
impl<T> LeafCropAnimal for T where T: Animal<State = LeafCrop> {}

pub trait FruitMealAnimal: Animal<State = FruitMeal> {}
impl<T> FruitMealAnimal for T where T: Animal<State = FruitMeal> {}

pub trait PreyCutAnimal: Animal<State = PreyCut> {}
impl<T> PreyCutAnimal for T where T: Animal<State = PreyCut> {}

pub trait AgeAnimal: Animal<State = AgeState> {}
impl<T> AgeAnimal for T where T: Animal<State = AgeState> {}

pub trait TimePerceptionAnimal: Animal<State = TimePerception> {}
impl<T> TimePerceptionAnimal for T where T: Animal<State = TimePerception> {}

pub trait ActivityScheduleAnimal: Animal<State = ActivitySchedule> {}
impl<T> ActivityScheduleAnimal for T where T: Animal<State = ActivitySchedule> {}
