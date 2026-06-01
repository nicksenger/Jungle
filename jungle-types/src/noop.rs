use crate::{Effect, EffectSchema, Id};
use typosaurus::num::consts::U654;

/// Built-in no-op effect for `Action`s that don't need any I/O.
///
/// The executor has a dedicated inline fast path for this effect to avoid
/// request/completion roundtrip overhead.
pub struct Noop;

impl<J> EffectSchema<J> for Noop {
    type Id = Id<U654>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J> Effect<J> for Noop {
    #[allow(clippy::manual_async_fn)]
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(()) }
    }
}
