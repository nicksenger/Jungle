# Jungle

Welcome to the `jungle`, we've got fun and games (unmute to hear the `jungle` `Animal`s):

https://github.com/user-attachments/assets/97682c49-485b-4b6f-bff4-586c51c8b5dc

The video is of the [welcome](./jungle-examples/examples/welcome/) example, which showcases some of the things possible in the `jungle` by representing Guns N' Roses' 1987 hit single "Welcome to the Jungle" as a `jungle` `Flow` performed by `jungle` `Animal`s, and visualized using `jungle-vision`.

The run shown uses the postgres persistence layer in combination with a fused (single-process) `JungleClient` + `JungleServer`, and 3 `JungleWorker`s. 

At its core, `jungle` is just another boring and opinionated distributed orchestration framework like airflow, temporal, DBOS, cadence, restate, etc.

The key differences are:

1. If you use `jungle` in production, at least for the time being, _you're gonna die_. It's extremely unstable and experimental on many levels.
2. In the `jungle`, "Workflows" are just called `Flow`s, because `Animal`s don't care about work.
3. The control-`Flow` of an `Animal`'s journey through the `Jungle` is expressed entirely through Rust ***types***. 

Instead of Workflow-as-Code (WaC), you can think of the `jungle` as Workflow-as-Type (WaT).

