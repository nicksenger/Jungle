//! Action adapters that connect structural state to action inputs/outputs.

use crate::actions;
use crate::state::{Hands, Scales, Skeleton, Torso, VitalReadings};
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

pub trait VitalsAnimal: Animal<State = VitalReadings> {}
impl<T> VitalsAnimal for T where T: Animal<State = VitalReadings> {}

pub trait HandsAnimal: Animal<State = Hands> {}
impl<T> HandsAnimal for T where T: Animal<State = Hands> {}

pub trait TorsoAnimal: Animal<State = Torso> {}
impl<T> TorsoAnimal for T where T: Animal<State = Torso> {}

pub trait ScalesAnimal: Animal<State = Scales> {}
impl<T> ScalesAnimal for T where T: Animal<State = Scales> {}

pub trait SkeletonAnimal: Animal<State = Skeleton> {}
impl<T> SkeletonAnimal for T where T: Animal<State = Skeleton> {}
