fn main() {
    let (client, journey_id) = jungle_examples::spawn_observe_runtime();

    jungle_viewer::JungleViewerBuilder::new()
        .title("Jungle Observe Example")
        .live_poll_interval(std::time::Duration::from_millis(750))
        .view_live_animal::<jungle_examples::ObserveAnimal, _>(client, journey_id)
        .expect("jungle-observe example should launch viewer");
}
