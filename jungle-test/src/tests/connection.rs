use jungle_sdk::server::ServerBuilder;
use jungle_sdk::{
    BackendError, FlowStatus, JungleClient, MockServer, RunnerOut, WireIn, WireOut, Work,
};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn client_exchanges_messages_with_mock_server() {
    let flow_id = Uuid::from_u128(0x11111111111111111111111111111111);
    let action_id = Uuid::from_u128(0x22222222222222222222222222222222);
    let expected_work = Work::StartFlow {
        flow_id,
        ordinal: 7,
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
                                WireIn::CreateFlow { ordinal, seed } => {
                                    if ordinal == 7 && seed == vec![1, 2, 3] {
                                        Ok(WireOut::FlowCreated(flow_id))
                                    } else {
                                        Err(BackendError::Message(
                                            "unexpected create_flow payload".to_string(),
                                        ))
                                    }
                                }
                                other => Err(BackendError::Message(format!(
                                    "expected create_flow first, got {:?}",
                                    other
                                ))),
                            },
                            1 => match msg {
                                WireIn::FlowStatus(id) if id == flow_id => {
                                    Ok(WireOut::FlowStatus(FlowStatus::Created))
                                }
                                other => Err(BackendError::Message(format!(
                                    "expected flow_status second, got {:?}",
                                    other
                                ))),
                            },
                            2 => match msg {
                                WireIn::PollWork => Ok(WireOut::PendingWork(expected_work)),
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
                                WireIn::FlowComplete(id) if id == flow_id => Ok(WireOut::Ack),
                                other => Err(BackendError::Message(format!(
                                    "expected flow_complete last, got {:?}",
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

    let listen_addr = reserve_local_addr();
    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .backend(server)
            .run()
            .await
    });

    let client = connect_client_with_retry(listen_addr).await;

    let created_flow = client
        .create_flow(7, vec![1, 2, 3])
        .await
        .expect("create_flow should succeed");
    assert_eq!(created_flow, flow_id);

    let status = client
        .flow_status(flow_id)
        .await
        .expect("flow_status should succeed");
    assert_eq!(status, FlowStatus::Created);

    let work = client.poll_work().await.expect("poll_work should succeed");
    match work {
        Some(Work::StartFlow {
            flow_id: returned_flow,
            ordinal,
            seed,
        }) => {
            assert_eq!(returned_flow, flow_id);
            assert_eq!(ordinal, 7);
            assert_eq!(seed, vec![1, 2, 3]);
        }
        None => panic!("expected pending work from server"),
    }

    client
        .action_input(action_id, vec![4, 5])
        .await
        .expect("action_input should ack");
    client
        .action_success_output(action_id, vec![6])
        .await
        .expect("action_success_output should ack");
    client
        .action_failure_output(action_id, vec![7, 8])
        .await
        .expect("action_failure_output should ack");
    client
        .flow_complete(flow_id)
        .await
        .expect("flow_complete should ack");

    let requests = captured_requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 7);

    assert!(matches!(
        requests[0],
        WireIn::CreateFlow {
            ordinal,
            ref seed,
        } if ordinal == 7 && seed == &vec![1, 2, 3]
    ));
    assert!(matches!(requests[1], WireIn::FlowStatus(id) if id == flow_id));
    assert!(matches!(requests[2], WireIn::PollWork));
    assert!(matches!(
        requests[3],
        WireIn::HistoryEvent(RunnerOut::ActionInput {
            uuid,
            ref data,
        }) if uuid == action_id && data == &vec![4, 5]
    ));
    assert!(matches!(
        requests[4],
        WireIn::HistoryEvent(RunnerOut::ActionSuccessOutput {
            uuid,
            ref data,
        }) if uuid == action_id && data == &vec![6]
    ));
    assert!(matches!(
        requests[5],
        WireIn::HistoryEvent(RunnerOut::ActionFailureOutput {
            uuid,
            ref data,
        }) if uuid == action_id && data == &vec![7, 8]
    ));
    assert!(matches!(requests[6], WireIn::FlowComplete(id) if id == flow_id));

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn flow_status_moves_created_to_alive_to_completed() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = reserve_local_addr();
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
    let flow_id = client
        .create_flow(7, vec![1, 2, 3])
        .await
        .expect("create_flow should succeed");

    let created = client
        .flow_status(flow_id)
        .await
        .expect("flow_status created should succeed");
    assert_eq!(created, FlowStatus::Created);

    client
        .action_input(flow_id, vec![9, 9, 9])
        .await
        .expect("action_input should succeed");
    let alive = client
        .flow_status(flow_id)
        .await
        .expect("flow_status alive should succeed");
    assert_eq!(alive, FlowStatus::Alive);

    client
        .flow_complete(flow_id)
        .await
        .expect("flow_complete should succeed");
    let completed = client
        .flow_status(flow_id)
        .await
        .expect("flow_status completed should succeed");
    assert_eq!(completed, FlowStatus::Completed);

    server_task.abort();
    let _ = server_task.await;
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::ClientBuilder::new()
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

fn reserve_local_addr() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
        .expect("should bind temporary udp socket for test port reservation");
    socket
        .local_addr()
        .expect("temporary udp socket should expose local address")
}
