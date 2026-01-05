//! KV Store Server
//!
//! High-performance TCP server for the data-bridge KV store.

use clap::Parser;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod protocol;
mod server;

#[derive(Parser, Debug)]
#[command(name = "kv-server")]
#[command(about = "High-performance KV store TCP server")]
struct Args {
    /// Address to bind to
    #[arg(short, long, default_value = "127.0.0.1:6380")]
    bind: SocketAddr,

    /// Number of shards for the KV engine
    #[arg(short, long, default_value = "256")]
    shards: usize,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: Level,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(args.log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting KV server on {}", args.bind);
    info!("Shards: {}", args.shards);

    // Create and run server
    let server = server::KvServer::new(args.shards);
    server.run(args.bind).await?;

    Ok(())
}
