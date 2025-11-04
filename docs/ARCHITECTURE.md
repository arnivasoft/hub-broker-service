# Hub-Broker Service - Detaylı Mimari Dokümantasyonu

## 🎯 Mimari Genel Bakış

Hub-Broker, merkezi bir relay servisi üzerinden şubeler arası PostgreSQL senkronizasyonu sağlayan, multi-tenant, event-driven bir sistemdir.

## 🏛️ Temel Prensipler

### 1. Multi-Tenancy

**Problem**: Farklı müşterilerin verileri birbirine karışmamalı.

**Çözüm**: 3-katmanlı izolasyon
```
Layer 1: Authentication (JWT + API Key)
Layer 2: Application Logic (Tenant-aware routing)
Layer 3: Database (Separate schemas per tenant)
```

### 2. NAT Traversal

**Problem**: Şubelerden port forwarding ve firewall config zor.

**Çözüm**: WebSocket ile outbound connection
- Şubeler merkeze bağlanır (OUTBOUND = firewall-friendly)
- Persistent WebSocket connection (wss://)
- Auto-reconnect with exponential backoff

### 3. Eventual Consistency

**Problem**: Network partition durumunda sync devam etmeli.

**Çözüm**:
- Vector clocks ile causality tracking
- Conflict detection & resolution
- Store-and-forward for offline branches

## 📐 Detaylı Komponent Mimarisi

### Hub-Broker Server

```rust
┌─────────────────────────────────────────────────────────────┐
│                     Hub-Broker Server                        │
│                                                              │
│  HTTP/WebSocket Layer (Axum)                                │
│  ├── /ws               → WebSocket upgrade                  │
│  ├── /health           → Health check                       │
│  ├── /metrics          → Prometheus metrics                 │
│  └── /admin/*          → Admin endpoints                    │
│                           ↓                                  │
│  Middleware Stack (Tower)                                   │
│  ├── CORS                                                   │
│  ├── Tracing                                                │
│  ├── Compression                                            │
│  └── Rate limiting                                          │
│                           ↓                                  │
│  Connection Manager (DashMap)                               │
│  ├── Active connections: HashMap<BranchId, Sender>          │
│  ├── Metadata: HashMap<BranchId, ConnectionMetadata>        │
│  └── Max connections: 10,000                                │
│                           ↓                                  │
│  Message Router                                             │
│  ├── Tenant isolation enforcement                           │
│  ├── Direct routing: branch A → branch B                    │
│  ├── Broadcast: branch A → all branches in tenant           │
│  └── Offline queue: Redis                                   │
│                           ↓                                  │
│  Storage Layer                                              │
│  ├── PostgreSQL: Metadata, audit logs                       │
│  │   ├── Tenants table                                      │
│  │   ├── Branches table (FK to tenants)                     │
│  │   └── Sync transactions                                  │
│  └── Redis: Session cache, pub/sub                          │
└─────────────────────────────────────────────────────────────┘
```

### Client Service (Branch)

```rust
┌─────────────────────────────────────────────────────────────┐
│                    Client Service (Branch)                   │
│                                                              │
│  WebSocket Client                                           │
│  ├── Connect to hub (with tenant_id + branch_id + api_key) │
│  ├── Maintain persistent connection                         │
│  ├── Auto-reconnect (exponential backoff)                   │
│  └── Heartbeat every 30s                                    │
│                           ↓                                  │
│  Sync Loop (Tokio task)                                     │
│  ├── Poll for local changes every 30s                       │
│  ├── Batch changes (max 100 per batch)                      │
│  ├── Send SyncBatch to hub                                  │
│  └── Wait for SyncAck                                       │
│                           ↓                                  │
│  CDC Engine (Change Data Capture)                           │
│  ├── PostgreSQL triggers on tables                          │
│  ├── sync_change_log table                                  │
│  ├── Capture INSERT/UPDATE/DELETE                           │
│  └── Store with vector clock                                │
│                           ↓                                  │
│  Replication Engine                                         │
│  ├── Receive SyncBatch from hub                             │
│  ├── Conflict detection                                     │
│  ├── Apply changes to local DB                              │
│  └── Send SyncAck                                           │
│                           ↓                                  │
│  Local PostgreSQL Database                                  │
│  └── Business data tables                                   │
└─────────────────────────────────────────────────────────────┘
```

## 🔄 Mesaj Akışı

### Scenario 1: Normal Sync (No Conflict)

```
Time  Branch A           Hub-Broker           Branch B
────────────────────────────────────────────────────────────
T0    INSERT customer
      id=123
      ↓
T1    Trigger logs
      to change_log
      ↓
T2    Sync loop detects
      ↓
T3    SyncBatch ─────────→ Route message
      [customer:123]         Tenant check ✓
                             ↓
T4                           Forward ────────→ Receive batch
                                               Conflict? NO
                                               ↓
T5                                             Apply INSERT
                                               customer:123
                                               ↓
T6                           ←───────────────  SyncAck
T7    ←──────────────────── Confirm
```

### Scenario 2: Conflict Detection & Resolution

```
Branch A                    Hub-Broker                    Branch B
────────────────────────────────────────────────────────────────────
UPDATE customer:123
email="a@test.com"                                    UPDATE customer:123
timestamp=T1                                          email="b@test.com"
vector_clock={A:5,B:3}                               timestamp=T2
                                                      vector_clock={A:3,B:5}
        ↓                         ↓                           ↓
    SyncBatch(A) ──────→  Receive both messages  ←────── SyncBatch(B)
                                  ↓
                          Detect conflict!
                          is_concurrent() = true
                                  ↓
                          Conflict Resolution:
                          Strategy = LastWriteWins
                          T2 > T1 → Branch B wins
                                  ↓
                   ConflictNotification ────────────────────→ Branch A
                   (winning_change = B's data)
                                  ↓
                          Apply B's change ────────────────→ Branch B
                                                              Apply normally
```

## 🔐 Güvenlik Mimarisi

### 1. Authentication Flow

```
Client                         Hub-Broker                    Storage
──────────────────────────────────────────────────────────────────────
POST /auth/token
{
  tenant_id,
  branch_id,
  api_key
}                    ──────→  Validate tenant active?
                                      ↓
                              Query DB: tenant status
                                      ↓                      ←── SELECT
                              Verify branch belongs
                              to tenant                      ←── SELECT
                                      ↓
                              Hash & compare API key
                                      ↓
                              Generate JWT:
                              {
                                tenant_id,
                                branch_id,
                                exp: now + 15min
                              }
             ←────────────  Return token

WebSocket /ws
Headers:
  Authorization: Bearer <JWT>
                     ──────→  Decode JWT
                              Verify signature
                              Check expiry
                              Extract tenant_id
                                      ↓
                              Connection established
                              (tenant-tagged)
```

### 2. Tenant Isolation Enforcement

```rust
// Her routing işleminde:
async fn route_message(message: Message) -> Result<()> {
    // 1. Extract sender tenant
    let sender_tenant = get_tenant_for_branch(&message.from)?;

    // 2. If has target, verify same tenant
    if let Some(target) = message.to {
        let target_tenant = get_tenant_for_branch(&target)?;

        if sender_tenant != target_tenant {
            // CRITICAL: Block cross-tenant routing
            audit_log("SECURITY", "Cross-tenant routing attempt blocked");
            return Err(Error::AuthorizationFailed);
        }
    }

    // 3. Route within tenant boundary
    forward_message(message)?;
}
```

## 📊 Data Model

### PostgreSQL Schema

```sql
-- Global tables (shared across all tenants)
┌─────────────┐
│   tenants   │
├─────────────┤
│ id          │ PK
│ name        │
│ status      │ active/suspended
│ max_branches│
│ schema_name │ UNIQUE
└─────────────┘
      │
      │ 1:N
      ↓
┌─────────────┐
│  branches   │
├─────────────┤
│ id          │ ─┐
│ tenant_id   │ ─┤ Composite PK
│ name        │
│ api_key_hash│
│ status      │
└─────────────┘

-- Per-tenant schemas (isolated)
Schema: tenant_acme_schema
┌─────────────┐
│ change_log  │  ← CDC triggers write here
├─────────────┤
│ id          │
│ table_name  │
│ operation   │
│ row_data    │
│ synced      │
└─────────────┘
```

### Redis Data Structures

```
# Session cache
KEY: session:{branch_id}
VALUE: {tenant_id, connected_at, last_heartbeat}
TTL: 1 hour

# Offline message queue
KEY: offline_queue:{tenant_id}:{branch_id}
TYPE: LIST
VALUE: [Message1, Message2, ...]

# Rate limiting
KEY: rate_limit:{tenant_id}:{branch_id}
TYPE: Counter
TTL: 1 second
```

## 🚀 Ölçeklendirme Stratejisi

### Horizontal Scaling

```
                    Load Balancer (Nginx)
                    ip_hash (sticky sessions)
                           │
          ┌────────────────┼────────────────┐
          ↓                ↓                ↓
    Hub-Broker-1    Hub-Broker-2    Hub-Broker-3
          │                │                │
          └────────────────┼────────────────┘
                           ↓
                    Redis Pub/Sub
                  (Inter-server messaging)
                           ↓
                    PostgreSQL Primary
                           │
              ┌────────────┼────────────┐
              ↓            ↓            ↓
          Read-Replica  Read-Replica  Read-Replica
```

### Connection Distribution

```rust
// Branch connects to any server
Branch A ──→ Server 1 (stores in Redis: branch_a → server_1)
Branch B ──→ Server 2 (stores in Redis: branch_b → server_2)

// Message routing between servers
Server 1: Branch A sends message to Branch B
  ↓
Check Redis: branch_b is on server_2
  ↓
Publish to Redis channel: server_2
  ↓
Server 2 receives and delivers to Branch B
```

## 📈 Performance Optimizations

### 1. Connection Pooling

```rust
// PostgreSQL
PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))

// Redis
ConnectionManager with pool_size=10
```

### 2. Message Batching

```rust
// Collect changes for 30s or 100 changes (whichever first)
let batch = changes.chunks(100).next();
send_batch(batch);
```

### 3. Zero-copy Serialization

```rust
// Use bincode for performance-critical paths
BincodeCodec::encode(message) // ~5x faster than JSON
```

## 🔍 Monitoring & Debugging

### Key Metrics to Watch

```
1. Connection metrics
   - hub_broker_active_connections (per tenant)
   - Connection churn rate

2. Message metrics
   - Message throughput (msg/sec)
   - Message latency (p50, p95, p99)
   - Queue depth

3. Error metrics
   - Routing errors
   - Authentication failures
   - Conflict rate

4. System metrics
   - CPU usage
   - Memory usage
   - Network bandwidth
```

### Debug Checklist

```
Issue: Branch not connecting
□ Check branch API key valid
□ Check tenant status = active
□ Check network connectivity
□ Check JWT not expired
□ Check hub-broker logs

Issue: Messages not routing
□ Verify branches same tenant
□ Check target branch online
□ Check message queue depth
□ Verify no rate limiting

Issue: Conflicts not resolving
□ Check conflict resolution strategy
□ Verify vector clock advancement
□ Check for network partitions
```

## 🎓 Best Practices

1. **API Key Management**
   - Rotate keys quarterly
   - Use strong keys (32+ chars)
   - Never log API keys

2. **Tenant Onboarding**
   - Start with low rate limits
   - Monitor first week closely
   - Gradually increase limits

3. **Monitoring**
   - Alert on connection drops
   - Alert on high conflict rates
   - Alert on queue depth > 1000

4. **Capacity Planning**
   - 1 server = 10K connections
   - Plan for 2x peak load
   - Keep CPU < 70%

---

**Questions? See** [README.md](../README.md) **or open an issue.**
