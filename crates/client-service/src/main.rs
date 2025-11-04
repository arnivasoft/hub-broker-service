mod config;
mod websocket_client;
mod sync_loop;

use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Client service runs on each branch location
/// Connects to Hub-Broker and handles local PostgreSQL sync
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "client_service=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Client Service...");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;

    info!("Configuration loaded");
    info!("Tenant ID: {}", config.tenant_id);
    info!("Branch ID: {}", config.branch_id);
    info!("Hub URL: {}", config.hub_url);

    // Connect to local PostgreSQL
    let pg_pool = sqlx::PgPool::connect(&config.local_database_url).await?;
    info!("Connected to local PostgreSQL");

    // Install CDC triggers
    let cdc_engine = sync_engine::CdcEngine::new(
        pg_pool.clone(),
        config.tracked_tables.clone(),
    );

    if let Err(e) = cdc_engine.install_triggers(&config.database_schema).await {
        error!("Failed to install CDC triggers: {}", e);
    } else {
        info!("CDC triggers installed");
    }

    // Create WebSocket client
    let ws_client = websocket_client::WebSocketClient::new(
        config.hub_url.clone(),
        config.tenant_id.clone(),
        config.branch_id.clone(),
        config.api_key.clone(),
    );

    // Start sync loop
    let sync_task = tokio::spawn(async move {
        sync_loop::run_sync_loop(ws_client, cdc_engine, pg_pool, config).await
    });

    // Wait for completion
    sync_task.await??;

    Ok(())
}
