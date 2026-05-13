use jungle_sdk::types::Running;

type S = jungle_zoo::animals::gorilla::State;
type Feed = jungle_zoo::animals::gorilla::GorillaFeedFlow;

type Tool = jungle_zoo::animals::gorilla::GorillaToolSocialFlow;
type Simple = jungle_zoo::animals::gorilla::GorillaSimpleSocialFlow;
type Active = jungle_zoo::animals::gorilla::GorillaActiveFlow;

type DayCond = jungle_sdk::types::Conditional<jungle_zoo::animals::gorilla::GorillaIsActiveNow, Active, jungle_sdk::types::Step<jungle_zoo::animals::gorilla::Gorilla, jungle_zoo::animals::gorilla::GorillaRest>>;

fn probe_running<R>()
where
    R: Running<In = (S, ())>,
{
}

fn main() {
    probe_running::<Feed>();
    probe_running::<Tool>();
    probe_running::<Simple>();
    probe_running::<Active>();
    probe_running::<DayCond>();
}
