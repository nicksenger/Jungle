use crate::{
    Animal, Aspect, BoundAct, EffectCompletion, EffectExec, EffectSchema, Id, Identity,
    StateCarrier,
};
use inception::primitive;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U65535;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepError {
    pub message: String,
}

pub struct Sleep;

impl EffectSchema for Sleep {
    type Id = Id<U65535>;
    type In = Duration;
    type Out = ();
    type Err = SleepError;
}

impl<J> EffectExec<J> for Sleep {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(input);
            Ok(())
        }
    }
}

#[primitive(property = crate::JungleEffects)]
impl crate::Effects for Sleep {
    type List = Node<U65535, Sleep>;
}

#[primitive(property = crate::Ident)]
impl crate::Identified for Sleep {
    type Id = U65535;
}

pub struct SleepStep<Focus = Identity>(PhantomData<fn() -> Focus>);

impl<T, Focus> BoundAct<T> for SleepStep<Focus>
where
    T: Animal,
    Focus: Aspect<T::State>,
{
    type Effect = Sleep;
    type Aspect = Focus;
    type Input = Duration;
    type Output = ();

    fn emit(
        _view: &<Focus as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        _view: &mut <Focus as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("Sleep effect should be resumed by worker runtime");
    }
}
