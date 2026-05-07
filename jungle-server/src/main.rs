use clap::Parser;

fn main() {
    if jungle_server::init_tracing().is_err() {
        eprintln!("ERROR: failed to initialize tracing subscriber");
        std::process::exit(1);
    }

    let opt = jungle_server::Opt::parse();
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let code = if let Err(e) = rt.block_on(jungle_server::run(opt)) {
        eprintln!("ERROR: {e}");
        1
    } else {
        0
    };
    std::process::exit(code);
}
