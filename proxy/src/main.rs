use clap::Parser;
use ramparts_common::tracing::{error, info};
use ramparts_proxy::MCPProxy;
use std::process;

#[derive(Parser)]
#[command(name = "ramparts-proxy")]
#[command(about = "Security-first AI Gateway with Javelin Guard integration")]
#[command(version)]
struct Args {
    /// Listen address for the proxy server
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("🛡️  Starting Ramparts AI Gateway (Security-First MCP Proxy)");
    info!("📍 Listen address: {}", args.listen);

    // Create and start the proxy
    match MCPProxy::new(args.listen) {
        Ok(proxy) => {
            info!("✅ Proxy initialized successfully");

            if let Err(e) = proxy.start().await {
                error!("❌ Failed to start proxy: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            error!("❌ Failed to initialize proxy: {}", e);
            process::exit(1);
        }
    }
}
