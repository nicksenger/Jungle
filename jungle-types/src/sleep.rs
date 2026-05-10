use crate::{Act, Action, ActionCompletion, ActionMember, Animal, Aspect, Id, Identity};
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
impl ActionMember for Sleep {}

#[derive(Debug, Clone, Copy, Default)]
pub struct SleepDependency;

impl<T> From<&T> for SleepDependency {
    fn from(_value: &T) -> Self {
        Self
    }
}

impl Action for Sleep {
    type Id = Id<U65535>;
    type Dependency = SleepDependency;
    type In = Duration;
    type Out = ();
    type Err = SleepError;

    fn act(
        _dependency: &SleepDependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Err(SleepError {
            message: "Sleep action must be intercepted by JungleWorker runtime".to_string(),
        }))
    }
}

#[primitive(property = crate::JungleActions)]
impl crate::Actions for Sleep {
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
    type Action = Sleep;
    type Aspect = Focus;
    type In = Duration;
    type Out = ();

    fn emit(
        _view: &<Focus as Aspect<T::State>>::View,
        input: Self::In,
    ) -> <Self::Action as Action>::In {
        input
    }

    fn absorb(
        _view: &mut <Focus as Aspect<T::State>>::View,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Out {
        output.expect("Sleep action should be resumed by worker runtime");
    }
}
