use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::JungleClient;
use std::path::PathBuf;
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
        let graph = jungle_viewer::debug_graph_for_animal::<jungle_zoo::probe::ProbeAnimal>();
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
        jungle_viewer::JungleViewerBuilder::new().title("Jungle View Example (probe::ProbeAnimal)");
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
        let client =
            setup_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));
        let worker_client =
            setup_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("worker runtime should start");
            runtime.block_on(async move {
                let worker = JungleWorker::new(jungle_zoo::probe::ProbeZoo, worker_client);
                let _ = worker.spawn().await;
            });
        });

        let seed = postcard::to_allocvec(&()).expect("probe seed should serialize");
        let journey_id = setup_runtime
            .block_on(client.start_journey::<jungle_zoo::probe::ProbeAnimal>(seed))
            .expect("start_journey probe should succeed");

        viewer
            .view_live_animal::<jungle_zoo::probe::ProbeAnimal, _>(client.clone(), journey_id)
            .expect("jungle-view example should launch live viewer");
    } else {
        viewer
            .view_animal::<jungle_zoo::probe::ProbeAnimal>()
            .expect("jungle-view example should launch viewer");
    }
}
