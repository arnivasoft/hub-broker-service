use crate::{config::Config, websocket_client::WebSocketClient};
use sync_engine::CdcEngine;
use sqlx::PgPool;
use anyhow::Result;
use std::time::Duration;
use tracing::info;

pub async fn run_sync_loop(
    _ws_client: WebSocketClient,
    cdc_engine: CdcEngine,
    _pg_pool: PgPool,
    config: Config,
) -> Result<()> {
    info!("Starting sync loop...");

    let mut interval = tokio::time::interval(Duration::from_secs(config.sync_interval_secs));

    loop {
        interval.tick().await;

        // Fetch pending changes
        match cdc_engine
            .fetch_pending_changes(&config.database_schema, 100)
            .await
        {
            Ok(changes) => {
                if !changes.is_empty() {
                    info!("Found {} pending changes", changes.len());
                    // TODO: Send changes to hub via WebSocket
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch changes: {}", e);
            }
        }
    }
}
