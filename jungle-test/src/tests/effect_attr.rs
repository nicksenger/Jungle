use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::collections::sp::Node;
use num::U90;

struct AutoPrimitiveEffect;

#[jungle::effect(id = 90)]
impl<J> Effect<J> for AutoPrimitiveEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}

fn assert_schema<T: EffectSchema>() {}
fn assert_exec<T: Effect<()>>() {}
fn assert_effects<T: Effects>() {}
fn assert_identified<T: EffectIdentified>() {}

#[test]
fn effect_attr_emits_schema_exec_and_primitives() {
    assert_schema::<AutoPrimitiveEffect>();
    assert_exec::<AutoPrimitiveEffect>();
    assert_effects::<AutoPrimitiveEffect>();
    assert_identified::<AutoPrimitiveEffect>();

    assert_type_eq!(<AutoPrimitiveEffect as EffectSchema>::Id, Id<U90>);
    assert_type_eq!(<AutoPrimitiveEffect as EffectIdentified>::Id, U90);
    assert_type_eq!(<AutoPrimitiveEffect as Effects>::List, Node<U90, AutoPrimitiveEffect>);
}
