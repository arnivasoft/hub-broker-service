# Hızlı Başlangıç Rehberi

## 🎯 5 Dakikada Hub-Broker

### Adım 1: Gereksinimler

```bash
# Rust kurulu mu?
rustc --version  # 1.75+ olmalı

# Docker kurulu mu?
docker --version
docker-compose --version
```

### Adım 2: Projeyi Klonla

```bash
git clone <repo-url>
cd hub-broker-service
```

### Adım 3: Environment Variables

```bash
cp .env.example .env

# .env dosyasını düzenle (en azından JWT_SECRET değiştir)
nano .env
```

### Adım 4: Veritabanlarını Başlat

```bash
# PostgreSQL + Redis + Prometheus + Grafana
docker-compose up -d

# Servislerin hazır olmasını bekle (30 saniye)
sleep 30

# Kontrol et
docker-compose ps
```

### Adım 5: Build & Migrate

```bash
# Cargo build
cargo build

# Database migrations
export DATABASE_URL="postgresql://postgres:password@localhost:5432/hub_broker"
cd crates/hub-broker
cargo install sqlx-cli --no-default-features --features postgres
sqlx database create
sqlx migrate run
cd ../..
```

### Adım 6: Hub-Broker'ı Başlat

```bash
# Terminal 1: Hub-Broker server
RUST_LOG=debug cargo run --bin hub-broker

# Şimdi çalışmalı:
# - http://localhost:8080/health
# - http://localhost:8080/metrics
# - ws://localhost:8080/ws
```

### Adım 7: Test Branch Setup

Yeni bir terminal aç:

```bash
# Terminal 2: Test için local PostgreSQL
docker run -d --name branch-db \
  -e POSTGRES_PASSWORD=password \
  -p 5433:5432 \
  postgres:16-alpine

# Client service .env
cat > .env.client << EOF
TENANT_ID=tenant_demo
BRANCH_ID=branch_test_001
API_KEY=test_api_key_12345
HUB_URL=ws://localhost:8080/ws
LOCAL_DATABASE_URL=postgresql://postgres:password@localhost:5433/postgres
DATABASE_SCHEMA=public
TRACKED_TABLES=test_table
SYNC_INTERVAL=10
EOF

# Client service başlat
env $(cat .env.client | xargs) cargo run --bin client-service
```

## ✅ Doğrulama

### 1. Health Check

```bash
curl http://localhost:8080/health
# {"status":"healthy","timestamp":"..."}
```

### 2. Metrics

```bash
curl http://localhost:8080/metrics | grep hub_broker
```

### 3. Grafana Dashboard

Tarayıcıda aç: http://localhost:3000
- Username: admin
- Password: admin

### 4. Prometheus

Tarayıcıda aç: http://localhost:9090

## 🐛 Sorun Giderme

### Problem: "Database connection failed"

```bash
# PostgreSQL çalışıyor mu?
docker-compose ps postgres

# Logları kontrol et
docker-compose logs postgres

# Restart
docker-compose restart postgres
```

### Problem: "Compilation failed"

```bash
# Dependencies update
cargo update

# Clean build
cargo clean
cargo build
```

### Problem: "WebSocket connection refused"

```bash
# Hub-broker çalışıyor mu?
curl http://localhost:8080/health

# Logları kontrol et
grep ERROR /tmp/hub-broker.log

# Port kullanımda mı?
lsof -i :8080
```

## 📊 İzleme

```bash
# Real-time logs
tail -f logs/hub-broker.log

# Connection count
curl -s http://localhost:8080/metrics | grep active_connections

# Message throughput
watch -n 1 'curl -s http://localhost:8080/metrics | grep messages_total'
```

## 🚀 Sonraki Adımlar

1. **Production Setup**: [docs/DEPLOYMENT.md](DEPLOYMENT.md)
2. **Tenant Management**: [docs/TENANT_MANAGEMENT.md](TENANT_MANAGEMENT.md)
3. **Monitoring**: [docs/MONITORING.md](MONITORING.md)

## 🆘 Yardım

Sorun mu var?
- [GitHub Issues](https://github.com/your-repo/issues)
- [Architecture Guide](ARCHITECTURE.md)
- [FAQ](FAQ.md)
