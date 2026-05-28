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

Each tier increases both the number of animals and per-animal journey length while forcing:

- `small`: 8 animals, 48-step journeys
- `medium`: 16 animals, new tier uses 80-step journeys
- `large`: 24 animals, new tier uses 112-step journeys
- `xlarge`: 32 animals, new tier uses 144-step journeys

- `AnimalSet<T::Animals>: SupportedAnimalGenerations<T>`
- `BoundAnimalJourney + BuildFlowWithContext + ArgputForState`

through a concrete `JungleWorker::new(CompileZoo, MockClient::default())` instantiation.
