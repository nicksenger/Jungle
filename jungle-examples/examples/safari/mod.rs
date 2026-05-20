use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::JourneyStatus;
use jungle_sdk::{JungleClient, LocalClient};
use std::path::PathBuf;
use std::time::Duration;

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

    if headless {
        run_headless();
        return;
    }

    let mut viewer = jungle_viewer::JungleViewerBuilder::new()
        .title("Jungle View Example (zoo::Gorilla)")
        .animation_duration(Duration::from_millis(280));
    if let Some(path) = screenshot {
        viewer = viewer.screenshot_path(path);
    }

    if live {
        let live_runtime = tokio::runtime::Runtime::new().expect("live runtime should start");

        let client = live_runtime
            .block_on(LocalClient::builder().build())
            .expect("local client should build");
        let worker_client = client.clone();

        let _worker_task = live_runtime.spawn(async move {
            let worker = JungleWorker::new(jungle_zoo::Zoo, worker_client);
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
            .expect("gorilla seed should serialize");
        let journey_id = live_runtime
            .block_on(client.start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed))
            .expect("start_journey gorilla should succeed");

        viewer
            .view_live_animal::<jungle_zoo::animals::gorilla::Gorilla, _>(
                client.clone(),
                journey_id,
            )
            .expect("safari example should launch live viewer");
    } else {
        viewer
            .view_animal::<jungle_zoo::animals::gorilla::Gorilla>()
            .expect("safari example should launch viewer");
    }
}

fn run_headless() {
    let runtime = tokio::runtime::Runtime::new().expect("headless runtime should start");
    runtime.block_on(async {
        let client = LocalClient::builder()
            .build()
            .await
            .expect("local client should build");
        let worker_client = client.clone();

        let worker_task = tokio::spawn(async move {
            let worker = JungleWorker::new(jungle_zoo::Zoo, worker_client);
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
            .expect("gorilla seed should serialize");
        let journey_id = client
            .start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed)
            .await
            .expect("start_journey gorilla should succeed");
        println!("started gorilla journey {journey_id} in headless mode");

        let final_status = loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            match status {
                JourneyStatus::Created | JourneyStatus::Alive => {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                JourneyStatus::Completed | JourneyStatus::Stopped | JourneyStatus::Dead => {
                    break status;
                }
            }
        };

        println!("gorilla journey {journey_id} finished with status {final_status:?}");
        worker_task.abort();
    });
}
