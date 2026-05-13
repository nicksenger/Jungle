fn main() {
    let by_id = std::env::args().any(|arg| arg == "--by-id");
    let (client, journey_id) = if by_id {
        jungle_examples::spawn_gorilla_runtime_by_id()
    } else {
        jungle_examples::spawn_gorilla_runtime_by_animal()
    };

    jungle_viewer::JungleViewerBuilder::new()
        .title("Jungle Observe Example")
        .live_poll_interval(std::time::Duration::from_millis(750))
        .view_live_animal::<jungle_zoo::animals::gorilla::Gorilla, _>(client, journey_id)
        .expect("jungle-observe example should launch viewer");
}

