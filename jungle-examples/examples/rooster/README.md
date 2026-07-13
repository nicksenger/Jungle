# Rooster

This is an example of a flow which prompts an LLM and parses tool-calls from its response in a continuous loop.

If you like buzzwords or are trying to sell something, you might consider it an _"observable, scalable, type-safe, deterministic harness for durable & fault-tolerant agents of the artificially intelligent kind,"_ but I prefer to keep things simple and just call it a rooster.

It's a CLI with 2 commands:
1. `roost`: this runs a jungle-server with the selected persistence, and does nothing special or specific to the example
2. `spawn`: this starts a jungle-worker and spawns 2 animal journeys to the provided backend:
    - the `Rooster`: when perturbed with a prompt, the `Rooster` forwards that prompt to an LLM (with tool descriptions, accumulated history, etc) over the given OpenAI compatible API and parses these tool calls from the response:
        1. `Cluck`: prints that a rooster clucked and, if the audio feature is enabled, plays a short waveform through cpal
        2. `Cockadoodledoo`: prints that a rooster cockadoodledood and, if audio is enabled, plays a longer cock-a-doodle-doo waveform through cpal
    - the `Trigger`: this just alternates sleeping for the provided duration (e.g. 1s, 5m, 1h, etc), and perturbing the associated `Rooster` with the following prompt:

> You are an experienced rooster.
> Evaluate the situation carefully, then use the 'Cockadoodledoo' and/or 'Cluck' tools to make sounds and assert your dominance.
> ONLY use the tools if it is safe to do so, and NEVER allow yourself to be 'snuffed.'

Since there is no way to terminate the rooster flows from the CLI, the number of active roosters associated with a given roost can be expected to increase over time even if the workers are killed.  

## Run a Roost

```bash
cargo run --release --example rooster --features fjall -- roost --fjall-path ~/.rooster/db
```

## Spawn a Rooster

```bash
cargo run --release --example rooster --features viewer -- spawn --roost-addr [::1]:4433 --openai-api-base-url http://localhost:8080 --circadian-interval 20s
```

Add `audio` to the feature list to enable live `cpal` sound playback for the `Cluck` and `Cockadoodledoo` tools:

```bash
cargo run --release --example rooster --features "viewer audio" -- spawn --roost-addr [::1]:4433 --openai-api-base-url http://localhost:8080 --circadian-interval 20s
```

## Video Overlay (BYOV)

If the `viewer` feature is enabled and `~/.rooster/rooster.mkv` exists, the spawn UI will play the video as a semi-transparent overlay.
AV1 video and Opus audio are expected.

The recommended viewing material is the music video for the 1993 hit single "Rooster," which can be downloaded from the internet archive and converted into the expected format with ffmpeg prior to spawning a rooster, using the following bash command:

```bash
mkdir -p "$HOME/.rooster" && \
  wget -O /tmp/rooster-source.mp4 \
    "https://archive.org/serve/alice-in-chains-rooster/Alice%20In%20Chains%20-%20Rooster.ia.mp4" && \
  ffmpeg -y -i /tmp/rooster-source.mp4 \
    -c:v libsvtav1 \
    -c:a libopus \
    /tmp/rooster.mkv && \
  mv /tmp/rooster.mkv "$HOME/.rooster/rooster.mkv" && \
  cargo run --release --example rooster --features viewer -- spawn --roost-addr [::1]:4433 --openai-api-base-url http://localhost:8080 --circadian-interval 20s
```

