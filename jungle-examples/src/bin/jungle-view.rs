use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{JourneyStatus, SupportedAnimal, Work};
use jungle_sdk::JungleClient;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

fn main() {
    let mut headless = false;
    let mut screenshot: Option<PathBuf> = None;
    let mut dump_graph = false;
    let mut live = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--dump-graph" => dump_graph = true,
            "--live" => live = true,
            "--screenshot" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| panic!("missing value for --screenshot"));
                screenshot = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--screenshot=") => {
                screenshot = Some(PathBuf::from(&arg["--screenshot=".len()..]));
            }
            _ => {}
        }
    }

    if dump_graph {
        let graph =
            jungle_viewer::debug_graph_for_animal::<jungle_zoo::animals::gorilla::Gorilla>();
        println!("nodes:");
        for node in &graph.nodes {
            println!("  {} {}", node.id, node.label);
        }
        println!("edges:");
        for (from, to) in &graph.edges {
            println!("  {} -> {}", from, to);
        }
        println!("while-clusters:");
        for (index, cluster) in graph.while_clusters.iter().enumerate() {
            println!("  #{index}: {:?}", cluster);
        }
    }

    let mut viewer =
        jungle_viewer::JungleViewerBuilder::new().title("Jungle View Example (zoo::Gorilla)");
    if let Some(path) = screenshot {
        viewer = viewer.screenshot_path(path);
    }
    if headless {
        viewer = viewer.headless(true);
    }

    if live {
        let listen_addr = jungle_examples::reserve_local_addr();
        let db_path =
            std::env::temp_dir().join(format!("jungle-view-example-{}.redb", Uuid::new_v4()));

        std::thread::spawn({
            let db_path = db_path.clone();
            move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("server runtime should start");
                runtime.block_on(async move {
                    let _ = ServerBuilder::new()
                        .listen(listen_addr)
                        .redb_path(db_path)
                        .run()
                        .await;
                });
            }
        });

        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("setup runtime should start");
        let client = setup_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));
        let worker_client =
            setup_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("worker runtime should start");
            runtime.block_on(async move {
                run_live_worker_loop(worker_client).await;
            });
        });

        let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
            .expect("gorilla seed should serialize");
        let journey_id = setup_runtime
            .block_on(client.start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed))
            .expect("start_journey gorilla should succeed");

        viewer
            .view_live_animal::<jungle_zoo::animals::gorilla::Gorilla, _>(client.clone(), journey_id)
            .expect("jungle-view example should launch live viewer");
    } else {
        viewer
            .view_animal::<jungle_zoo::animals::gorilla::Gorilla>()
            .expect("jungle-view example should launch viewer");
    }
}

async fn run_live_worker_loop(client: jungle_sdk::Client) {
    let supported = vec![SupportedAnimal {
        animal_id: 0,
        generation: 0,
    }];

    loop {
        let work = client.poll_work(supported.clone()).await;
        let Some(work) = (match work {
            Ok(value) => value,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(25)).await;
                continue;
            }
        }) else {
            let _ = client.poll_timers().await;
            tokio::time::sleep(Duration::from_millis(25)).await;
            continue;
        };

        let (journey_id, seed) = match work {
            Work::StartJourney {
                journey_id, seed, ..
            }
            | Work::ResumeJourney {
                journey_id, seed, ..
            } => (journey_id, seed),
        };

        let start = Instant::now();
        loop {
            let status = match client.journey_details(journey_id).await {
                Ok(status) => status,
                Err(_) => break,
            };
            if matches!(status, JourneyStatus::Completed) {
                break;
            }
            if start.elapsed() >= Duration::from_secs(8) {
                let _ = client.complete_journey(journey_id).await;
                break;
            }

            let sleep_timer_id = Uuid::new_v4();
            let wake_at_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64 + 75)
                .unwrap_or(75);
            let _ = client
                .schedule_sleep_timer(journey_id, sleep_timer_id, wake_at_unix_ms)
                .await;

            let _ = client
                .action_input(journey_id, 0, postcard::to_allocvec(&seed).unwrap_or_default())
                .await;
            let _ = client
                .action_success_output(journey_id, 0, postcard::to_allocvec(&seed).unwrap_or_default())
                .await;
            let _ = client.complete_journey(journey_id).await;
        }
    }
}
