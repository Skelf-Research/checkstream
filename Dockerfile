# Multi-stage build for CheckStream Proxy
# Optimized for minimal image size and maximum performance

# Build stage
FROM rust:1.75-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary with full optimizations
RUN cargo build --release --bin checkstream-proxy

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/checkstream-proxy /usr/local/bin/

# Copy default configuration
COPY policies/ /app/policies/

# Create non-root user
RUN useradd -m -u 1000 checkstream && \
    chown -R checkstream:checkstream /app

USER checkstream

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:8080/health/live"] || exit 1

ENTRYPOINT ["/usr/local/bin/checkstream-proxy"]
CMD ["--listen", "0.0.0.0", "--port", "8080"]
