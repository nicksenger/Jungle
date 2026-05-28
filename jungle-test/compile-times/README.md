# compile-times

Opt-in compile-time benchmark harness for the `JungleWorker::new(...)` type-check path.

This crate is intentionally excluded from the root workspace so it does not affect normal `cargo test`, examples, or regular project crate builds.

## Run

From repository root:

```bash
cargo check --manifest-path jungle-test/compile-times/Cargo.toml --features small --timings
cargo check --manifest-path jungle-test/compile-times/Cargo.toml --features medium --timings
cargo check --manifest-path jungle-test/compile-times/Cargo.toml --features large --timings
cargo check --manifest-path jungle-test/compile-times/Cargo.toml --features xlarge --timings
```

Each tier increases the number of animals and distinct journey types while forcing:

- `AnimalSet<T::Animals>: SupportedAnimalGenerations<T>`
- `BoundAnimalJourney + BuildFlowWithContext + ArgputForState`

through a concrete `JungleWorker::new(CompileZoo, MockClient::default())` instantiation.
