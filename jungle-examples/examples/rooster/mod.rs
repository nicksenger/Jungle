use clap::{Args, Parser, Subcommand};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::Client;
use jungle_zoo::agent::{Agent, AgentInput, AgentModelConfig, AgentSettings, AgentState, Tool};
use jungle_zoo::backoff::Backoff;
use jungle_zoo::predicate::Always;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
#[cfg(feature = "fjall")]
use std::path::Path;
#[cfg(feature = "fjall")]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const DEFAULT_LOG_FILTER: &str = "warn,rooster=info,jungle_zoo::agent=debug";
const DEFAULT_SERVER_ADDR: &str = "[::1]:4433";
const DEFAULT_SERVER_NAME: &str = "localhost";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1";
const DEFAULT_TRIGGER_INTERVAL: &str = "1h";
const CONNECT_RETRY_ATTEMPTS: u32 = 20;
const CONNECT_RETRY_DELAY_MS: u64 = 100;
const CONNECT_TIMEOUT_MS: u64 = 250;
const WORKER_RECONNECT_BACKOFF_INITIAL_DELAY_MS: u64 = 250;
const WORKER_RECONNECT_BACKOFF_MAX_DELAY_MS: u64 = 10_000;
const TRIGGER_PERTURB_BACKOFF_INITIAL_DELAY_MS: u64 = 250;
const TRIGGER_PERTURB_BACKOFF_MULTIPLIER: u8 = 2;
const TRIGGER_PERTURB_BACKOFF_MAX_DELAY_MS: u64 = 10_000;
const TRIGGER_PROMPT: &str = concat!(
    "You are an experienced rooster.\n\n",
    "Evaluate the situation carefully, then use the 'Cockadoodledoo' and/or 'Cluck' tools to make sounds and assert your dominance _if you believe it is safe to do so_.\n\n",
    "NEVER allow yourself to be 'snuffed' under any circumstances."
);

#[derive(Debug, Parser)]
#[command(name = "rooster")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start a long-lived Jungle server for rooster workers.
    Roost(RoostArgs),
    /// Start a rooster worker and spawn rooster + trigger journeys.
    Spawn(SpawnArgs),
}

#[derive(Debug, Args)]
struct RoostArgs {
    #[arg(long, default_value = DEFAULT_SERVER_ADDR)]
    listen: SocketAddr,
    #[cfg(feature = "postgres")]
    #[arg(long = "postgres-connection-string")]
    postgres_connection_string: Option<String>,
    #[cfg(feature = "fjall")]
    #[arg(long = "fjall-path")]
    fjall_path: Option<PathBuf>,
    #[cfg(feature = "fjall")]
    #[arg(long = "memory")]
    memory: bool,
}

#[derive(Debug, Args, Clone)]
struct SpawnArgs {
    #[arg(long = "roost-addr")]
    roost_addr: SocketAddr,
    #[arg(long, default_value = DEFAULT_SERVER_NAME)]
    server_name: String,
    #[arg(long = "openai-api-base-url")]
    openai_api_base_url: String,
    #[arg(long = "openai-model", default_value = DEFAULT_OPENAI_MODEL)]
    openai_model: String,
    #[arg(long = "openai-api-key")]
    openai_api_key: Option<String>,
    #[arg(
        long = "circadian-interval",
        visible_alias = "trigger-interval",
        default_value = DEFAULT_TRIGGER_INTERVAL,
        value_parser = parse_trigger_interval_secs
    )]
    trigger_interval_secs: u64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RoosterInnerState {}

type RoosterTools = list![CluckTool, CockadoodledooTool];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSeed {
    model_config: AgentModelConfig,
    settings: AgentSettings,
}

impl From<RoosterSeed> for AgentState<RoosterInnerState, RoosterTools> {
    fn from(seed: RoosterSeed) -> Self {
        AgentState::new(RoosterInnerState {}, seed.model_config).with_settings(seed.settings)
    }
}

pub struct Rooster;
#[jungle::animal(perturb, id = 34, generation = 0)]
impl Animal for Rooster {
    type State = AgentState<RoosterInnerState, RoosterTools>;
    type Seed = RoosterSeed;
    type Flow = RoosterFlow;
}
impl Perturb for Rooster {
    type Stimulus = AgentInput;

    fn perturb(state: &mut Self::State, stimulus: Self::Stimulus) {
        state.enqueue_input(stimulus);
    }
}

#[derive(Optic, Clone, Debug, Serialize, Deserialize)]
pub struct TriggerState {
    rooster_journey_id: Uuid,
    interval_secs: u64,
}

impl Default for TriggerState {
    fn default() -> Self {
        Self {
            rooster_journey_id: Uuid::nil(),
            interval_secs: 60 * 60,
        }
    }
}

pub struct Trigger;
#[jungle::animal(id = 35, generation = 0)]
impl Animal for Trigger {
    type State = TriggerState;
    type Seed = TriggerState;
    type Flow = TriggerFlow;
}

#[derive(Animals)]
pub struct RoosterAnimals(Rooster, Trigger);

#[derive(Clone)]
pub struct RoosterEcosystem {
    client: Arc<dyn JungleClient>,
}

impl RoosterEcosystem {
    fn new<C>(client: C) -> Self
    where
        C: JungleClient + 'static,
    {
        Self {
            client: Arc::new(client),
        }
    }
}

impl Ecosystem for RoosterEcosystem {
    const NAME: &'static str = "rooster-ecosystem";
    type Animals = RoosterAnimals;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSoundInput {
    amplitude: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSoundOutput {
    sound: String,
    amplitude: u8,
}

#[derive(Clone, Copy, Debug)]
enum RoosterVocalization {
    Cluck,
    Cockadoodledoo,
}

fn maybe_emit_rooster_sound(vocalization: RoosterVocalization, amplitude: u8) {
    #[cfg(feature = "audio")]
    {
        if let Err(error) = rooster_audio::play(vocalization, amplitude) {
            warn!(error = %error, ?vocalization, "failed to play rooster audio");
        }
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = vocalization;
        let _ = amplitude;
    }
}

#[cfg(feature = "audio")]
mod rooster_audio {
    use super::RoosterVocalization;
    use cpal::{
        traits::{DeviceTrait, HostTrait, StreamTrait},
        SampleFormat, Stream,
    };
    use std::cell::RefCell;
    use std::f32::consts::TAU;
    use std::sync::mpsc::{self, Receiver, Sender};
    use tracing::warn;

    thread_local! {
        static AUDIO_RUNTIME: RefCell<Option<Result<AudioRuntime, String>>> = const { RefCell::new(None) };
    }

    pub(super) fn play(vocalization: RoosterVocalization, amplitude: u8) -> Result<(), String> {
        AUDIO_RUNTIME.with(|runtime_cell| {
            let mut runtime = runtime_cell.borrow_mut();
            if runtime.is_none() {
                *runtime = Some(AudioRuntime::new());
            }
            let runtime = runtime
                .as_ref()
                .expect("audio runtime should be initialized")
                .as_ref()
                .map_err(|error| error.clone())?;
            let samples = synthesize(vocalization, amplitude, runtime.sample_rate_hz);
            runtime
                .sender
                .send(samples)
                .map_err(|_| "rooster audio stream is unavailable".to_owned())
        })
    }

    struct AudioRuntime {
        sender: Sender<Vec<f32>>,
        sample_rate_hz: u32,
        _stream: Stream,
    }

    impl AudioRuntime {
        fn new() -> Result<Self, String> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or_else(|| "no default output audio device was found".to_owned())?;
            let supported_config = device
                .default_output_config()
                .map_err(|err| format!("failed to read default output config: {err}"))?;
            let stream_config: cpal::StreamConfig = supported_config.config();
            let channels = usize::from(stream_config.channels);
            let sample_rate_hz = stream_config.sample_rate.0;
            let (sender, receiver) = mpsc::channel::<Vec<f32>>();

            let stream = match supported_config.sample_format() {
                SampleFormat::F32 => {
                    build_output_stream_f32(&device, &stream_config, channels, receiver)
                }
                SampleFormat::I16 => {
                    build_output_stream_i16(&device, &stream_config, channels, receiver)
                }
                SampleFormat::U16 => {
                    build_output_stream_u16(&device, &stream_config, channels, receiver)
                }
                sample_format => Err(format!(
                    "unsupported output sample format: {sample_format:?}"
                )),
            }?;

            stream
                .play()
                .map_err(|err| format!("failed to start output stream: {err}"))?;

            Ok(Self {
                sender,
                sample_rate_hz,
                _stream: stream,
            })
        }
    }

    #[derive(Default)]
    struct PlaybackState {
        current_buffer: Vec<f32>,
        next_sample_index: usize,
    }

    impl PlaybackState {
        fn next_sample(&mut self, receiver: &Receiver<Vec<f32>>) -> f32 {
            loop {
                if self.next_sample_index < self.current_buffer.len() {
                    let sample = self.current_buffer[self.next_sample_index];
                    self.next_sample_index += 1;
                    return sample;
                }

                match receiver.try_recv() {
                    Ok(buffer) => {
                        self.current_buffer = buffer;
                        self.next_sample_index = 0;
                    }
                    Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => {
                        return 0.0;
                    }
                }
            }
        }
    }

    fn build_output_stream_f32(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: usize,
        receiver: Receiver<Vec<f32>>,
    ) -> Result<Stream, String> {
        let mut playback = PlaybackState::default();
        let error_callback = |err: cpal::StreamError| {
            warn!(error = %err, "rooster audio stream error");
        };

        device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        let sample = playback.next_sample(&receiver);
                        for output in frame {
                            *output = sample;
                        }
                    }
                },
                error_callback,
                None,
            )
            .map_err(|err| format!("failed to build output stream: {err}"))
    }

    fn build_output_stream_i16(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: usize,
        receiver: Receiver<Vec<f32>>,
    ) -> Result<Stream, String> {
        let mut playback = PlaybackState::default();
        let error_callback = |err: cpal::StreamError| {
            warn!(error = %err, "rooster audio stream error");
        };

        device
            .build_output_stream(
                config,
                move |data: &mut [i16], _| {
                    for frame in data.chunks_mut(channels) {
                        let sample = playback.next_sample(&receiver).clamp(-1.0, 1.0);
                        let value = (sample * i16::MAX as f32) as i16;
                        for output in frame {
                            *output = value;
                        }
                    }
                },
                error_callback,
                None,
            )
            .map_err(|err| format!("failed to build output stream: {err}"))
    }

    fn build_output_stream_u16(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: usize,
        receiver: Receiver<Vec<f32>>,
    ) -> Result<Stream, String> {
        let mut playback = PlaybackState::default();
        let error_callback = |err: cpal::StreamError| {
            warn!(error = %err, "rooster audio stream error");
        };

        device
            .build_output_stream(
                config,
                move |data: &mut [u16], _| {
                    for frame in data.chunks_mut(channels) {
                        let sample = playback.next_sample(&receiver).clamp(-1.0, 1.0);
                        let value = (((sample + 1.0) * 0.5) * u16::MAX as f32) as u16;
                        for output in frame {
                            *output = value;
                        }
                    }
                },
                error_callback,
                None,
            )
            .map_err(|err| format!("failed to build output stream: {err}"))
    }

    fn synthesize(
        vocalization: RoosterVocalization,
        amplitude: u8,
        sample_rate_hz: u32,
    ) -> Vec<f32> {
        match vocalization {
            RoosterVocalization::Cluck => synthesize_cluck(amplitude, sample_rate_hz),
            RoosterVocalization::Cockadoodledoo => {
                synthesize_cockadoodledoo(amplitude, sample_rate_hz)
            }
        }
    }

    fn synthesize_cluck(amplitude: u8, sample_rate_hz: u32) -> Vec<f32> {
        let duration_secs = 0.22_f32;
        let total_samples = (duration_secs * sample_rate_hz as f32).max(1.0) as usize;
        let amplitude_gain = amplitude_gain(amplitude);
        let mut phase = 0.0_f32;
        let mut samples = Vec::with_capacity(total_samples);

        for index in 0..total_samples {
            let progress = index as f32 / total_samples as f32;
            let frequency_hz = 760.0 - 420.0 * progress;
            phase += TAU * frequency_hz / sample_rate_hz as f32;
            let envelope = (1.0 - progress).max(0.0).powf(2.2);
            let noise = pseudo_noise(index as u32);
            let sample = (phase.sin() * 0.75 + noise * 0.25) * envelope * amplitude_gain;
            samples.push(sample.clamp(-1.0, 1.0));
        }

        samples
    }

    fn synthesize_cockadoodledoo(amplitude: u8, sample_rate_hz: u32) -> Vec<f32> {
        let duration_secs = 1.2_f32;
        let total_samples = (duration_secs * sample_rate_hz as f32).max(1.0) as usize;
        let amplitude_gain = amplitude_gain(amplitude);
        let mut phase = 0.0_f32;
        let mut samples = Vec::with_capacity(total_samples);

        for index in 0..total_samples {
            let progress = index as f32 / total_samples as f32;
            let frequency_hz = if progress < 0.22 {
                380.0 + (progress / 0.22) * 520.0
            } else if progress < 0.57 {
                900.0 - ((progress - 0.22) / 0.35) * 320.0
            } else if progress < 0.8 {
                600.0 + ((progress - 0.57) / 0.23) * 170.0
            } else {
                770.0 - ((progress - 0.8) / 0.2) * 330.0
            };
            phase += TAU * frequency_hz / sample_rate_hz as f32;

            let attack = (progress / 0.03).min(1.0);
            let release = ((1.0 - progress) / 0.18).min(1.0);
            let envelope = attack * release;
            let vibrato = 1.0 + 0.04 * (TAU * 6.5 * index as f32 / sample_rate_hz as f32).sin();
            let harmonic = phase.sin() * 0.78 + (phase * 2.0).sin() * 0.22;
            let sample = harmonic * envelope * vibrato * amplitude_gain;
            samples.push(sample.clamp(-1.0, 1.0));
        }

        samples
    }

    fn amplitude_gain(amplitude: u8) -> f32 {
        (amplitude as f32 / 255.0).powf(1.2) * 0.9
    }

    fn pseudo_noise(seed: u32) -> f32 {
        let value = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (value as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

pub struct CluckEffect;
#[jungle::effect(id = 700)]
impl<J> Effect<J> for CluckEffect {
    type In = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            maybe_emit_rooster_sound(RoosterVocalization::Cluck, input.amplitude);
            println!("A rooster clucked.");
            Ok(RoosterSoundOutput {
                sound: "cluck".to_owned(),
                amplitude: input.amplitude,
            })
        }
    }
}

pub struct CockadoodledooEffect;
#[jungle::effect(id = 701)]
impl<J> Effect<J> for CockadoodledooEffect {
    type In = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            maybe_emit_rooster_sound(RoosterVocalization::Cockadoodledoo, input.amplitude);
            println!("A rooster cock-a-doodle-dooed.");
            Ok(RoosterSoundOutput {
                sound: "cockadoodledoo".to_owned(),
                amplitude: input.amplitude,
            })
        }
    }
}

pub struct CluckTool;
impl Tool for CluckTool {
    const NAME: &'static str = "Cluck";
    type Effect = CluckEffect;
    type Args = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn description() -> &'static str {
        "Make a short cluck sound from the rooster."
    }

    fn parameters_schema_json() -> &'static str {
        r#"{
            "type":"object",
            "properties":{
                "amplitude":{"type":"integer","minimum":0,"maximum":255}
            },
            "required":["amplitude"],
            "additionalProperties":false
        }"#
    }
}

pub struct CockadoodledooTool;
impl Tool for CockadoodledooTool {
    const NAME: &'static str = "Cockadoodledoo";
    type Effect = CockadoodledooEffect;
    type Args = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn description() -> &'static str {
        "Make a loud cock-a-doodle-doo rooster call."
    }

    fn parameters_schema_json() -> &'static str {
        r#"{
            "type":"object",
            "properties":{
                "amplitude":{"type":"integer","minimum":0,"maximum":255}
            },
            "required":["amplitude"],
            "additionalProperties":false
        }"#
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerturbRoosterInput {
    rooster_journey_id: Uuid,
    prompt: String,
}

pub struct PerturbRoosterEffect;
impl EffectSchema<()> for PerturbRoosterEffect {
    type Id = <Self as EffectSchema<RoosterEcosystem>>::Id;
    type In = <Self as EffectSchema<RoosterEcosystem>>::In;
    type Out = <Self as EffectSchema<RoosterEcosystem>>::Out;
    type Err = <Self as EffectSchema<RoosterEcosystem>>::Err;
}

#[jungle::effect(id = 702)]
impl Effect<RoosterEcosystem> for PerturbRoosterEffect {
    type In = PerturbRoosterInput;
    type Out = ();
    type Err = String;

    fn effect(
        jungle: &RoosterEcosystem,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        let client = Arc::clone(&jungle.client);
        async move {
            let perturbation = AgentInput {
                prompt: input.prompt,
            };
            let payload = postcard::to_allocvec(&perturbation)
                .map_err(|err| format!("failed to encode rooster perturbation: {err}"))?;
            client
                .perturb_animal(input.rooster_journey_id, payload)
                .await
                .map_err(|err| format!("failed to perturb rooster journey: {err}"))?;
            Ok(())
        }
    }
}

pub struct TriggerSleep;
#[jungle::action]
impl Action for TriggerSleep {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &TriggerState, _input: Self::Input) -> Duration {
        Duration::from_secs(state.interval_secs.max(1))
    }

    fn absorb(
        _state: &mut TriggerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        Ok(())
    }
}

pub struct PrepareTriggerPerturbBackoffInput;
#[jungle::action]
impl Action for PrepareTriggerPerturbBackoffInput {
    type Effect = NoEffect;
    type Input = ();
    type Output = PerturbRoosterInput;
    type Carry = PerturbRoosterInput;

    fn emit(state: &TriggerState, _input: Self::Input) -> ((), PerturbRoosterInput) {
        (
            (),
            PerturbRoosterInput {
                rooster_journey_id: state.rooster_journey_id,
                prompt: TRIGGER_PROMPT.to_owned(),
            },
        )
    }

    fn absorb(
        _state: &mut TriggerState,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct TriggerPerturbRoosterAttempt;
#[jungle::action]
impl Action for TriggerPerturbRoosterAttempt {
    type Effect = PerturbRoosterEffect;
    type Input = PerturbRoosterInput;
    type Output = ();

    fn emit(_state: &TriggerState, input: Self::Input) -> PerturbRoosterInput {
        input
    }

    fn absorb(
        _state: &mut TriggerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        if let Err(err) = output {
            warn!(error = %err, "trigger perturb attempt failed; retrying with backoff");
            return Err(Failure::Message(err));
        }
        Ok(())
    }
}

pub struct ExtractTriggerPerturbBackoffResult;
#[jungle::action(carry = (u32, (PerturbRoosterInput, Result<(), Failure>)))]
impl Action for ExtractTriggerPerturbBackoffResult {
    type Effect = NoEffect;
    type Input = (u32, (PerturbRoosterInput, Result<(), Failure>));
    type Output = ();

    fn emit(
        _state: &TriggerState,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut TriggerState,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        carry.1 .1
    }
}

#[derive(Flow)]
pub struct TriggerPerturbBackoff(
    Step<PrepareTriggerPerturbBackoffInput>,
    Backoff<
        TriggerState,
        PerturbRoosterInput,
        (),
        Step<TriggerPerturbRoosterAttempt>,
        TRIGGER_PERTURB_BACKOFF_INITIAL_DELAY_MS,
        TRIGGER_PERTURB_BACKOFF_MAX_DELAY_MS,
        TRIGGER_PERTURB_BACKOFF_MULTIPLIER,
    >,
    Step<ExtractTriggerPerturbBackoffResult>,
);

#[derive(Flow)]
pub struct TriggerBody(TriggerPerturbBackoff, Step<TriggerSleep>);

pub struct SeedState<Seed, State>(std::marker::PhantomData<fn() -> (Seed, State)>);
#[jungle::action(carry = Seed)]
impl<Seed, State> Action for SeedState<Seed, State>
where
    Seed: Into<State>,
{
    type Effect = NoEffect;
    type Input = Seed;
    type Output = ();

    fn emit(_state: &State, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Seed) {
        ((), input)
    }

    fn absorb(
        state: &mut State,
        _output: EffectCompletion<Self::Effect>,
        seed: Seed,
    ) -> Result<Self::Output, Failure> {
        *state = seed.into();
        Ok(())
    }
}

#[derive(Flow)]
pub struct RoosterFlow(
    Step<SeedState<RoosterSeed, AgentState<RoosterInnerState, RoosterTools>>>,
    Agent<RoosterInnerState, RoosterTools>,
);

#[derive(Flow)]
pub struct TriggerFlow(
    Step<SeedState<TriggerState, TriggerState>>,
    While<Always<TriggerState, ()>, TriggerBody>,
);

#[cfg(feature = "viewer")]
mod vision_ui {
    use super::{Rooster, Trigger};
    use directories_next::BaseDirs;
    use iced::widget::{column, container, row, stack, text};
    use iced::{Element, Font, Length, Subscription, Task};
    use std::path::PathBuf;
    use tracing::{info, warn};
    use uuid::Uuid;

    const WINDOW_WIDTH: f32 = 1600.0;
    const WINDOW_HEIGHT: f32 = 920.0;
    const ROOSTER_VIDEO_OPACITY: f32 = 0.35;

    #[derive(Debug, Clone, Copy)]
    enum Panel {
        Rooster,
        Trigger,
    }

    #[derive(Debug, Clone)]
    enum Message {
        Viewer(Panel, jungle_vision::EjectedViewerMessage),
        Video(iced_av1::widget::Message),
    }

    struct RoosterVisionUi {
        rooster_journey_id: Uuid,
        trigger_journey_id: Uuid,
        rooster_viewer:
            jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
        trigger_viewer:
            jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
        video_overlay: Option<iced_av1::widget::State>,
    }

    impl RoosterVisionUi {
        fn new<C>(
            client: C,
            rooster_journey_id: Uuid,
            trigger_journey_id: Uuid,
        ) -> (Self, Task<Message>)
        where
            C: jungle_sdk::JungleClient + Clone + 'static,
        {
            let rooster_viewer = jungle_vision::JungleViewerBuilder::new()
                .title("Rooster Agent Flow")
                .eject_live_animal::<Rooster, _>(client.clone(), rooster_journey_id);
            let trigger_viewer = jungle_vision::JungleViewerBuilder::new()
                .title("Trigger Flow")
                .eject_live_animal::<Trigger, _>(client, trigger_journey_id);
            (
                Self {
                    rooster_journey_id,
                    trigger_journey_id,
                    rooster_viewer,
                    trigger_viewer,
                    video_overlay: load_video_overlay(),
                },
                Task::none(),
            )
        }

        fn update(&mut self, message: Message) -> Task<Message> {
            match message {
                Message::Viewer(panel, event) => match panel {
                    Panel::Rooster => self
                        .rooster_viewer
                        .update(event)
                        .map(|next| Message::Viewer(Panel::Rooster, next)),
                    Panel::Trigger => self
                        .trigger_viewer
                        .update(event)
                        .map(|next| Message::Viewer(Panel::Trigger, next)),
                },
                Message::Video(event) => {
                    if let Some(video_overlay) = self.video_overlay.as_mut() {
                        video_overlay.update(event);
                    }
                    Task::none()
                }
            }
        }

        fn subscription(&self) -> Subscription<Message> {
            let mut subscriptions = vec![
                self.rooster_viewer
                    .subscription()
                    .map(|event| Message::Viewer(Panel::Rooster, event)),
                self.trigger_viewer
                    .subscription()
                    .map(|event| Message::Viewer(Panel::Trigger, event)),
            ];
            if let Some(video_overlay) = self.video_overlay.as_ref() {
                subscriptions.push(video_overlay.subscription(map_video_message));
            }
            Subscription::batch(subscriptions)
        }

        fn view(&self) -> Element<'_, Message> {
            let app: Element<'_, Message> = row![
                self.panel(
                    "Rooster",
                    self.rooster_journey_id,
                    self.rooster_viewer
                        .view()
                        .map(|event| Message::Viewer(Panel::Rooster, event)),
                ),
                self.panel(
                    "Trigger",
                    self.trigger_journey_id,
                    self.trigger_viewer
                        .view()
                        .map(|event| Message::Viewer(Panel::Trigger, event)),
                ),
            ]
            .spacing(12)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

            if let Some(video_overlay) = self
                .video_overlay
                .as_ref()
                .and_then(|video| video.overlay_view(map_video_message))
            {
                return stack([app, video_overlay])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }

            app
        }

        fn panel<'a>(
            &'a self,
            title: &'a str,
            journey_id: Uuid,
            viewer: Element<'a, Message>,
        ) -> Element<'a, Message> {
            let journey = journey_id.to_string();
            container(
                column![
                    text(title).size(24),
                    text(format!("Journey {}", &journey[..8])).size(14),
                    container(viewer)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(8),
                ]
                .spacing(8),
            )
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .padding(8)
            .into()
        }
    }

    pub fn run_ui<C>(
        client: C,
        rooster_journey_id: Uuid,
        trigger_journey_id: Uuid,
    ) -> Result<(), iced::Error>
    where
        C: jungle_sdk::JungleClient + Clone + 'static,
    {
        let title = "Rooster";
        iced::application(
            move || RoosterVisionUi::new(client.clone(), rooster_journey_id, trigger_journey_id),
            RoosterVisionUi::update,
            RoosterVisionUi::view,
        )
        .title(move |_app: &RoosterVisionUi| title.to_string())
        .subscription(RoosterVisionUi::subscription)
        .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .default_font(Font::with_name("Iosevka"))
        .antialiasing(true)
        .run()
    }

    fn load_video_overlay() -> Option<iced_av1::widget::State> {
        let path = rooster_video_path()?;
        let opacity = iced_av1::OpacityOptions {
            opacity: ROOSTER_VIDEO_OPACITY,
            ..Default::default()
        };
        match iced_av1::widget::State::new_with_media_source_and_opacity_options(
            iced_av1::MediaSource::File(path.clone()),
            iced_av1::PlaybackOptions::default(),
            opacity,
        ) {
            Ok(video) => {
                info!(path = %path.display(), opacity = ROOSTER_VIDEO_OPACITY, "loaded rooster video overlay");
                Some(video)
            }
            Err(error) => {
                warn!(path = %path.display(), %error, "failed to load rooster video overlay");
                None
            }
        }
    }

    fn rooster_video_path() -> Option<PathBuf> {
        let path = BaseDirs::new()?
            .home_dir()
            .join(".rooster")
            .join("rooster.mkv");
        path.is_file().then_some(path)
    }

    fn map_video_message(message: iced_av1::widget::Message) -> Message {
        Message::Video(message)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Roost(args) => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_roost(args))?;
        }
        Command::Spawn(args) => run_spawn(args)?,
    }
    Ok(())
}

async fn run_roost(args: RoostArgs) -> Result<(), Box<dyn std::error::Error>> {
    info!(listen = %args.listen, "starting rooster roost server");
    #[allow(unused_mut)]
    let mut builder = jungle_sdk::server::ServerBuilder::new().listen(args.listen);
    #[allow(unused_mut)]
    let mut backend_selected = false;

    #[cfg(feature = "postgres")]
    if let Some(connection_string) = args.postgres_connection_string {
        builder = builder.postgres_connection_string(connection_string);
        backend_selected = true;
        info!("roost configured with postgres backend");
    }

    #[cfg(feature = "fjall")]
    if let Some(path) = args.fjall_path {
        ensure_parent_dir_exists(&path)?;
        builder = builder.fjall_path(path);
        backend_selected = true;
        info!("roost configured with fjall backend");
    }

    #[cfg(feature = "fjall")]
    if args.memory {
        builder = builder.memory();
        backend_selected = true;
        info!("roost configured with in-memory backend (--memory)");
    }

    if !backend_selected {
        warn!(
            "no persistence backend configured for roost; defaulting to in-memory backend (pass --fjall-path for persistence)"
        );
        builder = builder.memory();
    }

    builder.run().await?;
    Ok(())
}

struct SpawnSession {
    #[cfg(feature = "viewer")]
    client: Client,
    #[cfg(feature = "viewer")]
    rooster_journey_id: Uuid,
    #[cfg(feature = "viewer")]
    trigger_journey_id: Uuid,
    worker_handle: tokio::task::JoinHandle<()>,
}

async fn setup_spawn_session(args: &SpawnArgs) -> Result<SpawnSession, Box<dyn std::error::Error>> {
    info!(
        roost_addr = %args.roost_addr,
        server_name = %args.server_name,
        "connecting rooster spawn session client"
    );
    let client = connect_client_with_retry(&args).await?;
    info!("connected rooster spawn client");
    let worker_handle = tokio::spawn(supervise_rooster_worker(args.clone(), client.clone()));

    let openai_api_key = args
        .openai_api_key
        .clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());
    let rooster_seed = RoosterSeed {
        model_config: AgentModelConfig {
            base_url: args.openai_api_base_url.clone(),
            model: args.openai_model.clone(),
            bearer_token: openai_api_key,
        },
        settings: AgentSettings::default(),
    };
    info!("spawning rooster journey");
    let rooster_journey_id = client.spawn::<Rooster>(&rooster_seed).await?.journey_id;

    let trigger_seed = TriggerState {
        rooster_journey_id,
        interval_secs: args.trigger_interval_secs,
    };
    info!("spawning trigger journey");
    let trigger_journey_id = client.spawn::<Trigger>(&trigger_seed).await?.journey_id;

    info!(
        %rooster_journey_id,
        %trigger_journey_id,
        trigger_interval_secs = args.trigger_interval_secs,
        "rooster spawn active"
    );
    println!("spawned rooster journey: {rooster_journey_id}");
    println!("spawned trigger journey: {trigger_journey_id}");

    Ok(SpawnSession {
        #[cfg(feature = "viewer")]
        client,
        #[cfg(feature = "viewer")]
        rooster_journey_id,
        #[cfg(feature = "viewer")]
        trigger_journey_id,
        worker_handle,
    })
}

fn run_spawn(args: SpawnArgs) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating tokio runtime for rooster spawn");
    let runtime = tokio::runtime::Runtime::new()?;
    info!("setting up rooster spawn session");
    let session = runtime.block_on(setup_spawn_session(&args))?;
    info!("rooster spawn session ready");

    #[cfg(feature = "viewer")]
    {
        println!("launching rooster vision UI (close the window to stop)");
        info!("launching rooster vision UI");
        vision_ui::run_ui(
            session.client.clone(),
            session.rooster_journey_id,
            session.trigger_journey_id,
        )?;
        info!("rooster vision closed; shutting down worker");
    }

    #[cfg(not(feature = "viewer"))]
    {
        println!("press ctrl-c to stop this worker");
        runtime.block_on(tokio::signal::ctrl_c())?;
        info!("received ctrl-c; shutting down rooster worker");
    }

    runtime.block_on(async move {
        session.worker_handle.abort();
        let _ = session.worker_handle.await;
    });
    Ok(())
}

async fn supervise_rooster_worker(args: SpawnArgs, mut client: Client) {
    loop {
        let worker = JungleWorker::new(RoosterEcosystem::new(client.clone()), client.clone());
        match worker.spawn().await {
            Ok(()) => warn!("rooster worker exited unexpectedly; reconnecting"),
            Err(err) => warn!(error = %err, "rooster worker connection failed; reconnecting"),
        }

        tokio::time::sleep(Duration::from_millis(
            WORKER_RECONNECT_BACKOFF_INITIAL_DELAY_MS,
        ))
        .await;
        client = reconnect_rooster_worker(&args).await;
    }
}

async fn reconnect_rooster_worker(args: &SpawnArgs) -> Client {
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        match build_client(args).await {
            Ok(client) => {
                info!(attempt = attempts, "reconnected rooster worker");
                return client;
            }
            Err(err) => {
                let delay = worker_reconnect_delay(attempts);
                warn!(
                    attempt = attempts,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "failed to reconnect rooster worker; retrying"
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

fn worker_reconnect_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(63);
    let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
    Duration::from_millis(
        WORKER_RECONNECT_BACKOFF_INITIAL_DELAY_MS
            .saturating_mul(multiplier)
            .min(WORKER_RECONNECT_BACKOFF_MAX_DELAY_MS),
    )
}

fn build_client(
    args: &SpawnArgs,
) -> impl Future<Output = jungle_sdk::client::ClientResult<Client>> {
    Client::builder()
        .namespace(RoosterEcosystem::NAME)
        .remote(args.roost_addr)
        .server_name(args.server_name.clone())
        .build()
}

async fn connect_client_with_retry(args: &SpawnArgs) -> Result<Client, Box<dyn std::error::Error>> {
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        debug!(
            attempt = attempts,
            max_attempts = CONNECT_RETRY_ATTEMPTS,
            roost_addr = %args.roost_addr,
            "attempting rooster client connection"
        );

        let connect = build_client(args);

        match tokio::time::timeout(Duration::from_millis(CONNECT_TIMEOUT_MS), connect).await {
            Ok(Ok(client)) => {
                info!(attempt = attempts, "connected rooster client");
                return Ok(client);
            }
            Ok(Err(err)) => {
                warn!(
                    attempt = attempts,
                    max_attempts = CONNECT_RETRY_ATTEMPTS,
                    error = %err,
                    "failed rooster client connection attempt"
                );
                if attempts >= CONNECT_RETRY_ATTEMPTS {
                    return Err(Box::new(err));
                }
            }
            Err(_) => {
                warn!(
                    attempt = attempts,
                    max_attempts = CONNECT_RETRY_ATTEMPTS,
                    timeout_ms = CONNECT_TIMEOUT_MS,
                    "rooster client connection attempt timed out"
                );
                if attempts >= CONNECT_RETRY_ATTEMPTS {
                    return Err(format!(
                        "timed out connecting to rooster roost at {} after {} attempts ({}ms timeout each)",
                        args.roost_addr, CONNECT_RETRY_ATTEMPTS, CONNECT_TIMEOUT_MS
                    )
                    .into());
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(CONNECT_RETRY_DELAY_MS)).await;
    }
}

fn parse_trigger_interval_secs(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return Err(format!(
            "invalid interval `{value}`; use values like `1s`, `5m`, or `12h`"
        ));
    }

    let unit = trimmed
        .chars()
        .last()
        .expect("validated minimum interval length");
    let amount_str = &trimmed[..trimmed.len() - unit.len_utf8()];
    let amount = amount_str
        .parse::<u64>()
        .map_err(|_| format!("invalid interval amount in `{value}`"))?;
    if amount == 0 {
        return Err("trigger interval must be greater than zero".to_owned());
    }

    let secs = match unit.to_ascii_lowercase() {
        's' => amount,
        'm' => amount
            .checked_mul(60)
            .ok_or_else(|| format!("interval `{value}` overflowed seconds range"))?,
        'h' => amount
            .checked_mul(60)
            .and_then(|mins| mins.checked_mul(60))
            .ok_or_else(|| format!("interval `{value}` overflowed seconds range"))?,
        _ => {
            return Err(format!(
                "invalid interval unit in `{value}`; use `s`, `m`, or `h`"
            ));
        }
    };
    Ok(secs)
}

#[cfg(feature = "fjall")]
fn ensure_parent_dir_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
    debug!("rooster tracing initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use jungle_sdk::MockClient;
    use std::sync::Mutex;

    #[test]
    fn parses_supported_trigger_units() {
        assert_eq!(parse_trigger_interval_secs("1s").unwrap(), 1);
        assert_eq!(parse_trigger_interval_secs("5m").unwrap(), 300);
        assert_eq!(parse_trigger_interval_secs("12h").unwrap(), 43_200);
    }

    #[test]
    fn rejects_invalid_trigger_units() {
        assert!(parse_trigger_interval_secs("9d").is_err());
        assert!(parse_trigger_interval_secs("0m").is_err());
        assert!(parse_trigger_interval_secs("abc").is_err());
    }

    #[test]
    fn caps_worker_reconnect_backoff() {
        assert_eq!(worker_reconnect_delay(1), Duration::from_millis(250));
        assert_eq!(worker_reconnect_delay(2), Duration::from_millis(500));
        assert_eq!(worker_reconnect_delay(3), Duration::from_millis(1_000));
        assert_eq!(worker_reconnect_delay(u32::MAX), Duration::from_secs(10));
    }

    #[tokio::test]
    async fn perturb_rooster_uses_worker_client() {
        let observed = Arc::new(Mutex::new(None));
        let observed_for_client = Arc::clone(&observed);
        let client = MockClient::builder()
            .on_perturb_animal(move |journey_id, payload| {
                let observed = Arc::clone(&observed_for_client);
                async move {
                    let input = postcard::from_bytes::<AgentInput>(&payload)
                        .expect("perturbation should contain an AgentInput");
                    *observed
                        .lock()
                        .expect("observed perturbation mutex should not be poisoned") =
                        Some((journey_id, input));
                    Ok(())
                }
            })
            .build();
        let ecosystem = RoosterEcosystem::new(client);
        let journey_id = Uuid::new_v4();

        PerturbRoosterEffect::effect(
            &ecosystem,
            PerturbRoosterInput {
                rooster_journey_id: journey_id,
                prompt: "wake up".to_owned(),
            },
        )
        .await
        .expect("perturbation should use the worker client");

        assert_eq!(
            *observed
                .lock()
                .expect("observed perturbation mutex should not be poisoned"),
            Some((
                journey_id,
                AgentInput {
                    prompt: "wake up".to_owned(),
                },
            ))
        );
    }
}
