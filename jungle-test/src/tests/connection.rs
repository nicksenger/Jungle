use futures::StreamExt;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::{
    BackendError, ClaimedAnimalPerturbation, JourneyStatus, JungleClient, MockServer, RunnerOut,
    RunnerUpdateOut, SupportedAnimal, WireIn, WireOut, Work,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

action!(ConnectionAction7, jungle_sdk::typosaurus::num::consts::U80);

animal!(
    ConnectionAnimal7,
    jungle_sdk::typosaurus::num::consts::U7,
    state = (),
    journey = ConnectionJourney7
);

struct ConnectionStep7;
impl jungle_sdk::types::Pulse<ConnectionAnimal7> for ConnectionStep7 {
    type Action = ConnectionAction7;
    type Aspect = jungle_sdk::types::Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(_state: &(), _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(
        _state: &mut (),
        output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
        output.expect("connection animal 7 action should succeed");
    }
}

#[derive(jungle_sdk::Journey)]
struct ConnectionJourney7(jungle_sdk::types::Step<ConnectionAnimal7, ConnectionStep7>);

action!(ConnectionAction9, jungle_sdk::typosaurus::num::consts::U81);

animal!(
    ConnectionAnimal9,
    jungle_sdk::typosaurus::num::consts::U9,
    state = (),
    journey = ConnectionJourney9
);

struct ConnectionStep9;
impl jungle_sdk::types::Pulse<ConnectionAnimal9> for ConnectionStep9 {
    type Action = ConnectionAction9;
    type Aspect = jungle_sdk::types::Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(_state: &(), _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(
        _state: &mut (),
        output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
        output.expect("connection animal 9 action should succeed");
    }
}

#[derive(jungle_sdk::Journey)]
struct ConnectionJourney9(jungle_sdk::types::Step<ConnectionAnimal9, ConnectionStep9>);

#[tokio::test]
async fn client_exchanges_messages_with_mock_server() {
    let journey_id = Uuid::from_u128(0x11111111111111111111111111111111);
    let action_id = Uuid::from_u128(0x22222222222222222222222222222222);
    let expected_work = Work::StartJourney {
        journey_id,
        animal_id: 7,
        generation: 0,
        seed: vec![1, 2, 3],
    };

    let captured_requests: Arc<Mutex<Vec<WireIn>>> = Arc::new(Mutex::new(Vec::new()));
    let request_count = Arc::new(AtomicUsize::new(0));

    let server = MockServer::builder()
        .on_request({
            let captured_requests = Arc::clone(&captured_requests);
            let request_count = Arc::clone(&request_count);
            let expected_work = expected_work.clone();

            move |request| {
                let captured_requests = Arc::clone(&captured_requests);
                let request_count = Arc::clone(&request_count);
                let expected_work = expected_work.clone();

                Box::pin(async move {
                    if let Some(msg) = request {
                        captured_requests.lock().unwrap().push(msg.clone());

                        let idx = request_count.fetch_add(1, Ordering::SeqCst);
                        match idx {
                            0 => match msg {
                                WireIn::CreateJourney {
                                    namespace,
                                    animal_id,
                                    generation,
                                    seed,
                                } => {
                                    if namespace == "default"
                                        && animal_id == 7
                                        && generation == 0
                                        && seed == vec![1, 2, 3]
                                    {
                                        Ok(WireOut::JourneyCreated(journey_id))
                                    } else {
                                        Err(BackendError::Message(
                                            "unexpected create_journey payload".to_string(),
                                        ))
                                    }
                                }
                                other => Err(BackendError::Message(format!(
                                    "expected create_journey first, got {:?}",
                                    other
                                ))),
                            },
                            1 => match msg {
                                WireIn::JourneyStatus(id) if id == journey_id => {
                                    Ok(WireOut::JourneyStatus(JourneyStatus::Created))
                                }
                                other => Err(BackendError::Message(format!(
                                    "expected journey_status second, got {:?}",
                                    other
                                ))),
                            },
                            2 => match msg {
                                WireIn::PollStep { namespace, .. } if namespace == "default" => {
                                    Ok(WireOut::PendingStep(expected_work))
                                }
                                other => Err(BackendError::Message(format!(
                                    "expected poll_work third, got {:?}",
                                    other
                                ))),
                            },
                            3 | 4 | 5 => match msg {
                                WireIn::HistoryEvent(_) => Ok(WireOut::Ack),
                                other => Err(BackendError::Message(format!(
                                    "expected history event, got {:?}",
                                    other
                                ))),
                            },
                            6 => match msg {
                                WireIn::JourneyComplete(id) if id == journey_id => Ok(WireOut::Ack),
                                other => Err(BackendError::Message(format!(
                                    "expected journey_complete last, got {:?}",
                                    other
                                ))),
                            },
                            _ => Err(BackendError::Message(
                                "received more requests than expected".to_string(),
                            )),
                        }
                    } else {
                        Err(BackendError::Message("expected a request".to_string()))
                    }
                })
            }
        })
        .build();

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .backend(server)
            .run()
            .await
    });

    let client = connect_client_with_retry(listen_addr).await;

    let created_flow = client
        .start_journey::<ConnectionAnimal7>(vec![1, 2, 3])
        .await
        .expect("start_journey should succeed");
    assert_eq!(created_flow, journey_id);

    let status = client
        .journey_details(journey_id)
        .await
        .expect("journey_details should succeed");
    assert_eq!(status, JourneyStatus::Created);

    let work = client
        .poll_work(default_supported(7))
        .await
        .expect("poll_work should succeed");
    match work {
        Some(Work::StartJourney {
            journey_id: returned_flow,
            animal_id,
            generation,
            seed,
        }) => {
            assert_eq!(returned_flow, journey_id);
            assert_eq!(animal_id, 7);
            assert_eq!(generation, 0);
            assert_eq!(seed, vec![1, 2, 3]);
        }
        Some(Work::ResumeJourney {
            journey_id,
            animal_id,
            generation,
            seed,
        }) => {
            panic!(
                "unexpected resume journey work in this test: {journey_id} {animal_id} {generation} {seed:?}"
            );
        }
        None => panic!("expected pending work from server"),
    }

    client
        .action_input(action_id, 11, vec![4, 5])
        .await
        .expect("action_input should ack");
    client
        .action_success_output(action_id, 11, vec![6])
        .await
        .expect("action_success_output should ack");
    client
        .action_failure_output(action_id, 11, vec![7, 8])
        .await
        .expect("action_failure_output should ack");
    client
        .complete_journey(journey_id)
        .await
        .expect("complete_journey should ack");

    let requests = captured_requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 7);

    assert!(matches!(
        requests[0],
        WireIn::CreateJourney {
            ref namespace,
            animal_id,
            generation,
            ref seed,
        } if namespace == "default"
            && animal_id == 7
            && generation == 0
            && seed == &vec![1, 2, 3]
    ));
    assert!(matches!(requests[1], WireIn::JourneyStatus(id) if id == journey_id));
    assert!(
        matches!(requests[2], WireIn::PollStep { ref namespace, .. } if namespace == "default")
    );
    assert!(matches!(
        requests[3],
        WireIn::HistoryEvent(RunnerOut::ActionInput {
            node_id,
            uuid,
            ref data,
        }) if node_id == 11 && uuid == action_id && data == &vec![4, 5]
    ));
    assert!(matches!(
        requests[4],
        WireIn::HistoryEvent(RunnerOut::ActionSuccessOutput {
            node_id,
            uuid,
            ref data,
        }) if node_id == 11 && uuid == action_id && data == &vec![6]
    ));
    assert!(matches!(
        requests[5],
        WireIn::HistoryEvent(RunnerOut::ActionFailureOutput {
            node_id,
            uuid,
            ref data,
        }) if node_id == 11 && uuid == action_id && data == &vec![7, 8]
    ));
    assert!(matches!(requests[6], WireIn::JourneyComplete(id) if id == journey_id));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn flow_status_moves_created_to_alive_to_completed() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let client = connect_client_with_retry(listen_addr).await;
    let journey_id = client
        .start_journey::<ConnectionAnimal7>(vec![1, 2, 3])
        .await
        .expect("start_journey should succeed");

    let created = client
        .journey_details(journey_id)
        .await
        .expect("journey_details created should succeed");
    assert_eq!(created, JourneyStatus::Created);

    client
        .action_input(journey_id, 9, vec![9, 9, 9])
        .await
        .expect("action_input should succeed");
    let alive = client
        .journey_details(journey_id)
        .await
        .expect("journey_details alive should succeed");
    assert_eq!(alive, JourneyStatus::Alive);

    client
        .complete_journey(journey_id)
        .await
        .expect("complete_journey should succeed");
    let completed = client
        .journey_details(journey_id)
        .await
        .expect("journey_details completed should succeed");
    assert_eq!(completed, JourneyStatus::Completed);

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn subscribe_journey_updates_streams_history_and_closes_when_terminal() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let client = connect_client_with_retry(listen_addr).await;
    let journey_id = client
        .start_journey::<ConnectionAnimal7>(vec![1, 2, 3])
        .await
        .expect("start_journey should succeed");

    client
        .action_input(journey_id, 12, vec![9, 9])
        .await
        .expect("action_input should succeed");
    client
        .action_success_output(journey_id, 12, vec![8])
        .await
        .expect("action_success_output should succeed");
    client
        .complete_journey(journey_id)
        .await
        .expect("complete_journey should succeed");

    let mut updates = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscription should open");

    let first = updates
        .next()
        .await
        .expect("first update should exist")
        .expect("first update should decode");
    assert_eq!(first.sequence_id, 0);
    assert!(matches!(
        first.event,
        RunnerUpdateOut::ActionInput { node_id, uuid } if node_id == 12 && uuid == journey_id
    ));

    let second = updates
        .next()
        .await
        .expect("second update should exist")
        .expect("second update should decode");
    assert_eq!(second.sequence_id, 1);
    assert!(matches!(
        second.event,
        RunnerUpdateOut::ActionSuccessOutput { node_id, uuid }
            if node_id == 12 && uuid == journey_id
    ));

    let done = updates.next().await;
    assert!(done.is_none(), "terminal journey stream should close");

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn poll_timers_promotes_due_sleep_to_resume_work() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let client = connect_client_with_retry(listen_addr).await;
    let journey_id = client
        .start_journey::<ConnectionAnimal7>(vec![1, 2, 3])
        .await
        .expect("start_journey should succeed");

    let first_work = client
        .poll_work(default_supported(7))
        .await
        .expect("poll_work should succeed");
    assert!(
        matches!(first_work, Some(Work::StartJourney { .. })),
        "expected start journey work item first"
    );

    let timer_id = Uuid::new_v4();
    let wake_at = chrono::Utc::now().timestamp_millis() - 10;
    client
        .schedule_sleep_timer(journey_id, timer_id, wake_at)
        .await
        .expect("schedule_sleep_timer should succeed");

    let _ = client
        .poll_timers()
        .await
        .expect("poll_timers should succeed");

    let resume_work = client
        .poll_work(default_supported(7))
        .await
        .expect("poll_work should succeed");
    match resume_work {
        Some(Work::ResumeJourney {
            journey_id: resumed,
            animal_id,
            generation,
            seed,
        }) => {
            assert_eq!(resumed, journey_id);
            assert_eq!(animal_id, 7);
            assert_eq!(generation, 0);
            assert_eq!(seed, vec![1, 2, 3]);
        }
        Some(Work::StartJourney { .. }) => {
            panic!("expected resume journey work item, got start journey");
        }
        None => panic!("expected resume journey work item"),
    }

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn client_handles_animal_appearance_round_trip() {
    let journey_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let appearance_bytes = vec![42_u8, 99_u8];
    let captured_requests: Arc<Mutex<Vec<WireIn>>> = Arc::new(Mutex::new(Vec::new()));
    let request_count = Arc::new(AtomicUsize::new(0));

    let server = MockServer::builder()
        .on_request({
            let captured_requests = Arc::clone(&captured_requests);
            let request_count = Arc::clone(&request_count);
            let appearance_bytes = appearance_bytes.clone();

            move |request| {
                let captured_requests = Arc::clone(&captured_requests);
                let request_count = Arc::clone(&request_count);
                let appearance_bytes = appearance_bytes.clone();
                Box::pin(async move {
                    let Some(msg) = request else {
                        return Err(BackendError::Message("expected a request".to_string()));
                    };
                    captured_requests.lock().unwrap().push(msg.clone());
                    let idx = request_count.fetch_add(1, Ordering::SeqCst);
                    match (idx, msg) {
                        (0, WireIn::AnimalAppearance(id)) if id == journey_id => {
                            Ok(WireOut::AnimalAppearance(Some(appearance_bytes)))
                        }
                        (1, WireIn::HistoryEvent(RunnerOut::Appearance { uuid, data }))
                            if uuid == journey_id && data == vec![7, 8, 9] =>
                        {
                            Ok(WireOut::Ack)
                        }
                        _ => Err(BackendError::Message(
                            "unexpected request sequence for animal appearance".to_string(),
                        )),
                    }
                })
            }
        })
        .build();

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .backend(server)
            .run()
            .await
    });

    let client = connect_client_with_retry(listen_addr).await;
    let appearance = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed")
        .expect("animal_appearance should return some bytes");
    assert_eq!(appearance, vec![42_u8, 99_u8]);

    client
        .animal_appearance_update(journey_id, vec![7, 8, 9])
        .await
        .expect("animal_appearance_update should ack");

    let requests = captured_requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0], WireIn::AnimalAppearance(id) if id == journey_id));
    assert!(matches!(
        requests[1],
        WireIn::HistoryEvent(RunnerOut::Appearance { uuid, ref data })
            if uuid == journey_id && data == &vec![7, 8, 9]
    ));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn client_handles_animal_perturbation_round_trip() {
    let journey_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
    let captured_requests: Arc<Mutex<Vec<WireIn>>> = Arc::new(Mutex::new(Vec::new()));
    let request_count = Arc::new(AtomicUsize::new(0));

    let server = MockServer::builder()
        .on_request({
            let captured_requests = Arc::clone(&captured_requests);
            let request_count = Arc::clone(&request_count);

            move |request| {
                let captured_requests = Arc::clone(&captured_requests);
                let request_count = Arc::clone(&request_count);
                Box::pin(async move {
                    let Some(msg) = request else {
                        return Err(BackendError::Message("expected a request".to_string()));
                    };
                    captured_requests.lock().unwrap().push(msg.clone());
                    let idx = request_count.fetch_add(1, Ordering::SeqCst);
                    match (idx, msg) {
                        (
                            0,
                            WireIn::PerturbAnimal {
                                journey_id: id,
                                data,
                            },
                        ) if id == journey_id && data == vec![1, 2, 3] => Ok(WireOut::Ack),
                        (1, WireIn::ClaimAnimalPerturbation(id)) if id == journey_id => Ok(
                            WireOut::ClaimedAnimalPerturbation(Some(ClaimedAnimalPerturbation {
                                id: 4,
                                data: vec![9, 8, 7],
                            })),
                        ),
                        (
                            2,
                            WireIn::AckAnimalPerturbation {
                                journey_id: id,
                                perturbation_id,
                            },
                        ) if id == journey_id && perturbation_id == 4 => Ok(WireOut::Ack),
                        _ => Err(BackendError::Message(
                            "unexpected request sequence for animal perturbation".to_string(),
                        )),
                    }
                })
            }
        })
        .build();

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .backend(server)
            .run()
            .await
    });

    let client = connect_client_with_retry(listen_addr).await;
    client
        .perturb_animal(journey_id, vec![1, 2, 3])
        .await
        .expect("perturb_animal should ack");

    let claimed = client
        .claim_animal_perturbation(journey_id)
        .await
        .expect("claim_animal_perturbation should succeed")
        .expect("claim_animal_perturbation should return a claim");
    assert_eq!(claimed.id, 4);
    assert_eq!(claimed.data, vec![9, 8, 7]);

    client
        .ack_animal_perturbation(journey_id, claimed.id)
        .await
        .expect("ack_animal_perturbation should ack");

    let requests = captured_requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 3);
    assert!(matches!(
        requests[0],
        WireIn::PerturbAnimal {
            journey_id: id,
            ref data
        } if id == journey_id && data == &vec![1, 2, 3]
    ));
    assert!(matches!(
        requests[1],
        WireIn::ClaimAnimalPerturbation(id) if id == journey_id
    ));
    assert!(matches!(
        requests[2],
        WireIn::AckAnimalPerturbation {
            journey_id: id,
            perturbation_id
        } if id == journey_id && perturbation_id == 4
    ));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn poll_work_is_scoped_by_namespace() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let alpha = connect_client_with_retry_namespace(listen_addr, "alpha").await;
    let beta = connect_client_with_retry_namespace(listen_addr, "beta").await;

    let alpha_id = alpha
        .start_journey::<ConnectionAnimal7>(vec![1, 2, 3])
        .await
        .expect("alpha start_journey should succeed");
    let beta_id = beta
        .start_journey::<ConnectionAnimal9>(vec![4, 5, 6])
        .await
        .expect("beta start_journey should succeed");

    let alpha_work = alpha
        .poll_work(default_supported(7))
        .await
        .expect("alpha poll_work should succeed");
    match alpha_work {
        Some(Work::StartJourney {
            journey_id,
            animal_id,
            generation,
            seed,
        }) => {
            assert_eq!(journey_id, alpha_id);
            assert_eq!(animal_id, 7);
            assert_eq!(generation, 0);
            assert_eq!(seed, vec![1, 2, 3]);
        }
        other => panic!("expected alpha start work, got {other:?}"),
    }

    let beta_work = beta
        .poll_work(default_supported(9))
        .await
        .expect("beta poll_work should succeed");
    match beta_work {
        Some(Work::StartJourney {
            journey_id,
            animal_id,
            generation,
            seed,
        }) => {
            assert_eq!(journey_id, beta_id);
            assert_eq!(animal_id, 9);
            assert_eq!(generation, 0);
            assert_eq!(seed, vec![4, 5, 6]);
        }
        other => panic!("expected beta start work, got {other:?}"),
    }

    assert!(
        alpha
            .poll_work(default_supported(7))
            .await
            .expect("alpha second poll_work should succeed")
            .is_none(),
        "alpha queue should be drained after claiming its own work"
    );
    assert!(
        beta.poll_work(default_supported(9))
            .await
            .expect("beta second poll_work should succeed")
            .is_none(),
        "beta queue should be drained after claiming its own work"
    );

    server_task.abort();
    let _ = server_task.await;
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    connect_client_with_retry_namespace(remote, "default").await
}

fn default_supported(animal_id: u32) -> Vec<SupportedAnimal> {
    vec![SupportedAnimal {
        animal_id,
        generation: 0,
    }]
}

async fn connect_client_with_retry_namespace(
    remote: SocketAddr,
    namespace: &str,
) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
            .namespace(namespace)
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return client,
            Err(err) if attempt < 39 => {
                std::thread::sleep(Duration::from_millis(25));
                let _ = err;
            }
            Err(err) => panic!("failed to connect to test server: {err}"),
        }
    }

    unreachable!("retry loop always returns or panics")
}
