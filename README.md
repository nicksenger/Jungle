# Jungle

Welcome to the `jungle`.

`jungle` explores the idea of "Workflow-as-Type" (WaT).

It has many similarities to event-replay driven "Worflow-as-Code" frameworks such as [Temporal](https://temporal.io/) or [Restate](https://www.restate.dev/), with the main difference being that in `jungle`, `Flow`s are expressed as type-level trees.

This limits the ways you can write `Flow`s, but allows for their traversal and inspection at compile-time which in turn enables:
- visualization of the execution graph
- type-safe generic `Flow` composition
- compile-time node/branch replacement & manipulation
- moving various runtime errors to compile-time
- ... other magical properties ...

The [welcome](./jungle-examples/examples/welcome/) example implements playback of the Guns N' Roses 1987 hit single "Welcome to the Jungle" using `jungle`:

https://github.com/user-attachments/assets/f5b2410b-f606-46b4-83d8-0dd34c06f7cb

In the example, `Flow`s handle the notation and timing for the instruments and vocalizations, while PCM synthesis and playback (through [cpal](https://github.com/RustAudio/cpal)) is performed by `Effect`s. Inputs for each note are persisted either through Fjall or PostgreSQL, depending on choice of backend, which allows the show to go on in the event of an outage.

The `jungle` is in a primitive state. The terrain is unforgiving and there are a lot of bugs. Have a look around, but don't let it bring you down!

