use crate::{
    Animal, Aspect, BoundAction, Effect, EffectCompletion, EffectSchema, Failure, Id, Identity,
    StateCarrier,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;
use typosaurus::num::consts::U65535;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepError {
    pub message: String,
}

pub struct Sleep;

impl<J> EffectSchema<J> for Sleep {
    type Id = Id<U65535>;
    type In = Duration;
    type Out = ();
    type Err = SleepError;
}

impl<J> Effect<J> for Sleep {
    #[allow(clippy::manual_async_fn)]
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            tokio::time::sleep(input).await;
            Ok(())
        }
    }
}

pub struct SleepStep<Focus = Identity>(PhantomData<fn() -> Focus>);

impl<T, Focus> BoundAction<T> for SleepStep<Focus>
where
    T: Animal,
    Focus: Aspect<T::State>,
{
    const NAME: &'static str = "SleepStep";
    type Effect = Sleep;
    type Aspect = Focus;
    type Input = Duration;
    type Output = ();
    type Carry = ();

    fn emit(
        _view: &<Focus as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn emit_with_carry(
        view: &<Focus as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
    }

    fn absorb(
        _view: &mut <Focus as StateCarrier<T::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))
    }
}

