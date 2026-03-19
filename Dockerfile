# =============================================================================
# Phantom Engine v0.1.0 — Multi-stage Docker build
# Purpose-built browser engine for AI agents
# =============================================================================

# ---- Stage 1: Build --------------------------------------------------------
FROM rust:latest AS builder

LABEL stage="builder"

# Install build dependencies (cmake for zstd, clang for rusty_v8, pkg-config for openssl)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    perl \
    python3 \
    curl \
    nasm \
    ninja-build \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Pin nightly for reproducibility — update this date intentionally when you want to bump
RUN rustup toolchain install nightly && rustup default nightly

# Copy dependency manifests first for cache layering
# This layer is only rebuilt when Cargo.toml/Cargo.lock change
COPY Cargo.toml Cargo.lock ./
COPY crates/phantom-anti-detect/Cargo.toml crates/phantom-anti-detect/
COPY crates/phantom-core/Cargo.toml          crates/phantom-core/
COPY crates/phantom-js/Cargo.toml            crates/phantom-js/
COPY crates/phantom-mcp/Cargo.toml           crates/phantom-mcp/
COPY crates/phantom-net/Cargo.toml           crates/phantom-net/
COPY crates/phantom-serializer/Cargo.toml    crates/phantom-serializer/
COPY crates/phantom-session/Cargo.toml       crates/phantom-session/
COPY crates/phantom-storage/Cargo.toml       crates/phantom-storage/

# Create stub lib.rs / main.rs so cargo can resolve deps without full source
RUN mkdir -p \
    crates/phantom-anti-detect/src \
    crates/phantom-core/src \
    crates/phantom-js/src \
    crates/phantom-mcp/src \
    crates/phantom-net/src \
    crates/phantom-serializer/src \
    crates/phantom-session/src \
    crates/phantom-storage/src && \
    echo "fn main() {}" > crates/phantom-mcp/src/main.rs && \
    for d in phantom-anti-detect phantom-core phantom-js phantom-net phantom-serializer phantom-session phantom-storage; do \
        echo "" > crates/$d/src/lib.rs; \
    done && \
    mkdir -p crates/phantom-serializer/benches && \
    echo "fn main() {}" > crates/phantom-serializer/benches/serializer_bench.rs

# Warm the dep cache — allowed to fail on stubs
RUN cargo build --release --bin phantom-mcp || true

# Now copy the full source and build for real
COPY crates/ crates/
RUN find crates -name "*.rs" | xargs touch && \
    cargo build --release --bin phantom-mcp

# ---- Stage 2: Runtime -------------------------------------------------------
FROM debian:bookworm-slim AS runtime

LABEL maintainer="phantom-engine@example.com"
LABEL version="0.1.0"
LABEL description="Phantom Engine — purpose-built browser engine for AI agents"

# Minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for security
RUN groupadd --gid 1001 phantom && \
    useradd --uid 1001 --gid phantom --no-create-home --shell /bin/false phantom

# Create storage directory with correct permissions
RUN mkdir -p /app/storage && chown phantom:phantom /app/storage

WORKDIR /app

# Copy the release binary from the builder
COPY --from=builder /build/target/release/phantom-mcp /usr/local/bin/phantom-mcp
RUN chown phantom:phantom /usr/local/bin/phantom-mcp && \
    chmod 755 /usr/local/bin/phantom-mcp

USER phantom

# MCP server port
EXPOSE 8080
# Prometheus metrics port
EXPOSE 9091

# Health check via /health endpoint — 30s start-period for BoringSSL/V8 init
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/phantom-mcp"]
CMD ["--port", "8080", "--metrics-port", "9091"]
