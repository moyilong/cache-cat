use core_raft::network::raft::{start_multi_raft};
use core_raft::server::core::config::{Config, ONE, THREE, TWO, load_config};
use core_raft::store::snapshot_handler::load_cache_from_path;
use mimalloc::MiMalloc;
use openraft::AsyncRuntime;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use std::{env, fs, thread};
use tokio::runtime::Builder;
#[cfg(feature = "flamegraph")]
use tracing_flame::FlushGuard;
use tracing_subscriber::EnvFilter;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(feature = "flamegraph")]
fn init_flamegraph(
    path: &str,
) -> Result<FlushGuard<std::io::BufWriter<std::fs::File>>, tracing_flame::Error> {
    use tracing_flame::FlameLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let (flame_layer, guard) = FlameLayer::with_file(path)?;
    tracing_subscriber::registry().with(flame_layer).init();
    eprintln!("flamegraph profiling enabled, output: {}", path);
    Ok(guard)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "tokio-console")]
    {
        console_subscriber::ConsoleLayer::builder()
            .server_addr(([127, 0, 0, 1], 6669))
            .with_default_env()
            .init();
        eprintln!("tokio-console server started on 127.0.0.1:6669");
    }

    #[cfg(feature = "flamegraph")]
    let _flame_guard = init_flamegraph("./flamegraph.folded")?;
    // multi_raft()
    start()
}

fn start() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 2 && args[1] == "--conf" {
        args[2].clone()
    } else {
        eprintln!("Usage: {} --conf <config-file>", args[0]);
        eprintln!("Example: {} --conf conf/node1.toml", args[0]);
        std::process::exit(1);
    };

    let config: Config = load_config(&config_path)?;
    //  创建 runtime 并执行 async
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(start_multi_raft(&config))?;

    Ok(())
}
