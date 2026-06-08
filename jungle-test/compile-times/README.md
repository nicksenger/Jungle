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

Each tier keeps a single animal and doubles journey length from a 24-step baseline:

- `small`: 1 animal, 24-step journey
- `medium`: 1 animal, 48-step journey
- `large`: 1 animal, 96-step journey
- `xlarge`: 1 animal, 192-step journey

- `AnimalSet<T::Animals>: SupportedAnimalGenerations<T>`
- `BoundAnimalJourney + BuildFlowWithContext + ArgputForState`

through a concrete `JungleWorker::new(CompileZoo, MockClient::default())` instantiation.

