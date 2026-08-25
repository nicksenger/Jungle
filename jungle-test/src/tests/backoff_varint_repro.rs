//! Regression test: `Backoff` with a non-unit input whose postcard encoding
//! contains a trailing varint field (e.g. black-hole-sun's
//! `OptimizeWithBackoff` with `In = Potentiation { f32, f32, u64 }`).
//!
//! With postcard >= 1.1 varint integer encoding, the backoff sentinel loop
//! input `(0u32, (In, Err(Failure::Message("test"))))` can be re-parsed by
//! `decode_loop_input` as a literal `(false, ...)` continue-flag envelope:
//! `u32::ZERO` is byte-identical to `bool::FALSE`, and the shifted varint
//! suffix still parses. The retry while then exits without ever running its
//! body, and `ExtractBackoffResult` fails with the sentinel
//! `Err(Message("test"))`, panicking the worker with
//! "while child inline completion should succeed".
//!
//! This test drives a real `Backoff` journey end-to-end. On the buggy code
//! the worker panics and the journey never completes, so the timeout below
//! fires and the test fails.
use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Same field layout as `black_hole_spec::Potentiation`: two f32 losses
/// followed by a u64 seed. `loss_up = 0.0` keeps the first byte of the
/// shifted `(bool, In)` misparse below the varint continuation threshold, and
/// the large seed makes its varint long enough to absorb the one-byte shift.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PotentiationLike {
    pub loss_up: f32,
    pub loss_down: f32,
    pub seed: u64,
}

struct SucceedOnPot;

#[jungle::action]
impl Action for SucceedOnPot {
    type Effect = NoEffect;
    type Input = PotentiationLike;
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) {}

    fn absorb(
        _state: &mut (),
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

type BackoffPotFlow = jungle_zoo::backoff::Backoff<
    (),
    PotentiationLike,
    (),
    Step<SucceedOnPot>,
    100u64,
    10_000u64,
    2u8,
>;

struct BackoffPotAnimal;

#[jungle::animal(id = 214, generation = 0)]
impl Animal for BackoffPotAnimal {
    type State = ();
    type Seed = PotentiationLike;
    type Flow = BackoffPotFlow;
}

#[derive(Animals)]
struct BackoffPotAnimals(BackoffPotAnimal);

struct BackoffPotZoo;

impl Ecosystem for BackoffPotZoo {
    const NAME: &'static str = "backoff-varint-repro-zoo";
    type Animals = BackoffPotAnimals;
}

#[tokio::test]
async fn backoff_with_varint_input_completes_first_attempt() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.fjall");
    let backend = Server::builder()
        .fjall_path(&db_path)
        .build()
        .await
        .expect("local server backend should build");
    let client = FusedClient::builder()
        .namespace(BackoffPotZoo::NAME)
        .backend(backend)
        .build()
        .await
        .expect("local fused client should build");

    let worker_client = client.clone();
    let worker = JungleWorker::new(BackoffPotZoo, worker_client);
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = PotentiationLike {
        loss_up: 0.0,
        loss_down: 0.0,
        seed: 0x9622_1c82_3527_3ab4,
    };
    let journey_id = client
        .spawn::<BackoffPotAnimal>(&seed)
        .await
        .expect("journey should spawn")
        .journey_id;
    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");

    // The attempt succeeds immediately: the journey must complete without
    // scheduling any backoff sleep. On the buggy code the worker panics on
    // the sentinel misparse and this times out.
    let sleep_count = tokio::time::timeout(Duration::from_secs(10), async {
        let mut sleep_count = 0;
        while let Some(update) = subscription.next().await {
            let update = update.expect("step update should decode");
            if matches!(update.event, RunnerUpdateOut::SleepScheduled { .. }) {
                sleep_count += 1;
            }
        }
        sleep_count
    })
    .await
    .expect(
        "backoff with a varint-carrying input should complete its first attempt \
         (regression: sentinel misparsed as a false continue-flag)",
    );

    assert_eq!(
        sleep_count, 0,
        "a successful first attempt must not schedule a backoff sleep"
    );

    worker_handle.abort();
    let _ = worker_handle.await;
}
