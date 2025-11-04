use crate::{config::Config, storage::Storage, websocket, auth, routing, metrics};
use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
};
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub storage: Storage,
    pub connection_manager: Arc<websocket::ConnectionManager>,
    pub message_router: Arc<routing::MessageRouter>,
}

pub struct Server {
    config: Config,
    state: AppState,
}

impl Server {
    pub async fn new(config: Config, storage: Storage) -> Result<Self> {
        let connection_manager = Arc::new(websocket::ConnectionManager::new(
            config.server.max_connections,
        ));

        let message_router = Arc::new(routing::MessageRouter::new(
            connection_manager.clone(),
            storage.clone(),
        ));

        let state = AppState {
            config: config.clone(),
            storage,
            connection_manager,
            message_router,
        };

        Ok(Self { config, state })
    }

    pub async fn run(self) -> Result<()> {
        let app = self.build_router();

        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        info!("Server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    fn build_router(&self) -> Router {
        Router::new()
            // WebSocket endpoint
            .route("/ws", get(websocket::ws_handler))

            // Health check
            .route("/health", get(health_check))

            // Metrics
            .route("/metrics", get(metrics::metrics_handler))

            // Admin endpoints
            .route("/admin/branches", get(admin::list_branches))
            .route("/admin/branches/:id/status", get(admin::branch_status))

            // Authentication
            .route("/auth/token", post(auth::generate_token))

            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .with_state(self.state.clone())
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

mod admin {
    use super::*;
    use axum::extract::Path;

    pub async fn list_branches(
        State(state): State<AppState>,
    ) -> Json<serde_json::Value> {
        let connections = state.connection_manager.list_connections().await;
        Json(serde_json::json!({
            "total": connections.len(),
            "branches": connections,
        }))
    }

    pub async fn branch_status(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> Json<serde_json::Value> {
        let branch_id = common::BranchId::new(id);
        let is_connected = state.connection_manager.is_connected(&branch_id).await;

        Json(serde_json::json!({
            "branch_id": branch_id.as_str(),
            "connected": is_connected,
        }))
    }
}
