use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::{mpsc, oneshot};

use super::{
    bass::{self, BassArticulation},
    cymbal::{self, CymbalArticulation},
    electric_guitar::{self, ElectricGuitarArticulation},
    hihat::{self, HiHatArticulation},
    kick_drum::{self, KickDrumArticulation},
    snare_drum::{self, SnareDrumArticulation},
    toms::{self, TomsArticulation},
    vocals::{self, VocalsArticulation},
    Error, Note,
};

const SYNTH_WORKER_THREADS: usize = 3;
const SYNTH_QUEUE_CAPACITY_PER_WORKER: usize = 128;

#[derive(Clone)]
pub struct SynthHandle {
    request_txs: Arc<[mpsc::Sender<SynthRequest>]>,
    next_worker: Arc<AtomicUsize>,
}

impl SynthHandle {
    pub fn new() -> Self {
        let mut request_txs = Vec::with_capacity(SYNTH_WORKER_THREADS);
        for worker_index in 0..SYNTH_WORKER_THREADS {
            let (request_tx, mut request_rx) =
                mpsc::channel::<SynthRequest>(SYNTH_QUEUE_CAPACITY_PER_WORKER);
            std::thread::Builder::new()
                .name(format!("welcome-synth-worker-{worker_index}"))
                .spawn(move || {
                    while let Some(request) = request_rx.blocking_recv() {
                        run_synth_request(request);
                    }
                })
                .expect("welcome synth worker thread should start");
            request_txs.push(request_tx);
        }

        Self {
            request_txs: Arc::from(request_txs),
            next_worker: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn bass(
        &self,
        note: Note<BassArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::Bass { note, response },
            move || bass::audio::synthesize_bass(&note),
        )
        .await
    }

    pub async fn cymbal(
        &self,
        note: Note<CymbalArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::Cymbal { note, response },
            move || cymbal::audio::synthesize_cymbal(&note),
        )
        .await
    }

    pub async fn electric_guitar(
        &self,
        note: Note<ElectricGuitarArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::ElectricGuitar { note, response },
            move || electric_guitar::audio::synthesize_electric_guitar(&note),
        )
        .await
    }

    pub async fn hihat(
        &self,
        note: Note<HiHatArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::HiHat { note, response },
            move || hihat::audio::synthesize_hihat(&note),
        )
        .await
    }

    pub async fn kick_drum(
        &self,
        note: Note<KickDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::KickDrum { note, response },
            move || kick_drum::audio::synthesize_kick_drum(&note),
        )
        .await
    }

    pub async fn snare_drum(
        &self,
        note: Note<SnareDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::SnareDrum { note, response },
            move || snare_drum::audio::synthesize_snare_drum(&note),
        )
        .await
    }

    pub async fn toms(
        &self,
        note: Note<TomsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::Toms { note, response },
            move || toms::audio::synthesize_toms(&note),
        )
        .await
    }

    pub async fn vocals(
        &self,
        note: Note<VocalsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        self.dispatch_with_fallback(
            move |response| SynthRequest::Vocals { note, response },
            move || vocals::audio::synthesize_vocals(&note),
        )
        .await
    }

    async fn dispatch_with_fallback<T, FBuild, FFallback>(
        &self,
        build_request: FBuild,
        fallback: FFallback,
    ) -> Result<T, Error>
    where
        T: Send + 'static,
        FBuild: FnOnce(oneshot::Sender<T>) -> SynthRequest,
        FFallback: FnOnce() -> T,
    {
        let (tx, rx) = oneshot::channel();
        let request = build_request(tx);

        match self.try_dispatch(request) {
            Ok(()) => rx.await.map_err(|_| Error::Playback),
            Err(SynthDispatchError::Overloaded(_request)) => Ok(fallback()),
            Err(SynthDispatchError::Closed(_request)) => Err(Error::Playback),
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

fn run_synth_request(request: SynthRequest) {
    match request {
        SynthRequest::Bass { note, response } => {
            let _ = response.send(bass::audio::synthesize_bass(&note));
        }
        SynthRequest::Cymbal { note, response } => {
            let _ = response.send(cymbal::audio::synthesize_cymbal(&note));
        }
        SynthRequest::ElectricGuitar { note, response } => {
            let _ = response.send(electric_guitar::audio::synthesize_electric_guitar(&note));
        }
        SynthRequest::HiHat { note, response } => {
            let _ = response.send(hihat::audio::synthesize_hihat(&note));
        }
        SynthRequest::KickDrum { note, response } => {
            let _ = response.send(kick_drum::audio::synthesize_kick_drum(&note));
        }
        SynthRequest::SnareDrum { note, response } => {
            let _ = response.send(snare_drum::audio::synthesize_snare_drum(&note));
        }
        SynthRequest::Toms { note, response } => {
            let _ = response.send(toms::audio::synthesize_toms(&note));
        }
        SynthRequest::Vocals { note, response } => {
            let _ = response.send(vocals::audio::synthesize_vocals(&note));
        }
    }
}
