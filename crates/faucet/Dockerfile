# Stage 1: Build faucet binary
FROM rust:1.80-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates crates/

RUN cargo build --release --bin faucet && strip target/release/faucet

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 faucet
USER faucet

COPY --from=builder /app/target/release/faucet /usr/local/bin/faucet

EXPOSE 5000

HEALTHCHECK --interval=30s --timeout=10s --start-period=20s --retries=3 \
    CMD curl -f http://localhost:5000/ || exit 1

CMD ["faucet"]
