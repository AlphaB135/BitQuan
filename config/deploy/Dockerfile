# syntax=docker/dockerfile:1
# BitQuan node - multi-stage build with cargo-chef dependency caching

FROM rust:1.84-slim AS planner
ARG TARGETARCH
WORKDIR /app
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN if [ "$TARGETARCH" = "arm64" ]; then ARCH="aarch64"; else ARCH="x86_64"; fi \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v0.1.67/cargo-chef-${ARCH}-unknown-linux-musl.tar.gz" \
    | tar xz -C /usr/local/cargo/bin/
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (cached layer)
FROM rust:1.84-slim AS builder
ARG TARGETARCH
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN if [ "$TARGETARCH" = "arm64" ]; then ARCH="aarch64"; else ARCH="x86_64"; fi \
    && curl -fsSL "https://github.com/LukeMathWalker/cargo-chef/releases/download/v0.1.67/cargo-chef-${ARCH}-unknown-linux-musl.tar.gz" \
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
