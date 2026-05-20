use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::{JourneyStatus, RunnerUpdateOut};
use jungle_sdk::{JungleClient, LocalClient};
use std::path::PathBuf;
use std::time::Duration;
use tracing::error;

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
            if let Err(err) = worker.spawn().await {
                error!(error = %err, "safari live worker exited with error");
                eprintln!("safari live worker exited with error: {err}");
            }
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

        let mut worker_task = Some(tokio::spawn(async move {
            let worker = JungleWorker::new(jungle_zoo::Zoo, worker_client);
            worker.spawn().await
        }));

        let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
            .expect("gorilla seed should serialize");
        let journey_id = client
            .start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed)
            .await
            .expect("start_journey gorilla should succeed");
        println!("started gorilla journey {journey_id} in headless mode");

        let mut updates = client
            .subscribe_step_updates(journey_id, None)
            .await
            .expect("subscribe_step_updates should succeed");

        let final_status = loop {
            tokio::select! {
                maybe_update = updates.next() => {
                    match maybe_update {
                        Some(Ok(update)) => {
                            match update.event {
                                RunnerUpdateOut::EffectInput { uuid, node_id } => {
                                    println!(
                                        "journey update seq={} effect_input journey={} node={}",
                                        update.sequence_id,
                                        uuid,
                                        node_id
                                    );
                                }
                                RunnerUpdateOut::EffectSuccessOutput { uuid, node_id } => {
                                    println!(
                                        "journey update seq={} effect_success journey={} node={}",
                                        update.sequence_id,
                                        uuid,
                                        node_id
                                    );
                                }
                                RunnerUpdateOut::EffectFailureOutput { uuid, node_id } => {
                                    println!(
                                        "journey update seq={} effect_failure journey={} node={}",
                                        update.sequence_id,
                                        uuid,
                                        node_id
                                    );
                                }
                                RunnerUpdateOut::SleepScheduled { uuid, timer_id, wake_at_unix_ms } => {
                                    println!(
                                        "journey update seq={} sleep_scheduled journey={} timer={} wake_at_unix_ms={}",
                                        update.sequence_id,
                                        uuid,
                                        timer_id,
                                        wake_at_unix_ms
                                    );
                                }
                                RunnerUpdateOut::SleepFired { uuid, timer_id, fired_at_unix_ms } => {
                                    println!(
                                        "journey update seq={} sleep_fired journey={} timer={} fired_at_unix_ms={}",
                                        update.sequence_id,
                                        uuid,
                                        timer_id,
                                        fired_at_unix_ms
                                    );
                                }
                            }
                        }
                        Some(Err(err)) => {
                            println!("journey update stream error: {err}");
                        }
                        None => {
                            println!("journey update stream closed");
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(250)) => {
                    if worker_task
                        .as_ref()
                        .map(|task| task.is_finished())
                        .unwrap_or(false)
                    {
                        match worker_task
                            .take()
                            .expect("worker task is present when finished")
                            .await
                        {
                            Ok(Ok(())) => {
                                println!("worker task exited cleanly before journey reached terminal status");
                            }
                            Ok(Err(err)) => {
                                println!("worker task exited with error: {err}");
                            }
                            Err(err) => {
                                println!("worker task join error: {err}");
                            }
                        }
                        let history_len = client
                            .journey_history(journey_id)
                            .await
                            .map(|history| history.len())
                            .unwrap_or(0);
                        let status = client
                            .journey_details(journey_id)
                            .await
                            .expect("journey_details should succeed after worker exit");
                        println!(
                            "journey snapshot after worker exit: status={status:?} history_events={history_len}"
                        );
                        break status;
                    }

                    let status = client
                        .journey_details(journey_id)
                        .await
                        .expect("journey_details should succeed");
                    let history_len = client
                        .journey_history(journey_id)
                        .await
                        .map(|history| history.len())
                        .unwrap_or(0);
                    println!("journey status poll: {status:?} (history_events={history_len})");
                    match status {
                        JourneyStatus::Created | JourneyStatus::Alive => {}
                        JourneyStatus::Completed | JourneyStatus::Stopped | JourneyStatus::Dead => {
                            break status;
                        }
                    }
                }
            }
        };

        println!("gorilla journey {journey_id} finished with status {final_status:?}");
        if let Some(worker_task) = worker_task.take() {
            worker_task.abort();
        }
    });
}
