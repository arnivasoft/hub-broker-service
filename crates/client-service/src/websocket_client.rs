use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use protocol::{Message, MessagePayload, ConnectRequest, JsonCodec, MessageCodec};
use common::{BranchId, TenantId};
use futures::{StreamExt, SinkExt};
use std::collections::HashMap;
use tracing::{info, error};

pub struct WebSocketClient {
    hub_url: String,
    tenant_id: TenantId,
    branch_id: BranchId,
    api_key: String,
}

impl WebSocketClient {
    pub fn new(hub_url: String, tenant_id: String, branch_id: String, api_key: String) -> Self {
        Self {
            hub_url,
            tenant_id: TenantId::new(tenant_id),
            branch_id: BranchId::new(branch_id),
            api_key,
        }
    }

    pub async fn connect(&self) -> anyhow::Result<()> {
        info!("Connecting to hub: {}", self.hub_url);

        let (ws_stream, _) = connect_async(&self.hub_url).await?;
        info!("WebSocket connected");

        let (mut write, mut read) = ws_stream.split();
        let codec = JsonCodec;

        // Send Connect message
        let connect_msg = Message::new(
            self.branch_id.clone(),
            None,
            MessagePayload::Connect(ConnectRequest {
                branch_id: self.branch_id.clone(),
                api_key: self.api_key.clone(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec!["sync_v1".to_string()],
                metadata: HashMap::new(),
            }),
        );

        let encoded = codec.encode(&connect_msg)?;
        write
            .send(WsMessage::Text(String::from_utf8(encoded)?))
            .await?;

        info!("Sent Connect message");

        // Handle incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(message) = serde_json::from_str::<Message>(&text) {
                        self.handle_message(message).await;
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    info!("Connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(&self, message: Message) {
        match message.payload {
            MessagePayload::ConnectAck(ack) => {
                info!("Connected! Session ID: {}", ack.session_id);
            }
            MessagePayload::HeartbeatAck => {
                // Heartbeat acknowledged
            }
            MessagePayload::SyncBatch(batch) => {
                info!("Received sync batch: {} changes", batch.changes.len());
                // TODO: Apply changes to local database
            }
            _ => {}
        }
    }
}
