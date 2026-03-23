# syntax=docker/dockerfile:1
# BitQuan node - multi-stage build with dependency caching

# Stage 1: Build dependencies (cached layer)
FROM rust:1.87-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create dummy main to cache dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
COPY Cargo.toml Cargo.lock ./
COPY crates crates/
RUN cargo build --release && rm -rf src

# Stage 2: Build application
FROM builder AS build
COPY . .
RUN cargo build --release --bin bitquan-node \
    && strip target/release/bitquan-node

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 bitquan

WORKDIR /app
COPY --from=build /app/target/release/bitquan-node /usr/local/bin/bitquan-node
COPY config ./config

RUN mkdir -p /data/bitquan /data/logs && \
    chown -R bitquan:bitquan /data /app

USER bitquan
EXPOSE 8080 8081 3333 8333

HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["bitquan-node", "--config", "/app/config/mainnet.toml"]
