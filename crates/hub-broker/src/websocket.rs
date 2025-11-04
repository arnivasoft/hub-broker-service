use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use common::{BranchId, ConnectionMetadata};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use protocol::{Message, MessagePayload, JsonCodec, MessageCodec};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};

use crate::server::AppState;

/// Connection manager handles all active WebSocket connections
pub struct ConnectionManager {
    connections: DashMap<BranchId, mpsc::UnboundedSender<Message>>,
    metadata: DashMap<BranchId, ConnectionMetadata>,
    max_connections: usize,
}

impl ConnectionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: DashMap::new(),
            metadata: DashMap::new(),
            max_connections,
        }
    }

    pub async fn add_connection(
        &self,
        branch_id: BranchId,
        sender: mpsc::UnboundedSender<Message>,
    ) -> common::Result<()> {
        if self.connections.len() >= self.max_connections {
            return Err(common::Error::ConnectionError(
                "Max connections reached".to_string(),
            ));
        }

        let metadata = ConnectionMetadata {
            branch_id: branch_id.clone(),
            connected_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            message_count: 0,
        };

        self.connections.insert(branch_id.clone(), sender);
        self.metadata.insert(branch_id, metadata);

        Ok(())
    }

    pub async fn remove_connection(&self, branch_id: &BranchId) {
        self.connections.remove(branch_id);
        self.metadata.remove(branch_id);
    }

    pub async fn send_message(&self, branch_id: &BranchId, message: Message) -> common::Result<()> {
        if let Some(sender) = self.connections.get(branch_id) {
            sender
                .send(message)
                .map_err(|e| common::Error::ConnectionError(format!("Failed to send: {}", e)))?;

            // Update metadata
            if let Some(mut meta) = self.metadata.get_mut(branch_id) {
                meta.message_count += 1;
            }

            Ok(())
        } else {
            Err(common::Error::ConnectionError(
                format!("Branch {} not connected", branch_id),
            ))
        }
    }

    pub async fn broadcast_message(&self, message: Message, exclude: Option<&BranchId>) {
        for entry in self.connections.iter() {
            let branch_id = entry.key();
            if let Some(exclude_id) = exclude {
                if branch_id == exclude_id {
                    continue;
                }
            }

            if let Err(e) = entry.value().send(message.clone()) {
                warn!("Failed to broadcast to {}: {}", branch_id, e);
            }
        }
    }

    pub async fn is_connected(&self, branch_id: &BranchId) -> bool {
        self.connections.contains_key(branch_id)
    }

    pub async fn update_heartbeat(&self, branch_id: &BranchId) {
        if let Some(mut meta) = self.metadata.get_mut(branch_id) {
            meta.last_heartbeat = chrono::Utc::now();
        }
    }

    pub async fn list_connections(&self) -> Vec<serde_json::Value> {
        self.metadata
            .iter()
            .map(|entry| {
                let meta = entry.value();
                serde_json::json!({
                    "branch_id": meta.branch_id.as_str(),
                    "connected_at": meta.connected_at,
                    "last_heartbeat": meta.last_heartbeat,
                    "message_count": meta.message_count,
                })
            })
            .collect()
    }
}

/// WebSocket handler - entry point for WebSocket connections
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let codec = JsonCodec;
    let mut branch_id: Option<BranchId> = None;
    let mut authenticated = false;

    // Spawn task to handle outgoing messages
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Ok(encoded) = codec.encode(&message) {
                if let Ok(text) = String::from_utf8(encoded) {
                    if sender.send(WsMessage::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                match serde_json::from_str::<Message>(&text) {
                    Ok(message) => {
                        if !authenticated {
                            // First message must be Connect
                            if let MessagePayload::Connect(connect_req) = &message.payload {
                                // Authenticate
                                match crate::auth::authenticate_branch(
                                    &state.storage,
                                    &connect_req.branch_id,
                                    &connect_req.api_key,
                                )
                                .await
                                {
                                    Ok(true) => {
                                        authenticated = true;
                                        branch_id = Some(connect_req.branch_id.clone());

                                        // Add to connection manager
                                        if let Err(e) = state
                                            .connection_manager
                                            .add_connection(connect_req.branch_id.clone(), tx.clone())
                                            .await
                                        {
                                            error!("Failed to add connection: {}", e);
                                            break;
                                        }

                                        info!("Branch {} connected", connect_req.branch_id);

                                        // Send ConnectAck
                                        let ack = Message::new(
                                            BranchId::new("hub"),
                                            Some(connect_req.branch_id.clone()),
                                            MessagePayload::ConnectAck(protocol::ConnectAck {
                                                session_id: uuid::Uuid::new_v4().to_string(),
                                                server_version: env!("CARGO_PKG_VERSION").to_string(),
                                                heartbeat_interval_secs: state
                                                    .config
                                                    .server
                                                    .heartbeat_interval_secs,
                                                assigned_config: std::collections::HashMap::new(),
                                            }),
                                        );

                                        let _ = tx.send(ack);
                                    }
                                    _ => {
                                        error!("Authentication failed for {}", connect_req.branch_id);
                                        break;
                                    }
                                }
                            } else {
                                warn!("First message must be Connect");
                                break;
                            }
                        } else {
                            // Handle authenticated messages
                            if let Err(e) = handle_message(message, &state).await {
                                error!("Error handling message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                    }
                }
            } else if let WsMessage::Close(_) = msg {
                info!("Client requested close");
                break;
            }
        }

        // Cleanup on disconnect
        if let Some(id) = branch_id {
            info!("Branch {} disconnected", id);
            state.connection_manager.remove_connection(&id).await;
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

/// Handle authenticated messages
async fn handle_message(message: Message, state: &AppState) -> common::Result<()> {
    debug!("Received message: {:?}", message.payload);

    match &message.payload {
        MessagePayload::Heartbeat => {
            state
                .connection_manager
                .update_heartbeat(&message.from)
                .await;

            // Send HeartbeatAck
            let ack = Message::new(
                BranchId::new("hub"),
                Some(message.from.clone()),
                MessagePayload::HeartbeatAck,
            );
            state.connection_manager.send_message(&message.from, ack).await?;
        }

        MessagePayload::SyncRequest(_) | MessagePayload::SyncBatch(_) => {
            // Route to message router for processing
            state.message_router.route_message(message).await?;
        }

        MessagePayload::RouteMessage(route) => {
            // Forward message to target branch
            if let Some(target) = &message.to {
                state
                    .message_router
                    .forward_to_branch(target, message)
                    .await?;
            }
        }

        _ => {
            debug!("Unhandled message type");
        }
    }

    Ok(())
}
