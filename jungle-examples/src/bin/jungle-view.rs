use std::path::PathBuf;

fn main() {
    let mut headless = false;
    let mut screenshot: Option<PathBuf> = None;
    let mut dump_graph = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--dump-graph" => dump_graph = true,
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
        let graph = jungle_viewer::debug_graph_for_animal::<jungle_examples::StaticAnimal>();
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

    let mut viewer = jungle_viewer::JungleViewerBuilder::new().title("Jungle View Example");
    if let Some(path) = screenshot {
        viewer = viewer.screenshot_path(path);
    }
    if headless {
        viewer = viewer.headless(true);
    }

    viewer
        .view_animal::<jungle_examples::StaticAnimal>()
        .expect("jungle-view example should launch viewer");
}
