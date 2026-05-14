use jungle_sdk::types::{Running, Step};

type G = jungle_zoo::animals::gorilla::Gorilla;
type S = jungle_zoo::animals::gorilla::State;

type Birth = Step<G, jungle_zoo::animals::gorilla::GorillaBirth>;
type Peel = Step<G, jungle_zoo::animals::gorilla::GorillaPeelFruit>;
type Eat = Step<G, jungle_zoo::animals::gorilla::GorillaEat>;

type Feed = jungle_zoo::animals::gorilla::GorillaFeedFlow;

fn probe<R>()
where
    R: Running<In = (S, ())>,
{
}

fn main() {
    probe::<Birth>();
    probe::<Peel>();
    probe::<Eat>();
    probe::<Feed>();
}
