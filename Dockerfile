# Vida AI — Headless Server Mode
# Build: docker build -t vida-ai .
# Run:   docker run -p 3690:3690 vida-ai

FROM rust:1.77-slim AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src-tauri/ src-tauri/

# Build release with remote feature
RUN cargo build --release -p vida-ai --features remote

# Runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/vida-ai /usr/local/bin/

EXPOSE 3690
ENV VIDA_PORT=3690
CMD ["vida-ai", "--headless"]
