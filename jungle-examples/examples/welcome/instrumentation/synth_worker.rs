use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, trace, warn};

use super::{
    bass::BassArticulation,
    cymbal::CymbalArticulation,
    electric_guitar::ElectricGuitarArticulation,
    hihat::HiHatArticulation,
    kick_drum::KickDrumArticulation,
    snare_drum::SnareDrumArticulation,
    toms::TomsArticulation,
    vocals::VocalsArticulation,
    Error, Note,
};

const DEFAULT_SYNTH_WORKER_THREADS: usize = 9;
const DEFAULT_SYNTH_QUEUE_CAPACITY_PER_WORKER: usize = 128;
const SYNTH_DISPATCH_LOG_INTERVAL: usize = 256;
const SYNTH_SLOW_REQUEST_WARN_THRESHOLD: Duration = Duration::from_millis(20);
const SYNTH_SLOW_FALLBACK_WARN_THRESHOLD: Duration = Duration::from_millis(10);

#[derive(Clone)]
pub struct SynthHandle {
    request_txs: Arc<[mpsc::Sender<SynthRequest>]>,
    next_worker: Arc<AtomicUsize>,
    queue_capacity_per_worker: usize,
    dispatch_attempts: Arc<AtomicUsize>,
    overload_fallbacks: Arc<AtomicUsize>,
    closed_dispatch_failures: Arc<AtomicUsize>,
}

impl SynthHandle {
    pub fn new() -> Self {
        Self::new_with_config(
            DEFAULT_SYNTH_WORKER_THREADS,
            DEFAULT_SYNTH_QUEUE_CAPACITY_PER_WORKER,
        )
    }

    pub fn new_with_config(worker_threads: usize, queue_capacity_per_worker: usize) -> Self {
        let worker_threads = worker_threads.max(1);
        let queue_capacity_per_worker = queue_capacity_per_worker.max(1);
        info!(
            worker_threads,
            queue_capacity_per_worker, "starting welcome synth worker pool"
        );
        let mut request_txs = Vec::with_capacity(worker_threads);
        for worker_index in 0..worker_threads {
            let (request_tx, mut request_rx) =
                mpsc::channel::<SynthRequest>(queue_capacity_per_worker);
            std::thread::Builder::new()
                .name(format!("welcome-synth-worker-{worker_index}"))
                .spawn(move || {
                    while let Some(request) = request_rx.blocking_recv() {
                        run_synth_request(worker_index, request);
                    }
                })
                .expect("welcome synth worker thread should start");
            request_txs.push(request_tx);
        }

        Self {
            request_txs: Arc::from(request_txs),
            next_worker: Arc::new(AtomicUsize::new(0)),
            queue_capacity_per_worker,
            dispatch_attempts: Arc::new(AtomicUsize::new(0)),
            overload_fallbacks: Arc::new(AtomicUsize::new(0)),
            closed_dispatch_failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn bass(
        &self,
        note: Note<BassArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "bass",
            move |response| SynthRequest::Bass { note, response },
            move || welcome_audio::dsp::bass::synthesize_bass(&to_dsp_note(note, ())),
        )
        .await
    }

    pub async fn cymbal(
        &self,
        note: Note<CymbalArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "cymbal",
            move |response| SynthRequest::Cymbal { note, response },
            move || welcome_audio::dsp::cymbal::synthesize_cymbal(&to_dsp_note(note, ())),
        )
        .await
    }

    pub async fn electric_guitar(
        &self,
        note: Note<ElectricGuitarArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32, f32), Error> {
        self.dispatch_with_fallback(
            "electric_guitar",
            move |response| SynthRequest::ElectricGuitar { note, response },
            move || {
                welcome_audio::dsp::electric_guitar::synthesize_electric_guitar(&to_dsp_note(
                    note,
                    to_dsp_electric_guitar_articulation(note.articulation),
                ))
            },
        )
        .await
    }

    pub async fn hihat(
        &self,
        note: Note<HiHatArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "hihat",
            move |response| SynthRequest::HiHat { note, response },
            move || welcome_audio::dsp::hihat::synthesize_hihat(&to_dsp_note(note, ())),
        )
        .await
    }

    pub async fn kick_drum(
        &self,
        note: Note<KickDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "kick_drum",
            move |response| SynthRequest::KickDrum { note, response },
            move || welcome_audio::dsp::kick_drum::synthesize_kick_drum(&to_dsp_note(note, ())),
        )
        .await
    }

    pub async fn snare_drum(
        &self,
        note: Note<SnareDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "snare_drum",
            move |response| SynthRequest::SnareDrum { note, response },
            move || {
                welcome_audio::dsp::snare_drum::synthesize_snare_drum(&to_dsp_note(note, ()))
            },
        )
        .await
    }

    pub async fn toms(
        &self,
        note: Note<TomsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "toms",
            move |response| SynthRequest::Toms { note, response },
            move || welcome_audio::dsp::toms::synthesize_toms(&to_dsp_note(note, ())),
        )
        .await
    }

    pub async fn vocals(
        &self,
        note: Note<VocalsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            "vocals",
            move |response| SynthRequest::Vocals { note, response },
            move || {
                welcome_audio::dsp::vocals::synthesize_vocals(&to_dsp_note(
                    note,
                    to_dsp_vocals_articulation(note.articulation),
                ))
            },
        )
        .await
    }

    async fn dispatch_with_fallback<T, FBuild, FFallback>(
        &self,
        synth: &'static str,
        build_request: FBuild,
        fallback: FFallback,
    ) -> Result<T, Error>
    where
        T: Send + 'static,
        FBuild: FnOnce(oneshot::Sender<T>) -> SynthRequest,
        FFallback: FnOnce() -> T,
    {
        let dispatch_started_at = Instant::now();
        let dispatch_attempt = self.dispatch_attempts.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        let request = build_request(tx);

        match self.try_dispatch(request) {
            Ok(()) => {
                let output = rx.await.map_err(|_| Error::Playback)?;
                let dispatch_elapsed = dispatch_started_at.elapsed();
                if dispatch_elapsed > SYNTH_SLOW_REQUEST_WARN_THRESHOLD {
                    warn!(
                        synth,
                        dispatch_attempt,
                        dispatch_elapsed_ms = dispatch_elapsed.as_millis(),
                        "slow synth request turnaround"
                    );
                } else if dispatch_attempt % SYNTH_DISPATCH_LOG_INTERVAL == 0 {
                    let (max_depth, avg_depth) = self.queue_depth_snapshot();
                    debug!(
                        synth,
                        dispatch_attempt,
                        worker_count = self.request_txs.len(),
                        queue_capacity_per_worker = self.queue_capacity_per_worker,
                        max_depth,
                        avg_depth,
                        "synth dispatch heartbeat"
                    );
                }
                Ok(output)
            }
            Err(SynthDispatchError::Overloaded(_request)) => {
                let fallback_started_at = Instant::now();
                let output = fallback();
                let fallback_elapsed = fallback_started_at.elapsed();
                let fallback_count = self.overload_fallbacks.fetch_add(1, Ordering::Relaxed) + 1;
                let (max_depth, avg_depth) = self.queue_depth_snapshot();
                if fallback_elapsed > SYNTH_SLOW_FALLBACK_WARN_THRESHOLD
                    || fallback_count % SYNTH_DISPATCH_LOG_INTERVAL == 0
                {
                    warn!(
                        synth,
                        fallback_count,
                        fallback_elapsed_ms = fallback_elapsed.as_millis(),
                        worker_count = self.request_txs.len(),
                        queue_capacity_per_worker = self.queue_capacity_per_worker,
                        max_depth,
                        avg_depth,
                        "synth worker queues overloaded; used inline synth fallback"
                    );
                } else {
                    debug!(
                        synth,
                        fallback_count, max_depth, avg_depth, "synth overload fallback engaged"
                    );
                }
                Ok(output)
            }
            Err(SynthDispatchError::Closed(_request)) => {
                let closed_count = self
                    .closed_dispatch_failures
                    .fetch_add(1, Ordering::Relaxed)
                    + 1;
                warn!(
                    synth,
                    closed_count, "synth worker channel closed during dispatch"
                );
                Err(Error::Playback)
            }
        }
    }

    fn try_dispatch(&self, mut request: SynthRequest) -> Result<(), SynthDispatchError> {
        let len = self.request_txs.len();
        let start = self.next_worker.fetch_add(1, Ordering::Relaxed) % len;
        let mut saw_closed = false;

        for offset in 0..len {
            let idx = (start + offset) % len;
            match self.request_txs[idx].try_send(request) {
                Ok(()) => return Ok(()),
                Err(mpsc::error::TrySendError::Full(returned)) => {
                    request = returned;
                }
                Err(mpsc::error::TrySendError::Closed(returned)) => {
                    saw_closed = true;
                    request = returned;
                }
            }
        }

        if saw_closed {
            Err(SynthDispatchError::Closed(request))
        } else {
            Err(SynthDispatchError::Overloaded(request))
        }
    }

    fn queue_depth_snapshot(&self) -> (usize, usize) {
        let mut max_depth = 0usize;
        let mut total_depth = 0usize;
        for tx in self.request_txs.iter() {
            let remaining_capacity = tx.capacity();
            let depth = self
                .queue_capacity_per_worker
                .saturating_sub(remaining_capacity);
            max_depth = max_depth.max(depth);
            total_depth = total_depth.saturating_add(depth);
        }
        let avg_depth = if self.request_txs.is_empty() {
            0
        } else {
            total_depth / self.request_txs.len()
        };
        (max_depth, avg_depth)
    }
}

enum SynthDispatchError {
    Overloaded(SynthRequest),
    Closed(SynthRequest),
}

enum SynthRequest {
    Bass {
        note: Note<BassArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    Cymbal {
        note: Note<CymbalArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    ElectricGuitar {
        note: Note<ElectricGuitarArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32, f32)>,
    },
    HiHat {
        note: Note<HiHatArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    KickDrum {
        note: Note<KickDrumArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    SnareDrum {
        note: Note<SnareDrumArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    Toms {
        note: Note<TomsArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
    Vocals {
        note: Note<VocalsArticulation>,
        response: oneshot::Sender<(Arc<[f32]>, f32, f32)>,
    },
}

fn run_synth_request(worker_index: usize, request: SynthRequest) {
    let synth = request_kind(&request);
    let started_at = Instant::now();
    match request {
        SynthRequest::Bass { note, response } => {
            let _ = response.send(welcome_audio::dsp::bass::synthesize_bass(&to_dsp_note(
                note, (),
            )));
        }
        SynthRequest::Cymbal { note, response } => {
            let _ = response.send(welcome_audio::dsp::cymbal::synthesize_cymbal(&to_dsp_note(
                note, (),
            )));
        }
        SynthRequest::ElectricGuitar { note, response } => {
            let _ = response.send(welcome_audio::dsp::electric_guitar::synthesize_electric_guitar(
                &to_dsp_note(
                    note,
                    to_dsp_electric_guitar_articulation(note.articulation),
                ),
            ));
        }
        SynthRequest::HiHat { note, response } => {
            let _ = response.send(welcome_audio::dsp::hihat::synthesize_hihat(&to_dsp_note(
                note, (),
            )));
        }
        SynthRequest::KickDrum { note, response } => {
            let _ = response.send(welcome_audio::dsp::kick_drum::synthesize_kick_drum(
                &to_dsp_note(note, ()),
            ));
        }
        SynthRequest::SnareDrum { note, response } => {
            let _ = response.send(welcome_audio::dsp::snare_drum::synthesize_snare_drum(
                &to_dsp_note(note, ()),
            ));
        }
        SynthRequest::Toms { note, response } => {
            let _ = response.send(welcome_audio::dsp::toms::synthesize_toms(&to_dsp_note(
                note, (),
            )));
        }
        SynthRequest::Vocals { note, response } => {
            let _ = response.send(welcome_audio::dsp::vocals::synthesize_vocals(&to_dsp_note(
                note,
                to_dsp_vocals_articulation(note.articulation),
            )));
        }
    }
    let elapsed = started_at.elapsed();
    if elapsed > SYNTH_SLOW_REQUEST_WARN_THRESHOLD {
        warn!(
            synth,
            worker_index,
            elapsed_ms = elapsed.as_millis(),
            "slow synth worker request"
        );
    } else {
        trace!(
            synth,
            worker_index,
            elapsed_us = elapsed.as_micros(),
            "synth worker request complete"
        );
    }
}

fn request_kind(request: &SynthRequest) -> &'static str {
    match request {
        SynthRequest::Bass { .. } => "bass",
        SynthRequest::Cymbal { .. } => "cymbal",
        SynthRequest::ElectricGuitar { .. } => "electric_guitar",
        SynthRequest::HiHat { .. } => "hihat",
        SynthRequest::KickDrum { .. } => "kick_drum",
        SynthRequest::SnareDrum { .. } => "snare_drum",
        SynthRequest::Toms { .. } => "toms",
        SynthRequest::Vocals { .. } => "vocals",
    }
}

fn to_dsp_note<A, B>(note: Note<A>, articulation: B) -> welcome_audio::dsp::Note<B> {
    welcome_audio::dsp::Note {
        n_midi: note.n_midi,
        duration: note.duration,
        velocity: note.velocity,
        expression: note.expression.map(|expression| welcome_audio::dsp::Expression {
            bend: expression.bend,
            vibrato: expression.vibrato,
        }),
        articulation,
    }
}

fn to_dsp_electric_guitar_articulation(
    articulation: ElectricGuitarArticulation,
) -> welcome_audio::dsp::electric_guitar::ElectricGuitarArticulation {
    match articulation {
        ElectricGuitarArticulation::Sustained => {
            welcome_audio::dsp::electric_guitar::ElectricGuitarArticulation::Sustained
        }
        ElectricGuitarArticulation::RhythmSustained => {
            welcome_audio::dsp::electric_guitar::ElectricGuitarArticulation::RhythmSustained
        }
    }
}

fn to_dsp_vocals_articulation(
    articulation: VocalsArticulation,
) -> welcome_audio::dsp::vocals::VocalsArticulation {
    match articulation {
        VocalsArticulation::Clean => welcome_audio::dsp::vocals::VocalsArticulation::Clean,
        VocalsArticulation::GroupHarmony => {
            welcome_audio::dsp::vocals::VocalsArticulation::GroupHarmony
        }
        VocalsArticulation::Formant(phonemes) => {
            welcome_audio::dsp::vocals::VocalsArticulation::Formant(phonemes)
        }
    }
}
