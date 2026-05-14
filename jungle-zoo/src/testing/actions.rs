//! Compact actions for deterministic progression and async flow tests.

use jungle_types::{Action, ActionMember};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterDependency {
    pub add_one: i32,
    pub add_two: i32,
}

impl Default for CounterDependency {
    fn default() -> Self {
        Self {
            add_one: 1,
            add_two: 2,
        }
    }
}

impl<T> From<&T> for CounterDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

pub struct AddOne;
impl ActionMember for AddOne {}
impl Action for AddOne {
    type Id = u16;
    type Dependency = CounterDependency;
    type In = ();
    type Out = i32;
    type Err = String;

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.add_one))
    }
}

pub struct AddTwo;
impl ActionMember for AddTwo {}
impl Action for AddTwo {
    type Id = u16;
    type Dependency = CounterDependency;
    type In = ();
    type Out = i32;
    type Err = String;

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.add_two))
    }
}

pub struct TimedValue;
impl ActionMember for TimedValue {}
impl Action for TimedValue {
    type Id = u16;
    type Dependency = ();
    type In = (u64, i32);
    type Out = i32;
    type Err = String;

    fn act(
        _dependency: &Self::Dependency,
        (sleep_ms, value): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
            Ok(value)
        }
    }
}
