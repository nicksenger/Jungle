use jungle_sdk::prelude::*;

pub struct Always<St, In>(St, In);
impl<St, In> Predicate<(&St, &In)> for Always<St, In> {
    fn eval((_state, _input): &(&St, &In)) -> bool {
        true
    }
}
