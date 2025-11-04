mod config;
mod server;
mod websocket;
mod auth;
mod routing;
mod storage;
mod metrics;

use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hub_broker=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Hub-Broker Service...");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;

    info!("Configuration loaded");
    info!("Server will listen on {}:{}", config.server.host, config.server.port);

    // Initialize storage
    let storage = storage::Storage::new(&config).await?;
    info!("Storage initialized");

    // Initialize metrics
    let metrics_recorder = metrics::init_metrics();
    info!("Metrics initialized");

    // Create and start server
    let server = server::Server::new(config.clone(), storage).await?;

    info!("Server initialized, starting WebSocket endpoint...");

    // Run server
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}
