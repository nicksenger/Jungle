use crate::{
    Act, Animal, Aspect, Effect, EffectCompletion, Id, Identity, StateCarrier,
};
use inception::primitive;
use std::marker::PhantomData;
use std::time::Duration;
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U65535;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SleepError {
    pub message: String,
}

pub struct Sleep;

impl<J> Effect<J> for Sleep {
    type Id = Id<U65535>;
    type In = Duration;
    type Out = ();
    type Err = SleepError;

    fn effect(_jungle: &J, input: Self::In) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
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

impl<T, Focus> Act<T> for SleepStep<Focus>
where
    T: Animal,
    Focus: Aspect<T::State>,
{
    type Effect = Sleep;
    type StateAspect = Focus;
    type Input = Duration;
    type Output = ();

    fn emit(
        _view: &<Focus as StateCarrier<T::State>>::View,
        input: Self::Input,
    ) -> <Self::Effect as Effect>::In {
        input
    }

    fn absorb(
        _view: &mut <Focus as StateCarrier<T::State>>::View,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("Sleep effect should be resumed by worker runtime");
    }
}
