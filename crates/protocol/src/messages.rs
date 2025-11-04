use common::{BranchId, TenantId, VectorClock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main message envelope for all WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub from: BranchId,
    pub to: Option<BranchId>,
    pub payload: MessagePayload,
}

impl Message {
    pub fn new(from: BranchId, to: Option<BranchId>, payload: MessagePayload) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            from,
            to,
            payload,
        }
    }
}

/// All possible message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessagePayload {
    // Connection lifecycle
    Connect(ConnectRequest),
    ConnectAck(ConnectAck),
    Disconnect(DisconnectReason),
    Heartbeat,
    HeartbeatAck,

    // Sync operations
    SyncRequest(SyncRequest),
    SyncBatch(SyncBatch),
    SyncAck(SyncAck),
    SyncComplete(SyncComplete),

    // Conflict resolution
    ConflictDetected(ConflictNotification),
    ConflictResolved(ConflictResolution),

    // Schema management
    SchemaVersion(SchemaVersionInfo),
    SchemaUpdate(SchemaUpdate),

    // Routing
    RouteMessage(RouteMessage),
    MessageDelivered(MessageDelivered),
    MessageFailed(MessageFailed),

    // Admin/Control
    BranchStatus(BranchStatusUpdate),
    SystemNotification(SystemNotification),

    // Error handling
    Error(ErrorPayload),
}

/// Connect request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub tenant_id: TenantId,
    pub branch_id: BranchId,
    pub api_key: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectAck {
    pub session_id: String,
    pub server_version: String,
    pub heartbeat_interval_secs: u64,
    pub assigned_config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectReason {
    pub code: u16,
    pub reason: String,
}

/// Sync request to get changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub transaction_id: String,
    pub last_sync_timestamp: Option<DateTime<Utc>>,
    pub vector_clock: VectorClock,
    pub tables: Vec<String>,
}

/// Batch of database changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBatch {
    pub transaction_id: String,
    pub vector_clock: VectorClock,
    pub changes: Vec<DatabaseChange>,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseChange {
    pub table_name: String,
    pub operation: Operation,
    pub primary_key: serde_json::Value,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operation {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAck {
    pub transaction_id: String,
    pub applied_changes: usize,
    pub failed_changes: Vec<FailedChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedChange {
    pub index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncComplete {
    pub transaction_id: String,
    pub total_changes: usize,
    pub duration_ms: u64,
}

/// Conflict notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictNotification {
    pub conflict_id: String,
    pub table_name: String,
    pub primary_key: serde_json::Value,
    pub local_change: DatabaseChange,
    pub remote_change: DatabaseChange,
    pub strategy: ConflictStrategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    LastWriteWins,
    FirstWriteWins,
    ManualResolution,
    MergeFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub resolution: ConflictResolutionType,
    pub winning_change: DatabaseChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionType {
    LocalWins,
    RemoteWins,
    Merged,
    Manual,
}

/// Schema version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersionInfo {
    pub version: u32,
    pub checksum: String,
    pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub version: u32,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaUpdate {
    pub old_version: u32,
    pub new_version: u32,
    pub migration_sql: String,
}

/// Route message to another branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMessage {
    pub target_branch: BranchId,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelivered {
    pub message_id: String,
    pub delivered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFailed {
    pub message_id: String,
    pub reason: String,
}

/// Branch status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStatusUpdate {
    pub status: common::BranchStatus,
    pub message: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNotification {
    pub level: NotificationLevel,
    pub message: String,
    pub action_required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
