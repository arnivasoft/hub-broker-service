# Hub-Broker Service 🦀

Multi-tenant PostgreSQL senkronizasyon servisi - Rust ile geliştirilmiştir.

## 🎯 Proje Özeti

Hub-Broker, şubelerdeki PostgreSQL veritabanları arasında veri senkronizasyonu sağlayan, NAT-friendly, güvenli ve ölçeklenebilir bir merkezi relay servisidir.

### ✨ Temel Özellikler

- **Multi-Tenant Architecture**: Her müşteri tamamen izole
- **WebSocket-based**: Gerçek zamanlı, çift yönlü iletişim
- **NAT-Friendly**: Şubeler sadece outbound bağlantı açar
- **Change Data Capture**: PostgreSQL trigger-based CDC
- **Conflict Resolution**: Vector clock ile otomatik çakışma çözümü
- **High Performance**: Rust + Tokio async runtime
- **Observable**: Prometheus metrics + structured logging
- **Secure**: JWT auth, TLS, tenant isolation

## 🏗️ Mimari

```
┌─────────────────────────────────────────────────────────────────┐
│              CENTRAL HUB-BROKER (Your Cloud Server)             │
│                                                                  │
│  ┌──────────────┐  ┌───────────────┐  ┌─────────────────┐     │
│  │  WebSocket   │←→│    Message    │←→│    Storage      │     │
│  │  Server      │  │    Router     │  │  (PG + Redis)   │     │
│  │  (Axum)      │  │  (Tenant-     │  │                 │     │
│  └──────────────┘  │   aware)      │  └─────────────────┘     │
│                     └───────────────┘                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket (wss://)
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
   ┌──────────┐         ┌──────────┐         ┌──────────┐
   │ Tenant A │         │ Tenant A │         │ Tenant B │
   │ Branch 1 │         │ Branch 2 │         │ Branch 1 │
   │          │         │          │         │          │
   │ Client   │         │ Client   │         │ Client   │
   │ Service  │         │ Service  │         │ Service  │
   │    ↕     │         │    ↕     │         │    ↕     │
   │   PG DB  │         │   PG DB  │         │   PG DB  │
   └──────────┘         └──────────┘         └──────────┘
```

## 📦 Proje Yapısı

```
hub-broker-service/
├── crates/
│   ├── common/           # Shared types, errors, utilities
│   ├── protocol/         # Message protocol definitions
│   ├── hub-broker/       # Central server (runs in cloud)
│   ├── client-service/   # Client service (runs at branches)
│   └── sync-engine/      # Sync logic & CDC
├── docs/                 # Documentation
├── config/              # Config files
├── docker-compose.yml   # Local development
└── Dockerfile          # Production build
```

## 🚀 Hızlı Başlangıç

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- Redis 7+
- Docker & Docker Compose (opsiyonel)

### 1. Development Environment Setup

```bash
# Clone repository
git clone <repository-url>
cd hub-broker-service

# Copy environment variables
cp .env.example .env

# Edit .env with your credentials
nano .env

# Start PostgreSQL & Redis with Docker
docker-compose up -d postgres redis

# Build the project
cargo build

# Run migrations (önce DATABASE_URL set et)
export DATABASE_URL="postgresql://postgres:password@localhost:5432/hub_broker"
cd crates/hub-broker
cargo install sqlx-cli
sqlx database create
sqlx migrate run

# Start hub-broker server
cargo run --bin hub-broker
```

### 2. Client Service Setup (Her şubede)

```bash
# .env dosyası oluştur
cat > .env << EOF
TENANT_ID=tenant_demo
BRANCH_ID=branch_001
API_KEY=your-api-key-here
HUB_URL=ws://localhost:8080/ws
LOCAL_DATABASE_URL=postgresql://user:pass@localhost:5432/branch_db
DATABASE_SCHEMA=public
TRACKED_TABLES=customers,orders,products
SYNC_INTERVAL=30
EOF

# Client service'i çalıştır
cargo run --bin client-service
```

## 🔐 Multi-Tenant Güvenlik

### Tenant İzolasyonu

Sistem 3 katmanda tenant izolasyonu sağlar:

1. **Authentication Layer**: TenantID + BranchID + API Key
2. **Routing Layer**: Mesajlar SADECE aynı tenant içinde yönlendirilir
3. **Database Layer**: Her tenant için ayrı PostgreSQL schema

Detaylı bilgi için: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## 📊 Monitoring

- Health Check: `http://localhost:8080/health`
- Metrics: `http://localhost:8080/metrics` (Prometheus format)
- Grafana: `http://localhost:3000` (docker-compose ile)

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Check compilation
cargo check --all-features
```

## 🚢 Production Deployment

```bash
# Docker build
docker build -t hub-broker:latest .

# Docker run
docker run -d \
  --name hub-broker \
  -p 8080:8080 \
  -e DATABASE_URL="postgresql://..." \
  -e JWT_SECRET="your-secret" \
  hub-broker:latest
```

Kubernetes deployment için örnek: `docs/kubernetes.yaml`

## 📚 Dokümantasyon

- [Architecture Guide](docs/ARCHITECTURE.md) - Detaylı mimari
- [API Reference](docs/API.md) - API dokümantasyonu
- [Deployment Guide](docs/DEPLOYMENT.md) - Production deployment

## 🛣️ Roadmap

- [x] Multi-tenant architecture
- [x] WebSocket server
- [x] Basic CDC
- [x] Conflict resolution
- [ ] Admin dashboard
- [ ] Horizontal scaling
- [ ] Mobile app support

## 📄 License

MIT OR Apache-2.0

---

**Built with ❤️ and 🦀 Rust**