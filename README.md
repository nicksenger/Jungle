# Jungle

Welcome to the `jungle`, we've got fun and games (unmute to hear the `jungle` `Animal`s):

https://github.com/user-attachments/assets/97682c49-485b-4b6f-bff4-586c51c8b5dc

In the video, all notes are persisted to postgres, and all audio, including formant speech synthesis, is generated on-the-fly. This showcases some of the things that are possible in the `jungle`, as codified in the [welcome](./jungle-examples/examples/welcome/) example.

But that's not really the important part, because MIDI playback and 80s speech-synthesis most likely won't impress anyone in 2026.

At its core, `jungle` is just another boring and opinionated orchestration framework like airflow, temporal, DBOS, cadence, restate, or whatever else it is you use at work for this sort of thing.

The key differences are:

1. If you use `jungle` in production, at least for the time being, _you're gonne die_
2. In the `jungle`, "Workflows" are just called `Flow`s, because `Animal`s don't care about work
3. `Flow`s are defined using Rust ***types***, because `Animal`s cannot be reliably controlled at runtime

So instead of Workflow-as-Code (WaaC), you can think of the `jungle` as Workflow-as-Type (WaaaT).

In the `jungle`, separation of a system's control flow and its effects is strictly enforced at compile time by the laws of nature. This is achieved through (ab)use of Rust's trait solver and type-level programming methods.

