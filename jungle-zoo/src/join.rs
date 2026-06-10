use jungle_sdk::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

pub struct Passthrough<St, In>(PhantomData<St>, PhantomData<In>);
#[jungle::action(carry = In)]
impl<St, In> Action for Passthrough<St, In>
where
    In: Serialize + DeserializeOwned + Send + 'static,
{
    type Effect = NoEffect;
    type Input = In;
    type Output = In;

    fn emit(_state: &St, input: Self::Input) -> ((), In) {
        ((), input)
    }

    fn absorb(
        _state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: In,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

#[derive(Flow)]
pub struct Pass<St, In: Serialize + DeserializeOwned + Send + 'static>(Step<Passthrough<St, In>>);

pub struct CloneJoinInput<In>(PhantomData<fn() -> In>);
pub struct CloneJoinInputAct<A, In>(PhantomData<fn() -> (A, In)>);
impl<A, In> BoundAction<A> for CloneJoinInputAct<A, In>
where
    A: Animal,
    In: Clone + DeserializeOwned + Send + Serialize + 'static,
{
    const NAME: &'static str = "CloneJoinInput";
    type Effect = NoEffect;
    type Aspect = Identity;
    type Input = In;
    type Output = (In, In);
    type Carry = (In, In);

    fn emit(
        _view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        _input: Self::Input,
    ) {
    }

    fn absorb(
        _view: &mut <<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        panic!("CloneJoinInput uses carry; absorb_with_carry should be called")
    }

    fn emit_with_carry(
        _view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), (input.clone(), input))
    }

    fn absorb_with_carry(
        _view: &mut <<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("clone join input should complete without effect"))?;
        Ok(carry)
    }
}

#[jungle::action(bind = CloneJoinInputAct<A, In>)]
impl<In> Action for CloneJoinInput<In> {
    type Effect = NoEffect;
    type Input = In;
    type Output = (In, In);
    type Carry = (In, In);
}

/// Adapts a shared cloneable input into the tuple input required by [`Join`].
#[derive(Flow)]
pub struct ClonedJoin<In, L, R>(Step<CloneJoinInput<In>>, Join<L, R>);
pub type ClonedJoinUnit<L, R> = ClonedJoin<(), L, R>;

/// Adapts a shared cloneable input into the tuple input required by [`Select`].
#[derive(Flow)]
pub struct ClonedSelect<In, L, R>(Step<CloneJoinInput<In>>, Select<L, R>);
pub type ClonedSelectUnit<L, R> = ClonedSelect<(), L, R>;
