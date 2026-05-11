fn main() {
    jungle_viewer::JungleViewerBuilder::new()
        .title("Jungle View Example")
        .view_animal::<jungle_examples::StaticAnimal>()
        .expect("jungle-view example should launch viewer");
}
