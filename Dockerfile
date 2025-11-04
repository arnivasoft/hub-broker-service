# Multi-stage build for minimal image size
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin hub-broker

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/hub-broker /usr/local/bin/hub-broker

# Copy migrations
COPY --from=builder /app/crates/hub-broker/migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 hubbroker && \
    chown -R hubbroker:hubbroker /app

USER hubbroker

EXPOSE 8080

CMD ["hub-broker"]
