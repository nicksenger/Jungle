use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc,
};

use tokio::sync::oneshot;

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

#[derive(Clone)]
pub struct SynthHandle {
    request_tx: Sender<SynthRequest>,
}

impl SynthHandle {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<SynthRequest>();
        std::thread::Builder::new()
            .name("welcome-synth-worker".to_string())
            .spawn(move || run_synth_worker(request_rx))
            .expect("welcome synth worker thread should start");
        Self { request_tx }
    }

    pub async fn bass(
        &self,
        note: Note<BassArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::Bass { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn cymbal(
        &self,
        note: Note<CymbalArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::Cymbal { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn electric_guitar(
        &self,
        note: Note<ElectricGuitarArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::ElectricGuitar { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn hihat(
        &self,
        note: Note<HiHatArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::HiHat { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn kick_drum(
        &self,
        note: Note<KickDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::KickDrum { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn snare_drum(
        &self,
        note: Note<SnareDrumArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::SnareDrum { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn toms(
        &self,
        note: Note<TomsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::Toms { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }

    pub async fn vocals(
        &self,
        note: Note<VocalsArticulation>,
    ) -> Result<(Arc<[f32]>, f32, f32), Error> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(SynthRequest::Vocals { note, response: tx })
            .map_err(|_| Error::Playback)?;
        rx.await.map_err(|_| Error::Playback)
    }
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

fn run_synth_worker(request_rx: Receiver<SynthRequest>) {
    while let Ok(request) = request_rx.recv() {
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
}
