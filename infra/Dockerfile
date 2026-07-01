# ── Stage 1: Build Rust binary ──
FROM rust:1.82-slim AS rust-builder
WORKDIR /build
COPY crates/ ./crates/
COPY proto/ ./proto/
RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /build/crates
RUN cargo build --release --bin nemesis

# ── Stage 2: Build Python package ──
FROM python:3.12-slim AS python-builder
WORKDIR /build
COPY python/nemesis_alpha/ ./nemesis_alpha/
COPY proto/ ./proto/
RUN pip install --no-cache-dir "./nemesis_alpha[dev]" && \
    python nemesis_alpha/scripts/generate_proto.py

# ── Stage 3: Runtime ──
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /build/crates/target/release/nemesis /usr/local/bin/nemesis
COPY --from=python-builder /usr/local/lib/python3.12/site-packages/ /usr/local/lib/python3.12/site-packages/
COPY config/ /etc/nemesis/
COPY migrations/ /etc/nemesis/migrations/

RUN useradd -m -s /bin/bash nemesis && chown -R nemesis:nemesis /etc/nemesis
USER nemesis

ENV NEMESIS_CONFIG=/etc/nemesis/nemesis.toml
EXPOSE 9090

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:9090/health || exit 1

ENTRYPOINT ["nemesis"]
