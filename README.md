# Jungle

Welcome to the `jungle`.

`jungle` explores the idea of "Workflow-as-Type" (WaT).

Unlike "Worflow-as-Code" frameworks such as [Temporal](https://temporal.io/) or [Restate](https://www.restate.dev/), `jungle` expects `Flow`s to be expressed as a Rust type, or more specifically, a tree of Rust types. This prevents using many common programming patterns for the expression of control flow, but in turn enables: visualization of the execution graph, type-safe generic Flow-composition, compile-time node replacement / graph traversal, and other seemingly magical properties.

The [welcome](./jungle-examples/examples/welcome/) example implements playback of the Guns N' Roses' 1987 hit single "Welcome to the Jungle" using `jungle`:

https://github.com/user-attachments/assets/f5b2410b-f606-46b4-83d8-0dd34c06f7cb

These `Flow`s handle the notation and timing for the instruments and vocalizations internally, while treating PCM synthesis and playback (through [cpal](https://github.com/RustAudio/cpal)) as I/O. Inputs for each note are persisted either in memory, redb, or postgres, depending on the choice of backend, facilitating recovery in the event of an outage.

I want to emphasize that the goal of the `jungle` is not to compete with, refine, simplify, translate, or port any existing orchestration tools. For the time being, `jungle` is more of a living art project sprouted from the Rust programming language and its ecosystem.

The`jungle` is still growing, and many bugs live here! Feel free to look around, but don't let it bring you down.

