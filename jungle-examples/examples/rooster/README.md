# Rooster

This is an example of a durable agent with tools and cross-flow communication.

The `roost` command runs a long-lived server, and `spawn` starts a worker that spawns and coordinates two durable journeys:

- `Rooster`: an agent flow with tool calls (`Cluck` and `Cockadoodledoo`)
- `Trigger`: a recurring flow that perturbs the rooster journey on an interval

## Run the Server

```bash
cargo run --release --example rooster --features fjall -- roost --fjall-path ~/.rooster
```

## Run the Worker

```bash
cargo run --release --example rooster --features viewer -- spawn --roost-addr [::1]:4433 --openai-api-base-url http://localhost:8080 --circadian-interval 20s
```

## Video Overlay (BYOV)

If `~/.rooster/rooster.mkv` exists, the viewer worker loads it as a semi-transparent overlay in the UI.
AV1 video and Opus audio are expected.

You can download the Rooster music video, convert it to the expected format, place it in the right location, and then start the worker in one command:

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
