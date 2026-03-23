# syntax=docker/dockerfile:1
# BitQuan node - multi-stage build with cargo-chef dependency caching

ARG CARGO_CHEF_VERSION=0.1.67

# Stage 1: cargo-chef planner
FROM rust:1.82-slim AS planner
WORKDIR /app
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then ARCH="aarch64"; else ARCH="x86_64"; fi \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-${ARCH}-unknown-linux-musl.tar.gz" \
    | tar xz -C /usr/local/cargo/bin/
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (cached layer)
FROM rust:1.82-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

ARG TARGETARCH
ARG CARGO_CHEF_VERSION
RUN if [ "$TARGETARCH" = "arm64" ]; then ARCH="aarch64"; else ARCH="x86_64"; fi \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CARGO_CHEF_VERSION}/cargo-chef-${ARCH}-unknown-linux-musl.tar.gz" \
    | tar xz -C /usr/local/cargo/bin/
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build application
FROM builder AS build
COPY . .
RUN cargo build --release --bin bitquan-node \
    && strip target/release/bitquan-node

# Stage 4: Runtime
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
